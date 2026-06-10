//! 통합 부하 오케스트레이션.
//!
//! - 컨슈머 태스크를 띄우고 파티션 할당(ready)까지 대기한 뒤 부하를 시작한다.
//! - 워크로드 "단위"별로 태스크를 spawn하고 단위 내부 명령은 직렬 전송해 순서를 보존한다
//!   (place → cancel/amend 의존성). 단위 간에는 `--concurrency`만큼 병렬 실행한다.
//! - 접수(SUBMITTED)된 명령만 송신 시각을 correlator에 보고하므로, 거부/에러는 상관에서 제외된다.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use matching_engine::engine::command::EngineCommand;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
    time::sleep,
};
use uuid::Uuid;

use crate::integration_stress::{
    client::{self, GrpcClient, SendOutcome, connect},
    config::{Config, RunMode},
    consumer::{ControlMsg, build_consumer, ensure_topic, run_consumer},
    metrics::{CorrelatorSummary, SentNotice},
    summary::print_report,
    workload::{WorkloadGenerator, command_interval, make_symbols},
};

#[derive(Clone, Copy, Default)]
pub struct DispatchSnapshot {
    pub accepted: u64,
    pub rejected: u64,
    pub resource_exhausted: u64,
    pub error: u64,
}

#[derive(Default)]
struct DispatchCounts {
    accepted: AtomicU64,
    rejected: AtomicU64,
    resource_exhausted: AtomicU64,
    error: AtomicU64,
}

impl DispatchCounts {
    fn snapshot(&self) -> DispatchSnapshot {
        DispatchSnapshot {
            accepted: self.accepted.load(Ordering::Relaxed),
            rejected: self.rejected.load(Ordering::Relaxed),
            resource_exhausted: self.resource_exhausted.load(Ordering::Relaxed),
            error: self.error.load(Ordering::Relaxed),
        }
    }
}

pub struct PhaseOutcome {
    pub attempted: usize,
    pub dispatch: DispatchSnapshot,
    pub dispatch_elapsed: Duration,
    pub pacing_lag_events: usize,
    pub pacing_lag: Duration,
}

pub struct RunReport {
    pub attempted: usize,
    pub dispatch: DispatchSnapshot,
    pub dispatch_elapsed: Duration,
    pub total_elapsed: Duration,
    pub settled: bool,
    pub pacing_lag_events: usize,
    pub pacing_lag: Duration,
    pub correlator: CorrelatorSummary,
}

pub async fn run(config: Config) {
    let symbols = make_symbols(config.symbols);
    let group_id = format!("integ-{}", Uuid::now_v7());

    if let Err(e) = ensure_topic(&config.kafka_brokers, &config.kafka_topic, 3).await {
        eprintln!("Kafka 토픽 준비 실패: {e}");
        return;
    }

    let consumer = match build_consumer(&config.kafka_brokers, &config.kafka_topic, &group_id) {
        Ok(consumer) => consumer,
        Err(e) => {
            eprintln!("Kafka 컨슈머 생성 실패: {e}");
            return;
        }
    };

    let received = Arc::new(AtomicU64::new(0));
    let (sent_tx, sent_rx) = mpsc::unbounded_channel::<SentNotice>();
    let (control_tx, control_rx) = mpsc::unbounded_channel::<ControlMsg>();
    let (ready_tx, ready_rx) = oneshot::channel::<()>();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let consumer_handle = tokio::spawn(run_consumer(
        consumer,
        sent_rx,
        control_rx,
        Arc::clone(&received),
        ready_tx,
        shutdown_rx,
    ));

    if ready_rx.await.is_err() {
        eprintln!("컨슈머 준비 신호 수신 실패");
        return;
    }

    let client = match connect(&config.grpc_endpoint).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("gRPC 연결 실패({}): {e}", config.grpc_endpoint);
            return;
        }
    };

    let report = run_phases(&config, &client, &sent_tx, &symbols, &received, &control_tx).await;

    // 송신 채널을 닫아 correlator가 송신 종료를 인지하게 한 뒤 컨슈머를 종료한다.
    drop(sent_tx);
    let _ = shutdown_tx.send(());
    let correlator = consumer_handle.await.unwrap_or_default();

    let report = RunReport {
        correlator,
        ..report
    };

    print_report(&config, &report);
}

async fn run_phases(
    config: &Config,
    client: &GrpcClient,
    sent_tx: &mpsc::UnboundedSender<SentNotice>,
    symbols: &[String],
    received: &Arc<AtomicU64>,
    control_tx: &mpsc::UnboundedSender<ControlMsg>,
) -> RunReport {
    match config.mode {
        RunMode::Burst { orders } => {
            let dispatch_started = Instant::now();
            let phase = dispatch_burst(config, client, sent_tx, symbols, orders).await;
            let settled = settle(received, phase.dispatch.accepted, config.settle_timeout).await;
            let total_elapsed = dispatch_started.elapsed();
            into_report(phase, total_elapsed, settled)
        }
        RunMode::Paced {
            warmup,
            duration,
            target_commands_per_sec,
        } => {
            if !warmup.is_zero() {
                let w = dispatch_paced(
                    config,
                    client,
                    sent_tx,
                    symbols,
                    warmup,
                    target_commands_per_sec,
                )
                .await;
                let _ = settle(received, w.dispatch.accepted, config.settle_timeout).await;
                reset_correlator(control_tx).await;
            }

            let dispatch_started = Instant::now();
            let phase = dispatch_paced(
                config,
                client,
                sent_tx,
                symbols,
                duration,
                target_commands_per_sec,
            )
            .await;
            let settled = settle(received, phase.dispatch.accepted, config.settle_timeout).await;
            let total_elapsed = dispatch_started.elapsed();
            into_report(phase, total_elapsed, settled)
        }
    }
}

fn into_report(phase: PhaseOutcome, total_elapsed: Duration, settled: bool) -> RunReport {
    RunReport {
        attempted: phase.attempted,
        dispatch: phase.dispatch,
        dispatch_elapsed: phase.dispatch_elapsed,
        total_elapsed,
        settled,
        pacing_lag_events: phase.pacing_lag_events,
        pacing_lag: phase.pacing_lag,
        correlator: CorrelatorSummary::default(),
    }
}

async fn dispatch_burst(
    config: &Config,
    client: &GrpcClient,
    sent_tx: &mpsc::UnboundedSender<SentNotice>,
    symbols: &[String],
    orders: usize,
) -> PhaseOutcome {
    let counts = Arc::new(DispatchCounts::default());
    let mut generator = WorkloadGenerator::default();
    let mut set: JoinSet<()> = JoinSet::new();
    let mut attempted = 0;
    let started = Instant::now();

    for i in 0..orders {
        let symbol = symbols[i % symbols.len()].clone();
        let commands = generator.make_workload(config.scenario, config.sweep_depth, &symbol);
        attempted += commands.len();

        bound_concurrency(&mut set, config.concurrency).await;
        spawn_unit(&mut set, client, sent_tx, &counts, symbol, commands);
    }

    drain(&mut set).await;

    PhaseOutcome {
        attempted,
        dispatch: counts.snapshot(),
        dispatch_elapsed: started.elapsed(),
        pacing_lag_events: 0,
        pacing_lag: Duration::ZERO,
    }
}

async fn dispatch_paced(
    config: &Config,
    client: &GrpcClient,
    sent_tx: &mpsc::UnboundedSender<SentNotice>,
    symbols: &[String],
    duration: Duration,
    target_commands_per_sec: u64,
) -> PhaseOutcome {
    let counts = Arc::new(DispatchCounts::default());
    let mut generator = WorkloadGenerator::default();
    let mut set: JoinSet<()> = JoinSet::new();
    let interval = command_interval(target_commands_per_sec);

    let started = Instant::now();
    let deadline = started + duration;
    let mut next_tick = started;
    let mut attempted = 0;
    let mut symbol_index = 0;
    let mut pacing_lag_events = 0;
    let mut pacing_lag = Duration::ZERO;

    while Instant::now() < deadline {
        let symbol = symbols[symbol_index % symbols.len()].clone();
        symbol_index += 1;
        let commands = generator.make_workload(config.scenario, config.sweep_depth, &symbol);
        let command_count = commands.len() as u32;

        let now = Instant::now();
        if now < next_tick {
            sleep(next_tick - now).await;
        } else if attempted > 0 {
            pacing_lag_events += 1;
            pacing_lag += now - next_tick;
        }

        if Instant::now() >= deadline {
            break;
        }

        attempted += commands.len();
        next_tick += interval * command_count;

        bound_concurrency(&mut set, config.concurrency).await;
        spawn_unit(&mut set, client, sent_tx, &counts, symbol, commands);
    }

    drain(&mut set).await;

    PhaseOutcome {
        attempted,
        dispatch: counts.snapshot(),
        dispatch_elapsed: started.elapsed(),
        pacing_lag_events,
        pacing_lag,
    }
}

fn spawn_unit(
    set: &mut JoinSet<()>,
    client: &GrpcClient,
    sent_tx: &mpsc::UnboundedSender<SentNotice>,
    counts: &Arc<DispatchCounts>,
    symbol: String,
    commands: Vec<EngineCommand>,
) {
    let client = client.clone();
    let sent_tx = sent_tx.clone();
    let counts = Arc::clone(counts);
    set.spawn(process_unit(client, sent_tx, counts, symbol, commands));
}

async fn process_unit(
    mut client: GrpcClient,
    sent_tx: mpsc::UnboundedSender<SentNotice>,
    counts: Arc<DispatchCounts>,
    symbol: String,
    commands: Vec<EngineCommand>,
) {
    for command in commands {
        let order_id = command.order_id().to_string();
        let at = Instant::now();

        match client::send(&mut client, &symbol, &command).await {
            SendOutcome::Submitted => {
                counts.accepted.fetch_add(1, Ordering::Relaxed);
                let _ = sent_tx.send(SentNotice { order_id, at });
            }
            SendOutcome::Rejected => {
                counts.rejected.fetch_add(1, Ordering::Relaxed);
            }
            SendOutcome::ResourceExhausted => {
                counts.resource_exhausted.fetch_add(1, Ordering::Relaxed);
            }
            SendOutcome::Error => {
                counts.error.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

async fn bound_concurrency(set: &mut JoinSet<()>, limit: usize) {
    while set.len() >= limit {
        set.join_next().await;
    }
}

async fn drain(set: &mut JoinSet<()>) {
    while set.join_next().await.is_some() {}
}

async fn settle(received: &Arc<AtomicU64>, target: u64, timeout: Duration) -> bool {
    let started = Instant::now();

    while received.load(Ordering::Relaxed) < target {
        if started.elapsed() >= timeout {
            return false;
        }
        sleep(Duration::from_millis(2)).await;
    }

    true
}

async fn reset_correlator(control_tx: &mpsc::UnboundedSender<ControlMsg>) {
    let (ack_tx, ack_rx) = oneshot::channel();
    if control_tx.send(ControlMsg::Reset(ack_tx)).is_ok() {
        let _ = ack_rx.await;
    }
}

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use matching_engine::engine::command::EngineCommand;
use tokio::{
    sync::{Mutex, mpsc},
    task::JoinSet,
    time::sleep,
};

use crate::integration_stress::{
    client::{self, GrpcClient, SendOutcome},
    config::Config,
    metrics::SentNotice,
    workload::{WorkloadGenerator, command_interval, commands_per_workload},
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
    pub pacing_lag_max: Duration,
}

#[derive(Default)]
pub(crate) struct PacingLag {
    pub events: usize,
    pub total: Duration,
    pub max: Duration,
}

impl PacingLag {
    pub fn record(&mut self, now: Instant, next_tick: Instant, attempted: usize) {
        if attempted == 0 || now <= next_tick {
            return;
        }

        let lag = now - next_tick;
        self.events += 1;
        self.total += lag;
        self.max = self.max.max(lag);
    }
}

struct WorkUnit {
    symbol: String,
    commands: Vec<EngineCommand>,
}

pub async fn dispatch_burst(
    config: &Config,
    clients: &[GrpcClient],
    sent_tx: &mpsc::UnboundedSender<SentNotice>,
    symbols: &[String],
    orders: usize,
) -> PhaseOutcome {
    let counts = Arc::new(DispatchCounts::default());
    let mut generator = WorkloadGenerator::default();
    let (work_tx, work_rx) = mpsc::channel(config.concurrency);
    let mut workers = spawn_workers(
        config.concurrency,
        clients,
        sent_tx,
        Arc::clone(&counts),
        work_rx,
    );
    let mut attempted = 0;
    let started = Instant::now();

    for i in 0..orders {
        let symbol = symbols[i % symbols.len()].clone();
        let commands = generator.make_workload(config.scenario, config.sweep_depth, &symbol);
        debug_assert_eq!(
            commands.len(),
            commands_per_workload(config.scenario, config.sweep_depth)
        );
        attempted += commands.len();
        if work_tx.send(WorkUnit { symbol, commands }).await.is_err() {
            break;
        }
    }

    drop(work_tx);
    drain(&mut workers).await;

    PhaseOutcome {
        attempted,
        dispatch: counts.snapshot(),
        dispatch_elapsed: started.elapsed(),
        pacing_lag_events: 0,
        pacing_lag: Duration::ZERO,
        pacing_lag_max: Duration::ZERO,
    }
}

pub async fn dispatch_paced(
    config: &Config,
    clients: &[GrpcClient],
    sent_tx: &mpsc::UnboundedSender<SentNotice>,
    symbols: &[String],
    duration: Duration,
    target_commands_per_sec: u64,
) -> PhaseOutcome {
    let counts = Arc::new(DispatchCounts::default());
    let mut generator = WorkloadGenerator::default();
    let (work_tx, work_rx) = mpsc::channel(config.concurrency);
    let mut workers = spawn_workers(
        config.concurrency,
        clients,
        sent_tx,
        Arc::clone(&counts),
        work_rx,
    );
    let interval = command_interval(target_commands_per_sec);

    let started = Instant::now();
    let deadline = started + duration;
    let mut next_tick = started;
    let mut attempted = 0;
    let mut symbol_index = 0;
    let mut pacing_lag = PacingLag::default();

    while Instant::now() < deadline {
        let symbol = symbols[symbol_index % symbols.len()].clone();
        symbol_index += 1;
        let commands = generator.make_workload(config.scenario, config.sweep_depth, &symbol);
        debug_assert_eq!(
            commands.len(),
            commands_per_workload(config.scenario, config.sweep_depth)
        );
        let command_count = commands.len() as u32;

        let now = Instant::now();
        if now < next_tick {
            sleep(next_tick - now).await;
        } else {
            pacing_lag.record(now, next_tick, attempted);
        }

        if Instant::now() >= deadline {
            break;
        }

        attempted += commands.len();
        next_tick += interval * command_count;
        if work_tx.send(WorkUnit { symbol, commands }).await.is_err() {
            break;
        }
    }

    drop(work_tx);
    drain(&mut workers).await;

    PhaseOutcome {
        attempted,
        dispatch: counts.snapshot(),
        dispatch_elapsed: started.elapsed(),
        pacing_lag_events: pacing_lag.events,
        pacing_lag: pacing_lag.total,
        pacing_lag_max: pacing_lag.max,
    }
}

fn spawn_workers(
    count: usize,
    clients: &[GrpcClient],
    sent_tx: &mpsc::UnboundedSender<SentNotice>,
    counts: Arc<DispatchCounts>,
    work_rx: mpsc::Receiver<WorkUnit>,
) -> JoinSet<()> {
    let mut workers = JoinSet::new();
    let work_rx = Arc::new(Mutex::new(work_rx));
    for i in 0..count {
        // 연결 풀을 round-robin으로 배분한다. 풀이 워커보다 작으면 일부 연결을 공유한다.
        let client = clients[i % clients.len()].clone();
        let sent_tx = sent_tx.clone();
        let counts = Arc::clone(&counts);
        let work_rx = Arc::clone(&work_rx);
        workers.spawn(worker_loop(client, sent_tx, counts, work_rx));
    }
    workers
}

async fn worker_loop(
    mut client: GrpcClient,
    sent_tx: mpsc::UnboundedSender<SentNotice>,
    counts: Arc<DispatchCounts>,
    work_rx: Arc<Mutex<mpsc::Receiver<WorkUnit>>>,
) {
    loop {
        let unit = {
            let mut receiver = work_rx.lock().await;
            receiver.recv().await
        };

        let Some(unit) = unit else {
            break;
        };

        process_unit(&mut client, &sent_tx, &counts, unit.symbol, unit.commands).await;
    }
}

async fn process_unit(
    client: &mut GrpcClient,
    sent_tx: &mpsc::UnboundedSender<SentNotice>,
    counts: &DispatchCounts,
    symbol: String,
    commands: Vec<EngineCommand>,
) {
    // unit 전체를 batch RPC 1콜로 전송한다. unit 내부 명령 묶음이므로 같은 T1/T2를 공유한다.
    let sent_at = Instant::now();
    let outcomes = client::send_batch(client, &symbol, &commands).await;
    let acked_at = Instant::now();

    for (command, outcome) in commands.iter().zip(outcomes) {
        match outcome {
            SendOutcome::Submitted => {
                counts.accepted.fetch_add(1, Ordering::Relaxed);
                let _ = sent_tx.send(SentNotice {
                    order_id: command.order_id().to_string(),
                    sent_at,
                    acked_at,
                });
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

async fn drain(set: &mut JoinSet<()>) {
    while set.join_next().await.is_some() {}
}

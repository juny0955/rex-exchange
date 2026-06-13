//! 통합 부하 오케스트레이션.
//!
//! - 컨슈머 태스크를 띄우고 파티션 할당(ready)까지 대기한 뒤 부하를 시작한다.
//! - 워크로드 "단위"별로 태스크를 spawn하고 단위 내부 명령은 직렬 전송해 순서를 보존한다
//!   (place → cancel/amend 의존성). 단위 간에는 `--concurrency`만큼 병렬 실행한다.
//! - 접수(SUBMITTED)된 명령만 송신 시각을 correlator에 보고하므로, 거부/에러는 상관에서 제외된다.

use std::{
    sync::{Arc, atomic::AtomicU64},
    time::{Duration, Instant},
};

use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::integration_stress::{
    client::{GrpcClient, connect_pool},
    config::{Config, RunMode},
    consumer::{ControlMsg, SettleResult, build_consumer, ensure_topic, run_consumer},
    dispatch::{PhaseOutcome, dispatch_burst, dispatch_paced},
    metrics::{CorrelatorSummary, SentNotice},
    summary::print_report,
    workload::make_symbols,
};

pub use crate::integration_stress::dispatch::DispatchSnapshot;

pub struct RunReport {
    pub attempted: usize,
    pub dispatch: DispatchSnapshot,
    pub dispatch_elapsed: Duration,
    pub total_elapsed: Duration,
    pub settled: bool,
    pub pacing_lag_events: usize,
    pub pacing_lag: Duration,
    pub pacing_lag_max: Duration,
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

    let consumer_handle = tokio::spawn(run_consumer(
        consumer,
        sent_rx,
        control_rx,
        Arc::clone(&received),
        ready_tx,
    ));

    if ready_rx.await.is_err() {
        eprintln!("컨슈머 준비 신호 수신 실패");
        return;
    }

    let clients = match connect_pool(&config.grpc_endpoint, config.connections).await {
        Ok(clients) => clients,
        Err(e) => {
            eprintln!("gRPC 연결 실패({}): {e}", config.grpc_endpoint);
            return;
        }
    };

    let report = run_phases(&config, &clients, &sent_tx, &symbols, &control_tx).await;

    let _ = finish_correlator(&control_tx).await;
    drop(sent_tx);
    drop(control_tx);
    let _ = consumer_handle.await;

    print_report(&config, &report);
}

async fn run_phases(
    config: &Config,
    clients: &[GrpcClient],
    sent_tx: &mpsc::UnboundedSender<SentNotice>,
    symbols: &[String],
    control_tx: &mpsc::UnboundedSender<ControlMsg>,
) -> RunReport {
    match config.mode {
        RunMode::Burst { orders } => {
            let dispatch_started = Instant::now();
            let phase = dispatch_burst(config, clients, sent_tx, symbols, orders).await;
            let settle =
                settle_correlator(control_tx, phase.dispatch.accepted, config.settle_timeout)
                    .await
                    .unwrap_or_default();
            let total_elapsed = dispatch_started.elapsed();
            into_report(phase, total_elapsed, settle)
        }
        RunMode::Paced {
            warmup,
            duration,
            target_commands_per_sec,
        } => {
            if !warmup.is_zero() {
                let w = dispatch_paced(
                    config,
                    clients,
                    sent_tx,
                    symbols,
                    warmup,
                    target_commands_per_sec,
                )
                .await;
                settle_and_reset_correlator(control_tx, w.dispatch.accepted, config.settle_timeout)
                    .await;
            }

            let dispatch_started = Instant::now();
            let phase = dispatch_paced(
                config,
                clients,
                sent_tx,
                symbols,
                duration,
                target_commands_per_sec,
            )
            .await;
            let settle =
                settle_correlator(control_tx, phase.dispatch.accepted, config.settle_timeout)
                    .await
                    .unwrap_or_default();
            let total_elapsed = dispatch_started.elapsed();
            into_report(phase, total_elapsed, settle)
        }
    }
}

fn into_report(phase: PhaseOutcome, total_elapsed: Duration, settle: SettleResult) -> RunReport {
    RunReport {
        attempted: phase.attempted,
        dispatch: phase.dispatch,
        dispatch_elapsed: phase.dispatch_elapsed,
        total_elapsed,
        settled: settle.settled,
        pacing_lag_events: phase.pacing_lag_events,
        pacing_lag: phase.pacing_lag,
        pacing_lag_max: phase.pacing_lag_max,
        correlator: settle.summary,
    }
}

async fn settle_and_reset_correlator(
    control_tx: &mpsc::UnboundedSender<ControlMsg>,
    expected: u64,
    timeout: Duration,
) {
    let (ack_tx, ack_rx) = oneshot::channel();
    if control_tx
        .send(ControlMsg::SettleAndReset {
            expected,
            timeout,
            ack: ack_tx,
        })
        .is_ok()
    {
        let _ = ack_rx.await;
    }
}

async fn settle_correlator(
    control_tx: &mpsc::UnboundedSender<ControlMsg>,
    expected: u64,
    timeout: Duration,
) -> Option<SettleResult> {
    let (ack_tx, ack_rx) = oneshot::channel();
    control_tx
        .send(ControlMsg::Settle {
            expected,
            timeout,
            ack: ack_tx,
        })
        .ok()?;
    ack_rx.await.ok()
}

async fn finish_correlator(control_tx: &mpsc::UnboundedSender<ControlMsg>) -> Option<SettleResult> {
    let (ack_tx, ack_rx) = oneshot::channel();
    control_tx
        .send(ControlMsg::Finish {
            expected: 0,
            timeout: Duration::ZERO,
            ack: ack_tx,
        })
        .ok()?;
    ack_rx.await.ok()
}

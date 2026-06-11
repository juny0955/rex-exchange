use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use matching_engine::engine::command::EngineCommand;
use tokio::{sync::mpsc, task::JoinSet, time::sleep};

use crate::integration_stress::{
    client::{self, GrpcClient, SendOutcome},
    config::Config,
    metrics::SentNotice,
    workload::{WorkloadGenerator, command_interval},
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

pub async fn dispatch_burst(
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
        pacing_lag_max: Duration::ZERO,
    }
}

pub async fn dispatch_paced(
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
    let mut pacing_lag_max = Duration::ZERO;

    while Instant::now() < deadline {
        let symbol = symbols[symbol_index % symbols.len()].clone();
        symbol_index += 1;
        let commands = generator.make_workload(config.scenario, config.sweep_depth, &symbol);
        let command_count = commands.len() as u32;

        let now = Instant::now();
        if now < next_tick {
            sleep(next_tick - now).await;
        } else if attempted > 0 {
            let lag = now - next_tick;
            pacing_lag_events += 1;
            pacing_lag += lag;
            pacing_lag_max = pacing_lag_max.max(lag);
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
        pacing_lag_max,
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

use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use matching_engine::engine::runtime::EngineRuntime;

use crate::runtime_stress::{
    config::{Config, RunMode, Scenario},
    latency::LatencyTracker,
    publisher::CountingPublisher,
    runner_dispatch::{dispatch_command, dispatch_commands, wait_until_published},
    runner_pacing::{CommandCursor, command_interval},
    stats::{DispatchStats, PhaseStats, ResultStats},
    summary::{print_burst_summary, print_paced_summary},
    workload::{WorkloadGenerator, make_symbols},
};

pub fn run(config: Config) {
    let symbols = make_symbols(config.symbols);
    let result_stats = Arc::new(ResultStats::default());
    let latency = LatencyTracker::new();

    let runtime = EngineRuntime::new(
        symbols.clone(),
        Box::new(CountingPublisher::new(
            Arc::clone(&result_stats),
            latency.correlator(),
            config.publisher_delay,
        )),
    );

    let dispatcher = runtime.dispatcher();
    let mut generator = WorkloadGenerator::default();

    match config.mode {
        RunMode::Burst { orders } => {
            let result = run_burst_phase(
                &dispatcher,
                &symbols,
                &mut generator,
                &result_stats,
                &latency,
                &config,
                orders,
            );
            print_burst_summary(&config, orders, &result);
            finish_runtime(runtime, dispatcher, result.completed);
        }
        RunMode::Paced {
            warmup,
            duration,
            target_commands_per_sec,
        } => {
            let mut cursor = CommandCursor::default();
            let warmup_result = if warmup.is_zero() {
                PhaseStats {
                    completed: true,
                    ..PhaseStats::default()
                }
            } else {
                run_paced_phase(
                    &dispatcher,
                    &symbols,
                    &mut cursor,
                    &result_stats,
                    &latency,
                    PacedPhase {
                        scenario: config.scenario,
                        sweep_depth: config.sweep_depth,
                        duration: warmup,
                        target_commands_per_sec,
                        timeout: config.timeout,
                    },
                )
            };

            let measurement_result = if warmup_result.completed {
                Some(run_paced_phase(
                    &dispatcher,
                    &symbols,
                    &mut cursor,
                    &result_stats,
                    &latency,
                    PacedPhase {
                        scenario: config.scenario,
                        sweep_depth: config.sweep_depth,
                        duration,
                        target_commands_per_sec,
                        timeout: config.timeout,
                    },
                ))
            } else {
                None
            };

            print_paced_summary(
                &config,
                target_commands_per_sec,
                warmup,
                &warmup_result,
                duration,
                measurement_result.as_ref(),
            );

            finish_runtime(
                runtime,
                dispatcher,
                warmup_result.completed
                    && measurement_result
                        .as_ref()
                        .is_some_and(|result| result.completed),
            );
        }
    }
}

struct PacedPhase {
    scenario: Scenario,
    sweep_depth: usize,
    duration: Duration,
    target_commands_per_sec: u64,
    timeout: Duration,
}

fn run_burst_phase(
    dispatcher: &matching_engine::engine::dispatcher::EngineDispatcher,
    symbols: &[String],
    generator: &mut WorkloadGenerator,
    result_stats: &ResultStats,
    latency: &LatencyTracker,
    config: &Config,
    orders: usize,
) -> PhaseStats {
    let baseline = result_stats.snapshot();
    let started = Instant::now();
    let mut dispatch = DispatchStats::default();
    let mut attempted_commands = 0;

    for i in 0..orders {
        let symbol = &symbols[i % symbols.len()];
        let commands = generator.make_workload(config.scenario, config.sweep_depth, symbol);
        attempted_commands += commands.len();

        dispatch_commands(
            dispatcher,
            symbol,
            commands,
            &mut dispatch,
            latency.recorder(),
        );
    }

    let dispatch_elapsed = started.elapsed();
    let completed = wait_until_published(
        result_stats,
        baseline.published + dispatch.accepted,
        config.timeout,
    );
    let total_elapsed = started.elapsed();

    PhaseStats {
        attempted_commands,
        dispatch,
        result: result_stats.snapshot() - baseline,
        completed,
        dispatch_elapsed,
        total_elapsed,
        pacing_lag_events: 0,
        pacing_lag: Duration::ZERO,
        latency: latency.take_summary(),
    }
}

fn run_paced_phase(
    dispatcher: &matching_engine::engine::dispatcher::EngineDispatcher,
    symbols: &[String],
    cursor: &mut CommandCursor,
    result_stats: &ResultStats,
    latency: &LatencyTracker,
    phase: PacedPhase,
) -> PhaseStats {
    let baseline = result_stats.snapshot();
    let started = Instant::now();
    let deadline = started + phase.duration;
    let interval = command_interval(phase.target_commands_per_sec);
    let mut next_tick = started;
    let mut dispatch = DispatchStats::default();
    let mut attempted_commands = 0;
    let mut pacing_lag_events = 0;
    let mut pacing_lag = Duration::ZERO;

    while Instant::now() < deadline {
        let now = Instant::now();
        if now < next_tick {
            thread::sleep(next_tick - now);
        } else if attempted_commands > 0 && now > next_tick {
            pacing_lag_events += 1;
            pacing_lag += now - next_tick;
        }

        if Instant::now() >= deadline {
            break;
        }

        let Some((symbol, command)) =
            cursor.next_command(symbols, phase.scenario, phase.sweep_depth)
        else {
            break;
        };

        attempted_commands += 1;
        dispatch_command(
            dispatcher,
            &symbol,
            command,
            &mut dispatch,
            latency.recorder(),
        );
        next_tick += interval;
    }

    let dispatch_elapsed = started.elapsed();
    let completed = wait_until_published(
        result_stats,
        baseline.published + dispatch.accepted,
        phase.timeout,
    );
    let total_elapsed = started.elapsed();

    PhaseStats {
        attempted_commands,
        dispatch,
        result: result_stats.snapshot() - baseline,
        completed,
        dispatch_elapsed,
        total_elapsed,
        pacing_lag_events,
        pacing_lag,
        latency: latency.take_summary(),
    }
}

fn finish_runtime(
    runtime: EngineRuntime,
    dispatcher: matching_engine::engine::dispatcher::EngineDispatcher,
    completed: bool,
) {
    drop(dispatcher);

    if completed {
        runtime.shutdown();
    } else {
        eprintln!("제한 시간 안에 접수된 모든 명령의 결과가 발행되지 않아 종료 대기를 생략합니다");
    }
}

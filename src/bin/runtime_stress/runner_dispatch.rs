use std::{
    thread,
    time::{Duration, Instant},
};

use matching_engine::engine::{
    command::EngineCommand, dispatcher::DispatchError, dispatcher::EngineDispatcher,
};

use crate::runtime_stress::{
    latency::LatencyRecorder,
    stats::{DispatchStats, ResultStats},
};

pub(super) fn dispatch_commands(
    dispatcher: &EngineDispatcher,
    symbol: &str,
    commands: Vec<EngineCommand>,
    stats: &mut DispatchStats,
    recorder: &LatencyRecorder,
) {
    for command in commands {
        dispatch_command(dispatcher, symbol, command, stats, recorder);
    }
}

pub(super) fn dispatch_command(
    dispatcher: &EngineDispatcher,
    symbol: &str,
    command: EngineCommand,
    stats: &mut DispatchStats,
    recorder: &LatencyRecorder,
) {
    let order_id = command.order_id();

    match dispatcher.dispatch(symbol, command) {
        Ok(()) => {
            recorder.record(order_id, Instant::now());
            stats.accepted += 1;
        }
        Err(DispatchError::ChannelFull { .. }) => stats.channel_full += 1,
        Err(DispatchError::UnknownSymbol { .. }) => stats.unknown_symbol += 1,
        Err(DispatchError::EngineStopped { .. }) => stats.engine_stopped += 1,
        Err(DispatchError::PublisherUnhealthy { .. }) => stats.engine_stopped += 1,
    }
}

pub(super) fn wait_until_published(
    result_stats: &ResultStats,
    expected_published: usize,
    timeout: Duration,
) -> bool {
    let started = std::time::Instant::now();

    while result_stats.published() < expected_published {
        if started.elapsed() >= timeout {
            return false;
        }

        thread::sleep(Duration::from_millis(1));
    }

    true
}

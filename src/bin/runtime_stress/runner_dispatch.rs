use std::{thread, time::Duration};

use matching_engine::engine::{
    command::EngineCommand, dispatcher::DispatchError, dispatcher::EngineDispatcher,
};

use crate::runtime_stress::stats::{DispatchStats, ResultStats};

pub(super) fn dispatch_commands(
    dispatcher: &EngineDispatcher,
    symbol: &str,
    commands: Vec<EngineCommand>,
    stats: &mut DispatchStats,
) {
    for command in commands {
        dispatch_command(dispatcher, symbol, command, stats);
    }
}

pub(super) fn dispatch_command(
    dispatcher: &EngineDispatcher,
    symbol: &str,
    command: EngineCommand,
    stats: &mut DispatchStats,
) {
    match dispatcher.dispatch(symbol, command) {
        Ok(()) => stats.accepted += 1,
        Err(DispatchError::ChannelFull { .. }) => stats.channel_full += 1,
        Err(DispatchError::UnknownSymbol { .. }) => stats.unknown_symbol += 1,
        Err(DispatchError::EngineStopped { .. }) => stats.engine_stopped += 1,
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

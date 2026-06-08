use std::time::Duration;

use matching_engine::engine::command::EngineCommand;

use crate::runtime_stress::{config::Scenario, workload::WorkloadGenerator};

#[derive(Default)]
pub(super) struct CommandCursor {
    generator: WorkloadGenerator,
    pending_commands: Vec<EngineCommand>,
    pending_symbol: String,
    symbol_index: usize,
}

impl CommandCursor {
    pub(super) fn next_command(
        &mut self,
        symbols: &[String],
        scenario: Scenario,
        sweep_depth: usize,
    ) -> Option<(String, EngineCommand)> {
        if self.pending_commands.is_empty() {
            let symbol = &symbols[self.symbol_index % symbols.len()];
            self.symbol_index += 1;
            self.pending_symbol.clear();
            self.pending_symbol.push_str(symbol);
            self.pending_commands = self.generator.make_workload(scenario, sweep_depth, symbol);
            self.pending_commands.reverse();
        }

        self.pending_commands
            .pop()
            .map(|command| (self.pending_symbol.clone(), command))
    }
}

pub(super) fn command_interval(target_commands_per_sec: u64) -> Duration {
    Duration::from_secs_f64(1.0 / target_commands_per_sec as f64)
}

use std::time::Duration;

use crate::runtime_stress::runner_pacing::command_interval;

#[test]
fn command_interval_uses_target_command_rate() {
    assert_eq!(command_interval(100), Duration::from_secs_f64(1.0 / 100.0));
}

#[test]
fn command_interval_scales_with_higher_rates() {
    assert!(command_interval(1_000) < command_interval(100));
}

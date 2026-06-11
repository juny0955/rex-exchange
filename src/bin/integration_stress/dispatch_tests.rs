use std::time::{Duration, Instant};

use crate::integration_stress::{
    config::Scenario, dispatch::PacingLag, workload::commands_per_workload,
};

#[test]
fn pacing_lag_records_cumulative_and_max_delay() {
    let base = Instant::now();
    let mut lag = PacingLag::default();

    lag.record(
        base + Duration::from_millis(25),
        base + Duration::from_millis(10),
        11,
    );
    lag.record(
        base + Duration::from_millis(70),
        base + Duration::from_millis(40),
        22,
    );

    assert_eq!(lag.events, 2);
    assert_eq!(lag.total, Duration::from_millis(45));
    assert_eq!(lag.max, Duration::from_millis(30));
}

#[test]
fn pacing_lag_ignores_first_attempt_and_early_ticks() {
    let base = Instant::now();
    let mut lag = PacingLag::default();

    lag.record(
        base + Duration::from_millis(25),
        base + Duration::from_millis(10),
        0,
    );
    lag.record(
        base + Duration::from_millis(30),
        base + Duration::from_millis(40),
        11,
    );

    assert_eq!(lag.events, 0);
    assert_eq!(lag.total, Duration::ZERO);
    assert_eq!(lag.max, Duration::ZERO);
}

#[test]
fn commands_per_workload_matches_sweep_scenarios() {
    assert_eq!(commands_per_workload(Scenario::MarketQuoteSweep, 10), 11);
    assert_eq!(commands_per_workload(Scenario::FullFillSameLevel, 10), 11);
    assert_eq!(commands_per_workload(Scenario::CancelRestingOrder, 10), 20);
    assert_eq!(commands_per_workload(Scenario::AmendDecreaseQty, 10), 3);
}

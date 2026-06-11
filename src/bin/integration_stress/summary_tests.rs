use std::time::Duration;

use crate::integration_stress::{
    config::{Config, RunMode},
    metrics::CorrelatorSummary,
    runner::{DispatchSnapshot, RunReport},
    summary::render_report,
};

fn report(attempted: usize, accepted: u64, elapsed: Duration) -> RunReport {
    RunReport {
        attempted,
        dispatch: DispatchSnapshot {
            accepted,
            rejected: 0,
            resource_exhausted: 0,
            error: 0,
        },
        dispatch_elapsed: elapsed,
        total_elapsed: elapsed,
        settled: true,
        pacing_lag_events: 1,
        pacing_lag: Duration::from_secs(3),
        pacing_lag_max: Duration::from_millis(250),
        correlator: CorrelatorSummary {
            sent: accepted,
            received: accepted,
            unique: accepted,
            duplicates: 0,
            matched: accepted,
            missing: 0,
            unmatched_recv: 0,
            latency: Default::default(),
        },
    }
}

#[test]
fn paced_report_shows_target_achievement_when_generator_misses_target() {
    let config = Config {
        mode: RunMode::Paced {
            warmup: Duration::from_secs(5),
            duration: Duration::from_secs(30),
            target_commands_per_sec: 20_000,
        },
        ..Config::default()
    };

    let output = render_report(&config, &report(503_140, 503_140, Duration::from_secs(30)));

    assert!(output.contains("목표 명령 수"));
    assert!(output.contains("600,000"));
    assert!(output.contains("target 달성률"));
    assert!(output.contains("83.86%"));
    assert!(output.contains("target 달성 여부"));
    assert!(output.contains("아니오"));
}

#[test]
fn report_uses_correlator_settle_fields_for_lossless_verdict() {
    let config = Config::default();
    let output = render_report(&config, &report(100, 100, Duration::from_secs(1)));

    assert!(output.contains("송신 알림 처리"));
    assert!(output.contains("상관 매칭"));
    assert!(output.contains("무손실 판정"));
    assert!(output.contains("통과"));
    assert!(output.contains("pacing 지연 누적"));
    assert!(output.contains("pacing 최대 지연"));
}

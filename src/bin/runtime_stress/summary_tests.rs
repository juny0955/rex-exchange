use std::time::Duration;

use crate::runtime_stress::{
    config::{Config, RunMode},
    stats::{DispatchStats, PhaseStats, ResultStatsSnapshot},
    summary::{render_burst_summary, render_paced_summary},
    summary_format::{completion_label, format_count, format_duration_display},
};

#[test]
fn completion_label_returns_korean_label() {
    assert_eq!(completion_label(true), "예");
    assert_eq!(completion_label(false), "아니오");
}

#[test]
fn format_count_uses_thousands_separator() {
    assert_eq!(format_count(0), "0");
    assert_eq!(format_count(1_000), "1,000");
    assert_eq!(format_count(1_234_567), "1,234,567");
}

#[test]
fn format_duration_display_uses_ms_or_seconds() {
    assert_eq!(
        format_duration_display(Duration::from_micros(123_456)),
        "123.456ms"
    );
    assert_eq!(
        format_duration_display(Duration::from_millis(1_234)),
        "1.234s"
    );
}

#[test]
fn render_burst_summary_groups_results_into_readable_sections() {
    let config = Config {
        mode: RunMode::Burst { orders: 100_000 },
        ..Config::default()
    };
    let result = PhaseStats {
        attempted_commands: 1_100_000,
        dispatch: DispatchStats {
            accepted: 1_100_000,
            channel_full: 0,
            unknown_symbol: 0,
            engine_stopped: 0,
        },
        result: ResultStatsSnapshot {
            published: 1_100_000,
            place_results: 1_100_000,
            cancel_results: 0,
            amend_results: 0,
            trades: 1_000_000,
            maker_updates: 1_000_000,
        },
        completed: true,
        dispatch_elapsed: Duration::from_micros(123_456),
        total_elapsed: Duration::from_micros(456_789),
        pacing_lag_events: 0,
        pacing_lag: Duration::ZERO,
    };

    let output = render_burst_summary(&config, 100_000, &result);

    assert!(output.contains("================================"));
    assert!(output.contains("[실행 조건]"));
    assert!(output.contains("[접수 결과]"));
    assert!(output.contains("[발행 결과]"));
    assert!(output.contains("[처리량]"));
    assert!(output.contains("동일 호가 전량 체결 (full-fill-same-level)"));
    assert!(output.contains("워크로드 반복 수   100,000"));
    assert!(output.contains("시도한 명령 수     1,100,000"));
    assert!(output.contains("접수 소요 시간     123.456ms"));
    assert!(output.contains("초당 접수 성공 수  "));
}

#[test]
fn render_paced_summary_splits_warmup_and_measurement() {
    let config = Config {
        mode: RunMode::Paced {
            warmup: Duration::from_secs(1),
            duration: Duration::from_secs(2),
            target_commands_per_sec: 100,
        },
        ..Config::default()
    };
    let phase = PhaseStats {
        attempted_commands: 100,
        completed: true,
        dispatch_elapsed: Duration::from_secs(1),
        total_elapsed: Duration::from_secs(1),
        ..PhaseStats::default()
    };

    let output = render_paced_summary(
        &config,
        100,
        Duration::from_secs(1),
        &phase,
        Duration::from_secs(2),
        Some(&phase),
    );

    assert!(output.contains("런타임 부하 테스트 결과"));
    assert!(output.contains("[warm-up 결과]"));
    assert!(output.contains("[측정 결과]"));
    assert!(output.contains("목표 명령 수/s     100"));
}

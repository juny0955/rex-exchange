use std::time::Duration;

use crate::runtime_stress::config::{Config, RunMode, Scenario, parse_config};

fn parse(args: &[&str]) -> Result<Config, String> {
    parse_config(args.iter().map(|arg| arg.to_string()))
}

#[test]
fn parse_config_uses_burst_defaults() {
    assert_eq!(
        parse(&[]).unwrap(),
        Config {
            scenario: Scenario::FullFillSameLevel,
            ..Config::default()
        }
    );
}

#[test]
fn parse_config_accepts_existing_burst_options() {
    let config = parse(&[
        "--scenario",
        "full-fill-same-level",
        "--orders",
        "42",
        "--symbols",
        "4",
        "--sweep-depth",
        "7",
        "--publisher-delay-ms",
        "3",
        "--timeout-sec",
        "5",
    ])
    .unwrap();

    assert_eq!(
        config,
        Config {
            scenario: Scenario::FullFillSameLevel,
            mode: RunMode::Burst { orders: 42 },
            symbols: 4,
            sweep_depth: 7,
            publisher_delay: Duration::from_millis(3),
            timeout: Duration::from_secs(5),
        }
    );
}

#[test]
fn parse_config_accepts_paced_options() {
    let config = parse(&[
        "--duration-sec",
        "30",
        "--warmup-sec",
        "10",
        "--target-commands-per-sec",
        "1000",
    ])
    .unwrap();

    assert_eq!(
        config.mode,
        RunMode::Paced {
            warmup: Duration::from_secs(10),
            duration: Duration::from_secs(30),
            target_commands_per_sec: 1000,
        }
    );
}

#[test]
fn parse_config_rejects_orders_with_duration() {
    assert_eq!(
        parse(&[
            "--orders",
            "10",
            "--duration-sec",
            "1",
            "--target-commands-per-sec",
            "10",
        ])
        .unwrap_err(),
        "--orders와 --duration-sec는 함께 사용할 수 없습니다"
    );
}

#[test]
fn parse_config_rejects_duration_without_target_rate() {
    assert_eq!(
        parse(&["--duration-sec", "1"]).unwrap_err(),
        "--duration-sec를 사용할 때는 --target-commands-per-sec가 필요합니다"
    );
}

#[test]
fn parse_config_rejects_target_rate_without_duration() {
    assert_eq!(
        parse(&["--target-commands-per-sec", "10"]).unwrap_err(),
        "--target-commands-per-sec는 --duration-sec와 함께 사용해야 합니다"
    );
}

#[test]
fn parse_config_rejects_warmup_without_duration() {
    assert_eq!(
        parse(&["--warmup-sec", "10"]).unwrap_err(),
        "--warmup-sec는 --duration-sec와 함께 사용해야 합니다"
    );
}

#[test]
fn parse_config_rejects_zero_values() {
    assert_eq!(
        parse(&["--symbols", "0"]).unwrap_err(),
        "--symbols 값은 0보다 커야 합니다"
    );
    assert_eq!(
        parse(&["--sweep-depth", "0"]).unwrap_err(),
        "--sweep-depth 값은 0보다 커야 합니다"
    );
    assert_eq!(
        parse(&["--duration-sec", "0"]).unwrap_err(),
        "--duration-sec 값은 0보다 커야 합니다"
    );
    assert_eq!(
        parse(&["--duration-sec", "1", "--target-commands-per-sec", "0"]).unwrap_err(),
        "--target-commands-per-sec 값은 0보다 커야 합니다"
    );
}

#[test]
fn parse_config_rejects_values_above_safety_limits() {
    assert_eq!(
        parse(&["--symbols", "1025"]).unwrap_err(),
        "--symbols 값은 1024 이하여야 합니다"
    );
    assert_eq!(
        parse(&["--sweep-depth", "10001"]).unwrap_err(),
        "--sweep-depth 값은 10000 이하여야 합니다"
    );
    assert_eq!(
        parse(&[
            "--duration-sec",
            "86401",
            "--target-commands-per-sec",
            "100"
        ])
        .unwrap_err(),
        "--duration-sec 값은 86400 이하여야 합니다"
    );
    assert_eq!(
        parse(&[
            "--duration-sec",
            "1",
            "--target-commands-per-sec",
            "1000001"
        ])
        .unwrap_err(),
        "--target-commands-per-sec 값은 1000000 이하여야 합니다"
    );
}

#[test]
fn parse_config_rejects_missing_value_in_korean() {
    assert_eq!(
        parse(&["--orders"]).unwrap_err(),
        "--orders 값이 누락되었습니다"
    );
}

#[test]
fn scenario_parse_accepts_matching_scenarios() {
    assert_eq!(
        Scenario::parse("market-quote-sweep").unwrap(),
        Scenario::MarketQuoteSweep
    );
    assert_eq!(
        Scenario::parse("partial-fill-rest").unwrap(),
        Scenario::PartialFillRest
    );
    assert_eq!(
        Scenario::parse("cancel-resting-order").unwrap(),
        Scenario::CancelRestingOrder
    );
    assert_eq!(
        Scenario::parse("amend-decrease-qty").unwrap(),
        Scenario::AmendDecreaseQty
    );
    assert_eq!(
        Scenario::parse("amend-price-change").unwrap(),
        Scenario::AmendPriceChange
    );
}

#[test]
fn scenario_display_name_returns_korean_label() {
    assert_eq!(
        Scenario::CancelMissing.display_name(),
        "기본 경로 확인(미존재 취소)"
    );
    assert_eq!(
        Scenario::PlaceRestingLimit.display_name(),
        "미체결 지정가 잔존"
    );
    assert_eq!(
        Scenario::FullFillSameLevel.display_name(),
        "동일 호가 전량 체결"
    );
    assert_eq!(
        Scenario::MarketQuoteSweep.display_name(),
        "시장가 금액 스윕"
    );
    assert_eq!(
        Scenario::PartialFillRest.display_name(),
        "부분 체결 후 잔존"
    );
    assert_eq!(
        Scenario::CancelRestingOrder.display_name(),
        "잔존 주문 취소"
    );
    assert_eq!(Scenario::AmendDecreaseQty.display_name(), "수량 감소 정정");
    assert_eq!(Scenario::AmendPriceChange.display_name(), "가격 변경 정정");
}

#[test]
fn scenario_parse_rejects_unknown_scenario_in_korean() {
    assert_eq!(
        Scenario::parse("invalid").unwrap_err(),
        "지원하지 않는 시나리오: invalid"
    );
}

#[test]
fn scenario_token_returns_cli_value() {
    assert_eq!(Scenario::FullFillSameLevel.token(), "full-fill-same-level");
    assert_eq!(Scenario::CancelMissing.token(), "cancel-missing");
    assert_eq!(Scenario::CancelRestingOrder.token(), "cancel-resting-order");
    assert_eq!(Scenario::AmendDecreaseQty.token(), "amend-decrease-qty");
    assert_eq!(Scenario::AmendPriceChange.token(), "amend-price-change");
}

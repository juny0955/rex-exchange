use std::time::Duration;

use crate::integration_stress::config::{Config, RunMode, Scenario, parse_config};

fn parse(args: &[&str]) -> Result<Config, String> {
    parse_config(args.iter().map(|s| s.to_string()))
}

#[test]
fn 기본값은_burst_모드() {
    let config = parse(&[]).unwrap();
    assert_eq!(config.mode, RunMode::Burst { orders: 100_000 });
    assert_eq!(config.grpc_endpoint, "http://localhost:50051");
    assert_eq!(config.kafka_topic, "matching-engine-events");
}

#[test]
fn 시나리오와_엔드포인트_파싱() {
    let config = parse(&[
        "--scenario",
        "place-resting-limit",
        "--grpc-endpoint",
        "http://10.0.0.1:6000",
        "--kafka-brokers",
        "broker:9092",
        "--concurrency",
        "128",
    ])
    .unwrap();

    assert_eq!(config.scenario, Scenario::PlaceRestingLimit);
    assert_eq!(config.grpc_endpoint, "http://10.0.0.1:6000");
    assert_eq!(config.kafka_brokers, "broker:9092");
    assert_eq!(config.concurrency, 128);
}

#[test]
fn connections_기본값은_8() {
    let config = parse(&[]).unwrap();
    assert_eq!(config.connections, 8);
}

#[test]
fn connections_파싱() {
    let config = parse(&["--connections", "64"]).unwrap();
    assert_eq!(config.connections, 64);
}

#[test]
fn connections_상한_초과는_에러() {
    let err = parse(&["--connections", "2000"]).unwrap_err();
    assert!(err.contains("--connections"));
}

#[test]
fn paced_모드는_target이_필요() {
    let err = parse(&["--duration-sec", "10"]).unwrap_err();
    assert!(err.contains("--target-commands-per-sec"));
}

#[test]
fn paced_모드_정상_파싱() {
    let config = parse(&[
        "--duration-sec",
        "30",
        "--target-commands-per-sec",
        "5000",
        "--warmup-sec",
        "5",
    ])
    .unwrap();

    assert_eq!(
        config.mode,
        RunMode::Paced {
            warmup: Duration::from_secs(5),
            duration: Duration::from_secs(30),
            target_commands_per_sec: 5000,
        }
    );
}

#[test]
fn orders와_duration은_동시_사용_불가() {
    let err = parse(&[
        "--orders",
        "1000",
        "--duration-sec",
        "10",
        "--target-commands-per-sec",
        "100",
    ])
    .unwrap_err();
    assert!(err.contains("함께 사용할 수 없습니다"));
}

#[test]
fn warmup은_duration_없이_사용_불가() {
    let err = parse(&["--warmup-sec", "5"]).unwrap_err();
    assert!(err.contains("--warmup-sec"));
}

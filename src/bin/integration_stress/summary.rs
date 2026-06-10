//! 통합 부하 결과 출력.

use std::time::Duration;

use crate::integration_stress::{
    config::{Config, RunMode},
    metrics::LatencySummary,
    runner::RunReport,
};

pub fn print_usage() {
    eprintln!(
        "사용법: cargo run --release --bin integration_stress -- \\\n\
         \x20 [--scenario cancel-missing|place-resting-limit|full-fill-same-level|market-quote-sweep|partial-fill-rest|cancel-resting-order|amend-decrease-qty|amend-price-change] \\\n\
         \x20 [--orders N] \\\n\
         \x20 [--duration-sec N --target-commands-per-sec N [--warmup-sec N]] \\\n\
         \x20 [--symbols N] [--sweep-depth N] [--concurrency N] \\\n\
         \x20 [--timeout-sec N] [--settle-timeout-sec N] \\\n\
         \x20 [--grpc-endpoint http://localhost:50051] \\\n\
         \x20 [--kafka-brokers localhost:9092] [--kafka-topic matching-engine-events]\n\
         기본 모드: --orders 기반 burst\n\
         부하 모드: --duration-sec와 --target-commands-per-sec를 함께 지정(선택 --warmup-sec)\n\
         사전 준비: docker compose up -d (Kafka), cargo run --release --bin matching-engine (SUT)"
    );
}

pub fn print_report(config: &Config, report: &RunReport) {
    print!("{}", render_report(config, report));
}

fn render_report(config: &Config, report: &RunReport) -> String {
    let mut out = String::new();

    push_header(&mut out, "gRPC + Kafka 통합 부하 테스트 결과");
    push_conditions(&mut out, config);
    out.push('\n');

    push_section(&mut out, "[gRPC 접수]");
    let d = &report.dispatch;
    push_row(&mut out, "시도한 명령 수", &count(report.attempted as u64));
    push_row(&mut out, "접수 성공(SUBMITTED)", &count(d.accepted));
    push_row(&mut out, "채널 포화(503)", &count(d.resource_exhausted));
    push_row(&mut out, "거부(REJECTED)", &count(d.rejected));
    push_row(&mut out, "기타 에러", &count(d.error));
    push_row(
        &mut out,
        "접수 성공률",
        &format!("{:.2}%", percentage(d.accepted, report.attempted as u64)),
    );
    if report.pacing_lag_events > 0 {
        push_row(&mut out, "pacing 지연 횟수", &count(report.pacing_lag_events as u64));
        push_row(&mut out, "pacing 지연 합계", &dur(report.pacing_lag));
    }
    out.push('\n');

    push_section(&mut out, "[Kafka 수신 / 정합성]");
    let c = &report.correlator;
    push_row(&mut out, "수신 이벤트(중복 포함)", &count(c.received));
    push_row(&mut out, "고유 이벤트", &count(c.unique));
    push_row(&mut out, "상관 매칭", &count(c.matched));
    push_row(&mut out, "누락(접수했으나 미수신)", &count(c.missing));
    push_row(&mut out, "중복 수신", &count(c.duplicates));
    push_row(&mut out, "미상관 수신", &count(c.unmatched_recv));
    push_row(&mut out, "settle 완료", yes_no(report.settled));
    push_row(&mut out, "무손실 판정", verdict(report));
    out.push('\n');

    push_section(&mut out, "[E2E 지연 (gRPC 송신 → Kafka 수신)]");
    push_latency(&mut out, &c.latency);
    out.push('\n');

    push_section(&mut out, "[처리량]");
    push_row(&mut out, "접수 소요 시간", &dur(report.dispatch_elapsed));
    push_row(&mut out, "전체 소요 시간(접수+settle)", &dur(report.total_elapsed));
    push_row(
        &mut out,
        "초당 접수 성공 수",
        &rate(per_sec(d.accepted, report.dispatch_elapsed)),
    );
    push_row(
        &mut out,
        "초당 종단 수신 수(E2E)",
        &rate(per_sec(c.received, report.total_elapsed)),
    );

    out
}

fn push_conditions(out: &mut String, config: &Config) {
    push_section(out, "[실행 조건]");
    push_row(
        out,
        "시나리오",
        &format!("{} ({})", config.scenario.display_name(), config.scenario.token()),
    );
    push_row(out, "실행 모드", &mode_label(&config.mode));
    push_row(out, "심볼 수", &count(config.symbols as u64));
    push_row(out, "스윕 깊이", &count(config.sweep_depth as u64));
    push_row(out, "동시 실행(units)", &count(config.concurrency as u64));
    push_row(out, "gRPC 엔드포인트", &config.grpc_endpoint);
    push_row(out, "Kafka 브로커", &config.kafka_brokers);
    push_row(out, "Kafka 토픽", &config.kafka_topic);
    push_row(out, "settle 제한 시간", &format!("{}s", config.settle_timeout.as_secs()));
}

fn push_latency(out: &mut String, latency: &LatencySummary) {
    if latency.count == 0 {
        out.push_str("  측정된 지연 표본이 없습니다\n");
        return;
    }

    push_row(out, "표본 수", &count(latency.count));
    push_row(out, "min", &micros(latency.min_us));
    push_row(out, "p50", &micros(latency.p50_us));
    push_row(out, "p90", &micros(latency.p90_us));
    push_row(out, "p99", &micros(latency.p99_us));
    push_row(out, "p99.9", &micros(latency.p999_us));
    push_row(out, "max", &micros(latency.max_us));
    push_row(out, "mean", &micros(latency.mean_us));
}

fn mode_label(mode: &RunMode) -> String {
    match mode {
        RunMode::Burst { orders } => format!("burst (orders={})", count(*orders as u64)),
        RunMode::Paced {
            warmup,
            duration,
            target_commands_per_sec,
        } => format!(
            "paced (target={}/s, duration={}, warmup={})",
            count(*target_commands_per_sec),
            dur(*duration),
            dur(*warmup),
        ),
    }
}

fn verdict(report: &RunReport) -> &'static str {
    let c = &report.correlator;
    if report.settled
        && c.missing == 0
        && c.duplicates == 0
        && c.unmatched_recv == 0
        && c.unique == report.dispatch.accepted
    {
        "통과 (누락·중복 없음)"
    } else {
        "실패 (누락/중복/미정착 확인 필요)"
    }
}

fn push_header(out: &mut String, title: &str) {
    out.push_str("================================\n");
    out.push_str(title);
    out.push('\n');
    out.push_str("================================\n\n");
}

fn push_section(out: &mut String, title: &str) {
    out.push_str(title);
    out.push('\n');
}

fn push_row(out: &mut String, label: &str, value: &str) {
    out.push_str(&format!("  {label:<28}: {value}\n"));
}

fn yes_no(value: bool) -> &'static str {
    if value { "예" } else { "아니오" }
}

fn count(value: u64) -> String {
    let digits = value.to_string();
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let len = bytes.len();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

fn micros(us: u64) -> String {
    if us >= 1_000_000 {
        format!("{:.3}s", us as f64 / 1_000_000.0)
    } else if us >= 1_000 {
        format!("{:.3}ms", us as f64 / 1_000.0)
    } else {
        format!("{us}µs")
    }
}

fn dur(d: Duration) -> String {
    let millis = d.as_millis();
    if millis >= 1000 {
        format!("{:.3}s", d.as_secs_f64())
    } else {
        format!("{millis}ms")
    }
}

fn rate(value: f64) -> String {
    format!("{value:.1}/s")
}

fn per_sec(count: u64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs <= 0.0 {
        return 0.0;
    }
    count as f64 / secs
}

fn percentage(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    part as f64 / whole as f64 * 100.0
}

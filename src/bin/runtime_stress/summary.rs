use std::time::Duration;

use crate::runtime_stress::{
    config::Config,
    latency::LatencySummary,
    stats::PhaseStats,
    summary_format::{
        completion_label, format_count, format_duration_display, format_micros, format_rate,
        per_sec, percentage, push_row,
    },
};

pub fn print_burst_summary(config: &Config, orders: usize, result: &PhaseStats) {
    print!("{}", render_burst_summary(config, orders, result));
}

pub fn print_paced_summary(
    config: &Config,
    target_commands_per_sec: u64,
    warmup: Duration,
    warmup_result: &PhaseStats,
    duration: Duration,
    measurement_result: Option<&PhaseStats>,
) {
    print!(
        "{}",
        render_paced_summary(
            config,
            target_commands_per_sec,
            warmup,
            warmup_result,
            duration,
            measurement_result,
        )
    );
}

pub fn print_usage() {
    eprintln!(
        "사용법: cargo run --release --bin runtime_stress -- \
         [--scenario cancel-missing|place-resting-limit|full-fill-same-level|market-quote-sweep|partial-fill-rest|cancel-resting-order|amend-decrease-qty|amend-price-change] \
         [--orders N] \
         [--duration-sec N --target-commands-per-sec N] \
         [--warmup-sec N] \
         [--symbols N] \
         [--sweep-depth N] \
         [--publisher-delay-ms N] \
         [--timeout-sec N]\n\
         기본 모드: --orders 기반 burst/스트레스 모드\n\
         시간 기반 부하 모드: --duration-sec와 --target-commands-per-sec를 함께 지정"
    );
}

pub(super) fn render_burst_summary(config: &Config, orders: usize, result: &PhaseStats) -> String {
    let mut output = String::new();

    push_header(&mut output, "런타임 스트레스 테스트 결과");
    push_common_conditions(&mut output, config);
    push_row(&mut output, "실행 모드", "burst");
    push_row(&mut output, "워크로드 반복 수", &format_count(orders));
    push_row(
        &mut output,
        "시도한 명령 수",
        &format_count(result.attempted_commands),
    );
    output.push('\n');

    push_phase_sections(&mut output, result);

    output
}

pub(super) fn render_paced_summary(
    config: &Config,
    target_commands_per_sec: u64,
    warmup: Duration,
    warmup_result: &PhaseStats,
    duration: Duration,
    measurement_result: Option<&PhaseStats>,
) -> String {
    let mut output = String::new();

    push_header(&mut output, "런타임 부하 테스트 결과");
    push_common_conditions(&mut output, config);
    push_row(&mut output, "실행 모드", "paced");
    push_row(
        &mut output,
        "목표 명령 수/s",
        &format_count(target_commands_per_sec as usize),
    );
    push_row(
        &mut output,
        "warm-up 시간",
        &format_duration_display(warmup),
    );
    push_row(&mut output, "측정 시간", &format_duration_display(duration));
    output.push('\n');

    output.push_str("[warm-up 결과]\n");
    push_phase_body(&mut output, warmup_result);
    output.push('\n');

    output.push_str("[측정 결과]\n");
    match measurement_result {
        Some(result) => push_phase_body(&mut output, result),
        None => output.push_str("  warm-up 미완료로 측정 구간을 실행하지 않았습니다\n"),
    }

    output
}

fn push_header(output: &mut String, title: &str) {
    output.push_str("================================\n");
    output.push_str(title);
    output.push('\n');
    output.push_str("================================\n\n");
}

fn push_common_conditions(output: &mut String, config: &Config) {
    output.push_str("[실행 조건]\n");
    push_row(
        output,
        "시나리오",
        &format!(
            "{} ({})",
            config.scenario.display_name(),
            config.scenario.token()
        ),
    );
    push_row(output, "심볼 수", &format_count(config.symbols));
    push_row(output, "스윕 깊이", &format_count(config.sweep_depth));
    push_row(
        output,
        "발행 지연",
        &format!("{}ms", config.publisher_delay.as_millis()),
    );
    push_row(
        output,
        "제한 시간",
        &format!("{}s", config.timeout.as_secs()),
    );
}

fn push_phase_sections(output: &mut String, result: &PhaseStats) {
    output.push_str("[접수 결과]\n");
    push_dispatch_rows(output, result);
    output.push('\n');

    output.push_str("[발행 결과]\n");
    push_result_rows(output, result);
    output.push('\n');

    output.push_str("[처리량]\n");
    push_throughput_rows(output, result);
    output.push('\n');

    output.push_str("[지연 분포]\n");
    push_latency_rows(output, &result.latency);
}

fn push_phase_body(output: &mut String, result: &PhaseStats) {
    push_row(
        output,
        "시도한 명령 수",
        &format_count(result.attempted_commands),
    );
    push_dispatch_rows(output, result);
    push_result_rows(output, result);
    push_throughput_rows(output, result);
    push_row(
        output,
        "pacing 지연 횟수",
        &format_count(result.pacing_lag_events),
    );
    push_row(
        output,
        "pacing 지연 합계",
        &format_duration_display(result.pacing_lag),
    );
    push_latency_rows(output, &result.latency);
}

fn push_latency_rows(output: &mut String, latency: &LatencySummary) {
    if latency.count == 0 {
        output.push_str("  측정된 지연 표본이 없습니다\n");
        return;
    }

    push_row(
        output,
        "지연 표본 수",
        &format_count(latency.count as usize),
    );
    push_row(output, "지연 min", &format_micros(latency.min_us));
    push_row(output, "지연 mean", &format_micros(latency.mean_us));
    push_row(output, "지연 p50", &format_micros(latency.p50_us));
    push_row(output, "지연 p95", &format_micros(latency.p95_us));
    push_row(output, "지연 p99", &format_micros(latency.p99_us));
    push_row(output, "지연 max", &format_micros(latency.max_us));
}

fn push_dispatch_rows(output: &mut String, result: &PhaseStats) {
    push_row(output, "접수 성공", &format_count(result.dispatch.accepted));
    push_row(
        output,
        "채널 포화",
        &format_count(result.dispatch.channel_full),
    );
    push_row(
        output,
        "미등록 심볼",
        &format_count(result.dispatch.unknown_symbol),
    );
    push_row(
        output,
        "엔진 중지",
        &format_count(result.dispatch.engine_stopped),
    );
    push_row(
        output,
        "거부 합계",
        &format_count(result.dispatch.rejected()),
    );
    push_row(
        output,
        "접수 성공률",
        &format!(
            "{:.2}%",
            percentage(result.dispatch.accepted, result.attempted_commands)
        ),
    );
}

fn push_result_rows(output: &mut String, result: &PhaseStats) {
    push_row(output, "발행 결과", &format_count(result.result.published));
    push_row(
        output,
        "주문 접수 결과",
        &format_count(result.result.place_results),
    );
    push_row(
        output,
        "주문 취소 결과",
        &format_count(result.result.cancel_results),
    );
    push_row(
        output,
        "주문 정정 결과",
        &format_count(result.result.amend_results),
    );
    push_row(output, "체결 수", &format_count(result.result.trades));
    push_row(
        output,
        "메이커 갱신 수",
        &format_count(result.result.maker_updates),
    );
    push_row(output, "완료 여부", completion_label(result.completed));
}

fn push_throughput_rows(output: &mut String, result: &PhaseStats) {
    push_row(
        output,
        "접수 소요 시간",
        &format_duration_display(result.dispatch_elapsed),
    );
    push_row(
        output,
        "전체 소요 시간",
        &format_duration_display(result.total_elapsed),
    );
    push_row(
        output,
        "실제 시도 수/s",
        &format_rate(per_sec(result.attempted_commands, result.dispatch_elapsed)),
    );
    push_row(
        output,
        "초당 접수 성공 수",
        &format_rate(per_sec(result.dispatch.accepted, result.dispatch_elapsed)),
    );
    push_row(
        output,
        "초당 발행 결과 수",
        &format_rate(per_sec(result.result.published, result.total_elapsed)),
    );
}

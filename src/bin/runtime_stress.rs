use std::{
    env,
    fmt::Write as _,
    process,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime},
};

use chrono::{DateTime, Utc};
use matching_engine::{
    domain::order::{Order, OrderSize, OrderStatus, OrderType, Side, TimeInForce},
    engine::{
        command::EngineCommand,
        dispatcher::DispatchError,
        result::EngineResult,
        result_handler::{EngineResultPublisher, PublishResult},
        runtime::EngineRuntime,
    },
};
use rust_decimal::Decimal;
use uuid::Uuid;

const DEFAULT_SCENARIO: Scenario = Scenario::FullFillSameLevel;
const DEFAULT_ORDERS: usize = 100_000;
const DEFAULT_SYMBOLS: usize = 1;
const DEFAULT_SWEEP_DEPTH: usize = 10;
const DEFAULT_PUBLISHER_DELAY_MS: u64 = 0;
const DEFAULT_TIMEOUT_SEC: u64 = 30;
const BASE_SYMBOL: &str = "BTCUSDT";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    CancelMissing,
    PlaceRestingLimit,
    FullFillSameLevel,
    MarketQuoteSweep,
    PartialFillRest,
}

impl Scenario {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "cancel-missing" => Ok(Self::CancelMissing),
            "place-resting-limit" => Ok(Self::PlaceRestingLimit),
            "full-fill-same-level" => Ok(Self::FullFillSameLevel),
            "market-quote-sweep" => Ok(Self::MarketQuoteSweep),
            "partial-fill-rest" => Ok(Self::PartialFillRest),
            _ => Err(format!("지원하지 않는 시나리오: {value}")),
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::CancelMissing => "기본 경로 확인(미존재 취소)",
            Self::PlaceRestingLimit => "미체결 지정가 잔존",
            Self::FullFillSameLevel => "동일 호가 전량 체결",
            Self::MarketQuoteSweep => "시장가 금액 스윕",
            Self::PartialFillRest => "부분 체결 후 잔존",
        }
    }

    fn token(self) -> &'static str {
        match self {
            Self::CancelMissing => "cancel-missing",
            Self::PlaceRestingLimit => "place-resting-limit",
            Self::FullFillSameLevel => "full-fill-same-level",
            Self::MarketQuoteSweep => "market-quote-sweep",
            Self::PartialFillRest => "partial-fill-rest",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Config {
    scenario: Scenario,
    orders: usize,
    symbols: usize,
    sweep_depth: usize,
    publisher_delay: Duration,
    timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scenario: DEFAULT_SCENARIO,
            orders: DEFAULT_ORDERS,
            symbols: DEFAULT_SYMBOLS,
            sweep_depth: DEFAULT_SWEEP_DEPTH,
            publisher_delay: Duration::from_millis(DEFAULT_PUBLISHER_DELAY_MS),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SEC),
        }
    }
}

#[derive(Default)]
struct DispatchStats {
    accepted: usize,
    channel_full: usize,
    unknown_symbol: usize,
    engine_stopped: usize,
}

impl DispatchStats {
    fn rejected(&self) -> usize {
        self.channel_full + self.unknown_symbol + self.engine_stopped
    }
}

#[derive(Default)]
struct ResultStats {
    published: AtomicUsize,
    place_results: AtomicUsize,
    cancel_results: AtomicUsize,
    amend_results: AtomicUsize,
    trades: AtomicUsize,
    maker_updates: AtomicUsize,
}

impl ResultStats {
    fn snapshot(&self) -> ResultStatsSnapshot {
        ResultStatsSnapshot {
            published: self.published.load(Ordering::Relaxed),
            place_results: self.place_results.load(Ordering::Relaxed),
            cancel_results: self.cancel_results.load(Ordering::Relaxed),
            amend_results: self.amend_results.load(Ordering::Relaxed),
            trades: self.trades.load(Ordering::Relaxed),
            maker_updates: self.maker_updates.load(Ordering::Relaxed),
        }
    }
}

struct ResultStatsSnapshot {
    published: usize,
    place_results: usize,
    cancel_results: usize,
    amend_results: usize,
    trades: usize,
    maker_updates: usize,
}

struct CountingPublisher {
    stats: Arc<ResultStats>,
    delay: Duration,
}

impl EngineResultPublisher for CountingPublisher {
    fn publish(&self, _result: &EngineResult) -> PublishResult {
        if !self.delay.is_zero() {
            thread::sleep(self.delay);
        }

        self.stats.published.fetch_add(1, Ordering::Relaxed);
        match _result {
            EngineResult::Place(result) => {
                self.stats.place_results.fetch_add(1, Ordering::Relaxed);
                self.stats
                    .trades
                    .fetch_add(result.trades.len(), Ordering::Relaxed);
                self.stats
                    .maker_updates
                    .fetch_add(result.updated_makers.len(), Ordering::Relaxed);
            }
            EngineResult::Cancel(_) => {
                self.stats.cancel_results.fetch_add(1, Ordering::Relaxed);
            }
            EngineResult::Amend(_) => {
                self.stats.amend_results.fetch_add(1, Ordering::Relaxed);
            }
        }

        Ok(())
    }
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    let config = match parse_config(args) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("오류: {message}");
            print_usage();
            process::exit(2);
        }
    };

    run(config);
}

fn run(config: Config) {
    let symbols = make_symbols(config.symbols);
    let result_stats = Arc::new(ResultStats::default());

    let runtime = EngineRuntime::new(
        symbols.clone(),
        Box::new(CountingPublisher {
            stats: Arc::clone(&result_stats),
            delay: config.publisher_delay,
        }),
    );

    let dispatcher = runtime.dispatcher();

    let started = Instant::now();
    let mut stats = DispatchStats::default();
    let mut attempted_commands = 0;
    let mut next_order_no = 0;

    for i in 0..config.orders {
        let symbol = &symbols[i % symbols.len()];
        let commands = make_workload(
            config.scenario,
            config.sweep_depth,
            symbol,
            &mut next_order_no,
        );
        attempted_commands += commands.len();

        for command in commands {
            match dispatcher.dispatch(symbol, command) {
                Ok(()) => stats.accepted += 1,
                Err(DispatchError::ChannelFull { .. }) => stats.channel_full += 1,
                Err(DispatchError::UnknownSymbol { .. }) => stats.unknown_symbol += 1,
                Err(DispatchError::EngineStopped { .. }) => stats.engine_stopped += 1,
            }
        }
    }

    let dispatch_elapsed = started.elapsed();
    let completed = wait_until_published(&result_stats.published, stats.accepted, config.timeout);
    let total_elapsed = started.elapsed();
    let result_stats = result_stats.snapshot();

    print_summary(
        &config,
        &stats,
        attempted_commands,
        completed,
        dispatch_elapsed,
        total_elapsed,
        &result_stats,
    );

    drop(dispatcher);

    if completed {
        runtime.shutdown();
    } else {
        eprintln!("제한 시간 안에 접수된 모든 명령의 결과가 발행되지 않아 종료 대기를 생략합니다");
    }
}

fn parse_config(args: impl IntoIterator<Item = String>) -> Result<Config, String> {
    let mut config = Config::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scenario" => {
                let value = next_arg(&mut args, "--scenario")?;
                config.scenario = Scenario::parse(&value)?;
            }
            "--orders" => {
                config.orders = parse_nonzero_usize(&next_arg(&mut args, "--orders")?, "--orders")?;
            }
            "--symbols" => {
                config.symbols =
                    parse_nonzero_usize(&next_arg(&mut args, "--symbols")?, "--symbols")?;
            }
            "--sweep-depth" => {
                config.sweep_depth =
                    parse_nonzero_usize(&next_arg(&mut args, "--sweep-depth")?, "--sweep-depth")?;
            }
            "--publisher-delay-ms" => {
                let millis = parse_u64(
                    &next_arg(&mut args, "--publisher-delay-ms")?,
                    "--publisher-delay-ms",
                )?;
                config.publisher_delay = Duration::from_millis(millis);
            }
            "--timeout-sec" => {
                let secs = parse_u64(&next_arg(&mut args, "--timeout-sec")?, "--timeout-sec")?;
                if secs == 0 {
                    return Err("--timeout-sec 값은 0보다 커야 합니다".to_string());
                }
                config.timeout = Duration::from_secs(secs);
            }
            _ => return Err(format!("알 수 없는 옵션: {arg}")),
        }
    }

    Ok(config)
}

fn next_arg(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} 값이 누락되었습니다"))
}

fn parse_nonzero_usize(value: &str, option: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{option} 값은 양의 정수여야 합니다"))?;

    if parsed == 0 {
        return Err(format!("{option} 값은 0보다 커야 합니다"));
    }

    Ok(parsed)
}

fn parse_u64(value: &str, option: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{option} 값은 0 이상의 정수여야 합니다"))
}

fn make_symbols(symbol_count: usize) -> Vec<String> {
    if symbol_count == 1 {
        return vec![BASE_SYMBOL.to_string()];
    }

    (0..symbol_count)
        .map(|i| format!("{BASE_SYMBOL}-{i}"))
        .collect()
}

fn make_workload(
    scenario: Scenario,
    sweep_depth: usize,
    symbol: &str,
    next_order_no: &mut usize,
) -> Vec<EngineCommand> {
    match scenario {
        Scenario::CancelMissing => vec![EngineCommand::Cancel(next_order_id(next_order_no))],
        Scenario::PlaceRestingLimit => vec![EngineCommand::Place(make_limit_order(
            next_order_id(next_order_no),
            symbol,
            Side::Buy,
            TimeInForce::GTC,
            10_000 + (*next_order_no % 50) as i64,
            1,
        ))],
        Scenario::FullFillSameLevel => {
            let mut commands = Vec::with_capacity(sweep_depth + 1);

            for _ in 0..sweep_depth {
                commands.push(EngineCommand::Place(make_limit_order(
                    next_order_id(next_order_no),
                    symbol,
                    Side::Sell,
                    TimeInForce::GTC,
                    100,
                    1,
                )));
            }

            commands.push(EngineCommand::Place(make_limit_order(
                next_order_id(next_order_no),
                symbol,
                Side::Buy,
                TimeInForce::IOC,
                100,
                sweep_depth as i64,
            )));
            commands
        }
        Scenario::MarketQuoteSweep => {
            let mut commands = Vec::with_capacity(sweep_depth + 1);
            let mut quote = 0;

            for index in 0..sweep_depth {
                let price = 100 + (index % 50) as i64;
                quote += price;
                commands.push(EngineCommand::Place(make_limit_order(
                    next_order_id(next_order_no),
                    symbol,
                    Side::Sell,
                    TimeInForce::GTC,
                    price,
                    1,
                )));
            }

            commands.push(EngineCommand::Place(make_market_quote_buy_order(
                next_order_id(next_order_no),
                symbol,
                quote,
            )));
            commands
        }
        Scenario::PartialFillRest => {
            let mut commands = Vec::with_capacity(sweep_depth + 1);

            for _ in 0..sweep_depth {
                commands.push(EngineCommand::Place(make_limit_order(
                    next_order_id(next_order_no),
                    symbol,
                    Side::Sell,
                    TimeInForce::GTC,
                    100,
                    1,
                )));
            }

            commands.push(EngineCommand::Place(make_limit_order(
                next_order_id(next_order_no),
                symbol,
                Side::Buy,
                TimeInForce::GTC,
                100,
                sweep_depth as i64 + 1,
            )));
            commands
        }
    }
}

fn next_order_id(next_order_no: &mut usize) -> Uuid {
    *next_order_no += 1;
    Uuid::from_u128(*next_order_no as u128)
}

fn make_limit_order(
    order_id: Uuid,
    symbol: &str,
    side: Side,
    tif: TimeInForce,
    price: i64,
    qty: i64,
) -> Order {
    Order {
        order_id,
        symbol: symbol.to_string(),
        side,
        order_type: OrderType::Limit,
        tif,
        price: Some(Decimal::new(price, 0)),
        size: OrderSize::Base(Decimal::new(qty, 0)),
        executed_base_qty: Decimal::ZERO,
        executed_quote_qty: Decimal::ZERO,
        status: OrderStatus::New,
        created_at: fixed_time(),
        updated_at: fixed_time(),
    }
}

fn make_market_quote_buy_order(order_id: Uuid, symbol: &str, quote: i64) -> Order {
    Order {
        order_id,
        symbol: symbol.to_string(),
        side: Side::Buy,
        order_type: OrderType::Market,
        tif: TimeInForce::IOC,
        price: None,
        size: OrderSize::Quote(Decimal::new(quote, 0)),
        executed_base_qty: Decimal::ZERO,
        executed_quote_qty: Decimal::ZERO,
        status: OrderStatus::New,
        created_at: fixed_time(),
        updated_at: fixed_time(),
    }
}

fn fixed_time() -> DateTime<Utc> {
    SystemTime::UNIX_EPOCH.into()
}

fn wait_until_published(published: &AtomicUsize, expected: usize, timeout: Duration) -> bool {
    let started = Instant::now();

    while published.load(Ordering::Relaxed) < expected {
        if started.elapsed() >= timeout {
            return false;
        }

        thread::sleep(Duration::from_millis(1));
    }

    true
}

fn print_summary(
    config: &Config,
    stats: &DispatchStats,
    attempted_commands: usize,
    completed: bool,
    dispatch_elapsed: Duration,
    total_elapsed: Duration,
    result_stats: &ResultStatsSnapshot,
) {
    print!(
        "{}",
        render_summary(
            config,
            stats,
            attempted_commands,
            completed,
            dispatch_elapsed,
            total_elapsed,
            result_stats,
        )
    );
}

fn render_summary(
    config: &Config,
    stats: &DispatchStats,
    attempted_commands: usize,
    completed: bool,
    dispatch_elapsed: Duration,
    total_elapsed: Duration,
    result_stats: &ResultStatsSnapshot,
) -> String {
    let mut output = String::new();

    writeln!(output, "================================").unwrap();
    writeln!(output, "런타임 스트레스 테스트 결과").unwrap();
    writeln!(output, "================================").unwrap();
    writeln!(output).unwrap();

    writeln!(output, "[실행 조건]").unwrap();
    push_row(
        &mut output,
        "시나리오",
        &format!(
            "{} ({})",
            config.scenario.display_name(),
            config.scenario.token()
        ),
    );
    push_row(
        &mut output,
        "워크로드 반복 수",
        &format_count(config.orders),
    );
    push_row(
        &mut output,
        "시도한 명령 수",
        &format_count(attempted_commands),
    );
    push_row(&mut output, "심볼 수", &format_count(config.symbols));
    push_row(&mut output, "스윕 깊이", &format_count(config.sweep_depth));
    push_row(
        &mut output,
        "발행 지연",
        &format!("{}ms", config.publisher_delay.as_millis()),
    );
    push_row(
        &mut output,
        "제한 시간",
        &format!("{}s", config.timeout.as_secs()),
    );
    writeln!(output).unwrap();

    writeln!(output, "[접수 결과]").unwrap();
    push_row(&mut output, "접수 성공", &format_count(stats.accepted));
    push_row(&mut output, "채널 포화", &format_count(stats.channel_full));
    push_row(
        &mut output,
        "미등록 심볼",
        &format_count(stats.unknown_symbol),
    );
    push_row(
        &mut output,
        "엔진 중지",
        &format_count(stats.engine_stopped),
    );
    push_row(&mut output, "거부 합계", &format_count(stats.rejected()));
    push_row(
        &mut output,
        "접수 성공률",
        &format!("{:.2}%", percentage(stats.accepted, attempted_commands)),
    );
    writeln!(output).unwrap();

    writeln!(output, "[발행 결과]").unwrap();
    push_row(
        &mut output,
        "발행 결과",
        &format_count(result_stats.published),
    );
    push_row(
        &mut output,
        "주문 접수 결과",
        &format_count(result_stats.place_results),
    );
    push_row(
        &mut output,
        "주문 취소 결과",
        &format_count(result_stats.cancel_results),
    );
    push_row(
        &mut output,
        "주문 정정 결과",
        &format_count(result_stats.amend_results),
    );
    push_row(&mut output, "체결 수", &format_count(result_stats.trades));
    push_row(
        &mut output,
        "메이커 갱신 수",
        &format_count(result_stats.maker_updates),
    );
    push_row(&mut output, "완료 여부", completion_label(completed));
    writeln!(output).unwrap();

    writeln!(output, "[처리량]").unwrap();
    push_row(
        &mut output,
        "접수 소요 시간",
        &format_duration_display(dispatch_elapsed),
    );
    push_row(
        &mut output,
        "전체 소요 시간",
        &format_duration_display(total_elapsed),
    );
    push_row(
        &mut output,
        "초당 접수 성공 수",
        &format_rate(per_sec(stats.accepted, dispatch_elapsed)),
    );
    push_row(
        &mut output,
        "초당 발행 결과 수",
        &format_rate(per_sec(result_stats.published, total_elapsed)),
    );

    output
}

fn percentage(count: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }

    count as f64 * 100.0 / total as f64
}

fn completion_label(completed: bool) -> &'static str {
    if completed { "예" } else { "아니오" }
}

fn per_sec(count: usize, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs == 0.0 {
        return 0.0;
    }

    count as f64 / secs
}

fn push_row(output: &mut String, label: &str, value: &str) {
    const LABEL_WIDTH: usize = 19;

    let padding = LABEL_WIDTH.saturating_sub(display_width(label)).max(1);
    writeln!(output, "  {label}{}{value}", " ".repeat(padding)).unwrap();
}

fn display_width(value: &str) -> usize {
    value
        .chars()
        .map(|ch| if ch.is_ascii() { 1 } else { 2 })
        .sum()
}

fn format_count(value: usize) -> String {
    let raw = value.to_string();
    let mut formatted = String::with_capacity(raw.len() + raw.len() / 3);

    for (index, ch) in raw.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(ch);
    }

    formatted.chars().rev().collect()
}

fn format_duration_display(duration: Duration) -> String {
    let secs = duration.as_secs_f64();

    if secs >= 1.0 {
        format!("{secs:.3}s")
    } else {
        format!("{:.3}ms", secs * 1_000.0)
    }
}

fn format_rate(value: f64) -> String {
    let raw = format!("{value:.2}");
    let Some((whole, fraction)) = raw.split_once('.') else {
        return format_count(raw.parse().unwrap_or_default());
    };

    format!(
        "{}.{}",
        format_count(whole.parse().unwrap_or_default()),
        fraction
    )
}

fn print_usage() {
    eprintln!(
        "사용법: cargo run --release --bin runtime_stress -- \
         [--scenario cancel-missing|place-resting-limit|full-fill-same-level|market-quote-sweep|partial-fill-rest] \
         [--orders N] \
         [--symbols N] \
         [--sweep-depth N] \
         [--publisher-delay-ms N] \
         [--timeout-sec N]\n\
         기본 시나리오: full-fill-same-level\n\
         보조 진단 시나리오: cancel-missing"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Config, String> {
        parse_config(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn parse_config_uses_defaults() {
        assert_eq!(
            parse(&[]).unwrap(),
            Config {
                scenario: Scenario::FullFillSameLevel,
                ..Config::default()
            }
        );
    }

    #[test]
    fn parse_config_accepts_supported_options() {
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
                orders: 42,
                symbols: 4,
                sweep_depth: 7,
                publisher_delay: Duration::from_millis(3),
                timeout: Duration::from_secs(5),
            }
        );
    }

    #[test]
    fn parse_config_rejects_zero_symbols() {
        assert_eq!(
            parse(&["--symbols", "0"]).unwrap_err(),
            "--symbols 값은 0보다 커야 합니다"
        );
    }

    #[test]
    fn parse_config_rejects_zero_sweep_depth() {
        assert_eq!(
            parse(&["--sweep-depth", "0"]).unwrap_err(),
            "--sweep-depth 값은 0보다 커야 합니다"
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
    }

    #[test]
    fn scenario_parse_rejects_unknown_scenario_in_korean() {
        assert_eq!(
            Scenario::parse("invalid").unwrap_err(),
            "지원하지 않는 시나리오: invalid"
        );
    }

    #[test]
    fn completion_label_returns_korean_label() {
        assert_eq!(completion_label(true), "예");
        assert_eq!(completion_label(false), "아니오");
    }

    #[test]
    fn scenario_token_returns_cli_value() {
        assert_eq!(Scenario::FullFillSameLevel.token(), "full-fill-same-level");
        assert_eq!(Scenario::CancelMissing.token(), "cancel-missing");
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
    fn render_summary_groups_results_into_readable_sections() {
        let config = Config {
            scenario: Scenario::FullFillSameLevel,
            orders: 100_000,
            symbols: 1,
            sweep_depth: 10,
            publisher_delay: Duration::ZERO,
            timeout: Duration::from_secs(30),
        };
        let stats = DispatchStats {
            accepted: 1_100_000,
            channel_full: 0,
            unknown_symbol: 0,
            engine_stopped: 0,
        };
        let result_stats = ResultStatsSnapshot {
            published: 1_100_000,
            place_results: 1_100_000,
            cancel_results: 0,
            amend_results: 0,
            trades: 1_000_000,
            maker_updates: 1_000_000,
        };

        let output = render_summary(
            &config,
            &stats,
            1_100_000,
            true,
            Duration::from_micros(123_456),
            Duration::from_micros(456_789),
            &result_stats,
        );

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
    fn parse_config_rejects_missing_value_in_korean() {
        assert_eq!(
            parse(&["--orders"]).unwrap_err(),
            "--orders 값이 누락되었습니다"
        );
    }

    #[test]
    fn make_symbols_uses_plain_base_symbol_for_single_symbol() {
        assert_eq!(make_symbols(1), vec!["BTCUSDT"]);
    }

    #[test]
    fn make_symbols_suffixes_multiple_symbols() {
        assert_eq!(make_symbols(3), vec!["BTCUSDT-0", "BTCUSDT-1", "BTCUSDT-2"]);
    }

    #[test]
    fn make_workload_full_fill_same_level_creates_makers_and_taker() {
        let mut next_order_no = 0;
        let commands = make_workload(
            Scenario::FullFillSameLevel,
            3,
            BASE_SYMBOL,
            &mut next_order_no,
        );

        assert_eq!(commands.len(), 4);
        assert_eq!(next_order_no, 4);
        assert!(commands[..3].iter().all(|cmd| matches!(
            cmd,
            EngineCommand::Place(Order {
                side: Side::Sell,
                order_type: OrderType::Limit,
                tif: TimeInForce::GTC,
                price: Some(_),
                ..
            })
        )));
        assert!(matches!(
            commands.last().unwrap(),
            EngineCommand::Place(Order {
                side: Side::Buy,
                order_type: OrderType::Limit,
                tif: TimeInForce::IOC,
                size: OrderSize::Base(qty),
                ..
            }) if *qty == Decimal::new(3, 0)
        ));
    }

    #[test]
    fn make_workload_market_quote_sweep_creates_quote_market_taker() {
        let mut next_order_no = 0;
        let commands = make_workload(
            Scenario::MarketQuoteSweep,
            3,
            BASE_SYMBOL,
            &mut next_order_no,
        );

        assert_eq!(commands.len(), 4);
        assert!(matches!(
            commands.last().unwrap(),
            EngineCommand::Place(Order {
                side: Side::Buy,
                order_type: OrderType::Market,
                tif: TimeInForce::IOC,
                price: None,
                size: OrderSize::Quote(quote),
                ..
            }) if *quote == Decimal::new(303, 0)
        ));
    }

    #[test]
    fn make_workload_partial_fill_rest_creates_gtc_taker_with_remaining_qty() {
        let mut next_order_no = 0;
        let commands = make_workload(
            Scenario::PartialFillRest,
            3,
            BASE_SYMBOL,
            &mut next_order_no,
        );

        assert_eq!(commands.len(), 4);
        assert!(matches!(
            commands.last().unwrap(),
            EngineCommand::Place(Order {
                side: Side::Buy,
                order_type: OrderType::Limit,
                tif: TimeInForce::GTC,
                size: OrderSize::Base(qty),
                ..
            }) if *qty == Decimal::new(4, 0)
        ));
    }
}

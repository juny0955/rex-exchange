use std::time::Duration;

const DEFAULT_ORDERS: usize = 100_000;
const DEFAULT_SCENARIO: Scenario = Scenario::FullFillSameLevel;
const DEFAULT_SYMBOLS: usize = 1;
const DEFAULT_SWEEP_DEPTH: usize = 10;
const DEFAULT_PUBLISHER_DELAY_MS: u64 = 0;
const DEFAULT_TIMEOUT_SEC: u64 = 30;
const MAX_DURATION_SEC: u64 = 86_400;
const MAX_ORDERS: usize = 10_000_000;
const MAX_PUBLISHER_DELAY_MS: u64 = 60_000;
const MAX_SWEEP_DEPTH: usize = 10_000;
const MAX_SYMBOLS: usize = 1_024;
const MAX_TARGET_COMMANDS_PER_SEC: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    CancelMissing,
    PlaceRestingLimit,
    FullFillSameLevel,
    MarketQuoteSweep,
    PartialFillRest,
}

impl Scenario {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "cancel-missing" => Ok(Self::CancelMissing),
            "place-resting-limit" => Ok(Self::PlaceRestingLimit),
            "full-fill-same-level" => Ok(Self::FullFillSameLevel),
            "market-quote-sweep" => Ok(Self::MarketQuoteSweep),
            "partial-fill-rest" => Ok(Self::PartialFillRest),
            _ => Err(format!("지원하지 않는 시나리오: {value}")),
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::CancelMissing => "기본 경로 확인(미존재 취소)",
            Self::PlaceRestingLimit => "미체결 지정가 잔존",
            Self::FullFillSameLevel => "동일 호가 전량 체결",
            Self::MarketQuoteSweep => "시장가 금액 스윕",
            Self::PartialFillRest => "부분 체결 후 잔존",
        }
    }

    pub fn token(self) -> &'static str {
        match self {
            Self::CancelMissing => "cancel-missing",
            Self::PlaceRestingLimit => "place-resting-limit",
            Self::FullFillSameLevel => "full-fill-same-level",
            Self::MarketQuoteSweep => "market-quote-sweep",
            Self::PartialFillRest => "partial-fill-rest",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunMode {
    Burst {
        orders: usize,
    },
    Paced {
        warmup: Duration,
        duration: Duration,
        target_commands_per_sec: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub scenario: Scenario,
    pub mode: RunMode,
    pub symbols: usize,
    pub sweep_depth: usize,
    pub publisher_delay: Duration,
    pub timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scenario: DEFAULT_SCENARIO,
            mode: RunMode::Burst {
                orders: DEFAULT_ORDERS,
            },
            symbols: DEFAULT_SYMBOLS,
            sweep_depth: DEFAULT_SWEEP_DEPTH,
            publisher_delay: Duration::from_millis(DEFAULT_PUBLISHER_DELAY_MS),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SEC),
        }
    }
}

pub fn parse_config(args: impl IntoIterator<Item = String>) -> Result<Config, String> {
    let mut scenario = DEFAULT_SCENARIO;
    let mut orders = None;
    let mut symbols = DEFAULT_SYMBOLS;
    let mut sweep_depth = DEFAULT_SWEEP_DEPTH;
    let mut publisher_delay = Duration::from_millis(DEFAULT_PUBLISHER_DELAY_MS);
    let mut timeout = Duration::from_secs(DEFAULT_TIMEOUT_SEC);
    let mut duration = None;
    let mut warmup = Duration::ZERO;
    let mut target_commands_per_sec = None;

    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scenario" => scenario = Scenario::parse(&next_arg(&mut args, "--scenario")?)?,
            "--orders" => {
                orders = Some(parse_nonzero_usize(
                    &next_arg(&mut args, "--orders")?,
                    "--orders",
                    MAX_ORDERS,
                )?);
            }
            "--symbols" => {
                symbols = parse_nonzero_usize(
                    &next_arg(&mut args, "--symbols")?,
                    "--symbols",
                    MAX_SYMBOLS,
                )?;
            }
            "--sweep-depth" => {
                sweep_depth = parse_nonzero_usize(
                    &next_arg(&mut args, "--sweep-depth")?,
                    "--sweep-depth",
                    MAX_SWEEP_DEPTH,
                )?;
            }
            "--publisher-delay-ms" => {
                let millis = parse_bounded_u64(
                    &next_arg(&mut args, "--publisher-delay-ms")?,
                    "--publisher-delay-ms",
                    MAX_PUBLISHER_DELAY_MS,
                )?;
                publisher_delay = Duration::from_millis(millis);
            }
            "--timeout-sec" => {
                timeout = Duration::from_secs(parse_nonzero_u64(
                    &next_arg(&mut args, "--timeout-sec")?,
                    "--timeout-sec",
                    MAX_DURATION_SEC,
                )?);
            }
            "--duration-sec" => {
                duration = Some(Duration::from_secs(parse_nonzero_u64(
                    &next_arg(&mut args, "--duration-sec")?,
                    "--duration-sec",
                    MAX_DURATION_SEC,
                )?));
            }
            "--warmup-sec" => {
                let secs = parse_bounded_u64(
                    &next_arg(&mut args, "--warmup-sec")?,
                    "--warmup-sec",
                    MAX_DURATION_SEC,
                )?;
                warmup = Duration::from_secs(secs);
            }
            "--target-commands-per-sec" => {
                let value = parse_nonzero_u64(
                    &next_arg(&mut args, "--target-commands-per-sec")?,
                    "--target-commands-per-sec",
                    MAX_TARGET_COMMANDS_PER_SEC,
                )?;
                target_commands_per_sec = Some(value);
            }
            _ => return Err(format!("알 수 없는 옵션: {arg}")),
        }
    }

    let mode = match duration {
        Some(duration) => {
            if orders.is_some() {
                return Err("--orders와 --duration-sec는 함께 사용할 수 없습니다".to_string());
            }

            let Some(target_commands_per_sec) = target_commands_per_sec else {
                return Err(
                    "--duration-sec를 사용할 때는 --target-commands-per-sec가 필요합니다"
                        .to_string(),
                );
            };

            RunMode::Paced {
                warmup,
                duration,
                target_commands_per_sec,
            }
        }
        None => RunMode::Burst {
            orders: parse_burst_orders(orders, warmup, target_commands_per_sec)?,
        },
    };

    Ok(Config {
        scenario,
        mode,
        symbols,
        sweep_depth,
        publisher_delay,
        timeout,
    })
}

fn parse_burst_orders(
    orders: Option<usize>,
    warmup: Duration,
    target_commands_per_sec: Option<u64>,
) -> Result<usize, String> {
    if target_commands_per_sec.is_some() {
        return Err(
            "--target-commands-per-sec는 --duration-sec와 함께 사용해야 합니다".to_string(),
        );
    }

    if !warmup.is_zero() {
        return Err("--warmup-sec는 --duration-sec와 함께 사용해야 합니다".to_string());
    }

    Ok(orders.unwrap_or(DEFAULT_ORDERS))
}

fn next_arg(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} 값이 누락되었습니다"))
}

fn parse_nonzero_usize(value: &str, option: &str, max: usize) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{option} 값은 양의 정수여야 합니다"))?;

    if parsed == 0 {
        return Err(format!("{option} 값은 0보다 커야 합니다"));
    }
    if parsed > max {
        return Err(format!("{option} 값은 {max} 이하여야 합니다"));
    }

    Ok(parsed)
}

fn parse_nonzero_u64(value: &str, option: &str, max: u64) -> Result<u64, String> {
    let parsed = parse_bounded_u64(value, option, max)?;
    if parsed == 0 {
        return Err(format!("{option} 값은 0보다 커야 합니다"));
    }

    Ok(parsed)
}

fn parse_bounded_u64(value: &str, option: &str, max: u64) -> Result<u64, String> {
    let parsed = parse_u64(value, option)?;
    if parsed > max {
        return Err(format!("{option} 값은 {max} 이하여야 합니다"));
    }

    Ok(parsed)
}

fn parse_u64(value: &str, option: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{option} 값은 0 이상의 정수여야 합니다"))
}

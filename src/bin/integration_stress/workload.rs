//! 워크로드 생성기. runtime_stress의 동일 로직을 통합 부하 바이너리에서 재사용하기 위해 복제.
//! 생성 산출물은 `EngineCommand`이며, gRPC 요청 변환은 `client` 모듈이 담당한다.

use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use matching_engine::{
    domain::order::{Order, OrderSize, OrderStatus, OrderType, Side, TimeInForce},
    engine::command::{AmendOrderCommand, EngineCommand},
};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::integration_stress::config::Scenario;

pub const BASE_SYMBOL: &str = "BTCUSDT";

#[derive(Default)]
pub struct WorkloadGenerator {
    next_order_no: usize,
}

impl WorkloadGenerator {
    pub fn make_workload(
        &mut self,
        scenario: Scenario,
        sweep_depth: usize,
        symbol: &str,
    ) -> Vec<EngineCommand> {
        make_workload(scenario, sweep_depth, symbol, &mut self.next_order_no)
    }
}

pub fn make_symbols(symbol_count: usize) -> Vec<String> {
    if symbol_count == 1 {
        return vec![BASE_SYMBOL.to_string()];
    }

    (0..symbol_count)
        .map(|i| format!("{BASE_SYMBOL}-{i}"))
        .collect()
}

pub fn command_interval(target_commands_per_sec: u64) -> Duration {
    Duration::from_secs_f64(1.0 / target_commands_per_sec as f64)
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
        Scenario::CancelRestingOrder => {
            let mut commands = Vec::with_capacity(sweep_depth * 2);
            let mut order_ids = Vec::with_capacity(sweep_depth);

            for _ in 0..sweep_depth {
                let order_id = next_order_id(next_order_no);
                order_ids.push(order_id);
                commands.push(EngineCommand::Place(make_limit_order(
                    order_id,
                    symbol,
                    Side::Buy,
                    TimeInForce::GTC,
                    10_000,
                    1,
                )));
            }

            commands.extend(order_ids.into_iter().map(EngineCommand::Cancel));
            commands
        }
        Scenario::AmendDecreaseQty => {
            let order_id = next_order_id(next_order_no);
            vec![
                EngineCommand::Place(make_limit_order(
                    order_id,
                    symbol,
                    Side::Buy,
                    TimeInForce::GTC,
                    10_000,
                    2,
                )),
                EngineCommand::Amend(make_amend_order_command(order_id, 10_000, 1)),
                EngineCommand::Cancel(order_id),
            ]
        }
        Scenario::AmendPriceChange => {
            let order_id = next_order_id(next_order_no);
            vec![
                EngineCommand::Place(make_limit_order(
                    order_id,
                    symbol,
                    Side::Buy,
                    TimeInForce::GTC,
                    10_000,
                    1,
                )),
                EngineCommand::Amend(make_amend_order_command(order_id, 10_001, 1)),
                EngineCommand::Cancel(order_id),
            ]
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

fn make_amend_order_command(order_id: Uuid, price: i64, qty: i64) -> AmendOrderCommand {
    AmendOrderCommand {
        order_id,
        price: Some(Decimal::new(price, 0)),
        base_qty: Some(Decimal::new(qty, 0)),
    }
}

fn fixed_time() -> DateTime<Utc> {
    SystemTime::UNIX_EPOCH.into()
}

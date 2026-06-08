use std::time::SystemTime;

use chrono::{DateTime, Utc};
use matching_engine::{
    domain::order::{Order, OrderSize, OrderStatus, OrderType, Side, TimeInForce},
    engine::command::EngineCommand,
};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::runtime_stress::config::Scenario;

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

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut generator = WorkloadGenerator::default();
        let commands = generator.make_workload(Scenario::FullFillSameLevel, 3, BASE_SYMBOL);

        assert_eq!(commands.len(), 4);
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
            commands.last(),
            Some(EngineCommand::Place(Order {
                side: Side::Buy,
                order_type: OrderType::Limit,
                tif: TimeInForce::IOC,
                size: OrderSize::Base(qty),
                ..
            })) if *qty == Decimal::new(3, 0)
        ));
    }

    #[test]
    fn make_workload_market_quote_sweep_creates_quote_market_taker() {
        let mut generator = WorkloadGenerator::default();
        let commands = generator.make_workload(Scenario::MarketQuoteSweep, 3, BASE_SYMBOL);

        assert_eq!(commands.len(), 4);
        assert!(matches!(
            commands.last(),
            Some(EngineCommand::Place(Order {
                side: Side::Buy,
                order_type: OrderType::Market,
                tif: TimeInForce::IOC,
                price: None,
                size: OrderSize::Quote(quote),
                ..
            })) if *quote == Decimal::new(303, 0)
        ));
    }

    #[test]
    fn make_workload_partial_fill_rest_creates_gtc_taker_with_remaining_qty() {
        let mut generator = WorkloadGenerator::default();
        let commands = generator.make_workload(Scenario::PartialFillRest, 3, BASE_SYMBOL);

        assert_eq!(commands.len(), 4);
        assert!(matches!(
            commands.last(),
            Some(EngineCommand::Place(Order {
                side: Side::Buy,
                order_type: OrderType::Limit,
                tif: TimeInForce::GTC,
                size: OrderSize::Base(qty),
                ..
            })) if *qty == Decimal::new(4, 0)
        ));
    }
}

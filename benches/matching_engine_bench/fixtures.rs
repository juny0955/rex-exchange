use std::time::SystemTime;

use chrono::{DateTime, Utc};
use matching_engine::{
    domain::order::{Order, OrderSize, OrderStatus, OrderType, Side, TimeInForce},
    engine::matching_engine::MatchingEngine,
};
use rust_decimal::Decimal;
use uuid::Uuid;

pub(super) const INPUT_SIZES: [usize; 3] = [10, 100, 1_000];
pub(super) const TAKER_ORDER_ID_BASE: u128 = 9_000_000;

const SYMBOL: &str = "BTCUSDT";
const PRICE_LEVELS: usize = 50;
const ASK_ORDER_ID_BASE: u128 = 1_000_000;
const SAME_ASK_ORDER_ID_BASE: u128 = 2_000_000;
const SAME_BID_ORDER_ID_BASE: u128 = 3_000_000;

fn fixed_time() -> DateTime<Utc> {
    SystemTime::UNIX_EPOCH.into()
}

pub(super) fn decimal(value: i64) -> Decimal {
    Decimal::new(value, 0)
}

pub(super) fn make_engine() -> MatchingEngine {
    let (_, engine_rx) = crossbeam::channel::unbounded();
    let (result_tx, _) = crossbeam::channel::unbounded();

    MatchingEngine::new(SYMBOL.to_string(), engine_rx, result_tx)
}

pub(super) fn limit_order(
    side: Side,
    order_id: u128,
    tif: TimeInForce,
    price: i64,
    qty: i64,
) -> Order {
    Order {
        order_id: Uuid::from_u128(order_id),
        symbol: SYMBOL.to_string(),
        side,
        order_type: OrderType::Limit,
        tif,
        price: Some(decimal(price)),
        size: OrderSize::Base(decimal(qty)),
        executed_base_qty: Decimal::ZERO,
        executed_quote_qty: Decimal::ZERO,
        status: OrderStatus::New,
        created_at: fixed_time(),
        updated_at: fixed_time(),
    }
}

pub(super) fn market_quote_buy(order_id: u128, quote: i64) -> Order {
    Order {
        order_id: Uuid::from_u128(order_id),
        symbol: SYMBOL.to_string(),
        side: Side::Buy,
        order_type: OrderType::Market,
        tif: TimeInForce::IOC,
        price: None,
        size: OrderSize::Quote(decimal(quote)),
        executed_base_qty: Decimal::ZERO,
        executed_quote_qty: Decimal::ZERO,
        status: OrderStatus::New,
        created_at: fixed_time(),
        updated_at: fixed_time(),
    }
}

fn ask_price(index: usize) -> i64 {
    100 + (index % PRICE_LEVELS) as i64
}

pub(super) fn seed_asks(engine: &mut MatchingEngine, count: usize) {
    for index in 0..count {
        engine.bench_seed_order(limit_order(
            Side::Sell,
            ASK_ORDER_ID_BASE + index as u128,
            TimeInForce::GTC,
            ask_price(index),
            1,
        ));
    }
}

pub(super) fn seed_same_price_asks(engine: &mut MatchingEngine, count: usize) {
    for index in 0..count {
        engine.bench_seed_order(limit_order(
            Side::Sell,
            SAME_ASK_ORDER_ID_BASE + index as u128,
            TimeInForce::GTC,
            100,
            1,
        ));
    }
}

pub(super) fn seed_same_price_bids(engine: &mut MatchingEngine, count: usize, qty: i64) {
    for index in 0..count {
        engine.bench_seed_order(limit_order(
            Side::Buy,
            SAME_BID_ORDER_ID_BASE + index as u128,
            TimeInForce::GTC,
            100,
            qty,
        ));
    }
}

pub(super) fn middle_bid_order_id(count: usize) -> Uuid {
    Uuid::from_u128(SAME_BID_ORDER_ID_BASE + (count / 2) as u128)
}

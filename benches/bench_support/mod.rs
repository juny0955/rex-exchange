use std::time::SystemTime;

use chrono::{DateTime, Utc};
use matching_engine::domain::order::{Order, OrderSize, OrderStatus, OrderType, Side, TimeInForce};
use rust_decimal::Decimal;
use uuid::Uuid;

pub const SYMBOL: &str = "BTCUSDT";
pub const PRICE_LEVELS: usize = 50;

pub fn fixed_time() -> DateTime<Utc> {
    SystemTime::UNIX_EPOCH.into()
}

pub fn decimal(value: i64) -> Decimal {
    Decimal::new(value, 0)
}

pub fn price_for(index: usize, base_price: i64) -> i64 {
    base_price + (index % PRICE_LEVELS) as i64
}

pub fn limit_order(side: Side, order_id: u128, tif: TimeInForce, price: i64, qty: i64) -> Order {
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

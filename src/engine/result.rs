use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::order::OrderStatus;

#[derive(Debug, Clone)]
pub enum EngineResult {
    Place(PlaceOrderResult),
}

#[derive(Debug, Clone)]
pub struct PlaceOrderResult {
    pub symbol: String,
    pub taker_order_id: Uuid,
    pub outcome: PlaceOrderOutcome,
    pub trades: Vec<TradeResult>,
    pub updated_makers: Vec<OrderSnapshot>,
}

#[derive(Debug, Clone)]
pub enum PlaceOrderOutcome {
    Accepted,
    Rested,
    Filled,
    PartiallyFilledAndRested,
    PartiallyFilledAndCancelled(CancelledReason),
    Cancelled(CancelledReason),
    Rejected(RejectedReason),
}

#[derive(Debug, Clone)]
pub struct TradeResult {
    pub trade_id: Uuid,
    pub taker_order_id: Uuid,
    pub maker_order_id: Uuid,
    pub price: Decimal,
    pub base_qty: Decimal,
    pub quote_qty: Decimal,
}

#[derive(Debug, Clone)]
pub struct OrderSnapshot {
    pub order_id: Uuid,
    pub executed_base_qty: Decimal,
    pub executed_quote_qty: Decimal,
    pub remaining_base_qty: Option<Decimal>,
    pub remaining_quote_qty: Option<Decimal>,
    pub status: OrderStatus,
}

#[derive(Debug, Clone)]
pub enum RejectedReason {
    InvalidOrder(String),
}

#[derive(Debug, Clone)]
pub enum CancelledReason {
    FokCannotFullyFill,
    IocRemainingCancelled,
}

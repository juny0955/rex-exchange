use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::order::{Order, OrderStatus};

#[derive(Debug, Clone)]
pub enum EngineResult {
    Place(PlaceOrderResult),
    Cancel(CancelOrderResult),
    Amend(AmendOrderResult),
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
pub struct CancelOrderResult {
    pub symbol: String,
    pub order_id: Uuid,
    pub outcome: CancelOrderOutcome,
}

#[derive(Debug, Clone)]
pub struct AmendOrderResult {
    pub symbol: String,
    pub order_id: Uuid,
    pub outcome: AmendOrderOutcome,
}

#[derive(Debug, Clone)]
pub enum PlaceOrderOutcome {
    Rested,
    Filled,
    PartiallyFilledAndRested,
    PartiallyFilledAndCancelled(CancelledReason),
    Cancelled(CancelledReason),
    Rejected(RejectedReason),
}

#[derive(Debug, Clone)]
pub enum CancelOrderOutcome {
    Cancelled(OrderSnapshot),
    Rejected(CancelRejectedReason),
}

#[derive(Debug, Clone)]
pub enum AmendOrderOutcome {
    Amended(OrderSnapshot),
    CancelReplaced {
        canceled: OrderSnapshot,
        placed: PlaceOrderResult,
    },
    Rejected(AmendRejectedReason),
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

impl From<&Order> for OrderSnapshot {
    fn from(o: &Order) -> Self {
        Self {
            order_id: o.order_id,
            executed_base_qty: o.executed_base_qty,
            executed_quote_qty: o.executed_quote_qty,
            remaining_base_qty: o.remaining_base_qty(),
            remaining_quote_qty: o.remaining_quote_qty(),
            status: o.status,
        }
    }
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

#[derive(Debug, Clone)]
pub enum CancelRejectedReason {
    OrderNotFound,
}

#[derive(Debug, Clone)]
pub enum AmendRejectedReason {
    OrderNotFound,
    AmendNotAllowed,
}

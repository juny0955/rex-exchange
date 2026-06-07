use serde::Serialize;

use crate::{
    domain::order::OrderStatus,
    engine::result::{OrderSnapshot, TradeResult},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TradeEvent {
    pub trade_id: String,
    pub taker_order_id: String,
    pub maker_order_id: String,
    pub price: String,
    pub base_qty: String,
    pub quote_qty: String,
}

impl From<&TradeResult> for TradeEvent {
    fn from(value: &TradeResult) -> Self {
        Self {
            trade_id: value.trade_id.to_string(),
            taker_order_id: value.taker_order_id.to_string(),
            maker_order_id: value.maker_order_id.to_string(),
            price: value.price.to_string(),
            base_qty: value.base_qty.to_string(),
            quote_qty: value.quote_qty.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrderSnapshotEvent {
    pub order_id: String,
    pub executed_base_qty: String,
    pub executed_quote_qty: String,
    pub remaining_base_qty: Option<String>,
    pub remaining_quote_qty: Option<String>,
    pub status: OrderStatusEvent,
}

impl From<&OrderSnapshot> for OrderSnapshotEvent {
    fn from(value: &OrderSnapshot) -> Self {
        Self {
            order_id: value.order_id.to_string(),
            executed_base_qty: value.executed_base_qty.to_string(),
            executed_quote_qty: value.executed_quote_qty.to_string(),
            remaining_base_qty: value
                .remaining_base_qty
                .map(|remaining| remaining.to_string()),
            remaining_quote_qty: value
                .remaining_quote_qty
                .map(|remaining| remaining.to_string()),
            status: OrderStatusEvent::from(&value.status),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatusEvent {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}

impl From<&OrderStatus> for OrderStatusEvent {
    fn from(value: &OrderStatus) -> Self {
        match value {
            OrderStatus::New => Self::New,
            OrderStatus::PartiallyFilled => Self::PartiallyFilled,
            OrderStatus::Filled => Self::Filled,
            OrderStatus::Cancelled => Self::Cancelled,
            OrderStatus::Rejected => Self::Rejected,
            OrderStatus::Expired => Self::Expired,
        }
    }
}

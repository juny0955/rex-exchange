use serde::Serialize;

use crate::{
    engine::result::{CancelledReason, PlaceOrderOutcome, PlaceOrderResult, RejectedReason},
    kafka::dto::common::{OrderSnapshotEvent, TradeEvent},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlaceOrderEvent {
    pub taker_order_id: String,
    pub outcome: PlaceOrderOutcomeEvent,
    pub trades: Vec<TradeEvent>,
    pub updated_makers: Vec<OrderSnapshotEvent>,
}

impl From<&PlaceOrderResult> for PlaceOrderEvent {
    fn from(value: &PlaceOrderResult) -> Self {
        Self {
            taker_order_id: value.taker_order_id.to_string(),
            outcome: PlaceOrderOutcomeEvent::from(&value.outcome),
            trades: value.trades.iter().map(TradeEvent::from).collect(),
            updated_makers: value
                .updated_makers
                .iter()
                .map(OrderSnapshotEvent::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlaceOrderOutcomeEvent {
    Rested,
    Filled,
    PartiallyFilledAndRested,
    PartiallyFilledAndCancelled { reason: CancelledReasonEvent },
    Cancelled { reason: CancelledReasonEvent },
    Rejected { reason: RejectedReasonEvent },
}

impl From<&PlaceOrderOutcome> for PlaceOrderOutcomeEvent {
    fn from(value: &PlaceOrderOutcome) -> Self {
        match value {
            PlaceOrderOutcome::Rested => Self::Rested,
            PlaceOrderOutcome::Filled => Self::Filled,
            PlaceOrderOutcome::PartiallyFilledAndRested => Self::PartiallyFilledAndRested,
            PlaceOrderOutcome::PartiallyFilledAndCancelled(reason) => {
                Self::PartiallyFilledAndCancelled {
                    reason: CancelledReasonEvent::from(reason),
                }
            }
            PlaceOrderOutcome::Cancelled(reason) => Self::Cancelled {
                reason: CancelledReasonEvent::from(reason),
            },
            PlaceOrderOutcome::Rejected(reason) => Self::Rejected {
                reason: RejectedReasonEvent::from(reason),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelledReasonEvent {
    FokCannotFullyFill,
    IocRemainingCancelled,
}

impl From<&CancelledReason> for CancelledReasonEvent {
    fn from(value: &CancelledReason) -> Self {
        match value {
            CancelledReason::FokCannotFullyFill => Self::FokCannotFullyFill,
            CancelledReason::IocRemainingCancelled => Self::IocRemainingCancelled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RejectedReasonEvent {
    InvalidOrder { message: String },
    DuplicateOrderId,
}

impl From<&RejectedReason> for RejectedReasonEvent {
    fn from(value: &RejectedReason) -> Self {
        match value {
            RejectedReason::InvalidOrder(msg) => Self::InvalidOrder {
                message: msg.clone(),
            },
            RejectedReason::DuplicateOrderId => Self::DuplicateOrderId,
        }
    }
}

use serde::Serialize;

use crate::{
    engine::result::{CancelOrderOutcome, CancelOrderResult, CancelRejectedReason},
    kafka::dto::common::OrderSnapshotEvent,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CancelOrderEvent {
    pub order_id: String,
    pub outcome: CancelOrderOutcomeEvent,
}

impl From<&CancelOrderResult> for CancelOrderEvent {
    fn from(value: &CancelOrderResult) -> Self {
        Self {
            order_id: value.order_id.to_string(),
            outcome: CancelOrderOutcomeEvent::from(&value.outcome),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CancelOrderOutcomeEvent {
    Cancelled { order: OrderSnapshotEvent },
    Rejected { reason: CancelRejectedReasonEvent },
}

impl From<&CancelOrderOutcome> for CancelOrderOutcomeEvent {
    fn from(value: &CancelOrderOutcome) -> Self {
        match value {
            CancelOrderOutcome::Cancelled(order_snapshot) => Self::Cancelled {
                order: OrderSnapshotEvent::from(order_snapshot),
            },
            CancelOrderOutcome::Rejected(reason) => Self::Rejected {
                reason: CancelRejectedReasonEvent::from(reason),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CancelRejectedReasonEvent {
    OrderNotFound,
}

impl From<&CancelRejectedReason> for CancelRejectedReasonEvent {
    fn from(value: &CancelRejectedReason) -> Self {
        match value {
            CancelRejectedReason::OrderNotFound => Self::OrderNotFound,
        }
    }
}

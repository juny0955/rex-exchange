use serde::Serialize;

use crate::{
    engine::result::{AmendOrderOutcome, AmendOrderResult, AmendRejectedReason},
    kafka::dto::{common::OrderSnapshotEvent, place_order::PlaceOrderEvent},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AmendOrderEvent {
    pub order_id: String,
    pub outcome: AmendOrderOutcomeEvent,
}

impl From<&AmendOrderResult> for AmendOrderEvent {
    fn from(value: &AmendOrderResult) -> Self {
        Self {
            order_id: value.order_id.to_string(),
            outcome: AmendOrderOutcomeEvent::from(&value.outcome),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AmendOrderOutcomeEvent {
    Amended {
        order: OrderSnapshotEvent,
    },
    CancelReplaced {
        cancelled: OrderSnapshotEvent,
        placed: PlaceOrderEvent,
    },
    Rejected {
        reason: AmendRejectedReasonEvent,
    },
}

impl From<&AmendOrderOutcome> for AmendOrderOutcomeEvent {
    fn from(value: &AmendOrderOutcome) -> Self {
        match value {
            AmendOrderOutcome::Amended(order_snapshot) => Self::Amended {
                order: OrderSnapshotEvent::from(order_snapshot),
            },
            AmendOrderOutcome::CancelReplaced { cancelled, placed } => Self::CancelReplaced {
                cancelled: OrderSnapshotEvent::from(cancelled),
                placed: PlaceOrderEvent::from(placed),
            },
            AmendOrderOutcome::Rejected(reason) => Self::Rejected {
                reason: AmendRejectedReasonEvent::from(reason),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AmendRejectedReasonEvent {
    OrderNotFound,
    AmendNotAllowed,
}

impl From<&AmendRejectedReason> for AmendRejectedReasonEvent {
    fn from(value: &AmendRejectedReason) -> Self {
        match value {
            AmendRejectedReason::OrderNotFound => Self::OrderNotFound,
            AmendRejectedReason::AmendNotAllowed => Self::AmendNotAllowed,
        }
    }
}

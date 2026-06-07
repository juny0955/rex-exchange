use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    engine::result::EngineResult,
    kafka::dto::{
        amend_order::AmendOrderEvent, cancel_order::CancelOrderEvent, place_order::PlaceOrderEvent,
    },
};

pub const MATCHING_ENGINE_EVENT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatchingEngineEvent {
    pub event_id: String,
    pub schema_version: u16,
    pub emitted_at: String,
    pub symbol: String,
    pub order_id: String,
    #[serde(flatten)]
    pub body: MatchingEngineEventBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "event_type", content = "data", rename_all = "snake_case")]
pub enum MatchingEngineEventBody {
    Place(PlaceOrderEvent),
    Cancel(CancelOrderEvent),
    Amend(AmendOrderEvent),
}

impl MatchingEngineEvent {
    pub fn from_engine_result(result: &EngineResult) -> Self {
        let (symbol, order_id, body) = match result {
            EngineResult::Place(result) => (
                result.symbol.clone(),
                result.taker_order_id.to_string(),
                MatchingEngineEventBody::Place(PlaceOrderEvent::from(result)),
            ),
            EngineResult::Cancel(result) => (
                result.symbol.clone(),
                result.order_id.to_string(),
                MatchingEngineEventBody::Cancel(CancelOrderEvent::from(result)),
            ),
            EngineResult::Amend(result) => (
                result.symbol.clone(),
                result.order_id.to_string(),
                MatchingEngineEventBody::Amend(AmendOrderEvent::from(result)),
            ),
        };

        Self {
            event_id: Uuid::now_v7().to_string(),
            schema_version: MATCHING_ENGINE_EVENT_SCHEMA_VERSION,
            emitted_at: Utc::now().to_rfc3339(),
            symbol,
            order_id,
            body,
        }
    }

    pub fn kafka_key(&self) -> &str {
        self.symbol.as_str()
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

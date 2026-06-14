use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    engine::result::{EngineResult, EngineResultBody},
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
    pub processed_at: String,
    pub command_id: String,
    pub engine_sequence: u64,
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
        let (symbol, order_id, body) = match &result.body {
            EngineResultBody::Place(result) => (
                result.symbol.clone(),
                result.taker_order_id.to_string(),
                MatchingEngineEventBody::Place(PlaceOrderEvent::from(result)),
            ),
            EngineResultBody::Cancel(result) => (
                result.symbol.clone(),
                result.order_id.to_string(),
                MatchingEngineEventBody::Cancel(CancelOrderEvent::from(result)),
            ),
            EngineResultBody::Amend(result) => (
                result.symbol.clone(),
                result.order_id.to_string(),
                MatchingEngineEventBody::Amend(AmendOrderEvent::from(result)),
            ),
        };

        Self {
            event_id: Uuid::now_v7().to_string(),
            schema_version: MATCHING_ENGINE_EVENT_SCHEMA_VERSION,
            emitted_at: Utc::now().to_rfc3339(),
            processed_at: result.metadata.processed_at.to_rfc3339(),
            command_id: result.metadata.command_id.to_string(),
            engine_sequence: result.metadata.engine_sequence,
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

#[cfg(test)]
mod tests {
    use crate::engine::result::{
        CancelOrderOutcome, CancelOrderResult, CancelRejectedReason, EngineResult, EngineResultBody,
    };

    use super::*;

    const SYMBOL: &str = "BTCUSDT";

    #[test]
    fn execution_report_envelope_includes_command_and_sequence() {
        let command_id = Uuid::from_u128(10);
        let order_id = Uuid::from_u128(20);
        let result = EngineResult::new(
            command_id,
            7,
            EngineResultBody::Cancel(CancelOrderResult {
                symbol: SYMBOL.to_string(),
                order_id,
                outcome: CancelOrderOutcome::Rejected(CancelRejectedReason::OrderNotFound),
            }),
        );

        let event = MatchingEngineEvent::from_engine_result(&result);

        assert_eq!(event.schema_version, 1);
        assert_eq!(event.command_id, command_id.to_string());
        assert_eq!(event.order_id, order_id.to_string());
        assert_eq!(event.symbol, SYMBOL);
        assert_eq!(event.engine_sequence, 7);
        assert_eq!(
            event.processed_at,
            result.metadata.processed_at.to_rfc3339()
        );
        assert_eq!(event.kafka_key(), SYMBOL);
    }
}

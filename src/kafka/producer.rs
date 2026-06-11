use std::time::Duration;

use rdkafka::{
    ClientConfig, ClientContext, Message,
    producer::{BaseRecord, Producer, ProducerContext, ThreadedProducer},
};
use tracing::{debug, error};

use crate::{
    engine::{
        result::EngineResult,
        result_handler::{EngineResultPublisher, PublishResult},
    },
    kafka::{dto::event::MatchingEngineEvent, error::KafkaProducerError},
};

const BOOTSTRAP_SERVERS: &str = "localhost:9092";
const TOPIC: &str = "matching-engine-events";
const CLIENT_ID: &str = "matching-engine";
const MESSAGE_TIMEOUT_MS: &str = "5000";

#[derive(Debug)]
struct DeliveryMeta {
    event_id: String,
    symbol: String,
    order_id: String,
}

struct KafkaDeliveryContext;

impl ClientContext for KafkaDeliveryContext {}
impl ProducerContext for KafkaDeliveryContext {
    type DeliveryOpaque = Box<DeliveryMeta>;

    fn delivery(
        &self,
        delivery_result: &rdkafka::message::DeliveryResult<'_>,
        meta: Self::DeliveryOpaque,
    ) {
        match delivery_result {
            Ok(message) => debug!(
                event_id = %meta.event_id,
                symbol = %meta.symbol,
                order_id = %meta.order_id,
                partition = message.partition(),
                offset = message.offset(),
                "Kafka 메세지 전송 성공"
            ),
            Err((e, _message)) => error!(
                event_id = %meta.event_id,
                symbol = %meta.symbol,
                order_id = %meta.order_id,
                error = %e,
                "Kafka 메세지 전송 실패"
            ),
        }
    }
}

pub struct KafkaProducer {
    producer: ThreadedProducer<KafkaDeliveryContext>,
}

impl EngineResultPublisher for KafkaProducer {
    fn publish(&self, result: &EngineResult) -> PublishResult {
        let event = MatchingEngineEvent::from_engine_result(result);
        self.publish_event(&event)?;
        Ok(())
    }
}

impl KafkaProducer {
    pub fn new() -> Result<Self, KafkaProducerError> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", BOOTSTRAP_SERVERS)
            .set("client.id", CLIENT_ID)
            .set("message.timeout.ms", MESSAGE_TIMEOUT_MS)
            .set("acks", "all")
            .create_with_context(KafkaDeliveryContext)
            .map_err(KafkaProducerError::Create)?;

        Ok(Self { producer })
    }

    pub fn publish_event(&self, event: &MatchingEngineEvent) -> Result<(), KafkaProducerError> {
        let payload = event
            .to_json_bytes()
            .map_err(KafkaProducerError::Serialize)?;

        let meta = Box::new(DeliveryMeta {
            event_id: event.event_id.clone(),
            symbol: event.symbol.clone(),
            order_id: event.order_id.clone(),
        });

        let record: BaseRecord<'_, str, [u8], Box<DeliveryMeta>> =
            BaseRecord::with_opaque_to(TOPIC, meta)
                .key(event.kafka_key())
                .payload(payload.as_slice());

        self.producer
            .send(record)
            .map_err(|(e, _record)| KafkaProducerError::Enqueue(e))?;

        Ok(())
    }
}

impl Drop for KafkaProducer {
    fn drop(&mut self) {
        if let Err(e) = self.producer.flush(Duration::from_secs(5)) {
            error!(
                error = %KafkaProducerError::Flush(e),
                "Kafka producer flush 실패"
            );
        }
    }
}

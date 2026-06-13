use std::{env, time::Duration};

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
const BROKERS_ENV: &str = "MATCHING_ENGINE_KAFKA_BROKERS";
const TOPIC_ENV: &str = "MATCHING_ENGINE_KAFKA_TOPIC";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KafkaProducerConfig {
    bootstrap_servers: String,
    topic: String,
}

impl KafkaProducerConfig {
    pub fn from_env() -> Self {
        let brokers = env::var(BROKERS_ENV).ok();
        let topic = env::var(TOPIC_ENV).ok();
        Self::from_env_values(brokers.as_deref(), topic.as_deref())
    }

    pub fn from_env_values(brokers: Option<&str>, topic: Option<&str>) -> Self {
        Self {
            bootstrap_servers: brokers.unwrap_or(BOOTSTRAP_SERVERS).to_string(),
            topic: topic.unwrap_or(TOPIC).to_string(),
        }
    }

    pub fn bootstrap_servers(&self) -> &str {
        &self.bootstrap_servers
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }
}

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
    config: KafkaProducerConfig,
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
        let config = KafkaProducerConfig::from_env();
        Self::with_config(config)
    }

    fn with_config(config: KafkaProducerConfig) -> Result<Self, KafkaProducerError> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", config.bootstrap_servers())
            .set("client.id", CLIENT_ID)
            .set("message.timeout.ms", MESSAGE_TIMEOUT_MS)
            .set("acks", "all")
            .create_with_context(KafkaDeliveryContext)
            .map_err(KafkaProducerError::Create)?;

        Ok(Self { producer, config })
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
            BaseRecord::with_opaque_to(self.config.topic(), meta)
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

#[cfg(test)]
mod tests {
    use crate::kafka::producer::KafkaProducerConfig;

    #[test]
    fn config_uses_host_defaults_when_env_is_absent() {
        let config = KafkaProducerConfig::from_env_values(None, None);

        assert_eq!(config.bootstrap_servers(), "localhost:9092");
        assert_eq!(config.topic(), "matching-engine-events");
    }

    #[test]
    fn config_uses_env_overrides_for_docker_network() {
        let config = KafkaProducerConfig::from_env_values(Some("kafka:9092"), Some("events"));

        assert_eq!(config.bootstrap_servers(), "kafka:9092");
        assert_eq!(config.topic(), "events");
    }
}

use std::{error::Error, fmt};

use rdkafka::error::KafkaError;

#[derive(Debug)]
pub enum KafkaProducerError {
    Create(KafkaError),
    Serialize(serde_json::Error),
    Enqueue(KafkaError),
    Flush(KafkaError),
}

impl fmt::Display for KafkaProducerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KafkaProducerError::Create(e) => write!(f, "Kafka producer 초기화 실패: {e}"),
            KafkaProducerError::Serialize(e) => write!(f, "Kafka message 직렬화 실패: {e}"),
            KafkaProducerError::Enqueue(e) => write!(f, "Kafka message enqueue 실패: {e}"),
            KafkaProducerError::Flush(e) => write!(f, "Kafka producer flush 실패: {e}"),
        }
    }
}

impl Error for KafkaProducerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Create(e) | Self::Enqueue(e) | Self::Flush(e) => Some(e),
            Self::Serialize(e) => Some(e),
        }
    }
}

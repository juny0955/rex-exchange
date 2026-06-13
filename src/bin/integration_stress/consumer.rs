//! Kafka 컨슈머 + 상관 로직을 하나의 태스크로 통합한다.
//! 단일 태스크가 `CorrelatorState`를 단독 소유하므로 송신/수신 갱신에 락이 필요 없다.
//!
//! `auto.offset.reset=latest` + 매 실행 고유 group.id로 이전 실행 잔여 이벤트를 건너뛴다.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use rdkafka::{
    ClientConfig, Message,
    admin::{AdminClient, AdminOptions, NewTopic, TopicReplication},
    client::DefaultClientContext,
    consumer::{Consumer, StreamConsumer},
    error::RDKafkaErrorCode,
};
use serde_json::Value;
use tokio::{
    sync::{mpsc, oneshot},
    time::{sleep, timeout},
};

use crate::integration_stress::metrics::{CorrelatorState, CorrelatorSummary, SentNotice};

const READY_TIMEOUT: Duration = Duration::from_secs(10);

/// 컨슈머 태스크에 보내는 제어 메시지.
pub enum ControlMsg {
    Settle {
        expected: u64,
        timeout: Duration,
        ack: oneshot::Sender<SettleResult>,
    },
    SettleAndReset {
        expected: u64,
        timeout: Duration,
        ack: oneshot::Sender<SettleResult>,
    },
    Finish {
        expected: u64,
        timeout: Duration,
        ack: oneshot::Sender<SettleResult>,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettleResult {
    pub settled: bool,
    pub summary: CorrelatorSummary,
}

/// 토픽을 미리 생성한다(이미 있으면 무시). 최신 Kafka는 컨슈머 측 자동 토픽 생성을 막으므로,
/// 컨슈머가 파티션 할당을 받으려면 토픽이 먼저 존재해야 한다.
pub async fn ensure_topic(
    brokers: &str,
    topic: &str,
    partitions: i32,
) -> Result<(), rdkafka::error::KafkaError> {
    let admin: AdminClient<DefaultClientContext> = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .create()?;

    let new_topic = NewTopic::new(topic, partitions, TopicReplication::Fixed(1));
    let results = admin
        .create_topics([&new_topic], &AdminOptions::new())
        .await?;

    for result in results {
        match result {
            Ok(_) => {}
            Err((_, RDKafkaErrorCode::TopicAlreadyExists)) => {}
            Err((name, code)) => {
                eprintln!("토픽 생성 경고: {name} -> {code:?}");
            }
        }
    }

    Ok(())
}

pub fn build_consumer(
    brokers: &str,
    topic: &str,
    group_id: &str,
) -> Result<StreamConsumer, rdkafka::error::KafkaError> {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("group.id", group_id)
        .set("auto.offset.reset", "latest")
        .set("enable.auto.commit", "false")
        .set("allow.auto.create.topics", "true")
        .create()?;

    consumer.subscribe(&[topic])?;
    Ok(consumer)
}

/// 컨슈머 태스크 본체. 파티션 할당이 끝나면 `ready_tx`로 신호하고, 이후 Kafka 수신과
/// 송신 알림(`sent_rx`)을 함께 처리한다. `shutdown_rx` 수신 시 상관 결과를 반환한다.
pub async fn run_consumer(
    consumer: StreamConsumer,
    mut sent_rx: mpsc::UnboundedReceiver<SentNotice>,
    mut control_rx: mpsc::UnboundedReceiver<ControlMsg>,
    received: Arc<AtomicU64>,
    ready_tx: oneshot::Sender<()>,
) -> CorrelatorSummary {
    let mut state = CorrelatorState::default();

    await_assignment(&consumer, &received, &mut state).await;
    let _ = ready_tx.send(());

    let mut sends_done = false;

    loop {
        tokio::select! {
            biased;

            Some(control) = control_rx.recv() => match control {
                ControlMsg::Settle { expected, timeout, ack } => {
                    let result = settle_correlator(
                        &consumer,
                        &mut sent_rx,
                        &mut state,
                        &received,
                        expected,
                        timeout,
                        &mut sends_done,
                    ).await;
                    let _ = ack.send(result);
                }
                ControlMsg::SettleAndReset { expected, timeout, ack } => {
                    let result = settle_correlator(
                        &consumer,
                        &mut sent_rx,
                        &mut state,
                        &received,
                        expected,
                        timeout,
                        &mut sends_done,
                    ).await;
                    state = CorrelatorState::default();
                    received.store(0, Ordering::Relaxed);
                    let _ = ack.send(result);
                }
                ControlMsg::Finish { expected, timeout, ack } => {
                    let result = settle_correlator(
                        &consumer,
                        &mut sent_rx,
                        &mut state,
                        &received,
                        expected,
                        timeout,
                        &mut sends_done,
                    ).await;
                    let _ = ack.send(result);
                    break;
                }
            },

            msg = consumer.recv() => match msg {
                Ok(message) => handle_message(&mut state, &received, message.payload()),
                Err(e) => eprintln!("Kafka 수신 오류: {e}"),
            },

            sent = sent_rx.recv(), if !sends_done => match sent {
                Some(notice) => state.on_sent(notice.order_id, notice.sent_at, notice.acked_at),
                None => sends_done = true,
            },
        }
    }

    state.finish()
}

async fn settle_correlator(
    consumer: &StreamConsumer,
    sent_rx: &mut mpsc::UnboundedReceiver<SentNotice>,
    state: &mut CorrelatorState,
    received: &Arc<AtomicU64>,
    expected: u64,
    timeout_duration: Duration,
    sends_done: &mut bool,
) -> SettleResult {
    let deadline = Instant::now() + timeout_duration;

    while !state.is_settled(expected) {
        if Instant::now() >= deadline {
            return SettleResult {
                settled: false,
                summary: state.snapshot(),
            };
        }

        tokio::select! {
            msg = consumer.recv() => match msg {
                Ok(message) => handle_message(state, received, message.payload()),
                Err(e) => eprintln!("Kafka 수신 오류(settle 중): {e}"),
            },

            sent = sent_rx.recv(), if !*sends_done => match sent {
                Some(notice) => state.on_sent(notice.order_id, notice.sent_at, notice.acked_at),
                None => *sends_done = true,
            },

            _ = sleep(Duration::from_millis(2)) => {}
        }
    }

    SettleResult {
        settled: true,
        summary: state.snapshot(),
    }
}

/// 파티션 할당이 완료될 때까지 폴링한다. 토픽이 아직 없거나 리밸런스 중이면 잠시 대기한다.
async fn await_assignment(
    consumer: &StreamConsumer,
    received: &Arc<AtomicU64>,
    state: &mut CorrelatorState,
) {
    let deadline = Instant::now() + READY_TIMEOUT;

    loop {
        if assignment_count(consumer) > 0 {
            return;
        }
        if Instant::now() >= deadline {
            eprintln!(
                "경고: {READY_TIMEOUT:?} 안에 파티션 할당을 받지 못했습니다. 토픽 상태를 확인하세요"
            );
            return;
        }

        match timeout(Duration::from_millis(200), consumer.recv()).await {
            Ok(Ok(message)) => handle_message(state, received, message.payload()),
            Ok(Err(e)) => eprintln!("Kafka 수신 오류(할당 대기 중): {e}"),
            Err(_elapsed) => {}
        }
    }
}

fn handle_message(state: &mut CorrelatorState, received: &Arc<AtomicU64>, payload: Option<&[u8]>) {
    let now = Instant::now();

    let Some(payload) = payload else {
        return;
    };

    let value: Value = match serde_json::from_slice(payload) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("Kafka 메시지 JSON 파싱 실패: {e}");
            return;
        }
    };

    let order_id = value.get("order_id").and_then(Value::as_str);
    let event_id = value.get("event_id").and_then(Value::as_str);

    if let (Some(order_id), Some(event_id)) = (order_id, event_id) {
        received.fetch_add(1, Ordering::Relaxed);
        state.on_recv(order_id.to_string(), event_id.to_string(), now);
    }
}

fn assignment_count(consumer: &StreamConsumer) -> usize {
    consumer.assignment().map(|tpl| tpl.count()).unwrap_or(0)
}

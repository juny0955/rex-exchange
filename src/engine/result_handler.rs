use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use crossbeam::channel::Receiver;
use tracing::{debug, error};

use crate::engine::result::EngineResult;

pub type PublishResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub trait EngineResultPublisher: Send + 'static {
    fn publish(&self, result: &EngineResult) -> PublishResult;
}

#[derive(Debug, Clone)]
pub struct EngineHealth {
    publisher_healthy: Arc<AtomicBool>,
}

impl Default for EngineHealth {
    fn default() -> Self {
        Self {
            publisher_healthy: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl EngineHealth {
    pub fn is_publisher_healthy(&self) -> bool {
        self.publisher_healthy.load(Ordering::SeqCst)
    }

    fn mark_publisher_unhealthy(&self) {
        self.publisher_healthy.store(false, Ordering::SeqCst);
    }

    fn mark_publisher_healthy(&self) {
        self.publisher_healthy.store(true, Ordering::SeqCst);
    }
}

pub struct EngineResultHandler {
    result_rx: Receiver<EngineResult>,
    publisher: Box<dyn EngineResultPublisher>,
    health: EngineHealth,
}

impl EngineResultHandler {
    pub fn new(
        result_rx: Receiver<EngineResult>,
        publisher: Box<dyn EngineResultPublisher>,
        health: EngineHealth,
    ) -> Self {
        Self {
            result_rx,
            publisher,
            health,
        }
    }

    pub fn run(self) {
        while let Ok(result) = self.result_rx.recv() {
            debug!(?result, "엔진 결과 수신");

            self.publish_until_success(&result);
        }
    }

    fn publish_until_success(&self, result: &EngineResult) {
        let mut delay = Duration::from_millis(10);

        loop {
            match self.publisher.publish(result) {
                Ok(()) => {
                    self.health.mark_publisher_healthy();
                    return;
                }
                Err(e) => {
                    self.health.mark_publisher_unhealthy();
                    error!(error = %e, "엔진 결과 발행 실패: 재시도 예정");
                    thread::sleep(delay);
                    delay = (delay * 2).min(Duration::from_secs(1));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use crossbeam::channel;
    use uuid::Uuid;

    use super::*;
    use crate::engine::result::{
        CancelOrderOutcome, CancelOrderResult, CancelRejectedReason, EngineResult, EngineResultBody,
    };

    const SYMBOL: &str = "BTCUSDT";

    fn make_cancel_rejected_result(order_id: Uuid) -> EngineResult {
        EngineResult::new(
            Uuid::now_v7(),
            1,
            EngineResultBody::Cancel(CancelOrderResult {
                symbol: SYMBOL.to_string(),
                order_id,
                outcome: CancelOrderOutcome::Rejected(CancelRejectedReason::OrderNotFound),
            }),
        )
    }

    struct RecordingPublisher {
        published_order_ids: Arc<Mutex<Vec<Uuid>>>,
    }

    impl EngineResultPublisher for RecordingPublisher {
        fn publish(&self, result: &EngineResult) -> PublishResult {
            let EngineResultBody::Cancel(result) = &result.body else {
                panic!("Cancel 결과여야 함");
            };

            self.published_order_ids
                .lock()
                .unwrap()
                .push(result.order_id);

            Ok(())
        }
    }

    #[test]
    fn 수신한_엔진_결과를_publisher로_전달_테스트() {
        let (result_tx, result_rx) = channel::unbounded();
        let published_order_ids = Arc::new(Mutex::new(Vec::new()));

        let order_id = Uuid::now_v7();
        result_tx
            .send(make_cancel_rejected_result(order_id))
            .expect("엔진 결과 전송 실패");
        drop(result_tx);

        let health = EngineHealth::default();
        EngineResultHandler::new(
            result_rx,
            Box::new(RecordingPublisher {
                published_order_ids: Arc::clone(&published_order_ids),
            }),
            health,
        )
        .run();

        assert_eq!(*published_order_ids.lock().unwrap(), vec![order_id]);
    }

    struct FailingOncePublisher {
        calls: Arc<AtomicUsize>,
    }

    impl EngineResultPublisher for FailingOncePublisher {
        fn publish(&self, _result: &EngineResult) -> PublishResult {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);

            if call == 0 {
                return Err(io::Error::other("publish failed").into());
            }

            Ok(())
        }
    }

    #[test]
    fn publish_실패_후에도_다음_결과_처리_테스트() {
        let (result_tx, result_rx) = channel::unbounded();
        let calls = Arc::new(AtomicUsize::new(0));

        result_tx
            .send(make_cancel_rejected_result(Uuid::now_v7()))
            .expect("첫 번째 엔진 결과 전송 실패");
        result_tx
            .send(make_cancel_rejected_result(Uuid::now_v7()))
            .expect("두 번째 엔진 결과 전송 실패");
        drop(result_tx);

        let health = EngineHealth::default();
        EngineResultHandler::new(
            result_rx,
            Box::new(FailingOncePublisher {
                calls: Arc::clone(&calls),
            }),
            health.clone(),
        )
        .run();

        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert!(health.is_publisher_healthy());
    }
}

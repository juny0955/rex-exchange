use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use matching_engine::engine::{
    result::EngineResult,
    result_handler::{EngineResultPublisher, PublishResult},
};

use crate::runtime_stress::{latency::LatencyCorrelator, stats::ResultStats};

pub struct CountingPublisher {
    stats: Arc<ResultStats>,
    latency: Arc<Mutex<LatencyCorrelator>>,
    delay: Duration,
}

impl CountingPublisher {
    pub fn new(
        stats: Arc<ResultStats>,
        latency: Arc<Mutex<LatencyCorrelator>>,
        delay: Duration,
    ) -> Self {
        Self {
            stats,
            latency,
            delay,
        }
    }
}

impl EngineResultPublisher for CountingPublisher {
    fn publish(&self, result: &EngineResult) -> PublishResult {
        // 인위적 발행 지연(sleep) 이전에 수신 시각을 캡처해, 해당 결과 자체의 delay는
        // 지연 표본에서 제외하고 앞선 결과들 뒤에 줄 선 대기 시간만 반영한다.
        let received_at = Instant::now();

        if !self.delay.is_zero() {
            thread::sleep(self.delay);
        }

        // wait_until_published가 최종 published 카운트를 관측한 시점에 모든 지연 표본이
        // correlator에 들어가 있도록, 카운터 증가보다 먼저 기록한다.
        self.latency
            .lock()
            .unwrap()
            .on_recv(result.order_id(), received_at);

        self.stats.add_published();
        match result {
            EngineResult::Place(result) => {
                self.stats
                    .add_place_result(result.trades.len(), result.updated_makers.len());
            }
            EngineResult::Cancel(_) => {
                self.stats.add_cancel_result();
            }
            EngineResult::Amend(_) => {
                self.stats.add_amend_result();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use matching_engine::engine::result::{
        CancelOrderOutcome, CancelOrderResult, CancelRejectedReason,
    };
    use uuid::Uuid;

    use super::*;

    #[test]
    fn publish_matches_recorded_send_and_counts_result() {
        let stats = Arc::new(ResultStats::default());
        let (recorder, correlator) = LatencyCorrelator::channel();
        let correlator = Arc::new(Mutex::new(correlator));
        let publisher =
            CountingPublisher::new(Arc::clone(&stats), Arc::clone(&correlator), Duration::ZERO);

        let order_id = Uuid::now_v7();
        recorder.record(order_id, Instant::now());

        publisher
            .publish(&EngineResult::Cancel(CancelOrderResult {
                symbol: "BTCUSDT".to_string(),
                order_id,
                outcome: CancelOrderOutcome::Rejected(CancelRejectedReason::OrderNotFound),
            }))
            .expect("publish 실패");

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.published, 1);
        assert_eq!(snapshot.cancel_results, 1);
        assert_eq!(correlator.lock().unwrap().take_summary().count, 1);
    }
}

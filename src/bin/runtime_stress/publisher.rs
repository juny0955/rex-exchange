use std::{sync::Arc, thread, time::Duration};

use matching_engine::engine::{
    result::EngineResult,
    result_handler::{EngineResultPublisher, PublishResult},
};

use crate::runtime_stress::stats::ResultStats;

pub struct CountingPublisher {
    stats: Arc<ResultStats>,
    delay: Duration,
}

impl CountingPublisher {
    pub fn new(stats: Arc<ResultStats>, delay: Duration) -> Self {
        Self { stats, delay }
    }
}

impl EngineResultPublisher for CountingPublisher {
    fn publish(&self, result: &EngineResult) -> PublishResult {
        if !self.delay.is_zero() {
            thread::sleep(self.delay);
        }

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

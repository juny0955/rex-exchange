use crossbeam::channel::Receiver;
use tracing::info;

use crate::engine::result::EngineResult;

pub struct EngineResultHandler {
    result_rx: Receiver<EngineResult>,
}

impl EngineResultHandler {
    pub fn new(result_rx: Receiver<EngineResult>) -> Self {
        Self { result_rx }
    }

    pub fn run(self) {
        while let Ok(result) = self.result_rx.recv() {
            info!(?result, "엔진 결과 수신");
        }
    }
}

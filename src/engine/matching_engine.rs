use crossbeam::channel::{Receiver, Sender};
use tracing::{error, info};

use crate::engine::{command::EngineCommand, orderbook::OrderBook, result::EngineResult};

mod amend;
mod matching;
mod place;

#[cfg(feature = "bench-internals")]
mod bench;
#[cfg(test)]
mod tests;

pub struct MatchingEngine {
    symbol: String,
    engine_rx: Receiver<EngineCommand>,
    result_tx: Sender<EngineResult>,
    orderbook: OrderBook,
}

impl MatchingEngine {
    pub fn new(
        symbol: String,
        engine_rx: Receiver<EngineCommand>,
        result_tx: Sender<EngineResult>,
    ) -> Self {
        Self {
            symbol,
            engine_rx,
            result_tx,
            orderbook: OrderBook::default(),
        }
    }

    /// 엔진 실행
    pub fn run(&mut self) {
        info!(symbol = %self.symbol, "엔진 시작");
        while let Ok(command) = self.engine_rx.recv() {
            let order_id = command.order_id();
            let result = match command {
                EngineCommand::Place(order) => self.place_order(order),
                EngineCommand::Cancel(order_id) => self.cancel_order(order_id),
                EngineCommand::Amend(cmd) => self.amend_order(cmd),
            };

            if let Err(e) = self.result_tx.send(result) {
                error!(symbol = %self.symbol, order_id = %order_id, error = %e, "매칭 결과 전송 오류");
            }
        }
    }
}

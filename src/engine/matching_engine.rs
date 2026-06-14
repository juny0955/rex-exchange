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
    next_sequence: u64,
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
            next_sequence: 0,
        }
    }

    /// 엔진 실행
    pub fn run(&mut self) {
        info!(symbol = %self.symbol, "엔진 시작");
        while let Ok(command) = self.engine_rx.recv() {
            let command_id = command.command_id();
            let order_id = command.order_id();
            let result_body = match command {
                EngineCommand::Place(command) => self.place_order(command.order),
                EngineCommand::Cancel(command) => self.cancel_order(command.order_id),
                EngineCommand::Amend(cmd) => self.amend_order(cmd),
            };
            self.next_sequence += 1;
            let result = EngineResult::new(command_id, self.next_sequence, result_body);

            if let Err(e) = self.result_tx.send(result) {
                error!(symbol = %self.symbol, order_id = %order_id, error = %e, "매칭 결과 전송 오류");
            }
        }
    }
}

use crate::engine::{
    command::EngineCommand, matching_engine::MatchingEngine, result::EngineResult,
};
use crossbeam::channel::Sender;
use std::{collections::HashMap, thread::JoinHandle};
use tracing::error;

pub struct EngineDispatcher {
    senders: HashMap<String, Sender<EngineCommand>>,
    handles: Vec<JoinHandle<()>>,
}

impl EngineDispatcher {
    pub fn new(symbols: Vec<String>, result_tx: Sender<EngineResult>) -> Self {
        let mut senders = HashMap::new();
        let mut handles = Vec::new();

        for symbol in symbols {
            let (engine_tx, engine_rx) = crossbeam::channel::bounded::<EngineCommand>(1024);

            let sym = symbol.clone();
            let res_tx = result_tx.clone();
            let handle = std::thread::spawn(move || {
                MatchingEngine::new(sym, engine_rx, res_tx).run();
            });

            handles.push(handle);
            senders.insert(symbol, engine_tx);
        }

        Self { senders, handles }
    }

    pub fn dispatch(&self, symbol: &str, cmd: EngineCommand) {
        let order_id = cmd.order_id();
        let Some(sender) = self.senders.get(symbol) else {
            error!(symbol = symbol, order_id = %order_id, "등록되지 않은 심볼");
            return;
        };

        if let Err(e) = sender.try_send(cmd) {
            error!(symbol = symbol, order_id = %order_id, error = %e, "매칭엔진 진입 오류");
        }
    }
}

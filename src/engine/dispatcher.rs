use crate::engine::{
    command::EngineCommand, matching_engine::MatchingEngine, result::EngineResult,
};
use crossbeam::channel::{Sender, TrySendError};
use std::{collections::HashMap, thread::JoinHandle};
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum DispatchError {
    UnknownSymbol { symbol: String, order_id: Uuid },
    EngineStopped { symbol: String, order_id: Uuid },
    ChannelFull { symbol: String, order_id: Uuid },
}

pub struct EngineDispatcher {
    senders: HashMap<String, Sender<EngineCommand>>,
    handles: Vec<JoinHandle<()>>,
}

impl EngineDispatcher {
    pub fn new(symbols: Vec<String>, result_tx: Sender<EngineResult>) -> Self {
        let mut senders = HashMap::new();
        let mut handles = Vec::new();

        for symbol in symbols {
            if senders.contains_key(&symbol) {
                error!(symbol = %symbol, "중복 심볼 등록");
                continue;
            }

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

    pub fn dispatch(&self, symbol: &str, cmd: EngineCommand) -> Result<(), DispatchError> {
        let order_id = cmd.order_id();
        let Some(sender) = self.senders.get(symbol) else {
            error!(symbol = symbol, order_id = %order_id, "등록되지 않은 심볼");
            return Err(DispatchError::UnknownSymbol {
                symbol: symbol.to_string(),
                order_id,
            });
        };

        if let Err(e) = sender.try_send(cmd) {
            return match e {
                TrySendError::Disconnected(_) => {
                    error!(symbol = symbol, order_id = %order_id, error = %e, "매칭엔진 채널 종료");
                    Err(DispatchError::EngineStopped {
                        symbol: symbol.to_string(),
                        order_id,
                    })
                }
                TrySendError::Full(_) => {
                    error!(symbol = symbol, order_id = %order_id, error = %e, "매칭엔진 큐 가득 참");
                    Err(DispatchError::ChannelFull {
                        symbol: symbol.to_string(),
                        order_id,
                    })
                }
            };
        }

        Ok(())
    }

    pub fn shutdown(self) {
        drop(self.senders);

        for handle in self.handles {
            if let Err(e) = handle.join() {
                error!(?e, "매칭엔진 종료 중 오류발생");
            }
        }
    }
}

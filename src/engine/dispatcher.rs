use crate::engine::command::EngineCommand;
use crossbeam::channel::{Sender, TrySendError};
use std::collections::HashMap;
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum DispatchError {
    UnknownSymbol { symbol: String, order_id: Uuid },
    EngineStopped { symbol: String, order_id: Uuid },
    ChannelFull { symbol: String, order_id: Uuid },
}

#[derive(Debug, Clone)]
pub struct EngineDispatcher {
    senders: HashMap<String, Sender<EngineCommand>>,
}

impl EngineDispatcher {
    pub(crate) fn new(senders: HashMap<String, Sender<EngineCommand>>) -> Self {
        Self { senders }
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
}

#[cfg(test)]
mod tests {
    use crossbeam::channel::Receiver;

    use super::*;

    const SYMBOL: &str = "BTCUSDT";

    fn make_dispatcher_with_receiver(
        capacity: usize,
    ) -> (EngineDispatcher, Receiver<EngineCommand>) {
        let (engine_tx, engine_rx) = crossbeam::channel::bounded(capacity);
        let mut senders = HashMap::new();
        senders.insert(SYMBOL.to_string(), engine_tx);

        (EngineDispatcher::new(senders), engine_rx)
    }

    fn make_cmd() -> EngineCommand {
        EngineCommand::Cancel(Uuid::now_v7())
    }

    #[test]
    fn 정상_dispatch_테스트() {
        let (dispatcher, engine_rx) = make_dispatcher_with_receiver(1);

        let cmd = make_cmd();
        let order_id = cmd.order_id();
        assert!(dispatcher.dispatch(SYMBOL, cmd).is_ok());

        let received = engine_rx.try_recv().expect("dispatch된 command 수신 실패");
        assert_eq!(received.order_id(), order_id);
    }

    #[test]
    fn rx_channel_닫힌_후_dispatch_테스트() {
        let (dispatcher, engine_rx) = make_dispatcher_with_receiver(1);
        drop(engine_rx);

        let result = dispatcher.dispatch(SYMBOL, make_cmd());

        assert!(matches!(result, Err(DispatchError::EngineStopped { .. })));
    }

    #[test]
    fn channel_full_dispatch_테스트() {
        let (dispatcher, _engine_rx) = make_dispatcher_with_receiver(0);

        let result = dispatcher.dispatch(SYMBOL, make_cmd());

        assert!(matches!(result, Err(DispatchError::ChannelFull { .. })));
    }

    #[test]
    fn 미등록_심볼_dispatch_테스트() {
        let (dispatcher, _) = make_dispatcher_with_receiver(1);
        let result = dispatcher.dispatch("ETHUSDT", make_cmd());
        assert!(matches!(result, Err(DispatchError::UnknownSymbol { .. })));
    }
}

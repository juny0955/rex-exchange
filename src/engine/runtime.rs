use std::{collections::HashMap, thread::JoinHandle};

use tracing::error;

use crate::engine::{
    command::EngineCommand,
    dispatcher::EngineDispatcher,
    matching_engine::MatchingEngine,
    result::EngineResult,
    result_handler::{EngineHealth, EngineResultHandler, EngineResultPublisher},
};

pub struct EngineRuntime {
    dispatcher: EngineDispatcher,
    engine_handles: Vec<JoinHandle<()>>,
    result_handle: JoinHandle<()>,
}

impl EngineRuntime {
    pub fn new(symbols: Vec<String>, result_publisher: Box<dyn EngineResultPublisher>) -> Self {
        let (result_tx, result_rx) = crossbeam::channel::bounded::<EngineResult>(1024);
        let health = EngineHealth::default();
        let result_health = health.clone();

        let result_handle = std::thread::spawn(move || {
            EngineResultHandler::new(result_rx, result_publisher, result_health).run();
        });

        let mut senders = HashMap::new();
        let mut engine_handles = Vec::new();

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

            senders.insert(symbol, engine_tx);
            engine_handles.push(handle);
        }

        let dispatcher = EngineDispatcher::new(senders, health);

        Self {
            dispatcher,
            engine_handles,
            result_handle,
        }
    }

    pub fn dispatcher(&self) -> EngineDispatcher {
        self.dispatcher.clone()
    }

    pub fn shutdown(self) {
        let Self {
            dispatcher,
            engine_handles,
            result_handle,
        } = self;
        drop(dispatcher);

        for handle in engine_handles {
            if let Err(e) = handle.join() {
                error!(?e, "매칭엔진 종료 중 오류발생");
            }
        }

        if let Err(e) = result_handle.join() {
            error! {?e, "엔진 결과 핸들러 종료 중 오류발생"};
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::result_handler::PublishResult;

    use super::*;

    struct NoopPublisher;
    impl EngineResultPublisher for NoopPublisher {
        fn publish(&self, _result: &EngineResult) -> PublishResult {
            Ok(())
        }
    }

    const SYMBOL: &str = "BTCUSDT";

    #[test]
    fn 중복_심볼_무시_테스트() {
        let runtime = EngineRuntime::new(
            vec![SYMBOL.to_string(), SYMBOL.to_string()],
            Box::new(NoopPublisher),
        );
        assert_eq!(runtime.engine_handles.len(), 1);
        runtime.shutdown();
    }

    #[test]
    fn shutdown_정상_종료_테스트() {
        let runtime = EngineRuntime::new(vec![SYMBOL.to_string()], Box::new(NoopPublisher));
        runtime.shutdown();
    }
}

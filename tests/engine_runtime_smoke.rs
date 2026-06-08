use matching_engine::engine::{
    command::EngineCommand,
    result::EngineResult,
    result_handler::{EngineResultPublisher, PublishResult},
    runtime::EngineRuntime,
};
use uuid::Uuid;

const SYMBOL: &str = "BTCUSDT";

struct NoopPublisher;
impl EngineResultPublisher for NoopPublisher {
    fn publish(&self, _result: &EngineResult) -> PublishResult {
        Ok(())
    }
}

#[test]
fn runtime_dispatch_후_shutdown_정상_종료_테스트() {
    let runtime = EngineRuntime::new(vec![SYMBOL.to_string()], Box::new(NoopPublisher));
    let dispatcher = runtime.dispatcher();

    let order_id = Uuid::now_v7();
    dispatcher
        .dispatch(SYMBOL, EngineCommand::Cancel(order_id))
        .expect("engine command dispatch 실패");
    drop(dispatcher);

    runtime.shutdown();
}

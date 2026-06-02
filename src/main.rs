use matching_engine::{
    engine::{
        dispatcher::EngineDispatcher, result::EngineResult, result_handler::EngineResultHandler,
    },
    init::init,
};

fn main() {
    init();

    let (result_tx, result_rx) = crossbeam::channel::bounded::<EngineResult>(1024);

    let symbols = vec!["BTCUSDT".to_string()];
    let _engine_dispatcher = EngineDispatcher::new(symbols, result_tx);

    std::thread::spawn(move || {
        EngineResultHandler::new(result_rx).run();
    });

    std::thread::park();
}

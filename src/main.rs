use std::net::SocketAddr;

use matching_engine::{
    engine::{
        dispatcher::EngineDispatcher, result::EngineResult, result_handler::EngineResultHandler,
    },
    grpc::{
        engine::matching_engine_service_server::MatchingEngineServiceServer,
        matching_engine_grpc::MatchingEngineGrpcService,
    },
    init::init,
};
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();

    let (result_tx, result_rx) = crossbeam::channel::bounded::<EngineResult>(1024);

    let symbols = vec!["BTCUSDT".to_string()];
    let engine_dispatcher = EngineDispatcher::new(symbols, result_tx);

    std::thread::spawn(move || {
        EngineResultHandler::new(result_rx).run();
    });

    let grpc_service = MatchingEngineGrpcService::new(engine_dispatcher);

    let addr: SocketAddr = "0.0.0.0:50051".parse()?;

    info!("gRPC server 시작");

    Server::builder()
        .add_service(MatchingEngineServiceServer::new(grpc_service))
        .serve(addr)
        .await?;

    Ok(())
}

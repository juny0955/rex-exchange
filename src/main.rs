use std::net::SocketAddr;

use matching_engine::{
    engine::runtime::EngineRuntime,
    grpc::{
        engine::matching_engine_service_server::MatchingEngineServiceServer,
        matching_engine_grpc::MatchingEngineGrpcService,
    },
    init::init,
    kafka::producer::KafkaProducer,
};
use tonic::transport::Server;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init();

    let symbols = vec!["BTCUSDT".to_string()];
    let kafka_producer = KafkaProducer::new()?;
    let runtime = EngineRuntime::new(symbols, Box::new(kafka_producer));

    let grpc_service = MatchingEngineGrpcService::new(runtime.dispatcher());

    let addr: SocketAddr = "0.0.0.0:50051".parse()?;

    info!("gRPC server 시작");

    let server_result = Server::builder()
        .add_service(MatchingEngineServiceServer::new(grpc_service))
        .serve_with_shutdown(addr, async {
            if let Err(e) = tokio::signal::ctrl_c().await {
                error!(?e, "종료 시그널 수신 오류");
            }
        })
        .await;

    runtime.shutdown();
    server_result?;
    Ok(())
}

//! gRPC 클라이언트 연결과 `EngineCommand` → proto 요청 변환.
//! 변환은 순수 함수로 분리해 네트워크 없이 단위 테스트가 가능하다.

use matching_engine::{
    domain::order::{Order, OrderSize, Side, TimeInForce},
    engine::command::{AmendOrderCommand, EngineCommand},
    grpc::engine::{
        AmendOrderRequest, CancelOrderRequest, OrderType as ProtoOrderType, PlaceOrderRequest,
        Side as ProtoSide, SubmitStatus, TimeInForce as ProtoTimeInForce,
        matching_engine_service_client::MatchingEngineServiceClient, place_order_request::Size,
    },
};
use tonic::{Code, transport::Channel};
use uuid::Uuid;

pub type GrpcClient = MatchingEngineServiceClient<Channel>;

/// 단일 gRPC 명령 호출의 결과 분류.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendOutcome {
    /// 접수 성공(SUBMITTED). Kafka 이벤트 정확히 1건을 생성한다.
    Submitted,
    /// 서버가 SUBMITTED 외 상태를 반환(현재 서버 경로상 거의 없음).
    Rejected,
    /// 503 RESOURCE_EXHAUSTED = 엔진 채널 포화(백프레셔).
    ResourceExhausted,
    /// 그 외 gRPC 에러(연결/타임아웃/invalid_argument 등).
    Error,
}

pub async fn connect(endpoint: &str) -> Result<GrpcClient, tonic::transport::Error> {
    MatchingEngineServiceClient::connect(endpoint.to_string()).await
}

/// 명령을 gRPC로 전송하고 결과를 분류한다.
pub async fn send(client: &mut GrpcClient, symbol: &str, command: &EngineCommand) -> SendOutcome {
    match command {
        EngineCommand::Place(order) => {
            classify(client.place_order(place_request(order)).await.map(|r| {
                let status = r.into_inner().status;
                status == SubmitStatus::Submitted as i32
            }))
        }
        EngineCommand::Cancel(order_id) => classify(
            client
                .cancel_order(cancel_request(*order_id, symbol))
                .await
                .map(|r| r.into_inner().status == SubmitStatus::Submitted as i32),
        ),
        EngineCommand::Amend(cmd) => classify(
            client
                .amend_order(amend_request(cmd, symbol))
                .await
                .map(|r| r.into_inner().status == SubmitStatus::Submitted as i32),
        ),
    }
}

fn classify(result: Result<bool, tonic::Status>) -> SendOutcome {
    match result {
        Ok(true) => SendOutcome::Submitted,
        Ok(false) => SendOutcome::Rejected,
        Err(status) if status.code() == Code::ResourceExhausted => SendOutcome::ResourceExhausted,
        Err(_) => SendOutcome::Error,
    }
}

pub fn place_request(order: &Order) -> PlaceOrderRequest {
    let size = match order.size {
        OrderSize::Base(qty) => Size::BaseQty(qty.to_string()),
        OrderSize::Quote(quote) => Size::QuoteQty(quote.to_string()),
    };

    PlaceOrderRequest {
        order_id: order.order_id.to_string(),
        symbol: order.symbol.clone(),
        side: side_to_proto(order.side),
        order_type: order_type_to_proto(order.order_type),
        tif: tif_to_proto(order.tif),
        price: order.price.map(|p| p.to_string()),
        size: Some(size),
    }
}

pub fn cancel_request(order_id: Uuid, symbol: &str) -> CancelOrderRequest {
    CancelOrderRequest {
        order_id: order_id.to_string(),
        symbol: symbol.to_string(),
    }
}

pub fn amend_request(cmd: &AmendOrderCommand, symbol: &str) -> AmendOrderRequest {
    AmendOrderRequest {
        order_id: cmd.order_id.to_string(),
        symbol: symbol.to_string(),
        new_price: cmd.price.map(|p| p.to_string()),
        new_base_qty: cmd.base_qty.map(|q| q.to_string()),
    }
}

fn side_to_proto(side: Side) -> i32 {
    match side {
        Side::Buy => ProtoSide::Buy as i32,
        Side::Sell => ProtoSide::Sell as i32,
    }
}

fn order_type_to_proto(order_type: matching_engine::domain::order::OrderType) -> i32 {
    use matching_engine::domain::order::OrderType;
    match order_type {
        OrderType::Limit => ProtoOrderType::Limit as i32,
        OrderType::Market => ProtoOrderType::Market as i32,
    }
}

fn tif_to_proto(tif: TimeInForce) -> i32 {
    match tif {
        TimeInForce::GTC => ProtoTimeInForce::Gtc as i32,
        TimeInForce::IOC => ProtoTimeInForce::Ioc as i32,
        TimeInForce::FOK => ProtoTimeInForce::Fok as i32,
    }
}

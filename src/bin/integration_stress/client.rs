//! gRPC 클라이언트 연결과 `EngineCommand` → proto 요청 변환.
//! 변환은 순수 함수로 분리해 네트워크 없이 단위 테스트가 가능하다.

use matching_engine::{
    domain::order::{OrderSize, Side, TimeInForce},
    engine::command::{AmendOrderCommand, EngineCommand},
    grpc::engine::{
        AckStatus, AmendOrderRequest, CancelOrderRequest, Command, CommandAck,
        OrderType as ProtoOrderType, PlaceOrderRequest, Side as ProtoSide, SubmitBatchRequest,
        TimeInForce as ProtoTimeInForce, command,
        matching_engine_service_client::MatchingEngineServiceClient, place_order_request::Size,
    },
};
use tonic::transport::Channel;
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

/// 독립 gRPC 연결 `count`개로 구성된 풀을 만든다.
///
/// tonic `Channel`은 clone해도 동일한 HTTP/2 연결을 공유하므로, 워커를 늘려도 단일 연결의
/// 동시 스트림 한도에 막힌다. 부하 생성 처리량을 높이려면 `connect()`를 여러 번 호출해
/// **물리적으로 분리된 연결**을 확보해야 한다.
pub async fn connect_pool(
    endpoint: &str,
    count: usize,
) -> Result<Vec<GrpcClient>, tonic::transport::Error> {
    let mut pool = Vec::with_capacity(count);
    for _ in 0..count {
        pool.push(connect(endpoint).await?);
    }
    Ok(pool)
}

/// 한 워크로드 unit의 명령들을 batch RPC 1콜로 전송하고, 입력 순서대로 결과를 분류한다.
/// 콜당 고정비를 분산해 서버 gRPC ingress CPU를 줄이는 것이 목적이다.
pub async fn send_batch(
    client: &mut GrpcClient,
    symbol: &str,
    commands: &[EngineCommand],
) -> Vec<SendOutcome> {
    let request = SubmitBatchRequest {
        commands: commands
            .iter()
            .map(|command| to_proto_command(symbol, command))
            .collect(),
    };

    match client.submit_batch(request).await {
        Ok(response) => {
            let mut outcomes: Vec<SendOutcome> = response
                .into_inner()
                .acks
                .iter()
                .map(classify_ack)
                .collect();
            // 응답 결과 수가 요청보다 적으면 부족분은 에러로 본다.
            if outcomes.len() < commands.len() {
                outcomes.resize(commands.len(), SendOutcome::Error);
            }
            outcomes
        }
        Err(_) => vec![SendOutcome::Error; commands.len()],
    }
}

pub fn classify_ack(ack: &CommandAck) -> SendOutcome {
    if ack.status == AckStatus::Accepted as i32 {
        SendOutcome::Submitted
    } else if ack.status == AckStatus::ResourceExhausted as i32 {
        SendOutcome::ResourceExhausted
    } else {
        SendOutcome::Rejected
    }
}

pub fn to_proto_command(symbol: &str, command: &EngineCommand) -> Command {
    let inner = match command {
        EngineCommand::Place(command) => command::Command::Place(place_request(command)),
        EngineCommand::Cancel(command) => {
            command::Command::Cancel(cancel_request(command.command_id, command.order_id, symbol))
        }
        EngineCommand::Amend(cmd) => command::Command::Amend(amend_request(cmd, symbol)),
    };
    Command {
        command: Some(inner),
    }
}

pub fn place_request(
    command: &matching_engine::engine::command::PlaceOrderCommand,
) -> PlaceOrderRequest {
    let order = &command.order;
    let size = match order.size {
        OrderSize::Base(qty) => Size::BaseQty(qty.to_string()),
        OrderSize::Quote(quote) => Size::QuoteQty(quote.to_string()),
    };

    PlaceOrderRequest {
        command_id: command.command_id.to_string(),
        order_id: order.order_id.to_string(),
        symbol: order.symbol.clone(),
        side: side_to_proto(order.side),
        order_type: order_type_to_proto(order.order_type),
        tif: tif_to_proto(order.tif),
        price: order.price.map(|p| p.to_string()),
        size: Some(size),
    }
}

pub fn cancel_request(command_id: Uuid, order_id: Uuid, symbol: &str) -> CancelOrderRequest {
    CancelOrderRequest {
        command_id: command_id.to_string(),
        order_id: order_id.to_string(),
        symbol: symbol.to_string(),
    }
}

pub fn amend_request(cmd: &AmendOrderCommand, symbol: &str) -> AmendOrderRequest {
    AmendOrderRequest {
        command_id: cmd.command_id.to_string(),
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

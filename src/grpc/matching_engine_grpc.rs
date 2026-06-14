use tonic::{Request, Response, Status};

use crate::{
    engine::{
        command::EngineCommand,
        dispatcher::{DispatchError, EngineDispatcher},
    },
    grpc::{
        engine::{
            AmendOrderRequest, AmendOrderResponse, CancelOrderRequest, CancelOrderResponse,
            Command, CommandResult, PlaceOrderRequest, PlaceOrderResponse, SubmitBatchRequest,
            SubmitBatchResponse, SubmitStatus, command,
            matching_engine_service_server::MatchingEngineService,
        },
        mapper::{map_amend_order_request, map_cancel_order_request, map_place_order_request},
    },
};

pub struct MatchingEngineGrpcService {
    dispatcher: EngineDispatcher,
}

impl MatchingEngineGrpcService {
    pub fn new(dispatcher: EngineDispatcher) -> Self {
        Self { dispatcher }
    }
}

#[tonic::async_trait]
impl MatchingEngineService for MatchingEngineGrpcService {
    async fn place_order(
        &self,
        request: Request<PlaceOrderRequest>,
    ) -> Result<Response<PlaceOrderResponse>, Status> {
        let req = request.into_inner();

        let order = map_place_order_request(req)?;

        let order_id = order.order_id.to_string();
        let symbol = order.symbol.clone();
        self.dispatcher
            .dispatch(&symbol, EngineCommand::Place(order))
            .map_err(map_dispatch_error)?;

        Ok(Response::new(PlaceOrderResponse {
            order_id,
            status: SubmitStatus::Submitted as i32,
            message: "주문 접수 성공".to_string(),
        }))
    }

    async fn cancel_order(
        &self,
        request: Request<CancelOrderRequest>,
    ) -> Result<Response<CancelOrderResponse>, Status> {
        let req = request.into_inner();

        let symbol = req.symbol.clone();
        let cmd = map_cancel_order_request(req)?;

        let order_id = cmd.order_id().to_string();
        self.dispatcher
            .dispatch(&symbol, cmd)
            .map_err(map_dispatch_error)?;

        Ok(Response::new(CancelOrderResponse {
            order_id,
            status: SubmitStatus::Submitted as i32,
            message: "주문 취소 접수 성공".to_string(),
        }))
    }

    async fn amend_order(
        &self,
        request: Request<AmendOrderRequest>,
    ) -> Result<Response<AmendOrderResponse>, Status> {
        let req = request.into_inner();

        let symbol = req.symbol.clone();
        let cmd = map_amend_order_request(req)?;

        let order_id = cmd.order_id().to_string();
        self.dispatcher
            .dispatch(&symbol, cmd)
            .map_err(map_dispatch_error)?;

        Ok(Response::new(AmendOrderResponse {
            order_id,
            status: SubmitStatus::Submitted as i32,
            message: "주문 정정 접수 성공".to_string(),
        }))
    }

    async fn submit_batch(
        &self,
        request: Request<SubmitBatchRequest>,
    ) -> Result<Response<SubmitBatchResponse>, Status> {
        let commands = request.into_inner().commands;

        // commands 순서대로 dispatch한다(unit 내부 place→cancel/amend 의존성 보존).
        // 명령 하나의 실패가 배치 전체를 실패시키지 않도록 per-command 결과로 수집한다.
        let results = commands
            .into_iter()
            .map(|command| self.dispatch_one(command))
            .collect();

        Ok(Response::new(SubmitBatchResponse { results }))
    }
}

impl MatchingEngineGrpcService {
    fn dispatch_one(&self, command: Command) -> CommandResult {
        let Some(command) = command.command else {
            return rejected_result(String::new(), "빈 명령입니다");
        };

        match command {
            command::Command::Place(req) => {
                let order_id = req.order_id.clone();
                match map_place_order_request(req) {
                    Ok(order) => {
                        let symbol = order.symbol.clone();
                        self.result_of(order_id, &symbol, EngineCommand::Place(order))
                    }
                    Err(status) => rejected_result(order_id, status.message()),
                }
            }
            command::Command::Cancel(req) => {
                let order_id = req.order_id.clone();
                let symbol = req.symbol.clone();
                match map_cancel_order_request(req) {
                    Ok(cmd) => self.result_of(order_id, &symbol, cmd),
                    Err(status) => rejected_result(order_id, status.message()),
                }
            }
            command::Command::Amend(req) => {
                let order_id = req.order_id.clone();
                let symbol = req.symbol.clone();
                match map_amend_order_request(req) {
                    Ok(cmd) => self.result_of(order_id, &symbol, cmd),
                    Err(status) => rejected_result(order_id, status.message()),
                }
            }
        }
    }

    fn result_of(&self, order_id: String, symbol: &str, cmd: EngineCommand) -> CommandResult {
        match self.dispatcher.dispatch(symbol, cmd) {
            Ok(()) => CommandResult {
                order_id,
                status: SubmitStatus::Submitted as i32,
                message: "접수 성공".to_string(),
            },
            Err(DispatchError::ChannelFull { .. }) => CommandResult {
                order_id,
                status: SubmitStatus::ResourceExhausted as i32,
                message: "엔진 채널 포화".to_string(),
            },
            Err(e) => rejected_result(order_id, &format!("{e:?}")),
        }
    }
}

fn rejected_result(order_id: String, message: &str) -> CommandResult {
    CommandResult {
        order_id,
        status: SubmitStatus::Rejected as i32,
        message: message.to_string(),
    }
}

fn map_dispatch_error(error: DispatchError) -> Status {
    match error {
        DispatchError::UnknownSymbol { symbol, order_id } => Status::not_found(format!(
            "알 수 없는 심볼: symbol={symbol}, order_id={order_id}"
        )),
        DispatchError::EngineStopped { symbol, order_id } => Status::unavailable(format!(
            "엔진 중지 상태: symbol={symbol}, order_id={order_id}"
        )),
        DispatchError::ChannelFull { symbol, order_id } => Status::resource_exhausted(format!(
            "엔진 포화 상태: symbol={symbol}, order_id={order_id}"
        )),
        DispatchError::PublisherUnhealthy { symbol, order_id } => Status::unavailable(format!(
            "결과 발행기 비정상 상태: symbol={symbol}, order_id={order_id}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crossbeam::channel::Receiver;
    use uuid::Uuid;

    use super::*;
    use crate::grpc::engine::{
        Side, TimeInForce, command::Command as CommandKind, place_order_request,
    };

    const SYMBOL: &str = "BTCUSDT";

    fn service_with_receiver(
        capacity: usize,
    ) -> (MatchingEngineGrpcService, Receiver<EngineCommand>) {
        let (tx, rx) = crossbeam::channel::bounded(capacity);
        let mut senders = HashMap::new();
        senders.insert(SYMBOL.to_string(), tx);
        (
            MatchingEngineGrpcService::new(EngineDispatcher::new(senders)),
            rx,
        )
    }

    fn place(order_id: Uuid) -> Command {
        Command {
            command: Some(CommandKind::Place(PlaceOrderRequest {
                order_id: order_id.to_string(),
                symbol: SYMBOL.to_string(),
                side: Side::Buy as i32,
                order_type: crate::grpc::engine::OrderType::Limit as i32,
                tif: TimeInForce::Gtc as i32,
                price: Some("100".to_string()),
                size: Some(place_order_request::Size::BaseQty("1".to_string())),
            })),
        }
    }

    fn cancel(order_id: Uuid) -> Command {
        Command {
            command: Some(CommandKind::Cancel(CancelOrderRequest {
                order_id: order_id.to_string(),
                symbol: SYMBOL.to_string(),
            })),
        }
    }

    #[tokio::test]
    async fn 배치는_순서대로_dispatch되고_per_command_결과를_반환한다() {
        let (service, rx) = service_with_receiver(8);
        let p = Uuid::from_u128(1);
        let c = Uuid::from_u128(1); // place 후 같은 order_id cancel

        let response = service
            .submit_batch(Request::new(SubmitBatchRequest {
                commands: vec![place(p), cancel(c)],
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.results.len(), 2);
        assert!(
            response
                .results
                .iter()
                .all(|r| r.status == SubmitStatus::Submitted as i32)
        );

        // place → cancel 순서로 엔진 채널에 적재된다.
        assert!(matches!(rx.try_recv().unwrap(), EngineCommand::Place(_)));
        assert!(matches!(rx.try_recv().unwrap(), EngineCommand::Cancel(_)));
    }

    #[tokio::test]
    async fn 채널_포화시_해당_명령만_resource_exhausted() {
        let (service, _rx) = service_with_receiver(1); // 1개만 수용

        let response = service
            .submit_batch(Request::new(SubmitBatchRequest {
                commands: vec![place(Uuid::from_u128(1)), place(Uuid::from_u128(2))],
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.results[0].status, SubmitStatus::Submitted as i32);
        assert_eq!(
            response.results[1].status,
            SubmitStatus::ResourceExhausted as i32
        );
    }

    #[tokio::test]
    async fn 빈_oneof_명령은_rejected() {
        let (service, _rx) = service_with_receiver(8);

        let response = service
            .submit_batch(Request::new(SubmitBatchRequest {
                commands: vec![Command { command: None }],
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.results[0].status, SubmitStatus::Rejected as i32);
    }
}

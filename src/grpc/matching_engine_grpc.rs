use tonic::{Request, Response, Status};

use crate::{
    engine::{command::EngineCommand, dispatcher::EngineDispatcher},
    grpc::{
        ack::{accepted_ack, command_parts, dispatch_error_ack, rejected_ack},
        engine::{
            AckReason, AmendOrderRequest, CancelOrderRequest, Command, CommandAck,
            PlaceOrderRequest, SubmitBatchRequest, SubmitBatchResponse, command,
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
    ) -> Result<Response<CommandAck>, Status> {
        let ack = self.dispatch_one(Command {
            command: Some(command::Command::Place(request.into_inner())),
        });
        Ok(Response::new(ack))
    }

    async fn cancel_order(
        &self,
        request: Request<CancelOrderRequest>,
    ) -> Result<Response<CommandAck>, Status> {
        let ack = self.dispatch_one(Command {
            command: Some(command::Command::Cancel(request.into_inner())),
        });
        Ok(Response::new(ack))
    }

    async fn amend_order(
        &self,
        request: Request<AmendOrderRequest>,
    ) -> Result<Response<CommandAck>, Status> {
        let ack = self.dispatch_one(Command {
            command: Some(command::Command::Amend(request.into_inner())),
        });
        Ok(Response::new(ack))
    }

    async fn submit_batch(
        &self,
        request: Request<SubmitBatchRequest>,
    ) -> Result<Response<SubmitBatchResponse>, Status> {
        let commands = request.into_inner().commands;

        // commands 순서대로 dispatch한다(unit 내부 place→cancel/amend 의존성 보존).
        // 명령 하나의 실패가 배치 전체를 실패시키지 않도록 per-command 결과로 수집한다.
        let acks = commands
            .into_iter()
            .map(|command| self.dispatch_one(command))
            .collect();

        Ok(Response::new(SubmitBatchResponse { acks }))
    }
}

impl MatchingEngineGrpcService {
    fn dispatch_one(&self, command: Command) -> CommandAck {
        let Some(command) = command.command else {
            return rejected_ack(
                String::new(),
                String::new(),
                String::new(),
                AckReason::InvalidArgument,
                "빈 명령입니다",
            );
        };

        let parts = command_parts(&command);
        if parts.command_id.is_empty() {
            return rejected_ack(
                parts.command_id,
                parts.order_id,
                parts.symbol,
                AckReason::InvalidArgument,
                "command_id는 필수입니다",
            );
        }

        let mapped = match command {
            command::Command::Place(req) => map_place_order_request(req).map(EngineCommand::Place),
            command::Command::Cancel(req) => map_cancel_order_request(req),
            command::Command::Amend(req) => map_amend_order_request(req),
        };

        match mapped {
            Ok(cmd) => self.result_of(parts.command_id, &parts.symbol, cmd),
            Err(status) => rejected_ack(
                parts.command_id,
                parts.order_id,
                parts.symbol,
                AckReason::InvalidArgument,
                status.message(),
            ),
        }
    }

    fn result_of(&self, command_id: String, symbol: &str, cmd: EngineCommand) -> CommandAck {
        let order_id = cmd.order_id().to_string();
        match self.dispatcher.dispatch(symbol, cmd) {
            Ok(()) => accepted_ack(command_id, order_id, symbol),
            Err(error) => dispatch_error_ack(command_id, order_id, symbol.to_string(), error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crossbeam::channel::Receiver;
    use uuid::Uuid;

    use super::*;
    use crate::{
        engine::result_handler::EngineHealth,
        grpc::engine::{
            AckStatus, Side, TimeInForce, command::Command as CommandKind, place_order_request,
        },
    };

    const SYMBOL: &str = "BTCUSDT";

    fn service_with_receiver(
        capacity: usize,
    ) -> (MatchingEngineGrpcService, Receiver<EngineCommand>) {
        let (tx, rx) = crossbeam::channel::bounded(capacity);
        let mut senders = HashMap::new();
        senders.insert(SYMBOL.to_string(), tx);
        (
            MatchingEngineGrpcService::new(EngineDispatcher::new(senders, EngineHealth::default())),
            rx,
        )
    }

    fn place(command_id: Uuid, order_id: Uuid) -> Command {
        Command {
            command: Some(CommandKind::Place(PlaceOrderRequest {
                command_id: command_id.to_string(),
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

    fn cancel(command_id: Uuid, order_id: Uuid) -> Command {
        Command {
            command: Some(CommandKind::Cancel(CancelOrderRequest {
                command_id: command_id.to_string(),
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
                commands: vec![
                    place(Uuid::from_u128(11), p),
                    cancel(Uuid::from_u128(12), c),
                ],
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.acks.len(), 2);
        assert!(
            response
                .acks
                .iter()
                .all(|r| r.status == AckStatus::Accepted as i32)
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
                commands: vec![
                    place(Uuid::from_u128(21), Uuid::from_u128(1)),
                    place(Uuid::from_u128(22), Uuid::from_u128(2)),
                ],
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.acks[0].status, AckStatus::Accepted as i32);
        assert_eq!(response.acks[1].status, AckStatus::ResourceExhausted as i32);
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

        assert_eq!(response.acks[0].status, AckStatus::Rejected as i32);
    }
}

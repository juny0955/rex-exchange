use tonic::{Request, Response, Status};

use crate::{
    engine::{
        command::EngineCommand,
        dispatcher::{DispatchError, EngineDispatcher},
    },
    grpc::{
        engine::{
            AmendOrderRequest, AmendOrderResponse, CancelOrderRequest, CancelOrderResponse,
            PlaceOrderRequest, PlaceOrderResponse, SubmitStatus,
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
    }
}

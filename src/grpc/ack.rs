use chrono::Utc;

use crate::{
    engine::dispatcher::DispatchError,
    grpc::engine::{AckReason, AckStatus, CommandAck, command},
};

#[derive(Debug)]
pub struct CommandParts {
    pub command_id: String,
    pub order_id: String,
    pub symbol: String,
}

pub fn command_parts(command: &command::Command) -> CommandParts {
    match command {
        command::Command::Place(req) => CommandParts {
            command_id: req.command_id.clone(),
            order_id: req.order_id.clone(),
            symbol: req.symbol.clone(),
        },
        command::Command::Cancel(req) => CommandParts {
            command_id: req.command_id.clone(),
            order_id: req.order_id.clone(),
            symbol: req.symbol.clone(),
        },
        command::Command::Amend(req) => CommandParts {
            command_id: req.command_id.clone(),
            order_id: req.order_id.clone(),
            symbol: req.symbol.clone(),
        },
    }
}

pub fn accepted_ack(command_id: String, order_id: String, symbol: &str) -> CommandAck {
    CommandAck {
        command_id,
        order_id,
        symbol: symbol.to_string(),
        status: AckStatus::Accepted as i32,
        reason: AckReason::Accepted as i32,
        retryable: false,
        accepted_at: Utc::now().to_rfc3339(),
        detail: "접수 성공".to_string(),
    }
}

pub fn rejected_ack(
    command_id: String,
    order_id: String,
    symbol: String,
    reason: AckReason,
    detail: &str,
) -> CommandAck {
    CommandAck {
        command_id,
        order_id,
        symbol,
        status: AckStatus::Rejected as i32,
        reason: reason as i32,
        retryable: false,
        accepted_at: String::new(),
        detail: detail.to_string(),
    }
}

pub fn dispatch_error_ack(
    command_id: String,
    order_id: String,
    symbol: String,
    error: DispatchError,
) -> CommandAck {
    match error {
        DispatchError::UnknownSymbol { .. } => rejected_ack(
            command_id,
            order_id,
            symbol,
            AckReason::UnknownSymbol,
            "알 수 없는 심볼입니다",
        ),
        DispatchError::EngineStopped { .. } => unavailable_ack(
            command_id,
            order_id,
            symbol,
            AckReason::EngineStopped,
            "엔진 중지 상태입니다",
        ),
        DispatchError::ChannelFull { .. } => CommandAck {
            command_id,
            order_id,
            symbol,
            status: AckStatus::ResourceExhausted as i32,
            reason: AckReason::EngineChannelFull as i32,
            retryable: true,
            accepted_at: String::new(),
            detail: "엔진 채널 포화".to_string(),
        },
        DispatchError::PublisherUnhealthy { .. } => unavailable_ack(
            command_id,
            order_id,
            symbol,
            AckReason::PublisherUnhealthy,
            "결과 발행기 비정상 상태입니다",
        ),
    }
}

fn unavailable_ack(
    command_id: String,
    order_id: String,
    symbol: String,
    reason: AckReason,
    detail: &str,
) -> CommandAck {
    CommandAck {
        command_id,
        order_id,
        symbol,
        status: AckStatus::Unavailable as i32,
        reason: reason as i32,
        retryable: true,
        accepted_at: String::new(),
        detail: detail.to_string(),
    }
}

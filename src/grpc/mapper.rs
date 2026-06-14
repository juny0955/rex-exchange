use chrono::Utc;
use rust_decimal::Decimal;
use tonic::Status;
use uuid::Uuid;

use crate::{
    domain::order::{self, Order},
    engine::command::{AmendOrderCommand, CancelOrderCommand, EngineCommand, PlaceOrderCommand},
    grpc::engine::{
        AmendOrderRequest, CancelOrderRequest, OrderType, PlaceOrderRequest, Side, TimeInForce,
        place_order_request::Size,
    },
};

pub fn map_place_order_request(request: PlaceOrderRequest) -> Result<PlaceOrderCommand, Status> {
    let command_id = parse_uuid(&request.command_id, "command_id")?;
    let order_id = Uuid::parse_str(&request.order_id)
        .map_err(|_| Status::invalid_argument("order_id 형식이 올바르지 않습니다"))?;

    if request.symbol.is_empty() {
        return Err(Status::invalid_argument("symbol은 필수입니다"));
    }

    let side = map_side(request.side)?;
    let order_type = map_order_type(request.order_type)?;
    let tif = map_tif(request.tif)?;

    if matches!(tif, order::TimeInForce::GTC) && matches!(order_type, order::OrderType::Market) {
        return Err(Status::invalid_argument(
            "MARKET 주문은 GTC를 사용할 수 없습니다",
        ));
    }

    let price = match request.price {
        Some(raw) if !raw.trim().is_empty() => Some(parse_decimal(&raw, "price")?),
        _ => None,
    };

    if matches!(order_type, order::OrderType::Limit) && price.is_none() {
        return Err(Status::invalid_argument("LIMIT 주문은 price가 필수입니다"));
    }

    if matches!(order_type, order::OrderType::Market) && price.is_some() {
        return Err(Status::invalid_argument(
            "MARKET 주문은 price를 지정할 수 없습니다",
        ));
    }

    let size = match request.size {
        Some(Size::BaseQty(raw)) => order::OrderSize::Base(parse_decimal(&raw, "base_qty")?),
        Some(Size::QuoteQty(raw)) => order::OrderSize::Quote(parse_decimal(&raw, "quote_qty")?),
        None => return Err(Status::invalid_argument("주문 수량은 필수입니다")),
    };

    if matches!(size, order::OrderSize::Quote(_))
        && (!matches!(order_type, order::OrderType::Market) || !matches!(side, order::Side::Buy))
    {
        return Err(Status::invalid_argument(
            "Quote는 MARKET BUY 주문에만 사용할 수 있습니다",
        ));
    }

    Ok(PlaceOrderCommand {
        command_id,
        order: Order {
            order_id,
            symbol: request.symbol,
            side,
            order_type,
            tif,
            price,
            size,
            executed_base_qty: Decimal::ZERO,
            executed_quote_qty: Decimal::ZERO,
            status: order::OrderStatus::New,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    })
}

pub fn map_cancel_order_request(request: CancelOrderRequest) -> Result<EngineCommand, Status> {
    let command_id = parse_uuid(&request.command_id, "command_id")?;
    let order_id = parse_uuid(&request.order_id, "order_id")?;

    if request.symbol.is_empty() {
        return Err(Status::invalid_argument("symbol은 필수입니다"));
    }

    Ok(EngineCommand::Cancel(CancelOrderCommand {
        command_id,
        order_id,
    }))
}

pub fn map_amend_order_request(request: AmendOrderRequest) -> Result<EngineCommand, Status> {
    let command_id = parse_uuid(&request.command_id, "command_id")?;
    let order_id = parse_uuid(&request.order_id, "order_id")?;

    if request.symbol.is_empty() {
        return Err(Status::invalid_argument("symbol은 필수입니다"));
    }

    let price = match request.new_price {
        Some(raw) if !raw.trim().is_empty() => Some(parse_decimal(&raw, "new_price")?),
        _ => None,
    };

    let base_qty = match request.new_base_qty {
        Some(raw) if !raw.trim().is_empty() => Some(parse_decimal(&raw, "new_base_qty")?),
        _ => None,
    };

    if price.is_none() && base_qty.is_none() {
        return Err(Status::invalid_argument(
            "new_price 또는 new_base_qty 중 하나는 필수입니다",
        ));
    }

    Ok(EngineCommand::Amend(AmendOrderCommand {
        command_id,
        order_id,
        price,
        base_qty,
    }))
}

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(raw)
        .map_err(|_| Status::invalid_argument(format!("{field} 형식이 올바르지 않습니다")))
}

fn map_side(value: i32) -> Result<order::Side, Status> {
    let side = Side::try_from(value)
        .map_err(|_| Status::invalid_argument("side 값이 올바르지 않습니다"))?;

    match side {
        Side::Buy => Ok(order::Side::Buy),
        Side::Sell => Ok(order::Side::Sell),
        Side::Unspecified => Err(Status::invalid_argument("side는 필수입니다")),
    }
}

fn map_order_type(value: i32) -> Result<order::OrderType, Status> {
    let order_type = OrderType::try_from(value)
        .map_err(|_| Status::invalid_argument("order_type 값이 올바르지 않습니다"))?;

    match order_type {
        OrderType::Limit => Ok(order::OrderType::Limit),
        OrderType::Market => Ok(order::OrderType::Market),
        OrderType::Unspecified => Err(Status::invalid_argument("order_type은 필수입니다")),
    }
}

fn map_tif(value: i32) -> Result<order::TimeInForce, Status> {
    let tif = TimeInForce::try_from(value)
        .map_err(|_| Status::invalid_argument("tif 값이 올바르지 않습니다"))?;

    match tif {
        TimeInForce::Gtc => Ok(order::TimeInForce::GTC),
        TimeInForce::Ioc => Ok(order::TimeInForce::IOC),
        TimeInForce::Fok => Ok(order::TimeInForce::FOK),
        TimeInForce::Unspecified => Err(Status::invalid_argument("tif는 필수입니다")),
    }
}

fn parse_decimal(raw: &str, field: &str) -> Result<Decimal, Status> {
    let value = raw
        .parse::<Decimal>()
        .map_err(|_| Status::invalid_argument(format!("{field} 형식이 올바르지 않습니다")))?;

    if value <= Decimal::ZERO {
        return Err(Status::invalid_argument(format!(
            "{field}는 0보다 커야 합니다"
        )));
    }

    Ok(value)
}

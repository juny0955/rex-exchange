use chrono::Utc;
use matching_engine::{
    domain::order::{Order, OrderSize, OrderStatus, OrderType, Side, TimeInForce},
    engine::command::AmendOrderCommand,
    grpc::engine::{
        OrderType as ProtoOrderType, Side as ProtoSide, TimeInForce as ProtoTimeInForce,
        place_order_request::Size,
    },
};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::integration_stress::client::{amend_request, cancel_request, place_request};

fn limit_buy(order_id: Uuid) -> Order {
    Order {
        order_id,
        symbol: "BTCUSDT".into(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        tif: TimeInForce::GTC,
        price: Some(Decimal::new(10_000, 0)),
        size: OrderSize::Base(Decimal::new(2, 0)),
        executed_base_qty: Decimal::ZERO,
        executed_quote_qty: Decimal::ZERO,
        status: OrderStatus::New,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn 지정가_매수_요청_변환() {
    let order_id = Uuid::from_u128(7);
    let req = place_request(&limit_buy(order_id));

    assert_eq!(req.order_id, order_id.to_string());
    assert_eq!(req.symbol, "BTCUSDT");
    assert_eq!(req.side, ProtoSide::Buy as i32);
    assert_eq!(req.order_type, ProtoOrderType::Limit as i32);
    assert_eq!(req.tif, ProtoTimeInForce::Gtc as i32);
    assert_eq!(req.price.as_deref(), Some("10000"));
    assert_eq!(req.size, Some(Size::BaseQty("2".into())));
}

#[test]
fn 시장가_금액_매수_요청은_quote_qty() {
    let mut order = limit_buy(Uuid::from_u128(8));
    order.order_type = OrderType::Market;
    order.tif = TimeInForce::IOC;
    order.price = None;
    order.size = OrderSize::Quote(Decimal::new(303, 0));

    let req = place_request(&order);

    assert_eq!(req.order_type, ProtoOrderType::Market as i32);
    assert_eq!(req.tif, ProtoTimeInForce::Ioc as i32);
    assert_eq!(req.price, None);
    assert_eq!(req.size, Some(Size::QuoteQty("303".into())));
}

#[test]
fn 취소_요청_변환() {
    let order_id = Uuid::from_u128(9);
    let req = cancel_request(order_id, "ETHUSDT");

    assert_eq!(req.order_id, order_id.to_string());
    assert_eq!(req.symbol, "ETHUSDT");
}

#[test]
fn 정정_요청_변환() {
    let order_id = Uuid::from_u128(10);
    let cmd = AmendOrderCommand {
        order_id,
        price: Some(Decimal::new(10_001, 0)),
        base_qty: Some(Decimal::new(1, 0)),
    };

    let req = amend_request(&cmd, "BTCUSDT");

    assert_eq!(req.order_id, order_id.to_string());
    assert_eq!(req.symbol, "BTCUSDT");
    assert_eq!(req.new_price.as_deref(), Some("10001"));
    assert_eq!(req.new_base_qty.as_deref(), Some("1"));
}

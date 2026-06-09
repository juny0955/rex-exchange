use super::*;
use chrono::Utc;
use crossbeam::channel;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::order::{Order, OrderSize, OrderStatus, OrderType, Side, TimeInForce};
use crate::engine::command::AmendOrderCommand;
use crate::engine::result::{
    AmendOrderOutcome, AmendRejectedReason, CancelOrderOutcome, CancelRejectedReason, EngineResult,
};

fn make_engine() -> MatchingEngine {
    let (_, engine_rx) = channel::unbounded();
    let (result_tx, _) = channel::unbounded();
    let symbol = "BTCUSDT".to_string();
    MatchingEngine::new(symbol, engine_rx, result_tx)
}

fn limit_order(side: Side, tif: TimeInForce, price: i64, qty: i64) -> Order {
    Order {
        order_id: Uuid::now_v7(),
        symbol: "BTCUSDT".to_string(),
        side,
        order_type: OrderType::Limit,
        tif,
        price: Some(Decimal::new(price, 0)),
        size: OrderSize::Base(Decimal::new(qty, 0)),
        executed_base_qty: Decimal::ZERO,
        executed_quote_qty: Decimal::ZERO,
        status: OrderStatus::New,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn market_quote_order(side: Side, quote: i64) -> Order {
    Order {
        order_id: Uuid::now_v7(),
        symbol: "BTCUSDT".to_string(),
        side,
        order_type: OrderType::Market,
        tif: TimeInForce::IOC,
        price: None,
        size: OrderSize::Quote(Decimal::new(quote, 0)),
        executed_base_qty: Decimal::ZERO,
        executed_quote_qty: Decimal::ZERO,
        status: OrderStatus::New,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn 매수_매도_완전_체결_테스트() {
    let mut engine = make_engine();
    engine
        .orderbook
        .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 10));

    engine.place_order(limit_order(Side::Buy, TimeInForce::GTC, 100, 10));

    assert!(engine.orderbook.best_opposite_price(&Side::Buy).is_none());
}

#[test]
fn fok_잔량_부족시_취소_테스트() {
    let mut engine = make_engine();
    engine
        .orderbook
        .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 5));

    // FOK Buy 10, 잔량 5만 존재 → 취소
    engine.place_order(limit_order(Side::Buy, TimeInForce::FOK, 100, 10));

    // SELL 여전히 원래 수량 그대로
    assert!(engine.orderbook.can_fully_fill_base(
        Side::Buy,
        Decimal::new(5, 0),
        Decimal::new(100, 0)
    ));
    assert!(!engine.orderbook.can_fully_fill_base(
        Side::Buy,
        Decimal::new(6, 0),
        Decimal::new(100, 0)
    ));
}

#[test]
fn 시장가_quote_매수_체결_테스트() {
    let mut engine = make_engine();
    // SELL 10 BTC @ 100 = 총 1000 USDT
    engine
        .orderbook
        .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 10));

    // MARKET BUY 400 USDT → 4 BTC 체결
    engine.place_order(market_quote_order(Side::Buy, 400));

    // 잔여 6 BTC @ 100 = 600 USDT
    assert!(
        engine
            .orderbook
            .can_fully_fill_quote(Side::Buy, Decimal::new(500, 0))
    );
    assert!(
        !engine
            .orderbook
            .can_fully_fill_quote(Side::Buy, Decimal::new(700, 0))
    );
}

#[test]
fn gtc_잔존_후_체결_테스트() {
    let mut engine = make_engine();

    // 매도 없음 → BUY GTC 잔존
    engine.place_order(limit_order(Side::Buy, TimeInForce::GTC, 100, 5));
    assert!(engine.orderbook.best_opposite_price(&Side::Sell).is_some());

    // SELL 진입 → 잔존 BUY와 체결
    engine.place_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 5));
    assert!(engine.orderbook.best_opposite_price(&Side::Sell).is_none());
}

#[test]
fn ioc_잔존_주문_미등록_테스트() {
    let mut engine = make_engine();
    engine
        .orderbook
        .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 5));

    // IOC Buy 10, Sell 5만 존재 → 5 체결, 잔량 5는 오더북 미등록
    engine.place_order(limit_order(Side::Buy, TimeInForce::IOC, 100, 10));

    assert!(engine.orderbook.best_opposite_price(&Side::Sell).is_none());
}

#[test]
fn 시장가_매도_quote_거부_테스트() {
    let mut engine = make_engine();
    engine
        .orderbook
        .add_order(limit_order(Side::Buy, TimeInForce::GTC, 100, 10));

    // MARKET SELL + Quote → 거부
    engine.place_order(market_quote_order(Side::Sell, 500));

    assert!(engine.orderbook.best_opposite_price(&Side::Sell).is_some());
}

#[test]
fn 주문_취소_성공_테스트() {
    let mut engine = make_engine();
    let order = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
    let order_id = order.order_id;
    engine.orderbook.add_order(order);

    let result = engine.cancel_order(order_id);

    let EngineResult::Cancel(r) = result else {
        panic!("Cancel 결과여야 함");
    };
    let CancelOrderOutcome::Cancelled(snapshot) = r.outcome else {
        panic!("취소 성공이어야 함");
    };
    assert_eq!(snapshot.order_id, order_id);
    assert_eq!(snapshot.status, OrderStatus::Cancelled);
    assert_eq!(snapshot.executed_base_qty, Decimal::ZERO);
    assert_eq!(snapshot.remaining_base_qty, Some(Decimal::new(10, 0)));
    assert!(engine.orderbook.best_opposite_price(&Side::Sell).is_none());
}

#[test]
fn 부분_체결_후_취소_테스트() {
    let mut engine = make_engine();
    engine
        .orderbook
        .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 5));

    let buy = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
    let buy_id = buy.order_id;
    engine.place_order(buy);

    let result = engine.cancel_order(buy_id);

    let EngineResult::Cancel(r) = result else {
        panic!("Cancel 결과여야 함");
    };
    let CancelOrderOutcome::Cancelled(snapshot) = r.outcome else {
        panic!("취소 성공이어야 함");
    };
    assert_eq!(snapshot.status, OrderStatus::Cancelled);
    assert_eq!(snapshot.executed_base_qty, Decimal::new(5, 0));
    assert_eq!(snapshot.remaining_base_qty, Some(Decimal::new(5, 0)));
    assert!(engine.orderbook.best_opposite_price(&Side::Sell).is_none());
}

#[test]
fn 존재하지_않는_주문_취소_테스트() {
    let mut engine = make_engine();

    let result = engine.cancel_order(Uuid::now_v7());

    let EngineResult::Cancel(r) = result else {
        panic!("Cancel 결과여야 함");
    };
    assert!(matches!(
        r.outcome,
        CancelOrderOutcome::Rejected(CancelRejectedReason::OrderNotFound)
    ));
}

#[test]
fn 존재하지_않는_주문_정정_테스트() {
    let mut engine = make_engine();

    let result = engine.amend_order(AmendOrderCommand {
        order_id: Uuid::now_v7(),
        price: Some(Decimal::new(100, 0)),
        base_qty: Some(Decimal::new(5, 0)),
    });

    let EngineResult::Amend(r) = result else {
        panic!("Amend 결과여야 함");
    };
    assert!(matches!(
        r.outcome,
        AmendOrderOutcome::Rejected(AmendRejectedReason::OrderNotFound)
    ));
}

#[test]
fn 정정_불가_주문_정정_테스트() {
    let mut engine = make_engine();
    let order = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
    let order_id = order.order_id;
    engine.orderbook.add_order(order);

    // price, qty 모두 None → 변경 없음 → AmendNotAllowed
    let result = engine.amend_order(AmendOrderCommand {
        order_id,
        price: None,
        base_qty: None,
    });

    let EngineResult::Amend(r) = result else {
        panic!("Amend 결과여야 함");
    };
    assert!(matches!(
        r.outcome,
        AmendOrderOutcome::Rejected(AmendRejectedReason::AmendNotAllowed)
    ));
}

#[test]
fn 수량_감소_정정_테스트() {
    let mut engine = make_engine();
    let order = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
    let order_id = order.order_id;
    engine.orderbook.add_order(order);

    let result = engine.amend_order(AmendOrderCommand {
        order_id,
        price: Some(Decimal::new(100, 0)),
        base_qty: Some(Decimal::new(5, 0)),
    });

    let EngineResult::Amend(r) = result else {
        panic!("Amend 결과여야 함");
    };
    let AmendOrderOutcome::Amended(snapshot) = r.outcome else {
        panic!("Amended여야 함");
    };
    assert_eq!(snapshot.remaining_base_qty, Some(Decimal::new(5, 0)));
    // 인플레이스 정정 → 같은 order_id로 오더북에 남아있어야 함
    assert!(engine.orderbook.get_order_mut(&order_id).is_some());
}

#[test]
fn 정정_후_전량_체결_제거_테스트() {
    let mut engine = make_engine();
    // SELL 5 → BUY 10 부분 체결: executed=5, remaining=5
    engine
        .orderbook
        .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 5));
    let buy = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
    let buy_id = buy.order_id;
    engine.place_order(buy);

    // new_qty=5 == executed_qty=5 → is_filled() → Amended + 오더북 제거
    let result = engine.amend_order(AmendOrderCommand {
        order_id: buy_id,
        price: Some(Decimal::new(100, 0)),
        base_qty: Some(Decimal::new(5, 0)),
    });

    let EngineResult::Amend(r) = result else {
        panic!("Amend 결과여야 함");
    };
    assert!(matches!(r.outcome, AmendOrderOutcome::Amended(_)));
    assert!(engine.orderbook.get_order_mut(&buy_id).is_none());
}

#[test]
fn 가격_변경_정정_테스트() {
    let mut engine = make_engine();
    let order = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
    let order_id = order.order_id;
    engine.orderbook.add_order(order);

    let result = engine.amend_order(AmendOrderCommand {
        order_id,
        price: Some(Decimal::new(101, 0)), // 가격 변경 → 우선순위 소실
        base_qty: Some(Decimal::new(10, 0)),
    });

    let EngineResult::Amend(r) = result else {
        panic!("Amend 결과여야 함");
    };
    assert!(matches!(
        r.outcome,
        AmendOrderOutcome::CancelReplaced { .. }
    ));
}

#[test]
fn 수량_증가_정정_테스트() {
    let mut engine = make_engine();
    let order = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
    let order_id = order.order_id;
    engine.orderbook.add_order(order);

    let result = engine.amend_order(AmendOrderCommand {
        order_id,
        price: Some(Decimal::new(100, 0)),
        base_qty: Some(Decimal::new(15, 0)), // 수량 증가 → 우선순위 소실
    });

    let EngineResult::Amend(r) = result else {
        panic!("Amend 결과여야 함");
    };
    assert!(matches!(
        r.outcome,
        AmendOrderOutcome::CancelReplaced { .. }
    ));
}

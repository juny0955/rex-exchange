//! `MatchingEngine` benchmark fixture.
//!
//! 엔진 benchmark는 명령을 실행할 때마다 내부 book이 변한다. 그래서 각 Criterion 반복마다
//! 새 엔진과 동일한 seeded orderbook을 만들어 setup 비용과 측정 비용을 분리한다.

use matching_engine::{
    domain::order::{Order, OrderSize, OrderStatus, OrderType, Side},
    engine::matching_engine::MatchingEngine,
};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::bench_support::{SYMBOL, fixed_time, price_for};
use matching_engine::domain::order::TimeInForce;

pub(super) const INPUT_SIZES: [usize; 3] = [10, 100, 1_000];
pub(super) const TAKER_ORDER_ID_BASE: u128 = 9_000_000;

const ASK_ORDER_ID_BASE: u128 = 1_000_000;
const SAME_ASK_ORDER_ID_BASE: u128 = 2_000_000;
const SAME_BID_ORDER_ID_BASE: u128 = 3_000_000;

pub(super) use crate::bench_support::{decimal, limit_order};

pub(super) fn make_engine() -> MatchingEngine {
    // 내부 메서드를 직접 호출하므로 channel은 사용하지 않는다. MatchingEngine 생성자가
    // 요구하는 필드만 채워 thread/runtime 비용이 benchmark에 섞이지 않게 한다.
    let (_, engine_rx) = crossbeam::channel::unbounded();
    let (result_tx, _) = crossbeam::channel::unbounded();

    MatchingEngine::new(SYMBOL.to_string(), engine_rx, result_tx)
}

pub(super) fn market_quote_buy(order_id: u128, quote: i64) -> Order {
    // Quote size는 Market Buy에서만 유효하므로 sweep benchmark 전용 주문으로 별도 생성한다.
    Order {
        order_id: Uuid::from_u128(order_id),
        symbol: SYMBOL.to_string(),
        side: Side::Buy,
        order_type: OrderType::Market,
        tif: TimeInForce::IOC,
        price: None,
        size: OrderSize::Quote(decimal(quote)),
        executed_base_qty: Decimal::ZERO,
        executed_quote_qty: Decimal::ZERO,
        status: OrderStatus::New,
        created_at: fixed_time(),
        updated_at: fixed_time(),
    }
}

pub(super) fn seed_asks(engine: &mut MatchingEngine, count: usize) {
    // 여러 ask level을 만들어 market quote buy가 가격 level을 순회하며 체결하도록 한다.
    for index in 0..count {
        engine.bench_seed_order(limit_order(
            Side::Sell,
            ASK_ORDER_ID_BASE + index as u128,
            TimeInForce::GTC,
            price_for(index, 100),
            1,
        ));
    }
}

pub(super) fn seed_same_price_asks(engine: &mut MatchingEngine, count: usize) {
    // 같은 가격 maker N개를 두어 단일 price level 안에서 연속 체결되는 경로를 측정한다.
    for index in 0..count {
        engine.bench_seed_order(limit_order(
            Side::Sell,
            SAME_ASK_ORDER_ID_BASE + index as u128,
            TimeInForce::GTC,
            100,
            1,
        ));
    }
}

pub(super) fn seed_same_price_bids(engine: &mut MatchingEngine, count: usize, qty: i64) {
    // cancel/amend는 resting bid를 대상으로 하므로 같은 가격의 bid queue를 고정 크기로 만든다.
    for index in 0..count {
        engine.bench_seed_order(limit_order(
            Side::Buy,
            SAME_BID_ORDER_ID_BASE + index as u128,
            TimeInForce::GTC,
            100,
            qty,
        ));
    }
}

pub(super) fn middle_bid_order_id(count: usize) -> Uuid {
    // middle 대상은 queue retain/priority 유지 경로에서 front/back 특수성이 섞이지 않게 한다.
    Uuid::from_u128(SAME_BID_ORDER_ID_BASE + (count / 2) as u128)
}

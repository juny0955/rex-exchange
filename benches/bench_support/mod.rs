//! 벤치마크 fixture 전용 공통 생성자.
//!
//! 두 bench target이 같은 형태의 주문을 만들도록 고정된 symbol, timestamp, UUID 입력값을
//! 사용한다. 이렇게 하면 benchmark별 차이가 fixture 생성 방식이 아니라 측정 대상 로직에서
//! 나오도록 맞출 수 있다.

use std::time::SystemTime;

use chrono::{DateTime, Utc};
use matching_engine::domain::order::{Order, OrderSize, OrderStatus, OrderType, Side, TimeInForce};
use rust_decimal::Decimal;
use uuid::Uuid;

pub const SYMBOL: &str = "BTCUSDT";
pub const PRICE_LEVELS: usize = 50;

pub fn fixed_time() -> DateTime<Utc> {
    // Fixture 주문의 시간값은 고정한다. 엔진 내부에서 체결/정정 시 호출하는 Utc::now()
    // 비용은 production path 비용이므로 matching-engine benchmark 안에 그대로 남긴다.
    SystemTime::UNIX_EPOCH.into()
}

pub fn decimal(value: i64) -> Decimal {
    Decimal::new(value, 0)
}

pub fn price_for(index: usize, base_price: i64) -> i64 {
    // 제한된 price level에 주문을 반복 배치해 "주문 수 증가"와 "호가 level 수 증가"가
    // 완전히 같은 의미가 되지 않도록 한다.
    base_price + (index % PRICE_LEVELS) as i64
}

pub fn limit_order(side: Side, order_id: u128, tif: TimeInForce, price: i64, qty: i64) -> Order {
    // Uuid::now_v7() 대신 deterministic UUID를 받아 Criterion 반복 사이 fixture 차이를
    // 줄인다. 실제 엔진이 trade id를 만드는 비용은 matching-engine 측정 경로에 남아 있다.
    Order {
        order_id: Uuid::from_u128(order_id),
        symbol: SYMBOL.to_string(),
        side,
        order_type: OrderType::Limit,
        tif,
        price: Some(decimal(price)),
        size: OrderSize::Base(decimal(qty)),
        executed_base_qty: Decimal::ZERO,
        executed_quote_qty: Decimal::ZERO,
        status: OrderStatus::New,
        created_at: fixed_time(),
        updated_at: fixed_time(),
    }
}

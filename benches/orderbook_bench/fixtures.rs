//! `OrderBook` benchmark fixture.
//!
//! 주문 생성 방식이 각 측정 모듈마다 달라지면 benchmark 결과를 비교하기 어렵다.
//! 이 모듈에서 호가 분포와 order id range를 고정해 자료구조 경로만 다르게 측정한다.

use matching_engine::{
    domain::order::{Order, Side, TimeInForce},
    engine::orderbook::OrderBook,
};

use crate::bench_support::{limit_order, price_for};

pub(super) const INPUT_SIZES: [usize; 3] = [100, 1_000, 10_000];

pub(super) fn ask_orders(count: usize) -> Vec<Order> {
    // 여러 ask price level에 주문을 분산해 top-of-book 조회와 유동성 스캔 경로를 만든다.
    (0..count)
        .map(|index| {
            let order_id = index as u128 + 1;
            limit_order(
                Side::Sell,
                order_id,
                TimeInForce::GTC,
                price_for(index, 100),
                1,
            )
        })
        .collect()
}

fn bid_orders(count: usize) -> Vec<Order> {
    // ask_orders와 반대편 book을 만들어 base 수량 FOK 스캔이 양쪽 자료구조를 모두 갖춘
    // 상태에서 실행되도록 한다.
    (0..count)
        .map(|index| {
            let order_id = 1_000_000_u128 + index as u128;
            limit_order(
                Side::Buy,
                order_id,
                TimeInForce::GTC,
                price_for(index, 90),
                1,
            )
        })
        .collect()
}

pub(super) fn same_price_ask_orders(count: usize) -> Vec<Order> {
    // 같은 price level에 주문을 몰아 remove_order의 queue scan 비용을 드러낸다.
    (0..count)
        .map(|index| {
            let order_id = 2_000_000_u128 + index as u128;
            limit_order(Side::Sell, order_id, TimeInForce::GTC, 100, 1)
        })
        .collect()
}

pub(super) fn orderbook_with(orders: &[Order]) -> OrderBook {
    // 조회 계열 benchmark에서는 setup 비용을 제외하고 이미 구성된 book을 입력으로 넘긴다.
    let mut orderbook = OrderBook::default();

    for order in orders {
        orderbook.add_order(order.clone());
    }

    orderbook
}

pub(super) fn two_sided_orders(count: usize) -> Vec<Order> {
    // buy/sell 양쪽 book이 모두 있는 상태를 만들지만, 측정 대상은 buy taker 관점의
    // 반대편 ask 유동성 스캔이다.
    ask_orders(count)
        .into_iter()
        .chain(bid_orders(count))
        .collect()
}

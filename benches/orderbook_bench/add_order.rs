//! 빈 `OrderBook`에 주문을 적재하는 비용을 측정한다.
//!
//! 주문 생성은 입력 fixture로 미리 끝내고, 측정 구간에는 `add_order` 반복만 남긴다.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion};
use matching_engine::engine::orderbook::OrderBook;

use super::fixtures::{INPUT_SIZES, ask_orders};

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook/add_order");

    for count in INPUT_SIZES {
        let asks = ask_orders(count);
        group.bench_with_input(BenchmarkId::from_parameter(count), &asks, |b, orders| {
            b.iter_batched(
                OrderBook::default,
                |mut orderbook| {
                    // 새 book을 매 iteration마다 사용해 이전 iteration의 누적 상태가
                    // 삽입 비용에 섞이지 않게 한다.
                    for order in orders {
                        let _ = orderbook.add_order(black_box(order.clone()));
                    }
                    black_box(orderbook);
                },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

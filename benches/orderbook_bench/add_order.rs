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
                    for order in orders {
                        orderbook.add_order(black_box(order.clone()));
                    }
                    black_box(orderbook);
                },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion};
use matching_engine::domain::order::Side;

use crate::bench_support::decimal;

use super::fixtures::{INPUT_SIZES, ask_orders, orderbook_with};

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook/can_fully_fill_quote");

    for count in INPUT_SIZES {
        let asks = ask_orders(count);
        let book = orderbook_with(&asks);
        let requested_quote = decimal((count * 75) as i64);

        group.bench_with_input(BenchmarkId::new("buy_quote", count), &book, |b, book| {
            b.iter(|| {
                black_box(
                    book.can_fully_fill_quote(black_box(Side::Buy), black_box(requested_quote)),
                );
            });
        });
    }

    group.finish();
}

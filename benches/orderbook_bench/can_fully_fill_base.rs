use std::hint::black_box;

use criterion::{BenchmarkId, Criterion};
use matching_engine::domain::order::Side;

use crate::bench_support::decimal;

use super::fixtures::{INPUT_SIZES, orderbook_with, two_sided_orders};

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook/can_fully_fill_base");

    for count in INPUT_SIZES {
        let orders = two_sided_orders(count);
        let book = orderbook_with(&orders);
        let requested_qty = decimal((count / 2) as i64);

        group.bench_with_input(
            BenchmarkId::new("buy_across_levels", count),
            &book,
            |b, book| {
                b.iter(|| {
                    black_box(book.can_fully_fill_base(
                        black_box(Side::Buy),
                        black_box(requested_qty),
                        black_box(decimal(149)),
                    ));
                });
            },
        );
    }

    group.finish();
}

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion};
use matching_engine::domain::order::Side;

use super::fixtures::{INPUT_SIZES, ask_orders, orderbook_with};

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook/get_best_opposite");

    for count in INPUT_SIZES {
        let asks = ask_orders(count);
        let ask_book = orderbook_with(&asks);
        group.bench_with_input(
            BenchmarkId::new("buy_from_asks", count),
            &ask_book,
            |b, book| {
                b.iter(|| black_box(book.get_best_opposite(black_box(&Side::Buy))));
            },
        );
    }

    group.finish();
}

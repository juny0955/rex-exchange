//! 최우선 반대 호가 조회 비용을 측정한다.
//!
//! 현재 구현은 top-of-book price 조회뿐 아니라 같은 price level의 `VecDeque<Uuid>` clone도
//! 포함하므로, count 증가에 따른 queue clone 영향까지 함께 본다.

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
                // 이미 구성된 book을 공유해 fixture build 비용을 제외하고 조회 경로만 반복한다.
                b.iter(|| black_box(book.get_best_opposite(black_box(&Side::Buy))));
            },
        );
    }

    group.finish();
}

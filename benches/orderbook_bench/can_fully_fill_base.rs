//! Base 수량 기준 전량 체결 가능성 확인 비용을 측정한다.
//!
//! Buy taker가 지정가 안에서 ask 유동성을 얼마나 스캔해야 하는지 보는 benchmark다.

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
                    // limit price 149는 fixture의 ask ladder 대부분을 통과시켜
                    // 단일 top level 조회가 아니라 누적 유동성 스캔 비용을 드러낸다.
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

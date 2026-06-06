//! Quote 금액 기준 전량 체결 가능성 확인 비용을 측정한다.
//!
//! Market Buy Quote 주문을 사전에 검증할 때 ask ladder를 가격 * 수량으로 누적하는 경로다.

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
                // requested_quote는 전체 ask 유동성보다 작게 잡아 일부 level을 스캔한 뒤
                // true로 끝나는 일반적인 사전 검증 경로를 측정한다.
                black_box(
                    book.can_fully_fill_quote(black_box(Side::Buy), black_box(requested_quote)),
                );
            });
        });
    }

    group.finish();
}

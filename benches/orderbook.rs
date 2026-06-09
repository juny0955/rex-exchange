//! `OrderBook` 자료구조 비용을 측정하는 Criterion entrypoint.
//!
//! 실제 측정 로직은 `orderbook_bench` 하위 모듈에 나누고, 이 파일은 benchmark group을
//! Criterion에 등록하는 역할만 맡는다.

use criterion::{Criterion, criterion_group, criterion_main};

mod bench_support;
mod orderbook_bench;

fn bench_add_order(c: &mut Criterion) {
    orderbook_bench::add_order::bench(c);
}

fn bench_can_fully_fill_base(c: &mut Criterion) {
    orderbook_bench::can_fully_fill_base::bench(c);
}

fn bench_can_fully_fill_quote(c: &mut Criterion) {
    orderbook_bench::can_fully_fill_quote::bench(c);
}

fn bench_remove_order(c: &mut Criterion) {
    orderbook_bench::remove_order::bench(c);
}

criterion_group!(
    benches,
    bench_add_order,
    bench_get_best_opposite,
    bench_can_fully_fill_base,
    bench_can_fully_fill_quote,
    bench_remove_order
);
criterion_main!(benches);

//! `MatchingEngine` 내부 주문 처리 경로를 측정하는 Criterion entrypoint.
//!
//! 채널, 스레드, gRPC 비용은 제외하고 `bench-internals` feature로 열린 내부 API만 호출한다.
//! 실제 scenario별 측정 로직은 `matching_engine_bench` 하위 모듈에 둔다.

use criterion::{Criterion, criterion_group, criterion_main};

mod bench_support;
mod matching_engine_bench;

fn bench_place_order(c: &mut Criterion) {
    matching_engine_bench::place_order::bench(c);
}

fn bench_cancel_order(c: &mut Criterion) {
    matching_engine_bench::cancel_order::bench(c);
}

fn bench_amend_order(c: &mut Criterion) {
    matching_engine_bench::amend_order::bench(c);
}

criterion_group!(
    benches,
    bench_place_order,
    bench_cancel_order,
    bench_amend_order
);
criterion_main!(benches);

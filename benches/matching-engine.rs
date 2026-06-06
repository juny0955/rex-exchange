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

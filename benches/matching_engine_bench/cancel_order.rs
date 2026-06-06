use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};
use uuid::Uuid;

use super::fixtures::{INPUT_SIZES, make_engine, middle_bid_order_id, seed_same_price_bids};

const MISSING_ORDER_ID: u128 = 99_999_999;

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("matching_engine/cancel_order");

    for count in INPUT_SIZES {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("existing_middle_same_price", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut engine = make_engine();
                        seed_same_price_bids(&mut engine, count, 1);
                        (engine, middle_bid_order_id(count))
                    },
                    |(mut engine, target_order_id)| {
                        black_box(engine.bench_cancel_order(black_box(target_order_id)));
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("missing_order", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut engine = make_engine();
                        seed_same_price_bids(&mut engine, count, 1);
                        (engine, Uuid::from_u128(MISSING_ORDER_ID))
                    },
                    |(mut engine, target_order_id)| {
                        black_box(engine.bench_cancel_order(black_box(target_order_id)));
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

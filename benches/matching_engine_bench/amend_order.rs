use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};
use matching_engine::engine::command::AmendOrderCommand;

use super::fixtures::{
    INPUT_SIZES, decimal, make_engine, middle_bid_order_id, seed_same_price_bids,
};

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("matching_engine/amend_order");

    for count in INPUT_SIZES {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("decrease_qty_in_place", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut engine = make_engine();
                        seed_same_price_bids(&mut engine, count, 2);
                        let command = AmendOrderCommand {
                            order_id: middle_bid_order_id(count),
                            price: Some(decimal(100)),
                            base_qty: Some(decimal(1)),
                        };
                        (engine, command)
                    },
                    |(mut engine, command)| {
                        black_box(engine.bench_amend_order(black_box(command)));
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("price_change_cancel_replace", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut engine = make_engine();
                        seed_same_price_bids(&mut engine, count, 2);
                        let command = AmendOrderCommand {
                            order_id: middle_bid_order_id(count),
                            price: Some(decimal(101)),
                            base_qty: Some(decimal(2)),
                        };
                        (engine, command)
                    },
                    |(mut engine, command)| {
                        black_box(engine.bench_amend_order(black_box(command)));
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

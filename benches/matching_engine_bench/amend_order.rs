//! `MatchingEngine::amend_order` 내부 경로를 측정한다.
//!
//! 정정은 가격/수량 변화에 따라 우선순위 유지와 cancel-replace로 나뉘므로 두 경로를 분리한다.

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

        // 가격은 유지하고 수량만 줄여 book index 안의 주문을 in-place로 갱신하는 경로.
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

        // 가격 변경으로 기존 주문을 취소하고 새 주문으로 재등록해 우선순위를 잃는 경로.
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

//! `MatchingEngine::cancel_order` 내부 경로를 측정한다.
//!
//! 성공 case는 same-price queue에서 주문을 제거하는 비용을 보고, 실패 case는 index lookup miss와
//! rejected result 생성 비용을 분리해서 본다.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};
use uuid::Uuid;

use super::fixtures::{INPUT_SIZES, make_engine, middle_bid_order_id, seed_same_price_bids};

const MISSING_ORDER_ID: u128 = 99_999_999;

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("matching_engine/cancel_order");

    for count in INPUT_SIZES {
        group.throughput(Throughput::Elements(count as u64));

        // 같은 가격 bid queue의 중간 주문을 취소해 remove_order의 queue scan 비용을 포함한다.
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

        // book에 없는 order id를 취소해 주문 조회 실패와 reject result 생성 비용만 측정한다.
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

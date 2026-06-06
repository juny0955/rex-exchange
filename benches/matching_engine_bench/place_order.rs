use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput};
use matching_engine::domain::order::{Side, TimeInForce};

use super::fixtures::{
    INPUT_SIZES, TAKER_ORDER_ID_BASE, limit_order, make_engine, market_quote_buy, seed_asks,
    seed_same_price_asks,
};

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("matching_engine/place_order");

    for count in INPUT_SIZES {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("resting_limit_no_cross", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut engine = make_engine();
                        seed_asks(&mut engine, count);
                        let taker = limit_order(
                            Side::Buy,
                            TAKER_ORDER_ID_BASE + count as u128,
                            TimeInForce::GTC,
                            90,
                            1,
                        );
                        (engine, taker)
                    },
                    |(mut engine, taker)| {
                        black_box(engine.bench_place_order(black_box(taker)));
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("full_fill_same_level", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut engine = make_engine();
                        seed_same_price_asks(&mut engine, count);
                        let taker = limit_order(
                            Side::Buy,
                            TAKER_ORDER_ID_BASE + 100_000 + count as u128,
                            TimeInForce::IOC,
                            100,
                            count as i64,
                        );
                        (engine, taker)
                    },
                    |(mut engine, taker)| {
                        black_box(engine.bench_place_order(black_box(taker)));
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("market_quote_sweep", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut engine = make_engine();
                        seed_asks(&mut engine, count);
                        let taker = market_quote_buy(
                            TAKER_ORDER_ID_BASE + 200_000 + count as u128,
                            count as i64 * 150,
                        );
                        (engine, taker)
                    },
                    |(mut engine, taker)| {
                        black_box(engine.bench_place_order(black_box(taker)));
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fok_reject_insufficient_liquidity", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut engine = make_engine();
                        seed_same_price_asks(&mut engine, count);
                        let taker = limit_order(
                            Side::Buy,
                            TAKER_ORDER_ID_BASE + 300_000 + count as u128,
                            TimeInForce::FOK,
                            100,
                            count as i64 + 1,
                        );
                        (engine, taker)
                    },
                    |(mut engine, taker)| {
                        black_box(engine.bench_place_order(black_box(taker)));
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

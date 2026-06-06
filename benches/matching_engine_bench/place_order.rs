//! `MatchingEngine::place_order` 내부 경로를 scenario별로 측정한다.
//!
//! 모든 case는 엔진 상태를 mutate하므로 `iter_batched`로 매 iteration마다 같은 fixture를
//! 다시 만들고, 측정 closure에는 place 호출만 남긴다.

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

        // 반대편 ask가 있어도 taker limit price가 낮아 매칭 없이 GTC로 book에 resting 되는 경로.
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

        // 같은 가격의 maker N개를 taker 하나가 모두 체결해 trade/result 생성 비용까지 포함한다.
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

        // Quote size Market Buy가 ask ladder를 sweep하며 가격 level과 잔여 quote를 갱신하는 경로.
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

        // FOK 주문이 실제 체결에 들어가기 전 유동성 검증에서 취소되는 사전 검사 경로.
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

//! 같은 price level 안에서 주문을 제거하는 비용을 측정한다.
//!
//! `remove_order`는 현재 queue retain 기반이라 삭제 위치보다 같은 level 주문 수에 더 민감하다.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion};

use super::fixtures::{INPUT_SIZES, orderbook_with, same_price_ask_orders};

#[derive(Clone, Copy)]
enum RemovalPosition {
    Front,
    Middle,
    Back,
}

impl RemovalPosition {
    const ALL: [Self; 3] = [Self::Front, Self::Middle, Self::Back];

    const fn label(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::Middle => "middle",
            Self::Back => "back",
        }
    }

    const fn index(self, len: usize) -> usize {
        match self {
            Self::Front => 0,
            Self::Middle => len / 2,
            Self::Back => len - 1,
        }
    }
}

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook/remove_order");

    for count in INPUT_SIZES {
        let asks = same_price_ask_orders(count);

        for position in RemovalPosition::ALL {
            let target_order_id = asks[position.index(asks.len())].order_id;

            group.bench_with_input(
                BenchmarkId::new(position.label(), count),
                &asks,
                |b, orders| {
                    b.iter_batched(
                        || orderbook_with(orders),
                        |mut orderbook| {
                            // 제거는 book을 mutate하므로 매 iteration마다 동일한 book을 다시 만들고,
                            // 측정 구간에는 remove_order 하나만 둔다.
                            black_box(orderbook.remove_order(black_box(target_order_id)));
                        },
                        BatchSize::LargeInput,
                    );
                },
            );
        }
    }

    group.finish();
}

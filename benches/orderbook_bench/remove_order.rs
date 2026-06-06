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

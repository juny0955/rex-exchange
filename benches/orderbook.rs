use std::{hint::black_box, time::SystemTime};

use chrono::{DateTime, Utc};
use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use matching_engine::{
    domain::order::{Order, OrderSize, OrderStatus, OrderType, Side, TimeInForce},
    engine::orderbook::OrderBook,
};
use rust_decimal::Decimal;
use uuid::Uuid;

const SYMBOL: &str = "BTCUSDT";
const PRICE_LEVELS: u128 = 50;
const INPUT_SIZES: [usize; 3] = [100, 1_000, 10_000];

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

fn fixed_time() -> DateTime<Utc> {
    SystemTime::UNIX_EPOCH.into()
}

fn decimal(value: i64) -> Decimal {
    Decimal::new(value, 0)
}

fn price_for(index: u128, base_price: i64) -> Decimal {
    decimal(base_price + (index % PRICE_LEVELS) as i64)
}

fn limit_order(side: Side, order_id: u128, price: Decimal, qty: Decimal) -> Order {
    Order {
        order_id: Uuid::from_u128(order_id),
        symbol: SYMBOL.to_string(),
        side,
        order_type: OrderType::Limit,
        tif: TimeInForce::GTC,
        price: Some(price),
        size: OrderSize::Base(qty),
        executed_base_qty: Decimal::ZERO,
        executed_quote_qty: Decimal::ZERO,
        status: OrderStatus::New,
        created_at: fixed_time(),
        updated_at: fixed_time(),
    }
}

fn ask_orders(count: usize) -> Vec<Order> {
    (0..count)
        .map(|index| {
            let order_id = index as u128 + 1;
            limit_order(
                Side::Sell,
                order_id,
                price_for(index as u128, 100),
                decimal(1),
            )
        })
        .collect()
}

fn bid_orders(count: usize) -> Vec<Order> {
    (0..count)
        .map(|index| {
            let order_id = 1_000_000_u128 + index as u128;
            limit_order(
                Side::Buy,
                order_id,
                price_for(index as u128, 90),
                decimal(1),
            )
        })
        .collect()
}

fn same_price_ask_orders(count: usize) -> Vec<Order> {
    (0..count)
        .map(|index| {
            let order_id = 2_000_000_u128 + index as u128;
            limit_order(Side::Sell, order_id, decimal(100), decimal(1))
        })
        .collect()
}

fn orderbook_with(orders: &[Order]) -> OrderBook {
    let mut orderbook = OrderBook::default();

    for order in orders {
        orderbook.add_order(order.clone());
    }

    orderbook
}

fn two_sided_orders(count: usize) -> Vec<Order> {
    ask_orders(count)
        .into_iter()
        .chain(bid_orders(count))
        .collect()
}

fn bench_add_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook/add_order");

    for count in INPUT_SIZES {
        let asks = ask_orders(count);
        group.bench_with_input(BenchmarkId::from_parameter(count), &asks, |b, orders| {
            b.iter_batched(
                OrderBook::default,
                |mut orderbook| {
                    for order in orders {
                        orderbook.add_order(black_box(order.clone()));
                    }
                    black_box(orderbook);
                },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

fn bench_get_best_opposite(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook/get_best_opposite");

    for count in INPUT_SIZES {
        let asks = ask_orders(count);
        let ask_book = orderbook_with(&asks);
        group.bench_with_input(
            BenchmarkId::new("buy_from_asks", count),
            &ask_book,
            |b, book| {
                b.iter(|| black_box(book.get_best_opposite(black_box(&Side::Buy))));
            },
        );
    }

    group.finish();
}

fn bench_can_fully_fill_base(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook/can_fully_fill_base");

    for count in INPUT_SIZES {
        let orders = two_sided_orders(count);
        let book = orderbook_with(&orders);
        let requested_qty = decimal((count / 2) as i64);

        group.bench_with_input(
            BenchmarkId::new("buy_across_levels", count),
            &book,
            |b, book| {
                b.iter(|| {
                    black_box(book.can_fully_fill_base(
                        black_box(Side::Buy),
                        black_box(requested_qty),
                        black_box(decimal(149)),
                    ));
                });
            },
        );
    }

    group.finish();
}

fn bench_can_fully_fill_quote(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook/can_fully_fill_quote");

    for count in INPUT_SIZES {
        let asks = ask_orders(count);
        let book = orderbook_with(&asks);
        let requested_quote = decimal((count * 75) as i64);

        group.bench_with_input(BenchmarkId::new("buy_quote", count), &book, |b, book| {
            b.iter(|| {
                black_box(
                    book.can_fully_fill_quote(black_box(Side::Buy), black_box(requested_quote)),
                );
            });
        });
    }

    group.finish();
}

fn bench_remove_order(c: &mut Criterion) {
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

criterion_group!(
    benches,
    bench_add_order,
    bench_get_best_opposite,
    bench_can_fully_fill_base,
    bench_can_fully_fill_quote,
    bench_remove_order
);
criterion_main!(benches);

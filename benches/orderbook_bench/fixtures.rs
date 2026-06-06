use matching_engine::{
    domain::order::{Order, Side, TimeInForce},
    engine::orderbook::OrderBook,
};

use crate::bench_support::{limit_order, price_for};

pub(super) const INPUT_SIZES: [usize; 3] = [100, 1_000, 10_000];

pub(super) fn ask_orders(count: usize) -> Vec<Order> {
    (0..count)
        .map(|index| {
            let order_id = index as u128 + 1;
            limit_order(
                Side::Sell,
                order_id,
                TimeInForce::GTC,
                price_for(index, 100),
                1,
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
                TimeInForce::GTC,
                price_for(index, 90),
                1,
            )
        })
        .collect()
}

pub(super) fn same_price_ask_orders(count: usize) -> Vec<Order> {
    (0..count)
        .map(|index| {
            let order_id = 2_000_000_u128 + index as u128;
            limit_order(Side::Sell, order_id, TimeInForce::GTC, 100, 1)
        })
        .collect()
}

pub(super) fn orderbook_with(orders: &[Order]) -> OrderBook {
    let mut orderbook = OrderBook::default();

    for order in orders {
        orderbook.add_order(order.clone());
    }

    orderbook
}

pub(super) fn two_sided_orders(count: usize) -> Vec<Order> {
    ask_orders(count)
        .into_iter()
        .chain(bid_orders(count))
        .collect()
}

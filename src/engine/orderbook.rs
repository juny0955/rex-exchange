use std::{
    cmp::Reverse,
    collections::{BTreeMap, VecDeque},
};

use rust_decimal::Decimal;

use crate::domain::order::Order;

#[derive(Debug, Default)]
pub struct OrderBook {
    // 매수 호가 Price 내림차순
    pub bids: BTreeMap<Reverse<Decimal>, VecDeque<Order>>,
    // 매도 호가 Price 오름차순
    pub asks: BTreeMap<Decimal, VecDeque<Order>>,
}

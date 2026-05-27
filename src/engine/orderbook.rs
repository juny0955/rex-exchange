use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap, VecDeque},
};

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::order::{Order, Side};

#[derive(Debug, Default)]
pub struct OrderBook {
    // 매수 호가 Price 내림차순
    bids: BTreeMap<Reverse<Decimal>, VecDeque<Uuid>>,
    // 매도 호가 Price 오름차순
    asks: BTreeMap<Decimal, VecDeque<Uuid>>,
    // 실제 주문 데이터
    index: HashMap<Uuid, Order>,
}

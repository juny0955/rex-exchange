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

impl OrderBook {
    /// 최상위 반대 호가 조회
    /// Side::Buy -> asks
    /// Side::Sell -> bids
    pub fn get_best_opposite(&self, side: &Side) -> Option<(Decimal, VecDeque<Uuid>)> {
        match side {
            Side::Buy => self
                .asks
                .iter()
                .next()
                .map(|(price, order_ids)| (*price, order_ids.clone())),
            Side::Sell => self
                .bids
                .iter()
                .next()
                .map(|(price, order_ids)| (price.0, order_ids.clone())),
        }
    }

    /// 주문 추가
    /// 주문을 index와 queue에 추가
    pub fn add_order(&mut self, order: Order) {
        match order.side {
            Side::Buy => {
                self.bids
                    .entry(Reverse(order.price))
                    .or_default()
                    .push_back(order.order_id);
            }
            Side::Sell => {
                self.asks
                    .entry(order.price)
                    .or_default()
                    .push_back(order.order_id);
            }
        }

        self.index.insert(order.order_id, order);
    }

    /// 주문 삭제
    /// TODO: 현재 O(n) 개선 필요
    pub fn remove_order(&mut self, order_id: Uuid) {
        let order = self.index.remove(&order_id).unwrap();

        match order.side {
            Side::Buy => {
                let price_key = Reverse(order.price);

                if let Some(queue) = self.bids.get_mut(&price_key) {
                    queue.retain(|id| *id != order_id);

                    if queue.is_empty() {
                        self.bids.remove(&price_key);
                    }
                }
            }
            Side::Sell => {
                if let Some(queue) = self.asks.get_mut(&order.price) {
                    queue.retain(|id| *id != order_id);

                    if queue.is_empty() {
                        self.asks.remove(&order.price);
                    }
                }
            }
        }
    }

    /// 전량 체결 가능 여부 확인
    pub fn can_fully_fill(&self, side: Side, qty: Decimal, price: Decimal) -> bool {
        let mut remaining = qty;
        match side {
            Side::Buy => {
                for (ask_price, queue) in &self.asks {
                    if *ask_price > price {
                        break;
                    }

                    for order_id in queue {
                        let Some(order) = self.index.get(order_id) else {
                            continue;
                        };

                        remaining -= order.remaining_quantity;

                        if remaining <= Decimal::ZERO {
                            return true;
                        }
                    }
                }

                false
            }
            Side::Sell => {
                for (ask_price, queue) in &self.bids {
                    if ask_price.0 < price {
                        break;
                    }

                    for order_id in queue {
                        let Some(order) = self.index.get(order_id) else {
                            continue;
                        };

                        remaining -= order.remaining_quantity;

                        if remaining <= Decimal::ZERO {
                            return true;
                        }
                    }
                }

                false
            }
        }
    }

    /// 주문 조회(mutable)
    pub fn get_order_mut(&mut self, order_id: &Uuid) -> Option<&mut Order> {
        self.index.get_mut(order_id)
    }
}

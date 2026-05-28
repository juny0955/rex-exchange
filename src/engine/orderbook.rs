use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap, VecDeque},
    ops::Mul,
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
                    .entry(Reverse(order.price.unwrap()))
                    .or_default()
                    .push_back(order.order_id);
            }
            Side::Sell => {
                self.asks
                    .entry(order.price.unwrap())
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
        let price = order.price.unwrap();

        match order.side {
            Side::Buy => {
                let price_key = Reverse(price);

                if let Some(queue) = self.bids.get_mut(&price_key) {
                    queue.retain(|id| *id != order_id);

                    if queue.is_empty() {
                        self.bids.remove(&price_key);
                    }
                }
            }
            Side::Sell => {
                if let Some(queue) = self.asks.get_mut(&price) {
                    queue.retain(|id| *id != order_id);

                    if queue.is_empty() {
                        self.asks.remove(&price);
                    }
                }
            }
        }
    }

    /// 전량 체결 가능 여부 확인 (base)
    pub fn can_fully_fill_base(&self, side: Side, mut qty: Decimal, price: Decimal) -> bool {
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

                        qty -= order.remaining_base_qty().unwrap();

                        if qty <= Decimal::ZERO {
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

                        qty -= order.remaining_base_qty().unwrap();

                        if qty <= Decimal::ZERO {
                            return true;
                        }
                    }
                }

                false
            }
        }
    }

    /// 전량 체결 가능 여부 확인 (quote)
    pub fn can_fully_fill_quote(&self, side: Side, mut quote: Decimal) -> bool {
        match side {
            Side::Buy => {
                for (ask_price, queue) in &self.asks {
                    for order_id in queue {
                        let Some(order) = self.index.get(order_id) else {
                            continue;
                        };

                        quote -= order.remaining_base_qty().unwrap().mul(*ask_price);

                        if quote <= Decimal::ZERO {
                            return true;
                        }
                    }
                }

                false
            }
            Side::Sell => {
                unreachable!("Quote + Sell 주문 지원하지않음");
            }
        }
    }

    /// 주문 조회(mutable)
    pub fn get_order_mut(&mut self, order_id: &Uuid) -> Option<&mut Order> {
        self.index.get_mut(order_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use uuid::Uuid;

    use crate::domain::order::{Order, OrderSize, OrderStatus, OrderType, Side, TimeInForce};

    fn make_ask(price: Decimal, qty: Decimal) -> Order {
        Order {
            order_id: Uuid::now_v7(),
            symbol: "BTC/USDT".to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            tif: TimeInForce::GTC,
            price: Some(price),
            size: OrderSize::Base(qty),
            executed_base_qty: Decimal::ZERO,
            executed_quote_qty: Decimal::ZERO,
            status: OrderStatus::New,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_bid(price: Decimal, qty: Decimal) -> Order {
        Order {
            order_id: Uuid::now_v7(),
            symbol: "BTC/USDT".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            tif: TimeInForce::GTC,
            price: Some(price),
            size: OrderSize::Base(qty),
            executed_base_qty: Decimal::ZERO,
            executed_quote_qty: Decimal::ZERO,
            status: OrderStatus::New,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn 최상위_반대편_호가_조회_테스트() {
        let mut ob = OrderBook::default();
        ob.add_order(make_ask(Decimal::new(200, 0), Decimal::new(5, 0)));
        ob.add_order(make_ask(Decimal::new(100, 0), Decimal::new(5, 0)));

        let (ask_price, _) = ob.get_best_opposite(&Side::Buy).unwrap();
        assert_eq!(ask_price, Decimal::new(100, 0)); // 최저 매도호가

        ob.add_order(make_bid(Decimal::new(80, 0), Decimal::new(5, 0)));
        ob.add_order(make_bid(Decimal::new(90, 0), Decimal::new(5, 0)));

        let (bid_price, _) = ob.get_best_opposite(&Side::Sell).unwrap();
        assert_eq!(bid_price, Decimal::new(90, 0)); // 최고 매수호가
    }

    #[test]
    fn base_전량_체결_가능_확인_테스트() {
        let mut ob = OrderBook::default();
        // 100에 qty 5, 120에 qty 10
        ob.add_order(make_ask(Decimal::new(100, 0), Decimal::new(5, 0)));
        ob.add_order(make_ask(Decimal::new(120, 0), Decimal::new(10, 0)));

        // 지정가 110 → ask@100만 유효(qty 5)
        assert!(!ob.can_fully_fill_base(Side::Buy, Decimal::new(8, 0), Decimal::new(110, 0)));
        assert!(ob.can_fully_fill_base(Side::Buy, Decimal::new(3, 0), Decimal::new(110, 0)));

        // 지정가 130 → 두 레벨 합산(qty 15)
        assert!(ob.can_fully_fill_base(Side::Buy, Decimal::new(12, 0), Decimal::new(130, 0)));
    }

    #[test]
    fn quote_전량_체결_가능_확인_테스트() {
        let mut ob = OrderBook::default();
        // ask@100 qty 3 (300), ask@200 qty 2 (400) → 총 700
        ob.add_order(make_ask(Decimal::new(100, 0), Decimal::new(3, 0)));
        ob.add_order(make_ask(Decimal::new(200, 0), Decimal::new(2, 0)));

        assert!(!ob.can_fully_fill_quote(Side::Buy, Decimal::new(800, 0)));
        assert!(ob.can_fully_fill_quote(Side::Buy, Decimal::new(500, 0)));
    }

    #[test]
    fn 삭제시_빈_호가_정리_테스트() {
        let mut ob = OrderBook::default();
        let order = make_ask(Decimal::new(100, 0), Decimal::new(5, 0));
        let order_id = order.order_id;
        ob.add_order(order);

        assert!(ob.get_best_opposite(&Side::Buy).is_some());

        ob.remove_order(order_id);

        assert!(ob.get_best_opposite(&Side::Buy).is_none());
    }
}

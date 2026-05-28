pub mod command;
mod orderbook;

use std::collections::VecDeque;

use command::EngineCommand;
use crossbeam::channel::Receiver;
use orderbook::OrderBook;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::order::{Order, OrderSize, OrderType, Side, TimeInForce};

pub struct Engine {
    orderbook: OrderBook,
    engine_rx: Receiver<EngineCommand>,
}

impl Engine {
    pub fn new(engine_rx: Receiver<EngineCommand>) -> Self {
        Self {
            orderbook: OrderBook::default(),
            engine_rx,
        }
    }

    /// 엔진 실행
    pub fn run(&mut self) {
        while let Ok(command) = self.engine_rx.recv() {
            match command {
                EngineCommand::Place(order) => match order.tif {
                    TimeInForce::GTC => {
                        assert_eq!(order.order_type, OrderType::Limit);
                        self.place_order(order, true, false);
                    }
                    TimeInForce::IOC => self.place_order(order, false, false),
                    TimeInForce::FOK => self.place_order(order, false, true),
                },
            }
        }
    }

    /// 주문 접수
    fn place_order(&mut self, mut incoming: Order, resting: bool, is_fok: bool) {
        if is_fok && !self.validation_fok_order(&incoming) {
            return;
        }

        while let Some((price, restings)) = self.orderbook.get_best_opposite(&incoming.side) {
            if !can_match(&incoming, price) {
                break;
            }

            incoming = self.match_loop(incoming, restings);

            if incoming.is_filled() {
                break;
            }
        }

        if !incoming.is_filled() && resting {
            self.orderbook.add_order(incoming);
        }
    }

    /// 단일 Price level과 주문 매칭 수행
    fn match_loop(&mut self, mut incoming: Order, restings: VecDeque<Uuid>) -> Order {
        for rest_id in restings {
            let rest_filled = {
                let rest = self.orderbook.get_order_mut(&rest_id).unwrap();
                let rest_price = rest.price.unwrap();

                let fill_base = match incoming.size {
                    OrderSize::Base(_) => rest
                        .remaining_base_qty()
                        .unwrap()
                        .min(incoming.remaining_base_qty().unwrap()),
                    OrderSize::Quote(_) => rest
                        .remaining_base_qty()
                        .unwrap()
                        .min(incoming.remaining_quote_qty().unwrap() / rest_price),
                };
                let fill_quote = fill_base * rest_price;
                rest.fill(fill_base, fill_quote);
                incoming.fill(fill_base, fill_quote);

                rest.is_filled()
            };

            if rest_filled {
                self.orderbook.remove_order(rest_id);
            }

            if incoming.is_filled() {
                break;
            }
        }

        incoming
    }

    fn validation_fok_order(&self, incoming: &Order) -> bool {
        match incoming.size {
            OrderSize::Base(qty) => {
                let price = match incoming.order_type {
                    // Market 주문은 price 미존재
                    OrderType::Market => match incoming.side {
                        Side::Buy => Decimal::MAX,
                        Side::Sell => Decimal::ZERO,
                    },
                    OrderType::Limit => incoming.price.unwrap(),
                };

                self.orderbook
                    .can_fully_fill_base(incoming.side, qty, price)
            }
            OrderSize::Quote(quote_qty) => {
                // MARKET + BUY + QUOTE 전용
                // LIMIT + BUY + QUOTE는 엔진 진입전 base로 변환됨
                self.orderbook
                    .can_fully_fill_quote(incoming.side, quote_qty)
            }
        }
    }
}

/// incoming 주문과 반대 호가 가격 조건 확인
fn can_match(incoming: &Order, resting_price: Decimal) -> bool {
    match incoming.size {
        OrderSize::Base(_) => match incoming.order_type {
            OrderType::Limit => match incoming.side {
                Side::Buy => resting_price <= incoming.price.unwrap(),
                Side::Sell => resting_price >= incoming.price.unwrap(),
            },
            OrderType::Market => true,
        },
        OrderSize::Quote(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crossbeam::channel;
    use rust_decimal::Decimal;
    use uuid::Uuid;

    use crate::domain::order::{Order, OrderSize, OrderStatus, OrderType, Side, TimeInForce};

    fn make_engine() -> Engine {
        let (_, rx) = channel::unbounded();
        Engine::new(rx)
    }

    fn limit_order(side: Side, price: i64, qty: i64) -> Order {
        Order {
            order_id: Uuid::now_v7(),
            symbol: "BTC/USDT".to_string(),
            side,
            order_type: OrderType::Limit,
            tif: TimeInForce::GTC,
            price: Some(Decimal::new(price, 0)),
            size: OrderSize::Base(Decimal::new(qty, 0)),
            executed_base_qty: Decimal::ZERO,
            executed_quote_qty: Decimal::ZERO,
            status: OrderStatus::New,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn market_quote_order(side: Side, quote: i64) -> Order {
        Order {
            order_id: Uuid::now_v7(),
            symbol: "BTC/USDT".to_string(),
            side,
            order_type: OrderType::Market,
            tif: TimeInForce::IOC,
            price: None,
            size: OrderSize::Quote(Decimal::new(quote, 0)),
            executed_base_qty: Decimal::ZERO,
            executed_quote_qty: Decimal::ZERO,
            status: OrderStatus::New,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn 매수_매도_완전_체결_테스트() {
        let mut engine = make_engine();
        engine.orderbook.add_order(limit_order(Side::Sell, 100, 10));

        engine.place_order(limit_order(Side::Buy, 100, 10), false, false);

        assert!(engine.orderbook.get_best_opposite(&Side::Buy).is_none());
    }

    #[test]
    fn fok_잔량_부족시_취소_테스트() {
        let mut engine = make_engine();
        engine.orderbook.add_order(limit_order(Side::Sell, 100, 5));

        // FOK Buy 10, 잔량 5만 존재 → 취소
        engine.place_order(limit_order(Side::Buy, 100, 10), false, true);

        // SELL 여전히 원래 수량 그대로
        assert!(engine.orderbook.can_fully_fill_base(
            Side::Buy,
            Decimal::new(5, 0),
            Decimal::new(100, 0)
        ));
        assert!(!engine.orderbook.can_fully_fill_base(
            Side::Buy,
            Decimal::new(6, 0),
            Decimal::new(100, 0)
        ));
    }

    #[test]
    fn 시장가_quote_매수_체결_테스트() {
        let mut engine = make_engine();
        // SELL 10 BTC @ 100 = 총 1000 USDT
        engine.orderbook.add_order(limit_order(Side::Sell, 100, 10));

        // MARKET BUY 400 USDT → 4 BTC 체결
        engine.place_order(market_quote_order(Side::Buy, 400), false, false);

        // 잔여 6 BTC @ 100 = 600 USDT
        assert!(
            engine
                .orderbook
                .can_fully_fill_quote(Side::Buy, Decimal::new(500, 0))
        );
        assert!(
            !engine
                .orderbook
                .can_fully_fill_quote(Side::Buy, Decimal::new(700, 0))
        );
    }

    #[test]
    fn gtc_잔존_후_체결_테스트() {
        let mut engine = make_engine();

        // 매도 없음 → BUY GTC 잔존
        engine.place_order(limit_order(Side::Buy, 100, 5), true, false);
        assert!(engine.orderbook.get_best_opposite(&Side::Sell).is_some());

        // SELL 진입 → 잔존 BUY와 체결
        engine.place_order(limit_order(Side::Sell, 100, 5), false, false);
        assert!(engine.orderbook.get_best_opposite(&Side::Sell).is_none());
    }
}

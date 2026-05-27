pub mod command;
mod orderbook;

use std::collections::VecDeque;

use command::EngineCommand;
use crossbeam::channel::Receiver;
use orderbook::OrderBook;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::order::{Order, Side, TimeInForce};

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
                    TimeInForce::GTC => self.place_order(order, true, false),
                    TimeInForce::IOC => self.place_order(order, false, false),
                    TimeInForce::FOK => self.place_order(order, false, true),
                },
            }
        }
    }

    /// 주문 접수
    fn place_order(&mut self, mut incoming: Order, resting: bool, is_fok: bool) {
        if is_fok
            && !self
                .orderbook
                .can_fully_fill(incoming.side, incoming.quantity, incoming.price)
        {
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

                let fill_qty = rest.remaining_quantity.min(incoming.remaining_quantity);
                rest.fill(fill_qty);
                incoming.fill(fill_qty);

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
}

/// incoming 주문과 반대 호가 가격 조건 확인
fn can_match(incoming: &Order, resting_price: Decimal) -> bool {
    match incoming.side {
        Side::Buy => resting_price <= incoming.price,
        Side::Sell => resting_price >= incoming.price,
    }
}

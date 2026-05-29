use std::collections::VecDeque;

use crossbeam::channel::Receiver;
use rust_decimal::Decimal;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    domain::order::{Order, OrderSize, OrderType, Side, TimeInForce},
    engine::{command::EngineCommand, orderbook::OrderBook},
};

pub struct MatchingEngine {
    symbol: String,
    engine_rx: Receiver<EngineCommand>,
    orderbook: OrderBook,
}

impl MatchingEngine {
    pub fn new(symbol: String, engine_rx: Receiver<EngineCommand>) -> Self {
        Self {
            symbol,
            engine_rx,
            orderbook: OrderBook::default(),
        }
    }

    /// 엔진 실행
    pub fn run(&mut self) {
        info!(symbol = %self.symbol, "엔진 시작");
        while let Ok(command) = self.engine_rx.recv() {
            match command {
                EngineCommand::Place(order) => self.place_order(order),
            }
        }
    }

    /// 주문 접수
    fn place_order(&mut self, mut taker: Order) {
        debug!(
            symbol = %self.symbol,
            order_id = %taker.order_id,
            side = ?taker.side,
            order_type = ?taker.order_type,
            tif = ?taker.tif,
            price = ?taker.price,
            size = ?taker.size,
            "주문 접수"
        );

        if matches!(taker.tif, TimeInForce::GTC) && taker.order_type != OrderType::Limit {
            error!(
                symbol = %self.symbol,
                order_id = %taker.order_id,
                order_type = ?taker.order_type,
                tif = ?taker.tif,
                "주문 거부: GTC 주문 LIMIT만 허용"
            );
            return;
        }

        if matches!(taker.tif, TimeInForce::FOK) && !self.validation_fok_order(&taker) {
            debug!(
                symbol = %self.symbol,
                order_id = %taker.order_id,
                side = ?taker.side,
                order_type = ?taker.order_type,
                price = ?taker.price,
                size = ?taker.size,
                "주문 취소: FOK 전량 체결 불가"
            );
            return;
        }

        while let Some((price, makers)) = self.orderbook.get_best_opposite(&taker.side) {
            if !can_match(&taker, price) {
                debug!(
                    symbol = %self.symbol,
                    order_id = %taker.order_id,
                    side = ?taker.side,
                    taker_price = ?taker.price,
                    best_maker_price = %price,
                    "매칭 중단: 가격 조건 불일치"
                );
                break;
            }

            taker = self.match_loop(taker, makers);

            if taker.is_filled() {
                break;
            }
        }

        if !taker.is_filled() && matches!(taker.tif, TimeInForce::GTC) {
            debug!(
                symbol = %self.symbol,
                order_id = %taker.order_id,
                remaining_base_qty = ?taker.remaining_base_qty(),
                remaining_quote_qty = ?taker.remaining_quote_qty(),
                "오더북 등록"
            );

            self.orderbook.add_order(taker);
        }
    }

    /// 단일 Price level과 주문 매칭 수행
    fn match_loop(&mut self, mut taker: Order, makers: VecDeque<Uuid>) -> Order {
        for maker_id in makers {
            let maker_filled = {
                let maker = self.orderbook.get_order_mut(&maker_id).unwrap();
                let maker_price = maker.price.unwrap();

                let fill_base = match taker.size {
                    OrderSize::Base(_) => maker
                        .remaining_base_qty()
                        .unwrap()
                        .min(taker.remaining_base_qty().unwrap()),
                    OrderSize::Quote(_) => maker
                        .remaining_base_qty()
                        .unwrap()
                        .min(taker.remaining_quote_qty().unwrap() / maker_price),
                };
                let fill_quote = fill_base * maker_price;
                maker.fill(fill_base, fill_quote);
                taker.fill(fill_base, fill_quote);
                info!(
                    symbol = %self.symbol,
                    taker_order_id = %taker.order_id,
                    maker_order_id = %maker.order_id,
                    price = %maker_price,
                    fill_base = %fill_base,
                    fill_quote = %fill_quote,
                    taker_filled = taker.is_filled(),
                    maker_filled = maker.is_filled(),
                    "주문 체결"
                );

                maker.is_filled()
            };

            if maker_filled {
                debug!(
                    symbol = %self.symbol,
                    maker_order_id = %maker_id,
                    "메이커 주문 완전 체결 후 오더북 제거"
                );
                self.orderbook.remove_order(maker_id);
            }

            if taker.is_filled() {
                break;
            }
        }

        taker
    }

    fn validation_fok_order(&self, taker: &Order) -> bool {
        match taker.size {
            OrderSize::Base(qty) => {
                let price = match taker.order_type {
                    // Market 주문은 price 미존재
                    OrderType::Market => match taker.side {
                        Side::Buy => Decimal::MAX,
                        Side::Sell => Decimal::ZERO,
                    },
                    OrderType::Limit => taker.price.unwrap(),
                };

                self.orderbook.can_fully_fill_base(taker.side, qty, price)
            }
            OrderSize::Quote(quote_qty) => {
                // MARKET + BUY + QUOTE 전용
                // LIMIT + BUY + QUOTE는 엔진 진입전 base로 변환됨
                self.orderbook.can_fully_fill_quote(taker.side, quote_qty)
            }
        }
    }
}

/// taker 주문과 반대 호가 가격 조건 확인
fn can_match(taker: &Order, maker_price: Decimal) -> bool {
    match taker.size {
        OrderSize::Base(_) => match taker.order_type {
            OrderType::Limit => match taker.side {
                Side::Buy => maker_price <= taker.price.unwrap(),
                Side::Sell => maker_price >= taker.price.unwrap(),
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

    fn make_engine() -> MatchingEngine {
        let (_, rx) = channel::unbounded();
        let symbol = "BTCUSDT".to_string();
        MatchingEngine::new(symbol, rx)
    }

    fn limit_order(side: Side, tif: TimeInForce, price: i64, qty: i64) -> Order {
        Order {
            order_id: Uuid::now_v7(),
            symbol: "BTCUSDT".to_string(),
            side,
            order_type: OrderType::Limit,
            tif,
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
            symbol: "BTCUSDT".to_string(),
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
        engine
            .orderbook
            .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 10));

        engine.place_order(limit_order(Side::Buy, TimeInForce::GTC, 100, 10));

        assert!(engine.orderbook.get_best_opposite(&Side::Buy).is_none());
    }

    #[test]
    fn fok_잔량_부족시_취소_테스트() {
        let mut engine = make_engine();
        engine
            .orderbook
            .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 5));

        // FOK Buy 10, 잔량 5만 존재 → 취소
        engine.place_order(limit_order(Side::Buy, TimeInForce::FOK, 100, 10));

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
        engine
            .orderbook
            .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 10));

        // MARKET BUY 400 USDT → 4 BTC 체결
        engine.place_order(market_quote_order(Side::Buy, 400));

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
        engine.place_order(limit_order(Side::Buy, TimeInForce::GTC, 100, 5));
        assert!(engine.orderbook.get_best_opposite(&Side::Sell).is_some());

        // SELL 진입 → 잔존 BUY와 체결
        engine.place_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 5));
        assert!(engine.orderbook.get_best_opposite(&Side::Sell).is_none());
    }
}

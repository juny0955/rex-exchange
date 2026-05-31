use std::collections::VecDeque;

use crossbeam::channel::{Receiver, Sender};
use rust_decimal::Decimal;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::engine::result::{OrderSnapshot, TradeResult};
use crate::{
    domain::order::{Order, OrderSize, OrderType, Side, TimeInForce},
    engine::{
        command::EngineCommand,
        orderbook::OrderBook,
        result::{
            CancelledReason, EngineResult, PlaceOrderOutcome, PlaceOrderResult, RejectedReason,
        },
    },
};

pub struct MatchingEngine {
    symbol: String,
    engine_rx: Receiver<EngineCommand>,
    result_tx: Sender<EngineResult>,
    orderbook: OrderBook,
}

impl MatchingEngine {
    pub fn new(
        symbol: String,
        engine_rx: Receiver<EngineCommand>,
        result_tx: Sender<EngineResult>,
    ) -> Self {
        Self {
            symbol,
            engine_rx,
            result_tx,
            orderbook: OrderBook::default(),
        }
    }

    /// 엔진 실행
    pub fn run(&mut self) {
        info!(symbol = %self.symbol, "엔진 시작");
        while let Ok(command) = self.engine_rx.recv() {
            match command {
                EngineCommand::Place(order) => {
                    let order_id = order.order_id;
                    let result = self.place_order(order);

                    if let Err(e) = self.result_tx.send(result) {
                        error!(symbol = %self.symbol, order_id = %order_id, error = %e, "매칭 결과 전송 오류");
                    }
                }
            }
        }
    }

    /// 주문 접수
    fn place_order(&mut self, taker: Order) -> EngineResult {
        self.log_order_received(&taker);

        if let Some(outcome) = self.validate_before_matching(&taker) {
            return EngineResult::Place(PlaceOrderResult {
                symbol: self.symbol.clone(),
                taker_order_id: taker.order_id,
                outcome,
                trades: Vec::new(),
                updated_makers: Vec::new(),
            });
        }

        let (taker, trades, updated_makers) = self.match_order(taker);
        let taker_order_id = taker.order_id;
        let taker_filled = taker.is_filled();
        let taker_tif = taker.tif;

        self.add_to_orderbook_if_remaining(taker);

        let outcome = match (
            taker_filled,
            trades.is_empty(),
            matches!(taker_tif, TimeInForce::GTC),
        ) {
            (true, _, _) => PlaceOrderOutcome::Filled,
            (false, true, true) => PlaceOrderOutcome::Rested,
            (false, true, false) => {
                PlaceOrderOutcome::Cancelled(CancelledReason::IocRemainingCancelled)
            }
            (false, false, true) => PlaceOrderOutcome::PartiallyFilledAndRested,
            (false, false, false) => PlaceOrderOutcome::PartiallyFilledAndCancelled(
                CancelledReason::IocRemainingCancelled,
            ),
        };

        EngineResult::Place(PlaceOrderResult {
            symbol: self.symbol.clone(),
            taker_order_id,
            outcome,
            trades,
            updated_makers,
        })
    }

    fn match_order(&mut self, mut taker: Order) -> (Order, Vec<TradeResult>, Vec<OrderSnapshot>) {
        let mut trade_results = Vec::new();
        let mut updated_makers = Vec::new();

        while let Some((price, makers)) = self.orderbook.get_best_opposite(&taker.side) {
            if !can_match(&taker, price) {
                self.log_matching_stopped_by_price(&taker, price);
                break;
            }

            let (new_taker, trades, up_makers) = self.match_price_level(taker, makers);
            taker = new_taker;
            trade_results.extend(trades);
            updated_makers.extend(up_makers);

            if taker.is_filled() {
                break;
            }
        }

        (taker, trade_results, updated_makers)
    }

    /// 단일 Price level과 주문 매칭 수행
    fn match_price_level(
        &mut self,
        mut taker: Order,
        makers: VecDeque<Uuid>,
    ) -> (Order, Vec<TradeResult>, Vec<OrderSnapshot>) {
        let mut trade_results = Vec::new();
        let mut updated_makers = Vec::new();

        for maker_id in makers {
            let Some(maker) = self.orderbook.get_order_mut(&maker_id) else {
                error!(symbol = %self.symbol, maker_order_id = %maker_id, "오더북 불일치: 인덱스에 주문 없음");
                continue;
            };
            let Some(maker_price) = maker.price else {
                error!(symbol = %self.symbol, maker_order_id = %maker_id, "잘못된 메이커 주문: LIMIT 주문 가격 없음");
                continue;
            };

            let Some(fill_base) = calc_fill_base(&taker, maker) else {
                error!(symbol = %self.symbol, taker_order_id = %taker.order_id, maker_order_id = %maker.order_id, "체결 수량 계산 실패");
                continue;
            };
            let fill_quote = fill_base * maker_price;

            maker.fill(fill_base, fill_quote);
            taker.fill(fill_base, fill_quote);

            trade_results.push(TradeResult {
                trade_id: Uuid::now_v7(),
                taker_order_id: taker.order_id,
                maker_order_id: maker_id,
                price: maker_price,
                base_qty: fill_base,
                quote_qty: fill_quote,
            });
            updated_makers.push(OrderSnapshot::from(&*maker));

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

            if maker.is_filled() {
                debug!(symbol = %self.symbol, maker_order_id = %maker_id, "메이커 주문 완전 체결 후 오더북 제거");
                self.orderbook.remove_order(maker_id);
            }

            if taker.is_filled() {
                break;
            }
        }

        (taker, trade_results, updated_makers)
    }

    fn add_to_orderbook_if_remaining(&mut self, taker: Order) {
        if taker.is_filled() || !matches!(taker.tif, TimeInForce::GTC) {
            return;
        }

        debug!(
            symbol = %self.symbol,
            order_id = %taker.order_id,
            remaining_base_qty = ?taker.remaining_base_qty(),
            remaining_quote_qty = ?taker.remaining_quote_qty(),
            "오더북 등록"
        );

        self.orderbook.add_order(taker);
    }

    fn validate_before_matching(&self, taker: &Order) -> Option<PlaceOrderOutcome> {
        if matches!(taker.size, OrderSize::Quote(_))
            && !(matches!(taker.order_type, OrderType::Market) && matches!(taker.side, Side::Buy))
        {
            self.log_quote_rejected(taker);
            return Some(PlaceOrderOutcome::Rejected(RejectedReason::InvalidOrder(
                "Quote 주문은 Market Buy만 허용".to_string(),
            )));
        }

        if matches!(taker.tif, TimeInForce::GTC) && taker.order_type != OrderType::Limit {
            self.log_gtc_rejected(taker);
            return Some(PlaceOrderOutcome::Rejected(RejectedReason::InvalidOrder(
                "GTC 주문은 Limit만 허용".to_string(),
            )));
        }

        if matches!(taker.tif, TimeInForce::FOK) && !self.can_fully_fill_fok(taker) {
            self.log_fok_cancelled(taker);
            return Some(PlaceOrderOutcome::Cancelled(
                CancelledReason::FokCannotFullyFill,
            ));
        }

        None
    }

    fn can_fully_fill_fok(&self, taker: &Order) -> bool {
        match taker.size {
            OrderSize::Base(qty) => {
                let price = match taker.order_type {
                    // Market 주문은 price 미존재
                    OrderType::Market => match taker.side {
                        Side::Buy => Decimal::MAX,
                        Side::Sell => Decimal::ZERO,
                    },
                    OrderType::Limit => {
                        let Some(price) = taker.price else {
                            error!(
                                symbol = %self.symbol,
                                order_id = %taker.order_id,
                                "잘못된 FOK LIMIT 주문: 가격 없음"
                            );
                            return false;
                        };
                        price
                    }
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

    fn log_order_received(&self, taker: &Order) {
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
    }

    fn log_gtc_rejected(&self, taker: &Order) {
        warn!(
            symbol = %self.symbol,
            order_id = %taker.order_id,
            order_type = ?taker.order_type,
            tif = ?taker.tif,
            "주문 거부: GTC 주문 LIMIT만 허용"
        );
    }

    fn log_quote_rejected(&self, taker: &Order) {
        warn!(
            symbol = %self.symbol,
            order_id = %taker.order_id,
            side = ?taker.side,
            order_type = ?taker.order_type,
            size = ?taker.size,
            "주문 거부: Quote size는 MARKET BUY만 허용"
        );
    }

    fn log_fok_cancelled(&self, taker: &Order) {
        debug!(
            symbol = %self.symbol,
            order_id = %taker.order_id,
            side = ?taker.side,
            order_type = ?taker.order_type,
            price = ?taker.price,
            size = ?taker.size,
            "주문 취소: FOK 전량 체결 불가"
        );
    }

    fn log_matching_stopped_by_price(&self, taker: &Order, maker_price: Decimal) {
        debug!(
            symbol = %self.symbol,
            order_id = %taker.order_id,
            side = ?taker.side,
            taker_price = ?taker.price,
            best_maker_price = %maker_price,
            "매칭 중단: 가격 조건 불일치"
        );
    }
}

/// taker 주문과 반대 호가 가격 조건 확인
fn can_match(taker: &Order, maker_price: Decimal) -> bool {
    match taker.order_type {
        OrderType::Market => true,
        OrderType::Limit => {
            let Some(taker_price) = taker.price else {
                return false;
            };

            match taker.side {
                Side::Buy => maker_price <= taker_price,
                Side::Sell => maker_price >= taker_price,
            }
        }
    }
}

fn calc_fill_base(taker: &Order, maker: &Order) -> Option<Decimal> {
    let maker_remaining = maker.remaining_base_qty()?;

    match taker.size {
        OrderSize::Base(_) => {
            let taker_remaining = taker.remaining_base_qty()?;
            Some(maker_remaining.min(taker_remaining))
        }
        OrderSize::Quote(_) => {
            let taker_remaining_quote = taker.remaining_quote_qty()?;
            let maker_price = maker.price?;
            Some(maker_remaining.min(taker_remaining_quote / maker_price))
        }
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
        let (_, engine_rx) = channel::unbounded();
        let (result_tx, _) = channel::unbounded();
        let symbol = "BTCUSDT".to_string();
        MatchingEngine::new(symbol, engine_rx, result_tx)
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

    #[test]
    fn ioc_잔존_주문_미등록_테스트() {
        let mut engine = make_engine();
        engine
            .orderbook
            .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 5));

        // IOC Buy 10, Sell 5만 존재 → 5 체결, 잔량 5는 오더북 미등록
        engine.place_order(limit_order(Side::Buy, TimeInForce::IOC, 100, 10));

        assert!(engine.orderbook.get_best_opposite(&Side::Sell).is_none());
    }

    #[test]
    fn 시장가_매도_quote_거부_테스트() {
        let mut engine = make_engine();
        engine
            .orderbook
            .add_order(limit_order(Side::Buy, TimeInForce::GTC, 100, 10));

        // MARKET SELL + Quote → 거부
        engine.place_order(market_quote_order(Side::Sell, 500));

        assert!(engine.orderbook.get_best_opposite(&Side::Sell).is_some());
    }
}

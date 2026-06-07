use std::collections::VecDeque;

use crossbeam::channel::{Receiver, Sender};
use rust_decimal::Decimal;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    domain::order::{Order, OrderSize, OrderType, Side, TimeInForce},
    engine::{
        command::{AmendOrderCommand, EngineCommand},
        orderbook::OrderBook,
        result::{
            AmendOrderOutcome, AmendOrderResult, AmendRejectedReason, CancelOrderOutcome,
            CancelOrderResult, CancelRejectedReason, CancelledReason, EngineResult, OrderSnapshot,
            PlaceOrderOutcome, PlaceOrderResult, RejectedReason, TradeResult,
        },
    },
};

#[cfg(feature = "bench-internals")]
mod bench;

struct MatchResult {
    taker: Order,
    trades: Vec<TradeResult>,
    updated_makers: Vec<OrderSnapshot>,
}

impl MatchResult {
    fn new(taker: Order) -> Self {
        Self {
            taker,
            trades: Vec::new(),
            updated_makers: Vec::new(),
        }
    }
}

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
            let order_id = command.order_id();
            let result = match command {
                EngineCommand::Place(order) => self.place_order(order),
                EngineCommand::Cancel(order_id) => self.cancel_order(order_id),
                EngineCommand::Amend(cmd) => self.amend_order(cmd),
            };

            if let Err(e) = self.result_tx.send(result) {
                error!(symbol = %self.symbol, order_id = %order_id, error = %e, "매칭 결과 전송 오류");
            }
        }
    }

    /// 주문 접수
    fn place_order(&mut self, taker: Order) -> EngineResult {
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

        if let Some(outcome) = self.validate_before_matching(&taker) {
            return EngineResult::Place(PlaceOrderResult {
                symbol: self.symbol.clone(),
                taker_order_id: taker.order_id,
                outcome,
                trades: Vec::new(),
                updated_makers: Vec::new(),
            });
        }

        let MatchResult {
            taker,
            trades,
            updated_makers,
        } = self.match_order(taker);

        let outcome = resolve_place_outcome(taker.is_filled(), !trades.is_empty(), taker.tif);

        let taker_order_id = taker.order_id;
        self.add_to_orderbook_if_remaining(taker);

        EngineResult::Place(PlaceOrderResult {
            symbol: self.symbol.clone(),
            taker_order_id,
            outcome,
            trades,
            updated_makers,
        })
    }

    /// 주문 취소
    fn cancel_order(&mut self, order_id: Uuid) -> EngineResult {
        debug!(symbol = %self.symbol, order_id = %order_id, "주문 취소");

        let Some(mut order) = self.orderbook.remove_order(order_id) else {
            warn!(symbol = %self.symbol, order_id = %order_id, "주문 취소 거부: 주문 찾을 수 없음");
            return EngineResult::Cancel(CancelOrderResult {
                symbol: self.symbol.clone(),
                order_id,
                outcome: CancelOrderOutcome::Rejected(CancelRejectedReason::OrderNotFound),
            });
        };

        // orderbook 내부 주문은 New, PartiallyFilled 상태이어야만함
        order
            .cancel()
            .expect("오더북 불변식 위반: 오더북 내 주문은 취소 가능한 상태여야한다");

        EngineResult::Cancel(CancelOrderResult {
            symbol: self.symbol.clone(),
            order_id,
            outcome: CancelOrderOutcome::Cancelled(OrderSnapshot::from(&order)),
        })
    }

    /// 주문 정정
    /// 수량감소 -> index 직접 변환
    /// 수량증가/가격변경 -> 취소 후 등록
    fn amend_order(&mut self, cmd: AmendOrderCommand) -> EngineResult {
        let Some(current_order) = self.orderbook.get_order_mut(&cmd.order_id) else {
            return EngineResult::Amend(AmendOrderResult {
                symbol: self.symbol.clone(),
                order_id: cmd.order_id,
                outcome: AmendOrderOutcome::Rejected(AmendRejectedReason::OrderNotFound),
            });
        };

        let Ok(amended_order) = current_order.amend(cmd.price, cmd.base_qty) else {
            return EngineResult::Amend(AmendOrderResult {
                symbol: self.symbol.clone(),
                order_id: cmd.order_id,
                outcome: AmendOrderOutcome::Rejected(AmendRejectedReason::AmendNotAllowed),
            });
        };

        let current_price = current_order.price.expect("오더북 불변식 위반: price 없음");
        let current_base_qty = current_order
            .base_qty()
            .expect("오더북 불변식 위반: base_qty 없음");
        let amended_price = amended_order.price.expect("정정 후 LIMIT 주문 price 없음");
        let amended_base_qty = amended_order.base_qty().expect("정정 후 base_qty 없음");

        if amended_order.is_filled() {
            self.orderbook
                .remove_order(cmd.order_id)
                .expect("검증된 주문이 오더북에 없음");

            return EngineResult::Amend(AmendOrderResult {
                symbol: self.symbol.clone(),
                order_id: cmd.order_id,
                outcome: AmendOrderOutcome::Amended(OrderSnapshot::from(&amended_order)),
            });
        }

        // 가격 변동 or 수량 증가시 우선순위 잃음
        if amended_price != current_price || amended_base_qty > current_base_qty {
            let mut cancelled = self
                .orderbook
                .remove_order(cmd.order_id)
                .expect("검증된 주문이 오더북에 없음");

            cancelled
                .cancel()
                .expect("오더북 주문은 취소 가능한 상태여야 함");

            let EngineResult::Place(placed) = self.place_order(amended_order) else {
                unreachable!("place 결과만 반환됨");
            };

            return EngineResult::Amend(AmendOrderResult {
                symbol: self.symbol.clone(),
                order_id: cmd.order_id,
                outcome: AmendOrderOutcome::CancelReplaced {
                    cancelled: OrderSnapshot::from(&cancelled),
                    placed,
                },
            });
        }

        *current_order = amended_order;

        EngineResult::Amend(AmendOrderResult {
            symbol: self.symbol.clone(),
            order_id: cmd.order_id,
            outcome: AmendOrderOutcome::Amended(OrderSnapshot::from(&*current_order)),
        })
    }

    /// 주문 매칭
    fn match_order(&mut self, taker: Order) -> MatchResult {
        let mut result = MatchResult::new(taker);

        while let Some((price, makers)) = self.orderbook.get_best_opposite(&result.taker.side) {
            if !can_match(&result.taker, price) {
                break;
            }

            self.match_price_level(makers, &mut result);

            if result.taker.is_filled() {
                break;
            }
        }

        result
    }

    /// 단일 Price level과 주문 매칭 수행
    fn match_price_level(&mut self, makers: VecDeque<Uuid>, result: &mut MatchResult) {
        for maker_id in makers {
            let Some(maker) = self.orderbook.get_order_mut(&maker_id) else {
                error!(symbol = %self.symbol, maker_order_id = %maker_id, "오더북 불일치: 인덱스에 주문 없음");
                continue;
            };
            let Some(maker_price) = maker.price else {
                error!(symbol = %self.symbol, maker_order_id = %maker_id, "잘못된 메이커 주문: LIMIT 주문 가격 없음");
                continue;
            };

            let Some(fill_base) = calc_fill_base(&result.taker, maker) else {
                error!(symbol = %self.symbol, taker_order_id = %result.taker.order_id, maker_order_id = %maker.order_id, "체결 수량 계산 실패");
                continue;
            };
            let fill_quote = fill_base * maker_price;

            let filled_maker = maker
                .fill(fill_base, fill_quote)
                .expect("오더북 불변식 위반: 메이커 체결 불가");
            let filled_taker = result
                .taker
                .fill(fill_base, fill_quote)
                .expect("오더북 불변식 위반: 테이커 체결 불가");

            *maker = filled_maker;
            result.taker = filled_taker;

            result.trades.push(TradeResult {
                trade_id: Uuid::now_v7(),
                taker_order_id: result.taker.order_id,
                maker_order_id: maker_id,
                price: maker_price,
                base_qty: fill_base,
                quote_qty: fill_quote,
            });
            result.updated_makers.push(OrderSnapshot::from(&*maker));

            info!(
                symbol = %self.symbol,
                taker_order_id = %result.taker.order_id,
                maker_order_id = %maker.order_id,
                price = %maker_price,
                fill_base = %fill_base,
                fill_quote = %fill_quote,
                taker_filled = result.taker.is_filled(),
                maker_filled = maker.is_filled(),
                "주문 체결"
            );

            if maker.is_filled() {
                debug!(symbol = %self.symbol, maker_order_id = %maker_id, "메이커 주문 완전 체결 후 오더북 제거");
                self.orderbook.remove_order(maker_id);
            }

            if result.taker.is_filled() {
                break;
            }
        }
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
            warn!(
                symbol = %self.symbol,
                order_id = %taker.order_id,
                side = ?taker.side,
                order_type = ?taker.order_type,
                size = ?taker.size,
                "주문 거부: Quote size는 MARKET BUY만 허용"
            );
            return Some(PlaceOrderOutcome::Rejected(RejectedReason::InvalidOrder(
                "Quote 주문은 Market Buy만 허용".to_string(),
            )));
        }

        if matches!(taker.tif, TimeInForce::GTC) && taker.order_type != OrderType::Limit {
            warn!(
                symbol = %self.symbol,
                order_id = %taker.order_id,
                order_type = ?taker.order_type,
                tif = ?taker.tif,
                "주문 거부: GTC 주문 LIMIT만 허용"
            );
            return Some(PlaceOrderOutcome::Rejected(RejectedReason::InvalidOrder(
                "GTC 주문은 Limit만 허용".to_string(),
            )));
        }

        if matches!(taker.tif, TimeInForce::FOK) && !self.can_fully_fill_fok(taker) {
            debug!(
                symbol = %self.symbol,
                order_id = %taker.order_id,
                side = ?taker.side,
                order_type = ?taker.order_type,
                price = ?taker.price,
                size = ?taker.size,
                "주문 취소: FOK 전량 체결 불가"
            );
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
                        Side::Sell => Decimal::MIN,
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

/// taker.size에 따라 체결 수량을 계산한다
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

/// 매칭 결과에 따른 주문 결과를 생성한다
fn resolve_place_outcome(
    taker_filled: bool,
    has_trades: bool,
    tif: TimeInForce,
) -> PlaceOrderOutcome {
    match (taker_filled, has_trades, matches!(tif, TimeInForce::GTC)) {
        (true, _, _) => PlaceOrderOutcome::Filled,
        (false, false, true) => PlaceOrderOutcome::Rested,
        (false, false, false) => {
            PlaceOrderOutcome::Cancelled(CancelledReason::IocRemainingCancelled)
        }
        (false, true, true) => PlaceOrderOutcome::PartiallyFilledAndRested,
        (false, true, false) => {
            PlaceOrderOutcome::PartiallyFilledAndCancelled(CancelledReason::IocRemainingCancelled)
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
    use crate::engine::command::AmendOrderCommand;
    use crate::engine::result::{
        AmendOrderOutcome, AmendRejectedReason, CancelOrderOutcome, CancelRejectedReason,
        EngineResult,
    };

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

    #[test]
    fn 주문_취소_성공_테스트() {
        let mut engine = make_engine();
        let order = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
        let order_id = order.order_id;
        engine.orderbook.add_order(order);

        let result = engine.cancel_order(order_id);

        let EngineResult::Cancel(r) = result else {
            panic!("Cancel 결과여야 함");
        };
        let CancelOrderOutcome::Cancelled(snapshot) = r.outcome else {
            panic!("취소 성공이어야 함");
        };
        assert_eq!(snapshot.order_id, order_id);
        assert_eq!(snapshot.status, OrderStatus::Cancelled);
        assert_eq!(snapshot.executed_base_qty, Decimal::ZERO);
        assert_eq!(snapshot.remaining_base_qty, Some(Decimal::new(10, 0)));
        assert!(engine.orderbook.get_best_opposite(&Side::Sell).is_none());
    }

    #[test]
    fn 부분_체결_후_취소_테스트() {
        let mut engine = make_engine();
        engine
            .orderbook
            .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 5));

        let buy = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
        let buy_id = buy.order_id;
        engine.place_order(buy);

        let result = engine.cancel_order(buy_id);

        let EngineResult::Cancel(r) = result else {
            panic!("Cancel 결과여야 함");
        };
        let CancelOrderOutcome::Cancelled(snapshot) = r.outcome else {
            panic!("취소 성공이어야 함");
        };
        assert_eq!(snapshot.status, OrderStatus::Cancelled);
        assert_eq!(snapshot.executed_base_qty, Decimal::new(5, 0));
        assert_eq!(snapshot.remaining_base_qty, Some(Decimal::new(5, 0)));
        assert!(engine.orderbook.get_best_opposite(&Side::Sell).is_none());
    }

    #[test]
    fn 존재하지_않는_주문_취소_테스트() {
        let mut engine = make_engine();

        let result = engine.cancel_order(Uuid::now_v7());

        let EngineResult::Cancel(r) = result else {
            panic!("Cancel 결과여야 함");
        };
        assert!(matches!(
            r.outcome,
            CancelOrderOutcome::Rejected(CancelRejectedReason::OrderNotFound)
        ));
    }

    #[test]
    fn 존재하지_않는_주문_정정_테스트() {
        let mut engine = make_engine();

        let result = engine.amend_order(AmendOrderCommand {
            order_id: Uuid::now_v7(),
            price: Some(Decimal::new(100, 0)),
            base_qty: Some(Decimal::new(5, 0)),
        });

        let EngineResult::Amend(r) = result else {
            panic!("Amend 결과여야 함");
        };
        assert!(matches!(
            r.outcome,
            AmendOrderOutcome::Rejected(AmendRejectedReason::OrderNotFound)
        ));
    }

    #[test]
    fn 정정_불가_주문_정정_테스트() {
        let mut engine = make_engine();
        let order = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
        let order_id = order.order_id;
        engine.orderbook.add_order(order);

        // price, qty 모두 None → 변경 없음 → AmendNotAllowed
        let result = engine.amend_order(AmendOrderCommand {
            order_id,
            price: None,
            base_qty: None,
        });

        let EngineResult::Amend(r) = result else {
            panic!("Amend 결과여야 함");
        };
        assert!(matches!(
            r.outcome,
            AmendOrderOutcome::Rejected(AmendRejectedReason::AmendNotAllowed)
        ));
    }

    #[test]
    fn 수량_감소_정정_테스트() {
        let mut engine = make_engine();
        let order = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
        let order_id = order.order_id;
        engine.orderbook.add_order(order);

        let result = engine.amend_order(AmendOrderCommand {
            order_id,
            price: Some(Decimal::new(100, 0)),
            base_qty: Some(Decimal::new(5, 0)),
        });

        let EngineResult::Amend(r) = result else {
            panic!("Amend 결과여야 함");
        };
        let AmendOrderOutcome::Amended(snapshot) = r.outcome else {
            panic!("Amended여야 함");
        };
        assert_eq!(snapshot.remaining_base_qty, Some(Decimal::new(5, 0)));
        // 인플레이스 정정 → 같은 order_id로 오더북에 남아있어야 함
        assert!(engine.orderbook.get_order_mut(&order_id).is_some());
    }

    #[test]
    fn 정정_후_전량_체결_제거_테스트() {
        let mut engine = make_engine();
        // SELL 5 → BUY 10 부분 체결: executed=5, remaining=5
        engine
            .orderbook
            .add_order(limit_order(Side::Sell, TimeInForce::GTC, 100, 5));
        let buy = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
        let buy_id = buy.order_id;
        engine.place_order(buy);

        // new_qty=5 == executed_qty=5 → is_filled() → Amended + 오더북 제거
        let result = engine.amend_order(AmendOrderCommand {
            order_id: buy_id,
            price: Some(Decimal::new(100, 0)),
            base_qty: Some(Decimal::new(5, 0)),
        });

        let EngineResult::Amend(r) = result else {
            panic!("Amend 결과여야 함");
        };
        assert!(matches!(r.outcome, AmendOrderOutcome::Amended(_)));
        assert!(engine.orderbook.get_order_mut(&buy_id).is_none());
    }

    #[test]
    fn 가격_변경_정정_테스트() {
        let mut engine = make_engine();
        let order = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
        let order_id = order.order_id;
        engine.orderbook.add_order(order);

        let result = engine.amend_order(AmendOrderCommand {
            order_id,
            price: Some(Decimal::new(101, 0)), // 가격 변경 → 우선순위 소실
            base_qty: Some(Decimal::new(10, 0)),
        });

        let EngineResult::Amend(r) = result else {
            panic!("Amend 결과여야 함");
        };
        assert!(matches!(
            r.outcome,
            AmendOrderOutcome::CancelReplaced { .. }
        ));
    }

    #[test]
    fn 수량_증가_정정_테스트() {
        let mut engine = make_engine();
        let order = limit_order(Side::Buy, TimeInForce::GTC, 100, 10);
        let order_id = order.order_id;
        engine.orderbook.add_order(order);

        let result = engine.amend_order(AmendOrderCommand {
            order_id,
            price: Some(Decimal::new(100, 0)),
            base_qty: Some(Decimal::new(15, 0)), // 수량 증가 → 우선순위 소실
        });

        let EngineResult::Amend(r) = result else {
            panic!("Amend 결과여야 함");
        };
        assert!(matches!(
            r.outcome,
            AmendOrderOutcome::CancelReplaced { .. }
        ));
    }
}

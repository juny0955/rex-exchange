use rust_decimal::Decimal;
use tracing::{debug, error, warn};

use crate::{
    domain::order::{Order, OrderSize, OrderType, Side, TimeInForce},
    engine::result::{
        CancelledReason, EngineResultBody, PlaceOrderOutcome, PlaceOrderResult, RejectedReason,
    },
};

use super::{MatchingEngine, matching::MatchResult};

impl MatchingEngine {
    /// 주문 접수
    pub(super) fn place_order(&mut self, taker: Order) -> EngineResultBody {
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
            return EngineResultBody::Place(PlaceOrderResult {
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
        if let Some(outcome) = self.add_to_orderbook_if_remaining(taker) {
            return EngineResultBody::Place(PlaceOrderResult {
                symbol: self.symbol.clone(),
                taker_order_id,
                outcome,
                trades,
                updated_makers,
            });
        }

        EngineResultBody::Place(PlaceOrderResult {
            symbol: self.symbol.clone(),
            taker_order_id,
            outcome,
            trades,
            updated_makers,
        })
    }

    fn add_to_orderbook_if_remaining(&mut self, taker: Order) -> Option<PlaceOrderOutcome> {
        if taker.is_filled() || !matches!(taker.tif, TimeInForce::GTC) {
            return None;
        }

        debug!(
            symbol = %self.symbol,
            order_id = %taker.order_id,
            remaining_base_qty = ?taker.remaining_base_qty(),
            remaining_quote_qty = ?taker.remaining_quote_qty(),
            "오더북 등록"
        );

        if self.orderbook.add_order(taker).is_err() {
            return Some(PlaceOrderOutcome::Rejected(
                RejectedReason::DuplicateOrderId,
            ));
        }

        None
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

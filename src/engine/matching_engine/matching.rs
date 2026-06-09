use rust_decimal::Decimal;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    domain::order::{Order, OrderSize, OrderType, Side},
    engine::result::{OrderSnapshot, TradeResult},
};

use super::MatchingEngine;

pub(super) struct MatchResult {
    pub(super) taker: Order,
    pub(super) trades: Vec<TradeResult>,
    pub(super) updated_makers: Vec<OrderSnapshot>,
}

impl MatchResult {
    pub(super) fn new(taker: Order) -> Self {
        Self {
            taker,
            trades: Vec::new(),
            updated_makers: Vec::new(),
        }
    }
}

impl MatchingEngine {
    /// 주문 매칭
    pub(super) fn match_order(&mut self, taker: Order) -> MatchResult {
        let mut result = MatchResult::new(taker);
        let taker_side = result.taker.side;

        while let Some(price) = self.orderbook.best_opposite_price(&taker_side) {
            if !can_match(&result.taker, price) {
                break;
            }

            let Some(maker_id) = self.orderbook.peek_best_opposite_maker(&taker_side) else {
                break;
            };

            let Some(maker) = self.orderbook.get_order_mut(&maker_id) else {
                error!(symbol = %self.symbol, maker_order_id = %maker_id, "오더북 불일치: 인덱스에 주문 없음");
                self.orderbook.pop_best_opposite_maker(&taker_side);
                continue;
            };

            let Some(maker_price) = maker.price else {
                error!(symbol = %self.symbol, maker_order_id = %maker_id, "잘못된 메이커 주문: LIMIT 주문 가격 없음");
                self.orderbook.pop_best_opposite_maker(&taker_side);
                continue;
            };

            let Some(fill_base) = calc_fill_base(&result.taker, maker) else {
                error!(symbol = %self.symbol, taker_order_id = %result.taker.order_id, maker_order_id = %maker_id, "체결 수량 계산 실패");
                self.orderbook.pop_best_opposite_maker(&taker_side);
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

            let maker_filled = maker.is_filled();
            result.updated_makers.push(OrderSnapshot::from(&*maker)); // ← maker 마지막 사용

            result.trades.push(TradeResult {
                trade_id: Uuid::now_v7(),
                taker_order_id: result.taker.order_id,
                maker_order_id: maker_id,
                price: maker_price,
                base_qty: fill_base,
                quote_qty: fill_quote,
            });

            info!(
                symbol = %self.symbol,
                taker_order_id = %result.taker.order_id,
                maker_order_id = %maker_id,
                price = %maker_price,
                fill_base = %fill_base,
                fill_quote = %fill_quote,
                taker_filled = result.taker.is_filled(),
                maker_filled = maker_filled,
                "주문 체결"
            );

            if maker_filled {
                debug!(symbol = %self.symbol, maker_order_id = %maker_id, "메이커 주문 완전 체결 후 오더북 제거");
                self.orderbook.pop_best_opposite_maker(&taker_side);
            }

            if result.taker.is_filled() {
                break;
            }
        }

        result
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

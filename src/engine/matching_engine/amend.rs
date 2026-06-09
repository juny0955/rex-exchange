use tracing::{debug, warn};
use uuid::Uuid;

use crate::engine::{
    command::AmendOrderCommand,
    result::{
        AmendOrderOutcome, AmendOrderResult, AmendRejectedReason, CancelOrderOutcome,
        CancelOrderResult, CancelRejectedReason, EngineResult, OrderSnapshot,
    },
};

use super::MatchingEngine;

impl MatchingEngine {
    /// 주문 취소
    pub(super) fn cancel_order(&mut self, order_id: Uuid) -> EngineResult {
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
    pub(super) fn amend_order(&mut self, cmd: AmendOrderCommand) -> EngineResult {
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
}

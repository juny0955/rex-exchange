use uuid::Uuid;

use crate::{
    domain::order::Order,
    engine::{command::AmendOrderCommand, result::EngineResult},
};

use super::MatchingEngine;

impl MatchingEngine {
    pub fn bench_place_order(&mut self, order: Order) -> EngineResult {
        self.place_order(order)
    }

    pub fn bench_cancel_order(&mut self, order_id: Uuid) -> EngineResult {
        self.cancel_order(order_id)
    }

    pub fn bench_amend_order(&mut self, cmd: AmendOrderCommand) -> EngineResult {
        self.amend_order(cmd)
    }

    pub fn bench_seed_order(&mut self, order: Order) {
        self.orderbook.add_order(order);
    }
}

//! `OrderBook` benchmark scenario 묶음.
//!
//! 각 모듈은 하나의 Criterion group만 책임져 benchmark 이름과 해석 포인트가 섞이지 않게 한다.

mod fixtures;

pub mod add_order;
pub mod can_fully_fill_base;
pub mod can_fully_fill_quote;
pub mod remove_order;

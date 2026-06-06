//! `MatchingEngine` benchmark scenario 묶음.
//!
//! 각 모듈은 place/cancel/amend 중 하나의 엔진 명령 계열만 측정한다.

mod fixtures;

pub mod amend_order;
pub mod cancel_order;
pub mod place_order;

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::order::Order;

#[derive(Debug, Clone)]
pub struct AmendOrderCommand {
    pub order_id: Uuid,
    pub price: Option<Decimal>,
    pub base_qty: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub enum EngineCommand {
    Place(Order),
    Cancel(Uuid),
    Amend(AmendOrderCommand),
}

impl EngineCommand {
    pub fn order_id(&self) -> Uuid {
        match self {
            EngineCommand::Place(o) => o.order_id,
            EngineCommand::Cancel(order_id) => *order_id,
            EngineCommand::Amend(c) => c.order_id,
        }
    }
}

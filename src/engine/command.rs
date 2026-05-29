use uuid::Uuid;

use crate::domain::order::Order;

#[derive(Debug)]
pub enum EngineCommand {
    Place(Order),
}

impl EngineCommand {
    pub fn order_id(&self) -> Uuid {
        match self {
            EngineCommand::Place(o) => o.order_id,
        }
    }
}

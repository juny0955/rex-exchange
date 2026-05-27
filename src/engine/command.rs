use crate::domain::order::Order;

#[derive(Debug)]
pub enum EngineCommand {
    Place(Order),
}

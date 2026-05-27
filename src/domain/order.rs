use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub order_id: Uuid,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Decimal,
    pub quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub created_at: u64,
}

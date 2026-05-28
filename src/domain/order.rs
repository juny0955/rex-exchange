use chrono::{DateTime, Utc};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
    GTC,
    IOC,
    FOK,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub order_id: Uuid,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub tif: TimeInForce,
    pub price: Decimal,
    pub quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Order {
    pub fn fill(&mut self, fill_qty: Decimal) {
        assert!(fill_qty > Decimal::ZERO);
        assert!(fill_qty <= self.remaining_quantity);
        assert!(!self.is_complated());

        self.remaining_quantity -= fill_qty;

        self.status = if self.remaining_quantity.is_zero() {
            OrderStatus::Filled
        } else {
            OrderStatus::PartiallyFilled
        };
        self.updated_at = Utc::now();
    }

    pub fn is_filled(&self) -> bool {
        self.status == OrderStatus::Filled
    }

    fn is_complated(&self) -> bool {
        !matches!(self.status, OrderStatus::PartiallyFilled | OrderStatus::New)
    }
}

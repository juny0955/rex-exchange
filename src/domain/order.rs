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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSize {
    Base(Decimal),
    Quote(Decimal),
}

#[derive(Debug, Clone)]
pub struct Order {
    pub order_id: Uuid,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub tif: TimeInForce,
    pub price: Option<Decimal>,
    pub size: OrderSize,
    pub executed_base_qty: Decimal,
    pub executed_quote_qty: Decimal,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Order {
    pub fn fill(&mut self, base_qty: Decimal, quote_qty: Decimal) {
        assert!(base_qty > Decimal::ZERO);
        assert!(quote_qty > Decimal::ZERO);
        assert!(!self.is_completed());

        match self.size {
            OrderSize::Base(qty) => assert!(self.executed_base_qty + base_qty <= qty),
            OrderSize::Quote(quote_qty) => {
                assert!(self.executed_quote_qty + quote_qty <= quote_qty)
            }
        }

        self.executed_base_qty += base_qty;
        self.executed_quote_qty += quote_qty;

        self.status = if self.is_filled_by_size() {
            OrderStatus::Filled
        } else {
            OrderStatus::PartiallyFilled
        };

        self.updated_at = Utc::now();
    }

    pub fn remaining_base_qty(&self) -> Option<Decimal> {
        match self.size {
            OrderSize::Base(qty) => Some(qty - self.executed_base_qty),
            OrderSize::Quote(_) => None,
        }
    }

    pub fn remaining_quote_qty(&self) -> Option<Decimal> {
        match self.size {
            OrderSize::Quote(quote_qty) => Some(quote_qty - self.executed_quote_qty),
            OrderSize::Base(_) => None,
        }
    }

    pub fn is_filled(&self) -> bool {
        self.status == OrderStatus::Filled
    }

    fn is_filled_by_size(&self) -> bool {
        match self.size {
            OrderSize::Base(qty) => self.executed_base_qty >= qty,
            OrderSize::Quote(quote_qty) => self.executed_quote_qty >= quote_qty,
        }
    }

    fn is_completed(&self) -> bool {
        !matches!(self.status, OrderStatus::PartiallyFilled | OrderStatus::New)
    }
}

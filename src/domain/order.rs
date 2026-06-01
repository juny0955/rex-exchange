use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::error::OrderError;

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
    /// 주문 체결
    /// 불변으로 새로운 Order 반환
    pub fn fill(&self, base_qty: Decimal, quote_qty: Decimal) -> Result<Order, OrderError> {
        if base_qty <= Decimal::ZERO {
            return Err(OrderError::InvalidFillBaseQty);
        }
        if quote_qty <= Decimal::ZERO {
            return Err(OrderError::InvalidFillQuoteQty);
        }
        if self.is_completed() {
            return Err(OrderError::AlreadyCompleted);
        }

        match self.size {
            OrderSize::Base(qty) => {
                if self.executed_base_qty + base_qty > qty {
                    return Err(OrderError::OverFilled);
                }
            }
            OrderSize::Quote(order_quote_qty) => {
                if self.executed_quote_qty + quote_qty > order_quote_qty {
                    return Err(OrderError::OverFilled);
                }
            }
        }

        let mut new_order = self.clone();

        new_order.executed_base_qty += base_qty;
        new_order.executed_quote_qty += quote_qty;

        new_order.status = if new_order.is_filled_by_size() {
            OrderStatus::Filled
        } else {
            OrderStatus::PartiallyFilled
        };

        new_order.updated_at = Utc::now();
        Ok(new_order)
    }

    pub fn cancel(&mut self) -> Result<(), OrderError> {
        if matches!(self.status, OrderStatus::Canceled) {
            return Err(OrderError::AlreadyCancelled);
        }
        if self.is_completed() {
            return Err(OrderError::AlreadyCompleted);
        }

        self.status = OrderStatus::Canceled;
        self.updated_at = Utc::now();

        Ok(())
    }

    /// 주문 정정
    /// 불변으로 새로운 Order 반환
    pub fn amend(
        &self,
        new_price: Option<Decimal>,
        new_base_qty: Option<Decimal>,
    ) -> Result<Order, OrderError> {
        if self.is_completed() {
            return Err(OrderError::AlreadyCompleted);
        }

        let current_price = self.price.ok_or(OrderError::AmendNotAllowed)?;
        let current_base_qty = match self.size {
            OrderSize::Base(qty) => qty,
            OrderSize::Quote(_) => return Err(OrderError::AmendNotAllowed),
        };

        let next_price = new_price.unwrap_or(current_price);
        let next_base_qty = new_base_qty.unwrap_or(current_base_qty);

        if next_base_qty < self.executed_base_qty {
            return Err(OrderError::AmendQtyBelowExecuted);
        }

        let mut next = self.clone();
        next.price = Some(next_price);
        next.size = OrderSize::Base(next_base_qty);

        next.status = if next.executed_base_qty == Decimal::ZERO {
            OrderStatus::New
        } else if next.is_filled_by_size() {
            OrderStatus::Filled
        } else {
            OrderStatus::PartiallyFilled
        };
        next.updated_at = Utc::now();

        Ok(next)
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_base_order(qty: Decimal) -> Order {
        Order {
            order_id: Uuid::now_v7(),
            symbol: "BTC/USDT".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            tif: TimeInForce::GTC,
            price: Some(Decimal::new(100, 0)),
            size: OrderSize::Base(qty),
            executed_base_qty: Decimal::ZERO,
            executed_quote_qty: Decimal::ZERO,
            status: OrderStatus::New,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_quote_order(quote: Decimal) -> Order {
        Order {
            order_id: Uuid::now_v7(),
            symbol: "BTC/USDT".to_string(),
            side: Side::Buy,
            order_type: OrderType::Market,
            tif: TimeInForce::IOC,
            price: None,
            size: OrderSize::Quote(quote),
            executed_base_qty: Decimal::ZERO,
            executed_quote_qty: Decimal::ZERO,
            status: OrderStatus::New,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn base_주문_분할_체결_테스트() {
        let order = make_base_order(Decimal::new(10, 0));

        let order = order
            .fill(Decimal::new(4, 0), Decimal::new(400, 0))
            .unwrap();
        assert_eq!(order.executed_base_qty, Decimal::new(4, 0));
        assert_eq!(order.executed_quote_qty, Decimal::new(400, 0));
        assert_eq!(order.status, OrderStatus::PartiallyFilled);

        let order = order
            .fill(Decimal::new(6, 0), Decimal::new(600, 0))
            .unwrap();
        assert_eq!(order.executed_base_qty, Decimal::new(10, 0));
        assert_eq!(order.executed_quote_qty, Decimal::new(1000, 0));
        assert_eq!(order.status, OrderStatus::Filled);
    }

    #[test]
    fn quote_주문_분할_체결_테스트() {
        let order = make_quote_order(Decimal::new(1000, 0));

        let order = order
            .fill(Decimal::new(4, 0), Decimal::new(400, 0))
            .unwrap();
        assert_eq!(order.executed_quote_qty, Decimal::new(400, 0));
        assert_eq!(order.status, OrderStatus::PartiallyFilled);

        let order = order
            .fill(Decimal::new(6, 0), Decimal::new(600, 0))
            .unwrap();
        assert_eq!(order.executed_quote_qty, Decimal::new(1000, 0));
        assert_eq!(order.status, OrderStatus::Filled);
    }

    #[test]
    fn base_주문_초과_체결_테스트() {
        let order = make_base_order(Decimal::new(10, 0));
        let result = order.fill(Decimal::new(11, 0), Decimal::new(1100, 0));
        assert!(matches!(result, Err(OrderError::OverFilled)));
    }

    #[test]
    fn quote_주문_초과_체결_테스트() {
        let order = make_quote_order(Decimal::new(1000, 0));
        let result = order.fill(Decimal::new(11, 0), Decimal::new(1100, 0));
        assert!(matches!(result, Err(OrderError::OverFilled)));
    }

    #[test]
    fn new_주문_취소_테스트() {
        let mut order = make_base_order(Decimal::new(10, 0));
        assert_eq!(order.cancel(), Ok(()));
        assert_eq!(order.status, OrderStatus::Canceled);
    }

    #[test]
    fn 부분체결_주문_취소_테스트() {
        let order = make_base_order(Decimal::new(10, 0));
        let mut order = order
            .fill(Decimal::new(4, 0), Decimal::new(400, 0))
            .unwrap();
        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        assert_eq!(order.cancel(), Ok(()));
        assert_eq!(order.status, OrderStatus::Canceled);
    }

    #[test]
    fn 이미_취소된_주문_취소_테스트() {
        let mut order = make_base_order(Decimal::new(10, 0));
        order.cancel().unwrap();
        assert_eq!(order.cancel(), Err(OrderError::AlreadyCancelled));
    }

    #[test]
    fn 완전체결_주문_취소_테스트() {
        let order = make_base_order(Decimal::new(10, 0));
        let mut order = order
            .fill(Decimal::new(10, 0), Decimal::new(1000, 0))
            .unwrap();
        assert_eq!(order.status, OrderStatus::Filled);
        assert_eq!(order.cancel(), Err(OrderError::AlreadyCompleted));
    }

    #[test]
    fn 주문_정정_가격_수량_변경_테스트() {
        let order = make_base_order(Decimal::new(10, 0));
        let amended = order
            .amend(Some(Decimal::new(200, 0)), Some(Decimal::new(20, 0)))
            .unwrap();
        assert_eq!(amended.price, Some(Decimal::new(200, 0)));
        assert_eq!(amended.size, OrderSize::Base(Decimal::new(20, 0)));
        assert_eq!(amended.status, OrderStatus::New);
    }

    #[test]
    fn 부분체결_주문_정정_테스트() {
        let order = make_base_order(Decimal::new(10, 0));
        let order = order
            .fill(Decimal::new(4, 0), Decimal::new(400, 0))
            .unwrap();
        let amended = order
            .amend(None, Some(Decimal::new(8, 0)))
            .unwrap();
        assert_eq!(amended.size, OrderSize::Base(Decimal::new(8, 0)));
        assert_eq!(amended.status, OrderStatus::PartiallyFilled);
    }

    #[test]
    fn quote_주문_정정_불허_테스트() {
        let order = make_quote_order(Decimal::new(1000, 0));
        let result = order.amend(None, Some(Decimal::new(5, 0)));
        assert!(matches!(result, Err(OrderError::AmendNotAllowed)));
    }

    #[test]
    fn 완전체결_주문_정정_불허_테스트() {
        let order = make_base_order(Decimal::new(10, 0));
        let order = order
            .fill(Decimal::new(10, 0), Decimal::new(1000, 0))
            .unwrap();
        let result = order.amend(None, Some(Decimal::new(20, 0)));
        assert!(matches!(result, Err(OrderError::AlreadyCompleted)));
    }

    #[test]
    fn 체결수량_미만_정정_불허_테스트() {
        let order = make_base_order(Decimal::new(10, 0));
        let order = order
            .fill(Decimal::new(5, 0), Decimal::new(500, 0))
            .unwrap();
        let result = order.amend(None, Some(Decimal::new(3, 0)));
        assert!(matches!(result, Err(OrderError::AmendQtyBelowExecuted)));
    }
}

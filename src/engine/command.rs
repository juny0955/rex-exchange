use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::order::Order;

#[derive(Debug, Clone)]
pub struct PlaceOrderCommand {
    pub command_id: Uuid,
    pub order: Order,
}

#[derive(Debug, Clone)]
pub struct CancelOrderCommand {
    pub command_id: Uuid,
    pub order_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct AmendOrderCommand {
    pub command_id: Uuid,
    pub order_id: Uuid,
    pub price: Option<Decimal>,
    pub base_qty: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub enum EngineCommand {
    Place(PlaceOrderCommand),
    Cancel(CancelOrderCommand),
    Amend(AmendOrderCommand),
}

impl EngineCommand {
    pub fn new_place(command_id: Uuid, order: Order) -> Self {
        Self::Place(PlaceOrderCommand { command_id, order })
    }

    pub fn generated_place(order: Order) -> Self {
        Self::new_place(Uuid::now_v7(), order)
    }

    pub fn new_cancel(command_id: Uuid, order_id: Uuid) -> Self {
        Self::Cancel(CancelOrderCommand {
            command_id,
            order_id,
        })
    }

    pub fn generated_cancel(order_id: Uuid) -> Self {
        Self::new_cancel(Uuid::now_v7(), order_id)
    }

    pub fn new_amend(
        command_id: Uuid,
        order_id: Uuid,
        price: Option<Decimal>,
        base_qty: Option<Decimal>,
    ) -> Self {
        Self::Amend(AmendOrderCommand {
            command_id,
            order_id,
            price,
            base_qty,
        })
    }

    pub fn command_id(&self) -> Uuid {
        match self {
            EngineCommand::Place(c) => c.command_id,
            EngineCommand::Cancel(c) => c.command_id,
            EngineCommand::Amend(c) => c.command_id,
        }
    }

    pub fn order_id(&self) -> Uuid {
        match self {
            EngineCommand::Place(c) => c.order.order_id,
            EngineCommand::Cancel(c) => c.order_id,
            EngineCommand::Amend(c) => c.order_id,
        }
    }
}

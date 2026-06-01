#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderError {
    InvalidFillBaseQty,
    InvalidFillQuoteQty,
    AlreadyCompleted,
    OverFilled,
    AlreadyCancelled,
    AmendNotAllowed,
    AmendQtyBelowExecuted,
}

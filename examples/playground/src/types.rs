#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefundRequest {
    pub order_id: String,
    pub amount_cents: u64,
    pub purchase_age_days: u16,
    pub already_refunded: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefundPolicy {
    pub refund_window_days: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefundDecision {
    Approved(RefundAuthorization),
    Rejected(RefundRejection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefundAuthorization {
    pub order_id: String,
    pub amount_cents: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefundRejection {
    pub stage: RejectionStage,
    pub reasons: Vec<RejectionReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionStage {
    Validation,
    Eligibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionReason {
    MissingOrderId,
    ZeroAmount,
    MissingReason,
    OutsideRefundWindow,
    AlreadyRefunded,
}

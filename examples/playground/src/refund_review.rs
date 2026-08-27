use contour::contour;

use crate::types::{
    RefundAuthorization, RefundDecision, RefundPolicy, RefundRejection, RefundRequest,
    RejectionReason, RejectionStage,
};

/// Review a refund request with a Contour graph.
///
/// `#[contour]` lowers the flat graph to executable Rust control flow.
#[contour]
pub fn review_refund(request: RefundRequest, policy: RefundPolicy) -> RefundDecision {
    #[question("Is the refund request valid?")]
    |&request| -> (valid, invalid) {
        !request.order_id.trim().is_empty()
            && request.amount_cents > 0
            && !request.reason.trim().is_empty()
    };

    #[question("Is the refund eligible?")]
    |valid, &request, &policy| -> (eligible, ineligible) {
        request.purchase_age_days <= policy.refund_window_days && !request.already_refunded
    };

    #[action("Authorize the refund.")]
    |eligible, &request| -> approved {
        RefundDecision::Approved(RefundAuthorization {
            order_id: request.order_id.clone(),
            amount_cents: request.amount_cents,
        })
    };

    #[action("Reject an ineligible refund.")]
    |ineligible, &request, &policy| -> rejected_ineligible {
        let mut reasons = Vec::new();

        if request.purchase_age_days > policy.refund_window_days {
            reasons.push(RejectionReason::OutsideRefundWindow);
        }
        if request.already_refunded {
            reasons.push(RejectionReason::AlreadyRefunded);
        }

        RefundDecision::Rejected(RefundRejection {
            stage: RejectionStage::Eligibility,
            reasons,
        })
    };

    #[action("Reject an invalid refund request.")]
    |invalid, &request| -> rejected_invalid {
        let mut reasons = Vec::new();

        if request.order_id.trim().is_empty() {
            reasons.push(RejectionReason::MissingOrderId);
        }
        if request.amount_cents == 0 {
            reasons.push(RejectionReason::ZeroAmount);
        }
        if request.reason.trim().is_empty() {
            reasons.push(RejectionReason::MissingReason);
        }

        RefundDecision::Rejected(RefundRejection {
            stage: RejectionStage::Validation,
            reasons,
        })
    };
}

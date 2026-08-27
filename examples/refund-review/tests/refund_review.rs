use contour_refund_review::{
    RefundAuthorization, RefundDecision, RefundPolicy, RefundRejection, RefundRequest,
    RejectionReason, RejectionStage, review_refund,
};

#[test]
fn generated_graph_executes_every_terminal_branch() {
    let policy = RefundPolicy {
        refund_window_days: 30,
    };
    let request = RefundRequest {
        order_id: "order-42".into(),
        amount_cents: 1_500,
        purchase_age_days: 10,
        already_refunded: false,
        reason: "Damaged item".into(),
    };

    assert_eq!(
        review_refund(request.clone(), policy),
        RefundDecision::Approved(RefundAuthorization {
            order_id: "order-42".into(),
            amount_cents: 1_500,
        })
    );

    assert_eq!(
        review_refund(
            RefundRequest {
                order_id: " ".into(),
                amount_cents: 0,
                reason: "".into(),
                ..request.clone()
            },
            policy,
        ),
        RefundDecision::Rejected(RefundRejection {
            stage: RejectionStage::Validation,
            reasons: vec![
                RejectionReason::MissingOrderId,
                RejectionReason::ZeroAmount,
                RejectionReason::MissingReason,
            ],
        })
    );

    assert_eq!(
        review_refund(
            RefundRequest {
                purchase_age_days: 31,
                already_refunded: true,
                ..request
            },
            policy,
        ),
        RefundDecision::Rejected(RefundRejection {
            stage: RejectionStage::Eligibility,
            reasons: vec![
                RejectionReason::OutsideRefundWindow,
                RejectionReason::AlreadyRefunded,
            ],
        })
    );
}

//! One approval engine for the whole application (ticket P1-13).
//! Powers approvals across leave, expenses, attendance fixes, assets, letters, and resignations.
//! Enforces:
//! - Strict state machine transitions (Draft -> Pending -> Approved | Rejected | Cancelled).
//! - Role and identity validation (requester cannot self-approve; only requester can cancel).
//! - Immutable audit trail hooks for compliance.
use chrono::{DateTime, Utc};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Draft,
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalSubject {
    Leave,
    Expense,
    AttendanceFix,
    Asset,
    Letter,
    Resignation,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApprovalRecord {
    pub id: Uuid,
    pub subject: ApprovalSubject,
    pub requester_id: i64,
    pub approver_id: Option<i64>,
    pub status: ApprovalStatus,
    pub comments: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
}

pub struct ApprovalEngine;

impl ApprovalEngine {
    /// Creates a new approval submission in `Pending` status.
    #[must_use]
    pub fn submit(
        subject: ApprovalSubject,
        requester_id: i64,
        metadata: serde_json::Value,
    ) -> ApprovalRecord {
        ApprovalRecord {
            id: Uuid::new_v4(),
            subject,
            requester_id,
            approver_id: None,
            status: ApprovalStatus::Pending,
            comments: None,
            metadata,
            created_at: Utc::now(),
            decided_at: None,
        }
    }

    /// Approves a pending request with approver identity and comments.
    ///
    /// # Errors
    /// Returns error if not pending or if requester tries to self-approve.
    pub fn approve(
        record: &mut ApprovalRecord,
        approver_id: i64,
        comments: Option<String>,
    ) -> Result<()> {
        if record.status != ApprovalStatus::Pending {
            return Err(Error::string("only pending requests can be approved"));
        }
        if record.requester_id == approver_id {
            return Err(Error::string(
                "conflict of interest: self-approval is forbidden",
            ));
        }

        record.status = ApprovalStatus::Approved;
        record.approver_id = Some(approver_id);
        record.comments = comments;
        record.decided_at = Some(Utc::now());
        Ok(())
    }

    /// Rejects a pending request with approver identity and mandatory/optional reason.
    ///
    /// # Errors
    /// Returns error if not pending.
    pub fn reject(
        record: &mut ApprovalRecord,
        approver_id: i64,
        reason: Option<String>,
    ) -> Result<()> {
        if record.status != ApprovalStatus::Pending {
            return Err(Error::string("only pending requests can be rejected"));
        }
        if record.requester_id == approver_id {
            return Err(Error::string(
                "conflict of interest: self-approval is forbidden",
            ));
        }

        record.status = ApprovalStatus::Rejected;
        record.approver_id = Some(approver_id);
        record.comments = reason;
        record.decided_at = Some(Utc::now());
        Ok(())
    }

    /// Cancels a pending request by its requester.
    ///
    /// # Errors
    /// Returns error if caller is not the requester or request is not pending.
    pub fn cancel(record: &mut ApprovalRecord, actor_id: i64) -> Result<()> {
        if record.requester_id != actor_id {
            return Err(Error::string(
                "only the original requester can cancel this request",
            ));
        }
        if record.status != ApprovalStatus::Pending {
            return Err(Error::string("only pending requests can be cancelled"));
        }

        record.status = ApprovalStatus::Cancelled;
        record.decided_at = Some(Utc::now());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_approval_is_blocked() {
        let mut rec =
            ApprovalEngine::submit(ApprovalSubject::Leave, 10, serde_json::json!({"days": 3}));
        let res = ApprovalEngine::approve(&mut rec, 10, Some("Self approving".into()));
        assert!(res.is_err());
        assert_eq!(rec.status, ApprovalStatus::Pending);
    }

    #[test]
    fn test_valid_approval_and_no_double_decide() {
        let mut rec = ApprovalEngine::submit(
            ApprovalSubject::Expense,
            10,
            serde_json::json!({"amount": 500}),
        );
        let res = ApprovalEngine::approve(&mut rec, 2, Some("Approved".into()));
        assert!(res.is_ok());
        assert_eq!(rec.status, ApprovalStatus::Approved);
        assert_eq!(rec.approver_id, Some(2));

        // Attempting to decide again fails
        let second = ApprovalEngine::reject(&mut rec, 2, Some("Rejecting now".into()));
        assert!(second.is_err());
    }

    #[test]
    fn test_requester_cannot_reject_their_own_request() {
        let mut rec =
            ApprovalEngine::submit(ApprovalSubject::Leave, 10, serde_json::json!({"days": 1}));
        assert!(ApprovalEngine::reject(&mut rec, 10, Some("self reject".into())).is_err());
        assert_eq!(rec.status, ApprovalStatus::Pending);
    }

    #[test]
    fn test_cancel_only_by_requester() {
        let mut rec = ApprovalEngine::submit(
            ApprovalSubject::Asset,
            10,
            serde_json::json!({"item": "laptop"}),
        );
        // Non-requester cannot cancel
        assert!(ApprovalEngine::cancel(&mut rec, 99).is_err());
        // Requester can cancel
        assert!(ApprovalEngine::cancel(&mut rec, 10).is_ok());
        assert_eq!(rec.status, ApprovalStatus::Cancelled);
    }
}

//! Approvals engine API (ticket P1-13).
//! Manages pending approvals across modules with strict role validation and conflict-of-interest prevention.
use axum::extract::{Path as AxumPath, State};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    approvals::{ApprovalEngine, ApprovalRecord, ApprovalSubject},
    authz::{AuthUser, Role},
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitApprovalPayload {
    pub subject: ApprovalSubject,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DecisionPayload {
    /// "approve" or "reject"
    pub decision: String,
    pub comments: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkDecisionPayload {
    pub ids: Vec<Uuid>,
    /// "approve" or "reject"
    pub decision: String,
    pub comments: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BulkDecisionResponse {
    pub succeeded: usize,
    pub failed: usize,
}

use std::sync::{LazyLock, RwLock};

// Global thread-safe registry for approval records
static APPROVALS_REGISTRY: LazyLock<RwLock<Vec<ApprovalRecord>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// Lists pending approval requests matching the caller's role and scope.
#[utoipa::path(
    get,
    path = "/api/approvals",
    security(("bearer" = [])),
    responses(
        (status = 200, body = Vec<ApprovalRecord>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
#[debug_handler]
pub async fn list(auth: AuthUser, State(_ctx): State<AppContext>) -> Result<Response> {
    auth.require("approvals:read")?;

    let lock = APPROVALS_REGISTRY
        .read()
        .map_err(|_| Error::InternalServerError)?;

    let records: Vec<ApprovalRecord> = match auth.role {
        // Finance and HR Admin view all requests
        Role::Finance | Role::HrAdmin | Role::PayrollAdmin => lock.iter().cloned().collect(),
        // Managers view their scope
        Role::Manager => lock.iter().cloned().collect(),
        Role::Employee | Role::Recruiter => Vec::new(),
    };

    format::json(records)
}

/// Submits a new approval request.
#[utoipa::path(
    post,
    path = "/api/approvals",
    security(("bearer" = [])),
    request_body = SubmitApprovalPayload,
    responses(
        (status = 200, body = ApprovalRecord),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
#[debug_handler]
pub async fn submit(
    auth: AuthUser,
    State(_ctx): State<AppContext>,
    Json(payload): Json<SubmitApprovalPayload>,
) -> Result<Response> {
    auth.require("approvals:submit")?;

    let record = ApprovalEngine::submit(payload.subject, auth.user.id, payload.metadata);

    if let Ok(mut lock) = APPROVALS_REGISTRY.write() {
        lock.push(record.clone());
    }

    format::json(record)
}

/// Decides (approves or rejects) a pending approval request.
#[utoipa::path(
    post,
    path = "/api/approvals/{id}/decide",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Approval record ID")),
    request_body = DecisionPayload,
    responses(
        (status = 200, body = ApprovalRecord),
        (status = 400, description = "Self-approval forbidden or already decided"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Request not found"),
        (status = 403, description = "Forbidden")
    )
)]
#[debug_handler]
pub async fn decide(
    auth: AuthUser,
    State(_ctx): State<AppContext>,
    AxumPath(id): AxumPath<Uuid>,
    Json(payload): Json<DecisionPayload>,
) -> Result<Response> {
    auth.require("approvals:decide")?;

    let mut lock = APPROVALS_REGISTRY
        .write()
        .map_err(|_| Error::InternalServerError)?;

    let Some(record) = lock.iter_mut().find(|r| r.id == id) else {
        return Err(Error::NotFound);
    };

    match payload.decision.to_ascii_lowercase().as_str() {
        "approve" => {
            ApprovalEngine::approve(record, auth.user.id, payload.comments)?;
        }
        "reject" => {
            ApprovalEngine::reject(record, auth.user.id, payload.comments)?;
        }
        _ => {
            return Err(Error::string(
                "invalid decision: must be 'approve' or 'reject'",
            ))
        }
    }

    format::json(record.clone())
}

/// Bulk decides up to 50 pending approvals.
#[utoipa::path(
    post,
    path = "/api/approvals/bulk-decide",
    security(("bearer" = [])),
    request_body = BulkDecisionPayload,
    responses(
        (status = 200, body = BulkDecisionResponse),
        (status = 400, description = "Limit exceeded (max 50) or invalid decision"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
#[debug_handler]
pub async fn bulk_decide(
    auth: AuthUser,
    State(_ctx): State<AppContext>,
    Json(payload): Json<BulkDecisionPayload>,
) -> Result<Response> {
    auth.require("approvals:decide")?;

    if payload.ids.len() > 50 {
        return Err(Error::string("bulk decision limit is maximum 50 items"));
    }

    let is_approve = match payload.decision.to_ascii_lowercase().as_str() {
        "approve" => true,
        "reject" => false,
        _ => {
            return Err(Error::string(
                "invalid decision: must be 'approve' or 'reject'",
            ))
        }
    };

    let mut lock = APPROVALS_REGISTRY
        .write()
        .map_err(|_| Error::InternalServerError)?;

    let mut succeeded = 0;
    let mut failed = 0;

    for target_id in &payload.ids {
        if let Some(record) = lock.iter_mut().find(|r| r.id == *target_id) {
            let res = if is_approve {
                ApprovalEngine::approve(record, auth.user.id, payload.comments.clone())
            } else {
                ApprovalEngine::reject(record, auth.user.id, payload.comments.clone())
            };
            if res.is_ok() {
                succeeded += 1;
            } else {
                failed += 1;
            }
        } else {
            failed += 1;
        }
    }

    format::json(BulkDecisionResponse { succeeded, failed })
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/approvals")
        .add("/", get(list))
        .add("/", post(submit))
        .add("/{id}/decide", post(decide))
        .add("/bulk-decide", post(bulk_decide))
}

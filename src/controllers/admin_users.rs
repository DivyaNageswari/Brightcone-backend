//! Account administration for HR (ticket P1-11): create, unlock, disable, change role, reset password.
//! Every action is audited and revokes sessions where that makes sense.
use diesel::{prelude::QueryableByName, sql_query, sql_types::BigInt};
use diesel_async::RunQueryDsl;
use loco_rs::{hash, prelude::*};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    audit::{self, Entry},
    authz::{AuthUser, Role},
    database::Database,
    models::{
        error::ModelError,
        refresh_tokens,
        users::{self, NewUser},
    },
    request_meta::RequestMeta,
};

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(Serialize)]
struct UserPage {
    results: Vec<AdminUserResponse>,
    #[serde(rename = "pagination")]
    meta: PaginationMeta,
}

#[derive(Serialize)]
struct PaginationMeta {
    page: u64,
    page_size: u64,
    total_pages: u64,
    total_items: u64,
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    page: Option<u64>,
    page_size: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUserResponse {
    pub pid: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub status: String,
    pub failed_logins: i32,
    pub must_change_password: bool,
    pub last_login_at: Option<String>,
    pub created_at: String,
}

impl From<&users::Model> for AdminUserResponse {
    fn from(u: &users::Model) -> Self {
        Self {
            pid: u.pid.to_string(),
            email: u.email.clone(),
            name: u.name.clone(),
            role: u.role.clone(),
            status: u.status.clone(),
            failed_logins: u.failed_logins,
            must_change_password: u.must_change_password,
            last_login_at: u.last_login_at.map(|t| t.to_rfc3339()),
            created_at: u.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserParams {
    pub email: String,
    pub name: String,
    #[schema(example = "employee")]
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetRoleParams {
    #[schema(example = "manager")]
    pub role: String,
}

/// Returned once. The temporary password cannot be shown again.
#[derive(Debug, Serialize, ToSchema)]
pub struct TemporaryPasswordResponse {
    pub user: AdminUserResponse,
    pub temporary_password: String,
}

/// 16 random characters plus fixed classes, so it always passes the password rules.
fn temporary_password() -> String {
    format!("{}-Aa1", hash::random_string(16))
}

pub async fn find(ctx: &AppContext, pid: &str) -> Result<users::Model> {
    let db = Database::from_context(ctx)?;
    users::Model::find_by_pid(&db, pid)
        .await
        .map_err(|_| Error::NotFound)
}

/// Another active HR admin must exist before we demote, disable or lock this one.
pub async fn ensure_other_hr_admin(ctx: &AppContext, user: &users::Model) -> Result<()> {
    if user.role != Role::HrAdmin.as_str() || user.status != "active" {
        return Ok(());
    }
    let db = Database::from_context(ctx)?;
    let mut conn = db.get().await?;
    let others = sql_query(
        "SELECT count(*) AS count FROM users WHERE role = $1 AND status = 'active' AND id <> $2",
    )
    .bind::<diesel::sql_types::Text, _>(Role::HrAdmin.as_str())
    .bind::<diesel::sql_types::BigInt, _>(user.id)
    .get_result::<CountRow>(&mut *conn)
    .await
    .map_err(Database::map_error)?
    .count;
    if others == 0 {
        return bad_request("There must always be at least one active HR Admin.");
    }
    Ok(())
}

/// Lists accounts, 25 per page by default.
#[utoipa::path(
    get,
    path = "/api/admin/users",
    security(("bearer" = [])),
    responses((status = 200, description = "A page of accounts"), (status = 403, description = "HR Admin only"))
)]
#[debug_handler]
pub async fn list(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Response> {
    auth.require("users:manage")?;
    let db = Database::from_context(&ctx)?;
    let page_size = pagination.page_size.unwrap_or(25).clamp(1, 100);
    let page = pagination.page.unwrap_or(1).max(1);
    let offset = page.saturating_sub(1).saturating_mul(page_size);
    let mut conn = db.get().await?;
    let count = sql_query("SELECT count(*) AS count FROM users")
        .get_result::<CountRow>(&mut *conn)
        .await
        .map_err(Database::map_error)?
        .count;
    let rows = sql_query(format!(
        "{} ORDER BY id LIMIT $1 OFFSET $2",
        users::USER_SELECT
    ))
    .bind::<BigInt, _>(i64::try_from(page_size).unwrap_or(100))
    .bind::<BigInt, _>(i64::try_from(offset).unwrap_or(i64::MAX))
    .load::<users::Model>(&mut *conn)
    .await
    .map_err(Database::map_error)?;
    let items: Vec<AdminUserResponse> = rows.iter().map(AdminUserResponse::from).collect();
    let total_pages = u64::try_from(count).unwrap_or(0).div_ceil(page_size);
    format::json(UserPage {
        results: items,
        meta: PaginationMeta {
            page,
            page_size,
            total_pages,
            total_items: u64::try_from(count).unwrap_or(0),
        },
    })
}

/// Creates an account with a temporary password the person must change at first sign-in.
#[utoipa::path(
    post,
    path = "/api/admin/users",
    security(("bearer" = [])),
    request_body = CreateUserParams,
    responses((status = 200, description = "Account created", body = TemporaryPasswordResponse), (status = 409, description = "E-mail already used"))
)]
#[debug_handler]
pub async fn create(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    meta: RequestMeta,
    Json(params): Json<CreateUserParams>,
) -> Result<Response> {
    auth.require("users:manage")?;
    let db = Database::from_context(&ctx)?;
    let Some(role) = Role::parse(&params.role) else {
        return bad_request("Unknown role.");
    };
    let password = temporary_password();
    let user = users::Model::create_with_password(
        &db,
        &NewUser {
            email: params.email,
            name: params.name,
            password: password.clone(),
            role,
            must_change_password: true,
        },
    )
    .await
    .map_err(|e| match e {
        ModelError::EntityAlreadyExists => Error::CustomError(
            axum::http::StatusCode::CONFLICT,
            loco_rs::controller::ErrorDetail::new("conflict", "That email is already in use."),
        ),
        ModelError::Validation(_) => Error::BadRequest("Check the name and email.".to_owned()),
        other => other.into(),
    })?;
    audit::record(
        &db,
        Some(&auth.user),
        &meta,
        Entry::new("user.created")
            .target("user", user.pid)
            .summary(format!("{} as {}", user.email, user.role)),
    )
    .await?;
    format::json(TemporaryPasswordResponse {
        user: AdminUserResponse::from(&user),
        temporary_password: password,
    })
}

pub async fn change_status(
    auth: &AuthUser,
    ctx: &AppContext,
    meta: &RequestMeta,
    pid: &str,
    status: &str,
    action: &str,
) -> Result<Response> {
    auth.require("users:manage")?;
    let db = Database::from_context(ctx)?;
    let user = find(ctx, pid).await?;
    if status != "active" {
        if user.id == auth.user.id {
            return bad_request("You cannot lock or disable your own account.");
        }
        ensure_other_hr_admin(ctx, &user).await?;
    }
    let updated = user.set_status(&db, status).await?;
    if status != "active" {
        refresh_tokens::Model::revoke_all_for_user(&db, updated.id).await?;
    }
    audit::record(
        &db,
        Some(&auth.user),
        meta,
        Entry::new(action).target("user", updated.pid),
    )
    .await?;
    format::json(AdminUserResponse::from(&updated))
}

/// Unlocks a locked account, or re-enables a disabled one.
#[utoipa::path(post, path = "/api/admin/users/{pid}/unlock", security(("bearer" = [])),
    params(("pid" = String, Path, description = "Account id")),
    responses((status = 200, description = "Account is active again")))]
#[debug_handler]
pub async fn unlock(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    meta: RequestMeta,
    Path(pid): Path<String>,
) -> Result<Response> {
    change_status(&auth, &ctx, &meta, &pid, "active", "user.unlocked").await
}

/// Disables an account and signs it out everywhere.
#[utoipa::path(post, path = "/api/admin/users/{pid}/disable", security(("bearer" = [])),
    params(("pid" = String, Path, description = "Account id")),
    responses((status = 200, description = "Account disabled")))]
#[debug_handler]
pub async fn disable(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    meta: RequestMeta,
    Path(pid): Path<String>,
) -> Result<Response> {
    change_status(&auth, &ctx, &meta, &pid, "disabled", "user.disabled").await
}

/// Changes an account's role.
#[utoipa::path(put, path = "/api/admin/users/{pid}/role", security(("bearer" = [])),
    params(("pid" = String, Path, description = "Account id")), request_body = SetRoleParams,
    responses((status = 200, description = "Role changed")))]
#[debug_handler]
pub async fn set_role(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    meta: RequestMeta,
    Path(pid): Path<String>,
    Json(params): Json<SetRoleParams>,
) -> Result<Response> {
    auth.require("users:manage")?;
    let db = Database::from_context(&ctx)?;
    let Some(role) = Role::parse(&params.role) else {
        return bad_request("Unknown role.");
    };
    let user = find(&ctx, &pid).await?;
    if role != Role::HrAdmin {
        ensure_other_hr_admin(&ctx, &user).await?;
    }
    let old = user.role.clone();
    let updated = user.set_role(&db, role.as_str()).await?;
    // The new role applies from the next request; old sessions keep working but get new rights.
    audit::record(
        &db,
        Some(&auth.user),
        &meta,
        Entry::new("user.role_changed")
            .target("user", updated.pid)
            .summary(format!("{old} -> {}", updated.role)),
    )
    .await?;
    format::json(AdminUserResponse::from(&updated))
}

/// Sets a new temporary password (shown once) and signs the person out everywhere.
#[utoipa::path(post, path = "/api/admin/users/{pid}/reset-password", security(("bearer" = [])),
    params(("pid" = String, Path, description = "Account id")),
    responses((status = 200, description = "Temporary password, shown once", body = TemporaryPasswordResponse)))]
#[debug_handler]
pub async fn reset_password(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    meta: RequestMeta,
    Path(pid): Path<String>,
) -> Result<Response> {
    auth.require("users:manage")?;
    let db = Database::from_context(&ctx)?;
    let user = find(&ctx, &pid).await?;
    let password = temporary_password();
    let updated = user.set_password(&db, &password, true).await?;
    refresh_tokens::Model::revoke_all_for_user(&db, updated.id).await?;
    audit::record(
        &db,
        Some(&auth.user),
        &meta,
        Entry::new("user.password_reset_by_admin").target("user", updated.pid),
    )
    .await?;
    format::json(TemporaryPasswordResponse {
        user: AdminUserResponse::from(&updated),
        temporary_password: password,
    })
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/admin/users")
        .add("/", get(list))
        .add("/", post(create))
        .add("/{pid}/unlock", post(unlock))
        .add("/{pid}/disable", post(disable))
        .add("/{pid}/role", put(set_role))
        .add("/{pid}/reset-password", post(reset_password))
}

use loco_rs::{
    hash,
    model::query::{self, PaginationQuery},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    audit::{self, Entry},
    authz::{AuthUser, Role},
    models::{
        _entities::users::Column as UserColumn,
        refresh_tokens,
        users::{self, NewUser},
    },
    request_meta::RequestMeta,
};

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
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetRoleParams {
    pub role: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TemporaryPasswordResponse {
    pub user: AdminUserResponse,
    pub temporary_password: String,
}

fn temporary_password() -> String {
    format!("{}-Aa1", hash::random_string(16))
}

pub async fn find(ctx: &AppContext, pid: &str) -> Result<users::Model> {
    users::Model::find_by_pid(&ctx.db, pid)
        .await
        .map_err(|_| Error::NotFound)
}

pub async fn ensure_other_hr_admin(ctx: &AppContext, user: &users::Model) -> Result<()> {
    if user.role != Role::HrAdmin.as_str() || user.status != "active" {
        return Ok(());
    }
    let others = users::Entity::find()
        .filter(UserColumn::Role.eq(Role::HrAdmin.as_str()))
        .filter(UserColumn::Status.eq("active"))
        .filter(UserColumn::Id.ne(user.id))
        .count(&ctx.db)
        .await?;
    if others == 0 {
        return bad_request("There must always be at least one active HR Admin.");
    }
    Ok(())
}

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
    let page = query::paginate(
        &ctx.db,
        users::Entity::find().order_by_asc(UserColumn::Id),
        None,
        &pagination,
    )
    .await?;
    let items: Vec<AdminUserResponse> = page.page.iter().map(AdminUserResponse::from).collect();
    format::json(Pager::new(items, page.meta))
}

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
    let Some(role) = Role::parse(&params.role) else {
        return bad_request("Unknown role.");
    };
    let password = temporary_password();
    let user = users::Model::create_with_password(
        &ctx.db,
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
        crate::models::error::ModelError::EntityAlreadyExists => Error::CustomError(
            axum::http::StatusCode::CONFLICT,
            loco_rs::controller::ErrorDetail::new("conflict", "That email is already in use."),
        ),
        crate::models::error::ModelError::Validation(_) => Error::BadRequest("Check the name and email.".to_owned()),
        other => other.into(),
    })?;
    audit::record(
        &ctx.db,
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
    let user = find(ctx, pid).await?;
    if status != "active" {
        if user.id == auth.user.id {
            return bad_request("You cannot lock or disable your own account.");
        }
        ensure_other_hr_admin(ctx, &user).await?;
    }
    let updated = user.into_active_model().set_status(&ctx.db, status).await?;
    if status != "active" {
        refresh_tokens::Model::revoke_all_for_user(&ctx.db, updated.id).await?;
    }
    audit::record(
        &ctx.db,
        Some(&auth.user),
        meta,
        Entry::new(action).target("user", updated.pid),
    )
    .await?;
    format::json(AdminUserResponse::from(&updated))
}

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
    let Some(role) = Role::parse(&params.role) else {
        return bad_request("Unknown role.");
    };
    let user = find(&ctx, &pid).await?;
    if role != Role::HrAdmin {
        ensure_other_hr_admin(&ctx, &user).await?;
    }
    let old = user.role.clone();
    let mut active = user.into_active_model();
    active.role = ActiveValue::set(role.as_str().to_owned());
    let updated = active.update(&ctx.db).await?;
    audit::record(
        &ctx.db,
        Some(&auth.user),
        &meta,
        Entry::new("user.role_changed")
            .target("user", updated.pid)
            .summary(format!("{old} -> {}", updated.role)),
    )
    .await?;
    format::json(AdminUserResponse::from(&updated))
}

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
    let user = find(&ctx, &pid).await?;
    let password = temporary_password();
    let updated = user
        .into_active_model()
        .set_password(&ctx.db, &password, true)
        .await?;
    refresh_tokens::Model::revoke_all_for_user(&ctx.db, updated.id).await?;
    audit::record(
        &ctx.db,
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
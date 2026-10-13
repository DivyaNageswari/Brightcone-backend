use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    authz::{permissions_of, Role},
    models::users,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginParams {
    #[schema(example = "ananya.rao@brightcone.ai")]
    pub email: String,
    pub password: String,
    /// "Keep me signed in": the refresh cookie lasts days instead of the browser session.
    #[serde(default)]
    pub remember: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotParams {
    pub email: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetParams {
    pub token: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordParams {
    pub current_password: String,
    pub new_password: String,
}

/// Who the signed-in user is and what they may do.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MeResponse {
    pub pid: String,
    pub email: String,
    pub name: String,
    #[schema(example = "employee")]
    pub role: String,
    pub must_change_password: bool,
    /// `resource:action:scope`, for example `approvals:decide:team`.
    pub permissions: Vec<String>,
}

impl MeResponse {
    #[must_use]
    pub fn new(user: &users::Model) -> Self {
        let permissions = Role::parse(&user.role)
            .map(permissions_of)
            .unwrap_or_default();
        Self {
            pid: user.pid.to_string(),
            email: user.email.clone(),
            name: user.name.clone(),
            role: user.role.clone(),
            must_change_password: user.must_change_password,
            permissions,
        }
    }
}

/// Returned by login, refresh and change-password. The refresh token travels separately, in an
/// `HttpOnly` cookie that scripts cannot read.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionResponse {
    pub access_token: String,
    #[schema(example = "Bearer")]
    pub token_type: String,
    /// Seconds until the access token expires.
    pub expires_in: u64,
    pub user: MeResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

use loco_rs::prelude::*;
use serde::Deserialize;

/// Our own knobs, read from the `settings:` block of `config/<env>.yaml`.
/// Every field has a safe default, so a missing key never weakens security.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Send the refresh cookie only over HTTPS. Browsers also accept it on `localhost`.
    pub cookie_secure: bool,
    /// Lifetime of a "keep me signed in" session.
    pub refresh_ttl_days: i64,
    /// Lifetime of a browser-session login (cookie without `Max-Age`).
    pub session_ttl_hours: i64,
    /// Wrong passwords before an account is locked.
    pub lockout_threshold: i32,
    /// Requests per minute and client IP on `/api/auth/*`.
    pub auth_rate_limit_per_minute: u32,
    /// Read the client IP from `X-Forwarded-For` (only when behind our own reverse proxy).
    pub trust_proxy: bool,
    /// Where the browser app lives. Used in e-mailed links.
    pub frontend_url: String,
    /// Minutes a password-reset link stays valid.
    pub reset_token_minutes: i64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            cookie_secure: true,
            refresh_ttl_days: 14,
            session_ttl_hours: 12,
            lockout_threshold: 5,
            auth_rate_limit_per_minute: 10,
            trust_proxy: false,
            frontend_url: "http://localhost:5173".to_owned(),
            reset_token_minutes: 30,
        }
    }
}

impl AppSettings {
    /// Reads the `settings:` block, falling back to the defaults above.
    #[must_use]
    pub fn from_ctx(ctx: &AppContext) -> Self {
        ctx.config.settings::<Self>().unwrap_or_default()
    }
}

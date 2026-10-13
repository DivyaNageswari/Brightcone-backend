//! Cookie and token plumbing shared by login, refresh and change-password.
use axum::http::{header, HeaderMap, HeaderValue};
use loco_rs::prelude::*;

use crate::{
    models::{refresh_tokens::Issued, users},
    settings::AppSettings,
    views::auth::{MeResponse, SessionResponse},
};

pub const COOKIE_NAME: &str = "brightcone_rt";
/// The cookie is only sent to the auth endpoints, never to the rest of the API.
const COOKIE_PATH: &str = "/api/auth";
/// Browsers do not add custom headers to cross-site form posts. Requiring this one on the cookie
/// endpoints is the second layer of CSRF defence next to `SameSite=Strict`.
const CSRF_HEADER: &str = "x-requested-with";
const CSRF_VALUE: &str = "brightcone";

fn base(value: &str, settings: &AppSettings) -> String {
    let secure = if settings.cookie_secure {
        "; Secure"
    } else {
        ""
    };
    format!("{COOKIE_NAME}={value}; HttpOnly; SameSite=Strict; Path={COOKIE_PATH}{secure}")
}

/// `Set-Cookie` value that stores the refresh token.
#[must_use]
pub fn set_cookie(issued: &Issued, settings: &AppSettings) -> String {
    let mut cookie = base(&issued.secret, settings);
    if issued.persistent {
        cookie.push_str(&format!(
            "; Max-Age={}",
            settings.refresh_ttl_days * 24 * 3600
        ));
    }
    cookie
}

/// `Set-Cookie` value that deletes the refresh token.
#[must_use]
pub fn clear_cookie(settings: &AppSettings) -> String {
    format!("{}; Max-Age=0", base("", settings))
}

/// Reads the refresh token from the `Cookie` header.
#[must_use]
pub fn read_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, value)| value.to_owned())
        .filter(|v| !v.is_empty())
}

/// # Errors
/// `400` when the anti-CSRF header is missing.
pub fn require_csrf_header(headers: &HeaderMap) -> Result<()> {
    match headers.get(CSRF_HEADER).and_then(|v| v.to_str().ok()) {
        Some(CSRF_VALUE) => Ok(()),
        _ => Err(Error::BadRequest(
            "Missing X-Requested-With header.".to_owned(),
        )),
    }
}

fn with_cookie(mut response: Response, cookie: &str) -> Result<Response> {
    let value = HeaderValue::from_str(cookie).map_err(|_| Error::InternalServerError)?;
    response.headers_mut().append(header::SET_COOKIE, value);
    // Tokens must never be stored by browsers or proxies.
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

/// Builds the JSON session response and attaches the refresh cookie.
///
/// # Errors
/// When the access token cannot be signed.
pub fn session_response(
    ctx: &AppContext,
    settings: &AppSettings,
    user: &users::Model,
    issued: &Issued,
) -> Result<Response> {
    let jwt = ctx.config.get_jwt_config()?;
    let access_token = user.generate_jwt(&jwt.secret, jwt.expiration)?;
    let body = SessionResponse {
        access_token,
        token_type: "Bearer".to_owned(),
        expires_in: jwt.expiration,
        user: MeResponse::new(user),
    };
    with_cookie(format::json(body)?, &set_cookie(issued, settings))
}

/// A response that also deletes the refresh cookie.
///
/// # Errors
/// When the header cannot be built.
pub fn clearing(response: Response, settings: &AppSettings) -> Result<Response> {
    with_cookie(response, &clear_cookie(settings))
}

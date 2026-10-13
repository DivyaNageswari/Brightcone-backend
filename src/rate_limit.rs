//! Per-client-IP rate limit for the sign-in routes (ticket P1-03). Loco ships no rate limiter.
use std::{net::SocketAddr, num::NonZeroU32, sync::Arc};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};

#[derive(Clone)]
pub struct IpRateLimit(Arc<DefaultKeyedRateLimiter<String>>);

impl IpRateLimit {
    /// Allows `per_minute` requests per minute for each client IP.
    #[must_use]
    pub fn per_minute(per_minute: u32) -> Self {
        let n = NonZeroU32::new(per_minute.max(1)).unwrap_or(NonZeroU32::MIN);
        Self(Arc::new(RateLimiter::keyed(Quota::per_minute(n))))
    }
}

/// Use with `axum::middleware::from_fn_with_state(limiter, rate_limit)`.
pub async fn rate_limit(State(limiter): State<IpRateLimit>, req: Request, next: Next) -> Response {
    // Without a socket address (in-process tests) all callers share one bucket.
    let key = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map_or_else(|| "unknown".to_owned(), |c| c.0.ip().to_string());

    if limiter.0.check_key(&key).is_err() {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, "60")],
            axum::Json(serde_json::json!({
                "error": "too_many_requests",
                "description": "Too many attempts. Please wait a minute and try again."
            })),
        )
            .into_response();
    }
    next.run(req).await
}

use axum::{middleware::from_fn_with_state, routing::get, Router};
use axum_test::TestServer;
use brightcone_hr_backend::rate_limit::{rate_limit, IpRateLimit};

#[tokio::test]
async fn the_sixth_request_in_a_minute_is_refused() {
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(from_fn_with_state(IpRateLimit::per_minute(5), rate_limit));
    let server = TestServer::new(app).unwrap();

    for n in 1..=5 {
        assert_eq!(server.get("/").await.status_code(), 200, "request {n}");
    }
    let refused = server.get("/").await;
    assert_eq!(refused.status_code(), 429);
    assert_eq!(refused.headers()["retry-after"], "60");
}

use brightcone_hr_backend::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn health_reports_ok_when_database_is_up() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/health").await;

        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "ok");
        assert_eq!(body["database"], "ok");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn health_returns_security_headers() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/health").await;

        assert_eq!(
            response.headers()["content-security-policy"],
            Some("default-src 'self'; script-src 'self' 'unsafe-inline' style-src 'self' 'unsafe-inline' img-src 'self' data:; connect-src 'self'; frame-ancestors 'none';")
        );
        assert_eq!(response.headers()["strict-transport-security"], Some("max-age=31536000; includeSubDomains; preload"));
        assert_eq!(response.headers()["x-content-type-options"], Some("nosniff"));
        assert_eq!(
            response.headers()["referrer-policy"],
            Some("strict-origin-when-cross-origin")
        );
        assert_eq!(response.headers()["frame-ancestors"], Some("none"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn openapi_is_served_outside_production() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/openapi.json").await;

        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert!(body["paths"]["/api/health"].is_object());
    })
    .await;
}

//! Deny-by-default checks (ticket P1-08).
use axum::http::Method;
use brightcone_hr_backend::{app::App, authz::Role};
use loco_rs::{app::Hooks, testing::prelude::*};
use serial_test::serial;

use super::support::*;

/// Routes that are meant to work without signing in.
fn is_public(uri: &str) -> bool {
    matches!(
        uri,
        "/api/health" | "/api/openapi.json" | "/_ping" | "/_health" | "/_readiness"
    ) || uri.starts_with("/api/auth/")
}

fn with_dummy_params(uri: &str) -> String {
    uri.split('/')
        .map(|part| if part.starts_with('{') { "x" } else { part })
        .collect::<Vec<_>>()
        .join("/")
}

#[tokio::test]
#[serial]
async fn every_non_public_route_refuses_anonymous_callers() {
    request::<App, _, _>(|request, ctx| async move {
        let routes = App::routes(&ctx).collect();
        let mut checked = 0;
        for route in routes.iter().filter(|r| !is_public(&r.uri)) {
            let path = with_dummy_params(&route.uri);
            for action in &route.actions {
                let response = match *action {
                    Method::GET => request.get(&path).await,
                    Method::POST => request.post(&path).await,
                    Method::PUT => request.put(&path).await,
                    Method::PATCH => request.patch(&path).await,
                    Method::DELETE => request.delete(&path).await,
                    _ => continue,
                };
                assert_eq!(
                    response.status_code(),
                    401,
                    "{action} {} must require a signed-in user",
                    route.uri
                );
                checked += 1;
            }
        }
        assert!(checked >= 8, "only {checked} protected routes were checked");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn permission_matrix_for_hr_only_endpoints() {
    request::<App, _, _>(|request, ctx| async move {
        let cases = [
            (Role::Employee, 403),
            (Role::Manager, 403),
            (Role::Finance, 403),
            (Role::HrAdmin, 200),
        ];
        for (role, expected) in cases {
            let email = format!("{}@brightcone.ai", role.as_str());
            make_user(&ctx, &email, role).await;
            let (token, _) = sign_in(&request, &email).await;
            for path in ["/api/audit", "/api/admin/users"] {
                let response = get_as(&request, path, &token).await;
                assert_eq!(
                    response.status_code(),
                    expected,
                    "{} calling GET {path}",
                    role.as_str()
                );
            }
        }
    })
    .await;
}

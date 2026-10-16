use async_trait::async_trait;
use axum::{
    extract::Request,
    http::{header, HeaderName, HeaderValue, Method, StatusCode, Uri},
    middleware::{from_fn, Next},
    response::IntoResponse,
    Router,
};
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::truncate_table,
    environment::Environment,
    task::Tasks,
    Result,
};
use std::path::Path;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, limit::RequestBodyLimitLayer,
    timeout::TimeoutLayer,
};

use crate::{controllers, data::Migrator, models::users, tasks};

pub struct App;

// ---------------------------------------------------------------------------
// Middleware helpers (free functions)
// ---------------------------------------------------------------------------

async fn security_headers(req: Request, next: Next) -> impl IntoResponse {
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();

    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
        ),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // NOTE: `frame-ancestors` is a CSP directive, not a standalone header.
    // It is already included in the Content-Security-Policy above.

    resp
}

fn cors_layer(frontend_url: &str) -> Result<CorsLayer> {
    // NOTE: `allow_credentials(true)` cannot be combined with `allow_headers(Any)`
    // or `allow_origin(Any)` — the CORS spec forbids the wildcard in credentialed
    // requests, and tower-http panics on that combination. So we name the exact
    // origins and headers we accept instead of using the wildcard.
    let uri = frontend_url.parse::<Uri>().map_err(|_| {
        loco_rs::prelude::Error::string("FRONTEND_URL must be an absolute HTTP(S) URL")
    })?;
    let scheme = uri
        .scheme_str()
        .filter(|value| *value == "http" || *value == "https")
        .ok_or_else(|| loco_rs::prelude::Error::string("FRONTEND_URL must use HTTP or HTTPS"))?;
    let authority = uri
        .authority()
        .filter(|value| !value.as_str().contains('@'))
        .ok_or_else(|| loco_rs::prelude::Error::string("FRONTEND_URL must contain a valid host"))?;
    if uri.query().is_some() || frontend_url.contains('#') {
        return Err(loco_rs::prelude::Error::string(
            "FRONTEND_URL must not contain a query or fragment",
        ));
    }
    let frontend_origin = HeaderValue::from_str(&format!("{scheme}://{authority}"))
        .map_err(|_| loco_rs::prelude::Error::string("FRONTEND_URL must be a valid origin"))?;
    Ok(CorsLayer::new()
        .allow_origin(frontend_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::COOKIE,
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static("x-csrf-token"),
        ])
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(3600)))
}

// ---------------------------------------------------------------------------
// Hooks impl
// ---------------------------------------------------------------------------

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(ctx: &AppContext) -> AppRoutes {
        let mut routes = AppRoutes::with_default_routes()
            .add_route(controllers::health::routes())
            // `auth::routes` needs the AppContext because it reads the JWT secret
            // and cookie settings when it builds the refresh cookie.
            .add_route(controllers::auth::routes(ctx))
            .add_route(controllers::me::routes())
            .add_route(controllers::admin_users::routes())
            .add_route(controllers::audit_log::routes());
        // File and approvals controllers are prototypes backed by process-local registries.
        // Keep them out of the HTTP surface until durable, tenant-scoped persistence exists.

        // API docs are for developers — not served in production.
        if ctx.environment != Environment::Production {
            routes = routes.add_route(controllers::docs::routes());
        }

        routes
    }

    async fn after_routes(router: Router, ctx: &AppContext) -> Result<Router> {
        let origin = crate::settings::AppSettings::from_ctx(ctx).frontend_url;
        let router = router.merge(crate::modules::domain_router());
        Ok(router
            .layer(cors_layer(&origin)?)
            .layer(CompressionLayer::new().gzip(true).br(true))
            .layer(RequestBodyLimitLayer::new(10_000_000))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                std::time::Duration::from_secs(15),
            ))
            .layer(from_fn(security_headers)))
    }

    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue
            .register(crate::workers::cleanup::TokenCleanupWorker::build(ctx))
            .await?;
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        // NOTE: the `tasks` parameter shadows the `tasks` module above.
        // Path-position `tasks::…` still resolves to the module — Rust keeps
        // the module namespace and value namespace separate. This matches
        // the loco-rs template.
        tasks.register(tasks::user_create::UserCreate);
        tasks.register(tasks::seed_demo::SeedDemo);
    }

    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        // Demo data comes from the seed_demo task.
        Ok(())
    }
}

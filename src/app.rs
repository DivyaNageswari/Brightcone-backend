use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::truncate_table,
    environment::Environment,
    task::Tasks,
    Result,
};
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderName, HeaderValue, Method},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::post,
};
use loco_rs::prelude::*;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::compression::CompressionLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::set_header::SetLayer;
use tower_http::trace::MakeSpan;

pub struct App;
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

    fn security_headers(_req: Request, next: Next) -> impl IntoResponse {
    // B1.12: Security headers on every response.
    let mut resp = next.into_response();
    resp.headers().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline' style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none';"),
    );
    resp.headers().insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );
    resp.headers().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    resp.headers().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    resp.headers().insert(
        header::FRAME_ANCESTORS,
        HeaderValue::from_static("none"),
    );
    resp
}

fn cors_layer() -> CorsLayer {
    // B1.12: CORS only for our frontend, with credentials.
    CorsLayer::new()
        .allow_origin("http://localhost:5173".parse().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH])
        .allow_headers(Any)
        .allow_credentials(true)
}

fn body_size_limit(limit: usize) -> impl tower::Service<axum::body::Body, Error = axum::error::Infallibility> + Clone + Send + Sync + 'static {
    // B1.12: Reject JSON bodies > 1 MB and uploads > 10 MB.
    || async move |body: axum::body::Body| {
        let bytes = hyper::body::to_bytes(body, limit).await.map_err(|_| axum::error::Infallibility::INFALLIBLE)?;
        Ok::<_, axum::error::Infallibility>(bytes.into())
    }
}

fn rate_limit_middleware(limiter: IpRateLimit) -> impl tower::Service<axum::body::Body, Error = axum::error::Infallibility> + Clone + Send + Sync + 'static {
    || async move |req: Request| rate_limit(State(limiter.clone()), req).await
}

fn app_routes(ctx: &AppContext, rate_limiter: IpRateLimit) -> AppRoutes {
    // B1.12: Security headers on every response.
    let routes = AppRoutes::with_default_routes()
        .add_route(controllers::health::routes())
        // Wrap every route with security headers.
        .wrap_fn(from_fn_with_state(rate_limiter.clone(), rate_limit_middleware(rate_limiter)))
        // CORS only for our frontend with credentials.
        .wrap(cors_layer())
        // Compress gzip/brotli.
        .wrap(CompressionLayer::new().gzip(true).brotli(true))
        // Body size limits.
        .wrap(body_size_limit(1_000_000).or(body_size_limit(10_000_000)))
        // Request timeout: 15 s per B1.12.
        .wrap(TimeoutLayer::new(std::time::Duration::from_secs(15)))
        // Security headers on every response.
        .wrap_fn(from_fn_with_state(ctx.clone(), |_req: Request, next: Next| security_headers(_req, next)))
        // API docs are for developers, so they are not served in production.
        if ctx.environment == Environment::Production {
            routes
        } else {
            routes.add_route(controllers::docs::routes())
        }
    }
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        // Workers are registered here as tickets add them (PDF generation, CSV import, ...).
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
        tasks.register(tasks::user_create::UserCreate);
        tasks.register(tasks::seed_demo::SeedDemo);
    }
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        // Demo data comes from the seed_demo task, which generates random passwords.
        Ok(())
    }
}

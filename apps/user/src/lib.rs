//! The user service: accounts, sessions, and (later) runs and preferences.
//!
//! Everything is one axum router, deployed as a single Vercel function that
//! every `/api/*` path is rewritten to (see `vercel.json`). It's a library so
//! the tests can run the same router against a local Postgres.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderValue, Method, header};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use shared::account::ErrorCode;
use sqlx::postgres::{PgConnectOptions, PgConnection, PgPool, PgPoolOptions};
use sqlx::{ConnectOptions, Connection};
use tokio::sync::OnceCell;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;

mod error;
mod json;
mod limits;
mod passwords;
mod routes;
mod session;

pub use error::AppError;

/// Where the web app is served; the only origin allowed in production.
pub const WEB_ORIGIN: &str = "https://pokeguess.piercew.xyz";
/// Also allowed outside production, for `trunk serve`.
pub const DEV_ORIGINS: [&str; 2] = ["http://localhost:8000", "http://127.0.0.1:8000"];

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

pub struct Config {
    /// For queries: Neon's pooled connection.
    pub database: PgConnectOptions,
    /// For migrations, which take a session-level lock, so they need a
    /// direct connection rather than the pooled one.
    pub migrations: PgConnectOptions,
    /// Origins allowed to call the API with cookies.
    pub origins: Vec<String>,
}

impl Config {
    /// From the environment Vercel sets: the Neon integration's
    /// `POKEGUESS_DATABASE_URL` (pooled) and `POKEGUESS_DATABASE_URL_UNPOOLED`,
    /// and `VERCEL_ENV`.
    pub fn from_env() -> Result<Self, String> {
        let url = std::env::var("POKEGUESS_DATABASE_URL")
            .map_err(|_| "POKEGUESS_DATABASE_URL isn't set".to_string())?;
        let unpooled = std::env::var("POKEGUESS_DATABASE_URL_UNPOOLED").unwrap_or(url.clone());
        let parse = |url: &str| {
            PgConnectOptions::from_str(url).map_err(|err| format!("bad database URL: {err}"))
        };
        let production = std::env::var("VERCEL_ENV").as_deref() == Ok("production");
        Ok(Self {
            database: parse(&url)?,
            migrations: parse(&unpooled)?,
            origins: allowed_origins(production),
        })
    }
}

pub fn allowed_origins(production: bool) -> Vec<String> {
    let mut origins = vec![WEB_ORIGIN.to_string()];
    if !production {
        origins.extend(DEV_ORIGINS.map(String::from));
    }
    origins
}

struct Inner {
    pool: PgPool,
    migrations: PgConnectOptions,
    migrated: OnceCell<()>,
    origins: Vec<String>,
}

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

impl AppState {
    pub fn new(config: Config) -> Self {
        // connects on first use, so a cold start doesn't wait for the database
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            // no named prepared statements, which a transaction-mode pooler
            // can't keep track of
            .connect_lazy_with(config.database.statement_cache_capacity(0));
        Self(Arc::new(Inner {
            pool,
            migrations: config.migrations,
            migrated: OnceCell::new(),
            origins: config.origins,
        }))
    }

    pub fn pool(&self) -> &PgPool {
        &self.0.pool
    }

    /// Brings the database's tables up to date, once per instance.
    pub async fn migrate(&self) -> Result<(), AppError> {
        self.0
            .migrated
            .get_or_try_init(|| run_migrations(&self.0.migrations))
            .await?;
        Ok(())
    }

    fn allows(&self, origin: &[u8]) -> bool {
        self.0.origins.iter().any(|o| o.as_bytes() == origin)
    }
}

async fn run_migrations(options: &PgConnectOptions) -> Result<(), AppError> {
    let mut connection: PgConnection = options.connect().await?;
    MIGRATOR.run_direct(&mut connection).await?;
    connection.close().await?;
    Ok(())
}

/// The whole API.
pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(
            state
                .0
                .origins
                .iter()
                .filter_map(|o| HeaderValue::from_str(o).ok()),
        ))
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE])
        .max_age(Duration::from_secs(600));

    Router::new()
        .route("/api/signup", post(routes::signup))
        .route("/api/login", post(routes::login))
        .route("/api/logout", post(routes::logout))
        .route("/api/me", get(routes::me).delete(routes::delete_account))
        .route("/api/password", post(routes::change_password))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            ensure_migrated,
        ))
        .route("/api/health", get(routes::health))
        .fallback(routes::not_found)
        .layer(middleware::from_fn_with_state(state.clone(), check_origin))
        .layer(cors)
        // nothing here should ever be cached
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .with_state(state)
}

async fn ensure_migrated(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    state.migrate().await?;
    Ok(next.run(request).await)
}

/// Requests that change anything must come from the web app. Together with
/// `SameSite=Lax` cookies and JSON-only bodies (which need a CORS preflight),
/// this stops other sites, including sibling subdomains, from acting for a
/// signed-in player.
async fn check_origin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let safe = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let allowed = request
        .headers()
        .get(header::ORIGIN)
        .is_some_and(|origin| state.allows(origin.as_bytes()));
    if !safe && !allowed {
        return Err(AppError::new(
            axum::http::StatusCode::FORBIDDEN,
            ErrorCode::ForbiddenOrigin,
            "Requests have to come from PokéGuess.",
        ));
    }
    Ok(next.run(request).await)
}

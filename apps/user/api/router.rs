//! The Vercel function behind every `/api/*` path.

use tower::ServiceBuilder;
use user::{AppState, Config, router};
use vercel_runtime::axum::VercelLayer;
use vercel_runtime::{Error, run};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let state = AppState::new(Config::from_env()?);
    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(router(state));
    run(app).await
}

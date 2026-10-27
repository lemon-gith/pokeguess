use serde_json::Value;
use shared::Health;
use vercel_runtime::{Error, Request, run, service_fn};

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(handler)).await
}

async fn handler(_req: Request) -> Result<Value, Error> {
    Ok(serde_json::to_value(Health::ok())?)
}

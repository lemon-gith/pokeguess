//! Calls to the user service (accounts). Requests carry the session cookie,
//! which the service sets and the browser keeps; this code never sees it.

use gloo::net::http::{Request, RequestBuilder};
use serde::Serialize;
use serde::de::DeserializeOwned;
use shared::account::{ApiError, ChangePassword, Credentials, DeleteAccount, ErrorCode, Me};
use web_sys::RequestCredentials;

/// The user service, by where the web app is running: the local service
/// (`cargo run -p user --bin router` with `VERCEL_DEV_PORT=3123`) when served
/// from this machine, else production.
fn base_url() -> &'static str {
    let host = gloo::utils::window()
        .location()
        .hostname()
        .unwrap_or_default();
    if host == "localhost" || host == "127.0.0.1" {
        "http://localhost:3123"
    } else {
        "https://user.pokeguess.piercew.xyz"
    }
}

/// What went wrong, fit to show to the player.
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub code: ErrorCode,
    pub message: String,
}

impl Problem {
    fn unreachable() -> Self {
        Self {
            code: ErrorCode::ServerError,
            message: "Couldn't reach PokéGuess accounts. Please try again in a moment.".into(),
        }
    }
}

fn request(builder: RequestBuilder) -> RequestBuilder {
    builder.credentials(RequestCredentials::Include)
}

/// Sends the request; an error status becomes a [`Problem`] from its body.
async fn send(
    request: Result<Request, gloo::net::Error>,
) -> Result<gloo::net::http::Response, Problem> {
    let response = request
        .map_err(|_| Problem::unreachable())?
        .send()
        .await
        .map_err(|_| Problem::unreachable())?;
    if response.ok() {
        return Ok(response);
    }
    match response.json::<ApiError>().await {
        Ok(error) => Err(Problem {
            code: error.error,
            message: error.message,
        }),
        Err(_) => Err(Problem::unreachable()),
    }
}

async fn post<B: Serialize, T: DeserializeOwned>(path: &str, body: &B) -> Result<T, Problem> {
    let response =
        send(request(Request::post(&format!("{}{path}", base_url()))).json(body)).await?;
    response.json().await.map_err(|_| Problem::unreachable())
}

/// Who's signed in on this browser, if anyone.
pub async fn me() -> Result<Option<Me>, Problem> {
    let request = request(Request::get(&format!("{}/api/me", base_url()))).build();
    match send(request).await {
        Ok(response) => response
            .json()
            .await
            .map(Some)
            .map_err(|_| Problem::unreachable()),
        Err(problem) if problem.code == ErrorCode::Unauthenticated => Ok(None),
        Err(problem) => Err(problem),
    }
}

pub async fn sign_up(credentials: &Credentials) -> Result<Me, Problem> {
    post("/api/signup", credentials).await
}

pub async fn log_in(credentials: &Credentials) -> Result<Me, Problem> {
    post("/api/login", credentials).await
}

pub async fn log_out() -> Result<(), Problem> {
    send(request(Request::post(&format!("{}/api/logout", base_url()))).build())
        .await
        .map(|_| ())
}

pub async fn change_password(change: &ChangePassword) -> Result<(), Problem> {
    send(request(Request::post(&format!("{}/api/password", base_url()))).json(change))
        .await
        .map(|_| ())
}

pub async fn delete_account(delete: &DeleteAccount) -> Result<(), Problem> {
    send(request(Request::delete(&format!("{}/api/me", base_url()))).json(delete))
        .await
        .map(|_| ())
}

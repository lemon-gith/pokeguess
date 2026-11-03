//! The endpoints. Request and response bodies are the types in
//! `shared::account`.

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use shared::Health;
use shared::account::{
    ChangePassword, Credentials, DeleteAccount, ErrorCode, Me, validate_password, validate_username,
};

use crate::json::Body;
use crate::limits::Limit;
use crate::session::{self, Session};
use crate::{AppError, AppState, passwords};

/// Postgres's code for a unique constraint violation.
const UNIQUE_VIOLATION: &str = "23505";

pub async fn health() -> Json<Health> {
    Json(Health::ok())
}

pub async fn not_found() -> AppError {
    AppError::new(
        StatusCode::NOT_FOUND,
        ErrorCode::NotFound,
        "There's nothing here.",
    )
}

/// A response that signs the player in.
fn signed_in(status: StatusCode, token: &str, me: Me) -> Response {
    (
        status,
        [(header::SET_COOKIE, session::cookie(token))],
        Json(me),
    )
        .into_response()
}

fn signed_out() -> Response {
    (
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, session::clear_cookie())],
    )
        .into_response()
}

/// `POST /api/signup`: creates an account and signs in. 201 with [`Me`].
pub async fn signup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Body(credentials): Body<Credentials>,
) -> Result<Response, AppError> {
    let Credentials { username, password } = credentials;
    validate_username(&username)
        .map_err(|message| AppError::bad_request(ErrorCode::InvalidUsername, message))?;
    validate_password(&password)
        .map_err(|message| AppError::bad_request(ErrorCode::InvalidPassword, message))?;
    let pool = state.pool();
    let limit = Limit::signup_address(&session::address(&headers));
    limit.check(pool).await?;
    limit.record(pool).await?;

    let hash = passwords::hash(password).await?;
    let inserted: Result<(i64, f64), sqlx::Error> = sqlx::query_as(
        "INSERT INTO users (username, password_hash) VALUES ($1, $2)
         RETURNING id, (EXTRACT(EPOCH FROM created_at) * 1000)::float8",
    )
    .bind(&username)
    .bind(hash)
    .fetch_one(pool)
    .await;
    let (user_id, created_at) = match inserted {
        Ok(row) => row,
        Err(sqlx::Error::Database(err)) if err.code().as_deref() == Some(UNIQUE_VIOLATION) => {
            return Err(AppError::new(
                StatusCode::CONFLICT,
                ErrorCode::UsernameTaken,
                "That username is taken.",
            ));
        }
        Err(err) => return Err(err.into()),
    };
    let token = session::create(pool, user_id).await?;
    Ok(signed_in(
        StatusCode::CREATED,
        &token,
        Me {
            username,
            created_at,
        },
    ))
}

/// `POST /api/login`: 200 with [`Me`]. The username isn't case-sensitive.
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Body(credentials): Body<Credentials>,
) -> Result<Response, AppError> {
    let Credentials { username, password } = credentials;
    let pool = state.pool();
    let limits = [
        Limit::login_username(&username),
        Limit::login_address(&session::address(&headers)),
    ];
    for limit in &limits {
        limit.check(pool).await?;
    }

    let user: Option<(i64, String, String, f64)> = sqlx::query_as(
        "SELECT id, username, password_hash, (EXTRACT(EPOCH FROM created_at) * 1000)::float8
         FROM users WHERE lower(username) = lower($1)",
    )
    .bind(&username)
    .fetch_optional(pool)
    .await?;
    let hash = user.as_ref().map(|(_, _, hash, _)| hash.clone());
    let matches = passwords::verify(password, hash).await?;
    let Some((user_id, username, _, created_at)) = user.filter(|_| matches) else {
        for limit in &limits {
            limit.record(pool).await?;
        }
        return Err(AppError::wrong_credentials());
    };
    let token = session::create(pool, user_id).await?;
    Ok(signed_in(
        StatusCode::OK,
        &token,
        Me {
            username,
            created_at,
        },
    ))
}

/// `POST /api/logout`: ends this session. Always 204.
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(token) = session::token(&headers) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(session::hash(&token))
            .execute(state.pool())
            .await?;
    }
    Ok(signed_out())
}

/// `GET /api/me`: who's signed in, or 401. The web app calls this when it
/// loads, so it's also where sessions are renewed.
pub async fn me(State(state): State<AppState>, session: Session) -> Result<Response, AppError> {
    let me = Me {
        username: session.username.clone(),
        created_at: session.created_at,
    };
    if session.stale {
        session.renew(state.pool()).await?;
        return Ok(signed_in(StatusCode::OK, &session.token, me));
    }
    Ok(Json(me).into_response())
}

/// Checks the signed-in player's password, rate limited per account.
async fn confirm_password(
    state: &AppState,
    session: &Session,
    password: String,
) -> Result<(), AppError> {
    let pool = state.pool();
    let limit = Limit::account_password(session.user_id);
    limit.check(pool).await?;
    let (hash,): (String,) = sqlx::query_as("SELECT password_hash FROM users WHERE id = $1")
        .bind(session.user_id)
        .fetch_one(pool)
        .await?;
    if !passwords::verify(password, Some(hash)).await? {
        limit.record(pool).await?;
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            ErrorCode::WrongCredentials,
            "That password isn't right.",
        ));
    }
    Ok(())
}

/// `POST /api/password`: 204. Signs out every other session.
pub async fn change_password(
    State(state): State<AppState>,
    session: Session,
    Body(change): Body<ChangePassword>,
) -> Result<StatusCode, AppError> {
    validate_password(&change.new_password)
        .map_err(|message| AppError::bad_request(ErrorCode::InvalidPassword, message))?;
    confirm_password(&state, &session, change.current_password).await?;
    let hash = passwords::hash(change.new_password).await?;
    let mut transaction = state.pool().begin().await?;
    sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
        .bind(session.user_id)
        .bind(hash)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND token_hash <> $2")
        .bind(session.user_id)
        .bind(session::hash(&session.token))
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/me`: deletes the account and everything in it. 204.
pub async fn delete_account(
    State(state): State<AppState>,
    session: Session,
    Body(delete): Body<DeleteAccount>,
) -> Result<Response, AppError> {
    confirm_password(&state, &session, delete.password).await?;
    // sessions go with it
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(session.user_id)
        .execute(state.pool())
        .await?;
    Ok(signed_out())
}

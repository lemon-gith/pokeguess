//! Errors, sent as JSON [`ApiError`] bodies.

use std::fmt::Display;

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use shared::account::{ApiError, ErrorCode};

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    code: ErrorCode,
    message: String,
    /// For [`ErrorCode::RateLimited`]: seconds until trying again is worth it.
    retry_after: Option<u64>,
}

impl AppError {
    pub fn new(status: StatusCode, code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            retry_after: None,
        }
    }

    pub fn bad_request(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn unauthenticated() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            ErrorCode::Unauthenticated,
            "You're not signed in.",
        )
    }

    pub fn wrong_credentials() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            ErrorCode::WrongCredentials,
            "That username and password don't match.",
        )
    }

    pub fn rate_limited(retry_after: u64) -> Self {
        Self {
            retry_after: Some(retry_after),
            ..Self::new(
                StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::RateLimited,
                "Too many tries. Please wait a bit and try again.",
            )
        }
    }

    /// Something went wrong on our side. The details are logged, not sent.
    pub fn server(err: impl Display) -> Self {
        eprintln!("server error: {err}");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::ServerError,
            "Something went wrong. Please try again.",
        )
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::server(err)
    }
}

impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        Self::server(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = ApiError {
            error: self.code,
            message: self.message,
        };
        let mut response = (self.status, Json(body)).into_response();
        if let Some(seconds) = self.retry_after {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
        }
        response
    }
}

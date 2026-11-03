//! A JSON request body whose rejections are [`AppError`]s, so every error
//! the API sends has the same shape.

use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use serde::de::DeserializeOwned;
use shared::account::ErrorCode;

use crate::AppError;

pub struct Body<T>(pub T);

impl<S, T> FromRequest<S> for Body<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request(request: Request, state: &S) -> Result<Self, AppError> {
        axum::Json::<T>::from_request(request, state)
            .await
            .map(|axum::Json(value)| Body(value))
            .map_err(|rejection: JsonRejection| {
                AppError::new(
                    rejection.status(),
                    ErrorCode::BadRequest,
                    rejection.body_text(),
                )
            })
    }
}

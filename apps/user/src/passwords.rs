//! Password hashing with argon2id.
//!
//! The parameters are the argon2 crate's defaults, which are OWASP's
//! recommended minimum (19 MiB, 2 passes, 1 lane): a few tens of
//! milliseconds, comfortably within a serverless function's memory. Hashing
//! runs on a blocking thread so it doesn't hold up other requests.

use std::sync::LazyLock;

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

use crate::AppError;

/// Checked against when the username doesn't exist, so a wrong username
/// takes as long as a wrong password.
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_now("not anyone's password").expect("hashing a constant can't fail"));

fn hash_now(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

pub async fn hash(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || hash_now(&password))
        .await
        .map_err(AppError::server)?
        .map_err(AppError::server)
}

/// Whether `password` matches `hash`, or with no hash, false after the same
/// amount of work.
pub async fn verify(password: String, hash: Option<String>) -> Result<bool, AppError> {
    tokio::task::spawn_blocking(move || {
        let known = hash.is_some();
        let hash = hash.unwrap_or_else(|| DUMMY_HASH.clone());
        let parsed = PasswordHash::new(&hash).map_err(AppError::server)?;
        let matches = Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        Ok(known && matches)
    })
    .await
    .map_err(AppError::server)?
}

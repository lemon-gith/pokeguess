//! Sessions: a random token in a cookie, stored only as its SHA-256 hash, so
//! a leaked database can't be used to sign in. Logging out deletes the row.

use argon2::password_hash::rand_core::{OsRng, RngCore};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, header};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::{AppError, AppState};

/// `__Host-` means the browser only accepts it from this exact host, over
/// HTTPS, for the whole site, so sibling subdomains can't set or read it.
pub const COOKIE: &str = "__Host-session";
pub const LIFETIME_DAYS: i64 = 30;
/// Sessions are renewed (by `GET /api/me`) once they're older than this.
const RENEW_AFTER_DAYS: i64 = 1;

pub fn hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Starts a session, returning its token.
pub async fn create(pool: &PgPool, user_id: i64) -> Result<String, AppError> {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, expires_at)
         VALUES ($1, $2, now() + make_interval(days => $3))",
    )
    .bind(hash(&token))
    .bind(user_id)
    .bind(LIFETIME_DAYS as i32)
    .execute(pool)
    .await?;
    // a good moment to clear out old sessions
    sqlx::query("DELETE FROM sessions WHERE expires_at < now()")
        .execute(pool)
        .await?;
    Ok(token)
}

/// The `Set-Cookie` value for a session token.
pub fn cookie(token: &str) -> HeaderValue {
    let max_age = LIFETIME_DAYS * 24 * 60 * 60;
    HeaderValue::from_str(&format!(
        "{COOKIE}={token}; Path=/; Max-Age={max_age}; Secure; HttpOnly; SameSite=Lax"
    ))
    .expect("tokens are base64url")
}

/// The `Set-Cookie` value that removes the session cookie.
pub fn clear_cookie() -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{COOKIE}=; Path=/; Max-Age=0; Secure; HttpOnly; SameSite=Lax"
    ))
    .expect("constant")
}

/// The session token from the request's cookies, if any.
pub fn token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE)
        .map(|(_, value)| value.to_string())
        .filter(|value| !value.is_empty())
}

/// The signed-in player. Extracting it fails with 401 without a valid,
/// unexpired session.
pub struct Session {
    pub user_id: i64,
    pub username: String,
    /// Milliseconds since the Unix epoch.
    pub created_at: f64,
    pub token: String,
    /// Whether it's due to be renewed.
    pub stale: bool,
}

impl FromRequestParts<AppState> for Session {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let token = token(&parts.headers).ok_or_else(AppError::unauthenticated)?;
        let row: Option<(i64, String, f64, bool)> = sqlx::query_as(
            "SELECT u.id, u.username, (EXTRACT(EPOCH FROM u.created_at) * 1000)::float8,
                    s.expires_at < now() + make_interval(days => $2)
             FROM sessions s JOIN users u ON u.id = s.user_id
             WHERE s.token_hash = $1 AND s.expires_at > now()",
        )
        .bind(hash(&token))
        .bind((LIFETIME_DAYS - RENEW_AFTER_DAYS) as i32)
        .fetch_optional(state.pool())
        .await?;
        let (user_id, username, created_at, stale) = row.ok_or_else(AppError::unauthenticated)?;
        Ok(Self {
            user_id,
            username,
            created_at,
            token,
            stale,
        })
    }
}

impl Session {
    /// Pushes the expiry back to a full lifetime from now.
    pub async fn renew(&self, pool: &PgPool) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE sessions SET expires_at = now() + make_interval(days => $2)
             WHERE token_hash = $1",
        )
        .bind(hash(&self.token))
        .bind(LIFETIME_DAYS as i32)
        .execute(pool)
        .await?;
        Ok(())
    }
}

/// The client's address, for rate limiting. Vercel sets `X-Forwarded-For`
/// to the client's address itself, so it can't be spoofed there.
pub fn address(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_session_cookie_among_others() {
        let mut headers = HeaderMap::new();
        headers.append(
            header::COOKIE,
            HeaderValue::from_static("a=1; __Host-session=abc"),
        );
        assert_eq!(token(&headers).as_deref(), Some("abc"));

        let mut headers = HeaderMap::new();
        headers.append(header::COOKIE, HeaderValue::from_static("a=1"));
        headers.append(
            header::COOKIE,
            HeaderValue::from_static("__Host-session=xyz"),
        );
        assert_eq!(token(&headers).as_deref(), Some("xyz"));

        let mut headers = HeaderMap::new();
        headers.append(header::COOKIE, HeaderValue::from_static("__Host-session="));
        assert_eq!(token(&headers), None);
    }

    #[test]
    fn the_first_forwarded_address_is_the_client() {
        let mut headers = HeaderMap::new();
        assert_eq!(address(&headers), "unknown");
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("1.2.3.4, 10.0.0.1"),
        );
        assert_eq!(address(&headers), "1.2.3.4");
    }
}

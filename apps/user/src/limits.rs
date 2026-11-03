//! Rate limiting, counted in the `attempts` table, so there's nothing else
//! to host. Each limit allows `max` attempts per key in a sliding window.

use sqlx::PgPool;

use crate::AppError;

pub struct Limit {
    pub key: String,
    pub max: i64,
    pub window_secs: i64,
}

const FIFTEEN_MINUTES: i64 = 15 * 60;
const HOUR: i64 = 60 * 60;

impl Limit {
    /// Wrong passwords for one username, from anywhere.
    pub fn login_username(username: &str) -> Self {
        Self {
            key: format!("login:user:{}", username.to_lowercase()),
            max: 10,
            window_secs: FIFTEEN_MINUTES,
        }
    }

    /// Wrong passwords from one address, for any usernames.
    pub fn login_address(address: &str) -> Self {
        Self {
            key: format!("login:ip:{address}"),
            max: 50,
            window_secs: FIFTEEN_MINUTES,
        }
    }

    /// Accounts created from one address.
    pub fn signup_address(address: &str) -> Self {
        Self {
            key: format!("signup:ip:{address}"),
            max: 10,
            window_secs: HOUR,
        }
    }

    /// Wrong passwords when changing the password or deleting the account,
    /// so a stolen session can't be used to guess it.
    pub fn account_password(user_id: i64) -> Self {
        Self {
            key: format!("password:{user_id}"),
            max: 10,
            window_secs: FIFTEEN_MINUTES,
        }
    }

    /// Fails with [`AppError::rate_limited`] once the window is full.
    pub async fn check(&self, pool: &PgPool) -> Result<(), AppError> {
        let (count, retry_after): (i64, Option<f64>) = sqlx::query_as(
            "SELECT count(*), EXTRACT(EPOCH FROM min(at) + make_interval(secs => $2) - now())::float8
             FROM attempts
             WHERE key = $1 AND at > now() - make_interval(secs => $2)",
        )
        .bind(&self.key)
        .bind(self.window_secs as f64)
        .fetch_one(pool)
        .await?;
        if count >= self.max {
            let seconds = retry_after.unwrap_or(1.0).ceil().max(1.0) as u64;
            return Err(AppError::rate_limited(seconds));
        }
        Ok(())
    }

    /// Counts an attempt, and forgets any older than a day.
    pub async fn record(&self, pool: &PgPool) -> Result<(), AppError> {
        sqlx::query("INSERT INTO attempts (key) VALUES ($1)")
            .bind(&self.key)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM attempts WHERE at < now() - interval '1 day'")
            .execute(pool)
            .await?;
        Ok(())
    }
}

//! Accounts: what the web app and the user service send each other, and the
//! rules for usernames and passwords, so both check them the same way.

use serde::{Deserialize, Serialize};

pub const USERNAME_MIN: usize = 3;
pub const USERNAME_MAX: usize = 20;
pub const PASSWORD_MIN: usize = 8;
pub const PASSWORD_MAX: usize = 128;

/// Signing up or logging in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// The signed-in account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Me {
    /// As it was typed when signing up.
    pub username: String,
    /// Milliseconds since the Unix epoch.
    pub created_at: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangePassword {
    pub current_password: String,
    pub new_password: String,
}

/// Deleting the account needs the password again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteAccount {
    pub password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidUsername,
    InvalidPassword,
    UsernameTaken,
    /// The username or password is wrong; which one isn't said.
    WrongCredentials,
    /// Too many attempts; try again later.
    RateLimited,
    /// Not signed in, or the session has expired.
    Unauthenticated,
    /// The request didn't come from the web app.
    ForbiddenOrigin,
    BadRequest,
    NotFound,
    ServerError,
}

/// The body of every error response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub error: ErrorCode,
    /// Fit to show to the player.
    pub message: String,
}

/// 3–20 letters, numbers, `_` or `-`. Usernames are unique regardless of
/// case.
pub fn validate_username(username: &str) -> Result<(), String> {
    let length = username.chars().count();
    if !(USERNAME_MIN..=USERNAME_MAX).contains(&length) {
        return Err(format!(
            "Usernames are {USERNAME_MIN} to {USERNAME_MAX} characters long."
        ));
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Usernames can only use letters, numbers, _ and -.".into());
    }
    Ok(())
}

/// 8–128 characters, and nothing else required.
pub fn validate_password(password: &str) -> Result<(), String> {
    let length = password.chars().count();
    if length < PASSWORD_MIN {
        return Err(format!(
            "Passwords need at least {PASSWORD_MIN} characters."
        ));
    }
    if length > PASSWORD_MAX {
        return Err(format!(
            "Passwords can't be longer than {PASSWORD_MAX} characters."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usernames() {
        for good in ["ash", "Red_2", "team-rocket", "abcdefghijklmnopqrst"] {
            assert!(validate_username(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            "ab",
            "abcdefghijklmnopqrstu",
            "with space",
            "émile",
            "a.b",
        ] {
            assert!(validate_username(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn passwords_count_characters_not_bytes() {
        assert!(validate_password("1234567").is_err());
        assert!(validate_password("12345678").is_ok());
        // 8 characters, 16 bytes
        assert!(validate_password("éééééééé").is_ok());
        assert!(validate_password(&"x".repeat(128)).is_ok());
        assert!(validate_password(&"x".repeat(129)).is_err());
    }

    #[test]
    fn error_codes_are_snake_case() {
        let json = serde_json::to_string(&ErrorCode::WrongCredentials).unwrap();
        assert_eq!(json, r#""wrong_credentials""#);
    }
}

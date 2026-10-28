//! Types and game logic shared between the web app and the API:
//! request/response DTOs, game rules, scoring, merge logic and validation.

use serde::{Deserialize, Serialize};

pub mod game;
pub mod language;
pub mod matching;
pub mod play;
pub mod pokedex;
pub mod runs;

pub use language::Language;

/// Response body of the user service's health endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    pub status: String,
    pub version: String,
}

impl Health {
    pub fn ok() -> Self {
        Self {
            status: "ok".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

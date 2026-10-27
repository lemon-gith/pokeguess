//! Languages the game's text can be shown in.

use serde::{Deserialize, Serialize};

/// A language the dataset and UI support. Add a variant here (and handle it
/// wherever the compiler asks) to add a language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "en")]
    English,
}

impl Language {
    /// The language's code, as PokeAPI names it in `language.name`
    /// (the ISO 639-1 code for the languages supported so far).
    pub fn code(self) -> &'static str {
        match self {
            Language::English => "en",
        }
    }
}

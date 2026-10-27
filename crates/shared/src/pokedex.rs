//! The `pokedex.json` dataset: everything the game needs about each Pokémon,
//! built ahead of time by `dex-builder` so the game never has to call PokeAPI.

use serde::{Deserialize, Serialize};

/// Bump whenever [`PokedexEntry`] changes shape, so clients can reject (or
/// migrate) a dataset they don't understand.
pub const SCHEMA_VERSION: u32 = 1;

/// Placeholder that replaces a Pokémon's own name in its flavour text.
pub const REDACTED_NAME: &str = "???";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pokedex {
    pub schema_version: u32,
    /// Sorted by national dex number.
    pub pokemon: Vec<PokedexEntry>,
}

/// One species, using its default form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PokedexEntry {
    /// National dex number.
    pub id: u16,
    /// PokeAPI slug, e.g. `mr-mime`.
    pub name: String,
    /// English name, e.g. `Mr. Mime`.
    pub display_name: String,
    /// English genus, e.g. `Mouse Pokémon`.
    pub genus: Option<String>,
    pub generation: u8,
    /// In slot order.
    pub types: Vec<String>,
    pub stats: BaseStats,
    /// In decimetres.
    pub height: u16,
    /// In hectograms.
    pub weight: u16,
    pub color: String,
    pub shape: Option<String>,
    pub habitat: Option<String>,
    pub is_baby: bool,
    pub is_legendary: bool,
    pub is_mythical: bool,
    /// The most recent English Pokédex entry, with the Pokémon's own name
    /// replaced by [`REDACTED_NAME`].
    pub flavor_text: Option<String>,
    pub sprite: Option<String>,
    pub artwork: Option<String>,
    pub cry: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseStats {
    pub hp: u8,
    pub attack: u8,
    pub defense: u8,
    pub special_attack: u8,
    pub special_defense: u8,
    pub speed: u8,
}

impl BaseStats {
    pub fn total(&self) -> u16 {
        [
            self.hp,
            self.attack,
            self.defense,
            self.special_attack,
            self.special_defense,
            self.speed,
        ]
        .iter()
        .map(|&s| u16::from(s))
        .sum()
    }
}

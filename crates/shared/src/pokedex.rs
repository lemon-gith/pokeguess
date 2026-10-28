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

impl Pokedex {
    /// The generations present in the dataset, in order.
    pub fn generations(&self) -> Vec<u8> {
        let mut generations: Vec<u8> = self.pokemon.iter().map(|p| p.generation).collect();
        generations.sort_unstable();
        generations.dedup();
        generations
    }

    pub fn get(&self, id: u16) -> Option<&PokedexEntry> {
        // entries are sorted by id, which is contiguous from 1
        self.pokemon
            .get(usize::from(id).wrapping_sub(1))
            .filter(|p| p.id == id)
            .or_else(|| self.pokemon.iter().find(|p| p.id == id))
    }
}

/// A minimal entry for tests.
#[cfg(test)]
pub(crate) fn sample(id: u16, slug: &str, display_name: &str) -> PokedexEntry {
    PokedexEntry {
        id,
        name: slug.into(),
        display_name: display_name.into(),
        genus: None,
        generation: 1,
        types: vec!["normal".into()],
        stats: BaseStats {
            hp: 1,
            attack: 1,
            defense: 1,
            special_attack: 1,
            special_defense: 1,
            speed: 1,
        },
        height: 1,
        weight: 1,
        color: "red".into(),
        shape: None,
        habitat: None,
        is_baby: false,
        is_legendary: false,
        is_mythical: false,
        flavor_text: None,
        sprite: None,
        artwork: None,
        cry: None,
    }
}

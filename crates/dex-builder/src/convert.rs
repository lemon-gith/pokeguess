//! Turns raw PokeAPI responses into compact [`PokedexEntry`] values.

use anyhow::{Context, Result, anyhow};
use pokeapi::{Pokemon, PokemonSpecies, resource_id};
use regex::Regex;
use shared::Language;
use shared::pokedex::{BaseStats, PokedexEntry, REDACTED_NAME};

/// Builds an entry with its text (name, genus, flavour text) in `language`.
pub fn entry(
    species: &PokemonSpecies,
    pokemon: &Pokemon,
    language: Language,
) -> Result<PokedexEntry> {
    let display_name = species
        .names
        .iter()
        .find(|n| n.language.name == language.code())
        .map_or_else(|| species.name.clone(), |n| n.name.clone());
    let genus = species
        .genera
        .iter()
        .find(|g| g.language.name == language.code())
        .map(|g| g.genus.clone());
    let flavor_text = latest_flavor_text(species, language)
        .map(|text| redact_name(&clean_flavor_text(text), &display_name));

    let mut types = pokemon.types.clone();
    types.sort_by_key(|t| t.slot);

    Ok(PokedexEntry {
        id: species.id.try_into().context("species id out of range")?,
        name: species.name.clone(),
        display_name,
        genus,
        generation: species
            .generation
            .id()
            .and_then(|id| id.try_into().ok())
            .ok_or_else(|| anyhow!("bad generation link {}", species.generation.url))?,
        types: types.into_iter().map(|t| t.kind.name).collect(),
        stats: base_stats(pokemon)?,
        height: pokemon.height.try_into().context("height out of range")?,
        weight: pokemon.weight.try_into().context("weight out of range")?,
        color: species.color.name.clone(),
        shape: species.shape.as_ref().map(|s| s.name.clone()),
        habitat: species.habitat.as_ref().map(|h| h.name.clone()),
        is_baby: species.is_baby,
        is_legendary: species.is_legendary,
        is_mythical: species.is_mythical,
        flavor_text,
        sprite: pokemon.sprites.front_default.clone(),
        artwork: pokemon.sprites.other.official_artwork.front_default.clone(),
        cry: pokemon.cries.latest.clone(),
    })
}

/// The entry in `language` from the most recent game version.
fn latest_flavor_text(species: &PokemonSpecies, language: Language) -> Option<&str> {
    species
        .flavor_text_entries
        .iter()
        .filter(|f| f.language.name == language.code())
        .max_by_key(|f| resource_id(&f.version.url).unwrap_or(0))
        .map(|f| f.flavor_text.as_str())
}

fn base_stats(pokemon: &Pokemon) -> Result<BaseStats> {
    let stat = |name: &str| -> Result<u8> {
        let stat = pokemon
            .stats
            .iter()
            .find(|s| s.stat.name == name)
            .ok_or_else(|| anyhow!("{} has no {name} stat", pokemon.name))?;
        stat.base_stat
            .try_into()
            .with_context(|| format!("{} {name} out of range", pokemon.name))
    };
    Ok(BaseStats {
        hp: stat("hp")?,
        attack: stat("attack")?,
        defense: stat("defense")?,
        special_attack: stat("special-attack")?,
        special_defense: stat("special-defense")?,
        speed: stat("speed")?,
    })
}

/// Undoes the in-game text box formatting: hard line breaks, page breaks
/// and soft hyphens, plus the old "POKéMON" spelling.
fn clean_flavor_text(text: &str) -> String {
    text.replace("\u{ad}\n", "")
        .replace('\u{ad}', "")
        .replace("POKéMON", "Pokémon")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Hides the Pokémon's own name (in any case) so the entry can be a clue,
/// including without a trailing symbol (e.g. "Nidoran" for "Nidoran♀").
fn redact_name(text: &str, name: &str) -> String {
    let text = redact_exact(text, name);
    let base = name.trim_end_matches(|c: char| !c.is_alphanumeric());
    if base.is_empty() || base == name {
        text
    } else {
        redact_exact(&text, base)
    }
}

fn redact_exact(text: &str, name: &str) -> String {
    let is_word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    let start = if is_word(name.chars().next()) {
        r"\b"
    } else {
        ""
    };
    let end = if is_word(name.chars().last()) {
        r"\b"
    } else {
        ""
    };
    let pattern = format!("(?i){start}{}{end}", regex::escape(name));
    let re = Regex::new(&pattern).expect("escaped name is a valid regex");
    re.replace_all(text, REDACTED_NAME).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture<T: serde::de::DeserializeOwned>(name: &str) -> T {
        let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn converts_pikachu() {
        let species: PokemonSpecies = fixture("species-25.json");
        let pokemon: Pokemon = fixture("pokemon-25.json");
        let entry = entry(&species, &pokemon, Language::English).unwrap();

        assert_eq!(entry.id, 25);
        assert_eq!(entry.display_name, "Pikachu");
        assert_eq!(entry.genus.as_deref(), Some("Mouse Pokémon"));
        assert_eq!(entry.generation, 1);
        assert_eq!(entry.types, ["electric"]);
        assert_eq!(entry.stats.hp, 35);
        assert_eq!(entry.stats.total(), 320);
        assert_eq!(entry.habitat.as_deref(), Some("forest"));
        // the fixture's latest English entry is from Legends: Arceus
        let text = entry.flavor_text.unwrap();
        assert!(text.starts_with("Possesses cheek sacs"), "{text}");
        assert!(!text.contains('\n'));
    }

    #[test]
    fn cleans_text_box_formatting() {
        assert_eq!(
            clean_flavor_text(
                "When several of\nthese POKéMON\ngather, their\x0celec\u{ad}\ntricity"
            ),
            "When several of these Pokémon gather, their electricity"
        );
    }

    #[test]
    fn redacts_names_case_insensitively() {
        assert_eq!(
            redact_name("PIKACHU that live together. Pikachu's cheeks", "Pikachu"),
            "??? that live together. ???'s cheeks"
        );
    }

    #[test]
    fn redacts_names_without_their_gender_symbol() {
        assert_eq!(
            redact_name("Nidoran♀ is gentle. A female Nidoran’s horn", "Nidoran♀"),
            "??? is gentle. A female ???’s horn"
        );
    }

    #[test]
    fn redacts_whole_words_only() {
        assert_eq!(
            redact_name("Mew is said to be Mewtwo's ancestor.", "Mew"),
            "??? is said to be Mewtwo's ancestor."
        );
    }

    #[test]
    fn redacts_names_with_symbols() {
        assert_eq!(
            redact_name("Nidoran♀ has barbs.", "Nidoran♀"),
            "??? has barbs."
        );
        assert_eq!(redact_name("Mr. Mime mimes.", "Mr. Mime"), "??? mimes.");
    }
}

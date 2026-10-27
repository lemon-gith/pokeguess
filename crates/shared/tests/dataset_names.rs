//! Guess matching against the real `pokedex.json`.

use std::collections::{BTreeSet, HashMap};

use shared::Language;
use shared::matching::{find, matches, normalize};
use shared::pokedex::Pokedex;

fn pokedex() -> Pokedex {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/web/data/pokedex.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn every_name_matches_exactly_one_pokemon() {
    let dex = pokedex();
    let mut by_name: HashMap<String, BTreeSet<u16>> = HashMap::new();
    for entry in &dex.pokemon {
        assert!(matches(&entry.display_name, entry, Language::English));
        for name in [&entry.display_name, &entry.name] {
            by_name
                .entry(normalize(name, Language::English))
                .or_default()
                .insert(entry.id);
        }
    }
    for (name, ids) in by_name {
        assert_eq!(ids.len(), 1, "{name:?} names {ids:?}");
    }
}

#[test]
fn accepts_loose_spellings() {
    let dex = pokedex();
    for (guess, expected) in [
        ("nidoran f", "Nidoran♀"),
        ("Nidoran Female", "Nidoran♀"),
        ("nidoran m", "Nidoran♂"),
        ("farfetchd", "Farfetch’d"),
        ("farfetch'd", "Farfetch’d"),
        ("sirfetchd", "Sirfetch’d"),
        ("mr mime", "Mr. Mime"),
        ("mime jr", "Mime Jr."),
        ("type null", "Type: Null"),
        ("flabebe", "Flabébé"),
        ("ho oh", "Ho-Oh"),
        ("porygon z", "Porygon-Z"),
        ("jangmo o", "Jangmo-o"),
        ("tapu koko", "Tapu Koko"),
        ("great tusk", "Great Tusk"),
    ] {
        let found = find(guess, &dex.pokemon, Language::English).map(|p| p.display_name.as_str());
        assert_eq!(found, Some(expected), "{guess:?}");
    }
}

//! Forgiving guess matching, so players don't need to type accents,
//! punctuation or gender symbols exactly.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::Language;
use crate::pokedex::PokedexEntry;

/// Whether `guess` names `entry`, by display name or PokeAPI slug.
pub fn matches(guess: &str, entry: &PokedexEntry, language: Language) -> bool {
    let guess = normalize(guess, language);
    !guess.is_empty()
        && (guess == normalize(&entry.display_name, language)
            || guess == normalize(&entry.name, language))
}

/// The Pokémon a guess refers to, if any.
pub fn find<'a>(
    guess: &str,
    pokemon: &'a [PokedexEntry],
    language: Language,
) -> Option<&'a PokedexEntry> {
    pokemon.iter().find(|entry| matches(guess, entry, language))
}

/// Reduces a name to the form guesses are compared in: two names that
/// normalise the same are treated as the same guess.
pub fn normalize(text: &str, language: Language) -> String {
    match language {
        Language::English => normalize_latin(text),
    }
}

/// Lowercase ASCII letters and digits only: accents are dropped
/// ("Flabébé" → "flabebe"), ♀/♂ and the words "female"/"male" become f/m
/// ("Nidoran♀", "nidoran female" and the slug "nidoran-f" → "nidoranf"), and
/// spaces and punctuation are ignored ("Mr. Mime" → "mrmime").
fn normalize_latin(text: &str) -> String {
    let text: String = text
        .nfd()
        .filter(|&c| !is_combining_mark(c))
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            '♀' => " f ".to_string(),
            '♂' => " m ".to_string(),
            c => c.to_string(),
        })
        .collect();
    text.split(|c: char| !c.is_alphanumeric())
        .map(|word| match word {
            "female" => "f",
            "male" => "m",
            word => word,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pokedex::sample as entry;

    fn accepts(display_name: &str, slug: &str, guesses: &[&str]) {
        let entry = entry(1, slug, display_name);
        for guess in guesses {
            assert!(
                matches(guess, &entry, Language::English),
                "{guess:?} should match {display_name}"
            );
        }
    }

    #[test]
    fn ignores_case_and_surrounding_space() {
        accepts("Pikachu", "pikachu", &["pikachu", "PIKACHU", "  Pikachu "]);
    }

    #[test]
    fn ignores_accents() {
        accepts("Flabébé", "flabebe", &["Flabébé", "flabebe", "FLABEBE"]);
    }

    #[test]
    fn ignores_punctuation_and_spacing() {
        accepts(
            "Mr. Mime",
            "mr-mime",
            &["mr mime", "mr. mime", "mrmime", "Mr-Mime"],
        );
        accepts(
            "Farfetch’d",
            "farfetchd",
            &["farfetch'd", "farfetchd", "Farfetch’d"],
        );
        accepts(
            "Type: Null",
            "type-null",
            &["type null", "typenull", "Type: Null"],
        );
        accepts(
            "Porygon-Z",
            "porygon-z",
            &["porygon z", "porygonz", "porygon-z"],
        );
    }

    #[test]
    fn accepts_gender_spelled_out() {
        accepts(
            "Nidoran♀",
            "nidoran-f",
            &[
                "Nidoran♀",
                "nidoran f",
                "nidoran-f",
                "Nidoran Female",
                "nidoranf",
            ],
        );
        accepts(
            "Nidoran♂",
            "nidoran-m",
            &["Nidoran♂", "nidoran m", "nidoran male"],
        );
    }

    #[test]
    fn rejects_other_names() {
        let female = entry(29, "nidoran-f", "Nidoran♀");
        for guess in ["nidoran", "nidoran m", "nidorina", "", "   ", "?"] {
            assert!(!matches(guess, &female, Language::English), "{guess:?}");
        }
    }

    #[test]
    fn finds_the_named_pokemon() {
        let dex = [
            entry(29, "nidoran-f", "Nidoran♀"),
            entry(32, "nidoran-m", "Nidoran♂"),
        ];
        let found = find("nidoran male", &dex, Language::English).map(|e| e.id);
        assert_eq!(found, Some(32));
        assert!(find("nidoran", &dex, Language::English).is_none());
    }
}

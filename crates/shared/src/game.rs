//! The modes, the clues they reveal, and which Pokémon a run asks about.
//! Playing a run (lives, guesses, hints) is in [`crate::play`].
//!
//! Randomness is passed in as a `pick(n)` function returning an index below
//! `n`, so this module stays deterministic and testable.

use serde::{Deserialize, Serialize};

use crate::pokedex::{Pokedex, PokedexEntry};

/// How many facts the [`Clue::Facts`] clue shows.
pub const FACT_COUNT: usize = 3;

/// Clues per Pokémon: the first is free, the rest are bought as hints.
pub const CLUES_PER_MODE: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Mode {
    Easy,
    Normal,
    Hard,
    Expert,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Easy, Mode::Normal, Mode::Hard, Mode::Expert];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Easy => "Easy",
            Mode::Normal => "Normal",
            Mode::Hard => "Hard",
            Mode::Expert => "Expert",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Mode::Easy => "The picture. Hints: the Pokédex entry, then the first letter. 5 lives.",
            Mode::Normal => "A shadow. Hints: the Pokédex entry, then the first letter.",
            Mode::Hard => {
                "The Pokédex entry. Hints: a shadow, then the picture. No name suggestions."
            }
            Mode::Expert => {
                "Three facts. Hints: the Pokédex entry, then a shadow. Only 3 balls, hints cost a whole life, and no name suggestions or extra lives."
            }
        }
    }

    /// The free clue, then the clues hints reveal, in order.
    pub fn clues(self) -> [Clue; CLUES_PER_MODE] {
        match self {
            Mode::Easy => [Clue::Picture, Clue::Description, Clue::FirstLetter],
            Mode::Normal => [Clue::Shadow, Clue::Description, Clue::FirstLetter],
            Mode::Hard => [Clue::Description, Clue::Shadow, Clue::Picture],
            Mode::Expert => [Clue::Facts, Clue::Description, Clue::Shadow],
        }
    }

    pub fn starting_lives(self) -> usize {
        match self {
            Mode::Easy => 5,
            Mode::Normal | Mode::Hard | Mode::Expert => 3,
        }
    }

    /// What a catch is worth on each guess. There's one ball (guess) per
    /// entry: missing with the last one makes the Pokémon flee.
    pub fn catch_points(self) -> &'static [u32] {
        match self {
            Mode::Easy | Mode::Normal | Mode::Hard => &[8, 5, 3, 2, 1],
            Mode::Expert => &[8, 5, 3],
        }
    }

    pub fn balls(self) -> usize {
        self.catch_points().len()
    }

    /// Points lost when a Pokémon flees after the last ball.
    pub fn flee_penalty(self) -> i32 {
        match self {
            Mode::Expert => 2,
            Mode::Easy | Mode::Normal | Mode::Hard => 0,
        }
    }

    /// What a hint costs, in half lives.
    pub fn hint_cost(self) -> u32 {
        match self {
            Mode::Expert => 2,
            Mode::Easy | Mode::Normal | Mode::Hard => 1,
        }
    }

    /// Whether a catch streak can win back a life.
    pub fn earns_lives(self) -> bool {
        self != Mode::Expert
    }

    /// Whether the guess box suggests Pokémon names. Off in the harder
    /// modes, where spelling the name is part of the challenge.
    pub fn suggests_names(self) -> bool {
        matches!(self, Mode::Easy | Mode::Normal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clue {
    Picture,
    Shadow,
    Description,
    Facts,
    FirstLetter,
}

/// One piece of data the [`Clue::Facts`] clue can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fact {
    Types,
    Generation,
    Color,
    Shape,
    Habitat,
    Height,
    Weight,
    StatTotal,
}

impl Fact {
    pub const ALL: [Fact; 8] = [
        Fact::Types,
        Fact::Generation,
        Fact::Color,
        Fact::Shape,
        Fact::Habitat,
        Fact::Height,
        Fact::Weight,
        Fact::StatTotal,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Fact::Types => "Type",
            Fact::Generation => "Generation",
            Fact::Color => "Colour",
            Fact::Shape => "Shape",
            Fact::Habitat => "Habitat",
            Fact::Height => "Height",
            Fact::Weight => "Weight",
            Fact::StatTotal => "Base stat total",
        }
    }

    /// The fact for `entry`, or `None` if the dataset doesn't have it.
    pub fn value(self, entry: &PokedexEntry) -> Option<String> {
        Some(match self {
            Fact::Types => entry
                .types
                .iter()
                .map(|t| title_case(t))
                .collect::<Vec<_>>()
                .join(" / "),
            Fact::Generation => entry.generation.to_string(),
            Fact::Color => title_case(&entry.color),
            Fact::Shape => title_case(entry.shape.as_deref()?),
            Fact::Habitat => title_case(entry.habitat.as_deref()?),
            Fact::Height => format!("{:.1} m", f64::from(entry.height) / 10.0),
            Fact::Weight => format!("{:.1} kg", f64::from(entry.weight) / 10.0),
            Fact::StatTotal => entry.stats.total().to_string(),
        })
    }
}

/// `"rough-terrain"` → `"Rough terrain"`.
fn title_case(slug: &str) -> String {
    let text = slug.replace('-', " ");
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => text,
    }
}

/// Picks [`FACT_COUNT`] different facts that `entry` has.
pub fn choose_facts(entry: &PokedexEntry, mut pick: impl FnMut(usize) -> usize) -> Vec<Fact> {
    let mut available: Vec<Fact> = Fact::ALL
        .into_iter()
        .filter(|fact| fact.value(entry).is_some())
        .collect();
    let mut chosen = Vec::with_capacity(FACT_COUNT);
    while chosen.len() < FACT_COUNT && !available.is_empty() {
        let index = pick(available.len()).min(available.len() - 1);
        chosen.push(available.remove(index));
    }
    chosen
}

/// Which Pokémon a run can ask about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSettings {
    pub mode: Mode,
    /// Sorted, without duplicates.
    pub generations: Vec<u8>,
}

impl RunSettings {
    /// Normal mode with every generation in the dataset.
    pub fn new(dex: &Pokedex) -> Self {
        Self {
            mode: Mode::Normal,
            generations: dex.generations(),
        }
    }

    /// The IDs of the Pokémon in the chosen generations.
    pub fn pool(&self, dex: &Pokedex) -> Vec<u16> {
        dex.pokemon
            .iter()
            .filter(|p| self.generations.contains(&p.generation))
            .map(|p| p.id)
            .collect()
    }
}

/// Generations as compact ranges, e.g. `[1, 2, 3, 5]` → `"1–3, 5"`.
pub fn format_generations(generations: &[u8]) -> String {
    let mut ranges: Vec<(u8, u8)> = Vec::new();
    for &generation in generations {
        match ranges.last_mut() {
            Some((_, end)) if *end + 1 == generation => *end = generation,
            _ => ranges.push((generation, generation)),
        }
    }
    ranges
        .iter()
        .map(|&(start, end)| match end - start {
            0 => start.to_string(),
            1 => format!("{start}, {end}"),
            _ => format!("{start}–{end}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Picks a target from `pool`, avoiding `seen` until every Pokémon has come up.
pub fn pick_target(pool: &[u16], seen: &[u16], pick: impl FnOnce(usize) -> usize) -> Option<u16> {
    let unseen: Vec<u16> = pool
        .iter()
        .copied()
        .filter(|id| !seen.contains(id))
        .collect();
    let candidates = if unseen.is_empty() { pool } else { &unseen };
    if candidates.is_empty() {
        return None;
    }
    let index = pick(candidates.len()).min(candidates.len() - 1);
    Some(candidates[index])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pokedex::sample;

    #[test]
    fn hints_never_repeat_a_clue() {
        for mode in Mode::ALL {
            let clues = mode.clues();
            for (i, clue) in clues.iter().enumerate() {
                assert!(!clues[i + 1..].contains(clue), "{mode:?} repeats {clue:?}");
            }
        }
    }

    #[test]
    fn facts_are_distinct_and_available() {
        let entry = sample(1, "bulbasaur", "Bulbasaur");
        // always take the first remaining fact
        let facts = choose_facts(&entry, |_| 0);
        assert_eq!(facts, [Fact::Types, Fact::Generation, Fact::Color]);
        // the sample has no shape or habitat, so they're never chosen
        let facts = choose_facts(&entry, |n| n - 1);
        assert_eq!(facts, [Fact::StatTotal, Fact::Weight, Fact::Height]);
    }

    #[test]
    fn formats_facts() {
        let mut entry = sample(25, "pikachu", "Pikachu");
        entry.types = vec!["electric".into(), "flying".into()];
        entry.height = 4;
        entry.weight = 60;
        entry.shape = Some("quadruped".into());
        entry.habitat = Some("rough-terrain".into());
        let value = |fact: Fact| fact.value(&entry).unwrap();
        assert_eq!(value(Fact::Types), "Electric / Flying");
        assert_eq!(value(Fact::Generation), "1");
        assert_eq!(value(Fact::Height), "0.4 m");
        assert_eq!(value(Fact::Weight), "6.0 kg");
        assert_eq!(value(Fact::Habitat), "Rough terrain");
        assert_eq!(value(Fact::StatTotal), "6");
    }

    #[test]
    fn formats_generation_ranges() {
        assert_eq!(format_generations(&[1, 2, 3, 5]), "1–3, 5");
        assert_eq!(format_generations(&[1, 2, 4, 6, 7, 8, 9]), "1, 2, 4, 6–9");
        assert_eq!(format_generations(&[3]), "3");
        assert_eq!(format_generations(&[]), "");
    }

    #[test]
    fn targets_avoid_repeats_until_the_pool_runs_out() {
        let pool = [1, 2, 3];
        assert_eq!(pick_target(&pool, &[1, 2], |_| 0), Some(3));
        assert_eq!(pick_target(&pool, &[1, 2, 3], |_| 1), Some(2));
        assert_eq!(pick_target(&[], &[], |_| 0), None);
    }

    #[test]
    fn pool_only_includes_chosen_generations() {
        let mut gen2 = sample(152, "chikorita", "Chikorita");
        gen2.generation = 2;
        let dex = Pokedex {
            schema_version: crate::pokedex::SCHEMA_VERSION,
            pokemon: vec![sample(1, "bulbasaur", "Bulbasaur"), gen2],
        };
        let mut settings = RunSettings::new(&dex);
        assert_eq!(settings.generations, [1, 2]);
        settings.generations = vec![2];
        assert_eq!(settings.pool(&dex), [152]);
    }
}

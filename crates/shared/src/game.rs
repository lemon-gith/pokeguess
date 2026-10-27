//! The rules of a round: which clues each mode reveals, and how guesses play out.
//!
//! Randomness is passed in as a `pick(n)` function returning an index below
//! `n`, so this module stays deterministic and testable.

use serde::{Deserialize, Serialize};

use crate::pokedex::{Pokedex, PokedexEntry};

/// Wrong guesses allowed before a round is lost.
pub const MAX_GUESSES: usize = 3;

/// Points for catching a Pokémon on the first, second, third… guess.
/// Guesses beyond the end of the list score nothing.
pub const POINTS: [u32; 7] = [10, 7, 5, 4, 3, 2, 1];

/// How many facts the [`Clue::Facts`] clue shows.
pub const FACT_COUNT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Mode {
    Baby,
    Easy,
    Medium,
    Hard,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Baby, Mode::Easy, Mode::Medium, Mode::Hard];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Baby => "Baby",
            Mode::Easy => "Easy",
            Mode::Medium => "Medium",
            Mode::Hard => "Hard",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Mode::Baby => "Just the picture. You've got this!",
            Mode::Easy => "A shadow, then the Pokédex entry, then the picture.",
            Mode::Medium => {
                "The Pokédex entry, then a shadow on your last guess. No name suggestions."
            }
            Mode::Hard => {
                "Three facts, then the Pokédex entry, then a shadow. No name suggestions."
            }
        }
    }

    /// Whether the guess box suggests Pokémon names. Off in the harder
    /// modes, where spelling the name is part of the challenge.
    pub fn suggests_names(self) -> bool {
        matches!(self, Mode::Baby | Mode::Easy)
    }

    /// The clue revealed before each guess; `None` means nothing new.
    pub fn clues(self) -> [Option<Clue>; MAX_GUESSES] {
        match self {
            Mode::Baby => [Some(Clue::Picture), None, None],
            Mode::Easy => [
                Some(Clue::Shadow),
                Some(Clue::Description),
                Some(Clue::Picture),
            ],
            Mode::Medium => [Some(Clue::Description), None, Some(Clue::Shadow)],
            Mode::Hard => [
                Some(Clue::Facts),
                Some(Clue::Description),
                Some(Clue::Shadow),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clue {
    Picture,
    Shadow,
    Description,
    Facts,
}

/// One piece of data the [`Clue::Facts`] clue can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Easy mode with every generation in the dataset.
    pub fn new(dex: &Pokedex) -> Self {
        Self {
            mode: Mode::Easy,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Guessed,
    Missed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessResult {
    Correct,
    /// Wrong, with guesses left.
    Wrong,
    /// Wrong, and that was the last guess.
    Missed,
    /// Not the name of any Pokémon; doesn't use up a guess.
    Unknown,
    /// Already guessed this round; doesn't use up a guess.
    Repeated,
    /// The round is already over.
    Finished,
}

/// One Pokémon to guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Round {
    pub target: u16,
    /// Shown by [`Clue::Facts`].
    pub facts: Vec<Fact>,
    pub wrong_guesses: Vec<u16>,
    pub outcome: Option<Outcome>,
}

impl Round {
    pub fn new(target: u16, facts: Vec<Fact>) -> Self {
        Self {
            target,
            facts,
            wrong_guesses: Vec::new(),
            outcome: None,
        }
    }

    /// Plays a guess, given the ID of the Pokémon it named (if any).
    pub fn guess(&mut self, guessed: Option<u16>) -> GuessResult {
        if self.outcome.is_some() {
            return GuessResult::Finished;
        }
        let Some(id) = guessed else {
            return GuessResult::Unknown;
        };
        if id == self.target {
            self.outcome = Some(Outcome::Guessed);
            return GuessResult::Correct;
        }
        if self.wrong_guesses.contains(&id) {
            return GuessResult::Repeated;
        }
        self.wrong_guesses.push(id);
        if self.wrong_guesses.len() >= MAX_GUESSES {
            self.outcome = Some(Outcome::Missed);
            GuessResult::Missed
        } else {
            GuessResult::Wrong
        }
    }

    pub fn guesses_left(&self) -> usize {
        MAX_GUESSES.saturating_sub(self.wrong_guesses.len())
    }

    /// Every clue revealed so far, in the order they were revealed.
    pub fn clues(&self, mode: Mode) -> Vec<Clue> {
        let stage = self.wrong_guesses.len().min(MAX_GUESSES - 1);
        mode.clues()[..=stage].iter().flatten().copied().collect()
    }

    /// Points for this round, from [`POINTS`] by the guess that got it.
    pub fn points(&self) -> u32 {
        match self.outcome {
            Some(Outcome::Guessed) => POINTS.get(self.wrong_guesses.len()).copied().unwrap_or(0),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pokedex::sample;

    #[test]
    fn every_mode_starts_with_a_clue() {
        for mode in Mode::ALL {
            assert!(mode.clues()[0].is_some(), "{mode:?}");
        }
    }

    #[test]
    fn clues_accumulate_with_wrong_guesses() {
        let mut round = Round::new(25, vec![]);
        assert_eq!(round.clues(Mode::Easy), [Clue::Shadow]);
        round.guess(Some(1));
        assert_eq!(round.clues(Mode::Easy), [Clue::Shadow, Clue::Description]);
        round.guess(Some(2));
        assert_eq!(
            round.clues(Mode::Easy),
            [Clue::Shadow, Clue::Description, Clue::Picture]
        );
        // Medium reveals nothing new on the second guess
        assert_eq!(round.clues(Mode::Medium), [Clue::Description, Clue::Shadow]);
    }

    #[test]
    fn three_wrong_guesses_lose_the_round() {
        let mut round = Round::new(25, vec![]);
        assert_eq!(round.guess(Some(1)), GuessResult::Wrong);
        assert_eq!(round.guess(Some(2)), GuessResult::Wrong);
        assert_eq!(round.guesses_left(), 1);
        assert_eq!(round.guess(Some(3)), GuessResult::Missed);
        assert_eq!(round.outcome, Some(Outcome::Missed));
        assert_eq!(round.points(), 0);
        assert_eq!(round.guess(Some(25)), GuessResult::Finished);
    }

    #[test]
    fn unknown_and_repeated_guesses_are_free() {
        let mut round = Round::new(25, vec![]);
        assert_eq!(round.guess(None), GuessResult::Unknown);
        assert_eq!(round.guess(Some(1)), GuessResult::Wrong);
        assert_eq!(round.guess(Some(1)), GuessResult::Repeated);
        assert_eq!(round.guesses_left(), 2);
    }

    #[test]
    fn fewer_guesses_score_more() {
        let mut first_try = Round::new(25, vec![]);
        first_try.guess(Some(25));
        assert_eq!(first_try.points(), 10);

        let mut second_try = Round::new(25, vec![]);
        second_try.guess(Some(1));
        second_try.guess(Some(25));
        assert_eq!(second_try.points(), 7);

        let mut last_try = Round::new(25, vec![]);
        last_try.guess(Some(1));
        last_try.guess(Some(2));
        assert_eq!(last_try.guess(Some(25)), GuessResult::Correct);
        assert_eq!(last_try.points(), 5);
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

//! Playing a run: lives, guesses, hints and running away.
//!
//! A run starts with its mode's lives. Each wrong guess, hint or run away
//! costs one, and the run ends when they're gone. Catching a Pokémon scores
//! [`CATCH_POINTS`], less the mode's penalty for each wrong guess on it.

use serde::{Deserialize, Serialize};

use crate::game::{CLUES_PER_MODE, Clue, Fact, Mode};

/// Points for catching a Pokémon without a wrong guess.
pub const CATCH_POINTS: u32 = 10;

/// Correct guesses in a row that win back a life (in modes that allow it).
pub const STREAK_FOR_LIFE: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Life {
    PokeBall,
    /// A life won back with a streak.
    PremierBall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Caught,
    RanAway,
    /// The last life went on this Pokémon, ending the run.
    OutOfLives,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessResult {
    Caught {
        points: u32,
        /// The catch completed a streak and won back a life.
        extra_life: bool,
    },
    /// Wrong, with lives left.
    Wrong,
    /// Wrong, and that was the last life.
    OutOfLives,
    /// Not the name of any Pokémon; costs nothing.
    Unknown,
    /// Already guessed for this Pokémon; costs nothing.
    Repeated,
    /// This Pokémon is already done.
    Finished,
}

/// One Pokémon to guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Round {
    pub target: u16,
    /// Shown by [`Clue::Facts`].
    pub facts: Vec<Fact>,
    pub wrong_guesses: Vec<u16>,
    /// Hints bought for this Pokémon.
    pub hints: usize,
    pub outcome: Option<Outcome>,
}

impl Round {
    pub fn new(target: u16, facts: Vec<Fact>) -> Self {
        Self {
            target,
            facts,
            wrong_guesses: Vec::new(),
            hints: 0,
            outcome: None,
        }
    }

    /// The free clue plus any bought with hints, in order.
    pub fn clues(&self, mode: Mode) -> Vec<Clue> {
        mode.clues()[..=self.hints.min(CLUES_PER_MODE - 1)].to_vec()
    }

    pub fn hints_left(&self) -> usize {
        CLUES_PER_MODE - 1 - self.hints.min(CLUES_PER_MODE - 1)
    }

    /// What catching this Pokémon is worth now.
    pub fn catch_points(&self, mode: Mode) -> u32 {
        let penalty = mode.wrong_guess_penalty() * self.wrong_guesses.len() as u32;
        CATCH_POINTS.saturating_sub(penalty)
    }
}

/// A run in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub mode: Mode,
    /// Lives left, oldest first; the last one is lost first.
    pub lives: Vec<Life>,
    pub score: u32,
    pub caught: u32,
    /// Correct guesses in a row; reset by a wrong guess or running away.
    pub streak: u32,
    pub round: Round,
}

impl Run {
    pub fn new(mode: Mode, round: Round) -> Self {
        Self {
            mode,
            lives: vec![Life::PokeBall; mode.starting_lives()],
            score: 0,
            caught: 0,
            streak: 0,
            round,
        }
    }

    pub fn max_lives(&self) -> usize {
        self.mode.starting_lives()
    }

    pub fn is_over(&self) -> bool {
        self.lives.is_empty()
    }

    /// Hints can't spend the last life.
    pub fn can_hint(&self) -> bool {
        self.round.outcome.is_none() && self.round.hints_left() > 0 && self.lives.len() > 1
    }

    /// Whether running away now would end the run.
    pub fn running_away_ends_run(&self) -> bool {
        self.lives.len() <= 1
    }

    /// Plays a guess, given the ID of the Pokémon it named (if any).
    pub fn guess(&mut self, guessed: Option<u16>) -> GuessResult {
        if self.round.outcome.is_some() {
            return GuessResult::Finished;
        }
        let Some(id) = guessed else {
            return GuessResult::Unknown;
        };
        if id == self.round.target {
            let points = self.round.catch_points(self.mode);
            self.round.outcome = Some(Outcome::Caught);
            self.score += points;
            self.caught += 1;
            self.streak += 1;
            let extra_life = self.mode.earns_lives()
                && self.streak.is_multiple_of(STREAK_FOR_LIFE)
                && self.lives.len() < self.max_lives();
            if extra_life {
                self.lives.push(Life::PremierBall);
            }
            return GuessResult::Caught { points, extra_life };
        }
        if self.round.wrong_guesses.contains(&id) {
            return GuessResult::Repeated;
        }
        self.round.wrong_guesses.push(id);
        self.streak = 0;
        if self.lose_life() {
            GuessResult::Wrong
        } else {
            GuessResult::OutOfLives
        }
    }

    /// Spends a life on the next clue. Returns whether it did.
    pub fn hint(&mut self) -> bool {
        if !self.can_hint() {
            return false;
        }
        self.lose_life();
        self.round.hints += 1;
        true
    }

    /// Spends a life to give up on this Pokémon. Returns whether it did.
    pub fn run_away(&mut self) -> bool {
        if self.round.outcome.is_some() {
            return false;
        }
        self.streak = 0;
        if self.lose_life() {
            self.round.outcome = Some(Outcome::RanAway);
        }
        true
    }

    /// Moves on to the next Pokémon, unless this one is still in play or the
    /// run is over. Returns whether it did.
    pub fn next_round(&mut self, round: Round) -> bool {
        if self.round.outcome.is_none() || self.is_over() {
            return false;
        }
        self.round = round;
        true
    }

    /// Returns whether any lives are left, ending the run if not.
    fn lose_life(&mut self) -> bool {
        self.lives.pop();
        if self.lives.is_empty() {
            self.round.outcome = Some(Outcome::OutOfLives);
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET: u16 = 25;

    fn run(mode: Mode) -> Run {
        Run::new(mode, Round::new(TARGET, vec![]))
    }

    /// Catches the current Pokémon and moves on to another.
    fn catch_and_continue(run: &mut Run) -> GuessResult {
        let result = run.guess(Some(TARGET));
        assert!(run.next_round(Round::new(TARGET, vec![])));
        result
    }

    #[test]
    fn modes_start_with_their_lives() {
        assert_eq!(run(Mode::Easy).lives.len(), 5);
        for mode in [Mode::Normal, Mode::Hard, Mode::Expert] {
            assert_eq!(run(mode).lives, [Life::PokeBall; 3]);
        }
    }

    #[test]
    fn wrong_guesses_cost_a_life_and_points_but_reveal_nothing() {
        let mut run = run(Mode::Normal);
        assert_eq!(run.guess(Some(1)), GuessResult::Wrong);
        assert_eq!(run.lives.len(), 2);
        assert_eq!(run.round.clues(Mode::Normal), [Clue::Shadow]);
        assert_eq!(run.round.catch_points(Mode::Normal), 8);
        assert_eq!(
            run.guess(Some(TARGET)),
            GuessResult::Caught {
                points: 8,
                extra_life: false
            }
        );
        assert_eq!(run.score, 8);
    }

    #[test]
    fn penalties_depend_on_the_mode() {
        for (mode, after_one_wrong) in [
            (Mode::Easy, 9),
            (Mode::Normal, 8),
            (Mode::Hard, 7),
            (Mode::Expert, 5),
        ] {
            let mut run = run(mode);
            run.guess(Some(1));
            assert_eq!(run.round.catch_points(mode), after_one_wrong, "{mode:?}");
        }
    }

    #[test]
    fn catch_points_never_go_below_zero() {
        let mut run = run(Mode::Expert);
        run.guess(Some(1));
        run.guess(Some(2));
        assert_eq!(
            run.guess(Some(TARGET)),
            GuessResult::Caught {
                points: 0,
                extra_life: false
            }
        );
        assert_eq!(run.caught, 1);
    }

    #[test]
    fn hints_cost_a_life_but_not_points() {
        let mut run = run(Mode::Hard);
        assert!(run.hint());
        assert_eq!(run.lives.len(), 2);
        assert_eq!(
            run.round.clues(Mode::Hard),
            [Clue::Description, Clue::Shadow]
        );
        assert_eq!(run.round.catch_points(Mode::Hard), CATCH_POINTS);
    }

    #[test]
    fn hints_run_out_and_never_spend_the_last_life() {
        let mut run = run(Mode::Easy);
        assert!(run.hint());
        assert!(run.hint());
        assert_eq!(
            run.round.clues(Mode::Easy),
            [Clue::Picture, Clue::Description, Clue::FirstLetter]
        );
        assert!(!run.can_hint(), "no clues left");

        let mut run = self::run(Mode::Normal);
        run.guess(Some(1));
        run.guess(Some(2));
        assert_eq!(run.lives.len(), 1);
        assert!(!run.hint(), "last life");
        assert_eq!(run.lives.len(), 1);
    }

    #[test]
    fn running_away_costs_a_life_and_moves_on() {
        let mut run = run(Mode::Normal);
        assert!(!run.running_away_ends_run());
        assert!(run.run_away());
        assert_eq!(run.round.outcome, Some(Outcome::RanAway));
        assert_eq!((run.lives.len(), run.score), (2, 0));
        assert!(run.next_round(Round::new(1, vec![])));
    }

    #[test]
    fn running_away_on_the_last_life_ends_the_run() {
        let mut run = run(Mode::Normal);
        run.guess(Some(1));
        run.guess(Some(2));
        assert!(run.running_away_ends_run());
        assert!(run.run_away());
        assert_eq!(run.round.outcome, Some(Outcome::OutOfLives));
        assert!(run.is_over());
        assert!(!run.next_round(Round::new(1, vec![])));
    }

    #[test]
    fn running_out_of_lives_ends_the_run() {
        let mut run = run(Mode::Hard);
        assert_eq!(run.guess(Some(1)), GuessResult::Wrong);
        assert_eq!(run.guess(Some(2)), GuessResult::Wrong);
        assert_eq!(run.guess(Some(3)), GuessResult::OutOfLives);
        assert!(run.is_over());
        assert_eq!(run.guess(Some(TARGET)), GuessResult::Finished);
    }

    #[test]
    fn lives_carry_over_between_pokemon() {
        let mut run = run(Mode::Normal);
        run.guess(Some(1));
        catch_and_continue(&mut run);
        assert_eq!(run.lives.len(), 2);
        assert!(run.round.wrong_guesses.is_empty());
    }

    #[test]
    fn unknown_and_repeated_guesses_are_free() {
        let mut run = run(Mode::Normal);
        assert_eq!(run.guess(None), GuessResult::Unknown);
        assert_eq!(run.guess(Some(1)), GuessResult::Wrong);
        assert_eq!(run.guess(Some(1)), GuessResult::Repeated);
        assert_eq!(run.lives.len(), 2);
    }

    #[test]
    fn a_streak_of_ten_wins_back_a_premier_ball() {
        let mut run = run(Mode::Normal);
        run.guess(Some(1));
        for _ in 0..9 {
            catch_and_continue(&mut run);
        }
        // the wrong guess came first, so this is only the 9th in a row
        assert_eq!(run.lives.len(), 2);
        assert_eq!(
            catch_and_continue(&mut run),
            GuessResult::Caught {
                points: 10,
                extra_life: true
            }
        );
        assert_eq!(
            run.lives,
            [Life::PokeBall, Life::PokeBall, Life::PremierBall]
        );
        // the premier ball is the first life lost
        run.guess(Some(1));
        assert_eq!(run.lives, [Life::PokeBall, Life::PokeBall]);
    }

    #[test]
    fn hints_dont_break_a_streak_but_wrong_guesses_and_running_away_do() {
        let mut run = run(Mode::Normal);
        run.hint();
        catch_and_continue(&mut run);
        assert_eq!(run.streak, 1);
        run.guess(Some(1));
        assert_eq!(run.streak, 0);
        catch_and_continue(&mut run);
        run.run_away();
        assert_eq!(run.streak, 0);
    }

    #[test]
    fn streaks_never_go_past_the_starting_lives() {
        let mut run = run(Mode::Normal);
        for _ in 0..10 {
            catch_and_continue(&mut run);
        }
        assert_eq!(run.lives.len(), 3);
    }

    #[test]
    fn expert_never_wins_lives_back() {
        let mut run = run(Mode::Expert);
        run.hint();
        for _ in 0..10 {
            catch_and_continue(&mut run);
        }
        assert_eq!(run.lives.len(), 2);
    }
}

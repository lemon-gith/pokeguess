//! Playing a run: guesses, hints, running away and lives.
//!
//! Each Pokémon gets its mode's balls (guesses). A catch is worth less with
//! each ball used, and missing with the last one makes the Pokémon flee,
//! costing a life. Hints cost half a life (a whole one on Expert) and
//! running away a whole one. The run ends when the lives are gone.

use serde::{Deserialize, Serialize};

use crate::game::{CLUES_PER_MODE, Clue, Fact, Mode};

/// Correct first guesses in a row that win back a life (in modes that allow it).
pub const STREAK_FOR_LIFE: u32 = 10;

/// What fills a life slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Life {
    PokeBall,
    /// A life won back with a streak.
    PremierBall,
}

/// How full a life slot is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    Full,
    Half,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Caught,
    RanAway,
    /// Missed with every ball.
    Fled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessResult {
    Caught {
        points: u32,
        /// The catch completed a streak and won back a life.
        extra_life: bool,
    },
    /// Wrong, with balls left.
    Wrong,
    /// Wrong with the last ball: the Pokémon fled.
    Fled,
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

    pub fn balls_left(&self, mode: Mode) -> usize {
        mode.balls().saturating_sub(self.wrong_guesses.len())
    }

    /// What catching this Pokémon with the next ball is worth.
    pub fn catch_points(&self, mode: Mode) -> u32 {
        mode.catch_points()
            .get(self.wrong_guesses.len())
            .copied()
            .unwrap_or(0)
    }
}

/// A run in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub mode: Mode,
    /// One per starting life, filled from the left.
    pub slots: Vec<Life>,
    /// Lives left, in halves.
    pub half_lives: u32,
    /// Can go negative on Expert, where fleeing costs points.
    pub score: i32,
    pub caught: u32,
    /// Pokémon caught first time in a row; reset by a wrong guess, running
    /// away or a Pokémon fleeing.
    pub streak: u32,
    pub round: Round,
}

impl Run {
    pub fn new(mode: Mode, round: Round) -> Self {
        let lives = mode.starting_lives();
        Self {
            mode,
            slots: vec![Life::PokeBall; lives],
            half_lives: 2 * lives as u32,
            score: 0,
            caught: 0,
            streak: 0,
            round,
        }
    }

    /// Lives left, e.g. 2.5.
    pub fn lives(&self) -> f32 {
        self.half_lives as f32 / 2.0
    }

    /// How full each life slot is, left to right.
    pub fn fills(&self) -> Vec<Fill> {
        (0..self.slots.len() as u32)
            .map(|i| match self.half_lives.saturating_sub(2 * i) {
                0 => Fill::Empty,
                1 => Fill::Half,
                _ => Fill::Full,
            })
            .collect()
    }

    pub fn is_over(&self) -> bool {
        self.half_lives == 0
    }

    /// Hints can't spend the last of the lives.
    pub fn can_hint(&self) -> bool {
        self.round.outcome.is_none()
            && self.round.hints_left() > 0
            && self.half_lives > self.mode.hint_cost()
    }

    /// Whether running away now would end the run.
    pub fn running_away_ends_run(&self) -> bool {
        self.half_lives <= 2
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
            let first_try = self.round.wrong_guesses.is_empty();
            self.round.outcome = Some(Outcome::Caught);
            self.score += points as i32;
            self.caught += 1;
            if first_try {
                self.streak += 1;
            }
            let extra_life = first_try
                && self.mode.earns_lives()
                && self.streak.is_multiple_of(STREAK_FOR_LIFE)
                && self.gain_life();
            return GuessResult::Caught { points, extra_life };
        }
        if self.round.wrong_guesses.contains(&id) {
            return GuessResult::Repeated;
        }
        self.round.wrong_guesses.push(id);
        self.streak = 0;
        if self.round.balls_left(self.mode) > 0 {
            return GuessResult::Wrong;
        }
        self.round.outcome = Some(Outcome::Fled);
        self.score -= self.mode.flee_penalty();
        self.lose(2);
        GuessResult::Fled
    }

    /// Spends lives on the next clue. Returns whether it did.
    pub fn hint(&mut self) -> bool {
        if !self.can_hint() {
            return false;
        }
        self.lose(self.mode.hint_cost());
        self.round.hints += 1;
        true
    }

    /// Spends a life to give up on this Pokémon. Returns whether it did.
    pub fn run_away(&mut self) -> bool {
        if self.round.outcome.is_some() {
            return false;
        }
        self.round.outcome = Some(Outcome::RanAway);
        self.streak = 0;
        self.lose(2);
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

    fn lose(&mut self, halves: u32) {
        self.half_lives = self.half_lives.saturating_sub(halves);
    }

    /// Restores a whole life, up to the starting lives, refilling the slots
    /// it touches with Premier Balls. Returns whether anything was restored.
    fn gain_life(&mut self) -> bool {
        let max = 2 * self.slots.len() as u32;
        let before = self.half_lives;
        self.half_lives = (before + 2).min(max);
        for slot in (before / 2)..self.half_lives.div_ceil(2) {
            self.slots[slot as usize] = Life::PremierBall;
        }
        self.half_lives > before
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

    /// Misses with `n` balls.
    fn miss(run: &mut Run, n: u16) {
        for id in 1..=n {
            run.guess(Some(id));
        }
    }

    #[test]
    fn modes_start_with_their_lives() {
        assert_eq!(run(Mode::Easy).lives(), 5.0);
        for mode in [Mode::Normal, Mode::Hard, Mode::Expert] {
            let run = run(mode);
            assert_eq!(run.lives(), 3.0);
            assert_eq!(run.fills(), [Fill::Full; 3]);
        }
    }

    #[test]
    fn each_ball_used_lowers_the_catch() {
        for (mode, points) in [
            (Mode::Normal, vec![8, 5, 3, 2, 1]),
            (Mode::Expert, vec![8, 5, 3]),
        ] {
            for (wrong, &expected) in points.iter().enumerate() {
                let mut run = run(mode);
                miss(&mut run, wrong as u16);
                assert_eq!(
                    run.guess(Some(TARGET)),
                    GuessResult::Caught {
                        points: expected,
                        extra_life: false
                    },
                    "{mode:?} after {wrong} wrong"
                );
            }
        }
    }

    #[test]
    fn wrong_guesses_cost_no_lives_or_score_and_reveal_nothing() {
        let mut run = run(Mode::Hard);
        miss(&mut run, 4);
        assert_eq!(run.lives(), 3.0);
        assert_eq!(run.score, 0);
        assert_eq!(run.round.clues(Mode::Hard), [Clue::Description]);
        assert_eq!(run.round.balls_left(Mode::Hard), 1);
    }

    #[test]
    fn missing_with_every_ball_makes_it_flee() {
        let mut run = run(Mode::Normal);
        miss(&mut run, 4);
        assert_eq!(run.guess(Some(99)), GuessResult::Fled);
        assert_eq!(run.round.outcome, Some(Outcome::Fled));
        assert_eq!((run.lives(), run.score), (2.0, 0));
        assert!(run.next_round(Round::new(1, vec![])));
    }

    #[test]
    fn expert_has_three_balls_and_fleeing_costs_two_points() {
        let mut run = run(Mode::Expert);
        assert_eq!(run.guess(Some(1)), GuessResult::Wrong);
        assert_eq!(run.guess(Some(2)), GuessResult::Wrong);
        assert_eq!(run.score, 0, "only fleeing costs points");
        assert_eq!(run.guess(Some(3)), GuessResult::Fled);
        assert_eq!((run.lives(), run.score), (2.0, -2));
    }

    #[test]
    fn hints_cost_half_a_life_or_a_whole_one_on_expert() {
        let mut run = self::run(Mode::Normal);
        assert!(run.hint());
        assert_eq!(run.lives(), 2.5);
        assert_eq!(run.fills(), [Fill::Full, Fill::Full, Fill::Half]);
        assert_eq!(
            run.round.catch_points(Mode::Normal),
            8,
            "hints are free of points"
        );

        let mut run = self::run(Mode::Expert);
        assert!(run.hint());
        assert_eq!(run.lives(), 2.0);
    }

    #[test]
    fn hint_then_run_leaves_one_and_a_half() {
        let mut run = run(Mode::Normal);
        run.hint();
        run.run_away();
        assert_eq!(run.lives(), 1.5);
        assert_eq!(run.fills(), [Fill::Full, Fill::Half, Fill::Empty]);
    }

    #[test]
    fn hints_never_take_the_last_of_the_lives() {
        let mut run = run(Mode::Normal);
        run.half_lives = 2;
        assert!(run.hint(), "1 → 0.5 is allowed");
        assert_eq!(run.lives(), 0.5);
        assert!(!run.can_hint(), "0.5 → 0 isn't");

        let mut run = self::run(Mode::Expert);
        run.half_lives = 2;
        assert!(!run.can_hint(), "a whole-life hint can't use the last life");
    }

    #[test]
    fn hints_run_out() {
        let mut run = run(Mode::Easy);
        assert!(run.hint());
        assert!(run.hint());
        assert_eq!(
            run.round.clues(Mode::Easy),
            [Clue::Picture, Clue::Description, Clue::FirstLetter]
        );
        assert!(!run.can_hint());
    }

    #[test]
    fn running_away_ends_the_run_at_one_life_or_less() {
        let mut run = run(Mode::Normal);
        run.half_lives = 3;
        assert!(!run.running_away_ends_run());
        run.half_lives = 2;
        assert!(run.running_away_ends_run());
        assert!(run.run_away());
        assert!(run.is_over());
        assert!(!run.next_round(Round::new(1, vec![])));

        // half a life left: running away still just ends the run
        let mut run = self::run(Mode::Normal);
        run.half_lives = 1;
        run.run_away();
        assert_eq!(run.half_lives, 0);
    }

    #[test]
    fn fleeing_on_the_last_life_ends_the_run() {
        let mut run = run(Mode::Expert);
        run.half_lives = 2;
        miss(&mut run, 3);
        assert!(run.is_over());
        assert_eq!(run.guess(Some(TARGET)), GuessResult::Finished);
    }

    #[test]
    fn unknown_and_repeated_guesses_are_free() {
        let mut run = run(Mode::Expert);
        assert_eq!(run.guess(None), GuessResult::Unknown);
        assert_eq!(run.guess(Some(1)), GuessResult::Wrong);
        assert_eq!(run.guess(Some(1)), GuessResult::Repeated);
        assert_eq!(run.round.balls_left(Mode::Expert), 2);
    }

    #[test]
    fn a_streak_of_ten_first_try_catches_wins_back_a_premier_ball() {
        let mut run = run(Mode::Normal);
        run.run_away();
        assert!(run.next_round(Round::new(TARGET, vec![])));
        for _ in 0..9 {
            catch_and_continue(&mut run);
        }
        assert_eq!(run.lives(), 2.0);
        assert_eq!(
            catch_and_continue(&mut run),
            GuessResult::Caught {
                points: 8,
                extra_life: true
            }
        );
        assert_eq!(run.lives(), 3.0);
        assert_eq!(
            run.slots,
            [Life::PokeBall, Life::PokeBall, Life::PremierBall]
        );
    }

    #[test]
    fn a_premier_ball_tops_up_a_half_life() {
        let mut run = run(Mode::Normal);
        run.hint(); // 2.5
        for _ in 0..10 {
            catch_and_continue(&mut run);
        }
        assert_eq!(run.lives(), 3.0);
        assert_eq!(run.slots[2], Life::PremierBall);

        // 1.5 → 2.5 refills the half slot and half of the next
        let mut run = self::run(Mode::Normal);
        run.half_lives = 3;
        for _ in 0..10 {
            catch_and_continue(&mut run);
        }
        assert_eq!(run.fills(), [Fill::Full, Fill::Full, Fill::Half]);
        assert_eq!(
            run.slots,
            [Life::PokeBall, Life::PremierBall, Life::PremierBall]
        );
    }

    #[test]
    fn hints_dont_break_a_streak_but_anything_else_does() {
        let mut run = run(Mode::Normal);
        run.hint();
        catch_and_continue(&mut run);
        assert_eq!(run.streak, 1);
        // a catch after a wrong guess still counts as a catch, not a streak
        run.guess(Some(1));
        assert_eq!(run.streak, 0);
        catch_and_continue(&mut run);
        assert_eq!((run.streak, run.caught), (0, 2));
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
        assert_eq!(run.lives(), 3.0);
        assert_eq!(run.slots, [Life::PokeBall; 3]);
    }

    #[test]
    fn expert_never_wins_lives_back() {
        let mut run = run(Mode::Expert);
        run.hint();
        for _ in 0..10 {
            catch_and_continue(&mut run);
        }
        assert_eq!(run.lives(), 2.0);
    }
}

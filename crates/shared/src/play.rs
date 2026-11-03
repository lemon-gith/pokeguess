//! Playing a run: guesses, hints, running away and lives.
//!
//! Each Pokémon gets its mode's balls (guesses). A catch is worth less with
//! each ball used, and missing with the last one makes the Pokémon flee,
//! costing a life. Hints and running away cost half a life (a whole one on
//! Expert). The run ends when the lives are gone.
//!
//! Outside Expert, a hidden streak can win lives back with Premier Balls.
//! It grows with each Pokémon caught on the first ball without a hint, and
//! loses [`STREAK_PENALTY`] (at most once per Pokémon) for a missed first
//! ball or a hint, and [`STREAK_PENALTY`] more for running away. When a
//! Pokémon is caught or flees, a streak of [`STREAK_FOR_PREMIER_BALL`] or
//! more restores up to a life. A Pokémon fleeing with the last of the lives
//! only ends the run if the streak is under [`STREAK_TO_SURVIVE`].

use serde::{Deserialize, Serialize};

use crate::game::{CLUES_PER_MODE, Clue, Fact, Mode};

/// Streak needed for a Premier Ball when a Pokémon is caught or flees.
pub const STREAK_FOR_PREMIER_BALL: u32 = 10;

/// Streak needed to survive a Pokémon fleeing with the last of the lives.
pub const STREAK_TO_SURVIVE: u32 = 25;

/// What a missed first ball or hint (once per Pokémon), or running away,
/// takes off the streak.
pub const STREAK_PENALTY: u32 = 10;

/// What fills a life slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Life {
    PokeBall,
    /// A life won back with a Premier Ball.
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

/// How much a Premier Ball restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restored {
    Half,
    Whole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessResult {
    Caught {
        points: u32,
        premier_ball: Option<Restored>,
    },
    /// Wrong, with balls left.
    Wrong,
    /// Wrong with the last ball: the Pokémon fled.
    Fled { premier_ball: Option<Restored> },
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
    /// A missed first ball or a hint has already cost the streak.
    #[serde(default)]
    pub streak_penalised: bool,
}

impl Round {
    pub fn new(target: u16, facts: Vec<Fact>) -> Self {
        Self {
            target,
            facts,
            wrong_guesses: Vec::new(),
            hints: 0,
            outcome: None,
            streak_penalised: false,
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
    /// Hidden from the player; see the module docs. Always 0 on Expert.
    pub streak: u32,
    pub round: Round,
    /// Hints bought over the whole run.
    #[serde(default)]
    pub hints_bought: u32,
    /// Times the player ran away.
    #[serde(default)]
    pub ran_away: u32,
    /// Lives won back with Premier Balls, in halves.
    #[serde(default)]
    pub half_lives_regained: u32,
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
            hints_bought: 0,
            ran_away: 0,
            half_lives_regained: 0,
        }
    }

    /// Lives left, e.g. 2.5.
    pub fn lives(&self) -> f32 {
        self.half_lives as f32 / 2.0
    }

    fn max_half_lives(&self) -> u32 {
        2 * self.slots.len() as u32
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
        self.half_lives <= self.mode.run_away_cost()
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
            self.score += points as i32;
            self.caught += 1;
            // on the first ball, without a hint
            if self.mode.earns_lives() && !self.round.streak_penalised {
                self.streak += 1;
            }
            let premier_ball = self.premier_ball();
            return GuessResult::Caught {
                points,
                premier_ball,
            };
        }
        if self.round.wrong_guesses.contains(&id) {
            return GuessResult::Repeated;
        }
        self.round.wrong_guesses.push(id);
        self.penalise_streak_once();
        if self.round.balls_left(self.mode) > 0 {
            return GuessResult::Wrong;
        }
        self.round.outcome = Some(Outcome::Fled);
        self.score -= self.mode.flee_penalty();
        let premier_ball = if self.half_lives <= 2 {
            self.survive_last_flee()
        } else {
            self.lose(2);
            self.premier_ball()
        };
        GuessResult::Fled { premier_ball }
    }

    /// Spends lives on the next clue. Returns whether it did.
    pub fn hint(&mut self) -> bool {
        if !self.can_hint() {
            return false;
        }
        self.lose(self.mode.hint_cost());
        self.round.hints += 1;
        self.hints_bought += 1;
        self.penalise_streak_once();
        true
    }

    /// Spends lives to give up on this Pokémon. Returns whether it did.
    pub fn run_away(&mut self) -> bool {
        if self.round.outcome.is_some() {
            return false;
        }
        self.round.outcome = Some(Outcome::RanAway);
        self.ran_away += 1;
        if self.mode.earns_lives() {
            self.streak = self.streak.saturating_sub(STREAK_PENALTY);
        }
        self.lose(self.mode.run_away_cost());
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

    /// A missed first ball or a hint costs the streak, once per Pokémon.
    fn penalise_streak_once(&mut self) {
        if self.mode.earns_lives() && !self.round.streak_penalised {
            self.round.streak_penalised = true;
            self.streak = self.streak.saturating_sub(STREAK_PENALTY);
        }
    }

    /// After a catch or flee: with a big enough streak, restores up to a
    /// life. A whole life uses up the streak; half a life halves it; with
    /// nothing missing, the streak is kept.
    fn premier_ball(&mut self) -> Option<Restored> {
        if !self.mode.earns_lives() || self.streak < STREAK_FOR_PREMIER_BALL {
            return None;
        }
        match self.max_half_lives() - self.half_lives {
            0 => None,
            1 => {
                self.restore(1, self.half_lives);
                self.streak /= 2;
                Some(Restored::Half)
            }
            _ => {
                self.restore(2, self.half_lives);
                self.streak = 0;
                Some(Restored::Whole)
            }
        }
    }

    /// A Pokémon fleeing with the last of the lives (leaving 0 or −½) ends
    /// the run, unless the streak can pay for another life.
    fn survive_last_flee(&mut self) -> Option<Restored> {
        if !self.mode.earns_lives() || self.streak < STREAK_TO_SURVIVE {
            self.lose(2);
            return None;
        }
        // lives are unchanged overall: −1 for fleeing, +1 from the Premier Ball
        let after_fleeing = self.half_lives as i32 - 2;
        self.half_lives_regained += 2;
        self.mark_premier_ball(after_fleeing.unsigned_abs());
        self.streak = 0;
        Some(Restored::Whole)
    }

    fn restore(&mut self, halves: u32, before: u32) {
        self.half_lives += halves;
        self.half_lives_regained += halves;
        self.mark_premier_ball(before);
    }

    /// The ball at floor(|lives before the Premier Ball|) becomes a Premier
    /// Ball, or the next one if it already is; if both already are, nothing
    /// changes. `half_lives` is that |lives| in halves.
    fn mark_premier_ball(&mut self, half_lives: u32) {
        let index = (half_lives / 2) as usize;
        if let Some(slot) = self
            .slots
            .iter_mut()
            .skip(index)
            .take(2)
            .find(|slot| **slot == Life::PokeBall)
        {
            *slot = Life::PremierBall;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET: u16 = 25;
    const P: Life = Life::PokeBall;
    const PB: Life = Life::PremierBall;

    fn run(mode: Mode) -> Run {
        Run::new(mode, Round::new(TARGET, vec![]))
    }

    fn next(run: &mut Run) {
        assert!(run.next_round(Round::new(TARGET, vec![])));
    }

    /// Catches the current Pokémon (first ball, no hint) and moves on.
    fn catch_and_continue(run: &mut Run) -> GuessResult {
        let result = run.guess(Some(TARGET));
        next(run);
        result
    }

    /// Misses with `n` balls.
    fn miss(run: &mut Run, n: u16) {
        for id in 1..=n {
            run.guess(Some(id));
        }
    }

    /// Misses with every ball, so the Pokémon flees.
    fn flee(run: &mut Run) -> GuessResult {
        let balls = run.mode.balls() as u16;
        miss(run, balls - 1);
        run.guess(Some(balls))
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
                        premier_ball: None
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
        assert_eq!(flee(&mut run), GuessResult::Fled { premier_ball: None });
        assert_eq!(run.round.outcome, Some(Outcome::Fled));
        assert_eq!((run.lives(), run.score), (2.0, 0));
        next(&mut run);
    }

    #[test]
    fn expert_has_three_balls_and_fleeing_costs_two_points() {
        let mut run = run(Mode::Expert);
        assert_eq!(run.guess(Some(1)), GuessResult::Wrong);
        assert_eq!(run.guess(Some(2)), GuessResult::Wrong);
        assert_eq!(run.score, 0, "only fleeing costs points");
        assert_eq!(run.guess(Some(3)), GuessResult::Fled { premier_ball: None });
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
    fn running_away_costs_what_a_hint_does() {
        let mut run = self::run(Mode::Normal);
        assert!(run.run_away());
        assert_eq!(run.round.outcome, Some(Outcome::RanAway));
        assert_eq!((run.lives(), run.score), (2.5, 0));
        next(&mut run);

        let mut run = self::run(Mode::Expert);
        run.run_away();
        assert_eq!(run.lives(), 2.0);
    }

    #[test]
    fn running_away_ends_the_run_on_the_last_of_the_lives() {
        let mut run = run(Mode::Normal);
        run.half_lives = 2;
        assert!(!run.running_away_ends_run());
        run.half_lives = 1;
        assert!(run.running_away_ends_run());
        assert!(run.run_away());
        assert!(run.is_over());
        assert!(!run.next_round(Round::new(1, vec![])));

        let mut run = self::run(Mode::Expert);
        run.half_lives = 2;
        assert!(run.running_away_ends_run());
    }

    #[test]
    fn unknown_and_repeated_guesses_are_free() {
        let mut run = run(Mode::Expert);
        assert_eq!(run.guess(None), GuessResult::Unknown);
        assert_eq!(run.guess(Some(1)), GuessResult::Wrong);
        assert_eq!(run.guess(Some(1)), GuessResult::Repeated);
        assert_eq!(run.round.balls_left(Mode::Expert), 2);
    }

    // the hidden streak

    #[test]
    fn only_clean_first_ball_catches_grow_the_streak() {
        let mut run = run(Mode::Normal);
        catch_and_continue(&mut run);
        assert_eq!(run.streak, 1);
        // full lives don't stop it counting
        for _ in 0..20 {
            catch_and_continue(&mut run);
        }
        assert_eq!(run.streak, 21);

        let mut run = self::run(Mode::Normal);
        run.streak = 15;
        run.guess(Some(1));
        run.guess(Some(TARGET));
        assert_eq!(run.streak, 5, "−10 for the miss, nothing for the catch");
    }

    #[test]
    fn misses_and_hints_cost_the_streak_once_per_pokemon() {
        let mut run = run(Mode::Normal);
        run.streak = 40;
        run.hint();
        assert_eq!(run.streak, 30);
        miss(&mut run, 3);
        run.hint();
        assert_eq!(run.streak, 30, "still once for this Pokémon");

        // on full lives (so no Premier Ball), the next Pokémon costs it again
        let mut run = self::run(Mode::Normal);
        run.streak = 40;
        miss(&mut run, 2);
        run.guess(Some(TARGET));
        next(&mut run);
        run.guess(Some(1));
        assert_eq!(run.streak, 20);
    }

    #[test]
    fn running_away_costs_the_streak_ten_more() {
        let mut run = run(Mode::Normal);
        run.streak = 40;
        run.run_away();
        assert_eq!(run.streak, 30);
        next(&mut run);
        run.guess(Some(1));
        run.run_away();
        assert_eq!(run.streak, 10, "−10 for the miss and −10 for running away");
    }

    #[test]
    fn the_streak_never_goes_below_zero() {
        let mut run = run(Mode::Normal);
        run.streak = 4;
        run.guess(Some(1));
        assert_eq!(run.streak, 0);
        run.run_away();
        assert_eq!(run.streak, 0);
    }

    #[test]
    fn expert_has_no_streak() {
        let mut run = run(Mode::Expert);
        run.hint();
        for _ in 0..30 {
            catch_and_continue(&mut run);
        }
        assert_eq!((run.streak, run.lives()), (0, 2.0));
    }

    // Premier Balls

    #[test]
    fn a_catch_with_a_streak_of_ten_restores_a_whole_life() {
        let mut run = run(Mode::Normal);
        run.run_away(); // 2.5
        next(&mut run);
        run.run_away(); // 2
        next(&mut run);
        run.streak = 9;
        assert_eq!(
            run.guess(Some(TARGET)),
            GuessResult::Caught {
                points: 8,
                premier_ball: Some(Restored::Whole)
            }
        );
        assert_eq!((run.lives(), run.streak), (3.0, 0));
        // floor(2) = slot 2 becomes the Premier Ball
        assert_eq!(run.slots, [P, P, PB]);
        assert_eq!(run.half_lives_regained, 2);
    }

    #[test]
    fn half_a_life_missing_restores_half_and_halves_the_streak() {
        let mut run = run(Mode::Normal);
        run.hint(); // 2.5, streak −10 (from 0)
        run.guess(Some(TARGET));
        next(&mut run);
        run.streak = 14;
        assert_eq!(
            run.guess(Some(TARGET)),
            GuessResult::Caught {
                points: 8,
                premier_ball: Some(Restored::Half)
            }
        );
        assert_eq!((run.lives(), run.streak), (3.0, 7));
        assert_eq!(run.slots, [P, P, PB]);
        assert_eq!(run.half_lives_regained, 1);
    }

    #[test]
    fn full_lives_keep_the_streak() {
        let mut run = run(Mode::Normal);
        run.streak = 30;
        assert_eq!(
            catch_and_continue(&mut run),
            GuessResult::Caught {
                points: 8,
                premier_ball: None
            }
        );
        assert_eq!(run.streak, 31);
        assert_eq!(run.slots, [P; 3]);
    }

    #[test]
    fn no_premier_ball_under_ten_or_for_running_away() {
        let mut run = run(Mode::Normal);
        run.half_lives = 4;
        run.streak = 8;
        run.guess(Some(TARGET)); // 9
        assert_eq!(run.lives(), 2.0);
        next(&mut run);
        run.streak = 50;
        run.run_away();
        assert_eq!(run.lives(), 1.5, "running away is never refunded");
    }

    #[test]
    fn a_flee_is_refunded_with_ten_left_after_the_miss() {
        // your example: 2.5 lives and a streak of 32 (22 after the missed
        // first ball): fleeing leaves 1.5, so floor(1.5) = slot 1 is replaced
        let mut run = run(Mode::Normal);
        run.half_lives = 5;
        run.streak = 32;
        assert_eq!(
            flee(&mut run),
            GuessResult::Fled {
                premier_ball: Some(Restored::Whole)
            }
        );
        assert_eq!((run.lives(), run.streak), (2.5, 0));
        assert_eq!(run.slots, [P, PB, P]);
        assert_eq!(run.fills(), [Fill::Full, Fill::Full, Fill::Half]);

        // if slot 1 is already a Premier Ball, slot 2 is replaced instead
        let mut run = self::run(Mode::Normal);
        run.half_lives = 5;
        run.slots = vec![P, PB, P];
        run.streak = 32;
        flee(&mut run);
        assert_eq!(run.slots, [P, PB, PB]);
    }

    #[test]
    fn two_premier_balls_in_a_row_are_left_alone() {
        let mut run = run(Mode::Normal);
        run.half_lives = 3;
        run.slots = vec![P, PB, PB];
        run.streak = 20;
        flee(&mut run); // 1.5 → 0.5 → 1.5: slot 0 is the floor, and a Poké Ball
        assert_eq!(run.slots, [PB, PB, PB]);

        let mut run = self::run(Mode::Easy);
        run.half_lives = 5;
        run.slots = vec![P, P, PB, PB, P];
        run.streak = 20;
        flee(&mut run); // 2.5 → 1.5: slot 1 is a Poké Ball
        assert_eq!(run.slots, [P, PB, PB, PB, P]);

        let mut run = self::run(Mode::Easy);
        run.half_lives = 7;
        run.slots = vec![P, P, PB, PB, P];
        run.streak = 20;
        flee(&mut run); // 3.5 → 2.5: slots 2 and 3 are both Premier Balls
        assert_eq!(run.slots, [P, P, PB, PB, P]);
        assert_eq!(run.lives(), 3.5, "the life still comes back");
    }

    #[test]
    fn a_flee_isnt_refunded_once_the_miss_takes_the_streak_under_ten() {
        let mut run = run(Mode::Normal);
        run.streak = 15;
        assert_eq!(flee(&mut run), GuessResult::Fled { premier_ball: None });
        assert_eq!((run.lives(), run.streak), (2.0, 5));
    }

    #[test]
    fn a_streak_of_25_survives_fleeing_with_the_last_life() {
        let mut run = run(Mode::Normal);
        run.half_lives = 2;
        run.streak = 35; // 25 after the miss
        assert_eq!(
            flee(&mut run),
            GuessResult::Fled {
                premier_ball: Some(Restored::Whole)
            }
        );
        assert!(!run.is_over());
        assert_eq!((run.lives(), run.streak), (1.0, 0));
        assert_eq!(run.slots, [PB, P, P], "floor(|0|) = slot 0");
        assert_eq!(run.half_lives_regained, 2);
        next(&mut run);

        // half a life left: −0.5 + 1 = 0.5, and floor(|−0.5|) = slot 0
        let mut run = self::run(Mode::Normal);
        run.half_lives = 1;
        run.streak = 35;
        flee(&mut run);
        assert_eq!((run.lives(), run.slots[0]), (0.5, PB));
    }

    #[test]
    fn under_25_fleeing_with_the_last_life_ends_the_run() {
        let mut run = run(Mode::Normal);
        run.half_lives = 2;
        run.streak = 34; // 24 after the miss
        assert_eq!(flee(&mut run), GuessResult::Fled { premier_ball: None });
        assert!(run.is_over());
    }

    #[test]
    fn counts_hints_running_away_and_lives_regained() {
        let mut run = run(Mode::Normal);
        run.hint();
        run.hint();
        run.run_away(); // 3 − ½ − ½ − ½ = 1.5
        next(&mut run);
        run.streak = 20;
        catch_and_continue(&mut run); // +1 = 2.5, streak 0
        run.streak = 20;
        catch_and_continue(&mut run); // +½ = 3, streak 10
        assert_eq!(run.hints_bought, 2);
        assert_eq!(run.ran_away, 1);
        assert_eq!(run.half_lives_regained, 3);
        assert_eq!(run.lives(), 3.0);
    }
}

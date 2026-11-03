//! Run history and the personal leaderboard.
//!
//! A run is recorded when it starts, as [`Status::Ongoing`], and updated as
//! it's played; it becomes [`Status::Complete`] when the lives run out. Any
//! run can be archived, which hides it from the recent runs but not from the
//! leaderboard.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::game::Mode;

/// How many runs per mode the leaderboard shows, and so never forgets.
pub const TOP_RUNS: usize = 3;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// Still being played, or left partway.
    Ongoing,
    /// Out of lives. Records from before statuses were kept were all
    /// complete.
    #[default]
    Complete,
    /// Hidden from the recent runs. Unarchiving works out from the record
    /// whether it's ongoing or complete.
    Archived,
}

/// A run, finished or not.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    /// Unique per run, so the same run can be synced more than once safely.
    pub id: String,
    pub mode: Mode,
    pub generations: Vec<u8>,
    pub score: i32,
    pub caught: u32,
    /// Milliseconds since the Unix epoch. Missing from older records.
    #[serde(default)]
    pub started_at: Option<f64>,
    /// Milliseconds since the Unix epoch, once the run is over.
    #[serde(default)]
    pub finished_at: Option<f64>,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub hints_bought: u32,
    /// Times the player ran away.
    #[serde(default)]
    pub ran_away: u32,
    /// Pokémon that fled after the last ball.
    #[serde(default)]
    pub fled: u32,
    /// Premier Balls won.
    #[serde(default)]
    pub premier_balls: u32,
    /// Lives won back with Premier Balls, in halves.
    #[serde(default)]
    pub half_lives_regained: u32,
}

impl RunRecord {
    /// Whether the run is over, archived or not.
    pub fn is_finished(&self) -> bool {
        self.finished_at.is_some()
    }

    /// When the run started, or for older records, when it finished.
    pub fn sort_time(&self) -> f64 {
        self.started_at.or(self.finished_at).unwrap_or(0.0)
    }

    /// Hides the run from the recent runs, or brings it back.
    pub fn set_archived(&mut self, archived: bool) {
        self.status = if archived {
            Status::Archived
        } else if self.is_finished() {
            Status::Complete
        } else {
            Status::Ongoing
        };
    }
}

/// Better runs first: higher score, then the earlier of two equal scores.
pub fn rank(a: &RunRecord, b: &RunRecord) -> Ordering {
    b.score.cmp(&a.score).then(
        a.finished_at
            .unwrap_or(f64::INFINITY)
            .total_cmp(&b.finished_at.unwrap_or(f64::INFINITY)),
    )
}

/// The best finished runs in `mode`, best first, archived or not.
pub fn top_runs(runs: &[RunRecord], mode: Mode, count: usize) -> Vec<&RunRecord> {
    let mut runs: Vec<&RunRecord> = runs
        .iter()
        .filter(|r| r.mode == mode && r.is_finished())
        .collect();
    runs.sort_by(|a, b| rank(a, b));
    runs.truncate(count);
    runs
}

/// Archived runs, or the others, newest first.
pub fn recent_runs(runs: &[RunRecord], archived: bool) -> Vec<&RunRecord> {
    let mut runs: Vec<&RunRecord> = runs
        .iter()
        .filter(|r| (r.status == Status::Archived) == archived)
        .collect();
    runs.sort_by(|a, b| b.sort_time().total_cmp(&a.sort_time()));
    runs
}

/// Adds a run, or updates it if it's already there (keeping it archived if
/// it was). Then drops the oldest runs beyond `cap`, but never a mode's top
/// [`TOP_RUNS`].
pub fn save_run(runs: &mut Vec<RunRecord>, mut run: RunRecord, cap: usize) {
    match runs.iter_mut().find(|r| r.id == run.id) {
        Some(existing) => {
            if existing.status == Status::Archived {
                run.status = Status::Archived;
            }
            *existing = run;
        }
        None => runs.push(run),
    }
    while runs.len() > cap {
        let keep: Vec<String> = Mode::ALL
            .into_iter()
            .flat_map(|mode| top_runs(runs, mode, TOP_RUNS))
            .map(|r| r.id.clone())
            .collect();
        let oldest = runs
            .iter()
            .enumerate()
            .filter(|(_, r)| !keep.contains(&r.id))
            .min_by(|(_, a), (_, b)| a.sort_time().total_cmp(&b.sort_time()))
            .map(|(i, _)| i);
        match oldest {
            Some(i) => {
                runs.remove(i);
            }
            None => break,
        }
    }
}

/// How long a run took, e.g. "under a minute", "12 min" or "1 h 5 min".
pub fn format_duration(millis: f64) -> String {
    let minutes = (millis.max(0.0) / 60_000.0) as u64;
    match (minutes / 60, minutes % 60) {
        (0, 0) => "under a minute".to_string(),
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(id: &str, mode: Mode, score: i32, finished_at: f64) -> RunRecord {
        RunRecord {
            id: id.into(),
            mode,
            generations: vec![1],
            score,
            caught: score.unsigned_abs(),
            started_at: Some(finished_at),
            finished_at: Some(finished_at),
            status: Status::Complete,
            hints_bought: 0,
            ran_away: 0,
            fled: 0,
            premier_balls: 0,
            half_lives_regained: 0,
        }
    }

    fn ongoing(id: &str, mode: Mode, score: i32, started_at: f64) -> RunRecord {
        RunRecord {
            finished_at: None,
            status: Status::Ongoing,
            ..run(id, mode, score, started_at)
        }
    }

    fn ids<'a>(runs: impl IntoIterator<Item = &'a RunRecord>) -> Vec<&'a str> {
        runs.into_iter().map(|r| r.id.as_str()).collect()
    }

    #[test]
    fn ranks_by_score_then_age() {
        let runs = [
            run("a", Mode::Easy, 5, 1.0),
            run("b", Mode::Easy, 9, 2.0),
            run("c", Mode::Easy, 5, 0.5),
            run("d", Mode::Hard, 99, 0.0),
            run("e", Mode::Easy, 1, 3.0),
        ];
        assert_eq!(ids(top_runs(&runs, Mode::Easy, 3)), ["b", "c", "a"]);
    }

    #[test]
    fn the_leaderboard_counts_archived_runs_but_not_ongoing_ones() {
        let mut archived = run("archived", Mode::Easy, 5, 1.0);
        archived.set_archived(true);
        let runs = [
            ongoing("ongoing", Mode::Easy, 50, 2.0),
            archived,
            run("complete", Mode::Easy, 3, 3.0),
        ];
        assert_eq!(
            ids(top_runs(&runs, Mode::Easy, 3)),
            ["archived", "complete"]
        );
    }

    #[test]
    fn recent_runs_are_newest_first_and_split_by_archiving() {
        let mut runs = vec![
            run("old", Mode::Easy, 1, 1.0),
            ongoing("new", Mode::Hard, 1, 3.0),
            run("middle", Mode::Normal, 1, 2.0),
        ];
        runs[2].set_archived(true);
        assert_eq!(ids(recent_runs(&runs, false)), ["new", "old"]);
        assert_eq!(ids(recent_runs(&runs, true)), ["middle"]);
    }

    #[test]
    fn unarchiving_works_out_the_status() {
        let mut complete = run("a", Mode::Easy, 1, 1.0);
        let mut unfinished = ongoing("b", Mode::Easy, 1, 1.0);
        for record in [&mut complete, &mut unfinished] {
            record.set_archived(true);
            assert_eq!(record.status, Status::Archived);
            record.set_archived(false);
        }
        assert_eq!(complete.status, Status::Complete);
        assert_eq!(unfinished.status, Status::Ongoing);
    }

    #[test]
    fn saving_updates_a_run_and_keeps_it_archived() {
        let mut runs = vec![ongoing("a", Mode::Easy, 1, 1.0)];
        save_run(&mut runs, ongoing("a", Mode::Easy, 4, 1.0), 10);
        assert_eq!((runs.len(), runs[0].score), (1, 4));

        runs[0].set_archived(true);
        save_run(&mut runs, run("a", Mode::Easy, 6, 1.0), 10);
        assert_eq!(runs[0].status, Status::Archived);
        assert!(runs[0].is_finished());
    }

    #[test]
    fn reads_records_from_before_statuses() {
        let json =
            r#"{"id":"x","mode":"Easy","generations":[1],"score":3,"caught":3,"finished_at":5.0}"#;
        let record: RunRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.status, Status::Complete);
        assert_eq!((record.started_at, record.finished_at), (None, Some(5.0)));
        assert_eq!(record.sort_time(), 5.0);
        assert_eq!((record.fled, record.premier_balls), (0, 0));
    }

    #[test]
    fn formats_durations() {
        let min = 60_000.0;
        assert_eq!(format_duration(59_999.0), "under a minute");
        assert_eq!(format_duration(12.5 * min), "12 min");
        assert_eq!(format_duration(60.0 * min), "1 h");
        assert_eq!(format_duration(65.0 * min), "1 h 5 min");
    }

    #[test]
    fn drops_the_oldest_runs_but_keeps_the_best() {
        let mut runs = vec![];
        // the oldest run is the best, so it must survive
        save_run(&mut runs, run("best", Mode::Easy, 50, 0.0), 4);
        for (i, (id, score)) in [("a", 1), ("b", 9), ("c", 1), ("d", 1), ("e", 1)]
            .into_iter()
            .enumerate()
        {
            save_run(&mut runs, run(id, Mode::Easy, score, 1.0 + i as f64), 4);
        }
        // the top 3 are best, b and a (the earlier of the tied 1s), so the
        // next oldest runs, c and d, are the ones dropped
        assert_eq!(ids(&runs), ["best", "a", "b", "e"]);
    }

    #[test]
    fn keeps_everything_when_only_top_runs_remain() {
        let mut runs = vec![];
        for (i, mode) in Mode::ALL.into_iter().enumerate() {
            save_run(&mut runs, run(&format!("{i}"), mode, 1, i as f64), 2);
        }
        assert_eq!(runs.len(), 4);
    }
}

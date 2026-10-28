//! Finished runs and the personal leaderboard.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::game::Mode;

/// How many runs per mode the leaderboard shows, and so never forgets.
pub const TOP_RUNS: usize = 3;

/// A finished run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    /// Unique per run, so the same run can be synced more than once safely.
    pub id: String,
    pub mode: Mode,
    pub generations: Vec<u8>,
    pub score: u32,
    pub caught: u32,
    /// Milliseconds since the Unix epoch.
    pub finished_at: f64,
}

/// Better runs first: higher score, then the earlier of two equal scores.
pub fn rank(a: &RunRecord, b: &RunRecord) -> Ordering {
    b.score
        .cmp(&a.score)
        .then(a.finished_at.total_cmp(&b.finished_at))
}

/// The best runs in `mode`, best first.
pub fn top_runs(runs: &[RunRecord], mode: Mode, count: usize) -> Vec<&RunRecord> {
    let mut runs: Vec<&RunRecord> = runs.iter().filter(|r| r.mode == mode).collect();
    runs.sort_by(|a, b| rank(a, b));
    runs.truncate(count);
    runs
}

/// Adds a run, then drops the oldest runs beyond `cap`, but never a mode's
/// top [`TOP_RUNS`].
pub fn add_run(runs: &mut Vec<RunRecord>, run: RunRecord, cap: usize) {
    runs.push(run);
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
            .min_by(|(_, a), (_, b)| a.finished_at.total_cmp(&b.finished_at))
            .map(|(i, _)| i);
        match oldest {
            Some(i) => {
                runs.remove(i);
            }
            None => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(id: &str, mode: Mode, score: u32, finished_at: f64) -> RunRecord {
        RunRecord {
            id: id.into(),
            mode,
            generations: vec![1],
            score,
            caught: score,
            finished_at,
        }
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
        let ids: Vec<&str> = top_runs(&runs, Mode::Easy, 3)
            .iter()
            .map(|r| r.id.as_str())
            .collect();
        assert_eq!(ids, ["b", "c", "a"]);
    }

    #[test]
    fn drops_the_oldest_runs_but_keeps_the_best() {
        let mut runs = vec![];
        // the oldest run is the best, so it must survive
        add_run(&mut runs, run("best", Mode::Easy, 50, 0.0), 4);
        for (i, (id, score)) in [("a", 1), ("b", 9), ("c", 1), ("d", 1), ("e", 1)]
            .into_iter()
            .enumerate()
        {
            add_run(&mut runs, run(id, Mode::Easy, score, 1.0 + i as f64), 4);
        }
        // the top 3 are best, b and a (the earlier of the tied 1s), so the
        // next oldest runs, c and d, are the ones dropped
        let ids: Vec<&str> = runs.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["best", "a", "b", "e"]);
    }

    #[test]
    fn keeps_everything_when_only_top_runs_remain() {
        let mut runs = vec![];
        for (i, mode) in Mode::ALL.into_iter().enumerate() {
            add_run(&mut runs, run(&format!("{i}"), mode, 1, i as f64), 2);
        }
        assert_eq!(runs.len(), 4);
    }
}

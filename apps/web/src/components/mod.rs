pub mod clues;
pub mod game;
pub mod leaderboard;
pub mod menu;
pub mod tooltip;

/// A score with a proper minus sign, e.g. "−2".
pub fn format_score(score: i32) -> String {
    if score < 0 {
        format!("−{}", score.unsigned_abs())
    } else {
        score.to_string()
    }
}

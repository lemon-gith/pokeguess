pub mod clues;
pub mod game;
pub mod history;
pub mod leaderboard;
pub mod menu;
pub mod tooltip;

use yew::prelude::*;

/// Premier Balls won in a run, as the icon and the count; nothing for none.
pub fn premier_balls(count: u32) -> Html {
    if count == 0 {
        return html! {};
    }
    let label = match count {
        1 => "1 Premier Ball".to_string(),
        n => format!("{n} Premier Balls"),
    };
    html! {
        <span class="inline-flex items-center gap-0.5 align-middle" title={label.clone()} aria-label={label}>
            <img src="img/icons/premier-ball.svg" alt="" class="size-4" />
            <span aria-hidden="true">{ format!("×{count}") }</span>
        </span>
    }
}

/// A score with a proper minus sign, e.g. "−2".
pub fn format_score(score: i32) -> String {
    if score < 0 {
        format!("−{}", score.unsigned_abs())
    } else {
        score.to_string()
    }
}

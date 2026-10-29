use shared::game::{Mode, format_generations};
use shared::runs::{RunRecord, TOP_RUNS, top_runs};
use yew::prelude::*;

use super::format_score;
use crate::browser;

#[derive(Properties, PartialEq)]
pub struct LeaderboardProps {
    pub runs: Vec<RunRecord>,
}

/// Each mode's best runs on this device.
#[function_component(Leaderboard)]
pub fn leaderboard(props: &LeaderboardProps) -> Html {
    let modes: Vec<Html> = Mode::ALL
        .into_iter()
        .filter_map(|mode| {
            let best = top_runs(&props.runs, mode, TOP_RUNS);
            (!best.is_empty()).then(|| {
                html! {
                    <div>
                        <h3 class="font-semibold">{ mode.label() }</h3>
                        <ol class="list-decimal pl-6 text-sm">
                            { for best.into_iter().map(run_row) }
                        </ol>
                    </div>
                }
            })
        })
        .collect();

    html! {
        <section class="space-y-2">
            <h2 class="text-lg font-semibold">{ "Your best runs" }</h2>
            if modes.is_empty() {
                <p class="text-sm text-gray-600">{ "No runs yet. Your best runs will show up here." }</p>
            } else {
                <div class="grid gap-4 sm:grid-cols-2">{ for modes }</div>
            }
        </section>
    }
}

fn run_row(run: &RunRecord) -> Html {
    html! {
        <li>
            <span class="font-semibold">{ format!("{} pts", format_score(run.score)) }</span>
            <span class="text-gray-600">
                { format!(
                    " · {} caught · Gen {} · {}",
                    run.caught,
                    format_generations(&run.generations),
                    browser::format_date(run.finished_at),
                ) }
            </span>
        </li>
    }
}

//! Recent runs, newest first: ongoing ones can be resumed, and any run can be
//! archived out of the way.

use shared::game::format_generations;
use shared::runs::{RunRecord, format_duration, recent_runs};
use yew::prelude::*;

use super::{format_score, premier_balls};
use crate::browser;

/// Runs shown at first, and how many more each "Show more" adds.
const PAGE: usize = 5;

#[derive(Properties, PartialEq)]
pub struct HistoryProps {
    pub runs: Vec<RunRecord>,
    /// Ongoing runs whose game is saved on this device.
    pub resumable: Vec<String>,
    pub on_resume: Callback<String>,
    /// A run's ID, and whether to archive it (or unarchive it).
    pub on_archive: Callback<(String, bool)>,
}

#[function_component(History)]
pub fn history(props: &HistoryProps) -> Html {
    let shown = use_state(|| PAGE);
    let show_archived = use_state(|| false);

    let recent = recent_runs(&props.runs, false);
    let archived = recent_runs(&props.runs, true);
    if recent.is_empty() && archived.is_empty() {
        return html! {};
    }

    let row = |run: &RunRecord| {
        html! {
            <RunRow
                key={run.id.clone()}
                run={run.clone()}
                resumable={props.resumable.contains(&run.id)}
                on_resume={props.on_resume.clone()}
                on_archive={props.on_archive.clone()}
            />
        }
    };

    let more = (recent.len() > *shown).then(|| {
        let shown = shown.clone();
        html! {
            <button type="button" class="cursor-pointer text-sm text-blue-700 underline" onclick={Callback::from(move |_| shown.set(*shown + PAGE))}>
                { "Show more" }
            </button>
        }
    });

    let archived_list = (!archived.is_empty()).then(|| {
        let toggle = {
            let show_archived = show_archived.clone();
            Callback::from(move |_| show_archived.set(!*show_archived))
        };
        html! {
            <div class="space-y-2">
                <button type="button" class="cursor-pointer text-sm text-gray-600 underline" aria-expanded={show_archived.to_string()} onclick={toggle}>
                    { if *show_archived {
                        "Hide archived runs".to_string()
                    } else {
                        format!("Show archived runs ({})", archived.len())
                    } }
                </button>
                if *show_archived {
                    <ul class="space-y-2">{ for archived.iter().map(|run| row(run)) }</ul>
                }
            </div>
        }
    });

    html! {
        <section class="space-y-2">
            <h2 class="text-lg font-semibold">{ "Recent runs" }</h2>
            if recent.is_empty() {
                <p class="text-sm text-gray-600">{ "Nothing here: every run is archived." }</p>
            } else {
                <ul class="space-y-2">{ for recent.iter().take(*shown).map(|run| row(run)) }</ul>
            }
            { for more }
            { for archived_list }
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct RunRowProps {
    run: RunRecord,
    resumable: bool,
    on_resume: Callback<String>,
    on_archive: Callback<(String, bool)>,
}

/// One run: a summary that opens to show the details.
#[function_component(RunRow)]
fn run_row(props: &RunRowProps) -> Html {
    let run = &props.run;
    let archived = run.status == shared::runs::Status::Archived;
    let (status, status_class) = if run.is_finished() {
        ("Complete", "bg-gray-100 text-gray-700")
    } else {
        ("Ongoing", "bg-amber-100 text-amber-800")
    };
    let when = run
        .started_at
        .or(run.finished_at)
        .map(browser::format_date_time);

    let resume = (props.resumable && !archived && !run.is_finished()).then(|| {
        let on_resume = props.on_resume.clone();
        let id = run.id.clone();
        html! {
            <button type="button" class="shrink-0 cursor-pointer rounded-lg bg-blue-600 px-3 py-1.5 text-sm font-semibold text-white hover:bg-blue-700" onclick={Callback::from(move |_| on_resume.emit(id.clone()))}>
                { "Resume" }
            </button>
        }
    });
    let archive = {
        let on_archive = props.on_archive.clone();
        let id = run.id.clone();
        Callback::from(move |_| on_archive.emit((id.clone(), !archived)))
    };

    let mut details = vec![];
    if let Some(started) = run.started_at {
        details.push(("Started", browser::format_date_time(started)));
    }
    if let Some(finished) = run.finished_at {
        details.push(("Finished", browser::format_date_time(finished)));
        if let Some(started) = run.started_at {
            details.push(("Took", format_duration(finished - started)));
        }
    }
    details.extend([
        ("Caught", run.caught.to_string()),
        ("Fled", run.fled.to_string()),
        ("Ran away", run.ran_away.to_string()),
        ("Hints", run.hints_bought.to_string()),
        ("Premier Balls", run.premier_balls.to_string()),
        (
            "Lives won back",
            (run.half_lives_regained as f32 / 2.0).to_string(),
        ),
    ]);

    html! {
        <li class="flex items-start gap-2 rounded-lg border border-gray-200 px-3 py-2">
            <details class="group min-w-0 flex-1">
                <summary class="cursor-pointer list-none [&::-webkit-details-marker]:hidden">
                    <span class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm">
                        <span class="font-semibold">{ run.mode.label() }</span>
                        <span class={classes!("rounded-full", "px-2", "text-xs", "font-semibold", status_class)}>{ status }</span>
                        <span class="font-semibold">{ format!("{} pts", format_score(run.score)) }</span>
                        { premier_balls(run.premier_balls) }
                    </span>
                    <span class="block text-xs text-gray-600">
                        { format!("{} caught · Gen {}", run.caught, format_generations(&run.generations)) }
                        if let Some(when) = when {
                            { format!(" · {when}") }
                        }
                        <span class="ml-1 inline-block transition group-open:rotate-90" aria-hidden="true">{ "›" }</span>
                    </span>
                </summary>
                <dl class="mt-2 grid grid-cols-[auto_1fr] gap-x-3 text-xs">
                    { for details.into_iter().map(|(term, value)| html! {
                        <>
                            <dt class="text-gray-600">{ term }</dt>
                            <dd>{ value }</dd>
                        </>
                    }) }
                </dl>
                <button type="button" class="mt-2 cursor-pointer text-xs text-gray-600 underline" onclick={archive}>
                    { if archived { "Unarchive" } else { "Archive" } }
                </button>
            </details>
            { for resume }
        </li>
    }
}

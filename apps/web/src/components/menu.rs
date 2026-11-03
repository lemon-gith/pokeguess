use shared::game::{Mode, RunSettings};
use shared::runs::RunRecord;
use yew::prelude::*;

use super::leaderboard::Leaderboard;
use crate::Dex;

#[derive(Properties, PartialEq)]
pub struct MenuProps {
    pub dex: Dex,
    /// Where the pickers start.
    pub settings: RunSettings,
    pub runs: Vec<RunRecord>,
    pub on_start: Callback<RunSettings>,
}

const CHOICE: &str = "cursor-pointer rounded-lg border-2 px-3 py-2 transition";
const CHOSEN: &str = "border-blue-500 bg-blue-50";
const UNCHOSEN: &str = "border-gray-200 hover:border-gray-400";

#[function_component(Menu)]
pub fn menu(props: &MenuProps) -> Html {
    let settings = use_state(|| props.settings.clone());
    let all_generations = props.dex.generations();
    let pool_size = settings.pool(&props.dex).len();

    let modes = Mode::ALL.into_iter().map(|mode| {
        let onclick = {
            let settings = settings.clone();
            Callback::from(move |_| {
                settings.set(RunSettings {
                    mode,
                    ..(*settings).clone()
                })
            })
        };
        let state = if settings.mode == mode {
            CHOSEN
        } else {
            UNCHOSEN
        };
        html! {
            <button type="button" class={classes!(CHOICE, state, "text-left")} {onclick}>
                <span class="block font-semibold">{ mode.label() }</span>
                <span class="block text-sm text-gray-600">{ mode.blurb() }</span>
            </button>
        }
    });

    let generations = all_generations.iter().map(|&generation| {
        let onclick = {
            let settings = settings.clone();
            Callback::from(move |_| {
                let mut updated = (*settings).clone();
                if let Some(i) = updated.generations.iter().position(|&g| g == generation) {
                    updated.generations.remove(i);
                } else {
                    updated.generations.push(generation);
                    updated.generations.sort_unstable();
                }
                settings.set(updated);
            })
        };
        let state = if settings.generations.contains(&generation) {
            CHOSEN
        } else {
            UNCHOSEN
        };
        html! {
            <button
                type="button"
                class={classes!(CHOICE, state, "min-w-12", "text-center", "font-semibold")}
                aria-pressed={settings.generations.contains(&generation).to_string()}
                {onclick}
            >
                { generation }
            </button>
        }
    });

    let all_chosen = settings.generations == all_generations;
    let toggle_all = {
        let settings = settings.clone();
        let all_generations = all_generations.clone();
        Callback::from(move |_| {
            settings.set(RunSettings {
                generations: if all_chosen {
                    Vec::new()
                } else {
                    all_generations.clone()
                },
                ..(*settings).clone()
            })
        })
    };

    let start = {
        let settings = settings.clone();
        let on_start = props.on_start.clone();
        Callback::from(move |_| on_start.emit((*settings).clone()))
    };

    html! {
        <div class="space-y-6">
            <header class="text-center">
                <h1 class="text-4xl font-bold">{ "PokéGuess" }</h1>
                <p class="text-gray-600">{ "Who's that Pokémon? Catch it with as few balls as you can: hints and running away cost lives. How many can you catch?" }</p>
            </header>

            <section class="space-y-2">
                <h2 class="text-lg font-semibold">{ "Difficulty" }</h2>
                <div class="grid gap-2 sm:grid-cols-2">{ for modes }</div>
            </section>

            <section class="space-y-2">
                <div class="flex items-baseline justify-between">
                    <h2 class="text-lg font-semibold">{ "Generations" }</h2>
                    <button type="button" class="text-sm text-blue-700 underline" onclick={toggle_all}>
                        { if all_chosen { "Clear" } else { "Select all" } }
                    </button>
                </div>
                <div class="flex flex-wrap gap-2">{ for generations }</div>
                <p class="text-sm text-gray-600">{ format!("{pool_size} Pokémon to guess from") }</p>
            </section>

            // stays in reach at the bottom of the screen on phones
            <div class="sticky bottom-0 -mx-4 bg-white/95 px-4 pt-3 pb-[max(0.75rem,env(safe-area-inset-bottom))] md:static md:mx-0 md:bg-transparent md:p-0">
                <button
                    type="button"
                    class="w-full cursor-pointer rounded-xl bg-blue-600 px-4 py-3 text-lg font-semibold text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:bg-gray-400"
                    disabled={pool_size == 0}
                    onclick={start}
                >
                    { "Start" }
                </button>
            </div>

            <Leaderboard runs={props.runs.clone()} />
        </div>
    }
}

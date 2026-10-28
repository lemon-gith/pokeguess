use std::ops::Deref;
use std::rc::Rc;

use gloo::net::http::Request;
use shared::game::RunSettings;
use shared::pokedex::{Pokedex, SCHEMA_VERSION};
use shared::runs::{RunRecord, add_run};
use yew::platform::spawn_local;
use yew::prelude::*;

mod browser;
mod components;

use components::game::Game;
use components::menu::Menu;

const SETTINGS_KEY: &str = "pokeguess.settings";
const RUNS_KEY: &str = "pokeguess.runs";
const CARD: &str = "box-border flex w-full flex-col bg-white/95 p-4 md:my-8 md:w-11/12 md:max-w-3xl md:rounded-xl md:p-8 md:shadow-lg md:outline md:outline-black/5";

/// Runs kept in local storage; each mode's best runs are always kept.
const MAX_STORED_RUNS: usize = 100;

/// The loaded dataset. Compared by pointer, so passing it as a prop is cheap.
#[derive(Clone)]
pub struct Dex(Rc<Pokedex>);

impl PartialEq for Dex {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for Dex {
    type Target = Pokedex;

    fn deref(&self) -> &Pokedex {
        &self.0
    }
}

enum DexState {
    Loading,
    Ready(Dex),
    Failed(String),
}

async fn fetch_pokedex() -> Result<Pokedex, String> {
    let response = Request::get("pokedex.json")
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !response.ok() {
        return Err(format!("the server said {}", response.status()));
    }
    let dex: Pokedex = response.json().await.map_err(|err| err.to_string())?;
    if dex.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported dataset version {}",
            dex.schema_version
        ));
    }
    Ok(dex)
}

/// The last settings used, limited to generations the dataset has.
fn stored_settings(dex: &Pokedex) -> RunSettings {
    let defaults = RunSettings::new(dex);
    let Some(mut settings) = browser::load::<RunSettings>(SETTINGS_KEY) else {
        return defaults;
    };
    settings
        .generations
        .retain(|g| defaults.generations.contains(g));
    if settings.generations.is_empty() {
        settings.generations = defaults.generations;
    }
    settings
}

#[function_component(App)]
fn app() -> Html {
    let dex = use_state(|| DexState::Loading);
    {
        let dex = dex.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                dex.set(match fetch_pokedex().await {
                    Ok(pokedex) => DexState::Ready(Dex(Rc::new(pokedex))),
                    Err(err) => DexState::Failed(err),
                });
            });
        });
    }
    let runs = use_state(|| browser::load::<Vec<RunRecord>>(RUNS_KEY).unwrap_or_default());
    let playing = use_state(|| None::<RunSettings>);

    let body = match &*dex {
        DexState::Loading => html! {
            <p class="text-center text-gray-600">{ "Loading the Pokédex…" }</p>
        },
        DexState::Failed(err) => html! {
            <div class="space-y-2 text-center">
                <p class="font-semibold">{ "Couldn't load the Pokédex." }</p>
                <p class="text-sm text-gray-600">{ err }</p>
            </div>
        },
        DexState::Ready(dex) => match &*playing {
            Some(settings) => {
                let on_finish = {
                    let runs = runs.clone();
                    Callback::from(move |run: RunRecord| {
                        let mut updated = (*runs).clone();
                        add_run(&mut updated, run, MAX_STORED_RUNS);
                        browser::save(RUNS_KEY, &updated);
                        runs.set(updated);
                    })
                };
                let on_quit = {
                    let playing = playing.clone();
                    Callback::from(move |()| playing.set(None))
                };
                html! {
                    <Game dex={dex.clone()} settings={settings.clone()} {on_finish} {on_quit} />
                }
            }
            None => {
                let on_start = {
                    let playing = playing.clone();
                    Callback::from(move |settings: RunSettings| {
                        browser::save(SETTINGS_KEY, &settings);
                        playing.set(Some(settings));
                    })
                };
                html! {
                    <Menu
                        dex={dex.clone()}
                        settings={stored_settings(dex)}
                        runs={(*runs).clone()}
                        {on_start}
                    />
                }
            }
        },
    };

    // full-screen on phones, a floating card on wider screens; during a game
    // it's exactly the visible height, so the guess box sits above the keyboard
    let height = if playing.is_some() {
        "h-[var(--app-height,100dvh)] md:h-auto"
    } else {
        "min-h-dvh md:min-h-0"
    };
    html! {
        <main class={classes!(CARD, height)}>
            { body }
        </main>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}

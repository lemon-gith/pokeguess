use std::collections::BTreeMap;
use std::ops::Deref;
use std::rc::Rc;

use gloo::net::http::Request;
use shared::game::RunSettings;
use shared::pokedex::{Pokedex, SCHEMA_VERSION};
use shared::runs::{RunRecord, save_run};
use yew::platform::spawn_local;
use yew::prelude::*;

mod browser;
mod components;

use components::game::{Game, SavedGame, Start};
use components::menu::Menu;

// bumped when the rules change, since scores under different rules aren't
// comparable (v3: balls and half lives)
const SETTINGS_KEY: &str = "pokeguess.settings.v3";
const RUNS_KEY: &str = "pokeguess.runs.v3";
/// Games that can be resumed, by run ID.
const GAMES_KEY: &str = "pokeguess.games.v3";
/// In session storage: the game this tab is playing, so a refresh resumes it.
const CURRENT_GAME_KEY: &str = "pokeguess.current.v3";
const OLD_KEYS: [&str; 4] = [
    "pokeguess.settings",
    "pokeguess.runs",
    "pokeguess.settings.v2",
    "pokeguess.runs.v2",
];
/// In session storage; replaced by [`GAMES_KEY`] and [`CURRENT_GAME_KEY`].
const OLD_SESSION_KEYS: [&str; 1] = ["pokeguess.game.v3"];
const CARD: &str = "box-border flex w-full flex-col bg-white/95 p-4 md:my-8 md:w-11/12 md:max-w-3xl md:rounded-xl md:p-8 md:shadow-lg md:outline md:outline-black/5";

/// Runs kept in local storage; each mode's best runs are always kept.
const MAX_STORED_RUNS: usize = 100;

type SavedGames = BTreeMap<String, SavedGame>;

fn load_runs() -> Vec<RunRecord> {
    browser::load(RUNS_KEY).unwrap_or_default()
}

fn load_games() -> SavedGames {
    browser::load(GAMES_KEY).unwrap_or_default()
}

/// Saves the runs, and the games of the runs still in progress.
fn store(runs: &[RunRecord], mut games: SavedGames) {
    games.retain(|id, _| runs.iter().any(|r| &r.id == id && !r.is_finished()));
    browser::save(RUNS_KEY, &runs);
    browser::save(GAMES_KEY, &games);
}

/// Records a game after a change, completing its run once it's over.
fn save_game(game: SavedGame) -> Vec<RunRecord> {
    let mut runs = load_runs();
    let mut games = load_games();
    let over = game.run.is_over();
    let finished_at = over.then(|| {
        runs.iter()
            .find(|r| r.id == game.id)
            .and_then(|r| r.finished_at)
            .unwrap_or_else(browser::now)
    });
    save_run(&mut runs, game.record(finished_at), MAX_STORED_RUNS);
    if over {
        browser::forget_session(CURRENT_GAME_KEY);
        games.remove(&game.id);
    } else {
        browser::save_session(CURRENT_GAME_KEY, &game.id);
        games.insert(game.id.clone(), game);
    }
    store(&runs, games);
    runs
}

/// Leaves this tab's game, which stays saved to resume later, unless nothing
/// was played: then it's forgotten.
fn quit_game() -> Vec<RunRecord> {
    let mut runs = load_runs();
    let games = load_games();
    if let Some(id) = browser::load_session::<String>(CURRENT_GAME_KEY) {
        if games.get(&id).is_some_and(SavedGame::is_untouched) {
            runs.retain(|r| r.id != id);
        }
        browser::forget_session(CURRENT_GAME_KEY);
    }
    store(&runs, games);
    runs
}

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
    let runs = use_state(load_runs);
    // pick up a game this tab was playing before a refresh
    let playing = use_state(|| {
        browser::load_session::<String>(CURRENT_GAME_KEY)
            .and_then(|id| load_games().remove(&id))
            .map(|game| Start::Resume(Box::new(game)))
    });
    use_effect_with((), |_| {
        for key in OLD_KEYS {
            browser::forget(key);
        }
        for key in OLD_SESSION_KEYS {
            browser::forget_session(key);
        }
    });

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
            Some(start) => {
                let on_save = {
                    let runs = runs.clone();
                    Callback::from(move |game: SavedGame| runs.set(save_game(game)))
                };
                let on_quit = {
                    let runs = runs.clone();
                    let playing = playing.clone();
                    Callback::from(move |()| {
                        runs.set(quit_game());
                        playing.set(None);
                    })
                };
                html! {
                    <Game dex={dex.clone()} start={start.clone()} {on_save} {on_quit} />
                }
            }
            None => {
                let on_start = {
                    let playing = playing.clone();
                    Callback::from(move |settings: RunSettings| {
                        browser::save(SETTINGS_KEY, &settings);
                        playing.set(Some(Start::New(settings)));
                    })
                };
                let on_resume = {
                    let playing = playing.clone();
                    Callback::from(move |id: String| {
                        if let Some(game) = load_games().remove(&id) {
                            playing.set(Some(Start::Resume(Box::new(game))));
                        }
                    })
                };
                let on_archive = {
                    let runs = runs.clone();
                    Callback::from(move |(id, archived): (String, bool)| {
                        let mut updated = load_runs();
                        if let Some(run) = updated.iter_mut().find(|r| r.id == id) {
                            run.set_archived(archived);
                        }
                        browser::save(RUNS_KEY, &updated);
                        runs.set(updated);
                    })
                };
                html! {
                    <Menu
                        dex={dex.clone()}
                        settings={stored_settings(dex)}
                        runs={(*runs).clone()}
                        resumable={load_games().into_keys().collect::<Vec<_>>()}
                        {on_start}
                        {on_resume}
                        {on_archive}
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

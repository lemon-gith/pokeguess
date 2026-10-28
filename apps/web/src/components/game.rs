use std::rc::Rc;

use serde::{Deserialize, Serialize};
use shared::Language;
use shared::game::{RunSettings, choose_facts, pick_target};
use shared::matching;
use shared::play::{GuessResult, Life, Outcome, Round, Run};
use shared::pokedex::Pokedex;
use shared::runs::RunRecord;
use web_sys::{HtmlButtonElement, HtmlInputElement};
use yew::prelude::*;

use super::clues::Clues;
use crate::{Dex, browser};

/// Name suggestions shown at once (in the modes that offer them).
const SUGGESTIONS: usize = 6;

/// Session storage key for the game in progress, so it survives a refresh.
pub const SAVED_GAME_KEY: &str = "pokeguess.game.v2";

#[derive(Properties, PartialEq)]
pub struct GameProps {
    pub dex: Dex,
    pub settings: RunSettings,
    /// Called once when a run ends, with its record.
    pub on_finish: Callback<RunRecord>,
    /// Back to the menu, without saving the run.
    pub on_quit: Callback<()>,
}

/// Everything about a game in progress that survives a page refresh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedGame {
    pub settings: RunSettings,
    /// Targets already used this run, so they don't come up again.
    seen: Vec<u16>,
    /// Picked ahead of time so its picture can be prefetched.
    next: Option<u16>,
    /// Which Pokémon of the run this is, counting from 1.
    number: u32,
    run: Run,
}

/// What the last action did, if it needs a message.
#[derive(Clone, PartialEq)]
enum Feedback {
    Unknown(String),
    Repeated(String),
    Wrong(String),
    Caught { points: u32, extra_life: bool },
}

#[derive(Clone)]
struct GameState {
    dex: Dex,
    pool: Vec<u16>,
    game: SavedGame,
    feedback: Option<Feedback>,
    /// Set when the run ends.
    finished: Option<RunRecord>,
}

enum Action {
    Guess(String),
    Hint,
    RunAway,
    NextPokemon,
    Restart,
}

fn new_round(dex: &Pokedex, target: u16) -> Round {
    let facts = dex
        .get(target)
        .map(|entry| choose_facts(entry, browser::pick))
        .unwrap_or_default();
    Round::new(target, facts)
}

fn pool(dex: &Pokedex, settings: &RunSettings) -> Vec<u16> {
    let pool = settings.pool(dex);
    if pool.is_empty() {
        dex.pokemon.iter().map(|p| p.id).collect()
    } else {
        pool
    }
}

impl GameState {
    fn new(dex: Dex, settings: RunSettings) -> Self {
        let pool = pool(&dex, &settings);
        let target = pick_target(&pool, &[], browser::pick).unwrap_or(1);
        let next = pick_target(&pool, &[target], browser::pick);
        let run = Run::new(settings.mode, new_round(&dex, target));
        Self {
            game: SavedGame {
                settings,
                seen: vec![target],
                next,
                number: 1,
                run,
            },
            dex,
            pool,
            feedback: None,
            finished: None,
        }
    }

    /// The game saved in this tab, if it's for these settings and still
    /// playable, otherwise a new one.
    fn restore_or_new(dex: Dex, settings: RunSettings) -> Self {
        match browser::load_session::<SavedGame>(SAVED_GAME_KEY) {
            Some(game)
                if game.settings == settings
                    && !game.run.is_over()
                    && dex.get(game.run.round.target).is_some() =>
            {
                Self {
                    pool: pool(&dex, &settings),
                    dex,
                    game,
                    feedback: None,
                    finished: None,
                }
            }
            _ => Self::new(dex, settings),
        }
    }

    fn name(&self, id: u16) -> String {
        self.dex
            .get(id)
            .map_or_else(|| format!("#{id}"), |p| p.display_name.clone())
    }

    fn finish_if_over(&mut self) {
        let run = &self.game.run;
        if run.is_over() && self.finished.is_none() {
            self.finished = Some(RunRecord {
                id: browser::unique_id(),
                mode: run.mode,
                generations: self.game.settings.generations.clone(),
                score: run.score,
                caught: run.caught,
                finished_at: browser::now(),
            });
        }
    }
}

impl Reducible for GameState {
    type Action = Action;

    fn reduce(self: Rc<Self>, action: Action) -> Rc<Self> {
        let mut state = (*self).clone();
        let game = &mut state.game;
        match action {
            Action::Guess(text) => {
                let guessed =
                    matching::find(&text, &state.dex.pokemon, Language::English).map(|p| p.id);
                let name = guessed.map(|id| self.name(id)).unwrap_or_default();
                state.feedback = match game.run.guess(guessed) {
                    GuessResult::Unknown => Some(Feedback::Unknown(text.trim().to_string())),
                    GuessResult::Repeated => Some(Feedback::Repeated(name)),
                    GuessResult::Wrong => Some(Feedback::Wrong(name)),
                    GuessResult::Caught { points, extra_life } => {
                        Some(Feedback::Caught { points, extra_life })
                    }
                    GuessResult::OutOfLives => None,
                    GuessResult::Finished => return self,
                };
            }
            Action::Hint => {
                if !game.run.hint() {
                    return self;
                }
                state.feedback = None;
            }
            Action::RunAway => {
                if !game.run.run_away() {
                    return self;
                }
                state.feedback = None;
            }
            Action::NextPokemon => {
                if game.seen.len() >= state.pool.len() {
                    game.seen.clear();
                }
                let target = game
                    .next
                    .or_else(|| pick_target(&state.pool, &game.seen, browser::pick))
                    .unwrap_or(game.run.round.target);
                if !game.run.next_round(new_round(&state.dex, target)) {
                    return self;
                }
                game.seen.push(target);
                game.next = pick_target(&state.pool, &game.seen, browser::pick);
                game.number += 1;
                state.feedback = None;
            }
            Action::Restart => {
                return Rc::new(GameState::new(state.dex, state.game.settings));
            }
        }
        state.finish_if_over();
        Rc::new(state)
    }
}

#[function_component(Game)]
pub fn game(props: &GameProps) -> Html {
    let state = {
        let dex = props.dex.clone();
        let settings = props.settings.clone();
        use_reducer(move || GameState::restore_or_new(dex, settings))
    };
    let guess = use_state(String::new);
    let input = use_node_ref();
    let continue_button = use_node_ref();

    // keep the game in this tab's storage until it ends
    use_effect_with(state.game.clone(), |game| {
        if game.run.is_over() {
            browser::forget_session(SAVED_GAME_KEY);
        } else {
            browser::save_session(SAVED_GAME_KEY, game);
        }
    });
    // report the run once it ends
    {
        let on_finish = props.on_finish.clone();
        use_effect_with(state.finished.clone(), move |finished| {
            if let Some(run) = finished {
                on_finish.emit(run.clone());
            }
        });
    }
    // download the next Pokémon's picture in advance
    {
        let dex = props.dex.clone();
        use_effect_with(state.game.next, move |next| {
            if let Some(url) = next
                .and_then(|id| dex.get(id))
                .and_then(|p| p.artwork.as_ref())
            {
                browser::prefetch_image(url);
            }
        });
    }
    // keep the keyboard where it's needed: the guess box while guessing,
    // then the continue button, so Enter plays on
    {
        let input = input.clone();
        let continue_button = continue_button.clone();
        let round = &state.game.run.round;
        use_effect_with((round.target, round.outcome), move |(_, outcome)| {
            if outcome.is_some() {
                if let Some(button) = continue_button.cast::<HtmlButtonElement>() {
                    let _ = button.focus();
                }
            } else if let Some(input) = input.cast::<HtmlInputElement>() {
                let _ = input.focus();
            }
        });
    }

    let run = &state.game.run;
    let round = &run.round;
    let mode = run.mode;
    let Some(entry) = props.dex.get(round.target).cloned() else {
        return html! { <p>{ "Something went wrong picking a Pokémon." }</p> };
    };

    let dispatch = |make: fn() -> Action| {
        let state = state.clone();
        Callback::from(move |_| state.dispatch(make()))
    };
    let onsubmit = {
        let state = state.clone();
        let guess = guess.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if !guess.trim().is_empty() {
                state.dispatch(Action::Guess((*guess).clone()));
                guess.set(String::new());
            }
        })
    };
    let oninput = {
        let guess = guess.clone();
        Callback::from(move |e: InputEvent| {
            guess.set(e.target_unchecked_into::<HtmlInputElement>().value());
        })
    };
    let quit = {
        let on_quit = props.on_quit.clone();
        Callback::from(move |_| on_quit.emit(()))
    };

    let lives = (0..run.max_lives()).map(|i| match run.lives.get(i) {
        Some(Life::PokeBall) => html! {
            <img src="img/icons/pokeball.svg" alt="life" class="size-6" />
        },
        Some(Life::PremierBall) => html! {
            <img
                src="img/icons/premier-ball.png"
                alt="life won back"
                class="size-7 [image-rendering:pixelated]"
            />
        },
        None => html! {
            <img src="img/icons/pokeball.svg" alt="lost life" class="size-6 opacity-25 grayscale" />
        },
    });

    let feedback = state.feedback.as_ref().and_then(|feedback| {
        let message = match feedback {
            Feedback::Unknown(text) => format!("Hmm, I don't know a Pokémon called “{text}”."),
            Feedback::Repeated(name) => format!("You've already guessed {name}."),
            Feedback::Wrong(name) => format!("Not {name}!"),
            Feedback::Caught { .. } => return None,
        };
        Some(html! { <p class="text-center text-amber-700" role="status">{ message }</p> })
    });

    let wrong_guesses = (!round.wrong_guesses.is_empty()).then(|| {
        let names: Vec<String> = round.wrong_guesses.iter().map(|&id| state.name(id)).collect();
        html! { <p class="text-center text-sm text-gray-600">{ format!("Not: {}", names.join(", ")) }</p> }
    });

    let suggestions = if mode.suggests_names() && round.outcome.is_none() {
        matching::suggestions(&guess, &props.dex.pokemon, Language::English, SUGGESTIONS)
    } else {
        Vec::new()
    };
    // nothing to suggest once the guess is already a full name
    let suggestions = match suggestions.as_slice() {
        [only] if matching::matches(&guess, only, Language::English) => Vec::new(),
        _ => suggestions,
    };
    let suggestions = (!suggestions.is_empty()).then(|| {
        html! {
            <div class="flex gap-2 overflow-x-auto pb-1" aria-label="Suggestions">
                { for suggestions.iter().map(|entry| {
                    let name = entry.display_name.clone();
                    let onclick = {
                        let guess = guess.clone();
                        let name = name.clone();
                        Callback::from(move |_| guess.set(name.clone()))
                    };
                    html! {
                        <button
                            type="button"
                            class="shrink-0 cursor-pointer rounded-full border-2 border-blue-200 bg-blue-50 px-3 py-1.5 text-sm hover:border-blue-400"
                            // keep focus (and the keyboard) on the guess box
                            onpointerdown={Callback::from(|e: PointerEvent| e.prevent_default())}
                            {onclick}
                        >
                            { name }
                        </button>
                    }
                }) }
            </div>
        }
    });

    let controls = match round.outcome {
        None => {
            let hint_label = if round.hints_left() == 0 {
                "No hints left"
            } else {
                "Hint (−1 life)"
            };
            let run_away_ends_run = run.running_away_ends_run();
            html! {
                <>
                    <div class="flex gap-2">
                        <button
                            type="button"
                            class="flex-1 cursor-pointer rounded-lg border-2 border-blue-600 px-3 py-2 text-sm font-semibold text-blue-700 hover:bg-blue-50 disabled:cursor-not-allowed disabled:border-gray-300 disabled:text-gray-400 disabled:hover:bg-transparent"
                            disabled={!run.can_hint()}
                            title={if run.lives.len() <= 1 { "Hints can't use your last life" } else { "Reveal the next clue" }}
                            onclick={dispatch(|| Action::Hint)}
                        >
                            { hint_label }
                        </button>
                        <button
                            type="button"
                            class={classes!(
                                "flex-1", "cursor-pointer", "rounded-lg", "px-3", "py-2", "text-sm", "font-semibold", "text-white",
                                if run_away_ends_run { "bg-red-600 hover:bg-red-700" } else { "bg-blue-600 hover:bg-blue-700" },
                            )}
                            title={if run_away_ends_run { "Your last life: running away ends the run" } else { "Skip this Pokémon (−1 life)" }}
                            onclick={dispatch(|| Action::RunAway)}
                        >
                            { if run_away_ends_run { "Run (ends the run)" } else { "Run (−1 life)" } }
                        </button>
                    </div>
                    <form class="flex gap-2" {onsubmit}>
                        <input
                            ref={input}
                            type="text"
                            autocomplete="off"
                            autocapitalize="off"
                            autocorrect="off"
                            spellcheck="false"
                            enterkeyhint="go"
                            placeholder="Who's that Pokémon?"
                            aria-label="Your guess"
                            class="min-w-0 flex-1 rounded-lg border-2 border-gray-300 px-3 py-2.5 text-base focus:border-blue-500 focus:outline-none"
                            value={(*guess).clone()}
                            {oninput}
                        />
                        <button type="submit" class="cursor-pointer rounded-lg bg-blue-600 px-4 py-2.5 font-semibold text-white hover:bg-blue-700">
                            { "Guess" }
                        </button>
                    </form>
                </>
            }
        }
        Some(Outcome::Caught) => {
            let (points, extra_life) = match state.feedback {
                Some(Feedback::Caught { points, extra_life }) => (Some(points), extra_life),
                _ => (None, false),
            };
            html! {
                <div class="space-y-3 text-center">
                    <p class="text-lg font-semibold text-green-700">
                        { match points {
                            Some(points) => format!("Got it! +{points} points"),
                            None => "Got it!".to_string(),
                        } }
                    </p>
                    if extra_life {
                        <p class="flex items-center justify-center gap-2 text-blue-700">
                            <img src="img/icons/premier-ball.png" alt="" class="size-7 [image-rendering:pixelated]" />
                            { "10 in a row! A Premier Ball restores a life." }
                        </p>
                    }
                    <button ref={continue_button} type="button" class="cursor-pointer rounded-lg bg-blue-600 px-6 py-2 font-semibold text-white hover:bg-blue-700" onclick={dispatch(|| Action::NextPokemon)}>
                        { "Next Pokémon" }
                    </button>
                </div>
            }
        }
        Some(Outcome::RanAway) => html! {
            <div class="space-y-3 text-center">
                <p class="text-lg font-semibold text-gray-700">{ "Got away safely!" }</p>
                <button ref={continue_button} type="button" class="cursor-pointer rounded-lg bg-blue-600 px-6 py-2 font-semibold text-white hover:bg-blue-700" onclick={dispatch(|| Action::NextPokemon)}>
                    { "Next Pokémon" }
                </button>
            </div>
        },
        Some(Outcome::OutOfLives) => html! {
            <div class="space-y-3 text-center">
                <p class="text-lg font-semibold text-red-700">{ "Out of lives!" }</p>
                <p>{ format!("Run over: {} points, {} caught.", run.score, run.caught) }</p>
                <div class="flex justify-center gap-2">
                    <button ref={continue_button} type="button" class="cursor-pointer rounded-lg bg-blue-600 px-6 py-2 font-semibold text-white hover:bg-blue-700" onclick={dispatch(|| Action::Restart)}>
                        { "Play again" }
                    </button>
                    <button type="button" class="cursor-pointer rounded-lg border-2 border-gray-300 px-6 py-2 font-semibold hover:border-gray-500" onclick={quit.clone()}>
                        { "Menu" }
                    </button>
                </div>
            </div>
        },
    };

    let score_line = if mode.earns_lives() && run.streak > 0 {
        format!("Score: {} · Streak: {}", run.score, run.streak)
    } else {
        format!("Score: {}", run.score)
    };

    // on phones: header at the top, guess box pinned to the bottom (above the
    // keyboard), and the clues in between, scrolling if they don't fit
    html! {
        <div class="flex min-h-0 flex-1 flex-col gap-3 md:gap-5">
            <header class="flex shrink-0 items-center justify-between gap-2">
                <div>
                    <p class="font-semibold">{ format!("{} · Pokémon #{}", mode.label(), state.game.number) }</p>
                    <p class="text-sm text-gray-600">{ score_line }</p>
                </div>
                <div class="flex items-center gap-1" aria-label={format!("{} lives left", run.lives.len())}>
                    { for lives }
                </div>
                // hidden rather than removed, so the header doesn't shift
                <button
                    type="button"
                    class={classes!("cursor-pointer", "text-sm", "text-gray-600", "underline", run.is_over().then_some("invisible"))}
                    onclick={quit}
                >
                    { "Quit" }
                </button>
            </header>

            <div class="flex min-h-0 flex-1 flex-col overflow-y-auto md:flex-none md:overflow-visible">
                <div class="my-auto">
                    <Clues
                        key={state.game.number}
                        entry={entry}
                        clues={round.clues(mode)}
                        facts={round.facts.clone()}
                        revealed={round.outcome.is_some()}
                    />
                </div>
            </div>

            <div class="shrink-0 space-y-2 pb-[env(safe-area-inset-bottom)]">
                { for feedback }
                { for wrong_guesses }
                { for suggestions }
                { controls }
            </div>
        </div>
    }
}

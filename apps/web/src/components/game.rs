use std::rc::Rc;

use shared::Language;
use shared::game::{
    GuessResult, MAX_GUESSES, Outcome, Round, RunSettings, choose_facts, pick_target,
};
use shared::matching;
use shared::pokedex::Pokedex;
use shared::runs::RunRecord;
use web_sys::{HtmlButtonElement, HtmlInputElement};
use yew::prelude::*;

use super::clues::Clues;
use crate::{Dex, browser};

#[derive(Properties, PartialEq)]
pub struct GameProps {
    pub dex: Dex,
    pub settings: RunSettings,
    /// Called once when a run ends, with its record.
    pub on_finish: Callback<RunRecord>,
    /// Back to the menu, without saving the run.
    pub on_quit: Callback<()>,
}

/// What the last guess did, if it needs a message.
#[derive(Clone, PartialEq)]
enum Feedback {
    Unknown(String),
    Repeated(String),
    Wrong(String),
}

#[derive(Clone)]
struct RunState {
    dex: Dex,
    settings: RunSettings,
    pool: Vec<u16>,
    /// Targets already used this run, so they don't come up again.
    seen: Vec<u16>,
    round: Round,
    /// Picked ahead of time so its picture can be prefetched.
    next: Option<u16>,
    score: u32,
    rounds_won: u32,
    feedback: Option<Feedback>,
    /// Set when the run ends.
    finished: Option<RunRecord>,
}

enum Action {
    Guess(String),
    NextRound,
    Restart,
}

fn new_round(dex: &Pokedex, target: u16) -> Round {
    let facts = dex
        .get(target)
        .map(|entry| choose_facts(entry, browser::pick))
        .unwrap_or_default();
    Round::new(target, facts)
}

impl RunState {
    fn new(dex: Dex, settings: RunSettings) -> Self {
        let mut pool = settings.pool(&dex);
        if pool.is_empty() {
            pool = dex.pokemon.iter().map(|p| p.id).collect();
        }
        let target = pick_target(&pool, &[], browser::pick).unwrap_or(1);
        let next = pick_target(&pool, &[target], browser::pick);
        Self {
            round: new_round(&dex, target),
            dex,
            settings,
            pool,
            seen: vec![target],
            next,
            score: 0,
            rounds_won: 0,
            feedback: None,
            finished: None,
        }
    }

    fn name(&self, id: u16) -> String {
        self.dex
            .get(id)
            .map_or_else(|| format!("#{id}"), |p| p.display_name.clone())
    }
}

impl Reducible for RunState {
    type Action = Action;

    fn reduce(self: Rc<Self>, action: Action) -> Rc<Self> {
        let mut state = (*self).clone();
        match action {
            Action::Guess(text) => {
                let guessed =
                    matching::find(&text, &state.dex.pokemon, Language::English).map(|p| p.id);
                state.feedback = match state.round.guess(guessed) {
                    GuessResult::Unknown => Some(Feedback::Unknown(text.trim().to_string())),
                    GuessResult::Repeated => guessed.map(|id| Feedback::Repeated(state.name(id))),
                    GuessResult::Wrong => guessed.map(|id| Feedback::Wrong(state.name(id))),
                    GuessResult::Correct => {
                        state.score += state.round.points();
                        state.rounds_won += 1;
                        None
                    }
                    GuessResult::Missed => {
                        state.finished = Some(RunRecord {
                            id: browser::unique_id(),
                            mode: state.settings.mode,
                            generations: state.settings.generations.clone(),
                            score: state.score,
                            rounds_won: state.rounds_won,
                            finished_at: browser::now(),
                        });
                        None
                    }
                    GuessResult::Finished => return self,
                };
            }
            Action::NextRound => {
                if state.round.outcome != Some(Outcome::Guessed) {
                    return self;
                }
                if state.seen.len() >= state.pool.len() {
                    state.seen.clear();
                }
                let target = state
                    .next
                    .or_else(|| pick_target(&state.pool, &state.seen, browser::pick))
                    .unwrap_or(state.round.target);
                state.seen.push(target);
                state.round = new_round(&state.dex, target);
                state.next = pick_target(&state.pool, &state.seen, browser::pick);
                state.feedback = None;
            }
            Action::Restart => return Rc::new(RunState::new(state.dex, state.settings)),
        }
        Rc::new(state)
    }
}

#[function_component(Game)]
pub fn game(props: &GameProps) -> Html {
    let state = {
        let dex = props.dex.clone();
        let settings = props.settings.clone();
        use_reducer(move || RunState::new(dex, settings))
    };
    let guess = use_state(String::new);
    let input = use_node_ref();
    let continue_button = use_node_ref();

    // report the run once it ends
    {
        let on_finish = props.on_finish.clone();
        use_effect_with(state.finished.clone(), move |finished| {
            if let Some(run) = finished {
                on_finish.emit(run.clone());
            }
        });
    }
    // download the next round's picture in advance
    {
        let dex = props.dex.clone();
        use_effect_with(state.next, move |next| {
            if let Some(url) = next
                .and_then(|id| dex.get(id))
                .and_then(|p| p.artwork.as_ref())
            {
                browser::prefetch_image(url);
            }
        });
    }
    // keep the keyboard where it's needed: the guess box during a round,
    // then the continue button, so Enter plays on
    {
        let input = input.clone();
        let continue_button = continue_button.clone();
        use_effect_with(
            (state.round.target, state.round.outcome),
            move |(_, outcome)| {
                if outcome.is_some() {
                    if let Some(button) = continue_button.cast::<HtmlButtonElement>() {
                        let _ = button.focus();
                    }
                } else if let Some(input) = input.cast::<HtmlInputElement>() {
                    let _ = input.focus();
                }
            },
        );
    }
    let names = use_memo(props.dex.clone(), |dex| {
        let mut names: Vec<String> = dex.pokemon.iter().map(|p| p.display_name.clone()).collect();
        names.sort_unstable();
        names
    });

    let Some(entry) = props.dex.get(state.round.target).cloned() else {
        return html! { <p>{ "Something went wrong picking a Pokémon." }</p> };
    };
    let round = &state.round;
    let mode = state.settings.mode;
    // a won round already counts in rounds_won
    let round_number = state.rounds_won + u32::from(round.outcome != Some(Outcome::Guessed));

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
    let next_round = {
        let state = state.clone();
        Callback::from(move |_| state.dispatch(Action::NextRound))
    };
    let restart = {
        let state = state.clone();
        Callback::from(move |_| state.dispatch(Action::Restart))
    };
    let quit = {
        let on_quit = props.on_quit.clone();
        Callback::from(move |_| on_quit.emit(()))
    };

    let strikes = (0..MAX_GUESSES).map(|i| {
        let used = i < round.wrong_guesses.len();
        html! {
            <img
                src="img/icons/pokeball.svg"
                alt={if used { "used guess" } else { "guess left" }}
                class={classes!("h-6", "w-6", used.then_some("opacity-25 grayscale"))}
            />
        }
    });

    let feedback = state.feedback.as_ref().map(|feedback| {
        let message = match feedback {
            Feedback::Unknown(text) => format!("Hmm, I don't know a Pokémon called “{text}”."),
            Feedback::Repeated(name) => format!("You've already guessed {name}."),
            Feedback::Wrong(name) => match round.guesses_left() {
                1 => format!("Not {name}! Last guess…"),
                left => format!("Not {name}! {left} guesses left."),
            },
        };
        html! { <p class="text-center text-amber-700" role="status">{ message }</p> }
    });

    let wrong_guesses = (!round.wrong_guesses.is_empty()).then(|| {
        let names: Vec<String> = round.wrong_guesses.iter().map(|&id| state.name(id)).collect();
        html! { <p class="text-center text-sm text-gray-600">{ format!("Not: {}", names.join(", ")) }</p> }
    });

    let controls = match round.outcome {
        None => html! {
            <form class="flex gap-2" {onsubmit}>
                <input
                    ref={input}
                    type="text"
                    list="pokemon-names"
                    autocomplete="off"
                    autocapitalize="off"
                    spellcheck="false"
                    placeholder="Who's that Pokémon?"
                    aria-label="Your guess"
                    class="min-w-0 flex-1 rounded-lg border-2 border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none"
                    value={(*guess).clone()}
                    {oninput}
                />
                <button type="submit" class="cursor-pointer rounded-lg bg-blue-600 px-4 py-2 font-semibold text-white hover:bg-blue-700">
                    { "Guess" }
                </button>
                <datalist id="pokemon-names">
                    { for names.iter().map(|name| html! { <option value={name.clone()} /> }) }
                </datalist>
            </form>
        },
        Some(Outcome::Guessed) => html! {
            <div class="space-y-3 text-center">
                <p class="text-lg font-semibold text-green-700">
                    { match round.points() {
                        1 => "Got it on the last guess! +1 point".to_string(),
                        points => format!("Got it! +{points} points"),
                    } }
                </p>
                <button ref={continue_button} type="button" class="cursor-pointer rounded-lg bg-blue-600 px-6 py-2 font-semibold text-white hover:bg-blue-700" onclick={next_round}>
                    { "Next Pokémon" }
                </button>
            </div>
        },
        Some(Outcome::Missed) => html! {
            <div class="space-y-3 text-center">
                <p class="text-lg font-semibold text-red-700">{ "It got away!" }</p>
                <p>{ format!("Run over: {} points, {} caught.", state.score, state.rounds_won) }</p>
                <div class="flex justify-center gap-2">
                    <button ref={continue_button} type="button" class="cursor-pointer rounded-lg bg-blue-600 px-6 py-2 font-semibold text-white hover:bg-blue-700" onclick={restart}>
                        { "Play again" }
                    </button>
                    <button type="button" class="cursor-pointer rounded-lg border-2 border-gray-300 px-6 py-2 font-semibold hover:border-gray-500" onclick={quit.clone()}>
                        { "Menu" }
                    </button>
                </div>
            </div>
        },
    };

    html! {
        <div class="space-y-5">
            <header class="flex items-center justify-between gap-2">
                <div>
                    <p class="font-semibold">{ format!("{} · Pokémon #{}", mode.label(), round_number) }</p>
                    <p class="text-sm text-gray-600">{ format!("Score: {}", state.score) }</p>
                </div>
                <div class="flex gap-1">{ for strikes }</div>
                // hidden rather than removed, so the header doesn't shift
                <button
                    type="button"
                    class={classes!("cursor-pointer", "text-sm", "text-gray-600", "underline", round.outcome.is_some().then_some("invisible"))}
                    onclick={quit}
                >
                    { "Quit" }
                </button>
            </header>

            <Clues
                key={round.target}
                entry={entry}
                clues={round.clues(mode)}
                facts={round.facts.clone()}
                revealed={round.outcome.is_some()}
            />

            { for feedback }
            { for wrong_guesses }
            { controls }
        </div>
    }
}

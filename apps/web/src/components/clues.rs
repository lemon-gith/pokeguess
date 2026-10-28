use shared::game::{Clue, Fact};
use shared::pokedex::{PokedexEntry, REDACTED_NAME};
use yew::prelude::*;

/// Up to 12rem, but smaller on short screens (or above a keyboard).
const IMAGE_SIZE: &str = "size-[min(12rem,calc(var(--app-height,100dvh)*0.3))]";

#[derive(Properties, PartialEq)]
pub struct CluesProps {
    pub entry: PokedexEntry,
    /// The clues revealed so far.
    pub clues: Vec<Clue>,
    pub facts: Vec<Fact>,
    /// The round is over, so show everything, name included.
    pub revealed: bool,
}

#[function_component(Clues)]
pub fn clues(props: &CluesProps) -> Html {
    let entry = &props.entry;
    let shows = |clue| props.clues.contains(&clue);
    let image_url = entry.artwork.clone().or_else(|| entry.sprite.clone());

    // one <img> for shadow, picture and reveal, so the filter change animates
    let image = match image_url {
        Some(src) if props.revealed || shows(Clue::Picture) || shows(Clue::Shadow) => {
            let shadow = !props.revealed && !shows(Clue::Picture);
            let alt = if props.revealed {
                entry.display_name.clone()
            } else {
                "The mystery Pokémon".to_string()
            };
            html! {
                <img
                    {src}
                    {alt}
                    draggable="false"
                    class={classes!(IMAGE_SIZE, "select-none", "object-contain", "transition-[filter]", "duration-700")}
                    style={if shadow { "filter: brightness(0)" } else { "filter: none" }}
                />
            }
        }
        _ => html! {
            <div class={classes!(IMAGE_SIZE, "flex", "items-center", "justify-center", "rounded-full", "bg-gray-100", "text-6xl", "font-bold", "text-gray-300")}>
                { "?" }
            </div>
        },
    };

    let description = (props.revealed || shows(Clue::Description))
        .then_some(entry.flavor_text.as_deref())
        .flatten()
        .map(|text| {
            let name = props.revealed.then_some(entry.display_name.as_str());
            html! {
                <blockquote class="border-l-4 border-blue-300 pl-3 italic text-gray-800">
                    { redacted_text(text, name) }
                </blockquote>
            }
        });

    let first_letter = (shows(Clue::FirstLetter) && !props.revealed)
        .then(|| entry.display_name.chars().next())
        .flatten()
        .map(|letter| {
            html! {
                <p class="text-gray-800">
                    { "Its name starts with " }
                    <span class="rounded bg-gray-200 px-1.5 font-mono font-bold">
                        { letter.to_uppercase().collect::<String>() }
                    </span>
                </p>
            }
        });

    let facts = shows(Clue::Facts).then(|| {
        html! {
            <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
                { for props.facts.iter().filter_map(|&fact| {
                    let value = fact.value(entry)?;
                    Some(html! {
                        <>
                            <dt class="font-semibold text-gray-600">{ fact.label() }</dt>
                            <dd>{ value }</dd>
                        </>
                    })
                }) }
            </dl>
        }
    });

    html! {
        <div class="flex flex-col items-center gap-4">
            { image }
            if props.revealed {
                <div class="text-center">
                    <p class="text-2xl font-bold">{ &entry.display_name }</p>
                    if let Some(genus) = &entry.genus {
                        <p class="text-gray-600">{ genus }</p>
                    }
                </div>
            }
            if let Some(facts) = facts {
                { facts }
            }
            if let Some(first_letter) = first_letter {
                { first_letter }
            }
            if let Some(description) = description {
                { description }
            }
        </div>
    }
}

/// Flavour text with each redacted name shown as a blank, or as `name`.
fn redacted_text(text: &str, name: Option<&str>) -> Html {
    let mut parts = text.split(REDACTED_NAME);
    let first = parts.next().unwrap_or_default().to_string();
    let blank = match name {
        Some(name) => html! { <strong class="not-italic">{ name }</strong> },
        None => html! {
            <span class="rounded bg-gray-200 px-1 font-mono not-italic">{ REDACTED_NAME }</span>
        },
    };
    html! {
        <>
            { first }
            { for parts.map(|part| html! { <>{ blank.clone() }{ part.to_string() }</> }) }
        </>
    }
}

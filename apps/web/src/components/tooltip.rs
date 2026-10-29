use std::cell::RefCell;
use std::rc::Rc;

use gloo::timers::callback::Timeout;
use yew::prelude::*;

use crate::browser;

/// How long a press must last to show the tooltip instead of clicking.
const LONG_PRESS_MS: u32 = 500;
/// How long the tooltip stays up after a long press ends.
const LINGER_MS: u32 = 2000;

#[derive(Properties, PartialEq)]
pub struct TooltipButtonProps {
    pub label: AttrValue,
    /// Extra detail: shown on hover or keyboard focus, or on a long press.
    pub tooltip: AttrValue,
    pub class: Classes,
    #[prop_or_default]
    pub disabled: bool,
    pub onclick: Callback<()>,
}

/// A button with a tooltip that works on touch screens too: a long press
/// shows the tooltip without clicking.
#[function_component(TooltipButton)]
pub fn tooltip_button(props: &TooltipButtonProps) -> Html {
    let shown = use_state(|| false);
    let timer = use_mut_ref(|| None::<Timeout>);
    let long_pressed = use_mut_ref(|| false);
    let id = use_memo((), |_| format!("tooltip-{}", browser::unique_id()));

    let onpointerdown = {
        let shown = shown.clone();
        let timer = timer.clone();
        let long_pressed = long_pressed.clone();
        Callback::from(move |e: PointerEvent| {
            if e.pointer_type() == "mouse" {
                return;
            }
            *long_pressed.borrow_mut() = false;
            let shown = shown.clone();
            let long_pressed = long_pressed.clone();
            *timer.borrow_mut() = Some(Timeout::new(LONG_PRESS_MS, move || {
                *long_pressed.borrow_mut() = true;
                shown.set(true);
            }));
        })
    };
    // a press ending: cancel a pending long press, or hide the tooltip soon
    let release = {
        let shown = shown.clone();
        let timer = timer.clone();
        Callback::from(move |_: PointerEvent| {
            let hide = shown.clone();
            *timer.borrow_mut() =
                (*shown).then(|| Timeout::new(LINGER_MS, move || hide.set(false)));
        })
    };
    let onclick = {
        let long_pressed: Rc<RefCell<bool>> = long_pressed.clone();
        let onclick = props.onclick.clone();
        let disabled = props.disabled;
        Callback::from(move |_: MouseEvent| {
            // the click that ends a long press isn't a click
            if long_pressed.replace(false) || disabled {
                return;
            }
            onclick.emit(());
        })
    };

    let visibility = if *shown {
        "block"
    } else {
        "hidden group-hover:block group-has-focus-visible:block"
    };

    html! {
        <div
            class="group relative flex-1 select-none [-webkit-touch-callout:none]"
            {onpointerdown}
            onpointerup={release.clone()}
            onpointercancel={release.clone()}
            onpointerleave={release}
            oncontextmenu={Callback::from(|e: MouseEvent| e.prevent_default())}
        >
            <button
                type="button"
                class={classes!(props.class.clone(), "w-full", "disabled:pointer-events-none")}
                disabled={props.disabled}
                aria-describedby={(*id).clone()}
                {onclick}
            >
                { &props.label }
            </button>
            <div
                id={(*id).clone()}
                role="tooltip"
                class={classes!(
                    // as wide as the button, so it never runs off the screen
                    "pointer-events-none", "absolute", "inset-x-0", "bottom-full", "z-10", "mb-2",
                    "rounded-md", "bg-gray-900", "px-3", "py-1.5",
                    "text-center", "text-xs", "text-white", "shadow", visibility,
                )}
            >
                { &props.tooltip }
            </div>
        </div>
    }
}

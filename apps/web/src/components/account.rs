//! Signing in, from the menu: a button in the header, and a panel that
//! either signs the player in (one form, "Log in" or "Create account") or
//! shows their account (log out, change password, delete account).

use shared::account::{
    ChangePassword, Credentials, DeleteAccount, Me, PASSWORD_MIN, USERNAME_MAX, USERNAME_MIN,
    validate_password, validate_username,
};
use web_sys::HtmlInputElement;
use yew::platform::spawn_local;
use yew::prelude::*;

use crate::{api, browser};

/// Who's signed in, as far as the web app knows.
#[derive(Debug, Clone, PartialEq)]
pub enum Account {
    /// Asking the service, when the app loads.
    Checking,
    SignedOut,
    SignedIn(Me),
    /// The service couldn't be reached; signing in can still be tried.
    Unavailable,
}

/// Asks the service who's signed in on this browser.
pub fn check(account: UseStateHandle<Account>) {
    spawn_local(async move {
        account.set(match api::me().await {
            Ok(Some(me)) => Account::SignedIn(me),
            Ok(None) => Account::SignedOut,
            Err(_) => Account::Unavailable,
        });
    });
}

const INPUT: &str = "w-full rounded-lg border-2 border-gray-300 px-3 py-2 text-base focus:border-blue-500 focus:outline-none";
const PRIMARY: &str = "cursor-pointer rounded-lg bg-blue-600 px-4 py-2 font-semibold text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:bg-gray-400";
const SECONDARY: &str = "cursor-pointer rounded-lg border-2 border-blue-600 px-4 py-2 font-semibold text-blue-700 hover:bg-blue-50 disabled:cursor-not-allowed disabled:border-gray-300 disabled:text-gray-400";

#[derive(Properties, PartialEq)]
pub struct AccountButtonProps {
    pub account: Account,
    pub onclick: Callback<()>,
}

/// "Sign in", or the signed-in player's name.
#[function_component(AccountButton)]
pub fn account_button(props: &AccountButtonProps) -> Html {
    let label = match &props.account {
        Account::Checking => return html! { <span class="h-9" aria-hidden="true"></span> },
        Account::SignedIn(me) => me.username.clone(),
        Account::SignedOut | Account::Unavailable => "Sign in".to_string(),
    };
    let onclick = props.onclick.reform(|_| ());
    html! {
        <button
            type="button"
            class="cursor-pointer rounded-full border-2 border-gray-200 bg-white px-3 py-1 text-sm font-semibold text-gray-700 hover:border-gray-400"
            aria-haspopup="dialog"
            {onclick}
        >
            { label }
        </button>
    }
}

#[derive(Properties, PartialEq)]
pub struct AccountPanelProps {
    pub account: Account,
    pub on_change: Callback<Account>,
    pub on_close: Callback<()>,
}

/// A text field bound to a piece of state.
fn field(
    id: &'static str,
    label: &'static str,
    kind: &'static str,
    autocomplete: &'static str,
    value: &UseStateHandle<String>,
    node: Option<NodeRef>,
) -> Html {
    let oninput = {
        let value = value.clone();
        Callback::from(move |e: InputEvent| {
            value.set(e.target_unchecked_into::<HtmlInputElement>().value())
        })
    };
    html! {
        <label class="block space-y-1" for={id}>
            <span class="block text-sm font-semibold">{ label }</span>
            <input
                {id}
                ref={node.unwrap_or_default()}
                type={kind}
                class={INPUT}
                {autocomplete}
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
                value={(**value).clone()}
                {oninput}
            />
        </label>
    }
}

/// The account dialog, over the menu.
#[function_component(AccountPanel)]
pub fn account_panel(props: &AccountPanelProps) -> Html {
    let username = use_state(String::new);
    let password = use_state(String::new);
    let current = use_state(String::new);
    let new_password = use_state(String::new);
    let delete_password = use_state(String::new);
    let error = use_state(|| None::<String>);
    let notice = use_state(|| None::<String>);
    let busy = use_state(|| false);
    let first_field = use_node_ref();
    let dialog = use_node_ref();

    // a modal <dialog> sits above the whole page (sticky bars included),
    // keeps focus inside, and closes on Escape
    {
        let dialog = dialog.clone();
        use_effect_with((), move |_| {
            if let Some(dialog) = dialog.cast::<web_sys::HtmlDialogElement>() {
                let _ = dialog.show_modal();
            }
        });
    }
    // start typing straight away
    {
        let first_field = first_field.clone();
        let signed_in = matches!(props.account, Account::SignedIn(_));
        use_effect_with(signed_in, move |_| {
            if let Some(element) = first_field.cast::<web_sys::HtmlElement>() {
                let _ = element.focus();
            }
        });
    }

    // runs an action, showing it as busy and any problem as the error
    let run = {
        let error = error.clone();
        let notice = notice.clone();
        let busy = busy.clone();
        move |action: std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Option<String>, String>>>,
        >| {
            let error = error.clone();
            let notice = notice.clone();
            let busy = busy.clone();
            busy.set(true);
            error.set(None);
            notice.set(None);
            spawn_local(async move {
                match action.await {
                    Ok(message) => notice.set(message),
                    Err(message) => error.set(Some(message)),
                }
                busy.set(false);
            });
        }
    };

    let close = props.on_close.reform(|_: MouseEvent| ());
    // Escape: let the app close it, so its state stays in charge
    let oncancel = {
        let on_close = props.on_close.clone();
        Callback::from(move |e: Event| {
            e.prevent_default();
            on_close.emit(());
        })
    };
    // the dialog itself is only ever clicked on its backdrop: its contents
    // fill it
    let backdrop = {
        let on_close = props.on_close.clone();
        let dialog = dialog.clone();
        Callback::from(move |e: MouseEvent| {
            let on_dialog = e
                .target()
                .zip(dialog.get())
                .is_some_and(|(target, dialog)| target == dialog.into());
            if on_dialog {
                on_close.emit(());
            }
        })
    };

    let body = match &props.account {
        Account::SignedIn(me) => {
            let log_out = {
                let run = run.clone();
                let on_change = props.on_change.clone();
                Callback::from(move |_: MouseEvent| {
                    let on_change = on_change.clone();
                    run(Box::pin(async move {
                        api::log_out().await.map_err(|p| p.message)?;
                        on_change.emit(Account::SignedOut);
                        Ok(None)
                    }))
                })
            };
            let change = {
                let run = run.clone();
                let current = current.clone();
                let new_password = new_password.clone();
                Callback::from(move |e: SubmitEvent| {
                    e.prevent_default();
                    let change = ChangePassword {
                        current_password: (*current).clone(),
                        new_password: (*new_password).clone(),
                    };
                    let (current, new_password) = (current.clone(), new_password.clone());
                    run(Box::pin(async move {
                        validate_password(&change.new_password)?;
                        api::change_password(&change).await.map_err(|p| p.message)?;
                        current.set(String::new());
                        new_password.set(String::new());
                        Ok(Some(
                            "Password changed. Any other devices have been signed out.".into(),
                        ))
                    }))
                })
            };
            let delete = {
                let run = run.clone();
                let delete_password = delete_password.clone();
                let on_change = props.on_change.clone();
                Callback::from(move |e: SubmitEvent| {
                    e.prevent_default();
                    let delete = DeleteAccount {
                        password: (*delete_password).clone(),
                    };
                    let on_change = on_change.clone();
                    run(Box::pin(async move {
                        api::delete_account(&delete).await.map_err(|p| p.message)?;
                        on_change.emit(Account::SignedOut);
                        Ok(Some("Your account has been deleted.".into()))
                    }))
                })
            };
            html! {
                <>
                    <h2 id="account-title" class="text-xl font-bold">{ "Your account" }</h2>
                    <p>
                        { "Signed in as " }<span class="font-semibold">{ &me.username }</span>
                        <span class="block text-sm text-gray-600">
                            { format!("Since {}", browser::format_date(me.created_at)) }
                        </span>
                    </p>
                    <button ref={first_field.clone()} type="button" class={SECONDARY} disabled={*busy} onclick={log_out}>
                        { "Log out" }
                    </button>
                    <details class="rounded-lg border border-gray-200 p-3">
                        <summary class="cursor-pointer font-semibold">{ "Change password" }</summary>
                        <form class="mt-3 space-y-3" onsubmit={change}>
                            { field("current-password", "Current password", "password", "current-password", &current, None) }
                            { field("new-password", "New password", "password", "new-password", &new_password, None) }
                            <button type="submit" class={PRIMARY} disabled={*busy}>{ "Change password" }</button>
                        </form>
                    </details>
                    <details class="rounded-lg border border-red-200 p-3">
                        <summary class="cursor-pointer font-semibold text-red-700">{ "Delete account" }</summary>
                        <form class="mt-3 space-y-3" onsubmit={delete}>
                            <p class="text-sm text-gray-700">{ "This deletes your account for good. It can't be undone." }</p>
                            { field("delete-password", "Password", "password", "current-password", &delete_password, None) }
                            <button type="submit" class="cursor-pointer rounded-lg bg-red-600 px-4 py-2 font-semibold text-white hover:bg-red-700 disabled:cursor-not-allowed disabled:bg-gray-400" disabled={*busy}>
                                { "Delete my account" }
                            </button>
                        </form>
                    </details>
                </>
            }
        }
        _ => {
            let credentials = {
                let username = username.clone();
                let password = password.clone();
                move || Credentials {
                    username: username.trim().to_string(),
                    password: (*password).clone(),
                }
            };
            let log_in = {
                let run = run.clone();
                let credentials = credentials.clone();
                let on_change = props.on_change.clone();
                let password = password.clone();
                Callback::from(move |e: SubmitEvent| {
                    e.prevent_default();
                    let credentials = credentials();
                    let (on_change, password) = (on_change.clone(), password.clone());
                    run(Box::pin(async move {
                        if credentials.username.is_empty() || credentials.password.is_empty() {
                            return Err("Enter your username and password.".into());
                        }
                        let me = api::log_in(&credentials).await.map_err(|p| p.message)?;
                        password.set(String::new());
                        on_change.emit(Account::SignedIn(me));
                        Ok(None)
                    }))
                })
            };
            let sign_up = {
                let run = run.clone();
                let on_change = props.on_change.clone();
                let password = password.clone();
                Callback::from(move |_: MouseEvent| {
                    let credentials = credentials();
                    let (on_change, password) = (on_change.clone(), password.clone());
                    run(Box::pin(async move {
                        validate_username(&credentials.username)?;
                        validate_password(&credentials.password)?;
                        let me = api::sign_up(&credentials).await.map_err(|p| p.message)?;
                        password.set(String::new());
                        on_change.emit(Account::SignedIn(me));
                        Ok(None)
                    }))
                })
            };
            html! {
                <form class="space-y-3" onsubmit={log_in}>
                    <h2 id="account-title" class="text-xl font-bold">{ "Sign in" }</h2>
                    if props.account == Account::Unavailable {
                        <p class="text-sm text-amber-700">{ "PokéGuess accounts couldn't be reached just now, but you can still try." }</p>
                    }
                    { field("username", "Username", "text", "username", &username, Some(first_field.clone())) }
                    { field("password", "Password", "password", "current-password", &password, None) }
                    <p class="text-sm text-gray-600">
                        { format!(
                            "New here? Pick a username of {USERNAME_MIN}–{USERNAME_MAX} letters, numbers, _ or -, and a password of at least {PASSWORD_MIN} characters, then create your account."
                        ) }
                    </p>
                    <div class="flex flex-wrap gap-2">
                        <button type="submit" class={PRIMARY} disabled={*busy}>{ "Log in" }</button>
                        <button type="button" class={SECONDARY} disabled={*busy} onclick={sign_up}>{ "Create account" }</button>
                    </div>
                </form>
            }
        }
    };

    html! {
        <dialog
            ref={dialog}
            class="m-auto w-[calc(100%-2rem)] max-w-sm rounded-2xl bg-white p-0 shadow-xl backdrop:bg-black/40"
            aria-labelledby="account-title"
            {oncancel}
            onclick={backdrop}
        >
            <div class="space-y-4 p-5">
                <div class="flex justify-end">
                    <button type="button" class="cursor-pointer text-sm text-gray-600 underline" onclick={close}>{ "Close" }</button>
                </div>
                { body }
                if let Some(message) = &*error {
                    <p class="text-sm font-semibold text-red-700" role="alert">{ message }</p>
                }
                if let Some(message) = &*notice {
                    <p class="text-sm text-green-700" role="status">{ message }</p>
                }
            </div>
        </dialog>
    }
}

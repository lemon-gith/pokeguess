//! Small wrappers around browser APIs.

use gloo::storage::{LocalStorage, SessionStorage, Storage};
use serde::Serialize;
use serde::de::DeserializeOwned;
use web_sys::HtmlImageElement;

/// A random index below `n` (or 0 when `n` is 0).
pub fn pick(n: usize) -> usize {
    (js_sys::Math::random() * n as f64) as usize
}

/// Milliseconds since the Unix epoch.
pub fn now() -> f64 {
    js_sys::Date::now()
}

/// A practically unique ID, e.g. for a finished run.
pub fn unique_id() -> String {
    let random = (js_sys::Math::random() * (1u64 << 52) as f64) as u64;
    format!("{:x}-{random:x}", now() as u64)
}

/// A date like "27 Oct 2026".
pub fn format_date(millis: f64) -> String {
    let date = js_sys::Date::new(&millis.into());
    let options = js_sys::Object::new();
    for (key, value) in [("day", "numeric"), ("month", "short"), ("year", "numeric")] {
        let _ = js_sys::Reflect::set(&options, &key.into(), &value.into());
    }
    date.to_locale_date_string("en-GB", &options).into()
}

/// Starts downloading an image so it shows instantly later.
pub fn prefetch_image(url: &str) {
    if let Ok(image) = HtmlImageElement::new() {
        image.set_src(url);
    }
}

/// Reads a value from local storage. Storage can be unavailable (e.g. in
/// private browsing) or hold an older format, so failures just give `None`.
pub fn load<T: DeserializeOwned>(key: &str) -> Option<T> {
    LocalStorage::get(key).ok()
}

/// Saves a value to local storage, ignoring failures for the same reasons.
pub fn save<T: Serialize>(key: &str, value: &T) {
    let _ = LocalStorage::set(key, value);
}

/// Removes a value from local storage.
pub fn forget(key: &str) {
    LocalStorage::delete(key);
}

/// Like [`load`], but for this tab only: survives a refresh, not a new tab.
pub fn load_session<T: DeserializeOwned>(key: &str) -> Option<T> {
    SessionStorage::get(key).ok()
}

/// Like [`save`], but for this tab only.
pub fn save_session<T: Serialize>(key: &str, value: &T) {
    let _ = SessionStorage::set(key, value);
}

pub fn forget_session(key: &str) {
    SessionStorage::delete(key);
}

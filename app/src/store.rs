//! Browser persistence, file export, sharing and small DOM helpers.

use chrono::NaiveDate;
use meds_core::Data;
use wasm_bindgen::{JsCast, JsValue};

const DATA_KEY: &str = "medstracker.data.v1";
const PREV_KEY: &str = "medstracker.data.v1.prev";
const THEME_KEY: &str = "medstracker.theme";
const BACKUP_KEY: &str = "medstracker.lastBackup";
/// Where an unreadable saved copy is parked so it's never overwritten.
const CORRUPT_KEY: &str = "medstracker.data.unreadable";

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub fn today() -> NaiveDate {
    let d = js_sys::Date::new_0();
    NaiveDate::from_ymd_opt(d.get_full_year() as i32, d.get_month() + 1, d.get_date())
        .expect("browser date is valid")
}

/// Loads saved data. The flag is true when a saved copy existed but couldn't be read;
/// that copy is parked under its own key first so later saves can't overwrite it.
pub fn load() -> (Data, bool) {
    let Some(s) = storage() else {
        return (Data::default(), false);
    };
    let mut unreadable = false;
    for key in [DATA_KEY, PREV_KEY] {
        if let Ok(Some(raw)) = s.get_item(key) {
            match Data::from_json(&raw) {
                Ok(d) => return (d, unreadable),
                Err(_) => {
                    if !unreadable && s.get_item(CORRUPT_KEY).ok().flatten().is_none() {
                        let _ = s.set_item(CORRUPT_KEY, &raw);
                    }
                    unreadable = true;
                }
            }
        }
    }
    (Data::default(), unreadable)
}

/// Saves `d`, keeping the previous copy as a fallback. False if the browser refused the write.
pub fn save(d: &Data) -> bool {
    let Some(s) = storage() else { return false };
    // Only rotate a copy that is itself readable, so the fallback is always a good one.
    if let Ok(Some(old)) = s.get_item(DATA_KEY) {
        if Data::from_json(&old).is_ok() {
            let _ = s.set_item(PREV_KEY, &old);
        }
    }
    s.set_item(DATA_KEY, &d.to_json()).is_ok()
}

pub fn last_backup() -> Option<f64> {
    storage()?.get_item(BACKUP_KEY).ok()??.parse().ok()
}

pub fn mark_backup() -> f64 {
    let t = now_ms();
    if let Some(s) = storage() {
        let _ = s.set_item(BACKUP_KEY, &t.to_string());
    }
    t
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Theme::System => "System",
            Theme::Light => "Light",
            Theme::Dark => "Dark",
        }
    }
}

pub fn load_theme() -> Theme {
    match storage()
        .and_then(|s| s.get_item(THEME_KEY).ok().flatten())
        .as_deref()
    {
        Some("light") => Theme::Light,
        Some("dark") => Theme::Dark,
        _ => Theme::System,
    }
}

pub fn save_theme(t: Theme) {
    if let Some(s) = storage() {
        let _ = match t {
            Theme::System => s.remove_item(THEME_KEY),
            Theme::Light => s.set_item(THEME_KEY, "light"),
            Theme::Dark => s.set_item(THEME_KEY, "dark"),
        };
    }
}

pub fn apply_theme(t: Theme) {
    let Some(el) = document().document_element() else {
        return;
    };
    let _ = match t {
        Theme::System => el.remove_attribute("data-theme"),
        Theme::Light => el.set_attribute("data-theme", "light"),
        Theme::Dark => el.set_attribute("data-theme", "dark"),
    };
}

pub fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

/// Asks the browser not to evict our storage under pressure.
pub fn request_persistence() {
    if let Some(w) = web_sys::window() {
        let _ = w.navigator().storage().persist();
    }
}

pub fn remove_boot_screen() {
    if let Some(el) = document().get_element_by_id("boot") {
        el.remove();
    }
}

/// Triggers a download of `contents` as a file.
pub fn download(filename: &str, mime: &str, contents: &str) {
    let parts = js_sys::Array::of1(&JsValue::from_str(contents));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime);
    let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };
    let doc = document();
    if let Ok(a) = doc.create_element("a") {
        let a: web_sys::HtmlAnchorElement = a.unchecked_into();
        a.set_href(&url);
        a.set_download(filename);
        if let Some(body) = doc.body() {
            let _ = body.append_child(&a);
            a.click();
            a.remove();
        }
    }
    // Revoking straight away can cancel the download in Safari.
    leptos::leptos_dom::helpers::set_timeout(
        move || {
            let _ = web_sys::Url::revoke_object_url(&url);
        },
        std::time::Duration::from_secs(30),
    );
}

pub fn confirm(msg: &str) -> bool {
    web_sys::window()
        .and_then(|w| w.confirm_with_message(msg).ok())
        .unwrap_or(false)
}

pub fn print() {
    if let Some(w) = web_sys::window() {
        let _ = w.print();
    }
}

/// Whether the browser has a native share sheet (phones, mostly).
pub fn can_share() -> bool {
    web_sys::window()
        .map(|w| js_sys::Reflect::has(&w.navigator(), &JsValue::from_str("share")).unwrap_or(false))
        .unwrap_or(false)
}

/// Opens the native share sheet with `text`. Resolves to false if it was dismissed or failed.
pub async fn share(text: &str) -> bool {
    let Some(w) = web_sys::window() else {
        return false;
    };
    let data = web_sys::ShareData::new();
    data.set_text(text);
    wasm_bindgen_futures::JsFuture::from(w.navigator().share_with_data(&data))
        .await
        .is_ok()
}

pub async fn copy(text: &str) -> bool {
    let Some(w) = web_sys::window() else {
        return false;
    };
    wasm_bindgen_futures::JsFuture::from(w.navigator().clipboard().write_text(text))
        .await
        .is_ok()
}

pub fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .map(|m| m.matches())
        .unwrap_or(false)
}

/// A short vibration as feedback when something is ticked off, where supported.
pub fn tick() {
    if let Some(w) = web_sys::window() {
        let _ = w.navigator().vibrate_with_duration(12);
    }
}

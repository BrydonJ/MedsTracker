//! Name, low-stock threshold, theme, and backup / restore.

use leptos::prelude::*;
use leptos::task::spawn_local;
use meds_core::Data;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::ui::{Icon, Segmented};
use crate::{fmt, store, use_app};

#[component]
pub fn SettingsView() -> impl IntoView {
    let app = use_app();
    let file_input = NodeRef::<leptos::html::Input>::new();

    let on_import = move |e: leptos::ev::Event| {
        let Some(input) = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        else {
            return;
        };
        let Some(file) = input.files().and_then(|f| f.get(0)) else {
            return;
        };
        input.set_value("");
        spawn_local(async move {
            let text = match JsFuture::from(file.text()).await {
                Ok(t) => t.as_string().unwrap_or_default(),
                Err(_) => {
                    app.notify("Couldn't read that file", None);
                    return;
                }
            };
            match Data::from_json(&text) {
                Ok(d) => {
                    let n = d.meds.len() as u32;
                    let empty = app.data.with_untracked(|d| d.meds.is_empty());
                    if empty
                        || store::confirm(&format!(
                            "Replace everything on this device with the backup ({})?",
                            fmt::plural(n, "medicine", "medicines")
                        ))
                    {
                        let before = app.data.get_untracked();
                        app.data.set(d);
                        app.notify(format!("Restored {}", fmt::plural(n, "medicine", "medicines")), Some(before));
                    }
                }
                Err(err) => app.notify(format!("Not a valid backup: {err}"), None),
            }
        });
    };

    let export = move |_| {
        let json = app.data.with_untracked(|d| d.to_json());
        let today = app.today.get_untracked();
        store::download(&format!("meds-backup-{today}.json"), "application/json", &json);
        app.last_backup.set(Some(store::mark_backup()));
    };

    let erase = move |_| {
        if store::confirm("Erase every medicine and all history from this device?") {
            app.mutate_with_undo("Everything erased", |d| *d = Data::default());
        }
    };

    let backup_text = move || match app.last_backup.get() {
        None => "Never backed up on this device".to_string(),
        Some(ms) => {
            let d = js_sys::Date::new(&ms.into());
            format!(
                "Last backup {}",
                chrono::NaiveDate::from_ymd_opt(d.get_full_year() as i32, d.get_month() + 1, d.get_date())
                    .map(fmt::date_long)
                    .unwrap_or_default()
            )
        }
    };

    view! {
        <section class="page-head">
            <div>
                <h1>"Settings"</h1>
            </div>
        </section>

        <div class="settings">
            <section class="card reveal">
                <label class="field">
                    <span>"Whose medicine is it?"</span>
                    <input
                        type="text"
                        placeholder="Name (shown on the printed sheet)"
                        autocapitalize="words"
                        prop:value=move || app.data.with(|d| d.settings.person.clone())
                        on:change=move |e| {
                            let v = event_target_value(&e).trim().to_string();
                            app.data.update(|d| d.settings.person = v);
                        }
                    />
                </label>
                <div class="toggle-row">
                    <div>
                        <strong>"Order when stock is under"</strong>
                        <span class="muted">"For medicines with a stock count"</span>
                    </div>
                    <Segmented
                        options=vec![(7u32, "1 wk"), (14, "2 wks"), (21, "3 wks"), (28, "4 wks")]
                        value=Signal::derive(move || app.data.with(|d| d.settings.low_days))
                        on_change=Callback::new(move |v| app.data.update(|d| d.settings.low_days = v))
                        label="Low stock threshold"
                    />
                </div>
                <div class="toggle-row">
                    <div>
                        <strong>"Appearance"</strong>
                    </div>
                    <Segmented
                        options=vec![
                            (store::Theme::System, "Auto"),
                            (store::Theme::Light, "Light"),
                            (store::Theme::Dark, "Dark"),
                        ]
                        value=app.theme
                        on_change=Callback::new(move |t| {
                            app.theme.set(t);
                            store::save_theme(t);
                        })
                        label="Theme"
                    />
                </div>
            </section>

            <section class="card reveal" style="--i:1">
                <div class="card-head">
                    <h3>
                        <Icon name="download"/>
                        "Backup"
                    </h3>
                </div>
                <p class="card-sub">
                    "Everything stays on this device; nothing is sent anywhere. Save a backup now and then, and restore it to move to a new phone."
                </p>
                <div class="action-row">
                    <button class="btn primary" on:click=export>
                        <Icon name="download"/>
                        "Save backup"
                    </button>
                    <button
                        class="btn"
                        on:click=move |_| {
                            if let Some(el) = file_input.get() {
                                el.click();
                            }
                        }
                    >
                        <Icon name="upload"/>
                        "Restore"
                    </button>
                    <input type="file" accept="application/json,.json" class="hidden" node_ref=file_input on:change=on_import/>
                </div>
                <p class="fine">{backup_text}</p>
            </section>

            <section class="card reveal" style="--i:2">
                <div class="card-head">
                    <h3>
                        <Icon name="print"/>
                        "Paper checklist"
                    </h3>
                </div>
                <p class="card-sub">"Print the medicine list as a packing checklist, morning and night."</p>
                <button class="btn" on:click=move |_| store::print()>
                    <Icon name="print"/>
                    "Print sheet"
                </button>
            </section>

            <section class="card reveal" style="--i:3">
                <button class="btn ghost danger" on:click=erase>
                    <Icon name="trash"/>
                    "Erase all data"
                </button>
            </section>
        </div>
    }
}

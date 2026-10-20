mod fmt;
mod meds;
mod order;
mod pack;
mod print;
mod settings;
mod store;
mod ui;

use std::time::Duration;

use chrono::NaiveDate;
use leptos::ev;
use leptos::leptos_dom::helpers::{set_timeout, window_event_listener};
use leptos::prelude::*;
use meds_core::{Data, Medicine};

use crate::ui::Icon;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Pack,
    Meds,
    Order,
    Settings,
}

impl Tab {
    const ALL: [Tab; 4] = [Tab::Pack, Tab::Meds, Tab::Order, Tab::Settings];

    fn label(self) -> &'static str {
        match self {
            Tab::Pack => "Pack",
            Tab::Meds => "Medicines",
            Tab::Order => "Order",
            Tab::Settings => "Settings",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Tab::Pack => "grid",
            Tab::Meds => "pill",
            Tab::Order => "bag",
            Tab::Settings => "settings",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: u32,
    pub message: String,
    /// Snapshot to restore when the user taps Undo.
    pub undo: Option<Data>,
}

/// Global app state, shared through context. All fields are cheap `Copy` handles.
#[derive(Clone, Copy)]
pub struct AppState {
    pub data: RwSignal<Data>,
    pub today: RwSignal<NaiveDate>,
    pub tab: RwSignal<Tab>,
    /// The medicine open in the editor sheet (a new one has an unused id).
    pub editing: RwSignal<Option<Medicine>>,
    pub toast: RwSignal<Option<Toast>>,
    pub theme: RwSignal<store::Theme>,
    pub last_backup: RwSignal<Option<f64>>,
}

impl AppState {
    pub fn notify(&self, message: impl Into<String>, undo: Option<Data>) {
        // Ids never repeat, so an old toast's timer can't dismiss a newer one.
        let id = TOAST_ID.with(|c| {
            c.set(c.get() + 1);
            c.get()
        });
        self.toast.set(Some(Toast {
            id,
            message: message.into(),
            undo,
        }));
        let toast = self.toast;
        set_timeout(
            move || {
                if toast.get_untracked().map(|t| t.id) == Some(id) {
                    toast.set(None);
                }
            },
            Duration::from_millis(5200),
        );
    }

    /// Applies a change and offers an undo for it. Undo restores the whole snapshot, so
    /// toast-less edits made in the next few seconds (ticking Packed) are undone too; that's
    /// acceptable for a 5 s window and keeps undo exact for the change it names.
    pub fn mutate_with_undo(&self, message: &str, f: impl FnOnce(&mut Data)) {
        let before = self.data.get_untracked();
        self.data.update(f);
        self.notify(message, Some(before));
    }

    pub fn add_medicine(&self) {
        let id = self.data.with_untracked(|d| d.next_id());
        self.editing.set(Some(Medicine::new(id)));
    }

    pub fn edit_medicine(&self, id: u32) {
        self.editing
            .set(self.data.with_untracked(|d| d.med(id).cloned()));
    }

    pub fn go(&self, tab: Tab) {
        self.tab.set(tab);
        ui::scroll_to_top();
    }
}

thread_local! {
    static TOAST_ID: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

pub fn use_app() -> AppState {
    expect_context::<AppState>()
}

#[component]
fn App() -> impl IntoView {
    let (loaded, unreadable) = store::load();
    let data = RwSignal::new(loaded);
    let today = RwSignal::new(store::today());
    let state = AppState {
        data,
        today,
        tab: RwSignal::new(Tab::Pack),
        editing: RwSignal::new(None),
        toast: RwSignal::new(None),
        theme: RwSignal::new(store::load_theme()),
        last_backup: RwSignal::new(store::last_backup()),
    };
    provide_context(state);

    // Persist every change. The first run only records the loaded data.
    let save_failed = StoredValue::new(false);
    Effect::new(move |prev: Option<()>| {
        data.with(|d| {
            if prev.is_some() {
                let ok = store::save(d);
                if !ok && !save_failed.get_value() {
                    state.notify("Couldn't save on this device. Check storage space, then save a backup.", None);
                }
                save_failed.set_value(!ok);
            }
        });
    });
    if unreadable {
        state.notify("Saved data couldn't be read; it's been kept aside. Restore a backup in Settings.", None);
    }
    Effect::new(move |_| store::apply_theme(state.theme.get()));

    // Keep "today" correct if the app is left open past midnight.
    let _ = window_event_listener(ev::visibilitychange, move |_| {
        let t = store::today();
        if t != today.get_untracked() {
            today.set(t);
        }
    });

    // Shortcuts: N = new medicine, 1–4 = switch tab.
    let _ = window_event_listener(ev::keydown, move |e| {
        if e.ctrl_key() || e.meta_key() || e.alt_key() || state.editing.get_untracked().is_some() {
            return;
        }
        if ui::typing_in_field(&e) {
            return;
        }
        match e.key().as_str() {
            "n" | "N" => {
                e.prevent_default();
                state.add_medicine();
            }
            k @ ("1" | "2" | "3" | "4") => {
                let i = k.parse::<usize>().unwrap() - 1;
                state.go(Tab::ALL[i]);
            }
            _ => {}
        }
    });

    store::request_persistence();
    store::remove_boot_screen();

    view! {
        <Shell/>
        <meds::Editor/>
        <ToastHost/>
        <print::PrintSheet/>
    }
}

#[component]
fn Shell() -> impl IntoView {
    let app = use_app();
    let tab_index = move || {
        Tab::ALL
            .iter()
            .position(|t| *t == app.tab.get())
            .unwrap_or(0)
    };
    let order_count = Memo::new(move |_| app.data.with(|d| meds_core::to_order(d).len()));
    let title = move || {
        app.data.with(|d| {
            let p = d.settings.person.trim();
            if p.is_empty() {
                "Meds".to_string()
            } else {
                format!("{p}'s meds")
            }
        })
    };

    view! {
        <div class="shell">
            <header class="topbar">
                <div class="brand">
                    <span class="brand-mark" aria-hidden="true">
                        <Icon name="pill"/>
                    </span>
                    <span class="brand-name">{title}</span>
                </div>
                <nav class="tabs" aria-label="Sections" style=move || format!("--tab:{}", tab_index())>
                    <span class="tab-pill" aria-hidden="true"></span>
                    {Tab::ALL
                        .into_iter()
                        .map(|t| {
                            view! {
                                <button
                                    class="tab"
                                    class:active=move || app.tab.get() == t
                                    aria-current=move || (app.tab.get() == t).then_some("page")
                                    on:click=move |_| app.go(t)
                                >
                                    <span class="tab-icon">
                                        <Icon name=t.icon()/>
                                        {(t == Tab::Order)
                                            .then(|| {
                                                view! {
                                                    <Show when=move || { order_count.get() > 0 }>
                                                        <span class="badge">{move || order_count.get()}</span>
                                                    </Show>
                                                }
                                            })}
                                    </span>
                                    <span class="tab-label">{t.label()}</span>
                                </button>
                            }
                        })
                        .collect_view()}
                </nav>
                <div class="topbar-actions">
                    <ThemeToggle/>
                </div>
            </header>
            <main class="content">
                {move || {
                    let view = match app.tab.get() {
                        Tab::Pack => view! { <pack::PackView/> }.into_any(),
                        Tab::Meds => view! { <meds::MedsView/> }.into_any(),
                        Tab::Order => view! { <order::OrderView/> }.into_any(),
                        Tab::Settings => view! { <settings::SettingsView/> }.into_any(),
                    };
                    view! { <div class="page">{view}</div> }
                }}
            </main>
        </div>
    }
}

#[component]
fn ThemeToggle() -> impl IntoView {
    let app = use_app();
    let next = move || match app.theme.get() {
        store::Theme::System => store::Theme::Light,
        store::Theme::Light => store::Theme::Dark,
        store::Theme::Dark => store::Theme::System,
    };
    view! {
        <button
            class="icon-btn"
            title=move || format!("Theme: {}", app.theme.get().label())
            aria-label=move || format!("Theme: {}. Click to change.", app.theme.get().label())
            on:click=move |_| {
                let t = next();
                app.theme.set(t);
                store::save_theme(t);
            }
        >
            {move || {
                let name = match app.theme.get() {
                    store::Theme::System => "auto",
                    store::Theme::Light => "sun",
                    store::Theme::Dark => "moon",
                };
                view! { <Icon name=name/> }
            }}
        </button>
    }
}

#[component]
fn ToastHost() -> impl IntoView {
    let app = use_app();
    view! {
        <div class="toast-host" aria-live="polite">
            {move || {
                app.toast
                    .get()
                    .map(|t| {
                        let undo = t.undo.clone();
                        view! {
                            <div class="toast">
                                <span>{t.message.clone()}</span>
                                {undo
                                    .map(|snap| {
                                        view! {
                                            <button
                                                class="toast-action"
                                                on:click=move |_| {
                                                    app.data.set(snap.clone());
                                                    app.toast.set(None);
                                                }
                                            >
                                                "Undo"
                                            </button>
                                        }
                                    })}
                            </div>
                        }
                    })
            }}
        </div>
    }
}

fn main() {
    console_error_panic_hook_lite();
    leptos::mount::mount_to_body(App);
}

/// Surfaces panics in the console without pulling in an extra crate.
fn console_error_panic_hook_lite() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&info.to_string().into());
        // The app can't continue after a panic; say so rather than freezing silently.
        let doc = store::document();
        if let (Some(body), Ok(el)) = (doc.body(), doc.create_element("div")) {
            el.set_class_name("crash");
            el.set_inner_html(
                "<strong>Something went wrong.</strong><span>Reload to continue.</span>\
                 <button onclick=\"location.reload()\">Reload</button>",
            );
            let _ = body.append_child(&el);
        }
    }));
}

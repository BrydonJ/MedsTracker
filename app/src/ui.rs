//! Shared widgets: icons, pill swatches, segmented controls, switches, steppers, sheets.

use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Pill colours offered in the editor: (name, fill).
pub const PILL_COLOURS: [(&str, &str); 10] = [
    ("White", "#f6f6f2"),
    ("Cream", "#efe2bf"),
    ("Yellow", "#f4cc45"),
    ("Orange", "#ef8a3d"),
    ("Pink", "#f2a3bf"),
    ("Red", "#d4463f"),
    ("Purple", "#9a6cd2"),
    ("Blue", "#4a8ae0"),
    ("Green", "#3fb07a"),
    ("Brown", "#9a6a48"),
];

pub fn pill_fill(colour: u8) -> &'static str {
    PILL_COLOURS
        .get(colour as usize)
        .map(|c| c.1)
        .unwrap_or(PILL_COLOURS[0].1)
}

fn icon_paths(name: &str) -> &'static str {
    match name {
        "pill" => {
            r#"<rect x="2.8" y="8.3" width="18.4" height="7.4" rx="3.7" transform="rotate(-38 12 12)"/><path d="m9.3 8.6 5.4 6.8"/>"#
        }
        "list" => {
            r#"<path d="M9 6h11M9 12h11M9 18h11"/><path d="M4 6h.01M4 12h.01M4 18h.01" stroke-width="2.6"/>"#
        }
        "bag" => {
            r#"<path d="M5 8h14l-1 12.5H6z"/><path d="M9 10.5V6.5a3 3 0 0 1 6 0v4"/>"#
        }
        "settings" => {
            r#"<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2.2"/><circle cx="9" cy="17" r="2.2"/>"#
        }
        "plus" => r#"<path d="M12 5v14M5 12h14"/>"#,
        "minus" => r#"<path d="M5 12h14"/>"#,
        "sun" => {
            r#"<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2M12 19.5v2M4.6 4.6l1.4 1.4M18 18l1.4 1.4M2.5 12h2M19.5 12h2M4.6 19.4 6 18M18 6l1.4-1.4"/>"#
        }
        "moon" => r#"<path d="M20 14.2A8.2 8.2 0 1 1 9.8 4a6.4 6.4 0 0 0 10.2 10.2z"/>"#,
        "auto" => {
            r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 3.5v17a8.5 8.5 0 0 0 0-17z" fill="currentColor" stroke="none"/>"#
        }
        "download" => r#"<path d="M12 4v11M7.5 10.5 12 15l4.5-4.5M5 20h14"/>"#,
        "upload" => r#"<path d="M12 15.5V4.5M7.5 9 12 4.5 16.5 9M5 20h14"/>"#,
        "alert" => r#"<path d="M12 3.5 2.5 20h19z"/><path d="M12 10v4.5M12 17.2h.01"/>"#,
        "clock" => r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>"#,
        "x" => r#"<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>"#,
        "trash" => r#"<path d="M4.5 7h15M9.5 7V4.5h5V7M6.5 7l1 13h9l1-13"/>"#,
        "check" => r#"<path d="M5 12.5 9.5 17 19 7.5"/>"#,
        "chev-left" => r#"<path d="M14.5 6 8.5 12l6 6"/>"#,
        "chev-right" => r#"<path d="M9.5 6l6 6-6 6"/>"#,
        "print" => {
            r#"<path d="M7 9V3.5h10V9"/><rect x="3.5" y="9" width="17" height="8" rx="2"/><path d="M7 14h10v6.5H7z"/>"#
        }
        "share" => {
            r#"<path d="M12 15V3.5M8 7.5l4-4 4 4"/><path d="M6 11H5v9.5h14V11h-1"/>"#
        }
        "copy" => {
            r#"<rect x="8.5" y="8.5" width="12" height="12" rx="2.5"/><path d="M15.5 8.5V5.5a2 2 0 0 0-2-2h-8a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h3"/>"#
        }
        "box" => {
            r#"<path d="M3.5 7.5 12 3.5l8.5 4v9L12 20.5l-8.5-4z"/><path d="M3.5 7.5 12 11.5l8.5-4M12 11.5v9"/>"#
        }
        "grid" => {
            r#"<rect x="3.5" y="5.5" width="17" height="13" rx="2.5"/><path d="M3.5 12h17M9.2 5.5v13M14.8 5.5v13"/>"#
        }
        "scissors" => {
            r#"<circle cx="6.5" cy="7" r="2.5"/><circle cx="6.5" cy="17" r="2.5"/><path d="M8.6 8.4 19.5 17M8.6 15.6 19.5 7"/>"#
        }
        "pause" => r#"<path d="M9 6v12M15 6v12"/>"#,
        "history" => {
            r#"<path d="M4 12a8 8 0 1 0 2.4-5.7"/><path d="M4 4.5v3.8h3.8"/><path d="M12 8v4.5l3 1.8"/>"#
        }
        "flag" => r#"<path d="M5.5 21V4M5.5 4.5h11l-2 4 2 4h-11"/>"#,
        _ => "",
    }
}

#[component]
pub fn Icon(#[prop(into)] name: String) -> impl IntoView {
    view! {
        <svg
            class="icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            inner_html=icon_paths(&name)
        ></svg>
    }
}

/// A small capsule in the medicine's colour.
#[component]
pub fn PillDot(#[prop(into)] colour: Signal<u8>) -> impl IntoView {
    view! { <span class="pill-dot" aria-hidden="true" style=move || format!("--pill:{}", pill_fill(colour.get()))></span> }
}

#[component]
pub fn Segmented<T>(
    options: Vec<(T, &'static str)>,
    #[prop(into)] value: Signal<T>,
    #[prop(into)] on_change: Callback<T>,
    #[prop(optional)] label: &'static str,
) -> impl IntoView
where
    T: Copy + PartialEq + Send + Sync + 'static,
{
    let n = options.len();
    let opts = options.clone();
    let index = move || {
        let v = value.get();
        opts.iter().position(|(o, _)| *o == v).unwrap_or(0)
    };
    view! {
        <div class="segmented" role="radiogroup" aria-label=label style=move || format!("--n:{n};--i:{}", index())>
            <span class="seg-pill" aria-hidden="true"></span>
            {options
                .into_iter()
                .map(|(o, text)| {
                    view! {
                        <button
                            type="button"
                            role="radio"
                            class="seg"
                            class:active=move || value.get() == o
                            aria-checked=move || (value.get() == o).to_string()
                            on:click=move |_| on_change.run(o)
                        >
                            {text}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
pub fn Switch(
    #[prop(into)] checked: Signal<bool>,
    #[prop(into)] on_change: Callback<bool>,
    #[prop(optional)] label: &'static str,
) -> impl IntoView {
    view! {
        <button
            type="button"
            role="switch"
            class="switch"
            class:on=move || checked.get()
            aria-checked=move || checked.get().to_string()
            aria-label=label
            on:click=move |_| on_change.run(!checked.get_untracked())
        >
            <span class="knob"></span>
        </button>
    }
}

/// − value + in half-tablet steps.
#[component]
pub fn Stepper(value: RwSignal<u32>, #[prop(optional)] label: &'static str) -> impl IntoView {
    view! {
        <div class="stepper" role="group" aria-label=label>
            <button
                type="button"
                class="step-btn"
                aria-label=format!("Less {label}")
                disabled=move || value.get() == 0
                on:click=move |_| value.update(|v| *v = v.saturating_sub(1))
            >
                <Icon name="minus"/>
            </button>
            <span class="step-val" class:zero=move || value.get() == 0>
                {move || match value.get() {
                    0 => "—".to_string(),
                    h => crate::fmt::halves(h),
                }}
            </span>
            <button
                type="button"
                class="step-btn"
                aria-label=format!("More {label}")
                disabled={move || value.get() >= 40}
                on:click=move |_| value.update(|v| *v = (*v + 1).min(40))
            >
                <Icon name="plus"/>
            </button>
        </div>
    }
}

/// A bottom sheet (a centred dialog on wide screens). Escape or the backdrop closes it.
#[component]
pub fn Sheet(
    #[prop(into)] title: Signal<String>,
    #[prop(into)] on_close: Callback<()>,
    children: Children,
) -> impl IntoView {
    let handle = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" {
            on_close.run(());
        }
    });
    on_cleanup(move || handle.remove());
    view! {
        <div class="sheet-root">
            <div class="sheet-backdrop" on:click=move |_| on_close.run(())></div>
            <div class="sheet" role="dialog" aria-modal="true" aria-label=move || title.get()>
                <div class="sheet-grip" aria-hidden="true"></div>
                <header class="sheet-head">
                    <h2>{move || title.get()}</h2>
                    <button class="icon-btn" aria-label="Close" on:click=move |_| on_close.run(())>
                        <Icon name="x"/>
                    </button>
                </header>
                <div class="sheet-body">{children()}</div>
            </div>
        </div>
    }
}

pub fn typing_in_field(e: &web_sys::KeyboardEvent) -> bool {
    e.target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .map(|el| matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT"))
        .unwrap_or(false)
}

pub fn scroll_to_top() {
    if let Some(w) = web_sys::window() {
        w.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

/// Parses a whole number typed by a person; blank means "not set".
pub fn parse_count(s: &str) -> Option<u32> {
    s.trim().parse::<u32>().ok()
}

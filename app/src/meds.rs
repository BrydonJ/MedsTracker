//! The medicine list and the add/edit sheet.

use leptos::prelude::*;
use meds_core::{days_left, halves, order_reason, Medicine, OrderState, EVERY_DAY};

use crate::ui::{parse_count, pill_fill, Icon, PillDot, Sheet, Stepper, Switch, PILL_COLOURS};
use crate::{fmt, store, use_app};

#[component]
pub fn MedsView() -> impl IntoView {
    let app = use_app();
    let ids = move |paused: bool| {
        app.data.with(|d| {
            d.sorted_meds()
                .into_iter()
                .filter(|m| m.paused == paused)
                .map(|m| m.id)
                .collect::<Vec<_>>()
        })
    };
    let active = Memo::new(move |_| ids(false));
    let paused = Memo::new(move |_| ids(true));

    view! {
        <section class="page-head">
            <div>
                <h1>"Medicines"</h1>
                <p class="sub">
                    {move || {
                        let n = active.with(|a| a.len()) as u32;
                        format!("{} taken · tap one to change it", fmt::plural(n, "medicine", "medicines"))
                    }}
                </p>
            </div>
            <div class="head-actions">
                <button class="btn" on:click=move |_| store::print() title="Print a packing checklist">
                    <Icon name="print"/>
                    <span class="hide-sm">"Print sheet"</span>
                </button>
                <button class="btn primary" on:click=move |_| app.add_medicine() title="Add medicine (N)">
                    <Icon name="plus"/>
                    "Add"
                </button>
            </div>
        </section>

        <Show
            when=move || !active.with(|a| a.is_empty()) || !paused.with(|p| p.is_empty())
            fallback=move || {
                view! {
                    <div class="card empty reveal">
                        <Icon name="pill"/>
                        <p>"No medicines yet."</p>
                        <button class="btn primary" on:click=move |_| app.add_medicine()>
                            <Icon name="plus"/>
                            "Add medicine"
                        </button>
                    </div>
                }
            }
        >
            <section class="card list reveal">
                <For each=move || active.get() key=|id| *id let:id>
                    <MedRow id=id/>
                </For>
            </section>
            <Show when=move || !paused.with(|p| p.is_empty())>
                <h3 class="section-label">"Paused"</h3>
                <section class="card list paused reveal" style="--i:1">
                    <For each=move || paused.get() key=|id| *id let:id>
                        <MedRow id=id/>
                    </For>
                </section>
            </Show>
        </Show>
    }
}

#[component]
fn MedRow(id: u32) -> impl IntoView {
    let app = use_app();
    let med = Memo::new(move |_| app.data.with(|d| d.med(id).cloned()));
    let low_days = move || app.data.with(|d| d.settings.low_days);
    view! {
        {move || {
            med.get()
                .map(|m| {
                    let days = (!m.is_daily()).then(|| view! { <span class="tag">{fmt::days(m.days)}</span> });
                    let stock = m
                        .stock
                        .map(|s| {
                            let left = days_left(&m);
                            let low = order_reason(&m, low_days()).is_some();
                            view! {
                                <span class="stock" class:low=low>
                                    <strong>{s}</strong>
                                    <span>{left.map(fmt::duration_days).unwrap_or_else(|| "in box".into())}</span>
                                </span>
                            }
                        });
                    let ordered = (m.order == OrderState::Ordered).then(|| view! { <span class="tag accent">"Ordered"</span> });
                    view! {
                        <button class="med-row" on:click=move |_| app.edit_medicine(id)>
                            <PillDot colour=m.colour/>
                            <span class="med-main">
                                <span class="med-name">
                                    {m.name.clone()}
                                    <span class="med-strength">{m.strength.clone()}</span>
                                </span>
                                <span class="med-sub">
                                    {fmt::dose_summary(&m)}
                                    {days}
                                    {ordered}
                                </span>
                            </span>
                            {stock}
                            <Icon name="chev-right"/>
                        </button>
                    }
                })
        }}
    }
}

/// Form state for the editor; separate signals keep typing from re-rendering the whole form.
#[derive(Clone, Copy)]
struct Draft {
    id: u32,
    is_new: bool,
    name: RwSignal<String>,
    strength: RwSignal<String>,
    morning: RwSignal<u32>,
    night: RwSignal<u32>,
    days: RwSignal<u8>,
    colour: RwSignal<u8>,
    note: RwSignal<String>,
    stock: RwSignal<String>,
    box_size: RwSignal<String>,
    spare: RwSignal<u32>,
    paused: RwSignal<bool>,
    order: RwSignal<OrderState>,
}

impl Draft {
    fn from(m: &Medicine, is_new: bool) -> Self {
        let num = |v: Option<u32>| v.map(|n| n.to_string()).unwrap_or_default();
        Draft {
            id: m.id,
            is_new,
            name: RwSignal::new(m.name.clone()),
            strength: RwSignal::new(m.strength.clone()),
            morning: RwSignal::new(halves(m.morning)),
            night: RwSignal::new(halves(m.night)),
            days: RwSignal::new(m.days & EVERY_DAY),
            colour: RwSignal::new(m.colour),
            note: RwSignal::new(m.note.clone()),
            stock: RwSignal::new(num(m.stock)),
            box_size: RwSignal::new(num(m.box_size)),
            spare: RwSignal::new(m.spare_halves),
            paused: RwSignal::new(m.paused),
            order: RwSignal::new(m.order),
        }
    }

    fn apply(&self, base: &Medicine) -> Medicine {
        let mut m = base.clone();
        m.name = self.name.get_untracked().trim().to_string();
        m.strength = self.strength.get_untracked().trim().to_string();
        m.morning = self.morning.get_untracked() as f64 / 2.0;
        m.night = self.night.get_untracked() as f64 / 2.0;
        m.days = self.days.get_untracked();
        m.colour = self.colour.get_untracked();
        m.note = self.note.get_untracked().trim().to_string();
        m.stock = parse_count(&self.stock.get_untracked());
        m.box_size = parse_count(&self.box_size.get_untracked()).filter(|n| *n > 0);
        m.spare_halves = self.spare.get_untracked();
        m.paused = self.paused.get_untracked();
        let order = self.order.get_untracked();
        if order != m.order {
            m.ordered_on = (order == OrderState::Ordered).then(crate::store::today);
        }
        m.order = order;
        m
    }
}

#[component]
pub fn Editor() -> impl IntoView {
    let app = use_app();
    view! {
        {move || {
            app.editing
                .get()
                .map(|m| {
                    let is_new = app.data.with_untracked(|d| d.med(m.id).is_none());
                    view! { <EditorSheet base=m is_new=is_new/> }
                })
        }}
    }
}

#[component]
fn EditorSheet(base: Medicine, is_new: bool) -> impl IntoView {
    let app = use_app();
    let draft = Draft::from(&base, is_new);
    let base = StoredValue::new(base);
    let error = RwSignal::new(None::<&'static str>);
    let close = Callback::new(move |_| app.editing.set(None));

    let save = move || {
        if draft.name.get_untracked().trim().is_empty() {
            error.set(Some("Give it a name"));
            return;
        }
        if draft.morning.get_untracked() + draft.night.get_untracked() == 0 && !draft.paused.get_untracked() {
            error.set(Some("Set a morning or night dose, or pause it"));
            return;
        }
        if draft.days.get_untracked() == 0 {
            error.set(Some("Pick at least one day"));
            return;
        }
        // A typo shouldn't quietly switch stock tracking off.
        let bad = |s: String| !s.trim().is_empty() && parse_count(&s).is_none();
        if bad(draft.stock.get_untracked()) || bad(draft.box_size.get_untracked()) {
            error.set(Some("Stock and box size must be whole numbers"));
            return;
        }
        let med = draft.apply(&base.get_value());
        let name = med.display_name();
        app.data.update(|d| d.upsert(med));
        app.editing.set(None);
        app.notify(if draft.is_new { format!("Added {name}") } else { format!("Saved {name}") }, None);
    };
    let delete = move |_| {
        let name = base.with_value(|m| m.display_name());
        if store::confirm(&format!("Delete {name}? Pausing keeps it for later instead.")) {
            app.editing.set(None);
            app.mutate_with_undo(&format!("Deleted {name}"), |d| d.remove(draft.id));
        }
    };

    let title = Signal::derive(move || if draft.is_new { "Add medicine".to_string() } else { "Edit medicine".to_string() });
    let weekly_total = move || {
        let per_day = draft.morning.get() + draft.night.get();
        fmt::halves(per_day * draft.days.get().count_ones())
    };

    view! {
        <Sheet title=title on_close=close>
            <form
                class="form"
                on:submit=move |e| {
                    e.prevent_default();
                    save();
                }
            >
                <div class="form-row two">
                    <label class="field grow">
                        <span>"Name"</span>
                        <input
                            type="text"
                            placeholder="e.g. Euthyrox"
                            autocomplete="off"
                            autocapitalize="words"
                            prop:value=move || draft.name.get()
                            on:input=move |e| {
                                error.set(None);
                                draft.name.set(event_target_value(&e));
                            }
                            autofocus=draft.is_new
                        />
                    </label>
                    <label class="field">
                        <span>"Strength"</span>
                        <input
                            type="text"
                            placeholder="100ug"
                            autocomplete="off"
                            prop:value=move || draft.strength.get()
                            on:input=move |e| draft.strength.set(event_target_value(&e))
                        />
                    </label>
                </div>

                <div class="field">
                    <span>"Tablets per dose"</span>
                    <div class="dose-grid">
                        <div class="dose-box">
                            <span class="dose-label">
                                <Icon name="sun"/>
                                "Morning"
                            </span>
                            <Stepper value=draft.morning label="morning"/>
                        </div>
                        <div class="dose-box night">
                            <span class="dose-label">
                                <Icon name="moon"/>
                                "Night"
                            </span>
                            <Stepper value=draft.night label="night"/>
                        </div>
                    </div>
                </div>

                <div class="field">
                    <span class="field-head">
                        "Days"
                        <span class="muted">{move || fmt::days(draft.days.get())}</span>
                    </span>
                    <div class="day-chips">
                        {(0..7)
                            .map(|d| {
                                let bit = 1u8 << d;
                                view! {
                                    <button
                                        type="button"
                                        class="day-chip"
                                        class:on=move || draft.days.get() & bit != 0
                                        aria-pressed=move || (draft.days.get() & bit != 0).to_string()
                                        aria-label=fmt::DAY_NAMES[d]
                                        on:click=move |_| draft.days.update(|m| *m ^= bit)
                                    >
                                        {fmt::DAY_LETTERS[d]}
                                    </button>
                                }
                            })
                            .collect_view()}
                        <button
                            type="button"
                            class="btn small ghost"
                            class:hidden=move || draft.days.get() == EVERY_DAY
                            on:click=move |_| draft.days.set(EVERY_DAY)
                        >
                            "Every day"
                        </button>
                    </div>
                    <p class="hint">{move || format!("{} tablets a week", weekly_total())}</p>
                </div>

                <div class="field">
                    <span>"Colour"</span>
                    <div class="colour-chips" role="radiogroup" aria-label="Pill colour">
                        {PILL_COLOURS
                            .iter()
                            .enumerate()
                            .map(|(i, (name, _))| {
                                let i = i as u8;
                                view! {
                                    <button
                                        type="button"
                                        role="radio"
                                        class="colour-chip"
                                        class:on=move || draft.colour.get() == i
                                        aria-checked=move || (draft.colour.get() == i).to_string()
                                        aria-label=*name
                                        title=*name
                                        style=format!("--pill:{}", pill_fill(i))
                                        on:click=move |_| draft.colour.set(i)
                                    ></button>
                                }
                            })
                            .collect_view()}
                    </div>
                </div>

                <fieldset class="group">
                    <legend>
                        <Icon name="box"/>
                        "Stock (optional)"
                    </legend>
                    <p class="hint">"Track it and the app subtracts each week's tablets and tells you when to order."</p>
                    <div class="form-row two">
                        <label class="field">
                            <span>"Tablets in the box now"</span>
                            <input
                                type="number"
                                inputmode="numeric"
                                min="0"
                                placeholder="Not tracked"
                                prop:value=move || draft.stock.get()
                                on:input=move |e| draft.stock.set(event_target_value(&e))
                            />
                        </label>
                        <label class="field">
                            <span>"Tablets per box"</span>
                            <input
                                type="number"
                                inputmode="numeric"
                                min="1"
                                placeholder="e.g. 30"
                                prop:value=move || draft.box_size.get()
                                on:input=move |e| draft.box_size.set(event_target_value(&e))
                            />
                        </label>
                    </div>
                    <Show when=move || { draft.spare.get() > 0 }>
                        <p class="hint">
                            {move || format!("{} left over from cutting. ", fmt::plural(draft.spare.get(), "spare half", "spare halves"))}
                            <button type="button" class="link" on:click=move |_| draft.spare.set(0)>
                                "None left"
                            </button>
                        </p>
                    </Show>
                </fieldset>

                <label class="field">
                    <span>"Note"</span>
                    <input
                        type="text"
                        placeholder="e.g. take with food"
                        prop:value=move || draft.note.get()
                        on:input=move |e| draft.note.set(event_target_value(&e))
                    />
                </label>

                <div class="toggle-row">
                    <div>
                        <strong>"Needs ordering"</strong>
                        <span class="muted">"Puts it on the order list"</span>
                    </div>
                    <Switch
                        checked=Signal::derive(move || draft.order.get() != OrderState::None)
                        on_change=Callback::new(move |on: bool| {
                            draft.order.set(if on { OrderState::Needed } else { OrderState::None })
                        })
                        label="Needs ordering"
                    />
                </div>
                <div class="toggle-row">
                    <div>
                        <strong>"Paused"</strong>
                        <span class="muted">"Kept on the list but not packed"</span>
                    </div>
                    <Switch
                        checked=draft.paused
                        on_change=Callback::new(move |v| draft.paused.set(v))
                        label="Paused"
                    />
                </div>

                {move || error.get().map(|e| view! { <p class="form-error" role="alert">{e}</p> })}

                <div class="sheet-actions">
                    {(!draft.is_new)
                        .then(|| {
                            view! {
                                <button type="button" class="btn ghost danger" on:click=delete>
                                    <Icon name="trash"/>
                                    "Delete"
                                </button>
                            }
                        })}
                    <span class="spacer"></span>
                    <button type="button" class="btn" on:click=move |_| close.run(())>
                        "Cancel"
                    </button>
                    <button type="submit" class="btn primary">
                        <Icon name="check"/>
                        {if draft.is_new { "Add" } else { "Save" }}
                    </button>
                </div>
            </form>
        </Sheet>
    }
}

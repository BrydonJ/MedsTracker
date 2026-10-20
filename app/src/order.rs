//! What to order from the pharmacy, what's on its way, and receiving it into stock.

use leptos::prelude::*;
use leptos::task::spawn_local;
use meds_core::{
    awaiting, days_left, mark_ordered, order_reason, order_text, receive, to_order, OrderReason, OrderState,
};

use crate::ui::{parse_count, Icon, PillDot, Sheet};
use crate::{fmt, store, use_app, Tab};

#[component]
pub fn OrderView() -> impl IntoView {
    let app = use_app();
    let needed = Memo::new(move |_| app.data.with(|d| to_order(d).iter().map(|m| m.id).collect::<Vec<_>>()));
    let waiting = Memo::new(move |_| app.data.with(|d| awaiting(d).iter().map(|m| m.id).collect::<Vec<_>>()));
    let can_share = store::can_share();

    let share = move |_| {
        let text = app.data.with_untracked(order_text);
        spawn_local(async move {
            if can_share {
                store::share(&text).await;
            } else if store::copy(&text).await {
                app.notify("Order list copied", None);
            } else {
                app.notify("Couldn't copy the list", None);
            }
        });
    };
    let all_ordered = move |_| {
        let today = app.today.get_untracked();
        let ids = needed.get_untracked();
        app.mutate_with_undo("Marked as ordered", |d| {
            for id in ids {
                if let Some(m) = d.med_mut(id) {
                    mark_ordered(m, today);
                }
            }
        });
    };

    view! {
        <section class="page-head">
            <div>
                <h1>"Order"</h1>
                <p class="sub">
                    {move || {
                        let low = app.data.with(|d| d.settings.low_days);
                        format!("Flagged while packing, or with stock for under {}.", fmt::plural(low, "day", "days"))
                    }}
                </p>
            </div>
            <Show when=move || app.data.with(|d| d.meds.iter().any(|m| m.stock.is_some()))>
                <div class="head-actions">
                    <button class="btn primary" on:click=move |_| app.topup.set(true)>
                        <Icon name="box"/>
                        "Add a delivery"
                    </button>
                </div>
            </Show>
        </section>

        <Show
            when=move || !needed.with(|n| n.is_empty())
            fallback=move || {
                view! {
                    <div class="card empty reveal">
                        <Icon name="check"/>
                        <p>"Nothing to order right now."</p>
                        <p class="muted small">"Tap “Running low?” on a medicine while packing to add it here."</p>
                    </div>
                }
            }
        >
            <section class="card list reveal">
                <For each=move || needed.get() key=|id| *id let:id>
                    <NeedRow id=id/>
                </For>
            </section>
            <div class="action-row reveal" style="--i:1">
                <button class="btn primary" on:click=share>
                    <Icon name=if can_share { "share" } else { "copy" }/>
                    {if can_share { "Send list" } else { "Copy list" }}
                </button>
                <button class="btn" on:click=all_ordered>
                    <Icon name="check"/>
                    "Mark all ordered"
                </button>
            </div>
        </Show>

        <Show when=move || !waiting.with(|w| w.is_empty())>
            <h3 class="section-label">"On the way"</h3>
            <section class="card list reveal" style="--i:2">
                <For each=move || waiting.get() key=|id| *id let:id>
                    <WaitingRow id=id/>
                </For>
            </section>
        </Show>

        <Show when={move || app.data.with(|d| d.meds.iter().all(|m| m.stock.is_none())) && !app.data.with(|d| d.meds.is_empty())}>
            <p class="tip reveal" style="--i:3">
                <Icon name="box"/>
                <span>
                    "Tip: add a stock count to a medicine and it shows up here by itself before it runs out. "
                    <button class="link" on:click=move |_| app.go(Tab::Meds)>
                        "Medicines"
                    </button>
                </span>
            </p>
        </Show>
    }
}

#[component]
fn NeedRow(id: u32) -> impl IntoView {
    let app = use_app();
    let med = Memo::new(move |_| app.data.with(|d| d.med(id).cloned()));
    let low_days = move || app.data.with(|d| d.settings.low_days);
    view! {
        {move || {
            med.get()
                .map(|m| {
                    let reason = match order_reason(&m, low_days()) {
                        Some(OrderReason::LowStock) => {
                            format!("{} left", days_left(&m).map(fmt::duration_days).unwrap_or_default())
                        }
                        _ => "Flagged".to_string(),
                    };
                    let name = m.display_name();
                    view! {
                        <div class="med-row static">
                            <PillDot colour=m.colour/>
                            <span class="med-main">
                                <span class="med-name">{m.name.clone()} <span class="med-strength">{m.strength.clone()}</span></span>
                                <span class="med-sub">{reason}</span>
                            </span>
                            <button
                                class="btn small"
                                on:click=move |_| {
                                    let today = app.today.get_untracked();
                                    app.mutate_with_undo(&format!("{name} ordered"), |d| {
                                        if let Some(m) = d.med_mut(id) {
                                            mark_ordered(m, today);
                                        }
                                    });
                                }
                            >
                                "Ordered"
                            </button>
                        </div>
                    }
                })
        }}
    }
}

#[component]
fn WaitingRow(id: u32) -> impl IntoView {
    let app = use_app();
    let med = Memo::new(move |_| app.data.with(|d| d.med(id).cloned()));
    let open = RwSignal::new(false);
    let count = RwSignal::new(String::new());

    view! {
        {move || {
            med.get()
                .map(|m| {
                    let since = m
                        .ordered_on
                        .map(|d| format!("Ordered {}", fmt::ago(d, app.today.get())))
                        .unwrap_or_else(|| "Ordered".into());
                    let name = m.display_name();
                    let tracked = m.stock.is_some();
                    let box_size = m.box_size;
                    let arrive = move |n: u32| {
                        let name = name.clone();
                        app.mutate_with_undo(&format!("{name} received"), |d| {
                            if let Some(m) = d.med_mut(id) {
                                receive(m, n);
                            }
                        });
                    };
                    let arrive2 = arrive.clone();
                    view! {
                        <div class="med-row static wrap">
                            <PillDot colour=m.colour/>
                            <span class="med-main">
                                <span class="med-name">{m.name.clone()} <span class="med-strength">{m.strength.clone()}</span></span>
                                <span class="med-sub">{since}</span>
                            </span>
                            <button
                                class="btn small"
                                on:click=move |_| {
                                    if tracked {
                                        count.set(box_size.map(|n| n.to_string()).unwrap_or_default());
                                        open.set(true);
                                    } else {
                                        arrive(0);
                                    }
                                }
                            >
                                <Icon name="box"/>
                                "Arrived"
                            </button>
                            <Show when=move || open.get()>
                                <form
                                    class="receive"
                                    on:submit={
                                        let arrive2 = arrive2.clone();
                                        move |e| {
                                            e.prevent_default();
                                            open.set(false);
                                            arrive2(parse_count(&count.get_untracked()).unwrap_or(0));
                                        }
                                    }
                                >
                                    <label class="field inline">
                                        <span>"Tablets received"</span>
                                        <input
                                            type="number"
                                            inputmode="numeric"
                                            min="0"
                                            prop:value=move || count.get()
                                            on:input=move |e| count.set(event_target_value(&e))
                                        />
                                    </label>
                                    <button type="submit" class="btn small primary">
                                        "Add to stock"
                                    </button>
                                </form>
                            </Show>
                        </div>
                    }
                })
        }}
    }
}

/// The "add a delivery" sheet, open while `AppState::topup` is set.
#[component]
pub fn TopUp() -> impl IntoView {
    let app = use_app();
    move || app.topup.get().then(|| view! { <TopUpSheet/> })
}

/// Count what came in for each medicine and add it all to stock in one go.
#[component]
fn TopUpSheet() -> impl IntoView {
    let app = use_app();
    let close = Callback::new(move |_| app.topup.set(false));
    // Only medicines with a stock count: adding to an untracked one would ignore what's
    // already in its box. Every row starts empty, so nothing is added that didn't come.
    let rows: Vec<(u32, RwSignal<String>)> = app.data.with_untracked(|d| {
        d.sorted_meds()
            .into_iter()
            .filter(|m| m.stock.is_some())
            .map(|m| (m.id, RwSignal::new(String::new())))
            .collect()
    });
    let untracked = app
        .data
        .with_untracked(|d| d.meds.iter().filter(|m| m.stock.is_none() && !m.paused).count());
    let rows = StoredValue::new(rows);
    let total = move || {
        rows.with_value(|r| {
            r.iter()
                .filter_map(|(_, s)| parse_count(&s.get()))
                .fold(0u32, u32::saturating_add)
        })
    };
    let bad = move || {
        rows.with_value(|r| {
            r.iter().any(|(_, s)| {
                let t = s.get();
                !t.trim().is_empty() && parse_count(&t).is_none()
            })
        })
    };
    let save = move || {
        let list: Vec<(u32, u32)> = rows.with_value(|r| {
            r.iter()
                .filter_map(|(id, s)| parse_count(&s.get_untracked()).filter(|n| *n > 0).map(|n| (*id, n)))
                .collect()
        });
        if list.is_empty() {
            return;
        }
        let message = match list.as_slice() {
            [(id, n)] => app
                .data
                .with_untracked(|d| d.med(*id).map(|m| format!("Added {n} to {}", m.display_name())))
                .unwrap_or_default(),
            l => format!("Added {} to stock", fmt::plural(l.len() as u32, "delivery", "deliveries")),
        };
        app.topup.set(false);
        app.mutate_with_undo(&message, |d| {
            for (id, n) in list {
                if let Some(m) = d.med_mut(id) {
                    receive(m, n);
                }
            }
        });
    };

    view! {
        <Sheet title="Add a delivery" on_close=close>
            <form
                class="form"
                on:submit=move |e| {
                    e.prevent_default();
                    save();
                }
            >
                <p class="hint">"Type or tap how many tablets came for each. They go on top of what's in the box."</p>
                {move || {
                    rows.with_value(|r| r.is_empty())
                        .then(|| {
                            view! {
                                <p class="muted">
                                    "No medicine has a stock count yet. Add one under Medicines to track it."
                                </p>
                            }
                        })
                }}
                <div class="topup-list">
                    {rows
                        .with_value(|r| r.clone())
                        .into_iter()
                        .map(|(id, count)| view! { <TopUpRow id=id count=count/> })
                        .collect_view()}
                </div>
                {(untracked > 0)
                    .then(|| {
                        view! {
                            <p class="hint">
                                {format!(
                                    "{} without a stock count {} not listed.",
                                    fmt::plural(untracked as u32, "medicine", "medicines"),
                                    if untracked == 1 { "is" } else { "are" },
                                )}
                            </p>
                        }
                    })}
                {move || bad().then(|| view! { <p class="form-error" role="alert">"Counts must be whole numbers"</p> })}
                <div class="sheet-actions">
                    <span class="spacer"></span>
                    <button type="button" class="btn" on:click=move |_| close.run(())>
                        "Cancel"
                    </button>
                    <button type="submit" class="btn primary" disabled=move || total() == 0 || bad()>
                        <Icon name="check"/>
                        {move || match total() {
                            0 => "Add to stock".to_string(),
                            n => format!("Add {}", fmt::plural(n, "tablet", "tablets")),
                        }}
                    </button>
                </div>
            </form>
        </Sheet>
    }
}

#[component]
fn TopUpRow(id: u32, count: RwSignal<String>) -> impl IntoView {
    let app = use_app();
    let Some(m) = app.data.with_untracked(|d| d.med(id).cloned()) else {
        return ().into_any();
    };
    let stock = m.stock.unwrap_or(0);
    let n = Signal::derive(move || parse_count(&count.get()).unwrap_or(0));
    let bump = move |by: i64| {
        let next = (n.get_untracked() as i64 + by).max(0);
        count.set(if next == 0 { String::new() } else { next.to_string() });
    };
    let name = m.display_name();
    let on_order = m.order == OrderState::Ordered;
    view! {
        <div class="topup-row">
            <PillDot colour=m.colour/>
            <span class="tu-main">
                <span class="tu-name">
                    {m.name.clone()}
                    <span class="med-strength">{m.strength.clone()}</span>
                </span>
                <span class="tu-sub" class:up=move || { n.get() > 0 }>
                    {move || match n.get() {
                        0 => format!("{stock} in the box"),
                        k => format!("{stock} in the box → {}", stock.saturating_add(k)),
                    }}
                    {on_order.then(|| view! { <span class="tu-tag">"On order"</span> })}
                </span>
            </span>
            <div class="tu-controls">
                {m
                    .box_size
                    .map(|b| {
                        view! {
                            <button type="button" class="tu-box" on:click=move |_| bump(b as i64)>
                                <Icon name="plus"/>
                                {format!("Box of {b}")}
                            </button>
                        }
                    })}
                <div class="tu-count">
                    <button
                        type="button"
                        class="step-btn"
                        aria-label=format!("Fewer {name}")
                        disabled=move || n.get() == 0
                        on:click=move |_| bump(-1)
                    >
                        <Icon name="minus"/>
                    </button>
                    // Text, not number: browsers blank a half-typed number, which would clear the field.
                    <input
                        type="text"
                        inputmode="numeric"
                        autocomplete="off"
                        placeholder="0"
                        aria-label=format!("Tablets of {name} that came")
                        prop:value=move || count.get()
                        on:input=move |e| count.set(event_target_value(&e))
                    />
                    <button
                        type="button"
                        class="step-btn"
                        aria-label=format!("More {name}")
                        on:click=move |_| bump(1)
                    >
                        <Icon name="plus"/>
                    </button>
                </div>
            </div>
        </div>
    }
        .into_any()
}

//! What to order from the pharmacy, what's on its way, and receiving it into stock.

use leptos::prelude::*;
use leptos::task::spawn_local;
use meds_core::{awaiting, days_left, mark_ordered, order_reason, order_text, receive, to_order, OrderReason};

use crate::ui::{parse_count, Icon, PillDot};
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

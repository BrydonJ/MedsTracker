//! The weekly packing flow: start a week, tick medicines off one by one, check the organiser.

use std::time::Duration;

use chrono::Days;
use leptos::leptos_dom::helpers::set_timeout;
use leptos::prelude::*;
use meds_core::{
    compartment_pieces, finish_packing, next_week, order_reason, set_packed, short_for_week,
    start_packing, week_plan, OrderState, PackLine, Slot,
};

use crate::ui::{Icon, PillDot, Segmented};
use crate::{fmt, store, use_app, Tab};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Welcome,
    Start,
    Packing,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Pack,
    Check,
}

#[component]
pub fn PackView() -> impl IntoView {
    let app = use_app();
    // Memoised so ticking a medicine off doesn't rebuild the whole screen.
    let stage = Memo::new(move |_| {
        app.data.with(|d| {
            if d.meds.is_empty() {
                Stage::Welcome
            } else if d.packing.is_some() {
                Stage::Packing
            } else {
                Stage::Start
            }
        })
    });
    view! {
        {move || match stage.get() {
            Stage::Welcome => view! { <Welcome/> }.into_any(),
            Stage::Packing => view! { <Packing/> }.into_any(),
            Stage::Start => view! { <StartWeek/> <Recent/> }.into_any(),
        }}
    }
}

#[component]
fn Welcome() -> impl IntoView {
    let app = use_app();
    view! {
        <section class="welcome card reveal">
            <div class="welcome-art" aria-hidden="true">
                <span class="cap a"></span>
                <span class="cap b"></span>
                <span class="cap c"></span>
            </div>
            <h1>"Pack the week in minutes"</h1>
            <p class="muted">
                "Add each medicine once. Every week the app walks you through the pill organiser one medicine at a time, works out the half tablets, and keeps a running list of what to order."
            </p>
            <div class="welcome-actions">
                <button class="btn primary big" on:click=move |_| app.add_medicine()>
                    <Icon name="plus"/>
                    "Add the first medicine"
                </button>
                <button class="btn big" on:click=move |_| app.go(Tab::Settings)>
                    <Icon name="upload"/>
                    "Restore a backup"
                </button>
            </div>
            <p class="fine">"Everything stays on this device."</p>
        </section>
    }
}

#[component]
fn StartWeek() -> impl IntoView {
    let app = use_app();
    let week = RwSignal::new(app.data.with_untracked(|d| next_week(d, app.today.get_untracked())));
    let plan = Memo::new(move |_| app.data.with(week_plan));
    let tablets = move || plan.with(|p| p.iter().map(|l| l.take).sum::<u32>());
    let cuts = move || plan.with(|p| p.iter().map(|l| l.cut).sum::<u32>());
    let short = Memo::new(move |_| {
        app.data.with(|d| {
            d.sorted_meds()
                .into_iter()
                .filter(|m| m.is_active())
                .filter_map(|m| short_for_week(m).map(|n| (m.display_name(), m.stock.unwrap_or(0), n)))
                .collect::<Vec<_>>()
        })
    });
    let paused = move || {
        app.data.with(|d| {
            d.sorted_meds()
                .into_iter()
                .filter(|m| m.paused)
                .map(|m| m.display_name())
                .collect::<Vec<_>>()
        })
    };
    let order_count = move || app.data.with(|d| meds_core::to_order(d).len());

    let start = move |_| {
        app.data.update(|d| start_packing(d, week.get_untracked()));
        crate::ui::scroll_to_top();
    };

    view! {
        <section class="hero reveal">
            <div class="hero-top">
                <span class="eyebrow">"Next organiser"</span>
                <div class="week-nav">
                    <button class="icon-btn on-hero" aria-label="Previous week" on:click=move |_| week.update(|w| *w = *w - Days::new(7))>
                        <Icon name="chev-left"/>
                    </button>
                    <span class="week-label">{move || fmt::week_range(week.get())}</span>
                    <button class="icon-btn on-hero" aria-label="Next week" on:click=move |_| week.update(|w| *w = *w + Days::new(7))>
                        <Icon name="chev-right"/>
                    </button>
                </div>
            </div>
            <div class="hero-stats">
                <div>
                    <strong>{move || plan.with(|p| p.len())}</strong>
                    <span>"medicines"</span>
                </div>
                <div>
                    <strong>{tablets}</strong>
                    <span>"tablets"</span>
                </div>
                <div>
                    <strong>{cuts}</strong>
                    <span>"to halve"</span>
                </div>
            </div>
            <button class="btn hero-btn" on:click=start disabled=move || plan.with(|p| p.is_empty())>
                "Start packing"
                <Icon name="chev-right"/>
            </button>
        </section>

        <Show when=move || !short.with(|s| s.is_empty())>
            <div class="notice warn reveal" style="--i:1">
                <Icon name="alert"/>
                <div>
                    <strong>"Not enough for a full week"</strong>
                    <ul>
                        {move || {
                            short
                                .get()
                                .into_iter()
                                .map(|(name, have, missing)| {
                                    view! {
                                        <li>
                                            {name} " · " {have} " in the box, " {missing} " short"
                                        </li>
                                    }
                                })
                                .collect_view()
                        }}
                    </ul>
                </div>
            </div>
        </Show>
        <Show when=move || { order_count() > 0 }>
            <button class="notice reveal link-notice" style="--i:2" on:click=move |_| app.go(Tab::Order)>
                <Icon name="bag"/>
                <span>
                    {move || fmt::plural(order_count() as u32, "medicine", "medicines")} " to order"
                </span>
                <Icon name="chev-right"/>
            </button>
        </Show>
        <Show when=move || !paused().is_empty()>
            <p class="paused-note reveal" style="--i:3">
                <Icon name="pause"/>
                {move || format!("Paused, not packed: {}", paused().join(", "))}
            </p>
        </Show>
    }
}

#[component]
fn Recent() -> impl IntoView {
    let app = use_app();
    let rows = move || {
        app.data
            .with(|d| d.history.iter().take(6).cloned().collect::<Vec<_>>())
    };
    view! {
        <Show when=move || !rows().is_empty()>
            <section class="card recent reveal" style="--i:3">
                <div class="card-head">
                    <h3>
                        <Icon name="history"/>
                        "Recently packed"
                    </h3>
                </div>
                <ul class="rows">
                    {move || {
                        let today = app.today.get();
                        rows()
                            .into_iter()
                            .map(|r| {
                                let skipped = (!r.skipped.is_empty())
                                    .then(|| match r.skipped.len() {
                                        n @ 4.. => format!("Skipped {} and {} more", r.skipped[..2].join(", "), n - 2),
                                        _ => format!("Skipped {}", r.skipped.join(", ")),
                                    });
                                view! {
                                    <li class="row">
                                        <div class="row-main">
                                            <strong>{format!("Week of {}", fmt::week_range(r.week_of))}</strong>
                                            <span class="muted">
                                                {format!(
                                                    "Packed {} · {} · {}",
                                                    fmt::ago(r.finished, today),
                                                    fmt::plural(r.packed, "medicine", "medicines"),
                                                    fmt::plural(r.tablets, "tablet", "tablets"),
                                                )}
                                            </span>
                                            {skipped.map(|s| view! { <span class="warn-text">{s}</span> })}
                                        </div>
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
            </section>
        </Show>
    }
}

#[component]
fn Packing() -> impl IntoView {
    let app = use_app();
    let mode = RwSignal::new(Mode::Pack);
    let plan = Memo::new(move |_| app.data.with(week_plan));
    let packed_ids = Memo::new(move |_| {
        app.data
            .with(|d| d.packing.as_ref().map(|p| p.packed.clone()).unwrap_or_default())
    });
    let todo = Memo::new(move |_| {
        let packed = packed_ids.get();
        plan.with(|p| {
            p.iter()
                .map(|l| l.med_id)
                .filter(|id| !packed.contains(id))
                .collect::<Vec<_>>()
        })
    });
    let done = Memo::new(move |_| {
        let packed = packed_ids.get();
        plan.with(|p| {
            p.iter()
                .map(|l| l.med_id)
                .filter(|id| packed.contains(id))
                .collect::<Vec<_>>()
        })
    });
    let total = move || plan.with(|p| p.len());
    let done_n = move || done.with(|d| d.len());
    let week_of = move || {
        app.data
            .with(|d| d.packing.as_ref().map(|p| p.week_of))
            .unwrap_or_else(|| app.today.get())
    };

    let finish = move || {
        let left: Vec<String> = app.data.with_untracked(|d| {
            todo.get_untracked()
                .iter()
                .filter_map(|id| d.med(*id).map(|m| m.display_name()))
                .collect()
        });
        if !left.is_empty()
            && !store::confirm(&format!(
                "Not packed yet: {}.\n\nFinish anyway? Only packed medicines come out of stock.",
                left.join(", ")
            ))
        {
            return;
        }
        let today = app.today.get_untracked();
        app.mutate_with_undo("Week packed, stock updated", |d| {
            finish_packing(d, today);
        });
        crate::ui::scroll_to_top();
    };
    let cancel = move |_| {
        if store::confirm("Stop packing this week? Nothing comes out of stock.") {
            app.mutate_with_undo("Packing cancelled", |d| d.packing = None);
        }
    };

    view! {
        <section class="pack-head">
            <div class="pack-title">
                <div>
                    <span class="eyebrow">"Packing"</span>
                    <h1>{move || format!("Week of {}", fmt::week_range(week_of()))}</h1>
                </div>
                <button class="btn ghost small" on:click=cancel>"Cancel"</button>
            </div>
            <div class="progress" role="progressbar" aria-valuemin="0" aria-valuemax=total aria-valuenow=done_n>
                <span
                    class="progress-fill"
                    style=move || format!("--p:{}", if total() == 0 { 1.0 } else { done_n() as f64 / total() as f64 })
                ></span>
            </div>
            <div class="pack-meta">
                <span>
                    <strong>{done_n}</strong>
                    " of "
                    {total}
                    " packed"
                </span>
                <Segmented
                    options=vec![(Mode::Pack, "One by one"), (Mode::Check, "Check organiser")]
                    value=mode
                    on_change=Callback::new(move |m| mode.set(m))
                    label="View"
                />
            </div>
        </section>

        {move || match mode.get() {
            Mode::Pack => {
                view! {
                    <Show when=move || todo.with(|t| t.is_empty())>
                        <div class="all-done reveal">
                            <span class="done-mark">
                                <Icon name="check"/>
                            </span>
                            <h2>"Everything's in the organiser"</h2>
                            <p class="muted">"Have a quick look in Check organiser, then finish the week."</p>
                        </div>
                    </Show>
                    <div class="pack-list">
                        <For each=move || todo.get() key=|id| *id let:id>
                            <PackCard id=id/>
                        </For>
                    </div>
                    <Show when=move || !done.with(|d| d.is_empty())>
                        <section class="packed-list">
                            <h3 class="section-label">{move || format!("Packed · {}", done_n())}</h3>
                            <div class="chips-wrap">
                                <For each=move || done.get() key=|id| *id let:id>
                                    <PackedChip id=id/>
                                </For>
                            </div>
                        </section>
                    </Show>
                }
                    .into_any()
            }
            Mode::Check => view! { <Organiser/> }.into_any(),
        }}

        <div class="finish-bar">
            <button class="btn big" class:primary=move || todo.with(|t| t.is_empty()) on:click=move |_| finish()>
                <Icon name="check"/>
                {move || {
                    let left = todo.with(|t| t.len());
                    if left == 0 { "Finish week".to_string() } else { format!("Finish week ({left} left)") }
                }}
            </button>
        </div>
    }
}

#[component]
fn PackCard(id: u32) -> impl IntoView {
    let app = use_app();
    let med = Memo::new(move |_| app.data.with(|d| d.med(id).cloned()));
    let line = Memo::new(move |_| med.with(|m| m.as_ref().map(PackLine::for_med)));
    let leaving = RwSignal::new(false);

    let pack = move || {
        if leaving.get_untracked() {
            return;
        }
        store::tick();
        leaving.set(true);
        let delay = if store::prefers_reduced_motion() { 0 } else { 260 };
        set_timeout(
            move || app.data.update(|d| set_packed(d, id, true)),
            Duration::from_millis(delay),
        );
    };

    let low_days = move || app.data.with(|d| d.settings.low_days);
    let order = move || med.with(|m| m.as_ref().map(|m| (m.order, order_reason(m, low_days()))));
    let toggle_flag = move |_| {
        app.data.update(|d| {
            if let Some(m) = d.med_mut(id) {
                m.order = match m.order {
                    OrderState::Needed => OrderState::None,
                    _ => OrderState::Needed,
                };
                m.ordered_on = None;
            }
        })
    };

    view! {
        {move || {
            let (Some(m), Some(l)) = (med.get(), line.get()) else {
                return ().into_any();
            };
            let colour = m.colour;
            let slots = Slot::ALL
                .into_iter()
                .filter_map(|s| fmt::slot_instruction(&l, s).map(|t| (s, t)))
                .map(|(s, text)| {
                    let icon = if s == Slot::Morning { "sun" } else { "moon" };
                    view! {
                        <div class="pc-slot" class:night={s == Slot::Night}>
                            <Icon name=icon/>
                            <span class="pc-slot-label">{s.label()}</span>
                            <strong>{text}</strong>
                        </div>
                    }
                })
                .collect_view();
            let grid = (!m.is_daily()).then(|| view! { <MiniGrid line=l.clone()/> });
            let halves = (l.cut > 0 || l.use_spare > 0)
                .then(|| {
                    let mut parts = Vec::new();
                    if l.use_spare > 0 {
                        parts.push(format!("use {} spare from last week", fmt::plural(l.use_spare, "half", "halves")));
                    }
                    if l.cut > 0 {
                        parts.push(format!("cut {} in half", fmt::plural(l.cut, "tablet", "tablets")));
                    }
                    if l.spare_after > 0 {
                        parts.push(format!("keep {} for next week", fmt::plural(l.spare_after, "half", "halves")));
                    }
                    let mut text = parts.join(", ");
                    if let Some(c) = text.get_mut(0..1) {
                        c.make_ascii_uppercase();
                    }
                    view! {
                        <p class="pc-cut">
                            <Icon name="scissors"/>
                            {text}
                        </p>
                    }
                });
            let stock = m
                .stock
                .map(|s| {
                    let after = s.saturating_sub(l.take);
                    if l.take > s {
                        view! {
                            <p class="pc-stock bad">
                                <Icon name="alert"/>
                                {format!("Only {s} in the box, {} short", l.take - s)}
                            </p>
                        }
                            .into_any()
                    } else {
                        view! {
                            <p class="pc-stock">
                                <Icon name="box"/>
                                {format!("Take {} · {after} left after", l.take)}
                            </p>
                        }
                            .into_any()
                    }
                });
            let note = (!m.note.trim().is_empty()).then(|| view! { <p class="pc-note">{m.note.clone()}</p> });
            let strength = m.strength.clone();
            view! {
                <article class="pack-card" class:leaving=move || leaving.get()>
                    <div class="pc-top">
                        <PillDot colour=colour/>
                        <h3 class="pc-name">
                            {m.name.clone()}
                            <span class="pc-strength">{strength}</span>
                        </h3>
                        {move || {
                            let (state, reason) = order().unwrap_or((OrderState::None, None));
                            // Ordered and automatic low-stock flags are status, not toggles.
                            let status = match (state, reason) {
                                (OrderState::Ordered, _) => Some("Ordered"),
                                (OrderState::None, Some(_)) => Some("Low stock"),
                                _ => None,
                            };
                            if let Some(label) = status {
                                return view! {
                                    <span class="flag-chip on static">
                                        <Icon name="flag"/>
                                        {label}
                                    </span>
                                }
                                    .into_any();
                            }
                            let on = state == OrderState::Needed;
                            let label = if on { "To order" } else { "Running low?" };
                            view! {
                                <button
                                    class="flag-chip"
                                    class:on=on
                                    aria-pressed=on.to_string()
                                    title="Add to the order list"
                                    on:click=toggle_flag
                                >
                                    <Icon name="flag"/>
                                    {label}
                                </button>
                            }
                                .into_any()
                        }}
                    </div>
                    <div class="pc-slots">{slots}</div>
                    {grid}
                    {halves}
                    {stock}
                    {note}
                    <button class="btn big primary pack-btn" on:click=move |_| pack()>
                        <Icon name="check"/>
                        "Packed"
                    </button>
                </article>
            }
                .into_any()
        }}
    }
}

/// A 7 × 2 picture of the organiser showing which compartments a medicine goes in.
#[component]
fn MiniGrid(line: PackLine) -> impl IntoView {
    view! {
        <div class="mini-grid" aria-hidden="true">
            {fmt::DAY_LETTERS.iter().map(|d| view! { <span class="mg-head">{*d}</span> }).collect_view()}
            {Slot::ALL
                .into_iter()
                .flat_map(|s| {
                    let row = line.grid[s.index()];
                    row.into_iter()
                        .map(move |h| {
                            view! {
                                <span class="mg-cell" class:on={h > 0} class:night={s == Slot::Night}>
                                    {(h > 0).then(|| fmt::halves(h))}
                                </span>
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn PackedChip(id: u32) -> impl IntoView {
    let app = use_app();
    let med = Memo::new(move |_| app.data.with(|d| d.med(id).map(|m| (m.colour, m.display_name()))));
    view! {
        {move || {
            med.get()
                .map(|(colour, name)| {
                    view! {
                        <button
                            class="packed-chip"
                            title="Tap to put back on the list"
                            on:click=move |_| app.data.update(|d| set_packed(d, id, false))
                        >
                            <PillDot colour=colour/>
                            <span>{name}</span>
                            <Icon name="check"/>
                        </button>
                    }
                })
        }}
    }
}

/// Piece counts per compartment, to check the organiser by counting.
#[component]
fn Organiser() -> impl IntoView {
    let app = use_app();
    let plan = Memo::new(move |_| app.data.with(week_plan));
    let counts = Memo::new(move |_| plan.with(|p| compartment_pieces(p)));
    let selected = RwSignal::new(None::<(Slot, usize)>);

    let contents = move || {
        let (slot, day) = selected.get()?;
        let items = app.data.with(|d| {
            plan.with(|p| {
                p.iter()
                    .filter_map(|l| {
                        let h = l.grid[slot.index()][day];
                        (h > 0).then(|| d.med(l.med_id).map(|m| (m.colour, m.display_name(), h)))?
                    })
                    .collect::<Vec<_>>()
            })
        });
        Some((slot, day, items))
    };

    view! {
        <section class="card organiser reveal">
            <p class="card-sub">"Pieces in each compartment (a half counts as one). Tap one to see what's in it."</p>
            <div class="org-grid">
                <span></span>
                {fmt::DAY_NAMES.iter().map(|d| view! { <span class="org-head">{*d}</span> }).collect_view()}
                {Slot::ALL
                    .into_iter()
                    .map(|s| {
                        let icon = if s == Slot::Morning { "sun" } else { "moon" };
                        view! {
                            <span class="org-row-label" class:night={s == Slot::Night} title=s.label()>
                                <Icon name=icon/>
                            </span>
                            {(0..7)
                                .map(|day| {
                                    let n = move || counts.with(|c| c[s.index()][day]);
                                    view! {
                                        <button
                                            class="org-cell"
                                            class:night={s == Slot::Night}
                                            class:empty=move || n() == 0
                                            class:sel=move || selected.get() == Some((s, day))
                                            aria-label=move || format!("{} {}: {} pieces", fmt::DAY_NAMES[day], s.label(), n())
                                            on:click=move |_| {
                                                selected.update(|x| *x = if *x == Some((s, day)) { None } else { Some((s, day)) })
                                            }
                                        >
                                            {n}
                                        </button>
                                    }
                                })
                                .collect_view()}
                        }
                    })
                    .collect_view()}
            </div>
            {move || {
                contents()
                    .map(|(slot, day, items)| {
                        view! {
                            <div class="org-detail">
                                <h4>{format!("{} {}", fmt::DAY_NAMES[day], slot.label().to_lowercase())}</h4>
                                <ul>
                                    {items
                                        .into_iter()
                                        .map(|(colour, name, h)| {
                                            view! {
                                                <li>
                                                    <PillDot colour=colour/>
                                                    <span>{name}</span>
                                                    <strong>{fmt::halves(h)}</strong>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            </div>
                        }
                    })
            }}
        </section>
    }
}

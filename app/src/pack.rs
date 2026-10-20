//! The weekly packing flow: start a week, fill the morning row then the night row one
//! medicine at a time, check the organiser.

use std::time::Duration;

use chrono::Days;
use leptos::leptos_dom::helpers::set_timeout;
use leptos::prelude::*;
use meds_core::{
    choose_days, compartment_pieces, finish_packing, fit_to_stock, next_week, order_reason, pack_step,
    pack_steps, set_packed, short_for_week, start_packing, week_need, week_plan, OrderState, PackLine,
    Medicine, Slot, EVERY_DAY,
};

use crate::ui::{Icon, PillDot};
use crate::{fmt, store, use_app, AppState, Tab};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Welcome,
    Start,
    Packing,
}

/// What the packing screen shows: one organiser row to fill, or the whole organiser to check.
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Row(Slot),
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
                "Add each medicine once. Every week the app walks you through the organiser's morning row, then the night row, one medicine at a time. It works out the half tablets and keeps a running list of what to order."
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
                    <p>"Choose which days to pack on its card, or add a delivery that came in."</p>
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
                    <button class="btn small" on:click=move |_| app.topup.set(true)>
                        <Icon name="box"/>
                        "Add a delivery"
                    </button>
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
    let steps = Memo::new(move |_| app.data.with(pack_steps));
    let done = Memo::new(move |_| {
        app.data
            .with(|d| d.packing.as_ref().map(|p| p.done.clone()).unwrap_or_default())
    });
    // Medicine ids in each row, in packing order, split by whether they're in yet.
    let row_ids = move |slot: Slot, packed: bool| {
        Memo::new(move |_| {
            let done = done.get();
            steps.with(|s| {
                s.iter()
                    .filter(|x| x.slot == slot && done.contains(&(x.med_id, slot)) == packed)
                    .map(|x| x.med_id)
                    .collect::<Vec<_>>()
            })
        })
    };
    let todo = [row_ids(Slot::Morning, false), row_ids(Slot::Night, false)];
    let packed = [row_ids(Slot::Morning, true), row_ids(Slot::Night, true)];
    let left = move || todo[0].with(|t| t.len()) + todo[1].with(|t| t.len());
    let week_of = move || {
        app.data
            .with(|d| d.packing.as_ref().map(|p| p.week_of))
            .unwrap_or_else(|| app.today.get())
    };

    let morning_left = todo[0].with_untracked(|t| !t.is_empty());
    let view = RwSignal::new(View::Row(if morning_left { Slot::Morning } else { Slot::Night }));
    // Tapping the tab you're on keeps the row as it is, including a medicine picked from it.
    let show = move |v: View| {
        if view.get_untracked() != v {
            view.set(v);
        }
    };
    // Once the last morning medicine is in, carry straight on with the night row.
    Effect::new(move |prev: Option<usize>| {
        let n = todo[0].with(|t| t.len());
        if prev.is_some_and(|p| p > 0)
            && n == 0
            && view.get_untracked() == View::Row(Slot::Morning)
            && todo[1].with_untracked(|t| !t.is_empty())
        {
            view.set(View::Row(Slot::Night));
            crate::ui::scroll_to_top();
            app.notify("Morning row done. On to the night row.", None);
        }
        n
    });

    let finish = move || {
        let missing: Vec<String> = app.data.with_untracked(|d| {
            Slot::ALL
                .into_iter()
                .flat_map(|s| {
                    todo[s.index()]
                        .get_untracked()
                        .into_iter()
                        .filter_map(move |id| d.med(id).map(|m| format!("{} ({})", m.display_name(), s.label().to_lowercase())))
                })
                .collect()
        });
        if !missing.is_empty()
            && !store::confirm(&format!(
                "Not packed yet: {}.\n\nFinish anyway? Only packed rows come out of stock.",
                missing.join(", ")
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
            <div class="row-tabs" role="group" aria-label="Organiser row">
                {Slot::ALL
                    .into_iter()
                    .map(|s| {
                        let i = s.index();
                        let n_done = move || packed[i].with(|p| p.len());
                        let n_all = move || n_done() + todo[i].with(|t| t.len());
                        let icon = if s == Slot::Morning { "sun" } else { "moon" };
                        view! {
                            <button
                                class="row-tab"
                                class:night={s == Slot::Night}
                                class:active=move || view.get() == View::Row(s)
                                class:complete=move || { n_all() > 0 && n_done() == n_all() }
                                aria-pressed=move || (view.get() == View::Row(s)).to_string()
                                on:click=move |_| show(View::Row(s))
                            >
                                <span class="rt-top">
                                    <Icon name=icon/>
                                    <span class="rt-label">{s.label()}</span>
                                    <span class="rt-count">{n_done} "/" {n_all}</span>
                                </span>
                                <span class="rt-bar" aria-hidden="true">
                                    <span style=move || {
                                        let all = n_all();
                                        format!("--p:{}", if all == 0 { 1.0 } else { n_done() as f64 / all as f64 })
                                    }></span>
                                </span>
                            </button>
                        }
                    })
                    .collect_view()}
                <button
                    class="row-tab check"
                    class:active=move || view.get() == View::Check
                    aria-pressed=move || (view.get() == View::Check).to_string()
                    aria-label="Check the organiser"
                    on:click=move |_| show(View::Check)
                >
                    <span class="rt-top">
                        <Icon name="grid"/>
                        <span class="rt-label">"Check"</span>
                    </span>
                </button>
            </div>
        </section>

        {move || match view.get() {
            View::Row(s) => {
                let i = s.index();
                let other = s.other();
                let other_left = Signal::derive(move || todo[other.index()].with(|t| t.len()));
                view! {
                    <RowSteps
                        row=s
                        todo=todo[i]
                        packed=packed[i]
                        other_left=other_left
                        go_other=Callback::new(move |_| {
                            view.set(View::Row(other));
                            crate::ui::scroll_to_top();
                        })
                    />
                }
                    .into_any()
            }
            View::Check => view! { <Organiser/> }.into_any(),
        }}

        <div class="finish-bar" class:ready=move || left() == 0>
            <button class="btn big" class:primary=move || left() == 0 on:click=move |_| finish()>
                <Icon name="check"/>
                {move || match left() {
                    0 => "Finish week".to_string(),
                    n => format!("Finish week ({n} left)"),
                }}
            </button>
        </div>
    }
}

/// One row of the organiser: the medicine to put in now, then the rest of the row.
#[component]
fn RowSteps(
    row: Slot,
    todo: Memo<Vec<u32>>,
    packed: Memo<Vec<u32>>,
    other_left: Signal<usize>,
    go_other: Callback<()>,
) -> impl IntoView {
    // A medicine picked from "Up next"; otherwise the first one left in the row.
    let picked = RwSignal::new(None::<u32>);
    let current = Memo::new(move |_| {
        let p = picked.get();
        todo.with(|t| p.filter(|id| t.contains(id)).or_else(|| t.first().copied()))
    });
    // Once the picked one is in, go back to following the row in order.
    Effect::new(move |_| {
        if let Some(id) = picked.get() {
            if !todo.with(|t| t.contains(&id)) {
                picked.set(None);
            }
        }
    });
    let rest = Memo::new(move |_| {
        let c = current.get();
        todo.with(|t| t.iter().copied().filter(|id| Some(*id) != c).collect::<Vec<_>>())
    });
    let focus_ref = NodeRef::<leptos::html::Div>::new();
    let pick = Callback::new(move |id: u32| {
        picked.set(Some(id));
        if let Some(el) = focus_ref.get_untracked() {
            el.scroll_into_view_with_bool(true);
        }
    });
    view! {
        <Show when=move || todo.with(|t| t.is_empty())>
            {move || {
                    if other_left.get() > 0 {
                        let (other, icon) = match row {
                            Slot::Morning => ("night", "moon"),
                            Slot::Night => ("morning", "sun"),
                        };
                        view! {
                            <div class="all-done reveal">
                                <span class="done-mark">
                                    <Icon name="check"/>
                                </span>
                                <h2>{format!("{} row done", row.label())}</h2>
                                <p class="muted">
                                    {format!("{} still to go in the {other} row.", fmt::plural(other_left.get() as u32, "medicine", "medicines"))}
                                </p>
                                <button class="btn primary big" on:click=move |_| go_other.run(())>
                                    <Icon name=icon/>
                                    {format!("Go to the {other} row")}
                                </button>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="all-done reveal">
                                <span class="done-mark">
                                    <Icon name="check"/>
                                </span>
                                <h2>{format!("The {} row is done", row.label().to_lowercase())}</h2>
                                <p class="muted">"Have a quick look in Check, then finish the week."</p>
                            </div>
                        }
                            .into_any()
                    }
                }}
        </Show>
        <div class="step-focus" node_ref=focus_ref>
            {move || current.get().map(|id| view! { <StepCard id=id row=row/> })}
        </div>
        <Show when=move || !rest.with(|r| r.is_empty())>
            <section class="up-next">
                <h3 class="section-label">{move || format!("Up next · {}", rest.with(|r| r.len()))}</h3>
                <div class="step-rows">
                    <For each=move || rest.get() key=|id| *id let:id>
                        <StepRow id=id row=row on_pick=pick/>
                    </For>
                </div>
            </section>
        </Show>
        <Show when=move || !packed.with(|p| p.is_empty())>
            <section class="packed-list">
                <h3 class="section-label">{move || format!("In the {} row · {}", row.label().to_lowercase(), packed.with(|p| p.len()))}</h3>
                <div class="chips-wrap">
                    <For each=move || packed.get() key=|id| *id let:id>
                        <PackedChip id=id row=row/>
                    </For>
                </div>
            </section>
        </Show>
    }
}

/// Marks a step packed after a short exit animation.
fn pack_later(app: AppState, id: u32, row: Slot, leaving: RwSignal<bool>) {
    if leaving.get_untracked() {
        return;
    }
    store::tick();
    leaving.set(true);
    let delay = if store::prefers_reduced_motion() { 0 } else { 240 };
    set_timeout(
        move || app.data.update(|d| set_packed(d, id, row, true)),
        Duration::from_millis(delay),
    );
}

/// Half tablets in each compartment of this row, and the weekdays that get them.
fn row_dose(line: &PackLine, row: Slot) -> (u32, u8) {
    let days = line.slot_days(row);
    let per = days.first().map(|d| line.grid[row.index()][*d]).unwrap_or(0);
    let mask = days.iter().fold(0u8, |m, d| m | (1 << d));
    (per, mask)
}

/// Everything a step card shows, worked out from the saved data.
#[derive(Clone, PartialEq)]
struct StepInfo {
    med: Medicine,
    /// This row's step on this week's days.
    line: PackLine,
    /// This week's days, `[morning, night]`.
    days: [u8; 2],
    /// Tablets both rows need from the box this week.
    need: u32,
    /// The days the box covers, for "Fit to stock".
    fit: u8,
    other_done: bool,
}

impl StepInfo {
    fn short(&self) -> bool {
        self.med.stock.is_some_and(|s| self.need > s)
    }
}

/// The medicine to put in right now, with everything needed to do it. The card itself
/// stays put while its parts update, so choosing days doesn't replay its entry animation
/// or lose focus.
#[component]
fn StepCard(id: u32, row: Slot) -> impl IntoView {
    let app = use_app();
    let info = Memo::new(move |_| {
        app.data.with(|d| {
            d.med(id).map(|m| {
                let days = d.week_days(m);
                let other_done = d.row_done(id, row.other());
                StepInfo {
                    line: pack_step(m, days, row, other_done).line,
                    need: week_need(d, m),
                    fit: fit_to_stock(d, m, row),
                    med: m.clone(),
                    days,
                    other_done,
                }
            })
        })
    });
    // Short, or already narrowed down (so the days can be put back after a delivery).
    let picking = Memo::new(move |_| {
        info.with(|i| {
            i.as_ref()
                .is_some_and(|i| i.short() || i.days[row.index()] != i.med.days & EVERY_DAY)
        })
    });
    let head = Memo::new(move |_| {
        info.with(|i| i.as_ref().map(|i| (i.med.colour, i.med.name.clone(), i.med.strength.clone(), i.med.note.clone())))
    });
    let leaving = RwSignal::new(false);
    let icon = if row == Slot::Morning { "sun" } else { "moon" };

    let dose = move || {
        info.with(|i| {
            let i = i.as_ref()?;
            let (per, mask) = row_dose(&i.line, row);
            let unit = if per <= 2 { "tablet" } else { "tablets" };
            let caption = if per == 0 {
                "No days chosen this week".to_string()
            } else if mask == EVERY_DAY {
                format!("{unit} in every compartment")
            } else {
                format!("{unit} · {}", fmt::days(mask))
            };
            let amount = if per == 0 { "–".to_string() } else { fmt::halves(per) };
            Some(view! {
                <strong>{amount}</strong>
                <span>{caption}</span>
            })
        })
    };
    let strip = move || {
        if picking.get() {
            return view! { <DayPicker id=id row=row info=info/> }.into_any();
        }
        info.with(|i| {
            i.as_ref()
                .filter(|i| row_dose(&i.line, row).1 != EVERY_DAY)
                .map(|i| view! { <DayStrip line=i.line.clone() row=row/> })
        })
        .into_any()
    };
    let halves = move || {
        info.with(|i| {
            let i = i.as_ref()?;
            let l = &i.line;
            if l.cut == 0 && l.use_spare == 0 {
                return None;
            }
            let mut parts = Vec::new();
            if l.use_spare > 0 {
                parts.push(format!("use {} you already have", fmt::plural(l.use_spare, "spare half", "spare halves")));
            }
            if l.cut > 0 {
                parts.push(format!("cut {} in half", fmt::plural(l.cut, "tablet", "tablets")));
            }
            if l.spare_after > 0 {
                // An odd number of halves in the other row, still to pack, uses it up.
                let other = row.other();
                let other_takes = i.med.dose_halves(other) % 2 == 1 && i.days[other.index()] != 0;
                let when = if other_takes && !i.other_done {
                    format!("the {} row", other.label().to_lowercase())
                } else {
                    "next week".to_string()
                };
                parts.push(format!("keep {} for {when}", fmt::plural(l.spare_after, "half", "halves")));
            }
            let mut text = parts.join(", ");
            if let Some(c) = text.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            Some(view! {
                <p class="pc-cut">
                    <Icon name="scissors"/>
                    {text}
                </p>
            })
        })
    };
    // When short, the day picker says so instead.
    let stock = move || {
        info.with(|i| {
            let i = i.as_ref().filter(|i| !i.short())?;
            let s = i.med.stock?;
            let take = match i.line.take {
                0 => "Nothing new from the box".to_string(),
                n => format!("Take {n} from the box"),
            };
            Some(view! {
                <p class="pc-stock">
                    <Icon name="box"/>
                    {format!("{take} · {} left after this week", s - i.need)}
                </p>
            })
        })
    };

    view! {
        <Show when=move || head.with(|h| h.is_some())>
            <article class="pack-card" class:leaving=move || leaving.get()>
                {move || {
                    head.get()
                        .map(|(colour, name, strength, _)| {
                            view! {
                                <div class="pc-top">
                                    <PillDot colour=colour/>
                                    <h3 class="pc-name">
                                        {name}
                                        <span class="pc-strength">{strength}</span>
                                    </h3>
                                    <FlagChip id=id/>
                                </div>
                            }
                        })
                }}
                <div class="step-dose" class:night={row == Slot::Night}>
                    <span class="sd-row">
                        <Icon name=icon/>
                        {format!("{} row", row.label())}
                    </span>
                    <span class="sd-amount">{dose}</span>
                </div>
                {strip}
                {halves}
                // The button comes before stock and notes so it stays in reach on small phones.
                <button class="btn big primary pack-btn" on:click=move |_| pack_later(app, id, row, leaving)>
                    <Icon name="check"/>
                    "Packed"
                </button>
                {stock}
                {move || {
                    head.get()
                        .map(|h| h.3)
                        .filter(|n| !n.trim().is_empty())
                        .map(|n| view! { <p class="pc-note">{n}</p> })
                }}
            </article>
        </Show>
    }
}

/// "Running low?" toggle, or the order status when it isn't the user's to toggle.
#[component]
fn FlagChip(id: u32) -> impl IntoView {
    let app = use_app();
    let order = Memo::new(move |_| {
        app.data.with(|d| {
            d.med(id)
                .map(|m| (m.order, order_reason(m, d.settings.low_days)))
                .unwrap_or((OrderState::None, None))
        })
    });
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
    move || {
        let (state, reason) = order.get();
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
    }
}

const DAY_FULL: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// Which days this row gets this week, for when the box won't cover all of them. Each day
/// is a toggle that updates in place, so it keeps focus between taps.
#[component]
fn DayPicker(id: u32, row: Slot, info: Memo<Option<StepInfo>>) -> impl IntoView {
    let app = use_app();
    let pick = move |f: fn(&StepInfo, Slot) -> u32| {
        Memo::new(move |_| info.with(|i| i.as_ref().map(|i| f(i, row)).unwrap_or(0)))
    };
    let mask = pick(|i, row| i.days[row.index()] as u32);
    let own = pick(|i, _| (i.med.days & EVERY_DAY) as u32);
    let dose = pick(|i, row| i.med.dose_halves(row));
    let fit = pick(|i, _| i.fit as u32);
    let short = Memo::new(move |_| info.with(|i| i.as_ref().is_some_and(StepInfo::short)));
    let tracked = Memo::new(move |_| info.with(|i| i.as_ref().is_some_and(|i| i.med.stock.is_some())));
    let shared = Memo::new(move |_| {
        info.with(|i| i.as_ref().is_some_and(|i| !i.other_done && i.med.dose_halves(row.other()) > 0))
    });
    let set = move |mask: u32| app.data.update(|d| choose_days(d, id, row, mask as u8));
    let status = move || {
        info.with(|i| {
            let i = i.as_ref()?;
            let need = i.need;
            Some(match i.med.stock {
                Some(s) if need > s => format!("Needs {need}, only {s} in the box: {} short", need - s),
                Some(s) => format!("Needs {need} of the {s} in the box"),
                None => format!("Needs {need}"),
            })
        })
    };
    let row_name = row.label().to_lowercase();
    let unit = move || if dose.get() <= 2 { "tablet" } else { "tablets" };
    view! {
        <div class="day-pick" class:short=move || short.get()>
            <p class="dp-status">
                {move || view! { <Icon name=if short.get() { "alert" } else { "box" }/> }}
                {status}
            </p>
            <div
                class="day-strip pick"
                class:night={row == Slot::Night}
                role="group"
                aria-label=format!("Days to pack in the {row_name} row")
            >
                {(0..7)
                    .map(|d| {
                        let bit = 1u32 << d;
                        let on = move || mask.get() & bit != 0;
                        view! {
                            <button
                                type="button"
                                class="ds-cell"
                                class:on=on
                                disabled=move || own.get() & bit == 0
                                aria-pressed=move || on().to_string()
                                aria-label=move || {
                                    if on() {
                                        format!("{}, {} {}", DAY_FULL[d], fmt::halves(dose.get()), unit())
                                    } else {
                                        format!("{}, not packed", DAY_FULL[d])
                                    }
                                }
                                on:click=move |_| set(mask.get_untracked() ^ bit)
                            >
                                <span class="ds-day">{fmt::DAY_LETTERS[d]}</span>
                                <span class="ds-n">{move || if on() { fmt::halves(dose.get()) } else { "–".into() }}</span>
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <div class="dp-actions">
                {move || {
                    (short.get() && fit.get() != mask.get())
                        .then(|| {
                            let f = fit.get();
                            let label = match f {
                                0 => "Fit to stock: skip this week".to_string(),
                                f => format!("Fit to stock: {}", fmt::days(f as u8)),
                            };
                            view! {
                                <button type="button" class="dp-btn primary" on:click=move |_| set(f)>
                                    <Icon name="check"/>
                                    {label}
                                </button>
                            }
                        })
                }}
                {move || {
                    (mask.get() != own.get())
                        .then(|| {
                            view! {
                                <button type="button" class="dp-btn" on:click=move |_| set(own.get_untracked())>
                                    "All days"
                                </button>
                            }
                        })
                }}
                // The delivery sheet only lists medicines with a stock count.
                <Show when=move || tracked.get()>
                    <button type="button" class="dp-btn" on:click=move |_| app.topup.set(true)>
                        <Icon name="box"/>
                        "Add stock"
                    </button>
                </Show>
            </div>
            <Show when=move || shared.get()>
                <p class="hint">{format!("The {} row gets the same days.", row.other().label().to_lowercase())}</p>
            </Show>
        </div>
    }
}

/// The week along one row of the organiser, with the dose in the days that get it.
#[component]
fn DayStrip(line: PackLine, row: Slot) -> impl IntoView {
    let cells = line.grid[row.index()];
    view! {
        <div class="day-strip" class:night={row == Slot::Night} aria-hidden="true">
            {cells
                .into_iter()
                .enumerate()
                .map(|(d, h)| {
                    view! {
                        <span class="ds-cell" class:on={h > 0}>
                            <span class="ds-day">{fmt::DAY_LETTERS[d]}</span>
                            <span class="ds-n">{if h > 0 { fmt::halves(h) } else { "–".into() }}</span>
                        </span>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// A medicine further down the row: tap the name to open it, or the tick if it's already in.
#[component]
fn StepRow(id: u32, row: Slot, on_pick: Callback<u32>) -> impl IntoView {
    let app = use_app();
    let info = Memo::new(move |_| {
        app.data.with(|d| {
            d.med(id).map(|m| {
                let (per, mask) = row_dose(&pack_step(m, d.week_days(m), row, d.row_done(id, row.other())).line, row);
                let when = (mask != EVERY_DAY).then(|| fmt::days(mask));
                (m.colour, m.name.clone(), m.strength.clone(), per, when)
            })
        })
    });
    let leaving = RwSignal::new(false);
    move || {
        info.get().map(|(colour, name, strength, per, when)| {
            view! {
                <div class="step-row" class:leaving=move || leaving.get()>
                    <button class="sr-main" on:click=move |_| on_pick.run(id)>
                        <PillDot colour=colour/>
                        <span class="sr-name">
                            {name.clone()}
                            <span class="sr-strength">{strength.clone()}</span>
                        </span>
                        <span class="sr-dose" class:night={row == Slot::Night}>
                            {if per == 0 { "–".to_string() } else { fmt::halves(per) }}
                            {when.clone().map(|w| view! { <small>{w}</small> })}
                        </span>
                    </button>
                    <button
                        class="sr-tick"
                        aria-label=format!("Mark {name} packed in the {} row", row.label().to_lowercase())
                        on:click=move |_| pack_later(app, id, row, leaving)
                    >
                        <Icon name="check"/>
                    </button>
                </div>
            }
        })
    }
}

#[component]
fn PackedChip(id: u32, row: Slot) -> impl IntoView {
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
                            on:click=move |_| app.data.update(|d| set_packed(d, id, row, false))
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

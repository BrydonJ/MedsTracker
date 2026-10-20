//! A printable packing checklist in the layout of the paper sheet. Hidden on screen.

use leptos::prelude::*;
use meds_core::{Medicine, Slot};

use crate::{fmt, use_app};

#[component]
pub fn PrintSheet() -> impl IntoView {
    let app = use_app();
    let title = move || {
        app.data.with(|d| {
            let p = d.settings.person.trim();
            if p.is_empty() {
                "Medicine".to_string()
            } else {
                format!("{p}'s Medicine")
            }
        })
    };
    let rows = move |slot: Slot| {
        app.data.with(|d| {
            d.sorted_meds()
                .into_iter()
                .filter(|m| !m.paused && m.dose_halves(slot) > 0 && m.day_count() > 0)
                .cloned()
                .collect::<Vec<_>>()
        })
    };

    view! {
        <div class="print-sheet" aria-hidden="true">
            <header>
                <h1>{title}</h1>
                <span>{move || format!("Printed {}", fmt::date_long(app.today.get()))}</span>
            </header>
            {Slot::ALL
                .into_iter()
                .map(|slot| {
                    view! {
                        <table class:night={slot == Slot::Night}>
                            <thead>
                                <tr class="band">
                                    <th colspan="12">{slot.label()} " · Checklist for date: ____________"</th>
                                </tr>
                                <tr>
                                    <th class="l">"Medicine"</th>
                                    <th>"Strength"</th>
                                    <th>"Dose"</th>
                                    <th>"Packed?"</th>
                                    {fmt::DAY_LETTERS.iter().map(|d| view! { <th class="day">{*d}</th> }).collect_view()}
                                    <th>"Order?"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || rows(slot).into_iter().map(|m| row(&m, slot)).collect_view()}
                            </tbody>
                        </table>
                    }
                })
                .collect_view()}
        </div>
    }
}

fn row(m: &Medicine, slot: Slot) -> impl IntoView {
    view! {
        <tr>
            <td class="l">{m.name.clone()}</td>
            <td>{m.strength.clone()}</td>
            <td>{fmt::halves(m.dose_halves(slot))}</td>
            <td class="box">"☐"</td>
            {(0..7)
                .map(|d| {
                    let on = m.takes_on(d);
                    view! { <td class="day" class:off=!on>{on.then(|| fmt::DAY_LETTERS[d])}</td> }
                })
                .collect_view()}
            <td class="box">"☐"</td>
        </tr>
    }
}

//! The weekly packing plan, compartment check, and stock / ordering maths.

use chrono::{Datelike, Days, NaiveDate};

use crate::model::{Data, Medicine, OrderState, PackRecord, Packing, Slot};

/// How to pack one medicine into a 7-day, morning/night organiser.
#[derive(Clone, Debug, PartialEq)]
pub struct PackLine {
    pub med_id: u32,
    /// Half tablets per compartment, `[slot][weekday]`, weekday 0 = Monday.
    pub grid: [[u32; 7]; 2],
    pub total_halves: u32,
    /// Tablets that go in whole.
    pub whole: u32,
    /// Tablets to cut in half this week.
    pub cut: u32,
    /// Spare halves from last week that go in first.
    pub use_spare: u32,
    /// Tablets out of the box: `whole + cut`.
    pub take: u32,
    /// Halves left over after packing, kept for next week.
    pub spare_after: u32,
}

impl PackLine {
    pub fn for_med(med: &Medicine) -> PackLine {
        PackLine::for_slots(med, &Slot::ALL)
    }

    /// The line for just some of a medicine's rows; the others stay empty in `grid`.
    pub fn for_slots(med: &Medicine, slots: &[Slot]) -> PackLine {
        let mut grid = [[0u32; 7]; 2];
        let mut whole = 0;
        let mut half_pieces = 0;
        for &slot in slots {
            let h = med.dose_halves(slot);
            for (day, cell) in grid[slot.index()].iter_mut().enumerate() {
                if h > 0 && med.takes_on(day) {
                    *cell = h;
                    whole += h / 2;
                    half_pieces += h % 2;
                }
            }
        }
        let use_spare = med.spare_halves.min(half_pieces);
        let from_cutting = half_pieces - use_spare;
        let cut = from_cutting.div_ceil(2);
        let spare_after = med.spare_halves - use_spare + (cut * 2 - from_cutting);
        PackLine {
            med_id: med.id,
            grid,
            total_halves: whole * 2 + half_pieces,
            whole,
            cut,
            use_spare,
            take: whole + cut,
            spare_after,
        }
    }

    /// Weekdays (0 = Monday) with a dose in this slot.
    pub fn slot_days(&self, slot: Slot) -> Vec<usize> {
        (0..7).filter(|d| self.grid[slot.index()][*d] > 0).collect()
    }
}

/// Lines for every active medicine, alphabetical.
pub fn week_plan(data: &Data) -> Vec<PackLine> {
    data.sorted_meds()
        .into_iter()
        .filter(|m| m.is_active())
        .map(PackLine::for_med)
        .collect()
}

/// One pass along the organiser: one medicine into one row (morning or night).
#[derive(Clone, Debug, PartialEq)]
pub struct PackStep {
    pub med_id: u32,
    pub slot: Slot,
    /// Only this row's compartments are filled in.
    pub line: PackLine,
}

/// The step for one row. Once the other row is in, a half left over from cutting for it
/// goes in this row; until then only last week's spares are there. Either order adds up
/// to the same tablets and spares as packing both rows at once.
pub fn pack_step(med: &Medicine, slot: Slot, other_done: bool) -> PackStep {
    let line = if other_done {
        let mut after_other = med.clone();
        after_other.spare_halves = PackLine::for_slots(med, &[slot.other()]).spare_after;
        PackLine::for_slots(&after_other, &[slot])
    } else {
        PackLine::for_slots(med, &[slot])
    };
    PackStep {
        med_id: med.id,
        slot,
        line,
    }
}

/// Every step for the week: the whole morning row first, then the night row,
/// each alphabetical.
pub fn pack_steps(data: &Data) -> Vec<PackStep> {
    let meds: Vec<&Medicine> = data.sorted_meds().into_iter().filter(|m| m.is_active()).collect();
    Slot::ALL
        .into_iter()
        .flat_map(|slot| {
            meds.iter()
                .map(move |m| pack_step(m, slot, data.row_done(m.id, slot.other())))
                .filter(|s| s.line.total_halves > 0)
        })
        .collect()
}

/// Physical pieces (a half counts as one piece) per compartment, `[slot][weekday]`.
/// Lets you check a compartment by counting what's in it.
pub fn compartment_pieces(lines: &[PackLine]) -> [[u32; 7]; 2] {
    let mut out = [[0u32; 7]; 2];
    for line in lines {
        for (out_row, row) in out.iter_mut().zip(line.grid.iter()) {
            for (cell, h) in out_row.iter_mut().zip(row.iter()) {
                *cell += h / 2 + h % 2;
            }
        }
    }
    out
}

/// The Monday a new packing session is for: today if it's Monday, else the next one.
pub fn upcoming_monday(today: NaiveDate) -> NaiveDate {
    let back = today.weekday().num_days_from_monday() as u64;
    if back == 0 {
        today
    } else {
        today + Days::new(7 - back)
    }
}

/// The week to pack next: the upcoming Monday, or the week after the last one packed.
pub fn next_week(data: &Data, today: NaiveDate) -> NaiveDate {
    let upcoming = upcoming_monday(today);
    match data.history.iter().map(|r| r.week_of).max() {
        Some(last) if last >= upcoming => last + Days::new(7),
        _ => upcoming,
    }
}

/// Average tablets per day across the week.
pub fn daily_use(med: &Medicine) -> f64 {
    med.weekly_halves() as f64 / 2.0 / 7.0
}

/// How many days the box (plus spare halves) lasts, if stock is tracked.
pub fn days_left(med: &Medicine) -> Option<f64> {
    let stock = med.stock?;
    let per_day = daily_use(med);
    if per_day <= 0.0 {
        return None;
    }
    Some((stock as f64 + med.spare_halves as f64 / 2.0) / per_day)
}

/// Whole tablets short for packing next week, if stock is tracked.
pub fn short_for_week(med: &Medicine) -> Option<u32> {
    let stock = med.stock?;
    let take = PackLine::for_med(med).take;
    (take > stock).then(|| take - stock)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderReason {
    /// Flagged by hand.
    Flagged,
    /// Stock covers fewer days than the low-stock setting.
    LowStock,
}

/// Why a medicine needs ordering, if it does. Ordered items are waiting, not needed.
pub fn order_reason(med: &Medicine, low_days: u32) -> Option<OrderReason> {
    if med.paused {
        return None;
    }
    match med.order {
        OrderState::Needed => Some(OrderReason::Flagged),
        OrderState::Ordered => None,
        OrderState::None => match days_left(med) {
            Some(d) if d < low_days as f64 => Some(OrderReason::LowStock),
            _ => None,
        },
    }
}

/// Medicines to order, alphabetical.
pub fn to_order(data: &Data) -> Vec<&Medicine> {
    data.sorted_meds()
        .into_iter()
        .filter(|m| order_reason(m, data.settings.low_days).is_some())
        .collect()
}

/// Medicines ordered and not yet received, alphabetical.
pub fn awaiting(data: &Data) -> Vec<&Medicine> {
    data.sorted_meds()
        .into_iter()
        .filter(|m| m.order == OrderState::Ordered)
        .collect()
}

/// A plain-text list to send to the pharmacy.
pub fn order_text(data: &Data) -> String {
    let mut s = String::new();
    let who = data.settings.person.trim();
    if who.is_empty() {
        s.push_str("Medicine to order:\n");
    } else {
        s.push_str(&format!("Medicine to order for {who}:\n"));
    }
    for m in to_order(data) {
        s.push_str("• ");
        s.push_str(&m.display_name());
        s.push('\n');
    }
    s
}

/// Starts packing the week of `week_of`, keeping progress if that same week is already open.
pub fn start_packing(data: &mut Data, week_of: NaiveDate) {
    match data.packing.as_mut() {
        Some(p) if p.week_of == week_of => {}
        Some(p) => {
            // Ticks belong to the week they were made for.
            p.week_of = week_of;
            p.done.clear();
        }
        None => data.packing = Some(Packing::new(week_of)),
    }
}

pub fn set_packed(data: &mut Data, med_id: u32, slot: Slot, packed: bool) {
    let Some(p) = data.packing.as_mut() else {
        return;
    };
    p.done.retain(|x| *x != (med_id, slot));
    if packed {
        p.done.push((med_id, slot));
    }
}

/// Closes the session: packed rows come out of stock and spare halves carry over.
pub fn finish_packing(data: &mut Data, today: NaiveDate) -> Option<PackRecord> {
    let packing = data.packing.take()?;
    let mut packed = 0;
    let mut tablets = 0;
    let mut skipped = Vec::new();
    let ids: Vec<u32> = week_plan(data).iter().map(|l| l.med_id).collect();
    for id in ids {
        let Some(med) = data.med_mut(id) else {
            continue;
        };
        // Only active medicines (a dose on at least one day) get here, so `due` matches the steps.
        let due: Vec<Slot> = Slot::ALL
            .into_iter()
            .filter(|s| med.dose_halves(*s) > 0)
            .collect();
        let packed_slots: Vec<Slot> = due
            .iter()
            .copied()
            .filter(|s| packing.is_done(id, *s))
            .collect();
        if packed_slots.is_empty() {
            skipped.push(med.display_name());
            continue;
        }
        for s in due.iter().filter(|s| !packed_slots.contains(s)) {
            skipped.push(format!("{} ({})", med.display_name(), s.label().to_lowercase()));
        }
        let line = PackLine::for_slots(med, &packed_slots);
        if let Some(stock) = med.stock.as_mut() {
            *stock = stock.saturating_sub(line.take);
        }
        med.spare_halves = line.spare_after;
        packed += 1;
        tablets += line.take;
    }
    let rec = PackRecord {
        week_of: packing.week_of,
        finished: today,
        packed,
        tablets,
        skipped,
    };
    data.push_history(rec.clone());
    Some(rec)
}

/// Marks a medicine as ordered from the pharmacy.
pub fn mark_ordered(med: &mut Medicine, today: NaiveDate) {
    med.order = OrderState::Ordered;
    med.ordered_on = Some(today);
}

/// An order arrived: add the tablets to stock and clear the flag.
pub fn receive(med: &mut Medicine, tablets: u32) {
    if tablets > 0 {
        med.stock = Some(med.stock.unwrap_or(0).saturating_add(tablets));
    }
    med.order = OrderState::None;
    med.ordered_on = None;
}

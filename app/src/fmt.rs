//! Text formatting for tablets, days and dates.

use chrono::{Datelike, NaiveDate};
use meds_core::{Medicine, Slot, EVERY_DAY};

pub const DAY_LETTERS: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];
pub const DAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Half tablets as a friendly count: 1 → "½", 3 → "1½", 4 → "2".
pub fn halves(h: u32) -> String {
    match (h / 2, h % 2) {
        (0, 1) => "½".into(),
        (w, 0) => w.to_string(),
        (w, _) => format!("{w}½"),
    }
}

pub fn plural(n: u32, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// "Mon 26 Oct"
pub fn date(d: NaiveDate) -> String {
    format!(
        "{} {} {}",
        DAY_NAMES[d.weekday().num_days_from_monday() as usize],
        d.day(),
        MONTHS[d.month0() as usize]
    )
}

/// "26 Oct 2026"
pub fn date_long(d: NaiveDate) -> String {
    format!("{} {} {}", d.day(), MONTHS[d.month0() as usize], d.year())
}

/// "26 Oct – 1 Nov"
pub fn week_range(monday: NaiveDate) -> String {
    let sunday = monday + chrono::Days::new(6);
    if monday.month() == sunday.month() {
        format!("{}–{} {}", monday.day(), sunday.day(), MONTHS[sunday.month0() as usize])
    } else {
        format!(
            "{} {} – {} {}",
            monday.day(),
            MONTHS[monday.month0() as usize],
            sunday.day(),
            MONTHS[sunday.month0() as usize]
        )
    }
}

/// "Every day", "Weekdays", "Wed only", "Mon, Wed, Fri".
pub fn days(mask: u8) -> String {
    let mask = mask & EVERY_DAY;
    match mask {
        EVERY_DAY => "Every day".into(),
        0 => "No days".into(),
        0b001_1111 => "Weekdays".into(),
        0b110_0000 => "Weekends".into(),
        m if m.count_ones() == 1 => format!("{} only", DAY_NAMES[m.trailing_zeros() as usize]),
        // A run of three or more days in a row: "Mon–Thu".
        m if m.count_ones() >= 3 && (m >> m.trailing_zeros()).count_ones() == (m >> m.trailing_zeros()).trailing_ones() => {
            let first = m.trailing_zeros() as usize;
            format!("{}–{}", DAY_NAMES[first], DAY_NAMES[first + m.count_ones() as usize - 1])
        }
        m => (0..7)
            .filter(|d| m & (1 << d) != 0)
            .map(|d| DAY_NAMES[d])
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// "1 morning · 2 night", "½ morning".
pub fn dose_summary(m: &Medicine) -> String {
    let parts: Vec<String> = Slot::ALL
        .iter()
        .filter(|s| m.dose_halves(**s) > 0)
        .map(|s| format!("{} {}", halves(m.dose_halves(*s)), s.label().to_lowercase()))
        .collect();
    if parts.is_empty() {
        "No dose set".into()
    } else {
        parts.join(" · ")
    }
}

/// "≈ 3 weeks", "9 days", "1 day".
pub fn duration_days(days: f64) -> String {
    let d = days.floor().max(0.0) as u32;
    if d >= 21 {
        format!("≈ {} weeks", d / 7)
    } else {
        plural(d, "day", "days")
    }
}

/// "today", "yesterday", "3 days ago", or a date.
pub fn ago(then: NaiveDate, today: NaiveDate) -> String {
    match (today - then).num_days() {
        0 => "today".into(),
        1 => "yesterday".into(),
        n @ 2..=13 => format!("{n} days ago"),
        _ => date(then),
    }
}

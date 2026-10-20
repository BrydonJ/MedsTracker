//! Persisted data: medicines, the packing session in progress and its history.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

pub const DATA_VERSION: u32 = 1;
/// Bit mask with all seven weekdays set (bit 0 = Monday).
pub const EVERY_DAY: u8 = 0b111_1111;
const HISTORY_LIMIT: usize = 104;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Slot {
    Morning,
    Night,
}

impl Slot {
    pub const ALL: [Slot; 2] = [Slot::Morning, Slot::Night];

    pub fn index(self) -> usize {
        match self {
            Slot::Morning => 0,
            Slot::Night => 1,
        }
    }

    pub fn other(self) -> Slot {
        match self {
            Slot::Morning => Slot::Night,
            Slot::Night => Slot::Morning,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Slot::Morning => "Morning",
            Slot::Night => "Night",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderState {
    /// Nothing flagged by hand; low stock can still raise it automatically.
    #[default]
    None,
    /// Flagged by hand ("running low") while packing or from the list.
    Needed,
    /// Ordered from the pharmacy and waiting to arrive.
    Ordered,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Medicine {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub strength: String,
    /// Tablets per morning dose, in steps of ½.
    #[serde(default)]
    pub morning: f64,
    /// Tablets per night dose, in steps of ½.
    #[serde(default)]
    pub night: f64,
    /// Weekdays it is taken on, bit 0 = Monday.
    #[serde(default = "every_day")]
    pub days: u8,
    #[serde(default)]
    pub paused: bool,
    /// Index into the app's pill colour palette, to recognise it at a glance.
    #[serde(default)]
    pub colour: u8,
    #[serde(default)]
    pub note: String,
    /// Whole tablets left in the box; `None` when stock isn't tracked.
    #[serde(default)]
    pub stock: Option<u32>,
    /// Half tablets left over from cutting, used first next week.
    #[serde(default)]
    pub spare_halves: u32,
    /// Tablets in a box from the pharmacy, added to stock when an order arrives.
    #[serde(default)]
    pub box_size: Option<u32>,
    #[serde(default)]
    pub order: OrderState,
    #[serde(default)]
    pub ordered_on: Option<NaiveDate>,
}

fn every_day() -> u8 {
    EVERY_DAY
}

impl Medicine {
    pub fn new(id: u32) -> Self {
        Medicine {
            id,
            name: String::new(),
            strength: String::new(),
            morning: 1.0,
            night: 0.0,
            days: EVERY_DAY,
            paused: false,
            colour: 0,
            note: String::new(),
            stock: None,
            spare_halves: 0,
            box_size: None,
            order: OrderState::None,
            ordered_on: None,
        }
    }

    pub fn dose(&self, slot: Slot) -> f64 {
        match slot {
            Slot::Morning => self.morning,
            Slot::Night => self.night,
        }
    }

    /// The dose in half tablets, so ½ steps add up exactly.
    pub fn dose_halves(&self, slot: Slot) -> u32 {
        halves(self.dose(slot))
    }

    /// `weekday` is 0 = Monday … 6 = Sunday.
    pub fn takes_on(&self, weekday: usize) -> bool {
        weekday < 7 && self.days & (1 << weekday) != 0
    }

    pub fn day_count(&self) -> u32 {
        (self.days & EVERY_DAY).count_ones()
    }

    pub fn is_daily(&self) -> bool {
        self.days & EVERY_DAY == EVERY_DAY
    }

    /// Half tablets a full week of doses uses.
    pub fn weekly_halves(&self) -> u32 {
        Slot::ALL.iter().map(|s| self.dose_halves(*s)).sum::<u32>() * self.day_count()
    }

    /// Paused or zero-dose medicines aren't packed.
    pub fn is_active(&self) -> bool {
        !self.paused && self.weekly_halves() > 0
    }

    pub fn display_name(&self) -> String {
        if self.strength.trim().is_empty() {
            self.name.trim().to_string()
        } else {
            format!("{} {}", self.name.trim(), self.strength.trim())
        }
    }
}

/// Rounds a tablet count to the nearest half and returns it in halves.
pub fn halves(tablets: f64) -> u32 {
    if tablets.is_finite() && tablets > 0.0 {
        (tablets * 2.0).round() as u32
    } else {
        0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Whose medicine this is, used in titles and the printed sheet.
    #[serde(default)]
    pub person: String,
    /// Flag a medicine for ordering when its stock covers fewer days than this.
    #[serde(default = "default_low_days")]
    pub low_days: u32,
}

fn default_low_days() -> u32 {
    14
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            person: String::new(),
            low_days: default_low_days(),
        }
    }
}

/// A week being packed right now; survives closing the app half way.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "PackingSaved")]
pub struct Packing {
    /// Monday of the week the organiser is for.
    pub week_of: NaiveDate,
    /// Medicine and row pairs already put in the organiser.
    pub done: Vec<(u32, Slot)>,
    /// Rows packed on fewer days this week (short on stock): medicine, row, weekday mask.
    pub days: Vec<(u32, Slot, u8)>,
}

impl Packing {
    pub fn new(week_of: NaiveDate) -> Self {
        Packing {
            week_of,
            done: Vec::new(),
            days: Vec::new(),
        }
    }

    pub fn is_done(&self, med_id: u32, slot: Slot) -> bool {
        self.done.contains(&(med_id, slot))
    }

    /// The days chosen for this row this week, if they were narrowed down.
    pub fn chosen_days(&self, med_id: u32, slot: Slot) -> Option<u8> {
        self.days
            .iter()
            .find(|(id, s, _)| *id == med_id && *s == slot)
            .map(|(_, _, mask)| *mask)
    }
}

/// `Packing` as saved, including the whole-medicine ticks older versions stored.
#[derive(Deserialize)]
struct PackingSaved {
    week_of: NaiveDate,
    #[serde(default)]
    done: Vec<(u32, Slot)>,
    #[serde(default)]
    packed: Vec<u32>,
    #[serde(default)]
    days: Vec<(u32, Slot, u8)>,
}

impl From<PackingSaved> for Packing {
    fn from(s: PackingSaved) -> Self {
        let mut done = s.done;
        // A medicine ticked as a whole had both of its rows packed. A row it has no dose
        // for is ignored by the steps and by finishing, so marking it too is harmless.
        for id in s.packed {
            for slot in Slot::ALL {
                if !done.contains(&(id, slot)) {
                    done.push((id, slot));
                }
            }
        }
        Packing {
            week_of: s.week_of,
            done,
            days: s.days,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackRecord {
    pub week_of: NaiveDate,
    pub finished: NaiveDate,
    /// Number of medicines packed.
    pub packed: u32,
    /// Whole tablets taken out of boxes.
    pub tablets: u32,
    /// Names of medicines that were due but not packed.
    #[serde(default)]
    pub skipped: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    #[serde(default = "data_version")]
    pub version: u32,
    #[serde(default)]
    pub meds: Vec<Medicine>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub packing: Option<Packing>,
    /// Newest first.
    #[serde(default)]
    pub history: Vec<PackRecord>,
}

fn data_version() -> u32 {
    DATA_VERSION
}

impl Default for Data {
    fn default() -> Self {
        Data {
            version: DATA_VERSION,
            meds: Vec::new(),
            settings: Settings::default(),
            packing: None,
            history: Vec::new(),
        }
    }
}

impl Data {
    pub fn next_id(&self) -> u32 {
        self.meds.iter().map(|m| m.id).max().unwrap_or(0).saturating_add(1)
    }

    pub fn med(&self, id: u32) -> Option<&Medicine> {
        self.meds.iter().find(|m| m.id == id)
    }

    pub fn med_mut(&mut self, id: u32) -> Option<&mut Medicine> {
        self.meds.iter_mut().find(|m| m.id == id)
    }

    /// Inserts a new medicine or replaces the one with the same id.
    pub fn upsert(&mut self, med: Medicine) {
        match self.med_mut(med.id) {
            Some(m) => *m = med,
            None => self.meds.push(med),
        }
    }

    /// Whether this week's `slot` row already has this medicine in it.
    pub fn row_done(&self, med_id: u32, slot: Slot) -> bool {
        self.packing.as_ref().is_some_and(|p| p.is_done(med_id, slot))
    }

    /// Weekdays each row (`[morning, night]`) gets this week: the medicine's own days,
    /// narrowed to the ones chosen when short on stock.
    pub fn week_days(&self, med: &Medicine) -> [u8; 2] {
        Slot::ALL.map(|slot| {
            let chosen = self.packing.as_ref().and_then(|p| p.chosen_days(med.id, slot));
            med.days & chosen.unwrap_or(EVERY_DAY)
        })
    }

    pub fn remove(&mut self, id: u32) {
        self.meds.retain(|m| m.id != id);
        if let Some(p) = self.packing.as_mut() {
            p.done.retain(|(x, _)| *x != id);
            p.days.retain(|(x, _, _)| *x != id);
        }
    }

    /// Medicines in a stable, alphabetical order (name, then strength).
    pub fn sorted_meds(&self) -> Vec<&Medicine> {
        let mut v: Vec<&Medicine> = self.meds.iter().collect();
        v.sort_by_cached_key(|m| (m.name.trim().to_lowercase(), m.strength.trim().to_lowercase(), m.id));
        v
    }

    pub(crate) fn push_history(&mut self, rec: PackRecord) {
        self.history.insert(0, rec);
        self.history.truncate(HISTORY_LIMIT);
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Parses and checks a backup, so a bad file can never replace good data.
    pub fn from_json(s: &str) -> Result<Data, String> {
        let v: serde_json::Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
        // Every field has a default, so insist on the one key a real backup always has.
        if v.get("meds").is_none_or(|m| !m.is_array()) {
            return Err("not a Meds Tracker backup".into());
        }
        let d: Data = serde_json::from_value(v).map_err(|e| e.to_string())?;
        if d.version > DATA_VERSION {
            return Err("this backup is from a newer version of the app".into());
        }
        let mut ids = std::collections::HashSet::new();
        for m in &d.meds {
            if !ids.insert(m.id) {
                return Err(format!("duplicate medicine id {}", m.id));
            }
            if m.name.trim().is_empty() {
                return Err(format!("medicine {} has no name", m.id));
            }
            for dose in [m.morning, m.night] {
                if !dose.is_finite() || dose < 0.0 || dose > 20.0 || (dose * 2.0).fract() != 0.0 {
                    return Err(format!("{}: dose {dose} must be a whole or half tablet", m.name));
                }
            }
        }
        Ok(d)
    }
}

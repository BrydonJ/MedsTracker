use chrono::NaiveDate;

use crate::*;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn med(id: u32, name: &str, morning: f64, night: f64) -> Medicine {
    let mut m = Medicine::new(id);
    m.name = name.into();
    m.morning = morning;
    m.night = night;
    m
}

#[test]
fn daily_whole_tablets() {
    let line = PackLine::for_med(&med(1, "Cipalat", 1.0, 0.0));
    assert_eq!(line.grid[0], [2; 7]);
    assert_eq!(line.grid[1], [0; 7]);
    assert_eq!((line.whole, line.cut, line.take, line.spare_after), (7, 0, 7, 0));
}

#[test]
fn both_slots_add_up() {
    // Essentiale: 1 in the morning and 2 at night.
    let line = PackLine::for_med(&med(1, "Essentiale", 1.0, 2.0));
    assert_eq!(line.take, 21);
    assert_eq!(line.total_halves, 42);
}

#[test]
fn half_tablets_cut_and_carry_a_spare() {
    // Cardicor ½ a day: 7 halves need 4 tablets cut, leaving one spare half.
    let mut m = med(1, "Cardicor", 0.5, 0.0);
    let line = PackLine::for_med(&m);
    assert_eq!((line.whole, line.cut, line.take, line.spare_after), (0, 4, 4, 1));

    // Next week the spare goes in first, so only 3 are cut and nothing is left.
    m.spare_halves = line.spare_after;
    let line = PackLine::for_med(&m);
    assert_eq!((line.use_spare, line.cut, line.take, line.spare_after), (1, 3, 3, 0));
}

#[test]
fn one_and_a_half_tablets() {
    let line = PackLine::for_med(&med(1, "X", 1.5, 0.0));
    assert_eq!((line.whole, line.cut, line.take, line.spare_after), (7, 4, 11, 1));
}

#[test]
fn weekly_medicine_only_on_its_day() {
    // Calciferol once a week on Wednesday.
    let mut m = med(1, "Calciferol", 1.0, 0.0);
    m.days = 1 << 2;
    let line = PackLine::for_med(&m);
    assert_eq!(line.grid[0], [0, 0, 2, 0, 0, 0, 0]);
    assert_eq!(line.take, 1);
    assert_eq!(line.slot_days(Slot::Morning), vec![2]);
    assert!((daily_use(&m) - 1.0 / 7.0).abs() < 1e-9);
}

#[test]
fn paused_and_zero_dose_are_not_packed() {
    let mut data = Data::default();
    let mut lyrica = med(1, "Lyrica", 1.0, 0.0);
    lyrica.paused = true;
    data.meds = vec![lyrica, med(2, "Nothing", 0.0, 0.0), med(3, "Ecotrin", 1.0, 0.0)];
    let plan = week_plan(&data);
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].med_id, 3);
}

#[test]
fn compartments_count_pieces() {
    let mut weekly = med(3, "Weekly", 1.0, 0.0);
    weekly.days = 1 << 2;
    let lines: Vec<PackLine> = [med(1, "A", 1.0, 2.0), med(2, "Half", 0.5, 1.5), weekly]
        .iter()
        .map(PackLine::for_med)
        .collect();
    let c = compartment_pieces(&lines);
    assert_eq!(c[0], [2, 2, 3, 2, 2, 2, 2]);
    // 2 + (1 whole + 1 half) = 4 pieces each night.
    assert_eq!(c[1], [4; 7]);
}

#[test]
fn upcoming_monday_from_any_day() {
    assert_eq!(upcoming_monday(d(2026, 10, 19)), d(2026, 10, 19)); // Monday
    assert_eq!(upcoming_monday(d(2026, 10, 20)), d(2026, 10, 26)); // Tuesday
    assert_eq!(upcoming_monday(d(2026, 9, 13)), d(2026, 9, 14)); // Sunday
}

#[test]
fn next_week_skips_the_week_already_packed() {
    let mut data = Data::default();
    let today = d(2026, 10, 20);
    assert_eq!(next_week(&data, today), d(2026, 10, 26));
    data.meds = vec![med(1, "A", 1.0, 0.0)];
    start_packing(&mut data, d(2026, 10, 26));
    finish_packing(&mut data, today);
    assert_eq!(next_week(&data, today), d(2026, 11, 2));
    // Packing an earlier week afterwards doesn't pull the suggestion back.
    start_packing(&mut data, d(2026, 10, 19));
    finish_packing(&mut data, today);
    assert_eq!(next_week(&data, today), d(2026, 11, 2));
    // An old record doesn't hold the next week back.
    assert_eq!(next_week(&data, d(2026, 12, 1)), d(2026, 12, 7));
}

#[test]
fn stock_days_and_low_stock_ordering() {
    let mut m = med(1, "Euthyrox", 1.0, 0.0);
    assert_eq!(days_left(&m), None);
    assert_eq!(order_reason(&m, 14), None);

    m.stock = Some(10);
    assert_eq!(days_left(&m), Some(10.0));
    assert_eq!(order_reason(&m, 14), Some(OrderReason::LowStock));
    assert_eq!(short_for_week(&m), None);

    m.stock = Some(5);
    assert_eq!(short_for_week(&m), Some(2));

    m.stock = Some(30);
    assert_eq!(order_reason(&m, 14), None);

    m.order = OrderState::Needed;
    assert_eq!(order_reason(&m, 14), Some(OrderReason::Flagged));

    m.paused = true;
    assert_eq!(order_reason(&m, 14), None);
}

#[test]
fn ordering_then_receiving() {
    let mut m = med(1, "Folate", 1.0, 0.0);
    m.stock = Some(3);
    mark_ordered(&mut m, d(2026, 10, 20));
    assert_eq!(order_reason(&m, 14), None, "ordered items aren't needed again");
    receive(&mut m, 30);
    assert_eq!(m.stock, Some(33));
    assert_eq!(m.order, OrderState::None);
    assert_eq!(m.ordered_on, None);
}

#[test]
fn finishing_deducts_stock_and_records_history() {
    let mut data = Data::default();
    let mut a = med(1, "Cardicor", 0.5, 0.0);
    a.stock = Some(20);
    let mut b = med(2, "Rinvoq", 1.0, 0.0);
    b.stock = Some(3);
    let c = med(3, "Collagen", 3.0, 0.0);
    data.meds = vec![a, b, c];

    start_packing(&mut data, d(2026, 10, 26));
    set_packed(&mut data, 1, true);
    set_packed(&mut data, 2, true);
    set_packed(&mut data, 2, true); // idempotent
    let rec = finish_packing(&mut data, d(2026, 10, 25)).unwrap();

    assert_eq!(data.packing, None);
    assert_eq!(rec.packed, 2);
    assert_eq!(rec.tablets, 4 + 7);
    assert_eq!(rec.skipped, vec!["Collagen".to_string()]);
    assert_eq!(data.med(1).unwrap().stock, Some(16));
    assert_eq!(data.med(1).unwrap().spare_halves, 1);
    // Never goes negative.
    assert_eq!(data.med(2).unwrap().stock, Some(0));
    assert_eq!(data.history.len(), 1);
}

#[test]
fn start_packing_keeps_progress() {
    let mut data = Data::default();
    data.meds = vec![med(1, "A", 1.0, 0.0)];
    start_packing(&mut data, d(2026, 10, 26));
    set_packed(&mut data, 1, true);
    start_packing(&mut data, d(2026, 10, 26));
    assert_eq!(data.packing.as_ref().unwrap().packed, vec![1]);
    // A different week starts with nothing ticked.
    start_packing(&mut data, d(2026, 11, 2));
    let p = data.packing.as_ref().unwrap();
    assert_eq!(p.week_of, d(2026, 11, 2));
    assert!(p.packed.is_empty());
}

#[test]
fn removing_a_medicine_clears_it_from_packing() {
    let mut data = Data::default();
    data.meds = vec![med(1, "A", 1.0, 0.0)];
    start_packing(&mut data, d(2026, 10, 26));
    set_packed(&mut data, 1, true);
    data.remove(1);
    assert!(data.packing.unwrap().packed.is_empty());
}

#[test]
fn order_text_lists_names_and_strengths() {
    let mut data = Data::default();
    data.settings.person = "Shannon".into();
    let mut m = med(1, "Euthyrox", 1.0, 0.0);
    m.strength = "100ug".into();
    m.order = OrderState::Needed;
    data.meds = vec![m, med(2, "Other", 1.0, 0.0)];
    assert_eq!(order_text(&data), "Medicine to order for Shannon:\n• Euthyrox 100ug\n");
}

#[test]
fn json_round_trip_and_validation() {
    let mut data = Data::default();
    data.meds = vec![med(1, "A", 0.5, 2.0)];
    data.packing = Some(Packing {
        week_of: d(2026, 10, 26),
        packed: vec![1],
    });
    let back = Data::from_json(&data.to_json()).unwrap();
    assert_eq!(back, data);

    // Missing fields fall back to defaults.
    let minimal = Data::from_json(r#"{"meds":[{"id":1,"name":"A","morning":1}]}"#).unwrap();
    assert_eq!(minimal.meds[0].days, EVERY_DAY);
    assert_eq!(minimal.settings.low_days, 14);

    assert!(Data::from_json(r#"{"meds":[{"id":1,"name":"A","morning":0.3}]}"#).is_err());
    assert!(Data::from_json(r#"{"meds":[{"id":1,"name":" "}]}"#).is_err());
    assert!(Data::from_json(r#"{"meds":[{"id":1,"name":"A"},{"id":1,"name":"B"}]}"#).is_err());
    assert!(Data::from_json(r#"{"version":99,"meds":[]}"#).is_err());
    assert!(Data::from_json("{}").is_err(), "an unrelated JSON file isn't a backup");
    assert!(Data::from_json(r#"{"meds":{}}"#).is_err());
    assert!(Data::from_json("not json").is_err());
}

#[test]
fn halves_rounds_to_nearest_half() {
    assert_eq!(halves(0.5), 1);
    assert_eq!(halves(0.74), 1);
    assert_eq!(halves(0.76), 2);
    assert_eq!(halves(-1.0), 0);
    assert_eq!(halves(f64::NAN), 0);
}

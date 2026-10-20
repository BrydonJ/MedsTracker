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
    set_packed(&mut data, 1, Slot::Morning, true);
    set_packed(&mut data, 2, Slot::Morning, true);
    set_packed(&mut data, 2, Slot::Morning, true); // idempotent
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
    set_packed(&mut data, 1, Slot::Morning, true);
    start_packing(&mut data, d(2026, 10, 26));
    assert_eq!(data.packing.as_ref().unwrap().done, vec![(1, Slot::Morning)]);
    // A different week starts with nothing ticked.
    start_packing(&mut data, d(2026, 11, 2));
    let p = data.packing.as_ref().unwrap();
    assert_eq!(p.week_of, d(2026, 11, 2));
    assert!(p.done.is_empty());
}

#[test]
fn removing_a_medicine_clears_it_from_packing() {
    let mut data = Data::default();
    data.meds = vec![med(1, "A", 1.0, 0.0)];
    start_packing(&mut data, d(2026, 10, 26));
    set_packed(&mut data, 1, Slot::Night, true);
    data.remove(1);
    assert!(data.packing.unwrap().done.is_empty());
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
        done: vec![(1, Slot::Night)],
        days: vec![(1, Slot::Morning, 0b101)],
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

#[test]
fn steps_pack_the_morning_row_then_the_night_row() {
    let mut data = Data::default();
    let mut paused = med(4, "Lyrica", 1.0, 1.0);
    paused.paused = true;
    data.meds = vec![
        med(1, "Zinc", 1.0, 1.0),
        med(2, "Aspen", 0.0, 2.0),
        med(3, "Bisop", 0.5, 0.0),
        paused,
    ];
    let order: Vec<(u32, Slot)> = pack_steps(&data).iter().map(|s| (s.med_id, s.slot)).collect();
    assert_eq!(
        order,
        vec![(3, Slot::Morning), (1, Slot::Morning), (2, Slot::Night), (1, Slot::Night)]
    );
}

#[test]
fn a_half_cut_in_the_morning_goes_in_at_night() {
    // ½ morning and ½ night: cut 4 for the morning, the spare half starts the night row.
    let m = med(1, "Cardicor", 0.5, 0.5);
    let am = pack_step(&m, [m.days; 2], Slot::Morning, false).line;
    let pm = pack_step(&m, [m.days; 2], Slot::Night, true).line;
    assert_eq!((am.cut, am.spare_after), (4, 1));
    assert_eq!((pm.use_spare, pm.cut, pm.spare_after), (1, 3, 0));
    assert_eq!(am.grid[1], [0; 7]);
    assert_eq!(pm.grid[0], [0; 7]);
    // The two steps add up to packing the medicine in one go.
    for (am_dose, pm_dose, spare) in [(0.5, 0.5, 0), (1.5, 0.5, 1), (0.5, 1.5, 1), (1.5, 1.5, 0), (2.0, 0.5, 1)] {
        let mut m = med(1, "X", am_dose, pm_dose);
        m.spare_halves = spare;
        let (am, pm) = (pack_step(&m, [m.days; 2], Slot::Morning, false).line, pack_step(&m, [m.days; 2], Slot::Night, true).line);
        let all = PackLine::for_med(&m);
        assert_eq!(am.take + pm.take, all.take, "{am_dose}/{pm_dose}/{spare}");
        assert_eq!(pm.spare_after, all.spare_after, "{am_dose}/{pm_dose}/{spare}");
    }
}

#[test]
fn finishing_with_one_row_packed_takes_only_that_row() {
    let mut data = Data::default();
    let mut a = med(1, "Essentiale", 1.0, 2.0);
    a.stock = Some(40);
    data.meds = vec![a];
    start_packing(&mut data, d(2026, 10, 26));
    set_packed(&mut data, 1, Slot::Morning, true);
    let rec = finish_packing(&mut data, d(2026, 10, 25)).unwrap();
    assert_eq!(rec.packed, 1);
    assert_eq!(rec.tablets, 7);
    assert_eq!(rec.skipped, vec!["Essentiale (night)".to_string()]);
    assert_eq!(data.med(1).unwrap().stock, Some(33));
}

#[test]
fn old_saves_with_whole_medicine_ticks_still_load() {
    let json = r#"{"meds":[{"id":1,"name":"A","morning":1,"night":1}],
        "packing":{"week_of":"2026-10-26","packed":[1]}}"#;
    let data = Data::from_json(json).unwrap();
    let p = data.packing.unwrap();
    assert!(p.is_done(1, Slot::Morning) && p.is_done(1, Slot::Night));
}

#[test]
fn night_first_only_counts_on_spares_already_there() {
    let mut data = Data::default();
    let mut m = med(1, "Cardicor", 0.5, 0.5);
    m.stock = Some(20);
    data.meds = vec![m];
    start_packing(&mut data, d(2026, 10, 26));
    // No spare half exists until the morning row is cut, so night cuts its own.
    let night = pack_steps(&data).into_iter().find(|s| s.slot == Slot::Night).unwrap();
    assert_eq!((night.line.use_spare, night.line.cut, night.line.take), (0, 4, 4));
    // Finishing with only the night row agrees with what the card said.
    set_packed(&mut data, 1, Slot::Night, true);
    finish_packing(&mut data, d(2026, 10, 25));
    assert_eq!(data.med(1).unwrap().stock, Some(16));
    assert_eq!(data.med(1).unwrap().spare_halves, 1);
}

#[test]
fn night_uses_the_morning_half_once_the_morning_is_in() {
    let mut data = Data::default();
    data.meds = vec![med(1, "Cardicor", 0.5, 0.5)];
    start_packing(&mut data, d(2026, 10, 26));
    set_packed(&mut data, 1, Slot::Morning, true);
    let night = pack_steps(&data).into_iter().find(|s| s.slot == Slot::Night).unwrap();
    assert_eq!((night.line.use_spare, night.line.cut), (1, 3));
}

#[test]
fn night_then_morning_adds_up_like_packing_both_at_once() {
    let mut data = Data::default();
    let mut m = med(1, "Cardicor", 0.5, 0.5);
    m.stock = Some(20);
    data.meds = vec![m];
    start_packing(&mut data, d(2026, 10, 26));
    let step = |data: &Data, slot| pack_steps(data).into_iter().find(|s| s.slot == slot).unwrap().line;
    let night = step(&data, Slot::Night);
    set_packed(&mut data, 1, Slot::Night, true);
    // The half left over from the night row starts the morning row.
    let morning = step(&data, Slot::Morning);
    assert_eq!((morning.use_spare, morning.cut), (1, 3));
    assert_eq!(night.take + morning.take, 7);
    set_packed(&mut data, 1, Slot::Morning, true);
    finish_packing(&mut data, d(2026, 10, 25));
    assert_eq!(data.med(1).unwrap().stock, Some(13));
    assert_eq!(data.med(1).unwrap().spare_halves, morning.spare_after);
    assert_eq!(morning.spare_after, 0);
}

fn packing_with(meds: Vec<Medicine>) -> Data {
    let mut data = Data::default();
    data.meds = meds;
    start_packing(&mut data, d(2026, 10, 26));
    data
}

const MON_TO_THU: u8 = 0b000_1111;

#[test]
fn chosen_days_only_fill_those_compartments() {
    let mut data = packing_with(vec![med(1, "Cipalat", 1.0, 1.0)]);
    choose_days(&mut data, 1, Slot::Morning, MON_TO_THU);
    // Neither row is packed yet, so both follow.
    assert_eq!(data.week_days(data.med(1).unwrap()), [MON_TO_THU; 2]);
    let plan = week_plan(&data);
    assert_eq!(plan[0].grid[0], [2, 2, 2, 2, 0, 0, 0]);
    assert_eq!(plan[0].take, 8);
    assert_eq!(week_need(&data, data.med(1).unwrap()), 8);
}

#[test]
fn choosing_days_leaves_a_packed_row_alone() {
    let mut data = packing_with(vec![med(1, "Cipalat", 1.0, 1.0)]);
    set_packed(&mut data, 1, Slot::Morning, true);
    choose_days(&mut data, 1, Slot::Night, MON_TO_THU);
    assert_eq!(data.week_days(data.med(1).unwrap()), [EVERY_DAY, MON_TO_THU]);
    // Back to every day forgets the choice.
    choose_days(&mut data, 1, Slot::Night, EVERY_DAY);
    assert!(data.packing.as_ref().unwrap().days.is_empty());
}

#[test]
fn a_row_with_no_days_keeps_its_step() {
    let mut data = packing_with(vec![med(1, "Cipalat", 1.0, 0.0)]);
    choose_days(&mut data, 1, Slot::Morning, 0);
    let steps = pack_steps(&data);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].line.total_halves, 0);
}

#[test]
fn fit_to_stock_picks_the_earliest_days_the_box_covers() {
    let mut m = med(1, "Cardicor", 0.5, 0.5);
    m.stock = Some(3);
    m.spare_halves = 1;
    let data = packing_with(vec![m]);
    let m = data.med(1).unwrap();
    // 3 tablets plus a spare half is 7 halves: three days of a half morning and night.
    assert_eq!(fit_to_stock(&data, m, Slot::Morning), 0b000_0111);
}

#[test]
fn fit_to_stock_counts_what_a_packed_row_already_took() {
    let mut m = med(1, "Cipalat", 1.0, 1.0);
    m.stock = Some(10);
    let mut data = packing_with(vec![m]);
    set_packed(&mut data, 1, Slot::Morning, true);
    let m = data.med(1).unwrap().clone();
    assert_eq!(fit_to_stock(&data, &m, Slot::Night), 0b000_0111);
    choose_days(&mut data, 1, Slot::Night, 0b000_0111);
    assert_eq!(week_need(&data, &m), 10);
}

#[test]
fn fit_to_stock_only_offers_the_medicines_own_days() {
    let mut m = med(1, "Vitamin D", 1.0, 0.0);
    m.days = 0b010_0101; // Mon, Wed, Sat
    m.stock = Some(2);
    let data = packing_with(vec![m]);
    assert_eq!(fit_to_stock(&data, data.med(1).unwrap(), Slot::Morning), 0b000_0101);
}

#[test]
fn finishing_takes_only_the_chosen_days_from_stock() {
    let mut m = med(1, "Cipalat", 1.0, 1.0);
    m.stock = Some(9);
    let mut data = packing_with(vec![m]);
    choose_days(&mut data, 1, Slot::Morning, MON_TO_THU);
    set_packed(&mut data, 1, Slot::Morning, true);
    set_packed(&mut data, 1, Slot::Night, true);
    let rec = finish_packing(&mut data, d(2026, 10, 25)).unwrap();
    assert_eq!(data.med(1).unwrap().stock, Some(1));
    assert_eq!(rec.tablets, 8);
    assert_eq!(rec.packed, 1);
    assert_eq!(rec.skipped, vec!["Cipalat (Fri, Sat, Sun)".to_string()]);
}

#[test]
fn finishing_names_the_missing_days_per_row() {
    let mut data = packing_with(vec![med(1, "Cipalat", 1.0, 1.0)]);
    set_packed(&mut data, 1, Slot::Morning, true);
    choose_days(&mut data, 1, Slot::Night, 0b001_1111);
    set_packed(&mut data, 1, Slot::Night, true);
    let rec = finish_packing(&mut data, d(2026, 10, 25)).unwrap();
    assert_eq!(rec.skipped, vec!["Cipalat (night: Sat, Sun)".to_string()]);
}

#[test]
fn a_packed_row_with_no_days_counts_as_skipped() {
    let mut m = med(1, "Cipalat", 1.0, 0.0);
    m.stock = Some(5);
    let mut data = packing_with(vec![m]);
    choose_days(&mut data, 1, Slot::Morning, 0);
    set_packed(&mut data, 1, Slot::Morning, true);
    let rec = finish_packing(&mut data, d(2026, 10, 25)).unwrap();
    assert_eq!((rec.packed, rec.tablets), (0, 0));
    assert_eq!(rec.skipped, vec!["Cipalat".to_string()]);
    assert_eq!(data.med(1).unwrap().stock, Some(5));
}

#[test]
fn rows_on_different_days_add_up_in_either_order() {
    let mut m = med(1, "Cardicor", 0.5, 0.5);
    m.stock = Some(20);
    let both = |first: Slot| {
        let mut data = packing_with(vec![m.clone()]);
        // Morning every day, night Mon to Thu.
        data.packing.as_mut().unwrap().days = vec![(1, Slot::Night, MON_TO_THU)];
        let step = |data: &Data, slot| pack_steps(data).into_iter().find(|s| s.slot == slot).unwrap().line;
        let a = step(&data, first);
        set_packed(&mut data, 1, first, true);
        let b = step(&data, first.other());
        set_packed(&mut data, 1, first.other(), true);
        finish_packing(&mut data, d(2026, 10, 25));
        (a.take + b.take, b.spare_after, data.med(1).unwrap().stock, data.med(1).unwrap().spare_halves)
    };
    let whole = PackLine::for_rows(&m, [EVERY_DAY, MON_TO_THU], &Slot::ALL);
    let expected = (whole.take, whole.spare_after, Some(20 - whole.take), whole.spare_after);
    assert_eq!(both(Slot::Morning), expected);
    assert_eq!(both(Slot::Night), expected);
}

#[test]
fn chosen_days_survive_a_reload_and_clear_for_a_new_week() {
    let mut data = packing_with(vec![med(1, "Cipalat", 1.0, 0.0)]);
    choose_days(&mut data, 1, Slot::Morning, MON_TO_THU);
    let mut back = Data::from_json(&data.to_json()).unwrap();
    assert_eq!(back.week_days(back.med(1).unwrap())[0], MON_TO_THU);
    start_packing(&mut back, d(2026, 11, 2));
    assert_eq!(back.week_days(back.med(1).unwrap())[0], EVERY_DAY);
}

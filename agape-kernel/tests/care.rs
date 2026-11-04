//! When things go wrong: late orders flagged while still waiting, refunds paid by whoever was at
//! fault, limits that send repeat claims to a person, and the venue's emergency stop.

use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night, YOU};
use agape_kernel::value::{acct, Unit};
use agape_kernel::world::WorldDef;
use serde_json::Value;

fn night() -> Night {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    Night::new(w, t, Config { you_seat: Some("s104".into()), ..Config::default() })
}
fn until(n: &mut Night, min: i64) { while n.t < min * 60_000 && n.step() {} }
fn card(n: &Night) -> i64 { -n.ledger.balance(&acct(YOU, "card"), &Unit::usd()) }
fn order(n: &mut Night, id: usize) -> Value {
    let v = n.view_fan(YOU);
    v["orders"].as_array().unwrap().iter().chain(v["me"]["receipts"].as_array().unwrap()).find(|o| o["id"] == id).cloned().unwrap_or(Value::Null)
}
fn balanced(n: &Night) -> bool { n.ledger.units().iter().all(|u| n.ledger.total(u) == 0) }
/// Order something and wait until it's delivered and settled.
fn delivered(n: &mut Night, thing: &str) -> usize {
    let id: usize = n.you_order(thing, 1, true, 0).unwrap().parse().unwrap();
    for _ in 0..600 { n.step(); if order(n, id)["stage"] == "Settled" { return id; } }
    panic!("never delivered");
}

#[test]
fn a_late_order_says_so_while_you_wait_and_the_slow_kitchen_pays_the_credit() {
    let mut n = night();
    until(&mut n, 50);
    let stand = n.world.thing("burger").unwrap().stand.clone();
    n.clock_in(&stand).unwrap(); // a kitchen run by people who, tonight, never mark anything ready
    let id: usize = n.you_order("burger", 1, true, 0).unwrap().parse().unwrap();
    let before = n.ledger.balance(&acct(&stand, "revenue"), &Unit::usd());
    for _ in 0..200 { n.step(); if order(&mut n, id)["late"] == true { break; } }
    let o = order(&mut n, id);
    assert_eq!(o["late"], true, "flagged while still waiting");
    assert_eq!(o["late_credit"], 300, "with the credit promised");
    assert_ne!(o["stage"], "Delivered");
    // the kitchen catches up; it gets there; the credit comes out of the kitchen's revenue
    n.live.remove(&stand);
    for _ in 0..600 { n.step(); if order(&mut n, id)["stage"] == "Settled" { break; } }
    let charge = o["charge"].as_i64().unwrap();
    assert!(n.ledger.txns.iter().any(|t| t.memo == "settle+late_credit" && t.postings.iter().any(|p| p.account == acct(&stand, "revenue") && p.amount == -300)), "the stand that was slow pays it");
    let _ = (before, charge);
    assert!(balanced(&n));
}

fn credit(n: &Night) -> i64 { n.ledger.balance(&acct(YOU, "credit"), &Unit::usd()) }
fn rev(n: &Night, who: &str) -> i64 { n.ledger.balance(&acct(who, "revenue"), &Unit::usd()) }

#[test]
fn a_wrong_order_needs_a_photo_and_is_made_right_as_credit_by_the_stand_that_got_it_wrong() {
    let mut n = night();
    until(&mut n, 50);
    let id = delivered(&mut n, "burger");
    let o = order(&mut n, id);
    assert_eq!(o["can_report"], true);
    let (paid, stand) = (card(&n), n.world.thing("burger").unwrap().stand.clone());
    let (stand_before, agape_before) = (rev(&n, &stand), rev(&n, "agape"));
    assert!(n.report_problem(YOU, id, "nope", "").is_err());
    assert_eq!(n.report_with("fan9", id, "wrong_item", "", true), Err("That isn't your order".into()));
    assert!(n.report_with(YOU, id, "wrong_item", "", false).unwrap_err().contains("photo"), "a wrong order needs a photo");
    n.report_with(YOU, id, "wrong_item", "Got a cheeseburger, ordered a burger", true).unwrap();
    let b = &o["breakdown"];
    let (items, tax, fee) = (b["subtotal"].as_i64().unwrap() - b["discount"].as_i64().unwrap(), b["tax"].as_i64().unwrap(), b["fee"].as_i64().unwrap());
    assert_eq!(card(&n), paid, "credit, not a refund, by default");
    assert_eq!(credit(&n), items + tax + fee, "the food, its tax and the delivery fee, as Agape credit");
    assert_eq!(stand_before - rev(&n, &stand), items + fee, "the stand that got it wrong pays for the food and the wasted delivery");
    assert_eq!(rev(&n, "agape"), agape_before, "Agape pays nothing for a stand's mistake");
    assert_eq!(order(&mut n, id)["problem"]["state"]["state"], "remedied");
    assert_eq!(n.report_problem(YOU, id, "quality", ""), Err("You've already reported this one".into()));
    assert!(balanced(&n));
}

#[test]
fn not_to_your_taste_is_the_stands_to_make_right_and_agape_keeps_its_fee() {
    let mut n = night();
    until(&mut n, 50);
    let id = delivered(&mut n, "burger");
    let stand = n.world.thing("burger").unwrap().stand.clone();
    let (s0, a0, t0) = (rev(&n, &stand), rev(&n, "agape"), n.ledger.balance(&acct("tax_authority", "payable"), &Unit::usd()));
    n.report_problem(YOU, id, "quality", "Cold").unwrap();
    let items = order(&mut n, id)["breakdown"]["subtotal"].as_i64().unwrap();
    assert_eq!(credit(&n), items);
    assert_eq!(s0 - rev(&n, &stand), items);
    assert_eq!(rev(&n, "agape"), a0, "delivery wasn't the problem");
    assert_eq!(n.ledger.balance(&acct("tax_authority", "payable"), &Unit::usd()), t0, "a goodwill credit doesn't unwind the sale's tax");
    assert!(balanced(&n));
}

#[test]
fn credit_goes_toward_the_next_order_before_the_card() {
    let mut n = night();
    until(&mut n, 50);
    let id = delivered(&mut n, "burger");
    n.report_problem(YOU, id, "quality", "").unwrap();
    let have = credit(&n);
    let before = card(&n);
    let next = delivered(&mut n, "nachos");
    let charge = order(&mut n, next)["charge"].as_i64().unwrap();
    let used = have.min(charge);
    assert_eq!(order(&mut n, next)["credit_used"], used);
    assert_eq!(card(&n) - before, charge - used, "only the rest reaches the card");
    assert_eq!(credit(&n), have - used);
    assert!(balanced(&n));
}

#[test]
fn agapes_team_can_switch_to_refunds_and_say_who_pays_for_any_part() {
    let mut n = night();
    until(&mut n, 50);
    let mut p = n.policy.clone();
    p.form = agape_kernel::remedy::Form::Refund;
    // a ruined delivery, paid in full by the runner who carried it, tip included
    p.rules.get_mut("damaged").unwrap().covers = vec![
        agape_kernel::remedy::Cover { part: agape_kernel::remedy::Part::Items, from: agape_kernel::remedy::Source::Runner, bps: 10_000 },
        agape_kernel::remedy::Cover { part: agape_kernel::remedy::Part::Tip, from: agape_kernel::remedy::Source::Runner, bps: 10_000 }];
    n.set_policy(p.clone()).unwrap();
    let mut bad = p.clone();
    bad.rules.get_mut("damaged").unwrap().covers.push(agape_kernel::remedy::Cover { part: agape_kernel::remedy::Part::Tip, from: agape_kernel::remedy::Source::Agape, bps: 1 });
    assert!(n.set_policy(bad).is_err(), "never more than all of a part");
    let id: usize = n.you_order("burger", 1, true, 200).unwrap().parse().unwrap();
    for _ in 0..600 { n.step(); if order(&mut n, id)["stage"] == "Settled" { break; } }
    let o = order(&mut n, id);
    let runner = o["runner"].as_str().unwrap().to_string();
    let (pay0, tips0, paid) = (n.ledger.balance(&acct(&runner, "pay"), &Unit::usd()), n.ledger.balance(&acct(&runner, "tips"), &Unit::usd()), card(&n));
    n.report_with(YOU, id, "damaged", "", true).unwrap();
    let items = o["breakdown"]["subtotal"].as_i64().unwrap();
    assert_eq!(paid - card(&n), items + 200, "back to the card");
    assert_eq!(pay0 - n.ledger.balance(&acct(&runner, "pay"), &Unit::usd()), items);
    assert_eq!(tips0 - n.ledger.balance(&acct(&runner, "tips"), &Unit::usd()), 200);
    assert!(balanced(&n));
}

#[test]
fn never_arrived_is_held_for_a_person_when_the_runners_phone_was_at_the_seat() {
    let mut n = night();
    until(&mut n, 50);
    let id = delivered(&mut n, "burger");
    let p = n.report_problem(YOU, id, "never_arrived", "").unwrap();
    let pr = n.problems.iter().find(|x| x.id == p).unwrap();
    assert_eq!(pr.state, agape_kernel::care::ProblemState::Open);
    assert!(pr.held.as_deref().unwrap().contains("of the seat"));
    assert_eq!(n.view_care()["problems"][0]["handoff_m"].as_i64().map(|m| m < 40), Some(true), "Agape's team sees the evidence");
    assert!(n.view_fan(YOU).to_string().find("handoff_m").is_none(), "the fan never does");
}

#[test]
fn repeat_claims_go_to_a_person_and_records_call_for_proof() {
    let mut n = night();
    until(&mut n, 50);
    let ids: Vec<usize> = (0..5).map(|_| delivered(&mut n, "nachos")).collect();
    let p1 = n.report_with(YOU, ids[0], "damaged", "", true).unwrap();
    let p2 = n.report_problem(YOU, ids[1], "quality", "").unwrap();
    let p3 = n.report_problem(YOU, ids[2], "quality", "").unwrap();
    let state = |n: &Night, p: &str| n.problems.iter().find(|x| x.id == p).unwrap().state.clone();
    assert!(matches!(state(&n, &p1), agape_kernel::care::ProblemState::Remedied { .. }));
    assert!(matches!(state(&n, &p2), agape_kernel::care::ProblemState::Remedied { .. }));
    assert_eq!(state(&n, &p3), agape_kernel::care::ProblemState::Open, "a third in one night waits for a person");
    n.resolve_problem("hq", &p3, false, "Photos show it was fine").unwrap();
    assert_eq!(n.resolve_problem("hq", &p3, true, ""), Err("Already decided".into()));
    // three claims on five orders: past the PIN line
    let r = n.fan_record(YOU);
    assert_eq!((r.orders, r.claims), (5, 3));
    assert_eq!(r.proof, agape_kernel::remedy::Proof::Pin);
    assert!(r.cost > 0, "the ruined delivery came out of Agape's pocket");
    let next: usize = n.you_order("burger", 1, true, 0).unwrap().parse().unwrap();
    assert_eq!(n.view_fan(YOU)["orders"].as_array().unwrap().iter().find(|o| o["id"] == next).unwrap()["pin"].as_str().map(|p| p.len()), Some(4), "the fan sees the PIN to give their runner");
    n.set_proof(YOU, Some(agape_kernel::remedy::Proof::None));
    assert_eq!(n.proof_of(next), agape_kernel::remedy::Proof::None, "Agape's team can override it");
    let late = delivered(&mut n, "nachos");
    for _ in 0..200 { n.step(); }
    assert!(n.report_problem(YOU, late, "quality", "").unwrap_err().contains("30 minutes"));
    assert!(balanced(&n));
}

#[test]
fn the_venue_can_stop_everything_and_nobody_is_charged_for_what_wasnt_started() {
    let mut n = night();
    until(&mut n, 80); // halftime rush: long queues
    let before = card(&n);
    let id: usize = n.you_order("burger", 1, true, 0).unwrap().parse().unwrap();
    assert_eq!(n.halt(""), Err("Say why, so fans know".into()));
    n.halt("Severe weather. Please stay in your seats.").unwrap();
    let o = order(&mut n, id);
    assert!(o["stage"].as_str().unwrap().contains("Halted"), "{}", o["stage"]);
    assert_eq!(card(&n), before, "not charged");
    assert!(n.you_order("nachos", 1, true, 0).unwrap_err().starts_with("Paused: Severe weather"));
    assert!(n.buy_tickets(YOU, "e2", "s104", 1).unwrap_err().starts_with("Paused"));
    assert_eq!(n.view_fan(YOU)["halt"]["reason"], "Severe weather. Please stay in your seats.");
    for _ in 0..30 { n.step(); }
    n.resume().unwrap();
    assert!(n.you_order("nachos", 1, true, 0).is_ok(), "and back to normal");
    while n.step() {}
    assert!(balanced(&n));
}

#[test]
fn a_fan_rates_who_brought_it_once_and_can_order_the_same_again_in_one_tap() {
    let mut n = night();
    until(&mut n, 50);
    let id = delivered(&mut n, "burger");
    let o = order(&mut n, id);
    let runner = o["runner"].as_str().unwrap().to_string();
    let stand = o["stand_id"].as_str().unwrap().to_string();
    assert_eq!(n.rate(YOU, id, "runner99", 5), Err("Rate the stand or runner on this order".into()));
    assert_eq!(n.rate(YOU, id, &runner, 7), Err("One to five stars".into()));
    n.rate(YOU, id, &runner, 5).unwrap();
    n.rate(YOU, id, &stand, 4).unwrap();
    assert_eq!(n.rate(YOU, id, &runner, 1), Err("Already rated".into()));
    assert_eq!(n.view_runner(&runner)["rating"], 5.0);
    assert_eq!(order(&mut n, id)["rated"].as_array().unwrap().len(), 2);
    let again: usize = n.reorder(YOU, id).unwrap().parse().unwrap();
    let a = order(&mut n, again);
    assert_eq!((a["items"][0]["thing"].as_str(), a["delivery"].as_bool()), (Some("burger"), Some(true)));
    assert!(n.log.signals.iter().any(|s| s.kind == "social" && s.fields["act"].key() == "rate"), "ratings are signals like everything else");
}

#[test]
fn agape_can_pay_runners_by_the_hour_and_make_delivery_free_over_an_amount() {
    let mut n = night();
    until(&mut n, 40);
    assert!(n.set_pay("weekly", None, None, None, None).is_err());
    n.set_pay("hourly", Some(1800), None, None, Some(2000)).unwrap();
    let r = "runner0";
    let t0 = n.t;
    while n.t < t0 + 3_600_000 { n.step(); }
    let earned: i64 = n.ledger.txns.iter().filter(|t| t.memo == "wages").flat_map(|t| t.postings.iter()).filter(|p| p.account == acct(r, "pay")).map(|p| p.amount).sum();
    assert_eq!(earned, 1800, "exactly the wage for an hour online, whatever the trips");
    // a burger is under $20: the fee applies; two of them are over: it doesn't
    let one = n.you_quote("burger", 1, true, 0, None).unwrap();
    let two = n.you_quote("burger", 2, true, 0, None).unwrap();
    assert_eq!(one["fee"], n.terms.delivery_fee);
    assert_eq!(two["fee"], 0);
    let id: usize = n.you_order("burger", 2, true, 0).unwrap().parse().unwrap();
    for _ in 0..600 { n.step(); if order(&mut n, id)["stage"] == "Settled" { break; } }
    assert_eq!(order(&mut n, id)["breakdown"]["fee"], 0);
    assert!(balanced(&n));
}

#[test]
fn a_stand_handing_over_pickups_by_hand_never_breaks_the_night() {
    let mut n = night();
    until(&mut n, 50);
    let stand = n.world.thing("burger").unwrap().stand.clone();
    n.clock_in(&stand).unwrap();
    // a run of pickup orders, handed over by the stand as soon as they're on the shelf
    for _ in 0..4 { let _ = n.you_order("burger", 1, false, 0); }
    for _ in 0..400 {
        n.step();
        let ready: Vec<usize> = n.view_stand(&stand)["pickups"].as_array().unwrap().iter().map(|o| o["id"].as_u64().unwrap() as usize).collect();
        for id in ready { let _ = n.handed_over(&stand, id); }
        let making: Vec<usize> = n.view_stand(&stand)["stations"].as_array().unwrap().iter().filter_map(|x| x["order"]["id"].as_u64()).map(|x| x as usize).collect();
        for id in making { let _ = n.mark_ready(&stand, id); }
    }
    assert!(balanced(&n));
}

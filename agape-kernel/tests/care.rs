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

#[test]
fn a_problem_is_refunded_by_whoever_was_at_fault_and_the_books_still_balance() {
    let mut n = night();
    until(&mut n, 50);
    let id = delivered(&mut n, "burger");
    let o = order(&mut n, id);
    assert_eq!(o["can_report"], true);
    let paid = card(&n);
    let stand = n.world.thing("burger").unwrap().stand.clone();
    let stand_before = n.ledger.balance(&acct(&stand, "revenue"), &Unit::usd());
    assert!(n.report_problem(YOU, id, "nope", "").is_err());
    assert_eq!(n.report_problem("fan9", id, "wrong_item", ""), Err("That isn't your order".into()));
    n.report_problem(YOU, id, "wrong_item", "Got a cheeseburger, ordered a burger").unwrap();
    let b = &o["breakdown"];
    let back = b["subtotal"].as_i64().unwrap() - b["discount"].as_i64().unwrap() + b["tax"].as_i64().unwrap();
    assert_eq!(paid - card(&n), back, "the food and its tax come back; the delivery happened, so its fee doesn't");
    assert!(n.ledger.balance(&acct(&stand, "revenue"), &Unit::usd()) < stand_before, "the stand that made it wrong pays");
    assert_eq!(order(&mut n, id)["problem"]["state"]["state"], "refunded");
    assert_eq!(n.report_problem(YOU, id, "quality", ""), Err("You've already reported this one".into()));
    assert!(balanced(&n));
}

#[test]
fn repeat_claims_go_to_a_person_and_old_orders_cant_be_reported() {
    let mut n = night();
    until(&mut n, 50);
    let ids: Vec<usize> = (0..3).map(|_| delivered(&mut n, "nachos")).collect();
    let p1 = n.report_problem(YOU, ids[0], "never_arrived", "").unwrap();
    let p2 = n.report_problem(YOU, ids[1], "quality", "").unwrap();
    let p3 = n.report_problem(YOU, ids[2], "quality", "").unwrap();
    let state = |n: &Night, p: &str| n.problems.iter().find(|x| x.id == p).unwrap().state.clone();
    assert!(matches!(state(&n, &p1), agape_kernel::care::ProblemState::Refunded { .. }));
    assert!(matches!(state(&n, &p2), agape_kernel::care::ProblemState::Refunded { .. }));
    assert_eq!(state(&n, &p3), agape_kernel::care::ProblemState::Open, "a third in one night waits for operations");
    assert!(n.view_ops()["problems"].as_array().unwrap().iter().any(|x| x["id"] == p3.as_str() && x["state"]["state"] == "open"));
    n.resolve_problem("ops", &p3, false, "Photos show it was fine").unwrap();
    assert_eq!(n.resolve_problem("ops", &p3, true, ""), Err("Already decided".into()));
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

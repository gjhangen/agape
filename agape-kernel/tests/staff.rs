//! Stands and runners run by people: the kitchen marks food ready, a runner takes a trip they
//! were shown first, picks up, and hands over at the seat. The simulation does none of it for them.

use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night, YOU};
use agape_kernel::value::{acct, Unit};
use agape_kernel::world::WorldDef;

fn night() -> Night {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    Night::new(w, t, Config { you_seat: Some("s104".into()), ..Config::default() })
}
fn until(n: &mut Night, min: i64) { while n.t < min * 60_000 && n.step() {} }
fn steps(n: &mut Night, k: usize) { for _ in 0..k { n.step(); } }
fn stage(n: &mut Night, id: usize) -> String {
    let v = n.view_fan(YOU);
    v["orders"].as_array().unwrap().iter().chain(v["me"]["receipts"].as_array().unwrap()).find(|o| o["id"] == id).map(|o| o["stage"].as_str().unwrap().to_string()).unwrap_or_default()
}
fn card(n: &Night) -> i64 { -n.ledger.balance(&acct(YOU, "card"), &Unit::usd()) }

/// The kitchen staff at work: everything else on a station is marked ready when it's done.
fn cook(n: &mut Night, stand: &str, except: usize) {
    let t = n.t;
    let done: Vec<usize> = n.view_stand(stand)["stations"].as_array().unwrap().iter()
        .filter(|s| s["until"].as_i64().unwrap() <= t).filter_map(|s| s["order"]["id"].as_u64()).map(|x| x as usize).filter(|&x| x != except).collect();
    for id in done { n.mark_ready(stand, id).unwrap(); }
}

/// The stand of the burger, so the test follows wherever the template puts it.
fn burger_stand(n: &Night) -> String { n.world.thing("burger").unwrap().stand.clone() }

#[test]
fn a_kitchen_run_by_people_marks_its_own_food_ready() {
    let mut n = night();
    until(&mut n, 50);
    let stand = burger_stand(&n);
    n.clock_in(&stand).unwrap();
    assert!(n.clock_in("you").is_err(), "fans don't clock in");
    let id: usize = n.you_order("burger", 1, true, 0).unwrap().parse().unwrap();
    let mut on_station = false;
    for _ in 0..360 { n.step(); cook(&mut n, &stand, id); if stage(&mut n, id) == "Preparing" { on_station = true; break; } }
    assert!(on_station, "it reached a station");
    for _ in 0..60 { n.step(); cook(&mut n, &stand, id); } // well past its prep time
    assert_eq!(stage(&mut n, id), "Preparing", "nobody marked it ready, so it isn't");
    let v = n.view_stand(&stand);
    assert!(v["stations"].as_array().unwrap().iter().any(|s| s["order"]["id"] == id && s["late"] == true), "the station shows it running over");
    assert!(n.mark_ready("grill-999", id).is_err());
    n.mark_ready(&stand, id).unwrap();
    assert_eq!(stage(&mut n, id), "Ready");
}

#[test]
fn a_kitchen_that_cannot_make_it_charges_nobody_and_says_so() {
    let mut n = night();
    until(&mut n, 50);
    let stand = burger_stand(&n);
    n.clock_in(&stand).unwrap();
    let before = card(&n);
    let id: usize = n.you_order("burger", 1, true, 0).unwrap().parse().unwrap();
    n.cannot_make(&stand, id).unwrap();
    assert_eq!(card(&n), before, "not charged");
    assert!(n.view_fan(YOU)["me"]["receipts"].as_array().unwrap().iter().any(|o| o["id"] == id && o["stage"].as_str().unwrap().contains("Cannot")), "and told");
    assert!(n.ledger.units().iter().all(|u| n.ledger.total(u) == 0));
}

#[test]
fn a_runner_sees_the_trip_first_takes_it_and_hands_it_over() {
    let mut n = night();
    until(&mut n, 50);
    let runners: Vec<String> = n.runners.iter().map(|r| r.id.clone()).collect();
    for r in &runners { n.clock_in(r).unwrap(); } // every runner tonight is a person
    let id: usize = n.you_order("burger", 1, true, 300).unwrap().parse().unwrap();
    let mut who = None;
    for _ in 0..400 { n.step(); if let Some((r, _)) = n.trips.iter().find(|(_, t)| t.orders.contains(&id)) { who = Some(r.clone()); break; } }
    let r = who.expect("the order was offered to a runner");
    let v = n.view_runner(&r);
    assert_eq!(v["trip"]["accepted"], false);
    assert!(v["trip"]["pay"].as_i64().unwrap() >= 150 + 300, "the pay, tip included, is shown before saying yes");
    assert_eq!(stage(&mut n, id), "Ready", "offered isn't assigned");
    // said no: it goes to someone else
    n.decline_trip(&r).unwrap();
    let mut next = None;
    for _ in 0..30 { n.step(); if let Some((x, _)) = n.trips.iter().find(|(_, t)| t.orders.contains(&id)) { next = Some(x.clone()); break; } }
    let r = next.expect("offered again");
    n.accept_trip(&r).unwrap();
    assert_eq!(stage(&mut n, id), "Assigned");
    assert_eq!(n.delivered(&r, id), Err("Pick it up first".into()));
    n.picked_up(&r).unwrap();
    assert_eq!(n.unreachable(&r, id), Err("Give them a few minutes first".into()));
    let (pay, tips) = (n.ledger.balance(&acct(&r, "pay"), &Unit::usd()), n.ledger.balance(&acct(&r, "tips"), &Unit::usd()));
    n.delivered(&r, id).unwrap();
    for other in n.trips.get(&r).map(|t| t.orders.clone()).unwrap_or_default() { let _ = n.delivered(&r, other); }
    steps(&mut n, 2);
    assert_eq!(stage(&mut n, id), "Settled");
    let others = n.ledger.balance(&acct(&r, "tips"), &Unit::usd()) - tips - 300;
    assert!(others >= 0, "the whole tip to the runner who brought it (plus the tips of anything else in the bag)");
    assert!(n.ledger.balance(&acct(&r, "pay"), &Unit::usd()) - pay >= 150);
    assert!(n.trips.get(&r).map_or(true, |t| !t.accepted && !t.orders.contains(&id)), "and they're free for the next one (maybe already offered it)");
}

#[test]
fn an_unanswered_trip_goes_back_on_the_shelf() {
    let mut n = night();
    until(&mut n, 50);
    let runners: Vec<String> = n.runners.iter().map(|r| r.id.clone()).collect();
    for r in &runners { n.clock_in(r).unwrap(); }
    let id: usize = n.you_order("burger", 1, true, 0).unwrap().parse().unwrap();
    let mut first = None;
    for _ in 0..400 { n.step(); if let Some((r, _)) = n.trips.iter().find(|(_, t)| t.orders.contains(&id)) { first = Some(r.clone()); break; } }
    let r = first.unwrap();
    steps(&mut n, 7); // more than a minute
    assert!(n.trips.get(&r).map_or(true, |t| !t.orders.contains(&id)), "not theirs any more");
}

#[test]
fn a_pickup_order_waits_at_the_counter_for_the_fan() {
    let mut n = night();
    until(&mut n, 50);
    let stand = burger_stand(&n);
    n.clock_in(&stand).unwrap();
    let id: usize = n.you_order("burger", 1, false, 0).unwrap().parse().unwrap();
    for _ in 0..360 { n.step(); cook(&mut n, &stand, id); if stage(&mut n, id) == "Preparing" { break; } }
    n.mark_ready(&stand, id).unwrap();
    steps(&mut n, 60);
    assert_eq!(stage(&mut n, id), "Ready", "nobody collected it for them");
    assert!(n.view_stand(&stand)["pickups"].as_array().unwrap().iter().any(|o| o["id"] == id));
    n.handed_over(&stand, id).unwrap();
    steps(&mut n, 2);
    assert_eq!(stage(&mut n, id), "Settled");
}

#[test]
fn a_whole_night_with_people_on_shift_still_balances() {
    let mut n = night();
    until(&mut n, 60);
    n.clock_in("runner3").unwrap();
    n.clock_in("runner4").unwrap();
    for _ in 0..90 {
        n.step();
        for r in ["runner3", "runner4"] {
            if let Some(t) = n.trips.get(r).cloned() {
                if !t.accepted { n.accept_trip(r).unwrap(); } else if !t.picked { n.picked_up(r).unwrap(); } else { for id in t.orders { let _ = n.delivered(r, id); } }
            }
        }
    }
    n.clock_out("runner3").unwrap_or(());
    let _ = n.clock_out("runner4");
    while n.step() {}
    assert!(n.ledger.units().iter().all(|u| n.ledger.total(u) == 0));
    assert!(n.ledger.balance(&acct("runner3", "pay"), &Unit::usd()) > 0, "the people on shift did trips");
}

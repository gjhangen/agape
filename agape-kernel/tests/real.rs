//! A real night: nobody is simulated. No crowd, no made-up game, no stand cooks and no runner walks
//! unless a person does it. The game moves when the venue's team marks it, and the books balance.

use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night};
use agape_kernel::value::{acct, Unit};
use agape_kernel::world::WorldDef;

fn night() -> Night {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    Night::new(w, t, Config { fans: 0, runners: 3, real: true, ..Config::default() })
}
fn steps(n: &mut Night, k: usize) { for _ in 0..k { n.step(); } }
fn stage(n: &Night, id: usize) -> String { n.view_night()["orders"].as_array().unwrap().iter().find(|o| o["id"] == id).map(|o| o["stage"].as_str().unwrap().to_string()).unwrap_or_default() }
fn count(n: &Night) -> usize { n.view_night()["orders"].as_array().unwrap().len() }
fn conserved(n: &Night) -> bool { n.ledger.units().iter().all(|u| n.ledger.total(u) == 0) }

#[test]
fn nothing_happens_by_itself() {
    let mut n = night();
    steps(&mut n, 6 * 120); // two hours
    assert_eq!(count(&n), 0, "no simulated orders");
    assert_eq!(n.score, (0, 0), "no made-up scoring");
    assert!(n.chat.is_empty(), "no simulated chat");
    assert_eq!(n.phase(), "pregame", "the game hasn't been marked as started");
    assert!(!n.done, "a real night doesn't end on a timer");
    assert!(conserved(&n));
}

#[test]
fn an_order_goes_from_a_real_fan_to_a_real_stand_and_runner() {
    let mut n = night();
    n.join_named("u1", "s104", "Gabe", true).unwrap();
    let burger_stand = n.world.thing("burger").unwrap().stand.clone();
    assert_eq!(n.order_as("u1", "burger", 1, true, 300, None), Err("StandClosed".into()), "nobody is at the grill yet");
    n.clock_in(&burger_stand).unwrap();
    let id: usize = n.order_as("u1", "burger", 1, true, 300, None).unwrap().parse().unwrap();
    // marked ready straight from the line, before a station took it, is fine too
    let quick: usize = n.order_as("u1", "burger", 1, false, 0, None).unwrap().parse().unwrap();
    n.mark_ready(&burger_stand, quick).unwrap();
    assert_eq!(stage(&n, quick), "Ready");
    n.handed_over(&burger_stand, quick).unwrap();
    // the stand's food is never finished for it, however long we wait
    steps(&mut n, 60);
    assert_eq!(stage(&n, id), "Preparing");
    n.mark_ready(&burger_stand, id).unwrap();
    // no runner on shift: it waits on the shelf, nobody delivers it
    steps(&mut n, 30);
    assert_eq!(stage(&n, id), "Ready");
    assert!(n.trips.is_empty());
    n.clock_in("runner1").unwrap();
    steps(&mut n, 3);
    let trip = n.trips.get("runner1").expect("offered to the runner on shift").clone();
    assert!(trip.orders.contains(&id));
    n.accept_trip("runner1").unwrap();
    n.picked_up("runner1").unwrap();
    n.delivered("runner1", id).unwrap();
    steps(&mut n, 2);
    assert_eq!(stage(&n, id), "Settled");
    assert_eq!(n.ledger.balance(&acct("runner1", "tips"), &Unit::usd()), 300, "the whole tip to the runner");
    let v = n.view_night();
    assert_eq!(v["books"]["conserved"], true);
    assert_eq!(v["books"]["totals"]["n_settled"], 2);
    assert!(v["books"]["fans_paid"].as_i64().unwrap() > 0);
    assert_eq!(v["orders"].as_array().unwrap().iter().find(|o| o["id"] == id).unwrap()["runner"], "runner1");
    assert!(conserved(&n));
}

#[test]
fn the_venue_marks_the_game_and_keeps_score() {
    let mut n = night();
    assert!(n.mark_game("end").is_err(), "nothing to end yet");
    n.mark_game("start").unwrap();
    assert_eq!(n.phase(), "in play");
    assert_eq!(n.view_night()["clock"]["label"], "Q1");
    n.keep_score("home", 3, "").unwrap();
    n.keep_score("away", 2, "").unwrap();
    assert!(n.keep_score("away", -5, "").is_err(), "a score can't go below zero");
    n.keep_score("away", -2, "").unwrap();
    assert_eq!(n.score, (3, 0));
    assert!(n.mark_game("start").is_err(), "end the period first");
    n.mark_game("end").unwrap();
    n.mark_game("start").unwrap();
    n.mark_game("end").unwrap();
    assert_eq!(n.phase(), "halftime");
    steps(&mut n, 6);
    n.mark_game("start").unwrap();
    assert_eq!(n.phase(), "in play");
    n.mark_game("final").unwrap();
    assert_eq!(n.phase(), "postgame");
    assert!(n.mark_game("start").is_err());
    // the night closes 20 minutes after the final once nothing is still on its way
    steps(&mut n, 6 * 25);
    assert!(n.done);
    assert!(conserved(&n));
}

#[test]
fn the_simulated_night_is_unchanged() {
    // the default night still runs itself
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let mut n = Night::new(w, t, Config { seed: 3, fans: 400, runners: 8, ..Config::default() });
    while n.t < 80 * 60_000 && n.step() {}
    assert!(count(&n) > 0 && n.score != (0, 0));
    assert!(n.mark_game("start").is_err() && n.keep_score("home", 2, "").is_err(), "only a real night is marked by hand");
}

#[test]
fn every_screen_opens_on_a_real_night() {
    let mut n = night();
    n.join_named("u1", "s104", "Gabe", true).unwrap();
    let stand = n.world.thing("burger").unwrap().stand.clone();
    n.clock_in(&stand).unwrap();
    n.order_as("u1", "burger", 1, true, 0, None).unwrap();
    steps(&mut n, 12);
    for lens in ["fan", "runner", "stand", "ops", "glance", "page", "night"] {
        let id = match lens { "fan" => "u1", "runner" => "runner0", "stand" => stand.as_str(), _ => "" };
        let v = agape_kernel::api::view(&mut n, "ops", &serde_json::json!({ "lens": lens, "id": id }));
        assert!(v.get("error").is_none(), "{lens}: {v}");
    }
    let fan = n.view_fan("u1");
    let menu = fan["items"].as_array().map(|m| m.len()).unwrap_or(0);
    assert!(menu > 0, "the fan sees a menu");
    assert_eq!(n.view_glance()["provenance"]["simulated_fans"], 0);
}

#[test]
fn what_each_card_owes_matches_the_books() {
    let mut n = night();
    n.join_named("u1", "s104", "Gabe", true).unwrap();
    n.join_named("u2", "s110", "Jeff", true).unwrap();
    let stand = n.world.thing("burger").unwrap().stand.clone();
    n.clock_in(&stand).unwrap();
    let a: usize = n.order_as("u1", "burger", 2, false, 0, None).unwrap().parse().unwrap();
    let b: usize = n.order_as("u2", "burger", 1, false, 0, None).unwrap().parse().unwrap();
    let c: usize = n.order_as("u2", "burger", 1, false, 0, None).unwrap().parse().unwrap();
    let due = |n: &Night, i: usize| n.payments_due()[i].clone();
    assert_eq!(due(&n, a)["final"], false, "nothing to capture until it's handed over");
    n.mark_ready(&stand, a).unwrap(); n.handed_over(&stand, a).unwrap();
    n.mark_ready(&stand, b).unwrap(); n.handed_over(&stand, b).unwrap();
    n.cannot_make(&stand, c).unwrap();
    assert_eq!(due(&n, c)["capture"], 0, "a stand that couldn't make it charges nothing");
    let paid = -n.ledger.balance(&acct("u1", "card"), &Unit::usd()) - n.ledger.balance(&acct("u2", "card"), &Unit::usd());
    let captured: i64 = n.payments_due().iter().map(|d| d["capture"].as_i64().unwrap() - d["refund"].as_i64().unwrap()).sum();
    assert_eq!(captured, paid, "what processors capture is exactly what the ledger says fans paid");
    assert!(conserved(&n));
}

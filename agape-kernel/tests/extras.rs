//! Gifts to another seat, paying to go first, moving to a better seat tonight, reselling tickets
//! to other fans, and halftime trivia. Each is a use of the same offers, orders and rules.

use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night, YOU, PRIORITY_POINTS};
use agape_kernel::social::points;
use agape_kernel::value::{acct, Unit};
use agape_kernel::world::WorldDef;
use serde_json::Value;

fn night() -> Night {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    Night::new(w, t, Config { you_seat: Some("s104".into()), ..Config::default() })
}
fn until(n: &mut Night, min: i64) { while n.t < min * 60_000 && n.step() {} }
fn order(n: &mut Night, who: &str, id: usize) -> Value {
    let v = n.view_fan(who);
    v["orders"].as_array().unwrap().iter().chain(v["me"]["receipts"].as_array().unwrap()).find(|o| o["id"] == id).cloned().unwrap_or(Value::Null)
}
fn pts(n: &Night, who: &str) -> i64 { n.ledger.balance(&acct(who, "points"), &points()) }
fn balanced(n: &Night) -> bool { n.ledger.units().iter().all(|u| n.ledger.total(u) == 0) }
fn grant(n: &mut Night, who: &str, p: i64) {
    n.ledger.post(agape_kernel::value::Txn { memo: "test grant".into(), postings: vec![agape_kernel::value::posting(acct("agape", "issued"), points(), -p), agape_kernel::value::posting(acct(who, "points"), points(), p)] }).unwrap();
}

#[test]
fn a_gift_goes_to_a_friends_seat_and_tells_them_who_sent_it() {
    let mut n = night();
    until(&mut n, 50);
    n.join("ana", "s212", Some(30)).unwrap();
    assert_eq!(n.gift(YOU, "draft", 1, "ana", "", 0), Err("Drinks for 21 and over can't be sent as gifts".into()));
    assert!(n.gift(YOU, "nachos", 1, "nowhere", "", 0).is_err());
    let id: usize = n.gift(YOU, "nachos", 1, "ana", "Happy birthday!", 100).unwrap().parse().unwrap();
    let o = order(&mut n, YOU, id);
    assert_eq!(o["seat"], "s212", "it goes to her seat");
    assert_eq!(o["gift"]["to"], "ana".to_string().replace("ana", &n.party_name("ana").unwrap()));
    assert!(n.social.messages.iter().any(|m| m.1 == YOU && m.2 == "ana" && m.3.contains("Happy birthday!")), "and she's told who it's from");
    for _ in 0..400 { n.step(); if order(&mut n, YOU, id)["stage"] == "Settled" { break; } }
    assert_eq!(order(&mut n, YOU, id)["stage"], "Settled", "paid for by the sender");
    let sec: usize = n.gift(YOU, "popcorn", 1, "s118", "", 0).unwrap().parse().unwrap();
    assert_eq!(order(&mut n, YOU, sec)["seat"], "s118", "or to any section");
    assert!(balanced(&n));
}

#[test]
fn going_first_costs_points_only_if_it_arrives() {
    let mut n = night();
    until(&mut n, 80); // halftime: long queues
    grant(&mut n, YOU, 1000);
    let a: usize = n.you_order("burger", 1, true, 0).unwrap().parse().unwrap();
    let start = pts(&n, YOU);
    n.prioritize(YOU, a).unwrap();
    assert_eq!(pts(&n, YOU), start - PRIORITY_POINTS, "held");
    assert_eq!(n.prioritize(YOU, a), Err("It's already first in line".into()));
    n.you_cancel(a).unwrap();
    n.step();
    assert_eq!(pts(&n, YOU), start, "cancelled: every point back");
    let b: usize = n.you_order("burger", 1, true, 0).unwrap().parse().unwrap();
    n.prioritize(YOU, b).unwrap();
    let stand = n.world.thing("burger").unwrap().stand.clone();
    assert_eq!(n.view_stand(&stand)["queue"][0]["id"], b, "to the front of the kitchen's queue");
    for _ in 0..600 { n.step(); if order(&mut n, YOU, b)["stage"] == "Settled" { break; } }
    n.step();
    assert_eq!(n.ledger.balance(&acct(YOU, "priority_held"), &points()), 0);
    assert!(pts(&n, YOU) < start, "delivered: the points were spent");
    assert!(balanced(&n));
}

#[test]
fn a_fan_moves_to_a_better_seat_tonight_once() {
    let mut n = night();
    until(&mut n, 50);
    let v = n.view_fan(YOU);
    let up = v["upgrades"].as_array().unwrap().iter().find(|u| u["ok"] == true).cloned().expect("an upgrade on offer");
    let sec = up["section"].as_str().unwrap().to_string();
    let card = n.ledger.balance(&acct(YOU, "card"), &Unit::usd());
    n.accept(&format!("upg:{sec}"), YOU, 1, None, "", "me").unwrap();
    assert_eq!(n.view_fan(YOU)["you"]["seat"], sec.as_str(), "deliveries now come to the new seat");
    assert_eq!(card - n.ledger.balance(&acct(YOU, "card"), &Unit::usd()), up["price"].as_i64().unwrap());
    assert!(n.view_fan(YOU)["upgrades"].as_array().unwrap().is_empty(), "no more upgrades on offer once you've moved");
    let id: usize = n.you_order("nachos", 1, true, 0).unwrap().parse().unwrap();
    assert_eq!(order(&mut n, YOU, id)["seat"], sec.as_str());
    assert!(balanced(&n));
}

#[test]
fn tickets_resell_between_fans_at_no_more_than_120_percent() {
    let mut n = night();
    until(&mut n, 50);
    let seats = n.buy_tickets(YOU, "e2", "s104", 2).unwrap();
    let face = n.section_price("s104") * 2;
    assert!(n.resell(YOU, "e2", "s104", face * 2).unwrap_err().contains("120%"));
    let l = n.resell(YOU, "e2", "s104", face * 12 / 10).unwrap();
    assert_eq!(n.resell(YOU, "e2", "s104", face).unwrap_err(), "They're already on sale");
    assert_eq!(n.accept(&l, YOU, 2, None, "", "event"), Err("These are your tickets".into()));
    n.join("ben", "s212", None).unwrap();
    n.accept(&l, "ben", 2, None, "", "event").unwrap();
    assert_eq!(n.seats_of("ben", "e2"), vec![("s104".to_string(), seats.clone())], "the same seats, now his");
    assert!(n.seats_of(YOU, "e2").is_empty());
    let paid = face * 12 / 10;
    assert_eq!(n.ledger.balance(&acct(YOU, "payouts"), &Unit::usd()), paid - 2 * (paid * 5 / 100), "the seller gets it less 5% to the venue and 5% to Agape");
    assert!(balanced(&n));
}

#[test]
fn halftime_trivia_is_a_question_in_the_template_and_pays_points_for_the_right_answer() {
    let mut n = night();
    loop { n.step(); if n.predictions.last().map_or(false, |p| p.ask == "trivia") { break; } }
    let before = pts(&n, YOU);
    n.you_predict("Jalen Ayers").unwrap();
    while n.predictions.last().map_or(true, |p| p.result.is_none()) { n.step(); }
    let p = n.predictions.iter().find(|p| p.ask == "trivia").unwrap();
    assert_eq!(p.result.as_deref(), Some("Jalen Ayers"));
    assert_eq!(pts(&n, YOU) - before, 25, "right answer, 25 points from Agape");
}

#[test]
fn fans_wear_the_home_team_until_they_choose() {
    let mut n = night();
    n.join_named("me", "s104", "Me", false).unwrap();
    let c = n.view_fan("me")["colors"].clone();
    let home = n.league.tonight.home.clone();
    let away = n.league.tonight.away.clone();
    assert_eq!(c["teams"].as_array().unwrap().len(), 2);
    assert_eq!(c["wearing"], home.as_str());
    n.wear("me", &away).unwrap();
    assert_eq!(n.view_fan("me")["colors"]["wearing"], away.as_str());
    n.wear("me", "agape").unwrap();
    assert_eq!(n.view_fan("me")["colors"]["wearing"], "agape");
    assert!(n.wear("me", "nobody").is_err());
    n.wear("me", "").unwrap();
    assert_eq!(n.view_fan("me")["colors"]["wearing"], home.as_str());
}

#[test]
fn a_fan_who_says_they_are_21_can_order_a_drink() {
    let mut n = night();
    n.join_named("me", "s104", "Me", false).unwrap();
    let beer = n.world.def.things.iter().find(|t| t.constraints.iter().any(|c| matches!(c, agape_kernel::world::Constraint::MinAge(_)))).unwrap().id.clone();
    let item = |n: &mut Night| n.view_fan("me")["items"].as_array().unwrap().iter().find(|i| i["id"] == beer.as_str()).cloned().unwrap();
    assert_eq!(item(&mut n)["why_not"], "AgeUnverified");
    n.attest_age("me").unwrap();
    assert_ne!(item(&mut n)["why_not"], "AgeUnverified");
}

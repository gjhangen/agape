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
    assert_eq!(n.ledger.balance(&acct(YOU, "payouts"), &Unit::usd()), paid - 2 * (paid * 5 / 100) - n.terms.processing(paid), "the seller gets it less 5% to the venue, 5% to Agape and card processing");
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

#[test]
fn a_cart_across_two_stands_pays_one_delivery_fee_and_splits_the_tip() {
    let mut n = night();
    n.join_named("me", "s104", "Me", false).unwrap();
    n.attest_age("me").unwrap();
    let food = n.world.def.things.iter().find(|t| t.category == "food").unwrap().clone();
    let other = n.world.def.things.iter().find(|t| t.stand != food.stand && t.category != "merch" && t.constraints.is_empty()).unwrap().clone();
    let lines = vec![(food.id.clone(), 2), (other.id.clone(), 1)];
    let q = n.cart_quote("me", &lines, true, 300).unwrap();
    let ids = n.order_cart("me", &lines, true, 300).unwrap();
    assert_eq!(ids.len(), 2, "one order per stand");
    let v = n.view_fan("me");
    let orders = v["orders"].as_array().unwrap();
    let fees: i64 = orders.iter().map(|o| o["breakdown"]["fee"].as_i64().unwrap()).sum();
    let tips: i64 = orders.iter().map(|o| o["breakdown"]["tip"].as_i64().unwrap()).sum();
    let charge: i64 = orders.iter().map(|o| o["charge"].as_i64().unwrap()).sum();
    assert_eq!(fees, q["fee"].as_i64().unwrap(), "one fee for the cart");
    assert_eq!(tips, 300);
    assert_eq!(charge, q["charge"].as_i64().unwrap(), "the quote is what you pay");
    assert!(n.order_cart("me", &[], true, 0).is_err());
}

#[test]
fn a_venue_sees_its_own_teams_and_players() {
    let mut w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    w.brand = serde_json::from_value(serde_json::json!({ "home": { "name": "Austin Hawks", "short": "Hawks", "abbr": "ATX", "color": "#0A7E4F",
        "players": [{ "name": "Maya Ortiz", "number": 11 }] } })).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let mut n = Night::new(w, t, Config { you_seat: Some("s104".into()), ..Config::default() });
    n.join_named("me", "s104", "Me", false).unwrap();
    let v = n.view_fan("me");
    assert_eq!(v["colors"]["teams"][0]["name"], "Austin Hawks");
    assert_eq!(v["colors"]["teams"][0]["color"], "#0A7E4F");
    assert_eq!(n.renamed("Halftime trivia: who holds the Harbor Kings' single-game scoring record?"), "Halftime trivia: who holds the Austin Hawks' single-game scoring record?");
    assert!(n.league.upcoming.iter().all(|e| !e.title.contains("Kings")), "events carry the venue's names: {:?}", n.league.upcoming.iter().map(|e| &e.title).collect::<Vec<_>>());
    until(&mut n, 100);
    let chat: Vec<String> = n.chat.iter().map(|c| c.3.clone()).collect();
    assert!(!chat.iter().any(|c| c.contains("KINGS") || c.contains("kings") || c.contains('{')), "the crowd uses the venue's names: {chat:?}");
}

#[test]
fn the_crowd_chat_is_kept_clean_and_runners_know_the_exact_seat() {
    let mut n = night();
    n.join_named("me", "s112", "Me", false).unwrap();
    assert!(n.chat_as("me", "DM me for cheap tix at 555-201-8890").is_err());
    assert!(n.chat_as("me", "go to tixdeals.biz").is_err());
    n.chat_as("me", "fuck the comets").unwrap();
    assert_eq!(n.chat.back().unwrap().3, "f••• the comets");
    n.chat_as("me", "LET'S GO KINGS").unwrap();
    assert_eq!(n.chat.back().unwrap().3, "LET'S GO KINGS");
    n.set_spot("me", "f", "7").unwrap();
    assert_eq!(n.view_fan("me")["you"]["spot"], "Row F, seat 7");
}

#[test]
fn last_call_comes_before_the_final_and_kitchens_close_at_it() {
    let mut n = night();
    n.join_named("me", "s104", "Me", false).unwrap();
    n.attest_age("me").unwrap();
    let beer = n.world.def.things.iter().find(|t| t.constraints.iter().any(|c| matches!(c, agape_kernel::world::Constraint::MinAge(_)))).unwrap().id.clone();
    let food = n.world.def.things.iter().find(|t| t.category == "food").unwrap().id.clone();
    until(&mut n, 156);
    assert!(n.order_as("me", &beer, 1, true, 0, None).unwrap_err().contains("last_call"));
    assert!(n.order_as("me", &food, 1, true, 0, None).is_ok());
    until(&mut n, 185);
    assert_eq!(n.order_as("me", &food, 1, true, 0, None).unwrap_err(), "ClosedForNight");
    let item = n.view_fan("me")["items"].as_array().unwrap().iter().find(|i| i["id"] == food.as_str()).cloned().unwrap();
    assert_eq!(item["why_not"], "ClosedForNight");
}

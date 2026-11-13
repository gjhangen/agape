//! Level 5 and the rewards around it: posts, likes and comments; profiles; the section battle's
//! prize; tonight's challenges; and a sponsor writing its own deal.

use agape_kernel::api;
use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night, YOU};
use agape_kernel::social::points;
use agape_kernel::value::{acct, Unit};
use agape_kernel::world::WorldDef;
use serde_json::json;

fn night() -> Night {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    Night::new(w, t, Config { you_seat: Some("s104".into()), ..Config::default() })
}
fn until(n: &mut Night, min: i64) { while n.t < min * 60_000 && n.step() {} }
fn pts(n: &Night, who: &str) -> i64 { n.ledger.balance(&acct(who, "points"), &points()) }
fn balanced(n: &Night) -> bool { n.ledger.units().iter().all(|u| n.ledger.total(u) == 0) }

#[test]
fn a_fan_posts_and_the_crowd_answers_and_teams_post_what_happens() {
    let mut n = night();
    let v = n.view_fan(YOU);
    assert!(v["community"]["everyone"].as_array().unwrap().iter().any(|p| p["kind"] == "team"), "the home team has posted before the doors");
    let id = n.post_as(YOU, "Section 104 is the loudest in the building").unwrap();
    assert!(n.post_as(YOU, "cheap tix, dm me").is_err(), "selling is refused");
    until(&mut n, 12);
    let p = n.social.posts.iter().find(|p| p.id == id).unwrap();
    assert!(!p.likes.is_empty(), "people like it");
    assert!(p.comments.iter().any(|c| c.1 != YOU), "someone answers");
    assert_eq!(n.like(YOU, &id), Ok(true));
    assert_eq!(n.like(YOU, &id), Ok(false));
    n.comment(YOU, &id, "thank you all").unwrap();
    n.delete_post(YOU, &id).unwrap();
    assert!(!n.view_fan(YOU)["community"]["everyone"].as_array().unwrap().iter().any(|p| p["id"] == id.as_str()));
}

#[test]
fn a_profile_is_what_the_fan_wrote_and_nothing_private() {
    let mut n = night();
    let home = n.league.tonight.home.clone();
    n.set_profile(YOU, "Gabe", "Upper deck, every night", &home).unwrap();
    assert!(n.set_profile(YOU, "", "call 555 123 4567", "").is_err());
    assert!(n.set_profile(YOU, "", "", "no-such-team").is_err());
    let page = n.view_page(YOU, "fan77");
    assert_eq!(page["bio"], "Upper deck, every night");
    assert_eq!(page["team"]["id"], home.as_str());
    assert!(page.get("seat").is_none() && page.get("orders").is_none());
}

#[test]
fn the_winning_section_is_paid_at_last_call_and_challenges_pay_once() {
    let mut n = night();
    let start = pts(&n, YOU);
    let home = n.league.tonight.home.clone();
    api::act(&mut n, YOU, &json!({ "do": "follow", "node": home }));
    until(&mut n, 200);
    let b = n.battle.clone().expect("decided at last call");
    let won = n.fans.iter().find(|f| f.id == YOU).unwrap().seat == b.section;
    let v = n.view_fan(YOU);
    let fan_club = v["live"]["challenges"].as_array().unwrap().iter().find(|c| c["id"] == "follow").unwrap().clone();
    assert_eq!(fan_club["done"], true);
    let expected = 20 + if won { agape_kernel::sim::play::BATTLE_PRIZE } else { 0 };
    assert!(pts(&n, YOU) - start >= expected, "the prize and the challenge reach the wallet");
    let once = pts(&n, YOU);
    n.step(); n.step();
    assert_eq!(pts(&n, YOU), once, "nothing pays twice");
    assert!(balanced(&n));
}

#[test]
fn a_sponsor_writes_a_deal_and_its_budget_is_set_aside() {
    let mut n = night();
    assert!(n.sponsor_offer(YOU, "halftime", "slice", 300, 50).is_err(), "a fan can't");
    assert!(n.sponsor_offer("dock-pizza", "halftime", "slice", 800, 50).is_err(), "more than half off is refused");
    n.sponsor_offer("dock-pizza", "halftime", "slice", 300, 50).unwrap();
    assert_eq!(n.ledger.balance(&acct("dock-pizza", "promo_budget"), &Unit::usd()), 300 * 50);
    let s = n.sponsor_json("dock-pizza");
    assert_eq!(s["deals"][0]["status"], "waiting");
    while n.view_fan(YOU)["clock"]["phase"] != "halftime" && n.step() {}
    for _ in 0..6 { n.step(); }
    let s = n.sponsor_json("dock-pizza");
    assert_ne!(s["deals"][0]["status"], "waiting", "it opened at halftime");
    assert!(balanced(&n));
}

#[test]
fn on_a_concert_night_the_venue_and_the_acts_post_and_a_sponsor_can_still_write_a_deal() {
    let mut w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    w.event = Some(serde_json::from_value(json!({ "kind": "concert", "title": "Lumen Rivers", "acts": ["Paper Kites Club", "Mara Vale", "Lumen Rivers"] })).unwrap());
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let mut n = Night::new(w, t, Config { you_seat: Some("s104".into()), ..Config::default() });
    let posts = n.view_fan(YOU)["community"]["everyone"].as_array().unwrap().clone();
    assert!(posts.iter().any(|p| p["kind"] == "venue") && posts.iter().any(|p| p["kind"] == "act" && p["who"] == "Lumen Rivers"));
    assert!(!posts.iter().any(|p| p["kind"] == "team" || p["kind"] == "player"), "no teams at a concert");
    assert!(n.sponsor_offer("dock-pizza", "home_run", "slice", 300, 50).is_err(), "no game moments at a concert");
    n.sponsor_offer("dock-pizza", "halftime", "slice", 300, 50).unwrap();
    until(&mut n, 80);
    assert!(n.social.posts.iter().any(|p| p.text == "Mara Vale is on now."), "the venue says who's on");
}

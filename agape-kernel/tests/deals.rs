//! Deals between parties, players claiming their profiles, and passes used at the counter.

use agape_kernel::dealing::pass_code;
use agape_kernel::exchange::Stage;
use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night, YOU};
use agape_kernel::social::{lifetime, points};
use agape_kernel::value::{acct, Unit};
use agape_kernel::world::WorldDef;

fn night() -> Night {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    Night::new(w, t, Config { you_seat: Some("s104".into()), ..Config::default() })
}
fn until(n: &mut Night, min: i64) { while n.t < min * 60_000 && n.step() {} }
fn mins(n: &mut Night, m: i64) { let end = n.t + m * 60_000; while n.t < end && n.step() {} }
fn pts(n: &Night, who: &str) -> i64 { n.ledger.balance(&acct(who, "points"), &points()) }
fn balanced(n: &Night) -> bool { n.ledger.units().iter().all(|u| n.ledger.total(u) == 0) }
fn rich(n: &mut Night) { n.buy_tickets(YOU, "e2", "sFL", 1).unwrap(); } // courtside earns thousands of points

#[test]
fn a_deal_holds_the_pay_until_it_is_done_then_pays_the_doer_less_the_fee() {
    let mut n = night();
    until(&mut n, 50);
    rich(&mut n);
    n.drive("hbk-1"); // a person is acting as Jalen Ayers
    let (before, level) = (pts(&n, YOU), lifetime(&n.ledger, YOU));
    let id = n.propose_deal(YOU, "hbk-1", "shoutout", "for my daughter Ava", 0).unwrap();
    assert_eq!(pts(&n, YOU), before - 1_200, "the pay leaves at once");
    assert_eq!(n.ledger.balance(&acct("escrow", &id), &points()), 1_200, "and waits in escrow");
    assert_eq!(n.confirm_deal(YOU, &id), Err("That deal has moved on".into()), "nothing to confirm yet");
    assert_eq!(n.respond_deal(YOU, &id, true), Err("That isn't your deal to act on".into()));
    n.respond_deal("hbk-1", &id, true).unwrap();
    assert_eq!(n.withdraw_deal(YOU, &id), Err("That deal has moved on".into()), "an accepted deal can't be pulled");
    n.deliver_deal("hbk-1", &id, "Ava! Thanks for coming out tonight.").unwrap();
    let v = n.view_fan(YOU);
    let thread = v["me"]["messages"].as_array().unwrap().iter().find(|t| t["with"] == "hbk-1").unwrap();
    assert!(thread["lines"].as_array().unwrap().iter().any(|l| l["text"] == "Ava! Thanks for coming out tonight."), "the shoutout lands in your messages");
    let issued = n.ledger.balance(&acct("agape", "issued"), &points());
    n.confirm_deal(YOU, &id).unwrap();
    assert_eq!(pts(&n, "hbk-1"), 1_080, "the doer gets the pay less 10%");
    assert_eq!(n.ledger.balance(&acct("agape", "issued"), &points()), issued + 120, "the fee returns to the issuer");
    assert_eq!(n.ledger.balance(&acct("escrow", &id), &points()), 0);
    assert_eq!(lifetime(&n.ledger, YOU), level, "spending never lowers your level");
    assert_eq!(lifetime(&n.ledger, "hbk-1"), 0, "points from fans don't raise anyone's level");
    assert!(balanced(&n));
}

#[test]
fn a_no_or_a_withdrawal_returns_every_point() {
    let mut n = night();
    until(&mut n, 50);
    assert_eq!(n.propose_deal(YOU, "hbk-1", "shoutout", "", 0), Err("CantAfford".into()), "can't offer points you don't have");
    rich(&mut n);
    n.drive("hbk-1");
    let before = pts(&n, YOU);
    let a = n.propose_deal(YOU, "hbk-1", "shoutout", "", 0).unwrap();
    n.respond_deal("hbk-1", &a, false).unwrap();
    assert_eq!(pts(&n, YOU), before);
    let b = n.propose_deal(YOU, "hbk-1", "shoutout", "", 0).unwrap();
    n.withdraw_deal(YOU, &b).unwrap();
    assert_eq!(pts(&n, YOU), before);
    assert_eq!(n.propose_deal(YOU, "hbk-1", "promo", "", 0), Err("Only businesses pay money for deals".into()));
    assert!(balanced(&n));
}

#[test]
fn an_unclaimed_player_never_answers_until_they_claim_their_profile() {
    let mut n = night();
    until(&mut n, 50);
    rich(&mut n);
    let bench = n.league.team("hbk").unwrap().roster[4].id.clone();
    assert!(!n.claimed.contains(&bench));
    let id = n.propose_deal(YOU, &bench, "shoutout", "", 0).unwrap();
    let page = n.view_page(&bench, YOU);
    assert_eq!(page["claimed"], false);
    assert_eq!(page["your_deals"][0]["waits_for_claim"], true, "the page says the offer is waiting for them");
    mins(&mut n, 15);
    assert_eq!(n.trade.get(&id).unwrap().stage, Stage::Held, "no one answers for them");
    n.claim(&bench).unwrap();
    mins(&mut n, 15);
    assert_ne!(n.trade.get(&id).unwrap().stage, Stage::Held, "once claimed, they answer");
}

#[test]
fn limited_rates_say_how_many_are_left_and_stop_at_the_limit() {
    let mut n = night();
    until(&mut n, 50);
    n.ledger.post(agape_kernel::value::Txn { memo: "test grant".into(), postings: vec![
        agape_kernel::value::posting(acct("agape", "issued"), points(), -20_000), agape_kernel::value::posting(acct(YOU, "points"), points(), 20_000)] }).unwrap();
    n.drive("hbk-1");
    let left = |n: &Night| n.view_page("hbk-1", YOU)["rates"].as_array().unwrap().iter().find(|r| r["kind"] == "tunnel").unwrap()["left"].as_i64().unwrap();
    let start = left(&n);
    for _ in 0..start { n.propose_deal(YOU, "hbk-1", "tunnel", "", 0).unwrap(); }
    assert_eq!(left(&n), 0);
    assert_eq!(n.propose_deal(YOU, "hbk-1", "tunnel", "", 0), Err("They're fully booked tonight".into()));
}

#[test]
fn a_silent_proposer_cannot_keep_the_doer_waiting() {
    let mut n = night();
    until(&mut n, 50);
    rich(&mut n);
    let id = n.propose_deal(YOU, "hbk-1", "shoutout", "", 0).unwrap();
    mins(&mut n, 40); // the simulated player answers and delivers; you never confirm
    let d = n.trade.get(&id).unwrap();
    if d.stage != Stage::Closed {
        assert_eq!(d.stage, Stage::Settled, "settled on its own 15 minutes after delivery");
        assert_eq!(pts(&n, "hbk-1") >= 1_080, true);
    }
}

#[test]
fn a_whole_night_of_deals_balances_and_escrow_holds_exactly_the_open_deals() {
    let mut n = night();
    while n.step() {}
    use agape_kernel::deals::Pay;
    use agape_kernel::exchange::Kind;
    let deals: Vec<_> = n.trade.of_kind(Kind::Deal).cloned().collect();
    assert!(deals.iter().any(|d| d.pay == Pay::Money && d.stage == Stage::Settled), "businesses sponsored players and paid");
    assert!(deals.iter().any(|d| d.offer.ends_with(":shoutout") && d.stage == Stage::Settled), "fans got shoutouts");
    for d in &deals {
        let u = agape_kernel::deals::unit_of(&d.pay);
        assert_eq!(n.ledger.balance(&acct("escrow", &d.id), &u), if d.live() { d.amount } else { 0 }, "{} {:?}", d.id, d.stage);
    }
    // each business's budget = funded - escrowed - paid
    let spent: i64 = deals.iter().filter(|d| d.buyer == "dock-pizza" && d.stage != Stage::Closed).map(|d| d.amount).sum();
    assert_eq!(n.ledger.balance(&acct("dock-pizza", "deal_budget"), &Unit::usd()), 3_000_00 - spent);
    assert!(n.view_business("dock-pizza")["passes"][0]["used"].as_i64().unwrap() > 0, "fans used their perks after the game");
    assert!(balanced(&n));
}

#[test]
fn a_perk_is_used_once_at_the_counter_with_the_code_on_the_fans_phone() {
    let mut n = night();
    until(&mut n, 50);
    rich(&mut n);
    n.unlock(YOU, "copper-tap").unwrap();
    let v = n.view_fan(YOU);
    let pass = v["me"]["passes"].as_array().unwrap().iter().find(|p| p["place"] == "copper-tap").unwrap().clone();
    assert!(pass["used"].is_null());
    assert_eq!(n.redeem("copper-tap", "000000"), Err("No pass with that code".into()));
    assert_eq!(n.redeem("dock-pizza", pass["code"].as_str().unwrap()), Err("No pass with that code".into()), "a code works only where it was issued");
    assert_eq!(n.redeem("copper-tap", pass["code"].as_str().unwrap()), Ok(YOU.to_string()));
    assert_eq!(pass["code"].as_str().unwrap(), pass_code(YOU, "copper-tap"));
    assert_eq!(n.redeem("copper-tap", pass["code"].as_str().unwrap()), Err("Already used".into()));
    let v = n.view_fan(YOU);
    assert!(v["me"]["passes"].as_array().unwrap().iter().any(|p| p["place"] == "copper-tap" && !p["used"].is_null()), "still shown, marked used");
    assert!(balanced(&n));
}

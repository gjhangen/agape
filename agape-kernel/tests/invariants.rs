//! The substrate's invariants, checked over every exchange, not only the food order: one world
//! (state is a fold over the log), attribution (every agreement and every transaction it causes
//! traces to an offer), the Agape check before acceptance, named exits, and conservation.

use agape_kernel::exchange::{allowed, Agreement, Book, Exit, Kind, Stage};
use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night, YOU};
use agape_kernel::value::{acct, posting, Txn, Unit};
use agape_kernel::world::WorldDef;
use std::collections::{BTreeMap, BTreeSet};

fn night_with(tpl: &str) -> Night {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(tpl).unwrap();
    Night::new(w, t, Config { you_seat: Some("s104".into()), ..Config::default() })
}
fn night() -> Night { night_with(include_str!("../templates/basketball.json")) }

/// A full night with the live fan doing a bit of everything.
fn busy_night() -> Night {
    let mut n = night();
    while n.t < 50 * 60_000 { n.step(); }
    n.buy_tickets(YOU, "e2", "sFL", 1).unwrap();
    n.unlock(YOU, "dock-pizza").unwrap();
    n.prepay_parking(YOU, "lot-c").unwrap();
    let fin = n.script_final();
    n.book_ride(YOU, "ride-b", fin + 10 * 60_000).unwrap();
    n.propose_deal(YOU, "hbk-1", "shoutout", "Ava", 0).unwrap();
    n.you_follow("hbk-1");
    n.you_message("hbk-1", "Big game tonight").unwrap();
    while n.step() {}
    n
}

#[test]
fn one_world_every_exchange_and_social_fact_is_a_fold_over_the_log() {
    let n = busy_night();
    let replay = Book::fold(&n.log.signals);
    assert!(replay.same_as(&n.trade), "offers and agreements rebuilt from the log alone match the live state");
    assert!(n.trade.all().len() > 300, "a real night's worth of exchanges: {}", n.trade.all().len());
    // follows, messages, claims and redemptions too
    let (mut follows, mut msgs, mut claimed, mut redeemed): (BTreeMap<String, BTreeSet<String>>, Vec<(i64, String, String, String)>, BTreeSet<String>, BTreeSet<(String, String)>) = Default::default();
    for s in n.log.signals.iter().filter(|s| s.kind == "social") {
        let f = |k: &str| s.fields[k].key();
        match f("act").as_str() {
            "follow" => { follows.entry(s.author.clone()).or_default().insert(f("target")); }
            "unfollow" => { follows.entry(s.author.clone()).or_default().remove(&f("target")); }
            "message" => msgs.push((s.t_ms, s.author.clone(), f("target"), f("text"))),
            "claim" => { claimed.insert(s.author.clone()); }
            "redeem" => { redeemed.insert((f("text"), f("target"))); }
            _ => {}
        }
    }
    let live_follows: BTreeMap<String, BTreeSet<String>> = n.social.follows.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| (k.clone(), v.clone())).collect();
    follows.retain(|_, v| !v.is_empty());
    assert_eq!(follows, live_follows);
    assert_eq!(msgs, n.social.messages);
    assert_eq!(claimed, n.claimed);
    assert_eq!(redeemed, n.redeemed.keys().cloned().collect::<BTreeSet<_>>());
    // seats on the map are exactly the seats the ticket agreements name
    let sold: usize = n.seatmaps.values().map(|m| m.owner.iter().filter(|o| o.as_deref().map_or(false, |x| x != "presold")).count()).sum();
    let agreed: i64 = n.trade.of_kind(Kind::Ticket).map(|a| a.qty).sum();
    assert_eq!(sold as i64, agreed);
}

#[test]
fn attribution_every_agreement_names_its_offer_and_every_transaction_names_its_agreement() {
    let n = busy_night();
    for a in n.trade.all() {
        assert!(n.trade.offer(&a.offer).is_some(), "{} accepted an offer that exists", a.id);
        assert!(!a.via.is_empty(), "{} says where it was seen", a.id);
    }
    const SEEDING: [&str; 9] = ["history", "welcome", "onsale", "lot", "perk", "mint", "grant", "deal", "fund"];
    let touches_exchange = |t: &Txn| t.postings.iter().any(|p| p.account.party == "escrow" || p.account.purpose == "deal_budget" || p.account.purpose == "passes" || p.account.purpose == "listed"
        || matches!(&p.unit, Unit::Thing(s) if s.starts_with("ticket:") || s.starts_with("pass:") || s.starts_with("card:")))
        || t.memo.starts_with("ride ") || t.memo.starts_with("parking ");
    let mut checked = 0;
    for t in &n.ledger.txns {
        if !touches_exchange(t) { continue; }
        let first = t.memo.split(' ').next().unwrap_or("");
        if SEEDING.contains(&first) && !t.memo.contains(" x:") && (first != "deal" || t.memo.starts_with("deal budget")) { continue; }
        let id = t.memo.split(" x:").nth(1).unwrap_or_else(|| panic!("unattributed transaction: {}", t.memo));
        assert!(n.trade.get(id).is_some() || n.trade.offer(id).is_some(), "{} names {id}, which exists", t.memo);
        checked += 1;
    }
    assert!(checked > 300, "checked {checked}");
    // every settled agreement posted its settlement exactly once
    let mut settles: BTreeMap<String, usize> = BTreeMap::new();
    for t in &n.ledger.txns {
        if let Some(id) = t.memo.split(" x:").nth(1) {
            if !(t.memo.starts_with("deal escrow") || t.memo.starts_with("deal returned") || t.memo.starts_with("list ") || t.memo.starts_with("delist ") || t.memo.starts_with("redeem ")) { *settles.entry(id.to_string()).or_default() += 1; }
        }
    }
    for a in n.trade.all().iter().filter(|a| a.stage == Stage::Settled) { assert_eq!(settles.get(&a.id), Some(&1), "{} {:?}", a.id, a.kind); }
    for a in n.trade.all().iter().filter(|a| a.stage != Stage::Settled) { assert_eq!(settles.get(&a.id), None, "{} {:?} {:?}", a.id, a.kind, a.stage); }
}

#[test]
fn the_agape_check_refuses_anything_that_leaves_a_non_payer_worse_off() {
    let n = night();
    let usd = Unit::usd();
    let fine = Txn { memo: "t".into(), postings: vec![posting(acct("fan1", "card"), usd.clone(), -1000), posting(acct("stand", "revenue"), usd.clone(), 900), posting(acct("agape", "revenue"), usd.clone(), 100)] };
    assert_eq!(n.agape_check(&fine, "fan1"), Ok(()));
    let loses = Txn { memo: "t".into(), postings: vec![posting(acct("fan1", "card"), usd.clone(), -1000), posting(acct("stand", "revenue"), usd.clone(), 1100), posting(acct("agape", "revenue"), usd, -100)] };
    assert_eq!(n.agape_check(&loses, "fan1"), Err("Loses(agape)".into()), "Agape doesn't quietly subsidise a sale either");
}

#[test]
fn every_exit_is_named_with_whose_doing_it_was() {
    let a = Agreement { id: "a1".into(), offer: "ride:ride-b".into(), kind: Kind::Ride, buyer: "fan".into(), maker: "co".into(), doer: Some("drv".into()), qty: 1, amount: 2000,
        pay: agape_kernel::deals::Pay::Money, at: Some(0), detail: String::new(), proof: None, step: "on_board".into(), stage: Stage::Active, exit: None, via: "x".into(), made: 0, history: vec![] };
    assert!(allowed(&a, Stage::Closed, "fan", Some(Exit::BuyerCancelled), false).is_err(), "no cancelling once you're in the car");
    let assigned = Agreement { step: "assigned".into(), ..a.clone() };
    assert!(allowed(&assigned, Stage::Closed, "fan", Some(Exit::BuyerCancelled), false).is_ok());
    assert!(allowed(&assigned, Stage::Closed, "co", Some(Exit::BuyerCancelled), false).is_err(), "only the rider cancels as the rider");
    let held = Agreement { stage: Stage::Held, doer: None, ..a.clone() };
    assert!(allowed(&held, Stage::Closed, "co", Some(Exit::NoDoer), true).is_ok());
    assert_eq!(Exit::NoDoer.fault(&held), Some("co"), "no driver is the company's failure");
    assert_eq!(Exit::BuyerCancelled.fault(&held), Some("fan"));
    assert_eq!(Exit::MakerDeclined.fault(&held), None, "saying no is nobody's fault");
}

#[test]
fn a_ride_nobody_can_take_closes_on_the_company_and_charges_nothing() {
    let mut n = night();
    while n.t < 50 * 60_000 { n.step(); }
    let at = n.t + 10 * 60_000;
    let id = n.book_ride(YOU, "ride-b", at).unwrap();
    // then every Harborline driver goes offline before pickup
    n.world.def.drivers.retain(|d| d.provider != "harborline");
    let before = -n.ledger.balance(&acct(YOU, "card"), &Unit::usd());
    while n.t < at + 60_000 { n.step(); }
    let a = n.trade.get(&id).unwrap();
    assert_eq!((a.stage, a.exit), (Stage::Closed, Some(Exit::NoDoer)));
    assert_eq!(Exit::NoDoer.fault(a), Some("harborline"));
    assert_eq!(-n.ledger.balance(&acct(YOU, "card"), &Unit::usd()), before, "not charged");
    assert_eq!(n.view_fan(YOU)["me"]["rides"][0]["stage"], "NoDriver", "and told plainly");
}

#[test]
fn concert_test_exchanges_work_unchanged_under_a_concert_template() {
    let mut n = night_with(include_str!("../templates/concert.json"));
    while n.t < 50 * 60_000 { n.step(); }
    let seats = n.buy_tickets(YOU, "e3", "s104", 2).unwrap();
    assert_eq!(seats.len(), 2);
    n.unlock(YOU, "dock-pizza").unwrap();
    n.book_ride(YOU, "ride-b", n.t + 30 * 60_000).unwrap();
    while n.step() {}
    assert!(Book::fold(&n.log.signals).same_as(&n.trade));
    assert!(n.ledger.units().iter().all(|u| n.ledger.total(u) == 0));
}

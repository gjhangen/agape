//! Moment cards, the points market, and level.

use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night, YOU};
use agape_kernel::social::*;
use agape_kernel::value::{acct, posting, Ledger, Txn};
use agape_kernel::world::WorldDef;

fn give_points(l: &mut Ledger, fan: &str, n: i64) {
    l.post(Txn { memo: "earn".into(), postings: vec![posting(acct(ISSUER, "issued"), points(), -n), posting(acct(fan, "points"), points(), n)] }).unwrap();
}
fn edition(id: &str, supply: i64) -> Edition {
    Edition { id: id.into(), kind: "splash".into(), title: "Ayers · splash".into(), team: "hbk".into(), party: Some("hbk-1".into()), t: 0, supply, rarity: "Rare".into(), open_until: 300_000 }
}
fn conserved(l: &Ledger) -> bool { l.units().iter().all(|u| l.total(u) == 0) }

fn night() -> Night {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    Night::new(w, t, Config { you_seat: Some("s104".into()), ..Config::default() })
}

#[test]
fn a_drop_has_a_fixed_supply_one_per_fan_and_closes_on_time() {
    let mut n = night();
    let mut e = edition("x0", 3); e.open_until = n.t + 300_000;
    n.social.mint(&mut n.ledger, e).unwrap();
    n.open_drop("x0");
    for f in ["fan1", "fan2", "fan3"] { n.accept("drop:x0", f, 1, None, "", "test").unwrap(); }
    assert_eq!(n.accept("drop:x0", "fan4", 1, None, "", "test"), Err("Closed".into()), "the supply is the supply");
    let mut e = edition("x1", 10); e.open_until = n.t + 300_000;
    n.social.mint(&mut n.ledger, e).unwrap();
    n.open_drop("x1");
    n.accept("drop:x1", "fan1", 1, None, "", "test").unwrap();
    assert_eq!(n.accept("drop:x1", "fan1", 1, None, "", "test"), Err("AlreadyCollected".into()));
    for _ in 0..31 { n.step(); }
    assert_eq!(n.accept("drop:x1", "fan2", 1, None, "", "test"), Err("Closed".into()), "five minutes, then it's gone");
    assert!(conserved(&n.ledger));
}

#[test]
fn trading_moves_points_between_fans_with_the_fee_back_to_the_issuer() {
    let mut n = night();
    let mut e = edition("x0", 5); e.open_until = n.t + 300_000;
    n.social.mint(&mut n.ledger, e).unwrap();
    n.open_drop("x0");
    n.accept("drop:x0", "fan1", 1, None, "", "test").unwrap();
    let (s0, b0) = (n.ledger.balance(&acct("fan1", "points"), &points()), n.ledger.balance(&acct("fan2", "points"), &points()));
    give_points(&mut n.ledger, "fan2", 300);
    let listing = n.list_card("fan1", "x0", 200).unwrap();
    assert_eq!(n.list_card("fan1", "x0", 200), Err("NotHeld".into()), "a listed card can't be listed twice");
    assert_eq!(n.accept(&listing, "fan1", 1, None, "", "test"), Err("OwnListing".into()));
    let poor = n.fans.iter().map(|f| f.id.clone()).find(|f| n.ledger.balance(&acct(f, "points"), &points()) < 200).unwrap();
    assert_eq!(n.accept(&listing, &poor, 1, None, "", "test"), Err("CantAfford".into()));
    let lvl = lifetime(&n.ledger, "fan1");
    n.accept(&listing, "fan2", 1, None, "", "test").unwrap();
    assert_eq!(n.ledger.balance(&acct("fan2", "points"), &points()), b0 + 100);
    assert_eq!(n.ledger.balance(&acct("fan1", "points"), &points()), s0 + 190, "5% fee");
    assert_eq!(n.ledger.balance(&acct("fan2", "holdings"), &card_unit("x0")), 1);
    assert_eq!(n.ledger.balance(&acct("fan1", "listed"), &card_unit("x0")), 0);
    assert_eq!(n.accept(&listing, "fan3", 1, None, "", "test"), Err("Unknown".into()), "sold once");
    assert!(conserved(&n.ledger));
    // level counts only what Agape issued: points from another fan are spendable, not status
    assert_eq!(lifetime(&n.ledger, "fan1"), lvl);
}

#[test]
fn a_whole_night_mints_moments_and_runs_a_market_without_breaking_the_books() {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let mut n = Night::new(w, t, Config { you_seat: Some("s104".into()), ..Config::default() });
    let mut collected = false;
    while n.step() {
        if !collected {
            if let Some(e) = n.social.editions.values().find(|e| e.kind != "season" && e.open_until >= n.t).map(|e| e.id.clone()) {
                n.you_collect(&e).unwrap();
                collected = true;
            }
        }
    }
    assert!(collected, "the live fan collected a moment");
    assert!(n.social.editions.values().filter(|e| e.kind != "season").count() >= 3, "moments became cards");
    assert!(n.trade.of_kind(agape_kernel::exchange::Kind::Card).filter(|a| a.stage == agape_kernel::exchange::Stage::Settled).count() > 10, "the crowd traded");
    assert!(n.ledger.units().iter().all(|u| n.ledger.total(u) == 0), "every unit, cards and points included, sums to zero");
    for f in n.fans.iter() {
        assert!(n.ledger.balance(&acct(&f.id, "points"), &points()) >= 0);
    }
    let lvl = lifetime(&n.ledger, YOU);
    assert!(lvl >= 100, "welcome points count toward level");
    let (name, _, _) = tier(lvl);
    assert!(TIERS.iter().any(|(_, t)| *t == name));
}

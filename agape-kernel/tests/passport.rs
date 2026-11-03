//! One fan across venues, and one deal across a league.

use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night};
use agape_kernel::social::points;
use agape_kernel::value::acct;
use agape_kernel::world::WorldDef;
use serde_json::json;

fn venue(seed: u64) -> Night {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let mut n = Night::new(w, t, Config { seed, fans: 800, runners: 12, ..Config::default() });
    while n.t < 50 * 60_000 { n.step(); }
    n
}
fn pts(n: &Night, who: &str) -> i64 { n.ledger.balance(&acct(who, "points"), &points()) }
fn balanced(n: &Night) -> bool { n.ledger.units().iter().all(|u| n.ledger.total(u) == 0) }

#[test]
fn a_fans_points_and_level_go_with_them_to_the_next_venue_exactly_once() {
    let mut a = venue(1);
    let mut b = venue(2);
    a.join_named("ana", "s104", "ana", true).unwrap();
    a.buy_tickets("ana", "e2", "s104", 2).unwrap(); // earns points and level here
    let (have, level) = (pts(&a, "ana"), a.level("ana"));
    assert!(have > 100 && level > 100);
    let out = a.carry_out("t1", "ana").unwrap();
    assert_eq!((out.points, out.level), (have, level));
    assert_eq!(pts(&a, "ana"), 0, "they left with her");
    assert_eq!(a.carry_out("t1", "ana").unwrap(), out, "repeating a transfer returns the same answer");
    assert_eq!(pts(&a, "ana"), 0, "and moves nothing twice");
    b.join_named("ana", "s212", "ana", false).unwrap();
    assert_eq!(pts(&b, "ana"), 0, "no second welcome");
    b.carry_in("t2", "ana", out.points, out.level, json!({ "venues": ["Harbor Arena"], "nights": 1 })).unwrap();
    b.carry_in("t2", "ana", out.points, out.level, json!({})).unwrap();
    assert_eq!(pts(&b, "ana"), have, "arrived once");
    assert_eq!(b.level("ana"), level, "her level came with her");
    b.buy_tickets("ana", "e3", "s212", 1).unwrap();
    assert!(b.level("ana") > level, "and keeps growing here");
    assert_eq!(b.view_fan("ana")["me"]["passport"]["venues"][0], "Harbor Arena");
    assert!(balanced(&a) && balanced(&b));
}

#[test]
fn a_league_wide_deal_is_one_rule_installed_in_every_venue() {
    let rule = json!({ "name": "league_kings_run", "stratum": 2, "when": { "match": { "sig": "moment", "where": { "kind": { "eq": "run" }, "team": { "eq": "home" } } } },
        "throttle_ms": 600000, "then": [{ "offer": { "thing": "foam_finger", "price": 1500, "discount": 500, "payer": "league_sponsor", "expires_ms": 600000 } }] });
    let mut venues = vec![venue(3), venue(4)];
    for n in venues.iter_mut() {
        assert!(n.install_rule(json!({ "name": "broken", "when": { "match": { "sig": "nope" } }, "then": [] }), "", 0).is_err(), "a rule that doesn't fit the schema is refused");
        n.install_rule(rule.clone(), "league_sponsor", 200_00).unwrap();
        n.install_rule(rule.clone(), "league_sponsor", 0).unwrap(); // installing twice is harmless
    }
    for n in venues.iter_mut() {
        while n.step() {}
        assert!(n.report.attribution.contains_key("league_kings_run") || n.attr.by_source.contains_key("league_kings_run") || n.book.offers.values().any(|o| o.source == "league_kings_run"),
            "the league's deal ran in this venue");
        assert!(balanced(n));
    }
}

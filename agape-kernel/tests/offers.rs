//! The no-surprise invariant for offers, and the points policy.

use agape_kernel::commitment::{fund, standard_policy, stock};
use agape_kernel::engines::{Attribution, OfferSpec};
use agape_kernel::fields;
use agape_kernel::offers::{CheckoutError, End, OfferBook, TapError};
use agape_kernel::points::{expire, expiring, lots, PointsPolicy};
use agape_kernel::rules::{Cond, Engine, Limits, Mode, Template, Tier};
use agape_kernel::signal::{Log, Roles};
use agape_kernel::sim::sports_schema;
use agape_kernel::value::{acct, posting, Ledger, Txn, Unit};
use agape_kernel::world::{Fan, World};

struct T { w: World, l: Ledger, e: Engine, log: Log, fans: Vec<Fan>, attr: Attribution, t: i64 }

fn setup(stock_n: i64, budget: i64, fans: usize) -> T {
    let w = World::build(serde_json::from_str(include_str!("../templates/arena.json")).unwrap());
    let mut l = Ledger::new(standard_policy());
    stock(&mut l, "snack-213", "water", stock_n).unwrap();
    fund(&mut l, "cola", budget).unwrap();
    let tpl: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let e = Engine::load(tpl, &sports_schema()).unwrap();
    let mut roles = Roles::default();
    roles.grant("keeper", "scorekeeper"); roles.grant("venue", "venue");
    let log = Log::new(sports_schema(), roles);
    let fans = (0..fans).map(|i| Fan { id: format!("f{i}"), seat: "s105".into(), age_verified: None, step_free: false,
        consents: ["offers".to_string()].into(), ceiling: 10_000 }).collect();
    T { w, l, e, log, fans, attr: Attribution::default(), t: 0 }
}

impl T {
    fn sig(&mut self, k: &str, f: agape_kernel::signal::Fields, by: &str) { self.t += 1000; self.e.ingest(&mut self.log, k, f, by, self.t).unwrap(); }
    fn open(&mut self, book: &mut OfferBook, limits: Limits, discount: i64, expires: i64) -> String {
        let spec = OfferSpec { source: "deal", thing: "water", price: 550, discount, payer: Some("cola"), expires_ms: Some(expires), t_ms: self.t, pushed: true };
        book.open(&self.w, &mut self.l, &self.e, &spec, &limits, &self.fans, &|_| 0, &mut self.attr).unwrap()
    }
    fn conserved(&self) -> bool { self.l.units().iter().all(|u| self.l.total(u) == 0) }
    fn inv(&self) -> i64 { self.l.balance(&acct("snack-213", "inventory"), &Unit::Thing("water".into())) }
    fn budget(&self) -> i64 { self.l.balance(&acct("cola", "promo_budget"), &Unit::usd()) }
}

#[test]
fn guaranteed_is_shown_only_to_those_it_can_serve_and_held_for_them() {
    let mut x = setup(5, 10_000, 10);
    let mut book = OfferBook::new(120_000);
    let id = x.open(&mut book, Limits::default(), 200, 600_000);
    assert_eq!(book.offers[&id].audience.len(), 5, "five units, five fans");
    assert_eq!(x.inv(), 0, "the promise takes the units off open stock: the menu shows sold out, not a broken promise");
    assert_eq!(x.budget(), 10_000 - 5 * 200);
    let f0 = x.fans[0].clone();
    let d = book.disclose(&id, "f0", &x.l).unwrap();
    assert!(d.guaranteed && d.held_for_you && d.left.is_none() && d.ends_at == Some(600_000));
    // f0 taps and walks away; the guarantee returns to f0, not to the world
    let c = book.tap(&id, &f0, &x.w, &mut x.l, &x.e, x.t).unwrap();
    book.expire_claims(&mut x.l, c.until + 1);
    assert!(book.disclose(&id, "f0", &x.l).unwrap().held_for_you);
    assert_eq!(x.inv(), 0);
    // f0 comes back and checks out
    let c = book.tap(&id, &f0, &x.w, &mut x.l, &x.e, x.t).unwrap();
    let (order, src) = book.checkout(&c.id, "o1", x.t, 0).unwrap();
    assert_eq!((order.quote.discounts[0].amount, src.as_str()), (200, "deal"));
    assert_eq!(book.tap(&id, &f0, &x.w, &mut x.l, &x.e, x.t), Err(TapError::Ended(End::CapReached)), "one per fan, shown as claimed");
    // a fan it was not shown to cannot claim it
    let f9 = x.fans[9].clone();
    assert!(book.offers[&id].audience.contains("f9") == false);
    assert_eq!(book.tap(&id, &f9, &x.w, &mut x.l, &x.e, x.t), Err(TapError::NotShown));
    // at expiry the four unclaimed promises go back
    book.refresh(&mut x.l, &x.e, 600_001);
    assert_eq!(x.inv(), 4);
    assert_eq!(x.budget(), 10_000 - 200);
    assert!(x.conserved());
}

#[test]
fn while_it_lasts_shows_what_is_left_and_never_fails_after_the_tap() {
    let mut x = setup(1000, 10_000, 10);
    let mut book = OfferBook::new(120_000);
    let limits = Limits { mode: Mode::WhileItLasts, cap: Some(3), label: Some("First 3 only".into()), ..Limits::default() };
    let id = x.open(&mut book, limits, 300, 600_000);
    assert_eq!(book.offers[&id].audience.len(), 10, "everyone eligible sees it");
    assert_eq!(x.inv(), 1000, "nothing is promised up front");
    for i in 0..3 {
        let d = book.disclose(&id, &format!("f{i}"), &x.l).unwrap();
        assert_eq!(d.left, Some(3 - i as i64));
        assert!(d.ended.is_none() && !d.guaranteed);
        let f = x.fans[i].clone();
        let c = book.tap(&id, &f, &x.w, &mut x.l, &x.e, x.t).unwrap();
        book.checkout(&c.id, &format!("o{i}"), x.t + 30_000, 0).unwrap();
    }
    // the fourth fan's card already says it is over, before any tap
    let d = book.disclose(&id, "f3", &x.l).unwrap();
    assert_eq!((d.left, d.ended), (Some(0), Some(End::CapReached)));
    let f3 = x.fans[3].clone();
    assert_eq!(book.tap(&id, &f3, &x.w, &mut x.l, &x.e, x.t), Err(TapError::Ended(End::CapReached)));
    assert!(x.conserved());
}

#[test]
fn a_lapsed_claim_frees_its_place_and_an_expired_checkout_is_refused_cleanly() {
    let mut x = setup(1000, 10_000, 3);
    let mut book = OfferBook::new(120_000);
    let id = x.open(&mut book, Limits { mode: Mode::WhileItLasts, cap: Some(1), ..Limits::default() }, 300, 600_000);
    let f0 = x.fans[0].clone();
    let c = book.tap(&id, &f0, &x.w, &mut x.l, &x.e, x.t).unwrap();
    assert_eq!(book.disclose(&id, "f1", &x.l).unwrap().ended, Some(End::CapReached), "held for f0 while f0 checks out");
    assert_eq!(x.budget(), 10_000 - 300);
    assert_eq!(book.checkout(&c.id, "o", c.until + 1, 0).err(), Some(CheckoutError::Expired));
    book.expire_claims(&mut x.l, c.until + 1);
    assert_eq!(x.budget(), 10_000);
    assert_eq!(book.disclose(&id, "f1", &x.l).unwrap().left, Some(1), "back for the next fan");
    assert!(x.conserved());
}

#[test]
fn budget_and_tiers_are_limits_the_fan_can_see() {
    let mut x = setup(1000, 1_000, 10);
    let mut book = OfferBook::new(120_000);
    let limits = Limits { mode: Mode::WhileItLasts, tiers: vec![Tier { units: 2, discount: 300 }, Tier { units: 5, discount: 100 }], ..Limits::default() };
    let id = x.open(&mut book, limits, 300, 600_000);
    let mut seen = vec![];
    for i in 0..10 {
        let d = book.disclose(&id, &format!("f{i}"), &x.l).unwrap();
        seen.push((d.discount_now, d.left, d.ended));
        if d.ended.is_some() { continue; }
        let f = x.fans[i].clone();
        let c = book.tap(&id, &f, &x.w, &mut x.l, &x.e, x.t).unwrap();
        book.checkout(&c.id, &format!("o{i}"), x.t, 0).unwrap();
    }
    // $10 budget: two at $3 off, then $1 off until the budget (not the tier) runs out at four more
    assert_eq!(seen[0], (300, Some(2), None));
    assert_eq!(seen[2].0, 100);
    assert_eq!(seen[2].1, Some(4), "the binding limit is the budget: $4 left at $1 each");
    assert_eq!(seen[6], (100, Some(0), Some(End::BudgetSpent)));
    assert_eq!(x.budget(), 0);
    assert!(x.conserved());
}

#[test]
fn a_condition_ends_the_offer_for_new_taps_but_not_for_claims_in_hand() {
    let mut x = setup(1000, 10_000, 3);
    x.sig("gates.open", fields!{}, "venue");
    for n in 1..=2 { x.sig("period.start", fields!{"n" => n}, "keeper"); x.sig("period.end", fields!{"n" => n}, "keeper"); }
    assert!(x.e.span_open("halftime"));
    let mut book = OfferBook::new(120_000);
    let limits = Limits { mode: Mode::WhileItLasts, while_: Some(Cond::SpanOpen { span: "halftime".into() }), label: Some("Halftime only".into()), ..Limits::default() };
    let id = x.open(&mut book, limits, 300, 3_600_000);
    assert_eq!(book.disclose(&id, "f0", &x.l).unwrap().condition.as_deref(), Some("Halftime only"));
    let f0 = x.fans[0].clone();
    let c = book.tap(&id, &f0, &x.w, &mut x.l, &x.e, x.t).unwrap();
    x.sig("period.start", fields!{"n" => 3}, "keeper");
    book.refresh(&mut x.l, &x.e, x.t);
    assert_eq!(book.disclose(&id, "f1", &x.l).unwrap().ended, Some(End::Condition));
    let f1 = x.fans[1].clone();
    assert_eq!(book.tap(&id, &f1, &x.w, &mut x.l, &x.e, x.t), Err(TapError::Ended(End::Condition)));
    assert!(book.checkout(&c.id, "o", x.t, 0).is_ok(), "f0 tapped during halftime: f0 keeps it");
    assert!(x.conserved());
}

#[test]
fn tiers_cannot_be_guaranteed() {
    let bad = r#"{"rules":[{"name":"r","when":{"match":{"sig":"gates.open"}},
      "then":[{"offer":{"thing":"water","price":550,"payer":"cola","limits":{"tiers":[{"units":5,"discount":100}]}}}]}]}"#;
    let t: Template = serde_json::from_str(bad).unwrap();
    assert!(t.validate(&sports_schema()).is_err());
}

// ---------- points ----------

const DAY: i64 = 86_400_000;

fn earn(l: &mut Ledger, fan: &str, n: i64, at: i64) {
    l.now = at;
    l.post(Txn { memo: "settle".into(), postings: vec![
        posting(acct("agape", "issued"), Unit::Points("agape".into()), -n),
        posting(acct(fan, "points"), Unit::Points("agape".into()), n),
    ]}).unwrap();
}
fn spend(l: &mut Ledger, fan: &str, n: i64, at: i64) {
    l.now = at;
    l.post(Txn { memo: "redeem".into(), postings: vec![
        posting(acct(fan, "points"), Unit::Points("agape".into()), -n),
        posting(acct("agape", "issued"), Unit::Points("agape".into()), n),
    ]}).unwrap();
}

#[test]
fn points_never_expire_by_default() {
    let mut l = Ledger::new(standard_policy());
    earn(&mut l, "f", 500, 0);
    l.now = 50 * 365 * DAY;
    assert_eq!(expire(&mut l, &["f".into()], &PointsPolicy::default()), 0);
    assert_eq!(l.balance(&acct("f", "points"), &Unit::Points("agape".into())), 500);
}

#[test]
fn a_later_expiry_policy_gives_notice_and_spends_oldest_first() {
    let mut l = Ledger::new(standard_policy());
    earn(&mut l, "f", 500, 0);
    earn(&mut l, "f", 300, 200 * DAY);
    spend(&mut l, "f", 400, 250 * DAY);             // takes 400 of the first lot
    assert_eq!(lots(&l, "f"), vec![(0, 100), (200 * DAY, 300)]);
    // Agape turns on one-year expiry on day 300. Old points get a full year from that day.
    let policy = PointsPolicy { expire_after_ms: Some(365 * DAY), effective_from_ms: 300 * DAY };
    l.now = 301 * DAY;
    assert_eq!(expire(&mut l, &["f".into()], &policy), 0, "nothing vanishes the day the policy starts");
    // the fan is told ahead of time what will expire and when
    assert_eq!(expiring(&l, "f", &policy, 665 * DAY), 400);
    assert_eq!(expiring(&l, "f", &policy, 664 * DAY), 0);
    earn(&mut l, "f", 50, 400 * DAY);
    l.now = 665 * DAY;
    assert_eq!(expire(&mut l, &["f".into()], &policy), 400, "exactly what was disclosed");
    assert_eq!(l.balance(&acct("f", "points"), &Unit::Points("agape".into())), 50);
    assert!(l.units().iter().all(|u| l.total(u) == 0));
}

#[test]
fn a_backed_up_stand_pauses_while_it_lasts_offers_visibly() {
    let mut x = setup(1000, 10_000, 3);
    let mut book = OfferBook::new(120_000);
    let id = x.open(&mut book, Limits { mode: Mode::WhileItLasts, ..Limits::default() }, 300, 600_000);
    let f0 = x.fans[0].clone();
    let held = book.tap(&id, &f0, &x.w, &mut x.l, &x.e, x.t).unwrap();
    book.busy.insert("snack-213".into());
    assert_eq!(book.disclose(&id, "f1", &x.l).unwrap().ended, Some(End::Paused), "the card says paused before anyone taps");
    let f1 = x.fans[1].clone();
    assert_eq!(book.tap(&id, &f1, &x.w, &mut x.l, &x.e, x.t), Err(TapError::Ended(End::Paused)));
    assert!(book.checkout(&held.id, "o", x.t, 0).is_ok(), "a claim in hand is kept through a pause");
    book.busy.clear();
    assert!(book.tap(&id, &f1, &x.w, &mut x.l, &x.e, x.t).is_ok(), "and it resumes");
}

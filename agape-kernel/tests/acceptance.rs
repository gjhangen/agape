//! The acceptance test from the substrate doc, as code.

use agape_kernel::commitment::*;
use agape_kernel::economy::*;
use agape_kernel::fields;
use agape_kernel::place::Graph;
use agape_kernel::rules::*;
use agape_kernel::signal::*;
use agape_kernel::value::*;
use rand::{rngs::StdRng, RngExt, SeedableRng};
use std::collections::BTreeMap;

// ---------- shared fixtures ----------

fn schema() -> Schema {
    add_span_kinds(Schema::default())
        // sports
        .kind("gates.open", &[], Some("venue"))
        .kind("period.start", &[("n", Ty::Int)], Some("scorekeeper"))
        .kind("period.end", &[("n", Ty::Int)], Some("scorekeeper"))
        .kind("score", &[("team", Ty::Str), ("points", Ty::Int), ("player", Ty::Str), ("star", Ty::Bool)], Some("scorekeeper"))
        .kind("game.final", &[], Some("scorekeeper"))
        // concerts
        .kind("doors.open", &[], Some("venue"))
        .kind("set.start", &[("n", Ty::Int)], Some("stage"))
        .kind("set.end", &[("n", Ty::Int)], Some("stage"))
        .kind("song", &[("title", Ty::Str), ("tag", Ty::Str)], Some("stage"))
        .kind("house.lights", &[], Some("venue"))
        .kind("crowd.level", &[("zone", Ty::Str), ("db", Ty::Int)], Some("sensor"))
        // shared
        .kind("order.placed", &[("stand", Ty::Str), ("section", Ty::Str)], None)
        .kind("moment", &[("kind", Ty::Str), ("team", Ty::Str)], Some("system"))
        .kind("ops.alert", &[("kind", Ty::Str)], Some("system"))
        .kind("seat.note", &[], None)
}

fn roles() -> Roles {
    let mut r = Roles::default();
    r.grant("venue", "venue");
    r.grant("keeper", "scorekeeper");
    r.grant("stage_mgr", "stage");
    r.grant("mic_a", "sensor");
    r
}

fn template(name: &str) -> Template {
    let src = match name {
        "basketball" => include_str!("../templates/basketball.json"),
        "concert" => include_str!("../templates/concert.json"),
        _ => unreachable!(),
    };
    serde_json::from_str(src).expect("template parses")
}

fn moments(out: &[Output]) -> Vec<(String, String)> {
    out.iter().filter_map(|o| match o {
        Output::Emitted { kind, fields, .. } if kind == "moment" =>
            Some((fields["kind"].key(), fields["team"].key())),
        _ => None,
    }).collect()
}

fn offers(out: &[Output]) -> Vec<String> {
    out.iter().filter_map(|o| match o { Output::Offer { thing, .. } => Some(thing.clone()), _ => None }).collect()
}

struct Game { e: Engine, log: Log, t: i64 }
impl Game {
    fn new(tpl: &str) -> Game {
        Game { e: Engine::load(template(tpl), &schema()).unwrap(), log: Log::new(schema(), roles()), t: 0 }
    }
    fn sig(&mut self, kind: &str, f: Fields, by: &str) -> Vec<Output> {
        self.t += 1000;
        self.e.ingest(&mut self.log, kind, f, by, self.t).unwrap()
    }
    fn score(&mut self, team: &str, pts: i64, player: &str) -> Vec<Output> {
        self.sig("score", fields!{"team" => team, "points" => pts, "player" => player, "star" => player == "star"}, "keeper")
    }
}

// ---------- 1. the concert test ----------

#[test]
fn concert_test_same_kernel_different_template() {
    let mut g = Game::new("concert");
    let mut out = vec![];
    out.extend(g.sig("doors.open", fields!{}, "venue"));
    out.extend(g.sig("set.start", fields!{"n" => 1}, "stage_mgr"));
    out.extend(g.sig("song", fields!{"title" => "Opener", "tag" => "deep"}, "stage_mgr"));
    out.extend(g.sig("set.end", fields!{"n" => 1}, "stage_mgr"));
    assert!(offers(&out).is_empty(), "no tee before the headliner set");
    out.extend(g.sig("set.start", fields!{"n" => 2}, "stage_mgr"));
    assert_eq!(offers(&out), vec!["tour_tee"]);
    out.extend(g.sig("song", fields!{"title" => "The Big One", "tag" => "hit"}, "stage_mgr"));
    for _ in 0..3 { out.extend(g.sig("crowd.level", fields!{"zone" => "floor", "db" => 101}, "mic_a")); }
    assert_eq!(offers(&out), vec!["tour_tee", "water"]);
    out.extend(g.sig("set.end", fields!{"n" => 2}, "stage_mgr"));
    out.extend(g.sig("song", fields!{"title" => "Last Song", "tag" => "encore"}, "stage_mgr"));
    assert!(g.e.span_open("encore"));
    out.extend(g.sig("house.lights", fields!{}, "venue"));
    assert!(!g.e.span_open("show") && !g.e.span_open("encore"), "closing the show closes the encore");
    assert_eq!(moments(&out), vec![("singalong".into(), "".into()), ("encore".into(), "".into())]);
}

#[test]
fn kernel_has_no_domain_words() {
    let src = [include_str!("../src/rules.rs"), include_str!("../src/signal.rs"), include_str!("../src/value.rs"),
               include_str!("../src/economy.rs"), include_str!("../src/commitment.rs"), include_str!("../src/place.rs"),
               include_str!("../src/world.rs"), include_str!("../src/engines.rs"), include_str!("../src/offers.rs"), include_str!("../src/points.rs")].concat()
        .to_lowercase();
    for w in ["basketball", "quarter", "halftime", "tipoff", "inning", "encore", "setlist", "touchdown"] {
        assert!(!src.contains(w), "kernel mentions '{w}'");
    }
}

// ---------- 2. moments from the rule language ----------

#[test]
fn run_lead_change_and_splash() {
    let mut g = Game::new("basketball");
    g.sig("gates.open", fields!{}, "venue");
    g.sig("period.start", fields!{"n" => 1}, "keeper");
    let mut out = vec![];
    out.extend(g.score("away", 2, "x"));
    out.extend(g.score("home", 3, "star"));   // lead change: away -> home
    out.extend(g.score("home", 2, "y"));
    out.extend(g.score("home", 3, "star"));   // 8 unanswered: run; 2nd star three: splash
    out.extend(g.score("home", 2, "y"));      // still the same run: no second run moment
    let m = moments(&out);
    assert_eq!(m, vec![
        ("lead_change".into(), "home".into()),
        ("run".into(), "home".into()),
        ("splash".into(), "home".into()),
    ]);
    assert_eq!(offers(&out), vec!["foam_finger"], "a home run moment drives a stratum-2 offer");
    assert_eq!(g.e.measure("margin", g.t), Some(8));
    // away answers: the streak resets, and a new away run can fire later
    let mut out = vec![];
    for _ in 0..4 { out.extend(g.score("away", 2, "x")); }
    assert_eq!(moments(&out), vec![("run".into(), "away".into())]);
    for _ in 0..3 { out.extend(g.score("away", 2, "x")); }
    assert!(moments(&out).contains(&("lead_change".into(), "away".into())));
}

#[test]
fn halftime_is_a_span_derived_from_periods() {
    let mut g = Game::new("basketball");
    g.sig("gates.open", fields!{}, "venue");
    let mut out = vec![];
    for n in 1..=2 {
        out.extend(g.sig("period.start", fields!{"n" => n}, "keeper"));
        out.extend(g.sig("period.end", fields!{"n" => n}, "keeper"));
    }
    assert!(g.e.span_open("halftime"));
    let o: Vec<_> = out.iter().filter_map(|o| match o { Output::Offer { thing, discount, payer, .. } => Some((thing.clone(), *discount, payer.clone())), _ => None }).collect();
    assert_eq!(o, vec![("combo".into(), 300, Some("sponsor_cola".into()))], "the discount names its payer");
    g.sig("period.start", fields!{"n" => 3}, "keeper");
    assert!(!g.e.span_open("halftime"));
    g.sig("game.final", fields!{}, "keeper");
    assert!(!g.e.span_open("period") && !g.e.span_open("game"));
}

#[test]
fn windowed_measure_crosses_once_then_rearms() {
    let mut g = Game::new("basketball");
    let mut alerts = 0;
    for _ in 0..25 {
        g.t += 1000 - 1000; // same second bucket area: 25 orders in ~25s
        let o = g.sig("order.placed", fields!{"stand" => "grill", "section" => "104"}, "fan1");
        alerts += o.iter().filter(|o| matches!(o, Output::Emitted { kind, .. } if kind == "ops.alert")).count();
    }
    assert_eq!(alerts, 1, "fires on the crossing, not on every order above it");
    g.t += 120_000; // the window empties
    g.sig("order.placed", fields!{"stand" => "grill", "section" => "104"}, "fan1");
    assert!(g.e.measure("orders_1m", g.t).unwrap() <= 1);
    let mut again = 0;
    for _ in 0..25 {
        let o = g.sig("order.placed", fields!{"stand" => "grill", "section" => "104"}, "fan1");
        again += o.iter().filter(|o| matches!(o, Output::Emitted { kind, .. } if kind == "ops.alert")).count();
    }
    assert_eq!(again, 1);
}

// ---------- 3. load-time guarantees ----------

#[test]
fn templates_are_typed_and_stratified() {
    let s = schema();
    let bad = |json: &str| serde_json::from_str::<Template>(json).unwrap().validate(&s).unwrap_err();
    assert!(matches!(bad(r#"{"rules":[{"name":"r","when":{"match":{"sig":"score","where":{"colour":{"eq":"red"}}}},"then":[]}]}"#),
        LoadError::UnknownField { .. }));
    assert!(matches!(bad(r#"{"rules":[{"name":"r","when":{"match":{"sig":"score","where":{"team":{"gte":3}}}},"then":[]}]}"#),
        LoadError::WrongType { .. }));
    assert!(matches!(bad(r#"{"rules":[{"name":"r","when":{"match":{"sig":"scroe"}},"then":[]}]}"#),
        LoadError::UnknownKind(_)));
    // a rule reading moments at the same stratum that emits them could loop forever: rejected
    assert!(matches!(bad(r#"{"rules":[{"name":"echo","when":{"match":{"sig":"moment"}},
        "then":[{"emit":{"kind":"moment","fields":{"kind":"x","team":""}}}]}]}"#), LoadError::StratumViolation { .. }));
    // A7: a discount with nobody paying for it cannot be written
    assert!(matches!(bad(r#"{"rules":[{"name":"free","when":{"match":{"sig":"gates.open"}},
        "then":[{"offer":{"thing":"beer","price":1200,"discount":1200}}]}]}"#), LoadError::DiscountWithoutPayer(_)));
    assert!(matches!(bad(r#"{"spans":[{"name":"loop","repeat":true,"opens":{"sig":"span.exit","where":{"span":{"eq":"loop"}}},
        "closes":{"sig":"gates.open"}}]}"#), LoadError::SelfOpeningSpan(_)));
}

#[test]
fn replay_is_deterministic() {
    let mut rng = StdRng::seed_from_u64(7);
    let mut g = Game::new("basketball");
    let mut out = g.sig("gates.open", fields!{}, "venue");
    for n in 1..=4 {
        out.extend(g.sig("period.start", fields!{"n" => n}, "keeper"));
        for _ in 0..60 {
            let team = if rng.random_bool(0.5) { "home" } else { "away" };
            let pts = [1, 2, 2, 2, 3][rng.random_range(0..5)];
            let player = if rng.random_bool(0.3) { "star" } else { "x" };
            out.extend(g.score(team, pts, player));
            if rng.random_bool(0.4) { out.extend(g.sig("order.placed", fields!{"stand" => "grill", "section" => "104"}, "fan")); }
        }
        out.extend(g.sig("period.end", fields!{"n" => n}, "keeper"));
    }
    out.extend(g.sig("game.final", fields!{}, "keeper"));
    let (e2, log2, out2) = Engine::replay(template("basketball"), schema(), roles(), &g.log.signals);
    assert_eq!(out, out2);
    assert_eq!(g.log.signals.len(), log2.signals.len());
    assert_eq!(g.e.measure("margin", g.t), e2.measure("margin", g.t));
    assert!(moments(&out).len() > 5, "a real game produces moments");
}

// ---------- 4. authority (A5) ----------

#[test]
fn claims_are_kept_but_never_acted_on() {
    let mut g = Game::new("basketball");
    g.sig("gates.open", fields!{}, "venue");
    let mut out = vec![];
    for _ in 0..5 { out.extend(g.score("home", 3, "star")); }
    let before = g.e.measure("margin", g.t);
    // a fan's phone claims the away team scored 20; it is logged, not counted
    let o = g.sig("score", fields!{"team" => "away", "points" => 20, "player" => "x", "star" => false}, "fan1");
    assert!(o.is_empty());
    assert_eq!(g.log.signals.last().unwrap().authority, Authority::Claim);
    assert_eq!(g.e.measure("margin", g.t), before);
    // and claims cannot open spans
    let mut g2 = Game::new("concert");
    g2.sig("doors.open", fields!{}, "fan1");
    assert!(!g2.e.span_open("show"));
}

#[test]
fn commitment_stages_move_only_by_their_owner() {
    let t = Terms::default();
    let mut l = Ledger::new(standard_policy());
    stock(&mut l, "grill", "burger", 10).unwrap();
    let q = quote("fan1", "grill", 1, 1350, vec![], 0, 300, true);
    let mut c = Commitment::accept("o1", q, &mut l).unwrap();
    assert!(matches!(c.act(Action::MarkReady, "fan1", &mut l, &t), Err(CommitError::NotOwner { .. })));
    assert!(matches!(c.act(Action::MarkReady, "grill", &mut l, &t), Err(CommitError::WrongStage { .. })));
    c.act(Action::StartPrep, "grill", &mut l, &t).unwrap();
    assert!(matches!(c.act(Action::Cancel, "fan1", &mut l, &t), Err(CommitError::WrongStage { .. })), "too late to cancel once cooking");
    c.act(Action::MarkReady, "grill", &mut l, &t).unwrap();
    c.act(Action::Assign("r1".into()), "agape", &mut l, &t).unwrap();
    assert!(matches!(c.act(Action::Deliver, "r2", &mut l, &t), Err(CommitError::NotOwner { .. })), "another runner cannot deliver it");
    c.act(Action::PickUp, "r1", &mut l, &t).unwrap();
    c.act(Action::Deliver, "r1", &mut l, &t).unwrap();
    c.act(Action::Settle, "agape", &mut l, &t).unwrap();
    assert_eq!(c.stage, Stage::Settled);
    assert_eq!(l.balance(&acct("r1", "tips"), &Unit::usd()), 300);
    assert_eq!(l.balance(&acct("fan1", "holdings"), &Unit::Thing("burger".into())), 1);
}

// ---------- 5. erasure (A4) ----------

#[test]
fn erasure_destroys_the_key_not_the_history() {
    let mut v = Vault::default();
    let mut log = Log::new(schema(), roles());
    let sealed = v.seal("fan1", b"Section 104, row F, seat 7. Nut allergy.");
    log.append_full("seat.note", fields!{}, "fan1", 1, 0, Some(sealed)).unwrap();
    let other = v.seal("fan2", b"Section 210");
    log.append_full("seat.note", fields!{}, "fan2", 2, 0, Some(other)).unwrap();
    let s1 = log.signals[0].sealed.clone().unwrap();
    assert!(v.open(&s1).unwrap().starts_with(b"Section 104"));
    assert!(v.erase("fan1"));
    assert_eq!(v.open(&s1), Err(VaultError::Erased));
    assert_eq!(log.signals.len(), 2, "the log is append-only; nothing was rewritten");
    assert!(v.open(log.signals[1].sealed.as_ref().unwrap()).is_ok(), "other people are untouched");
    assert!(!String::from_utf8_lossy(&s1.ct).contains("104"));
}

// ---------- 6. conservation ----------

fn quote(fan: &str, stand: &str, qty: i64, price: i64, discounts: Vec<Discount>, points: i64, tip: i64, delivery: bool) -> Quote {
    Quote { fan: fan.into(), stand: stand.into(), runner: None, lines: vec![Line { thing: "burger".into(), qty, unit_price: price }],
            discounts, points_redeemed: points, tip, delivery, fee_waived: false }
}

#[test]
fn bankers_rounding_is_exact() {
    assert_eq!(bps(50, 100), 0);     // 0.5 -> 0 (even)
    assert_eq!(bps(150, 100), 2);    // 1.5 -> 2
    assert_eq!(bps(250, 100), 2);    // 2.5 -> 2
    assert_eq!(bps(1350, 700), 94);  // 94.5 -> 94
    assert_eq!(bps(-150, 100), -2);
    assert_eq!(bps(i64::MAX / 2, 10_000), i64::MAX / 2);
}

#[test]
fn ledger_refuses_what_does_not_balance() {
    let mut l = Ledger::new(standard_policy());
    let usd = Unit::usd();
    let r = l.post(Txn { memo: "leak".into(), postings: vec![posting(acct("a", "card"), usd.clone(), -100), posting(acct("b", "revenue"), usd.clone(), 99)] });
    assert!(matches!(r, Err(LedgerError::Unbalanced { .. })));
    // a sponsor cannot spend budget it has not funded
    let r = l.post(Txn { memo: "overspend".into(), postings: vec![posting(acct("s", "promo_budget"), usd.clone(), -1), posting(acct("f", "card"), usd, 1)] });
    assert!(matches!(r, Err(LedgerError::Insufficient { .. })));
    // selling a burger the stand does not have is refused whole
    let t = Terms::default();
    let q = quote("f", "grill", 1, 1000, vec![], 0, 0, false);
    let b = breakdown(&q, &t).unwrap();
    assert!(l.post(settlement(&q, &t, &b)).is_err());
    assert!(l.txns.is_empty(), "nothing partial was applied");
}

#[test]
fn conservation_sweep_over_every_exit() {
    let t = Terms::default();
    let mut l = Ledger::new(standard_policy());
    let mut rng = StdRng::seed_from_u64(42);
    stock(&mut l, "grill", "burger", 1_000_000).unwrap();
    fund(&mut l, "sponsor_cola", 3_000_00).unwrap(); // small on purpose: it runs dry mid-sweep
    fund(&mut l, "venue", 50_000_00).unwrap();
    let mut exits: BTreeMap<String, usize> = BTreeMap::new();
    let mut points_held = 0i64;
    for i in 0..20_000 {
        let fan = format!("fan{}", i % 500);
        let price = rng.random_range(300..5000);
        let qty = rng.random_range(1..4);
        let mut discounts = vec![];
        if rng.random_bool(0.3) {
            let payer = if rng.random_bool(0.5) { "sponsor_cola" } else { "venue" };
            discounts.push(Discount { amount: rng.random_range(0..=price), payer: payer.into(), reason: "deal".into() });
        }
        let have = l.balance(&acct(&fan, "points"), &Unit::Points("agape".into()));
        let points = if have > 0 && rng.random_bool(0.2) { rng.random_range(0..=have.min(price * qty - discounts.iter().map(|d| d.amount).sum::<i64>())) } else { 0 };
        let delivery = rng.random_bool(0.7);
        let q = quote(&fan, "grill", qty, price, discounts, points, if rng.random_bool(0.6) { rng.random_range(0..600) } else { 0 }, delivery);
        let mut c = match Commitment::accept(&format!("o{i}"), q, &mut l) {
            Ok(c) => c,
            // a sponsor out of budget or a fan short on points is refused at acceptance, never later
            Err(LedgerError::Insufficient { .. }) => { *exits.entry("refused at accept".into()).or_insert(0) += 1; continue; }
            Err(e) => panic!("{e:?}"),
        };
        let roll = rng.random_range(0..100);
        let r: Result<(), CommitError> = (|| {
            if roll < 4 { c.act(Action::Cancel, &fan, &mut l, &t)?; return Ok(()); }
            if roll < 6 { c.act(Action::PaymentFailed, "agape", &mut l, &t)?; return Ok(()); }
            c.act(Action::StartPrep, "grill", &mut l, &t)?;
            if roll < 9 { c.act(Action::CannotFulfil, "grill", &mut l, &t)?; return Ok(()); }
            c.act(Action::MarkReady, "grill", &mut l, &t)?;
            if delivery {
                c.act(Action::Assign("r1".into()), "agape", &mut l, &t)?;
                if roll < 14 { c.act(Action::RunnerDropped, "agape", &mut l, &t)?; c.act(Action::Assign("r2".into()), "agape", &mut l, &t)?; }
                let runner = c.quote.runner.clone().unwrap();
                c.act(Action::PickUp, &runner, &mut l, &t)?;
                if roll < 18 { c.act(Action::Unreachable, &runner, &mut l, &t)?; return Ok(()); }
                if roll < 25 { c.late_by = Some((if roll % 2 == 0 { "agape".into() } else { "grill".into() }, 600)); }
                c.act(Action::Deliver, &runner, &mut l, &t)?;
            } else {
                c.act(Action::Deliver, "grill", &mut l, &t)?;
            }
            if roll < 31 {
                c.act(Action::Dispute, &fan, &mut l, &t)?;
                c.act(Action::Resolve { upheld: roll % 2 == 0 }, "agape", &mut l, &t)?;
                return Ok(());
            }
            c.act(Action::Settle, "agape", &mut l, &t)?;
            Ok(())
        })();
        match r {
            Ok(()) => {}
            Err(e) => panic!("order {i}: {e:?}"),
        }
        let k = format!("{:?}", c.stage);
        *exits.entry(k.split('{').next().unwrap().trim().to_string()).or_insert(0) += 1;
        points_held = l.balance(&acct(&fan, "points"), &Unit::Points("agape".into()));
        assert!(points_held >= 0);
    }
    let _ = points_held;
    // conservation: every unit sums to zero across the whole system
    for u in l.units() { assert_eq!(l.total(&u), 0, "unit {u:?} leaked"); }
    // every exit was exercised
    for e in ["Settled", "Closed(FanCancelled)", "Closed(PaymentFailed)", "Closed(StandCannotFulfil)", "Closed(FanUnreachable)", "Closed(DisputeUpheld"] {
        assert!(exits.keys().any(|k| k.starts_with(e.trim_end_matches('('))), "exit {e} never ran: {exits:?}");
    }
    // inventory moved exactly as many burgers as fans now hold
    let held: i64 = (0..500).map(|i| l.balance(&acct(&format!("fan{i}"), "holdings"), &Unit::Thing("burger".into()))).sum();
    let b = |p: &str| l.balance(&acct("grill", p), &Unit::Thing("burger".into()));
    assert_eq!(1_000_000 - b("inventory") - b("written_off"), held);
    assert_eq!(b("reserved"), 0, "every hold was captured or released");
    assert_eq!(l.balance(&acct("sponsor_cola", "promo_escrow"), &Unit::usd()), 0);
    assert!(exits.contains_key("refused at accept"), "the small budget ran out: {exits:?}");
    assert!(l.balance(&acct("sponsor_cola", "promo_budget"), &Unit::usd()) >= 0);
    eprintln!("exits: {exits:?}, txns: {}", l.txns.len());
}

// ---------- 7. the Agape invariant, ex-ante (A3) ----------

#[test]
fn offers_that_make_someone_lose_are_never_shown() {
    let t = Terms::default();
    let mut values: BTreeMap<PartyId, ValueFn> = BTreeMap::new();
    values.insert("grill".into(), ValueFn::Floor(0));
    values.insert("agape".into(), ValueFn::Floor(0));
    values.insert("r1".into(), ValueFn::Floor(0));
    let mut q = quote("fan1", "grill", 1, 1350, vec![], 0, 300, true);
    q.runner = Some("r1".into());
    values.insert("fan1".into(), ValueFn::Floor(-2500));
    let b = agape_check(&q, &t, &values).unwrap();
    assert_eq!(b.charge, 1350 + 94 + 499 + 300);
    // the fan said $25 is their ceiling; this order costs more: not shown
    values.insert("fan1".into(), ValueFn::Floor(-2000));
    assert!(matches!(agape_check(&q, &t, &values), Err(OfferError::Loses { party, .. }) if party == "fan1"));
    values.insert("fan1".into(), ValueFn::Floor(-2500));
    // redeeming 1,000 points on a pickup order: Agape would fund more than it earns
    let mut q2 = quote("fan1", "grill", 1, 1350, vec![], 1000, 0, false);
    q2.runner = None;
    assert!(matches!(agape_check(&q2, &t, &values), Err(OfferError::Loses { party, .. }) if party == "agape"));
    // a sponsor-paid discount leaves the stand whole: the stand is paid on the full price
    let q3 = quote("fan1", "grill", 1, 1350, vec![Discount { amount: 1350, payer: "sponsor_cola".into(), reason: "free".into() }], 0, 0, false);
    let b3 = agape_check(&q3, &t, &values).unwrap();
    assert_eq!(b3.stand_net, 1350 - bps(1350, 1500));
    // and the sponsor's ceiling binds too
    values.insert("sponsor_cola".into(), ValueFn::Floor(-1000));
    assert!(matches!(agape_check(&q3, &t, &values), Err(OfferError::Loses { party, .. }) if party == "sponsor_cola"));
}

// ---------- 8. place: scope and path ----------

#[test]
fn paths_respect_closures_and_accessibility() {
    let mut g = Graph::default();
    g.add("arena", "venue", None);
    for (id, parent) in [("L1", "arena"), ("L2", "arena"), ("c1", "L1"), ("c2", "L1"), ("stairs", "arena"), ("lift", "arena"), ("s104", "c2"), ("s210", "L2"), ("grill", "c1")] {
        g.add(id, "node", Some(parent));
    }
    g.link("grill", "c1", 10, true);
    g.link("c1", "c2", 60, true);
    g.link("c2", "s104", 25, true);
    g.link("c1", "stairs", 15, false);
    g.link("stairs", "s210", 40, false);
    g.link("c2", "lift", 20, true);
    g.link("lift", "s210", 90, true);
    assert_eq!(g.path("grill", "s104", false).unwrap().0, 95);
    assert_eq!(g.path("grill", "s210", false).unwrap().0, 65);
    assert_eq!(g.path("grill", "s210", true).unwrap().0, 180, "step-free takes the lift");
    g.close("c2");
    assert!(g.path("grill", "s104", false).is_none(), "closing the concourse cuts the section off");
    assert!(g.within("s104", "L1") && !g.within("s210", "L1"));
}

#[test]
fn an_accepted_deal_survives_the_sponsor_running_dry() {
    let t = Terms::default();
    let mut l = Ledger::new(standard_policy());
    stock(&mut l, "grill", "burger", 10).unwrap();
    fund(&mut l, "cola", 500).unwrap();
    let deal = |fan: &str| quote(fan, "grill", 1, 1000, vec![Discount { amount: 500, payer: "cola".into(), reason: "deal".into() }], 0, 0, false);
    let mut a = Commitment::accept("a", deal("fan1"), &mut l).unwrap();
    // the budget is now fully held for fan1; fan2 is refused at the door, not at settlement
    assert!(matches!(Commitment::accept("b", deal("fan2"), &mut l), Err(LedgerError::Insufficient { .. })));
    a.act(Action::StartPrep, "grill", &mut l, &t).unwrap();
    a.act(Action::MarkReady, "grill", &mut l, &t).unwrap();
    a.act(Action::Deliver, "grill", &mut l, &t).unwrap();
    a.act(Action::Settle, "agape", &mut l, &t).unwrap();
    assert_eq!(l.balance(&acct("cola", "promo_escrow"), &Unit::usd()), 0);
    assert_eq!(l.balance(&acct("cola", "promo_budget"), &Unit::usd()), 0);
    // a cancelled deal gives its budget back
    fund(&mut l, "cola", 500).unwrap();
    let mut c = Commitment::accept("c", deal("fan3"), &mut l).unwrap();
    c.act(Action::Cancel, "fan3", &mut l, &t).unwrap();
    assert_eq!(l.balance(&acct("cola", "promo_budget"), &Unit::usd()), 500);
    assert_eq!(l.balance(&acct("grill", "inventory"), &Unit::Thing("burger".into())), 9);
}

#[test]
fn questions_are_rules_too_so_a_concert_can_ask_its_own() {
    let mut g = Game::new("concert");
    let mut out = vec![];
    out.extend(g.sig("doors.open", fields!{}, "venue"));
    out.extend(g.sig("set.start", fields!{"n" => 1}, "stage_mgr"));
    out.extend(g.sig("set.end", fields!{"n" => 1}, "stage_mgr"));
    out.extend(g.sig("set.start", fields!{"n" => 2}, "stage_mgr"));
    let asked = out.iter().find_map(|o| match o { Output::Ask { question, options, .. } => Some((question.clone(), options.len())), _ => None });
    assert_eq!(asked, Some(("Will there be an encore?".to_string(), 2)));
    out.extend(g.sig("song", fields!{"title" => "Last Song", "tag" => "encore"}, "stage_mgr"));
    assert_eq!(g.e.measure("encore_songs", g.t), Some(1), "the measure that decides it moved");
    out.extend(g.sig("house.lights", fields!{}, "venue"));
    assert!(out.iter().any(|o| matches!(o, Output::Settle { ask, .. } if ask == "encore")));
    // and the template is checked: a question tied to a measure that doesn't exist is refused
    let bad = r#"{"rules":[{"name":"q","when":{"match":{"sig":"gates.open"}},"then":[{"ask":{"id":"x","question":"?","options":[{"id":"a","measure":"nope"},{"id":"b"}],"locks_ms":1,"points":1}}]}]}"#;
    assert!(serde_json::from_str::<Template>(bad).unwrap().validate(&schema()).is_err());
}

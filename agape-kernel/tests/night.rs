//! A full simulated game night through every layer: kernel, world, engines, adapter.

use agape_kernel::rules::Template;
use agape_kernel::sim::{run, Config, Report};
use agape_kernel::world::WorldDef;

fn night(seed: u64) -> Report { night_with(seed, Config::default()) }

fn night_with(seed: u64, cfg: Config) -> Report {
    let world: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let tpl: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    run(world, tpl, &Config { seed, ..cfg })
}

#[test]
fn a_whole_night_conserves_and_closes_every_order() {
  for seed in 1..=8 {
    let r = night(seed);
    assert!(r.conserved, "every unit sums to zero");
    assert_eq!(r.tap_failed_after_live_card, 0, "no fan tapped a live card and lost it");
    assert_eq!(r.checkout_failed, 0, "no claim failed at checkout");
    assert_eq!(r.refused_at_accept, 0, "every offer shown could be honoured when accepted");
    assert_eq!(r.open_holds, 0, "no budget or points left on hold after the night");
    let terminal = ["Settled", "FanCancelled", "PaymentFailed", "StandCannotFulfil", "FanUnreachable", "DisputeUpheld"];
    for (stage, n) in &r.by_stage { assert!(terminal.contains(&stage.as_str()), "{n} orders stuck in {stage}"); }
    assert!(r.orders > 1000, "a real night: {}", r.orders);
    // attribution reconciles to the ledger, per payer
    let mut attr_spend = std::collections::BTreeMap::new();
    for (src, s) in &r.attribution {
        let payer = match src.as_str() { "halftime_combo" => "sponsor_cola", "water_break" => "sponsor_water", _ => continue };
        *attr_spend.entry(payer.to_string()).or_insert(0) += s.spend;
    }
    for (payer, spend) in &r.sponsor_spend_ledger {
        assert_eq!(attr_spend.get(payer).copied().unwrap_or(0), *spend, "{payer}: attribution vs ledger");
    }
    // the rule language drove the night
    assert!(r.moments.iter().any(|m| m.1 == "run") && r.moments.iter().any(|m| m.1 == "last_call"));
    // the halftime deal ran, or was withheld because its kitchen could not deliver in time
    assert!(r.attribution.get("halftime_combo").map_or(false, |s| (s.shown > 0 && s.settled > 0) || s.refused.contains_key("StandBusy")));
    // age-restricted things were never shown or sold to unverified fans (checked in eligibility)
    assert!(r.attribution.values().all(|s| !s.refused.contains_key("UnknownThing")));
  }
}

#[test]
fn both_offer_modes_keep_every_promise() {
    use agape_kernel::rules::Mode;
    for mode in [Mode::Guaranteed, Mode::WhileItLasts] {
        for seed in 1..=4 {
            let r = night_with(seed, Config { force_mode: Some(mode), ..Config::default() });
            assert!(r.conserved && r.open_holds == 0, "{mode:?} seed {seed}");
            assert_eq!((r.tap_failed_after_live_card, r.checkout_failed, r.refused_at_accept), (0, 0, 0), "{mode:?} seed {seed}");
            assert!(r.taps > 0);
        }
    }
}

#[test]
fn same_seed_same_night() {
    let (a, b) = (night(9), night(9));
    assert_eq!(a.orders, b.orders);
    assert_eq!(a.money, b.money);
    assert_eq!(a.moments, b.moments);
}

#[test]
fn a_late_backlog_is_batched_not_left_waiting() {
    use agape_kernel::engines::{dispatch, Costs, Ready, Runner};
    use agape_kernel::world::World;
    let w = World::build(serde_json::from_str(include_str!("../templates/arena.json")).unwrap());
    let mut costs = Costs::new();
    // five orders at one stand, the oldest already 20 minutes past ready; one free runner
    let ready: Vec<Ready> = (0..5).map(|i| Ready { order: i.to_string(), stand: "grill-104".into(), seat: format!("s10{}", 3 + i % 3), step_free: false, ready_at: -1_200_000 + i as i64 * 1000 }).collect();
    let runners = vec![Runner { id: "r".into(), at: "grill-104".into(), free_at: 0 }];
    let a = dispatch(&w, &mut costs, &ready, &runners, 0);
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].orders.len(), 5.min(w.def.service.batch_max), "up to batch_max orders go together");
    assert_eq!(a[0].orders[0], "0", "oldest first");
    assert!(a[0].arrive_s.windows(2).all(|p| p[0] <= p[1]));
    // a step-free fan upstairs is routed by the lift
    let up = vec![Ready { order: "u".into(), stand: "grill-104".into(), seat: "s213".into(), step_free: true, ready_at: 0 }];
    let a = dispatch(&w, &mut costs, &up, &runners, 0);
    let stairs = w.graph.path("grill-104", "s213", false).unwrap().0;
    let lift = w.graph.path("grill-104", "s213", true).unwrap();
    assert!(lift.1.iter().any(|p| p.starts_with("lift")) && !lift.1.iter().any(|p| p.starts_with("stairs")));
    assert_eq!(a[0].arrive_s[0], lift.0);
    assert!(lift.0 > stairs, "the step-free route is the longer lift route");
}

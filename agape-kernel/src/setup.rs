//! Setup: a venue is onboarded by filling in its template. Before it opens, the kernel checks the
//! template (everything it names exists, every number is sane, and every item on the menu can be
//! sold without anyone losing on it) and runs a whole simulated night on it, so the operator sees
//! waits, money and problems before a single fan does.

use crate::economy::{breakdown, settlement, Line, Quote};
use crate::rules::Template;
use crate::sim::{Config, Night};
use crate::world::{Constraint, WorldDef};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Issue { pub path: String, pub message: String, pub blocking: bool }

fn err(path: impl Into<String>, m: impl Into<String>) -> Issue { Issue { path: path.into(), message: m.into(), blocking: true } }
fn warn(path: impl Into<String>, m: impl Into<String>) -> Issue { Issue { path: path.into(), message: m.into(), blocking: false } }

/// Every string under a "thing" key anywhere in the event template: what its rules offer.
fn template_things(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(m) => for (k, x) in m { if k == "thing" { if let Some(s) = x.as_str() { out.insert(s.into()); } } template_things(x, out); },
        Value::Array(a) => for x in a { template_things(x, out); },
        _ => {}
    }
}

pub fn check(w: &WorldDef, tpl: &Template) -> Vec<Issue> {
    let mut v = vec![];
    if w.venue.trim().is_empty() { v.push(err("venue", "Give the venue an id")); }
    if w.name.trim().is_empty() { v.push(err("name", "Name the venue the way fans know it")); }
    if !(1..=6).contains(&w.levels) { v.push(err("levels", "A venue has 1 to 6 levels")); }
    if !(4..=60).contains(&w.sections_per_level) { v.push(err("sections_per_level", "4 to 60 sections a level")); }
    if w.ring_s == 0 || w.aisle_s == 0 { v.push(err("ring_s", "Walking times must be more than zero")); }
    // stands
    let mut ids = BTreeSet::new();
    for (i, s) in w.stands.iter().enumerate() {
        let p = format!("stands[{i}]");
        if s.id.trim().is_empty() || !ids.insert(s.id.clone()) { v.push(err(&p, format!("Each stand needs its own id ({})", s.id))); }
        if s.name.trim().is_empty() { v.push(err(&p, "Name the stand")); }
        if s.level < 1 || s.level > w.levels { v.push(err(&p, format!("{} is on level {}, but the venue has {} levels", s.name, s.level, w.levels))); }
        if s.ring >= w.sections_per_level { v.push(err(&p, format!("{} sits by section {}, but a level has {} sections", s.name, s.ring + 1, w.sections_per_level))); }
        if !(1..=12).contains(&s.stations) { v.push(err(&p, format!("{} needs 1 to 12 stations", s.name))); }
    }
    if w.stands.is_empty() { v.push(err("stands", "Add at least one stand")); }
    // menu
    let mut things = BTreeSet::new();
    let mut used_stands = BTreeSet::new();
    for (i, t) in w.things.iter().enumerate() {
        let p = format!("things[{i}]");
        if t.id.trim().is_empty() || !things.insert(t.id.clone()) { v.push(err(&p, format!("Each item needs its own id ({})", t.id))); }
        if !ids.contains(&t.stand) { v.push(err(&p, format!("{} is sold at {}, which isn't a stand here", t.name, t.stand))); }
        used_stands.insert(t.stand.clone());
        if t.price <= 0 || t.price > 500_00 { v.push(err(&p, format!("{} needs a price between $0.01 and $500", t.name))); }
        if !(10..=3600).contains(&t.prep_s) { v.push(err(&p, format!("{} takes 10 seconds to an hour to make", t.name))); }
        if !matches!(t.category.as_str(), "food" | "drink" | "merch") { v.push(err(&p, format!("{} must be food, drink or merch", t.name))); }
        for c in &t.constraints {
            match c {
                Constraint::MinAge(a) if *a < 16 || *a > 25 => v.push(err(&p, format!("{}: an age limit between 16 and 25", t.name))),
                Constraint::Levels(ls) if ls.iter().any(|l| *l < 1 || *l > w.levels) => v.push(err(&p, format!("{} is limited to a level that doesn't exist", t.name))),
                _ => {}
            }
        }
        if t.stock == Some(0) { v.push(warn(&p, format!("{} starts sold out", t.name))); }
    }
    for s in &w.stands { if !used_stands.contains(&s.id) { v.push(warn("stands", format!("{} has nothing on its menu", s.name))); } }
    // the event template can only offer what the venue sells
    let mut offered = BTreeSet::new();
    template_things(&serde_json::to_value(tpl).unwrap_or(Value::Null), &mut offered);
    for t in offered { if !things.contains(&t) { v.push(err("template", format!("The event's rules offer \"{t}\", which isn't on this venue's menu"))); } }
    // terms
    let tm = &w.terms;
    if !(0..=5000).contains(&tm.platform_share_bps) { v.push(err("terms.platform_share_bps", "Agape's share is 0% to 50%")); }
    if !(0..=2000).contains(&tm.tax_bps) { v.push(err("terms.tax_bps", "Sales tax is 0% to 20%")); }
    if tm.delivery_fee < 0 || tm.runner_pay < 0 { v.push(err("terms", "Fees and pay can't be negative")); }
    if tm.runner_pay > tm.delivery_fee { v.push(warn("terms.runner_pay", "Runners are paid more per delivery than the delivery fee; Agape's share covers the rest")); }
    // invariant 1 at the template: every item, ordered alone, leaves every party at zero or better
    for t in &w.things {
        for delivery in [true, false] {
            let q = Quote { fan: "fan".into(), stand: t.stand.clone(), runner: if delivery { Some("runner".into()) } else { None },
                lines: vec![Line { thing: t.id.clone(), qty: 1, unit_price: t.price }], discounts: vec![], points_redeemed: 0, tip: 0, delivery };
            let Ok(b) = breakdown(&q, tm) else { continue };
            let s = settlement(&q, tm, &b);
            let agape: i64 = s.postings.iter().filter(|p| p.account.party == "agape" && p.account.purpose == "revenue").map(|p| p.amount).sum();
            if agape < 0 {
                v.push(err(format!("things.{}", t.id), format!("{} ordered alone for {} loses Agape {}¢ after card fees{}; raise the price, the fee or Agape's share",
                    t.name, if delivery { "delivery" } else { "pickup" }, -agape, if delivery { " and runner pay" } else { "" })));
            }
        }
    }
    // around the venue
    let biz: BTreeSet<&str> = w.businesses.iter().map(|b| b.id.as_str()).collect();
    for (i, a) in w.around.iter().enumerate() {
        let p = format!("around[{i}]");
        if let Some(o) = &a.operator { if !biz.contains(o.as_str()) { v.push(err(&p, format!("{} is run by {o}, which isn't one of the businesses", a.name))); } }
        if let Some(pk) = &a.perk { if pk.cost <= 0 || pk.supply <= 0 { v.push(err(&p, format!("{}'s perk needs a points cost and a supply", a.name))); } }
        if let Some(sv) = &a.service { if sv.kind == "parking" && (sv.price <= 0 || sv.supply <= 0) { v.push(err(&p, format!("{} needs a price and a number of spaces", a.name))); } }
    }
    for d in &w.drivers { if !biz.contains(d.provider.as_str()) { v.push(err("drivers", format!("{} drives for {}, which isn't one of the businesses", d.name, d.provider))); } }
    v
}

/// Check the template, then run a whole simulated night on it.
pub fn preview(w: WorldDef, tpl: Template, fans: usize, runners: usize) -> Value {
    let issues = check(&w, &tpl);
    if issues.iter().any(|i| i.blocking) { return json!({ "ok": false, "issues": issues }); }
    let mut n = Night::new(w, tpl, Config { fans, runners, ..Config::default() });
    while n.step() {}
    let report = n.finish();
    let pct = |k: &str| report.by_stage.get(k).copied().unwrap_or(0);
    let mut by_stand: Vec<Value> = n.world.def.stands.iter().map(|s| {
        let sold: i64 = n.ledger.txns.iter().filter(|t| t.memo.starts_with("settle")).flat_map(|t| t.postings.iter()).filter(|p| p.account.party == s.id && p.account.purpose == "revenue").map(|p| p.amount).sum();
        json!({ "id": s.id, "name": s.name, "revenue": sold })
    }).collect();
    by_stand.sort_by_key(|x| -x["revenue"].as_i64().unwrap_or(0));
    json!({ "ok": true, "issues": issues, "report": {
        "orders": report.orders, "delivered": pct("Settled"), "wait_p50_s": report.wait_p50_s, "wait_p90_s": report.wait_p90_s, "late": report.late,
        "cannot_fulfil": pct("StandCannotFulfil"), "balked": report.balked, "runner_hourly": report.runner_hourly, "batch_avg": report.batch_avg,
        "money": report.money, "conserved": report.conserved, "stands": by_stand, "fans": fans, "runners": runners,
    }})
}

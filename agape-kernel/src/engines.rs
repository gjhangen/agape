//! Layer 2: the engines. Offer presentation (eligibility, then the Agape check, per fan),
//! dispatch (batching ready orders into runner routes by walk cost), and attribution
//! (what each rule's offers were shown, accepted, settled and cost, reconciled to the ledger).

use crate::economy::{agape_check, Breakdown, Discount, Line, OfferError, Quote, ValueFn};
use crate::value::PartyId;
use crate::world::{Fan, Refusal, World};
use std::collections::{BTreeMap, HashMap};

// ---------- offers ----------

#[derive(Clone, Debug)]
pub struct Shown { pub offer_id: String, pub source: String, pub fan: PartyId, pub quote: Quote, pub breakdown: Breakdown, pub expires_at: Option<i64> }

/// What the world can still promise right now, read from the ledger and the stations.
pub struct Capacity<'a> {
    /// Units of a thing the stand still has unheld.
    pub available: &'a dyn Fn(&str) -> i64,
    /// Budget a payer still has unheld.
    pub budget: &'a dyn Fn(&str) -> i64,
    /// Seconds until a new order at this stand would be ready.
    pub stand_wait_s: &'a dyn Fn(&str) -> i64,
}

pub struct OfferSpec<'a> { pub source: &'a str, pub thing: &'a str, pub price: i64, pub discount: i64, pub payer: Option<&'a str>, pub expires_ms: Option<i64>, pub t_ms: i64, pub pushed: bool }

/// Build the quote each eligible fan would see and keep only those where nobody loses.
/// An offer is a promise, so it is shown only to as many fans as stock and budget can honour
/// if every one of them accepts, and only while the stand can still deliver inside the service promise.
pub fn present(world: &World, spec: &OfferSpec, fans: &[Fan], open_span: &dyn Fn(&str) -> bool, cap: &Capacity, attr: &mut Attribution) -> Vec<Shown> {
    let Some(thing) = world.thing(spec.thing) else { return vec![]; };
    let mut out = vec![];
    if (cap.stand_wait_s)(&thing.stand) > world.def.service.max_eta_s as i64 {
        attr.refuse(spec.source, Refusal::StandBusy);
        return out;
    }
    let mut units = (cap.available)(&thing.id);
    let mut budget = spec.payer.map_or(i64::MAX, |p| (cap.budget)(p));
    for fan in fans {
        if units < 1 { attr.refuse(spec.source, Refusal::SoldOut); continue; }
        if budget < spec.discount { attr.refuse(spec.source, Refusal::BudgetSpent); continue; }
        if let Err(r) = world.eligible(fan, spec.thing, open_span, spec.pushed) { attr.refuse(spec.source, r); continue; }
        let q = Quote {
            fan: fan.id.clone(), stand: thing.stand.clone(), runner: None,
            lines: vec![Line { thing: thing.id.clone(), qty: 1, unit_price: spec.price }],
            discounts: match (spec.discount, spec.payer) {
                (0, _) => vec![],
                (d, Some(p)) => vec![Discount { amount: d, payer: p.into(), reason: spec.source.into() }],
                (_, None) => { attr.refuse(spec.source, Refusal::Loses("no payer".into())); continue; }
            },
            points_redeemed: 0, tip: 0, delivery: true, fee_waived: false, runner_pay: None };
        let mut values = BTreeMap::new();
        values.insert(fan.id.clone(), ValueFn::Floor(-fan.ceiling));
        values.insert(thing.stand.clone(), ValueFn::Floor(0));
        values.insert("agape".to_string(), ValueFn::Floor(0));
        match agape_check(&q, &world.def.terms, &values) {
            Ok(b) => {
                attr.stat(spec.source).shown += 1;
                units -= 1;
                budget -= spec.discount;
                out.push(Shown { offer_id: format!("{}@{}#{}", spec.source, spec.t_ms, fan.id), source: spec.source.into(), fan: fan.id.clone(), quote: q, breakdown: b, expires_at: spec.expires_ms.map(|e| spec.t_ms + e) });
            }
            Err(OfferError::Loses { party, .. }) => attr.refuse(spec.source, if party == fan.id { Refusal::OverCeiling } else { Refusal::Loses(party) }),
            Err(_) => attr.refuse(spec.source, Refusal::Loses("invalid".into())),
        }
    }
    out
}

// ---------- dispatch ----------

#[derive(Clone, Debug)]
pub struct Ready { pub order: String, pub stand: String, pub seat: String, pub step_free: bool, pub ready_at: i64 }

#[derive(Clone, Debug)]
pub struct Runner { pub id: PartyId, pub at: String, pub free_at: i64 }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment { pub runner: PartyId, pub orders: Vec<String>, pub route: Vec<String>, pub pickup_at_s: u32,
    /// Each stand on the trip and when the runner reaches it (seconds from now).
    pub pickups: Vec<(String, u32)>, pub arrive_s: Vec<u32>, pub done_s: u32 }

/// Walk-cost cache over the place graph.
#[derive(Default)]
pub struct Costs { memo: HashMap<(String, String, bool), u32> }
impl Costs {
    pub fn new() -> Self { Costs::default() }
    pub fn get(&mut self, world: &World, a: &str, b: &str, step_free: bool) -> u32 {
        if a == b { return 0; }
        let k = (a.to_string(), b.to_string(), step_free);
        if let Some(c) = self.memo.get(&k) { return *c; }
        let c = world.graph.path(a, b, step_free).map_or(u32::MAX / 4, |p| p.0);
        self.memo.insert(k, c);
        c
    }
}

/// A trip's walk: from `start` to each pickup in turn, then the seats, nearest next. Returns the
/// arrival (seconds from now) at each pickup, the orders in the order they're handed off, the arrival
/// at each of those seats, and when the trip is done.
fn walk(world: &World, costs: &mut Costs, start: &str, stands: &[String], orders: &[&Ready]) -> (Vec<u32>, Vec<usize>, Vec<u32>, u32) {
    let h = world.def.service.handoff_s;
    let (mut t, mut at) = (0u32, start.to_string());
    let mut picks = vec![];
    for s in stands { t = t.saturating_add(costs.get(world, &at, s, false)); picks.push(t); at = s.clone(); }
    let mut left: Vec<usize> = (0..orders.len()).collect();
    let (mut seq, mut arrive) = (vec![], vec![]);
    while !left.is_empty() {
        let (k, c) = left.iter().enumerate().map(|(k, &i)| (k, costs.get(world, &at, &orders[i].seat, orders[i].step_free))).min_by_key(|(k, c)| (*c, orders[left[*k]].order.clone())).unwrap();
        let i = left.remove(k);
        t = t.saturating_add(c); arrive.push(t); t = t.saturating_add(h);
        at = orders[i].seat.clone(); seq.push(i);
    }
    (picks, seq, arrive, t)
}

/// Add orders from `from` to the bag, nearest seat first, while every order still arrives in time.
fn grow<'a>(world: &World, costs: &mut Costs, start: &str, stands: &[String], batch: &mut Vec<&'a Ready>, from: &str, pending: &[&'a Ready]) {
    let svc = &world.def.service;
    loop {
        if batch.len() >= svc.batch_max { break; }
        let last = batch.last().unwrap().seat.clone();
        let mut cands: Vec<(&'a Ready, u32)> = pending.iter().filter(|o| o.stand == from && !batch.iter().any(|b| b.order == o.order)).map(|o| (*o, costs.get(world, &last, &o.seat, o.step_free))).collect();
        cands.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.order.cmp(&b.0.order)));
        let mut added = false;
        for (o, _) in cands {
            let mut t = batch.clone(); t.push(o);
            let (_, _, arr, _) = walk(world, costs, start, stands, &t);
            if arr.iter().all(|&a| a <= svc.max_eta_s) { batch.push(o); added = true; break; }
        }
        if !added { break; }
    }
}

/// Batching and routing. The oldest ready order goes first; the nearest free runner goes to its
/// stand and takes more orders from it, then from one more stand close by, as long as every order
/// still reaches its seat within the promise (`max_eta_s` from now) and the bag holds no more than
/// `batch_max`. Seats are visited nearest next.
pub fn dispatch(world: &World, costs: &mut Costs, ready: &[Ready], runners: &[Runner], now: i64) -> Vec<Assignment> {
    let svc = &world.def.service;
    let mut pending: Vec<&Ready> = ready.iter().collect();
    pending.sort_by(|a, b| a.ready_at.cmp(&b.ready_at).then(a.order.cmp(&b.order)));
    let mut free: Vec<&Runner> = runners.iter().filter(|r| r.free_at <= now).collect();
    let mut out = vec![];
    while let Some(first) = pending.first().copied() {
        let Some((ri, _)) = free.iter().enumerate()
            .map(|(i, r)| (i, costs.get(world, &r.at, &first.stand, false)))
            .min_by_key(|(i, c)| (*c, free[*i].id.clone())) else { break };
        let runner = free.remove(ri);
        let mut stands = vec![first.stand.clone()];
        let mut batch: Vec<&Ready> = vec![first];
        grow(world, costs, &runner.at, &stands, &mut batch, &first.stand.clone(), &pending);
        // then one more stand close by, if the bag has room and the promise still holds
        if batch.len() < svc.batch_max {
            let mut near: Vec<(String, u32)> = pending.iter().filter(|o| o.stand != first.stand).map(|o| o.stand.clone()).collect::<std::collections::BTreeSet<_>>().into_iter()
                .map(|s| { let c = costs.get(world, &first.stand, &s, false); (s, c) }).filter(|(_, c)| *c <= svc.second_stand_s).collect();
            near.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
            for (s, _) in near {
                let two = vec![first.stand.clone(), s.clone()];
                let mut b2 = batch.clone();
                grow(world, costs, &runner.at, &two, &mut b2, &s, &pending);
                if b2.len() > batch.len() { batch = b2; stands = two; break; }
            }
        }
        pending.retain(|o| !batch.iter().any(|b| b.order == o.order));
        let (picks, seq, arrive, done) = walk(world, costs, &runner.at, &stands, &batch);
        let ordered: Vec<&Ready> = seq.iter().map(|&i| batch[i]).collect();
        let mut route = vec![runner.at.clone()];
        route.extend(stands.iter().cloned());
        route.extend(ordered.iter().map(|b| b.seat.clone()));
        out.push(Assignment { runner: runner.id.clone(), orders: ordered.iter().map(|b| b.order.clone()).collect(), route,
            pickup_at_s: *picks.last().unwrap(), pickups: stands.iter().cloned().zip(picks.iter().copied()).collect(), arrive_s: arrive, done_s: done });
    }
    out
}

/// Staging: send an idle runner to a stand ahead of its food, timed to arrive as it comes off the
/// line. `due` is each delivery order still being made: (stand, when it will be ready). A runner is
/// sent only if they'd arrive no later than a minute after it's ready and wait no longer than
/// `stage_wait_s` for it; otherwise they stay put and the order goes to whoever is nearest when it's
/// ready. One runner covers up to `batch_max` orders finishing close together at a stand.
pub fn stage(world: &World, costs: &mut Costs, idle: &[Runner], due: &[(String, i64)], now: i64) -> Vec<(PartyId, String, u32)> {
    let svc = &world.def.service;
    let per = svc.batch_max.max(1);
    let mut by: std::collections::BTreeMap<&str, Vec<i64>> = std::collections::BTreeMap::new();
    for (s, f) in due { by.entry(s.as_str()).or_default().push(*f); }
    let mut free: Vec<&Runner> = idle.iter().filter(|r| r.free_at <= now).collect();
    let mut out = vec![];
    // soonest food first
    let mut stands: Vec<(&str, Vec<i64>)> = by.into_iter().map(|(s, mut v)| { v.sort(); (s, v) }).collect();
    stands.sort_by_key(|(_, v)| v[0]);
    for (stand, times) in stands {
        let want = (times.len() + per - 1) / per;
        let have = free.iter().filter(|r| r.at == stand).count();
        // runners already standing there keep it covered
        free.retain(|r| r.at != stand || false);
        for k in have..want {
            let f = times[(k * per).min(times.len() - 1)];
            let best = free.iter().enumerate().map(|(i, r)| (i, costs.get(world, &r.at, stand, false)))
                .filter(|(_, c)| { let arrive = now + *c as i64 * 1000; arrive <= f + 60_000 && f - arrive <= svc.stage_wait_s as i64 * 1000 })
                .min_by_key(|(i, c)| (*c, free[*i].id.clone()));
            let Some((i, c)) = best else { continue };
            let r = free.remove(i);
            out.push((r.id.clone(), stand.to_string(), c));
        }
    }
    out
}

// ---------- attribution ----------

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stat { pub shown: u64, pub accepted: u64, pub settled: u64, pub gross: i64, pub spend: i64, pub refused: BTreeMap<String, u64> }

/// Invariant 8: every settled order traces back to what was shown, and the payer's spend here
/// equals what left its budget in the ledger.
#[derive(Default, Debug)]
pub struct Attribution { pub by_source: BTreeMap<String, Stat> }

impl Attribution {
    pub fn stat(&mut self, source: &str) -> &mut Stat { self.by_source.entry(source.into()).or_default() }
    pub fn refuse(&mut self, source: &str, r: Refusal) { *self.stat(source).refused.entry(format!("{r:?}")).or_insert(0) += 1; }
    pub fn accepted(&mut self, source: &str) { self.stat(source).accepted += 1; }
    /// Call on every capture (settled, or fan unreachable), since both spend the payer's discount.
    pub fn settled(&mut self, source: &str, q: &Quote, b: &Breakdown) {
        let s = self.stat(source);
        s.settled += 1;
        s.gross += b.subtotal;
        s.spend += q.discounts.iter().map(|d| d.amount).sum::<i64>();
    }
    pub fn spend_by_payer(&self, payer_of: &dyn Fn(&str) -> Option<String>) -> BTreeMap<String, i64> {
        let mut m = BTreeMap::new();
        for (src, s) in &self.by_source { if let Some(p) = payer_of(src) { *m.entry(p).or_insert(0) += s.spend; } }
        m
    }
}

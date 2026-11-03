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
            points_redeemed: 0, tip: 0, delivery: true,
        };
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
pub struct Assignment { pub runner: PartyId, pub orders: Vec<String>, pub route: Vec<String>, pub pickup_at_s: u32, pub arrive_s: Vec<u32>, pub done_s: u32 }

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

/// Greedy batching: oldest ready order first; the nearest free runner goes to its stand and
/// takes up to `batch_max` orders from that stand, adding the seat that lengthens the route
/// least, while every order in the batch still arrives within `max_eta_s` from now.
pub fn dispatch(world: &World, costs: &mut Costs, ready: &[Ready], runners: &[Runner], now: i64) -> Vec<Assignment> {
    let svc = &world.def.service;
    let mut pending: Vec<&Ready> = ready.iter().collect();
    pending.sort_by(|a, b| a.ready_at.cmp(&b.ready_at).then(a.order.cmp(&b.order)));
    let mut free: Vec<&Runner> = runners.iter().filter(|r| r.free_at <= now).collect();
    let mut out = vec![];
    while let Some(first) = pending.first().copied() {
        let Some((ri, to_stand)) = free.iter().enumerate()
            .map(|(i, r)| (i, costs.get(world, &r.at, &first.stand, false)))
            .min_by_key(|(i, c)| (*c, free[*i].id.clone())) else { break };
        let runner = free.remove(ri);
        let mut batch = vec![first];
        let mut at = first.seat.clone();
        let mut t = to_stand + costs.get(world, &first.stand, &first.seat, first.step_free) + svc.handoff_s;
        let mut arrive = vec![t - svc.handoff_s];
        // Each added order must still arrive within the service promise counted from now.
        // (Measuring from the oldest order's ready time would forbid batching exactly when a
        // backlog makes it most valuable: the alternative for the others is waiting for another runner.)
        let budget = svc.max_eta_s as i64;
        loop {
            if batch.len() >= svc.batch_max { break; }
            let cand = pending.iter().filter(|o| o.stand == first.stand && !batch.iter().any(|b| b.order == o.order))
                .map(|o| (*o, costs.get(world, &at, &o.seat, o.step_free)))
                .min_by_key(|(o, c)| (*c, o.order.clone()));
            let Some((o, c)) = cand else { break };
            if (t + c) as i64 > budget.max(0) { break; }
            t += c + svc.handoff_s;
            arrive.push(t - svc.handoff_s);
            at = o.seat.clone();
            batch.push(o);
        }
        pending.retain(|o| !batch.iter().any(|b| b.order == o.order));
        let mut route = vec![runner.at.clone(), first.stand.clone()];
        route.extend(batch.iter().map(|b| b.seat.clone()));
        out.push(Assignment { runner: runner.id.clone(), orders: batch.iter().map(|b| b.order.clone()).collect(), route, pickup_at_s: to_stand, arrive_s: arrive, done_s: t });
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

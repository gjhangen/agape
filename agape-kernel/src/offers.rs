//! Layer 2: the offer book. Every live offer, every claim, and what each fan is shown about
//! how the offer can end. The no-surprise invariant lives here:
//!
//!   A fan may watch an offer end. A fan never loses an offer they have started to claim.
//!
//! Guaranteed offers move their stock and budget into a promise when shown, so each fan
//! they were shown to can claim until the disclosed end. While-it-lasts offers are shown to
//! every eligible fan with their live limits (time, what is left, the condition); the tap
//! itself is the claim, a short hold, so checkout never fails.

use crate::commitment::Commitment;
use crate::economy::{hold, hold_from, release, release_to, Quote};
use crate::engines::{present, Attribution, Capacity, OfferSpec};
use crate::rules::{Engine, Limits, Mode};
use crate::value::{acct, posting, Ledger, PartyId, Txn, Unit};
use crate::world::{Fan, World};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum End { Time, Condition, SoldOut, BudgetSpent, CapReached, /** The stand cannot deliver inside the service promise right now; resumes when it can. */ Paused }

#[derive(Clone, Debug)]
pub struct Live {
    pub id: String,
    pub source: String,
    pub thing: String,
    pub stand: String,
    pub price: i64,
    pub discount: i64,
    pub payer: Option<String>,
    pub limits: Limits,
    pub expires_at: Option<i64>,
    /// Fans this offer is visible to.
    pub audience: BTreeSet<PartyId>,
    /// Claims currently held or converted to orders.
    pub claimed: i64,
    pub per_fan: BTreeMap<PartyId, i64>,
    /// Guaranteed only: promises still set aside for fans who have not claimed.
    pub promised_left: i64,
    /// Final ends (time, condition). Resource ends are recomputed live.
    pub closed: Option<End>,
    /// When it opened and, for while-it-lasts, how many were left then: so a card can show it running out.
    pub opened_at: i64,
    pub left_at_open: Option<i64>,
    pub closed_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim { pub id: String, pub offer: String, pub fan: PartyId, pub quote: Quote, pub until: i64 }

/// What a fan sees on the offer card. Everything that can end the offer is here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disclosure {
    pub guaranteed: bool,
    /// Guaranteed and still held for this fan.
    pub held_for_you: bool,
    pub ends_at: Option<i64>,
    /// While it lasts: how many claims are left at the terms shown now (the smallest of cap,
    /// stock, budget and the current tier).
    pub left: Option<i64>,
    /// The discount the next claim gets (tiers move it).
    pub discount_now: i64,
    pub condition: Option<String>,
    pub ended: Option<End>,
    pub opened_at: i64,
    pub left_at_open: Option<i64>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TapError { Ended(End), NotShown, PerFanLimit, NotEligible, Unknown }

#[derive(Debug, PartialEq, Eq)]
pub enum CheckoutError { Expired, Unknown }

/// An offer that fired while its stand could not deliver in time: it waits, inside its own
/// window, for the stand to recover, instead of being lost.
#[derive(Clone, Debug)]
pub struct Pending { pub source: String, pub thing: String, pub price: i64, pub discount: i64, pub payer: Option<String>, pub until: Option<i64>, pub limits: Limits }

pub struct OfferBook {
    pub offers: BTreeMap<String, Live>,
    pub pending: Vec<Pending>,
    /// Stands that cannot currently deliver inside the service promise. While-it-lasts offers
    /// at these stands show as paused and cannot be tapped until the stand recovers.
    pub busy: BTreeSet<String>,
    pub claims: BTreeMap<String, Claim>,
    /// How long a tap holds the offer while the fan checks out.
    pub claim_ttl_ms: i64,
    seq: u64,
}

const PROMISED: &str = "promised";
const PROMISED_BUDGET: &str = "promised_budget";

impl OfferBook {
    pub fn new(claim_ttl_ms: i64) -> Self { OfferBook { offers: BTreeMap::new(), pending: vec![], busy: BTreeSet::new(), claims: BTreeMap::new(), claim_ttl_ms, seq: 0 } }

    fn tier_discount(l: &Live, claimed: i64) -> Option<i64> {
        if l.limits.tiers.is_empty() { return Some(l.discount); }
        let mut n = claimed;
        for t in &l.limits.tiers { if n < t.units { return Some(t.discount); } n -= t.units; }
        None
    }

    /// Open an offer from a rule's output. Returns its id and the fans it is visible to.
    #[allow(clippy::too_many_arguments)]
    pub fn open(&mut self, world: &World, ledger: &mut Ledger, engine: &Engine, spec: &OfferSpec, limits: &Limits, fans: &[Fan],
                stand_wait_s: &dyn Fn(&str) -> i64, attr: &mut Attribution) -> Option<String> {
        let thing = world.thing(spec.thing)?;
        if let Some(c) = &limits.while_ { if !engine.cond(c, spec.t_ms) { return None; } }
        if stand_wait_s(&thing.stand) > world.def.service.max_eta_s as i64 {
            attr.refuse(spec.source, crate::world::Refusal::StandBusy);
            self.pending.push(Pending { source: spec.source.into(), thing: spec.thing.into(), price: spec.price, discount: spec.discount,
                payer: spec.payer.map(String::from), until: spec.expires_ms.map(|e| spec.t_ms + e), limits: limits.clone() });
            return None;
        }
        self.seq += 1;
        let id = format!("{}#{}", spec.source, self.seq);
        let stand = thing.stand.clone();
        let stock = ledger.balance(&acct(&stand, "inventory"), &Unit::Thing(thing.id.clone()));
        let budget = spec.payer.map_or(i64::MAX, |p| ledger.balance(&acct(p, "promo_budget"), &Unit::usd()));
        let guaranteed = limits.mode == Mode::Guaranteed;
        let cap = limits.cap.unwrap_or(i64::MAX);
        let unbounded = |_: &str| i64::MAX;
        let shown = if guaranteed {
            let avail = |_: &str| stock.min(cap);
            let bud = |_: &str| budget;
            present(world, spec, fans, &|s| engine.span_open(s), &Capacity { available: &avail, budget: &bud, stand_wait_s }, attr)
        } else {
            present(world, spec, fans, &|s| engine.span_open(s), &Capacity { available: &unbounded, budget: &unbounded, stand_wait_s }, attr)
        };
        if shown.is_empty() { return None; }
        let n = shown.len() as i64;
        if guaranteed {
            // set aside one unit and one discount for every fan it was shown to
            let mut p = vec![
                posting(acct(&stand, "inventory"), Unit::Thing(thing.id.clone()), -n),
                posting(acct(&stand, PROMISED), Unit::Thing(thing.id.clone()), n),
            ];
            if let (Some(payer), d) = (spec.payer, spec.discount) {
                if d > 0 {
                    p.push(posting(acct(payer, "promo_budget"), Unit::usd(), -d * n));
                    p.push(posting(acct(payer, PROMISED_BUDGET), Unit::usd(), d * n));
                }
            }
            ledger.post(Txn { memo: format!("promise {id}"), postings: p }).expect("present() capped the audience to what exists");
        }
        self.offers.insert(id.clone(), Live {
            id: id.clone(), source: spec.source.into(), thing: thing.id.clone(), stand, price: spec.price, discount: spec.discount,
            payer: spec.payer.map(String::from), limits: limits.clone(), expires_at: spec.expires_ms.map(|e| spec.t_ms + e),
            audience: shown.into_iter().map(|s| s.fan).collect(), claimed: 0, per_fan: BTreeMap::new(),
            promised_left: if guaranteed { n } else { 0 }, closed: None, opened_at: spec.t_ms, left_at_open: None, closed_at: None,
        });
        if !guaranteed {
            let l = &self.offers[&id];
            let left = self.remaining(l, ledger).0;
            self.offers.get_mut(&id).unwrap().left_at_open = Some(left);
        }
        Some(id)
    }

    /// Try pending offers again: open those whose stand has recovered, drop those whose window passed.
    #[allow(clippy::too_many_arguments)]
    pub fn retry_pending(&mut self, world: &World, ledger: &mut Ledger, engine: &Engine, fans: &[Fan], stand_wait_s: &dyn Fn(&str) -> i64, attr: &mut Attribution, t: i64) -> u64 {
        let pending = std::mem::take(&mut self.pending);
        let mut opened = 0;
        for p in pending {
            if p.until.map_or(false, |u| t >= u) { continue; }
            if p.limits.while_.as_ref().map_or(false, |c| !engine.cond(c, t)) { continue; }
            let stand = world.thing(&p.thing).map(|x| x.stand.clone()).unwrap_or_default();
            if stand_wait_s(&stand) > world.def.service.max_eta_s as i64 { self.pending.push(p); continue; }
            let spec = OfferSpec { source: &p.source, thing: &p.thing, price: p.price, discount: p.discount, payer: p.payer.as_deref(),
                expires_ms: p.until.map(|u| u - t), t_ms: t, pushed: true };
            if self.open(world, ledger, engine, &spec, &p.limits, fans, stand_wait_s, attr).is_some() { opened += 1; }
        }
        opened
    }

    /// What is left of a while-it-lasts offer right now, and which resource binds.
    fn remaining(&self, l: &Live, ledger: &Ledger) -> (i64, End) {
        let mut best = (i64::MAX, End::CapReached);
        let mut take = |v: i64, e: End| if v < best.0 { best = (v, e) };
        if let Some(c) = l.limits.cap { take(c - l.claimed, End::CapReached); }
        // "left" means left at the terms shown now: with tiers, what remains of the current tier
        if !l.limits.tiers.is_empty() {
            let mut n = l.claimed;
            let mut in_tier = 0;
            for t in &l.limits.tiers { if n < t.units { in_tier = t.units - n; break; } n -= t.units; }
            take(in_tier, End::CapReached);
        }
        take(ledger.balance(&acct(&l.stand, "inventory"), &Unit::Thing(l.thing.clone())), End::SoldOut);
        if let (Some(p), Some(d)) = (&l.payer, Self::tier_discount(l, l.claimed)) {
            if d > 0 { take(ledger.balance(&acct(p, "promo_budget"), &Unit::usd()) / d, End::BudgetSpent); }
        }
        (best.0.max(0), best.1)
    }

    /// Close offers whose time or condition has ended; release what guaranteed offers still promised.
    pub fn refresh(&mut self, ledger: &mut Ledger, engine: &Engine, t: i64) {
        for l in self.offers.values_mut().filter(|l| l.closed.is_none()) {
            let end = if l.expires_at.map_or(false, |e| t >= e) { Some(End::Time) }
                else if l.limits.while_.as_ref().map_or(false, |c| !engine.cond(c, t)) { Some(End::Condition) }
                else { None };
            if let Some(e) = end {
                l.closed = Some(e);
                l.closed_at = Some(t);
                if l.promised_left > 0 {
                    let mut p = vec![
                        posting(acct(&l.stand, PROMISED), Unit::Thing(l.thing.clone()), -l.promised_left),
                        posting(acct(&l.stand, "inventory"), Unit::Thing(l.thing.clone()), l.promised_left),
                    ];
                    if let Some(payer) = &l.payer { if l.discount > 0 {
                        p.push(posting(acct(payer, PROMISED_BUDGET), Unit::usd(), -l.discount * l.promised_left));
                        p.push(posting(acct(payer, "promo_budget"), Unit::usd(), l.discount * l.promised_left));
                    }}
                    ledger.post(Txn { memo: format!("unpromise {}", l.id), postings: p }).expect("promised units are there");
                    l.promised_left = 0;
                }
            }
        }
    }

    pub fn disclose(&self, offer: &str, fan: &str, ledger: &Ledger) -> Option<Disclosure> {
        let l = self.offers.get(offer)?;
        let guaranteed = l.limits.mode == Mode::Guaranteed;
        let used = *l.per_fan.get(fan).unwrap_or(&0);
        let (left, why) = if guaranteed { (None, End::CapReached) } else { let (n, e) = self.remaining(l, ledger); (Some(n), e) };
        let paused = !guaranteed && self.busy.contains(&l.stand);
        let ended = l.closed.or(if left == Some(0) { Some(why) } else { None })
            .or(if paused { Some(End::Paused) } else { None })
            .or(if guaranteed && used >= 1 { Some(End::CapReached) } else { None });
        Some(Disclosure {
            guaranteed, held_for_you: guaranteed && ended.is_none() && l.audience.contains(fan),
            ends_at: l.expires_at, left, discount_now: Self::tier_discount(l, l.claimed).unwrap_or(0),
            condition: l.limits.label.clone(), ended, opened_at: l.opened_at, left_at_open: l.left_at_open,
        })
    }

    /// The tap. Either the fan now holds a claim that checkout will honour, or the card already
    /// showed why it could not (`Ended`): there is no third outcome.
    pub fn tap(&mut self, offer: &str, fan: &Fan, world: &World, ledger: &mut Ledger, engine: &Engine, t: i64) -> Result<Claim, TapError> {
        self.refresh(ledger, engine, t);
        let d = self.disclose(offer, &fan.id, ledger).ok_or(TapError::Unknown)?;
        let l = self.offers.get_mut(offer).ok_or(TapError::Unknown)?;
        if !l.audience.contains(&fan.id) { return Err(TapError::NotShown); }
        if let Some(e) = d.ended { return Err(TapError::Ended(e)); }
        let used = *l.per_fan.get(&fan.id).unwrap_or(&0);
        if used >= l.limits.per_fan.unwrap_or(if d.guaranteed { 1 } else { i64::MAX }) { return Err(TapError::PerFanLimit); }
        if world.eligible(fan, &l.thing, &|s| engine.span_open(s), true).is_err() { return Err(TapError::NotEligible); }
        let quote = Quote {
            fan: fan.id.clone(), stand: l.stand.clone(), runner: None,
            lines: vec![crate::economy::Line { thing: l.thing.clone(), qty: 1, unit_price: l.price }],
            discounts: match (&l.payer, d.discount_now) { (Some(p), x) if x > 0 => vec![crate::economy::Discount { amount: x, payer: p.clone(), reason: l.source.clone() }], _ => vec![] },
            points_redeemed: 0, tip: 0, delivery: true,
        };
        let txn = if d.guaranteed { hold_from(&quote, PROMISED, PROMISED_BUDGET) } else { hold(&quote) };
        ledger.post(txn).expect("the disclosure said it was there; the tap is atomic with it");
        l.claimed += 1;
        if d.guaranteed { l.promised_left -= 1; }
        *l.per_fan.entry(fan.id.clone()).or_insert(0) += 1;
        self.seq += 1;
        let c = Claim { id: format!("claim{}", self.seq), offer: offer.into(), fan: fan.id.clone(), quote, until: t + self.claim_ttl_ms };
        self.claims.insert(c.id.clone(), c.clone());
        Ok(c)
    }

    /// Checkout turns a claim into a commitment. It cannot fail for lack of stock or budget:
    /// the claim already holds them. It fails only if the fan let the claim lapse.
    pub fn checkout(&mut self, claim: &str, order_id: &str, t: i64, tip: i64) -> Result<(Commitment, String), CheckoutError> {
        let c = self.claims.get(claim).ok_or(CheckoutError::Unknown)?;
        if t > c.until { return Err(CheckoutError::Expired); }
        let c = self.claims.remove(claim).unwrap();
        let mut q = c.quote;
        q.tip = tip;
        let source = self.offers.get(&c.offer).map(|l| l.source.clone()).unwrap_or_default();
        Ok((Commitment::accept_held(order_id, q), source))
    }

    /// Claims the fan walked away from go back to where they came from.
    pub fn expire_claims(&mut self, ledger: &mut Ledger, t: i64) -> u64 {
        let lapsed: Vec<String> = self.claims.values().filter(|c| t > c.until).map(|c| c.id.clone()).collect();
        for id in &lapsed {
            let c = self.claims.remove(id).unwrap();
            let l = self.offers.get_mut(&c.offer).unwrap();
            l.claimed -= 1;
            *l.per_fan.get_mut(&c.fan).unwrap() -= 1;
            if l.limits.mode == Mode::Guaranteed && l.closed.is_none() {
                // still promised to this fan until the offer's disclosed end
                ledger.post(release_to(&c.quote, PROMISED, PROMISED_BUDGET)).unwrap();
                l.promised_left += 1;
            } else if l.limits.mode == Mode::Guaranteed {
                ledger.post(release(&c.quote)).unwrap();
            } else {
                ledger.post(release(&c.quote)).unwrap();
            }
        }
        lapsed.len() as u64
    }

    /// Offers visible to a fan, newest first.
    pub fn visible(&self, fan: &str) -> Vec<&Live> {
        let mut v: Vec<&Live> = self.offers.values().filter(|l| l.audience.contains(fan)).collect();
        v.reverse();
        v
    }
}

//! Looking after people when things go wrong: an order running late is flagged (and its credit
//! promised) the moment it misses its promise, not when it finally arrives; a fan can report a
//! problem with an order and get their money back from whoever was at fault; and the venue can
//! stop everything at once.

use crate::commitment::{Action, Stage, DISPATCHER};
use crate::economy::breakdown;
use crate::sim::Night;
use crate::value::{acct, posting, Txn, Unit};
use serde::Serialize;

/// How long after delivery a fan can still report a problem.
pub const REPORT_WINDOW_MS: i64 = 30 * 60_000;
/// Refunds approved without a person looking, per fan per night. Beyond this, operations decides
/// (a velocity rule against abuse).
pub const AUTO_REFUNDS_PER_FAN: usize = 2;
/// The most a stand refunds in a night before operations looks at every further claim on it
/// (a venue term: a bad night is investigated, not silently absorbed).
pub const AUTO_REFUNDS_PER_STAND: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason { NeverArrived, WrongItem, Quality }

impl Reason {
    pub fn parse(s: &str) -> Option<Reason> { match s { "never_arrived" => Some(Reason::NeverArrived), "wrong_item" => Some(Reason::WrongItem), "quality" => Some(Reason::Quality), _ => None } }
    pub fn words(&self) -> &'static str { match self { Reason::NeverArrived => "It never arrived", Reason::WrongItem => "Wrong or missing item", Reason::Quality => "Cold, spilled or not right" } }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum ProblemState { Open, Refunded { amount: i64, by: String }, Declined { why: String } }

#[derive(Clone, Debug, Serialize)]
pub struct Problem { pub id: String, pub order: usize, pub fan: String, pub stand: String, pub reason: Reason, pub note: String, pub t: i64,
    /// Whose doing it was: the stand for what it made, Agape for getting it there.
    pub fault: String, pub state: ProblemState }

#[derive(Clone, Debug, Serialize)]
pub struct Halt { pub reason: String, pub since: i64 }

impl Night {
    /// Every order past its promise is flagged now, so the fan hears it's late while still
    /// waiting, with the credit already promised. A kitchen that hasn't finished owns it; once
    /// the food is made, getting it there is Agape's.
    pub(crate) fn flag_late(&mut self) {
        let t = self.t;
        for o in self.orders.iter_mut() {
            if o.c.late_by.is_some() || t <= o.promise || o.delivered_at.is_some() { continue; }
            let fault = match o.c.stage {
                Stage::Accepted | Stage::Preparing => o.c.quote.stand.clone(),
                Stage::Ready | Stage::Assigned | Stage::PickedUp => DISPATCHER.to_string(),
                _ => continue,
            };
            o.c.late_by = Some((fault, (t - o.promise) / 1000));
        }
    }

    // ---------------- problems and refunds ----------------

    pub fn report_problem(&mut self, fan: &str, order: usize, reason: &str, note: &str) -> Result<String, String> {
        let reason = Reason::parse(reason).ok_or("Pick what went wrong")?;
        let o = self.orders.get(order).ok_or("Unknown order")?;
        if o.c.quote.fan != fan { return Err("That isn't your order".into()); }
        if o.c.stage != Stage::Settled { return Err("You can report a problem once it's been delivered".into()); }
        let delivered = o.delivered_at.unwrap_or(0);
        if self.t - delivered > REPORT_WINDOW_MS { return Err("Problems can be reported up to 30 minutes after delivery".into()); }
        if self.problems.iter().any(|p| p.order == order) { return Err("You've already reported this one".into()); }
        let stand = o.c.quote.stand.clone();
        let fault = if reason == Reason::NeverArrived { DISPATCHER.to_string() } else { stand.clone() };
        let id = format!("p{}", self.problems.len() + 1);
        self.problems.push(Problem { id: id.clone(), order, fan: fan.into(), stand: stand.clone(), reason, note: note.trim().chars().take(280).collect(), t: self.t, fault, state: ProblemState::Open });
        // operations decides when a person runs it, or past the velocity and venue limits
        let fan_refunds = self.problems.iter().filter(|p| p.fan == fan && matches!(p.state, ProblemState::Refunded { .. })).count();
        let stand_refunds = self.problems.iter().filter(|p| p.stand == stand && matches!(p.state, ProblemState::Refunded { .. })).count();
        if !self.human("ops") && fan_refunds < AUTO_REFUNDS_PER_FAN && stand_refunds < AUTO_REFUNDS_PER_STAND {
            self.resolve_problem("agape", &id, true, "")?;
        }
        Ok(id)
    }

    /// Refund what the fan paid for the order, delivery fee included; the tip stays with the runner. The sale is
    /// unwound in the shares it was made in; whoever was at fault also bears the card processing
    /// already paid. Tips stay with whoever earned them unless operations decides otherwise.
    pub fn resolve_problem(&mut self, by: &str, id: &str, refund: bool, why: &str) -> Result<(), String> {
        let i = self.problems.iter().position(|p| p.id == id).ok_or("Unknown problem")?;
        if self.problems[i].state != ProblemState::Open { return Err("Already decided".into()); }
        if !refund {
            self.problems[i].state = ProblemState::Declined { why: if why.trim().is_empty() { "Operations looked into it".into() } else { why.trim().into() } };
            return Ok(());
        }
        let p = self.problems[i].clone();
        let q = self.orders[p.order].c.quote.clone();
        let b = breakdown(&q, &self.terms).map_err(|e| format!("{e:?}"))?;
        let usd = Unit::usd();
        // the fan gets everything back but the tip: the delivery fee too, since a ruined order wasn't delivered
        let fee = b.fee;
        let back = b.subtotal - b.discount + b.tax + fee;
        let fault_acct = if p.fault == DISPATCHER { acct("agape", "revenue") } else { acct(&p.fault, "revenue") };
        let mut post = vec![
            posting(acct(&p.fan, "card"), usd.clone(), back),
            posting(acct("tax_authority", "payable"), usd.clone(), -b.tax),
            posting(acct(&q.stand, "revenue"), usd.clone(), -b.stand_net),
            posting(acct("agape", "revenue"), usd.clone(), -b.agape_share),
            // the stand paid processing out of the sale and the fan gets it all back, with the delivery fee:
            // whoever was at fault covers both
            posting(fault_acct, usd.clone(), -(b.processing + fee)),
        ];
        for d in &q.discounts { post.push(posting(acct(&d.payer, "promo_budget"), usd.clone(), d.amount)); }
        if q.points_redeemed > 0 {
            post.push(posting(acct("agape", "revenue"), usd.clone(), q.points_redeemed * crate::economy::POINT_VALUE_CENTS));
            let pts = Unit::Points("agape".into());
            post.push(posting(acct("agape", "issued"), pts.clone(), -q.points_redeemed));
            post.push(posting(acct(&p.fan, "points"), pts, q.points_redeemed));
        }
        self.ledger.post(Txn { memo: format!("refund {} {id}", p.order), postings: post }).map_err(|e| format!("{e:?}"))?;
        self.problems[i].state = ProblemState::Refunded { amount: back, by: by.into() };
        Ok(())
    }

    // ---------------- the venue stops ----------------

    /// Stop service everywhere: nothing new can be ordered or bought, and every order
    /// not yet being made is closed with nobody charged. What's already cooking or on its way
    /// finishes, so nobody is left holding food. Every fan sees why at once.
    pub fn halt(&mut self, reason: &str) -> Result<(), String> {
        let reason = reason.trim();
        if reason.is_empty() { return Err("Say why, so fans know".into()); }
        if self.halt.is_some() { return Err("Service is already stopped".into()); }
        self.halt = Some(Halt { reason: reason.into(), since: self.t });
        for i in 0..self.orders.len() {
            if self.orders[i].c.stage == Stage::Accepted {
                let _ = self.orders[i].c.act(Action::VenueHalt, "venue", &mut self.ledger, &self.terms);
            }
        }
        for q in self.queues.values_mut() { q.clear(); }
        self.scheduled.clear();
        Ok(())
    }
    pub fn resume(&mut self) -> Result<(), String> { if self.halt.take().is_none() { return Err("Service isn't stopped".into()); } Ok(()) }
    pub fn halted(&self) -> Result<(), String> { match &self.halt { Some(h) => Err(format!("Paused: {}", h.reason)), None => Ok(()) } }
}

// ---------------- after: rating and ordering again ----------------

impl Night {
    /// Rate the runner who brought it (or the stand, for pickup and for the food itself). Once each.
    pub fn rate(&mut self, fan: &str, order: usize, who: &str, stars: i64) -> Result<(), String> {
        let o = self.orders.get(order).ok_or("Unknown order")?;
        if o.c.quote.fan != fan { return Err("That isn't your order".into()); }
        if !matches!(o.c.stage, Stage::Settled) { return Err("Rate it once it's arrived".into()); }
        let ok = who == o.c.quote.stand || o.c.quote.runner.as_deref() == Some(who);
        if !ok { return Err("Rate the stand or runner on this order".into()); }
        if !(1..=5).contains(&stars) { return Err("One to five stars".into()); }
        if self.ratings.iter().any(|r| r.0 == order && r.1 == who) { return Err("Already rated".into()); }
        self.social_act(fan, "rate", who, &format!("{order}:{stars}"));
        Ok(())
    }

    /// (average stars, how many ratings) for a stand or runner tonight.
    pub fn rating_of(&self, who: &str) -> (Option<f64>, usize) {
        let v: Vec<i64> = self.ratings.iter().filter(|r| r.1 == who).map(|r| r.2).collect();
        if v.is_empty() { (None, 0) } else { (Some(v.iter().sum::<i64>() as f64 / v.len() as f64), v.len()) }
    }

    /// The same order again, from the same stand, the same way.
    pub fn reorder(&mut self, fan: &str, order: usize) -> Result<String, String> {
        let o = self.orders.get(order).ok_or("Unknown order")?;
        if o.c.quote.fan != fan { return Err("That isn't your order".into()); }
        let q = o.c.quote.clone();
        let l = q.lines.first().ok_or("Nothing to reorder")?;
        self.order_as(fan, &l.thing, l.qty, q.delivery, q.tip, None)
    }
}

// ---------------- gifts and going first ----------------

impl Night {
    /// Send something to another seat: a friend on Agape (it goes to their section and they're told
    /// who it's from) or any section. Nothing 21+, because nobody can check the recipient's age.
    pub fn gift(&mut self, from: &str, thing: &str, qty: i64, to: &str, note: &str, tip: i64) -> Result<String, String> {
        let th = self.world.thing(thing).ok_or("unknown item")?;
        if th.constraints.iter().any(|c| matches!(c, crate::world::Constraint::MinAge(_))) { return Err("Drinks for 21 and over can't be sent as gifts".into()); }
        let (seat, friend) = match self.fans.iter().find(|f| f.id == to && f.id != from) {
            Some(f) => (f.seat.clone(), Some(f.id.clone())),
            None if self.world.graph.places.get(to).map_or(false, |p| p.kind == "section") => (to.to_string(), None),
            None => return Err("Send it to a friend or a section".into()),
        };
        let id: usize = self.order_as(from, thing, qty, true, tip, None)?.parse().unwrap();
        let note: String = note.trim().chars().take(140).collect();
        let label = friend.as_ref().and_then(|f| self.party_name(f)).unwrap_or_else(|| format!("Section {}", &seat[1..]));
        let o = &mut self.orders[id];
        o.seat = seat;
        o.gift = Some((label, note.clone()));
        if let Some(f) = friend {
            let what = self.world.thing(thing).map(|t| t.name.clone()).unwrap_or_default();
            let msg = if note.is_empty() { format!("Sent you {what}. It's on its way to your seat.") } else { format!("Sent you {what}: {note}") };
            let _ = self.message(from, &f, &msg);
        }
        Ok(id.to_string())
    }

    /// Pay points to go first. The points are held now and taken only if it's delivered.
    pub fn prioritize(&mut self, who: &str, order: usize) -> Result<(), String> {
        let o = self.orders.get(order).ok_or("Unknown order")?;
        if o.c.quote.fan != who { return Err("That isn't your order".into()); }
        if o.priority { return Err("It's already first in line".into()); }
        if !matches!(o.c.stage, Stage::Accepted | Stage::Preparing | Stage::Ready) { return Err("It's already on its way".into()); }
        let pts = crate::social::points();
        self.ledger.post(Txn { memo: format!("priority {order}"), postings: vec![posting(acct(who, "points"), pts.clone(), -crate::sim::PRIORITY_POINTS), posting(acct(who, "priority_held"), pts, crate::sim::PRIORITY_POINTS)] })
            .map_err(|_| "CantAfford".to_string())?;
        self.orders[order].priority = true;
        self.orders[order].priority_held = true;
        // to the front of the kitchen's queue (behind others who paid), and of the shelf if it's ready
        let stand = self.orders[order].c.quote.stand.clone();
        if let Some(q) = self.queues.get_mut(&stand) {
            if let Some(pos) = q.iter().position(|&i| i == order) {
                q.remove(pos);
                let at = q.iter().position(|&i| !self.orders[i].priority).unwrap_or(q.len());
                q.insert(at, order);
            }
        }
        for r in self.ready.iter_mut() { if r.order == order.to_string() { r.ready_at -= crate::sim::PRIORITY_AHEAD_MS; } }
        Ok(())
    }

    /// Held priority points settle with their order: taken when delivered, returned on any exit.
    pub(crate) fn settle_priority(&mut self) {
        let pts = crate::social::points();
        for i in 0..self.orders.len() {
            if !self.orders[i].priority_held { continue; }
            let fan = self.orders[i].c.quote.fan.clone();
            let done = match &self.orders[i].c.stage { Stage::Settled => Some(true), Stage::Closed(_) => Some(false), _ => None };
            let Some(taken) = done else { continue };
            let to = if taken { acct(crate::social::ISSUER, "issued") } else { acct(&fan, "points") };
            let _ = self.ledger.post(Txn { memo: format!("priority {} {}", if taken { "taken" } else { "returned" }, i), postings: vec![
                posting(acct(&fan, "priority_held"), pts.clone(), -crate::sim::PRIORITY_POINTS), posting(to, pts.clone(), crate::sim::PRIORITY_POINTS)] });
            self.orders[i].priority_held = false;
        }
    }
}

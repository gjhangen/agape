//! Looking after people when things go wrong: an order running late is flagged (and its credit
//! promised) the moment it misses its promise, not when it finally arrives; a fan can report a
//! problem with an order and have it made right by Agape's policy (remedy.rs), at the cost of
//! whoever was at fault; and the venue can stop everything at once.

use crate::commitment::{Action, Stage, DISPATCHER};
use crate::economy::breakdown;
use crate::sim::Night;
use crate::remedy::{Form, Part, Proof, Source};
use crate::value::{acct, bps, posting, Txn, Unit};
use serde::Serialize;

/// How long after delivery a fan can still report a problem.
pub const REPORT_WINDOW_MS: i64 = 30 * 60_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum ProblemState { Open, Remedied { amount: i64, form: Form, by: String }, Declined { why: String } }

#[derive(Clone, Debug, Serialize)]
pub struct Problem { pub id: String, pub order: usize, pub fan: String, pub stand: String, pub reason: String, pub words: String, pub note: String, pub t: i64,
    /// The fan's photo of it, when they added one.
    pub photo: bool,
    /// Whose doing it was: a stand, a runner, or Agape.
    pub fault: String, pub state: ProblemState,
    /// Who paid what toward the remedy: (party, cents).
    pub paid_by: Vec<(String, i64)>,
    /// Why it waits for a person, when it does.
    pub held: Option<String> }

/// What Agape knows about a fan, for Agape's team only.
#[derive(Clone, Debug, Serialize)]
pub struct Record { pub orders: usize, pub spent: i64, pub claims: usize, pub claim_bps: i64, pub remedied: i64,
    /// What this fan has earned Agape (its share, fees and processing margin, less runner pay) and what their claims cost it.
    pub earned: i64, pub cost: i64, pub net: i64, pub ratings_given: usize, pub avg_given: Option<f64>, pub proof: Proof, pub proof_set: bool }

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

    // ---------------- problems and remedies ----------------

    pub fn report_problem(&mut self, fan: &str, order: usize, reason: &str, note: &str) -> Result<String, String> { self.report_with(fan, order, reason, note, false) }

    /// A fan reports what went wrong. Settled at once by the policy when it allows, or held for a person.
    pub fn report_with(&mut self, fan: &str, order: usize, reason: &str, note: &str, photo: bool) -> Result<String, String> {
        let rule = self.policy.rules.get(reason).cloned().ok_or("Pick what went wrong")?;
        let o = self.orders.get(order).ok_or("Unknown order")?;
        if o.c.quote.fan != fan { return Err("That isn't your order".into()); }
        if o.c.stage != Stage::Settled { return Err("You can report a problem once it's been delivered".into()); }
        let delivered = o.delivered_at.unwrap_or(0);
        if self.t - delivered > REPORT_WINDOW_MS { return Err("Problems can be reported up to 30 minutes after delivery".into()); }
        if self.problems.iter().any(|p| p.order == order) { return Err("You've already reported this one".into()); }
        if rule.photo && !photo { return Err("Add a photo of it so we can make it right".into()); }
        let stand = o.c.quote.stand.clone();
        let fault = self.party_for(order, &rule.fault);
        let id = format!("p{}", self.problems.len() + 1);
        // the policy settles it unless: a person runs operations, the fan or stand is past tonight's
        // limits, or Agape's evidence disagrees (the runner's phone was at the seat when it was handed over)
        let fan_auto = self.problems.iter().filter(|p| p.fan == fan && matches!(p.state, ProblemState::Remedied { .. })).count();
        let stand_auto = self.problems.iter().filter(|p| p.stand == stand && matches!(p.state, ProblemState::Remedied { .. })).count();
        let at_seat = o.handoff_m.map_or(false, |m| m < self.policy.handoff_radius_m);
        let held = if !rule.auto { Some("This kind is always looked at".to_string()) }
            else if self.human("ops") { Some("Operations decides tonight".into()) }
            else if fan_auto >= self.policy.auto_per_fan { Some("Past this fan's claims for the night".into()) }
            else if stand_auto >= self.policy.auto_per_stand { Some("Past this stand's claims for the night".into()) }
            else if reason == "never_arrived" && at_seat { Some(format!("Handed over within {} m of the seat", self.policy.handoff_radius_m)) }
            else { None };
        self.problems.push(Problem { id: id.clone(), order, fan: fan.into(), stand, reason: reason.into(), words: rule.words.clone(), note: note.trim().chars().take(280).collect(),
            t: self.t, photo, fault, state: ProblemState::Open, paid_by: vec![], held: held.clone() });
        if held.is_none() { self.remedy(&id, "agape", None, None)?; }
        Ok(id)
    }

    /// The party a rule's "stand", "runner" or "agape" means on this order.
    fn party_for(&self, order: usize, who: &str) -> String {
        let q = &self.orders[order].c.quote;
        match who { "stand" => q.stand.clone(), "runner" => q.runner.clone().unwrap_or_else(|| DISPATCHER.into()), _ => DISPATCHER.into() }
    }

    /// A person decides: make it right (optionally against someone else, or as a refund instead of credit), or decline.
    pub fn resolve_problem(&mut self, by: &str, id: &str, remedy: bool, why: &str) -> Result<(), String> { self.decide(by, id, remedy, None, None, why) }
    pub fn decide(&mut self, by: &str, id: &str, remedy: bool, fault: Option<&str>, form: Option<Form>, why: &str) -> Result<(), String> {
        let i = self.problems.iter().position(|p| p.id == id).ok_or("Unknown problem")?;
        if self.problems[i].state != ProblemState::Open { return Err("Already decided".into()); }
        if !remedy {
            self.problems[i].state = ProblemState::Declined { why: if why.trim().is_empty() { "Operations looked into it".into() } else { why.trim().into() } };
            return Ok(());
        }
        self.remedy(id, by, fault, form)
    }

    /// Post the remedy: each part the rule covers, from whoever it names, to the fan as credit or to their card. One balanced transaction.
    fn remedy(&mut self, id: &str, by: &str, fault: Option<&str>, form: Option<Form>) -> Result<(), String> {
        let i = self.problems.iter().position(|p| p.id == id).ok_or("Unknown problem")?;
        let p = self.problems[i].clone();
        let rule = self.policy.rules.get(&p.reason).cloned().ok_or("That kind of problem isn't in the policy any more")?;
        if let Some(f) = fault { self.problems[i].fault = self.party_for(p.order, f); }
        let fault = self.problems[i].fault.clone();
        let q = self.orders[p.order].c.quote.clone();
        let b = breakdown(&q, &self.terms).map_err(|e| format!("{e:?}"))?;
        let form = form.or(rule.form).unwrap_or(self.policy.form);
        let usd = Unit::usd();
        let to = acct(&p.fan, match form { Form::Credit => "credit", Form::Refund => "card" });
        let mut post = vec![];
        let mut paid: Vec<(String, i64)> = vec![];
        let mut total = 0;
        for cv in &rule.covers {
            let whole = match cv.part { Part::Items => b.subtotal - b.discount, Part::Tax => b.tax, Part::Fee => b.fee, Part::Tip => b.tip };
            let amount = bps(whole, cv.bps);
            if amount <= 0 { continue; }
            let who = match cv.from { Source::Fault => fault.clone(), Source::Stand => q.stand.clone(), Source::Runner => q.runner.clone().unwrap_or_else(|| DISPATCHER.into()),
                                      Source::Agape => DISPATCHER.into(), Source::Tax => "tax_authority".into() };
            // where that party's share of this order sits: a stand's sale, a runner's pay or tips, Agape's revenue, the tax collected
            let from = if who == "tax_authority" { acct("tax_authority", "payable") }
                else if who == DISPATCHER { acct("agape", "revenue") }
                else if Some(&who) == q.runner.as_ref() { acct(&who, if cv.part == Part::Tip { "tips" } else { "pay" }) }
                else if cv.part == Part::Tip && q.runner.is_none() { acct(&who, "tips") }
                else { acct(&who, "revenue") };
            post.push(posting(from, usd.clone(), -amount));
            match paid.iter_mut().find(|x| x.0 == who) { Some(x) => x.1 += amount, None => paid.push((who, amount)) }
            total += amount;
        }
        if total == 0 { return Err("Nothing to return under this policy".into()); }
        post.push(posting(to, usd, total));
        self.ledger.post(Txn { memo: format!("remedy {} {id}", p.order), postings: post }).map_err(|e| format!("{e:?}"))?;
        self.problems[i].state = ProblemState::Remedied { amount: total, form, by: by.into() };
        self.problems[i].paid_by = paid;
        Ok(())
    }

    /// Agape credit the fan holds goes toward an order before their card does.
    pub(crate) fn use_credit(&mut self, order: usize, charge: i64) {
        let fan = self.orders[order].c.quote.fan.clone();
        let have = self.ledger.balance(&acct(&fan, "credit"), &Unit::usd());
        let x = have.min(charge);
        if x <= 0 { return; }
        if self.ledger.post(Txn { memo: format!("credit {order}"), postings: vec![posting(acct(&fan, "credit"), Unit::usd(), -x), posting(acct(&fan, "card"), Unit::usd(), x)] }).is_ok() {
            self.orders[order].credit_used = x;
        }
    }

    /// The simulated crowd complains now and then; a few fans complain about nearly everything.
    pub(crate) fn crowd_claim(&mut self, order: usize) {
        let fan = self.orders[order].c.quote.fan.clone();
        if self.live.contains(&fan) || !self.orders[order].c.quote.delivery { return; }
        let n: u64 = fan.trim_start_matches("fan").parse().unwrap_or(1);
        let serial = n % 211 == 7;
        if !self.rng.chance(if serial { 0.6 } else { 0.012 }) { return; }
        let reason = ["quality", "wrong_item", "missing_item", "damaged", "never_arrived"][(self.rng.below(100) as usize * if serial { 3 } else { 1 }) % 5];
        let _ = self.report_with(&fan, order, reason, "", true);
    }

    /// The handoff proof an order needs, from its fan's record (or what Agape's team set for them).
    pub fn proof_of(&self, order: usize) -> Proof {
        let fan = &self.orders[order].c.quote.fan;
        self.proof_set.get(fan).copied().unwrap_or_else(|| { let r = self.fan_record(fan); r.proof })
    }
    /// The PIN on the fan's phone for this order.
    pub fn pin_of(&self, order: usize) -> String {
        let fan = &self.orders[order].c.quote.fan;
        let h = fan.bytes().fold(order as u64 * 2_654_435_761, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
        format!("{:04}", h % 10_000)
    }

    /// Everything Agape knows about how a fan orders and complains. Agape's team only.
    pub fn fan_record(&self, fan: &str) -> Record {
        let mine: Vec<usize> = (0..self.orders.len()).filter(|&i| self.orders[i].c.quote.fan == fan && self.orders[i].c.stage == Stage::Settled).collect();
        let (mut spent, mut earned) = (0, 0);
        for &i in &mine {
            let q = &self.orders[i].c.quote;
            if let Ok(b) = breakdown(q, &self.terms) {
                spent += b.charge;
                earned += b.agape_share + b.fee + b.processing - b.interchange - if q.delivery { q.runner_pay.unwrap_or(self.terms.runner_pay) } else { 0 };
            }
        }
        let claims: Vec<&Problem> = self.problems.iter().filter(|p| p.fan == fan).collect();
        let remedied: i64 = claims.iter().filter_map(|p| match p.state { ProblemState::Remedied { amount, .. } => Some(amount), _ => None }).sum();
        let cost: i64 = claims.iter().flat_map(|p| p.paid_by.iter()).filter(|x| x.0 == DISPATCHER).map(|x| x.1).sum();
        let given: Vec<i64> = self.ratings.iter().filter(|r| r.3 == fan).map(|r| r.2).collect();
        let avg = if given.is_empty() { None } else { Some(given.iter().sum::<i64>() as f64 / given.len() as f64) };
        let claim_bps = claims.len() as i64 * 10_000 / mine.len().max(1) as i64;
        let auto = self.policy.proof_for(mine.len(), claims.len());
        Record { orders: mine.len(), spent, claims: claims.len(), claim_bps, remedied, earned, cost, net: earned - cost, ratings_given: given.len(), avg_given: avg,
                 proof: self.proof_set.get(fan).copied().unwrap_or(auto), proof_set: self.proof_set.contains_key(fan) }
    }

    /// Agape's team sets the policy (checked before it takes effect), or a fan's proof by hand.
    pub fn set_policy(&mut self, p: crate::remedy::Policy) -> Result<(), String> { p.check()?; self.policy = p; Ok(()) }
    pub fn set_proof(&mut self, fan: &str, proof: Option<Proof>) { match proof { Some(p) => { self.proof_set.insert(fan.into(), p); } None => { self.proof_set.remove(fan); } } }

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

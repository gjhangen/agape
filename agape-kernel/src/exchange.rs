//! One path for every exchange beyond the food order: tickets, rides, parking, perks, deals, card
//! trades and drops. Each is an Offer a party makes, accepted by another party into an Agreement
//! that moves through one lifecycle and settles in the ledger.
//!
//! One world (invariant 3): live state is a fold over signals. Every change to an offer or an
//! agreement is first appended to the log as a signal, and `apply` is the only code that changes
//! either. `Book::fold(&log)` rebuilds the same state from the log alone.
//!
//! Attribution (invariant 8): every agreement names the offer it accepted and where it was seen
//! (`via`), and every ledger transaction it causes carries its id in the memo.

use crate::deals::Pay;
use crate::signal::{Signal, Val};
use crate::value::PartyId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Kind { Ticket, Parking, Perk, Ride, Deal, Card, Drop, Upgrade, Resale }

/// Who an offer is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope { Fans, Businesses, Anyone }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Offer {
    pub id: String,
    pub kind: Kind,
    /// The party offering, who is paid (or, for a ride, whose company is).
    pub maker: PartyId,
    pub title: String,
    pub blurb: String,
    pub pay: Pay,
    /// The base price; the exact price for given terms comes from the quote.
    pub price: i64,
    pub scope: Scope,
    /// How many live agreements it will take (None: capacity comes from inventory).
    pub limit: Option<i64>,
    /// What it's about: an event and section, a zone, a lot, a place, an edition, a party.
    pub subject: String,
    /// The finer kind within its kind ("shoutout", "tunnel", "promo", ...).
    pub sub: String,
    pub opens: i64,
    pub closes: Option<i64>,
    #[serde(default = "yes")]
    pub open: bool,
}
fn yes() -> bool { true }

/// One lifecycle for every agreement. Not every kind uses every stage: a ticket is settled the
/// moment it's bought; a deal goes through all five.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage { Held, Active, Fulfilled, Settled, Closed }

/// Every way an agreement can end without being settled, with whose doing it was and what it costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exit { BuyerCancelled, BuyerWithdrew, MakerDeclined, NoDoer }

impl Exit {
    pub fn name(&self) -> &'static str { match self { Exit::BuyerCancelled => "buyer_cancelled", Exit::BuyerWithdrew => "buyer_withdrew", Exit::MakerDeclined => "maker_declined", Exit::NoDoer => "no_doer" } }
    pub fn parse(s: &str) -> Option<Exit> { [Exit::BuyerCancelled, Exit::BuyerWithdrew, Exit::MakerDeclined, Exit::NoDoer].into_iter().find(|e| e.name() == s) }
    /// Whose doing it was. The party at fault is the one who bears any cost; nobody here is
    /// charged for an exit, and whatever was held goes back to the buyer in full.
    pub fn fault<'a>(&self, a: &'a Agreement) -> Option<&'a str> {
        match self { Exit::BuyerCancelled | Exit::BuyerWithdrew => Some(&a.buyer), Exit::MakerDeclined => None, Exit::NoDoer => Some(&a.maker) }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Agreement {
    pub id: String,
    pub offer: String,
    pub kind: Kind,
    pub buyer: PartyId,
    pub maker: PartyId,
    /// Who does the work, once known (a driver).
    pub doer: Option<PartyId>,
    pub qty: i64,
    pub amount: i64,
    pub pay: Pay,
    /// When it's for (a ride's pickup).
    pub at: Option<i64>,
    /// What was agreed in detail: the seats, the fan's note.
    pub detail: String,
    /// What the doer says they did.
    pub proof: Option<String>,
    /// A finer step within Active ("assigned", "waiting", "on_board").
    pub step: String,
    pub stage: Stage,
    pub exit: Option<Exit>,
    /// Where the buyer saw the offer: a page, a screen section.
    pub via: String,
    pub made: i64,
    pub history: Vec<(i64, Stage, String)>,
}

impl Agreement {
    pub fn live(&self) -> bool { !matches!(self.stage, Stage::Settled | Stage::Closed) }
    pub fn changed(&self) -> i64 { self.history.last().map_or(self.made, |h| h.0) }
}

/// Every offer and agreement, folded from the log.
#[derive(Default, Clone)]
pub struct Book {
    offers: BTreeMap<String, Offer>,
    agreements: Vec<Agreement>,
    index: BTreeMap<String, usize>,
}

fn s(sig: &Signal, k: &str) -> String { sig.fields.get(k).map(|v| match v { Val::S(x) => x.clone(), other => other.key() }).unwrap_or_default() }
fn i(sig: &Signal, k: &str) -> i64 { sig.fields.get(k).and_then(|v| v.as_i()).unwrap_or(0) }

pub fn stage_name(st: Stage) -> &'static str { match st { Stage::Held => "held", Stage::Active => "active", Stage::Fulfilled => "fulfilled", Stage::Settled => "settled", Stage::Closed => "closed" } }
fn stage_of(x: &str) -> Option<Stage> { [Stage::Held, Stage::Active, Stage::Fulfilled, Stage::Settled, Stage::Closed].into_iter().find(|st| stage_name(*st) == x) }

impl Book {
    pub fn offer(&self, id: &str) -> Option<&Offer> { self.offers.get(id) }
    pub fn offers(&self) -> impl Iterator<Item = &Offer> { self.offers.values() }
    pub fn get(&self, id: &str) -> Option<&Agreement> { self.index.get(id).map(|&k| &self.agreements[k]) }
    pub fn all(&self) -> &[Agreement] { &self.agreements }
    pub fn next_id(&self) -> String { format!("a{}", self.agreements.len() + 1) }
    pub fn of_kind(&self, k: Kind) -> impl DoubleEndedIterator<Item = &Agreement> { self.agreements.iter().filter(move |a| a.kind == k) }
    /// Live agreements on an offer (for limits) and how many a party holds on it.
    pub fn taken(&self, offer: &str) -> i64 { self.agreements.iter().filter(|a| a.offer == offer && a.stage != Stage::Closed).count() as i64 }

    /// The only way state changes. Signals that don't concern the book are ignored.
    pub fn apply(&mut self, sig: &Signal) {
        match sig.kind.as_str() {
            "offer" => {
                let id = s(sig, "offer");
                match s(sig, "act").as_str() {
                    "open" => if let Ok(o) = serde_json::from_str::<Offer>(&s(sig, "spec")) { self.offers.insert(id, o); },
                    "close" => if let Some(o) = self.offers.get_mut(&id) { o.open = false; },
                    _ => {}
                }
            }
            "exchange" => {
                let id = s(sig, "agreement");
                let Some(st) = stage_of(&s(sig, "stage")) else { return };
                let step = s(sig, "step");
                let detail = s(sig, "detail");
                if let Some(&k) = self.index.get(&id) {
                    let a = &mut self.agreements[k];
                    a.stage = st;
                    match st {
                        Stage::Active => { if let Some(d) = step.strip_prefix("doer:") { a.doer = Some(d.into()); a.step = "assigned".into(); } else if !step.is_empty() { a.step = step.clone(); } }
                        Stage::Fulfilled => { if !detail.is_empty() { a.proof = Some(detail.clone()); } }
                        Stage::Closed => { a.exit = Exit::parse(&step); }
                        _ => {}
                    }
                    a.history.push((sig.t_ms, st, step));
                } else if let Some(o) = self.offers.get(&s(sig, "offer")) {
                    let at = i(sig, "at");
                    let a = Agreement { id: id.clone(), offer: o.id.clone(), kind: o.kind, buyer: sig.author.clone(), maker: o.maker.clone(), doer: None,
                        qty: i(sig, "qty"), amount: i(sig, "amount"), pay: o.pay.clone(), at: if at > 0 { Some(at) } else { None }, detail, proof: None,
                        step: String::new(), stage: st, exit: None, via: s(sig, "via"), made: sig.t_ms, history: vec![(sig.t_ms, st, step)] };
                    self.index.insert(id, self.agreements.len());
                    self.agreements.push(a);
                }
            }
            _ => {}
        }
    }

    /// Rebuild everything from the log alone.
    pub fn fold(signals: &[Signal]) -> Book {
        let mut b = Book::default();
        for sig in signals { b.apply(sig); }
        b
    }

    /// Two books hold the same world.
    pub fn same_as(&self, other: &Book) -> bool {
        self.agreements == other.agreements
            && self.offers.len() == other.offers.len()
            && self.offers.iter().all(|(k, o)| other.offers.get(k).map_or(false, |p| serde_json::to_string(o).ok() == serde_json::to_string(p).ok()))
    }
}

/// The kinds of transition each kind of agreement allows, from where, and by whom.
/// Returns an error in words if the move isn't allowed.
pub fn allowed(a: &Agreement, to: Stage, by: &str, exit: Option<Exit>, system: bool) -> Result<(), &'static str> {
    use Stage::*;
    let buyer = by == a.buyer;
    let maker = by == a.maker;
    let doer = a.doer.as_deref() == Some(by);
    // None: no such move from here. Some(false): the move exists but isn't this party's to make.
    let who: Option<bool> = match (a.kind, a.stage, to) {
        // rides: the company assigns a driver, the trip runs, the fare settles at drop-off
        (Kind::Ride, Held, Active) | (Kind::Ride, Active, Active) | (Kind::Ride, Active, Fulfilled) | (Kind::Ride, Fulfilled, Settled) => Some(system || maker || doer),
        (Kind::Ride, Held, Closed) => Some((buyer && exit == Some(Exit::BuyerCancelled)) || (system && exit == Some(Exit::NoDoer))),
        (Kind::Ride, Active, Closed) if a.step == "assigned" => Some(buyer && exit == Some(Exit::BuyerCancelled)),
        // deals: the maker answers and does it; the buyer releases the pay (or it releases itself)
        (Kind::Deal, Held, Active) => Some(maker),
        (Kind::Deal, Held, Closed) => Some((maker && exit == Some(Exit::MakerDeclined)) || (buyer && exit == Some(Exit::BuyerWithdrew))),
        (Kind::Deal, Active, Fulfilled) => Some(maker),
        (Kind::Deal, Fulfilled, Settled) => Some(buyer || system),
        _ => None,
    };
    match who {
        None => return Err("That has moved on"),
        Some(false) => return Err("That isn't your deal to act on"),
        Some(true) => {}
    }
    Ok(())
}

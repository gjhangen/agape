//! Commitments: an accepted offer moving through stages. Each stage is advanced only by the
//! party who owns it; every failure is a named exit with a fault owner (invariant 5).
//! Money is authorized at acceptance and captured only at settlement.

use crate::economy::{breakdown, hold, release, settlement, Quote, Terms};
use crate::value::{acct, posting, Ledger, LedgerError, PartyId, Txn, Unit};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage { Accepted, Preparing, Ready, Assigned, PickedUp, Delivered, Disputed, Settled, Closed(Exit) }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Exit {
    /// Fan cancelled before preparation started. Nobody is charged.
    FanCancelled,
    /// The stand could not make it. Nobody is charged; the stand owns the miss.
    StandCannotFulfil,
    /// Payment authorization failed before preparation. Nobody is charged.
    PaymentFailed,
    /// Runner reached the seat and the fan was not there. Order captured, tip released.
    FanUnreachable,
    /// A dispute was upheld: the fan is refunded in full by the party at fault.
    DisputeUpheld { fault: PartyId },
    /// The venue stopped service (an emergency). Nobody is charged; the venue owns it.
    VenueHalted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    StartPrep, MarkReady, CannotFulfil, Assign(PartyId), PickUp, Deliver, Unreachable,
    RunnerDropped, Cancel, PaymentFailed, Dispute, Resolve { upheld: bool }, Settle, VenueHalt,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CommitError {
    /// The action is not valid in this stage.
    WrongStage { stage: Stage, action: Action },
    /// The actor does not own this stage. The signal is kept as a claim, the stage does not move.
    NotOwner { actor: PartyId, owner: PartyId },
    Ledger(LedgerError),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commitment {
    pub id: String,
    pub quote: Quote,
    pub stage: Stage,
    /// Seconds late, set by the dispatcher when a deadline is missed; credited at settlement.
    pub late_by: Option<(PartyId, i64)>,
    pub history: Vec<(Stage, PartyId)>,
}

pub const DISPATCHER: &str = "agape";
pub const LATE_CREDIT: i64 = 300;

impl Commitment {
    /// Accept an offer: set aside everything settlement will need. If any of it is not there
    /// (sponsor budget spent, points short, item out), acceptance is refused and nothing moves.
    pub fn accept(id: &str, quote: Quote, ledger: &mut Ledger) -> Result<Commitment, LedgerError> {
        ledger.post(hold(&quote))?;
        Ok(Self::accept_held(id, quote))
    }

    /// Accept an order whose hold was already posted when the fan tapped (a claim).
    pub fn accept_held(id: &str, quote: Quote) -> Commitment {
        Commitment { id: id.into(), quote, stage: Stage::Accepted, late_by: None, history: vec![] }
    }

    fn owner(&self, a: &Action) -> PartyId {
        let runner = self.quote.runner.clone().unwrap_or_default();
        match a {
            Action::StartPrep | Action::MarkReady | Action::CannotFulfil => self.quote.stand.clone(),
            Action::Assign(_) | Action::RunnerDropped | Action::Resolve { .. } | Action::Settle | Action::PaymentFailed => DISPATCHER.into(),
            Action::Deliver if !self.quote.delivery => self.quote.stand.clone(), // pickup: handed over at the stand
            Action::PickUp | Action::Deliver | Action::Unreachable => runner,
            Action::Cancel | Action::Dispute => self.quote.fan.clone(),
            Action::VenueHalt => "venue".into(),
        }
    }

    /// Apply an action by an actor. Money moves only on Settle and on exits that capture.
    pub fn act(&mut self, a: Action, actor: &str, ledger: &mut Ledger, t: &Terms) -> Result<&Stage, CommitError> {
        let owner = self.owner(&a);
        if actor != owner { return Err(CommitError::NotOwner { actor: actor.into(), owner }); }
        use Stage::*;
        let delivery = self.quote.delivery;
        let next = match (&self.stage, &a) {
            (Accepted, Action::StartPrep) => Preparing,
            (Accepted, Action::Cancel) => { self.release(ledger)?; Closed(Exit::FanCancelled) }
            (Accepted, Action::PaymentFailed) => { self.release(ledger)?; Closed(Exit::PaymentFailed) }
            (Accepted | Preparing, Action::CannotFulfil) => { self.release(ledger)?; Closed(Exit::StandCannotFulfil) }
            (Accepted | Preparing, Action::VenueHalt) => { self.release(ledger)?; Closed(Exit::VenueHalted) }
            (Preparing, Action::MarkReady) => Ready,
            (Ready, Action::Assign(r)) if delivery => { self.quote.runner = Some(r.clone()); Assigned }
            (Assigned, Action::RunnerDropped) => { self.quote.runner = None; Ready }
            (Assigned, Action::PickUp) => PickedUp,
            (PickedUp, Action::Deliver) => Delivered,
            (Ready, Action::Deliver) if !delivery => Delivered, // pickup handoff at the stand
            (PickedUp, Action::Unreachable) => {
                let mut q = self.quote.clone();
                q.tip = 0;
                self.capture(&q, t, ledger)?;
                Closed(Exit::FanUnreachable)
            }
            (Delivered, Action::Dispute) => Disputed,
            (Disputed, Action::Resolve { upheld: false }) | (Delivered, Action::Settle) => {
                let q = self.quote.clone();
                self.capture(&q, t, ledger)?;
                Settled
            }
            (Disputed, Action::Resolve { upheld: true }) => {
                // The fan pays nothing; the stand did not deliver what was promised.
                // Discount budget and points go back; the units were handed over and are written off.
                let mut q = self.quote.clone();
                q.lines.clear();
                if !q.discounts.is_empty() || q.points_redeemed > 0 { ledger.post(release(&q)).map_err(CommitError::Ledger)?; }
                for l in &self.quote.lines {
                    ledger.post(Txn { memo: "write_off".into(), postings: vec![
                        posting(acct(&self.quote.stand, "reserved"), Unit::Thing(l.thing.clone()), -l.qty),
                        posting(acct(&self.quote.stand, "written_off"), Unit::Thing(l.thing.clone()), l.qty),
                    ]}).map_err(CommitError::Ledger)?;
                }
                Closed(Exit::DisputeUpheld { fault: self.quote.stand.clone() })
            }
            (s, _) => return Err(CommitError::WrongStage { stage: s.clone(), action: a }),
        };
        self.history.push((next.clone(), actor.into()));
        self.stage = next;
        Ok(&self.stage)
    }

    fn release(&self, ledger: &mut Ledger) -> Result<(), CommitError> {
        ledger.post(release(&self.quote)).map_err(CommitError::Ledger)
    }

    fn capture(&self, q: &Quote, t: &Terms, ledger: &mut Ledger) -> Result<(), CommitError> {
        let b = breakdown(q, t).expect("quote was checked at offer time");
        let mut txn = settlement(q, t, &b);
        if let Some((fault, _)) = &self.late_by {
            let usd = Unit::Money(t.currency.clone());
            let from = if fault == DISPATCHER { acct("agape", "revenue") } else { acct(fault, "revenue") };
            txn.postings.push(posting(from, usd.clone(), -LATE_CREDIT));
            // as Agape credit toward the next order, like every remedy by default
            txn.postings.push(posting(acct(&q.fan, "credit"), usd, LATE_CREDIT));
            txn.memo = "settle+late_credit".into();
        }
        ledger.post(txn).map_err(CommitError::Ledger)
    }
}

pub fn stock(ledger: &mut Ledger, stand: &str, thing: &str, qty: i64) -> Result<(), LedgerError> {
    ledger.post(Txn { memo: "stock".into(), postings: vec![
        posting(acct("world", "supply"), Unit::Thing(thing.into()), -qty),
        posting(acct(stand, "inventory"), Unit::Thing(thing.into()), qty),
    ]})
}

pub fn fund(ledger: &mut Ledger, payer: &str, cents: i64) -> Result<(), LedgerError> {
    ledger.post(Txn { memo: "fund".into(), postings: vec![
        posting(acct(payer, "bank"), Unit::usd(), -cents),
        posting(acct(payer, "promo_budget"), Unit::usd(), cents),
    ]})
}

/// The standard policy: external accounts (cards, banks, the world's supply) may go negative;
/// revenue, pay and tips may go negative to absorb a credit or remedy; inventory and budgets may not.
pub fn standard_policy() -> crate::value::Policy {
    crate::value::Policy { may_go_negative: vec![
        (None, Some("card".into())), (None, Some("bank".into())), (Some("world".into()), None), (None, Some("revenue".into())),
        // a remedy can take back a runner's pay or tip, or the tax collected on a sale it unwinds
        (None, Some("pay".into())), (None, Some("tips".into())), (None, Some("payable".into())),
    ]}
}

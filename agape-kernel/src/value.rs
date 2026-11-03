//! Value: exact quantities in units, held in accounts, moved only by balanced transactions.
//! Invariant 2 (conservation) lives here: every transaction sums to zero in every unit.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type PartyId = String;

/// A unit of value (amendment A1): money per currency, points per issuer, units of a thing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Unit {
    /// Integer minor units (cents) of an ISO currency (A6).
    Money(String),
    /// Points issued by a named party, who carries them as a liability.
    Points(PartyId),
    /// Units of a thing: inventory, seats, tickets.
    Thing(String),
}

impl Unit {
    pub fn usd() -> Unit { Unit::Money("USD".into()) }
}

/// An account is a party plus a purpose.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Account {
    pub party: PartyId,
    pub purpose: String,
}

pub fn acct(party: &str, purpose: &str) -> Account {
    Account { party: party.into(), purpose: purpose.into() }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Posting {
    pub account: Account,
    pub unit: Unit,
    pub amount: i64,
}

pub fn posting(account: Account, unit: Unit, amount: i64) -> Posting {
    Posting { account, unit, amount }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Txn {
    pub memo: String,
    pub postings: Vec<Posting>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LedgerError {
    Unbalanced { unit: Unit, sum: i64 },
    Insufficient { account: Account, unit: Unit, balance: i64, needed: i64 },
    Overflow,
    Empty,
}

/// Which accounts may go negative. Issuers (points) and external parties (the fan's
/// card, which is outside Agape) can; inventory, budgets and escrow cannot.
#[derive(Clone, Debug, Default)]
pub struct Policy {
    pub may_go_negative: Vec<(Option<PartyId>, Option<String>)>,
}

impl Policy {
    fn allows_negative(&self, a: &Account, u: &Unit) -> bool {
        if let Unit::Points(issuer) = u {
            if *issuer == a.party && a.purpose == "issued" { return true; }
        }
        self.may_go_negative.iter().any(|(p, pur)| {
            p.as_ref().map_or(true, |p| *p == a.party) && pur.as_ref().map_or(true, |x| *x == a.purpose)
        })
    }
}

#[derive(Default)]
pub struct Ledger {
    pub txns: Vec<Txn>,
    /// When each transaction posted (ms), parallel to `txns`. Set `now` before posting.
    pub times: Vec<i64>,
    pub now: i64,
    balances: BTreeMap<(Account, Unit), i64>,
    pub policy: Policy,
}

impl Ledger {
    pub fn new(policy: Policy) -> Self { Ledger { txns: vec![], times: vec![], now: 0, balances: BTreeMap::new(), policy } }

    pub fn balance(&self, a: &Account, u: &Unit) -> i64 {
        *self.balances.get(&(a.clone(), u.clone())).unwrap_or(&0)
    }

    /// Net position of a party in a unit, across all its accounts.
    pub fn net(&self, party: &str, u: &Unit) -> i64 {
        self.balances.iter().filter(|((a, uu), _)| a.party == party && uu == u).map(|(_, v)| v).sum()
    }

    pub fn check(t: &Txn) -> Result<(), LedgerError> {
        if t.postings.is_empty() { return Err(LedgerError::Empty); }
        let mut sums: BTreeMap<&Unit, i64> = BTreeMap::new();
        for p in &t.postings {
            let s = sums.entry(&p.unit).or_insert(0);
            *s = s.checked_add(p.amount).ok_or(LedgerError::Overflow)?;
        }
        for (u, s) in sums {
            if s != 0 { return Err(LedgerError::Unbalanced { unit: u.clone(), sum: s }); }
        }
        Ok(())
    }

    /// Post a transaction atomically: all postings apply or none do.
    pub fn post(&mut self, t: Txn) -> Result<(), LedgerError> {
        Self::check(&t)?;
        let mut next: BTreeMap<(Account, Unit), i64> = BTreeMap::new();
        for p in &t.postings {
            let k = (p.account.clone(), p.unit.clone());
            let cur = *next.get(&k).unwrap_or(&self.balance(&p.account, &p.unit));
            let nb = cur.checked_add(p.amount).ok_or(LedgerError::Overflow)?;
            next.insert(k, nb);
        }
        for ((a, u), b) in &next {
            if *b < 0 && !self.policy.allows_negative(a, u) {
                return Err(LedgerError::Insufficient {
                    account: a.clone(), unit: u.clone(), balance: self.balance(a, u), needed: self.balance(a, u) - b,
                });
            }
        }
        self.balances.extend(next);
        self.txns.push(t);
        self.times.push(self.now);
        Ok(())
    }

    /// Sum of all balances in a unit. Always zero if conservation holds.
    pub fn total(&self, u: &Unit) -> i64 {
        self.balances.iter().filter(|((_, uu), _)| uu == u).map(|(_, v)| v).sum()
    }

    pub fn balances_iter(&self) -> impl Iterator<Item = (&(Account, Unit), &i64)> { self.balances.iter() }

    pub fn units(&self) -> Vec<Unit> {
        let mut v: Vec<Unit> = self.balances.keys().map(|(_, u)| u.clone()).collect();
        v.sort(); v.dedup(); v
    }
}

/// amount * bps / 10_000 with banker's rounding (round half to even), exact (A6).
pub fn bps(amount: i64, bps: i64) -> i64 {
    let n = amount as i128 * bps as i128;
    let (q, r) = (n.div_euclid(10_000), n.rem_euclid(10_000));
    let q = if r * 2 > 10_000 || (r * 2 == 10_000 && q % 2 != 0) { q + 1 } else { q };
    q as i64
}

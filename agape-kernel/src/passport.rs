//! The passport: a fan is one person across every venue. Their points, level, and where they've
//! been travel with them. Each venue keeps its own ledger, so moving between venues is a transfer
//! with an id: points leave one venue's ledger through its passport account and enter the next's.
//! Both sides are journaled and idempotent, so a crash in the middle of a move is finished on
//! restart by repeating it, never by guessing.

use crate::sim::Night;
use crate::value::{acct, posting, Txn};
use serde_json::{json, Value};

/// What a venue hands back when a fan leaves it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CarriedOut { pub points: i64, pub level: i64 }

impl Night {
    /// The fan's level here: everything Agape has issued them, here and everywhere before.
    pub fn level(&self, fan: &str) -> i64 { crate::social::lifetime(&self.ledger, fan) + self.level_offset.get(fan).copied().unwrap_or(0) }

    /// A fan arrives with their passport. Repeating the same transfer does nothing.
    pub fn carry_in(&mut self, transfer: &str, fan: &str, points: i64, level: i64, passport: Value) -> Result<(), String> {
        if transfer.is_empty() || points < 0 { return Err("Bad transfer".into()); }
        if self.transfers.contains_key(transfer) { return Ok(()); }
        if points > 0 {
            let pts = crate::social::points();
            self.ledger.post(Txn { memo: format!("passport in {transfer}"), postings: vec![posting(acct("passport", "bank"), pts.clone(), -points), posting(acct(fan, "points"), pts, points)] })
                .map_err(|e| format!("{e:?}"))?;
        }
        // level already earned elsewhere counts here; what's earned here adds to it
        let here = crate::social::lifetime(&self.ledger, fan);
        self.level_offset.insert(fan.into(), level - here);
        self.passports.insert(fan.into(), passport);
        self.transfers.insert(transfer.into(), CarriedOut { points, level });
        Ok(())
    }

    /// A fan leaves for another venue: their spendable points go with them. Points held in a
    /// listing, an escrow or a claim stay until that settles. Repeating the same transfer returns
    /// what it returned the first time.
    pub fn carry_out(&mut self, transfer: &str, fan: &str) -> Result<CarriedOut, String> {
        if transfer.is_empty() { return Err("Bad transfer".into()); }
        if let Some(done) = self.transfers.get(transfer) { return Ok(done.clone()); }
        let pts = crate::social::points();
        let points = self.ledger.balance(&acct(fan, "points"), &pts).max(0);
        let level = self.level(fan);
        if points > 0 {
            self.ledger.post(Txn { memo: format!("passport out {transfer}"), postings: vec![posting(acct(fan, "points"), pts.clone(), -points), posting(acct("passport", "bank"), pts, points)] })
                .map_err(|e| format!("{e:?}"))?;
        }
        let out = CarriedOut { points, level };
        self.transfers.insert(transfer.into(), out.clone());
        Ok(out)
    }

    pub fn passport_json(&self, fan: &str) -> Value {
        let p = self.passports.get(fan).cloned().unwrap_or(Value::Null);
        json!({ "venues": p["venues"], "nights": p["nights"], "here": self.venue_name(), "level": self.level(fan) })
    }

    /// A league-wide deal: one rule, written once, installed in every venue's night, with this
    /// venue's share of its budget funded here so its sponsor is debited here.
    pub fn install_rule(&mut self, rule: Value, payer: &str, budget: i64) -> Result<(), String> {
        let r: crate::rules::RuleDef = serde_json::from_value(rule).map_err(|e| format!("The rule can't be read: {e}"))?;
        let schema = crate::sim::sports_schema();
        self.engine.add_rule(r, &schema).map_err(|e| format!("{e:?}"))?;
        if !payer.is_empty() && budget > 0 { crate::commitment::fund(&mut self.ledger, payer, budget).map_err(|e| format!("{e:?}"))?; }
        Ok(())
    }
}

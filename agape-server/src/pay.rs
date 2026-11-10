//! Cards on a real night, with any processor. An order's card is authorized for its price the moment
//! it's placed (declined: the order closes and the stand never makes it). Nothing is taken until
//! the order is final: then exactly what the ledger says the fan owes is captured (less credit used,
//! no tip if they couldn't be found), or the hold is released if it never happened. A refund for a
//! problem put right goes back to the same card. What the processor holds always equals the books.
//!
//! The processor is behind one small trait. The test processor runs now; connecting a real one
//! (Stripe, Adyen, a TSYS or Fiserv gateway) is writing those four calls against its API.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub trait Processor: Send {
    fn name(&self) -> &str;
    /// Hold `amount` cents on the fan's card. Returns the processor's id for the hold.
    fn authorize(&mut self, amount: i64, reference: &str, fan: &str) -> Result<String, String>;
    fn capture(&mut self, auth: &str, amount: i64) -> Result<(), String>;
    fn void(&mut self, auth: &str) -> Result<(), String>;
    fn refund(&mut self, auth: &str, amount: i64) -> Result<(), String>;
}

/// Approves everything, except a fan whose name starts with "decline" (to rehearse a declined card).
/// Moves no money; keeps its own record so the totals can be checked against the books.
#[derive(Default)]
pub struct TestProcessor { n: u64 }
impl Processor for TestProcessor {
    fn name(&self) -> &str { "test" }
    fn authorize(&mut self, amount: i64, _reference: &str, fan: &str) -> Result<String, String> {
        if amount <= 0 { return Err("Nothing to charge".into()); }
        if fan.to_lowercase().starts_with("decline") { return Err("Your card was declined".into()); }
        self.n += 1;
        Ok(format!("test_{}_{}", crate::now_s(), self.n))
    }
    fn capture(&mut self, _auth: &str, _amount: i64) -> Result<(), String> { Ok(()) }
    fn void(&mut self, _auth: &str) -> Result<(), String> { Ok(()) }
    fn refund(&mut self, _auth: &str, _amount: i64) -> Result<(), String> { Ok(()) }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Rec {
    pub auth: String, pub venue: String, pub night: u64, pub order: usize, pub fan: String, pub processor: String,
    pub authorized: i64, pub captured: i64, pub refunded: i64,
    /// "authorized", "captured", "voided" or "declined"
    pub state: String,
    pub log: Vec<(u64, String)>,
}

pub struct Payments { path: PathBuf, recs: BTreeMap<String, Rec>, proc: Box<dyn Processor> }

fn key(venue: &str, night: u64, order: usize) -> String { format!("{venue}:{night}:{order}") }

impl Payments {
    pub fn open(data: &std::path::Path) -> Payments {
        let path = data.join("payments.json");
        let recs = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Payments { path, recs, proc: Box::new(TestProcessor::default()) }
    }
    pub fn processor(&self) -> &str { self.proc.name() }

    fn save(&self) {
        let tmp = self.path.with_extension("tmp");
        if let Ok(mut f) = std::fs::File::create(&tmp) {
            use std::io::Write;
            if f.write_all(&serde_json::to_vec(&self.recs).unwrap_or_default()).and_then(|_| f.sync_all()).is_ok() { let _ = std::fs::rename(&tmp, &self.path); }
        }
    }

    pub fn known(&self, venue: &str, night: u64, order: usize) -> bool { self.recs.contains_key(&key(venue, night, order)) }

    /// Hold the order's price on the fan's card. Err means declined: close the order.
    pub fn authorize(&mut self, venue: &str, night: u64, order: usize, fan: &str, fan_name: &str, amount: i64) -> Result<(), String> {
        let k = key(venue, night, order);
        let r = self.proc.authorize(amount, &k, fan_name);
        let now = crate::now_s();
        let (auth, state, log) = match &r { Ok(a) => (a.clone(), "authorized", format!("held {amount}")), Err(e) => (String::new(), "declined", format!("declined: {e}")) };
        self.recs.insert(k, Rec { auth, venue: venue.into(), night, order, fan: fan.into(), processor: self.proc.name().into(), authorized: if r.is_ok() { amount } else { 0 }, captured: 0, refunded: 0, state: state.into(), log: vec![(now, log)] });
        self.save();
        r.map(|_| ())
    }

    /// Bring every card in line with what the ledger says each fan owes. Returns what it did.
    pub fn reconcile(&mut self, venue: &str, night: u64, due: &[Value]) -> Vec<String> {
        let mut did = vec![];
        for d in due {
            let order = d["order"].as_u64().unwrap_or(0) as usize;
            let Some(r) = self.recs.get_mut(&key(venue, night, order)) else { continue };
            let now = crate::now_s();
            if r.state == "authorized" && d["final"] == true {
                let owe = d["capture"].as_i64().unwrap_or(0);
                if owe <= 0 {
                    if self.proc.void(&r.auth).is_ok() { r.state = "voided".into(); r.log.push((now, "released the hold".into())); did.push(format!("{order}: released")); }
                } else {
                    let take = owe.min(r.authorized);
                    if take < owe { r.log.push((now, format!("owed {owe} but only {} was held", r.authorized))); }
                    match self.proc.capture(&r.auth, take) {
                        Ok(()) => { r.captured = take; r.state = "captured".into(); r.log.push((now, format!("captured {take}"))); did.push(format!("{order}: captured {take}")); }
                        Err(e) => r.log.push((now, format!("capture failed: {e}"))),
                    }
                }
            }
            if r.state == "captured" {
                let want = d["refund"].as_i64().unwrap_or(0).min(r.captured);
                if want > r.refunded {
                    let more = want - r.refunded;
                    match self.proc.refund(&r.auth, more) {
                        Ok(()) => { r.refunded = want; r.log.push((now, format!("refunded {more}"))); did.push(format!("{order}: refunded {more}")); }
                        Err(e) => r.log.push((now, format!("refund failed: {e}"))),
                    }
                }
            }
        }
        if !did.is_empty() { self.save(); }
        did
    }

    /// The night's cards, for the console and the report.
    pub fn summary(&self, venue: &str, night: u64) -> Value {
        let mine: Vec<&Rec> = self.recs.values().filter(|r| r.venue == venue && r.night == night).collect();
        let sum = |f: &dyn Fn(&Rec) -> i64| mine.iter().map(|r| f(r)).sum::<i64>();
        json!({ "processor": self.proc.name(), "holds": mine.iter().filter(|r| r.state == "authorized").count(), "held": sum(&|r| if r.state == "authorized" { r.authorized } else { 0 }),
                "captured": sum(&|r| r.captured), "refunded": sum(&|r| r.refunded), "net": sum(&|r| r.captured - r.refunded),
                "declined": mine.iter().filter(|r| r.state == "declined").count(), "voided": mine.iter().filter(|r| r.state == "voided").count() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_release_refund_follow_what_is_owed() {
        let d = tempfile::tempdir().unwrap();
        let mut p = Payments::open(d.path());
        p.authorize("v", 1, 0, "u1", "gabe", 1500).unwrap();
        p.authorize("v", 1, 1, "u1", "gabe", 900).unwrap();
        assert!(p.authorize("v", 1, 2, "u2", "declinejoe", 900).is_err());
        let due = |c0: i64, f: bool, r0: i64| vec![json!({ "order": 0, "final": f, "capture": c0, "refund": r0 }), json!({ "order": 1, "final": f, "capture": 0, "refund": 0 })];
        assert!(p.reconcile("v", 1, &due(1500, false, 0)).is_empty(), "nothing until final");
        p.reconcile("v", 1, &due(1400, true, 0));
        let s = p.summary("v", 1);
        assert_eq!((s["captured"].as_i64(), s["voided"].as_i64(), s["holds"].as_i64(), s["declined"].as_i64()), (Some(1400), Some(1), Some(0), Some(1)));
        p.reconcile("v", 1, &due(1400, true, 500));
        p.reconcile("v", 1, &due(1400, true, 500));
        assert_eq!(p.summary("v", 1)["net"], 900, "refunded once, not twice");
        // and it survives a restart
        assert_eq!(Payments::open(d.path()).summary("v", 1)["refunded"], 500);
    }
}

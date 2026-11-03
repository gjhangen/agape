//! Points policy. Points never expire by default. Because the ledger records when every
//! point was earned, expiry can be switched on later without guessing. A policy only starts
//! its clock on the day it takes effect, so nobody's existing points vanish overnight, and
//! what will expire is shown to the fan before it happens (the same no-surprise rule as offers).

use crate::value::{acct, posting, Ledger, Txn, Unit};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PointsPolicy {
    /// None (the default): points never expire.
    #[serde(default)]
    pub expire_after_ms: Option<i64>,
    /// When the policy took effect. A point's clock starts at the later of its earning and this.
    #[serde(default)]
    pub effective_from_ms: i64,
}

fn pts() -> Unit { Unit::Points("agape".into()) }

/// The fan's unspent points as lots (earned at, amount), oldest first. Spending, holding and
/// expiry consume the oldest lots first; points coming back (a released hold) form a new lot.
pub fn lots(ledger: &Ledger, fan: &str) -> Vec<(i64, i64)> {
    let a = acct(fan, "points");
    let mut lots: Vec<(i64, i64)> = vec![];
    for (txn, &t) in ledger.txns.iter().zip(&ledger.times) {
        for p in txn.postings.iter().filter(|p| p.account == a && p.unit == pts()) {
            if p.amount > 0 { lots.push((t, p.amount)); continue; }
            let mut need = -p.amount;
            for lot in lots.iter_mut() {
                let take = lot.1.min(need);
                lot.1 -= take; need -= take;
                if need == 0 { break; }
            }
            lots.retain(|l| l.1 > 0);
        }
    }
    lots
}

/// Points that will have expired by `by`, if a policy is on: what the fan is told ahead of time.
pub fn expiring(ledger: &Ledger, fan: &str, policy: &PointsPolicy, by: i64) -> i64 {
    let Some(after) = policy.expire_after_ms else { return 0 };
    lots(ledger, fan).iter().filter(|(t, _)| (*t).max(policy.effective_from_ms) + after <= by).map(|(_, n)| n).sum()
}

/// Post expiry for every fan whose oldest points have passed the policy's age. Returns points expired.
pub fn expire(ledger: &mut Ledger, fans: &[String], policy: &PointsPolicy) -> i64 {
    if policy.expire_after_ms.is_none() { return 0; }
    let now = ledger.now;
    let mut total = 0;
    for f in fans {
        let n = expiring(ledger, f, policy, now);
        if n > 0 {
            ledger.post(Txn { memo: "points_expired".into(), postings: vec![
                posting(acct(f, "points"), pts(), -n),
                posting(acct("agape", "issued"), pts(), n),
            ]}).expect("lots are a view of this balance");
            total += n;
        }
    }
    total
}

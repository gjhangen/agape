//! One order, seen by everyone it touches: the fan who placed it, the stand that makes it, the
//! runner who carries it with other seats' orders, and where each of its dollars ends up. The
//! demo's story mode reads this to show the same moment on every screen.

use super::*;
use crate::economy::breakdown;
use serde_json::{json, Value};

impl Night {
    pub fn follow_json(&self, id: usize) -> Value {
        let Some(o) = self.orders.get(id) else { return json!({ "error": "No such order" }) };
        let q = &o.c.quote;
        // the kitchen: what is ahead of it at the same stand
        let ahead = self.orders.iter().filter(|x| x.c.quote.stand == q.stand && x.started_at.is_none() && x.accepted_at < o.accepted_at && x.c.stage == Stage::Accepted).count();
        // the trip: every order the same runner is carrying alongside it right now
        let trip = q.runner.as_ref().map(|r| {
            let mates: Vec<&Order> = self.orders.iter().filter(|x| x.c.quote.runner.as_deref() == Some(r.as_str()) && matches!(x.c.stage, Stage::Assigned | Stage::PickedUp)).collect();
            let mut seats: Vec<String> = mates.iter().map(|x| x.seat.clone()).collect();
            seats.sort(); seats.dedup();
            let stops: Vec<Value> = self.plans.get(r).map(|p| p.iter().map(|(t, at)| json!({ "t": t, "at": at })).collect()).unwrap_or_default();
            json!({ "runner": r, "orders": mates.len(), "sections": seats,
                    "pay": mates.iter().map(|x| x.c.quote.runner_pay.unwrap_or(self.terms.runner_pay)).sum::<i64>(),
                    "tips": mates.iter().map(|x| x.c.quote.tip).sum::<i64>(), "stops": stops, "passes": o.passes })
        });
        let money = breakdown(q, &self.terms).ok().map(|b| {
            let runner_pay = if q.delivery && q.runner.is_some() { q.runner_pay.unwrap_or(self.terms.runner_pay) } else { 0 };
            let sponsors: Vec<Value> = q.discounts.iter().map(|d| json!({ "payer": d.payer, "name": self.party_name(&d.payer), "amount": d.amount })).collect();
            json!({ "charge": b.charge, "subtotal": b.subtotal, "discount": b.discount, "tax": b.tax, "fee": b.fee, "tip": b.tip,
                    "stand": b.stand_net, "runner": runner_pay + b.tip, "runner_pay": runner_pay, "networks": b.interchange,
                    "agape": b.agape_share + b.fee + b.processing - b.interchange - runner_pay - q.points_redeemed * crate::economy::POINT_VALUE_CENTS,
                    "sponsors": sponsors, "share_bps": self.terms.platform_share_bps })
        });
        json!({ "order": self.order_card(id),
                "times": { "placed": o.accepted_at, "started": o.started_at, "ready": o.ready_at, "picked": o.picked_at, "delivered": o.delivered_at, "promised": o.quoted, "eta": o.eta, "prep_s": o.prep_s },
                "stand": { "ahead": ahead }, "trip": trip, "money": money, "now": self.t })
    }
}

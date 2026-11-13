//! A sponsor writes its own deal: at a moment it picks (halftime, a home run, a lead change, last
//! call), fans get money off an item, paid from the sponsor's budget. The deal becomes a rule in
//! the night's engine, the same as the venue's own offers, and its budget is set aside the moment
//! it's written, so a fan who claims it is always covered.

use super::*;
use serde_json::{json, Value};

/// (id, what a sponsor sees, the rule's trigger)
pub const MOMENTS: &[(&str, &str)] = &[("halftime", "At halftime"), ("home_run", "When the home team goes on a run"), ("lead_change", "On a lead change"), ("last_call", "At the start of the 4th")];

#[derive(Clone, Debug)]
pub struct SponsorDeal { pub id: String, pub sponsor: String, pub moment: String, pub thing: String, pub discount: i64, pub cap: i64, pub t: i64 }

impl Night {
    fn moment_pattern(m: &str) -> Option<Value> {
        Some(match m {
            "halftime" => json!({ "enter": { "span": "halftime" } }),
            "last_call" => json!({ "enter": { "span": "last_call" } }),
            "home_run" => json!({ "match": { "sig": "moment", "where": { "kind": { "eq": "run" }, "team": { "eq": "home" } } } }),
            "lead_change" => json!({ "match": { "sig": "moment", "where": { "kind": { "eq": "lead_change" } } } }),
            _ => return None,
        })
    }

    /// A business writes a deal for tonight. Its budget (discount × how many) is set aside now.
    pub fn sponsor_offer(&mut self, who: &str, moment: &str, thing: &str, discount: i64, cap: i64) -> Result<String, String> {
        if self.concert().is_some() && !matches!(moment, "halftime" | "last_call") { return Err("Pick when the deal opens".into()); }
        if !self.world.def.businesses.iter().any(|b| b.id == who) { return Err("Only a business on Agape can write a deal".into()); }
        let when = Self::moment_pattern(moment).ok_or("Pick when the deal opens")?;
        let item = self.world.thing(thing).cloned().ok_or("Pick something from the menu")?;
        if discount < 100 || discount * 2 > item.price { return Err(format!("Take off between $1 and half the price ({})", crate::sim::dollars(item.price / 2))); }
        if !(10..=500).contains(&cap) { return Err("Offer it to between 10 and 500 fans".into()); }
        if self.sponsor_deals.iter().filter(|d| d.sponsor == who).count() >= 6 { return Err("Six deals a night is the most one business can run".into()); }
        let id = format!("deal_{}_{}", who.replace('-', "_"), self.sponsor_deals.len() + 1);
        let label = MOMENTS.iter().find(|m| m.0 == moment).map(|m| m.1).unwrap_or("");
        let rule = json!({ "name": id, "when": when, "then": [{ "offer": { "thing": item.id, "price": item.price, "discount": discount, "payer": who, "expires_ms": 600_000,
            "limits": { "mode": "while_it_lasts", "per_fan": 1, "cap": cap, "label": format!("From {}", self.party_name(who).unwrap_or_default()) } } }] });
        let r: crate::rules::RuleDef = serde_json::from_value(rule).map_err(|e| format!("That deal can't be read: {e}"))?;
        self.engine.add_rule(r, &crate::sim::sports_schema()).map_err(|e| format!("{e:?}"))?;
        crate::commitment::fund(&mut self.ledger, who, discount * cap).map_err(|e| format!("{e:?}"))?;
        let _ = label;
        self.sponsor_deals.push(SponsorDeal { id: id.clone(), sponsor: who.into(), moment: moment.into(), thing: item.id, discount, cap, t: self.t });
        Ok(id)
    }

    /// A sponsor's deals: when each opens, and what's been claimed so far.
    pub fn sponsor_json(&self, who: &str) -> Value {
        let over = phase_of(&self.script, self.t / 60_000).0 == "postgame";
        let deals: Vec<Value> = self.sponsor_deals.iter().filter(|d| d.sponsor == who).map(|d| {
            let live: Vec<_> = self.book.offers.values().filter(|l| l.source == d.id).collect();
            let claimed: i64 = live.iter().map(|l| l.claimed).sum();
            let open = live.iter().any(|l| l.closed.is_none());
            json!({ "id": d.id, "moment": d.moment, "when": if self.concert().is_some() { Some(if d.moment == "halftime" { "At the changeover" } else { "At the finale" }) } else { MOMENTS.iter().find(|m| m.0 == d.moment).map(|m| m.1) }, "thing": d.thing,
                    "name": self.world.thing(&d.thing).map(|t| t.name.clone()), "discount": d.discount, "cap": d.cap, "budget": d.discount * d.cap,
                    "claimed": claimed, "spent": claimed * d.discount, "status": if open { "live" } else if !live.is_empty() { "ended" } else if over { "next" } else { "waiting" } })
        }).collect();
        let menu: Vec<Value> = self.world.def.things.iter().filter(|t| t.price >= 300).map(|t| json!({ "id": t.id, "name": t.name, "price": t.price })).collect();
        let mut seen = BTreeSet::new();
        let menu: Vec<Value> = menu.into_iter().filter(|m| seen.insert(m["name"].as_str().unwrap_or("").to_string())).collect();
        json!({ "deals": deals, "menu": menu, "next_game": over, "moments": if self.concert().is_some() { vec![json!({ "id": "halftime", "label": "At the changeover before the headliner" }), json!({ "id": "last_call", "label": "At the start of the finale" })] }
                           else { MOMENTS.iter().map(|(id, l)| json!({ "id": id, "label": l })).collect::<Vec<_>>() },
                "budget_set_aside": self.ledger.balance(&acct(who, "promo_budget"), &crate::value::Unit::usd()) })
    }
}

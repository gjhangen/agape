//! Views for the parties beyond fans and staff: a player on Agape and a business. Each sees only
//! its own work: what's been asked of it, what it agreed to, what it earned. Deals are agreements
//! on deal offers; everything here reads the exchange book.

use crate::deals::Pay;
use crate::exchange::{Agreement, Exit, Kind, Stage};
use crate::sim::Night;
use crate::value::{acct, Unit};
use serde_json::{json, Value};

/// A deal's status in the words its parties use.
pub fn deal_status(a: &Agreement) -> &'static str {
    match a.stage {
        Stage::Held => "proposed", Stage::Active => "accepted", Stage::Fulfilled => "delivered", Stage::Settled => "done",
        Stage::Closed => if a.exit == Some(Exit::MakerDeclined) { "declined" } else { "withdrawn" },
    }
}

impl Night {
    fn deal_live(a: &Agreement) -> bool { a.stage != Stage::Closed }

    pub fn deal_json(&self, d: &Agreement, viewer: &str) -> Value {
        let mine_to_do = d.maker == viewer;
        let other = if mine_to_do { &d.buyer } else { &d.maker };
        let next = match d.stage {
            Stage::Held if mine_to_do => "respond",
            Stage::Held => "waiting",
            Stage::Active if mine_to_do => "deliver",
            Stage::Active => "in progress",
            Stage::Fulfilled if !mine_to_do => "confirm",
            Stage::Fulfilled => "awaiting confirmation",
            _ => "",
        };
        let sub = self.trade.offer(&d.offer).map(|o| o.sub.clone()).unwrap_or_default();
        let waits_for_claim = d.stage == Stage::Held && self.league.member(&d.maker).is_some() && !self.claimed.contains(&d.maker);
        let auto_at = if d.stage == Stage::Fulfilled { Some(d.changed() + crate::dealing::AUTO_CONFIRM_MS) } else { None };
        json!({ "id": d.id, "kind": sub, "ask": d.detail, "pay": d.pay, "amount": d.amount, "status": deal_status(d), "made": d.made, "proof": d.proof,
                "from": d.buyer, "from_name": self.party_name(&d.buyer), "to": d.maker, "to_name": self.party_name(&d.maker),
                "other": other, "other_name": self.party_name(other), "other_kind": self.node_kind(other), "incoming": mine_to_do, "next": next,
                "waits_for_claim": waits_for_claim, "auto_confirm_at": auto_at, "offer": d.offer, "via": d.via,
                "fee": crate::value::bps(d.amount, crate::dealing::DEAL_FEE_BPS),
                "history": d.history.iter().map(|(t, s, _)| json!({ "t": t, "status": crate::exchange::stage_name(*s) })).collect::<Vec<_>>() })
    }

    pub fn deals_of(&self, party: &str) -> Vec<Value> {
        let mut v: Vec<&Agreement> = self.trade.of_kind(Kind::Deal).filter(|d| d.buyer == party || d.maker == party).collect();
        // what needs you first, then what's in motion, then what's finished
        let rank = |d: &Agreement| {
            let incoming = d.maker == party;
            match (d.stage, incoming) {
                (Stage::Held, true) | (Stage::Active, true) | (Stage::Fulfilled, false) => 0,
                (Stage::Held, _) | (Stage::Active, _) | (Stage::Fulfilled, _) => 1,
                _ => 2,
            }
        };
        v.sort_by_key(|d| (rank(d), -d.changed()));
        v.into_iter().take(40).map(|d| self.deal_json(d, party)).collect()
    }

    /// A party's deal offers as a viewer can use them, each with the gate's answer for that viewer.
    pub fn rates_json(&self, party: &str, viewer: &str) -> Vec<Value> {
        let biz = self.is_business(viewer);
        let mut offers: Vec<&crate::exchange::Offer> = self.trade.offers().filter(|o| o.kind == Kind::Deal && o.maker == party && o.open && (if biz { o.pay == Pay::Money } else { o.pay == Pay::Points })).collect();
        offers.sort_by_key(|o| o.price);
        offers.into_iter().map(|o| {
            let gate = self.gate(&o.id, viewer, 1, None);
            let mine = self.trade.of_kind(Kind::Deal).filter(|d| d.buyer == viewer && d.offer == o.id && Self::deal_live(d)).last().map(|d| self.deal_json(d, viewer));
            json!({ "kind": o.sub, "offer": o.id, "title": o.title, "blurb": o.blurb, "pay": o.pay, "price": o.price, "limit": o.limit,
                    "left": o.limit.map(|l| (l - self.trade.taken(&o.id)).max(0)), "affordable": gate.is_ok() || gate.as_ref().err().map_or(false, |e| e != "CantAfford"),
                    "why_not": gate.err(), "yours": mine })
        }).collect()
    }

    /// A player's own screen once they've claimed their profile.
    pub fn view_player(&self, id: &str) -> Value {
        let Some((g, m)) = self.league.member(id) else { return json!({ "error": "unknown player" }) };
        let pts = crate::social::points();
        let earned_points = self.ledger.balance(&acct(id, "points"), &pts);
        let earned_money = self.ledger.balance(&acct(id, "earnings"), &Unit::usd());
        let mut threads: std::collections::BTreeMap<String, Vec<Value>> = Default::default();
        for (t, from, to, text) in &self.social.messages {
            if to != id && from != id { continue; }
            let other = if from == id { to } else { from };
            threads.entry(other.clone()).or_default().push(json!({ "t": t, "mine": from == id, "text": text }));
        }
        let mut inbox: Vec<Value> = threads.into_iter().map(|(k, v)| {
            let last = v.last().cloned().unwrap_or(Value::Null);
            json!({ "with": k, "name": self.party_name(&k), "kind": self.node_kind(&k), "lines": v, "last_t": last["t"], "needs_reply": last["mine"] == false })
        }).collect();
        inbox.sort_by_key(|x| (if x["needs_reply"] == true { 0 } else { 1 }, -x["last_t"].as_i64().unwrap_or(0)));
        let rates: Vec<Value> = self.trade.offers().filter(|o| o.kind == Kind::Deal && o.maker == id).map(|o| {
            json!({ "kind": o.sub, "title": o.title, "pay": o.pay, "price": o.price, "limit": o.limit, "booked": self.trade.taken(&o.id) })
        }).collect();
        let playing = g.id == self.league.tonight.home || g.id == self.league.tonight.away;
        json!({ "id": id, "name": m.name, "number": m.number, "pos": m.pos, "star": m.star, "group": { "id": g.id, "name": g.name, "color": g.color },
                "claimed": self.claimed.contains(id), "followers": self.social.followers(id),
                "tonight": { "playing": playing, "points": self.points_tonight(id) },
                "earned": { "points": earned_points, "money": earned_money,
                    "in_escrow": self.trade.of_kind(Kind::Deal).filter(|d| d.maker == id && matches!(d.stage, Stage::Active | Stage::Fulfilled)).map(|d| json!({ "pay": d.pay, "amount": d.amount })).collect::<Vec<_>>() },
                "deals": self.deals_of(id), "inbox": inbox.into_iter().take(30).collect::<Vec<_>>(), "rates": rates })
    }

    /// A business's own screen: tonight's money from the ledger, its perks and passes, its deals.
    pub fn view_business(&self, id: &str) -> Value {
        let usd = Unit::usd();
        let revenue = self.ledger.balance(&acct(id, "revenue"), &usd);
        let budget = self.ledger.balance(&acct(id, "deal_budget"), &usd);
        let spent_on_deals: i64 = self.trade.of_kind(Kind::Deal).filter(|d| d.buyer == id && d.stage == Stage::Settled).map(|d| d.amount).sum();
        let in_escrow: i64 = self.trade.of_kind(Kind::Deal).filter(|d| d.buyer == id && d.live()).map(|d| d.amount).sum();
        let places: Vec<&crate::world::AroundPlace> = self.world.def.around.iter().filter(|a| a.operator.as_deref() == Some(id) || a.id == id).collect();
        let passes: Vec<Value> = places.iter().map(|a| {
            let unit = crate::sim::pass_unit(&a.id);
            let supply = a.perk.as_ref().map(|p| p.supply).or(a.service.as_ref().map(|s| s.supply)).unwrap_or(0);
            let left = self.ledger.balance(&acct(&a.id, "passes"), &unit);
            let used = self.ledger.balance(&acct(&a.id, "redeemed"), &unit);
            let recent: Vec<Value> = { let mut r: Vec<_> = self.redeemed.iter().filter(|((_, p), _)| *p == a.id).collect(); r.sort_by_key(|x| -x.1);
                r.into_iter().take(6).map(|((f, _), t)| json!({ "t": t, "who": self.party_name(f) })).collect() };
            json!({ "place": a.id, "name": a.name, "kind": if a.perk.is_some() { "perk" } else { a.service.as_ref().map(|s| s.kind.as_str()).unwrap_or("") },
                    "title": a.perk.as_ref().map(|p| p.title.clone()), "cost": a.perk.as_ref().map(|p| p.cost), "price": a.service.as_ref().map(|s| s.price),
                    "supply": supply, "out": supply - left, "used": used, "waiting": supply - left - used, "recent": recent })
        }).filter(|p| p["kind"] != "ride").collect();
        let rides: Vec<Value> = self.trade.of_kind(Kind::Ride).filter(|r| r.maker == id && r.exit != Some(Exit::BuyerCancelled)).rev().take(8).map(|r| {
            json!({ "id": r.id, "at": r.at, "stage": Self::ride_stage(r), "driver": r.doer.as_ref().and_then(|d| self.party_name(d)), "fare": r.amount, "who": self.party_name(&r.buyer) })
        }).collect();
        let drivers: Vec<Value> = self.world.def.drivers.iter().filter(|d| d.provider == id).map(|d| {
            let on = self.trade.of_kind(Kind::Ride).find(|r| r.doer.as_deref() == Some(&d.id) && r.stage == Stage::Active);
            json!({ "id": d.id, "name": d.name, "status": on.map(Self::ride_stage).unwrap_or("Free"), "earned": self.ledger.balance(&acct(&d.id, "earnings"), &usd) })
        }).collect();
        let sales: Vec<Value> = self.ledger.txns.iter().enumerate().rev().filter(|(_, t)| t.postings.iter().any(|p| p.account.party == id && p.account.purpose == "revenue"))
            .take(10).map(|(i, t)| json!({ "t": self.ledger.times.get(i), "memo": t.memo, "amount": t.postings.iter().filter(|p| p.account.party == id && p.account.purpose == "revenue").map(|p| p.amount).sum::<i64>() })).collect();
        // who they could sponsor: players on Agape and the teams playing tonight
        let mut talent: Vec<Value> = self.claimed.iter().filter_map(|m| self.league.member(m).map(|(g, mm)| (m, g, mm))).map(|(m, g, mm)| json!({ "id": m, "name": mm.name, "team": g.short, "color": g.color, "star": mm.star,
            "followers": self.social.followers(m), "rates": self.rates_json(m, id) })).collect();
        for g in [&self.league.tonight.home, &self.league.tonight.away] {
            if let Some(t) = self.league.team(g) { talent.push(json!({ "id": t.id, "name": t.name, "team": t.short, "color": t.color, "star": false, "followers": self.social.followers(&t.id), "rates": self.rates_json(&t.id, id), "group": true })); }
        }
        let b = self.world.def.businesses.iter().find(|b| b.id == id);
        json!({ "id": id, "name": self.party_name(id), "kind": b.map(|b| b.kind.clone()), "blurb": b.map(|b| b.blurb.clone()),
                "money": { "revenue": revenue, "budget": budget, "spent_on_deals": spent_on_deals, "in_escrow": in_escrow },
                "passes": passes, "rides": rides, "drivers": drivers, "sales": sales, "deals": self.deals_of(id), "talent": talent,
                "followers": self.social.followers(id), "sponsor": if self.on("offers") { self.sponsor_json(id) } else { Value::Null } })
    }

    /// What Agape brought each local business tonight: fans who unlocked its perk or pass, fans who came in and used
    /// it, how many different fans that was, and what it sold through Agape. The record a business is shown when
    /// Agape asks to process its cards: proof of the customers before the ask.
    pub fn brought(&self) -> Value {
        let usd = Unit::usd();
        let mut ids: Vec<(String, String)> = self.world.def.businesses.iter().map(|b| (b.id.clone(), b.name.clone())).collect();
        for a in &self.world.def.around { let op = a.operator.clone().unwrap_or_else(|| a.id.clone()); if !ids.iter().any(|(i, _)| *i == op) { ids.push((op, a.name.clone())); } }
        ids.into_iter().map(|(id, name)| {
            let places: Vec<&crate::world::AroundPlace> = self.world.def.around.iter().filter(|a| a.operator.as_deref() == Some(id.as_str()) || a.id == id).collect();
            let mut unlocked = 0i64; let mut came = 0i64;
            let mut fans: std::collections::BTreeSet<String> = Default::default();
            for a in &places {
                let unit = crate::sim::pass_unit(&a.id);
                let supply = a.perk.as_ref().map(|p| p.supply).or(a.service.as_ref().map(|s| s.supply)).unwrap_or(0);
                unlocked += supply - self.ledger.balance(&acct(&a.id, "passes"), &unit);
                came += self.ledger.balance(&acct(&a.id, "redeemed"), &unit);
                for (f, p) in self.redeemed.keys() { if *p == a.id { fans.insert(f.clone()); } }
                for f in self.fans.iter().map(|f| f.id.clone()) { if self.ledger.balance(&acct(&f, "holdings"), &unit) > 0 { fans.insert(f); } }
            }
            let orders = self.ledger.txns.iter().filter(|t| t.postings.iter().any(|p| p.account.party == id && p.account.purpose == "revenue" && p.amount > 0)).count();
            json!({ "id": id, "name": name, "unlocked": unlocked, "came": came, "fans": fans.len(), "orders": orders,
                    "sales": self.ledger.balance(&acct(&id, "revenue"), &usd), "places": places.iter().map(|a| a.name.clone()).collect::<Vec<_>>() })
        }).collect::<Vec<_>>().into()
    }

    /// Who can be looked at as themselves: businesses, and every player (claimed or not).
    pub fn parties(&self) -> Value {
        let biz: Vec<Value> = self.world.def.businesses.iter().map(|b| json!({ "id": b.id, "name": b.name, "kind": b.kind })).collect();
        let mut players: Vec<Value> = vec![];
        for g in [&self.league.tonight.home, &self.league.tonight.away] {
            if let Some(t) = self.league.team(g) { for m in &t.roster { players.push(json!({ "id": m.id, "name": m.name, "number": m.number, "team": t.short, "claimed": self.claimed.contains(&m.id), "star": m.star })); } }
        }
        json!({ "businesses": biz, "players": players })
    }
}

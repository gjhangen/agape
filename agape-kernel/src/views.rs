//! Layer 4: lenses. Each perspective's screen is a projection of the one world: nothing here
//! stores state, and every number is read from the ledger, the log, the offer book or the
//! commitments. Screens render these and send back actions; they never compute.

use crate::commitment::Stage;
use crate::economy::breakdown;
use crate::sim::{Night, YOU};
use crate::value::{acct, Unit};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn stage_name(s: &Stage) -> String {
    match s {
        Stage::Closed(e) => format!("{e:?}").split([' ', '{']).next().unwrap().to_string(),
        s => format!("{s:?}"),
    }
}

impl Night {
    fn clock(&self) -> Value {
        // period n runs 28 real minutes for a 12-minute game clock
        let m = self.t / 60_000;
        let mut label = match self.phase() { "pregame" => "Pregame".to_string(), "halftime" => "Halftime".to_string(), "postgame" => "Final".to_string(), _ => String::new() };
        let mut period = 0;
        if self.engine.span_open("period") {
            let starts = [45, 76, 122, 153];
            for (i, s) in starts.iter().enumerate() {
                if m >= *s && m < s + 28 {
                    period = i + 1;
                    let elapsed = (self.t - s * 60_000) as f64 / (28.0 * 60_000.0);
                    let left = ((1.0 - elapsed) * 720.0).max(0.0) as i64;
                    label = format!("Q{} {}:{:02}", i + 1, left / 60, left % 60);
                }
            }
            // the period is still open after its clock ran out (the last possessions): show 0:00
            if label.is_empty() { if let Some(i) = starts.iter().rposition(|s| m >= *s) { period = i + 1; label = format!("Q{} 0:00", i + 1); } }
        }
        if label.is_empty() { label = "Break".into(); }
        let spans: Vec<&str> = self.engine.tpl.spans.iter().map(|s| s.name.as_str()).filter(|s| self.engine.span_open(s)).collect();
        json!({ "t": self.t, "label": label, "period": period, "phase": self.phase(), "spans": spans,
                "home": self.score.0, "away": self.score.1, "margin": self.engine.measure("margin", self.t) })
    }

    fn order_card(&self, id: usize) -> Value {
        let o = &self.orders[id];
        let q = &o.c.quote;
        let b = breakdown(q, &self.terms).ok();
        let items: Vec<Value> = q.lines.iter().map(|l| json!({ "thing": l.thing, "name": self.world.thing(&l.thing).map(|t| t.name.clone()), "qty": l.qty, "price": l.unit_price })).collect();
        let stand = self.world.def.stands.iter().find(|s| s.id == q.stand).map(|s| s.name.clone());
        json!({ "id": id, "stage": stage_name(&o.c.stage), "items": items, "stand": stand, "stand_id": q.stand, "delivery": q.delivery,
                "runner": q.runner, "eta": o.eta, "placed": o.accepted_at, "spot": self.spots.get(&q.fan),
                "check_id": q.lines.iter().any(|l| self.world.thing(&l.thing).map_or(false, |t| t.constraints.iter().any(|c| matches!(c, crate::world::Constraint::MinAge(_))))), "delivered": o.delivered_at, "source": o.source, "seat": o.seat,
                "charge": b.as_ref().map(|b| b.charge), "breakdown": b.map(|b| json!({ "subtotal": b.subtotal, "discount": b.discount, "tax": b.tax, "fee": b.fee, "tip": b.tip })),
                "late_credit": o.c.late_by.as_ref().map(|_| crate::commitment::LATE_CREDIT), "scheduled_for": o.scheduled_for,
                "can_cancel": o.c.stage == Stage::Accepted,
                "late": o.c.late_by.is_some(),
                "gift": o.gift.as_ref().map(|(to, note)| json!({ "to": to, "note": note })), "priority": o.priority,
                "rated": self.ratings.iter().filter(|r| r.0 == id).map(|r| json!({ "who": r.1, "stars": r.2 })).collect::<Vec<_>>(),
                "can_report": o.c.stage == Stage::Settled && o.delivered_at.map_or(false, |d| self.t - d <= crate::care::REPORT_WINDOW_MS) && !self.problems.iter().any(|p| p.order == id),
                "problem": self.problems.iter().find(|p| p.order == id).map(|p| json!({ "id": p.id, "reason": p.reason, "words": p.reason.words(), "state": p.state })) })
    }

    fn offer_card(&self, id: &str, fan: &str) -> Option<Value> {
        let l = self.book.offers.get(id)?;
        let d = self.book.disclose(id, fan, &self.ledger)?;
        let th = self.world.thing(&l.thing)?;
        let stand = self.world.def.stands.iter().find(|s| s.id == l.stand).map(|s| s.name.clone());
        let claim = self.book.claims.values().find(|c| c.offer == id && c.fan == fan).map(|c| json!({ "id": c.id, "until": c.until }));
        Some(json!({ "id": id, "source": l.source, "thing": l.thing, "name": th.name, "stand": stand, "price": l.price,
            "discount": d.discount_now, "payer": l.payer, "guaranteed": d.guaranteed, "held_for_you": d.held_for_you,
            "ends_at": d.ends_at, "left": d.left, "condition": d.condition, "ended": d.ended.map(|e| format!("{e:?}")), "claim": claim,
            "opened_at": d.opened_at, "left_at_open": d.left_at_open }))
    }

    fn runner_at(&self, r: &str) -> Value {
        let at = self.runners.iter().find(|x| x.id == r).map(|x| x.at.clone()).unwrap_or_default();
        match self.plans.get(r) {
            Some(p) if p.last().map_or(false, |w| w.0 > self.t) => {
                let i = p.iter().position(|w| w.0 > self.t).unwrap();
                let (a, b) = (&p[i.saturating_sub(1)], &p[i]);
                let prog = if b.0 > a.0 { (self.t - a.0) as f64 / (b.0 - a.0) as f64 } else { 1.0 };
                json!({ "id": r, "from": a.1, "to": b.1, "progress": prog.clamp(0.0, 1.0), "busy": true })
            }
            _ => json!({ "id": r, "from": at, "to": at, "progress": 1.0, "busy": false }),
        }
    }

    /// The fan's world, measured from them: you (here, now), then places to zoom out to
    /// (section, concourse, venue) and time either side (earlier, later). Everything a fan sees
    /// is about them or something they can act on; nothing appears in two places.
    /// The colors the fan's app wears: the teams playing tonight and which one they chose.
    pub fn colors_json(&self, fan: &str) -> Value {
        let tonight = [&self.league.tonight.home, &self.league.tonight.away];
        let teams: Vec<Value> = tonight.iter().filter_map(|id| self.league.team(id))
            .map(|g| json!({ "id": g.id, "name": g.name, "short": g.short, "color": g.color, "home": g.id == self.league.tonight.home })).collect();
        let wearing = self.wearing.get(fan).cloned().or_else(|| teams.first().and_then(|t| t["id"].as_str().map(String::from)));
        json!({ "teams": teams, "wearing": wearing })
    }

    pub fn view_fan(&mut self, fan: &str) -> Value {
        let me = self.fans.iter().find(|f| f.id == fan).cloned();
        let seat = me.as_ref().map(|f| f.seat.clone()).unwrap_or_default();
        let t = self.t;
        // ---- what you could get, with the honest time it reaches your seat ----
        let mut items = vec![];
        for th in self.world.def.things.clone() {
            if th.id == "combo" { continue; }
            let elig = me.as_ref().map(|f| self.world.eligible(f, &th.id, &|s| self.engine.span_open(s), false));
            let stock = self.ledger.balance(&acct(&th.stand, "inventory"), &Unit::Thing(th.id.clone()));
            let eta = self.estimate(&th.stand, th.prep_s, &seat, true);
            let stand = self.world.def.stands.iter().find(|s| s.id == th.stand).unwrap();
            let adult = th.constraints.iter().any(|c| matches!(c, crate::world::Constraint::MinAge(_)));
            let closes = self.script_final();
            items.push(json!({ "id": th.id, "name": th.name, "price": th.price, "category": th.category, "adult": adult, "closes": closes, "stand": stand.name, "stand_id": th.stand, "level": stand.level,
                "available": elig.as_ref().map_or(false, |e| e.is_ok()) && stock > 0 && self.closed_now(&th.id).is_none(),
                "why_not": self.closed_now(&th.id).map(String::from).or(elig.and_then(|e| e.err()).map(|r| format!("{r:?}"))).or(if stock <= 0 { Some("SoldOut".into()) } else { None }),
                "left": if stock < 1000 { Some(stock) } else { None }, "eta": eta }));
        }
        // fastest to you right now: the quickest available food and drink, one per stand
        let mut fastest: Vec<&Value> = items.iter().filter(|i| i["available"] == true && i["category"] != "merch").collect();
        fastest.sort_by_key(|i| i["eta"].as_i64().unwrap_or(i64::MAX));
        let mut seen = std::collections::BTreeSet::new();
        let fastest: Vec<Value> = fastest.into_iter().filter(|i| seen.insert(i["stand_id"].as_str().unwrap_or("").to_string())).take(4).cloned().collect();
        // ---- now: your orders, your claims, offers you can act on ----
        let offers: Vec<Value> = self.book.visible(fan).iter().filter_map(|l| {
            let c = self.offer_card(&l.id, fan)?;
            let claimed = !c["claim"].is_null();
            let ended = c["ended"].as_str();
            // paused offers are not shown to fans; an ended one stays 30 s so it is seen ending, not vanishing
            if !claimed && ended == Some("Paused") { return None; }
            // resource ends (all claimed, sold out) are approached on the card's own bar, then the card goes
            if !claimed && ended.is_some() && l.closed_at.map_or(true, |c| t - c > 30_000) { return None; }
            Some(c)
        }).take(4).collect();
        let mine: Vec<usize> = (0..self.orders.len()).filter(|&i| self.orders[i].c.quote.fan == fan).collect();
        let active: Vec<Value> = mine.iter().filter(|&&i| !matches!(self.orders[i].c.stage, Stage::Settled | Stage::Closed(_))).map(|&i| self.order_card(i)).collect();
        let runner = active.iter().find_map(|o| o["runner"].as_str().map(String::from)).map(|r| self.runner_at(&r));
        // ---- section: your neighbours ----
        let mut heat: BTreeMap<String, i64> = BTreeMap::new();
        for (_, sec, _) in &self.recent { *heat.entry(sec.clone()).or_insert(0) += 1; }
        let mut rank: Vec<(String, i64)> = heat.iter().map(|(k, v)| (k.clone(), *v)).collect();
        rank.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let pos = rank.iter().position(|(s, _)| *s == seat);
        let mine_n = *heat.get(&seat).unwrap_or(&0);
        let ahead = pos.and_then(|p| if p > 0 { rank.get(p - 1).cloned() } else { None }).or_else(|| if pos.is_none() { rank.last().cloned() } else { None });
        // the whole arena talks; each line carries its section so the fan can narrow to their own
        let who = |id: &str| if id == fan { "You".to_string() } else { self.party_name(id).unwrap_or_else(|| crate::social::handle(id)) };
        let chat_all: Vec<Value> = self.chat.iter().rev().take(40)
            .map(|c| json!({ "t": c.0, "you": c.1 == fan, "text": c.3, "who": who(&c.1), "id": c.1, "section": c.2, "mine": c.2 == seat })).collect();
        let chat_section: Vec<Value> = self.chat.iter().rev().filter(|c| c.2 == seat).take(40)
            .map(|c| json!({ "t": c.0, "you": c.1 == fan, "text": c.3, "who": who(&c.1), "id": c.1, "section": c.2, "mine": true })).collect();
        let prediction = self.predictions.last().map(|p| {
            let home = p.picks.values().filter(|v| *v == "home").count();
            let n = p.picks.len().max(1);
            let yours = p.picks.get(fan).cloned();
            let shares: Vec<Value> = p.options.iter().map(|(id, _, _)| json!({ "id": id, "share": p.picks.values().filter(|v| *v == id).count() as f64 / n as f64 })).collect();
            let labels: serde_json::Map<String, Value> = p.options.iter().map(|(id, _, _)| (id.clone(), json!(self.renamed(id)))).collect();
            json!({ "question": self.renamed(&p.question), "labels": labels, "options": shares, "locks": p.locks, "open": p.result.is_none() && t < p.locks, "result": p.result,
                    "home_share": home as f64 / n as f64, "picks": p.picks.len(), "yours": yours,
                    "won": p.result.is_some() && yours.is_some() && p.result == yours, "points": p.points })
        });
        let neighbours = self.fans.iter().filter(|f| f.seat == seat).count();
        // ---- venue: the event itself and the store ----
        let event: Vec<Value> = self.feed.iter().rev().filter(|f| matches!(f.kind.as_str(), "moment" | "score" | "game")).take(40)
            .map(|f| self.feed_json(f, fan)).collect();
        // ---- earlier: your night so far ----
        let receipts: Vec<Value> = mine.iter().filter(|&&i| matches!(self.orders[i].c.stage, Stage::Settled | Stage::Closed(_))).rev().map(|&i| self.order_card(i)).collect();
        let spent = -self.ledger.balance(&acct(fan, "card"), &Unit::usd());
        let holdings: Vec<Value> = self.ledger.balances_iter().filter(|((a, u), v)| a.party == fan && a.purpose == "holdings" && matches!(u, Unit::Thing(t) if !t.starts_with("card:") && !t.starts_with("ticket:") && !t.starts_with("pass:")) && **v > 0)
            .map(|((_, u), v)| { let id = match u { Unit::Thing(t) => t.clone(), _ => String::new() }; json!({ "thing": id, "name": self.world.thing(&id).map(|t| t.name.clone()), "qty": v }) }).collect();
        let later: Vec<Value> = self.later().into_iter().map(|(t, k)| json!({ "t": t, "kind": k })).collect();
        let stands: Vec<Value> = self.world.def.stands.iter().map(|s| {
            let eta = items.iter().filter(|i| i["stand_id"] == s.id.as_str()).filter_map(|i| i["eta"].as_i64()).min();
            json!({ "id": s.id, "name": s.name, "level": s.level, "eta": eta, "slow": self.book.busy.contains(&s.id) })
        }).collect();
        json!({
            "clock": self.clock(),
            "halt": self.halt,
            "provenance": self.provenance(),
            "you": { "id": fan, "seat": seat, "neighbours": neighbours, "spot": self.spots.get(fan) },
            "colors": self.colors_json(fan),
            "orders": active, "runner": runner,
            "upgrades": self.trade.offers().filter(|o| o.kind == crate::exchange::Kind::Upgrade && o.open).map(|o| {
                let g = self.gate(&o.id, fan, 1, None);
                json!({ "id": o.id, "section": o.subject, "title": o.title, "blurb": o.blurb, "price": o.price, "left": o.limit.map(|l| l - self.trade.taken(&o.id)), "ok": g.is_ok(), "why_not": g.err() })
            }).filter(|u| u["why_not"] != "The game's over" && u["why_not"] != "You've already moved seats tonight" && u["why_not"] != "You're already in one of the best sections" && u["why_not"] != "You're already there").collect::<Vec<_>>(),
            "moved": self.trade.of_kind(crate::exchange::Kind::Upgrade).find(|a| a.buyer == fan).map(|a| json!({ "section": self.trade.offer(&a.offer).map(|o| o.subject.clone()), "at": a.made })),
            "get": { "offers": offers, "fastest": fastest, "stands": stands, "later": later, "market": self.market_json(fan, 12), "events": self.events_json(fan), "around": self.around_json(fan) },
            "items": items,
            "live": { "event": event, "league": self.league_tonight(), "chat": chat_all, "chat_section": chat_section, "prediction": prediction,
                      "battle": { "orders_5m": mine_n, "rank": pos.map(|p| p + 1), "of": rank.len(), "ahead": ahead.map(|(s, n)| json!({ "section": s, "orders": n })) } },
            "me": self.me_json(fan, &active, receipts, spent, holdings),
            "map": self.view_map(Some(&seat), self.orders.iter().rev().find(|o| o.c.quote.fan == fan && matches!(o.c.stage, Stage::Assigned | Stage::PickedUp))
                    .and_then(|o| o.c.quote.runner.clone()).map(|r| self.runner_at(&r))),
        })
    }

    fn view_map(&self, seat: Option<&str>, runner: Option<Value>) -> Value {
        let mut heat: BTreeMap<String, i64> = BTreeMap::new();
        for (_, sec, _) in &self.recent { *heat.entry(sec.clone()).or_insert(0) += 1; }
        let stands: Vec<Value> = self.world.def.stands.iter().map(|s| json!({ "id": s.id, "name": s.name, "level": s.level, "ring": s.ring,
            "wait": self.stand_wait(&s.id), "busy": self.book.busy.contains(&s.id) })).collect();
        let runners: Vec<Value> = self.runners.iter().map(|r| self.runner_at(&r.id)).collect();
        json!({ "levels": self.world.def.levels, "sections_per_level": self.world.def.sections_per_level, "heat": heat,
                "stands": stands, "runners": runners, "you": seat, "your_runner": runner })
    }

    pub fn view_runner(&self, r: &str) -> Value {
        let bag: Vec<Value> = (0..self.orders.len()).filter(|&i| self.orders[i].c.quote.runner.as_deref() == Some(r)
            && matches!(self.orders[i].c.stage, Stage::Assigned | Stage::PickedUp)).map(|i| self.order_card(i)).collect();
        let plan: Vec<Value> = self.plans.get(r).map(|p| p.iter().map(|(t, pl)| json!({ "t": t, "place": pl, "done": *t <= self.t })).collect()).unwrap_or_default();
        let delivered = self.orders.iter().filter(|o| o.c.quote.runner.as_deref() == Some(r) && o.delivered_at.is_some()).count();
        // a trip offered to this runner, or the one they're on, when a person runs this runner
        let trip = self.trips.get(r).map(|x| {
            let tips: i64 = x.orders.iter().map(|&i| self.orders[i].c.quote.tip).sum();
            let stand = self.world.def.stands.iter().find(|s| s.id == x.stand).map(|s| s.name.clone());
            json!({ "accepted": x.accepted, "picked": x.picked, "stand": x.stand, "stand_name": stand, "expires": x.offered_at + crate::sim::TRIP_OFFER_MS,
                    "pay": x.pay + tips, "base": x.pay, "tips": tips, "raised": x.pay - x.first, "passes": x.passes, "mode": self.terms.pay_mode, "minutes": (x.done_s as i64 + 59) / 60, "sections": x.orders.iter().map(|&i| self.orders[i].seat.clone()).collect::<Vec<_>>(),
                    "orders": x.orders.iter().map(|&i| { let mut c = self.order_card(i); c["handoff_ok_at"] = json!(self.orders[i].picked_at.map(|p| p + 3 * 60_000)); c }).collect::<Vec<_>>() })
        });
        json!({ "clock": self.clock(), "runner": self.runner_at(r), "bag": bag, "plan": plan, "delivered": delivered, "map": self.view_map(None, None),
                "human": self.human(r), "on_shift": self.live.contains(r), "trip": trip,
                "pay": self.ledger.balance(&acct(r, "pay"), &Unit::usd()), "tips": self.ledger.balance(&acct(r, "tips"), &Unit::usd()),
                "rating": self.rating_of(r).0, "ratings": self.rating_of(r).1 })
    }

    pub fn view_stand(&self, stand: &str) -> Value {
        let stations: Vec<Value> = self.stations.get(stand).map(|v| v.iter().map(|(until, id)| json!({ "until": until, "late": id.is_some() && *until < self.t, "order": id.map(|i| self.order_card(i)) })).collect()).unwrap_or_default();
        let pickups: Vec<Value> = (0..self.orders.len()).filter(|&i| self.orders[i].c.quote.stand == stand && !self.orders[i].c.quote.delivery && self.orders[i].c.stage == Stage::Ready).map(|i| self.order_card(i)).collect();
        let queue: Vec<Value> = self.queues.get(stand).map(|q| q.iter().take(12).map(|&i| self.order_card(i)).collect()).unwrap_or_default();
        let ready: Vec<Value> = self.ready.iter().filter(|r| r.stand == stand).map(|r| self.order_card(r.order.parse().unwrap())).collect();
        let sold: i64 = self.orders.iter().filter(|o| o.c.quote.stand == stand && o.c.stage == Stage::Settled).map(|o| o.c.quote.lines.iter().map(|l| l.qty).sum::<i64>()).sum();
        let name = self.world.def.stands.iter().find(|s| s.id == stand).map(|s| s.name.clone());
        json!({ "clock": self.clock(), "stand": stand, "name": name, "wait": self.stand_wait(stand), "paused_offers": self.book.busy.contains(stand),
                "stations": stations, "pickups": pickups, "rating": self.rating_of(stand).0, "ratings": self.rating_of(stand).1, "human": self.human(stand), "on_shift": self.live.contains(stand), "queue_len": self.queues.get(stand).map_or(0, |q| q.len()), "queue": queue, "ready": ready, "sold": sold,
                "revenue": self.ledger.balance(&acct(stand, "revenue"), &Unit::usd()) })
    }

    /// Every exchange beyond food, by kind: how many, where they stand, what they moved, and every
    /// exit with whose doing it was.
    pub fn exchanges_json(&self) -> Value {
        use crate::exchange::{Kind, Stage};
        let kinds = [(Kind::Ticket, "Tickets"), (Kind::Ride, "Rides"), (Kind::Parking, "Parking"), (Kind::Perk, "Perks"), (Kind::Deal, "Deals"), (Kind::Card, "Card trades"), (Kind::Drop, "Cards collected"), (Kind::Upgrade, "Seat upgrades"), (Kind::Resale, "Tickets resold")];
        let rows: Vec<Value> = kinds.iter().map(|(k, name)| {
            let all: Vec<&crate::exchange::Agreement> = self.trade.of_kind(*k).collect();
            let count = |st: Stage| all.iter().filter(|a| a.stage == st).count();
            let money: i64 = all.iter().filter(|a| a.stage == Stage::Settled && a.pay == crate::deals::Pay::Money).map(|a| a.amount).sum();
            let pts: i64 = all.iter().filter(|a| a.stage == Stage::Settled && a.pay == crate::deals::Pay::Points).map(|a| a.amount).sum();
            json!({ "kind": k, "name": name, "total": all.len(), "live": all.iter().filter(|a| a.live()).count(), "settled": count(Stage::Settled), "closed": count(Stage::Closed), "money": money, "points": pts })
        }).collect();
        let mut exits: BTreeMap<(String, String, String), i64> = BTreeMap::new();
        for a in self.trade.all().iter().filter(|a| a.stage == Stage::Closed) {
            let e = a.exit.unwrap();
            let owner = e.fault(a).map(|p| if self.is_business(p) || self.league.team(p).is_some() || self.league.member(p).is_some() { self.party_name(p).unwrap_or_default() } else { "the fan".into() }).unwrap_or_else(|| "nobody".into());
            *exits.entry((format!("{:?}", a.kind), e.name().into(), owner)).or_default() += 1;
        }
        json!({ "kinds": rows, "exits": exits.into_iter().map(|((k, e, o), n)| json!({ "kind": k, "exit": e, "owner": o, "count": n })).collect::<Vec<_>>(),
                "replays": self.replays() })
    }

    /// Whether the exchange book rebuilds exactly from the log, rechecked as the log grows.
    pub fn replays(&self) -> bool {
        let (at, ok) = self.replay_check.get();
        if self.log.signals.len() < at + 2_000 && at > 0 { return ok; }
        let ok = crate::exchange::Book::fold(&self.log.signals).same_as(&self.trade);
        self.replay_check.set((self.log.signals.len(), ok));
        ok
    }

    /// Where the numbers on a screen come from (invariant 7): the crowd is simulated in the pilot,
    /// arrival times are estimates, money is the ledger's.
    pub fn provenance(&self) -> Value {
        json!({ "crowd": if self.cfg.fans > 0 { "simulated" } else { "live" }, "simulated_fans": self.cfg.fans, "people": self.live.len(),
                "eta": "estimate", "money": "ledger", "wait": "measured" })
    }

    fn problems_json(&self) -> Vec<Value> {
        let mut v: Vec<&crate::care::Problem> = self.problems.iter().collect(); v.sort_by_key(|p| (p.state != crate::care::ProblemState::Open, -p.t));
                    v.into_iter().take(20).map(|p| json!({ "id": p.id, "order": p.order, "who": self.party_name(&p.fan), "stand": self.party_name(&p.stand), "reason": p.reason, "words": p.reason.words(),
                        "note": p.note, "t": p.t, "fault": if p.fault == "agape" { "Agape (delivery)".to_string() } else { self.party_name(&p.fault).unwrap_or_default() }, "state": p.state,
                        "amount": breakdown(&self.orders[p.order].c.quote, &self.terms).map(|b| b.charge).ok() })).collect::<Vec<_>>()
    }

    pub fn view_ops(&self) -> Value {
        let mut stages: BTreeMap<String, i64> = BTreeMap::new();
        for o in &self.orders { *stages.entry(stage_name(&o.c.stage)).or_insert(0) += 1; }
        let mut waits: Vec<i64> = self.orders.iter().filter(|o| o.c.quote.delivery).filter_map(|o| o.delivered_at.map(|d| (d - o.accepted_at) / 1000)).collect();
        waits.sort();
        let pct = |p: usize| waits.get(waits.len() * p / 100).copied();
        let busy = self.runners.iter().filter(|r| r.free_at > self.t).count();
        let offers: Vec<Value> = self.book.offers.values().rev().take(8).filter_map(|l| self.offer_card(&l.id, "")).collect();
        let usd = Unit::usd();
        let ledger = json!({
            "agape": self.ledger.balance(&acct("agape", "revenue"), &usd),
            "stands": self.world.def.stands.iter().map(|s| self.ledger.balance(&acct(&s.id, "revenue"), &usd)).sum::<i64>(),
            "runners": self.runners.iter().map(|r| self.ledger.balance(&acct(&r.id, "pay"), &usd) + self.ledger.balance(&acct(&r.id, "tips"), &usd)).sum::<i64>(),
            "tax": self.ledger.balance(&acct("tax_authority", "payable"), &usd),
            "processing": self.ledger.balance(&acct("processor", "fees"), &usd),
            "agape_processing": self.ledger.balance(&acct("agape", "processing"), &usd),
            "flows": crate::sim::money_flows(&self.ledger, &self.world.def.stands.iter().map(|s| s.id.clone()).collect::<Vec<_>>(), &self.runners.iter().map(|r| r.id.clone()).collect::<Vec<_>>()),
            "sponsors_paid": self.ledger.txns.iter().filter(|t| t.memo == "settle").flat_map(|t| t.postings.iter()).filter(|p| p.account.purpose == "promo_escrow" && p.amount < 0).map(|p| -p.amount).sum::<i64>(),
            "trip_offers": self.pay_stats.0, "trips_taken": self.pay_stats.1, "taken_first": self.pay_stats.2,
            "fans_paid": -self.ledger.balances_iter().filter(|((a, u), _)| a.purpose == "card" && *u == usd).map(|(_, v)| *v).sum::<i64>(),
            "conserved": self.ledger.units().iter().all(|u| self.ledger.total(u) == 0),
            "txns": self.ledger.txns.len(),
        });
        let attribution: Vec<Value> = self.attr.by_source.iter().map(|(k, s)| json!({ "source": k, "shown": s.shown, "accepted": s.accepted, "settled": s.settled, "gross": s.gross, "spend": s.spend })).collect();
        json!({ "clock": self.clock(), "orders_5m": self.recent.len(), "stages": stages, "wait_p50": pct(50), "wait_p90": pct(90),
                "runners": { "total": self.runners.len(), "busy": busy }, "map": self.view_map(None, None), "offers": offers,
                "alerts": self.feed.iter().rev().filter(|f| f.kind == "alert").take(5).map(|f| json!({ "t": f.t, "text": f.text })).collect::<Vec<_>>(),
                "ledger": ledger, "attribution": attribution, "signals": self.log.signals.len(), "exchanges": self.exchanges_json(),
                "halt": self.halt, "provenance": self.provenance(),
                "problems": self.problems_json() })
    }
}

impl Night {
    fn feed_json(&self, f: &crate::sim::FeedItem, viewer: &str) -> Value {
        let drop = f.edition.as_ref().and_then(|e| self.edition_json(e, viewer).into());
        json!({ "t": f.t, "kind": f.kind, "team": f.team, "text": f.text, "party": f.party, "group": self.group_of(&f.team),
                "name": self.party_name(&f.party), "drop": drop })
    }

    fn resale_json(&self, event: &str, viewer: &str) -> Vec<Value> {
        let mut l: Vec<&crate::exchange::Offer> = self.trade.offers().filter(|o| o.kind == crate::exchange::Kind::Resale && o.open && o.subject.starts_with(&format!("{event}:"))).collect();
        l.sort_by_key(|o| o.price);
        l.into_iter().take(8).map(|o| json!({ "id": o.id, "price": o.price, "section": o.subject.split(':').nth(1), "seats": o.sub, "mine": o.maker == viewer, "seller": self.party_name(&o.maker) })).collect()
    }

    /// What anyone may see before signing in: the venue, the game right now, and how the night is going.
    pub fn view_glance(&self) -> Value {
        let teams: Vec<Value> = [&self.league.tonight.home, &self.league.tonight.away].iter().filter_map(|id| self.league.team(id))
            .map(|g| json!({ "name": g.name, "short": g.short, "abbr": g.abbr, "color": g.color })).collect();
        let moments: Vec<Value> = self.feed.iter().rev().filter(|f| matches!(f.kind.as_str(), "moment" | "score" | "game")).take(4)
            .map(|f| json!({ "t": f.t, "text": f.text, "name": self.party_name(&f.party), "kind": f.kind, "team": f.team })).collect();
        let delivered: Vec<i64> = self.orders.iter().filter(|o| matches!(o.c.stage, Stage::Settled) && o.c.quote.delivery)
            .filter_map(|o| o.delivered_at.map(|d| d - o.accepted_at)).collect();
        let avg_min = if delivered.is_empty() { None } else { Some((delivered.iter().sum::<i64>() as f64 / delivered.len() as f64 / 60_000.0).round() as i64) };
        json!({ "venue": self.venue_name(), "clock": self.clock(), "teams": teams, "moments": moments,
                "orders": self.orders.len(), "on_the_way": self.orders.iter().filter(|o| matches!(o.c.stage, Stage::Assigned | Stage::PickedUp)).count(),
                "avg_delivery_min": avg_min, "provenance": self.provenance() })
    }

    pub fn venue_name(&self) -> String { if self.world.def.name.is_empty() { "the venue".into() } else { self.world.def.name.clone() } }

    /// Display name for any node id: a member, a group, a fan.
    pub fn party_name(&self, id: &str) -> Option<String> {
        if id.is_empty() { return None; }
        if let Some((_, m)) = self.league.member(id) { return Some(m.name.clone()); }
        if let Some(g) = self.league.team(id) { return Some(g.name.clone()); }
        if id == crate::sim::YOU { return Some("You".into()); }
        if let Some(n) = self.names.get(id) { return Some(n.clone()); }
        if id == crate::sim::VENUE { return Some(self.venue_name()); }
        if let Some(b) = self.business_name(id) { return Some(b); }
        if let Some(d) = self.world.def.drivers.iter().find(|d| d.id == id) { return Some(d.name.clone()); }
        if let Some(e) = self.league.upcoming.iter().find(|e| e.id == id) { return Some(e.title.clone()); }
        if id.starts_with("fan") || self.fans.iter().any(|f| f.id == id) { return Some(crate::social::handle(id)); }
        None
    }

    fn edition_json(&self, id: &str, viewer: &str) -> Value {
        let Some(e) = self.social.editions.get(id) else { return Value::Null };
        let unit = crate::social::card_unit(id);
        let left = self.social.left(&self.ledger, id);
        let floor = self.listings().filter(|l| l.subject == id).map(|l| l.price).min();
        let holders = self.ledger.balances_iter().filter(|((a, u), v)| *u == unit && (a.purpose == "holdings" || a.purpose == "listed") && **v > 0).count();
        json!({ "id": e.id, "kind": e.kind, "title": e.title, "rarity": e.rarity, "team": e.team, "party": e.party, "party_name": e.party.as_ref().and_then(|p| self.party_name(p)),
                "t": e.t, "supply": e.supply, "left": left, "open": e.open_until >= self.t && left > 0, "open_until": e.open_until,
                "yours": self.ledger.balance(&acct(viewer, "holdings"), &unit) + self.ledger.balance(&acct(viewer, "listed"), &unit),
                "collected": self.trade.all().iter().any(|a| a.buyer == viewer && a.offer == format!("drop:{id}")), "floor": floor, "holders": holders,
                "color": self.league.team(&e.team).map(|g| g.color.clone()),
                "mark": e.party.as_ref().and_then(|p| self.league.member(p)).map(|(_, m)| format!("#{}", m.number))
                    .or_else(|| self.league.team(&e.team).map(|g| g.abbr.clone())) })
    }

    /// Cards fans have up for points: open card offers.
    pub fn listings(&self) -> impl Iterator<Item = &crate::exchange::Offer> { self.trade.offers().filter(|o| o.kind == crate::exchange::Kind::Card && o.open) }

    fn market_json(&self, viewer: &str, n: usize) -> Value {
        // the cheapest listing of each card, rarest first
        let mut best: BTreeMap<String, &crate::exchange::Offer> = BTreeMap::new();
        for l in self.listings() {
            if l.maker == viewer { continue; }
            let e = best.entry(l.subject.clone()).or_insert(l);
            if l.price < e.price { *e = l; }
        }
        let rank = |r: &str| match r { "Legendary" => 0, "Rare" => 1, "Uncommon" => 2, _ => 3 };
        let mut v: Vec<&crate::exchange::Offer> = best.into_values().collect();
        v.sort_by_key(|l| (rank(&self.social.editions[&l.subject].rarity), l.price));
        Value::Array(v.into_iter().take(n).map(|l| json!({ "listing": l.id, "price": l.price, "seller": l.maker, "seller_name": self.party_name(&l.maker),
            "card": self.edition_json(&l.subject, viewer) })).collect())
    }

    /// Other games in the league tonight. Their scores advance with the clock (simulated).
    pub fn league_tonight(&self) -> Value {
        let m = self.t / 60_000;
        let mut v = vec![];
        for (i, (h, a)) in self.league.tonight.others.iter().enumerate() {
            let (mut hs, mut as_) = (0i64, 0i64);
            let played = (m - 45).clamp(0, 160);
            for k in 0..played {
                if (76..122).contains(&(k + 45)) { continue; }
                let x = (k as u64 + 1).wrapping_mul(2654435761).wrapping_add(i as u64 * 97) % 100;
                if x < 23 { hs += [2, 2, 3, 1][(x % 4) as usize]; }
                if (x + 37) % 100 < 22 { as_ += [2, 3, 2, 1][((x + 1) % 4) as usize]; }
            }
            let status = if m < 45 { "7:30 PM".to_string() } else if m >= 181 { "Final".into() } else if (76..122).contains(&m) { "Halftime".into() } else { "Live".into() };
            v.push(json!({ "id": format!("g{i}"), "home": h, "away": a, "home_name": self.league.team(h).map(|g| g.short.clone()), "away_name": self.league.team(a).map(|g| g.short.clone()),
                           "home_score": hs, "away_score": as_, "status": status }));
        }
        Value::Array(v)
    }

    pub(crate) fn points_tonight(&self, member: &str) -> i64 {
        self.log.signals.iter().filter(|s| s.kind == "score" && s.authority == crate::signal::Authority::Fact && s.fields.get("player").map(|p| p.key()) == Some(member.into()))
            .filter_map(|s| s.fields.get("points").and_then(|p| p.as_i_opt())).sum()
    }

    fn badges(&self, fan: &str) -> Vec<Value> {
        let unit_cards = self.social.editions.keys().filter(|e| self.ledger.balance(&acct(fan, "holdings"), &crate::social::card_unit(e)) + self.ledger.balance(&acct(fan, "listed"), &crate::social::card_unit(e)) > 0).count();
        let ordered = self.orders.iter().any(|o| o.c.quote.fan == fan && o.c.stage == Stage::Settled);
        let called = self.predictions.iter().any(|p| p.result.is_some() && p.picks.get(fan) == p.result.as_ref());
        let traded = self.trade.of_kind(crate::exchange::Kind::Card).any(|a| a.stage == crate::exchange::Stage::Settled && (a.buyer == fan || a.maker == fan));
        let tips: i64 = self.orders.iter().filter(|o| o.c.quote.fan == fan && o.c.stage == Stage::Settled).map(|o| o.c.quote.tip).sum();
        let all = [("first_order", "First order", "Had something brought to your seat", ordered),
                   ("called_it", "Called it", "Picked a quarter right", called),
                   ("collector", "Collector", "Hold three different cards", unit_cards >= 3),
                   ("trader", "Trader", "Bought or sold a card", traded),
                   ("good_tipper", "Good tipper", "Tipped your runners $5 in a night", tips >= 500)];
        all.iter().map(|(id, name, how, got)| json!({ "id": id, "name": name, "how": how, "earned": got })).collect()
    }

    fn earn_history(&self, fan: &str) -> Vec<Value> {
        let mine = acct(fan, "points");
        let unit = crate::social::points();
        let mut v = vec![];
        for (txn, t) in self.ledger.txns.iter().zip(&self.ledger.times) {
            let d: i64 = txn.postings.iter().filter(|p| p.account == mine && p.unit == unit).map(|p| p.amount).sum();
            if d == 0 { continue; }
            let why = match txn.memo.split(' ').next().unwrap_or("") {
                "settle" | "settle+late_credit" => "Order delivered".to_string(), "welcome" => "Welcome to Agape".into(), "history" => "Before tonight".into(),
                "prediction" => format!("Called it: {}", txn.memo.splitn(2, ' ').nth(1).unwrap_or("")),
                "sale" => if d > 0 { "Sold a card".into() } else { "Bought a card".into() },
                "tickets" => "Bought tickets".into(),
                "unlock" => "Unlocked a partner perk".into(),
                m => m.to_string(),
            };
            v.push(json!({ "t": t, "points": d, "why": why }));
        }
        v.reverse();
        v.truncate(30);
        v
    }

    fn cards_of(&self, fan: &str) -> Vec<Value> {
        self.social.editions.keys().filter_map(|e| {
            let u = crate::social::card_unit(e);
            let n = self.ledger.balance(&acct(fan, "holdings"), &u) + self.ledger.balance(&acct(fan, "listed"), &u);
            if n == 0 { return None; }
            let listing = self.listings().find(|l| l.maker == fan && l.subject == *e).map(|l| json!({ "id": l.id, "price": l.price }));
            Some(json!({ "card": self.edition_json(e, fan), "count": n, "listing": listing }))
        }).collect()
    }

    fn events_json(&self, fan: &str) -> Value {
        Value::Array(self.league.upcoming.iter().map(|e| {
            let left: i64 = self.seatmaps.iter().filter(|((ev, _), _)| *ev == e.id).map(|(_, m)| m.free() as i64).sum();
            let from = self.ticket_sections().iter().filter(|sec| self.seatmaps.get(&(e.id.clone(), (*sec).clone())).map_or(false, |m| m.free() > 0))
                .map(|sec| self.section_price(sec)).min();
            let yours: usize = self.seats_of(fan, &e.id).iter().map(|(_, v)| v.len()).sum();
            json!({ "id": e.id, "kind": e.kind, "title": e.title, "date": e.date, "time": e.time, "home": e.home, "away": e.away, "left": left, "from": from, "yours": yours,
                    "away_name": e.away.as_ref().and_then(|a| self.league.team(a)).map(|g| g.name.clone()) })
        }).collect())
    }

    fn around_json(&self, fan: &str) -> Value {
        let final_t = self.later().into_iter().find(|l| l.1 == "final").map(|l| l.0);
        let balance = self.ledger.balance(&acct(fan, "points"), &crate::social::points());
        Value::Array(self.world.def.around.iter().map(|a| {
            let unit = crate::sim::pass_unit(&a.id);
            let held = self.ledger.balance(&acct(fan, "holdings"), &unit) > 0;
            json!({ "id": a.id, "kind": a.kind, "name": a.name, "walk_min": a.walk_min, "blurb": a.blurb, "final_at": final_t,
                    "operator": a.operator, "operator_name": a.operator.as_ref().and_then(|o| self.business_name(o)),
                    "service": a.service.as_ref().map(|sv| json!({ "kind": sv.kind, "price": sv.price, "left": self.ledger.balance(&acct(&a.id, "passes"), &unit), "yours": held,
                        "drivers": self.world.def.drivers.iter().filter(|d| Some(&d.provider) == a.operator.as_ref()).count(),
                        "busy": self.trade.of_kind(crate::exchange::Kind::Ride).filter(|r| r.offer == format!("ride:{}", a.id) && r.stage == crate::exchange::Stage::Active).count() })),
                    "perk": a.perk.as_ref().map(|p| json!({ "title": p.title, "cost": p.cost, "supply": p.supply,
                        "left": self.ledger.balance(&acct(&a.id, "passes"), &unit), "yours": held, "affordable": balance >= p.cost })) })
        }).collect())
    }

    fn passes_of(&self, fan: &str) -> Vec<Value> {
        self.world.def.around.iter().filter(|a| self.ledger.balance(&acct(fan, "holdings"), &crate::sim::pass_unit(&a.id)) > 0 || self.redeemed.contains_key(&(fan.to_string(), a.id.clone())))
            .map(|a| {
                json!({ "place": a.id, "name": a.name, "operator": a.operator.clone().unwrap_or_else(|| a.id.clone()),
                        "title": a.perk.as_ref().map(|p| p.title.clone()).or_else(|| a.service.as_ref().map(|_| "Exit paid. Show this at the gate.".to_string())),
                        "code": crate::dealing::pass_code(fan, &a.id), "used": self.redeemed.get(&(fan.to_string(), a.id.clone())) })
            }).collect()
    }

    fn tickets_of(&self, fan: &str) -> Vec<Value> {
        self.league.upcoming.iter().flat_map(|e| self.seats_of(fan, &e.id).into_iter().map(move |(sec, seats)| (e, sec, seats)))
            .map(|(e, sec, seats)| {
                let listing = self.trade.offers().find(|o| o.kind == crate::exchange::Kind::Resale && o.open && o.maker == fan && o.subject == format!("{}:{}", e.id, sec)).map(|o| json!({ "id": o.id, "price": o.price }));
                let cap = crate::value::bps(self.section_price(&sec) * seats.len() as i64, crate::dealing::RESALE_CAP_BPS);
                json!({ "event": e.id, "title": e.title, "date": e.date, "time": e.time, "section": sec, "seats": seats, "qty": seats.len(), "listing": listing, "resale_cap": cap })
            }).collect()
    }

    fn rides_of(&self, fan: &str) -> Vec<Value> {
        self.trade.of_kind(crate::exchange::Kind::Ride).filter(|r| r.buyer == fan && r.exit != Some(crate::exchange::Exit::BuyerCancelled)).rev().take(3).map(|r| self.ride_json(r)).collect()
    }

    /// Where a ride stands, in the words riders use.
    pub fn ride_stage(a: &crate::exchange::Agreement) -> &'static str {
        use crate::exchange::Stage::*;
        match (a.stage, a.step.as_str()) {
            (Held, _) => "Booked", (Active, "assigned") => "DriverAssigned", (Active, "waiting") => "Waiting", (Active, _) => "OnBoard",
            (Fulfilled | Settled, _) => "Done",
            (Closed, _) => if a.exit == Some(crate::exchange::Exit::NoDoer) { "NoDriver" } else { "Cancelled" },
        }
    }

    pub(crate) fn ride_json(&self, r: &crate::exchange::Agreement) -> Value {
        let d = r.doer.as_ref().and_then(|d| self.world.def.drivers.iter().find(|x| &x.id == d));
        let zone = self.trade.offer(&r.offer).map(|o| o.subject.clone()).unwrap_or_default();
        json!({ "id": r.id, "stage": Self::ride_stage(r), "at": r.at, "fare": r.amount, "zone": zone,
                "zone_name": self.world.def.around.iter().find(|a| a.id == zone).map(|a| a.name.clone()),
                "provider": r.maker, "provider_name": self.business_name(&r.maker),
                "driver": d.map(|d| json!({ "id": d.id, "name": d.name, "vehicle": d.vehicle, "plate": d.plate, "rating": d.rating })) })
    }

    pub fn business_name(&self, id: &str) -> Option<String> {
        self.world.def.businesses.iter().find(|b| b.id == id).map(|b| b.name.clone())
            .or_else(|| self.world.def.stands.iter().find(|s| s.id == id).map(|s| s.name.clone()))
            .or_else(|| self.world.def.around.iter().find(|a| a.id == id).map(|a| a.name.clone()))
    }

    fn me_json(&self, fan: &str, active: &[Value], receipts: Vec<Value>, spent: i64, holdings: Vec<Value>) -> Value {
        let level = self.level(fan);
        let (tier, floor, next) = crate::social::tier(level);
        let balance = self.ledger.balance(&acct(fan, "points"), &crate::social::points());
        let following: Vec<Value> = self.social.follows.get(fan).map(|s| s.iter().map(|id| json!({ "id": id, "name": self.party_name(id), "kind": self.node_kind(id) })).collect()).unwrap_or_default();
        let mut threads: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for (t, from, to, text) in &self.social.messages {
            if from != fan && to != fan { continue; }
            let other = if from == fan { to } else { from };
            threads.entry(other.clone()).or_default().push(json!({ "t": t, "mine": from == fan, "text": text }));
        }
        json!({
            "orders": active, "receipts": receipts, "spent": spent, "holdings": holdings, "tickets": self.tickets_of(fan), "passes": self.passes_of(fan), "rides": self.rides_of(fan),
            "level": level, "tier": tier, "tier_floor": floor, "next_tier": next.map(|(n, f)| json!({ "name": n, "at": f })),
            "balance": balance, "points": balance, "earned": self.earn_history(fan), "badges": self.badges(fan), "cards": self.cards_of(fan),
            "following": following, "deals": self.deals_of(fan), "passport": self.passport_json(fan),
            "messages": threads.into_iter().map(|(k, v)| json!({ "with": k, "name": self.party_name(&k), "lines": v })).collect::<Vec<_>>(),
            "events": [{ "name": format!("{} vs {}", self.league.team(&self.league.tonight.home).map(|g| g.short.clone()).unwrap_or_default(),
                                                    self.league.team(&self.league.tonight.away).map(|g| g.short.clone()).unwrap_or_default()), "when": "Tonight", "venue": self.venue_name() }],
        })
    }

    pub fn node_kind(&self, id: &str) -> &'static str {
        if self.league.member(id).is_some() { "member" } else if self.league.team(id).is_some() { "group" }
        else if self.social.editions.contains_key(id) { "card" } else if id.starts_with('g') && id.len() <= 3 { "game" }
        else if self.league.upcoming.iter().any(|e| e.id == id) { "event" }
        else if id == crate::sim::VENUE { "venue" }
        else if id == "sFL" || (id.len() == 4 && id.starts_with('s') && id[1..].chars().all(|c| c.is_ascii_digit())) { "section" }
        else if self.business_name(id).is_some() { "business" }
        else if self.world.def.drivers.iter().any(|d| d.id == id) { "driver" }
        else { "fan" }
    }

    /// Any node, as a page: who or what it is, how it's connected, its stats, its stream, and what you can do.
    pub fn view_page(&self, id: &str, viewer: &str) -> Value {
        let follows = self.social.follows.get(viewer).map_or(false, |s| s.contains(id));
        let base = json!({ "id": id, "node": self.node_kind(id), "kind": self.node_kind(id), "followers": self.social.followers(id), "you_follow": follows });
        let mut v = match self.node_kind(id) {
            "member" => {
                let (g, m) = self.league.member(id).unwrap();
                let moments: Vec<Value> = self.feed.iter().rev().filter(|f| f.party == id).take(20).map(|f| self.feed_json(f, viewer)).collect();
                let cards: Vec<Value> = self.social.editions.values().filter(|e| e.party.as_deref() == Some(id)).map(|e| self.edition_json(&e.id, viewer)).collect();
                json!({ "name": m.name, "number": m.number, "pos": m.pos, "star": m.star, "group": { "id": g.id, "name": g.name, "color": g.color },
                        "season": { "ppg": m.ppg, "rpg": m.rpg, "apg": m.apg }, "tonight": { "points": self.points_tonight(id), "playing": g.id == self.league.tonight.home || g.id == self.league.tonight.away },
                        "stream": moments, "cards": cards, "claimed": self.claimed.contains(id), "rates": self.rates_json(id, viewer),
                        "your_deals": self.trade.of_kind(crate::exchange::Kind::Deal).filter(|d| (d.buyer == viewer && d.maker == id) || (d.maker == viewer && d.buyer == id)).map(|d| self.deal_json(d, viewer)).collect::<Vec<_>>(),
                        "balance": self.ledger.balance(&acct(viewer, "points"), &crate::social::points()),
                        "thread": self.social.messages.iter().filter(|m| (m.1 == viewer && m.2 == id) || (m.1 == id && m.2 == viewer)).map(|m| json!({ "t": m.0, "mine": m.1 == viewer, "text": m.3 })).collect::<Vec<_>>() })
            }
            "group" => {
                let g = self.league.team(id).unwrap();
                let roster: Vec<Value> = g.roster.iter().map(|m| json!({ "id": m.id, "name": m.name, "number": m.number, "pos": m.pos, "ppg": m.ppg, "tonight": self.points_tonight(&m.id) })).collect();
                let side = if self.league.tonight.home == id { Some("home") } else if self.league.tonight.away == id { Some("away") } else { None };
                let stream: Vec<Value> = self.feed.iter().rev().filter(|f| side.map_or(false, |s| f.team == s) && f.kind == "moment").take(20).map(|f| self.feed_json(f, viewer)).collect();
                let cards: Vec<Value> = self.social.editions.values().filter(|e| e.team == id).map(|e| self.edition_json(&e.id, viewer)).collect();
                json!({ "name": g.name, "short": g.short, "abbr": g.abbr, "color": g.color, "record": format!("{}–{}", g.wins, g.losses), "roster": roster,
                        "tonight": side.map(|s| json!({ "side": s, "score": if s == "home" { self.score.0 } else { self.score.1 }, "opponent": if s == "home" { &self.league.tonight.away } else { &self.league.tonight.home } })),
                        "stream": stream, "cards": cards })
            }
            "card" => {
                let mut e = self.edition_json(id, viewer);
                let listings: Vec<Value> = { let mut l: Vec<_> = self.listings().filter(|l| l.subject == id).collect(); l.sort_by_key(|l| l.price);
                    l.into_iter().take(8).map(|l| json!({ "id": l.id, "price": l.price, "seller": l.maker, "seller_name": self.party_name(&l.maker), "yours": l.maker == viewer })).collect() };
                let sales: Vec<Value> = self.trade.of_kind(crate::exchange::Kind::Card).rev().filter(|a| a.stage == crate::exchange::Stage::Settled && self.trade.offer(&a.offer).map_or(false, |o| o.subject == id)).take(6)
                    .map(|a| json!({ "t": a.made, "price": a.amount, "buyer": self.party_name(&a.buyer) })).collect();
                e["listings"] = Value::Array(listings);
                e["sales"] = Value::Array(sales);
                e
            }
            "venue" => {
                let stands: Vec<Value> = self.world.def.stands.iter().map(|s| json!({ "id": s.id, "name": s.name, "level": s.level, "wait": self.stand_wait(&s.id) })).collect();
                let biz: Vec<Value> = self.world.def.businesses.iter().map(|b| json!({ "id": b.id, "name": b.name, "kind": b.kind })).collect();
                json!({ "name": self.venue_name(), "address": self.world.def.address, "capacity": self.world.def.capacity, "levels": self.world.def.levels,
                        "sections": self.ticket_sections().len(), "stands": stands, "events": self.events_json(viewer), "businesses": biz,
                        "home_of": [self.league.tonight.home.clone()], "tonight": self.party_name(&self.league.tonight.home).map(|h| format!("{h} vs {}", self.party_name(&self.league.tonight.away).unwrap_or_default())) })
            }
            "event" => {
                let e = self.league.upcoming.iter().find(|e| e.id == id).unwrap();
                let secs: Vec<Value> = self.ticket_sections().iter().map(|sec| {
                    let m = &self.seatmaps[&(id.to_string(), sec.clone())];
                    json!({ "id": sec, "price": self.section_price(sec), "left": m.free(), "of": m.rows * m.per_row, "view": self.section_view(sec) })
                }).collect();
                let mine: Vec<Value> = self.seats_of(viewer, id).into_iter().map(|(s, v)| json!({ "section": s, "seats": v })).collect();
                let team = |t: &Option<String>| t.as_ref().and_then(|g| self.league.team(g)).map(|g| json!({ "id": g.id, "name": g.name, "color": g.color, "record": format!("{}–{}", g.wins, g.losses) }));
                json!({ "name": e.title, "title": e.title, "event_kind": e.kind, "date": e.date, "time": e.time, "home": team(&e.home), "away": team(&e.away),
                        "venue": { "id": crate::sim::VENUE, "name": self.venue_name(), "address": self.world.def.address }, "sections": secs, "yours": mine,
                        "resale": self.resale_json(id, viewer),
                        "map": self.view_map(None, None) })
            }
            "section" => {
                let events: Vec<Value> = self.league.upcoming.iter().map(|e| {
                    let m = &self.seatmaps[&(e.id.clone(), id.to_string())];
                    json!({ "id": e.id, "title": e.title, "date": e.date, "left": m.free(), "price": self.section_price(id) })
                }).collect();
                let near = if id == "sFL" { vec![] } else {
                    let mut v: Vec<(u32, &crate::world::StandDef)> = self.world.def.stands.iter().map(|st| (self.world.graph.path(&st.id, id, false).map_or(9999, |p| p.0), st)).collect();
                    v.sort_by_key(|x| x.0);
                    v.into_iter().take(3).map(|(sec, st)| json!({ "id": st.id, "name": st.name, "walk_s": sec })).collect()
                };
                json!({ "name": if id == "sFL" { "Courtside floor".to_string() } else { format!("Section {}", &id[1..]) }, "view": self.section_view(id),
                        "level": if id == "sFL" { 0 } else { id[1..2].parse::<i64>().unwrap_or(1) }, "events": events, "nearby": near, "yours_tonight": self.fans.iter().any(|f| f.id == viewer && f.seat == id) })
            }
            "business" => {
                let b = self.world.def.businesses.iter().find(|b| b.id == id);
                let stand = self.world.def.stands.iter().find(|s| s.id == id);
                let place = self.world.def.around.iter().find(|a| a.operator.as_deref() == Some(id) || a.id == id);
                let menu: Vec<Value> = self.world.def.things.iter().filter(|t| t.stand == id && t.id != "combo").map(|t| json!({ "id": t.id, "name": t.name, "price": t.price })).collect();
                let drivers: Vec<Value> = self.world.def.drivers.iter().filter(|d| d.provider == id).map(|d| json!({ "id": d.id, "name": d.name, "rating": d.rating, "vehicle": d.vehicle,
                    "busy": self.trade.of_kind(crate::exchange::Kind::Ride).any(|r| r.doer.as_deref() == Some(&d.id) && r.stage == crate::exchange::Stage::Active) })).collect();
                let around = self.around_json(viewer);
                let here = around.as_array().and_then(|a| a.iter().find(|x| x["operator"] == id || x["id"] == id).cloned());
                json!({ "name": b.map(|b| b.name.clone()).or(stand.map(|s| s.name.clone())).or(place.map(|p| p.name.clone())), "business_kind": b.map(|b| b.kind.clone()).unwrap_or_else(|| "stand".into()),
                        "blurb": b.map(|b| b.blurb.clone()).or(place.map(|p| p.blurb.clone())).unwrap_or_else(|| stand.map(|s| format!("Level {} concourse, {} stations.", s.level, s.stations)).unwrap_or_default()),
                        "hours": b.map(|b| b.hours.clone()).unwrap_or_else(|| "Open during events".into()), "walk_min": place.map(|p| p.walk_min),
                        "wait": stand.map(|s| self.stand_wait(&s.id)), "menu": menu, "drivers": drivers, "here": here })
            }
            "driver" => {
                let d = self.world.def.drivers.iter().find(|d| d.id == id).unwrap();
                let tonight = self.trade.of_kind(crate::exchange::Kind::Ride).filter(|r| r.doer.as_deref() == Some(id) && r.stage == crate::exchange::Stage::Settled).count();
                let with_you = self.trade.of_kind(crate::exchange::Kind::Ride).filter(|r| r.buyer == viewer && r.doer.as_deref() == Some(id)).last().map(|r| self.ride_json(r));
                json!({ "name": d.name, "rating": d.rating, "trips": d.trips, "vehicle": d.vehicle, "plate": d.plate, "provider": { "id": d.provider, "name": self.business_name(&d.provider) },
                        "tonight": tonight, "with_you": with_you })
            }
            "game" => {
                let games = self.league_tonight();
                games.as_array().and_then(|a| a.iter().find(|g| g["id"] == id).cloned()).unwrap_or(Value::Null)
            }
            _ => {
                // a fan: their public side only. Never their orders, seat or spending.
                let level = self.level(id);
                let chat: Vec<Value> = self.chat.iter().rev().filter(|c| c.1 == id).take(8).map(|c| json!({ "t": c.0, "text": c.3 })).collect();
                let thread: Vec<Value> = self.social.messages.iter().filter(|m| (m.1 == viewer && m.2 == id) || (m.1 == id && m.2 == viewer))
                    .map(|m| json!({ "t": m.0, "mine": m.1 == viewer, "text": m.3 })).collect();
                json!({ "name": self.party_name(id), "level": level, "tier": crate::social::tier(level).0, "badges": self.badges(id),
                        "cards": self.cards_of(id), "said": chat, "following": self.social.follows.get(id).map_or(0, |s| s.len()), "thread": thread, "is_you": id == viewer })
            }
        };
        if let (Value::Object(a), Value::Object(b)) = (&mut v, base) { for (k, x) in b { a.entry(k).or_insert(x); } }
        v
    }
}

/// Convenience for the live fan.
pub fn you(n: &mut Night) -> Value { n.view_fan(YOU) }

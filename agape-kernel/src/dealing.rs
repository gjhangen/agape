//! The Night's side of exchanges: every ticket, ride, pass, perk, deal, card trade and drop is an
//! Offer accepted into an Agreement through one path (`accept`, `advance`), checked by one gate
//! (eligibility, window, capacity, affordability) and one Agape check, recorded as signals, and
//! settled in the ledger with the agreement's id on every transaction.
//!
//! Social acts (follows, messages, claims, redemptions, picks) are signals too; the state they
//! change is folded from them in `absorb_signal`.

use crate::deals::{Pay, Rate};
use crate::exchange::{allowed, Agreement, Exit, Kind, Offer, Scope, Stage};
use crate::fields;
use crate::signal::{Fields, Signal};
use crate::sim::{pass_unit, Night, VENUE, YOU};
use crate::social::{card_unit, points, ISSUER, MARKET_FEE_BPS};
use crate::value::{acct, bps, posting, Txn, Unit};

/// On a ticket resold between fans: this much to the venue and the same to Agape, from the seller.
pub const RESALE_FEE_BPS: i64 = 500;
/// The most a ticket can be resold for, as a share of what it cost (no scalping).
pub const RESALE_CAP_BPS: i64 = 12_000;

/// Agape's share when a deal settles.
pub const DEAL_FEE_BPS: i64 = 1_000;
/// A buyer who doesn't release a delivered deal within this long is taken to accept it.
pub const AUTO_CONFIRM_MS: i64 = 15 * 60_000;

/// A short code a fan shows at a counter for a pass or perk, stable for that fan and place.
pub fn pass_code(fan: &str, place: &str) -> String {
    let code: u64 = format!("{fan}:{place}").bytes().fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64)) % 900_000 + 100_000;
    code.to_string()
}

/// The exact terms an offer would be accepted on, before anyone says yes.
#[derive(Clone, Debug)]
pub struct Quote { pub amount: i64, pub detail: String }

fn tag(mut t: Txn, id: &str) -> Txn { t.memo = format!("{} x:{id}", t.memo); t }

impl Night {
    // ---------------- signals ----------------

    /// Append a signal to the log, let the rules see it, and fold it into state.
    pub(crate) fn record(&mut self, kind: &str, f: Fields, by: &str) -> Signal {
        let at = self.log.signals.len();
        let outs = self.engine.ingest(&mut self.log, kind, f, by, self.t).expect("platform signal kinds are in the schema");
        let sig = self.log.signals[at].clone();
        self.absorb_signal(&sig);
        self.absorb(outs);
        sig
    }

    /// Fold one signal into the state it changes. Nothing else changes that state.
    pub(crate) fn absorb_signal(&mut self, sig: &Signal) {
        let f = |k: &str| sig.fields.get(k).map(|v| v.key()).unwrap_or_default();
        match sig.kind.as_str() {
            "offer" => self.trade.apply(sig),
            "exchange" => {
                let new = self.trade.get(&f("agreement")).is_none();
                self.trade.apply(sig);
                // an upgrade moves the fan; a resale moves the seats to the buyer
                if new { if let Some(a) = self.trade.get(&f("agreement")).cloned() {
                    if a.kind == Kind::Upgrade {
                        if let Some(sec) = self.trade.offer(&a.offer).map(|o| o.subject.clone()) { if let Some(fan) = self.fans.iter_mut().find(|x| x.id == a.buyer) { fan.seat = sec; } }
                    }
                    if a.kind == Kind::Resale {
                        if let Some(o) = self.trade.offer(&a.offer).cloned() { if let Some((ev, sec)) = o.subject.split_once(':') {
                            if let Some(m) = self.seatmaps.get_mut(&(ev.to_string(), sec.to_string())) { for seat in o.sub.split(',') { if let Some(i) = m.index_of(seat) { m.owner[i] = Some(a.buyer.clone()); } } }
                        } }
                    }
                } }
                // a ticket purchase takes its seats on the seat map
                if new { if let Some(a) = self.trade.get(&f("agreement")).cloned() { if a.kind == Kind::Ticket {
                    if let Some((ev, sec)) = self.trade.offer(&a.offer).and_then(|o| o.subject.split_once(':').map(|(e, s)| (e.to_string(), s.to_string()))) {
                        if let Some(m) = self.seatmaps.get_mut(&(ev, sec)) { for seat in a.detail.split(',') { if let Some(i) = m.index_of(seat) { m.owner[i] = Some(a.buyer.clone()); } } }
                    }
                } } }
            }
            "social" => {
                let (target, text) = (f("target"), f("text"));
                match f("act").as_str() {
                    "follow" => { self.social.follows.entry(sig.author.clone()).or_default().insert(target); }
                    "unfollow" => { if let Some(s) = self.social.follows.get_mut(&sig.author) { s.remove(&target); } }
                    "message" => self.social.messages.push((sig.t_ms, sig.author.clone(), target, text)),
                    "claim" => { self.claimed.insert(sig.author.clone()); }
                    "redeem" => { self.redeemed.insert((text, target), sig.t_ms); }
                    "rate" => { if let Some((o, stars)) = text.split_once(':') { if let (Ok(o), Ok(stars)) = (o.parse(), stars.parse()) { self.ratings.push((o, target, stars, sig.author.clone(), sig.t_ms)); } } }
                    "pick" => { if let Some(p) = self.predictions.iter_mut().rev().find(|p| p.ask == target) { p.picks.insert(sig.author.clone(), text); } }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub(crate) fn social_act(&mut self, by: &str, act: &str, target: &str, text: &str) -> Signal {
        self.record("social", fields!{"act" => act, "target" => target, "text" => text}, by)
    }

    // ---------------- offers ----------------

    pub fn open_offer(&mut self, o: Offer) {
        let by = o.maker.clone();
        let spec = serde_json::to_string(&o).unwrap();
        self.record("offer", fields!{"offer" => o.id.as_str(), "act" => "open", "spec" => spec}, &by);
    }
    pub fn close_offer(&mut self, id: &str, by: &str) {
        self.record("offer", fields!{"offer" => id, "act" => "close", "spec" => ""}, by);
    }

    /// Every exchange offer in the world, as the template and the parties made them.
    pub(crate) fn seed_offers(&mut self) {
        let mk = |id: String, kind: Kind, maker: &str, title: String, blurb: String, pay: Pay, price: i64, scope: Scope, limit: Option<i64>, subject: String, sub: &str| Offer {
            id, kind, maker: maker.into(), title, blurb, pay, price, scope, limit, subject, sub: sub.into(), opens: 0, closes: None, open: true };
        let mut all = vec![];
        for e in self.league.upcoming.clone() {
            for sec in self.ticket_sections() {
                let name = if sec == "sFL" { "Courtside".to_string() } else { format!("Section {}", &sec[1..]) };
                all.push(mk(format!("tix:{}:{sec}", e.id), Kind::Ticket, VENUE, format!("{} · {name}", e.title), self.section_view(&sec).into(), Pay::Money, self.section_price(&sec), Scope::Fans, None, format!("{}:{sec}", e.id), "seat"));
            }
        }
        for a in self.world.def.around.clone() {
            let op = a.operator.clone().unwrap_or_else(|| a.id.clone());
            if let Some(sv) = &a.service {
                match sv.kind.as_str() {
                    "ride" => all.push(mk(format!("ride:{}", a.id), Kind::Ride, &op, a.name.clone(), a.blurb.clone(), Pay::Money, crate::services::ride_fare(0, 1), Scope::Fans, None, a.id.clone(), "ride")),
                    "parking" => all.push(mk(format!("park:{}", a.id), Kind::Parking, &op, a.name.clone(), a.blurb.clone(), Pay::Money, sv.price, Scope::Fans, None, a.id.clone(), "parking")),
                    _ => {}
                }
            }
            if let Some(p) = &a.perk { all.push(mk(format!("perk:{}", a.id), Kind::Perk, &a.id, p.title.clone(), a.blurb.clone(), Pay::Points, p.cost, Scope::Fans, None, a.id.clone(), "perk")); }
        }
        // tonight's empty seats in the best sections, offered to fans already in the building
        for r in 0..self.world.def.sections_per_level {
            let sec = crate::world::section_id(1, r);
            if self.section_view(&sec) != "Sideline" { continue; }
            let price = self.section_price(&sec) * 35 / 100 / 100 * 100;
            all.push(mk(format!("upg:{sec}"), Kind::Upgrade, VENUE, format!("Move to section {}", &sec[1..]), "Sideline, lower bowl, tonight".into(), Pay::Money, price, Scope::Fans, Some(10), sec.clone(), "upgrade"));
        }
        let mut parties: Vec<String> = self.league.teams.iter().flat_map(|g| g.roster.iter().map(|m| m.id.clone())).collect();
        parties.push(self.league.tonight.home.clone());
        parties.push(self.league.tonight.away.clone());
        for p in parties {
            for r in self.rates_of(&p) {
                let scope = if r.pay == Pay::Money { Scope::Businesses } else { Scope::Fans };
                all.push(mk(format!("rate:{p}:{}", r.kind), Kind::Deal, &p, r.title, r.blurb, r.pay, r.price, scope, r.limit, p.clone(), &r.kind));
            }
        }
        for o in all { self.open_offer(o); }
    }

    /// A card drop: the issuer offers one free card per fan while it's open and supply lasts.
    pub fn open_drop(&mut self, edition: &str) {
        let Some(e) = self.social.editions.get(edition).cloned() else { return };
        if e.open_until < self.t { return; }
        self.open_offer(Offer { id: format!("drop:{edition}"), kind: Kind::Drop, maker: ISSUER.into(), title: e.title.clone(), blurb: format!("Edition of {}", e.supply), pay: Pay::Points,
            price: 0, scope: Scope::Fans, limit: Some(e.supply), subject: edition.into(), sub: e.kind.clone(), opens: self.t, closes: Some(e.open_until), open: true });
    }

    /// A fan puts a card they hold up for points. The card waits in their listed account.
    pub fn list_card(&mut self, fan: &str, edition: &str, price: i64) -> Result<String, String> {
        if price < 1 { return Err("Set a price".into()); }
        if self.ledger.balance(&acct(fan, "holdings"), &card_unit(edition)) < 1 { return Err("NotHeld".into()); }
        let id = format!("list:{}", self.trade.offers().filter(|o| o.kind == Kind::Card).count() + 1);
        self.ledger.post(Txn { memo: format!("list {edition} x:{id}"), postings: vec![
            posting(acct(fan, "holdings"), card_unit(edition), -1), posting(acct(fan, "listed"), card_unit(edition), 1)] }).map_err(|e| format!("{e:?}"))?;
        let title = self.social.editions.get(edition).map(|e| e.title.clone()).unwrap_or_default();
        self.open_offer(Offer { id: id.clone(), kind: Kind::Card, maker: fan.into(), title, blurb: String::new(), pay: Pay::Points, price, scope: Scope::Fans,
            limit: Some(1), subject: edition.into(), sub: "card".into(), opens: self.t, closes: None, open: true });
        Ok(id)
    }

    pub fn delist_card(&mut self, fan: &str, listing: &str) -> Result<(), String> {
        let o = self.trade.offer(listing).filter(|o| o.kind == Kind::Card && o.open).cloned().ok_or("Unknown")?;
        if o.maker != fan { return Err("Unknown".into()); }
        self.ledger.post(Txn { memo: format!("delist {} x:{listing}", o.subject), postings: vec![
            posting(acct(fan, "listed"), card_unit(&o.subject), -1), posting(acct(fan, "holdings"), card_unit(&o.subject), 1)] }).map_err(|e| format!("{e:?}"))?;
        self.close_offer(listing, fan);
        Ok(())
    }

    /// Put your tickets for one section of an event back on sale to other fans, at up to 120% of
    /// what they cost. The tickets wait in your listed account until sold or taken back.
    pub fn resell(&mut self, fan: &str, event: &str, section: &str, price: i64) -> Result<String, String> {
        self.halted()?;
        let seats: Vec<String> = self.seats_of(fan, event).into_iter().find(|(s, _)| s == section).map(|(_, v)| v).unwrap_or_default();
        if seats.is_empty() { return Err("You don't have seats there".into()); }
        if self.trade.offers().any(|o| o.kind == Kind::Resale && o.open && o.maker == fan && o.subject == format!("{event}:{section}")) { return Err("They're already on sale".into()); }
        let qty = seats.len() as i64;
        let face = self.section_price(section) * qty;
        let cap = bps(face, RESALE_CAP_BPS);
        if price > cap { return Err(format!("Up to {} for these, 120% of what they cost", crate::sim::dollars(cap))); }
        if price < 20_00 { return Err("At least $20".into()); }
        let unit = crate::economy::ticket_unit(event, section);
        let id = format!("resale:{}", self.trade.offers().filter(|o| o.kind == Kind::Resale).count() + 1);
        self.ledger.post(Txn { memo: format!("list tickets {event} x:{id}"), postings: vec![posting(acct(fan, "holdings"), unit.clone(), -qty), posting(acct(fan, "listed"), unit, qty)] }).map_err(|e| format!("{e:?}"))?;
        let title = self.league.upcoming.iter().find(|e| e.id == event).map(|e| e.title.clone()).unwrap_or_default();
        self.open_offer(Offer { id: id.clone(), kind: Kind::Resale, maker: fan.into(), title, blurb: format!("{} seats", qty), pay: Pay::Money, price, scope: Scope::Fans,
            limit: Some(1), subject: format!("{event}:{section}"), sub: seats.join(","), opens: self.t, closes: None, open: true });
        Ok(id)
    }

    pub fn unlist_tickets(&mut self, fan: &str, listing: &str) -> Result<(), String> {
        let o = self.trade.offer(listing).filter(|o| o.kind == Kind::Resale && o.open && o.maker == fan).cloned().ok_or("Not your listing")?;
        let (ev, sec) = o.subject.split_once(':').unwrap();
        let qty = o.sub.split(',').count() as i64;
        let unit = crate::economy::ticket_unit(ev, sec);
        self.ledger.post(Txn { memo: format!("unlist tickets {ev} x:{listing}"), postings: vec![posting(acct(fan, "listed"), unit.clone(), -qty), posting(acct(fan, "holdings"), unit, qty)] }).map_err(|e| format!("{e:?}"))?;
        self.close_offer(listing, fan);
        Ok(())
    }

    // ---------------- the gate ----------------

    fn is_fan(&self, p: &str) -> bool { self.fans.iter().any(|f| f.id == p) }

    /// Whether this party can accept this offer on these terms right now, and the exact terms.
    /// Views use the same gate, so nothing is shown that can't be accepted without its reason.
    pub fn gate(&self, offer: &str, party: &str, qty: i64, at: Option<i64>) -> Result<Quote, String> {
        self.halted()?;
        let o = self.trade.offer(offer).ok_or("Unknown offer")?;
        if !o.open { return Err(if o.kind == Kind::Card { "Unknown".into() } else { "Closed".into() }); }
        if self.t < o.opens || o.closes.map_or(false, |c| self.t > c) { return Err("Closed".into()); }
        if o.maker == party { return Err(match o.kind { Kind::Card => "OwnListing".into(), Kind::Resale => "These are your tickets".into(), _ => "You can't make a deal with yourself".into() }); }
        match o.scope {
            Scope::Fans if !self.is_fan(party) => return Err("That's for fans".into()),
            Scope::Businesses if !self.is_business(party) => return Err("Only businesses pay money for deals".into()),
            _ => {}
        }
        if let Some(l) = o.limit { if self.trade.taken(offer) >= l { return Err(match o.kind { Kind::Deal => "They're fully booked tonight".into(), Kind::Drop => "Closed".into(), _ => "SoldOut".into() }); } }
        let q = match o.kind {
            Kind::Ticket => {
                let (ev, sec) = o.subject.split_once(':').ok_or("Unknown")?;
                let (b, _, seats) = self.ticket_quote(ev, sec, qty)?;
                if seats.is_empty() { return Err("SoldOut".into()); }
                Quote { amount: b.charge, detail: seats.join(",") }
            }
            Kind::Ride => {
                let at = at.ok_or("Choose a time")?;
                if at < self.t + 5 * 60_000 { return Err("Give the driver at least 5 minutes".into()); }
                if at > self.script_final() + 60 * 60_000 { return Err("Rides can be booked up to an hour after the final".into()); }
                if self.trade.of_kind(Kind::Ride).any(|a| a.buyer == party && a.live()) { return Err("You already have a ride booked".into()); }
                if self.drivers_free_at(&o.maker, at) < 1 { return Err("NoDriver".into()); }
                Quote { amount: self.ride_quote(at), detail: String::new() }
            }
            Kind::Parking | Kind::Perk => {
                if self.ledger.balance(&acct(party, "holdings"), &pass_unit(&o.subject)) > 0 || self.redeemed.contains_key(&(party.to_string(), o.subject.clone())) {
                    return Err(if o.kind == Kind::Perk { "Already yours".into() } else { "Already paid".into() });
                }
                if self.ledger.balance(&acct(&o.subject, "passes"), &pass_unit(&o.subject)) < 1 { return Err("SoldOut".into()); }
                Quote { amount: o.price, detail: String::new() }
            }
            Kind::Drop => {
                if self.trade.all().iter().any(|a| a.buyer == party && a.offer == offer) { return Err("AlreadyCollected".into()); }
                if self.social.left(&self.ledger, &o.subject) < 1 { return Err("Closed".into()); }
                Quote { amount: 0, detail: String::new() }
            }
            Kind::Upgrade => {
                let me = self.fans.iter().find(|f| f.id == party).ok_or("Join the night first")?;
                if self.phase() == "postgame" { return Err("The game's over".into()); }
                if self.trade.of_kind(Kind::Upgrade).any(|a| a.buyer == party) { return Err("You've already moved seats tonight".into()); }
                if me.seat == o.subject { return Err("You're already there".into()); }
                if self.world.level_of(&me.seat) == Some(1) && self.section_view(&me.seat) == "Sideline" { return Err("You're already in one of the best sections".into()); }
                Quote { amount: o.price, detail: String::new() }
            }
            Kind::Resale => {
                if !self.fans.iter().any(|f| f.id == party) { return Err("That's for fans".into()); }
                Quote { amount: o.price, detail: o.sub.clone() }
            }
            Kind::Deal | Kind::Card => Quote { amount: o.price, detail: String::new() },
        };
        let afford = match (o.kind, &o.pay) {
            (_, Pay::Points) => self.ledger.balance(&acct(party, "points"), &points()) >= q.amount,
            (Kind::Deal, Pay::Money) => self.ledger.balance(&acct(party, "deal_budget"), &Unit::usd()) >= q.amount,
            _ => true, // charged to the buyer's card
        };
        if !afford { return Err("CantAfford".into()); }
        Ok(q)
    }

    /// Invariant 1, checked before anyone says yes: in money and points, every party but the payer
    /// comes out at zero or better. Issuers, escrow and external accounts are not parties' value.
    pub fn agape_check(&self, txn: &Txn, payer: &str) -> Result<(), String> {
        let mut net: std::collections::BTreeMap<(String, String), i64> = Default::default();
        for p in &txn.postings {
            if p.account.party == payer || matches!(p.account.purpose.as_str(), "card" | "bank" | "supply" | "issued") || p.account.party == "escrow" { continue; }
            if !matches!(p.unit, Unit::Money(_) | Unit::Points(_)) { continue; }
            *net.entry((p.account.party.clone(), format!("{:?}", p.unit))).or_default() += p.amount;
        }
        match net.into_iter().find(|(_, v)| *v < 0) { Some(((party, _), _)) => Err(format!("Loses({party})")), None => Ok(()) }
    }

    /// What the agreement will post when it settles.
    fn settlement(&self, a: &Agreement) -> Txn {
        let o = self.trade.offer(&a.offer).expect("offer");
        let t = match a.kind {
            Kind::Ticket => {
                let (ev, sec) = o.subject.split_once(':').unwrap();
                let b = crate::economy::ticket_breakdown(self.section_price(sec), a.qty, &self.terms);
                crate::economy::ticket_sale(&a.buyer, VENUE, ev, sec, a.qty, &b, &self.terms)
            }
            Kind::Parking => crate::services::parking_sale(&a.buyer, &a.maker, &o.subject, a.amount, &self.terms),
            Kind::Perk => Txn { memo: format!("unlock {}", o.subject), postings: vec![
                posting(acct(&a.buyer, "points"), points(), -a.amount), posting(acct(ISSUER, "issued"), points(), a.amount),
                posting(acct(&o.subject, "passes"), pass_unit(&o.subject), -1), posting(acct(&a.buyer, "holdings"), pass_unit(&o.subject), 1)] },
            Kind::Ride => crate::services::ride_settlement(&crate::services::Ride { id: a.id.clone(), fan: a.buyer.clone(), provider: a.maker.clone(),
                driver: a.doer.clone().or(Some("driver".into())), zone: o.subject.clone(), at: a.at.unwrap_or(0), fare: a.amount, stage: crate::services::RideStage::Done }, &self.terms),
            Kind::Deal => {
                let u = crate::deals::unit_of(&a.pay);
                let fee = bps(a.amount, DEAL_FEE_BPS);
                let (to, fee_acct) = match a.pay { Pay::Points => (acct(&a.maker, "points"), acct(ISSUER, "issued")), Pay::Money => (acct(&a.maker, "earnings"), acct("agape", "revenue")) };
                Txn { memo: "deal settle".into(), postings: vec![posting(acct("escrow", &a.id), u.clone(), -a.amount), posting(to, u.clone(), a.amount - fee), posting(fee_acct, u, fee)] }
            }
            Kind::Card => {
                let fee = bps(a.amount, MARKET_FEE_BPS);
                Txn { memo: format!("sale {}", o.subject), postings: vec![
                    posting(acct(&a.buyer, "points"), points(), -a.amount), posting(acct(&a.maker, "points"), points(), a.amount - fee), posting(acct(ISSUER, "issued"), points(), fee),
                    posting(acct(&a.maker, "listed"), card_unit(&o.subject), -1), posting(acct(&a.buyer, "holdings"), card_unit(&o.subject), 1)] }
            }
            Kind::Upgrade => {
                let usd = Unit::usd();
                let share = bps(a.amount, self.terms.platform_share_bps);
                let (mut p, _) = crate::economy::card_postings(&a.buyer, acct(VENUE, "revenue"), a.amount, a.amount - share, &self.terms);
                p.push(posting(acct("agape", "revenue"), usd, share));
                Txn { memo: format!("upgrade {}", o.subject), postings: p }
            }
            Kind::Resale => {
                let usd = Unit::usd();
                let (ev, sec) = o.subject.split_once(':').unwrap();
                let unit = crate::economy::ticket_unit(ev, sec);
                let each = bps(a.amount, RESALE_FEE_BPS);
                // the seller pays processing on their sale, out of what they're paid
                let (mut p, _) = crate::economy::card_postings(&a.buyer, acct(&a.maker, "payouts"), a.amount, a.amount - 2 * each, &self.terms);
                p.extend([posting(acct(VENUE, "revenue"), usd.clone(), each), posting(acct("agape", "revenue"), usd, each),
                    posting(acct(&a.maker, "listed"), unit.clone(), -a.qty), posting(acct(&a.buyer, "holdings"), unit, a.qty)]);
                Txn { memo: format!("resale {ev}"), postings: p }
            }
            Kind::Drop => Txn { memo: format!("collect {}", o.subject), postings: vec![
                posting(acct(ISSUER, "drops"), card_unit(&o.subject), -1), posting(acct(&a.buyer, "holdings"), card_unit(&o.subject), 1)] },
        };
        tag(t, &a.id)
    }

    fn escrow_txn(a: &Agreement, back: bool) -> Txn {
        let u = crate::deals::unit_of(&a.pay);
        let from = match a.pay { Pay::Points => acct(&a.buyer, "points"), Pay::Money => acct(&a.buyer, "deal_budget") };
        let (x, y) = if back { (acct("escrow", &a.id), from) } else { (from, acct("escrow", &a.id)) };
        tag(Txn { memo: if back { "deal returned".into() } else { "deal escrow".into() }, postings: vec![posting(x, u.clone(), -a.amount), posting(y, u, a.amount)] }, &a.id)
    }

    // ---------------- accept and advance ----------------

    /// Accept an offer. Returns the agreement's id. Everything a later failure could need is held
    /// now (money in escrow, points, stock), so a yes can't bounce.
    pub fn accept(&mut self, offer: &str, party: &str, qty: i64, at: Option<i64>, note: &str, via: &str) -> Result<String, String> {
        let q = self.gate(offer, party, qty, at)?;
        let o = self.trade.offer(offer).cloned().unwrap();
        let id = self.trade.next_id();
        let detail: String = if o.kind == Kind::Ticket { q.detail.clone() } else { note.trim().chars().take(200).collect() };
        let draft = Agreement { id: id.clone(), offer: offer.into(), kind: o.kind, buyer: party.into(), maker: o.maker.clone(), doer: None, qty, amount: q.amount, pay: o.pay.clone(),
            at, detail: detail.clone(), proof: None, step: String::new(), stage: Stage::Held, exit: None, via: via.into(), made: self.t, history: vec![] };
        let settle = self.settlement(&draft);
        self.agape_check(&settle, party)?;
        let instant = matches!(o.kind, Kind::Ticket | Kind::Parking | Kind::Perk | Kind::Card | Kind::Drop | Kind::Upgrade | Kind::Resale);
        if instant { self.ledger.post(settle).map_err(|e| format!("{e:?}"))?; }
        if o.kind == Kind::Deal { self.ledger.post(Self::escrow_txn(&draft, false)).map_err(|_| "CantAfford".to_string())?; }
        let stage = if instant { "settled" } else { "held" };
        self.record("exchange", fields!{"agreement" => id.as_str(), "offer" => offer, "stage" => stage, "step" => "", "amount" => q.amount, "qty" => qty,
            "at" => at.unwrap_or(0), "detail" => detail, "via" => via}, party);
        if matches!(o.kind, Kind::Card | Kind::Resale) { self.close_offer(offer, &o.maker); }
        Ok(id)
    }

    /// Move an agreement along its lifecycle. `system` is the kernel acting on time (assigning a
    /// driver, releasing a quiet deal), never a person.
    pub fn advance(&mut self, id: &str, by: &str, to: Stage, exit: Option<Exit>, step: &str, detail: &str, system: bool) -> Result<(), String> {
        let a = self.trade.get(id).cloned().ok_or("That deal doesn't exist")?;
        allowed(&a, to, by, exit, system).map_err(|e| match (a.kind, e) {
            (Kind::Ride, "That has moved on") => "The driver is already here".to_string(),
            (Kind::Deal, "That has moved on") => "That deal has moved on".to_string(),
            (_, e) => e.to_string(),
        })?;
        match (a.kind, to) {
            (Kind::Deal, Stage::Closed) => { self.ledger.post(Self::escrow_txn(&a, true)).map_err(|e| format!("{e:?}"))?; }
            (Kind::Deal, Stage::Fulfilled) if detail.trim().is_empty() => return Err("Say what you did".into()),
            (_, Stage::Settled) => { self.ledger.post(self.settlement(&a)).map_err(|e| format!("{e:?}"))?; }
            _ => {}
        }
        let step = exit.map(|e| e.name().to_string()).unwrap_or_else(|| step.to_string());
        self.record("exchange", fields!{"agreement" => id, "offer" => a.offer.as_str(), "stage" => crate::exchange::stage_name(to), "step" => step,
            "amount" => a.amount, "qty" => a.qty, "at" => a.at.unwrap_or(0), "detail" => detail.trim(), "via" => a.via.as_str()}, by);
        // a shoutout lands where the fan reads their messages
        if a.kind == Kind::Deal && to == Stage::Fulfilled && self.trade.offer(&a.offer).map_or(false, |o| o.sub == "shoutout") {
            self.social_act(&a.maker, "message", &a.buyer, detail.trim());
        }
        Ok(())
    }

    /// Time moves agreements that wait on it: drivers are assigned and trips run; quiet deals release.
    pub(crate) fn advance_trade(&mut self) {
        let t = self.t;
        let live: Vec<Agreement> = self.trade.all().iter().filter(|a| a.live() && matches!(a.kind, Kind::Ride | Kind::Deal)).cloned().collect();
        for a in live {
            match a.kind {
                Kind::Ride => {
                    let at = a.at.unwrap_or(0);
                    match (a.stage, a.step.as_str()) {
                        (Stage::Held, _) if t >= at - 8 * 60_000 => {
                            let busy: Vec<String> = self.trade.of_kind(Kind::Ride).filter(|r| r.stage == Stage::Active).filter_map(|r| r.doer.clone()).collect();
                            let free: Vec<String> = self.world.def.drivers.iter().filter(|d| d.provider == a.maker && !busy.contains(&d.id)).map(|d| d.id.clone()).collect();
                            if !free.is_empty() {
                                let d = free[self.rng.below(free.len() as u64) as usize].clone();
                                let _ = self.advance(&a.id, &a.maker, Stage::Active, None, &format!("doer:{d}"), "", true);
                            } else if t >= at {
                                let _ = self.advance(&a.id, &a.maker, Stage::Closed, Some(Exit::NoDoer), "", "", true);
                            }
                        }
                        (Stage::Active, "assigned") if t >= at => { let _ = self.advance(&a.id, &a.maker, Stage::Active, None, "waiting", "", true); }
                        (Stage::Active, "waiting") if t >= at + 4 * 60_000 => { let _ = self.advance(&a.id, &a.maker, Stage::Active, None, "on_board", "", true); }
                        (Stage::Active, "on_board") if t >= at + 22 * 60_000 => {
                            let _ = self.advance(&a.id, &a.maker, Stage::Fulfilled, None, "", "Dropped off", true);
                            let _ = self.advance(&a.id, &a.maker, Stage::Settled, None, "", "", true);
                        }
                        _ => {}
                    }
                }
                Kind::Deal if a.stage == Stage::Fulfilled && t - a.changed() >= AUTO_CONFIRM_MS => { let _ = self.advance(&a.id, &a.buyer, Stage::Settled, None, "", "", true); }
                _ => {}
            }
        }
    }

    // ---------------- the features, each a use of the one path ----------------

    pub fn buy_tickets(&mut self, fan: &str, event: &str, section: &str, qty: i64) -> Result<Vec<String>, String> {
        let id = self.accept(&format!("tix:{event}:{section}"), fan, qty, None, "", "event").map_err(|e| if e == "Unknown offer" { "Unknown section".into() } else { e })?;
        Ok(self.trade.get(&id).unwrap().detail.split(',').map(String::from).collect())
    }

    pub fn ride_quote(&self, at: i64) -> i64 { crate::services::ride_fare(at, self.script_final()) }

    /// Drivers still free for a pickup at `at` once every ride already promised is counted.
    pub fn drivers_free_at(&self, provider: &str, at: i64) -> i64 {
        let total = self.world.def.drivers.iter().filter(|d| d.provider == provider).count() as i64;
        let taken = self.trade.of_kind(Kind::Ride).filter(|r| r.maker == provider && r.live() && (r.at.unwrap_or(0) - at).abs() < 30 * 60_000).count() as i64;
        total - taken
    }
    pub fn book_ride(&mut self, fan: &str, zone: &str, at: i64) -> Result<String, String> {
        if self.trade.offer(&format!("ride:{zone}")).is_none() { return Err("No rides from there".into()); }
        self.accept(&format!("ride:{zone}"), fan, 1, Some(at), "", "around")
    }
    pub fn cancel_ride(&mut self, fan: &str, id: &str) -> Result<(), String> {
        if self.trade.get(id).map_or(true, |a| a.kind != Kind::Ride || a.buyer != fan) { return Err("Unknown ride".into()); }
        self.advance(id, fan, Stage::Closed, Some(Exit::BuyerCancelled), "", "", false)
    }
    pub fn prepay_parking(&mut self, fan: &str, lot: &str) -> Result<(), String> { self.accept(&format!("park:{lot}"), fan, 1, None, "", "around").map(|_| ()) }
    pub fn unlock(&mut self, fan: &str, place: &str) -> Result<(), String> {
        if self.trade.offer(&format!("perk:{place}")).is_none() { return Err("No perk here".into()); }
        self.accept(&format!("perk:{place}"), fan, 1, None, "", "around").map(|_| ())
    }

    pub fn you_collect(&mut self, edition: &str) -> Result<(), String> { self.accept(&format!("drop:{edition}"), YOU, 1, None, "", "live").map(|_| ()).map_err(|e| if e == "Unknown offer" { "Closed".into() } else { e }) }
    pub fn you_list(&mut self, edition: &str, price: i64) -> Result<String, String> { self.list_card(YOU, edition, price) }
    pub fn you_delist(&mut self, listing: &str) -> Result<(), String> { self.delist_card(YOU, listing) }
    pub fn you_buy(&mut self, listing: &str) -> Result<(), String> { self.accept(listing, YOU, 1, None, "", "market").map(|_| ()) }

    pub fn follow(&mut self, who: &str, node: &str) -> bool {
        let on = self.social.follows.get(who).map_or(false, |s| s.contains(node));
        self.social_act(who, if on { "unfollow" } else { "follow" }, node, "");
        !on
    }
    pub fn you_follow(&mut self, node: &str) -> bool { self.follow(YOU, node) }
    pub fn message(&mut self, from: &str, to: &str, text: &str) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > 500 || from == to { return Err("Say something up to 500 characters".into()); }
        self.social_act(from, "message", to, text);
        Ok(())
    }
    pub fn you_message(&mut self, to: &str, text: &str) -> Result<(), String> { self.message(YOU, to, text) }

    /// Pick an answer to the open question, while picks are open.
    pub fn predict(&mut self, who: &str, pick: &str) -> Result<(), String> {
        let p = self.predictions.last().filter(|p| p.result.is_none()).ok_or("No question is open")?;
        if self.t >= p.locks { return Err("Picks are closed".into()); }
        if !p.options.iter().any(|o| o.0 == pick) { return Err("Pick one of the answers".into()); }
        let ask = p.ask.clone();
        self.social_act(who, "pick", &ask, pick);
        Ok(())
    }
    pub fn you_predict(&mut self, pick: &str) -> Result<(), String> { self.predict(YOU, pick) }

    // ---------------- deals ----------------

    pub fn is_business(&self, id: &str) -> bool { self.world.def.businesses.iter().any(|b| b.id == id) }

    /// What a party offers to do. Fans pay in points; businesses pay money.
    pub fn rates_of(&self, party: &str) -> Vec<Rate> {
        if let Some((_, m)) = self.league.member(party) {
            let s = m.star;
            return vec![
                Rate { kind: "shoutout".into(), title: "Personal shoutout".into(), blurb: "A message from them to you, in your inbox.".into(), pay: Pay::Points, price: if s { 1_200 } else { 500 }, limit: None },
                Rate { kind: "tunnel".into(), title: "Meet at the tunnel".into(), blurb: "Say hello as they walk off after the game.".into(), pay: Pay::Points, price: if s { 4_000 } else { 2_000 }, limit: Some(3) },
                Rate { kind: "promo".into(), title: "Promote your business".into(), blurb: "A post to their followers about you.".into(), pay: Pay::Money, price: if s { 400_00 } else { 120_00 }, limit: Some(4) },
                Rate { kind: "appearance".into(), title: "Post-game appearance".into(), blurb: "Thirty minutes at your place after the game.".into(), pay: Pay::Money, price: if s { 1_500_00 } else { 500_00 }, limit: Some(1) },
            ];
        }
        if self.league.team(party).is_some() {
            return vec![Rate { kind: "timeout".into(), title: "Sponsor a timeout".into(), blurb: "Your name on the scoreboard and in Live for one timeout.".into(), pay: Pay::Money, price: 800_00, limit: Some(4) }];
        }
        vec![]
    }

    pub fn propose_deal(&mut self, from: &str, to: &str, kind: &str, ask: &str, _amount: i64) -> Result<String, String> {
        if self.party_name(to).is_none() { return Err("Unknown profile".into()); }
        let offer = format!("rate:{to}:{kind}");
        if self.trade.offer(&offer).is_none() { return Err(if from == to { "You can't make a deal with yourself".into() } else { "They don't offer that".into() }); }
        let title = self.trade.offer(&offer).unwrap().title.clone();
        let note = if ask.trim().is_empty() { title } else { format!("{title}: {}", ask.trim()) };
        self.accept(&offer, from, 1, None, &note, "profile")
    }
    pub fn respond_deal(&mut self, by: &str, id: &str, yes: bool) -> Result<(), String> {
        if yes { self.advance(id, by, Stage::Active, None, "", "", false) } else { self.advance(id, by, Stage::Closed, Some(Exit::MakerDeclined), "", "", false) }
    }
    pub fn deliver_deal(&mut self, by: &str, id: &str, proof: &str) -> Result<(), String> { self.advance(id, by, Stage::Fulfilled, None, "", proof, false) }
    pub fn confirm_deal(&mut self, by: &str, id: &str) -> Result<(), String> { self.advance(id, by, Stage::Settled, None, "", "", false) }
    pub fn withdraw_deal(&mut self, by: &str, id: &str) -> Result<(), String> { self.advance(id, by, Stage::Closed, Some(Exit::BuyerWithdrew), "", "", false) }

    /// A player claims their profile (in the pilot, the team vouches for them).
    pub fn claim(&mut self, member: &str) -> Result<(), String> {
        if self.league.member(member).is_none() { return Err("Only players can claim a profile".into()); }
        if self.claimed.contains(member) { return Err("Already claimed".into()); }
        self.social_act(member, "claim", member, "");
        Ok(())
    }

    /// A business checks a code at one of its own places (a partner's counter, its lot's gate).
    pub fn redeem_at(&mut self, by: &str, place: &str, code: &str) -> Result<String, String> {
        let owns = place == by || self.world.def.around.iter().any(|a| a.id == place && a.operator.as_deref() == Some(by));
        if !owns { return Err("That isn't your counter".into()); }
        self.redeem(place, code)
    }

    /// The counter enters the code a fan shows. The pass is used once.
    pub fn redeem(&mut self, place: &str, code: &str) -> Result<String, String> {
        let code = code.trim();
        let unit = pass_unit(place);
        let fan = self.ledger.balances_iter().filter(|((a, u), v)| a.purpose == "holdings" && *u == unit && **v > 0)
            .map(|((a, _), _)| a.party.clone()).find(|f| pass_code(f, place) == code);
        let Some(fan) = fan else {
            return Err(if self.redeemed.keys().any(|(f, p)| p == place && pass_code(f, place) == code) { "Already used".into() } else { "No perk or pass matches that code".into() });
        };
        let a = self.trade.all().iter().rev().find(|a| a.buyer == fan && matches!(a.kind, Kind::Perk | Kind::Parking) && self.trade.offer(&a.offer).map_or(false, |o| o.subject == place)).map(|a| a.id.clone()).unwrap_or_default();
        self.ledger.post(Txn { memo: format!("redeem {place} x:{a}"), postings: vec![posting(acct(&fan, "holdings"), unit.clone(), -1), posting(acct(place, "redeemed"), unit, 1)] }).map_err(|e| format!("{e:?}"))?;
        self.social_act(place, "redeem", place, &fan);
        Ok(fan)
    }

    /// Act for this party from now on: the simulation stops answering for them.
    pub fn drive(&mut self, party: &str) { self.driven.insert(party.into()); }

    pub(crate) fn decides(&self, party: &str) -> bool {
        !self.live.contains(party) && !self.driven.contains(party) && (self.league.member(party).map_or(true, |_| self.claimed.contains(party)))
    }

    pub(crate) fn seed_deals(&mut self) {
        let stars: Vec<String> = self.league.teams.iter().filter_map(|g| g.roster.iter().find(|m| m.star).map(|m| m.id.clone())).collect();
        for s in stars.into_iter().take(4) { let _ = self.claim(&s); }
        let home = self.league.tonight.home.clone();
        let away = self.league.tonight.away.clone();
        if let Some(m) = self.league.team(&home).and_then(|g| g.roster.get(1)).map(|m| m.id.clone()) { let _ = self.claim(&m); }
        // fans already follow the players and teams they care about
        let teams: Vec<(String, Vec<(String, bool)>)> = self.league.teams.iter().map(|g| (g.id.clone(), g.roster.iter().map(|m| (m.id.clone(), m.star)).collect())).collect();
        for i in 0..self.cfg.fans {
            let f = self.fans[i].id.clone();
            for (g, roster) in &teams {
                let near = *g == home || *g == away;
                if self.rng.chance(if *g == home { 0.35 } else if near { 0.12 } else { 0.02 }) { self.social_act(&f, "follow", g, ""); }
                for (m, star) in roster {
                    let p = match (near, *star) { (true, true) => 0.18, (true, false) => 0.03, (false, true) => 0.02, _ => 0.002 };
                    if self.rng.chance(p) { self.social_act(&f, "follow", m, ""); }
                }
            }
        }
        // businesses fund a budget for deals with players and teams
        for (b, cents) in [("dock-pizza", 3_000_00), ("copper-tap", 2_000_00), ("harbor-cinema", 1_500_00), ("harborline", 2_500_00), ("pierpark", 1_000_00)] {
            self.ledger.post(Txn { memo: format!("deal budget {b}"), postings: vec![posting(acct(b, "bank"), Unit::usd(), -cents), posting(acct(b, "deal_budget"), Unit::usd(), cents)] }).unwrap();
        }
    }

    /// Simulated parties answer, do what they agreed and release pay; players on Agape answer
    /// messages; the crowd and businesses make offers of their own; fans use perks after the game.
    pub(crate) fn advance_deals(&mut self) {
        let t = self.t;
        let fin = self.script_final();
        let open: Vec<Agreement> = self.trade.of_kind(Kind::Deal).filter(|d| d.live()).cloned().collect();
        for d in open {
            let since = t - d.changed();
            let sub = self.trade.offer(&d.offer).map(|o| o.sub.clone()).unwrap_or_default();
            match d.stage {
                Stage::Held if self.decides(&d.maker) && since >= 3 * 60_000 => {
                    let yes = self.rng.chance(0.85);
                    let _ = self.respond_deal(&d.maker, &d.id, yes);
                }
                Stage::Active if self.decides(&d.maker) => {
                    let after_game = matches!(sub.as_str(), "tunnel" | "appearance");
                    if if after_game { t >= fin + 5 * 60_000 } else { since >= 5 * 60_000 } {
                        let buyer = self.party_name(&d.buyer).unwrap_or_default();
                        let proof = match sub.as_str() {
                            "shoutout" => match d.detail.split_once(": ").map(|x| x.1.trim().to_string()).filter(|x| !x.is_empty()) {
                                Some(n) => format!("{n}: thank you for being in the building tonight. See you next game!"),
                                None => "Thank you for being in the building tonight. See you next game!".to_string() },
                            "tunnel" => "Met at the tunnel after the game.".to_string(),
                            "promo" => format!("Posted about {buyer} to my followers."),
                            "appearance" => format!("Spent thirty minutes at {buyer}."),
                            "timeout" => format!("{buyer} sponsored the timeout on the scoreboard."),
                            _ => "Done, as asked.".to_string(),
                        };
                        let _ = self.deliver_deal(&d.maker, &d.id, &proof);
                    }
                }
                Stage::Fulfilled if !self.live.contains(&d.buyer) && !self.driven.contains(&d.buyer) && since >= 2 * 60_000 => { let _ = self.confirm_deal(&d.buyer, &d.id); }
                _ => {}
            }
        }
        // players on Agape answer messages
        let due: Vec<(i64, String, String, String)> = self.replies.iter().filter(|r| r.0 <= t).cloned().collect();
        self.replies.retain(|r| r.0 > t);
        for (_, from, to, text) in due { if self.decides(&from) { let _ = self.message(&from, &to, &text); } }
        let answered: Vec<(String, String)> = self.replies.iter().map(|r| (r.1.clone(), r.2.clone())).collect();
        let waiting: Vec<(String, String)> = self.social.messages.iter().filter(|m| m.0 >= t - 10_000 && self.claimed.contains(&m.2) && self.league.member(&m.2).is_some())
            .map(|m| (m.1.clone(), m.2.clone())).collect();
        for (from, to) in waiting {
            if answered.contains(&(to.clone(), from.clone())) { continue; }
            const LINES: [&str; 4] = ["Appreciate you! Loud building tonight.", "Thank you. We feel it from the floor.", "Means a lot. See you next home game.", "Thanks for the love!"];
            let line = LINES[self.rng.below(4) as usize].to_string();
            let when = t + (2 + self.rng.below(4) as i64) * 60_000;
            self.replies.push((when, to, from, line));
        }
        let (players_on, around_on) = (self.on("players"), self.on("around"));
        // fans write to the players on the floor tonight
        if players_on && self.rng.chance(0.06) {
            let on_floor: Vec<String> = [&self.league.tonight.home, &self.league.tonight.away].iter().filter_map(|g| self.league.team(g)).flat_map(|g| g.roster.iter().take(5).map(|m| m.id.clone())).collect();
            let to = on_floor[self.rng.below(on_floor.len() as u64) as usize].clone();
            let f = self.fans[self.rng.below(self.cfg.fans as u64) as usize].id.clone();
            const SAY: [&str; 6] = ["Let's go!! Section 2 is loud for you", "That last bucket was nasty", "Been a fan since your rookie year", "My son wears your jersey every game", "Need a big second half from you", "Sign my card after?"];
            let line = SAY[self.rng.below(6) as usize];
            if !self.live.contains(&f) { let _ = self.message(&f, &to, line); }
        }
        // the crowd asks players for shoutouts; businesses sponsor every twelve minutes
        if players_on && self.rng.chance(0.06) && !self.claimed.is_empty() {
            let who: Vec<String> = self.claimed.iter().cloned().collect();
            let to = who[self.rng.below(who.len() as u64) as usize].clone();
            let f = self.fans[self.rng.below(self.cfg.fans as u64) as usize].id.clone();
            if !self.live.contains(&f) { let _ = self.propose_deal(&f, &to, "shoutout", "", 0); }
        }
        if players_on && !self.claimed.is_empty() && t % (12 * 60_000) == 0 && t > 0 && t < fin {
            let bizs = ["dock-pizza", "copper-tap", "harbor-cinema", "harborline"];
            let b = bizs[self.rng.below(4) as usize];
            if !self.driven.contains(b) {
                let home = self.league.tonight.home.clone();
                let (to, kind) = if self.rng.chance(0.3) { (home, "timeout".to_string()) } else {
                    let who: Vec<String> = self.claimed.iter().cloned().collect();
                    (who[self.rng.below(who.len() as u64) as usize].clone(), if self.rng.chance(0.8) { "promo".into() } else { "appearance".into() })
                };
                let _ = self.propose_deal(b, &to, &kind, "", 0);
            }
        }
        // the crowd unlocks partner perks, and uses them after the game
        if around_on && t < fin && self.rng.chance(0.04) {
            let places: Vec<String> = self.world.def.around.iter().filter(|a| a.perk.is_some()).map(|a| a.id.clone()).collect();
            let p = places[self.rng.below(places.len() as u64) as usize].clone();
            let f = self.fans[self.rng.below(self.cfg.fans as u64) as usize].id.clone();
            if !self.live.contains(&f) { let _ = self.unlock(&f, &p); }
        }
        if around_on && t > fin && self.rng.chance(0.2) {
            let held: Vec<(String, String)> = self.ledger.balances_iter().filter(|((a, u), v)| a.purpose == "holdings" && !self.live.contains(&a.party) && **v > 0 && matches!(u, Unit::Thing(s) if s.starts_with("pass:")))
                .filter_map(|((a, u), _)| match u { Unit::Thing(s) => Some((a.party.clone(), s[5..].to_string())), _ => None })
                .filter(|(_, p)| self.world.def.around.iter().any(|a| a.id == *p && a.perk.is_some()) && !self.driven.contains(p)).collect();
            if let Some((f, p)) = held.first().cloned() { let _ = self.redeem(&p, &pass_code(&f, &p)); }
        }
    }
}

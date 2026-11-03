//! Offers, prices and settlement. Every discount names a payer (A7); every offer passes the
//! ex-ante Agape check against each party's declared value function (A3) before it is shown.

use crate::value::{acct, bps, posting, PartyId, Posting, Txn, Unit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Venue-level economics, all exact: cents and basis points.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Terms {
    pub currency: String,
    pub platform_share_bps: i64,
    pub tax_bps: i64,
    pub delivery_fee: i64,
    pub proc_bps: i64,
    pub proc_fixed: i64,
    /// Paid by Agape to the runner per delivery, on top of tips.
    pub runner_pay: i64,
    /// Points the fan earns per dollar of subtotal, issued by Agape.
    pub points_per_dollar: i64,
}

impl Default for Terms {
    fn default() -> Self {
        Terms { currency: "USD".into(), platform_share_bps: 1500, tax_bps: 700, delivery_fee: 499, proc_bps: 290, proc_fixed: 30, runner_pay: 150, points_per_dollar: 10 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line { pub thing: String, pub qty: i64, pub unit_price: i64 }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discount { pub amount: i64, pub payer: PartyId, pub reason: String }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    pub fan: PartyId,
    pub stand: PartyId,
    pub runner: Option<PartyId>,
    pub lines: Vec<Line>,
    pub discounts: Vec<Discount>,
    /// Points the fan spends; Agape funds their cash value as a discount it pays.
    pub points_redeemed: i64,
    pub tip: i64,
    pub delivery: bool,
    /// Part of a cart whose delivery fee another order in it already carries.
    #[serde(default)]
    pub fee_waived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Breakdown {
    pub subtotal: i64,
    pub discount: i64,
    pub tax: i64,
    pub fee: i64,
    pub tip: i64,
    /// What the fan's card is charged.
    pub charge: i64,
    pub processing: i64,
    pub stand_net: i64,
    pub agape_share: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OfferError {
    DiscountExceedsSubtotal,
    Negative,
    /// The Agape invariant: this party would lose against its declared value function.
    Loses { party: PartyId, net: i64, floor: i64 },
}

/// 1 point = 1 cent of discount, funded by Agape.
pub const POINT_VALUE_CENTS: i64 = 1;

pub fn breakdown(q: &Quote, t: &Terms) -> Result<Breakdown, OfferError> {
    if q.tip < 0 || q.points_redeemed < 0 || q.lines.iter().any(|l| l.qty <= 0 || l.unit_price < 0) || q.discounts.iter().any(|d| d.amount < 0) {
        return Err(OfferError::Negative);
    }
    let subtotal: i64 = q.lines.iter().map(|l| l.qty * l.unit_price).sum();
    let discount: i64 = q.discounts.iter().map(|d| d.amount).sum::<i64>() + q.points_redeemed * POINT_VALUE_CENTS;
    if discount > subtotal { return Err(OfferError::DiscountExceedsSubtotal); }
    // Tax on the price the stand sells at; whoever funds a discount, the sale price is the subtotal.
    let tax = bps(subtotal, t.tax_bps);
    let fee = if q.delivery && !q.fee_waived { t.delivery_fee } else { 0 };
    let charge = subtotal - discount + tax + fee + q.tip;
    let processing = if charge > 0 { bps(charge, t.proc_bps) + t.proc_fixed } else { 0 };
    let agape_share = bps(subtotal, t.platform_share_bps);
    Ok(Breakdown { subtotal, discount, tax, fee, tip: q.tip, charge, processing, stand_net: subtotal - agape_share, agape_share })
}

/// The settlement transaction for a delivered (or picked-up) order. Balanced in every unit.
pub fn settlement(q: &Quote, t: &Terms, b: &Breakdown) -> Txn {
    let usd = Unit::Money(t.currency.clone());
    let mut p: Vec<Posting> = vec![
        posting(acct(&q.fan, "card"), usd.clone(), -b.charge),
        posting(acct(&q.stand, "revenue"), usd.clone(), b.stand_net),
        posting(acct("tax_authority", "payable"), usd.clone(), b.tax),
        posting(acct("processor", "fees"), usd.clone(), b.processing),
    ];
    let mut agape = b.agape_share + b.fee - b.processing;
    for d in &q.discounts {
        p.push(posting(acct(&d.payer, "promo_escrow"), usd.clone(), -d.amount));
    }
    if q.points_redeemed > 0 {
        agape -= q.points_redeemed * POINT_VALUE_CENTS;
        let pts = Unit::Points("agape".into());
        p.push(posting(acct(&q.fan, "points_held"), pts.clone(), -q.points_redeemed));
        p.push(posting(acct("agape", "issued"), pts, q.points_redeemed));
    }
    if let Some(r) = &q.runner {
        if b.tip > 0 { p.push(posting(acct(r, "tips"), usd.clone(), b.tip)); }
        if q.delivery { p.push(posting(acct(r, "pay"), usd.clone(), t.runner_pay)); agape -= t.runner_pay; }
    } else if b.tip > 0 {
        p.push(posting(acct(&q.stand, "tips"), usd.clone(), b.tip));
    }
    p.push(posting(acct("agape", "revenue"), usd.clone(), agape));
    // the things themselves move from the stand to the fan
    for l in &q.lines {
        p.push(posting(acct(&q.stand, "reserved"), Unit::Thing(l.thing.clone()), -l.qty));
        p.push(posting(acct(&q.fan, "holdings"), Unit::Thing(l.thing.clone()), l.qty));
    }
    // points earned
    let earned = (b.subtotal - b.discount) / 100 * t.points_per_dollar;
    if earned > 0 {
        let pts = Unit::Points("agape".into());
        p.push(posting(acct("agape", "issued"), pts.clone(), -earned));
        p.push(posting(acct(&q.fan, "points"), pts, earned));
    }
    Txn { memo: "settle".into(), postings: p }
}

/// A declared value function: the least a party will accept from this exchange, in money.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ValueFn {
    /// Net money from this exchange must be at least this (negative = willing to pay up to).
    Floor(i64),
}

/// Ex-ante Agape check (A3): compute everyone's net from the settlement this offer would
/// produce and refuse the offer if any party with a declared value function would lose.
pub fn agape_check(q: &Quote, t: &Terms, values: &BTreeMap<PartyId, ValueFn>) -> Result<Breakdown, OfferError> {
    let b = breakdown(q, t)?;
    let txn = settlement(q, t, &b);
    let usd = Unit::Money(t.currency.clone());
    let mut net: BTreeMap<&str, i64> = BTreeMap::new();
    for p in txn.postings.iter().filter(|p| p.unit == usd) {
        *net.entry(p.account.party.as_str()).or_insert(0) += p.amount;
    }
    for (party, ValueFn::Floor(floor)) in values {
        let n = *net.get(party.as_str()).unwrap_or(&0);
        if n < *floor { return Err(OfferError::Loses { party: party.clone(), net: n, floor: *floor }); }
    }
    Ok(b)
}

/// What acceptance sets aside, so an accepted order can always settle: each discount from its
/// payer's budget, the fan's redeemed points, and the stand's units. Released on exits that do not capture.
pub fn hold(q: &Quote) -> Txn { hold_from(q, "inventory", "promo_budget") }

/// The same hold, taking units and budget from the given purposes (a guaranteed offer takes
/// them from what it already promised, rather than from open stock and budget).
pub fn hold_from(q: &Quote, inventory: &str, budget: &str) -> Txn {
    let mut p = vec![];
    for d in q.discounts.iter().filter(|d| d.amount > 0) {
        p.push(posting(acct(&d.payer, budget), Unit::usd(), -d.amount));
        p.push(posting(acct(&d.payer, "promo_escrow"), Unit::usd(), d.amount));
    }
    if q.points_redeemed > 0 {
        let pts = Unit::Points("agape".into());
        p.push(posting(acct(&q.fan, "points"), pts.clone(), -q.points_redeemed));
        p.push(posting(acct(&q.fan, "points_held"), pts, q.points_redeemed));
    }
    for l in &q.lines {
        p.push(posting(acct(&q.stand, inventory), Unit::Thing(l.thing.clone()), -l.qty));
        p.push(posting(acct(&q.stand, "reserved"), Unit::Thing(l.thing.clone()), l.qty));
    }
    Txn { memo: "hold".into(), postings: p }
}

pub fn release(q: &Quote) -> Txn { release_to(q, "inventory", "promo_budget") }

pub fn release_to(q: &Quote, inventory: &str, budget: &str) -> Txn {
    let mut t = hold_from(q, inventory, budget);
    for p in &mut t.postings { p.amount = -p.amount; }
    t.memo = "release".into();
    t
}

/// Agape's service fee per ticket.
pub const TICKET_FEE: i64 = 450;

pub fn ticket_unit(event: &str, tier: &str) -> Unit { Unit::Thing(format!("ticket:{event}:{tier}")) }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketBreakdown { pub subtotal: i64, pub fee: i64, pub tax: i64, pub charge: i64, pub processing: i64, pub venue_net: i64, pub agape_share: i64, pub points: i64 }

pub fn ticket_breakdown(unit_price: i64, qty: i64, t: &Terms) -> TicketBreakdown {
    let subtotal = unit_price * qty;
    let fee = TICKET_FEE * qty;
    let tax = bps(subtotal, t.tax_bps);
    let charge = subtotal + fee + tax;
    let processing = bps(charge, t.proc_bps) + t.proc_fixed;
    let agape_share = bps(subtotal, t.platform_share_bps);
    TicketBreakdown { subtotal, fee, tax, charge, processing, venue_net: subtotal - agape_share, agape_share, points: subtotal / 100 * t.points_per_dollar }
}

/// Tickets move from the venue's inventory to the fan; money and points move in the same transaction.
pub fn ticket_sale(fan: &str, venue: &str, event: &str, tier: &str, qty: i64, b: &TicketBreakdown, t: &Terms) -> Txn {
    let usd = Unit::Money(t.currency.clone());
    let pts = Unit::Points("agape".into());
    let unit = ticket_unit(event, tier);
    let mut p = vec![
        posting(acct(fan, "card"), usd.clone(), -b.charge),
        posting(acct(venue, "revenue"), usd.clone(), b.venue_net),
        posting(acct("tax_authority", "payable"), usd.clone(), b.tax),
        posting(acct("processor", "fees"), usd.clone(), b.processing),
        posting(acct("agape", "revenue"), usd, b.agape_share + b.fee - b.processing),
        posting(acct(venue, "tickets"), unit.clone(), -qty),
        posting(acct(fan, "holdings"), unit, qty),
    ];
    if b.points > 0 { p.push(posting(acct("agape", "issued"), pts.clone(), -b.points)); p.push(posting(acct(fan, "points"), pts, b.points)); }
    Txn { memo: format!("tickets {event}"), postings: p }
}

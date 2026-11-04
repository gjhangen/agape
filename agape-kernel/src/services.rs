//! Layer 2: seats and rides. A ticket is a specific seat; a ride is a commitment between a fan,
//! a company and a driver. Both settle through the ledger like everything else.

use crate::economy::Terms;
use crate::value::{acct, bps, posting, PartyId, Txn, Unit};
use serde::Serialize;

/// One section's seats for one event: rows front to back, seats left to right.
#[derive(Clone, Debug)]
pub struct SeatMap { pub rows: usize, pub per_row: usize, pub owner: Vec<Option<PartyId>> }

pub const ROWS: &str = "ABCDEFGHJKLMNPQRSTUV";

impl SeatMap {
    pub fn new(rows: usize, per_row: usize) -> SeatMap { SeatMap { rows, per_row, owner: vec![None; rows * per_row] } }
    pub fn free(&self) -> usize { self.owner.iter().filter(|o| o.is_none()).count() }
    /// The front-most block of `qty` seats side by side, as (row index, first seat index).
    pub fn best(&self, qty: usize) -> Option<(usize, usize)> {
        for r in 0..self.rows {
            let row = &self.owner[r * self.per_row..(r + 1) * self.per_row];
            let mut run = 0;
            for (i, o) in row.iter().enumerate() {
                run = if o.is_none() { run + 1 } else { 0 };
                if run == qty { return Some((r, i + 1 - qty)); }
            }
        }
        None
    }
    pub fn take(&mut self, r: usize, s: usize, qty: usize, who: &str) {
        for i in 0..qty { self.owner[r * self.per_row + s + i] = Some(who.into()); }
    }
    pub fn label(r: usize) -> char { ROWS.chars().nth(r).unwrap_or('?') }
    /// "F7" back to its position on the map.
    pub fn index_of(&self, seat: &str) -> Option<usize> {
        let mut c = seat.chars();
        let r = ROWS.find(c.next()?)?;
        let n: usize = c.as_str().parse().ok()?;
        if r < self.rows && n >= 1 && n <= self.per_row { Some(r * self.per_row + n - 1) } else { None }
    }
}

/// Where a section sits relative to the floor decides its price.
pub fn section_view(ring: u32, sections: u32) -> &'static str {
    let q = sections / 4;
    let d = |c: u32| { let x = (ring as i64 - c as i64).rem_euclid(sections as i64); x.min(sections as i64 - x) as u32 };
    if d(0) <= 1 || d(2 * q) <= 1 { "Sideline" } else if d(q) <= 1 || d(3 * q) <= 1 { "Baseline" } else { "Corner" }
}

pub fn section_price(base: i64, view: &str) -> i64 {
    (match view { "Sideline" => base * 14 / 10, "Baseline" => base * 85 / 100, _ => base }) / 100 * 100
}

// ---------- rides ----------

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum RideStage { Booked, DriverAssigned, Waiting, OnBoard, Done, Cancelled }

#[derive(Clone, Debug, Serialize)]
pub struct Ride {
    pub id: String, pub fan: PartyId, pub provider: PartyId, pub driver: Option<PartyId>,
    pub zone: String, pub at: i64, pub fare: i64, pub stage: RideStage,
}

/// The driver's share and the company's share of a fare, in basis points; Agape keeps a referral share.
pub const DRIVER_BPS: i64 = 7_800;
pub const PROVIDER_BPS: i64 = 1_700;

/// A transparent fare: a base, plus more in the rush right after the final.
pub fn ride_fare(at: i64, final_t: i64) -> i64 {
    let base = 1_890;
    if at >= final_t && at <= final_t + 20 * 60_000 { base * 125 / 100 } else { base }
}

pub fn ride_settlement(r: &Ride, t: &Terms) -> Txn {
    let usd = Unit::Money(t.currency.clone());
    let driver = bps(r.fare, DRIVER_BPS);
    let provider = bps(r.fare, PROVIDER_BPS);
    let agape = r.fare - driver - provider;
    // the ride company is the seller and pays processing out of its part
    let (mut p, _) = crate::economy::card_postings(&r.fan, acct(&r.provider, "revenue"), r.fare, provider, t);
    p.push(posting(acct(r.driver.as_deref().unwrap_or("unassigned"), "earnings"), usd.clone(), driver));
    p.push(posting(acct("agape", "revenue"), usd, agape));
    Txn { memo: format!("ride {}", r.id), postings: p }
}

/// Prepaid parking: the operator gets the fee less Agape's share.
pub fn parking_sale(fan: &str, operator: &str, lot: &str, price: i64, t: &Terms) -> Txn {
    let usd = Unit::Money(t.currency.clone());
    let share = bps(price, 1_500);
    let (mut p, _) = crate::economy::card_postings(fan, acct(operator, "revenue"), price, price - share, t);
    p.extend([
        posting(acct("agape", "revenue"), usd, share),
        posting(acct(lot, "passes"), crate::sim::pass_unit(lot), -1),
        posting(acct(fan, "holdings"), crate::sim::pass_unit(lot), 1),
    ]);
    Txn { memo: format!("parking {lot}"), postings: p }
}

//! Layer 2: the social economy. Moment cards are things minted from real signals with a fixed
//! supply; fans collect them while they last and trade them for points. Points move between
//! fans only inside Agape (closed loop), through the same ledger, with a small fee that returns
//! to the issuer. Level is what Agape has ever issued to you for using it; Balance is what you
//! can spend. Follows and messages are relations and signals between parties.

use crate::value::{acct, posting, Ledger, LedgerError, PartyId, Txn, Unit};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ISSUER: &str = "agape";
/// Basis points of each card sale returned to the issuer, so points don't pile up forever.
pub const MARKET_FEE_BPS: i64 = 500;

pub fn points() -> Unit { Unit::Points(ISSUER.into()) }
pub fn card_unit(id: &str) -> Unit { Unit::Thing(format!("card:{id}")) }

#[derive(Clone, Debug, Serialize)]
pub struct Edition {
    pub id: String,
    /// What happened, by the template's word for it ("run", "splash", "lead_change", "season").
    pub kind: String,
    pub title: String,
    pub team: String,
    pub party: Option<PartyId>,
    pub t: i64,
    pub supply: i64,
    pub rarity: String,
    /// Collecting closes at this time (or when the supply is gone).
    pub open_until: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Listing { pub id: String, pub edition: String, pub seller: PartyId, pub price: i64, pub t: i64 }

#[derive(Debug, PartialEq, Eq)]
pub enum SocialError { Closed, AlreadyCollected, NotHeld, Unknown, CantAfford, OwnListing, Ledger(String) }

impl From<LedgerError> for SocialError { fn from(e: LedgerError) -> Self { SocialError::Ledger(format!("{e:?}")) } }

/// Something posted for everyone here: by a fan, a player or a team.
#[derive(Clone, Debug)]
pub struct Post { pub id: String, pub t: i64, pub author: PartyId, /** fan, player, team, act, venue, or removed */ pub kind: String, pub text: String,
    pub likes: BTreeSet<PartyId>, pub comments: Vec<(i64, PartyId, String)>,
    /** the name shown, when the author is an act or the venue and has no page of its own */ pub shown_as: Option<String> }

/// What a person writes about themselves.
#[derive(Clone, Debug, Default)]
pub struct Profile { pub bio: String, /** a league team id, or empty */ pub team: String }

#[derive(Default)]
pub struct Social {
    pub editions: BTreeMap<String, Edition>,
    pub follows: BTreeMap<PartyId, BTreeSet<String>>,
    pub messages: Vec<(i64, PartyId, PartyId, String)>,
    pub posts: Vec<Post>,
    pub profiles: BTreeMap<PartyId, Profile>,
}

impl Social {
    /// Mint an edition: its whole supply moves from the world into the issuer's drop account.
    pub fn mint(&mut self, ledger: &mut Ledger, e: Edition) -> Result<String, SocialError> {
        ledger.post(Txn { memo: format!("mint {}", e.id), postings: vec![
            posting(acct("world", "supply"), card_unit(&e.id), -e.supply),
            posting(acct(ISSUER, "drops"), card_unit(&e.id), e.supply),
        ]})?;
        let id = e.id.clone();
        self.editions.insert(id.clone(), e);
        Ok(id)
    }

    pub fn left(&self, ledger: &Ledger, edition: &str) -> i64 { ledger.balance(&acct(ISSUER, "drops"), &card_unit(edition)) }

    /// Give a card from the issuer outright (season history, rewards).
    pub fn grant(&mut self, ledger: &mut Ledger, fan: &str, edition: &str) -> Result<(), SocialError> {
        ledger.post(Txn { memo: format!("grant {edition}"), postings: vec![
            posting(acct(ISSUER, "drops"), card_unit(edition), -1),
            posting(acct(fan, "holdings"), card_unit(edition), 1),
        ]})?;
        Ok(())
    }

    pub fn followers(&self, node: &str) -> usize { self.follows.values().filter(|s| s.contains(node)).count() }


}

/// Everything Agape itself has ever issued to this fan: their level. Points received from other
/// fans are spendable but don't raise level, so level can only come from using the app.
pub fn lifetime(ledger: &Ledger, fan: &str) -> i64 {
    let mine = acct(fan, "points");
    let issuer = acct(ISSUER, "issued");
    ledger.txns.iter().filter(|t| t.postings.iter().any(|p| p.account == issuer && p.amount < 0))
        .flat_map(|t| t.postings.iter()).filter(|p| p.account == mine && p.amount > 0 && p.unit == points()).map(|p| p.amount).sum()
}

pub const TIERS: [(i64, &str); 5] = [(0, "Rookie"), (500, "Starter"), (1500, "All-Star"), (4000, "MVP"), (10000, "Legend")];

/// (tier name, tier floor, next tier floor if any)
pub fn tier(level: i64) -> (&'static str, i64, Option<(&'static str, i64)>) {
    let i = TIERS.iter().rposition(|(f, _)| level >= *f).unwrap_or(0);
    (TIERS[i].1, TIERS[i].0, TIERS.get(i + 1).map(|(f, n)| (*n, *f)))
}

/// A short public name for a fan, stable for their id.
pub fn handle(id: &str) -> String {
    const A: [&str; 16] = ["Courtside", "Baseline", "Rafter", "Upper", "Loud", "Corner", "Front", "Fast", "Late", "Brick", "Golden", "Harbor", "Night", "Steady", "Lucky", "Prime"];
    const B: [&str; 16] = ["Cal", "Bea", "Mo", "Jo", "Ty", "Ren", "Ash", "Lou", "Max", "Kit", "Dee", "Sol", "Ari", "Bo", "Gus", "Ivy"];
    let n: u64 = id.bytes().fold(1469598103934665603u64, |h, b| (h ^ b as u64).wrapping_mul(1099511628211));
    format!("{}{}{}", A[(n % 16) as usize], B[((n >> 8) % 16) as usize], (n >> 16) % 90 + 10)
}

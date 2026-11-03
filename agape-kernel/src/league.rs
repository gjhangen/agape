//! Layer 1: the people and groups an event belongs to, loaded from a template. For a sports
//! league these are teams and their rosters; for a tour they'd be acts and their members. Each
//! is a party with a page; stats on those pages are folds over the signals they took part in.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member { pub id: String, pub name: String, pub number: i64, pub pos: String, pub star: bool, pub ppg: f64, pub rpg: f64, pub apg: f64 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group { pub id: String, pub name: String, pub short: String, pub abbr: String, pub color: String, pub wins: i64, pub losses: i64, pub roster: Vec<Member> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tonight { pub home: String, pub away: String, pub others: Vec<(String, String)> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct League { pub name: String, pub season: String, pub teams: Vec<Group>, pub tonight: Tonight,
    #[serde(default)] pub upcoming: Vec<Upcoming>, #[serde(default)] pub tiers: Vec<Tier> }

/// A future event at the venue: a game, a concert, anything with tickets.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Upcoming { pub id: String, pub kind: String, pub home: Option<String>, pub away: Option<String>, pub title: String, pub date: String, pub time: String }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tier { pub id: String, pub name: String, pub price: i64, pub stock: i64 }

impl League {
    pub fn team(&self, id: &str) -> Option<&Group> { self.teams.iter().find(|t| t.id == id) }
    pub fn member(&self, id: &str) -> Option<(&Group, &Member)> {
        self.teams.iter().find_map(|t| t.roster.iter().find(|m| m.id == id).map(|m| (t, m)))
    }
}

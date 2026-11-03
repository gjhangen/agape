//! Layer 1: the world model. A venue is built entirely from a JSON template (onboarding is
//! filling this in): its place graph, stands, things, terms and service policy.

use crate::economy::Terms;
use crate::place::Graph;
use crate::value::PartyId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StandDef { pub id: String, pub name: String, pub level: u32, pub ring: u32, pub stations: u32 }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Constraint {
    /// Requires a verified age at or above this.
    MinAge(u8),
    /// Only deliverable to these levels.
    Levels(Vec<u32>),
    /// Not offered while this span is open (last call, for example).
    NotDuring(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThingDef {
    pub id: String,
    pub name: String,
    pub stand: String,
    pub price: i64,
    pub prep_s: u32,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    #[serde(default)]
    pub stock: Option<i64>,
    /// What kind of thing it is, for browsing by craving: food, drink, merch.
    #[serde(default)]
    pub category: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Service { pub batch_max: usize, pub max_eta_s: u32, pub handoff_s: u32 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldDef {
    pub venue: String,
    /// The venue's name as people know it.
    #[serde(default)]
    pub name: String,
    pub levels: u32,
    pub sections_per_level: u32,
    /// Walk seconds between neighbouring concourse points.
    pub ring_s: u32,
    pub stairs_s: u32,
    pub lift_s: u32,
    pub aisle_s: u32,
    pub stands: Vec<StandDef>,
    pub things: Vec<ThingDef>,
    pub terms: Terms,
    pub service: Service,
    /// Places beyond the venue: rides, parking, partner businesses. The outermost ring of Get.
    #[serde(default)]
    pub around: Vec<AroundPlace>,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub capacity: i64,
    /// Every business in and around the venue that isn't a stand: rideshare, parking, partners.
    #[serde(default)]
    pub businesses: Vec<Business>,
    /// People who work for those businesses and have their own profiles (drivers).
    #[serde(default)]
    pub drivers: Vec<Driver>,
    /// The venue's own teams, colors and players, laid over the league (a venue trying Agape as itself).
    #[serde(default)]
    pub brand: Option<Brand>,
    /// What the venue switches on for its fans. Anything not named is on.
    #[serde(default)]
    pub features: std::collections::BTreeMap<String, bool>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Brand { #[serde(default)] pub home: Option<TeamBrand>, #[serde(default)] pub away: Option<TeamBrand> }

/// A team as the venue presents it. Empty fields keep the league's.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TeamBrand {
    #[serde(default)] pub name: String, #[serde(default)] pub short: String, #[serde(default)] pub abbr: String, #[serde(default)] pub color: String,
    /// A picture of the logo (a data URL), shown by the apps; the kernel only carries it.
    #[serde(default)] pub logo: String,
    /// Players in roster order: name and number. Fewer than the roster keeps the rest.
    #[serde(default)] pub players: Vec<PlayerBrand>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlayerBrand { #[serde(default)] pub name: String, #[serde(default)] pub number: Option<i64> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Business { pub id: String, pub kind: String, pub name: String, pub blurb: String, #[serde(default)] pub hours: String }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Driver { pub id: String, pub name: String, pub provider: String, pub rating: f64, pub trips: i64, pub vehicle: String, pub plate: String }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AroundService { pub kind: String, #[serde(default)] pub price: i64, #[serde(default)] pub supply: i64 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AroundPlace { pub id: String, pub kind: String, pub name: String, pub walk_min: i64, pub blurb: String, #[serde(default)] pub perk: Option<Perk>,
    /// The business that runs it.
    #[serde(default)] pub operator: Option<String>,
    #[serde(default)] pub service: Option<AroundService> }

/// Something a partner offers fans, unlocked with points. Points spent here go back to the issuer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Perk { pub title: String, pub cost: i64, pub supply: i64 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fan {
    pub id: PartyId,
    /// Section place id, e.g. "s104".
    pub seat: String,
    pub age_verified: Option<u8>,
    pub step_free: bool,
    pub consents: BTreeSet<String>,
    /// Most this fan has said they will spend on one order (their declared value function).
    pub ceiling: i64,
}

pub struct World { pub def: WorldDef, pub graph: Graph }

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Refusal { UnknownThing, SoldOut, BudgetSpent, StandBusy, AgeUnverified, WrongLevel, SpanClosed(String), PlaceClosed, NoConsent, Loses(PartyId), OverCeiling }

pub fn section_id(level: u32, ring: u32) -> String { format!("s{}", level * 100 + ring + 1) }
pub fn concourse_id(level: u32, ring: u32) -> String { format!("c{level}-{ring}") }

impl World {
    pub fn build(def: WorldDef) -> World {
        let mut g = Graph::default();
        g.add(&def.venue, "venue", None);
        let n = def.sections_per_level;
        for l in 1..=def.levels {
            let lid = format!("L{l}");
            g.add(&lid, "level", Some(&def.venue));
            for r in 0..n {
                g.add(&concourse_id(l, r), "concourse", Some(&lid));
                g.add(&section_id(l, r), "section", Some(&lid));
                g.link(&concourse_id(l, r), &section_id(l, r), def.aisle_s, true);
            }
            for r in 0..n { g.link(&concourse_id(l, r), &concourse_id(l, (r + 1) % n), def.ring_s, true); }
        }
        for l in 1..def.levels {
            for r in (0..n).step_by((n / 4).max(1) as usize) {
                let id = format!("stairs{l}-{r}");
                g.add(&id, "stairs", Some(&def.venue));
                g.link(&concourse_id(l, r), &id, def.stairs_s / 2, false);
                g.link(&id, &concourse_id(l + 1, r), def.stairs_s / 2, false);
            }
            let r = n / 8;
            let id = format!("lift{l}");
            g.add(&id, "lift", Some(&def.venue));
            g.link(&concourse_id(l, r), &id, def.lift_s / 2, true);
            g.link(&id, &concourse_id(l + 1, r), def.lift_s / 2, true);
        }
        for s in &def.stands {
            g.add(&s.id, "stand", Some(&format!("L{}", s.level)));
            g.link(&s.id, &concourse_id(s.level, s.ring), 10, true);
        }
        World { def, graph: g }
    }

    pub fn thing(&self, id: &str) -> Option<&ThingDef> { self.def.things.iter().find(|t| t.id == id) }

    pub fn level_of(&self, place: &str) -> Option<u32> {
        (1..=self.def.levels).find(|l| self.graph.within(place, &format!("L{l}")))
    }

    /// Invariant 6, eligibility before exposure. `pushed` offers (from rules) also need consent.
    pub fn eligible(&self, fan: &Fan, thing: &str, open_span: &dyn Fn(&str) -> bool, pushed: bool) -> Result<(), Refusal> {
        let t = self.thing(thing).ok_or(Refusal::UnknownThing)?;
        if pushed && !fan.consents.contains("offers") { return Err(Refusal::NoConsent); }
        if !self.graph.places.get(&t.stand).map_or(false, |p| p.open) { return Err(Refusal::PlaceClosed); }
        for c in &t.constraints {
            match c {
                Constraint::MinAge(a) => if fan.age_verified.map_or(true, |v| v < *a) { return Err(Refusal::AgeUnverified); },
                Constraint::Levels(ls) => if !self.level_of(&fan.seat).map_or(false, |l| ls.contains(&l)) { return Err(Refusal::WrongLevel); },
                Constraint::NotDuring(s) => if open_span(s) { return Err(Refusal::SpanClosed(s.clone())); },
            }
        }
        Ok(())
    }
}

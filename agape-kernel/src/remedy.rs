//! Making things right when an order goes wrong, by a policy Agape's team sets.
//!
//! A remedy returns any part of what the fan paid (the items, their tax, the delivery fee, the
//! tip), each part taken from whichever party the policy names (the stand, the runner, Agape, or
//! the tax that was collected on the sale), as Agape credit (the default) or back to the card.
//! Every remedy is one balanced transaction. Agape is the delivery service: what a stand makes
//! is the stand's to make right, and Agape only carries what went wrong in getting it there.
//!
//! Behind the scenes Agape also keeps each fan's record (claims, what they have cost, ratings
//! they give) and can ask for more proof at the handoff for fans whose record calls for it.
//! None of that is shown to fans or runners.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What can be returned: what the fan paid for the items (after any deal), their tax, the
/// delivery fee and the tip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Part { Items, Tax, Fee, Tip }

/// Who a part is taken from. `Fault` is whoever the case is decided against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source { Fault, Stand, Runner, Agape, Tax }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Form { Credit, Refund }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cover { pub part: Part, pub from: Source, /** Share of the part returned, in basis points. */ pub bps: i64 }

/// How one kind of problem is made right.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// In the fan's words.
    pub words: String,
    /// Who it's decided against unless a person decides otherwise: "stand", "runner" or "agape".
    pub fault: String,
    pub covers: Vec<Cover>,
    /// The fan must add a photo to report it.
    pub photo: bool,
    /// Settled without a person looking, within the limits below.
    pub auto: bool,
    /// Credit or refund for this kind; unset means the policy's default.
    #[serde(default)]
    pub form: Option<Form>,
}

/// Proof asked for at the handoff, by fan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Proof { None, Photo, Pin }

/// Agape's policy for problems, tips and proof. Set by Agape's team; the same for every venue unless changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    /// Credit unless a rule says otherwise.
    pub form: Form,
    pub rules: BTreeMap<String, Rule>,
    /// Claims settled automatically per fan per night, and per stand per night; past these a person decides.
    pub auto_per_fan: usize,
    pub auto_per_stand: usize,
    /// A delivery marked handed over this far (metres) or more from the seat counts as evidence it never arrived.
    pub handoff_radius_m: i64,
    /// The tip the fan sees first: this share of the items (basis points), or `tip_min`, whichever is more.
    pub tip_bps: i64,
    pub tip_min: i64,
    /// Proof at the handoff is asked for automatically once a fan has this many orders and their
    /// claims reach these rates (basis points of their orders).
    pub proof_after_orders: usize,
    pub photo_at_bps: i64,
    pub pin_at_bps: i64,
    /// Runners' shifts are the event: they go online and off when they like during it. A fixed
    /// shift keeps them online for all of it.
    #[serde(default)]
    pub fixed_shift: bool,
}

fn c(part: Part, from: Source) -> Cover { Cover { part, from, bps: 10_000 } }
fn rule(words: &str, fault: &str, covers: Vec<Cover>, photo: bool, auto: bool) -> Rule {
    Rule { words: words.into(), fault: fault.into(), covers, photo, auto, form: None }
}

impl Default for Policy {
    fn default() -> Self {
        use Source::{Fault, Runner, Stand}; use Part::{Fee, Items, Tip};
        let mut rules = BTreeMap::new();
        // nothing reached the fan: everything back, the tip from the runner who didn't deliver it
        rules.insert("never_arrived".into(), rule("It never arrived", "agape", vec![c(Items, Fault), c(Part::Tax, Source::Tax), c(Fee, Fault), c(Tip, Runner)], false, true));
        // ruined on the way: Agape stands behind its delivery; the runner keeps the tip
        rules.insert("damaged".into(), rule("Spilled or ruined on the way", "agape", vec![c(Items, Fault), c(Part::Tax, Source::Tax), c(Fee, Fault)], true, true));
        // the wrong thing, or something missing: whoever is found at fault, the stand unless shown otherwise
        rules.insert("wrong_item".into(), rule("Wrong item", "stand", vec![c(Items, Fault), c(Part::Tax, Source::Tax), c(Fee, Fault)], true, true));
        rules.insert("missing_item".into(), rule("Something was missing", "stand", vec![c(Items, Fault), c(Part::Tax, Source::Tax)], true, true));
        // not to the fan's taste: the stand's to make right, as credit; Agape delivered it and keeps its fee
        rules.insert("quality".into(), rule("Not up to standard", "stand", vec![c(Items, Stand)], false, true));
        Policy { form: Form::Credit, rules, auto_per_fan: 2, auto_per_stand: 10, handoff_radius_m: 40, tip_bps: 2000, tip_min: 300,
                 proof_after_orders: 4, photo_at_bps: 2500, pin_at_bps: 5000, fixed_shift: false }
    }
}

impl Policy {
    /// The tip offered first on an order with these items.
    pub fn tip_for(&self, items: i64) -> i64 {
        let pct = crate::value::bps(items, self.tip_bps);
        // a whole dollar, rounded up: $3, $4, never $3.27
        ((pct.max(self.tip_min) + 99) / 100) * 100
    }
    /// The proof a fan's record calls for.
    pub fn proof_for(&self, orders: usize, claims: usize) -> Proof {
        if orders < self.proof_after_orders { return Proof::None; }
        let rate = claims as i64 * 10_000 / orders.max(1) as i64;
        if rate >= self.pin_at_bps { Proof::Pin } else if rate >= self.photo_at_bps { Proof::Photo } else { Proof::None }
    }
    /// Every rule names real parts and sources, and returns no more than all of a part.
    pub fn check(&self) -> Result<(), String> {
        for (k, r) in &self.rules {
            if !["stand", "runner", "agape"].contains(&r.fault.as_str()) { return Err(format!("{k}: fault must be stand, runner or agape")); }
            for p in [Part::Items, Part::Tax, Part::Fee, Part::Tip] {
                let total: i64 = r.covers.iter().filter(|x| x.part == p).map(|x| x.bps).sum();
                if total > 10_000 || r.covers.iter().any(|x| x.bps < 0) { return Err(format!("{k}: more than all of {p:?} is returned")); }
            }
        }
        for (k, r) in &self.rules { if r.covers.iter().all(|c| c.bps == 0) { return Err(format!("{}: nothing would come back to the fan", r.words.clone() + if k.is_empty() { "" } else { "" })); } }
        if self.tip_bps < 0 || self.tip_min < 0 { return Err("Tips can't be negative".into()); }
        if self.tip_bps > 5000 { return Err("The tip shown first can't be more than 50% of the items".into()); }
        if self.handoff_radius_m < 5 { return Err("The handoff distance needs to be at least 5 m; phones aren't more precise than that".into()); }
        if self.proof_after_orders < 1 { return Err("Proof can start after one order at the earliest".into()); }
        if self.photo_at_bps <= 0 || self.pin_at_bps <= 0 || self.photo_at_bps > 10_000 || self.pin_at_bps > 10_000 { return Err("Claim rates for proof must be between 1% and 100%".into()); }
        if self.photo_at_bps > self.pin_at_bps { return Err("A photo should be asked for at a lower claim rate than a PIN".into()); }
        Ok(())
    }
}

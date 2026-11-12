//! Play's rewards: the section battle is decided at last call and its prize paid, and tonight's
//! challenges pay points the moment a live fan finishes one. Every reward is points from Agape,
//! through the same ledger as everything else, so it counts toward the fan's level.

use super::*;
use crate::social::{points, ISSUER};
use serde_json::{json, Value};

/// Points each fan in the winning section gets.
pub const BATTLE_PRIZE: i64 = 150;

/// (id, name, what to do, points, the feature it needs; empty for always)
const CHALLENGES: &[(&str, &str, &str, i64, &str)] = &[
    ("two_orders", "Two rounds", "Have two orders brought to you tonight", 50, ""),
    ("three_calls", "Fortune teller", "Make three calls tonight", 60, "predictions"),
    ("first_card", "Keepsake", "Collect a moment card", 30, "cards"),
    ("follow", "Fan club", "Follow a player or a team", 20, "chat"),
    ("say", "Loud section", "Post or comment for the first time", 20, "chat"),
];

#[derive(Clone, Debug)]
pub struct BattleResult { pub section: String, pub orders: usize, pub t: i64, pub fans: usize }

impl Night {
    fn last_call_at(&self) -> i64 { self.script.iter().find(|x| x.1 == "period.start" && x.2 == 4).map(|x| x.0 * 60_000).unwrap_or(NOT_YET * 60_000) }

    fn seat_of(&self, fan: &str) -> Option<String> { self.fans.iter().find(|f| f.id == fan).map(|f| f.seat.clone()) }

    fn orders_had(&self, fan: &str) -> i64 { self.orders.iter().filter(|o| o.c.quote.fan == fan && o.c.stage == Stage::Settled).count() as i64 }

    fn progress(&self, fan: &str, id: &str) -> (i64, i64) {
        match id {
            "two_orders" => (self.orders_had(fan).min(2), 2),
            "three_calls" => (self.predictions.iter().filter(|p| p.picks.contains_key(fan)).count().min(3) as i64, 3),
            "first_card" => (self.social.editions.keys().any(|e| self.ledger.balance(&acct(fan, "holdings"), &crate::social::card_unit(e)) + self.ledger.balance(&acct(fan, "listed"), &crate::social::card_unit(e)) > 0) as i64, 1),
            "follow" => (self.social.follows.get(fan).map_or(false, |s| s.iter().any(|n| self.league.member(n).is_some() || self.league.team(n).is_some())) as i64, 1),
            "say" => (self.social.posts.iter().any(|p| p.author == fan || p.comments.iter().any(|c| c.1 == fan)) as i64, 1),
            _ => (0, 1),
        }
    }

    fn reward(&mut self, memo: String, to: &[String], n: i64) {
        if to.is_empty() || n <= 0 { return; }
        let mut postings: Vec<_> = to.iter().map(|f| crate::value::posting(acct(f, "points"), points(), n)).collect();
        postings.push(crate::value::posting(acct(ISSUER, "issued"), points(), -n * to.len() as i64));
        let _ = self.ledger.post(crate::value::Txn { memo, postings });
    }

    fn challenge_on(&self, need: &str) -> bool { need.is_empty() || self.on(need) }

    /// Each tick: decide the battle at last call, and pay any challenge a live fan just finished.
    pub(super) fn play_tick(&mut self, t: i64) {
        if self.on("battle") && self.battle.is_none() && t >= self.last_call_at() && t < self.script_final() {
            let lc = self.last_call_at();
            let mut by: BTreeMap<String, usize> = BTreeMap::new();
            // the same board fans watch all night: orders in the last five minutes, by section
            let _ = lc;
            for (_, sec, _) in &self.recent { if sec.starts_with('s') { *by.entry(sec.clone()).or_insert(0) += 1; } }
            if let Some((section, orders)) = by.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))) {
                let winners: Vec<String> = self.fans.iter().filter(|f| f.seat == section).map(|f| f.id.clone()).collect();
                self.reward(format!("prize Section {} won the section battle", section.trim_start_matches('s')), &winners, BATTLE_PRIZE);
                self.battle = Some(BattleResult { section, orders, t, fans: winners.len() });
            }
        }
        let live: Vec<String> = self.live.iter().cloned().collect();
        for fan in live {
            for (id, name, _, pts, need) in CHALLENGES {
                if !self.challenge_on(need) || self.challenged.contains(&(fan.clone(), id.to_string())) { continue; }
                let (have, of) = self.progress(&fan, id);
                if have >= of { self.challenged.insert((fan.clone(), id.to_string())); self.reward(format!("challenge {name}"), &[fan.clone()], *pts); }
            }
        }
    }

    pub fn challenges_json(&self, fan: &str) -> Vec<Value> {
        CHALLENGES.iter().filter(|c| self.challenge_on(c.4)).map(|(id, name, how, pts, _)| {
            let (have, of) = self.progress(fan, id);
            json!({ "id": id, "name": name, "how": how, "points": pts, "have": have, "of": of, "done": self.challenged.contains(&(fan.to_string(), id.to_string())) })
        }).collect()
    }

    pub fn battle_json(&self, fan: &str) -> Value {
        match &self.battle {
            Some(b) => json!({ "section": b.section, "orders": b.orders, "t": b.t, "fans": b.fans, "prize": BATTLE_PRIZE, "yours": self.seat_of(fan).as_deref() == Some(b.section.as_str()) }),
            None => json!({ "decides_at": self.last_call_at(), "prize": BATTLE_PRIZE }),
        }
    }

    pub fn won_battle(&self, fan: &str) -> bool { self.battle.as_ref().map_or(false, |b| self.seat_of(fan).as_deref() == Some(b.section.as_str())) }
}

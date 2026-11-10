//! Layer 3 adapter: a simulated event night. Every action goes through the kernel and the
//! engines exactly as live traffic would: signals into the log, rules into offers, offers
//! through eligibility and the Agape check, orders through holds, stations, dispatch and
//! settlement. Nothing here computes money; it only reads the ledger afterwards.

use crate::commitment::{fund, stock, standard_policy, Action, Commitment, CommitError, Stage};
use crate::economy::{breakdown, Line, Quote};
use crate::engines::{dispatch, Attribution, Costs, OfferSpec, Ready, Runner};
use crate::offers::{OfferBook, TapError};
use crate::rules::Mode;
use crate::fields;
use crate::rules::{Engine, Output, Template};
use crate::signal::{Log, Roles, Schema, Ty};
use crate::value::{acct, Ledger, Unit};
use crate::world::{section_id, Fan, World, WorldDef};
use std::collections::{BTreeMap, BTreeSet};

/// Small deterministic PRNG (xorshift64*), so the kernel needs no randomness dependency.
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Rng { Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1) }
    pub fn next(&mut self) -> u64 { let mut x = self.0; x ^= x >> 12; x ^= x << 25; x ^= x >> 27; self.0 = x; x.wrapping_mul(0x2545_F491_4F6C_DD1D) }
    pub fn f(&mut self) -> f64 { (self.next() >> 11) as f64 / (1u64 << 53) as f64 }
    pub fn below(&mut self, n: u64) -> u64 { self.next() % n.max(1) }
    pub fn chance(&mut self, p: f64) -> bool { self.f() < p }
}

pub fn sports_schema() -> Schema {
    crate::rules::add_span_kinds(Schema::default())
        .kind("gates.open", &[], Some("venue"))
        .kind("period.start", &[("n", Ty::Int)], Some("scorekeeper"))
        .kind("period.end", &[("n", Ty::Int)], Some("scorekeeper"))
        .kind("score", &[("team", Ty::Str), ("points", Ty::Int), ("player", Ty::Str), ("star", Ty::Bool)], Some("scorekeeper"))
        .kind("game.final", &[], Some("scorekeeper"))
        // concerts: the same platform runs any event; which kinds a night uses is up to its template
        .kind("doors.open", &[], Some("venue"))
        .kind("set.start", &[("n", Ty::Int)], Some("stage"))
        .kind("set.end", &[("n", Ty::Int)], Some("stage"))
        .kind("song", &[("title", Ty::Str), ("tag", Ty::Str)], Some("stage"))
        .kind("house.lights", &[], Some("venue"))
        .kind("crowd.level", &[("zone", Ty::Str), ("db", Ty::Int)], Some("sensor"))
        .kind("order.placed", &[("stand", Ty::Str), ("section", Ty::Str)], None)
        .kind("moment", &[("kind", Ty::Str), ("team", Ty::Str)], Some("system"))
        .kind("ops.alert", &[("kind", Ty::Str)], Some("system"))
        .kind("chat", &[("section", Ty::Str), ("text", Ty::Str)], None)
        .kind("offer", &[("offer", Ty::Str), ("act", Ty::Str), ("spec", Ty::Str)], None)
        .kind("exchange", &[("agreement", Ty::Str), ("offer", Ty::Str), ("stage", Ty::Str), ("step", Ty::Str), ("amount", Ty::Int), ("qty", Ty::Int), ("at", Ty::Int), ("detail", Ty::Str), ("via", Ty::Str)], None)
        .kind("social", &[("act", Ty::Str), ("target", Ty::Str), ("text", Ty::Str)], None)
}

const CHAT: &[&str] = &["LET'S GO {HOME}", "defense!!", "that was a foul", "who's getting nachos?", "this section is LOUD", "{star} is cooking tonight",
    "refs??", "need a pretzel run", "{away} can't miss right now", "best seats in the house", "one more stop", "anyone else freezing?",
    "drinks on me next round", "WHAT A PASS", "my runner was so fast", "free throws please", "we're winning the section battle",
    "first game here, this place is wild", "box out!!", "{p2} needs the ball more", "who's got the halftime pick?", "hot dog came in 4 min, unreal",
    "upper deck making noise", "that's a travel all day", "ref you're missing a great game", "can we get a stop", "my kid caught a shirt!!",
    "anyone collecting the cards tonight?", "{p3} with the hustle", "sweep the boards", "love this building", "big run coming, I can feel it"];
const CHAT_PRE: &[&str] = &["let's gooo", "first time in this building", "who's getting food before it starts?", "these seats are amazing", "{HOME} warmups look sharp", "can't wait", "ordered from my seat, this is the way", "section's filling up", "it's gonna be loud tonight"];
const CHAT_SHOW: &[&str] = &["{ACT}!!!", "this sound is unreal", "who's getting merch?", "I can't believe we're here", "play Fever Line!!", "the lights tonight", "best opener I've seen in a while", "my voice is already gone", "hot dog came in 6 min, didn't miss a song", "this section is LOUD"];
const CHAT_SONG: &[&str] = &["THIS SONG", "everybody's singing", "chills", "I'm actually crying", "LOUDER", "filming this whole thing", "best night ever", "the whole arena knows every word"];
const CHAT_RUN: &[&str] = &["{HOME} RUN", "here we go!!", "they can't stop us", "LOUDER", "WHAT A PLAY", "this building is shaking", "{STAR}!!!", "get that card!!", "we're so back"];
const CHAT_AWAY: &[&str] = &["ugh", "wake up {home}", "timeout please", "we're fine. we're fine.", "defense??", "they can't keep shooting like this", "still plenty of time", "need a stop here", "come on {home}"];

#[derive(Clone, Debug)]
pub struct Config {
    pub seed: u64, pub fans: usize, pub runners: usize, pub delivery_share: f64, pub consent_share: f64,
    /// Chance per 10 s that a fan who can see an offer taps it.
    pub tap_rate: f64,
    /// Multiplier on that chance when the card shows few left or ending soon. An assumption, not a
    /// measurement: run with 1.0 to see results without it.
    pub urgency_lift: f64,
    /// Override every offer's mode (to compare guaranteed with while-it-lasts on the same night).
    pub force_mode: Option<Mode>,
    /// A live fan ("you") in this section, acting through `Night::you_*` instead of the simulator.
    pub you_seat: Option<String>,
    /// Idle runners wait at the stands about to need them (see `engines::stage`).
    pub staging: bool,
    /// A real night: nobody is simulated. No crowd, no scripted game, no stand or runner acts unless a
    /// person does; the game moves when the venue's team marks it (`mark_game`, `keep_score`).
    pub real: bool,
}
impl Default for Config { fn default() -> Self { Config { seed: 1, fans: 3000, runners: 40, delivery_share: 0.75, consent_share: 0.7, tap_rate: 0.0015, urgency_lift: 2.0, force_mode: None, you_seat: None, staging: true, real: false } } }

#[derive(Debug, Default)]
pub struct Report {
    pub orders: u64,
    pub refused_at_accept: u64,
    pub by_stage: BTreeMap<String, u64>,
    pub wait_p50_s: i64,
    pub wait_p90_s: i64,
    pub late: u64,
    pub moments: Vec<(i64, String, String)>,
    pub alerts: u64,
    pub final_score: (i64, i64),
    pub money: BTreeMap<String, i64>,
    pub attribution: BTreeMap<String, crate::engines::Stat>,
    pub sponsor_spend_ledger: BTreeMap<String, i64>,
    pub open_holds: i64,
    pub conserved: bool,
    pub orders_per_min: Vec<(String, f64)>,
    pub runner_hourly: i64,
    /// Deliveries per runner per hour of the night.
    pub per_runner_hour: f64,
    /// Deliveries per runner in the busiest hour of the night.
    pub peak_per_runner_hour: f64,
    /// Trips offered to runners, how many were taken on the first offer, and the average pay per trip taken.
    pub flows: BTreeMap<String, i64>,
    pub trip_offers: u64,
    pub trips_taken: u64,
    pub taken_first: u64,
    pub trip_pay_avg: i64,
    pub batch_avg: f64,
    pub balked: u64,
    pub avg_queue_s: i64,
    pub avg_prep_s: i64,
    pub avg_dispatch_s: i64,
    pub avg_walk_s: i64,
    pub drained_min: i64,
    pub menu_sold_out: u64,
    pub offers_opened: u64,
    pub taps: u64,
    pub claims_lapsed: u64,
    pub offer_orders: u64,
    /// A fan wanted the offer but its card already showed it had ended. Seen, not a surprise.
    pub saw_it_end: u64,
    /// Must be zero: a card said live, the tap failed.
    pub tap_failed_after_live_card: u64,
    /// Must be zero except for claims the fan let lapse (those never reach checkout here).
    pub checkout_failed: u64,
}

#[derive(Clone)]
enum Ev { PickUp(Vec<usize>, String), Arrive(usize, String), FanCollects(usize), Settle(usize) }

pub(crate) struct Order {
    pub(crate) c: Commitment, pub(crate) source: String, pub(crate) accepted_at: i64, pub(crate) delivered_at: Option<i64>,
    pub(crate) started_at: Option<i64>, pub(crate) ready_at: Option<i64>, pub(crate) picked_at: Option<i64>,
    pub(crate) prep_s: u32, pub(crate) seat: String, pub(crate) step_free: bool,
    /// When the fan is told to expect it. Re-estimated at each stage.
    pub(crate) eta: i64,
    /// The arrival time the fan was shown when they ordered.
    pub(crate) quoted: i64,
    /// The latest it was promised for: 30 minutes after ordering, or the moment it was ordered for.
    pub(crate) promise: i64,
    /// Ordered ahead for this moment; held until the kitchen needs to start it.
    pub(crate) scheduled_for: Option<i64>,
    /// Sent to someone else: (who or where, the giver's note).
    pub(crate) gift: Option<(String, String)>,
    /// Paid for in points to go first: to the front of the kitchen's queue and of the runners' shelf.
    pub(crate) priority: bool,
    /// Its priority points are still held, waiting on how the order ends.
    pub(crate) priority_held: bool,
    /// Runners who were offered it and passed (each pass raises the next offer, in offers mode).
    pub(crate) passes: u32,
    /// Agape credit the fan's earlier remedies paid toward it, taken off the card at settlement.
    pub(crate) credit_used: i64,
    /// How far from the seat (metres) the runner's phone was when it was handed over. Agape's
    /// evidence only: never shown to the runner or the fan.
    pub(crate) handoff_m: Option<i64>,
}

pub const TICK: i64 = 10_000;
pub const YOU: &str = "you";
/// The venue as a party: it holds the box office and its revenue.
pub const VENUE: &str = "venue";

/// The night's script: (minute, signal). Periods of 28 minutes, breaks of 3, halftime of 18.
fn script() -> Vec<(i64, &'static str, i64)> {
    let mut v = vec![(0, "gates.open", 0)];
    let mut m = 45;
    for n in 1..=4 {
        v.push((m, "period.start", n));
        m += 28;
        v.push((m, "period.end", n));
        m += if n == 2 { 18 } else { 3 };
    }
    v.push((m - 3, "game.final", 0));
    v
}

/// Minute of a real night's game moments not yet marked: far enough that the clock never reaches it.
pub const NOT_YET: i64 = 100_000;

/// A real night's script: the doors are open; everything else waits for the venue's team to mark it.
fn real_script() -> Vec<(i64, &'static str, i64)> {
    let mut v = vec![(0, "gates.open", 0)];
    for n in 1..=4 { v.push((NOT_YET, "period.start", n)); v.push((NOT_YET, "period.end", n)); }
    v.push((NOT_YET, "game.final", 0));
    v
}

fn phase_of(s: &[(i64, &'static str, i64)], minute: i64) -> (&'static str, f64) {
    // orders per fan per minute, by phase of the night
    let p = |k: &str, n: i64| s.iter().find(|x| x.1 == k && x.2 == n).map_or(NOT_YET, |x| x.0);
    if minute >= p("game.final", 0) { ("postgame", 0.0) }
    else if minute < p("period.start", 1) { ("pregame", 0.0035) }
    else if minute >= p("period.end", 2) && minute < p("period.start", 3) { ("halftime", 0.011) }
    else if (1..=4).any(|n| minute >= p("period.start", n) && minute < p("period.end", n)) { ("in play", 0.0014) }
    else { ("break", 0.004) }
}

/// One line of what happened, for the feed.
#[derive(Clone, Debug)]
pub struct FeedItem { pub t: i64, pub kind: String, pub team: String, pub text: String, /** who it's about: a member or group id */ pub party: String, /** a card minted from it */ pub edition: Option<String> }

/// A whole event night as a world you can step through and act in.
pub struct Night {
    pub cfg: Config,
    pub world: World,
    pub terms: crate::economy::Terms,
    pub(crate) rng: Rng,
    pub log: Log,
    pub engine: Engine,
    pub ledger: Ledger,
    pub fans: Vec<Fan>,
    pub runners: Vec<Runner>,
    pub(crate) costs: Costs,
    pub attr: Attribution,
    pub(crate) orders: Vec<Order>,
    pub(crate) queues: BTreeMap<String, Vec<usize>>,
    pub(crate) stations: BTreeMap<String, Vec<(i64, Option<usize>)>>,
    pub(crate) ready: Vec<Ready>,
    events: Vec<(i64, u64, Ev)>,
    ev_seq: u64,
    pub book: OfferBook,
    checkouts: Vec<(i64, String)>,
    /// Orders placed ahead: (when the kitchen should start, order).
    pub(crate) scheduled: Vec<(i64, usize)>,
    pub report: Report,
    placed_by_phase: BTreeMap<&'static str, (u64, i64)>,
    batches: (u64, u64),
    pub score: (i64, i64),
    pub(crate) script: Vec<(i64, &'static str, i64)>,
    pub end_min: i64,
    menu: Vec<String>,
    pub t: i64,
    pub feed: Vec<FeedItem>,
    /// (t, section, thing) of recent orders, for the crowd's heat.
    pub recent: std::collections::VecDeque<(i64, String, String)>,
    /// Runner waypoints: (time, place) for the current run.
    pub plans: BTreeMap<String, Vec<(i64, String)>>,
    pub done: bool,
    /// Section chat: (t, author, section, text), newest last.
    pub chat: std::collections::VecDeque<(i64, String, String, String)>,
    /// Quarter predictions: who scores more this quarter. Correct picks earn points through the ledger.
    pub predictions: Vec<Prediction>,
    pub league: crate::league::League,
    pub social: crate::social::Social,
    pub last_scorer: BTreeMap<String, String>,
    /// Every section's seats for every upcoming event, and who holds which.
    pub seatmaps: BTreeMap<(String, String), crate::services::SeatMap>,
    /// Every exchange offer and agreement beyond the food order, folded from the log.
    pub trade: crate::exchange::Book,
    /// Players who have claimed their Agape profile. Until then, what's sent to them waits.
    pub claimed: BTreeSet<String>,
    /// Parties a person is acting as right now; the simulation stops deciding for them.
    pub driven: BTreeSet<String>,
    /// (fan, place) -> when the pass or perk was used at the counter.
    pub redeemed: BTreeMap<(String, String), i64>,
    /// Replies the simulated parties will send: (when, from, to, text).
    pub(crate) replies: Vec<(i64, String, String, String)>,
    /// (log length, result) of the last check that the exchange book rebuilds from the log.
    pub(crate) replay_check: std::cell::Cell<(usize, bool)>,
    /// Fans who are real people acting through a device. The simulation never acts for them.
    pub live: BTreeSet<String>,
    /// Batches offered to runners who are people, and the trips they've taken on.
    pub trips: BTreeMap<String, Trip>,
    /// Songs of the headliner's setlist played so far (a concert).
    pub(crate) songs_played: usize,
    /// Trip offers made, trips taken, taken on the first offer, and total trip pay.
    pub(crate) pay_stats: (u64, u64, u64, i64),
    /// Problems fans reported with orders, and how each was settled.
    pub problems: Vec<crate::care::Problem>,
    /// Minutes of wage paid to each runner so far tonight (hourly mode).
    pub(crate) wage_minutes: BTreeMap<String, i64>,
    /// Agape's policy for problems, tips and proof at the handoff; set from Agape's own app.
    pub policy: crate::remedy::Policy,
    /// Proof Agape's team has set for a fan by hand, over what their record calls for.
    pub proof_set: BTreeMap<String, crate::remedy::Proof>,
    /// Service stopped by the venue, and why.
    pub halt: Option<crate::care::Halt>,
    /// (order, who was rated, stars 1-5, by, when), folded from rating signals.
    pub ratings: Vec<(usize, String, i64, String, i64)>,
    /// The names real people chose for themselves.
    pub names: BTreeMap<String, String>,
    /// Whose colors each fan wears in the app: a team playing tonight, or "agape". Unset means the home team.
    pub wearing: BTreeMap<String, String>,
    /// Where exactly each fan sits inside their section ("Row F, seat 7"), as they told us.
    pub spots: BTreeMap<String, String>,
    /// League names the venue shows differently (its own teams and players).
    pub renames: BTreeMap<String, String>,
    /// Level earned at other venues, as an offset on what's been earned here.
    pub level_offset: BTreeMap<String, i64>,
    /// What each fan's passport says about them (venues, nights), as carried in.
    pub passports: BTreeMap<String, serde_json::Value>,
    /// Passport transfers already applied here, so repeating one is harmless.
    pub transfers: BTreeMap<String, crate::passport::CarriedOut>,
}

/// A batch of deliveries for a runner who is a person: offered first (route and pay shown), then
/// theirs once accepted. Declined or left unanswered, it goes back to the shelf for someone else.
#[derive(Clone, Debug)]
pub struct Trip {
    pub orders: Vec<usize>, pub stand: String, pub offered_at: i64, pub accepted: bool, pub picked: bool,
    /// What Agape pays for the trip (tips on top), the first offer it was priced at, and how many runners passed first.
    pub pay: i64, pub first: i64, pub passes: u32,
    pub route: Vec<String>, pub pickup_s: u32, pub pickups: Vec<(String, u32)>, pub arrive_s: Vec<u32>, pub done_s: u32,
}

/// What going first costs, in points, and how far ahead of others on the shelf it puts an order.
pub const PRIORITY_POINTS: i64 = 200;
pub const PRIORITY_AHEAD_MS: i64 = 10 * 60_000;

/// How long a runner has to take an offered trip.
pub const TRIP_OFFER_MS: i64 = 60_000;

/// What an hour of a simulated runner's time is worth to them (pay and tips), $15 to $30, fixed per runner.
fn reservation(id: &str) -> i64 {
    let h = id.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    1500 + (h % 1500) as i64
}

#[derive(Clone, Debug)]
pub struct Prediction {
    /// The question's id in the template, and its text as asked.
    pub ask: String,
    pub question: String,
    /// Option ids, each with the measure that decides it and its value when the question opened.
    pub options: Vec<(String, Option<String>, i64)>,
    pub otherwise: Option<String>,
    pub points: i64,
    pub opens: i64,
    /// Picks close a few minutes into the quarter.
    pub locks: i64,
    pub picks: BTreeMap<String, String>,
    /// "home", "away" or "tie" once the quarter ends.
    pub result: Option<String>,
}

/// Points a correct prediction earns, issued by Agape.
pub const PREDICTION_POINTS: i64 = 50;

impl Night {
    pub fn new(world_def: WorldDef, tpl: Template, cfg: Config) -> Night {
        let mut league: crate::league::League = serde_json::from_str(include_str!("../templates/league.json")).expect("league");
        let original = league.clone();
        // a performance night doesn't rename the league's teams: the brand there is the performers' colors
        let performance = world_def.event.as_ref().map_or(false, |e| e.kind == "concert");
        if let Some(b) = world_def.brand.as_ref().filter(|_| !performance) {
            let ids = [league.tonight.home.clone(), league.tonight.away.clone()];
            for (id, tb) in ids.iter().zip([&b.home, &b.away]) {
                let (Some(tb), Some(g)) = (tb, league.teams.iter_mut().find(|g| &g.id == id)) else { continue };
                if !tb.name.trim().is_empty() { g.name = tb.name.trim().into(); }
                if !tb.short.trim().is_empty() { g.short = tb.short.trim().into(); }
                if !tb.abbr.trim().is_empty() { g.abbr = tb.abbr.trim().to_uppercase(); }
                if tb.color.starts_with('#') { g.color = tb.color.clone(); }
                for (m, p) in g.roster.iter_mut().zip(&tb.players) {
                    if !p.name.trim().is_empty() { m.name = p.name.trim().into(); }
                    if let Some(n) = p.number { m.number = n; }
                }
            }
        }
        // words the template wrote with the league's names, shown with the venue's
        let mut renames = BTreeMap::new();
        for (a, b) in original.teams.iter().zip(&league.teams) {
            if a.name != b.name { renames.insert(a.name.clone(), b.name.clone()); }
            if a.short != b.short { renames.insert(a.short.clone(), b.short.clone()); }
            for (x, y) in a.roster.iter().zip(&b.roster) { if x.name != y.name { renames.insert(x.name.clone(), y.name.clone()); } }
        }
        // upcoming events are titled with the league's names too
        for e in league.upcoming.iter_mut() {
            // longest names first, so "Harbor Kings" is replaced before "Kings"
            let mut keys: Vec<(&String, &String)> = renames.iter().collect();
            keys.sort_by_key(|(a, _)| std::cmp::Reverse(a.len()));
            for (a, b) in keys { e.title = e.title.replace(a.as_str(), b); }
        }
        let mut n = Self::new_in(world_def, tpl, league, cfg);
        n.renames = renames;
        n
    }

    /// Text written with the league's names, in the venue's.
    pub fn renamed(&self, s: &str) -> String {
        let mut keys: Vec<(&String, &String)> = self.renames.iter().collect();
        keys.sort_by_key(|(a, _)| std::cmp::Reverse(a.len()));
        // a placeholder per name, so a new name containing an old one isn't replaced twice
        let mut out = s.to_string();
        for (i, (a, _)) in keys.iter().enumerate() { out = out.replace(a.as_str(), &format!("\u{1}{i}\u{2}")); }
        for (i, (_, b)) in keys.iter().enumerate() { out = out.replace(&format!("\u{1}{i}\u{2}"), b); }
        out
    }

    pub fn new_in(world_def: WorldDef, tpl: Template, league: crate::league::League, cfg: Config) -> Night {
        let world = World::build(world_def);
        let terms = world.def.terms.clone();
        let mut rng = Rng::new(cfg.seed);
        let schema = sports_schema();
        let mut roles = Roles::default();
        roles.grant("venue", "venue");
        roles.grant("keeper", "scorekeeper");
        let log = Log::new(schema.clone(), roles);
        let engine = Engine::load(tpl, &schema).expect("template loads");
        let mut ledger = Ledger::new(standard_policy());
        for t in &world.def.things { stock(&mut ledger, &t.stand, &t.id, t.stock.unwrap_or(1_000_000)).unwrap(); }
        for (sponsor, budget) in [("sponsor_cola", 400_00), ("sponsor_water", 300_00)] { fund(&mut ledger, sponsor, budget).unwrap(); }
        let n = world.def.sections_per_level;
        let mut fans: Vec<Fan> = (0..cfg.fans).map(|i| {
            let level = if rng.chance(0.6) { 1 } else { 2 };
            let mut consents = BTreeSet::new();
            if rng.chance(cfg.consent_share) { consents.insert("offers".to_string()); }
            Fan { id: format!("fan{i}"), seat: section_id(level, rng.below(n as u64) as u32), age_verified: if rng.chance(0.55) { Some(21 + rng.below(40) as u8) } else { None },
                  step_free: rng.chance(0.04), consents, ceiling: 2_500 + rng.below(6_000) as i64 }
        }).collect();
        if let Some(seat) = &cfg.you_seat {
            fans.push(Fan { id: YOU.into(), seat: seat.clone(), age_verified: Some(25), step_free: false, consents: ["offers".to_string()].into(), ceiling: 20_000 });
        }
        let runners: Vec<Runner> = (0..cfg.runners).map(|i| Runner { id: format!("runner{i}"), at: world.def.stands[i % world.def.stands.len()].id.clone(), free_at: 0 }).collect();
        let stations = world.def.stands.iter().map(|s| (s.id.clone(), vec![(0, None); s.stations as usize])).collect();
        let script = if cfg.real { real_script() } else { script() };
        let end_min = if cfg.real { NOT_YET + 20 } else { script.last().unwrap().0 + 20 };
        // on a real night every stand and runner is a person: the simulation never acts for them
        let driven: BTreeSet<String> = if cfg.real { world.def.stands.iter().map(|s| s.id.clone()).chain(runners.iter().map(|r| r.id.clone())).collect() } else { BTreeSet::new() };
        let menu = world.def.things.iter().filter(|t| t.id != "combo").map(|t| t.id.clone()).collect();
        Night {
            cfg, terms, rng, log, engine, ledger, fans, runners, costs: Costs::new(), attr: Attribution::default(), orders: vec![],
            queues: BTreeMap::new(), stations, ready: vec![], events: vec![], ev_seq: 0, book: OfferBook::new(120_000), checkouts: vec![], scheduled: vec![],
            report: Report::default(), placed_by_phase: BTreeMap::new(), batches: (0, 0), score: (0, 0), script, end_min, menu, t: 0,
            feed: vec![], recent: Default::default(), plans: BTreeMap::new(), done: false, chat: Default::default(), predictions: vec![], world,
            league, social: crate::social::Social::default(), last_scorer: BTreeMap::new(), seatmaps: BTreeMap::new(),
            trade: Default::default(), claimed: BTreeSet::new(), driven, redeemed: BTreeMap::new(), replies: vec![], replay_check: std::cell::Cell::new((0, true)), live: BTreeSet::new(), trips: BTreeMap::new(), songs_played: 0, pay_stats: (0, 0, 0, 0), problems: vec![], policy: Default::default(), proof_set: BTreeMap::new(), wage_minutes: BTreeMap::new(), halt: None, ratings: vec![], names: BTreeMap::new(), wearing: BTreeMap::new(), renames: BTreeMap::new(), spots: BTreeMap::new(), level_offset: BTreeMap::new(), passports: BTreeMap::new(), transfers: BTreeMap::new(),
        }.seeded_live()
    }

    fn sched(&mut self, t: i64, e: Ev) { self.ev_seq += 1; self.events.push((t, self.ev_seq, e)); }

    /// For each delivered order: how many seconds after its first estimate it actually arrived (negative: early).
    pub fn eta_errors(&self) -> Vec<(i64, i64)> {
        self.orders.iter().filter(|o| o.c.quote.delivery && o.scheduled_for.is_none()).filter_map(|o| o.delivered_at.map(|d| ((o.quoted - o.accepted_at) / 1000, (d - o.quoted) / 1000))).collect()
    }

    /// A crowd line in tonight's names: the home team, the visitors, the home stars.
    fn crowd_words(&self, line: &str) -> String {
        let home = self.league.team(&self.league.tonight.home);
        let away = self.league.team(&self.league.tonight.away).map(|g| g.short.clone()).unwrap_or_default();
        let short = home.map(|g| g.short.clone()).unwrap_or_default();
        let who = |i: usize| home.and_then(|g| g.roster.get(i)).map(|m| last_name(&m.name)).unwrap_or_default();
        let star = home.and_then(|g| g.roster.iter().find(|m| m.star)).map(|m| last_name(&m.name)).unwrap_or_else(|| who(0));
        let act = self.concert().and_then(|c| c.acts.last().cloned()).unwrap_or_default();
        line.replace("{ACT}", &act.to_uppercase()).replace("{HOME}", &short.to_uppercase()).replace("{home}", &short.to_lowercase()).replace("{away}", &away)
            .replace("{STAR}", &star.to_uppercase()).replace("{star}", &star).replace("{p2}", &who(1)).replace("{p3}", &who(2))
    }

    pub fn stand_wait(&self, stand: &str) -> i64 { stand_wait(&self.stations, &self.queues, &self.orders, stand, self.t) }

    pub fn phase(&self) -> &'static str { phase_of(&self.script, self.t / 60_000).0 }

    fn note(&mut self, kind: &str, team: &str, text: String) { self.note_about(kind, team, text, "", None) }

    fn note_about(&mut self, kind: &str, team: &str, text: String, party: &str, edition: Option<String>) {
        self.feed.push(FeedItem { t: self.t, kind: kind.into(), team: team.into(), text, party: party.into(), edition });
        if self.feed.len() > 400 { self.feed.drain(0..100); }
    }

    /// Estimate when a new order would reach `seat`: kitchen wait, its own prep, the walk, and a minute to dispatch.
    pub fn estimate(&mut self, stand: &str, prep_s: u32, seat: &str, delivery: bool) -> i64 {
        let walk = if delivery { self.costs.get(&self.world, stand, seat, false) as i64 + 60 } else { 0 };
        // Measured over whole nights, the bare sum is late half the time: runners batch and the line
        // grows behind you. 30% more and two minutes puts nearly 9 in 10 at the seat by the time shown.
        let base = self.stand_wait(stand) + prep_s as i64 + walk;
        self.t + (base * 13 / 10 + 120) * 1000
    }

    fn place(&mut self, held: Commitment, source: String, outs: &mut Vec<Output>) {
        let q = held.quote.clone();
        let fan = self.fans.iter().find(|f| f.id == q.fan).unwrap().clone();
        let stand = q.stand.clone();
        let prep: u32 = q.lines.iter().map(|l| self.world.thing(&l.thing).unwrap().prep_s * l.qty as u32).sum();
        let t = self.t;
        outs.extend(self.engine.ingest(&mut self.log, "order.placed", fields!{"stand" => stand.as_str(), "section" => fan.seat.as_str()}, &fan.id, t).unwrap());
        self.recent.push_back((t, fan.seat.clone(), q.lines[0].thing.clone()));
        let mut c = held;
        if !self.live.contains(&fan.id) && self.rng.chance(0.005) { c.act(Action::PaymentFailed, "agape", &mut self.ledger, &self.terms).unwrap(); }
        let eta = self.estimate(&stand, prep, &fan.seat, q.delivery);
        let id = self.orders.len();
        if source != "menu" { self.report.offer_orders += 1; }
        self.orders.push(Order { c, source, accepted_at: t, delivered_at: None, started_at: None, ready_at: None, picked_at: None, prep_s: prep, seat: fan.seat.clone(), step_free: fan.step_free, eta, quoted: eta, promise: t + 1_800_000, scheduled_for: None, gift: None, priority: false, priority_held: false, passes: 0, credit_used: 0, handoff_m: None });
        if self.orders[id].c.stage == Stage::Accepted { self.queues.entry(stand).or_default().push(id); }
        self.report.orders += 1;
        let ph = phase_of(&self.script, t / 60_000).0;
        self.placed_by_phase.entry(ph).or_insert((0, 0)).0 += 1;
    }

    // ---------- what the live fan can do ----------

    /// Order from the menu. Adding to the order holds the items, so this either succeeds or says why now.
    pub fn you_order(&mut self, thing: &str, qty: i64, delivery: bool, tip: i64) -> Result<String, String> { self.order_as(YOU, thing, qty, delivery, tip, None) }
    pub fn you_order_at(&mut self, thing: &str, qty: i64, delivery: bool, tip: i64, at: Option<i64>) -> Result<String, String> { self.order_as(YOU, thing, qty, delivery, tip, at) }

    /// A person joins the night as a fan in a section. Their phone acts for them from now on.
    pub fn join_named(&mut self, id: &str, seat: &str, name: &str, welcome: bool) -> Result<(), String> {
        let name: String = name.trim().chars().take(24).collect();
        if !name.is_empty() { self.names.insert(id.into(), name); }
        self.join_with(id, seat, None, welcome)
    }

    pub fn join(&mut self, id: &str, seat: &str, age_verified: Option<u8>) -> Result<(), String> { self.join_with(id, seat, age_verified, true) }

    fn join_with(&mut self, id: &str, seat: &str, age_verified: Option<u8>, welcome: bool) -> Result<(), String> {
        if self.fans.iter().any(|f| f.id == id) { self.live.insert(id.into()); return Ok(()); }
        if !self.world.graph.places.contains_key(seat) || !seat.starts_with('s') { return Err("Unknown section".into()); }
        self.fans.push(Fan { id: id.into(), seat: seat.into(), age_verified, step_free: false, consents: ["offers".to_string()].into(), ceiling: 20_000 });
        self.live.insert(id.into());
        if !welcome { return Ok(()); }
        self.ledger.post(crate::value::Txn { memo: "welcome".into(), postings: vec![
            crate::value::posting(acct(crate::social::ISSUER, "issued"), crate::social::points(), -100), crate::value::posting(acct(id, "points"), crate::social::points(), 100)] }).map_err(|e| format!("{e:?}"))?;
        Ok(())
    }

    fn live_fan(&self, id: &str) -> Result<Fan, String> { self.fans.iter().find(|f| f.id == id).cloned().ok_or_else(|| "no live fan".to_string()) }

    /// Order now for a later moment (halftime, the start of the 4th). Stock and payment are held
    /// now; the kitchen starts it so it arrives at that moment, and lateness counts from that moment.
    pub fn order_as(&mut self, who: &str, thing: &str, qty: i64, delivery: bool, tip: i64, at: Option<i64>) -> Result<String, String> {
        self.halted()?;
        if let Some(a) = at {
            let (earliest, closes) = self.order_window_for(who, thing, delivery).ok_or("unknown item")?;
            if a < earliest - 60_000 { return Err("TooSoon".into()); }
            if a > closes { return Err("Ordering closes at the final".into()); }
        }
        let fan = self.live_fan(who)?;
        let th = self.world.thing(thing).ok_or("unknown item")?.clone();
        self.world.eligible(&fan, thing, &|s| self.engine.span_open(s), false).map_err(|r| format!("{r:?}"))?;
        if let Some(why) = self.closed_now(thing) { return Err(why.into()); }
        let q = Quote { fan: who.into(), stand: th.stand.clone(), runner: None, lines: vec![Line { thing: th.id.clone(), qty, unit_price: th.price }],
                        discounts: vec![], points_redeemed: 0, tip, delivery, fee_waived: false, runner_pay: None };
        let c = Commitment::accept(&format!("o{}", self.orders.len()), q, &mut self.ledger).map_err(|_| "SoldOut".to_string())?;
        let mut outs = vec![];
        self.place(c, "menu".into(), &mut outs);
        self.absorb(outs);
        let id = self.orders.len() - 1;
        if let Some(at) = at {
            let o = &self.orders[id];
            let walk = if delivery { self.costs.get(&self.world, &th.stand, &o.seat, false) as i64 } else { 0 };
            let start = at - (o.prep_s as i64 + walk + 180) * 1000;
            if start > self.t {
                for q in self.queues.values_mut() { q.retain(|&i| i != id); }
                self.scheduled.push((start, id));
            }
            let o = &mut self.orders[id];
            o.eta = at; o.promise = at + 600_000; o.scheduled_for = Some(at);
        }
        Ok(format!("{id}"))
    }

    /// Order a cart: everything in it at once. Items from one stand travel as one order; the cart
    /// pays one delivery fee however many stands it spans, and the tip is shared between its runners.
    /// Nothing is placed unless all of it can be.
    pub fn order_cart(&mut self, who: &str, lines: &[(String, i64)], delivery: bool, tip: i64) -> Result<Vec<usize>, String> {
        self.halted()?;
        if lines.is_empty() { return Err("Your cart is empty".into()); }
        if tip < 0 { return Err("A tip can't be negative".into()); }
        let fan = self.live_fan(who)?;
        let mut groups: Vec<(String, Vec<Line>)> = vec![];
        for (thing, qty) in lines {
            if *qty <= 0 { return Err("Pick at least one".into()); }
            let th = self.world.thing(thing).ok_or("unknown item")?.clone();
            self.world.eligible(&fan, thing, &|s| self.engine.span_open(s), false).map_err(|r| format!("{r:?}"))?;
            if let Some(why) = self.closed_now(thing) { return Err(why.into()); }
            let have = self.ledger.balance(&acct(&th.stand, "inventory"), &Unit::Thing(th.id.clone()));
            if have < *qty { return Err(format!("{} just sold out", th.name)); }
            match groups.iter_mut().find(|g| g.0 == th.stand) {
                Some(g) => match g.1.iter_mut().find(|l| l.thing == th.id) { Some(l) => l.qty += qty, None => g.1.push(Line { thing: th.id.clone(), qty: *qty, unit_price: th.price }) },
                None => groups.push((th.stand.clone(), vec![Line { thing: th.id.clone(), qty: *qty, unit_price: th.price }])),
            }
        }
        let n = groups.len() as i64;
        // the whole cart counts toward free delivery
        let total: i64 = groups.iter().flat_map(|g| g.1.iter()).map(|l| l.qty * l.unit_price).sum();
        let free = self.terms.fee_for(total) == 0;
        let mut ids = vec![];
        for (k, (stand, lines)) in groups.into_iter().enumerate() {
            let share = if delivery { tip / n + if k == 0 { tip % n } else { 0 } } else { 0 };
            let q = Quote { fan: who.into(), stand, runner: None, lines, discounts: vec![], points_redeemed: 0, tip: share, delivery, fee_waived: k > 0 || free, runner_pay: None };
            let c = match Commitment::accept(&format!("o{}", self.orders.len()), q, &mut self.ledger) {
                Ok(c) => c,
                Err(_) => { for &id in &ids { let _ = self.cancel_as(who, id); } return Err("Something in your cart just sold out".into()); }
            };
            let mut outs = vec![];
            self.place(c, "menu".into(), &mut outs);
            self.absorb(outs);
            ids.push(self.orders.len() - 1);
        }
        Ok(ids)
    }

    /// What a cart would cost, without placing it.
    pub fn cart_quote(&self, who: &str, lines: &[(String, i64)], delivery: bool, tip: i64) -> Result<serde_json::Value, String> {
        let mut groups: Vec<(String, i64)> = vec![];
        let mut items = 0i64;
        for (thing, qty) in lines {
            let th = self.world.thing(thing).ok_or("unknown item")?;
            items += qty * th.price;
            if !groups.iter().any(|g| g.0 == th.stand) { groups.push((th.stand.clone(), 0)); }
            groups.iter_mut().find(|g| g.0 == th.stand).unwrap().1 += qty * th.price;
        }
        let _ = who;
        let tax: i64 = groups.iter().map(|g| crate::value::bps(g.1, self.terms.tax_bps)).sum();
        let fee = if delivery && !groups.is_empty() { self.terms.fee_for(items) } else { 0 };
        let tip = if delivery { tip } else { 0 };
        Ok(serde_json::json!({ "items": items, "tax": tax, "fee": fee, "tip": tip, "charge": items + tax + fee + tip, "stands": groups.len() }))
    }

    /// When the live fan can have this item: from the soonest it could reach them to when ordering closes.
    pub fn order_window(&mut self, thing: &str, delivery: bool) -> Option<(i64, i64)> { self.order_window_for(YOU, thing, delivery) }
    pub fn order_window_for(&mut self, who: &str, thing: &str, delivery: bool) -> Option<(i64, i64)> {
        let th = self.world.thing(thing)?.clone();
        let seat = self.fans.iter().find(|f| f.id == who)?.seat.clone();
        let earliest = self.estimate(&th.stand, th.prep_s, &seat, delivery);
        let mut closes = self.script.iter().find(|x| x.1 == "game.final").map(|x| x.0 * 60_000).unwrap_or(self.end_min * 60_000);
        // a real game's final isn't known ahead: orders can be placed for up to three hours from now
        if closes >= NOT_YET * 60_000 { closes = self.t + 3 * 3_600_000; }
        Some((earliest, closes.max(earliest)))
    }

    /// The exact price breakdown before the fan commits: what the screen shows is what the ledger will post.
    pub fn you_quote(&self, thing: &str, qty: i64, delivery: bool, tip: i64, claim: Option<&str>) -> Result<serde_json::Value, String> { self.quote_for(YOU, thing, qty, delivery, tip, claim) }
    pub fn quote_for(&self, who: &str, thing: &str, qty: i64, delivery: bool, tip: i64, claim: Option<&str>) -> Result<serde_json::Value, String> {
        let q = match claim {
            Some(c) => { let mut q = self.book.claims.get(c).filter(|c| c.quote.fan == who).ok_or("claim lapsed")?.quote.clone(); q.tip = tip; q }
            None => {
                let th = self.world.thing(thing).ok_or("unknown item")?;
                Quote { fan: who.into(), stand: th.stand.clone(), runner: None, lines: vec![Line { thing: th.id.clone(), qty, unit_price: th.price }],
                        discounts: vec![], points_redeemed: 0, tip, delivery, fee_waived: false, runner_pay: None }
            }
        };
        let b = breakdown(&q, &self.terms).map_err(|e| format!("{e:?}"))?;
        Ok(serde_json::json!({ "subtotal": b.subtotal, "discount": b.discount, "tax": b.tax, "fee": b.fee, "tip": b.tip, "charge": b.charge,
            "payer": q.discounts.first().map(|d| d.payer.clone()) }))
    }

    pub fn you_tap(&mut self, offer: &str) -> Result<String, String> { self.tap_as(YOU, offer) }
    pub fn tap_as(&mut self, who: &str, offer: &str) -> Result<String, String> {
        self.halted()?;
        let fan = self.live_fan(who)?;
        let c = self.book.tap(offer, &fan, &self.world, &mut self.ledger, &self.engine, self.t).map_err(|e| format!("{e:?}"))?;
        self.report.taps += 1;
        Ok(c.id)
    }

    pub fn you_checkout(&mut self, claim: &str, tip: i64) -> Result<String, String> { self.checkout_as(YOU, claim, tip) }
    pub fn checkout_as(&mut self, who: &str, claim: &str, tip: i64) -> Result<String, String> {
        self.halted()?;
        if self.book.claims.get(claim).map_or(true, |c| c.quote.fan != who) { return Err("claim lapsed".into()); }
        let id = format!("o{}", self.orders.len());
        let (c, source) = self.book.checkout(claim, &id, self.t, tip).map_err(|e| format!("{e:?}"))?;
        self.attr.accepted(&source);
        let mut outs = vec![];
        self.place(c, source, &mut outs);
        self.absorb(outs);
        Ok(format!("{}", self.orders.len() - 1))
    }

    /// Post to your section's chat.
    pub fn you_chat(&mut self, text: &str) -> Result<(), String> { self.chat_as(YOU, text) }
    /// A fan says they're 21 or older. Their runner checks ID at the seat on the first delivery that needs it.
    pub fn attest_age(&mut self, who: &str) -> Result<(), String> {
        let f = self.fans.iter_mut().find(|f| f.id == who).ok_or("Join first")?;
        if f.age_verified.map_or(true, |a| a < 21) { f.age_verified = Some(21); }
        Ok(())
    }

    /// Why nothing more of this can be ordered tonight: the kitchens close at the final.
    pub fn closed_now(&self, thing: &str) -> Option<&'static str> {
        let m = self.t / 60_000;
        if phase_of(&self.script, m).0 == "postgame" { return Some("ClosedForNight"); }
        // on a real night a stand takes orders only while someone is there to make them
        if self.cfg.real { if let Some(th) = self.world.thing(thing) { if !self.live.contains(&th.stand) { return Some("StandClosed"); } } } // last call for alcohol is the template's own span
        None
    }

    /// Where in the section: row and seat, so the runner walks straight to you.
    pub fn set_spot(&mut self, who: &str, row: &str, seat: &str) -> Result<(), String> {
        self.live_fan(who)?;
        let clean = |x: &str| x.trim().chars().filter(|c| c.is_ascii_alphanumeric()).take(4).collect::<String>().to_uppercase();
        let (row, seat) = (clean(row), clean(seat));
        if row.is_empty() && seat.is_empty() { self.spots.remove(who); return Ok(()); }
        let words = match (row.is_empty(), seat.is_empty()) { (false, false) => format!("Row {row}, seat {seat}"), (false, true) => format!("Row {row}"), _ => format!("Seat {seat}") };
        self.spots.insert(who.into(), words);
        Ok(())
    }

    /// Wear a team's colors in the app (or Agape's own). Only a team playing tonight; "" goes back to the home team.
    pub fn wear(&mut self, who: &str, team: &str) -> Result<(), String> {
        if !self.fans.iter().any(|f| f.id == who) { return Err("Join first".into()); }
        match team {
            "" => { self.wearing.remove(who); }
            "agape" => { self.wearing.insert(who.into(), "agape".into()); }
            t if t == self.league.tonight.home || t == self.league.tonight.away => { self.wearing.insert(who.into(), t.into()); }
            _ => return Err("That team isn't playing tonight".into()),
        }
        Ok(())
    }

    pub fn chat_as(&mut self, who: &str, text: &str) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > 140 { return Err("Say something up to 140 characters".into()); }
        let seat = self.live_fan(who)?.seat.clone();
        let text = moderate(text)?;
        self.post_chat(who, &seat, &text);
        Ok(())
    }

    fn post_chat(&mut self, who: &str, section: &str, text: &str) {
        let t = self.t;
        let _ = self.engine.ingest(&mut self.log, "chat", fields!{"section" => section, "text" => text}, who, t);
        self.chat.push_back((t, who.to_string(), section.to_string(), text.to_string()));
        if self.chat.len() > 3000 { self.chat.pop_front(); }
    }

    /// The league group playing as "home" or "away" tonight.
    pub fn group_of(&self, side: &str) -> String {
        match side { "home" => self.league.tonight.home.clone(), "away" => self.league.tonight.away.clone(), x => x.to_string() }
    }

    /// Before the night: fans arrive with the history they've built in the app, and last
    /// season's cards are already out in the world and on the market.
    fn seeded_live(mut self) -> Night {
        if self.fans.iter().any(|f| f.id == YOU) { self.live.insert(YOU.into()); }
        self.seeded()
    }

    fn seeded(mut self) -> Night {
        use crate::social::{Edition, ISSUER, points};
        let n = self.cfg.fans;
        let mut hist = vec![];
        for i in 0..n {
            let u = self.rng.f().max(0.002);
            let pts = ((60.0 / u.powf(0.85)) as i64).min(14_000);
            hist.push(crate::value::posting(acct(&self.fans[i].id, "points"), points(), pts));
        }
        let total: i64 = hist.iter().map(|p| p.amount).sum();
        hist.push(crate::value::posting(acct(ISSUER, "issued"), points(), -total));
        self.ledger.post(crate::value::Txn { memo: "history".into(), postings: hist }).unwrap();
        if self.fans.iter().any(|f| f.id == YOU) {
            self.ledger.post(crate::value::Txn { memo: "welcome".into(), postings: vec![
                crate::value::posting(acct(ISSUER, "issued"), points(), -100), crate::value::posting(acct(YOU, "points"), points(), 100)] }).unwrap();
        }
        // the box office: every seat of every upcoming event; some already sold before tonight
        let sections = self.ticket_sections();
        for (k, e) in self.league.upcoming.clone().into_iter().enumerate() {
            let sold_share = [0.62, 0.45, 0.3][k.min(2)];
            for sec in &sections {
                let (rows, per) = if sec == "sFL" { (2, 12) } else { (20, 20) };
                let mut m = crate::services::SeatMap::new(rows, per);
                let pull = if sec == "sFL" { 0.8 } else if self.section_view(sec) == "Sideline" { sold_share + 0.15 } else { sold_share };
                for i in 0..rows * per { if self.rng.chance(pull) { m.owner[i] = Some("presold".into()); } }
                let free = m.free() as i64;
                self.ledger.post(crate::value::Txn { memo: format!("onsale {}", e.id), postings: vec![
                    crate::value::posting(acct("world", "supply"), crate::economy::ticket_unit(&e.id, sec), -free),
                    crate::value::posting(acct(VENUE, "tickets"), crate::economy::ticket_unit(&e.id, sec), free),
                ]}).unwrap();
                self.seatmaps.insert((e.id.clone(), sec.clone()), m);
            }
        }
        // parking passes the operator can sell tonight
        for a in self.world.def.around.clone() {
            if let Some(sv) = a.service.as_ref().filter(|s| s.kind == "parking") {
                self.ledger.post(crate::value::Txn { memo: format!("lot {}", a.id), postings: vec![
                    crate::value::posting(acct("world", "supply"), pass_unit(&a.id), -sv.supply),
                    crate::value::posting(acct(&a.id, "passes"), pass_unit(&a.id), sv.supply),
                ]}).unwrap();
            }
        }
        // partners put their perks into the world
        for a in self.world.def.around.clone() {
            if let Some(p) = &a.perk {
                self.ledger.post(crate::value::Txn { memo: format!("perk {}", a.id), postings: vec![
                    crate::value::posting(acct("world", "supply"), pass_unit(&a.id), -p.supply),
                    crate::value::posting(acct(&a.id, "passes"), pass_unit(&a.id), p.supply),
                ]}).unwrap();
            }
        }
        let home = self.league.tonight.home.clone();
        let away = self.league.tonight.away.clone();
        let hs = self.league.team(&home).and_then(|g| g.roster.iter().find(|m| m.star)).map(|m| (m.id.clone(), m.name.clone())).unwrap();
        let as_ = self.league.team(&away).and_then(|g| g.roster.iter().find(|m| m.star)).map(|m| (m.id.clone(), m.name.clone())).unwrap();
        let season = [
            (format!("{} · 41-point night", last_name(&hs.1)), home.clone(), Some(hs.0.clone()), 120, "Uncommon"),
            (format!("{} · buzzer-beater", last_name(&as_.1)), away.clone(), Some(as_.0.clone()), 60, "Rare"),
            (format!("{} · 18–0 run", self.league.team(&home).unwrap().short), home.clone(), None, 25, "Legendary"),
        ];
        let cards_on = self.on("cards") && self.concert().is_none() && n > 0 && !self.cfg.real;
        for (k, (title, team, party, supply, rarity)) in season.into_iter().enumerate().filter(|_| cards_on) {
            let id = format!("s{k}");
            self.social.mint(&mut self.ledger, Edition { id: id.clone(), kind: "season".into(), title, team, party, t: 0, supply, rarity: rarity.into(), open_until: -1 }).unwrap();
            for _ in 0..(supply * 7 / 10) {
                let f = self.fans[self.rng.below(n as u64) as usize].id.clone();
                self.social.grant(&mut self.ledger, &f, &id).unwrap();
                if self.rng.chance(0.25) { let price = price_for(rarity, &mut self.rng); let _ = self.list_card(&f, &id, price); }
            }
        }
        // a real night starts with nothing made up: no sponsor offers or player deals until real ones are made
        if self.on("offers") && !self.cfg.real { self.seed_offers(); }
        if self.on("players") && !self.cfg.real { self.seed_deals(); }
        self
    }

    /// A concert rather than a game.
    pub fn concert(&self) -> Option<&crate::world::EventDef> { self.world.def.event.as_ref().filter(|e| e.kind == "concert") }
    /// Which act is on: 0 opener, 1 support, 2 headliner (periods 3 and 4).
    pub fn act_on(&self, period: i64) -> Option<String> { self.concert().and_then(|c| c.acts.get(((period - 1).clamp(0, 2)) as usize).cloned()) }

    /// Whether the venue has this switched on. Anything it hasn't mentioned is on.
    pub fn on(&self, feature: &str) -> bool { self.world.def.features.get(feature).copied().unwrap_or(true) }

    /// A moment becomes a card with a fixed supply, collectable for five minutes.
    fn mint_moment(&mut self, kind: &str, side: &str, scorer: &str) -> Option<String> {
        if !self.on("cards") { return None; }
        if let Some(c) = self.concert().cloned() {
            // only songs become cards at a show: not the game's runs, leads or last call
            if kind != "encore" && kind != "favorite" { return None; }
            // a song the whole building sings becomes a card; the encore is the rarest
            let (supply, rarity) = if kind == "encore" { (80, "Rare") } else { (180, "Uncommon") };
            let head = c.acts.get(2).cloned().unwrap_or_default();
            let id = format!("m{}", self.social.editions.len());
            let title = format!("{head} · {scorer}");
            let group = self.group_of("home");
            self.social.mint(&mut self.ledger, crate::social::Edition { id: id.clone(), kind: kind.into(), title, team: group, party: None, t: self.t,
                supply, rarity: rarity.into(), open_until: self.t + 5 * 60_000 }).ok()?;
            self.open_drop(&id);
            return Some(id);
        }
        let (supply, rarity) = match kind { "splash" => (60, "Rare"), "run" => (150, "Uncommon"), "lead_change" => (200, "Common"), _ => return None };
        let group = self.group_of(side);
        let who = self.league.member(scorer).map(|(_, m)| last_name(&m.name)).unwrap_or_default();
        let short = self.league.team(&group).map(|g| g.short.clone()).unwrap_or_default();
        let title = match kind { "splash" => format!("{who} · splash"), "run" => format!("{short} · run"), _ => format!("{who} · go-ahead bucket") };
        let id = format!("m{}", self.social.editions.len());
        let party = if kind == "run" { None } else { Some(scorer.to_string()) };
        self.social.mint(&mut self.ledger, crate::social::Edition { id: id.clone(), kind: kind.into(), title, team: group, party, t: self.t,
            supply, rarity: rarity.into(), open_until: self.t + 5 * 60_000 }).ok()?;
        self.open_drop(&id);
        Some(id)
    }

    /// The exact price of tickets before buying.
    /// Every section that sells tickets: the floor, then both levels.
    pub fn ticket_sections(&self) -> Vec<String> {
        let mut v = vec!["sFL".to_string()];
        for l in 1..=self.world.def.levels { for r in 0..self.world.def.sections_per_level { v.push(crate::world::section_id(l, r)); } }
        v
    }

    pub fn section_view(&self, sec: &str) -> &'static str {
        if sec == "sFL" { return "Courtside"; }
        let ring = sec[2..].parse::<u32>().unwrap_or(1) - 1;
        crate::services::section_view(ring, self.world.def.sections_per_level)
    }

    pub fn section_price(&self, sec: &str) -> i64 {
        let tier = |id: &str| self.league.tiers.iter().find(|t| t.id == id).map_or(5000, |t| t.price);
        if sec == "sFL" { return tier("courtside"); }
        let base = if sec.starts_with("s1") { tier("lower") } else { tier("upper") };
        crate::services::section_price(base, self.section_view(sec))
    }

    /// The exact price and the exact seats you'd get, before buying.
    pub fn ticket_quote(&self, event: &str, section: &str, qty: i64) -> Result<(crate::economy::TicketBreakdown, i64, Vec<String>), String> {
        if !(1..=8).contains(&qty) { return Err("Choose 1 to 8 tickets".into()); }
        self.league.upcoming.iter().find(|e| e.id == event).ok_or("Unknown event")?;
        let m = self.seatmaps.get(&(event.to_string(), section.to_string())).ok_or("Unknown section")?;
        let seats = m.best(qty as usize).map(|(r, s)| (0..qty as usize).map(|i| format!("{}{}", crate::services::SeatMap::label(r), s + i + 1)).collect()).unwrap_or_default();
        Ok((crate::economy::ticket_breakdown(self.section_price(section), qty, &self.terms), m.free() as i64, seats))
    }

    /// Seats a fan holds for an event, by section.
    pub fn seats_of(&self, fan: &str, event: &str) -> Vec<(String, Vec<String>)> {
        self.seatmaps.iter().filter(|((e, _), _)| e == event).filter_map(|((_, sec), m)| {
            let mine: Vec<String> = m.owner.iter().enumerate().filter(|(_, o)| o.as_deref() == Some(fan))
                .map(|(i, _)| format!("{}{}", crate::services::SeatMap::label(i / m.per_row), i % m.per_row + 1)).collect();
            if mine.is_empty() { None } else { Some((sec.clone(), mine)) }
        }).collect()
    }

    fn open_question(&mut self, ask: String, question: String, options: Vec<crate::rules::AskOption>, otherwise: Option<String>, locks_ms: i64, points: i64, t: i64) {
        let opts: Vec<(String, Option<String>, i64)> = options.into_iter()
            .map(|o| { let base = o.measure.as_ref().and_then(|m| self.engine.measure(m, t)).unwrap_or(0); (o.id, o.measure, base) }).collect();
        self.predictions.push(Prediction { ask: ask.clone(), question, options: opts.clone(), otherwise, points, opens: t, locks: t + locks_ms, picks: BTreeMap::new(), result: None });
        // the crowd picks, each pick a signal like anyone else's
        let crowd: Vec<String> = self.fans.iter().filter(|f| !self.live.contains(&f.id)).map(|f| f.id.clone()).collect();
        for f in crowd {
            if self.rng.chance(0.25) { let k = if self.rng.chance(0.6) { 0 } else { 1 + self.rng.below((opts.len() - 1) as u64) as usize }; self.social_act(&f, "pick", &ask, &opts[k].0); }
        }
    }

    fn settle_question(&mut self, ask: &str) {
        let t = self.t;
        let Some(i) = self.predictions.iter().rposition(|p| p.ask == ask && p.result.is_none()) else { return };
        let p = &self.predictions[i];
        let grew: Vec<(String, i64)> = p.options.iter().filter_map(|(id, m, base)| m.as_ref().map(|m| (id.clone(), self.engine.measure(m, t).unwrap_or(0) - base))).collect();
        let best = grew.iter().map(|g| g.1).max().unwrap_or(0);
        let leaders: Vec<&(String, i64)> = grew.iter().filter(|g| g.1 == best).collect();
        let result = if best > 0 && leaders.len() == 1 { leaders[0].0.clone() } else if best <= 0 { p.otherwise.clone().unwrap_or_else(|| "none".into()) } else { "tie".into() };
        let winners: Vec<String> = p.picks.iter().filter(|(_, v)| **v == result).map(|(k, _)| k.clone()).collect();
        let pts = crate::value::Unit::Points("agape".into());
        let (n, memo) = (p.points, format!("prediction {}", p.question));
        for w in winners {
            self.ledger.post(crate::value::Txn { memo: memo.clone(), postings: vec![
                crate::value::posting(acct("agape", "issued"), pts.clone(), -n),
                crate::value::posting(acct(&w, "points"), pts.clone(), n),
            ]}).expect("issuer may go negative");
        }
        self.predictions[i].result = Some(result);
    }

    /// When ordering closes for the night.
    pub fn script_final(&self) -> i64 { self.script.iter().find(|x| x.1 == "game.final").map(|x| x.0 * 60_000).unwrap_or(self.end_min * 60_000) }

    /// Scheduled moments still ahead (hints from the event's script): (time, kind).
    pub fn later(&self) -> Vec<(i64, &'static str)> {
        let at = |k: &str, n: i64| self.script.iter().find(|x| x.1 == k && x.2 == n).map(|x| x.0 * 60_000).unwrap_or(0);
        let mut v = vec![(at("period.start", 1), "start"), (at("period.end", 2), "halftime"), (at("period.start", 3), "second_half"),
                         (at("period.start", 4), "last_call"), (at("game.final", 0), "final")];
        // a real game's moments not yet marked have no time to aim for
        v.retain(|x| x.0 > self.t && x.0 < NOT_YET * 60_000);
        v
    }

    pub fn you_cancel(&mut self, order: usize) -> Result<(), String> { self.cancel_as(YOU, order) }
    pub fn cancel_as(&mut self, who: &str, order: usize) -> Result<(), String> {
        let o = self.orders.get_mut(order).ok_or("unknown order")?;
        if o.c.quote.fan != who { return Err("not yours".into()); }
        o.c.act(Action::Cancel, who, &mut self.ledger, &self.terms).map_err(|e| format!("{e:?}"))?;
        for q in self.queues.values_mut() { q.retain(|&i| i != order); }
        Ok(())
    }

    /// Rule outputs become moments, alerts and offers.
    pub(crate) fn absorb(&mut self, outs: Vec<Output>) {
        let t = self.t;
        for o in outs {
            match o {
                Output::Emitted { kind, fields, .. } if kind == "moment" => {
                    let (k, team) = (fields["kind"].key(), fields["team"].key());
                    self.report.moments.push((t / 60_000, k.clone(), team.clone()));
                    let scorer = self.last_scorer.get(&team).cloned().unwrap_or_default();
                    let edition = self.mint_moment(&k, &team, &scorer);
                    // a run belongs to the team; a splash or a lead change to whoever made it
                    let party = if k == "run" { self.group_of(&team) } else { scorer };
                    self.note_about("moment", &team, k, &party, edition);
                }
                Output::Emitted { kind, fields, .. } if kind == "ops.alert" => { self.report.alerts += 1; self.note("alert", "", fields["kind"].key()); }
                Output::Ask { ask, question, options, otherwise, locks_ms, points, .. } if self.on("predictions") => self.open_question(ask, question, options, otherwise, locks_ms, points, t),
                Output::Settle { ask, .. } => self.settle_question(&ask),
                Output::Offer { rule, thing, price, discount, payer, expires_ms, limits, .. } if self.on("offers") => {
                    let mut limits = limits.clone();
                    if let Some(m) = self.cfg.force_mode { limits.mode = m; if m == Mode::Guaranteed { limits.tiers.clear(); } }
                    let spec = OfferSpec { source: &rule, thing: &thing, price, discount, payer: payer.as_deref(), expires_ms: expires_ms.or(Some(600_000)), t_ms: t, pushed: true };
                    let (stations, queues, orders) = (&self.stations, &self.queues, &self.orders);
                    let wait = |st: &str| stand_wait(stations, queues, orders, st, t);
                    if self.book.open(&self.world, &mut self.ledger, &self.engine, &spec, &limits, &self.fans, &wait, &mut self.attr).is_some() {
                        self.report.offers_opened += 1;
                        let name = self.world.thing(&thing).map(|x| x.name.clone()).unwrap_or_default();
                        self.note("offer", "", name);
                    }
                }
                _ => {}
            }
        }
    }

    /// Advance one tick (10 s). Returns false once the night is over and every order has closed.
    pub fn step(&mut self) -> bool {
        if self.done { return false; }
        let t = self.t;
        let in_flight = self.orders.iter().any(|o| !matches!(o.c.stage, Stage::Settled | Stage::Closed(_)))
            || self.trade.of_kind(crate::exchange::Kind::Ride).any(|r| r.live());
        if t > self.end_min * 60_000 && (!in_flight || t > (self.end_min + 30) * 60_000) { self.done = true; return false; }
        let (ph, rate) = phase_of(&self.script, t / 60_000);
        let sim = !self.cfg.real;
        let mut outs: Vec<Output> = vec![];
        self.ledger.now = t;
        if self.terms.hourly() && t % 60_000 == 0 { self.pay_wages(); }
        // 1. the game itself
        for (m, kind, k) in self.script.clone() {
            // a real night's moments are taken in when the venue's team marks them (mark_game)
            if m * 60_000 == t && (sim || m == 0) {
                let f = if kind == "gates.open" || kind == "game.final" { fields!{} } else { fields!{"n" => k} };
                let by = if kind == "gates.open" { "venue" } else { "keeper" };
                outs.extend(self.engine.ingest(&mut self.log, kind, f, by, t).unwrap());
                let text = match (kind, self.act_on(k)) {
                    ("gates.open", Some(_)) => "Doors open".to_string(),
                    ("game.final", Some(_)) => "That's the show. Thank you, Harbor!".to_string(),
                    ("period.start", Some(a)) => if k == 4 { format!("{a}, back for the big finish") } else { format!("{a} takes the stage") },
                    ("period.end", Some(a)) => if k == 3 { format!("{a}: a short break") } else { format!("{a} leaves the stage") },
                    ("gates.open", None) => "Gates open".to_string(), ("game.final", None) => "Final".to_string(),
                    ("period.start", None) => format!("Q{k} underway"), _ => format!("End of Q{k}") };
                self.note("game", "", text);

            }
        }
        if !sim {
            // a real game: the venue's team marks it and keeps the score (mark_game, keep_score)
        } else if let Some(c) = self.concert().cloned() {
            // the music: a new song every few minutes; the crowd's favorites and the encore become moments
            let period = self.script.iter().filter(|x| x.1 == "period.start" && x.0 * 60_000 <= t).map(|x| x.2).max().unwrap_or(0);
            let encore_at = self.script.iter().find(|x| x.1 == "period.end" && x.2 == 4).map(|x| (x.0 - 6) * 60_000).unwrap_or(i64::MAX);
            if ph == "in play" && t == encore_at {
                let song = c.songs.last().cloned().unwrap_or_else(|| "the encore".into());
                let ed = self.mint_moment("encore", "home", &song);
                self.note_about("moment", "home", "encore".into(), "", ed);
                self.note("song", "home", format!("Encore: {song}"));
            } else if ph == "in play" && t < encore_at && self.rng.chance(if period >= 3 { 0.03 } else { 0.045 }) && !(period >= 3 && self.songs_played + 1 >= c.songs.len().max(1)) {
                let act = self.act_on(period).unwrap_or_default();
                let song = if period >= 3 && !c.songs.is_empty() {
                    let i = self.songs_played; self.songs_played += 1; c.songs[i].clone()
                } else {
                    let n = self.feed.iter().filter(|f| f.kind == "song" && f.text.starts_with(&act)).count();
                    let k = self.feed.iter().filter(|f| f.kind == "song").count();
                    match c.early.get(k % c.early.len().max(1)) { Some(x) => format!("{act}: {x}"), None => format!("{act}: song {}", n + 1) } };
                let fav = c.favorites.contains(&song);
                if period >= 3 { self.note("song", "home", format!("Now playing: {song}")); } else { self.note("song", "home", song.clone()); }
                if fav {
                    let ed = self.mint_moment("favorite", "home", &song);
                    self.note_about("moment", "home", "favorite".into(), "", ed);
                }
            }
        } else if ph == "in play" && self.rng.chance(0.14) {
            let home = self.rng.chance(0.51);
            let pts = [1, 2, 2, 2, 2, 3, 3][self.rng.below(7) as usize];
            if home { self.score.0 += pts } else { self.score.1 += pts }
            let team = if home { "home" } else { "away" };
            // who scored: the star about a third of the time, otherwise anyone on the floor
            let gid = self.group_of(team);
            let roster = self.league.team(&gid).map(|g| g.roster.clone()).unwrap_or_default();
            let m = if self.rng.chance(0.32) { roster.iter().find(|m| m.star).cloned() } else { None }
                .unwrap_or_else(|| roster[self.rng.below(roster.len() as u64) as usize].clone());
            self.last_scorer.insert(team.to_string(), m.id.clone());
            outs.extend(self.engine.ingest(&mut self.log, "score", fields!{"team" => team, "points" => pts, "player" => m.id.as_str(), "star" => m.star}, "keeper", t).unwrap());
            self.note_about("score", team, format!("+{pts}"), &m.id, None);
        }
        // the crowd talks, more when something happens
        if sim && t <= self.end_min * 60_000 && ph != "postgame" && self.on("chat") {
            let last_moment = self.feed.iter().rev().find(|f| f.kind == "moment" && t - f.t < 60_000).map(|f| (f.text.clone(), f.team.clone()));
            // a few voices at a time, never the same line twice in a row of recent chat
            let n = 1 + if last_moment.is_some() { 3 } else { 0 };
            for _ in 0..n {
                if !self.rng.chance(0.45) { continue; }
                let fan = self.fans[self.rng.below(sim_fans_n(&self.cfg) as u64) as usize].clone();
                let pool = if self.concert().is_some() { if last_moment.is_some() { CHAT_SONG } else { CHAT_SHOW } }
                    else if ph == "pregame" { CHAT_PRE }
                    else { match &last_moment { Some((_, team)) if team == "home" => CHAT_RUN, Some((_, team)) if team == "away" => CHAT_AWAY, _ => CHAT } };
                let text = pool[self.rng.below(pool.len() as u64) as usize];
                if self.chat.iter().rev().take(12).any(|c| c.3 == text) { continue; }
                let text = self.crowd_words(text);
                self.post_chat(&fan.id, &fan.seat, &text);
            }
        }
        // the crowd unlocks perks around the arena with their points (more of them for a business a
        // player has promoted tonight), and after the final walks over and uses them
        {
            let perks: Vec<(String, Option<String>)> = self.world.def.around.iter().filter(|a| a.perk.is_some()).map(|a| (a.id.clone(), a.operator.clone())).collect();
            if sim && !perks.is_empty() && self.cfg.fans > 0 && self.on("around") {
                if ph != "postgame" && t <= self.end_min * 60_000 {
                    let (place, op) = perks[self.rng.below(perks.len() as u64) as usize].clone();
                    let promoted = op.as_ref().map_or(false, |o| self.trade.of_kind(crate::exchange::Kind::Deal).any(|d| d.buyer == *o && d.stage == crate::exchange::Stage::Settled));
                    if self.rng.chance(if promoted { 0.45 } else { 0.18 }) {
                        let f = self.fans[self.rng.below(self.cfg.fans as u64) as usize].id.clone();
                        let _ = self.unlock(&f, &place);
                    }
                } else {
                    for (place, op) in perks {
                        if !op.as_ref().map_or(true, |o| self.decides(o)) || !self.rng.chance(0.5) { continue; }
                        let unit = pass_unit(&place);
                        let holder = self.ledger.balances_iter().find(|((a, u), v)| a.purpose == "holdings" && *u == unit && **v > 0 && !self.live.contains(&a.party)).map(|((a, _), _)| a.party.clone());
                        if let Some(f) = holder { let code = crate::dealing::pass_code(&f, &place); let _ = self.redeem(&place, &code); }
                    }
                }
            }
        }
        // the crowd collects open drops and trades cards
        if sim && self.cfg.fans > 0 && t <= self.end_min * 60_000 && self.on("cards") {
            let open: Vec<(String, i64)> = self.social.editions.values().filter(|e| e.open_until >= t).map(|e| (e.id.clone(), e.supply)).collect();
            for (id, supply) in open {
                let k = self.rng.below((supply / 12).max(2) as u64);
                for _ in 0..k {
                    let f = self.fans[self.rng.below(self.cfg.fans as u64) as usize].id.clone();
                    let _ = self.accept(&format!("drop:{id}"), &f, 1, None, "", "live");
                }
            }
            for _ in 0..2 {
                let f = self.fans[self.rng.below(self.cfg.fans as u64) as usize].id.clone();
                let held: Vec<String> = self.social.editions.keys().filter(|e| self.ledger.balance(&acct(&f, "holdings"), &crate::social::card_unit(e)) > 0).cloned().collect();
                if let Some(e) = held.first() {
                    if self.rng.chance(0.3) {
                        let rarity = self.social.editions[e].rarity.clone();
                        let price = price_for(&rarity, &mut self.rng);
                        let _ = self.list_card(&f, e, price);
                    }
                }
                let ids: Vec<String> = self.trade.offers().filter(|o| o.kind == crate::exchange::Kind::Card && o.open).map(|o| o.id.clone()).collect();
                if self.rng.chance(0.25) && !ids.is_empty() {
                    let l = ids[self.rng.below(ids.len() as u64) as usize].clone();
                    let b = self.fans[self.rng.below(self.cfg.fans as u64) as usize].id.clone();
                    let _ = self.accept(&l, &b, 1, None, "", "market");
                }
            }
        }
        if sim && self.cfg.fans > 0 && t <= self.end_min * 60_000 && self.on("tickets") && self.rng.chance(0.08) && !self.league.upcoming.is_empty() {
            let e = self.league.upcoming[self.rng.below(self.league.upcoming.len() as u64) as usize].id.clone();
            let secs = self.ticket_sections();
            let sec = secs[self.rng.below(secs.len() as u64) as usize].clone();
            let f = self.fans[self.rng.below(self.cfg.fans as u64) as usize].id.clone();
            let q = 1 + self.rng.below(3) as i64;
            let _ = self.buy_tickets(&f, &e, &sec, q);
        }
        // some of the crowd books rides home around the final
        let fin = self.script_final();
        if sim && self.cfg.fans > 0 && t > fin - 40 * 60_000 && t < fin && self.on("around") && self.rng.chance(0.15) {
            let f = self.fans[self.rng.below(self.cfg.fans as u64) as usize].id.clone();
            let at = fin + self.rng.below(15) as i64 * 60_000;
            let _ = self.book_ride(&f, "ride-b", at);
        }
        self.advance_trade();
        self.advance_deals();
        // 2. menu orders from simulated fans
        let sim_fans = self.cfg.fans;
        let expected = rate * sim_fans as f64 * (TICK as f64 / 60_000.0);
        let count = expected.floor() as u64 + self.rng.chance(expected.fract()) as u64;
        let mut placed: Vec<(Commitment, String)> = vec![];
        for _ in 0..count {
            let fan = self.fans[self.rng.below(sim_fans as u64) as usize].clone();
            let thing = self.world.thing(&self.menu[self.rng.below(self.menu.len() as u64) as usize]).unwrap().clone();
            if self.world.eligible(&fan, &thing.id, &|s| self.engine.span_open(s), false).is_err() || self.closed_now(&thing.id).is_some() { continue; }
            let qty = 1 + self.rng.chance(0.3) as i64;
            if self.ledger.balance(&acct(&thing.stand, "inventory"), &Unit::Thing(thing.id.clone())) < qty { self.report.menu_sold_out += 1; continue; }
            let delivery = self.rng.chance(self.cfg.delivery_share);
            let tip = if delivery && self.rng.chance(0.6) { [100, 200, 300, 500][self.rng.below(4) as usize] } else { 0 };
            let q = Quote { fan: fan.id.clone(), stand: thing.stand.clone(), runner: None, lines: vec![Line { thing: thing.id.clone(), qty, unit_price: thing.price }],
                            discounts: vec![], points_redeemed: 0, tip, delivery, fee_waived: false, runner_pay: None };
            if breakdown(&q, &self.terms).map_or(true, |b| b.charge > fan.ceiling * 2) { continue; }
            let wait = self.stand_wait(&thing.stand);
            if wait > 1200 && self.rng.chance(((wait - 1200) as f64 / 1200.0).min(0.9)) { self.report.balked += 1; continue; }
            match Commitment::accept(&format!("o{}", self.orders.len() + placed.len()), q, &mut self.ledger) {
                Ok(c) => placed.push((c, "menu".into())),
                Err(_) => self.report.menu_sold_out += 1,
            }
        }
        // 3. offers: close what ended, pause busy stands, retry pending, free lapsed claims, simulated fans tap
        self.book.refresh(&mut self.ledger, &self.engine, t);
        let busy: BTreeSet<String> = self.world.def.stands.iter().map(|st| st.id.clone())
            .filter(|st| self.stand_wait(st) > self.world.def.service.max_eta_s as i64).collect();
        self.book.busy = busy;
        {
            let (stations, queues, orders) = (&self.stations, &self.queues, &self.orders);
            let wait = |st: &str| stand_wait(stations, queues, orders, st, t);
            self.report.offers_opened += self.book.retry_pending(&self.world, &mut self.ledger, &self.engine, &self.fans, &wait, &mut self.attr, t);
        }
        self.report.claims_lapsed += self.book.expire_claims(&mut self.ledger, t);
        let live: Vec<String> = self.book.offers.values().filter(|l| l.closed.is_none()).map(|l| l.id.clone()).collect();
        if t <= self.end_min * 60_000 {
            for oid in live {
                let audience: Vec<String> = self.book.offers[&oid].audience.iter().filter(|f| !self.live.contains(*f)).cloned().collect();
                let probe = self.book.disclose(&oid, "", &self.ledger).unwrap();
                let urgent = !probe.guaranteed && (probe.left.map_or(false, |n| n <= 20) || probe.ends_at.map_or(false, |e| e - t <= 180_000));
                let p = self.cfg.tap_rate * if urgent { self.cfg.urgency_lift } else { 1.0 };
                for f in audience {
                    if !self.rng.chance(p) { continue; }
                    let d = self.book.disclose(&oid, &f, &self.ledger).unwrap();
                    if d.ended.is_some() { self.report.saw_it_end += 1; continue; }
                    let fan = self.fans.iter().find(|x| x.id == f).unwrap().clone();
                    match self.book.tap(&oid, &fan, &self.world, &mut self.ledger, &self.engine, t) {
                        Ok(c) => {
                            self.report.taps += 1;
                            if self.rng.chance(0.9) { let at = t + 20_000 + self.rng.below(70_000) as i64; self.checkouts.push((at, c.id.clone())); }
                        }
                        Err(TapError::PerFanLimit) | Err(TapError::NotEligible) => {}
                        Err(_) => self.report.tap_failed_after_live_card += 1,
                    }
                }
            }
        }
        self.checkouts.sort();
        while self.checkouts.first().map_or(false, |c| c.0 <= t) {
            let (_, claim) = self.checkouts.remove(0);
            let tip = if self.rng.chance(0.5) { 200 } else { 0 };
            match self.book.checkout(&claim, &format!("o{}", self.orders.len() + placed.len()), t, tip) {
                Ok((c, source)) => { self.attr.accepted(&source); placed.push((c, source)); }
                Err(_) => self.report.checkout_failed += 1,
            }
        }
        for (c, source) in placed { self.place(c, source, &mut outs); }
        self.placed_by_phase.entry(ph).or_insert((0, 0)).1 += TICK;
        // 4. rule outputs
        self.absorb(outs);
        // 5. simulated fans cancelling while still waiting
        let mut cancel = vec![];
        for q in self.queues.values() { for &id in q { if !self.live.contains(&self.orders[id].c.quote.fan) && self.rng.chance(0.001) { cancel.push(id); } } }
        for id in cancel {
            let f = self.orders[id].c.quote.fan.clone();
            self.orders[id].c.act(Action::Cancel, &f, &mut self.ledger, &self.terms).unwrap();
            for q in self.queues.values_mut() { q.retain(|&i| i != id); }
        }
        // orders placed ahead join their stand's queue when it's time to start them
        let due: Vec<usize> = self.scheduled.iter().filter(|(s, _)| *s <= t).map(|(_, i)| *i).collect();
        self.scheduled.retain(|(s, _)| *s > t);
        for id in due {
            if self.orders[id].c.stage == Stage::Accepted {
                let stand = self.orders[id].c.quote.stand.clone();
                self.queues.entry(stand).or_default().insert(0, id);
            }
        }
        // 6. stations: finish, then start
        let stands: Vec<String> = self.stations.keys().cloned().collect();
        for stand in stands {
            for si in 0..self.stations[&stand].len() {
                let st = self.stations[&stand][si];
                if let (busy, Some(_)) = st {
                    // a stand run by people marks its own food ready; the simulation only finishes its own
                    if busy <= t && !self.human(&stand) { self.finish_station(&stand, si); }
                }
                if self.stations[&stand][si].1.is_none() {
                    let q = self.queues.entry(stand.clone()).or_default();
                    if !q.is_empty() {
                        let id = q.remove(0);
                        if !self.live.contains(&self.orders[id].c.quote.fan) && !self.human(&stand) && self.rng.chance(0.008) {
                            self.orders[id].c.act(Action::CannotFulfil, &stand, &mut self.ledger, &self.terms).unwrap();
                            continue;
                        }
                        self.orders[id].c.act(Action::StartPrep, &stand, &mut self.ledger, &self.terms).unwrap();
                        self.orders[id].started_at = Some(t);
                        let until = t + self.orders[id].prep_s as i64 * 1000;
                        self.stations.get_mut(&stand).unwrap()[si] = (until, Some(id));
                    }
                }
            }
        }
        self.flag_late();
        self.settle_priority();
        // 7. dispatch (trips offered to people and not taken in time go back on the shelf first)
        let lapsed: Vec<String> = self.trips.iter().filter(|(_, x)| !x.accepted && t - x.offered_at >= TRIP_OFFER_MS).map(|(k, _)| k.clone()).collect();
        for r in lapsed { self.return_trip(&r, t + 30_000); }
        if !self.ready.is_empty() {
            // a runner run by a person gets trips only while on shift, and is offered them first; the simulation never runs their trips
            let pool: Vec<crate::engines::Runner> = self.runners.iter().filter(|r| !self.driven.contains(&r.id) || self.live.contains(&r.id)).cloned().collect();
            let (people, sim): (Vec<_>, Vec<_>) = pool.into_iter().partition(|r| self.live.contains(&r.id));
            let mut assigns = if people.is_empty() { vec![] } else { dispatch(&self.world, &mut self.costs, &self.ready, &people, t) };
            let taken: BTreeSet<String> = assigns.iter().flat_map(|a| a.orders.clone()).collect();
            let rest: Vec<crate::engines::Ready> = self.ready.iter().filter(|r| !taken.contains(&r.order)).cloned().collect();
            assigns.extend(dispatch(&self.world, &mut self.costs, &rest, &sim, t));
            for a in assigns {
                let ids: Vec<usize> = a.orders.iter().map(|s| s.parse().unwrap()).collect();
                let (pay, first, passes) = self.trip_pay(&ids, a.done_s);
                self.pay_stats.0 += 1;
                if self.human(&a.runner) {
                    // a person is offered the trip first, with its route and pay; nothing is theirs until they say yes
                    let stand = self.orders[ids[0]].c.quote.stand.clone();
                    self.ready.retain(|x| !a.orders.contains(&x.order));
                    if let Some(r) = self.runners.iter_mut().find(|r| r.id == a.runner) { r.free_at = i64::MAX; }
                    self.trips.insert(a.runner.clone(), Trip { orders: ids, stand, offered_at: t, accepted: false, picked: false, pay, first, passes,
                        route: a.route.clone(), pickup_s: a.pickup_at_s, pickups: a.pickups.clone(), arrive_s: a.arrive_s.clone(), done_s: a.done_s });
                    continue;
                }
                // a simulated runner weighs the offer (pay and tips for the time it takes) against what their time is worth to them
                let tips: i64 = ids.iter().map(|&i| self.orders[i].c.quote.tip).sum();
                let per_hour = (pay + tips) * 3600 / (a.done_s as i64).max(120);
                let ratio = per_hour as f64 / reservation(&a.runner) as f64;
                // a runner paid by the hour takes the trips they're given
                if !self.terms.hourly() && !self.rng.chance(((ratio - 0.55) / 0.5).clamp(0.03, 0.97)) {
                    // passed: it stays on the shelf for the next runner, at a higher offer; this one rests a moment
                    for &i in &ids { self.orders[i].passes += 1; }
                    if let Some(r) = self.runners.iter_mut().find(|r| r.id == a.runner) { r.free_at = t + 20_000; }
                    continue;
                }
                self.took_trip(&ids, pay, passes);
                let r = self.runners.iter_mut().find(|r| r.id == a.runner).unwrap();
                let mut plan = vec![(t, r.at.clone())];
                plan.extend(a.pickups.iter().map(|(s, at)| (t + *at as i64 * 1000, s.clone())));
                r.free_at = t + a.done_s as i64 * 1000;
                r.at = a.route.last().unwrap().clone();
                let ids: Vec<usize> = a.orders.iter().map(|s| s.parse().unwrap()).collect();
                for (k, &id) in ids.iter().enumerate() {
                    self.orders[id].c.act(Action::Assign(a.runner.clone()), "agape", &mut self.ledger, &self.terms).unwrap();
                    self.orders[id].eta = t + a.arrive_s[k] as i64 * 1000;
                    plan.push((self.orders[id].eta, self.orders[id].seat.clone()));
                }
                self.plans.insert(a.runner.clone(), plan);
                self.ready.retain(|x| !a.orders.contains(&x.order));
                self.batches.0 += 1; self.batches.1 += ids.len() as u64;
                // picked up stand by stand, each when the runner gets there
                for (stand, at) in &a.pickups {
                    let here: Vec<usize> = ids.iter().copied().filter(|&i| &self.orders[i].c.quote.stand == stand).collect();
                    self.sched(t + *at as i64 * 1000, Ev::PickUp(here, a.runner.clone()));
                }
                for (k, &id) in ids.iter().enumerate() { self.sched(t + a.arrive_s[k] as i64 * 1000, Ev::Arrive(id, a.runner.clone())); }
            }
        }
        // 7b. staging: simulated runners left idle walk to the stands about to need them
        if self.cfg.staging {
            let mut due: Vec<(String, i64)> = vec![];
            for (stand, sts) in &self.stations {
                for (until, id) in sts { if let Some(i) = id { if self.orders[*i].c.quote.delivery && *until <= t + 600_000 { due.push((stand.clone(), *until)); } } }
            }
            let idle: Vec<crate::engines::Runner> = self.runners.iter().filter(|r| r.free_at <= t && !self.driven.contains(&r.id) && !self.live.contains(&r.id)).cloned().collect();
            for (rid, stand, walk) in crate::engines::stage(&self.world, &mut self.costs, &idle, &due, t) {
                if let Some(r) = self.runners.iter_mut().find(|r| r.id == rid) {
                    self.plans.insert(rid.clone(), vec![(t, r.at.clone()), (t + walk as i64 * 1000, stand.clone())]);
                    r.at = stand; r.free_at = t + walk as i64 * 1000;
                }
            }
        }
        // 8. scheduled events due by now
        self.events.sort_by_key(|e| (e.0, e.1));
        while self.events.first().map_or(false, |e| e.0 <= t) {
            let (_, _, e) = self.events.remove(0);
            match e {
                Ev::PickUp(ids, r) => for id in ids { self.orders[id].c.act(Action::PickUp, &r, &mut self.ledger, &self.terms).unwrap(); self.orders[id].picked_at = Some(t); },
                Ev::Arrive(id, r) => {
                    if !self.live.contains(&self.orders[id].c.quote.fan) && self.rng.chance(0.01) {
                        let o = &mut self.orders[id];
                        o.c.act(Action::Unreachable, &r, &mut self.ledger, &self.terms).unwrap();
                        let mut q = o.c.quote.clone(); q.tip = 0;
                        let b = breakdown(&q, &self.terms).unwrap();
                        self.attr.settled(&o.source.clone(), &q, &b);
                        continue;
                    }
                    let o = &mut self.orders[id];
                    if t > o.promise { if o.c.late_by.is_none() { o.c.late_by = Some(("agape".into(), (t - o.promise) / 1000)); } self.report.late += 1; }
                    o.c.act(Action::Deliver, &r, &mut self.ledger, &self.terms).unwrap();
                    o.handoff_m = Some(2 + (id as i64 * 7919) % 14);
                    o.delivered_at = Some(t);
                    self.sched(t + 1000, Ev::Settle(id));
                }
                Ev::FanCollects(id) => {
                    // the stand may already have handed it over by hand
                    if self.orders[id].c.stage != Stage::Ready { continue; }
                    let stand = self.orders[id].c.quote.stand.clone();
                    self.orders[id].c.act(Action::Deliver, &stand, &mut self.ledger, &self.terms).unwrap();
                    self.orders[id].delivered_at = Some(t);
                    self.sched(t + 1000, Ev::Settle(id));
                }
                Ev::Settle(id) => self.settle_now(id),
            }
        }
        while self.recent.front().map_or(false, |r| t - r.0 > 300_000) { self.recent.pop_front(); }
        self.t += TICK;
        true
    }

    /// Read the night back from the ledger.
    pub fn finish(&mut self) -> Report {
        let mut report = std::mem::take(&mut self.report);
        let ledger = &self.ledger;
        let usd = Unit::usd();
        let mut waits: Vec<i64> = vec![];
        for o in &self.orders {
            let k = match &o.c.stage { Stage::Closed(e) => format!("{e:?}").split([' ', '{']).next().unwrap().to_string(), s => format!("{s:?}") };
            *report.by_stage.entry(k).or_insert(0) += 1;
            if o.c.quote.delivery { if let Some(d) = o.delivered_at { waits.push((d - o.accepted_at) / 1000); } }
        }
        let orders = &self.orders;
        let avg = |f: &dyn Fn(&Order) -> Option<i64>| { let v: Vec<i64> = orders.iter().filter_map(|o| f(o)).collect(); if v.is_empty() { 0 } else { v.iter().sum::<i64>() / v.len() as i64 / 1000 } };
        report.avg_queue_s = avg(&|o| Some(o.started_at? - o.accepted_at));
        report.avg_prep_s = avg(&|o| Some(o.ready_at? - o.started_at?));
        report.avg_dispatch_s = avg(&|o| Some(o.picked_at? - o.ready_at?));
        report.avg_walk_s = avg(&|o| Some(o.delivered_at? - o.picked_at?));
        report.drained_min = self.t / 60_000 - self.end_min;
        waits.sort();
        if !waits.is_empty() { report.wait_p50_s = waits[waits.len() / 2]; report.wait_p90_s = waits[waits.len() * 9 / 10]; }
        let sum = |purpose: &str, prefix: &str| ledger_sum(ledger, purpose, prefix);
        report.money.insert("fans charged".into(), -sum("card", "fan") - sum("card", YOU));
        report.money.insert("stands net".into(), sum("revenue", "") - ledger.balance(&acct("agape", "revenue"), &usd));
        report.money.insert("agape net".into(), ledger.balance(&acct("agape", "revenue"), &usd));
        report.money.insert("runner pay".into(), sum("pay", "runner"));
        report.money.insert("runner tips".into(), sum("tips", "runner"));
        report.money.insert("tax".into(), ledger.balance(&acct("tax_authority", "payable"), &usd));
        report.money.insert("processing".into(), ledger.balance(&acct("processor", "fees"), &usd));
        report.money.insert("agape processing".into(), ledger.balance(&acct("agape", "processing"), &usd));
        let stand_ids: Vec<String> = self.world.def.stands.iter().map(|s| s.id.clone()).collect();
        let runner_ids: Vec<String> = self.runners.iter().map(|r| r.id.clone()).collect();
        report.flows = money_flows(ledger, &stand_ids, &runner_ids);
        for s in ["sponsor_cola", "sponsor_water"] {
            let funded = if s == "sponsor_cola" { 400_00 } else { 300_00 };
            report.sponsor_spend_ledger.insert(s.into(), funded - ledger.balance(&acct(s, "promo_budget"), &usd) - ledger.balance(&acct(s, "promo_escrow"), &usd) - ledger.balance(&acct(s, "promised_budget"), &usd));
        }
        report.money.insert("sponsor spend".into(), report.sponsor_spend_ledger.values().sum());
        report.open_holds = ledger_sum(ledger, "promo_escrow", "") + ledger_sum(ledger, "promised_budget", "")
            + ledger.balances_iter().filter(|((a, _), v)| (a.purpose == "points_held" || a.purpose == "promised" || a.purpose == "reserved") && **v != 0).count() as i64;
        report.conserved = ledger.units().iter().all(|u| ledger.total(u) == 0);
        report.attribution = std::mem::take(&mut self.attr.by_source);
        report.final_score = self.score;
        report.orders_per_min = self.placed_by_phase.iter().map(|(k, (n, ms))| (k.to_string(), *n as f64 / (*ms as f64 / 60_000.0))).collect();
        let hours = self.end_min as f64 / 60.0;
        report.runner_hourly = ((report.money["runner pay"] + report.money["runner tips"]) as f64 / self.cfg.runners as f64 / hours) as i64;
        report.per_runner_hour = self.batches.1 as f64 / self.cfg.runners.max(1) as f64 / hours;
        let done: Vec<i64> = self.orders.iter().filter(|o| o.c.quote.delivery).filter_map(|o| o.delivered_at).collect();
        let peak = done.iter().map(|&a| done.iter().filter(|&&b| b >= a && b < a + 3_600_000).count()).max().unwrap_or(0);
        report.peak_per_runner_hour = peak as f64 / self.cfg.runners.max(1) as f64;
        (report.trip_offers, report.trips_taken, report.taken_first) = (self.pay_stats.0, self.pay_stats.1, self.pay_stats.2);
        report.trip_pay_avg = if self.pay_stats.1 > 0 { self.pay_stats.3 / self.pay_stats.1 as i64 } else { 0 };
        report.batch_avg = if self.batches.0 > 0 { self.batches.1 as f64 / self.batches.0 as f64 } else { 0.0 };
        report
    }
}

pub fn run(world_def: WorldDef, tpl: Template, cfg: &Config) -> Report {
    let mut n = Night::new(world_def, tpl, cfg.clone());
    while n.step() {}
    n.finish()
}

/// Where every dollar came from and went, from the ledger itself. In: fans' cards, and what sponsors and
/// businesses put in. Out: everyone who holds money now. In minus out is always zero (the ledger balances).
pub fn money_flows(l: &Ledger, stands: &[String], runners: &[String]) -> BTreeMap<String, i64> {
    let mut f: BTreeMap<String, i64> = BTreeMap::new();
    for ((a, u), v) in l.balances_iter() {
        if !matches!(u, Unit::Money(_)) || *v == 0 { continue }
        let k = match (a.party.as_str(), a.purpose.as_str()) {
            (_, "card") => "in_fans",
            (_, "bank" | "promo_budget" | "promo_escrow" | "promised_budget" | "deal_budget") => "in_partners",
            ("agape", "revenue") => "agape",
            ("agape", "processing") => "agape_processing",
            ("processor", _) => "interchange",
            ("tax_authority", _) => "tax",
            (p, "pay" | "tips") if runners.iter().any(|r| r == p) => "runners",
            (p, _) if stands.iter().any(|s| s == p) => "stands",
            (_, "revenue") => "venue_partners",
            (_, "earnings" | "payouts") => "people",
            (_, "credit") => "credit",
            _ => "held",
        };
        let sign = if k.starts_with("in_") { -1 } else { 1 };
        *f.entry(k.into()).or_insert(0) += sign * v;
    }
    f
}

fn ledger_sum(l: &Ledger, purpose: &str, prefix: &str) -> i64 {
    l.balances_iter().filter(|((a, u), _)| a.purpose == purpose && a.party.starts_with(prefix) && matches!(u, Unit::Money(_))).map(|(_, v)| *v).sum()
}

/// Seconds until a new order at `stand` would start being prepared plus its own prep: the
/// queue's prep work spread over the stations, after the stations finish what they hold.
pub(crate) fn stand_wait(stations: &BTreeMap<String, Vec<(i64, Option<usize>)>>, queues: &BTreeMap<String, Vec<usize>>, orders: &[Order], stand: &str, t: i64) -> i64 {
    let Some(sts) = stations.get(stand) else { return 0 };
    let busy: i64 = sts.iter().map(|(until, _)| (until - t).max(0) / 1000).sum();
    let queued: i64 = queues.get(stand).map_or(0, |q| q.iter().map(|&i| orders[i].prep_s as i64).sum());
    (busy + queued) / sts.len().max(1) as i64
}

fn sim_fans_n(c: &Config) -> usize { c.fans }

/// What the crowd may say: no links, phone numbers or selling, and swearing is masked.
pub fn moderate(text: &str) -> Result<String, String> {
    let lower = text.to_lowercase();
    let digits = text.chars().filter(|c| c.is_ascii_digit()).count();
    let linky = ["http", "www.", ".com", ".net", ".org", ".biz", ".io", ".ly", "@"].iter().any(|k| lower.contains(k));
    let selling = ["dm me", "cheap tix", "cheap tickets", "venmo", "cashapp", "cash app", "paypal", "crypto", "onlyfans"].iter().any(|k| lower.contains(k));
    if linky || digits >= 7 || selling { return Err("Links, phone numbers and selling aren't allowed in the crowd chat".into()); }
    const MASK: &[&str] = &["fuck", "shit", "bitch", "cunt", "asshole", "dick", "pussy", "bastard", "slut", "whore", "nigg", "fag", "retard"];
    let mut out = String::new();
    for word in text.split_inclusive(|c: char| c.is_whitespace()) {
        let w = word.to_lowercase();
        if MASK.iter().any(|m| w.contains(m)) {
            let core = word.trim_end();
            let mut first = true;
            for c in core.chars() { out.push(if first || !c.is_alphanumeric() { first = false; c } else { '•' }); }
            out.push_str(&word[core.len()..]);
        } else { out.push_str(word); }
    }
    Ok(out)
}

fn last_name(n: &str) -> String { n.split(' ').last().unwrap_or(n).to_string() }

fn price_for(rarity: &str, rng: &mut Rng) -> i64 {
    let (lo, hi) = match rarity { "Common" => (40, 120), "Uncommon" => (90, 260), "Rare" => (250, 700), _ => (600, 2000) };
    (lo + rng.below((hi - lo) as u64) as i64) / 5 * 5
}

pub fn pass_unit(place: &str) -> Unit { Unit::Thing(format!("pass:{place}")) }

impl Night {
    /// Fans whose own screen an action by `who` changes: themselves, the fan of the order it names,
    /// and the fans of a runner's trip. Everyone else's screen can catch up on the next refresh.
    pub fn fans_touched(&self, who: &str, v: &serde_json::Value) -> Vec<String> {
        let mut out: Vec<String> = vec![];
        if self.fans.iter().any(|f| f.id == who) { out.push(who.into()); }
        if let Some(o) = v["order"].as_u64().and_then(|i| self.orders.get(i as usize)) { out.push(o.c.quote.fan.clone()); }
        if let Some(tr) = self.trips.get(who) { for &i in &tr.orders { if let Some(o) = self.orders.get(i) { out.push(o.c.quote.fan.clone()); } } }
        // a gift lands on another fan's screen
        if let Some(to) = v["to"].as_str() { if self.fans.iter().any(|f| f.id == to) { out.push(to.into()); } }
        out.sort(); out.dedup();
        out
    }
}

// ---------------- a real night's game, as the venue's team marks it ----------------

impl Night {
    /// The next moment of a real game: "start" (the next period), "end" (the period under way), or
    /// "final". Taken in now, so phases, windows and the rules move with the real game.
    pub fn mark_game(&mut self, what: &str) -> Result<String, String> {
        if !self.cfg.real { return Err("The simulated game runs itself".into()); }
        let m = self.t / 60_000;
        let marked = |s: &[(i64, &'static str, i64)], k: &str, n: i64| s.iter().any(|x| x.1 == k && x.2 == n && x.0 != NOT_YET);
        if marked(&self.script, "game.final", 0) { return Err("The game is already final".into()); }
        let started = (1..=4).filter(|&n| marked(&self.script, "period.start", n)).max().unwrap_or(0);
        let open = started > 0 && !marked(&self.script, "period.end", started);
        let mut take: Vec<(&'static str, i64)> = vec![];
        match what {
            "start" => {
                if open { return Err(format!("Period {started} is still under way: end it first")); }
                if started >= 4 { return Err("All four periods have been played: mark the final".into()); }
                take.push(("period.start", started + 1));
            }
            "end" => {
                if !open { return Err("No period is under way".into()); }
                take.push(("period.end", started));
            }
            "final" => {
                if open { take.push(("period.end", started)); }
                take.push(("game.final", 0));
            }
            _ => return Err("Mark start, end or final".into()),
        }
        let mut outs = vec![];
        for (kind, n) in take.clone() {
            if let Some(x) = self.script.iter_mut().find(|x| x.1 == kind && x.2 == n) { x.0 = m; }
            let f = if kind == "game.final" { fields!{} } else { fields!{"n" => n} };
            outs.extend(self.engine.ingest(&mut self.log, kind, f, "keeper", self.t).map_err(|e| format!("{e:?}"))?);
            let text = match (kind, self.act_on(n)) {
                ("game.final", Some(_)) => "That's the show".to_string(), ("game.final", None) => "Final".to_string(),
                ("period.start", Some(a)) => format!("{a} takes the stage"), ("period.end", Some(a)) => format!("{a} leaves the stage"),
                ("period.start", None) => format!("Q{n} underway"), _ => format!("End of Q{n}") };
            self.note("game", "", text);
        }
        if what == "final" {
            // moments the game never reached aren't coming; the night closes 20 minutes after the final
            self.script.retain(|x| x.0 != NOT_YET);
            self.end_min = m + 20;
        }
        self.absorb(outs);
        Ok(take.last().map(|x| format!("{}:{}", x.0, x.1)).unwrap_or_default())
    }

    /// A score as the venue's scorekeeper enters it. A negative number corrects a mistake.
    pub fn keep_score(&mut self, team: &str, points: i64, player: &str) -> Result<String, String> {
        if !self.cfg.real { return Err("The simulated game keeps its own score".into()); }
        if team != "home" && team != "away" { return Err("Home or away".into()); }
        if points == 0 || points.abs() > 9 { return Err("Between 1 and 9 points (negative to correct)".into()); }
        let cur = if team == "home" { self.score.0 } else { self.score.1 };
        if cur + points < 0 { return Err("A score can't go below zero".into()); }
        if team == "home" { self.score.0 += points } else { self.score.1 += points }
        if points < 0 { self.note("score", team, format!("Score corrected ({points})")); return Ok(String::new()); }
        let gid = self.group_of(team);
        let star = self.league.team(&gid).and_then(|g| g.roster.iter().find(|m| m.id == player).map(|m| m.star)).unwrap_or(false);
        let who = if star || self.league.team(&gid).map_or(false, |g| g.roster.iter().any(|m| m.id == player)) { player.to_string() } else { String::new() };
        if !who.is_empty() { self.last_scorer.insert(team.to_string(), who.clone()); }
        let outs = self.engine.ingest(&mut self.log, "score", fields!{"team" => team, "points" => points, "player" => who.as_str(), "star" => star}, "keeper", self.t).map_err(|e| format!("{e:?}"))?;
        self.note_about("score", team, format!("+{points}"), &who, None);
        self.absorb(outs);
        Ok(String::new())
    }

    /// Where the real game stands, for the venue's console: what can be marked next.
    pub fn game_state(&self) -> serde_json::Value {
        let marked = |k: &str, n: i64| self.script.iter().any(|x| x.1 == k && x.2 == n && x.0 != NOT_YET);
        let started = (1..=4).filter(|&n| marked("period.start", n)).max().unwrap_or(0);
        let open = started > 0 && !marked("period.end", started);
        let fin = marked("game.final", 0);
        serde_json::json!({ "real": self.cfg.real, "period": started, "in_period": open, "final": fin, "score": [self.score.0, self.score.1],
            "can": { "start": !fin && !open && started < 4, "end": open, "final": !fin } })
    }
}

// ---------------- stands and runners who are people ----------------

impl Night {
    /// A person is acting for this party (signed in on a device, or the harness acting as them).
    pub fn human(&self, party: &str) -> bool { self.live.contains(party) || self.driven.contains(party) }

    /// Staff start and end their shift. While on shift, the simulation does nothing for them.
    pub fn clock_in(&mut self, who: &str) -> Result<(), String> {
        let known = who == "ops" || self.runners.iter().any(|r| r.id == who) || self.world.def.stands.iter().any(|s| s.id == who);
        if !known { return Err("Only stands and runners clock in".into()); }
        self.live.insert(who.into());
        Ok(())
    }
    pub fn clock_out(&mut self, who: &str) -> Result<(), String> {
        if self.trips.get(who).map_or(false, |x| x.accepted) { return Err("Finish your trip first".into()); }
        if self.policy.fixed_shift && self.runners.iter().any(|r| r.id == who) && self.t < self.end_min * 60_000 {
            return Err("This is a fixed shift: you stay online until the event ends".into());
        }
        if self.trips.contains_key(who) { self.return_trip(who, self.t); }
        self.live.remove(who);
        Ok(())
    }

    fn finish_station(&mut self, stand: &str, si: usize) {
        let t = self.t;
        let Some(id) = self.stations[stand][si].1 else { return };
        self.orders[id].c.act(Action::MarkReady, stand, &mut self.ledger, &self.terms).unwrap();
        self.orders[id].ready_at = Some(t);
        if self.orders[id].c.quote.delivery {
            let o = &self.orders[id];
            let shelf_at = if o.priority { t - PRIORITY_AHEAD_MS } else { t };
            self.ready.push(Ready { order: id.to_string(), stand: stand.into(), seat: o.seat.clone(), step_free: o.step_free, ready_at: shelf_at });
        } else if !self.human(stand) {
            let at = t + 60_000 + self.rng.below(240_000) as i64;
            self.orders[id].eta = at;
            self.sched(at, Ev::FanCollects(id));
        } else {
            self.orders[id].eta = t; // waiting at the counter for the fan
        }
        self.stations.get_mut(stand).unwrap()[si] = (t, None);
    }

    /// The counter hands a pickup order to the fan who ordered it.
    pub fn handed_over(&mut self, stand: &str, order: usize) -> Result<(), String> {
        let o = self.orders.get(order).ok_or("Unknown order")?;
        if o.c.quote.stand != stand || o.c.quote.delivery { return Err("That isn't a pickup order here".into()); }
        let t = self.t;
        self.orders[order].c.act(Action::Deliver, stand, &mut self.ledger, &self.terms).map_err(|e| format!("{e:?}"))?;
        self.orders[order].delivered_at = Some(t);
        self.settle_now(order);
        Ok(())
    }

    /// Capture a delivered order: the money moves, credit is used first, and now and then the crowd complains.
    pub(crate) fn settle_now(&mut self, id: usize) {
                    if self.orders[id].c.stage != Stage::Delivered { return; }
                    let fan = self.orders[id].c.quote.fan.clone();
                    // fans' complaints go through problems and remedies (care.rs, remedy.rs) now
                    let dispute = false;
                    let upheld = self.rng.chance(0.3);
                    let o = &mut self.orders[id];
                    let res: Result<(), CommitError> = if dispute {
                        o.c.act(Action::Dispute, &fan, &mut self.ledger, &self.terms).map(|_| ())
                            .and_then(|_| o.c.act(Action::Resolve { upheld }, "agape", &mut self.ledger, &self.terms).map(|_| ()))
                    } else { o.c.act(Action::Settle, "agape", &mut self.ledger, &self.terms).map(|_| ()) };
                    res.unwrap();
                    if o.c.stage == Stage::Settled {
                        let b = breakdown(&o.c.quote, &self.terms).unwrap();
                        let (src, q) = (o.source.clone(), o.c.quote.clone());
                        self.attr.settled(&src, &q, &b);
                        self.use_credit(id, b.charge);
                        self.crowd_claim(id);
                    }
    }

    /// The kitchen marks an order ready.
    pub fn mark_ready(&mut self, stand: &str, order: usize) -> Result<(), String> {
        let on = self.stations.get(stand).and_then(|v| v.iter().position(|x| x.1 == Some(order)));
        if let Some(si) = on { self.finish_station(stand, si); return Ok(()); }
        // still waiting in line: the people at the counter made it anyway (quicker than the line said)
        let queued = self.queues.get(stand).map_or(false, |q| q.contains(&order));
        if !queued { return Err("That order isn't on one of your stations".into()); }
        self.queues.get_mut(stand).unwrap().retain(|&i| i != order);
        self.orders[order].c.act(Action::StartPrep, stand, &mut self.ledger, &self.terms).map_err(|e| format!("{e:?}"))?;
        self.orders[order].started_at = Some(self.t);
        let sts = self.stations.get_mut(stand).ok_or("Unknown stand")?;
        let si = match sts.iter().position(|x| x.1.is_none()) { Some(i) => i, None => { sts.push((self.t, None)); sts.len() - 1 } };
        let extra = si >= self.world.def.stands.iter().find(|x| x.id == stand).map_or(0, |x| x.stations as usize);
        sts[si] = (self.t, Some(order));
        self.finish_station(stand, si);
        if extra { self.stations.get_mut(stand).unwrap().pop(); }
        Ok(())
    }

    /// The kitchen can't make it: the fan isn't charged and is told at once; the stand owns the miss.
    pub fn cannot_make(&mut self, stand: &str, order: usize) -> Result<(), String> {
        let o = self.orders.get(order).ok_or("Unknown order")?;
        if o.c.quote.stand != stand { return Err("That isn't your order".into()); }
        self.orders[order].c.act(Action::CannotFulfil, stand, &mut self.ledger, &self.terms).map_err(|e| format!("{e:?}"))?;
        for q in self.queues.values_mut() { q.retain(|&i| i != order); }
        if let Some(v) = self.stations.get_mut(stand) { for x in v.iter_mut() { if x.1 == Some(order) { *x = (self.t, None); } } }
        Ok(())
    }

    /// What a trip pays the runner (tips on top): its first offer and, in offers mode, raised for each runner
    /// who passed on any of its orders; or the fixed amount per delivery. Returns (pay, first offer, passes).
    pub(crate) fn trip_pay(&self, ids: &[usize], secs: u32) -> (i64, i64, u32) {
        let passes = ids.iter().map(|&i| self.orders[i].passes).max().unwrap_or(0);
        if self.terms.hourly() { return (0, 0, passes); } // paid by the hour instead (see `pay_wages`)
        if !self.terms.offers() { return (self.terms.runner_pay * ids.len() as i64, self.terms.runner_pay * ids.len() as i64, passes); }
        let items: i64 = ids.iter().flat_map(|&i| self.orders[i].c.quote.lines.iter().map(|l| l.qty)).sum();
        let first = self.terms.first_offer(secs as i64, items);
        (self.terms.offer_after(first, passes as i64), first, passes)
    }

    /// Hourly mode, once a minute: every runner online earns the minute's wage from Agape. Simulated
    /// runners are online for the whole night; a person only while they're clocked in. Paid to the
    /// cent over the hour, not rounded per minute.
    fn pay_wages(&mut self) {
        let on: Vec<String> = self.runners.iter().map(|r| r.id.clone())
            .filter(|id| if self.driven.contains(id) { self.live.contains(id) } else { true }).collect();
        let mut post = vec![];
        let mut total = 0;
        for id in on {
            let m = self.wage_minutes.entry(id.clone()).or_insert(0);
            let due = self.terms.hourly_wage * (*m + 1) / 60 - self.terms.hourly_wage * *m / 60;
            *m += 1;
            if due > 0 { post.push(crate::value::posting(acct(&id, "pay"), Unit::usd(), due)); total += due; }
        }
        if total == 0 { return; }
        post.push(crate::value::posting(acct("agape", "revenue"), Unit::usd(), -total));
        let _ = self.ledger.post(crate::value::Txn { memo: "wages".into(), postings: post });
    }

    /// A runner took a trip: each of its deliveries carries its share of the pay into settlement.
    fn took_trip(&mut self, ids: &[usize], pay: i64, passes: u32) {
        let n = ids.len() as i64;
        for (k, &i) in ids.iter().enumerate() { self.orders[i].c.quote.runner_pay = Some(pay / n + if k == 0 { pay % n } else { 0 }); }
        self.pay_stats.1 += 1; self.pay_stats.2 += (passes == 0) as u64; self.pay_stats.3 += pay;
    }

    /// An offered trip goes back to the shelf; the runner isn't offered another until `rest_until`.
    fn return_trip(&mut self, runner: &str, rest_until: i64) {
        let Some(trip) = self.trips.remove(runner) else { return };
        for &id in &trip.orders { self.orders[id].passes += 1; }
        for id in trip.orders {
            let o = &self.orders[id];
            let at = o.ready_at.unwrap_or(self.t) - if o.priority { PRIORITY_AHEAD_MS } else { 0 };
            self.ready.push(Ready { order: id.to_string(), stand: o.c.quote.stand.clone(), seat: o.seat.clone(), step_free: o.step_free, ready_at: at });
        }
        if let Some(r) = self.runners.iter_mut().find(|r| r.id == runner) { r.free_at = rest_until; }
    }

    pub fn accept_trip(&mut self, runner: &str) -> Result<(), String> {
        let t = self.t;
        let trip = self.trips.get(runner).filter(|x| !x.accepted).cloned().ok_or("No trip is waiting for you")?;
        let r = self.runners.iter_mut().find(|r| r.id == runner).ok_or("Unknown runner")?;
        let mut plan = vec![(t, r.at.clone())];
        plan.extend(trip.pickups.iter().map(|(s, at)| (t + *at as i64 * 1000, s.clone())));
        self.took_trip(&trip.orders, trip.pay, trip.passes);
        for (k, &id) in trip.orders.iter().enumerate() {
            self.orders[id].c.act(Action::Assign(runner.into()), "agape", &mut self.ledger, &self.terms).map_err(|e| format!("{e:?}"))?;
            self.orders[id].eta = t + trip.arrive_s[k] as i64 * 1000;
            plan.push((self.orders[id].eta, self.orders[id].seat.clone()));
        }
        self.plans.insert(runner.into(), plan);
        self.batches.0 += 1; self.batches.1 += trip.orders.len() as u64;
        self.trips.get_mut(runner).unwrap().accepted = true;
        Ok(())
    }

    pub fn decline_trip(&mut self, runner: &str) -> Result<(), String> {
        if !self.trips.get(runner).map_or(false, |x| !x.accepted) { return Err("No trip is waiting for you".into()); }
        self.return_trip(runner, self.t + 30_000);
        Ok(())
    }

    pub fn picked_up(&mut self, runner: &str) -> Result<(), String> {
        let trip = self.trips.get(runner).filter(|x| x.accepted && !x.picked).cloned().ok_or("Nothing to pick up")?;
        for &id in &trip.orders { self.orders[id].c.act(Action::PickUp, runner, &mut self.ledger, &self.terms).map_err(|e| format!("{e:?}"))?; self.orders[id].picked_at = Some(self.t); }
        self.trips.get_mut(runner).unwrap().picked = true;
        Ok(())
    }

    fn trip_order(&self, runner: &str, order: usize) -> Result<(), String> {
        let trip = self.trips.get(runner).ok_or("You're not on a trip")?;
        if !trip.picked { return Err("Pick it up first".into()); }
        if !trip.orders.contains(&order) { return Err("That order isn't in your bag".into()); }
        if self.orders[order].c.stage != Stage::PickedUp { return Err("That one's done".into()); }
        Ok(())
    }

    fn after_handoff(&mut self, runner: &str) {
        let done = self.trips.get(runner).map_or(false, |x| x.orders.iter().all(|&i| self.orders[i].c.stage != Stage::PickedUp));
        if done {
            let trip = self.trips.remove(runner).unwrap();
            let last = trip.orders.last().map(|&i| self.orders[i].seat.clone());
            if let Some(r) = self.runners.iter_mut().find(|r| r.id == runner) { r.free_at = self.t; if let Some(l) = last { r.at = l; } }
            self.plans.remove(runner);
        }
    }

    pub fn delivered(&mut self, runner: &str, order: usize) -> Result<(), String> { self.delivered_with(runner, order, false, "") }
    /// Handed over, with whatever proof this fan's record calls for: a photo of the handoff, or the
    /// PIN on the fan's phone. The runner is told what to collect, never why.
    pub fn delivered_with(&mut self, runner: &str, order: usize, photo: bool, pin: &str) -> Result<(), String> {
        self.trip_order(runner, order)?;
        match self.proof_of(order) {
            crate::remedy::Proof::Photo if !photo => return Err("Take a photo of the handoff first".into()),
            crate::remedy::Proof::Pin if pin.trim() != self.pin_of(order) => return Err(if pin.trim().is_empty() { "Ask the fan for the PIN on their phone".into() } else { "That PIN doesn't match".into() }),
            _ => {}
        }
        let t = self.t;
        // where the runner's phone was: short of the seat by however much of the walk was left
        let left = (self.orders[order].eta - t).max(0) / 1000;
        self.orders[order].handoff_m = Some(if left > 20 { left * 13 / 10 } else { 3 + order as i64 % 9 });
        let o = &mut self.orders[order];
        if t > o.promise { if o.c.late_by.is_none() { o.c.late_by = Some(("agape".into(), (t - o.promise) / 1000)); } self.report.late += 1; }
        o.c.act(Action::Deliver, runner, &mut self.ledger, &self.terms).map_err(|e| format!("{e:?}"))?;
        o.delivered_at = Some(t);
        // a person's handoff is paid the moment they tap it
        self.settle_now(order);
        self.after_handoff(runner);
        Ok(())
    }

    /// The fan isn't at their seat. Only after a fair wait at the seat; the runner is still paid.
    pub fn unreachable(&mut self, runner: &str, order: usize) -> Result<(), String> {
        self.trip_order(runner, order)?;
        let since = self.t - self.orders[order].picked_at.unwrap_or(self.t);
        if since < 3 * 60_000 { return Err("Give them a few minutes first".into()); }
        let o = &mut self.orders[order];
        o.c.act(Action::Unreachable, runner, &mut self.ledger, &self.terms).map_err(|e| format!("{e:?}"))?;
        let mut q = o.c.quote.clone(); q.tip = 0;
        let b = breakdown(&q, &self.terms).unwrap();
        self.attr.settled(&o.source.clone(), &q, &b);
        self.after_handoff(runner);
        Ok(())
    }
}

/// Cents as dollars, for messages.
pub fn dollars(c: i64) -> String { format!("${}.{:02}", c / 100, c % 100) }

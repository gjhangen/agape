//! The rule language (A2): measures as folds, spans opened and closed by signals,
//! patterns over facts, effects as new signals. Total, incremental, deterministic,
//! typed at load time, and stratified so no rule can trigger itself.

use crate::signal::{Authority, Fields, Log, Schema, Signal, SignalError, Ty, Val};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

// ---------- the AST (this is the JSON a template carries) ----------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Op { Eq, Ne, Lt, Lte, Gt, Gte }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sel {
    pub sig: String,
    #[serde(default, rename = "where")]
    pub preds: BTreeMap<String, BTreeMap<Op, Val>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Term {
    pub of: Sel,
    /// Field to sum; absent means count matches.
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default = "one")]
    pub coef: i64,
}
fn one() -> i64 { 1 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeasureDef {
    pub name: String,
    pub terms: Vec<Term>,
    /// Sliding window in ms (8 fixed buckets, so state stays bounded). Absent: whole event.
    #[serde(default)]
    pub window_ms: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpanDef {
    pub name: String,
    pub opens: Sel,
    pub closes: Sel,
    #[serde(default)]
    pub repeat: bool,
    #[serde(default)]
    pub parent: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    Match(Sel),
    Count { of: Sel, gte: i64 },
    Streak { of: Sel, key: String, #[serde(default)] field: Option<String>, gte: i64 },
    Every { n: i64, of: Sel },
    Changes { measure: String },
    Crosses { measure: String, above: i64 },
    Seq { first: Sel, then: Sel, within_ms: i64 },
    Idle { of: Sel, ms: i64 },
    Enter { span: String, #[serde(default)] n: Option<i64> },
    Exit { span: String, #[serde(default)] n: Option<i64> },
    Any(Vec<Pattern>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Cond {
    Measure { measure: String, op: Op, value: i64 },
    SpanOpen { span: String },
    Not(Box<Cond>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// Fields may use "$key" (streak key), "$scope", "$n" (span index), "$f.<field>" (trigger field).
    Emit { kind: String, #[serde(default)] fields: BTreeMap<String, Val> },
    /// An offer request; layer 2 runs eligibility and the Agape check before anyone sees it.
    Offer { thing: String, price: i64, #[serde(default)] discount: i64, #[serde(default)] payer: Option<String>, #[serde(default)] expires_ms: Option<i64>,
            #[serde(default)] limits: Limits },
    /// Ask the crowd a question. Each option is tied to a measure; when settled, the option whose
    /// measure grew most since the question opened wins (`otherwise` wins if none grew). "$n" in the
    /// question is the span index. Correct picks earn `points`, issued by Agape.
    Ask { id: String, question: String, options: Vec<AskOption>, #[serde(default)] otherwise: Option<String>, locks_ms: i64, points: i64 },
    /// Settle the most recent open question with this id.
    Settle { ask: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AskOption { pub id: String, #[serde(default)] pub measure: Option<String> }

/// How an offer may end. Guaranteed (the default): shown only to as many fans as can all be
/// served, and held for each of them until it expires. While it lasts: shown to every eligible
/// fan, and ends when time runs out, a resource runs out (stock, the payer's budget, the cap,
/// the current tier) or the `while` condition stops holding. Either way, every limit is shown
/// to the fan before they act, and a tap is a claim: once tapped, checkout cannot fail.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Limits {
    #[serde(default)]
    pub mode: Mode,
    /// At most this many claims in total ("first 150").
    #[serde(default)]
    pub cap: Option<i64>,
    /// At most this many claims per fan.
    #[serde(default)]
    pub per_fan: Option<i64>,
    /// The offer lasts only while this holds ("during the break", "while the home side leads").
    #[serde(default, rename = "while")]
    pub while_: Option<Cond>,
    /// Tiered terms: the first `units` claims get `discount`, the next tier the next, and so on.
    #[serde(default)]
    pub tiers: Vec<Tier>,
    /// What the fan reads for the condition, e.g. "Only during the break".
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode { #[default] Guaranteed, WhileItLasts }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tier { pub units: i64, pub discount: i64 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleDef {
    pub name: String,
    #[serde(default = "one_u")]
    pub stratum: u32,
    pub when: Pattern,
    /// Partition state by this field of the triggering signal ("section", "stand"). Absent: whole event.
    #[serde(default)]
    pub per: Option<String>,
    #[serde(default, rename = "if")]
    pub guard: Option<Cond>,
    #[serde(default)]
    pub unless: Option<Cond>,
    pub then: Vec<Effect>,
    #[serde(default)]
    pub throttle_ms: Option<i64>,
}
fn one_u() -> u32 { 1 }

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Template {
    #[serde(default)]
    pub measures: Vec<MeasureDef>,
    #[serde(default)]
    pub spans: Vec<SpanDef>,
    #[serde(default)]
    pub rules: Vec<RuleDef>,
}

// ---------- load-time checks ----------

#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    UnknownKind(String),
    UnknownField { kind: String, field: String },
    WrongType { kind: String, field: String },
    UnknownMeasure(String),
    UnknownSpan(String),
    StratumViolation { rule: String, reads: String, emitted_by: String },
    DiscountWithoutPayer(String),
    SelfOpeningSpan(String),
    BadNumber(String),
}

pub fn add_span_kinds(s: Schema) -> Schema {
    s.kind("span.enter", &[("span", Ty::Str), ("n", Ty::Int)], Some("system"))
     .kind("span.exit", &[("span", Ty::Str), ("n", Ty::Int)], Some("system"))
}

fn check_sel(schema: &Schema, s: &Sel) -> Result<(), LoadError> {
    let spec = schema.kinds.get(&s.sig).ok_or_else(|| LoadError::UnknownKind(s.sig.clone()))?;
    for (f, cmps) in &s.preds {
        let ty = spec.fields.get(f).ok_or_else(|| LoadError::UnknownField { kind: s.sig.clone(), field: f.clone() })?;
        for (op, v) in cmps {
            let ordered = matches!(op, Op::Lt | Op::Lte | Op::Gt | Op::Gte);
            if !ty.admits(v) || (ordered && *ty != Ty::Int) {
                return Err(LoadError::WrongType { kind: s.sig.clone(), field: f.clone() });
            }
        }
    }
    Ok(())
}

fn check_int_field(schema: &Schema, s: &Sel, f: &str) -> Result<(), LoadError> {
    match schema.field_ty(&s.sig, f) {
        Some(Ty::Int) => Ok(()),
        Some(_) => Err(LoadError::WrongType { kind: s.sig.clone(), field: f.into() }),
        None => Err(LoadError::UnknownField { kind: s.sig.clone(), field: f.into() }),
    }
}

fn pattern_kinds(p: &Pattern, ms: &[MeasureDef], out: &mut Vec<String>) {
    match p {
        Pattern::Match(s) | Pattern::Count { of: s, .. } | Pattern::Streak { of: s, .. }
        | Pattern::Every { of: s, .. } | Pattern::Idle { of: s, .. } => out.push(s.sig.clone()),
        Pattern::Seq { first, then, .. } => { out.push(first.sig.clone()); out.push(then.sig.clone()); }
        Pattern::Enter { .. } => out.push("span.enter".into()),
        Pattern::Exit { .. } => out.push("span.exit".into()),
        // a measure-reading pattern reads whatever its measure folds over
        Pattern::Changes { measure } | Pattern::Crosses { measure, .. } => {
            for m in ms.iter().filter(|m| &m.name == measure) { out.extend(m.terms.iter().map(|t| t.of.sig.clone())); }
        }
        Pattern::Any(ps) => ps.iter().for_each(|p| pattern_kinds(p, ms, out)),
    }
}

impl Template {
    pub fn validate(&self, schema: &Schema) -> Result<(), LoadError> {
        let measures: Vec<&str> = self.measures.iter().map(|m| m.name.as_str()).collect();
        let spans: Vec<&str> = self.spans.iter().map(|s| s.name.as_str()).collect();
        for m in &self.measures {
            for t in &m.terms {
                check_sel(schema, &t.of)?;
                if let Some(f) = &t.field { check_int_field(schema, &t.of, f)?; }
            }
            if m.window_ms.map_or(false, |w| w < 8) { return Err(LoadError::BadNumber(m.name.clone())); }
        }
        for s in &self.spans {
            check_sel(schema, &s.opens)?;
            check_sel(schema, &s.closes)?;
            if let Some(p) = &s.parent { if !spans.contains(&p.as_str()) { return Err(LoadError::UnknownSpan(p.clone())); } }
            let own = |sel: &Sel| sel.sig.starts_with("span.")
                && sel.preds.get("span").and_then(|c| c.get(&Op::Eq)) == Some(&Val::S(s.name.clone()));
            if own(&s.opens) { return Err(LoadError::SelfOpeningSpan(s.name.clone())); }
        }
        fn check_pat(p: &Pattern, schema: &Schema, measures: &[&str], spans: &[&str]) -> Result<(), LoadError> {
            match p {
                Pattern::Match(s) => check_sel(schema, s),
                Pattern::Count { of, gte } => { if *gte < 1 { return Err(LoadError::BadNumber("count".into())); } check_sel(schema, of) }
                Pattern::Every { n, of } => { if *n < 1 { return Err(LoadError::BadNumber("every".into())); } check_sel(schema, of) }
                Pattern::Idle { of, ms } => { if *ms < 1 { return Err(LoadError::BadNumber("idle".into())); } check_sel(schema, of) }
                Pattern::Streak { of, key, field, .. } => {
                    check_sel(schema, of)?;
                    if schema.field_ty(&of.sig, key).is_none() { return Err(LoadError::UnknownField { kind: of.sig.clone(), field: key.clone() }); }
                    if let Some(f) = field { check_int_field(schema, of, f)?; }
                    Ok(())
                }
                Pattern::Seq { first, then, .. } => { check_sel(schema, first)?; check_sel(schema, then) }
                Pattern::Changes { measure } | Pattern::Crosses { measure, .. } =>
                    if measures.contains(&measure.as_str()) { Ok(()) } else { Err(LoadError::UnknownMeasure(measure.clone())) },
                Pattern::Enter { span, .. } | Pattern::Exit { span, .. } =>
                    if spans.contains(&span.as_str()) { Ok(()) } else { Err(LoadError::UnknownSpan(span.clone())) },
                Pattern::Any(ps) => ps.iter().try_for_each(|p| check_pat(p, schema, measures, spans)),
            }
        }
        fn check_cond(c: &Cond, measures: &[&str], spans: &[&str]) -> Result<(), LoadError> {
            match c {
                Cond::Measure { measure, .. } => if measures.contains(&measure.as_str()) { Ok(()) } else { Err(LoadError::UnknownMeasure(measure.clone())) },
                Cond::SpanOpen { span } => if spans.contains(&span.as_str()) { Ok(()) } else { Err(LoadError::UnknownSpan(span.clone())) },
                Cond::Not(c) => check_cond(c, measures, spans),
            }
        }
        // who emits what, at which stratum
        let mut emitters: Vec<(String, u32, String)> = vec![];
        for r in &self.rules {
            check_pat(&r.when, schema, &measures, &spans)?;
            if let Some(c) = &r.guard { check_cond(c, &measures, &spans)?; }
            if let Some(c) = &r.unless { check_cond(c, &measures, &spans)?; }
            if r.stratum == 0 { return Err(LoadError::BadNumber(format!("{}: stratum starts at 1", r.name))); }
            for e in &r.then {
                match e {
                    Effect::Emit { kind, fields } => {
                        let spec = schema.kinds.get(kind).ok_or_else(|| LoadError::UnknownKind(kind.clone()))?;
                        for (f, v) in fields {
                            let ty = spec.fields.get(f).ok_or_else(|| LoadError::UnknownField { kind: kind.clone(), field: f.clone() })?;
                            let substituted = matches!(v, Val::S(s) if s.starts_with('$'));
                            if !substituted && !ty.admits(v) { return Err(LoadError::WrongType { kind: kind.clone(), field: f.clone() }); }
                        }
                        emitters.push((kind.clone(), r.stratum, r.name.clone()));
                    }
                    Effect::Ask { options, otherwise, points, locks_ms, .. } => {
                        if options.len() < 2 || *points < 0 || *locks_ms < 0 { return Err(LoadError::BadNumber(r.name.clone())); }
                        for o in options { if let Some(m) = &o.measure { if !measures.contains(&m.as_str()) { return Err(LoadError::UnknownMeasure(m.clone())); } } }
                        if let Some(x) = otherwise { if !options.iter().any(|o| &o.id == x) { return Err(LoadError::BadNumber(format!("{}: otherwise must be an option", r.name))); } }
                    }
                    Effect::Settle { ask } => {
                        let asked = self.rules.iter().flat_map(|r| r.then.iter()).any(|e| matches!(e, Effect::Ask { id, .. } if id == ask));
                        if !asked { return Err(LoadError::BadNumber(format!("{}: settles a question nobody asks", r.name))); }
                    }
                    Effect::Offer { discount, payer, price, limits, .. } => {
                        if *discount < 0 || *price < 0 || discount > price { return Err(LoadError::BadNumber(r.name.clone())); }
                        if *discount > 0 && payer.is_none() { return Err(LoadError::DiscountWithoutPayer(r.name.clone())); }
                        if limits.tiers.iter().any(|t| t.units < 1 || t.discount < 0 || t.discount > *price) || limits.cap.map_or(false, |c| c < 1) || limits.per_fan.map_or(false, |c| c < 1) {
                            return Err(LoadError::BadNumber(r.name.clone()));
                        }
                        if limits.tiers.iter().any(|t| t.discount > 0) && payer.is_none() { return Err(LoadError::DiscountWithoutPayer(r.name.clone())); }
                        if let Some(c) = &limits.while_ { check_cond(c, &measures, &spans)?; }
                        // tiers move the price claim by claim, which only a while-it-lasts offer can do honestly
                        if !limits.tiers.is_empty() && limits.mode == Mode::Guaranteed { return Err(LoadError::BadNumber(format!("{}: tiers need while_it_lasts", r.name))); }
                    }
                }
            }
        }
        // stratification: a rule may only read kinds emitted by strictly lower strata
        for r in &self.rules {
            let mut reads = vec![];
            pattern_kinds(&r.when, &self.measures, &mut reads);
            for (kind, s, by) in &emitters {
                let reads_it = reads.iter().any(|k| k == kind);
                if reads_it && *s >= r.stratum {
                    return Err(LoadError::StratumViolation { rule: r.name.clone(), reads: kind.clone(), emitted_by: by.clone() });
                }
            }
        }
        Ok(())
    }
}

// ---------- evaluation ----------

fn cmp(a: &Val, op: &Op, b: &Val) -> bool {
    match op {
        Op::Eq => a == b,
        Op::Ne => a != b,
        _ => match (a.as_i(), b.as_i()) {
            (Some(x), Some(y)) => match op { Op::Lt => x < y, Op::Lte => x <= y, Op::Gt => x > y, Op::Gte => x >= y, _ => false },
            _ => false,
        },
    }
}

impl Sel {
    pub fn matches(&self, s: &Signal) -> bool {
        s.kind == self.sig && self.preds.iter().all(|(f, cmps)| {
            s.fields.get(f).map_or(false, |v| cmps.iter().all(|(op, x)| cmp(v, op, x)))
        })
    }
}

fn field_i(s: &Signal, f: &Option<String>) -> i64 {
    match f { None => 1, Some(f) => s.fields.get(f).and_then(Val::as_i).unwrap_or(0) }
}

struct MeasureState { total: i64, buckets: [i64; 8], bucket_t: [i64; 8] }

impl MeasureState {
    fn add(&mut self, def: &MeasureDef, t: i64, d: i64) {
        match def.window_ms {
            None => self.total += d,
            Some(w) => {
                let width = w / 8;
                let idx = t.div_euclid(width);
                let slot = idx.rem_euclid(8) as usize;
                if self.bucket_t[slot] != idx { self.bucket_t[slot] = idx; self.buckets[slot] = 0; }
                self.buckets[slot] += d;
            }
        }
    }
    fn value(&self, def: &MeasureDef, now: i64) -> i64 {
        match def.window_ms {
            None => self.total,
            Some(w) => {
                let idx = now.div_euclid(w / 8);
                (0..8).filter(|i| self.bucket_t[*i] > idx - 8 && self.bucket_t[*i] <= idx).map(|i| self.buckets[i]).sum()
            }
        }
    }
}

#[derive(Default, Clone)]
struct SpanState { open: bool, n: i64 }

#[derive(Default, Clone, Debug)]
struct PState {
    count: i64,
    fired: bool,
    streak_key: Option<Val>,
    streak_sum: i64,
    last_sign: i64,
    above: bool,
    first_at: Option<i64>,
    last_seen: Option<i64>,
    idle_fired: bool,
    any: Vec<PState>,
}

/// What evaluation produced, in order. Layer 2 consumes offers; emitted signals are already in the log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Output {
    Emitted { seq: u64, rule: String, kind: String, fields: Fields },
    Offer { rule: String, scope: String, thing: String, price: i64, discount: i64, payer: Option<String>, expires_ms: Option<i64>, t_ms: i64, limits: Limits },
    Ask { ask: String, question: String, options: Vec<AskOption>, otherwise: Option<String>, locks_ms: i64, points: i64, t_ms: i64 },
    Settle { ask: String, t_ms: i64 },
}

pub struct Engine {
    pub tpl: Template,
    measures: Vec<MeasureState>,
    spans: Vec<SpanState>,
    state: Vec<HashMap<String, PState>>,
    last_fire: Vec<HashMap<String, i64>>,
}

struct Ctx<'a> { key: Option<Val>, n: Option<i64>, sig: &'a Signal, scope: String }

impl Engine {
    pub fn load(tpl: Template, schema: &Schema) -> Result<Engine, LoadError> {
        tpl.validate(schema)?;
        Ok(Engine {
            measures: tpl.measures.iter().map(|_| MeasureState { total: 0, buckets: [0; 8], bucket_t: [i64::MIN; 8] }).collect(),
            spans: vec![SpanState::default(); tpl.spans.len()],
            state: vec![HashMap::new(); tpl.rules.len()],
            last_fire: vec![HashMap::new(); tpl.rules.len()],
            tpl,
        })
    }

    /// Add a rule to a running night (a league-wide deal installed in every venue). It is checked
    /// against the schema like any template rule; its state starts empty.
    pub fn add_rule(&mut self, rule: RuleDef, schema: &Schema) -> Result<(), LoadError> {
        let mut t = self.tpl.clone();
        if t.rules.iter().any(|r| r.name == rule.name) { return Ok(()); }
        t.rules.push(rule);
        t.validate(schema)?;
        self.tpl = t;
        self.state.push(HashMap::new());
        self.last_fire.push(HashMap::new());
        Ok(())
    }

    pub fn measure(&self, name: &str, now: i64) -> Option<i64> {
        let i = self.tpl.measures.iter().position(|m| m.name == name)?;
        Some(self.measures[i].value(&self.tpl.measures[i], now))
    }

    pub fn span_open(&self, name: &str) -> bool {
        self.tpl.spans.iter().position(|s| s.name == name).map_or(false, |i| self.spans[i].open)
    }

    pub fn cond(&self, c: &Cond, now: i64) -> bool {
        match c {
            Cond::Measure { measure, op, value } => self.measure(measure, now).map_or(false, |m| cmp(&Val::I(m), op, &Val::I(*value))),
            Cond::SpanOpen { span } => self.span_open(span),
            Cond::Not(c) => !self.cond(c, now),
        }
    }

    /// Append a world signal and run everything it causes. Returns the outputs in order.
    pub fn ingest(&mut self, log: &mut Log, kind: &str, fields: Fields, author: &str, t_ms: i64) -> Result<Vec<Output>, SignalError> {
        let s = log.append(kind, fields, author, t_ms)?;
        let mut out = vec![];
        self.process(log, s, &mut out);
        Ok(out)
    }

    /// Re-derive all state from a recorded log of world signals (determinism and projection).
    pub fn replay(tpl: Template, schema: Schema, roles: crate::signal::Roles, world: &[Signal]) -> (Engine, Log, Vec<Output>) {
        let mut log = Log::new(schema.clone(), roles);
        let mut e = Engine::load(tpl, &schema).expect("template");
        let mut out = vec![];
        for s in world.iter().filter(|s| s.stratum == 0 && !s.kind.starts_with("span.")) {
            out.extend(e.ingest(&mut log, &s.kind, s.fields.clone(), &s.author, s.t_ms).expect("valid"));
        }
        (e, log, out)
    }

    fn process(&mut self, log: &mut Log, s: Signal, out: &mut Vec<Output>) {
        if s.authority != Authority::Fact { return; } // claims are kept, never acted on (A5)
        // 1. measures
        for (i, m) in self.tpl.measures.iter().enumerate() {
            let d: i64 = m.terms.iter().filter(|t| t.of.matches(&s)).map(|t| t.coef * field_i(&s, &t.field)).sum();
            if d != 0 { self.measures[i].add(m, s.t_ms, d); }
        }
        // 2. spans (may emit span.enter / span.exit, processed recursively)
        let mut span_events = vec![];
        for (i, d) in self.tpl.spans.iter().enumerate() {
            let st = self.spans[i].clone();
            let parent_open = d.parent.as_ref().map_or(true, |p| self.span_open(p));
            if !st.open && parent_open && (d.repeat || st.n == 0) && d.opens.matches(&s) {
                self.spans[i].open = true; self.spans[i].n += 1;
                span_events.push(("span.enter", d.name.clone(), self.spans[i].n));
            } else if st.open && d.closes.matches(&s) {
                self.spans[i].open = false;
                span_events.push(("span.exit", d.name.clone(), st.n));
                // closing a parent closes its children
                for (j, c) in self.tpl.spans.iter().enumerate() {
                    if c.parent.as_deref() == Some(&d.name) && self.spans[j].open {
                        self.spans[j].open = false;
                        span_events.push(("span.exit", c.name.clone(), self.spans[j].n));
                    }
                }
            }
        }
        // 3. rules that may read this signal
        for ri in 0..self.tpl.rules.len() {
            if self.tpl.rules[ri].stratum <= s.stratum { continue; }
            let scope = match &self.tpl.rules[ri].per {
                None => String::new(),
                Some(f) => s.fields.get(f).map(Val::key).unwrap_or_default(),
            };
            let mut st = self.state[ri].remove(&scope).unwrap_or_default();
            let pat = self.tpl.rules[ri].when.clone();
            let hit = self.step(&pat, &mut st, &s);
            self.state[ri].insert(scope.clone(), st);
            if let Some((key, n)) = hit {
                let r = &self.tpl.rules[ri];
                if r.guard.as_ref().map_or(false, |c| !self.cond(c, s.t_ms)) { continue; }
                if r.unless.as_ref().map_or(false, |c| self.cond(c, s.t_ms)) { continue; }
                if let Some(th) = r.throttle_ms {
                    if let Some(last) = self.last_fire[ri].get(&scope) { if s.t_ms - last < th { continue; } }
                }
                self.last_fire[ri].insert(scope.clone(), s.t_ms);
                let ctx = Ctx { key, n, sig: &s, scope };
                self.fire(log, ri, &ctx, out);
            }
        }
        for (kind, span, n) in span_events {
            let sig = log.append_derived(kind, crate::fields!{"span" => span, "n" => n}, "system", s.t_ms, 0).expect("span schema");
            self.process(log, sig, out);
        }
    }

    fn fire(&mut self, log: &mut Log, ri: usize, ctx: &Ctx, out: &mut Vec<Output>) {
        let r = self.tpl.rules[ri].clone();
        for e in &r.then {
            match e {
                Effect::Emit { kind, fields } => {
                    let f: Fields = fields.iter().map(|(k, v)| (k.clone(), subst(v, ctx))).collect();
                    let sig = match log.append_derived(kind, f.clone(), &format!("rule:{}", r.name), ctx.sig.t_ms, r.stratum) {
                        Ok(s) => s,
                        Err(_) => continue, // a substitution produced a value the schema refuses: effect rejected, rule continues
                    };
                    out.push(Output::Emitted { seq: sig.seq, rule: r.name.clone(), kind: kind.clone(), fields: f });
                    self.process(log, sig, out);
                }
                Effect::Ask { id, question, options, otherwise, locks_ms, points } => out.push(Output::Ask {
                    ask: id.clone(), question: question.replace("$n", &ctx.n.map(|n| n.to_string()).unwrap_or_default()), options: options.clone(),
                    otherwise: otherwise.clone(), locks_ms: *locks_ms, points: *points, t_ms: ctx.sig.t_ms }),
                Effect::Settle { ask } => out.push(Output::Settle { ask: ask.clone(), t_ms: ctx.sig.t_ms }),
                Effect::Offer { thing, price, discount, payer, expires_ms, limits } => out.push(Output::Offer { limits: limits.clone(),
                    rule: r.name.clone(), scope: ctx.scope.clone(), thing: thing.clone(), price: *price, discount: *discount,
                    payer: payer.clone(), expires_ms: *expires_ms, t_ms: ctx.sig.t_ms,
                }),
            }
        }
    }

    /// Advance one pattern's state by one signal. Returns Some((key, span n)) when it fires.
    fn step(&self, p: &Pattern, st: &mut PState, s: &Signal) -> Option<(Option<Val>, Option<i64>)> {
        match p {
            Pattern::Match(sel) => sel.matches(s).then_some((None, None)),
            Pattern::Count { of, gte } => {
                if !of.matches(s) { return None; }
                st.count += 1;
                (st.count == *gte).then_some((None, None))
            }
            Pattern::Every { n, of } => {
                if !of.matches(s) { return None; }
                st.count += 1;
                (st.count % n == 0).then_some((None, None))
            }
            Pattern::Streak { of, key, field, gte } => {
                if !of.matches(s) { return None; }
                let k = s.fields.get(key).cloned();
                if st.streak_key != k { st.streak_key = k.clone(); st.streak_sum = 0; st.fired = false; }
                st.streak_sum += field_i(s, field);
                if !st.fired && st.streak_sum >= *gte { st.fired = true; return Some((k, None)); }
                None
            }
            Pattern::Changes { measure } => {
                let v = self.measure(measure, s.t_ms).unwrap_or(0).signum();
                let fired = v != 0 && st.last_sign != 0 && v != st.last_sign;
                if v != 0 { st.last_sign = v; }
                fired.then_some((None, None))
            }
            Pattern::Crosses { measure, above } => {
                let v = self.measure(measure, s.t_ms).unwrap_or(0);
                let now_above = v > *above;
                let fired = now_above && !st.above;
                st.above = now_above;
                fired.then_some((None, None))
            }
            Pattern::Seq { first, then, within_ms } => {
                if then.matches(s) {
                    if let Some(t0) = st.first_at { st.first_at = None; if s.t_ms - t0 <= *within_ms { return Some((None, None)); } }
                }
                if first.matches(s) { st.first_at = Some(s.t_ms); }
                None
            }
            Pattern::Idle { of, ms } => {
                if of.matches(s) { st.last_seen = Some(s.t_ms); st.idle_fired = false; return None; }
                match st.last_seen {
                    Some(t0) if !st.idle_fired && s.t_ms - t0 >= *ms => { st.idle_fired = true; Some((None, None)) }
                    _ => None,
                }
            }
            Pattern::Enter { span, n } | Pattern::Exit { span, n } => {
                let kind = if matches!(p, Pattern::Enter { .. }) { "span.enter" } else { "span.exit" };
                if s.kind != kind || s.fields.get("span") != Some(&Val::S(span.clone())) { return None; }
                let sn = s.fields.get("n").and_then(Val::as_i);
                if n.is_some() && sn != *n { return None; }
                Some((None, sn))
            }
            Pattern::Any(ps) => {
                if st.any.len() < ps.len() { st.any.resize(ps.len(), PState::default()); }
                let mut hit = None;
                for (i, q) in ps.iter().enumerate() {
                    let mut sub = std::mem::take(&mut st.any[i]);
                    let h = self.step(q, &mut sub, s);
                    st.any[i] = sub;
                    if hit.is_none() { hit = h; }
                }
                hit
            }
        }
    }
}

fn subst(v: &Val, ctx: &Ctx) -> Val {
    match v {
        Val::S(x) if x == "$key" => ctx.key.clone().unwrap_or(Val::S(String::new())),
        Val::S(x) if x == "$scope" => Val::S(ctx.scope.clone()),
        Val::S(x) if x == "$n" => Val::I(ctx.n.unwrap_or(0)),
        Val::S(x) if x.starts_with("$f.") => ctx.sig.fields.get(&x[3..]).cloned().unwrap_or(Val::S(String::new())),
        other => other.clone(),
    }
}

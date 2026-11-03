//! Agape's server: accounts, one thread per venue running the kernel, a journal per venue on disk,
//! and a small JSON API. Phones call the same `view` and `act` the in-page kernel exposes, but the
//! server decides who is asking from the session, and which screens and actions that role has.

pub mod auth;
pub mod venue;
pub mod passport;

use auth::{Account, Auth, Role};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::stream::Stream;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

pub struct App {
    auth: Mutex<Auth>,
    passports: tokio::sync::Mutex<passport::Passports>,
    venues: Mutex<BTreeMap<String, venue::Handle>>,
    data: std::path::PathBuf,
    speed: f64,
    admin_token: Option<String>,
}
pub type S = Arc<App>;

impl App {
    pub fn venue(&self, id: &str) -> Option<venue::Handle> { self.venues.lock().unwrap().get(id).cloned() }
    /// Stop every venue's thread after writing out its journal.
    pub fn shutdown(&self) { for v in self.venues.lock().unwrap().values() { v.stop(); } }
}

fn err(code: StatusCode, msg: &str) -> Response { (code, Json(json!({ "ok": false, "error": msg }))).into_response() }

fn account(app: &App, h: &HeaderMap) -> Result<Account, Response> {
    let token = h.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("");
    app.auth.lock().unwrap().session(token).cloned().ok_or_else(|| err(StatusCode::UNAUTHORIZED, "Sign in first"))
}

/// Which screens each role may open. Role screens are always the account's own.
fn view_allowed(role: Role, lens: &str) -> bool {
    match role {
        Role::Fan => matches!(lens, "fan" | "page"),
        Role::Runner => matches!(lens, "runner" | "page"),
        Role::Stand => matches!(lens, "stand"),
        Role::Business => matches!(lens, "business" | "page"),
        Role::Player => matches!(lens, "player" | "page"),
        Role::Ops => matches!(lens, "ops" | "page" | "stand" | "runner"),
    }
}

/// Which actions each role may take.
fn act_allowed(role: Role, act: &str) -> bool {
    const FAN: &[&str] = &["order", "quote", "window", "tap", "checkout", "cancel", "predict", "collect", "list", "delist", "buy", "follow", "message", "unlock",
        "tickets", "ticket_quote", "ride", "ride_quote", "cancel_ride", "parking", "deal", "withdraw", "confirm", "chat", "join", "wear", "attest_age", "report", "rate", "reorder", "gift", "prioritize", "upgrade", "resell", "unlist_tickets", "buy_resale"];
    match role {
        Role::Fan => FAN.contains(&act),
        Role::Player => matches!(act, "respond" | "deliver" | "reply" | "message" | "follow" | "claim"),
        Role::Business => matches!(act, "deal" | "withdraw" | "confirm" | "redeem" | "reply" | "message"),
        Role::Runner => matches!(act, "clock_in" | "clock_out" | "accept_trip" | "decline_trip" | "picked_up" | "delivered" | "unreachable"),
        Role::Stand => matches!(act, "clock_in" | "clock_out" | "ready" | "cannot_make" | "handed_over"),
        Role::Ops => matches!(act, "clock_in" | "clock_out" | "halt" | "resume" | "resolve"),
    }
}

async fn signup(State(app): State<S>, Json(b): Json<Value>) -> Response {
    let (h, p) = (b["handle"].as_str().unwrap_or(""), b["password"].as_str().unwrap_or(""));
    let mut a = app.auth.lock().unwrap();
    match a.create(h, p, Role::Fan, None, None) {
        Ok(_) => match a.sign_in(h, p) { Ok((token, acc)) => Json(json!({ "ok": true, "token": token, "account": acc.public() })).into_response(), Err(e) => err(StatusCode::BAD_REQUEST, &e) },
        Err(e) => err(StatusCode::BAD_REQUEST, &e),
    }
}

async fn signin(State(app): State<S>, Json(b): Json<Value>) -> Response {
    let r = app.auth.lock().unwrap().sign_in(b["handle"].as_str().unwrap_or(""), b["password"].as_str().unwrap_or(""));
    match r { Ok((token, acc)) => Json(json!({ "ok": true, "token": token, "account": acc.public() })).into_response(), Err(e) => err(StatusCode::UNAUTHORIZED, &e) }
}

async fn me(State(app): State<S>, h: HeaderMap) -> Response {
    match account(&app, &h) { Ok(a) => Json(json!({ "ok": true, "account": a.public() })).into_response(), Err(r) => r }
}

async fn venues(State(app): State<S>) -> Response {
    Json(json!({ "venues": app.venues.lock().unwrap().values().map(|v| json!({ "id": v.id, "name": v.name })).collect::<Vec<_>>() })).into_response()
}

fn venue_for(app: &App, acc: &Account, id: &str) -> Result<venue::Handle, Response> {
    let v = app.venue(id).ok_or_else(|| err(StatusCode::NOT_FOUND, "No such venue"))?;
    if acc.role != Role::Fan && acc.venue.as_deref() != Some(id) { return Err(err(StatusCode::FORBIDDEN, "Your account works at another venue")); }
    Ok(v)
}

async fn view(State(app): State<S>, Path(vid): Path<String>, h: HeaderMap, Json(mut b): Json<Value>) -> Response {
    let acc = match account(&app, &h) { Ok(a) => a, Err(r) => return r };
    let v = match venue_for(&app, &acc, &vid) { Ok(v) => v, Err(r) => return r };
    let lens = b["lens"].as_str().unwrap_or("").to_string();
    if !view_allowed(acc.role, &lens) { return err(StatusCode::FORBIDDEN, "Not your screen"); }
    // a role's own screen is always its own party's; only ops may look at a stand or runner by id
    if lens != "page" && !(acc.role == Role::Ops && lens != "ops") { b["id"] = json!(acc.party); }
    Json(v.view(&acc.party, b).await).into_response()
}

async fn act(State(app): State<S>, Path(vid): Path<String>, h: HeaderMap, Json(mut b): Json<Value>) -> Response {
    let acc = match account(&app, &h) { Ok(a) => a, Err(r) => return r };
    let v = match venue_for(&app, &acc, &vid) { Ok(v) => v, Err(r) => return r };
    let what = b["do"].as_str().unwrap_or("").to_string();
    if !act_allowed(acc.role, &what) { return err(StatusCode::FORBIDDEN, "That isn't something your account can do"); }
    // nobody names who they are inside a request
    if let Some(o) = b.as_object_mut() {
        o.remove("as"); o.remove("party");
        // the name a person shows others is their account's, set when they join
        if what == "join" { o.insert("name".into(), json!(acc.handle)); }
    }
    if what == "join" { return join(&app, &acc, &v, b).await; }
    Json(v.act(&acc.party, b).await).into_response()
}

/// Joining a venue brings the fan's passport: their points leave wherever they were and arrive
/// here, and their level comes with them. Each step is written down first and is safe to repeat.
async fn join(app: &App, acc: &Account, v: &venue::Handle, mut b: Value) -> Response {
    let mut ps = app.passports.lock().await;
    let mut p = ps.all.get(&acc.id).cloned().unwrap_or_default();
    finish_move(app, &acc.party, &mut p).await;
    let first = p.nights == 0 && p.venues.is_empty();
    b["welcome"] = json!(first);
    let r = v.act(&acc.party, b).await;
    if r["ok"] != true { ps.all.insert(acc.id.clone(), p); ps.save(); return Json(r).into_response(); }
    if p.at.as_deref() != Some(&v.id) {
        // out of wherever the points are now
        if let Some(from) = p.at.clone() {
            p.seq += 1;
            p.pending = Some((from, format!("{}-{}-out", acc.id, p.seq), "out".into()));
            ps.all.insert(acc.id.clone(), p.clone()); ps.save();
            finish_move(app, &acc.party, &mut p).await;
        }
        // and into here
        p.seq += 1;
        if !p.venues.contains(&v.name) { p.venues.push(v.name.clone()); }
        p.nights += 1;
        p.pending = Some((v.id.clone(), format!("{}-{}-in", acc.id, p.seq), "in".into()));
        ps.all.insert(acc.id.clone(), p.clone()); ps.save();
        finish_move(app, &acc.party, &mut p).await;
    }
    ps.all.insert(acc.id.clone(), p); ps.save();
    Json(r).into_response()
}

/// Complete a move that was written down, by repeating it (venues ignore a transfer they've done).
async fn finish_move(app: &App, fan: &str, p: &mut passport::Passport) {
    let Some((venue, transfer, dir)) = p.pending.clone() else { return };
    let Some(h) = app.venue(&venue) else { p.pending = None; return };
    if dir == "out" {
        let r = h.act("agape", json!({ "do": "carry_out", "transfer": transfer, "fan": fan })).await;
        if let Some(c) = r["id"].as_str().and_then(|s| serde_json::from_str::<agape_kernel::passport::CarriedOut>(s).ok()) {
            p.points += c.points; p.level = c.level; p.at = None; p.pending = None;
        }
    } else {
        let r = h.act("agape", json!({ "do": "carry_in", "transfer": transfer, "fan": fan, "points": p.points, "level": p.level,
            "passport": { "venues": p.venues, "nights": p.nights } })).await;
        if r["ok"] == true { p.points = 0; p.at = Some(venue); p.pending = None; }
    }
}

/// Before signing in: the game and the night at a glance. Nothing about any person.
async fn glance(State(app): State<S>, Path(vid): Path<String>) -> Response {
    match app.venue(&vid) { Some(v) => Json(v.view("", json!({ "lens": "glance" })).await).into_response(), None => err(StatusCode::NOT_FOUND, "No such venue") }
}

async fn events(State(app): State<S>, Path(vid): Path<String>) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, Response> {
    let v = app.venue(&vid).ok_or_else(|| err(StatusCode::NOT_FOUND, "No such venue"))?;
    let rx = BroadcastStream::new(v.events.subscribe());
    let s = rx.filter_map(|m| m.ok()).map(|m| Ok(Event::default().event("tick").data(m)));
    Ok(Sse::new(s).keep_alive(KeepAlive::default()))
}

/// Operators create staff, business and player accounts; the role is the venue's to grant.
async fn admin_account(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    if !operator(&app, &h) { return err(StatusCode::FORBIDDEN, "Operators only"); }
    let role: Role = match serde_json::from_value(b["role"].clone()) { Ok(r) => r, Err(_) => return err(StatusCode::BAD_REQUEST, "Unknown role") };
    let r = app.auth.lock().unwrap().create(b["handle"].as_str().unwrap_or(""), b["password"].as_str().unwrap_or(""), role,
        b["party"].as_str().map(String::from), b["venue"].as_str().map(String::from));
    match r { Ok(a) => Json(json!({ "ok": true, "account": a.public() })).into_response(), Err(e) => err(StatusCode::BAD_REQUEST, &e) }
}

fn operator(app: &App, h: &HeaderMap) -> bool {
    let given = h.get("x-admin-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    app.admin_token.as_deref().map_or(false, |t| !t.is_empty() && t == given)
}

/// Open a new venue from its template. The template is checked first; nothing opens with a
/// blocking issue, and the operator gets every issue back in words.
async fn admin_venue(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    if !operator(&app, &h) { return err(StatusCode::FORBIDDEN, "Operators only"); }
    let id = b["id"].as_str().unwrap_or("").to_string();
    if !auth::valid_handle(&id) { return err(StatusCode::BAD_REQUEST, "Give the venue a short id of letters and numbers"); }
    if app.venue(&id).is_some() { return err(StatusCode::BAD_REQUEST, "That venue is already open"); }
    let (w, t) = match (serde_json::from_value(b["world"].clone()), serde_json::from_value(b["template"].clone())) { (Ok(w), Ok(t)) => (w, t), _ => return err(StatusCode::BAD_REQUEST, "The template can't be read") };
    let issues = agape_kernel::setup::check(&w, &t);
    if issues.iter().any(|i| i.blocking) { return (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": "The template has problems to fix", "issues": issues }))).into_response(); }
    let init = venue::Init { id: id.clone(), name: b["name"].as_str().unwrap_or(&id).to_string(), world: b["world"].clone(), template: b["template"].clone(),
        seed: b["seed"].as_u64().unwrap_or(1), fans: b["fans"].as_u64().unwrap_or(3000) as usize, runners: b["runners"].as_u64().unwrap_or(40) as usize, start_min: b["start_min"].as_i64().unwrap_or(0), build: venue::BUILD.to_string() };
    match venue::spawn(&app.data.join("venues"), Some(init), &id, app.speed) {
        Ok(hd) => { app.venues.lock().unwrap().insert(id.clone(), hd); Json(json!({ "ok": true, "id": id, "issues": issues })).into_response() }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// One rule, installed in every open venue, each venue funding its share of the budget.
async fn admin_league_deal(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    if !operator(&app, &h) { return err(StatusCode::FORBIDDEN, "Operators only"); }
    let venues: Vec<venue::Handle> = app.venues.lock().unwrap().values().cloned().collect();
    let mut out = serde_json::Map::new();
    for v in venues {
        let r = v.act("agape", json!({ "do": "install_rule", "rule": b["rule"], "payer": b["payer"], "budget": b["budget_per_venue"] })).await;
        out.insert(v.id.clone(), r);
    }
    let ok = out.values().all(|r| r["ok"] == true);
    (if ok { StatusCode::OK } else { StatusCode::BAD_REQUEST }, Json(json!({ "ok": ok, "venues": out }))).into_response()
}

pub fn router(app: S, static_dir: Option<String>) -> Router {
    let api = Router::new()
        .route("/api/signup", post(signup))
        .route("/api/signin", post(signin))
        .route("/api/me", get(me))
        .route("/api/venues", get(venues))
        .route("/api/v/{venue}/view", post(view))
        .route("/api/v/{venue}/act", post(act))
        .route("/api/v/{venue}/events", get(events))
        .route("/api/v/{venue}/glance", get(glance))
        .route("/api/admin/accounts", post(admin_account))
        .route("/api/admin/venues", post(admin_venue))
        .route("/api/admin/league-deal", post(admin_league_deal))
        .with_state(app);
    match static_dir { Some(d) => api.fallback_service(tower_http::services::ServeDir::new(d)), None => api }
}

/// The pilot venue, from the kernel's own templates.
pub fn harbor_init(seed: u64) -> venue::Init {
    venue::Init {
        id: "harbor".into(), name: "Harbor Arena".into(),
        world: serde_json::from_str(include_str!("../../agape-kernel/templates/arena.json")).unwrap(),
        template: serde_json::from_str(include_str!("../../agape-kernel/templates/basketball.json")).unwrap(),
        seed, fans: 3000, runners: 40, start_min: 38, build: venue::BUILD.to_string(),
    }
}

pub fn app(data: &std::path::Path, ticks_per_sec: f64, admin_token: Option<String>) -> S {
    std::fs::create_dir_all(data).unwrap();
    let mut venues = BTreeMap::new();
    // every venue ever opened comes back from its journal; the pilot venue is opened the first time
    let dir = data.join("venues");
    std::fs::create_dir_all(&dir).unwrap();
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if let Some(id) = name.strip_suffix(".journal") {
            match venue::spawn(&dir, None, id, ticks_per_sec) { Ok(h) => { venues.insert(h.id.clone(), h); } Err(e) => eprintln!("venue {id} didn't come back: {e}") }
        }
    }
    if !venues.contains_key("harbor") {
        let h = venue::spawn(&dir, Some(harbor_init(7)), "harbor", ticks_per_sec).expect("venue starts");
        venues.insert(h.id.clone(), h);
    }
    Arc::new(App { auth: Mutex::new(Auth::open(data.join("accounts.jsonl"))), venues: Mutex::new(venues), passports: tokio::sync::Mutex::new(passport::Passports::open(data.join("passports.json"))), data: data.to_path_buf(), speed: ticks_per_sec, admin_token })
}

/// A venue whose night is over opens the next one: every fan's points go home to their passport,
/// the finished night's journal is kept under a new name, and a new night starts from the same
/// template with a new seed. Fans pick their seat again and their points come back with them.
pub async fn roll_finished(app: &App) {
    let done: Vec<venue::Handle> = app.venues.lock().unwrap().values().filter(|v| v.over.load(std::sync::atomic::Ordering::Relaxed)).cloned().collect();
    for v in done {
        {
            let mut ps = app.passports.lock().await;
            let here: Vec<String> = ps.all.iter().filter(|(_, p)| p.at.as_deref() == Some(&v.id) && p.pending.is_none()).map(|(k, _)| k.clone()).collect();
            for id in here {
                let party = app.auth.lock().unwrap().party_of(&id).unwrap_or_default();
                let mut p = ps.all[&id].clone();
                p.seq += 1;
                p.pending = Some((v.id.clone(), format!("{}-{}-out", id, p.seq), "out".into()));
                ps.all.insert(id.clone(), p.clone()); ps.save();
                finish_move(app, &party, &mut p).await;
                ps.all.insert(id, p); ps.save();
            }
        }
        v.stop();
        let dir = app.data.join("venues");
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = std::fs::rename(dir.join(format!("{}.journal", v.id)), dir.join(format!("{}.{stamp}.night", v.id)));
        let mut init = v.init.clone();
        init.seed += 1;
        init.build = venue::BUILD.to_string();
        match venue::spawn(&dir, Some(init), &v.id, app.speed) {
            Ok(h) => { app.venues.lock().unwrap().insert(v.id.clone(), h); println!("{}: night over, the next one is open", v.id); }
            Err(e) => eprintln!("{}: couldn't open the next night: {e}", v.id),
        }
    }
}

/// Demo staff so the pilot can be tried end to end. Created once; passwords come from the environment.
/// Finish any passport move a crash interrupted.
pub async fn recover(app: &App) {
    let mut ps = app.passports.lock().await;
    let ids: Vec<String> = ps.all.iter().filter(|(_, p)| p.pending.is_some()).map(|(k, _)| k.clone()).collect();
    for id in ids {
        let party = app.auth.lock().unwrap().party_of(&id).unwrap_or_default();
        let mut p = ps.all[&id].clone();
        finish_move(app, &party, &mut p).await;
        ps.all.insert(id, p);
    }
    ps.save();
}

pub fn demo_accounts(app: &App, password: &str) {
    let mut a = app.auth.lock().unwrap();
    for (handle, role, party) in [("runner0", Role::Runner, "runner0"), ("grill", Role::Stand, "grill-104"), ("ops", Role::Ops, "ops"),
                                  ("dockpizza", Role::Business, "dock-pizza"), ("harborline", Role::Business, "harborline"), ("jalen", Role::Player, "hbk-1")] {
        let _ = a.create(handle, password, role, Some(party.into()), Some("harbor".into()));
    }
}


//! Agape's server: accounts, one thread per venue running the kernel, a journal per venue on disk,
//! and a small JSON API. Phones call the same `view` and `act` the in-page kernel exposes, but the
//! server decides who is asking from the session, and which screens and actions that role has.

pub mod auth;
pub mod venue;
pub mod passport;
pub mod admin;
pub mod mail;
pub mod pilot;
pub mod ops;

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
    office: Mutex<admin::Admin>,
    pub mail: mail::Mail,
    limits: Mutex<Limits>,
}
pub type S = Arc<App>;

impl App {
    pub fn data_dir(&self) -> std::path::PathBuf { self.data.clone() }
    pub fn venue(&self, id: &str) -> Option<venue::Handle> { self.venues.lock().unwrap().get(id).cloned() }
    /// Stop every venue's thread after writing out its journal.
    pub fn shutdown(&self) { for v in self.venues.lock().unwrap().values() { v.stop(); } }
}

pub fn now_s() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }

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
        Role::Ops => matches!(lens, "ops" | "page" | "stand" | "runner" | "night"),
    }
}

/// Which actions each role may take.
fn act_allowed(role: Role, act: &str) -> bool {
    const FAN: &[&str] = &["order", "quote", "window", "tap", "checkout", "cancel", "predict", "collect", "list", "delist", "buy", "follow", "message", "unlock",
        "tickets", "ticket_quote", "ride", "ride_quote", "cancel_ride", "parking", "deal", "withdraw", "confirm", "chat", "join", "wear", "attest_age", "cart", "cart_quote", "spot", "report", "rate", "reorder", "gift", "prioritize", "upgrade", "resell", "unlist_tickets", "buy_resale"];
    match role {
        Role::Fan => FAN.contains(&act),
        Role::Player => matches!(act, "respond" | "deliver" | "reply" | "message" | "follow" | "claim"),
        Role::Business => matches!(act, "deal" | "withdraw" | "confirm" | "redeem" | "reply" | "message"),
        Role::Runner => matches!(act, "clock_in" | "clock_out" | "accept_trip" | "decline_trip" | "picked_up" | "delivered" | "unreachable"),
        Role::Stand => matches!(act, "clock_in" | "clock_out" | "ready" | "cannot_make" | "handed_over"),
        Role::Ops => matches!(act, "clock_in" | "clock_out" | "halt" | "resume" | "resolve" | "game" | "score"),
    }
}

/// Who is asking, as far as limits go: Cloudflare's view of the address, or the connection's.
fn client_ip(h: &HeaderMap) -> String { h.get("cf-connecting-ip").or_else(|| h.get("x-forwarded-for")).and_then(|v| v.to_str().ok()).map(|s| s.split(',').next().unwrap_or("").trim().to_string()).unwrap_or_else(|| "local".into()) }

/// Counts per key in a window. `hit` says whether one more is allowed (and counts it).
#[derive(Default)]
pub struct Limits { seen: std::collections::HashMap<String, (u32, u64)> }
impl Limits {
    pub fn hit(&mut self, key: &str, max: u32, window_s: u64) -> bool {
        let now = now_s();
        if self.seen.len() > 50_000 { self.seen.retain(|_, v| v.1 + 3600 > now); }
        let e = self.seen.entry(key.to_string()).or_insert((0, now));
        if e.1 + window_s <= now { *e = (0, now); }
        if e.0 >= max { return false; }
        e.0 += 1; true
    }
    pub fn clear(&mut self, key: &str) { self.seen.remove(key); }
}
const SLOW_DOWN: &str = "Too many tries. Wait a few minutes and try again.";

async fn signup(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let (hd, p) = (b["handle"].as_str().unwrap_or("").to_string(), b["password"].as_str().unwrap_or("").to_string());
    // a whole venue can share one address (its Wi-Fi), so this only stops a script
    if !app.limits.lock().unwrap().hit(&format!("signup:{}", client_ip(&h)), 5000, 3600) { return err(StatusCode::TOO_MANY_REQUESTS, SLOW_DOWN); }
    if let Err(e) = app.auth.lock().unwrap().can_create(&hd, &p) { return err(StatusCode::BAD_REQUEST, &e); }
    // hashing takes a moment of CPU: never while holding the lock every request needs
    let pw = p.clone();
    let hash = match tokio::task::spawn_blocking(move || auth::hash_password(&pw)).await { Ok(Ok(x)) => x, _ => return err(StatusCode::INTERNAL_SERVER_ERROR, "Couldn't make the account; try again") };
    let mut a = app.auth.lock().unwrap();
    match a.create_hashed(&hd, hash, Role::Fan, None, None) {
        Ok(acc) => match a.open_session(&acc.id) { Some((token, acc)) => Json(json!({ "ok": true, "token": token, "account": acc.public() })).into_response(), None => err(StatusCode::INTERNAL_SERVER_ERROR, "Try signing in") },
        Err(e) => err(StatusCode::BAD_REQUEST, &e),
    }
}

async fn signin(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let (hd, p) = (b["handle"].as_str().unwrap_or("").trim().to_lowercase(), b["password"].as_str().unwrap_or("").to_string());
    {
        let mut l = app.limits.lock().unwrap();
        // ten wrong passwords for one name in fifteen minutes, and it waits: guessing a staff password is slow
        if !l.hit(&format!("signin-ip:{}", client_ip(&h)), 300, 900) || !l.hit(&format!("signin-try:{hd}"), 10, 900) { return err(StatusCode::TOO_MANY_REQUESTS, SLOW_DOWN); }
    }
    let wrong = || err(StatusCode::UNAUTHORIZED, "That name and password don't match");
    let Some((id, hash)) = app.auth.lock().unwrap().credentials(&hd) else { return wrong() };
    let pw = p.clone();
    let ok = tokio::task::spawn_blocking(move || auth::verify_password(&hash, &pw)).await.unwrap_or(false);
    if !ok { return wrong(); }
    app.limits.lock().unwrap().clear(&format!("signin-try:{hd}"));
    match app.auth.lock().unwrap().open_session(&id) { Some((token, acc)) => Json(json!({ "ok": true, "token": token, "account": acc.public() })).into_response(), None => wrong() }
}

async fn signout(State(app): State<S>, h: HeaderMap) -> Response {
    let token = h.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("");
    app.auth.lock().unwrap().sign_out(token);
    Json(json!({ "ok": true })).into_response()
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
    // one phone asking hundreds of times a minute would slow the venue for everyone
    if !app.limits.lock().unwrap().hit(&format!("view:{}", acc.id), 120, 10) { return err(StatusCode::TOO_MANY_REQUESTS, SLOW_DOWN); }
    let lens = b["lens"].as_str().unwrap_or("").to_string();
    if !view_allowed(acc.role, &lens) { return err(StatusCode::FORBIDDEN, "Not your screen"); }
    // a role's own screen is always its own party's; only ops may look at a stand or runner by id
    if lens != "page" && !(acc.role == Role::Ops && lens != "ops") { b["id"] = json!(acc.party); }
    let mut out = v.view(&acc.party, b).await;
    // every screen knows whether tonight is real, a rehearsal, or the simulated demo
    if let Some(o) = out.as_object_mut() { o.insert("venue_mode".into(), json!(v.init.mode)); o.insert("venue_id".into(), json!(v.id)); }
    Json(out).into_response()
}

async fn act(State(app): State<S>, Path(vid): Path<String>, h: HeaderMap, Json(mut b): Json<Value>) -> Response {
    let acc = match account(&app, &h) { Ok(a) => a, Err(r) => return r };
    let v = match venue_for(&app, &acc, &vid) { Ok(v) => v, Err(r) => return r };
    let what = b["do"].as_str().unwrap_or("").to_string();
    if !app.limits.lock().unwrap().hit(&format!("act:{}", acc.id), 40, 10) { return err(StatusCode::TOO_MANY_REQUESTS, SLOW_DOWN); }
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

/// A venue's own version from the Studio, saved so it can be sent as a link. The simulator opens
/// it in the visitor's browser; nothing on the server runs it. Checked before it's kept.
async fn studio_save(State(app): State<S>, body: axum::body::Bytes) -> Response {
    if body.len() > 3_000_000 { return err(StatusCode::PAYLOAD_TOO_LARGE, "That's too big to share; try smaller pictures"); }
    let w: agape_kernel::world::WorldDef = match serde_json::from_slice(&body) { Ok(w) => w, Err(_) => return err(StatusCode::BAD_REQUEST, "That isn't a venue the Studio made") };
    let tpl: agape_kernel::rules::Template = serde_json::from_str(include_str!("../../agape-kernel/templates/basketball.json")).unwrap();
    if agape_kernel::setup::check(&w, &tpl).iter().any(|i| i.blocking) { return err(StatusCode::BAD_REQUEST, "Fix what the Studio flags first"); }
    let dir = app.data.join("studio");
    let _ = std::fs::create_dir_all(&dir);
    let mut b = [0u8; 6];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut b);
    let id: String = b.iter().map(|x| format!("{x:02x}")).collect();
    match std::fs::write(dir.join(format!("{id}.json")), &body) { Ok(_) => Json(json!({ "ok": true, "id": id })).into_response(), Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "Couldn't save it") }
}

async fn studio_load(State(app): State<S>, Path(id): Path<String>) -> Response {
    if !id.chars().all(|c| c.is_ascii_hexdigit()) || id.len() != 12 { return err(StatusCode::NOT_FOUND, "No such version"); }
    match std::fs::read(app.data.join("studio").join(format!("{id}.json"))) {
        Ok(b) => ([("content-type", "application/json")], b).into_response(),
        Err(_) => err(StatusCode::NOT_FOUND, "No such version"),
    }
}

/// Someone who wants Agape at their venue, or wants to talk. Kept for the team; never shown publicly.
async fn lead(State(app): State<S>, Json(b): Json<Value>) -> Response {
    let get = |k: &str, n: usize| b[k].as_str().unwrap_or("").trim().chars().take(n).collect::<String>();
    let (name, contact) = (get("name", 80), get("contact", 120));
    if name.is_empty() || contact.is_empty() { return err(StatusCode::BAD_REQUEST, "Add your name and an email or phone number"); }
    // something we can actually answer: an email address, or a phone number
    let email = { let p: Vec<&str> = contact.split('@').collect(); p.len() == 2 && !p[0].is_empty() && p[1].contains('.') && !p[1].starts_with('.') && !p[1].ends_with('.') && !contact.contains(' ') };
    let phone = contact.chars().filter(|c| c.is_ascii_digit()).count() >= 10;
    if !email && !phone { return err(StatusCode::BAD_REQUEST, "Add an email address or a phone number we can reach you on"); }
    let at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let rec = json!({ "at": at, "name": name, "contact": contact, "org": get("org", 120), "role": get("role", 40), "note": get("note", 1000), "version": get("version", 20) });
    let mut line = rec.to_string(); line.push('\n');
    use std::io::Write;
    match std::fs::OpenOptions::new().create(true).append(true).open(app.data.join("leads.jsonl")).and_then(|mut f| f.write_all(line.as_bytes())) {
        Ok(_) => {
            let who = [rec["role"].as_str().unwrap_or(""), rec["org"].as_str().unwrap_or("")].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(", ");
            app.office.lock().unwrap().notify_perm("leads", "lead", &format!("New lead: {}", rec["name"].as_str().unwrap_or("")), &format!("{}{} asked to talk. Reach them at {}.{}", rec["name"].as_str().unwrap_or(""), if who.is_empty() { String::new() } else { format!(" ({who})") }, rec["contact"].as_str().unwrap_or(""), rec["note"].as_str().filter(|n| !n.is_empty()).map(|n| format!("\n\n“{n}”")).unwrap_or_default()));
            Json(json!({ "ok": true })).into_response()
        }
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "Couldn't save that; please try again"),
    }
}

async fn admin_leads(State(app): State<S>, h: HeaderMap) -> Response {
    if !operator(&app, &h) { return err(StatusCode::FORBIDDEN, "Operators only"); }
    let all: Vec<Value> = std::fs::read_to_string(app.data.join("leads.jsonl")).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    Json(json!({ "ok": true, "leads": all })).into_response()
}

/// Before signing in: the game and the night at a glance. Nothing about any person.
async fn glance(State(app): State<S>, Path(vid): Path<String>) -> Response {
    match app.venue(&vid) {
        Some(v) => {
            let mut g = v.view("", json!({ "lens": "glance" })).await;
            // what the join screen needs: the venue's own sections, and whether tonight is real
            if let Some(o) = g.as_object_mut() {
                o.insert("venue_id".into(), json!(v.id)); o.insert("mode".into(), json!(v.init.mode));
                o.insert("levels".into(), v.init.world["levels"].clone()); o.insert("sections_per_level".into(), v.init.world["sections_per_level"].clone());
            }
            Json(g).into_response()
        }
        None => err(StatusCode::NOT_FOUND, "No such venue"),
    }
}

/// The venue as its template describes it (names, stands, menu, sections): public, for the apps to draw it.
async fn venue_world(State(app): State<S>, Path(vid): Path<String>) -> Response {
    match app.venue(&vid) {
        Some(v) => {
            let mut w = v.init.world.clone();
            // how fans pay tonight, as the venue's team wrote it
            if let Some(d) = app.office.lock().unwrap().venue_draft(&vid) { w["pay_note"] = d.form["pay_note"].clone(); }
            w["mode"] = json!(v.init.mode);
            if v.init.real() {
                w["opened"] = json!(v.init.opened);
                // runners are shown to fans by their first name: "rupp.maria" is Maria
                let names: serde_json::Map<String, Value> = app.auth.lock().unwrap().at_venue(&vid).into_iter().filter(|a| a.role == Role::Runner).map(|a| {
                    let n = a.handle.rsplit('.').next().unwrap_or(&a.handle).replace('_', " ");
                    let mut c = n.chars(); let n = c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default();
                    (a.party, json!(n))
                }).collect();
                w["staff_names"] = Value::Object(names);
            }
            Json(w).into_response()
        }
        None => err(StatusCode::NOT_FOUND, "No such venue"),
    }
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
        seed: b["seed"].as_u64().unwrap_or(1), fans: b["fans"].as_u64().unwrap_or(3000) as usize, runners: b["runners"].as_u64().unwrap_or(40) as usize, start_min: b["start_min"].as_i64().unwrap_or(0), build: venue::BUILD.to_string(), mode: String::new(), opened: now_s() };
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
        .route("/api/signout", post(signout))
        .route("/api/venues", get(venues))
        .route("/api/v/{venue}/view", post(view))
        .route("/api/v/{venue}/act", post(act))
        .route("/api/v/{venue}/events", get(events))
        .route("/api/v/{venue}/glance", get(glance))
        .route("/api/v/{venue}/world", get(venue_world))
        .route("/api/studio", post(studio_save))
        .route("/api/leads", post(lead))
        .route("/api/admin/leads", get(admin_leads))
        .route("/api/studio/{id}", get(studio_load))
        .route("/api/admin/accounts", post(admin_account))
        .route("/api/admin/venues", post(admin_venue))
        .route("/api/admin/league-deal", post(admin_league_deal))
        .route("/api/office/state", get(office_state))
        .route("/api/office/setup", post(office_setup))
        .route("/api/office/signin", post(office_signin))
        .route("/api/office/signout", post(office_signout))
        .route("/api/office/me", get(office_me))
        .route("/api/office/password", post(office_password))
        .route("/api/office/team", get(office_team))
        .route("/api/office/invite", post(office_invite))
        .route("/api/office/invite/cancel", post(office_invite_cancel))
        .route("/api/office/invitation/{code}", get(office_invitation))
        .route("/api/office/accept", post(office_accept))
        .route("/api/office/user", post(office_user))
        .route("/api/office/merchants", get(office_merchants))
        .route("/api/office/merchant", post(office_merchant))
        .route("/api/office/merchant/delete", post(office_merchant_delete))
        .route("/api/office/printed", post(office_printed))
        .route("/api/office/leads", get(office_leads))
        .route("/api/office/activity", get(office_activity))
        .route("/api/office/inbox", get(office_inbox))
        .route("/api/office/accounts", get(office_accounts))
        .route("/api/office/email", post(office_email))
        .route("/api/office/mailbox", post(office_mailbox))
        .route("/api/office/mail/setup", get(office_mail_setup))
        .route("/api/office/mail/test", post(office_mail_test))
        .route("/api/mail/health", get(mail_health))
        .route("/api/mail/inbound", post(mail_inbound).layer(axum::extract::DefaultBodyLimit::max(30_000_000)))
        .route("/api/office/role", post(office_role))
        .route("/api/office/role/delete", post(office_role_delete))
        .route("/api/office/template", post(office_template))
        .route("/api/office/template/delete", post(office_template_delete))
        .route("/api/office/group", post(office_group))
        .route("/api/office/group/delete", post(office_group_delete))
        .route("/api/office/send", post(office_send))
        .route("/api/office/read", post(office_read))
        .route("/api/office/mute", post(office_mute))
        .route("/api/office/user/delete", post(office_user_delete))
        .route("/api/office/attachment/{note}/{index}", get(office_attachment))
        .route("/api/office/trouble", post(office_trouble).get(office_trouble_list))
        .route("/api/health", get(health))
        .route("/api/office/backup", get(office_backup))
        .route("/api/office/venues", get(office_venues))
        .route("/api/office/venue", post(office_venue_save))
        .route("/api/office/venue/check", post(office_venue_check))
        .route("/api/office/venue/delete", post(office_venue_delete))
        .route("/api/office/venue/open", post(office_venue_open))
        .route("/api/office/venue/staff", post(office_venue_staff))
        .route("/api/office/venue/staff/reset", post(office_venue_staff_reset))
        .route("/api/office/night/{venue}", get(office_night))
        .route("/api/office/night/{venue}/act", post(office_night_act))
        .route("/api/office/night/{venue}/end", post(office_night_end))
        .route("/api/office/report/{file}", get(office_report))
        .layer(axum::extract::DefaultBodyLimit::max(3_500_000))
        .with_state(app);
    let r = match static_dir { Some(d) => api.fallback_service(tower_http::services::ServeDir::new(d)), None => api };
    // every response: no sniffing types, no framing by other sites, and links don't leak paths
    use axum::http::{header, HeaderValue};
    use tower_http::set_header::SetResponseHeaderLayer as H;
    r.layer(H::if_not_present(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
     .layer(H::if_not_present(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
     .layer(H::if_not_present(header::REFERRER_POLICY, HeaderValue::from_static("strict-origin-when-cross-origin")))
     .layer(H::if_not_present(header::HeaderName::from_static("permissions-policy"), HeaderValue::from_static("camera=(), microphone=(), geolocation=()")))
}

/// The pilot venue, from the kernel's own templates.
pub fn harbor_init(seed: u64) -> venue::Init {
    venue::Init {
        id: "harbor".into(), name: "Harbor Arena".into(),
        world: serde_json::from_str(include_str!("../../agape-kernel/templates/arena.json")).unwrap(),
        template: serde_json::from_str(include_str!("../../agape-kernel/templates/basketball.json")).unwrap(),
        seed, fans: 3000, runners: 40, start_min: 38, build: venue::BUILD.to_string(), mode: String::new(), opened: now_s(),
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
    Arc::new(App { auth: Mutex::new(Auth::open(data.join("accounts.jsonl"))), venues: Mutex::new(venues), passports: tokio::sync::Mutex::new(passport::Passports::open(data.join("passports.json"))), data: data.to_path_buf(), speed: ticks_per_sec, admin_token, office: Mutex::new(admin::Admin::open(data.to_path_buf())), limits: Mutex::new(Limits::default()), mail: mail::Mail::open(data, &std::env::var("AGAPE_MAIL_DOMAIN").ok().filter(|d| !d.is_empty()).unwrap_or_else(|| "agapetoyourseat.com".into())) })
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
        if v.init.real() { archive_real(app, &v).await; continue; }
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


// ---------------- Agape Admin (the company's own accounts, merchants and VAR sheets) ----------------

fn bearer(h: &HeaderMap) -> String { h.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("").to_string() }
fn staffer(app: &App, h: &HeaderMap) -> Result<admin::User, Response> { app.office.lock().unwrap().who(&bearer(h)).ok_or_else(|| err(StatusCode::UNAUTHORIZED, "Sign in to Agape Admin")) }
fn person_in(o: &admin::Admin, u: &admin::User) -> Value { json!({ "mailbox": u.mailbox, "id": u.id, "email": u.email, "name": u.name, "role": u.role, "role_name": o.role_name(&u.role), "title": u.title, "boss": u.boss, "perms": o.perms_of(u), "oversees": o.oversees_anyone(u) }) }
fn done(r: Result<(), String>) -> Response { match r { Ok(()) => Json(json!({ "ok": true })).into_response(), Err(e) => err(StatusCode::BAD_REQUEST, &e) } }
fn signed(app: &App, r: Result<(String, admin::User), String>, code: StatusCode) -> Response {
    match r { Ok((t, u)) => { let me = person_in(&app.office.lock().unwrap(), &u); Json(json!({ "ok": true, "token": t, "me": me })).into_response() } Err(e) => err(code, &e) }
}
fn s(b: &Value, k: &str) -> String { b[k].as_str().unwrap_or("").to_string() }
fn strs(b: &Value, k: &str) -> Vec<String> { b[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default() }

async fn office_state(State(app): State<S>) -> Response { Json(json!({ "ok": true, "setup": app.office.lock().unwrap().setup_needed() })).into_response() }
async fn office_setup(State(app): State<S>, Json(b): Json<Value>) -> Response {
    let r = app.office.lock().unwrap().setup(&s(&b, "code"), &s(&b, "email"), &s(&b, "name"), &s(&b, "password"));
    signed(&app, r, StatusCode::BAD_REQUEST)
}
async fn office_signin(State(app): State<S>, Json(b): Json<Value>) -> Response {
    let r = app.office.lock().unwrap().sign_in(&s(&b, "email"), &s(&b, "password"));
    signed(&app, r, StatusCode::UNAUTHORIZED)
}
async fn office_signout(State(app): State<S>, h: HeaderMap) -> Response { app.office.lock().unwrap().sign_out(&bearer(&h)); Json(json!({ "ok": true })).into_response() }
async fn office_me(State(app): State<S>, h: HeaderMap) -> Response { match staffer(&app, &h) { Ok(u) => { let o = app.office.lock().unwrap(); let (mine, pings) = o.unread(&u); Json(json!({ "ok": true, "me": person_in(&o, &u), "unread": mine, "pings": pings, "domain": app.mail.domain })).into_response() } Err(r) => r } }
async fn office_password(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    done(app.office.lock().unwrap().change_password(&u, &s(&b, "old"), &s(&b, "new")))
}
async fn office_team(State(app): State<S>, h: HeaderMap) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    let mut v = app.office.lock().unwrap().team(&u);
    v["ok"] = json!(true);
    Json(v).into_response()
}
async fn office_invite(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    match app.office.lock().unwrap().invite(&u, &s(&b, "email"), &s(&b, "name"), &s(&b, "role"), &s(&b, "boss"), &s(&b, "title")) { Ok(code) => Json(json!({ "ok": true, "code": code })).into_response(), Err(e) => err(StatusCode::BAD_REQUEST, &e) }
}
async fn office_invite_cancel(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    done(app.office.lock().unwrap().cancel_invite(&u, &s(&b, "email")))
}
async fn office_invitation(State(app): State<S>, Path(code): Path<String>) -> Response {
    match app.office.lock().unwrap().invite_info(&code) { Some(i) => Json(json!({ "ok": true, "invite": i })).into_response(), None => err(StatusCode::NOT_FOUND, "This invitation has expired or was already used. Ask for a new one.") }
}
async fn office_accept(State(app): State<S>, Json(b): Json<Value>) -> Response {
    let r = app.office.lock().unwrap().accept(&s(&b, "code"), &s(&b, "name"), &s(&b, "password"));
    signed(&app, r, StatusCode::BAD_REQUEST)
}
async fn office_user(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    done(app.office.lock().unwrap().set_user(&u, &s(&b, "id"), b["role"].as_str(), b["off"].as_bool(), b["boss"].as_str(), b["title"].as_str()))
}
async fn office_merchants(State(app): State<S>, h: HeaderMap) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    if !app.office.lock().unwrap().can(&u, "merchants_view") { return err(StatusCode::FORBIDDEN, "Your role doesn't include merchants"); }
    let venues: Vec<Value> = app.venues.lock().unwrap().values().map(|v| json!({ "id": v.id, "name": v.name })).collect();
    Json(json!({ "ok": true, "merchants": app.office.lock().unwrap().merchants(), "venues": venues })).into_response()
}
async fn office_merchant(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    let Some(f) = b["fields"].as_object() else { return err(StatusCode::BAD_REQUEST, "Nothing to save") };
    match app.office.lock().unwrap().save_merchant(&u, b["id"].as_str(), f, &s(&b, "status")) { Ok(m) => Json(json!({ "ok": true, "merchant": m })).into_response(), Err(e) => err(StatusCode::BAD_REQUEST, &e) }
}
async fn office_merchant_delete(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    done(app.office.lock().unwrap().delete_merchant(&u, &s(&b, "id")))
}
async fn office_printed(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    app.office.lock().unwrap().printed(&u, &s(&b, "id"));
    Json(json!({ "ok": true })).into_response()
}
async fn office_leads(State(app): State<S>, h: HeaderMap) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    if !app.office.lock().unwrap().can(&u, "leads") { return err(StatusCode::FORBIDDEN, "Your role doesn't include leads"); }
    let mut all: Vec<Value> = std::fs::read_to_string(app.data.join("leads.jsonl")).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    all.reverse();
    Json(json!({ "ok": true, "leads": all })).into_response()
}
async fn office_activity(State(app): State<S>, h: HeaderMap) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    Json(json!({ "ok": true, "log": app.office.lock().unwrap().log(&u) })).into_response()
}
async fn office_inbox(State(app): State<S>, h: HeaderMap) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    let mut v = app.office.lock().unwrap().inbox(&u);
    v["ok"] = json!(true);
    Json(v).into_response()
}
async fn office_send(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    let to = strs(&b, "to");
    done(app.office.lock().unwrap().send(&u, &to, &s(&b, "subject"), &s(&b, "body")))
}
async fn office_read(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    let ids: Vec<String> = b["ids"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    app.office.lock().unwrap().mark_read(&u, &ids);
    Json(json!({ "ok": true })).into_response()
}

async fn office_role(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    match app.office.lock().unwrap().save_role(&u, b["id"].as_str(), &s(&b, "name"), &s(&b, "about"), &strs(&b, "perms"), &s(&b, "template")) { Ok(r) => Json(json!({ "ok": true, "role": r })).into_response(), Err(e) => err(StatusCode::BAD_REQUEST, &e) }
}
async fn office_role_delete(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    done(app.office.lock().unwrap().delete_role(&u, &s(&b, "id")))
}
async fn office_template(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    done(app.office.lock().unwrap().save_template(&u, b["id"].as_str(), &s(&b, "name"), &s(&b, "about"), &strs(&b, "perms")))
}
async fn office_template_delete(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    done(app.office.lock().unwrap().delete_template(&u, &s(&b, "id")))
}
async fn office_group(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    done(app.office.lock().unwrap().save_group(&u, b["id"].as_str(), &s(&b, "name"), &s(&b, "about"), &strs(&b, "members"), b["boss"].as_str()))
}
async fn office_group_delete(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    done(app.office.lock().unwrap().delete_group(&u, &s(&b, "id")))
}

/// Everyone with an app account (fans, runners, stands, businesses, players), with where their points are.
async fn office_accounts(State(app): State<S>, h: HeaderMap) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    if !app.office.lock().unwrap().can(&u, "accounts") { return err(StatusCode::FORBIDDEN, "Your role doesn't include app accounts"); }
    let all = app.auth.lock().unwrap().all();
    let ps = app.passports.lock().await;
    let venues: BTreeMap<String, String> = app.venues.lock().unwrap().values().map(|v| (v.id.clone(), v.name.clone())).collect();
    let list: Vec<Value> = all.iter().map(|a| {
        let p = ps.all.get(&a.id);
        json!({ "id": a.id, "handle": a.handle, "role": a.role, "party": a.party, "venue": a.venue, "venue_name": a.venue.as_ref().and_then(|v| venues.get(v)),
                "points": p.map(|p| p.points), "level": p.map(|p| p.level), "nights": p.map(|p| p.nights), "visited": p.map(|p| p.venues.iter().map(|v| venues.get(v).cloned().unwrap_or(v.clone())).collect::<Vec<_>>()),
                "at": p.and_then(|p| p.at.as_ref()).map(|v| venues.get(v).cloned().unwrap_or(v.clone())) })
    }).collect();
    Json(json!({ "ok": true, "accounts": list })).into_response()
}

// ---------------- email ----------------

/// Send an email from someone's own address. People at Agape get it right away; everyone else gets
/// it straight from Agape's server. Either way it's kept, under the same see-down rule.
async fn office_email(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    let (subject, body) = (s(&b, "subject").trim().chars().take(200).collect::<String>(), s(&b, "body").trim().chars().take(50_000).collect::<String>());
    if body.is_empty() { return err(StatusCode::BAD_REQUEST, "Write something first"); }
    let subject = if subject.is_empty() { "(no subject)".to_string() } else { subject };
    let (inside, outside, mailbox) = {
        let o = app.office.lock().unwrap();
        match o.recipients(&strs(&b, "to"), &app.mail.domain) { Ok((i, x)) => (i, x, u.mailbox.clone()), Err(e) => return err(StatusCode::BAD_REQUEST, &e) }
    };
    if mailbox.is_empty() { return err(StatusCode::BAD_REQUEST, "You don't have an address yet. Choose one under Your account."); }
    let (mut status, mut retry) = (vec![], vec![]);
    let mut failed = 0;
    if !outside.is_empty() {
        for (to, r) in app.mail.send(&u.name, &mailbox, &outside, &subject, &body, None).await {
            match r {
                Ok(m) => status.push(format!("{to}: delivered ({m})")),
                Err(e) if mail::permanent(&e) => { failed += 1; status.push(format!("{to}: not delivered. {e}")) }
                Err(e) => { status.push(format!("{to}: not delivered yet; Agape will keep trying for two days. {e}")); retry.push((to, 0u32, admin::now() + 600)); }
            }
        }
    }
    app.office.lock().unwrap().record_sent(&u, inside, outside, &subject, &body, status.clone(), retry);
    Json(json!({ "ok": true, "failed": failed, "status": status })).into_response()
}

async fn office_mailbox(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    let id = { let i = s(&b, "id"); if i.is_empty() { u.id.clone() } else { i } };
    done(app.office.lock().unwrap().set_mailbox(&u, &id, &s(&b, "mailbox")))
}

/// For the host: the DNS records to add, the Worker to paste into Cloudflare, and a live check.
async fn office_mail_setup(State(app): State<S>, h: HeaderMap) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    if u.role != admin::HOST { return err(StatusCode::FORBIDDEN, "Only the host sets up email"); }
    let m = &app.mail;
    let check = m.diagnose().await;
    let ip = check["mail_host"]["ips"][0].as_str().unwrap_or("").to_string();
    Json(json!({ "ok": true, "domain": m.domain, "host": m.host, "check": check,
        "records": [
            { "type": "A", "name": "mail", "value": if ip.is_empty() { "207.231.111.60".to_string() } else { ip.clone() }, "note": "Proxy status: DNS only (grey cloud). This is the name Agape's server uses when it sends." },
            { "type": "TXT", "name": "@", "value": format!("v=spf1 a:{} include:_spf.mx.cloudflare.net ~all", m.host), "note": "Replaces the SPF record that's there now. Says Agape's server may send for the domain." },
            { "type": "TXT", "name": format!("{}._domainkey", mail::SELECTOR), "value": m.dkim_record(), "note": "Agape's public signing key. Receivers check every message against it." },
            { "type": "TXT", "name": "_dmarc", "value": "v=DMARC1; p=none".to_string(), "note": "Tells receivers what to do with mail that fails the checks. Start with p=none." },
        ],
        "worker": mail::worker_code(&m.secret, &format!("https://{}/api/mail/inbound", m.domain)),
    })).into_response()
}

async fn office_mail_test(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    if u.role != admin::HOST { return err(StatusCode::FORBIDDEN, "Only the host sets up email"); }
    let to = s(&b, "to");
    let r = app.mail.send("Agape", if u.mailbox.is_empty() { "team" } else { &u.mailbox }, &[to.clone()], "Agape mail test", "This is a test from Agape's own mail server.\n\nIf it landed in your inbox (not spam), sending works without a mail provider.", None).await;
    let (ok, detail) = match &r[0].1 { Ok(m) => (true, m.clone()), Err(e) => (false, e.clone()) };
    Json(json!({ "ok": true, "delivered": ok, "detail": detail })).into_response()
}

/// Cloudflare's Worker hands each incoming message here.
async fn mail_inbound(State(app): State<S>, h: HeaderMap, body: axum::body::Bytes) -> Response {
    let given = h.get("x-agape-mail-secret").and_then(|v| v.to_str().ok()).unwrap_or("");
    // compare in constant time so the secret can't be guessed a character at a time
    let ok = given.len() == app.mail.secret.len() && given.bytes().zip(app.mail.secret.bytes()).fold(0u8, |a, (x, y)| a | (x ^ y)) == 0;
    if !ok { return err(StatusCode::FORBIDDEN, "Not from Agape's mail worker"); }
    let hv = |k: &str| h.get(k).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let m = mail::parse(&body, &hv("x-envelope-to"), &hv("x-envelope-from"));
    let (n, note) = app.office.lock().unwrap().receive(&m.from, &m.from_name, &m.to, &m.subject, &m.body, m.attachments, &app.mail.domain);
    // attachments are kept on disk beside the mail, readable only by people who can see the message
    if !m.files.is_empty() {
        let dir = app.data.join("mail-files").join(&note);
        let _ = std::fs::create_dir_all(&dir);
        for (i, (name, ctype, bytes)) in m.files.iter().enumerate() {
            let _ = std::fs::write(dir.join(format!("{i}.bin")), bytes);
            let _ = std::fs::write(dir.join(format!("{i}.json")), json!({ "name": name, "type": ctype, "size": bytes.len() }).to_string());
        }
    }
    Json(json!({ "ok": true, "kept_for": n })).into_response()
}

/// Whether the server can send mail, and which public DNS records are in place. Nothing private:
/// all of it can be read from public DNS by anyone.
async fn mail_health(State(app): State<S>) -> Response { Json(app.mail.diagnose().await).into_response() }

async fn office_mute(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    app.office.lock().unwrap().mute(&u, &s(&b, "id"), b["on"].as_bool().unwrap_or(false));
    Json(json!({ "ok": true })).into_response()
}

/// Try again any email a receiving server couldn't take yet. Runs every couple of minutes.
pub async fn retry_mail(app: &App) {
    let due = app.office.lock().unwrap().due_retries();
    for (note, name, mailbox, to, subject, body, _) in due {
        let r = app.mail.send(&name, &mailbox, &[to.clone()], &subject, &body, None).await.pop().map(|x| x.1).unwrap_or(Err("Nothing happened".into()));
        let perm = r.as_ref().err().map_or(false, |e| mail::permanent(e));
        app.office.lock().unwrap().retried(&note, &to, r, perm);
    }
}

async fn office_user_delete(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    match app.office.lock().unwrap().delete_user(&u, &s(&b, "id")) { Ok(name) => Json(json!({ "ok": true, "name": name })).into_response(), Err(e) => err(StatusCode::BAD_REQUEST, &e) }
}

/// A browser tells us a request to Agape Admin failed on its side (it never got an answer).
/// Kept so the host can see when and where people hit trouble. Never holds passwords.
async fn office_trouble(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let clip = |k: &str, n: usize| b[k].as_str().unwrap_or("").chars().take(n).collect::<String>();
    let path = app.data.join("admin-trouble.jsonl");
    if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > 2_000_000 { return Json(json!({ "ok": true })).into_response(); }
    let ua = h.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("").chars().take(200).collect::<String>();
    let rec = json!({ "at": admin::now(), "path": clip("path", 60), "what": clip("what", 200), "tries": b["tries"].as_u64().unwrap_or(0), "page": clip("page", 30), "ua": ua });
    let mut line = rec.to_string(); line.push('\n');
    use std::io::Write;
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f| f.write_all(line.as_bytes()));
    Json(json!({ "ok": true })).into_response()
}
async fn office_trouble_list(State(app): State<S>, h: HeaderMap) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    if u.role != admin::HOST { return err(StatusCode::FORBIDDEN, "Only the host sees this"); }
    let mut all: Vec<Value> = std::fs::read_to_string(app.data.join("admin-trouble.jsonl")).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    all.reverse(); all.truncate(200);
    Json(json!({ "ok": true, "trouble": all })).into_response()
}

/// One attachment from a received email, for someone allowed to see that email. Always offered as a
/// download, never shown inline, so a file can't run as a page on Agape's domain.
async fn office_attachment(State(app): State<S>, h: HeaderMap, Path((note, index)): Path<(String, usize)>) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    if !note.chars().all(|c| c.is_ascii_alphanumeric()) || !app.office.lock().unwrap().may_see(&u, &note) { return err(StatusCode::NOT_FOUND, "No such attachment"); }
    let dir = app.data.join("mail-files").join(&note);
    let (Ok(meta), Ok(bytes)) = (std::fs::read_to_string(dir.join(format!("{index}.json"))), std::fs::read(dir.join(format!("{index}.bin")))) else { return err(StatusCode::NOT_FOUND, "No such attachment") };
    let meta: Value = serde_json::from_str(&meta).unwrap_or(json!({}));
    let name: String = meta["name"].as_str().unwrap_or("attachment").chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') { c } else { '_' }).collect();
    (StatusCode::OK, [("content-type", "application/octet-stream".to_string()), ("content-disposition", format!("attachment; filename=\"{name}\"")), ("x-content-type-options", "nosniff".to_string())], bytes).into_response()
}


// ---------------- real venues and their nights (Agape Admin → Venues) ----------------

fn venue_lead(app: &App, h: &HeaderMap) -> Result<admin::User, Response> {
    let u = staffer(app, h)?;
    if !app.office.lock().unwrap().can(&u, "venues") { return Err(err(StatusCode::FORBIDDEN, "Your role doesn't include venues")); }
    Ok(u)
}

fn nights_dir(app: &App) -> std::path::PathBuf { let d = app.data.join("nights"); let _ = std::fs::create_dir_all(&d); d }

/// A finished real night: its full report is kept, its journal is kept under a new name, and the
/// venue waits for its next night to be opened from Admin.
pub async fn archive_real(app: &App, v: &venue::Handle) -> Option<String> {
    let night = v.view("agape", json!({ "lens": "night" })).await;
    let staff: Vec<Value> = app.auth.lock().unwrap().at_venue(&v.id).iter().map(|a| json!({ "handle": a.handle, "role": a.role, "party": a.party })).collect();
    let closed = now_s();
    let file = format!("{}-{}.json", v.id, closed);
    let rep = json!({ "venue": v.id, "name": v.name, "mode": v.init.mode, "opened": v.init.opened, "closed": closed, "staff": staff, "night": night });
    let path = nights_dir(app).join(&file);
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, serde_json::to_vec_pretty(&rep).unwrap_or_default()).and_then(|_| std::fs::rename(&tmp, &path)).is_err() {
        eprintln!("{}: couldn't write the night's report; keeping the night open", v.id);
        return None;
    }
    v.stop();
    let dir = app.data.join("venues");
    let _ = std::fs::rename(dir.join(format!("{}.journal", v.id)), dir.join(format!("{}.{closed}.night", v.id)));
    app.venues.lock().unwrap().remove(&v.id);
    let b = &rep["night"]["books"];
    let money = |k: &str| format!("${:.2}", b[k].as_i64().unwrap_or(0) as f64 / 100.0);
    let n = b["totals"]["n_settled"].as_i64().unwrap_or(0);
    let what = if v.init.mode == "rehearsal" { "Rehearsal" } else { "Night" };
    app.office.lock().unwrap().notify_perm("venues", "venue", &format!("{what} closed: {}", v.name),
        &format!("{} closed. {n} orders delivered, {} paid by fans, {} to the stands, {} to runners. The books {}.\n\nThe full report is in Admin → Venues.",
            v.name, money("fans_paid"), money("stands"), money("runners_pay"), if b["conserved"] == true { "balance to the cent" } else { "DO NOT balance: look at the report" }));
    println!("{}: real night closed, report {file}", v.id);
    Some(file)
}

fn reports(app: &App) -> Vec<Value> {
    let mut v: Vec<Value> = std::fs::read_dir(nights_dir(app)).map(|rd| rd.flatten().filter_map(|e| {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.ends_with(".json") { return None; }
        let r: Value = serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok()?;
        let b = &r["night"]["books"];
        Some(json!({ "file": name, "venue": r["venue"], "name": r["name"], "mode": r["mode"], "opened": r["opened"], "closed": r["closed"],
            "orders": b["totals"]["n_settled"], "fans_paid": b["fans_paid"], "conserved": b["conserved"] }))
    }).collect()).unwrap_or_default();
    v.sort_by_key(|r| std::cmp::Reverse(r["closed"].as_u64().unwrap_or(0)));
    v
}

async fn office_venues(State(app): State<S>, h: HeaderMap) -> Response {
    if let Err(r) = venue_lead(&app, &h) { return r; }
    let drafts = app.office.lock().unwrap().venue_drafts();
    let mut out = vec![];
    for d in drafts {
        let open = app.venue(&d.id);
        let night = match &open { Some(v) => { let n = v.view("agape", json!({ "lens": "night" })).await; Some(json!({ "mode": v.init.mode, "opened": v.init.opened, "game": n["game"], "clock": n["clock"], "halt": n["halt"],
            "orders": n["orders"].as_array().map(|a| a.len()).unwrap_or(0), "people": n["people"], "stands_open": n["stands"].as_array().map(|a| a.iter().filter(|s| s["open"] == true).count()).unwrap_or(0),
            "runners_on": n["runners"].as_array().map(|a| a.iter().filter(|s| s["on_shift"] == true).count()).unwrap_or(0) })) } None => None };
        let staff: Vec<Value> = app.auth.lock().unwrap().at_venue(&d.id).iter().map(|a| json!({ "id": a.id, "handle": a.handle, "role": a.role, "party": a.party })).collect();
        out.push(json!({ "id": d.id, "form": d.form, "at": d.at, "night": night, "staff": staff }));
    }
    Json(json!({ "ok": true, "venues": out, "reports": reports(&app), "starter": pilot::starter_form(), "optional": pilot::OPTIONAL })).into_response()
}

async fn office_venue_check(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    if let Err(r) = venue_lead(&app, &h) { return r; }
    let (_, issues) = pilot::check(&b["form"]);
    Json(json!({ "ok": true, "issues": issues })).into_response()
}

async fn office_venue_save(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match venue_lead(&app, &h) { Ok(u) => u, Err(r) => return r };
    let (_, issues) = pilot::check(&b["form"]);
    // saved even with problems (a form is filled in over days); opening is what needs it right
    let r = app.office.lock().unwrap().save_venue(&u, b["form"].clone());
    match r { Ok(()) => Json(json!({ "ok": true, "issues": issues })).into_response(), Err(e) => err(StatusCode::BAD_REQUEST, &e) }
}

async fn office_venue_delete(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match venue_lead(&app, &h) { Ok(u) => u, Err(r) => return r };
    let id = s(&b, "id");
    if app.venue(&id).is_some() { return err(StatusCode::BAD_REQUEST, "A night is open there: end it first"); }
    done(app.office.lock().unwrap().delete_venue(&u, &id))
}

async fn office_venue_open(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match venue_lead(&app, &h) { Ok(u) => u, Err(r) => return r };
    let (id, mode) = (s(&b, "id"), s(&b, "mode"));
    if mode != "live" && mode != "rehearsal" { return err(StatusCode::BAD_REQUEST, "Open it live or as a rehearsal"); }
    let Some(d) = app.office.lock().unwrap().venue_draft(&id) else { return err(StatusCode::NOT_FOUND, "No such venue") };
    if app.venue(&id).is_some() { return err(StatusCode::BAD_REQUEST, "A night is already open there"); }
    let (world, issues) = pilot::check(&d.form);
    if issues.iter().any(|i| i.blocking) { return (StatusCode::BAD_REQUEST, Json(json!({ "ok": false, "error": "Fix what's flagged in the venue first", "issues": issues }))).into_response(); }
    let name = d.form["name"].as_str().unwrap_or(&id).to_string();
    let mut seed = [0u8; 8];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut seed);
    let init = venue::Init { id: id.clone(), name: name.clone(), world, template: pilot::plain_template(), seed: u64::from_le_bytes(seed) >> 1,
        fans: 0, runners: d.form["runners"].as_u64().unwrap_or(6) as usize, start_min: 0, build: venue::BUILD.to_string(), mode: mode.clone(), opened: now_s() };
    // a journal left from an earlier night that never closed cleanly is kept, not replayed into this one
    let dir = app.data.join("venues");
    let jp = dir.join(format!("{id}.journal"));
    if jp.exists() { let _ = std::fs::rename(&jp, dir.join(format!("{id}.{}.orphan", now_s()))); }
    match venue::spawn(&dir, Some(init), &id, app.speed) {
        Ok(hd) => {
            app.venues.lock().unwrap().insert(id.clone(), hd);
            app.office.lock().unwrap().logged(&u, format!("opened {} at {name}", if mode == "live" { "a live night" } else { "a rehearsal" }));
            Json(json!({ "ok": true })).into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

/// Staff accounts for a venue: a runner takes the next free runner slot, a stand account works that
/// stand, an ops account runs the venue's console. The password is shown once, to hand over.
async fn office_venue_staff(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match venue_lead(&app, &h) { Ok(u) => u, Err(r) => return r };
    let (vid, role, handle) = (s(&b, "venue"), s(&b, "role"), s(&b, "handle"));
    let Some(d) = app.office.lock().unwrap().venue_draft(&vid) else { return err(StatusCode::NOT_FOUND, "No such venue") };
    let mut a = app.auth.lock().unwrap();
    let here = a.at_venue(&vid);
    let (r, party) = match role.as_str() {
        "runner" => {
            let slots = d.form["runners"].as_u64().unwrap_or(6) as usize;
            match (0..slots).map(|i| format!("runner{i}")).find(|p| !here.iter().any(|x| &x.party == p)) {
                Some(p) => (Role::Runner, p),
                None => return err(StatusCode::BAD_REQUEST, &format!("All {slots} runner places have accounts: plan for more runners in the venue first")),
            }
        }
        "stand" => {
            let st = s(&b, "stand");
            if !d.form["stands"].as_array().map_or(false, |a| a.iter().any(|x| x["id"] == st.as_str())) { return err(StatusCode::BAD_REQUEST, "Pick one of the venue's stands"); }
            (Role::Stand, st)
        }
        "ops" => (Role::Ops, "ops".to_string()),
        _ => return err(StatusCode::BAD_REQUEST, "A runner, a stand or ops"),
    };
    let pw = pilot::readable_password();
    match a.create(&handle, &pw, r, Some(party.clone()), Some(vid.clone())) {
        Ok(acc) => {
            drop(a);
            app.office.lock().unwrap().logged(&u, format!("gave {} a {role} account at {vid}", acc.handle));
            Json(json!({ "ok": true, "account": acc.public(), "password": pw })).into_response()
        }
        Err(e) => err(StatusCode::BAD_REQUEST, &e),
    }
}

async fn office_venue_staff_reset(State(app): State<S>, h: HeaderMap, Json(b): Json<Value>) -> Response {
    let u = match venue_lead(&app, &h) { Ok(u) => u, Err(r) => return r };
    let id = s(&b, "account");
    let pw = pilot::readable_password();
    let r = {
        let mut a = app.auth.lock().unwrap();
        // only staff accounts: a fan's own account is theirs
        match a.all().into_iter().find(|x| x.id == id) { Some(x) if x.role != Role::Fan && x.venue.is_some() => a.reset_password(&id, &pw), _ => Err("Only a venue's staff accounts can be reset here".into()) }
    };
    match r { Ok(acc) => { app.office.lock().unwrap().logged(&u, format!("reset the password of {}", acc.handle)); Json(json!({ "ok": true, "password": pw })).into_response() } Err(e) => err(StatusCode::BAD_REQUEST, &e) }
}

async fn office_night(State(app): State<S>, h: HeaderMap, Path(vid): Path<String>) -> Response {
    if let Err(r) = venue_lead(&app, &h) { return r; }
    let Some(v) = app.venue(&vid).filter(|v| v.init.real()) else { return err(StatusCode::NOT_FOUND, "No night is open there") };
    let mut n = v.view("agape", json!({ "lens": "night" })).await;
    let staff: BTreeMap<String, String> = app.auth.lock().unwrap().at_venue(&vid).into_iter().map(|a| (a.party, a.handle)).collect();
    n["staff"] = json!(staff);
    n["mode"] = json!(v.init.mode); n["opened"] = json!(v.init.opened); n["ok"] = json!(true);
    Json(n).into_response()
}

async fn office_night_act(State(app): State<S>, h: HeaderMap, Path(vid): Path<String>, Json(b): Json<Value>) -> Response {
    let u = match venue_lead(&app, &h) { Ok(u) => u, Err(r) => return r };
    let Some(v) = app.venue(&vid).filter(|v| v.init.real()) else { return err(StatusCode::NOT_FOUND, "No night is open there") };
    let what = s(&b, "do");
    if !matches!(what.as_str(), "game" | "score" | "halt" | "resume" | "resolve") { return err(StatusCode::BAD_REQUEST, "Not something the console does"); }
    let r = v.act("ops", b.clone()).await;
    if r["ok"] == true && what != "score" {
        let detail = match what.as_str() { "game" => format!("marked {} at {}", s(&b, "what"), v.name), "halt" => format!("paused ordering at {}: {}", v.name, s(&b, "reason")), "resume" => format!("resumed ordering at {}", v.name), _ => format!("settled a problem at {}", v.name) };
        app.office.lock().unwrap().logged(&u, detail);
    }
    Json(r).into_response()
}

/// End a night now (the event is over and nothing is still on its way).
async fn office_night_end(State(app): State<S>, h: HeaderMap, Path(vid): Path<String>) -> Response {
    let u = match venue_lead(&app, &h) { Ok(u) => u, Err(r) => return r };
    let Some(v) = app.venue(&vid).filter(|v| v.init.real()) else { return err(StatusCode::NOT_FOUND, "No night is open there") };
    let n = v.view("agape", json!({ "lens": "night" })).await;
    let open = n["books"]["totals"]["n_open"].as_i64().unwrap_or(0);
    if open > 0 { return err(StatusCode::BAD_REQUEST, &format!("{open} order{} still on {}: finish or cancel {} first", if open == 1 { " is" } else { "s are" }, if open == 1 { "its way" } else { "their way" }, if open == 1 { "it" } else { "them" })); }
    if n["game"]["final"] != true { let _ = v.act("ops", json!({ "do": "game", "what": "final" })).await; }
    // everyone's points go home to their passport first, as when any night ends
    {
        let mut ps = app.passports.lock().await;
        let here: Vec<String> = ps.all.iter().filter(|(_, p)| p.at.as_deref() == Some(&v.id) && p.pending.is_none()).map(|(k, _)| k.clone()).collect();
        for id in here {
            let party = app.auth.lock().unwrap().party_of(&id).unwrap_or_default();
            let mut p = ps.all[&id].clone();
            p.seq += 1;
            p.pending = Some((v.id.clone(), format!("{}-{}-out", id, p.seq), "out".into()));
            ps.all.insert(id.clone(), p.clone()); ps.save();
            finish_move(&app, &party, &mut p).await;
            ps.all.insert(id, p); ps.save();
        }
    }
    match archive_real(&app, &v).await {
        Some(file) => { app.office.lock().unwrap().logged(&u, format!("ended the night at {}", v.name)); Json(json!({ "ok": true, "report": file })).into_response() }
        None => err(StatusCode::INTERNAL_SERVER_ERROR, "Couldn't write the night's report; the night is still open"),
    }
}

async fn office_report(State(app): State<S>, h: HeaderMap, Path(file): Path<String>) -> Response {
    if let Err(r) = venue_lead(&app, &h) { return r; }
    if file.contains('/') || file.contains("..") || !file.ends_with(".json") { return err(StatusCode::NOT_FOUND, "No such report"); }
    match std::fs::read(nights_dir(&app).join(&file)) { Ok(b) => ([("content-type", "application/json")], b).into_response(), Err(_) => err(StatusCode::NOT_FOUND, "No such report") }
}


// ---------------- staying up ----------------

/// Is Agape working: every venue answers, there's room on the disk. 503 if not, so an outside
/// check (GitHub's, every few minutes) emails the owner.
async fn health(State(app): State<S>) -> Response {
    let (ok, detail) = check_health(&app).await;
    (if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE }, Json(json!({ "ok": ok, "detail": detail }))).into_response()
}

pub async fn check_health(app: &App) -> (bool, Value) {
    let venues: Vec<venue::Handle> = app.venues.lock().unwrap().values().cloned().collect();
    let mut vs = serde_json::Map::new();
    let mut ok = true;
    for v in venues {
        let r = tokio::time::timeout(std::time::Duration::from_secs(5), v.view("", json!({ "lens": "glance" }))).await;
        let fine = matches!(&r, Ok(x) if x.get("error").is_none());
        if !fine { ok = false; }
        vs.insert(v.id.clone(), json!(if fine { "ok" } else { "not answering" }));
    }
    let free = ops::disk_free_mb(&app.data);
    if free.map_or(false, |f| f < 1024) { ok = false; }
    (ok, json!({ "venues": vs, "disk_free_mb": free }))
}

/// Every minute: if something is wrong, the people who run venues hear about it in Admin, once
/// per problem, and again when it's fixed.
pub async fn watch(app: &App, was: &mut Option<String>) {
    let (ok, d) = check_health(app).await;
    let now = if ok { None } else {
        let mut why = vec![];
        if let Some(m) = d["venues"].as_object() { for (k, v) in m { if v != "ok" { why.push(format!("{k} isn't answering")); } } }
        if d["disk_free_mb"].as_u64().map_or(false, |f| f < 1024) { why.push(format!("only {} MB of disk left", d["disk_free_mb"])); }
        Some(why.join("; "))
    };
    if now != *was {
        let mut o = app.office.lock().unwrap();
        match &now { Some(w) => o.notify_perm("venues", "alert", "Agape needs attention", &format!("Something is wrong on the server: {w}.\n\nThis was noticed by Agape's own check, which runs every minute.")),
                     None => o.notify_perm("venues", "alert", "Agape is fine again", "The problem the server's check found has cleared.") }
        *was = now;
    }
}

/// The host downloads everything as one file, to keep somewhere other than this server.
async fn office_backup(State(app): State<S>, h: HeaderMap) -> Response {
    let u = match staffer(&app, &h) { Ok(u) => u, Err(r) => return r };
    if u.role != admin::HOST { return err(StatusCode::FORBIDDEN, "Only the host downloads backups"); }
    app.office.lock().unwrap().logged(&u, "downloaded a full backup".into());
    let b = ops::bundle(&app.data);
    ([("content-type", "application/json"), ("content-disposition", "attachment; filename=\"agape-backup.json\"")], serde_json::to_vec(&b).unwrap_or_default()).into_response()
}

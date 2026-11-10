//! A real venue, set up in Agape Admin, staffed, and run for a night with real people, then closed
//! with a report whose books balance. Nothing in it is simulated.

use agape_server::{app, router, S};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};

struct Srv { base: String, app: S, c: Client }

async fn start(dir: &std::path::Path) -> Srv {
    let app = app(dir, 0.0, None);
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let r = router(app.clone(), None);
    tokio::spawn(async move { axum::serve(l, r).await.unwrap() });
    Srv { base, app, c: Client::new() }
}

impl Srv {
    async fn post(&self, path: &str, token: Option<&str>, body: Value) -> (StatusCode, Value) {
        let mut r = self.c.post(format!("{}{path}", self.base)).json(&body);
        if let Some(t) = token { r = r.bearer_auth(t); }
        let res = r.send().await.unwrap();
        let st = res.status();
        (st, res.json().await.unwrap_or(Value::Null))
    }
    async fn get(&self, path: &str, token: Option<&str>) -> (StatusCode, Value) {
        let mut r = self.c.get(format!("{}{path}", self.base));
        if let Some(t) = token { r = r.bearer_auth(t); }
        let res = r.send().await.unwrap();
        let st = res.status();
        (st, res.json().await.unwrap_or(Value::Null))
    }
    async fn signin(&self, h: &str, p: &str) -> String { let (_, v) = self.post("/api/signin", None, json!({ "handle": h, "password": p })).await; v["token"].as_str().unwrap_or("").to_string() }
    async fn act(&self, t: &str, body: Value) -> Value { self.post("/api/v/rupp/act", Some(t), body).await.1 }
    async fn view(&self, t: &str, lens: &str) -> Value { self.post("/api/v/rupp/view", Some(t), json!({ "lens": lens })).await.1 }
    async fn step(&self, n: u32) { self.app.venue("rupp").unwrap().step(n).await; }
}

async fn owner(s: &Srv, dir: &std::path::Path) -> String {
    let code = std::fs::read_to_string(dir.join("admin-setup-code")).unwrap();
    let (st, v) = s.post("/api/office/setup", None, json!({ "code": code.trim(), "email": "owner@venue.example", "name": "Owner", "password": "a long owner password" })).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    v["token"].as_str().unwrap().to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_venue_is_set_up_staffed_run_for_real_and_closed_with_balanced_books() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path()).await;
    let boss = owner(&s, dir.path()).await;

    // the venue, from the starter form
    let (_, list) = s.get("/api/office/venues", Some(&boss)).await;
    let mut form = list["starter"].clone();
    form["id"] = json!("rupp"); form["name"] = json!("Rupp Arena");
    form["home"] = json!({ "name": "Kentucky Wildcats", "short": "Wildcats", "abbr": "UK", "color": "#0033A0" });
    form["away"] = json!({ "name": "Louisville Cardinals", "short": "Cardinals", "abbr": "UL", "color": "#C9001F" });
    let mut broken = form.clone();
    broken["menu"][0]["stand"] = json!("nowhere");
    let (_, chk) = s.post("/api/office/venue/check", Some(&boss), json!({ "form": broken })).await;
    assert!(chk["issues"].as_array().unwrap().iter().any(|i| i["blocking"] == true), "a menu item at a stand that doesn't exist is flagged: {chk}");
    let (st, saved) = s.post("/api/office/venue", Some(&boss), json!({ "form": form })).await;
    assert_eq!(st, StatusCode::OK, "{saved}");
    assert!(!saved["issues"].as_array().unwrap().iter().any(|i| i["blocking"] == true), "{saved}");

    // staff accounts: passwords shown once
    let mut accounts = vec![];
    for (role, handle, extra) in [("stand", "rupp.grill", json!({ "stand": "grill" })), ("runner", "rupp.maria", json!({})), ("ops", "rupp.ops", json!({}))] {
        let mut b = json!({ "venue": "rupp", "role": role, "handle": handle }); if let Some(st) = extra["stand"].as_str() { b["stand"] = json!(st); }
        let (st, v) = s.post("/api/office/venue/staff", Some(&boss), b).await;
        assert_eq!(st, StatusCode::OK, "{v}");
        accounts.push((handle, v["password"].as_str().unwrap().to_string()));
    }
    assert_eq!(s.post("/api/office/venue/staff", Some(&boss), json!({ "venue": "rupp", "role": "stand", "handle": "rupp.x", "stand": "nope" })).await.0, StatusCode::BAD_REQUEST);

    // a rehearsal night opens: nothing simulated
    let (st, v) = s.post("/api/office/venue/open", Some(&boss), json!({ "id": "rupp", "mode": "rehearsal" })).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(s.post("/api/office/venue/open", Some(&boss), json!({ "id": "rupp", "mode": "live" })).await.0, StatusCode::BAD_REQUEST, "one night at a time");
    let (_, g) = s.get("/api/v/rupp/glance", None).await;
    assert_eq!(g["mode"], "rehearsal");
    assert_eq!(g["provenance"]["simulated_fans"], 0);
    assert_eq!(g["sections_per_level"], 20);

    // a fan on their own phone
    let (_, up) = s.post("/api/signup", None, json!({ "handle": "gabe", "password": "correct horse" })).await;
    let fan = up["token"].as_str().unwrap().to_string();
    assert_eq!(s.act(&fan, json!({ "do": "join", "seat": "s105", "row": "F", "place": "7" })).await["ok"], true);
    let fv = s.view(&fan, "fan").await;
    assert_eq!(fv["venue_mode"], "rehearsal");
    assert_eq!(s.act(&fan, json!({ "do": "order", "thing": "hotdog", "qty": 2, "delivery": true, "tip": 200 })).await["error"], "StandClosed");

    // the grill opens on its own tablet
    let grill = s.signin("rupp.grill", &accounts[0].1).await;
    assert_eq!(s.act(&grill, json!({ "do": "clock_in" })).await["ok"], true);
    let o = s.act(&fan, json!({ "do": "order", "thing": "hotdog", "qty": 2, "delivery": true, "tip": 200 })).await;
    assert_eq!(o["ok"], true, "{o}");
    let id: u64 = o["id"].as_str().unwrap().parse().unwrap();
    s.step(30).await;
    assert_eq!(s.act(&grill, json!({ "do": "ready", "order": id })).await["ok"], true);

    // a runner on her phone
    let maria = s.signin("rupp.maria", &accounts[1].1).await;
    assert_eq!(s.act(&maria, json!({ "do": "clock_in" })).await["ok"], true);
    s.step(2).await;
    let rv = s.view(&maria, "runner").await;
    assert!(rv["trip"]["orders"].to_string().contains(&id.to_string()), "the trip is offered to her: {}", rv["trip"]);
    for a in ["accept_trip", "picked_up"] { assert_eq!(s.act(&maria, json!({ "do": a })).await["ok"], true, "{a}"); }
    let d = s.act(&maria, json!({ "do": "delivered", "order": id })).await;
    assert_eq!(d["ok"], true, "{d}");
    s.step(2).await;

    // the venue's ops account marks the game from its own phone; Admin sees it all
    let ops = s.signin("rupp.ops", &accounts[2].1).await;
    assert_eq!(s.act(&ops, json!({ "do": "game", "what": "start" })).await["ok"], true);
    assert_eq!(s.act(&fan, json!({ "do": "game", "what": "final" })).await["ok"], false, "a fan can't");
    let (_, sc) = s.post("/api/office/night/rupp/act", Some(&boss), json!({ "do": "score", "team": "home", "points": 3 })).await;
    assert_eq!(sc["ok"], true, "{sc}");
    let (_, night) = s.get("/api/office/night/rupp", Some(&boss)).await;
    assert_eq!(night["game"]["score"][0], 3);
    assert_eq!(night["staff"]["runner0"], "rupp.maria");
    assert_eq!(night["orders"][0]["stage"], "Settled");
    assert_eq!(night["books"]["conserved"], true);
    let paid = night["books"]["fans_paid"].as_i64().unwrap();
    assert!(paid >= 1400 + 200, "two hot dogs, the tip, fee and tax: {paid}");

    // the night ends from Admin, with a report
    let (st, end) = s.post("/api/office/night/rupp/end", Some(&boss), json!({})).await;
    assert_eq!(st, StatusCode::OK, "{end}");
    assert!(s.app.venue("rupp").is_none());
    let (_, list) = s.get("/api/office/venues", Some(&boss)).await;
    let r = &list["reports"][0];
    assert_eq!(r["venue"], "rupp"); assert_eq!(r["conserved"], true); assert_eq!(r["orders"], 1); assert_eq!(r["fans_paid"].as_i64().unwrap(), paid);
    let (st, full) = s.get(&format!("/api/office/report/{}", r["file"].as_str().unwrap()), Some(&boss)).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(full["night"]["books"]["conserved"], true);
    assert_eq!(s.get("/api/office/report/..%2Fadmin.json", Some(&boss)).await.0, StatusCode::NOT_FOUND);

    // staff can be given a new password; the old one stops working
    let maria_id = list["venues"][0]["staff"].as_array().unwrap().iter().find(|a| a["handle"] == "rupp.maria").unwrap()["id"].as_str().unwrap().to_string();
    let (_, np) = s.post("/api/office/venue/staff/reset", Some(&boss), json!({ "account": maria_id })).await;
    assert_eq!(s.signin("rupp.maria", &accounts[1].1).await, "");
    assert!(!s.signin("rupp.maria", np["password"].as_str().unwrap()).await.is_empty());

    // a restart keeps the report and doesn't reopen the night by itself
    s.app.shutdown();
    drop(s);
    let s2 = start(dir.path()).await;
    assert!(s2.app.venue("rupp").is_none());
    assert!(s2.app.venue("harbor").is_some(), "the demo is untouched");
    let (_, list) = s2.get("/api/office/venues", Some(&boss)).await;
    assert_eq!(list["reports"].as_array().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_live_night_survives_a_restart_mid_event() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path()).await;
    let boss = owner(&s, dir.path()).await;
    let (_, list) = s.get("/api/office/venues", Some(&boss)).await;
    let mut form = list["starter"].clone();
    form["id"] = json!("rupp"); form["name"] = json!("Rupp Arena");
    s.post("/api/office/venue", Some(&boss), json!({ "form": form })).await;
    let (_, g) = s.post("/api/office/venue/staff", Some(&boss), json!({ "venue": "rupp", "role": "stand", "handle": "rupp.bar", "stand": "bar" })).await;
    assert_eq!(s.post("/api/office/venue/open", Some(&boss), json!({ "id": "rupp", "mode": "live" })).await.0, StatusCode::OK);
    let bar = s.signin("rupp.bar", g["password"].as_str().unwrap()).await;
    s.act(&bar, json!({ "do": "clock_in" })).await;
    let (_, up) = s.post("/api/signup", None, json!({ "handle": "jeff", "password": "correct horse" })).await;
    let fan = up["token"].as_str().unwrap().to_string();
    s.act(&fan, json!({ "do": "join", "seat": "s110" })).await;
    assert_eq!(s.act(&fan, json!({ "do": "order", "thing": "water", "qty": 1, "delivery": false, "tip": 0 })).await["ok"], true);
    s.post("/api/office/night/rupp/act", Some(&boss), json!({ "do": "game", "what": "start" })).await;
    s.step(5).await;
    s.app.shutdown();
    drop(s);
    let s2 = start(dir.path()).await;
    let (_, night) = s2.get("/api/office/night/rupp", Some(&boss)).await;
    assert_eq!(night["mode"], "live");
    assert_eq!(night["orders"].as_array().unwrap().len(), 1, "the order is still there");
    assert_eq!(night["game"]["period"], 1, "and so is the game");
    assert_eq!(night["stands"].as_array().unwrap().iter().find(|x| x["id"] == "bar").unwrap()["open"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn guessing_a_password_is_slowed_down() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path()).await;
    s.post("/api/signup", None, json!({ "handle": "staffer", "password": "the right one" })).await;
    for _ in 0..10 { assert_eq!(s.post("/api/signin", None, json!({ "handle": "staffer", "password": "wrong guess" })).await.0, StatusCode::UNAUTHORIZED); }
    assert_eq!(s.post("/api/signin", None, json!({ "handle": "Staffer", "password": "the right one" })).await.0, StatusCode::TOO_MANY_REQUESTS, "even the right password waits");
    assert!(s.signin("someoneelse", "x").await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_backup_restores_onto_a_new_machine_mid_night() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path()).await;
    let boss = owner(&s, dir.path()).await;
    let (_, list) = s.get("/api/office/venues", Some(&boss)).await;
    let mut form = list["starter"].clone();
    form["id"] = json!("rupp"); form["name"] = json!("Rupp Arena");
    s.post("/api/office/venue", Some(&boss), json!({ "form": form })).await;
    let (_, g) = s.post("/api/office/venue/staff", Some(&boss), json!({ "venue": "rupp", "role": "stand", "handle": "rupp.bar", "stand": "bar" })).await;
    s.post("/api/office/venue/open", Some(&boss), json!({ "id": "rupp", "mode": "live" })).await;
    let bar = s.signin("rupp.bar", g["password"].as_str().unwrap()).await;
    s.act(&bar, json!({ "do": "clock_in" })).await;
    let (_, up) = s.post("/api/signup", None, json!({ "handle": "jeff", "password": "correct horse" })).await;
    let fan = up["token"].as_str().unwrap().to_string();
    s.act(&fan, json!({ "do": "join", "seat": "s110" })).await;
    assert_eq!(s.act(&fan, json!({ "do": "order", "thing": "water", "qty": 1, "delivery": false, "tip": 0 })).await["ok"], true);
    let (st, health) = s.get("/api/health", None).await;
    assert_eq!(st, StatusCode::OK, "{health}");
    assert_eq!(s.get("/api/office/backup", None).await.0, StatusCode::UNAUTHORIZED);
    let bundle = s.c.get(format!("{}/api/office/backup", s.base)).bearer_auth(&boss).send().await.unwrap().json::<Value>().await.unwrap();
    assert!(agape_server::ops::snapshot(dir.path()).unwrap().is_some(), "today's copy is made");
    assert!(agape_server::ops::snapshot(dir.path()).unwrap().is_none(), "once a day");
    s.app.shutdown();
    drop(s);
    // the server is gone; a new machine with an empty folder
    let fresh = tempfile::tempdir().unwrap();
    assert!(agape_server::ops::restore(&bundle, fresh.path()).unwrap() >= 4);
    assert!(agape_server::ops::restore(&bundle, fresh.path()).is_err(), "never over existing data");
    let s2 = start(fresh.path()).await;
    let (_, me) = s2.post("/api/office/signin", None, json!({ "email": "owner@venue.example", "password": "a long owner password" })).await;
    let boss2 = me["token"].as_str().unwrap().to_string();
    let (_, night) = s2.get("/api/office/night/rupp", Some(&boss2)).await;
    assert_eq!(night["orders"].as_array().unwrap().len(), 1, "the night carries on with its order: {night}");
    assert!(!s2.signin("jeff", "correct horse").await.is_empty(), "fans can still sign in");
}

#[tokio::test(flavor = "multi_thread")]
async fn cards_are_held_at_the_order_taken_when_final_and_match_the_books() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path()).await;
    let boss = owner(&s, dir.path()).await;
    let (_, list) = s.get("/api/office/venues", Some(&boss)).await;
    let mut form = list["starter"].clone();
    form["id"] = json!("rupp"); form["name"] = json!("Rupp Arena");
    s.post("/api/office/venue", Some(&boss), json!({ "form": form })).await;
    let (_, g) = s.post("/api/office/venue/staff", Some(&boss), json!({ "venue": "rupp", "role": "stand", "handle": "rupp.grill", "stand": "grill" })).await;
    s.post("/api/office/venue/open", Some(&boss), json!({ "id": "rupp", "mode": "rehearsal" })).await;
    let grill = s.signin("rupp.grill", g["password"].as_str().unwrap()).await;
    s.act(&grill, json!({ "do": "clock_in" })).await;
    let mut toks = vec![];
    for h in ["gabe", "declinejoe"] { let (_, up) = s.post("/api/signup", None, json!({ "handle": h, "password": "correct horse" })).await; toks.push(up["token"].as_str().unwrap().to_string()); }
    let (gabe, joe) = (toks[0].clone(), toks[1].clone());
    for t in [&gabe, &joe] { s.act(t, json!({ "do": "join", "seat": "s105" })).await; }
    // a declined card: the order closes and the stand never sees it
    let d = s.act(&joe, json!({ "do": "order", "thing": "nachos", "qty": 1, "delivery": false, "tip": 0 })).await;
    assert_eq!(d["ok"], false); assert!(d["error"].as_str().unwrap().contains("declined"), "{d}");
    // two pickups: one handed over, one the stand can't make
    let a: u64 = s.act(&gabe, json!({ "do": "order", "thing": "hotdog", "qty": 2, "delivery": false, "tip": 0 })).await["id"].as_str().unwrap().parse().unwrap();
    let b: u64 = s.act(&gabe, json!({ "do": "order", "thing": "nachos", "qty": 1, "delivery": false, "tip": 0 })).await["id"].as_str().unwrap().parse().unwrap();
    let (_, n) = s.get("/api/office/night/rupp", Some(&boss)).await;
    assert_eq!(n["cards"]["holds"], 2, "both held: {}", n["cards"]);
    assert_eq!(n["cards"]["captured"], 0, "nothing taken yet");
    assert_eq!(n["cards"]["declined"], 1);
    assert!(n["orders"].as_array().unwrap().iter().any(|o| o["stage"].as_str().unwrap().contains("PaymentFailed")));
    s.act(&grill, json!({ "do": "ready", "order": a })).await;
    s.act(&grill, json!({ "do": "handed_over", "order": a })).await;
    s.act(&grill, json!({ "do": "cannot_make", "order": b })).await;
    agape_server::settle_cards(&s.app).await;
    let (_, n) = s.get("/api/office/night/rupp", Some(&boss)).await;
    assert_eq!(n["cards"]["holds"], 0); assert_eq!(n["cards"]["voided"], 1);
    assert_eq!(n["cards"]["net"], n["books"]["fans_paid"], "the cards equal the books: {} vs {}", n["cards"], n["books"]["fans_paid"]);
    // a problem reported: Agape credit by default, so the card stays as it is and still equals the books
    assert_eq!(s.act(&gabe, json!({ "do": "report", "order": a, "reason": "quality", "note": "cold" })).await["ok"], true);
    agape_server::settle_cards(&s.app).await;
    let (_, n) = s.get("/api/office/night/rupp", Some(&boss)).await;
    assert_eq!(n["cards"]["net"], n["books"]["fans_paid"]);
    let (_, end) = s.post("/api/office/night/rupp/end", Some(&boss), json!({})).await;
    let (_, rep) = s.get(&format!("/api/office/report/{}", end["report"].as_str().unwrap()), Some(&boss)).await;
    assert_eq!(rep["night"]["cards"]["net"], rep["night"]["books"]["fans_paid"]);
}

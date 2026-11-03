//! Two people on two phones, staff with their own screens, an operator, and a restart.

use agape_server::{app, router, S};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};

struct Srv { base: String, app: S, c: Client }

async fn start(dir: &std::path::Path) -> Srv {
    let app = app(dir, 0.0, Some("op-secret".into()));
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
    async fn signup(&self, h: &str) -> String { let (_, v) = self.post("/api/signup", None, json!({ "handle": h, "password": "correct horse" })).await; v["token"].as_str().unwrap().to_string() }
    async fn signin(&self, h: &str, p: &str) -> String { let (_, v) = self.post("/api/signin", None, json!({ "handle": h, "password": p })).await; v["token"].as_str().unwrap_or("").to_string() }
    async fn view(&self, t: &str, lens: &str, id: &str) -> (StatusCode, Value) { self.post("/api/v/harbor/view", Some(t), json!({ "lens": lens, "id": id })).await }
    async fn act(&self, t: &str, body: Value) -> Value { self.post("/api/v/harbor/act", Some(t), body).await.1 }
    async fn staff(&self, h: &str, role: &str, party: &str) -> String {
        let (st, _) = self.post("/api/admin/accounts", None, json!({ "handle": h, "password": "staff pass 1", "role": role, "party": party, "venue": "harbor" })).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "no operator token, no staff account");
        let r = self.c.post(format!("{}/api/admin/accounts", self.base)).header("x-admin-token", "op-secret")
            .json(&json!({ "handle": h, "password": "staff pass 1", "role": role, "party": party, "venue": "harbor" })).send().await.unwrap();
        assert!(r.status().is_success());
        self.signin(h, "staff pass 1").await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn two_fans_on_two_phones_see_their_own_night_and_staff_see_their_work() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path()).await;
    assert_eq!(s.view("nope", "fan", "").await.0, StatusCode::UNAUTHORIZED);
    let ana = s.signup("ana").await;
    let ben = s.signup("ben").await;
    assert_eq!(s.post("/api/signup", None, json!({ "handle": "Ana", "password": "another one" })).await.1["error"], "That name is taken");
    assert_eq!(s.post("/api/signin", None, json!({ "handle": "ana", "password": "wrong password" })).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(s.act(&ana, json!({ "do": "join", "seat": "s104" })).await["ok"], true);
    assert_eq!(s.act(&ben, json!({ "do": "join", "seat": "s112" })).await["ok"], true);
    let o = s.act(&ana, json!({ "do": "order", "thing": "burger", "qty": 1, "delivery": true, "tip": 200 })).await;
    assert_eq!(o["ok"], true, "{o}");
    let a = s.view(&ana, "fan", "").await.1;
    let b = s.view(&ben, "fan", "").await.1;
    assert_eq!(a["you"]["seat"], "s104");
    assert_eq!(a["orders"].as_array().unwrap().len(), 1, "Ana sees her order");
    assert_eq!(b["orders"].as_array().unwrap().len(), 0, "Ben doesn't");
    assert_eq!(b["you"]["seat"], "s112");
    // Ben can't cancel Ana's order, and can't act as anyone else by naming them
    let theirs = a["orders"][0]["id"].clone();
    assert_eq!(s.act(&ben, json!({ "do": "cancel", "order": theirs, "as": a["you"]["id"] })).await["error"], "not yours");
    assert_eq!(s.post("/api/v/harbor/act", Some(&ben), json!({ "do": "redeem", "place": "dock-pizza", "code": "123456" })).await.0, StatusCode::FORBIDDEN);
    assert_eq!(s.view(&ben, "ops", "").await.0, StatusCode::FORBIDDEN, "fans don't get the operations screen");
    // staff accounts see their own work
    let grill = s.staff("grill", "stand", "grill-104").await;
    let st = s.view(&grill, "stand", "kiosk-212").await.1;
    assert_eq!(st["stand"], "grill-104", "a stand's screen is always its own, whatever it asks for");
    let ops = s.staff("ops", "ops", "ops").await;
    assert!(s.view(&ops, "ops", "").await.1["ledger"]["conserved"].as_bool().unwrap());
    // the clock moves; Ana's order gets made and brought
    let v = s.app.venue("harbor").unwrap();
    v.step(240).await;
    let a = s.view(&ana, "fan", "").await.1;
    let receipts = a["me"]["receipts"].as_array().unwrap();
    assert_eq!(receipts.len(), 1, "delivered and settled: {}", a["orders"]);
    s.app.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_restarted_server_comes_back_exactly_where_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let (ana_view, ops_view, token) = {
        let s = start(dir.path()).await;
        let ana = s.signup("ana").await;
        s.act(&ana, json!({ "do": "join", "seat": "s104" })).await;
        let v = s.app.venue("harbor").unwrap();
        v.step(30).await;
        assert_eq!(s.act(&ana, json!({ "do": "order", "thing": "nachos", "qty": 2, "delivery": true, "tip": 100 })).await["ok"], true);
        v.step(20).await;
        assert_eq!(s.act(&ana, json!({ "do": "tickets", "event": "e2", "section": "s104", "qty": 2 })).await["ok"], true);
        assert_eq!(s.act(&ana, json!({ "do": "follow", "node": "hbk-1" })).await["ok"], true);
        v.step(15).await;
        let ops = s.staff("ops", "ops", "ops").await;
        let out = (s.view(&ana, "fan", "").await.1, s.view(&ops, "ops", "").await.1, ana);
        s.app.shutdown();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        out
    };
    // a new process: accounts, sessions and the whole night come back from disk
    let s = start(dir.path()).await;
    assert_eq!(s.post("/api/me", None, json!({})).await.0, StatusCode::METHOD_NOT_ALLOWED);
    let again = s.view(&token, "fan", "").await.1;
    assert_eq!(again, ana_view, "the fan's screen is identical after a restart");
    let ops = s.signin("ops", "staff pass 1").await;
    assert_eq!(s.view(&ops, "ops", "").await.1, ops_view, "and so is operations, ledger included");
    s.app.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_kitchen_works_from_its_own_device_and_only_its_own_kitchen() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path()).await;
    let ana = s.signup("ana").await;
    s.act(&ana, json!({ "do": "join", "seat": "s104" })).await;
    let grill = s.staff("grill", "stand", "grill-104").await;
    let runner = s.staff("marco", "runner", "runner0").await;
    assert_eq!(s.post("/api/v/harbor/act", Some(&ana), json!({ "do": "ready", "order": 0 })).await.0, StatusCode::FORBIDDEN, "fans don't run kitchens");
    assert_eq!(s.post("/api/v/harbor/act", Some(&runner), json!({ "do": "ready", "order": 0 })).await.0, StatusCode::FORBIDDEN, "nor do runners");
    assert_eq!(s.act(&grill, json!({ "do": "clock_in" })).await["ok"], true);
    assert_eq!(s.act(&runner, json!({ "do": "clock_in" })).await["ok"], true);
    assert_eq!(s.act(&runner, json!({ "do": "accept_trip" })).await["error"], "No trip is waiting for you");
    let id = s.act(&ana, json!({ "do": "order", "thing": "burger", "qty": 1, "delivery": false, "tip": 0 })).await["id"].as_str().unwrap().parse::<u64>().unwrap();
    let v = s.app.venue("harbor").unwrap();
    // the kitchen works through its stations until Ana's burger is up, then marks it ready
    let mut done = false;
    for _ in 0..200 {
        v.step(1).await;
        let st = s.view(&grill, "stand", "").await.1;
        assert_eq!(st["on_shift"], true);
        for station in st["stations"].as_array().unwrap() {
            if let Some(o) = station["order"]["id"].as_u64() {
                if station["until"].as_i64().unwrap() <= st["clock"]["t"].as_i64().unwrap() || o == id {
                    assert_eq!(s.act(&grill, json!({ "do": "ready", "order": o })).await["ok"], true);
                    if o == id { done = true; }
                }
            }
        }
        if done { break; }
    }
    assert!(done, "Ana's burger reached a station and was marked ready");
    let st = s.view(&grill, "stand", "").await.1;
    assert!(st["pickups"].as_array().unwrap().iter().any(|o| o["id"] == id), "waiting at the counter");
    assert_eq!(s.act(&grill, json!({ "do": "handed_over", "order": id })).await["ok"], true);
    v.step(2).await;
    let a = s.view(&ana, "fan", "").await.1;
    assert!(a["me"]["receipts"].as_array().unwrap().iter().any(|o| o["id"] == id && o["stage"] == "Settled"), "she has her burger and her receipt");
    s.app.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_operator_opens_a_new_venue_from_its_template_and_it_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut world: Value = serde_json::from_str(include_str!("../../agape-kernel/templates/arena.json")).unwrap();
    let template: Value = serde_json::from_str(include_str!("../../agape-kernel/templates/basketball.json")).unwrap();
    let open = |s: &Srv, body: Value| { let c = s.c.clone(); let url = format!("{}/api/admin/venues", s.base);
        async move { let r = c.post(url).header("x-admin-token", "op-secret").json(&body).send().await.unwrap(); (r.status(), r.json::<Value>().await.unwrap()) } };
    {
        let s = start(dir.path()).await;
        let mut broken = world.clone();
        broken["things"][0]["stand"] = json!("nowhere");
        let (st, v) = open(&s, json!({ "id": "pier", "name": "Pier Pavilion", "world": broken, "template": template, "fans": 500, "runners": 10 })).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert!(v["issues"].as_array().unwrap().iter().any(|i| i["message"].as_str().unwrap().contains("isn't a stand here")), "{v}");
        world["stands"][0]["name"] = json!("Pier Grill");
        let (st, v) = open(&s, json!({ "id": "pier", "name": "Pier Pavilion", "world": world, "template": template, "fans": 500, "runners": 10 })).await;
        assert_eq!(st, StatusCode::OK, "{v}");
        let ana = s.signup("ana").await;
        assert_eq!(s.post("/api/v/pier/act", Some(&ana), json!({ "do": "join", "seat": "s210" })).await.1["ok"], true);
        assert_eq!(s.post("/api/v/pier/act", Some(&ana), json!({ "do": "order", "thing": "burger", "qty": 1, "delivery": true, "tip": 0 })).await.1["ok"], true);
        let v = s.post("/api/v/pier/view", Some(&ana), json!({ "lens": "fan" })).await.1;
        assert_eq!(v["orders"][0]["stand"], "Pier Grill");
        s.app.shutdown();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    let s = start(dir.path()).await;
    let list = s.c.get(format!("{}/api/venues", s.base)).send().await.unwrap().json::<Value>().await.unwrap();
    assert!(list["venues"].as_array().unwrap().iter().any(|v| v["id"] == "pier"), "the new venue came back after a restart");
    let ana = s.signin("ana", "correct horse").await;
    assert_eq!(s.post("/api/v/pier/view", Some(&ana), json!({ "lens": "fan" })).await.1["orders"].as_array().unwrap().len(), 1);
    s.app.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fans_passport_moves_with_them_between_venues_and_a_league_deal_reaches_every_venue() {
    let dir = tempfile::tempdir().unwrap();
    let world: Value = serde_json::from_str(include_str!("../../agape-kernel/templates/arena.json")).unwrap();
    let template: Value = serde_json::from_str(include_str!("../../agape-kernel/templates/basketball.json")).unwrap();
    let s = start(dir.path()).await;
    let r = s.c.post(format!("{}/api/admin/venues", s.base)).header("x-admin-token", "op-secret")
        .json(&json!({ "id": "pier", "name": "Pier Pavilion", "world": world, "template": template, "fans": 300, "runners": 8 })).send().await.unwrap();
    assert!(r.status().is_success());
    let ana = s.signup("ana").await;
    s.act(&ana, json!({ "do": "join", "seat": "s104" })).await;
    assert_eq!(s.act(&ana, json!({ "do": "tickets", "event": "e2", "section": "s104", "qty": 2 })).await["ok"], true);
    let here = s.view(&ana, "fan", "").await.1["me"].clone();
    let (points, level) = (here["balance"].as_i64().unwrap(), here["level"].as_i64().unwrap());
    assert!(points > 100);
    // to the other venue: her points and level arrive, and no second welcome
    let r = s.post("/api/v/pier/act", Some(&ana), json!({ "do": "join", "seat": "s210" })).await.1;
    assert_eq!(r["ok"], true, "{r}");
    let there = s.post("/api/v/pier/view", Some(&ana), json!({ "lens": "fan" })).await.1["me"].clone();
    assert_eq!(there["balance"].as_i64().unwrap(), points);
    assert_eq!(there["level"].as_i64().unwrap(), level);
    assert_eq!(there["passport"]["venues"], json!(["Harbor Arena", "Pier Pavilion"]));
    assert_eq!(s.view(&ana, "fan", "").await.1["me"]["balance"], 0, "they left the first venue with her");
    // and back again
    s.act(&ana, json!({ "do": "join", "seat": "s104" })).await;
    assert_eq!(s.view(&ana, "fan", "").await.1["me"]["balance"].as_i64().unwrap(), points);
    // a league deal: refused if broken, otherwise in every venue
    let deal = |rule: Value| { let c = s.c.clone(); let u = format!("{}/api/admin/league-deal", s.base);
        async move { c.post(u).header("x-admin-token", "op-secret").json(&json!({ "rule": rule, "payer": "league_sponsor", "budget_per_venue": 10000 })).send().await.unwrap().status() } };
    assert_eq!(deal(json!({ "name": "bad", "when": { "match": { "sig": "nope" } }, "then": [] })).await, StatusCode::BAD_REQUEST);
    let ok = deal(json!({ "name": "league_run", "stratum": 2, "when": { "match": { "sig": "moment", "where": { "kind": { "eq": "run" } } } },
        "then": [{ "offer": { "thing": "foam_finger", "price": 1500, "discount": 500, "payer": "league_sponsor", "expires_ms": 600000 } }] })).await;
    assert_eq!(ok, StatusCode::OK);
    s.app.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_finished_night_opens_the_next_and_fans_keep_their_points() {
    let dir = tempfile::tempdir().unwrap();
    let s = start(dir.path()).await;
    let ana = s.signup("ana").await;
    assert_eq!(s.act(&ana, json!({ "do": "join", "seat": "s104" })).await["ok"], true);
    let before = s.view(&ana, "fan", "").await.1["me"]["balance"].as_i64().unwrap();
    assert!(before > 0, "a welcome balance");
    let h = s.app.venue("harbor").unwrap();
    for _ in 0..40 { h.step(1000).await; if h.over.load(std::sync::atomic::Ordering::Relaxed) { break; } }
    assert!(h.over.load(std::sync::atomic::Ordering::Relaxed), "the night ends");
    let first_t = h.view("agape", json!({ "lens": "ops" })).await["clock"]["t"].as_i64().unwrap_or(i64::MAX);
    agape_server::roll_finished(&s.app).await;
    let next = s.app.venue("harbor").unwrap();
    assert!(!next.over.load(std::sync::atomic::Ordering::Relaxed));
    let v = s.view(&ana, "fan", "").await.1;
    assert!(v["clock"]["t"].as_i64().unwrap() < first_t, "a new night, from the top");
    assert!(v["you"]["seat"].as_str().unwrap_or("").is_empty(), "pick a seat again");
    assert_eq!(s.act(&ana, json!({ "do": "join", "seat": "s112" })).await["ok"], true);
    let after = s.view(&ana, "fan", "").await.1["me"]["balance"].as_i64().unwrap();
    assert_eq!(after, before, "points came home to the passport and back");
    assert!(dir.path().join("venues").read_dir().unwrap().flatten().any(|e| e.file_name().to_string_lossy().ends_with(".night")), "the old night is kept");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_night_from_another_kernel_is_closed_and_the_next_opened() {
    let dir = tempfile::tempdir().unwrap();
    let mut init = agape_server::harbor_init(7);
    init.build = "an older kernel".into();
    let h = agape_server::venue::spawn(&dir.path().join("venues"), Some(init), "harbor", 0.0).unwrap();
    assert!(h.over.load(std::sync::atomic::Ordering::Relaxed), "it won't replay exactly, so it's over");
    h.stop();
    let s = start(dir.path()).await;
    agape_server::roll_finished(&s.app).await;
    let next = s.app.venue("harbor").unwrap();
    assert!(!next.over.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(next.init.build, agape_server::venue::BUILD);
}

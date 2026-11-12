//! Real venues: a venue is set up by filling in a form in Agape Admin, and each event there is a real
//! night with real people (or a rehearsal of one). The form becomes the kernel's own world template;
//! the kernel checks it exactly as it checks any venue before it opens.

use serde_json::{json, Value};

/// The venue form as Admin fills it in, made into the kernel's world. Everything the form doesn't
/// ask about (walking speeds, terms, service rules) starts from Agape's standard arena.
pub fn world_from_form(f: &Value) -> Value {
    let mut w: Value = serde_json::from_str(include_str!("../../agape-kernel/templates/arena.json")).unwrap();
    let s = |k: &str| f[k].as_str().unwrap_or("").trim().to_string();
    let n = |v: &Value, d: i64| v.as_i64().or_else(|| v.as_f64().map(|x| x.round() as i64)).or_else(|| v.as_str().and_then(|x| x.trim().parse().ok())).unwrap_or(d);
    w["venue"] = json!(s("id"));
    w["name"] = json!(s("name"));
    w["address"] = json!(s("address"));
    w["capacity"] = json!(n(&f["capacity"], 0));
    w["levels"] = json!(n(&f["levels"], 1));
    w["sections_per_level"] = json!(n(&f["sections_per_level"], 12));
    for (k, d) in [("ring_s", 14), ("stairs_s", 75), ("lift_s", 120), ("aisle_s", 30)] { if !f["walk"][k].is_null() { w[k] = json!(n(&f["walk"][k], d)); } }
    // nothing outside the building until a venue's own partners are set up
    w["around"] = json!([]); w["businesses"] = json!([]); w["drivers"] = json!([]);
    w["stands"] = json!(f["stands"].as_array().cloned().unwrap_or_default().iter().map(|x| json!({
        "id": x["id"].as_str().unwrap_or("").trim(), "name": x["name"].as_str().unwrap_or("").trim(),
        "level": n(&x["level"], 1), "ring": (n(&x["near"], 1) - 1).max(0), "stations": n(&x["stations"], 2) })).collect::<Vec<_>>());
    w["things"] = json!(f["menu"].as_array().cloned().unwrap_or_default().iter().map(|x| {
        let mut t = json!({ "id": x["id"].as_str().unwrap_or("").trim(), "name": x["name"].as_str().unwrap_or("").trim(), "stand": x["stand"].as_str().unwrap_or(""),
            "price": n(&x["price"], 0), "prep_s": n(&x["prep_s"], 180), "category": x["category"].as_str().unwrap_or("food") });
        if x["adult"] == true { t["constraints"] = json!([{ "min_age": 21 }]); }
        if let Some(st) = x["stock"].as_i64().or_else(|| x["stock"].as_str().and_then(|v| v.trim().parse().ok())) { t["stock"] = json!(st); }
        t
    }).collect::<Vec<_>>());
    if let Some(fee) = f["fees"]["delivery_fee"].as_i64() { w["terms"]["delivery_fee"] = json!(fee); }
    if let Some(tax) = f["fees"]["tax_bps"].as_i64() { w["terms"]["tax_bps"] = json!(tax); }
    // tonight's teams, as the venue shows them
    let team = |t: &Value| json!({ "name": t["name"].as_str().unwrap_or("").trim(), "short": t["short"].as_str().unwrap_or("").trim(),
        "abbr": t["abbr"].as_str().unwrap_or("").trim(), "color": t["color"].as_str().unwrap_or("") });
    w["brand"] = json!({ "home": team(&f["home"]), "away": team(&f["away"]) });
    // the launch set only, unless the venue switches more on
    let mut features = serde_json::Map::new();
    for k in OPTIONAL { features.insert(k.to_string(), json!(f["features"][k] == true)); }
    w["features"] = Value::Object(features);
    w
}

/// What a venue can switch on beyond ordering to the seat. Off unless chosen.
pub const OPTIONAL: &[&str] = &["chat", "predictions", "cards", "tickets", "around", "offers", "players"];

/// The event's rules without the demo's sponsor offers (a real venue's offers are its own, made later).
pub fn plain_template() -> Value {
    let mut t: Value = serde_json::from_str(include_str!("../../agape-kernel/templates/basketball.json")).unwrap();
    fn names_thing(v: &Value) -> bool {
        match v { Value::Object(m) => m.iter().any(|(k, x)| k == "thing" || names_thing(x)), Value::Array(a) => a.iter().any(names_thing), _ => false }
    }
    if let Some(rules) = t["rules"].as_array_mut() { rules.retain(|r| !names_thing(r)); }
    t
}

/// Check a venue form the way the kernel checks any venue, and say what's wrong in words.
pub fn check(f: &Value) -> (Value, Vec<agape_kernel::setup::Issue>) {
    let w = world_from_form(f);
    let mut issues = vec![];
    let id = f["id"].as_str().unwrap_or("");
    if !crate::auth::valid_handle(id) || id.contains('.') { issues.push(agape_kernel::setup::Issue { path: "id".into(), message: "Give the venue a short id: 3 to 24 letters, numbers or underscores".into(), blocking: true }); }
    if id == "harbor" { issues.push(agape_kernel::setup::Issue { path: "id".into(), message: "harbor is the demo's venue; pick another id".into(), blocking: true }); }
    let runners = f["runners"].as_i64().unwrap_or(0);
    if !(1..=200).contains(&runners) { issues.push(agape_kernel::setup::Issue { path: "runners".into(), message: "Plan for 1 to 200 runners".into(), blocking: true }); }
    match (serde_json::from_value::<agape_kernel::world::WorldDef>(w.clone()), serde_json::from_value::<agape_kernel::rules::Template>(plain_template())) {
        (Ok(wd), Ok(t)) => issues.extend(agape_kernel::setup::check(&wd, &t)),
        (Err(e), _) => issues.push(agape_kernel::setup::Issue { path: "form".into(), message: format!("Something in the form can't be read: {e}"), blocking: true }),
        (_, Err(e)) => issues.push(agape_kernel::setup::Issue { path: "template".into(), message: format!("The event rules can't be read: {e}"), blocking: true }),
    }
    // every stand's money goes to a named business
    let ps = payees(f);
    for (pid, pname) in &ps { if pname.trim().is_empty() { issues.push(agape_kernel::setup::Issue { path: format!("payees.{pid}"), message: "Name the business that gets paid (usually the venue or its concessions company)".into(), blocking: true }); } }
    for st in f["stands"].as_array().cloned().unwrap_or_default() {
        let to = payee_of(f, st["id"].as_str().unwrap_or(""));
        if !ps.iter().any(|(id, _)| *id == to) { issues.push(agape_kernel::setup::Issue { path: format!("stands.{}.payee", st["id"].as_str().unwrap_or("")), message: format!("{} is paid to a business that isn't listed", st["name"].as_str().unwrap_or("A stand")), blocking: true }); }
    }
    // the dry run's measurements are about a simulated crowd: not useful here, and slow
    issues.retain(|i| !i.path.starts_with("dry_run"));
    (w, issues)
}

/// A password to read out or write down: no look-alike letters, about 50 bits.
pub fn readable_password() -> String {
    const A: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
    let mut b = [0u8; 10];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut b);
    let s: String = b.iter().map(|x| A[*x as usize % A.len()] as char).collect();
    format!("{}-{}", &s[..5], &s[5..])
}

/// The first form a new venue starts from: a small arena with two stands and a short menu.
pub fn starter_form() -> Value {
    json!({
        "id": "", "name": "", "address": "", "capacity": 8000,
        "home": { "name": "", "short": "", "abbr": "", "color": "#1F4FA8" }, "away": { "name": "", "short": "", "abbr": "", "color": "#B3261E" },
        "levels": 1, "sections_per_level": 20, "walk": { "ring_s": 14, "stairs_s": 75, "lift_s": 120, "aisle_s": 30 },
        "stands": [
            { "id": "grill", "name": "Main Grill", "level": 1, "near": 3, "stations": 3, "payee": "concessions" },
            { "id": "bar", "name": "Bar", "level": 1, "near": 12, "stations": 2, "payee": "concessions" }
        ],
        "payees": [{ "id": "concessions", "name": "" }],
        "menu": [
            { "id": "hotdog", "name": "Hot dog", "stand": "grill", "price": 700, "prep_s": 90, "category": "food" },
            { "id": "nachos", "name": "Nachos", "stand": "grill", "price": 900, "prep_s": 120, "category": "food" },
            { "id": "water", "name": "Bottled water", "stand": "bar", "price": 500, "prep_s": 20, "category": "drink" },
            { "id": "beer", "name": "Draft beer", "stand": "bar", "price": 1200, "prep_s": 45, "category": "drink", "adult": true }
        ],
        "fees": { "delivery_fee": 499, "tax_bps": 600 },
        "runners": 6,
        "features": {}
    })
}

/// The businesses Agape pays at a venue, as (id, name). A form from before payees existed has one: the venue's concessions.
pub fn payees(f: &Value) -> Vec<(String, String)> {
    match f["payees"].as_array() {
        Some(a) if !a.is_empty() => a.iter().map(|x| (x["id"].as_str().unwrap_or("").trim().to_string(), x["name"].as_str().unwrap_or("").trim().to_string())).collect(),
        _ => vec![("concessions".into(), format!("{} concessions", f["name"].as_str().unwrap_or("").trim()).trim().to_string())],
    }
}

/// Which business a stand's money goes to.
pub fn payee_of(f: &Value, stand: &str) -> String {
    f["stands"].as_array().and_then(|a| a.iter().find(|s| s["id"].as_str().map(str::trim) == Some(stand)))
        .and_then(|s| s["payee"].as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        .unwrap_or_else(|| payees(f).first().map(|(id, _)| id.clone()).unwrap_or_else(|| "concessions".into()))
}

/// Who is owed what once a night's orders are final: each business for its stands, the runners, the tax held to
/// remit, and what Agape keeps. All of it comes from the night's books; fans paid Agape, so nobody has to be
/// collected from: Agape's share is kept before anything is paid out.
pub fn payouts(f: &Value, night: &Value, approved: &dyn Fn(&str) -> bool) -> Value {
    let b = &night["books"];
    let i = |v: &Value| v.as_i64().unwrap_or(0);
    let stands = night["stands"].as_array().cloned().unwrap_or_default();
    let list: Vec<Value> = payees(f).into_iter().map(|(id, name)| {
        let mine: Vec<Value> = stands.iter().filter(|s| payee_of(f, s["id"].as_str().unwrap_or("")) == id)
            .map(|s| json!({ "id": s["id"], "name": s["name"], "owed": i(&s["revenue"]) })).collect();
        let owed: i64 = mine.iter().map(|s| i(&s["owed"])).sum();
        json!({ "id": id, "name": name, "owed": owed, "stands": mine, "approved": approved(&id) })
    }).collect();
    let runners: Vec<Value> = night["runners"].as_array().cloned().unwrap_or_default().iter()
        .map(|r| json!({ "id": r["id"], "pay": i(&r["pay"]), "tips": i(&r["tips"]), "owed": i(&r["pay"]) + i(&r["tips"]) })).filter(|r| r["owed"].as_i64().unwrap_or(0) != 0).collect();
    json!({ "payees": list, "runners": runners, "runners_total": runners.iter().map(|r| i(&r["owed"])).sum::<i64>(), "tax": i(&b["tax"]), "agape": i(&b["agape"]), "fans_paid": i(&b["fans_paid"]) })
}

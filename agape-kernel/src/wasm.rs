//! Device bridge: the browser and phones run this same kernel. JSON in, JSON out, through
//! linear memory: `alloc` a buffer, write UTF-8 JSON, call, then read `out_ptr`/`out_len`.

use crate::rules::Template;
use crate::sim::{Config, Night};
use crate::world::WorldDef;
use serde_json::{json, Value};
use std::cell::RefCell;

thread_local! {
    static NIGHT: RefCell<Option<Night>> = const { RefCell::new(None) };
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn input(ptr: *mut u8, len: usize) -> Value {
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

fn output(v: Value) -> usize {
    OUT.with(|o| { let mut o = o.borrow_mut(); *o = serde_json::to_vec(&v).unwrap(); o.len() })
}

#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

#[unsafe(no_mangle)]
pub extern "C" fn dealloc(ptr: *mut u8, len: usize) { unsafe { drop(Vec::from_raw_parts(ptr, 0, len)) } }

#[unsafe(no_mangle)]
pub extern "C" fn out_ptr() -> *const u8 { OUT.with(|o| o.borrow().as_ptr()) }

/// {"world": {...}, "template": {...}, "seed": 1, "fans": 3000, "runners": 40, "you_seat": "s104"}
#[unsafe(no_mangle)]
pub extern "C" fn init(ptr: *mut u8, len: usize) -> usize {
    let v = input(ptr, len);
    let world: Result<WorldDef, _> = serde_json::from_value(v["world"].clone());
    let tpl: Result<Template, _> = serde_json::from_value(v["template"].clone());
    match (world, tpl) {
        (Ok(w), Ok(t)) => {
            let d = Config::default();
            let cfg = Config {
                seed: v["seed"].as_u64().unwrap_or(1), fans: v["fans"].as_u64().map_or(d.fans, |x| x as usize),
                runners: v["runners"].as_u64().map_or(d.runners, |x| x as usize), you_seat: v["you_seat"].as_str().map(String::from), ..d
            };
            NIGHT.with(|n| *n.borrow_mut() = Some(Night::new(w, t, cfg)));
            output(json!({ "ok": true }))
        }
        (w, t) => output(json!({ "ok": false, "error": format!("{:?} {:?}", w.err(), t.err()) })),
    }
}

/// Advance `ticks` steps of 10 s. Returns 1 while the night is running.
#[unsafe(no_mangle)]
pub extern "C" fn step(ticks: u32) -> u32 {
    NIGHT.with(|n| {
        let mut n = n.borrow_mut();
        let Some(n) = n.as_mut() else { return 0 };
        let mut running = true;
        for _ in 0..ticks { if !n.step() { running = false; break; } }
        running as u32
    })
}

/// {"lens": "fan" | "runner" | "stand" | "ops" | "page" | "player" | "business" | "parties", "id", "as"}
#[unsafe(no_mangle)]
pub extern "C" fn view(ptr: *mut u8, len: usize) -> usize {
    let v = input(ptr, len);
    let out = NIGHT.with(|n| {
        let mut n = n.borrow_mut();
        let Some(n) = n.as_mut() else { return json!({ "error": "no night" }) };
        let who = v["as"].as_str().unwrap_or(crate::sim::YOU).to_string();
        crate::api::view(n, &who, &v)
    });
    output(out)
}

/// {"do": ..., "as": who}. The harness can act as anyone; the server never takes "as" from a request.
#[unsafe(no_mangle)]
pub extern "C" fn act(ptr: *mut u8, len: usize) -> usize {
    let v = input(ptr, len);
    let out = NIGHT.with(|n| {
        let mut n = n.borrow_mut();
        let Some(n) = n.as_mut() else { return json!({ "ok": false, "error": "no night" }) };
        let who = v["as"].as_str().unwrap_or(crate::sim::YOU).to_string();
        match v["do"].as_str() {
            Some("drive") => { n.drive(v["party"].as_str().unwrap_or("")); json!({ "ok": true }) }
            Some("release") => { n.driven.remove(v["party"].as_str().unwrap_or("")); json!({ "ok": true }) }
            Some("claim") => crate::api::act(n, v["member"].as_str().unwrap_or(""), &v),
            _ => crate::api::act(n, &who, &v),
        }
    });
    output(out)
}

/// Setup: {"world", "template"} → the template's issues.
#[unsafe(no_mangle)]
pub extern "C" fn check(ptr: *mut u8, len: usize) -> usize {
    let v = input(ptr, len);
    let out = match (serde_json::from_value::<WorldDef>(v["world"].clone()), serde_json::from_value::<Template>(v["template"].clone())) {
        (Ok(w), Ok(t)) => json!({ "ok": true, "issues": crate::setup::check(&w, &t) }),
        (w, t) => json!({ "ok": false, "issues": [{ "path": "template", "message": format!("The template can't be read: {}", w.err().map(|e| e.to_string()).or(t.err().map(|e| e.to_string())).unwrap_or_default()), "blocking": true }] }),
    };
    output(out)
}

/// Setup: {"world", "template", "fans", "runners"} → issues, and a whole simulated night's report.
#[unsafe(no_mangle)]
pub extern "C" fn preview(ptr: *mut u8, len: usize) -> usize {
    let v = input(ptr, len);
    let out = match (serde_json::from_value::<WorldDef>(v["world"].clone()), serde_json::from_value::<Template>(v["template"].clone())) {
        (Ok(w), Ok(t)) => crate::setup::preview(w, t, v["fans"].as_u64().unwrap_or(1000) as usize, v["runners"].as_u64().unwrap_or(15) as usize),
        _ => json!({ "ok": false, "issues": [{ "path": "template", "message": "The template can't be read", "blocking": true }] }),
    };
    output(out)
}

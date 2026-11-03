//! The one way in for every device and the server: a party asks for a view or takes an action,
//! as JSON. Who is asking (`who`) is decided by the caller: the server from the signed-in session,
//! the simulation harness from its "see it as" choice. Nothing here trusts a party named inside
//! the request.

use crate::sim::Night;
use serde_json::{json, Value};

/// A projection for `who`. `id` names whose screen for role lenses (the server sets it to `who`).
pub fn view(n: &mut Night, who: &str, v: &Value) -> Value {
    let id = v["id"].as_str().unwrap_or("");
    match v["lens"].as_str() {
        Some("fan") => n.view_fan(if id.is_empty() { who } else { id }),
        Some("runner") => n.view_runner(id),
        Some("stand") => n.view_stand(id),
        Some("ops") => n.view_ops(),
        Some("page") => n.view_page(id, who),
        Some("player") => n.view_player(id),
        Some("business") => n.view_business(id),
        Some("parties") => n.parties(),
        Some("glance") => n.view_glance(),
        _ => json!({ "error": "unknown lens" }),
    }
}

/// An action taken by `who`.
pub fn act(n: &mut Night, who: &str, v: &Value) -> Value {
    let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
    if v["do"] == "ticket_quote" {
        return match n.ticket_quote(&s("event"), &s("section"), v["qty"].as_i64().unwrap_or(1)) {
            Ok((b, left, seats)) => json!({ "ok": true, "quote": b, "left": left, "seats": seats }), Err(e) => json!({ "ok": false, "error": e }) };
    }
    if v["do"] == "ride_quote" { let at = v["at"].as_i64().unwrap_or(0); return json!({ "ok": true, "fare": n.ride_quote(at), "free": n.drivers_free_at(&s("provider"), at) }); }
    if v["do"] == "window" {
        return match n.order_window_for(who, &s("thing"), v["delivery"].as_bool().unwrap_or(true)) {
            Some((a, b)) => json!({ "ok": true, "earliest": a, "closes": b }), None => json!({ "ok": false, "error": "unknown item" }) };
    }
    if v["do"] == "quote" {
        return match n.quote_for(who, &s("thing"), v["qty"].as_i64().unwrap_or(1), v["delivery"].as_bool().unwrap_or(true), v["tip"].as_i64().unwrap_or(0), v["claim"].as_str()) {
            Ok(q) => json!({ "ok": true, "quote": q }), Err(e) => json!({ "ok": false, "error": e }) };
    }
    let r: Result<String, String> = match v["do"].as_str() {
        Some("order") => n.order_as(who, &s("thing"), v["qty"].as_i64().unwrap_or(1), v["delivery"].as_bool().unwrap_or(true), v["tip"].as_i64().unwrap_or(0), v["at"].as_i64()),
        Some("tap") => n.tap_as(who, &s("offer")),
        Some("checkout") => n.checkout_as(who, &s("claim"), v["tip"].as_i64().unwrap_or(0)),
        Some("predict") => n.predict(who, &s("pick")).map(|_| String::new()),
        Some("collect") => n.accept(&format!("drop:{}", s("edition")), who, 1, None, "", "live").map(|_| ()).map_err(|e| if e == "Unknown offer" { "Closed".into() } else { e }).map(|_| String::new()),
        Some("list") => n.list_card(who, &s("edition"), v["price"].as_i64().unwrap_or(0)),
        Some("delist") => n.delist_card(who, &s("listing")).map(|_| String::new()),
        Some("buy") => n.accept(&s("listing"), who, 1, None, "", "market").map(|_| ()).map(|_| String::new()),
        Some("follow") => Ok(n.follow(who, &s("node")).to_string()),
        Some("message") => n.message(who, &s("to"), &s("text")).map(|_| String::new()),
        Some("unlock") => n.unlock(who, &s("place")).map(|_| String::new()),
        Some("tickets") => n.buy_tickets(who, &s("event"), &s("section"), v["qty"].as_i64().unwrap_or(1)).map(|seats| seats.join(", ")),
        Some("ride") => n.book_ride(who, &s("zone"), v["at"].as_i64().unwrap_or(0)),
        Some("cancel_ride") => n.cancel_ride(who, &s("ride")).map(|_| String::new()),
        Some("parking") => n.prepay_parking(who, &s("lot")).map(|_| String::new()),
        Some("deal") => n.propose_deal(who, &s("to"), &s("kind"), &s("ask"), v["amount"].as_i64().unwrap_or(0)),
        Some("respond") => n.respond_deal(who, &s("deal"), v["yes"].as_bool().unwrap_or(false)).map(|_| String::new()),
        Some("deliver") => n.deliver_deal(who, &s("deal"), &s("proof")).map(|_| String::new()),
        Some("confirm") => n.confirm_deal(who, &s("deal")).map(|_| String::new()),
        Some("withdraw") => n.withdraw_deal(who, &s("deal")).map(|_| String::new()),
        Some("claim") => n.claim(who).map(|_| String::new()),
        Some("redeem") => n.redeem_at(who, &s("place"), &s("code")).map(|f| n.party_name(&f).unwrap_or(f)),
        Some("reply") => n.message(who, &s("to"), &s("text")).map(|_| String::new()),
        Some("clock_in") => n.clock_in(who).map(|_| String::new()),
        Some("clock_out") => n.clock_out(who).map(|_| String::new()),
        Some("ready") => n.mark_ready(who, v["order"].as_u64().unwrap_or(u64::MAX) as usize).map(|_| String::new()),
        Some("cannot_make") => n.cannot_make(who, v["order"].as_u64().unwrap_or(u64::MAX) as usize).map(|_| String::new()),
        Some("handed_over") => n.handed_over(who, v["order"].as_u64().unwrap_or(u64::MAX) as usize).map(|_| String::new()),
        Some("accept_trip") => n.accept_trip(who).map(|_| String::new()),
        Some("decline_trip") => n.decline_trip(who).map(|_| String::new()),
        Some("picked_up") => n.picked_up(who).map(|_| String::new()),
        Some("delivered") => n.delivered(who, v["order"].as_u64().unwrap_or(u64::MAX) as usize).map(|_| String::new()),
        Some("unreachable") => n.unreachable(who, v["order"].as_u64().unwrap_or(u64::MAX) as usize).map(|_| String::new()),
        Some("gift") => n.gift(who, &s("thing"), v["qty"].as_i64().unwrap_or(1), &s("to"), &s("note"), v["tip"].as_i64().unwrap_or(0)),
        Some("prioritize") => n.prioritize(who, v["order"].as_u64().unwrap_or(u64::MAX) as usize).map(|_| String::new()),
        Some("upgrade") => n.accept(&format!("upg:{}", s("section")), who, 1, None, "", "me"),
        Some("resell") => n.resell(who, &s("event"), &s("section"), v["price"].as_i64().unwrap_or(0)),
        Some("unlist_tickets") => n.unlist_tickets(who, &s("listing")).map(|_| String::new()),
        Some("buy_resale") => { let q = n.trade.offer(&s("listing")).map(|o| o.sub.split(',').count() as i64).unwrap_or(1); n.accept(&s("listing"), who, q, None, "", "event") }
        // the server moves passports and installs league deals; no person's account can ask for these
        Some("carry_in") if who == "agape" => n.carry_in(&s("transfer"), &s("fan"), v["points"].as_i64().unwrap_or(0), v["level"].as_i64().unwrap_or(0), v["passport"].clone()).map(|_| String::new()),
        Some("carry_out") if who == "agape" => n.carry_out(&s("transfer"), &s("fan")).map(|c| serde_json::to_string(&c).unwrap()),
        Some("install_rule") if who == "agape" => n.install_rule(v["rule"].clone(), &s("payer"), v["budget"].as_i64().unwrap_or(0)).map(|_| String::new()),
        Some("rate") => n.rate(who, v["order"].as_u64().unwrap_or(u64::MAX) as usize, &s("who"), v["stars"].as_i64().unwrap_or(0)).map(|_| String::new()),
        Some("reorder") => n.reorder(who, v["order"].as_u64().unwrap_or(u64::MAX) as usize),
        Some("report") => n.report_problem(who, v["order"].as_u64().unwrap_or(u64::MAX) as usize, &s("reason"), &s("note")),
        Some("resolve") => n.resolve_problem(who, &s("problem"), v["refund"].as_bool().unwrap_or(false), &s("why")).map(|_| String::new()),
        Some("halt") => n.halt(&s("reason")).map(|_| String::new()),
        Some("resume") => n.resume().map(|_| String::new()),
        Some("join") => n.join_named(who, &s("seat"), &s("name"), v["welcome"].as_bool().unwrap_or(true)).map(|_| String::new()),
        Some("attest_age") => n.attest_age(who).map(|_| String::new()),
        Some("wear") => n.wear(who, &s("team")).map(|_| String::new()),
        Some("chat") => n.chat_as(who, &s("text")).map(|_| String::new()),
        Some("cancel") => n.cancel_as(who, v["order"].as_u64().unwrap_or(u64::MAX) as usize).map(|_| String::new()),
        _ => Err("unknown action".into()),
    };
    match r { Ok(id) => json!({ "ok": true, "id": id }), Err(e) => json!({ "ok": false, "error": e }) }
}

//! A live fan acting inside a simulated night, seen only through the projections.

use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night, YOU};
use agape_kernel::world::WorldDef;
use serde_json::Value;

fn night() -> Night {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    Night::new(w, t, Config { you_seat: Some("s104".into()), ..Config::default() })
}

fn until(n: &mut Night, min: i64) { while n.t < min * 60_000 && n.step() {} }

#[test]
fn a_live_fan_orders_watches_it_arrive_and_finds_the_receipt() {
    let mut n = night();
    until(&mut n, 50);
    let v = n.view_fan(YOU);
    let burger = v["items"].as_array().unwrap().iter().find(|m| m["id"] == "burger").unwrap().clone();
    assert_eq!(burger["available"], true);
    let promised_eta = burger["eta"].as_i64().unwrap();
    let id: usize = n.you_order("burger", 1, true, 200).unwrap().parse().unwrap();
    let v = n.view_fan(YOU);
    let card = &v["orders"][0];
    assert_eq!((card["id"].as_u64(), card["stage"].as_str()), (Some(id as u64), Some("Accepted")));
    let mut stages = vec!["Accepted".to_string()];
    for _ in 0..400 {
        n.step();
        let v = n.view_fan(YOU);
        let s = v["orders"].as_array().unwrap().iter().find(|o| o["id"] == id).map(|o| o["stage"].as_str().unwrap().to_string());
        match s { Some(s) => if stages.last() != Some(&s) { stages.push(s) }, None => break }
    }
    // stages only move forward (a step can pass through more than one, e.g. ready and assigned together)
    let order = ["Accepted", "Preparing", "Ready", "Assigned", "PickedUp", "Delivered"];
    let idx: Vec<usize> = stages.iter().map(|s| order.iter().position(|o| o == s).unwrap()).collect();
    assert!(idx.windows(2).all(|w| w[0] < w[1]), "{stages:?}");
    assert_eq!((stages.first().map(|s| s.as_str()), stages.last().map(|s| s.as_str())), (Some("Accepted"), Some("Delivered")));
    let v = n.view_fan(YOU);
    let r = &v["me"]["receipts"][0];
    assert_eq!(r["stage"], "Settled");
    assert_eq!(r["charge"], 1350 + 94 + 499 + 200);
    assert_eq!(v["me"]["spent"], r["charge"], "what the fan was charged is what the ledger says");
    assert!(v["me"]["points"].as_i64().unwrap() > 0);
    let delivered = r["delivered"].as_i64().unwrap();
    assert!(delivered <= promised_eta + 10 * 60_000, "arrived within ten minutes of the estimate shown before ordering");
}

#[test]
fn the_live_fan_sees_and_claims_an_offer_with_its_limits() {
    let mut n = night();
    let mut claimed = None;
    while n.step() {
        let v = n.view_fan(YOU);
        if let Some(o) = v["get"]["offers"].as_array().unwrap().iter().find(|o| o["ended"].is_null()) {
            // every card states how it can end
            assert!(o["guaranteed"].as_bool().unwrap() || o["left"].is_i64() || o["condition"].is_string() || o["ends_at"].is_i64());
            let claim = n.you_tap(o["id"].as_str().unwrap()).expect("a live card can be claimed");
            let v = n.view_fan(YOU);
            let card = v["get"]["offers"].as_array().unwrap().iter().find(|x| x["id"] == o["id"]).unwrap();
            assert_eq!(card["claim"]["id"], Value::String(claim.clone()), "the card shows the claim in hand");
            n.step();
            let order: usize = n.you_checkout(&claim, 0).unwrap().parse().unwrap();
            claimed = Some((order, o["source"].as_str().unwrap().to_string()));
            break;
        }
    }
    let (order, source) = claimed.expect("an offer reached the live fan");
    let v = n.view_fan(YOU);
    let card = v["orders"].as_array().unwrap().iter().find(|x| x["id"] == order).unwrap();
    assert_eq!(card["source"], Value::String(source));
}

#[test]
fn every_lens_reads_one_world() {
    let mut n = night();
    until(&mut n, 110);
    n.you_order("draft", 1, true, 0).unwrap();
    for _ in 0..30 { n.step(); }
    let ops = n.view_ops();
    assert_eq!(ops["ledger"]["conserved"], true);
    let fan = n.view_fan(YOU);
    // the runner carrying your order is the same runner the runner lens shows carrying it
    if let Some(r) = fan["map"]["your_runner"]["id"].as_str() {
        let rv = n.view_runner(r);
        assert!(rv["bag"].as_array().unwrap().iter().any(|o| o["seat"] == "s104"));
    }
    // the stand lens and the fan's menu agree on the wait
    let st = n.view_stand("grill-104");
    let menu_eta = fan["items"].as_array().unwrap().iter().find(|m| m["id"] == "burger").unwrap()["eta"].as_i64().unwrap();
    assert!(menu_eta >= n.t + st["wait"].as_i64().unwrap() * 1000);
    // last call is shown on the menu before anyone tries
    until(&mut n, 155);
    let fan = n.view_fan(YOU);
    let draft = fan["items"].as_array().unwrap().iter().find(|m| m["id"] == "draft").unwrap();
    assert_eq!((draft["available"].as_bool(), draft["why_not"].as_str()), (Some(false), Some("SpanClosed(\"last_call\")")));
    assert!(n.you_order("draft", 1, true, 0).is_err());
}

#[test]
fn fans_never_see_paused_offers_and_every_live_one_shows_it_running_out() {
    let mut n = night();
    let mut saw_paused_in_book = false;
    while n.step() {
        if n.t % 60_000 != 0 { continue; }
        let v = n.view_fan(YOU);
        for o in v["get"]["offers"].as_array().unwrap() {
            assert_ne!(o["ended"], "Paused", "a paused offer reached a fan");
            if o["ended"].is_null() && o["guaranteed"] == false {
                // a while-it-lasts card carries what it needs to draw itself draining
                assert!(o["opened_at"].is_i64() && (o["ends_at"].is_i64() || o["left_at_open"].is_i64()));
            }
        }
        saw_paused_in_book |= n.book.offers.keys().any(|id| n.book.disclose(id, YOU, &n.ledger).map_or(false, |d| d.ended == Some(agape_kernel::offers::End::Paused)));
    }
    assert!(saw_paused_in_book, "the night had paused offers; the fan never saw them");
}

#[test]
fn your_section_hears_you_and_only_your_section() {
    let mut n = night();
    until(&mut n, 60);
    n.you_chat("Section 104 is louder than you").unwrap();
    let v = n.view_fan(YOU);
    let chat = v["live"]["chat_section"].as_array().unwrap();
    assert_eq!(chat[0]["you"], true);
    let other = n.fans.iter().find(|f| f.seat != "s104").unwrap().id.clone();
    let ov = n.view_fan(&other);
    assert!(ov["live"]["chat_section"].as_array().unwrap().iter().all(|c| c["text"] != "Section 104 is louder than you"));
    assert!(n.you_chat("").is_err());
}

#[test]
fn predicting_a_quarter_pays_points_through_the_ledger() {
    let mut n = night();
    until(&mut n, 46);
    let v = n.view_fan(YOU);
    assert_eq!(v["live"]["prediction"]["open"], true, "picks open as the quarter starts");
    n.you_predict("home").unwrap();
    let before = n.view_fan(YOU)["me"]["points"].as_i64().unwrap();
    until(&mut n, 52);
    assert!(n.you_predict("away").is_err(), "picks lock a few minutes in");
    until(&mut n, 74);
    let v = n.view_fan(YOU);
    let p = &v["live"]["prediction"];
    let won = p["won"].as_bool().unwrap();
    let after = v["me"]["points"].as_i64().unwrap();
    assert_eq!(after - before, if won { 50 } else { 0 });
    assert!(n.ledger.units().iter().all(|u| n.ledger.total(u) == 0));
}

#[test]
fn chat_is_the_whole_arena_and_your_section_is_a_filter() {
    let mut n = night();
    until(&mut n, 60);
    n.you_chat("hello arena").unwrap();
    let v = n.view_fan(YOU);
    let all = v["live"]["chat"].as_array().unwrap();
    assert!(all.iter().any(|c| c["mine"] == false), "other sections are heard");
    assert_eq!(all[0]["text"], "hello arena");
    assert!(v["live"]["chat_section"].as_array().unwrap().iter().all(|c| c["section"] == "s104"));
}

#[test]
fn every_name_in_live_opens_a_page_and_fan_pages_stay_public() {
    let mut n = night();
    until(&mut n, 80);
    let v = n.view_fan(YOU);
    let mut opened = 0;
    for e in v["live"]["event"].as_array().unwrap() {
        if let Some(p) = e["party"].as_str().filter(|p| !p.is_empty()) {
            let page = n.view_page(p, YOU);
            assert!(page["name"].is_string(), "{p} has a page");
            opened += 1;
        }
    }
    assert!(opened > 5);
    let other = v["live"]["chat"].as_array().unwrap().iter().find(|c| c["you"] == false).unwrap()["id"].as_str().unwrap().to_string();
    let page = n.view_page(&other, YOU);
    for private in ["orders", "receipts", "spent", "seat", "balance"] { assert!(page.get(private).is_none(), "a fan page exposed {private}"); }
    assert_eq!(n.view_page("hbk", YOU)["roster"].as_array().unwrap().len(), 8);
    assert!(n.you_follow("hbk-1"));
    assert_eq!(n.view_page("hbk-1", YOU)["you_follow"], true);
    n.you_message(&other, "great seats tonight").unwrap();
    assert_eq!(n.view_page(&other, YOU)["thread"][0]["text"], "great seats tonight");
}

#[test]
fn ordering_ahead_arrives_at_the_moment_not_before() {
    let mut n = night();
    until(&mut n, 50);
    let halftime = n.later().into_iter().find(|l| l.1 == "halftime").unwrap().0;
    let id: usize = n.you_order_at("burger", 1, true, 200, Some(halftime)).unwrap().parse().unwrap();
    let card = |n: &mut Night| n.view_fan(YOU)["orders"].as_array().unwrap().iter().find(|o| o["id"] == id).cloned();
    assert_eq!(card(&mut n).unwrap()["scheduled_for"], halftime);
    until(&mut n, halftime / 60_000 - 15);
    assert_eq!(card(&mut n).unwrap()["stage"], "Accepted", "the kitchen hasn't started it this early");
    let mut delivered = None;
    while n.step() {
        if let Some(r) = n.view_fan(YOU)["me"]["receipts"].as_array().unwrap().iter().find(|o| o["id"] == id) { delivered = r["delivered"].as_i64(); break; }
    }
    let d = delivered.expect("it arrived");
    assert!(d >= halftime - 6 * 60_000 && d <= halftime + 10 * 60_000, "arrived around halftime: {} vs {}", d / 60_000, halftime / 60_000);
    assert!(n.you_order_at("burger", 1, true, 0, Some(n.t - 1)).is_err(), "can't order for a moment that has passed");
}

#[test]
fn buying_tickets_gets_real_seats_side_by_side() {
    let mut n = night();
    until(&mut n, 60);
    let (b, left, preview) = n.ticket_quote("e2", "s104", 2).unwrap();
    assert_eq!(b.charge, b.subtotal + b.fee + b.tax);
    assert_eq!(preview.len(), 2);
    let before = agape_kernel::social::lifetime(&n.ledger, YOU);
    let seats = n.buy_tickets(YOU, "e2", "s104", 2).unwrap();
    assert_eq!(seats, preview, "you get exactly the seats the quote showed");
    let (row_a, row_b) = (&seats[0][..1], &seats[1][..1]);
    let (na, nb): (u32, u32) = (seats[0][1..].parse().unwrap(), seats[1][1..].parse().unwrap());
    assert!(row_a == row_b && nb == na + 1, "side by side: {seats:?}");
    let v = n.view_fan(YOU);
    assert_eq!(v["me"]["tickets"][0]["seats"].as_array().unwrap().len(), 2);
    assert_eq!(n.ticket_quote("e2", "s104", 1).unwrap().1, left - 2);
    assert_eq!(agape_kernel::social::lifetime(&n.ledger, YOU) - before, b.points);
    let page = n.view_page("e2", YOU);
    assert_eq!(page["home"]["id"], "hbk");
    assert_eq!(page["yours"][0]["section"], "s104");
    assert!(n.buy_tickets(YOU, "e2", "sFL", 9).is_err());
    assert!(n.ledger.units().iter().all(|u| n.ledger.total(u) == 0));
}

#[test]
fn points_unlock_partner_perks_and_go_back_to_the_issuer() {
    use agape_kernel::social::{lifetime, points};
    use agape_kernel::value::acct;
    let mut n = night();
    until(&mut n, 50);
    assert_eq!(n.unlock(YOU, "dock-pizza"), Err("CantAfford".into()), "100 welcome points don't cover a 150-point perk");
    n.buy_tickets(YOU, "e2", "s104", 2).unwrap(); // earns over 1,000 points
    let (bal, lvl) = (n.ledger.balance(&acct(YOU, "points"), &points()), lifetime(&n.ledger, YOU));
    let issued = n.ledger.balance(&acct("agape", "issued"), &points());
    n.unlock(YOU, "dock-pizza").unwrap();
    assert_eq!(n.ledger.balance(&acct(YOU, "points"), &points()), bal - 150);
    assert_eq!(n.ledger.balance(&acct("agape", "issued"), &points()), issued + 150, "spent points return to the issuer");
    assert_eq!(lifetime(&n.ledger, YOU), lvl, "spending never lowers your level");
    assert_eq!(n.unlock(YOU, "dock-pizza"), Err("Already yours".into()));
    let v = n.view_fan(YOU);
    assert_eq!(v["me"]["passes"][0]["place"], "dock-pizza");
    assert!(n.ledger.units().iter().all(|u| n.ledger.total(u) == 0));
}

#[test]
fn any_time_inside_the_window_can_be_ordered_for() {
    let mut n = night();
    until(&mut n, 60);
    let (earliest, closes) = n.order_window("burger", true).unwrap();
    assert!(n.you_order_at("burger", 1, true, 0, Some(earliest - 5 * 60_000)).is_err(), "sooner than it can arrive");
    assert!(n.you_order_at("burger", 1, true, 0, Some(closes + 60_000)).is_err(), "after ordering closes");
    let odd = earliest + 37 * 60_000 + 20_000;
    assert!(n.you_order_at("burger", 1, true, 0, Some(odd)).is_ok(), "any time in between, not just set moments");
}

#[test]
fn a_ride_has_a_driver_a_lifecycle_and_a_fare_that_balances() {
    let mut n = night();
    until(&mut n, 150);
    let fin = n.script_final();
    // a booking is a promise: when every driver is already promised around a time, it's refused up front
    let full = (0..60).map(|m| fin + m * 60_000).find(|&at| n.drivers_free_at("harborline", at) < 1);
    if let Some(at) = full { assert_eq!(n.book_ride(YOU, "ride-b", at), Err("NoDriver".into())); }
    let at = (0..60).map(|m| fin + m * 60_000).find(|&at| n.drivers_free_at("harborline", at) > 0).expect("a free driver at some point");
    let id = n.book_ride(YOU, "ride-b", at).unwrap();
    assert!(n.book_ride(YOU, "ride-b", fin + 10 * 60_000).is_err(), "one ride at a time");
    n.prepay_parking(YOU, "lot-c").unwrap();
    assert!(n.prepay_parking(YOU, "lot-c").is_err());
    let mut saw_driver = None;
    while n.step() {
        let v = n.view_fan(YOU);
        if let Some(r) = v["me"]["rides"].as_array().unwrap().iter().find(|r| r["id"] == id.as_str()) {
            if let Some(d) = r["driver"]["id"].as_str() { saw_driver = Some(d.to_string()); }
            if r["stage"] == "Done" { break; }
        }
    }
    let d = saw_driver.expect("a driver was assigned");
    let page = n.view_page(&d, YOU);
    assert_eq!(page["provider"]["id"], "harborline");
    assert!(page["with_you"]["fare"].as_i64().unwrap() > 0);
    assert!(n.ledger.balance(&agape_kernel::value::acct(&d, "earnings"), &agape_kernel::value::Unit::usd()) > 0, "the driver was paid");
    assert!(n.ledger.units().iter().all(|u| n.ledger.total(u) == 0));
}

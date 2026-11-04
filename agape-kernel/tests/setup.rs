//! Onboarding a venue is filling in its template; Setup checks it and previews a night on it.

use agape_kernel::rules::Template;
use agape_kernel::setup::{check, preview};
use agape_kernel::world::WorldDef;

fn arena() -> WorldDef { serde_json::from_str(include_str!("../templates/arena.json")).unwrap() }
fn game() -> Template { serde_json::from_str(include_str!("../templates/basketball.json")).unwrap() }

#[test]
fn the_pilot_venue_passes_its_own_checks() {
    let issues = check(&arena(), &game());
    assert!(issues.iter().all(|i| !i.blocking), "{issues:#?}");
}

#[test]
fn a_template_with_mistakes_is_told_exactly_what_to_fix() {
    let mut w = arena();
    w.things[0].stand = "grill-999".into();
    w.things[1].price = 0;
    w.stands[0].ring = 99;
    w.things.retain(|t| t.id != "combo");
    let cheap = w.things.iter().position(|t| t.category == "drink").unwrap();
    w.things[cheap].price = 20; // a 20¢ drink can't even cover its card processing
    let issues = check(&w, &game());
    let says = |s: &str| issues.iter().any(|i| i.blocking && i.message.contains(s));
    assert!(says("which isn't a stand here"), "{issues:#?}");
    assert!(says("needs a price"));
    assert!(says("but a level has 24 sections"));
    assert!(says("\"combo\", which isn't on this venue's menu"), "the event's rules offer something the venue doesn't sell");
    assert!(says("leaves its stand"), "the Agape check runs on the template: {issues:#?}");
}

#[test]
fn a_preview_night_runs_on_the_template_before_anyone_arrives() {
    let mut w = arena();
    w.stands[0].stations = 8; // what if the grill had twice the stations?
    let v = preview(w, game(), 1000, 15);
    assert_eq!(v["ok"], true, "{v}");
    let r = &v["report"];
    assert!(r["orders"].as_u64().unwrap() > 300);
    assert_eq!(r["conserved"], true);
    assert!(r["wait_p90_s"].as_i64().unwrap() > 0);
    let bad = preview({ let mut w = arena(); w.things[0].price = -5; w }, game(), 1000, 15);
    assert_eq!(bad["ok"], false, "a broken template doesn't run");
}

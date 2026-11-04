use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night};
use agape_kernel::world::{EventDef, WorldDef};
fn main() {
    let mut w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    w.event = Some(EventDef { kind: "concert".into(), title: "Lumen Rivers: The Tidelines Tour".into(), acts: vec!["Paper Kites Club".into(), "Northbound".into(), "Lumen Rivers".into()],
        songs: ["Harbor Lights", "Slow Burn", "Paper Moons", "Northern Static", "Satellite Hearts", "Undertow", "Glasshouse", "Gold in the Water", "Saltwater Radio", "Long Way Home", "Fever Line", "Night Swimmer", "Ember", "Tidelines"].iter().map(|s| s.to_string()).collect(),
        favorites: vec!["Satellite Hearts".into(), "Fever Line".into()], early: vec![] });
    w.features.insert("predictions".into(), false);
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let mut n = Night::new(w, t, Config { seed: 5, you_seat: Some("s104".into()), ..Config::default() });
    let mut last = String::new();
    while n.step() {
        let v = n.view_fan("you");
        let l = v["clock"]["label"].as_str().unwrap().to_string();
        if l != last { println!("{} {}", n.t / 60000, l); last = l; }
    }
    let v = n.view_fan("you");
    for f in v["live"]["event"].as_array().unwrap().iter().take(12) { println!("  {} {}", f["kind"], f["text"]); }
    println!("conserved {}", n.ledger.units().iter().all(|u| n.ledger.total(u) == 0));
}

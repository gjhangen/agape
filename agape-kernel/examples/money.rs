use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night};
use agape_kernel::world::WorldDef;
use agape_kernel::value::Unit;
fn main() {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let mut n = Night::new(w, t, Config { seed: 7, fans: 3000, runners: 40, ..Config::default() });
    while n.step() {}
    let mut g = std::collections::BTreeMap::new();
    for ((a, u), v) in n.ledger.balances_iter() { if *u != Unit::usd() { continue }
        let p = a.party.trim_end_matches(|c: char| c.is_ascii_digit()).to_string();
        let p = if p.starts_with("fan") || p == "you" { "fan".into() } else { p };
        *g.entry(format!("{p}/{}", a.purpose)).or_insert(0i64) += *v; }
    for (k, v) in g { println!("{k:40} {:>12.2}", v as f64 / 100.0); }
}

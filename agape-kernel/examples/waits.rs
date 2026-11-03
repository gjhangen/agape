use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night};
use agape_kernel::world::WorldDef;
fn main() {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let ids: Vec<(String, u32)> = w.stands.iter().map(|s| (s.id.clone(), s.stations)).collect();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let mut n = Night::new(w, t, Config { seed: 7, fans: 3000, runners: 40, you_seat: None, ..Config::default() });
    let mut worst = std::collections::BTreeMap::new();
    while n.step() { if n.t % (10 * 60_000) == 0 { print!("{}m:{} ", n.t / 60_000, n.stand_wait("grill-104") / 60); } if n.t % (5 * 60_000) == 0 { for (s, _) in &ids { let w = n.stand_wait(s); let e = worst.entry(s.clone()).or_insert(0); if w > *e { *e = w; } } } }
    println!();
    for (s, st) in &ids { println!("{s:14} stations {st}  worst wait {} min", worst[s] / 60); }
}

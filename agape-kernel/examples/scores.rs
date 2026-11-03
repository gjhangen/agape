use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night};
use agape_kernel::world::WorldDef;
fn main() {
    for seed in 1..=10 {
        let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
        let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
        let mut n = Night::new(w, t, Config { seed, fans: 3000, runners: 40, you_seat: None, ..Config::default() });
        let mut half = (0, 0);
        while n.step() { if n.t == 100 * 60_000 { half = n.score; } }
        println!("seed {seed}: half {:?} final {:?} chat {}", half, n.score, n.chat.len());
    }
}

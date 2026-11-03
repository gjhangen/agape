use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night};
use agape_kernel::world::WorldDef;
fn main() {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let mut n = Night::new(w, t, Config { seed: 7, fans: 3000, runners: 40, you_seat: None, ..Config::default() });
    let mut mid = None;
    while n.step() { if n.t == 120 * 60_000 { mid = Some(n.view_business("dock-pizza")["passes"].clone()); } }
    println!("mid {}", mid.unwrap());
    println!("end {}", n.view_business("dock-pizza")["passes"]);
}

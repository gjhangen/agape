//! How long each screen takes on a packed real night: 3,000 fans in, orders at halftime pace.
use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night};
use agape_kernel::world::WorldDef;
use std::time::Instant;
fn main() {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let mut n = Night::new(w, t, Config { fans: 0, runners: 40, real: true, ..Config::default() });
    let stands: Vec<String> = n.world.def.stands.iter().map(|s| s.id.clone()).collect();
    for s in &stands { n.clock_in(s).unwrap(); }
    for r in 0..40 { n.clock_in(&format!("runner{r}")).unwrap(); }
    let fans = 3000;
    for i in 0..fans { n.join_named(&format!("u{i}"), &format!("s{}{:02}", 1 + i % 2, 1 + i % 24), "x", true).unwrap(); }
    let things: Vec<String> = n.world.def.things.iter().filter(|t| t.id != "combo" && t.constraints.is_empty()).map(|t| t.id.clone()).collect();
    let mut k = 0;
    for _ in 0..60 { for _ in 0..50 { let _ = n.order_as(&format!("u{}", k % fans), &things[k % things.len()], 1, true, 0, None); k += 1; } n.step(); }
    let time = |name: &str, f: &mut dyn FnMut()| { let t0 = Instant::now(); for _ in 0..50 { f(); } println!("{name}: {:.2} ms", t0.elapsed().as_secs_f64() * 1000.0 / 50.0); };
    time("view fan", &mut || { let _ = n.view_fan("u7"); });
    if std::env::var("ONLY").is_ok() { for _ in 0..200 { let _ = n.view_fan("u7"); } return; }
    time("view stand", &mut || { let _ = n.view_stand(&stands[0]); });
    time("view runner", &mut || { let _ = n.view_runner("runner3"); });
    if std::env::var("ONLY").is_ok() { return; }
    time("view night", &mut || { let _ = n.view_night(); });
    time("glance", &mut || { let _ = n.view_glance(); });
    let mut j = 0;
    time("order", &mut || { let _ = n.order_as(&format!("u{}", j % fans), &things[j % things.len()], 1, true, 0, None); j += 1; });
    time("step", &mut || { n.step(); });
    println!("orders {}", k);
}

//! How honest are the arrival times? cargo run --release --example eta
use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night};
use agape_kernel::world::WorldDef;
fn main() {
    let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
    let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
    let mut n = Night::new(w, t, Config { seed: 7, fans: 3000, runners: 40, you_seat: None, ..Config::default() });
    while n.step() {}
    let mut e: Vec<(i64, i64)> = n.eta_errors();
    let mut late: Vec<i64> = e.iter().map(|x| x.1).collect(); late.sort();
    let pct = |v: &Vec<i64>, p: f64| v[((v.len() as f64 - 1.0) * p) as usize];
    println!("orders {}  late-by (s): p10 {} p50 {} p75 {} p90 {} p95 {}", late.len(), pct(&late, 0.1), pct(&late, 0.5), pct(&late, 0.75), pct(&late, 0.9), pct(&late, 0.95));
    for k in [1.0f64, 1.15, 1.3] { for c in [0i64, 60, 120, 180, 240] {
        let ok = e.iter().filter(|x| (x.0 + x.1) as f64 <= x.0 as f64 * k + c as f64 + 60.0).count() as f64 / e.len() as f64;
        let mut slack: Vec<i64> = e.iter().map(|x| (x.0 as f64 * k) as i64 + c - (x.0 + x.1)).collect(); slack.sort();
        println!("k {k} c {c}: on time {:.0}%  median early by {}s  shown median {}s", ok * 100.0, slack[slack.len() / 2], { let mut s: Vec<i64> = e.iter().map(|x| (x.0 as f64 * k) as i64 + c).collect(); s.sort(); s[s.len() / 2] });
    } }
    let ratio: Vec<f64> = e.iter_mut().filter(|x| x.0 > 0).map(|x| (x.0 + x.1) as f64 / x.0 as f64).collect();
    let mut r = ratio.clone(); r.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("actual/estimate: p50 {:.2} p90 {:.2}", r[r.len() / 2], r[r.len() * 9 / 10]);
}
#[allow(dead_code)]
fn unused() {}

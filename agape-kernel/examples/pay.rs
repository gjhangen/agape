//! How runners are paid: shift earnings, trip offers, and on-time, for each pay mode and rate.
use agape_kernel::rules::Template;
use agape_kernel::sim::{run, Config};
use agape_kernel::world::WorldDef;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).cloned().unwrap_or("offers".into());
    let rate: i64 = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(1800);
    let runners: usize = args.get(3).and_then(|x| x.parse().ok()).unwrap_or(40);
    for seed in [7u64, 11] {
        let mut w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
        w.terms.pay_mode = mode.clone();
        if mode == "fixed" { w.terms.runner_pay = rate; } else { w.terms.offer_hourly = rate; }
        if let Some(m) = args.get(4).and_then(|x| x.parse().ok()) { w.terms.offer_min = m; }
        let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
        let r = run(w, t, &Config { seed, fans: 3000, runners, ..Config::default() });
        println!("{mode} {rate} seed {seed}: shift ${:.2}/h  trips {} offers {} first-take {:.0}%  avg trip pay ${:.2}  batch {:.2}  late {} of {}  p50 {}m p90 {}m  agape ${:.0} proc ${:.0} stands ${:.0} conserved {}",
            r.runner_hourly as f64 / 100.0, r.trips_taken, r.trip_offers, 100.0 * r.taken_first as f64 / r.trips_taken.max(1) as f64, r.trip_pay_avg as f64 / 100.0, r.batch_avg, r.late, r.orders,
            r.wait_p50_s / 60, r.wait_p90_s / 60, r.money["agape net"] as f64 / 100.0, r.money["agape processing"] as f64 / 100.0, r.money["stands net"] as f64 / 100.0, r.conserved);
    }
}

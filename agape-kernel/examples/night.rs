//! cargo run --release --example night   — staffing sweep over one simulated game night.
use agape_kernel::rules::Template;
use agape_kernel::sim::{run, Config};
use agape_kernel::world::WorldDef;

fn main() {
    println!("runners  orders  balked  p50min  p90min  late  batch  queue_s  dispatch_s  runner$/h  agape$");
    for runners in [15, 20, 30, 40, 60] {
        let mut agg = (0u64, 0u64, 0i64, 0i64, 0u64, 0f64, 0i64, 0i64, 0i64, 0i64);
        let seeds = 5;
        for seed in 1..=seeds {
            let world: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
            let tpl: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
            let r = run(world, tpl, &Config { seed, runners, ..Config::default() });
            assert!(r.conserved && r.open_holds == 0);
            agg.0 += r.orders; agg.1 += r.balked; agg.2 += r.wait_p50_s; agg.3 += r.wait_p90_s; agg.4 += r.late;
            agg.5 += r.batch_avg; agg.6 += r.avg_queue_s; agg.7 += r.avg_dispatch_s; agg.8 += r.runner_hourly; agg.9 += r.money["agape net"];
        }
        let s = seeds as f64;
        println!("{:>7}  {:>6.0}  {:>6.0}  {:>6.1}  {:>6.1}  {:>4.0}  {:>5.2}  {:>7.0}  {:>10.0}  {:>9.2}  {:>6.0}",
            runners, agg.0 as f64 / s, agg.1 as f64 / s, agg.2 as f64 / s / 60.0, agg.3 as f64 / s / 60.0, agg.4 as f64 / s,
            agg.5 / s, agg.6 as f64 / s, agg.7 as f64 / s, agg.8 as f64 / s / 100.0, agg.9 as f64 / s / 100.0);
    }
}

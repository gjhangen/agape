//! cargo run --release --example modes — guaranteed vs while-it-lasts on the same nights.
use agape_kernel::rules::{Mode, Template};
use agape_kernel::sim::{run, Config};
use agape_kernel::world::WorldDef;

fn main() {
    println!("mode            urgency  offer_orders  cola_spend$  shown_combo  saw_end  menu_sold_out  p90min  agape$");
    for (mode, lift) in [(Mode::Guaranteed, 1.0), (Mode::WhileItLasts, 1.0), (Mode::WhileItLasts, 2.0)] {
        let seeds = 8u64;
        let mut a = [0f64; 7];
        for seed in 1..=seeds {
            let w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
            let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
            let r = run(w, t, &Config { seed, force_mode: Some(mode), urgency_lift: lift, ..Config::default() });
            assert!(r.conserved && r.open_holds == 0 && r.tap_failed_after_live_card == 0 && r.checkout_failed == 0);
            a[0] += r.offer_orders as f64;
            a[1] += r.sponsor_spend_ledger["sponsor_cola"] as f64 / 100.0;
            a[2] += r.attribution.get("halftime_combo").map_or(0, |s| s.shown) as f64;
            a[3] += r.saw_it_end as f64;
            a[4] += r.menu_sold_out as f64;
            a[5] += r.wait_p90_s as f64 / 60.0;
            a[6] += r.money["agape net"] as f64 / 100.0;
        }
        let s = seeds as f64;
        println!("{:<15} {:>7.1}  {:>12.0}  {:>11.0}  {:>11.0}  {:>7.0}  {:>13.0}  {:>6.1}  {:>6.0}",
            format!("{mode:?}"), lift, a[0]/s, a[1]/s, a[2]/s, a[3]/s, a[4]/s, a[5]/s, a[6]/s);
    }
}

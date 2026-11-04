use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night};
use agape_kernel::world::WorldDef;
fn main() {
    let l2 = vec!["points", "offers", "gifting", "priority", "upgrades", "colors"];
    let mut l3 = l2.clone(); l3.extend(["predictions", "battle", "cards"]);
    let mut l4 = l3.clone(); l4.extend(["tickets", "around"]);
    let mut l5 = l4.clone(); l5.extend(["chat", "players"]);
    for feats in [vec![], l2, l3, l4, l5] {
        let mut w: WorldDef = serde_json::from_str(include_str!("../templates/arena.json")).unwrap();
        for k in ["points", "offers", "gifting", "priority", "upgrades", "colors", "predictions", "battle", "cards", "tickets", "around", "chat", "players"] { w.features.insert(k.into(), feats.contains(&k)); }
        let t: Template = serde_json::from_str(include_str!("../templates/basketball.json")).unwrap();
        let mut n = Night::new(w, t, Config { seed: 23, you_seat: Some("s104".into()), ..Config::default() });
        for i in 0..900 { n.step(); if i % 50 == 0 { let _ = n.view_fan("you"); let _ = n.view_ops(); } }
        println!("ok {:?}", feats);
    }
}

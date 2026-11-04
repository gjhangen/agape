//! Agape kernel, layer 0: the nine generators and the invariants that hold between them.
//!
//! - `value`      Value, accounts, the exact ledger (conservation, A1, A6)
//! - `signal`     Signal log, schema, authority, per-person encryption (A4, A5, invariant 10)
//! - `place`      Place graph: Scope and Path
//! - `rules`      Rule language: measures, spans, patterns, effects (A2)
//! - `economy`    Offers, prices, settlement, the ex-ante Agape check (A3, A7)
//! - `commitment` Commitment lifecycle with owners and named exits (invariant 5)

pub mod signal;
pub mod value;
pub mod place;
pub mod rules;
pub mod economy;
pub mod commitment;
pub mod world;
pub mod engines;
pub mod league;
pub mod social;
pub mod services;
pub mod deals;
pub mod exchange;
pub mod dealing;
pub mod lenses;
pub mod offers;
pub mod points;
pub mod sim;
pub mod views;
pub mod care;
pub mod remedy;
pub mod setup;
pub mod passport;
pub mod api;
pub mod wasm;

/// Device entry point (WASM / mobile): the same settlement arithmetic the server posts.
/// Returns the fan's charge in cents for a one-line order under default terms.
#[unsafe(no_mangle)]
pub extern "C" fn agape_charge(subtotal: i64, discount: i64, tip: i64, delivery: i32) -> i64 {
    let q = economy::Quote {
        fan: "fan".into(), stand: "stand".into(), runner: None,
        lines: vec![economy::Line { thing: "item".into(), qty: 1, unit_price: subtotal }],
        discounts: if discount > 0 { vec![economy::Discount { amount: discount, payer: "payer".into(), reason: String::new() }] } else { vec![] },
        points_redeemed: 0, tip, delivery: delivery != 0, fee_waived: false, runner_pay: None };
    economy::breakdown(&q, &economy::Terms::default()).map(|b| b.charge).unwrap_or(-1)
}

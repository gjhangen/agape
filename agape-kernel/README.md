# agape-kernel

Layer 0 of Agape: the generators, the exact ledger, the signal log and the rule language.
Spec: the "Agape Substrate" doc (tabs: Substrate, Rule language, Generator specs, Coverage matrix).

```
cargo test --release            # 16 acceptance + 9 offer/points + 4 full-night tests
cargo run --release --example night   # staffing sweep
cargo run --release --example modes   # guaranteed vs while-it-lasts
cargo build --release --target wasm32-unknown-unknown --no-default-features   # device build (~24 KB)
```

| Module | Generators | Invariants and amendments it enforces |
|---|---|---|
| `value` | Value | conservation, A1 units of things, A6 integer cents with banker's rounding |
| `signal` | Signal, Party (roles) | append-only time, A5 authority (claims vs facts), A4 per-person keys |
| `place` | Place | Scope and Path, closures, step-free routes |
| `rules` | Span, Rule | A2 rules and spans as data; typed, total, incremental, stratified |
| `economy` | Offer, Thing | A3 ex-ante Agape check, A7 every discount has a payer |
| `commitment` | Commitment | defined exits with fault owners; stages move only by their owner; holds at acceptance |
| `world` (L1) | Place, Thing, Party | a venue built from one JSON file (`templates/arena.json`); eligibility before exposure |
| `offers` (L2) | Offer | guaranteed and while-it-lasts offers, claims on tap, live disclosure (A8) |
| `points` | Value | points never expire by default; switchable expiry with notice (A9) |
| `engines` (L2) | Offer, Commitment | offers shown only when stock, budget and kitchen can honour them; batching dispatch; attribution |
| `sim` (L3) | all | a full game night through every layer |

Templates live in `templates/`. `basketball.json` and `concert.json` run on the same kernel;
the kernel source contains no sport or show vocabulary (a test checks this).

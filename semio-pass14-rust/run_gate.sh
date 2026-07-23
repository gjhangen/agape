#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$(dirname "$0")"
mkdir -p "$ROOT/reports"
cargo test --all-targets
cargo run -- "$ROOT/fixtures/candidate" "$ROOT/fixtures/expected_hashes.json" "$ROOT/reports/rust_independent_reload_report.json"
python3 - "$ROOT/reports/rust_independent_reload_report.json" <<'PY'
import json,sys
path=sys.argv[1]
data=json.load(open(path))
assert data['fixtures']==64 and data['passed']==64 and data['all_passed'] is True,data
print('Rust isolated reduction gate: 64/64')
PY

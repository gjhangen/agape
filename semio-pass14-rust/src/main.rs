use semio_pass14_reloader::{digest_value, load_json, reload_candidate};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 4 {
        return Err("usage: semio-pass14-reloader CANDIDATE_DIR EXPECTED_HASHES OUTPUT".into());
    }
    let hashes = load_json(Path::new(&arguments[2]))?;
    let mut expected = BTreeMap::new();
    for entry in hashes["fixtures"].as_array().ok_or("fixtures")? {
        expected.insert(
            entry["id"].as_str().ok_or("id")?.to_string(),
            entry["canonical_digest"].as_str().ok_or("digest")?.to_string(),
        );
    }
    let mut entries = fs::read_dir(&arguments[1])
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut results = Vec::new();
    let mut passed = 0usize;
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let id = path.file_stem().and_then(|value| value.to_str()).ok_or("filename")?.to_string();
        let candidate = load_json(&path)?;
        let canonical = reload_candidate(&candidate)?;
        let observed = digest_value(&canonical);
        let expected_digest = expected.get(&id).ok_or("missing expected hash")?;
        let passed_fixture = &observed == expected_digest;
        if passed_fixture { passed += 1; }
        results.push(json!({"id": id, "passed": passed_fixture, "expected": expected_digest, "observed": observed}));
    }
    let output: Value = json!({
        "implementation": "independent_rust_isolated_reloader",
        "fixtures": results.len(),
        "passed": passed,
        "all_passed": passed == results.len(),
        "results": results
    });
    fs::write(&arguments[3], serde_json::to_vec_pretty(&output).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    println!("{passed}/{}", output["fixtures"]);
    if passed != output["fixtures"].as_u64().unwrap_or(0) as usize {
        return Err("parity failure".into());
    }
    Ok(())
}

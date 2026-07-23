use semio_pass14_reloader::{digest_value, load_json, reload_candidate};
use std::fs;
use std::path::PathBuf;

#[test]
fn all_64_fixtures_match() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has repository parent")
        .to_path_buf();
    let hashes = load_json(&root.join("fixtures/expected_hashes.json")).unwrap();
    let mut files = fs::read_dir(root.join("fixtures/candidate"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    files.sort();
    assert_eq!(files.len(), 64);
    for path in files {
        let id = path.file_stem().unwrap().to_str().unwrap();
        let expected = hashes["fixtures"]
            .as_array().unwrap()
            .iter()
            .find(|entry| entry["id"] == id).unwrap()["canonical_digest"]
            .as_str().unwrap();
        let candidate = load_json(&path).unwrap();
        let canonical = reload_candidate(&candidate).unwrap();
        assert_eq!(digest_value(&canonical), expected, "{id}");
    }
}

#[test]
fn rejects_duplicate_trace_authority_and_held_entity_merger() {
    let mut candidate = serde_json::json!({
        "schema_version": "semio-reduction-candidate-2.0",
        "canonical_schema_version": "semio-snapshot-1.0",
        "nodes": [], "quotes": [], "variables": [], "expressions": [], "records": [],
        "traces": []
    });
    assert!(reload_candidate(&candidate).is_err());
    candidate.as_object_mut().unwrap().remove("traces");
    candidate.as_object_mut().unwrap().insert("entities".into(), serde_json::json!([]));
    assert!(reload_candidate(&candidate).is_err());
}

#[test]
fn rejects_implicit_presence_shape() {
    let candidate = serde_json::json!({
        "schema_version": "semio-reduction-candidate-2.0",
        "canonical_schema_version": "semio-snapshot-1.0",
        "nodes": [], "quotes": [], "variables": [], "expressions": [],
        "records": [{
            "id": "record:1",
            "ordinal": 1,
            "scope": "scope:root",
            "payload": {"kind": "Presence"}
        }]
    });
    assert!(reload_candidate(&candidate).is_err());
}

//! Passports on the server: where each fan's points and level are between venues. A move is
//! written down before each step, so a crash mid-move is finished on restart by repeating the
//! step, which every venue treats as a no-op if it already happened.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Passport {
    pub points: i64,
    pub level: i64,
    pub venues: Vec<String>,
    pub nights: i64,
    /// The venue holding the fan's points right now (None: they're here, in the passport).
    pub at: Option<String>,
    /// A move in progress: (venue, transfer id, "out" or "in").
    pub pending: Option<(String, String, String)>,
    pub seq: u64,
}

pub struct Passports { path: PathBuf, pub all: BTreeMap<String, Passport> }

impl Passports {
    pub fn open(path: PathBuf) -> Passports {
        let all = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Passports { path, all }
    }
    /// Written whole and swapped in, so the file is never half-written.
    pub fn save(&self) {
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec(&self.all).unwrap()).expect("passport write");
        let f = std::fs::File::open(&tmp).unwrap(); f.sync_all().unwrap();
        std::fs::rename(&tmp, &self.path).expect("passport swap");
    }
}

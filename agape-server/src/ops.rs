//! Keeping Agape up: a copy of everything every day, a bundle the host can download and restore
//! from on any machine, and checks that say when something is wrong before people notice.

use base64::Engine;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Everything that can't be rebuilt: accounts, passports, Admin, leads, the open nights' journals,
/// past nights' reports, saved Studio versions and the mail keys.
fn kept_files(data: &Path) -> Vec<PathBuf> {
    let mut v = vec![];
    for f in ["accounts.jsonl", "passports.json", "admin.json", "leads.jsonl", "admin-trouble.jsonl", "mail-dkim.pem", "mail-inbound-secret"] {
        let p = data.join(f); if p.is_file() { v.push(p); }
    }
    for dir in ["venues", "nights", "studio"] {
        if let Ok(rd) = std::fs::read_dir(data.join(dir)) {
            for e in rd.flatten() {
                let p = e.path();
                let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                // the journal of every night still open, and every report; finished simulated nights are history nobody needs back
                let keep = match dir { "venues" => n.ends_with(".journal"), "nights" => n.ends_with(".json"), _ => n.ends_with(".json") };
                if keep && p.is_file() { v.push(p); }
            }
        }
    }
    v
}

fn day(secs: u64) -> String {
    // civil date from days since 1970 (Howard Hinnant's algorithm)
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097); let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", yoe + era * 400 + (m <= 2) as i64, m, d)
}

/// Today's copy, once a day; copies older than two weeks are removed.
pub fn snapshot(data: &Path) -> Result<Option<PathBuf>, String> {
    let root = data.join("backups");
    let now = crate::now_s();
    let dir = root.join(day(now));
    if dir.exists() { return Ok(None); }
    let tmp = root.join(format!(".{}", day(now)));
    let _ = std::fs::remove_dir_all(&tmp);
    for p in kept_files(data) {
        let rel = p.strip_prefix(data).map_err(|e| e.to_string())?;
        let to = tmp.join(rel);
        std::fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::copy(&p, &to).map_err(|e| format!("{}: {e}", rel.display()))?;
    }
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &dir).map_err(|e| e.to_string())?;
    if let Ok(rd) = std::fs::read_dir(&root) {
        let cutoff = day(now.saturating_sub(14 * 86_400));
        for e in rd.flatten() { let n = e.file_name().to_string_lossy().to_string(); if !n.starts_with('.') && n < cutoff { let _ = std::fs::remove_dir_all(e.path()); } }
    }
    Ok(Some(dir))
}

/// Everything, as one file the host downloads and keeps somewhere else.
pub fn bundle(data: &Path) -> Value {
    let b64 = base64::engine::general_purpose::STANDARD;
    let files: serde_json::Map<String, Value> = kept_files(data).into_iter().filter_map(|p| {
        let rel = p.strip_prefix(data).ok()?.to_string_lossy().to_string();
        Some((rel, json!(b64.encode(std::fs::read(&p).ok()?))))
    }).collect();
    json!({ "agape_backup": 1, "made": crate::now_s(), "files": files })
}

/// Put a bundle back into an empty data folder (a new machine, or after a loss).
pub fn restore(bundle: &Value, data: &Path) -> Result<usize, String> {
    if bundle["agape_backup"] != 1 { return Err("That isn't an Agape backup".into()); }
    if data.join("accounts.jsonl").exists() || data.join("admin.json").exists() { return Err(format!("{} already has data in it; restore into an empty folder", data.display())); }
    let b64 = base64::engine::general_purpose::STANDARD;
    let files = bundle["files"].as_object().ok_or("The backup has no files")?;
    for (rel, v) in files {
        if rel.contains("..") || rel.starts_with('/') { return Err(format!("Refusing a strange path: {rel}")); }
        let bytes = b64.decode(v.as_str().unwrap_or("")).map_err(|e| format!("{rel}: {e}"))?;
        let to = data.join(rel);
        std::fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(&to, bytes).map_err(|e| format!("{rel}: {e}"))?;
    }
    Ok(files.len())
}

/// Megabytes free where the data lives.
pub fn disk_free_mb(data: &Path) -> Option<u64> {
    let out = std::process::Command::new("df").arg("-Pm").arg(data).output().ok()?;
    String::from_utf8_lossy(&out.stdout).lines().nth(1)?.split_whitespace().nth(3)?.parse().ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn dates() { assert_eq!(super::day(0), "1970-01-01"); assert_eq!(super::day(1_762_732_800), "2025-11-10"); }
}

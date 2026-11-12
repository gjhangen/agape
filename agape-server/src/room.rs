//! The investor room: a private page of documents, links and notes for investors, each with their own link.
//! The host (or anyone whose role covers leads) decides what's in it and who can see it, and sees who opened what.
//! Kept in its own file (room.json) beside Agape's other data; uploaded files in room-files/, never public.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Item {
    pub id: String, pub title: String, #[serde(default)] pub about: String,
    /// "file" (a document), "link" (a page, opened through the room so the visit is counted) or "text" (written here)
    pub kind: String,
    #[serde(default)] pub url: String, #[serde(default)] pub file: String, #[serde(default)] pub fname: String, #[serde(default)] pub size: u64,
    #[serde(default)] pub text: String,
    /// Not shown to investors (a draft, or something to fill in before sharing).
    #[serde(default)] pub hidden: bool,
    pub at: u64,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Guest {
    pub id: String, pub name: String, #[serde(default)] pub firm: String, #[serde(default)] pub email: String,
    pub key: String, pub at: u64, #[serde(default)] pub off: bool,
    /// What they opened and when: (time, item id, or "room" for the room itself).
    #[serde(default)] pub views: Vec<(u64, String)>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Room { #[serde(default)] pub items: Vec<Item>, #[serde(default)] pub guests: Vec<Guest>, #[serde(default)] pub seeded: bool }

fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }
fn token(n: usize) -> String { use rand::RngCore; let mut b = vec![0u8; n]; rand::rng().fill_bytes(&mut b); b.iter().map(|x| format!("{x:02x}")).collect() }
pub fn files(dir: &Path) -> PathBuf { dir.join("room-files") }

/// What the room starts with: the summary, the long paper, the product, the Q&A, and the parts only the founders can
/// fill in (hidden until they do).
fn seed() -> Vec<Item> {
    let t = now();
    let it = |id: &str, title: &str, about: &str, kind: &str, url: &str, file: &str, fname: &str, text: &str, hidden: bool| Item { id: id.into(), title: title.into(), about: about.into(), kind: kind.into(), url: url.into(), file: file.into(), fname: fname.into(), size: 0, text: text.into(), hidden, at: t };
    vec![
        it("summary", "Agape in two pages", "The problem, the product, how the money moves, where it goes, and the founders.", "file", "", "static:agape-summary.pdf", "Agape summary.pdf", "", false),
        it("paper", "Where Agape goes", "The web of moves from food to the seat to local processing, with the assets each leaves behind, sized markets, comparable companies, the order of moves and the risks.", "file", "", "bundled:room-where-agape-goes.pdf", "Where Agape goes.pdf", "", false),
        it("tour", "The 2-minute tour", "A whole night at the demo arena, every role at once.", "link", "/demo/?tour=1", "", "", "", false),
        it("roles", "Try each role yourself", "Fan, runner, stand, venue, partner and Agape HQ, each as its own app.", "link", "/get/", "", "", "", false),
        it("qa", "Questions investors ask", "Straight answers, including what isn't done yet.", "text", "", "", "", include_str!("room_qa.md"), false),
        it("company", "The company", "Legal name, where it's incorporated, who owns what.", "text", "", "", "", "Add: legal name, state of incorporation and date, ownership (cap table), any notes or SAFEs outstanding.", true),
        it("round", "The round", "What you're raising and on what terms.", "text", "", "", "", "Add: amount, instrument (SAFE or priced), cap or valuation, minimum check, what the money is for and how long it lasts.", true),
        it("founders", "The founders", "Who's building Agape.", "text", "", "", "", "Phillip Morgan · Co-founder · Deals and operations\nGabriel Hangen · Co-founder · Technology: designed and built Agape's technology\n\nAdd anything worth knowing about each of you.", true),
    ]
}

impl Room {
    fn path(dir: &Path) -> PathBuf { dir.join("room.json") }
    /// Read, change and save the room under one lock, so two changes at once never undo each other.
    pub fn with<R>(dir: &Path, f: impl FnOnce(&mut Room) -> R) -> R {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut r: Room = std::fs::read(Self::path(dir)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        if !r.seeded { r.items = seed(); r.seeded = true; }
        let out = f(&mut r);
        let tmp = dir.join("room.json.tmp");
        if std::fs::write(&tmp, serde_json::to_vec_pretty(&r).unwrap_or_default()).is_ok() { let _ = std::fs::rename(&tmp, Self::path(dir)); }
        out
    }
    pub fn guest(&self, key: &str) -> Option<&Guest> { if key.len() < 20 { return None; } self.guests.iter().find(|g| g.key == key && !g.off) }

    /// What an investor sees: the visible items, in order, without anything private.
    pub fn view_for(&self, g: &Guest) -> Value {
        json!({ "guest": { "name": g.name, "firm": g.firm }, "items": self.items.iter().filter(|i| !i.hidden).map(|i| json!({
            "id": i.id, "title": i.title, "about": i.about, "kind": i.kind, "text": if i.kind == "text" { i.text.clone() } else { String::new() },
            "fname": i.fname, "size": i.size, "external": i.kind == "link" && i.url.starts_with("http") })).collect::<Vec<_>>() })
    }
    /// Note that a guest opened something. Returns true the first time they open the room at all.
    pub fn seen(&mut self, key: &str, what: &str) -> Option<(bool, String)> {
        let g = self.guests.iter_mut().find(|g| g.key == key && !g.off)?;
        let first = g.views.is_empty();
        let t = now();
        // the room itself counts once an hour, so a reload isn't a new visit
        if !(what == "room" && g.views.iter().rev().any(|v| v.1 == "room" && t - v.0 < 3600)) { g.views.push((t, what.to_string())); }
        if g.views.len() > 2000 { g.views.drain(0..500); }
        Some((first, if g.firm.is_empty() { g.name.clone() } else { format!("{} ({})", g.name, g.firm) }))
    }
    pub fn add_guest(&mut self, name: &str, firm: &str, email: &str) -> Result<Guest, String> {
        let name = name.trim().chars().take(80).collect::<String>();
        if name.is_empty() { return Err("Add who it's for".into()); }
        let g = Guest { id: format!("g{}", token(4)), name, firm: firm.trim().chars().take(80).collect(), email: email.trim().chars().take(120).collect(), key: token(16), at: now(), off: false, views: vec![] };
        self.guests.push(g.clone()); Ok(g)
    }
    pub fn save_item(&mut self, b: &Value) -> Result<(), String> {
        let s = |k: &str| b[k].as_str().unwrap_or("").to_string();
        let id = s("id");
        if b["delete"] == true { let n = self.items.len(); self.items.retain(|i| i.id != id); return if n == self.items.len() { Err("No such item".into()) } else { Ok(()) }; }
        if let Some(d) = b["move"].as_i64() {
            let i = self.items.iter().position(|x| x.id == id).ok_or("No such item")?;
            let j = (i as i64 + d).clamp(0, self.items.len() as i64 - 1) as usize;
            self.items.swap(i, j); return Ok(());
        }
        let title = s("title").trim().chars().take(120).collect::<String>();
        if title.is_empty() { return Err("Give it a title".into()); }
        let kind = s("kind");
        if !["link", "text", "file"].contains(&kind.as_str()) { return Err("Unknown kind".into()); }
        let url = s("url").trim().to_string();
        if kind == "link" && !(url.starts_with("https://") || url.starts_with("http://") || url.starts_with('/')) { return Err("A link starts with https:// (or / for a page on this site)".into()); }
        match self.items.iter_mut().find(|x| !id.is_empty() && x.id == id) {
            Some(x) => { x.title = title; x.about = s("about"); if kind != "file" { x.kind = kind; } x.url = url; x.text = s("text"); x.hidden = b["hidden"] == true; }
            None => { if kind == "file" { return Err("Upload the file".into()); } self.items.push(Item { id: format!("i{}", token(4)), title, about: s("about"), kind, url, file: String::new(), fname: String::new(), size: 0, text: s("text"), hidden: b["hidden"] == true, at: now() }); }
        }
        Ok(())
    }
}

/// Where an item's file is on disk: an uploaded one, one that ships with Agape (private), or a public site file.
pub fn file_path(dir: &Path, it: &Item) -> Option<PathBuf> {
    if let Some(n) = it.file.strip_prefix("bundled:") { return Some(PathBuf::from(std::env::var("AGAPE_PARTNER").unwrap_or_else(|_| "/partner".into())).join(n)); }
    if let Some(n) = it.file.strip_prefix("static:") { return Some(PathBuf::from(std::env::var("AGAPE_STATIC").unwrap_or_else(|_| "/app".into())).join(n)); }
    if it.file.is_empty() || it.file.contains('/') || it.file.contains("..") { return None; }
    Some(files(dir).join(&it.file))
}

pub fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("").to_lowercase().as_str() {
        "pdf" => "application/pdf", "png" => "image/png", "jpg" | "jpeg" => "image/jpeg", "webp" => "image/webp", "gif" => "image/gif",
        "csv" => "text/csv; charset=utf-8", "txt" | "md" => "text/plain; charset=utf-8",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation", "zip" => "application/zip", "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

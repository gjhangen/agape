//! Agape Admin: the company's own accounts, and the payment-processing profile of every merchant
//! Agape sells for, from which the VAR sheet is printed.
//!
//! Separate from fan and staff accounts on purpose: these people run the company, so they sign in
//! with an email and a password of their own, are let in by invitation only, and every change they
//! make is written to an activity log. The first owner is created with a one-time setup code that
//! the server writes to its data folder, so only someone with access to the server can claim it.
//!
//! What is never stored here: card numbers, bank account numbers, or processor API secrets.
//! Passwords are Argon2 hashes; session and invitation tokens are kept only as SHA-256 hashes.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;

const SESSION_SECS: u64 = 7 * 24 * 3600;
const INVITE_SECS: u64 = 7 * 24 * 3600;
const MAX_FAILS: u32 = 8;
const LOCK_SECS: u64 = 15 * 60;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    #[serde(alias = "viewer")] Member,
    Manager,
    #[serde(alias = "editor")] Admin,
    #[serde(alias = "owner")] Host,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct User { pub id: String, pub email: String, pub name: String, hash: String, pub level: Level, pub at: u64, #[serde(default)] pub off: bool,
    /// Who they report to. Everyone but the host reports to someone, and sees nothing above them.
    #[serde(default)] pub boss: Option<String>, #[serde(default)] pub title: String }

#[derive(Clone, Serialize, Deserialize)]
struct Invite { hash: String, email: String, name: String, level: Level, by: String, until: u64, #[serde(default)] boss: String, #[serde(default)] title: String }

#[derive(Clone, Serialize, Deserialize)]
struct Session { user: String, until: u64 }

#[derive(Clone, Serialize, Deserialize)]
pub struct Merchant { pub id: String, pub fields: BTreeMap<String, String>, pub status: String, pub at: u64, pub by: String }

#[derive(Clone, Serialize, Deserialize)]
pub struct Entry { pub at: u64, pub who: String, pub what: String, #[serde(default)] pub actor: String }

/// A message between people on Agape's team, or a notice Agape sent one of them.
/// Every communication is kept, and is visible to the people in it and to everyone they report up to.
#[derive(Clone, Serialize, Deserialize)]
pub struct Note { pub id: String, pub at: u64, pub from: String, pub to: Vec<String>, pub subject: String, pub body: String, #[serde(default)] pub kind: String, #[serde(default)] pub read: Vec<String> }

#[derive(Default, Serialize, Deserialize)]
struct State {
    users: Vec<User>,
    #[serde(default)] invites: Vec<Invite>,
    #[serde(default)] sessions: BTreeMap<String, Session>,
    #[serde(default)] merchants: Vec<Merchant>,
    #[serde(default)] log: Vec<Entry>,
    #[serde(default)] notes: Vec<Note>,
    #[serde(default)] setup_hash: String,
}

pub struct Admin { path: PathBuf, s: State, fails: BTreeMap<String, (u32, u64)> }

pub fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }
fn sha(s: &str) -> String { Sha256::digest(s.as_bytes()).iter().map(|b| format!("{b:02x}")).collect() }
fn token(n: usize) -> String { let mut b = vec![0u8; n]; rand::rng().fill_bytes(&mut b); b.iter().map(|x| format!("{x:02x}")).collect() }
fn hash_pw(p: &str) -> Result<String, String> {
    let mut raw = [0u8; 16];
    rand::rng().fill_bytes(&mut raw);
    let salt = SaltString::encode_b64(&raw).map_err(|e| e.to_string())?;
    Ok(Argon2::default().hash_password(p.as_bytes(), &salt).map_err(|e| e.to_string())?.to_string())
}
fn check_pw(p: &str, h: &str) -> bool { PasswordHash::new(h).map_or(false, |x| Argon2::default().verify_password(p.as_bytes(), &x).is_ok()) }
fn good_email(e: &str) -> bool { let p: Vec<&str> = e.split('@').collect(); p.len() == 2 && !p[0].is_empty() && p[1].contains('.') && !p[1].starts_with('.') && !p[1].ends_with('.') && !e.contains(char::is_whitespace) && e.len() <= 120 }
fn good_pw(p: &str) -> Result<(), String> {
    if p.chars().count() < 10 { return Err("Use a password of at least 10 characters".into()); }
    if p.chars().count() > 200 { return Err("That password is too long".into()); }
    Ok(())
}

/// A run of 13 to 19 digits that passes the Luhn check looks like a card number.
fn looks_like_card(s: &str) -> bool {
    let digits: String = s.chars().filter(|c| !matches!(c, ' ' | '-')).collect();
    for run in digits.split(|c: char| !c.is_ascii_digit()) {
        if (13..=19).contains(&run.len()) {
            let sum: u32 = run.chars().rev().enumerate().map(|(i, c)| { let d = c.to_digit(10).unwrap(); if i % 2 == 1 { let x = d * 2; if x > 9 { x - 9 } else { x } } else { d } }).sum();
            if sum % 10 == 0 { return true; }
        }
    }
    false
}

/// The numbers a processor issues. They may be long and are checked for shape, not for Luhn.
const ISSUED: &[&str] = &["mid", "tid", "v_number", "bin", "agent", "chain", "store", "terminal", "location", "amex_se", "discover_mid", "mvv", "mcc", "zip", "phone", "cs_phone", "contact_phone", "currency", "country", "tax_id"];
/// Fields that must never be filled: Agape doesn't keep them.
const NEVER: &[&str] = &["card_number", "pan", "cvv", "bank_account", "account_number", "routing_number", "api_key", "api_secret", "password", "ssn"];

impl Admin {
    pub fn open(dir: PathBuf) -> Admin {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("admin.json");
        let s: State = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let mut a = Admin { path, s, fails: BTreeMap::new() };
        if a.s.users.iter().all(|u| u.level != Level::Host || u.off) {
            // no owner yet: a fresh one-time setup code, readable only on the server itself
            let code = token(9);
            a.s.setup_hash = sha(&code);
            let file = dir.join("admin-setup-code");
            let _ = std::fs::write(&file, format!("{code}\n"));
            #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)); }
            println!("Agape Admin has no owner yet. The setup code is in {}", file.display());
            a.save();
        }
        a
    }

    fn save(&self) {
        let tmp = self.path.with_extension("json.tmp");
        if std::fs::write(&tmp, serde_json::to_vec_pretty(&self.s).unwrap()).is_ok() {
            #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)); }
            let _ = std::fs::rename(&tmp, &self.path);
        }
    }

    fn note(&mut self, who: &User, what: String) {
        self.s.log.push(Entry { at: now(), who: who.name.clone(), what, actor: who.id.clone() });
        if self.s.log.len() > 5000 { self.s.log.drain(0..1000); }
    }

    pub fn setup_needed(&self) -> bool { !self.s.setup_hash.is_empty() }

    fn add_user(&mut self, email: &str, name: &str, pw: &str, level: Level) -> Result<User, String> {
        let email = email.trim().to_lowercase();
        let name = name.trim().chars().take(80).collect::<String>();
        if !good_email(&email) { return Err("Enter a valid email address".into()); }
        if name.is_empty() { return Err("Add your name".into()); }
        good_pw(pw)?;
        if self.s.users.iter().any(|u| u.email == email) { return Err("There's already an account with that email".into()); }
        let u = User { id: format!("a{}", self.s.users.len() + 1), email, name, hash: hash_pw(pw)?, level, at: now(), off: false, boss: None, title: String::new() };
        self.s.users.push(u.clone());
        Ok(u)
    }

    fn start_session(&mut self, user: &str) -> String {
        let t = token(32);
        let n = now();
        self.s.sessions.retain(|_, s| s.until > n);
        self.s.sessions.insert(sha(&t), Session { user: user.into(), until: n + SESSION_SECS });
        t
    }

    /// The first owner, with the code from the server's data folder.
    pub fn setup(&mut self, code: &str, email: &str, name: &str, pw: &str) -> Result<(String, User), String> {
        if !self.setup_needed() { return Err("Agape Admin is already set up. Sign in instead.".into()); }
        if sha(code.trim()) != self.s.setup_hash { return Err("That setup code isn't right".into()); }
        let u = self.add_user(email, name, pw, Level::Host)?;
        self.s.setup_hash.clear();
        self.note(&u.clone(), "Set up Agape Admin as its first owner".into());
        let t = self.start_session(&u.id);
        self.save();
        Ok((t, u))
    }

    pub fn sign_in(&mut self, email: &str, pw: &str) -> Result<(String, User), String> {
        let email = email.trim().to_lowercase();
        let n = now();
        if let Some((k, until)) = self.fails.get(&email) { if *k >= MAX_FAILS && *until > n { return Err("Too many tries. Wait 15 minutes and try again.".into()); } }
        let found = self.s.users.iter().find(|u| u.email == email && !u.off).cloned();
        match found {
            Some(u) if check_pw(pw, &u.hash) => {
                self.fails.remove(&email);
                let t = self.start_session(&u.id);
                self.save();
                Ok((t, u))
            }
            _ => {
                let e = self.fails.entry(email).or_insert((0, 0));
                if e.1 <= n { *e = (0, n + LOCK_SECS); }
                e.0 += 1;
                Err("That email and password don't match".into())
            }
        }
    }

    pub fn sign_out(&mut self, t: &str) { if self.s.sessions.remove(&sha(t)).is_some() { self.save(); } }

    pub fn who(&self, t: &str) -> Option<User> {
        let s = self.s.sessions.get(&sha(t))?;
        if s.until <= now() { return None; }
        self.s.users.iter().find(|u| u.id == s.user && !u.off).cloned()
    }

    pub fn invite(&mut self, by: &User, email: &str, name: &str, level: Level, boss: &str, title: &str) -> Result<String, String> {
        if by.level == Level::Member { return Err("Ask your manager to invite people".into()); }
        if level >= by.level { return Err("You can only invite people to a level below your own".into()); }
        let boss = if boss.is_empty() { by.id.clone() } else { boss.to_string() };
        if boss != by.id && !self.below(&by.id).contains(&boss) { return Err("They have to report to you or to someone on your team".into()); }
        let bl = self.s.users.iter().find(|u| u.id == boss).map(|u| u.level).ok_or("No such person to report to")?;
        if level >= bl { return Err("They have to report to someone at a higher level than theirs".into()); }
        let email = email.trim().to_lowercase();
        if !good_email(&email) { return Err("Enter a valid email address".into()); }
        if self.s.users.iter().any(|u| u.email == email) { return Err("They already have an account".into()); }
        let code = token(18);
        self.s.invites.retain(|i| i.email != email && i.until > now());
        self.s.invites.push(Invite { hash: sha(&code), email: email.clone(), name: name.trim().chars().take(80).collect(), level, by: by.id.clone(), until: now() + INVITE_SECS, boss, title: title.trim().chars().take(60).collect() });
        self.note(&by.clone(), format!("Invited {email} as {}", level_name(level)));
        self.save();
        Ok(code)
    }

    pub fn invite_info(&self, code: &str) -> Option<Value> {
        let h = sha(code.trim());
        self.s.invites.iter().find(|i| i.hash == h && i.until > now()).map(|i| json!({ "email": i.email, "name": i.name, "level": i.level }))
    }

    pub fn accept(&mut self, code: &str, name: &str, pw: &str) -> Result<(String, User), String> {
        let h = sha(code.trim());
        let i = self.s.invites.iter().find(|i| i.hash == h && i.until > now()).cloned().ok_or("This invitation has expired or was already used. Ask for a new one.")?;
        let mut u = self.add_user(&i.email, if name.trim().is_empty() { &i.name } else { name }, pw, i.level)?;
        let boss = if i.boss.is_empty() { i.by.clone() } else { i.boss.clone() };
        if let Some(x) = self.s.users.iter_mut().find(|x| x.id == u.id) { x.boss = Some(boss.clone()); x.title = i.title.clone(); u = x.clone(); }
        self.s.invites.retain(|x| x.hash != h);
        self.notify(&[boss.as_str(), i.by.as_str()], "joined", &format!("{} joined Agape Admin", u.name), &format!("{} accepted the invitation and now reports to {}.", u.name, self.name_of(&boss)));
        self.note(&u.clone(), "Accepted an invitation and joined".into());
        let t = self.start_session(&u.id);
        self.save();
        Ok((t, u))
    }

    pub fn cancel_invite(&mut self, by: &User, email: &str) -> Result<(), String> {
        let email = email.trim().to_lowercase();
        let mine = self.below(&by.id);
        if !self.s.invites.iter().any(|i| i.email == email && (i.by == by.id || mine.contains(&i.by) || by.level == Level::Host)) { return Err("No invitation of yours for that email".into()); }
        let before = self.s.invites.len();
        self.s.invites.retain(|i| i.email != email);
        if self.s.invites.len() == before { return Err("No invitation for that email".into()); }
        self.note(&by.clone(), format!("Cancelled the invitation for {email}"));
        self.save();
        Ok(())
    }

    pub fn set_user(&mut self, by: &User, id: &str, level: Option<Level>, off: Option<bool>, boss: Option<&str>, title: Option<&str>) -> Result<(), String> {
        if id == by.id { return Err("You can't change your own access. Ask the person you report to.".into()); }
        let mine = self.below(&by.id);
        if !mine.contains(&id.to_string()) { return Err("You can only change people who report up to you".into()); }
        let i = self.s.users.iter().position(|u| u.id == id).ok_or("No such person")?;
        if let Some(l) = level { if l >= by.level { return Err("You can only give a level below your own".into()); } }
        if let Some(b) = boss {
            if b == id || self.below(id).contains(&b.to_string()) { return Err("Someone can't report to themselves or to their own team".into()); }
            if b != by.id && !mine.contains(&b.to_string()) { return Err("They have to report to you or to someone on your team".into()); }
        }
        let new_level = level.unwrap_or(self.s.users[i].level);
        let new_boss = boss.map(String::from).or(self.s.users[i].boss.clone()).unwrap_or_default();
        let bl = self.s.users.iter().find(|u| u.id == new_boss).map(|u| u.level).unwrap_or(Level::Host);
        if new_level >= bl { return Err("They have to report to someone at a higher level than theirs".into()); }
        if let Some(l) = level { self.s.users[i].level = l; }
        if let Some(b) = boss { self.s.users[i].boss = Some(b.to_string()); }
        if let Some(t) = title { self.s.users[i].title = t.trim().chars().take(60).collect(); }
        if let Some(o) = off { self.s.users[i].off = o; if o { let uid = id.to_string(); self.s.sessions.retain(|_, s| s.user != uid); } }
        let u = self.s.users[i].clone();
        self.note(&by.clone(), format!("{} {} ({}, reports to {})", if u.off { "Turned off access for" } else { "Set access for" }, u.email, level_name(u.level), self.name_of(u.boss.as_deref().unwrap_or(""))));
        self.save();
        Ok(())
    }

    pub fn change_password(&mut self, by: &User, old: &str, new: &str) -> Result<(), String> {
        let i = self.s.users.iter().position(|u| u.id == by.id).ok_or("No such person")?;
        if !check_pw(old, &self.s.users[i].hash) { return Err("Your current password isn't right".into()); }
        good_pw(new)?;
        self.s.users[i].hash = hash_pw(new)?;
        let uid = by.id.clone();
        self.s.sessions.retain(|_, s| s.user != uid);
        self.note(&by.clone(), "Changed their password".into());
        self.save();
        Ok(())
    }

    /// Everyone who reports up to `id`, at any depth (not `id` itself).
    pub fn below(&self, id: &str) -> Vec<String> {
        let mut out: Vec<String> = vec![];
        let mut frontier = vec![id.to_string()];
        while let Some(x) = frontier.pop() {
            for u in self.s.users.iter().filter(|u| u.boss.as_deref() == Some(x.as_str())) {
                if u.id != id && !out.contains(&u.id) { out.push(u.id.clone()); frontier.push(u.id.clone()); }
            }
        }
        out
    }
    /// The chain above `id`, nearest first.
    pub fn above(&self, id: &str) -> Vec<String> {
        let mut out = vec![];
        let mut cur = self.s.users.iter().find(|u| u.id == id).and_then(|u| u.boss.clone());
        while let Some(b) = cur { if out.contains(&b) { break; } out.push(b.clone()); cur = self.s.users.iter().find(|u| u.id == b).and_then(|u| u.boss.clone()); }
        out
    }
    fn name_of(&self, id: &str) -> String { self.s.users.iter().find(|u| u.id == id).map(|u| u.name.clone()).unwrap_or_else(|| "nobody".into()) }
    /// The one rule: you see what you're part of, and everything made by anyone who reports up to you.
    fn sees(&self, viewer: &User, maker: &str, parties: &[String]) -> bool {
        if viewer.level == Level::Host { return true; }
        if maker == viewer.id || parties.contains(&viewer.id) { return true; }
        self.below(&viewer.id).iter().any(|b| b == maker || (maker.is_empty() && parties.contains(b)))
    }

    /// Who the viewer can see: themselves, the people they report to (names only), and their whole team.
    pub fn users(&self, viewer: &User) -> Value {
        let mine = self.below(&viewer.id);
        let up = self.above(&viewer.id);
        let shown = |u: &&User| viewer.level == Level::Host || u.id == viewer.id || mine.contains(&u.id);
        json!({
            "users": self.s.users.iter().filter(shown).map(|u| json!({ "id": u.id, "email": u.email, "name": u.name, "level": u.level, "at": u.at, "off": u.off, "boss": u.boss, "title": u.title })).collect::<Vec<_>>(),
            "chain": up.iter().filter_map(|id| self.s.users.iter().find(|u| &u.id == id)).map(|u| json!({ "id": u.id, "name": u.name, "level": u.level, "title": u.title })).collect::<Vec<_>>(),
            "invites": self.s.invites.iter().filter(|i| i.until > now() && (viewer.level == Level::Host || i.by == viewer.id || mine.contains(&i.by))).map(|i| json!({ "email": i.email, "name": i.name, "level": i.level, "until": i.until, "boss": i.boss })).collect::<Vec<_>>(),
            "contacts": self.s.users.iter().filter(|u| !u.off && u.id != viewer.id).map(|u| json!({ "id": u.id, "name": u.name, "title": u.title })).collect::<Vec<_>>(),
        })
    }

    /// A notice from Agape to some people on the team. Kept like any other communication.
    pub fn notify(&mut self, to: &[&str], kind: &str, subject: &str, body: &str) {
        let mut ids: Vec<String> = vec![];
        for t in to { if !t.is_empty() && !ids.iter().any(|x| x == t) && self.s.users.iter().any(|u| u.id == *t) { ids.push(t.to_string()); } }
        if ids.is_empty() { return; }
        self.s.notes.push(Note { id: format!("n{}", token(5)), at: now(), from: String::new(), to: ids, subject: subject.into(), body: body.into(), kind: kind.into(), read: vec![] });
        if self.s.notes.len() > 20000 { self.s.notes.drain(0..2000); }
    }
    /// The people at or above a level: who hears about company-wide things like a new lead.
    pub fn at_least(&self, l: Level) -> Vec<String> { self.s.users.iter().filter(|u| !u.off && u.level >= l).map(|u| u.id.clone()).collect() }
    pub fn notify_level(&mut self, l: Level, kind: &str, subject: &str, body: &str) {
        let ids = self.at_least(l);
        let refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
        self.notify(&refs, kind, subject, body);
        self.save();
    }

    pub fn send(&mut self, by: &User, to: &[String], subject: &str, body: &str) -> Result<(), String> {
        let subject = subject.trim().chars().take(140).collect::<String>();
        let body = body.trim().chars().take(8000).collect::<String>();
        if body.is_empty() { return Err("Write a message first".into()); }
        if looks_like_card(&body) || looks_like_card(&subject) { return Err("That looks like a card number. Never send card numbers, even here.".into()); }
        let mut ids: Vec<String> = vec![];
        for t in to { if t != &by.id && self.s.users.iter().any(|u| &u.id == t && !u.off) && !ids.contains(t) { ids.push(t.clone()); } }
        if ids.is_empty() { return Err("Choose who it's for".into()); }
        self.s.notes.push(Note { id: format!("n{}", token(5)), at: now(), from: by.id.clone(), to: ids, subject: if subject.is_empty() { "(no subject)".into() } else { subject }, body, kind: "message".into(), read: vec![by.id.clone()] });
        self.save();
        Ok(())
    }

    /// Every communication the viewer may see, newest first, marked by how they see it.
    pub fn inbox(&self, viewer: &User) -> Value {
        let name = |id: &str| if id.is_empty() { "Agape".to_string() } else { self.name_of(id) };
        let list: Vec<Value> = self.s.notes.iter().rev().filter(|n| self.sees(viewer, &n.from, &n.to)).take(500).map(|n| {
            let mine = n.from == viewer.id || n.to.contains(&viewer.id);
            json!({ "id": n.id, "at": n.at, "from": n.from, "from_name": name(&n.from), "to": n.to, "to_names": n.to.iter().map(|t| name(t)).collect::<Vec<_>>(),
                    "subject": n.subject, "body": n.body, "kind": n.kind, "mine": mine, "sent": n.from == viewer.id, "unread": n.to.contains(&viewer.id) && !n.read.contains(&viewer.id) })
        }).collect();
        json!({ "notes": list })
    }
    pub fn mark_read(&mut self, viewer: &User, ids: &[String]) {
        for n in self.s.notes.iter_mut().filter(|n| ids.contains(&n.id) && n.to.contains(&viewer.id)) { if !n.read.contains(&viewer.id) { n.read.push(viewer.id.clone()); } }
        self.save();
    }
    pub fn unread(&self, viewer: &User) -> usize { self.s.notes.iter().filter(|n| n.to.contains(&viewer.id) && !n.read.contains(&viewer.id)).count() }

    pub fn merchants(&self) -> &Vec<Merchant> { &self.s.merchants }

    pub fn save_merchant(&mut self, by: &User, id: Option<&str>, fields: &serde_json::Map<String, Value>, status: &str) -> Result<Merchant, String> {
        if by.level < Level::Admin { return Err("You can view, but not change, merchant details".into()); }
        if fields.len() > 120 { return Err("Too many fields".into()); }
        let mut clean = BTreeMap::new();
        for (k, v) in fields {
            if k.is_empty() || k.len() > 40 || !k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') { return Err(format!("Unknown field {k}")); }
            if NEVER.contains(&k.as_str()) { return Err("Agape doesn't keep card numbers, bank account numbers or processor secrets. Leave those with the processor.".into()); }
            let v = v.as_str().unwrap_or("").trim().to_string();
            if v.chars().count() > 600 { return Err(format!("{k} is too long")); }
            if !ISSUED.contains(&k.as_str()) && looks_like_card(&v) { return Err("That looks like a card number. Agape never keeps card numbers.".into()); }
            if ISSUED.contains(&k.as_str()) && !v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '+' | '(' | ')' | '.')) { return Err(format!("{} should be letters and numbers only", k.to_uppercase().replace('_', " "))); }
            if !v.is_empty() { clean.insert(k.clone(), v); }
        }
        let name = clean.get("dba").cloned().unwrap_or_default();
        if name.is_empty() { return Err("Add the merchant's DBA name (the name customers see)".into()); }
        let status = match status { "draft" | "requested" | "issued" | "live" => status.to_string(), _ => "draft".into() };
        let n = now();
        let m = match id.and_then(|id| self.s.merchants.iter().position(|m| m.id == id)) {
            Some(i) => {
                let changed: Vec<String> = clean.keys().chain(self.s.merchants[i].fields.keys()).filter(|k| clean.get(*k) != self.s.merchants[i].fields.get(*k)).cloned().collect::<std::collections::BTreeSet<_>>().into_iter().collect();
                let was = self.s.merchants[i].status.clone();
                self.s.merchants[i] = Merchant { id: self.s.merchants[i].id.clone(), fields: clean, status: status.clone(), at: n, by: by.name.clone() };
                let mut what = format!("Updated {name}");
                if !changed.is_empty() { what += &format!(": {}", changed.iter().take(8).map(|k| k.replace('_', " ")).collect::<Vec<_>>().join(", ")); if changed.len() > 8 { what += &format!(" and {} more", changed.len() - 8); } }
                if was != status { what += &format!(" · status {was} → {status}"); }
                self.note(&by.clone(), what);
                self.s.merchants[i].clone()
            }
            None => {
                let m = Merchant { id: format!("m{}", token(4)), fields: clean, status, at: n, by: by.name.clone() };
                self.s.merchants.push(m.clone());
                self.note(&by.clone(), format!("Added merchant {name}"));
                m
            }
        };
        self.save();
        Ok(m)
    }

    pub fn delete_merchant(&mut self, by: &User, id: &str) -> Result<(), String> {
        if by.level != Level::Host { return Err("Only the host can delete a merchant".into()); }
        let i = self.s.merchants.iter().position(|m| m.id == id).ok_or("No such merchant")?;
        let m = self.s.merchants.remove(i);
        self.note(&by.clone(), format!("Deleted merchant {}", m.fields.get("dba").cloned().unwrap_or_default()));
        self.save();
        Ok(())
    }

    /// Someone printed or downloaded a VAR sheet: worth knowing who and when.
    pub fn printed(&mut self, by: &User, id: &str) {
        if let Some(m) = self.s.merchants.iter().find(|m| m.id == id) {
            let name = m.fields.get("dba").cloned().unwrap_or_default();
            self.note(&by.clone(), format!("Printed the VAR sheet for {name}"));
            self.save();
        }
    }

    pub fn log(&self, viewer: &User) -> Vec<Entry> {
        let mine = self.below(&viewer.id);
        self.s.log.iter().rev().filter(|e| viewer.level == Level::Host || e.actor == viewer.id || mine.contains(&e.actor)).take(300).cloned().collect()
    }
}

pub fn level_name(l: Level) -> &'static str { match l { Level::Host => "host", Level::Admin => "admin", Level::Manager => "manager", Level::Member => "team member" } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn card_numbers_are_refused_but_issued_numbers_are_kept() {
        let d = tempfile::tempdir().unwrap();
        let mut a = Admin::open(d.path().to_path_buf());
        let code = std::fs::read_to_string(d.path().join("admin-setup-code")).unwrap();
        assert!(a.setup("wrong", "g@x.com", "G", "longenough1").is_err());
        let (_, owner) = a.setup(&code, "G@x.com", "Gabriel", "longenough1").unwrap();
        assert!(!a.setup_needed());
        let mut f = serde_json::Map::new();
        f.insert("dba".into(), json!("Harbor Arena"));
        f.insert("mid".into(), json!("4111111111111111"));
        assert!(a.save_merchant(&owner, None, &f, "draft").is_ok());
        f.insert("notes".into(), json!("test card 4111 1111 1111 1111"));
        assert!(a.save_merchant(&owner, None, &f, "draft").is_err());
        f.remove("notes");
        f.insert("bank_account".into(), json!("123"));
        assert!(a.save_merchant(&owner, None, &f, "draft").is_err());
    }
    #[test]
    fn invitations_sessions_and_lockout() {
        let d = tempfile::tempdir().unwrap();
        let mut a = Admin::open(d.path().to_path_buf());
        let code = std::fs::read_to_string(d.path().join("admin-setup-code")).unwrap();
        let (t, owner) = a.setup(&code, "g@x.com", "Gabriel", "longenough1").unwrap();
        assert_eq!(a.who(&t).unwrap().id, owner.id);
        let inv = a.invite(&owner, "p@y.com", "Partner", Level::Admin, "", "").unwrap();
        assert!(a.invite_info(&inv).is_some());
        let (t2, p) = a.accept(&inv, "", "anotherlong1").unwrap();
        assert_eq!(p.level, Level::Admin);
        assert!(a.accept(&inv, "", "anotherlong1").is_err());
        assert!(a.invite(&p, "z@z.com", "Z", Level::Host, "", "").is_err());
        // reopened from disk, sessions still work and the setup code is gone
        drop(a);
        let mut a = Admin::open(d.path().to_path_buf());
        assert!(a.who(&t2).is_some());
        assert!(!a.setup_needed());
        for _ in 0..MAX_FAILS { let _ = a.sign_in("p@y.com", "wrongwrong"); }
        assert!(a.sign_in("p@y.com", "anotherlong1").is_err());
        a.set_user(&owner, &p.id, None, Some(true), None, None).unwrap();
        assert!(a.who(&t2).is_none());
    }

    #[test]
    fn everyone_sees_down_never_up() {
        let d = tempfile::tempdir().unwrap();
        let mut a = Admin::open(d.path().to_path_buf());
        let code = std::fs::read_to_string(d.path().join("admin-setup-code")).unwrap();
        let (_, host) = a.setup(&code, "g@x.com", "Gabriel", "longenough1").unwrap();
        let join = |a: &mut Admin, by: &User, e: &str, l: Level| { let c = a.invite(by, e, e, l, "", "").unwrap(); a.accept(&c, "", "longenough1").unwrap().1 };
        let admin = join(&mut a, &host, "admin@x.com", Level::Admin);
        let mgr = join(&mut a, &admin, "mgr@x.com", Level::Manager);
        let mem = join(&mut a, &mgr, "mem@x.com", Level::Member);
        let other = join(&mut a, &admin, "other@x.com", Level::Manager);
        assert!(a.invite(&mgr, "x@x.com", "x", Level::Manager, "", "").is_err(), "no one invites at their own level");
        a.send(&host, &[admin.id.clone()], "From the host", "private to admin").unwrap();
        a.send(&mem, &[mgr.id.clone()], "From the member", "hello").unwrap();
        a.send(&other, &[admin.id.clone()], "From the other manager", "sideways").unwrap();
        let subjects = |a: &Admin, u: &User| a.inbox(u)["notes"].as_array().unwrap().iter().filter(|n| n["kind"] == "message").map(|n| n["subject"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(subjects(&a, &host).len(), 3, "the host sees everything");
        assert!(subjects(&a, &admin).contains(&"From the host".to_string()), "addressed to them");
        assert!(subjects(&a, &admin).contains(&"From the member".to_string()), "their team, two levels down");
        let m = subjects(&a, &mgr);
        assert!(m.contains(&"From the member".to_string()) && !m.contains(&"From the other manager".to_string()) && !m.contains(&"From the host".to_string()), "nothing sideways or above");
        assert_eq!(subjects(&a, &other), vec!["From the other manager".to_string()]);
        // the host's own actions never show in an admin's activity
        assert!(a.log(&admin).iter().all(|e| e.actor != host.id));
        assert!(a.log(&host).iter().any(|e| e.actor == host.id));
        // a manager can't move someone outside their team or promote to their own level
        assert!(a.set_user(&mgr, &other.id, Some(Level::Member), None, None, None).is_err());
        assert!(a.set_user(&mgr, &mem.id, Some(Level::Manager), None, None, None).is_err());
        assert!(a.set_user(&admin, &mgr.id, None, None, Some(&mem.id), None).is_err(), "no cycles");
        assert!(a.users(&mgr)["users"].as_array().unwrap().len() == 2);
    }
}

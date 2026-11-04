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
pub enum Level { Viewer, Editor, Owner }

#[derive(Clone, Serialize, Deserialize)]
pub struct User { pub id: String, pub email: String, pub name: String, hash: String, pub level: Level, pub at: u64, #[serde(default)] pub off: bool }

#[derive(Clone, Serialize, Deserialize)]
struct Invite { hash: String, email: String, name: String, level: Level, by: String, until: u64 }

#[derive(Clone, Serialize, Deserialize)]
struct Session { user: String, until: u64 }

#[derive(Clone, Serialize, Deserialize)]
pub struct Merchant { pub id: String, pub fields: BTreeMap<String, String>, pub status: String, pub at: u64, pub by: String }

#[derive(Clone, Serialize, Deserialize)]
pub struct Entry { pub at: u64, pub who: String, pub what: String }

#[derive(Default, Serialize, Deserialize)]
struct State {
    users: Vec<User>,
    #[serde(default)] invites: Vec<Invite>,
    #[serde(default)] sessions: BTreeMap<String, Session>,
    #[serde(default)] merchants: Vec<Merchant>,
    #[serde(default)] log: Vec<Entry>,
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
        if a.s.users.iter().all(|u| u.level != Level::Owner || u.off) {
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

    fn note(&mut self, who: &str, what: String) {
        self.s.log.push(Entry { at: now(), who: who.into(), what });
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
        let u = User { id: format!("a{}", self.s.users.len() + 1), email, name, hash: hash_pw(pw)?, level, at: now(), off: false };
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
        let u = self.add_user(email, name, pw, Level::Owner)?;
        self.s.setup_hash.clear();
        self.note(&u.name.clone(), "Set up Agape Admin as its first owner".into());
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

    pub fn invite(&mut self, by: &User, email: &str, name: &str, level: Level) -> Result<String, String> {
        if by.level != Level::Owner { return Err("Only an owner can invite people".into()); }
        let email = email.trim().to_lowercase();
        if !good_email(&email) { return Err("Enter a valid email address".into()); }
        if self.s.users.iter().any(|u| u.email == email) { return Err("They already have an account".into()); }
        let code = token(18);
        self.s.invites.retain(|i| i.email != email && i.until > now());
        self.s.invites.push(Invite { hash: sha(&code), email: email.clone(), name: name.trim().chars().take(80).collect(), level, by: by.id.clone(), until: now() + INVITE_SECS });
        self.note(&by.name, format!("Invited {email} as {}", level_name(level)));
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
        let u = self.add_user(&i.email, if name.trim().is_empty() { &i.name } else { name }, pw, i.level)?;
        self.s.invites.retain(|x| x.hash != h);
        self.note(&u.name.clone(), "Accepted an invitation and joined".into());
        let t = self.start_session(&u.id);
        self.save();
        Ok((t, u))
    }

    pub fn cancel_invite(&mut self, by: &User, email: &str) -> Result<(), String> {
        if by.level != Level::Owner { return Err("Only an owner can do that".into()); }
        let email = email.trim().to_lowercase();
        let before = self.s.invites.len();
        self.s.invites.retain(|i| i.email != email);
        if self.s.invites.len() == before { return Err("No invitation for that email".into()); }
        self.note(&by.name, format!("Cancelled the invitation for {email}"));
        self.save();
        Ok(())
    }

    pub fn set_user(&mut self, by: &User, id: &str, level: Option<Level>, off: Option<bool>) -> Result<(), String> {
        if by.level != Level::Owner { return Err("Only an owner can change who has access".into()); }
        if id == by.id { return Err("You can't change your own access. Ask another owner.".into()); }
        let i = self.s.users.iter().position(|u| u.id == id).ok_or("No such person")?;
        if let Some(l) = level { self.s.users[i].level = l; }
        if let Some(o) = off { self.s.users[i].off = o; if o { let uid = id.to_string(); self.s.sessions.retain(|_, s| s.user != uid); } }
        let u = self.s.users[i].clone();
        self.note(&by.name, format!("{} {} ({})", if u.off { "Turned off access for" } else { "Set access for" }, u.email, level_name(u.level)));
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
        self.note(&by.name, "Changed their password".into());
        self.save();
        Ok(())
    }

    pub fn users(&self) -> Value {
        json!({
            "users": self.s.users.iter().map(|u| json!({ "id": u.id, "email": u.email, "name": u.name, "level": u.level, "at": u.at, "off": u.off })).collect::<Vec<_>>(),
            "invites": self.s.invites.iter().filter(|i| i.until > now()).map(|i| json!({ "email": i.email, "name": i.name, "level": i.level, "until": i.until })).collect::<Vec<_>>(),
        })
    }

    pub fn merchants(&self) -> &Vec<Merchant> { &self.s.merchants }

    pub fn save_merchant(&mut self, by: &User, id: Option<&str>, fields: &serde_json::Map<String, Value>, status: &str) -> Result<Merchant, String> {
        if by.level < Level::Editor { return Err("You can view, but not change, merchant details".into()); }
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
                self.note(&by.name, what);
                self.s.merchants[i].clone()
            }
            None => {
                let m = Merchant { id: format!("m{}", token(4)), fields: clean, status, at: n, by: by.name.clone() };
                self.s.merchants.push(m.clone());
                self.note(&by.name, format!("Added merchant {name}"));
                m
            }
        };
        self.save();
        Ok(m)
    }

    pub fn delete_merchant(&mut self, by: &User, id: &str) -> Result<(), String> {
        if by.level != Level::Owner { return Err("Only an owner can delete a merchant".into()); }
        let i = self.s.merchants.iter().position(|m| m.id == id).ok_or("No such merchant")?;
        let m = self.s.merchants.remove(i);
        self.note(&by.name, format!("Deleted merchant {}", m.fields.get("dba").cloned().unwrap_or_default()));
        self.save();
        Ok(())
    }

    /// Someone printed or downloaded a VAR sheet: worth knowing who and when.
    pub fn printed(&mut self, by: &User, id: &str) {
        if let Some(m) = self.s.merchants.iter().find(|m| m.id == id) {
            let name = m.fields.get("dba").cloned().unwrap_or_default();
            self.note(&by.name, format!("Printed the VAR sheet for {name}"));
            self.save();
        }
    }

    pub fn log(&self) -> Vec<Entry> { self.s.log.iter().rev().take(300).cloned().collect() }
}

pub fn level_name(l: Level) -> &'static str { match l { Level::Owner => "owner", Level::Editor => "editor", Level::Viewer => "viewer" } }

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
        let inv = a.invite(&owner, "p@y.com", "Partner", Level::Editor).unwrap();
        assert!(a.invite_info(&inv).is_some());
        let (t2, p) = a.accept(&inv, "", "anotherlong1").unwrap();
        assert_eq!(p.level, Level::Editor);
        assert!(a.accept(&inv, "", "anotherlong1").is_err());
        assert!(a.invite(&p, "z@z.com", "Z", Level::Owner).is_err());
        // reopened from disk, sessions still work and the setup code is gone
        drop(a);
        let mut a = Admin::open(d.path().to_path_buf());
        assert!(a.who(&t2).is_some());
        assert!(!a.setup_needed());
        for _ in 0..MAX_FAILS { let _ = a.sign_in("p@y.com", "wrongwrong"); }
        assert!(a.sign_in("p@y.com", "anotherlong1").is_err());
        a.set_user(&owner, &p.id, None, Some(true)).unwrap();
        assert!(a.who(&t2).is_none());
    }
}

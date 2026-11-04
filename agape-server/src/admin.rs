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

/// What a role lets someone do. Who someone can SEE never comes from here: that comes only from
/// where they sit in the structure (everything below them, nothing above or beside them).
pub const PERMS: &[(&str, &str)] = &[
    ("merchants_view", "See merchants and print VAR sheets"),
    ("merchants_edit", "Add and change merchants"),
    ("merchants_delete", "Delete merchants"),
    ("leads", "See leads from the website and demo sign-ups"),
    ("accounts", "See everyone with an Agape app account: fans, runners, stands, partners"),
    ("invite", "Invite people to report to them or to someone below them"),
    ("people", "Change the role, reporting line and access of people below them"),
    ("structure", "Create and edit roles, templates and groups below them"),
];
pub const HOST: &str = "host";
/// Addresses nobody can take: they belong to Agape itself.
const RESERVED: &[&str] = &["admin", "postmaster", "abuse", "hostmaster", "webmaster", "root", "noreply", "no-reply", "mailer-daemon", "security", "support", "billing", "legal", "privacy", "dmarc"];

/// A blueprint for roles. Editable; changing one doesn't change roles already made from it.
#[derive(Clone, Serialize, Deserialize)]
pub struct Template { pub id: String, pub name: String, #[serde(default)] pub about: String, pub perms: Vec<String> }
/// A role someone holds: made from a template, then its own.
#[derive(Clone, Serialize, Deserialize)]
pub struct Role { pub id: String, pub name: String, #[serde(default)] pub about: String, pub perms: Vec<String>, #[serde(default)] pub template: String }
/// A body people can report to, like a board. Its members see everything below it.
#[derive(Clone, Serialize, Deserialize)]
pub struct Group { pub id: String, pub name: String, #[serde(default)] pub about: String, #[serde(default)] pub members: Vec<String>, #[serde(default)] pub boss: Option<String> }

#[derive(Clone, Serialize, Deserialize)]
pub struct User { pub id: String, pub email: String, pub name: String, hash: String, #[serde(default)] pub role: String, pub at: u64, #[serde(default)] pub off: bool,
    /// Who they report to: a person ("a3") or a group ("g2"). Everyone but the host reports to someone.
    #[serde(default)] pub boss: Option<String>, #[serde(default)] pub title: String,
    /// Their address at Agape's domain, before the @ ("gabriel" for gabriel@agapetoyourseat.com).
    #[serde(default)] pub mailbox: String,
    /// Channels (people below them) whose pings they've muted. Muting never hides anything.
    #[serde(default)] pub muted: Vec<String>,
    /// From before roles existed ("owner", "admin", ...); read once and turned into a role.
    #[serde(default, rename = "level", skip_serializing)] legacy: Option<String> }

#[derive(Clone, Serialize, Deserialize)]
struct Invite { hash: String, email: String, name: String, #[serde(default)] role: String, by: String, until: u64, #[serde(default)] boss: String, #[serde(default)] title: String,
    #[serde(default, rename = "level", skip_serializing)] legacy: Option<String> }

#[derive(Clone, Serialize, Deserialize)]
struct Session { user: String, until: u64 }

#[derive(Clone, Serialize, Deserialize)]
pub struct Merchant { pub id: String, pub fields: BTreeMap<String, String>, pub status: String, pub at: u64, pub by: String }

#[derive(Clone, Serialize, Deserialize)]
pub struct Entry { pub at: u64, pub who: String, pub what: String, #[serde(default)] pub actor: String }

/// A message between people on Agape's team, or a notice Agape sent one of them.
/// Every communication is kept, and is visible to the people in it and to everyone they report up to.
#[derive(Clone, Serialize, Deserialize)]
pub struct Note { pub id: String, pub at: u64, pub from: String, pub to: Vec<String>, pub subject: String, pub body: String, #[serde(default)] pub kind: String, #[serde(default)] pub read: Vec<String>,
    /// Email only: who outside Agape sent it, or who outside Agape it went to, and what happened.
    #[serde(default)] pub ext_from: String, #[serde(default)] pub ext_to: Vec<String>, #[serde(default)] pub status: Vec<String>, #[serde(default)] pub attachments: Vec<String>,
    /// Addresses a temporary failure kept from getting it, with tries so far and when to try next.
    #[serde(default)] pub retry: Vec<(String, u32, u64)> }

#[derive(Default, Serialize, Deserialize)]
struct State {
    users: Vec<User>,
    #[serde(default)] invites: Vec<Invite>,
    #[serde(default)] sessions: BTreeMap<String, Session>,
    #[serde(default)] merchants: Vec<Merchant>,
    #[serde(default)] log: Vec<Entry>,
    #[serde(default)] notes: Vec<Note>,
    #[serde(default)] setup_hash: String,
    #[serde(default)] templates: Vec<Template>,
    #[serde(default)] roles: Vec<Role>,
    #[serde(default)] groups: Vec<Group>,
}

fn starting_templates() -> Vec<Template> {
    let t = |id: &str, name: &str, about: &str, p: &[&str]| Template { id: id.into(), name: name.into(), about: about.into(), perms: p.iter().map(|x| x.to_string()).collect() };
    vec![
        t("t-admin", "Admin", "Runs the company day to day: merchants, people, structure", &["merchants_view", "merchants_edit", "leads", "accounts", "invite", "people", "structure"]),
        t("t-manager", "Manager", "Runs a team", &["merchants_view", "invite", "people"]),
        t("t-member", "Team member", "Does the work; sees their own messages", &[]),
        t("t-board", "Board member", "Oversees the company; sees merchants, leads and accounts", &["merchants_view", "leads", "accounts"]),
        t("t-holder", "Shareholder", "Owns part of the company", &[]),
    ]
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
        a.migrate();
        if a.s.users.iter().all(|u| u.role != HOST || u.off) {
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

    /// Templates and roles for a store that has none, and roles for people who only had a level.
    fn migrate(&mut self) {
        if self.s.templates.is_empty() && self.s.roles.is_empty() {
            self.s.templates = starting_templates();
            for t in self.s.templates.clone().iter().take(3) { self.s.roles.push(Role { id: t.id.replacen("t-", "r-", 1), name: t.name.clone(), about: t.about.clone(), perms: t.perms.clone(), template: t.id.clone() }); }
        }
        // "accounts" came later: the starting admin and board templates, and the admin role, get it once
        if !self.s.templates.iter().any(|t| t.perms.iter().any(|p| p == "accounts")) {
            for t in self.s.templates.iter_mut().filter(|t| t.id == "t-admin" || t.id == "t-board") { t.perms.push("accounts".into()); }
            for r in self.s.roles.iter_mut().filter(|r| r.id == "r-admin") { if !r.perms.iter().any(|p| p == "accounts") { r.perms.push("accounts".into()); } }
        }
        let map = |l: &str| match l { "owner" | "host" => HOST, "admin" | "editor" => "r-admin", "manager" => "r-manager", _ => "r-member" }.to_string();
        for u in self.s.users.iter_mut() { if u.role.is_empty() { u.role = map(u.legacy.as_deref().unwrap_or("member")); } }
        for id in self.s.users.iter().filter(|u| u.mailbox.is_empty()).map(|u| u.id.clone()).collect::<Vec<_>>() { self.give_mailbox(&id); }
        for i in self.s.invites.iter_mut() { if i.role.is_empty() { i.role = map(i.legacy.as_deref().unwrap_or("member")); } }
    }

    /// Written whole, flushed to disk, then swapped in, so a crash never leaves half a file. Once a
    /// day the previous version is also kept as a dated copy (the last 60 days), so nothing is lost
    /// even to a bad change.
    fn save(&self) {
        let tmp = self.path.with_extension("json.tmp");
        let ok = std::fs::File::create(&tmp).and_then(|mut f| { use std::io::Write; f.write_all(&serde_json::to_vec_pretty(&self.s).unwrap())?; f.sync_all() }).is_ok();
        if !ok { eprintln!("admin: couldn't write {}", tmp.display()); return; }
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)); }
        if let Some(dir) = self.path.parent() {
            let day = now() / 86400;
            let keep = dir.join("admin-backups");
            let snap = keep.join(format!("admin-day{day}.json"));
            if self.path.exists() && !snap.exists() {
                let _ = std::fs::create_dir_all(&keep);
                let _ = std::fs::copy(&self.path, &snap);
                if let Ok(rd) = std::fs::read_dir(&keep) { for e in rd.flatten() { let n = e.file_name().to_string_lossy().to_string();
                    if let Some(d) = n.strip_prefix("admin-day").and_then(|x| x.strip_suffix(".json")).and_then(|x| x.parse::<u64>().ok()) { if d + 60 < day { let _ = std::fs::remove_file(e.path()); } } } }
            }
        }
        let _ = std::fs::rename(&tmp, &self.path);
    }

    fn note(&mut self, who: &User, what: String) {
        self.s.log.push(Entry { at: now(), who: who.name.clone(), what, actor: who.id.clone() });
        if self.s.log.len() > 5000 { self.s.log.drain(0..1000); }
    }

    pub fn setup_needed(&self) -> bool { !self.s.setup_hash.is_empty() }

    fn add_user(&mut self, email: &str, name: &str, pw: &str, role: &str) -> Result<User, String> {
        let email = email.trim().to_lowercase();
        let name = name.trim().chars().take(80).collect::<String>();
        if !good_email(&email) { return Err("Enter a valid email address".into()); }
        if name.is_empty() { return Err("Add your name".into()); }
        good_pw(pw)?;
        if self.s.users.iter().any(|u| u.email == email) { return Err("There's already an account with that email".into()); }
        let u = User { id: format!("a{}", self.s.users.len() + 1), email, name, hash: hash_pw(pw)?, role: role.into(), at: now(), off: false, boss: None, title: String::new(), mailbox: String::new(), muted: vec![], legacy: None };
        self.s.users.push(u.clone());
        self.give_mailbox(&u.id);
        let u = self.s.users.iter().find(|x| x.id == u.id).cloned().unwrap();
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
        let u = self.add_user(email, name, pw, HOST)?;
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

    // ---------------- structure: who sees whom ----------------

    pub fn perms_of(&self, u: &User) -> Vec<String> {
        if u.role == HOST { return PERMS.iter().map(|p| p.0.to_string()).collect(); }
        self.s.roles.iter().find(|r| r.id == u.role).map(|r| r.perms.clone()).unwrap_or_default()
    }
    pub fn can(&self, u: &User, perm: &str) -> bool { u.role == HOST || self.perms_of(u).iter().any(|p| p == perm) }
    fn covers(&self, u: &User, perms: &[String]) -> bool { u.role == HOST || { let mine = self.perms_of(u); perms.iter().all(|p| mine.contains(p)) } }
    pub fn role_name(&self, id: &str) -> String { if id == HOST { "Host".into() } else { self.s.roles.iter().find(|r| r.id == id).map(|r| r.name.clone()).unwrap_or_else(|| "No role".into()) } }

    /// Everything below a person or a group: the people and groups that report to it, the members
    /// of those groups, and so on down. A person also oversees what's below the groups they're in.
    pub fn down(&self, id: &str) -> (Vec<String>, Vec<String>) {
        let (mut users, mut groups): (Vec<String>, Vec<String>) = (vec![], vec![]);
        let mut frontier = vec![id.to_string()];
        if id.starts_with('a') { for g in self.s.groups.iter().filter(|g| g.members.iter().any(|m| m == id)) { frontier.push(g.id.clone()); } }
        let mut seen: Vec<String> = frontier.clone();
        while let Some(x) = frontier.pop() {
            for u in self.s.users.iter().filter(|u| u.boss.as_deref() == Some(x.as_str())) {
                if u.id != id && !users.contains(&u.id) { users.push(u.id.clone()); }
                if !seen.contains(&u.id) { seen.push(u.id.clone()); frontier.push(u.id.clone()); }
            }
            for g in self.s.groups.iter().filter(|g| g.boss.as_deref() == Some(x.as_str())) {
                if !groups.contains(&g.id) { groups.push(g.id.clone()); }
                for m in &g.members { if m != id && !users.contains(m) { users.push(m.clone()); } }
                if !seen.contains(&g.id) { seen.push(g.id.clone()); frontier.push(g.id.clone()); }
            }
        }
        (users, groups)
    }
    fn oversees(&self, by: &User, node: &str) -> bool {
        if by.role == HOST { return true; }
        let (u, g) = self.down(&by.id);
        u.iter().any(|x| x == node) || g.iter().any(|x| x == node)
    }
    /// Somewhere a person or group can report: you, or anything you oversee.
    fn may_report_to(&self, by: &User, target: &str) -> bool { target == by.id || self.oversees(by, target) }
    /// Reporting `node` to `target` would make a loop.
    fn loops(&self, node: &str, target: &str) -> bool {
        if node == target { return true; }
        let (u, g) = self.down(node);
        // a group's own members sit below it too
        u.iter().any(|x| x == target) || g.iter().any(|x| x == target)
    }
    fn node_name(&self, id: &str) -> String {
        if id.is_empty() { return "Agape".into(); }
        if let Some(u) = self.s.users.iter().find(|u| u.id == id) { return u.name.clone(); }
        if let Some(g) = self.s.groups.iter().find(|g| g.id == id) { return g.name.clone(); }
        "nobody".into()
    }
    fn exists(&self, id: &str) -> bool { self.s.users.iter().any(|u| u.id == id && !u.off) || self.s.groups.iter().any(|g| g.id == id) }
    /// The chain above someone: who they report to, and who that reports to, nearest first.
    pub fn chain(&self, id: &str) -> Vec<String> {
        let mut out = vec![];
        let boss_of = |x: &str| self.s.users.iter().find(|u| u.id == x).and_then(|u| u.boss.clone()).or_else(|| self.s.groups.iter().find(|g| g.id == x).and_then(|g| g.boss.clone()));
        let mut cur = boss_of(id);
        while let Some(b) = cur { if out.contains(&b) || out.len() > 50 { break; } out.push(b.clone()); cur = boss_of(&b); }
        out
    }
    /// The one rule: you see what you're part of, and everything made by anyone below you.
    fn sees(&self, viewer: &User, maker: &str, parties: &[String]) -> bool {
        if viewer.role == HOST || maker == viewer.id || parties.contains(&viewer.id) { return true; }
        let (below, _) = self.down(&viewer.id);
        below.iter().any(|b| b == maker || (maker.is_empty() && parties.contains(b)))
    }
    /// Who hears about something sent to a person or a group.
    fn people_in(&self, id: &str) -> Vec<String> {
        if let Some(g) = self.s.groups.iter().find(|g| g.id == id) { return g.members.clone(); }
        vec![id.to_string()]
    }

    // ---------------- people ----------------

    pub fn invite(&mut self, by: &User, email: &str, name: &str, role: &str, boss: &str, title: &str) -> Result<String, String> {
        if !self.can(by, "invite") { return Err("Your role can't invite people".into()); }
        if role == HOST { return Err("There's only one host".into()); }
        let r = self.s.roles.iter().find(|r| r.id == role).cloned().ok_or("Choose a role")?;
        if !self.covers(by, &r.perms) { return Err(format!("You can't give the role {}: it can do things yours can't", r.name)); }
        let boss = if boss.is_empty() { by.id.clone() } else { boss.to_string() };
        if !self.exists(&boss) || !self.may_report_to(by, &boss) { return Err("They have to report to you, or to someone or a group below you".into()); }
        let email = email.trim().to_lowercase();
        if !good_email(&email) { return Err("Enter a valid email address".into()); }
        if self.s.users.iter().any(|u| u.email == email) { return Err("They already have an account".into()); }
        let code = token(18);
        self.s.invites.retain(|i| i.email != email && i.until > now());
        self.s.invites.push(Invite { hash: sha(&code), email: email.clone(), name: name.trim().chars().take(80).collect(), role: role.into(), by: by.id.clone(), until: now() + INVITE_SECS, boss: boss.clone(), title: title.trim().chars().take(60).collect(), legacy: None });
        self.note(&by.clone(), format!("Invited {email} as {} reporting to {}", r.name, self.node_name(&boss)));
        self.save();
        Ok(code)
    }

    pub fn invite_info(&self, code: &str) -> Option<Value> {
        let h = sha(code.trim());
        self.s.invites.iter().find(|i| i.hash == h && i.until > now()).map(|i| json!({ "email": i.email, "name": i.name, "role": i.role, "role_name": self.role_name(&i.role), "boss_name": self.node_name(&i.boss) }))
    }

    pub fn accept(&mut self, code: &str, name: &str, pw: &str) -> Result<(String, User), String> {
        let h = sha(code.trim());
        let i = self.s.invites.iter().find(|i| i.hash == h && i.until > now()).cloned().ok_or("This invitation has expired or was already used. Ask for a new one.")?;
        let mut u = self.add_user(&i.email, if name.trim().is_empty() { &i.name } else { name }, pw, &i.role)?;
        let boss = if i.boss.is_empty() || !self.exists(&i.boss) { i.by.clone() } else { i.boss.clone() };
        if let Some(x) = self.s.users.iter_mut().find(|x| x.id == u.id) { x.boss = Some(boss.clone()); x.title = i.title.clone(); u = x.clone(); }
        self.s.invites.retain(|x| x.hash != h);
        let mut to = self.people_in(&boss); to.push(i.by.clone());
        let to: Vec<&str> = to.iter().map(|s| s.as_str()).collect();
        let (sub, body) = (format!("{} joined Agape Admin", u.name), format!("{} accepted the invitation as {} and reports to {}.", u.name, self.role_name(&u.role), self.node_name(&boss)));
        self.notify(&to, "joined", &sub, &body);
        self.note(&u.clone(), "Accepted an invitation and joined".into());
        let t = self.start_session(&u.id);
        self.save();
        Ok((t, u))
    }

    pub fn cancel_invite(&mut self, by: &User, email: &str) -> Result<(), String> {
        let email = email.trim().to_lowercase();
        let (mine, _) = self.down(&by.id);
        if !self.s.invites.iter().any(|i| i.email == email && (by.role == HOST || i.by == by.id || mine.contains(&i.by))) { return Err("No invitation of yours for that email".into()); }
        self.s.invites.retain(|i| i.email != email);
        self.note(&by.clone(), format!("Cancelled the invitation for {email}"));
        self.save();
        Ok(())
    }

    pub fn set_user(&mut self, by: &User, id: &str, role: Option<&str>, off: Option<bool>, boss: Option<&str>, title: Option<&str>) -> Result<(), String> {
        if id == by.id { return Err("You can't change your own role or reporting line. Ask someone above you.".into()); }
        if !self.can(by, "people") { return Err("Your role can't change people".into()); }
        if !self.oversees(by, id) { return Err("You can only change people below you".into()); }
        let i = self.s.users.iter().position(|u| u.id == id).ok_or("No such person")?;
        if self.s.users[i].role == HOST { return Err("Nobody can change the host".into()); }
        if let Some(r) = role {
            let rr = self.s.roles.iter().find(|x| x.id == r).cloned().ok_or("No such role")?;
            if !self.covers(by, &rr.perms) { return Err(format!("You can't give the role {}: it can do things yours can't", rr.name)); }
        }
        if let Some(b) = boss {
            if !self.exists(b) || !self.may_report_to(by, b) { return Err("They have to report to you, or to someone or a group below you".into()); }
            if self.loops(id, b) { return Err("That would make a loop: someone can't report to their own team".into()); }
        }
        if let Some(r) = role { self.s.users[i].role = r.into(); }
        if let Some(b) = boss { self.s.users[i].boss = Some(b.into()); }
        if let Some(t) = title { self.s.users[i].title = t.trim().chars().take(60).collect(); }
        if let Some(o) = off { self.s.users[i].off = o; if o { let uid = id.to_string(); self.s.sessions.retain(|_, s| s.user != uid); } }
        let u = self.s.users[i].clone();
        self.note(&by.clone(), format!("{} {}: {}, reports to {}", if u.off { "Turned off access for" } else { "Updated" }, u.email, self.role_name(&u.role), self.node_name(u.boss.as_deref().unwrap_or(""))));
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

    // ---------------- roles, templates, groups ----------------

    fn clean_perms(p: &[String]) -> Vec<String> { PERMS.iter().map(|x| x.0.to_string()).filter(|x| p.contains(x)).collect() }

    pub fn save_template(&mut self, by: &User, id: Option<&str>, name: &str, about: &str, perms: &[String]) -> Result<(), String> {
        if !self.can(by, "structure") { return Err("Your role can't change templates".into()); }
        let perms = Self::clean_perms(perms);
        if !self.covers(by, &perms) { return Err("A template can't do more than your own role can".into()); }
        let name = name.trim().chars().take(60).collect::<String>();
        if name.is_empty() { return Err("Name the template".into()); }
        let t = Template { id: id.map(String::from).unwrap_or_else(|| format!("t{}", token(4))), name: name.clone(), about: about.trim().chars().take(200).collect(), perms };
        match self.s.templates.iter().position(|x| x.id == t.id) { Some(i) => self.s.templates[i] = t, None => self.s.templates.push(t) }
        self.note(&by.clone(), format!("Saved the template {name}"));
        self.save(); Ok(())
    }
    pub fn delete_template(&mut self, by: &User, id: &str) -> Result<(), String> {
        if !self.can(by, "structure") { return Err("Your role can't change templates".into()); }
        let t = self.s.templates.iter().find(|t| t.id == id).cloned().ok_or("No such template")?;
        if !self.covers(by, &t.perms) { return Err("You can't remove a template that can do more than your role".into()); }
        self.s.templates.retain(|t| t.id != id);
        self.note(&by.clone(), format!("Deleted the template {}", t.name));
        self.save(); Ok(())
    }
    /// Who holds a role, all of them below the editor: otherwise editing it would change someone above.
    fn holders_below(&self, by: &User, role: &str) -> bool { by.role == HOST || self.s.users.iter().filter(|u| u.role == role).all(|u| self.oversees(by, &u.id)) }
    pub fn save_role(&mut self, by: &User, id: Option<&str>, name: &str, about: &str, perms: &[String], template: &str) -> Result<Role, String> {
        if !self.can(by, "structure") { return Err("Your role can't change roles".into()); }
        let perms = Self::clean_perms(perms);
        if !self.covers(by, &perms) { return Err("A role can't do more than your own role can".into()); }
        if let Some(id) = id { if !self.holders_below(by, id) { return Err("Someone at or above your level holds this role, so only someone above them can change it".into()); } }
        let name = name.trim().chars().take(60).collect::<String>();
        if name.is_empty() { return Err("Name the role".into()); }
        let r = Role { id: id.map(String::from).unwrap_or_else(|| format!("r{}", token(4))), name: name.clone(), about: about.trim().chars().take(200).collect(), perms, template: template.into() };
        match self.s.roles.iter().position(|x| x.id == r.id) { Some(i) => self.s.roles[i] = r.clone(), None => self.s.roles.push(r.clone()) }
        self.note(&by.clone(), format!("Saved the role {name}"));
        self.save(); Ok(r)
    }
    pub fn delete_role(&mut self, by: &User, id: &str) -> Result<(), String> {
        if !self.can(by, "structure") { return Err("Your role can't change roles".into()); }
        if self.s.users.iter().any(|u| u.role == id) || self.s.invites.iter().any(|i| i.role == id) { return Err("Someone holds this role or is invited to it. Give them another role first.".into()); }
        let r = self.s.roles.iter().find(|r| r.id == id).cloned().ok_or("No such role")?;
        if !self.covers(by, &r.perms) { return Err("You can't remove a role that can do more than yours".into()); }
        self.s.roles.retain(|r| r.id != id);
        self.note(&by.clone(), format!("Deleted the role {}", r.name));
        self.save(); Ok(())
    }
    pub fn save_group(&mut self, by: &User, id: Option<&str>, name: &str, about: &str, members: &[String], boss: Option<&str>) -> Result<(), String> {
        if !self.can(by, "structure") { return Err("Your role can't change groups".into()); }
        if let Some(id) = id { if !self.oversees(by, id) { return Err("Only someone above this group can change it".into()); } }
        let gid = id.map(String::from).unwrap_or_else(|| format!("g{}", token(4)));
        let boss = boss.filter(|b| !b.is_empty());
        if by.role != HOST && boss.is_none() { return Err("Choose who the group reports to. Only the host can put a group at the top.".into()); }
        if let Some(b) = boss {
            if !self.exists(b) || !self.may_report_to(by, b) { return Err("The group has to report to you, or to someone or a group below you".into()); }
            if id.is_some() && self.loops(&gid, b) { return Err("That would make a loop: a group can't report to someone below it".into()); }
        }
        let mut ms: Vec<String> = vec![];
        for m in members { if self.s.users.iter().any(|u| &u.id == m) && !ms.contains(m) {
            // joining a group means seeing everything below it, so nobody can bring in someone from above
            if !(by.role == HOST || self.oversees(by, m) || self.s.groups.iter().any(|g| g.id == gid && g.members.contains(m))) { return Err(format!("{} isn't below you, so you can't put them in a group", self.node_name(m))); }
            if boss == Some(m.as_str()) { return Err("A group can't report to one of its own members".into()); }
            ms.push(m.clone());
        } }
        let name = name.trim().chars().take(60).collect::<String>();
        if name.is_empty() { return Err("Name the group".into()); }
        let g = Group { id: gid.clone(), name: name.clone(), about: about.trim().chars().take(200).collect(), members: ms, boss: boss.map(String::from) };
        match self.s.groups.iter().position(|x| x.id == gid) { Some(i) => self.s.groups[i] = g, None => self.s.groups.push(g) }
        self.note(&by.clone(), format!("Saved the group {name}"));
        self.save(); Ok(())
    }
    pub fn delete_group(&mut self, by: &User, id: &str) -> Result<(), String> {
        if !self.can(by, "structure") || !self.oversees(by, id) { return Err("Only someone above this group can remove it".into()); }
        if self.s.users.iter().any(|u| u.boss.as_deref() == Some(id)) || self.s.groups.iter().any(|g| g.boss.as_deref() == Some(id)) { return Err("People or groups report to this group. Move them first.".into()); }
        let n = self.node_name(id);
        self.s.groups.retain(|g| g.id != id);
        self.note(&by.clone(), format!("Deleted the group {n}"));
        self.save(); Ok(())
    }

    /// The viewer's part of Agape: themselves, everything below them, the chain above (names only),
    /// and what they're allowed to hand out.
    pub fn team(&self, viewer: &User) -> Value {
        let host = viewer.role == HOST;
        let (mine, my_groups) = self.down(&viewer.id);
        let shown = |u: &&User| host || u.id == viewer.id || mine.contains(&u.id);
        let gshown = |g: &&Group| host || my_groups.contains(&g.id) || g.members.contains(&viewer.id);
        let user = |u: &User| json!({ "id": u.id, "email": u.email, "name": u.name, "role": u.role, "role_name": self.role_name(&u.role), "at": u.at, "off": u.off, "boss": u.boss, "title": u.title, "mailbox": u.mailbox, "mine": host || mine.contains(&u.id) });
        json!({
            "users": self.s.users.iter().filter(shown).map(user).collect::<Vec<_>>(),
            "groups": self.s.groups.iter().filter(gshown).map(|g| json!({ "id": g.id, "name": g.name, "about": g.about, "members": g.members, "member_names": g.members.iter().map(|m| self.node_name(m)).collect::<Vec<_>>(), "boss": g.boss, "mine": host || my_groups.contains(&g.id) })).collect::<Vec<_>>(),
            "chain": self.chain(&viewer.id).iter().map(|id| json!({ "id": id, "name": self.node_name(id), "group": id.starts_with('g') })).collect::<Vec<_>>(),
            "invites": self.s.invites.iter().filter(|i| i.until > now() && (host || i.by == viewer.id || mine.contains(&i.by))).map(|i| json!({ "email": i.email, "name": i.name, "role": i.role, "role_name": self.role_name(&i.role), "until": i.until, "boss": i.boss })).collect::<Vec<_>>(),
            "contacts": self.s.users.iter().filter(|u| !u.off && u.id != viewer.id).map(|u| json!({ "id": u.id, "name": u.name, "title": u.title })).collect::<Vec<_>>(),
            "roles": self.s.roles.iter().map(|r| json!({ "id": r.id, "name": r.name, "about": r.about, "perms": r.perms, "template": r.template, "holders": self.s.users.iter().filter(|u| u.role == r.id).count(), "grantable": self.covers(viewer, &r.perms), "editable": self.can(viewer, "structure") && self.covers(viewer, &r.perms) && self.holders_below(viewer, &r.id) })).collect::<Vec<_>>(),
            "templates": self.s.templates.iter().map(|t| json!({ "id": t.id, "name": t.name, "about": t.about, "perms": t.perms, "editable": self.can(viewer, "structure") && self.covers(viewer, &t.perms) })).collect::<Vec<_>>(),
            "perms": PERMS.iter().map(|(k, d)| json!({ "id": k, "about": d, "mine": self.can(viewer, k) })).collect::<Vec<_>>(),
        })
    }
    pub fn oversees_anyone(&self, u: &User) -> bool { u.role == HOST || !self.down(&u.id).0.is_empty() }

    // ---------------- messages and notices ----------------

    /// A notice from Agape to some people on the team. Kept like any other communication.
    pub fn notify(&mut self, to: &[&str], kind: &str, subject: &str, body: &str) {
        let mut ids: Vec<String> = vec![];
        for t in to { if !t.is_empty() && !ids.iter().any(|x| x == t) && self.s.users.iter().any(|u| u.id == *t) { ids.push(t.to_string()); } }
        if ids.is_empty() { return; }
        self.s.notes.push(Note { id: format!("n{}", token(5)), at: now(), from: String::new(), to: ids, subject: subject.into(), body: body.into(), kind: kind.into(), read: vec![], ext_from: String::new(), ext_to: vec![], status: vec![], attachments: vec![], retry: vec![] });
        if self.s.notes.len() > 20000 { self.s.notes.drain(0..2000); }
    }
    /// Tell everyone whose role has a permission (and the host).
    pub fn notify_perm(&mut self, perm: &str, kind: &str, subject: &str, body: &str) {
        let ids: Vec<String> = self.s.users.iter().filter(|u| !u.off && self.can(u, perm)).map(|u| u.id.clone()).collect();
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
        for t in to { for p in self.people_in(t) { if p != by.id && self.s.users.iter().any(|u| u.id == p && !u.off) && !ids.contains(&p) { ids.push(p); } } }
        if ids.is_empty() { return Err("Choose who it's for".into()); }
        self.s.notes.push(Note { id: format!("n{}", token(5)), at: now(), from: by.id.clone(), to: ids, subject: if subject.is_empty() { "(no subject)".into() } else { subject }, body, kind: "message".into(), read: vec![by.id.clone()], ext_from: String::new(), ext_to: vec![], status: vec![], attachments: vec![], retry: vec![] });
        self.save();
        Ok(())
    }

    /// Every communication the viewer may see, newest first, marked by how they see it.
    /// The people below the viewer a note belongs to: its channels.
    fn channels_of(&self, n: &Note, below: &[String]) -> Vec<String> {
        let mut c: Vec<String> = vec![];
        for p in std::iter::once(&n.from).chain(n.to.iter()) { if below.contains(p) && !c.contains(p) { c.push(p.clone()); } }
        c
    }

    /// Everything the viewer may see, newest first. Their own mail is "mine"; the rest sits in the
    /// channels of the people below them it belongs to, and is unread until the viewer opens it.
    pub fn inbox(&self, viewer: &User) -> Value {
        let (below, _) = self.down(&viewer.id);
        let below: Vec<String> = if viewer.role == HOST { self.s.users.iter().filter(|u| u.id != viewer.id).map(|u| u.id.clone()).collect() } else { below };
        let list: Vec<Value> = self.s.notes.iter().rev().filter(|n| self.sees(viewer, &n.from, &n.to)).take(2000).map(|n| {
            let mine = n.from == viewer.id || n.to.contains(&viewer.id);
            json!({ "id": n.id, "at": n.at, "from": n.from, "from_name": if !n.ext_from.is_empty() { n.ext_from.clone() } else { self.node_name(&n.from) }, "to": n.to,
                    "to_names": n.to.iter().map(|t| self.node_name(t)).chain(if n.from.is_empty() { vec![] } else { n.ext_to.clone() }).collect::<Vec<_>>(), "ext_from": n.ext_from, "ext_to": n.ext_to, "status": n.status, "attachments": n.attachments,
                    "subject": n.subject, "body": n.body, "kind": n.kind, "mine": mine, "sent": n.from == viewer.id, "channels": self.channels_of(n, &below),
                    "unread": n.from != viewer.id && !n.read.contains(&viewer.id) })
        }).collect();
        json!({ "notes": list, "muted": viewer.muted })
    }
    /// Opening something marks it read for the viewer only; everyone else's unread stays as it was.
    pub fn mark_read(&mut self, viewer: &User, ids: &[String]) {
        let ok: Vec<String> = self.s.notes.iter().filter(|n| ids.contains(&n.id) && self.sees(viewer, &n.from, &n.to)).map(|n| n.id.clone()).collect();
        for n in self.s.notes.iter_mut().filter(|n| ok.contains(&n.id)) { if !n.read.contains(&viewer.id) { n.read.push(viewer.id.clone()); } }
        self.save();
    }
    /// Unread in their own mailbox, and pings from the channels below them they haven't muted.
    pub fn unread(&self, viewer: &User) -> (usize, usize) {
        let (below, _) = self.down(&viewer.id);
        let below: Vec<String> = if viewer.role == HOST { self.s.users.iter().filter(|u| u.id != viewer.id).map(|u| u.id.clone()).collect() } else { below };
        let (mut mine, mut pings) = (0, 0);
        for n in self.s.notes.iter().filter(|n| n.from != viewer.id && !n.read.contains(&viewer.id)) {
            if n.to.contains(&viewer.id) { mine += 1; continue; }
            if self.channels_of(n, &below).iter().any(|c| !viewer.muted.contains(c)) && self.sees(viewer, &n.from, &n.to) { pings += 1; }
        }
        (mine, pings)
    }
    pub fn mute(&mut self, viewer: &User, channel: &str, on: bool) {
        if let Some(u) = self.s.users.iter_mut().find(|u| u.id == viewer.id) { u.muted.retain(|c| c != channel); if on { u.muted.push(channel.to_string()); } }
        self.save();
    }

    // ---------------- email ----------------

    /// A first address for someone: their first name, or first.last, or with a number.
    fn give_mailbox(&mut self, id: &str) {
        let Some(u) = self.s.users.iter().find(|u| u.id == id) else { return };
        let clean = |x: &str| x.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>();
        let parts: Vec<String> = u.name.split_whitespace().map(clean).filter(|p| !p.is_empty()).collect();
        let first = parts.first().cloned().unwrap_or_else(|| clean(u.email.split('@').next().unwrap_or("team")));
        let mut tries = vec![first.clone()];
        if parts.len() > 1 { tries.push(format!("{}.{}", first, parts.last().unwrap())); }
        for n in 2..50 { tries.push(format!("{first}{n}")); }
        let pick = tries.into_iter().find(|t| !t.is_empty() && !RESERVED.contains(&t.as_str()) && !self.s.users.iter().any(|x| x.mailbox == *t));
        if let (Some(p), Some(u)) = (pick, self.s.users.iter_mut().find(|u| u.id == id)) { u.mailbox = p; }
    }
    pub fn set_mailbox(&mut self, by: &User, id: &str, local: &str) -> Result<(), String> {
        if id != by.id && !(self.can(by, "people") && self.oversees(by, id)) { return Err("You can change your own address, or the address of people below you".into()); }
        let l = local.trim().to_lowercase();
        if l.len() < 2 || l.len() > 40 || !l.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_') || l.starts_with('.') || l.ends_with('.') { return Err("Use 2 to 40 letters, numbers, dots, dashes or underscores".into()); }
        if RESERVED.contains(&l.as_str()) { return Err("That address is kept for Agape itself".into()); }
        if self.s.users.iter().any(|u| u.mailbox == l && u.id != id) { return Err("Someone already has that address".into()); }
        let i = self.s.users.iter().position(|u| u.id == id).ok_or("No such person")?;
        let was = std::mem::replace(&mut self.s.users[i].mailbox, l.clone());
        self.note(&by.clone(), format!("Changed an address from {was} to {l}"));
        self.save(); Ok(())
    }
    pub fn mailbox_of(&self, id: &str) -> Option<(String, String)> { self.s.users.iter().find(|u| u.id == id && !u.off).map(|u| (u.name.clone(), u.mailbox.clone())) }
    fn by_mailbox(&self, local: &str) -> Option<String> { self.s.users.iter().find(|u| !u.off && u.mailbox.eq_ignore_ascii_case(local)).map(|u| u.id.clone()) }

    /// Split what someone typed in "To" into people at Agape and addresses outside it.
    pub fn recipients(&self, to: &[String], domain: &str) -> Result<(Vec<String>, Vec<String>), String> {
        let (mut inside, mut outside): (Vec<String>, Vec<String>) = (vec![], vec![]);
        for t in to.iter().flat_map(|t| t.split([',', ';'])).map(|t| t.trim()).filter(|t| !t.is_empty()) {
            if self.exists(t) { for p in self.people_in(t) { if !inside.contains(&p) { inside.push(p); } } continue; }
            let addr = t.rsplit('<').next().unwrap_or(t).trim_end_matches('>').trim().to_lowercase();
            if !good_email(&addr) { return Err(format!("{t} isn't an email address")); }
            match addr.split_once('@') {
                Some((local, d)) if d.eq_ignore_ascii_case(domain) => match self.by_mailbox(local) { Some(id) => if !inside.contains(&id) { inside.push(id) }, None => return Err(format!("Nobody at Agape has the address {addr}")) },
                _ => if !outside.contains(&addr) { outside.push(addr) },
            }
        }
        if inside.is_empty() && outside.is_empty() { return Err("Choose who it's for".into()); }
        Ok((inside, outside))
    }

    /// Keep an email someone at Agape sent (to people inside, outside, or both).
    pub fn record_sent(&mut self, by: &User, inside: Vec<String>, outside: Vec<String>, subject: &str, body: &str, status: Vec<String>, retry: Vec<(String, u32, u64)>) {
        let inside: Vec<String> = inside.into_iter().filter(|x| x != &by.id).collect();
        self.s.notes.push(Note { id: format!("n{}", token(5)), at: now(), from: by.id.clone(), to: inside, subject: subject.into(), body: body.into(), kind: "email".into(), read: vec![by.id.clone()], ext_from: String::new(), ext_to: outside, status, attachments: vec![], retry });
        self.save();
    }

    /// Keep an email that came in from outside, for whoever it was addressed to. Mail for an address
    /// nobody has goes to the host, so nothing is lost.
    pub fn receive(&mut self, from: &str, from_name: &str, to: &[String], subject: &str, body: &str, attachments: Vec<String>, domain: &str) -> usize {
        let mut ids: Vec<String> = vec![];
        for t in to { if let Some((local, d)) = t.split_once('@') { if d.eq_ignore_ascii_case(domain) { if let Some(id) = self.by_mailbox(local.split('+').next().unwrap_or(local)) { if !ids.contains(&id) { ids.push(id); } } } } }
        if ids.is_empty() { if let Some(h) = self.s.users.iter().find(|u| u.role == HOST && !u.off) { ids.push(h.id.clone()); } }
        let n = ids.len();
        let from_full = if from_name.trim().is_empty() { from.to_string() } else { format!("{} <{}>", from_name.trim(), from) };
        self.s.notes.push(Note { id: format!("n{}", token(5)), at: now(), from: String::new(), to: ids, subject: if subject.trim().is_empty() { "(no subject)".into() } else { subject.into() }, body: body.into(), kind: "email".into(), read: vec![], ext_from: from_full, ext_to: to.to_vec(), status: vec![], attachments, retry: vec![] });
        if self.s.notes.len() > 20000 { self.s.notes.drain(0..2000); }
        self.save();
        n
    }

    /// Sent emails with addresses due another try: (note id, sender name, sender address, to, subject, body, tries).
    pub fn due_retries(&self) -> Vec<(String, String, String, String, String, String, u32)> {
        let n = now();
        self.s.notes.iter().flat_map(|m| m.retry.iter().filter(|r| r.2 <= n).filter_map(move |r| self.mailbox_of(&m.from).map(|(name, mb)| (m.id.clone(), name, mb, r.0.clone(), m.subject.clone(), m.body.clone(), r.1)))).collect()
    }
    /// Record how a retry went. Gives up after about two days, and says so where the sender sees it.
    pub fn retried(&mut self, note: &str, to: &str, result: Result<String, String>, permanent: bool) {
        let Some(m) = self.s.notes.iter_mut().find(|m| m.id == note) else { return };
        let Some(i) = m.retry.iter().position(|r| r.0 == to) else { return };
        let tries = m.retry[i].1 + 1;
        match result {
            Ok(x) => { m.retry.remove(i); m.status.push(format!("{to}: delivered on try {} ({x})", tries + 1)); }
            Err(e) if permanent || tries >= 12 => { m.retry.remove(i); m.status.push(format!("{to}: not delivered. Gave up after {} tries. {e}", tries + 1)); m.read.clear(); }
            Err(_) => { m.retry[i].1 = tries; m.retry[i].2 = now() + 600 * (1u64 << tries.min(8)).min(36); }
        }
        self.save();
    }

    pub fn merchants(&self) -> &Vec<Merchant> { &self.s.merchants }

    pub fn save_merchant(&mut self, by: &User, id: Option<&str>, fields: &serde_json::Map<String, Value>, status: &str) -> Result<Merchant, String> {
        if !self.can(by, "merchants_edit") { return Err("Your role can view, but not change, merchant details".into()); }
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
        if !self.can(by, "merchants_delete") { return Err("Your role can't delete merchants".into()); }
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
        let (mine, _) = self.down(&viewer.id);
        self.s.log.iter().rev().filter(|e| viewer.role == HOST || e.actor == viewer.id || mine.contains(&e.actor)).take(300).cloned().collect()
    }
}


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
        let inv = a.invite(&owner, "p@y.com", "Partner", "r-admin", "", "").unwrap();
        assert!(a.invite_info(&inv).is_some());
        let (t2, p) = a.accept(&inv, "", "anotherlong1").unwrap();
        assert_eq!(p.role, "r-admin");
        assert!(a.accept(&inv, "", "anotherlong1").is_err());
        assert!(a.invite(&p, "z@z.com", "Z", HOST, "", "").is_err());
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
        let join = |a: &mut Admin, by: &User, e: &str, l: &str| { let c = a.invite(by, e, e, l, "", "").unwrap(); a.accept(&c, "", "longenough1").unwrap().1 };
        let admin = join(&mut a, &host, "admin@x.com", "r-admin");
        let mgr = join(&mut a, &admin, "mgr@x.com", "r-manager");
        let mem = join(&mut a, &mgr, "mem@x.com", "r-member");
        let other = join(&mut a, &admin, "other@x.com", "r-manager");
        assert!(a.invite(&mgr, "x@x.com", "x", "r-admin", "", "").is_err(), "no one hands out a role that can do more than theirs");
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
        assert!(a.set_user(&mgr, &other.id, Some("r-member"), None, None, None).is_err(), "not below them");
        assert!(a.set_user(&mgr, &mem.id, Some("r-admin"), None, None, None).is_err(), "can't grant more than they have");
        assert!(a.set_user(&admin, &mgr.id, None, None, Some(&mem.id), None).is_err(), "no cycles");
        assert!(a.team(&mgr)["users"].as_array().unwrap().len() == 2);
    }

    #[test]
    fn groups_like_a_board_oversee_whoever_reports_to_them() {
        let d = tempfile::tempdir().unwrap();
        let mut a = Admin::open(d.path().to_path_buf());
        let code = std::fs::read_to_string(d.path().join("admin-setup-code")).unwrap();
        let (_, host) = a.setup(&code, "g@x.com", "Gabriel", "longenough1").unwrap();
        a.save_template(&host, Some("t-board"), "Board member", "", &["merchants_view".into(), "leads".into()]).unwrap();
        let board_role = a.save_role(&host, None, "Director", "", &["merchants_view".into(), "leads".into()], "t-board").unwrap();
        let join = |a: &mut Admin, by: &User, e: &str, r: &str, boss: &str| { let c = a.invite(by, e, e, r, boss, "").unwrap(); a.accept(&c, "", "longenough1").unwrap().1 };
        let dir = join(&mut a, &host, "dir@x.com", &board_role.id, "");
        a.save_group(&host, None, "Board of Directors", "", &[dir.id.clone()], None).unwrap();
        let board = a.team(&host)["groups"][0]["id"].as_str().unwrap().to_string();
        let ceo = join(&mut a, &host, "ceo@x.com", "r-admin", &board);
        assert_eq!(ceo.boss.as_deref(), Some(board.as_str()));
        let staff = join(&mut a, &ceo, "staff@x.com", "r-member", "");
        a.send(&staff, &[ceo.id.clone()], "From staff", "x").unwrap();
        a.send(&dir, &[host.id.clone()], "Director to host", "x").unwrap();
        let subjects = |a: &Admin, u: &User| a.inbox(u)["notes"].as_array().unwrap().iter().filter(|n| n["kind"] == "message").map(|n| n["subject"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert!(subjects(&a, &dir).contains(&"From staff".to_string()), "the board sees everything below it");
        assert!(!subjects(&a, &ceo).contains(&"Director to host".to_string()), "the CEO sees nothing of the board's");
        // the CEO can't join the board, or point the board at themselves
        assert!(a.save_group(&ceo, Some(&board), "Board of Directors", "", &[dir.id.clone(), ceo.id.clone()], Some(&ceo.id)).is_err());
        // and can't make the board report to their own staff (a loop), even as host
        assert!(a.save_group(&host, Some(&board), "Board of Directors", "", &[dir.id.clone()], Some(&staff.id)).is_err());
        // a role held by the CEO can't be edited by the CEO
        assert!(a.save_role(&ceo, Some("r-admin"), "Admin", "", &["merchants_view".into()], "").is_err());
        // templates: editable, and a role made from one is its own
        assert!(a.save_template(&ceo, Some("t-member"), "Team member", "", &["merchants_view".into()]).is_ok());
        assert!(a.team(&host)["roles"].as_array().unwrap().iter().any(|r| r["id"] == "r-member" && r["perms"].as_array().unwrap().is_empty()));
    }
}
#[cfg(test)]
mod channel_tests {
    use super::*;
    #[test]
    fn mail_to_someone_below_pings_everyone_above_until_they_read_it() {
        let d = tempfile::tempdir().unwrap();
        let mut a = Admin::open(d.path().to_path_buf());
        let code = std::fs::read_to_string(d.path().join("admin-setup-code")).unwrap();
        let (_, host) = a.setup(&code, "g@x.com", "Gabriel", "longenough1").unwrap();
        let c = a.invite(&host, "p@x.com", "Phillip Ames", "r-admin", "", "").unwrap();
        let (_, phil) = a.accept(&c, "", "longenough1").unwrap();
        assert_eq!(phil.mailbox, "phillip");
        let base = a.unread(&host);
        a.receive("sam@venue.example", "Sam", &["phillip@agapetoyourseat.com".into()], "Pilot", "March?", vec![], "agapetoyourseat.com");
        let phil = a.s.users.iter().find(|u| u.id == phil.id).cloned().unwrap();
        assert_eq!(a.unread(&phil).0, 1, "it goes straight to Phillip");
        assert_eq!(a.unread(&host).1, base.1 + 1, "and pings the host in Phillip's channel");
        let id = a.inbox(&host)["notes"][0]["id"].as_str().unwrap().to_string();
        a.mark_read(&host, &[id]);
        assert_eq!(a.unread(&host).1, base.1);
        assert_eq!(a.unread(&phil).0, 1, "the host reading it doesn't mark it read for Phillip");
        // mail to an address nobody has still reaches the host
        a.receive("x@y.example", "", &["nobody@agapetoyourseat.com".into()], "Hi", "?", vec![], "agapetoyourseat.com");
        let host = a.s.users.iter().find(|u| u.id == host.id).cloned().unwrap();
        assert!(a.unread(&host).0 >= 1);
        // muting stops the pings, not the mail
        a.receive("sam@venue.example", "Sam", &["phillip@agapetoyourseat.com".into()], "Again", "?", vec![], "agapetoyourseat.com");
        a.mute(&host, &phil.id, true);
        let host = a.s.users.iter().find(|u| u.id == host.id).cloned().unwrap();
        assert_eq!(a.unread(&host).1, base.1);
        assert!(a.inbox(&host)["notes"].as_array().unwrap().iter().any(|n| n["subject"] == "Again"));
    }
}

#[cfg(test)]
mod migrate_tests {
    use super::*;
    #[test]
    fn accounts_from_before_roles_keep_working() {
        let d = tempfile::tempdir().unwrap();
        let h = hash_pw("longenough1").unwrap();
        std::fs::write(d.path().join("admin.json"), serde_json::to_vec(&json!({ "users": [
            { "id": "a1", "email": "g@x.com", "name": "Gabriel", "hash": h, "level": "host", "at": 1, "off": false, "boss": null, "title": "" },
            { "id": "a2", "email": "p@x.com", "name": "Pat", "hash": h, "level": "admin", "at": 1, "off": false, "boss": "a1", "title": "" }] })).unwrap()).unwrap();
        let mut a = Admin::open(d.path().to_path_buf());
        assert!(!a.setup_needed());
        let (_, g) = a.sign_in("g@x.com", "longenough1").unwrap();
        assert_eq!(g.role, HOST);
        let (_, p) = a.sign_in("p@x.com", "longenough1").unwrap();
        assert_eq!(p.role, "r-admin");
        assert!(a.can(&p, "merchants_edit") && !a.can(&p, "merchants_delete"));
        drop(a);
        let saved = std::fs::read_to_string(d.path().join("admin.json")).unwrap();
        assert!(saved.contains("\"role\": \"host\"") && !saved.contains("\"level\""));
    }
}

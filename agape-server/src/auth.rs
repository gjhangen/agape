//! Accounts and sessions. An account is a person; it acts in a venue as one party with one role.
//! Fans sign themselves up. Staff, businesses and players are given accounts by an operator,
//! because their role is something the venue grants, not something a person can claim.
//! Passwords are stored only as Argon2 hashes; sessions are random 256-bit tokens.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role { Fan, Runner, Stand, Business, Player, Ops }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub handle: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub hash: String,
    pub role: Role,
    /// The party this account acts as in the world (a fan's own id, "runner3", "grill-104", "hbk-1").
    pub party: String,
    /// The venue a staff account belongs to (fans may go anywhere).
    pub venue: Option<String>,
}

impl Account { pub fn public(&self) -> Account { Account { hash: String::new(), ..self.clone() } } }

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Rec { Account(Account), Session { token: String, account: String }, Ended { token: String } }

pub struct Auth { path: PathBuf, accounts: HashMap<String, Account>, by_handle: HashMap<String, String>, sessions: HashMap<String, String> }

pub fn valid_handle(h: &str) -> bool { (3..=24).contains(&h.len()) && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') }

impl Auth {
    pub fn open(path: PathBuf) -> Auth {
        let mut a = Auth { path: path.clone(), accounts: HashMap::new(), by_handle: HashMap::new(), sessions: HashMap::new() };
        if let Ok(f) = std::fs::File::open(&path) {
            for line in BufReader::new(f).lines().map_while(Result::ok) {
                match serde_json::from_str::<Rec>(&line) {
                    Ok(Rec::Account(acc)) => { a.by_handle.insert(acc.handle.to_lowercase(), acc.id.clone()); a.accounts.insert(acc.id.clone(), acc); }
                    Ok(Rec::Session { token, account }) => { a.sessions.insert(token, account); }
                    Ok(Rec::Ended { token }) => { a.sessions.remove(&token); }
                    Err(_) => {}
                }
            }
        }
        a
    }

    fn append(&self, r: &Rec) {
        let mut f = OpenOptions::new().create(true).append(true).open(&self.path).expect("accounts file");
        let mut line = serde_json::to_vec(r).unwrap();
        line.push(b'\n');
        f.write_all(&line).unwrap();
        f.sync_data().unwrap();
    }

    /// Signing out ends the session everywhere it was used.
    pub fn sign_out(&mut self, token: &str) {
        if self.sessions.remove(token).is_some() { self.append(&Rec::Ended { token: token.into() }); }
    }

    pub fn create(&mut self, handle: &str, password: &str, role: Role, party: Option<String>, venue: Option<String>) -> Result<Account, String> {
        let handle = handle.trim();
        if !valid_handle(handle) { return Err("Pick a name of 3 to 24 letters, numbers, dots or underscores".into()); }
        if password.chars().count() < 8 { return Err("Use a password of at least 8 characters".into()); }
        if self.by_handle.contains_key(&handle.to_lowercase()) { return Err("That name is taken".into()); }
        let id = format!("u{}", self.accounts.len() + 1);
        let mut raw = [0u8; 16];
        rand::rng().fill_bytes(&mut raw);
        let salt = SaltString::encode_b64(&raw).map_err(|e| e.to_string())?;
        let hash = Argon2::default().hash_password(password.as_bytes(), &salt).map_err(|e| e.to_string())?.to_string();
        let party = match role { Role::Fan => id.clone(), _ => party.ok_or("Staff accounts need a party")? };
        let acc = Account { id: id.clone(), handle: handle.into(), hash, role, party, venue };
        self.append(&Rec::Account(acc.clone()));
        self.by_handle.insert(handle.to_lowercase(), id.clone());
        self.accounts.insert(id, acc.clone());
        Ok(acc)
    }

    pub fn sign_in(&mut self, handle: &str, password: &str) -> Result<(String, Account), String> {
        let wrong = || "That name and password don't match".to_string();
        let handle = handle.trim();
        let id = self.by_handle.get(&handle.to_lowercase()).ok_or_else(wrong)?.clone();
        let acc = self.accounts[&id].clone();
        let parsed = PasswordHash::new(&acc.hash).map_err(|_| wrong())?;
        Argon2::default().verify_password(password.as_bytes(), &parsed).map_err(|_| wrong())?;
        let mut b = [0u8; 32];
        rand::rng().fill_bytes(&mut b);
        let token: String = b.iter().map(|x| format!("{x:02x}")).collect();
        self.append(&Rec::Session { token: token.clone(), account: id });
        self.sessions.insert(token.clone(), acc.id.clone());
        Ok((token, acc))
    }

    pub fn party_of(&self, account: &str) -> Option<String> { self.accounts.get(account).map(|a| a.party.clone()) }

    pub fn session(&self, token: &str) -> Option<&Account> { self.sessions.get(token).and_then(|id| self.accounts.get(id)) }
}

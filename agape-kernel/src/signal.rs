//! Signals: the append-only log (invariant 10), typed by a schema, with authority (A5)
//! and per-person encryption so erasure is destroying a key (A4).

use crate::value::PartyId;
#[cfg(feature = "vault")]
use chacha20poly1305::aead::{Aead, Generate, Key, KeyInit};
#[cfg(feature = "vault")]
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Val {
    B(bool),
    I(i64),
    S(String),
}

impl Val {
    pub fn as_i(&self) -> Option<i64> { if let Val::I(i) = self { Some(*i) } else { None } }
    pub fn as_i_opt(&self) -> Option<i64> { self.as_i() }
    pub fn key(&self) -> String {
        match self { Val::B(b) => b.to_string(), Val::I(i) => i.to_string(), Val::S(s) => s.clone() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ty { Bool, Int, Str }

impl Ty {
    pub fn admits(&self, v: &Val) -> bool {
        matches!((self, v), (Ty::Bool, Val::B(_)) | (Ty::Int, Val::I(_)) | (Ty::Str, Val::S(_)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Authority { Fact, Claim }

pub type Fields = BTreeMap<String, Val>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signal {
    pub seq: u64,
    pub kind: String,
    pub fields: Fields,
    pub author: PartyId,
    pub t_ms: i64,
    pub authority: Authority,
    /// Rule stratum that emitted this signal; 0 for signals from the world.
    pub stratum: u32,
    /// Personal payload, sealed under the subject's key.
    pub sealed: Option<Sealed>,
}

/// What a kind of signal looks like and which role may state it as fact.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KindSpec {
    pub fields: BTreeMap<String, Ty>,
    /// Role required for this signal to be a fact. None: anyone's signal is a fact.
    pub fact_role: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Schema {
    pub kinds: BTreeMap<String, KindSpec>,
}

impl Schema {
    pub fn kind(mut self, name: &str, fields: &[(&str, Ty)], fact_role: Option<&str>) -> Self {
        self.kinds.insert(name.into(), KindSpec {
            fields: fields.iter().map(|(k, t)| (k.to_string(), *t)).collect(),
            fact_role: fact_role.map(String::from),
        });
        self
    }
    pub fn field_ty(&self, kind: &str, field: &str) -> Option<Ty> {
        self.kinds.get(kind)?.fields.get(field).copied()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SignalError {
    UnknownKind(String),
    UnknownField(String, String),
    WrongType(String, String),
    MissingField(String, String),
}

/// Roles held by parties; scoped to the event this log belongs to.
#[derive(Clone, Debug, Default)]
pub struct Roles(pub HashMap<PartyId, Vec<String>>);

impl Roles {
    pub fn grant(&mut self, p: &str, role: &str) { self.0.entry(p.into()).or_default().push(role.into()); }
    pub fn has(&self, p: &str, role: &str) -> bool { self.0.get(p).map_or(false, |r| r.iter().any(|x| x == role)) }
}

#[derive(Default)]
pub struct Log {
    pub schema: Schema,
    pub roles: Roles,
    pub signals: Vec<Signal>,
}

impl Log {
    pub fn new(schema: Schema, mut roles: Roles) -> Self {
        if !roles.has("system", "system") { roles.grant("system", "system"); }
        Log { schema, roles, signals: vec![] }
    }

    /// Append a signal derived by the kernel itself (span changes, rule emits). Always a fact,
    /// because it is a deterministic consequence of facts already in the log.
    pub fn append_derived(&mut self, kind: &str, fields: Fields, author: &str, t_ms: i64, stratum: u32) -> Result<Signal, SignalError> {
        self.validate(kind, &fields)?;
        let s = Signal { seq: self.signals.len() as u64, kind: kind.into(), fields, author: author.into(), t_ms, authority: Authority::Fact, stratum, sealed: None };
        self.signals.push(s.clone());
        Ok(s)
    }

    pub fn validate(&self, kind: &str, fields: &Fields) -> Result<(), SignalError> {
        let spec = self.schema.kinds.get(kind).ok_or_else(|| SignalError::UnknownKind(kind.into()))?;
        for (k, v) in fields {
            let ty = spec.fields.get(k).ok_or_else(|| SignalError::UnknownField(kind.into(), k.clone()))?;
            if !ty.admits(v) { return Err(SignalError::WrongType(kind.into(), k.clone())); }
        }
        for k in spec.fields.keys() {
            if !fields.contains_key(k) { return Err(SignalError::MissingField(kind.into(), k.clone())); }
        }
        Ok(())
    }

    /// Append a signal from the world. Authority is decided here, never by the author.
    pub fn append(&mut self, kind: &str, fields: Fields, author: &str, t_ms: i64) -> Result<Signal, SignalError> {
        self.append_full(kind, fields, author, t_ms, 0, None)
    }

    pub fn append_full(&mut self, kind: &str, fields: Fields, author: &str, t_ms: i64, stratum: u32, sealed: Option<Sealed>) -> Result<Signal, SignalError> {
        self.validate(kind, &fields)?;
        let spec = &self.schema.kinds[kind];
        let authority = match &spec.fact_role {
            None => Authority::Fact,
            Some(r) if self.roles.has(author, r) => Authority::Fact,
            Some(_) => Authority::Claim,
        };
        let s = Signal { seq: self.signals.len() as u64, kind: kind.into(), fields, author: author.into(), t_ms, authority, stratum, sealed };
        self.signals.push(s.clone());
        Ok(s)
    }
}

#[macro_export]
macro_rules! fields {
    ($($k:expr => $v:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut m = $crate::signal::Fields::new();
        $( m.insert($k.to_string(), $crate::signal::IntoVal::into_val($v)); )*
        m
    }};
}

pub trait IntoVal { fn into_val(self) -> Val; }
impl IntoVal for i64 { fn into_val(self) -> Val { Val::I(self) } }
impl IntoVal for i32 { fn into_val(self) -> Val { Val::I(self as i64) } }
impl IntoVal for bool { fn into_val(self) -> Val { Val::B(self) } }
impl IntoVal for &str { fn into_val(self) -> Val { Val::S(self.into()) } }
impl IntoVal for String { fn into_val(self) -> Val { Val::S(self) } }
impl IntoVal for Val { fn into_val(self) -> Val { self } }

// ---------- per-person encryption ----------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sealed {
    pub subject: PartyId,
    pub nonce: Vec<u8>,
    pub ct: Vec<u8>,
}

#[cfg(feature = "vault")]
/// Holds one key per person. Erasure deletes the key; the sealed bytes in the
/// append-only log become unreadable without rewriting history.
#[derive(Default)]
pub struct Vault {
    keys: HashMap<PartyId, Key<XChaCha20Poly1305>>,
}

#[cfg(feature = "vault")]
#[derive(Debug, PartialEq, Eq)]
pub enum VaultError { Erased, Corrupt }

#[cfg(feature = "vault")]
impl Vault {
    pub fn seal(&mut self, subject: &str, plaintext: &[u8]) -> Sealed {
        let key = self.keys.entry(subject.into()).or_insert_with(Key::<XChaCha20Poly1305>::generate);
        let cipher = XChaCha20Poly1305::new(key);
        let nonce = XNonce::generate();
        let ct = cipher.encrypt(&nonce, plaintext).expect("encrypt");
        Sealed { subject: subject.into(), nonce: nonce.to_vec(), ct }
    }
    pub fn open(&self, s: &Sealed) -> Result<Vec<u8>, VaultError> {
        let key = self.keys.get(&s.subject).ok_or(VaultError::Erased)?;
        let cipher = XChaCha20Poly1305::new(key);
        let nonce = XNonce::try_from(s.nonce.as_slice()).map_err(|_| VaultError::Corrupt)?;
        cipher.decrypt(&nonce, s.ct.as_slice()).map_err(|_| VaultError::Corrupt)
    }
    pub fn erase(&mut self, subject: &str) -> bool { self.keys.remove(subject).is_some() }
}

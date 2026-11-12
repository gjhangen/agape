//! Email for @agapetoyourseat.com, run by Agape itself: no mail provider.
//!
//! Receiving: Cloudflare Email Routing hands each message to a tiny Cloudflare Worker, which posts
//! the raw message here with a shared secret. Sending: Agape talks SMTP straight to the recipient's
//! mail server (found through its MX record), signed with Agape's own DKIM key so the receiver can
//! tell it really came from agapetoyourseat.com. Every message is kept in Agape Admin under the same
//! rule as everything else there: the people in it see it, and so does everyone above them.

use hickory_resolver::TokioResolver;
use mail_parser::MimeHeaders;
use lettre::message::dkim::{DkimConfig, DkimSigningAlgorithm, DkimSigningKey};
use lettre::message::{header::ContentType, Mailbox, Message};
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::transport::smtp::extension::ClientId;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const SELECTOR: &str = "agape1";

/// The receiver said no for good (a 5xx answer, or no such address); anything else is tried again.
pub fn permanent(e: &str) -> bool { e.contains("refused it for good") || e.contains("isn't an email address") }

pub struct Mail { pub domain: String, pub host: String, key_pem: String, pub public_b64: String, pub secret: String, dir: PathBuf }

fn private_file(path: &Path, contents: &str) {
    let _ = std::fs::write(path, contents);
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)); }
}

impl Mail {
    /// The signing key and the Worker's secret are made once, on the server, and never leave it
    /// (only the public half of the key is published, in DNS).
    pub fn open(dir: &Path, domain: &str) -> Mail {
        use rsa::pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey};
        use rsa::pkcs8::EncodePublicKey;
        let kp: PathBuf = dir.join("mail-dkim.pem");
        let key = match std::fs::read_to_string(&kp).ok().and_then(|p| rsa::RsaPrivateKey::from_pkcs1_pem(&p).ok()) {
            Some(k) => k,
            None => {
                let k = rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).expect("dkim key");
                private_file(&kp, &k.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF).unwrap());
                k
            }
        };
        let key_pem = key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF).unwrap().to_string();
        let der = rsa::RsaPublicKey::from(&key).to_public_key_der().unwrap();
        use base64::Engine;
        let public_b64 = base64::engine::general_purpose::STANDARD.encode(der.as_bytes());
        let sp = dir.join("mail-inbound-secret");
        let secret = match std::fs::read_to_string(&sp) { Ok(s) if s.trim().len() >= 32 => s.trim().to_string(), _ => {
            let mut b = [0u8; 24]; rand::RngCore::fill_bytes(&mut rand::rng(), &mut b);
            let s: String = b.iter().map(|x| format!("{x:02x}")).collect();
            private_file(&sp, &s); s
        } };
        Mail { domain: domain.into(), host: format!("mail.{domain}"), key_pem, public_b64, secret, dir: dir.to_path_buf() }
    }

    pub fn dkim_record(&self) -> String { format!("v=DKIM1; k=rsa; p={}", self.public_b64) }

    /// Send one message to people outside Agape, straight to each recipient's mail server.
    /// Returns, for each address, "delivered" or why not, in words.
    pub async fn send(&self, from_name: &str, from_local: &str, to: &[String], subject: &str, body: &str, reply_to: Option<&str>) -> Vec<(String, Result<String, String>)> {
        // with a sending service set up, everything goes through it, still from @domain and signed for it
        if let Some(svc) = Service::load(&self.dir) {
            let from = format!("{} <{}@{}>", from_name.replace(['<', '>', '"', ','], ""), from_local, self.domain);
            let mut out = vec![];
            for rcpt in to { out.push((rcpt.clone(), svc.send(&from, rcpt, subject, body, reply_to, &[]).await)); }
            return out;
        }
        let mut out = vec![];
        let resolver = match TokioResolver::builder_tokio() { Ok(b) => b.build(), Err(e) => { return to.iter().map(|t| (t.clone(), Err(format!("Couldn't look up mail servers: {e}")))).collect(); } };
        let from: Mailbox = match format!("{} <{}@{}>", from_name.replace(['<', '>', '"'], ""), from_local, self.domain).parse() { Ok(m) => m, Err(e) => return to.iter().map(|t| (t.clone(), Err(format!("Bad sender: {e}")))).collect() };
        for rcpt in to {
            let r = self.send_one(&resolver, &from, rcpt, subject, body, reply_to).await;
            out.push((rcpt.clone(), r));
        }
        out
    }

    async fn send_one(&self, resolver: &TokioResolver, from: &Mailbox, rcpt: &str, subject: &str, body: &str, reply_to: Option<&str>) -> Result<String, String> {
        let to: Mailbox = rcpt.parse().map_err(|_| format!("{rcpt} isn't an email address"))?;
        let mut b = Message::builder().from(from.clone()).to(to.clone()).subject(subject).header(ContentType::TEXT_PLAIN);
        if let Some(r) = reply_to { if let Ok(m) = r.parse::<Mailbox>() { b = b.reply_to(m); } }
        let mut msg = b.body(body.to_string()).map_err(|e| e.to_string())?;
        let key = DkimSigningKey::new(&self.key_pem, DkimSigningAlgorithm::Rsa).map_err(|e| format!("{e:?}"))?;
        lettre::message::dkim::dkim_sign(&mut msg, &DkimConfig::default_config(SELECTOR.into(), self.domain.clone(), key));
        let domain = to.email.domain().to_string();
        let mut hosts: Vec<(u16, String)> = match resolver.mx_lookup(domain.as_str()).await {
            Ok(mx) => mx.iter().map(|m| (m.preference(), m.exchange().to_utf8().trim_end_matches('.').to_string())).collect(),
            Err(_) => vec![(0, domain.clone())],
        };
        hosts.sort();
        let mut last = format!("No mail server found for {domain}");
        for (_, host) in hosts.iter().take(3) {
            // opportunistic TLS, as mail servers do with each other: encrypted whenever the other side offers it
            let tls = TlsParameters::builder(host.clone()).dangerous_accept_invalid_certs(true).dangerous_accept_invalid_hostnames(true).build_rustls().map_err(|e| e.to_string())?;
            let t: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host.as_str()).port(25)
                .hello_name(ClientId::Domain(self.host.clone())).tls(Tls::Opportunistic(tls)).timeout(Some(Duration::from_secs(25))).build();
            match t.send(msg.clone()).await {
                Ok(r) => return Ok(format!("Accepted by {host}: {}", r.message().collect::<Vec<_>>().join(" "))),
                Err(e) => { if e.is_permanent() { return Err(format!("{host} refused it for good: {e}")); } last = format!("{host} couldn't take it yet: {e}"); }
            }
        }
        Err(last)
    }

    /// A copy of an incoming email, to someone's own address. Only through the sending service (Agape's server can't
    /// reach most inboxes on its own). Replying to it goes to the copy's private reply address, back through Agape.
    pub async fn send_copy(&self, sender: &str, to: &str, subject: &str, body: &str, key: &str, files: &[(String, Vec<u8>)]) -> Result<String, String> {
        let svc = Service::load(&self.dir).ok_or("No sending service, so no copy")?;
        let from = format!("{} via Agape <mail@{}>", sender.replace(['<', '>', '"', ','], "").trim(), self.domain);
        let reply = format!("reply+{key}@{}", self.domain);
        svc.send(&from, to, subject, body, Some(&reply), files).await
    }

    /// Everything that decides whether mail from Agape lands in inboxes, checked live.
    pub async fn diagnose(&self) -> Value {
        let resolver = match TokioResolver::builder_tokio() { Ok(b) => b.build(), Err(e) => return json!({ "error": e.to_string() }) };
        let txt = |name: String| { let r = resolver.clone(); async move { r.txt_lookup(name).await.map(|t| t.iter().map(|x| x.txt_data().iter().map(|d| String::from_utf8_lossy(d).to_string()).collect::<String>()).collect::<Vec<_>>()).unwrap_or_default() } };
        // can this server reach other mail servers at all? (many hosts block port 25)
        let port25 = match tokio::time::timeout(Duration::from_secs(8), tokio::net::TcpStream::connect(("gmail-smtp-in.l.google.com", 25))).await {
            Ok(Ok(mut s)) => {
                use tokio::io::AsyncReadExt;
                let mut buf = [0u8; 200];
                let banner = match tokio::time::timeout(Duration::from_secs(8), s.read(&mut buf)).await { Ok(Ok(n)) => String::from_utf8_lossy(&buf[..n]).trim().to_string(), _ => String::new() };
                json!({ "ok": banner.starts_with("220"), "detail": if banner.is_empty() { "Connected, but Gmail said nothing".to_string() } else { banner } })
            }
            Ok(Err(e)) => json!({ "ok": false, "detail": format!("Blocked: {e}") }),
            Err(_) => json!({ "ok": false, "detail": "Blocked: timed out (the hosting company probably blocks outgoing mail)" }),
        };
        let a = resolver.ipv4_lookup(self.host.as_str()).await.map(|r| r.iter().map(|x| x.to_string()).collect::<Vec<_>>()).unwrap_or_default();
        let public = a.first().cloned().unwrap_or_default();
        let ptr = if public.is_empty() { vec![] } else { match public.parse::<std::net::IpAddr>() { Ok(ip) => resolver.reverse_lookup(ip).await.map(|r| r.iter().map(|n| n.to_utf8().trim_end_matches('.').to_string()).collect()).unwrap_or_default(), Err(_) => vec![] } };
        let spf = txt(self.domain.clone()).await.into_iter().find(|t| t.starts_with("v=spf1")).unwrap_or_default();
        let dkim = txt(format!("{SELECTOR}._domainkey.{}", self.domain)).await.join("");
        let dmarc = txt(format!("_dmarc.{}", self.domain)).await.into_iter().find(|t| t.starts_with("v=DMARC1")).unwrap_or_default();
        let mx = resolver.mx_lookup(self.domain.as_str()).await.map(|m| m.iter().map(|x| x.exchange().to_utf8().trim_end_matches('.').to_string()).collect::<Vec<_>>()).unwrap_or_default();
        let compact = |s: &str| s.replace([' ', '"'], "");
        json!({
            "port25": port25,
            "mail_host": { "name": self.host, "ips": a, "ok": !a.is_empty() },
            "reverse_dns": { "names": ptr, "ok": ptr.iter().any(|n| n.eq_ignore_ascii_case(&self.host)) },
            "spf": { "value": spf, "ok": !public.is_empty() && (spf.contains(&format!("ip4:{public}")) || spf.contains(&format!("a:{}", self.host))) },
            "dkim": { "value": dkim, "ok": compact(&dkim).contains(&self.public_b64) },
            "dmarc": { "value": dmarc, "ok": !dmarc.is_empty() },
            "mx": { "value": mx, "ok": mx.iter().any(|m| m.ends_with("mx.cloudflare.net")) },
        })
    }
}

/// A received message, as Agape keeps it.
pub struct Incoming { pub from: String, pub from_name: String, pub to: Vec<String>, pub subject: String, pub body: String, pub attachments: Vec<String>, pub files: Vec<(String, String, Vec<u8>)> }

pub fn parse(raw: &[u8], envelope_to: &str, envelope_from: &str) -> Incoming {
    let m = mail_parser::MessageParser::default().parse(raw);
    let (mut from, mut from_name) = (envelope_from.to_string(), String::new());
    let mut to: Vec<String> = vec![];
    let (mut subject, mut body, mut attachments, mut files) = (String::new(), String::new(), vec![], vec![]);
    if let Some(m) = &m {
        if let Some(a) = m.from().and_then(|f| f.first()) { if let Some(e) = a.address() { from = e.to_string(); } from_name = a.name().unwrap_or("").to_string(); }
        for list in [m.to(), m.cc()].into_iter().flatten() { for a in list.iter() { if let Some(e) = a.address() { to.push(e.to_lowercase()); } } }
        subject = m.subject().unwrap_or("").to_string();
        body = m.body_text(0).map(|b| b.to_string()).unwrap_or_default();
        for a in m.attachments() {
            let name: String = a.attachment_name().unwrap_or("attachment").chars().filter(|c| !matches!(c, '/' | '\\' | '\0')).take(120).collect();
            let ctype = a.content_type().map(|c| match c.subtype() { Some(sub) => format!("{}/{}", c.ctype(), sub), None => c.ctype().to_string() }).unwrap_or_else(|| "application/octet-stream".into());
            attachments.push(name.clone());
            files.push((name, ctype, a.contents().to_vec()));
        }
    }
    if !envelope_to.is_empty() && !to.iter().any(|t| t.eq_ignore_ascii_case(envelope_to)) { to.insert(0, envelope_to.to_lowercase()); }
    Incoming { from, from_name, to, subject: subject.chars().take(300).collect(), body: body.chars().take(100_000).collect(), attachments, files }
}

/// The Cloudflare Worker that hands each incoming message to Agape. Pasted once into Cloudflare.
pub fn worker_code(secret: &str, url: &str) -> String {
    format!(r#"// Agape mail: Cloudflare hands every message for @agapetoyourseat.com to this Worker,
// which passes it to Agape. So nothing is ever lost: if Agape can't take it, it goes to
// BACKUP (a verified address in Email Routing) if one is set, and otherwise bounces back to
// the sender, who is told to try again.
const BACKUP = ""; // e.g. "you@gmail.com" — add it under Email Routing → Destination addresses first
export default {{
  async email(message, env, ctx) {{
    const raw = await new Response(message.raw).arrayBuffer();
    let ok = false;
    for (let i = 0; i < 3 && !ok; i++) {{
      try {{
        const r = await fetch("{url}", {{
          method: "POST",
          headers: {{ "x-agape-mail-secret": "{secret}", "x-envelope-to": message.to, "x-envelope-from": message.from, "content-type": "message/rfc822" }},
          body: raw,
        }});
        ok = r.ok;
      }} catch (e) {{}}
      if (!ok) await new Promise(res => setTimeout(res, 2000 * (i + 1)));
    }}
    if (ok) return;
    if (BACKUP) {{ await message.forward(BACKUP); return; }}
    message.setReject("Agape couldn't accept this message right now. Please try again later.");
  }},
}};
"#)
}

/// What someone wrote in a reply, without the message they were replying to underneath (Gmail's "On … wrote:",
/// Outlook's "From:" block or line, lines quoted with ">").
pub fn new_part(body: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut cut = lines.len();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim();
        let next = lines.get(i + 1).map(|x| x.trim()).unwrap_or("");
        if t.starts_with('>') || t.starts_with("-----Original Message") || t.starts_with("________________________________")
            || (t.starts_with("On ") && (t.ends_with("wrote:") || next.ends_with("wrote:")))
            || (t.starts_with("From:") && i > 0 && lines[..i].iter().rev().take(2).any(|x| x.trim().is_empty())) { cut = i; break; }
    }
    let kept = lines[..cut].join("\n");
    let kept = kept.trim();
    if kept.is_empty() { body.trim().to_string() } else { kept.to_string() }
}

/// The sending service (Resend): mail from @domain goes out through it, signed for the domain with its own key, so
/// it lands in inboxes even though Agape's server isn't a known mail sender. The API key is kept only on the server.
/// Receiving doesn't change: Cloudflare still hands every message to Agape.
#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Service { pub key: String, #[serde(default)] pub since: u64 }

fn http() -> reqwest::Client { reqwest::Client::builder().timeout(Duration::from_secs(20)).build().unwrap_or_default() }

impl Service {
    pub fn load(dir: &Path) -> Option<Service> { std::fs::read(dir.join("mail-service.json")).ok().and_then(|b| serde_json::from_slice::<Service>(&b).ok()).filter(|s| !s.key.is_empty()) }
    pub fn save(&self, dir: &Path) { private_file(&dir.join("mail-service.json"), &serde_json::to_string(self).unwrap_or_default()); }
    pub fn clear(dir: &Path) { let _ = std::fs::remove_file(dir.join("mail-service.json")); }

    /// Is the key real, and is the domain verified at Resend? A key limited to sending can't list domains;
    /// that's fine, it's still a real key (the test email then shows whether the domain is ready).
    pub async fn check(&self, domain: &str) -> Result<String, String> {
        let r = http().get("https://api.resend.com/domains").bearer_auth(self.key.trim()).send().await.map_err(|e| format!("Couldn't reach Resend: {e}"))?;
        let code = r.status().as_u16();
        let j: Value = r.json().await.unwrap_or(Value::Null);
        if code == 200 {
            let d = j["data"].as_array().and_then(|a| a.iter().find(|d| d["name"].as_str() == Some(domain)).cloned());
            return match d {
                Some(d) if d["status"] == "verified" => Ok("verified".into()),
                Some(d) => Ok(d["status"].as_str().unwrap_or("pending").to_string()),
                None => Err(format!("This Resend account doesn't have {domain} yet. In Resend, add it under Domains first.")),
            };
        }
        if j["name"] == "restricted_api_key" { return Ok("unknown".into()); }
        Err(match code { 400 | 401 | 403 if code != 400 || j["message"].as_str().unwrap_or("").contains("API key") => "Resend didn't accept that API key. Copy it again from Resend → API Keys (it starts with re_).".into(), _ => format!("Resend answered {code}: {}", j["message"].as_str().unwrap_or("")) })
    }

    /// One message to one person. A refusal that won't change by trying again says "refused it for good".
    pub async fn send(&self, from: &str, to: &str, subject: &str, body: &str, reply_to: Option<&str>, files: &[(String, Vec<u8>)]) -> Result<String, String> {
        if to.parse::<Mailbox>().is_err() { return Err(format!("{to} isn't an email address")); }
        let mut m = json!({ "from": from, "to": [to], "subject": subject, "text": body });
        if !files.is_empty() {
            use base64::Engine;
            m["attachments"] = json!(files.iter().map(|(n, b)| json!({ "filename": n, "content": base64::engine::general_purpose::STANDARD.encode(b) })).collect::<Vec<_>>());
        }
        if let Some(r) = reply_to.filter(|r| r.parse::<Mailbox>().is_ok()) { m["reply_to"] = json!(r); }
        let r = http().post("https://api.resend.com/emails").bearer_auth(self.key.trim()).json(&m).send().await.map_err(|e| format!("Couldn't reach Resend yet: {e}"))?;
        let code = r.status().as_u16();
        let j: Value = r.json().await.unwrap_or(Value::Null);
        let why = j["message"].as_str().unwrap_or("").to_string();
        match code {
            200 | 201 | 202 => Ok(format!("Sent through Resend ({})", j["id"].as_str().unwrap_or("accepted"))),
            // a setup problem (the key, the domain) is fixed in Admin, then the mail goes: keep trying
            400 | 422 if why.contains("API key") || why.contains("domain") => Err(format!("Resend won't send yet: {why}")),
            400 | 422 => Err(format!("Resend refused it for good: {why}")),
            429 => Err(format!("Over Resend's sending limit for now; it will go when the limit resets. {why}")),
            401 | 403 => Err(format!("Resend won't send yet: {why}")),
            _ => Err(format!("Resend couldn't take it yet ({code}): {why}")),
        }
    }
}

/// A temporary way for Agape's alerts to reach people's phones and inboxes while its own mail can't send yet: a
/// sending account (for example a Gmail address with an app password), set by the host in Admin and kept only on the
/// server, readable by no one else. Alerts go from that account to the addresses listed; a reply goes straight to
/// whoever wrote in. Swapped out or removed in Admin at any time.
#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Relay { pub address: String, pub password: String, #[serde(default)] pub host: String, #[serde(default)] pub alert_to: Vec<String> }

impl Relay {
    pub fn load(dir: &Path) -> Option<Relay> { std::fs::read(dir.join("mail-relay.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()) }
    pub fn save(&self, dir: &Path) { private_file(&dir.join("mail-relay.json"), &serde_json::to_string(self).unwrap_or_default()); }
    pub fn clear(dir: &Path) { let _ = std::fs::remove_file(dir.join("mail-relay.json")); }
    /// The sending server: Gmail's unless another was given.
    pub fn server(&self) -> String { if self.host.trim().is_empty() { "smtp.gmail.com".into() } else { self.host.trim().into() } }

    pub async fn send(&self, to: &[String], subject: &str, body: &str, reply_to: Option<&str>) -> Result<(), String> {
        use lettre::transport::smtp::authentication::Credentials;
        let from: Mailbox = format!("Agape alerts <{}>", self.address.trim()).parse().map_err(|_| "The sending address isn't an email address".to_string())?;
        let creds = Credentials::new(self.address.trim().to_string(), self.password.replace(' ', ""));
        // port 587 first; some hosts block it, so 465 (encrypted from the start) if it can't be reached
        let tls587: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.server()).map_err(|e| e.to_string())?
            .credentials(creds.clone()).timeout(Some(Duration::from_secs(20))).build();
        let t = if tls587.test_connection().await.unwrap_or(false) { tls587 } else {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.server()).map_err(|e| e.to_string())?.credentials(creds).timeout(Some(Duration::from_secs(20))).build()
        };
        let mut last = Ok(());
        for rcpt in to {
            let Ok(tb) = rcpt.parse::<Mailbox>() else { last = Err(format!("{rcpt} isn't an email address")); continue };
            let mut b = Message::builder().from(from.clone()).to(tb).subject(subject).header(ContentType::TEXT_PLAIN);
            if let Some(r) = reply_to { if let Ok(mb) = r.parse::<Mailbox>() { b = b.reply_to(mb); } }
            let msg = b.body(body.to_string()).map_err(|e| e.to_string())?;
            if let Err(e) = t.send(msg).await { last = Err(if e.to_string().contains("535") || e.to_string().to_lowercase().contains("credentials") { "The sending account refused the password. For Gmail, use an app password, not the account password.".into() } else if e.to_string().to_lowercase().contains("connection") { format!("Couldn't reach {} to send: {e}", self.server()) } else { format!("Couldn't send: {e}") }); }
        }
        last
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_reply_keeps_only_what_was_written() {
        let gmail = "Sounds good, see you Tuesday.\n\nOn Thu, Nov 12, 2026 at 1:16 AM Kentucky via Agape <mail@agapetoyourseat.com>\nwrote:\n> Your application\n> was received.";
        assert_eq!(super::new_part(gmail), "Sounds good, see you Tuesday.");
        let one_line = "Yes.\nOn Thu, Nov 12, 2026, Sam wrote:\n> hi";
        assert_eq!(super::new_part(one_line), "Yes.");
        let outlook = "Thanks!\n\nFrom: Kentucky <ky@state.example>\nSent: Thursday\nSubject: x";
        assert_eq!(super::new_part(outlook), "Thanks!");
        assert_eq!(super::new_part("Just this."), "Just this.");
    }
}

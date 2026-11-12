// Agape Admin, at /admin/: the company's own accounts, every merchant's payment-processing
// details, and the VAR sheet printed from them for processors, venues and investors.
// Everything here comes from the server and needs an Agape Admin account; nothing is simulated.
import { React, ReactDOM, useEffect, useMemo, useRef, useState } from "./r";
import { Wordmark } from "./logo";
import qrcode from "qrcode-generator";

type Me = { send_as?: string[]; copies?: boolean; mailbox: string; domain: string; id: string; email: string; name: string; role: string; role_name: string; perms: string[]; oversees: boolean; title?: string; boss?: string | null };
type Merchant = { id: string; fields: Record<string, string>; status: Status; at: number; by: string };
type Status = "draft" | "requested" | "issued" | "live";
type Field = { k: string; l: string; hint?: string; ph?: string; opts?: string[]; wide?: boolean; long?: boolean; num?: boolean; secret?: boolean };
type Section = { id: string; title: string; sub: string; fields: Field[] };

// ---------------- what a VAR sheet holds ----------------

const YN = ["Yes", "No"];
const SECTIONS: Section[] = [
  { id: "merchant", title: "Merchant", sub: "The business that takes the payment: usually the venue or its concessionaire.", fields: [
    { k: "dba", l: "DBA name", hint: "The name on fans' card statements", ph: "Harbor Arena Concessions" },
    { k: "legal_name", l: "Legal name", ph: "As registered" },
    { k: "address", l: "Street address", wide: true },
    { k: "city", l: "City" }, { k: "state", l: "State", ph: "e.g. MA" }, { k: "zip", l: "ZIP", num: true },
    { k: "phone", l: "Business phone", num: true }, { k: "cs_phone", l: "Customer service phone", hint: "Printed on statements", num: true },
    { k: "website", l: "Website" }, { k: "email", l: "Business email" },
    { k: "time_zone", l: "Time zone", opts: ["Eastern", "Central", "Mountain", "Arizona", "Pacific", "Alaska", "Hawaii"] },
    { k: "mcc", l: "MCC", hint: "Merchant category code", ph: "5812, 5814 or 7941", num: true },
    { k: "industry", l: "Industry", opts: ["Restaurant", "Quick service", "Retail", "E-commerce", "Sports and recreation"] },
    { k: "tax_id", l: "Federal tax ID (EIN)", num: true, secret: true },
  ] },
  { id: "processor", title: "Processor and platform", sub: "Who processes the cards, and through which systems.", fields: [
    { k: "processor", l: "Processor", ph: "e.g. TSYS, Fiserv, Worldpay, Elavon, Global Payments" },
    { k: "platform", l: "Front-end platform", ph: "e.g. TSYS Sierra (Vital), Fiserv Nashville / Omaha / North" },
    { k: "acquirer", l: "Acquiring bank", ph: "The sponsor bank" },
    { k: "gateway", l: "Gateway", ph: "e.g. Authorize.net, NMI, Payeezy" },
    { k: "gateway_id", l: "Gateway merchant / account ID" },
    { k: "iso", l: "ISO or agent", ph: "Who sold the account" },
    { k: "rep", l: "Processor contact", ph: "Name, phone, email", wide: true },
  ] },
  { id: "numbers", title: "Numbers the processor issues", sub: "These come back from the processor. Leave blank until they do; the sheet shows what's still pending.", fields: [
    { k: "mid", l: "Merchant number (MID)", num: true, secret: true },
    { k: "tid", l: "Terminal ID (TID)", num: true, secret: true },
    { k: "v_number", l: "V number", hint: "TSYS only", num: true, secret: true },
    { k: "bin", l: "Acquirer BIN", num: true },
    { k: "agent", l: "Agent bank number", num: true },
    { k: "chain", l: "Chain number", num: true },
    { k: "store", l: "Store number", num: true },
    { k: "terminal", l: "Terminal number", num: true },
    { k: "location", l: "Location number", num: true },
    { k: "mvv", l: "Merchant verification value (MVV)", num: true },
    { k: "currency", l: "Currency code", ph: "840 (US dollars)", num: true },
    { k: "country", l: "Country code", ph: "840 (United States)", num: true },
    { k: "batch_close", l: "Batch close", ph: "e.g. Auto at 3:00 AM Eastern" },
  ] },
  { id: "cards", title: "Card acceptance", sub: "What the merchant can take, and how.", fields: [
    { k: "visa_mc", l: "Visa and Mastercard", opts: YN },
    { k: "discover", l: "Discover", opts: ["Yes", "No", "Through Discover direct"] },
    { k: "discover_mid", l: "Discover merchant number", num: true, secret: true },
    { k: "amex", l: "American Express", opts: ["OptBlue", "Direct (ESA)", "No"] },
    { k: "amex_se", l: "Amex SE number", num: true, secret: true },
    { k: "wallets", l: "Apple Pay and Google Pay", opts: YN },
    { k: "debit", l: "Debit", opts: ["Signature debit only", "PIN-less debit", "No"] },
    { k: "tips", l: "Tips on the transaction", opts: YN, hint: "Agape adds the runner's tip to the order" },
    { k: "cnp", l: "Card not present", opts: ["Yes (in-app)", "No"] },
    { k: "auth_capture", l: "Authorization", opts: ["Authorize at order, capture at delivery", "Sale at order"] },
  ] },
  { id: "software", title: "Software (Agape)", sub: "How Agape connects. Filled in for you; change it only if the integration is different.", fields: [
    { k: "software", l: "Software", ph: "Agape to Your Seat" },
    { k: "software_version", l: "Version", ph: "Pilot" },
    { k: "integration", l: "Integration", wide: true, long: true },
    { k: "agape_contact", l: "Agape contact", ph: "Name, phone, email", wide: true },
  ] },
  { id: "contacts", title: "Merchant contact", sub: "Who the processor and Agape talk to at the merchant.", fields: [
    { k: "contact_name", l: "Name" }, { k: "contact_title", l: "Title" },
    { k: "contact_phone", l: "Phone", num: true }, { k: "contact_email", l: "Email" },
  ] },
  { id: "notes", title: "Notes", sub: "Anything the processor or the venue should know. Never a card or bank account number.", fields: [
    { k: "notes", l: "Notes", wide: true, long: true },
  ] },
];
const ALL = SECTIONS.flatMap(s => s.fields);
const LABEL = Object.fromEntries(ALL.map(f => [f.k, f.l]));
/** What a processor needs before a merchant can take a card. */
const NEEDED = ["dba", "legal_name", "address", "city", "state", "zip", "phone", "mcc", "time_zone", "processor", "mid", "tid", "bin", "agent", "chain", "store", "terminal"];
const DEFAULTS: Record<string, string> = {
  software: "Agape to Your Seat", software_version: "Pilot", currency: "840", country: "840", visa_mc: "Yes", wallets: "Yes", tips: "Yes", cnp: "Yes (in-app)",
  auth_capture: "Authorize at order, capture at delivery", industry: "Quick service",
  integration: "Card not present, in the Agape app. Card data is entered into the gateway's hosted fields and tokenized by the gateway; Agape never receives, stores or transmits card numbers. Each order is authorized when placed and captured when it is delivered, so fans are charged only for what reaches them.",
};
const STATUS: [Status, string, string][] = [
  ["draft", "Draft", "Still filling it in"], ["requested", "Requested", "Sent to the processor, waiting for numbers"],
  ["issued", "Issued", "The processor issued the numbers"], ["live", "Live", "Taking cards"],
];

// ---------------- talking to the server ----------------

const KEY = "agape.admin.session";
let token = (() => { try { return localStorage.getItem(KEY) ?? ""; } catch { return ""; } })();
const keep = (t: string) => { token = t; try { t ? localStorage.setItem(KEY, t) : localStorage.removeItem(KEY); } catch {} };
/** Talk to Agape's server. If the request never gets an answer (a dropped connection, or the server
 *  restarting for an update) it tries again a few times before giving up, and then tells the host it
 *  happened, so a failure is never silent. A gateway error page (502/503/504) counts the same way. */
async function api(path: string, body?: unknown): Promise<any> {
  const wait = (ms: number) => new Promise(r => setTimeout(r, ms));
  let last = "", retried = false;
  // only requests that are safe to repeat are tried again: never one that could send an email twice
  const safe = body === undefined || ["signin", "accept", "read", "mute", "user", "mailbox", "setup"].includes(path);
  for (let tries = 0; tries < (safe ? 4 : 1); tries++) {
    if (tries) { retried = true; await wait(1500 * tries); }
    try {
      const r = await fetch("/api/office/" + path, { method: body === undefined ? "GET" : "POST", headers: { "content-type": "application/json", ...(token ? { authorization: "Bearer " + token } : {}) }, body: body === undefined ? undefined : JSON.stringify(body) });
      if ([502, 503, 504, 520, 521, 522, 523, 524, 530].includes(r.status)) { last = `gateway ${r.status}`; continue; }
      const j = await r.json().catch(() => ({ ok: false, error: "The server sent something unexpected. Try again in a moment." }));
      if (r.status === 401 && path !== "signin" && path !== "setup") { keep(""); }
      return { status: r.status, retried, ...j };
    } catch (e: any) { last = String(e?.message ?? e); }
  }
  if (path !== "trouble") fetch("/api/office/trouble", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ path: path.split("/")[0], what: last, tries: safe ? 4 : 1, page: location.search.includes("invite") ? "invitation" : location.hash.slice(1) }) }).catch(() => {});
  if (!safe) return { ok: false, retried, error: "Agape didn't answer, so this may or may not have gone through. Check before trying again (for an email, look in Messages for it)." };
  return { ok: false, retried, error: "Agape couldn't be reached after a few tries. It may be updating: wait a minute and try again. If it keeps happening, check your internet connection." };
}

/** Download an email attachment (it needs the signed-in session, so it can't be a plain link). */
async function download(note: string, i: number, name: string) {
  try {
    const r = await fetch(`/api/office/attachment/${note}/${i}`, { headers: { authorization: "Bearer " + token } });
    if (!r.ok) { alert("That attachment isn't available. It may have arrived before Agape kept attachments."); return; }
    const url = URL.createObjectURL(await r.blob());
    const a = document.createElement("a"); a.href = url; a.download = name; document.body.append(a); a.click(); a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 5000);
  } catch { alert("Couldn't download it. Check your connection and try again."); }
}

const when = (s: number) => new Date(s * 1000).toLocaleString(undefined, { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
const day = (d = new Date()) => d.toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" });
const mask = (v: string) => v.length <= 4 ? v : "•••• " + v.slice(-4);
const filled = (m: Record<string, string>) => NEEDED.filter(k => (m[k] ?? "").trim()).length;

// ---------------- the app ----------------

function App() {
  const [me, setMe] = useState<Me | null>(null);
  const [boot, setBoot] = useState<"loading" | "setup" | "signin" | "invite" | "in">("loading");
  const invite = new URLSearchParams(location.search).get("invite");
  useEffect(() => {
    (async () => {
      if (invite) { setBoot("invite"); return; }
      if (token) { const r = await api("me"); if (r.ok) { setMe({ ...r.me, domain: r.domain }); setBoot("in"); return; } }
      const s = await api("state");
      setBoot(s.setup ? "setup" : "signin");
    })();
  }, []);
  const enter = (t: string, m: Me) => { keep(t); setMe({ ...m, domain: m.domain || "agapetoyourseat.com" }); setBoot("in"); history.replaceState(null, "", "/admin/"); };
  if (boot === "loading") return <div className="ox-gate"><Mark /></div>;
  if (boot === "setup") return <Setup enter={enter} />;
  if (boot === "invite") return <Accept code={invite!} enter={enter} toSignin={() => { history.replaceState(null, "", "/admin/"); setBoot("signin"); }} />;
  if (boot === "signin" || !me) return <SignIn enter={enter} />;
  return <Shell me={me} out={async () => { await api("signout", {}); keep(""); setMe(null); setBoot("signin"); }} />;
}

const Mark = () => <div className="ox-mark"><Wordmark /><span>admin</span></div>;

function Gate({ title, sub, children, foot }: { title: string; sub: React.ReactNode; children: React.ReactNode; foot?: React.ReactNode }) {
  return <div className="ox-gate"><div className="ox-gate-card">
    <Mark />
    <h1>{title}</h1>
    <p className="ox-sub">{sub}</p>
    {children}
    {foot && <div className="ox-gate-foot">{foot}</div>}
  </div></div>;
}

function useForm<T extends Record<string, string>>(init: T) {
  const [v, setV] = useState<T>(init);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const bind = (k: keyof T) => ({ value: v[k], onChange: (e: any) => { setV({ ...v, [k]: e.target.value }); setError(""); } });
  const run = async (e: React.FormEvent, f: () => Promise<string | void>) => { e.preventDefault(); setBusy(true); const x = await f(); setBusy(false); if (x) setError(x); };
  const reset = (p: Partial<T>) => setV(x => ({ ...x, ...p }));
  return { v, bind, error, setError, busy, run, reset };
}

function SignIn({ enter }: { enter: (t: string, m: Me) => void }) {
  const f = useForm({ email: "", password: "" });
  return <Gate title="Sign in" sub="Agape Admin is for Agape's team: merchants, payment processing and VAR sheets.">
    <form onSubmit={e => f.run(e, async () => { const r = await api("signin", f.v); if (r.ok) enter(r.token, r.me); else return r.error; })}>
      <label className="ox-f"><span>Email</span><input type="email" autoComplete="username" autoFocus required {...f.bind("email")} /></label>
      <label className="ox-f"><span>Password</span><input type="password" autoComplete="current-password" required {...f.bind("password")} /></label>
      {f.error && <p className="ox-err" role="alert">{f.error}</p>}
      <button className="ox-btn primary block" disabled={f.busy}>{f.busy ? "Signing in…" : "Sign in"}</button>
    </form>
    <p className="ox-fine">No account? An owner invites you by email address. Forgot your password? Ask an owner to turn your access off and invite you again.</p>
  </Gate>;
}

function Setup({ enter }: { enter: (t: string, m: Me) => void }) {
  const f = useForm({ code: "", name: "", email: "", password: "", again: "" });
  return <Gate title="Set up Agape Admin" sub={<>You'll be its first owner. The setup code is on the server, so only someone who runs it can do this: <code>cat /opt/agape-data/admin-setup-code</code></>}>
    <form onSubmit={e => f.run(e, async () => {
      if (f.v.password !== f.v.again) return "The two passwords don't match";
      const r = await api("setup", f.v); if (r.ok) enter(r.token, r.me); else return r.error;
    })}>
      <label className="ox-f"><span>Setup code</span><input autoFocus required className="mono" autoComplete="off" {...f.bind("code")} /></label>
      <label className="ox-f"><span>Your name</span><input required autoComplete="name" {...f.bind("name")} /></label>
      <label className="ox-f"><span>Email</span><input type="email" required autoComplete="username" {...f.bind("email")} /></label>
      <label className="ox-f"><span>Password</span><input type="password" required minLength={10} autoComplete="new-password" {...f.bind("password")} /><small>At least 10 characters</small></label>
      <label className="ox-f"><span>Password again</span><input type="password" required autoComplete="new-password" {...f.bind("again")} /></label>
      {f.error && <p className="ox-err" role="alert">{f.error}</p>}
      <button className="ox-btn primary block" disabled={f.busy}>Create the owner account</button>
    </form>
  </Gate>;
}

function Accept({ code, enter, toSignin }: { code: string; enter: (t: string, m: Me) => void; toSignin: () => void }) {
  const [inv, setInv] = useState<{ email: string; name: string; role_name: string; boss_name: string } | null | "gone">(null);
  const f = useForm({ name: "", password: "", again: "" });
  useEffect(() => { api("invitation/" + encodeURIComponent(code)).then(r => { if (r.ok) { setInv(r.invite); f.bind("name").onChange({ target: { value: r.invite.name } }); } else setInv("gone"); }); }, []);
  if (inv === null) return <div className="ox-gate"><Mark /></div>;
  if (inv === "gone") return <Gate title="This invitation can't be used" sub="It has expired or was already used. Ask the person who invited you for a new one." foot={<button className="ox-link" onClick={toSignin}>Sign in instead</button>}><></></Gate>;
  return <Gate title="Join Agape Admin" sub={<>You've been invited as {inv.role_name}, reporting to {inv.boss_name}, with <b>{inv.email}</b>. Choose a password to finish.</>}>
    <form onSubmit={e => f.run(e, async () => {
      if (f.v.password !== f.v.again) return "The two passwords don't match";
      const r = await api("accept", { code, name: f.v.name, password: f.v.password });
      if (r.ok) { enter(r.token, r.me); return; }
      // if an earlier try got through but its answer was lost, the account already exists: sign in instead
      if (r.retried && /already used/.test(r.error ?? "")) { const s = await api("signin", { email: inv.email, password: f.v.password }); if (s.ok) { enter(s.token, s.me); return; } }
      return r.error;
    })}>
      <label className="ox-f"><span>Your name</span><input required autoComplete="name" {...f.bind("name")} /></label>
      <label className="ox-f"><span>Password</span><input type="password" required minLength={10} autoFocus autoComplete="new-password" {...f.bind("password")} /><small>At least 10 characters</small></label>
      <label className="ox-f"><span>Password again</span><input type="password" required autoComplete="new-password" {...f.bind("again")} /></label>
      {f.error && <p className="ox-err" role="alert">{f.error}</p>}
      <button className="ox-btn primary block" disabled={f.busy}>Join</button>
    </form>
  </Gate>;
}

// ---------------- signed in ----------------

type Tab = "room" | "brand" | "mail" | "channels" | "inbox" | "merchants" | "venues" | "team" | "leads" | "accounts" | "activity" | "account";

function Shell({ me, out }: { me: Me; out: () => void }) {
  const [tab, setTab] = useState<Tab>(() => (location.hash.slice(1) as Tab) || "merchants");
  const [toast, setToast] = useState("");
  useEffect(() => { history.replaceState(null, "", "#" + tab); }, [tab]);
  useEffect(() => { const go = (e: Event) => setTab((e as CustomEvent).detail as Tab); addEventListener("ox-go", go); return () => removeEventListener("ox-go", go); }, []);
  useEffect(() => { if (!toast) return; const t = setTimeout(() => setToast(""), 3200); return () => clearTimeout(t); }, [toast]);
  const [unread, setUnread] = useState(0);
  const [pings, setPings] = useState(0);
  const counts = useMemo(() => ({ last: -1 }), []);
  const refresh = () => api("me").then(r => {
    if (!r.ok) return;
    setUnread(r.unread ?? 0); setPings(r.pings ?? 0);
    const total = (r.unread ?? 0) + (r.pings ?? 0);
    if (counts.last >= 0 && total > counts.last) setToast((r.unread ?? 0) > 0 && total - counts.last <= (r.unread ?? 0) ? "New message for you" : "New activity in your channels");
    counts.last = total;
    document.title = total ? `(${total}) Agape Admin` : "Agape Admin";
  });
  useEffect(() => { refresh(); const t = setInterval(refresh, 20000); return () => clearInterval(t); }, []);
  const tabs: [Tab, string][] = [["inbox", "Messages"], ...(me.oversees ? [["channels", "Channels"]] as [Tab, string][] : []), ...(me.perms.includes("venues") ? [["venues", "Venues"]] as [Tab, string][] : []), ...(me.perms.includes("merchants_view") ? [["merchants", "Processing"]] as [Tab, string][] : []), ["team", "Team"], ...(me.perms.includes("leads") ? [["leads", "Leads"]] as [Tab, string][] : []), ...(me.perms.includes("accounts") ? [["accounts", "App accounts"]] as [Tab, string][] : []), ["activity", "Activity"], ...(me.role === "host" || me.perms.includes("leads") ? [["room", "Investor room"]] as [Tab, string][] : []), ["brand", "Brand"], ...(me.role === "host" ? [["mail", "Email setup"]] as [Tab, string][] : []), ["account", "Your account"]];
  return <div className="ox-app">
    <header className="ox-top">
      <Mark />
      <nav className="ox-nav" aria-label="Sections">{tabs.map(([k, l]) => <button key={k} className={tab === k ? "on" : ""} aria-current={tab === k ? "page" : undefined} onClick={() => setTab(k)}>{l}{k === "inbox" && unread > 0 && <i className="ox-badge" aria-label={`${unread} unread`}>{unread}</i>}{k === "channels" && pings > 0 && <i className="ox-badge soft" aria-label={`${pings} new`}>{pings}</i>}</button>)}</nav>
      <div className="ox-who"><span>{me.name}<small>{me.role_name}</small></span><button className="ox-btn ghost sm" onClick={out}>Sign out</button></div>
    </header>
    <main className="ox-main">
      {tab === "inbox" && <Inbox me={me} toast={setToast} seen={refresh} />}
      {tab === "channels" && <Channels me={me} seen={refresh} />}
      {tab === "venues" && <Venues me={me} toast={setToast} />}
      {tab === "merchants" && <Merchants me={me} toast={setToast} />}
      {tab === "team" && <Team me={me} toast={setToast} />}
      {tab === "leads" && <Leads />}
      {tab === "activity" && <Activity me={me} />}
      {tab === "accounts" && <Accounts />}
      {tab === "brand" && <Brand />}
      {tab === "room" && <InvestorRoom me={me} toast={setToast} />}
      {tab === "mail" && <MailSetup toast={setToast} />}
      {tab === "account" && <Account me={me} toast={setToast} out={out} />}
    </main>
    {toast && <div className="ox-toast" role="status">{toast}</div>}
  </div>;
}

// ---------------- investor room ----------------

type RItem = { id: string; title: string; about: string; kind: "file" | "link" | "text"; url: string; file: string; fname: string; size: number; text: string; hidden: boolean; at: number };
type RGuest = { id: string; name: string; firm: string; email: string; key: string; at: number; off: boolean; views: [number, string][] };

/** The investor room: who has a link and what they've opened, and what's in it. */
function InvestorRoom({ me, toast }: { me: Me; toast: (s: string) => void }) {
  const [d, setD] = useState<{ items: RItem[]; guests: RGuest[] } | null>(null);
  const [g, setG] = useState({ name: "", firm: "", email: "" }); const [gErr, setGErr] = useState("");
  const [edit, setEdit] = useState<Partial<RItem> | null>(null); const [eErr, setEErr] = useState(""); const [busy, setBusy] = useState(false);
  const load = () => api("room").then(r => r.ok ? setD(r.room) : toast(r.error));
  useEffect(() => { load(); }, []);
  if (!d) return <><Head title="Investor room" /><p className="ox-empty">Loading…</p></>;
  const link = (k: string) => `${location.origin}/room/?k=${k}`;
  const title = (id: string) => id === "room" ? "the room" : d.items.find(i => i.id === id)?.title ?? "something since removed";
  const copy = (t: string) => navigator.clipboard?.writeText(t).then(() => toast("Link copied."));
  const guest = async (b: object, ok: string) => { const r = await api("room/guest", b); if (r.ok) { toast(ok); load(); } else toast(r.error); };
  const item = async (b: object, ok?: string) => { const r = await api("room/item", b); if (r.ok) { if (ok) toast(ok); load(); return ""; } return r.error as string; };
  const openFile = async (i: RItem) => {
    const w = window.open("", "_blank");
    try { const r = await fetch(`/api/office/room/open/${i.id}`, { headers: { authorization: "Bearer " + token } }); if (!r.ok) throw 0; const u = URL.createObjectURL(await r.blob()); if (w) w.location.href = u; else location.href = u; } catch { w?.close(); toast("Couldn't open it."); }
  };
  const upload = (id?: string) => { const inp = document.createElement("input"); inp.type = "file"; inp.onchange = async () => {
    const f = inp.files?.[0]; if (!f) return; if (f.size > 30e6) { toast("Files up to 30 MB."); return; }
    setBusy(true); toast(`Uploading ${f.name}…`);
    const data = await new Promise<string>(res => { const fr = new FileReader(); fr.onload = () => res(String(fr.result).split(",")[1] ?? ""); fr.readAsDataURL(f); });
    const r = await api("room/file", { id: id ?? "", name: f.name, title: id ? "" : f.name.replace(/\.[^.]+$/, ""), data }); setBusy(false);
    toast(r.ok ? (id ? "New version uploaded." : "Uploaded. It's in the room now.") : r.error); load();
  }; inp.click(); };
  const saveEdit = async (e: React.FormEvent) => { e.preventDefault(); if (!edit) return; setEErr(""); const err = await item({ ...edit, hidden: !!edit.hidden }, "Saved."); if (err) setEErr(err); else setEdit(null); };
  const ago = (t: number) => when(t);
  return <>
    <Head title="Investor room" sub="A private page for investors at agapetoyourseat.com/room: each gets their own link, so you see who opened it and what they read, and can turn a link off at any time. You'll get a notice (and an email, if alerts are on) the first time each one opens it." />
    <section className="ox-card">
      <h2>Who has a link</h2>
      <form className="ox-invite" onSubmit={async e => { e.preventDefault(); setGErr(""); const r = await api("room/guest", g); if (!r.ok) { setGErr(r.error); return; } copy(link(r.key)); setG({ name: "", firm: "", email: "" }); load(); }}>
        <label className="ox-f"><span>Who it's for</span><input required value={g.name} onChange={e => setG({ ...g, name: e.target.value })} placeholder="Jane Smith" /></label>
        <label className="ox-f"><span>Firm</span><input value={g.firm} onChange={e => setG({ ...g, firm: e.target.value })} placeholder="Bluegrass Angels" /></label>
        <label className="ox-f"><span>Their email</span><input type="email" value={g.email} onChange={e => setG({ ...g, email: e.target.value })} placeholder="optional" /></label>
        <button className="ox-btn primary">Make a link</button>
      </form>
      {gErr && <p className="ox-err">{gErr}</p>}
      {!d.guests.length ? <p className="ox-empty">Nobody yet. Make a link for each investor, so you can see who opened what.</p> : <div className="rmx-guests">{[...d.guests].reverse().map(x => {
        const opened = x.views.filter(v => v[1] !== "room"); const last = x.views.length ? x.views[x.views.length - 1][0] : 0;
        return <div key={x.id} className={"rmx-guest" + (x.off ? " off" : "")}>
          <div className="rmx-who"><b>{x.name}</b>{x.firm && <small>{x.firm}</small>}<span className={x.views.length ? "seen" : ""}>{x.off ? "Link turned off" : x.views.length ? `Last opened ${ago(last)}` : "Not opened yet"}</span></div>
          {opened.length > 0 && <ul className="rmx-views">{[...opened].reverse().slice(0, 8).map(([t, id], i) => <li key={i}><b>{title(id)}</b> · {ago(t)}</li>)}</ul>}
          <div className="ox-row">
            <button className="ox-btn sm" onClick={() => copy(link(x.key))}>Copy link</button>
            {x.email && <button className="ox-btn sm" onClick={() => { pendingDraft = { to: [x.email], subject: "Agape: your investor room", body: `Hi ${x.name.split(" ")[0]},\n\nHere's a private room with everything on Agape: the two-page summary, our paper on where Agape goes, and the live product you can try yourself.\n\n${link(x.key)}\n\nThe link is just for you. Any questions, reply here.\n\n${me.name}` }; window.dispatchEvent(new CustomEvent("ox-go", { detail: "inbox" })); }}>Email it</button>}
            <button className="ox-btn sm ghost" onClick={() => guest({ id: x.id, off: !x.off }, x.off ? "Link turned back on." : "Link turned off. It stops working right away.")}>{x.off ? "Turn on" : "Turn off"}</button>
            <button className="ox-btn sm ghost" onClick={() => { if (confirm(`Remove ${x.name}'s link and its history?`)) guest({ id: x.id, delete: true }, "Removed."); }}>Remove</button>
          </div>
        </div>; })}</div>}
    </section>
    <section className="ox-card">
      <div className="ox-row between"><h2>What's in it</h2><div className="ox-row"><button className="ox-btn sm" disabled={busy} onClick={() => upload()}>Upload a file</button><button className="ox-btn sm" onClick={() => setEdit({ kind: "link", title: "", about: "", url: "", hidden: false })}>Add a link</button><button className="ox-btn sm" onClick={() => setEdit({ kind: "text", title: "", about: "", text: "", hidden: false })}>Write a note</button></div></div>
      <p className="ox-sub">Investors see these in this order. Hidden ones stay here for you to finish first.</p>
      {edit && !edit.id && <ItemForm edit={edit} setEdit={setEdit} save={saveEdit} err={eErr} />}
      <ol className="rmx-items">{d.items.map((i, n) => <li key={i.id} className={i.hidden ? "hid" : ""}>
        {edit && edit.id === i.id ? <ItemForm edit={edit} setEdit={setEdit} save={saveEdit} err={eErr} /> : <>
          <div className="rmx-item"><i className="rmx-kind">{i.kind === "file" ? (i.fname.split(".").pop() || "file").toUpperCase() : i.kind === "link" ? "LINK" : "NOTE"}</i>
            <div><b>{i.title}</b>{i.hidden && <span className="rmx-hid">Hidden from investors</span>}<small>{i.about}</small>{i.kind === "link" && <small className="mono">{i.url}</small>}</div></div>
          <div className="ox-row">
            <button className="ox-btn sm ghost" aria-label="Move up" disabled={n === 0} onClick={() => item({ id: i.id, move: -1 })}>↑</button>
            <button className="ox-btn sm ghost" aria-label="Move down" disabled={n === d.items.length - 1} onClick={() => item({ id: i.id, move: 1 })}>↓</button>
            {i.kind === "file" && <button className="ox-btn sm" onClick={() => openFile(i)}>Open</button>}
            {i.kind === "file" && <button className="ox-btn sm" disabled={busy} onClick={() => upload(i.id)}>New version</button>}
            <button className="ox-btn sm" onClick={() => { setEErr(""); setEdit({ ...i }); }}>Edit</button>
            <button className="ox-btn sm ghost" onClick={() => item({ ...i, hidden: !i.hidden }, i.hidden ? "Investors can see it now." : "Hidden from investors.")}>{i.hidden ? "Show" : "Hide"}</button>
            <button className="ox-btn sm ghost" onClick={() => { if (confirm(`Remove "${i.title}" from the room?`)) item({ id: i.id, delete: true }, "Removed."); }}>Remove</button>
          </div></>}
      </li>)}</ol>
    </section>
  </>;
}
function ItemForm({ edit, setEdit, save, err }: { edit: Partial<RItem>; setEdit: (e: Partial<RItem> | null) => void; save: (e: React.FormEvent) => void; err: string }) {
  return <form className="ox-grid rmx-form" onSubmit={save}>
    <label className="ox-f"><span>Title</span><input required value={edit.title ?? ""} onChange={e => setEdit({ ...edit, title: e.target.value })} /></label>
    <label className="ox-f"><span>One line about it</span><input value={edit.about ?? ""} onChange={e => setEdit({ ...edit, about: e.target.value })} /></label>
    {edit.kind === "link" && <label className="ox-f wide"><span>Link</span><input required value={edit.url ?? ""} onChange={e => setEdit({ ...edit, url: e.target.value })} placeholder="https://… or /demo/" /></label>}
    {edit.kind === "text" && <label className="ox-f wide"><span>Note</span><textarea rows={12} value={edit.text ?? ""} onChange={e => setEdit({ ...edit, text: e.target.value })} /><small>A line starting with "## " is a heading, "- " a bullet, **words** in bold. A blank line starts a new paragraph.</small></label>}
    <label className="ox-check wide"><input type="checkbox" checked={!!edit.hidden} onChange={e => setEdit({ ...edit, hidden: e.target.checked })} /> Hidden from investors</label>
    {err && <p className="ox-err wide">{err}</p>}
    <div className="ox-row wide"><button className="ox-btn primary">Save</button><button type="button" className="ox-btn ghost" onClick={() => setEdit(null)}>Cancel</button></div>
  </form>;
}

// ---------------- brand ----------------

type BrandImg = { file: string; caption: string };
type BrandRound = { id: string; n: number; date: string; title: string; intro: string; kind: "logo" | "look"; groups: { title: string; intro: string; images: BrandImg[] }[] };
type BrandMan = { final: BrandImg[]; themes: BrandImg; kit: { file: string; size: number }; rounds: BrandRound[] };

/** A brand file needs the signed-in session, so it's fetched and shown from memory; and only once it scrolls near. */
const brandCache = new Map<string, Promise<string>>();
function brandUrl(file: string) {
  if (!brandCache.has(file)) brandCache.set(file, fetch(`/api/office/brand/${file}`, { headers: { authorization: "Bearer " + token } })
    .then(r => { if (!r.ok) throw new Error(); return r.blob(); }).then(b => URL.createObjectURL(b)).catch(e => { brandCache.delete(file); throw e; }));
  return brandCache.get(file)!;
}
function BrandPic({ img, onOpen, eager }: { img: BrandImg; onOpen?: () => void; eager?: boolean }) {
  const [src, setSrc] = useState(""); const [bad, setBad] = useState(false); const ref = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const go = () => brandUrl(img.file).then(setSrc).catch(() => setBad(true));
    if (eager || !("IntersectionObserver" in window)) { go(); return; }
    const io = new IntersectionObserver(es => { if (es.some(e => e.isIntersecting)) { io.disconnect(); go(); } }, { rootMargin: "600px" });
    if (ref.current) io.observe(ref.current); return () => io.disconnect();
  }, [img.file]);
  return <button ref={ref} className="bx-pic" onClick={onOpen} aria-label={`Open ${img.caption}`}>
    {src ? <img src={src} alt="" /> : <span className="bx-wait">{bad ? "Couldn't load this one" : ""}</span>}
    <small>{img.caption}</small>
  </button>;
}
function Brand() {
  const [m, setM] = useState<BrandMan | null>(null); const [error, setError] = useState("");
  const [view, setView] = useState<{ list: BrandImg[]; i: number } | null>(null);
  useEffect(() => { api("brand").then(j => j.ok === false ? setError(j.error) : setM(j)); }, []);
  useEffect(() => {
    if (!view) return;
    const k = (e: KeyboardEvent) => { if (e.key === "Escape") setView(null); if (e.key === "ArrowRight") setView(v => v && { ...v, i: (v.i + 1) % v.list.length }); if (e.key === "ArrowLeft") setView(v => v && { ...v, i: (v.i + v.list.length - 1) % v.list.length }); };
    addEventListener("keydown", k); return () => removeEventListener("keydown", k);
  }, [!!view]);
  const kit = async () => { try { const url = await brandUrl(m!.kit.file); const a = document.createElement("a"); a.href = url; a.download = "Agape-brand-kit.zip"; document.body.append(a); a.click(); a.remove(); } catch { alert("Couldn't download the kit. Try again in a moment."); } };
  if (error) return <><Head title="Brand" /><p className="ox-sub">{error}</p></>;
  if (!m) return <><Head title="Brand" /><p className="ox-sub">Loading…</p></>;
  const round = (r: BrandRound) => { const all = r.groups.flatMap(g => g.images); return <details key={r.id} className="bx-round">
    <summary><span className="bx-n">{r.n}</span><b>{r.title}</b><small>{r.date} · {all.length} {all.length === 1 ? "board" : "boards"}</small></summary>
    {r.intro && <p className="ox-sub">{r.intro}</p>}
    {r.groups.map((g, gi) => <div key={gi} className="bx-group">{g.title && <h3>{g.title}</h3>}{g.intro && <p className="ox-sub">{g.intro}</p>}
      <div className="bx-grid">{g.images.map(img => <BrandPic key={img.file} img={img} onOpen={() => setView({ list: all, i: all.indexOf(img) })} />)}</div></div>)}
  </details>; };
  const cur = view && view.list[view.i];
  return <>
    <Head title="Brand" sub="Everything designed so far for how Agape looks: what's decided, the app's looks, and every round of exploration in the order it happened. Only people signed in to Agape Admin can see this page.">
      <button className="btn" onClick={kit}>Download the brand kit ({Math.round(m.kit.size / 1024)} KB)</button>
    </Head>
    <section className="bx-sec"><div className="bx-tag ok">Decided</div><h2>The logo and app icon</h2>
      <p className="ox-sub">The Evolved Classic wordmark (condensed, narrow cut, no symbol) with a quiet stacked "to your seat" beside it. The app icon is the "a" with the stacked tagline, which becomes three bars at small sizes. Agape crimson #B8174F.</p>
      <div className="bx-final">{m.final.map(img => <BrandPic key={img.file} img={img} eager onOpen={() => setView({ list: m.final, i: m.final.indexOf(img) })} />)}</div></section>
    <section className="bx-sec"><div className="bx-tag ok">Built into the app</div><h2>App looks</h2><p className="ox-sub">{m.themes.caption}</p>
      <div className="bx-wide"><BrandPic img={m.themes} onOpen={() => setView({ list: [m.themes], i: 0 })} /></div></section>
    <section className="bx-sec"><div className="bx-tag ok">Decided</div><h2>Logo rounds</h2><p className="ox-sub">How the logo and icon were chosen.</p>{m.rounds.filter(r => r.kind === "logo").map(round)}</section>
    <section className="bx-sec"><div className="bx-tag">Exploring · on hold</div><h2>Brand look rounds</h2><p className="ox-sub">A later, wider exploration of colour, lettering and the overall look. Nothing here is decided yet.</p>{m.rounds.filter(r => r.kind === "look").map(round)}</section>
    {cur && <div className="bx-view" role="dialog" aria-label={cur.caption} onClick={() => setView(null)}>
      <div className="bx-view-in" onClick={e => e.stopPropagation()}>
        <div className="bx-view-top"><b>{cur.caption}</b><span>{view!.list.length > 1 ? `${view!.i + 1} of ${view!.list.length}` : ""}</span><button className="btn sm" onClick={() => setView(null)}>Close</button></div>
        <div className="bx-view-img"><BrandPic key={cur.file} img={cur} eager /></div>
        {view!.list.length > 1 && <div className="bx-view-nav"><button className="btn" onClick={() => setView({ ...view!, i: (view!.i + view!.list.length - 1) % view!.list.length })}>← Previous</button><button className="btn" onClick={() => setView({ ...view!, i: (view!.i + 1) % view!.list.length })}>Next →</button></div>}
      </div></div>}
  </>;
}

function Head({ title, sub, children }: { title: string; sub?: React.ReactNode; children?: React.ReactNode }) {
  return <div className="ox-head"><div><h1>{title}</h1>{sub && <p className="ox-sub">{sub}</p>}</div>{children && <div className="ox-acts">{children}</div>}</div>;
}

function StatusTag({ s }: { s: Status }) { return <span className={"ox-tag " + s}>{STATUS.find(x => x[0] === s)?.[1] ?? s}</span>; }

/** A VAR sheet with nothing filled in but Agape's own software details, to print and fill in by hand. */
const BLANK = (): Merchant => ({ id: "blank", fields: { software: DEFAULTS.software, software_version: DEFAULTS.software_version, integration: DEFAULTS.integration }, status: "draft", at: Math.floor(Date.now() / 1000), by: "" });

function Merchants({ me, toast }: { me: Me; toast: (s: string) => void }) {
  const [list, setList] = useState<Merchant[] | null>(null);
  const [venues, setVenues] = useState<{ id: string; name: string }[]>([]);
  const [open, setOpen] = useState<{ m: Merchant | null; sheet: boolean } | null>(null);
  const [error, setError] = useState("");
  const load = async () => { const r = await api("merchants"); if (r.ok) { setList(r.merchants); setVenues(r.venues); } else setError(r.error); };
  const [brought, setBrought] = useState<string | null>(null);
  useEffect(() => { load(); }, []);
  const can = me.perms.includes("merchants_edit");
  if (brought != null) return <Brought pick={brought} back={() => setBrought(null)} />;
  if (open?.sheet && open.m) return <Sheet m={open.m} back={open.m.id === "blank" ? () => setOpen(null) : () => setOpen({ m: open.m, sheet: false })} close={() => setOpen(null)} />;
  if (open) return <Editor m={open.m} can={can} owner={me.perms.includes("merchants_delete")} venues={venues} toast={toast}
    saved={m => { load(); setOpen({ m, sheet: false }); }} sheet={m => setOpen({ m, sheet: true })} close={() => { setOpen(null); load(); }} />;
  return <>
    <Head title="Processing" sub="Local businesses Agape processes cards for, as their agent: each one's processor, the numbers it issued, and the VAR sheet that sets Agape up on their account. Businesses Agape pays after event nights are under Venues.">
      <button className="ox-btn" onClick={() => setBrought("")}>What Agape brought them</button>
      <button className="ox-btn" onClick={() => setOpen({ m: BLANK(), sheet: true })}>Blank VAR sheet</button>
      {can && <button className="ox-btn primary" onClick={() => setOpen({ m: null, sheet: false })}>Add a merchant</button>}
    </Head>
    {error && <p className="ox-err">{error}</p>}
    {list === null ? <p className="ox-empty">Loading…</p> : !list.length ? <div className="ox-blank">
      <h2>No local businesses yet</h2>
      <p>Start with a business fans already visit through Agape: "What Agape brought them" shows the customers Agape has sent it, which is the case for processing with Agape. Add the first one with what you know today: the business, its processor, and how cards are taken. The numbers the processor issues can come later, and the VAR sheet shows what's still pending.</p>
      {can && <button className="ox-btn primary" onClick={() => setOpen({ m: null, sheet: false })}>Add a merchant</button>}
    </div> : <div className="ox-table-wrap"><table className="ox-table merch">
      <thead><tr><th>Merchant</th><th>Processor</th><th>MID</th><th>Ready</th><th>Status</th><th>Updated</th><th></th></tr></thead>
      <tbody>{list.map(m => { const n = filled(m.fields); return <tr key={m.id}>
        <td><button className="ox-link strong" onClick={() => setOpen({ m, sheet: false })}>{m.fields.dba}</button>{m.fields.city && <small>{m.fields.city}{m.fields.state ? ", " + m.fields.state : ""}</small>}</td>
        <td data-l="Processor">{m.fields.processor || <span className="ox-muted">Not chosen</span>}</td>
        <td className="mono nowrap" data-l="MID">{m.fields.mid ? mask(m.fields.mid) : <span className="ox-muted">Pending</span>}</td>
        <td data-l="Ready"><span className="ox-meter" title={`${n} of ${NEEDED.length} required fields`}><i style={{ width: `${(n / NEEDED.length) * 100}%` }} /></span><small>{n}/{NEEDED.length}</small></td>
        <td data-l="Status"><StatusTag s={m.status} /></td>
        <td data-l="Updated"><small>{when(m.at)} · {m.by}</small></td>
        <td className="nowrap"><button className="ox-btn sm" onClick={() => setOpen({ m, sheet: true })}>VAR sheet</button> <button className="ox-btn sm ghost" onClick={() => setBrought(m.fields.dba ?? "")}>Brought</button></td>
      </tr>; })}</tbody>
    </table></div>}
  </>;
}

/** What Agape brought a local business: fans who unlocked its offer, fans who came in, and what it sold through Agape,
 * night by night. Printable, to hand them when asking to process their cards. Simulated nights are marked and never
 * counted in the totals of a real business. */
function Brought({ pick, back }: { pick: string; back: () => void }) {
  const [data, setData] = useState<any>(null);
  const [error, setError] = useState("");
  useEffect(() => { (async () => { const r = await api("brought"); if (r.ok) setData(r); else setError(r.error); })(); }, []);
  const names: string[] = useMemo(() => Array.from(new Set<string>((data?.nights ?? []).flatMap((n: any) => n.businesses.map((b: any) => b.name)))).sort(), [data]);
  const [who, setWho] = useState(pick);
  useEffect(() => { if (data && !names.includes(who)) setWho(names.find(n => pick && n.toLowerCase().includes(pick.toLowerCase().split(" ")[0])) ?? names[0] ?? ""); }, [data]);
  if (error) return <p className="ox-err">{error}</p>;
  if (!data) return <p className="ox-empty">Loading…</p>;
  const rows = data.nights.map((n: any) => ({ n, b: n.businesses.find((b: any) => b.name === who) })).filter((x: any) => x.b);
  const real = rows.filter((x: any) => !x.n.simulated), shown = real.length ? real : rows;
  const sum = (k: string) => shown.reduce((a: number, x: any) => a + (x.b[k] ?? 0), 0);
  const sample = !real.length && rows.length > 0;
  return <>
    <Head title="What Agape brought them" sub="The customers Agape has sent a local business, from the books of every night. Print it or save it as a PDF to hand them.">
      <button className="ox-btn ghost" onClick={back}>Back</button>
      <button className="ox-btn primary" onClick={() => window.print()}>Print</button>
    </Head>
    {!names.length ? <div className="ox-blank"><h2>No local businesses on Agape yet</h2><p>When a venue switches on partners nearby, every perk unlocked and every visit is recorded here, night by night.</p></div> : <>
      <label className="ox-f no-print"><span>Business</span><select value={who} onChange={e => setWho(e.target.value)}>{names.map(n => <option key={n}>{n}</option>)}</select></label>
      <article className="ox-card brought">
        <header><Wordmark tag /><div><h1>{who}</h1><p>{sample ? "A sample from the demo's simulated night: no real business's numbers yet" : `${real.length} night${real.length === 1 ? "" : "s"} with Agape`}</p></div></header>
        <div className="brought-kpis">
          <div><b>{sum("fans")}</b><span>different fans Agape sent your way</span></div>
          <div><b>{sum("unlocked")}</b><span>unlocked your offer during an event</span></div>
          <div><b>{sum("came")}</b><span>came in and used it</span></div>
          <div><b>{sum("unlocked") ? Math.round((sum("came") / sum("unlocked")) * 100) : 0}%</b><span>of those who unlocked it came in</span></div>
        </div>
        {sum("sales") > 0 && <p className="brought-sales">{money(sum("sales"))} sold through Agape across {sum("orders")} orders.</p>}
        <table className="ox-table tight"><thead><tr><th>Night</th><th>Venue</th><th>Unlocked</th><th>Came in</th><th>Fans</th></tr></thead>
          <tbody>{rows.map((x: any, i: number) => <tr key={i}><td><small>{when(x.n.at)}</small>{x.n.simulated && <span className="ox-tag draft">Simulated</span>}{x.n.open && <span className="ox-tag live">Tonight</span>}</td><td>{x.n.venue}</td><td className="mono">{x.b.unlocked}</td><td className="mono">{x.b.came}</td><td className="mono">{x.b.fans}</td></tr>)}</tbody></table>
        <p className="brought-ask">Every card these customers paid you with went through another company. Process with Agape, and the same fans keep coming, with the marketing and support of people who know your business.</p>
      </article>
    </>}
  </>;
}

function Editor({ m, can, owner, venues, toast, saved, sheet, close }: { m: Merchant | null; can: boolean; owner: boolean; venues: { id: string; name: string }[]; toast: (s: string) => void; saved: (m: Merchant) => void; sheet: (m: Merchant) => void; close: () => void }) {
  const start = useMemo(() => m ? { ...m.fields } : { ...DEFAULTS }, [m?.id, m?.at]);
  const [f, setF] = useState<Record<string, string>>(start);
  const [status, setStatus] = useState<Status>(m?.status ?? "draft");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [sure, setSure] = useState(false);
  useEffect(() => { setF(start); setStatus(m?.status ?? "draft"); }, [start]);
  const changed = JSON.stringify(f) !== JSON.stringify(start) || status !== (m?.status ?? "draft");
  useEffect(() => { const h = (e: BeforeUnloadEvent) => { if (changed) { e.preventDefault(); e.returnValue = ""; } }; addEventListener("beforeunload", h); return () => removeEventListener("beforeunload", h); }, [changed]);
  const save = async () => {
    setBusy(true); setError("");
    const r = await api("merchant", { id: m?.id, fields: f, status });
    setBusy(false);
    if (r.ok) { toast("Saved."); saved(r.merchant); } else setError(r.error);
  };
  const missing = NEEDED.filter(k => !(f[k] ?? "").trim());
  const leave = () => { if (changed && !confirm("Leave without saving your changes?")) return; close(); };
  return <>
    <Head title={m ? f.dba || "Merchant" : "Add a merchant"} sub={m ? <>Last changed {when(m.at)} by {m.by}.</> : "Fill in what you know. Only the DBA name is needed to save."}>
      <button className="ox-btn ghost" onClick={leave}>Back to merchants</button>
      {m && <button className="ox-btn" disabled={changed} title={changed ? "Save first" : undefined} onClick={() => sheet(m)}>VAR sheet</button>}
    </Head>
    {!can && <p className="ox-note">You can view this merchant and print its VAR sheet. Ask an owner if you need to make changes.</p>}
    <div className="ox-edit">
      <div className="ox-sections">
        {SECTIONS.map(sec => <section key={sec.id} className="ox-card">
          <h2>{sec.title}</h2><p className="ox-sub">{sec.sub}</p>
          <div className="ox-grid">
            {sec.id === "merchant" && venues.length > 0 && <label className="ox-f"><span>Agape venue</span><select disabled={!can} value={f.venue ?? ""} onChange={e => setF({ ...f, venue: e.target.value })}><option value="">Not linked</option>{venues.map(v => <option key={v.id} value={v.id}>{v.name}</option>)}</select><small>Which venue's orders this merchant takes</small></label>}
            {sec.fields.map(fd => <label key={fd.k} className={"ox-f" + (fd.wide ? " wide" : "")}>
              <span>{fd.l}{NEEDED.includes(fd.k) && <i className="ox-req" title="Needed before cards can be taken">•</i>}</span>
              {fd.opts ? <select disabled={!can} value={f[fd.k] ?? ""} onChange={e => setF({ ...f, [fd.k]: e.target.value })}><option value="">—</option>{fd.opts.map(o => <option key={o}>{o}</option>)}</select>
                : fd.long ? <textarea disabled={!can} rows={4} value={f[fd.k] ?? ""} placeholder={fd.ph} onChange={e => setF({ ...f, [fd.k]: e.target.value })} />
                : <input disabled={!can} className={fd.num ? "mono" : ""} inputMode={fd.num ? "text" : undefined} autoComplete="off" spellCheck={!fd.num} value={f[fd.k] ?? ""} placeholder={fd.ph} onChange={e => setF({ ...f, [fd.k]: e.target.value })} />}
              {fd.hint && <small>{fd.hint}</small>}
            </label>)}
          </div>
        </section>)}
        <p className="ox-fine">Agape Admin never keeps card numbers, bank account numbers or processor API secrets. Those stay with the processor.</p>
      </div>
      <aside className="ox-side">
        <div className="ox-card sticky">
          <h2>Status</h2>
          <div className="ox-status">{STATUS.map(([k, l, d]) => <label key={k} className={status === k ? "on" : ""}><input type="radio" name="status" disabled={!can} checked={status === k} onChange={() => setStatus(k)} /><span><b>{l}</b><small>{d}</small></span></label>)}</div>
          <h3>Needed before cards can be taken</h3>
          {missing.length ? <ul className="ox-missing">{missing.map(k => <li key={k}>{LABEL[k]}</li>)}</ul> : <p className="ox-ok">Everything the processor needs is here.</p>}
          {error && <p className="ox-err" role="alert">{error}</p>}
          {can && <button className="ox-btn primary block" disabled={busy || !changed || !(f.dba ?? "").trim()} onClick={save}>{busy ? "Saving…" : changed ? (m ? "Save changes" : "Add merchant") : "Saved"}</button>}
          {owner && m && (sure
            ? <div className="ox-row"><button className="ox-btn ghost sm" onClick={() => setSure(false)}>Keep it</button><button className="ox-btn danger sm" onClick={async () => { const r = await api("merchant/delete", { id: m.id }); if (r.ok) { toast("Deleted."); close(); } else setError(r.error); }}>Delete for good</button></div>
            : <button className="ox-link danger" onClick={() => setSure(true)}>Delete this merchant</button>)}
        </div>
      </aside>
    </div>
  </>;
}

// ---------------- the VAR sheet ----------------

type Copy = "processor" | "venue" | "investor";

function Sheet({ m, back, close }: { m: Merchant; back: () => void; close: () => void }) {
  const [copy, setCopy] = useState<Copy>("processor");
  const [copied, setCopied] = useState(false);
  const f = m.fields;
  const hide = copy === "investor";
  const secret = new Set(ALL.filter(x => x.secret).map(x => x.k));
  const val = (k: string) => { const v = (f[k] ?? "").trim(); return v && hide && secret.has(k) ? mask(v) : v; };
  const pending = NEEDED.filter(k => !(f[k] ?? "").trim());
  const blank = m.id === "blank";
  const ref = blank ? "" : `VAR-${m.id.slice(1).toUpperCase()}-${new Date(m.at * 1000).toISOString().slice(0, 10).replace(/-/g, "")}`;
  const print = () => { api("printed", { id: m.id }); const t = document.title; document.title = blank ? "Agape blank VAR sheet" : `${ref} ${f.dba} VAR sheet`; window.print(); document.title = t; };
  const text = () => {
    const lines = [`VAR SHEET · ${f.dba}`, `Reference ${ref} · Prepared ${day()} by Agape`, ""];
    for (const s of SECTIONS) { lines.push(s.title.toUpperCase()); for (const fd of s.fields) { const v = val(fd.k); if (v) lines.push(`${fd.l}: ${v}`); } lines.push(""); }
    if (pending.length) lines.push(`Still pending: ${pending.map(k => LABEL[k]).join(", ")}`);
    navigator.clipboard?.writeText(lines.join("\n")).then(() => { setCopied(true); setTimeout(() => setCopied(false), 2000); });
  };
  return <>
    <div className="ox-head noprint">
      <div><h1>VAR sheet</h1><p className="ox-sub">Print it or save it as a PDF from the print window. Every print is noted in Activity.</p></div>
      <div className="ox-acts">
        {!blank && <button className="ox-btn ghost" onClick={back}>Edit details</button>}
        <button className="ox-btn ghost" onClick={close}>All merchants</button>
        {!blank && <button className="ox-btn" onClick={text}>{copied ? "Copied" : "Copy as text"}</button>}
        <button className="ox-btn primary" onClick={print}>Print or save PDF</button>
      </div>
    </div>
    {!blank && <div className="ox-copy noprint" role="group" aria-label="Who this copy is for">
      <span>This copy is for</span>
      {([["processor", "The processor", "Every number, in full"], ["venue", "The venue", "Every number, in full"], ["investor", "Investors", "Account numbers masked to the last four"]] as [Copy, string, string][]).map(([k, l, d]) =>
        <button key={k} className={copy === k ? "on" : ""} aria-pressed={copy === k} onClick={() => setCopy(k)}><b>{l}</b><small>{d}</small></button>)}
    </div>}
    <article className="var-sheet">
      <header className="var-top">
        <div className="var-brand"><Wordmark tag /></div>
        <div className="var-title"><h1>VAR Sheet</h1><span>Merchant processing setup</span></div>
        {blank ? <dl className="var-ref"><div><dt>Date</dt><dd><span className="var-line short" /></dd></div><div><dt>Reference</dt><dd><span className="var-line short" /></dd></div></dl>
          : <dl className="var-ref"><div><dt>Reference</dt><dd className="mono">{ref}</dd></div><div><dt>Prepared</dt><dd>{day()}</dd></div><div><dt>Status</dt><dd>{STATUS.find(s => s[0] === m.status)?.[1]}</dd></div></dl>}
      </header>
      {blank ? null
      : <div className="var-who"><h2>{f.dba}</h2><p>{[f.legal_name && f.legal_name !== f.dba ? f.legal_name : "", [f.address, f.city, [f.state, f.zip].filter(Boolean).join(" ")].filter(Boolean).join(", ")].filter(Boolean).join(" · ")}</p></div>}
      {!blank && pending.length > 0 && <p className="var-pending"><b>Pending from the processor or merchant:</b> {pending.map(k => LABEL[k]).join(", ")}.</p>}
      {SECTIONS.filter(s => s.id !== "notes").map(s => <section key={s.id} className="var-sec">
        <h3>{s.title}</h3>
        <dl className="var-grid">{s.fields.filter(fd => s.id !== "software" || val(fd.k)).map(fd => { const v = val(fd.k); return <div key={fd.k} className={(fd.wide ? "wide " : "") + (v ? "" : "blank")}>
          <dt>{fd.l}</dt><dd className={fd.num ? "mono" : ""}>{v || <span className="var-line" aria-label="Blank" />}</dd></div>; })}</dl>
      </section>)}
      {blank && <section className="var-sec"><h3>Notes</h3>{[0, 1, 2].map(i => <span key={i} className="var-line" />)}</section>}
      {val("notes") && <section className="var-sec"><h3>Notes</h3><p className="var-notes">{val("notes")}</p></section>}
      <section className="var-sec var-sign">
        <h3>Confirmed by</h3>
        <div className="var-signs">{["Merchant", "Processor", "Agape"].map(w => <div key={w}><span className="var-line" /><small>{w}: name, signature and date</small></div>)}</div>
      </section>
      <footer className="var-foot">
        <span>Confidential. {hide ? "Account numbers are masked on this copy." : "Contains merchant account numbers: share only with the processor and the merchant."} Agape never stores or transmits card numbers.</span>
        <span>Agape to Your Seat · agapetoyourseat.com</span>
      </footer>
    </article>
  </>;
}

// ---------------- team, leads, activity, account ----------------

type Person = { mailbox?: string; id: string; email: string; name: string; role: string; role_name: string; at: number; off: boolean; boss?: string | null; title?: string; mine: boolean };
type GroupV = { id: string; name: string; about: string; members: string[]; member_names: string[]; boss?: string | null; mine: boolean };
type RoleV = { id: string; name: string; about: string; perms: string[]; template: string; holders: number; grantable: boolean; editable: boolean };
type TemplateV = { id: string; name: string; about: string; perms: string[]; editable: boolean };
type DirPerson = { id: string; name: string; title: string; role_name: string; mailbox: string; off: boolean; at: number; boss?: string | null; boss_name: string; groups: string[]; shared: string[]; email: string | null; yours: boolean };
type TeamView = { directory?: DirPerson[]; shared?: { local: string; name: string; members: string[] }[]; users: Person[]; groups: GroupV[]; chain: { id: string; name: string; group: boolean }[]; invites: { email: string; name: string; role: string; role_name: string; until: number; boss: string }[];
  contacts: { id: string; name: string; title?: string }[]; roles: RoleV[]; templates: TemplateV[]; perms: { id: string; about: string; mine: boolean }[] };

function useTeam() {
  const [t, setT] = useState<TeamView | null>(null);
  const load = async () => { const r = await api("team"); if (r.ok) setT(r); };
  useEffect(() => { load(); }, []);
  return { t, load };
}

type Sub = "directory" | "chart" | "people" | "groups" | "roles";

/** Someone asked to write to a person from elsewhere in Admin: Messages opens a new email to them. */
let pendingDraft: Draft | null = null;
const goWrite = (to: string[]) => { pendingDraft = { to, subject: "", body: "" }; window.dispatchEvent(new CustomEvent("ox-go", { detail: "inbox" })); };
function Team({ me, toast }: { me: Me; toast: (s: string) => void }) {
  const { t, load } = useTeam();
  const [sub, setSub] = useState<Sub>("directory");
  const has = (p: string) => me.perms.includes(p);
  const subs: [Sub, string][] = [["directory", "Directory"], ["chart", "Chart"], ["people", "People"], ["groups", "Groups"], ...(has("structure") ? [["roles", "Roles and templates"]] as [Sub, string][] : [])];
  return <>
    <Head title="Team" sub="Everyone at Agape and their addresses, and who reports to whom. Messages and activity: you see what's below you, never what's above or beside you." />
    <div className="ox-seg" role="tablist">{subs.map(([k, l]) => <button key={k} role="tab" aria-selected={sub === k} className={sub === k ? "on" : ""} onClick={() => setSub(k)}>{l}</button>)}</div>
    {!t ? <p className="ox-empty">Loading…</p> : <>
      {sub === "directory" && <Directory me={me} t={t} />}
      {sub === "chart" && <Chart me={me} t={t} />}
      {sub === "people" && <People me={me} t={t} load={load} toast={toast} />}
      {sub === "groups" && <Groups me={me} t={t} load={load} toast={toast} />}
      {sub === "roles" && <Roles t={t} load={load} toast={toast} />}
    </>}
  </>;
}

/** Everyone at Agape: their address here, what they do, who they report to. Click someone for their profile. */
function Directory({ me, t }: { me: Me; t: TeamView }) {
  const [q, setQ] = useState(""); const [open, setOpen] = useState<string | null>(null);
  const all = (t.directory ?? []).filter(p => !p.off).sort((a, b) => (a.id === me.id ? -1 : b.id === me.id ? 1 : a.name.localeCompare(b.name)));
  const shown = all.filter(p => !q.trim() || [p.name, p.title, p.role_name, p.mailbox].join(" ").toLowerCase().includes(q.trim().toLowerCase()));
  const byId = (id: string) => all.find(p => p.id === id);
  const initials = (n: string) => n.split(/\s+/).map(w => w[0]).slice(0, 2).join("").toUpperCase();
  const p = open ? byId(open) : null;
  const addr = (l: string) => `${l}@${me.domain}`;
  const copy = (x: string) => navigator.clipboard?.writeText(x);
  useEffect(() => { if (!open) return; const k = (e: KeyboardEvent) => e.key === "Escape" && setOpen(null); addEventListener("keydown", k); return () => removeEventListener("keydown", k); }, [open]);
  return <>
    <section className="ox-card">
      <div className="ox-row between"><h2>Everyone at Agape</h2><input className="ox-search" type="search" placeholder="Search names, titles, addresses" aria-label="Search the directory" value={q} onChange={e => setQ(e.target.value)} /></div>
      <div className="dir-grid">{shown.map(x => <button key={x.id} className={"dir-card" + (x.id === me.id ? " you" : "")} onClick={() => setOpen(x.id)}>
        <i className="dir-av" aria-hidden="true">{initials(x.name)}</i>
        <span className="dir-who"><b>{x.name}{x.id === me.id && <small> (you)</small>}</b><small>{x.title || x.role_name}</small><span className="dir-addr">{x.mailbox ? addr(x.mailbox) : "No address yet"}</span></span>
      </button>)}</div>
      {!shown.length && <p className="ox-empty">Nobody matches.</p>}
    </section>
    {(t.shared ?? []).length > 0 && <section className="ox-card">
      <h2>Shared addresses</h2>
      <ul className="ox-shared">{(t.shared ?? []).map(x => <li key={x.local}><div><b>{addr(x.local)}</b>{x.name !== x.local && <small>{x.name}</small>}<span>{x.members.map(m => byId(m)?.name).filter(Boolean).join(", ")}</span></div>
        <div className="ox-row"><button className="ox-btn sm" onClick={() => copy(addr(x.local))}>Copy</button></div></li>)}</ul>
    </section>}
    {p && <div className="dir-view" role="dialog" aria-label={p.name} onClick={() => setOpen(null)}><div className="dir-prof" onClick={e => e.stopPropagation()}>
      <div className="dir-top"><i className="dir-av big" aria-hidden="true">{initials(p.name)}</i><div><h2>{p.name}</h2><span>{[p.title, p.role_name].filter(Boolean).join(" · ")}</span></div><button className="ox-btn sm ghost" onClick={() => setOpen(null)}>Close</button></div>
      {p.mailbox && <div className="dir-mail"><b>{addr(p.mailbox)}</b><div className="ox-row">{p.id !== me.id && <button className="ox-btn primary sm" onClick={() => goWrite([p.id])}>Email {p.name.split(" ")[0]}</button>}<button className="ox-btn sm" onClick={() => copy(addr(p.mailbox))}>Copy</button></div></div>}
      <dl className="dir-facts">
        <dt>Reports to</dt><dd>{p.boss ? (byId(p.boss) ? <button className="ox-link" onClick={() => setOpen(p.boss!)}>{p.boss_name}</button> : p.boss_name) : "Nobody: the top"}</dd>
        {(() => { const kids = all.filter(x => x.boss === p.id); return kids.length > 0 && <><dt>Reporting to them</dt><dd>{kids.map((k, i) => <React.Fragment key={k.id}>{i > 0 && ", "}<button className="ox-link" onClick={() => setOpen(k.id)}>{k.name}</button></React.Fragment>)}</dd></>; })()}
        {p.groups.length > 0 && <><dt>Groups</dt><dd>{p.groups.join(", ")}</dd></>}
        {p.shared.length > 0 && <><dt>Shared addresses</dt><dd>{p.shared.map(addr).join(", ")}</dd></>}
        <dt>At Agape since</dt><dd>{new Date(p.at * 1000).toLocaleDateString(undefined, { month: "long", day: "numeric", year: "numeric" })}</dd>
        {p.email && <><dt>Signs in with</dt><dd>{p.email}<small>{p.id === me.id ? "Only you and the people above you see this." : "Only they and the people above them see this."}</small></dd></>}
      </dl>
    </div></div>}
  </>;
}

const nodeName = (t: TeamView, id?: string | null) => !id ? "" : t.users.find(u => u.id === id)?.name ?? t.groups.find(g => g.id === id)?.name ?? t.chain.find(c => c.id === id)?.name ?? "someone above you";
/** Where someone can report: you, and every person and group below you. */
const targets = (me: Me, t: TeamView, not?: string) => [
  { id: me.id, label: `You (${me.name})` },
  ...t.users.filter(u => u.mine && !u.off && u.id !== me.id && u.id !== not).map(u => ({ id: u.id, label: u.name + (u.title ? ` · ${u.title}` : "") })),
  ...t.groups.filter(g => g.mine && g.id !== not).map(g => ({ id: g.id, label: `${g.name} (group)` })),
];

function Chart({ me, t }: { me: Me; t: TeamView }) {
  const kids = (id: string) => [...t.groups.filter(g => g.boss === id).map(g => ({ g })), ...t.users.filter(u => u.boss === id).map(u => ({ u }))] as { g?: GroupV; u?: Person }[];
  const seen = new Set<string>();
  const Node = ({ n }: { n: { g?: GroupV; u?: Person } }): any => {
    const id = n.g?.id ?? n.u!.id;
    if (seen.has(id)) return null; seen.add(id);
    const below = kids(id);
    return <li>
      {n.g ? <div className="ox-node group"><b>{n.g.name}</b><span>Group · {n.g.member_names.length ? n.g.member_names.join(", ") : "no members yet"}</span></div>
        : <div className={"ox-node" + (n.u!.off ? " off" : "") + (n.u!.id === me.id ? " you" : "") + (n.u!.role === "host" ? " host" : "")}>
            <b>{n.u!.name}{n.u!.id === me.id && <small> (you)</small>}</b><span>{[n.u!.title, n.u!.role_name].filter(Boolean).join(" · ")}</span></div>}
      {below.length > 0 && <ul>{below.map(k => <Node key={k.g?.id ?? k.u!.id} n={k} />)}</ul>}
    </li>;
  };
  const host = me.role === "host";
  const roots: { g?: GroupV; u?: Person }[] = host
    ? [...t.users.filter(u => u.role === "host").map(u => ({ u })), ...t.groups.filter(g => !g.boss).map(g => ({ g })), ...t.users.filter(u => u.role !== "host" && !u.boss).map(u => ({ u }))]
    : [{ u: t.users.find(u => u.id === me.id)! }, ...t.groups.filter(g => g.members.includes(me.id)).map(g => ({ g }))];
  const loose = host ? t.users.filter(u => u.role !== "host" && !u.boss) : [];
  return <section className="ox-card">
    {t.chain.length > 0 && <p className="ox-sub">You report to {t.chain.map(c => c.name + (c.group ? " (group)" : "")).join(", which reports to ")}. They can see your messages and activity; you can't see theirs.</p>}
    {host && <p className="ox-sub">You're the host: everything at Agape is below you.</p>}
    <ul className="ox-tree">{roots.filter(r => r.u || r.g).map(r => <Node key={r.g?.id ?? r.u!.id} n={r} />)}</ul>
    {loose.length > 0 && <p className="ox-note">Not placed yet: {loose.map(u => u.name).join(", ")}. Choose who they report to under People.</p>}
  </section>;
}

function People({ me, t, load, toast }: { me: Me; t: TeamView; load: () => void; toast: (s: string) => void }) {
  const [link, setLink] = useState("");
  const grantable = t.roles.filter(r => r.grantable);
  const f = useForm({ email: "", name: "", title: "", role: grantable.find(r => r.id === "r-member")?.id ?? grantable[0]?.id ?? "", boss: me.id });
  const canInvite = me.perms.includes("invite"), canPeople = me.perms.includes("people");
  const [sure, setSure] = useState("");
  const set = async (id: string, b: Record<string, unknown>, ok: string) => { const r = await api("user", { id, ...b }); toast(r.ok ? ok : r.error); load(); };
  const role = t.roles.find(r => r.id === f.v.role);
  return <>
    {canInvite && <section className="ox-card">
      <h2>Invite someone</h2>
      <p className="ox-sub">They get a link to choose their own password. It works once, for 7 days. Send it to them yourself, by email or text.</p>
      <form className="ox-invite" onSubmit={e => f.run(e, async () => { const r = await api("invite", f.v); if (!r.ok) return r.error; setLink(`${location.origin}/admin/?invite=${r.code}`); f.reset({ email: "", name: "", title: "" }); load(); })}>
        <label className="ox-f"><span>Name</span><input required {...f.bind("name")} /></label>
        <label className="ox-f"><span>Email</span><input type="email" required {...f.bind("email")} /></label>
        <label className="ox-f"><span>Title</span><input placeholder="e.g. Head of payments" {...f.bind("title")} /></label>
        <label className="ox-f"><span>Role</span><select required {...f.bind("role")}>{grantable.map(r => <option key={r.id} value={r.id}>{r.name}</option>)}</select></label>
        <label className="ox-f"><span>Reports to</span><select {...f.bind("boss")}>{targets(me, t).map(x => <option key={x.id} value={x.id}>{x.label}</option>)}</select></label>
        <button className="ox-btn primary" disabled={f.busy}>Create invitation</button>
      </form>
      {role && <p className="ox-fine">{role.name} can: {role.perms.length ? role.perms.map(p => t.perms.find(x => x.id === p)?.about.toLowerCase()).join("; ") : "see their own messages, and what's below them"}.</p>}
      {f.error && <p className="ox-err">{f.error}</p>}
      {link && <div className="ox-linkbox"><span>Send them this link. It won't be shown again.</span><code>{link}</code><button className="ox-btn sm" onClick={() => navigator.clipboard?.writeText(link).then(() => toast("Link copied."))}>Copy link</button></div>}
    </section>}
    <section className="ox-card">
      <h2>People</h2>
      <div className="ox-table-wrap"><table className="ox-table">
        <thead><tr><th>Name</th><th>Email</th><th>Role</th><th>Reports to</th>{canPeople && <th></th>}</tr></thead>
        <tbody>{t.users.map(u => { const edit = canPeople && u.mine && u.id !== me.id && u.role !== "host" && !u.off; return <tr key={u.id} className={u.off ? "off" : ""}>
          <td>{u.name}{u.id === me.id ? <small>You</small> : u.title && <small>{u.title}</small>}</td><td>{u.email}</td>
          <td>{edit && t.roles.find(r => r.id === u.role)?.grantable !== false ? <select aria-label={`Role for ${u.name}`} value={u.role} onChange={e => set(u.id, { role: e.target.value }, "Role changed.")}>{grantable.map(r => <option key={r.id} value={r.id}>{r.name}</option>)}</select> : u.off ? "Turned off" : u.role_name}</td>
          <td>{edit ? <select aria-label={`Who ${u.name} reports to`} value={u.boss ?? ""} onChange={e => set(u.id, { boss: e.target.value }, "Reporting line changed.")}>{!u.boss && <option value="">Choose…</option>}{targets(me, t, u.id).map(x => <option key={x.id} value={x.id}>{x.label}</option>)}</select> : nodeName(t, u.boss) || "—"}</td>
          {canPeople && <td>{u.mine && u.id !== me.id && u.role !== "host" && <div className="ox-row nowrap">
            <button className="ox-btn ghost sm" onClick={() => set(u.id, { off: !u.off }, u.off ? "Access turned back on." : "Access turned off. They're signed out everywhere.")}>{u.off ? "Turn access on" : "Turn access off"}</button>
            {sure === u.id
              ? <><button className="ox-btn ghost sm" onClick={() => setSure("")}>Keep</button><button className="ox-btn danger sm" onClick={async () => { const r = await api("user/delete", { id: u.id }); setSure(""); toast(r.ok ? `${r.name}'s account is deleted. Their messages are kept, and mail to their address now comes to the host.` : r.error); load(); }}>Delete {u.name.split(" ")[0]}</button></>
              : <button className="ox-btn ghost sm" onClick={() => setSure(u.id)}>Delete…</button>}
          </div>}</td>}
        </tr>; })}
        {t.invites.map(i => <tr key={i.email} className="pending">
          <td>{i.name || "—"}<small>Invited</small></td><td>{i.email}</td><td>{i.role_name}</td><td><small>{nodeName(t, i.boss)} · link works until {when(i.until)}</small></td>
          {canPeople && <td><button className="ox-btn ghost sm" onClick={async () => { const r = await api("invite/cancel", { email: i.email }); toast(r.ok ? "Invitation cancelled." : r.error); load(); }}>Cancel</button></td>}
        </tr>)}</tbody>
      </table></div>
    </section>
  </>;
}

function Groups({ me, t, load, toast }: { me: Me; t: TeamView; load: () => void; toast: (s: string) => void }) {
  const [edit, setEdit] = useState<GroupV | "new" | null>(null);
  const can = me.perms.includes("structure");
  return <>
    <section className="ox-card">
      <div className="ox-row between"><h2>Groups</h2>{can && !edit && <button className="ox-btn primary sm" onClick={() => setEdit("new")}>New group</button>}</div>
      <p className="ox-sub">A group is a body people or other groups can report to, like a board of directors or the principal shareholders. Everyone in a group sees everything below it.</p>
      {edit && <GroupForm me={me} t={t} g={edit === "new" ? null : edit} done={(m) => { if (m) toast(m); setEdit(null); load(); }} />}
      {!t.groups.length ? <p className="ox-empty">No groups yet.</p> : <div className="ox-table-wrap"><table className="ox-table">
        <thead><tr><th>Group</th><th>Members</th><th>Reports to</th><th></th></tr></thead>
        <tbody>{t.groups.map(g => <tr key={g.id}>
          <td><b>{g.name}</b>{g.about && <small>{g.about}</small>}</td><td>{g.member_names.join(", ") || <span className="ox-muted">Nobody yet</span>}</td>
          <td>{g.boss ? nodeName(t, g.boss) : "Nobody: the top"}</td>
          <td>{can && g.mine && <button className="ox-btn ghost sm" onClick={() => setEdit(g)}>Edit</button>}</td>
        </tr>)}</tbody>
      </table></div>}
    </section>
  </>;
}

function GroupForm({ me, t, g, done }: { me: Me; t: TeamView; g: GroupV | null; done: (m?: string) => void }) {
  const host = me.role === "host";
  const f = useForm({ name: g?.name ?? "", about: g?.about ?? "", boss: g?.boss ?? (host ? "" : me.id) });
  const [members, setMembers] = useState<string[]>(g?.members ?? []);
  const pickable = t.users.filter(u => !u.off && (host || u.mine || members.includes(u.id)) && u.role !== "host" || (host && u.role === "host"));
  return <form className="ox-subform" onSubmit={e => f.run(e, async () => { const r = await api("group", { id: g?.id, ...f.v, members }); if (!r.ok) return r.error; done(g ? "Group saved." : "Group created."); })}>
    <div className="ox-grid">
      <label className="ox-f"><span>Name</span><input required placeholder="e.g. Board of Directors" {...f.bind("name")} /></label>
      <label className="ox-f"><span>Reports to</span><select {...f.bind("boss")}>{host && <option value="">Nobody: it's at the top</option>}{targets(me, t, g?.id).map(x => <option key={x.id} value={x.id}>{x.label}</option>)}</select></label>
      <label className="ox-f wide"><span>What it's for</span><input {...f.bind("about")} /></label>
    </div>
    <fieldset className="ox-checks"><legend>Members</legend>{pickable.map(u => <label key={u.id}><input type="checkbox" checked={members.includes(u.id)} onChange={e => setMembers(m => e.target.checked ? [...m, u.id] : m.filter(x => x !== u.id))} /> {u.name}{u.id === me.id ? " (you)" : ""}</label>)}
      {!pickable.length && <p className="ox-muted">Nobody below you yet.</p>}</fieldset>
    <p className="ox-fine">Members see everything below this group. You can only add people who are below you.</p>
    {f.error && <p className="ox-err">{f.error}</p>}
    <div className="ox-row"><button className="ox-btn ghost" type="button" onClick={() => done()}>Cancel</button><button className="ox-btn primary" disabled={f.busy}>{g ? "Save group" : "Create group"}</button>
      {g && <button className="ox-link danger" type="button" onClick={async () => { const r = await api("group/delete", { id: g.id }); if (r.ok) done("Group removed."); else f.setError(r.error); }}>Remove group</button>}</div>
  </form>;
}

function PermBoxes({ t, value, set }: { t: TeamView; value: string[]; set: (v: string[]) => void }) {
  return <fieldset className="ox-checks perms"><legend>What it can do</legend>{t.perms.map(p => <label key={p.id} className={p.mine ? "" : "ox-muted"} title={p.mine ? undefined : "Your own role can't do this, so you can't give it"}>
    <input type="checkbox" disabled={!p.mine} checked={value.includes(p.id)} onChange={e => set(e.target.checked ? [...value, p.id] : value.filter(x => x !== p.id))} /> {p.about}</label>)}
    <small>Everyone sees their own messages, and everything from the people and groups below them, whatever their role.</small></fieldset>;
}

function Roles({ t, load, toast }: { t: TeamView; load: () => void; toast: (s: string) => void }) {
  const [edit, setEdit] = useState<{ kind: "role" | "template"; id?: string; name: string; about: string; perms: string[]; template?: string } | null>(null);
  const [from, setFrom] = useState(t.templates[0]?.id ?? "");
  const [error, setError] = useState("");
  const save = async () => {
    if (!edit) return;
    const r = await api(edit.kind, { id: edit.id, name: edit.name, about: edit.about, perms: edit.perms, template: edit.template ?? "" });
    if (!r.ok) { setError(r.error); return; }
    toast(edit.kind === "role" ? "Role saved." : "Template saved."); setEdit(null); setError(""); load();
  };
  const remove = async () => { if (!edit?.id) return; const r = await api(edit.kind + "/delete", { id: edit.id }); if (!r.ok) { setError(r.error); return; } toast("Removed."); setEdit(null); load(); };
  const perm = (id: string) => t.perms.find(p => p.id === id)?.about ?? id;
  const editor = edit && <div className="ox-subform">
    <h3>{edit.id ? `Edit ${edit.kind}` : `New ${edit.kind}`}</h3>
    <div className="ox-grid"><label className="ox-f"><span>Name</span><input value={edit.name} onChange={e => setEdit({ ...edit, name: e.target.value })} /></label>
      <label className="ox-f wide"><span>What it's for</span><input value={edit.about} onChange={e => setEdit({ ...edit, about: e.target.value })} /></label></div>
    <PermBoxes t={t} value={edit.perms} set={p => setEdit({ ...edit, perms: p })} />
    {edit.kind === "template" && edit.id && <p className="ox-fine">Changing a template doesn't change roles already made from it.</p>}
    {error && <p className="ox-err">{error}</p>}
    <div className="ox-row"><button className="ox-btn ghost" onClick={() => { setEdit(null); setError(""); }}>Cancel</button><button className="ox-btn primary" onClick={save}>Save</button>{edit.id && <button className="ox-link danger" onClick={remove}>Remove</button>}</div>
  </div>;
  return <>
    <section className="ox-card">
      <div className="ox-row between"><h2>Roles</h2>
        <div className="ox-row"><select aria-label="Template" value={from} onChange={e => setFrom(e.target.value)}>{t.templates.map(x => <option key={x.id} value={x.id}>{x.name}</option>)}</select>
          <button className="ox-btn primary sm" onClick={() => { const x = t.templates.find(y => y.id === from); if (x) setEdit({ kind: "role", name: x.name, about: x.about, perms: x.perms.filter(p => t.perms.find(q => q.id === p)?.mine), template: x.id }); }}>New role from template</button></div></div>
      <p className="ox-sub">What someone can do. Each role starts from a template and is then its own.</p>
      {edit?.kind === "role" && editor}
      <div className="ox-table-wrap"><table className="ox-table"><thead><tr><th>Role</th><th>Can</th><th>Held by</th><th></th></tr></thead>
        <tbody><tr><td><b>Host</b><small>Fixed</small></td><td><small>Everything</small></td><td>1</td><td></td></tr>
        {t.roles.map(r => <tr key={r.id}><td><b>{r.name}</b>{r.about && <small>{r.about}</small>}</td><td><small>{r.perms.length ? r.perms.map(perm).join("; ") : "Their own messages, and what's below them"}</small></td><td>{r.holders}</td>
          <td>{r.editable && <button className="ox-btn ghost sm" onClick={() => setEdit({ kind: "role", id: r.id, name: r.name, about: r.about, perms: r.perms, template: r.template })}>Edit</button>}</td></tr>)}</tbody></table></div>
    </section>
    <section className="ox-card">
      <div className="ox-row between"><h2>Templates</h2><button className="ox-btn sm" onClick={() => setEdit({ kind: "template", name: "", about: "", perms: [] })}>New template</button></div>
      <p className="ox-sub">Blueprints for roles, like Manager or Team member. Edit them freely; roles made from them keep their own settings.</p>
      {edit?.kind === "template" && editor}
      <div className="ox-table-wrap"><table className="ox-table"><thead><tr><th>Template</th><th>Can</th><th></th></tr></thead>
        <tbody>{t.templates.map(x => <tr key={x.id}><td><b>{x.name}</b>{x.about && <small>{x.about}</small>}</td><td><small>{x.perms.length ? x.perms.map(perm).join("; ") : "Their own messages, and what's below them"}</small></td>
          <td>{x.editable && <button className="ox-btn ghost sm" onClick={() => setEdit({ kind: "template", id: x.id, name: x.name, about: x.about, perms: x.perms })}>Edit</button>}</td></tr>)}</tbody></table></div>
    </section>
  </>;
}

type Msg = { id: string; at: number; from: string; from_name: string; to: string[]; to_names: string[]; subject: string; body: string; kind: string; mine: boolean; sent: boolean; unread: boolean;
  ext_from: string; ext_to: string[]; status: string[]; attachments: string[]; channels: string[] };
type Draft = { to: string[]; subject: string; body: string; from?: string };

function useNotes() {
  const [notes, setNotes] = useState<Msg[] | null>(null);
  const [muted, setMuted] = useState<string[]>([]);
  const load = async () => { const r = await api("inbox"); if (r.ok) { setNotes(r.notes); setMuted(r.muted ?? []); } };
  useEffect(() => { load(); const t = setInterval(load, 20000); return () => clearInterval(t); }, []);
  return { notes, setNotes, muted, setMuted, load };
}

/** A list of messages. Opening one marks it read for you (and only you). */
function MsgList({ notes, setNotes, seen, reply, empty, showWho }: { notes: Msg[]; setNotes: (f: (n: Msg[] | null) => Msg[] | null) => void; seen: () => void; reply?: (n: Msg) => void; empty: string; showWho: boolean }) {
  const [open, setOpen] = useState<string | null>(null);
  const toggle = (n: Msg) => {
    setOpen(open === n.id ? null : n.id);
    if (n.unread) { setNotes(all => all && all.map(x => x.id === n.id ? { ...x, unread: false } : x)); api("read", { ids: [n.id] }).then(seen); }
  };
  const who = (n: Msg) => n.kind === "lead" || n.kind === "joined" ? "Agape" : n.sent ? `To ${n.to_names.join(", ")}` : n.from_name;
  if (!notes.length) return <p className="ox-empty">{empty}</p>;
  return <ul className="ox-inbox">{notes.map(n => <li key={n.id} className={(n.unread ? "unread " : "") + (open === n.id ? "open" : "")}>
    <button className="ox-msg" onClick={() => toggle(n)} aria-expanded={open === n.id}>
      <span className="ox-from">{showWho ? n.from_name : who(n)}{showWho && <small>to {n.to_names.join(", ") || "—"}</small>}</span>
      <span className="ox-subj">{n.kind === "email" && n.ext_from && <i className="ox-kind">Email</i>}{n.status.some(x => x.includes("not delivered")) && <i className="ox-flag">Not delivered</i>}<b>{n.subject}</b> <span className="ox-muted">{n.body.slice(0, 120)}</span></span>
      <time>{when(n.at)}</time>
    </button>
    {open === n.id && <div className="ox-body">
      <p className="ox-meta">From {n.from_name} to {n.to_names.join(", ") || "—"} · {when(n.at)}</p>
      <p>{n.body}</p>
      {n.attachments.length > 0 && <div className="ox-files"><span>Attachments</span>{n.attachments.map((name, i) => <button key={i} className="ox-btn sm" onClick={() => download(n.id, i, name)}>{name}</button>)}</div>}
      {n.status.length > 0 && <ul className="ox-status">{n.status.map((x, i) => <li key={i} className={x.includes("not delivered") ? "bad" : "good"}>{x}</li>)}</ul>}
      {reply && n.mine && n.kind !== "lead" && n.kind !== "joined" && !n.sent && <button className="ox-btn sm" onClick={() => reply(n)}>Reply</button>}
    </div>}
  </li>)}</ul>;
}

/** Your own mailbox: email to your address, messages from your team, and notices for you. */
function Inbox({ me, toast, seen }: { me: Me; toast: (s: string) => void; seen: () => void }) {
  const { notes, setNotes, load } = useNotes();
  const [draft, setDraft] = useState<Draft | null>(() => { const d = pendingDraft; pendingDraft = null; return d; });
  const [contacts, setContacts] = useState<{ id: string; name: string; title?: string }[]>([]);
  useEffect(() => { api("team").then(r => r.ok && setContacts([...r.contacts, ...r.groups.map((g: any) => ({ id: g.id, name: g.name, title: "everyone in the group" }))])); }, []);
  const cameTo = (n: Msg) => (n.ext_to ?? []).map(t => t.toLowerCase().replace(/.*</, "").replace(/>.*/, "")).filter(t => t.endsWith("@" + me.domain)).map(t => t.split("@")[0].split("+")[0]).find(l => (me.send_as ?? []).includes(l));
  const reply = (n: Msg) => setDraft({ from: n.ext_from ? cameTo(n) : undefined, to: [n.ext_from ? n.ext_from.replace(/.*</, "").replace(/>.*/, "") : n.from].filter(Boolean), subject: /^re:/i.test(n.subject) ? n.subject : "Re: " + n.subject, body: `\n\nOn ${when(n.at)}, ${n.from_name} wrote:\n` + n.body.trimEnd().split("\n").map(l => "> " + l).join("\n") });
  const mine = (notes ?? []).filter(n => n.mine);
  return <>
    <Head title="Messages" sub={<>{me.mailbox && <>Your address is <b>{me.mailbox}@{me.domain}</b>. </>}Email to it, messages from your team and notices for you.{me.boss ? " The people above you can see these too." : ""}</>}>
      <button className="ox-btn primary" onClick={() => setDraft(d => d ? null : { to: [], subject: "", body: "" })}>{draft ? "Close" : "New email"}</button>
    </Head>
    {draft && <Compose me={me} d={draft} contacts={contacts} done={(m) => { if (m) toast(m); setDraft(null); load(); }} />}
    {notes === null ? <p className="ox-empty">Loading…</p> : <MsgList notes={mine} setNotes={setNotes} seen={seen} reply={reply} showWho={false} empty="Nothing yet. Email to your address, messages from your team, and notices like new leads all land here." />}
  </>;
}

/** One channel per person below you: everything they send and receive. Nothing here can be deleted. */
function Channels({ me, seen }: { me: Me; seen: () => void }) {
  const { notes, setNotes, muted, setMuted } = useNotes();
  const [people, setPeople] = useState<Person[]>([]);
  const [pick, setPick] = useState<string>("");
  useEffect(() => { api("team").then(r => r.ok && setPeople(r.users.filter((u: Person) => u.id !== me.id && u.mine))); }, []);
  const others = (notes ?? []).filter(n => !n.mine);
  const of = (id: string) => others.filter(n => (n.channels as any as string[] ?? []).includes(id));
  const unread = (id: string) => of(id).filter(n => n.unread).length;
  const chans = people.map(p => ({ p, n: of(p.id).length, u: unread(p.id) })).sort((a, b) => (b.u - a.u) || (b.n - a.n) || a.p.name.localeCompare(b.p.name));
  const shown = pick ? of(pick) : others;
  const mute = async (id: string, on: boolean) => { setMuted(m => on ? [...m, id] : m.filter(x => x !== id)); await api("mute", { id, on }); seen(); };
  const cur = people.find(p => p.id === pick);
  return <>
    <Head title="Channels" sub={me.role === "host" ? "One channel for each person at Agape. Everything they send and receive lands here as well as in their own Messages, and new activity pings you. Nothing can be deleted." : "One channel for each person below you. Everything they send and receive lands here as well as in their own Messages, and new activity pings you."} />
    <div className="ox-channels">
      <nav className="ox-chanlist" aria-label="Channels">
        <button className={!pick ? "on" : ""} onClick={() => setPick("")}><span>Everyone</span>{others.filter(n => n.unread).length > 0 && <i className="ox-badge soft">{others.filter(n => n.unread).length}</i>}</button>
        {chans.map(({ p, u, n }) => <button key={p.id} className={(pick === p.id ? "on " : "") + (muted.includes(p.id) ? "muted" : "")} onClick={() => setPick(p.id)}>
          <span>{p.name}<small>{p.mailbox ? `${p.mailbox}@${me.domain}` : p.role_name} · {n}</small></span>{u > 0 && <i className={"ox-badge" + (muted.includes(p.id) ? " soft" : "")}>{u}</i>}</button>)}
        {!people.length && <p className="ox-muted">Nobody below you yet.</p>}
      </nav>
      <div className="ox-chanbody">
        {cur && <div className="ox-row between ox-chanhead"><div><h2>{cur.name}</h2><small>{[cur.title, cur.role_name, cur.mailbox && `${cur.mailbox}@${me.domain}`].filter(Boolean).join(" · ")}</small></div>
          <label className="ox-checkline"><input type="checkbox" checked={muted.includes(cur.id)} onChange={e => mute(cur.id, e.target.checked)} /> Mute pings</label></div>}
        {notes === null ? <p className="ox-empty">Loading…</p> : <MsgList notes={shown} setNotes={setNotes} seen={seen} showWho empty={cur ? `Nothing to or from ${cur.name} yet.` : "Nothing from anyone below you yet."} />}
      </div>
    </div>
  </>;
}

function Compose({ me, d, contacts, done }: { me: Me; d: Draft; contacts: { id: string; name: string; title?: string }[]; done: (m?: string) => void }) {
  const [to, setTo] = useState<string[]>(d.to);
  const [typed, setTyped] = useState("");
  const [from, setFrom] = useState(d.from ?? me.mailbox);
  const as = me.send_as ?? [me.mailbox];
  const f = useForm({ subject: d.subject, body: d.body });
  const label = (x: string) => contacts.find(c => c.id === x)?.name ?? x;
  const add = (x: string) => { const v = x.trim().replace(/[,;]$/, ""); if (v && !to.includes(v)) setTo([...to, v]); setTyped(""); };
  return <section className="ox-card narrow">
    <form onSubmit={e => f.run(e, async () => {
      const all = typed.trim() ? [...to, typed.trim()] : to;
      if (!all.length) return "Add who it's for";
      const r = await api("email", { to: all, subject: f.v.subject, body: f.v.body, from });
      if (!r.ok) return r.error;
      done(r.failed ? `Sent, but ${r.failed} address${r.failed > 1 ? "es" : ""} couldn't be reached. Open it to see why.` : "Sent.");
    })}>
      {as.length > 1 ? <label className="ox-f"><span>From</span><select value={from} onChange={e => setFrom(e.target.value)}>{as.map(l => <option key={l} value={l}>{me.name} &lt;{l}@{me.domain}&gt;{l === me.mailbox ? "" : " · shared"}</option>)}</select></label>
        : <p className="ox-meta">From {me.name} &lt;{me.mailbox}@{me.domain}&gt;</p>}
      <div className="ox-f"><span>To</span>
        <div className="ox-chips">{to.map(x => <span key={x} className="ox-chip">{label(x)}<button type="button" aria-label={`Remove ${label(x)}`} onClick={() => setTo(to.filter(y => y !== x))}>×</button></span>)}
          <input value={typed} placeholder={to.length ? "Add another" : "Email address, or pick someone below"} onChange={e => { const v = e.target.value; if (/[,;\s]$/.test(v) && v.includes("@")) add(v); else setTyped(v); }} onKeyDown={e => { if (e.key === "Enter" && typed.includes("@")) { e.preventDefault(); add(typed); } }} />
        </div>
        <select aria-label="Add someone at Agape" value="" onChange={e => e.target.value && add(e.target.value)}><option value="">Add someone at Agape…</option>{contacts.filter(c => !to.includes(c.id)).map(c => <option key={c.id} value={c.id}>{c.name}{c.title ? ` · ${c.title}` : ""}</option>)}</select>
      </div>
      <label className="ox-f"><span>Subject</span><input {...f.bind("subject")} /></label>
      <label className="ox-f"><span>Message</span><textarea rows={8} required {...f.bind("body")} /></label>
      {f.error && <p className="ox-err">{f.error}</p>}
      <div className="ox-row"><button className="ox-btn ghost" type="button" onClick={() => done()}>Discard</button><button className="ox-btn primary" disabled={f.busy}>{f.busy ? "Sending…" : "Send"}</button></div>
      <p className="ox-fine">Kept for good, and visible to the people in it and everyone above them. Never put card or bank numbers in an email.</p>
    </form>
  </section>;
}

/** For the host: what has to be true for Agape's own email to work, checked live, with the exact records to add. */
/** Until Agape's own mail can send: a sending account (a Gmail address with an app password) that emails the
 * founders the moment someone writes in through the site. The password is kept on the server and never shown again. */
function Alerts({ toast, service }: { toast: (s: string) => void; service: boolean }) {
  const [d, setD] = useState<any>(null);
  const [f, setF] = useState({ address: "", password: "", alert_to: "" });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const load = async () => { const r = await api("mail/relay"); if (r.ok) { setD(r); setF({ address: "", password: "", alert_to: (r.alert_to ?? []).join(", ") }); } };
  useEffect(() => { load(); }, [service]);
  const save = async (e: React.FormEvent) => {
    e.preventDefault(); setBusy(true); setError("");
    const r = await api("mail/relay", { address: f.address, password: f.password, alert_to: f.alert_to.split(/[,\s]+/).filter(Boolean) });
    setBusy(false);
    if (r.ok) { toast("Alerts are on. A test email is on its way."); load(); } else setError(r.error);
  };
  const off = async () => { if (!confirm("Stop sending alerts by email?")) return; await api("mail/relay", { clear: true }); toast("Alerts are off."); load(); };
  if (!d) return null;
  const from = service ? `alerts@${location.hostname.replace(/^www\./, "")}` : d.address;
  return <section className="ox-card">
    <h2>Alerts by email</h2>
    <p className="ox-sub">{d.set ? <>On: when someone writes in through the site, <b>{from}</b> emails {d.alert_to.join(", ")}. Reply to the alert to answer them directly.</> : "Off. When someone writes in through the site, it only lands in Messages here. Turn this on so you hear about it right away, by email or on your phone."}</p>
    <form className="ox-grid" onSubmit={save}>
      {!service && <>
        <label className="ox-f"><span>Sending address</span><input type="email" value={f.address} onChange={e => setF({ ...f, address: e.target.value })} placeholder={d.set && d.address ? d.address : "a Gmail address"} autoComplete="off" /><small>Only until Resend is set up above; then alerts come from alerts@ the domain</small></label>
        <label className="ox-f"><span>App password</span><input type="password" value={f.password} onChange={e => setF({ ...f, password: e.target.value })} placeholder={d.set ? "Kept on the server · leave empty to keep it" : "16 letters from Google"} autoComplete="new-password" /><small>Google Account → Security → 2-Step Verification → App passwords. Not your Gmail password.</small></label>
      </>}
      <label className="ox-f wide"><span>Send alerts to</span><input value={f.alert_to} onChange={e => setF({ ...f, alert_to: e.target.value })} placeholder="you@example.com, partner@example.com" /><small>Where each founder wants to hear about it, separated by commas. These addresses are never shown to anyone.</small></label>
      {error && <p className="ox-err wide" role="alert">{error}</p>}
      <div className="ox-row wide"><button className="ox-btn primary" disabled={busy}>{busy ? "Sending a test…" : d.set ? "Save and send a test" : "Turn on alerts"}</button>{d.set && <button type="button" className="ox-btn ghost" onClick={off}>Turn off</button>}</div>
    </form>
  </section>;
}

/** Addresses several people share (apply@, team@, support@): mail reaches everyone in it, any of them can send as it,
 * and each gets their copy at their own email if they have copies on. */
function SharedAddresses({ domain, toast }: { domain: string; toast: (s: string) => void }) {
  const [d, setD] = useState<{ shared: { local: string; name: string; members: string[] }[]; manage: boolean } | null>(null);
  const [people, setPeople] = useState<{ id: string; name: string; title?: string }[]>([]);
  const [edit, setEdit] = useState<{ was: string; local: string; name: string; members: string[] } | null>(null);
  const [error, setError] = useState(""); const [busy, setBusy] = useState(false);
  const load = () => api("shared").then(r => r.ok && setD(r));
  useEffect(() => { load(); Promise.all([api("me"), api("team")]).then(([m, t]) => setPeople([...(m.ok ? [{ id: m.me.id, name: m.me.name + " (you)", title: m.me.title }] : []), ...(t.ok ? t.contacts : [])])); }, []);
  if (!d) return null;
  const nameOf = (id: string) => people.find(p => p.id === id)?.name ?? id;
  const save = async (e: React.FormEvent) => {
    e.preventDefault(); if (!edit) return; setBusy(true); setError("");
    const r = await api("shared", edit); setBusy(false);
    if (r.ok) { toast(`${edit.local}@${domain} is ready.`); setEdit(null); load(); } else setError(r.error);
  };
  const del = async (local: string) => { if (!confirm(`Remove ${local}@${domain}? Mail to it would go to the host instead. Past email stays in Messages.`)) return; const r = await api("shared", { delete: true, local }); if (r.ok) { toast("Removed."); load(); } else toast(r.error); };
  return <section className="ox-card">
    <h2>Shared addresses</h2>
    <p className="ox-sub">An address several people share, like apply@ for applications or team@. Email to it reaches everyone in it (and their copies), and any of them can send as it from Messages. Replies come back to all of them.</p>
    {d.shared.length ? <ul className="ox-shared">{d.shared.map(x => <li key={x.local}>
      <div><b>{x.local}@{domain}</b>{x.name !== x.local && <small>{x.name}</small>}<span>{x.members.map(nameOf).join(", ")}</span></div>
      {d.manage && <div className="ox-row"><button className="ox-btn sm" onClick={() => setEdit({ was: x.local, local: x.local, name: x.name, members: x.members })}>Change</button><button className="ox-btn sm ghost" onClick={() => del(x.local)}>Remove</button></div>}
    </li>)}</ul> : <p className="ox-empty">None yet.</p>}
    {d.manage && !edit && <button className="ox-btn" onClick={() => setEdit({ was: "", local: "", name: "", members: [] })}>Add a shared address</button>}
    {edit && <form className="ox-grid" onSubmit={save}>
      <label className="ox-f"><span>Address</span><div className="ox-addr"><input required value={edit.local} onChange={e => setEdit({ ...edit, local: e.target.value.toLowerCase().replace(/\s/g, "") })} placeholder="apply" /><small>@{domain}</small></div></label>
      <label className="ox-f"><span>What it's for</span><input value={edit.name} onChange={e => setEdit({ ...edit, name: e.target.value })} placeholder="Applications and grants" /></label>
      <fieldset className="ox-f wide"><span>Goes to</span><div className="ox-checks">{people.map(p => <label key={p.id} className="ox-check"><input type="checkbox" checked={edit.members.includes(p.id)} onChange={e => setEdit({ ...edit, members: e.target.checked ? [...edit.members, p.id] : edit.members.filter(m => m !== p.id) })} /> {p.name}{p.title ? <small> · {p.title}</small> : null}</label>)}</div></fieldset>
      {error && <p className="ox-err wide" role="alert">{error}</p>}
      <div className="ox-row wide"><button className="ox-btn primary" disabled={busy}>{busy ? "Saving…" : edit.was ? "Save" : "Create it"}</button><button type="button" className="ox-btn ghost" onClick={() => { setEdit(null); setError(""); }}>Cancel</button></div>
    </form>}
  </section>;
}

function MailSetup({ toast }: { toast: (s: string) => void }) {
  const [d, setD] = useState<any>(null);
  const [testTo, setTestTo] = useState("");
  const [test, setTest] = useState<{ delivered: boolean; detail: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [key, setKey] = useState(""); const [keyErr, setKeyErr] = useState(""); const [keyBusy, setKeyBusy] = useState(false);
  const load = async () => { setD(null); const r = await api("mail/setup"); setD(r.ok ? r : { error: r.error }); };
  useEffect(() => { load(); }, []);
  const copy = (t: string) => navigator.clipboard?.writeText(t).then(() => toast("Copied."));
  if (!d) return <><Head title="Email setup" /><p className="ox-empty">Checking everything live…</p></>;
  if (d.error) return <><Head title="Email setup" /><p className="ox-err">{d.error}</p></>;
  const c = d.check, sv = d.service ?? { set: false };
  const verified = sv.set && (sv.status === "verified" || sv.status === "unknown");
  const row = (ok: boolean, title: string, detail: React.ReactNode) => <li className={ok ? "good" : "bad"}><b>{ok ? "✓" : "✗"} {title}</b><span>{detail}</span></li>;
  const saveKey = async (e: React.FormEvent) => {
    e.preventDefault(); setKeyBusy(true); setKeyErr("");
    const r = await api("mail/service", { key }); setKeyBusy(false);
    if (r.ok) { setKey(""); toast(r.status === "verified" ? "Saved. Sending goes through Resend now." : "Saved. Finish verifying the domain in Resend."); load(); } else setKeyErr(r.error);
  };
  const dropKey = async () => { if (!confirm("Stop sending through Resend? Mail would go straight from Agape's server again, which Gmail and Outlook refuse until the server has reverse DNS.")) return; await api("mail/service", { clear: true }); toast("Resend removed."); load(); };
  return <>
    <Head title="Email setup" sub={<>Everyone at Agape has their own @{d.domain} address. Mail comes in through Cloudflare and lands in Messages here. Mail goes out through Resend, from that same address. The account behind Resend is never shown to anyone.</>}><button className="ox-btn" onClick={load}>Check again</button></Head>
    <section className="ox-card">
      <h2>Right now</h2>
      <ul className="ox-checklist">
        {row(!!d.last_inbound, "Receiving", d.last_inbound ? <>Working. Last message came in {when(d.last_inbound)}.</> : c.mx.ok ? <>Cloudflare is set to receive, but nothing has arrived yet. Send any email to an @{d.domain} address; if it never shows up in Messages, redo “Receiving” at the bottom.</> : <>Not set up. See “Receiving” at the bottom.</>)}
        {row(verified, "Sending", !sv.set ? <>Not set up yet. Step 1 below, about ten minutes.</> : verified ? <>Through Resend{sv.status === "verified" ? ", domain verified" : ""}. Send a test below to be sure.</> : <>Resend has the key, but says the domain is <b>{sv.status}</b>. Finish step 1.2 in Resend; it usually verifies within minutes of the records going in.</>)}
      </ul>
    </section>
    <section className="ox-card">
      <h2>1 · Sending through Resend (free)</h2>
      <p className="ox-sub">Resend sends Agape's mail, signed for {d.domain}, so it lands in inboxes. Free up to 3,000 emails a month (100 a day).</p>
      <ol className="ox-steps">
        <li>Go to <a href="https://resend.com/signup" target="_blank" rel="noreferrer">resend.com</a> and sign up. Any email can be the login, a Gmail is fine: it's never shown to anyone you email.</li>
        <li><b>Domains → Add domain → {d.domain}.</b> Resend lists a few DNS records (on <code>send</code> and <code>resend._domainkey</code>). If it offers to add them to Cloudflare for you, let it. Otherwise add each in Cloudflare → {d.domain} → DNS → Records, with the proxy off (grey cloud). They don't touch receiving. Wait until Resend says <b>Verified</b>.</li>
        <li><b>API Keys → Create API key.</b> “Sending access” is enough. Copy it (it starts with <code>re_</code>) and paste it here. It's kept only on Agape's server.</li>
      </ol>
      {sv.set && <p className="ox-ok">Resend is set{sv.since ? ` since ${when(sv.since)}` : ""}. Paste a new key below only to replace it.</p>}
      <form className="ox-row" onSubmit={saveKey}>
        <input className="ox-search" type="password" required autoComplete="off" placeholder="re_…" value={key} onChange={e => setKey(e.target.value)} aria-label="Resend API key" />
        <button className="ox-btn primary" disabled={keyBusy}>{keyBusy ? "Checking with Resend…" : sv.set ? "Replace key" : "Save key"}</button>
        {sv.set && <button type="button" className="ox-btn ghost" onClick={dropKey}>Remove</button>}
      </form>
      {keyErr && <p className="ox-err" role="alert">{keyErr}</p>}
    </section>
    <section className="ox-card">
      <h2>2 · Test both ways</h2>
      <p className="ox-sub">Send a test to an address you can check, like your own Gmail. It comes from your @{d.domain} address. Then reply to it: the reply should show up here under Messages within a minute.</p>
      <form className="ox-row" onSubmit={async e => { e.preventDefault(); setBusy(true); const r = await api("mail/test", { to: testTo }); setBusy(false); setTest(r.ok ? r : { delivered: false, detail: r.error }); }}>
        <input className="ox-search" type="email" required placeholder="you@gmail.com" value={testTo} onChange={e => setTestTo(e.target.value)} /><button className="ox-btn primary" disabled={busy}>{busy ? "Sending…" : "Send a test"}</button>
      </form>
      {test && <p className={test.delivered ? "ox-ok" : "ox-err"}>{test.delivered ? "Sent. Check that it's in the inbox (not spam), then reply to it. " : "Not sent. "}{test.detail}</p>}
    </section>
    <SharedAddresses domain={d.domain} toast={toast} />
    <Alerts toast={toast} service={!!sv.set} />
    <details className="ox-card ox-fold">
      <summary><h2>Receiving: Cloudflare Email Routing</h2><small>{d.last_inbound ? "Working" : c.mx.ok ? "Set to Cloudflare; nothing received yet" : "Not set up"}</small></summary>
      <ol className="ox-steps">
        <li>Cloudflare → Workers & Pages → Create → Worker. Name it <b>agape-mail</b>, replace its code with the code below, and Deploy.</li>
        <li>Cloudflare → {d.domain} → Email → Email Routing → Get started. Let it replace any old MX records.</li>
        <li>Email Routing → Routing rules → Catch-all address → Action: <b>Send to a Worker</b> → agape-mail → Save, and turn it on.</li>
      </ol>
      <div className="ox-code"><button className="ox-btn sm" onClick={() => copy(d.worker)}>Copy code</button><pre>{d.worker}</pre></div>
      <p className="ox-fine">The code holds a secret only this server knows, so nobody else can drop mail into Agape. Don't share it.</p>
    </details>
    <details className="ox-card ox-fold">
      <summary><h2>Later: sending from Agape's own server</h2><small>Not needed while Resend sends</small></summary>
      <p className="ox-sub">Agape can also send straight from its server, without Resend. That needs reverse DNS, which only the server's host can set.</p>
      <ul className="ox-checklist">
        {row(c.port25.ok, "Agape's server can reach other mail servers", c.port25.ok ? "Port 25 is open." : <>{c.port25.detail}. Ask the server's host to allow outgoing mail (port 25) for 207.231.111.60.</>)}
        {row(c.mail_host.ok, `${d.host} points at the server`, c.mail_host.ok ? c.mail_host.ips.join(", ") : "Add the A record below.")}
        {row(c.reverse_dns.ok, "Reverse DNS names the server", c.reverse_dns.ok ? c.reverse_dns.names.join(", ") : <>Gmail and Outlook refuse mail from servers without this. Ask the server's host to set the reverse DNS (PTR) for 207.231.111.60 to <b>{d.host}</b>.{c.reverse_dns.names.length ? ` It's now ${c.reverse_dns.names.join(", ")}.` : ""}</>)}
        {row(c.spf.ok, "SPF lets the server send for the domain", c.spf.value || "Not set.")}
        {row(c.dkim.ok, "DKIM key published", c.dkim.ok ? "Matches the server's key." : "Add the DKIM record below.")}
        {row(c.dmarc.ok, "DMARC policy published", c.dmarc.value || "Not set.")}
      </ul>
      <div className="ox-table-wrap"><table className="ox-table"><thead><tr><th>Type</th><th>Name</th><th>Value</th><th></th></tr></thead>
        <tbody>{d.records.map((r: any) => <tr key={r.name + r.type}><td className="mono">{r.type}</td><td className="mono">{r.name}</td><td><code className="ox-wrap">{r.value}</code><small>{r.note}</small></td><td><button className="ox-btn sm" onClick={() => copy(r.value)}>Copy</button></td></tr>)}</tbody></table></div>
    </details>
  </>;
}

function Leads() {
  const [l, setL] = useState<any[] | null>(null);
  const [q, setQ] = useState("");
  useEffect(() => { api("leads").then(r => setL(r.ok ? r.leads : [])); }, []);
  const shown = (l ?? []).filter(x => !q.trim() || JSON.stringify(x).toLowerCase().includes(q.trim().toLowerCase()));
  return <>
    <Head title="Leads" sub="Everyone who asked to talk from the website, newest first.">
      <input className="ox-search" type="search" placeholder="Search leads" aria-label="Search leads" value={q} onChange={e => setQ(e.target.value)} />
    </Head>
    {l === null ? <p className="ox-empty">Loading…</p> : !shown.length ? <p className="ox-empty">{q ? "No leads match." : "Nobody has asked yet."}</p> : <div className="ox-leads">{shown.map((x, i) => <div key={i} className="ox-card lead">
      <div className="ox-row between"><b>{x.name}</b><small>{when(x.at)}</small></div>
      <div className="ox-meta">{[x.role, x.org].filter(Boolean).join(" · ")}</div>
      <a href={String(x.contact).includes("@") ? `mailto:${x.contact}` : `tel:${String(x.contact).replace(/[^\d+]/g, "")}`}>{x.contact}</a>
      {x.note && <p>{x.note}</p>}
    </div>)}</div>}
  </>;
}

const ROLE_NAMES: Record<string, string> = { fan: "Fan", runner: "Runner", stand: "Stand", business: "Partner business", player: "Player", ops: "Venue operations" };
/** Everyone with an account in the Agape apps themselves. People trying the demo show up under Leads when they ask to hear from Agape. */
function Accounts() {
  const [list, setList] = useState<any[] | null>(null);
  const [q, setQ] = useState("");
  const [kind, setKind] = useState("");
  useEffect(() => { api("accounts").then(r => setList(r.ok ? r.accounts : [])); }, []);
  const shown = (list ?? []).filter(a => (!kind || a.role === kind) && (!q.trim() || JSON.stringify(a).toLowerCase().includes(q.trim().toLowerCase())));
  const counts = (list ?? []).reduce((m: Record<string, number>, a) => ({ ...m, [a.role]: (m[a.role] ?? 0) + 1 }), {});
  return <>
    <Head title="App accounts" sub="Everyone with an account in the Agape apps: fans who signed up, and the runners, stands, partners and venue staff Agape set up. Passwords are never shown or kept readable. People who try the demo and ask to hear from Agape are under Leads.">
      <input className="ox-search" type="search" placeholder="Search accounts" aria-label="Search accounts" value={q} onChange={e => setQ(e.target.value)} />
    </Head>
    {list && <div className="ox-seg" role="group" aria-label="Kind of account"><button className={!kind ? "on" : ""} onClick={() => setKind("")}>All {list.length}</button>{Object.entries(counts).map(([k, n]) => <button key={k} className={kind === k ? "on" : ""} onClick={() => setKind(k)}>{ROLE_NAMES[k] ?? k} {String(n)}</button>)}</div>}
    {list === null ? <p className="ox-empty">Loading…</p> : !shown.length ? <p className="ox-empty">{q || kind ? "No accounts match." : "No app accounts yet."}</p> : <div className="ox-table-wrap"><table className="ox-table">
      <thead><tr><th>Name</th><th>Kind</th><th>Venue</th><th>Points</th><th>Nights</th><th>Been to</th></tr></thead>
      <tbody>{shown.map(a => <tr key={a.id}><td><b>{a.handle}</b><small className="mono">{a.id}{a.party && a.party !== a.id ? ` · ${a.party}` : ""}</small></td><td>{ROLE_NAMES[a.role] ?? a.role}</td>
        <td>{a.venue_name ?? a.at ?? "—"}</td><td className="mono">{a.points ?? "—"}{a.level != null && <small>Level {a.level}</small>}</td><td>{a.nights ?? "—"}</td><td><small>{(a.visited ?? []).join(", ") || "—"}</small></td></tr>)}</tbody>
    </table></div>}
  </>;
}

function Activity({ me }: { me: Me }) {
  const [log, setLog] = useState<{ at: number; who: string; what: string }[] | null>(null);
  const [trouble, setTrouble] = useState<any[]>([]);
  useEffect(() => { api("activity").then(r => setLog(r.ok ? r.log : [])); if (me.role === "host") api("trouble").then(r => r.ok && setTrouble(r.trouble)); }, []);
  return <>
    {trouble.length > 0 && <section className="ox-card">
      <h2>When people couldn't reach Agape</h2>
      <p className="ox-sub">Each time someone's browser tried a few times and never got an answer. Only you see this.</p>
      <ol className="ox-log">{trouble.slice(0, 20).map((t, i) => <li key={i}><time>{when(t.at)}</time><b>{t.page === "invitation" ? "Joining" : t.path}</b><span>{t.what} · {t.ua}</span></li>)}</ol>
    </section>}
    <Head title="Activity" sub={me.role === "host" ? "Every change anyone has made, and every printed VAR sheet, newest first. It can't be edited." : "Every change made by you and by everyone who reports up to you, newest first. It can't be edited."} />
    {log === null ? <p className="ox-empty">Loading…</p> : <ol className="ox-log">{log.map((e, i) => <li key={i}><time>{when(e.at)}</time><b>{e.who}</b><span>{e.what}</span></li>)}</ol>}
  </>;
}

function MailboxForm({ me, toast }: { me: Me; toast: (s: string) => void }) {
  const f = useForm({ mailbox: me.mailbox });
  return <form className="ox-row" onSubmit={e => f.run(e, async () => { const r = await api("mailbox", { mailbox: f.v.mailbox }); if (!r.ok) return r.error; toast("Address changed. Mail to the old one now goes to the host."); })}>
    <input className="ox-search mono" aria-label="Address" {...f.bind("mailbox")} /><span className="ox-muted">@{me.domain}</span><button className="ox-btn" disabled={f.busy || f.v.mailbox === me.mailbox}>Change</button>
    {f.error && <p className="ox-err">{f.error}</p>}
  </form>;
}

/** A copy of each email at the address they sign in with, so it reaches their phone. Replying to the copy answers
 * from their Agape address; their own address is never shown. */
function Copies({ me, toast }: { me: Me; toast: (s: string) => void }) {
  const [on, setOn] = useState(me.copies !== false);
  const atAgape = me.email.toLowerCase().endsWith("@" + me.domain);
  if (atAgape) return null;
  return <section className="ox-card narrow">
    <h2>Copies at your own email</h2>
    <p className="ox-sub">{on ? <>Every email to your Agape addresses also goes to <b>{me.email}</b>, attachments included. Reply to it from your phone or Gmail and it goes out from your Agape address; <b>{me.email}</b> is never shown to anyone.</> : <>Off. Email to your Agape addresses only shows up here, in Messages.</>}</p>
    <label className="ox-check"><input type="checkbox" checked={on} onChange={async e => { const v = e.target.checked; setOn(v); const r = await api("copies", { on: v }); toast(r.ok ? (v ? "Copies are on." : "Copies are off.") : r.error); }} /> Send me a copy of each email</label>
  </section>;
}

function Account({ me, toast, out }: { me: Me; toast: (s: string) => void; out: () => void }) {
  const f = useForm({ old: "", new: "", again: "" });
  return <>
    <Head title="Your account" sub={<>{me.name} · {me.email} · {me.role_name}</>} />
    {me.role === "host" && <section className="ox-card narrow">
      <h2>Back up everything</h2>
      <p className="ox-sub">The server keeps a copy of everything each day for two weeks, but on the same machine. Download one now and then and keep it somewhere else (your computer, a drive). It holds every account, venue, night and message. Passwords are only in protected form, but treat it as private.</p>
      <button className="ox-btn" onClick={async () => {
        try {
          const r = await fetch("/api/office/backup", { headers: { authorization: "Bearer " + token } });
          if (!r.ok) { toast("Couldn't make the backup. Try again."); return; }
          const url = URL.createObjectURL(await r.blob());
          const a = document.createElement("a"); a.href = url; a.download = `agape-backup-${new Date().toISOString().slice(0, 10)}.json`; document.body.append(a); a.click(); a.remove();
          setTimeout(() => URL.revokeObjectURL(url), 5000);
        } catch { toast("Couldn't download it. Check your connection."); }
      }}>Download a backup</button>
    </section>}
    <section className="ox-card narrow">
      <h2>Your email address</h2>
      <p className="ox-sub">Mail to it lands in Messages. People above you can read it, as with everything here.</p>
      <MailboxForm me={me} toast={toast} />
    </section>
    <Copies me={me} toast={toast} />
    <section className="ox-card narrow">
      <h2>Change your password</h2>
      <p className="ox-sub">You'll be signed out everywhere, including here.</p>
      <form onSubmit={e => f.run(e, async () => {
        if (f.v.new !== f.v.again) return "The two new passwords don't match";
        const r = await api("password", { old: f.v.old, new: f.v.new });
        if (!r.ok) return r.error;
        toast("Password changed. Sign in again."); setTimeout(out, 1200);
      })}>
        <label className="ox-f"><span>Current password</span><input type="password" required autoComplete="current-password" {...f.bind("old")} /></label>
        <label className="ox-f"><span>New password</span><input type="password" required minLength={10} autoComplete="new-password" {...f.bind("new")} /><small>At least 10 characters</small></label>
        <label className="ox-f"><span>New password again</span><input type="password" required autoComplete="new-password" {...f.bind("again")} /></label>
        {f.error && <p className="ox-err" role="alert">{f.error}</p>}
        <button className="ox-btn primary" disabled={f.busy}>Change password</button>
      </form>
    </section>
  </>;
}

/** If anything on the page breaks, say so in words, with a way back, instead of leaving a blank page. */
class Guard extends React.Component<{ children: React.ReactNode }, { error: string }> {
  state = { error: "" };
  static getDerivedStateFromError(e: any) { return { error: String(e?.message ?? e) }; }
  componentDidCatch(e: any) { console.error(e); }
  render() {
    if (!this.state.error) return this.props.children;
    return <div className="ox-gate"><div className="ox-gate-card"><Mark /><h1>Something went wrong on this page</h1>
      <p className="ox-sub">Your account and everything you saved are safe. Reload to try again. If it keeps happening, send Agape this line:</p>
      <p className="ox-err mono">{this.state.error} · {navigator.userAgent.replace(/^Mozilla\/5\.0 /, "")}</p>
      <button className="ox-btn primary block" onClick={() => location.reload()}>Reload</button>
      <button className="ox-link" style={{ marginTop: 12 }} onClick={() => { try { localStorage.removeItem(KEY); } catch {} location.href = "/admin/"; }}>Sign out and start over</button></div></div>;
  }
}
ReactDOM.createRoot(document.getElementById("root")!).render(<Guard><App /></Guard>);

// ---------------- venues: set up, staffed, and run for real ----------------

type VForm = any;
type VenueRow = { id: string; form: VForm; at: number; night: any | null; payouts?: Record<string, { status: string; account: string; at: number; by: string }>; staff: { id: string; handle: string; role: string; party: string }[] };
const money = (c: number | null | undefined) => c == null ? "—" : (c < 0 ? "−$" : "$") + (Math.abs(c) / 100).toFixed(2);
const slug = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "").slice(0, 24);
const FEATURE_NAMES: Record<string, string> = { chat: "Crowd chat", predictions: "Call the quarter", cards: "Moment cards", tickets: "Tickets and resale", around: "Rides, parking and partner offers", offers: "Sponsor offers", players: "Player deals" };
const STAGE_WORDS: Record<string, string> = { Accepted: "Waiting", Preparing: "Being made", Ready: "Ready", Assigned: "Runner on it", PickedUp: "On the way", Delivered: "Handed over", Settled: "Done", Disputed: "Problem" };
const stageWord = (s: string) => STAGE_WORDS[s] ?? s.replace(/([a-z])([A-Z])/g, "$1 $2");
const clockAt = (opened: number, t: number) => new Date((opened * 1000) + t).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
const fanLink = (id: string) => `${location.origin}/app/?v=${id}`;

function Venues({ me, toast }: { me: Me; toast: (s: string) => void }) {
  const [data, setData] = useState<{ venues: VenueRow[]; reports: any[]; starter: VForm; optional: string[] } | null>(null);
  const [error, setError] = useState("");
  const [view, setView] = useState<{ k: "edit"; v: VenueRow | null } | { k: "signs"; v: VenueRow } | { k: "staff"; id: string } | { k: "console"; id: string } | { k: "report"; file: string } | null>(null);
  const load = async () => { const r = await api("venues"); if (r.ok) setData(r); else setError(r.error); };
  useEffect(() => { load(); }, [view]);
  if (error) return <p className="ox-err">{error}</p>;
  if (!data) return <p className="ox-empty">Loading…</p>;
  if (view?.k === "signs") return <SeatSigns v={view.v} back={() => setView(null)} />;
  if (view?.k === "edit") return <VenueEditor v={view.v} starter={data.starter} optional={data.optional} toast={toast} done={() => setView(null)} />;
  if (view?.k === "staff") { const v = data.venues.find(x => x.id === view.id); return v ? <Staff v={v} toast={toast} reload={load} back={() => setView(null)} /> : null; }
  if (view?.k === "console") return <Console id={view.id} toast={toast} back={() => setView(null)} ended={file => setView({ k: "report", file })} />;
  if (view?.k === "report") return <Report file={view.file} back={() => setView(null)} />;
  const open = async (v: VenueRow, mode: "live" | "rehearsal") => {
    if (mode === "live" && !confirm(`Open a LIVE night at ${v.form.name}? Real fans will be able to order from the stands that are open.`)) return;
    const r = await api("venue/open", { id: v.id, mode });
    if (r.ok) { toast(mode === "live" ? "The night is open." : "The rehearsal is open."); setView({ k: "console", id: v.id }); }
    else setError(r.error + (r.issues ? ": " + r.issues.filter((i: any) => i.blocking).map((i: any) => i.message).join("; ") : ""));
  };
  return <>
    <Head title="Venues" sub="Real venues and their nights. Set a venue up, give its staff accounts, then open a night: a rehearsal first, with friends walking orders around, then live. Nothing here is simulated.">
      <button className="ox-btn primary" onClick={() => setView({ k: "edit", v: null })}>Add a venue</button>
    </Head>
    {!data.venues.length ? <div className="ox-blank">
      <h2>No venues yet</h2>
      <p>Start with the building: its sections, its stands and what each one sells. It takes about fifteen minutes with the venue's menu in front of you, and you can save it half done.</p>
      <button className="ox-btn primary" onClick={() => setView({ k: "edit", v: null })}>Add a venue</button>
    </div> : <div className="ox-venues">{data.venues.map(v => {
      const n = v.night;
      const stands = (v.form.stands ?? []).length, items = (v.form.menu ?? []).length;
      return <section key={v.id} className={"ox-card ox-venue" + (n ? " live" : "")}>
        <div className="ox-row between wrap">
          <div><h2>{v.form.name || v.id}</h2><small>{[v.form.address, `${v.form.levels} level${v.form.levels == 1 ? "" : "s"} · ${stands} stand${stands === 1 ? "" : "s"} · ${items} item${items === 1 ? "" : "s"} · ${v.form.runners} runner places`].filter(Boolean).join(" · ")}</small></div>
          {n ? <span className={"ox-tag " + (n.mode === "live" ? "live" : "requested")}>{n.mode === "live" ? "Live now" : "Rehearsal open"}</span> : <span className="ox-tag draft">No night open</span>}
        </div>
        {n && <p className="ox-vstat"><b>{n.clock?.label}</b>{n.game && <span>{n.game.score?.[0]}–{n.game.score?.[1]}</span>}<span>{n.orders} order{n.orders === 1 ? "" : "s"}</span><span>{n.stands_open}/{stands} stands open</span><span>{n.runners_on} runner{n.runners_on === 1 ? "" : "s"} on</span><span>{n.people} fan{n.people === 1 ? "" : "s"} in</span>{n.halt && <span className="ox-flag">Ordering paused</span>}</p>}
        <div className="ox-row wrap">
          {n ? <button className="ox-btn primary" onClick={() => setView({ k: "console", id: v.id })}>Open the console</button> : <>
            <button className="ox-btn" onClick={() => open(v, "rehearsal")}>Open a rehearsal</button>
            <button className="ox-btn primary" onClick={() => open(v, "live")}>Open a live night</button>
          </>}
          <button className="ox-btn ghost" onClick={() => setView({ k: "staff", id: v.id })}>Staff ({v.staff.length})</button>
          <button className="ox-btn ghost" onClick={() => setView({ k: "signs", v })}>Seat signs</button>
          <button className="ox-btn ghost" onClick={() => setView({ k: "edit", v })} disabled={!!n} title={n ? "End the night to change the venue" : undefined}>Edit venue</button>
        </div>
        <Payouts v={v} toast={toast} reload={load} />
        <p className="ox-fine">Fans open <a href={fanLink(v.id)} target="_blank" rel="noopener">{fanLink(v.id).replace(/^https?:\/\//, "")}</a> <button className="ox-link" onClick={() => { navigator.clipboard?.writeText(fanLink(v.id)); toast("Link copied"); }}>Copy</button></p>
      </section>;
    })}</div>}
    {data.reports.length > 0 && <>
      <h3>Past nights</h3>
      <div className="ox-table-wrap"><table className="ox-table">
        <thead><tr><th>Venue</th><th>Kind</th><th>Opened</th><th>Closed</th><th>Orders</th><th>Fans paid</th><th>Books</th><th></th></tr></thead>
        <tbody>{data.reports.map(r => <tr key={r.file}><td><b>{r.name}</b></td><td>{r.mode === "live" ? "Live" : "Rehearsal"}</td><td><small>{when(r.opened)}</small></td><td><small>{when(r.closed)}</small></td>
          <td className="mono">{r.orders ?? 0}</td><td className="mono">{money(r.fans_paid)}</td><td>{r.conserved ? <span className="ox-tag live">Balance</span> : <span className="ox-tag bad">Check</span>}</td>
          <td><button className="ox-btn sm" onClick={() => setView({ k: "report", file: r.file })}>Report</button></td></tr>)}</tbody>
      </table></div>
    </>}
  </>;
}

/** The venue, as a form: the kernel checks it exactly as it checks any venue. */
function VenueEditor({ v, starter, optional, toast, done }: { v: VenueRow | null; starter: VForm; optional: string[]; toast: (s: string) => void; done: () => void }) {
  const [f, setF] = useState<VForm>(() => JSON.parse(JSON.stringify(v?.form ?? starter)));
  const [issues, setIssues] = useState<{ path: string; message: string; blocking: boolean }[]>([]);
  const [busy, setBusy] = useState(false);
  const [changed, setChanged] = useState(false);
  const [error, setError] = useState("");
  const set = (path: (string | number)[], val: any) => setF((old: VForm) => { const n = JSON.parse(JSON.stringify(old)); let o = n; for (const k of path.slice(0, -1)) o = o[k] ??= {}; o[path[path.length - 1]] = val; setChanged(true); return n; });
  useEffect(() => { const t = setTimeout(async () => { const r = await api("venue/check", { form: f }); if (r.ok) setIssues(r.issues); }, 500); return () => clearTimeout(t); }, [f]);
  const save = async () => { setBusy(true); setError(""); const r = await api("venue", { form: f }); setBusy(false); if (r.ok) { setChanged(false); toast("Saved."); if (!v) done(); } else setError(r.error); };
  const num = (x: any) => x === "" || x == null ? "" : String(x);
  const blocking = issues.filter(i => i.blocking), warns = issues.filter(i => !i.blocking);
  const stands: any[] = f.stands ?? [], menu: any[] = f.menu ?? [];
  const payees: any[] = f.payees?.length ? f.payees : [{ id: "concessions", name: "" }];
  const leave = () => { if (changed && !confirm("Leave without saving your changes?")) return; done(); };
  return <>
    <Head title={v ? f.name || v.id : "Add a venue"} sub="Everything a night at this venue needs. Save as you go; a night can only open once nothing is flagged.">
      <button className="ox-btn ghost" onClick={leave}>Back to venues</button>
    </Head>
    <div className="ox-edit">
      <div className="ox-sections">
        <section className="ox-card"><h2>The venue</h2><p className="ox-sub">How fans know it. The id is used in its link and can't change once saved.</p>
          <div className="ox-grid">
            <label className="ox-f"><span>Name</span><input value={f.name} placeholder="Riverside Arena" onChange={e => { set(["name"], e.target.value); if (!v) set(["id"], slug(e.target.value)); }} /></label>
            <label className="ox-f"><span>Id</span><input className="mono" value={f.id} disabled={!!v} onChange={e => set(["id"], slug(e.target.value))} /><small>Fans open /app/?v={f.id || "id"}</small></label>
            <label className="ox-f wide"><span>Address</span><input value={f.address} onChange={e => set(["address"], e.target.value)} /></label>
            <label className="ox-f"><span>Capacity</span><input inputMode="numeric" value={num(f.capacity)} onChange={e => set(["capacity"], parseInt(e.target.value) || 0)} /></label>
          </div>
        </section>
        <section className="ox-card"><h2>Tonight's teams</h2><p className="ox-sub">As the venue shows them. Change these before each night.</p>
          {(["home", "away"] as const).map(side => <div key={side} className="ox-grid">
            <label className="ox-f"><span>{side === "home" ? "Home team" : "Visitors"}</span><input value={f[side]?.name ?? ""} placeholder={side === "home" ? "Riverside Hawks" : "Northfield Foxes"} onChange={e => set([side, "name"], e.target.value)} /></label>
            <label className="ox-f"><span>Short name</span><input value={f[side]?.short ?? ""} placeholder={side === "home" ? "Hawks" : "Foxes"} onChange={e => set([side, "short"], e.target.value)} /></label>
            <label className="ox-f"><span>Abbreviation</span><input value={f[side]?.abbr ?? ""} maxLength={4} placeholder={side === "home" ? "RIV" : "NOR"} onChange={e => set([side, "abbr"], e.target.value.toUpperCase())} /></label>
            <label className="ox-f"><span>Color</span><input type="color" value={f[side]?.color || "#888888"} onChange={e => set([side, "color"], e.target.value)} /></label>
          </div>)}
        </section>
        <section className="ox-card"><h2>The building</h2><p className="ox-sub">Sections are numbered by level: level 1 is 101, 102…, level 2 is 201… Stands sit next to a section on their level, which is how runners' walks are worked out.</p>
          <div className="ox-grid">
            <label className="ox-f"><span>Levels</span><input inputMode="numeric" value={num(f.levels)} onChange={e => set(["levels"], parseInt(e.target.value) || 0)} /><small>1 to 6</small></label>
            <label className="ox-f"><span>Sections on each level</span><input inputMode="numeric" value={num(f.sections_per_level)} onChange={e => set(["sections_per_level"], parseInt(e.target.value) || 0)} /><small>4 to 60</small></label>
            <label className="ox-f"><span>Seconds between neighbouring sections</span><input inputMode="numeric" value={num(f.walk?.ring_s)} onChange={e => set(["walk", "ring_s"], parseInt(e.target.value) || 0)} /><small>Walking the concourse</small></label>
            <label className="ox-f"><span>Seconds from concourse to a seat</span><input inputMode="numeric" value={num(f.walk?.aisle_s)} onChange={e => set(["walk", "aisle_s"], parseInt(e.target.value) || 0)} /></label>
            <label className="ox-f"><span>Seconds by stairs, one level</span><input inputMode="numeric" value={num(f.walk?.stairs_s)} onChange={e => set(["walk", "stairs_s"], parseInt(e.target.value) || 0)} /></label>
          </div>
        </section>
        <section className="ox-card"><h2>Stands</h2><p className="ox-sub">Each place that makes orders. Stations are how many orders it can make at once.</p>
          <div className="ox-table-wrap"><table className="ox-table ox-formtable"><thead><tr><th>Name</th><th>Id</th><th>Level</th><th>Next to section</th><th>Stations</th><th></th></tr></thead>
            <tbody>{stands.map((s, i) => <tr key={i}>
              <td><input aria-label="Stand name" value={s.name} onChange={e => { const old = s.id; set(["stands", i, "name"], e.target.value); if (!old || old === slug(s.name)) { const id = slug(e.target.value); set(["stands", i, "id"], id); menu.forEach((m, j) => m.stand === old && set(["menu", j, "stand"], id)); } }} /></td>
              <td><input aria-label="Stand id" className="mono" value={s.id} onChange={e => { const old = s.id, id = slug(e.target.value); set(["stands", i, "id"], id); menu.forEach((m, j) => m.stand === old && set(["menu", j, "stand"], id)); }} /></td>
              <td><input aria-label="Level" inputMode="numeric" value={num(s.level)} onChange={e => set(["stands", i, "level"], parseInt(e.target.value) || 0)} /></td>
              <td><input aria-label="Next to section" inputMode="numeric" value={num(s.near)} onChange={e => set(["stands", i, "near"], parseInt(e.target.value) || 0)} /><small>{s.level && s.near ? `Section ${s.level}${String(s.near).padStart(2, "0")}` : ""}</small></td>
              <td><input aria-label="Stations" inputMode="numeric" value={num(s.stations)} onChange={e => set(["stands", i, "stations"], parseInt(e.target.value) || 0)} /></td>
              <td><button className="ox-link danger" onClick={() => set(["stands"], stands.filter((_, j) => j !== i))}>Remove</button></td>
            </tr>)}</tbody></table></div>
          <button className="ox-btn sm" onClick={() => set(["stands"], [...stands, { id: "", name: "", level: 1, near: 1, stations: 2, payee: payees[0]?.id ?? "concessions" }])}>Add a stand</button>
        </section>
        <section className="ox-card"><h2>Who gets paid</h2><p className="ox-sub">Fans pay Agape. After each night Agape keeps its share and pays each business for its stands, so nobody ever has to be collected from. Usually one business runs every stand: the venue or its concessions company.</p>
          {payees.map((p, i) => <div key={i} className="ox-grid">
            <label className="ox-f wide"><span>Business name</span><input value={p.name} placeholder="Riverside Arena Concessions" onChange={e => { set(["payees", i, "name"], e.target.value); if (!p.id || (i > 0 && p.id === slug(p.name))) set(["payees", i, "id"], i === 0 && !p.id ? "concessions" : slug(e.target.value)); }} /><small>As the processor will know them</small></label>
            {payees.length > 1 && <button className="ox-link danger" onClick={() => { set(["payees"], payees.filter((_, j) => j !== i)); stands.forEach((st, j) => st.payee === p.id && set(["stands", j, "payee"], payees[0].id)); }}>Remove</button>}
          </div>)}
          <button className="ox-btn sm" onClick={() => set(["payees"], [...payees, { id: "", name: "" }])}>Add another business</button>
          {payees.length > 1 && <div className="ox-table-wrap"><table className="ox-table ox-formtable"><thead><tr><th>Stand</th><th>Paid to</th></tr></thead>
            <tbody>{stands.map((st, i) => <tr key={i}><td>{st.name || st.id}</td><td><select aria-label="Paid to" value={st.payee ?? payees[0]?.id} onChange={e => set(["stands", i, "payee"], e.target.value)}>{payees.map(p => <option key={p.id} value={p.id}>{p.name || p.id}</option>)}</select></td></tr>)}</tbody></table></div>}
        </section>
        <section className="ox-card"><h2>Menu</h2><p className="ox-sub">What each stand sells, at the venue's prices. Make time is the usual minutes from starting the order to handing it over. Leave stock empty for no limit.</p>
          <div className="ox-table-wrap"><table className="ox-table ox-formtable"><thead><tr><th>Item</th><th>Stand</th><th>Price</th><th>Make time (min)</th><th>Kind</th><th>21+</th><th>Stock</th><th></th></tr></thead>
            <tbody>{menu.map((m, i) => <tr key={i}>
              <td><input aria-label="Item name" value={m.name} onChange={e => { set(["menu", i, "name"], e.target.value); if (!m.id || m.id === slug(m.name)) set(["menu", i, "id"], slug(e.target.value)); }} /></td>
              <td><select aria-label="Stand" value={m.stand} onChange={e => set(["menu", i, "stand"], e.target.value)}><option value="">—</option>{stands.map(s => <option key={s.id} value={s.id}>{s.name || s.id}</option>)}</select></td>
              <td><input aria-label="Price in dollars" inputMode="decimal" defaultValue={m.price ? (m.price / 100).toFixed(2) : ""} onBlur={e => set(["menu", i, "price"], Math.round(parseFloat(e.target.value.replace(/[$,]/g, "")) * 100) || 0)} /></td>
              <td><input aria-label="Make time in minutes" inputMode="decimal" defaultValue={m.prep_s ? String(Math.round(m.prep_s / 6) / 10) : ""} onBlur={e => set(["menu", i, "prep_s"], Math.round(parseFloat(e.target.value) * 60) || 0)} /></td>
              <td><select aria-label="Kind" value={m.category} onChange={e => set(["menu", i, "category"], e.target.value)}><option value="food">Food</option><option value="drink">Drink</option><option value="merch">Merch</option></select></td>
              <td><input aria-label="21 and over" type="checkbox" checked={!!m.adult} onChange={e => set(["menu", i, "adult"], e.target.checked)} /></td>
              <td><input aria-label="Stock" inputMode="numeric" value={m.stock ?? ""} placeholder="No limit" onChange={e => set(["menu", i, "stock"], e.target.value === "" ? undefined : parseInt(e.target.value) || 0)} /></td>
              <td><button className="ox-link danger" onClick={() => set(["menu"], menu.filter((_, j) => j !== i))}>Remove</button></td>
            </tr>)}</tbody></table></div>
          <button className="ox-btn sm" onClick={() => set(["menu"], [...menu, { id: "", name: "", stand: stands[0]?.id ?? "", price: 0, prep_s: 120, category: "food" }])}>Add an item</button>
        </section>
        <section className="ox-card"><h2>Fees and runners</h2>
          <div className="ox-grid">
            <label className="ox-f"><span>Delivery fee</span><input inputMode="decimal" defaultValue={((f.fees?.delivery_fee ?? 0) / 100).toFixed(2)} onBlur={e => set(["fees", "delivery_fee"], Math.round(parseFloat(e.target.value.replace(/[$,]/g, "")) * 100) || 0)} /><small>Per delivered order; pickup is free</small></label>
            <label className="ox-f"><span>Sales tax (%)</span><input inputMode="decimal" defaultValue={((f.fees?.tax_bps ?? 0) / 100).toString()} onBlur={e => set(["fees", "tax_bps"], Math.round(parseFloat(e.target.value) * 100) || 0)} /></label>
            <label className="ox-f wide"><span>How fans pay tonight</span><input value={f.pay_note ?? ""} placeholder="Payments aren't connected yet, so nothing is charged to your card in the app." onChange={e => set(["pay_note"], e.target.value)} /><small>Shown to fans on a real night. Leave empty for the wording above.</small></label>
            <label className="ox-f"><span>Runner places</span><input inputMode="numeric" value={num(f.runners)} onChange={e => set(["runners"], parseInt(e.target.value) || 0)} /><small>The most runners on at once; each gets an account</small></label>
          </div>
        </section>
        <section className="ox-card"><h2>Optional</h2><p className="ox-sub">At launch fans order to their seat or for pickup, see honest arrival times and earn points. Anything else is off unless the venue wants it.</p>
          <div className="ox-checks">{optional.map(k => <label key={k} className="ox-checkline"><input type="checkbox" checked={f.features?.[k] === true} onChange={e => set(["features", k], e.target.checked)} /><span>{FEATURE_NAMES[k] ?? k}</span></label>)}</div>
        </section>
      </div>
      <aside className="ox-side">
        <div className="ox-card sticky">
          <h2>Ready to open?</h2>
          {blocking.length ? <><p className="ox-sub">Fix these before a night can open:</p><ul className="ox-missing one">{blocking.map((i, k) => <li key={k}>{i.message}</li>)}</ul></> : <p className="ox-ok">Nothing is in the way of opening a night.</p>}
          {warns.length > 0 && <><h3>Worth a look</h3><ul className="ox-missing one">{warns.map((i, k) => <li key={k}>{i.message}</li>)}</ul></>}
          {error && <p className="ox-err" role="alert">{error}</p>}
          <button className="ox-btn primary block" disabled={busy || !changed || !f.id} onClick={save}>{busy ? "Saving…" : changed ? "Save venue" : "Saved"}</button>
          {v && <DeleteVenue id={v.id} toast={toast} done={done} />}
        </div>
      </aside>
    </div>
  </>;
}

function DeleteVenue({ id, toast, done }: { id: string; toast: (s: string) => void; done: () => void }) {
  const [sure, setSure] = useState(false);
  const [error, setError] = useState("");
  return sure ? <>{error && <p className="ox-err">{error}</p>}<div className="ox-row"><button className="ox-btn ghost sm" onClick={() => setSure(false)}>Keep it</button><button className="ox-btn danger sm" onClick={async () => { const r = await api("venue/delete", { id }); if (r.ok) { toast("Deleted. Its past nights' reports are kept."); done(); } else setError(r.error); }}>Delete the venue</button></div></>
    : <button className="ox-link danger" onClick={() => setSure(true)}>Delete this venue</button>;
}

/** Staff accounts: made here, the password shown once to hand over. */
function Staff({ v, toast, reload, back }: { v: VenueRow; toast: (s: string) => void; reload: () => void; back: () => void }) {
  const [role, setRole] = useState("runner");
  const [stand, setStand] = useState(v.form.stands?.[0]?.id ?? "");
  const [handle, setHandle] = useState("");
  const [given, setGiven] = useState<{ handle: string; password: string; what: string } | null>(null);
  const [error, setError] = useState("");
  const standName = (id: string) => v.form.stands?.find((s: any) => s.id === id)?.name ?? id;
  const what = (a: { role: string; party: string }) => a.role === "runner" ? `Runner (place ${parseInt(a.party.replace("runner", "")) + 1})` : a.role === "stand" ? `Stand · ${standName(a.party)}` : "Venue operations";
  const add = async (e: React.FormEvent) => {
    e.preventDefault(); setError("");
    const r = await api("venue/staff", { venue: v.id, role, handle: handle.trim(), stand });
    if (r.ok) { setGiven({ handle: r.account.handle, password: r.password, what: what(r.account) }); setHandle(""); reload(); } else setError(r.error);
  };
  const reset = async (a: { id: string; handle: string; role: string; party: string }) => {
    if (!confirm(`Give ${a.handle} a new password? The old one stops working right away and they're signed out everywhere.`)) return;
    const r = await api("venue/staff/reset", { account: a.id });
    if (r.ok) setGiven({ handle: a.handle, password: r.password, what: what(a) }); else toast(r.error);
  };
  const order: Record<string, number> = { ops: 0, stand: 1, runner: 2 };
  return <>
    <Head title={`Staff at ${v.form.name}`} sub="Each person working a night signs in on their own phone or tablet at the same address fans use. Runners take trips, stands mark food ready, and venue operations can pause ordering and mark the game.">
      <button className="ox-btn ghost" onClick={back}>Back to venues</button>
    </Head>
    {given && <div className="ox-card ox-given" role="status">
      <h2>Hand this to {given.handle}</h2>
      <p className="ox-sub">It's shown only now. Write it down or copy it before leaving this page.</p>
      <dl><dt>Open</dt><dd className="mono">{location.host}/app/</dd><dt>Name</dt><dd className="mono">{given.handle}</dd><dt>Password</dt><dd className="mono big">{given.password}</dd><dt>Works as</dt><dd>{given.what}</dd></dl>
      <div className="ox-row"><button className="ox-btn" onClick={() => { navigator.clipboard?.writeText(`Agape at ${v.form.name}\nOpen ${location.host}/app/\nName: ${given.handle}\nPassword: ${given.password}\nYou work as: ${given.what}`); toast("Copied"); }}>Copy</button><button className="ox-btn ghost" onClick={() => setGiven(null)}>Done</button></div>
    </div>}
    <section className="ox-card"><h2>Give someone an account</h2>
      <form className="ox-grid" onSubmit={add}>
        <label className="ox-f"><span>Works as</span><select value={role} onChange={e => setRole(e.target.value)}><option value="runner">Runner</option><option value="stand">Stand</option><option value="ops">Venue operations</option></select></label>
        {role === "stand" && <label className="ox-f"><span>Stand</span><select value={stand} onChange={e => setStand(e.target.value)}>{(v.form.stands ?? []).map((s: any) => <option key={s.id} value={s.id}>{s.name}</option>)}</select><small>One account per tablet; a stand can have several</small></label>}
        <label className="ox-f"><span>Sign-in name</span><input value={handle} onChange={e => setHandle(e.target.value.replace(/\s/g, ""))} placeholder={`${v.id}.maria`} required minLength={3} maxLength={24} autoCapitalize="none" /><small>Letters, numbers, dots</small></label>
        <div className="ox-f"><span>&nbsp;</span><button className="ox-btn primary">Make the account</button></div>
      </form>
      {error && <p className="ox-err">{error}</p>}
    </section>
    {v.staff.length > 0 && <div className="ox-table-wrap"><table className="ox-table">
      <thead><tr><th>Name</th><th>Works as</th><th></th></tr></thead>
      <tbody>{[...v.staff].sort((a, b) => (order[a.role] - order[b.role]) || a.party.localeCompare(b.party, undefined, { numeric: true })).map(a => <tr key={a.id}><td><b className="mono">{a.handle}</b></td><td>{what(a)}</td><td><button className="ox-btn sm ghost" onClick={() => reset(a)}>New password</button></td></tr>)}</tbody>
    </table></div>}
  </>;
}

/** The night as it happens: the game, every stand and runner, every order, and the books. */
function Console({ id, toast, back, ended }: { id: string; toast: (s: string) => void; back: () => void; ended: (file: string) => void }) {
  const [n, setN] = useState<any>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [filter, setFilter] = useState<"open" | "all">("open");
  const load = async () => { const r = await api(`night/${id}`); if (r.ok) { setN(r); setError(""); } else setError(r.error); };
  useEffect(() => { load(); const t = setInterval(load, 3000); return () => clearInterval(t); }, [id]);
  const act = async (body: any, said?: string) => { setBusy(true); const r = await api(`night/${id}/act`, body); setBusy(false); if (r.ok) { if (said) toast(said); load(); } else toast(r.error); };
  if (error && !n) return <><Head title="Console"><button className="ox-btn ghost" onClick={back}>Back to venues</button></Head><p className="ox-err">{error}</p></>;
  if (!n) return <p className="ox-empty">Loading…</p>;
  const g = n.game, b = n.books, tot = b.totals ?? {};
  const [home, away] = n.teams ?? [];
  const staff: Record<string, string> = n.staff ?? {};
  const orders: any[] = (n.orders ?? []).filter((o: any) => filter === "all" || !["Settled"].includes(o.stage) && !o.stage.startsWith("Cancel") && !o.stage.startsWith("Cannot") && !o.stage.startsWith("Refund") && !o.stage.startsWith("Unreach"));
  const openCount = tot.n_open ?? 0;
  const end = async () => {
    if (!confirm(`End the night at ${n.venue}? Fans' points go home to their passports and the report is written.`)) return;
    setBusy(true); const r = await api(`night/${id}/end`, {}); setBusy(false);
    if (r.ok) { toast("The night is closed."); ended(r.report); } else toast(r.error);
  };
  const pause = () => { const why = prompt("Why is ordering paused? Fans see this.", "Ordering is paused for a few minutes"); if (why) act({ do: "halt", reason: why }, "Ordering paused"); };
  const period = g.period, next = g.can.start ? `Start ${period === 0 ? "the game" : period === 2 ? "the second half" : `Q${period + 1}`}` : null;
  return <>
    <Head title={n.venue} sub={<>{n.mode === "live" ? <span className="ox-tag live">Live</span> : <span className="ox-tag requested">Rehearsal</span>} Opened {when(n.opened)} · it's {clockAt(n.opened, n.clock.t)} · {n.people} fan{n.people === 1 ? "" : "s"} in the app</>}>
      <button className="ox-btn ghost" onClick={back}>Back to venues</button>
    </Head>
    {n.halt && <div className="ox-note ox-row between wrap"><span><b>Ordering is paused:</b> {n.halt.reason}</span><button className="ox-btn primary" disabled={busy} onClick={() => act({ do: "resume" }, "Ordering is back on")}>Resume ordering</button></div>}
    <div className="ox-console">
      <section className="ox-card ox-game">
        <div className="ox-score">
          <span className="ox-team"><i style={{ background: home?.color }} />{home?.short || "Home"}</span><b className="mono">{g.score[0]}</b>
          <span className="ox-period">{n.clock.label}</span>
          <b className="mono">{g.score[1]}</b><span className="ox-team right">{away?.short || "Away"}<i style={{ background: away?.color }} /></span>
        </div>
        <div className="ox-row wrap center">
          {next && <button className="ox-btn primary" disabled={busy} onClick={() => act({ do: "game", what: "start" }, next)}>{next}</button>}
          {g.can.end && <button className="ox-btn primary" disabled={busy} onClick={() => act({ do: "game", what: "end" }, period === 2 ? "Halftime" : `End of Q${period}`)}>{period === 2 ? "Halftime" : `End Q${period}`}</button>}
          {g.can.final && <button className="ox-btn" disabled={busy} onClick={() => confirm("Mark the game final? Ordering closes for the night.") && act({ do: "game", what: "final" }, "Final")}>Final</button>}
        </div>
        {!g.final && <div className="ox-scorepad">{(["home", "away"] as const).map((t, k) => <div key={t}><small>{(k ? away : home)?.short || t}</small><div className="ox-row">{[1, 2, 3].map(p => <button key={p} className="ox-btn sm" disabled={busy} onClick={() => act({ do: "score", team: t, points: p })}>+{p}</button>)}<button className="ox-btn sm ghost" title="Take a point back" disabled={busy || g.score[k] === 0} onClick={() => act({ do: "score", team: t, points: -1 }, "Corrected")}>−1</button></div></div>)}</div>}
        <p className="ox-fine">The game moves only when you mark it here: fans see the period and score, and ordering closes at the final.</p>
      </section>
      <section className="ox-card ox-kpis">
        <div><small>Orders</small><b className="mono">{(n.orders ?? []).length}</b></div>
        <div><small>Still open</small><b className="mono">{openCount}</b></div>
        <div><small>Delivered</small><b className="mono">{tot.n_settled ?? 0}</b></div>
        <div><small>To the seat, typical</small><b className="mono">{n.wait_p50 != null ? `${Math.round(n.wait_p50 / 60)} min` : "—"}</b></div>
        <div><small>Fans paid</small><b className="mono">{money(b.fans_paid)}</b></div>
        <div><small>Books</small><b className={b.conserved ? "ox-okt" : "ox-badt"}>{b.conserved ? "Balance" : "Off"}</b></div>
        <div className="ox-row wrap">{!n.halt && <button className="ox-btn" disabled={busy} onClick={pause}>Pause ordering</button>}
          <button className="ox-btn danger" disabled={busy || openCount > 0} title={openCount > 0 ? "Every order must be finished first" : undefined} onClick={end}>End the night</button></div>
      </section>
    </div>
    <div className="ox-console two">
      <section className="ox-card"><h2>Stands</h2>
        <table className="ox-table tight"><thead><tr><th>Stand</th><th>Open</th><th>Waiting</th><th>Making</th><th>Ready</th><th>Sales</th></tr></thead>
          <tbody>{n.stands.map((s: any) => <tr key={s.id}><td><b>{s.name}</b>{s.wait_s > 0 && <small>~{Math.ceil(s.wait_s / 60)} min wait</small>}</td><td>{s.open ? <span className="ox-tag live">Open</span> : <span className="ox-tag draft">Closed</span>}</td><td className="mono">{s.queued}</td><td className="mono">{s.making}</td><td className="mono">{s.ready}</td><td className="mono">{money(s.sales)}</td></tr>)}</tbody></table>
        <p className="ox-fine">A stand opens when someone signs in on its tablet and starts their shift. Fans can't order from a closed stand.</p>
      </section>
      <section className="ox-card"><h2>Runners</h2>
        <table className="ox-table tight"><thead><tr><th>Runner</th><th>On</th><th>Now</th><th>Delivered</th><th>Earned</th></tr></thead>
          <tbody>{n.runners.filter((r: any) => staff[r.id] || r.on_shift || r.delivered).map((r: any) => <tr key={r.id}><td><b>{staff[r.id] ?? `Place ${parseInt(r.id.slice(6)) + 1}`}</b></td><td>{r.on_shift ? <span className="ox-tag live">On</span> : <span className="ox-tag draft">Off</span>}</td>
            <td><small>{r.trip ? (r.trip.picked ? `Delivering ${r.trip.orders.length}` : r.trip.accepted ? `Picking up ${r.trip.orders.length}` : `Offered ${r.trip.orders.length}`) : r.on_shift ? "Waiting for a trip" : ""}</small></td>
            <td className="mono">{r.delivered}</td><td className="mono">{money(r.pay + r.tips)}</td></tr>)}</tbody></table>
        {!Object.keys(staff).some(k => k.startsWith("runner")) && <p className="ox-fine">No runner accounts yet. Add them under Staff.</p>}
      </section>
    </div>
    {(n.problems ?? []).filter((p: any) => p.state === "Open" || p.state === "open").length > 0 && <section className="ox-card"><h2>Problems fans reported</h2>
      {n.problems.filter((p: any) => p.state === "Open" || p.state === "open").map((p: any) => <div key={p.id} className="ox-row between wrap ox-problem"><span><b>#{p.order}</b> {p.who}: {String(p.reason).replace(/_/g, " ")}{p.words ? ` · “${p.words}”` : ""}</span>
        <span className="ox-row"><button className="ox-btn sm" disabled={busy} onClick={() => act({ do: "resolve", problem: p.id, remedy: true, form: "refund", why: "Settled by the venue's team" }, "Refunded")}>Refund</button><button className="ox-btn sm" disabled={busy} onClick={() => act({ do: "resolve", problem: p.id, remedy: true, form: "credit", why: "Settled by the venue's team" }, "Credit given")}>Credit</button><button className="ox-btn sm ghost" disabled={busy} onClick={() => act({ do: "resolve", problem: p.id, remedy: false, why: "Looked into it" }, "Closed")}>No remedy</button></span></div>)}
    </section>}
    <section className="ox-card">
      <div className="ox-row between wrap"><h2>Orders</h2><div className="ox-seg"><button className={filter === "open" ? "on" : ""} onClick={() => setFilter("open")}>In progress</button><button className={filter === "all" ? "on" : ""} onClick={() => setFilter("all")}>All {(n.orders ?? []).length}</button></div></div>
      {!orders.length ? <p className="ox-empty">{filter === "open" ? "Nothing in progress." : "No orders yet."}</p> : <div className="ox-table-wrap"><table className="ox-table tight">
        <thead><tr><th>#</th><th>Placed</th><th>Fan</th><th>To</th><th>From</th><th>Items</th><th>Where it is</th><th>Runner</th><th>Total</th></tr></thead>
        <tbody>{orders.map((o: any) => <tr key={o.id} className={o.late && !o.delivered ? "late" : ""}><td className="mono">{o.id}</td><td><small>{clockAt(n.opened, o.placed)}</small></td><td>{o.fan}</td>
          <td>{o.delivery ? <>Sec {String(o.seat).slice(1)}{o.spot && <small>{o.spot}</small>}</> : <span className="ox-muted">Pickup</span>}</td>
          <td>{n.stands.find((s: any) => s.id === o.stand)?.name ?? o.stand}</td><td><small>{o.items.map((i: any) => `${i.qty}× ${i.name}`).join(", ")}</small></td>
          <td>{stageWord(o.stage)}{o.late && !o.delivered && <small className="ox-badt">Late</small>}{!o.delivered && o.eta && <small>due {clockAt(n.opened, o.eta)}</small>}</td>
          <td><small>{o.runner ? staff[o.runner] ?? o.runner : ""}</small></td><td className="mono">{money(o.charge)}</td></tr>)}</tbody>
      </table></div>}
    </section>
    <Books b={b} cards={n.cards} />
    <PayoutsOwed p={n.payouts} staff={staff} />
  </>;
}


/** Each business Agape pays at this venue, and where its payout sign-up stands with the processor. */
function Payouts({ v, toast, reload }: { v: VenueRow; toast: (s: string) => void; reload: () => void }) {
  const list: { id: string; name: string }[] = v.form.payees?.length ? v.form.payees : [{ id: "concessions", name: `${v.form.name} concessions` }];
  const [approving, setApproving] = useState<string | null>(null);
  const [acct, setAcct] = useState("");
  const go = async (payee: string, d: string, account = "") => {
    const r = await api("venue/payee", { venue: v.id, payee, do: d, account });
    if (r.ok) { toast(d === "approve" ? "Approved for payouts." : d === "send" ? "Marked as sent." : "Reset."); setApproving(null); setAcct(""); reload(); } else toast(r.error);
  };
  return <div className="ox-payouts">
    <h3>Payouts</h3>
    {list.map(p => { const st = v.payouts?.[p.id]; const status = st?.status || "";
      return <div key={p.id} className="ox-payee">
        <div><b>{p.name || "Unnamed business"}</b><small>{status === "approved" ? `Approved · ${st!.account} · by ${st!.by}` : status === "sent" ? `Sign-up sent ${when(st!.at)} · waiting on the processor` : "Not set up yet: needed before a live night"}</small></div>
        <span className={"ox-tag " + (status === "approved" ? "live" : status === "sent" ? "requested" : "draft")}>{status === "approved" ? "Approved" : status === "sent" ? "Waiting" : "Not set up"}</span>
        <div className="ox-row wrap">
          {status !== "approved" && approving !== p.id && <>
            <button className="ox-btn sm" onClick={() => go(p.id, "send")} title="The processor's own sign-up form collects the owner's and bank details; Agape never stores them">{status === "sent" ? "Send again" : "Send payout sign-up"}</button>
            <button className="ox-btn sm ghost" onClick={() => setApproving(p.id)}>Processor approved them</button>
          </>}
          {approving === p.id && <form className="ox-row wrap" onSubmit={e => { e.preventDefault(); go(p.id, "approve", acct); }}>
            <input className="mono" autoFocus value={acct} onChange={e => setAcct(e.target.value)} placeholder="Account reference from the processor" aria-label="Account reference from the processor" />
            <button className="ox-btn sm primary" disabled={acct.trim().length < 4}>Approve</button><button type="button" className="ox-btn sm ghost" onClick={() => setApproving(null)}>Cancel</button></form>}
          {status && <button className="ox-link" onClick={() => confirm(`Start ${p.name}'s payout set-up again?`) && go(p.id, "reset")}>Start over</button>}
        </div>
      </div>; })}
    <p className="ox-fine">The processor's sign-up form collects owner and bank details; Agape never stores them. Until a processor is connected, send them its form yourself and record the account reference here once they're approved.</p>
  </div>;
}

/** After a night: what each business and runner is owed. Agape's share was kept before any of it. */
function PayoutsOwed({ p, staff }: { p: any; staff?: Record<string, string> }) {
  if (!p) return null;
  return <section className="ox-card ox-owed">
    <h3>Owed from tonight</h3>
    <div className="ox-table-wrap"><table className="ox-table">
      <thead><tr><th>Who</th><th>For</th><th className="num">Owed</th><th></th></tr></thead>
      <tbody>
        {p.payees.map((x: any) => <tr key={x.id}><td><b>{x.name}</b></td><td><small>{x.stands.map((s: any) => s.name || s.id).join(", ")}</small></td><td className="mono num">{money(x.owed)}</td>
          <td>{x.approved ? <span className="ox-tag live">Approved</span> : <span className="ox-tag bad">Not approved</span>}</td></tr>)}
        {p.runners.map((r: any) => <tr key={r.id}><td>{staff?.[r.id] ?? r.id}</td><td><small>Runner · {money(r.pay)} pay{r.tips ? ` + ${money(r.tips)} tips` : ""}</small></td><td className="mono num">{money(r.owed)}</td><td /></tr>)}
        <tr><td>Sales tax</td><td><small>Held to remit</small></td><td className="mono num">{money(p.tax)}</td><td /></tr>
        <tr className="ox-total"><td><b>Agape keeps</b></td><td><small>Its share, kept before anything is paid out</small></td><td className="mono num"><b>{money(p.agape)}</b></td><td /></tr>
      </tbody>
    </table></div>
  </section>;
}

function Books({ b, cards }: { b: any; cards?: any }) {
  const t = b.totals ?? {};
  const rows: [string, number, string][] = [["Fans paid", b.fans_paid, "Every card charge for delivered orders, tips included"], ["Food and drink sold", t.sales, "Before tax and fees"], ["Sales tax collected", b.tax, "Owed to the state"],
    ["Delivery fees", t.fees, ""], ["Tips", b.runners_tips, "Every cent to the runner who carried it"], ["Runner pay", b.runners_pay, "Paid by Agape per trip"], ["To the stands", b.stands, "After Agape's share and card processing"],
    ["Agape's share", b.agape, "After paying runners"], ["Card processing", (b.agape_processing ?? 0) + (b.interchange ?? 0), "Agape's processing, including what the card networks take"]];
  return <section className="ox-card"><h2>The books</h2>
    <p className="ox-sub">Straight from the ledger: every dollar in equals every dollar out. {b.conserved ? <b className="ox-okt">They balance to the cent.</b> : <b className="ox-badt">They don't balance. Don't close the night; tell the host.</b>}</p>
    <table className="ox-table tight books"><tbody>{rows.map(([l, v, h]) => <tr key={l}><td>{l}{h && <small>{h}</small>}</td><td className="mono num">{money(v ?? 0)}</td></tr>)}</tbody></table>
    {cards && <>
      <h3>Cards {cards.processor === "test" && <span className="ox-tag requested">Test processor: no real money moves</span>}</h3>
      <p className="ox-sub">Each order's card is held for its price when it's placed, and charged only when the order is final: what the fan owes, less any credit. Holds for orders that didn't happen are released.</p>
      <table className="ox-table tight books"><tbody>
        <tr><td>Still held<small>{cards.holds} order{cards.holds === 1 ? "" : "s"} not final yet</small></td><td className="mono num">{money(cards.held)}</td></tr>
        <tr><td>Charged</td><td className="mono num">{money(cards.captured)}</td></tr>
        <tr><td>Refunded</td><td className="mono num">{money(cards.refunded)}</td></tr>
        <tr><td>Declined at the order<small>Closed at once; never made</small></td><td className="mono num">{cards.declined}</td></tr>
        <tr><td><b>Cards, net</b><small>{cards.holds > 0 ? "Matches the books once every order is final" : cards.net === b.fans_paid ? "Equals what the books say fans paid" : "Doesn't equal the books yet; it settles every 15 seconds"}</small></td><td className={"mono num " + (cards.holds === 0 && cards.net === b.fans_paid ? "ox-okt" : "")}>{money(cards.net)}</td></tr>
      </tbody></table>
    </>}
  </section>;
}

/** A closed night, to read and print. */
function Report({ file, back }: { file: string; back: () => void }) {
  const [r, setR] = useState<any>(null);
  const [error, setError] = useState("");
  useEffect(() => { fetch(`/api/office/report/${file}`, { headers: { authorization: "Bearer " + token } }).then(x => x.json()).then(j => j.ok === false ? setError(j.error) : setR(j)).catch(() => setError("Couldn't load the report. Try again.")); }, [file]);
  if (error) return <><Head title="Report"><button className="ox-btn ghost" onClick={back}>Back to venues</button></Head><p className="ox-err">{error}</p></>;
  if (!r) return <p className="ox-empty">Loading…</p>;
  const n = r.night, t = n.books.totals ?? {};
  const staff: Record<string, string> = Object.fromEntries((r.staff ?? []).map((s: any) => [s.party, s.handle]));
  const [home, away] = n.teams ?? [];
  return <div className="ox-report">
    <Head title={`${r.name}: ${r.mode === "live" ? "night" : "rehearsal"} report`} sub={<>{when(r.opened)} to {when(r.closed)}{home && away ? ` · ${home.short || home.name} ${n.game.score[0]}, ${away.short || away.name} ${n.game.score[1]}` : ""}</>}>
      <button className="ox-btn ghost" onClick={back}>Back to venues</button><button className="ox-btn" onClick={() => print()}>Print</button>
    </Head>
    <section className="ox-card ox-kpis">
      <div><small>Orders delivered</small><b className="mono">{t.n_settled ?? 0}</b></div>
      <div><small>Not completed</small><b className="mono">{t.n_closed ?? 0}</b></div>
      <div><small>To the seat, typical</small><b className="mono">{n.wait_p50 != null ? `${Math.round(n.wait_p50 / 60)} min` : "—"}</b></div>
      <div><small>Nine in ten within</small><b className="mono">{n.wait_p90 != null ? `${Math.round(n.wait_p90 / 60)} min` : "—"}</b></div>
      <div><small>Fans paid</small><b className="mono">{money(n.books.fans_paid)}</b></div>
    </section>
    <Books b={n.books} cards={n.cards} />
    <PayoutsOwed p={n.payouts} staff={staff} />
    <section className="ox-card"><h2>By stand</h2><table className="ox-table tight"><thead><tr><th>Stand</th><th>Orders</th><th>Sales</th><th>Stand receives</th></tr></thead>
      <tbody>{n.stands.map((s: any) => <tr key={s.id}><td>{s.name}</td><td className="mono">{s.settled}</td><td className="mono">{money(s.sales)}</td><td className="mono">{money(s.revenue)}</td></tr>)}</tbody></table></section>
    <section className="ox-card"><h2>By runner</h2><table className="ox-table tight"><thead><tr><th>Runner</th><th>Delivered</th><th>Pay</th><th>Tips</th><th>Total</th></tr></thead>
      <tbody>{n.runners.filter((x: any) => x.delivered || x.pay || x.tips).map((x: any) => <tr key={x.id}><td>{staff[x.id] ?? x.id}</td><td className="mono">{x.delivered}</td><td className="mono">{money(x.pay)}</td><td className="mono">{money(x.tips)}</td><td className="mono">{money(x.pay + x.tips)}</td></tr>)}</tbody></table></section>
    <section className="ox-card"><h2>Every order</h2><div className="ox-table-wrap"><table className="ox-table tight">
      <thead><tr><th>#</th><th>Placed</th><th>Fan</th><th>Items</th><th>From</th><th>Outcome</th><th>Minutes</th><th>Runner</th><th>Total</th></tr></thead>
      <tbody>{[...n.orders].reverse().map((o: any) => <tr key={o.id}><td className="mono">{o.id}</td><td><small>{clockAt(r.opened, o.placed)}</small></td><td>{o.fan}</td><td><small>{o.items.map((i: any) => `${i.qty}× ${i.name}`).join(", ")}</small></td>
        <td>{n.stands.find((s: any) => s.id === o.stand)?.name ?? o.stand}</td><td>{stageWord(o.stage)}{o.late && <small>Late</small>}</td><td className="mono">{o.delivered ? Math.round((o.delivered - o.placed) / 60000) : "—"}</td><td><small>{o.runner ? staff[o.runner] ?? o.runner : ""}</small></td><td className="mono">{money(o.charge)}</td></tr>)}</tbody>
    </table></div></section>
    <p className="ox-fine">Ledger entries: {n.books.txns}. Report written by Agape when the night closed; it can't be edited.</p>
  </div>;
}

/** A QR code as an SVG path: crisp at any print size. */
function QR({ text, size = 150 }: { text: string; size?: number }) {
  const q = qrcode(0, "M"); q.addData(text); q.make();
  const n = q.getModuleCount(); let d = "";
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) if (q.isDark(r, c)) d += `M${c + 4} ${r + 4}h1v1h-1z`;
  return <svg viewBox={`0 0 ${n + 8} ${n + 8}`} width={size} height={size} role="img" aria-label={`QR code for ${text}`} shapeRendering="crispEdges"><rect width={n + 8} height={n + 8} fill="#fff" /><path d={d} fill="#000" /></svg>;
}

/** One sign per section, to print and put on seat backs, rails and stand counters. Scanning one
 *  opens Agape at this venue with the section already chosen. */
function SeatSigns({ v, back }: { v: VenueRow; back: () => void }) {
  const f = v.form, L = f.levels ?? 1, n = f.sections_per_level ?? 12;
  const all: string[] = []; for (let l = 1; l <= L; l++) for (let i = 1; i <= n; i++) all.push(`s${l * 100 + i}`);
  const [pick, setPick] = useState<string>("all");
  const shown = pick === "all" ? all : pick === "venue" ? [""] : all.filter(x => x.startsWith("s" + pick));
  return <div className="ox-signs-page">
    <Head title={`Seat signs for ${f.name}`} sub="Print these and put them where fans sit: seat backs, rails, cup holders. Scanning one opens Agape at this venue with the section already picked, so ordering takes a few taps. They print four to a page.">
      <button className="ox-btn ghost" onClick={back}>Back to venues</button><button className="ox-btn primary" onClick={() => print()}>Print</button>
    </Head>
    <div className="ox-seg noprint">{[["all", "Every section"], ...Array.from({ length: L }, (_, k) => [String(k + 1), `Level ${k + 1}`]), ["venue", "One for the whole venue"]].map(([k, l]) => <button key={k} className={pick === k ? "on" : ""} onClick={() => setPick(k)}>{l}</button>)}</div>
    <div className="ox-signs">{shown.map(sec => { const url = `${location.origin}/app/?v=${v.id}${sec ? `&s=${sec}` : ""}`; return <div key={sec || "venue"} className="ox-sign">
      <div className="ox-sign-mark"><Wordmark /></div>
      <p className="ox-sign-head">Order to your seat</p>
      <QR text={url} size={190} />
      <p className="ox-sign-sec">{sec ? <>Section <b>{sec.slice(1)}</b></> : f.name}</p>
      <p className="ox-sign-how">Point your phone's camera here. Food and drinks come to you; no app to download.</p>
      <p className="ox-sign-url">{url.replace(/^https?:\/\//, "")}</p>
    </div>; })}</div>
  </div>;
}

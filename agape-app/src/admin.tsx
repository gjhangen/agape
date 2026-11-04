// Agape Admin, at /admin/: the company's own accounts, every merchant's payment-processing
// details, and the VAR sheet printed from them for processors, venues and investors.
// Everything here comes from the server and needs an Agape Admin account; nothing is simulated.
import { React, ReactDOM, useEffect, useMemo, useState } from "./r";

type Level = "owner" | "editor" | "viewer";
type Me = { id: string; email: string; name: string; level: Level };
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
const LEVELS: [Level, string, string][] = [
  ["owner", "Owner", "Everything, including who has access"],
  ["editor", "Editor", "Add and change merchants, print VAR sheets"],
  ["viewer", "Viewer", "See merchants and print VAR sheets"],
];

// ---------------- talking to the server ----------------

const KEY = "agape.admin.session";
let token = (() => { try { return localStorage.getItem(KEY) ?? ""; } catch { return ""; } })();
const keep = (t: string) => { token = t; try { t ? localStorage.setItem(KEY, t) : localStorage.removeItem(KEY); } catch {} };
async function api(path: string, body?: unknown): Promise<any> {
  try {
    const r = await fetch("/api/office/" + path, { method: body === undefined ? "GET" : "POST", headers: { "content-type": "application/json", ...(token ? { authorization: "Bearer " + token } : {}) }, body: body === undefined ? undefined : JSON.stringify(body) });
    const j = await r.json().catch(() => ({ ok: false, error: "The server sent something unexpected" }));
    if (r.status === 401 && path !== "signin" && path !== "setup") { keep(""); }
    return { status: r.status, ...j };
  } catch { return { ok: false, error: "Can't reach the server. Check your connection and try again." }; }
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
      if (token) { const r = await api("me"); if (r.ok) { setMe(r.me); setBoot("in"); return; } }
      const s = await api("state");
      setBoot(s.setup ? "setup" : "signin");
    })();
  }, []);
  const enter = (t: string, m: Me) => { keep(t); setMe(m); setBoot("in"); history.replaceState(null, "", "/admin/"); };
  if (boot === "loading") return <div className="ox-gate"><Mark /></div>;
  if (boot === "setup") return <Setup enter={enter} />;
  if (boot === "invite") return <Accept code={invite!} enter={enter} toSignin={() => { history.replaceState(null, "", "/admin/"); setBoot("signin"); }} />;
  if (boot === "signin" || !me) return <SignIn enter={enter} />;
  return <Shell me={me} out={async () => { await api("signout", {}); keep(""); setMe(null); setBoot("signin"); }} />;
}

const Mark = () => <div className="ox-mark"><b>agape</b><span>admin</span></div>;

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
  return { v, bind, error, setError, busy, run };
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
  const [inv, setInv] = useState<{ email: string; name: string; level: Level } | null | "gone">(null);
  const f = useForm({ name: "", password: "", again: "" });
  useEffect(() => { api("invitation/" + encodeURIComponent(code)).then(r => { if (r.ok) { setInv(r.invite); f.bind("name").onChange({ target: { value: r.invite.name } }); } else setInv("gone"); }); }, []);
  if (inv === null) return <div className="ox-gate"><Mark /></div>;
  if (inv === "gone") return <Gate title="This invitation can't be used" sub="It has expired or was already used. Ask the person who invited you for a new one." foot={<button className="ox-link" onClick={toSignin}>Sign in instead</button>}><></></Gate>;
  return <Gate title="Join Agape Admin" sub={<>You've been invited as {LEVELS.find(l => l[0] === inv.level)?.[1].toLowerCase()} with <b>{inv.email}</b>. Choose a password to finish.</>}>
    <form onSubmit={e => f.run(e, async () => {
      if (f.v.password !== f.v.again) return "The two passwords don't match";
      const r = await api("accept", { code, name: f.v.name, password: f.v.password }); if (r.ok) enter(r.token, r.me); else return r.error;
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

type Tab = "merchants" | "team" | "leads" | "activity" | "account";

function Shell({ me, out }: { me: Me; out: () => void }) {
  const [tab, setTab] = useState<Tab>(() => (location.hash.slice(1) as Tab) || "merchants");
  const [toast, setToast] = useState("");
  useEffect(() => { history.replaceState(null, "", "#" + tab); }, [tab]);
  useEffect(() => { if (!toast) return; const t = setTimeout(() => setToast(""), 3200); return () => clearTimeout(t); }, [toast]);
  const tabs: [Tab, string][] = [["merchants", "Merchants"], ["team", "Team"], ["leads", "Leads"], ["activity", "Activity"], ["account", "Your account"]];
  return <div className="ox-app">
    <header className="ox-top">
      <Mark />
      <nav className="ox-nav" aria-label="Sections">{tabs.map(([k, l]) => <button key={k} className={tab === k ? "on" : ""} aria-current={tab === k ? "page" : undefined} onClick={() => setTab(k)}>{l}</button>)}</nav>
      <div className="ox-who"><span>{me.name}<small>{LEVELS.find(l => l[0] === me.level)?.[1]}</small></span><button className="ox-btn ghost sm" onClick={out}>Sign out</button></div>
    </header>
    <main className="ox-main">
      {tab === "merchants" && <Merchants me={me} toast={setToast} />}
      {tab === "team" && <Team me={me} toast={setToast} />}
      {tab === "leads" && <Leads />}
      {tab === "activity" && <Activity />}
      {tab === "account" && <Account me={me} toast={setToast} out={out} />}
    </main>
    {toast && <div className="ox-toast" role="status">{toast}</div>}
  </div>;
}

function Head({ title, sub, children }: { title: string; sub?: React.ReactNode; children?: React.ReactNode }) {
  return <div className="ox-head"><div><h1>{title}</h1>{sub && <p className="ox-sub">{sub}</p>}</div>{children && <div className="ox-acts">{children}</div>}</div>;
}

function StatusTag({ s }: { s: Status }) { return <span className={"ox-tag " + s}>{STATUS.find(x => x[0] === s)?.[1] ?? s}</span>; }

function Merchants({ me, toast }: { me: Me; toast: (s: string) => void }) {
  const [list, setList] = useState<Merchant[] | null>(null);
  const [venues, setVenues] = useState<{ id: string; name: string }[]>([]);
  const [open, setOpen] = useState<{ m: Merchant | null; sheet: boolean } | null>(null);
  const [error, setError] = useState("");
  const load = async () => { const r = await api("merchants"); if (r.ok) { setList(r.merchants); setVenues(r.venues); } else setError(r.error); };
  useEffect(() => { load(); }, []);
  const can = me.level !== "viewer";
  if (open?.sheet && open.m) return <Sheet m={open.m} back={() => setOpen({ m: open.m, sheet: false })} close={() => setOpen(null)} />;
  if (open) return <Editor m={open.m} can={can} owner={me.level === "owner"} venues={venues} toast={toast}
    saved={m => { load(); setOpen({ m, sheet: false }); }} sheet={m => setOpen({ m, sheet: true })} close={() => { setOpen(null); load(); }} />;
  return <>
    <Head title="Merchants" sub="Each business Agape takes payments for: its processor, the numbers the processor issued, and the VAR sheet to send.">
      {can && <button className="ox-btn primary" onClick={() => setOpen({ m: null, sheet: false })}>Add a merchant</button>}
    </Head>
    {error && <p className="ox-err">{error}</p>}
    {list === null ? <p className="ox-empty">Loading…</p> : !list.length ? <div className="ox-blank">
      <h2>No merchants yet</h2>
      <p>Add the first one with what you know today: the business, its processor, and how cards are taken. The numbers the processor issues can come later, and the VAR sheet shows what's still pending.</p>
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
        <td><button className="ox-btn sm" onClick={() => setOpen({ m, sheet: true })}>VAR sheet</button></td>
      </tr>; })}</tbody>
    </table></div>}
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
  const ref = `VAR-${m.id.slice(1).toUpperCase()}-${new Date(m.at * 1000).toISOString().slice(0, 10).replace(/-/g, "")}`;
  const print = () => { api("printed", { id: m.id }); const t = document.title; document.title = `${ref} ${f.dba} VAR sheet`; window.print(); document.title = t; };
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
        <button className="ox-btn ghost" onClick={back}>Edit details</button>
        <button className="ox-btn ghost" onClick={close}>All merchants</button>
        <button className="ox-btn" onClick={text}>{copied ? "Copied" : "Copy as text"}</button>
        <button className="ox-btn primary" onClick={print}>Print or save PDF</button>
      </div>
    </div>
    <div className="ox-copy noprint" role="group" aria-label="Who this copy is for">
      <span>This copy is for</span>
      {([["processor", "The processor", "Every number, in full"], ["venue", "The venue", "Every number, in full"], ["investor", "Investors", "Account numbers masked to the last four"]] as [Copy, string, string][]).map(([k, l, d]) =>
        <button key={k} className={copy === k ? "on" : ""} aria-pressed={copy === k} onClick={() => setCopy(k)}><b>{l}</b><small>{d}</small></button>)}
    </div>
    <article className="var-sheet">
      <header className="var-top">
        <div className="var-brand"><b>agape</b><span>to your seat</span></div>
        <div className="var-title"><h1>VAR Sheet</h1><span>Merchant processing setup</span></div>
        <dl className="var-ref"><div><dt>Reference</dt><dd className="mono">{ref}</dd></div><div><dt>Prepared</dt><dd>{day()}</dd></div><div><dt>Status</dt><dd>{STATUS.find(s => s[0] === m.status)?.[1]}</dd></div></dl>
      </header>
      <div className="var-who"><h2>{f.dba}</h2><p>{[f.legal_name && f.legal_name !== f.dba ? f.legal_name : "", [f.address, f.city, [f.state, f.zip].filter(Boolean).join(" ")].filter(Boolean).join(", ")].filter(Boolean).join(" · ")}</p></div>
      {pending.length > 0 && <p className="var-pending"><b>Pending from the processor or merchant:</b> {pending.map(k => LABEL[k]).join(", ")}.</p>}
      {SECTIONS.filter(s => s.id !== "notes").map(s => <section key={s.id} className="var-sec">
        <h3>{s.title}</h3>
        <dl className="var-grid">{s.fields.filter(fd => s.id !== "software" || val(fd.k)).map(fd => { const v = val(fd.k); return <div key={fd.k} className={(fd.wide ? "wide " : "") + (v ? "" : "blank")}>
          <dt>{fd.l}</dt><dd className={fd.num ? "mono" : ""}>{v || <span className="var-line" aria-label="Blank" />}</dd></div>; })}</dl>
      </section>)}
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

function Team({ me, toast }: { me: Me; toast: (s: string) => void }) {
  const [t, setT] = useState<{ users: (Me & { at: number; off: boolean })[]; invites: { email: string; name: string; level: Level; until: number }[] } | null>(null);
  const [link, setLink] = useState("");
  const f = useForm({ email: "", name: "", level: "editor" });
  const load = async () => { const r = await api("team"); if (r.ok) setT(r); };
  useEffect(() => { load(); }, []);
  const owner = me.level === "owner";
  const set = async (id: string, b: Record<string, unknown>, ok: string) => { const r = await api("user", { id, ...b }); toast(r.ok ? ok : r.error); load(); };
  return <>
    <Head title="Team" sub="Everyone with an Agape Admin account. Owners invite people and decide what they can do." />
    {owner && <section className="ox-card">
      <h2>Invite someone</h2>
      <p className="ox-sub">They get a link to choose their own password. It works once, for 7 days. Send it to them yourself, by email or text.</p>
      <form className="ox-invite" onSubmit={e => f.run(e, async () => {
        const r = await api("invite", f.v);
        if (!r.ok) return r.error;
        setLink(`${location.origin}/admin/?invite=${r.code}`); f.bind("email").onChange({ target: { value: "" } }); load();
      })}>
        <label className="ox-f"><span>Name</span><input required {...f.bind("name")} /></label>
        <label className="ox-f"><span>Email</span><input type="email" required {...f.bind("email")} /></label>
        <label className="ox-f"><span>They can</span><select {...f.bind("level")}>{LEVELS.map(([k, l, d]) => <option key={k} value={k}>{l}: {d}</option>)}</select></label>
        <button className="ox-btn primary" disabled={f.busy}>Create invitation</button>
      </form>
      {f.error && <p className="ox-err">{f.error}</p>}
      {link && <div className="ox-linkbox"><span>Send them this link. It won't be shown again.</span><code>{link}</code><button className="ox-btn sm" onClick={() => navigator.clipboard?.writeText(link).then(() => toast("Link copied."))}>Copy link</button></div>}
    </section>}
    <section className="ox-card">
      <h2>People</h2>
      {!t ? <p className="ox-empty">Loading…</p> : <div className="ox-table-wrap"><table className="ox-table">
        <thead><tr><th>Name</th><th>Email</th><th>Access</th><th>Joined</th>{owner && <th></th>}</tr></thead>
        <tbody>{t.users.map(u => <tr key={u.id} className={u.off ? "off" : ""}>
          <td>{u.name}{u.id === me.id && <small>You</small>}</td><td>{u.email}</td>
          <td>{owner && u.id !== me.id && !u.off ? <select aria-label={`Access for ${u.name}`} value={u.level} onChange={e => set(u.id, { level: e.target.value }, "Access changed.")}>{LEVELS.map(([k, l]) => <option key={k} value={k}>{l}</option>)}</select> : u.off ? "Turned off" : LEVELS.find(l => l[0] === u.level)?.[1]}</td>
          <td><small>{when(u.at)}</small></td>
          {owner && <td>{u.id !== me.id && <button className="ox-btn ghost sm" onClick={() => set(u.id, { off: !u.off }, u.off ? "Access turned back on." : "Access turned off. They're signed out everywhere.")}>{u.off ? "Turn access on" : "Turn access off"}</button>}</td>}
        </tr>)}
        {t.invites.map(i => <tr key={i.email} className="pending">
          <td>{i.name || "—"}<small>Invited</small></td><td>{i.email}</td><td>{LEVELS.find(l => l[0] === i.level)?.[1]}</td><td><small>Link works until {when(i.until)}</small></td>
          {owner && <td><button className="ox-btn ghost sm" onClick={async () => { const r = await api("invite/cancel", { email: i.email }); toast(r.ok ? "Invitation cancelled." : r.error); load(); }}>Cancel</button></td>}
        </tr>)}</tbody>
      </table></div>}
    </section>
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

function Activity() {
  const [log, setLog] = useState<{ at: number; who: string; what: string }[] | null>(null);
  useEffect(() => { api("activity").then(r => setLog(r.ok ? r.log : [])); }, []);
  return <>
    <Head title="Activity" sub="Every change and every printed VAR sheet, newest first. It can't be edited." />
    {log === null ? <p className="ox-empty">Loading…</p> : <ol className="ox-log">{log.map((e, i) => <li key={i}><time>{when(e.at)}</time><b>{e.who}</b><span>{e.what}</span></li>)}</ol>}
  </>;
}

function Account({ me, toast, out }: { me: Me; toast: (s: string) => void; out: () => void }) {
  const f = useForm({ old: "", new: "", again: "" });
  return <>
    <Head title="Your account" sub={<>{me.name} · {me.email} · {LEVELS.find(l => l[0] === me.level)?.[1]}</>} />
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

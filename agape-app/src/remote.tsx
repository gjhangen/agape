// The app as people use it: signed in, talking to the venue's server. One account, one role,
// one screen. The simulation harness (main.tsx) is not part of this build.
import { React, ReactDOM, useEffect, useState } from "./r";
import { kernel, server } from "./kernel";
import { FanApp, FanView, Sheet } from "./fan";
import { OpsLens, RunnerLens, StandLens } from "./roles";
import { BusinessLens, PlayerLens } from "./deals";
import { BRAND, venueTime } from "./parts";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";
import { LookPicker } from "./lookpicker";

type Account = { id: string; handle: string; role: "fan" | "runner" | "stand" | "business" | "player" | "ops"; party: string; venue: string | null };
const store = {
  get: (k: string) => { try { return localStorage.getItem(k); } catch { return null; } },
  set: (k: string, v: string | null) => { try { v == null ? localStorage.removeItem(k) : localStorage.setItem(k, v); } catch { /* private mode: sign in each visit */ } },
};
const VENUE = "harbor";
const SECTIONS = [...Array.from({ length: 24 }, (_, i) => `s${101 + i}`), ...Array.from({ length: 24 }, (_, i) => `s${201 + i}`)];

type Glance = { venue: string; clock: { label: string; home: number; away: number; phase: string }; teams: { name: string; short: string; abbr: string; color: string }[];
  moments: { t: number; text: string; name: string | null }[]; orders: number; on_the_way: number; avg_delivery_min: number | null; provenance: { simulated_fans: number } };

/** The front door: the game happening right now, what Agape is, and a way in. */
function Gate({ done }: { done: (token: string, a: Account) => void }) {
  const [mode, setMode] = useState<"in" | "up">("up");
  const [handle, setHandle] = useState("");
  const [pw, setPw] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [g, setG] = useState<Glance | null>(null);
  useEffect(() => {
    const load = () => fetch(`/api/v/${VENUE}/glance`).then(r => r.json()).then(setG).catch(() => {});
    load(); const id = setInterval(load, 5000); return () => clearInterval(id);
  }, []);
  const go = async (e: React.FormEvent) => {
    e.preventDefault(); setBusy(true); setError("");
    const r = await server.post(mode === "up" ? "/api/signup" : "/api/signin", { handle, password: pw });
    setBusy(false);
    if (r.ok) done(r.token, r.account); else setError(r.error);
  };
  const [home, away] = g?.teams ?? [];
  const latest = g?.moments.find(m => m.text !== "Gates open");
  return (
    <div className="landing">
      <header className="l-top"><span className="l-mark">agape</span><span className="l-tag">to your seat</span></header>
      <section className="l-live" aria-live="polite">
        <p className="l-on"><i className="live-dot" />Live at {g?.venue ?? BRAND.venue}</p>
        {g && home && away ? <>
          <div className="l-score">
            <span className="l-team"><i style={{ background: home.color }} />{home.short}</span>
            <b className="num">{g.clock.home}</b>
            <span className="l-clock num">{g.clock.label}</span>
            <b className="num">{g.clock.away}</b>
            <span className="l-team right">{away.short}<i style={{ background: away.color }} /></span>
          </div>
          {latest && <p className="l-moment">{latest.text}{latest.name ? ` · ${latest.name}` : ""}</p>}
          <p className="l-stats"><span><b className="num">{g.orders.toLocaleString()}</b> orders tonight</span><span><b className="num">{g.on_the_way}</b> on the way now</span>
            {g.avg_delivery_min != null && <span><b className="num">{g.avg_delivery_min} min</b> to the seat, on average</span>}</p>
        </> : <p className="l-moment">Opening the doors…</p>}
      </section>
      <h1 className="l-head">The game, and everything around it, from your seat.</h1>
      <ul className="l-points">
        <li><b>Order to your seat.</b> Food, drinks and gear, with the honest time it'll reach you before you tap.</li>
        <li><b>Play along.</b> Call each quarter for points. Collect the night's big plays as cards before they're gone.</li>
        <li><b>Points that travel.</b> Your level and points come with you to every venue on Agape.</li>
      </ul>
      <form className="l-form" onSubmit={go}>
        <h2>{mode === "up" ? "Get in. It's free." : "Welcome back."}</h2>
        <label>Pick a name<input value={handle} onChange={e => setHandle(e.target.value)} autoComplete="username" autoCapitalize="none" autoCorrect="off" spellCheck={false} placeholder="what other fans see" required /></label>
        <label>Password<input type="password" value={pw} onChange={e => setPw(e.target.value)} autoComplete={mode === "up" ? "new-password" : "current-password"} required minLength={8} placeholder="8 or more characters" /></label>
        {error && <p className="banner warn" role="alert">{error}</p>}
        <button className="btn primary big wide" disabled={busy}>{busy ? "One moment…" : mode === "up" ? "Take me to my seat" : "Sign in"}</button>
        <button type="button" className="link-inline" onClick={() => { setMode(mode === "up" ? "in" : "up"); setError(""); }}>{mode === "up" ? "I already have an account" : "I'm new here"}</button>
      </form>
      <p className="l-note">This is the Agape pilot. The game and the {g ? g.provenance.simulated_fans.toLocaleString() : "3,000"} other fans are simulated, and nothing is charged. Your account, orders, picks and points are real.</p>
    </div>
  );
}

const BOWLS: [string, string, string[]][] = [
  ["Lower bowl", "Closest to the floor", Array.from({ length: 24 }, (_, i) => `s${101 + i}`)],
  ["Upper bowl", "The whole court in view", Array.from({ length: 24 }, (_, i) => `s${201 + i}`)],
];

function Join({ joined }: { joined: () => void }) {
  const [seat, setSeat] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const go = () => { setBusy(true); const r = kernel.act({ do: "join", seat }); setBusy(false); if (r.ok) joined(); else setError(r.error); };
  return (
    <div className="landing join">
      <header className="l-top"><span className="l-mark">agape</span><span className="l-tag">to your seat</span></header>
      <h1 className="l-head">Where are you sitting?</h1>
      <p className="l-sub">Your section is where we bring things, and who you're up against in tonight's section battle.</p>
      {BOWLS.map(([name, blurb, secs]) => (
        <section key={name} className="bowl">
          <div className="block-head"><h2>{name}</h2><span className="sub">{blurb}</span></div>
          <div className="sec-grid">{secs.map(x => <button key={x} className={"sec-chip" + (seat === x ? " on" : "")} aria-pressed={seat === x} onClick={() => setSeat(x)}>{x.slice(1)}</button>)}</div>
        </section>
      ))}
      {error && <p className="banner warn" role="alert">{error}</p>}
      <div className="join-go"><button className="btn primary big wide" disabled={!seat || busy} onClick={go}>{seat ? `I'm in section ${seat.slice(1)}` : "Tap your section"}</button></div>
    </div>
  );
}

function Signed({ a, signOut }: { a: Account; signOut: () => void }) {
  const [, bump] = useState(0);
  const [sheet, setSheet] = useState<Sheet>(null);
  const [toasts, setToasts] = useState<{ id: number; text: string }[]>([]);
  useEffect(() => { const es = server.connect(VENUE, store.get("agape.token") ?? ""); const off = server.subscribe(() => bump(n => n + 1)); return () => { off(); es.close(); }; }, []);
  const refresh = () => bump(n => n + 1);
  const toast = (text: string) => { const id = Date.now() + Math.random(); setToasts(t => [...t, { id, text }]); setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 4200); };
  const lens = a.role === "fan" ? "fan" : a.role;
  const v = kernel.view(lens as any, "");
  if (v?.error || v?.ok === false) return <div className="gate"><p>{v.error}</p><button className="btn" onClick={signOut}>Sign out</button></div>;
  if (a.role === "fan" && !(v as FanView).you?.seat) return <Join joined={refresh} />;
  const noop = () => {};
  if (a.role === "fan") {
    const settings = (
      <section className="block settings">
        <h2>Settings</h2>
        <p className="sub">Signed in as <b>{a.handle}</b></p>
        <LookPicker />
        <button className="btn wide" onClick={signOut}>Sign out</button>
      </section>
    );
    return (
      <div className="app-shell">
        <FanApp v={v} refresh={refresh} sheet={sheet} setSheet={setSheet} toast={toast} settings={settings} />
        <div className="toasts" aria-live="polite">{toasts.map(t => <p key={t.id} className="toast">{t.text}</p>)}</div>
      </div>
    );
  }
  return (
    <>
      <header className="harness signed">
        <div className="harness-in">
          <span className="eyebrow">{BRAND.venue} · {a.handle}</span>
          <LookPicker />
          <button className="btn h-btn" onClick={signOut}>Sign out</button>
        </div>
      </header>
      <main className={"stage " + a.role}>
        {v.clock && <p className="staff-clock"><b className="num">{venueTime(v.clock.t)}</b></p>}
        {a.role === "runner" && <RunnerLens r={v} map={v.map} pick={noop} runners={[a.party]} x={{ toast, refresh }} />}
        {a.role === "stand" && <StandLens s={v} pick={noop} stands={[{ id: v.stand, name: v.name } as any]} x={{ toast, refresh }} />}
        {a.role === "business" && <BusinessLens b={v} businesses={[{ id: v.id, name: v.name }]} pick={noop} toast={toast} refresh={refresh} />}
        {a.role === "player" && <PlayerLens p={v} players={[{ id: v.id, name: v.name, team: v.group?.name ?? "", claimed: v.claimed }]} pick={noop} toast={toast} refresh={refresh} />}
        {a.role === "ops" && <OpsLens o={v} x={{ toast, refresh }} />}
      </main>
      <div className="toasts" aria-live="polite">{toasts.map(t => <p key={t.id} className="toast">{t.text}</p>)}</div>
    </>
  );
}

function App() {
  const [a, setA] = useState<Account | null>(null);
  const [checked, setChecked] = useState(false);
  useEffect(() => {
    const token = store.get("agape.token");
    if (!token) { setChecked(true); return; }
    fetch("/api/me", { headers: { authorization: `Bearer ${token}` } }).then(r => r.json()).then(r => { if (r.ok) { server.connect(VENUE, token).close(); setA(r.account); } setChecked(true); }).catch(() => setChecked(true));
  }, []);
  if (!checked) return <div className="gate"><h1>{BRAND.venue}</h1><p>Opening the doors…</p></div>;
  if (!a) return <Gate done={(token, acc) => { store.set("agape.token", token); server.connect(VENUE, token).close(); setA(acc); }} />;
  return <Signed a={a} signOut={() => { store.set("agape.token", null); location.reload(); }} />;
}

applyLook(savedLook()); applyMode(savedMode());
ReactDOM.createRoot(document.getElementById("root")!).render(<App />);

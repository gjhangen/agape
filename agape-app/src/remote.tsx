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

function Gate({ done }: { done: (token: string, a: Account) => void }) {
  const [mode, setMode] = useState<"in" | "up">("up");
  const [handle, setHandle] = useState("");
  const [pw, setPw] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const go = async (e: React.FormEvent) => {
    e.preventDefault(); setBusy(true); setError("");
    const r = await server.post(mode === "up" ? "/api/signup" : "/api/signin", { handle, password: pw });
    setBusy(false);
    if (r.ok) done(r.token, r.account); else setError(r.error);
  };
  return (
    <div className="gate signin">
      <h1>{BRAND.venue}</h1>
      <p>{mode === "up" ? "Food, drinks and gear to your seat, the game live, and everything around it." : "Welcome back."}</p>
      <form className="signin-form" onSubmit={go}>
        <label>Name<input value={handle} onChange={e => setHandle(e.target.value)} autoComplete="username" autoCapitalize="none" required /></label>
        <label>Password<input type="password" value={pw} onChange={e => setPw(e.target.value)} autoComplete={mode === "up" ? "new-password" : "current-password"} required minLength={8} /></label>
        {error && <p className="banner warn" role="alert">{error}</p>}
        <button className="btn primary big wide" disabled={busy}>{mode === "up" ? "Make my account" : "Sign in"}</button>
      </form>
      <button className="link-inline" onClick={() => { setMode(mode === "up" ? "in" : "up"); setError(""); }}>{mode === "up" ? "I already have an account" : "I'm new here"}</button>
    </div>
  );
}

function Join({ joined }: { joined: () => void }) {
  const [seat, setSeat] = useState("s104");
  const [error, setError] = useState("");
  return (
    <div className="gate signin">
      <h1>Where are you sitting?</h1>
      <p>Your section is where we bring things and who you're up against in the section battle.</p>
      <form className="signin-form" onSubmit={e => { e.preventDefault(); const r = kernel.act({ do: "join", seat }); if (r.ok) joined(); else setError(r.error); }}>
        <label>Section<select value={seat} onChange={e => setSeat(e.target.value)}>{SECTIONS.map(s => <option key={s} value={s}>Section {s.slice(1)}</option>)}</select></label>
        {error && <p className="banner warn" role="alert">{error}</p>}
        <button className="btn primary big wide">That's my seat</button>
      </form>
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
  return (
    <>
      <header className="harness signed">
        <div className="harness-in">
          <span className="eyebrow">{BRAND.venue} · {a.handle}</span>
          <LookPicker />
          <button className="btn h-btn" onClick={signOut}>Sign out</button>
        </div>
      </header>
      <main className={"stage " + (a.role === "fan" ? "fan" : a.role)}>
        {a.role !== "fan" && v.clock && <p className="staff-clock"><b className="num">{venueTime(v.clock.t)}</b></p>}
        {a.role === "fan" && <div className="fan-layout"><FanApp v={v} refresh={refresh} sheet={sheet} setSheet={setSheet} toast={toast} /></div>}
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

// The app as people use it: signed in, talking to the venue's server. One account, one role,
// one screen. The simulation harness (main.tsx) is not part of this build.
import { React, ReactDOM, useEffect, useState } from "./r";
import { kernel, server } from "./kernel";
import { FanApp, FanView, Sheet } from "./fan";
import { OpsLens, RunnerLens, StandLens } from "./roles";
import { BusinessLens, PlayerLens } from "./deals";
import { BRAND, momentText, venueTime } from "./parts";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";
import { LookPicker } from "./lookpicker";
import { TalkToUs } from "./talk";

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
  const [info, setInfo] = useState(false);
  const [talk, setTalk] = useState(false);
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
      <header className="l-top"><span className="l-mark">agape</span><span className="l-tag">to your seat</span><a className="l-sim-link" href="/demo/">Simulator</a></header>
      <section className="l-live" aria-live="polite">
        <p className="l-on"><i className="live-dot" />Now playing at {g?.venue ?? BRAND.venue}<span className="l-sim-tag">simulated game</span></p>
        {g && home && away ? <>
          <div className="l-score">
            <span className="l-team"><i style={{ background: home.color }} />{home.short}</span>
            <b className="num">{g.clock.home}</b>
            <span className="l-clock num">{g.clock.label}</span>
            <b className="num">{g.clock.away}</b>
            <span className="l-team right">{away.short}<i style={{ background: away.color }} /></span>
          </div>
          {latest && <p className="l-moment">{(latest as any).kind === "moment" ? momentText(latest.text, (latest as any).team ?? "") : (latest as any).kind === "score" ? `${latest.text} points` : latest.text}{latest.name ? ` · ${latest.name}` : ""}</p>}
          <p className="l-stats"><span><b className="num">{g.orders.toLocaleString()}</b> orders tonight</span><span><b className="num">{g.on_the_way}</b> on the way now</span>
            {g.avg_delivery_min != null && <span><b className="num">{g.avg_delivery_min} min</b> to the seat, on average</span>}</p>
        </> : <p className="l-moment">Opening the doors…</p>}
      </section>
      <h1 className="l-head">The game, and everything around it, from your seat.</h1>
      <form className="l-form" onSubmit={go}>
        <h2>{mode === "up" ? "Get in. It's free." : "Welcome back."}</h2>
        <label>{mode === "up" ? "Pick a name" : "Your name"}<input value={handle} onChange={e => setHandle(e.target.value)} autoComplete="username" autoCapitalize="none" autoCorrect="off" spellCheck={false} placeholder={mode === "up" ? "shown to other fans" : ""} required /></label>
        <label>Password<input type="password" value={pw} onChange={e => setPw(e.target.value)} autoComplete={mode === "up" ? "new-password" : "current-password"} required minLength={8} placeholder="8 or more characters" /></label>
        {error && <p className="banner warn" role="alert">{error}{mode === "in" && " In the pilot there's no password reset, so if it's lost, just make a new account."}</p>}
        <button className="btn primary big wide" disabled={busy}>{busy ? "One moment…" : mode === "up" ? "Take me to my seat" : "Sign in"}</button>
        <button type="button" className="link-inline" onClick={() => { setMode(mode === "up" ? "in" : "up"); setError(""); }}>{mode === "up" ? "I already have an account" : "I'm new here"}</button>
      </form>
      <ul className="l-points">
        <li><b>Order to your seat.</b> Food, drinks and gear, with the honest time it'll reach you before you tap.</li>
        <li><b>Play along.</b> Call each quarter for points. Collect the night's big plays as cards before they're gone.</li>
        <li><b>Points that travel.</b> Your level and points come with you to every venue on Agape.</li>
      </ul>
      <section className="l-sim">
        <div>
          <span className="eyebrow">For venues, teams, businesses and investors</span>
          <h2>See the whole night, from every seat.</h2>
          <p>The simulator runs a full arena night in your browser: thousands of fans, the runners, every kitchen and every dollar. Watch it as anyone, speed it up, and put your own team, logo, menu and rules on it in the Studio.</p>
        </div>
        <ul className="l-roles">
          <li><b>Venues</b><span>Orders to every seat, live operations, a ledger that balances to the cent</span></li>
          <li><b>Teams and players</b><span>Fans who play along all game, and paid promotions they control</span></li>
          <li><b>Businesses</b><span>Offers fans unlock with points, and proof of who walked in</span></li>
        </ul>
        <div className="l-sim-acts"><a className="btn primary big" href="/demo/">Open the simulator</a><button className="btn big ghost-board" onClick={() => setTalk(true)}>Talk to the team</button></div>
      </section>
      {talk && <TalkToUs close={() => setTalk(false)} />}
      <p className="l-note">This is the Agape pilot. The game and the {g ? g.provenance.simulated_fans.toLocaleString() : "3,000"} other fans are simulated, and nothing is charged. Your account, orders, picks and points are real. <button className="link-inline small" onClick={() => setInfo(true)}>How points, privacy and the pilot work</button></p>
      {info && <PilotInfo close={() => setInfo(false)} />}
    </div>
  );
}

const BOWLS: [string, string, string[]][] = [
  ["Lower bowl", "Closest to the floor", Array.from({ length: 24 }, (_, i) => `s${101 + i}`)],
  ["Upper bowl", "The whole court in view", Array.from({ length: 24 }, (_, i) => `s${201 + i}`)],
];

/** Change row and seat after joining. */
function SpotEditor({ refresh }: { refresh: () => void }) {
  const [open, setOpen] = useState(false);
  const [row, setRow] = useState("");
  const [place, setPlace] = useState("");
  if (!open) return <button className="link-inline small" onClick={() => setOpen(true)}>Change my row and seat</button>;
  return (
    <form className="spot-row" onSubmit={e => { e.preventDefault(); kernel.act({ do: "spot", row, place }); setOpen(false); refresh(); }}>
      <label>Row<input value={row} onChange={e => setRow(e.target.value)} maxLength={4} autoCapitalize="characters" /></label>
      <label>Seat<input value={place} onChange={e => setPlace(e.target.value)} maxLength={4} inputMode="numeric" /></label>
      <button className="btn primary">Save</button>
    </form>
  );
}

/** The plain-words page: what's simulated, how points work, what we keep. */
function PilotInfo({ close }: { close: () => void }) {
  useEffect(() => { const k = (e: KeyboardEvent) => e.key === "Escape" && close(); addEventListener("keydown", k); return () => removeEventListener("keydown", k); }, []);
  return (
    <div className="info-back" onClick={e => e.target === e.currentTarget && close()}>
      <article className="info" role="dialog" aria-modal="true" aria-label="How the pilot works">
        <div className="sheet-head"><h2>How the pilot works</h2><button className="btn ghost sm" onClick={close}>Close</button></div>
        <h3>What's real and what's simulated</h3>
        <p>Agape itself is real: your account, your orders, the arrival times, the points you earn and the cards you collect. The basketball game and the other fans in the building are simulated, so you can try everything before Agape opens at a venue near you. Nothing is charged; prices show what you'd pay.</p>
        <h3>Points</h3>
        <p>You earn points by ordering, calling the quarter and collecting. You can spend them on moment cards and offers around the venue, or pass cards to other fans for points. Points can't be bought with money, can't be cashed out, and don't expire. Your level is the total you've ever earned and comes with you to every venue on Agape.</p>
        <h3>Drinks</h3>
        <p>Drinks with alcohol are for people 21 and older. We don't ask your age when you sign up; the first time you order one, we ask you to confirm you're 21, and your runner checks ID at your seat. The venue's own last call applies.</p>
        <h3>What's in the launch, and what's optional</h3>
        <p>At launch: ordering to your seat or for pickup, live arrival times, points, sending something to a friend's seat, going first with points, your section's battle and wearing your team's colors. Tagged <span className="opt">Optional</span>: built and working here, but switched on only by a venue that wants it. That's crowd chat, calling the quarter, moment cards and trading them, tickets and resale, seat upgrades, rides and partner offers, and player deals.</p>
        <h3>Moment cards</h3>
        <p>Cards are for fun. They're free to collect and trade only for points. Nobody can promise what a card will be worth.</p>
        <h3>The crowd chat</h3>
        <p>Links, phone numbers and selling aren't allowed, and swearing is hidden. Your chosen name is shown next to what you say.</p>
        <h3>Your information</h3>
        <p>We keep your name, a protected password, your section and what you do in the app, to run it. We don't sell it, and there's no tracking or advertising. Ask us and we'll delete your account.</p>
      </article>
    </div>
  );
}

function Join({ joined, a }: { joined: () => void; a: Account }) {
  useEffect(() => { scrollTo(0, 0); }, []);
  const key = `agape.joined.${a.id}`;
  const back = store.get(key) === "1";
  const [seat, setSeat] = useState("");
  const [row, setRow] = useState("");
  const [place, setPlace] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const go = () => { setBusy(true); const r = kernel.act({ do: "join", seat, row, place }); setBusy(false); if (r.ok) { store.set(key, "1"); joined(); } else setError(r.error); };
  return (
    <div className="landing join">
      <header className="l-top"><span className="l-mark">agape</span><span className="l-tag">to your seat</span></header>
      <h1 className="l-head">{back ? `A new night, ${a.handle}.` : "Where are you sitting?"}</h1>
      <p className="l-sub">{back ? "Your level and points came with you. Where are you sitting tonight?" : "Your section is where we bring things, and who you're up against in tonight's section battle."}</p>
      {BOWLS.map(([name, blurb, secs]) => (
        <section key={name} className="bowl">
          <div className="block-head"><h2>{name}</h2><span className="sub">{blurb}</span></div>
          <div className="sec-grid">{secs.map(x => <button key={x} className={"sec-chip" + (seat === x ? " on" : "")} aria-pressed={seat === x} onClick={() => setSeat(x)}>{x.slice(1)}</button>)}</div>
        </section>
      ))}
      {seat && <section className="bowl spot-in">
        <div className="block-head"><h2>Row and seat</h2><span className="sub">So your runner walks straight to you</span></div>
        <div className="spot-row">
          <label>Row<input value={row} onChange={e => setRow(e.target.value)} maxLength={4} autoCapitalize="characters" placeholder="F" /></label>
          <label>Seat<input value={place} onChange={e => setPlace(e.target.value)} maxLength={4} inputMode="numeric" placeholder="7" /></label>
        </div>
      </section>}
      {error && <p className="banner warn" role="alert">{error}</p>}
      <div className="join-go"><button className="btn primary big wide" disabled={!seat || busy} onClick={go}>{seat ? `I'm in section ${seat.slice(1)}${row ? `, row ${row.toUpperCase()}` : ""}${place ? `, seat ${place}` : ""}` : "Tap your section"}</button></div>
    </div>
  );
}

function Signed({ a, signOut }: { a: Account; signOut: () => void }) {
  const [, bump] = useState(0);
  const [sheet, setSheet] = useState<Sheet>(null);
  const [toasts, setToasts] = useState<{ id: number; text: string }[]>([]);
  const [info, setInfo] = useState(false);
  useEffect(() => { const es = server.connect(VENUE, store.get("agape.token") ?? ""); const off = server.subscribe(() => bump(n => n + 1)); return () => { off(); es.close(); }; }, []);
  const refresh = () => bump(n => n + 1);
  const toast = (text: string) => { const id = Date.now() + Math.random(); setToasts(t => t.some(x => x.text === text) ? t : [{ id, text }]); setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 4200); };
  const lens = a.role === "fan" ? "fan" : a.role;
  const v = kernel.view(lens as any, "");
  if (v?.error || v?.ok === false) return <div className="gate"><p>{v.error}</p><button className="btn" onClick={signOut}>Sign out</button></div>;
  if (a.role === "fan" && !(v as FanView).you?.seat) return <Join joined={refresh} a={a} />;
  const noop = () => {};
  if (a.role === "fan") {
    const settings = (
      <section className="block settings">
        <h2>Settings</h2>
        <p className="sub">Signed in as <b>{a.handle}</b> · section {(v.you?.seat ?? "").slice(1)}{v.you?.spot ? `, ${v.you.spot}` : ""}</p>
        <SpotEditor refresh={refresh} />
        <LookPicker />
        <button className="link-inline small" onClick={() => setInfo(true)}>How points, privacy and the pilot work</button>
        <button className="btn wide" onClick={signOut}>Sign out</button>
        {info && <PilotInfo close={() => setInfo(false)} />}
      </section>
    );
    return (
      <div className="app-shell">
        <FanApp v={v} refresh={refresh} sheet={sheet} setSheet={setSheet} toast={toast} settings={settings} />
        <aside className="desk-side" aria-label="About this page">
          <span className="l-mark">agape</span>
          <p>Agape is made for the phone in your hand at the game. Open <b>agapetoyourseat.com</b> on your phone to use it there, signed in as {a.handle}.</p>
          <p>Here for a venue, a team or a business? See the whole night from every seat, and put your own team on it.</p>
          <a className="btn primary big" href="/demo/">Open the simulator</a>
        </aside>
        <div className="toasts" aria-live="polite">{toasts.map(t => <p key={t.id} className="toast">{t.text}</p>)}</div>
      </div>
    );
  }
  return (
    <>
      <header className="harness signed">
        <div className="harness-in">
          <span className="eyebrow">{BRAND.venue} · {a.handle}</span>
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
  return <Signed a={a} signOut={() => { const t = store.get("agape.token"); fetch("/api/signout", { method: "POST", headers: { authorization: `Bearer ${t}` } }).finally(() => { store.set("agape.token", null); location.reload(); }); }} />;
}

applyLook(savedLook()); applyMode(savedMode());
ReactDOM.createRoot(document.getElementById("root")!).render(<App />);

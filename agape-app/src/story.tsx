// The fan demo: Agape on a phone, opened the way a fan would open it, with the night's story told beside it.
// Everything about the demo (the story, levels, speed, sound) lives here, outside the phone. The app inside
// never mentions a demo.
import { React, ReactDOM, useEffect, useRef, useState } from "./r";
import { Wordmark } from "./logo";
import { boot, kernel, setWorldOverride } from "./kernel";
import { FanApp, FanView, Sheet, profileName } from "./fan";
import { BRAND, feature, Icon, LEVELS, levelFeatures, momentText, runnerName, venueTime } from "./parts";
import { applyLook, applyMode, savedLook, savedMode, setVenueColor } from "./themes";
import { TalkToUs } from "./talk";
import { ExplainCard, ExplainLayer, FAN_NOTES, Note } from "./explain";
import { DemoModal, OnYourPhone, useTopHeight, EVENT, EventSwitch, HeldCard, demoWorld, useFull, useHoldOnBeat, withParam, useStages } from "./shell";

declare const AGAPE_WORLD: any;
const SPEEDS = [{ label: "1×", tps: 0.1 }, { label: "6×", tps: 0.6 }, { label: "30×", tps: 3 }];
const START_MINUTE = 32; // a quarter of an hour before tip-off: kitchens open, seats filling
const SEED = 11;

type Stage = "boot" | "welcome" | "signup" | "signin" | "perms" | "venue" | "seat" | "app";

function levelFromUrl(): number { const n = Number(new URLSearchParams(location.search).get("level")); return n >= 1 && n <= LEVELS.length ? n : 1; }
function useWorld(level: number) { setWorldOverride(demoWorld(levelFeatures(level))); }

// ---------- inside the phone: opening Agape for the first time ----------

function Onboarding({ stage, next, seatChosen, onLocated, awayChosen }: { stage: Stage; next: (s: Stage) => void; seatChosen: (section: string, row: string, seat: string) => void; onLocated: (on: boolean) => void; awayChosen: () => void }) {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [sec, setSec] = useState(""); const [row, setRow] = useState(""); const [seat, setSeat] = useState("");
  const [scanning, setScanning] = useState(false);
  const [pw, setPw] = useState(""); const [tried, setTried] = useState(false); const [keepPosted, setKeepPosted] = useState(true);
  const [located, setLocated] = useState(true);
  // Apple and Google hand over the name and email they have
  const social = (n: string, e: string) => { try { localStorage.setItem("agape.profile", JSON.stringify({ name: n, email: e })); } catch { /* fine */ } };
  // a returning fan: their name and the card they saved last time
  const returning = () => { try { localStorage.setItem("agape.profile", JSON.stringify({ name: "Jordan Lee", email: email.trim() || "jordan.lee@gmail.com" })); localStorage.setItem("agape.prefs", JSON.stringify({ cards: ["Visa •••• 7612"], pay: "Visa •••• 7612" })); } catch { /* fine */ } };
  const ok = /^(1(0[1-9]|1\d|2[0-4])|2(0[1-9]|1\d|2[0-4]))$/.test(sec);
  if (stage === "boot") return <div className="ob ob-boot"><div className="ob-mark"><Wordmark /></div></div>;
  if (stage === "welcome") return (
    <div className="ob ob-welcome">
      <div className="ob-hero"><span className="ob-logo"><Wordmark /></span><h1>Order from your seat.<br />It comes to you.</h1><p>Food, drinks and gear at live events, brought to you by the venue's own kitchens, so you never miss the moment.</p></div>
      <div className="ob-show" aria-hidden="true">
        <div className="ob-card back"><b>Draft beer</b><span>Runner 2 min away</span></div>
        <div className="ob-card"><small>Section 104 · Row F · Seat 7</small><b>Hot dog, nachos</b><span>At your seat by <strong>7:42 PM</strong></span>
          <ol><li className="done">Ordered</li><li className="now">Being made</li><li>On the way</li><li>At your seat</li></ol></div>
      </div>
      <div className="ob-acts"><button className="btn ob-primary" onClick={() => next("signup")}>Create an account</button><button className="btn ob-second" onClick={() => next("signin")}>Sign in</button><button className="link-inline" onClick={() => { try { localStorage.setItem("agape.profile", JSON.stringify({ guest: true })); } catch { /* fine */ } next("perms"); }}>Continue as a guest</button></div>
    </div>
  );
  if (stage === "signup") return (
    <form className="ob ob-form" onSubmit={e => { e.preventDefault(); if (!name.trim() || !/.+@.+\..+/.test(email) || pw.length < 8) { setTried(true); return; } try { localStorage.setItem("agape.profile", JSON.stringify({ name: name.trim(), email: email.trim() })); } catch { /* fine */ }
      // the demo's accounts live on this device; if they said yes, Agape's team hears who tried it (never the password)
      if (keepPosted) fetch("/api/leads", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ name: name.trim(), contact: email.trim(), role: "Fan (tried the demo)", note: "Created an account in the fan demo.", version: "" }) }).catch(() => {});
      next("perms"); }}>
      <h2>Create your account</h2>
      <button type="button" className="btn ob-social" onClick={() => { social("Jordan Lee", "jordan@icloud.com"); next("perms"); }}> Continue with Apple</button>
      <button type="button" className="btn ob-social light" onClick={() => { social("Jordan Lee", "jordan.lee@gmail.com"); next("perms"); }}>Continue with Google</button>
      <div className="ob-or"><span>or</span></div>
      <label className="sf wide"><span>Your name</span><input value={name} onChange={e => setName(e.target.value)} placeholder="Jordan Lee" autoComplete="name" /></label>
      <label className="sf wide"><span>Email</span><input type="email" value={email} onChange={e => setEmail(e.target.value)} placeholder="you@example.com" autoComplete="email" /></label>
      <label className="sf wide"><span>Password</span><input type="password" value={pw} onChange={e => setPw(e.target.value)} placeholder="8 or more characters" autoComplete="new-password" /></label>
      {tried && <small className="ob-warn">{!name.trim() ? "Add your name." : !email.trim() ? "Add your email." : !/.+@.+\..+/.test(email) ? "That email doesn't look right." : "Choose a password of 8 or more characters."}</small>}
      <label className="ob-check"><input type="checkbox" checked={keepPosted} onChange={e => setKeepPosted(e.target.checked)} /> <span>Tell me when Agape comes to my venue. Agape's team gets your name and email (not your password) and can contact you.</span></label>
      <button className="btn ob-primary">Continue</button>
      <small className="ob-fine">This is a demo: your account lives on this device. By continuing you agree to Agape's terms and privacy policy.</small>
    </form>
  );
  if (stage === "signin") return (
    <form className="ob ob-form" onSubmit={e => { e.preventDefault(); returning(); next("perms"); }}>
      <h2>Welcome back</h2>
      <button type="button" className="btn ob-social" onClick={() => { returning(); next("perms"); }}> Sign in with Apple</button>
      <button type="button" className="btn ob-social light" onClick={() => { returning(); next("perms"); }}>Sign in with Google</button>
      <div className="ob-or"><span>or</span></div>
      <label className="sf wide"><span>Email</span><input type="email" value={email} onChange={e => setEmail(e.target.value)} placeholder="you@example.com" autoComplete="email" /></label>
      <label className="sf wide"><span>Password</span><input type="password" placeholder="Your password" autoComplete="current-password" /></label>
      <button className="btn ob-primary">Sign in</button>
    </form>
  );
  if (stage === "perms") return (
    <div className="ob ob-perms">
      <h2>Two quick things</h2>
      <div className="ob-perm"><span className="ob-pi">📍</span><div><b>Know when you're at an event</b><small>Agape uses your location only while it's open, to find the venue you're in.</small></div></div>
      <div className="ob-perm"><span className="ob-pi"><Icon name="bell" /></span><div><b>Tell you when it's here</b><small>A heads-up when your order is on its way and when it reaches you.</small></div></div>
      <div className="ob-acts"><button className="btn ob-primary" onClick={() => { setLocated(true); onLocated(true); next("venue"); }}>Allow</button><button className="link-inline" onClick={() => { setLocated(false); onLocated(false); try { const o = JSON.parse(localStorage.getItem("agape.prefs") || "{}"); o.loc = false; o["n-orders"] = false; localStorage.setItem("agape.prefs", JSON.stringify(o)); } catch { /* fine */ } next("venue"); }}>Not now</button></div>
    </div>
  );
  if (stage === "venue") return (
    <div className="ob ob-venue">
      <span className="eyebrow">{located ? "You're at" : "Events tonight"}</span>
      <h2>{BRAND.venue}</h2>
      {EVENT === "concert" ? <div className="ob-event"><div className="ob-teams"><i style={{ background: BRAND.home.color }} />Lumen Rivers: The Tidelines Tour</div><small>Tonight · with Northbound and Paper Kites Club · doors 6:45 PM</small></div>
        : <div className="ob-event"><div className="ob-teams"><i style={{ background: BRAND.home.color }} />{BRAND.home.name}<span>vs</span><i style={{ background: BRAND.away.color }} />{BRAND.away.name}</div><small>Tonight · tip-off 7:30 PM</small></div>}
      <div className="ob-acts"><button className="btn ob-primary" onClick={() => next("seat")}>{located ? "Yes, I'm here" : "I'm at this one"}</button>
        {feature("tickets") && <button className="link-inline ob-away" onClick={awayChosen}>Not at a game right now</button>}</div>
    </div>
  );
  // seat
  return (
    <form className="ob ob-form" onSubmit={e => { e.preventDefault(); if (ok) seatChosen(sec, row.trim().toUpperCase(), seat.trim()); }}>
      <h2>Where are you sitting?</h2>
      <p className="ob-sub">Your runner walks straight to you.</p>
      <button type="button" className={"btn ob-scan" + (scanning ? " on" : "")} onClick={() => { setScanning(true); setTimeout(() => { setSec("104"); setRow("F"); setSeat("7"); setScanning(false); }, 1300); }}>{scanning ? "Reading your ticket…" : "Scan your ticket"}</button>
      <div className="ob-or"><span>or type it</span></div>
      <div className="sf-grid three">
        <label className="sf"><span>Section</span><input inputMode="numeric" value={sec} onChange={e => setSec(e.target.value.replace(/\D/g, "").slice(0, 3))} /></label>
        <label className="sf"><span>Row</span><input value={row} onChange={e => setRow(e.target.value.slice(0, 3))} /></label>
        <label className="sf"><span>Seat</span><input inputMode="numeric" value={seat} onChange={e => setSeat(e.target.value.slice(0, 3))} /></label>
      </div>
      {!ok && sec.length === 3 && <small className="ob-warn">Sections here are 101 to 124 and 201 to 224.</small>}
      <button className="btn ob-primary" disabled={!ok}>Take me to my seat</button>
    </form>
  );
}

// ---------- outside the phone: the story ----------

type Beat = { id: string; title: string; body: React.ReactNode };

function beatFor(stage: Stage, v: FanView | null, level: number, seen: Set<string>, located = true): Beat {
  if (stage === "boot" || stage === "welcome") return { id: "arrive", title: "You've just walked into the arena", body: <>It's {EVENT === "concert" ? "concert" : "game"} night at {BRAND.venue}. The crowd is filling in and the {EVENT === "concert" ? "first act" : "tip-off"} is a few minutes away. You open Agape, the way any fan would. {matchMedia("(max-width: 760px)").matches ? null : <>It's made for a phone; you can try this same demo on yours at agapetoyourseat.com/try.</>}</> };
  if (stage === "signup") return { id: "signup", title: "Signing up takes seconds", body: <>A name and an email, or one tap with Apple or Google. How you'll pay is asked when you first order, and age only when something needs it.</> };
  if (stage === "signin") return { id: "signin", title: "Back again", body: <>A fan who used Agape before signs in and finds their name and saved card ready. Here, any email and password sign you in as Jordan, a returning fan. A guest can order without an account at all.</> };
  if (stage === "perms") return { id: "perms", title: "Two permissions, each with its reason", body: <>Location tells Agape which venue you're in. Notifications tell you when your order is coming. Both are optional.</> };
  if (stage === "venue") return located ? { id: "venue", title: "Agape knows where you are", body: <>Agape recognizes the venue and tonight's event, so there's nothing to search for.</> }
    : { id: "venue", title: "Pick your event", body: <>Without location, Agape asks instead: the events on tonight, nearest first. It's one tap either way.</> };
  if (stage === "seat") return { id: "seat", title: "Your seat is your address", body: <>Section, row and seat: that's where everything comes. Scanning the ticket fills it in.</> };
  if (!v) return { id: "wait", title: "", body: null };
  const o = v.orders[0];
  // once something has reached the fan, the story never goes back to the start
  const done = v.me.receipts.find(r => r.delivery && (r.stage === "Settled" || r.stage === "Delivered"));
  const show = EVENT === "concert" ? "show" : "game";
  if (o && o.stage !== "Settled" && o.stage !== "Delivered") {
    if (o.stage === "Accepted" || o.stage === "Preparing") return { id: "kitchen", title: o.stage === "Accepted" ? `${o.stand} has your order` : `${o.stand} is making it`, body: <>Your order went straight to the stand's screen. The arrival time you were shown is measured from how fast this stand and its runners are going right now. Keep watching the {show}: wherever you go in the app, your order's progress comes with you.</> };
    if (o.stage === "Ready" || o.stage === "Assigned") return { id: "runner", title: "A runner takes the trip", body: <>It's on the ready shelf, and the nearest runner has been offered the trip, with its route and pay shown up front. Runners keep every tip.</> };
    if (o.stage === "PickedUp") return { id: "walk", title: `${v.runner ? runnerName(v.runner.id) : "Your runner"} is on the way`, body: <>Walking the concourse to section {v.you.seat.slice(1)}. The map under your order shows them coming.</> };
  }
  if (done && v.clock.t - (done.delivered ?? 0) < 180_000 && !seen.has("more" + level)) return { id: "here", title: "It's here, and you never left your seat", body: <>Charged only when it arrived, itemized in your history, and the tip went to the runner. That's Agape at its core.{level === 1 ? <> Everything else is built on top of it: switch the level at the top to see what comes next.</> : null}</> };
  if (done) return { id: "more" + level, title: NEXT[level].title, body: NEXT[level].body };
  if (level > 1) return { id: "lv" + level, title: `Level ${level}: ${LEVELS[level - 1].name}`, body: <>{LEVELS[level - 1].line}. Ordering works exactly as at Level 1, and the new tabs at the bottom are what this level adds. Order something to see it all work together, or go straight to the new tabs.</> };
  return { id: "order", title: "You're hungry, and you don't want to miss anything", body: <>Everything on Get shows when it would reach your seat. Pick something and order it. The top of the screen keeps the {show} in one line; tap it to open it.</> };
}

/** What each level asks the fan to try once ordering is understood. */
const NEXT: Record<number, { title: string; body: React.ReactNode }> = {
  1: { title: "Order to your seat, done completely", body: <>This is the launch app: ordering, done completely. Switch the level at the top to see what's built on it.</> },
  2: { title: "Open Deals", body: <>Deals holds this event's offers and sponsor perks, the points you earn on every order, and a better seat when one opens up. A deal shows only during the event it belongs to.</> },
  3: { title: "Open Play", body: <>Make a call before it locks, follow the section battle, and collect moments as they happen. A new call opens with each quarter and locks a minute in, so get your pick in fast.</> },
  4: { title: "Beyond the night", body: <>Deals now carries what happens after tonight: tickets to the next event and reselling yours, a ride home, prepaid parking and perks at places nearby.</> },
  5: { title: "Open People", body: <>Your section's chat, the crowd, and the people you follow. It's what brings fans back to Agape between events.</> },
};

// ---------- the crowd around the phone ----------

function useCrowd(v: FanView | null, sound: boolean) {
  const [pulse, setPulse] = useState(0);
  const last = useRef<string>("");
  const ctx = useRef<AudioContext | null>(null);
  useEffect(() => {
    const m = v?.live.event.find(e => e.kind === "moment");
    const k = m ? m.t + m.text : "";
    if (!m || k === last.current) return;
    const first = last.current === "";
    last.current = k;
    if (first) return;
    setPulse(p => p + 1);
    if (sound) roar(ctx);
  }, [v?.live.event[0]?.t]);
  return pulse;
}
function roar(ctx: { current: AudioContext | null }) {
  try {
    const a = ctx.current ?? (ctx.current = new AudioContext());
    const len = a.sampleRate * 2.4, buf = a.createBuffer(1, len, a.sampleRate), d = buf.getChannelData(0);
    for (let i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * Math.sin(Math.PI * i / len) ** 1.5;
    const src = a.createBufferSource(); src.buffer = buf;
    const f = a.createBiquadFilter(); f.type = "bandpass"; f.frequency.value = 900; f.Q.value = 0.6;
    const g = a.createGain(); g.gain.value = 0.35;
    src.connect(f).connect(g).connect(a.destination); src.start();
  } catch { /* no sound here */ }
}

function Story() {
  const [level, setLevel] = useState(levelFromUrl());
  const [stage, setStage, stepBack] = useStages<Stage>("boot");
  const [v, setV] = useState<FanView | null>(null);
  const [sheet, setSheet] = useState<Sheet>(null);
  const [speed, setSpeed] = useState(1);
  const [playing, setPlaying] = useState(true);
  const [sound, setSound] = useState(false);
  const [toasts, setToasts] = useState<{ id: number; text: string }[]>([]);
  const [hint, setHint] = useState(true);
  const [talk, setTalk] = useState(false);
  const [phone, setPhone] = useState(false);
  const [seen] = useState(() => new Set<string>());
  const [ready, setReady] = useState(false);
  const [located, setLocated] = useState(true);
  const beat = beatFor(stage, v, level, seen, located);
  const waiting = useHoldOnBeat(stage === "app" ? beat.id : "");
  const { full, setFull } = useFull();
  useTopHeight();
  const [explain, setExplain] = useState(false);
  const [note, setNote] = useState<Note | null>(null);
  const acc = useRef(0);
  const toast = (text: string) => { const id = Date.now() + Math.random(); setToasts(t => [...t, { id, text }]); setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 4200); };
  const refresh = () => setV(kernel.view("fan") as FanView);
  const start = async (lv: number, seat: string) => {
    useWorld(lv);
    const r = await boot(SEED, seat);
    if (!r.ok) throw new Error(r.error);
    kernel.step(START_MINUTE * 6);
    setReady(true);
  };
  useEffect(() => { start(level, "s104").then(() => { setTimeout(() => setStage("welcome"), 1900); }); }, []);
  // the night runs from the moment the doors open, whether or not anyone is looking at the phone
  useEffect(() => {
    if (!ready || !playing || sheet || waiting) return;
    const i = setInterval(() => {
      acc.current += SPEEDS[speed].tps * 0.25;
      const n = Math.floor(acc.current);
      if (n < 1) return;
      acc.current -= n;
      kernel.step(n);
      if (stage === "app") refresh(); else setV(kernel.view("fan") as FanView);
    }, 250);
    return () => clearInterval(i);
  }, [ready, playing, speed, sheet, stage, waiting]);
  useEffect(() => { seen.add(beat.id); if (beat.id !== "arrive") setHint(false); }, [beat.id]);
  const pulse = useCrowd(v, sound);
  const changeLevel = async (n: number) => {
    setLevel(n); history.replaceState(null, "", withParam("level", String(n)));
    const seat = v?.you.seat ?? "s104";
    const t = v?.clock.t ?? START_MINUTE * 60000;
    useWorld(n); const r = await boot(SEED, seat); if (!r.ok) return;
    kernel.step(Math.max(START_MINUTE * 6, Math.floor(t / 10000)));
    setSheet(null); refresh();
  };
  const restart = () => { try { localStorage.removeItem("agape.profile"); localStorage.removeItem("agape.prefs"); Object.keys(localStorage).filter(k => k.startsWith("agape.welcomed")).forEach(k => localStorage.removeItem(k)); } catch { /* fine */ } location.reload(); };
  const c = v?.clock;
  const moment = v?.live.event.find(e => e.kind === "moment");
  return (
    <div className={"story phone-demo" + (full ? " full" : "")} style={{ ["--venue" as any]: BRAND.home.color }}>
      <header className="st-top">
        <a className="st-brand" href="/"><b>agape</b><span>Fan · Agape</span></a>
        <button className="btn h-btn st-menu" aria-label="Demo controls" onClick={e => { const h = (e.currentTarget as HTMLElement).closest(".st-top")!; h.classList.toggle("open"); (e.currentTarget as HTMLElement).setAttribute("aria-expanded", String(h.classList.contains("open"))); }}>Controls</button>
        <div className="st-strip">
        <nav className="st-seats" aria-label="Other roles">{[["Fan", "/try/"], ["Runner", "/try/runner/"], ["Stand", "/try/kitchen/"], ["Venue", "/try/venue/"], ["Partner", "/try/partner/"], ["HQ", "/try/hq/"]].map(([l, h]) => <a key={h} href={h + (EVENT === "concert" ? "?event=concert" : "")} className={l === "Fan" ? "on" : ""}>{l}</a>)}</nav>
        <div className="st-levels" role="group" aria-label="Level">
          {LEVELS.map(l => <button key={l.n} className={level === l.n ? "on" : ""} onClick={() => changeLevel(l.n)} title={l.line}><b>{l.n}</b><span>{l.name}</span></button>)}
        </div>
        <EventSwitch />
        </div>
        <div className="st-ctl">
          <button className="btn h-btn" onClick={() => setPlaying(!playing)}>{playing ? "Pause" : "Resume"}</button>
          <div className="seg h-seg" role="group" aria-label="Speed">{SPEEDS.map((s, i) => <button key={s.label} className={speed === i ? "on" : ""} onClick={() => setSpeed(i)}>{s.label}</button>)}</div>
          <button className={"btn h-btn" + (explain ? " lit" : "")} aria-pressed={explain} disabled={stage !== "app"} title={stage === "app" ? "Tap parts of the app to see what they are and why they're there" : "Available once you've taken your seat"} onClick={() => { setExplain(!explain); setNote(null); }}>{explain ? "Explaining" : "Explain"}</button>
          <button className="btn h-btn snd" aria-pressed={sound} onClick={() => setSound(!sound)}>{sound ? "Crowd on" : "Crowd off"}</button>
          <button className="btn h-btn" onClick={() => setFull(true)}>Full screen</button>
          <button className="btn h-btn" onClick={restart}>Start over</button>
        </div>
      </header>
      <main className="st-stage">
        <div className="st-venue" aria-hidden="true">
          <div key={pulse} className={"st-glow" + (pulse ? " roar" : "")} />
          {c && EVENT === "concert" && <div className="st-board"><span>{BRAND.home.name}</span><span>{c.label}</span></div>}
          {c && EVENT !== "concert" && <div className="st-board"><span><i style={{ background: BRAND.home.color }} />{BRAND.home.short} <b className="num">{c.home}</b></span><span className="num">{c.label}</span><span><b className="num">{c.away}</b> {BRAND.away.short} <i style={{ background: BRAND.away.color }} /></span></div>}
          {moment && c && c.t - moment.t < 120000 && <div key={moment.t} className="st-moment">{momentText(moment.text, moment.team)}</div>}
        </div>
        <div className="st-device">
          <div className="device-frame">
            <div className="device-notch" />
            <div className="device-screen">
              {stepBack && <button className="ob-back" onClick={stepBack} aria-label="Back to the step before"><span aria-hidden="true">‹</span> Back</button>}
              {stage === "app" && v ? <FanApp v={v} refresh={refresh} sheet={sheet} setSheet={setSheet} toast={toast} />
                : <Onboarding stage={stage} next={setStage} onLocated={setLocated} seatChosen={async (sec, row, place) => {
                  if (`s${sec}` !== "s104") { await start(level, `s${sec}`); }
                  kernel.act({ do: "spot", row: `Row ${row}`, place: `seat ${place}` });
                  try { const o = JSON.parse(localStorage.getItem("agape.prefs") || "{}"); o.spot = `Row ${row}, seat ${place}`; o.away = false; localStorage.setItem("agape.prefs", JSON.stringify(o)); } catch { /* fine */ }
                  refresh(); setStage("app");
                }} awayChosen={() => {
                  try { const o = JSON.parse(localStorage.getItem("agape.prefs") || "{}"); o.away = true; localStorage.setItem("agape.prefs", JSON.stringify(o)); } catch { /* fine */ }
                  refresh(); setStage("app");
                }} />}
              <ExplainLayer on={explain && stage === "app"} root={() => document.querySelector(".device-screen")} notes={FAN_NOTES} pick={setNote} picked={note} />
              <div className="toasts in-phone" aria-live="polite">{toasts.slice(-1).map(t => <p key={t.id} className="toast">{t.text}</p>)}</div>
            </div>
          </div>
          {hint && <p className="st-hint">Made for your phone. On a computer, it's shown at phone size.</p>}
        </div>
        <aside className="st-story" aria-live="polite">
          {<>
          {explain && stage === "app" && <p className="st-tip">Explain is on: tap anything glowing on the phone.</p>}
          <span className="eyebrow">{stage === "app" ? `Level ${level} · ${LEVELS[level - 1].name}` : "Opening Agape"}</span>
          <h2 key={beat.id}>{beat.title}</h2>
          <p key={beat.id + "b"}>{beat.body}</p>
          {stage === "app" && <p className="st-status"><b className={"lv-" + LEVELS[level - 1].status.toLowerCase().replace(" ", "-")}>{LEVELS[level - 1].status}</b> {LEVELS[level - 1].gaps}</p>}
          {stage === "app" && v && <div className="st-meta">
            <span>{profileName() || "You"} · section {v.you.seat.slice(1)}</span>
            {v.orders[0] && speed < SPEEDS.length - 1 && <button className="btn sm st-ff" onClick={() => setSpeed(SPEEDS.length - 1)}>Fast-forward until it arrives</button>}
            {v.orders[0] && <span>Order: {v.orders[0].items[0]?.name} · {({ Accepted: "placed", Preparing: "being made", Ready: "ready", Assigned: "runner on it", PickedUp: "on the way" } as Record<string, string>)[v.orders[0].stage] ?? "on its way"} · here by {venueTime(v.orders[0].eta)}</span>}
          </div>}
          </>}
          <div className="st-more">
            <a className="btn ghost sm" href="/">What is Agape?</a>
            <a className="btn ghost sm" href="/demo/">Behind the scenes</a>
            <button className="btn ghost sm" onClick={() => setTalk(true)}>Talk to us</button>
            <button className="btn ghost sm" onClick={() => setPhone(true)}>Get it on your phone</button>
          </div>
        </aside>
      </main>
      {full && <HeldCard beat={beat} on={waiting} exit={() => setFull(false)} />}
      {note && <DemoModal close={() => setNote(null)}><ExplainCard n={note} close={() => setNote(null)} /></DemoModal>}
      {phone && <DemoModal close={() => setPhone(false)}><OnYourPhone close={() => setPhone(false)} /></DemoModal>}
      {talk && <TalkToUs close={() => setTalk(false)} version={null} />}
    </div>
  );
}

applyLook(savedLook()); applyMode(savedMode());
setVenueColor(null);
try { localStorage.removeItem("agape.profile"); localStorage.removeItem("agape.prefs"); Object.keys(localStorage).filter(k => k.startsWith("agape.welcomed")).forEach(k => localStorage.removeItem(k)); } catch { /* fine */ }
ReactDOM.createRoot(document.getElementById("root")!).render(<Story />);

// The fan demo: Agape on a phone, opened the way a fan would open it, with the night's story told beside it.
// Everything about the demo (the story, levels, speed, sound) lives here, outside the phone. The app inside
// never mentions a demo.
import { React, ReactDOM, useEffect, useRef, useState } from "./r";
import { boot, kernel, setWorldOverride } from "./kernel";
import { FanApp, FanView, Sheet, profileName } from "./fan";
import { BRAND, Icon, LEVELS, levelFeatures, momentText, runnerName, venueTime } from "./parts";
import { applyLook, applyMode, savedLook, savedMode, setVenueColor } from "./themes";
import { TalkToUs } from "./talk";
import { ExplainCard, ExplainLayer, FAN_NOTES, Note } from "./explain";

declare const AGAPE_WORLD: any;
const SPEEDS = [{ label: "1×", tps: 0.1 }, { label: "6×", tps: 0.6 }, { label: "30×", tps: 3 }];
const START_MINUTE = 32; // a quarter of an hour before tip-off: kitchens open, seats filling
const SEED = 11;

type Stage = "boot" | "welcome" | "signup" | "perms" | "venue" | "seat" | "app";

function levelFromUrl(): number { const n = Number(new URLSearchParams(location.search).get("level")); return n >= 1 && n <= LEVELS.length ? n : 1; }
function useWorld(level: number) { setWorldOverride({ ...AGAPE_WORLD, features: levelFeatures(level) }); }

// ---------- inside the phone: opening Agape for the first time ----------

function Onboarding({ stage, next, seatChosen }: { stage: Stage; next: (s: Stage) => void; seatChosen: (section: string, row: string, seat: string) => void }) {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [sec, setSec] = useState("104"); const [row, setRow] = useState("F"); const [seat, setSeat] = useState("7");
  const [scanning, setScanning] = useState(false);
  const ok = /^(1(0[1-9]|1\d|2[0-4])|2(0[1-9]|1\d|2[0-4]))$/.test(sec);
  if (stage === "boot") return <div className="ob ob-boot"><div className="ob-mark"><b>agape</b></div></div>;
  if (stage === "welcome") return (
    <div className="ob ob-welcome">
      <div className="ob-hero"><b className="ob-logo">agape</b><h1>Order from your seat.<br />It comes to you.</h1><p>Food, drinks and gear at live events, brought to you by the venue's own kitchens, so you never miss the moment.</p></div>
      <div className="ob-acts"><button className="btn ob-primary" onClick={() => next("signup")}>Get started</button><button className="link-inline" onClick={() => next("signup")}>I already have an account</button></div>
    </div>
  );
  if (stage === "signup") return (
    <form className="ob ob-form" onSubmit={e => { e.preventDefault(); try { localStorage.setItem("agape.profile", JSON.stringify({ name: name.trim() || "Jordan", email: email.trim() })); } catch { /* fine */ } next("perms"); }}>
      <h2>Create your account</h2>
      <button type="button" className="btn ob-social" onClick={() => { setName(n => n || "Jordan Lee"); setEmail(e => e || "jordan@icloud.com"); }}> Continue with Apple</button>
      <button type="button" className="btn ob-social light" onClick={() => { setName(n => n || "Jordan Lee"); setEmail(e => e || "jordan.lee@gmail.com"); }}>Continue with Google</button>
      <div className="ob-or"><span>or</span></div>
      <label className="sf wide"><span>Your name</span><input value={name} onChange={e => setName(e.target.value)} placeholder="Jordan Lee" autoComplete="name" /></label>
      <label className="sf wide"><span>Email</span><input type="email" value={email} onChange={e => setEmail(e.target.value)} placeholder="you@example.com" autoComplete="email" /></label>
      <button className="btn ob-primary">Continue</button>
      <small className="ob-fine">By continuing you agree to Agape's terms and privacy policy.</small>
    </form>
  );
  if (stage === "perms") return (
    <div className="ob ob-perms">
      <h2>Two quick things</h2>
      <div className="ob-perm"><span className="ob-pi">📍</span><div><b>Know when you're at an event</b><small>Agape uses your location only while it's open, to find the venue you're in.</small></div></div>
      <div className="ob-perm"><span className="ob-pi"><Icon name="bell" /></span><div><b>Tell you when it's here</b><small>A heads-up when your order is on its way and when it reaches you.</small></div></div>
      <div className="ob-acts"><button className="btn ob-primary" onClick={() => next("venue")}>Allow</button><button className="link-inline" onClick={() => next("venue")}>Not now</button></div>
    </div>
  );
  if (stage === "venue") return (
    <div className="ob ob-venue">
      <span className="eyebrow">You're at</span>
      <h2>{BRAND.venue}</h2>
      <div className="ob-event"><div className="ob-teams"><i style={{ background: BRAND.home.color }} />{BRAND.home.name}<span>vs</span><i style={{ background: BRAND.away.color }} />{BRAND.away.name}</div><small>Tonight · tip-off 7:30 PM</small></div>
      <div className="ob-acts"><button className="btn ob-primary" onClick={() => next("seat")}>That's me</button></div>
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
      {!ok && <small className="ob-warn">Sections here are 101 to 124 and 201 to 224.</small>}
      <button className="btn ob-primary" disabled={!ok}>Take me to my seat</button>
    </form>
  );
}

// ---------- outside the phone: the story ----------

type Beat = { id: string; title: string; body: React.ReactNode };

function beatFor(stage: Stage, v: FanView | null, level: number, seen: Set<string>): Beat {
  if (stage === "boot" || stage === "welcome") return { id: "arrive", title: "You've just walked into the arena", body: <>It's game night at {BRAND.venue}. The crowd is filling in and tip-off is a few minutes away. You open Agape, the way any fan would. It's made for a phone; you can try this same demo on yours at agapetoyourseat.com/try.</> };
  if (stage === "signup") return { id: "signup", title: "Signing up takes seconds", body: <>A name and an email, or one tap with Apple or Google. No birthday, no card yet: Agape asks for those only when they're needed.</> };
  if (stage === "perms") return { id: "perms", title: "Two permissions, each with its reason", body: <>Location tells Agape which venue you're in. Notifications tell you when your order is coming. Both are optional.</> };
  if (stage === "venue") return { id: "venue", title: "Agape knows where you are", body: <>The venue and tonight's game are recognized, so there's nothing to search for.</> };
  if (stage === "seat") return { id: "seat", title: "Your seat is your address", body: <>Section, row and seat: that's where everything comes. Scanning the ticket fills it in.</> };
  if (!v) return { id: "wait", title: "", body: null };
  const o = v.orders[0];
  const done = v.me.receipts.find(r => r.stage === "Settled" && r.delivery);
  if (o) {
    if (o.stage === "Accepted" || o.stage === "Preparing") return { id: "kitchen", title: `${o.stand} is making it`, body: <>Your order went straight to the kitchen's screen. The arrival time you were shown comes from how long this kitchen and its runners are actually taking right now, not a guess. Keep watching the game; the bar at the top of your screen follows your order.</> };
    if (o.stage === "Ready" || o.stage === "Assigned") return { id: "runner", title: "A runner takes the trip", body: <>It's on the ready shelf, and the nearest runner has been offered the trip, with its route and pay shown up front. Runners keep every tip.</> };
    if (o.stage === "PickedUp") return { id: "walk", title: `${v.runner ? runnerName(v.runner.id) : "Your runner"} is on the way`, body: <>Walking the concourse to section {v.you.seat.slice(1)}. Open Me to watch them come.</> };
  }
  if (done && !seen.has("more")) return { id: "here", title: "It's here, and you never left your seat", body: <>Charged only when it arrived, itemized in your history, and the tip went to the runner. That's Agape at its core. {level < LEVELS.length ? <>Everything else is built on top of it: switch the level above to see what comes next.</> : null}</> };
  if (done) return { id: "more", title: LEVELS[level - 1].name, body: <>{LEVELS[level - 1].line}. {level === 1 ? "This is the launch app: ordering, done completely." : "Each tab arrived at a level: try them."}</> };
  return { id: "order", title: "You're hungry, and you don't want to miss anything", body: <>Everything on Get shows when it would reach your seat. Pick something and order it. The top of the screen keeps the game in one line; tap it to open the game.</> };
}

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
  const [stage, setStage] = useState<Stage>("boot");
  const [v, setV] = useState<FanView | null>(null);
  const [sheet, setSheet] = useState<Sheet>(null);
  const [speed, setSpeed] = useState(1);
  const [playing, setPlaying] = useState(true);
  const [sound, setSound] = useState(false);
  const [toasts, setToasts] = useState<{ id: number; text: string }[]>([]);
  const [hint, setHint] = useState(true);
  const [talk, setTalk] = useState(false);
  const [seen] = useState(() => new Set<string>());
  const [ready, setReady] = useState(false);
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
    if (!ready || !playing || sheet) return;
    const i = setInterval(() => {
      acc.current += SPEEDS[speed].tps * 0.25;
      const n = Math.floor(acc.current);
      if (n < 1) return;
      acc.current -= n;
      kernel.step(n);
      if (stage === "app") refresh(); else setV(kernel.view("fan") as FanView);
    }, 250);
    return () => clearInterval(i);
  }, [ready, playing, speed, sheet, stage]);
  const beat = beatFor(stage, v, level, seen);
  useEffect(() => { seen.add(beat.id); if (beat.id !== "arrive") setHint(false); }, [beat.id]);
  const pulse = useCrowd(v, sound);
  const changeLevel = async (n: number) => {
    setLevel(n); history.replaceState(null, "", `?level=${n}`);
    const seat = v?.you.seat ?? "s104";
    const t = v?.clock.t ?? START_MINUTE * 60000;
    useWorld(n); const r = await boot(SEED, seat); if (!r.ok) return;
    kernel.step(Math.max(START_MINUTE * 6, Math.floor(t / 10000)));
    setSheet(null); refresh();
  };
  const restart = () => { try { localStorage.removeItem("agape.profile"); Object.keys(localStorage).filter(k => k.startsWith("agape.welcomed")).forEach(k => localStorage.removeItem(k)); } catch { /* fine */ } location.reload(); };
  const c = v?.clock;
  const moment = v?.live.event.find(e => e.kind === "moment");
  return (
    <div className="story" style={{ ["--venue" as any]: BRAND.home.color }}>
      <header className="st-top">
        <a className="st-brand" href="/"><b>agape</b><span>demo</span></a>
        <nav className="st-seats" aria-label="Other seats">{[["Fan", "/try/"], ["Runner", "/try/runner/"], ["Kitchen", "/try/kitchen/"], ["Venue", "/try/venue/"], ["Partner", "/try/partner/"]].map(([l, h]) => <a key={h} href={h} className={l === "Fan" ? "on" : ""}>{l}</a>)}</nav>
        <div className="st-levels" role="group" aria-label="Level">
          {LEVELS.map(l => <button key={l.n} className={level === l.n ? "on" : ""} onClick={() => changeLevel(l.n)} title={l.line}><b>{l.n}</b><span>{l.name}</span></button>)}
        </div>
        <div className="st-ctl">
          <button className="btn h-btn" onClick={() => setPlaying(!playing)}>{playing ? "Pause" : "Play"}</button>
          <div className="seg h-seg" role="group" aria-label="Speed">{SPEEDS.map((s, i) => <button key={s.label} className={speed === i ? "on" : ""} onClick={() => setSpeed(i)}>{s.label}</button>)}</div>
          <button className={"btn h-btn" + (explain ? " lit" : "")} aria-pressed={explain} disabled={stage !== "app"} onClick={() => { setExplain(!explain); setNote(null); }}>{explain ? "Explaining" : "Explain"}</button>
          <button className="btn h-btn" aria-pressed={sound} onClick={() => setSound(!sound)}>{sound ? "Crowd on" : "Crowd off"}</button>
          <button className="btn h-btn" onClick={restart}>Start over</button>
        </div>
      </header>
      <main className="st-stage">
        <div className="st-venue" aria-hidden="true">
          <div key={pulse} className={"st-glow" + (pulse ? " roar" : "")} />
          {c && <div className="st-board"><span><i style={{ background: BRAND.home.color }} />{BRAND.home.short} <b className="num">{c.home}</b></span><span className="num">{c.label}</span><span><b className="num">{c.away}</b> {BRAND.away.short} <i style={{ background: BRAND.away.color }} /></span></div>}
          {moment && c && c.t - moment.t < 120000 && <div key={moment.t} className="st-moment">{momentText(moment.text, moment.team)}</div>}
        </div>
        <div className="st-device">
          <div className="device-frame">
            <div className="device-notch" />
            <div className="device-screen">
              {stage === "app" && v ? <FanApp v={v} refresh={refresh} sheet={sheet} setSheet={setSheet} toast={toast} />
                : <Onboarding stage={stage} next={setStage} seatChosen={async (sec, row, place) => {
                  if (`s${sec}` !== "s104") { await start(level, `s${sec}`); }
                  kernel.act({ do: "spot", row: `Row ${row}`, place: `seat ${place}` });
                  try { const o = JSON.parse(localStorage.getItem("agape.prefs") || "{}"); o.spot = `Row ${row}, seat ${place}`; localStorage.setItem("agape.prefs", JSON.stringify(o)); } catch { /* fine */ }
                  refresh(); setStage("app");
                }} />}
              <ExplainLayer on={explain && stage === "app"} root={() => document.querySelector(".device-screen")} notes={FAN_NOTES} pick={setNote} picked={note} />
              <div className="toasts in-phone" aria-live="polite">{toasts.slice(-1).map(t => <p key={t.id} className="toast">{t.text}</p>)}</div>
            </div>
          </div>
          {hint && <p className="st-hint">Made for your phone. On a computer, it's shown at phone size.</p>}
        </div>
        <aside className="st-story" aria-live="polite">
          {note ? <ExplainCard n={note} close={() => setNote(null)} /> : <>
          {explain && stage === "app" && <p className="st-tip">Explain is on: tap anything glowing on the phone.</p>}
          <span className="eyebrow">{stage === "app" ? `Level ${level} · ${LEVELS[level - 1].name}` : "Opening Agape"}</span>
          <h2 key={beat.id}>{beat.title}</h2>
          <p key={beat.id + "b"}>{beat.body}</p>
          {stage === "app" && v && <div className="st-meta">
            <span>{profileName() || "You"} · section {v.you.seat.slice(1)}</span>
            {v.orders[0] && <span>Order: {v.orders[0].items[0]?.name} · {v.orders[0].stage === "PickedUp" ? "on the way" : v.orders[0].stage.toLowerCase()} · here by {venueTime(v.orders[0].eta)}</span>}
          </div>}
          </>}
          <div className="st-more">
            <a className="btn ghost sm" href="/">What is Agape?</a>
            <a className="btn ghost sm" href="/demo/">Behind the scenes</a>
            <button className="btn ghost sm" onClick={() => setTalk(true)}>Talk to us</button>
          </div>
        </aside>
      </main>
      {talk && <TalkToUs close={() => setTalk(false)} version={null} />}
    </div>
  );
}

applyLook(savedLook()); applyMode(savedMode());
setVenueColor(null);
try { localStorage.removeItem("agape.profile"); Object.keys(localStorage).filter(k => k.startsWith("agape.welcomed")).forEach(k => localStorage.removeItem(k)); } catch { /* fine */ }
ReactDOM.createRoot(document.getElementById("root")!).render(<Story />);

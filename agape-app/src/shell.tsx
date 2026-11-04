// The demo's frame for any one person's story: the night around the device, the device sized to that
// person's hands, and the story beside it. The app inside the device never knows it's in a demo.
import { React, useEffect, useRef, useState } from "./r";
import { kernel } from "./kernel";
import { BRAND, LEVELS, concertWorld, isConcert, momentText } from "./parts";
import { ExplainCard, ExplainLayer, Note } from "./explain";
import { TalkToUs } from "./talk";

export const SPEEDS = [{ label: "1×", tps: 0.1 }, { label: "6×", tps: 0.6 }, { label: "30×", tps: 3 }];

/** Runs the night at the chosen speed and calls back after each step. */
export function useNight(ready: boolean, paused: boolean, speed: number, tick: () => void) {
  const acc = useRef(0);
  const cb = useRef(tick); cb.current = tick;
  useEffect(() => {
    if (!ready || paused) return;
    const i = setInterval(() => {
      acc.current += SPEEDS[speed].tps * 0.25;
      const n = Math.floor(acc.current);
      if (n < 1) return;
      acc.current -= n;
      kernel.step(n);
      cb.current();
    }, 250);
    return () => clearInterval(i);
  }, [ready, paused, speed]);
}

export type Beat = { id: string; title: string; body: React.ReactNode };

export function Shell({ tag, device, screen, beat, meta, notes, explainable, toasts, ready, onTick, hold, level, minLevel = 1 }: {
  tag: string; device: "phone" | "tablet" | "laptop"; screen: React.ReactNode; beat: Beat; meta?: React.ReactNode;
  notes: Note[]; explainable: boolean; toasts: { id: number; text: string }[]; ready: boolean; onTick: () => void; hold?: boolean; level?: number; minLevel?: number;
}) {
  const [speed, setSpeed] = useState(1);
  const [playing, setPlaying] = useState(true);
  const [explain, setExplain] = useState(false);
  const [note, setNote] = useState<Note | null>(null);
  const [talk, setTalk] = useState(false);
  const [phone, setPhone] = useState(false);
  const [fan, setFan] = useState<any>(null);
  const [pulse, setPulse] = useState(0);
  const last = useRef("");
  const waiting = useHoldOnBeat(beat.id);
  const { full, setFull } = useFull();
  useTopHeight();
  useNight(ready, !playing || !!hold || waiting, speed, onTick);
  // the event around the device: the scoreboard and the roar on big moments
  useEffect(() => {
    const i = setInterval(() => {
      try {
        const f = kernel.view("fan"); setFan(f);
        const m = f.live.event.find((e: any) => e.kind === "moment"); const k = m ? m.t + m.text : "";
        if (k && last.current && k !== last.current) setPulse(p => p + 1);
        last.current = k;
      } catch { /* not booted yet */ }
    }, 1000);
    return () => clearInterval(i);
  }, []);
  const c = fan?.clock;
  const moment = fan?.live.event.find((e: any) => e.kind === "moment");
  return (
    <div className={"story " + (device === "phone" ? "phone-demo" : device) + (full ? " full" : "")} style={{ ["--venue" as any]: BRAND.home.color }}>
      <header className="st-top">
        <a className="st-brand" href="/"><b>agape</b><span>{tag}</span></a>
        <div className="st-strip">
        <nav className="st-seats" aria-label="Other roles">
          {[["Fan", "/try/"], ["Runner", "/try/runner/"], ["Stand", "/try/kitchen/"], ["Venue", "/try/venue/"], ["Partner", "/try/partner/"], ["HQ", "/try/hq/"]].map(([l, h]) =>
            <a key={h} href={h + (EVENT === "concert" ? "?event=concert" : "")} className={location.pathname === h ? "on" : ""}>{l}</a>)}
        </nav>
        {level != null && <div className="st-levels" role="group" aria-label="Level">
          {LEVELS.filter(l => l.n >= minLevel).map(l => <a key={l.n} className={"st-lv" + (level === l.n ? " on" : "")} href={withParam("level", String(l.n))} title={l.line}><b>{l.n}</b><span>{l.name}</span></a>)}
        </div>}
        <EventSwitch />
        </div>
        <div className="st-ctl">
          <button className="btn h-btn" onClick={() => setPlaying(!playing)}>{playing ? "Pause" : "Play"}</button>
          <div className="seg h-seg" role="group" aria-label="Speed">{SPEEDS.map((s, i) => <button key={s.label} className={speed === i ? "on" : ""} onClick={() => setSpeed(i)}>{s.label}</button>)}</div>
          <button className={"btn h-btn" + (explain ? " lit" : "")} aria-pressed={explain} disabled={!explainable} onClick={() => { setExplain(!explain); setNote(null); }}>{explain ? "Explaining" : "Explain"}</button>
          <button className="btn h-btn" onClick={() => setFull(true)}>Full screen</button>
          <button className="btn h-btn" onClick={() => location.reload()}>Start over</button>
        </div>
      </header>
      <main className="st-stage">
        <div className="st-venue" aria-hidden="true">
          <div key={pulse} className={"st-glow" + (pulse ? " roar" : "")} />
          {c && isConcert() && <div className="st-board"><span>{BRAND.home.name}</span><span>{c.label}</span></div>}
          {c && !isConcert() && <div className="st-board"><span><i style={{ background: BRAND.home.color }} />{BRAND.home.short} <b className="num">{c.home}</b></span><span className="num">{c.label}</span><span><b className="num">{c.away}</b> {BRAND.away.short} <i style={{ background: BRAND.away.color }} /></span></div>}
          {moment && c && c.t - moment.t < 120000 && <div key={moment.t} className="st-moment">{momentText(moment.text, moment.team)}</div>}
        </div>
        <div className="st-device">
          <div className={"device-frame " + device}>
            {device === "phone" && <div className="device-notch" />}
            <div className="device-screen">
              {screen}
              <ExplainLayer on={explain && explainable} root={() => document.querySelector(".device-screen")} notes={notes} pick={setNote} picked={note} />
              <div className="toasts in-phone" aria-live="polite">{toasts.slice(-1).map(t => <p key={t.id} className="toast">{t.text}</p>)}</div>
            </div>
          </div>
        </div>
        <aside className="st-story" aria-live="polite">
          {<>
            {explain && <p className="st-tip">Explain is on: tap anything glowing.</p>}
            <span className="eyebrow">{tag}</span>
            <h2 key={beat.id}>{beat.title}</h2>
            <p key={beat.id + "b"}>{beat.body}</p>
            {meta && <div className="st-meta">{meta}</div>}
          </>}
          <div className="st-more">
            <a className="btn ghost sm" href="/">What is Agape?</a>
            <a className="btn ghost sm" href="/demo/">Behind the scenes</a>
            <button className="btn ghost sm" onClick={() => setTalk(true)}>Talk to us</button>
            <button className="btn ghost sm" onClick={() => setPhone(true)}>Get it on your phone</button>
          </div>
        </aside>
      </main>
      {phone && <DemoModal close={() => setPhone(false)}><OnYourPhone close={() => setPhone(false)} /></DemoModal>}
      {full && <HeldCard beat={beat} on={waiting} exit={() => setFull(false)} />}
      {note && <DemoModal close={() => setNote(null)}><ExplainCard n={note} close={() => setNote(null)} /></DemoModal>}
      {talk && <TalkToUs close={() => setTalk(false)} version={null} />}
    </div>
  );
}

export function useToasts() {
  const [toasts, setToasts] = useState<{ id: number; text: string }[]>([]);
  const toast = (text: string) => { const id = Date.now() + Math.random(); setToasts(t => [...t, { id, text }]); setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 4200); };
  return { toasts, toast };
}

/** The level asked for in the address, kept within what this seat has. */
export function levelParam(min: number, d: number) { const n = Number(new URLSearchParams(location.search).get("level")); return n >= min && n <= LEVELS.length ? n : d; }

declare const AGAPE_WORLD: any;
/** Game or concert, from the address. */
export const EVENT = new URLSearchParams(location.search).get("event") === "concert" ? "concert" : "game";
/** The world this demo runs: the sample arena, as a game or a concert, with the given features. */
export function demoWorld(features?: Record<string, boolean>) {
  const base = EVENT === "concert" ? concertWorld(AGAPE_WORLD) : { ...AGAPE_WORLD };
  return features ? { ...base, features: { ...features, ...(EVENT === "concert" ? { predictions: false, colors: false } : {}) } } : base;
}
/** A link to this page with one setting changed. */
export function withParam(k: string, v: string) { const q = new URLSearchParams(location.search); q.set(k, v); return `?${q}`; }
/** Game or concert, as two links. */
export function EventSwitch() {
  return <div className="st-levels st-event" role="group" aria-label="Kind of night">
    {[["game", "Game"], ["concert", "Concert"]].map(([k, l]) => <a key={k} className={"st-lv" + (EVENT === k ? " on" : "")} href={withParam("event", k)}><span>{l}</span></a>)}
  </div>;
}

/** The story holds the night at each new step until the person does something on the page: a tap, a click
 *  or a key, inside the app or outside it. Then the night carries on. */
export function useHoldOnBeat(beatId: string) {
  const [waiting, setWaiting] = useState(false);
  const first = useRef(true);
  // held is shown by stillness: every moving thing on the page stops until the person acts
  useEffect(() => { document.documentElement.classList.toggle("night-held", waiting); }, [waiting]);
  useEffect(() => {
    if (first.current) { first.current = false; return; }
    if (!beatId || beatId === "x" || beatId === "wait") return;
    setWaiting(true);
  }, [beatId]);
  useEffect(() => {
    if (!waiting) return;
    const go = () => setWaiting(false);
    // the next action, not the one that brought this step on
    const t = setTimeout(() => { addEventListener("pointerdown", go, { once: true, capture: true }); addEventListener("keydown", go, { once: true, capture: true }); }, 350);
    return () => { clearTimeout(t); removeEventListener("pointerdown", go, true); removeEventListener("keydown", go, true); };
  }, [waiting]);
  return waiting;
}

/** The header's height, so the device can be sized to the space left under it. */
export function useTopHeight() {
  useEffect(() => {
    const el = document.querySelector(".st-top") as HTMLElement | null; if (!el) return;
    // a phone is drawn at a real phone's size (390 × 844) and scaled to fit, so what's inside lays out as on a phone
    const set = () => { const r = document.documentElement.style; r.setProperty("--st-top", el.offsetHeight + "px"); r.setProperty("--dz", String(Math.min(1, Math.max(0.5, (innerHeight - el.offsetHeight - 92) / 868)))); };
    set(); const o = new ResizeObserver(set); o.observe(el); addEventListener("resize", set); return () => { o.disconnect(); removeEventListener("resize", set); };
  }, []);
}

/** Full screen: the app alone, edge to edge, with a way back. */
/** Opened from a phone's home screen: the app on its own, edge to edge. */
export const standalone = () => { try { return matchMedia("(display-mode: standalone)").matches || !!(navigator as any).standalone || new URLSearchParams(location.search).get("app") === "1"; } catch { return false; } };

export function useFull() {
  const [full, setFullState] = useState(standalone);
  useEffect(() => { document.documentElement.classList.toggle("demo-full", full); }, [full]);
  useEffect(() => { document.documentElement.classList.toggle("standalone", standalone()); }, []);
  // the screen's real full screen where the browser allows it (computers, Android, iPad); on an
  // iPhone, where Safari only allows it for video, the app fills the browser window instead
  useEffect(() => {
    const sync = () => { const d = document as any; if (!(d.fullscreenElement || d.webkitFullscreenElement)) setFullState(false); };
    document.addEventListener("fullscreenchange", sync); document.addEventListener("webkitfullscreenchange", sync);
    return () => { document.removeEventListener("fullscreenchange", sync); document.removeEventListener("webkitfullscreenchange", sync); };
  }, []);
  const setFull = (on: boolean) => {
    const el = document.documentElement as any, d = document as any;
    setFullState(on);
    try {
      if (on && standalone()) return;
      if (on) { const req = el.requestFullscreen ?? el.webkitRequestFullscreen; const p = req?.call(el, { navigationUI: "hide" }); p?.catch?.(() => {}); }
      else if (d.fullscreenElement || d.webkitFullscreenElement) { const p = (d.exitFullscreen ?? d.webkitExitFullscreen)?.call(d); p?.catch?.(() => {}); }
    } catch { /* the in-page full screen still applies */ }
  };
  return { full, setFull };
}

/** The story, shrunk to one card over a full-screen app while the night is held for it. */
export function HeldCard({ beat, on, exit }: { beat: Beat; on: boolean; exit: () => void }) {
  return <>
    <button className="full-exit" onClick={exit} aria-label="Leave full screen">Exit full screen</button>
    {on && <div className="demo-dim held"><div className="held-card" role="status"><b>{beat.title}</b><span>{beat.body}</span><button className="btn sm">Continue</button></div></div>}
  </>;
}

/** How to put this app on a phone's home screen, or send it to someone. */
export function OnYourPhone({ close }: { close: () => void }) {
  const url = location.origin + location.pathname;
  const [copied, setCopied] = useState(false);
  const ios = /iPhone|iPad|iPod/.test(navigator.userAgent);
  return <div className="explain-card">
    <div className="row gap"><span className="eyebrow grow">On your phone</span><button className="btn ghost sm" onClick={close}>Close</button></div>
    <h2>Use it like an app</h2>
    <dl>
      <dt>On this phone</dt><dd>{ios ? <>Tap Share in Safari, then <b>Add to Home Screen</b>.</> : <>Open your browser's menu, then <b>Install app</b> or <b>Add to Home screen</b>.</>} It opens on its own, full screen, with its own icon.</dd>
      <dt>On another phone</dt><dd>Send this link and do the same there: <span className="num">{url.replace(/^https?:\/\//, "")}</span></dd>
      <dt>Updates</dt><dd>It's always the latest version. Nothing to update.</dd>
    </dl>
    <button className="btn primary block" onClick={() => { try { navigator.clipboard?.writeText(url); setCopied(true); } catch { /* fine */ } }}>{copied ? "Link copied" : "Copy the link"}</button>
  </div>;
}

/** Anything the demo says over the app: the app darkens behind it until it's put away. */
export function DemoModal({ close, children, wide }: { close: () => void; children: React.ReactNode; wide?: boolean }) {
  useEffect(() => { const k = (e: KeyboardEvent) => e.key === "Escape" && close(); addEventListener("keydown", k); return () => removeEventListener("keydown", k); }, []);
  return <div className="demo-dim" onClick={e => e.target === e.currentTarget && close()}><div className={"demo-modal" + (wide ? " wide" : "")} role="dialog" aria-modal="true">{children}</div></div>;
}

/** Inviting someone takes their email first. */
export function Invite({ toast }: { toast: (t: string) => void }) {
  const [email, setEmail] = useState("");
  return <form className="row gap invite" onSubmit={e => { e.preventDefault(); if (!/.+@.+\..+/.test(email)) { toast("Add their work email first."); return; } toast(`Invitation sent to ${email}.`); setEmail(""); }}>
    <input className="grow" type="email" value={email} onChange={e => setEmail(e.target.value)} placeholder="Their work email" aria-label="Email to invite" />
    <button className="btn ghost">Invite</button></form>;
}


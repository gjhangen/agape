// The demo's frame for any one person's story: the night around the device, the device sized to that
// person's hands, and the story beside it. The app inside the device never knows it's in a demo.
import { React, useEffect, useRef, useState } from "./r";
import { kernel } from "./kernel";
import { BRAND, momentText } from "./parts";
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

export function Shell({ tag, device, screen, beat, meta, notes, explainable, toasts, ready, onTick, hold }: {
  tag: string; device: "phone" | "tablet" | "laptop"; screen: React.ReactNode; beat: Beat; meta?: React.ReactNode;
  notes: Note[]; explainable: boolean; toasts: { id: number; text: string }[]; ready: boolean; onTick: () => void; hold?: boolean;
}) {
  const [speed, setSpeed] = useState(1);
  const [playing, setPlaying] = useState(true);
  const [explain, setExplain] = useState(false);
  const [note, setNote] = useState<Note | null>(null);
  const [talk, setTalk] = useState(false);
  const [fan, setFan] = useState<any>(null);
  const [pulse, setPulse] = useState(0);
  const last = useRef("");
  useNight(ready, !playing || !!hold, speed, onTick);
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
    <div className={"story " + device} style={{ ["--venue" as any]: BRAND.home.color }}>
      <header className="st-top">
        <a className="st-brand" href="/"><b>agape</b><span>{tag}</span></a>
        <nav className="st-seats" aria-label="Other seats">
          {[["Fan", "/try/"], ["Runner", "/try/runner/"], ["Kitchen", "/try/kitchen/"], ["Venue", "/try/venue/"], ["Partner", "/try/partner/"]].map(([l, h]) =>
            <a key={h} href={h} className={location.pathname === h ? "on" : ""}>{l}</a>)}
        </nav>
        <div className="st-ctl">
          <button className="btn h-btn" onClick={() => setPlaying(!playing)}>{playing ? "Pause" : "Play"}</button>
          <div className="seg h-seg" role="group" aria-label="Speed">{SPEEDS.map((s, i) => <button key={s.label} className={speed === i ? "on" : ""} onClick={() => setSpeed(i)}>{s.label}</button>)}</div>
          <button className={"btn h-btn" + (explain ? " lit" : "")} aria-pressed={explain} disabled={!explainable} onClick={() => { setExplain(!explain); setNote(null); }}>{explain ? "Explaining" : "Explain"}</button>
          <button className="btn h-btn" onClick={() => location.reload()}>Start over</button>
        </div>
      </header>
      <main className="st-stage">
        <div className="st-venue" aria-hidden="true">
          <div key={pulse} className={"st-glow" + (pulse ? " roar" : "")} />
          {c && <div className="st-board"><span><i style={{ background: BRAND.home.color }} />{BRAND.home.short} <b className="num">{c.home}</b></span><span className="num">{c.label}</span><span><b className="num">{c.away}</b> {BRAND.away.short} <i style={{ background: BRAND.away.color }} /></span></div>}
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
          {note ? <ExplainCard n={note} close={() => setNote(null)} /> : <>
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
          </div>
        </aside>
      </main>
      {talk && <TalkToUs close={() => setTalk(false)} version={null} />}
    </div>
  );
}

export function useToasts() {
  const [toasts, setToasts] = useState<{ id: number; text: string }[]>([]);
  const toast = (text: string) => { const id = Date.now() + Math.random(); setToasts(t => [...t, { id, text }]); setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 4200); };
  return { toasts, toast };
}

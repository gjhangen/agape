import { React, ReactDOM, useEffect, useRef, useState } from "./r";
import { boot, kernel } from "./kernel";
import { FanApp, FanView, Sheet } from "./fan";
import { OpsLens, OpsView, RunnerLens, RunnerView, StandLens, StandView } from "./roles";
import { BRAND, runnerName, venueTime } from "./parts";
import { BusinessLens, PlayerLens } from "./deals";
import { SetupLens, resetVenue } from "./setup";
import { worldOverride } from "./kernel";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";
import { LookPicker } from "./lookpicker";

const SPEEDS = [
  { id: "live", label: "1×", tps: 0.1 },
  { id: "x6", label: "6×", tps: 0.6 },
  { id: "x30", label: "30×", tps: 3 },
  { id: "x120", label: "120×", tps: 12 },
];
type Persp = "fan" | "runner" | "stand" | "player" | "business" | "ops" | "setup";
const PERSPECTIVES: [Persp, string][] = [["fan", "Fan in 104"], ["runner", "Runner"], ["stand", "Stand"], ["player", "Player"], ["business", "Business"], ["ops", "Operations"], ["setup", "Set up"]];

const START_MINUTE = 47; // a couple of minutes into the first quarter
const YOU_SEAT = "s104";

function App() {
  const [fan, setFan] = useState<FanView | null>(null);
  const [ops, setOps] = useState<OpsView | null>(null);
  const [runner, setRunner] = useState<RunnerView | null>(null);
  const [stand, setStand] = useState<StandView | null>(null);
  const [persp, setPersp] = useState<Persp>("fan");
  const [speed, setSpeed] = useState(1);
  const [playing, setPlaying] = useState(true);
  const [done, setDone] = useState(false);
  const [sheet, setSheet] = useState<Sheet>(null);
  const [toasts, setToasts] = useState<{ id: number; text: string }[]>([]);
  const [runnerId, setRunnerId] = useState<string | null>(null);
  const [standId, setStandId] = useState("grill-104");
  const [playerId, setPlayerId] = useState("hbk-1");
  const [bizId, setBizId] = useState("dock-pizza");
  const [player, setPlayer] = useState<any>(null);
  const [biz, setBiz] = useState<any>(null);
  const [parties, setParties] = useState<any>(null);
  const [wide, setWide] = useState(() => matchMedia("(min-width: 1080px)").matches);
  const acc = useRef(0);
  const st = useRef({ persp, runnerId, standId, wide, playerId, bizId });
  st.current = { persp, runnerId, standId, wide, playerId, bizId };

  const refresh = () => {
    const f = kernel.view("fan") as FanView;
    setFan(f);
    const s = st.current;
    if (s.persp === "ops" || (s.persp === "fan" && s.wide)) setOps(kernel.view("ops"));
    if (s.persp === "runner") {
      const id = s.runnerId ?? f.map.your_runner?.id ?? f.map.runners.find(r => r.busy)?.id ?? "runner0";
      if (!s.runnerId) setRunnerId(id);
      setRunner(kernel.view("runner", id));
    }
    if (s.persp === "stand") setStand(kernel.view("stand", s.standId));
    if (s.persp === "player") setPlayer(kernel.view("player", s.playerId));
    if (s.persp === "business") setBiz(kernel.view("business", s.bizId));
    if (s.persp === "player" || s.persp === "business") setParties(kernel.view("parties"));
  };
  const toast = (text: string) => { const id = Date.now() + Math.random(); setToasts(t => [...t, { id, text }]); setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 4200); };

  useEffect(() => {
    const m = matchMedia("(min-width: 1080px)");
    const on = () => setWide(m.matches);
    m.addEventListener("change", on);
    return () => m.removeEventListener("change", on);
  }, []);

  // the clock: step the kernel at the chosen speed, but never while the fan is deciding
  useEffect(() => {
    if (!playing || sheet || done) return;
    const i = setInterval(() => {
      acc.current += SPEEDS[speed].tps * 0.25;
      const n = Math.floor(acc.current);
      if (n < 1) return;
      acc.current -= n;
      if (!kernel.step(n)) setDone(true);
      refresh();
    }, 250);
    return () => clearInterval(i);
  }, [playing, speed, sheet, done]);
  useEffect(() => { if (fan) refresh(); }, [persp, runnerId, standId, wide, playerId, bizId]);
  // whoever you are looking as is yours to act for; the simulation stops answering for them
  useEffect(() => {
    const who = persp === "player" ? playerId : persp === "business" ? bizId : persp === "stand" ? standId : persp === "runner" ? runnerId : persp === "ops" ? "ops" : null;
    if (!who) return;
    kernel.act({ do: "drive", party: who });
    return () => { kernel.act({ do: "release", party: who }); };
  }, [persp, playerId, bizId, standId, runnerId]);

  const skip = (to: string) => {
    const reached = (f: FanView) =>
      to === "half" ? f.clock.phase === "halftime" : to === "q4" ? f.clock.label.startsWith("Q4") : to === "final" ? f.clock.phase === "postgame" : true;
    let f = kernel.view("fan") as FanView, guard = 0;
    while (!reached(f) && guard++ < 2000) { if (!kernel.step(3)) { setDone(true); break; } f = kernel.view("fan"); }
    refresh();
  };

  if (!fan) return <div className="gate"><h1>{BRAND.venue}</h1><p>Opening the doors…</p></div>;
  const c = fan.clock;
  const runners = (ops ?? (kernel.view("ops") as OpsView)).map.runners;
  return (
    <>
      <header className="harness">
        <div className="harness-in">
          <span className="eyebrow">Simulated night · {BRAND.venue}{worldOverride() && <> · your template <button className="link-inline small" onClick={resetVenue}>back to the pilot venue</button></>}</span>
          <div className="controls">
            <button className="btn h-btn" onClick={() => setPlaying(!playing)} aria-pressed={!playing}>{done ? "Night over" : playing ? "Pause" : "Play"}</button>
            <div className="seg h-seg" role="group" aria-label="Speed">
              {SPEEDS.map((s, i) => <button key={s.id} className={speed === i ? "on" : ""} title={i === 0 ? "Real time" : `${s.label} real time`} onClick={() => setSpeed(i)}>{s.label}</button>)}
            </div>
            <LookPicker />
            <select id="skip" className="h-btn" value="" onChange={e => { skip(e.target.value); e.target.value = ""; }} aria-label="Skip ahead">
              <option value="">Skip to…</option><option value="half">Halftime</option><option value="q4">4th quarter</option><option value="final">Final</option>
            </select>
          </div>
        </div>
        <nav className="persp" aria-label="Whose screen">
          <span className="eyebrow">See it as</span>
          {PERSPECTIVES.map(([id, label]) => (
            <button key={id} className={persp === id ? "on" : ""} aria-current={persp === id} onClick={() => setPersp(id)}>{label}</button>
          ))}
        </nav>
      </header>
      <main className={"stage " + persp}>
        {persp === "fan" && (
          <div className="fan-layout">
            <FanApp v={fan} refresh={refresh} sheet={sheet} setSheet={setSheet} toast={toast} />
            {wide && ops && (
              <aside className="world-pane" aria-label="The same moment, seen by operations">
                <p className="eyebrow">Same moment · operations</p>
                <OpsLens o={ops} compact />
              </aside>
            )}
          </div>
        )}
        {persp !== "fan" && persp !== "setup" && <p className="staff-clock"><b className="num">{venueTime(c.t)}</b><span>{c.phase === "halftime" ? "Halftime" : c.phase === "pregame" ? "Before tip-off" : c.phase === "postgame" ? "After the final" : c.label.split(" ")[0]}</span></p>}
        {persp === "runner" && runner && <RunnerLens r={runner} map={fan.map} pick={setRunnerId} runners={runners.map(r => r.id)} x={{ toast, refresh }} />}
        {persp === "stand" && stand && <StandLens s={stand} pick={setStandId} stands={fan.map.stands} x={{ toast, refresh }} />}
        {persp === "player" && player && parties && <PlayerLens p={player} players={parties.players} pick={setPlayerId} toast={toast} refresh={refresh} />}
        {persp === "business" && biz && parties && <BusinessLens b={biz} businesses={parties.businesses} pick={setBizId} toast={toast} refresh={refresh} />}
        {persp === "ops" && ops && <OpsLens o={ops} x={{ toast, refresh }} />}
        {persp === "setup" && <SetupLens toast={toast} />}
      </main>
      <div className="toasts" aria-live="polite">{toasts.map(t => <p key={t.id} className="toast">{t.text}</p>)}</div>
    </>
  );
}

(async () => {
  applyLook(savedLook()); applyMode(savedMode());
  const r = await boot(7, YOU_SEAT);
  if (!r.ok) throw new Error(r.error);
  kernel.step(START_MINUTE * 6);
  ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
})();

export { runnerName };

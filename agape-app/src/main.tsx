import { React, ReactDOM, useEffect, useRef, useState } from "./r";
import { boot, kernel } from "./kernel";
import { FanApp, FanView, Sheet } from "./fan";
import { OpsLens, OpsView, RunnerLens, RunnerView, StandLens, StandView } from "./roles";
import { BRAND, isConcert, runnerName, venueTime } from "./parts";
import { demoWorld } from "./shell";
import { BusinessLens, PlayerLens } from "./deals";
import { SetupLens, resetVenue } from "./setup";
import { setWorldOverride, worldOverride } from "./kernel";
import { applyLook, applyMode, savedLook, savedMode, setVenueColor } from "./themes";
import { LookPicker } from "./lookpicker";
import { TalkToUs } from "./talk";
import { Follow } from "./follow";
void LookPicker;

const SPEEDS = [
  { id: "live", label: "1×", tps: 0.1 },
  { id: "x6", label: "6×", tps: 0.6 },
  { id: "x30", label: "30×", tps: 3 },
  { id: "x120", label: "120×", tps: 12 },
];
type Persp = "fan" | "runner" | "stand" | "player" | "business" | "ops" | "setup";
const PERSPECTIVES: [Persp, string, string][] = [["fan", "Fan", "Section 104, on their phone"], ["runner", "Runner", "Carries orders to seats"], ["stand", "Stand", "Makes the orders"],
  ["ops", "Venue", "The whole night, live"], ["business", "Partner", "Sponsors and nearby businesses"], ["player", "Player", "Level 5 · requests and fans"], ["setup", "Studio", "Make it yours"]];

/** A walk through the night, one seat at a time. */
const TOUR_ALL: { persp: Persp; title: string; body: string }[] = [
  { persp: "fan", title: "A fan's phone", body: "This is Agape as a fan sees it in section 104. Tap any food or drink: the time shown is when it reaches the seat, estimated from how fast that stand and its runners are going right now. Add a few things to the cart and order." },
  { persp: "fan", title: "The game, live", body: "The line at the top of the phone is the game: tap it to open the score and everything that's happened. At higher levels, Play and People tabs add calls for points, your section's chat and the night's big plays as cards to collect." },
  { persp: "stand", title: "The stand", body: "Every order lands here the second it's placed: stations, the queue, the ready shelf. Start a shift and you're the one cooking; the simulation steps aside." },
  { persp: "runner", title: "The runner", body: "Runners are offered trips batched by section, with the route and the pay up front. Pass on one and it goes to the next runner for a little more, until someone takes it. Tips go entirely to them." },
  { persp: "ops", title: "The whole venue", body: "Operations sees every order, every runner and every dollar. The ledger balances to the cent all night, and one tap pauses service everywhere if something goes wrong." },
  { persp: "ops", title: "Where the money goes", body: "Further down the Venue screen, Money tonight: every dollar fans paid, split between the stands, runners, sales tax, the card networks and Agape. Agape plans to process the cards itself, so card processing becomes income rather than a cost. Every term is set per event in the Studio." },
  { persp: "business", title: "Businesses and sponsors", body: "Nearby businesses offer perks fans unlock with points, and sponsors can back players. They see how many fans came through their door, and what it brought in." },
  { persp: "player", title: "Players", body: "Optional, not in the launch set: players see their following and can take paid promotions from fans and businesses, only inside the rules their team and league set. A venue switches it on in the Studio." },
  { persp: "setup", title: "Make it yours", body: "Put your own venue, teams or acts, colors and menu on it, switch off what you don't want, choose how runners are paid, and run a whole night before you open. Nothing here touches anyone else." },
  { persp: "fan", title: "Speed it up", body: "Use the speed buttons to run the night at 6×, 30× or 120×, or skip ahead to the busiest moment or the end. Everything stays consistent: same night, every seat." },
];
/** On a concert night there are no players, so the tour skips them. */
const tourList = () => TOUR_ALL.filter(t => !(t.persp === "player" && isConcert()));

const START_MINUTE = 47; // a couple of minutes into the first quarter
const YOU_SEAT = "s104";

function App() {
  const [fan, setFan] = useState<FanView | null>(null);
  const [ops, setOps] = useState<OpsView | null>(null);
  const [runner, setRunner] = useState<RunnerView | null>(null);
  const [stand, setStand] = useState<StandView | null>(null);
  const [persp, setPersp] = useState<Persp>(() => { const p = new URLSearchParams(location.search).get("as"); return (["runner", "stand", "ops", "business", "player", "setup"].includes(p ?? "") ? p : "fan") as Persp; });
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
  const [tour, setTour] = useState<number | null>(null);
  const [follow, setFollow] = useState(false);
  const [talk, setTalk] = useState(false);
  const [intro, setIntro] = useState(() => { if (new URLSearchParams(location.search).get("as")) return false; try { return localStorage.getItem("agape.sim.intro") !== "seen"; } catch { return true; } });
  const closeIntro = () => { setIntro(false); try { localStorage.setItem("agape.sim.intro", "seen"); } catch { /* shown again next visit */ } };
  useEffect(() => { try { if (sessionStorage.getItem("agape.badlink")) { sessionStorage.removeItem("agape.badlink"); setTimeout(() => toast("That link didn't open a saved version, so this is the sample venue."), 600); } } catch { /* fine */ } }, []);
  const goTour = (i: number | null) => { setTour(i); if (i != null) setPersp(tourList()[i].persp); };
  // a link straight into the guided tour (the investor page sends people here)
  useEffect(() => { if (new URLSearchParams(location.search).get("tour")) { const t = setTimeout(() => { setIntro(false); setFollow(true); }, 400); return () => clearTimeout(t); } }, []);
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
      <header className="harness sim">
        <div className="harness-in">
          <span className="sim-brand"><b>agape</b> simulator<span className="sub"> · {BRAND.venue}{isConcert() && !sessionStorage.getItem("agape.shared") ? <> · concert night</> : null}{worldOverride() && !(isConcert() && !sessionStorage.getItem("agape.shared")) && <> · {sessionStorage.getItem("agape.shared") ? "a version made for you" : "your version"} <button className="link-inline small" onClick={() => { try { sessionStorage.removeItem("agape.shared"); } catch { /* fine */ } resetVenue(); }}>see the sample venue</button></>}</span></span>
          <div className="controls">
            <button className="btn h-btn" onClick={() => setPlaying(!playing)} aria-pressed={!playing}>{done ? "Night over" : playing ? "Pause" : "Play"}</button>
            <div className="seg h-seg" role="group" aria-label="Speed">
              {SPEEDS.map((s, i) => <button key={s.id} className={speed === i ? "on" : ""} title={i === 0 ? "Real time" : `${s.label} real time`} onClick={() => setSpeed(i)}>{s.label}</button>)}
            </div>
            <select id="skip" className="h-btn" value="" onChange={e => { if (e.target.value === "restart") { location.reload(); return; } skip(e.target.value); e.target.value = ""; }} aria-label="Skip ahead">
              <option value="">Skip to…</option><option value="half">{isConcert() ? "The changeover" : "Halftime"}</option><option value="q4">{isConcert() ? "The finale" : "4th quarter"}</option><option value="final">{isConcert() ? "The end of the show" : "Final"}</option><option value="restart">Start the night again</option>
            </select>
            <button className="btn h-btn" onClick={() => { setTour(null); setFollow(true); }}>Tour</button>
            <button className="btn h-btn primary" onClick={() => setTalk(true)}>Talk to us</button>
          </div>
        </div>
        <nav className="persp" aria-label="Whose screen">
          <span className="eyebrow">See it as</span>
          {PERSPECTIVES.filter(([id]) => !(id === "player" && isConcert())).map(([id, label, hint]) => (
            <button key={id} className={(persp === id ? "on" : "") + (id === "setup" ? " studio" : "")} aria-current={persp === id} title={hint} onClick={() => { setPersp(id); setTour(null); setFollow(false); }}><b>{label}</b><small>{hint}</small></button>
          ))}
        </nav>
      </header>
      {intro && !follow && (
        <section className="sim-intro" aria-label="About the simulator">
          <div>
            <span className="eyebrow">You're looking at a whole night, simulated</span>
            <h1>A full arena, running in your browser.</h1>
            <p>{isConcert() ? "A concert" : "A basketball game"} at {BRAND.venue}: {(worldOverride()?.pilot?.fans ?? 3000).toLocaleString()} simulated fans, {worldOverride()?.pilot?.runners ?? 40} runners, every kitchen and every dollar. Agape is real; the crowd and the {isConcert() ? "show" : "game"} are simulated so you can see all of it at once. Step into any seat above.</p>
            <details className="sim-more"><summary>More about what you're seeing</summary><p>Speed time up, switch features off, or put your own team on it in the Studio. Nothing you do here affects anyone else. Anything tagged with a package name, such as <span className="opt">Deals</span>, is built and working but beyond what Agape launches with; a venue switches it on if it wants it.</p></details>
          </div>
          <div className="sim-intro-acts">
            <button className="btn primary big" onClick={() => { closeIntro(); setFollow(true); }}>Take the 2-minute tour</button>
            <button className="btn big" onClick={() => { closeIntro(); setPersp("setup"); }}>{isConcert() ? "Make it your venue" : "Put my team on it"}</button>
            <button className="link-inline" onClick={closeIntro}>Just let me explore</button>
          </div>
        </section>
      )}
      {talk && <TalkToUs close={() => setTalk(false)} version={sessionStorage.getItem("agape.shared")} />}
      {tour != null && (
        <aside className="tour" role="dialog" aria-label="Tour">
          <span className="eyebrow">Tour · {tour + 1} of {tourList().length}</span>
          <b>{tourList()[tour].title}</b>
          <p>{tourList()[tour].body}</p>
          <div className="tour-acts">
            <button className="link-inline small" onClick={() => setTour(null)}>End tour</button>
            <span className="grow" />
            {tour > 0 && <button className="btn sm" onClick={() => goTour(tour - 1)}>Back</button>}
            {tour < tourList().length - 1 ? <button className="btn primary sm" onClick={() => goTour(tour + 1)}>Next</button> : <button className="btn primary sm" onClick={() => setTour(null)}>Done</button>}
          </div>
        </aside>
      )}
      {follow && <Follow v={fan} refresh={refresh} exit={() => setFollow(false)} screens={() => { setFollow(false); goTour(0); }}
        open={(as, ids) => { if (ids?.runner) setRunnerId(ids.runner); if (ids?.stand) setStandId(ids.stand); setPersp(as); setFollow(false); }} />}
      <main className={"stage " + persp} hidden={follow}>
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
        {persp !== "fan" && persp !== "setup" && <p className="staff-clock"><b className="num">{venueTime(c.t)}</b><span>{isConcert() ? c.label : c.phase === "halftime" ? "Halftime" : c.phase === "pregame" ? "Before tip-off" : c.phase === "postgame" ? "After the final" : c.label.split(" ")[0]}</span></p>}
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
  // a shared Studio version: open the simulator as that venue
  const shared = new URLSearchParams(location.search).get("v");
  if (shared && /^[0-9a-f]{12}$/.test(shared) && sessionStorage.getItem("agape.shared") !== shared) {
    try {
      const w = await fetch(`/api/studio/${shared}`).then(r => r.ok ? r.json() : null);
      if (w) { setWorldOverride(w); try { sessionStorage.setItem("agape.shared", shared); } catch { /* opens again next time */ } }
      else { setWorldOverride(null); try { sessionStorage.removeItem("agape.shared"); sessionStorage.setItem("agape.badlink", "1"); } catch { /* fine */ } }
    } catch { /* the sample venue, then */ }
  }
  // a concert night, when asked for
  if (!worldOverride() && new URLSearchParams(location.search).get("event") === "concert") setWorldOverride(demoWorld());
  setVenueColor(worldOverride()?.brand?.home?.color || null);
  const r = await boot(7, YOU_SEAT);
  if (!r.ok) throw new Error(r.error);
  kernel.step(START_MINUTE * 6);
  ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
})();

export { runnerName };

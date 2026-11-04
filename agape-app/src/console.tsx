// Agape Venue on a laptop: the venue's operations console, and a partner's home inside it.
// Which one is shown depends on the seat chosen: /try/venue/ or /try/partner/.
import { React, ReactDOM, useEffect, useState } from "./r";
import { boot, kernel, setWorldOverride } from "./kernel";
import { BRAND, Icon, money, levelFeatures } from "./parts";
import { OpsLens, OpsView } from "./roles";
import { BusinessLens } from "./deals";
import { SetupLens } from "./setup";
import { Beat, Shell, levelParam, useToasts } from "./shell";
import { Note } from "./explain";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";

declare const AGAPE_WORLD: any;
const PARTNER = location.pathname.includes("partner");
const BIZ = "dock-pizza";
const START_MINUTE = 46;
const LEVEL = PARTNER ? levelParam(4, 4) : levelParam(1, 1);
type Stage = "boot" | "signin" | "tonight" | "app";

const VENUE_NOTES: Note[] = [
  { sel: ".tiles", title: "The night at a glance", what: "Orders in the last five minutes, how long it takes to reach a seat, runners out, and whether the books balance.", who: "Venue operations", why: "One screen for the whole night: what used to take radios and runners shouting is in one place.", level: 1 },
  { sel: ".map-wrap", title: "The floor, live", what: "Every stand, every runner and where orders are coming from, as it happens.", who: "Operations", why: "Problems are seen before fans feel them: a slow stand, a busy section.", level: 1 },
  { sel: ".flows", title: "Every dollar, to the cent", what: "What fans paid, and where every dollar went: stands, runners, tax, card networks, Agape.", who: "Venue finance", why: "Trust. Agape earns its share and its processing margin in the open, and the books balance to zero every night.", level: 1 },
  { sel: ".ops-foot", title: "Stop service", what: "One switch pauses ordering everywhere and tells every fan why.", who: "Operations", why: "Safety first, and the venue stays in charge of its own building.", level: 1 },
];
const PARTNER_NOTES: Note[] = [
  { sel: ".role-head", title: "A partner's home", what: "A business near the venue, or a sponsor, sees its offers and what they brought in.", who: "Partners", why: "Partners pay to reach fans at the moment they care most, and see exactly what it returned.", level: 2 },
  { sel: ".tiles", title: "Results, not impressions", what: "Fans who came through the door, what they spent, what it cost.", who: "Partners", why: "Proven results are why partners renew: that's Agape's attention business.", level: 2 },
];

function Console({ stage, next, toast, o, b, parties, refresh }: { stage: Stage; next: (s: Stage) => void; toast: (t: string) => void; o: OpsView | null; b: any; parties: any; refresh: () => void }) {
  const [tab, setTab] = useState<"live" | "setup" | "team" | "settings">("live");
  if (stage === "boot") return <div className="ob ob-boot"><div className="ob-mark"><b>agape</b><small>{PARTNER ? "partners" : "venue"}</small></div></div>;
  if (stage === "signin") return (
    <form className="ob lp-signin" onSubmit={e => { e.preventDefault(); next("tonight"); }}>
      <div className="lp-card">
        <b className="ob-logo dark">agape {PARTNER ? "partners" : "venue"}</b>
        <h2>Sign in</h2>
        <label className="sf wide"><span>Work email</span><input defaultValue={PARTNER ? "owner@dockpizza.com" : "ops@harborarena.com"} /></label>
        <label className="sf wide"><span>Password</span><input type="password" defaultValue="••••••••••" /></label>
        <button className="btn ob-primary">Sign in</button>
        <small className="ob-fine">Protected with two-step sign-in.</small>
      </div>
    </form>
  );
  if (stage === "tonight") return (
    <div className="ob lp-signin">
      <div className="lp-card wide">
        <span className="eyebrow">{PARTNER ? "Dock Pizza · partner of " + BRAND.venue : BRAND.venue}</span>
        <h2>Tonight: {BRAND.home.name} vs {BRAND.away.name}</h2>
        {PARTNER ? <p className="ob-sub">Your offer for fans tonight: a slice after the game, unlocked with points. You'll see who comes through your door, live.</p>
          : <div className="sx-setup-grid">
            <div className="cw-check"><span className="cw-tick">✓</span><div><b>3,000 fans on Agape</b><small>Expected tonight, from ticket holders</small></div></div>
            <div className="cw-check"><span className="cw-tick">✓</span><div><b>40 runners on shift</b><small>Claimed through Agape Crew</small></div></div>
            <div className="cw-check"><span className="cw-tick">✓</span><div><b>9 stands open</b><small>Every menu confirmed</small></div></div>
            <div className="cw-check"><span className="cw-tick">✓</span><div><b>Last call at the 4th quarter</b><small>Set for this event</small></div></div>
          </div>}
        <button className="btn ob-primary" onClick={() => next("app")}>{PARTNER ? "Open my dashboard" : "Open tonight's console"}</button>
      </div>
    </div>
  );
  return (
    <div className="lp-app">
      <aside className="lp-nav">
        <b className="sx-logo">agape</b>
        {(PARTNER ? [["live", "chart", "Results"], ["settings", "store", "Business"]] : [["live", "chart", "Tonight"], ["setup", "list", "Events"], ["team", "people", "Team"], ["settings", "store", "Venue"]]).map(([id, ic, l]) =>
          <button key={id} className={tab === id ? "on" : ""} onClick={() => setTab(id as any)}><Icon name={ic as any} /><small>{l}</small></button>)}
      </aside>
      <div className="lp-main">
        {tab === "live" && !PARTNER && o && <OpsLens o={o} x={{ toast, refresh }} />}
        {tab === "live" && PARTNER && b && parties && <BusinessLens b={b} businesses={parties.businesses.filter((x: any) => x.id === BIZ)} pick={() => {}} toast={toast} refresh={refresh} />}
        {tab === "setup" && <SetupLens toast={toast} />}
        {tab === "team" && <div className="sx-report settings-list"><h2>Team</h2>
          {[["Alex Rivera", "Operations manager · admin"], ["Sam Okafor", "Food service director"], ["Priya Shah", "Finance · reports and payouts"], ["Stand managers", "9 stands, one manager each"]].map(([a, r]) =>
            <button key={a} className="set-row" onClick={() => toast(`${a}: ${r}.`)}><span className="me-av">{a[0]}</span><span className="grow"><b>{a}</b><small>{r}</small></span><Icon name="chev" size={18} /></button>)}
          <button className="btn ghost" onClick={() => toast("Invitation sent.")}>Invite someone</button></div>}
        {tab === "settings" && <div className="sx-report settings-list"><h2>{PARTNER ? "Your business" : "Your venue"}</h2>
          {(PARTNER ? [["Profile", "Dock Pizza, 4 min walk from the arena"], ["Billing", `Paid per redemption · ${money(300)} each`], ["Team", "2 people"], ["Help", "Talk to Agape"]]
            : [["Venue profile", `${BRAND.venue} · 18,000 seats`], ["Point of sale and ticketing", "Connected"], ["Payouts", "Weekly, itemized"], ["Agreement", "Agape's share and fees, as negotiated"], ["Help", "Your Agape account manager"]]).map(([a, r]) =>
            <button key={a} className="set-row" onClick={() => toast(`${a}: ${r}.`)}><span className="grow"><b>{a}</b><small>{r}</small></span><Icon name="chev" size={18} /></button>)}</div>}
      </div>
    </div>
  );
}

function beat(stage: Stage, o: OpsView | null): Beat {
  if (PARTNER) {
    if (stage !== "app") return { id: "p0", title: "A pizzeria four minutes from the arena", body: <>Dock Pizza partners with {BRAND.venue}. Fans unlock its offer with the points they earn ordering, and walk over after the game.</> };
    return { id: "p1", title: "Results, as they happen", body: <>Every unlock and every redemption shows up here with what it brought in. Partners pay for results, not for impressions.</> };
  }
  if (stage !== "app") return { id: "v0", title: `Operations at ${BRAND.venue}`, body: <>The venue runs its night on Agape: every stand, every runner and every dollar, on one screen. Tonight is set up before the doors open.</> };
  if (!o) return { id: "x", title: "", body: null };
  if ((o.problems?.length ?? 0) > 0) return { id: "v2", title: "A fan reported a problem", body: <>Problems land here with who's at fault. One tap refunds the fan, and the party at fault covers it.</> };
  return { id: "v1", title: "The whole night, live", body: <>Orders, seat waits and runners, the floor as it happens, and further down, every dollar to the cent. Switch to Events to set up the next night: teams, menu, terms and what fans get.</> };
}

function App() {
  const [stage, setStage] = useState<Stage>("boot");
  const [o, setO] = useState<OpsView | null>(null);
  const [b, setB] = useState<any>(null);
  const [parties, setParties] = useState<any>(null);
  const [ready, setReady] = useState(false);
  const { toasts, toast } = useToasts();
  const refresh = () => { if (PARTNER) { setB(kernel.view("business", BIZ)); } else setO(kernel.view("ops") as OpsView); };
  useEffect(() => { (async () => {
    setWorldOverride({ ...AGAPE_WORLD, features: levelFeatures(LEVEL) });
    const x = await boot(19, "s104"); if (!x.ok) return;
    kernel.step(START_MINUTE * 6);
    if (PARTNER) { kernel.act({ do: "drive", party: BIZ }); setParties(kernel.view("parties")); } else kernel.act({ do: "drive", party: "ops" });
    refresh(); setReady(true);
    setTimeout(() => setStage("signin"), 1500);
  })(); }, []);
  return <Shell tag={PARTNER ? "Partner · Agape Venue" : "Venue · Agape Venue"} device="laptop" ready={ready} onTick={refresh} toasts={toasts} notes={PARTNER ? PARTNER_NOTES : VENUE_NOTES} explainable={stage === "app"} level={LEVEL} minLevel={PARTNER ? 4 : 1}
    beat={beat(stage, o)} screen={<Console stage={stage} next={setStage} toast={toast} o={o} b={b} parties={parties} refresh={refresh} />} />;
}

applyLook(savedLook()); applyMode(savedMode());
ReactDOM.createRoot(document.getElementById("root")!).render(<App />);

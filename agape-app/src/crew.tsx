// Agape Crew: the runner's app, on the runner's phone, inside the demo's story of one runner's night.
import { React, ReactDOM, useEffect, useState } from "./r";
import { boot, kernel, setWorldOverride } from "./kernel";
import { Icon, money, runnerName, venueTime, countdown, BRAND } from "./parts";
import { RunnerView } from "./roles";
import { Beat, Shell, useToasts } from "./shell";
import { Note } from "./explain";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";

declare const AGAPE_WORLD: any;
const RUNNER = "runner4";
const START_MINUTE = 38;

type Stage = "boot" | "welcome" | "ready" | "shift" | "app";
type Tab = "home" | "earn" | "me";

const NOTES: Note[] = [
  { sel: ".cw-online", title: "Going online", what: "A runner works when they choose. Online puts them in line for trips near where they are.", who: "Independent runners", why: "Supply that flexes with demand: more runners come online for busy nights because the pay rises when they're needed.", level: 1 },
  { sel: ".cw-offer", title: "The offer", what: "Pay before saying yes: priced on the trip's walk and wait time and how much there is to carry. Tips are shown on top.", who: "Runners", why: "If a runner passes, the next one is offered a little more, until someone takes it. Agape pays out of its share and the delivery fee.", level: 1 },
  { sel: ".cw-steps", title: "One step at a time", what: "Pick up at the stand, then hand off seat by seat, with the row and seat and an ID check where needed.", who: "Runners on a trip", why: "Fewer mistakes, faster handoffs, honest arrival times for fans.", level: 1 },
  { sel: ".cw-id", title: "Checking ID", what: "Drinks with alcohol are handed over only after the runner checks ID at the seat.", who: "Runners certified for it", why: "The venue's license is protected, so venues can let Agape carry drinks.", level: 1 },
  { sel: ".cw-earn-big", title: "Earnings, live", what: "Trip pay and every tip, added the moment each delivery is made.", who: "Runners", why: "Runners see exactly what a night is worth, which is why they come back for the next one.", level: 1 },
  { sel: ".cw-cashout", title: "Cash out tonight", what: "Earnings go to a debit card the same night.", who: "Runners", why: "Agape processes the payments, so it can pay out instantly.", level: 1 },
];

const crewBase = { set: false, pay: 0, tips: 0, delivered: 0 };

function Crew({ r, stage, next, toast, refresh }: { r: RunnerView | null; stage: Stage; next: (s: Stage) => void; toast: (s: string) => void; refresh: () => void }) {
  const [tab, setTab] = useState<Tab>("home");
  const name = runnerName(RUNNER).replace(/ .\.$/, "");
  const act = (a: Record<string, unknown>, ok: string) => {
    if (a.do === "clock_in" && !crewBase.set && r) { crewBase.set = true; crewBase.pay = r.pay; crewBase.tips = r.tips; crewBase.delivered = r.delivered; }
    const x = kernel.act({ ...a, as: RUNNER }); toast(x.ok ? ok : x.error); refresh();
  };
  if (stage === "boot") return <div className="ob ob-boot crew"><div className="ob-mark"><b>agape</b><small>crew</small></div></div>;
  if (stage === "welcome") return (
    <div className="ob ob-welcome crew">
      <div className="ob-hero"><b className="ob-logo">agape crew</b><h1>Earn at live events, on your schedule.</h1><p>Pick your nights. See the pay before you take a trip. Keep every tip.</p></div>
      <div className="ob-acts"><button className="btn ob-primary" onClick={() => next("ready")}>Sign in</button></div>
    </div>
  );
  if (stage === "ready") return (
    <div className="ob ob-form">
      <h2>You're all set, {name}</h2>
      <p className="ob-sub">Everything a runner needs before their first night.</p>
      {[["ID and background check", "Approved"], ["Runner training", "Completed · 25 min"], ["ID-check certification", "Certified to hand over drinks"], ["Payouts", "Debit card •••• 1234"]].map(([a, b]) =>
        <div key={a} className="cw-check"><span className="cw-tick">✓</span><div><b>{a}</b><small>{b}</small></div></div>)}
      <button className="btn ob-primary" onClick={() => next("shift")}>Find a shift</button>
    </div>
  );
  if (stage === "shift") return (
    <div className="ob ob-form">
      <h2>Shifts near you</h2>
      <button className="cw-shift on" onClick={() => next("app")}>
        <div className="grow"><b>{BRAND.venue}</b><small>{BRAND.home.name} vs {BRAND.away.name} · tonight 6:45 to 10:30 PM</small><small className="cw-hot">Busy night expected · higher pay at halftime</small></div><span className="btn primary sm">Claim</span>
      </button>
      <div className="cw-shift"><div className="grow"><b>{BRAND.venue}</b><small>Concert · Saturday 7:00 PM to 11:30 PM</small></div><span className="sub">Opens Thursday</span></div>
    </div>
  );
  if (!r) return null;
  // her night starts when she first goes online
  const pay = crewBase.set ? r.pay - crewBase.pay : 0, tips = crewBase.set ? r.tips - crewBase.tips : 0, delivered = crewBase.set ? r.delivered - crewBase.delivered : 0;
  const t = r.trip;
  const on = !!r.on_shift;
  const now = r.clock.t;
  const tabs: [Tab, "home" | "cash" | "me", string][] = [["home", "home", "Trips"], ["earn", "cash", "Earnings"], ["me", "me", "Me"]];
  return (
    <div className="phone crew-app">
      <header className="cw-top">
        <button className={"cw-online" + (on ? " on" : "")} onClick={() => act({ do: on ? "clock_out" : "clock_in" }, on ? "You're offline. No new trips." : "You're online. Trips near you will show up here.")}>
          <i />{on ? "Online" : "Offline"}</button>
        <span className="cw-where">{BRAND.venue} · till 10:30 PM</span>
      </header>
      <div className="scope-body">
        {tab === "home" && <>
          {!on && <section className="cw-hero"><h2>Ready when you are</h2><p>Go online to get trips near you. Tonight is busy: most runners are out on trips right now.</p>
            <button className="btn primary big wide" onClick={() => act({ do: "clock_in" }, "You're online. Trips near you will show up here.")}>Go online</button></section>}
          {on && !t && <section className="cw-hero waiting"><span className="cw-radar" aria-hidden="true" /><h2>Finding your next trip</h2><p>Orders are batched by section from each stand's ready shelf. The nearest runner is offered first.</p></section>}
          {t && !t.accepted && (
            <section className="cw-offer" aria-live="assertive">
              <div className="row gap"><span className="eyebrow grow">New trip</span><span className="num cw-timer">{countdown(t.expires - now)}</span></div>
              <b className="cw-pay num">{money(t.pay)}</b>
              <small>{money(t.base ?? t.pay)} pay{t.tips ? ` + ${money(t.tips)} in tips, all yours` : ""}{t.raised > 0 ? ` · up ${money(t.raised)} since ${t.passes === 1 ? "a runner" : `${t.passes} runners`} passed` : ""}</small>
              <ol className="cw-route"><li><b>Pick up</b> {t.stand_name}</li>{t.sections.map((s, i) => <li key={i}><b>Hand off</b> section {s.slice(1)}</li>)}</ol>
              <p className="sub">About {t.minutes} min · {t.orders.length} order{t.orders.length === 1 ? "" : "s"}</p>
              <div className="deal-act two">
                <button className="btn ghost big" onClick={() => act({ do: "decline_trip" }, t.mode === "fixed" ? "Passed. It goes to someone else." : "Passed. It goes to the next runner, for a little more.")}>Pass</button>
                <button className="btn primary big" onClick={() => act({ do: "accept_trip" }, `It's yours. Head to ${t.stand_name}.`)}>Take it</button>
              </div>
            </section>
          )}
          {t && t.accepted && (
            <section className="cw-steps">
              <div className={"cw-step" + (t.picked ? " done" : " here")}>
                <span className="cw-n">1</span>
                <div className="grow"><b>Pick up at {t.stand_name}</b>
                  {t.orders.map(o => <small key={o.id}>#{o.id} · {o.items.map(i => `${i.qty}× ${i.name}`).join(", ")}</small>)}
                  {!t.picked && <button className="btn primary big wide" onClick={() => act({ do: "picked_up" }, "Got it. On to the seats.")}>I've picked it up</button>}</div>
              </div>
              {t.orders.map((o, i) => (
                <div key={o.id} className={"cw-step" + (o.stage === "Settled" || o.stage === "Delivered" ? " done" : t.picked ? " here" : "")}>
                  <span className="cw-n">{i + 2}</span>
                  <div className="grow"><b>Section {o.seat.slice(1)}{(o as any).spot ? `, ${(o as any).spot}` : ""}</b>
                    <small>{o.items.map(x => `${x.qty}× ${x.name}`).join(", ")}{o.breakdown?.tip ? ` · ${money(o.breakdown.tip)} tip` : ""}</small>
                    {(o as any).check_id && <span className="id-chip cw-id">Check ID · 21+</span>}
                    {t.picked && o.stage === "PickedUp" && <div className="row gap">
                      <button className="btn ghost sm" disabled={!o.handoff_ok_at || now < o.handoff_ok_at} onClick={() => act({ do: "unreachable", order: o.id }, "Marked. You're still paid for the trip.")}>Can't find them</button>
                      <button className="btn primary sm grow" onClick={() => act({ do: "delivered", order: o.id }, (o as any).check_id ? "ID checked, handed over." : "Handed over.")}>Handed over</button></div>}
                  </div>
                </div>
              ))}
            </section>
          )}
          <section className="block"><div className="block-head"><h2>Tonight</h2></div>
            <div className="pair"><div><span className="eyebrow">Earned</span><b className="big num">{money(pay + tips)}</b><small>{money(tips)} in tips</small></div>
              <div><span className="eyebrow">Delivered</span><b className="big num">{delivered}</b><small>{r.rating ? `★ ${r.rating.toFixed(1)}` : "no ratings yet"}</small></div></div></section>
        </>}
        {tab === "earn" && <>
          <section className="block cw-earn"><span className="eyebrow">Tonight at {BRAND.venue}</span><b className="cw-earn-big num">{money(pay + tips)}</b>
            <div className="lines"><div className="ledger"><span className="grow">Trip pay</span><span className="num">{money(pay)}</span></div><div className="ledger"><span className="grow">Tips, all yours</span><span className="num">{money(tips)}</span></div></div>
            <button className="btn primary block cw-cashout" disabled={pay + tips <= 0} onClick={() => toast(`${money(pay + tips)} is on its way to your debit card.`)}>Cash out now</button>
            <small className="sub">Arrives in minutes. Or it's paid out automatically every Monday.</small></section>
          <section className="block"><h2>This week</h2><div className="cw-bars">{[0, 0, 0, 0, 0, 0, pay + tips].map((v, i) => <div key={i}><i style={{ height: `${Math.max(4, Math.min(100, v / 40))}%` }} /><small>{"MTWTFSS"[i]}</small></div>)}</div></section>
        </>}
        {tab === "me" && <>
          <header className="me-head"><span className="me-av big">{name.slice(0, 1)}</span><div className="grow"><h2>{name}</h2><small>Runner since this season · {r.rating ? `★ ${r.rating.toFixed(1)}` : "new"}</small></div></header>
          <section className="block settings-list">
            {[["Documents", "ID and background check approved"], ["ID-check certification", "Valid for drinks with alcohol"], ["Payouts", "Debit card •••• 1234"], ["How I get around", "On foot"], ["Safety", "Share your trip, call venue security"], ["Notifications", "New trips, shift reminders"], ["Help", "Talk to Agape crew support"]].map(([a, b]) =>
              <button key={a} className="set-row" onClick={() => toast(a === "Safety" ? "Venue security is one tap away during a shift." : `${a}: ${b}.`)}><span className="grow"><b>{a}</b><small>{b}</small></span><Icon name="chev" size={18} /></button>)}
          </section>
        </>}
      </div>
      <nav className="space-bar icons labelled" aria-label="Spaces">
        {tabs.map(([id, icon, label]) => <button key={id} className={tab === id ? "on" : ""} aria-label={label} onClick={() => setTab(id)}><Icon name={icon} /><small>{label}</small></button>)}
      </nav>
    </div>
  );
}

function beat(stage: Stage, r: RunnerView | null): Beat {
  const name = runnerName(RUNNER).replace(/ .\.$/, "");
  if (stage === "boot" || stage === "welcome") return { id: "w", title: `Meet ${name}`, body: <>She runs for Agape at {BRAND.venue} on the nights she chooses. Runners are independent: they pick their nights, see the pay for every trip before they take it, and keep every tip.</> };
  if (stage === "ready") return { id: "r", title: "Before the first night", body: <>An ID and background check, a short training, and certification to check IDs, so she can carry drinks. Payouts go to her debit card.</> };
  if (stage === "shift") return { id: "s", title: "She picks her night", body: <>Shifts are posted per event. Busy nights say so, and offers rise when runners are scarce.</> };
  if (!r) return { id: "x", title: "", body: null };
  const t = r.trip;
  if (!r.on_shift) return { id: "off", title: "Going online", body: <>She's at the venue. Going online puts her in line for trips near where she's standing.</> };
  if (!t) return r.delivered > crewBase.delivered ? { id: "done", title: "Paid as she goes", body: <>Each delivery adds its trip pay and tip to her earnings at once. She can cash out tonight from Earnings, or keep going.</> }
    : { id: "wait", title: "Waiting for a trip", body: <>Orders reach the ready shelf as kitchens finish them, and the nearest free runner is offered the trip. On a busy night it's seconds.</> };
  if (!t.accepted) return { id: "offer", title: "An offer, with the pay up front", body: <>{money(t.pay)} for about {t.minutes} minutes: the pay is priced on the walk and wait, and the tip is shown on top. If she passes, it goes to the next runner for a little more.</> };
  if (!t.picked) return { id: "pick", title: `To ${t.stand_name}`, body: <>The order is on the ready shelf. She grabs it by its number and confirms the pickup.</> };
  return { id: "hand", title: "Seat by seat", body: <>Section, row and seat for each order. Drinks with alcohol need an ID check first. The fan's phone shows her coming.</> };
}

function App() {
  const [stage, setStage] = useState<Stage>("boot");
  const [r, setR] = useState<RunnerView | null>(null);
  const [ready, setReady] = useState(false);
  const { toasts, toast } = useToasts();
  const refresh = () => setR(kernel.view("runner", RUNNER) as RunnerView);
  useEffect(() => { (async () => {
    setWorldOverride({ ...AGAPE_WORLD });
    const x = await boot(13, "s104"); if (!x.ok) return;
    kernel.step(START_MINUTE * 6); refresh(); setReady(true);
    setTimeout(() => setStage("welcome"), 1700);
  })(); }, []);
  return <Shell tag="Runner · Agape Crew" device="phone" ready={ready} onTick={refresh} hold={false} toasts={toasts} notes={NOTES} explainable={stage === "app"}
    beat={beat(stage, r)} meta={r && stage === "app" ? <><span>Online runners can be offered trips; offline, the night goes on without them.</span></> : undefined}
    screen={<Crew r={r} stage={stage} next={setStage} toast={toast} refresh={refresh} />} />;
}

applyLook(savedLook()); applyMode(savedMode());
ReactDOM.createRoot(document.getElementById("root")!).render(<App />);

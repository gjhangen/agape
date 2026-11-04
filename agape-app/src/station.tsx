// Agape Station: the stand's screen, on a tablet at the counter, inside the story of one kitchen's night.
import { React, ReactDOM, useEffect, useState } from "./r";
import { boot, kernel, setWorldOverride } from "./kernel";
import { Icon, money, venueTime, BRAND, countdown } from "./parts";
import { StandView } from "./roles";
import { ItemArt } from "./Art";
import { Beat, Shell, useToasts } from "./shell";
import { Note } from "./explain";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";

declare const AGAPE_WORLD: any;
const STAND = "grill-104";
const START_MINUTE = 40;
type Stage = "boot" | "pin" | "setup" | "app";
type Tab = "orders" | "tonight" | "settings";

const NOTES: Note[] = [
  { sel: ".sx-open", title: "Open for orders", what: "When the stand opens, orders from every seat start landing here.", who: "The stand manager", why: "The venue's own kitchens make everything, so the venue keeps its food quality and most of every sale.", level: 1 },
  { sel: ".sx-col.making", title: "On the stations", what: "Each station shows one order and how long it should take. Bump it when it's ready.", who: "Cooks", why: "Prep times are measured, which is what makes every arrival time fans see honest.", level: 1 },
  { sel: ".sx-col.next", title: "Coming up", what: "Orders waiting for a free station, oldest first. Orders placed ahead for halftime wait until it's time to start them.", who: "The kitchen", why: "Fans can order ahead for the rush, so the kitchen's busiest minutes are spread out.", level: 1 },
  { sel: ".sx-col.shelf", title: "The ready shelf", what: "Finished orders waiting for a runner or for the fan to pick up.", who: "Runners and fans picking up", why: "Runners are offered trips the moment orders land here, batched by section.", level: 1 },
  { sel: ".sx-wait", title: "The wait, live", what: "How long a new order takes to reach a seat right now. Fans see the same number.", who: "The stand and every fan", why: "When it climbs, fans are steered to faster stands, so no stand drowns.", level: 1 },
];

function Station({ s, stage, next, toast, refresh }: { s: StandView | null; stage: Stage; next: (x: Stage) => void; toast: (t: string) => void; refresh: () => void }) {
  const [tab, setTab] = useState<Tab>("orders");
  const [pin, setPin] = useState("");
  const act = (a: Record<string, unknown>, ok: string) => { const x = kernel.act({ ...a, as: STAND }); toast(x.ok ? ok : x.error); refresh(); };
  if (stage === "boot") return <div className="ob ob-boot"><div className="ob-mark"><b>agape</b><small>station</small></div></div>;
  if (stage === "pin") return (
    <div className="ob sx-pin">
      <b className="ob-logo dark">agape station</b>
      <h2>{s?.name ?? "Grill 104"}</h2>
      <p className="ob-sub">Stand manager, enter your PIN</p>
      <div className="pin-dots">{[0, 1, 2, 3].map(i => <i key={i} className={pin.length > i ? "on" : ""} />)}</div>
      <div className="pin-pad">{["1", "2", "3", "4", "5", "6", "7", "8", "9", "", "0", "⌫"].map(k => <button key={k} disabled={!k} onClick={() => {
        const p = k === "⌫" ? pin.slice(0, -1) : (pin + k).slice(0, 4); setPin(p); if (p.length === 4) setTimeout(() => next("setup"), 250);
      }}>{k}</button>)}</div>
    </div>
  );
  if (stage === "setup") return (
    <div className="ob ob-form sx-setup">
      <h2>Tonight at {s?.name}</h2>
      <div className="sx-setup-grid">
        <div className="cw-check"><span className="cw-tick">✓</span><div><b>6 stations</b><small>Grills, fryers and the counter</small></div></div>
        <div className="cw-check"><span className="cw-tick">✓</span><div><b>Menu confirmed</b><small>Hot dogs, burgers, fries, drinks</small></div></div>
        <div className="cw-check"><span className="cw-tick">✓</span><div><b>8 on the crew</b><small>You can change who's on any time</small></div></div>
        <div className="cw-check"><span className="cw-tick">✓</span><div><b>Halftime forecast</b><small>The busiest 15 minutes of the night, starting about 8:20 PM</small></div></div>
      </div>
      <button className="btn ob-primary sx-open" onClick={() => { act({ do: "clock_in" }, "You're open. Orders are coming in."); next("app"); }}>Open for orders</button>
    </div>
  );
  if (!s) return null;
  const now = s.clock.t;
  const making = s.stations.filter(x => x.order);
  const ticket = (o: any, actions?: React.ReactNode, until?: number) => (
    <div key={o.id} className={"sx-ticket" + (until && until < now ? " late" : "")}>
      <div className="sx-t-head"><b className="num">#{o.id}</b><span>{o.delivery ? `Section ${o.seat.slice(1)}` : "Pickup"}</span>{until ? <span className="num sx-timer">{until < now ? "late" : countdown(until - now)}</span> : <span className="sub">{venueTime(o.placed)}</span>}</div>
      <ul>{o.items.map((i: any) => <li key={i.name}><ItemArt name={i.name} size={22} /><b>{i.qty}×</b> {i.name}</li>)}</ul>
      {actions}
    </div>
  );
  return (
    <div className="sx-app">
      <aside className="sx-nav">
        <b className="sx-logo">agape</b>
        {([["orders", "list", "Orders"], ["tonight", "chart", "Tonight"], ["settings", "store", "Stand"]] as [Tab, any, string][]).map(([id, ic, l]) =>
          <button key={id} className={tab === id ? "on" : ""} onClick={() => setTab(id)}><Icon name={ic} /><small>{l}</small></button>)}
      </aside>
      <div className="sx-main">
        <header className="sx-top">
          <div><b>{s.name}</b><small>{s.on_shift ? "Open" : "Closed"} · {venueTime(now)}</small></div>
          <div className="sx-wait"><span className="eyebrow">New orders reach seats in</span><b className="num">{Math.max(1, Math.round(s.wait / 60))} min</b></div>
          <button className={"sx-open" + (s.on_shift ? " on" : "")} onClick={() => act({ do: s.on_shift ? "clock_out" : "clock_in" }, s.on_shift ? "Closed for new orders." : "You're open.")}>{s.on_shift ? "Open" : "Closed"}</button>
        </header>
        {tab === "orders" && <div className="sx-cols">
          <section className="sx-col next"><h3>Coming up <span className="num">{s.queue_len}</span></h3>{s.queue.length ? s.queue.slice(0, 6).map(o => ticket(o)) : <p className="empty">Nothing waiting.</p>}</section>
          <section className="sx-col making"><h3>Making <span className="num">{making.length} of {s.stations.length}</span></h3>
            {making.length ? s.stations.map(x => x.order && ticket(x.order, <div className="row gap"><button className="btn ghost sm" onClick={() => act({ do: "cannot_make", order: x.order!.id }, "The fan was told at once and isn't charged.")}>Can't make</button><button className="btn primary sm grow" onClick={() => act({ do: "ready", order: x.order!.id }, `#${x.order!.id} is on the shelf.`)}>Ready</button></div>, x.until))
              : <p className="empty">Stations are free.</p>}</section>
          <section className="sx-col shelf"><h3>Ready shelf <span className="num">{s.ready.length + (s.pickups?.length ?? 0)}</span></h3>
            {s.ready.map(o => ticket(o, <small className="sub">A runner is being offered it</small>))}
            {(s.pickups ?? []).map(o => ticket(o, <button className="btn primary sm block" onClick={() => act({ do: "handed_over", order: o.id }, "Handed to the fan.")}>Fan picked it up</button>))}
            {!s.ready.length && !s.pickups?.length && <p className="empty">Shelf's clear.</p>}</section>
        </div>}
        {tab === "tonight" && <div className="sx-report">
          <div className="tiles three"><div className="tile"><span className="eyebrow">Items sold</span><b className="num">{s.sold}</b></div><div className="tile"><span className="eyebrow">Your sales, after fees</span><b className="num">{money(s.revenue)}</b></div><div className="tile"><span className="eyebrow">Fans' rating</span><b className="num">{s.rating ? `★ ${s.rating.toFixed(1)}` : "–"}</b><small>{s.ratings ?? 0} ratings</small></div></div>
          <p className="explain">Paid out after the event: every sale less Agape's share and card processing, itemized in your end-of-night report.</p>
        </div>}
        {tab === "settings" && <div className="sx-report settings-list">
          {[["Menu", "What's on tonight, prices and prep times"], ["Crew", "Who's working and their roles"], ["Stations", "6 stations"], ["Printer and screens", "Kitchen printer connected"], ["Payouts", "Harbor Arena food service"], ["Help", "Talk to Agape"]].map(([a, b]) =>
            <button key={a} className="set-row" onClick={() => toast(`${a}: ${b}.`)}><span className="grow"><b>{a}</b><small>{b}</small></span><Icon name="chev" size={18} /></button>)}
        </div>}
      </div>
    </div>
  );
}

function beat(stage: Stage, s: StandView | null): Beat {
  if (stage === "boot" || stage === "pin") return { id: "a", title: `${s?.name ?? "Grill 104"}, an hour before tip-off`, body: <>This is the stand's screen: a tablet at the counter, or a screen on the wall. The kitchen is the venue's own. Agape sends it orders from every seat and tells fans honestly how long they'll take.</> };
  if (stage === "setup") return { id: "b", title: "Ready for the night", body: <>Stations, menu and crew are set once and confirmed each night. The halftime forecast says when the rush will come.</> };
  if (!s) return { id: "x", title: "", body: null };
  const late = s.stations.some(x => x.order && x.until < s.clock.t);
  if (late) return { id: "late", title: "Something's running late", body: <>A ticket has passed its time. Bump it the moment it's done: the fan's arrival time updates, and if it ends up late they get money back without asking.</> };
  if (s.ready.length) return { id: "shelf", title: "Off the shelf in seconds", body: <>As soon as an order is bumped, the nearest runner is offered the trip. The kitchen never has to find anyone.</> };
  if (s.stations.some(x => x.order)) return { id: "make", title: "You're cooking", body: <>Each station holds one order with its time. Tap Ready when it's done, and it moves to the shelf for a runner.</> };
  return { id: "wait", title: "Orders land here", body: <>Orders from every seat in the arena arrive here as fans place them, oldest first.</> };
}

function App() {
  const [stage, setStage] = useState<Stage>("boot");
  const [s, setS] = useState<StandView | null>(null);
  const [ready, setReady] = useState(false);
  const { toasts, toast } = useToasts();
  const refresh = () => setS(kernel.view("stand", STAND) as StandView);
  useEffect(() => { (async () => {
    setWorldOverride({ ...AGAPE_WORLD });
    const x = await boot(17, "s104"); if (!x.ok) return;
    kernel.step(START_MINUTE * 6); refresh(); setReady(true);
    setTimeout(() => setStage("pin"), 1600);
  })(); }, []);
  return <Shell tag={`Kitchen · Agape Station`} device="tablet" ready={ready} onTick={refresh} toasts={toasts} notes={NOTES} explainable={stage === "app"}
    beat={beat(stage, s)} meta={s && stage === "app" ? <span>{BRAND.venue} · {s.queue_len} waiting · {s.ready.length} on the shelf</span> : undefined}
    screen={<Station s={s} stage={stage} next={setStage} toast={toast} refresh={refresh} />} />;
}

applyLook(savedLook()); applyMode(savedMode());
ReactDOM.createRoot(document.getElementById("root")!).render(<App />);

import { React } from "./r";
import { kernel } from "./kernel";
import { ItemArt } from "./Art";
import {
  BRAND, CommitmentCard, LedgerLine, MapView, MeasureTile, Offer, OFFER_COPY, Order, PlaceView,
  countdown, money, rel, runnerName, venueTime,
} from "./parts";

const placeName = (p: string, stands: MapView["stands"]) =>
  p.startsWith("s") && /^s\d+$/.test(p) ? `Section ${p.slice(1)}` : stands.find(s => s.id === p)?.name ?? p;

// ---------- Runner ----------

export type RunnerView = {
  clock: { t: number }; runner: { id: string; from: string; to: string; progress: number; busy: boolean };
  bag: Order[]; plan: { t: number; place: string; done: boolean }[]; delivered: number; pay: number; tips: number;
  human?: boolean; on_shift?: boolean; rating?: number | null; ratings?: number;
  trip?: { accepted: boolean; picked: boolean; stand: string; stand_name: string; expires: number; pay: number; minutes: number; sections: string[]; orders: (Order & { handoff_ok_at: number | null })[] } | null;
};

/** Acting from a staff screen: as this party, with the answer said back in a toast. */
export type StaffActs = { toast: (s: string) => void; refresh: () => void };
function staffAct(as: string, x: StaffActs | undefined, a: Record<string, unknown>, ok: string) {
  const r = kernel.act({ ...a, as });
  x?.toast(r.ok ? ok : r.error);
  x?.refresh();
}

function Shift({ on, human, as, x, who }: { on?: boolean; human?: boolean; as: string; x?: StaffActs; who: string }) {
  if (human && !on) return null; // acting as them from the simulation: no shift to start
  return on
    ? <div className="shift on"><span className="grow">On shift. {who} is yours to run.</span><button className="btn ghost sm" onClick={() => staffAct(as, x, { do: "clock_out" }, "Shift ended. The simulation covers for you.")}>End shift</button></div>
    : <div className="shift"><span className="grow">You're off shift, so the simulation is covering {who}.</span><button className="btn primary" onClick={() => staffAct(as, x, { do: "clock_in" }, "You're on. Nothing moves until you move it.")}>Start my shift</button></div>;
}

export function RunnerLens({ r, map, pick, runners, x }: { r: RunnerView; map: MapView; pick: (id: string) => void; runners: string[]; x?: StaffActs }) {
  const now = r.clock.t;
  const me = r.runner.id;
  const t = r.trip;
  const next = r.plan.find(p => !p.done);
  const m: MapView = { ...map, you: null, your_runner: r.runner };
  return (
    <div className="role">
      <header className="role-head">
        <span className="eyebrow">Signed in as</span>
        <select id="runner-pick" value={r.runner.id} onChange={e => pick(e.target.value)} aria-label="Runner">
          {runners.map(id => <option key={id} value={id}>{runnerName(id)}</option>)}
        </select>
      </header>
      {x && <Shift on={r.on_shift} human={r.human} as={me} x={x} who="this runner" />}
      {t && !t.accepted && (
        <section className="trip-offer" aria-live="polite">
          <span className="eyebrow">New trip · take it in {countdown(t.expires - now)}</span>
          <b className="trip-pay num">{money(t.pay)}</b>
          <p>Pick up at <b>{t.stand_name}</b>, then {t.sections.map(s => `section ${s.slice(1)}`).join(", ")}. About {t.minutes} min.</p>
          <div className="deal-act two">
            <button className="btn ghost big" onClick={() => staffAct(me, x, { do: "decline_trip" }, "Passed. It goes to someone else.")}>Pass</button>
            <button className="btn primary big" onClick={() => staffAct(me, x, { do: "accept_trip" }, `It's yours. Head to ${t.stand_name}.`)}>Take it</button>
          </div>
        </section>
      )}
      {t && t.accepted && !t.picked && (
        <section className="trip-offer on">
          <span className="eyebrow">Your trip · {t.orders.length} order{t.orders.length === 1 ? "" : "s"}</span>
          <p>Go to <b>{t.stand_name}</b> and collect from the ready shelf.</p>
          <button className="btn primary big wide" onClick={() => staffAct(me, x, { do: "picked_up" }, "Got it. On to the seats.")}>I've picked it up</button>
        </section>
      )}
      {t && t.picked && (
        <section className="trip-offer on">
          <span className="eyebrow">Hand off</span>
          {t.orders.filter(o => o.stage === "PickedUp").map(o => (
            <div key={o.id} className="handoff">
              <div className="grow"><b>Section {o.seat.slice(1)}</b><small>{o.items.map(i => `${i.qty}× ${i.name}`).join(", ")}{o.breakdown?.tip ? ` · ${money(o.breakdown.tip)} tip` : ""}</small></div>
              <button className="btn ghost sm" disabled={!o.handoff_ok_at || now < o.handoff_ok_at} title="After a few minutes at the seat"
                onClick={() => staffAct(me, x, { do: "unreachable", order: o.id }, "Marked. You're still paid for the trip.")}>Can't find them</button>
              <button className="btn primary sm" onClick={() => staffAct(me, x, { do: "delivered", order: o.id }, "Handed over.")}>Handed over</button>
            </div>
          ))}
        </section>
      )}
      <div className="role-grid">
        <section className="block">
          <div className="block-head"><h2>{r.runner.busy && next ? `Next: ${placeName(next.place, map.stands)}` : "Free"}</h2>
            {next && <span className="sub num">{venueTime(next.t)} · {rel(next.t, now)}</span>}</div>
          <div className="map-wrap"><PlaceView m={m} showRunners={false} heat={false} selfRunner /></div>
          <ol className="route-steps">
            {r.plan.map((p, i) => <li key={i} className={p.done ? "done" : p === next ? "here" : ""}>
              <time className="num">{venueTime(p.t)}</time><span>{i === 0 ? `Start at ${placeName(p.place, map.stands)}` : i === 1 ? `Pick up at ${placeName(p.place, map.stands)}` : `Hand off at ${placeName(p.place, map.stands)}`}</span></li>)}
          </ol>
        </section>
        <section className="block">
          <div className="block-head"><h2>Bag</h2><span className="sub">{r.bag.length} order{r.bag.length === 1 ? "" : "s"}</span></div>
          {r.bag.length === 0 && <p className="empty">Nothing in the bag. The dispatcher sends the next run when orders are ready.</p>}
          {r.bag.map(o => <CommitmentCard key={o.id} o={o} now={now} compact as="runner" />)}
          <div className="block-head"><h2>Your night</h2></div>
          <div className="tiles two">
            <MeasureTile label="Delivered" value={r.delivered} sub={r.rating ? `★ ${r.rating.toFixed(1)} from ${r.ratings} fan${r.ratings === 1 ? "" : "s"}` : "no ratings yet"} />
            <MeasureTile label="Earned" value={money(r.pay + r.tips)} sub={`${money(r.tips)} in tips`} />
          </div>
          <div className="lines"><LedgerLine label="Per-delivery pay" amount={r.pay} /><LedgerLine label="Tips, all yours" amount={r.tips} /><LedgerLine label="Total" amount={r.pay + r.tips} strong /></div>
        </section>
      </div>
    </div>
  );
}

// ---------- Stand ----------

export type StandView = {
  clock: { t: number }; stand: string; name: string; wait: number; paused_offers: boolean;
  stations: { until: number; late?: boolean; order: Order | null }[]; queue_len: number; queue: Order[]; ready: Order[]; sold: number; revenue: number;
  pickups?: Order[]; human?: boolean; on_shift?: boolean; rating?: number | null; ratings?: number;
};

export function StandLens({ s, pick, stands, x }: { s: StandView; pick: (id: string) => void; stands: MapView["stands"]; x?: StaffActs }) {
  const now = s.clock.t;
  const me = s.stand;
  const hands = !!s.human;
  return (
    <div className="role">
      <header className="role-head">
        <span className="eyebrow">Signed in at</span>
        <select id="stand-pick" value={s.stand} onChange={e => pick(e.target.value)} aria-label="Stand">
          {stands.map(x => <option key={x.id} value={x.id}>{x.name}</option>)}
        </select>
      </header>
      {x && <Shift on={s.on_shift} human={s.human} as={me} x={x} who="this kitchen" />}
      {s.paused_offers && <p className="banner warn">Your kitchen is past the 15-minute promise. Offers for your items are paused on fans' phones until you catch up.</p>}
      <div className="tiles four">
        <MeasureTile label="Kitchen wait" value={`${Math.round(s.wait / 60)} min`} tone={s.paused_offers ? "bad" : s.wait > 600 ? "warn" : "good"} />
        <MeasureTile label="In the queue" value={s.queue_len} />
        <MeasureTile label="Sold tonight" value={s.sold} sub={s.rating ? `★ ${s.rating.toFixed(1)} from ${s.ratings} fan${s.ratings === 1 ? "" : "s"}` : undefined} />
        <MeasureTile label="Your revenue" value={money(s.revenue)} sub="after Agape's share" />
      </div>
      <div className="role-grid">
        <section className="block">
          <div className="block-head"><h2>Stations</h2></div>
          <ul className="stations">
            {s.stations.map((st, i) => (
              <li key={i} className={(st.order ? "busy" : "") + (st.late ? " late" : "")}>
                <span className="eyebrow">Station {i + 1}</span>
                {st.order ? <><ItemArt name={st.order.items[0].name} size={32} /><span className="grow">{st.order.items.map(y => `${y.qty}× ${y.name}`).join(", ")}</span>
                  <span className="num">{st.late ? `${countdown(now - st.until)} over` : countdown(st.until - now)}</span>
                  {hands && <span className="station-acts">
                    <button className="btn ghost sm" onClick={() => staffAct(me, x, { do: "cannot_make", order: st.order!.id }, "The fan isn't charged and has been told.")}>Can't make it</button>
                    <button className="btn primary sm" onClick={() => staffAct(me, x, { do: "ready", order: st.order!.id }, st.order!.delivery ? "On the shelf for a runner." : "Ready at the counter.")}>Ready</button>
                  </span>}</> : <span className="grow sub">Open</span>}
              </li>
            ))}
          </ul>
          {hands && (s.pickups?.length ?? 0) > 0 && <>
            <div className="block-head"><h2>At the counter</h2><span className="sub">Fans collecting</span></div>
            {s.pickups!.map(o => <div key={o.id} className="handoff"><div className="grow"><b>Order {o.id}</b><small>{o.items.map(i => `${i.qty}× ${i.name}`).join(", ")}</small></div>
              <button className="btn primary sm" onClick={() => staffAct(me, x, { do: "handed_over", order: o.id }, "Handed over.")}>Handed over</button></div>)}
          </>}
          <div className="block-head"><h2>Ready shelf</h2><span className="sub">Waiting for a runner</span></div>
          {s.ready.length === 0 && <p className="empty">Shelf is clear.</p>}
          {s.ready.map(o => <CommitmentCard key={o.id} o={o} now={now} compact as="stand" />)}
        </section>
        <section className="block">
          <div className="block-head"><h2>Up next</h2></div>
          {s.queue.length === 0 && <p className="empty">No orders waiting.</p>}
          {s.queue.map(o => <CommitmentCard key={o.id} o={o} now={now} compact as="stand" />)}
        </section>
      </div>
    </div>
  );
}

// ---------- Operations ----------

const EXIT_COPY: Record<string, string> = { buyer_cancelled: "cancelled by the buyer", buyer_withdrew: "offer taken back", maker_declined: "declined", no_doer: "no one to do it" };

export type OpsView = {
  clock: { t: number; label: string }; orders_5m: number; stages: Record<string, number>; wait_p50: number | null; wait_p90: number | null;
  runners: { total: number; busy: number }; map: MapView; offers: Offer[]; alerts: { t: number; text: string }[];
  ledger: { agape: number; stands: number; runners: number; tax: number; conserved: boolean; txns: number };
  exchanges: { kinds: { kind: string; name: string; total: number; live: number; settled: number; closed: number; money: number; points: number }[];
    exits: { kind: string; exit: string; owner: string; count: number }[]; replays: boolean };
  attribution: { source: string; shown: number; accepted: number; settled: number; gross: number; spend: number }[]; signals: number;
  halt: { reason: string; since: number } | null; provenance: { crowd: string; simulated_fans: number; people: number };
  problems: { id: string; order: number; who: string; stand: string; words: string; note: string; t: number; fault: string; amount: number | null; state: { state: string; amount?: number; why?: string } }[];
};

const mins = (s: number | null) => (s == null ? "–" : `${Math.round(s / 60)} min`);

function ServiceControl({ o, x }: { o: OpsView; x: StaffActs }) {
  const [reason, setReason] = React.useState("");
  const [sure, setSure] = React.useState(false);
  if (o.halt) return (
    <section className="block halt-control on">
      <div className="block-head"><h2>Service is stopped</h2><span className="sub">since {venueTime(o.halt.since)}</span></div>
      <p>Fans see: “{o.halt.reason}”</p>
      <button className="btn primary big wide" onClick={() => staffAct("ops", x, { do: "resume" }, "Service is back on.")}>Resume service</button>
    </section>
  );
  return (
    <section className="block halt-control">
      <div className="block-head"><h2>Emergency stop</h2><span className="sub">every stand, every phone</span></div>
      <input value={reason} onChange={e => { setReason(e.target.value); setSure(false); }} maxLength={160} placeholder="What fans should be told" aria-label="Reason" />
      {!sure ? <button className="btn ghost wide" disabled={!reason.trim()} onClick={() => setSure(true)}>Stop service…</button>
        : <button className="btn danger wide" onClick={() => { staffAct("ops", x, { do: "halt", reason }, "Service stopped. Every fan has been told."); setSure(false); setReason(""); }}>Stop all service now</button>}
      <small className="sub">Nothing new can be ordered. Orders not yet being made are cancelled and nobody is charged; food already cooking or on its way still goes out.</small>
    </section>
  );
}

function Problems({ o, x }: { o: OpsView; x: StaffActs }) {
  if (o.problems.length === 0) return null;
  return (
    <section className="block">
      <div className="block-head"><h2>Problems reported</h2><span className="sub">{o.problems.filter(p => p.state.state === "open").length} to decide</span></div>
      {o.problems.map(p => (
        <div key={p.id} className="handoff">
          <div className="grow"><b>{p.words}</b><small>{p.who} · order {p.order} from {p.stand} · {money(p.amount ?? 0)} · on {p.fault}{p.note ? ` · “${p.note}”` : ""}</small></div>
          {p.state.state === "open" ? <>
            <button className="btn ghost sm" onClick={() => staffAct("ops", x, { do: "resolve", problem: p.id, refund: false, why: "Operations looked into it" }, "Declined. The fan sees why.")}>Decline</button>
            <button className="btn primary sm" onClick={() => staffAct("ops", x, { do: "resolve", problem: p.id, refund: true }, "Refunded.")}>Refund</button>
          </> : <span className={"tag " + (p.state.state === "refunded" ? "good" : "")}>{p.state.state === "refunded" ? `Refunded ${money(p.state.amount ?? 0)}` : "Declined"}</span>}
        </div>
      ))}
    </section>
  );
}

export function OpsLens({ o, compact, x }: { o: OpsView; compact?: boolean; x?: StaffActs }) {
  const inFlight = ["Accepted", "Preparing", "Ready", "Assigned", "PickedUp"].reduce((n, k) => n + (o.stages[k] ?? 0), 0);
  const stands = [...o.map.stands].sort((a, b) => b.wait - a.wait);
  const maxWait = Math.max(900, ...stands.map(s => s.wait));
  return (
    <div className={"role ops" + (compact ? " compact" : "")}>
      {!compact && o.provenance?.crowd === "simulated" && <p className="provenance">Includes a simulated crowd of {o.provenance.simulated_fans.toLocaleString()} fans{o.provenance.people ? ` and ${o.provenance.people} real ${o.provenance.people === 1 ? "person" : "people"}` : ""}. Money figures are the ledger's; arrival times are estimates.</p>}
      {compact && o.halt && <p className="banner warn">Service stopped: {o.halt.reason}</p>}
      {!compact && x && <div className="role-grid"><ServiceControl o={o} x={x} /><Problems o={o} x={x} /></div>}
      <div className={"tiles " + (compact ? "two" : "four")}>
        <MeasureTile label="Orders, last 5 min" value={o.orders_5m} sub={`${inFlight} in flight`} />
        <MeasureTile label="Seat waits" value={mins(o.wait_p50)} sub={`90% within ${mins(o.wait_p90)}`} tone={(o.wait_p90 ?? 0) > 1800 ? "bad" : (o.wait_p90 ?? 0) > 1200 ? "warn" : "good"} />
        <MeasureTile label="Runners out" value={`${o.runners.busy}/${o.runners.total}`} />
        <MeasureTile label="Ledger" value={o.ledger.conserved ? "Balanced" : "Out of balance"} sub={`${o.ledger.txns.toLocaleString()} postings`} tone={o.ledger.conserved ? "good" : "bad"} />
      </div>
      <div className="role-grid">
        <section className="block">
          <div className="block-head"><h2>Floor</h2><span className="sub">{o.signals.toLocaleString()} signals tonight</span></div>
          <div className="map-wrap"><PlaceView m={o.map} /></div>
          <ul className="waits">
            {stands.map(s => (
              <li key={s.id}><span className="grow">{s.name}</span>
                <span className="bar"><i className={s.busy ? "bad" : s.wait > 600 ? "warn" : "good"} style={{ width: `${(s.wait / maxWait) * 100}%` }} /><b className="mark" style={{ left: `${(900 / maxWait) * 100}%` }} title="15-minute promise" /></span>
                <span className="num">{Math.round(s.wait / 60)}m</span></li>
            ))}
          </ul>
        </section>
        {!compact && <section className="block">
          <div className="block-head"><h2>Offers</h2><span className="sub">What fans see, live</span></div>
          {o.offers.length === 0 && <p className="empty">No offers yet tonight.</p>}
          <ul className="offer-rows">
            {o.offers.map(f => (
              <li key={f.id}>
                <span className="grow"><b>{OFFER_COPY[f.source]?.title ?? f.name}</b><small>{f.guaranteed ? "Guaranteed" : "While it lasts"}{f.payer ? ` · ${BRAND.sponsors[f.payer] ?? f.payer}` : ""}</small></span>
                <span className={"tag " + (f.ended === "Paused" ? "warn" : f.ended ? "" : "good")}>{f.ended === "Paused" ? "Paused" : f.ended ? "Ended" : f.left != null ? `${f.left} left` : "Live"}</span>
              </li>
            ))}
          </ul>
          <div className="block-head"><h2>Who paid for what</h2></div>
          <div className="table-wrap"><table className="attr">
            <thead><tr><th>Source</th><th>Shown</th><th>Taken</th><th>Sold</th><th>Gross</th><th>Sponsor paid</th></tr></thead>
            <tbody>{o.attribution.map(a => <tr key={a.source}><td>{OFFER_COPY[a.source]?.title ?? (a.source === "menu" ? "Menu" : a.source)}</td>
              <td className="num">{a.shown || "–"}</td><td className="num">{a.accepted || "–"}</td><td className="num">{a.settled}</td><td className="num">{money(a.gross)}</td><td className="num">{a.spend ? money(a.spend) : "–"}</td></tr>)}</tbody>
          </table></div>
          <div className="block-head"><h2>Beyond food</h2><span className="sub">{o.exchanges.replays ? "Rebuilt from the log: matches" : "Log replay does not match"}</span></div>
          <div className="table-wrap"><table className="attr">
            <thead><tr><th>Kind</th><th>All</th><th>Open</th><th>Done</th><th>Ended</th><th>Settled</th></tr></thead>
            <tbody>{o.exchanges.kinds.filter(k => k.total > 0).map(k => <tr key={k.kind}><td>{k.name}</td><td className="num">{k.total}</td><td className="num">{k.live || "–"}</td><td className="num">{k.settled}</td>
              <td className="num">{k.closed || "–"}</td><td className="num">{[k.money ? money(k.money) : "", k.points ? `${k.points.toLocaleString()} pts` : ""].filter(Boolean).join(" + ") || "–"}</td></tr>)}</tbody>
          </table></div>
          {o.exchanges.exits.length > 0 && <ul className="earned">{o.exchanges.exits.map((e, i) => <li key={i}><span className="grow">{e.kind}: {EXIT_COPY[e.exit] ?? e.exit}</span><small>{e.owner === "nobody" ? "nobody's fault" : `on ${e.owner}`}</small><b className="num">{e.count}</b></li>)}</ul>}
          <div className="block-head"><h2>Money tonight</h2><span className="sub">Settled orders only</span></div>
          <div className="lines">
            <LedgerLine label="Stands" amount={o.ledger.stands} />
            <LedgerLine label="Runners" amount={o.ledger.runners} note="pay and tips" />
            <LedgerLine label="Sales tax held" amount={o.ledger.tax} />
            <LedgerLine label="Agape" amount={o.ledger.agape} strong note="after card fees and runner pay" />
          </div>
        </section>}
      </div>
    </div>
  );
}

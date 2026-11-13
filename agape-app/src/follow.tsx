// The simulator's story: one order followed through everyone's hands. The fan pays, the stand
// cooks it to the runner's clock, one runner carries it with other seats' orders, and every
// dollar lands where the terms say. Each panel is the same order as that person sees it.
import { React, useRef, useState } from "./r";
import { kernel, setup } from "./kernel";
import { FanView } from "./fan";
import { BRAND, isConcert, money, runnerName, venueTime } from "./parts";

type Stop = { t: number; at: string };
type Trip = { runner: string; orders: number; sections: string[]; pay: number; tips: number; stops: Stop[]; passes: number };
type Money = { charge: number; subtotal: number; discount: number; tax: number; fee: number; tip: number; stand: number; runner: number; runner_pay: number; networks: number; agape: number; sponsors: { payer: string; name: string | null; amount: number }[]; share_bps: number };
type Followed = {
  error?: string;
  order: { id: number; stage: string; items: { name: string; qty: number; price: number }[]; stand: string; stand_id: string; seat: string; charge: number };
  times: { placed: number; started: number | null; ready: number | null; picked: number | null; delivered: number | null; promised: number; eta: number; prep_s: number };
  stand: { ahead: number }; trip: Trip | null; money: Money | null; now: number;
};
export type Open = (as: "fan" | "stand" | "runner" | "ops" | "business", ids?: { runner?: string; stand?: string }) => void;

const secName = (s: string) => s === "sFL" ? "courtside" : s.replace(/^s/, "");
const mins = (ms: number) => Math.max(1, Math.round(ms / 60_000));

/** How far the order has got: 0 ordered, 1 cooking, 2 a runner has it on a trip, 3 picked up, 4 delivered, 5 paid out. */
function stageOf(f: Followed): number {
  if (f.order.stage === "Settled") return 5;
  if (f.times.delivered) return 4;
  if (f.times.picked) return 3;
  if (f.trip && f.times.ready) return 2;
  if (f.times.started) return 1;
  return 0;
}

const STEPS = ["Ordered", "Cooking", "Runner", "On the way", "Delivered", "Paid out"];

export function Follow({ v, refresh, open, exit, screens }: { v: FanView; refresh: () => void; open: Open; exit: () => void; screens: () => void }) {
  const [id, setId] = useState<number | null>(null);
  const [error, setError] = useState("");
  // the story moves only when the viewer says so; the panels underneath stay live
  const [shown, setShown] = useState(0);
  const best = useRef<Trip | null>(null);
  const read = (i: number): Followed => {
    const f = kernel.view("follow", String(i)) as Followed;
    if (f.trip && (!best.current || f.trip.orders >= best.current.orders)) best.current = f.trip;
    return f;
  };

  // what the fan orders: something hot from the nearest stand, with the default tip
  const can = v.items.filter(i => i.available && !i.adult);
  const pick = ["burger", "tenders", "tacos", "nachos"].map(k => can.find(i => i.id === k)).find(Boolean) ?? can.filter(i => i.category === "food").sort((a, b) => b.price - a.price)[0] ?? can.filter(i => i.category !== "merch")[0];
  const start = () => {
    if (!pick) { setError("Ordering is closed at this point in the night. Start the night again from the Skip menu."); return; }
    const r = kernel.act({ do: "order", thing: pick.id, qty: 2, delivery: true, tip: 300 });
    if (!r.ok) { setError("That order couldn't be placed just now. Try again in a moment."); return; }
    best.current = null; setShown(0); setId(Number(r.id)); refresh();
  };

  if (id == null) return (
    <section className="fw fw-start" aria-label="Follow one order">
      <span className="eyebrow">The 2-minute tour</span>
      <h1>Follow one order through everyone's hands.</h1>
      <p>A fan in section {secName(v.you.seat)} is about to order from their seat at {BRAND.venue}. You'll see that one order on four screens at once: the fan's phone, the stand that makes it, the runner who carries it, and where each dollar goes. The {isConcert() ? "show" : "game"} and the crowd are simulated; the system running them is the real one.</p>
      {error && <p className="fw-err" role="alert">{error}</p>}
      <div className="fw-acts">
        <button className="btn primary big" onClick={start}>{pick ? `Order 2 × ${pick.name} to the seat` : "Place the order"}</button>
        <button className="link-inline" onClick={exit}>Skip the tour and explore</button>
      </div>
    </section>
  );

  const f = read(id);
  if (f.error) return <section className="fw fw-start"><h1>That order is gone.</h1><div className="fw-acts"><button className="btn primary" onClick={() => setId(null)}>Start again</button></div></section>;
  const live = stageOf(f), s = Math.min(live, shown), t = f.times, trip = best.current, m = f.money;
  const next = () => {
    let g = 0;
    while (stageOf(read(id)) <= s && g++ < 900) { if (!kernel.step(1)) break; }
    setShown(s + 1); refresh();
  };
  const early = t.delivered ? Math.round((t.promised - t.delivered) / 60_000) : 0;
  const items = f.order.items.map(i => `${i.qty} × ${i.name}`).join(", ");
  const runner = trip ? runnerName(trip.runner) : null;
  const others = trip ? trip.orders - 1 : 0;

  const say: [string, string][] = [
    [`The fan paid ${money(f.order.charge)} and was promised it by ${venueTime(t.promised)}.`, "That time isn't a guess for show. It's worked out from this stand's queue and how busy the runners are right now, and the fan sees it before paying."],
    [`${f.order.stand} is making it.`, "While it cooks, a free runner is already being sent toward this stand, so the food doesn't sit getting cold on a shelf."],
    others > 0
      ? [`${runner} has it, on one trip with ${others === 1 ? "1 other order" : `${others} other orders`}.`, "This is what makes delivery to every seat pay for itself. Orders going the same way leave on the same trip, so one runner serves several seats in the time one order would take."]
      : [`${runner ?? "A runner"} has it and is taking it straight to the seat.`, "The floor is quiet at this moment, so it travels alone. When orders pile up, the ones going the same way leave on the same trip, and one runner serves several seats at once. The last step shows how much that changes."],
    [`${runner ?? "The runner"} picked it up and is walking it to section ${secName(f.order.seat)}.`, "The fan's phone shows the runner coming. Nobody leaves their seat, and nobody stands in a line."],
    [`Delivered at ${venueTime(t.delivered ?? f.now)}${early > 0 ? `, ${early} minute${early === 1 ? "" : "s"} ahead of the promise` : early < 0 ? `, ${-early} minute${early === -1 ? "" : "s"} after the promise` : ", right on the promise"}.`, early < 0 ? "When an order runs late, the fan is credited automatically. Nobody has to ask." : "Handed over at the seat. The money moves only once the order is delivered."],
    [`Every dollar of the ${money(f.order.charge)} has landed.`, "Agape is paid first and pays the stand, the runner and the tax. No one has to collect from anyone, and each of them sees their own share."],
  ];

  return (
    <section className="fw" aria-label="Follow one order">
      <header className="fw-head">
        <div>
          <span className="eyebrow">One order, every screen · step {s + 1} of 6</span>
          <h1 aria-live="polite">{say[s][0]}</h1>
          <p>{say[s][1]}</p>
        </div>
        <div className="fw-acts">
          {s < 5 ? <button className="btn primary big" onClick={next}>Next: {STEPS[s + 1].toLowerCase()}</button>
            : <><button className="btn primary big" onClick={screens}>Now see each screen</button><button className="btn big" onClick={() => { setShown(0); setId(null); }}>Follow another order</button></>}
          <button className="link-inline" onClick={exit}>{s < 5 ? "Leave the tour" : "Explore on your own"}</button>
        </div>
      </header>

      <ol className="fw-rail" aria-label="Where the order is">
        {STEPS.map((label, i) => {
          const at = [t.placed, t.started, t.ready, t.picked, t.delivered, s >= 5 ? t.delivered : null][i];
          return <li key={label} className={i < s ? "done" : i === s ? "now" : ""} aria-current={i === s ? "step" : undefined}><i aria-hidden="true">{i < s ? "✓" : i + 1}</i><b>{label}</b><small className="num">{i <= s && at ? venueTime(at) : ""}</small></li>;
        })}
      </ol>

      {s >= 5 && <Dial />}
      <div className="fw-cols">
        <article className={"fw-card" + (s === 0 || s === 3 || s === 4 ? " live" : "")}>
          <header><span className="eyebrow">The fan</span><b>Section {secName(f.order.seat)}</b></header>
          <p className="fw-big">{items}</p>
          <dl>
            <div><dt>Paid</dt><dd className="num">{money(f.order.charge)}</dd></div>
            <div><dt>Promised by</dt><dd className="num">{venueTime(t.promised)}</dd></div>
            <div><dt>{t.delivered ? "Delivered" : "Now"}</dt><dd>{t.delivered ? venueTime(t.delivered) : live >= 3 ? `${runner} is on the way` : t.ready ? "Ready, waiting for its runner" : live >= 1 ? "Being made" : "Order received"}</dd></div>
          </dl>
          <button className="link-inline small" onClick={() => open("fan")}>Open the fan's phone</button>
        </article>

        <article className={"fw-card" + (s === 1 ? " live" : "") + (s < 1 ? " wait" : "")}>
          <header><span className="eyebrow">The stand</span><b>{f.order.stand}</b></header>
          <p className="fw-big">{t.ready ? `Ready at ${venueTime(t.ready)}` : t.started ? `On the grill since ${venueTime(t.started)}` : f.stand.ahead ? `${f.stand.ahead} order${f.stand.ahead === 1 ? "" : "s"} ahead of it` : "Next up"}</p>
          <dl>
            <div><dt>Takes</dt><dd className="num">about {Math.max(1, Math.round(t.prep_s / 60))} min</dd></div>
            <div><dt>Queued for</dt><dd className="num">{t.started ? `${mins(t.started - t.placed)} min` : "…"}</dd></div>
            <div><dt>Keeps</dt><dd className="num">{m ? money(m.stand) : "…"}</dd></div>
          </dl>
          <button className="link-inline small" onClick={() => open("stand", { stand: f.order.stand_id })}>Open this stand's screen</button>
        </article>

        <article className={"fw-card" + (s === 2 || s === 3 ? " live" : "") + (!trip ? " wait" : "")}>
          <header><span className="eyebrow">The runner</span><b>{runner ?? "Not assigned yet"}</b></header>
          {trip ? <>
            <p className="fw-big"><span className="fw-n num">{trip.orders}</span> order{trip.orders === 1 ? "" : "s"} on one trip</p>
            <dl>
              <div><dt>Going to</dt><dd>section{trip.sections.length === 1 ? "" : "s"} {trip.sections.map(secName).join(", ")}</dd></div>
              <div><dt>Trip pays</dt><dd className="num">{money(trip.pay)} + {money(trip.tips)} tips</dd></div>
              {trip.passes > 0 && <div><dt>Offer</dt><dd>rose after {trip.passes} runner{trip.passes === 1 ? "" : "s"} passed</dd></div>}
            </dl>
          </> : <>
            <p className="fw-big">Waiting for the order to be nearly ready</p>
            <p className="fw-note">It's then offered to a free runner, together with other orders headed the same way.</p>
          </>}
          <button className="link-inline small" disabled={!trip} onClick={() => trip && open("runner", { runner: trip.runner })}>Open this runner's screen</button>
        </article>

        <article className={"fw-card" + (s === 5 ? " live" : "") + (s < 4 ? " wait" : "")}>
          <header><span className="eyebrow">The money</span><b>{live >= 5 ? "Paid out" : "Held until it's delivered"}</b></header>
          {m && <>
            <ul className="fw-split" aria-label={`How the ${money(m.charge)} is split`}>
              {([["Stand", m.stand, "stand"], ["Runner (pay and tip)", m.runner, "runner"], ["Agape", m.agape, "agape"], ["Sales tax", m.tax, "tax"], ["Card networks", m.networks, "net"]] as [string, number, string][]).map(([label, amt, k]) =>
                <li key={k} className={k}><span>{label}</span><i style={{ width: `${Math.max(2, Math.round(100 * Math.max(0, amt) / Math.max(1, m.charge + m.discount)))}%` }} aria-hidden="true" /><b className="num">{money(amt)}</b></li>)}
            </ul>
            {m.sponsors.map(sp => <p key={sp.payer} className="fw-note">{sp.name ?? "A sponsor"} paid {money(sp.amount)} of this order for the fan.</p>)}
            <p className="fw-note">These are the sample terms: Agape keeps {(m.share_bps / 100).toFixed(0)}% of the sale and the delivery fee, and pays the runner from it. Each venue sets its own.</p>
          </>}
          <button className="link-inline small" onClick={() => open("ops")}>Open the venue's screen</button>
        </article>
      </div>
    </section>
  );
}

type Night = { runners: number; per: number; peak: number; batch: number; wait: number; late: number; orders: number; hourly: number };

/** The staffing dial: the same simulated night run with fewer runners, to show what batching buys and what it costs in waiting. */
function Dial() {
  const [rows, setRows] = useState<Night[] | null>(null);
  const [busy, setBusy] = useState(false);
  const base = setup.runners();
  const run = () => {
    setBusy(true);
    setTimeout(() => {
      const out: Night[] = [];
      for (const n of [base, Math.round(base * 0.6), Math.round(base * 0.4)]) {
        const r = setup.staffed(n);
        if (r?.ok) out.push({ runners: n, per: r.report.per_runner_hour, peak: r.report.peak_per_runner_hour, batch: r.report.batch_avg, wait: r.report.wait_p50_s, late: r.report.late, orders: r.report.orders, hourly: r.report.runner_hourly });
      }
      setRows(out); setBusy(false);
    }, 40);
  };
  return (
    <section className="fw-dial" aria-label="The staffing dial">
      <div>
        <span className="eyebrow">The question everyone asks</span>
        <h2>How many deliveries can one runner make in an hour?</h2>
        <p>It depends on how many runners are on the floor. With fewer runners, more orders leave on each trip, each runner delivers more and earns more, and fans wait longer. Run this same night three ways and see the trade.</p>
        {!rows && <button className="btn primary" onClick={run} disabled={busy}>{busy ? "Running three whole nights…" : "Run the night three ways"}</button>}
      </div>
      {rows && <div className="fw-dial-out">
        <table>
          <thead><tr><th scope="col">Runners on the floor</th><th scope="col">Deliveries per runner, per hour</th><th scope="col">In the busiest hour</th><th scope="col">Orders per trip</th><th scope="col">Typical wait</th><th scope="col">Late</th><th scope="col">Runner earns per hour</th></tr></thead>
          <tbody>{rows.map(r => <tr key={r.runners}>
            <th scope="row" className="num">{r.runners}</th><td className="num"><b>{r.per.toFixed(1)}</b></td><td className="num">{r.peak.toFixed(1)}</td><td className="num">{r.batch.toFixed(1)}</td>
            <td className="num">{Math.round(r.wait / 60)} min</td><td className="num">{Math.round(100 * r.late / Math.max(1, r.orders))}%</td><td className="num">{money(r.hourly)}</td></tr>)}</tbody>
        </table>
        <p className="fw-note">Each row is a full simulated night at {BRAND.venue}, with the same crowd and the same {isConcert() ? "show" : "game"}, run just now in your browser. These are simulated results, not measurements from a real venue. The first real nights will replace them.</p>
      </div>}
    </section>
  );
}

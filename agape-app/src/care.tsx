// Agape HQ's care desk: the policy for making things right, every problem and who paid for it, and
// the fans whose records call for a look. Agape's team only: nothing here is shown to fans or runners.
import { React, useState } from "./r";
import { kernel } from "./kernel";
import { money, MeasureTile, venueTime } from "./parts";

type Cover = { part: "items" | "tax" | "fee" | "tip"; from: "fault" | "stand" | "runner" | "agape" | "tax"; bps: number };
type Rule = { words: string; fault: string; covers: Cover[]; photo: boolean; auto: boolean; form?: "credit" | "refund" | null };
export type Policy = { form: "credit" | "refund"; rules: Record<string, Rule>; auto_per_fan: number; auto_per_stand: number; handoff_radius_m: number; tip_bps: number; tip_min: number; proof_after_orders: number; photo_at_bps: number; pin_at_bps: number };
type Rec = { id: string; name: string; orders: number; spent: number; claims: number; claim_bps: number; remedied: number; earned: number; cost: number; net: number; ratings_given: number; avg_given: number | null; proof: "none" | "photo" | "pin"; proof_set: boolean };
export type Pay = { mode: "offers" | "fixed" | "hourly"; hourly_wage: number; runner_pay: number; offer_hourly: number; free_delivery_over: number; delivery_fee: number };
export type CareView = { policy: Policy; pay: Pay; settled: number; claims: number; claim_bps: number; open: number; credit: number; refund: number; paid_by: Record<string, number>;
  problems: { id: string; order: number; fan: string; who: string; stand: string; reason: string; words: string; photo: boolean; held: string | null; state: { state: string; amount?: number; form?: string; why?: string }; t: number; fault: string; handoff_m: number | null; paid_by: { who: string; amount: number }[] }[];
  fans: Rec[] };

const PARTS: [Cover["part"], string][] = [["items", "Items"], ["tax", "Tax"], ["fee", "Delivery fee"], ["tip", "Tip"]];
const FROM: [Cover["from"] | "", string][] = [["", "Not returned"], ["fault", "Whoever's at fault"], ["stand", "The stand"], ["runner", "The runner"], ["agape", "Agape"], ["tax", "Tax collected"]];
const pct = (bps: number) => `${Math.round(bps / 100)}%`;

export function CareDesk({ c, toast, refresh }: { c: CareView; toast: (t: string) => void; refresh: () => void }) {
  const [tab, setTab] = useState<"queue" | "fans">("queue");
  const act = (a: Record<string, unknown>, ok: string) => { const r = kernel.act({ ...a, as: "hq" }); toast(r.ok ? ok : r.error); refresh(); };
  const agapePaid = c.paid_by["Agape"] ?? 0;
  return (
    <div className="role care">
      <div className="tiles four">
        <MeasureTile label="Claims tonight" value={c.claims} sub={`${pct(c.claim_bps)} of ${c.settled} orders`} tone={c.claim_bps > 300 ? "warn" : "good"} />
        <MeasureTile label="Waiting on us" value={c.open} sub="held for a person" tone={c.open ? "warn" : "good"} />
        <MeasureTile label="Made right" value={money(c.credit + c.refund)} sub={`${money(c.credit)} as credit · ${money(c.refund)} refunded`} />
        <MeasureTile label="Agape paid" value={money(agapePaid)} sub={Object.entries(c.paid_by).filter(([k]) => k !== "Agape").map(([k, v]) => `${k} ${money(v)}`).join(" · ") || "stands and runners paid the rest"} />
      </div>
      <div className="seg" role="group" aria-label="Care">{([["queue", "Problems"], ["fans", "Fans to watch"]] as const).map(([k, l]) => <button key={k} className={tab === k ? "on" : ""} onClick={() => setTab(k)}>{l}</button>)}</div>
      {tab === "queue" && <Queue c={c} act={act} />}
      {tab === "fans" && <Fans c={c} act={act} />}
    </div>
  );
}

function Queue({ c, act }: { c: CareView; act: (a: Record<string, unknown>, ok: string) => void }) {
  if (!c.problems.length) return <p className="empty">No problems reported tonight. When a fan reports one, it lands here with Agape's evidence beside it.</p>;
  return <section className="block">
    {c.problems.map(p => <div key={p.id} className="care-row">
      <div className="grow">
        <b>{p.words}</b> <small className="sub">{p.who} · order {p.order} from {p.stand} · {venueTime(p.t)}</small>
        <small className="care-evidence">{p.photo ? "Photo attached" : "No photo"}{p.handoff_m != null ? ` · handed over ${p.handoff_m} m from the seat` : ""} · on {p.fault}{p.held ? ` · held: ${p.held}` : ""}</small>
        {p.paid_by.length > 0 && <small className="sub">Paid by {p.paid_by.map(x => `${x.who} ${money(x.amount)}`).join(", ")}</small>}
      </div>
      {p.state.state === "open" ? <div className="care-acts">
        <button className="btn ghost sm" onClick={() => act({ do: "resolve", problem: p.id, remedy: false, why: "We checked the trip and the photos" }, "Declined. The fan sees why.")}>Decline</button>
        <button className="btn ghost sm" onClick={() => act({ do: "resolve", problem: p.id, remedy: true, fault: "runner" }, "Made right, at the runner's cost.")}>Runner's fault</button>
        <button className="btn ghost sm" onClick={() => act({ do: "resolve", problem: p.id, remedy: true, fault: "stand" }, "Made right, at the stand's cost.")}>Stand's fault</button>
        <button className="btn primary sm" onClick={() => act({ do: "resolve", problem: p.id, remedy: true }, "Made right, as the policy says.")}>Make it right</button>
        <button className="btn ghost sm" onClick={() => act({ do: "resolve", problem: p.id, remedy: true, form: "refund" }, "Refunded to their card.")}>Refund instead</button>
      </div> : <span className={"tag " + (p.state.state === "remedied" ? "good" : "")}>{p.state.state === "remedied" ? `${p.state.form === "refund" ? "Refunded" : "Credit"} ${money(p.state.amount ?? 0)}` : "Declined"}</span>}
    </div>)}
  </section>;
}

function Fans({ c, act }: { c: CareView; act: (a: Record<string, unknown>, ok: string) => void }) {
  if (!c.fans.length) return <p className="empty">Fans show up here once they've reported a problem.</p>;
  return <section className="block">
    <p className="explain">Every fan who has reported a problem, the ones costing Agape money first. Proof at the handoff is asked for automatically once a fan has {c.policy.proof_after_orders} orders and their claims reach {pct(c.policy.photo_at_bps)} (a photo) or {pct(c.policy.pin_at_bps)} (their PIN). Nobody outside Agape sees any of this.</p>
    <div className="table-wrap"><table className="attr">
      <thead><tr><th>Fan</th><th>Orders</th><th>Claims</th><th>Made right</th><th>Agape net</th><th>Stars given</th><th>Proof at handoff</th></tr></thead>
      <tbody>{c.fans.map(f => <tr key={f.id}>
        <td><b>{f.name}</b></td><td className="num">{f.orders}</td><td className="num">{f.claims} · {pct(f.claim_bps)}</td><td className="num">{money(f.remedied)}</td>
        <td className={"num " + (f.net < 0 ? "bad" : "")}>{money(f.net)}</td><td className="num">{f.avg_given != null ? `${f.avg_given.toFixed(1)} (${f.ratings_given})` : "–"}</td>
        <td><select value={f.proof_set ? f.proof : "auto"} aria-label={`Proof for ${f.name}`} onChange={e => act({ do: "set_proof", fan: f.id, proof: e.target.value === "auto" ? "" : e.target.value }, "Saved. Their runner is told what to collect, not why.")}>
          <option value="auto">Automatic ({f.proof === "none" ? "none" : f.proof})</option><option value="none">None</option><option value="photo">Runner takes a photo</option><option value="pin">Fan gives a PIN</option></select></td>
      </tr>)}</tbody>
    </table></div>
  </section>;
}

/** Agape's policies, in one place: runner pay, delivery, and making things right. */
export function Policies({ c, toast, refresh }: { c: CareView; toast: (t: string) => void; refresh: () => void }) {
  const act = (a: Record<string, unknown>, ok: string) => { const r = kernel.act({ ...a, as: "hq" }); toast(r.ok ? ok : r.error); refresh(); };
  return <div className="role care">
    <PayEditor p={c.pay} act={act} />
    <PolicyEditor p={c.policy} act={act} />
  </div>;
}

function PayEditor({ p, act }: { p: Pay; act: (a: Record<string, unknown>, ok: string) => void }) {
  const [d, setD] = useState<Pay>(p);
  const dollars = (c: number) => (c / 100).toFixed(2);
  const cents = (v: string) => Math.round((Number(v.replace(/[^\d.]/g, "")) || 0) * 100);
  const changed = JSON.stringify(d) !== JSON.stringify(p);
  return <section className="block care-policy">
    <div className="block-head"><h2>Runner pay</h2><span className="sub">For every venue. Tips always go to the runner on top.</span></div>
    <div className="seg">{([["offers", "Offers per trip"], ["fixed", "Fixed per delivery"], ["hourly", "By the hour"]] as const).map(([k, l]) => <button key={k} className={d.mode === k ? "on" : ""} onClick={() => setD({ ...d, mode: k })}>{l}</button>)}</div>
    <p className="explain">{d.mode === "offers" ? "Each trip is offered with its pay, priced on its walk and wait; if a runner passes, the next offer is a little higher. Runners choose their trips." : d.mode === "fixed" ? "The same pay for every delivery, whoever takes it." : "A wage for every hour online, paid by the minute, whatever the trips. Runners take the trips they're given; the more efficient the routes, the less each delivery costs Agape."}</p>
    <div className="care-grid">
      {d.mode === "hourly" && <label className="sf"><span>Per hour online $</span><input inputMode="decimal" defaultValue={dollars(d.hourly_wage)} onChange={e => setD({ ...d, hourly_wage: cents(e.target.value) })} /></label>}
      {d.mode === "fixed" && <label className="sf"><span>Per delivery $</span><input inputMode="decimal" defaultValue={dollars(d.runner_pay)} onChange={e => setD({ ...d, runner_pay: cents(e.target.value) })} /></label>}
      {d.mode === "offers" && <label className="sf"><span>Offers priced at, per hour of trip $</span><input inputMode="decimal" defaultValue={dollars(d.offer_hourly)} onChange={e => setD({ ...d, offer_hourly: cents(e.target.value) })} /></label>}
    </div>
    <div className="block-head"><h2>Delivery</h2><span className="sub">The fee is {money(d.delivery_fee)} unless an order qualifies for free delivery.</span></div>
    <div className="care-grid">
      <label className="check"><input type="checkbox" checked={d.free_delivery_over > 0} onChange={e => setD({ ...d, free_delivery_over: e.target.checked ? 2500 : 0 })} /> Free delivery over an amount</label>
      {d.free_delivery_over > 0 && <label className="sf"><span>Free when the items come to at least $</span><input inputMode="decimal" defaultValue={dollars(d.free_delivery_over)} onChange={e => setD({ ...d, free_delivery_over: cents(e.target.value) })} /></label>}
    </div>
    <div className="row gap"><span className="grow sub">{changed ? "Not saved yet." : "In force now."}</span>
      <button className="btn primary" disabled={!changed} onClick={() => act({ do: "set_pay", mode: d.mode, hourly_wage: d.hourly_wage, runner_pay: d.runner_pay, offer_hourly: d.offer_hourly, free_delivery_over: d.free_delivery_over }, "Saved. It applies from the next trip and order.")}>Save</button></div>
  </section>;
}

function PolicyEditor({ p, act }: { p: Policy; act: (a: Record<string, unknown>, ok: string) => void }) {
  const [d, setD] = useState<Policy>(() => JSON.parse(JSON.stringify(p)));
  const edit = (f: (x: Policy) => void) => setD(x => { const y = JSON.parse(JSON.stringify(x)); f(y); return y; });
  const num = (v: string) => Number(v.replace(/[^\d.]/g, "")) || 0;
  const changed = JSON.stringify(d) !== JSON.stringify(p);
  return <section className="block care-policy">
    <div className="block-head"><h2>Making it right</h2><span className="sub">Applies to every venue from the next claim. Checked before it's saved.</span></div>
    <div className="care-line"><b>By default, make it right as</b>
      <div className="seg">{(["credit", "refund"] as const).map(f => <button key={f} className={d.form === f ? "on" : ""} onClick={() => edit(x => { x.form = f; })}>{f === "credit" ? "Agape credit" : "A refund to the card"}</button>)}</div></div>
    {Object.entries(d.rules).map(([k, r]) => <div key={k} className="care-rule">
      <div className="row gap"><b className="grow">{r.words}</b>
        <label className="check"><input type="checkbox" checked={r.photo} onChange={e => edit(x => { x.rules[k].photo = e.target.checked; })} /> Fan's photo needed</label>
        <label className="check"><input type="checkbox" checked={r.auto} onChange={e => edit(x => { x.rules[k].auto = e.target.checked; })} /> Settle automatically</label>
      </div>
      <div className="care-grid">
        <label className="sf"><span>Fault unless decided</span><select value={r.fault} onChange={e => edit(x => { x.rules[k].fault = e.target.value; })}><option value="stand">The stand</option><option value="runner">The runner</option><option value="agape">Agape</option></select></label>
        <label className="sf"><span>As</span><select value={r.form ?? ""} onChange={e => edit(x => { x.rules[k].form = (e.target.value || null) as any; })}><option value="">The default</option><option value="credit">Credit</option><option value="refund">Refund</option></select></label>
        {PARTS.map(([part, label]) => { const cv = r.covers.find(x => x.part === part); return <label key={part} className="sf"><span>{label} back, from</span>
          <select value={cv?.from ?? ""} onChange={e => edit(x => { const cs = x.rules[k].covers.filter(y => y.part !== part); if (e.target.value) cs.push({ part, from: e.target.value as any, bps: cv?.bps ?? 10000 }); x.rules[k].covers = cs; })}>
            {FROM.filter(([f]) => f !== "tax" || part === "tax").map(([f, l]) => <option key={f} value={f}>{l}</option>)}</select></label>; })}
      </div>
    </div>)}
    <div className="block-head"><h2>Shifts</h2><span className="sub">A shift is the event: where, when and how long.</span></div>
    <label className="check"><input type="checkbox" checked={!!(d as any).fixed_shift} onChange={e => edit(x => { (x as any).fixed_shift = e.target.checked; })} /> Fixed shifts: runners stay online for the whole event and can't end early</label>
    <div className="block-head"><h2>Tips, limits and proof</h2></div>
    <div className="care-grid">
      <label className="sf"><span>Tip shown first, % of items</span><input inputMode="decimal" value={d.tip_bps / 100} onChange={e => edit(x => { x.tip_bps = Math.round(num(e.target.value) * 100); })} /></label>
      <label className="sf"><span>Or at least $</span><input inputMode="decimal" value={(d.tip_min / 100).toFixed(2)} onChange={e => edit(x => { x.tip_min = Math.round(num(e.target.value) * 100); })} /></label>
      <label className="sf"><span>Automatic claims per fan, per night</span><input inputMode="numeric" value={d.auto_per_fan} onChange={e => edit(x => { x.auto_per_fan = num(e.target.value); })} /></label>
      <label className="sf"><span>Per stand, per night</span><input inputMode="numeric" value={d.auto_per_stand} onChange={e => edit(x => { x.auto_per_stand = num(e.target.value); })} /></label>
      <label className="sf"><span>Handed over within (m) counts as delivered</span><input inputMode="numeric" value={d.handoff_radius_m} onChange={e => edit(x => { x.handoff_radius_m = num(e.target.value); })} /></label>
      <label className="sf"><span>Proof starts after orders</span><input inputMode="numeric" value={d.proof_after_orders} onChange={e => edit(x => { x.proof_after_orders = num(e.target.value); })} /></label>
      <label className="sf"><span>Runner photo at claim rate %</span><input inputMode="decimal" value={d.photo_at_bps / 100} onChange={e => edit(x => { x.photo_at_bps = Math.round(num(e.target.value) * 100); })} /></label>
      <label className="sf"><span>Fan's PIN at claim rate %</span><input inputMode="decimal" value={d.pin_at_bps / 100} onChange={e => edit(x => { x.pin_at_bps = Math.round(num(e.target.value) * 100); })} /></label>
    </div>
    <div className="row gap"><span className="grow sub">{changed ? "Not saved yet." : "This is the policy in force."}</span>
      <button className="btn ghost" disabled={!changed} onClick={() => setD(JSON.parse(JSON.stringify(p)))}>Undo</button>
      <button className="btn primary" disabled={!changed} onClick={() => act({ do: "set_policy", policy: d }, "Policy saved. It applies from the next claim.")}>Save policy</button></div>
  </section>;
}

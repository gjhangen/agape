// Setup: onboarding a venue is filling in its template. Every change is checked by the kernel as
// you type; a preview runs a whole simulated night on the template; then it opens.
import { React, useMemo, useState } from "./r";
import { setup, setWorldOverride, world as currentWorld } from "./kernel";
import { MeasureTile, money } from "./parts";

type Issue = { path: string; message: string; blocking: boolean };
type W = any;

const num = (v: string) => (v.trim() === "" ? 0 : Number(v));
const dollars = (c: number) => (c / 100).toFixed(2);
const cents = (v: string) => Math.round(num(v) * 100);
const slug = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 24) || "item";

function Field({ label, value, set, type = "text", step, wide }: { label: string; value: string | number; set: (v: string) => void; type?: string; step?: string; wide?: boolean }) {
  return <label className={"sf" + (wide ? " wide" : "")}><span>{label}</span><input type={type} step={step} value={value} onChange={e => set(e.target.value)} /></label>;
}

export function SetupLens({ toast }: { toast: (s: string) => void }) {
  const [w, setW] = useState<W>(() => JSON.parse(JSON.stringify(currentWorld())));
  const [report, setReport] = useState<any>(null);
  const [running, setRunning] = useState(false);
  const issues: Issue[] = useMemo(() => setup.check(w).issues ?? [], [w]);
  const blocking = issues.filter(i => i.blocking);
  const edit = (f: (d: W) => void) => setW((prev: W) => { const d = JSON.parse(JSON.stringify(prev)); f(d); return d; });
  const stands: any[] = w.stands;
  const run = () => { setRunning(true); setReport(null); setTimeout(() => { setReport(setup.preview(w)); setRunning(false); }, 30); };
  const open = () => { setWorldOverride(w); location.reload(); };
  const r = report?.report;
  const m = r?.money ?? {};
  return (
    <div className="role setup">
      <header className="role-head"><span className="eyebrow">Set up a venue</span><span className="sub">Fill it in; the kernel checks every change and can run a whole night on it before it opens.</span></header>
      <div className={"issues " + (blocking.length ? "bad" : "good")} role="status">
        <b>{blocking.length ? `${blocking.length} thing${blocking.length === 1 ? "" : "s"} to fix before it can open` : "Ready to open"}</b>
        {issues.length > 0 && <ul>{issues.map((i, k) => <li key={k} className={i.blocking ? "block" : "note"}>{i.message}</li>)}</ul>}
      </div>
      <div className="role-grid">
        <section className="block">
          <div className="block-head"><h2>The venue</h2></div>
          <div className="sf-grid">
            <Field label="Name fans know it by" value={w.name ?? ""} set={v => edit(d => { d.name = v; })} wide />
            <Field label="Address" value={w.address ?? ""} set={v => edit(d => { d.address = v; })} wide />
            <Field label="Capacity" type="number" value={w.capacity ?? 0} set={v => edit(d => { d.capacity = num(v); })} />
            <Field label="Levels" type="number" value={w.levels} set={v => edit(d => { d.levels = num(v); })} />
            <Field label="Sections a level" type="number" value={w.sections_per_level} set={v => edit(d => { d.sections_per_level = num(v); })} />
          </div>
          <div className="block-head"><h2>Terms</h2></div>
          <div className="sf-grid">
            <Field label="Delivery fee $" type="number" step="0.01" value={dollars(w.terms.delivery_fee)} set={v => edit(d => { d.terms.delivery_fee = cents(v); })} />
            <Field label="Runner pay per delivery $" type="number" step="0.01" value={dollars(w.terms.runner_pay)} set={v => edit(d => { d.terms.runner_pay = cents(v); })} />
            <Field label="Agape's share %" type="number" step="0.5" value={w.terms.platform_share_bps / 100} set={v => edit(d => { d.terms.platform_share_bps = Math.round(num(v) * 100); })} />
            <Field label="Sales tax %" type="number" step="0.25" value={w.terms.tax_bps / 100} set={v => edit(d => { d.terms.tax_bps = Math.round(num(v) * 100); })} />
          </div>
        </section>
        <section className="block">
          <div className="block-head"><h2>Stands</h2><button className="btn ghost sm" onClick={() => edit(d => { const n = d.stands.length + 1; d.stands.push({ id: `stand-${n}`, name: `Stand ${n}`, level: 1, ring: 0, stations: 2 }); })}>Add a stand</button></div>
          <table className="sf-table"><thead><tr><th>Name</th><th>Level</th><th>By section</th><th>Stations</th><th /></tr></thead>
            <tbody>{stands.map((s, i) => <tr key={i}>
              <td><input value={s.name} onChange={e => edit(d => { d.stands[i].name = e.target.value; })} aria-label="Stand name" /></td>
              <td><input type="number" value={s.level} onChange={e => edit(d => { d.stands[i].level = num(e.target.value); })} aria-label="Level" /></td>
              <td><input type="number" value={s.ring + 1} onChange={e => edit(d => { d.stands[i].ring = num(e.target.value) - 1; })} aria-label="Nearest section on its level" /></td>
              <td><input type="number" value={s.stations} onChange={e => edit(d => { d.stands[i].stations = num(e.target.value); })} aria-label="Stations" /></td>
              <td><button className="link-inline small" onClick={() => edit(d => { d.stands.splice(i, 1); })}>Remove</button></td></tr>)}</tbody></table>
        </section>
        <section className="block span2">
          <div className="block-head"><h2>Menu</h2><button className="btn ghost sm" onClick={() => edit(d => { d.things.push({ id: `item-${d.things.length + 1}`, name: "New item", stand: d.stands[0]?.id ?? "", price: 800, prep_s: 120, category: "food", constraints: [] }); })}>Add an item</button></div>
          <table className="sf-table"><thead><tr><th>Item</th><th>Price $</th><th>Stand</th><th>Minutes to make</th><th>Kind</th><th>21+</th><th /></tr></thead>
            <tbody>{w.things.map((t: any, i: number) => {
              const adult = (t.constraints ?? []).some((c: any) => c.min_age);
              return <tr key={i}>
                <td><input value={t.name} onChange={e => edit(d => { d.things[i].name = e.target.value; if (d.things[i].id.startsWith("item-")) d.things[i].id = slug(e.target.value); })} aria-label="Item name" /></td>
                <td><input type="number" step="0.25" value={dollars(t.price)} onChange={e => edit(d => { d.things[i].price = cents(e.target.value); })} aria-label="Price" /></td>
                <td><select value={t.stand} onChange={e => edit(d => { d.things[i].stand = e.target.value; })} aria-label="Stand">{stands.map(s => <option key={s.id} value={s.id}>{s.name}</option>)}{!stands.some(s => s.id === t.stand) && <option value={t.stand}>{t.stand} (missing)</option>}</select></td>
                <td><input type="number" step="0.5" value={Math.round(t.prep_s / 6) / 10} onChange={e => edit(d => { d.things[i].prep_s = Math.round(num(e.target.value) * 60); })} aria-label="Minutes to make" /></td>
                <td><select value={t.category} onChange={e => edit(d => { d.things[i].category = e.target.value; })} aria-label="Kind"><option value="food">Food</option><option value="drink">Drink</option><option value="merch">Gear</option></select></td>
                <td><input type="checkbox" checked={adult} onChange={e => edit(d => { const c = (d.things[i].constraints ?? []).filter((x: any) => !x.min_age); if (e.target.checked) c.push({ min_age: 21 }); d.things[i].constraints = c; })} aria-label="21 and over" /></td>
                <td><button className="link-inline small" onClick={() => edit(d => { d.things.splice(i, 1); })}>Remove</button></td></tr>;
            })}</tbody></table>
        </section>
        <section className="block span2">
          <div className="block-head"><h2>Try a night on it</h2><span className="sub">1,000 simulated fans, 15 runners, a whole game</span></div>
          <div className="setup-actions">
            <button className="btn primary big" disabled={blocking.length > 0 || running} onClick={run}>{running ? "Running a night…" : "Preview a night"}</button>
            <button className="btn ghost big" disabled={blocking.length > 0} onClick={open}>Open it here</button>
            <details className="tpl-json"><summary className="btn ghost big">See the template</summary>
              <textarea readOnly value={JSON.stringify(w, null, 2)} onFocus={e => e.currentTarget.select()} aria-label="The template as JSON" rows={14} /></details>
          </div>
          {r && <>
            <div className="tiles four">
              <MeasureTile label="Orders" value={r.orders.toLocaleString()} sub={`${r.delivered.toLocaleString()} delivered`} />
              <MeasureTile label="Seat waits" value={`${Math.round(r.wait_p50_s / 60)} min`} sub={`90% within ${Math.round(r.wait_p90_s / 60)} min`} tone={r.wait_p90_s > 1800 ? "bad" : r.wait_p90_s > 1200 ? "warn" : "good"} />
              <MeasureTile label="Late" value={r.late} sub={`${r.cannot_fulfil} the stand couldn't make`} tone={r.late > r.orders * 0.1 ? "warn" : "good"} />
              <MeasureTile label="Runners earn" value={`${money(r.runner_hourly)}/h`} sub={`${r.batch_avg.toFixed(1)} orders a trip`} />
            </div>
            <div className="lines">
              <div className="ll"><span>Fans paid</span><b className="num">{money(m["fans charged"])}</b></div>
              <div className="ll"><span>Stands</span><b className="num">{money(m["stands net"])}</b></div>
              <div className="ll"><span>Runners, pay and tips</span><b className="num">{money((m["runner pay"] ?? 0) + (m["runner tips"] ?? 0))}</b></div>
              <div className="ll"><span>Agape</span><b className="num">{money(m["agape net"])}</b></div>
              <div className="ll"><span>Sales tax held</span><b className="num">{money(m["tax"])}</b></div>
            </div>
            <p className="provenance">A simulation of this template, not a forecast. Every unit {r.conserved ? "balanced to zero" : "did NOT balance"}. Busiest stands: {r.stands.slice(0, 3).map((s: any) => `${s.name} ${money(s.revenue)}`).join(", ")}.</p>
          </>}
        </section>
      </div>
    </div>
  );
}

export function resetVenue() { setWorldOverride(null); location.reload(); }

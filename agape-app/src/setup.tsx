// Setup: onboarding a venue is filling in its template. Every change is checked by the kernel as
// you type; a preview runs a whole simulated night on the template; then it opens.
import { React, useMemo, useState } from "./r";
import { kernel, setup, setWorldOverride, world as currentWorld } from "./kernel";
import { LEVELS, MeasureTile, money, MoneyFlows, isConcert } from "./parts";

/** The level a feature arrives at. */
const levelOf = (k: string) => LEVELS.find(l => l.on.includes(k))?.n ?? 5;
import { LookPicker } from "./lookpicker";
import { ItemArt } from "./Art";

type Issue = { path: string; message: string; blocking: boolean };
type W = any;

const num = (v: string) => (v.trim() === "" ? 0 : Number(v));
const dollars = (c: number) => (c / 100).toFixed(2);
const cents = (v: string) => Math.round(num(v) * 100);
const slug = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 24) || "item";

/** A picture from the venue's computer, made small enough to travel with the template. */
function picture(file: File, max = 256): Promise<string> {
  return new Promise((ok, no) => {
    const img = new Image();
    img.onload = () => {
      const k = Math.min(1, max / Math.max(img.width, img.height));
      const c = document.createElement("canvas"); c.width = Math.round(img.width * k); c.height = Math.round(img.height * k);
      c.getContext("2d")!.drawImage(img, 0, 0, c.width, c.height);
      ok(c.toDataURL(file.type === "image/png" || file.type === "image/svg+xml" ? "image/png" : "image/jpeg", 0.85)); URL.revokeObjectURL(img.src);
    };
    img.onerror = no; img.src = URL.createObjectURL(file);
  });
}

function Upload({ label, value, set, round }: { label: string; value?: string; set: (v: string | undefined) => void; round?: boolean }) {
  return (
    <label className={"upload" + (round ? " round" : "")}>
      {value ? <img src={value} alt="" /> : <span className="upload-empty">+</span>}
      <span className="upload-l">{value ? "Change" : label}</span>
      <input type="file" accept="image/*" onChange={async e => { const el = e.currentTarget; const f = el.files?.[0]; if (f) set(await picture(f)); el.value = ""; }} />
      {value && <button type="button" className="link-inline small" onClick={e => { e.preventDefault(); set(undefined); }}aria-label="Remove" title="Remove">×</button>}
    </label>
  );
}

/** What a venue can switch on or off for its fans. Everything else about Agape stays as it is. */
export const FEATURES: [string, string, string][] = [
  ["chat", "Crowd chat", "Fans talk to the whole arena or just their section"],
  ["predictions", "Call the quarter", "Pick who wins each part of the game for points; halftime trivia"],
  ["cards", "Moment cards", "Big plays become limited cards fans collect and trade"],
  ["battle", "Section battle", "Sections compete on orders, live"],
  ["tickets", "Tickets", "Upcoming events, seat maps, buying and reselling"],
  ["around", "Around the venue", "Rides, parking and local business offers"],
  ["upgrades", "Seat upgrades", "Move to a better section during the game"],
  ["gifting", "Send to a seat", "Buy something for a friend in another section"],
  ["priority", "Go first", "Spend points to jump the kitchen line"],
  ["colors", "Wear your colors", "Fans can switch the app to either team's colors"],
  ["players", "Player deals", "Fans and businesses make paid offers to players (shout-outs, appearances), within team and league rules"],
];

const TEAM_DEFAULT = { home: { name: "Harbor Kings", short: "Kings", abbr: "HBK", color: "#B8174F" }, away: { name: "Mesa Comets", short: "Comets", abbr: "MSC", color: "#2F6DB5" } };

function TeamEditor({ side, w, edit, roster }: { side: "home" | "away"; w: W; edit: (f: (d: W) => void) => void; roster: { name: string; number: number }[] }) {
  const t = w.brand?.[side] ?? {};
  const d = TEAM_DEFAULT[side];
  const set = (k: string, v: any) => edit(x => { x.brand = x.brand ?? {}; x.brand[side] = { ...(x.brand[side] ?? {}), [k]: v }; });
  const players: any[] = t.players ?? [];
  const setPlayer = (i: number, k: string, v: any) => edit(x => { x.brand = x.brand ?? {}; const b = x.brand[side] = { ...(x.brand[side] ?? {}) }; const ps = [...(b.players ?? [])]; while (ps.length <= i) ps.push({ name: "" }); ps[i] = { ...ps[i], [k]: v }; b.players = ps; });
  return (
    <div className="team-ed" style={{ ["--team" as any]: t.color || d.color }}>
      <div className="team-ed-head">
        <Upload label="Logo" value={t.logo} set={v => set("logo", v)} />
        <div className="sf-grid">
          <label className="sf wide"><span>{side === "home" ? "Home team" : "Visitors"}</span><input value={t.name ?? ""} placeholder={d.name} maxLength={40} onChange={e => set("name", e.target.value)} /></label>
          <label className="sf"><span>Short name</span><input value={t.short ?? ""} placeholder={d.short} maxLength={16} onChange={e => set("short", e.target.value)} /></label>
          <label className="sf"><span>Initials</span><input value={t.abbr ?? ""} placeholder={d.abbr} maxLength={4} onChange={e => set("abbr", e.target.value.toUpperCase().replace(/[^A-Z0-9]/g, ""))} /></label>
          <label className="sf color"><span>Color</span><input type="color" value={t.color || d.color} onChange={e => set("color", e.target.value)} /></label>
        </div>
      </div>
      <details className="roster-ed"><summary>Players</summary>
        {roster.slice(0, 8).map((p, i) => <div key={i} className="roster-row">
          <input className="num-in" type="number" value={players[i]?.number ?? ""} placeholder={String(p.number)} onChange={e => setPlayer(i, "number", e.target.value === "" ? undefined : Number(e.target.value))} aria-label="Number" />
          <input value={players[i]?.name ?? ""} placeholder={p.name} maxLength={32} onChange={e => setPlayer(i, "name", e.target.value)} aria-label="Player name" />
        </div>)}
      </details>
    </div>
  );
}

function Field({ label, value, set, type = "text", step, wide }: { label: string; value: string | number; set: (v: string) => void; type?: string; step?: string; wide?: boolean }) {
  return <label className={"sf" + (wide ? " wide" : "")}><span>{label}</span><input type={type} step={step} value={value} onChange={e => set(e.target.value)} /></label>;
}

export function SetupLens({ toast, inApp }: { toast: (s: string) => void; inApp?: boolean }) {
  const [w, setW] = useState<W>(() => JSON.parse(JSON.stringify(currentWorld())));
  const [report, setReport] = useState<any>(null);
  const [running, setRunning] = useState(false);
  const issues: Issue[] = useMemo(() => setup.check(w).issues ?? [], [w]);
  const blocking = issues.filter(i => i.blocking);
  const edit = (f: (d: W) => void) => setW((prev: W) => { const d = JSON.parse(JSON.stringify(prev)); f(d); return d; });
  const stands: any[] = w.stands;
  const parties = useMemo(() => kernel.view("parties" as any) as any, []);
  const roster = (team: string) => ((parties?.players ?? []) as any[]).filter(p => String(p.id).startsWith(team + "-")).sort((a, b) => Number(a.id.split("-")[1]) - Number(b.id.split("-")[1])).map(p => ({ name: p.name, number: p.number ?? 0 }));
  const run = () => { setRunning(true); setReport(null); setTimeout(() => { setReport(setup.preview(w)); setRunning(false); setTimeout(() => document.querySelector(".preview-results")?.scrollIntoView({ behavior: "smooth", block: "start" }), 60); }, 30); };
  const open = () => { setWorldOverride(w); location.reload(); };
  const [link, setLink] = useState<string | null>(null);
  const [sharing, setSharing] = useState(false);
  const share = async () => {
    setSharing(true);
    try {
      const r = await fetch("/api/studio", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(w) }).then(r => r.json());
      if (r.ok) { setLink(`${location.origin}/demo/?v=${r.id}`); setTimeout(() => document.querySelector(".share-link")?.scrollIntoView({ behavior: "smooth", block: "center" }), 60); } else toast(r.error);
    } catch { toast("Sharing works on agapetoyourseat.com, not in this preview."); }
    setSharing(false);
  };
  const r = report?.report;
  const m = r?.money ?? {};
  return (
    <div className="role setup">
      {inApp ? <header className="role-head"><span className="eyebrow">Events · set up a night</span><span className="sub">{isConcert() ? "Who's on" : "Who's playing"}, your menu, your terms and what fans get. Every change is checked as you go, and you can preview a whole night before it opens.</span></header>
        : <header className="role-head"><span className="eyebrow">Studio · make it yours</span><span className="sub">Your teams, your menu, your rules. Every change is checked as you go; preview a whole night on it, then open it and see it from every seat.</span></header>}
      <div className={"issues " + (blocking.length ? "bad" : "good")} role="status">
        <b>{blocking.length ? `${blocking.length} thing${blocking.length === 1 ? "" : "s"} to fix before it can open` : "Ready to open"}</b>
        {issues.length > 0 && <ul>{issues.map((i, k) => <li key={k} className={i.blocking ? "block" : "note"}>{i.message}</li>)}</ul>}
      </div>
      <div className="role-grid">
        <section className="block span2 studio-brand">
          {isConcert() ? <><div className="block-head"><h2>Tonight's lineup</h2><span className="sub">Set by the promoter's booking. The running order sets when the rush comes and when last call is.</span></div>
            <ul className="lineup">{(w.event?.acts ?? []).map((a: string, i: number) => <li key={a}><b>{a}</b><small>{i === (w.event?.acts?.length ?? 1) - 1 ? "Headliner" : "Opener"}</small></li>)}</ul></>
          : <><div className="block-head"><h2>Your teams</h2><span className="sub">Names, colors, logos and players. Leave a field empty to keep what's there now.</span></div>
          <div className="team-eds">
            <TeamEditor side="home" w={w} edit={edit} roster={roster("hbk")} />
            <TeamEditor side="away" w={w} edit={edit} roster={roster("msc")} />
          </div></>}
          <div className="studio-look"><span className="eyebrow">The app's look</span><LookPicker /><small>{isConcert() ? "Fans see the tour's colors automatically; this sets everything else." : "Fans see the home team's colors automatically; this sets everything else."}</small></div>
        </section>
        <section className="block span2">
          <div className="block-head"><h2>What fans get</h2><span className="sub">Ordering to the seat, tracking, receipts and ratings are always on. Everything else comes in packages your venue switches on when it wants them.</span>
            <button className="btn ghost sm" onClick={() => edit(d => { d.features = Object.fromEntries([...FEATURES.map(([k]) => [k, false]), ["points", false], ["offers", false]]); })}>Ordering only</button>
            <button className="btn ghost sm" onClick={() => edit(d => { d.features = Object.fromEntries(FEATURES.map(([k]) => [k, true])); })}>Everything on</button></div>
          <div className="features">{FEATURES.filter(([k]) => !isConcert() || !["predictions", "colors", "cards"].includes(k)).map(([k, name, blurb]) => {
            const on = w.features?.[k] !== false;
            return <label key={k} className={"feature" + (on ? " on" : "")}><input type="checkbox" checked={on} onChange={e => edit(d => { d.features = { ...(d.features ?? {}), [k]: e.target.checked }; })} />
              <span><b>{name} <em className="opt">{LEVELS[levelOf(k) - 1].name}</em></b><small>{blurb}</small></span></label>;
          })}</div>
        </section>
        <section className="block">
          <div className="block-head"><h2>The venue</h2></div>
          <div className="sf-grid">
            <Field label="Name fans know it by" value={w.name ?? ""} set={v => edit(d => { d.name = v; })} wide />
            <Field label="Address" value={w.address ?? ""} set={v => edit(d => { d.address = v; })} wide />
            <Field label="Capacity" type="number" value={w.capacity ?? 0} set={v => edit(d => { d.capacity = num(v); })} />
            <Field label="Fans using Agape tonight" type="number" value={w.pilot?.fans ?? 3000} set={v => edit(d => { d.pilot = { ...(d.pilot ?? {}), fans: Math.max(100, Math.min(20000, num(v))) }; })} />
            <Field label="Runners on shift" type="number" value={w.pilot?.runners ?? 40} set={v => edit(d => { d.pilot = { ...(d.pilot ?? {}), runners: Math.max(1, Math.min(300, num(v))) }; })} />
            <Field label="Levels" type="number" value={w.levels} set={v => edit(d => { d.levels = num(v); })} />
            <Field label="Sections a level" type="number" value={w.sections_per_level} set={v => edit(d => { d.sections_per_level = num(v); })} />
          </div>
          <div className="block-head"><h2>Terms</h2><span className="sub">Set per event and negotiated per deal</span></div>
          <div className="sf-grid">
            {inApp ? <p className="pay-explain wide">Agape's share, card processing and interchange are as set in your agreement with Agape.</p> : <Field label="Agape's share of sales %" type="number" step="0.5" value={w.terms.platform_share_bps / 100} set={v => edit(d => { d.terms.platform_share_bps = Math.round(num(v) * 100); })} />}
            <Field label="Delivery fee $" type="number" step="0.01" value={dollars(w.terms.delivery_fee)} set={v => edit(d => { d.terms.delivery_fee = cents(v); })} />
            {!inApp && <><Field label="Card processing %" type="number" step="0.1" value={w.terms.proc_bps / 100} set={v => edit(d => { d.terms.proc_bps = Math.round(num(v) * 100); })} />
            <Field label="plus per charge $" type="number" step="0.01" value={dollars(w.terms.proc_fixed)} set={v => edit(d => { d.terms.proc_fixed = cents(v); })} />
            <Field label="Interchange we pay %" type="number" step="0.05" value={(w.terms.interchange_bps ?? 200) / 100} set={v => edit(d => { d.terms.interchange_bps = Math.round(num(v) * 100); })} />
            <Field label="plus per charge $" type="number" step="0.01" value={dollars(w.terms.interchange_fixed ?? 10)} set={v => edit(d => { d.terms.interchange_fixed = cents(v); })} /></>}
            <Field label="Sales tax %" type="number" step="0.25" value={w.terms.tax_bps / 100} set={v => edit(d => { d.terms.tax_bps = Math.round(num(v) * 100); })} />
          </div>
          <div className="block-head"><h2>Runner pay</h2></div>
          {inApp ? <p className="pay-explain">Runners are independent and paid by Agape, out of its share and the delivery fee, on Agape's terms for every venue. Tips go to them in full.</p> : <>
          <div className="seg fill pay-mode" role="group" aria-label="How runners are paid">
            <button className={(w.terms.pay_mode ?? "offers") === "offers" ? "on" : ""} onClick={() => edit(d => { d.terms.pay_mode = "offers"; })}>Offers that rise</button>
            <button className={w.terms.pay_mode === "fixed" ? "on" : ""} onClick={() => edit(d => { d.terms.pay_mode = "fixed"; })}>Fixed per delivery</button>
            <button className={w.terms.pay_mode === "hourly" ? "on" : ""} onClick={() => edit(d => { d.terms.pay_mode = "hourly"; })}>By the hour</button>
          </div>
          {w.terms.pay_mode === "hourly" ? <>
            <p className="pay-explain">A wage for every hour online, whatever the trips. Runners keep every tip on top.</p>
            <div className="sf-grid"><Field label="Per hour online $" type="number" step="0.5" value={dollars(w.terms.hourly_wage ?? 1800)} set={v => edit(d => { d.terms.hourly_wage = cents(v); })} /></div>
          </> : w.terms.pay_mode === "fixed" ? <>
            <p className="pay-explain">Every delivery pays the same, whoever takes it. Runners keep every tip on top.</p>
            <div className="sf-grid"><Field label="Pay per delivery $" type="number" step="0.05" value={dollars(w.terms.runner_pay)} set={v => edit(d => { d.terms.runner_pay = cents(v); })} /></div>
          </> : <>
            <p className="pay-explain">Each trip is offered to the nearest runner, priced on its walk and wait time and how much there is to carry. If they pass, it goes to the next runner for a little more, until someone takes it. Runners keep every tip on top.</p>
            <div className="sf-grid">
              <Field label="First offer, per hour of the trip $" type="number" step="0.5" value={dollars(w.terms.offer_hourly ?? 1500)} set={v => edit(d => { d.terms.offer_hourly = cents(v); })} />
              <Field label="Each extra item $" type="number" step="0.05" value={dollars(w.terms.offer_per_item ?? 25)} set={v => edit(d => { d.terms.offer_per_item = cents(v); })} />
              <Field label="Smallest offer $" type="number" step="0.05" value={dollars(w.terms.offer_min ?? 150)} set={v => edit(d => { d.terms.offer_min = cents(v); })} />
              <Field label="Raise each pass %" type="number" step="1" value={(w.terms.offer_raise_bps ?? 1500) / 100} set={v => edit(d => { d.terms.offer_raise_bps = Math.round(num(v) * 100); })} />
              <Field label="Highest, % of first" type="number" step="5" value={(w.terms.offer_cap_bps ?? 16000) / 100} set={v => edit(d => { d.terms.offer_cap_bps = Math.round(num(v) * 100); })} />
            </div>
          </>}</>}
        </section>
        <section className="block span2">
          <div className="block-head"><h2>Around the venue</h2><span className="sub">Your rideshare, parking and partner businesses, as fans will see them</span></div>
          <div className="sf-grid">
            {(w.businesses ?? []).map((b: any, i: number) => <Field key={b.id} label={b.kind === "rideshare" ? "Rideshare" : b.kind === "parking" ? "Parking" : "Partner"} value={b.name} set={v => edit(d => { d.businesses[i].name = v; (d.around ?? []).forEach((a: any) => { if (a.operator === b.id || a.id === b.id) a.name = a.name === b.name ? v : a.name; }); })} />)}
            {(w.around ?? []).filter((a: any) => !(w.businesses ?? []).some((b: any) => b.id === a.id || b.id === a.operator)).map((a: any) => <Field key={a.id} label="Place nearby" value={a.name} set={v => edit(d => { const x = d.around.find((y: any) => y.id === a.id); if (x) x.name = v; })} />)}
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
              <td><button className="link-inline small" onClick={() => edit(d => { d.stands.splice(i, 1); })}aria-label="Remove" title="Remove">×</button></td></tr>)}</tbody></table>
        </section>
        <section className="block span2">
          <div className="block-head"><h2>Menu</h2><button className="btn ghost sm" onClick={() => edit(d => { d.things.push({ id: `item-${d.things.length + 1}`, name: "New item", stand: d.stands[0]?.id ?? "", price: 800, prep_s: 120, category: "food", constraints: [] }); })}>Add an item</button></div>
          <table className="sf-table"><thead><tr><th>Photo</th><th>Item</th><th>Price $</th><th>Stand</th><th>Minutes to make</th><th>Kind</th><th>21+</th><th /></tr></thead>
            <tbody>{w.things.map((t: any, i: number) => {
              const adult = (t.constraints ?? []).some((c: any) => c.min_age);
              return <tr key={i}>
                <td className="photo-cell"><label className="photo-pick" title="Add a photo">{t.image ? <img src={t.image} alt="" /> : <ItemArt name={t.name} size={34} />}
                  <input type="file" accept="image/*" onChange={async e => { const el = e.currentTarget; const f = el.files?.[0]; if (f) { const v = await picture(f, 200); edit(d => { d.things[i].image = v; }); } el.value = ""; }} /></label></td>
                <td><input value={t.name} onChange={e => edit(d => { d.things[i].name = e.target.value; if (d.things[i].id.startsWith("item-")) d.things[i].id = slug(e.target.value); })} aria-label="Item name" /></td>
                <td><input type="number" step="0.25" value={dollars(t.price)} onChange={e => edit(d => { d.things[i].price = cents(e.target.value); })} aria-label="Price" /></td>
                <td><select value={t.stand} onChange={e => edit(d => { d.things[i].stand = e.target.value; })} aria-label="Stand">{stands.map(s => <option key={s.id} value={s.id}>{s.name}</option>)}{!stands.some(s => s.id === t.stand) && <option value={t.stand}>{t.stand} (missing)</option>}</select></td>
                <td><input type="number" step="0.5" value={Math.round(t.prep_s / 6) / 10} onChange={e => edit(d => { d.things[i].prep_s = Math.round(num(e.target.value) * 60); })} aria-label="Minutes to make" /></td>
                <td><select value={t.category} onChange={e => edit(d => { d.things[i].category = e.target.value; })} aria-label="Kind"><option value="food">Food</option><option value="drink">Drink</option><option value="merch">Gear</option></select></td>
                <td><input type="checkbox" checked={adult} onChange={e => edit(d => { const c = (d.things[i].constraints ?? []).filter((x: any) => !x.min_age); if (e.target.checked) c.push({ min_age: 21 }); d.things[i].constraints = c; })} aria-label="21 and over" /></td>
                <td><button className="link-inline small" onClick={() => edit(d => { d.things.splice(i, 1); })}aria-label="Remove" title="Remove">×</button></td></tr>;
            })}</tbody></table>
        </section>
        <section className="block span2">
          <div className="block-head"><h2>Try a night on it</h2><span className="sub">{(w.pilot?.fans ?? 3000).toLocaleString()} fans using Agape, {w.pilot?.runners ?? 40} runners, a whole event</span></div>
          <div className="setup-actions">
            <button className="btn primary big" disabled={blocking.length > 0 || running} onClick={run}>{running ? "Running a night…" : "Preview a night"}</button>
            {!inApp && <button className="btn ghost big" disabled={blocking.length > 0} onClick={open}>Open it here</button>}
            {!inApp && <button className="btn ghost big" disabled={blocking.length > 0 || sharing} onClick={share}>{sharing ? "Saving…" : "Share as a link"}</button>}
            <details className="tpl-json"><summary className="btn ghost big">See the template</summary>
              <textarea readOnly value={JSON.stringify(w, null, 2)} onFocus={e => e.currentTarget.select()} aria-label="The template as JSON" rows={14} /></details>
          </div>
          {link && <div className="share-link" role="status"><span>Anyone with this link opens the simulator as this venue, with your teams, menu and settings:</span>
            <div className="row gap"><input readOnly value={link} onFocus={e => e.currentTarget.select()} aria-label="Link to this version" /><button className="btn primary" onClick={() => { navigator.clipboard?.writeText(link).then(() => toast("Link copied."), () => toast("Select the link and copy it.")); }}>Copy</button></div></div>}
          {r && <div className="preview-results">
            <div className="tiles four">
              <MeasureTile label="Orders" value={r.orders.toLocaleString()} sub={`${r.delivered.toLocaleString()} delivered`} />
              <MeasureTile label="Seat waits" value={`${Math.round(r.wait_p50_s / 60)} min`} sub={`90% within ${Math.round(r.wait_p90_s / 60)} min`} tone={r.wait_p90_s > 1800 ? "bad" : r.wait_p90_s > 1200 ? "warn" : "good"} />
              <MeasureTile label="Late" value={r.late} sub={`${r.cannot_fulfil} the stand couldn't make`} tone={r.late > r.orders * 0.1 ? "warn" : "good"} />
              <MeasureTile label="Runners earn" value={`${money(r.runner_hourly)}/h`} sub={r.trips_taken ? `pay and tips · ${Math.round(100 * r.taken_first / r.trips_taken)}% of trips taken on the first offer · ${money(r.trip_pay_avg)} a trip before tips` : "pay and tips"} tone={r.runner_hourly < 2000 ? "warn" : "good"} />
            </div>
            <MoneyFlows f={r.flows ?? {}} />
            <p className="provenance">Agape keeps {(w.terms.platform_share_bps / 100).toFixed(1)}% of each sale and the {money(w.terms.delivery_fee)} delivery fee, and pays runners {w.terms.pay_mode === "fixed" ? `${money(w.terms.runner_pay)} a delivery` : w.terms.pay_mode === "hourly" ? `${money(w.terms.hourly_wage ?? 1800)} an hour` : "by trip offers"} out of it; runners keep every tip. Agape also processes the cards: sellers pay {(w.terms.proc_bps / 100).toFixed(1)}% + {money(w.terms.proc_fixed)}, and Agape pays the card networks' interchange out of that. A preview of this setup, not a forecast. Every unit {r.conserved ? "balanced to zero" : "did NOT balance"}. Busiest stands: {r.stands.slice(0, 3).map((s: any) => `${s.name} ${money(s.revenue)}`).join(", ")}.</p>
          </div>}
        </section>
      </div>
    </div>
  );
}

export function resetVenue() { setWorldOverride(null); location.reload(); }

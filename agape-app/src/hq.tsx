// Agape HQ: the app Agape's own people run the company on. Every venue at once, support, crew,
// partners, payouts and the company's own money. On a laptop, in the story of one shift at HQ.
import { React, ReactDOM, useEffect, useState } from "./r";
import { boot, kernel, setWorldOverride } from "./kernel";
import { BRAND, Icon, money, MeasureTile, venueTime } from "./parts";
import { OpsView } from "./roles";
import { CareDesk, CareView } from "./care";
import { Beat, Invite, Shell, demoWorld, useToasts } from "./shell";
import { Note } from "./explain";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";

declare const AGAPE_WORLD: any;
type Stage = "boot" | "signin" | "app";
type Tab = "venues" | "support" | "crew" | "money" | "people";

const NOTES: Note[] = [
  { sel: ".hq-venues", title: "Every venue at once", what: "Each venue's night in one row: orders, seat waits, runners, problems.", who: "Agape operations", why: "One team watches many venues, which is how Agape grows without growing its staff at the same rate.", level: 1 },
  { sel: ".hq-take", title: "Agape's own money", what: "What Agape keeps: its share of sales, delivery fees and the margin on card processing, after paying runners.", who: "Agape's leadership", why: "The company's health, live, from the same ledger every party sees.", level: 1 },
  { sel: ".hq-crew", title: "Crew supply", what: "Runners online against the orders waiting, and how often offers are taken first time.", who: "Agape operations", why: "If too few runners take offers, pay rises and more come online. This is where that's watched.", level: 1 },
];

function HQ({ o, c, stage, next, toast, refresh }: { o: OpsView | null; c: CareView | null; stage: Stage; next: (s: Stage) => void; toast: (t: string) => void; refresh: () => void }) {
  const [tab, setTab] = useState<Tab>("venues");
  if (stage === "boot") return <div className="ob ob-boot"><div className="ob-mark"><b>agape</b><small>hq</small></div></div>;
  if (stage === "signin") return (
    <form className="ob lp-signin" onSubmit={e => { e.preventDefault(); next("app"); }}>
      <div className="lp-card"><b className="ob-logo dark">agape hq</b><h2>Sign in</h2>
        <label className="sf wide"><span>Agape email</span><input defaultValue="sam@agapetoyourseat.com" /></label>
        <label className="sf wide"><span>Security key</span><input defaultValue="Touch your key" readOnly /></label>
        <button className="btn ob-primary">Sign in</button><small className="ob-fine">Staff only. Every action here is recorded.</small></div>
    </form>
  );
  if (!o) return null;
  const L = o.ledger as any; const f = L.flows ?? {};
  const take = (f.agape ?? 0) + (f.agape_processing ?? 0);
  const open = c?.open ?? 0;
  const venues = [
    { name: BRAND.venue, city: "Harbor City", state: "Live now", orders: Object.values(o.stages).reduce((a: number, b: any) => a + (b as number), 0), wait: o.wait_p50, runners: `${o.runners.busy}/${o.runners.total}`, problems: open },
    { name: "Mesa Dome", city: "Mesa", state: "Opens Dec 12", orders: null, wait: null, runners: "28 signed up", problems: 0 },
    { name: "Riverside Amphitheater", city: "Riverside", state: "In talks", orders: null, wait: null, runners: "–", problems: 0 },
  ];
  return (
    <div className="lp-app">
      <aside className="lp-nav">
        <b className="sx-logo">agape</b>
        {([["venues", "store", "Venues"], ["support", "help", "Care"], ["crew", "people", "Crew"], ["money", "chart", "Money"], ["people", "user", "Staff"]] as [Tab, any, string][]).map(([id, ic, l]) =>
          <button key={id} className={tab === id ? "on" : ""} onClick={() => setTab(id)}><Icon name={ic} /><small>{l}{id === "support" && open ? ` · ${open}` : ""}</small></button>)}
      </aside>
      <div className="lp-main"><div className="role">
        {tab === "venues" && <>
          <header className="role-head"><span className="eyebrow">Agape HQ · {venueTime(o.clock.t)}</span><h2 className="hq-h">Venues</h2></header>
          <div className="tiles four">
            <MeasureTile label="Live tonight" value="1 venue" sub={`${o.orders_5m} orders in the last 5 min`} />
            <MeasureTile label="Agape keeps tonight" value={money(take)} sub="share, fees and processing, after runner pay" />
            <MeasureTile label="Open problems" value={open} sub="across every venue" tone={open ? "warn" : "good"} />
            <MeasureTile label="Books" value={L.conserved ? "Balanced" : "Check"} sub="every venue, to the cent" tone={L.conserved ? "good" : "bad"} />
          </div>
          <section className="block hq-venues"><div className="table-wrap"><table className="attr">
            <thead><tr><th>Venue</th><th>Status</th><th>Orders tonight</th><th>Seat wait</th><th>Runners</th><th>Problems</th></tr></thead>
            <tbody>{venues.map(v => <tr key={v.name}><td><b>{v.name}</b><br /><small className="sub">{v.city}</small></td><td><span className={"tag" + (v.state === "Live now" ? " good" : "")}>{v.state}</span></td>
              <td className="num">{v.orders ?? "–"}</td><td className="num">{v.wait != null ? `${Math.round(v.wait / 60)} min` : "–"}</td><td className="num">{v.runners}</td><td className="num">{v.problems || "–"}</td></tr>)}</tbody>
          </table></div></section>
        </>}
        {tab === "support" && c && <>
          <header className="role-head"><h2 className="hq-h">Care</h2><span className="sub">Fans' problems from every venue, made right by Agape's policy, and the records behind them. Only Agape sees this.</span></header>
          <CareDesk c={c} toast={toast} refresh={refresh} />
        </>}
        {tab === "crew" && <>
          <header className="role-head"><h2 className="hq-h">Crew</h2><span className="sub">Independent runners across every venue</span></header>
          <div className="tiles four hq-crew">
            <MeasureTile label="Out on trips" value={`${o.runners.busy} of ${o.runners.total}`} sub={BRAND.venue} />
            <MeasureTile label="Taken on the first offer" value={L.trips_taken ? `${Math.round(100 * L.taken_first / L.trips_taken)}%` : "–"} sub={`${L.trips_taken ?? 0} trips tonight`} />
            <MeasureTile label="Paid to runners tonight" value={money(f.runners ?? 0)} sub="trip pay and every tip" />
            <MeasureTile label="Applications" value="14" sub="waiting for a background check" />
          </div>
          <section className="block settings-list"><h2>To do</h2>
            {[["Approve 6 runners for Mesa Dome", "Background checks cleared and training done. Review each before approving."], ["Renew ID-check certification", "3 runners' certifications expire this month. Remind them to retake it."], ["Review a rating", "A runner was rated 2 stars at section 212. Read the fan's note and the trip first."]].map(([a, b]) =>
              <details key={a} className="faq todo"><summary>{a}</summary><p>{b}</p><button className="btn primary sm" onClick={() => toast(`${a}: marked for review by operations.`)}>Open the review</button></details>)}
          </section>
        </>}
        {tab === "money" && <>
          <header className="role-head"><h2 className="hq-h">Agape's money</h2><span className="sub">Tonight, from the same ledger every venue sees</span></header>
          <div className="tiles three hq-take">
            <MeasureTile label="Share and delivery fees" value={money(f.agape ?? 0)} sub="after runner pay and points" />
            <MeasureTile label="Card processing" value={money(f.agape_processing ?? 0)} sub="what sellers paid, less interchange" />
            <MeasureTile label="Agape keeps" value={money(take)} sub={f.in_fans ? `${(100 * take / f.in_fans).toFixed(1)}% of what fans paid` : ""} />
          </div>
          <section className="block"><h2>Payouts</h2>
            <ul className="earned">
              <li><span className="grow">{BRAND.venue} stands and venue</span><small>weekly</small><b className="num">{money((f.stands ?? 0) + (f.venue_partners ?? 0))}</b></li>
              <li><span className="grow">Runners</span><small>within minutes, or every Monday</small><b className="num">{money(f.runners ?? 0)}</b></li>
              <li><span className="grow">Sales tax</span><small>to the state, monthly</small><b className="num">{money(f.tax ?? 0)}</b></li>
            </ul>
          </section>
        </>}
        {tab === "people" && <>
          <header className="role-head"><h2 className="hq-h">Staff</h2><span className="sub">Who works at Agape and what they can do here</span></header>
          <section className="block settings-list">
            {[["Founder", "Full access"], ["Co-founder", "Full access"], ["Sam", "Operations · venues, support, crew"], ["Riley", "Finance · money and payouts"]].map(([a, b]) =>
              <div key={a} className="set-row static"><span className="me-av">{a[0]}</span><span className="grow"><b>{a}</b><small>{b}</small></span></div>)}
            <Invite toast={toast} />
          </section>
        </>}
      </div></div>
    </div>
  );
}

function beat(stage: Stage, o: OpsView | null, c: CareView | null): Beat {
  void c;
  if (stage !== "app") return { id: "a", title: "Agape's own team", body: <>This is the app you'd use if you worked at Agape: every venue at once, the support queue, the crew, partners, payouts and Agape's own money. It's separate from the apps venues and runners use.</> };
  if (o && o.problems.some(p => p.state.state === "open")) return { id: "p", title: "A claim waits for a person", body: <>Most problems are made right automatically by Agape's policy: as credit by default, paid by whoever was at fault. The rest wait in Care, with the photo and where the runner's phone was at the handoff. Agape delivers; a stand's food is the stand's to make right.</> };
  return { id: "b", title: "One team, many venues", body: <>You're on Agape's operations team. Harbor Arena is live; Mesa Dome and Riverside are examples of venues on the way, not real ones. Every number here comes from the same books as the venue's own, so there's nothing to reconcile.</> };
}

function App() {
  const [stage, setStage] = useState<Stage>("boot");
  const [o, setO] = useState<OpsView | null>(null);
  const [c, setC] = useState<CareView | null>(null);
  const [ready, setReady] = useState(false);
  const { toasts, toast } = useToasts();
  const refresh = () => { setO(kernel.view("ops") as OpsView); setC(kernel.view("care", "", "hq") as CareView); };
  useEffect(() => { (async () => {
    setWorldOverride(demoWorld());
    const x = await boot(23, "s104"); if (!x.ok) return;
    kernel.step(70 * 6); kernel.act({ do: "drive", party: "ops" }); refresh(); setReady(true);
    setTimeout(() => setStage("signin"), 1500);
  })(); }, []);
  return <Shell tag="HQ · Agape HQ" device="laptop" ready={ready} onTick={refresh} toasts={toasts} notes={NOTES} explainable={stage === "app"}
    beat={beat(stage, o, c)} screen={<HQ o={o} c={c} stage={stage} next={setStage} toast={toast} refresh={refresh} />} />;
}

applyLook(savedLook()); applyMode(savedMode());
ReactDOM.createRoot(document.getElementById("root")!).render(<App />);

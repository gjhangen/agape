// The visual pieces the front pages tell Agape's story with: the line fans walk away from, the product as three
// screens, how money moves, and where Agape goes. Each page frames them for its reader.
import { React } from "./r";

export const SRC = {
  mashgin: "https://business.ricentral.com/ricentral/article/gnwcq-2025-4-8-new-report-finds-long-concession-lines-at-mlb-games-cause-fans-to-miss-crucial-plays-vendors-to-lose-revenue",
  oracle: "https://prnewswire.com/news-releases/oracle-cloud-solves-undiscovered-stadium-revenue-growth-as-sports-fans-desire-mobile-food--beverage-ordering-300297035.html",
  ln: "https://news.livenationentertainment.com/news/live-nation-entertainment-full-year-and-fourth-quarter-2025-results",
  mlb: "https://frontofficesports.com/article/mlb-attendance-fourth-straight-year/",
};

const FIG = <svg viewBox="0 0 24 24"><circle cx="12" cy="7" r="4" /><path d="M4 22c0-5 3.6-8 8-8s8 3 8 8z" /></svg>;

/** 8 in 10 fans walk away from a line: the number, ten figures, and three supporting numbers. */
export function TheLine({ say, trio }: { say: string; trio?: [string, string][] }) {
  return <section className="iv2-line">
    <div className="iv2-line-in">
      <div className="iv2-big"><b>8<small>in</small>10</b><span>{say}</span></div>
      <div className="iv2-people" aria-hidden="true">{Array.from({ length: 10 }, (_, i) => <i key={i} className={i < 8 ? "gone" : ""}>{FIG}</i>)}</div>
      <div className="iv2-trio">{(trio ?? [["79%", "missed a key play in line"], ["77%", "would buy more with a shorter wait"], ["+43%", "more spent per game if waits were halved"]]).map(([b, s]) => <div key={b}><b>{b}</b><span>{s}</span></div>)}</div>
      <small className="iv2-src">Fans surveyed by <a href={SRC.mashgin}>Mashgin (2025)</a> and <a href={SRC.oracle}>Oracle</a>.</small>
    </div>
  </section>;
}

/** The product as three screens: the fan orders, the stand makes it, a runner brings it. */
export function Flow({ title, steps }: { title: string; steps?: [string, string, string, "ph" | "tb"][] }) {
  const s = steps ?? [["/img/fan-menu.jpg", "The fan orders", "and sees when it'll reach their seat", "ph"], ["/img/stand.jpg", "The stand makes it", "timed to leave on time", "tb"], ["/img/runner.jpg", "A runner brings it", "several orders on one route", "ph"]];
  return <section className="site-sec iv2-how">
    <h2>{title}</h2>
    <ol className="iv2-flow">{s.map(([img, b, line, k]) => <li key={b}><figure className={k}><img src={img} alt="" /></figure><b>{b}</b><span>{line}</span></li>)}</ol>
  </section>;
}

/** Fans pay Agape; Agape keeps its share and pays everyone else. */
export function Money({ kicker, title, line, chips, out }: { kicker: string; title: string; line: string; chips: React.ReactNode[]; out?: string[] }) {
  return <section className="iv2-money">
    <div className="iv2-money-in">
      <div className="iv2-money-copy"><span className="hm-kicker">{kicker}</span><h2>{title}</h2><p>{line}</p></div>
      <div className="iv2-diagram" aria-label="Fans pay Agape; Agape keeps its share and pays the stands, the runners and the tax">
        <div className="n fan">Fans pay</div>
        <div className="arrow" />
        <div className="n hub"><b>agape</b><span>keeps its share</span></div>
        <div className="fan-out">{(out ?? ["Stands", "Runners", "Tax"]).map(o => <div key={o} className="n">{o}</div>)}</div>
      </div>
      <ul className="iv2-earn">{chips.map((c, i) => <li key={i}>{c}</li>)}</ul>
    </div>
  </section>;
}

/** Where Agape goes, as five steps. Each step keeps everything before it and adds more, so the stack in each column
 * grows: what it brings in (investors), what the venue gains (venues), or what fans can do (everyone). Above each
 * column, how often fans open Agape at that step: once a game at launch, every day by the last. */
const STEPS = [
  { name: "Order to your seat", line: "Food, drinks and gear, brought to the seat", when: "Once a game" },
  { name: "Rewards and offers", line: "Points, and offers from sponsors and places nearby", when: "Every offer" },
  { name: "Play along", line: "Calls, section battles and prizes during the game", when: "All game long" },
  { name: "Tickets and travel", line: "Tickets, rides and parking in the same app", when: "Before and after" },
  { name: "All year", line: "Teams, players and friends between events", when: "Every day" },
];
const ADDS: Record<string, string[]> = {
  investor: ["Share of every sale", "Sponsors", "Sponsored prizes", "Booking fees", "A daily audience"],
  venue: ["More sales per fan", "Sponsor revenue", "Fans engaged all game", "Tickets and parking sold", "Fans all year"],
  fan: ["Order from your seat", "Points and perks", "Play for prizes", "One app for the night", "Your teams, all year"],
};
export function Steps({ kind, kicker, title, line, tryLinks }: { kind: "investor" | "venue" | "fan"; kicker: string; title: string; line: string; tryLinks?: boolean }) {
  const adds = ADDS[kind];
  return <section className="site-sec gr">
    <div className="sec-head"><span className="hm-kicker">{kicker}</span><h2>{title}</h2><p className="site-why-demo">{line}</p></div>
    <ol className="gr-cols">{STEPS.map((s, i) => <li key={s.name} className={i === 0 ? "now" : ""}>
      <span className="gr-when"><small>Fans open Agape</small>{s.when}</span>
      <div className="gr-stack">{adds.slice(0, i + 1).map((a, j) => <span key={a} className={"gr-chip" + (j === i ? " new" : "")} style={{ ["--j" as any]: j }}>{a}</span>)}</div>
      <div className="gr-name"><em>{i === 0 ? "At launch" : `Step ${i + 1}`}</em><b>{s.name}</b><small>{s.line}</small>
        {tryLinks && <a href={`/try/?level=${i + 1}`}>Try it <span aria-hidden="true">→</span></a>}</div>
    </li>)}</ol>
  </section>;
}

/** Real screens from the working apps. */
export function Built({ title, kicker }: { title: string; kicker: string }) {
  return <section className="site-sec iv2-built">
    <div className="sec-head"><span className="hm-kicker">{kicker}</span><h2>{title}</h2></div>
    <div className="iv2-collage">
      <figure className="c-stand"><img src="/img/stand.jpg" alt="" /><figcaption>Stand</figcaption></figure>
      <figure className="c-partner"><img src="/img/partner.jpg" alt="" /><figcaption>Partner</figcaption></figure>
      <figure className="c-fan ph"><img src="/img/fan-item.jpg" alt="" /><figcaption>Fan · ordering</figcaption></figure>
      <figure className="c-run ph"><img src="/img/fan-track.jpg" alt="" /><figcaption>Fan · tracking</figcaption></figure>
    </div>
    <div className="iv2-proof"><div><b>6 min</b><span>to the seat</span></div><div><b>3,000</b><span>fans at once</span></div><div><b>$0.00</b><span>off in the books</span></div><small>A simulated night at the demo arena</small></div>
  </section>;
}

export function Tiles({ title, items }: { title: string; items: [string, string, string][] }) {
  return <section className="site-sec iv2-wins">
    <h2>{title}</h2>
    <div className="iv2-tiles">{items.map(([i, b, s]) => <div key={b}><i>{i}</i><b>{b}</b><span>{s}</span></div>)}</div>
  </section>;
}

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

/** For investors: where Agape goes. One way in (food to the seat), which leaves Agape holding the moment, the money
 * path, the merchants and the fans; then four lines that run at once, each opening a bigger, sourced pool of money.
 * The numbers grow from card to card on purpose: the last line isn't limited to event nights. */
const WEB_SRC = {
  sponsor: "https://sportfive.com/beyond-the-match/insights/stability-shifts-and-the-new-shape-of-sports-marketing",
  merch: "https://fashionunited.com/news/business/licensed-merchandise-reports-continued-growth-in-2025/2026052072459",
  resale: "https://www.businesswire.com/news/home/20260304475774/en/StubHub-Announces-Full-Year-and-Fourth-Quarter-2025-Results",
  ads: "https://www.bia.com/press-releases/bia-2025-forecast-update-169-billion-in-u-s-local-advertising-and-2-4-year-over-year-decline/",
  cards: "https://nilsonreport.com/articles/merchant-processing-fees-in-the-united-states-2024/",
};
const LINES: { chain: string[]; how: string; pool: string; of: string; when: string }[] = [
  { chain: ["Sponsors"], how: "Prizes, samples and offers, handed to fans while they're buying.", pool: "$39B", of: "sponsorship, worldwide", when: "Event nights" },
  { chain: ["Wallet", "Marketplace"], how: "Fans' money and buyers already inside: resale, merch, collectibles, upgrades.", pool: "$80B", of: "sports merch, concert tickets and resale", when: "Every day" },
  { chain: ["Social", "Advertising"], how: "Fans follow teams, sections and friends between events, so brands reach them daily.", pool: "$169B", of: "US local advertising", when: "Every day" },
  { chain: ["Local deals", "Processing"], how: "Shown the customers Agape sent them, the businesses around every venue run their cards through Agape.", pool: "$11.9T", of: "US card purchases a year", when: "Every day, all year" },
];
export function Web() {
  return <section className="site-sec wb">
    <div className="sec-head"><span className="hm-kicker">Where it goes</span><h2>One way in. Four lines out.</h2>
      <p className="site-why-demo">Food to the seat puts Agape in the seat, in the money path and in front of every business nearby. From there, four lines run at once, and each reaches a bigger pool of money than the last.</p></div>
    <div className="wb-door"><div><em>The way in</em><b>Food to the seat</b><span>Fans pay Agape first. Agape pays the stands, the runners and the tax.</span></div>
      <ul aria-label="What it leaves Agape holding"><li>The moment</li><li>The money path</li><li>The merchants</li><li>The fans</li></ul></div>
    <ol className="wb-lines">{LINES.map((l, i) => <li key={l.pool} style={{ ["--i" as any]: i }} className={i === LINES.length - 1 ? "top" : ""}>
      <span className="wb-when">{l.when}</span>
      <div className="wb-chain">{l.chain.map((c, j) => <React.Fragment key={c}>{j > 0 && <i aria-hidden="true">→</i>}<b>{c}</b></React.Fragment>)}</div>
      <p>{l.how}</p>
      <div className="wb-pool"><strong>{l.pool}</strong><small>{l.of}</small></div>
    </li>)}</ol>
    <p className="ld-goal"><span>The goal</span><b>30% of all live-event spending flowing through Agape, and the businesses around every venue running on it every day.</b></p>
    <small className="iv2-src wb-src">Pools are yearly, 2025; card purchases 2024. <a href={WEB_SRC.sponsor}>GlobalData via SPORTFIVE</a>, <a href={WEB_SRC.merch}>Licensing International</a>, <a href={SRC.ln}>Live Nation</a>, <a href={WEB_SRC.resale}>StubHub</a>, <a href={WEB_SRC.ads}>BIA</a>, <a href={WEB_SRC.cards}>Nilson Report</a>.</small>
  </section>;
}

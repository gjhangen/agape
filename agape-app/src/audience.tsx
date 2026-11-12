// One page per kind of person Agape is for: what it means to them, and only the demos that matter to them.
// Served at /for/<who>/.
import { React, ReactDOM, useState } from "./r";
import { Wordmark } from "./logo";
import { TalkToUs } from "./talk";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";

type Demo = { href: string; who: string; line: string; device: string; time: string };
type Page = { eyebrow: string; title: string; lede: string; points: [string, string][]; demos: Demo[]; ask: string; role: string; more?: string; shot: string; wide?: boolean; proof?: boolean };

const D = {
  fan: { href: "/try/", who: "Be the fan", line: "Take your seat, order, and watch it reach you.", device: "Phone", time: "3 min" },
  runner: { href: "/try/runner/", who: "Be the runner", line: "Take trips with the route on a map and the pay up front.", device: "Phone", time: "2 min" },
  stand: { href: "/try/kitchen/", who: "Run the stand", line: "Orders land on one screen, timed, and leave on time.", device: "Tablet", time: "2 min" },
  venue: { href: "/try/venue/", who: "Run the venue", line: "Every order, runner and dollar, live, on one screen.", device: "Laptop", time: "3 min" },
  partner: { href: "/try/partner/", who: "Be the partner", line: "Fans unlock your perk and walk over after the event.", device: "Laptop", time: "2 min" },
  hq: { href: "/try/hq/", who: "Run Agape", line: "Every venue, the crew, the policies and the money.", device: "Laptop", time: "2 min" },
  tour: { href: "/demo/?tour=1", who: "Take the 2-minute tour", line: "A whole night at the arena, every role at once, with someone pointing out what matters.", device: "Computer", time: "2 min" },
  sim: { href: "/demo/", who: "Behind the scenes", line: "The whole arena at once, sped up: 3,000 fans, 40 runners, every dollar.", device: "Computer", time: "5 min" },
} satisfies Record<string, Demo>;

const PAGES: Record<string, Page> = {
  fans: { shot: "fan-menu", eyebrow: "For fans", title: "Never miss the moment for a line.", role: "other",
    lede: "Order food, drinks and gear from your seat at games and concerts. The venue's own kitchens make it, a runner brings it, and you keep watching.",
    points: [["Honest times", "Every item shows when it would reach your seat, measured from how fast each stand and runner is going right now."], ["Charged when it arrives", "Not when you order. If it's late, you get Agape credit automatically."], ["Your runner keeps every tip", "All of it goes to the person who brought it."]],
    demos: [D.fan], ask: "Want Agape at your venue? Tell us where you go." },
  runners: { shot: "runner", eyebrow: "For runners", title: "Earn at the events you love.", role: "runner",
    lede: "Run orders to the seats at games and concerts in your city. Pick the events you want, go online when you're there, and keep every tip.",
    points: [["The pay before the trip", "Every trip shows its pay up front, or you're paid by the hour where the venue runs that way."], ["The route on a map", "Pickups and seats in the shortest order, grouped so one walk delivers several orders."], ["Your events, your hours", "A shift is the event. Go online and off when you like, unless it's posted as a fixed shift."]],
    demos: [D.runner], ask: "Want to run at an Agape venue? Leave a way to reach you.", more: "Runners are independent and paid by Agape." },
  venues: { shot: "venue", wide: true, proof: true, eyebrow: "For venues and teams", title: "Sell more at every seat.", role: "venue",
    lede: "Fans order from their seats and your own kitchens make it. Shorter lines, more sales per fan, and the whole night on one screen.",
    points: [["Your kitchens, your brand", "Agape sends orders to the stands you already run and brings them to the seats with independent runners."], ["Every dollar, to the cent", "Sales, Agape's share, card processing, runner pay and tax, reconciled every night."], ["Start simple", "Launch with ordering to the seat. Add deals, games, tickets and community when you want them."]],
    demos: [D.venue, D.stand, D.fan], ask: "Bring Agape to your venue.", more: "Agape earns a share of what it sells and the delivery fee, so it only earns when you do." },
  stands: { shot: "stand", wide: true, eyebrow: "For food service and stands", title: "More orders, without the line.", role: "stand",
    lede: "Orders from every seat land on one screen at your counter, timed to your kitchen. Runners take them the moment they're ready.",
    points: [["One screen, timed", "Coming up, on the stations, on the shelf. Each order shows how long it should take."], ["Honest times protect you", "Fans see times built from your real pace, and busy stands get steered to quieter ones."], ["Problems made right fairly", "Delivery problems are Agape's. Food problems come to you, as credit you agree to, never quietly deducted."]],
    demos: [D.stand, D.fan], ask: "Run concessions? Talk to us." },
  partners: { shot: "fan-item", eyebrow: "For businesses and sponsors", title: "Reach fans when they care most.", role: "business",
    lede: "Put a perk in front of every fan at the event. They unlock it with the points they earn ordering, and you pay for the people who walk through your door.",
    points: [["Results, not impressions", "Every unlock and every redemption at your counter, live."], ["At the right moment", "Fans see your perk during the event and come to you after it."], ["Simple to run", "Check a code at the counter. That's the whole integration."]],
    demos: [{ ...D.partner, href: "/try/partner/" }, D.fan], ask: "Partner with Agape." },
  investors: { shot: "venue", wide: true, proof: true, more: "The 2-minute tour walks you through a whole night, every role at once.", eyebrow: "For investors", title: "The money every venue loses in line.", role: "investor",
    lede: "Most fans have walked away from a concession line without buying. Agape brings what they wanted to their seat, and turns every order into a relationship with the fan that follows them to every event on Agape.",
    points: [],
    demos: [D.tour, D.fan, D.hq], ask: "Talk to the founders" },
  team: { shot: "venue", wide: true, eyebrow: "For Agape's team", title: "Run many venues from one place.", role: "other",
    lede: "Agape HQ is where Agape's own people run the company: every venue at once, problems made right by policy, runner pay, and the money.",
    points: [["Policies, not tickets", "How problems are made right, who pays for what, tips, proof at the handoff and runner pay, set once for every venue."], ["Evidence at hand", "Photos, where the runner's phone was at the handoff, and every fan's record, for Agape's eyes only."], ["One set of books", "The same ledger every party sees, so there's nothing to reconcile."]],
    demos: [D.hq, D.sim, D.venue], ask: "Questions? Talk to us." },
};

const AUDIENCES: [string, string][] = [["fans", "Fans"], ["runners", "Runners"], ["venues", "Venues"], ["stands", "Food service"], ["partners", "Partners"], ["investors", "Investors"], ["team", "Agape team"]];


const SRC = {
  mashgin: "https://business.ricentral.com/ricentral/article/gnwcq-2025-4-8-new-report-finds-long-concession-lines-at-mlb-games-cause-fans-to-miss-crucial-plays-vendors-to-lose-revenue",
  oracle: "https://prnewswire.com/news-releases/oracle-cloud-solves-undiscovered-stadium-revenue-growth-as-sports-fans-desire-mobile-food--beverage-ordering-300297035.html",
  ln: "https://news.livenationentertainment.com/news/live-nation-entertainment-full-year-and-fourth-quarter-2025-results",
  mlb: "https://frontofficesports.com/article/mlb-attendance-fourth-straight-year/",
};

/** The investor page, told mostly in pictures: the line, the product, how money moves, where it goes, what exists. */
function Invest() {
  const people = Array.from({ length: 10 }, (_, i) => i < 8);
  return <>
    <section className="iv2-line">
      <div className="iv2-line-in">
        <div className="iv2-big"><b>8<small>in</small>10</b><span>fans have walked away from a concession line without buying.</span></div>
        <div className="iv2-people" aria-hidden="true">{people.map((gone, i) => <i key={i} className={gone ? "gone" : ""}><svg viewBox="0 0 24 24"><circle cx="12" cy="7" r="4" /><path d="M4 22c0-5 3.6-8 8-8s8 3 8 8z" /></svg></i>)}</div>
        <div className="iv2-trio">
          <div><b>79%</b><span>missed a key play in line</span></div>
          <div><b>77%</b><span>would buy more with a shorter wait</span></div>
          <div><b>+43%</b><span>more spent per game if waits were halved</span></div>
        </div>
        <small className="iv2-src">Fans surveyed by <a href={SRC.mashgin}>Mashgin (2025)</a> and <a href={SRC.oracle}>Oracle</a>.</small>
      </div>
    </section>

    <section className="site-sec iv2-how">
      <h2>Agape takes the line away</h2>
      <ol className="iv2-flow">
        <li><figure className="ph"><img src="/img/fan-menu.jpg" alt="The fan's app" /></figure><b>The fan orders</b><span>and sees when it'll reach their seat</span></li>
        <li><figure className="tb"><img src="/img/stand.jpg" alt="The stand's tablet" /></figure><b>The stand makes it</b><span>timed to leave on time</span></li>
        <li><figure className="ph"><img src="/img/runner.jpg" alt="The runner's app" /></figure><b>A runner brings it</b><span>several orders on one route</span></li>
      </ol>
    </section>

    <section className="iv2-money">
      <div className="iv2-money-in">
        <div className="iv2-money-copy"><span className="hm-kicker">How the money moves</span><h2>Every dollar flows through Agape</h2><p>Agape keeps its share first, then pays everyone else. Nobody is ever collected from.</p></div>
        <div className="iv2-diagram" aria-label="Fans pay Agape; Agape keeps its share and pays the stands, the runners and the tax">
          <div className="n fan">Fans pay</div>
          <div className="arrow" />
          <div className="n hub"><b>agape</b><span>keeps its share</span></div>
          <div className="fan-out"><div className="n">Stands</div><div className="n">Runners</div><div className="n">Tax</div></div>
        </div>
        <ul className="iv2-earn"><li><b>A share</b> of every sale</li><li><b>A delivery fee</b> on every order</li><li><b>Sponsors, deals and tickets</b> as each level switches on</li></ul>
      </div>
    </section>

    <section className="site-sec iv2-levels">
      <div className="sec-head"><span className="hm-kicker">Where it goes</span><h2>Food is the way in</h2></div>
      <ol className="iv2-stairs">{LEVELS_IV.map(([n, name, line], i) => <li key={n} style={{ ["--h" as any]: `${40 + i * 15}%` }} className={i === 0 ? "now" : ""}>
        <span className="st-tag">{i === 0 ? "Launch" : `Level ${n}`}</span><b>{name}</b><small>{line}</small></li>)}</ol>
      <p className="iv2-note">A fan's level, points and history follow them to every venue on Agape. Every level is built and working in the demo.</p>
    </section>

    <section className="iv2-market">
      <div><b>159M</b><span>fans at Live Nation events alone, 2025</span></div>
      <div><b>71.75M</b><span>at Major League Baseball games, 2026</span></div>
      <div className="txt"><span>Games, concerts, college sports and festivals. Built for any live event, with our first venues likely in Kentucky.</span><small><a href={SRC.ln}>Live Nation</a> · <a href={SRC.mlb}>Front Office Sports</a></small></div>
    </section>

    <section className="site-sec iv2-wins">
      <h2>Why Agape wins</h2>
      <div className="iv2-tiles">
        <div><i>⏱</i><b>Times that come true</b><span>Measured live from every kitchen and runner</span></div>
        <div><i>↗</i><b>Delivery that pays</b><span>Built around deliveries per runner hour</span></div>
        <div><i>≡</i><b>One set of books</b><span>Every party, one ledger, to the cent</span></div>
        <div><i>✓</i><b>Live in an afternoon</b><span>A form, not an integration project</span></div>
      </div>
    </section>

    <section className="site-sec iv2-built">
      <div className="sec-head"><span className="hm-kicker">Built, not slides</span><h2>Six apps, working today</h2></div>
      <div className="iv2-collage">
        <figure className="c-stand"><img src="/img/stand.jpg" alt="" /><figcaption>Stand</figcaption></figure>
        <figure className="c-partner"><img src="/img/partner.jpg" alt="" /><figcaption>Partner</figcaption></figure>
        <figure className="c-fan ph"><img src="/img/fan-item.jpg" alt="" /><figcaption>Fan · ordering</figcaption></figure>
        <figure className="c-run ph"><img src="/img/fan-track.jpg" alt="" /><figcaption>Fan · tracking</figcaption></figure>
      </div>
      <div className="iv2-proof"><div><b>6 min</b><span>to the seat</span></div><div><b>3,000</b><span>fans at once</span></div><div><b>$0.00</b><span>off in the books</span></div><small>A simulated night at the demo arena</small></div>
    </section>

    <section className="site-sec iv2-team">
      <h2>The founders</h2>
      <div className="iv2-founders">
        <div><b>Phillip Morgan</b><span>Co-founder · Deals and operations</span></div>
        <div><b>Gabriel Hangen</b><span>Co-founder · Technology</span><small>Designed and built Agape's technology</small></div>
      </div>
    </section>
  </>;
}

const LEVELS_IV: [number, string, string][] = [[1, "Order to your seat", "Food, drinks and gear"], [2, "Deals", "Points, offers, sponsors"], [3, "Play", "Calls, battles, collectibles"], [4, "Beyond the night", "Tickets, rides, places nearby"], [5, "Community", "Sections, profiles, following"]];

function Audience() {
  const key = location.pathname.split("/").filter(Boolean)[1] ?? "fans";
  const p = PAGES[key] ?? PAGES.fans;
  const [talk, setTalk] = useState(false);
  const [kind, setKind] = useState<"game" | "concert">("game");
  const q = (href: string) => kind === "concert" && href.startsWith("/try/") ? `${href}?event=concert` : href;
  return (
    <div className="site hm">
      <header className="site-top">
        <a className="site-mark" href="/" aria-label="Agape home"><Wordmark tag /></a>
        <nav><a href="/">Home</a><a className="btn sm" href="#demo">Try it</a><button className="btn sm primary" onClick={() => setTalk(true)}>Talk to us</button></nav>
      </header>
      <section className={"hm-hero for" + (p.wide ? " wide" : "")}>
        <div className="hm-copy">
          <span className="hm-kicker">{p.eyebrow}</span>
          <h1>{p.title}</h1>
          <p className="lede">{p.lede}</p>
          <div className="site-acts"><a className="btn primary big" href={q(p.demos[0].href)}>{p.demos[0].who} <b aria-hidden="true">→</b></a>
            {key === "investors" ? <a className="btn big hm-ghost" href="/agape-summary.pdf" download="Agape investor summary.pdf">Download the summary (PDF)</a> : <button className="btn big hm-ghost" onClick={() => setTalk(true)}>{p.ask}</button>}</div>
          {p.more && <small className="hero-note">{p.more}</small>}
        </div>
        <figure className={"hm-shot " + (p.wide ? "k-wide" : "k-phone")}><img src={`/img/${p.shot}.jpg`} alt="" /></figure>
      </section>
      {p.proof && key !== "investors" && <section className="hm-proof" aria-label="A simulated night at Harbor Arena">
        <div><b>6 min</b><span>average from order to seat</span></div>
        <div><b>90%</b><span>at the seat within 10 minutes</span></div>
        <div><b>3,000</b><span>fans ordering at once</span></div>
        <div><b>$0.00</b><span>off in the books, every night</span></div>
        <small>Measured on a simulated night at Harbor Arena: 3,000 fans, 40 runners, 8 stands.</small>
      </section>}
      {key === "investors" ? <Invest /> : <section className="site-sec">
        <ol className="for-points">{p.points.map(([a, b]) => <li key={a}><b>{a}</b><span>{b}</span></li>)}</ol>
      </section>}
      {key !== "investors" && <section className="site-sec" id="demo">
        <div className="sec-head"><h2>See it for yourself</h2><p className="site-why-demo">Agape isn't at a venue yet, so we built a whole night you can step into. These are the parts that matter to you.</p></div>
        <div className="site-seats">{p.demos.map(d => <a key={d.href} className="site-seat" href={q(d.href)}>
          <span className="ss-top"><span className="ss-who">{d.who}</span></span><span className="ss-line">{d.line}</span>
          <span className="ss-foot"><span className="ss-meta">{d.device} · {d.time}</span><span className="ss-go">Open <b aria-hidden="true">→</b></span></span></a>)}</div>
      </section>}
      <section className="site-cta">
        <div><h2>{p.ask}</h2><p>Leave a way to reach you and we'll get back to you within a day.</p></div>
        <div className="site-acts"><button className="btn primary big" onClick={() => setTalk(true)}>Talk to us <b aria-hidden="true">→</b></button><a className="btn big" href={q(p.demos[0].href)}>{p.demos[0].who}</a></div>
      </section>
      <footer className="site-foot">
        <span>Agape · to your seat</span>
        {AUDIENCES.filter(([k]) => k !== key).map(([k, l]) => <a key={k} href={`/for/${k}/`}>{l}</a>)}
        <a href="/privacy/">Privacy</a><a href="/terms/">Terms</a>
      </footer>
      {talk && <TalkToUs close={() => setTalk(false)} version={null} role={p.role} />}
    </div>
  );
}

applyLook(savedLook()); applyMode(savedMode());
ReactDOM.createRoot(document.getElementById("root")!).render(<Audience />);

// agapetoyourseat.com: what Agape is, why there's a demo, and a way into it from every seat.
import { React, ReactDOM, useEffect, useState } from "./r";
import { TalkToUs } from "./talk";
import { LEVELS } from "./parts";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";

type Glance = { home: string; away: string; home_score: number; away_score: number; label: string } | null;

const SEATS: { id: string; who: string; line: string; time: string; href: string; ready: boolean }[] = [
  { id: "fan", who: "Fan", line: "Open the app, take your seat, order a hot dog and watch it reach you.", time: "3 min", href: "/try/", ready: true },
  { id: "runner", who: "Runner", line: "See trips offered with the pay up front, then carry them to the seats.", time: "2 min", href: "/try/runner/", ready: true },
  { id: "kitchen", who: "Kitchen", line: "Watch orders land on the stand's screen and leave on time.", time: "2 min", href: "/try/kitchen/", ready: true },
  { id: "venue", who: "Venue", line: "Run the whole night: every order, every runner, every dollar.", time: "3 min", href: "/try/venue/", ready: true },
  { id: "partner", who: "Partner", line: "Reach fans in the moment, and see exactly what it brought in.", time: "2 min", href: "/try/partner/", ready: true },
  { id: "hq", who: "Agape HQ", line: "Where Agape's own team runs every venue, the crew and the money.", time: "2 min", href: "/try/hq/", ready: true },
];

function Site() {
  const [talk, setTalk] = useState(false);
  const [g, setG] = useState<Glance>(null);
  useEffect(() => { fetch("/api/v/harbor/glance").then(r => r.ok ? r.json() : null).then(x => x && setG({ home: x.home?.abbr ?? "", away: x.away?.abbr ?? "", home_score: x.home?.score ?? 0, away_score: x.away?.score ?? 0, label: x.clock?.label ?? "" })).catch(() => {}); }, []);
  void g;
  return (
    <div className="site">
      <header className="site-top">
        <span className="site-mark"><b>agape</b><span>to your seat</span></span>
        <nav><a href="#how">How it works</a><a href="#demo">Try it</a><button className="btn sm primary" onClick={() => setTalk(true)}>Talk to us</button></nav>
      </header>

      <section className="site-hero">
        <h1>Order from your seat.<br />It comes to you.</h1>
        <p className="lede">Agape brings food, drinks and gear to your seat at games, concerts and every live event, made by the venue's own kitchens and carried by runners, so you never miss the moment you came for.</p>
        <div className="site-acts"><a className="btn primary big" href="/try/">See it as a fan</a><a className="btn big" href="#demo">Choose another seat</a></div>
      </section>

      <section className="site-sec" id="how">
        <h2>How it works</h2>
        <ol className="site-steps">
          <li><b>Open Agape at the event.</b><span>It knows the venue. Tell it your seat, or scan your ticket.</span></li>
          <li><b>See when it reaches you, then order.</b><span>Every time shown is when it gets to your seat, measured from how fast each kitchen and runner is going right now.</span></li>
          <li><b>Stay in the moment.</b><span>The kitchen makes it, a runner brings it, and you're charged when it arrives. Your runner keeps every tip.</span></li>
        </ol>
      </section>

      <section className="site-sec">
        <h2>Why it matters</h2>
        <div className="site-why">
          <div><b>Fans</b><span>Never miss the play, the song or the moment for a line.</span></div>
          <div><b>Venues</b><span>Sell more at every seat, serve faster, see the whole night as it happens.</span></div>
          <div><b>Runners</b><span>Work on your own schedule, see the pay before you take a trip, keep every tip.</span></div>
          <div><b>Partners</b><span>Reach people at the moment they care most, and see what it brought in.</span></div>
        </div>
      </section>

      <section className="site-sec" id="demo">
        <h2>Step into a night</h2>
        <p className="site-why-demo">Agape isn't at a venue near you yet, so we built a whole night you can step into: a full arena, a game in progress and thousands of fans, running in your browser. Choose a seat. Each one is a short story about one person's night.</p>
        <div className="site-seats">
          {SEATS.map(s => (
            <a key={s.id} className={"site-seat" + (s.id === "fan" ? " ready" : "")} href={s.href}>
              <span className="ss-who">{s.who}</span><span className="ss-line">{s.line}</span>
              <span className="ss-meta">{s.time}</span>
            </a>
          ))}
        </div>
        <a className="site-behind" href="/demo/"><b>Behind the scenes</b><span>Every seat at once: the whole arena, sped up, for anyone who wants to see all of it.</span></a>
      </section>

      <section className="site-sec">
        <h2>Where Agape is headed</h2>
        <p className="site-why-demo">Agape launches doing one thing completely: ordering to your seat. Each level after it is a complete product of its own. The fan demo lets you switch between them.</p>
        <ol className="site-levels">{LEVELS.map(l => <li key={l.n}><b>Level {l.n} · {l.name}</b><span>{l.line}</span></li>)}</ol>
      </section>

      <footer className="site-foot">
        <span>Agape · to your seat</span>
        <button className="link-inline" onClick={() => setTalk(true)}>Talk to us</button>
        <a href="/app/">Live pilot</a>
      </footer>
      {talk && <TalkToUs close={() => setTalk(false)} version={null} />}
    </div>
  );
}

applyLook(savedLook()); applyMode(savedMode());
ReactDOM.createRoot(document.getElementById("root")!).render(<Site />);

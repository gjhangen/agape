// agapetoyourseat.com: what Agape is, why there's a demo, and a way into it from every seat.
import { React, ReactDOM, useEffect, useState } from "./r";
import { Wordmark } from "./logo";
import { TalkToUs } from "./talk";
import { Icon, LEVELS } from "./parts";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";


type Seat = { id: string; who: string; icon: Parameters<typeof Icon>[0]["name"]; line: string; time: string; href: string; device: string };
const SEATS: Seat[] = [
  { id: "fan", who: "Fan", icon: "me", line: "Open the app, take your seat, order, and watch it reach you.", time: "3 min", href: "/try/", device: "Phone" },
  { id: "runner", who: "Runner", icon: "cash", line: "Trips offered with the pay up front, carried to the seats.", time: "2 min", href: "/try/runner/", device: "Phone" },
  { id: "kitchen", who: "Stand", icon: "get", line: "Orders land on the stand's screen and leave on time.", time: "2 min", href: "/try/kitchen/", device: "Tablet" },
  { id: "venue", who: "Venue", icon: "chart", line: "The whole night: every order, every runner, every dollar.", time: "3 min", href: "/try/venue/", device: "Laptop" },
  { id: "partner", who: "Partner", icon: "store", line: "Reach fans in the moment, and see what it brought in.", time: "2 min", href: "/try/partner/", device: "Laptop" },
  { id: "hq", who: "Agape HQ", icon: "home", line: "Agape's own team: every venue, the crew and the money.", time: "2 min", href: "/try/hq/", device: "Laptop" },
];

/** What the fan sees, drawn small: one order on its way to a seat. */
function OrderGlance() {
  const steps = ["Ordered", "Being made", "On the way", "At your seat"];
  const [i, setI] = useState(0);
  useEffect(() => { const t = setInterval(() => setI(x => (x + 1) % (steps.length + 1)), 1600); return () => clearInterval(t); }, []);
  const at = Math.min(i, steps.length - 1);
  return (
    <div className="glance" aria-hidden="true">
      <div className="glance-head"><span className="eyebrow">Section 104 · Row F · Seat 7</span><b>Hot dog</b><small>{at === steps.length - 1 ? <>At your seat, <b>7:41 PM</b></> : <>At your seat by <b>7:42 PM</b></>}</small></div>
      <ol className="glance-steps">{steps.map((x, k) => <li key={x} className={k < at ? "done" : k === at ? "now" : ""}><i />{x}</li>)}</ol>
      <div className="glance-foot"><span>Charged when it arrives</span><span>Your runner keeps every tip</span></div>
    </div>
  );
}

function Site() {
  const [talk, setTalk] = useState(() => location.hash === "#talk");
  const [kind, setKind] = useState<"game" | "concert">("game");
  // Talk to us is its own page in the history: Back closes it
  const open = () => { setTalk(true); history.pushState({ talk: 1 }, "", "#talk"); };
  const close = () => { if (location.hash === "#talk" && history.state?.talk) history.back(); else { setTalk(false); history.replaceState(null, "", location.pathname); } };
  useEffect(() => { const f = () => setTalk(location.hash === "#talk"); addEventListener("popstate", f); return () => removeEventListener("popstate", f); }, []);
  const q = kind === "concert" ? "?event=concert" : "";
  return (
    <div className="site">
      <header className="site-top">
        <span className="site-mark"><Wordmark tag /></span>
        <nav><a href="#how">How it works</a><a className="btn sm" href="#demo">Try the demo</a><button className="btn sm primary" onClick={open}>Talk to us</button></nav>
      </header>

      <section className="site-hero">
        <div className="hero-copy">
          <h1>Order from your seat.<br />It comes to you.</h1>
          <p className="lede">Food, drinks and gear at games, concerts and every live event, made by the venue's own kitchens and carried to your seat, so you never miss the moment you came for.</p>
          <div className="site-acts"><a className="btn primary big" href="#demo">Try the demo <b aria-hidden="true">↓</b></a><button className="btn big" onClick={open}>Bring Agape to your venue</button></div>
          <small className="hero-note">No sign-up. Runs in your browser, on your phone or computer.</small>
        </div>
        <OrderGlance />
      </section>

      <section className="site-sec who">
        <h2>Who are you?</h2>
        <div className="who-grid">{([["fans", "A fan", "Order from your seat"], ["runners", "A runner", "Earn at live events"], ["venues", "A venue or team", "Sell more at every seat"], ["stands", "Food service", "More orders, no line"], ["partners", "A business or sponsor", "Reach fans in the moment"], ["investors", "An investor", "How Agape grows"], ["team", "On Agape's team", "Run many venues"]] as const).map(([k, a, b]) =>
          <a key={k} className="who-card" href={`/for/${k}/`}><b>{a}</b><span>{b}</span><i aria-hidden="true">→</i></a>)}</div>
      </section>

      <section className="site-sec" id="how">
        <h2>How it works</h2>
        <ol className="site-steps">
          <li><b>Take your seat</b><span>Agape knows the venue. Scan your ticket or type your seat.</span></li>
          <li><b>See when it reaches you</b><span>Every time shown is when it gets to your seat, measured from how fast each kitchen and runner is going right now.</span></li>
          <li><b>Stay in the moment</b><span>The kitchen makes it, a runner brings it, and you're charged when it arrives.</span></li>
        </ol>
      </section>

      <section className="site-sec">
        <h2>Good for everyone in the building</h2>
        <div className="site-why">
          {([["me", "Fans", "Never miss the play, the song or the moment for a line."], ["chart", "Venues", "Sell more at every seat, serve faster, see the whole night live."], ["cash", "Runners", "Your own schedule, the pay shown before you take a trip, every tip kept."], ["store", "Partners", "Reach people when they care most, and see what it brought in."]] as const).map(([ic, who, line]) =>
            <div key={who}><span className="why-ic"><Icon name={ic} /></span><b>{who}</b><span>{line}</span></div>)}
        </div>
      </section>

      <section className="site-sec" id="demo">
        <div className="sec-head">
          <h2>Step into a night</h2>
          <p className="site-why-demo">Agape isn't at a venue yet, so we built a whole night you can walk into: a full arena and thousands of fans, running in your browser. Pick a role; each is a short story of one person's night.</p>
        </div>
        <div className="demo-pick">
          <div className="seg kind" role="group" aria-label="Kind of night">{(["game", "concert"] as const).map(k => <button key={k} className={kind === k ? "on" : ""} aria-pressed={kind === k} onClick={() => setKind(k)}>{k === "game" ? "A game" : "A concert"}</button>)}</div>
        </div>
        <div className="site-seats">
          {SEATS.map(x => (
            <a key={x.id} className="site-seat" href={x.href + q}>
              <span className="ss-top"><span className="ss-ic"><Icon name={x.icon} /></span><span className="ss-who">{x.who}</span></span>
              <span className="ss-line">{x.line}</span>
              <span className="ss-foot"><span className="ss-meta">{x.device} · {x.time}</span><span className="ss-go">Try it as {x.who === "Agape HQ" ? "HQ" : x.who.toLowerCase()} <b aria-hidden="true">→</b></span></span>
            </a>
          ))}
        </div>
        <a className="site-behind" href={"/demo/" + q}><span><b>Behind the scenes</b><span>Every role at once: the whole arena, sped up.</span></span><span className="sb-go">Open it <b aria-hidden="true">→</b></span></a>
      </section>

      <section className="site-sec">
        <div className="sec-head">
          <h2>Where Agape is headed</h2>
          <p className="site-why-demo">Agape launches doing one thing completely: ordering to your seat. Each level after it is a complete product of its own, and the fan demo lets you switch between them.</p>
        </div>
        <ol className="site-levels">{LEVELS.map(l => <li key={l.n} className={"lv-card s-" + l.status.toLowerCase().replace(" ", "-")}>
          <span className="lv-n">{l.n}</span>
          <b>{l.name}</b>
          <span>{l.line}</span>
          <em className={"lv-tag lv-" + l.status.toLowerCase().replace(" ", "-")}>{l.status}</em>
          <details><summary>What works, and what's next</summary><small>{l.gaps}</small></details>
          <a className="lv-go" href={`/try/?level=${l.n}${kind === "concert" ? "&event=concert" : ""}`}>Try Level {l.n} <b aria-hidden="true">→</b></a>
        </li>)}</ol>
      </section>

      <section className="site-cta">
        <div><h2>See it for yourself</h2><p>Three minutes as a fan is the fastest way to understand Agape. If you run a venue, partner with one or invest, talk to us.</p></div>
        <div className="site-acts"><a className="btn primary big" href={"/try/" + q}>Start as a fan <b aria-hidden="true">→</b></a><button className="btn big" onClick={open}>Talk to us</button></div>
      </section>

      <footer className="site-foot">
        <span>Agape · to your seat</span>
        <button className="link-inline" onClick={open}>Talk to us</button>
      </footer>
      {talk && <TalkToUs close={close} version={null} />}
    </div>
  );
}

applyLook(savedLook()); applyMode(savedMode());
ReactDOM.createRoot(document.getElementById("root")!).render(<Site />);

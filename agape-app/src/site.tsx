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
    <div className="site hm">
      <header className="site-top hm-top">
        <a className="site-mark" href="/"><Wordmark tag /></a>
        <nav><a href="/for/venues/">Venues</a><a href="/for/investors/">Investors</a><a href="#demo">Demo</a><button className="btn sm primary" onClick={open}>Talk to us</button></nav>
      </header>

      <section className="hm-hero">
        <div className="hm-copy">
          <span className="hm-kicker">In-seat ordering for live events</span>
          <h1>Order from your seat.<br />It comes to you.</h1>
          <p className="lede">Food, drinks and gear at games, concerts and every live event, made by the venue's own kitchens and carried to your seat, so you never miss the moment you came for.</p>
          <div className="site-acts"><a className="btn primary big" href={"/try/" + q}>Try it as a fan <b aria-hidden="true">→</b></a><button className="btn big hm-ghost" onClick={open}>Bring Agape to your venue</button></div>
          <small className="hero-note">No sign-up. Runs in your browser, on your phone or computer.</small>
        </div>
        <div className="hm-art" aria-hidden="true">
          <figure className="hm-phone back"><img src="/img/fan-item.jpg" alt="" /></figure>
          <figure className="hm-phone"><img src="/img/fan-menu.jpg" alt="" /></figure>
          <OrderGlance />
        </div>
      </section>

      <section className="hm-proof" aria-label="A simulated night at Harbor Arena">
        <div><b>6 min</b><span>average from order to seat</span></div>
        <div><b>90%</b><span>at the seat within 10 minutes</span></div>
        <div><b>3,000</b><span>fans ordering at once</span></div>
        <div><b>$0.00</b><span>off in the books, every night</span></div>
        <small>Measured on a simulated night at Harbor Arena: 3,000 fans, 40 runners, 8 stands.</small>
      </section>

      <section className="site-sec" id="how">
        <h2>How it works</h2>
        <ol className="site-steps">
          <li><b>Take your seat</b><span>Agape knows the venue. Scan your ticket or type your seat.</span></li>
          <li><b>See when it reaches you</b><span>Every time shown is when it gets to your seat, measured from how fast each kitchen and runner is going right now.</span></li>
          <li><b>Stay in the moment</b><span>The kitchen makes it, a runner brings it, and you're charged when it arrives.</span></li>
        </ol>
      </section>

      <section className="site-sec hm-roles" id="demo">
        <div className="sec-head">
          <h2>One night, every seat in the building</h2>
          <p className="site-why-demo">Agape isn't at a venue yet, so we built a whole night you can walk into: a full arena and thousands of fans, running in your browser. Step into anyone's night.</p>
          <div className="seg kind" role="group" aria-label="Kind of night">{(["game", "concert"] as const).map(k => <button key={k} className={kind === k ? "on" : ""} aria-pressed={kind === k} onClick={() => setKind(k)}>{k === "game" ? "A game" : "A concert"}</button>)}</div>
        </div>
        {([
          ["fan", "The fan", "Never miss the play for a line.", "Everything shows when it would reach your seat, before you tap. Order, keep watching, and it arrives. Charged only when it does.", "/try/", "/img/fan-track.jpg", "phone", "/for/fans/"],
          ["runner", "The runner", "Earn at live events, on your own schedule.", "Every trip is offered with its route and pay up front. Take it or pass. Every tip is the runner's.", "/try/runner/", "/img/runner.jpg", "phone", "/for/runners/"],
          ["stand", "The stand", "More orders, no line.", "Orders land on the stand's screen in the order they need to leave. The kitchen marks them ready and a runner is already on the way.", "/try/kitchen/", "/img/stand.jpg", "wide", "/for/stands/"],
          ["venue", "The venue", "Sell more at every seat, and see the whole night live.", "Every order, every runner, every problem and every dollar, as it happens. The books balance to the cent.", "/try/venue/", "/img/venue.jpg", "wide", "/for/venues/"],
        ] as const).map(([id, who, head, line, href, img, kind2, more], i) => (
          <article key={id} className={"hm-row" + (i % 2 ? " flip" : "") + " k-" + kind2}>
            <div className="hm-row-copy">
              <span className="hm-kicker">{who}</span>
              <h3>{head}</h3>
              <p>{line}</p>
              <div className="hm-row-acts"><a className="btn primary" href={href + q}>Try it as {id === "stand" ? "a stand" : id === "venue" ? "the venue" : "a " + id} <b aria-hidden="true">→</b></a><a className="link-inline" href={more}>Why it works for {id === "fan" ? "fans" : id + "s"}</a></div>
            </div>
            <figure className={"hm-shot k-" + kind2}><img src={img} alt={`The ${id} screen in the Agape demo`} loading="lazy" /></figure>
          </article>
        ))}
        <div className="hm-more">
          <a href={"/try/partner/" + q}><b>Partners and sponsors</b><span>Reach fans in the moment, and see what it brought in.</span></a>
          <a href={"/try/hq/" + q}><b>Agape HQ</b><span>Every venue, the crew and the money, in one place.</span></a>
          <a className="dark" href={"/demo/" + q}><b>Behind the scenes</b><span>Every role at once: the whole arena, sped up.</span></a>
        </div>
      </section>

      <section className="site-sec" id="levels">
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

      <footer className="site-foot hm-foot">
        <span>Agape · to your seat</span>
        <nav>{([["fans", "Fans"], ["runners", "Runners"], ["venues", "Venues"], ["stands", "Food service"], ["partners", "Partners"], ["investors", "Investors"], ["team", "Agape team"]] as const).map(([k, l]) => <a key={k} href={`/for/${k}/`}>{l}</a>)}</nav>
        <button className="link-inline" onClick={open}>Talk to us</button>
      </footer>
      {talk && <TalkToUs close={close} version={null} />}
    </div>
  );
}

applyLook(savedLook()); applyMode(savedMode());
ReactDOM.createRoot(document.getElementById("root")!).render(<Site />);

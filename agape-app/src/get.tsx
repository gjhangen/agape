// agapetoyourseat.com/get: putting Agape on a phone. The app itself first (fans sign up; staff sign in with the
// account their venue gives them), then each role's demo, which installs as its own app too.
import { React, ReactDOM } from "./r";
import { Wordmark } from "./logo";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";
import { InstallSteps, device } from "./install";

const DEMOS: [string, string, string, string][] = [
  ["", "Agape", "The fan", "Take a seat at Harbor Arena, order, and watch it reach you."],
  ["runner", "Agape Crew", "The runner", "Trips offered with the pay up front, carried to the seats."],
  ["kitchen", "Agape Station", "The stand", "Orders land on the stand's screen and leave on time."],
  ["venue", "Agape Venue", "The venue", "Every order, every runner and every dollar of the night."],
  ["partner", "Agape Partner", "Partners and sponsors", "Reach fans in the moment, and see what it brought in."],
  ["hq", "Agape HQ", "Agape's own team", "Every venue, the crew and the money, in one place."],
];

function Get() {
  const d = device();
  const here = location.origin + "/get/";
  return <div className="site hm">
    <header className="site-top hm-top">
      <a className="site-mark" href="/"><Wordmark tag /></a>
      <nav><a href="/">Home</a><a className="btn sm" href="/try/">Try it</a></nav>
    </header>
    <section className="gt-hero">
      <span className="hm-kicker">On your phone</span>
      <h1>Get Agape</h1>
      <p className="lede">No app store needed. Agape installs straight from the web: its own icon on your home screen, full screen, and always the latest version.</p>
    </section>
    <section className="gt-main">
      <div className="gt-app">
        <img src="/icon-192.png" alt="" width="88" height="88" />
        <div><h2>Agape</h2><p>Fans sign up free and order to their seat. Runners, stands and venue staff sign in with the account their venue gives them.</p></div>
      </div>
      <InstallSteps name="Agape" link={here} />
      <a className="link-inline gt-open" href="/app/">{d.installed ? "Open Agape" : "Or open it in the browser"} <b aria-hidden="true">→</b></a>
    </section>
    <section className="site-sec gt-demos">
      <div className="sec-head"><span className="hm-kicker">The demo, as apps</span><h2>Every role, on your home screen</h2>
        <p className="site-why-demo">Agape isn't at a venue yet, so each role's night is a demo you can install as its own app, and try the way the people in that role will use it.</p></div>
      <div className="gt-grid">{DEMOS.map(([dir, name, who, line]) => <a key={name} className="gt-demo" href={`/try/${dir ? dir + "/" : ""}?install=1`}>
        <img src="/icon-192.png" alt="" width="52" height="52" />
        <span><small>{who}</small><b>{name}</b><span>{line}</span></span>
        <em>{d.phone ? "Open and install" : "Open"} <b aria-hidden="true">→</b></em>
      </a>)}</div>
    </section>
    <footer className="site-foot hm-foot">
      <span>Agape · to your seat</span>
      <nav><a href="/">Home</a><a href="/for/venues/">Venues</a><a href="/for/investors/">Investors</a><a href="/privacy/">Privacy</a><a href="/terms/">Terms</a></nav>
    </footer>
  </div>;
}

applyLook(savedLook()); applyMode(savedMode());
ReactDOM.createRoot(document.getElementById("root")!).render(<Get />);

// Explain mode: a layer the demo puts over a device, never part of the app. With it on, the parts worth
// explaining glow; tapping one says what it is, who it's for, why Agape builds it and how it earns, and its level.
import { React, useEffect, useState } from "./r";

export type Note = { sel: string; title: string; what: string; who: string; why: string; level: number };

export const FAN_NOTES: Note[] = [
  { sel: ".anchor", title: "The event, in one line", what: "The score and the latest thing that happened, always at the top of Get. Tap it and the game opens.", who: "Fans", why: "The event is one tap away, so nobody leaves the app to check it, and it never competes with ordering for the front door.", level: 1 },
  { sel: ".welcome", title: "The first-night card", what: "Greets a new fan with the one thing Agape is for: something good, at their seat, soon.", who: "First-time fans", why: "The first order is what turns a download into a customer.", level: 1 },
  { sel: ".item-row", title: "When it reaches you", what: "Every item shows the time it would reach this seat, from how fast this kitchen and its runners are going right now.", who: "Fans", why: "An honest time is why people order instead of standing in line. Agape keeps a share of every sale and the delivery fee.", level: 1 },
  { sel: ".order-pill", title: "Your order follows you", what: "Wherever you are in the app, your order's stage and arrival time stay at the top.", who: "Fans with an order on the way", why: "Fans stop worrying and keep watching the game, which brings them back to order again.", level: 1 },
  { sel: ".cart-bar", title: "One cart, one fee", what: "Items from different stands, ordered together for one delivery fee.", who: "Fans", why: "Bigger orders for the same walk: more for the stands, the runner and Agape.", level: 1 },
  { sel: ".me-head", title: "Your profile", what: "Who you are, where you're sitting, and later your level and collection.", who: "Every fan", why: "One identity that travels to every venue on Agape: the fan relationship is Agape's, not one venue's.", level: 1 },
  { sel: ".wallet", title: "The wallet", what: "What you've spent and how you pay. You're charged only when your order reaches you.", who: "Fans", why: "Agape processes the card itself: sellers pay 3% + 30¢, Agape pays the card networks less, and keeps the difference.", level: 1 },
  { sel: ".settings-list", title: "Real settings", what: "Payment, seat, notifications, accessibility, privacy and help, each working the way it would at launch.", who: "Fans", why: "Trust: a fan can fix anything themselves, at the seat, without asking anyone.", level: 1 },
  { sel: ".space-bar button[aria-label=Deals]", title: "Deals", what: "This event's offers, sponsor perks, points and your seat. Each deal shows only during the event it belongs to.", who: "Fans at an event", why: "Sponsors and nearby businesses pay to reach fans at the moment they care most.", level: 2 },
  { sel: ".points-head", title: "Points", what: "Earned on every dollar, spent on deals or to have an order go first. They can't be bought or cashed out.", who: "Fans", why: "A reason to order through Agape every time, at every venue.", level: 2 },
  { sel: ".space-bar button[aria-label=Play]", title: "Play", what: "Calls, the section battle, moments to collect and trade.", who: "Fans who want to be part of the game", why: "Games are sponsored, prizes are funded by venues and sponsors, and the section battle is won by ordering.", level: 3 },
  { sel: ".battle-block", title: "The section battle", what: "Sections compete on orders in the last five minutes. The section on top at the final wins a prize.", who: "Everyone in a section", why: "A reason to order now, together. It's the most direct link between playing and ordering.", level: 3 },
  { sel: ".space-bar button[aria-label=People]", title: "People", what: "Your section, the crowd, the people you follow.", who: "Fans", why: "A reason to open Agape between events, which is when tickets and merch sell.", level: 5 },
];

type Box = { i: number; x: number; y: number; w: number; h: number };

/** Glowing outlines over the parts of `root` that have a note; a tap picks one. */
export function ExplainLayer({ root, notes, on, pick, picked }: { root: () => HTMLElement | null; notes: Note[]; on: boolean; pick: (n: Note | null) => void; picked: Note | null }) {
  const [boxes, setBoxes] = useState<Box[]>([]);
  useEffect(() => {
    if (!on) { setBoxes([]); return; }
    const measure = () => {
      const r = root(); if (!r) return;
      const base = r.getBoundingClientRect();
      const out: Box[] = [];
      notes.forEach((n, i) => {
        const el = r.querySelector(n.sel) as HTMLElement | null;
        if (!el) return;
        const b = el.getBoundingClientRect();
        const y = Math.max(b.top, base.top), bottom = Math.min(b.bottom, base.bottom);
        if (bottom - y < 14 || b.width < 10) return;
        out.push({ i, x: b.left - base.left, y: y - base.top, w: b.width, h: bottom - y });
      });
      setBoxes(out);
    };
    measure();
    const t = setInterval(measure, 300);
    return () => clearInterval(t);
  }, [on, notes]);
  if (!on) return null;
  return (
    <div className="explain-layer">
      {boxes.map(b => <button key={b.i} className={"explain-box" + (picked === notes[b.i] ? " on" : "")} style={{ left: b.x - 3, top: b.y - 3, width: b.w + 6, height: b.h + 6 }}
        aria-label={`Explain: ${notes[b.i].title}`} onClick={() => pick(notes[b.i])} />)}
    </div>
  );
}

export function ExplainCard({ n, close }: { n: Note; close: () => void }) {
  return (
    <div className="explain-card">
      <div className="row gap"><span className="eyebrow grow">Level {n.level} · explained</span><button className="link-inline small" onClick={close}>Back to the story</button></div>
      <h2>{n.title}</h2>
      <dl>
        <dt>What it is</dt><dd>{n.what}</dd>
        <dt>Who it's for</dt><dd>{n.who}</dd>
        <dt>Why Agape builds it</dt><dd>{n.why}</dd>
      </dl>
    </div>
  );
}

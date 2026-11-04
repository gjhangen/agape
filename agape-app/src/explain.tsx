// Explain mode: a layer the demo puts over a device, never part of the app. With it on, the parts worth
// explaining glow; tapping one says what it is, who it's for, why Agape builds it and how it earns, and its level.
import { React, useEffect, useState } from "./r";

export type Note = { sel: string; title: string; what: string; who: string; why: string; level: number };

export const FAN_NOTES: Note[] = [
  { sel: ".anchor", title: "The event, in one line", what: "The score and the latest thing that happened, always at the top of Get. Tap it to open the event.", who: "Fans", why: "The event is one tap away, so nobody leaves the app to check it, and it never pushes ordering off the main screen.", level: 1 },
  { sel: ".welcome", title: "The first-night card", what: "Greets a new fan with the one thing Agape is for: something good, at their seat, soon.", who: "First-time fans", why: "The first order is what turns a download into a customer.", level: 1 },
  { sel: ".item-row", title: "When it reaches you", what: "Every item shows the time it would reach this seat, from how fast this kitchen and its runners are going right now.", who: "Fans", why: "An honest time is why people order instead of standing in line. Agape keeps a share of every sale and the delivery fee.", level: 1 },
  { sel: ".order-pill", title: "Your order follows you", what: "Wherever you are in the app, your order's stage and arrival time stay at the top.", who: "Fans with an order on the way", why: "Fans stop worrying and keep watching, which brings them back to order again.", level: 1 },
  { sel: ".cart-bar", title: "One cart, one fee", what: "Items from different stands, ordered together for one delivery fee.", who: "Fans", why: "Bigger orders for the same walk: more for the stands, the runner and Agape.", level: 1 },
  { sel: ".me-head", title: "Your profile", what: "Who you are, where you're sitting, and later your level and collection.", who: "Every fan", why: "One identity that travels to every venue on Agape: the fan relationship is Agape's, not one venue's.", level: 1 },
  { sel: ".wallet", title: "The wallet", what: "What you've spent and how you pay. You're charged only when your order reaches you.", who: "Fans", why: "Agape runs the payment itself: sellers pay 3% + 30¢, Agape pays the card networks less, and keeps the difference.", level: 1 },
  { sel: ".settings-list", title: "Settings that work", what: "Payment, seat, notifications, accessibility, privacy and help, each working the way it would at launch.", who: "Fans", why: "Trust: a fan can fix anything themselves, at the seat, without asking anyone.", level: 1 },
  { sel: ".points-head", title: "Points", what: "Earned on every dollar of food, drinks and gear, spent on perks or to have an order go first. They can't be bought or cashed out.", who: "Fans", why: "A reason to order through Agape every time, at every venue.", level: 2 },
  { sel: ".offer", title: "Tonight's deals", what: "Offers that open at moments in the event, each paid for by a named sponsor or stand, with how many are left and when they end.", who: "Fans at an event", why: "Sponsors pay to reach fans at the moment they care most; the fan sees exactly who's paying.", level: 2 },
  { sel: ".upgrade-row", title: "A better seat", what: "Empty seats elsewhere in the building, offered for the rest of the event. Your orders follow you.", who: "Fans", why: "Revenue for the venue from seats that would otherwise stay empty.", level: 2 },
  { sel: ".predict", title: "Call it", what: "A call on each part of the game, locked soon after it starts. Right answers earn points.", who: "Fans who want to play along", why: "Fans keep Agape open through the game, which is when they order.", level: 3 },
  { sel: ".battle-block", title: "The section battle", what: "Sections compete on orders in the last five minutes. The section on top when ordering closes wins free snacks for everyone in it at the next event.", who: "Everyone in a section", why: "A reason to order now, together. It's the most direct link between playing and ordering.", level: 3 },
  { sel: ".card-grid", title: "Moments to collect", what: "The night's big plays become limited cards, free while they last, then traded between fans for points.", who: "Collectors", why: "Something to keep from the night, and a market that runs between events.", level: 3 },
  { sel: ".event-row", title: "Tickets for the next event", what: "The venue's next events with seats on a map, and reselling yours at up to 120% of face value.", who: "Fans planning their next night", why: "Agape is where fans already are when they decide to come back.", level: 4 },
  { sel: ".around-row", title: "Around the venue", what: "A ride home, prepaid parking, and perks at places nearby that fans unlock with points.", who: "Fans leaving the event", why: "The night doesn't end at the gate, and partners pay for the fans who come through their door.", level: 4 },
  { sel: ".chat", title: "Your section", what: "Chat with the people around you, follow fans and players, and see who's here tonight.", who: "Fans", why: "A reason to open Agape between events, which is when tickets and merch sell.", level: 5 },
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
      // the device may be drawn scaled: boxes are placed in its own, unscaled units
      const z = r.offsetWidth ? base.width / r.offsetWidth : 1;
      const out: Box[] = [];
      notes.forEach((n, i) => {
        const el = r.querySelector(n.sel) as HTMLElement | null;
        if (!el) return;
        const b = el.getBoundingClientRect();
        const y = Math.max(b.top, base.top), bottom = Math.min(b.bottom, base.bottom);
        if (bottom - y < 14 || b.width < 10) return;
        out.push({ i, x: (b.left - base.left) / z, y: (y - base.top) / z, w: b.width / z, h: (bottom - y) / z });
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
      <div className="row gap"><span className="eyebrow grow">Level {n.level} · explained</span><button className="btn ghost sm" onClick={close} aria-label="Close">Close</button></div>
      <h2>{n.title}</h2>
      <dl>
        <dt>What it is</dt><dd>{n.what}</dd>
        <dt>Who it's for</dt><dd>{n.who}</dd>
        <dt>Why Agape builds it</dt><dd>{n.why}</dd>
      </dl>
    </div>
  );
}

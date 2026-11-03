import { world } from "./kernel";
// The seven shared components. Every screen in every perspective is built from these:
// offer card, commitment card, signal row, measure tile, place view, party chip, ledger line.

import { React, useEffect, useState } from "./r";
import { ItemArt } from "./Art";
import { R, SECS, ang, pt, wedge, walk, along, path, placePoint, Stand } from "./geo";

// ---------- what the screens call things (display only; the kernel uses ids) ----------

type TeamLook = { name: string; short: string; abbr: string; color?: string; logo?: string };
const side = (k: "home" | "away", d: TeamLook): TeamLook => {
  const b = world()?.brand?.[k] ?? {};
  return { name: b.name?.trim() || d.name, short: b.short?.trim() || d.short, abbr: (b.abbr?.trim() || d.abbr).toUpperCase(), color: b.color || d.color, logo: b.logo || undefined };
};
export const BRAND = {
  get venue(): string { return world()?.name || "Harbor Arena"; },
  get home(): TeamLook { return side("home", { name: "Harbor Kings", short: "Kings", abbr: "HBK", color: "#B8174F" }); },
  get away(): TeamLook { return side("away", { name: "Mesa Comets", short: "Comets", abbr: "MSC", color: "#2F6DB5" }); },
  get star(): string { const p = world()?.brand?.home?.players?.[0]?.name?.trim(); if (!p) return "J. Ayers"; const w = p.split(/\s+/); return w.length > 1 ? `${w[0][0]}. ${w[w.length - 1]}` : p; },
  sponsors: { sponsor_cola: "Crown Cola", sponsor_water: "Clearwater" } as Record<string, string>,
  gatesAt: 18 * 60 + 45, // 6:45 PM; the kernel's clock starts when the gates open
};
/** What the venue has switched on for its fans; anything it hasn't mentioned is on. */
export const feature = (k: string): boolean => world()?.features?.[k] !== false;
/** A team's logo when the venue has given one, otherwise its color dot. */
export function TeamMark({ side, big }: { side: "home" | "away"; big?: boolean }) {
  const t = side === "home" ? BRAND.home : BRAND.away;
  return t.logo ? <img className={"team-logo" + (big ? " big" : "")} src={t.logo} alt="" /> : <i />;
}
export const team = (t: string) => (t === "home" ? BRAND.home.short : t === "away" ? BRAND.away.short : "");

export const OFFER_COPY: Record<string, { title: string; blurb: string }> = {
  halftime_combo: { title: "Halftime combo", blurb: "Cheeseburger and a fountain soda" },
  get run_merch() { return { title: `${BRAND.home.short} run · foam finger`, blurb: "Wave it while it's hot" }; },
  water_break: { title: "Water break", blurb: "On the house" },
};

// ---------- formatting ----------

export const money = (c: number | null | undefined) => (c == null ? "–" : (c < 0 ? "−$" : "$") + (Math.abs(c) / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 }));
export function venueTime(t: number) {
  const m = BRAND.gatesAt + Math.floor(t / 60000);
  const h = Math.floor(m / 60) % 24, mm = m % 60;
  return `${((h + 11) % 12) + 1}:${String(mm).padStart(2, "0")} ${h >= 12 ? "PM" : "AM"}`;
}
export function rel(t: number, now: number) {
  const d = Math.round((t - now) / 60000);
  if (Math.abs(d) < 1) return "now";
  return d > 0 ? `in ${d} min` : `${-d} min ago`;
}
export function countdown(ms: number) {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

export const STAGES: Record<string, { label: string; step: number; tone?: "good" | "bad" | "info" }> = {
  Accepted: { label: "Order placed", step: 0 },
  Preparing: { label: "Being made", step: 1 },
  Ready: { label: "Ready at the stand", step: 2 },
  Assigned: { label: "Runner heading to the stand", step: 2 },
  PickedUp: { label: "On the way to you", step: 3 },
  Delivered: { label: "Delivered", step: 4, tone: "good" },
  Disputed: { label: "We're looking into it", step: 4, tone: "info" },
  Settled: { label: "Done", step: 4, tone: "good" },
  FanCancelled: { label: "Cancelled. You weren't charged.", step: -1, tone: "info" },
  PaymentFailed: { label: "Payment didn't go through. You weren't charged.", step: -1, tone: "bad" },
  StandCannotFulfil: { label: "The stand couldn't make it. You weren't charged.", step: -1, tone: "bad" },
  FanUnreachable: { label: "We couldn't find you at your seat. Tip returned.", step: -1, tone: "bad" },
  DisputeUpheld: { label: "Refunded in full.", step: -1, tone: "info" },
  VenueHalted: { label: "The venue stopped service before it was made. You weren't charged.", step: -1, tone: "info" },
};

export const WHY_NOT: Record<string, string> = {
  AgeUnverified: "21+ · tap to unlock",
  WrongLevel: "Upper level only",
  SoldOut: "Sold out",
  ClosedForNight: "Kitchens are closed for the night",
  PlaceClosed: "Stand closed",
  StandBusy: "Kitchen is backed up",
};
export const whyNot = (w: string | null) => (!w ? "" : w.startsWith("SpanClosed") ? "Last call has passed" : WHY_NOT[w] ?? w);

export function momentText(kind: string, t: string) {
  const T = team(t);
  switch (kind) {
    case "run": return `${T} on a run`;
    case "lead_change": return `${T} take the lead`;
    case "splash": return t === "home" ? `Splash! ${BRAND.star}` : `${T} hit from deep`;
    case "last_call": return "Last call for beer";
    default: return kind.replace(/_/g, " ");
  }
}

/** Re-render every second so countdowns move between kernel ticks. */
export function useSecond() {
  const [, set] = useState(0);
  useEffect(() => { const i = setInterval(() => set(n => n + 1), 1000); return () => clearInterval(i); }, []);
}

// ---------- 1. offer card ----------

export type Offer = {
  id: string; source: string; thing: string; name: string; stand: string; price: number; discount: number; payer: string | null;
  guaranteed: boolean; held_for_you: boolean; ends_at: number | null; left: number | null; condition: string | null;
  ended: string | null; claim: { id: string; until: number } | null; opened_at: number; left_at_open: number | null;
};

function endedText(o: Offer) {
  switch (o.ended) {
    case "Time": return "This one ended";
    case "Condition": return o.condition ? `${o.condition}. That's over now` : "Ended";
    case "SoldOut": return "Sold out";
    case "BudgetSpent": case "CapReached": return o.guaranteed ? "You claimed this one" : "All claimed";
    case "Paused": return "Paused while the kitchen catches up";
    default: return "";
  }
}

/** How much of an offer is left, as a fraction, by time and by quantity. Null when that limit doesn't apply. */
function drains(o: Offer, now: number) {
  const time = o.ends_at != null && o.ends_at > o.opened_at ? Math.max(0, Math.min(1, (o.ends_at - now) / (o.ends_at - o.opened_at))) : null;
  const qty = !o.guaranteed && o.left != null && o.left_at_open != null && o.left_at_open > 0 && o.left_at_open <= 500 ? Math.max(0, Math.min(1, o.left / o.left_at_open)) : null;
  return { time, qty };
}

function Drain({ f, label, tone }: { f: number; label: string; tone: string }) {
  return (
    <div className={"drain " + tone} role="meter" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(f * 100)} aria-label={label}>
      <span className="drain-label num">{label}</span>
      <span className="drain-bar"><i style={{ width: `${f * 100}%` }} /></span>
    </div>
  );
}

export function OfferCard({ o, now, onTap, onCheckout }: { o: Offer; now: number; onTap?: () => void; onCheckout?: () => void }) {
  const copy = OFFER_COPY[o.source] ?? { title: o.name, blurb: "" };
  const live = !o.ended;
  const { time, qty } = drains(o, now);
  const tone = (f: number) => (f < 0.15 ? "low" : f < 0.35 ? "mid" : "ok");
  return (
    <article className={"offer" + (live ? "" : " ended") + (o.claim ? " claimed" : "")} aria-label={copy.title}>
      <ItemArt name={o.name} size={56} />
      <div className="offer-body">
        <div className="offer-top">
          <h3>{copy.title}</h3>
          <div className="price num">
            {o.discount > 0 && <s>{money(o.price)}</s>}
            <b>{o.discount >= o.price ? "Free" : money(o.price - o.discount)}</b>
          </div>
        </div>
        <p className="sub">{copy.blurb} · {o.stand}{o.payer ? <> · <span className="payer">on {BRAND.sponsors[o.payer] ?? o.payer}</span></> : null}</p>
        {o.guaranteed && live && <p className="held">{o.held_for_you ? `Held for you until ${venueTime(o.ends_at ?? now)}` : "Guaranteed"}</p>}
        {!o.guaranteed && live && (
          <div className="drains">
            {o.condition && <span className="cond">{o.condition}</span>}
            {time != null && <Drain f={time} tone={tone(time)} label={o.ends_at! - now <= 300000 ? `Ends in ${countdown(o.ends_at! - now)}` : `Until ${venueTime(o.ends_at!)}`} />}
            {qty != null && <Drain f={qty} tone={tone(qty)} label={`${o.left} of ${o.left_at_open} left${o.discount ? " at this price" : ""}`} />}
          </div>
        )}
        {o.claim ? (
          <button className="btn primary block" onClick={onCheckout}>Yours for {countdown(o.claim.until - now)} · Check out</button>
        ) : live ? (
          <button className="btn primary block" onClick={onTap}>Claim</button>
        ) : (
          <p className="ended-note">{endedText(o)}</p>
        )}
      </div>
    </article>
  );
}

// ---------- 2. commitment card ----------

export type Order = {
  id: number; stage: string; items: { thing: string; name: string; qty: number; price: number }[]; stand: string; stand_id: string;
  delivery: boolean; runner: string | null; eta: number; placed: number; delivered: number | null; source: string; seat: string;
  charge: number | null; breakdown: { subtotal: number; discount: number; tax: number; fee: number; tip: number } | null;
  late_credit: number | null; can_cancel: boolean; scheduled_for?: number | null;
  late?: boolean; can_report?: boolean; gift?: { to: string; note: string } | null; priority?: boolean; rated?: { who: string; stars: number }[]; problem?: { id: string; words: string; state: { state: "open" | "refunded" | "declined"; amount?: number; why?: string } } | null;
};

const TRACK = ["Placed", "Making", "Ready", "On the way", "Here"];
const TRACK_PICKUP = ["Placed", "Making", "Ready", "Collected", "Done"];

export type Viewer = "fan" | "runner" | "stand";

/** What each viewer is told about a stage. Every viewer is "you" on their own screen, never a third party. */
const VOICE: Record<Exclude<Viewer, "fan">, Record<string, string>> = {
  runner: { Assigned: "Pick up", PickedUp: "Hand off", Delivered: "Handed off", Settled: "Done" },
  stand: { Accepted: "Up next", Preparing: "Cooking", Ready: "On the shelf", Assigned: "Runner on the way", PickedUp: "Picked up", Delivered: "Collected", Settled: "Done" },
};

export function CommitmentCard({ o, now, onCancel, compact, as = "fan" }: { o: Order; now: number; onCancel?: () => void; compact?: boolean; as?: Viewer }) {
  const st = STAGES[o.stage] ?? { label: o.stage, step: 0 };
  const items = o.items.map(i => (i.qty > 1 ? `${i.qty}× ` : "") + i.name).join(", ");
  const exit = st.step < 0;
  const done = o.stage === "Settled" || o.stage === "Delivered";
  const to = o.delivery ? `Section ${o.seat.slice(1)}` : "Pickup at the counter";
  if (as === "runner") {
    // the runner's card leads with where they're going next
    const where = o.stage === "Assigned" ? o.stand : to;
    return (
      <article className={"commit" + (compact ? " compact" : "")}>
        <div className="commit-head">
          <ItemArt name={o.items[0]?.name ?? ""} size={36} />
          <div className="grow"><h3>{VOICE.runner[o.stage] ?? o.stage} · {where}</h3><p className="sub">{items}{o.stage === "Assigned" ? ` for ${to.toLowerCase()}` : ` from ${o.stand}`}</p></div>
          <div className="eta num"><b>{venueTime(o.eta)}</b><small>due {rel(o.eta, now)}</small></div>
        </div>
        {o.breakdown && o.breakdown.tip > 0 && <p className="credit">{money(o.breakdown.tip)} tip, all yours</p>}
      </article>
    );
  }
  if (as === "stand") {
    return (
      <article className={"commit" + (compact ? " compact" : "")}>
        <div className="commit-head">
          <ItemArt name={o.items[0]?.name ?? ""} size={36} />
          <div className="grow"><h3>{items}</h3><p className="sub">{o.delivery ? `Runner to section ${o.seat.slice(1)}` : "Fan collects here"}{o.source !== "menu" ? " · offer" : ""}</p></div>
          <span className="tag">{VOICE.stand[o.stage] ?? o.stage}</span>
        </div>
        {o.runner && o.stage === "Assigned" && <div className="commit-foot"><PartyChip kind="runner" name={runnerName(o.runner)} /><span className="sub">arriving for it</span></div>}
      </article>
    );
  }
  return (
    <article className={"commit" + (exit ? " exit" : "") + (compact ? " compact" : "")}>
      <div className="commit-head">
        <ItemArt name={o.items[0]?.name ?? ""} size={compact ? 36 : 48} />
        <div className="grow">
          <h3>{items}</h3>
          <p className="sub">{o.stand} · {o.gift ? `a gift to ${o.gift.to}` : o.delivery ? `to section ${o.seat.slice(1)}` : "pickup"}{o.source !== "menu" ? " · from an offer" : ""}{o.priority ? " · first in line" : ""}</p>
        </div>
        <div className="eta num">
          {exit ? null : done ? <span className="tag good">{o.delivered ? venueTime(o.delivered) : "Done"}</span> :
            <><b>{venueTime(o.eta)}</b><small title="An estimate, updated at every step">est. {rel(o.eta, now)}</small></>}
        </div>
      </div>
      {!exit && !compact && (
        <ol className="track" aria-label="Progress">
          {(o.delivery ? TRACK : TRACK_PICKUP).map((s, i) => <li key={s} className={i < st.step ? "done" : i === st.step ? "here" : ""}>{s}</li>)}
        </ol>
      )}
      <div className="commit-foot">
        <span className={"status " + (st.tone ?? "")}>{o.scheduled_for && o.stage === "Accepted" ? `Held for ${venueTime(o.scheduled_for)}` : st.label}</span>
        {o.runner && !exit && !done && <PartyChip kind="runner" name={runnerName(o.runner)} />}
        <span className="grow" />
        {o.charge != null && !exit && <b className="num">{money(o.charge)}</b>}
        {o.can_cancel && onCancel && <button className="btn ghost sm" onClick={onCancel}>Cancel</button>}
      </div>
      {o.late_credit ? <p className="credit">{done ? `Late. ${money(o.late_credit)} back to your card, automatically.` : `Running late. You'll get ${money(o.late_credit)} back automatically, nothing to ask for.`}</p> : null}
      {o.problem ? <p className={"credit" + (o.problem.state.state === "declined" ? " no" : "")}>{o.problem.words}: {o.problem.state.state === "refunded" ? `${money(o.problem.state.amount)} refunded to your card.` : o.problem.state.state === "open" ? "the venue's team is looking at it." : `not refunded. ${o.problem.state.why}`}</p> : null}
    </article>
  );
}

const NAMES = ["Maya", "Dre", "Lena", "Omar", "Priya", "Theo", "Ines", "Kofi", "Sam", "Rosa", "Jules", "Nate", "Ama", "Leo", "Zoe", "Ravi", "Tess", "Marco", "Uma", "Eli"];
export const runnerName = (id: string) => NAMES[(+id.replace(/\D/g, "") || 0) % NAMES.length] + " " + String.fromCharCode(65 + ((+id.replace(/\D/g, "") || 0) % 26)) + ".";

// ---------- 3. signal row ----------

export type FeedItem = { t: number; kind: string; team: string; text: string };

export function SignalRow({ f }: { f: FeedItem }) {
  let text = f.text, cls = f.kind;
  if (f.kind === "moment") text = momentText(f.text, f.team);
  if (f.kind === "score") { const [pts, star] = f.text.replace("+", "").split(" "); text = `${team(f.team)} +${pts}${star && f.team === "home" ? ` · ${BRAND.star}` : ""}`; cls += " " + f.team; }
  if (f.kind === "offer") text = `New offer: ${f.text}`;
  if (f.kind === "alert") text = "Kitchens are surging. Waits are climbing";
  return (
    <li className={"signal " + cls}>
      <time className="num">{venueTime(f.t)}</time>
      <span className="dot" aria-hidden="true" />
      <span className="grow">{text}</span>
    </li>
  );
}

// ---------- 4. measure tile ----------

export function MeasureTile({ label, value, sub, tone }: { label: string; value: React.ReactNode; sub?: React.ReactNode; tone?: "good" | "warn" | "bad" }) {
  return (
    <div className={"tile " + (tone ?? "")}>
      <span className="eyebrow">{label}</span>
      <b className="num">{value}</b>
      {sub != null && <small>{sub}</small>}
    </div>
  );
}

// ---------- 5. place view ----------

export type MapView = {
  levels: number; sections_per_level: number; heat: Record<string, number>; stands: Stand[];
  runners: { id: string; from: string; to: string; progress: number; busy: boolean }[];
  you: string | null; your_runner: { id: string; from: string; to: string; progress: number; busy: boolean } | null;
};

export function PlaceView({ m, onStand, focusStand, showRunners = true, heat = true, selfRunner = false }: { m: MapView; onStand?: (id: string) => void; focusStand?: string | null; showRunners?: boolean; heat?: boolean; selfRunner?: boolean }) {
  const max = Math.max(4, ...Object.values(m.heat));
  const secs: React.ReactNode[] = [];
  for (let level = 1; level <= m.levels; level++) {
    for (let ring = 0; ring < SECS; ring++) {
      const id = `s${level * 100 + ring + 1}`;
      const a0 = -90 + ring * 15 + 0.8, a1 = -90 + (ring + 1) * 15 - 0.8;
      const [r0, r1] = level === 1 ? [R.l1i, R.l1o] : [R.l2i, R.l2o];
      const h = heat ? (m.heat[id] ?? 0) / max : 0;
      const you = m.you === id;
      secs.push(<path key={id} d={wedge(r0, r1, a0, a1)} className={"sec" + (you ? " you" : "")} style={{ ["--h" as any]: h.toFixed(2) }}>
        <title>{`Section ${level * 100 + ring + 1} · ${m.heat[id] ?? 0} orders in 5 min`}</title></path>);
    }
  }
  const yr = m.your_runner;
  const yrPath = yr && yr.busy ? walk(yr.from, yr.to, m.stands) : [];
  const youPt = m.you ? placePoint(m.you, m.stands) : null;
  return (
    <svg className="place" viewBox="-222 -222 444 444" role="img" aria-label="Arena map">
      <circle r={R.c2 + 10} className="ground" />
      <circle r={R.c2} className="conc" /><circle r={R.l2i - 6} className="ground" /><circle r={R.c1} className="conc" /><circle r={R.l1i - 6} className="ground" />
      <rect x={-50} y={-30} width={100} height={60} rx={6} className="court" />
      <line x1={0} y1={-30} x2={0} y2={30} className="courtline" /><circle r={9} className="courtline" fill="none" />
      {secs}
      {[0, 6, 12, 18].map(r => { const [x, y] = pt((R.c1 + R.l2i) / 2 + 2, ang(r)); return <rect key={r} x={x - 3} y={y - 3} width={6} height={6} className="stairs"><title>Stairs</title></rect>; })}
      {(() => { const [x, y] = pt((R.c1 + R.l2i) / 2 + 2, ang(3)); return <rect x={x - 4} y={y - 4} width={8} height={8} rx={2} className="lift"><title>Elevator</title></rect>; })()}
      {yrPath.length > 1 && <path d={path(yrPath)} className="route" />}
      {m.runners.filter(r => (showRunners || r.id === yr?.id) && !(selfRunner && r.id === yr?.id)).map(r => {
        const p = r.busy ? along(walk(r.from, r.to, m.stands), r.progress) : placePoint(r.from, m.stands);
        return <circle key={r.id} cx={p[0]} cy={p[1]} r={yr?.id === r.id ? 6 : 3.2} className={"runner" + (r.busy ? " busy" : "") + (yr?.id === r.id ? " yours" : "")} />;
      })}
      {m.stands.map(s => {
        const [x, y] = placePoint(s.id, m.stands);
        const tone = s.busy ? "bad" : s.wait > 600 ? "warn" : "good";
        return (
          <g key={s.id} transform={`translate(${x},${y})`} className={"stand " + tone + (focusStand === s.id ? " focus" : "")} onClick={() => onStand?.(s.id)} role={onStand ? "button" : undefined} tabIndex={onStand ? 0 : undefined}>
            <circle r={9} /><text y={3.5} textAnchor="middle">{s.name.match(/\d+/)?.[0]}</text>
            <title>{`${s.name} · ${Math.round(s.wait / 60)} min kitchen wait`}</title>
          </g>
        );
      })}
      {youPt && <g transform={`translate(${youPt[0]},${youPt[1]})`} className="youpin"><circle r={11} className="pulse" /><circle r={5} /></g>}
      {selfRunner && yr && (() => { const p = yr.busy ? along(walk(yr.from, yr.to, m.stands), yr.progress) : placePoint(yr.from, m.stands);
        return <g transform={`translate(${p[0]},${p[1]})`} className="youpin"><circle r={11} className="pulse" /><circle r={5} /></g>; })()}
    </svg>
  );
}

// ---------- 6. party chip ----------

export function PartyChip({ kind, name }: { kind: "runner" | "stand" | "sponsor" | "fan" | "agape"; name: string }) {
  return <span className={"chip " + kind}><i aria-hidden="true">{name.slice(0, 1)}</i>{name}</span>;
}

// ---------- 7. ledger line ----------

export function LedgerLine({ label, amount, strong, note }: { label: React.ReactNode; amount: number | null; strong?: boolean; note?: React.ReactNode }) {
  return (
    <div className={"ledger" + (strong ? " strong" : "")}>
      <span className="grow">{label}{note && <small> {note}</small>}</span>
      <span className="num">{money(amount)}</span>
    </div>
  );
}

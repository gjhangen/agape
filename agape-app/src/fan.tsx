// The fan relates to the world in three ways, each with its own natural order:
//   Get   things and offers, by how soon they reach you (zooming out from your seat)
//   Live  the event, shared with everyone, by the event's own time (your section is a filter)
//   Me    what's yours, by its lifecycle (on the way, then done)
// The score, your live orders and search stay on every screen.

import { React, useEffect, useRef, useState } from "./r";
import { actWith, Deal, DealCard } from "./deals";
import { Card, LevelBadge, LevelPanel, MomentCard, Page } from "./pages";
import { kernel } from "./kernel";
import { wearColor } from "./themes";
import { cart, useCart } from "./cart";
import { ItemArt } from "./Art";
import { feature, TeamMark,
  BRAND, CommitmentCard, FeedItem, LedgerLine, MapView, Offer, OfferCard, Order, PlaceView, SignalRow,
  countdown, momentText, money, rel, runnerName, team, useSecond, venueTime, whyNot,
} from "./parts";

export type Item = { id: string; name: string; price: number; category: string; adult?: boolean; stand: string; stand_id: string; level: number; available: boolean; why_not: string | null; left: number | null; eta: number };
type ChatLine = { t: number; you: boolean; who: string; id: string; text: string; section: string; mine: boolean };
type LiveItem = FeedItem & { party: string; group: string; name: string | null; drop: Card | null };
type Ride = { id: string; stage: string; at: number; fare: number; zone: string; zone_name: string; provider: string; provider_name: string; driver: { id: string; name: string; vehicle: string; plate: string; rating: number } | null };
type Listing = { listing: string; price: number; seller: string; seller_name: string; card: Card };
export type FanView = {
  clock: { t: number; label: string; phase: string; home: number; away: number; spans: string[] };
  you: { id: string; seat: string; neighbours: number };
  colors?: { teams: { id: string; name: string; short: string; color: string; home: boolean }[]; wearing: string | null };
  halt?: { reason: string; since: number } | null;
  upgrades?: { id: string; section: string; title: string; blurb: string; price: number; left: number | null; ok: boolean; why_not: string | null }[];
  moved?: { section: string; at: number } | null;
  provenance?: { crowd: string; simulated_fans: number; people: number; eta: string };
  orders: Order[];
  runner: { id: string; to: string; busy: boolean } | null;
  get: { offers: Offer[]; fastest: Item[]; stands: { id: string; name: string; level: number; eta: number | null; slow: boolean }[]; later: { t: number; kind: string }[]; market: Listing[];
    events: { id: string; kind: string; title: string; date: string; time: string; away_name: string | null; yours: number; left: number; from: number | null }[];
    around: { id: string; kind: string; name: string; walk_min: number; blurb: string; final_at: number | null; operator: string | null; operator_name: string | null;
      service: { kind: string; price: number; left: number; yours: boolean; drivers: number; busy: number } | null;
      perk: { title: string; cost: number; supply: number; left: number; yours: boolean; affordable: boolean } | null }[] };
  items: Item[];
  live: {
    event: LiveItem[]; league: { id: string; home: string; away: string; home_name: string; away_name: string; home_score: number; away_score: number; status: string }[]; chat: ChatLine[]; chat_section: ChatLine[];
    prediction: { question: string; options: { id: string; share: number }[]; locks: number; open: boolean; result: string | null; picks: number; yours: string | null; won: boolean; points: number } | null;
    battle: { orders_5m: number; rank: number | null; of: number; ahead: { section: string; orders: number } | null };
  };
  me: {
    orders: Order[]; receipts: Order[]; points: number; spent: number; holdings: { thing: string; name: string; qty: number }[];
    level: number; tier: string; tier_floor: number; next_tier: { name: string; at: number } | null; balance: number;
    earned: { t: number; points: number; why: string }[]; badges: { id: string; name: string; how: string; earned: boolean }[];
    cards: { card: Card; count: number; listing: { id: string; price: number } | null }[];
    following: { id: string; name: string | null; kind: string }[]; messages: { with: string; name: string | null; lines: { t: number; mine: boolean; text: string }[] }[];
    events: { name: string; when: string; venue: string }[];
    tickets: { event: string; title: string; date: string; time: string; section: string; seats: string[]; qty: number }[];
    rides: Ride[];
    passes: { place: string; name: string; title: string; code: string; used: number | null; operator: string }[];
    deals: Deal[];
    passport?: { venues: string[] | null; nights: number | null; here: string; level: number };
  };
  map: MapView;
};

export type Sheet = { kind: "cart" } | { kind: "item"; item: string; at?: number } | { kind: "checkout"; offer: Offer; claim: string } | { kind: "ride"; zone: string } | null;
type Space = "get" | "live" | "me";
type Ctx = { v: FanView; refresh: () => void; setSheet: (s: Sheet) => void; toast: (s: string) => void; go: (s: Space) => void; open: (id: string) => void; settings?: React.ReactNode };
const sec = (s: string) => s.slice(1);

export function FanApp({ v, refresh, sheet, setSheet, toast, settings }: Omit<Ctx, "go" | "open"> & { sheet: Sheet }) {
  const [space, setSpace] = useState<Space>("get");
  const [q, setQ] = useState("");
  const body = useRef<HTMLDivElement>(null);
  useSecond();
  const [pages, setPages] = useState<string[]>([]);
  const [tucked, setTucked] = useState(false);
  // the phone's back button closes what's open (a sheet, a page) instead of leaving the app
  const overlay = (sheet ? 1 : 0) + pages.length;
  const depth = useRef(0), fromBack = useRef(false), skipPop = useRef(0);
  const live = useRef({ sheet, pages }); live.current = { sheet, pages };
  useEffect(() => {
    if (overlay > depth.current) history.pushState({ agape: overlay }, "");
    else if (overlay < depth.current && !fromBack.current) { skipPop.current += depth.current - overlay; history.go(overlay - depth.current); }
    fromBack.current = false; depth.current = overlay;
  }, [overlay]);
  useEffect(() => {
    const on = () => {
      if (skipPop.current > 0) { skipPop.current--; return; }
      fromBack.current = true;
      if (live.current.pages.length) setPages(p => p.slice(0, -1)); else if (live.current.sheet) setSheet(null); else fromBack.current = false;
    };
    addEventListener("popstate", on); return () => removeEventListener("popstate", on);
  }, []);
  const lastY = useRef(0);
  const go = (s: Space) => { setSpace(s); setQ(""); setPages([]); body.current?.scrollTo({ top: 0 }); };
  const open = (id: string) => setPages(p => [...p, id]);
  const ctx: Ctx = { v, refresh, setSheet, toast, go, open, settings };
  const worn = v.colors?.teams.find(t => t.id === v.colors?.wearing)?.color ?? null;
  useEffect(() => { wearColor(worn); }, [worn]);
  useEffect(() => () => wearColor(null), []);
  // the moment something reaches the seat is worth saying out loud
  const active = useRef<Set<number> | null>(null);
  useEffect(() => {
    const now = new Set(v.orders.map(o => o.id));
    if (active.current) for (const id of active.current) if (!now.has(id)) {
      const r = v.me.receipts.find(o => o.id === id);
      if (r?.stage === "Settled" && !r.gift) toast(r.delivery ? `Your ${r.items[0]?.name?.toLowerCase() ?? "order"} is here. Enjoy!` : `Your ${r.items[0]?.name?.toLowerCase() ?? "order"} is ready at ${r.stand}.`);
    }
    active.current = now;
  }, [v.orders.map(o => o.id).join(",")]);
  const readyFor = useRef<Set<number>>(new Set());
  useEffect(() => {
    for (const o of v.orders) if (!o.delivery && o.stage === "Ready" && !readyFor.current.has(o.id)) {
      readyFor.current.add(o.id); toast(`Your ${o.items[0]?.name?.toLowerCase() ?? "order"} is ready. Pick it up at ${o.stand}.`);
    }
  }, [v.orders.map(o => o.id + o.stage).join(",")]);
  const c = v.clock;
  const lead = v.orders[0];
  return (
    <div className={"phone" + (tucked && !q ? " tucked" : "")}>
      {space !== "live" && <button className="event-strip" onClick={() => go("live")} aria-label="Open Live">
        <span className="es-team home"><TeamMark side="home" />{BRAND.home.abbr} <b key={c.home} className="num score-bump">{c.home}</b></span>
        <span className="es-clock num">{c.label}</span>
        <span className="es-team away"><b key={c.away} className="num score-bump">{c.away}</b> {BRAND.away.abbr}<TeamMark side="away" /></span>
      </button>}
      {lead && space !== "me" && (
        <button className="order-pill" onClick={() => go("me")}>
          <ItemArt name={lead.items[0]?.name ?? ""} size={26} />
          <span className="grow"><b>{lead.items[0]?.name}</b>{v.orders.length > 1 ? ` +${v.orders.length - 1} more` : ""} · {stageWord(lead.stage)}</span>
          <span className="num">{lead.delivery ? "here by" : "ready by"} {venueTime(lead.eta)}</span>
        </button>
      )}
      <div className="finder">
        <input id="find" type="search" placeholder="Search food, drinks, gear…" value={q} onChange={e => setQ(e.target.value)} aria-label="Find anything" />
      </div>
      <div className="scope-body" ref={body} onScroll={e => { const y = e.currentTarget.scrollTop; const d = y - lastY.current; lastY.current = y; if (y < 40) setTucked(false); else if (d > 6) setTucked(true); else if (d < -6) setTucked(false); }}>
        {v.halt && <div className="halt-banner" role="alert"><b>Service is paused</b><p>{v.halt.reason}</p><small>Nothing new can be ordered right now. Anything not yet being made was cancelled, and you weren't charged for it.</small></div>}
        {q ? <Search q={q} {...ctx} /> : space === "get" ? <Get {...ctx} /> : space === "live" ? <Live {...ctx} /> : <Me {...ctx} />}
      </div>
      <nav className="space-bar" aria-label="Spaces">
        {([["get", "Get"], ["live", "Live"], ["me", "Me"]] as [Space, string][]).map(([id, label]) => (
          <button key={id} className={space === id && !q ? "on" : ""} aria-current={space === id} onClick={() => go(id)}>
            {label}{id === "me" && v.orders.length > 0 && <i className="badge">{v.orders.length}</i>}
            {id === "live" && v.live.prediction?.open && !v.live.prediction.yours && <i className="dot-badge" aria-label="A pick is open" />}
          </button>
        ))}
      </nav>
      {pages.length > 0 && <Page key={pages.join("/")} id={pages[pages.length - 1]} depth={pages.length} close={() => setPages([])} back={() => setPages(p => p.slice(0, -1))} open={open} toast={toast} now={c.t}
        acts={{ order: item => { setPages([]); setSheet({ kind: "item", item }); }, ride: zone => { setPages([]); setSheet({ kind: "ride", zone }); }, refreshAll: refresh }} />}
      {!sheet && pages.length === 0 && <CartBar v={v} open={() => setSheet({ kind: "cart" })} />}
      {sheet?.kind === "cart" && <CartSheet {...ctx} />}
      {sheet?.kind === "item" && <ItemSheet {...ctx} id={sheet.item} at={sheet.at} />}
      {sheet?.kind === "ride" && <RideSheet {...ctx} zone={sheet.zone} />}
      {sheet?.kind === "checkout" && <CheckoutSheet {...ctx} offer={sheet.offer} claim={sheet.claim} />}
    </div>
  );
}

const stageWord = (s: string) => ({ Accepted: "placed", Preparing: "being made", Ready: "ready", Assigned: "runner on it", PickedUp: "on the way", Delivered: "here" } as Record<string, string>)[s] ?? s;

function tapOffer(o: Offer, { refresh, setSheet, toast }: Ctx) {
  const r = kernel.act({ do: "tap", offer: o.id });
  if (!r.ok) { toast("That one just ran out."); refresh(); return; }
  refresh();
  setSheet({ kind: "checkout", offer: o, claim: r.id });
}

function ItemRow({ it, now, onPick, showStand = true }: { it: Item; now: number; onPick: () => void; showStand?: boolean }) {
  return (
    <button className="item-row" disabled={!it.available && it.why_not !== "AgeUnverified"} onClick={onPick}>
      <ItemArt name={it.name} size={44} />
      <span className="grow"><b>{it.name}</b>
        <small>{it.available ? <>At your seat {venueTime(it.eta)} · {rel(it.eta, now)}{showStand ? ` · ${it.stand}` : ""}{it.left != null ? ` · ${it.left} left` : ""}</> : <>{whyNot(it.why_not)}{showStand ? ` · ${it.stand}` : ""}</>}</small></span>
      <span className="num price-tag">{money(it.price)}</span>
    </button>
  );
}

// ---------- Get: by how soon it reaches you ----------

function Nudge({ v, setSheet }: Ctx) {
  const now = v.clock.t;
  const next = v.get.later[0];
  const fastest = v.get.fastest[0];
  if (!next || !fastest) return null;
  const mins = (next.t - now) / 60000;
  const drink = v.items.filter(i => i.category === "drink" && i.adult && i.available).sort((a, b) => a.eta - b.eta)[0];
  if (next.kind === "halftime" && mins <= 25 && mins > 8)
    return <Callout eyebrow={`Halftime ${rel(next.t, now)}`} text={`Lines get long at the break. Order ahead and it's at your seat when halftime starts.`} cta="Have it waiting at halftime" onClick={() => setSheet({ kind: "item", item: fastest.id, at: next.t })} />;
  if (next.kind === "last_call" && mins <= 10 && drink)
    return <Callout eyebrow={`Last call ${rel(next.t, now)}`} text={`Beer stops when the 4th quarter starts. ${drink.name} reaches you by ${venueTime(drink.eta)}.`} cta={`Get a ${drink.name.toLowerCase()}`} onClick={() => setSheet({ kind: "item", item: drink.id })} />;
  return null;
}

function Callout({ eyebrow, text, cta, onClick }: { eyebrow: string; text: string; cta: string; onClick: () => void }) {
  return <div className="callout"><span className="eyebrow">{eyebrow}</span><p>{text}</p><button className="btn primary" onClick={onClick}>{cta}</button></div>;
}

const ALL_CATS = [["all", "Everything"], ["food", "Food"], ["drink", "Drinks"], ["merch", "Gear"], ["cards", "Cards"], ["tickets", "Tickets"], ["around", "Around"]];
const CAT_FEATURE: Record<string, string> = { cards: "cards", tickets: "tickets", around: "around" };

/** A first-night hello: what's theirs already and three things worth trying. Gone once dismissed. */
function Welcome({ v, go }: Ctx) {
  const key = `agape.welcomed.${v.you.id}`;
  const [gone, setGone] = useState(() => { try { return localStorage.getItem(key) === "1"; } catch { return false; } });
  const fresh = v.me.earned.length <= 1 && v.me.orders.length === 0 && v.orders.length === 0 && v.clock.phase !== "postgame";
  if (gone || !fresh) return null;
  const close = () => { setGone(true); try { localStorage.setItem(key, "1"); } catch { /* shown again next visit */ } };
  return (
    <section className="welcome" aria-label="Welcome">
      <button className="welcome-x" onClick={close} aria-label="Dismiss">×</button>
      <p className="eyebrow">Welcome to {BRAND.venue}</p>
      <h2>{v.me.balance.toLocaleString()} points are yours.</h2>
      <p>{feature("cards") ? "Spend them on moment cards, or earn more as the night goes." : "Earn more as the night goes."} A few things to try:</p>
      <small className="welcome-pilot">This is the Agape pilot: the game and most of the crowd are simulated, and nothing is charged.</small>
      <div className="welcome-acts">
        <button onClick={() => { document.querySelector(".item-row")?.scrollIntoView({ behavior: "smooth", block: "center" }); }}><b>Order to your seat</b><small>see when it'll arrive before you tap</small></button>
        {feature("predictions") && <button onClick={() => go("live")}><b>Call the quarter</b><small>50 points if you're right</small></button>}
        {feature("cards") && !feature("predictions") && <button onClick={() => go("live")}><b>Collect the big plays</b><small>limited cards, gone fast</small></button>}
        {feature("colors") && <button onClick={() => go("me")}><b>Wear your colors</b><small>the app takes your team's look</small></button>}
      </div>
    </section>
  );
}

function Get(ctx: Ctx) {
  const { v, setSheet } = ctx;
  const [cat, setCat] = useState("all");
  const now = v.clock.t;
  const stands = [...v.get.stands].filter(s => v.items.some(i => i.stand_id === s.id && (cat === "all" || i.category === cat))).sort((a, b) => (a.eta ?? 1e15) - (b.eta ?? 1e15));
  const standBlock = (s: typeof stands[number]) => (
    <section key={s.id} className="block">
      <div className="block-head"><h2>{s.name}</h2><span className="sub">{s.eta ? `From ${venueTime(s.eta)}` : ""}{s.slow ? " · busy" : ""}</span></div>
      {v.items.filter(i => i.stand_id === s.id && (cat === "all" || i.category === cat)).sort((a, b) => Number(b.available) - Number(a.available))
        .map(it => <ItemRow key={it.id} it={it} now={now} showStand={false} onPick={() => setSheet({ kind: "item", item: it.id })} />)}
    </section>
  );
  return (
    <>
      <Welcome {...ctx} />
      {v.orders.length === 0 && <p className="orient">You're in <b>section {sec(v.you.seat)}</b>. Anything in {BRAND.venue} can come to your seat. Every time shown is when it reaches you.</p>}
      <Nudge {...ctx} />
      {v.get.offers.length > 0 && (
        <section className="block">
          <h2>For you right now</h2>
          {v.get.offers.map(o => <OfferCard key={o.id} o={o} now={now} onTap={() => tapOffer(o, ctx)} onCheckout={() => o.claim && setSheet({ kind: "checkout", offer: o, claim: o.claim.id })} />)}
        </section>
      )}
      <section className="block">
        <h2>Fastest to section {sec(v.you.seat)}</h2>
        {v.get.fastest.map(it => <ItemRow key={it.id} it={it} now={now} onPick={() => setSheet({ kind: "item", item: it.id })} />)}
      </section>
      <div className="cats-bar"><div className="cats chips" role="group" aria-label="What you want">
        {ALL_CATS.filter(([id]) => !CAT_FEATURE[id] || feature(CAT_FEATURE[id])).map(([id, label]) => <button key={id} className={cat === id ? "on" : ""} onClick={() => { setCat(id); requestAnimationFrame(() => document.querySelector(".cats-bar")?.scrollIntoView({ block: "start", behavior: "smooth" })); }}>{label}</button>)}
      </div></div>
      {!["cards", "tickets", "around"].includes(cat) && stands.slice(0, cat === "all" ? 3 : 99).map(standBlock)}
      {feature("cards") && (cat === "cards" || cat === "all") && v.get.market.length > 0 && (
        <section className="block">
          <div className="block-head"><h2>Cards from fans</h2><span className="sub">Pay in points · you have {v.me.balance.toLocaleString()}</span></div>
          <div className="card-grid">{(cat === "all" ? v.get.market.slice(0, 4) : v.get.market).map(l => (
            <div key={l.listing} className="market-item"><MomentCard c={l.card} size="sm" onClick={() => ctx.open(l.card.id)} /><b className="num">{l.price.toLocaleString()} pts</b><small>{l.seller_name}</small></div>))}</div>
        </section>
      )}
      {feature("tickets") && (cat === "tickets" || cat === "all") && <Events {...ctx} />}
      {feature("around") && (cat === "around" || cat === "all") && <Around {...ctx} />}
      {cat === "all" && stands.slice(3).map(standBlock)}
    </>
  );
}

function Events({ v, open }: Ctx) {
  return (
    <section className="block">
      <div className="block-head"><h2>Next at <button className="link-inline" onClick={() => open("venue")}>{BRAND.venue}</button></h2></div>
      {v.get.events.map(e => (
        <button key={e.id} className="event-row" onClick={() => open(e.id)}>
          <span className="er-date"><b>{e.date.split(", ")[1]}</b><small>{e.date.split(", ")[0]}</small></span>
          <span className="grow"><b>{e.title}</b><small>{e.time} · {e.left.toLocaleString()} seats left{e.yours ? ` · you have ${e.yours}` : ""}</small></span>
          <span className="num price-tag">{e.from ? `from ${money(e.from)}` : "Sold out"}</span>
        </button>
      ))}
    </section>
  );
}

function Around({ v, refresh, toast, open, setSheet }: Ctx) {
  const act = (a: any, ok: string) => { const r = kernel.act(a); toast(r.ok ? ok : r.error === "CantAfford" ? "Not enough points yet." : r.error); refresh(); };
  const fin = v.get.around.find(a => a.final_at)?.final_at;
  const ride = v.me.rides.find(r => r.stage !== "Done");
  return (
    <section className="block">
      <div className="block-head"><h2>Around {BRAND.venue}</h2><span className="sub">You have {v.me.balance.toLocaleString()} pts</span></div>
      {v.get.around.map(a => (
        <div key={a.id} className={"around-row " + a.kind}>
          <div className="grow">
            <button className="link-inline" onClick={() => open(a.operator ?? a.id)}>{a.kind === "partner" ? a.name : a.operator_name ?? a.name}</button>
            <small>{a.kind === "partner" ? a.blurb : `${a.name}. ${a.blurb}`}</small>
            {(a.kind === "ride" || a.kind === "parking") && fin ? <small className="when">Leave at the final ({venueTime(fin)}) and you're there by {venueTime(fin + a.walk_min * 60000)}.</small>
              : <small className="when">{a.walk_min} min walk</small>}
          </div>
          {a.service?.kind === "ride" && (ride ? <span className="tag good">Booked {venueTime(ride.at)}</span> : <button className="btn sm primary" onClick={() => setSheet({ kind: "ride", zone: a.id })}>Book a ride</button>)}
          {a.service?.kind === "parking" && (a.service.yours ? <span className="tag good">Exit paid</span> :
            <button className="btn sm primary" onClick={() => act({ do: "parking", lot: a.id }, "Paid. Show the code in Me at the gate.")}>Prepay exit · {money(a.service.price)}</button>)}
          {a.perk && (a.perk.yours ? <span className="tag good">Unlocked</span> : (
            <button className="btn sm primary perk" disabled={!a.perk.affordable || a.perk.left < 1} onClick={() => act({ do: "unlock", place: a.id }, "Unlocked. Show the code in Me at the counter.")}>
              <span>{a.perk.title}</span><b className="num">{a.perk.affordable ? `${a.perk.cost} pts · ${a.perk.left} left` : `${(a.perk.cost - v.me.balance).toLocaleString()} more pts to unlock`}</b>
            </button>))}
        </div>
      ))}
    </section>
  );
}

const SHORT: Record<string, string> = { Halftime: "Half", "Start of the 3rd": "Q3", "Start of the 4th": "Q4", "Final buzzer": "Final", "Tip-off": "Tip" };

/** Any time from the soonest possible to the latest allowed, with the night's moments marked. */
function WhenPicker({ from, to, value, set, marks }: { from: number; to: number; value: number; set: (t: number) => void; marks: { t: number; label: string }[] }) {
  const span = Math.max(60000, to - from);
  const pct = (t: number) => ((t - from) / span) * 100;
  const near = marks.find(m => Math.abs(m.t - value) <= 3 * 60000);
  return (
    <div className="when-picker">
      <div className="wp-value"><b className="num">{venueTime(value)}</b><small>{near ? near.label : value - from < 90000 ? "as soon as it can be ready" : rel(value, from).replace("in ", "") + " from the soonest"}</small></div>
      <div className="wp-track">
        <input id="when" type="range" min={from} max={to} step={60000} value={value} onChange={e => set(+e.target.value)} aria-label="When it reaches you" />
        {marks.filter(m => m.t > from && m.t < to).map(m => <button key={m.t} className="wp-mark" style={{ left: `${pct(m.t)}%` }} onClick={() => set(m.t)} title={m.label}><i /><span>{SHORT[m.label] ?? m.label}</span></button>)}
      </div>
      <div className="wp-ends num"><span>{venueTime(from)}</span><span>{venueTime(to)}</span></div>
    </div>
  );
}

function RideSheet({ v, refresh, setSheet, toast, zone, open }: Ctx & { zone: string }) {
  const a = v.get.around.find(x => x.id === zone)!;
  const fin = a.final_at ?? v.clock.t + 3600000;
  const from = v.clock.t + 5 * 60000, to = fin + 60 * 60000;
  const [at, setAt] = useState(Math.max(from, fin + 5 * 60000));
  const rq = kernel.act({ do: "ride_quote", at, provider: a.operator });
  const fare = rq.fare as number, free = rq.free as number;
  const book = () => { const r = kernel.act({ do: "ride", zone, at }); if (!r.ok) { toast(r.error === "NoDriver" ? "Every driver is promised around then. Try another time." : r.error); refresh(); return; } toast(`Booked. A driver will be assigned 8 minutes before ${venueTime(at)}.`); setSheet(null); refresh(); };
  return (
    <SheetFrame title="Book a ride" onClose={() => setSheet(null)}>
      <div className="build">
        <p className="sub">With <button className="link-inline" onClick={() => { setSheet(null); open(a.operator!); }}>{a.operator_name}</button> from {a.name}, {a.walk_min} min from your seat. {a.service!.drivers - a.service!.busy} drivers free right now.</p>
        <span className="eyebrow">Pickup time</span>
        <WhenPicker from={from} to={to} value={at} set={setAt} marks={[{ t: fin, label: "Final buzzer" }]} />
        <div className="lines"><LedgerLine label="Fare" amount={fare} note={fare > 1890 ? "busier right after the final" : ""} /><LedgerLine label="You pay when the ride ends" amount={fare} strong /></div>
        {free > 0 ? <p className="promise">{free} driver{free === 1 ? "" : "s"} still free around {venueTime(at)}. Booking holds one for you; their name, car and plate show in Me when they're assigned. Cancel free until they arrive.</p>
          : <p className="note">Every driver is already promised around {venueTime(at)}. Slide to another time to see who's free.</p>}
        <button className="btn primary block big" disabled={free < 1 || !!v.halt} onClick={book}>{v.halt ? "Service is paused" : `Book for ${venueTime(at)} · ${money(fare)}`}</button>
      </div>
    </SheetFrame>
  );
}

function RideCard({ r, open, cancel }: { r: Ride; open: (id: string) => void; cancel: () => void }) {
  const say: Record<string, string> = { Booked: `Booked for ${venueTime(r.at)}. A driver is assigned 8 minutes before.`, DriverAssigned: "On the way to the pickup", Waiting: "Waiting for you at the pickup", OnBoard: "On your way home", Done: "Done", NoDriver: "No driver could make it, so you weren't charged." };
  return (
    <div className="ride-card">
      <div className="row gap"><b className="grow">Ride · {r.zone_name}</b><span className="num">{venueTime(r.at)} · {money(r.fare)}</span></div>
      <small>{say[r.stage] ?? r.stage}</small>
      {r.driver && <button className="driver-row" onClick={() => open(r.driver!.id)}><span className="dr-av">{r.driver.name.split(" ").map(x => x[0]).join("")}</span>
        <span className="grow"><b>{r.driver.name}</b><small>★ {r.driver.rating.toFixed(2)} · {r.driver.vehicle} · {r.driver.plate}</small></span></button>}
      {(r.stage === "Booked" || r.stage === "DriverAssigned") && <button className="btn ghost sm" onClick={cancel}>Cancel ride</button>}
    </div>
  );
}

// ---------- Live: the event, shared with everyone ----------

const optLabel = (id: string) => team(id) || ({ yes: "Yes", no: "No", tie: "Tie", none: "Nobody" } as Record<string, string>)[id] || id;

function Prediction({ v, refresh, toast }: Ctx) {
  const p = v.live.prediction;
  if (!p) return null;
  const pick = (o: string) => { const r = kernel.act({ do: "predict", pick: o }); if (!r.ok) toast(r.error); refresh(); };
  if (p.result) {
    return <div className="predict done"><span className="eyebrow">{p.question}</span><b>{optLabel(p.result)}{p.result === "tie" || p.result === "none" ? "" : ", it was"}</b>
      <small>{p.yours ? (p.won ? `You called it. +${p.points} points.` : "Not this time. The next question opens soon.") : `Answer the next one for ${p.points} points.`}</small></div>;
  }
  return (
    <div className="predict">
      <div className="row gap"><span className="eyebrow grow">{p.question} · {p.points} points if you're right</span>{p.open && <span className="num lock">Locks in {countdown(p.locks - v.clock.t)}</span>}</div>
      {p.yours ? <b>You picked {optLabel(p.yours)}. It settles when this part ends.</b> : p.open ? (
        <div className="seg fill big-seg" role="group" aria-label={p.question}>
          {p.options.map(o => <button key={o.id} onClick={() => pick(o.id)}>{optLabel(o.id)}</button>)}
        </div>
      ) : <b>Picks are locked. It settles when this part ends.</b>}
      {p.options.length > 2 && <div className="shares">{p.options.map(o => <div key={o.id} className={"share-row" + (p.yours === o.id ? " mine" : "")}><span className="grow">{optLabel(o.id)}</span><i style={{ width: `${Math.round(o.share * 100)}%` }} /><b className="num">{Math.round(o.share * 100)}%</b></div>)}</div>}
      {p.options.length <= 2 && <div className="split" aria-label="How the crowd picked">
        <i style={{ width: `${Math.round((p.options[0]?.share ?? 0) * 100)}%` }} />
        {p.options.map(o => <span key={o.id}>{Math.round(o.share * 100)}% {optLabel(o.id)}</span>)}
      </div>}
    </div>
  );
}

function LiveRow({ f, open, now, collect }: { f: LiveItem; open: (id: string) => void; now: number; collect: (e: string) => void }) {
  const who = f.name && f.party ? <button className="link-inline" onClick={() => open(f.party)}>{f.name}</button> : null;
  let body: React.ReactNode = f.text;
  if (f.kind === "score") body = <>{who ?? team(f.team)} <span className={"pts " + f.team}>{f.text}</span></>;
  if (f.kind === "moment") body = <><b>{momentText(f.text, f.team)}</b>{who && f.text !== "run" ? <> · {who}</> : null}</>;
  return (
    <li className={"signal " + f.kind + " " + f.team}>
      <time className="num">{venueTime(f.t)}</time><span className="dot" aria-hidden="true" />
      <span className="grow">{body}
        {feature("cards") && f.drop && (f.drop.open && !f.drop.collected ? (
          <span className="drop-inline"><button className="btn primary sm" onClick={() => collect(f.drop!.id)}>Collect the card</button><small className="num">{f.drop.left} of {f.drop.supply} left · {countdown(f.drop.open_until - now)}</small></span>
        ) : f.drop.collected || f.drop.yours > 0 ? <button className="link-inline small" onClick={() => open(f.drop!.id)}>In your collection</button>
          : <button className="link-inline small" onClick={() => open(f.drop!.id)}>{f.drop.rarity} card · {f.drop.floor ? `from ${f.drop.floor} pts` : "all collected"}</button>)}
      </span>
    </li>
  );
}

function Live(ctx: Ctx) {
  const { v, refresh, toast, go, open } = ctx;
  const c = v.clock;
  const [scope, setScope] = useState<"all" | "section">("all");
  const [text, setText] = useState("");
  const latest = v.live.event.find(e => e.kind === "moment");
  const fresh = latest && c.t - latest.t < 150000;
  const b = v.live.battle;
  const gap = b.ahead ? b.ahead.orders - b.orders_5m + 1 : 0;
  const chat = scope === "all" ? v.live.chat : v.live.chat_section;
  const send = (e: React.FormEvent) => { e.preventDefault(); const r = kernel.act({ do: "chat", text }); if (!r.ok) { toast(r.error); return; } setText(""); refresh(); };
  const collect = (e: string) => { const r = kernel.act({ do: "collect", edition: e }); toast(r.ok ? "It's yours. It's in Me with the rest of your collection." : "That drop just closed."); refresh(); };
  return (
    <>
      <section className="block event">
        <div className="big-score">
          <button className="bs-team home" onClick={() => open("hbk")}><span><TeamMark side="home" big />{BRAND.home.name}</span><b className="num">{c.home}</b></button>
          <div className="bs-clock num">{c.label}</div>
          <button className="bs-team away" onClick={() => open("msc")}><span><TeamMark side="away" big />{BRAND.away.name}</span><b className="num">{c.away}</b></button>
        </div>
        {fresh && (
          <div className={"moment-hero " + latest!.team}>
            <span className="eyebrow">{venueTime(latest!.t)}</span><b>{momentText(latest!.text, latest!.team)}</b>
            {feature("cards") && latest!.drop && latest!.drop.open && !latest!.drop.collected && (
              <button className="btn hero-cta" onClick={() => collect(latest!.drop!.id)}>Collect this moment · {latest!.drop.left} of {latest!.drop.supply} left</button>)}
          </div>
        )}
        {feature("predictions") && <Prediction {...ctx} />}
      </section>
      <section className="block">
        <h2>Play by play</h2>
        <ol className="signals">{v.live.event.slice(0, 16).map((f, i) => <LiveRow key={f.t + f.text + i} f={f} open={open} now={c.t} collect={collect} />)}</ol>
      </section>
      {(feature("chat") || feature("battle")) && <section className="block">
        {feature("chat") && <><div className="block-head"><h2>The crowd</h2>
          <div className="seg" role="group" aria-label="Whose chat"><button className={scope === "all" ? "on" : ""} onClick={() => setScope("all")}>Everyone</button><button className={scope === "section" ? "on" : ""} onClick={() => setScope("section")}>Section {sec(v.you.seat)}</button></div></div>
        <form className="say" onSubmit={send}>
          <input id="say" value={text} maxLength={140} onChange={e => setText(e.target.value)} placeholder="Say something" aria-label="Message the crowd" />
          <button className="btn primary sm" disabled={!text.trim()}>Send</button>
        </form>
        <ol className="chat">
          {chat.slice(0, 12).map((m, i) => <li key={m.t + m.text + i} className={m.you ? "you" : ""}>
            {m.you ? <b>You</b> : <button className="link-inline who" onClick={() => open(m.id)}>{m.who}{scope === "all" ? ` · ${sec(m.section)}` : ""}</button>}
            <span>{m.text}</span><time className="num">{venueTime(m.t)}</time></li>)}
          {chat.length === 0 && <li className="empty">Quiet so far. Start it.</li>}
        </ol>
        {v.provenance?.crowd === "simulated" && <p className="provenance">Pilot: most of the crowd here is simulated ({v.provenance.simulated_fans.toLocaleString()} fans), so the chat is lively even before launch.</p>}</>}
        {feature("battle") && <div className="battle">
          <div><span className="eyebrow">Section battle · last 5 min</span>
            <b className="big num">{b.rank ? `Section ${sec(v.you.seat)} is #${b.rank}` : `Section ${sec(v.you.seat)} isn't on the board`}</b>
            <small>{b.rank === 1 ? "Leading the arena." : b.ahead ? `${gap} order${gap === 1 ? "" : "s"} to pass section ${sec(b.ahead.section)}` : "One order puts you on it"}</small></div>
          <button className="btn primary" onClick={() => go("get")}>{b.rank === 1 ? "Hold the lead" : "Order to climb"}</button>
        </div>}
      </section>}
      <section className="block">
        <h2>Around the league</h2>
        <ul className="league">{v.live.league.map(g => (
          <li key={g.id}><button className="league-row" onClick={() => open(g.id)}>
            <span className="grow"><b>{g.home_name}</b> {g.home_score} · {g.away_score} <b>{g.away_name}</b></span><span className="tag">{g.status}</span></button></li>))}</ul>
      </section>
    </>
  );
}

// ---------- Me: what's yours, by its lifecycle ----------

/** Whose colors the app wears tonight: the home team by default, the other side, or Agape's own. */
function Colors({ c, toast, refresh }: { c: NonNullable<FanView["colors"]>; toast: (t: string) => void; refresh: () => void }) {
  const wear = (team: string) => { const r = kernel.act({ do: "wear", team }); if (!r.ok) toast(r.error); refresh(); };
  const choices = [...c.teams.map(t => ({ id: t.id, label: t.short, color: t.color })), { id: "agape", label: "Agape", color: "#B8174F" }];
  return (
    <section className="block">
      <div className="block-head"><h2>Your colors</h2><span className="sub">Tonight only</span></div>
      <div className="seg fill wear" role="group" aria-label="Whose colors the app wears">
        {choices.map(x => <button key={x.id} className={c.wearing === x.id ? "on" : ""} aria-pressed={c.wearing === x.id} onClick={() => wear(x.id)}><i style={{ background: x.color }} />{x.label}</button>)}
      </div>
    </section>
  );
}

function Me({ v, refresh, toast, open, settings }: Ctx) {
  const [rec, setRec] = useState<number | null>(null);
  const now = v.clock.t;
  const r = v.runner;
  const m = v.me;
  const got = m.badges.filter(b => b.earned).length;
  const fresh = m.receipts.filter(o => o.stage === "Settled" && o.delivered != null && now - o.delivered < 10 * 60_000).slice(0, 2);
  return (
    <>
      {fresh.length > 0 && (
        <section className="block arrived">
          <h2>Just arrived</h2>
          {fresh.map(o => <div key={o.id} className="arrived-card"><ItemArt name={o.items[0]?.name ?? ""} size={40} /><div className="grow"><b>{o.items.map(i => i.name).join(", ")}</b><small>{o.delivery ? `Reached section ${sec(o.seat)} at ${venueTime(o.delivered!)}` : `Picked up at ${o.stand}, ${venueTime(o.delivered!)}`}, {Math.max(1, Math.round((o.delivered! - o.placed) / 60_000))} min after you ordered</small></div>
            <After o={o} toast={toast} refresh={refresh} /></div>)}
        </section>
      )}
      {m.orders.length > 0 && (
        <section className="block">
          <h2>Your orders</h2>
          {m.orders.map(o => <CommitmentCard key={o.id} o={o} now={now} onCancel={() => { const x = kernel.act({ do: "cancel", order: o.id }); if (!x.ok) toast(x.error); refresh(); }} />)}
          {r && r.busy && (
            <div className="mini-map">
              <PlaceView m={{ ...v.map, runners: v.map.runners.filter(x => x.id === r.id) }} showRunners={false} heat={false} />
              <p className="sub">{runnerName(r.id)} is {r.to === v.you.seat ? "heading to you now" : /^s\d+$/.test(r.to) ? `dropping off at section ${sec(r.to)}, then you` : "picking up your order"}.</p>
            </div>
          )}
        </section>
      )}
      {(m.tickets.length > 0 || m.passes.length > 0 || m.rides.length > 0) && (
        <section className="block">
          <h2>Coming up</h2>
          {m.rides.filter(r => r.stage !== "Done").map(r => <RideCard key={r.id} r={r} open={open} cancel={() => { const x = kernel.act({ do: "cancel_ride", ride: r.id }); toast(x.ok ? "Ride cancelled. No charge." : x.error); refresh(); }} />)}
          {m.tickets.map(t => <div key={t.event + t.section}>
            <button className="ticket" onClick={() => open(t.event)}><div className="grow"><b>{t.title}</b><small>{t.date} · {t.time} · {t.section === "sFL" ? "Courtside" : `Section ${t.section.slice(1)}`}, row {t.seats[0][0]}, seats {t.seats.map(x => x.slice(1)).join(", ")}</small></div><span className="tq num">{t.qty}×</span></button>
            <Resell t={t} toast={toast} refresh={refresh} />
          </div>)}
          {m.passes.map(p => <div key={p.place} className={"ticket pass" + (p.used ? " used" : "")} onClick={() => open(p.operator)}><div className="grow"><b>{p.name}</b><small>{p.title}</small></div>
            <div className="code-wrap">{p.used ? <small>Used {venueTime(p.used)}</small> : <><span className="code num">{p.code}</span><small>show at the counter</small></>}</div></div>)}
        </section>
      )}
      {m.deals.length > 0 && (
        <section className="block">
          <div className="block-head"><h2>Your deals</h2><span className="sub">{m.deals.filter(d => d.next === "confirm").length ? "1 to confirm" : ""}</span></div>
          <div className="deal-list">{m.deals.slice(0, 6).map(d => <DealCard key={d.id} d={d} as="you" act={actWith(toast, refresh)} open={open} now={now} />)}</div>
        </section>
      )}
      <section className="block">
        <LevelPanel level={m.level} tier={m.tier} floor={m.tier_floor} next={m.next_tier} balance={m.balance} />
        <p className="passport-line">{m.passport?.venues?.length ? <>Your passport: {m.passport.venues.join(", ")} · {m.passport.nights} night{m.passport.nights === 1 ? "" : "s"}. </> : null}Your level and points go with you to every venue on Agape.</p>
      </section>
      {feature("upgrades") && (v.upgrades?.length || v.moved) ? (
        <section className="block">
          <div className="block-head"><h2>Your seat</h2><span className="sub">Section {sec(v.you.seat)}</span></div>
          {v.moved ? <p className="sub">You moved here at {venueTime(v.moved.at)}. Everything you order now comes to this section.</p> :
            v.upgrades!.slice(0, 3).map(u => <div key={u.id} className="upgrade-row"><div className="grow"><b>{u.title}</b><small>{u.blurb}{u.left != null ? ` · ${u.left} left tonight` : ""}</small></div>
              <button className="btn primary sm" disabled={!u.ok} onClick={() => { const r = kernel.act({ do: "upgrade", section: u.section }); toast(r.ok ? `Moved to section ${sec(u.section)}. Your orders follow you.` : r.error); refresh(); }}>{u.ok ? `Move · ${money(u.price)}` : (u.why_not ?? "Not now")}</button></div>)}
        </section>
      ) : null}
      {feature("colors") && v.colors && v.colors.teams.length > 0 && <Colors c={v.colors} toast={toast} refresh={refresh} />}
      {feature("cards") && <section className="block">
        <div className="block-head"><h2>Your collection</h2><span className="sub">{m.cards.length} card{m.cards.length === 1 ? "" : "s"}</span></div>
        {m.cards.length === 0 ? <p className="empty">Moments become cards. When something big happens in Live, collect it before it's gone.</p> :
          <div className="card-grid">{m.cards.map(x => (
            <div key={x.card.id} className="market-item"><MomentCard c={x.card} size="sm" onClick={() => open(x.card.id)} />
              <small>{x.listing ? `Listed at ${x.listing.price.toLocaleString()} pts` : x.count > 1 ? `You hold ${x.count}` : x.card.floor ? `Others ask ${x.card.floor.toLocaleString()}` : "Tap to sell"}</small></div>))}</div>}
      </section>}
      <section className="block">
        <div className="block-head"><h2>Badges</h2><span className="sub">{got} of {m.badges.length}</span></div>
        <div className="badges">{m.badges.map(b => <div key={b.id} className={"badge-tile" + (b.earned ? " on" : "")}><b>{b.name}</b><small>{b.how}</small></div>)}</div>
      </section>
      <section className="block">
        <h2>Your points</h2>
        <ul className="earned">{m.earned.slice(0, 8).map((e, i) => <li key={i}><span className="grow">{e.why}</span><b className={"num " + (e.points > 0 ? "plus" : "minus")}>{e.points > 0 ? "+" : ""}{e.points.toLocaleString()}</b><time className="num">{e.t ? venueTime(e.t) : ""}</time></li>)}</ul>
      </section>
      {(m.following.length > 0 || m.messages.length > 0) && (
        <section className="block">
          <h2>Your people</h2>
          {m.following.length > 0 && <div className="follow-list">{m.following.map(f => <button key={f.id} className="chip" onClick={() => open(f.id)}><i>{(f.name ?? "?").slice(0, 1)}</i>{f.name}</button>)}</div>}
          {m.messages.map(t => <button key={t.with} className="thread-row" onClick={() => open(t.with)}><b>{t.name}</b><span className="grow">{t.lines[t.lines.length - 1]?.text}</span></button>)}
        </section>
      )}
      <section className="block">
        <h2>Tonight</h2>
        <div className="pair">
          <div><span className="eyebrow">Spent</span><b className="big num">{money(m.spent + m.orders.reduce((a, o) => a + (o.charge ?? 0), 0))}</b><small>{m.receipts.length} receipt{m.receipts.length === 1 ? "" : "s"}{m.orders.length ? ` · ${money(m.orders.reduce((a, o) => a + (o.charge ?? 0), 0))} on the way` : ""}</small></div>
          <div><span className="eyebrow">Events</span><b className="big num">{m.events.length}</b><small>{m.events[0]?.name} · {m.events[0]?.venue}</small></div>
        </div>
        {m.holdings.length > 0 && <div className="holdings">{m.holdings.map(h => <span key={h.thing} className="chip"><ItemArt name={h.name} size={22} />{h.qty > 1 ? `${h.qty}× ` : ""}{h.name}</span>)}</div>}
        {m.receipts.map(o => (
          <div key={o.id} className="receipt">
            <button className="receipt-head" aria-expanded={rec === o.id} onClick={() => setRec(rec === o.id ? null : o.id)}><CommitmentCard o={o} now={now} compact /></button>
            {rec === o.id && o.breakdown && (
              <div className="lines">
                <LedgerLine label="Items" amount={o.breakdown.subtotal} />
                {o.breakdown.discount > 0 && <LedgerLine label="Offer" amount={-o.breakdown.discount} />}
                <LedgerLine label="Tax" amount={o.breakdown.tax} />
                {o.breakdown.fee > 0 && <LedgerLine label="Delivery" amount={o.breakdown.fee} />}
                {o.breakdown.tip > 0 && <LedgerLine label="Tip" amount={o.breakdown.tip} note="all to your runner" />}
                <LedgerLine label={o.stage === "Settled" ? "Charged" : "Not charged"} amount={o.stage === "Settled" ? o.charge : 0} strong />
              </div>
            )}
            {o.stage === "Settled" && <After o={o} toast={toast} refresh={refresh} />}
            {o.can_report && <Report id={o.id} toast={toast} refresh={refresh} />}
          </div>
        ))}
      </section>
      {settings}
    </>
  );
}

// ---------- Search: from anywhere ----------

function Search({ q, v, setSheet }: Ctx & { q: string }) {
  const n = q.trim().toLowerCase();
  const words = (i: Item) => [i.name, i.stand, i.category, i.adult ? "beer alcohol booze" : "", i.category === "drink" ? "drinks" : "", i.category === "merch" ? "gear merch shirt hat" : "", i.category === "food" ? "eat snack" : ""];
  const hits = v.items.filter(i => words(i).join(" ").toLowerCase().includes(n)).sort((a, b) => Number(b.available) - Number(a.available) || a.eta - b.eta);
  return (
    <section className="block">
      <div className="block-head"><h2>{hits.length ? `${hits.length} match${hits.length === 1 ? "" : "es"}` : "Nothing by that name"}</h2><span className="sub">Soonest to your seat first</span></div>
      {hits.map(it => <ItemRow key={it.id} it={it} now={v.clock.t} onPick={() => setSheet({ kind: "item", item: it.id })} />)}
    </section>
  );
}

// ---------- sheets ----------

const TIPS = [0, 100, 200, 300, 500];

function useQuote(args: Record<string, unknown>, deps: unknown[]) {
  const [q, setQ] = useState<any>(null);
  useEffect(() => { const r = kernel.act({ do: "quote", ...args }); setQ(r.ok ? r.quote : null); }, deps);
  return q;
}

/** Everything picked so far, one tap from checkout. */
function CartBar({ v, open }: { v: FanView; open: () => void }) {
  const lines = useCart();
  if (lines.length === 0) return null;
  const n = lines.reduce((a, l) => a + l.qty, 0);
  const total = lines.reduce((a, l) => a + l.qty * (v.items.find(i => i.id === l.thing)?.price ?? 0), 0);
  return <button className="cart-bar" onClick={open}><span className="cart-n num">{n}</span><span className="grow">View cart</span><b className="num">{money(total)}</b></button>;
}

/** One checkout for the whole cart: one delivery fee, the tip shared by whoever carries it. */
function CartSheet({ v, refresh, setSheet, toast, go }: Ctx) {
  const lines = useCart();
  const [delivery, setDelivery] = useState(true);
  const [tip, setTip] = useState(200);
  const [busy, setBusy] = useState(false);
  const key = JSON.stringify(lines) + delivery + tip;
  const [q, setQ] = useState<any>(null);
  useEffect(() => { if (!lines.length) return; const r = kernel.act({ do: "cart_quote", lines, delivery, tip }); setQ(r.ok ? r.quote : null); }, [key]);
  const items = lines.map(l => ({ ...l, it: v.items.find(i => i.id === l.thing)! })).filter(x => x.it);
  const blocked = items.filter(x => !x.it.available);
  const latest = items.reduce((m, x) => Math.max(m, x.it.eta), 0);
  const place = () => {
    setBusy(true);
    const r = kernel.act({ do: "cart", lines, delivery, tip });
    setBusy(false);
    if (!r.ok) { toast(whyNot(r.error) || r.error); refresh(); return; }
    cart.clear(); setSheet(null); refresh(); go("me");
    toast(delivery ? `Ordered. It'll be at section ${sec(v.you.seat)} around ${venueTime(latest)}.` : "Ordered. We'll tell you when each part is on the shelf.");
  };
  if (!lines.length) return <SheetFrame title="Your cart" onClose={() => setSheet(null)}><p className="empty">Your cart is empty. Tap anything on Get to add it.</p></SheetFrame>;
  return (
    <SheetFrame title="Your cart" onClose={() => setSheet(null)}>
      <div className="build">
        <div className="cart-lines">{items.map(x => (
          <div key={x.thing} className="cart-line">
            <ItemArt name={x.it.name} size={40} />
            <div className="grow"><b>{x.it.name}</b><small>{x.it.stand}{!x.it.available ? ` · ${whyNot(x.it.why_not)}` : ""}</small></div>
            <div className="qty"><button className="btn ghost sm" aria-label={`One fewer ${x.it.name}`} onClick={() => cart.set(x.thing, x.qty - 1)}>−</button><b className="num">{x.qty}</b><button className="btn ghost sm" aria-label={`One more ${x.it.name}`} onClick={() => cart.set(x.thing, x.qty + 1)}>+</button></div>
            <span className="num price-tag">{money(x.qty * x.it.price)}</span>
          </div>))}</div>
        <div className="seg fill" role="group" aria-label="How you get it">
          <button className={delivery ? "on" : ""} onClick={() => setDelivery(true)}>Bring it to my seat</button>
          <button className={!delivery ? "on" : ""} onClick={() => setDelivery(false)}>I'll pick it up</button>
        </div>
        {delivery && <Tips tip={tip} setTip={setTip} />}
        <p className="promise">{delivery ? `At section ${sec(v.you.seat)} by about ${venueTime(latest)}.${q?.stands > 1 ? ` It comes from ${q.stands} stands, so it may arrive in parts; you pay one delivery fee.` : ""}` : `Each part is ready at its stand. We'll tell you the moment it's on the shelf.`}</p>
        {q && <div className="lines">
          <LedgerLine label="Items" amount={q.items} /><LedgerLine label="Tax" amount={q.tax} />
          {q.fee > 0 && <LedgerLine label="Delivery" amount={q.fee} note="once for the cart" />}{q.tip > 0 && <LedgerLine label="Tip" amount={q.tip} note="all to your runners" />}
          <LedgerLine label="You pay" amount={q.charge} strong />
          <small className="pilot-note">Pilot: nothing is actually charged.</small>
        </div>}
        <button className="btn ghost sm" onClick={() => { cart.clear(); setSheet(null); }}>Empty the cart</button>
        <button className="btn primary block big" disabled={busy || !!v.halt || blocked.length > 0} onClick={place}>{v.halt ? "Service is paused" : blocked.length ? "Remove what's unavailable" : `Place order · ${money(q?.charge)}`}</button>
      </div>
    </SheetFrame>
  );
}

function SheetFrame({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  useEffect(() => { const k = (e: KeyboardEvent) => e.key === "Escape" && onClose(); addEventListener("keydown", k); return () => removeEventListener("keydown", k); }, []);
  return (
    <div className="sheet-back" onClick={e => e.target === e.currentTarget && onClose()}>
      <div className="sheet" role="dialog" aria-modal="true" aria-label={title}>
        <div className="sheet-head"><h2>{title}</h2><button className="btn ghost sm" onClick={onClose}>Close</button></div>
        {children}
      </div>
    </div>
  );
}

function Tips({ tip, setTip }: { tip: number; setTip: (n: number) => void }) {
  return (
    <div className="tips">
      <span className="eyebrow">Tip your runner · all of it goes to them</span>
      <div className="seg fill" role="group" aria-label="Tip">
        {TIPS.map(t => <button key={t} className={tip === t ? "on" : ""} onClick={() => setTip(t)}>{t ? money(t).replace(".00", "") : "None"}</button>)}
      </div>
    </div>
  );
}

const MOMENTS: Record<string, string> = { halftime: "Halftime", last_call: "Start of the 4th", final: "Final buzzer", second_half: "Start of the 3rd", start: "Tip-off" };

function ItemSheet({ v, refresh, setSheet, toast, go, id, at: preset }: Ctx & { id: string; at?: number }) {
  const item = v.items.find(i => i.id === id)!;
  const moments = v.get.later.filter(l => MOMENTS[l.kind] && l.t - v.clock.t > 8 * 60000).slice(0, 2);
  const [delivery0] = useState(true);
  const win = (() => { const r = kernel.act({ do: "window", thing: id, delivery: delivery0 }); return r.ok ? r as { earliest: number; closes: number } : null; })();
  const [at, setAt] = useState<number | null>(preset ?? null);
  const [qty, setQty] = useState(1);
  const [delivery, setDelivery] = useState(true);
  const [tip, setTip] = useState(200);
  const [giftTo, setGiftTo] = useState<string | null>(null);
  const [giftNote, setGiftNote] = useState("");
  const [first, setFirst] = useState(false);
  const friends = v.me.following.filter(f => f.kind === "fan");
  const quote = useQuote({ thing: id, qty, delivery, tip: delivery ? tip : 0 }, [id, qty, delivery, tip]);
  const place = () => {
    const r = giftTo
      ? kernel.act({ do: "gift", thing: id, qty, to: giftTo, note: giftNote, tip })
      : kernel.act({ do: "order", thing: id, qty, delivery, tip: delivery ? tip : 0, at });
    if (r.ok && first) { const p = kernel.act({ do: "prioritize", order: Number(r.id) }); if (!p.ok) toast(p.error === "CantAfford" ? "Not enough points to go first; it's in line as usual." : p.error); }
    if (r.ok && giftTo) { toast(`Sent to ${giftTo.startsWith("s") && /^s\d+$/.test(giftTo) ? `section ${sec(giftTo)}` : friends.find(f => f.id === giftTo)?.name ?? "them"}.`); setSheet(null); refresh(); return; }
    if (!r.ok) { toast(r.error === "SoldOut" ? "That just sold out." : r.error === "TooSoon" ? "It can't reach you that soon. Pick a later time." : whyNot(r.error) || r.error); refresh(); return; }
    toast(at ? `Done. It'll be at section ${sec(v.you.seat)} around ${venueTime(at)}.` : delivery ? `Ordered. It'll be at section ${sec(v.you.seat)} around ${venueTime(item.eta)}.` : "Ordered. We'll tell you when it's on the shelf.");
    setSheet(null); refresh(); go("me");
  };
  if (item.why_not === "AgeUnverified") {
    const yes = () => { const r = kernel.act({ do: "attest_age" }); if (!r.ok) toast(r.error); refresh(); };
    return (
      <SheetFrame title={item.name} onClose={() => setSheet(null)}>
        <div className="build">
          <div className="build-item"><ItemArt name={item.name} size={72} /><div><b className="num price-big">{money(item.price)}</b><p className="sub">{item.stand} · level {item.level}</p></div></div>
          <div className="age-gate"><b>You need to be 21 or older for this.</b><p>Confirm it once and drinks unlock for tonight. Your runner checks your ID at your seat when the first one arrives.</p></div>
          <button className="btn primary big wide" onClick={yes}>I'm 21 or older</button>
        </div>
      </SheetFrame>
    );
  }
  return (
    <SheetFrame title={item.name} onClose={() => setSheet(null)}>
      <div className="build">
        <div className="build-item"><ItemArt name={item.name} size={72} /><div><b className="num price-big">{money(item.price)}</b><p className="sub">{item.stand} · level {item.level}{item.left != null ? ` · ${item.left} left` : ""}</p></div></div>
        <div className="seg fill" role="group" aria-label="How you get it">
          <button className={delivery ? "on" : ""} onClick={() => setDelivery(true)}>Bring it to my seat</button>
          <button className={!delivery ? "on" : ""} onClick={() => setDelivery(false)}>I'll pick it up</button>
        </div>
        <div className="tips"><span className="eyebrow">When</span>
          <div className="seg fill" role="group" aria-label="When">
            <button className={at == null ? "on" : ""} onClick={() => setAt(null)}>As soon as it can</button>
            <button className={at != null ? "on" : ""} onClick={() => win && setAt(at ?? Math.min(win.closes, (moments[0]?.t ?? win.earliest + 20 * 60000)))}>Pick a time</button>
          </div>
          {at != null && win && <WhenPicker from={win.earliest} to={win.closes} value={Math.max(at, win.earliest)} set={setAt} marks={v.get.later.map(l => ({ t: l.t, label: MOMENTS[l.kind] ?? l.kind }))} />}
        </div>
        <div className="qty"><span className="eyebrow">How many</span>
          <div className="stepper"><button onClick={() => setQty(Math.max(1, qty - 1))} aria-label="One fewer">−</button><b className="num">{qty}</b><button onClick={() => { if (qty >= 4) toast("Up to 4 at a time. Add more to your cart."); setQty(Math.min(4, qty + 1)); }} aria-label="One more">+</button></div></div>
        {delivery && <Tips tip={tip} setTip={setTip} />}
        {feature("gifting") && delivery && !item.adult && at == null && <div className="gift">
          <label className="check"><input type="checkbox" checked={giftTo != null} onChange={e => setGiftTo(e.target.checked ? (friends[0]?.id ?? "s101") : null)} /> Send it to someone else</label>
          {giftTo != null && <>
            <select value={giftTo} onChange={e => setGiftTo(e.target.value)} aria-label="Who it's for">
              {friends.length > 0 && <optgroup label="People you follow">{friends.map(f => <option key={f.id} value={f.id}>{f.name}</option>)}</optgroup>}
              <optgroup label="A section">{Array.from({ length: 48 }, (_, i) => i < 24 ? `s${101 + i}` : `s${177 + i}`).map(x => <option key={x} value={x}>Section {x.slice(1)}</option>)}</optgroup>
            </select>
            <input value={giftNote} maxLength={140} onChange={e => setGiftNote(e.target.value)} placeholder="Add a note (optional)" aria-label="Note" />
          </>}
        </div>}
        {feature("priority") && at == null && delivery && v.me.balance >= 200 && <label className="check first"><input type="checkbox" checked={first} disabled={v.me.balance < 200} onChange={e => setFirst(e.target.checked)} /> Go first · 200 pts, only if it arrives <small>{v.me.balance < 200 ? `you have ${v.me.balance}` : `you have ${v.me.balance.toLocaleString()}`}</small></label>}
        <p className="promise">{giftTo ? `It goes to ${/^s\d+$/.test(giftTo) ? `section ${sec(giftTo)}` : friends.find(f => f.id === giftTo)?.name ?? "them"}, with your note. Their runner brings it straight there.` : at && !delivery ? `We start it so it's ready at ${item.stand} around ${venueTime(at)}.` : at ? `We hold it now and start it so it reaches section ${sec(v.you.seat)} around ${venueTime(at)}. If it's more than 10 minutes late, ${money(300)} comes back to you automatically.` : delivery ? `At section ${sec(v.you.seat)} around ${venueTime(item.eta)}. If it takes more than 30 minutes, ${money(300)} comes back to you automatically.` : `Ready at ${item.stand}. We'll tell you the moment it's on the shelf.`}</p>
        {quote && <div className="lines">
          <LedgerLine label="Items" amount={quote.subtotal} /><LedgerLine label="Tax" amount={quote.tax} />
          {quote.fee > 0 && <LedgerLine label="Delivery" amount={quote.fee} />}{quote.tip > 0 && <LedgerLine label="Tip" amount={quote.tip} />}
          <LedgerLine label="You pay" amount={quote.charge} strong />
          <small className="pilot-note">Pilot: nothing is actually charged.</small>
        </div>}
        <div className="sheet-actions">
          {at == null && !giftTo && !first && <button className="btn big" disabled={!!v.halt} onClick={() => { cart.add(id, qty); toast(`${qty > 1 ? `${qty}× ` : ""}${item.name} is in your cart.`); setSheet(null); }}>Add to cart</button>}
          <button className="btn primary big grow" disabled={!!v.halt} onClick={place}>{v.halt ? "Service is paused" : `${giftTo ? "Send it" : "Order now"} · ${money(quote?.charge)}`}</button>
        </div>
      </div>
    </SheetFrame>
  );
}

function CheckoutSheet({ v, refresh, setSheet, toast, offer, claim }: Ctx & { offer: Offer; claim: string }) {
  const live = v.get.offers.find(o => o.id === offer.id);
  const until = live?.claim?.until ?? offer.claim?.until ?? v.clock.t + 120000;
  const [tip, setTip] = useState(200);
  const quote = useQuote({ claim, tip }, [claim, tip]);
  const place = () => {
    const r = kernel.act({ do: "checkout", claim, tip });
    if (!r.ok) { toast(r.error === "Expired" ? "Your hold ran out, so it went back for the next fan." : r.error); refresh(); setSheet(null); return; }
    toast(`On its way to section ${sec(v.you.seat)}.`); setSheet(null); refresh();
  };
  return (
    <SheetFrame title="It's yours. Check out" onClose={() => setSheet(null)}>
      <div className="build">
        <div className="build-item"><ItemArt name={offer.name} size={72} /><div><h3>{offer.name}</h3><p className="sub">{offer.stand}</p></div></div>
        <p className="hold"><b className="num">{countdown(until - v.clock.t)}</b> Held for you. Nobody else can take it while you check out. The game clock waits while this is open.</p>
        <Tips tip={tip} setTip={setTip} />
        {quote && <div className="lines">
          <LedgerLine label="Price" amount={quote.subtotal} />
          {quote.discount > 0 && <LedgerLine label="Offer" amount={-quote.discount} note={quote.payer ? `paid by ${BRAND.sponsors[quote.payer] ?? quote.payer}` : ""} />}
          <LedgerLine label="Tax" amount={quote.tax} /><LedgerLine label="Delivery" amount={quote.fee} />
          {quote.tip > 0 && <LedgerLine label="Tip" amount={quote.tip} />}
          <LedgerLine label="You pay" amount={quote.charge} strong />
          <small className="pilot-note">Pilot: nothing is actually charged.</small>
        </div>}
        <button className="btn primary block big" disabled={!!v.halt} onClick={place}>{v.halt ? "Service is paused" : `Place order · ${money(quote?.charge)}`}</button>
      </div>
    </SheetFrame>
  );
}

export { team, LevelBadge };

/** Something wrong with an order: say what, and the money comes back from whoever was at fault. */
function Report({ id, toast, refresh }: { id: number; toast: (s: string) => void; refresh: () => void }) {
  const [open, setOpen] = useState(false);
  const [note, setNote] = useState("");
  const send = (reason: string) => {
    const r = kernel.act({ do: "report", order: id, reason, note });
    if (!r.ok) { toast(r.error); return; }
    toast("Thanks for telling us. Check the receipt for what happens next.");
    setOpen(false); refresh();
  };
  if (!open) return <button className="link-inline small report-link" onClick={() => setOpen(true)}>Something wrong with this order?</button>;
  return (
    <div className="report">
      <input value={note} maxLength={280} onChange={e => setNote(e.target.value)} placeholder="Anything we should know (optional)" aria-label="Details" />
      <div className="report-reasons">
        <button className="btn ghost sm" onClick={() => send("never_arrived")}>It never arrived</button>
        <button className="btn ghost sm" onClick={() => send("wrong_item")}>Wrong or missing item</button>
        <button className="btn ghost sm" onClick={() => send("quality")}>Cold, spilled or not right</button>
      </div>
      <small className="sub">You can report up to 30 minutes after delivery. Refunds go back to your card.</small>
    </div>
  );
}

/** After an order: rate who brought it and the kitchen, once each, and order the same again. */
function After({ o, toast, refresh }: { o: Order; toast: (s: string) => void; refresh: () => void }) {
  const rated = (who: string | null) => (o.rated ?? []).find(r => r.who === who)?.stars;
  const rate = (who: string, stars: number) => { const r = kernel.act({ do: "rate", order: o.id, who, stars }); toast(r.ok ? "Thanks. That goes straight to them." : r.error); refresh(); };
  const again = () => { const r = kernel.act({ do: "reorder", order: o.id }); toast(r.ok ? "Same again, on its way." : r.error); refresh(); };
  const Stars = ({ who, label }: { who: string; label: string }) => {
    const got = rated(who);
    return <div className="rate-row"><span className="grow">{label}</span>{[1, 2, 3, 4, 5].map(n =>
      <button key={n} className={"star" + ((got ?? 0) >= n ? " on" : "")} disabled={got != null} aria-label={`${n} star${n === 1 ? "" : "s"} for ${label}`} onClick={() => rate(who, n)}>★</button>)}</div>;
  };
  return (
    <div className="after">
      {o.runner && <Stars who={o.runner} label={runnerName(o.runner)} />}
      <Stars who={o.stand_id} label={o.stand} />
      <button className="btn ghost sm" onClick={again}>Order this again</button>
    </div>
  );
}

/** Can't go any more? Sell the seats to another fan, at no more than 120% of what they cost. */
function Resell({ t, toast, refresh }: { t: any; toast: (s: string) => void; refresh: () => void }) {
  const [open, setOpen] = useState(false);
  const [price, setPrice] = useState(String(Math.floor(t.resale_cap / 100)));
  if (t.listing) return <div className="resell"><span className="grow sub">On sale to other fans for {money(t.listing.price)}</span>
    <button className="btn ghost sm" onClick={() => { const r = kernel.act({ do: "unlist_tickets", listing: t.listing.id }); toast(r.ok ? "Taken off sale. They're yours again." : r.error); refresh(); }}>Take off sale</button></div>;
  if (!open) return <button className="link-inline small" onClick={() => setOpen(true)}>Can't make it? Sell them to another fan</button>;
  return <form className="resell" onSubmit={e => { e.preventDefault(); const r = kernel.act({ do: "resell", event: t.event, section: t.section, price: Math.round(Number(price) * 100) }); toast(r.ok ? "On sale. If they sell, the money comes back to you, less 10%." : r.error); if (r.ok) setOpen(false); refresh(); }}>
    <span className="sub">$</span><input type="number" min={20} max={t.resale_cap / 100} step={1} value={price} onChange={e => setPrice(e.target.value)} aria-label="Price for all of them" />
    <button className="btn primary sm">Put on sale</button><small className="sub">up to {money(t.resale_cap)}</small></form>;
}

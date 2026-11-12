// The fan relates to the world in three ways, each with its own natural order:
//   Get   things and offers, by how soon they reach you (zooming out from your seat)
//   Live  the event, shared with everyone, by the event's own time (your section is a filter)
//   Me    what's yours, by its lifecycle (on the way, then done)
// The score, your live orders and search stay on every screen.

import { React, useEffect, useRef, useState } from "./r";
import { actWith, Deal, DealCard } from "./deals";
import { Card, LevelBadge, LevelPanel, MomentCard, Page } from "./pages";
import { kernel, world, sectionIds } from "./kernel";
import { wearColor } from "./themes";
import { cart, useCart } from "./cart";
import { ItemArt } from "./Art";
import { payReady, concourse, feature, Icon, isConcert, Opt, TeamMark, W,
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
    orders: Order[]; receipts: Order[]; points: number; spent: number; credit?: number; tip?: { bps: number; min: number }; delivery?: { fee: number; free_over: number }; holdings: { thing: string; name: string; qty: number }[];
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
type Space = "get" | "deals" | "play" | "people" | "me" | "live";
type Ctx = { v: FanView; refresh: () => void; setSheet: (s: Sheet) => void; toast: (s: string) => void; go: (s: Space) => void; open: (id: string) => void; settings?: React.ReactNode; setting: (s: string | null) => void };

/** The tabs, by what's switched on: Get and Me always; the others arrive as the venue's level grows. */
const SPACES: { id: Space; icon: "get" | "deals" | "play" | "people" | "me"; label: string; when: () => boolean }[] = [
  { id: "get", icon: "get", label: "Get", when: () => true },
  { id: "deals", icon: "deals", label: "Deals", when: () => feature("offers") || feature("tickets") || feature("upgrades") || feature("around") },
  { id: "play", icon: "play", label: "Play", when: () => feature("predictions") || feature("battle") || feature("cards") },
  { id: "people", icon: "people", label: "People", when: () => feature("chat") || feature("players") },
  { id: "me", icon: "me", label: "Me", when: () => true },
];
/** Words under the icons for a fan's first week; after that, the icons alone. */
function useFirstWeek() {
  const [on] = useState(() => { try { const k = "agape.first"; const f = Number(localStorage.getItem(k)) || Date.now(); localStorage.setItem(k, String(f)); return Date.now() - f < 7 * 864e5; } catch { return true; } });
  return on;
}
const sec = (s: string) => s.slice(1);

/** Lets anything deep in the app open an item to order. */
const SheetCtx = React.createContext<((s: Sheet) => void) | null>(null);

export function FanApp({ v, refresh, sheet, setSheet, toast, settings }: Omit<Ctx, "go" | "open" | "setting"> & { sheet: Sheet }) {
  const [space, setSpace] = useState<Space>("get");
  const [q, setQ] = useState("");
  const body = useRef<HTMLDivElement>(null);
  useSecond();
  const [pages, setPages] = useState<string[]>([]);
  const setTucked = (_: boolean) => {};
  const [liveOpen, setLiveOpen] = useState(false);
  const [setting, setSetting] = useState<string | null>(null);
  // the phone's back button closes what's open (a sheet, a page) instead of leaving the app
  const overlay = (sheet ? 1 : 0) + pages.length + (liveOpen ? 1 : 0) + (setting ? 1 : 0);
  const depth = useRef(0), fromBack = useRef(false), skipPop = useRef(0);
  const live = useRef({ sheet, pages, liveOpen, setting }); live.current = { sheet, pages, liveOpen, setting };
  useEffect(() => {
    if (overlay > depth.current) history.pushState({ agape: overlay }, "");
    else if (overlay < depth.current && !fromBack.current) { skipPop.current += depth.current - overlay; history.go(overlay - depth.current); }
    fromBack.current = false; depth.current = overlay;
  }, [overlay]);
  useEffect(() => {
    const on = () => {
      if (skipPop.current > 0) { skipPop.current--; return; }
      fromBack.current = true;
      if (live.current.pages.length) setPages(p => p.slice(0, -1)); else if (live.current.sheet) setSheet(null); else if (live.current.setting) setSetting(null); else if (live.current.liveOpen) setLiveOpen(false); else fromBack.current = false;
    };
    addEventListener("popstate", on); return () => removeEventListener("popstate", on);
  }, []);
  const go = (s: Space) => { if (s === "live") { setLiveOpen(true); return; } setLiveOpen(false); setSpace(s); setQ(""); setPages([]); setTucked(false); body.current?.scrollTo({ top: 0 }); };
  const open = (id: string) => { setLiveOpen(false); setPages(p => [...p, id]); };
  const ctx: Ctx = { v, refresh, setSheet, toast, go, open, settings, setting: setSetting };
  const tabs = SPACES.filter(t => t.when());
  const labels = useFirstWeek();
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
    <SheetCtx.Provider value={setSheet}>
    <div className="phone">
      {lead && space !== "me" && (
        <button className="order-pill" onClick={() => go("me")}>
          <ItemArt name={lead.items[0]?.name ?? ""} size={26} />
          <span className="grow pill-txt"><b>{lead.items[0]?.name}{v.orders.length > 1 ? ` +${v.orders.length - 1} more` : ""}</b><small>{stageWord(lead.stage)} · {lead.delivery ? "here by" : "ready by"} <span className="num">{venueTime(lead.eta)}</span></small></span>
        </button>
      )}
      <div className="scope-body" ref={body}>
        {space === "get" && !q && <EventAnchor {...ctx} />}
        {space === "get" && <div className="finder">
          <input id="find" type="search" placeholder="Search food, drinks, gear…" value={q} onChange={e => setQ(e.target.value)} aria-label="Find anything" />
        </div>}
        {v.halt && <div className="halt-banner" role="alert"><b>Service is paused</b><p>{v.halt.reason}</p><small>Nothing new can be ordered right now. Anything not yet being made was cancelled, and you weren't charged for it.</small></div>}
        {q.trim() ? <Search q={q.trim()} {...ctx} /> : space === "get" ? <Get {...ctx} /> : space === "deals" ? <Deals {...ctx} /> : space === "play" ? <Play {...ctx} /> : space === "people" ? <People {...ctx} /> : <Me {...ctx} />}
      </div>
      {tabs.length > 1 && <nav className={"space-bar icons" + (labels ? " labelled" : "")} aria-label="Spaces">
        {tabs.map(t => (
          <button key={t.id} className={space === t.id && !q ? "on" : ""} aria-current={space === t.id} aria-label={t.label} onClick={() => go(t.id)}>
            {t.id === "me" ? <span className="me-av">{(v.me as any).name?.slice?.(0, 1) ?? <Icon name="me" />}</span> : <Icon name={t.icon} />}
            {labels && <small>{t.label}</small>}
            {t.id === "me" && v.orders.length > 0 && <i className="badge">{v.orders.length}</i>}
            {t.id === "play" && v.live.prediction?.open && !v.live.prediction.yours && feature("predictions") && <i className="dot-badge" aria-label="A pick is open" />}
          </button>
        ))}
      </nav>}
      {liveOpen && <LiveSheet {...ctx} close={() => setLiveOpen(false)} />}
      {setting && <SettingsPage which={setting} {...ctx} close={() => setSetting(null)} />}
      {pages.length > 0 && <Page key={pages.join("/")} id={pages[pages.length - 1]} depth={pages.length} close={() => setPages([])} back={() => setPages(p => p.slice(0, -1))} open={open} toast={toast} now={c.t}
        acts={{ order: item => { setPages([]); setSheet({ kind: "item", item }); }, ride: zone => { setPages([]); setSheet({ kind: "ride", zone }); }, refreshAll: refresh }} />}
      {!sheet && pages.length === 0 && <CartBar v={v} open={() => setSheet({ kind: "cart" })} />}
      {sheet?.kind === "cart" && <CartSheet {...ctx} />}
      {sheet?.kind === "item" && <ItemSheet {...ctx} id={sheet.item} at={sheet.at} />}
      {sheet?.kind === "ride" && <RideSheet {...ctx} zone={sheet.zone} />}
      {sheet?.kind === "checkout" && <CheckoutSheet {...ctx} offer={sheet.offer} claim={sheet.claim} />}
    </div>
    </SheetCtx.Provider>
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
        <small>{it.available ? <><span className="eta">At your seat {venueTime(it.eta)}</span> · {rel(it.eta, now)}</> : whyNot(it.why_not)}</small>
        {(showStand || it.left != null) && <small className="from">{[showStand ? it.stand : null, it.left != null ? `${it.left} left` : null].filter(Boolean).join(" · ")}</small>}</span>
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
    return <Callout eyebrow={`${W.Half} ${rel(next.t, now)}`} text={`Lines get long at ${W.half}. Order ahead and it's at your seat when ${W.half} starts.`} cta={`Have it waiting at ${W.half}`} onClick={() => setSheet({ kind: "item", item: fastest.id, at: next.t })} />;
  if (next.kind === "last_call" && mins <= 10 && drink)
    return <Callout eyebrow={`Last call ${rel(next.t, now)}`} text={`Beer stops when ${W.lastCallWhen}. ${drink.name} reaches you by ${venueTime(drink.eta)}.`} cta={`Get a ${drink.name.toLowerCase()}`} onClick={() => setSheet({ kind: "item", item: drink.id })} />;
  return null;
}

function Callout({ eyebrow, text, cta, onClick }: { eyebrow: string; text: string; cta: string; onClick: () => void }) {
  return <div className="callout"><span className="eyebrow">{eyebrow}</span><p>{text}</p><button className="btn primary" onClick={onClick}>{cta}</button></div>;
}

const ALL_CATS = [["all", "Everything"], ["food", "Food"], ["drink", "Drinks"], ["merch", "Gear"]];

/** The fan's name as they gave it (in onboarding, or in Settings). */
export function profileName(): string { try { return JSON.parse(localStorage.getItem("agape.profile") || "{}").name || ""; } catch { return ""; } }
function setProfile(p: Record<string, unknown>) { try { const o = JSON.parse(localStorage.getItem("agape.profile") || "{}"); localStorage.setItem("agape.profile", JSON.stringify({ ...o, ...p })); } catch { /* kept for this visit only */ } }
/** A fan using Agape without an account: everything works, and nothing is kept after tonight. */
export function isGuest(): boolean { try { return !!JSON.parse(localStorage.getItem("agape.profile") || "{}").guest; } catch { return false; } }
function pref<T>(k: string, d: T): T { try { const o = JSON.parse(localStorage.getItem("agape.prefs") || "{}"); return k in o ? o[k] : d; } catch { return d; } }
function setPref(k: string, v: unknown) { try { const o = JSON.parse(localStorage.getItem("agape.prefs") || "{}"); o[k] = v; localStorage.setItem("agape.prefs", JSON.stringify(o)); } catch { /* fine */ } }

/** A first-night hello that leads with the one thing Agape is for: something good, at your seat, soon. */
function Welcome({ v, setSheet }: Ctx) {
  const key = `agape.welcomed.${v.you.id}`;
  const [gone, setGone] = useState(() => { try { return localStorage.getItem(key) === "1"; } catch { return false; } });
  const fresh = v.me.orders.length === 0 && v.orders.length === 0 && v.me.receipts.length === 0 && v.clock.phase !== "postgame";
  const pick = v.get.fastest.find(i => i.category === "food" && i.available) ?? v.get.fastest[0];
  if (gone || !fresh || !pick) return null;
  const close = () => { setGone(true); try { localStorage.setItem(key, "1"); } catch { /* shown again next visit */ } };
  const name = profileName().split(" ")[0];
  return (
    <section className="welcome" aria-label="Welcome">
      <button className="welcome-x" onClick={close} aria-label="Dismiss"><Icon name="close" size={18} /></button>
      <p className="eyebrow">Welcome to {BRAND.venue}{name ? `, ${name}` : ""}</p>
      <h2>Anything here, brought to section {sec(v.you.seat)}.</h2>
      <p>A {pick.name.toLowerCase()} could be at your seat by <b>{venueTime(pick.eta)}</b>. Every time you see is when it reaches you, so you never have to leave the moment.</p>
      <button className="btn welcome-cta" onClick={() => { close(); setSheet({ kind: "item", item: pick.id }); }}>Get a {pick.name.toLowerCase()} · {money(pick.price)}</button>
      {feature("points") && <small className="welcome-pts">You start with {v.me.balance.toLocaleString()} points, and every order earns more.</small>}
    </section>
  );
}

/** One line of the event, as it happens. */
function feedLine(f: LiveItem): React.ReactNode {
  if (f.kind === "song") return f.text;
  if (f.kind === "score") return <>{f.name ?? team(f.team)} <span className={"pts " + f.team}>{f.text}</span></>;
  if (f.kind === "moment") return <b>{momentText(f.text, f.team)}</b>;
  return f.text;
}

/** The event, folded into one line at the top of Get: the score and the latest thing that happened. Tap to open it. */
function EventAnchor({ v, go }: Ctx) {
  const c = v.clock;
  const f = v.live.event[0];
  return (
    <button className="anchor" onClick={() => go("live")} aria-label={isConcert() ? "Open the show" : "Open the game"}>
      {isConcert() ? <span className="an-score show"><span className="an-act">{(world() as any)?.event?.title?.split(":")[0] ?? BRAND.home.name}</span><span className="es-clock">{c.label}</span></span> : <span className="an-score">
        <span className="es-team home"><TeamMark side="home" />{BRAND.home.abbr} <b key={c.home} className="num score-bump">{c.home}</b></span>
        <span className="es-clock num">{c.label}</span>
        <span className="es-team away"><b key={c.away} className="num score-bump">{c.away}</b> {BRAND.away.abbr}<TeamMark side="away" /></span>
      </span>}
      {f && <span className="an-feed" key={f.t + f.text}><i className="an-dot" aria-hidden="true" /><span className="grow">{feedLine(f)}</span><Icon name="chev" size={16} /></span>}
    </button>
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
      <Nudge {...ctx} />
      <section className="block">
        <h2>Food and drinks soonest to section {sec(v.you.seat)}</h2>
        {v.get.fastest.map(it => <ItemRow key={it.id} it={it} now={now} onPick={() => setSheet({ kind: "item", item: it.id })} />)}
      </section>
      <div className="cats-bar"><div className="cats chips" role="group" aria-label="What you want">
        {ALL_CATS.map(([id, label]) => <button key={id} className={cat === id ? "on" : ""} onClick={() => { setCat(id); requestAnimationFrame(() => document.querySelector(".cats-bar")?.scrollIntoView({ block: "start", behavior: "smooth" })); }}>{label}</button>)}
      </div></div>
      {stands.map(standBlock)}
    </>
  );
}

function Events({ v, open }: Ctx) {
  return (
    <section className="block">
      <div className="block-head"><h2>Next at <button className="link-inline" onClick={() => open("venue")}>{BRAND.venue}</button> <Opt k="tickets" /></h2></div>
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
  const [confirm, setConfirm] = useState<string | null>(null);
  const act = (a: any, ok: string) => { const r = kernel.act(a); toast(r.ok ? ok : r.error === "CantAfford" ? "Not enough points yet." : r.error); refresh(); };
  const fin = v.get.around.find(a => a.final_at)?.final_at;
  const ride = v.me.rides.find(r => r.stage !== "Done");
  return (
    <section className="block">
      <div className="block-head"><h2>Around {BRAND.venue} <Opt k="around" /></h2><span className="sub">You have {v.me.balance.toLocaleString()} pts</span></div>
      {v.get.around.map(a => (
        <div key={a.id} className={"around-row " + a.kind}>
          <div className="grow">
            <button className="link-inline" onClick={() => open(a.operator ?? a.id)}>{a.kind === "partner" ? a.name : a.operator_name ?? a.name}</button>
            <small>{a.kind === "partner" ? a.blurb : `${a.name}. ${a.blurb}`}</small>
            {(a.kind === "ride" || a.kind === "parking") && fin ? <small className="when">Leave at {W.end} ({venueTime(fin)}) and you're there by {venueTime(fin + a.walk_min * 60000)}.</small>
              : <small className="when">{a.walk_min} min walk</small>}
          </div>
          {a.service?.kind === "ride" && (ride ? <span className="tag good">Booked {venueTime(ride.at)}</span> : <button className="btn sm primary" onClick={() => setSheet({ kind: "ride", zone: a.id })}>Book a ride</button>)}
          {a.service?.kind === "parking" && (a.service.yours ? <span className="tag good">Exit paid</span> :
            <button className="btn sm primary" onClick={() => { if (confirm !== a.id) { setConfirm(a.id); return; } setConfirm(null); if (!payReady(toast)) return; act({ do: "parking", lot: a.id }, "Paid. Show the code in Me at the gate."); }}>{confirm === a.id ? `Pay ${money(a.service.price)} now` : `Prepay exit · ${money(a.service.price)}`}</button>)}
          {a.perk && (a.perk.yours ? <span className="tag good">Unlocked</span> : (
            <button className="btn sm primary perk" disabled={!a.perk.affordable || a.perk.left < 1} onClick={() => act({ do: "unlock", place: a.id }, "Unlocked. Show the code in Me at the counter.")}>
              <span>{a.perk.title}</span><b className="num">{a.perk.affordable ? `${a.perk.cost} pts · ${a.perk.left} left` : `${(a.perk.cost - v.me.balance).toLocaleString()} more pts to unlock`}</b>
            </button>))}
        </div>
      ))}
    </section>
  );
}



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
        {marks.filter(m => m.t > from && m.t < to).map(m => <button key={m.t} className="wp-mark" style={{ left: `${pct(m.t)}%` }} onClick={() => set(m.t)} title={m.label}><i /><span>{W.short[m.label] ?? m.label}</span></button>)}
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
  const book = () => { if (!payReady(toast)) return; const r = kernel.act({ do: "ride", zone, at }); if (!r.ok) { toast(r.error === "NoDriver" ? "Every driver is promised around then. Try another time." : r.error); refresh(); return; } toast(`Booked. A driver will be assigned 8 minutes before ${venueTime(at)}.`); setSheet(null); refresh(); };
  return (
    <SheetFrame title="Book a ride" onClose={() => setSheet(null)}>
      <div className="build">
        <p className="sub">With <button className="link-inline" onClick={() => { setSheet(null); open(a.operator!); }}>{a.operator_name}</button> from {a.name}, {a.walk_min} min from your seat. {a.service!.drivers - a.service!.busy} drivers free right now.</p>
        <span className="eyebrow">Pickup time</span>
        <WhenPicker from={from} to={to} value={at} set={setAt} marks={[{ t: fin, label: "Final buzzer" }]} />
        <div className="lines"><LedgerLine label="Fare" amount={fare} note={fare > 1890 ? `busier right after ${W.end}` : ""} /><LedgerLine label="You pay when the ride ends" amount={fare} strong /></div>
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
      <small>{p.yours ? (p.won ? `You called it. +${p.points} points.` : (v.clock.phase === "postgame" ? "Not this time." : "Not this time. The next question opens soon.")) : v.clock.phase === "postgame" ? "That was the last call of the night." : `Answer the next one for ${p.points} points.`}</small></div>;
  }
  return (
    <div className="predict">
      <div className="row gap"><span className="eyebrow grow">{p.question} · {p.points} points if you're right <Opt k="predictions" /></span>{p.open && <span className="num lock">Locks in {countdown(p.locks - v.clock.t)}</span>}</div>
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

/** The event, opened from the anchor on Get: the score, the moment, everything that's happened. */
function LiveSheet(ctx: Ctx & { close: () => void }) {
  const { v, refresh, toast, open, close } = ctx;
  const c = v.clock;
  const latest = v.live.event.find(e => e.kind === "moment");
  const fresh = latest && c.t - latest.t < 150000;
  const collect = (e: string) => { const r = kernel.act({ do: "collect", edition: e }); toast(r.ok ? "It's yours. Find it in Play, with the rest of your collection." : "That one just closed."); refresh(); };
  return (
    <div className="overlay-page live-page" role="dialog" aria-label={`The ${W.event}`}>
      <header className="op-head"><button className="op-back" onClick={close} aria-label="Back"><Icon name="back" /></button><b>{isConcert() ? "The show" : `${BRAND.home.short} vs ${BRAND.away.short}`}</b><span className="op-sp" /></header>
      <div className="op-body">
        <section className="block event">
          {isConcert() ? <div className="show-head"><span className="eyebrow">{(world() as any)?.event?.title}</span><b>{c.label}</b>
            {(() => { const head = String((world() as any)?.event?.title ?? "").split(":")[0].trim(); const rest = ((world() as any)?.event?.acts ?? []).filter((x: string) => x.trim() !== head); return rest.length ? <small>With {rest.join(" and ")}</small> : null; })()}</div> :
          <div className="big-score">
            <button className="bs-team home" onClick={() => open("hbk")}><span><TeamMark side="home" big />{BRAND.home.name}</span><b className="num">{c.home}</b></button>
            <div className="bs-clock num">{c.label}</div>
            <button className="bs-team away" onClick={() => open("msc")}><span><TeamMark side="away" big />{BRAND.away.name}</span><b className="num">{c.away}</b></button>
          </div>}
          {fresh && (
            <div className={"moment-hero " + latest!.team}>
              <span className="eyebrow">{venueTime(latest!.t)}</span><b>{momentText(latest!.text, latest!.team)}</b>
              {feature("cards") && latest!.drop && latest!.drop.open && !latest!.drop.collected && (
                <button className="btn hero-cta" onClick={() => collect(latest!.drop!.id)}>Collect this moment · {latest!.drop.left} of {latest!.drop.supply} left</button>)}
            </div>
          )}
          {feature("predictions") && v.live.prediction?.open && !v.live.prediction.yours && <button className="btn block primary" onClick={() => { close(); ctx.go("play"); }}>{v.live.prediction.question} · make your call</button>}
        </section>
        <section className="block">
          <h2>{isConcert() ? "Tonight so far" : "Play by play"}</h2>
          <ol className="signals">{v.live.event.slice(0, 20).map((f, i) => <LiveRow key={f.t + f.text + i} f={f} open={open} now={c.t} collect={collect} />)}</ol>
        </section>
        {!isConcert() && <section className="block">
          <h2>Around the league</h2>
          <ul className="league">{v.live.league.map(g => (
            <li key={g.id}><button className="league-row" onClick={() => open(g.id)}>
              <span className="grow"><b>{g.home_name}</b> {/^(Tip|Starts|Upcoming|Later|\d)/i.test(String(g.status)) && !g.home_score && !g.away_score ? "vs" : <>{g.home_score} · {g.away_score}</>} <b>{g.away_name}</b></span><span className="tag">{g.status}</span></button></li>))}</ul>
        </section>}
      </div>
    </div>
  );
}

// ---------- Deals: what this event offers you ----------

function Deals(ctx: Ctx) {
  const { v, setSheet, open } = ctx;
  const now = v.clock.t;
  return (
    <>
      {feature("points") && (
        <section className="block points-head">
          <span className="eyebrow">Your points</span>
          <b className="pts-big num">{v.me.balance.toLocaleString()}</b>
          <small>Every dollar of food, drinks and gear earns 10. Spend them on perks{feature("priority") ? " or to have an order go first" : ""}.</small>
        </section>
      )}
      {feature("offers") && (
        <section className="block">
          <div className="block-head"><h2>Tonight's deals</h2><span className="sub">{BRAND.venue} only</span></div>
          {v.get.offers.length > 0 ? v.get.offers.map(o => <OfferCard key={o.id} o={o} now={now} onTap={() => tapOffer(o, ctx)} onCheckout={() => o.claim && setSheet({ kind: "checkout", offer: o, claim: o.claim.id })} />)
            : <p className="empty">New deals open as the night goes: at big moments and before last call. They show up here the second they do.</p>}
        </section>
      )}
      {(feature("upgrades") || feature("tickets")) && <YourSeat {...ctx} />}
      {feature("around") && <Around {...ctx} />}
      {feature("tickets") && <Events {...ctx} />}
      {!feature("tickets") && !feature("around") && v.get.events.length > 0 && null}
      <button className="link-row" onClick={() => open("venue")}><span className="grow">About {BRAND.venue}</span><Icon name="chev" size={18} /></button>
    </>
  );
}

function YourSeat({ v, toast, refresh, setting }: Ctx) {
  const [sure, setSure] = useState<string | null>(null);
  const spot = pref<string>("spot", "");
  return (
    <section className="block">
      <div className="block-head"><h2>Your seat</h2><button className="link-inline small" onClick={() => setting("seat")}>Row and seat</button></div>
      <div className="seat-ticket"><Icon name="seat" /><div className="grow"><b>Section {sec(v.you.seat)}{spot ? `, ${spot}` : ""}</b><small>{W.matchup} · tonight</small></div></div>
      {feature("upgrades") && (v.moved ? <p className="sub">You moved here at {venueTime(v.moved.at)}. Everything you order now comes to this section.</p> :
        (v.upgrades ?? []).slice(0, 3).map(u => <div key={u.id} className="upgrade-row"><div className="grow"><b>{u.title}</b><small>{isConcert() ? u.blurb.replace("Sideline", "Side view") : u.blurb}{u.left != null ? ` · ${u.left} left` : ""}</small></div>
          <button className="btn primary sm" disabled={!u.ok} onClick={() => { if (sure !== u.id) { setSure(u.id); return; } setSure(null); if (!payReady(toast)) return; const r = kernel.act({ do: "upgrade", section: u.section }); toast(r.ok ? `Moved to section ${sec(u.section)}. Your orders follow you.` : r.error); refresh(); }}>{!u.ok ? (u.why_not ?? "Not now") : sure === u.id ? `Pay ${money(u.price)} and move` : `Move · ${money(u.price)}`}</button></div>))}
    </section>
  );
}

// ---------- Play: things to do and win tonight ----------

function Battle({ v, go }: Ctx) {
  const b = v.live.battle;
  const gap = b.ahead ? b.ahead.orders - b.orders_5m + 1 : 0;
  return (
    <section className="block battle-block">
      <div className="block-head"><h2>Section battle</h2><span className="sub">orders in the last 5 min</span></div>
      <p className="explain">Every order from your section counts. The section on top when ordering closes wins free snacks for everyone in it at the next event here.</p>
      <div className="battle">
        <div><b className="big num">{b.rank ? `Section ${sec(v.you.seat)} is #${b.rank} of ${b.of}` : `Section ${sec(v.you.seat)} isn't on the board yet`}</b>
          <small>{b.rank === 1 ? "Leading the arena." : b.ahead ? `${gap} order${gap === 1 ? "" : "s"} to pass section ${sec(b.ahead.section)}` : "One order puts you on it"}</small></div>
        <button className="btn primary" onClick={() => go("get")}>{b.rank === 1 ? "Hold the lead" : "Order to climb"}</button>
      </div>
    </section>
  );
}

function Play(ctx: Ctx) {
  const { v, open, refresh, toast } = ctx;
  const m = v.me;
  const now = v.clock.t;
  const collect = (e: string) => { const r = kernel.act({ do: "collect", edition: e }); toast(r.ok ? "It's yours. It's in your collection below." : "That one just closed."); refresh(); };
  const moments = v.live.event.filter(f => f.kind === "moment" && f.drop).slice(0, 4);
  const got = m.badges.filter(b => b.earned).length;
  return (
    <>
      {feature("predictions") && <section className="block"><div className="block-head"><h2>Call it</h2><span className="sub">50 pts for each one you get right</span></div>{v.live.prediction ? <Prediction {...ctx} /> : <p className="empty">The first call opens at tip-off: who takes the first quarter. Get it right for 50 points.</p>}</section>}
      {feature("battle") && <Battle {...ctx} />}
      {feature("cards") && (
        <section className="block">
          <div className="block-head"><h2>Moments</h2><span className="sub">free to collect while they last</span></div>
          {moments.length ? <ol className="signals">{moments.map((f, i) => <LiveRow key={f.t + f.text + i} f={f} open={open} now={now} collect={collect} />)}</ol>
            : <p className="empty">When something big happens, it becomes a card here for a few minutes. The next one could be any moment.</p>}
        </section>
      )}
      {feature("cards") && v.get.market.length > 0 && (
        <section className="block">
          <div className="block-head"><h2>From other fans</h2><span className="sub">traded for points</span></div>
          <div className="card-grid">{v.get.market.slice(0, 6).map(l => (
            <div key={l.listing} className="market-item"><MomentCard c={l.card} size="sm" onClick={() => open(l.card.id)} /><b className="num">{l.price.toLocaleString()} pts</b><button className="link-inline small" onClick={() => open(l.seller)}>{l.seller_name}</button></div>))}</div>
        </section>
      )}
      {feature("cards") && <section className="block">
        <div className="block-head"><h2>Your collection</h2><span className="sub">{m.cards.length} card{m.cards.length === 1 ? "" : "s"}</span></div>
        {m.cards.length === 0 ? <p className="empty">Your first card is waiting for the next big moment.</p> :
          <div className="card-grid">{m.cards.map(x => (
            <div key={x.card.id} className="market-item"><MomentCard c={x.card} size="sm" onClick={() => open(x.card.id)} />
              <small>{x.listing ? `Listed at ${x.listing.price.toLocaleString()} pts` : x.count > 1 ? `You hold ${x.count}` : "Tap to trade"}</small></div>))}</div>}
      </section>}
      {feature("points") && <section className="block">
        <div className="block-head"><h2>Badges</h2><span className="sub">{got} of {m.badges.length}</span></div>
        <div className="badges">{m.badges.map(b => <div key={b.id} className={"badge-tile" + (b.earned ? " on" : "")}><b>{b.name}</b><small>{b.how}</small></div>)}</div>
      </section>}
    </>
  );
}

// ---------- People: your section and the crowd ----------

function People(ctx: Ctx) {
  const { v, refresh, toast, open } = ctx;
  const [scope, setScope] = useState<"all" | "section">("section");
  const [text, setText] = useState("");
  const chat = scope === "all" ? v.live.chat : v.live.chat_section;
  const send = (e: React.FormEvent) => { e.preventDefault(); const r = kernel.act({ do: "chat", text }); if (!r.ok) { toast(r.error); return; } setText(""); refresh(); };
  const m = v.me;
  return (
    <>
      {feature("chat") && <section className="block">
        <div className="block-head"><h2>{scope === "section" ? `Section ${sec(v.you.seat)}` : "Everyone here"}</h2>
          <div className="seg" role="group" aria-label="Whose chat"><button className={scope === "section" ? "on" : ""} onClick={() => setScope("section")}>Your section</button><button className={scope === "all" ? "on" : ""} onClick={() => setScope("all")}>Everyone</button></div></div>
        <form className="say" onSubmit={send}>
          <input id="say" value={text} maxLength={140} onChange={e => setText(e.target.value)} placeholder="Say something" aria-label="Message the crowd" />
          <button className="btn primary sm" disabled={!text.trim()}>Send</button>
        </form>
        <ol className="chat">
          {chat.slice(0, 14).map((c, i) => <li key={c.t + c.text + i} className={c.you ? "you" : ""}>
            {c.you ? <b>You</b> : <button className="link-inline who" onClick={() => open(c.id)}>{c.who}{scope === "all" ? ` · ${sec(c.section)}` : ""}</button>}
            <span>{c.text}</span><time className="num">{venueTime(c.t)}</time></li>)}
          {chat.length === 0 && <li className="empty">Quiet so far. Start it.</li>}
        </ol>
      </section>}
      <section className="block">
        <h2>Your people</h2>
        {m.following.length > 0 ? <div className="follow-list">{m.following.map(f => <button key={f.id} className="chip" onClick={() => open(f.id)}><i>{(f.name ?? "?").slice(0, 1)}</i>{f.name}</button>)}</div>
          : <p className="empty">Tap anyone's name to see their profile and follow them.</p>}
        {m.messages.map(t => <button key={t.with} className="thread-row" onClick={() => open(t.with)}><b>{t.name}</b><span className="grow">{t.lines[t.lines.length - 1]?.text}</span></button>)}
      </section>
      {feature("players") && m.deals.length > 0 && (
        <section className="block">
          <div className="block-head"><h2>Your requests</h2><span className="sub">{m.deals.filter(d => d.next === "confirm").length ? "1 to confirm" : ""}</span></div>
          <div className="deal-list">{m.deals.slice(0, 6).map(d => <DealCard key={d.id} d={d} as="you" act={actWith(toast, refresh)} open={open} now={v.clock.t} />)}</div>
        </section>
      )}
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

function Me({ v, refresh, toast, open, settings, setting }: Ctx) {
  const [rec, setRec] = useState<number | null>(null);
  const now = v.clock.t;
  const r = v.runner;
  const m = v.me;
  const guest = isGuest() || !profileName();
  const name = profileName() || "Guest";
  const spot = pref<string>("spot", "");
  const fresh = m.receipts.filter(o => o.stage === "Settled" && o.delivered != null && now - o.delivered < 10 * 60_000).slice(0, 2);
  const onWay = m.orders.reduce((a, o) => a + (o.charge ?? 0), 0);
  const row = (id: string, icon: Parameters<typeof Icon>[0]["name"], label: string, sub?: string) =>
    <button key={id} className="set-row" onClick={() => setting(id)}><Icon name={icon} size={20} /><span className="grow"><b>{label}</b>{sub && <small>{sub}</small>}</span><Icon name="chev" size={18} /></button>;
  return (
    <>
      <header className="me-head">
        <span className="me-av big">{name.slice(0, 1)}</span>
        <div className="grow"><h2>{name}</h2><small>Section {sec(v.you.seat)}{spot ? `, ${spot}` : ""} · {BRAND.venue}</small>{guest && <button className="link-inline small" onClick={() => setting("account")}>Create an account to keep your receipts</button>}</div>
        {feature("points") && <LevelBadge tier={m.tier} small />}
      </header>
      {fresh.length > 0 && (
        <section className="block arrived">
          <h2>Just arrived</h2>
          {fresh.map(o => <div key={o.id} className="arrived-card"><ItemArt name={o.items[0]?.name ?? ""} size={40} /><div className="grow"><b>{o.items.map(i => i.name).join(", ")}</b><small>{o.delivery ? `Reached section ${sec(o.seat)} at ${venueTime(o.delivered!)}` : `Picked up at ${o.stand}, ${venueTime(o.delivered!)}`}, {Math.max(1, Math.round((o.delivered! - o.placed) / 60_000))} min after you ordered</small></div>
            <After o={o} toast={toast} refresh={refresh} /></div>)}
        </section>
      )}
      {m.orders.length > 0 && (
        <section className="block">
          <h2>Your order{m.orders.length > 1 ? "s" : ""}</h2>
          {m.orders.map(o => <CommitmentCard key={o.id} o={o} now={now} onCancel={() => { const x = kernel.act({ do: "cancel", order: o.id }); if (!x.ok) toast(x.error); refresh(); }} />)}
          {r && r.busy && (
            <div className="mini-map">
              <PlaceView m={{ ...v.map, runners: v.map.runners.filter(x => x.id === r.id) }} showRunners={false} heat={false} />
              <p className="sub">{runnerName(r.id)} is {r.to === v.you.seat ? "heading to you now" : /^s\d+$/.test(r.to) ? `dropping off at section ${sec(r.to)}, then you` : "picking up your order"}.</p>
            </div>
          )}
        </section>
      )}
      {((feature("tickets") && m.tickets.length > 0) || (feature("around") && (m.passes.length > 0 || m.rides.length > 0))) && (
        <section className="block">
          <h2>{[m.tickets.length && "Tickets", m.passes.length && "passes", m.rides.some(r => r.stage !== "Done") && "rides"].filter(Boolean).join(", ").replace(/^./, c => c.toUpperCase()).replace(/, ([^,]*)$/, " and $1")}</h2>
          {m.rides.filter(r => r.stage !== "Done").map(r => <RideCard key={r.id} r={r} open={open} cancel={() => { const x = kernel.act({ do: "cancel_ride", ride: r.id }); toast(x.ok ? "Ride cancelled. No charge." : x.error); refresh(); }} />)}
          {m.tickets.map(t => <div key={t.event + t.section}>
            <button className="ticket" onClick={() => open(t.event)}><div className="grow"><b>{t.title}</b><small>{t.date} · {t.time} · {t.section === "sFL" ? "Courtside" : `Section ${t.section.slice(1)}`}, row {t.seats[0][0]}, seats {t.seats.map(x => x.slice(1)).join(", ")}</small></div><span className="tq num">{t.qty}×</span></button>
            <Resell t={t} toast={toast} refresh={refresh} />
          </div>)}
          {m.passes.map(p => <div key={p.place} className={"ticket pass" + (p.used ? " used" : "")} onClick={() => open(p.operator)}><div className="grow"><b>{p.name}</b><small>{p.title}</small></div>
            <div className="code-wrap">{p.used ? <small>Used {venueTime(p.used)}</small> : <><span className="code num">{p.code}</span><small>{(p as any).kind === "parking" || /park|lot|garage/i.test((p.title ?? "") + p.name) ? "show at the lot entrance" : /exit|gate/i.test(p.title ?? "") ? "show at the gate" : "show at the counter"}</small></>}</div></div>)}
        </section>
      )}
      <section className="block wallet">
        <div className="block-head"><h2>Wallet</h2><button className="link-inline small" onClick={() => setting("payment")}>Payment</button></div>
        <div className="pair">
          <div><span className="eyebrow">Spent tonight</span><b className="big num">{money(m.spent)}</b><small>{(() => { const n = m.receipts.filter(o => o.stage === "Settled").length; return `${n} order${n === 1 ? "" : "s"}${m.tickets.length || m.passes.length ? ", plus tickets and passes" : ""}`; })()}{onWay ? ` · ${money(onWay)} on the way, charged when it arrives` : ""}</small></div>
          {feature("points") ? <div><span className="eyebrow">Points</span><b className="big num">{m.balance.toLocaleString()}</b><small>{m.tier} status</small></div>
            : <div><span className="eyebrow">Paying with</span><b className="pay-name">{pref<string>("pay", "") || "Not set"}</b><small>{pref<string>("pay", "") ? "change in Payment" : "asked when you first order"}</small></div>}
        </div>
        {(m.credit ?? 0) > 0 && <p className="credit-line"><b className="num">{money(m.credit!)}</b> Agape credit · goes toward your next order before your card</p>}
      </section>
      <section className="block">
        <div className="block-head"><h2>History</h2><span className="sub">tap a receipt for its details</span></div>
        {m.receipts.length === 0 && <p className="empty">Your receipts will be here, with every charge itemized.</p>}
        {m.receipts.map(o => (
          <div key={o.id} className="receipt">
            <button className="receipt-head" aria-expanded={rec === o.id} onClick={() => setRec(rec === o.id ? null : o.id)}><CommitmentCard o={o} now={now} compact /></button>
            {rec === o.id && o.breakdown && (
              <div className="lines">
                <LedgerLine label="Items" amount={o.breakdown.subtotal} />
                {o.breakdown.discount > 0 && <LedgerLine label="Deal" amount={-o.breakdown.discount} />}
                <LedgerLine label="Tax" amount={o.breakdown.tax} />
                {o.breakdown.fee > 0 && <LedgerLine label="Delivery" amount={o.breakdown.fee} />}
                {o.breakdown.tip > 0 && <LedgerLine label="Tip" amount={o.breakdown.tip} />}
                {o.stage === "Settled" && (o.credit_used ?? 0) > 0 && <><LedgerLine label="Total" amount={o.charge} /><LedgerLine label="Paid with Agape credit" amount={-(o.credit_used ?? 0)} /></>}
                <LedgerLine label={o.stage === "Settled" ? "Charged to your card" : "Not charged"} amount={o.stage === "Settled" ? (o.charge ?? 0) - (o.credit_used ?? 0) : 0} strong />
                {o.stage === "Settled" && o.late && o.late_credit ? <LedgerLine label="Agape credit for being late" amount={-o.late_credit} /> : null}
                {o.problem?.state.state === "remedied" && <LedgerLine label={`${o.problem.state.form === "refund" ? "Refunded to your card" : "Agape credit"} · ${o.problem.words.toLowerCase()}`} amount={-(o.problem.state.amount ?? 0)} />}
                {o.problem?.state.state === "open" && <small className="sub">Your report is with the venue. You'll hear back here.</small>}
                {o.problem?.state.state === "declined" && <small className="sub">Report reviewed: {o.problem.state.why}</small>}
              </div>
            )}
            {o.stage === "Settled" && <After o={o} toast={toast} refresh={refresh} />}
            {o.can_report && <Report id={o.id} toast={toast} refresh={refresh} />}
          </div>
        ))}
      </section>
      {feature("colors") && v.colors && v.colors.teams.length > 0 && <Colors c={v.colors} toast={toast} refresh={refresh} />}
      <section className="block settings-list">
        <h2>Settings</h2>
        {row("account", "user", "Account", guest ? "Create an account" : name)}
        {row("payment", "card", "Payment", pref<string>("pay", "") || "None yet")}
        {row("seat", "seat", "Your seat", `Section ${sec(v.you.seat)}${spot ? `, ${spot}` : ""}`)}
        {row("notifications", "bell", "Notifications")}
        {row("accessibility", "access", "Accessibility")}
        {row("privacy", "lock", "Privacy")}
        {row("help", "help", "Help")}
        {row("about", "info", "About Agape")}
      </section>
      {settings}
    </>
  );
}

/** Settings that behave the way they would in the real app. */
function SettingsPage({ which, v, toast, refresh, go, close }: Ctx & { which: string; close: () => void }) {
  const [, bump] = useState(0);
  const set = (k: string, val: unknown) => { setPref(k, val); bump(x => x + 1); };
  const tog = (k: string, label: string, sub: string, d = true) => {
    const on = pref<boolean>(k, d);
    return <label className="set-toggle"><span className="grow"><b>{label}</b><small>{sub}</small></span><input type="checkbox" role="switch" checked={on} onChange={e => { set(k, e.target.checked); if (k === "motion") document.documentElement.classList.toggle("calm", e.target.checked); }} /></label>;
  };
  const title = ({ account: "Account", payment: "Payment", seat: "Your seat", notifications: "Notifications", accessibility: "Accessibility", privacy: "Privacy", help: "Help", about: "About Agape" } as Record<string, string>)[which] ?? "Settings";
  const [name, setName] = useState(profileName());
  const [email, setEmail] = useState(() => { try { return JSON.parse(localStorage.getItem("agape.profile") || "{}").email || ""; } catch { return ""; } });
  const [spot] = useState(pref<string>("spot", ""));
  const [rowIn, setRowIn] = useState(() => (/row\s*([^,]+)/i.exec(spot)?.[1] ?? "").trim());
  const [seatIn, setSeatIn] = useState(() => (/seat\s*(\w+)/i.exec(spot)?.[1] ?? "").trim());
  const [card, setCard] = useState(false);
  const pay = pref<string>("pay", "");
  const cards = pref<string[]>("cards", []);
  const size = pref<string>("text", "Default");
  const [thread, setThread] = useState(false);
  const [lines, setLines] = useState<{ me: boolean; text: string }[]>([]);
  const [say, setSay] = useState("");
  const send = (e: React.FormEvent) => {
    e.preventDefault(); const text = say.trim(); if (!text) return;
    setLines(l => [...l, { me: true, text }]); setSay("");
    setTimeout(() => setLines(l => [...l, { me: false, text: l.filter(x => !x.me).length === 0 ? `Hi${profileName() ? ` ${profileName().split(" ")[0]}` : ""}, this is Sam at Agape. I can see your seat and your orders. ${v.orders.length ? `Your ${v.orders[0].items[0]?.name?.toLowerCase()} is ${stageWord(v.orders[0].stage)} and due by ${venueTime(v.orders[0].eta)}.` : ""} How can I help?` : "Got it. I'll take care of that now and let you know here." }]), 1600);
  };
  return (
    <div className="overlay-page" role="dialog" aria-label={title}>
      <header className="op-head"><button className="op-back" onClick={close} aria-label="Back"><Icon name="back" /></button><b>{title}</b><span className="op-sp" /></header>
      <div className="op-body set-body">
        {which === "account" && <form onSubmit={e => { e.preventDefault(); if (!name.trim() || !/.+@.+\..+/.test(email)) { toast("Add your name and email."); return; } const was = isGuest() || !profileName(); setProfile({ name: name.trim(), email: email.trim(), guest: false }); toast(was ? "Account created. Your receipts from tonight are in it." : "Saved."); close(); }}>
          {(isGuest() || !profileName()) && <p className="explain">You're using Agape as a guest. Create an account to keep tonight's receipts and use Agape at any venue.</p>}
          <label className="sf wide"><span>Your name</span><input value={name} onChange={e => setName(e.target.value)} maxLength={40} autoComplete="name" /></label>
          <label className="sf wide"><span>Email</span><input type="email" value={email} onChange={e => setEmail(e.target.value)} autoComplete="email" placeholder="you@example.com" /></label>
          <button className="btn primary block">{isGuest() || !profileName() ? "Create account" : "Save"}</button>
          {!isGuest() && !!profileName() && <button type="button" className="btn ghost block" onClick={() => toast(email ? `We sent a link to ${email} to set a new password.` : "Add your email first, and we'll send a link there.")}>Change password</button>}
        </form>}
        {which === "payment" && <>
          <p className="explain">What you order is charged when it reaches you, never before. Tips go entirely to your runner.</p>
          {!pay && !cards.length && <p className="empty">No way to pay yet. Add one now, or when you first order.</p>}
          {[...(pay === "Apple Pay" ? ["Apple Pay"] : []), ...cards].map(c => <label key={c} className="set-toggle radio"><span className="grow"><b>{c}</b>{c === pay && <small>Used for your orders</small>}</span><input type="radio" name="pay" checked={c === pay} onChange={() => set("pay", c)} /></label>)}
          {pay !== "Apple Pay" && <button className="btn apple-pay block" onClick={() => { set("pay", "Apple Pay"); toast("Apple Pay added."); }}>Add Apple Pay</button>}
          {card ? <CardForm toast={toast} done={() => { setCard(false); bump(x => x + 1); toast("Card added."); }} />
            : <button className="btn ghost block" onClick={() => setCard(true)}>Add a card</button>}
        </>}
        {which === "seat" && <form onSubmit={e => { e.preventDefault(); const r = rowIn.trim().replace(/^row\s*/i, ""), p = seatIn.trim().replace(/^seat\s*/i, "");
          if (!r || !p) { toast("Add both your row and your seat."); return; }
          kernel.act({ do: "spot", row: "Row " + r.toUpperCase(), place: "seat " + p }); set("spot", `Row ${r.toUpperCase()}, seat ${p}`); refresh(); toast("Your runner will come right to you."); close(); }}>
          <p className="explain">You're in section {sec(v.you.seat)} at {BRAND.venue}. Tell us your row and seat so your runner walks straight to you.</p>
          <div className="pair"><label className="sf"><span>Row</span><input value={rowIn} onChange={e => setRowIn(e.target.value)} placeholder="F" autoCapitalize="characters" maxLength={4} /></label>
          <label className="sf"><span>Seat</span><input value={seatIn} onChange={e => setSeatIn(e.target.value.replace(/[^\dA-Za-z]/g, ""))} placeholder="7" inputMode="numeric" maxLength={4} /></label></div>
          <button className="btn primary block">Save</button>
        </form>}
        {which === "notifications" && <>
          {tog("n-orders", "Your orders", "When it's being made, on the way and here")}
          {feature("offers") && tog("n-deals", "Deals", "When a deal opens at your event")}
          {tog("n-moments", "Big moments", "The biggest moments of the night, if you've stepped away", false)}
          {tog("n-events", "Upcoming events", "From venues and the people you follow", false)}
          {tog("n-email", "Email", "Receipts and the occasional update", true)}
        </>}
        {which === "accessibility" && <>
          <div className="set-group"><b>Text size</b><div className="seg fill" role="group" aria-label="Text size">{["Default", "Large", "Larger"].map(x => <button key={x} className={size === x ? "on" : ""} onClick={() => { set("text", x); document.documentElement.dataset.text = x.toLowerCase(); }}>{x}</button>)}</div></div>
          {tog("motion", "Reduce motion", "Fewer animations and no pulsing", false)}
          {tog("stepfree", "Step-free delivery", "Your runner avoids stairs, and meets you at the aisle if you'd like", false)}
          {tog("contrast", "Higher contrast", "Darker text and stronger outlines", false)}
          {tog("haptics", "Haptics", "A tap you can feel when your order arrives", true)}
        </>}
        {which === "privacy" && <>
          {(feature("chat") || feature("players")) && <div className="set-group"><b>Who sees your profile</b><div className="seg fill" role="group" aria-label="Who sees your profile">{["Everyone", "My section", "Only me"].map(x => <button key={x} className={pref("vis", "My section") === x ? "on" : ""} onClick={() => set("vis", x)}>{x}</button>)}</div></div>}
          {tog("loc", "Location while using Agape", "So we know when you've arrived at an event. Never in the background.", true)}
          {feature("offers") && tog("ads", "Personalized deals", "Use what you order to choose which deals you see", true)}
          {isGuest() || !profileName() ? <p className="explain">As a guest, Agape keeps only your seat and tonight's orders, to deliver them and settle payment. Nothing is tied to a name or an email, and we don't sell any of it.</p> : <>
          <p className="explain">We keep your name, email, seat and orders to run Agape. We don't sell them.</p>
          <button className="btn ghost block" onClick={() => toast("We're preparing your data. You'll get an email with a download link.")}>Download my data</button>
          <button className="btn ghost block danger" onClick={() => toast("To delete your account, confirm from the email we just sent. It takes effect within 30 days.")}>Delete my account</button></>}
        </>}
        {which === "help" && thread && <div className="support">
          <p className="explain">Agape support, here at {BRAND.venue}. Most questions are answered in a few minutes.</p>
          <ol className="chat">{lines.map((l, i) => <li key={i} className={l.me ? "you" : ""}>{l.me ? <b>You</b> : <b>Sam · Agape</b>}<span>{l.text}</span></li>)}</ol>
          <form className="say" onSubmit={send}><input value={say} onChange={e => setSay(e.target.value)} placeholder="Write a message" aria-label="Message Agape support" autoFocus /><button className="btn primary sm" disabled={!say.trim()}>Send</button></form>
        </div>}
        {which === "help" && !thread && <>
          <button className="set-row" onClick={() => { close(); go("me"); }}><Icon name="help" size={20} /><span className="grow"><b>A problem with an order</b><small>Report it from the order itself; most are settled on the spot</small></span><Icon name="chev" size={18} /></button>
          {[["When will my order arrive?", "Every time you see is when it reaches your seat, measured from how long the kitchen and runners are actually taking right now. If anything changes, the time on your order changes with it."],
            ["What if it's late?", `If an order takes more than 30 minutes, or a scheduled one is more than 10 minutes past its time, you get ${money(300)} in Agape credit automatically. You don't need to ask.`],
            ["Who gets my tip?", "Your runner keeps all of it."],
            ["Ordering drinks", "Drinks with alcohol are for people 21 and older. The first time you order one, we'll ask you to confirm, and your runner checks ID at your seat."],
            ["What if I move seats?", "Change your seat in Settings and anything not yet on its way follows you."]].map(([q, a]) =>
            <details key={q} className="faq"><summary>{q}</summary><p>{a}</p></details>)}
          <button className="btn primary block" onClick={() => setThread(true)}>Message us</button>
        </>}
        {which === "about" && <>
          <div className="about-mark"><b>agape</b><small>to your seat</small></div>
          <p className="explain">Agape brings what you want to your seat at live events, made by the venue's own kitchens and carried by runners, so you never miss the moment.</p>
          <details className="faq"><summary>Terms of service</summary><p>You order from the venue's stands through Agape. You're charged when your order reaches you or is ready to pick up. If it's late, you get Agape credit automatically, as described in Help. Drinks with alcohol are for people 21 and older, and your runner checks ID. Points have no cash value and can't be bought or sold for money.</p></details>
          <details className="faq"><summary>Privacy policy</summary><p>Agape keeps the name you show, your seat and your orders to run the service. It never sells your information. <a href="/privacy/" target="_blank" rel="noopener">Read the full privacy policy</a> · <a href="/terms/" target="_blank" rel="noopener">Terms of use</a></p></details>
          <p className="sub center">Version 1.0</p>
        </>}
      </div>
    </div>
  );
}

// ---------- Search: from anywhere ----------

function Search({ q, v, setSheet }: Ctx & { q: string }) {
  const n = q.trim().toLowerCase();
  const words = (i: Item) => [i.name, i.stand, i.category, i.adult ? "beer alcohol booze" : "", i.category === "drink" ? "drinks" : "", i.category === "merch" ? "gear merch" : "", /tee|hoodie|jersey|shirt/i.test(i.name) ? "shirt" : "", /beanie|cap\b|hat\b/i.test(i.name) ? "hat" : "", i.category === "food" ? "eat snack" : "", i.category === "food" && /\bbean\b|knots|popcorn|nachos|pretzel|fries/i.test(i.name) && !/chicken|beef|hot dog|cheeseburger|taco|brisket|wings|sausage/i.test(i.name) ? "vegetarian veggie meatless" : ""];
  const hits = v.items.filter(i => words(i).join(" ").toLowerCase().includes(n)).sort((a, b) => Number(b.available) - Number(a.available) || a.eta - b.eta);
  return (
    <section className="block">
      <div className="block-head"><h2>{hits.length ? `${hits.length} match${hits.length === 1 ? "" : "es"}` : "Nothing by that name"}</h2>{hits.length > 0 && <span className="sub">Soonest to your seat first</span>}</div>
      {!hits.length && <p className="empty">Try a kind of thing: burger, beer, vegetarian, gear.</p>}
      {hits.map(it => <ItemRow key={it.id} it={it} now={v.clock.t} onPick={() => setSheet({ kind: "item", item: it.id })} />)}
    </section>
  );
}

// ---------- sheets ----------

/** The tip offered first: Agape's policy (a share of the items, or a minimum, whichever is more), rounded to a dollar. */
export function tipFor(items: number, p?: { bps: number; min: number }) { const pol = p ?? { bps: 2000, min: 300 }; return Math.ceil(Math.max(items * pol.bps / 10000, pol.min) / 100) * 100; }

function useQuote(args: Record<string, unknown>, deps: unknown[]) {
  const [q, setQ] = useState<any>(null);
  useEffect(() => { const r = kernel.act({ do: "quote", ...args }); setQ(r.ok ? r.quote : null); }, deps);
  return q;
}

/** Everything picked so far, one tap from checkout. */
/** The check every card number passes (Luhn). */
const luhn = (n: string) => n.split("").reverse().map(Number).reduce((a, d, i) => a + (i % 2 ? (d * 2 > 9 ? d * 2 - 9 : d * 2) : d), 0) % 10 === 0;

/** Adding a card: the number, its expiry and its code. */
function CardForm({ toast, done }: { toast: (t: string) => void; done: (label: string) => void }) {
  const [err, setErr] = useState(""); void toast;
  return <form className="add-card" onSubmit={e => { e.preventDefault(); const fd = new FormData(e.currentTarget); const n = (fd.get("n") as string).replace(/\D/g, ""); if (n.length < 13 || !luhn(n)) { setErr(n.length < 13 ? "Enter the full card number." : "That card number isn't right. Check it and try again."); return; }
    { const m = String(fd.get("x")).replace(/\s/g, "").match(/^(\d{2})\/?(\d{2})$/); if (m) { const end = new Date(2000 + Number(m[2]), Number(m[1]), 1); if (end <= new Date()) { setErr("That card has expired."); return; } } } if (!/^(0[1-9]|1[0-2])\s*\/?\s*\d{2}$/.test(String(fd.get("x")).trim())) { setErr("Enter the expiry as MM/YY."); return; } if (!/^\d{3,4}$/.test(String(fd.get("c")).trim())) { setErr("Enter the 3- or 4-digit security code."); return; } const label = `${({ "4": "Visa", "5": "Mastercard", "3": "Amex", "6": "Discover" } as Record<string, string>)[n[0]] ?? "Card"} •••• ${n.slice(-4)}`; setPref("cards", [...pref<string[]>("cards", []), label]); setPref("pay", label); done(label); }}>
    <label className="sf wide"><span>Card number</span><input name="n" inputMode="numeric" autoComplete="cc-number" placeholder="1234 5678 9012 3456" /></label>
    <div className="sf-grid"><label className="sf"><span>Expires</span><input name="x" inputMode="numeric" autoComplete="cc-exp" placeholder="MM/YY" /></label><label className="sf"><span>Security code</span><input name="c" inputMode="numeric" autoComplete="cc-csc" placeholder="123" /></label></div>
    {err && <small className="ob-warn" role="alert">{err}</small>}
    <button className="btn primary block">Add card</button></form>;
}

/** The order button. The first time a fan orders, it asks how they'll pay, once, then places the order. */
function PayButton({ label, disabled, onPay, toast, adult, className = "btn primary block big" }: { label: string; disabled?: boolean; onPay: () => void; toast: (t: string) => void; adult?: boolean; className?: string }) {
  const [ask, setAsk] = useState(false);
  const [card, setCard] = useState(false);
  const [, bump] = useState(0);
  const has = !!pref<string>("pay", "");
  const of21 = !adult || pref<boolean>("age21", false);
  if (ask && !of21) return (
    <div className="pay-ask">
      <b>Drinks with alcohol are 21+</b>
      <small>Confirm you're 21 or older. Your runner checks ID at your seat before handing it over.</small>
      <button className="btn primary block big" onClick={() => { setPref("age21", true); bump(x => x + 1); if (has) { setAsk(false); onPay(); } }}>I'm 21 or older</button>
      <button className="btn ghost block" onClick={() => setAsk(false)}>Not now</button>
    </div>
  );
  if (ask && !has) return (
    <div className="pay-ask">
      <b>How do you want to pay?</b>
      <small>You're charged only when it reaches you.</small>
      {card ? <><CardForm toast={toast} done={() => { setAsk(false); onPay(); }} /><button className="link-inline small" onClick={() => setCard(false)}>Back</button></> : <>
        <button className="btn apple-pay block big" onClick={() => { setPref("pay", "Apple Pay"); setAsk(false); onPay(); }}>Apple Pay</button>
        <button className="btn ghost block" onClick={() => setCard(true)}>Add a card</button>
      </>}
    </div>
  );
  return <button className={className} disabled={disabled} onClick={() => has && of21 ? onPay() : setAsk(true)}>{label}</button>;
}

/** Agape credit the fan holds goes first; the rest reaches the card when it arrives. */
function CreditLine({ v, charge }: { v: FanView; charge: number }) {
  const c = Math.min(v.me.credit ?? 0, charge ?? 0);
  if (c <= 0) return null;
  return <><LedgerLine label="Agape credit" amount={-c} /><LedgerLine label="To your card" amount={charge - c} strong /></>;
}

/** How close an order is to free delivery, when the venue has a threshold. */
function FreeHint({ v, items }: { v: FanView; items: number }) {
  const over = v.me.delivery?.free_over ?? 0;
  if (!over || items >= over) return null;
  return <small className="free-hint">Add {money(over - items)} more and delivery is free.</small>;
}

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
  const [tipPick, setTip] = useState<number | null>(null);
  const tipD = tipFor(lines.reduce((a, l) => a + l.qty * (v.items.find(i => i.id === l.thing)?.price ?? 0), 0), v.me.tip);
  const tip = tipPick ?? tipD;
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
    toast(delivery ? `Ordered. It'll be at section ${sec(v.you.seat)} around ${venueTime(latest)}.` : "Ordered. We'll tell you when each part is ready to pick up.");
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
        {delivery && <Tips tip={tip} setTip={setTip} d={tipD} />}
        <p className="promise">{delivery ? `At section ${sec(v.you.seat)} by about ${venueTime(latest)}.${q?.stands > 1 ? ` It comes from ${q.stands} stands, so it may arrive in parts; you pay one delivery fee.` : ""}` : `Each part is ready at its stand. We'll tell you the moment it's ready to pick up.`}</p>
        {q && <div className="lines">
          <LedgerLine label="Items" amount={q.items} /><LedgerLine label="Tax" amount={q.tax} />
          {q.fee > 0 && <LedgerLine label="Delivery" amount={q.fee} note="once for the cart" />}{delivery && q.fee === 0 && (v.me.delivery?.free_over ?? 0) > 0 && <LedgerLine label="Delivery" amount={0} note={`free over ${money(v.me.delivery!.free_over)}`} />}
          {delivery && <FreeHint v={v} items={q.items} />}{q.tip > 0 && <LedgerLine label="Tip" amount={q.tip} />}
          <LedgerLine label="You pay" amount={q.charge} strong /><CreditLine v={v} charge={q.charge} />
          <small className="pilot-note">{delivery ? `Charged when it reaches you. If it takes more than 30 minutes, you get ${money(300)} in Agape credit automatically.` : "Charged when it's ready to pick up."}</small>
        </div>}
        <button className="btn ghost sm" onClick={() => { cart.clear(); setSheet(null); }}>Empty the cart</button>
        <PayButton toast={toast} adult={items.some(x => x.it.adult)} disabled={busy || !!v.halt || blocked.length > 0} onPay={place} label={v.halt ? "Service is paused" : blocked.length ? "Remove what's unavailable" : `Place order · ${money(q?.charge)}`} />
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

function Tips({ tip, setTip, d }: { tip: number; setTip: (n: number) => void; d: number }) {
  return (
    <div className="tips">
      <span className="eyebrow">Tip your runner · all of it goes to them</span>
      <div className="seg fill" role="group" aria-label="Tip">
        {[0, d, Math.ceil(d * 1.25 / 100) * 100, Math.ceil(d * 1.5 / 100) * 100].filter((t, i, a) => a.indexOf(t) === i).concat(![0, d, Math.ceil(d * 1.25 / 100) * 100, Math.ceil(d * 1.5 / 100) * 100].includes(tip) ? [tip] : []).map(t => <button key={t} className={tip === t ? "on" : ""} onClick={() => setTip(t)}>{t ? money(t).replace(".00", "") : "None"}</button>)}
      </div>
    </div>
  );
}



function ItemSheet({ v, refresh, setSheet, toast, go, id, at: preset }: Ctx & { id: string; at?: number }) {
  const item = v.items.find(i => i.id === id)!;
  const moments = v.get.later.filter(l => W.moments[l.kind] && l.t - v.clock.t > 8 * 60000).slice(0, 2);
  const [delivery0] = useState(true);
  const win = (() => { const r = kernel.act({ do: "window", thing: id, delivery: delivery0 }); return r.ok ? r as { earliest: number; closes: number } : null; })();
  const [at, setAt] = useState<number | null>(preset ?? null);
  const [qty, setQty] = useState(1);
  const [delivery, setDelivery] = useState(true);
  const [tipPick, setTip] = useState<number | null>(null);
  const tipD = tipFor((v.items.find(i => i.id === id)?.price ?? 0) * qty, v.me.tip);
  const tip = tipPick ?? tipD;
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
    toast(at ? `Done. It'll be at section ${sec(v.you.seat)} around ${venueTime(at)}.` : delivery ? `Ordered. It'll be at section ${sec(v.you.seat)} around ${venueTime(item.eta)}.` : "Ordered. We'll tell you when it's ready to pick up.");
    setSheet(null); refresh(); go("me");
  };
  if (item.why_not === "AgeUnverified") {
    const yes = () => { const r = kernel.act({ do: "attest_age" }); if (!r.ok) toast(r.error); refresh(); };
    return (
      <SheetFrame title={item.name} onClose={() => setSheet(null)}>
        <div className="build">
          <div className="build-item"><ItemArt name={item.name} size={72} /><div><b className="num price-big">{money(item.price)}</b><p className="sub">{item.stand} · {concourse(item.level)}</p></div></div>
          <div className="age-gate"><b>You need to be 21 or older for this.</b><p>Confirm it once and drinks unlock for tonight. Your runner checks your ID at your seat when the first one arrives.</p></div>
          <button className="btn primary big wide" onClick={yes}>I'm 21 or older</button>
        </div>
      </SheetFrame>
    );
  }
  return (
    <SheetFrame title={item.name} onClose={() => setSheet(null)}>
      <div className="build">
        <div className="build-item"><ItemArt name={item.name} size={72} /><div><b className="num price-big">{money(item.price)}</b><p className="sub">{item.stand} · {concourse(item.level)}{item.left != null ? ` · ${item.left} left` : ""}</p></div></div>
        <div className="seg fill" role="group" aria-label="How you get it">
          <button className={delivery ? "on" : ""} onClick={() => setDelivery(true)}>Bring it to my seat</button>
          <button className={!delivery ? "on" : ""} onClick={() => setDelivery(false)}>I'll pick it up</button>
        </div>
        <div className="tips"><span className="eyebrow">When</span>
          <div className="seg fill" role="group" aria-label="When">
            <button className={at == null ? "on" : ""} onClick={() => setAt(null)}>As soon as it can</button>
            <button className={at != null ? "on" : ""} onClick={() => win && setAt(at ?? Math.min(win.closes, (moments[0]?.t ?? win.earliest + 20 * 60000)))}>Pick a time</button>
          </div>
          {at != null && win && <WhenPicker from={win.earliest} to={win.closes} value={Math.max(at, win.earliest)} set={setAt} marks={v.get.later.map(l => ({ t: l.t, label: W.moments[l.kind] ?? l.kind }))} />}
        </div>
        <div className="qty"><span className="eyebrow">How many</span>
          <div className="stepper"><button onClick={() => setQty(Math.max(1, qty - 1))} aria-label="One fewer">−</button><b className="num">{qty}</b><button onClick={() => { if (qty >= 4) toast("Up to 4 at a time. Add more to your cart."); setQty(Math.min(4, qty + 1)); }} aria-label="One more">+</button></div></div>
        {delivery && <Tips tip={tip} setTip={setTip} d={tipD} />}
        {feature("gifting") && delivery && !item.adult && at == null && <div className="gift">
          <label className="check"><input type="checkbox" checked={giftTo != null} onChange={e => setGiftTo(e.target.checked ? (friends[0]?.id ?? "s101") : null)} /> Send it to someone else</label>
          {giftTo != null && <>
            <select value={giftTo} onChange={e => setGiftTo(e.target.value)} aria-label="Who it's for">
              {friends.length > 0 && <optgroup label="People you follow">{friends.map(f => <option key={f.id} value={f.id}>{f.name}</option>)}</optgroup>}
              <optgroup label="A section">{sectionIds().map(x => <option key={x} value={x}>Section {x.slice(1)}</option>)}</optgroup>
            </select>
            <input value={giftNote} maxLength={140} onChange={e => setGiftNote(e.target.value)} placeholder="Add a note (optional)" aria-label="Note" />
          </>}
        </div>}
        {feature("priority") && at == null && delivery && v.me.balance >= 200 && <label className="check first"><input type="checkbox" checked={first} disabled={v.me.balance < 200} onChange={e => setFirst(e.target.checked)} /> Go first · 200 pts, taken only if it arrives <small>{v.me.balance < 200 ? `you have ${v.me.balance}` : `you have ${v.me.balance.toLocaleString()}`}</small></label>}
        <p className="promise">{giftTo ? `It goes to ${/^s\d+$/.test(giftTo) ? `section ${sec(giftTo)}` : friends.find(f => f.id === giftTo)?.name ?? "them"}, with your note. Their runner brings it straight there.` : at && !delivery ? `We start it so it's ready at ${item.stand} around ${venueTime(at)}.` : at ? `We hold it now and start it so it reaches section ${sec(v.you.seat)} around ${venueTime(at)}. If it's more than 10 minutes late, you get ${money(300)} in Agape credit automatically.` : delivery ? `At section ${sec(v.you.seat)} around ${venueTime(item.eta)}. If it takes more than 30 minutes, you get ${money(300)} in Agape credit automatically.` : `Ready at ${item.stand}. We'll tell you the moment it's ready to pick up.`}</p>
        {quote && <div className="lines">
          <LedgerLine label="Items" amount={quote.subtotal} /><LedgerLine label="Tax" amount={quote.tax} />
          {quote.fee > 0 && <LedgerLine label="Delivery" amount={quote.fee} />}{delivery && quote.fee === 0 && (v.me.delivery?.free_over ?? 0) > 0 && <LedgerLine label="Delivery" amount={0} note={`free over ${money(v.me.delivery!.free_over)}`} />}{quote.tip > 0 && <LedgerLine label="Tip" amount={quote.tip} />}
          {delivery && <FreeHint v={v} items={quote.subtotal - (quote.discount ?? 0)} />}
          <LedgerLine label="You pay" amount={quote.charge} strong /><CreditLine v={v} charge={quote.charge} />
          <small className="pilot-note">{delivery ? "Charged when it reaches you." : "Charged when it's ready to pick up."}</small>
        </div>}
        <div className="sheet-actions">
          {at == null && !giftTo && !first && <button className="btn big" disabled={!!v.halt} onClick={() => { cart.add(id, qty); toast(`Added to your cart: ${qty > 1 ? `${qty}× ` : ""}${item.name}.`); setSheet(null); }}>Add to cart</button>}
          <PayButton className="btn primary big grow" toast={toast} adult={item.adult} disabled={!!v.halt} onPay={place} label={v.halt ? "Service is paused" : `${giftTo ? "Send it" : at != null ? "Schedule it" : "Place order"} · ${money(quote?.charge)}`} />
        </div>
      </div>
    </SheetFrame>
  );
}

function CheckoutSheet({ v, refresh, setSheet, toast, offer, claim }: Ctx & { offer: Offer; claim: string }) {
  const live = v.get.offers.find(o => o.id === offer.id);
  const until = live?.claim?.until ?? offer.claim?.until ?? v.clock.t + 120000;
  const [tipPick, setTip] = useState<number | null>(null);
  const tipD = tipFor(offer.price ?? 0, v.me.tip);
  const tip = tipPick ?? tipD;
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
        <p className="hold"><b className="num">{countdown(until - v.clock.t)}</b> Held for you. Nobody else can take it while you check out.</p>
        <Tips tip={tip} setTip={setTip} d={tipD} />
        {quote && <div className="lines">
          <LedgerLine label="Price" amount={quote.subtotal} />
          {quote.discount > 0 && <LedgerLine label="Deal" amount={-quote.discount} note={quote.payer ? `paid by ${BRAND.sponsors[quote.payer] ?? quote.payer}` : ""} />}
          <LedgerLine label="Tax" amount={quote.tax} /><LedgerLine label="Delivery" amount={quote.fee} />
          {quote.tip > 0 && <LedgerLine label="Tip" amount={quote.tip} />}
          <LedgerLine label="You pay" amount={quote.charge} strong /><CreditLine v={v} charge={quote.charge} />
          <small className="pilot-note">Charged when it reaches you.</small>
        </div>}
        <PayButton toast={toast} disabled={!!v.halt} onPay={place} label={v.halt ? "Service is paused" : `Place order · ${money(quote?.charge)}`} />
      </div>
    </SheetFrame>
  );
}

export { team, LevelBadge };

/** Something wrong with an order: say what, and the money comes back from whoever was at fault. */
/** What went wrong, in the fan's words, and whether a photo is needed to make it right. */
const REASONS: [string, string, boolean, string][] = [
  ["wrong_item", "Wrong item", true, "Take a photo of what came. We'll make it right with the stand, or with whoever got it wrong."],
  ["missing_item", "Something was missing", true, "Take a photo of what came, so we can see what's missing."],
  ["damaged", "Spilled or ruined on the way", true, "Take a photo of it. Getting it to you in one piece is on us."],
  ["never_arrived", "It never arrived", false, "We'll check the trip, then make it right."],
  ["quality", "Not up to standard", false, "We'll pass it to the stand. They make their food, and they'll make it right."],
];

function Report({ id, toast, refresh }: { id: number; toast: (s: string) => void; refresh: () => void }) {
  const [open, setOpen] = useState(false);
  const [note, setNote] = useState("");
  const [sure, setSure] = useState<string | null>(null);
  const [photo, setPhoto] = useState<string | null>(null);
  const need = REASONS.find(x => x[0] === sure);
  const send = (reason: string) => {
    const r = kernel.act({ do: "report", order: id, reason, note, photo: !!photo });
    if (!r.ok) { toast(r.error); return; }
    toast("Thanks for telling us. The receipt shows what happens next.");
    setOpen(false); refresh();
  };
  if (!open) return <button className="link-inline small report-link" onClick={() => setOpen(true)}>Something wrong with this order?</button>;
  return (
    <div className="report">
      <div className="report-reasons">
        {REASONS.map(([k, l]) => <button key={k} className={"btn sm " + (sure === k ? "primary" : "ghost")} aria-pressed={sure === k} onClick={() => { setSure(k); }}>{l}</button>)}
      </div>
      {need && <>
        <small className="sub">{need[3]}</small>
        {need[2] && (photo ? <div className="report-photo"><img src={photo} alt="Your photo" /><button className="link-inline small" onClick={() => setPhoto(null)}>Retake</button></div>
          : <label className="btn ghost block report-cam">Take a photo<input type="file" accept="image/*" capture="environment" hidden onChange={e => { const f = e.target.files?.[0]; if (f) setPhoto(URL.createObjectURL(f)); }} /></label>)}
        <input value={note} maxLength={280} onChange={e => setNote(e.target.value)} placeholder="Anything we should know (optional)" aria-label="Details" />
        <button className="btn primary block" disabled={need[2] && !photo} onClick={() => send(need[0])}>{need[2] && !photo ? "Add a photo to send" : "Send report"}</button>
      </>}
      <small className="sub">You can report up to 30 minutes after it arrives. What's made right comes back as Agape credit, which goes toward your next order first.</small>
    </div>
  );
}

/** After an order: rate who brought it and the kitchen, once each, and order the same again. */
function After({ o, toast, refresh }: { o: Order; toast: (s: string) => void; refresh: () => void }) {
  const ctx = React.useContext(SheetCtx);
  const rated = (who: string | null) => (o.rated ?? []).find(r => r.who === who)?.stars;
  const rate = (who: string, stars: number) => { const r = kernel.act({ do: "rate", order: o.id, who, stars }); toast(r.ok ? "Thanks. That goes straight to them." : r.error); refresh(); };
  // the same thing again opens it to order, with its time and price, rather than charging at once
  const again = () => { const id = o.items[0]?.thing; if (id && ctx) ctx({ kind: "item", item: id }); else { const r = kernel.act({ do: "reorder", order: o.id }); toast(r.ok ? "Same again, on its way." : r.error); refresh(); } };
  const Stars = ({ who, label }: { who: string; label: string }) => {
    const got = rated(who);
    return <div className="rate-row"><span className="grow">{label}</span>{[1, 2, 3, 4, 5].map(n =>
      <button key={n} className={"star" + ((got ?? 0) >= n ? " on" : "")} disabled={got != null} aria-label={`${n} star${n === 1 ? "" : "s"} for ${label}`} onClick={() => rate(who, n)}>★</button>)}</div>;
  };
  return (
    <div className="after">
      {o.runner && <Stars who={o.runner} label={`Your runner, ${runnerName(o.runner)}`} />}
      <Stars who={o.stand_id} label={`The stand, ${o.stand}`} />
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

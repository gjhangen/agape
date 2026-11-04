// One page for any node in the world (a player, a team, a fan, a card, a game), plus the card
// and level visuals. Pages stack: open a player from a feed, their team from the player, a fan
// who holds their card from the card, and back out the way you came.

import { React, useEffect, useState } from "./r";
import { actWith, Deal, DealCard, Rates, Say, Thread } from "./deals";
import { kernel } from "./kernel";
import { feature, Opt, countdown, money, venueTime, SignalRow, momentText, LedgerLine } from "./parts";
import { R, SECS, wedge } from "./geo";
import { CardArt } from "./cardart";

export type Card = { id: string; mark?: string | null; kind: string; title: string; rarity: string; team: string; party: string | null; party_name: string | null; t: number; supply: number; left: number; open: boolean; open_until: number; yours: number; collected: boolean; floor: number | null; holders: number; color: string | null };
export type OpenPage = (id: string) => void;

const pts = (n: number) => `${n.toLocaleString()} pts`;
export const KIND_WORD: Record<string, string> = { splash: "Splash", run: "Run", lead_change: "Go-ahead", season: "Season moment" };

// ---------- the card itself ----------

export function MomentCard({ c, size = "md", onClick }: { c: Card; size?: "sm" | "md" | "lg"; onClick?: () => void }) {
  const Tag = onClick ? "button" : "div";
  return (
    <Tag className={`mcard ${size} r-${c.rarity.toLowerCase()}`} onClick={onClick} style={{ ["--team" as any]: c.color ?? "var(--accent)" }}>
      <CardArt id={c.id} kind={c.kind} rarity={c.rarity} mark={c.mark ?? c.title.slice(0, 1)} />
      <span className="mc-shine" aria-hidden="true" />
      <span className="mc-top"><span className="mc-rarity">{c.rarity}</span><span className="mc-kind">{KIND_WORD[c.kind] ?? c.kind}</span></span>
      <span className="mc-mark" aria-hidden="true" />
      <span className="mc-title">{c.title}</span>
      <span className="mc-foot num">{c.kind === "season" ? "Last season" : venueTime(c.t)} · edition of {c.supply}</span>
    </Tag>
  );
}

// ---------- level ----------

export function LevelBadge({ tier, small }: { tier: string; small?: boolean }) {
  return <span className={"tier t-" + tier.toLowerCase().replace(/[^a-z]/g, "") + (small ? " small" : "")}>{tier}</span>;
}

export function LevelPanel({ level, tier, floor, next, balance }: { level: number; tier: string; floor: number; next: { name: string; at: number } | null; balance: number }) {
  const f = next ? (level - floor) / (next.at - floor) : 1;
  return (
    <div className="level-panel">
      <div className="lp-left">
        <LevelBadge tier={tier} />
        <b className="num lp-level">{level.toLocaleString()}</b>
        <small>lifetime points · your level</small>
        {next && <div className="lp-bar" role="meter" aria-valuenow={Math.round(f * 100)} aria-valuemin={0} aria-valuemax={100} aria-label={`Progress to ${next.name}`}><i style={{ width: `${f * 100}%` }} /></div>}
        {next && <small className="num">{(next.at - level).toLocaleString()} to {next.name}</small>}
      </div>
      <div className="lp-right">
        <span className="eyebrow">Balance</span>
        <b className="num lp-balance">{balance.toLocaleString()}</b>
        <small>{feature("cards") ? "Spend them on cards and offers. Points can't be bought or cashed out." : "Spend them on offers around the venue. Points can't be bought or cashed out."}</small>
      </div>
    </div>
  );
}

// ---------- page ----------

export type PageActs = { order: (item: string) => void; ride: (zone: string) => void; refreshAll: () => void };

export function Page({ id, close, back, open, toast, now, depth, acts }: { id: string; close: () => void; back: () => void; open: OpenPage; toast: (s: string) => void; now: number; depth: number; acts: PageActs }) {
  const [, bump] = useState(0);
  const p = kernel.view("page", id);
  const refresh = () => { bump(n => n + 1); acts.refreshAll(); };
  const follow = () => { kernel.act({ do: "follow", node: id }); refresh(); };
  return (
    <div className="page" role="dialog" aria-modal="true" aria-label={p.name ?? p.title ?? "Page"}>
      <div className="page-bar">
        <button className="btn ghost sm" onClick={depth > 1 ? back : close}>{depth > 1 ? "Back" : "Close"}</button>
        {p.node !== "card" && p.node !== "game" && !p.is_you && <button className={"btn sm " + (p.you_follow ? "ghost" : "primary")} onClick={follow}>{p.you_follow ? "Following" : "Follow"}</button>}
      </div>
      <div className="page-body">
        {p.node === "member" && <MemberPage p={p} open={open} now={now} toast={toast} refresh={refresh} />}
        {p.node === "group" && <GroupPage p={p} open={open} />}
        {p.node === "card" && <CardPage p={p} open={open} toast={toast} refresh={refresh} now={now} />}
        {p.node === "fan" && <FanPage p={p} open={open} toast={toast} refresh={refresh} />}
        {p.node === "game" && <GamePage p={p} open={open} />}
        {p.node === "event" && <EventPage p={p} open={open} toast={toast} refresh={refresh} />}
        {p.node === "venue" && <VenuePage p={p} open={open} />}
        {p.node === "section" && <SectionPage p={p} open={open} />}
        {p.node === "business" && <BusinessPage p={p} open={open} toast={toast} refresh={refresh} acts={acts} now={now} />}
        {p.node === "driver" && <DriverPage p={p} open={open} />}
      </div>
    </div>
  );
}

function Followers({ n }: { n: number }) { return <small>{n.toLocaleString()} follower{n === 1 ? "" : "s"} on Agape</small>; }

function MemberPage({ p, open, now, toast, refresh }: { p: any; open: OpenPage; now: number; toast: (s: string) => void; refresh: () => void }) {
  const act = actWith(toast, refresh);
  const first = p.name.split(" ")[0];
  return (
    <>
      <header className="ph" style={{ ["--team" as any]: p.group.color }}>
        <span className="ph-num num">{p.number}</span>
        <div><h2>{p.name}</h2><button className="link-inline" onClick={() => open(p.group.id)}>{p.group.name}</button> · {p.pos}<br /><Followers n={p.followers} /> {p.claimed && <span className="on-agape">✓ On Agape</span>}</div>
      </header>
      <div className="tiles three">
        <div className="tile"><span className="eyebrow">Tonight</span><b className="num">{p.tonight.playing ? p.tonight.points : "–"}</b><small>{p.tonight.playing ? "points" : "not playing"}</small></div>
        <div className="tile"><span className="eyebrow">Season</span><b className="num">{p.season.ppg}</b><small>points a game</small></div>
        <div className="tile"><span className="eyebrow">Also</span><b className="num">{p.season.rpg}<small> reb</small></b><small>{p.season.apg} assists</small></div>
      </div>
      {feature("players") && p.your_deals.length > 0 && <section className="pg-sec"><h3 className="eyebrow">Your deals with {first}</h3><div className="deal-list">{p.your_deals.map((d: Deal) => <DealCard key={d.id} d={d} as="you" act={act} now={now} />)}</div></section>}
      {feature("players") && p.rates.length > 0 && <section className="pg-sec"><h3 className="eyebrow">Ask {first} <Opt k="players" /></h3>
        {!p.claimed && <p className="note">{first} isn't on Agape yet. You can still make an offer: it waits in escrow until they claim their profile, and you can take it back any time before they answer.</p>}
        <Rates rates={p.rates} to={p.id} name={p.name} act={act} balance={p.balance} />
        <small className="sub">You have {p.balance.toLocaleString()} pts.</small>
      </section>}
      <section className="pg-sec"><h3 className="eyebrow">Message</h3>
        {!p.claimed && p.thread.length === 0 && <p className="note">Messages wait for {first} until they join.</p>}
        <Thread lines={p.thread} name={p.name} />
        <Say name={first} send={text => act({ do: "message", to: p.id, text }, "Sent.")} />
      </section>
      {p.cards.length > 0 && <section className="pg-sec"><h3 className="eyebrow">Their cards</h3><div className="card-row">{p.cards.map((c: Card) => <MomentCard key={c.id} c={c} size="sm" onClick={() => open(c.id)} />)}</div></section>}
      {p.stream.length > 0 && <section className="pg-sec"><h3 className="eyebrow">Tonight</h3><ol className="signals">{p.stream.map((f: any, i: number) => <SignalRow key={i} f={f} />)}</ol></section>}
    </>
  );
}

function GroupPage({ p, open }: { p: any; open: OpenPage }) {
  return (
    <>
      <header className="ph" style={{ ["--team" as any]: p.color }}>
        <span className="ph-num">{p.abbr}</span>
        <div><h2>{p.name}</h2>{p.record} this season<br /><Followers n={p.followers} /></div>
      </header>
      {p.tonight && <p className="note">Playing tonight. {p.short} {p.tonight.score} right now.</p>}
      <section className="pg-sec"><h3 className="eyebrow">Roster</h3>
        <ul className="roster">{p.roster.map((m: any) => <li key={m.id}><button className="roster-row" onClick={() => open(m.id)}><span className="num rn">{m.number}</span><span className="grow"><b>{m.name}</b><small>{m.pos} · {m.ppg} a game</small></span>{p.tonight && <span className="num">{m.tonight} tonight</span>}</button></li>)}</ul>
      </section>
      {p.cards.length > 0 && <section className="pg-sec"><h3 className="eyebrow">Cards</h3><div className="card-row">{p.cards.map((c: Card) => <MomentCard key={c.id} c={c} size="sm" onClick={() => open(c.id)} />)}</div></section>}
    </>
  );
}

function CardPage({ p, open, toast, refresh, now }: { p: any; open: OpenPage; toast: (s: string) => void; refresh: () => void; now: number }) {
  const [price, setPrice] = useState(String(p.floor ?? 100));
  const act = (a: any, ok: string) => { const r = kernel.act(a); toast(r.ok ? ok : r.error === "CantAfford" ? "Not enough points for that one." : r.error === "Closed" ? "That drop just closed." : r.error); refresh(); };
  return (
    <>
      <div className="card-hero"><MomentCard c={p} size="lg" /></div>
      <p className="note">{p.party_name ? <><button className="link-inline" onClick={() => open(p.party)}>{p.party_name}</button> · </> : null}<button className="link-inline" onClick={() => open(p.team)}>{p.team.toUpperCase()}</button> · {p.supply} made · {p.holders} fans hold one</p>
      {p.open && !p.collected && (
        <div className="drop">
          <div className="drain"><span className="drain-label num">{p.left} of {p.supply} left · closes in {countdown(p.open_until - now)}</span><span className="drain-bar"><i style={{ width: `${(p.left / p.supply) * 100}%` }} /></span></div>
          <button className="btn primary block" onClick={() => act({ do: "collect", edition: p.id }, "It's yours. Find it in Me.")}>Collect it, free</button>
        </div>
      )}
      {p.yours > 0 && (
        <section className="pg-sec"><h3 className="eyebrow">You hold {p.yours}</h3>
          {p.listings.some((l: any) => l.yours) ? (
            <button className="btn ghost block" onClick={() => act({ do: "delist", listing: p.listings.find((l: any) => l.yours).id }, "Taken off the market.")}>Take yours off the market</button>
          ) : (
            <form className="sell" onSubmit={e => { e.preventDefault(); act({ do: "list", edition: p.id, price: Math.max(1, Math.round(+price || 0)) }, "Listed. You'll get the points when it sells."); }}>
              <label htmlFor="sell-price">Sell for</label>
              <input id="sell-price" inputMode="numeric" value={price} onChange={e => setPrice(e.target.value.replace(/\D/g, ""))} /><span>pts</span>
              <button className="btn primary sm">List it</button>
            </form>
          )}
          <small>Agape keeps 5% of each sale, in points.</small>
        </section>
      )}
      <section className="pg-sec"><h3 className="eyebrow">For sale</h3>
        {p.listings.length === 0 && <p className="empty">Nobody's selling this one right now.</p>}
        <ul className="listings">{p.listings.filter((l: any) => !l.yours).map((l: any) => (
          <li key={l.id}><button className="link-inline" onClick={() => open(l.seller)}>{l.seller_name}</button><span className="grow" /><b className="num">{pts(l.price)}</b>
            <button className="btn primary sm" onClick={() => act({ do: "buy", listing: l.id }, "Bought. It's in your collection.")}>Buy</button></li>))}</ul>
      </section>
      <p className="fineprint">Cards are for fun. They trade only for points, points can't be bought or cashed out, and nobody can promise what a card will be worth.</p>
      {p.sales.length > 0 && <section className="pg-sec"><h3 className="eyebrow">Recent sales</h3><ul className="listings">{p.sales.map((s: any, i: number) => <li key={i}><span>{s.buyer}</span><span className="grow" /><span className="num">{pts(s.price)}</span><time className="num sub">{venueTime(s.t)}</time></li>)}</ul></section>}
    </>
  );
}

function FanPage({ p, open, toast, refresh }: { p: any; open: OpenPage; toast: (s: string) => void; refresh: () => void }) {
  const [text, setText] = useState("");
  const send = (e: React.FormEvent) => { e.preventDefault(); const r = kernel.act({ do: "message", to: p.id, text }); if (!r.ok) toast(r.error); else setText(""); refresh(); };
  const earned = p.badges.filter((b: any) => b.earned);
  return (
    <>
      <header className="ph fan">
        <span className="ph-num">{p.name.slice(0, 1)}</span>
        <div><h2>{p.name}</h2><LevelBadge tier={p.tier} small /> <span className="num">{p.level.toLocaleString()}</span> · {p.cards.length} card{p.cards.length === 1 ? "" : "s"}<br /><Followers n={p.followers} /></div>
      </header>
      {earned.length > 0 && <div className="badges small">{earned.map((b: any) => <span key={b.id} className="badge-chip on">{b.name}</span>)}</div>}
      {p.cards.length > 0 && <section className="pg-sec"><h3 className="eyebrow">Collection</h3><div className="card-row">{p.cards.map((x: any) => <MomentCard key={x.card.id} c={x.card} size="sm" onClick={() => open(x.card.id)} />)}</div></section>}
      {p.said.length > 0 && <section className="pg-sec"><h3 className="eyebrow">Said tonight</h3><ul className="said">{p.said.map((c: any, i: number) => <li key={i}>{c.text}<time className="num">{venueTime(c.t)}</time></li>)}</ul></section>}
      {!p.is_you && <section className="pg-sec"><h3 className="eyebrow">Message</h3>
        {p.thread.length > 0 && <ol className="thread">{p.thread.map((m: any, i: number) => <li key={i} className={m.mine ? "mine" : ""}>{m.text}</li>)}</ol>}
        <form className="say" onSubmit={send}><input id="dm" value={text} maxLength={500} onChange={e => setText(e.target.value)} placeholder={`Message ${p.name}`} aria-label={`Message ${p.name}`} /><button className="btn primary sm" disabled={!text.trim()}>Send</button></form>
      </section>}
    </>
  );
}

function GamePage({ p, open }: { p: any; open: OpenPage }) {
  return (
    <>
      <div className="big-score">
        <button className="bs-team home" onClick={() => open(p.home)}><span>{p.home_name}</span><b className="num">{p.home_score}</b></button>
        <div className="bs-clock num">{p.status}</div>
        <button className="bs-team away" onClick={() => open(p.away)}><span>{p.away_name}</span><b className="num">{p.away_score}</b></button>
      </div>
      <p className="note">Elsewhere in the league tonight. Tap a team to follow them.</p>
    </>
  );
}

// ---------- places, events and businesses ----------

const PRICE_BANDS = (p: number) => (p >= 9000 ? "b3" : p >= 6000 ? "b2" : "b1");

/** The bowl as a seat map: each section coloured by price, faded by how sold it is, tappable. */
function SeatMapView({ sections, pick, picked }: { sections: { id: string; price: number; left: number; of: number }[]; pick: (id: string) => void; picked: string | null }) {
  const by = new Map(sections.map(s => [s.id, s]));
  const wedges: React.ReactNode[] = [];
  for (let level = 1; level <= 2; level++) for (let ring = 0; ring < SECS; ring++) {
    const id = `s${level * 100 + ring + 1}`; const s = by.get(id); if (!s) continue;
    const [r0, r1] = level === 1 ? [R.l1i, R.l1o] : [R.l2i, R.l2o];
    const sold = s.left === 0;
    wedges.push(<path key={id} d={wedge(r0, r1, -90 + ring * 15 + 0.8, -90 + (ring + 1) * 15 - 0.8)} className={"seat-sec " + (sold ? "sold" : PRICE_BANDS(s.price)) + (picked === id ? " picked" : "")}
      style={{ ["--fill" as any]: (0.35 + 0.65 * (s.left / s.of)).toFixed(2) }} onClick={() => !sold && pick(id)} role="button" tabIndex={sold ? -1 : 0} aria-label={`Section ${id.slice(1)}, ${s.left} seats left`}><title>{`Section ${id.slice(1)} · ${money(s.price)} · ${s.left} left`}</title></path>);
  }
  const fl = by.get("sFL");
  return (
    <svg className="place seatmap" viewBox="-222 -222 444 444" role="img" aria-label="Choose a section">
      <circle r={R.c2 + 10} className="ground" /><circle r={R.c2} className="conc" /><circle r={R.l2i - 6} className="ground" /><circle r={R.c1} className="conc" /><circle r={R.l1i - 6} className="ground" />
      <rect x={-50} y={-30} width={100} height={60} rx={6} className="court" />
      {fl && <rect x={-54} y={-40} width={108} height={8} rx={3} className={"seat-sec " + (fl.left ? "b3" : "sold") + (picked === "sFL" ? " picked" : "")} onClick={() => fl.left && pick("sFL")} role="button" aria-label={`Courtside, ${fl.left} seats left`}><title>{`Courtside · ${money(fl.price)} · ${fl.left} left`}</title></rect>}
      {wedges}
    </svg>
  );
}

function EventPage({ p, open, toast, refresh }: { p: any; open: OpenPage; toast: (s: string) => void; refresh: () => void }) {
  const [sec, setSec] = useState<string | null>(null);
  const [qty, setQty] = useState(2);
  const [sure, setSure] = useState(false);
  const [resale, setResale] = useState<string | null>(null);
  useEffect(() => setSure(false), [sec, qty]);
  const s = p.sections.find((x: any) => x.id === sec);
  const q = sec ? kernel.act({ do: "ticket_quote", event: p.id, section: sec, qty }) : null;
  const buy = () => {
    const r = kernel.act({ do: "tickets", event: p.id, section: sec, qty });
    if (!r.ok) { toast(r.error === "SoldOut" ? "Those seats just went. Pick again." : r.error); refresh(); return; }
    toast(`You're going. Seats ${r.id} in ${sec === "sFL" ? "courtside" : "section " + sec!.slice(1)}. They're in Me.`); setSec(null); setSure(false); refresh();
  };
  const left = p.sections.reduce((n: number, x: any) => n + x.left, 0);
  return (
    <>
      <header className="ph event-h" style={{ ["--team" as any]: p.home?.color ?? "var(--accent)" }}>
        <div className="eh-date"><b>{p.date.split(", ")[1]}</b><small>{p.date.split(", ")[0]} · {p.time}</small></div>
        <div><h2>{p.title}</h2><button className="link-inline" onClick={() => open(p.venue.id)}>{p.venue.name}</button><br /><small>{left.toLocaleString()} seats left</small></div>
      </header>
      {p.home && p.away && (
        <div className="matchup">
          {[p.home, p.away].map((t: any) => <button key={t.id} className="mu-team" style={{ ["--team" as any]: t.color }} onClick={() => open(t.id)}><b>{t.name}</b><small>{t.record}</small></button>)}
        </div>
      )}
      {p.yours.length > 0 && <div className="ticket"><div className="grow"><b>You're going</b><small>{p.yours.map((y: any) => `${y.section === "sFL" ? "Courtside" : "Sec " + y.section.slice(1)} · ${y.seats.join(", ")}`).join(" · ")}</small></div></div>}
      {p.resale?.filter((l: any) => !l.mine).length > 0 && <section className="pg-sec"><h3 className="eyebrow">From other fans</h3>
        {p.resale.filter((l: any) => !l.mine).map((l: any) => <div key={l.id} className="upgrade-row"><div className="grow"><b>{l.section === "sFL" ? "Courtside" : `Section ${l.section.slice(1)}`}, seats {l.seats.split(",").join(", ")}</b><small>from {l.seller} · at most 120% of face value</small></div>
          {resale === l.id
            ? <span className="confirm-inline"><button className="btn ghost sm" onClick={() => setResale(null)}>Cancel</button><button className="btn primary sm" onClick={() => { const r = kernel.act({ do: "buy_resale", listing: l.id }); toast(r.ok ? "They're yours. They're in Me." : r.error === "CantAfford" ? "Not enough to cover them." : r.error); setResale(null); refresh(); }}>Confirm · {money(l.price)}</button></span>
            : <button className="btn primary sm" onClick={() => setResale(l.id)}>Buy · {money(l.price)}</button>}</div>)}
      </section>}
      <section className="pg-sec"><h3 className="eyebrow">Choose where to sit</h3>
        <SeatMapView sections={p.sections} pick={setSec} picked={sec} />
        <p className="legend"><i className="lg b3" />$90+ <i className="lg b2" />$60–90 <i className="lg b1" />Under $60 <i className="lg sold" />Sold out</p>
      </section>
      {s && q?.ok && (
        <section className="pg-sec seat-pick">
          <div className="row gap"><b className="grow">{sec === "sFL" ? "Courtside floor" : `Section ${sec!.slice(1)}`} · {s.view ?? ""}</b><button className="link-inline small" onClick={() => open(sec!)}>About this section</button></div>
          <div className="qty"><span className="eyebrow">How many</span>
            <div className="stepper"><button onClick={() => setQty(Math.max(1, qty - 1))} aria-label="One fewer">−</button><b className="num">{qty}</b><button onClick={() => setQty(Math.min(8, qty + 1))} aria-label="One more">+</button></div></div>
          {q.seats.length ? <p className="promise">Row {q.seats[0][0]}, seats {q.seats.map((x: string) => x.slice(1)).join(", ")}. The best seats side by side in this section, held the moment you buy.</p>
            : <p className="note">Not {qty} seats together here. Try fewer or another section.</p>}
          <div className="lines"><LedgerLine label={`${qty} × ${money(s.price)}`} amount={q.quote.subtotal} /><LedgerLine label="Service fee" amount={q.quote.fee} /><LedgerLine label="Tax" amount={q.quote.tax} /><LedgerLine label="You pay" amount={q.quote.charge} strong /></div>
          <small>You earn {q.quote.points.toLocaleString()} points toward your level.</small>
          {!sure ? <button className="btn primary block big" disabled={!q.seats.length} onClick={() => setSure(true)}>Review · {money(q.quote.charge)}</button> : (
            <div className="confirm-box" role="group" aria-label="Confirm your tickets">
              <b>{p.title} · {p.date} · {p.time}</b>
              <span>{sec === "sFL" ? "Courtside" : `Section ${sec!.slice(1)}`}, row {q.seats[0]?.[0]}, seats {q.seats.map((x: string) => x.slice(1)).join(", ")}</span>
              <small>{money(q.quote.charge)} to your card. Can't go after all? Resell them to other fans in Me, at up to 120% of face value. (Pilot: nothing is actually charged.)</small>
              <div className="row gap"><button className="btn" onClick={() => setSure(false)}>Change</button><button className="btn primary big grow" onClick={buy}>Buy · {money(q.quote.charge)}</button></div>
            </div>
          )}
        </section>
      )}
    </>
  );
}

function VenuePage({ p, open }: { p: any; open: OpenPage }) {
  return (
    <>
      <header className="ph fan"><span className="ph-num">HA</span><div><h2>{p.name}</h2>{p.address}<br /><small>{p.capacity.toLocaleString()} seats · {p.sections} sections · {p.followers} followers</small></div></header>
      {p.tonight && <p className="note">Tonight: {p.tonight}.</p>}
      <section className="pg-sec"><h3 className="eyebrow">Coming up</h3>
        {p.events.map((e: any) => <button key={e.id} className="event-row" onClick={() => open(e.id)}><span className="er-date"><b>{e.date.split(", ")[1]}</b><small>{e.date.split(", ")[0]}</small></span><span className="grow"><b>{e.title}</b><small>{e.time} · {e.left.toLocaleString()} seats left</small></span></button>)}
      </section>
      <section className="pg-sec"><h3 className="eyebrow">Stands</h3>
        <div className="follow-list">{p.stands.map((s: any) => <button key={s.id} className="chip" onClick={() => open(s.id)}><i>{s.name.slice(0, 1)}</i>{s.name}</button>)}</div>
      </section>
      <section className="pg-sec"><h3 className="eyebrow">Around the arena</h3>
        <div className="follow-list">{p.businesses.map((b: any) => <button key={b.id} className="chip" onClick={() => open(b.id)}><i>{b.name.slice(0, 1)}</i>{b.name}</button>)}</div>
      </section>
    </>
  );
}

function SectionPage({ p, open }: { p: any; open: OpenPage }) {
  return (
    <>
      <header className="ph fan"><span className="ph-num">{p.name.replace("Section ", "")}</span><div><h2>{p.name}</h2>{p.view}{p.level ? ` · level ${p.level}` : ""}{p.yours_tonight ? " · you're here tonight" : ""}</div></header>
      <section className="pg-sec"><h3 className="eyebrow">Seats here</h3>
        {p.events.map((e: any) => <button key={e.id} className="event-row" onClick={() => open(e.id)}><span className="grow"><b>{e.title}</b><small>{e.date} · {e.left ? `${e.left} left` : "Sold out"}</small></span><span className="num price-tag">{money(e.price)}</span></button>)}
      </section>
      {p.nearby.length > 0 && <section className="pg-sec"><h3 className="eyebrow">Closest stands</h3>
        {p.nearby.map((s: any) => <button key={s.id} className="event-row" onClick={() => open(s.id)}><span className="grow"><b>{s.name}</b><small>{Math.max(1, Math.round(s.walk_s / 60))} min walk</small></span></button>)}</section>}
    </>
  );
}

function BusinessPage({ p, open, toast, refresh, acts, now }: { p: any; open: OpenPage; toast: (s: string) => void; refresh: () => void; acts: PageActs; now: number }) {
  const h = p.here;
  const act = (a: any, ok: string) => { const r = kernel.act(a); toast(r.ok ? ok : r.error === "CantAfford" ? "Not enough points yet." : r.error); refresh(); };
  return (
    <>
      <header className="ph fan"><span className="ph-num">{p.name.slice(0, 1)}</span><div><h2>{p.name}</h2>{p.blurb}<br /><small>{p.hours}{p.walk_min ? ` · ${p.walk_min} min walk from the arena` : ""} · {p.followers} followers</small></div></header>
      {p.wait != null && <p className="note">Kitchen wait right now: about {Math.max(1, Math.round(p.wait / 60))} min.</p>}
      {p.menu.length > 0 && <section className="pg-sec"><h3 className="eyebrow">Menu</h3>
        {p.menu.map((m: any) => <button key={m.id} className="event-row" onClick={() => acts.order(m.id)}><span className="grow"><b>{m.name}</b></span><span className="num price-tag">{money(m.price)}</span></button>)}</section>}
      {h?.perk && <section className="pg-sec"><h3 className="eyebrow">For Agape fans</h3>
        <div className="perk-card"><b>{h.perk.title}</b><small>{h.perk.left} of {h.perk.supply} left · {h.perk.cost} points</small>
          {h.perk.yours ? <span className="tag good">Unlocked. Your code is in Me.</span> :
            <button className="btn primary" disabled={!h.perk.affordable || h.perk.left < 1} onClick={() => act({ do: "unlock", place: h.id }, "Unlocked. Show the code in Me at the counter.")}>{h.perk.affordable ? `Unlock for ${h.perk.cost} points` : `You need ${h.perk.cost} points`}</button>}</div></section>}
      {h?.service?.kind === "ride" && <section className="pg-sec"><h3 className="eyebrow">Ride home</h3>
        <p className="note">Pickup at {h.name}, {h.walk_min} min from your seat. {h.service.drivers - h.service.busy} of {h.service.drivers} drivers free right now.</p>
        <button className="btn primary block" onClick={() => acts.ride(h.id)}>Book a ride</button></section>}
      {h?.service?.kind === "parking" && <section className="pg-sec"><h3 className="eyebrow">{h.name}</h3>
        <p className="note">{h.blurb}</p>
        {h.service.yours ? <span className="tag good">Exit paid. Your code is in Me.</span> :
          <button className="btn primary block" onClick={() => act({ do: "parking", lot: h.id }, "Paid. Skip the kiosk and show the code in Me at the gate.")}>Pay {money(h.service.price)} now and skip the exit kiosk</button>}</section>}
      {p.drivers.length > 0 && <section className="pg-sec"><h3 className="eyebrow">Drivers tonight</h3>
        <ul className="roster">{p.drivers.map((d: any) => <li key={d.id}><button className="roster-row" onClick={() => open(d.id)}><span className="rn">★</span><span className="grow"><b>{d.name}</b><small>{d.rating.toFixed(2)} · {d.vehicle}</small></span><span className={"tag " + (d.busy ? "" : "good")}>{d.busy ? "On a ride" : "Free"}</span></button></li>)}</ul></section>}
    </>
  );
}

function DriverPage({ p, open }: { p: any; open: OpenPage }) {
  return (
    <>
      <header className="ph fan"><span className="ph-num">{p.name.split(" ").map((x: string) => x[0]).join("")}</span><div><h2>{p.name}</h2>Drives for <button className="link-inline" onClick={() => open(p.provider.id)}>{p.provider.name}</button><br /><small>★ {p.rating.toFixed(2)} · {p.trips.toLocaleString()} trips · {p.followers} followers</small></div></header>
      <div className="tiles three">
        <div className="tile"><span className="eyebrow">Car</span><b className="small-b">{p.vehicle}</b></div>
        <div className="tile"><span className="eyebrow">Plate</span><b className="num small-b">{p.plate}</b></div>
        <div className="tile"><span className="eyebrow">Tonight</span><b className="num">{p.tonight}</b><small>rides done</small></div>
      </div>
      {p.with_you && <p className="note">Your ride: {p.with_you.zone_name}, {venueTime(p.with_you.at)}, {money(p.with_you.fare)}.</p>}
    </>
  );
}

export { momentText, money };

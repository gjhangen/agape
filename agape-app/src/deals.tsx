// Deals between parties, rates on profiles, and the two new lenses: a player on Agape and a business.
import { React, useState } from "./r";
import { kernel } from "./kernel";
import { feature, MeasureTile, money, Opt, venueTime } from "./parts";

export type Deal = {
  id: string; kind: string; ask: string; pay: "points" | "money"; amount: number; status: "proposed" | "accepted" | "delivered" | "done" | "declined" | "withdrawn";
  made: number; proof: string | null; from: string; from_name: string; to: string; to_name: string; other: string; other_name: string; other_kind: string;
  incoming: boolean; next: string; waits_for_claim: boolean; auto_confirm_at: number | null; fee: number; history: { t: number; status: string }[];
};
export type Rate = { kind: string; title: string; blurb: string; pay: "points" | "money"; price: number; limit: number | null; left: number | null; affordable: boolean; yours: Deal | null };

export const amount = (pay: string, n: number) => pay === "money" ? money(n) : `${n.toLocaleString()} pts`;
const STEPS = ["proposed", "accepted", "delivered", "done"] as const;
const STEP_NAME: Record<string, string> = { proposed: "Offered", accepted: "Agreed", delivered: "Delivered", done: "Paid" };

type Act = (a: Record<string, unknown>, ok: string) => boolean;
export const actWith = (toast: (s: string) => void, refresh: () => void): Act => (a, ok) => {
  const r = kernel.act(a);
  toast(r.ok ? ok : r.error === "CantAfford" ? (a.do === "deal" ? "Not enough to cover that offer." : "Not enough points.") : r.error);
  refresh();
  return r.ok;
};

/** One deal, from the viewer's side: where it stands and the one thing they can do next. */
export function DealCard({ d, as, act, open, now }: { d: Deal; as: string; act: Act; open?: (id: string) => void; now: number }) {
  const [proof, setProof] = useState("");
  const ended = d.status === "declined" || d.status === "withdrawn";
  const at = STEPS.indexOf(d.status as any);
  const who = open ? <button className="link-inline" onClick={() => open(d.other)}>{d.other_name}</button> : <b>{d.other_name}</b>;
  return (
    <article className={"deal" + (ended ? " ended" : "") + (d.next === "respond" || d.next === "deliver" || d.next === "confirm" ? " needs" : "")}>
      <header>
        <span className="eyebrow">{d.incoming ? "From" : "To"} {who} · <span className="num">{venueTime(d.history[d.history.length - 1].t)}</span></span>
        <b className="num deal-amt">{amount(d.pay, d.amount)}</b>
      </header>
      <p className="deal-ask">{d.ask}</p>
      {!ended && <ol className="deal-track" aria-label="Where this deal stands">{STEPS.map((s, i) => <li key={s} className={i <= at ? "on" : ""}>{STEP_NAME[s]}</li>)}</ol>}
      {d.proof && <p className="deal-proof">“{d.proof}”</p>}
      {ended && <p className="sub">{d.status === "declined" ? `${d.incoming ? "You" : d.other_name.split(" ")[0]} said no.` : "Withdrawn."} {d.incoming ? "" : `${amount(d.pay, d.amount)} came back to you.`}</p>}
      {d.status === "done" && <p className="sub">{d.incoming ? `You received ${amount(d.pay, d.amount - d.fee)} after Agape's 10%.` : `Paid. ${d.other_name.split(" ")[0]} received ${amount(d.pay, d.amount - d.fee)}.`}</p>}
      {d.next === "waiting" && (
        <div className="deal-act">
          <p className="sub grow">{d.waits_for_claim ? `${d.other_name} isn't on Agape yet. Your ${amount(d.pay, d.amount)} waits in escrow until they claim their profile and answer.` : `Held in escrow while ${d.other_name.split(" ")[0]} decides. If they say no, it all comes back.`}</p>
          <button className="btn ghost sm" onClick={() => act({ do: "withdraw", as, deal: d.id }, "Withdrawn. Every point is back.")}>Take it back</button>
        </div>
      )}
      {d.next === "in progress" && <p className="sub">Agreed. Paid when they deliver{d.kind === "tunnel" || d.kind === "appearance" ? ", after the game" : ""}.</p>}
      {d.next === "respond" && (
        <div className="deal-act two">
          <button className="btn ghost" onClick={() => act({ do: "respond", as, deal: d.id, yes: false }, "Declined. The offer went back to them.")}>Decline</button>
          <button className="btn primary" onClick={() => act({ do: "respond", as, deal: d.id, yes: true }, "Agreed. The pay is already in escrow for you.")}>Accept</button>
        </div>
      )}
      {d.next === "deliver" && (
        <form className="deal-act col" onSubmit={e => { e.preventDefault(); if (act({ do: "deliver", as, deal: d.id, proof }, "Marked done. Paid when they confirm, or in 15 minutes.")) setProof(""); }}>
          <textarea value={proof} onChange={e => setProof(e.target.value)} rows={2} maxLength={500} aria-label="What you did"
            placeholder={d.kind === "shoutout" ? `Write the shoutout. It goes straight to ${d.other_name}.` : "Say what you did"} />
          <button className="btn primary wide" disabled={!proof.trim()}>{d.kind === "shoutout" ? "Send shoutout" : "Mark it done"}</button>
        </form>
      )}
      {d.next === "confirm" && (
        <div className="deal-act col">
          <button className="btn primary wide" onClick={() => act({ do: "confirm", as, deal: d.id }, "Released. They've been paid.")}>It arrived · release {amount(d.pay, d.amount)}</button>
          {d.auto_confirm_at && <p className="sub">Released on its own at {venueTime(d.auto_confirm_at)} unless you do it first.</p>}
        </div>
      )}
      {d.next === "awaiting confirmation" && d.auto_confirm_at && <p className="sub">Waiting for {d.other_name.split(" ")[0]} to confirm. Paid by {venueTime(d.auto_confirm_at)} at the latest.</p>}
    </article>
  );
}

/** What a profile offers, each one a way to ask. The pay is held from the moment you ask. */
export function Rates({ rates, to, name, as, act, balance }: { rates: Rate[]; to: string; name: string; as?: string; act: Act; balance?: number }) {
  const [open, setOpen] = useState<string | null>(null);
  const [note, setNote] = useState("");
  if (rates.length === 0) return null;
  const first = name.split(" ")[0];
  return (
    <div className="rates">
      {rates.map(r => {
        const full = r.left === 0;
        const live = r.yours && !["declined", "withdrawn"].includes(r.yours.status);
        return (
          <div key={r.kind} className={"rate" + (open === r.kind ? " open" : "")}>
            <button className="rate-head" disabled={full && !live} onClick={() => setOpen(open === r.kind ? null : r.kind)} aria-expanded={open === r.kind}>
              <span className="grow"><b>{r.title}</b><small>{r.blurb}</small>{r.left != null && <small className={r.left <= 1 ? "warn-ink" : ""}>{full ? "Fully booked tonight" : `${r.left} of ${r.limit} left tonight`}</small>}</span>
              <span className="rate-price num">{amount(r.pay, r.price)}</span>
            </button>
            {open === r.kind && (
              <form className="rate-ask" onSubmit={e => { e.preventDefault(); if (act({ do: "deal", as, to, kind: r.kind, ask: note }, `Offer sent. ${amount(r.pay, r.price)} is held until ${first} delivers.`)) { setNote(""); setOpen(null); } }}>
                <input value={note} onChange={e => setNote(e.target.value)} maxLength={200} aria-label="Add a note" placeholder={r.kind === "shoutout" ? "Who it's for, by name (optional)" : "Anything they should know (optional)"} />
                <button className="btn primary wide" disabled={!r.affordable}>{r.affordable ? `Offer ${amount(r.pay, r.price)}` : r.pay === "points" ? `${(r.price - (balance ?? 0)).toLocaleString()} more pts needed` : "Not enough budget left"}</button>
                {!r.affordable && r.pay === "points" && <small className="sub">Earn points by ordering, calling a quarter in Live, or buying tickets. Points from other fans count too.</small>}
                <small className="sub">Held in escrow from now. Paid to {first} only when it's done; all of it back if they say no or you take it back first.</small>
              </form>
            )}
          </div>
        );
      })}
    </div>
  );
}

export function Thread({ lines, name }: { lines: { t: number; mine: boolean; text: string }[]; name: string }) {
  return lines.length > 0 ? <ol className="thread" aria-label={`Messages with ${name}`}>{lines.map((m, i) => <li key={i} className={m.mine ? "mine" : ""}>{m.text}</li>)}</ol> : null;
}

export function Say({ name, send }: { name: string; send: (text: string) => boolean }) {
  const [text, setText] = useState("");
  return (
    <form className="say" onSubmit={e => { e.preventDefault(); if (send(text)) setText(""); }}>
      <input value={text} maxLength={500} onChange={e => setText(e.target.value)} placeholder={`Message ${name}`} aria-label={`Message ${name}`} />
      <button className="btn primary sm" disabled={!text.trim()}>Send</button>
    </form>
  );
}

// ---------------- a player on Agape ----------------

export function PlayerLens({ p, players, pick, toast, refresh }: { p: any; players: any[]; pick: (id: string) => void; toast: (s: string) => void; refresh: () => void }) {
  const act = actWith(toast, refresh);
  const now = p.tonight?.t ?? 0;
  const [thread, setThread] = useState<string | null>(null);
  const needs = p.deals.filter((d: Deal) => d.next === "respond" || d.next === "deliver");
  const rest = p.deals.filter((d: Deal) => !(d.next === "respond" || d.next === "deliver"));
  const escrow = p.earned.in_escrow.reduce((a: any, x: any) => (x.pay === "money" ? { ...a, m: a.m + x.amount } : { ...a, p: a.p + x.amount }), { m: 0, p: 0 });
  const sel = p.inbox.find((t: any) => t.with === thread);
  return (
    <div className="role">
      <header className="role-head">
        <span className="eyebrow">Signed in as</span>
        <select id="player-pick" value={p.id} onChange={e => pick(e.target.value)} aria-label="Player">
          {players.map(x => <option key={x.id} value={x.id}>{x.name} · {x.team}{x.claimed ? "" : " (not on Agape yet)"}</option>)}
        </select>
      </header>
      {!p.claimed ? (
        <section className="block claim">
          <h2>This is your profile, {p.name.split(" ")[0]}</h2>
          <p>{p.followers > 0 ? `${p.followers.toLocaleString()} fans already follow you here` : "Your page is already here"}{p.deals.length ? `, and ${p.deals.length} offer${p.deals.length === 1 ? " is" : "s are"} waiting in escrow for you` : ""}. Claim it to answer messages, take deals and get paid.</p>
          <p className="sub">In the pilot, {p.group.name} confirm it's you.</p>
          <button className="btn primary big wide" onClick={() => act({ do: "claim", member: p.id }, "Profile claimed. It's yours now.")}>Claim my profile</button>
        </section>
      ) : (
        <>
          <div className="tiles four">
            <MeasureTile label="Followers" value={p.followers.toLocaleString()} />
            <MeasureTile label="Earned in points" value={p.earned.points.toLocaleString()} sub="spend them on cards and perks" />
            <MeasureTile label="Earned in money" value={money(p.earned.money)} sub="from businesses" />
            <MeasureTile label="Agreed, not yet paid" value={escrow.m ? money(escrow.m) : `${escrow.p.toLocaleString()} pts`} sub={escrow.m && escrow.p ? `+ ${escrow.p.toLocaleString()} pts` : "held for you in escrow"} />
          </div>
          <div className="role-grid">
            <section className="block">
              <div className="block-head"><h2>Needs you</h2><span className="sub">{needs.length}</span></div>
              {needs.length === 0 ? <p className="empty">Nothing waiting on you. New offers show up here.</p> : needs.map((d: Deal) => <DealCard key={d.id} d={d} as={p.id} act={act} now={now} />)}
            </section>
            <section className="block">
              <div className="block-head"><h2>Messages</h2><span className="sub">{p.inbox.filter((t: any) => t.needs_reply).length} to answer</span></div>
              {sel ? (
                <div className="pg-sec">
                  <button className="btn ghost sm" onClick={() => setThread(null)}>All messages</button>
                  <b>{sel.name}</b>
                  <Thread lines={sel.lines} name={sel.name} />
                  <Say name={sel.name} send={text => act({ do: "reply", as: p.id, to: sel.with, text }, "Sent.")} />
                </div>
              ) : p.inbox.length === 0 ? <p className="empty">No messages yet.</p> :
                p.inbox.slice(0, 12).map((t: any) => <button key={t.with} className={"thread-row" + (t.needs_reply ? " unread" : "")} onClick={() => setThread(t.with)}><b>{t.name}</b><span className="grow">{t.lines[t.lines.length - 1]?.text}</span></button>)}
            </section>
            <section className="block">
              <div className="block-head"><h2>What you offer</h2></div>
              <ul className="rate-list">{p.rates.map((r: any) => <li key={r.kind}><span className="grow"><b>{r.title}</b><small>{r.pay === "money" ? "Businesses pay money" : "Fans pay points"}</small></span><span className="num">{amount(r.pay, r.price)}</span>{r.limit != null && <small className="num">{r.booked}/{r.limit}</small>}</li>)}</ul>
            </section>
            {rest.length > 0 && (
              <section className="block">
                <div className="block-head"><h2>Your deals</h2></div>
                {rest.slice(0, 12).map((d: Deal) => <DealCard key={d.id} d={d} as={p.id} act={act} now={now} />)}
              </section>
            )}
          </div>
        </>
      )}
    </div>
  );
}

// ---------------- a business ----------------

export function BusinessLens({ b, businesses, pick, toast, refresh }: { b: any; businesses: any[]; pick: (id: string) => void; toast: (s: string) => void; refresh: () => void }) {
  const act = actWith(toast, refresh);
  const [code, setCode] = useState("");
  const [last, setLast] = useState<{ ok: boolean; text: string } | null>(null);
  const [who, setWho] = useState<string | null>(null);
  const perks = b.passes.filter((x: any) => x.kind === "perk" || x.kind === "parking");
  const redeem = (e: React.FormEvent, place: string) => {
    e.preventDefault();
    const r = kernel.act({ do: "redeem", as: b.id, place, code });
    setLast(r.ok ? { ok: true, text: `Good for ${r.id}. Marked used.` } : { ok: false, text: r.error });
    if (r.ok) setCode("");
    refresh();
  };
  const needs = b.deals.filter((d: Deal) => d.next === "confirm" || d.next === "respond" || d.next === "deliver");
  const rest = b.deals.filter((d: Deal) => !needs.includes(d));
  const talent = b.talent.find((t: any) => t.id === who);
  return (
    <div className="role">
      <header className="role-head">
        <span className="eyebrow">Signed in as</span>
        <select id="biz-pick" value={b.id} onChange={e => pick(e.target.value)} aria-label="Business">
          {businesses.map(x => <option key={x.id} value={x.id}>{x.name}</option>)}
        </select>
      </header>
      <div className="tiles four">
        {b.kind === "partner"
          ? <MeasureTile label="Fans sent your way" value={perks.reduce((a: number, x: any) => a + x.out, 0)} sub={`${perks.reduce((a: number, x: any) => a + x.used, 0)} came in so far`} />
          : <MeasureTile label="Revenue tonight" value={money(b.money.revenue)} sub="after Agape's share" />}
        <MeasureTile label="Deal budget" value={money(b.money.budget)} sub="free to offer" />
        <MeasureTile label="Offered, not yet paid" value={money(b.money.in_escrow)} sub="held in escrow" />
        <MeasureTile label="Paid to players" value={money(b.money.spent_on_deals)} />
      </div>
      <div className="role-grid">
        {perks.map((x: any) => (
          <section key={x.place} className="block">
            <div className="block-head"><h2>{x.kind === "perk" ? "At the counter" : "At the gate"}</h2><span className="sub">{x.title ?? x.name}</span></div>
            <form className="redeem" onSubmit={e => redeem(e, x.place)}>
              <input inputMode="numeric" value={code} onChange={e => setCode(e.target.value.replace(/\D/g, "").slice(0, 6))} placeholder="Code on the fan's phone" aria-label="Code" className="num" />
              <button className="btn primary" disabled={code.length !== 6}>Check</button>
            </form>
            {last && <p className={"banner " + (last.ok ? "good" : "warn")} role="status">{last.text}</p>}
            <div className="pair three">
              <div><span className="eyebrow">{x.kind === "perk" ? "Unlocked" : "Sold"}</span><b className="big num">{x.out}</b><small>of {x.supply}</small></div>
              <div><span className="eyebrow">Used</span><b className="big num">{x.used}</b></div>
              <div><span className="eyebrow">Still to come</span><b className="big num">{x.waiting}</b></div>
            </div>
            {x.recent.length > 0 && <ul className="earned">{x.recent.map((r: any, i: number) => <li key={i}><span className="grow">{r.who}</span><time className="num">{venueTime(r.t)}</time></li>)}</ul>}
          </section>
        ))}
        {b.drivers.length > 0 && (
          <section className="block">
            <div className="block-head"><h2>Drivers</h2><span className="sub">{b.drivers.filter((d: any) => d.status === "Free").length} free</span></div>
            <ul className="earned">{b.drivers.map((d: any) => <li key={d.id}><span className="grow">{d.name}</span><small>{d.status === "Free" ? "Free" : d.status === "DriverAssigned" ? "Heading to pickup" : d.status === "OnBoard" ? "On a trip" : "At pickup"}</small><b className="num">{money(d.earned)}</b></li>)}</ul>
            {b.rides.length > 0 && <><h3 className="eyebrow">Booked through Agape</h3><ul className="earned">{b.rides.map((r: any) => <li key={r.id}><span className="grow">{r.who}{r.driver ? ` · ${r.driver}` : ""}</span><time className="num">{venueTime(r.at)}</time><b className="num">{money(r.fare)}</b></li>)}</ul></>}
          </section>
        )}
        <section className="block">
          <div className="block-head"><h2>Deals</h2><span className="sub">{needs.length ? `${needs.length} need you` : ""}</span></div>
          {needs.map((d: Deal) => <DealCard key={d.id} d={d} as={b.id} act={act} now={0} />)}
          {rest.slice(0, 8).map((d: Deal) => <DealCard key={d.id} d={d} as={b.id} act={act} now={0} />)}
          {b.deals.length === 0 && <p className="empty">Sponsor a player or a timeout below. The money is held until they deliver.</p>}
        </section>
        <section className="block">
          <div className="block-head"><h2>Sponsor someone <Opt k="players" /></h2><span className="sub">players on Agape and tonight's teams</span></div>
          {talent ? (
            <div className="pg-sec">
              <button className="btn ghost sm" onClick={() => setWho(null)}>Everyone</button>
              <div className="talent-head" style={{ ["--team" as any]: talent.color }}><b>{talent.name}</b><small>{talent.team} · {talent.followers.toLocaleString()} followers</small></div>
              <Rates rates={talent.rates} to={talent.id} name={talent.name} as={b.id} act={act} />
            </div>
          ) : (
            <ul className="talent">{b.talent.map((t: any) => (
              <li key={t.id}><button className="talent-row" style={{ ["--team" as any]: t.color }} onClick={() => setWho(t.id)}>
                <span className="grow"><b>{t.name}</b><small>{t.group ? "Team" : t.team} · {t.followers.toLocaleString()} followers</small></span>
                <span className="num sub">from {money(Math.min(...t.rates.map((r: Rate) => r.price)))}</span>
              </button></li>))}</ul>
          )}
        </section>
      </div>
    </div>
  );
}

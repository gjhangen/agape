// Level 5, Community: the feed (posts from fans, players and teams, with likes and comments),
// the profile you write about yourself, and the market as a place of its own.
import { React, useState } from "./r";
import { kernel } from "./kernel";
import { venueTime } from "./parts";

export type PostView = { id: string; t: number; author: string; who: string; kind: "fan" | "player" | "team" | "act" | "venue"; text: string; you: boolean;
  likes: number; liked: boolean; replies: number; comments: { t: number; id: string; who: string; text: string; you: boolean }[]; color: string | null; verified: boolean };
export type Community = {
  everyone: PostView[]; following: PostView[];
  profile: { name: string | null; bio: string; team: string; posts: number };
  teams: { id: string; name: string; short: string; color: string }[];
  resale: { id: string; event: string; title: string | null; date: string | null; price: number; section: string; seats: string; qty: number; mine: boolean; seller: string | null }[];
};

const money = (c: number) => `$${(c / 100).toFixed(c % 100 ? 2 : 0)}`;

export function Composer({ toast, refresh }: { toast: (s: string) => void; refresh: () => void }) {
  const [text, setText] = useState("");
  const send = (e: React.FormEvent) => { e.preventDefault(); const r = kernel.act({ do: "post", text }); if (!r.ok) { toast(r.error); return; } setText(""); toast("Posted"); refresh(); };
  const left = 280 - text.length;
  return <form className="cm-compose" onSubmit={send}>
    <textarea value={text} maxLength={280} rows={text.length > 60 ? 3 : 2} onChange={e => setText(e.target.value)} placeholder="Share something with everyone here" aria-label="Write a post" />
    <div><small className={"num" + (left < 20 ? " warn" : "")}>{text ? left : ""}</small><button className="btn primary sm" disabled={!text.trim()}>Post</button></div>
  </form>;
}

export function PostCard({ p, open, toast, refresh }: { p: PostView; open: (id: string) => void; toast: (s: string) => void; refresh: () => void }) {
  const [show, setShow] = useState(false);
  const [text, setText] = useState("");
  const [reporting, setReporting] = useState(false);
  const report = (reason: string) => { const r = kernel.act({ do: "report_post", post: p.id, reason }); toast(r.ok ? "Reported. Agape's team will look at it, and you won't see it again." : r.error); setReporting(false); refresh(); };
  const like = () => { const r = kernel.act({ do: "like", post: p.id }); if (!r.ok) toast(r.error); refresh(); };
  const reply = (e: React.FormEvent) => { e.preventDefault(); const r = kernel.act({ do: "comment", post: p.id, text }); if (!r.ok) { toast(r.error); return; } setText(""); refresh(); };
  const remove = () => { const r = kernel.act({ do: "unpost", post: p.id }); toast(r.ok ? "Post taken down" : r.error); refresh(); };
  const shown = show ? p.comments : p.comments.slice(-1);
  return <article className={"cm-post " + p.kind}>
    <header>
      <button className="cm-av" style={p.color ? { background: p.color } : undefined} onClick={() => !p.you && p.author && open(p.author)} aria-label={p.author ? `${p.who}'s profile` : p.who}>{p.who.slice(0, 1)}</button>
      <div className="grow">
        {p.you ? <b>You</b> : p.author ? <button className="link-inline who" onClick={() => open(p.author)}>{p.who}</button> : <b>{p.who}</b>}
        {p.verified && <i className="cm-tick">{({ team: "Team", player: "Player", act: "Artist", venue: "Venue" } as Record<string, string>)[p.kind] ?? "Official"}</i>}
        <time className="num">{venueTime(p.t)}</time>
      </div>
      {p.you ? <button className="link-inline small" onClick={remove}>Delete</button>
        : <button className="cm-more" aria-expanded={reporting} aria-label={`Report ${p.who}'s post`} onClick={() => setReporting(!reporting)}>Report</button>}
    </header>
    {reporting && <div className="cm-report" role="group" aria-label="Why are you reporting this?">
      <span>What's wrong with it?</span>
      {[["spam", "Spam or selling"], ["abuse", "Harassment or hate"], ["unsafe", "Dangerous or illegal"], ["other", "Something else"]].map(([id, label]) => <button key={id} className="btn ghost sm" onClick={() => report(id)}>{label}</button>)}
    </div>}
    <p>{p.text}</p>
    <footer>
      <button className={"cm-like" + (p.liked ? " on" : "")} aria-pressed={p.liked} onClick={like}>
        <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10z" fill={p.liked ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" /></svg>
        <span className="num">{p.likes.toLocaleString()}</span><span className="vh"> likes</span></button>
      <button className="cm-reply" aria-expanded={show} onClick={() => setShow(!show)}>
        <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 18l-1 3 4-2h9a3 3 0 0 0 3-3V8a3 3 0 0 0-3-3H7a3 3 0 0 0-3 3v7a3 3 0 0 0 1 3z" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" /></svg>
        <span className="num">{p.replies}</span><span className="vh"> comments</span></button>
    </footer>
    {shown.length > 0 && <ol className="cm-comments">{shown.map((c, i) => <li key={c.t + c.id + i}>
      {c.you ? <b>You</b> : <button className="link-inline who" onClick={() => open(c.id)}>{c.who}</button>} <span>{c.text}</span></li>)}
      {!show && p.replies > 1 && <li><button className="link-inline small" onClick={() => setShow(true)}>See all {p.replies} comments</button></li>}
    </ol>}
    {show && <form className="say cm-say" onSubmit={reply}>
      <input value={text} maxLength={200} onChange={e => setText(e.target.value)} placeholder="Add a comment" aria-label={`Comment on ${p.who}'s post`} />
      <button className="btn primary sm" disabled={!text.trim()}>Reply</button></form>}
  </article>;
}

export function Feed({ c, open, toast, refresh }: { c: Community; open: (id: string) => void; toast: (s: string) => void; refresh: () => void }) {
  const [scope, setScope] = useState<"everyone" | "following">("everyone");
  const posts = scope === "everyone" ? c.everyone : c.following;
  return <>
    <Composer toast={toast} refresh={refresh} />
    <div className="seg cm-scope" role="group" aria-label="Whose posts"><button className={scope === "everyone" ? "on" : ""} onClick={() => setScope("everyone")}>Everyone</button><button className={scope === "following" ? "on" : ""} onClick={() => setScope("following")}>Following</button></div>
    <div className="cm-feed">
      {posts.map(p => <PostCard key={p.id} p={p} open={open} toast={toast} refresh={refresh} />)}
      {posts.length === 0 && <p className="empty">{scope === "following" ? "Follow players, teams and fans from their profiles, and their posts land here." : "Nothing yet. Say the first thing."}</p>}
    </div>
  </>;
}

/** Edit what others see: name, a line about you, your team. */
export function ProfileEditor({ c, done, toast, refresh }: { c: Community; done: () => void; toast: (s: string) => void; refresh: () => void }) {
  const [name, setName] = useState(c.profile.name ?? "");
  const [bio, setBio] = useState(c.profile.bio);
  const [team, setTeam] = useState(c.profile.team);
  const save = (e: React.FormEvent) => {
    e.preventDefault();
    const r = kernel.act({ do: "profile", name, bio, team });
    if (!r.ok) { toast(r.error); return; }
    try { if (name.trim()) { const o = JSON.parse(localStorage.getItem("agape.profile") || "{}"); localStorage.setItem("agape.profile", JSON.stringify({ ...o, name: name.trim() })); } } catch { /* kept for this visit only */ }
    toast("Profile saved"); refresh(); done();
  };
  return <form className="cm-edit" onSubmit={save}>
    <label>Name others see<input value={name} maxLength={24} onChange={e => setName(e.target.value)} placeholder="Your name or a nickname" /></label>
    <label>About you<textarea value={bio} maxLength={160} rows={2} onChange={e => setBio(e.target.value)} placeholder="Season ticket holder, upper deck, loudest section" /><small className="num">{160 - bio.length}</small></label>
    <fieldset><legend>Your team</legend><div className="cm-teams">
      <button type="button" className={team === "" ? "on" : ""} onClick={() => setTeam("")}>None</button>
      {c.teams.map(t => <button type="button" key={t.id} className={team === t.id ? "on" : ""} aria-pressed={team === t.id} onClick={() => setTeam(t.id)}><i style={{ background: t.color }} />{t.short}</button>)}
    </div></fieldset>
    <p className="sub">Everyone at Agape can see your name, this line and your team. Your seat, orders and spending stay private.</p>
    <div className="cm-edit-go"><button type="button" className="btn ghost sm" onClick={done}>Cancel</button><button className="btn primary sm">Save</button></div>
  </form>;
}

/** The market: cards fans are trading, and tickets fans are reselling, in one place. */
export function Market({ c, cards, open, toast, refresh, balance, showCards, showTickets }: { c: Community; cards: any[]; open: (id: string) => void; toast: (s: string) => void; refresh: () => void; balance: number; showCards: boolean; showTickets: boolean }) {
  const buyTickets = (id: string) => { const r = kernel.act({ do: "buy_resale", listing: id }); toast(r.ok ? "They're yours. Find them under Me." : r.error); refresh(); };
  const [sure, setSure] = useState<string | null>(null);
  const tix = c.resale.filter(r => !r.mine);
  return <>
    {showTickets && <section className="block cm-mk">
      <div className="block-head"><h2>Tickets from fans</h2><span className="sub">at or under face value</span></div>
      {tix.length ? <ul className="cm-tix">{tix.map(r => <li key={r.id}>
        <button className="grow cm-tix-what" onClick={() => open(r.event)}><b>{r.title}</b><small>{r.date} · {r.section === "sFL" ? "Courtside" : `Section ${r.section?.slice(1)}`} · {r.qty} ticket{r.qty === 1 ? "" : "s"}{r.seller ? ` · from ${r.seller}` : ""}</small></button>
        {sure === r.id ? <button className="btn primary sm" onClick={() => { setSure(null); buyTickets(r.id); }}>Pay {money(r.price)}</button>
          : <button className="btn ghost sm" onClick={() => setSure(r.id)}>{money(r.price)}</button>}
      </li>)}</ul> : <p className="empty">No fan is reselling right now. When someone can't make a game, their seats show up here, never above what they paid.</p>}
    </section>}
    {showCards && <section className="block cm-mk">
      <div className="block-head"><h2>Cards from fans</h2><span className="sub">You have {balance.toLocaleString()} pts</span></div>
      {cards.length ? <ul className="cm-cards">{cards.map((l: any) => <li key={l.listing}>
        <button className="grow cm-tix-what" onClick={() => open(l.card.id)}><b>{l.card.title}</b><small>{l.card.rarity} · from {l.seller_name}</small></button>
        <button className="btn ghost sm" onClick={() => open(l.card.id)}>{l.price.toLocaleString()} pts</button>
      </li>)}</ul> : <p className="empty">No cards for sale yet.</p>}
      <p className="fineprint">Cards trade only for points. Points can't be bought or cashed out.</p>
    </section>}
  </>;
}

// agapetoyourseat.com/room/?k=…: the investor room. Each investor has their own link; what they open is counted,
// so the founders know what's been read. Nothing here is public: without a working link, there's nothing to see.
import { React, ReactDOM, useEffect, useState } from "./r";
import { Wordmark } from "./logo";
import { applyLook, applyMode, savedLook, savedMode } from "./themes";

type Item = { id: string; title: string; about: string; kind: "file" | "link" | "text"; text: string; fname: string; size: number; external: boolean };
type Room = { guest: { name: string; firm: string }; items: Item[] };
const key = new URLSearchParams(location.search).get("k") ?? "";

/** Notes written in Admin: "## " starts a heading, "- " a bullet, a blank line a new paragraph, **words** in bold. */
export function Prose({ text }: { text: string }) {
  const bold = (s: string) => s.split(/\*\*(.+?)\*\*/g).map((x, i) => i % 2 ? <b key={i}>{x}</b> : x);
  // a heading line starts a block of its own, whatever follows it
  const blocks = text.trim().split(/\n\s*\n/).flatMap(b => { const l = b.split("\n"); return l[0].startsWith("## ") && l.length > 1 ? [l[0], l.slice(1).join("\n")] : [b]; });
  return <div className="rm-prose">{blocks.map((b, i) => {
    const lines = b.split("\n");
    if (lines[0].startsWith("## ")) return <h3 key={i}>{lines[0].slice(3)}</h3>;
    if (lines.every(l => l.startsWith("- "))) return <ul key={i}>{lines.map((l, j) => <li key={j}>{bold(l.slice(2))}</li>)}</ul>;
    return <p key={i}>{lines.map((l, j) => <React.Fragment key={j}>{j > 0 && <br />}{bold(l)}</React.Fragment>)}</p>;
  })}</div>;
}

const size = (n: number) => n ? n > 1e6 ? `${(n / 1e6).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1e3))} KB` : "";
const kindOf = (i: Item) => i.kind === "file" ? (i.fname.split(".").pop() ?? "file").toUpperCase() : i.kind === "link" ? "Live" : "Read";

function RoomPage() {
  const [r, setR] = useState<Room | null>(null); const [error, setError] = useState("");
  const [open, setOpen] = useState<string | null>(null);
  useEffect(() => {
    if (!key) { setError("This page needs the link you were sent."); return; }
    fetch(`/api/room?k=${encodeURIComponent(key)}`).then(x => x.json()).then(j => j.ok ? setR(j) : setError(j.error)).catch(() => setError("Couldn't reach Agape. Check your connection and try again."));
  }, []);
  if (error) return <div className="rm"><header className="rm-top"><a className="site-mark" href="/"><Wordmark tag /></a></header><main className="rm-main"><h1>Investor room</h1><p className="rm-err">{error}</p></main></div>;
  if (!r) return <div className="rm"><header className="rm-top"><a className="site-mark" href="/"><Wordmark tag /></a></header><main className="rm-main"><p className="rm-wait">Opening the room…</p></main></div>;
  const docs = r.items.filter(i => i.kind !== "text"), notes = r.items.filter(i => i.kind === "text");
  const go = (i: Item) => i.kind === "file" ? `/api/room/file/${i.id}?k=${encodeURIComponent(key)}` : `/api/room/go/${i.id}?k=${encodeURIComponent(key)}`;
  return <div className="rm">
    <header className="rm-top"><a className="site-mark" href="/"><Wordmark tag /></a><span>Investor room · Confidential</span></header>
    <main className="rm-main">
      <section className="rm-hero">
        <span className="rm-k">Prepared for {r.guest.name}{r.guest.firm ? `, ${r.guest.firm}` : ""}</span>
        <h1>Agape, for investors</h1>
        <p>In-seat ordering for live events, and the money path around every venue. Start with the two pages, then the long paper. The demo is the product itself, running.</p>
      </section>
      {docs.length > 0 && <section className="rm-docs">{docs.map(i => <a key={i.id} className="rm-doc" href={go(i)} target="_blank" rel="noopener">
        <i className={"rm-ic " + i.kind}>{kindOf(i)}</i>
        <span><b>{i.title}</b>{i.about && <small>{i.about}</small>}</span>
        <em>{i.kind === "file" ? `Open${i.size ? " · " + size(i.size) : ""}` : "Open"} <b aria-hidden="true">→</b></em>
      </a>)}</section>}
      {notes.map(i => <section key={i.id} className={"rm-note" + (open === i.id ? " open" : "")}>
        <button className="rm-note-head" aria-expanded={open === i.id} onClick={() => { setOpen(open === i.id ? null : i.id); if (open !== i.id) fetch(`/api/room/go/${i.id}?k=${encodeURIComponent(key)}`, { redirect: "manual" }).catch(() => {}); }}>
          <span><b>{i.title}</b>{i.about && <small>{i.about}</small>}</span><em>{open === i.id ? "Close" : "Read"}</em>
        </button>
        {open === i.id && <Prose text={i.text} />}
      </section>)}
      <section className="rm-talk"><b>Questions?</b><span>Reply to the email this link came in, or write to the founders through <a href="/#talk">agapetoyourseat.com</a>.</span></section>
    </main>
    <footer className="rm-foot">This room was shared with you alone. Please don't forward the link.</footer>
  </div>;
}

if (document.getElementById("room-root")) { applyLook(savedLook()); applyMode(savedMode()); }
if (document.getElementById("room-root")) ReactDOM.createRoot(document.getElementById("room-root")!).render(<RoomPage />);

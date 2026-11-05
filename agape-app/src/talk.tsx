// "Bring Agape to your venue": the one way to reach the team from anywhere on the site.
import { React, useEffect, useState } from "./r";
import { Wordmark } from "./logo";

export function TalkToUs({ close, version, role = "venue" }: { close: () => void; version?: string | null; role?: string }) {
  const [f, setF] = useState({ name: "", contact: "", org: "", role, note: "" });
  const [state, setState] = useState<"idle" | "busy" | "sent" | string>("idle");
  useEffect(() => { document.documentElement.classList.add("talk-open"); return () => document.documentElement.classList.remove("talk-open"); }, []);
  useEffect(() => { const k = (e: KeyboardEvent) => e.key === "Escape" && close(); addEventListener("keydown", k); return () => removeEventListener("keydown", k); }, []);
  const set = (k: string) => (e: any) => setF({ ...f, [k]: e.target.value });
  const send = async (e: React.FormEvent) => {
    e.preventDefault();
    const c = f.contact.trim();
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(c) && c.replace(/\D/g, "").length < 10) { setState("Add an email address or a phone number we can reach you on."); return; }
    setState("busy");
    try {
      const r = await fetch("/api/leads", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ ...f, version: version ?? "" }) }).then(r => r.json());
      setState(r.ok ? "sent" : r.error);
    } catch { setState("We couldn't send that from here. Please try again on agapetoyourseat.com."); }
  };
  return (
    <div className="talk-page" role="dialog" aria-modal="true" aria-label="Talk to the Agape team">
      <header className="talk-top"><span className="site-mark"><Wordmark tag /></span><button type="button" className="btn ghost sm" onClick={close}>Close</button></header>
      <div className="talk-body">
        <div className="talk-pitch">
          <h1>Talk to the Agape team</h1>
          <p>Bringing Agape to a venue, partnering, or investing: leave a way to reach you and we'll get back to you within a day.</p>
          <ul><li>Start with ordering to the seat; switch on more when you want it</li><li>Your own kitchens make everything, and independent runners carry it</li><li>Agape earns a share of what it sells, so it only earns when you do</li></ul>
        </div>
        {state === "sent" ? <div className="talk-form talk-done"><h2>Thanks, {f.name.split(" ")[0]}.</h2><p>We'll be in touch within a day.</p><button type="button" className="btn primary big wide" onClick={close}>Done</button></div> :
        <form className="talk-form talk" onSubmit={send}>
          <span className="talk-q">You are</span>
          <div className="talk-roles" role="group" aria-label="You are">
            {[["venue", "A venue or team"], ["stand", "A food service operator"], ["business", "A business or sponsor"], ["runner", "I'd like to run"], ["investor", "An investor"], ["other", "Something else"]].map(([id, label]) =>
              <button type="button" key={id} className={"chip" + (f.role === id ? " on" : "")} aria-pressed={f.role === id} onClick={() => setF({ ...f, role: id })}>{label}</button>)}
          </div>
          <label>Your name<input value={f.name} onChange={set("name")} required autoComplete="name" autoFocus /></label>
          <label>Email or phone<input value={f.contact} onChange={set("contact")} required autoComplete="email" /></label>
          <label>Organization <small>optional</small><input value={f.org} onChange={set("org")} autoComplete="organization" /></label>
          <label>Anything we should know <small>optional</small><textarea value={f.note} onChange={set("note")} rows={3} /></label>
          {state !== "idle" && state !== "busy" && <p className="banner warn" role="alert">{state}</p>}
          <button className="btn primary big wide" disabled={state === "busy"}>{state === "busy" ? "Sending…" : "Send"}</button>
        </form>}
      </div>
    </div>
  );
}

// "Bring Agape to your venue": the one way to reach the team from anywhere on the site.
import { React, useEffect, useState } from "./r";

export function TalkToUs({ close, version }: { close: () => void; version?: string | null }) {
  const [f, setF] = useState({ name: "", contact: "", org: "", role: "venue", note: "" });
  const [state, setState] = useState<"idle" | "busy" | "sent" | string>("idle");
  useEffect(() => { const k = (e: KeyboardEvent) => e.key === "Escape" && close(); addEventListener("keydown", k); return () => removeEventListener("keydown", k); }, []);
  const set = (k: string) => (e: any) => setF({ ...f, [k]: e.target.value });
  const send = async (e: React.FormEvent) => {
    e.preventDefault(); setState("busy");
    try {
      const r = await fetch("/api/leads", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ ...f, version: version ?? "" }) }).then(r => r.json());
      setState(r.ok ? "sent" : r.error);
    } catch { setState("We couldn't send that from here. Please try again on agapetoyourseat.com."); }
  };
  return (
    <div className="info-back" onClick={e => e.target === e.currentTarget && close()}>
      <form className="info talk" role="dialog" aria-modal="true" aria-label="Talk to the Agape team" onSubmit={send}>
        <div className="sheet-head"><h2>Talk to the Agape team</h2><button type="button" className="btn ghost sm" onClick={close}>Close</button></div>
        {state === "sent" ? <><p>Thanks, {f.name.split(" ")[0]}. We'll be in touch within a day.</p><button type="button" className="btn primary big wide" onClick={close}>Done</button></> : <>
          <p>Bringing Agape to a venue, partnering, or investing: leave a way to reach you and we'll get back to you within a day.</p>
          <div className="talk-roles" role="group" aria-label="You are">
            {[["venue", "A venue or team"], ["business", "A business"], ["investor", "An investor"], ["other", "Something else"]].map(([id, label]) =>
              <button type="button" key={id} className={"chip" + (f.role === id ? " on" : "")} aria-pressed={f.role === id} onClick={() => setF({ ...f, role: id })}>{label}</button>)}
          </div>
          <label>Your name<input value={f.name} onChange={set("name")} required autoComplete="name" /></label>
          <label>Email or phone<input value={f.contact} onChange={set("contact")} required autoComplete="email" /></label>
          <label>Organization <small>optional</small><input value={f.org} onChange={set("org")} autoComplete="organization" /></label>
          <label>Anything we should know <small>optional</small><textarea value={f.note} onChange={set("note")} rows={3} /></label>
          {state !== "idle" && state !== "busy" && <p className="banner warn" role="alert">{state}</p>}
          <button className="btn primary big wide" disabled={state === "busy"}>{state === "busy" ? "Sending…" : "Send"}</button>
        </>}
      </form>
    </div>
  );
}

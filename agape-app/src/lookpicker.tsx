// Choosing a look: a preset, or each axis on its own, plus light and dark.
import { React, useState } from "./r";
import { AXES, Look, PRESETS, applyLook, applyMode, presetName, savedLook, savedMode } from "./themes";

export function LookPicker() {
  const [look, setLook] = useState<Look>(savedLook);
  const [mode, setMode] = useState(savedMode);
  const [mix, setMix] = useState(false);
  const set = (l: Look) => { setLook(l); applyLook(l); };
  const name = presetName(look);
  return (
    <div className="look">
      <select className="h-btn" value={name ?? ""} onChange={e => { const p = PRESETS.find(([n]) => n === e.target.value); if (p) set(p[1]); }} aria-label="Look">
        {!name && <option value="">Look: your mix</option>}
        {PRESETS.map(([n]) => <option key={n} value={n}>Look: {n}</option>)}
      </select>
      <div className="seg h-seg" role="group" aria-label="Light or dark">
        {[["auto", "Auto"], ["light", "Light"], ["dark", "Dark"]].map(([id, l]) => <button key={id} className={mode === id ? "on" : ""} onClick={() => { setMode(id); applyMode(id); }}>{l}</button>)}
      </div>
      <button className="btn h-btn" aria-expanded={mix} onClick={() => setMix(!mix)}>{mix ? "Done" : "Mix your own"}</button>
      {mix && <div className="look-mix">{AXES.map(a => (
        <label key={a.key}><span>{a.label}</span><select value={look[a.key]} onChange={e => set({ ...look, [a.key]: e.target.value })}>
          {a.options.map(([id, n]) => <option key={id} value={id}>{n}</option>)}</select></label>))}</div>}
    </div>
  );
}

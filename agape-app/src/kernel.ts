// The page runs the same Rust kernel as the server, compiled to WebAssembly. Screens call
// view() for a projection and act() for an action; they never compute prices or state.

declare const AGAPE_WASM: string;
declare const AGAPE_WORLD: any;
declare const AGAPE_TEMPLATE: any;

type Exports = {
  memory: WebAssembly.Memory;
  alloc(n: number): number; dealloc(p: number, n: number): void; out_ptr(): number;
  init(p: number, n: number): number; step(ticks: number): number;
  check(p: number, n: number): number; preview(p: number, n: number): number;
  view(p: number, n: number): number; act(p: number, n: number): number;
};

let x: Exports | null = null;
const enc = new TextEncoder(), dec = new TextDecoder();

function call(fn: (p: number, n: number) => number, obj: unknown): any {
  const k = x!;
  const b = enc.encode(JSON.stringify(obj));
  const p = k.alloc(b.length);
  new Uint8Array(k.memory.buffer, p, b.length).set(b);
  const n = fn(p, b.length);
  k.dealloc(p, b.length);
  return JSON.parse(dec.decode(new Uint8Array(k.memory.buffer, k.out_ptr(), n)));
}

/** A venue template being set up, in place of the pilot venue. Survives a reload in this tab only. */
export function worldOverride(): any { try { const s = sessionStorage.getItem("agape.world"); return s ? JSON.parse(s) : null; } catch { return null; } }
export function setWorldOverride(w: any) { try { w ? sessionStorage.setItem("agape.world", JSON.stringify(w)) : sessionStorage.removeItem("agape.world"); } catch { /* storage off: the change lasts until reload */ } }
export const setup = {
  check: (world: any) => call(x!.check, { world, template: AGAPE_TEMPLATE }),
  preview: (world: any) => call(x!.preview, { world, template: AGAPE_TEMPLATE, fans: world?.pilot?.fans ?? 3000, runners: world?.pilot?.runners ?? 40 }),
};

export async function boot(seed: number, youSeat: string) {
  const bin = Uint8Array.from(atob(AGAPE_WASM), c => c.charCodeAt(0));
  const { instance } = await WebAssembly.instantiate(bin, {});
  x = instance.exports as unknown as Exports;
  const w = worldOverride() ?? AGAPE_WORLD;
  return call(x.init, { world: w, template: AGAPE_TEMPLATE, seed, you_seat: youSeat, fans: w?.pilot?.fans ?? 3000, runners: w?.pilot?.runners ?? 40 });
}

type Lens = "fan" | "runner" | "stand" | "ops" | "page" | "player" | "business" | "parties" | "care" | "glance";

// ---------- remote: the same calls, answered by the venue's server ----------
// Screens read views synchronously while rendering. Remotely, a view is served from a cache that
// the server's clock events refresh in the background; a view never seen before is fetched at once.
// Actions wait for the server's answer, like a payment button, so the screen never guesses.
const remote = { on: false, venue: "harbor", token: "", cache: new Map<string, any>(), listeners: new Set<() => void>() };
function xhr(path: string, body: unknown): any {
  const r = new XMLHttpRequest();
  r.open("POST", path, false);
  r.setRequestHeader("content-type", "application/json");
  if (remote.token) r.setRequestHeader("authorization", `Bearer ${remote.token}`);
  r.send(JSON.stringify(body));
  try { return JSON.parse(r.responseText); } catch { return { ok: false, error: `The venue didn't answer (${r.status})` }; }
}
async function post(path: string, body: unknown): Promise<any> {
  const r = await fetch(path, { method: "POST", headers: { "content-type": "application/json", ...(remote.token ? { authorization: `Bearer ${remote.token}` } : {}) }, body: JSON.stringify(body) });
  return r.json();
}
let refreshing = false;
async function refreshRemote() {
  if (refreshing) return;
  refreshing = true;
  try {
    const keys = [...remote.cache.keys()];
    const got = await Promise.all(keys.map(k => { const [lens, ...rest] = k.split(":"); return post(`/api/v/${remote.venue}/view`, { lens, id: rest.join(":") }); }));
    keys.forEach((k, i) => remote.cache.set(k, got[i]));
    remote.listeners.forEach(f => f());
  } finally { refreshing = false; }
}
export const server = {
  get on() { return remote.on; },
  connect(venue: string, token: string) {
    remote.on = true; remote.venue = venue; remote.token = token;
    const es = new EventSource(`/api/v/${venue}/events`);
    es.addEventListener("tick", () => { refreshRemote(); });
    return es;
  },
  subscribe(f: () => void) { remote.listeners.add(f); return () => { remote.listeners.delete(f); }; },
  refresh: refreshRemote,
  post,
};

export const kernel = {
  step: (ticks: number) => remote.on ? true : x!.step(ticks) === 1,
  view: (lens: Lens, id = "", as?: string): any => {
    if (!remote.on) return call(x!.view, as ? { lens, id, as } : { lens, id });
    const k = `${lens}:${id}`;
    if (!remote.cache.has(k)) remote.cache.set(k, xhr(`/api/v/${remote.venue}/view`, { lens, id }));
    return remote.cache.get(k);
  },
  act: (a: Record<string, unknown>): any => {
    if (!remote.on) return call(x!.act, a);
    const r = xhr(`/api/v/${remote.venue}/act`, a);
    if (!["quote", "ticket_quote", "ride_quote", "window", "cart_quote"].includes(String(a.do))) {
      // the screen you acted from is current at once; everything else catches up in the background
      remote.cache.forEach((_, k) => { const [lens, ...id] = k.split(":"); if (lens !== "page") remote.cache.set(k, xhr(`/api/v/${remote.venue}/view`, { lens, id: id.join(":") })); });
      for (const k of [...remote.cache.keys()]) if (k.startsWith("page:")) remote.cache.delete(k);
      remote.listeners.forEach(f => f());
    }
    return r;
  },
};

export const world = () => worldOverride() ?? AGAPE_WORLD;

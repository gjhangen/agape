// The fan's cart: things picked to order together. Kept for this visit in this browser tab.
import { useEffect, useState } from "./r";

export type CartLine = { thing: string; qty: number };
let lines: CartLine[] = (() => { try { return JSON.parse(localStorage.getItem("agape.cart") || "[]"); } catch { return []; } })();
const subs = new Set<() => void>();
const save = () => { try { localStorage.setItem("agape.cart", JSON.stringify(lines)); } catch { /* this page only */ } subs.forEach(f => f()); };

export const cart = {
  get: () => lines,
  count: () => lines.reduce((n, l) => n + l.qty, 0),
  add(thing: string, qty: number) { const l = lines.find(x => x.thing === thing); lines = l ? lines.map(x => x === l ? { ...x, qty: x.qty + qty } : x) : [...lines, { thing, qty }]; save(); },
  set(thing: string, qty: number) { lines = qty <= 0 ? lines.filter(x => x.thing !== thing) : lines.map(x => x.thing === thing ? { ...x, qty } : x); save(); },
  clear() { lines = []; save(); },
};

export function useCart() {
  const [, bump] = useState(0);
  useEffect(() => { const f = () => bump(n => n + 1); subs.add(f); return () => { subs.delete(f); }; }, []);
  return cart.get();
}

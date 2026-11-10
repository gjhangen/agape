// Venue geometry for drawing places named by the kernel: sections (s104), concourse points
// (c1-3), stands, stairs (stairs1-6) and lifts (lift1). A round bowl, court in the middle.

export const R = { court: 64, l1i: 78, l1o: 128, c1: 138, l2i: 150, l2o: 196, c2: 208 };
/** Sections on each level: 24 in the demo arena; a real venue says its own (setSecs). */
export let SECS = 24;
export function setSecs(n: number) { if (Number.isFinite(n) && n >= 4 && n <= 60) SECS = Math.round(n); }
const rad = (d: number) => (d * Math.PI) / 180;
export const ang = (ring: number) => { const step = 360 / SECS; return -90 + ring * step + step / 2; };
export const pt = (r: number, deg: number): [number, number] => [r * Math.cos(rad(deg)), r * Math.sin(rad(deg))];

export function wedge(r0: number, r1: number, a0: number, a1: number) {
  const [x0, y0] = pt(r1, a0), [x1, y1] = pt(r1, a1), [x2, y2] = pt(r0, a1), [x3, y3] = pt(r0, a0);
  return `M${x0.toFixed(1)},${y0.toFixed(1)} A${r1},${r1} 0 0 1 ${x1.toFixed(1)},${y1.toFixed(1)} L${x2.toFixed(1)},${y2.toFixed(1)} A${r0},${r0} 0 0 0 ${x3.toFixed(1)},${y3.toFixed(1)} Z`;
}

export type Stand = { id: string; name: string; level: number; ring: number; wait: number; busy: boolean };

/** Level and ring of any place id, and the radius it sits at. */
export function locate(id: string, stands: Stand[]): { level: number; ring: number; r: number; deg: number } | null {
  let m = /^s(\d)(\d\d)$/.exec(id);
  if (m) { const level = +m[1], ring = +m[2] - 1; const r = level === 1 ? (R.l1i + R.l1o) / 2 : (R.l2i + R.l2o) / 2; return { level, ring, r, deg: ang(ring) }; }
  m = /^c(\d)-(\d+)$/.exec(id);
  if (m) { const level = +m[1], ring = +m[2]; return { level, ring, r: level === 1 ? R.c1 : R.c2, deg: ang(ring) }; }
  m = /^stairs(\d)-(\d+)$/.exec(id);
  if (m) { const ring = +m[2]; return { level: 1.5, ring, r: (R.c1 + R.c2) / 2, deg: ang(ring) }; }
  m = /^lift(\d)$/.exec(id);
  if (m) { const ring = 3; return { level: 1.5, ring, r: (R.c1 + R.c2) / 2, deg: ang(ring) }; }
  const s = stands.find(x => x.id === id);
  if (s) return { level: s.level, ring: s.ring, r: s.level === 1 ? R.c1 : R.c2, deg: ang(s.ring) };
  return null;
}

export function placePoint(id: string, stands: Stand[]): [number, number] {
  const l = locate(id, stands);
  return l ? pt(l.r, l.deg) : [0, 0];
}

/** A walking path between two places: out to the concourse, around it, across levels, in to the target. */
export function walk(a: string, b: string, stands: Stand[]): [number, number][] {
  const A = locate(a, stands), B = locate(b, stands);
  if (!A || !B) return [];
  const conc = (lv: number) => (lv >= 2 ? R.c2 : R.c1);
  const pts: [number, number][] = [pt(A.r, A.deg)];
  const rc = conc(Math.round(A.level));
  pts.push(pt(rc, A.deg));
  let d = ((B.deg - A.deg + 540) % 360) - 180;
  const n = Math.max(1, Math.ceil(Math.abs(d) / 5));
  for (let i = 1; i <= n; i++) pts.push(pt(rc, A.deg + (d * i) / n));
  if (Math.round(B.level) !== Math.round(A.level)) pts.push(pt(conc(Math.round(B.level)), B.deg));
  pts.push(pt(B.r, B.deg));
  return pts;
}

export function along(pts: [number, number][], p: number): [number, number] {
  if (pts.length === 0) return [0, 0];
  if (pts.length === 1) return pts[0];
  const seg: number[] = []; let total = 0;
  for (let i = 1; i < pts.length; i++) { const d = Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]); seg.push(d); total += d; }
  let target = Math.max(0, Math.min(1, p)) * total;
  for (let i = 0; i < seg.length; i++) {
    if (target <= seg[i] || i === seg.length - 1) {
      const f = seg[i] ? Math.min(1, target / seg[i]) : 1;
      return [pts[i][0] + (pts[i + 1][0] - pts[i][0]) * f, pts[i][1] + (pts[i + 1][1] - pts[i][1]) * f];
    }
    target -= seg[i];
  }
  return pts[pts.length - 1];
}

export const path = (pts: [number, number][]) => pts.map((p, i) => `${i ? "L" : "M"}${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" ");

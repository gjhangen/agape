// Looks: the same app with a different palette, corner shape, card style, scoreboard and type.
// Each axis is independent; the presets are combinations worth comparing. Remembered in this browser.
export type Look = { palette: string; shape: string; cards: string; board: string; type: string };

export const AXES: { key: keyof Look; label: string; options: [string, string][] }[] = [
  { key: "palette", label: "Palette", options: [["rose", "Agape Rose"], ["night", "Night Game"], ["court", "Hardwood"], ["concrete", "Concrete & Citrus"], ["gold", "Ink & Gold"],
    ["sunset", "Sunset"], ["ice", "Ice"], ["electric", "Electric"], ["royal", "Royal"], ["sand", "Terracotta"], ["mono", "Black & Red"], ["ocean", "Harbor Blue"], ["forest", "Forest"]] },
  { key: "shape", label: "Corners", options: [["soft", "Soft"], ["sharp", "Sharp"], ["round", "Round"]] },
  { key: "cards", label: "Cards", options: [["outline", "Outlined"], ["filled", "Filled"], ["raised", "Raised"], ["flat", "Flat list"]] },
  { key: "board", label: "Scoreboard", options: [["dark", "Dark band"], ["accent", "Team color"], ["light", "Light"]] },
  { key: "type", label: "Type", options: [["stadium", "Big Shoulders + Public Sans"], ["condensed", "Oswald + Inter"], ["poster", "Anton + Work Sans"], ["grotesk", "Space Grotesk + DM Sans"],
    ["editorial", "Fraunces + Public Sans"], ["rounded", "Nunito"], ["mono", "Plex Mono + Plex Sans"], ["bebas", "Bebas Neue + Barlow"]] },
];

const L = (palette: string, shape: string, cards: string, board: string, type: string): Look => ({ palette, shape, cards, board, type });
export const PRESETS: [string, Look][] = [
  ["Agape Rose", L("rose", "soft", "outline", "dark", "stadium")],
  ["Night Game", L("night", "soft", "raised", "dark", "condensed")],
  ["Hardwood", L("court", "round", "filled", "dark", "bebas")],
  ["Concrete", L("concrete", "sharp", "flat", "light", "mono")],
  ["Ink & Gold", L("gold", "soft", "raised", "dark", "editorial")],
  ["Sunset Strip", L("sunset", "round", "raised", "accent", "poster")],
  ["Ice Rink", L("ice", "round", "filled", "light", "grotesk")],
  ["Neon", L("electric", "sharp", "outline", "dark", "condensed")],
  ["Royal Box", L("royal", "soft", "raised", "accent", "editorial")],
  ["Terracotta", L("sand", "round", "filled", "dark", "rounded")],
  ["Newsprint", L("mono", "sharp", "flat", "light", "editorial")],
  ["Box Score", L("mono", "sharp", "outline", "dark", "mono")],
  ["Harbor Blue", L("ocean", "soft", "raised", "dark", "grotesk")],
  ["Forest", L("forest", "round", "filled", "accent", "bebas")],
  ["Poster", L("rose", "sharp", "outline", "accent", "poster")],
  ["Arcade", L("electric", "round", "raised", "accent", "rounded")],
  ["Clubhouse", L("court", "soft", "flat", "dark", "editorial")],
  ["Signal", L("concrete", "round", "raised", "accent", "grotesk")],
  ["Midnight Gold", L("gold", "sharp", "outline", "dark", "condensed")],
  ["Coastal", L("ocean", "round", "filled", "light", "rounded")],
  ["Brutalist", L("mono", "sharp", "outline", "accent", "poster")],
  ["Varsity", L("royal", "sharp", "filled", "dark", "bebas")],
  ["Desert", L("sand", "sharp", "flat", "light", "condensed")],
  ["Glacier", L("ice", "soft", "outline", "dark", "mono")],
];

const DEFAULT = PRESETS[0][1];
export function savedLook(): Look { try { return { ...DEFAULT, ...JSON.parse(localStorage.getItem("agape.look") || "{}") }; } catch { return DEFAULT; } }
export function applyLook(l: Look) {
  const root = document.documentElement;
  const set = (k: string, v: string, none: string) => v === none ? root.removeAttribute(`data-${k}`) : root.setAttribute(`data-${k}`, v);
  set("palette", l.palette, "rose"); set("shape", l.shape, "soft"); set("cards", l.cards, "outline"); set("board", l.board, "dark"); set("type", l.type, "stadium");
  try { localStorage.setItem("agape.look", JSON.stringify(l)); } catch { /* remembered for this visit only */ }
  paintTeam();
}
export const presetName = (l: Look) => PRESETS.find(([, p]) => (Object.keys(p) as (keyof Look)[]).every(k => p[k] === l[k]))?.[0] ?? null;

/** Light, dark, or whatever the device prefers. */
export function savedMode(): string { try { return localStorage.getItem("agape.mode") || "auto"; } catch { return "auto"; } }
export function applyMode(m: string) {
  if (m === "auto") document.documentElement.removeAttribute("data-theme"); else document.documentElement.setAttribute("data-theme", m);
  try { localStorage.setItem("agape.mode", m); } catch { /* this visit only */ }
  paintTeam();
}

// ---- Team colors: at a game the app wears the home team's color (or the team the fan chose). ----
// Only color moves; corners, cards and type stay. The color is adjusted until text on it, and it
// on the page, stay readable in light and in dark. A palette picked by hand in "Mix your own" wins.
type RGB = [number, number, number];
const hex = (h: string): RGB => { const n = parseInt(h.replace("#", ""), 16); return [(n >> 16) & 255, (n >> 8) & 255, n & 255]; };
const css = (c: RGB) => "#" + c.map(x => Math.round(Math.max(0, Math.min(255, x))).toString(16).padStart(2, "0")).join("");
const mix = (a: RGB, b: RGB, t: number): RGB => [0, 1, 2].map(i => a[i] + (b[i] - a[i]) * t) as RGB;
const lum = (c: RGB) => { const f = (v: number) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }; return 0.2126 * f(c[0]) + 0.7152 * f(c[1]) + 0.0722 * f(c[2]); };
export const contrast = (a: RGB, b: RGB) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
const WHITE: RGB = [255, 255, 255], BLACK: RGB = [0, 0, 0];
const toHsl = ([r, g, b]: RGB): RGB => { r /= 255; g /= 255; b /= 255; const mx = Math.max(r, g, b), mn = Math.min(r, g, b), l = (mx + mn) / 2, d = mx - mn;
  if (!d) return [0, 0, l]; const s = d / (1 - Math.abs(2 * l - 1)); const h = mx === r ? ((g - b) / d) % 6 : mx === g ? (b - r) / d + 2 : (r - g) / d + 4; return [(h * 60 + 360) % 360, s, l]; };
const fromHsl = ([h, s, l]: RGB): RGB => { const c = (1 - Math.abs(2 * l - 1)) * s, x = c * (1 - Math.abs((h / 60) % 2 - 1)), m = l - c / 2;
  const [r, g, b] = h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x]; return [(r + m) * 255, (g + m) * 255, (b + m) * 255]; };
/** Raise lightness (keeping the hue, and the color vivid) until c reads at `ratio` against `on`. */
const brighten = (c: RGB, on: RGB, ratio: number) => { const [h, s0] = toHsl(c); let l = toHsl(c)[2]; const s = Math.min(1, Math.max(s0, 0.7)); let x = c;
  for (let i = 0; i < 40 && contrast(x, on) < ratio; i++) { l = Math.min(0.92, l + 0.02); x = fromHsl([h, s, l]); } return x; };
/** Move c toward `to` in small steps until it reads at `ratio` against `on`. */
const until = (c: RGB, to: RGB, on: RGB, ratio: number) => { let x = c; for (let i = 0; i < 20 && contrast(x, on) < ratio; i++) x = mix(x, to, 0.08); return x; };

let wearing: string | null = null;
const TEAM_PROPS = ["--accent", "--accent-ink", "--accent-soft", "--board", "--board-2", "--board-dim"];
export function teamTokens(color: string, dark: boolean): Record<string, string> {
  const c = hex(color);
  if (dark) {
    const accent = brighten(c, hex("#171A20"), 5.5);
    return { "--accent": css(accent), "--accent-ink": css(mix(c, BLACK, 0.82)), "--accent-soft": `color-mix(in srgb, ${css(accent)} 18%, var(--surface))`,
      "--board": css(mix(c, BLACK, 0.9)), "--board-2": css(mix(c, BLACK, 0.8)), "--board-dim": css(mix(c, WHITE, 0.5)) };
  }
  const accent = until(c, BLACK, WHITE, 4.6);
  return { "--accent": css(accent), "--accent-ink": "#FFFFFF", "--accent-soft": `color-mix(in srgb, ${css(accent)} 12%, var(--surface))`,
    "--board": css(mix(c, hex("#111318"), 0.82)), "--board-2": css(mix(c, hex("#1C1F26"), 0.72)), "--board-dim": css(mix(c, WHITE, 0.55)) };
}
function paintTeam() {
  const root = document.documentElement;
  TEAM_PROPS.forEach(p => root.style.removeProperty(p));
  if (!wearing || root.hasAttribute("data-palette")) return;
  const t = root.getAttribute("data-theme");
  const dark = t === "dark" || (t !== "light" && typeof matchMedia === "function" && matchMedia("(prefers-color-scheme: dark)").matches);
  Object.entries(teamTokens(wearing, dark)).forEach(([k, v]) => root.style.setProperty(k, v));
}
/** Wear a team's color (a hex), or null for Agape's own. */
export function wearColor(color: string | null) { wearing = color; paintTeam(); }
try { matchMedia("(prefers-color-scheme: dark)").addEventListener("change", paintTeam); } catch { /* no media queries: light only */ }

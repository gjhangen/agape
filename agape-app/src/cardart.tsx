// Artwork for moment cards, drawn from the moment itself: what kind of play, whose team, how rare.
// Same card, same picture: every variation comes from a hash of the card's id.
import { React } from "./r";

const hash = (s: string) => { let h = 2166136261; for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); } return h >>> 0; };
const rnd = (seed: number) => () => { seed = (seed * 1664525 + 1013904223) >>> 0; return seed / 4294967296; };

/** A 100×140 picture behind the card's words. */
export function CardArt({ id, kind, rarity, mark }: { id: string; kind: string; rarity: string; mark: string }) {
  const r = rnd(hash(id));
  const g = `g${hash(id).toString(36)}`;
  const tilt = (r() - 0.5) * 18;
  const glow = rarity === "Legendary" ? "#FFE08A" : rarity === "Rare" ? "#9DB8FF" : rarity === "Uncommon" ? "#8EF0C0" : "#FFFFFF";
  return (
    <svg className="mc-art" viewBox="0 0 100 140" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      <defs>
        <radialGradient id={`${g}l`} cx={0.3 + r() * 0.4} cy="0.38" r="0.7"><stop offset="0" stopColor={glow} stopOpacity=".45" /><stop offset="1" stopColor={glow} stopOpacity="0" /></radialGradient>
        <linearGradient id={`${g}s`} x1="0" y1="0" x2="1" y2="1"><stop offset="0" stopColor="#fff" stopOpacity=".9" /><stop offset="1" stopColor="#fff" stopOpacity=".2" /></linearGradient>
        <pattern id={`${g}p`} width="6" height="6" patternUnits="userSpaceOnUse" patternTransform={`rotate(${35 + tilt})`}><rect width="3" height="6" fill="#fff" opacity=".05" /></pattern>
      </defs>
      <rect width="100" height="140" fill={`url(#${g}p)`} />
      <rect width="100" height="140" fill={`url(#${g}l)`} />
      {/* the court, faint */}
      <g fill="none" stroke="#fff" strokeOpacity=".12" strokeWidth=".8">
        <circle cx="50" cy="140" r="26" /><rect x="34" y="104" width="32" height="36" /><path d="M8 140 V118 A42 42 0 0 1 92 118 V140" />
      </g>
      {/* the jersey number or team mark, huge and quiet */}
      <text x="50" y="86" textAnchor="middle" style={{ fontFamily: "var(--display)", fontWeight: 800 }} fontSize={mark.length > 2 ? 46 : 74} fill="#fff" fillOpacity=".09" transform={`rotate(${tilt / 2} 50 70)`}>{mark}</text>
      <g transform={`rotate(${tilt / 3} 50 58)`}>{play(kind, r, g)}</g>
      {rarity === "Legendary" && <g fill="#FFE08A">{[...Array(9)].map((_, i) => <circle key={i} cx={r() * 100} cy={r() * 90 + 8} r={r() * 0.9 + 0.3} opacity={0.4 + r() * 0.6} />)}</g>}
    </svg>
  );
}

function play(kind: string, r: () => number, g: string) {
  if (kind === "splash") {
    // a long arc that drops through the rim
    const sx = 10 + r() * 14;
    return <>
      <path d={`M${sx} 78 Q 46 ${8 + r() * 10} 74 46`} fill="none" stroke={`url(#${g}s)`} strokeWidth="1.6" strokeDasharray="2.5 2.5" strokeLinecap="round" />
      <circle cx={sx} cy="78" r="4.2" fill="#F08A3E" stroke="#fff" strokeOpacity=".6" strokeWidth=".6" />
      <ellipse cx="78" cy="50" rx="10" ry="2.6" fill="none" stroke="#fff" strokeWidth="1.4" />
      <path d="M69 51 L72 64 L84 64 L87 51" fill="none" stroke="#fff" strokeOpacity=".7" strokeWidth=".7" />
      <path d="M72 55 L84 55 M71 59 L85 59" stroke="#fff" strokeOpacity=".45" strokeWidth=".5" />
      {[0, 1, 2].map(i => <path key={i} d={`M${64 - i * 4} ${42 - i * 3} l-4 -3`} stroke="#fff" strokeOpacity=".7" strokeWidth="1" strokeLinecap="round" />)}
    </>;
  }
  if (kind === "run") {
    // the run as rising bars, each a basket
    const n = 6;
    return <>{[...Array(n)].map((_, i) => { const h = 8 + i * 7 + r() * 4; return <rect key={i} x={14 + i * 12.5} y={86 - h} width="8" height={h} rx="1.5" fill="#fff" fillOpacity={0.25 + i * 0.12} />; })}
      <path d={`M12 78 ${[...Array(n)].map((_, i) => `L${18 + i * 12.5} ${80 - (8 + i * 7)}`).join(" ")}`} fill="none" stroke="#fff" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M86 34 l4 -2 l-1 4.5" fill="none" stroke="#fff" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" /></>;
  }
  if (kind === "lead_change") {
    // two lines cross; ours ends on top
    const pts = (up: boolean) => [...Array(7)].map((_, i) => `${10 + i * 13.3} ${up ? 74 - i * 6 - r() * 6 : 38 + i * 5 + r() * 6}`);
    return <>
      <polyline points={pts(false).join(" ")} fill="none" stroke="#fff" strokeOpacity=".35" strokeWidth="1.4" strokeLinejoin="round" />
      <polyline points={pts(true).join(" ")} fill="none" stroke="#fff" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round" />
      <circle cx="50" cy="56" r="5" fill="none" stroke="#fff" strokeWidth="1" /><circle cx="50" cy="56" r="1.6" fill="#fff" />
    </>;
  }
  // a season moment: a star burst and a banner
  return <>
    <g transform="translate(50 52)">{[...Array(12)].map((_, i) => <path key={i} d="M0 -6 L1.6 -30 L-1.6 -30 Z" fill="#fff" fillOpacity=".22" transform={`rotate(${i * 30})`} />)}</g>
    <path d="M50 36 l4.7 9.6 10.6 1.5 -7.7 7.5 1.8 10.5 -9.4 -5 -9.4 5 1.8 -10.5 -7.7 -7.5 10.6 -1.5z" fill="#fff" />
  </>;
}

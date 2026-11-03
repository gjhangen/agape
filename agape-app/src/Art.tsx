import { React } from "./r";

// Flat illustrations for menu items, drawn on a 64×64 grid. Colors are the food's own,
// placed on a themed tile so they read in light and dark.
const C = {
  bun: "#E9A44C", bunD: "#C9812E", patty: "#6B3A22", cheese: "#F6C445", lettuce: "#6DB24A", tomato: "#E2483D",
  fry: "#F3C24B", box: "#D7263D", boxD: "#A81C2F", cup: "#F4F4F2", cupD: "#D9D9D4", cola: "#5A2A1C", lid: "#2F6DB5",
  water: "#8CC8F0", waterD: "#4F9FD6", crust: "#E3A956", sauce: "#D9402B", pep: "#B12A1E", mozz: "#F7D88A",
  beer: "#F2B233", foam: "#FFF8E7", glass: "#CFE7F2", tortilla: "#EBC27A", meat: "#8A4B2A", onion: "#F3F0E8",
  cilantro: "#4FA04A", chip: "#F0BE4C", queso: "#F6D24A", jal: "#3E8E3A", bowl: "#E9E4D8", rice: "#FBF7EE", bean: "#3A2A2A",
  horch: "#F3E6D0", cinn: "#B9864F", jersey: "#B8174F", jerseyD: "#8E0F3B", num: "#FFFFFF", beanie: "#2F6DB5", pom: "#F4F4F2",
  finger: "#F2B233", fingerD: "#D18E14", dog: "#B5532F", mustard: "#F2C230", pop: "#FFF6DC", popBox: "#D7263D",
  pretzel: "#9A5A2A", salt: "#FFFFFF", candy: "#E8578C", candy2: "#5BBAE8", seltzer: "#C9E6D7", can: "#9AB4C8", ink: "#1D1D1F",
};

const art: Record<string, JSX.Element> = {
  burger: <g>
    <path d="M12 30c0-10 9-16 20-16s20 6 20 16z" fill={C.bun} /><circle cx="24" cy="21" r="1.2" fill="#fff" /><circle cx="33" cy="18" r="1.2" fill="#fff" /><circle cx="40" cy="23" r="1.2" fill="#fff" />
    <path d="M10 31h44l-4 4h-36z" fill={C.lettuce} /><rect x="11" y="34" width="42" height="4" rx="1" fill={C.cheese} />
    <rect x="11" y="37" width="42" height="7" rx="3.5" fill={C.patty} /><path d="M12 45h40c0 4-3 6-6 6H18c-3 0-6-2-6-6z" fill={C.bunD} />
  </g>,
  veggie: <g>
    <path d="M12 30c0-10 9-16 20-16s20 6 20 16z" fill={C.bun} /><path d="M10 31h44l-4 4h-36z" fill={C.lettuce} />
    <rect x="11" y="34" width="42" height="4" rx="1" fill={C.tomato} /><rect x="11" y="37" width="42" height="7" rx="3.5" fill={C.bean} />
    <circle cx="20" cy="40" r="1.2" fill={C.lettuce} /><circle cx="30" cy="41" r="1.2" fill={C.lettuce} /><circle cx="42" cy="40" r="1.2" fill={C.lettuce} />
    <path d="M12 45h40c0 4-3 6-6 6H18c-3 0-6-2-6-6z" fill={C.bunD} />
  </g>,
  fries: <g>
    {[18, 23, 28, 33, 38, 43].map((x, i) => <rect key={x} x={x} y={10 + (i % 3) * 3} width="4" height="22" rx="1" fill={C.fry} transform={`rotate(${(i - 2.5) * 4} ${x} 32)`} />)}
    <path d="M14 28h36l-4 26H18z" fill={C.box} /><path d="M14 28h36l-1 6H15z" fill={C.boxD} />
  </g>,
  tenders: <g>
    <path d="M10 40h44l-4 12H14z" fill={C.box} />
    <path d="M16 40c-2-10 2-18 8-19s7 8 6 19z" fill={C.crust} /><path d="M28 40c0-12 5-20 10-19s6 10 3 19z" fill={C.bun} />
    {[40, 44, 48].map((x, i) => <rect key={x} x={x} y={16 + i * 2} width="3.5" height="24" rx="1" fill={C.fry} />)}
  </g>,
  soda: <g>
    <path d="M18 18h28l-3 38H21z" fill={C.cup} /><path d="M19.5 30h25l-1.6 20H21.1z" fill={C.box} />
    <rect x="16" y="14" width="32" height="6" rx="2" fill={C.lid} /><rect x="34" y="4" width="3" height="12" rx="1" transform="rotate(12 35 10)" fill={C.ink} />
  </g>,
  water: <g>
    <rect x="24" y="8" width="16" height="6" rx="2" fill={C.lid} /><path d="M24 14h16v4c4 3 6 7 6 12v22c0 3-2 5-5 5H23c-3 0-5-2-5-5V30c0-5 2-9 6-12z" fill={C.water} />
    <rect x="18" y="32" width="28" height="10" fill={C.waterD} /><path d="M22 20c-2 3-3 6-3 9" stroke="#fff" strokeWidth="2" fill="none" strokeLinecap="round" opacity=".7" />
  </g>,
  slice: <g>
    <path d="M32 56L10 14c14-6 30-6 44 0z" fill={C.mozz} /><path d="M10 14c14-6 30-6 44 0l-2 5c-13-5-27-5-40 0z" fill={C.crust} />
    <circle cx="26" cy="24" r="4" fill={C.pep} /><circle cx="38" cy="26" r="4" fill={C.pep} /><circle cx="32" cy="38" r="3.5" fill={C.pep} />
  </g>,
  cheese: <g>
    <path d="M32 56L10 14c14-6 30-6 44 0z" fill={C.mozz} /><path d="M10 14c14-6 30-6 44 0l-2 5c-13-5-27-5-40 0z" fill={C.crust} />
    <path d="M24 26c3 2 6 1 8 4s5 2 7 4" stroke={C.cheese} strokeWidth="3" fill="none" strokeLinecap="round" />
  </g>,
  knots: <g>
    {[[22, 26], [40, 24], [30, 40]].map(([x, y], i) => <g key={i}><circle cx={x} cy={y} r="10" fill={C.crust} /><path d={`M${x - 6} ${y}c4-6 8 6 12 0`} stroke={C.bunD} strokeWidth="2.5" fill="none" /><circle cx={x - 2} cy={y - 4} r="1.2" fill={C.lettuce} /></g>)}
  </g>,
  beer: <g>
    <path d="M16 16h28v38c0 2-2 4-4 4H20c-2 0-4-2-4-4z" fill={C.beer} /><path d="M44 24h5c3 0 5 2 5 5v10c0 3-2 5-5 5h-5" stroke={C.glass} strokeWidth="4" fill="none" />
    <path d="M14 18c0-5 4-8 8-7 2-3 7-4 10-1 3-2 8-1 9 3 4 0 6 3 5 6H14z" fill={C.foam} />
    {[[24, 34], [32, 44], [36, 30]].map(([x, y], i) => <circle key={i} cx={x} cy={y} r="1.5" fill="#fff" opacity=".7" />)}
  </g>,
  craft: <g>
    <path d="M18 14h28l-4 40c0 2-2 4-4 4H26c-2 0-4-2-4-4z" fill="#C46A1F" /><path d="M16 16c0-5 4-7 8-6 2-3 7-3 9 0 3-2 8-1 9 3 3 0 5 2 4 5H16z" fill={C.foam} />
  </g>,
  seltzer: <g>
    <rect x="20" y="10" width="24" height="46" rx="5" fill={C.seltzer} /><rect x="20" y="10" width="24" height="5" rx="2" fill={C.can} />
    <rect x="20" y="51" width="24" height="5" rx="2" fill={C.can} /><path d="M26 26c4-6 8 6 12 0" stroke={C.waterD} strokeWidth="2.5" fill="none" /><circle cx="32" cy="38" r="4" fill={C.candy} />
  </g>,
  cocktail: <g>
    <rect x="20" y="10" width="24" height="46" rx="5" fill={C.candy} /><rect x="20" y="10" width="24" height="5" rx="2" fill={C.can} />
    <rect x="20" y="51" width="24" height="5" rx="2" fill={C.can} /><path d="M26 28h12l-6 8z" fill="#fff" /><rect x="31" y="36" width="2" height="7" fill="#fff" />
  </g>,
  pretzel: <g>
    <path d="M32 50c-14-6-22-14-20-24 1-8 10-10 14-4 3 5 4 12 6 18 2-6 3-13 6-18 4-6 13-4 14 4 2 10-6 18-20 24z" fill="none" stroke={C.pretzel} strokeWidth="7" strokeLinejoin="round" />
    {[[18, 24], [44, 22], [26, 36], [40, 38], [32, 46]].map(([x, y], i) => <rect key={i} x={x} y={y} width="2" height="2" fill={C.salt} />)}
  </g>,
  tacos: <g>
    {[0, 1, 2].map(i => <g key={i} transform={`translate(${-14 + i * 14} ${i % 2 ? 2 : 0})`}>
      <path d="M22 44c0-12 7-20 10-20s10 8 10 20z" fill={C.tortilla} /><path d="M24 40c1-8 5-12 8-12s7 4 8 12z" fill={C.meat} />
      <circle cx="29" cy="34" r="1.6" fill={C.onion} /><circle cx="34" cy="31" r="1.6" fill={C.cilantro} /><circle cx="36" cy="36" r="1.4" fill={C.tomato} />
    </g>)}
  </g>,
  nachos: <g>
    <path d="M8 44h48l-5 10H13z" fill={C.box} />
    {[[14, 40, -20], [24, 34, 10], [34, 38, -8], [44, 33, 20], [28, 28, 0]].map(([x, y, r], i) => <path key={i} d={`M${x} ${y}l10 0l-5-10z`} fill={C.chip} transform={`rotate(${r} ${x + 5} ${y})`} />)}
    <path d="M14 38c6 4 12-2 18 2s12-2 18 1" stroke={C.queso} strokeWidth="4" fill="none" strokeLinecap="round" /><circle cx="22" cy="34" r="2" fill={C.jal} /><circle cx="40" cy="34" r="2" fill={C.jal} />
  </g>,
  bowl: <g>
    <path d="M8 32h48c0 14-10 22-24 22S8 46 8 32z" fill={C.bowl} />
    <path d="M10 32c2-6 8-9 14-8 3-4 12-4 15 0 6-1 12 2 15 8z" fill={C.rice} />
    <circle cx="20" cy="29" r="3" fill={C.bean} /><circle cx="27" cy="27" r="3" fill={C.bean} /><circle cx="38" cy="27" r="3.5" fill={C.lettuce} /><circle cx="45" cy="29" r="3" fill={C.tomato} /><circle cx="32" cy="30" r="3" fill={C.cheese} />
  </g>,
  horchata: <g>
    <path d="M18 16h28l-3 40H21z" fill={C.cupD} opacity=".6" /><path d="M19 22h26l-2.5 34H21.5z" fill={C.horch} />
    {[[26, 30], [34, 38], [29, 46], [37, 28]].map(([x, y], i) => <circle key={i} cx={x} cy={y} r="1.3" fill={C.cinn} />)}
    <rect x="36" y="6" width="3" height="16" rx="1" transform="rotate(14 37 14)" fill={C.ink} />
  </g>,
  jersey: <g>
    <path d="M22 10l-12 8 4 12 6-3v29h24V27l6 3 4-12-12-8c-2 4-6 6-10 6s-8-2-10-6z" fill={C.jersey} />
    <path d="M22 10c2 4 6 6 10 6s8-2 10-6" stroke={C.jerseyD} strokeWidth="3" fill="none" />
    <text x="32" y="44" textAnchor="middle" fontSize="16" fontWeight="800" fill={C.num} fontFamily="Big Shoulders Display, Impact, sans-serif">23</text>
  </g>,
  beanie: <g>
    <circle cx="32" cy="12" r="6" fill={C.pom} /><path d="M12 42c0-14 9-26 20-26s20 12 20 26z" fill={C.beanie} />
    <rect x="10" y="40" width="44" height="12" rx="4" fill="#244F86" />{[16, 24, 32, 40, 48].map(x => <rect key={x} x={x - 1} y="42" width="2" height="8" fill="#1A3A66" />)}
  </g>,
  finger: <g>
    <path d="M26 6h10v24h10c3 0 5 2 5 5v14c0 6-5 9-10 9H24c-5 0-10-4-10-9V36c0-3 2-5 5-5h7z" fill={C.finger} />
    <path d="M14 40h37" stroke={C.fingerD} strokeWidth="2" /><text x="32" y="52" textAnchor="middle" fontSize="8" fontWeight="800" fill={C.ink} fontFamily="Big Shoulders Display, Impact, sans-serif">#1</text>
  </g>,
  hotdog: <g>
    <path d="M6 36c0-6 4-8 8-8h36c4 0 8 2 8 8 0 8-8 12-26 12S6 44 6 36z" fill={C.bun} />
    <rect x="4" y="27" width="56" height="10" rx="5" fill={C.dog} />
    <path d="M10 31c4-3 6 3 10 0s6 3 10 0 6 3 10 0 6 3 10 0 4 2 6 0" stroke={C.mustard} strokeWidth="2.5" fill="none" strokeLinecap="round" />
  </g>,
  popcorn: <g>
    {[[20, 20], [28, 14], [36, 16], [44, 21], [24, 26], [34, 24], [40, 27], [16, 27], [48, 28]].map(([x, y], i) => <circle key={i} cx={x} cy={y} r="6" fill={C.pop} stroke={C.queso} strokeWidth="1" />)}
    <path d="M12 28h40l-5 28H17z" fill="#fff" />{[16, 26, 36, 46].map(x => <path key={x} d={`M${x} 28h5l-1.6 28h-4.4z`} fill={C.popBox} />)}
  </g>,
  candy: <g>
    <g transform="rotate(-18 32 32)"><rect x="14" y="24" width="36" height="16" rx="8" fill={C.candy} /><path d="M14 32l-8-7v14zM50 32l8-7v14z" fill={C.candy} /><path d="M24 24v16M40 24v16" stroke="#fff" strokeWidth="2" opacity=".6" /></g>
    <circle cx="16" cy="48" r="6" fill={C.candy2} /><circle cx="48" cy="14" r="5" fill={C.cheese} />
  </g>,
};

export function artKey(name: string): string {
  const n = name.toLowerCase();
  if (n.includes("black bean")) return "veggie";
  if (n.includes("burger")) return "burger";
  if (n.includes("tender")) return "tenders";
  if (n.includes("fries")) return "fries";
  if (n.includes("water")) return "water";
  if (n.includes("soda")) return "soda";
  if (n.includes("pepperoni")) return "slice";
  if (n.includes("cheese slice")) return "cheese";
  if (n.includes("knot")) return "knots";
  if (n.includes("craft")) return "craft";
  if (n.includes("draft")) return "beer";
  if (n.includes("seltzer")) return "seltzer";
  if (n.includes("cocktail")) return "cocktail";
  if (n.includes("pretzel")) return "pretzel";
  if (n.includes("taco")) return "tacos";
  if (n.includes("nacho")) return "nachos";
  if (n.includes("bowl")) return "bowl";
  if (n.includes("horchata")) return "horchata";
  if (n.includes("jersey")) return "jersey";
  if (n.includes("beanie")) return "beanie";
  if (n.includes("finger")) return "finger";
  if (n.includes("hot dog")) return "hotdog";
  if (n.includes("popcorn")) return "popcorn";
  if (n.includes("candy")) return "candy";
  return "soda";
}

export function ItemArt({ name, size = 64, className }: { name: string; size?: number; className?: string }) {
  return <svg className={"art " + (className ?? "")} width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">{art[artKey(name)]}</svg>;
}

/** Signature item for a stand: the first non-drink item, else the first item. */
export function standSignature(menu: { standId: string; name: string; category: string }[], standId: string) {
  const items = menu.filter(m => m.standId === standId) as any[];
  const booze = items.filter(m => m.alcohol);
  if (booze.length >= 2) return booze[0].name;
  return (items.find(m => m.category !== "drink") ?? items[0])?.name ?? "";
}

// The web of where Agape goes: nine moves, and on every arrow what one move hands the next. Drawn once, shown on
// the investor page (wide screens) and in the summary PDF. Colours come from CSS variables with print fallbacks,
// so the page's light and dark looks both work.
const A = "var(--wb-accent, #B8174F)", INK = "var(--wb-ink, #17181C)", SOFT = "var(--wb-soft, #4A4D57)";
const BOX = "var(--wb-box, #FFFFFF)", LINE = "var(--wb-line, #D9DAE0)";
const W = 212, H = 74, cx = { L: 122, C: 380, R: 638 }, cy = { 1: 76, 2: 241, 3: 406 };

const esc = s => s.replace(/&/g, "&amp;").replace(/</g, "&lt;");
function node(col, row, name, a, b, pool, kind = "") {
  const x = cx[col] - W / 2, y = cy[row] - H / 2;
  const fill = kind === "door" ? A : kind === "big" ? "#120A0E" : BOX;
  const stroke = kind ? fill : LINE;
  const ink = kind ? "#FFFFFF" : INK, soft = kind ? "rgba(255,255,255,.82)" : SOFT;
  const num = kind === "big" ? "#FFFFFF" : A;
  return `<g><rect x="${x}" y="${y}" width="${W}" height="${H}" rx="12" fill="${fill}" stroke="${stroke}" stroke-width="1.25"/>`
    + `<text x="${x + 14}" y="${y + 27}" font-family="Big Shoulders Display, sans-serif" font-weight="800" font-size="20" fill="${ink}" style="text-transform:uppercase">${esc(name.toUpperCase())}</text>`
    + (pool ? `<text x="${x + W - 14}" y="${y + 28}" text-anchor="end" font-family="Big Shoulders Display, sans-serif" font-weight="800" font-size="${kind === "big" ? 26 : 23}" fill="${num}">${esc(pool)}</text>` : "")
    + `<text x="${x + 14}" y="${y + 47}" font-family="Public Sans, sans-serif" font-size="11.5" fill="${soft}">${esc(a)}</text>`
    + `<text x="${x + 14}" y="${y + 62}" font-family="Public Sans, sans-serif" font-size="11" font-weight="${pool ? 700 : 400}" fill="${pool && !kind ? SOFT : soft}">${esc(b)}</text></g>`;
}
const say = (x, y, t, anchor = "start") => `<text x="${x}" y="${y}" text-anchor="${anchor}" font-family="Public Sans, sans-serif" font-size="11.5" font-weight="700" fill="${A}">${esc(t)}</text>`;

export const WEB_SVG = `<svg viewBox="0 0 760 500" role="img" aria-label="Where Agape goes: food to the seat leads to sponsors, local deals, the wallet and social; local deals lead to processing, the wallet and social to the marketplace, social and processing to advertising" xmlns="http://www.w3.org/2000/svg">
<defs><marker id="wb-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" fill="${A}"/></marker></defs>
<g fill="none" stroke="${A}" stroke-width="1.6" stroke-opacity=".6">
<path d="M274 76H231" marker-end="url(#wb-arrow)"/><path d="M486 76H529" marker-end="url(#wb-arrow)"/>
<path d="M380 113V201" marker-end="url(#wb-arrow)"/><path d="M300 113L162 201" marker-end="url(#wb-arrow)"/><path d="M460 113L598 201" marker-end="url(#wb-arrow)"/>
<path d="M122 278V366" marker-end="url(#wb-arrow)"/><path d="M380 278V366" marker-end="url(#wb-arrow)"/><path d="M690 278V366" marker-end="url(#wb-arrow)"/><path d="M560 278L462 366" marker-end="url(#wb-arrow)"/>
<path d="M122 443V468H638V446" marker-end="url(#wb-arrow)"/>
</g>
${say(251, 28, "venues run on it", "middle")}${say(508, 28, "fans mid-purchase", "middle")}
${say(16, 150, "fans nearby, before + after")}${say(372, 160, "every fan pays Agape", "end")}${say(560, 150, "fans by team + section")}
${say(130, 306, "proof of customers sent")}${say(372, 344, "money already inside", "end")}${say(496, 344, "buyers already there")}${say(682, 300, "daily attention", "end")}
${say(380, 490, "closed loop: Agape shows the ad, then processes the sale it caused", "middle")}
${node("L", 1, "Runners + venue tools", "A paid crew inside every venue,", "running the night and its books")}
${node("C", 1, "Food to the seat", "Fans order from their seat", "and pay Agape first", "", "door")}
${node("R", 1, "Sponsors", "Prizes, samples, offers", "sponsorship, worldwide", "$39B")}
${node("L", 2, "Local deals", "Offers from places nearby,", "before and after the event")}
${node("C", 2, "Wallet + payouts", "Fans keep value in Agape;", "everyone is paid through it")}
${node("R", 2, "Social", "Teams, players, sections", "and friends, between events")}
${node("L", 3, "Processing", "Their cards run through Agape", "US card purchases a year", "$11.9T", "big")}
${node("C", 3, "Marketplace", "Resale, merch, collectibles", "sports merch, tickets, resale", "$80B")}
${node("R", 3, "Advertising", "Brands reach fans every day", "US local advertising", "$169B")}
</svg>`;

// Renders brand/summary.html to brand/agape-summary.pdf (one Letter page). Run after the investor case changes:
//   node brand/make-summary.cjs
const { chromium } = require("playwright");
const path = require("path");
(async () => {
  const b = await chromium.launch(); const p = await b.newPage();
  await p.goto("file://" + path.join(__dirname, "summary.html"), { waitUntil: "networkidle" });
  await p.evaluate(() => document.fonts.ready);
  const over = await p.evaluate(() => { const pg = document.querySelector(".page"); return pg.scrollHeight - pg.clientHeight; });
  if (over > 0) console.log("WARNING: content runs " + over + "px past the page");
  await p.pdf({ path: path.join(__dirname, "agape-summary.pdf"), format: "Letter", printBackground: true, pageRanges: "1" });
  await p.screenshot({ path: process.argv[2] || "/dev/null", fullPage: false }).catch(() => {});
  await b.close(); console.log("brand/agape-summary.pdf");
})();

// Renders brand/paper.html ("Where Agape goes", the long paper for the investor room) to
// brand/partner/room-where-agape-goes.pdf, which ships with the server as a private file.
//   node brand/make-paper.cjs
const { chromium } = require("playwright");
const path = require("path");
(async () => {
  const b = await chromium.launch(); const p = await b.newPage();
  await p.goto("file://" + path.join(__dirname, "paper.html"), { waitUntil: "networkidle" });
  const { WEB_SVG } = await import(path.join(__dirname, "web.mjs"));
  await p.evaluate(svg => { document.getElementById("web").innerHTML = svg; }, WEB_SVG);
  await p.evaluate(() => document.fonts.ready);
  const out = path.join(__dirname, "partner", "room-where-agape-goes.pdf");
  await p.pdf({ path: out, format: "Letter", printBackground: true, displayHeaderFooter: true, headerTemplate: "<span></span>",
    footerTemplate: '<div style="width:100%;font:7pt Helvetica,sans-serif;color:#8A8D96;padding:0 .75in;display:flex;justify-content:space-between"><span>Agape · Where Agape goes · Confidential</span><span class="pageNumber"></span></div>' });
  await b.close(); console.log(out);
})();

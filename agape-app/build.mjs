import { build } from "esbuild";
import { readFileSync, writeFileSync, mkdirSync } from "fs";

const K = "/home/claude/agape-kernel";
const wasm = readFileSync(`${K}/target/wasm32-unknown-unknown/wasm/agape_kernel.wasm`).toString("base64");
const world = readFileSync(`${K}/templates/arena.json`, "utf8");
const tpl = readFileSync(`${K}/templates/basketball.json`, "utf8");

const r = await build({ entryPoints: ["src/main.tsx"], bundle: true, write: false, format: "iife", target: "es2020", minify: true,
  jsxFactory: "React.createElement", jsxFragment: "React.Fragment", logLevel: "warning" });
const js = r.outputFiles[0].text.replace(/<\/script/gi, "<\\/script");
const css = readFileSync("src/styles.css", "utf8");
const html = `<title>Agape Live Night</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Big+Shoulders+Display:wght@700;800&family=Public+Sans:wght@400;500;600;700;800&family=IBM+Plex+Mono:wght@500;600&family=IBM+Plex+Sans:wght@400;600;700&family=Oswald:wght@600;700&family=Inter:wght@400;500;600;700&family=Anton&family=Work+Sans:wght@400;500;600;700&family=Space+Grotesk:wght@500;700&family=DM+Sans:wght@400;500;700&family=Fraunces:opsz,wght@9..144,700&family=Nunito:wght@400;600;700;900&family=Bebas+Neue&family=Barlow:wght@400;500;600;700&display=swap">
<style>${css}</style>
<div id="root"><div class="gate"><h1>Harbor Arena</h1><p>Opening the doors…</p></div></div>
<script src="https://cdnjs.cloudflare.com/ajax/libs/react/18.3.1/umd/react.production.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/react-dom/18.3.1/umd/react-dom.production.min.js"></script>
<script>var AGAPE_WASM="${wasm}";var AGAPE_WORLD=${world};var AGAPE_TEMPLATE=${tpl};</script>
<script>${js}</script>
`;
mkdirSync("dist", { recursive: true });
writeFileSync("dist/agape-live.html", html);
console.log("dist/agape-live.html", (html.length / 1024).toFixed(0) + " KB");

// The app people use, served by the venue's server: no simulation, no kernel in the page.
const r2 = await build({ entryPoints: ["src/remote.tsx"], bundle: true, write: false, format: "iife", target: "es2020", minify: true,
  jsxFactory: "React.createElement", jsxFragment: "React.Fragment", logLevel: "warning" });
const js2 = r2.outputFiles[0].text.replace(/<\/script/gi, "<\\/script");
const page = `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>Agape</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Big+Shoulders+Display:wght@700;800&family=Public+Sans:wght@400;500;600;700;800&family=IBM+Plex+Mono:wght@500;600&family=IBM+Plex+Sans:wght@400;600;700&family=Oswald:wght@600;700&family=Inter:wght@400;500;600;700&family=Anton&family=Work+Sans:wght@400;500;600;700&family=Space+Grotesk:wght@500;700&family=DM+Sans:wght@400;500;700&family=Fraunces:opsz,wght@9..144,700&family=Nunito:wght@400;600;700;900&family=Bebas+Neue&family=Barlow:wght@400;500;600;700&display=swap">
<style>${css}</style></head><body>
<div id="root"><div class="gate"><h1>Harbor Arena</h1><p>Opening the doors…</p></div></div>
<script src="https://cdnjs.cloudflare.com/ajax/libs/react/18.3.1/umd/react.production.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/react-dom/18.3.1/umd/react-dom.production.min.js"></script>
<script>var AGAPE_WORLD=${world};var AGAPE_TEMPLATE=${tpl};</script>
<script>${js2}</script></body></html>
`;
mkdirSync("dist/server", { recursive: true });
writeFileSync("dist/server/index.html", page);
console.log("dist/server/index.html", (page.length / 1024).toFixed(0) + " KB");

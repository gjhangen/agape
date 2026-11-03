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
const html = `<title>Agape Simulator</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Big+Shoulders+Display:wght@700;800&family=Public+Sans:wght@400;500;600;700;800&family=IBM+Plex+Mono:wght@500;600&family=IBM+Plex+Sans:wght@400;600;700&family=Oswald:wght@600;700&family=Inter:wght@400;500;600;700&family=Anton&family=Work+Sans:wght@400;500;600;700&family=Space+Grotesk:wght@500;700&family=DM+Sans:wght@400;500;700&family=Fraunces:opsz,wght@9..144,700&family=Nunito:wght@400;600;700;900&family=Bebas+Neue&family=Barlow:wght@400;500;600;700&display=swap">
<style>${css}</style>
<div id="root"><div class="landing"><header class="l-top"><span class="l-mark">agape</span><span class="l-tag">to your seat</span></header><p class="l-sub">Opening the doors…</p></div></div>
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
<title>Agape · to your seat</title>
<meta name="description" content="The game, and everything around it, from your seat. Order to your seat, play along live, and keep points that travel with you.">
<meta property="og:title" content="Agape · to your seat">
<meta property="og:description" content="The game, and everything around it, from your seat.">
<meta property="og:image" content="https://agapetoyourseat.com/og.png">
<meta property="og:image:width" content="1200"><meta property="og:image:height" content="630">
<meta name="twitter:card" content="summary_large_image">
<meta name="theme-color" content="#B8174F">
<meta name="apple-mobile-web-app-capable" content="yes"><meta name="mobile-web-app-capable" content="yes">
<meta name="apple-mobile-web-app-title" content="Agape"><meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
<link rel="apple-touch-icon" href="/apple-touch-icon.png"><link rel="icon" href="/favicon.png"><link rel="manifest" href="/manifest.json">
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

// the icons, share image and manifest that make it feel like an app on a phone
import { copyFileSync } from "fs";
for (const f of ["og.png", "icon-512.png", "icon-192.png", "apple-touch-icon.png", "favicon.png"]) copyFileSync(`brand/${f}`, `dist/server/${f}`);
writeFileSync("dist/server/manifest.json", JSON.stringify({ name: "Agape", short_name: "Agape", start_url: "/", display: "standalone", background_color: "#F3F4F7", theme_color: "#B8174F",
  icons: [{ src: "/icon-192.png", sizes: "192x192", type: "image/png" }, { src: "/icon-512.png", sizes: "512x512", type: "image/png" }] }));

// The simulator, hosted next to the app: a whole night in the visitor's own browser.
mkdirSync("dist/server/demo", { recursive: true });
writeFileSync("dist/server/demo/index.html", `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta name="description" content="A whole arena night on Agape, simulated in your browser. See every seat, speed up time, and put your own team on it.">
<meta property="og:title" content="Agape Simulator"><meta property="og:description" content="A whole arena night on Agape, simulated in your browser.">
<meta property="og:image" content="https://agapetoyourseat.com/og.png"><meta name="twitter:card" content="summary_large_image">
<meta name="theme-color" content="#B8174F"><link rel="icon" href="/favicon.png"><link rel="apple-touch-icon" href="/apple-touch-icon.png">
</head><body>${html.replace("<title>Agape Simulator</title>", "<title>Agape Simulator</title>")}</body></html>`);
console.log("dist/server/demo/index.html");

// The team's own page: who asked to talk. Needs the operator key from the server.
mkdirSync("dist/server/leads", { recursive: true });
writeFileSync("dist/server/leads/index.html", `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta name="robots" content="noindex">
<title>Agape · who asked to talk</title><style>
body{font:15px/1.45 -apple-system,system-ui,sans-serif;margin:0;background:#F3F4F7;color:#14161B}main{max-width:900px;margin:0 auto;padding:24px 16px}
h1{font-size:22px}input{font:inherit;padding:10px 12px;border:1px solid #D8DCE3;border-radius:10px;width:min(420px,100%)}button{font:inherit;font-weight:700;padding:10px 16px;border:0;border-radius:10px;background:#B8174F;color:#fff;margin-left:6px}
.l{background:#fff;border:1px solid #E3E6EC;border-radius:12px;padding:14px;margin-top:10px}.l b{font-size:16px}.m{color:#626977;font-size:13px}p.n{white-space:pre-wrap;margin:6px 0 0}
@media (prefers-color-scheme: dark){body{background:#0E1014;color:#EDEFF3}.l{background:#171A20;border-color:#2A2F38}input{background:#171A20;color:#EDEFF3;border-color:#2A2F38}.m{color:#8A91A0}}
</style></head><body><main><h1>Who asked to talk</h1>
<form id="f"><input id="k" type="password" placeholder="Operator key (AGAPE_ADMIN_TOKEN on the server)" autocomplete="current-password"><button>Show</button></form><div id="out"></div></main>
<script>
const out=document.getElementById('out'),k=document.getElementById('k');k.value=localStorage.getItem('agape.op')||'';
const esc=s=>String(s||'').replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
async function load(){const r=await fetch('/api/admin/leads',{headers:{'x-admin-token':k.value}});if(!r.ok){out.textContent='That key is not right.';return}localStorage.setItem('agape.op',k.value);
const {leads}=await r.json();out.innerHTML=leads.length?leads.reverse().map(l=>'<div class="l"><b>'+esc(l.name)+'</b> · '+esc(l.role)+(l.org?' · '+esc(l.org):'')+'<div class="m">'+esc(l.contact)+' · '+new Date(l.at*1000).toLocaleString()+(l.version?' · from shared version '+esc(l.version):'')+'</div>'+(l.note?'<p class="n">'+esc(l.note)+'</p>':'')+'</div>').join(''):'<p>Nobody yet.</p>'}
document.getElementById('f').onsubmit=e=>{e.preventDefault();load()};if(k.value)load();
</script></body></html>`);

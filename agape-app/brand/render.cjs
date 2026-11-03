const { chromium } = require('playwright');
(async () => { const d = process.argv[2]; const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1200, height: 630 } });
  await p.goto('file://' + __dirname + '/og.html'); await p.evaluate(() => document.fonts.ready); await p.waitForTimeout(800);
  await p.screenshot({ path: d + '/og.png' });
  await p.setViewportSize({ width: 512, height: 512 }); await p.goto('file://' + __dirname + '/icon.html'); await p.evaluate(() => document.fonts.ready); await p.waitForTimeout(500);
  await p.screenshot({ path: d + '/icon-512.png' });
  await b.close(); })();

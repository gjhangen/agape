// Putting Agape on a phone's home screen (or a computer's dock) from the web: its own icon, full screen, always the
// latest version. Chrome and Edge (Android, Windows, Mac) offer an install the page can trigger; it fires once, early,
// so it's caught here as soon as any page loads. iPhones and iPads only install from Safari's Share menu, so they get
// the steps instead.
import { React, useEffect, useState } from "./r";
import qrcode from "qrcode-generator";

let offer: any = null;
const subs = new Set<() => void>();
if (typeof window !== "undefined") {
  addEventListener("beforeinstallprompt", (e: any) => { e.preventDefault(); offer = e; subs.forEach(f => f()); });
  addEventListener("appinstalled", () => { offer = null; subs.forEach(f => f()); });
  // a small background worker: some browsers only offer an install once a site has one. It caches nothing,
  // so what people see is always the live site.
  if ("serviceWorker" in navigator) addEventListener("load", () => { navigator.serviceWorker.register("/sw.js").catch(() => {}); });
}

export const device = () => {
  const ua = navigator.userAgent;
  const ios = /iPhone|iPad|iPod/.test(ua) || (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1);
  const android = /Android/.test(ua);
  const phone = ios || android || /Mobi/.test(ua);
  const installed = matchMedia("(display-mode: standalone)").matches || (navigator as any).standalone === true;
  return { ios, android, phone, installed };
};

/** Whether this browser can install right now with one tap, and the tap. */
export function useInstall() {
  const [, tick] = useState(0);
  useEffect(() => { const f = () => tick(x => x + 1); subs.add(f); return () => { subs.delete(f); }; }, []);
  return { can: !!offer, install: async () => { if (!offer) return false; offer.prompt(); const r = await offer.userChoice.catch(() => null); offer = null; subs.forEach(f => f()); return r?.outcome === "accepted"; } };
}

/** A QR code (as an inline picture) that opens a link on a phone. */
export function QR({ text, size = 168 }: { text: string; size?: number }) {
  const q = qrcode(0, "M"); q.addData(text); q.make();
  const n = q.getModuleCount(), cell = size / (n + 8);
  let d = "";
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) if (q.isDark(r, c)) d += `M${(c + 4) * cell} ${(r + 4) * cell}h${cell}v${cell}h-${cell}z`;
  return <svg className="qr" viewBox={`0 0 ${size} ${size}`} width={size} height={size} role="img" aria-label="QR code to open this on your phone"><rect width={size} height={size} rx="12" fill="#fff" /><path d={d} fill="#14161B" /></svg>;
}

const ShareIcon = () => <svg viewBox="0 0 24 24" aria-hidden="true" className="ins-ico"><path d="M12 3v12M7.5 7.5 12 3l4.5 4.5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" /><path d="M8 11H6a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-8a1 1 0 0 0-1-1h-2" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" /></svg>;

/** How to install the app this page belongs to, for the device in hand. */
export function InstallSteps({ name, link }: { name: string; link?: string }) {
  const d = device(); const { can, install } = useInstall();
  const [done, setDone] = useState(false);
  if (d.installed) return <p className="ins-done">{name} is installed. It opens from its icon, full screen.</p>;
  if (done) return <p className="ins-done">Installed. {name} is on your {d.phone ? "home screen" : "computer"} now.</p>;
  return <div className="ins">
    {can && <button className="btn primary big block" onClick={async () => setDone(await install())}>Install {name}</button>}
    {!can && d.ios && <ol className="ins-steps">
      <li><span><b>Tap Share</b> <ShareIcon />{" "}at the {/iPad/.test(navigator.userAgent) ? "top" : "bottom"} of Safari{/CriOS|FxiOS/.test(navigator.userAgent) ? " (or in Chrome's address bar)" : ""}.</span></li>
      <li><span>Scroll down and tap <b>Add to Home Screen</b>.</span></li>
      <li><span>Tap <b>Add</b>. {name} is on your home screen.</span></li>
    </ol>}
    {!can && !d.ios && d.phone && <ol className="ins-steps">
      <li><span>Open your browser's menu <b>⋮</b>.</span></li>
      <li><span>Tap <b>Install app</b> or <b>Add to Home screen</b>.</span></li>
      <li><span>Tap <b>Install</b>. {name} is on your home screen.</span></li>
    </ol>}
    {!d.phone && link && <div className="ins-qr"><QR text={link} /><div><b>On your phone</b><span>Point your phone's camera at the code, open the link, then install it there.</span>{!can && <span className="ins-fine">On this computer: Chrome and Edge show an install button at the right of the address bar.</span>}</div></div>}
  </div>;
}

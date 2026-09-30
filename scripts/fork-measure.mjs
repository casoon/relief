#!/usr/bin/env node
// Messtreiber für die Update-Latenz im Relief-Build (Paket 17).
//
//   node scripts/fork-measure.mjs [--port 9222] [--count 60] [--interval 250]
//                                 [--scroll 0] [--parent body]
//
// Verbindet sich über CDP mit einem laufenden Relief-Build (gestartet mit
// --enable-relief --relief-log=<datei> --remote-debugging-port=<port>) und
// schreibt `count`-mal im Abstand `interval` ms den Text
// „relief-probe:<Date.now()>“ in ein kleines, sichtbares Element der ersten
// Seite. Der Relief-Tab-Helfer erkennt den Knoten nach dem Anwenden der Delta
// und protokolliert `probe latency_ms=…` = DOM-Änderung → Graph aktualisiert
// (relief/bridge/runtime_host.cc). Beide Seiten lesen die Systemuhr.
// `--scroll <px>` scrollt die Seite je Messknoten um so viele Pixel weiter;
// das löst Positionsänderungen aus (Protokollzeilen `location`, Paket 33).
// `--parent <CSS-Selektor>` hängt den Messknoten dort statt an `body` an:
// Liegt ein Dialog mit aria-modal offen (Consent auf spiegel.de), nimmt
// Blink alles außerhalb aus dem Baum, auch einen Messknoten in `body`.
//
// Nur Runtime.evaluate, keine Accessibility-Domäne: CDP ändert so den
// AXMode des Tabs nicht. Braucht Node ≥ 22 (globales WebSocket), keine
// Pakete.

const args = Object.fromEntries(
  process.argv.slice(2).reduce((pairs, arg, i, all) => {
    if (arg.startsWith("--")) pairs.push([arg.slice(2), all[i + 1]]);
    return pairs;
  }, []),
);
const port = Number(args.port ?? 9222);
const count = Number(args.count ?? 60);
const interval = Number(args.interval ?? 250);
const scroll = Number(args.scroll ?? 0);
const parent = args.parent ?? "body";

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const page = targets.find((t) => t.type === "page");
if (!page) {
  console.error("Keine Seite gefunden.");
  process.exit(1);
}

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});
let nextId = 1;
const pending = new Map();
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.id && pending.has(message.id)) {
    pending.get(message.id)(message);
    pending.delete(message.id);
  }
};
const send = (method, params) =>
  new Promise((resolve) => {
    const id = nextId++;
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params }));
  });

// Sichtbar, damit Blink den Text nicht aus dem Baum nimmt; 1 px groß, oben
// links, ohne Einfluss auf das Layout der Seite.
const probe = `(() => {
  let el = document.getElementById("relief-probe");
  if (!el) {
    el = document.createElement("div");
    el.id = "relief-probe";
    el.style.cssText =
      "position:fixed;left:0;top:0;font-size:1px;line-height:1px;z-index:2147483647";
    document.querySelector(${JSON.stringify(parent)}).appendChild(el);
  }
  el.textContent = "relief-probe:" + Date.now();
  if (${scroll}) window.scrollBy(0, ${scroll});
})()`;

for (let i = 0; i < count; i++) {
  const reply = await send("Runtime.evaluate", { expression: probe });
  if (reply.result?.exceptionDetails) {
    console.error("Fehler im Messskript:", reply.result.exceptionDetails.text);
    process.exit(1);
  }
  await sleep(interval);
}
ws.close();
console.log(`${count} Messknoten geschrieben (${page.url}).`);

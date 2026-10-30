// node native/tools/golden.mjs URL cases.json outdir
// Reference pictures from the web room, for the native port's tests: headless Chrome (no window, no
// packages) loads the page at URL (a BURROW_DEMO=1 server, never the real one), with room.js served
// so that its drawing functions are reachable as `window.__in`; each case is a JS expression giving a
// data URL (or a canvas) and is saved as outdir/<name>.png. Math.random is seeded, so a case that rolls
// dice rolls the same ones every time.
//   cases.json: [{"name": "icon-plant", "js": "room.icon('plant', 1, 0)"}, …]; {"json": true, …} saves the text it gives as .json
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";

const [url, casesFile, outdir] = process.argv.slice(2);
const cases = JSON.parse(readFileSync(casesFile, "utf8"));
mkdirSync(outdir, { recursive: true });
const dir = mkdtempSync(tmpdir() + "/golden");
const port = 9800 + Math.floor(Math.random() * 150);
const chrome = spawn("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  ["--headless=new", "--disable-gpu", "--hide-scrollbars", `--user-data-dir=${dir}`, `--remote-debugging-port=${port}`,
   "--window-size=1280,820", "about:blank"], { stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let target;
for (let i = 0; i < 50 && !target; i++) {
  await sleep(200);
  try { target = (await (await fetch(`http://127.0.0.1:${port}/json`)).json()).find((t) => t.type === "page"); } catch {}
}
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let id = 0;
const pending = new Map();
const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
// room.js with its insides exposed: everything named in startRoom's scope that a test may want
const EXPOSE = `window.__in = { body, face, wears, halo, put, slab, tiny, tinyW, floor, drawDesk, drawKind, drawFigure, drawSleeper,
  drawChair, drawFire, drawCloset, drawEasel, drawPair, drawAdd: typeof drawAdd === "undefined" ? null : drawAdd, drawSwitchboard, drawReception, drawDoor, drawClock, drawWindow,
  drawRug, drawMat, drawBooth, drawGunCloset, drawRadio, drawNeuralyzer, drawTable, drawChess, drawStool, drawCoffee, drawCooler,
  drawPrinter, drawArmchair, drawPlant, drawPong, drawDoghouse, dogPic, heart, drawGrave: typeof drawGrave === "undefined" ? null : drawGrave,
  projectColor, shade, mix, fnv, foot, layout, defaults, icon, doorPicture, printerPicture, PAINT, PROJECT, WOOD,
  get W() { return W; }, get H() { return H; }, world, artSmall, newArt, paintArt, ART, ART_SPEED, ART_REST, plate, outlined, tinted, dissolve, snapshot, pouf, zzz, screenAt,
  app, drawPart, drawNotes, drawStick, ELL, boothRise, boothGlow, boothShut, flashA, START, FIRE, GOLD, RED,
  drawPumpkin: typeof drawPumpkin === "undefined" ? null : drawPumpkin, drawCobwebs: typeof drawCobwebs === "undefined" ? null : drawCobwebs,
  drawSausage: typeof drawSausage === "undefined" ? null : drawSausage, HAT: typeof HAT === "undefined" ? 0 : HAT,
  get pointer() { return pointer; }, set pointer(p) { pointer = p; } };\n`;
ws.onmessage = async (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
  if (m.method === "Runtime.exceptionThrown") console.error("exception:", m.params.exceptionDetails.exception?.description || m.params.exceptionDetails.text);
  if (m.method === "Fetch.requestPaused") {
    const r = m.params;
    const body = await (await fetch(r.request.url)).text();
    const patched = body.replace("  return { talk, pairTalk, typed,", EXPOSE + "  return { talk, pairTalk, typed,");
    if (patched === body) console.error("room.js: could not expose its insides");
    send("Fetch.fulfillRequest", { requestId: r.requestId, responseCode: 200,
      responseHeaders: [{ name: "Content-Type", value: "text/javascript" }], body: Buffer.from(patched).toString("base64") });
  }
};
await send("Runtime.enable");
await send("Fetch.enable", { patterns: [{ urlPattern: "*room.js*", requestStage: "Request" }] });
// the same dice every time
await send("Page.addScriptToEvaluateOnNewDocument", { source: `(() => { let s = 1234567; Math.random = () => { s = (Math.imul(s, 1103515245) + 12345) >>> 0; return (s >>> 8) / 16777216; }; })();` });
await send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 820, deviceScaleFactor: 1, mobile: false });
await send("Page.enable");
await send("Page.navigate", { url });
await sleep(3000);
let ok = 0;
for (const c of cases) {
  const js = `(async () => { const v = await (${c.js}); return v && v.toDataURL ? v.toDataURL() : v; })()`;
  const r = await send("Runtime.evaluate", { expression: js, returnByValue: true, awaitPromise: true });
  const v = r.result?.result?.value;
  if (c.json && typeof v === "string") { writeFileSync(`${outdir}/${c.name}.json`, v); ok++; continue; }   // a case with json: true gives text
  if (typeof v !== "string" || !v.startsWith("data:image/png;base64,")) {
    console.error(c.name + ":", JSON.stringify(v ?? r.result?.exceptionDetails?.exception?.description).slice(0, 300));
    continue;
  }
  writeFileSync(`${outdir}/${c.name}.png`, Buffer.from(v.split(",")[1], "base64"));
  ok++;
}
console.log(`${ok}/${cases.length} written to ${outdir}`);
ws.close();
chrome.kill();
process.exit(0);

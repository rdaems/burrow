// node web/look.mjs URL W H steps.json: drive headless Chrome over CDP (no packages) to look at the page.
// steps: [{wait:ms} | {shot:"file.png"} | {js:"expr"} | {click:[x,y]} | {move:[x,y]} | {moveTo|clickTo:"js giving [x,y]"} | {drag:[[x,y],…]} | {dragTo:"js giving [[x,y],…]"} | {drag:…, hold:true} then {release:[x,y]} | {key:"text"} | {press:"Enter", shift:true}]
import { spawn } from "node:child_process";
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";

const [url, w = "1280", h = "800", stepsFile] = process.argv.slice(2);
const steps = JSON.parse(readFileSync(stepsFile, "utf8"));
const dir = mkdtempSync(tmpdir() + "/cdp");
const port = 9300 + Math.floor(Math.random() * 500);
const chrome = spawn("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  ["--headless=new", "--disable-gpu", "--hide-scrollbars", `--user-data-dir=${dir}`, `--remote-debugging-port=${port}`,
   `--window-size=${w},${h}`, "--force-device-scale-factor=2", "about:blank"], { stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let target;
for (let i = 0; i < 50 && !target; i++) {
  await sleep(200);
  try { target = (await (await fetch(`http://127.0.0.1:${port}/json`)).json()).find((t) => t.type === "page"); } catch {}
}
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let id = 0; const pending = new Map();
ws.onmessage = (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
  if (m.method === "Runtime.consoleAPICalled") console.log("console:", m.params.args.map((a) => a.value ?? a.description).join(" "));
  if (m.method === "Runtime.exceptionThrown") console.log("exception:", m.params.exceptionDetails.exception?.description || m.params.exceptionDetails.text);
};
const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
await send("Runtime.enable");
await send("Emulation.setDeviceMetricsOverride", { width: +w, height: +h, deviceScaleFactor: 2, mobile: false });
await send("Page.navigate", { url });
const mouse = async (type, [x, y], buttons = 0) => send("Input.dispatchMouseEvent", { type, x, y, button: "left", buttons, clickCount: 1, pointerType: "mouse" });
for (const s of steps) {
  if (s.wait) await sleep(s.wait);
  if (s.js) { const r = await send("Runtime.evaluate", { expression: s.js, returnByValue: true, awaitPromise: true }); console.log("js:", JSON.stringify(r.result?.result?.value ?? r.result?.exceptionDetails?.exception?.description)); }
  if (s.move) await mouse("mouseMoved", s.move);
  if (s.moveTo || s.clickTo) {               // a JS expression giving [x, y] in CSS pixels: for things that move
    const r = await send("Runtime.evaluate", { expression: s.moveTo || s.clickTo, returnByValue: true }), at = r.result.result.value;
    await mouse("mouseMoved", at);
    if (s.clickTo) { await mouse("mousePressed", at, 1); await mouse("mouseReleased", at); }
  }
  if (s.click) { await mouse("mouseMoved", s.click); await mouse("mousePressed", s.click); await mouse("mouseReleased", s.click); }
  if (s.release) { await mouse("mouseMoved", s.release, 1); await mouse("mouseReleased", s.release); }   // let go of a held drag, here
  if (s.dragTo) { const r = await send("Runtime.evaluate", { expression: s.dragTo, returnByValue: true }); s.drag = r.result.result.value; }   // the points from the page
  if (s.drag) {                              // press at the first point, move through the others, let go at the last (not with hold: true)
    await mouse("mouseMoved", s.drag[0]); await mouse("mousePressed", s.drag[0], 1);
    for (const p of s.drag.slice(1)) { await mouse("mouseMoved", p, 1); await sleep(60); }
    if (!s.hold) await mouse("mouseReleased", s.drag.at(-1));
  }
  if (s.key) await send("Input.insertText", { text: s.key });
  if (s.press) {                             // one key with modifiers (shift, alt, ctrl, meta): named keys or a letter
    const modifiers = (s.alt ? 1 : 0) | (s.ctrl ? 2 : 0) | (s.meta ? 4 : 0) | (s.shift ? 8 : 0);
    const [vk, text] = { Enter: [13, "\r"], Tab: [9, "\t"], Escape: [27, ""], Backspace: [8, ""], ArrowLeft: [37, ""], ArrowUp: [38, ""], ArrowRight: [39, ""], ArrowDown: [40, ""] }[s.press] || [s.press.toUpperCase().charCodeAt(0), s.press];
    const ev = { key: s.press, code: s.press.length > 1 ? s.press : "Key" + s.press.toUpperCase(), windowsVirtualKeyCode: vk, modifiers };
    await send("Input.dispatchKeyEvent", { type: text ? "keyDown" : "rawKeyDown", text, ...ev });
    await send("Input.dispatchKeyEvent", { type: "keyUp", ...ev });
  }
  if (s.shot) { const r = await send("Page.captureScreenshot", { format: "png" }); writeFileSync(s.shot, Buffer.from(r.result.data, "base64")); console.log("shot:", s.shot); }
}
ws.close(); chrome.kill(); await sleep(300); rmSync(dir, { recursive: true, force: true });
process.exit(0);

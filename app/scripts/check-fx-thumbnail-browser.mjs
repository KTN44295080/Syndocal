// Browser plugin not available. Focused Chromium CDP fixture; no native/output APIs.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const browserPath = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe", "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"].filter(Boolean).find(existsSync);
assert.ok(browserPath, "Chromium browser is required");
const modulePath = "/src/__fx-thumbnail-fixture.ts";
const moduleId = join(root, modulePath.slice(1)).replaceAll("\\", "/");
const fixture = `import {builtinFxEntries} from '/src/fxCatalogModel.ts';
import {renderFxThumbnail} from '/src/fxThumbnail.ts';
const results=builtinFxEntries.map(entry=>{const first=renderFxThumbnail(entry.source);return {id:entry.id,rendered:first.rendered,reason:first.reason,cacheSame:first===renderFxThumbnail(entry.source),crlfCacheSame:first===renderFxThumbnail(entry.source.replaceAll('\\n','\\r\\n')),png:first.url?.startsWith('data:image/png;base64,')};});
const complex=renderFxThumbnail('/*{"INPUTS":[]}*/ void main(){for(int i=0;i<100;i++){} gl_FragColor=vec4(1.0);}');
window.__fxThumbnailResult={results,complex};document.body.textContent=JSON.stringify(window.__fxThumbnailResult);`;
const plugin = {
  name: "fx-thumbnail-fixture", enforce: "pre",
  resolveId(id) { if (id === modulePath) return moduleId; },
  load(id) { if (id === moduleId) return fixture; },
  configureServer(server) { server.middlewares.use((request, response, next) => {
    if (request.url !== "/__fx-thumbnail") return next();
    response.setHeader("Content-Type", "text/html");
    response.end(`<!doctype html><title>FX thumbnail fixture</title><script type="module" src="${modulePath}"></script>`);
  }); },
};
const delay = (ms) => new Promise((done) => setTimeout(done, ms));
async function until(check) { const limit = Date.now() + 30000; let last; while (Date.now() < limit) { try { const value = await check(); if (value) return value; } catch (error) { last = error; } await delay(75); } throw new Error(`Fixture timed out: ${last?.message ?? "no result"}`); }
let server, browser, profile, socket;
const pending = new Map();
let id = 0;
const errors = [];
async function send(method, params = {}) {
  return new Promise((done, reject) => { const callId = ++id; const timer = setTimeout(() => { pending.delete(callId); reject(new Error(`CDP timeout: ${method}`)); }, 15000); pending.set(callId, { done, reject, timer }); socket.send(JSON.stringify({ id: callId, method, params })); });
}
async function evaluate(expression) { const value = await send("Runtime.evaluate", { expression, returnByValue: true }); assert.ok(!value.exceptionDetails, JSON.stringify(value.exceptionDetails)); return value.result.value; }
try {
  server = await createServer({ root, server: { host: "127.0.0.1", port: 0, strictPort: false }, plugins: [plugin], clearScreen: false });
  await server.listen();
  profile = await mkdtemp(join(tmpdir(), "syndocal-fx-thumbnail-"));
  browser = spawn(browserPath, ["--headless=new", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank"], { windowsHide: true, stdio: "ignore" });
  const port = await until(async () => Number((await readFile(join(profile, "DevToolsActivePort"), "utf8")).split("\n")[0]));
  const page = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((entry) => entry.type === "page");
  socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((done, reject) => { socket.addEventListener("open", done, { once: true }); socket.addEventListener("error", reject, { once: true }); });
  socket.addEventListener("message", ({ data }) => { const value = JSON.parse(data); if (value.method === "Runtime.exceptionThrown") errors.push(value.params.exceptionDetails); const call = pending.get(value.id); if (!call) return; clearTimeout(call.timer); pending.delete(value.id); if (value.error) call.reject(new Error(JSON.stringify(value.error))); else call.done(value.result); });
  await send("Runtime.enable"); await send("Page.enable");
  await send("Page.navigate", { url: `${server.resolvedUrls.local[0]}__fx-thumbnail` });
  const result = await until(() => evaluate("window.__fxThumbnailResult"));
  assert.equal(await evaluate("document.title"), "FX thumbnail fixture");
  assert.equal(await evaluate("Boolean(document.querySelector('vite-error-overlay'))"), false);
  assert.equal(result.results.length, 14);
  for (const entry of result.results) { assert.equal(entry.rendered, true, `${entry.id}: ${entry.reason}`); assert.equal(entry.png, true); assert.equal(entry.cacheSame, true); assert.equal(entry.crlfCacheSame, true); }
  assert.equal(result.complex.rendered, false, "complex imported shader must fall back without compiling");
  assert.equal(errors.length, 0, JSON.stringify(errors));
  console.log("FX thumbnail browser: all 14 canonical shaders rendered to PNG, source cache reused, complex shader safely fell back; no framework overlay or runtime exceptions.");
} finally {
  socket?.close();
  for (const call of pending.values()) clearTimeout(call.timer);
  if (browser && browser.exitCode === null) { const exited = new Promise((done) => browser.once("exit", done)); browser.kill(); await Promise.race([exited, delay(3000)]); }
  await server?.close();
  if (profile) await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }).catch(() => {});
}

// Autopsy a possibly-frozen harness Chromium WITHOUT stealing the harness's
// page WebSocket (page WS connections are exclusive; attaching one kills the
// existing client). Connects to the BROWSER-level WS and uses
// Target.attachToTarget flat sessions instead.
//
// Verdicts it can distinguish (from the 2026-07-16 renderer-freeze incident):
// - eval 1+1 responds            -> main thread alive (harness-side wait bug)
// - eval times out, dialog found -> native JS dialog blocked the main thread
// - eval times out, "No dialog"  -> renderer main thread hard-blocked
//   (pair with a CPU-delta sample: high CPU = layout/JS storm, ~zero CPU =
//   lock/IPC wait; also check thread suspension via PowerShell WaitReason)
//
// Usage: node qa/harnesses/cdp-autopsy.mjs [cdpPort=9227]
const port = Number(process.argv[2] ?? 9227);
const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
const ws = new WebSocket(version.webSocketDebuggerUrl);
let messageId = 0;
const pending = new Map();
function send(method, params = {}, sessionId) {
  const id = ++messageId;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
    setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        reject(new Error(`timeout: ${method}`));
      }
    }, 5000);
  });
}
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = () => reject(new Error("browser-level WS failed to open"));
});
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.id && pending.has(message.id)) {
    const { resolve, reject } = pending.get(message.id);
    pending.delete(message.id);
    message.error ? reject(new Error(message.error.message)) : resolve(message.result);
  }
};
console.log("browser WS connected:", version.Browser);
const targets = await send("Target.getTargets");
const pages = targets.targetInfos.filter((t) => t.type === "page");
console.log("pages:", JSON.stringify(pages.map((p) => p.url)));
const target = pages.find((p) => p.url.includes("syndocal") || p.url.includes("127.0.0.1")) ?? pages[0];
if (!target) {
  console.log("NO PAGE TARGET");
  process.exit(1);
}
const { sessionId } = await send("Target.attachToTarget", { targetId: target.targetId, flatten: true });
const evalIn = async (expr, label) => {
  try {
    const r = await send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true }, sessionId);
    console.log(`${label}:`, JSON.stringify(r.result?.value).slice(0, 400));
    return true;
  } catch (error) {
    console.log(`${label}: ERROR ${error.message}`);
    return false;
  }
};
const alive = await evalIn("1+1", "sync eval");
if (!alive) {
  // Page.handleJavaScriptDialog is handled browser-side, so it works even
  // with a blocked renderer; "No dialog is showing" rules the dialog out.
  try {
    await send("Page.handleJavaScriptDialog", { accept: false }, sessionId);
    console.log("DIALOG WAS OPEN -> dismissed; re-probing");
    await evalIn("1+1", "post-dismiss eval");
  } catch (error) {
    console.log("dialog probe:", error.message);
    console.log("VERDICT: renderer main thread hard-blocked (no dialog). Sample CPU deltas and thread WaitReasons next.");
  }
} else {
  await evalIn(
    "new Promise(r => { const t = setTimeout(() => r('NO-RAF-500ms'), 500); requestAnimationFrame(() => { clearTimeout(t); r('raf-ok'); }); })",
    "rAF health",
  );
}
ws.close();

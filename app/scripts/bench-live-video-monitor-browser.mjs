// Browser plugin not available. Focused Chromium decode/draw microbenchmark;
// real Tauri IPC/render/output measurements are separate native QA evidence.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
const executable = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe", "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"].filter(Boolean).find(existsSync);
assert(executable, "Chromium is required");
const server = createServer(); await new Promise(done => server.listen(0, "127.0.0.1", done));
const port = server.address().port; await new Promise(done => server.close(done));
const profile = await mkdtemp(join(tmpdir(), "syndocal-task17-decode-"));
const browser = spawn(executable, ["--headless=new", "--no-first-run", "--no-default-browser-check", `--user-data-dir=${profile}`, `--remote-debugging-port=${port}`, "about:blank"], { windowsHide: true, stdio: "ignore" });
let socket;
try {
  let tab;
  for (let attempt = 0; attempt < 200; attempt++) {
    try { tab = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find(item => item.type === "page"); if (tab) break; } catch {}
    await new Promise(done => setTimeout(done, 50));
  }
  assert(tab, "benchmark browser started");
  socket = new WebSocket(tab.webSocketDebuggerUrl);
  await new Promise((done, reject) => { socket.addEventListener("open", done, { once: true }); socket.addEventListener("error", reject, { once: true }); });
  const pending = new Map(); let id = 0;
  socket.addEventListener("message", event => { const message = JSON.parse(event.data); if (message.id) { pending.get(message.id)?.(message); pending.delete(message.id); } });
  const call = (method, params) => new Promise((done, reject) => { const requestId = ++id; pending.set(requestId, message => message.error ? reject(message.error) : done(message.result)); socket.send(JSON.stringify({ id: requestId, method, params })); });
  const expression = `(async () => {
    const width = 320, height = 180;
    const source = document.createElement('canvas'); source.width = width; source.height = height;
    const context = source.getContext('2d', {alpha:false});
    const raw = context.createImageData(width,height); let seed=17;
    for(let i=0;i<raw.data.length;i+=4){seed=(Math.imul(seed,1664525)+1013904223)>>>0;raw.data[i]=seed&255;raw.data[i+1]=(seed>>>8)&255;raw.data[i+2]=(seed>>>16)&255;raw.data[i+3]=255;}
    context.putImageData(raw,0,0);
    const blob = await new Promise(done => source.toBlob(done,'image/jpeg',0.68));
    const destination=document.createElement('canvas');destination.width=width;destination.height=height;document.body.append(destination);
    const draw=destination.getContext('2d',{alpha:false});
    const measures=[];
    for(let round=0;round<3;round++)for(const mode of ['jpeg-img','jpeg-bitmap-canvas','rgba-canvas']){
      const durations=[];const started=performance.now();
      for(let iteration=0;iteration<350;iteration++){
        const start=performance.now();
        if(mode==='rgba-canvas')draw.putImageData(new ImageData(raw.data,width,height),0,0);
        else if(mode==='jpeg-bitmap-canvas'){const bitmap=await createImageBitmap(blob);draw.drawImage(bitmap,0,0);bitmap.close();}
        else {const url=URL.createObjectURL(blob);const img=new Image();await new Promise((done,reject)=>{img.onload=done;img.onerror=reject;img.src=url;});draw.drawImage(img,0,0);URL.revokeObjectURL(url);}
        durations.push(performance.now()-start);
      }
      durations.sort((a,b)=>a-b);
      measures.push({round,mode,meanMs:(performance.now()-started)/durations.length,p50Ms:durations[Math.floor(durations.length*.50)],p95Ms:durations[Math.floor(durations.length*.95)],p99Ms:durations[Math.floor(durations.length*.99)],frames:durations.length});
    }
    return {userAgent:navigator.userAgent,width,height,jpegBytes:blob.size,rgbaBytes:raw.data.byteLength,measures};
  })()`;
  const result = await call("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  assert(!result.exceptionDetails, JSON.stringify(result.exceptionDetails));
  const evidence = resolve(process.env.SYNDOCAL_MONITOR_BENCH_RESULT ?? "../target/qa/task17/browser-decode-benchmark.json");
  await mkdir(join(evidence, ".."), { recursive: true }); await writeFile(evidence, JSON.stringify(result.result.value, null, 2) + "\n");
  console.log(JSON.stringify(result.result.value, null, 2));
} finally {
  socket?.close(); browser.kill();
  await new Promise(done => browser.once("exit", done));
  await rm(profile, { recursive: true, force: true, maxRetries: 20, retryDelay: 100 });
}

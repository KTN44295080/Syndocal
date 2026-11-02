// Browser plugin not available. Render the actual Solid controls in an owned,
// headless Chromium profile. No native IPC, device or external URL is opened.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { homedir, tmpdir } from "node:os";
import { dirname, resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer as tcp } from "node:net";
import { createServer } from "vite";

const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require("playwright")); } catch {
  ({ chromium } = require(resolve(homedir(), ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright")));
}
const app = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const evidence = resolve(process.env.SYNDOCAL_NDI_QA_DIR ?? join(tmpdir(), "syndocal-ndi-attribution"));
const fixturePath = "/src/__ndi-attribution.tsx";
const fixtureId = join(app, fixturePath.slice(1)).replaceAll("\\", "/");
const fixture = `
import { createSignal } from 'solid-js';
import { render } from 'solid-js/web';
import { VideoNdiSourcePicker } from '/src/components/VideoNdiSourcePicker';
import { VideoOutputCreatePanel } from '/src/components/VideoOutputCreatePanel';
import { VideoOutputConfigPanel } from '/src/components/VideoOutputConfigPanel';
import { WorkspaceChrome } from '/src/components/WorkspaceChrome';
import { installUiLocalization } from '/src/uiLocalization';
import '/src/theme.css'; import '/src/styles.css';
const locale=new URLSearchParams(location.search).get('locale')??'en';
const noop=()=>{}, done=async()=>{};
const [kind,setKind]=createSignal('NdiSender');
const [draft,setDraft]=createSignal({kind:'NdiSender',label:'Program',width:1920,height:1080,fade_ms:0,monitor_id:0,fullscreen:false,endpoint_name:'Syndocal Program'});
const props={workspaceTab:'setup',setupSubTab:'video',controlMode:'live',touchControlDomain:'lighting',
 blackout:false,videoBlackout:false,lightingMaster:1,videoMaster:1,bpm:120,clock:{bpm:120,source:'Manual',external_sync_locked:false,beat_counter:0},
 liveAudioInputRunning:false,liveAudioInputStale:false,liveAudioInputSafetyClearPending:false,liveAudioInputTelemetryFresh:false,liveAudioInputRms:0,liveAudioInputPeak:0,
 tickMs:25,jitterUs:0,packetBytes:0,dmxSuccessCount:0,dmxOutputCount:0,projectLabel:'Untitled.sdc',projectDirty:false,currentProjectPath:null,
 recentProjectPaths:[],recoveryCheckpoint:null,projectBackups:[],historyStatus:{can_undo:false,can_redo:false,undo_depth:0,redo_depth:0},uiScale:100,uiLocale:locale,uiTheme:'tally',
 operatorLockMode:null,controlLearnMode:null,controlLearnBusy:false,controlLearnTargetLabel:null,
 showClockStatus:{running:false,state:'STOPPED',output_armed:false},showClockError:null,
 onWorkspaceTab:noop,onSetupSubTab:noop,onControlMode:noop,onUiTheme:noop,onUiLocale:noop,onUiScale:noop,
 onControlDomain:noop,onOpenVideoLiveControls:noop,onCheckForUpdates:noop,onInstallUpdate:noop};
const invoke=async(command)=>command==='list_ndi_sources_v1'?{state:'available',sources:[{name:'Test camera',endpoint_name:'Test (Camera)'}],message:null}:[];
render(()=><main class="app"><WorkspaceChrome {...props}/><div style={{padding:'16px',display:'grid',gap:'16px','max-width':'100%'}}>
 <section class="panel" data-proof-input><VideoNdiSourcePicker path="" onSetPath={noop} invokeCommand={invoke}/></section>
 <section class="panel" data-proof-create><VideoOutputCreatePanel kind={kind()} onKind={setKind} label="Program" width={1920} height={1080} fadeMs={0} monitorId={0} fullscreen={false} endpoint="Syndocal Program"
 syphonAvailable={false} spoutAvailable={true} ndiAvailable={true} compositions={[{id:1,label:'Program',layer_ids:[],output_ids:[]}]} invokeCommand={invoke}
 onLabel={noop} onWidth={noop} onHeight={noop} onFadeMs={noop} onMonitorId={noop} onFullscreen={noop} onEndpoint={noop} onAddDisplayOutput={done} onAddExternalOutput={done} onEnableShowSpoutOutputs={done} onResetShowSpoutOutputs={done}/></section>
 <section class="panel" data-proof-config><VideoOutputConfigPanel output={{id:1,label:'Program',kind:'NdiSender'}} draft={draft()} ndiAvailable={true} spoutAvailable={true}
 onDraft={(_,patch)=>setDraft({...draft(),...patch})} onApply={done}/></section>
 </div></main>,document.getElementById('root'));
installUiLocalization(document.body,()=>locale);
window.__proof={setKind};
`;
let server, browser, profile;
const cases = [], errors = [];
try {
  await mkdir(evidence, { recursive: true });
  const reservation = tcp();
  await new Promise((ok, fail) => { reservation.once("error", fail); reservation.listen(0, "127.0.0.1", ok); });
  const port = reservation.address().port;
  await new Promise(ok => reservation.close(ok));
  server = await createServer({ root: app, cacheDir: join(evidence, "vite-cache"), logLevel: "error", server: { host: "127.0.0.1", port, strictPort: true }, plugins: [{
    name: "ndi-attribution-proof", enforce: "pre",
    resolveId(id) { if (id === fixturePath) return fixtureId; }, load(id) { if (id === fixtureId) return fixture; },
    configureServer(instance) { instance.middlewares.use((req, res, next) => {
      if (!req.url?.startsWith("/__ndi-attribution")) return next();
      res.setHeader("Content-Type", "text/html");
      res.end('<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="icon" href="data:,"><title>NDI attribution proof</title><div id="root"></div><script type="module" src="'+fixturePath+'"></script>');
    }); },
  }] });
  await server.listen();
  profile = await mkdtemp(join(evidence, "chromium-"));
  browser = await chromium.launchPersistentContext(profile, { channel: "chrome", headless: true });
  const page = await browser.newPage();
  page.on("pageerror", error => errors.push(String(error)));
  page.on("console", message => { if (["error", "warning"].includes(message.type())) errors.push(message.text()); });
  for (const width of [1920, 960]) for (const locale of ["en", "ja"]) {
    const url = `http://127.0.0.1:${port}/__ndi-attribution?locale=${locale}`;
    await page.setViewportSize({ width, height: 1080 });
    await page.goto(url);
    assert.equal(await page.title(), "NDI attribution proof");
    assert.equal(page.url(), url);
    await page.locator("[data-proof-input] [data-ndi-trademark-notice]").waitFor();
    assert.equal(await page.locator("vite-error-overlay").count(), 0);
    for (const scope of ["[data-proof-input]", "[data-proof-create]", "[data-proof-config]"]) {
      const legal = page.locator(scope).locator("[data-ndi-trademark-notice], [data-ndi-output-attribution]").filter({ visible: true });
      assert.ok(await legal.count() > 0, scope);
      assert.ok((await legal.first().innerText()).includes("NDI® is a registered trademark of Vizrt NDI AB."));
      assert.equal(await legal.first().locator('[data-ndi-official-link="site"]').innerText(), "https://ndi.video/");
      assert.equal(await legal.first().locator('[data-ndi-official-link="tools"]').innerText(), "https://ndi.video/tools/");
      assert.equal(await legal.first().evaluate(el => el.scrollWidth > el.clientWidth + 1), false, scope+" legal overflow");
      if (locale === "ja") assert.ok((await legal.first().innerText()).includes("登録商標"));
    }
    await page.screenshot({ path: join(evidence, `panels-${locale}-${width}.png`), fullPage: true });
    // Changing the live output type updates the real panel without a dialog.
    await page.evaluate(() => window.__proof.setKind("SpoutSender"));
    assert.equal(await page.locator("[data-proof-create] [data-ndi-output-attribution]:visible").count(), 0);
    await page.evaluate(() => window.__proof.setKind("NdiSender"));
    await page.locator("[data-proof-create] [data-ndi-output-attribution]:visible").first().waitFor();
    await page.locator(".appMenuButton").click();
    await page.locator("[data-about-legal-disclosure] summary").click();
    const about = page.locator("[data-about-legal-disclosure]");
    await about.locator("[data-ndi-official-link]").last().scrollIntoViewIfNeeded();
    assert.equal(await about.locator('[data-ndi-official-link="tools"]').innerText(), "https://ndi.video/tools/");
    assert.ok((await about.innerText()).includes("NDI® is a registered trademark of Vizrt NDI AB."));
    assert.equal(await about.evaluate(el => el.scrollWidth > el.clientWidth + 1), false, "About legal overflow");
    assert.equal(await about.evaluate(el => [...el.querySelectorAll("span, small")].some(node => node.scrollWidth > node.clientWidth + 1)), false, "About legal notice truncated");
    await page.screenshot({ path: join(evidence, `about-${locale}-${width}.png`), fullPage: false });
    cases.push({ width, height: 1080, locale, url, status: "pass" });
  }
  assert.deepEqual(errors, [], "console and runtime errors");
  console.log(JSON.stringify({ status: "pass", cases, errors, boundary: "real components; fixture IPC; no native or physical proof" }));
} finally {
  await browser?.close();
  await server?.close();
  if (profile) await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  await writeFile(join(evidence, "result.json"), JSON.stringify({ cases, errors, cleanup: "owned Chromium and Vite closed; profile removed" }, null, 2));
}

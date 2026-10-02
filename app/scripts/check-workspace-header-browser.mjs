import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdir, readFile, unlink, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { createServer as createTcpServer } from "node:net";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

// Browser plugin is not available. Use the existing Playwright runtime and
// actual Solid components; this fixture never opens a device or native output.
const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require("playwright")); } catch {
  ({ chromium } = require(resolve(homedir(), ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright")));
}
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const artifactDir = resolve(process.env.SYNDOCAL_HEADER_ARTIFACT_DIR ?? resolve(root, "../target/qa/header-20261002"));
const fixtureName = `.header-proof-${process.pid}.tsx`;
const baselineName = `.header-baseline-${process.pid}.tsx`;
const fixturePath = resolve(root, fixtureName);
const baselinePath = resolve(root, "src/components", baselineName);
let server, browser;
await mkdir(artifactDir, { recursive: true });
const baselineRef = process.env.SYNDOCAL_HEADER_BASELINE_REF;
try {
  const baseline = baselineRef
    ? execFileSync("git", ["show", `${baselineRef}:app/src/components/WorkspaceChrome.tsx`], { cwd: root, encoding: "utf8" })
    : await readFile(resolve(root, "src/components/WorkspaceChrome.tsx"), "utf8");
  await writeFile(baselinePath, baseline);
  await writeFile(fixturePath, `
import {createSignal} from 'solid-js'; import {render} from 'solid-js/web';
import {WorkspaceChrome} from '/src/components/WorkspaceChrome';
import {WorkspaceChrome as BaselineChrome} from '/src/components/${baselineName}';
import {WorkspaceOperationsMenu} from '/src/components/WorkspaceOperationsMenu';
import '/src/styles.css';
const [workspace,setWorkspace]=createSignal('touch'),[bpm,setBpm]=createSignal(120),[learn,setLearn]=createSignal(null);
const [dmx,setDmx]=createSignal(false),[video,setVideo]=createSignal(false),[safety,setSafety]=createSignal(false);
const [dirty,setDirty]=createSignal(false),[lock,setLock]=createSignal(null),[label,setLabel]=createSignal('Untitled.sdc');
window.proof={calls:[],setWorkspace,setSafety,setDirty,setLock,setLabel,setBpm,get bpm(){return bpm()}};
const call=name=>()=>window.proof.calls.push(name), noop=()=>{}, done=async()=>{};
const operations=()=> <WorkspaceOperationsMenu profiles={[]} selectedProfileId={null} poppedPanes={[]}
 paneTransitions={{}} workspaceBusy={false} operatorPolicy={null} operatorLockMode={lock()}
 onSelectProfile={noop} onSaveProfile={done} onApplyProfile={done} onDeleteProfile={noop}
 onTogglePane={noop} onConfigurePolicy={async()=>true} onClearPolicy={async()=>true}
 onLock={setLock} onUnlock={async()=>true}/>;
const props={get workspaceTab(){return workspace()},setupSubTab:'output',controlMode:'live',
 get blackout(){return dmx()},get videoBlackout(){return video()},get safetyBlackoutEngaged(){return safety()},
 onReleaseSafetyBlackout:()=>setSafety(false),lightingMaster:1,videoMaster:1,get bpm(){return bpm()},
 liveAudioInputRunning:false,liveAudioInputStale:false,liveAudioInputSafetyClearPending:false,
 liveAudioInputTelemetryFresh:false,liveAudioInputRms:0,liveAudioInputPeak:0,tickMs:25,jitterUs:0,
 packetBytes:0,dmxSuccessCount:0,dmxOutputCount:0,get projectLabel(){return label()},get projectDirty(){return dirty()},
 currentProjectPath:null,recentProjectPaths:[],recoveryCheckpoint:null,projectBackups:[],daslightProjectImportBusy:false,
 historyStatus:{can_undo:false,can_redo:false,undo_depth:0,redo_depth:0},uiScale:100,uiLocale:'en',
 canBack:true,canGo:true,canPauseFade:true,fadePaused:false,canToggleTimeline:true,timelinePlaying:false,anyFixtureFlags:true,
 nextCueLabel:'Next cue',get operatorLockMode(){return lock()},get controlLearnMode(){return learn()},
 controlLearnBusy:false,controlLearnTargetLabel:null,showClockStatus:{running:false,state:'STOPPED',output_armed:false},showClockError:null,
 get operations(){return operations()},onWorkspaceTab:setWorkspace,onSetupSubTab:noop,onControlMode:noop,
 onBack:call('back'),onGo:call('go'),onToggleFade:call('fade'),onToggleTimeline:call('timeline'),
 onSetBlackout:setDmx,onSetVideoBlackout:setVideo,onSetAllBlackout:value=>{setDmx(value);setVideo(value)},
 onClearFixtureFlags:call('clear-flags'),onControlLearnMode:setLearn,onLightingMaster:noop,onVideoMaster:noop,
 onTapBpm:call('tap'),onSetBpm:async value=>setBpm(value),onOpenLiveAudioInputSettings:call('audio-settings'),
 onNewProject:noop,onSaveUserTemplate:noop,onLoadUserTemplate:noop,onSaveProject:call('save'),onSaveProjectAs:noop,
 onLoadProject:noop,onImportDaslightProject:noop,onLoadRecentProject:noop,onClearRecentProjects:noop,
 onLoadRecovery:noop,onDiscardRecovery:noop,onLoadBackup:noop,onDeleteBackup:noop,onExportDiagnostics:noop,
 onCheckForUpdates:noop,onInstallUpdate:noop,onUndo:noop,onRedo:noop,onUiLocale:noop,onUiScale:noop,
 onResetWorkspaceLayout:noop,onLoadSample:noop,onRunSmoke:noop};
const Chrome=new URL(location.href).searchParams.has('baseline')?BaselineChrome:WorkspaceChrome;
render(()=><main class="app"><Chrome {...props}/><section>Control header proof</section></main>,document.body);
`);
  const reservation = createTcpServer();
  await new Promise((resolve, reject) => { reservation.once("error",reject); reservation.listen(0,"127.0.0.1",resolve); });
  const port = reservation.address().port;
  await new Promise((resolve, reject) => reservation.close(error=>error?reject(error):resolve()));
  // Vite 6 embeds port 0 into its client when configured with an ephemeral
  // port. Resolve a real port first and fail closed if it becomes occupied.
  server = await createServer({ root, server: { host: "127.0.0.1", port, strictPort: true }, logLevel: "error",
    plugins:[{name:'header-proof',configureServer(server){server.middlewares.use((request,response,next)=> {
      if (!request.url?.startsWith('/header-proof')) return next();
      response.setHeader('Content-Type','text/html');
      response.end(`<html><head><link rel="icon" href="data:,"></head><body><script type="module" src="/${fixtureName}"></script></body></html>`);
    });}}] });
  await server.listen();
  const origin = `http://127.0.0.1:${server.httpServer.address().port}/`;
  const executablePath = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"].filter(Boolean).find(existsSync);
  browser = await chromium.launch({ headless: true, ...(executablePath ? { executablePath } : {}) });
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", error => errors.push(String(error)));
  page.on("console", message => { if (message.type() === "error") errors.push(message.text()); });
  const measure = () => page.evaluate(() => {
    const header = document.querySelector('.topbar'), nav = document.querySelector('.topbarLeft');
    const rect = el => { const r=el.getBoundingClientRect(); return {x:r.x,y:r.y,w:r.width,h:r.height,right:r.right,bottom:r.bottom}; };
    const bounds=rect(header);
    const elements=[...header.querySelectorAll('button, .topbarTools > summary, .topbarProject > strong, .topbarProject > span, .topbarStateCluster > span')]
      .filter(el=>el.checkVisibility({checkVisibilityCSS:true})&&el.getBoundingClientRect().width>0);
    const items=elements.map(el=>({label:el.getAttribute('aria-label')||el.textContent.trim(),...rect(el)}));
    const overlaps=[];
    items.forEach((a,i)=>items.slice(i+1).forEach(b=>{
      if(Math.min(a.right,b.right)-Math.max(a.x,b.x)>1&&Math.min(a.bottom,b.bottom)-Math.max(a.y,b.y)>1) overlaps.push([a.label,b.label]);
    }));
    return {bounds,items,overlaps,contained:items.every(r=>r.x>=bounds.x-1&&r.right<=bounds.right+1&&r.y>=bounds.y-1&&r.bottom<=bounds.bottom+1),
      horizontalOverflow:header.scrollWidth>header.clientWidth+1,nav:rect(nav),status:rect(header.querySelector('.status')),
      controlSelected:header.querySelector('[data-workspace-option="touch"]').getAttribute('aria-pressed')==='true'};
  });
  const hash = contents => createHash('sha256').update(contents).digest('hex');
  const componentHashes = {};
  for (const name of ['WorkspaceChrome.tsx','WorkspaceChrome.css','TopbarTools.tsx']) {
    componentHashes[name] = hash(await readFile(resolve(root,'src/components',name)));
  }
  const evidence = { timestamp:new Date().toISOString(), runnerSha256:hash(await readFile(fileURLToPath(import.meta.url))),
    componentHashes,browserVersion:browser.version(),baselineRef: baselineRef ?? null,
    browserPlugin: "unavailable", softwareOnly: true, viewports: [] };
  if (baselineRef) {
    await page.setViewportSize({width:1280,height:776});
    await page.goto(`${origin}header-proof?baseline=1`);
    await page.locator('.topbar').waitFor();
    evidence.before = await measure();
    assert.ok(evidence.before.overlaps.length>0 || evidence.before.horizontalOverflow || !evidence.before.contained,
      'baseline must reproduce the reported overlap or loss of containment');
    await page.locator('.workspaceChrome').screenshot({path:resolve(artifactDir,'header-before-1280.png')});
  }
  for (const width of [960,1100,1280,1920,2560,3840]) {
    await page.setViewportSize({width,height:776});
    await page.goto(`${origin}header-proof`);
    await page.locator('.topbar').waitFor();
    const geometry = await measure();
    assert.deepEqual(geometry.overlaps, [], `${width}px overlapping header items`);
    assert.equal(geometry.contained, true, `${width}px containment: ${JSON.stringify(geometry)}`);
    assert.equal(geometry.horizontalOverflow, false, `${width}px horizontal overflow`);
    assert.equal(geometry.controlSelected, true, 'actual Control route is touch');
    assert.ok(Math.abs((geometry.status.y+geometry.status.h/2)-(geometry.nav.y+geometry.nav.h/2))<=1&&geometry.bounds.h===42, 'header remains one 42px row');
    await page.evaluate(()=>{window.proof.setSafety(true);window.proof.setDirty(true);window.proof.setLock('Partial');window.proof.setLabel('長いプロジェクト名'.repeat(12)+'.sdc *')});
    const expanded=await measure();
    assert.deepEqual(expanded.overlaps, [], `${width}px safe/dirty/locked header overlap`);
    assert.equal(expanded.contained,true,`${width}px safe/dirty/locked containment`);
    assert.equal(expanded.horizontalOverflow,false,`${width}px safe/dirty/locked overflow`);
    const go=expanded.items.find(i=>i.label==='GO');
    const tap=expanded.items.find(i=>i.label==='Tap');
    assert.ok(go.w>=48&&go.h>=40&&tap.w>=40&&tap.h>=40, 'existing hit sizes preserved');
    await page.evaluate(()=>{window.proof.setSafety(false);window.proof.setDirty(false);window.proof.setLock(null);window.proof.setLabel('Untitled.sdc')});
    await page.locator('.workspaceChrome').screenshot({path:resolve(artifactDir,`header-after-${width}.png`)});
    await page.locator('.topbarTools > summary').focus();
    await page.locator('.topbarTools > summary').press('Enter');
    const tools = await page.locator('.topbarToolsPopover').evaluate(panel=>{
      const rect=panel.getBoundingClientRect();
      const buttons=[...panel.querySelectorAll('button')].map(button=>{
          const r=button.getBoundingClientRect();
          const hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);
          return {label:button.getAttribute('aria-label'),hit:hit?.outerHTML.slice(0,150),reachable:r.width>0&&r.height>0&&button.contains(hit)};
        });
      return {contained:rect.left>=0&&rect.right<=innerWidth&&rect.top>=0&&rect.bottom<=innerHeight,
        buttonsReachable:buttons.every(button=>button.reachable),buttons};
    });
    assert.equal(tools.contained,true,`${width}px Tools popup containment`);
    assert.equal(tools.buttonsReachable,true,`${width}px Tools hit testing: ${JSON.stringify(tools.buttons)}`);
    if(width===1280) await page.screenshot({path:resolve(artifactDir,'header-tools-1280.png')});
    await page.locator('.topbarTools > summary').press('Escape');
    assert.equal(await page.locator('.topbarTools').getAttribute('open'),null);
    evidence.viewports.push({width,geometry,expanded,tools});
  }
  for (const action of ['back','go']) await page.locator(`[data-global-operator-action="${action}"]`).click();
  await page.locator('.topbarTools > summary').click();
  for (const action of ['fade','timeline','clear-flags']) await page.locator(`[data-global-operator-action="${action}"]`).click();
  await page.locator('.topbarTools > summary').click();
  await page.locator('[data-topbar-tap]').click();
  assert.deepEqual(await page.evaluate(()=>window.proof.calls), ['back','go','fade','timeline','clear-flags','tap']);
  await page.locator('[data-global-operator-action="dmx-blackout"]').click();
  assert.equal(await page.locator('[data-global-operator-action="dmx-blackout"]').getAttribute('aria-pressed'),'true');
  await page.locator('[data-global-operator-action="all-blackout"]').click();
  assert.equal(await page.locator('[data-global-operator-action="all-blackout"]').getAttribute('aria-pressed'),'true');
  await page.locator('[data-global-operator-action="all-blackout"]').click();
  await page.locator('.topbarTools > summary').click();
  for(const mode of ['midi','osc','dmx']) {
    await page.locator(`[data-control-learn-toggle="${mode}"]`).click();
    assert.equal(await page.locator(`[data-control-learn-toggle="${mode}"]`).getAttribute('aria-pressed'),'true');
    assert.ok(await page.locator('.controlLearnPrompt').evaluate(el=>el.getBoundingClientRect().top>=document.querySelector('.topbar').getBoundingClientRect().bottom), 'learn prompt follows header');
    await page.locator(`[data-control-learn-toggle="${mode}"]`).click();
  }
  await page.locator('[data-topbar-pulse]').click();
  assert.equal(await page.evaluate(()=>window.proof.calls.at(-1)),'audio-settings');
  await page.locator('[data-topbar-tap]').click();
  assert.equal(await page.locator('.topbarTools').getAttribute('open'),null, 'outside pointer dismisses Tools');
  await page.locator('.topbarTools > summary').focus();
  await page.locator('.topbarTools > summary').press('Enter');
  await page.locator('.topbarTools > summary').press('Escape');
  assert.equal(await page.locator('.topbarTools').getAttribute('open'),null);
  assert.equal(await page.locator('.topbarTools > summary').evaluate(el=>document.activeElement===el),true);
  await page.evaluate(()=>window.proof.setBpm(80));
  assert.equal(await page.getByRole('button',{name:'Edit BPM'}).textContent(),'80', 'passive tapped tempo changes update the readout without entering edit mode');
  await page.getByRole('button',{name:'Edit BPM'}).click();
  await page.getByRole('spinbutton',{name:'BPM'}).fill('137.5');
  await page.getByRole('spinbutton',{name:'BPM'}).press('Enter');
  assert.equal(await page.evaluate(()=>window.proof.bpm),137.5);
  assert.equal(await page.getByRole('button',{name:'Edit BPM'}).textContent(),'138', 'existing rounded BPM readout');
  await page.getByRole('button',{name:'Project menu',exact:true}).click();
  await page.locator('.workspaceOperationsButton').click();
  assert.equal(await page.getByRole('dialog',{name:'Workspace and operator controls'}).isVisible(),true);
  await page.locator('.workspaceOperationsButton').click();
  await page.locator('[data-project-menu-action="save"]').click();
  assert.equal(await page.evaluate(()=>window.proof.calls.at(-1)),'save');
  assert.deepEqual(errors,[]);
  evidence.interactionsPassed=true;
  evidence.errors=errors;
  await writeFile(resolve(artifactDir,'header-evidence.json'),JSON.stringify(evidence,null,2)+'\n');
  console.log(`PASS single-row Control header: six widths, safe/dirty/locked states, no overlap, retained hit targets, transport/blackout/learn/tempo/project/workspace actions and Escape focus; evidence=${artifactDir}`);
} finally {
  await browser?.close();
  await server?.close();
  await unlink(fixturePath).catch(()=>{});
  await unlink(baselinePath).catch(()=>{});
}

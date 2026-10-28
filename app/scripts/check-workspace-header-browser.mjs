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
const baselineFixtureName = `.header-proof-before-${process.pid}.tsx`;
const baselineName = `.header-baseline-${process.pid}.tsx`;
const baselineCssName = `.header-baseline-${process.pid}.css`;
const fixturePath = resolve(root, fixtureName);
const baselineFixturePath = resolve(root, baselineFixtureName);
const baselinePath = resolve(root, "src/components", baselineName);
const baselineCssPath = resolve(root, "src/components", baselineCssName);
let server, browser;
await mkdir(artifactDir, { recursive: true });
const baselineRef = process.env.SYNDOCAL_HEADER_BASELINE_REF;
try {
  const baseline = baselineRef
    ? execFileSync("git", ["show", `${baselineRef}:app/src/components/WorkspaceChrome.tsx`], { cwd: root, encoding: "utf8" })
    : await readFile(resolve(root, "src/components/WorkspaceChrome.tsx"), "utf8");
  const baselineCss = baselineRef
    ? execFileSync("git", ["show", `${baselineRef}:app/src/components/WorkspaceChrome.css`], { cwd: root, encoding: "utf8" })
    : await readFile(resolve(root, "src/components/WorkspaceChrome.css"), "utf8");
  assert.ok(baseline.includes('import "./WorkspaceChrome.css";'), 'baseline imports its owned chrome CSS');
  await writeFile(baselinePath, baseline.replace('import "./WorkspaceChrome.css";', `import "./${baselineCssName}";`));
  await writeFile(baselineCssPath, baselineCss);
  const fixture = component => `
import {createSignal} from 'solid-js'; import {render} from 'solid-js/web';
import {WorkspaceChrome as Chrome} from '${component}';
import {WorkspaceOperationsMenu} from '/src/components/WorkspaceOperationsMenu';
import {installUiLocalization} from '/src/uiLocalization';
import '/src/theme.css';import '/src/styles.css';
const [workspace,setWorkspace]=createSignal('touch'),[bpm,setBpm]=createSignal(120),[learn,setLearn]=createSignal(null);
const [dmx,setDmx]=createSignal(false),[video,setVideo]=createSignal(false),[safety,setSafety]=createSignal(false);
const [dirty,setDirty]=createSignal(false),[lock,setLock]=createSignal(null),[label,setLabel]=createSignal('Untitled.sdc');
const [locale,setLocale]=createSignal('en'),[scale,setScale]=createSignal(100);
const localization=installUiLocalization(document.body,locale);
window.proof={calls:[],setWorkspace,setSafety,setDirty,setLock,setLabel,setBpm,get bpm(){return bpm()},
 setLocale:value=>{setLocale(value);localization.refresh()},setScale:value=>{setScale(value);
 document.documentElement.style.setProperty('--ui-scale',String(value/100));
 document.documentElement.style.setProperty('--ui-scale-inverse',String(10000/value)+'%')}};
const call=name=>()=>window.proof.calls.push(name), noop=()=>{}, done=async()=>{};
const operations=()=> <WorkspaceOperationsMenu profiles={[]} selectedProfileId={null} poppedPanes={[]}
 paneTransitions={{}} workspaceBusy={false} operatorPolicy={null} operatorLockMode={lock()}
 onSelectProfile={noop} onSaveProfile={done} onApplyProfile={done} onDeleteProfile={noop}
 onTogglePane={noop} onConfigurePolicy={async()=>true} onClearPolicy={async()=>true}
 onLock={setLock} onUnlock={async()=>true}/>;
const props={get workspaceTab(){return workspace()},setupSubTab:'output',controlMode:'live',
 get blackout(){return dmx()||safety()},get videoBlackout(){return video()},lightingMaster:1,videoMaster:1,get bpm(){return bpm()},clock:{bpm:120,source:'Manual',external_sync_locked:false,beat_counter:0},
 liveAudioInputRunning:false,liveAudioInputStale:false,liveAudioInputSafetyClearPending:false,
 liveAudioInputTelemetryFresh:false,liveAudioInputRms:0,liveAudioInputPeak:0,tickMs:25,jitterUs:0,
 packetBytes:0,dmxSuccessCount:0,dmxOutputCount:0,get projectLabel(){return label()},get projectDirty(){return dirty()},
 currentProjectPath:null,recentProjectPaths:[],recoveryCheckpoint:null,projectBackups:[],daslightProjectImportBusy:false,
 historyStatus:{can_undo:false,can_redo:false,undo_depth:0,redo_depth:0},get uiScale(){return scale()},get uiLocale(){return locale()},
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
render(()=><main class="app"><Chrome {...props}/><section>Control header proof</section></main>,document.body);
`;
  await writeFile(fixturePath, fixture('/src/components/WorkspaceChrome'));
  await writeFile(baselineFixturePath, fixture(`/src/components/${baselineName}`));
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
      const entry = new URL(request.url, 'http://127.0.0.1').searchParams.has('baseline') ? baselineFixtureName : fixtureName;
      response.end(`<html><head><link rel="icon" href="data:,"></head><body><script type="module" src="/${entry}"></script></body></html>`);
    });}}] });
  await server.listen();
  const origin = `http://127.0.0.1:${server.httpServer.address().port}/`;
  const executablePath = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"].filter(Boolean).find(existsSync);
  browser = await chromium.launch({ headless: true, ...(executablePath ? { executablePath } : {}) });
  const page = await browser.newPage();
  const openTools = async () => {
    if (await page.locator('.topbarTools').getAttribute('open') === null) await page.locator('.topbarTools > summary').click();
  };
  const errors = [];
  page.on("pageerror", error => errors.push(String(error)));
  page.on("console", message => { if (message.type() === "error") errors.push(message.text()); });
  const measure = () => page.evaluate(() => {
    const header = document.querySelector('.topbar'), nav = document.querySelector('.topbarLeft');
    const rect = el => { const r=el.getBoundingClientRect(); return {x:r.x,y:r.y,w:r.width,h:r.height,right:r.right,bottom:r.bottom}; };
    const bounds=rect(header);
    const visible = el => el.checkVisibility({checkVisibilityCSS:true}) && el.getBoundingClientRect().width>0 && el.getBoundingClientRect().height>0;
    const elements=[...header.querySelectorAll('button, .topbarTools > summary, .topbarProject > strong, .topbarProject > span')].filter(el=>visible(el)&&!el.closest('.topbarToolsPopover'));
    const items=elements.map(el=>{
      const r=rect(el), interactive=el.matches('button,summary');
      const hit=document.elementFromPoint(r.x+r.w/2,r.y+r.h/2);
      return {label:el.getAttribute('aria-label')||el.textContent.trim(),...r,interactive,reachable:!interactive||el.contains(hit),fontSize:parseFloat(getComputedStyle(el).fontSize)};
    });
    const overlaps=[];
    items.forEach((a,i)=>items.slice(i+1).forEach(b=>{
      if(Math.min(a.right,b.right)-Math.max(a.x,b.x)>1&&Math.min(a.bottom,b.bottom)-Math.max(a.y,b.y)>1) overlaps.push([a.label,b.label]);
    }));
    const text = [...header.querySelectorAll('.topbarSyncSource, .topbarSyncStatus, .topbarShowClockStatus > span, .topbarShowClockStatus > strong')].map(el=>{
      const range=document.createRange();range.selectNodeContents(el);const r=range.getBoundingClientRect(),box=el.getBoundingClientRect();
      return {label:el.textContent.trim(),visible:visible(el),fits:r.width<=box.width+1&&r.height<=box.height+1&&el.scrollWidth<=el.clientWidth+1,
        textWidth:r.width,textHeight:r.height,boxWidth:box.width,boxHeight:box.height,scrollWidth:el.scrollWidth,clientWidth:el.clientWidth};
    });
    const expected=['[data-workspace-option="setup"]','[data-workspace-option="control"]','[data-workspace-option="touch"]',
      '[data-global-operator-action="back"]','[data-global-operator-action="go"]','[data-global-operator-action="dmx-blackout"]',
      '[data-global-operator-action="video-blackout"]','[data-global-operator-action="all-blackout"]','.topbarBpmValue','[data-topbar-tap]','.topbarTools > summary',
      '[data-window-control="minimize"]','[data-window-control="maximize"]','[data-window-control="close"]'];
    const tempo=[...header.querySelectorAll('[data-topbar-tap]')];
    const tempoDisclosed=Boolean(tempo[0]?.closest('.topbarToolsPopover'));
    const missing = expected.filter(selector=>{const matches=[...header.querySelectorAll(selector)].filter(visible);return matches.length!==1;});
    const sizes=Object.fromEntries(expected.map(selector=>[selector,rect(header.querySelector(selector))]));
    const source=header.querySelector('.topbarSyncSource').getBoundingClientRect(),sync=header.querySelector('.topbarSyncRow').getBoundingClientRect();
    const headerStyle=getComputedStyle(header);
    const rowBoxExtras=['paddingTop','paddingBottom','borderTopWidth','borderBottomWidth'].reduce((sum,key)=>sum+parseFloat(headerStyle[key]),0);
    const bpmValue=header.querySelector('.topbarBpmValue');
    const bpmReadable=Boolean(bpmValue)&&visible(bpmValue)&&bpmValue.scrollWidth<=bpmValue.clientWidth+1&&/^[0-9]+$/.test(bpmValue.textContent.trim());
    const projectName=header.querySelector('.topbarProject > span');
    return {bounds,items,overlaps,text,missing,sizes,tempoDisclosed,bpmReadable,projectNameVisible:Boolean(projectName)&&visible(projectName),tempoCount:tempo.length,bpmCount:header.querySelectorAll('.topbarBpmValue').length,tempoRowsSeparate:source.bottom<=sync.top+1,
      contained:items.every(r=>r.x>=bounds.x-1&&r.right<=bounds.right+1&&r.y>=bounds.y-1&&r.bottom<=bounds.bottom+1),
      horizontalOverflow:header.scrollWidth>header.clientWidth+1||bounds.right>innerWidth+1,nav:rect(nav),status:rect(header.querySelector('.status')),
      windows:rect(header.querySelector('.windowControls')),project:rect(header.querySelector('.topbarProject')),
      rows:headerStyle.gridTemplateRows.split(' ').map(parseFloat),rowGap:parseFloat(headerStyle.rowGap),rowBoxExtras,
      controlSelected:header.querySelector('[data-workspace-option="touch"]').getAttribute('aria-pressed')==='true'};
  });
  // BPM/Tap move into Tools only when the row cannot hold them at full size
  // after the project name is gone (locked header, 110% scale, 960px).
  const assertHeader = (geometry, width, state, scale = 1, allowTempoDisclosure = false) => {
    const label=`${width}px ${state}`;
    const disclosed=allowTempoDisclosure&&geometry.tempoDisclosed;
    if(disclosed) assert.equal(geometry.projectNameVisible,false,`${label}: tempo is disclosed only after the project name was dropped`);
    assert.deepEqual(geometry.missing.filter(selector=>!(disclosed&&['.topbarBpmValue','[data-topbar-tap]'].includes(selector))),[],`${label}: required controls remain visible`);
    assert.deepEqual(geometry.overlaps,[],`${label}: overlapping header items`);
    assert.equal(geometry.contained,true,`${label}: containment: ${JSON.stringify(geometry)}`);
    assert.equal(geometry.horizontalOverflow,false,`${label}: horizontal overflow`);
    assert.ok(geometry.items.every(item=>item.reachable),`${label}: all visible buttons are hit-testable`);
    if(!disclosed) {
      assert.equal(geometry.tempoDisclosed,false,`${label}: BPM and Tap share the header row`);
      assert.equal(geometry.bpmReadable,true,`${label}: current BPM is readable in the header`);
    }
    assert.equal(geometry.tempoCount,1,`${label}: one Tap control`);
    assert.equal(geometry.bpmCount,1,`${label}: one BPM control`);
    assert.ok(geometry.items.every(item=>item.fontSize>=13),`${label}: text size is preserved`);
    for(const [selector,minimumWidth] of [['[data-global-operator-action="back"]',32],['[data-global-operator-action="go"]',48],
      ['[data-global-operator-action="dmx-blackout"]',42],['[data-global-operator-action="video-blackout"]',42],['[data-global-operator-action="all-blackout"]',42],
      ['[data-topbar-tap]',40],['[data-window-control="minimize"]',40],['[data-window-control="maximize"]',40],['[data-window-control="close"]',40]]) {
      if(disclosed&&selector==='[data-topbar-tap]') continue;
      const size=geometry.sizes[selector];assert.ok(size.w>=minimumWidth&&size.h>=40,`${label}: authored button size ${selector}: ${JSON.stringify(size)}`);
    }
    if(!disclosed) assert.ok(geometry.sizes['.topbarBpmValue'].h>=28,`${label}: BPM readout keeps its 28px edit target`);
    assert.equal(geometry.rows.length,1,`${label}: header always has one row`);
    if(scale===1) {
      assert.deepEqual(geometry.rows,[40],`${label}: one full-size 40px row`);
      assert.equal(geometry.bounds.h,42,`${label}: one row plus borders`);
    } else assert.ok(geometry.rows[0]>=40,`${label}: UI scale retains the authored minimum`);
    for(const group of [geometry.status,geometry.project,geometry.windows])
      assert.ok(Math.abs((group.y+group.h/2)-(geometry.nav.y+geometry.nav.h/2))<=1,`${label}: navigation, project, operator and window controls share one row`);
    const expectedHeight=(geometry.rows.reduce((sum,height)=>sum+height,0)+geometry.rowGap*(geometry.rows.length-1)+geometry.rowBoxExtras)*scale;
    assert.ok(Math.abs(geometry.bounds.h-expectedHeight)<0.1,`${label}: rendered height matches grid tracks, gap, padding, borders and UI zoom: ${geometry.bounds.h}/${expectedHeight}`);
  };
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
    assert.ok(evidence.before.overlaps.length>0 || evidence.before.horizontalOverflow || !evidence.before.contained || evidence.before.text.some(item=>!item.visible||!item.fits),
      'baseline must reproduce the reported overlap or loss of containment');
    await page.locator('.workspaceChrome').screenshot({path:resolve(artifactDir,'header-before-1280.png')});
  }
  for (const [width,height] of [[960,640],[1100,720],[1280,720],[1366,768],[1500,900],[1501,900],[1920,1080],[2560,1440],[3840,2160]]) {
    await page.setViewportSize({width,height});
    await page.goto(`${origin}header-proof`);
    await page.locator('.topbar').waitFor();
    const geometry = await measure();
    await writeFile(resolve(artifactDir,`header-geometry-${width}.json`),JSON.stringify(geometry,null,2));
    await page.locator('.workspaceChrome').screenshot({path:resolve(artifactDir,`header-after-${width}.png`)});
    assertHeader(geometry,width,'normal');
    assert.equal(geometry.controlSelected, true, 'actual Control route is touch');
    if(width>=1366) assert.equal(geometry.projectNameVisible,true,`${width}px: the project name is only dropped when the row needs its space`);
    await page.evaluate(()=>{window.proof.setSafety(true);window.proof.setDirty(true);window.proof.setLock('Partial');window.proof.setLabel('長いプロジェクト名'.repeat(12)+'.sdc *')});
    assert.equal(await page.locator('[data-global-operator-action="safety-blackout-release"]').count(),0, "no independent SAFE control");
    assert.equal(await page.locator('[data-global-operator-action="dmx-blackout"]').getAttribute("aria-pressed"),"true", "runtime blackout uses the DMX control");
    const expanded=await measure();
    assertHeader(expanded,width,'safe/dirty/locked');
    const go=expanded.items.find(i=>i.label==='GO');
    assert.ok(go.w>=48&&go.h>=40, 'existing GO hit size preserved');
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
      const texts=[...panel.querySelectorAll('.topbarSyncSource,.topbarSyncStatus,.topbarShowClockStatus > span,.topbarShowClockStatus > strong')].map(el=>{
        const range=document.createRange();range.selectNodeContents(el);const text=range.getBoundingClientRect(),box=el.getBoundingClientRect();
        return {label:el.textContent,fits:text.width<=box.width+1&&text.height<=box.height+1&&el.scrollWidth<=el.clientWidth+1};
      });
      const tap=panel.querySelector('[data-topbar-tap]')?.getBoundingClientRect();
      return {contained:rect.left>=0&&rect.right<=innerWidth&&rect.top>=0&&rect.bottom<=innerHeight,
        buttonsReachable:buttons.every(button=>button.reachable),buttons,texts,tap:tap?{width:tap.width,height:tap.height}:null};
    });
    assert.equal(tools.contained,true,`${width}px Tools popup containment`);
    assert.equal(tools.buttonsReachable,true,`${width}px Tools hit testing: ${JSON.stringify(tools.buttons)}`);
    assert.ok(tools.texts.length===4&&tools.texts.every(text=>text.fits),`${width}px full clock/source labels in Tools`);
    assert.equal(tools.tap,null,`${width}px Tools carries no duplicate Tap`);
    if(width===1280) await page.screenshot({path:resolve(artifactDir,'header-tools-1280.png')});
    const learnStates=[];
    if(width===960||width===1280) {
      for(const mode of ['midi','osc','dmx']) {
        await openTools();
        await page.locator(`[data-control-learn-toggle="${mode}"]`).click();
        assert.equal(await page.locator('.topbarTools').getAttribute('open'),null,'Learn choice dismisses Tools to expose its targets and instruction');
        const learn=await page.evaluate(()=>{
          const prompt=document.querySelector('.controlLearnPrompt'),r=prompt.getBoundingClientRect();
          const header=document.querySelector('.topbar').getBoundingClientRect();
          const otherRows=[...prompt.parentElement.children,document.querySelector('.topbarToolsPopover')].filter(el=>el&&el!==prompt&&el.checkVisibility({checkVisibilityCSS:true})).map(el=>el.getBoundingClientRect());
          const buttons=[...document.querySelectorAll('.topbar button,.topbar summary')].filter(el=>el.checkVisibility({checkVisibilityCSS:true})).map(el=>{
            const b=el.getBoundingClientRect(),hit=document.elementFromPoint(b.x+b.width/2,b.y+b.height/2);
            return {label:el.getAttribute('aria-label')||el.textContent.trim(),reachable:b.width>0&&b.height>0&&el.contains(hit)};
          });
          return {belowHeader:r.top>=header.bottom,contained:r.left>=0&&r.right<=innerWidth&&r.bottom<=innerHeight,
            staticPosition:getComputedStyle(prompt).position==='static',
            overlaps:otherRows.some(b=>Math.min(b.right,r.right)-Math.max(b.left,r.left)>1&&Math.min(b.bottom,r.bottom)-Math.max(b.top,r.top)>1),buttons};
        });
        assert.ok(learn.belowHeader&&learn.contained&&learn.staticPosition&&!learn.overlaps&&learn.buttons.every(button=>button.reachable),`${width}px ${mode} Learn prompt and controls remain usable: ${JSON.stringify(learn)}`);
        if(mode==='midi') await page.screenshot({path:resolve(artifactDir,`header-learn-${width}.png`)});
        learnStates.push({mode,...learn});
        await openTools();
        await page.locator(`[data-control-learn-toggle="${mode}"]`).click();
      }
    }
    await page.locator('.topbarTools > summary').press('Escape');
    assert.equal(await page.locator('.topbarTools').getAttribute('open'),null);
    evidence.viewports.push({width,height,geometry,expanded,tools,learnStates});
  }
  for (const action of ['back','go']) await page.locator(`[data-global-operator-action="${action}"]`).click();
  assert.equal(await page.locator('.topbarTools').getAttribute('open'),null,'Tools is closed before Tap');
  await page.locator('[data-topbar-tap]').click();
  assert.equal(await page.locator('.topbarTools').getAttribute('open'),null,'Tap is one click without opening Tools');
  await page.locator('.topbarTools > summary').click();
  for (const action of ['fade','timeline','clear-flags']) await page.locator(`[data-global-operator-action="${action}"]`).click();
  assert.deepEqual(await page.evaluate(()=>window.proof.calls), ['back','go','tap','fade','timeline','clear-flags']);
  await page.locator('[data-global-operator-action="dmx-blackout"]').click();
  assert.equal(await page.locator('[data-global-operator-action="dmx-blackout"]').getAttribute('aria-pressed'),'true');
  await page.locator('[data-global-operator-action="all-blackout"]').click();
  assert.equal(await page.locator('[data-global-operator-action="all-blackout"]').getAttribute('aria-pressed'),'true');
  await page.locator('[data-global-operator-action="all-blackout"]').click();
  await page.locator('.topbarTools > summary').click();
  for(const mode of ['midi','osc','dmx']) {
    await openTools();
    await page.locator(`[data-control-learn-toggle="${mode}"]`).click();
    assert.equal(await page.locator(`[data-control-learn-toggle="${mode}"]`).getAttribute('aria-pressed'),'true');
    assert.ok(await page.locator('.controlLearnPrompt').evaluate(el=>el.getBoundingClientRect().top>=document.querySelector('.topbar').getBoundingClientRect().bottom), 'learn prompt follows header');
    await openTools();
    await page.locator(`[data-control-learn-toggle="${mode}"]`).click();
  }
  await openTools();
  await page.locator('[data-topbar-pulse]').click();
  assert.equal(await page.evaluate(()=>window.proof.calls.at(-1)),'audio-settings');
  await page.locator('[data-workspace-option="touch"]').click();
  assert.equal(await page.locator('.topbarTools').getAttribute('open'),null, 'outside pointer dismisses Tools');
  await page.locator('.topbarTools > summary').focus();
  await page.locator('.topbarTools > summary').press('Enter');
  await page.locator('.topbarTools > summary').press('Escape');
  assert.equal(await page.locator('.topbarTools').getAttribute('open'),null);
  assert.equal(await page.locator('.topbarTools > summary').evaluate(el=>document.activeElement===el),true);
  await page.evaluate(()=>window.proof.setBpm(80));
  await openTools();
  assert.equal(await page.getByRole('button',{name:'Edit BPM'}).textContent(),'80', 'passive tapped tempo changes update the readout without entering edit mode');
  await page.getByRole('button',{name:'Edit BPM'}).click();
  await page.getByRole('spinbutton',{name:'BPM'}).fill('137.5');
  await page.evaluate(()=>window.editingBpm=document.querySelector('.topbarBpmInput'));
  evidence.tempoResize=[];
  for(const width of [960,1280,960]) {
    await page.setViewportSize({width,height:800});
    const edit=await page.evaluate(()=>{
      const input=document.querySelector('.topbarBpmInput'),r=input.getBoundingClientRect();
      return {sameNode:input===window.editingBpm,focused:document.activeElement===input,value:input.value,bpm:window.proof.bpm,
        hit:input.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))};
    });
    assert.deepEqual(edit,{sameNode:true,focused:true,value:'137.5',bpm:80,hit:true},'resizing retains the focused BPM draft without an implicit commit');
    evidence.tempoResize.push({width,...edit});
  }
  await page.getByRole('spinbutton',{name:'BPM'}).press('Enter');
  assert.equal(await page.evaluate(()=>window.proof.bpm),137.5);
  assert.equal(await page.getByRole('button',{name:'Edit BPM'}).textContent(),'138', 'existing rounded BPM readout');
  await page.getByRole('button',{name:'Edit BPM'}).click();
  await page.getByRole('spinbutton',{name:'BPM'}).fill('142');
  await page.getByRole('spinbutton',{name:'BPM'}).press('Escape');
  assert.equal(await page.evaluate(()=>window.proof.bpm),137.5,'Escape cancels a BPM draft without blur-committing it');
  assert.equal(await page.locator('.topbarBpmInput').count(),0,'BPM input owns its first Escape');
  assert.notEqual(await page.locator('.topbarTools').getAttribute('open'),null,'BPM cancellation keeps Tools open');
  await page.getByRole('button',{name:'Project menu',exact:true}).click();
  await page.locator('.workspaceOperationsButton').click();
  assert.equal(await page.getByRole('dialog',{name:'Workspace and operator controls'}).isVisible(),true);
  await page.locator('.workspaceOperationsButton').click();
  await page.locator('[data-project-menu-action="save"]').click();
  assert.equal(await page.evaluate(()=>window.proof.calls.at(-1)),'save');
  evidence.presentation=[];
  for(const width of [960,1280,1501]) {
    await page.setViewportSize({width,height:800});
    await page.goto(`${origin}header-proof`);
    await page.locator('.topbar').waitFor();
    await page.evaluate(()=>{window.proof.setLocale('ja');window.proof.setScale(110)});
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    const geometry=await measure();
    await writeFile(resolve(artifactDir,`header-geometry-ja-110-${width}.json`),JSON.stringify(geometry,null,2));
    await page.locator('.workspaceChrome').screenshot({path:resolve(artifactDir,`header-ja-110-${width}.png`)});
    assert.equal(await page.locator('html').getAttribute('lang'),'ja','actual localization is active');
    assertHeader(geometry,width,'Japanese, 110% UI scale',1.1);
    await page.evaluate(()=>{window.proof.setLock('Partial');window.proof.setLabel('長いプロジェクト名'.repeat(12)+'.sdc *')});
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    const expanded=await measure();
    assertHeader(expanded,width,'Japanese, 110% UI scale, locked',1.1,width===960);
    await openTools();
    const disclosed=await measure();
    assert.ok(disclosed.text.length===4&&disclosed.text.every(item=>item.visible&&item.fits),`${width}px Japanese clock details remain fully readable`);
    const tempoReachable=await page.locator('[data-topbar-tap]').evaluate(el=>{
      const r=el.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);
      return r.width>=44&&r.height>=44&&el.contains(hit);
    });
    assert.equal(tempoReachable,true,`${width}px Japanese 110% tempo remains reachable at scaled full size`);
    await page.locator('.topbarTools > summary').press('Escape');
    await page.evaluate(()=>{window.proof.setLock(null);window.proof.setLabel('Untitled.sdc')});
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    const restored=await measure();
    assertHeader(restored,width,'Japanese, 110% UI scale, unlocked again',1.1);
    evidence.presentation.push({width,locale:'ja',uiScale:110,geometry,expanded,disclosed,tempoReachable,restored});
  }
  assert.deepEqual(errors,[]);
  evidence.interactionsPassed=true;
  evidence.errors=errors;
  await writeFile(resolve(artifactDir,'header-evidence.json'),JSON.stringify(evidence,null,2)+'\n');
  console.log(`PASS responsive Control header: nine widths, safe/dirty/locked states, no clipping or overlap, retained hit targets, transport/blackout/learn/tempo/project/workspace actions and Escape focus; evidence=${artifactDir}`);
} finally {
  await browser?.close();
  await server?.close();
  await unlink(fixturePath).catch(()=>{});
  await unlink(baselineFixturePath).catch(()=>{});
  await unlink(baselinePath).catch(()=>{});
  await unlink(baselineCssPath).catch(()=>{});
}

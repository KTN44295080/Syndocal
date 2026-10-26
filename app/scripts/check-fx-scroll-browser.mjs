import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "vite";

const optionalPort = (name) => {
  const raw = process.env[name];
  if (raw === undefined) return 0;
  const value = Number(raw);
  assert.ok(Number.isInteger(value) && value >= 1 && value <= 65535, name + " must be a port from 1 to 65535");
  return value;
};
const vitePort = optionalPort("SYNDOCAL_VITE_PORT");
const cdpPort = optionalPort("SYNDOCAL_CDP_PORT");

const require = createRequire(import.meta.url);
let chromium;
try { ({chromium} = require("playwright")); }
catch { ({chromium} = require(resolve(homedir(), ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright"))); }
let server, browser, profile;
const results = [];
const openFamily = async (page, url, family) => {
  await page.goto(url + "/?syndocalViewportFixture=scene-matrix");
  await page.locator('[data-workspace-option="control"]').click();
  await page.locator('[data-edit-domain-navigation] [data-control-mode-option="edit"]').click();
  await page.locator('[data-scene-matrix-edit-strip="301"]').click();
  await page.locator('[data-scene-settings-surface-control="fx"]').click();
  const add = page.locator('[data-scene-fx-disclosure="add"]');
  if (!(await add.evaluate((node) => node.open))) await add.locator(":scope > summary").click();
  await page.locator('.effectFamilyChooser button').filter({hasText: family}).click();
  await page.locator('.sceneFxEditor .effectSubmitActions button.primary').waitFor();
};
const measureOwner = (page) => page.locator('[data-scene-settings]').evaluate((root) => {
  const editor = root.querySelector('.sceneFxEditor'), owner = root.querySelector('.sceneSettingsScroller');
  const visible = (node) => node instanceof HTMLElement && node.checkVisibility() && node.getBoundingClientRect().width > 0 && node.getBoundingClientRect().height > 0;
  const nodes = [root, ...root.querySelectorAll('*')].filter(visible);
  const vertical = nodes.filter((node) => /auto|scroll/.test(getComputedStyle(node).overflowY) && node.scrollHeight > node.clientHeight + 1);
  const horizontal = nodes.filter((node) => /auto|scroll/.test(getComputedStyle(node).overflowX) && node.scrollWidth > node.clientWidth + 1);
  const e = editor.getBoundingClientRect(), o = owner.getBoundingClientRect();
  const save = editor.querySelector('.effectSubmitActions button.primary').getBoundingClientRect();
  return {fxHeight: e.height, fxScrollHeight: editor.scrollHeight, scrollHeight: owner.scrollHeight, clientHeight: owner.clientHeight,
    ownerTop: owner.scrollTop, horizontal: horizontal.map((node) => node.className),
    vertical: vertical.map((node) => node.className), onlyOwner: vertical.every((node) => node === owner),
    nested: vertical.filter((node) => vertical.some((parent) => parent !== node && parent.contains(node))).length,
    editorNatural: editor.scrollHeight <= editor.clientHeight + 2,
    saveContained: save.top >= o.top + owner.clientTop - 1 && save.bottom <= o.top + owner.clientTop + owner.clientHeight + 1 && save.left >= o.left - 1 && save.right <= o.left + owner.clientWidth + 1,
    outerFixed: scrollX === 0 && scrollY === 0 && document.documentElement.scrollWidth <= innerWidth && document.documentElement.scrollHeight <= innerHeight,
    disclosureOpenCount: root.querySelectorAll('[data-scene-fx-disclosure][open]').length};
});
const measurePointLists = (page) => page.locator('[data-scene-settings]').evaluate(async (root) => {
  const owner=root.querySelector('.sceneSettingsScroller'), beforeTop=owner.scrollTop;
  const visible=node=>node instanceof HTMLElement&&node.checkVisibility()&&node.getBoundingClientRect().width>0&&node.getBoundingClientRect().height>0;
  const tracked=[document.documentElement,document.body,root,...root.querySelectorAll('*')].filter(node=>node instanceof HTMLElement);
  const result=[];
  for(const list of [...root.querySelectorAll('.sceneFxEditor .moveEffectPointList')].filter(visible)) {
    const bounds=list.getBoundingClientRect(), o=owner.getBoundingClientRect();
    const controls=[...list.querySelectorAll('button,input,select')].filter(visible);
    const dimensions=controls.map(node=>{
      const b=node.getBoundingClientRect(),row=node.closest('.moveEffectPointRow')?.getBoundingClientRect();
      return {label:node.getAttribute('aria-label')||node.textContent.trim(),width:b.width,height:b.height,font:parseFloat(getComputedStyle(node).fontSize),
        left:b.left,right:b.right,listLeft:bounds.left,listRight:bounds.left+list.clientWidth,
        horizontalContained:b.left>=Math.max(bounds.left,o.left,row?.left??-Infinity)-1&&b.right<=Math.min(bounds.left+list.clientWidth,o.left+owner.clientWidth,row?.right??Infinity)+1};
    });
    const actions=controls.filter(node=>node instanceof HTMLButtonElement&&!node.disabled), last=actions.at(-1);
    const before=tracked.map(node=>({node,left:node.scrollLeft,top:node.scrollTop}));
    let lastAction=null;
    if(last) {
      const b=last.getBoundingClientRect(),p=owner.getBoundingClientRect();
      // On tall desks the existing Save dock sticks to the owner's bottom.
      // Reveal at the top through this same owner, then require a real hit.
      owner.scrollTop+=b.top-(p.top+owner.clientTop+2);
      await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
      const b2=last.getBoundingClientRect(),p2=owner.getBoundingClientRect(),hit=document.elementFromPoint(b2.left+b2.width/2,b2.top+b2.height/2);
      lastAction={label:last.getAttribute('aria-label')||last.textContent.trim(),width:b2.width,height:b2.height,
        contained:b2.left>=p2.left-1&&b2.right<=p2.left+owner.clientWidth+1&&b2.top>=p2.top-1&&b2.bottom<=p2.top+owner.clientHeight+1,
        hit:hit===last||last.contains(hit),onlyOwnerVerticalScroll:before.every(({node,left,top})=>node.scrollLeft===left&&(node===owner||node.scrollTop===top))};
    }
    result.push({label:list.getAttribute('aria-label'),controls:dimensions, naturalHeight:list.scrollHeight<=list.clientHeight+1,
      horizontalContained:dimensions.length>0&&dimensions.every(item=>item.horizontalContained),
      sizesPreserved:dimensions.length>0&&dimensions.every(item=>item.width>=39.5&&item.height>=39.5&&item.font>=14),
      lastAction});
  }
  owner.scrollTop=beforeTop;
  await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
  return result;
});
const measureColorStops = (page) => page.locator('[data-scene-settings]').evaluate(async (root) => {
  const owner=root.querySelector('.sceneSettingsScroller'), beforeTop=owner.scrollTop;
  const tracked=[document.documentElement,document.body,root,...root.querySelectorAll('*')].filter(node=>node instanceof HTMLElement);
  const visible=node=>node instanceof HTMLElement&&node.checkVisibility()&&node.getBoundingClientRect().width>0&&node.getBoundingClientRect().height>0;
  const bounds=node=>{const b=node.getBoundingClientRect();return {left:b.left+node.clientLeft,top:b.top+node.clientTop,right:b.left+node.clientLeft+node.clientWidth,bottom:b.top+node.clientTop+node.clientHeight};};
  const contained=(b,p)=>b.left>=p.left-1&&b.right<=p.right+1&&b.top>=p.top-1&&b.bottom<=p.bottom+1;
  const inkFits=(node,ink,row)=>{
    if(!contained(ink,bounds(row))||!contained(ink,bounds(owner)))return false;
    for(let current=node;current;current=current.parentElement) {
      const style=getComputedStyle(current),b=bounds(current);
      if(/auto|scroll|hidden|clip/.test(style.overflowX)&&(ink.left<b.left-1||ink.right>b.right+1))return false;
      if(/auto|scroll|hidden|clip/.test(style.overflowY)&&(ink.top<b.top-1||ink.bottom>b.bottom+1))return false;
    }
    return ink.left>=0&&ink.right<=innerWidth&&ink.top>=0&&ink.bottom<=innerHeight;
  };
  const result=[];
  for(const row of [...root.querySelectorAll('.sceneFxEditor .colorEffectStopRow')].filter(visible)) {
    const before=tracked.map(node=>({node,left:node.scrollLeft,top:node.scrollTop})),b=row.getBoundingClientRect(),o=bounds(owner);
    // A bottom-aligned row may legitimately sit under the sticky Save dock.
    // Top alignment is normal owner scrolling, not a hit/containment exemption.
    owner.scrollTop+=b.top-(o.top+2);
    await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
    const rowBounds=bounds(row),ownerBounds=bounds(owner);
    const labels=[...row.querySelectorAll('.colorEffectStopNumber,.colorEffectStopPosition > span')].map(node=>{
      const range=document.createRange();range.selectNodeContents(node);
      const rects=[...range.getClientRects()].filter(rect=>rect.width>0&&rect.height>0),ink=range.getBoundingClientRect();
      return {text:node.textContent.trim(),width:ink.width,height:ink.height,font:parseFloat(getComputedStyle(node).fontSize),
        lines:new Set(rects.map(rect=>Math.round(rect.top*2)/2)).size,
        fullInk:rects.length>0&&inkFits(node,ink,row)&&node.scrollWidth<=node.clientWidth+1};
    });
    const controls=[...row.querySelectorAll('input,button')].filter(visible).map(node=>{
      const rect=node.getBoundingClientRect(),hit=document.elementFromPoint(rect.left+rect.width/2,rect.top+rect.height/2);
      return {label:node.getAttribute('aria-label')||node.textContent.trim(),action:node.classList.contains('colorEffectStopAction'),
        width:rect.width,height:rect.height,font:parseFloat(getComputedStyle(node).fontSize),
        rect:{left:rect.left,top:rect.top,right:rect.right,bottom:rect.bottom},
        hitElement:hit?{tag:hit.tagName,className:hit.className,text:hit.textContent?.trim().slice(0,100)}:null,
        horizontalContained:rect.left>=Math.max(rowBounds.left,ownerBounds.left)-1&&rect.right<=Math.min(rowBounds.right,ownerBounds.right)+1,
        contained:contained(rect,rowBounds)&&contained(rect,ownerBounds),hit:hit===node||node.contains(hit)};
    });
    const input=row.querySelector('.colorEffectStopPosition input[type="number"]');
    let number=null;
    if(input) {
      const style=getComputedStyle(input),minimum=Number(input.min),maximum=Number(input.max),step=Number(input.step);
      const parts=input.step.toLowerCase().split('e'),precision=Math.max(0,(parts[0].split('.')[1]?.length??0)-Number(parts[1]??0));
      // The editor displays toFixed(4), with range 0..1 and step 0.0001.
      // Measure the actual font (including tabular digits) for both boundaries,
      // adjacent steps and the current value; reserve native Chromium spinner
      // space instead of hiding it or accepting a clipped "0.".
      const digits=Math.min(20,precision),samples=[minimum,maximum,minimum+step,maximum-step,Number(input.value)].map(value=>value.toFixed(digits));
      const probe=document.createElement('span');
      probe.style.cssText='position:fixed;left:0;top:0;visibility:hidden;white-space:pre;width:max-content;pointer-events:none';
      for(const key of ['fontFamily','fontSize','fontWeight','fontStyle','fontVariant','fontFeatureSettings','fontVariationSettings','letterSpacing'])probe.style[key]=style[key];
      document.body.append(probe);
      let requiredTextWidth=0;
      for(const sample of samples){probe.textContent=sample;requiredTextWidth=Math.max(requiredTextWidth,probe.getBoundingClientRect().width);}
      probe.remove();
      const padding=parseFloat(style.paddingLeft)+parseFloat(style.paddingRight),spinnerReserve=16;
      const availableTextWidth=input.clientWidth-padding-spinnerReserve;
      number={value:input.value,minimum,maximum,step,precision,samples,requiredTextWidth,availableTextWidth,padding,spinnerReserve,
        domainPreserved:minimum===0&&maximum===1&&step===0.0001&&precision===4,
        completeValueFits:Number.isFinite(requiredTextWidth)&&requiredTextWidth>0&&availableTextWidth>=requiredTextWidth-0.5};
    }
    result.push({labels,number,controls,actions:controls.filter(control=>control.action).length,rowBounds,ownerBounds,ownerTop:owner.scrollTop,
      onlyOwnerVerticalScroll:before.every(({node,left,top})=>node.scrollLeft===left&&(node===owner||node.scrollTop===top))});
  }
  owner.scrollTop=beforeTop;
  await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
  return result;
});
const reachSave = async (page, family) => {
  const before = await measureOwner(page);
  const points = await measurePointLists(page);
  if (["MOVE FX","VALUE FX","CURVE FX"].includes(family.replace(/ wide$/, ""))) assert(points.length > 0, family + " has lost its point operations");
  assert(points.every(point=>point.naturalHeight&&point.horizontalContained&&point.sizesPreserved&&point.lastAction?.contained&&point.lastAction?.hit&&point.lastAction?.onlyOwnerVerticalScroll), family + " point controls are clipped, undersized or need an inner scrollport: " + JSON.stringify(points));
  const colorStops = family.replace(/ wide$/, "") === "COLOR FX" ? await measureColorStops(page) : [];
  if(family.replace(/ wide$/, "") === "COLOR FX") {
    assert(colorStops.length>0, family + " has lost its palette stop rows");
    assert(colorStops.every(stop=>stop.labels.length===2&&stop.labels.every(label=>label.fullInk&&label.lines===1&&label.font>=14)
      &&stop.number?.domainPreserved&&stop.number.completeValueFits&&stop.actions===3&&stop.onlyOwnerVerticalScroll
      &&stop.controls.length===6&&stop.controls.every(control=>control.width>=39.5&&control.height>=39.5&&control.font>=14&&control.horizontalContained&&control.contained&&control.hit)),
      family + " palette labels, four-decimal positions or 40px actions are clipped: " + JSON.stringify(colorStops));
  }
  assert(before.editorNatural, family + " editor has clipped or internally scrolling form content");
  assert.equal(before.horizontal.length, 0, family + " has a horizontal scrollport");
  assert.equal(before.nested, 0, family + " has nested scrollports");
  assert.equal(before.onlyOwner, true, family + " scrolls outside Scene Settings");
  assert.equal(before.disclosureOpenCount, 0, family + " does not prioritize the selected editor");
  const box = await page.locator('.sceneSettingsScroller').boundingBox();
  await page.mouse.move(box.x + box.width - 12, box.y + box.height / 2);
  await page.mouse.wheel(0, 900);
  // A shorter disclosed form can fit completely. When it does overflow, the
  // wheel must move the sole owner, never an editor column or the document.
  if (before.scrollHeight > before.clientHeight + 1) await page.waitForFunction(() => document.querySelector('.sceneSettingsScroller').scrollTop > 0);
  await page.locator('.sceneSettingsScroller').focus();
  await page.keyboard.press('End');
  await page.waitForFunction(() => {
    const owner = document.querySelector('.sceneSettingsScroller');
    const save = owner.querySelector('.sceneFxEditor .effectSubmitActions button.primary').getBoundingClientRect(), b = owner.getBoundingClientRect();
    return save.top >= b.top + owner.clientTop - 1 && save.bottom <= b.top + owner.clientTop + owner.clientHeight + 1;
  });
  const after = await measureOwner(page);
  assert(after.saveContained && after.outerFixed && after.onlyOwner && after.nested === 0 && after.horizontal.length === 0, family + " Save is not reachable through the sole owner: " + JSON.stringify(after));
  return {...after, pointLists:points, colorStops, wheel: before.scrollHeight <= before.clientHeight + 1 ? "no-overflow-needed" : "owner-moved", endSaveReachable: true};
};
try {
  let url = process.env.SYNDOCAL_UI_TEST_URL;
  if (!url) {
    server = await createServer({root: resolve(import.meta.dirname, ".."), server: {host: "127.0.0.1", port: vitePort, strictPort: vitePort !== 0}});
    await server.listen();
    url = "http://127.0.0.1:" + server.httpServer.address().port;
  }
  // Prefer Playwright's bundled browser; fall back to an installed Chrome/Edge.
  const executablePath = process.env.SYNDOCAL_UI_TEST_BROWSER ?? [
    "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  ].find(existsSync);
  const profileRoot = resolve(import.meta.dirname, "../../target/qa/browser-profiles");
  await mkdir(profileRoot, { recursive: true });
  profile = await mkdtemp(resolve(profileRoot, "syndocal-fx-scroll-"));
  const args = cdpPort ? ["--remote-debugging-port=" + cdpPort] : [];
  try { browser = await chromium.launchPersistentContext(profile, {headless: true, args}); }
  catch (error) {
    if (!executablePath) throw error;
    browser = await chromium.launchPersistentContext(profile, {headless: true, executablePath, args});
  }
  const page = browser.pages()[0] ?? await browser.newPage();
  for (const viewport of [{width:1920,height:1080},{width:1920,height:1032},{width:2048,height:1152},{width:1366,height:768},{width:1280,height:720}]) {
    await page.setViewportSize(viewport);
    for (const family of ["COLOR FX", "CHASER FX", "MOVE FX", "VALUE FX", "CURVE FX", "2D MAPPING"]) {
      await openFamily(page, url, family);
      const measured = await reachSave(page, family);
      results.push({...viewport, family, ...measured});
    }
  }
  // Wide FX editing: the header toggle gives the pane most of the upper
  // workspace. Batch3 replaces the old side-by-side rack/editor and internal
  // editor scroll with a selected-first full-width form and the same sole owner.
  // The original width gain and persisted preference remain required.
  for (const viewport of [{width:1920,height:1080},{width:1366,height:768},{width:1280,height:720}]) {
    await page.setViewportSize(viewport);
    for (const family of ["COLOR FX", "CHASER FX", "2D MAPPING"]) {
      await openFamily(page, url, family);
      // After the first iteration the persisted preference reopens the pane
      // wide; narrow it first so both directions are exercised.
      const loadedWide = await page.locator('[data-scene-settings][data-scene-settings-wide="true"]').count() === 1;
      if (loadedWide) {
        await page.locator('[data-scene-settings-wide-toggle]').click();
        await page.waitForFunction(() => document.querySelector('[data-scene-settings]')?.getAttribute('data-scene-settings-wide') === null);
      }
      const narrowWidth = await page.locator('.sceneFxEditor').evaluate((editor) => editor.getBoundingClientRect().width);
      await page.locator('[data-scene-settings-wide-toggle]').click();
      await page.locator('[data-scene-settings][data-scene-settings-wide="true"]').waitFor();
      const wide = await page.locator('[data-scene-settings]').evaluate((root) => {
        const editor = root.querySelector('.sceneFxEditor').getBoundingClientRect();
        const controls = root.querySelector('[data-scene-fx-controls]').getBoundingClientRect();
        const owner = root.querySelector('.sceneSettingsScroller');
        const content = owner.getBoundingClientRect(), style = getComputedStyle(owner);
        const pane = root.getBoundingClientRect();
        return {editorWidth: editor.width, editorTop: editor.top, controlsBottom: controls.bottom, editorLeft: editor.left,
          fullWidth: editor.left >= content.left - 1 && editor.right <= content.left + owner.clientWidth + 1 && editor.width >= owner.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight) - 4,
          paneShare: pane.width / window.innerWidth, stored: window.localStorage.getItem("syndocal.sceneFxEditorWide.v1")};
      });
      assert(wide.editorWidth > narrowWidth + 100, family + " wide editor did not gain width");
      assert(wide.editorTop >= wide.controlsBottom - 1 && wide.editorTop - wide.controlsBottom <= 32, family + " wide editor does not follow its compact disclosure controls");
      assert.equal(wide.fullWidth, true, family + " wide editor does not use the available width");
      assert(wide.paneShare > 0.55, family + " wide pane did not take most of the upper workspace");
      assert.equal(wide.stored, "1", family + " wide preference was not persisted");
      const measured = await reachSave(page, family + " wide");
      results.push({...viewport, family, wide: true, loadedWide, narrowWidth, ...wide, ...measured, wheelSaveReachable: true});
    }
  }
  assert(results.filter((result) => result.wide && result.loadedWide).length > 0, "wide FX preference did not survive a reload");
  await page.locator('[data-scene-settings-wide-toggle]').click();
  await page.waitForFunction(() => document.querySelector('[data-scene-settings]')?.getAttribute('data-scene-settings-wide') === null
    && window.localStorage.getItem("syndocal.sceneFxEditorWide.v1") === "0");
  if (process.env.SYNDOCAL_FX_SCROLL_ARTIFACT_DIR) {
    await mkdir(process.env.SYNDOCAL_FX_SCROLL_ARTIFACT_DIR, {recursive: true});
    await writeFile(resolve(process.env.SYNDOCAL_FX_SCROLL_ARTIFACT_DIR, "fx-scroll-browser.json"), JSON.stringify({passed: true, results}, null, 2) + "\n");
  }
  console.log("FX form content and wheel/End save reachability: " + results.length + " passed");
} finally {
  await browser?.close();
  if (profile) await rm(profile, {recursive: true, force: true, maxRetries: 5, retryDelay: 200});
  await server?.close();
}

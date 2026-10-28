import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { resolve } from "node:path";
import { writeFile, unlink, mkdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { createServer } from "vite";
import { createServer as createTcpServer } from "node:net";

// The network universes are row groups of the one DMX universes table
// (DmxOutputConfigPanel). Every callback is a stand-in: no Tauri, no hardware.
const require = createRequire(import.meta.url);
let chromium;
try { ({chromium} = require("playwright")); }
catch { ({chromium} = require(resolve(homedir(), ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright"))); }
// Browser plugin unavailable: Playwright runs actual Solid/production styles with stand-in IPC only.
const root = resolve(import.meta.dirname, ".."), name = ".network-routes-proof-" + process.pid + ".tsx", fixture = resolve(root, name);
const artifactDir = process.env.SYNDOCAL_DMX_UI_EVIDENCE ?? resolve(root, "../../ui-evidence");
let browser, server;
try {
  await mkdir(artifactDir,{recursive:true});
  await writeFile(fixture, `
import {render} from 'solid-js/web';import {createSignal} from 'solid-js';
import {DmxOutputConfigPanel} from './src/components/DmxOutputConfigPanel';
import './src/theme.css';import './src/styles.css';
import {installUiLocalization} from './src/uiLocalization';
const sentinel={enabled:false,protocol:'ArtNet',target_ip:'0.0.0.0',port:0,universe:0,serial_port:'',serial_baud_rate:57600};
const device={name:'COM7',port_type:'USB',usb_vid:1027,usb_pid:24577,serial_number:'one',manufacturer:'FTDI',product:'Open DMX',windows_device_instance_id:'USB\\VID_0403&PID_6001\\one'};
const [routes,setRoutes]=createSignal([]),[epoch,setEpoch]=createSignal(1),[usb,setUsb]=createSignal({bindings:[],cables:[],persistenceError:null});
window.__networkProof={calls:[],fail:false,replace(rows){setRoutes(rows);},seed(){setRoutes([1,2].map(universe=>({...sentinel,enabled:true,target_ip:'10.0.0.'+(universe+10),port:6454,universe})));setUsb({bindings:[{universe:0,state:'selected_and_present',identity:{...device,port_name:device.name},detail:''}],cables:[{universe:0,active:true,faulted:false,liveFrameQueued:true,workerShutdownCompleted:false}],persistenceError:null});setEpoch(epoch()+1);},newProject(){setEpoch(epoch()+1);setRoutes([]);setUsb({bindings:[],cables:[],persistenceError:null});}};
render(()=><div style={{height:'100vh',overflow:'auto',padding:'12px'}}><DmxOutputConfigPanel output={sentinel} routes={routes()} routeTelemetry={[]} projectEpoch={epoch()} serialPorts={[device]} usbDmxUniverses={usb()} usbDmxUniversesStale={false} safetyBlackoutEngaged={true} onRefreshUsbDmxUniverses={()=>{}} onRescanSerialPorts={()=>{}} onApplyUniverseTable={async plan=>{window.__networkProof.calls.push(structuredClone(plan));if(window.__networkProof.fail)throw Error('fixture device failed; completed settings refreshed');setRoutes(plan.network);setUsb({bindings:plan.usb.map(row=>({universe:row.universe,state:'selected_and_present',identity:{...device,port_name:device.name},detail:''})),cables:plan.usb.map(row=>({universe:row.universe,active:row.enabled,workerShutdownCompleted:!row.enabled,faulted:false,liveFrameQueued:false})),persistenceError:null});}}/></div>,document.body);
window.__dmxClicks=[];document.addEventListener('click',event=>{const target=event.target.closest('button,input,select');if(target)window.__dmxClicks.push({tag:target.tagName,label:target.getAttribute('aria-label')??target.textContent.trim()});});
installUiLocalization(document.body,()=>new URLSearchParams(location.search).get('locale')==='ja'?'ja':'en');
`);
  const reservation=createTcpServer();await new Promise(resolve=>reservation.listen(0,"127.0.0.1",resolve));const fixturePort=reservation.address().port;await new Promise((resolve,reject)=>reservation.close(error=>error?reject(error):resolve()));
  server = await createServer({root, cacheDir: resolve(root, ".vite-task74-table"), server: {host: "127.0.0.1", port: fixturePort, strictPort: true}, plugins:[{name:"network-route-fixture",configureServer(vite){vite.middlewares.use((request,response,next)=>{if(request.url?.split("?")[0]==="/favicon.ico"){response.statusCode=204;response.end();return;}if(request.url?.split("?")[0]!=="/network-routes-proof")return next();response.setHeader("Content-Type","text/html");response.end('<html><head><title>Syndocal DMX universe table proof</title><link rel="icon" href="data:,"></head><body><script type="module" src="/'+name+'"></script></body></html>');});}}]});
  await server.listen();
  const url = "http://127.0.0.1:" + server.httpServer.address().port;
  // Prefer Playwright's bundled browser; fall back to an installed Chrome/Edge.
  const executablePath = process.env.SYNDOCAL_UI_TEST_BROWSER ?? [
    "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  ].find(existsSync);
  try { browser = await chromium.launch({headless: true}); }
  catch (error) {
    if (!executablePath) throw error;
    browser = await chromium.launch({headless: true, executablePath});
  }
  const page = await browser.newPage();
  const errors=[];page.on('pageerror',error=>errors.push(error.message));page.on('console',message=>{if(message.type()==='error')errors.push(message.text());});
  const open=async()=>{await page.goto(url+'/network-routes-proof');await page.locator('[data-dmx-universe-table]').waitFor();};
  const add=async(kind='ArtNet')=>{await page.locator('[data-dmx-universe-add-transport]').selectOption(kind);await page.locator('[data-dmx-universe-add-button]').click();};
  const save=page.locator('[data-dmx-universe-apply]');
  for(const viewport of [{width:1920,height:1080},{width:960,height:640}]){
    await page.setViewportSize(viewport);await open();
    assert.equal(await page.locator('[data-network-dmx-route]').count(),0);
    await add();assert.equal(await page.locator('[data-network-dmx-universe]').inputValue(),'0');
    await page.locator('[data-network-dmx-destination]').fill('10.0.0.8');await add('Usb');
    assert.equal(await page.evaluate(()=>window.__networkProof.calls.length),0,'Add only drafts');
    await save.click();await page.waitForFunction(()=>window.__networkProof.calls.length===1);
    const plan=await page.evaluate(()=>window.__networkProof.calls[0]);assert.equal(plan.network[0].universe,0);assert.equal(plan.usb[0].universe,0);assert.equal(plan.usb[0].port.name,'COM7');
    assert(await save.isDisabled());
    await page.locator('[data-network-dmx-route] input[type=checkbox]').uncheck();await save.click();await page.waitForFunction(()=>window.__networkProof.calls.length===2);
    assert.equal(await page.evaluate(()=>window.__networkProof.calls[1].network[0].enabled),false);assert.equal(await page.evaluate(()=>window.__networkProof.calls[1].usb[0].enabled),true);
    await page.locator('[data-network-dmx-destination]').fill('10.0.0.9');await page.evaluate(()=>window.__networkProof.replace([{enabled:false,protocol:'ArtNet',target_ip:'127.0.0.1',port:6454,universe:0,serial_port:'',serial_baud_rate:57600}]));
    await page.getByText('Output settings changed. Reload the current settings before applying.').waitFor();assert(await save.isDisabled());await page.locator('[data-dmx-universe-reload]').click();assert.equal(await page.locator('[data-network-dmx-destination]').inputValue(),'127.0.0.1');
    await page.locator('[data-network-dmx-destination]').fill('10.0.0.10');await page.evaluate(()=>{window.__networkProof.fail=true;});await save.click();await page.getByText('Some output settings could not be applied. Reload the refreshed settings before retrying.').waitFor();await page.getByText('Diagnostics',{exact:true}).click();await page.getByText('Error: fixture device failed; completed settings refreshed',{exact:true}).waitFor();assert(await save.isDisabled());
    await page.evaluate(()=>window.__networkProof.newProject());await page.waitForFunction(()=>document.querySelectorAll('[data-network-dmx-route]').length===0);assert(await save.isDisabled());
    for(let i=0;i<7;i++)await add();assert.equal(await page.locator('[data-network-dmx-route]').count(),2);await page.getByRole('button',{name:'Previous',exact:true}).click();assert.equal(await page.locator('[data-network-dmx-route]').count(),5);
  }
  await open();await page.locator('[data-dmx-universe-add-button]').click();await page.locator('[data-network-dmx-destination]').click();await page.locator('[data-network-dmx-destination]').fill('10.0.0.99');await save.click();
  await page.waitForFunction(()=>window.__networkProof.calls.length===1);const artNetClicks=await page.evaluate(()=>window.__dmxClicks);assert.equal(artNetClicks.length,3,JSON.stringify(artNetClicks));
  await open();await page.locator('[data-dmx-universe-add-transport]').click();await page.locator('[data-dmx-universe-add-transport]').selectOption('Usb');await page.locator('[data-dmx-universe-add-button]').click();await save.click();
  await page.waitForFunction(()=>window.__networkProof.calls.length===1);const usbClicks=await page.evaluate(()=>window.__dmxClicks);assert.equal(usbClicks.length,3,JSON.stringify(usbClicks));
  const captures=[];
  for(const viewport of [{width:1920,height:1080},{width:960,height:640}])for(const locale of ['en','ja']){
    await page.setViewportSize(viewport);await page.goto(url+'/network-routes-proof?locale='+locale);await page.evaluate(()=>window.__networkProof.seed());
    await page.waitForFunction(locale=>document.documentElement.lang===locale,locale);
    assert.equal(await page.locator('[data-usb-dmx-cable]').count(),1);assert.equal(await page.locator('[data-network-dmx-route]').count(),2);
    const geometry=await page.locator('.dmxOutputConfigPanel').evaluate(panel=>{
      const bounds=panel.getBoundingClientRect();const controls=[...panel.querySelectorAll('input,select,button,summary')].filter(el=>el.getBoundingClientRect().width>0);
      const overflowing=controls.filter(el=>{const box=el.getBoundingClientRect(),cell=el.closest('td,th')?.getBoundingClientRect();return box.left<bounds.left-1||box.right>bounds.right+1||(cell&&(box.left<cell.left-1||box.right>cell.right+1));}).map(el=>({tag:el.tagName,label:el.getAttribute('aria-label')??el.textContent,rect:el.getBoundingClientRect().toJSON()}));
      const clippedLabels=[...panel.querySelectorAll('button,th,td > span,.ioConnectionState')].filter(el=>el.getBoundingClientRect().width>0&&el.scrollWidth>el.clientWidth+1).map(el=>el.textContent.trim());
      return {panel:bounds.toJSON(),horizontalOverflow:panel.scrollWidth-panel.clientWidth,overflowing,clippedLabels,title:document.title,url:location.href,lang:document.documentElement.lang,heading:panel.querySelector('h2').textContent.trim(),content:panel.innerText,rows:[...panel.querySelectorAll('tbody tr')].map(el=>el.getBoundingClientRect().toJSON())};
    });
    assert.equal(geometry.horizontalOverflow,0,JSON.stringify(geometry));assert.deepEqual(geometry.overflowing,[],JSON.stringify(geometry));assert.deepEqual(geometry.clippedLabels,[],JSON.stringify(geometry));
    assert.equal(geometry.heading,locale==='ja'?'DMX\u30e6\u30cb\u30d0\u30fc\u30b9':'DMX universes');assert(!geometry.content.includes('????'),'Japanese copy must be genuine text');
    if(viewport.height===1080)assert(geometry.rows.every(row=>row.top>=0&&row.bottom<=1080),'USB0 + ArtNet1/2 all visible without scrolling');
    const screenshot=resolve(artifactDir,`dmx-table-${locale}-${viewport.width}x${viewport.height}.png`);await page.screenshot({path:screenshot,fullPage:true});captures.push({viewport,locale,screenshot,...geometry});
  }
  const fullAppCaptures=[];
  for(const viewport of [{width:1920,height:1080},{width:960,height:640}])for(const locale of ['en','ja']){
    const appPage=await browser.newPage({viewport});
    appPage.on('response',response=>{if(response.status()>=400)errors.push(response.status()+' '+response.url());});
    appPage.on('pageerror',error=>errors.push(error.message));appPage.on('console',message=>{if(message.type()==='error')errors.push(message.text());});
    await appPage.addInitScript(locale=>{localStorage.clear();localStorage.setItem('syndocal.uiLocale.v1',locale);},locale);
    await appPage.goto(url+'/?syndocalViewportFixture=setup-io');
    await appPage.locator('[data-workspace-option="setup"]').click();await appPage.getByRole('button',{name:'I/O',exact:true}).click();await appPage.locator('[data-io-primary-connection="dmx"] > button').click();
    await appPage.locator('[data-dmx-universe-table]').waitFor();
    const geometry=await appPage.evaluate(()=>{
      const panel=document.querySelector('.dmxOutputConfigPanel'),bounds=panel.getBoundingClientRect(),app=document.querySelector('.app');
      const overflowing=[...panel.querySelectorAll('input,select,button')].filter(el=>{const r=el.getBoundingClientRect(),cell=el.closest('td,th')?.getBoundingClientRect();return r.width>0&&(r.left<bounds.left-1||r.right>bounds.right+1||(cell&&(r.left<cell.left-1||r.right>cell.right+1)));}).map(el=>el.getAttribute('aria-label')??el.textContent);
      return {title:document.title,url:location.href,lang:document.documentElement.lang,heading:panel.querySelector('h2').textContent.trim(),bodyWidth:document.body.scrollWidth,viewportWidth:innerWidth,appWidth:app.clientWidth,appScrollWidth:app.scrollWidth,panel:bounds.toJSON(),overflowing,content:panel.innerText};
    });
    assert.equal(geometry.lang,locale);assert.equal(geometry.bodyWidth,viewport.width);assert.equal(geometry.appWidth,geometry.appScrollWidth);assert.deepEqual(geometry.overflowing,[],JSON.stringify(geometry));
    const screenshot=resolve(artifactDir,`setup-io-app-${locale}-${viewport.width}x${viewport.height}.png`);await appPage.screenshot({path:screenshot});fullAppCaptures.push({viewport,locale,screenshot,...geometry});await appPage.close();
  }
  assert.deepEqual(errors,[]);
  await writeFile(resolve(artifactDir,'dmx-table-results.json'),JSON.stringify({passed:true,captures,fullAppCaptures,errors,clickEvidence:{artNetClicks,usbClicks,note:'Recorded DOM click events; IP typing and USB dropdown selection excluded from click count. Apply callback is a stand-in; no native confirmation or hardware.'},scope:'Actual Solid table and production styles/localizer in browser; stand-in apply, no native/hardware'},null,2));
  assert.deepEqual(errors,[]);console.log('DMX universe table browser: combined draft Apply, independent U0/enable, conflicts, failure, epoch reset, pagination passed (1920/960).');
} finally { await browser?.close();await server?.close();await unlink(fixture).catch(()=>{}); }

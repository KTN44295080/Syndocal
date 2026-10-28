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
const root = resolve(import.meta.dirname, ".."), name = ".network-routes-proof-" + process.pid + ".tsx", fixture = resolve(root, name);
const artifactDir = process.env.SYNDOCAL_DMX_UI_EVIDENCE ?? resolve(root, "../../ui-evidence");
let browser, server;
try {
  await mkdir(artifactDir,{recursive:true});
  await writeFile(fixture, `
import {render} from 'solid-js/web';import {createSignal} from 'solid-js';
import {DmxOutputConfigPanel} from './src/components/DmxOutputConfigPanel';
import {serialDmxConfirmationRequest} from './src/serialDmxDeviceInstance';
import './src/theme.css';import './src/styles.css';
const sentinel={enabled:false,protocol:'ArtNet',target_ip:'0.0.0.0',port:0,universe:0,serial_port:'',serial_baud_rate:57600};
const [ports,setPorts]=createSignal([]),[epoch,setEpoch]=createSignal(1),[usb,setUsb]=createSignal({bindings:[],cables:[],persistenceError:null}),[blackout,setBlackout]=createSignal(true),[stale,setStale]=createSignal(false);
window.__usbProof={calls:[],requests:[],blackout:setBlackout,stale:setStale,fault(){setUsb({...usb(),cables:usb().cables.map(row=>({...row,faulted:true}))});},platform(value){const valid={name:value==='mac'?'/dev/cu.usbserial-A':value==='macusb'?'usb-ftdi://000000010000abcd':'COM4',port_type:'USB',usb_vid:1027,usb_pid:24577,serial_number:'A',manufacturer:'FTDI',product:'FT232R USB UART',...(value==='windows'?{windows_device_instance_id:'FTDIBUS-A'}:{macos_device_instance_id:'ioreg:000000010000abcd'})};setPorts([valid,{...valid,name:'incomplete',windows_device_instance_id:null,macos_device_instance_id:null}]);setUsb({bindings:[],cables:[],persistenceError:null});setEpoch(epoch()+1);},missing(){setPorts([]);setUsb({bindings:[],cables:[],persistenceError:null});setEpoch(epoch()+1);}};
render(()=><main class="app" style="padding:24px;overflow:auto"><DmxOutputConfigPanel output={sentinel} routes={[]} routeTelemetry={[]} projectEpoch={epoch()} serialPorts={ports()} usbDmxUniverses={usb()} usbDmxUniversesStale={stale()} safetyBlackoutEngaged={blackout()} onRefreshUsbDmxUniverses={()=>{}} onRescanSerialPorts={()=>{}} onApplyUniverseTable={async plan=>{window.__usbProof.calls.push(structuredClone(plan));window.__usbProof.requests.push(serialDmxConfirmationRequest(plan.usb[0].port));const identity={...plan.usb[0].port,port_name:plan.usb[0].port.name};setUsb({bindings:[{universe:0,state:'selected_and_present',identity,detail:''}],cables:[{universe:0,active:true,faulted:false,workerShutdownCompleted:false,liveFrameQueued:true}],persistenceError:null});}}/></main>,document.body);
`);
  const reservation=createTcpServer();await new Promise(resolve=>reservation.listen(0,"127.0.0.1",resolve));const fixturePort=reservation.address().port;await new Promise((resolve,reject)=>reservation.close(error=>error?reject(error):resolve()));
  server = await createServer({root, cacheDir: resolve(root, ".vite-task74-usb"), server: {host: "127.0.0.1", port: fixturePort, strictPort: true}, plugins:[{name:"network-route-fixture",configureServer(vite){vite.middlewares.use((request,response,next)=>{if(request.url?.split("?")[0]!=="/network-routes-proof")return next();response.setHeader("Content-Type","text/html");response.end('<html><head><link rel="icon" href="data:,"></head><body><script type="module" src="/'+name+'"></script></body></html>');});}}]});
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
  const page=await browser.newPage(),errors=[],results=[];
  page.on('pageerror',error=>errors.push(error.message));page.on('console',message=>{if(message.type()==='error')errors.push(message.text());});
  for(const viewport of [{width:1920,height:1080},{width:960,height:640}])for(const platform of ['windows','mac','macusb']){
    await page.setViewportSize(viewport);await page.goto(url+'/network-routes-proof');await page.evaluate(platform=>window.__usbProof.platform(platform),platform);
    await page.locator('[data-dmx-universe-add-transport]').selectOption('Usb');
    const select=page.locator('[data-usb-dmx-cable-device]');
    assert.equal(await select.locator('option').count(),2,'only eligible device plus prompt');
    assert.match(await select.inputValue(),platform==='windows'?/^COM4/:platform==='mac'?/^\/dev\/cu.usbserial-A/:/^usb-ftdi:\/\//);
    await page.locator('[data-dmx-universe-add-button]').click();assert.equal(await page.evaluate(()=>window.__usbProof.calls.length),0,'Add creates a draft only');
    await page.locator('[data-dmx-universe-apply]').click();await page.waitForFunction(()=>window.__usbProof.calls.length===1);
    const proof=await page.evaluate(()=>({plan:window.__usbProof.calls[0],request:window.__usbProof.requests[0]}));assert.equal(proof.plan.usb[0].universe,0);assert.deepEqual(proof.plan.network,[]);assert.equal(proof.request.windowsDeviceInstanceId,platform==='windows'?'FTDIBUS-A':null);assert.equal(proof.request.macosDeviceInstanceId,platform==='windows'?null:'ioreg:000000010000abcd');
    assert.match(await page.locator('[data-usb-dmx-cable-state]').innerText(),/Sending/);
    const screenshot=resolve(artifactDir,`usb-${platform}-${viewport.width}x${viewport.height}.png`);await page.screenshot({path:screenshot,fullPage:true});
    await page.evaluate(()=>window.__usbProof.blackout(false));await page.locator('[data-usb-dmx-cable] input[type=checkbox]').uncheck();assert(await page.locator('[data-dmx-universe-apply]').isDisabled());await page.getByText('Engage safety blackout before disabling, removing or changing an active USB-DMX device.').waitFor();assert.equal(await page.evaluate(()=>window.__usbProof.calls.length),1,'disabled draft never stops live output');
    await page.locator('[data-dmx-universe-reload]').click();await page.evaluate(()=>window.__usbProof.fault());await page.waitForFunction(()=>document.querySelector('[data-usb-dmx-cable-state]').textContent==='Fault');assert(await page.locator('[data-usb-dmx-cable] input[type=checkbox]').isChecked(),'fault must retain active enabled truth');
    await page.evaluate(()=>window.__usbProof.stale(true));await page.getByText('Status unavailable',{exact:true}).waitFor();assert(await page.locator('[data-usb-dmx-cable] input[type=checkbox]').isChecked(),'stale last-known active must not become stopped');
    await page.evaluate(()=>window.__usbProof.missing());await page.locator('[data-dmx-universe-add-transport]').selectOption('Usb');assert(await page.locator('[data-dmx-universe-add-button]').isDisabled());assert(await page.locator('[data-dmx-universe-apply]').isDisabled());
    results.push({platform,viewport,universe:0,clicks:3,identity:proof.request,screenshot,missingDeviceBlocked:true});
  }
  assert.deepEqual(errors,[]);await writeFile(resolve(artifactDir,'usb-platform-results.json'),JSON.stringify({passed:true,results,errors,scope:'Browser fixtures only; no native or physical DMX output'},null,2));console.log(`USB-DMX unified table platform browser: PASS (${results.length} Windows/macOS identity + USB0 draft/single Apply fixtures; no hardware).`);
}finally{await browser?.close();await server?.close();await unlink(fixture).catch(()=>{});}

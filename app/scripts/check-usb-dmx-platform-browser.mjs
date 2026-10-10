import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { mkdir, writeFile, unlink } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { homedir } from 'node:os';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer as createTcpServer } from 'node:net';
import { createServer } from 'vite';

// Browser plugin unavailable; actual Solid component in headless Playwright.
// All output callbacks are stand-ins: this test never invokes Tauri or hardware.
const require = createRequire(import.meta.url);
let chromium;
try { ({chromium} = require('playwright')); } catch {
  ({chromium} = require(resolve(homedir(), '.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright')));
}
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const artifactDir = resolve(root, '../target/qa/usb-dmx-alpha71');
const fixtureName = `.usb-dmx-proof-${process.pid}.tsx`;
const fixturePath = resolve(root, fixtureName);
let server, browser;
try {
  await mkdir(artifactDir, {recursive:true});
  await writeFile(fixturePath, `
import {render} from 'solid-js/web';
import {createSignal} from 'solid-js';
import {DmxOutputConfigPanel} from './src/components/DmxOutputConfigPanel';
import {serialDmxConfirmationRequest} from './src/serialDmxDeviceInstance';
import './src/styles.css';
const [ports,setPorts]=createSignal([]), [binding,setBinding]=createSignal(null);
const proof={requests:[], platform: v=>{
 const valid={name:v==='mac'?'/dev/cu.usbserial-A':v==='macusb'?'usb-ftdi://000000010000abcd':'COM4',port_type:'USB',usb_vid:1027,usb_pid:24577,
 serial_number:'A',manufacturer:'FTDI',product:'FT232R USB UART',
 ...(v!=='windows'?{macos_device_instance_id:'ioreg:000000010000abcd'}:{windows_device_instance_id:'FTDIBUS-A'})};
 setPorts([valid,{...valid,name:'incomplete',windows_device_instance_id:null,macos_device_instance_id:null}]);setBinding(null);
}, missing:()=>{setPorts([]);setBinding(null);}};
window.proof=proof;
const confirm=port=>{proof.requests.push(serialDmxConfirmationRequest(port));setBinding({state:'selected_and_present',selected:{...port,port_name:port.name}});};
render(()=><main class="app" style="padding:24px;overflow:auto"><DmxOutputConfigPanel
 output={{protocol:'ArtNet',target_ip:'127.0.0.1',port:6454,universe:0,serial_port:'',enabled:true}}
 serialPorts={ports()} serialDmxMachineBindingStatus={binding()}
 showSerialDmxSafetyBlackoutRouteStatus={{active:false,faulted:false,workerShutdownCompleted:true}}
 safetyBlackoutEngaged={true} showDmxPreparationBusy={false} showDmxPreparationStage={null}
 onPrepareShowDmx={port=>proof.requests.push(serialDmxConfirmationRequest(port))}
 onConfirmSerialDmxMachineBinding={confirm} onEnableStagedShowArtNetLoopbackRoute={()=>{}}
 onEnableShowSerialDmxSafetyBlackoutRoute={()=>{}} onStopShowSerialDmxSafetyBlackoutRoute={()=>{}}
/></main>,document.body);
proof.platform('mac');
`);
  const reservation=createTcpServer();
  await new Promise(r=>reservation.listen(0,'127.0.0.1',r));
  const port=reservation.address().port;
  await new Promise((r,j)=>reservation.close(e=>e?j(e):r()));
  server=await createServer({root,server:{host:'127.0.0.1',port,strictPort:true},logLevel:'error',
    plugins:[{name:'usb-dmx-proof',configureServer(s){s.middlewares.use((req,res,next)=>{
      if(!req.url?.startsWith('/usb-dmx-proof'))return next();
      res.setHeader('Content-Type','text/html');
      res.end(`<html><head><title>USB-DMX proof</title><link rel="icon" href="data:,"></head><body><script type="module" src="/${fixtureName}"></script></body></html>`);
    });}}]});
  await server.listen();
  const executablePath=[process.env.CHROME_PATH,'C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].filter(Boolean).find(existsSync);
  browser=await chromium.launch({headless:true,...(executablePath?{executablePath}:{})});
  const page=await browser.newPage(), errors=[];
  page.on('pageerror',e=>errors.push(String(e)));
  page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
  const results=[];
  for(const width of [1920,1280])for(const platform of ['mac','macusb','windows']) {
    await page.setViewportSize({width,height:900});
    await page.goto(`http://127.0.0.1:${port}/usb-dmx-proof`);
    assert.equal(await page.title(),'USB-DMX proof');
    await page.evaluate(p=>window.proof.platform(p),platform);
    const select=page.locator('#show-usb-dmx-device');
    await select.waitFor();
    assert.match(await select.inputValue(),platform==='mac'?/^\/dev\/cu\.usbserial-A/:platform==='macusb'?/^usb-ftdi:\/\/000000010000abcd/:/^COM4/);
    assert.equal(await select.locator('option').nth(2).evaluate(option=>option.disabled),true, await select.innerHTML());
    await page.locator('[data-io-control="dmx-prepare-show-dmx"]').click();
    const request=await page.evaluate(()=>window.proof.requests.at(-1));
    assert.equal(request.macosDeviceInstanceId,platform!=='windows'?'ioreg:000000010000abcd':null);
    assert.equal(request.windowsDeviceInstanceId,platform==='windows'?'FTDIBUS-A':null);
    await page.locator('[data-io-disclosure="dmx-individual-diagnostics"] > summary').click();
    await page.locator('[data-io-disclosure="dmx-usb-manual-controls"] > summary').click();
    await page.locator('[data-io-control="dmx-confirm-serial-machine-binding"]').click();
    assert.equal(await page.locator('[data-io-control="dmx-enable-show-serial-dmx-safety-blackout-route"]').isEnabled(),true);
    await page.screenshot({path:resolve(artifactDir,`${platform}-${width}.png`),fullPage:true});
    assert.equal(await page.locator('vite-error-overlay').count(),0);
    await page.evaluate(()=>window.proof.missing());
    assert.equal(await select.inputValue(),'');
    assert.equal(await page.locator('[data-io-control="dmx-confirm-serial-machine-binding"]').isDisabled(),true);
    assert.equal(await page.locator('[data-io-control="dmx-enable-show-serial-dmx-safety-blackout-route"]').isDisabled(),true);
    results.push({platform,width,request,missingDeviceBlocked:true});
  }
  assert.deepEqual(errors,[]);
  await writeFile(resolve(artifactDir,'browser-report.json'),JSON.stringify({browserPlugin:'unavailable',softwareOnly:true,results,errors},null,2)+'\n');
  console.log('USB-DMX platform browser: PASS (Mac VCP/native USB/Windows selection, exact confirmation payload, missing-device rejection, 1920/1280px)');
} finally {
  await browser?.close();await server?.close();await unlink(fixturePath).catch(()=>{});
}

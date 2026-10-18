import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { resolve } from "node:path";
import { writeFile, unlink } from "node:fs/promises";
import { existsSync } from "node:fs";
import { createServer } from "vite";

// The network universes are row groups of the one DMX universes table
// (DmxOutputConfigPanel). Every callback is a stand-in: no Tauri, no hardware.
const require = createRequire(import.meta.url);
let chromium;
try { ({chromium} = require("playwright")); }
catch { ({chromium} = require(resolve(homedir(), ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright"))); }
const root = resolve(import.meta.dirname, ".."), name = ".network-routes-proof-" + process.pid + ".tsx", fixture = resolve(root, name);
let browser, server;
try {
  await writeFile(fixture, `
import {render} from 'solid-js/web';
import {createSignal} from 'solid-js';
import {DmxOutputConfigPanel} from './src/components/DmxOutputConfigPanel';
import './src/theme.css';import './src/styles.css';
const primary={enabled:true,protocol:'ArtNet',target_ip:'127.0.0.1',port:6454,universe:0,serial_port:'',serial_baud_rate:57600};
const [routes,setRoutes]=createSignal([primary]),[epoch,setEpoch]=createSignal(1);
window.__networkProof={calls:[],replace(rows){setRoutes([primary,...rows]);},newProject(){setEpoch(epoch()+1);setRoutes([primary]);}};
const none=()=>{};
render(()=><div style={{height:'100vh',overflow:'auto',padding:'12px'}}><DmxOutputConfigPanel
 output={primary} serialPorts={[]} serialDmxMachineBindingStatus={null}
 showSerialDmxSafetyBlackoutRouteStatus={{active:false,faulted:false,workerShutdownCompleted:true}}
 safetyBlackoutEngaged={false} showDmxPreparationBusy={false} showDmxPreparationStage={null}
 onPrepareShowDmx={none} onEnableStagedShowArtNetLoopbackRoute={none} onConfirmSerialDmxMachineBinding={none}
 onEnableShowSerialDmxSafetyBlackoutRoute={none} onStopShowSerialDmxSafetyBlackoutRoute={none}
 routes={routes()} routeTelemetry={[]} projectEpoch={epoch()}
 onApplyNetworkRoutes={async next=>{window.__networkProof.calls.push(structuredClone(next));setRoutes(next);}}
 usbDmxUniverses={{bindings:[],cables:[],persistenceError:null}} usbDmxUniversesStale={false}
 onRefreshUsbDmxUniverses={none} onRescanSerialPorts={none} onBindUsbDmxUniverse={none}
 onClearUsbDmxUniverse={none} onStartUsbDmxUniverse={none} onStopUsbDmxUniverse={none}
/></div>,document.body);
`);
  server = await createServer({root, server: {host: "127.0.0.1", port: 0}, plugins:[{name:"network-route-fixture",configureServer(vite){vite.middlewares.use((request,response,next)=>{if(request.url?.split("?")[0]!=="/network-routes-proof")return next();response.setHeader("Content-Type","text/html");response.end('<html><head><link rel="icon" href="data:,"></head><body><script type="module" src="/'+name+'"></script></body></html>');});}}]});
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
  const page = await browser.newPage(); page.on("pageerror",error=>console.log("PAGE ERROR",error.message));
  const open = async () => {await page.goto(url+"/network-routes-proof");await page.locator('[data-network-dmx-routes]').waitFor({state:'attached'});};
  const addArtNet = async () => {
    await page.locator('[data-dmx-universe-add-transport]').selectOption('ArtNet');
    await page.locator('[data-dmx-universe-add-button]').click();
  };
  for (const viewport of [{width:1920,height:1080},{width:1920,height:1032},{width:2048,height:1152},{width:1366,height:768},{width:1280,height:720}]) {
    await page.setViewportSize(viewport); await open();
    await addArtNet(); await addArtNet();
    const fields = page.locator('[data-network-dmx-universe]');
    assert.equal(await fields.nth(0).inputValue(), '1'); assert.equal(await fields.nth(1).inputValue(), '2');
    assert.equal(await page.locator('[data-network-dmx-route]').nth(1).locator('.ioConnectionState').innerText(), 'Not applied');
    await fields.nth(1).fill('1'); await page.locator('[data-network-dmx-save]').click();
    await page.getByRole('alert').waitFor(); assert.equal(await page.evaluate(() => window.__networkProof.calls.length),0);
    await fields.nth(1).fill('2'); await page.locator('[data-network-dmx-save]').click();
    await page.waitForFunction(() => window.__networkProof.calls.length === 1);
    assert.deepEqual(await page.evaluate(() => window.__networkProof.calls[0].map(row => row.universe)), [0,1,2]);
    assert(await page.locator('[data-network-dmx-save]').isDisabled());
    assert.equal(await page.locator('[data-network-dmx-route]').nth(1).locator('.ioConnectionState').innerText(), 'Enabled',
      'an applied row shows its live state, not "Not applied"');
    assert(await page.locator('.dmxOutputConfigPanel').evaluate(panel => [...panel.querySelectorAll('table input:not([type="checkbox"]),table select,table button')].every(e => {
      const b=e.getBoundingClientRect(),p=e.closest('td,th').getBoundingClientRect(); return b.left>=p.left-1&&b.right<=p.right+1;
    })), 'universe table fields overflow their cells at '+viewport.width);
    assert(await page.locator('.dmxOutputConfigPanel').evaluate(panel => panel.querySelector('table').getBoundingClientRect().right <= panel.getBoundingClientRect().right + 1),
      'the universes table must fit its panel at '+viewport.width);
  }
  await open(); await addArtNet();
  await page.evaluate(() => window.__networkProof.replace([{enabled:true,protocol:'Sacn',target_ip:'multicast',port:5568,universe:2,serial_port:'',serial_baud_rate:57600}]));
  await page.getByText('Routes changed while editing. Reload routes before applying.',{exact:true}).waitFor();
  assert(await page.locator('[data-network-dmx-save]').isDisabled());
  await page.getByRole('button',{name:'Reload routes',exact:true}).click();
  assert.equal(await page.locator('[data-network-dmx-universe]').inputValue(),'2');
  await page.locator('[data-network-dmx-destination]').fill('127.0.0.1');
  await page.evaluate(() => window.__networkProof.newProject());
  await page.waitForFunction(() => document.querySelectorAll('[data-network-dmx-route]').length === 0);
  assert(await page.locator('[data-network-dmx-save]').isDisabled());
  await page.evaluate(() => window.__networkProof.replace(Array.from({length:13},(_,i)=>({enabled:true,protocol:'ArtNet',target_ip:'127.0.0.1',port:16454,universe:i+1,serial_port:'',serial_baud_rate:57600}))));
  assert.equal(await page.locator('[data-network-dmx-route]').count(),5);
  await page.getByRole('button',{name:'Next',exact:true}).click(); await page.getByRole('button',{name:'Next',exact:true}).click();
  assert.equal(await page.locator('[data-network-dmx-route]').count(),3);
  await page.locator('[data-network-dmx-universe]').last().fill('20');
  await page.locator('[data-network-dmx-save]').click();
  await page.waitForFunction(() => window.__networkProof.calls.length === 1);
  assert.deepEqual(await page.evaluate(() => window.__networkProof.calls[0].map(r=>r.universe)), [0,1,2,3,4,5,6,7,8,9,10,11,12,20]);
  console.log('DMX universes table (network rows): 5 viewports, one Add row, duplicate rejection, exact apply, applied-vs-draft state, conflict, new-project reset and pagination PASS');
} finally {
  await browser?.close(); await server?.close(); await unlink(fixture).catch(()=>{});
}

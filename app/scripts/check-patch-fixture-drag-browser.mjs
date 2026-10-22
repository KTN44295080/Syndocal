// Browser plugin unavailable; use Playwright and the real Solid patch panel.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { homedir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { mkdir, writeFile, unlink } from 'node:fs/promises';
import { createServer } from 'vite';
import { execFileSync } from 'node:child_process';
const require=createRequire(import.meta.url);
const {chromium}=require(resolve(homedir(),'.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright'));
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const fixtureName=`.patch-drag-proof-${process.pid}.tsx`,fixturePath=resolve(root,fixtureName);
const htmlName=`.patch-drag-proof-${process.pid}.html`,htmlPath=resolve(root,htmlName);
const baselineName=`.patch-drag-baseline-${process.pid}.tsx`,baselinePath=resolve(root,'src/components',baselineName);
const artifact=resolve(process.env.SYNDOCAL_PATCH_ARTIFACT_DIR??resolve(root,'../target/qa/patch-bulk-alpha74'));
const baseline=process.argv.includes('--baseline');
let server,browser;
try {
 await mkdir(artifact,{recursive:true});
 if(baseline) await writeFile(baselinePath,execFileSync('git',['show','efd8374b5e19e054dd7f60c9531c832e8d881e78:app/src/components/DmxPatchMapPanel.tsx'],{cwd:root,encoding:'utf8'}));
 await writeFile(fixturePath,`
import {createSignal,createMemo} from 'solid-js';import {render} from 'solid-js/web';
import {DmxPatchMapPanel} from '/src/components/${baseline?baselineName:'DmxPatchMapPanel'}';import '/src/theme.css';import '/src/styles.css';
const fixture=(id,address,label)=>({id,address,label,universe:1,controls:[{attribute:'Dimmer',offsets:[8]}],manufacturer:'Generic',profile_name:'8ch'});
const [fixtures,setFixtures]=createSignal([fixture(1,1,'Front'),fixture(2,17,'Back'),fixture(3,65,'Fixed')]);
const [selected,setSelected]=createSignal(null),[hover,setHover]=createSignal(null),[drag,setDrag]=createSignal(null),[mode,setMode]=createSignal('grid');
const [revealRequest,setRevealRequest]=createSignal();
const map=createMemo(()=>({universe:1,used:24,free:488,largestFree:440,segments:fixtures().map(f=>({fixture:f,start:f.address,end:f.address+7,left:(f.address-1)/512*100,width:8/512*100}))}));
const cells=createMemo(()=>Array.from({length:512},(_,i)=>{const channel=i+1,segment=map().segments.find(s=>channel>=s.start&&channel<=s.end)??null;return{channel,segment,isStart:segment?.start===channel,isSelected:segment?.fixture.id===selected(),plannedIndex:hover()===channel?0:null,plannedStart:hover()===channel,plannedConflict:false}}));
window.proof={drops:[],starts:[],selected,setSelected,setFixtures,fixtures,mode,setMode,setRevealRequest,clone:()=>setFixtures(fixtures().map(f=>({...f,controls:[...f.controls]})))};
render(()=> <div class="panel" style={{width:'100%',height:'850px'}}><DmxPatchMapPanel activeUniverse={1} universeOptions={[1]} viewMode={mode()} revealAddressRequest={revealRequest()} activeMap={map()} addressCells={cells()} universeMaps={[map()]} fixtureCount={3} selectedFixtureId={selected()}
plannedAddressSummary="" profileDragActive={false} profileDndStatus={null} fixtureDragActive={Boolean(drag())} fixtureDndStatus={hover()===null?null:'valid'}
onUniverse={()=>{}} onViewMode={setMode} onSelectFixture={f=>setSelected(f.id)} onAddressCell={c=>setSelected(c.segment?.fixture.id??null)}
onProfileDragHover={()=>{}} onProfileDragLeave={()=>{}} onProfileDrop={()=>{}}
onFixtureDragStart={(f,selection)=>{window.proof.starts.push({id:f.id,selection:selection?.map(f=>f.id)});setDrag(f)}} onFixtureDragEnd={()=>{setDrag(null);setHover(null)}} onFixtureDragHover={setHover} onFixtureDragLeave={()=>setHover(null)} onFixtureDrop={channel=>{window.proof.drops.push(channel);setDrag(null);setHover(null)}}/></div>,document.getElementById('root'));
`);
 server=await createServer({root,server:{host:'127.0.0.1',port:0},logLevel:'error'});
 await writeFile(htmlPath,`<html><link rel="icon" href="data:,"><div id="root"></div><script type="module" src="/${fixtureName}"></script></html>`);
 await server.listen();const port=server.httpServer.address().port;
 browser=await chromium.launch({channel:'msedge',headless:true});
 const page=await browser.newPage({viewport:{width:1600,height:1000}}),errors=[];
 page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
 await page.goto(`http://127.0.0.1:${port}/${htmlName}`);await page.waitForFunction(()=>Boolean(window.proof));await page.locator('[data-dmx-address="512"]').waitFor();
 assert.equal(await page.locator('[data-dmx-address]').count(),512);
 await page.evaluate(()=>window.originalCell=document.querySelector('[data-dmx-address="1"]'));
 await page.locator('[data-dmx-address="1"]').click();
 const stableAfterSelection=await page.evaluate(()=>window.originalCell===document.querySelector('[data-dmx-address="1"]'));
 const source=await page.locator('[data-dmx-address="1"]').boundingBox();const target=await page.locator('[data-dmx-address="33"]').boundingBox();
 await page.mouse.move(source.x+source.width/2,source.y+source.height/2);await page.mouse.down();
 await page.mouse.move(target.x+target.width/2,target.y+target.height/2,{steps:8});await page.mouse.up();
 const result=await page.evaluate(()=>({stableAfterDrag:window.originalCell===document.querySelector('[data-dmx-address="1"]'),lostDocument:!window.proof,drops:window.proof?.drops??[],starts:window.proof?.starts??[]}));
 await page.screenshot({path:resolve(artifact,baseline?'baseline.png':'grid.png'),fullPage:true});
 if(baseline){assert.equal(stableAfterSelection,false);assert.equal(result.stableAfterDrag,false);assert.deepEqual(result.drops,[33],"Browser HTML drag fallback still works despite recreated cells");}else{
  assert.equal(stableAfterSelection,true,'selection retains the captured address node');
  assert.equal(result.stableAfterDrag,true,'live drag previews retain the source node');
  assert.deepEqual(result.drops,[33],'trusted pointer drag reaches the address drop callback');
  await page.locator('[data-dmx-address="17"]').click({modifiers:['Control']});
  await page.locator('[aria-label="Patch selection and move"] output').filter({hasText:'2 selected'}).waitFor();
  const inside=await page.locator('[data-dmx-address="3"]').boundingBox();
  const destination=await page.locator('[data-dmx-address="99"]').boundingBox();
  await page.mouse.move(inside.x+inside.width/2,inside.y+inside.height/2);await page.mouse.down();
  await page.mouse.move(destination.x+destination.width/2,destination.y+destination.height/2,{steps:8});
  await page.evaluate(()=>window.proof.clone());await page.mouse.up();
  const group=await page.evaluate(()=>({last:window.proof.starts.at(-1),drop:window.proof.drops.at(-1)}));
  result.group=group;
  assert.deepEqual(group,{last:{id:1,selection:[1,2]},drop:97},'group and grabbed-channel offset survive snapshot refresh during drag');
  const count=await page.evaluate(()=>window.proof.drops.length);
  await page.mouse.move(source.x+source.width/2,source.y+source.height/2);await page.mouse.down();
  await page.mouse.move(destination.x+destination.width/2,destination.y+destination.height/2,{steps:8});
  await page.locator('[data-dmx-address="1"]').dispatchEvent('pointercancel',{pointerId:1,clientX:destination.x+5,clientY:destination.y+5});
  await page.mouse.up();
  assert.equal(await page.evaluate(()=>window.proof.drops.length),count,'pointer cancellation never applies a move');
  await page.evaluate(()=>window.proof.setMode('list'));
  await page.locator('.dmxPatchListItem').first().click();
  await page.locator('.dmxPatchListItem').nth(1).click({modifiers:['Shift']});
  assert.equal(await page.locator('.dmxPatchListItem[aria-pressed="true"]').count(),2);
  await page.getByLabel('Move selected fixtures to address',{exact:true}).fill('80');
  await page.getByRole('button',{name:'Move Selected',exact:true}).click();
  assert.deepEqual(await page.evaluate(()=>window.proof.starts.at(-1)),{id:1,selection:[1,2]});
  assert.equal(await page.evaluate(()=>window.proof.drops.at(-1)),80);
  result.listMove=true;result.pointerCancelNoApply=true;
  await page.screenshot({path:resolve(artifact,'list.png'),fullPage:true});
  // Source's sole Next Free action must reveal an off-screen address after
  // returning from List, and a repeated request must work after manual scroll.
  await page.evaluate(()=>{
    document.querySelector('.dmxPatchMap').style.height='260px';
    window.proof.setRevealRequest({universe:1,address:512});
    window.proof.setMode('grid');
  });
  const addressRevealed=()=>{
    const grid=document.querySelector('.dmxAddressGrid'),cell=grid?.querySelector('[data-dmx-address="512"]');
    if(!grid||!cell)return false;
    const g=grid.getBoundingClientRect(),c=cell.getBoundingClientRect();
    return grid.scrollHeight>grid.clientHeight&&grid.scrollTop>0&&cell.tabIndex===0
      &&c.top>=g.top&&c.bottom<=g.bottom&&c.left>=g.left&&c.right<=g.right;
  };
  try { await page.waitForFunction(addressRevealed,undefined,{timeout:5000}); }
  catch(error) {
    const failure=await page.evaluate(()=>{
      const map=document.querySelector('.dmxPatchMap'),grid=document.querySelector('.dmxAddressGrid'),cell=grid?.querySelector('[data-dmx-address="512"]');
      return {map:map?.getBoundingClientRect().toJSON(),grid:grid?.getBoundingClientRect().toJSON(),cell:cell?.getBoundingClientRect().toJSON(),
        scrollTop:grid?.scrollTop,scrollLeft:grid?.scrollLeft,scrollHeight:grid?.scrollHeight,clientHeight:grid?.clientHeight,tabIndex:cell?.tabIndex};
    });
    await writeFile(resolve(artifact,'address-reveal-failure.json'),JSON.stringify(failure,null,2)+'\n');
    await page.screenshot({path:resolve(artifact,'address-reveal-failure.png'),fullPage:true});
    throw error;
  }
  await page.evaluate(()=>document.querySelector('.dmxAddressGrid').scrollTo({top:0,left:0}));
  assert.equal(await page.evaluate(addressRevealed),false,'manual scroll hides the requested final address');
  await page.evaluate(()=>window.proof.setRevealRequest({universe:1,address:512}));
  await page.waitForFunction(addressRevealed);
  assert.equal(await page.evaluate(()=>window.proof.drops.length),count+1,'address reveal never applies an extra move');
  result.repeatedSourceAddressReveal=true;
 }
 assert.deepEqual(errors,[]);
 await writeFile(resolve(artifact,baseline?'baseline.json':'browser.json'),JSON.stringify({baseline,stableAfterSelection,...result,errors},null,2)+'\n');
 console.log(JSON.stringify({baseline,stableAfterSelection,...result,errors}));
}finally{await browser?.close();await server?.close();for(const file of [fixturePath,htmlPath,baselinePath])await unlink(file).catch(()=>{});}

import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {resolve} from 'node:path';
import ts from 'typescript';
const require=createRequire(import.meta.url),cache=new Map();
function load(file){const path=resolve(import.meta.dirname,'../src',file+'.ts');if(cache.has(path))return cache.get(path);const module={exports:{}};cache.set(path,module.exports);const js=ts.transpileModule(readFileSync(path,'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;new Function('require','module','exports',js)((name)=>name.startsWith('./')?load(name.slice(2)):require(name),module,module.exports);return module.exports;}
const {applyDmxUniverseTable,DmxUniverseApplyError}=load('applyDmxUniverseTable');
const port={name:'COM7',port_type:'USB',usb_vid:1027,usb_pid:24577,serial_number:'one',manufacturer:'FTDI',product:'Open DMX',windows_device_instance_id:'USB-one'};
const port2={...port,name:'COM8',serial_number:'two',windows_device_instance_id:'USB-two'};
const network={enabled:true,protocol:'ArtNet',target_ip:'10.0.0.1',port:6454,universe:0,serial_port:'',serial_baud_rate:57600};
const binding=(universe=0)=>({universe,state:'selected_and_present',identity:{...port,port_name:port.name},detail:''});
const cable=(patch={})=>({universe:0,active:true,faulted:false,workerShutdownCompleted:false,liveFrameQueued:true,zeroFrameQueued:false,zeroFramePhysicalWriteCompleted:false,...patch});
const empty={bindings:[],cables:[],persistenceError:null};
function fixture(patch={}){
 const calls=[];let current=true;let fail='';let staleAfter='';
 const op=name=>async()=>{calls.push(name);if(name===staleAfter)current=false;if(name===fail)throw Error('source '+name);};
 const ctx={network:[],usbStatus:empty,usbStatusStale:false,safetyBlackoutEngaged:true,legacyRouteActive:false,isCurrent:()=>current,
 ensureAuthority:op('authority'),stopUsb:u=>op('stop '+u)(),bindUsb:u=>op('bind '+u)(),applyNetwork:op('network'),startUsb:u=>op('start '+u)(),clearUsb:u=>op('clear '+u)(),refresh:op('refresh'),...patch};
 return {ctx,calls,setFail:value=>fail=value,setStaleAfter:value=>staleAfter=value};
}
const combined={network:[network],usb:[{universe:0,enabled:true,port}]};
let f=fixture();await applyDmxUniverseTable(combined,f.ctx);assert.deepEqual(f.calls,['authority','bind 0','network','start 0','refresh']);assert.equal(f.calls.filter(x=>x==='authority').length,1);
f=fixture();await applyDmxUniverseTable({network:[],usb:[{universe:0,enabled:true,port}]},f.ctx);assert.deepEqual(f.calls,['authority','bind 0','start 0','refresh']);
f=fixture({usbStatus:{...empty,bindings:[binding()],cables:[cable()]},network:[network]});await applyDmxUniverseTable({network:[network],usb:[{universe:0,enabled:false}]},f.ctx);assert.deepEqual(f.calls,['authority','stop 0','refresh']);
f=fixture({usbStatus:{...empty,bindings:[binding()],cables:[cable()]}});await applyDmxUniverseTable({network:[],usb:[]},f.ctx);assert.deepEqual(f.calls,['authority','stop 0','clear 0','refresh']);
f=fixture({usbStatus:{...empty,bindings:[binding()],cables:[cable()]}});await applyDmxUniverseTable({network:[network],usb:[{universe:0,enabled:true,port:port2}]},f.ctx);assert.deepEqual(f.calls,['authority','stop 0','clear 0','bind 0','network','start 0','refresh']);
for(const plan of [{network:[],usb:[]},{network:[],usb:[{universe:0,enabled:false}]},{network:[],usb:[{universe:0,enabled:true,port:port2}]}]){
 f=fixture({safetyBlackoutEngaged:false,usbStatus:{...empty,bindings:[binding()],cables:[cable()]}});await assert.rejects(applyDmxUniverseTable(plan,f.ctx),/blackout/);assert.deepEqual(f.calls,['refresh']);
}
for(const patch of [{usbStatusStale:true},{usbStatus:null},{legacyRouteActive:true}]){f=fixture(patch);await assert.rejects(applyDmxUniverseTable(combined,f.ctx),/Refresh|previous USB/);assert.deepEqual(f.calls,['refresh']);}
for(const failure of ['authority','bind 0','network','start 0']){
 f=fixture();f.setFail(failure);await assert.rejects(applyDmxUniverseTable(combined,f.ctx),err=>{assert(err instanceof DmxUniverseApplyError);assert.match(String(err.cause),new RegExp('source '+failure));assert.match(err.message,/Confirmed completed stages:/);assert(!err.completedStages.includes(failure));return true;});
 const expected=['authority','bind 0','network','start 0'];assert.deepEqual(f.calls,[...expected.slice(0,expected.indexOf(failure)+1),'refresh']);
}
for(const staleStage of ['authority','bind 0']){f=fixture();f.setStaleAfter(staleStage);await assert.rejects(applyDmxUniverseTable(combined,f.ctx),err=>{assert.match(err.message,/project changed/);assert(err.completedStages.includes(staleStage==='authority'?'output authority':'bind USB universe 0'));return true;});assert.deepEqual(f.calls,staleStage==='authority'?['authority']:['authority','bind 0']);}
f=fixture();await applyDmxUniverseTable({network:[],usb:[]},f.ctx);assert.deepEqual(f.calls,[]);
f=fixture({network:[network],usbStatus:{...empty,bindings:[binding()],cables:[cable()]},safetyBlackoutEngaged:false});await applyDmxUniverseTable({network:[{...network}],usb:[{universe:0,enabled:true,port}]},f.ctx);assert.deepEqual(f.calls,[]);
f=fixture();await assert.rejects(applyDmxUniverseTable({network:[],usb:[{universe:0,enabled:true,port},{universe:1,enabled:true,port}]},f.ctx),/own device/);assert.deepEqual(f.calls,['refresh']);
f=fixture({usbStatus:{...empty,bindings:[binding()],cables:[cable({active:false,faulted:true,workerShutdownCompleted:false,liveFrameQueued:false})]}});await assert.rejects(applyDmxUniverseTable({network:[],usb:[{universe:0,enabled:true}]},f.ctx),/finish stopping/);assert.deepEqual(f.calls,['refresh']);
f=fixture();f.ctx.refresh=async()=>{throw Error('refresh secondary');};f.setFail('network');await assert.rejects(applyDmxUniverseTable(combined,f.ctx),err=>String(err.cause).includes('source network'));
console.log('DMX Apply orchestration: combined/empty-network USB0, stop/rebind/start/clear ordering, preflight blackout/stale/legacy/device fences, partial failures, project changes, no-op, one authority, preserved source errors passed.');

f=fixture({usbStatus:{...empty,bindings:[binding()],cables:[cable()]}});
await applyDmxUniverseTable({network:[],usb:[{universe:1,enabled:true,port}]},f.ctx);
assert.deepEqual(f.calls,['authority','stop 0','clear 0','bind 1','start 1','refresh']);
f=fixture({usbStatus:{...empty,bindings:[binding(),{...binding(1),identity:{...port2,port_name:port2.name}}],cables:[cable(),cable({universe:1})]}});
await applyDmxUniverseTable({network:[],usb:[{universe:0,enabled:true,port:port2},{universe:1,enabled:true,port}]},f.ctx);
assert.deepEqual(f.calls,['authority','stop 0','stop 1','clear 0','clear 1','bind 0','bind 1','start 0','start 1','refresh']);
console.log('USB cable transfers/swaps clear stopped assignments before new bindings: PASS');

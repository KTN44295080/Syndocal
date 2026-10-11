import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/continuousFixtureFader.ts',import.meta.url),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText;
const {createContinuousFixtureFader}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
let transactions=0,values=[],unblock;
const controller=createContinuousFixtureFader(async(command,args)=>{
 assert.equal(command,'stream_fixture_attribute_v1');transactions++;
 await args.__fixtureFaderStream(async write=>{values.push(write.value);await new Promise(resolve=>unblock=resolve);});
});
const promises=[controller.enqueue('7:Dimmer',{targets:[{fixture_id:7,attribute:'Dimmer'}],value:0})];
const awaitCount=async count=>{const deadline=Date.now()+3000;while(values.length<count&&Date.now()<deadline)await wait(1);assert.equal(values.length,count);};
await awaitCount(1);
for(let burst=0;burst<6;burst++){
 for(let value=burst*10+1;value<=burst*10+9;value++)promises.push(controller.enqueue('7:Dimmer',{targets:[{fixture_id:7,attribute:'Dimmer'}],value}));
 unblock();await awaitCount(burst+2);
}
unblock();await Promise.all(promises);
assert.equal(transactions,1,'one drag has one history transaction');
assert.deepEqual(values,[0,9,19,29,39,49,59],'each pending burst retains only its latest value without losing intermediate live progress or the final value');
let entered,release;
const blocked=new Promise(resolve=>release=resolve);
const enteredPromise=new Promise(resolve=>entered=resolve);
let nextValues=[];
const cancellable=createContinuousFixtureFader(async(_command,args)=>{
  await args.__fixtureFaderStream(async write=>{
    nextValues.push(write.value);
    if(write.value===1){entered();await blocked;}
  });
});
const old=cancellable.enqueue('7:Dimmer',{targets:[{fixture_id:7,attribute:'Dimmer'}],value:1});
const rejection=assert.rejects(old,/project change/);
await enteredPromise;
cancellable.cancel();
const fresh=cancellable.enqueue('7:Dimmer',{targets:[{fixture_id:7,attribute:'Dimmer'}],value:99});
release();await rejection;await fresh;
assert.deepEqual(nextValues,[1,99],'new-project input survives cancellation of the old gesture');
let faultTransactions=0;
const failing=createContinuousFixtureFader(async()=>{faultTransactions++;throw Error('native rejected');});
await assert.rejects(failing.enqueue('x',{targets:[],value:9}),/native rejected/);
await wait(0);
await assert.rejects(failing.enqueue('x',{targets:[],value:10}),/native rejected/);
assert.equal(faultTransactions,2,'failure releases the controller for a new gesture');
let unblockOrder, enteredOrder; const ordered=[], gate=new Promise(resolve=>enteredOrder=resolve);
const mixed=createContinuousFixtureFader(async(_cmd,args)=>args.__fixtureFaderStream(async write=>{ordered.push(write.value);if(write.value===0){enteredOrder();await new Promise(resolve=>unblockOrder=resolve);}}));
const pendingOrder=[mixed.enqueue('block',{targets:[{fixture_id:1,attribute:'Dimmer'}],value:0})];await gate;
pendingOrder.push(mixed.enqueue('group',{targets:[{fixture_id:1,attribute:'Dimmer'},{fixture_id:2,attribute:'Dimmer'}],value:10}));
pendingOrder.push(mixed.enqueue('1:Dimmer',{targets:[{fixture_id:1,attribute:'Dimmer'}],value:20}));
pendingOrder.push(mixed.enqueue('group',{targets:[{fixture_id:1,attribute:'Dimmer'},{fixture_id:2,attribute:'Dimmer'}],value:30}));
unblockOrder();await Promise.all(pendingOrder);assert.deepEqual(ordered,[0,20,30],'latest overlapping group write follows earlier fixture write');
// A continuous gesture rotates before the native 512-sample bound and settles completed input.
let rotations=0,sampleCount=0;const accumulated=[];let rotating;
rotating=createContinuousFixtureFader(async(_command,args)=>{rotations++;await args.__fixtureFaderStream(async write=>{sampleCount++;if(sampleCount<520)accumulated.push(rotating.enqueue('next',{targets:[{fixture_id:1,attribute:'Dimmer'}],value:sampleCount}));});});
accumulated.push(rotating.enqueue('first',{targets:[{fixture_id:1,attribute:'Dimmer'}],value:0}));await accumulated[0];
while(sampleCount<520)await wait(1);await Promise.all(accumulated);assert.equal(rotations,2);assert.equal(sampleCount,520);
console.log(JSON.stringify({passed:true,transactions,intermediateValues:values.length,finalValue:values.at(-1),cancelAndRestart:true}));

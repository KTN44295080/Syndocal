import assert from 'node:assert/strict';
import {flushProjectControlMappingsAuthorityBridge as flush} from '../src/projectControlMappingsAuthorityBridge.ts';
import {createProjectControlMappingsFlusher} from '../src/createProjectControlMappingsFlusher.ts';
import {createProjectAuthoritySyncState} from '../src/projectAuthority.ts';
const token={project_epoch:1,project_revision:2,checkpoint_hash:'b'.repeat(64)};
const defer=()=>{let resolve;const promise=new Promise(r=>resolve=r);return {promise,resolve}};
const tick=()=>new Promise(resolve=>setImmediate(resolve));
function harness(){let sync={...createProjectAuthoritySyncState(),identityGeneration:1,localGeneration:1},inFlight=false,pending=null,calls=0;const first=defer(),ack=defer();const options={cancelTimer:()=>false,state:()=>({disposed:false,authorityReady:true,sync,mappingSyncInFlight:inFlight,timerFlushOwner:null}),persist:()=>{if(pending)return pending;inFlight=true;calls++;const initial=calls===1;pending=(initial?first.promise:ack.promise).then(()=>{inFlight=false;pending=null;if(!initial)sync={...sync,persistedGeneration:sync.localGeneration};return {kind:initial?'retryable':'acknowledged',authority:token}});return pending}};return {options,first,ack,calls:()=>calls,replace:()=>{sync={...sync,identityGeneration:2}},clean:()=>{sync={...sync,persistedGeneration:sync.localGeneration}}}}
// Reproduce the real failure: Learn opens R2, and New mistakes that shared R2
// for a foreign write and returns untrusted after the same successful R1.
let h=harness();let learn=flush(h.options),fresh=flush(h.options);h.first.resolve();await tick();assert.equal(h.calls(),2);assert.equal((await fresh).trusted,false);h.ack.resolve();assert.equal((await learn).trusted,true);
// The production flusher gives both consumers one owner and one bounded replay.
h=harness();let run=createProjectControlMappingsFlusher(()=>flush(h.options));learn=run();fresh=run();await tick();h.first.resolve();await tick();assert.equal(h.calls(),2);h.ack.resolve();const good=await Promise.all([learn,fresh]);assert.ok(good.every(r=>r.trusted));assert.deepEqual(good[0].ownAcknowledgements,[token]);assert.deepEqual(good[1].ownAcknowledgements,[token]);assert.equal(h.calls(),2);
// A real replacement remains untrusted, with no replay into the replacement.
h=harness();run=createProjectControlMappingsFlusher(()=>flush(h.options));learn=run();fresh=run();await tick();h.replace();h.first.resolve();const stale=await Promise.all([learn,fresh]);assert.ok(stale.every(r=>!r.trusted));assert.equal(h.calls(),1);h.clean();assert.equal((await run()).trusted,true);
// A thrown flush clears ownership; it is never cached as the next project's ACK.
let attempts=0;run=createProjectControlMappingsFlusher(async()=>{attempts++;if(attempts===1)throw Error('Rejected');return {trusted:true,ownAcknowledgements:[]}});const one=run(),two=run();assert.ok((await Promise.allSettled([one,two])).every(r=>r.status==='rejected'));assert.equal(attempts,1);assert.equal((await run()).trusted,true);assert.equal(attempts,2);
console.log('PASS: actual parallel-flush failure reproduced; shared Learn/New ACK replay succeeds; replacement and rejection stay fenced');

import assert from 'node:assert/strict';
import { projectControlMappingsCasRecoveryIsRetryable as retry } from '../src/projectControlMappingsCasRecovery.ts';
import { flushProjectControlMappingsAuthorityBridge as flush } from '../src/projectControlMappingsAuthorityBridge.ts';
import { createProjectAuthoritySyncState } from '../src/projectAuthority.ts';
const before={project_epoch:2,project_revision:3,checkpoint_hash:'a'.repeat(64)};
const after={...before,project_revision:4,checkpoint_hash:'b'.repeat(64)};
const state={error:'Project mappings are stale (expected epoch 2 revision 3, current epoch 2 revision 4)',expected:before,recovered:after,originalIdentity:1,currentIdentity:1,responseIsCurrent:true,baselineSignature:'mapping-A',recoveredSignature:'mapping-A',mappingReplacementGeneration:2,observedMappingReplacementGeneration:2};
assert.equal(retry(state),true);
assert.equal(retry({...state,error:'Error: '+state.error}),true);
for(const patch of [{error:'worker unavailable'},{recovered:{...after,project_epoch:3}},{currentIdentity:2},{responseIsCurrent:false},{baselineSignature:null},{recoveredSignature:'foreign edit'},{mappingReplacementGeneration:3},{recovered:before}])assert.equal(retry({...state,...patch}),false);
// A background ACK has marked the draft clean while its promise is still
// settling. An ordinary edit must join that promise rather than fail flush.
let pending=true;let joined=0;
const result=await flush({cancelTimer:()=>false,state:()=>({disposed:false,authorityReady:true,sync:createProjectAuthoritySyncState(),mappingSyncInFlight:pending,timerFlushOwner:null}),persist:async()=>{joined++;await Promise.resolve();pending=false;return{kind:'acknowledged',authority:after};}});
assert.equal(joined,1);assert.equal(result.trusted,true);assert.deepEqual(result.ownAcknowledgements,[after]);
console.log('PASS: mapping CAS recovery success/rejection and clean in-flight ACK join (13 assertions)');

import assert from 'node:assert/strict';
import {typescriptFixtureModuleUrl} from './typescript-fixture-module.mjs';
const {queryExpiredVideoOutputLease,releaseExpiredVideoOutputLease}=await import(await typescriptFixtureModuleUrl(new URL('../src/outputLeaseExpiryRecovery.ts',import.meta.url)));
const fence={process_incarnation:1,session_incarnation:2,project_epoch:3,project_revision:4,project_checkpoint_hash:'a'.repeat(64),project_publication_generation:5,output_epoch:6,output_generation:7,safety_blackout_epoch:8,safety_blackout_generation:9};
const authority={lease_id:'lease-0000000000000001',generation:4};
const standby={role:'Standby',effective_role:'Standby',desired_role:'Standby',persisted_role:'Standby',state:'Ready',epoch:5,generation:6,lighting_allowed:false,video_allowed:false,error:null};
const query=()=>({operationId:'syndocal.query.output.video.authoring.lease.v1',fence:{...fence},status:'expired',authority:{...authority},resources:['video'],projectMatches:true,managed:false});
function fixture(options={}) {
 const calls=[];let queryIndex=0;
 const invoke=async(command,args)=>{
  calls.push({command,args});
  if(command==='query_video_authoring_lease_authority_v1') {
   const value=query();options.query?.(value,queryIndex++);return value;
  }
  if(command==='set_output_ownership_role') {assert.deepEqual(args,{role:'Standby'});if(options.stopThrows)throw Error('retirement failed');return {...standby,...options.standby};}
  if(command==='query_output_control_authority_v1')return {operation_id:'syndocal.query.output.control.authority.v1',fence:{...fence,...options.dispatchFence}};
  if(command==='allocate_output_control_request_id_v1')return 101;
  if(command==='relinquish_output_lease_v2') {
   if(options.rejected)return {type:'rejected',rejection:{operation_id:args.request.operation_id,request_id:101,error:'stale_fence'}};
   const result={authority:{lease_id:authority.lease_id,generation:5},resources:['video'],phase:'unclaimed',outcome:'relinquished',audit_sequence:1,changes:[{lease_id:authority.lease_id,before_generation:4,after_generation:5,before_resources:['video'],after_resources:[],before_phase:'held_active',after_phase:'unclaimed'}]};
   options.result?.(result);
   return {type:'receipt',receipt:{operation_id:args.request.operation_id,request_id:101,shape_sha256:'a'.repeat(64),argument_fingerprint:'b'.repeat(64),audit_sequence:1,fence_before:{...fence},fence_after:{...fence},outcome:'no_op',lease_result:result}};
  }
  throw Error('Unexpected command: '+command);
 };
 return {invoke,calls};
}
const expected={authority:{...authority},fence:{...fence}};
const mutations=f=>f.calls.filter(c=>c.command==='set_output_ownership_role'||c.command==='relinquish_output_lease_v2');
const happy=fixture();
const result=await releaseExpiredVideoOutputLease(happy.invoke,expected);
assert.equal(result.lease_result.phase,'unclaimed');
assert.deepEqual(mutations(happy).map(c=>c.command),['set_output_ownership_role','relinquish_output_lease_v2']);
assert.deepEqual(mutations(happy)[1].args.request.action,{kind:'relinquish_output_lease',lease:authority});
assert.deepEqual(mutations(happy)[1].args.request.expected_fence,fence);
assert.equal(happy.calls.filter(c=>c.command==='query_output_lease_authority_v1').length,0,'do not advertise expiry as usable active authority');
for(const mutate of [q=>Object.assign(q,{status:'unavailable',authority:null,resources:[],projectMatches:null}),q=>q.status='held_active',q=>q.status='held_orphaned',q=>q.managed=true,q=>q.projectMatches=false,q=>q.resources=['lighting','video']]){
 const f=fixture({query:mutate});assert.equal(await queryExpiredVideoOutputLease(f.invoke),null);
 await assert.rejects(releaseExpiredVideoOutputLease(f.invoke,expected),/expired lease changed/);assert.equal(mutations(f).length,0);
}
for(const mutate of [q=>q.authority.generation++,q=>q.authority.lease_id='lease-0000000000000002',q=>q.fence.project_revision++,q=>q.fence.process_incarnation++]){
 const f=fixture({query:mutate});await assert.rejects(releaseExpiredVideoOutputLease(f.invoke,expected));assert.equal(mutations(f).length,0);
}
for(const invalid of [{state:'Failed'},{state:'Activating'},{video_allowed:true},{lighting_allowed:true},{persisted_role:'Video'},{epoch:null},{error:'retirement failed'}]){
 const f=fixture({standby:invalid});await assert.rejects(releaseExpiredVideoOutputLease(f.invoke,expected),/Standby could not be confirmed/);assert.equal(mutations(f).length,1);
}
const stopFailure=fixture({stopThrows:true});await assert.rejects(releaseExpiredVideoOutputLease(stopFailure.invoke,expected),/retirement failed/);assert.equal(mutations(stopFailure).length,1);
for(const mutate of [q=>q.authority.generation++,q=>q.managed=true,q=>q.projectMatches=false,q=>q.resources=['lighting','video'],q=>q.fence.project_revision++,q=>q.fence.output_generation++]){
 const f=fixture({query:(q,i)=>{if(i===1)mutate(q);}});await assert.rejects(releaseExpiredVideoOutputLease(f.invoke,expected));assert.equal(mutations(f).length,1,'changed authority after Standby cannot be relinquished');
}
for(const field of Object.keys(fence)){
 const changed=field==='project_checkpoint_hash'?'c'.repeat(64):fence[field]+1;
 const f=fixture({dispatchFence:{[field]:changed}});await assert.rejects(releaseExpiredVideoOutputLease(f.invoke,expected),/Output authority changed after Standby/);assert.equal(mutations(f).length,1,'every query fence field binds pre-dispatch admission');
}
for(const mutate of [r=>r.resources=['lighting','video'],r=>r.authority.generation=6,r=>r.changes[0].before_generation=3,r=>r.changes[0].before_resources=['lighting','video'],r=>r.changes[0].before_phase='held_orphaned']){
 const f=fixture({result:mutate});await assert.rejects(releaseExpiredVideoOutputLease(f.invoke,expected),/lease result was invalid|release could not be verified/);assert.equal(mutations(f).length,2);
}
const stale=fixture({rejected:true});await assert.rejects(releaseExpiredVideoOutputLease(stale.invoke,expected),/stale_fence/);assert.equal(mutations(stale).length,2,'native stale CAS has no fallback or new Enable');
console.log('Expired Video lease recovery: PASS (explicit Standby, exact retirement receipt, owner/project/scope/generation changes, all full-fence fields, native rejection; no automatic Enable).');

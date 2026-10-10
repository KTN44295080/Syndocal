import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {createHash,randomUUID} from 'node:crypto';
import {openNativeStdioSession} from './native-stdio-session.mjs';
const operation='syndocal.output.blackout.set.v2';
const token=b=>({project_epoch:b.project_epoch,project_revision:b.project_revision,checkpoint_hash:b.checkpoint_hash});
const checkpoint=b=>b.invoke('get_project_checkpoint',{midiMappings:[],oscMappings:[],dmxMappings:[]});
const grant=(b,o,operationId,capability='output')=>b.invoke('agent_authority_grant_v1',{principalId:o.principalId,
  principalIncarnation:o.principalIncarnation,grant:{adapter:'external_mcp',capability,operation_id:operationId,project_id:null}});
async function settle(mcp,receipt,id){
  const deadline=Date.now()+15000;
  while(receipt.status==='pending'&&Date.now()<deadline){await new Promise(r=>setTimeout(r,50));receipt=await mcp.call('syndocal_get_request_status',{requestId:id});}
  assert.equal(receipt.requestId,id);assert.equal(receipt.status,'completed',JSON.stringify(receipt));return receipt;
}
async function send(mcp,name,args){const id=args.requestId??randomUUID();return settle(mcp,await mcp.call(name,{...args,requestId:id}),id);}
function canonicalAccepted(receipt){assert.equal(receipt.result.ok,true,JSON.stringify(receipt));assert.equal(receipt.result.result.type,'receipt');return receipt.result.result.receipt;}
async function canonical(backend,mcp,operationId,action,id){
  const authority=await backend.invoke('query_output_control_authority_v1');
  return send(mcp,'syndocal_execute_control_plane',{operationId,request:{request:{operation_id:operationId,
    request_id:id,expected_fence:authority.fence,action}}});
}
async function noPhysicalTargets(backend){
  const p=await checkpoint(backend);
  assert.deepEqual(p.snapshot.fixtures,[]);assert.deepEqual(p.snapshot.video.outputs,[]);
  assert.ok(p.snapshot.dmx_outputs.every(route=>route.enabled===false),'no enabled DMX route may enter this native lane');
  return p;
}

// Actual public compatibility tool against its uniquely identified empty QA instance.
export async function nativeVideoBlackout(backend,options,checks){
  await backend.invoke('register_project_transaction_owner',{ownerId:`video-blackout-${randomUUID()}`});
  await noPhysicalTargets(backend);const closed=await backend.invoke('get_output_ownership_status');
  assert.equal(closed.lighting_allowed,false);assert.equal(closed.video_allowed,false);
  await backend.invoke('agent_authority_promote_v1',{principalId:options.principalId,principalIncarnation:options.principalIncarnation});
  for(const id of [operation,'syndocal.output.lease.acquire.v2','syndocal.output.lease.renew.v2','syndocal.output.ownership.arm.v2','syndocal.output.lease.relinquish.v2'])await grant(backend,options,id);
  const mcp=await openNativeStdioSession(options);let lease,armed=false,disarmed=false,failure,nextCanonicalId=3;
  const disarmOwnedProject=async()=>{
    const queryId='syndocal.query.project.replacement.authority.v1',newId='syndocal.project.new.v1';
    await grant(backend,options,queryId,'read');await grant(backend,options,newId,'file');
    const authority=await send(mcp,'syndocal_execute_control_plane',{operationId:queryId,request:{request:{}}});
    assert.equal(authority.result.ok,true,JSON.stringify(authority));
    const fresh=await send(mcp,'syndocal_execute_control_plane',{operationId:newId,request:{request:{schema_version:1,
      operation_id:newId,request_id:5,expected_fence:authority.result.result.fence,action:{kind:'new'}}}});
    assert.equal(fresh.result.ok,true,JSON.stringify(fresh));assert.equal(fresh.result.result.status,'receipt');
    const after=await backend.invoke('get_output_ownership_status');assert.equal(after.lighting_allowed,false);assert.equal(after.video_allowed,false);
    disarmed=true;
  };
  try{
    let project=token(await backend.invoke('get_project_authority_bundle'));
    const beforeRejected=await checkpoint(backend);
    const absent=await send(mcp,'syndocal_set_video_blackout',{enabled:true,expectedProject:project});
    assert.equal(absent.result.ok,false);assert.match(absent.result.error.message,/agent_video_blackout_active_both_lease_required/);
    assert.deepEqual(await backend.invoke('get_output_ownership_status'),closed);
    assert.deepEqual(await checkpoint(backend),beforeRejected,'missing lease must not mutate the project');
    checks.push({check:'external-legacy-video-blackout-missing-both-lease-refuses-without-acquire-or-arm',passed:true});
    const stale=await send(mcp,'syndocal_set_video_blackout',{enabled:false,expectedProject:{...project,project_revision:project.project_revision+1}});
    assert.equal(stale.result.ok,false);assert.match(stale.result.error.message,/agent_video_blackout_project_changed/);
    assert.deepEqual(await checkpoint(backend),beforeRejected,'stale project must not mutate the project');
    checks.push({check:'external-legacy-video-blackout-stale-project-refuses-without-effect',passed:true});
    lease=canonicalAccepted(await canonical(backend,mcp,'syndocal.output.lease.acquire.v2',{kind:'acquire_lease',role:'both'},1)).lease_result.authority;
    canonicalAccepted(await canonical(backend,mcp,'syndocal.output.ownership.arm.v2',{kind:'arm',role:'both',lease},2));
    armed=true;
    await noPhysicalTargets(backend);
    const active=await backend.invoke('get_output_ownership_status');assert.equal(active.lighting_allowed,true);assert.equal(active.video_allowed,true);
    checks.push({check:'external-empty-project-both-lease-and-arm-with-no-physical-targets-or-dialog',passed:true});
    for(const [enabled,outcome] of [[true,'applied'],[true,'no_op'],[false,'applied']]){
      // Explicit fixture-owned renewal keeps this test independent of host RPC
      // startup time. The Video BO tool itself never acquires, renews or arms.
      lease=canonicalAccepted(await canonical(backend,mcp,'syndocal.output.lease.renew.v2',
        {kind:'renew_lease',lease},nextCanonicalId++)).lease_result.authority;
      project=token(await backend.invoke('get_project_authority_bundle'));const before=await checkpoint(backend),requestId=randomUUID();
      const args={requestId,enabled,expectedProject:project};const receipt=await send(mcp,'syndocal_set_video_blackout',args);
      assert.equal(receipt.result.ok,true,JSON.stringify(receipt));const result=receipt.result;
      assert.equal(result.verification,'committed_project_state');assert.equal(result.receipt.outcome,outcome);
      assert.equal(result.video.blackout,enabled);assert.equal(result.video.authored_blackout,enabled);
      assert.equal(result.video.safety_blackout_engaged,before.snapshot.safety_blackout_engaged);
      const after=await checkpoint(backend);assert.equal(after.snapshot.video.blackout,enabled);
      assert.deepEqual(await mcp.call('syndocal_set_video_blackout',args),receipt);
      assert.deepEqual(await checkpoint(backend),after,'exact UUID replay must not republish');
      assert.deepEqual(await backend.invoke('get_output_ownership_status'),active,'Video BO never arms or changes gates');
      assert.equal(result.project.project_revision,project.project_revision+(outcome==='applied'?1:0));
      checks.push({check:`external-legacy-video-blackout-${enabled}-${outcome}-native-readback-and-exact-replay`,passed:true,
        requestId,outcome,revisionBefore:project.project_revision,revisionAfter:result.project.project_revision});
    }
    // Low canonical IDs share the GUI receipt origin. Native compatibility IDs
    // must not make the next ordinary control command stale or collide with it.
    for(const enabled of [true,false]){
      canonicalAccepted(await canonical(backend,mcp,operation,{kind:'set_blackout',target:'video',enabled,lease},nextCanonicalId++));
      assert.equal((await checkpoint(backend)).snapshot.video.blackout,enabled);
      assert.deepEqual(await backend.invoke('get_output_ownership_status'),active);
    }
    checks.push({check:'external-native-video-blackout-interleaves-with-low-canonical-gui-origin-identities',passed:true});
    canonicalAccepted(await canonical(backend,mcp,'syndocal.output.lease.relinquish.v2',{kind:'relinquish_output_lease',lease},nextCanonicalId++));lease=undefined;
    // Relinquish intentionally preserves the current output image/gates. Explicit
    // owned project replacement, rather than a fictitious implicit disarm, closes them.
    await disarmOwnedProject();
    // New restores authored default routes (enabled Art-Net), while its actual
    // output gates are disarmed. Assert that canonical cleanup contract rather
    // than treating an authored route flag as an open physical sender.
    const empty=await checkpoint(backend);
    assert.deepEqual(empty.snapshot.fixtures,[]);assert.deepEqual(empty.snapshot.video.outputs,[]);
    assert.equal(empty.snapshot.timeline.playing,false);
    checks.push({check:'external-video-blackout-owned-lease-relinquished-and-output-closed',passed:true});
  }catch(error){failure=error;throw error;}finally{
    const cleanupErrors=[];
    try{if(lease)canonicalAccepted(await canonical(backend,mcp,'syndocal.output.lease.relinquish.v2',{kind:'relinquish_output_lease',lease},nextCanonicalId++));}
    catch(error){cleanupErrors.push(error);}
    try{if(armed&&!disarmed)await disarmOwnedProject();}catch(error){cleanupErrors.push(error);}
    try{await mcp.close();}catch(error){cleanupErrors.push(error);}
    if(cleanupErrors.length)throw new AggregateError(failure?[failure,...cleanupErrors]:cleanupErrors,'Video BO acceptance/cleanup failed');
  }
}

export async function nativeVideoBlackoutRevocation(backend,options,checks,closeDirectory){
  // Legitimate canonical SaveAs/Acknowledge keeps graceful close protection enforced.
  const ids={authority:'syndocal.query.project.file.authority.v1',save:'syndocal.project.save_as.v1',ack:'syndocal.project.file.acknowledge.v1'};
  await backend.invoke('agent_authority_promote_v1',{principalId:options.principalId,principalIncarnation:options.principalIncarnation});
  for(const id of Object.values(ids))await grant(backend,options,id,id===ids.authority?'read':'file');
  const saving=await openNativeStdioSession(options);
  try{
    const call=async(operationId,request)=>{const r=await send(saving,'syndocal_execute_control_plane',{operationId,request:{request}});assert.equal(r.result.ok,true,JSON.stringify(r));return r.result.result;};
    const destination=path.join(closeDirectory,'empty-project.sdc'),authority=await call(ids.authority,{schema_version:1,operation_id:ids.save,destination});
    const request={schema_version:1,operation_id:ids.save,request_id:authority.next_request_id,expected_fence:authority.fence,
      expected_path_generation:authority.path_generation,expected_disposition_generation:authority.disposition_generation,
      destination:authority.destination,expected_target_sha256:authority.target_sha256};
    const saved=await call(ids.save,request);assert.equal(saved.phase,'succeeded');assert.equal(saved.target_path,authority.destination);
    assert.equal(saved.artifact_sha256,createHash('sha256').update(await fs.readFile(destination)).digest('hex'));
    assert.equal((await call(ids.ack,request)).phase,'acknowledged');
    checks.push({check:'external-video-blackout-owned-empty-project-saved-with-close-protection-enforced',passed:true});
  }finally{await saving.close();}
  const generation=await backend.invoke('agent_bridge_register_v1'),principalId=`video-blackout-revoke-${randomUUID()}`;
  const credentialFile=path.join(path.dirname(options.credentialFile),`${principalId}.credential`);let incarnation,mcp,written=false,revoked=false;
  try{
    const challenge=await backend.invoke('agent_authority_begin_pairing_v1',{principalId}),approval=await backend.invoke('agent_authority_approve_pairing_v1',{
      challengeId:challenge.challengeId,challenge:challenge.challenge});incarnation=approval.principalIncarnation;
    await fs.writeFile(credentialFile,approval.credential,{flag:'wx'});written=true;
    const own={...options,principalId,principalIncarnation:incarnation,credentialFile};
    await backend.invoke('agent_authority_promote_v1',{principalId,principalIncarnation:incarnation});await grant(backend,own,operation);
    mcp=await openNativeStdioSession(own);const requestId=randomUUID(),project=token(await backend.invoke('get_project_authority_bundle'));
    const before={project:await checkpoint(backend),output:await backend.invoke('get_output_ownership_status')};
    assert.equal((await mcp.call('syndocal_set_video_blackout',{requestId,enabled:true,expectedProject:project})).status,'pending');
    const claimed=await backend.invoke('agent_bridge_claim_v1',{rendererGeneration:generation,requestId});assert.equal(claimed.principalId,principalId);
    assert.deepEqual(claimed.params,{enabled:true,expectedProject:project});
    await backend.invoke('agent_authority_revoke_v1',{principalId,principalIncarnation:incarnation});revoked=true;
    const execute=()=>backend.evaluate(`window.__TAURI_INTERNALS__.invoke('agent_bridge_execute_native_v1',${JSON.stringify({rendererGeneration:generation,requestId})}).then(value=>({value}),error=>({error:String(error)}))`);
    assert.deepEqual(await execute(),{error:'agent_principal_revoked'});assert.deepEqual(await execute(),{error:'request_not_executable'});
    const denied=await mcp.call('syndocal_get_request_status',{requestId});assert.equal(denied.status,'rejected');assert.equal(denied.result,undefined);
    assert.deepEqual({project:await checkpoint(backend),output:await backend.invoke('get_output_ownership_status')},before);
    checks.push({check:'external-legacy-video-blackout-revoke-after-claim-consumes-native-request-without-effect',passed:true});
  }finally{
    await mcp?.close();if(incarnation!==undefined&&!revoked)await backend.invoke('agent_authority_revoke_v1',{principalId,principalIncarnation:incarnation});
    if(written)await fs.rm(credentialFile,{force:true});
  }
}

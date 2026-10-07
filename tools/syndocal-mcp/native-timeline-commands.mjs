import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {createHash,randomUUID} from 'node:crypto';
import {openNativeStdioSession} from './native-stdio-session.mjs';

const operations=[
  ['syndocal.runtime.timeline.transport.set_playing.v1','query_timeline_transport_authority_v1','live'],
  ['syndocal.runtime.timeline.loop.commit.v1','query_timeline_loop_runtime_authority_v1','runtime'],
  ['syndocal.runtime.timeline.follow.abort.v1','query_timeline_follow_abort_authority_v1','runtime'],
];
const body=(index,authority,id=1)=>({...authority,expected_fence:authority.fence,request_id:id,
  ...(index===0?{payload:{playing:false}}:index===1?{action:{kind:'set_enabled',enabled:false}}:{})});
// Authority bundles expose `fence`; command bodies must expose only expected_fence.
function request(index,authority,id=1){const value=body(index,authority,id);delete value.fence;return value;}
const checkpoint=backend=>backend.invoke('get_project_checkpoint',{midiMappings:[],oscMappings:[],dmxMappings:[]});
async function setup(backend){
  const ownerId=`timeline-commands-${randomUUID()}`;
  await backend.invoke('register_project_transaction_owner',{ownerId});
  const current=await backend.invoke('get_project_authority_bundle');
  await backend.invoke('new_project',{ownerId,expectedEpoch:current.project_epoch,
    expectedRevision:current.project_revision,expectedCheckpointHash:current.checkpoint_hash});
  const output=await backend.invoke('get_output_ownership_status');
  assert.equal(output.lighting_allowed,false);assert.equal(output.video_allowed,false);
}
async function settle(mcp,receipt,id){
  const until=Date.now()+15000;
  while(receipt.status==='pending'&&Date.now()<until){
    await new Promise(resolve=>setTimeout(resolve,50));receipt=await mcp.call('syndocal_get_request_status',{requestId:id});
  }
  assert.equal(receipt.requestId,id);assert.equal(receipt.status,'completed',JSON.stringify(receipt));return receipt;
}

// Actual empty/disarmed native instance, actual canonical bodies and real MCP.
export async function nativeTimelineCommands(backend,options,checks){
  await setup(backend);const authoredBefore=await checkpoint(backend),outputBefore=await backend.invoke('get_output_ownership_status');
  await backend.invoke('agent_authority_promote_v1',{principalId:options.principalId,principalIncarnation:options.principalIncarnation});
  for(const [operationId,,capability] of operations)await backend.invoke('agent_authority_grant_v1',{
    principalId:options.principalId,principalIncarnation:options.principalIncarnation,
    grant:{adapter:'external_mcp',capability,operation_id:operationId,project_id:null}});
  const mcp=await openNativeStdioSession(options);
  try{
    let serial=0;
    for(let index=0;index<operations.length;index++){
      const [operationId,query]=operations[index];const authority=await backend.invoke(query);
      const value=request(index,authority,++serial),requestId=randomUUID();
      const args={requestId,operationId,request:{request:value}};
      const receipt=await settle(mcp,await mcp.call('syndocal_execute_control_plane',args),requestId);
      assert.equal(receipt.result.ok,true,JSON.stringify(receipt));assert.equal(receipt.result.result.kind,'receipt');
      const runtimeBefore=(await backend.invoke('get_snapshot')).timeline_runtime;
      assert.deepEqual(await mcp.call('syndocal_execute_control_plane',args),receipt);
      assert.deepEqual((await backend.invoke('get_snapshot')).timeline_runtime,runtimeBefore);
      checks.push({check:`external-timeline-${index}-typed-success-and-exact-replay`,passed:true,
        operationId,outcome:receipt.result.result.result.outcome,requestId});

      const fresh=await backend.invoke(query),stale=request(index,fresh,++serial);
      const field=index===0?'source_runtime_generation':index===1?'source_loop_generation':'follow_generation';
      stale.expected_fence[field]++;
      const staleId=randomUUID(),failed=await settle(mcp,await mcp.call('syndocal_execute_control_plane',{
        requestId:staleId,operationId,request:{request:stale}}),staleId);
      assert.equal(failed.result.ok,false,JSON.stringify(failed));assert.equal(failed.result.result.kind,'rejected');
      // Transport/Loop compare the issued capability first (Forbidden). Follow
      // checks its current generation earlier (StaleFence). Preserve each
      // canonical operation's exact existing category, rather than accept either.
      assert.equal(failed.result.result.result.error.code,index===2?'stale_fence':'forbidden');
      checks.push({check:`external-timeline-${index}-typed-authority-fence-mismatch-refusal-is-not-success`,passed:true,operationId,
        code:failed.result.result.result.error.code});
    }
    // This private empty project has no A-B bounds. The real Engine refuses
    // enabling Loop; the adapter must preserve that typed refusal as false.
    const authority=await backend.invoke(operations[1][1]),value=request(1,authority,++serial);
    value.action.enabled=true;const requestId=randomUUID();
    const failed=await settle(mcp,await mcp.call('syndocal_execute_control_plane',{
      requestId,operationId:operations[1][0],request:{request:value}}),requestId);
    assert.equal(failed.result.ok,false);assert.equal(failed.result.result.kind,'rejected');
    assert.equal(failed.result.result.result.error.code,'publication_failed');
    checks.push({check:'external-timeline-loop-engine-refusal-is-not-success',passed:true,code:'publication_failed'});

    const oldTransport=await backend.invoke(operations[0][1]);
    const oldLoop=await backend.invoke(operations[1][1]);
    const playAuthority=await backend.invoke(operations[0][1]),play=request(0,playAuthority,++serial);
    play.payload.playing=true;const playId=randomUUID();
    const played=await settle(mcp,await mcp.call('syndocal_execute_control_plane',{
      requestId:playId,operationId:operations[0][0],request:{request:play}}),playId);
    assert.equal(played.result.ok,true);assert.equal(played.result.result.kind,'receipt');
    assert.equal(played.result.result.result.outcome,'applied');
    assert.ok(played.result.result.result.generation_after>play.expected_fence.source_runtime_generation);
    checks.push({check:'external-timeline-play-actual-publication-with-output-disarmed',passed:true,
      outcome:'applied',generationBefore:play.expected_fence.source_runtime_generation,
      generationAfter:played.result.result.result.generation_after});
    // These original, unmodified authorities really predate the published
    // transport successor. The Engine must reject both as StaleFence.
    for(const [index,authority] of [[0,oldTransport],[1,oldLoop]]){
      const staleId=randomUUID(),stale=request(index,authority,++serial);
      const rejected=await settle(mcp,await mcp.call('syndocal_execute_control_plane',{
        requestId:staleId,operationId:operations[index][0],request:{request:stale}}),staleId);
      assert.equal(rejected.result.ok,false,JSON.stringify(rejected));assert.equal(rejected.result.result.kind,'rejected');
      assert.equal(rejected.result.result.result.error.code,'stale_fence');
      checks.push({check:`external-timeline-${index}-real-successor-stale-authority-refusal-is-not-success`,passed:true,
        operationId:operations[index][0],code:'stale_fence'});
    }
    assert.deepEqual(await checkpoint(backend),authoredBefore);assert.deepEqual(await backend.invoke('get_output_ownership_status'),outputBefore);
  }finally{await mcp.close();}
}

// Last boundary before process shutdown. Retire the automatic renderer using
// the real native generation, then claim/revoke/execute in a deterministic order.
export async function nativeTimelineRevocation(backend,options,checks,closeDirectory){
  await setup(backend);
  // New establishes a valid Follow generation, but also an unsaved project.
  // Save that owned empty image through canonical File before the renderer is
  // retired. Normal close protection must succeed honestly, never be bypassed.
  const ids={authority:'syndocal.query.project.file.authority.v1',save:'syndocal.project.save_as.v1',ack:'syndocal.project.file.acknowledge.v1'};
  await backend.invoke('agent_authority_promote_v1',{principalId:options.principalId,principalIncarnation:options.principalIncarnation});
  for(const [operationId,capability] of [[ids.authority,'read'],[ids.save,'file'],[ids.ack,'file']])
    await backend.invoke('agent_authority_grant_v1',{principalId:options.principalId,principalIncarnation:options.principalIncarnation,
      grant:{adapter:'external_mcp',capability,operation_id:operationId,project_id:null}});
  const closeMcp=await openNativeStdioSession(options);
  try{
    const send=async(operationId,value)=>{const id=randomUUID();const receipt=await settle(closeMcp,
      await closeMcp.call('syndocal_execute_control_plane',{requestId:id,operationId,request:{request:value}}),id);
      assert.equal(receipt.result.ok,true,JSON.stringify(receipt));return receipt.result.result;};
    const destination=path.join(closeDirectory,'empty-project.sdc');
    const authority=await send(ids.authority,{schema_version:1,operation_id:ids.save,destination});
    const value={schema_version:1,operation_id:ids.save,request_id:authority.next_request_id,expected_fence:authority.fence,
      expected_path_generation:authority.path_generation,expected_disposition_generation:authority.disposition_generation,
      destination:authority.destination,expected_target_sha256:authority.target_sha256};
    const saved=await send(ids.save,value);
    assert.equal(saved.phase,'succeeded');assert.equal(saved.target_path,authority.destination);
    assert.equal(saved.artifact_sha256,createHash('sha256').update(await fs.readFile(destination)).digest('hex'));
    const acknowledged=await send(ids.ack,value);assert.equal(acknowledged.phase,'acknowledged');
    checks.push({check:'external-timeline-owned-empty-project-saved-before-normal-close',passed:true,
      artifactSha256:saved.artifact_sha256,closeProtectionBypassed:false});
  }finally{await closeMcp.close();}
  const generation=await backend.invoke('agent_bridge_register_v1');
  for(let index=0;index<operations.length;index++){
    const [operationId,query,capability]=operations[index],principalId=`timeline-revoke-${randomUUID()}`;
    const credentialFile=path.join(path.dirname(options.credentialFile),`${principalId}.credential`);
    let incarnation,mcp,written=false,revoked=false;
    try{
      const challenge=await backend.invoke('agent_authority_begin_pairing_v1',{principalId});
      const approval=await backend.invoke('agent_authority_approve_pairing_v1',{challengeId:challenge.challengeId,challenge:challenge.challenge});
      incarnation=approval.principalIncarnation;await fs.writeFile(credentialFile,approval.credential,{flag:'wx'});written=true;
      await backend.invoke('agent_authority_promote_v1',{principalId,principalIncarnation:incarnation});
      await backend.invoke('agent_authority_grant_v1',{principalId,principalIncarnation:incarnation,
        grant:{adapter:'external_mcp',capability,operation_id:operationId,project_id:null}});
      mcp=await openNativeStdioSession({...options,principalId,principalIncarnation:incarnation,credentialFile});
      const authority=await backend.invoke(query),value=request(index,authority,index+1),requestId=randomUUID();
      const before={project:await checkpoint(backend),runtime:(await backend.invoke('get_snapshot')).timeline_runtime,
        output:await backend.invoke('get_output_ownership_status')};
      const pending=await mcp.call('syndocal_execute_control_plane',{requestId,operationId,request:{request:value}});
      assert.equal(pending.status,'pending');
      const claimed=await backend.invoke('agent_bridge_claim_v1',{rendererGeneration:generation,requestId});
      assert.equal(claimed.principalId,principalId);assert.deepEqual(claimed.params.request,{request:value});
      await backend.invoke('agent_authority_revoke_v1',{principalId,principalIncarnation:incarnation});revoked=true;
      const execute=()=>backend.evaluate(`window.__TAURI_INTERNALS__.invoke('agent_bridge_execute_native_v1',${JSON.stringify({rendererGeneration:generation,requestId})}).then(value=>({value}),error=>({error:String(error)}))`);
      assert.deepEqual(await execute(),{error:'agent_principal_revoked'});
      assert.deepEqual(await execute(),{error:'request_not_executable'});
      const hidden=await mcp.call('syndocal_get_request_status',{requestId});assert.equal(hidden.status,'rejected');
      assert.equal(hidden.error,'agent_principal_revoked');assert.equal(hidden.result,undefined);
      assert.deepEqual({project:await checkpoint(backend),runtime:(await backend.invoke('get_snapshot')).timeline_runtime,
        output:await backend.invoke('get_output_ownership_status')},before);
      checks.push({check:`external-timeline-${index}-revoke-after-claim-consumes-immutable-request`,passed:true,
        operationId,executionError:'agent_principal_revoked',replayError:'request_not_executable',stateUnchanged:true});
    }finally{
      await mcp?.close();if(incarnation!==undefined&&!revoked)await backend.invoke('agent_authority_revoke_v1',{principalId,principalIncarnation:incarnation});
      if(written)await fs.rm(credentialFile,{force:true});
    }
  }
}

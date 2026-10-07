import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {createHash,randomUUID} from 'node:crypto';
import {openNativeStdioSession} from './native-stdio-session.mjs';

// Called only in the runner's verified empty, disarmed isolated QA instance.
export async function nativeCanonicalRiskAdmission(backend,options,checks) {
  const operations=[
    ['syndocal.effects.set_enabled.v1','r3'],['syndocal.cue_lists.reorder.v1','r3'],
    ['syndocal.cue_lists.rename.v1','r3'],['syndocal.cue_lists.delete.v1','r3'],['syndocal.scenes.create.v1','r3'],
    ['syndocal.runtime.timeline.transport.set_playing.v1','r2'],
    ['syndocal.runtime.timeline.loop.commit.v1','r1'],['syndocal.runtime.timeline.follow.abort.v1','r1'],
  ];
  const principalId=`native-risk-${randomUUID()}`;
  const credentialFile=path.join(path.dirname(options.credentialFile),`${principalId}.credential`);
  let principalIncarnation,mcp,credentialWritten=false;
  const state=async()=>({project:await backend.invoke('get_project_checkpoint',{midiMappings:[],oscMappings:[],dmxMappings:[]}),
    output:await backend.invoke('get_output_ownership_status')});
  const before=await state();assert.equal(before.output.lighting_allowed,false);assert.equal(before.output.video_allowed,false);
  try{
    const challenge=await backend.invoke('agent_authority_begin_pairing_v1',{principalId});
    const approval=await backend.invoke('agent_authority_approve_pairing_v1',{challengeId:challenge.challengeId,challenge:challenge.challenge});
    principalIncarnation=approval.principalIncarnation;
    await fs.writeFile(credentialFile,approval.credential,{flag:'wx'});credentialWritten=true;
    await backend.invoke('agent_authority_promote_v1',{principalId,principalIncarnation});
    mcp=await openNativeStdioSession({...options,principalId,principalIncarnation,credentialFile});
    const registry=await backend.invoke('get_control_plane_canonical_registry');
    for(const [operationId,risk] of operations){
      const operation=registry.canonical_operations.find(row=>row.operation_id===operationId);
      assert.equal(operation.risk,risk);assert.equal(operation.class,'mutation');
      assert.equal(operation.audit,risk==='r1'?'not_applicable':'immutable');
      await backend.invoke('agent_authority_grant_v1',{principalId,principalIncarnation,
        grant:{adapter:'external_mcp',capability:'read',operation_id:operationId,project_id:null}});
      const requestId=randomUUID();
      const receipt=await mcp.call('syndocal_execute_control_plane',{requestId,operationId,request:{}});
      assert.equal(receipt.requestId,requestId);assert.equal(receipt.status,'rejected',JSON.stringify(receipt));
      assert.equal(receipt.error,'agent_missing_grant');assert.equal(receipt.result,undefined);
      assert.equal((await mcp.call('syndocal_get_request_status',{requestId})).status,'unknown');
      const status=await backend.invoke('agent_authority_status_v1');
      if(risk!=='r1'){
        const hash=createHash('sha256').update('syndocal-audit-sha256-v1\0').update(requestId).digest('hex');
        const row=status.audit.find(row=>row.principalId===principalId&&row.operationId===operationId&&row.bridgeAttempt?.request_sha256===hash);
        assert.ok(row,'Rejected canonical mutation has immutable authorization metadata');assert.equal(row.outcome,'agent_missing_grant');
        assert.equal(row.bridgeAttempt.risk,risk.toUpperCase());assert.equal(row.bridgeAttempt.principal_incarnation,principalIncarnation);
      }
      assert.deepEqual(await state(),before);
      checks.push({check:`external-canonical-${operationId}-rejects-read-grant-before-dispatch`,passed:true,risk,
        missingGrant:'agent_missing_grant',noReceiptAdmitted:true,projectAndOutputUnchanged:true});
    }
  }finally{
    await mcp?.close();
    if(principalIncarnation!==undefined)await backend.invoke('agent_authority_revoke_v1',{principalId,principalIncarnation});
    if(credentialWritten)await fs.rm(credentialFile,{force:true});
  }
}

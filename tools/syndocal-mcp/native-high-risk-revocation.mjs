import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { openNativeStdioSession } from './native-stdio-session.mjs';

// Run at the last backend-only boundary before the harness closes this QA
// process. Registering a new generation retires the automatic renderer so the
// native claim/revoke/execute order is deterministic. No DOM or invoke patching.
export async function nativeHighRiskRevocation(backend, options, checks, { projectReplacement = false, projectFile = false, projectBackup = false } = {}) {
  const authorityObservations=[];
  const authority = async (kind, command, args = {}) => {
    for(let attempt=1;attempt<=8;attempt++) {
      const result = await backend.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})
      .then(value=>({value}),error=>{
        if(typeof error==='string')return {error:error.slice(0,768)};
        if(error && typeof error==='object' && Object.keys(error).length===4
          && typeof error.code==='string' && /^[a-z_]{1,64}$/.test(error.code)
          && typeof error.message==='string' && error.message.length<=1024
          && typeof error.retryable==='boolean' && typeof error.resnapshot_required==='boolean')
          return {error:{code:error.code,message:error.message,retryable:error.retryable,resnapshot_required:error.resnapshot_required}};
        return {error:'Unrecognized native authority error'};
      })`);
      if(result.value)return result.value;
      const error=result.error;
      // Only an explicit transient query error permits a fresh observation.
      // No admitted mutation, claim, execution or terminal replay is retried.
      assert.ok(error && typeof error==='object', `Revocation fixture authority ${kind}: ${JSON.stringify(error)}`);
      assert.deepEqual(Object.keys(error).sort(),['code','message','resnapshot_required','retryable']);
      assert.ok(['overloaded','unavailable'].includes(error.code),JSON.stringify(error));
      assert.equal(error.message,error.code==='overloaded'?'query service overloaded':'query service unavailable');
      assert.equal(error.retryable,true);assert.equal(error.resnapshot_required,false);
      authorityObservations.push({kind,command,attempt,error});
      assert.ok(attempt<8,`Revocation fixture read exhausted: ${JSON.stringify(error)}`);
      await new Promise(resolve=>setTimeout(resolve,25*attempt));
    }
  };
  const generation = await backend.invoke('agent_bridge_register_v1');
  assert.ok(Number.isSafeInteger(generation) && generation > 0);
  const checkpoint = async () => {
    const bundle = await backend.invoke('get_project_authority_bundle');
    return backend.invoke('get_project_checkpoint', { midiMappings: bundle.midi_mappings,
      oscMappings: bundle.osc_mappings, dmxMappings: bundle.dmx_mappings });
  };
  for (const kind of ['file', 'output', ...(projectReplacement ? ['project_new', 'project_open'] : []),
    ...(projectFile ? ['project_save', 'project_save_as', 'project_template'] : []),
    ...(projectBackup ? ['project_backup', 'project_restore', 'project_delete'] : [])]) {
    const publication = ['project_save', 'project_save_as', 'project_template', 'project_backup'].includes(kind);
    const project = kind.startsWith('project_') && !publication;
    const deletion = kind === 'project_delete';
    const observationStart=authorityObservations.length;
    const capability = project || publication ? 'file' : kind;
    const principalId = `native-revoke-${randomUUID()}`;
    const requestId = randomUUID();
    const credentialFile = path.join(path.dirname(options.credentialFile), `${principalId}.credential`);
    let destination = path.join(path.dirname(options.credentialFile), `${requestId}.${kind === 'project_template' ? 'sdctemplate' : publication || kind === 'project_open' ? 'sdc' : 'zip'}`);
    let source;
    let beforeProject;
    let principalIncarnation;
    let revoked = false;
    let mcp;
    try {
      const challenge = await backend.invoke('agent_authority_begin_pairing_v1', { principalId });
      const approval = await backend.invoke('agent_authority_approve_pairing_v1', {
        challengeId: challenge.challengeId, challenge: challenge.challenge,
      });
      principalIncarnation = approval.principalIncarnation;
      // The parent is the runner's existing ACL-restricted credential directory.
      await fs.writeFile(credentialFile, approval.credential, { flag: 'wx' });
      const operationId = publication ? `syndocal.project.${kind === 'project_backup' ? 'backup.create' : kind === 'project_template' ? 'template.save' : kind === 'project_save' ? 'save' : 'save_as'}.v1`
        : project ? `syndocal.project.${kind === 'project_new' ? 'new' : deletion ? 'backup.delete' : kind === 'project_restore' ? 'backup.restore' : 'open'}.v1` : capability === 'file'
        ? 'syndocal.diagnostics.export.v1' : 'syndocal.output.lease.acquire.v2';
      await backend.invoke('agent_authority_promote_v1', { principalId, principalIncarnation });
      await backend.invoke('agent_authority_grant_v1', { principalId, principalIncarnation,
        grant: { adapter: 'external_mcp', capability, operation_id: operationId, project_id: null },
      });
      const method = capability === 'file' && !project && !publication ? 'diagnostics.export' : 'control_plane.execute';
      if (project || publication) {
        beforeProject = await checkpoint();
        if (kind === 'project_open') {
          source = Buffer.from(JSON.stringify(beforeProject));
          await fs.writeFile(destination, source, { flag: 'wx' });
        }
      }
      let authorityResult;
      if (kind === 'project_backup') {
        // Issue the managed target for this exact external principal. The
        // automatic renderer is retired, so execute/complete the R0 read here.
        const readOperation='syndocal.query.project.backup.authority.v1';
        await backend.invoke('agent_authority_grant_v1', {principalId,principalIncarnation,
          grant:{adapter:'external_mcp',capability:'read',operation_id:readOperation,project_id:null}});
        mcp=await openNativeStdioSession({...options,principalId,principalIncarnation,credentialFile});
        const readId=randomUUID();const readPending=await mcp.call('syndocal_execute_control_plane',
          {requestId:readId,operationId:readOperation,request:{request:{schema_version:1}}});
        assert.equal(readPending.status,'pending');
        await backend.invoke('agent_bridge_claim_v1',{rendererGeneration:generation,requestId:readId});
        const readResult=await backend.invoke('agent_bridge_execute_native_v1',{rendererGeneration:generation,requestId:readId});
        assert.equal(readResult.ok,true);
        await backend.invoke('agent_bridge_complete_v1',{rendererGeneration:generation,requestId:readId,result:readResult});
        const completed=await mcp.call('syndocal_get_request_status',{requestId:readId});
        assert.equal(completed.status,'completed');assert.equal(completed.result.ok,true);
        authorityResult={value:completed.result.result};
      } else if (publication) {
        authorityResult={value:await authority(kind,'query_project_file_authority_v1', {
          request: {schema_version:1,operation_id:operationId,destination},
        })};
      }
      if (publication) assert.ok(authorityResult.value, `Revocation fixture authority ${kind}: ${authorityResult.error}`);
      const fileAuthority = authorityResult?.value;
      if (kind === 'project_backup') destination = fileAuthority.destination;
      const params = publication ? { operationId, request: { request: { schema_version: 1,
        operation_id: operationId, request_id: 1, expected_fence: fileAuthority.fence,
        expected_path_generation: fileAuthority.path_generation,
        expected_disposition_generation: fileAuthority.disposition_generation,
        destination: fileAuthority.destination, expected_target_sha256: fileAuthority.target_sha256,
      } } } : deletion ? { operationId, request: { request: {schema_version:1,
        operation_id:operationId,request_id:Date.now(),
        expected_fence:(await authority(kind,'query_project_replacement_authority_v1',{request:{}})).fence,
        backup_id:Number.MAX_SAFE_INTEGER,expected_artifact_sha256:'0'.repeat(64),
      } } } : project ? { operationId, request: { request: { schema_version: 1,
        operation_id: operationId, request_id: Date.now(),
        expected_fence: (await authority(kind,'query_project_replacement_authority_v1', { request: {} })).fence,
        action: kind === 'project_new' ? { kind: 'new' } : kind === 'project_restore' ? {
          kind: 'restore_backup', backup_id: Number.MAX_SAFE_INTEGER,
          expected_file_sha256: '0'.repeat(64), expected_source_path: null,
        } : { kind: 'open', path: destination,
          expected_file_sha256: createHash('sha256').update(source).digest('hex') },
      } } } : capability === 'file' ? { destination } : {
        operationId,
        request: { request: {
          operation_id: operationId, request_id: Date.now(),
          expected_fence: (await authority(kind,'query_output_control_authority_v1')).fence,
          action: { kind: 'acquire_lease', role: 'lighting' },
        } },
      };
      mcp ??= await openNativeStdioSession({ ...options, principalId, principalIncarnation, credentialFile });
      const admitted = await mcp.call(capability === 'file' && !project && !publication
        ? 'syndocal_export_diagnostics' : 'syndocal_execute_control_plane', { requestId, ...params });
      assert.equal(admitted.status, 'pending');
      assert.equal(admitted.requestId, requestId);
      assert.equal(admitted.result, undefined);
      const claimed = await backend.invoke('agent_bridge_claim_v1', { rendererGeneration: generation, requestId });
      assert.equal(claimed.principalId, principalId);
      assert.equal(claimed.principalIncarnation, principalIncarnation);
      assert.equal(claimed.method, method);
      assert.deepEqual(claimed.params, params);
      await backend.invoke('agent_authority_revoke_v1', { principalId, principalIncarnation });
      revoked = true;
      const execute = () => backend.evaluate(`window.__TAURI_INTERNALS__.invoke('agent_bridge_execute_native_v1', ${JSON.stringify({ rendererGeneration: generation, requestId })}).then(value => ({value}), error => ({error:String(error)}))`);
      assert.deepEqual(await execute(), { error: 'agent_principal_revoked' });
      assert.deepEqual(await execute(), { error: 'request_not_executable' });
      const hidden = await mcp.call('syndocal_get_request_status', { requestId });
      assert.equal(hidden.status, 'rejected');
      assert.equal(hidden.error, 'agent_principal_revoked');
      assert.equal(hidden.result, undefined);
      if (source) assert.deepEqual(await fs.readFile(destination), source);
      else assert.equal(await fs.stat(destination).then(() => true, error => {
        if (error.code === 'ENOENT') return false;
        throw error;
      }), false);
      if (project || publication) assert.deepEqual(await checkpoint(), beforeProject);
      assert.deepEqual((await authority(kind,'query_output_lease_authority_v1')).statuses, [{ status: 'unavailable' }]);
      const ownership = await backend.invoke('get_output_ownership_status');
      assert.equal(ownership.lighting_allowed, false);
      assert.equal(ownership.video_allowed, false);
      checks.push({ check: `external-${project || publication ? kind : capability === 'file' ? 'r5' : 'r4'}-revoke-after-claim-before-native-execution`,
        passed: true, operationId, requestId, rendererGeneration: generation,
        executionError: 'agent_principal_revoked', replayError: 'request_not_executable',
        destinationAbsent: !source, sourceBytesPreserved: Boolean(source), leaseUnavailable: true, credentialRevoked: true,
        authorityObservations:authorityObservations.slice(observationStart),
      });
    } finally {
      try { await mcp?.close(); }
      finally {
        if (principalIncarnation !== undefined && !revoked) {
          await backend.invoke('agent_authority_revoke_v1', { principalId, principalIncarnation });
        }
        assert.equal(path.dirname(credentialFile), path.dirname(options.credentialFile));
        await fs.rm(credentialFile, { force: true });
        if (source) await fs.rm(destination, { force: true });
      }
    }
  }
}

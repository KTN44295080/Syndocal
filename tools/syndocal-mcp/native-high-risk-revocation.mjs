import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { openNativeStdioSession } from './native-stdio-session.mjs';

// Run at the last backend-only boundary before the harness closes this QA
// process. Registering a new generation retires the automatic renderer so the
// native claim/revoke/execute order is deterministic. No DOM or invoke patching.
export async function nativeHighRiskRevocation(backend, options, checks, { projectReplacement = false } = {}) {
  const generation = await backend.invoke('agent_bridge_register_v1');
  assert.ok(Number.isSafeInteger(generation) && generation > 0);
  for (const kind of ['file', 'output', ...(projectReplacement ? ['project_new', 'project_open'] : [])]) {
    const project = kind.startsWith('project_');
    const capability = project ? 'file' : kind;
    const principalId = `native-revoke-${randomUUID()}`;
    const requestId = randomUUID();
    const credentialFile = path.join(path.dirname(options.credentialFile), `${principalId}.credential`);
    const destination = path.join(path.dirname(options.credentialFile), `${requestId}.${kind === 'project_open' ? 'sdc' : 'zip'}`);
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
      const operationId = project ? `syndocal.project.${kind === 'project_new' ? 'new' : 'open'}.v1` : capability === 'file'
        ? 'syndocal.diagnostics.export.v1' : 'syndocal.output.lease.acquire.v2';
      await backend.invoke('agent_authority_promote_v1', { principalId, principalIncarnation });
      await backend.invoke('agent_authority_grant_v1', { principalId, principalIncarnation,
        grant: { adapter: 'external_mcp', capability, operation_id: operationId, project_id: null },
      });
      const method = capability === 'file' && !project ? 'diagnostics.export' : 'control_plane.execute';
      if (project) {
        beforeProject = await backend.invoke('get_project_checkpoint', { midiMappings: [], oscMappings: [], dmxMappings: [] });
        if (kind === 'project_open') {
          source = Buffer.from(JSON.stringify(beforeProject));
          await fs.writeFile(destination, source, { flag: 'wx' });
        }
      }
      const params = project ? { operationId, request: { request: { schema_version: 1,
        operation_id: operationId, request_id: Date.now(),
        expected_fence: (await backend.invoke('query_project_replacement_authority_v1', { request: {} })).fence,
        action: kind === 'project_new' ? { kind: 'new' } : { kind: 'open', path: destination,
          expected_file_sha256: createHash('sha256').update(source).digest('hex') },
      } } } : capability === 'file' ? { destination } : {
        operationId,
        request: { request: {
          operation_id: operationId, request_id: Date.now(),
          expected_fence: (await backend.invoke('query_output_control_authority_v1')).fence,
          action: { kind: 'acquire_lease', role: 'lighting' },
        } },
      };
      mcp = await openNativeStdioSession({ ...options, principalId, principalIncarnation, credentialFile });
      const admitted = await mcp.call(capability === 'file' && !project
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
      if (project) assert.deepEqual(await backend.invoke('get_project_checkpoint', {
        midiMappings: [], oscMappings: [], dmxMappings: [],
      }), beforeProject);
      assert.deepEqual((await backend.invoke('query_output_lease_authority_v1')).statuses, [{ status: 'unavailable' }]);
      const ownership = await backend.invoke('get_output_ownership_status');
      assert.equal(ownership.lighting_allowed, false);
      assert.equal(ownership.video_allowed, false);
      checks.push({ check: `external-${project ? kind : capability === 'file' ? 'r5' : 'r4'}-revoke-after-claim-before-native-execution`,
        passed: true, operationId, requestId, rendererGeneration: generation,
        executionError: 'agent_principal_revoked', replayError: 'request_not_executable',
        destinationAbsent: !source, sourceBytesPreserved: Boolean(source), leaseUnavailable: true, credentialRevoked: true,
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

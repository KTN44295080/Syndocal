import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { openNativeStdioSession } from './native-stdio-session.mjs';

// A valid unbound-label Touch fixture makes real checkpoint captures substantial.
// One exact-grant R5 Open loads it; no lock-hold route, file publication, output activation or retry.
export async function nativeFileQueryPressure(backend, options, checks) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-file-query-'));
  const destination = path.join(directory, 'never-published.sdc');
  const queries = [
    { operationId: 'syndocal.query.project.file.authority.v1', command: 'query_project_file_authority_v1',
      request: { schema_version: 1, operation_id: 'syndocal.project.save_as.v1', destination } },
    { operationId: 'syndocal.query.project.backup.authority.v1', command: 'query_project_backup_authority_v1',
      request: { schema_version: 1 } },
  ];
  const state = async () => {
    const bundle = await backend.invoke('get_project_authority_bundle');
    const project = await backend.invoke('get_project_checkpoint', {
      midiMappings: bundle.midi_mappings, oscMappings: bundle.osc_mappings, dmxMappings: bundle.dmx_mappings });
    delete bundle.snapshot;
    return { project, bundle, output: await backend.invoke('get_output_ownership_status') };
  };
  const validateError = error => {
    assert.deepEqual(Object.keys(error).sort(), ['code', 'message', 'resnapshot_required', 'retryable']);
    const messages = { overloaded: 'query service overloaded', unavailable: 'query service unavailable' };
    assert.ok(Object.hasOwn(messages, error.code), JSON.stringify(error));
    assert.equal(error.message, messages[error.code]);
    assert.equal(error.retryable, true);
    assert.equal(error.resnapshot_required, false);
  };
  const validateAuthority = (value, before) => {
    assert.equal(value.schema_version, 1);
    assert.equal(value.fence.project_epoch, before.bundle.project_epoch);
    assert.equal(value.fence.project_revision, before.bundle.project_revision);
    assert.equal(value.fence.project_checkpoint_hash, before.bundle.checkpoint_hash);
    assert.equal(value.fence.project_publication_generation, before.bundle.publication_generation);
    assert.equal(value.target_sha256, null);
  };
  let mcp;
  const clients = [];
  const send = async (query, client = mcp) => {
    const requestId = randomUUID();
    let receipt = await client.call('syndocal_execute_control_plane', {
      requestId, operationId: query.operationId, request: { request: query.request } });
    const deadline = Date.now() + 15000;
    while (receipt.status === 'pending' && Date.now() < deadline) {
      await new Promise(resolve => setTimeout(resolve, 50));
      receipt = await client.call('syndocal_get_request_status', { requestId });
    }
    assert.equal(receipt.status, 'completed', JSON.stringify(receipt));
    return receipt.result;
  };
  const grant = (capability, operationId) => backend.invoke('agent_authority_grant_v1', {
    principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    grant: { adapter: 'external_mcp', capability, operation_id: operationId, project_id: null } });
  let fixturePath, fixtureBytes;
  try {
    const empty = await state();
    assert.equal(empty.project.snapshot.fixtures.length, 0);
    assert.equal(empty.project.snapshot.video.outputs.length, 0);
    assert.equal(empty.project.snapshot.timeline.playing, false);
    const surface = { pages: Array.from({ length: 64 }, (_, page) => ({
      id: page + 1, label: `Read pressure ${page + 1}`,
      controls: Array.from({ length: 96 }, (_, cell) => ({
        id: page * 96 + cell + 1, kind: 'Label', x: cell % 12, y: Math.floor(cell / 12), w: 1, h: 1,
        label: `Unbound read fixture ${page}:${cell}`.padEnd(128, '.'),
      })),
    })) };
    mcp = await openNativeStdioSession(options);
    clients.push(mcp);
    const replacement = 'syndocal.query.project.replacement.authority.v1', open = 'syndocal.project.open.v1';
    await grant('read', replacement);
    await backend.invoke('agent_authority_promote_v1', {
      principalId: options.principalId, principalIncarnation: options.principalIncarnation });
    await grant('file', open);
    const issued = await send({ operationId: replacement, request: {} });
    assert.equal(issued.ok, true, JSON.stringify(issued));
    const fixture = structuredClone(empty.project);
    fixture.snapshot.touch_surface = surface;
    fixturePath = path.join(directory, 'unbound-touch.sdc');
    fixtureBytes = Buffer.from(JSON.stringify(fixture));
    await fs.writeFile(fixturePath, fixtureBytes, { flag: 'wx' });
    const opened = await send({ operationId: open, request: { schema_version: 1, operation_id: open,
      request_id: Date.now(), expected_fence: issued.result.fence, action: { kind: 'open', path: fixturePath,
        expected_file_sha256: createHash('sha256').update(fixtureBytes).digest('hex') } } });
    assert.equal(opened.ok, true, JSON.stringify(opened));
    assert.equal(opened.result.status, 'receipt');
    const before = await state();
    assert.deepEqual(before.project, fixture, 'R5 Open retains the complete independently constructed safe Touch image');
    assert.equal(before.output.lighting_allowed, false);
    assert.equal(before.output.video_allowed, false);
    checks.push({ check: 'file-query-pressure-native-valid-unbound-touch-fixture', passed: true,
      pages: 64, controls: 6144, bytes: Buffer.byteLength(JSON.stringify(before.project)),
      bindingCount: 0, physicalFixtureCount: 0, videoOutputCount: 0 });
    for (const query of queries) await grant('read', query.operationId);
    for (let index = 1; index < 4; index++) clients.push(await openNativeStdioSession(options));
    const local = [];
    const external = [];
    let checkpointReads = 0, checkpointSuccess = 0, checkpointBusy = 0;
    const started = performance.now();
    for (let round = 0; round < 8; round++) {
      // Issue all invocations once and wait for their individual results. The
      // aggregate contains no renderer snapshots/project content or credentials.
      const pressure = backend.evaluate(`(async () => {
        const queries=${JSON.stringify(queries)};
        const invoke=window.__TAURI_INTERNALS__.invoke;
        const expected=${JSON.stringify({expectedEpoch:before.bundle.project_epoch,
          expectedRevision:before.bundle.project_revision,expectedCheckpointHash:before.bundle.checkpoint_hash})};
        const start=performance.now();
        const probes=Array.from({length:64},(_,i)=>{
          const q=queries[i%queries.length];
          return invoke(q.command,{request:q.request}).then(value=>({operationId:q.operationId,value}),
            error=>({operationId:q.operationId,error}));
        });
        const results=Promise.all(probes);
        let readCount=0,successCount=0,busyCount=0;
        for(let batch=0;batch<128 && performance.now()-start<2500;batch++){
          const captures=await Promise.all(Array.from({length:64},()=>invoke('get_project_checkpoint_bundle',expected)
            .then(value=>({epoch:value.project_epoch,revision:value.project_revision,hash:value.checkpoint_hash}),error=>({error}))));
          for(const capture of captures){
            if(capture.error === 'Project recovery capture is busy; retry')busyCount++;
            else if(!capture.error && capture.epoch===expected.expectedEpoch && capture.revision===expected.expectedRevision
              && capture.hash===expected.expectedCheckpointHash)successCount++;
            else throw new Error('Unexpected companion checkpoint capture');
          }
          readCount+=64;
        }
        return {readCount,successCount,busyCount,results:await results};
      })()`);
      // Each real stdio client permits one in-flight tools/call. Separate
      // authenticated clients create actual concurrent native execution.
      const remote = Promise.all(clients.map((client, index) => send(queries[index % 2], client)));
      const [observed, receipts] = await Promise.all([pressure, remote]);
      assert.ok(observed.readCount >= 64 && observed.readCount <= 8192);
      checkpointReads += observed.readCount;
      checkpointSuccess += observed.successCount;
      checkpointBusy += observed.busyCount;
      local.push(...observed.results);
      external.push(...receipts.map((result, index) => ({ operationId: queries[index % 2].operationId, ...result })));
    }
    const counts = { localSuccess: 0, externalSuccess: 0, localErrors: {}, externalErrors: {} };
    for (const result of local) {
      if (result.error) {
        validateError(result.error);
        counts.localErrors[result.error.code] = (counts.localErrors[result.error.code] ?? 0) + 1;
      } else { validateAuthority(result.value, before); counts.localSuccess++; }
    }
    for (const result of external) {
      if (!result.ok) {
        assert.equal(result.error.code, 'request_rejected');
        validateError(result.error.native_query);
        assert.equal(result.error.message, result.error.native_query.message);
        counts.externalErrors[result.error.native_query.code] = (counts.externalErrors[result.error.native_query.code] ?? 0) + 1;
      } else { validateAuthority(result.result, before); counts.externalSuccess++; }
    }
    const elapsedMs = performance.now() - started;
    checks.push({ check: 'file-authority-real-concurrent-read-query-wire', passed: true,
      checkpointReads, checkpointSuccess, checkpointBusy,
      localCalls: local.length, externalCalls: external.length, stdioClients: clients.length, counts, elapsedMs,
      localErrorSamples: local.filter(result => result.error).slice(0, 4),
      externalErrorSamples: external.filter(result => !result.ok).slice(0, 4) });
    // Require evidence from the actual external route, not just a local unit.
    assert.ok(checkpointSuccess > 0);
    assert.equal(checkpointSuccess + checkpointBusy, checkpointReads);
    assert.ok((counts.externalErrors.overloaded ?? 0) > 0,
      'This bounded pressure run must observe an external overloaded QueryError to establish that native error wire');
    for (const query of queries) assert.ok(external.some(result => result.operationId === query.operationId
      && result.error?.native_query?.code === 'overloaded'), 'Each authority read must preserve an actual external overload');
    for (const query of queries) {
      const result = await send(query);
      assert.equal(result.ok, true, JSON.stringify(result));
      validateAuthority(result.result, before);
    }
    assert.deepEqual(await state(), before, 'Read pressure preserves complete project, authority and closed outputs');
    assert.deepEqual(await fs.readdir(directory), [path.basename(fixturePath)], 'No query creates its publication target');
    assert.deepEqual(await fs.readFile(fixturePath), fixtureBytes, 'Read pressure never rewrites the fixture source');
    checks.push({ check: 'file-authority-new-observation-after-pressure-and-complete-state-preservation', passed: true });
  } finally {
    const closed = await Promise.allSettled(clients.map(client => client.close()));
    assert.ok(closed.every(result => result.status === 'fulfilled'), 'Owned MCP clients must all close');
    assert.equal(path.dirname(directory), await fs.realpath(os.tmpdir()));
    assert.ok(path.basename(directory).startsWith('syndocal-file-query-'));
    await fs.rm(directory, { recursive: true, force: true });
  }
}

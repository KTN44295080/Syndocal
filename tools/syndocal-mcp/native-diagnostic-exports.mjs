import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';

// Independently inspect production STORED payloads and the integrity manifest.
// This is an observation helper, never a production archive reader.
export function inspectDiagnosticAudit(bytes) {
  assert.ok(bytes.length<=160*1024);
  const names=['manifest.json','project-summary.json','engine-telemetry.json','video-runtime.json','audit-history.json','integrity-manifest.json'];
  const entries=[];let offset=0;
  for(const name of names){
    assert.equal(bytes.readUInt32LE(offset),0x04034b50);
    assert.equal(bytes.readUInt16LE(offset+6),0);assert.equal(bytes.readUInt16LE(offset+8),0);
    const size=bytes.readUInt32LE(offset+18),rawSize=bytes.readUInt32LE(offset+22);
    assert.equal(size,rawSize);assert.ok(size<=64*1024);
    const nameLength=bytes.readUInt16LE(offset+26),extraLength=bytes.readUInt16LE(offset+28);
    const start=offset+30+nameLength+extraLength;
    assert.equal(bytes.subarray(offset+30,offset+30+nameLength).toString('utf8'),name);
    assert.ok(start+size<=bytes.length);entries.push({name,bytes:bytes.subarray(start,start+size)});offset=start+size;
  }
  assert.equal(bytes.readUInt32LE(offset),0x02014b50);
  const decoded=entries.map(e=>JSON.parse(e.bytes));
  assert.equal(decoded[0].version,2);assert.equal(decoded[5].schema_version,2);assert.equal(decoded[5].redaction_schema_version,2);
  assert.equal(decoded[5].entries.length,5);
  entries.slice(0,5).forEach((e,index)=>assert.deepEqual(decoded[5].entries[index],{
    name:e.name,size_bytes:e.bytes.length,sha256:createHash('sha256').update(e.bytes).digest('hex'),
  }));
  const audit=decoded[4];assert.equal(audit.schema_version,1);
  assert.ok(Number.isSafeInteger(audit.process_incarnation)&&audit.process_incarnation>0);
  assert.equal(audit.independent_source_observations,true);assert.equal(audit.full_attempt_fields_available,false);
  assert.deepEqual(audit.pages.map(p=>p.source),['agent_authority','output_lease','project_file','project_replacement','safety','output_control']);
  for(const page of audit.pages){
    assert.ok(page.records.length<=16&&page.records.length<=page.retained_count);
    let previous=0;
    for(const row of page.records){
      assert.ok(Number.isSafeInteger(row.sequence)&&row.sequence>previous);previous=row.sequence;
      assert.ok(row.sequence>=page.retained_first_sequence&&row.sequence<=page.retained_last_sequence);
      if(page.selected_before_sequence!==null)assert.ok(row.sequence<page.selected_before_sequence);
      for(const [key,value] of Object.entries(row))if(key.endsWith('_sha256')&&value!==null)assert.match(value,/^[0-9a-f]{64}$/);
      assert.equal(Object.keys(row).some(key=>['caller','principal','result','path','error','operation_id'].includes(key)),false);
    }
    if(page.next_before_sequence!==null)assert.equal(page.next_before_sequence,page.records[0].sequence);
  }
  return audit;
}

// Invoked only after the lifecycle runner verifies its blank isolated QA app.
// No GUI actions, project writes or external MCP permission changes.
export async function nativeDiagnosticExports(invoke) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-diagnostic-qa-'));
  const destination = path.join(directory, 'support.zip');
  const absent = async () => assert.equal(await fs.stat(destination).then(() => true, error => {
    if (error.code === 'ENOENT') return false;
    throw error;
  }), false, 'No destination may be created before approval');
  const prepare = () => invoke('prepare_diagnostic_export_v1', { destination });
  const finish = (preview, approved, sha256 = preview.sha256) => invoke('finish_diagnostic_export_v1', {
    captureId: preview.captureId, sha256, approved,
  });
  try {
    await assert.rejects(invoke('prepare_diagnostic_export_v1', { destination: 'relative.zip' }));
    const cancelled = await prepare();
    assert.equal(cancelled.destination, destination);
    assert.equal(cancelled.validForMs, 120000);
    assert.match(cancelled.summary, /Excludes project\/media files, paths, credentials/);
    await absent();
    assert.equal(await finish(cancelled, false), null);
    await assert.rejects(finish(cancelled, true));
    await absent();
    const preview = await prepare();
    await assert.rejects(finish(preview, true, '0'.repeat(64)));
    await absent();
    assert.equal(await finish(preview, true), destination);
    const bytes = await fs.readFile(destination);
    assert.equal(createHash('sha256').update(bytes).digest('hex'), preview.sha256);
    assert.equal(bytes.readUInt32LE(0), 0x04034b50);
    assert.ok(bytes.length <= 160 * 1024);
    const audit=inspectDiagnosticAudit(bytes);
    assert.ok(!bytes.includes(Buffer.from(directory)), 'Package must not expose export path');
    await assert.rejects(finish(preview, true));
    assert.equal(createHash('sha256').update(await fs.readFile(destination)).digest('hex'), preview.sha256);
    const race = path.join(directory, 'race.zip');
    const racingPreview = await invoke('prepare_diagnostic_export_v1', { destination: race });
    await fs.writeFile(race, 'new occupant', { flag: 'wx' });
    await assert.rejects(finish(racingPreview, true));
    assert.equal(await fs.readFile(race, 'utf8'), 'new occupant', 'Later file must not be replaced');
    await assert.rejects(invoke('prepare_diagnostic_export_v1', { destination: race }));
    // Leave one in-memory capture to prove it cannot survive the runner's restart.
    const retired = await invoke('prepare_diagnostic_export_v1', {
      destination: path.join(directory, 'retired.zip'),
    });
    return { retired, check: { check: 'native-diagnostic-preview-cancel-digest-new-file-race-replay', passed: true,
      archiveBytes: bytes.length, archiveSha256: preview.sha256, formatVersion:2,
      auditProcessIncarnation:audit.process_incarnation, auditSources:audit.pages.map(p=>({source:p.source,retainedCount:p.retained_count,exportedCount:p.records.length})) } };
  } finally {
    assert.equal(path.dirname(path.resolve(directory)), path.resolve(os.tmpdir()));
    assert.ok(path.basename(directory).startsWith('syndocal-diagnostic-qa-'));
    for (const entry of await fs.readdir(directory)) {
      assert.ok(['support.zip', 'race.zip', 'retired.zip'].includes(entry)
        || /^\.syndocal-diagnostics-\d+-[0-9a-f]{32}\.partial\.zip$/.test(entry),
      'Only this QA run may create files in its private directory');
      await fs.unlink(path.join(directory, entry));
    }
    await fs.rmdir(directory);
  }
}

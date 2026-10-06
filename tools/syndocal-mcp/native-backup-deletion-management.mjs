import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {createHash, randomUUID} from 'node:crypto';
import {isDeepStrictEqual} from 'node:util';

// Uses the already authenticated private native MCP session. Fixtures append
// only run-owned facts; existing origins/receipts are never erased or adopted.
export async function nativeBackupDeletionManagement(backend, checks, context) {
  const {send,success,failed,grant,state,directory,saved,bytes,deleted,deleteRequest,deletion,ids:backup}=context;
  const ids={query:'syndocal.query.project.backup.delete.journal.v1',
    status:'syndocal.query.project.backup.delete.journal.status.v1',
    manage:'syndocal.project.backup.delete.journal.manage.v1'};
  const journal=path.join(path.dirname(directory),'project-backup-deletions-v1.json');
  const digest=value=>createHash('sha256').update(value).digest('hex');
  const started=performance.now();const authorityBefore=await state();
  const baseline=JSON.parse(await fs.readFile(journal));
  const originalFact=baseline.records.find(row=>isDeepStrictEqual(row.receipt.request,deleteRequest));
  assert.ok(originalFact);assert.equal(originalFact.phase,'succeeded');
  const protectedFacts=baseline.records.filter(row=>row!==originalFact);
  const registry=await backend.invoke('get_control_plane_canonical_registry');
  for(const [id,risk] of [[ids.query,'r0'],[ids.status,'r0'],[ids.manage,'r5']]) {
    const operation=registry.canonical_operations.find(row=>row.operation_id===id);assert.ok(operation);
    assert.equal(operation.risk,risk);
    assert.equal(operation.adapter_policy,risk==='r0'?'local_window_read_only':'local_window_project_publication');
    const denied=await send(id,{schema_version:1});assert.equal(denied.status,'rejected');assert.equal(denied.error,'agent_missing_grant');
    await grant(risk==='r0'?'read':'file',id);
  }
  checks.push({check:'external-deletion-management-exact-r0-read-r5-file-grants-without-human-confirmation',passed:true});
  const queryRequest={schema_version:1,limit:16,after_record_id:null,expected_journal_sha256:null};
  const listing=async()=>{
    const first=await success(ids.query,queryRequest),rows=[...first.records];let cursor=first.next_after_record_id;
    for(let page=0;cursor!==null&&page<40;page++) {
      const next=await success(ids.query,{...queryRequest,after_record_id:cursor,expected_journal_sha256:first.journal_sha256});
      assert.equal(next.generation,first.generation);assert.equal(next.journal_sha256,first.journal_sha256);
      assert.ok(next.records.length<=16);rows.push(...next.records);cursor=next.next_after_record_id;
    }
    assert.equal(cursor,null);assert.equal(rows.length,first.deletion_count+first.management_count+first.retired_count);
    assert.ok(rows.every((row,index)=>index===0||rows[index-1].record_id<row.record_id));
    for(const row of rows)for(const field of ['origin','owner','principal','path','receipt'])assert.equal(row[field],undefined);
    return {...first,records:rows};
  };
  let sequence=1;
  const request=async action=>{const observed=await listing();return {schema_version:1,operation_id:ids.manage,request_id:sequence++,
    expected_fence:structuredClone(deleteRequest.expected_fence),expected_generation:observed.generation,
    expected_journal_sha256:observed.journal_sha256,action};};
  const readBefore=await fs.readFile(journal);const initial=await listing();
  assert.equal(initial.storage_schema_version,2);assert.equal(initial.journal_sha256,digest(readBefore));
  for(const field of ['origin','owner_id','principal','path','skip_confirmation'])await failed(ids.query,{...queryRequest,[field]:true},/arguments_invalid/);
  for(const value of [{...queryRequest,limit:0},{...queryRequest,limit:17},{...queryRequest,schema_version:2},
    {...queryRequest,after_record_id:'bad'},{...queryRequest,expected_journal_sha256:'f'.repeat(64)}]) {
    await failed(ids.query,value,/invalid deletion journal query|snapshot_changed/);
  }
  assert.deepEqual(await fs.readFile(journal),readBefore);assert.deepEqual(await state(),authorityBefore);
  checks.push({check:'external-deletion-journal-bounded-redacted-pagination-strict-ingress-and-read-purity',passed:true,
    initialCounts:{deletion:initial.deletion_count,management:initial.management_count,retired:initial.retired_count}});
  // Explicitly owned prepared-fact fixture, not an assertion of actual process
  // interruption. Preserve the observed original completed fact unchanged.
  const fixtureOrigin=digest(`owned-management-fixture:${randomUUID()}`);
  const facts=JSON.parse(readBefore);assert.ok(facts.records.length<256);assert.ok(facts.generation<Number.MAX_SAFE_INTEGER);
  facts.records.push({...structuredClone(originalFact),origin:fixtureOrigin,phase:'prepared',error:null});facts.generation++;
  await fs.writeFile(saved.target_path,bytes);await fs.writeFile(journal,JSON.stringify(facts));
  const prepared=(await listing()).records.filter(row=>row.kind==='deletion'&&row.phase==='prepared'&&row.backup_id===saved.backup.id);
  assert.equal(prepared.length,1);const record=prepared[0];const protectedBefore=await fs.readFile(journal);
  const ackUnresolved=await request({kind:'acknowledge',record_ids:[record.record_id]});
  await failed(ids.manage,ackUnresolved,/unresolved_cannot_acknowledge/);
  const releaseAction={kind:'release_unknown_protection',record_id:record.record_id,
    expected_artifact:{kind:'present',sha256:digest(bytes)}};
  const release=await request(releaseAction);assert.equal(await success(ids.status,release),null);
  for(const field of ['path','owner_id','principal','skip_confirmation'])await failed(ids.manage,{...release,request_id:sequence++,[field]:true},/arguments_invalid/);
  for(const value of [{...release,request_id:sequence++,expected_generation:release.expected_generation+1},
    {...release,request_id:sequence++,expected_journal_sha256:'f'.repeat(64)},
    {...release,request_id:sequence++,action:{...releaseAction,expected_artifact:{kind:'missing'}}},
    {...release,request_id:sequence++,action:{kind:'acknowledge',record_ids:[record.record_id,record.record_id]}},
    {...release,request_id:sequence++,schema_version:2},{...release,request_id:0}]) {
    await failed(ids.manage,value,/snapshot_changed|artifact_changed|invalid deletion management/);
  }
  await failed(ids.manage,{...release,request_id:sequence++,action:{...releaseAction,expected_artifact:{kind:'missing',path:'forged'}}},/arguments_invalid/);
  await failed(ids.manage,{...release,request_id:Number.MAX_SAFE_INTEGER+1},/invalid deletion management request/);
  const forged={...release,request_id:sequence++,expected_fence:{...release.expected_fence,session_incarnation:release.expected_fence.session_incarnation+1}};
  await failed(ids.manage,forged,/unissued_fence/);
  const writer=await fs.open(saved.target_path,'r+');
  try {await failed(ids.manage,{...release,request_id:sequence++},/artifact_open/);}finally{await writer.close();}
  const local=await backend.evaluate(`window.__TAURI_INTERNALS__.invoke('delete_project_backup',${JSON.stringify({backupId:saved.backup.id})}).then(value=>({value}),error=>({error:String(error)}))`);
  assert.match(local.error,/unresolved_journal_protects/);
  assert.deepEqual(await fs.readFile(journal),protectedBefore);assert.deepEqual(await fs.readFile(saved.target_path),bytes);
  checks.push({check:'external-deletion-management-unresolved-ack-stale-snapshot-forged-fence-writer-and-changed-artifact-rejection-preserves-all-bytes',passed:true});
  const released=await success(ids.manage,release);assert.deepEqual(released.request,release);
  assert.deepEqual(released.observed_artifact,releaseAction.expected_artifact);assert.equal(released.generation_after,released.generation_before+1);
  assert.deepEqual(await success(ids.status,release),released);assert.deepEqual(await success(ids.manage,release),released);
  await failed(ids.manage,{...release,expected_generation:release.expected_generation+1},/different shape/);
  assert.deepEqual(await fs.readFile(saved.target_path),bytes);
  const resolved=(await listing()).records.find(row=>row.kind==='deletion'&&row.backup_id===saved.backup.id&&row.phase==='resolved_unknown');
  assert.ok(resolved);const rawResolved=JSON.parse(await fs.readFile(journal)).records.find(row=>row.origin===fixtureOrigin);
  assert.equal(rawResolved.phase,'resolved_unknown');assert.match(rawResolved.error,/outcome remains unknown/);
  assert.deepEqual(await success(backup.deleteStatus,deleteRequest).then(value=>value.receipt),deleted);
  const newDelete=await deletion(saved.backup.id,digest(bytes));await success(backup.remove,newDelete);
  await assert.rejects(fs.stat(saved.target_path),/ENOENT/);assert.deepEqual(await state(),authorityBefore);
  const missingFacts=JSON.parse(await fs.readFile(journal));
  const newFact=missingFacts.records.find(row=>isDeepStrictEqual(row.receipt.request,newDelete));assert.ok(newFact);
  const missingOrigin=digest(`owned-missing-management-fixture:${randomUUID()}`);
  assert.ok(missingFacts.records.length<256);missingFacts.records.push({...structuredClone(newFact),origin:missingOrigin,phase:'prepared',error:null});
  missingFacts.generation++;await fs.writeFile(journal,JSON.stringify(missingFacts));
  const missingRows=(await listing()).records.filter(row=>row.kind==='deletion'&&row.phase==='prepared'&&row.backup_id===saved.backup.id);
  assert.equal(missingRows.length,1);
  const missingRequest=await request({kind:'release_unknown_protection',record_id:missingRows[0].record_id,expected_artifact:{kind:'missing'}});
  const missingResult=await success(ids.manage,missingRequest);assert.deepEqual(missingResult.observed_artifact,{kind:'missing'});
  await assert.rejects(fs.stat(saved.target_path),/ENOENT/);
  const missingResolved=JSON.parse(await fs.readFile(journal)).records.find(row=>row.origin===missingOrigin);
  assert.equal(missingResolved.phase,'resolved_unknown');assert.match(missingResolved.error,/outcome remains unknown/);
  checks.push({check:'external-deletion-management-observed-release-retains-unknown-outcome-and-permits-only-explicit-new-delete-intent',passed:true,
    receipt:released,missingArtifactReceipt:missingResult});
  const rows=(await listing()).records;
  const ownedIds=rows.filter(row=>row.kind==='deletion'&&row.backup_id===saved.backup.id).map(row=>row.record_id);
  const releaseRow=rows.find(row=>row.kind==='management'&&row.request_id===release.request_id
    &&row.process_incarnation===release.expected_fence.process_incarnation);assert.ok(releaseRow);ownedIds.push(releaseRow.record_id);
  const missingResultRow=rows.find(row=>row.kind==='management'&&row.request_id===missingRequest.request_id
    &&row.process_incarnation===missingRequest.expected_fence.process_incarnation);assert.ok(missingResultRow);ownedIds.push(missingResultRow.record_id);
  assert.equal(ownedIds.length,6);
  const ackRequest=await request({kind:'acknowledge',record_ids:ownedIds});const acknowledged=await success(ids.manage,ackRequest);
  assert.deepEqual(await success(ids.status,ackRequest),acknowledged);
  const recreated=Buffer.from('late retired deletion must not touch this owned invalid artifact');await fs.writeFile(saved.target_path,recreated);
  await failed(backup.remove,deleteRequest,/receipt_expired/);await failed(backup.deleteStatus,deleteRequest,/receipt_expired/);
  await failed(backup.remove,newDelete,/receipt_expired/);await failed(ids.manage,release,/receipt_expired/);await failed(ids.status,release,/receipt_expired/);
  assert.deepEqual(await fs.readFile(saved.target_path),recreated);
  const validJournal=await fs.readFile(journal);
  try {
    await fs.writeFile(journal,JSON.stringify({...JSON.parse(validJournal),version:3}));
    await failed(ids.manage,ackRequest,/version_unsupported/);await failed(ids.status,ackRequest,/version_unsupported/);
    assert.deepEqual(await fs.readFile(saved.target_path),recreated);
  }finally{await fs.writeFile(journal,validJournal);}
  assert.deepEqual(await success(ids.manage,ackRequest),acknowledged);
  checks.push({check:'external-deletion-management-ack-retired-high-water-blocks-late-cached-delete-and-management-replays-and-future-journal-bypass',passed:true,
    receipt:acknowledged});
  const retiredRows=(await listing()).records.filter(row=>row.kind==='retired'&&row.process_incarnation===release.expected_fence.process_incarnation);
  assert.ok(retiredRows.length>0);
  await failed(ids.manage,await request({kind:'compact_retired',record_ids:[retiredRows[0].record_id]}),/process_still_current/);
  const afterAck=JSON.parse(await fs.readFile(journal));
  const oldProcess=release.expected_fence.process_incarnation===Number.MAX_SAFE_INTEGER?1:release.expected_fence.process_incarnation+1;
  const oldOrigin=digest(`owned-old-process-fixture:${randomUUID()}`);assert.ok(afterAck.retired.length<256);
  afterAck.retired.push({kind:'deletion',origin:oldOrigin,process_incarnation:oldProcess,high_water_request_id:1});afterAck.generation++;
  await fs.writeFile(journal,JSON.stringify(afterAck));
  const oldRows=(await listing()).records.filter(row=>row.kind==='retired'&&row.process_incarnation===oldProcess);
  assert.equal(oldRows.length,1);const compact=await request({kind:'compact_retired',record_ids:[oldRows[0].record_id]});
  const compacted=await success(ids.manage,compact);assert.deepEqual(await success(ids.status,compact),compacted);
  const reclaimFacts=JSON.parse(await fs.readFile(journal));
  const reclaimOrigin=digest(`owned-reclaim-fixture:${randomUUID()}`);
  reclaimFacts.retired.push({kind:'deletion',origin:reclaimOrigin,process_incarnation:oldProcess,high_water_request_id:1});reclaimFacts.generation++;
  await fs.writeFile(journal,JSON.stringify(reclaimFacts));
  const reclaimRows=(await listing()).records;
  const oldReclaim=reclaimRows.filter(row=>row.kind==='retired'&&row.process_incarnation===oldProcess);assert.equal(oldReclaim.length,1);
  const priorResult=reclaimRows.find(row=>row.kind==='management'&&row.request_id===ackRequest.request_id
    &&row.process_incarnation===ackRequest.expected_fence.process_incarnation);assert.ok(priorResult);
  const reclaimRequest=await request({kind:'reclaim',acknowledge_record_ids:[priorResult.record_id],compact_retired_record_ids:[oldReclaim[0].record_id]});
  const reclaimed=await success(ids.manage,reclaimRequest);assert.deepEqual(await success(ids.status,reclaimRequest),reclaimed);
  assert.deepEqual(await success(ids.manage,reclaimRequest),reclaimed);
  await failed(ids.manage,ackRequest,/receipt_expired/);await failed(ids.status,ackRequest,/receipt_expired/);
  assert.deepEqual(await fs.readFile(saved.target_path),recreated);
  checks.push({check:'external-deletion-management-atomic-reclaim-removes-only-selected-old-fence-and-confirmed-result-with-exact-replay',passed:true,
    receipt:reclaimed,nonclaims:['Native batch covers exact atomic management; simultaneous cap saturation is separately proven by Rust policy tests']});
  await failed(backup.remove,{...await deletion(saved.backup.id,digest(recreated)),expected_fence:{...deleteRequest.expected_fence,process_incarnation:oldProcess}},/unissued_fence/);
  assert.deepEqual(await fs.readFile(saved.target_path),recreated);await backend.invoke('delete_project_backup',{backupId:saved.backup.id});
  const final=JSON.parse(await fs.readFile(journal));
  for(const row of protectedFacts)assert.ok(final.records.some(value=>isDeepStrictEqual(value,row)),'Preserve every existing unowned deletion fact');
  for(const row of baseline.management)assert.ok(final.management.some(value=>isDeepStrictEqual(value,row)),'Preserve existing management results');
  for(const row of baseline.retired)assert.ok(final.retired.some(value=>isDeepStrictEqual(value,row)),'Preserve existing replay fences');
  assert.equal(final.retired.some(row=>row.origin===oldOrigin),false);
  assert.equal(final.retired.some(row=>row.origin===reclaimOrigin),false);
  assert.equal(final.records.some(row=>row.origin===fixtureOrigin),false);
  assert.equal(final.records.some(row=>row.origin===missingOrigin),false);
  assert.deepEqual(await state(),authorityBefore);
  checks.push({check:'external-deletion-management-current-process-compaction-rejected-old-process-only-compaction-and-all-unowned-facts-preserved',passed:true,
    journalSha256:digest(await fs.readFile(journal)),elapsedMs:performance.now()-started,
    nonclaims:['Prepared and retired old-process rows are explicitly owned fixtures, not actual crash or restart proof',
      'No artifact retention scheduling, audit export, power-loss or full storage/revocation matrix']});
}

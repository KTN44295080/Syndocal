import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import {createHash, randomUUID} from 'node:crypto';
import {isDeepStrictEqual} from 'node:util';
import {openNativeStdioSession} from './native-stdio-session.mjs';
import {nativeBackupDeletionManagement} from './native-backup-deletion-management.mjs';
import {exportNativeFileAudit} from './native-diagnostic-exports.mjs';

// Authenticated MCP with individually owned IDs, explicit acknowledgement and
// native cleanup. Never enter retention against existing unowned QA backups.
export async function nativeProjectBackup(backend, options, checks) {
  const ids={create:'syndocal.project.backup.create.v1',authority:'syndocal.query.project.backup.authority.v1',
    inspect:'syndocal.query.project.backup.inspect.v1', list:'syndocal.query.project.backup.list.v1',
    remove:'syndocal.project.backup.delete.v1',
    deleteStatus:'syndocal.query.project.backup.delete.status.v1',
    status:'syndocal.query.project.file.status.v1',ack:'syndocal.project.file.acknowledge.v1'};
  const directory=path.join(process.env.LOCALAPPDATA,options.profileId,'project-backups');
  const baseline=await fs.readdir(directory).catch(error=>{if(error.code==='ENOENT')return [];throw error;});
  const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
  const baselineHashes=new Map(await Promise.all(baseline.map(async name=>{
    const filename=path.join(directory,name),stat=await fs.lstat(filename);
    assert.ok(stat.isFile()&&!stat.isSymbolicLink(),'QA fixture requires regular existing artifacts');
    return [name,digest(await fs.readFile(filename))];
  })));
  const baselineSummaries=await backend.invoke('list_project_backups');
  assert.ok(baselineSummaries.length<10,'Do not enter retention against unowned QA backups');
  const created=[];
  const state=async()=>{
    const project=await backend.invoke('get_project_checkpoint',{midiMappings:[],oscMappings:[],dmxMappings:[]});
    const bundle=await backend.invoke('get_project_authority_bundle');delete bundle.snapshot;
    return {project,bundle,output:await backend.invoke('get_output_ownership_status')};
  };
  const mcp=await openNativeStdioSession(options);
  const grant=(capability,operationId)=>backend.invoke('agent_authority_grant_v1',{principalId:options.principalId,
    principalIncarnation:options.principalIncarnation,grant:{adapter:'external_mcp',capability,operation_id:operationId,project_id:null}});
  const send=async(operationId,request)=>{
    const requestId=randomUUID();let receipt=await mcp.call('syndocal_execute_control_plane',{requestId,operationId,request:{request}});
    const deadline=Date.now()+15000;
    while(receipt.status==='pending'&&Date.now()<deadline){await new Promise(resolve=>setTimeout(resolve,100));
      receipt=await mcp.call('syndocal_get_request_status',{requestId});}
    return receipt;
  };
  const success=async(operationId,request)=>{const receipt=await send(operationId,request);
    assert.equal(receipt.status,'completed',JSON.stringify(receipt));assert.equal(receipt.result.ok,true,JSON.stringify(receipt.result));return receipt.result.result;};
  const failed=async(operationId,request,pattern)=>{const before=await state(),receipt=await send(operationId,request);
    assert.equal(receipt.status,'completed');assert.equal(receipt.result.ok,false,JSON.stringify(receipt.result));
    assert.match(receipt.result.error.message,pattern);assert.deepEqual(await state(),before);};
  const prepare=async()=>{const authority=await success(ids.authority,{schema_version:1});assert.equal(authority.target_sha256,null);
    return {schema_version:1,operation_id:ids.create,request_id:authority.next_request_id,expected_fence:authority.fence,
      expected_path_generation:authority.path_generation,expected_disposition_generation:authority.disposition_generation,
      destination:authority.destination,expected_target_sha256:null};};
  let deleteSequence=1;
  let deletionFence;
  const deletion=async(id,hash)=>({schema_version:1,operation_id:ids.remove,request_id:deleteSequence++,
    expected_fence:structuredClone(deletionFence),backup_id:id,expected_artifact_sha256:hash});
  try {
    const registry=await backend.invoke('get_control_plane_canonical_registry');
    const deleteOperation=registry.canonical_operations.find(value=>value.operation_id===ids.remove);
    assert.equal(deleteOperation.risk,'r5');assert.equal(deleteOperation.adapter_policy,'local_window_project_publication');
    const deniedDelete=await send(ids.remove,{schema_version:1});
    assert.equal(deniedDelete.status,'rejected');assert.equal(deniedDelete.error,'agent_missing_grant');
    await grant('file',ids.remove);
    const operation=registry.canonical_operations.find(value=>value.operation_id===ids.create);
    assert.equal(operation.risk,'r5');assert.equal(operation.adapter_policy,'local_window_project_publication');
    assert.equal(operation.audit,'immutable');assert.equal(operation.receipt_policy,'exact_terminal_receipt');
    const deniedRead=await send(ids.authority,{schema_version:1});assert.equal(deniedRead.status,'rejected');assert.equal(deniedRead.error,'agent_missing_grant');
    await grant('read',ids.authority);const request=await prepare();const deniedCreate=await send(ids.create,request);
    assert.equal(deniedCreate.status,'rejected');assert.equal(deniedCreate.error,'agent_missing_grant');await grant('file',ids.create);
    checks.push({check:'external-backup-exact-r0-read-and-r5-file-grants-without-human-approval',passed:true});
    const inspectionOperation=registry.canonical_operations.find(value=>value.operation_id===ids.inspect);
    assert.equal(inspectionOperation.risk,'r0');assert.equal(inspectionOperation.adapter_policy,'local_window_read_only');
    const deniedInspection=await send(ids.inspect,{schema_version:1,backup_id:1});
    assert.equal(deniedInspection.status,'rejected');assert.equal(deniedInspection.error,'agent_missing_grant');
    await grant('read',ids.inspect);
    for(const value of [{schema_version:2,backup_id:1},{schema_version:1,backup_id:0},
      {schema_version:1,backup_id:Number.MAX_SAFE_INTEGER+1}]){
      await failed(ids.inspect,value,/invalid project backup inspection request/);
    }
    for(const field of ['path','destination','owner_id','principal','skip_confirmation']){
      await failed(ids.inspect,{schema_version:1,backup_id:1,[field]:'forged'},/request_invalid/);
    }
    checks.push({check:'external-backup-inspection-exact-r0-grant-and-strict-id-only-ingress',passed:true});
    const listRequest={schema_version:1,limit:16,before_id:null};
    const listOperation=registry.canonical_operations.find(value=>value.operation_id===ids.list);
    assert.equal(listOperation.risk,'r0');assert.equal(listOperation.adapter_policy,'local_window_read_only');
    const deniedList=await send(ids.list,listRequest);assert.equal(deniedList.status,'rejected');assert.equal(deniedList.error,'agent_missing_grant');
    await grant('read',ids.list);
    for(const value of [{...listRequest,schema_version:2},{...listRequest,limit:0},{...listRequest,limit:17},
      {...listRequest,before_id:0},{...listRequest,before_id:Number.MAX_SAFE_INTEGER+1}]){
      await failed(ids.list,value,/invalid project backup list request/);
    }
    for(const field of ['path','destination','owner_id','principal','skip_confirmation']){
      await failed(ids.list,{...listRequest,[field]:'forged'},/request_invalid/);
    }
    checks.push({check:'external-backup-list-exact-r0-grant-and-strict-bounded-id-pagination-ingress',passed:true});
    await failed(ids.authority,{schema_version:2},/backup authority schema/);
    await failed(ids.authority,{schema_version:1,destination:'C:/forged'},/request_invalid/);
    await failed(ids.create,{...request,owner_id:'forged'},/request_invalid/);
    await failed(ids.create,{...request,expected_target_sha256:'a'.repeat(64)},/invalid project file request/);
    await failed(ids.create,{...request,destination:path.join(path.dirname(directory),'backup-1.json')},/backup_authority_expired_or_unknown/);
    await failed(ids.create,{...request,expected_disposition_generation:request.expected_disposition_generation+1},/stale_fence/);
    checks.push({check:'external-backup-future-forged-replace-outside-and-stale-request-rejection',passed:true});
    assert.match(path.basename(request.destination),/^backup-[1-9][0-9]*\.json$/);
    const before=await state(),start=performance.now();const saved=await success(ids.create,request);
    assert.equal(saved.phase,'succeeded');created.push({request,receipt:saved,acknowledged:false});
    assert.equal(saved.backup.reason,'MCP backup');assert.equal(saved.backup.source_path,before.bundle.current_project_path);
    const bytes=await fs.readFile(saved.target_path);assert.equal(saved.backup.bytes,bytes.length);assert.equal(saved.artifact_sha256,digest(bytes));
    const envelope=JSON.parse(bytes),combined={...envelope.project};
    assert.equal(envelope.id,saved.backup.id);assert.equal(envelope.reason,'MCP backup');assert.equal(envelope.source_path,before.bundle.current_project_path);
    for(const key of ['midi_mappings','osc_mappings','dmx_mappings','dj_track_triggers']){
      assert.deepEqual(envelope[key]??[],before.project[key]??[],key);if(envelope[key]?.length)combined[key]=envelope[key];}
    const expected=structuredClone(before.project);
    // Known native f32 BPM serialization boundary; all other leaves stay exact.
    combined.snapshot.clock.bpm=Math.fround(combined.snapshot.clock.bpm);expected.snapshot.clock.bpm=Math.fround(expected.snapshot.clock.bpm);
    assert.deepEqual(combined,expected,'Complete project and all mappings');
    assert.deepEqual(await state(),before,'Backup keeps project/path/disposition and closed output authority');
    checks.push({check:'external-backup-full-project-byte-hash-source-metadata-and-unchanged-authority',passed:true,
      receipt:saved,bytes:bytes.length,elapsedMs:performance.now()-start});
    const inspectStarted=performance.now();
    const inspected=await success(ids.inspect,{schema_version:1,backup_id:saved.backup.id});
    assert.equal(inspected.schema_version,1);assert.deepEqual(inspected.backup,saved.backup);
    assert.equal(inspected.artifact_sha256,digest(bytes));assert.equal(inspected.restore_source_path,saved.backup.source_path);
    assert.deepEqual(await state(),before);assert.deepEqual(await fs.readFile(saved.target_path),bytes);
    checks.push({check:'external-backup-inspection-same-byte-hash-metadata-restoration-path-and-read-purity',passed:true,
      inspection:inspected,elapsedMs:performance.now()-inspectStarted});
    const listStarted=performance.now(),listing=await success(ids.list,listRequest);
    assert.equal(listing.schema_version,1);assert.equal(listing.next_before_id,null);
    assert.deepEqual(listing.backups.map(row=>row.backup),await backend.invoke('list_project_backups'));
    for(const row of listing.backups)assert.equal(row.artifact_sha256,digest(await fs.readFile(path.join(directory,`backup-${row.backup.id}.json`))));
    const pages=[];let beforeId=null;
    for(let page=0;page<16;page++){
      const result=await success(ids.list,{schema_version:1,limit:1,before_id:beforeId});
      assert.ok(result.backups.length<=1);pages.push(...result.backups);
      if(result.next_before_id===null){beforeId=null;break;}
      assert.equal(result.next_before_id,result.backups.at(-1).backup.id);beforeId=result.next_before_id;
    }
    assert.equal(beforeId,null,'Bounded pages must finish');assert.deepEqual(pages,listing.backups);
    assert.deepEqual(await state(),before);assert.deepEqual(await fs.readFile(saved.target_path),bytes);
    checks.push({check:'external-backup-list-descending-pagination-original-digests-and-read-purity',passed:true,
      itemCount:listing.backups.length,elapsedMs:performance.now()-listStarted});
    const competingWriter=await fs.open(saved.target_path,'r+');
    try { await failed(ids.inspect,{schema_version:1,backup_id:saved.backup.id},/inspect_open/);
      await failed(ids.list,listRequest,/inspect_open/); }
    finally { await competingWriter.close(); }
    assert.deepEqual(await success(ids.inspect,{schema_version:1,backup_id:saved.backup.id}),inspected);
    assert.deepEqual(await fs.readFile(saved.target_path),bytes);
    checks.push({check:'external-backup-inspection-competing-writer-rejection-and-explicit-read-recovery',passed:true});
    assert.deepEqual(await success(ids.status,request),saved);assert.deepEqual(await success(ids.create,request),saved);
    await failed(ids.create,{...request,destination:request.destination.replace(/backup-[0-9]+\.json$/,'backup-1.json')},/shape_conflict/);
    const protectedDelete=await backend.evaluate(`window.__TAURI_INTERNALS__.invoke('delete_project_backup', ${JSON.stringify({backupId:saved.backup.id})}).then(value=>({value}),error=>({error:String(error).slice(0,768)}))`);
    assert.match(protectedDelete.error,/publication|receipt|referenced/i);assert.deepEqual(await fs.readFile(saved.target_path),bytes);
    // Backup publication/inspection and acknowledgement leave project authority
    // unchanged (asserted above/below). Reuse its real issued fence instead of
    // issuing an unrelated backup destination for every deletion rejection.
    deletionFence=structuredClone(request.expected_fence);
    const canonicalProtectedDelete=await deletion(saved.backup.id,saved.artifact_sha256);
    const statusOperation=registry.canonical_operations.find(value=>value.operation_id===ids.deleteStatus);
    assert.equal(statusOperation.risk,'r0');assert.equal(statusOperation.adapter_policy,'local_window_read_only');
    const deniedStatus=await send(ids.deleteStatus,canonicalProtectedDelete);
    assert.equal(deniedStatus.status,'rejected');assert.equal(deniedStatus.error,'agent_missing_grant');
    await grant('read',ids.deleteStatus);
    const unknown=await success(ids.deleteStatus,canonicalProtectedDelete);
    assert.equal(unknown.phase,'unknown');assert.equal(unknown.receipt,null);assert.equal(unknown.error,null);
    for(const field of ['path','origin','owner_id','principal','skip_confirmation']){
      await failed(ids.deleteStatus,{...canonicalProtectedDelete,[field]:true},/arguments_invalid/);
    }
    await failed(ids.deleteStatus,{...canonicalProtectedDelete,schema_version:2},/invalid project backup delete request/);
    assert.deepEqual(await fs.readFile(saved.target_path),bytes);
    checks.push({check:'external-backup-delete-status-exact-r0-grant-and-strict-request-only-read-purity',passed:true});
    await failed(ids.remove,canonicalProtectedDelete,/unacknowledged/);
    assert.deepEqual(await fs.readFile(saved.target_path),bytes);
    await failed(ids.remove,{...await deletion(saved.backup.id,saved.artifact_sha256),expected_artifact_sha256:'f'.repeat(64)},/artifact_changed/);
    const forgedDelete=await deletion(saved.backup.id,saved.artifact_sha256);forgedDelete.expected_fence.session_incarnation++;
    await failed(ids.remove,forgedDelete,/unissued_fence/);
    const deleteShape=await deletion(saved.backup.id,saved.artifact_sha256);
    for(const value of [{...deleteShape,schema_version:2},{...deleteShape,request_id:0},
      {...deleteShape,backup_id:0},{...deleteShape,backup_id:Number.MAX_SAFE_INTEGER+1},
      {...deleteShape,expected_artifact_sha256:'A'.repeat(64)}]){
      await failed(ids.remove,value,/invalid project backup delete request/);
    }
    for(const field of ['path','owner_id','principal','skip_confirmation']){
      await failed(ids.remove,{...await deletion(saved.backup.id,saved.artifact_sha256),[field]:true},/arguments_invalid/);
    }
    checks.push({check:'external-backup-delete-exact-r5-grant-no-human-confirmation-and-unacknowledged-hash-forged-fence-preservation',passed:true});
    const ack=await success(ids.ack,request);assert.equal(ack.phase,'acknowledged');created[0].acknowledged=true;
    assert.deepEqual(await success(ids.ack,request),ack);assert.deepEqual(await success(ids.status,request),ack);
    const deleteRequest=await deletion(saved.backup.id,saved.artifact_sha256),deleteBefore=await state(),deleteStarted=performance.now();
    const deleted=await success(ids.remove,deleteRequest);created[0].deleted=true;
    assert.deepEqual(deleted.request,deleteRequest);assert.equal(deleted.deleted_backup.backup.id,saved.backup.id);
    assert.equal(deleted.deleted_backup.artifact_sha256,saved.artifact_sha256);
    await assert.rejects(fs.stat(saved.target_path),/ENOENT/);assert.deepEqual(await state(),deleteBefore);
    await exportNativeFileAudit(mcp,grant,state,checks,ids.remove,'backup_deleted');
    const terminalStatus=await success(ids.deleteStatus,deleteRequest);
    assert.equal(terminalStatus.phase,'succeeded');assert.deepEqual(terminalStatus.request,deleteRequest);
    assert.deepEqual(terminalStatus.receipt,deleted);assert.equal(terminalStatus.error,null);
    assert.deepEqual(await success(ids.remove,deleteRequest),deleted);
    const recreated=Buffer.from('owned recreated artifact must survive exact replay');await fs.writeFile(saved.target_path,recreated);
    try {
      assert.deepEqual(await success(ids.remove,deleteRequest),deleted);
      assert.deepEqual(await fs.readFile(saved.target_path),recreated);
      await failed(ids.remove,{...deleteRequest,expected_artifact_sha256:'d'.repeat(64)},/different shape/);
      assert.deepEqual(await fs.readFile(saved.target_path),recreated);
      const deletionJournal=path.join(path.dirname(directory),'project-backup-deletions-v1.json');
      const originalJournal=await fs.readFile(deletionJournal),facts=JSON.parse(originalJournal);
      const owned=facts.records.find(record=>isDeepStrictEqual(record.receipt.request,deleteRequest));
      assert.ok(owned,'The exact native request must have a durable terminal');assert.equal(owned.phase,'succeeded');
      assert.deepEqual(owned.receipt,deleted);assert.equal(owned.error,null);
      try {
        await fs.writeFile(deletionJournal,Buffer.from(JSON.stringify({...facts,version:3})));
        await failed(ids.deleteStatus,deleteRequest,/journal_version_unsupported/);
        await failed(ids.remove,deleteRequest,/journal_version_unsupported/);
        assert.deepEqual(await fs.readFile(saved.target_path),recreated);
        await fs.writeFile(deletionJournal,originalJournal);
        await fs.writeFile(saved.target_path,bytes);owned.phase='prepared';
        await fs.writeFile(deletionJournal,Buffer.from(JSON.stringify(facts)));
        const unresolved=await success(ids.deleteStatus,deleteRequest);
        assert.equal(unresolved.phase,'indeterminate');assert.equal(unresolved.receipt,null);assert.match(unresolved.error,/unresolved/);
        await failed(ids.remove,deleteRequest,/indeterminate/);
        await failed(ids.remove,await deletion(saved.backup.id,saved.artifact_sha256),/unresolved_journal_protects/);
        const local=await backend.evaluate(`window.__TAURI_INTERNALS__.invoke('delete_project_backup',${JSON.stringify({backupId:saved.backup.id})}).then(value=>({value}),error=>({error:String(error)}))`);
        assert.match(local.error,/unresolved_journal_protects/);assert.deepEqual(await fs.readFile(saved.target_path),bytes);
      } finally {await fs.writeFile(deletionJournal,originalJournal);await fs.writeFile(saved.target_path,recreated);}
      assert.deepEqual(await fs.readFile(deletionJournal),originalJournal);
      assert.deepEqual(await success(ids.deleteStatus,deleteRequest),terminalStatus);
      assert.deepEqual(await success(ids.remove,deleteRequest),deleted);
      assert.deepEqual(await fs.readFile(saved.target_path),recreated);
      checks.push({check:'external-backup-delete-durable-prepared-and-future-journal-protect-cached-replay-and-new-intent-without-repair',passed:true,
        journalSha256:digest(originalJournal)});
    } finally {await backend.invoke('delete_project_backup',{backupId:saved.backup.id});}
    checks.push({check:'external-backup-delete-same-artifact-receipt-and-exact-replay-preserve-project-authority-output-and-recreated-bytes',passed:true,
      receipt:deleted,elapsedMs:performance.now()-deleteStarted});
    await failed(ids.inspect,{schema_version:1,backup_id:saved.backup.id},/inspect_metadata/);
    await nativeBackupDeletionManagement(backend,checks,{send,success,failed,grant,state,directory,saved,bytes,
      deleted,deleteRequest,deletion,ids});
    await exportNativeFileAudit(mcp,grant,state,checks,'syndocal.project.backup.delete.journal.manage.v1','deletion_journal_managed');
    const next=await prepare();assert.equal(next.request_id,2);const second=await success(ids.create,next);
    assert.equal(second.phase,'succeeded');created.push({request:next,receipt:second,acknowledged:false});
    await success(ids.ack,next);created[1].acknowledged=true;
    checks.push({check:'external-backup-exact-replay-status-protected-delete-ack-reack-and-next-sequence',passed:true});
    const original=await fs.readFile(second.target_path),secondEnvelope=JSON.parse(original);
    const caseAlias=path.join(directory,`BACKUP-${second.backup.id}.json`);
    await fs.rename(second.target_path,caseAlias);
    try {
      assert.ok((await fs.readdir(directory)).includes(path.basename(caseAlias)),'Native OS must expose the owned case rename');
      await failed(ids.list,listRequest,/project_backup_list_filename_invalid/);
      assert.deepEqual(await fs.readFile(caseAlias),original);
    } finally { await fs.rename(caseAlias,second.target_path); }
    try {
      for(const [name,bytes,pattern] of [
        ['filename-id-mismatch',Buffer.from(JSON.stringify({...secondEnvelope,id:second.backup.id+1})),/does not match filename ID/],
        ['future-envelope',Buffer.from(JSON.stringify({...secondEnvelope,version:2})),/Unsupported project backup version/],
        ['ignored-duplicate-key',Buffer.from(`{"future":0,"future":1,${original.toString('utf8').trimStart().slice(1)}`),/duplicate object key/],
        ['invalid-utf8',Buffer.from([255,123]),/not valid UTF-8/],
        ['unsafe-metadata',Buffer.from(JSON.stringify({...secondEnvelope,created_at_unix_ms:Number.MAX_SAFE_INTEGER+1})),/invalid project backup inspection metadata/],
      ]) {
        await fs.writeFile(second.target_path,bytes);
        await failed(ids.inspect,{schema_version:1,backup_id:second.backup.id},pattern);
        await failed(ids.list,listRequest,pattern);
        assert.deepEqual(await fs.readFile(second.target_path),bytes,name);
      }
      const file=await fs.open(second.target_path,'w');
      try { await file.truncate(128*1024*1024+1); } finally { await file.close(); }
      await failed(ids.inspect,{schema_version:1,backup_id:second.backup.id},/limit is 134217728/);
      await failed(ids.list,listRequest,/limit is 134217728/);
      assert.equal((await fs.stat(second.target_path)).size,128*1024*1024+1);
    } finally { await fs.writeFile(second.target_path,original); }
    assert.equal((await success(ids.inspect,{schema_version:1,backup_id:second.backup.id})).artifact_sha256,digest(original));
    checks.push({check:'external-backup-inspection-missing-mismatched-future-duplicate-utf8-unsafe-and-oversize-rejection-without-writes',passed:true});
    assert.equal((await success(ids.list,listRequest)).backups.find(row=>row.backup.id===second.backup.id).artifact_sha256,digest(original));
    checks.push({check:'external-backup-list-case-alias-writer-corrupt-future-duplicate-utf8-unsafe-and-oversize-fail-closed-without-writes',passed:true});
  } finally {
    try {
      for(const entry of created){if(!entry.acknowledged){await success(ids.ack,entry.request);entry.acknowledged=true;}
        if(!entry.deleted)await backend.invoke('delete_project_backup',{backupId:entry.receipt.backup.id});}
    } finally { await mcp.close(); }
    assert.deepEqual((await fs.readdir(directory)).sort(),[...baseline].sort(),'Only owned backup IDs were removed');
    for(const [name,hash] of baselineHashes)assert.equal(digest(await fs.readFile(path.join(directory,name))),hash,name);
    assert.deepEqual(await backend.invoke('list_project_backups'),baselineSummaries);
  }
  checks.push({check:'external-backup-owned-artifact-cleanup-preserves-all-existing-managed-files',passed:true});
}

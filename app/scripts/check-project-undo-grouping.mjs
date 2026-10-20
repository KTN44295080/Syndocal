import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
import { typescriptFixtureModuleUrl } from "./typescript-fixture-module.mjs";
const { createProjectUndoGroup } = await import(await typescriptFixtureModuleUrl(new URL("../src/projectUndoGrouping.ts", import.meta.url)));
const { preflightAndBeginMediaAssetOperation } = await import(await typescriptFixtureModuleUrl(new URL("../src/mediaAssetAuthority.ts", import.meta.url)));
const controller = await readFile(new URL("../src/createVideoRuntimeController.ts", import.meta.url), "utf8");
const extract = (name, nextName, deps) => {
 const start=controller.indexOf("  const "+name+" = async (");
 const end=controller.indexOf("  const "+nextName+" = async (",start+1);
 assert.ok(start>=0 && end>start, "production "+name+" helper is present");
 const code=ts.transpileModule(controller.slice(start,end)+"\nreturn "+name+";",{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
 return new Function(...Object.keys(deps),code)(...Object.values(deps));
};
const same=(a,b)=>a.project_epoch===b.project_epoch && a.project_revision===b.project_revision && a.checkpoint_hash===b.checkpoint_hash;
const token=(revision=10,epoch=4)=>({project_epoch:epoch,project_revision:revision,checkpoint_hash:"h"+revision});
const A=token(),B=token(11),C=token(12);
{
 const group=createProjectUndoGroup(A,"test-group");
 assert.equal(group.isCurrent(A),true);
 assert.equal(group.acceptPublication(B,B),true);
 assert.equal(group.isCurrent(B),true);
 assert.equal(group.isCurrent(C),false,"an unrelated revision closes the group");
 assert.equal(group.isCurrent(B),false,"a closed group cannot be resurrected by a stale token");
 const replaced=createProjectUndoGroup(A,"next-group");
 assert.equal(replaced.acceptPublication(token(11,5),token(11,5)),false,"project epochs cannot cross");
 const stale=createProjectUndoGroup(A,"stale");
 assert.equal(stale.acceptPublication(B,C),false,"superseded terminal publication closes the group");
 const closed=createProjectUndoGroup(A,"closed"); closed.close(); assert.equal(closed.isCurrent(A),false);
 assert.notEqual(createProjectUndoGroup(A).id,createProjectUndoGroup(A).id,"separate gestures never reuse a nonce");
}
async function scenario({failureAt,abortAt,interleaveAfter,preflightInterleaveAt}={}) {
 let current=token(); let calls=[]; let released=0; let preflights=0; let refreshes=0;
 const requests=[1,2,3].map(assetId=>({assetId,label:"Asset "+assetId,previousPath:"old-"+assetId,replacementPath:"new-"+assetId}));
 const assets=requests.map(r=>({id:r.assetId,label:r.label,source:{kind:"File",path:r.previousPath}}));
 const options={
  getCurrentProjectAuthority:()=>({...current}), isProjectAuthorityCurrent:a=>same(a,current),
  snapshot:()=>({video:{media_assets:assets}}), projectTransactionOwnerId:"owner",
  prepareMediaAssetOperationStart:async()=>{preflights++; if(preflights===preflightInterleaveAt) current=token(current.project_revision+1); return {authority:{...current},provenance:"test"};},
  beginMediaAssetOperation:()=>({signal:new AbortController().signal,setPhase:()=>{},release:()=>{released++;}}),
  refreshSnapshot:async()=>{refreshes++; if(refreshes===interleaveAfter)current=token(current.project_revision+1);},
 };
 const run=extract("runMediaAssetRelink","relinkMediaAsset",{
  options,preflightAndBeginMediaAssetOperation,sameProjectAuthority:same,
  prepareFinalizeAndCommitMediaAssetRelink:async args=>{
   calls.push(args); const index=calls.length;
   if(index===failureAt)throw new Error("hash failed");
   if(index===abortAt)throw new DOMException("cancelled","AbortError");
   assert.deepEqual(args.expectedAuthority,current,"each file keeps its exact preflight authority");
   current=token(current.project_revision+1);
   return {committed:true,applicationCurrent:true,terminalAuthority:{...current},outcome:{kind:"relinked",asset_id:args.assetId}};
  },
 });
 const apply=extract("applyMediaRelinkRequests","setVideoLayerIsfEffect",{
  options,createProjectUndoGroup,runMediaAssetRelink:run,mediaAssetRelinkOutcomeMessage:outcome=>outcome.kind,
 });
 const result=await apply(requests);
 return {result,calls,released,current};
}
{
 const {result,calls,released}=await scenario();
 assert.deepEqual(result.result.relinked,[1,2,3]); assert.equal(released,3);
 assert.equal(new Set(calls.map(c=>c.undoGroupId)).size,1,"all files carry one explicit undo group");
 assert.ok(calls[0].undoGroupId,"group metadata is nonempty");
 assert.deepEqual(calls.map(c=>c.expectedAuthority.project_revision),[10,11,12],"grouping does not reuse publication tickets");
 assert.equal(result.terminalAuthority.project_revision,13);
}
for(const failure of [{failureAt:2},{abortAt:2}]) {
 const {result,calls,released}=await scenario(failure);
 assert.deepEqual(result.result.relinked,[1]); assert.deepEqual(result.result.skipped,[3]); assert.equal(result.result.failed[0].assetId,2);
 assert.equal(result.terminalAuthority.project_revision,11,"successful prefix remains committed and undoable");
 assert.equal(calls.length,2); assert.equal(released,2,"failed/cancelled member releases its lease");
 assert.equal(calls[0].undoGroupId,calls[1].undoGroupId);
}
{
 const {result,calls}=await scenario({interleaveAfter:1});
 assert.deepEqual(result.result.relinked,[1]); assert.deepEqual(result.result.skipped,[2,3]); assert.equal(calls.length,1,"unrelated publication prevents further grouped relinks");
}
{
 const {result,calls,released}=await scenario({preflightInterleaveAt:2});
 assert.deepEqual(result.result.relinked,[1]); assert.equal(result.result.failed[0].assetId,2); assert.deepEqual(result.result.skipped,[3]);
 assert.equal(calls.length,1,"mapping flush changing authority aborts before the next hash or publication");
 assert.equal(released,2,"post-preflight group rejection still releases the lease");
}
console.log("project undo grouping contract ok: contiguous folder publications, cancellation/failure prefix, authority interleaving, and group lifecycle");

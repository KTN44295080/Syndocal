import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
const source = await readFile(new URL("../src/App.tsx",import.meta.url),"utf8");
const section = (start,end) => {
  const a=source.indexOf(start),b=source.indexOf(end,a);
  assert.ok(a>=0&&b>a,`missing ${start}`);return source.slice(a,b);
};
const functions=section("  const performUndoProject =", "  const [projectHistoryNavigationBusy,")
  +section("  const navigateProjectHistory =", "  const hasPendingHistoryMappings =")
  +"\nreturn {undoProject,redoProject};";
const code=ts.transpileModule(functions,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return {promise,resolve,reject};};
function fixture() {
  let busy=false,confirmed=true,epoch=7,pulse=false;
  let history={can_undo:false,can_redo:false,undo_entry_id:10,undo_checkpoint_hash:'old',redo_entry_id:11,redo_checkpoint_hash:'redo'};
  const calls=[];let flush=async()=>epoch, refresh=async()=>true;
  const ports={projectHistoryNavigationBusy:()=>busy,setProjectHistoryNavigationBusy:v=>{busy=v;},isfEventPulseBusy:()=>pulse,
    confirmDiscardTimelineEditorDrafts:()=>confirmed,isTauriRuntime:()=>true,viewportFixture:null,
    flushProjectControlMappingsBeforeMutation:async()=>{calls.push('flush');return flush();},
    refreshProjectHistoryStatus:async()=>{calls.push('refresh');return refresh();},
    captureProjectAuthorityIdentity:()=>({project_epoch:epoch}),projectHistoryStatus:()=>history,
    setMessage:m=>calls.push(['message',m]),pollProjectAuthorityBundle:()=>calls.push('poll'),
    projectTransactionOwnerId:'owner',
    invoke:async(command,args)=>{calls.push([command,args]);return {authority:{},history_status:{...history,redo_label:'mapping',undo_label:'mapping'}};},
    applyAuthorityBundleAsReplacement:()=>true,projectAuthorityApplicationResultIsCurrent:()=>true,
    applyAuthoritativeProjectHistoryStatus:s=>{history=s;return true;}};
  const actions=new Function(...Object.keys(ports),code)(...Object.values(ports));
  return {...actions,calls,get busy(){return busy;},set confirmed(v){confirmed=v;},set epoch(v){epoch=v;},set pulse(v){pulse=v;},
    get history(){return history;},set history(v){history=v;},set flush(v){flush=v;},set refresh(v){refresh=v;}};
}
const commands=f=>f.calls.filter(v=>Array.isArray(v)&&v[0].endsWith('_project_transaction'));
const workCalls=f=>f.calls.filter(v=>!Array.isArray(v)||v[0]!=='message');
{
  const f=fixture();f.confirmed=false;await f.undoProject();assert.deepEqual(workCalls(f),[]);assert.equal(f.busy,false);
  assert.match(f.calls[0][1],/未保存/);
  f.confirmed=true;f.pulse=true;await f.undoProject();assert.deepEqual(workCalls(f),[]);
}
{
  const f=fixture();f.flush=async()=>{throw Error('untrusted');};await f.undoProject();
  assert.equal(f.calls[0],'flush');assert.ok(!f.calls.includes('refresh'));assert.equal(commands(f).length,0);assert.equal(f.busy,false);
}
for(const cause of ['stale','epoch']) {
  const f=fixture();f.refresh=async()=>{if(cause==='epoch')f.epoch=8;return cause!=='stale';};
  await f.undoProject();assert.equal(commands(f).length,0);assert.equal(f.busy,false);
}
{
  const f=fixture();assert.equal(f.history.can_undo,false);
  f.refresh=async()=>{f.history={...f.history,can_undo:true,undo_entry_id:99,undo_checkpoint_hash:'fresh'};return true;};
  await f.undoProject();assert.deepEqual(f.calls.slice(0,2),['flush','refresh']);
  assert.deepEqual(commands(f),[['undo_project_transaction',{ownerId:'owner',expectedEpoch:7,expectedEntryId:99,expectedCheckpointHash:'fresh'}]]);
}
{
  const f=fixture();f.history={...f.history,can_redo:true};
  f.refresh=async()=>{f.history={...f.history,can_redo:false};return true;};
  await f.redoProject();assert.equal(commands(f).length,0);
}
{
  const f=fixture(),pending=deferred();f.history={...f.history,can_undo:true};f.flush=()=>pending.promise;
  const first=f.undoProject();assert.equal(f.busy,true);await f.undoProject();await f.redoProject();
  assert.deepEqual(workCalls(f),['flush']);pending.resolve(7);await first;assert.equal(commands(f).length,1);assert.equal(f.busy,false);
}
assert.match(source,/can_undo:[\s\S]{0,180}hasPendingHistoryMappings\(\)/);
assert.match(source,/can_redo:[\s\S]{0,180}!hasPendingHistoryMappings\(\)/);
// All synchronization-state replacement must publish the header signal too.
const ast=ts.createSourceFile('App.tsx',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
const writes=[];
function visit(node) {
  if(ts.isBinaryExpression(node)&&node.operatorToken.kind===ts.SyntaxKind.EqualsToken
    &&node.left.getText(ast)==='projectAuthoritySync')writes.push(node.getText(ast));
  if(ts.isBinaryExpression(node)&&node.operatorToken.kind===ts.SyntaxKind.EqualsToken
    &&node.left.getText(ast).startsWith('projectAuthoritySync.'))assert.fail('unpublished in-place synchronization state write');
  ts.forEachChild(node,visit);
}
visit(ast);assert.deepEqual(writes,['projectAuthoritySync = next']);
assert.match(source,/const hasPendingHistoryMappings = projectHistoryMappingsDirty;/);
const stateSource=await readFile(new URL('../src/projectAuthority.ts',import.meta.url),'utf8');
const stateCode=ts.transpileModule(stateSource,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText;
const transitions=await import(`data:text/javascript;base64,${Buffer.from(stateCode).toString('base64')}`);
const require=createRequire(import.meta.url);
const solid=await import(pathToFileURL(require.resolve('solid-js/dist/solid.js')).href);
const signalBlock=section('  let projectAuthoritySync = createProjectAuthoritySyncState();','  // Coherent authority publication');
const signalCode=ts.transpileModule(signalBlock+ '\nreturn {assignProjectAuthoritySync, projectHistoryMappingsDirty};',
  {compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
let writeMapping,assign,headerDirty,visibleDirty=false,liveState=transitions.createProjectAuthoritySyncState(),dispose;
solid.createRoot(d=>{
  dispose=d;
  const [mapping,setMapping]=solid.createSignal(0);writeMapping=setMapping;
  const api=new Function('createSignal','createProjectAuthoritySyncState','projectAuthorityHasDirtyMappings',signalCode)(
    solid.createSignal,transitions.createProjectAuthoritySyncState,transitions.projectAuthorityHasDirtyMappings);
  assign=api.assignProjectAuthoritySync;headerDirty=api.projectHistoryMappingsDirty;
  solid.createEffect(()=>{if(mapping()>0){liveState=transitions.noteLocalProjectAuthorityEdit(liveState);assign(liveState);}});
  // Real render-effect ordering previously read the plain state before dirty
  // detection's user effect and left Undo disabled until another update.
  solid.createRenderEffect(()=>{mapping();visibleDirty=headerDirty();});
});
assert.equal(visibleDirty,false);
writeMapping(1);assert.equal(visibleDirty,true);
liveState=transitions.markProjectAuthorityPersisted(liveState);assign(liveState);assert.equal(visibleDirty,false);
writeMapping(2);assert.equal(visibleDirty,true);
liveState=transitions.invalidateProjectAuthorityIdentity(liveState);assign(liveState);
assert.equal(visibleDirty,transitions.projectAuthorityHasDirtyMappings(liveState));
dispose();
console.log('Project history preflight checks passed: production wrapper and Undo/Redo, cancel, busy, flush failure, stale/epoch fence, initial dirty mapping, current entry CAS and invalidated redo branch.');

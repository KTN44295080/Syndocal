import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import ts from "typescript";
const require = createRequire(import.meta.url);
const solidUrl = pathToFileURL(require.resolve("solid-js/dist/solid.js")).href;
const {createRoot,createSignal,batch} = await import(solidUrl);
const source = await readFile(new URL("../src/createWorkspaceNavigationController.ts",import.meta.url),"utf8");
const code = ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2022}}).outputText
  .replaceAll('from "solid-js"',`from "${solidUrl}"`);
const {createWorkspaceNavigationController:create} = await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`);
const A={workspace:'setup',setupSubTab:'patch'}, B={workspace:'control',controlMode:'live',timelineDeskSurface:'show',timelineChildCueId:null};
const C={...B,timelineChildCueId:42};
function fixture(session='one') {
  const states=[{external:true}]; let cursor=0, listener, dispose, setRoute, setEpoch, route, reject=false, fail=false;
  const pending=[], restores=[];
  const history={get state(){return states[cursor];},replaceState(s){states[cursor]=structuredClone(s);},
    pushState(s){states.splice(cursor+1); states.push(structuredClone(s));cursor++;},
    go(n){const target=cursor+n;if(target<0||target>=states.length)return;cursor=target;pending.push(states[cursor]);}};
  const events={addEventListener(t,fn){assert.equal(t,'popstate');listener=fn;},removeEventListener(t,fn){assert.equal(listener,fn);listener=null;}};
  createRoot(d=>{dispose=d;[route,setRoute]=createSignal(A);const [epoch,write]=createSignal(1);setEpoch=write;
    create({route,projectEpoch:epoch,history,events,sessionId:session,restore:r=>{restores.push(r);if(fail)throw Error('rejected');if(reject)return false;setRoute(r);return true;}});
  });
  const flush=()=>{for(let i=0;pending.length;i++){assert.ok(i<20,'history bounce loop');listener?.({state:pending.shift()});}};
  return {states,history,restores,flush,dispose,setRoute,setEpoch,route,set reject(v){reject=v;},set fail(v){fail=v;},get cursor(){return cursor;},get listener(){return listener;},inject(state){listener({state});}};
}
const f=fixture();
assert.equal(f.states.length,2);assert.equal(f.cursor,1);
f.history.go(-1);f.flush();assert.equal(f.cursor,1);assert.equal(f.restores.length,0);
f.setRoute(B);f.setRoute(C);assert.equal(f.states.length,4);
f.history.go(-1);f.flush();assert.deepEqual(f.route(),B);assert.equal(f.cursor,2);
f.history.go(1);f.flush();assert.deepEqual(f.route(),C);assert.equal(f.cursor,3);
f.setRoute({...C});assert.equal(f.states.length,4);
f.reject=true;f.history.go(-1);f.flush();assert.equal(f.cursor,3);assert.deepEqual(f.route(),C);
f.reject=false;f.fail=true;f.history.go(-1);f.flush();assert.equal(f.cursor,3);f.fail=false;
f.history.go(-1);f.flush();f.setRoute(A);assert.equal(f.states.length,4);assert.equal(f.cursor,3);
f.history.go(1);f.flush();assert.deepEqual(f.route(),A);
batch(()=>{f.setEpoch(2);f.setRoute(B);});assert.equal(f.cursor,4);
f.history.go(-1);f.flush();assert.equal(f.cursor,4);assert.deepEqual(f.route(),B);
f.history.go(-2);f.flush();assert.equal(f.cursor,4);assert.deepEqual(f.route(),B);
const n=f.restores.length;f.inject({syndocalNavigation:1,session:'old',index:1,route:C});
assert.equal(f.restores.length,n);assert.deepEqual(f.route(),B);
const g=fixture('two');g.inject(f.history.state);assert.equal(g.restores.length,0);
// Browser cursor movement can precede delivery of popstate. A new route
// during that gap must branch at the physical cursor and ignore the old event.
const race=fixture('race');race.setRoute(B);race.setRoute(C);
race.history.go(-1);race.setRoute(A);race.flush();
assert.deepEqual(race.route(),A,'queued Back must not overwrite a newer route');
assert.equal(race.cursor,3);
assert.deepEqual(race.states.map(state=>state.index),[0,1,2,3],'ledger indices retain physical history distance');
assert.equal(race.restores.length,0,'superseded pop must never restore');
race.reject=true;race.history.go(-1);race.flush();
assert.equal(race.cursor,3,'rejected Back repairs exactly one physical entry');
assert.deepEqual(race.route(),A);
race.reject=false;race.history.go(-1);race.flush();assert.deepEqual(race.route(),B);

// Branches reuse positions, not entry identities. An old event for the same
// index must not be mistaken for the replacement at that index.
const oldBranch=structuredClone(race.states[3]);race.setRoute(C);
assert.equal(race.history.state.index,oldBranch.index);
assert.notEqual(race.history.state.token,oldBranch.token);
const beforeStale=race.restores.length;race.inject(oldBranch);
assert.equal(race.restores.length,beforeStale);assert.deepEqual(race.route(),C);
race.dispose();

const epochRace=fixture('epoch-race');epochRace.setRoute(B);epochRace.setRoute(C);
epochRace.history.go(-1);batch(()=>{epochRace.setEpoch(2);epochRace.setRoute(A);});epochRace.flush();
assert.deepEqual(epochRace.route(),A);
assert.deepEqual(epochRace.states.map(state=>state.index),[0,1,2,3]);
epochRace.history.go(-1);epochRace.flush();assert.equal(epochRace.cursor,3);
assert.deepEqual(epochRace.route(),A,'queued traversal cannot cross the new project guard');epochRace.dispose();

// Several traversals may queue before any callback. Only the event for the
// browser's final position may restore or request a bounce.
const queued=fixture('queued');queued.setRoute(B);queued.setRoute(C);
queued.history.go(-1);queued.history.go(-1);queued.flush();
assert.deepEqual(queued.route(),A);assert.equal(queued.cursor,1);
assert.equal(queued.restores.length,1);
queued.history.go(2);queued.flush();assert.deepEqual(queued.route(),C);
queued.reject=true;queued.history.go(-1);queued.history.go(-1);queued.flush();
assert.deepEqual(queued.route(),C);assert.equal(queued.cursor,3);queued.dispose();

g.dispose();assert.equal(g.listener,null);f.dispose();assert.equal(f.listener,null);
console.log('Workspace navigation controller checks passed: back/forward, guard, branching, same-route dedupe, rejection/throw rollback, project fencing, foreign/reload state isolation, pending traversal/route and epoch races, branch token reuse, queued Back cancellation and cleanup.');

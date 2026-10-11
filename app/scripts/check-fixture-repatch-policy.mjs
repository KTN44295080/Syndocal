import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const load=async name=>ts.transpileModule(await readFile(new URL('../src/'+name+'.ts',import.meta.url),'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText;
const url=js=>'data:text/javascript;base64,'+Buffer.from(js).toString('base64');
const dependency=url(await load('dmxAddressing'));
const {fixtureMovePlan}=await import(url((await load('fixtureRepatchPolicy')).replace('"./dmxAddressing"',JSON.stringify(dependency))));
const f=(id,address,footprint=8,universe=1)=>({id,address,universe,label:'Fixture '+id,controls:[{offsets:[footprint]}]});
const fixtures=[f(1,1),f(2,17),f(3,65)];
assert.deepEqual(fixtureMovePlan(fixtures.slice(0,2),fixtures[0],1,17,fixtures).map(c=>c.address),[17,33]);
assert.deepEqual(fixtureMovePlan(fixtures.slice(0,2),fixtures[1],2,100,fixtures).map(c=>c.address),[84,100]);
for(const [selection,anchor,u,address,existing,message] of [
 [fixtures.slice(0,2),fixtures[0],1,49,fixtures,/overlap/],
 [fixtures.slice(0,2),fixtures[0],1,497,fixtures,/outside/],
 [fixtures.slice(0,2),fixtures[1],1,1,fixtures,/outside/],
 [[fixtures[0],f(2,17,8,2)],fixtures[0],1,33,fixtures,/source universe/],
 [[fixtures[0],fixtures[0]],fixtures[0],1,33,fixtures,/distinct/],
 [[],fixtures[0],1,33,fixtures,/distinct/],
 [[fixtures[0]],fixtures[1],1,33,fixtures,/distinct/],
 [[f(1,1,24),fixtures[1]],f(1,1,24),1,100,[],/overlap/],
 [[fixtures[0]],fixtures[0],-1,33,fixtures,/invalid/],
 [[fixtures[0]],fixtures[0],1,3.5,fixtures,/invalid/],
])assert.throws(()=>fixtureMovePlan(selection,anchor,u,address,existing),message);
assert.deepEqual(fixtures.map(f=>f.address),[1,17,65],'invalid plans never mutate source');
console.log('Fixture repatch policy: 12 cases passed');

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const cache = new Map();
async function compile(url) {
  if (cache.has(url.href)) return cache.get(url.href);
  let code = ts.transpileModule(await readFile(url, 'utf8'), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
  for (const match of [...code.matchAll(/from\s+"(\.[^"]+)"/g)]) code = code.replace(match[0], `from "${await compile(new URL(match[1] + '.ts', url))}"`);
  const result = 'data:text/javascript;base64,' + Buffer.from(code).toString('base64'); cache.set(url.href, result); return result;
}
const { mappingFixtureBeamShape: shape, mappingBeamPoints: points } = await import(await compile(new URL('../src/mappingFixtureBeam.ts', import.meta.url)));
const { mappingInstallationRotationToward: aim } = await import(await compile(new URL('../src/mappingFixtureOrientation.ts', import.meta.url)));
const identity = [1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1];
const fixture = { id: 1, manufacturer: '', profile_name: 'Spot', mode_name: '', label: 'Spot', controls: [], geometries: [], rotation: { yaw: 0, pitch: 0, roll: 0 } };
const near = (a,b) => assert(Math.abs(a-b)<1e-8, `${a} != ${b}`);
const extent = polygon => { const rows = polygon.split(' ').map(p => p.split(',').map(Number)); return { minX: Math.min(...rows.map(p=>p[0])), maxX: Math.max(...rows.map(p=>p[0])), minZ: Math.min(...rows.map(p=>p[1])), maxZ: Math.max(...rows.map(p=>p[1])) }; };
let s = shape(fixture,0); assert.equal(s.angle,10); assert.match(s.description,/概略/); near(s.direction.z,1);
let e = extent(points(0,0,s)); near(e.minZ,0); near(e.maxZ,52); near(e.maxX,52*Math.tan(5*Math.PI/180));
const authored = { ...fixture, geometries: [{ name:'Beam', kind:'Beam', matrix:identity, beam_angle_deg:6, field_angle_deg:40 }] };
s = shape(authored,0); assert.equal(s.angle,6); assert.match(s.description,/プロファイル/);
assert.equal(shape({...authored,geometries:[{...authored.geometries[0],beam_angle_deg:NaN}]},0),null);
assert.equal(points(0,0,null),'');
for (const target of [{x:2,y:0,z:5},{x:-3,y:2,z:-4},{x:0,y:-4,z:0}]) {
  const rotation=aim({x:0,y:0,z:0},target), actual=shape({...authored,rotation},rotation.yaw).direction, distance=Math.hypot(target.x,target.y,target.z);
  for(const axis of ['x','y','z'])near(actual[axis],target[axis]/distance);
}
// Head pan followed by installation tilt is not Euler-angle addition.
s=shape({...authored,rotation:{yaw:0,pitch:90,roll:0}},0,32768+65535/6,32768);
near(s.direction.x,1); near(s.direction.y,0); near(s.direction.z,0);
s=shape({...authored,rotation:{yaw:0,pitch:90,roll:0}},0,32768,32768);
e=extent(points(0,0,s)); near(e.maxX,-e.minX); near(e.maxZ,-e.minZ); assert(e.maxZ < 3,'vertical beam projects a footprint, not a 52-unit horizontal ray');
const sideMatrix=[0,0,1,0,0,1,0,0,-1,0,0,0,0,0,0,1];
s=shape({...authored,geometries:[authored.geometries[0],{name:'Side',kind:'Beam',matrix:sideMatrix,beam_angle_deg:12}]},0,undefined,undefined,'Side');
near(s.direction.x,1); near(s.direction.z,0); assert.equal(s.angle,12);
const washed=shape({...fixture,profile_name:'Wash',label:'Wash'},0);assert.equal(washed.angle,30);
console.log('PASS beam optics and projection: authored angle precedence, explicit schematic, invalid rejection, +Z world basis, aim agreement, head/mount composition, vertical footprint and per-geometry direction.');

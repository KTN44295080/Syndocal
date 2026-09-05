import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/liveVideoMonitorPacket.ts", import.meta.url), "utf8");
const code = ts.transpileModule(source, {compilerOptions:{module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2022}}).outputText;
const {parseLiveVideoMonitorPacket:parse}=await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`);
const backend=await readFile(new URL('../src-tauri/src/live_video_monitor_packet.rs',import.meta.url),'utf8');
assert.match(backend,/LIVE_VIDEO_MONITOR_VERSION: u8 = 2/);
assert.match(backend,/LIVE_VIDEO_MONITOR_HEADER_LEN: usize = 40/);
const packet=({encoding=1,status=0,kind=0,width=2,height=1,payload=new Uint8Array([255,0,1,255,0,128,255,0])}={})=>{
  const bytes=new Uint8Array(40+payload.length);bytes.set([83,89,76,86,2,status,kind,encoding]);
  const view=new DataView(bytes.buffer);view.setBigUint64(8,9007199254740993n,true);view.setBigUint64(16,34n,true);
  view.setUint32(24,1200,true);view.setUint32(28,0,true);view.setUint16(32,width,true);view.setUint16(34,height,true);
  view.setUint32(36,payload.length,true);bytes.set(payload,40);return bytes;
};
const rgba=packet(),parsed=parse(rgba);
assert.equal(parsed.encoding,'rgba');assert.deepEqual([...parsed.rgba],[255,0,1,255,0,128,255,0]);
assert.equal(parsed.rgba.buffer,rgba.buffer,'raw payload must share the IPC ArrayBuffer');
assert.equal(parsed.sequence,9007199254740993n);assert.equal(parsed.ptsMs,34n);assert.equal(parsed.renderUs,1200);
const padded=new Uint8Array(rgba.length+31);padded.set(rgba,17);const slice=parse(padded.subarray(17,17+rgba.length));
assert.deepEqual([...slice.rgba],[...parsed.rgba]);assert.equal(slice.rgba.buffer,padded.buffer);assert.equal(slice.rgba.byteOffset,57);
const jpeg=parse(packet({encoding:0,kind:1,payload:new Uint8Array([255,216,255,217])}));
assert.equal(jpeg.encoding,'jpeg');assert.equal(jpeg.kind,'preview');assert.equal(jpeg.jpeg.length,4);
for(const encoding of [0,1])assert.equal(parse(packet({encoding,status:1,payload:new Uint8Array()})).status,'busy');
for(const [offset,value] of [[4,0],[4,1],[4,3],[5,2],[6,2],[7,2],[0,0]]){
  const invalid=rgba.slice();invalid[offset]=value;assert.throws(()=>parse(invalid));
}
for(const invalid of [packet({width:0}),packet({height:0}),packet({payload:new Uint8Array(7)}),packet({status:1}),packet({encoding:0}),rgba.subarray(0,39),rgba.subarray(0,rgba.length-1)])assert.throws(()=>parse(invalid));
assert.throws(()=>parse([256]));assert.throws(()=>parse('not binary'));
console.log('PASS monitor v2 JPEG/RGBA parser, exact bytes/dimensions, zero-copy buffer views, busy packets and retired/malformed packet rejection.');

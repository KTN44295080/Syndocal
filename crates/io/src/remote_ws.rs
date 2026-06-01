use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use protocol::{
    CueId, EngineSnapshot, FixtureId, RemoteControlConfig, VideoLayerId, VideoOutputId, VideoParam,
};
use serde_json::Value;
use thiserror::Error;
use tungstenite::{accept, Message};

const HTTP_PEEK_SIZE: usize = 2048;
const REMOTE_PAGE_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
  <meta name="theme-color" content="#111419">
  <meta name="mobile-web-app-capable" content="yes">
  <meta name="apple-mobile-web-app-capable" content="yes">
  <meta name="apple-mobile-web-app-title" content="KDMX">
  <link rel="manifest" href="/manifest.webmanifest">
  <link rel="icon" href="/icon.svg" type="image/svg+xml">
  <link rel="apple-touch-icon" href="/icon.svg">
  <title>KDMX Remote</title>
  <style>
    :root{color:#edf3fb;background:#111419;font:14px system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}
    *{box-sizing:border-box} body{margin:0;min-height:100vh;background:#111419;color:#edf3fb}
    main{display:grid;gap:12px;padding:14px;max-width:860px;margin:0 auto}
    header{display:flex;align-items:center;justify-content:space-between;border-bottom:1px solid #242c36;padding-bottom:12px}
    h1,h2{margin:0;line-height:1.1} h1{font-size:22px} h2{font-size:15px}
    section{display:grid;gap:10px;border:1px solid #242c36;border-radius:8px;background:#171d25;padding:12px}
    label{display:grid;gap:6px;color:#aab6c6} input,select,button{min-height:40px;border:1px solid #323b48;border-radius:6px;background:#151b22;color:#edf3fb;padding:7px 9px;font:inherit}
    button{background:#202833;border-color:#3d4654} button.primary{background:#1261a6;border-color:#2278c5} button:disabled,select:disabled,input:disabled{opacity:.45}
    .grid{display:grid;grid-template-columns:1fr 1fr;gap:8px}.triple{display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px}
    .status{padding:6px 10px;border-radius:999px;background:#202833;color:#bcc8d7}.ok{color:#8ff0b6;background:#123322}.bad{color:#ffc0c0;background:#3a1517}
    .summary{display:grid;grid-template-columns:repeat(4,1fr);gap:8px}.tile{border:1px solid #242c36;border-radius:7px;background:#121820;padding:9px}.tile strong{display:block;font-size:20px;margin-top:2px}.muted{color:#91a0b2;font-size:12px}
    .liveGrid{display:grid;grid-template-columns:repeat(4,1fr);gap:8px}.liveTile{border:1px solid #242c36;border-radius:7px;background:#121820;padding:9px}.liveTile strong{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;margin-top:2px}
    .row{display:grid;grid-template-columns:1fr auto;align-items:center;gap:8px;border:1px solid #242c36;border-radius:7px;background:#121820;padding:8px}.row.active{border-color:#287fca;background:#132236}
    .videoDeck{display:grid;gap:8px;border:1px solid #242c36;border-radius:7px;background:#121820;padding:9px}.videoDeck.active{border-color:#287fca;background:#132236}.videoDeckHeader{display:flex;justify-content:space-between;gap:8px;align-items:flex-start}.videoDeckHeader strong,.videoDeckHeader span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.videoDeckControls{display:grid;grid-template-columns:repeat(5,1fr);gap:6px}.videoDeckSliders{display:grid;gap:7px}.deckSlider{display:grid;grid-template-columns:70px minmax(0,1fr) 54px;gap:8px;align-items:center;color:#aab6c6}.deckSlider input{width:100%;padding:0}.deckSlider strong{text-align:right;font-size:12px;color:#edf3fb}.loopGrid{display:grid;grid-template-columns:1fr 1fr;gap:7px}.cuePointRow{display:flex;gap:6px;overflow:auto;padding-bottom:2px}.cuePointRow button{white-space:nowrap}
    .row strong,.row span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.list{display:grid;gap:7px;max-height:220px;overflow:auto}.small{font-size:12px;color:#91a0b2}.bar{height:6px;border-radius:99px;background:#2a3340;overflow:hidden}.bar span{display:block;height:100%;background:#3ba1ff}
    .faderBank{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:8px;max-height:300px;overflow:auto}.fixtureFader{border:1px solid #242c36;border-radius:7px;background:#121820;padding:8px}.fixtureFader span{display:flex;justify-content:space-between;gap:8px}.fixtureFader strong,.fixtureFader small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.fixtureFader input{width:100%;padding:0}
    .log{min-height:32px;color:#91a0b2;overflow-wrap:anywhere}.inline{display:flex;gap:8px;align-items:center;flex-wrap:wrap}
    .cuePadHeader{display:grid;grid-template-columns:1fr auto auto auto auto;gap:8px;align-items:center}.compact{display:inline-flex;gap:6px;align-items:center}.compact input{min-height:0;width:auto}.cuePadGrid{display:grid;grid-template-columns:repeat(5,1fr);gap:8px}.cuePad{display:grid;grid-template-columns:24px minmax(0,1fr);grid-template-rows:auto auto;gap:2px 8px;min-height:58px;text-align:left}.cuePad span{grid-row:1 / span 2;display:grid;width:24px;height:24px;place-items:center;border-radius:4px;background:#151b22;color:#91a0b2;font-weight:700}.cuePad strong,.cuePad small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.cuePad.active{border-color:#74d99f;background:#173122}.cuePad.next{border-color:#5f9ddc}
    @media (max-width:640px){main{padding:10px}.grid,.triple,.summary,.liveGrid,.videoDeckControls,.loopGrid{grid-template-columns:1fr}.deckSlider{grid-template-columns:58px minmax(0,1fr) 48px}.cuePadHeader{grid-template-columns:1fr auto}.cuePadGrid{grid-template-columns:repeat(2,1fr)}header{align-items:flex-start;gap:8px;flex-direction:column}}
  </style>
</head>
<body>
<main>
  <header><h1>KDMX Remote</h1><span id="status" class="status bad">Disconnected</span></header>
  <section class="summary">
    <div class="tile"><span class="muted">Fixtures</span><strong id="fixtureCount">0</strong></div>
    <div class="tile"><span class="muted">Cues</span><strong id="cueCount">0</strong></div>
    <div class="tile"><span class="muted">Layers</span><strong id="layerCount">0</strong></div>
    <div class="tile"><span class="muted">BPM</span><strong id="bpmReadout">120.0</strong></div>
  </section>
  <section>
    <h2>Live Desk</h2>
    <div class="liveGrid">
      <div class="liveTile"><span class="muted">Active cue</span><strong id="remoteActiveCue">None</strong></div>
      <div class="liveTile"><span class="muted">Next cue</span><strong id="remoteNextCue">None</strong></div>
      <div class="liveTile"><span class="muted">Timeline</span><strong id="remoteTimelineState">Stopped</strong></div>
      <div class="liveTile"><span class="muted">Guard</span><strong id="remoteGuardState">Live</strong></div>
    </div>
    <div class="triple">
      <button onclick='send({type:"triggerPreviousCue"})'>Back</button>
      <button class="primary" onclick='send({type:"triggerNextCue"})'>GO</button>
      <button onclick='send({type:"tapBpm"})'>Tap</button>
      <button onclick='setTimelinePlaying(true)'>Play TL</button>
      <button onclick='setTimelinePlaying(false)'>Pause TL</button>
      <button onclick='send({type:"blackout",enabled:true})'>DMX BO</button>
      <button onclick='send({type:"blackout",enabled:false})'>DMX Clear</button>
      <button onclick='send({type:"videoBlackout",enabled:true})'>Video BO</button>
      <button onclick='send({type:"videoBlackout",enabled:false})'>Video Clear</button>
    </div>
  </section>
  <section>
    <h2>Fader</h2>
    <div class="grid">
      <label>Fixture<select id="fixtureId" onchange="applyAttributeOptions()"><option value="1">1</option></select></label>
      <label>Attribute<select id="attribute"><option>Dimmer</option></select></label>
    </div>
    <div class="inline"><span class="muted">Value</span><strong id="attributeValueLabel">0</strong></div>
    <input id="attributeValue" type="range" min="0" max="1" step="0.001" value="0" oninput="sendAttribute()">
    <div id="fixtureFaderBank" class="faderBank"></div>
    <div class="grid">
      <button onclick='setSelectedFixtureHighlight(true)'>Highlight</button>
      <button onclick='setSelectedFixtureHighlight(false)'>Clear Highlight</button>
      <button onclick='setSelectedFixtureSolo(true)'>Solo</button>
      <button onclick='setSelectedFixtureSolo(false)'>Clear Solo</button>
      <button onclick='setSelectedFixturePark(true)'>Park</button>
      <button onclick='setSelectedFixturePark(false)'>Clear Park</button>
    </div>
    <div id="fixtureList" class="list"></div>
  </section>
  <section>
    <h2>Cues</h2>
    <div class="triple">
      <button onclick='send({type:"triggerPreviousCue"})'>Back</button>
      <button class="primary" onclick='sendCue()'>GO</button>
      <button onclick='send({type:"triggerNextCue"})'>Next</button>
      <button onclick='send({type:"setCueFadePaused",paused:true})'>Pause</button>
      <button onclick='send({type:"setCueFadePaused",paused:false})'>Resume</button>
    </div>
    <div class="cuePadHeader">
      <h3>Cue Pads</h3>
      <span id="cuePadRange" class="small">0 / 0</span>
      <label class="compact small"><input id="cuePadFollow" type="checkbox" checked onchange="setCuePadFollow(this.checked)">Follow</label>
      <button id="cuePadPrev" onclick="moveCuePadBank(-1)">Prev</button>
      <button id="cuePadNext" onclick="moveCuePadBank(1)">Next</button>
    </div>
    <div id="cuePad" class="cuePadGrid"></div>
    <label>Cue<select id="cueId"><option value="1">1</option></select></label>
    <div id="cueButtons" class="list"></div>
  </section>
  <section>
    <h2>Clock</h2>
    <div class="grid">
      <label>BPM<input id="bpm" type="number" min="20" max="300" step="0.1" value="120"></label>
      <button class="primary" onclick='send({type:"setBpm",bpm:numberValue("bpm")})'>Set BPM</button>
    </div>
    <button onclick='send({type:"tapBpm"})'>Tap</button>
    <div id="clockInfo" class="small"></div>
  </section>
  <section>
    <h2>Video</h2>
    <div class="triple">
      <label>Layer<select id="layerId"><option value="1">1</option></select></label>
      <label>Param<select id="videoParam"><option>opacity</option><option>speed</option><option>position</option><option>bpm_sync</option><option>bpm_sync_ratio</option><option>bpm_sync_loop_bars</option><option>transform_x</option><option>transform_y</option><option>scale_x</option><option>scale_y</option><option>rotation</option><option>crop_left</option><option>crop_top</option><option>crop_right</option><option>crop_bottom</option><option>brightness</option><option>contrast</option><option>hue</option><option>saturation</option><option>gamma</option><option>pixelate</option><option>blur</option><option>glow</option><option>edge</option><option>key_red</option><option>key_green</option><option>key_blue</option><option>key_threshold</option></select></label>
      <label>Value<input id="videoValue" type="number" step="0.01" value="1"></label>
    </div>
    <button class="primary" onclick='sendVideoParam()'>Send Video Param</button>
    <div class="triple">
      <button class="primary" onclick='setVideoPlaying(true)'>Play</button>
      <button onclick='setVideoPlaying(false)'>Pause</button>
      <button onclick='seekSelectedLayer(0)'>Restart</button>
    </div>
    <div class="grid">
      <button onclick='setSelectedLayerEnabled(true)'>Layer On</button>
      <button onclick='setSelectedLayerEnabled(false)'>Layer Off</button>
      <button onclick='setSelectedLayerSolo(true)'>Solo Layer</button>
      <button onclick='setSelectedLayerSolo(false)'>Clear Solo</button>
    </div>
    <div class="grid">
      <label>Seek ms<input id="videoSeekMs" type="number" min="0" step="1" value="0"></label>
      <button onclick='seekSelectedLayer(numberValue("videoSeekMs"))'>Seek</button>
    </div>
    <div class="grid">
      <button onclick='addSelectedLayerCuePoint()'>Add Cue Point</button>
      <button onclick='jumpSelectedLayerCuePoint(0)'>Jump First Cue</button>
    </div>
    <div class="grid">
      <button onclick='send({type:"videoBlackout",enabled:true})'>Video Blackout</button>
      <button onclick='send({type:"videoBlackout",enabled:false})'>Video Clear</button>
    </div>
    <div id="layerList" class="list"></div>
  </section>
  <section>
    <h2>Video Outputs</h2>
    <label>Fade ms<input id="videoOutputFadeMs" type="number" min="0" step="10" value="1000"></label>
    <div id="videoOutputList" class="list"></div>
  </section>
  <section>
    <h2>Timeline</h2>
    <div id="timelineInfo" class="small"></div>
    <div class="triple">
      <button class="primary" onclick='setTimelinePlaying(true)'>Play</button>
      <button onclick='setTimelinePlaying(false)'>Pause</button>
      <button onclick='seekTimelineRemote(0)'>Restart</button>
    </div>
    <div class="grid">
      <label>Seek ms<input id="timelineSeekMs" type="number" min="0" step="1" value="0"></label>
      <button onclick='seekTimelineRemote(numberValue("timelineSeekMs"))'>Seek</button>
    </div>
    <div id="timelineEvents" class="list"></div>
  </section>
  <section>
    <h2>Master</h2>
    <div class="grid">
      <button onclick='send({type:"blackout",enabled:true})'>Blackout</button>
      <button onclick='send({type:"blackout",enabled:false})'>Clear</button>
    </div>
    <label>Lighting Master<input id="lightingMaster" type="range" min="0" max="1" step="0.01" value="1" oninput='send({type:"lightingMaster",master:numberValue("lightingMaster")})'></label>
    <h3>Submasters</h3>
    <div id="submasterList" class="list"></div>
    <label>Video Master<input id="videoMaster" type="range" min="0" max="1" step="0.01" value="1" oninput='send({type:"videoMaster",opacity:numberValue("videoMaster")})'></label>
    <div id="masterInfo" class="small"></div>
  </section>
  <div id="log" class="log"></div>
</main>
<script>
let ws;
let latestSnapshot=null;
let snapshotTimer=null;
let snapshotPending=false;
let cuePadBank=0;
let cuePadFollowActive=true;
const cuePadSize=10;
const statusEl=document.getElementById("status");
const logEl=document.getElementById("log");
function numberValue(id){return Number(document.getElementById(id).value)}
function selectedNumber(id){const raw=document.getElementById(id).value;if(raw==="")return null;const value=Number(raw);return Number.isFinite(value)?value:null}
function text(id,value){document.getElementById(id).textContent=value}
function escapeHtml(value){return String(value).replace(/[&<>"']/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#39;"}[c]))}
function setStatus(ok){statusEl.textContent=ok?"Connected":"Disconnected";statusEl.className=ok?"status ok":"status bad"}
function connect(){
  if(snapshotTimer)clearInterval(snapshotTimer);
  ws=new WebSocket(`ws://${location.host}/ws`);
  ws.onopen=()=>{setStatus(true);requestSnapshot();snapshotTimer=setInterval(requestSnapshot,1000)};
  ws.onclose=()=>{setStatus(false);if(snapshotTimer)clearInterval(snapshotTimer);setTimeout(connect,1000)};
  ws.onerror=()=>setStatus(false);
  ws.onmessage=e=>handleMessage(e.data);
}
function send(payload,refresh=true){
  if(!ws||ws.readyState!==WebSocket.OPEN){logEl.textContent="Remote socket is not connected";return}
  ws.send(JSON.stringify(payload));
  if(refresh)queueSnapshot(90);
}
function queueSnapshot(delay){
  if(snapshotPending)return;
  snapshotPending=true;
  setTimeout(()=>{snapshotPending=false;requestSnapshot()},delay);
}
function requestSnapshot(){send({type:"getSnapshot"},false)}
function handleMessage(messageText){
  logEl.textContent=messageText;
  try{
    const msg=JSON.parse(messageText);
    if(msg.type==="snapshot")applySnapshot(msg.snapshot);
    else if(msg.ok)queueSnapshot(120);
  }catch(_e){}
}
function fillSelect(id,items,labelFn,emptyLabel){
  const el=document.getElementById(id);const current=el.value;el.innerHTML="";
  if(!items.length){const opt=document.createElement("option");opt.value="";opt.textContent=emptyLabel;el.appendChild(opt);el.disabled=true;return}
  el.disabled=false;
  for(const item of items){const opt=document.createElement("option");opt.value=String(item.id);opt.textContent=labelFn(item);el.appendChild(opt)}
  if([...el.options].some(o=>o.value===current))el.value=current
}
function applySnapshot(snapshot){
  latestSnapshot=snapshot;
  const fixtures=snapshot.fixtures||[];
  const cues=snapshot.cues||[];
  const video=snapshot.video||{layers:[],master_opacity:1,blackout:false};
  const layers=video.layers||[];
  fillSelect("fixtureId",fixtures,f=>`${f.id}: ${f.label}`,"No fixtures");
  fillSelect("cueId",cues,c=>`${c.id}: ${c.label}`,"No cues");
  fillSelect("layerId",layers,l=>`${l.id}: ${l.label}`,"No layers");
  applyAttributeOptions();
  text("fixtureCount",fixtures.length);
  text("cueCount",cues.length);
  text("layerCount",layers.length);
  text("bpmReadout",((snapshot.clock&&snapshot.clock.bpm)||120).toFixed(1));
  document.getElementById("bpm").value=((snapshot.clock&&snapshot.clock.bpm)||120).toFixed(1);
  document.getElementById("lightingMaster").value=snapshot.lighting_master??1;
  document.getElementById("videoMaster").value=video.master_opacity??1;
  renderFixtureList(fixtures);
  renderCuePads(cues,snapshot.active_cue_id);
  renderCueButtons(cues,snapshot.active_cue_id);
  renderLayerList(layers);
  renderVideoOutputList(video.outputs||[],video.compositions||[]);
  renderSubmasters(snapshot.submasters||[]);
  renderTimeline(snapshot.timeline||{events:[],position_ms:0,duration_ms:0,playing:false});
  renderStatus(snapshot,video);
  renderLiveDesk(snapshot,cues,video,snapshot.timeline||{events:[],position_ms:0,duration_ms:0,playing:false});
}
function applyAttributeOptions(){
  const fixtureId=selectedNumber("fixtureId");
  const fixture=(latestSnapshot&&latestSnapshot.fixtures||[]).find(f=>f.id===fixtureId);
  const attrs=fixture?(fixture.controls||[]).map(c=>({id:c.attribute,label:c.attribute})):[];
  fillSelect("attribute",attrs,a=>a.label,"No attributes");
  syncSelectedAttributeValue();
  renderFixtureFaderBank(fixture);
}
function normalizedAttributeValue(fixture,attribute){
  const value=(fixture&&fixture.attribute_values||[]).find(v=>v.attribute===attribute);
  return value?Math.max(0,Math.min(1,value.value/65535)):0;
}
function syncSelectedAttributeValue(){
  const fixtureId=selectedNumber("fixtureId");
  const attr=document.getElementById("attribute").value;
  const fixture=(latestSnapshot&&latestSnapshot.fixtures||[]).find(f=>f.id===fixtureId);
  const value=(fixture&&fixture.attribute_values||[]).find(v=>v.attribute===attr);
  const normalized=normalizedAttributeValue(fixture,attr);
  document.getElementById("attributeValue").value=normalized;
  text("attributeValueLabel",value?String(value.value):"0");
}
function renderFixtureFaderBank(fixture){
  const el=document.getElementById("fixtureFaderBank");
  if(!fixture){el.innerHTML=`<span class="small">No fixture selected</span>`;return}
  const controls=fixture.controls||[];
  el.innerHTML=controls.length?controls.map(control=>{
    const value=normalizedAttributeValue(fixture,control.attribute);
    const dmxValue=Math.round(value*65535);
    return `<label class="fixtureFader"><span><strong>${escapeHtml(control.attribute)}</strong><small>${dmxValue}</small></span><input type="range" min="0" max="1" step="0.001" value="${value.toFixed(4)}" data-fixture="${fixture.id}" data-attribute="${escapeHtml(control.attribute)}" oninput="setFixtureAttributeFromInput(this)"></label>`
  }).join(""):`<span class="small">No attributes</span>`;
}
function renderFixtureList(fixtures){
  document.getElementById("fixtureList").innerHTML=fixtures.map(f=>`<div class="row ${f.soloed||f.highlighted||f.parked?"active":""}"><div><strong>${escapeHtml(f.label)}</strong><span class="small">U${f.universe} / ${f.address} / ${escapeHtml(f.profile_name)}${f.highlighted?" / Highlight":""}${f.soloed?" / Solo":""}${f.parked?" / Park":""}</span></div><button onclick="selectFixture(${f.id})">Select</button></div>`).join("");
}
function renderCueButtons(cues,activeCueId){
  document.getElementById("cueButtons").innerHTML=cues.map(c=>`<div class="row ${c.id===activeCueId?"active":""}"><div><strong>${escapeHtml(c.label)}</strong><span class="small">${c.targets.length} light / ${c.video_targets.length} video / ${c.fade_ms}ms</span></div><button class="primary" onclick="triggerCue(${c.id})">GO</button></div>`).join("");
}
function renderCuePads(cues,activeCueId){
  const bankCount=Math.max(1,Math.ceil(cues.length/cuePadSize));
  if(cuePadBank>bankCount-1)cuePadBank=bankCount-1;
  const activeIndex=cues.findIndex(c=>c.id===activeCueId);
  if(cuePadFollowActive&&activeIndex>=0)cuePadBank=Math.floor(activeIndex/cuePadSize);
  const start=cuePadBank*cuePadSize;
  const end=Math.min(start+cuePadSize,cues.length);
  const nextCueId=cues.length?cues[((activeIndex<0?-1:activeIndex)+1+cues.length)%cues.length].id:null;
  document.getElementById("cuePadRange").textContent=cues.length?`${start+1}-${end} / ${cues.length}`:"0 / 0";
  document.getElementById("cuePadFollow").checked=cuePadFollowActive;
  document.getElementById("cuePadPrev").disabled=cuePadBank===0;
  document.getElementById("cuePadNext").disabled=cuePadBank>=bankCount-1;
  document.getElementById("cuePad").innerHTML=Array.from({length:cuePadSize},(_,index)=>{
    const cue=cues[start+index];
    const slot=index===9?"0":String(index+1);
    if(!cue)return `<button class="cuePad" disabled><span>${slot}</span><strong>Empty</strong><small>-</small></button>`;
    const className=`cuePad ${cue.id===activeCueId?"active":""} ${cue.id===nextCueId?"next":""}`;
    return `<button class="${className}" onclick="triggerCue(${cue.id})"><span>${slot}</span><strong>${escapeHtml(cue.label)}</strong><small>#${start+index+1} / ${cue.fade_ms}ms</small></button>`
  }).join("");
}
function renderLayerList(layers){
  const el=document.getElementById("layerList");
  if(!layers.length){el.innerHTML=`<span class="small">No video layers</span>`;return}
  el.innerHTML=layers.map(l=>{
    const state=l.state||{};
    const metadata=(l.source&&l.source.metadata)||{};
    const duration=Number(metadata.duration_ms)||0;
    const sliderMax=Math.max(duration,state.position_ms||0,state.loop_end_ms||0,1);
    const position=Math.max(0,Math.min(sliderMax,state.position_ms||0));
    const pct=Math.round(Math.max(0,Math.min(1,state.opacity??1))*100);
    const seekPct=Math.round((position/sliderMax)*100);
    const enabled=state.enabled!==false;
    const solo=state.solo===true;
    const loop=state.loop_enabled===true;
    const speed=Number(state.speed??1);
    const loopStartMax=Math.max(0,sliderMax-1);
    const loopStart=Math.max(0,Math.min(loopStartMax,state.loop_start_ms||0));
    const loopEnd=Math.max(loopStart+1,Math.min(sliderMax,state.loop_end_ms||sliderMax));
    const cueButtons=(state.cue_points_ms||[]).map((ms,index)=>`<button onclick="jumpLayerCuePoint(${l.id},${index})">${formatRemoteTime(ms)}</button>`).join("");
    return `<div class="videoDeck ${enabled&&solo?"active":""}">
      <div class="videoDeckHeader"><div><strong>${escapeHtml(l.label)}</strong><span class="small">${escapeHtml(l.source.kind)} / ${escapeHtml(l.blend_mode)} / ${enabled?"On":"Off"}${solo?" / Solo":""} / ${state.playing?"Playing":"Paused"}</span></div><span class="small">${formatRemoteTime(position)}${duration?` / ${formatRemoteTime(duration)}`:""}</span></div>
      <div class="bar"><span style="width:${seekPct}%"></span></div>
      <div class="videoDeckControls">
        <button onclick="selectLayer(${l.id})">Select</button>
        <button onclick="setLayerPlaying(${l.id},${!state.playing})">${state.playing?"Pause":"Play"}</button>
        <button onclick="setLayerEnabled(${l.id},${!enabled})">${enabled?"Off":"On"}</button>
        <button onclick="setLayerSolo(${l.id},${!solo})">${solo?"Unsolo":"Solo"}</button>
        <button onclick="setLayerLoopFromInputs(${l.id},${!loop})">${loop?"Loop Off":"Loop On"}</button>
      </div>
      <div class="videoDeckSliders">
        <label class="deckSlider"><span>Opacity</span><input type="range" min="0" max="1" step="0.01" value="${Number(state.opacity??1)}" oninput="setLayerParamFromInput(${l.id},'opacity',this.value)"><strong>${pct}%</strong></label>
        <label class="deckSlider"><span>Speed</span><input type="range" min="-4" max="4" step="0.01" value="${speed}" oninput="setLayerParamFromInput(${l.id},'speed',this.value)"><strong>${speed.toFixed(2)}x</strong></label>
        <label class="deckSlider"><span>Seek</span><input type="range" min="0" max="${sliderMax}" step="1" value="${position}" oninput="seekLayerFromInput(${l.id},this.value)"><strong>${formatRemoteTime(position)}</strong></label>
      </div>
      <div class="loopGrid">
        <label class="deckSlider"><span>Loop In</span><input id="loopStart-${l.id}" type="range" min="0" max="${loopStartMax}" step="1" value="${loopStart}" oninput="setLayerLoopBoundsFromInput(${l.id})"><strong>${formatRemoteTime(loopStart)}</strong></label>
        <label class="deckSlider"><span>Loop Out</span><input id="loopEnd-${l.id}" type="range" min="1" max="${sliderMax}" step="1" value="${loopEnd}" oninput="setLayerLoopBoundsFromInput(${l.id})"><strong>${formatRemoteTime(loopEnd)}</strong></label>
      </div>
      <div class="inline"><button onclick="setLayerSpeedPreset(${l.id},-1)">Reverse</button><button onclick="setLayerSpeedPreset(${l.id},0.5)">0.5x</button><button onclick="setLayerSpeedPreset(${l.id},1)">1x</button><button onclick="setLayerSpeedPreset(${l.id},2)">2x</button><button onclick="seekLayerFromInput(${l.id},0)">Restart</button><button onclick="addLayerCuePoint(${l.id})">Add Cue</button></div>
      <div class="cuePointRow">${cueButtons||`<span class="small">No cue points</span>`}</div>
    </div>`
  }).join("");
}
function renderVideoOutputList(outputs,compositions){
  const el=document.getElementById("videoOutputList");
  if(!outputs.length){el.innerHTML=`<span class="small">No video outputs</span>`;return}
  el.innerHTML=outputs.map(o=>{
    const pct=Math.round((o.opacity??1)*100);
    const comp=compositions.find(c=>c.id===o.composition_id);
    const active=o.enabled&&!o.blackout&&(o.opacity??1)>0;
    return `<div class="row ${active?"active":""}"><div><strong>${escapeHtml(o.label)}</strong><span class="small">${escapeHtml(o.kind)} / ${escapeHtml(comp?comp.label:`Composition ${o.composition_id}`)} / ${pct}%${o.enabled?"":" / Disabled"}${o.blackout?" / Blackout":""}</span><div class="bar"><span style="width:${Math.max(0,Math.min(100,pct))}%"></span></div></div><div class="inline"><button onclick="setVideoOutputEnabled(${o.id},${!o.enabled})">${o.enabled?"Disable":"Enable"}</button><button onclick="setVideoOutputBlackout(${o.id},${!o.blackout})">${o.blackout?"Clear":"Blackout"}</button><button onclick="fadeVideoOutput(${o.id},0)">Fade Out</button><button onclick="fadeVideoOutput(${o.id},1)">Fade In</button><button onclick="setVideoOutputOpacity(${o.id},1)">Full</button></div></div>`
  }).join("");
}
function renderSubmasters(submasters){
  const el=document.getElementById("submasterList");
  el.innerHTML=submasters.length?submasters.map(s=>`<div class="row"><div><strong>${escapeHtml(s.label)}</strong><label><span class="small">Submaster</span><input type="range" min="0" max="1" step="0.01" value="${Number(s.level??1)}" data-group="${escapeHtml(s.group_id)}" oninput="setGroupSubmasterFromInput(this)"></label></div><div class="inline"><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupHighlightFromButton(this,true)">Hi</button><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupHighlightFromButton(this,false)">-Hi</button><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupSoloFromButton(this,true)">Solo</button><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupSoloFromButton(this,false)">-Solo</button><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupParkFromButton(this,true)">Park</button><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupParkFromButton(this,false)">-Park</button></div></div>`).join(""):`<span class="small">No fixture groups</span>`;
}
function renderTimeline(timeline){
  const duration=timeline.duration_ms||0;
  const position=timeline.position_ms||0;
  document.getElementById("timelineSeekMs").value=position;
  text("timelineInfo",`${timeline.playing?"Playing":"Stopped"} / ${position}ms / ${duration}ms`);
  document.getElementById("timelineEvents").innerHTML=(timeline.events||[]).map(e=>`<div class="row"><div><strong>${e.time_ms}ms</strong><span class="small">${escapeHtml(e.track)} cue ${e.cue_id}</span></div><button onclick="send({type:'triggerCue',cue_id:${e.cue_id}})">GO</button></div>`).join("");
}
function renderLiveDesk(snapshot,cues,video,timeline){
  const activeIndex=cues.findIndex(c=>c.id===snapshot.active_cue_id);
  const activeCue=activeIndex>=0?cues[activeIndex]:null;
  const nextCue=cues.length?cues[((activeIndex<0?-1:activeIndex)+1+cues.length)%cues.length]:null;
  text("remoteActiveCue",activeCue?activeCue.label:"None");
  text("remoteNextCue",nextCue?nextCue.label:"None");
  text("remoteTimelineState",`${timeline.playing?"Playing":"Stopped"} / ${timeline.position_ms||0}ms`);
  text("remoteGuardState",`${snapshot.blackout?"DMX BO":"DMX Live"} / ${video.blackout?"Video BO":"Video Live"}`);
}
function renderStatus(snapshot,video){
  const clock=snapshot.clock||{};
  text("clockInfo",`${escapeHtml(clock.source||"Manual")} / beat ${clock.beat_counter||0} / phase ${Number(clock.beat_phase||0).toFixed(3)}`);
  text("masterInfo",`${snapshot.blackout?"Lighting blackout":"Lighting live"} / lighting master ${Math.round((snapshot.lighting_master??1)*100)}% / ${video.blackout?"Video blackout":"Video live"} / video master ${Math.round((video.master_opacity??1)*100)}%`);
}
function formatRemoteTime(ms){
  const value=Math.max(0,Math.round(Number(ms)||0));
  const minutes=Math.floor(value/60000);
  const seconds=Math.floor((value%60000)/1000);
  const millis=value%1000;
  return `${minutes}:${String(seconds).padStart(2,"0")}.${String(millis).padStart(3,"0")}`;
}
function selectFixture(id){document.getElementById("fixtureId").value=String(id);applyAttributeOptions()}
function setSelectedFixtureHighlight(enabled){const fixture_id=selectedNumber("fixtureId");if(fixture_id!==null)send({type:"setFixtureHighlight",fixture_id,enabled})}
function setSelectedFixtureSolo(enabled){const fixture_id=selectedNumber("fixtureId");if(fixture_id!==null)send({type:"setFixtureSolo",fixture_id,enabled})}
function setSelectedFixturePark(enabled){const fixture_id=selectedNumber("fixtureId");if(fixture_id!==null)send({type:"setFixturePark",fixture_id,enabled})}
function setGroupSubmasterFromInput(el){send({type:"setGroupSubmaster",group_id:el.dataset.group,level:Number(el.value)})}
function setGroupHighlightFromButton(el,enabled){send({type:"setGroupHighlight",group_id:el.dataset.group,enabled})}
function setGroupSoloFromButton(el,enabled){send({type:"setGroupSolo",group_id:el.dataset.group,enabled})}
function setGroupParkFromButton(el,enabled){send({type:"setGroupPark",group_id:el.dataset.group,enabled})}
function selectLayer(id){document.getElementById("layerId").value=String(id)}
function triggerCue(id){document.getElementById("cueId").value=String(id);send({type:"triggerCue",cue_id:id})}
function moveCuePadBank(delta){
  if(!latestSnapshot)return;
  cuePadFollowActive=false;
  const cues=latestSnapshot.cues||[];
  const bankCount=Math.max(1,Math.ceil(cues.length/cuePadSize));
  cuePadBank=Math.max(0,Math.min(bankCount-1,cuePadBank+delta));
  renderCuePads(cues,latestSnapshot.active_cue_id);
}
function setCuePadFollow(enabled){
  cuePadFollowActive=enabled;
  if(latestSnapshot)renderCuePads(latestSnapshot.cues||[],latestSnapshot.active_cue_id);
}
function sendAttribute(){
  const fixture_id=selectedNumber("fixtureId");
  const attribute=document.getElementById("attribute").value;
  if(fixture_id===null||!attribute)return;
  const value=numberValue("attributeValue");
  text("attributeValueLabel",String(Math.round(value*65535)));
  send({type:"setAttribute",fixture_id,attribute,value},false);
  queueSnapshot(160);
}
function setFixtureAttributeFromInput(el){
  const fixture_id=Number(el.dataset.fixture);
  const attribute=el.dataset.attribute;
  const value=Number(el.value);
  if(!Number.isFinite(fixture_id)||!attribute||!Number.isFinite(value))return;
  const dmxValue=String(Math.round(value*65535));
  const valueLabel=el.closest(".fixtureFader").querySelector("small");
  if(valueLabel)valueLabel.textContent=dmxValue;
  if(selectedNumber("fixtureId")===fixture_id&&document.getElementById("attribute").value===attribute){
    document.getElementById("attributeValue").value=value;
    text("attributeValueLabel",dmxValue);
  }
  send({type:"setAttribute",fixture_id,attribute,value},false);
  queueSnapshot(160);
}
function sendCue(){const cue_id=selectedNumber("cueId");if(cue_id!==null)send({type:"triggerCue",cue_id})}
function sendVideoParam(){const layer_id=selectedNumber("layerId");if(layer_id!==null)send({type:"setVideoParam",layer_id,param:document.getElementById("videoParam").value,value:numberValue("videoValue")})}
function setLayerParamFromInput(layer_id,param,value){
  const numeric=Number(value);
  if(!Number.isFinite(numeric))return;
  send({type:"setVideoParam",layer_id,param,value:numeric},false);
  queueSnapshot(160);
}
function setLayerSpeedPreset(layer_id,speed){setLayerParamFromInput(layer_id,"speed",speed)}
function setLayerEnabled(layer_id,enabled){send({type:"setVideoLayerEnabled",layer_id,enabled})}
function setSelectedLayerEnabled(enabled){const layer_id=selectedNumber("layerId");if(layer_id!==null)setLayerEnabled(layer_id,enabled)}
function setLayerSolo(layer_id,solo){send({type:"setVideoLayerSolo",layer_id,solo})}
function setSelectedLayerSolo(solo){const layer_id=selectedNumber("layerId");if(layer_id!==null)setLayerSolo(layer_id,solo)}
function setLayerPlaying(layer_id,playing){send({type:"setVideoPlaying",layer_id,playing})}
function setVideoPlaying(playing){const layer_id=selectedNumber("layerId");if(layer_id!==null)setLayerPlaying(layer_id,playing)}
function setVideoOutputEnabled(output_id,enabled){send({type:"setVideoOutputEnabled",output_id,enabled})}
function setVideoOutputBlackout(output_id,blackout){send({type:"setVideoOutputBlackout",output_id,blackout})}
function setVideoOutputOpacity(output_id,opacity){send({type:"setVideoOutputOpacity",output_id,opacity})}
function fadeVideoOutput(output_id,opacity){send({type:"fadeVideoOutputOpacity",output_id,opacity,duration_ms:Math.max(0,Math.round(numberValue("videoOutputFadeMs")))})}
function seekSelectedLayer(position_ms){const layer_id=selectedNumber("layerId");if(layer_id!==null&&Number.isFinite(position_ms))send({type:"seekVideoLayer",layer_id,position_ms:Math.max(0,Math.round(position_ms))})}
function seekLayerFromInput(layer_id,value){
  const position_ms=Math.max(0,Math.round(Number(value)));
  if(Number.isFinite(position_ms))send({type:"seekVideoLayer",layer_id,position_ms},false);
  queueSnapshot(160);
}
function setLayerLoopFromInputs(layer_id,enabled){
  const startEl=document.getElementById(`loopStart-${layer_id}`);
  const endEl=document.getElementById(`loopEnd-${layer_id}`);
  const loop_start_ms=startEl?Math.max(0,Math.round(Number(startEl.value))):0;
  const loop_end_ms=endEl?Math.max(loop_start_ms+1,Math.round(Number(endEl.value))):loop_start_ms+1;
  send({type:"setVideoLoop",layer_id,enabled,loop_start_ms,loop_end_ms});
}
function setLayerLoopBoundsFromInput(layer_id){
  const startEl=document.getElementById(`loopStart-${layer_id}`);
  const endEl=document.getElementById(`loopEnd-${layer_id}`);
  if(!startEl||!endEl)return;
  const loop_start_ms=Math.max(0,Math.round(Number(startEl.value)));
  const loop_end_ms=Math.max(loop_start_ms+1,Math.round(Number(endEl.value)));
  send({type:"setVideoLoop",layer_id,enabled:true,loop_start_ms,loop_end_ms},false);
  queueSnapshot(160);
}
function addLayerCuePoint(layer_id){send({type:"addVideoCuePoint",layer_id})}
function addSelectedLayerCuePoint(){const layer_id=selectedNumber("layerId");if(layer_id!==null)addLayerCuePoint(layer_id)}
function jumpLayerCuePoint(layer_id,cue_point_index){send({type:"jumpVideoCuePoint",layer_id,cue_point_index})}
function jumpSelectedLayerCuePoint(cue_point_index){const layer_id=selectedNumber("layerId");if(layer_id!==null)jumpLayerCuePoint(layer_id,cue_point_index)}
function setTimelinePlaying(playing){send({type:"setTimelinePlaying",playing})}
function seekTimelineRemote(position_ms){if(Number.isFinite(position_ms))send({type:"seekTimeline",position_ms:Math.max(0,Math.round(position_ms))})}
if("serviceWorker" in navigator&&window.isSecureContext){navigator.serviceWorker.register("/remote-sw.js").catch(()=>{})}
connect();
</script>
</body>
</html>
"##;
const REMOTE_MANIFEST: &str = r##"{"name":"KDMX Remote","short_name":"KDMX","start_url":"/","scope":"/","display":"standalone","background_color":"#111419","theme_color":"#111419","icons":[{"src":"/icon.svg","sizes":"any","type":"image/svg+xml","purpose":"any maskable"}]}"##;
const REMOTE_ICON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128"><rect width="128" height="128" rx="24" fill="#111419"/><path d="M28 88h72" stroke="#4aa8ff" stroke-width="10" stroke-linecap="round"/><path d="M36 34v42M64 24v52M92 44v32" stroke="#edf3fb" stroke-width="10" stroke-linecap="round"/><circle cx="36" cy="58" r="12" fill="#f2c14e"/><circle cx="64" cy="42" r="12" fill="#74d99f"/><circle cx="92" cy="66" r="12" fill="#4aa8ff"/></svg>"##;
const REMOTE_SERVICE_WORKER_JS: &str = r##"const CACHE_NAME="kdmx-remote-v1";const SHELL=["/","/manifest.webmanifest","/icon.svg"];self.addEventListener("install",event=>{event.waitUntil(caches.open(CACHE_NAME).then(cache=>cache.addAll(SHELL)).then(()=>self.skipWaiting()))});self.addEventListener("activate",event=>{event.waitUntil(caches.keys().then(keys=>Promise.all(keys.filter(key=>key!==CACHE_NAME).map(key=>caches.delete(key)))).then(()=>self.clients.claim()))});self.addEventListener("fetch",event=>{if(event.request.method!=="GET")return;event.respondWith(fetch(event.request).then(response=>{const copy=response.clone();if(event.request.url.startsWith(self.location.origin)){caches.open(CACHE_NAME).then(cache=>cache.put(event.request,copy))}return response}).catch(()=>caches.match(event.request).then(cached=>cached||caches.match("/"))))});"##;

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteInputEvent {
    SetAttribute {
        fixture_id: FixtureId,
        attribute: String,
        value: u16,
    },
    SetFixtureHighlight {
        fixture_id: FixtureId,
        enabled: bool,
    },
    SetFixtureSolo {
        fixture_id: FixtureId,
        enabled: bool,
    },
    SetFixturePark {
        fixture_id: FixtureId,
        enabled: bool,
    },
    SetGroupHighlight {
        group_id: String,
        enabled: bool,
    },
    SetGroupSolo {
        group_id: String,
        enabled: bool,
    },
    SetGroupPark {
        group_id: String,
        enabled: bool,
    },
    Blackout(bool),
    SetBpm(f32),
    TapBpm,
    TriggerCue(CueId),
    TriggerNextCue,
    TriggerPreviousCue,
    SetCueFadePaused(bool),
    SetTimelinePlaying(bool),
    SeekTimeline {
        position_ms: u64,
    },
    SetVideoParam {
        layer_id: VideoLayerId,
        param: VideoParam,
        value: f32,
    },
    SetVideoLayerEnabled {
        layer_id: VideoLayerId,
        enabled: bool,
    },
    SetVideoLayerSolo {
        layer_id: VideoLayerId,
        solo: bool,
    },
    SetVideoPlaying {
        layer_id: VideoLayerId,
        playing: bool,
    },
    SeekVideoLayer {
        layer_id: VideoLayerId,
        position_ms: u64,
    },
    SetVideoLoop {
        layer_id: VideoLayerId,
        enabled: bool,
        loop_start_ms: Option<u64>,
        loop_end_ms: Option<u64>,
    },
    AddVideoCuePoint {
        layer_id: VideoLayerId,
        position_ms: Option<u64>,
    },
    RemoveVideoCuePoint {
        layer_id: VideoLayerId,
        position_ms: u64,
    },
    JumpVideoCuePoint {
        layer_id: VideoLayerId,
        cue_point_index: usize,
    },
    SetVideoOutputEnabled {
        output_id: VideoOutputId,
        enabled: bool,
    },
    SetVideoOutputOpacity {
        output_id: VideoOutputId,
        opacity: f32,
    },
    FadeVideoOutputOpacity {
        output_id: VideoOutputId,
        opacity: f32,
        duration_ms: u64,
    },
    SetVideoOutputBlackout {
        output_id: VideoOutputId,
        blackout: bool,
    },
    VideoMasterOpacity(f32),
    VideoBlackout(bool),
    LightingMaster(f32),
    SetGroupSubmaster {
        group_id: String,
        level: f32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteClientRequest {
    Event(RemoteInputEvent),
    GetSnapshot,
}

#[derive(Debug, Error)]
pub enum RemoteWsError {
    #[error("remote bind address is required")]
    MissingBindAddress,
    #[error("remote port must be greater than 0")]
    InvalidPort,
    #[error("failed to bind remote WebSocket server {bind}: {source}")]
    Bind {
        bind: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteParseError {
    InvalidJson(String),
    MissingType,
    UnknownType(String),
    MissingField(&'static str),
    InvalidField(&'static str),
}

pub struct RemoteWsServer {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl RemoteWsServer {
    pub fn start<F>(config: RemoteControlConfig, callback: F) -> Result<Self, RemoteWsError>
    where
        F: Fn(RemoteInputEvent) + Send + Sync + 'static,
    {
        Self::start_with_snapshot(config, callback, EngineSnapshot::default)
    }

    pub fn start_with_snapshot<F, S>(
        config: RemoteControlConfig,
        callback: F,
        snapshot_provider: S,
    ) -> Result<Self, RemoteWsError>
    where
        F: Fn(RemoteInputEvent) + Send + Sync + 'static,
        S: Fn() -> EngineSnapshot + Send + Sync + 'static,
    {
        if config.bind_ip.trim().is_empty() {
            return Err(RemoteWsError::MissingBindAddress);
        }
        if config.port == 0 {
            return Err(RemoteWsError::InvalidPort);
        }

        let bind = format!("{}:{}", config.bind_ip, config.port);
        let listener = TcpListener::bind(&bind).map_err(|source| RemoteWsError::Bind {
            bind: bind.clone(),
            source,
        })?;
        let _ = listener.set_nonblocking(true);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let callback = Arc::new(callback);
        let snapshot_provider = Arc::new(snapshot_provider);
        let thread = thread::Builder::new()
            .name("kdmx-remote-ws".to_string())
            .spawn(move || {
                while !thread_stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let client_stop = Arc::clone(&thread_stop);
                            let client_callback = Arc::clone(&callback);
                            let client_snapshot_provider = Arc::clone(&snapshot_provider);
                            let _ = thread::Builder::new()
                                .name("kdmx-remote-ws-client".to_string())
                                .spawn(move || {
                                    handle_connection(
                                        stream,
                                        client_stop,
                                        client_callback.as_ref(),
                                        client_snapshot_provider.as_ref(),
                                    );
                                });
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => {
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                }
            })
            .map_err(|source| RemoteWsError::Bind { bind, source })?;

        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for RemoteWsServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn event_from_text(text: &str) -> Result<RemoteInputEvent, RemoteParseError> {
    match request_from_text(text)? {
        RemoteClientRequest::Event(event) => Ok(event),
        RemoteClientRequest::GetSnapshot => {
            Err(RemoteParseError::UnknownType("getSnapshot".to_string()))
        }
    }
}

pub fn request_from_text(text: &str) -> Result<RemoteClientRequest, RemoteParseError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|error| RemoteParseError::InvalidJson(error.to_string()))?;
    let command_type = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or(RemoteParseError::MissingType)?;
    match command_type {
        "getSnapshot" => Ok(RemoteClientRequest::GetSnapshot),
        "setAttribute" => Ok(RemoteClientRequest::Event(RemoteInputEvent::SetAttribute {
            fixture_id: read_u64(&value, "fixture_id")?,
            attribute: read_string(&value, "attribute")?,
            value: read_u16_value(&value, "value")?,
        })),
        "setFixtureHighlight" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetFixtureHighlight {
                fixture_id: read_u64(&value, "fixture_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        "setFixtureSolo" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetFixtureSolo {
                fixture_id: read_u64(&value, "fixture_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        "setFixturePark" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetFixturePark {
                fixture_id: read_u64(&value, "fixture_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        "setGroupHighlight" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetGroupHighlight {
                group_id: read_string(&value, "group_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        "setGroupSolo" => Ok(RemoteClientRequest::Event(RemoteInputEvent::SetGroupSolo {
            group_id: read_string(&value, "group_id")?,
            enabled: read_bool(&value, "enabled")?,
        })),
        "setGroupPark" => Ok(RemoteClientRequest::Event(RemoteInputEvent::SetGroupPark {
            group_id: read_string(&value, "group_id")?,
            enabled: read_bool(&value, "enabled")?,
        })),
        "blackout" => Ok(RemoteClientRequest::Event(RemoteInputEvent::Blackout(
            read_bool(&value, "enabled")?,
        ))),
        "lightingMaster" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::LightingMaster(read_f32(&value, "master")?),
        )),
        "setGroupSubmaster" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetGroupSubmaster {
                group_id: read_string(&value, "group_id")?,
                level: read_f32(&value, "level")?,
            },
        )),
        "setBpm" => Ok(RemoteClientRequest::Event(RemoteInputEvent::SetBpm(
            read_f32(&value, "bpm")?,
        ))),
        "tapBpm" => Ok(RemoteClientRequest::Event(RemoteInputEvent::TapBpm)),
        "triggerCue" => Ok(RemoteClientRequest::Event(RemoteInputEvent::TriggerCue(
            read_u64(&value, "cue_id")?,
        ))),
        "triggerNextCue" => Ok(RemoteClientRequest::Event(RemoteInputEvent::TriggerNextCue)),
        "triggerPreviousCue" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::TriggerPreviousCue,
        )),
        "setCueFadePaused" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetCueFadePaused(read_bool(&value, "paused")?),
        )),
        "setTimelinePlaying" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetTimelinePlaying(read_bool(&value, "playing")?),
        )),
        "seekTimeline" => Ok(RemoteClientRequest::Event(RemoteInputEvent::SeekTimeline {
            position_ms: read_u64(&value, "position_ms")?,
        })),
        "setVideoParam" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoParam {
                layer_id: read_u64(&value, "layer_id")?,
                param: video_param_from_str(&read_string(&value, "param")?)
                    .ok_or(RemoteParseError::InvalidField("param"))?,
                value: read_f32(&value, "value")?,
            },
        )),
        "setVideoLayerEnabled" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoLayerEnabled {
                layer_id: read_u64(&value, "layer_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        "setVideoLayerSolo" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoLayerSolo {
                layer_id: read_u64(&value, "layer_id")?,
                solo: read_bool(&value, "solo")?,
            },
        )),
        "setVideoPlaying" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoPlaying {
                layer_id: read_u64(&value, "layer_id")?,
                playing: read_bool(&value, "playing")?,
            },
        )),
        "seekVideoLayer" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SeekVideoLayer {
                layer_id: read_u64(&value, "layer_id")?,
                position_ms: read_u64(&value, "position_ms")?,
            },
        )),
        "setVideoLoop" => Ok(RemoteClientRequest::Event(RemoteInputEvent::SetVideoLoop {
            layer_id: read_u64(&value, "layer_id")?,
            enabled: read_bool(&value, "enabled")?,
            loop_start_ms: read_optional_u64(&value, "loop_start_ms")?,
            loop_end_ms: read_optional_u64(&value, "loop_end_ms")?,
        })),
        "addVideoCuePoint" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::AddVideoCuePoint {
                layer_id: read_u64(&value, "layer_id")?,
                position_ms: read_optional_u64(&value, "position_ms")?,
            },
        )),
        "removeVideoCuePoint" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::RemoveVideoCuePoint {
                layer_id: read_u64(&value, "layer_id")?,
                position_ms: read_u64(&value, "position_ms")?,
            },
        )),
        "jumpVideoCuePoint" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::JumpVideoCuePoint {
                layer_id: read_u64(&value, "layer_id")?,
                cue_point_index: read_u64(&value, "cue_point_index")? as usize,
            },
        )),
        "setVideoOutputEnabled" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoOutputEnabled {
                output_id: read_u64(&value, "output_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        "setVideoOutputOpacity" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoOutputOpacity {
                output_id: read_u64(&value, "output_id")?,
                opacity: read_f32(&value, "opacity")?,
            },
        )),
        "fadeVideoOutputOpacity" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::FadeVideoOutputOpacity {
                output_id: read_u64(&value, "output_id")?,
                opacity: read_f32(&value, "opacity")?,
                duration_ms: read_u64(&value, "duration_ms")?,
            },
        )),
        "setVideoOutputBlackout" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoOutputBlackout {
                output_id: read_u64(&value, "output_id")?,
                blackout: read_bool(&value, "blackout")?,
            },
        )),
        "videoMaster" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::VideoMasterOpacity(read_f32(&value, "opacity")?),
        )),
        "videoBlackout" => Ok(RemoteClientRequest::Event(RemoteInputEvent::VideoBlackout(
            read_bool(&value, "enabled")?,
        ))),
        other => Err(RemoteParseError::UnknownType(other.to_string())),
    }
}

fn handle_connection<F, S>(
    stream: TcpStream,
    stop: Arc<AtomicBool>,
    callback: &F,
    snapshot_provider: &S,
) where
    F: Fn(RemoteInputEvent) + ?Sized,
    S: Fn() -> EngineSnapshot + ?Sized,
{
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    let mut peek_buffer = [0u8; HTTP_PEEK_SIZE];
    match stream.peek(&mut peek_buffer) {
        Ok(size) if is_websocket_request(&peek_buffer[..size]) => {
            handle_websocket_client(stream, stop, callback, snapshot_provider);
        }
        Ok(size) => {
            let request = String::from_utf8_lossy(&peek_buffer[..size]);
            let path = request_path(&request).unwrap_or("/");
            serve_http_client(stream, path);
        }
        Err(_) => {}
    }
}

fn handle_websocket_client<F, S>(
    stream: TcpStream,
    stop: Arc<AtomicBool>,
    callback: &F,
    snapshot_provider: &S,
) where
    F: Fn(RemoteInputEvent) + ?Sized,
    S: Fn() -> EngineSnapshot + ?Sized,
{
    let Ok(mut websocket) = accept(stream) else {
        return;
    };
    while !stop.load(Ordering::Relaxed) {
        match websocket.read() {
            Ok(Message::Text(text)) => match request_from_text(&text) {
                Ok(RemoteClientRequest::Event(event)) => {
                    callback(event);
                    let _ =
                        websocket.send(Message::Text(r#"{"ok":true,"type":"ack"}"#.to_string()));
                }
                Ok(RemoteClientRequest::GetSnapshot) => {
                    let response = snapshot_response_json(&(snapshot_provider)());
                    let _ = websocket.send(Message::Text(response));
                }
                Err(error) => {
                    let _ = websocket.send(Message::Text(format!(
                        r#"{{"ok":false,"error":"{error:?}"}}"#
                    )));
                }
            },
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }
}

fn snapshot_response_json(snapshot: &EngineSnapshot) -> String {
    serde_json::json!({
        "ok": true,
        "type": "snapshot",
        "snapshot": snapshot,
    })
    .to_string()
}

fn serve_http_client(mut stream: TcpStream, path: &str) {
    let mut discard = [0u8; HTTP_PEEK_SIZE];
    let _ = stream.read(&mut discard);
    let response = http_response_for_path(path);
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn is_websocket_request(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    text.contains("upgrade: websocket")
}

fn request_path(request: &str) -> Option<&str> {
    let mut parts = request.lines().next()?.split_whitespace();
    let method = parts.next()?;
    if method != "GET" {
        return None;
    }
    parts.next()
}

fn http_response_for_path(path: &str) -> String {
    let (status, content_type, body) = match path {
        "/" | "/remote" | "/index.html" => ("200 OK", "text/html; charset=utf-8", REMOTE_PAGE_HTML),
        "/manifest.webmanifest" => (
            "200 OK",
            "application/manifest+json; charset=utf-8",
            REMOTE_MANIFEST,
        ),
        "/icon.svg" | "/apple-touch-icon.svg" => ("200 OK", "image/svg+xml", REMOTE_ICON_SVG),
        "/remote-sw.js" => (
            "200 OK",
            "application/javascript; charset=utf-8",
            REMOTE_SERVICE_WORKER_JS,
        ),
        "/health" => (
            "200 OK",
            "application/json; charset=utf-8",
            r#"{"ok":true}"#,
        ),
        _ => ("404 Not Found", "text/plain; charset=utf-8", "Not found"),
    };
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.as_bytes().len()
    )
}

fn read_string(value: &Value, key: &'static str) -> Result<String, RemoteParseError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .ok_or(RemoteParseError::MissingField(key))
}

fn read_bool(value: &Value, key: &'static str) -> Result<bool, RemoteParseError> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or(RemoteParseError::MissingField(key))
}

fn read_u64(value: &Value, key: &'static str) -> Result<u64, RemoteParseError> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or(RemoteParseError::MissingField(key))
}

fn read_optional_u64(value: &Value, key: &'static str) -> Result<Option<u64>, RemoteParseError> {
    match value.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(number) => number
            .as_u64()
            .map(Some)
            .ok_or(RemoteParseError::InvalidField(key)),
    }
}

fn read_f32(value: &Value, key: &'static str) -> Result<f32, RemoteParseError> {
    let number = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or(RemoteParseError::MissingField(key))?;
    if number.is_finite() {
        Ok(number as f32)
    } else {
        Err(RemoteParseError::InvalidField(key))
    }
}

fn read_u16_value(value: &Value, key: &'static str) -> Result<u16, RemoteParseError> {
    let number = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or(RemoteParseError::MissingField(key))?;
    if !number.is_finite() {
        return Err(RemoteParseError::InvalidField(key));
    }
    let scaled = if (0.0..=1.0).contains(&number) {
        number * 65_535.0
    } else {
        number
    };
    Ok(scaled.round().clamp(0.0, 65_535.0) as u16)
}

fn video_param_from_str(value: &str) -> Option<VideoParam> {
    match value.to_ascii_lowercase().as_str() {
        "opacity" => Some(VideoParam::Opacity),
        "speed" => Some(VideoParam::Speed),
        "position" | "position_ms" | "positionms" => Some(VideoParam::PositionMs),
        "bpm_sync" | "bpm_sync_enabled" | "bpmsync" | "bpmsyncenabled" => {
            Some(VideoParam::BpmSyncEnabled)
        }
        "bpm_sync_ratio" | "bpmsyncratio" | "sync_ratio" | "syncratio" => {
            Some(VideoParam::BpmSyncRatio)
        }
        "bpm_sync_loop_bars" | "bpmsyncloopbars" | "loop_bars" | "loopbars" => {
            Some(VideoParam::BpmSyncLoopBars)
        }
        "transform_x" | "transformx" | "x" => Some(VideoParam::TransformX),
        "transform_y" | "transformy" | "y" => Some(VideoParam::TransformY),
        "scale_x" | "scalex" | "transform_scale_x" | "transformscalex" => {
            Some(VideoParam::TransformScaleX)
        }
        "scale_y" | "scaley" | "transform_scale_y" | "transformscaley" => {
            Some(VideoParam::TransformScaleY)
        }
        "rotation" | "rotation_deg" | "rotationdeg" | "transform_rotation_deg" => {
            Some(VideoParam::TransformRotationDeg)
        }
        "crop_left" | "cropleft" => Some(VideoParam::TransformCropLeft),
        "crop_top" | "croptop" => Some(VideoParam::TransformCropTop),
        "crop_right" | "cropright" => Some(VideoParam::TransformCropRight),
        "crop_bottom" | "cropbottom" => Some(VideoParam::TransformCropBottom),
        "brightness" | "color_brightness" | "colorbrightness" => Some(VideoParam::ColorBrightness),
        "contrast" | "color_contrast" | "colorcontrast" => Some(VideoParam::ColorContrast),
        "hue" | "hue_deg" | "huedeg" | "color_hue" | "color_hue_deg" => {
            Some(VideoParam::ColorHueDeg)
        }
        "saturation" | "sat" | "color_saturation" | "colorsaturation" => {
            Some(VideoParam::ColorSaturation)
        }
        "gamma" | "color_gamma" | "colorgamma" => Some(VideoParam::ColorGamma),
        "pixelate" | "fx_pixelate" | "fxpixelate" => Some(VideoParam::FxPixelate),
        "blur" | "fx_blur" | "fxblur" => Some(VideoParam::FxBlur),
        "glow" | "fx_glow" | "fxglow" => Some(VideoParam::FxGlow),
        "edge" | "edges" | "fx_edge" | "fxedge" => Some(VideoParam::FxEdge),
        "key_red" | "keyred" | "color_key_red" | "colorkeyred" => Some(VideoParam::FxKeyRed),
        "key_green" | "keygreen" | "color_key_green" | "colorkeygreen" => {
            Some(VideoParam::FxKeyGreen)
        }
        "key_blue" | "keyblue" | "color_key_blue" | "colorkeyblue" => Some(VideoParam::FxKeyBlue),
        "key_threshold" | "keythreshold" | "color_key_threshold" | "colorkeythreshold" => {
            Some(VideoParam::FxKeyThreshold)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lighting_and_transport_commands() {
        assert_eq!(
            request_from_text(r#"{"type":"getSnapshot"}"#),
            Ok(RemoteClientRequest::GetSnapshot)
        );
        assert_eq!(
            event_from_text(
                r#"{"type":"setAttribute","fixture_id":4,"attribute":"Dimmer","value":0.5}"#
            ),
            Ok(RemoteInputEvent::SetAttribute {
                fixture_id: 4,
                attribute: "Dimmer".to_string(),
                value: 32_768,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"triggerCue","cue_id":9}"#),
            Ok(RemoteInputEvent::TriggerCue(9))
        );
        assert_eq!(
            event_from_text(r#"{"type":"setFixtureHighlight","fixture_id":4,"enabled":true}"#),
            Ok(RemoteInputEvent::SetFixtureHighlight {
                fixture_id: 4,
                enabled: true,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setFixtureSolo","fixture_id":4,"enabled":false}"#),
            Ok(RemoteInputEvent::SetFixtureSolo {
                fixture_id: 4,
                enabled: false,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setFixturePark","fixture_id":4,"enabled":true}"#),
            Ok(RemoteInputEvent::SetFixturePark {
                fixture_id: 4,
                enabled: true,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setGroupSubmaster","group_id":"front","level":0.25}"#),
            Ok(RemoteInputEvent::SetGroupSubmaster {
                group_id: "front".to_string(),
                level: 0.25,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setGroupHighlight","group_id":"front","enabled":true}"#),
            Ok(RemoteInputEvent::SetGroupHighlight {
                group_id: "front".to_string(),
                enabled: true,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setGroupSolo","group_id":"front","enabled":false}"#),
            Ok(RemoteInputEvent::SetGroupSolo {
                group_id: "front".to_string(),
                enabled: false,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setGroupPark","group_id":"front","enabled":true}"#),
            Ok(RemoteInputEvent::SetGroupPark {
                group_id: "front".to_string(),
                enabled: true,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"triggerNextCue"}"#),
            Ok(RemoteInputEvent::TriggerNextCue)
        );
        assert_eq!(
            event_from_text(r#"{"type":"setCueFadePaused","paused":true}"#),
            Ok(RemoteInputEvent::SetCueFadePaused(true))
        );
        assert_eq!(
            event_from_text(r#"{"type":"setTimelinePlaying","playing":true}"#),
            Ok(RemoteInputEvent::SetTimelinePlaying(true))
        );
        assert_eq!(
            event_from_text(r#"{"type":"seekTimeline","position_ms":24000}"#),
            Ok(RemoteInputEvent::SeekTimeline { position_ms: 24000 })
        );
    }

    #[test]
    fn parses_video_commands() {
        assert_eq!(
            event_from_text(
                r#"{"type":"setVideoParam","layer_id":2,"param":"transform_x","value":-0.25}"#
            ),
            Ok(RemoteInputEvent::SetVideoParam {
                layer_id: 2,
                param: VideoParam::TransformX,
                value: -0.25,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"videoMaster","opacity":0.7}"#),
            Ok(RemoteInputEvent::VideoMasterOpacity(0.7))
        );
        assert_eq!(
            event_from_text(
                r#"{"type":"setVideoParam","layer_id":2,"param":"bpm_sync","value":1}"#
            ),
            Ok(RemoteInputEvent::SetVideoParam {
                layer_id: 2,
                param: VideoParam::BpmSyncEnabled,
                value: 1.0,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"lightingMaster","master":0.4}"#),
            Ok(RemoteInputEvent::LightingMaster(0.4))
        );
        assert_eq!(
            event_from_text(r#"{"type":"videoBlackout","enabled":true}"#),
            Ok(RemoteInputEvent::VideoBlackout(true))
        );
        assert_eq!(
            event_from_text(r#"{"type":"setVideoLayerEnabled","layer_id":2,"enabled":false}"#),
            Ok(RemoteInputEvent::SetVideoLayerEnabled {
                layer_id: 2,
                enabled: false,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setVideoLayerSolo","layer_id":2,"solo":true}"#),
            Ok(RemoteInputEvent::SetVideoLayerSolo {
                layer_id: 2,
                solo: true,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setVideoPlaying","layer_id":2,"playing":true}"#),
            Ok(RemoteInputEvent::SetVideoPlaying {
                layer_id: 2,
                playing: true,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"seekVideoLayer","layer_id":2,"position_ms":1500}"#),
            Ok(RemoteInputEvent::SeekVideoLayer {
                layer_id: 2,
                position_ms: 1500,
            })
        );
        assert_eq!(
            event_from_text(
                r#"{"type":"setVideoLoop","layer_id":2,"enabled":true,"loop_start_ms":500,"loop_end_ms":2500}"#
            ),
            Ok(RemoteInputEvent::SetVideoLoop {
                layer_id: 2,
                enabled: true,
                loop_start_ms: Some(500),
                loop_end_ms: Some(2500),
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"addVideoCuePoint","layer_id":2,"position_ms":1500}"#),
            Ok(RemoteInputEvent::AddVideoCuePoint {
                layer_id: 2,
                position_ms: Some(1500),
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"addVideoCuePoint","layer_id":2}"#),
            Ok(RemoteInputEvent::AddVideoCuePoint {
                layer_id: 2,
                position_ms: None,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"jumpVideoCuePoint","layer_id":2,"cue_point_index":1}"#),
            Ok(RemoteInputEvent::JumpVideoCuePoint {
                layer_id: 2,
                cue_point_index: 1,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"removeVideoCuePoint","layer_id":2,"position_ms":1500}"#),
            Ok(RemoteInputEvent::RemoveVideoCuePoint {
                layer_id: 2,
                position_ms: 1500,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setVideoOutputEnabled","output_id":4,"enabled":false}"#),
            Ok(RemoteInputEvent::SetVideoOutputEnabled {
                output_id: 4,
                enabled: false,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setVideoOutputOpacity","output_id":4,"opacity":0.6}"#),
            Ok(RemoteInputEvent::SetVideoOutputOpacity {
                output_id: 4,
                opacity: 0.6,
            })
        );
        assert_eq!(
            event_from_text(
                r#"{"type":"fadeVideoOutputOpacity","output_id":4,"opacity":0,"duration_ms":1000}"#
            ),
            Ok(RemoteInputEvent::FadeVideoOutputOpacity {
                output_id: 4,
                opacity: 0.0,
                duration_ms: 1000,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setVideoOutputBlackout","output_id":4,"blackout":true}"#),
            Ok(RemoteInputEvent::SetVideoOutputBlackout {
                output_id: 4,
                blackout: true,
            })
        );
    }

    #[test]
    fn rejects_invalid_commands() {
        assert_eq!(
            event_from_text(r#"{"type":"setVideoParam","layer_id":2,"param":"bad","value":1}"#),
            Err(RemoteParseError::InvalidField("param"))
        );
        assert_eq!(
            event_from_text(r#"{"fixture_id":1}"#),
            Err(RemoteParseError::MissingType)
        );
    }

    #[test]
    fn serves_remote_page_and_manifest_over_http() {
        let page = http_response_for_path("/");
        let manifest = http_response_for_path("/manifest.webmanifest");
        let icon = http_response_for_path("/icon.svg");
        let service_worker = http_response_for_path("/remote-sw.js");
        let missing = http_response_for_path("/missing");

        assert!(page.starts_with("HTTP/1.1 200 OK"));
        assert!(page.contains("KDMX Remote"));
        assert!(page.contains(r#"rel="manifest" href="/manifest.webmanifest""#));
        assert!(page.contains("apple-mobile-web-app-capable"));
        assert!(page.contains("serviceWorker"));
        assert!(page.contains("remoteActiveCue"));
        assert!(page.contains("renderLiveDesk"));
        assert!(page.contains("new WebSocket"));
        assert!(page.contains("getSnapshot"));
        assert!(page.contains("fixtureCount"));
        assert!(page.contains("fixtureFaderBank"));
        assert!(page.contains("renderFixtureFaderBank"));
        assert!(page.contains("setFixtureAttributeFromInput"));
        assert!(page.contains("cueButtons"));
        assert!(page.contains("cuePad"));
        assert!(page.contains("moveCuePadBank"));
        assert!(page.contains("layerList"));
        assert!(page.contains("videoOutputList"));
        assert!(page.contains("renderVideoOutputList"));
        assert!(page.contains("timelineInfo"));
        assert!(page.contains("timelineSeekMs"));
        assert!(page.contains("setTimelinePlaying"));
        assert!(page.contains("setInterval(requestSnapshot,1000)"));
        assert!(page.contains("setVideoLayerEnabled"));
        assert!(page.contains("setVideoLayerSolo"));
        assert!(page.contains("videoDeckControls"));
        assert!(page.contains("setLayerParamFromInput"));
        assert!(page.contains("setLayerLoopFromInputs"));
        assert!(page.contains("setVideoLoop"));
        assert!(page.contains("setVideoOutputEnabled"));
        assert!(page.contains("fadeVideoOutputOpacity"));
        assert!(page.contains("setVideoPlaying"));
        assert!(page.contains("seekVideoLayer"));
        assert!(page.contains("addVideoCuePoint"));
        assert!(page.contains("jumpVideoCuePoint"));
        assert!(page.contains("setFixtureHighlight"));
        assert!(page.contains("setFixtureSolo"));
        assert!(manifest.contains("application/manifest+json"));
        assert!(manifest.contains(r#""short_name":"KDMX""#));
        assert!(manifest.contains(r#""icons""#));
        assert!(icon.contains("image/svg+xml"));
        assert!(icon.contains("<svg"));
        assert!(service_worker.contains("application/javascript"));
        assert!(service_worker.contains("CACHE_NAME"));
        assert!(missing.starts_with("HTTP/1.1 404 Not Found"));
    }

    #[test]
    fn detects_websocket_upgrade_and_http_paths() {
        let request = "GET /ws HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\n\r\n";
        let http = "GET /remote HTTP/1.1\r\nHost: localhost\r\n\r\n";

        assert!(is_websocket_request(request.as_bytes()));
        assert!(!is_websocket_request(http.as_bytes()));
        assert_eq!(request_path(http), Some("/remote"));
    }

    #[test]
    fn serializes_snapshot_response() {
        let response = snapshot_response_json(&EngineSnapshot::default());

        assert!(response.contains(r#""ok":true"#));
        assert!(response.contains(r#""type":"snapshot""#));
        assert!(response.contains(r#""fixtures":[]"#));
    }
}

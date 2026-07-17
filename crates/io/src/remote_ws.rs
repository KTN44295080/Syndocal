use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use protocol::{
    canonical_video_output_mapping_field, ClockSource, CueId, EffectId, EngineSnapshot, FixtureId,
    NodeGraphId, RemoteClientSummary, RemoteControlConfig, RemoteControlStatus, VideoLayerId,
    VideoOutputId, VideoOutputMapping, VideoParam, VideoRuntimeStatus,
};
use serde_json::{json, Value};
use thiserror::Error;
use tungstenite::{accept, Message};

use crate::{parse_clock_source_label, parse_timecode_position_ms};

const HTTP_PEEK_SIZE: usize = 2048;
const REMOTE_PAGE_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
  <meta name="theme-color" content="#111419">
  <meta name="mobile-web-app-capable" content="yes">
  <meta name="apple-mobile-web-app-capable" content="yes">
  <meta name="apple-mobile-web-app-title" content="Syndocal">
  <link rel="manifest" href="/manifest.webmanifest">
  <link rel="icon" href="/icon.svg" type="image/svg+xml">
  <link rel="apple-touch-icon" href="/icon.svg">
  <title>Syndocal Remote</title>
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
    .status{padding:6px 10px;border-radius:999px;background:#202833;color:#bcc8d7}.ok{color:#8ff0b6;background:#123322}.bad{color:#ffc0c0;background:#3a1517}.warn{color:#ffe28a;background:#342d12}
    .summary{display:grid;grid-template-columns:repeat(auto-fit,minmax(104px,1fr));gap:8px}.tile{border:1px solid #242c36;border-radius:7px;background:#121820;padding:9px}.tile strong{display:block;font-size:20px;margin-top:2px}.muted{color:#91a0b2;font-size:12px}
    .liveGrid{display:grid;grid-template-columns:repeat(auto-fit,minmax(130px,1fr));gap:8px}.liveTile{border:1px solid #242c36;border-radius:7px;background:#121820;padding:9px}.liveTile strong{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;margin-top:2px}.liveTile small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#91a0b2;margin-top:2px}
    .backendTile.available{border-color:#247c55}.backendTile.missing{border-color:#a84949}.backendTile.notbuilt{border-color:#75642b}.backendTile span{display:inline-block;margin-top:4px}
    .remoteDmxRoutes{display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:8px}.remoteDmxRoute{display:grid;gap:3px;border:1px solid #242c36;border-radius:7px;background:#121820;padding:8px}.remoteDmxRoute.live{border-color:#247c55}.remoteDmxRoute.off{opacity:.62}.remoteDmxRoute.fail{border-color:#a84949;background:#231519}.remoteDmxRoute strong,.remoteDmxRoute span,.remoteDmxRoute small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remoteDmxRoute span,.remoteDmxRoute small{color:#91a0b2;font-size:12px}
    .row{display:grid;grid-template-columns:1fr auto;align-items:center;gap:8px;border:1px solid #242c36;border-radius:7px;background:#121820;padding:8px}.row.active{border-color:#287fca;background:#132236}.row.selected{border-color:#d8e5f1;box-shadow:inset 3px 0 0 #d8e5f1}
    .videoDeck{display:grid;gap:8px;border:1px solid #242c36;border-radius:7px;background:#121820;padding:9px}.videoDeck.active{border-color:#287fca;background:#132236}.videoDeckHeader{display:flex;justify-content:space-between;gap:8px;align-items:flex-start}.videoDeckHeader strong,.videoDeckHeader span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.videoDeckControls{display:grid;grid-template-columns:repeat(5,1fr);gap:6px}.videoDeckSliders{display:grid;gap:7px}.deckSlider{display:grid;grid-template-columns:70px minmax(0,1fr) 54px;gap:8px;align-items:center;color:#aab6c6}.deckSlider input{width:100%;padding:0}.deckSlider strong{text-align:right;font-size:12px;color:#edf3fb}.loopGrid{display:grid;grid-template-columns:1fr 1fr;gap:7px}.remoteBpmPresetRow{display:grid;grid-template-columns:repeat(auto-fit,minmax(68px,1fr));gap:6px}.remoteBpmPresetRow button{min-height:30px;padding:3px 6px}.remoteBpmPresetRow button.active{border-color:#74d99f;background:#173122}.cuePointRow{display:flex;gap:6px;overflow:auto;padding-bottom:2px}.cuePointRow button{white-space:nowrap}.effectDeck{display:grid;gap:8px;border:1px solid #242c36;border-radius:7px;background:#121820;padding:9px}.effectDeck.active{border-color:#247c55;background:#13271f}.effectDeck.off{opacity:.72}.effectDeckHeader{display:flex;justify-content:space-between;gap:8px;align-items:flex-start}.effectDeckHeader strong,.effectDeckHeader span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.effectDeckChips{display:flex;gap:5px;flex-wrap:wrap}.effectDeckChip{border:1px solid #323b48;border-radius:999px;background:#151b22;color:#aab6c6;padding:3px 7px;font-size:12px}.effectDeckChip.sync{border-color:#5dd64c;color:#c9f4d4}.effectDeckControls{display:grid;grid-template-columns:repeat(4,1fr);gap:6px}.effectDeckControls button{min-height:30px;padding:3px 6px}
    .remoteVideoTimeline{display:grid;gap:4px}.remoteVideoTimelineSurface{display:block;width:100%;min-height:58px;border:1px solid #242c36;border-radius:6px;background:#0b1017;touch-action:none}.remoteVideoTimelineHit{fill:transparent;cursor:crosshair}.remoteVideoTimelineTrack{fill:#151f2a;stroke:#334253;stroke-width:.8}.remoteVideoTimelineLoop{fill:rgba(85,214,106,.26)}.remoteVideoTimelineLoop.off{fill:rgba(150,164,179,.16)}.remoteVideoTimelineCueLine{stroke:var(--remote-cue-color,#4aa8ff);stroke-width:.65;opacity:.72;pointer-events:none}.remoteVideoTimelineCue{fill:var(--remote-cue-color,#4aa8ff);stroke:#0b1017;stroke-width:.8;cursor:pointer}.remoteVideoTimelineLoopHandle{fill:#55d66a;stroke:#0b1017;stroke-width:.8;cursor:ew-resize}.remoteVideoTimelineLoopHandle.end{fill:#8be88f}.remoteVideoTimelinePlayhead{stroke:#fff;stroke-width:1.15;pointer-events:none}.remoteVideoTimelineReadout{display:grid;grid-template-columns:repeat(3,1fr);gap:4px}.remoteVideoTimelineReadout span{border:1px solid #242c36;border-radius:4px;background:#0d141c;padding:3px 5px;color:#aab6c6;font-size:12px;font-weight:700;text-align:center;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
    .row strong,.row span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.list{display:grid;gap:7px;max-height:220px;overflow:auto}.small{font-size:12px;color:#91a0b2}.bar{height:6px;border-radius:99px;background:#2a3340;overflow:hidden}.bar span{display:block;height:100%;background:#3ba1ff}
    .faderBank{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:8px;max-height:300px;overflow:auto}.fixtureFader{border:1px solid #242c36;border-radius:7px;background:#121820;padding:8px}.fixtureFader span{display:flex;justify-content:space-between;gap:8px}.fixtureFader strong,.fixtureFader small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.fixtureFader input{width:100%;padding:0}
    .visualDesk{display:grid;gap:8px}.remoteTargetInfo{display:grid;gap:5px;border:1px solid #242c36;border-radius:7px;background:#121820;padding:8px}.remoteTargetInfo strong,.remoteTargetInfo span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remoteTargetSubmaster{display:grid;grid-template-columns:82px minmax(0,1fr) 42px;gap:8px;align-items:center;color:#aab6c6}.remoteTargetSubmaster input{width:100%;min-height:24px;padding:0}.remoteTargetSubmaster strong{text-align:right;color:#edf3fb;font-size:12px}.visualCard{display:grid;gap:8px;border:1px solid #242c36;border-radius:7px;background:#121820;padding:9px}.visualCardHeader{display:flex;justify-content:space-between;gap:8px}.visualCardHeader strong,.visualCardHeader span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remoteDimmerQuick{display:grid;grid-template-columns:repeat(4,1fr);gap:6px}.remoteDimmerQuick button{min-height:30px;padding:3px 6px}.remoteDimmerQuick .momentary{border-color:#6b5d2f;background:#2b2418;color:#ffd98a}.remoteDimmerQuick .momentary.active,.remoteDimmerQuick .momentary:active,.remoteDimmerQuick .momentary:focus-visible{border-color:#ffce5c;background:#463817;color:#fff3c4}.remoteDimmerSlider{display:grid;grid-template-columns:54px minmax(0,1fr) 42px;align-items:center;gap:8px;color:#aab6c6}.remoteDimmerSlider input{padding:0}.remoteDimmerSlider strong{text-align:right;color:#edf3fb;font-size:12px}.remotePanTiltPad{position:relative;min-height:180px;overflow:hidden;border:1px solid #323b48;border-radius:6px;background:linear-gradient(to right,transparent calc(50% - 1px),rgba(255,255,255,.25) calc(50% - 1px),rgba(255,255,255,.25) calc(50% + 1px),transparent calc(50% + 1px)),linear-gradient(to bottom,transparent calc(50% - 1px),rgba(255,255,255,.25) calc(50% - 1px),rgba(255,255,255,.25) calc(50% + 1px),transparent calc(50% + 1px)),repeating-linear-gradient(to right,transparent 0,transparent 22px,rgba(255,255,255,.055) 23px),repeating-linear-gradient(to bottom,transparent 0,transparent 22px,rgba(255,255,255,.055) 23px),#171d25;touch-action:none;cursor:crosshair}.remotePanTiltLimit{position:absolute;display:block;min-width:6px;min-height:6px;border:2px solid rgba(116,217,159,.78);border-radius:4px;background:rgba(116,217,159,.08);pointer-events:none}.remotePanTiltPad i{position:absolute;width:20px;height:20px;border:3px solid #d8e5f1;border-radius:999px;background:#101820;box-shadow:0 1px 8px rgba(0,0,0,.45);transform:translate(-50%,-50%);pointer-events:none}.remoteTargetGrid,.remotePanTiltMirrorRow{display:grid;grid-template-columns:repeat(3,1fr);gap:6px}.remoteTargetGrid button,.remotePanTiltMirrorRow button{min-height:30px;padding:3px 6px}.remotePositionFavoriteHeader{display:grid;grid-template-columns:minmax(0,1fr) auto auto;gap:6px;align-items:center}.remotePositionFavoriteGrid{display:grid;grid-template-columns:repeat(auto-fit,minmax(96px,1fr));gap:6px}.remotePositionFavoriteGrid button{display:grid;grid-template-columns:34px minmax(0,1fr);grid-template-rows:auto auto;gap:2px 7px;min-height:46px;padding:5px 7px;text-align:left}.remotePositionFavoriteGrid button.active{border-color:#74d99f;background:#173122}.remotePositionFavoriteMap{grid-row:1/span 2;position:relative;width:34px;height:26px;border:1px solid #323b48;border-radius:4px;background:#101820}.remotePositionFavoriteMap i{position:absolute;width:8px;height:8px;border:2px solid #d8e5f1;border-radius:99px;background:#101820;transform:translate(-50%,-50%)}.remotePositionFavoriteGrid strong,.remotePositionFavoriteGrid small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remotePositionFavoriteGrid small{color:#91a0b2;font-size:11px}.remoteColorPlane{display:grid;grid-template-columns:minmax(170px,1fr) minmax(100px,.42fr);gap:8px}.remoteColorPreview{min-height:120px;border:1px solid #323b48;border-radius:6px}.remoteColorGrid{display:grid;grid-template-columns:repeat(auto-fit,minmax(34px,1fr));gap:6px}.remoteColorPresetGrid{display:grid;grid-template-columns:repeat(auto-fit,minmax(86px,1fr));gap:6px}.remoteColorPresetGrid button{display:grid;grid-template-columns:16px minmax(0,1fr);align-items:center;gap:6px;min-height:32px;padding:5px 7px;text-align:left}.remoteColorPresetGrid button.active,.remoteColorButton.active{border-color:#74d99f;background:#173122;box-shadow:0 0 0 1px rgba(116,217,159,.42) inset}.remoteColorPresetGrid i{width:16px;height:16px;border:1px solid rgba(255,255,255,.45);border-radius:4px}.remoteColorPresetGrid span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remoteColorButton{min-height:34px;border-color:rgba(255,255,255,.2);padding:0}.remoteColorOptions label{display:flex;align-items:center;justify-content:center;min-height:34px;border:1px solid #242c36;border-radius:6px;background:#151b22;color:#edf3fb;font-weight:700;text-transform:uppercase}.remoteColorOptions input{width:auto;min-height:16px;margin:0 7px 0 0}.remoteRgbSliders,.remoteExtraColorSliders{display:grid;grid-template-columns:repeat(3,1fr);gap:6px}.remoteExtraColorSliders{grid-template-columns:repeat(auto-fit,minmax(42px,1fr))}.remoteRgbSliders label,.remoteExtraColorSliders label{border:1px solid #242c36;border-radius:6px;background:#151b22;padding:6px}.remoteRgbSliders input,.remoteExtraColorSliders input{padding:0}.remoteColorWheelList{display:grid;gap:8px}.remoteColorWheelGroup{display:grid;gap:5px}.remoteColorWheelGrid{display:grid;grid-template-columns:repeat(auto-fit,minmax(92px,1fr));gap:5px}.remoteColorWheelGrid button{display:grid;grid-template-columns:18px minmax(0,1fr);grid-template-rows:auto auto;align-items:center;gap:2px 6px;min-height:46px;padding:5px 7px;text-align:left}.remoteColorWheelGrid button.active{border-color:#74d99f;background:#173122}.remoteColorWheelSwatch{grid-row:1/span 2;width:18px;height:18px;border:1px solid rgba(255,255,255,.45);border-radius:4px;background:#2a3340}.remoteColorWheelGrid strong,.remoteColorWheelGrid span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remoteColorWheelGrid span{color:#91a0b2;font-size:11px}.remoteGoboWheelList{display:grid;gap:8px}.remoteGoboWheelGrid{display:grid;grid-template-columns:repeat(auto-fit,minmax(104px,1fr));gap:6px}.remoteGoboWheelGrid button{display:grid;grid-template-columns:30px minmax(0,1fr);grid-template-rows:auto auto;align-items:center;gap:2px 7px;min-height:52px;padding:6px 7px;text-align:left}.remoteGoboWheelGrid button.active{border-color:#74d99f;background:#173122;box-shadow:0 0 0 1px rgba(116,217,159,.42) inset}.remoteGoboWheelGrid strong,.remoteGoboWheelGrid span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remoteGoboWheelGrid span{color:#91a0b2;font-size:11px}.remoteGoboSlotIcon{grid-row:1/span 2;width:30px;height:30px;border:1px solid rgba(255,255,255,.38);border-radius:999px;background:#101820;box-shadow:inset 0 0 0 2px rgba(0,0,0,.35)}.remoteGoboSlotIcon.open{background:#e7edf3}.remoteGoboSlotIcon.bars{background:repeating-linear-gradient(45deg,#e7edf3 0 4px,transparent 4px 8px),#111820}.remoteGoboSlotIcon.dots{background:radial-gradient(circle at 30% 32%,#e7edf3 0 3px,transparent 4px),radial-gradient(circle at 62% 38%,#e7edf3 0 2px,transparent 3px),radial-gradient(circle at 45% 68%,#e7edf3 0 4px,transparent 5px),#111820}.remoteGoboSlotIcon.breakup{background:radial-gradient(circle at 24% 32%,#e7edf3 0 5px,transparent 6px),radial-gradient(circle at 68% 24%,#e7edf3 0 4px,transparent 5px),radial-gradient(circle at 54% 70%,#e7edf3 0 6px,transparent 7px),radial-gradient(circle at 78% 66%,#e7edf3 0 3px,transparent 4px),#111820}.remoteGoboSlotIcon.ring{background:radial-gradient(circle,transparent 0 32%,#e7edf3 34% 54%,transparent 56%),#111820}.remoteGoboSlotIcon.spin{background:conic-gradient(from 20deg,#e7edf3 0 18deg,transparent 18deg 56deg,#e7edf3 56deg 78deg,transparent 78deg 126deg,#e7edf3 126deg 150deg,transparent 150deg 360deg),#111820}.remoteFunctionList{display:grid;gap:8px}.remoteFunctionGroup{display:grid;gap:5px}.remoteFunctionTitle{display:flex;justify-content:space-between;gap:8px;color:#aab6c6}.remoteFunctionTitle strong,.remoteFunctionTitle span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remoteFunctionGrid{display:grid;grid-template-columns:repeat(auto-fit,minmax(88px,1fr));gap:5px}.remoteFunctionGrid button{display:grid;gap:2px;min-height:44px;padding:5px 7px;text-align:left}.remoteFunctionGrid button.active{border-color:#74d99f;background:#173122}.remoteFunctionGrid strong,.remoteFunctionGrid span{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remoteFunctionGrid span{color:#91a0b2;font-size:11px}
    .remoteStagePanel{display:grid;gap:8px}.remoteStageSurface{display:block;width:100%;min-height:230px;aspect-ratio:16/9;border:1px solid #242c36;border-radius:7px;background:#0b1017;touch-action:manipulation}.remoteStageSurface pattern path{fill:none;stroke:#263847;stroke-width:.35}.remoteStageFloor{fill:#0d1720}.remoteStageGrid{opacity:.62}.remoteStageAxis{stroke:#405367;stroke-width:.04;stroke-dasharray:.22 .22}.remoteStageObject{fill:rgba(120,137,156,.14);stroke:#708095;stroke-width:.07}.remoteStageObject.kind-stage{fill:rgba(82,101,126,.18)}.remoteStageObject.kind-truss{fill:rgba(242,193,78,.16);stroke:#d5ad4b}.remoteStageObject.kind-screen{fill:rgba(74,168,255,.16);stroke:#63b8ff}.remoteStageObject.kind-riser{fill:rgba(116,217,159,.13);stroke:#74d99f}.remoteStageObject.kind-mask{fill:rgba(235,92,92,.12);stroke:#e26d6d}.remoteStageSurfaceLabel{fill:#d8e5f1;font-size:.42px;font-weight:700;paint-order:stroke;stroke:#0b1017;stroke-width:.12px}.remoteStageOutputGroup{cursor:pointer}.remoteStageOutput{fill:rgba(34,120,197,.2);stroke:#4aa8ff;stroke-width:.09}.remoteStageOutput.selected{stroke:#fff;stroke-width:.18;filter:drop-shadow(0 0 3px rgba(74,168,255,.55))}.remoteStageOutput.inactive{opacity:.36}.remoteStageOutputCenter{fill:#4aa8ff;stroke:#0b1017;stroke-width:.08}.remoteStageBeam{stroke:rgba(242,193,78,.3);stroke-width:.08;stroke-dasharray:.24 .18}.remoteStageFixture{cursor:pointer}.remoteStageFixtureShape{fill:#f2c14e;stroke:#0b1017;stroke-width:.12}.remoteStageFixtureShape.highlighted{fill:#8af071}.remoteStageFixtureShape.soloed{stroke:#fff;stroke-width:.18}.remoteStageFixtureShape.parked{fill:#8d96a3}.remoteStageFixture.selected .remoteStageFixtureShape{stroke:#fff;stroke-width:.22}.remoteStageReadout{display:grid;grid-template-columns:repeat(auto-fit,minmax(96px,1fr));gap:6px}.remoteStageReadout span{border:1px solid #242c36;border-radius:6px;background:#121820;padding:6px 8px;color:#aab6c6;font-size:12px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remoteStageSelection{display:grid;gap:6px}.remoteStageSelectionRow{display:grid;grid-template-columns:minmax(0,1fr) repeat(3,minmax(54px,auto));gap:6px;align-items:center;border:1px solid #242c36;border-radius:7px;background:#101720;padding:7px}.remoteStageSelectionRow.output{grid-template-columns:minmax(0,1fr) repeat(5,minmax(48px,auto));border-color:#244a5d;background:#0d1a22}.remoteStageSelectionRow.empty{grid-template-columns:minmax(0,1fr)}.remoteStageSelectionRow strong,.remoteStageSelectionRow span{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remoteStageSelectionRow strong{color:#edf3fb;font-size:12px}.remoteStageSelectionRow span{color:#91a0b2;font-size:11px}.remoteStageSelectionRow button{min-height:30px;padding:4px 8px;font-size:12px}.remoteStageLegend{display:flex;gap:8px;flex-wrap:wrap;color:#91a0b2;font-size:12px}.remoteStageLegend b{display:inline-flex;width:10px;height:10px;border-radius:2px;margin-right:4px;vertical-align:-1px}.remoteStageLegend .fixture{background:#f2c14e}.remoteStageLegend .output{background:#4aa8ff}.remoteStageLegend .object{background:#708095}
    .remoteOpticsGrid{display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:8px}.remoteOpticsCard{display:grid;gap:7px;border:1px solid #242c36;border-radius:7px;background:#0d141c;padding:8px}.remoteOpticsTitle,.remoteOpticsMeta{display:flex;justify-content:space-between;gap:8px}.remoteOpticsTitle strong,.remoteOpticsTitle span,.remoteOpticsMeta span,.remoteOpticsMeta b{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.remoteOpticsTitle span,.remoteOpticsMeta span{color:#91a0b2;font-size:12px}.remoteOpticsPreview{position:relative;display:grid;place-items:center;min-height:86px;overflow:hidden;border:1px solid #323b48;border-radius:6px;background:linear-gradient(to right,transparent calc(var(--remote-optics-level) - 1px),rgba(242,193,78,.48) calc(var(--remote-optics-level) - 1px),rgba(242,193,78,.48) calc(var(--remote-optics-level) + 1px),transparent calc(var(--remote-optics-level) + 1px)),radial-gradient(circle,#182430 0,#0b1117 66%);touch-action:none;cursor:ew-resize}.remoteOpticsHalo,.remoteOpticsCore,.remoteOpticsPrism{position:absolute;border-radius:999px;pointer-events:none}.remoteOpticsHalo{width:calc(var(--remote-optics-size) + 24px);height:calc(var(--remote-optics-size) + 24px);background:rgba(242,193,78,.2);filter:blur(calc(var(--remote-optics-blur) + 3px));opacity:var(--remote-optics-opacity)}.remoteOpticsCore{width:var(--remote-optics-size);height:var(--remote-optics-size);background:#f4e6bc;box-shadow:0 0 14px rgba(242,193,78,.45);filter:blur(var(--remote-optics-blur));opacity:var(--remote-optics-opacity)}.remoteOpticsPreview.iris .remoteOpticsCore{box-shadow:0 0 0 6px rgba(16,24,32,.8) inset,0 0 14px rgba(242,193,78,.45)}.remoteOpticsPreview.focus .remoteOpticsCore,.remoteOpticsPreview.frost .remoteOpticsCore{background:rgba(244,230,188,.86)}.remoteOpticsPreview.strobe .remoteOpticsCore{background:repeating-linear-gradient(90deg,#f4e6bc 0 6px,rgba(244,230,188,.2) 6px 12px)}.remoteOpticsPrism{width:var(--remote-optics-ring-size);height:var(--remote-optics-ring-size);border:1px dashed rgba(242,193,78,var(--remote-optics-prism-opacity));opacity:var(--remote-optics-prism-opacity);transform:rotate(var(--remote-optics-prism-rotation))}.remoteOpticsScale{position:absolute;left:8px;right:8px;bottom:7px;height:4px;border-radius:99px;background:#222b36;overflow:hidden}.remoteOpticsScale i{display:block;width:var(--remote-optics-level);height:100%;background:#f2c14e}.remoteOpticsCard input{width:100%;padding:0}.remoteOpticsQuickRow,.remoteOpticsFunctionChips{display:grid;grid-template-columns:repeat(3,1fr);gap:5px}.remoteOpticsQuickRow button,.remoteOpticsFunctionChips button{min-height:29px;padding:3px 6px;font-size:12px}.remoteOpticsFunctionChips{grid-template-columns:repeat(auto-fit,minmax(62px,1fr))}.remoteOpticsFunctionChips button.active{border-color:#74d99f;background:#173122}.remoteOpticsFunctionChips button{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
    .remoteOutputMap{display:grid;gap:7px;margin-top:8px;border:1px solid #242c36;border-radius:7px;background:#0d141c;padding:8px}.remoteMapSurface{display:block;width:100%;min-height:86px;aspect-ratio:16/9;border-radius:5px;background:#0b1117;touch-action:none}.remoteMapFloor{fill:#0d1720}.remoteMapGrid{opacity:.5}.remoteMapSurface pattern path{fill:none;stroke:#263847;stroke-width:.5}.remoteMapAxis{stroke:#355064;stroke-width:.45;stroke-dasharray:2 2}.remoteMapBase{fill:rgba(255,255,255,.025);stroke:#536273;stroke-width:.8;stroke-dasharray:2 2}.remoteMapWarp{fill:rgba(34,120,197,.18);stroke:#4aa8ff;stroke-width:1.2}.remoteMapHandleGuide{stroke:#f0b35a;stroke-width:.75;stroke-dasharray:1.8 1.8}.remoteMapBasePoint{fill:#536273}.remoteMapHandle{fill:#f2c14e;stroke:#101820;stroke-width:1.2;cursor:grab}.remoteMapCenterHandle{fill:#101820;stroke:#d8e5f1;stroke-width:1.4;cursor:move}.remoteMapKeyGuide{stroke:#74d99f;stroke-width:.75;stroke-dasharray:1.8 1.8}.remoteMapKeyHandle{fill:#74d99f;stroke:#101820;stroke-width:1.15;cursor:grab}.remoteMapKeyHandle.y{fill:#67b7ff}.remoteMapHandle:active,.remoteMapCenterHandle:active,.remoteMapKeyHandle:active{cursor:grabbing}.remoteMapHandleLabel,.remoteMapKeyLabel{fill:#d8e5f1;font-size:5px;font-weight:700;pointer-events:none}.remoteMapKeyLabel{fill:#cfe8db;font-size:4.6px}.remoteMapButtons,.remoteMapPresetRow,.remoteMapKeyPresetRow{display:grid;grid-template-columns:repeat(auto-fit,minmax(62px,1fr));gap:5px}.remoteMapButtons button,.remoteMapPresetRow button{min-height:28px;padding:3px 6px;font-size:12px}.remoteMapPresetRow button.active,.remoteMapKeyPresetRow button.active{border-color:#74d99f;background:#173122;color:#edf3fb}.remoteMapKeyPresetRow button{display:grid;justify-items:center;gap:2px;min-height:44px;padding:4px 5px;font-size:11px}.remoteMapKeyMini{width:38px;height:22px}.remoteMapKeyMini polygon{fill:rgba(34,120,197,.24);stroke:#74d99f;stroke-width:1.4}.remoteMapModeRow{display:grid;grid-template-columns:repeat(3,1fr);gap:5px}.remoteMapModeRow button{min-height:28px;padding:3px 6px;font-size:12px}.remoteMapModeRow button.active{border-color:#4aa8ff;background:#16304a;color:#edf3fb}.remoteMapTuningGrid{display:grid;gap:6px}.remoteMapTuningGrid label{display:grid;grid-template-columns:56px minmax(0,1fr) 54px;gap:7px;align-items:center;color:#aab6c6;font-size:12px}.remoteMapTuningGrid input{width:100%;padding:0}.remoteMapTuningGrid strong{font-size:11px;text-align:right;color:#edf3fb;white-space:nowrap}.remoteOutputMap span{color:#91a0b2;font-size:12px;overflow-wrap:anywhere}
    .log{min-height:32px;color:#91a0b2;overflow-wrap:anywhere}.inline{display:flex;gap:8px;align-items:center;flex-wrap:wrap}
    .cuePadHeader{display:grid;grid-template-columns:1fr auto auto auto auto;gap:8px;align-items:center}.compact{display:inline-flex;gap:6px;align-items:center}.compact input{min-height:0;width:auto}.cuePadGrid{display:grid;grid-template-columns:repeat(5,1fr);gap:8px}.cuePad{display:grid;grid-template-columns:24px minmax(0,1fr);grid-template-rows:auto auto;gap:2px 8px;min-height:58px;text-align:left}.cuePad span{grid-row:1 / span 2;display:grid;width:24px;height:24px;place-items:center;border-radius:4px;background:#151b22;color:#91a0b2;font-weight:700}.cuePad strong,.cuePad small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.cuePad.active{border-color:#74d99f;background:#173122}.cuePad.next{border-color:#5f9ddc}
    .sceneToolbar{display:grid;grid-template-columns:minmax(0,1fr) auto auto;gap:8px;align-items:end}.sceneSelectorGrid{display:grid;grid-template-columns:repeat(auto-fit,minmax(132px,1fr));gap:8px;max-height:260px;overflow:auto}.sceneButton{display:grid;grid-template-columns:28px minmax(0,1fr);grid-template-rows:auto auto auto auto;gap:2px 8px;min-height:72px;text-align:left}.sceneButton b{grid-row:1/span 4;display:grid;width:28px;height:28px;place-items:center;border-radius:5px;background:#151b22;color:#91a0b2;font-size:12px}.sceneButton strong,.sceneButton span,.sceneButton small,.sceneButton em{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.sceneButton span,.sceneButton small{color:#91a0b2;font-size:11px}.sceneButton em{width:max-content;border:1px solid rgba(116,217,159,.58);border-radius:999px;background:rgba(116,217,159,.13);padding:1px 7px;color:#a8ff99;font-size:11px;font-style:normal;font-weight:700;text-transform:uppercase}.sceneButton.active{border-color:#74d99f;background:#173122}.sceneButton.next{border-color:#5f9ddc}.sceneButton.firstMatch{border-color:#74d99f;box-shadow:inset 0 0 0 1px rgba(116,217,159,.42)}
    .remoteTouchHeader{display:flex;align-items:center;justify-content:space-between;gap:8px}.remoteTouchPages{display:flex;gap:6px;overflow:auto}.remoteTouchPages button{min-width:72px;min-height:48px}.remoteTouchPages button.active{border-color:#f2c14e;background:#342719}.remoteTouchSurface{display:grid;grid-template-columns:repeat(12,minmax(0,1fr));grid-template-rows:repeat(8,minmax(48px,1fr));gap:4px;min-height:384px;border:1px solid #323b48;background:#0d1116;padding:4px;overflow:hidden}.remoteTouchControl{display:grid;min-width:0;min-height:48px;place-items:stretch;border:1px solid #323b48;border-radius:3px;background:#171d25;overflow:hidden}.remoteTouchControl>button,.remoteTouchControl>label,.remoteTouchControl>div,.remoteTouchControl>strong{min-width:0;min-height:48px}.remoteTouchLabel,.remoteTouchImage,.remoteTouchUnsupported{display:grid;place-items:center;padding:6px;text-align:center}.remoteTouchButton{width:100%;height:100%;min-height:48px;font-weight:800}.remoteTouchButton.primary{font-size:19px}.remoteTouchFader{display:grid;grid-template-rows:auto minmax(48px,1fr) auto;place-items:center;gap:3px;padding:5px;color:#aab6c6;font-size:11px;text-align:center}.remoteTouchFader input{width:100%;min-height:48px;padding:0}.remoteTouchColor{display:grid;grid-template-rows:auto minmax(48px,1fr);place-items:stretch;gap:4px;padding:5px;color:#aab6c6;font-size:11px;text-align:center}.remoteTouchColor input{width:100%;height:100%;min-width:48px;min-height:48px;padding:3px}
    @media (max-width:640px){main{padding:10px}.grid,.triple,.summary,.liveGrid,.videoDeckControls,.effectDeckControls,.loopGrid,.sceneToolbar{grid-template-columns:1fr}.deckSlider{grid-template-columns:58px minmax(0,1fr) 48px}.cuePadHeader{grid-template-columns:1fr auto}.cuePadGrid{grid-template-columns:repeat(2,1fr)}header{align-items:flex-start;gap:8px;flex-direction:column}}
  </style>
</head>
<body>
<main>
  <header><h1>Syndocal Remote</h1><span id="status" class="status bad">Disconnected</span></header>
  <section class="summary">
    <div class="tile"><span class="muted">Fixtures</span><strong id="fixtureCount">0</strong></div>
    <div class="tile"><span class="muted">Cues</span><strong id="cueCount">0</strong></div>
    <div class="tile"><span class="muted">Layers</span><strong id="layerCount">0</strong></div>
    <div class="tile"><span class="muted">Effects</span><strong id="effectCount">0</strong></div>
    <div class="tile"><span class="muted">BPM</span><strong id="bpmReadout">120.0</strong></div>
  </section>
  <section>
    <h2>Live Desk</h2>
    <div class="liveGrid">
      <div class="liveTile"><span class="muted">Active cue</span><strong id="remoteActiveCue">None</strong></div>
      <div class="liveTile"><span class="muted">Next cue</span><strong id="remoteNextCue">None</strong></div>
      <div class="liveTile"><span class="muted">Timeline</span><strong id="remoteTimelineState">Stopped</strong></div>
      <div class="liveTile"><span class="muted">Guard</span><strong id="remoteGuardState">Live</strong></div>
      <div class="liveTile"><span class="muted">Realtime</span><strong id="remoteRtState">RT WARMUP</strong><small id="remoteRtDetail">Waiting for samples</small></div>
      <div class="liveTile"><span class="muted">DMX</span><strong id="remoteDmxState">DMX OFF</strong><small id="remoteDmxDetail">No output routes</small></div>
      <div class="liveTile"><span class="muted">Latency</span><strong id="remoteLatencyState">LAT OK</strong><small id="remoteLatencyDetail">cmd/dmx p99</small></div>
    </div>
    <div id="remoteDmxRoutes" class="remoteDmxRoutes"></div>
    <div class="triple">
      <button onclick='send({type:"triggerPreviousCue"})'>Back</button>
      <button class="primary" onclick='send({type:"triggerNextCue"})'>GO</button>
      <button onclick='send({type:"tapBpm"})'>Tap</button>
      <button onclick='send({type:"resetTelemetry"})'>Reset RT</button>
      <button onclick='setTimelinePlaying(true)'>Play TL</button>
      <button onclick='setTimelinePlaying(false)'>Pause TL</button>
      <button onclick='send({type:"blackout",enabled:true})'>DMX BO</button>
      <button onclick='send({type:"blackout",enabled:false})'>DMX Clear</button>
      <button onclick='send({type:"videoBlackout",enabled:true})'>Video BO</button>
      <button onclick='send({type:"videoBlackout",enabled:false})'>Video Clear</button>
      <button onclick='setAllBlackout(true)'>All BO</button>
      <button onclick='setAllBlackout(false)'>All Clear</button>
    </div>
  </section>
  <section id="remoteTouchPanel">
    <div class="remoteTouchHeader"><h2>Touch Surface</h2><span id="remoteTouchPageLabel" class="small">Default Desk</span></div>
    <div id="remoteTouchPages" class="remoteTouchPages"></div>
    <div id="remoteTouchSurface" class="remoteTouchSurface" aria-label="Composed Touch surface"></div>
  </section>
  <section class="remoteStagePanel">
    <h2>Stage</h2>
    <div class="grid">
      <label>Stage Group<select id="remoteStageGroupFilter" onchange="renderRemoteStage(latestSnapshot||{})"><option value="">All fixtures</option></select></label>
      <button onclick='document.getElementById("remoteStageGroupFilter").value="";renderRemoteStage(latestSnapshot||{})'>Show All</button>
    </div>
    <svg id="remoteStage" class="remoteStageSurface" viewBox="-10 -10 20 20" role="img" aria-label="Remote 2D stage overview"></svg>
    <div id="remoteStageReadout" class="remoteStageReadout"></div>
    <div id="remoteStageSelection" class="remoteStageSelection"></div>
    <div class="remoteStageLegend"><span><b class="fixture"></b>Fixtures</span><span><b class="output"></b>Projectors</span><span><b class="object"></b>Stage objects</span></div>
  </section>
  <section>
    <h2>Fader</h2>
    <div class="triple">
      <label>Fixture<select id="fixtureId" onchange="applyAttributeOptions()"><option value="1">1</option></select></label>
      <label>Group<select id="groupId" onchange="applyAttributeOptions()"><option value="">Fixture only</option></select></label>
      <label>Attribute<select id="attribute"><option>Dimmer</option></select></label>
    </div>
    <div id="remoteTargetInfo" class="remoteTargetInfo"><strong>Fixture target</strong><span class="small">Select a fixture or group</span></div>
    <div class="inline"><span class="muted">Value</span><strong id="attributeValueLabel">0</strong></div>
    <input id="attributeValue" type="range" min="0" max="1" step="0.001" value="0" oninput="sendAttribute()">
    <div id="remoteVisualDesk" class="visualDesk"></div>
    <div id="fixtureFaderBank" class="faderBank"></div>
    <div class="grid">
      <button onclick='setSelectedTargetHighlight(true)'>Highlight</button>
      <button onclick='setSelectedTargetHighlight(false)'>Clear Highlight</button>
      <button onclick='setSelectedTargetSolo(true)'>Solo</button>
      <button onclick='setSelectedTargetSolo(false)'>Clear Solo</button>
      <button onclick='setSelectedTargetPark(true)'>Park</button>
      <button onclick='setSelectedTargetPark(false)'>Clear Park</button>
    </div>
    <div class="grid">
      <button onclick='clearFixtureFlags("highlight")'>Clear All Highlight</button>
      <button onclick='clearFixtureFlags("solo")'>Clear All Solo</button>
      <button onclick='clearFixtureFlags("park")'>Clear All Park</button>
      <button onclick='clearFixtureFlags("all")'>Clear All Flags</button>
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
    <div class="sceneToolbar">
      <label>Scene Search<input id="sceneFilter" type="search" placeholder="label, cue id, light/video" oninput="setSceneFilter(this.value)" onkeydown="remoteSceneFilterKeyDown(event)"></label>
      <button class="primary" onclick="triggerFirstFilteredScene()">GO First</button>
      <button onclick="clearSceneFilter()">Clear</button>
    </div>
    <div id="sceneSelector" class="sceneSelectorGrid"></div>
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
      <label>Link phase<input id="linkPhase" type="number" min="0" max="0.999" step="0.001" value="0"></label>
      <button onclick='send({type:"syncAbletonLinkClock",bpm:numberValue("bpm"),beat_phase:numberValue("linkPhase")})'>Sync Link</button>
    </div>
    <button onclick='send({type:"tapBpm"})'>Tap</button>
    <div id="clockInfo" class="small"></div>
  </section>
  <section>
    <h2>Effects</h2>
    <div id="effectList" class="list"></div>
  </section>
  <section>
    <h2>Node Graphs</h2>
    <div id="nodeGraphList" class="list"></div>
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
      <label>Layer Fade ms<input id="videoLayerFadeMs" type="number" min="0" step="10" value="1000"></label>
      <button onclick='fadeSelectedLayer(0)'>Fade Out</button>
      <button onclick='fadeSelectedLayer(1)'>Fade In</button>
    </div>
    <div class="grid">
      <label>Seek ms<input id="videoSeekMs" type="number" min="0" step="1" value="0"></label>
      <button onclick='seekSelectedLayer(numberValue("videoSeekMs"))'>Seek</button>
    </div>
    <div class="grid">
      <button onclick='addSelectedLayerCuePoint()'>Add Cue Point</button>
      <button onclick='jumpSelectedLayerCuePoint(0)'>Jump First Cue</button>
      <button onclick='jumpSelectedLayerRelativeCuePoint(-1)'>Prev Cue</button>
      <button onclick='jumpSelectedLayerRelativeCuePoint(1)'>Next Cue</button>
    </div>
    <div class="grid">
      <button onclick='send({type:"videoBlackout",enabled:true})'>Video Blackout</button>
      <button onclick='send({type:"videoBlackout",enabled:false})'>Video Clear</button>
    </div>
    <div class="inline"><button onclick='requestVideoRuntimeStatus()'>Refresh Backends</button><span id="videoRuntimeSummary" class="small">Backend status not loaded</span></div>
    <div id="videoRuntimeBackends" class="liveGrid"></div>
    <div class="inline"><button onclick='requestExternalVideoIoPlans()'>Refresh I/O Plans</button><span id="externalVideoIoPlanSummary" class="small">I/O plans not loaded</span></div>
    <div id="externalVideoIoPlanRoutes" class="liveGrid"></div>
    <div class="inline"><button onclick='requestExternalVideoTransportStatus()'>Refresh I/O Routes</button><button onclick='syncExternalVideoTransportsRemote()'>Sync Routes</button><span id="externalVideoTransportSummary" class="small">Transport routes not loaded</span><span id="externalVideoTransportSyncSummary" class="small">Routes not synced</span></div>
    <div id="externalVideoTransportRoutes" class="liveGrid"></div>
    <div id="externalVideoTransportSyncEvents" class="liveGrid"></div>
    <div id="layerList" class="list"></div>
  </section>
  <section>
    <h2>Video Outputs</h2>
    <div class="inline"><button onclick='requestVideoOutputRenderPlans()'>Refresh Render Plans</button><span id="videoOutputRenderPlanSummary" class="small">Render plans not loaded</span></div>
    <div id="videoOutputRenderPlans" class="liveGrid"></div>
    <label>Fade ms<input id="videoOutputFadeMs" type="number" min="0" step="10" value="1000"></label>
    <div id="videoOutputList" class="list"></div>
  </section>
  <section>
    <h2>Timeline</h2>
    <div id="timelineInfo" class="small"></div>
    <div class="bar"><span id="timelineProgress"></span></div>
    <div class="triple">
      <button class="primary" onclick='setTimelinePlaying(true)'>Play</button>
      <button onclick='setTimelinePlaying(false)'>Pause</button>
      <button onclick='seekTimelineRemote(0)'>Restart</button>
      <button onclick='seekTimelineAdjacentBeat(-1)'>Prev Beat</button>
      <button onclick='seekTimelineAdjacentBeat(1)'>Next Beat</button>
    </div>
    <div class="grid">
      <label>Seek ms<input id="timelineSeekMs" type="number" min="0" step="1" value="0"></label>
      <button onclick='seekTimelineRemote(numberValue("timelineSeekMs"))'>Seek</button>
    </div>
    <div class="grid">
      <label>Timecode<input id="timelineTimecodeMs" type="text" value="0" placeholder="ms or 01:02:03.500"></label>
      <label>Source<select id="timelineTimecodeSource"><option value="ltc">LTC</option><option value="mtc">MTC</option><option value="ableton_link">Link</option></select></label>
      <button onclick='syncTimelineTimecodeRemote()'>Sync Timecode</button>
    </div>
    <div id="timelineBeatInfo" class="small"></div>
    <div id="timelineEvents" class="list"></div>
  </section>
  <section>
    <h2>Master</h2>
    <div class="grid">
      <button onclick='send({type:"blackout",enabled:true})'>Blackout</button>
      <button onclick='send({type:"blackout",enabled:false})'>Clear</button>
      <button onclick='setAllBlackout(true)'>All Blackout</button>
      <button onclick='setAllBlackout(false)'>All Clear</button>
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
let latestVideoRuntimeStatus={backends:[]};
let latestVideoOutputRenderPlans=[];
let latestExternalVideoIoPlans={inputs:[],outputs:[]};
let latestExternalVideoTransportStatus={active_routes:[],active_count:0};
let latestExternalVideoTransportSync={report:null,events:[]};
let snapshotTimer=null;
let snapshotPending=false;
let cuePadBank=0;
let cuePadFollowActive=true;
let sceneFilterText="";
let remoteStageSelectedOutputId=null;
let remoteColorAutoWhite=false;
let remoteDimmerBumpRestore=null;
let remoteTouchPageId=null;
const remotePositionFavoritesStorageKey="syndocal.remote.positionFavorites.v1";
const remotePositionFavoriteTolerance=512;
let remotePositionFavorites=loadRemotePositionFavorites();
const cuePadSize=10;
const remoteColorPresets=[
  {label:"White",color:"#ffffff"},
  {label:"Black",color:"#000000"},
  {label:"Warm",color:"#ffb35c"},
  {label:"Tungsten",color:"#ffc58f"},
  {label:"Neutral",color:"#fff1df"},
  {label:"Cool",color:"#79c8ff"},
  {label:"Daylight",color:"#d8ecff"}
];
const remoteColorExtraDefinitions=[
  {key:"white",label:"White",short:"W",candidates:["ColorWhite","White","ColorAdd_W"]},
  {key:"amber",label:"Amber",short:"A",candidates:["ColorAmber","Amber","ColorAdd_A"]},
  {key:"uv",label:"UV",short:"UV",candidates:["ColorUV","ColorUv","UV","Uv","Ultraviolet","ColorAdd_UV"]}
];
const statusEl=document.getElementById("status");
const logEl=document.getElementById("log");
function numberValue(id){return Number(document.getElementById(id).value)}
function selectedNumber(id){const raw=document.getElementById(id).value;if(raw==="")return null;const value=Number(raw);return Number.isFinite(value)?value:null}
function text(id,value){document.getElementById(id).textContent=value}
function escapeHtml(value){return String(value).replace(/[&<>"']/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#39;"}[c]))}
function remoteJsString(value){return escapeHtml(JSON.stringify(String(value)))}
function setStatus(ok){statusEl.textContent=ok?"Connected":"Disconnected";statusEl.className=ok?"status ok":"status bad"}
function connect(){
  if(snapshotTimer)clearInterval(snapshotTimer);
  const token=new URLSearchParams(location.search).get("token")||"";
  const wsScheme=location.protocol==="https:"?"wss":"ws";
  ws=new WebSocket(`${wsScheme}://${location.host}/ws?token=${encodeURIComponent(token)}`);
  ws.onopen=()=>{setStatus(true);requestSnapshot();requestVideoRuntimeStatus();requestVideoOutputRenderPlans();requestExternalVideoIoPlans();requestExternalVideoTransportStatus();snapshotTimer=setInterval(requestSnapshot,1000)};
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
function requestVideoRuntimeStatus(){send({type:"getVideoRuntimeStatus"},false)}
function requestVideoOutputRenderPlans(){send({type:"getVideoOutputRenderPlans"},false)}
function requestExternalVideoIoPlans(){send({type:"getExternalVideoIoPlans"},false)}
function requestExternalVideoTransportStatus(){send({type:"getExternalVideoTransportStatus"},false)}
function handleMessage(messageText){
  logEl.textContent=messageText;
  try{
    const msg=JSON.parse(messageText);
    if(msg.type==="snapshot"){applySnapshot(msg.snapshot);requestVideoOutputRenderPlans();}
    else if(msg.type==="videoRuntimeStatus")renderVideoRuntimeStatus(msg.video_runtime_status||msg.videoRuntimeStatus||{backends:[]});
    else if(msg.type==="videoOutputRenderPlans")renderVideoOutputRenderPlans(msg.video_output_render_plans||msg.videoOutputRenderPlans||[]);
    else if(msg.type==="externalVideoIoPlans")renderExternalVideoIoPlans(msg.external_video_io_plans||msg.externalVideoIoPlans||{inputs:[],outputs:[]});
    else if(msg.type==="externalVideoTransportStatus")renderExternalVideoTransportStatus(msg.external_video_transport_status||msg.externalVideoTransportStatus||{active_routes:[],active_count:0});
    else if(msg.type==="externalVideoTransportSync"){renderExternalVideoTransportSync(msg.external_video_transport_sync||msg.externalVideoTransportSync||{report:null,events:[]});requestExternalVideoIoPlans();requestExternalVideoTransportStatus();}
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
function remoteNormalizeGroup(value){
  return String(value||"").split("/").map(segment=>segment.trim()).filter(Boolean).join("/");
}
function remoteGroupHierarchyIds(value){
  const group=remoteNormalizeGroup(value);
  if(!group)return [];
  const segments=group.split("/");
  return segments.map((_,index)=>segments.slice(0,index+1).join("/"));
}
function remoteGroupMatches(fixtureGroupId,requestedGroupId){
  const fixtureGroup=remoteNormalizeGroup(fixtureGroupId);
  const requested=remoteNormalizeGroup(requestedGroupId);
  return Boolean(fixtureGroup&&requested&&(fixtureGroup===requested||fixtureGroup.startsWith(`${requested}/`)));
}
function remoteFixtureMatchesGroup(fixture,groupId){
  return Boolean(fixture&&groupId&&(fixture.group_ids||[]).some(candidate=>remoteGroupMatches(candidate,groupId)));
}
function remoteGroupOptions(fixtures){
  const counts=new Map();
  for(const fixture of fixtures||[]){
    const groups=new Set((fixture.group_ids||[]).flatMap(remoteGroupHierarchyIds));
    for(const group of groups)counts.set(group,(counts.get(group)||0)+1);
  }
  return [...counts.entries()].sort(([a],[b])=>a.localeCompare(b)).map(([id,count])=>({id,label:`${id} (${count})`,count}));
}
function fillGroupSelect(groups){
  const el=document.getElementById("groupId");
  const current=el.value;
  el.innerHTML="";
  const fixtureOnly=document.createElement("option");
  fixtureOnly.value="";
  fixtureOnly.textContent="Fixture only";
  el.appendChild(fixtureOnly);
  for(const group of groups){
    const opt=document.createElement("option");
    opt.value=String(group.id);
    opt.textContent=group.label;
    el.appendChild(opt);
  }
  el.disabled=false;
  el.value=[...el.options].some(option=>option.value===current)?current:"";
}
function fillRemoteStageGroupFilter(groups){
  const el=document.getElementById("remoteStageGroupFilter");
  if(!el)return;
  const current=el.value;
  el.innerHTML="";
  const all=document.createElement("option");
  all.value="";
  all.textContent="All fixtures";
  el.appendChild(all);
  for(const group of groups){
    const opt=document.createElement("option");
    opt.value=String(group.id);
    opt.textContent=group.label;
    el.appendChild(opt);
  }
  el.value=[...el.options].some(option=>option.value===current)?current:"";
}
function selectedGroupId(){
  const value=remoteNormalizeGroup(document.getElementById("groupId").value);
  return value||null;
}
function selectedRemoteStageGroupId(){
  const el=document.getElementById("remoteStageGroupFilter");
  if(!el)return null;
  const value=remoteNormalizeGroup(el.value);
  return value||null;
}
function remoteGroupFixtures(groupId){
  return (latestSnapshot&&latestSnapshot.fixtures||[]).filter(fixture=>remoteFixtureMatchesGroup(fixture,groupId));
}
function remoteControlFixture(){
  const groupId=selectedGroupId();
  const selected=remoteFixtureById(selectedNumber("fixtureId"));
  if(!groupId)return selected;
  if(remoteFixtureMatchesGroup(selected,groupId))return selected;
  return remoteGroupFixtures(groupId)[0]||null;
}
function remoteSelectedGroupSubmaster(groupId){
  const submasters=latestSnapshot&&latestSnapshot.submasters||[];
  const exact=submasters.find(submaster=>remoteNormalizeGroup(submaster.group_id)===groupId);
  return exact||submasters.find(submaster=>remoteGroupMatches(submaster.group_id,groupId))||null;
}
function remoteSubmasterInput(groupId,submaster){
  if(!groupId)return "";
  const level=Math.max(0,Math.min(1,Number((submaster&&submaster.level)??1)));
  const pct=Math.round(level*100);
  return `<label class="remoteTargetSubmaster"><span class="small">Submaster</span><input type="range" min="0" max="1" step="0.01" value="${level.toFixed(2)}" data-group="${escapeHtml(groupId)}" oninput="setGroupSubmasterFromInput(this)"><strong>${pct}%</strong></label>`;
}
function remoteGroupStateText(fixtures){
  const flags=[
    ["highlighted","Highlight"],
    ["soloed","Solo"],
    ["parked","Park"]
  ].map(([field,label])=>{
    const count=fixtures.filter(fixture=>fixture&&fixture[field]===true).length;
    if(!count)return "";
    return count===fixtures.length?label:`${label} ${count}/${fixtures.length}`;
  }).filter(Boolean);
  return flags.join(" / ");
}
function renderRemoteTargetInfo(fixture){
  const el=document.getElementById("remoteTargetInfo");
  if(!el)return;
  const groupId=selectedGroupId();
  if(groupId){
    const fixtures=remoteGroupFixtures(groupId);
    const representative=fixture||fixtures[0]||null;
    const names=fixtures.slice(0,3).map(candidate=>candidate.label).join(", ");
    const more=fixtures.length>3?` +${fixtures.length-3}`:"";
    const state=remoteGroupStateText(fixtures);
    const submaster=remoteSelectedGroupSubmaster(groupId);
    el.innerHTML=`<strong>Group: ${escapeHtml(groupId)}</strong><span class="small">${fixtures.length} fixture(s)${representative?` / Rep ${escapeHtml(representative.label)}`:""}${names?` / ${escapeHtml(names)}${more}`:""}${state?` / ${escapeHtml(state)}`:""}</span>${remoteSubmasterInput(groupId,submaster)}`;
    return;
  }
  if(fixture){
    const states=[fixture.highlighted?"Highlight":"",fixture.soloed?"Solo":"",fixture.parked?"Park":""].filter(Boolean).join(" / ");
    el.innerHTML=`<strong>Fixture: ${escapeHtml(fixture.label)}</strong><span class="small">U${fixture.universe} / ${fixture.address} / ${escapeHtml(fixture.profile_name)}${states?` / ${states}`:""}</span>`;
    return;
  }
  el.innerHTML=`<strong>No target</strong><span class="small">Patch or select a fixture first</span>`;
}
function applySnapshot(snapshot){
  latestSnapshot=snapshot;
  const fixtures=snapshot.fixtures||[];
  const cues=snapshot.cues||[];
  const effects=snapshot.effects||[];
  const nodeGraphs=snapshot.node_graphs||[];
  const video=snapshot.video||{layers:[],master_opacity:1,blackout:false};
  const layers=video.layers||[];
  fillSelect("fixtureId",fixtures,f=>`${f.id}: ${f.label}`,"No fixtures");
  const groupOptions=remoteGroupOptions(fixtures);
  fillGroupSelect(groupOptions);
  fillRemoteStageGroupFilter(groupOptions);
  fillSelect("cueId",cues,c=>`${c.id}: ${c.label}`,"No cues");
  fillSelect("layerId",layers,l=>`${l.id}: ${l.label}`,"No layers");
  applyAttributeOptions();
  text("fixtureCount",fixtures.length);
  text("cueCount",cues.length);
  text("layerCount",layers.length);
  text("effectCount",effects.length);
  text("bpmReadout",((snapshot.clock&&snapshot.clock.bpm)||120).toFixed(1));
  document.getElementById("bpm").value=((snapshot.clock&&snapshot.clock.bpm)||120).toFixed(1);
  document.getElementById("lightingMaster").value=snapshot.lighting_master??1;
  document.getElementById("videoMaster").value=video.master_opacity??1;
  renderFixtureList(fixtures);
  renderCuePads(cues,snapshot.active_cue_id);
  renderCueButtons(cues,snapshot.active_cue_id);
  renderSceneSelector(cues,snapshot.active_cue_id);
  renderEffectList(effects);
  renderNodeGraphList(nodeGraphs);
  renderLayerList(layers);
  renderVideoOutputList(video.outputs||[],video.compositions||[]);
  renderSubmasters(snapshot.submasters||[]);
  renderTimeline(snapshot.timeline||{events:[],position_ms:0,duration_ms:0,playing:false});
  renderStatus(snapshot,video);
  renderRemoteDmxRoutes(snapshot,snapshot.telemetry||{});
  renderLiveDesk(snapshot,cues,video,snapshot.timeline||{events:[],position_ms:0,duration_ms:0,playing:false});
  renderRemoteTouchSurface(snapshot);
  renderRemoteStage(snapshot);
}
function applyAttributeOptions(){
  const fixture=remoteControlFixture();
  renderRemoteTargetInfo(fixture);
  const attrs=fixture?(fixture.controls||[]).map(c=>({id:c.attribute,label:c.attribute})):[];
  fillSelect("attribute",attrs,a=>a.label,"No attributes");
  syncSelectedAttributeValue();
  renderRemoteVisualControls(fixture);
  renderFixtureFaderBank(fixture);
}
function normalizedAttributeValue(fixture,attribute){
  const value=(fixture&&fixture.attribute_values||[]).find(v=>v.attribute===attribute);
  return value?Math.max(0,Math.min(1,value.value/65535)):0;
}
function remoteControlAttribute(fixture,candidates){
  if(!fixture)return null;
  const normalizedCandidates=candidates.map(name=>String(name).toLowerCase().replace(/[^a-z0-9]/g,""));
  const controls=fixture.controls||[];
  const exact=controls.find(control=>normalizedCandidates.includes(String(control.attribute).toLowerCase().replace(/[^a-z0-9]/g,"")));
  if(exact)return exact.attribute;
  const fuzzy=controls.find(control=>{
    const attr=String(control.attribute).toLowerCase().replace(/[^a-z0-9]/g,"");
    return normalizedCandidates.some(candidate=>attr===candidate||attr.endsWith(candidate));
  });
  return fuzzy?fuzzy.attribute:null;
}
function remoteDmxValue(fixture,attribute){
  const value=(fixture&&fixture.attribute_values||[]).find(v=>v.attribute===attribute);
  return value?Math.max(0,Math.min(65535,Math.round(Number(value.value)||0))):0;
}
function remoteDmxClamp(value){return Math.round(Math.max(0,Math.min(65535,Number.isFinite(Number(value))?Number(value):0)))}
function remoteDefaultPositionFavorites(){
  return [
    {id:"center",label:"Center",pan:32768,tilt:32768},
    {id:"up",label:"Up",pan:32768,tilt:65535},
    {id:"down",label:"Down",pan:32768,tilt:0},
    {id:"left",label:"Left",pan:0,tilt:32768},
    {id:"right",label:"Right",pan:65535,tilt:32768}
  ];
}
function remotePositionFavoriteFromUnknown(candidate,index){
  if(!candidate||typeof candidate!=="object")return null;
  const pan=remoteDmxClamp(candidate.pan);
  const tilt=remoteDmxClamp(candidate.tilt);
  const label=String(candidate.label||`P${index+1}`).trim().slice(0,16)||`P${index+1}`;
  const id=String(candidate.id||`${label}-${pan}-${tilt}`).trim().slice(0,48)||`${index}-${pan}-${tilt}`;
  return {id,label,pan,tilt};
}
function loadRemotePositionFavorites(){
  try{
    const raw=window.localStorage&&window.localStorage.getItem(remotePositionFavoritesStorageKey);
    const parsed=raw?JSON.parse(raw):null;
    if(Array.isArray(parsed)){
      const favorites=parsed.map(remotePositionFavoriteFromUnknown).filter(Boolean).slice(0,18);
      if(favorites.length)return favorites;
    }
  }catch(_e){}
  return remoteDefaultPositionFavorites();
}
function saveRemotePositionFavorites(){
  try{
    if(window.localStorage)window.localStorage.setItem(remotePositionFavoritesStorageKey,JSON.stringify(remotePositionFavorites.slice(0,18)));
  }catch(_e){}
}
function remotePositionFavoriteMatches(position,favorite){
  return Boolean(position&&favorite&&Math.abs(remoteDmxClamp(position.panValue)-remoteDmxClamp(favorite.pan))<=remotePositionFavoriteTolerance&&Math.abs(remoteDmxClamp(position.tiltValue)-remoteDmxClamp(favorite.tilt))<=remotePositionFavoriteTolerance);
}
function remoteNormalizeLimitRange(min,max){
  const a=remoteDmxClamp(min);
  const b=remoteDmxClamp(max);
  return {min:Math.min(a,b),max:Math.max(a,b)};
}
function remoteFixtureLimits(fixture){
  const limits=(fixture&&fixture.limits)||{};
  return {
    dimmer_min:remoteDmxClamp(limits.dimmer_min??0),
    dimmer_max:remoteDmxClamp(limits.dimmer_max??65535),
    pan_min:remoteDmxClamp(limits.pan_min??0),
    pan_max:remoteDmxClamp(limits.pan_max??65535),
    tilt_min:remoteDmxClamp(limits.tilt_min??0),
    tilt_max:remoteDmxClamp(limits.tilt_max??65535),
    invert_pan:limits.invert_pan===true,
    invert_tilt:limits.invert_tilt===true,
    swap_pan_tilt:limits.swap_pan_tilt===true
  };
}
function remoteSourceValueForAxisLimit(desiredValue,min,max,invert){
  const range=remoteNormalizeLimitRange(min,max);
  const clamped=remoteDmxClamp(Math.max(range.min,Math.min(range.max,Number(desiredValue)||0)));
  return invert?range.min+range.max-clamped:clamped;
}
function remoteApplyAxisLimit(value,min,max,invert){
  return remoteSourceValueForAxisLimit(value,min,max,invert);
}
function remoteEffectivePanTiltValues(fixture,panValue,tiltValue){
  const limits=remoteFixtureLimits(fixture);
  const panSource=limits.swap_pan_tilt?tiltValue:panValue;
  const tiltSource=limits.swap_pan_tilt?panValue:tiltValue;
  return {
    pan:remoteApplyAxisLimit(panSource,limits.pan_min,limits.pan_max,limits.invert_pan),
    tilt:remoteApplyAxisLimit(tiltSource,limits.tilt_min,limits.tilt_max,limits.invert_tilt)
  };
}
function remoteSourcePanTiltValues(fixture,panValue,tiltValue){
  const limits=remoteFixtureLimits(fixture);
  const panSource=remoteSourceValueForAxisLimit(panValue,limits.pan_min,limits.pan_max,limits.invert_pan);
  const tiltSource=remoteSourceValueForAxisLimit(tiltValue,limits.tilt_min,limits.tilt_max,limits.invert_tilt);
  return limits.swap_pan_tilt?{pan:tiltSource,tilt:panSource}:{pan:panSource,tilt:tiltSource};
}
function remotePanTiltLimitStyle(fixture){
  const limits=remoteFixtureLimits(fixture);
  const pan=remoteNormalizeLimitRange(limits.pan_min,limits.pan_max);
  const tilt=remoteNormalizeLimitRange(limits.tilt_min,limits.tilt_max);
  const width=Math.max(1.5,((pan.max-pan.min)/65535)*100);
  const height=Math.max(1.5,((tilt.max-tilt.min)/65535)*100);
  return `left:${(pan.min/65535)*100}%;width:${width}%;top:${((65535-tilt.max)/65535)*100}%;height:${height}%`;
}
function remotePanTiltTargetValues(fixture,panRatio,tiltRatio){
  const limits=remoteFixtureLimits(fixture);
  const pan=remoteNormalizeLimitRange(limits.pan_min,limits.pan_max);
  const tilt=remoteNormalizeLimitRange(limits.tilt_min,limits.tilt_max);
  const safePan=Math.max(0,Math.min(1,Number(panRatio)||0));
  const safeTilt=Math.max(0,Math.min(1,Number(tiltRatio)||0));
  return {
    pan:Math.round(pan.min+(pan.max-pan.min)*safePan),
    tilt:Math.round(tilt.min+(tilt.max-tilt.min)*safeTilt)
  };
}
function remoteDimmerWithinLimits(fixture,value){
  const limits=remoteFixtureLimits(fixture);
  const range=remoteNormalizeLimitRange(limits.dimmer_min,limits.dimmer_max);
  return remoteDmxClamp(Math.max(range.min,Math.min(range.max,Number(value)||0)));
}
function remoteDimmerTargetValue(fixture,ratio){
  const limits=remoteFixtureLimits(fixture);
  const range=remoteNormalizeLimitRange(limits.dimmer_min,limits.dimmer_max);
  const safeRatio=Math.max(0,Math.min(1,Number(ratio)||0));
  return Math.round(range.min+(range.max-range.min)*safeRatio);
}
function remoteDmxPercent(value){return `${Math.round((Math.max(0,Math.min(65535,Number(value)||0))/65535)*100)}%`}
function remoteHexByte(value){return Math.round(Math.max(0,Math.min(65535,Number(value)||0))/257).toString(16).padStart(2,"0")}
function remoteRgbHex(red,green,blue){return `#${remoteHexByte(red)}${remoteHexByte(green)}${remoteHexByte(blue)}`}
function remoteNormalizeColorHex(value){
  const hex=String(value||"").trim().toLowerCase();
  return /^#[0-9a-f]{6}$/.test(hex)?hex:"";
}
function remoteColorMatches(current,candidate){
  return Boolean(remoteNormalizeColorHex(current)&&remoteNormalizeColorHex(current)===remoteNormalizeColorHex(candidate));
}
function remoteColorDisplayHex(color){
  const white=(color.extras||[]).find(extra=>extra.key==="white");
  const whiteValue=white?white.value:0;
  return remoteRgbHex(color.redValue+whiteValue,color.greenValue+whiteValue,color.blueValue+whiteValue);
}
function sendFixtureDmxValue(fixture,attribute,dmxValue){
  if(!fixture||!attribute)return;
  const safe=Math.max(0,Math.min(65535,Math.round(Number(dmxValue)||0)));
  const normalized=safe/65535;
  const group_id=selectedGroupId();
  if((group_id||selectedNumber("fixtureId")===fixture.id)&&document.getElementById("attribute").value===attribute){
    document.getElementById("attributeValue").value=normalized;
    text("attributeValueLabel",String(safe));
  }
  if(group_id){
    send({type:"setGroupAttribute",group_id,attribute,value:safe},false);
  }else{
    send({type:"setAttribute",fixture_id:fixture.id,attribute,value:normalized},false);
  }
  queueSnapshot(160);
}
function remoteDimmerControl(fixture){
  const attribute=remoteControlAttribute(fixture,["Dimmer","Intensity","MasterIntensity"]);
  if(!attribute)return null;
  return {attribute,value:remoteDimmerWithinLimits(fixture,remoteDmxValue(fixture,attribute)),limits:remoteFixtureLimits(fixture)};
}
function remoteControlCategory(control){
  const normalized=String(control&&control.attribute||"").toLowerCase().replace(/[^a-z0-9]/g,"");
  if(/gobo|animationwheel/.test(normalized))return "Gobo";
  if(/focus|focal/.test(normalized))return "Focus";
  if(/zoom|iris|prism|frost|beam|shutter|strobe|blade|framing|wash|spot/.test(normalized))return "Beam";
  return null;
}
function remoteIsColorControl(control){
  const normalized=String(control&&control.attribute||"").toLowerCase().replace(/[^a-z0-9]/g,"");
  return /color|colour|red|green|blue|cyan|magenta|yellow|amber|white|warmwhite|coldwhite|uv|hue|saturation|cto|ctc|ctb/.test(normalized);
}
function remoteSortedFunctions(control){
  return [...(control&&control.functions||[])]
    .filter(fn=>Number.isFinite(Number(fn.dmx_from))&&Number.isFinite(Number(fn.dmx_to)))
    .sort((a,b)=>Number(a.dmx_from)-Number(b.dmx_from));
}
function remoteFunctionValue(fn){
  return remoteDmxClamp((Number(fn.dmx_from)+Number(fn.dmx_to))/2);
}
function remoteFunctionLabel(fn){
  return String(fn.wheel_slot_name||fn.name||fn.attribute||"Function").trim();
}
function remoteNormalizedFunctionText(control,fn){
  return `${fn&&fn.name||""} ${fn&&fn.parent_function||""} ${fn&&fn.attribute||""} ${fn&&fn.wheel_slot||""} ${fn&&fn.wheel_slot_name||""} ${control&&control.channel_name||""}`.toLowerCase().replace(/[^a-z0-9]+/g," ");
}
function remoteNamedFunctionColor(text){
  const value=String(text||"").toLowerCase();
  const pairs=[
    [/\b(open|clear|white|coldwhite|warmwhite)\b/,"#ffffff"],
    [/\b(red)\b/,"#ff1f1f"],
    [/\b(green)\b/,"#00e83a"],
    [/\b(blue)\b/,"#244dff"],
    [/\b(cyan|aqua)\b/,"#00e5ff"],
    [/\b(magenta|pink)\b/,"#ff27d7"],
    [/\b(yellow)\b/,"#ffe600"],
    [/\b(amber|orange)\b/,"#ff9f1a"],
    [/\b(purple|violet|uv)\b/,"#9f66ff"]
  ];
  const match=pairs.find(([pattern])=>pattern.test(value));
  return match?match[1]:null;
}
function remoteFunctionSwatchColor(control,fn){
  const slot=String(fn&&fn.wheel_slot_color||"").trim();
  if(/^#[0-9a-fA-F]{6}$/.test(slot))return slot.toLowerCase();
  return remoteNamedFunctionColor(remoteNormalizedFunctionText(control,fn));
}
function remoteFunctionRangeLabel(fn){
  return `${remoteDmxPercent(fn.dmx_from)}-${remoteDmxPercent(fn.dmx_to)}`;
}
function remoteFunctionActive(fn,value){
  const safe=remoteDmxClamp(value);
  return safe>=remoteDmxClamp(fn.dmx_from)&&safe<=remoteDmxClamp(fn.dmx_to);
}
function remoteIsGoboControl(control){
  return remoteControlCategory(control)==="Gobo";
}
function remoteGoboSlotPattern(control,fn,index){
  const text=remoteNormalizedFunctionText(control,fn);
  if(/\b(open|clear|empty|none|white)\b/.test(text))return "open";
  if(/\b(spin|rotate|rotation|continuous|shake)\b/.test(text))return "spin";
  if(/\b(line|bar|stripe|slash)\b/.test(text))return "bars";
  if(/\b(dot|circle|bubble|spot)\b/.test(text))return "dots";
  if(/\b(ring|radial|iris)\b/.test(text))return "ring";
  return ["breakup","bars","dots","ring"][index%4];
}
function remoteGoboSlotDetail(fn){
  const media=String(fn&&fn.wheel_slot_media||"").split(/[\\/]/).filter(Boolean).pop();
  return media||remoteFunctionRangeLabel(fn);
}
function remoteOpticsRole(attribute){
  const normalized=String(attribute||"").toLowerCase().replace(/[^a-z0-9]/g,"");
  if(/zoom|beam|wash|spot/.test(normalized))return "Zoom";
  if(/iris/.test(normalized))return "Iris";
  if(/focus|focal/.test(normalized))return "Focus";
  if(/frost/.test(normalized))return "Frost";
  if(/prism/.test(normalized))return "Prism";
  if(/strobe/.test(normalized))return "Strobe";
  if(/shutter/.test(normalized))return "Shutter";
  return "Beam";
}
function remotePickFunctionValue(control,predicate){
  const fn=remoteSortedFunctions(control).find(candidate=>predicate(remoteNormalizedFunctionText(control,candidate)));
  return fn?remoteFunctionValue(fn):null;
}
function remoteRankedFunctionValue(control,rank){
  const functions=remoteSortedFunctions(control);
  if(!functions.length)return null;
  const index=rank==="middle"?Math.floor(functions.length/2):rank==="last"?functions.length-1:0;
  return remoteFunctionValue(functions[Math.max(0,Math.min(functions.length-1,index))]);
}
function remoteOpticsPresetButtons(row){
  const control=row.control;
  const role=String(row.role||"").toLowerCase();
  const openOrOff=()=>remotePickFunctionValue(control,text=>/\b(open|off|none|disable|disabled|clear|home)\b/.test(text));
  const narrow=()=>remotePickFunctionValue(control,text=>/\b(tight|narrow|small|min|minimum)\b/.test(text));
  const wide=()=>remotePickFunctionValue(control,text=>/\b(wide|large|max|maximum|open)\b/.test(text));
  const soft=()=>remotePickFunctionValue(control,text=>/\b(soft|frost|diffusion|diffuse|on|enable)\b/.test(text));
  if(role==="focus")return [
    {label:"Near",value:remoteRankedFunctionValue(control,"first")??0},
    {label:"Mid",value:remoteRankedFunctionValue(control,"middle")??32768},
    {label:"Far",value:remoteRankedFunctionValue(control,"last")??65535}
  ];
  if(role==="zoom"||role==="iris")return [
    {label:"Tight",value:narrow()??0},
    {label:"Mid",value:32768},
    {label:"Wide",value:wide()??65535}
  ];
  if(role==="frost")return [
    {label:"Clear",value:openOrOff()??0},
    {label:"Half",value:32768},
    {label:"Soft",value:soft()??65535}
  ];
  if(role==="shutter")return [
    {label:"Closed",value:0},
    {label:"Open",value:openOrOff()??65535},
    {label:"Full",value:65535}
  ];
  if(role==="strobe"||role==="prism")return [
    {label:"Off",value:openOrOff()??0},
    {label:"Mid",value:32768},
    {label:"On",value:65535}
  ];
  return [
    {label:"Min",value:0},
    {label:"Mid",value:32768},
    {label:"Max",value:65535}
  ];
}
function remoteOpticsPreviewStyle(row){
  const level=remoteDmxClamp(row.value)/65535;
  const role=String(row.role||"").toLowerCase();
  const isZoom=role==="zoom";
  const isIris=role==="iris";
  const isFocus=role==="focus";
  const isFrost=role==="frost";
  const isStrobe=role==="strobe";
  const isPrism=role==="prism";
  const size=isZoom||isIris?22+level*58:48;
  const blur=isFocus?Math.abs(level-.5)*7:isFrost?level*8:isZoom?(1-level)*1.5:.8;
  const opacity=isStrobe ? .35+level*.65 : .88;
  const ring=Math.max(18,size-10);
  const prismOpacity=isPrism ? .28+level*.55 : 0;
  return `--remote-optics-level:${level*100}%;--remote-optics-size:${size.toFixed(1)}px;--remote-optics-blur:${blur.toFixed(2)}px;--remote-optics-opacity:${opacity.toFixed(2)};--remote-optics-ring-size:${ring.toFixed(1)}px;--remote-optics-prism-rotation:${(level*120).toFixed(1)}deg;--remote-optics-prism-opacity:${prismOpacity.toFixed(2)}`;
}
function remoteOpticsRows(fixture){
  const rows=[];
  for(const control of fixture&&fixture.controls||[]){
    const category=remoteControlCategory(control);
    if(category!=="Beam"&&category!=="Focus")continue;
    const value=remoteDmxValue(fixture,control.attribute);
    const functions=remoteSortedFunctions(control);
    rows.push({category,control,value,functions,role:remoteOpticsRole(control.attribute),activeFunction:functions.find(fn=>remoteFunctionActive(fn,value))});
  }
  return rows.slice(0,8);
}
function remoteFunctionControlRows(fixture){
  const rows=[];
  for(const control of fixture&&fixture.controls||[]){
    const category=remoteControlCategory(control);
    if(!category)continue;
    if(category==="Gobo"||category==="Beam"||category==="Focus")continue;
    const functions=remoteSortedFunctions(control);
    if(!functions.length)continue;
    rows.push({category,control,functions,value:remoteDmxValue(fixture,control.attribute)});
  }
  return rows.slice(0,6);
}
function remoteColorWheelRows(fixture){
  const rows=[];
  for(const control of fixture&&fixture.controls||[]){
    if(!remoteIsColorControl(control))continue;
    const functions=remoteSortedFunctions(control).filter(fn=>remoteFunctionSwatchColor(control,fn)||String(fn.wheel_slot||fn.wheel_slot_name||"").trim());
    if(!functions.length)continue;
    rows.push({control,functions,value:remoteDmxValue(fixture,control.attribute)});
  }
  return rows.slice(0,4);
}
function remoteGoboWheelRows(fixture){
  const rows=[];
  for(const control of fixture&&fixture.controls||[]){
    if(!remoteIsGoboControl(control))continue;
    const functions=remoteSortedFunctions(control);
    if(!functions.length)continue;
    rows.push({control,functions,value:remoteDmxValue(fixture,control.attribute)});
  }
  return rows.slice(0,4);
}
function remotePositionControls(fixture){
  const pan=remoteControlAttribute(fixture,["Pan"]);
  const tilt=remoteControlAttribute(fixture,["Tilt"]);
  if(!pan||!tilt)return null;
  const rawPanValue=remoteDmxValue(fixture,pan);
  const rawTiltValue=remoteDmxValue(fixture,tilt);
  const effective=remoteEffectivePanTiltValues(fixture,rawPanValue,rawTiltValue);
  return {pan,tilt,panValue:effective.pan,tiltValue:effective.tilt,rawPanValue,rawTiltValue,limits:remoteFixtureLimits(fixture)};
}
function remoteColorControls(fixture){
  const red=remoteControlAttribute(fixture,["ColorRed","Red","ColorAdd_R"]);
  const green=remoteControlAttribute(fixture,["ColorGreen","Green","ColorAdd_G"]);
  const blue=remoteControlAttribute(fixture,["ColorBlue","Blue","ColorAdd_B"]);
  if(!red||!green||!blue)return null;
  const extras=remoteColorExtraDefinitions.map(definition=>{
    const attribute=remoteControlAttribute(fixture,definition.candidates);
    return attribute?{...definition,attribute,value:remoteDmxValue(fixture,attribute)}:null;
  }).filter(Boolean);
  return {red,green,blue,redValue:remoteDmxValue(fixture,red),greenValue:remoteDmxValue(fixture,green),blueValue:remoteDmxValue(fixture,blue),extras};
}
function renderRemoteVisualControls(fixture){
  const el=document.getElementById("remoteVisualDesk");
  if(!fixture){el.innerHTML=`<span class="small">No fixture selected</span>`;return}
  const dimmer=remoteDimmerControl(fixture);
  const position=remotePositionControls(fixture);
  const color=remoteColorControls(fixture);
  const colorWheelRows=remoteColorWheelRows(fixture);
  const goboWheelRows=remoteGoboWheelRows(fixture);
  const opticsRows=remoteOpticsRows(fixture);
  const functionRows=remoteFunctionControlRows(fixture);
  const panels=[];
  if(dimmer){
    const dimmerRange=remoteNormalizeLimitRange(dimmer.limits.dimmer_min,dimmer.limits.dimmer_max);
    panels.push(`<div class="visualCard">
      <div class="visualCardHeader"><strong>Dimmer</strong><span>${remoteDmxPercent(dimmer.value)} / ${escapeHtml(dimmer.attribute)}</span></div>
      <div class="remoteDimmerQuick">
        <button onclick="setRemoteDimmerTarget(${fixture.id},0)">Out</button>
        <button onclick="setRemoteDimmerTarget(${fixture.id},0.5)">Half</button>
        <button class="primary" onclick="setRemoteDimmerTarget(${fixture.id},1)">Full</button>
        <button class="momentary" onpointerdown="startRemoteDimmerBump(event,${fixture.id})" onpointerup="endRemoteDimmerBump(event,${fixture.id})" onpointercancel="endRemoteDimmerBump(event,${fixture.id})" onpointerleave="endRemoteDimmerBump(event,${fixture.id})" onblur="endRemoteDimmerBump(event,${fixture.id})" onkeydown="remoteDimmerBumpKeyDown(event,${fixture.id})" onkeyup="remoteDimmerBumpKeyUp(event,${fixture.id})">Bump</button>
      </div>
      <label class="remoteDimmerSlider"><span>Level</span><input type="range" min="${dimmerRange.min}" max="${dimmerRange.max}" step="1" value="${dimmer.value}" oninput="setRemoteDimmerValue(${fixture.id},this.value)"><strong>${remoteDmxPercent(dimmer.value)}</strong></label>
    </div>`);
  }
  if(position){
    const limitFlags=[position.limits.invert_pan?"Inv Pan":"",position.limits.invert_tilt?"Inv Tilt":"",position.limits.swap_pan_tilt?"Swap":""].filter(Boolean).join(" / ");
    const favoriteButtons=remotePositionFavorites.map(favorite=>{
      const active=remotePositionFavoriteMatches(position,favorite);
      return `<button class="${active?"active":""}" title="${escapeHtml(favorite.label)}: ${remoteDmxPercent(favorite.pan)} / ${remoteDmxPercent(favorite.tilt)}${active?" / current":""}" onclick="applyRemotePositionFavorite(${fixture.id},${remoteJsString(favorite.id)})"><span class="remotePositionFavoriteMap"><i style="left:${(remoteDmxClamp(favorite.pan)/65535)*100}%;top:${100-(remoteDmxClamp(favorite.tilt)/65535)*100}%"></i></span><strong>${escapeHtml(favorite.label)}</strong><small>${remoteDmxPercent(favorite.pan)} / ${remoteDmxPercent(favorite.tilt)}${active?" / Current":""}</small></button>`;
    }).join("");
    panels.push(`<div class="visualCard">
      <div class="visualCardHeader"><strong>Pan/Tilt</strong><span>${remoteDmxPercent(position.panValue)} / ${remoteDmxPercent(position.tiltValue)}${limitFlags?` / ${limitFlags}`:""}</span></div>
      <div class="remotePanTiltPad" data-fixture="${fixture.id}" onpointerdown="setRemotePanTiltFromPointer(event)" onpointermove="dragRemotePanTilt(event)" onpointerup="releaseRemotePointer(event)" onpointercancel="releaseRemotePointer(event)">
        <b class="remotePanTiltLimit" style="${remotePanTiltLimitStyle(fixture)}"></b>
        <i style="left:${(position.panValue/65535)*100}%;top:${100-(position.tiltValue/65535)*100}%"></i>
      </div>
      <div class="remoteTargetGrid">
        <button onclick="setRemotePanTiltTarget(${fixture.id},0,1)">UL</button><button onclick="setRemotePanTiltTarget(${fixture.id},0.5,1)">U</button><button onclick="setRemotePanTiltTarget(${fixture.id},1,1)">UR</button>
        <button onclick="setRemotePanTiltTarget(${fixture.id},0,0.5)">L</button><button class="primary" onclick="setRemotePanTiltTarget(${fixture.id},0.5,0.5)">C</button><button onclick="setRemotePanTiltTarget(${fixture.id},1,0.5)">R</button>
        <button onclick="setRemotePanTiltTarget(${fixture.id},0,0)">DL</button><button onclick="setRemotePanTiltTarget(${fixture.id},0.5,0)">D</button><button onclick="setRemotePanTiltTarget(${fixture.id},1,0)">DR</button>
      </div>
      <div class="remotePanTiltMirrorRow">
        <button onclick="setRemotePanTiltMirror(${fixture.id},'pan')">Mirror Pan</button>
        <button onclick="setRemotePanTiltMirror(${fixture.id},'tilt')">Mirror Tilt</button>
        <button onclick="setRemotePanTiltMirror(${fixture.id},'both')">Opposite</button>
      </div>
      <div class="remotePositionFavoriteHeader"><span class="small">Position Favorites</span><button onclick="addRemotePositionFavorite(${fixture.id})">Store</button><button onclick="resetRemotePositionFavorites()">Reset</button></div>
      <div class="remotePositionFavoriteGrid">${favoriteButtons}</div>
    </div>`);
  }
  if(color){
    const current=remoteColorDisplayHex(color);
    const hasWhite=color.extras.some(extra=>extra.key==="white");
    const extraSliders=color.extras.map(extra=>`<label>${escapeHtml(extra.short)}<input type="range" min="0" max="65535" step="1" value="${extra.value}" oninput="setRemoteExtraColorChannel(${fixture.id},'${escapeHtml(extra.key)}',this.value)"><span class="small">${remoteDmxPercent(extra.value)}</span></label>`).join("");
    panels.push(`<div class="visualCard">
      <div class="visualCardHeader"><strong>Color</strong><span>${current.toUpperCase()}</span></div>
      <div class="remoteColorPlane">
        <div class="remoteColorPreview" style="background:${current}"></div>
        <input type="color" value="${current}" onchange="setRemoteFixtureColor(${fixture.id},this.value)">
      </div>
      ${hasWhite?`<div class="remoteColorOptions"><label><input type="checkbox" ${remoteColorAutoWhite?"checked":""} onchange="setRemoteColorAutoWhite(this.checked)">Auto White</label></div>`:""}
      <div class="remoteColorPresetGrid">
        ${remoteColorPresets.map(preset=>`<button class="${remoteColorMatches(current,preset.color)?"active":""}" title="${escapeHtml(preset.color)}" onclick="setRemoteFixtureColor(${fixture.id},'${preset.color}')"><i style="background:${escapeHtml(preset.color)}"></i><span>${escapeHtml(preset.label)}</span></button>`).join("")}
      </div>
      <div class="remoteColorGrid">
        ${["#ff0000","#00ff00","#0000ff","#ffff00","#00ffff","#ff00ff","#ff7a00","#7a2cff"].map(hex=>`<button class="remoteColorButton ${remoteColorMatches(current,hex)?"active":""}" title="${hex}" style="background:${hex}" onclick="setRemoteFixtureColor(${fixture.id},'${hex}')"></button>`).join("")}
      </div>
      <div class="remoteRgbSliders">
        <label>R<input type="range" min="0" max="65535" step="1" value="${color.redValue}" oninput="setRemoteColorChannel(${fixture.id},'red',this.value)"><span class="small">${remoteDmxPercent(color.redValue)}</span></label>
        <label>G<input type="range" min="0" max="65535" step="1" value="${color.greenValue}" oninput="setRemoteColorChannel(${fixture.id},'green',this.value)"><span class="small">${remoteDmxPercent(color.greenValue)}</span></label>
        <label>B<input type="range" min="0" max="65535" step="1" value="${color.blueValue}" oninput="setRemoteColorChannel(${fixture.id},'blue',this.value)"><span class="small">${remoteDmxPercent(color.blueValue)}</span></label>
      </div>
      ${extraSliders?`<div class="remoteExtraColorSliders">${extraSliders}</div>`:""}
    </div>`);
  }
  if(colorWheelRows.length){
    const totalColorFunctions=colorWheelRows.reduce((sum,row)=>sum+row.functions.length,0);
    const groups=colorWheelRows.map(row=>{
      const buttons=row.functions.slice(0,12).map(fn=>{
        const value=remoteFunctionValue(fn);
        const active=remoteFunctionActive(fn,row.value);
        const color=remoteFunctionSwatchColor(row.control,fn)||"#2a3340";
        return `<button class="${active?"active":""}" title="${escapeHtml(row.control.attribute)} / ${escapeHtml(remoteFunctionRangeLabel(fn))}" onclick="setRemoteChannelFunction(${fixture.id},${remoteJsString(row.control.attribute)},${value})"><i class="remoteColorWheelSwatch" style="background:${escapeHtml(color)}"></i><strong>${escapeHtml(remoteFunctionLabel(fn))}</strong><span>${escapeHtml(remoteFunctionRangeLabel(fn))}</span></button>`;
      }).join("");
      return `<div class="remoteColorWheelGroup"><div class="remoteFunctionTitle"><strong>Color Wheel / ${escapeHtml(row.control.attribute)}</strong><span>${row.functions.length} fn</span></div><div class="remoteColorWheelGrid">${buttons}</div></div>`;
    }).join("");
    panels.push(`<div class="visualCard">
      <div class="visualCardHeader"><strong>Color Wheel</strong><span>${colorWheelRows.length} attr / ${totalColorFunctions} fn</span></div>
      <div class="remoteColorWheelList">${groups}</div>
    </div>`);
  }
  if(goboWheelRows.length){
    const totalGoboFunctions=goboWheelRows.reduce((sum,row)=>sum+row.functions.length,0);
    const groups=goboWheelRows.map(row=>{
      const buttons=row.functions.slice(0,14).map((fn,index)=>{
        const value=remoteFunctionValue(fn);
        const active=remoteFunctionActive(fn,row.value);
        const pattern=remoteGoboSlotPattern(row.control,fn,index);
        const label=remoteFunctionLabel(fn);
        const detail=remoteGoboSlotDetail(fn);
        return `<button class="${active?"active":""}" title="${escapeHtml(row.control.attribute)} / ${escapeHtml(remoteFunctionRangeLabel(fn))}" onclick="setRemoteChannelFunction(${fixture.id},${remoteJsString(row.control.attribute)},${value})"><i class="remoteGoboSlotIcon ${pattern}"></i><strong>${escapeHtml(label)}</strong><span>${escapeHtml(detail)}</span></button>`;
      }).join("");
      return `<div class="remoteFunctionGroup"><div class="remoteFunctionTitle"><strong>Gobo Slots / ${escapeHtml(row.control.attribute)}</strong><span>${row.functions.length} fn</span></div><div class="remoteGoboWheelGrid">${buttons}</div></div>`;
    }).join("");
    panels.push(`<div class="visualCard">
      <div class="visualCardHeader"><strong>Gobo Slots</strong><span>${goboWheelRows.length} attr / ${totalGoboFunctions} fn</span></div>
      <div class="remoteGoboWheelList">${groups}</div>
    </div>`);
  }
  if(opticsRows.length){
    const cards=opticsRows.map(row=>{
      const roleClass=String(row.role||"beam").toLowerCase().replace(/[^a-z0-9]/g,"")||"beam";
      const presets=remoteOpticsPresetButtons(row).map(preset=>`<button onclick="setRemoteOpticsValue(${fixture.id},${remoteJsString(row.control.attribute)},${preset.value})">${escapeHtml(preset.label)}</button>`).join("");
      const chips=row.functions.slice(0,5).map(fn=>`<button class="${remoteFunctionActive(fn,row.value)?"active":""}" title="${escapeHtml(remoteFunctionRangeLabel(fn))}" onclick="setRemoteChannelFunction(${fixture.id},${remoteJsString(row.control.attribute)},${remoteFunctionValue(fn)})">${escapeHtml(remoteFunctionLabel(fn))}</button>`).join("");
      return `<div class="remoteOpticsCard">
        <div class="remoteOpticsTitle"><strong>${escapeHtml(row.role)}</strong><span>${remoteDmxPercent(row.value)}</span></div>
        <div class="remoteOpticsPreview ${roleClass}" style="${remoteOpticsPreviewStyle(row)}" role="slider" tabindex="0" data-fixture="${fixture.id}" data-attribute="${escapeHtml(row.control.attribute)}" aria-label="${escapeHtml(row.control.attribute)} visual value" onpointerdown="setRemoteOpticsFromPointer(event)" onpointermove="dragRemoteOpticsValue(event)" onpointerup="releaseRemotePointer(event)" onpointercancel="releaseRemotePointer(event)" onkeydown="keyRemoteOpticsValue(event,${fixture.id},${remoteJsString(row.control.attribute)},${row.value})">
          <b class="remoteOpticsHalo"></b><b class="remoteOpticsCore"></b><b class="remoteOpticsPrism"></b><span class="remoteOpticsScale"><i></i></span>
        </div>
        <input type="range" min="0" max="65535" step="1" value="${remoteDmxClamp(row.value)}" oninput="setRemoteOpticsValue(${fixture.id},${remoteJsString(row.control.attribute)},this.value)">
        <div class="remoteOpticsQuickRow">${presets}</div>
        <div class="remoteOpticsMeta"><span>${escapeHtml(row.control.attribute)}</span>${row.activeFunction?`<b>${escapeHtml(remoteFunctionLabel(row.activeFunction))}</b>`:""}</div>
        ${chips?`<div class="remoteOpticsFunctionChips">${chips}</div>`:""}
      </div>`;
    }).join("");
    panels.push(`<div class="visualCard">
      <div class="visualCardHeader"><strong>Optics</strong><span>${opticsRows.length} attr</span></div>
      <div class="remoteOpticsGrid">${cards}</div>
    </div>`);
  }
  if(functionRows.length){
    const totalFunctions=functionRows.reduce((sum,row)=>sum+row.functions.length,0);
    const groups=functionRows.map(row=>{
      const buttons=row.functions.slice(0,10).map(fn=>{
        const value=remoteFunctionValue(fn);
        const active=remoteFunctionActive(fn,row.value);
        return `<button class="${active?"active":""}" title="${escapeHtml(row.control.attribute)} / ${escapeHtml(remoteFunctionRangeLabel(fn))}" onclick="setRemoteChannelFunction(${fixture.id},${remoteJsString(row.control.attribute)},${value})"><strong>${escapeHtml(remoteFunctionLabel(fn))}</strong><span>${escapeHtml(remoteFunctionRangeLabel(fn))}</span></button>`;
      }).join("");
      return `<div class="remoteFunctionGroup"><div class="remoteFunctionTitle"><strong>${escapeHtml(row.category)} / ${escapeHtml(row.control.attribute)}</strong><span>${row.functions.length} fn</span></div><div class="remoteFunctionGrid">${buttons}</div></div>`;
    }).join("");
    panels.push(`<div class="visualCard">
      <div class="visualCardHeader"><strong>Functions</strong><span>${functionRows.length} attr / ${totalFunctions} fn</span></div>
      <div class="remoteFunctionList">${groups}</div>
    </div>`);
  }
  el.innerHTML=panels.length?panels.join(""):`<span class="small">No Dimmer, Pan/Tilt, or RGB controls on this fixture</span>`;
}
function syncSelectedAttributeValue(){
  const attr=document.getElementById("attribute").value;
  const fixture=remoteControlFixture();
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
function remoteSceneSearchText(cue,index){
  return [
    cue.id,
    index+1,
    cue.label,
    `${(cue.targets||[]).length} light`,
    `${(cue.video_targets||[]).length} video`,
    `${(cue.video_output_targets||[]).length} output`,
    `${(cue.node_graph_targets||[]).length} graph`,
    `${cue.fade_ms||0}ms`
  ].join(" ").toLowerCase();
}
function remoteFilteredSceneRows(cues){
  const query=sceneFilterText.trim().toLowerCase();
  return (cues||[]).map((cue,index)=>({cue,index})).filter(row=>!query||remoteSceneSearchText(row.cue,row.index).includes(query));
}
function renderSceneSelector(cues,activeCueId){
  const el=document.getElementById("sceneSelector");
  if(!el)return;
  const search=document.getElementById("sceneFilter");
  if(search&&search.value!==sceneFilterText)search.value=sceneFilterText;
  if(!cues.length){el.innerHTML=`<span class="small">No scenes</span>`;return}
  const activeIndex=cues.findIndex(c=>c.id===activeCueId);
  const nextCueId=cues.length?cues[((activeIndex<0?-1:activeIndex)+1+cues.length)%cues.length].id:null;
  const query=sceneFilterText.trim().toLowerCase();
  const rows=remoteFilteredSceneRows(cues);
  const firstMatchId=query&&rows.length?rows[0].cue.id:null;
  const visible=rows.slice(0,query?32:16);
  const more=rows.length-visible.length;
  el.innerHTML=visible.length?visible.map(({cue,index})=>{
    const className=`sceneButton ${cue.id===activeCueId?"active":""} ${cue.id===nextCueId?"next":""} ${cue.id===firstMatchId?"firstMatch":""}`;
    return `<button class="${className}" onclick="triggerCue(${cue.id})"><b>${index+1}</b><strong>${escapeHtml(cue.label)}</strong><span>${(cue.targets||[]).length} light / ${(cue.video_targets||[]).length} video</span><small>${cue.fade_ms||0}ms fade / cue ${cue.id}</small>${cue.id===firstMatchId?"<em>Enter GO</em>":""}</button>`;
  }).join("")+(more>0?`<span class="small">+${more} more scene(s). Narrow search to show them.</span>`:""):`<span class="small">No scenes match "${escapeHtml(sceneFilterText)}"</span>`;
}
function remoteEffectTypeLabel(effect){
  return effect&&effect.effect_type==="PositionWave"?"Wave":"LFO";
}
function remoteEffectTimingText(effect){
  if(effect&&effect.clock_sync&&Number(effect.clock_sync.beats)>0){
    const beats=Number(effect.clock_sync.beats);
    return `${beats.toFixed(beats<1?2:beats<10?1:0)} beat sync`;
  }
  if(effect&&Number(effect.period_ms)>0)return `${Math.round(Number(effect.period_ms))} ms`;
  if(effect&&Number(effect.wavelength)>0)return `wave ${Number(effect.wavelength).toFixed(2)}m`;
  return "free";
}
function remoteEffectRangeText(effect){
  const low=remoteDmxPercent(effect&&effect.low);
  const high=remoteDmxPercent(effect&&effect.high);
  return `${low}-${high}`;
}
function remoteEffectTargetText(effect){
  const fixtureCount=(effect&&effect.fixture_ids||[]).length;
  const groupCount=(effect&&effect.target_group_ids||[]).length;
  const videoTargets=effect&&effect.video_targets||[];
  const videoLayerCount=videoTargets.reduce((sum,target)=>sum+(target.layer_ids||[]).length,0);
  const parts=[];
  if(fixtureCount)parts.push(`${fixtureCount} fixture`);
  if(groupCount)parts.push(`${groupCount} group`);
  if(effect&&effect.attribute)parts.push(effect.attribute);
  if(videoLayerCount)parts.push(`${videoLayerCount} video`);
  return parts.join(" / ")||"No targets";
}
function remoteVideoEffectTargetText(effect){
  const targets=effect&&effect.video_targets||[];
  if(!targets.length)return "";
  return targets.slice(0,3).map(target=>{
    const layers=(target.layer_ids||[]).join(",");
    return `${target.param||"Param"} L${layers||"-"} ${Number(target.low||0).toFixed(2)}-${Number(target.high||0).toFixed(2)}`;
  }).join(" / ")+(targets.length>3?` / +${targets.length-3}`:"");
}
function renderEffectList(effects){
  const el=document.getElementById("effectList");
  if(!el)return;
  if(!effects.length){el.innerHTML=`<span class="small">No effects</span>`;return}
  el.innerHTML=effects.map((effect,index)=>{
    const enabled=effect.enabled!==false;
    const type=remoteEffectTypeLabel(effect);
    const timing=remoteEffectTimingText(effect);
    const sync=effect.clock_sync&&Number(effect.clock_sync.beats)>0;
    const videoText=remoteVideoEffectTargetText(effect);
    return `<div class="effectDeck ${enabled?"active":"off"}">
      <div class="effectDeckHeader"><div><strong>${escapeHtml(effect.label||`Effect ${effect.id}`)}</strong><span class="small">#${index+1} / ${type} / ${escapeHtml(effect.shape||"Shape")} / ${escapeHtml(effect.blend_mode||"Override")}</span></div><span class="small">${enabled?"On":"Off"}</span></div>
      <div class="effectDeckChips">
        <span class="effectDeckChip">${escapeHtml(remoteEffectTargetText(effect))}</span>
        <span class="effectDeckChip ${sync?"sync":""}">${escapeHtml(timing)}</span>
        <span class="effectDeckChip">${escapeHtml(remoteEffectRangeText(effect))}</span>
        ${videoText?`<span class="effectDeckChip">${escapeHtml(videoText)}</span>`:""}
      </div>
      <div class="effectDeckControls">
        <button class="${enabled?"":"primary"}" onclick="setRemoteEffectEnabled(${effect.id},${!enabled})">${enabled?"Disable":"Enable"}</button>
        <button onclick="moveRemoteEffect(${effect.id},-1)" ${index===0?"disabled":""}>Up</button>
        <button onclick="moveRemoteEffect(${effect.id},1)" ${index===effects.length-1?"disabled":""}>Down</button>
        <button onclick="removeRemoteEffect(${effect.id},${remoteJsString(effect.label||`Effect ${effect.id}`)})">Remove</button>
      </div>
    </div>`;
  }).join("");
}
function remoteNodeGraphSourceText(graph){
  const nodes=graph&&graph.nodes||[];
  const wave=nodes.find(node=>node.kind==="PositionWave"&&node.position_wave);
  if(wave){
    const body=wave.position_wave||{};
    return `Wave ${body.shape||"Shape"} / wl ${Number(body.wavelength||0).toFixed(2)} / speed ${Number(body.speed||0).toFixed(2)}`;
  }
  const lfo=nodes.find(node=>node.kind==="Lfo"&&node.lfo);
  if(lfo){
    const body=lfo.lfo||{};
    if(body.clock_sync&&Number(body.clock_sync.beats)>0)return `LFO ${body.shape||"Shape"} / sync ${Number(body.clock_sync.beats).toFixed(2)} beat`;
    return `LFO ${body.shape||"Shape"} / ${Math.round(Number(body.period_ms)||0)} ms`;
  }
  return "No source";
}
function remoteNodeGraphTargetText(graph){
  const output=(graph&&graph.nodes||[]).find(node=>node.kind==="Output"&&node.output);
  const body=output&&output.output||{};
  const parts=[];
  if((body.fixture_ids||[]).length)parts.push(`${body.fixture_ids.length} fixture`);
  if((body.target_group_ids||[]).length)parts.push(`${body.target_group_ids.length} group`);
  if(body.attribute)parts.push(body.attribute);
  const videoCount=(body.video_targets||[]).reduce((sum,target)=>sum+(target.layer_ids||[]).length,0);
  if(videoCount)parts.push(`${videoCount} video`);
  return parts.join(" / ")||"No output";
}
function renderNodeGraphList(graphs){
  const el=document.getElementById("nodeGraphList");
  if(!el)return;
  if(!graphs.length){el.innerHTML=`<span class="small">No node graphs</span>`;return}
  el.innerHTML=graphs.map((graph,index)=>{
    const enabled=graph.enabled!==false;
    return `<div class="effectDeck ${enabled?"active":"off"}">
      <div class="effectDeckHeader"><div><strong>${escapeHtml(graph.label||`Graph ${graph.id}`)}</strong><span class="small">#${index+1} / ${(graph.nodes||[]).length} node / ${(graph.edges||[]).length} edge</span></div><span class="small">${enabled?"On":"Off"}</span></div>
      <div class="effectDeckChips">
        <span class="effectDeckChip">${escapeHtml(remoteNodeGraphSourceText(graph))}</span>
        <span class="effectDeckChip">${escapeHtml(remoteNodeGraphTargetText(graph))}</span>
      </div>
      <div class="effectDeckControls">
        <button class="${enabled?"":"primary"}" onclick="setRemoteNodeGraphEnabled(${graph.id},${!enabled})">${enabled?"Disable":"Enable"}</button>
      </div>
    </div>`;
  }).join("");
}
function remoteVideoTimeline(layer,position,duration,loopStart,loopEnd,cuePoints){
  const max=Math.max(1,Number(duration)||1);
  const xFor=time=>4+Math.max(0,Math.min(1,(Number(time)||0)/max))*92;
  const playX=xFor(position);
  const startX=xFor(loopStart);
  const endX=xFor(loopEnd);
  const triangle=(x,top)=>top?`${x},6 ${x-3},11 ${x+3},11`:`${x},28 ${x-3},23 ${x+3},23`;
  const markers=cuePoints.map((cue,index)=>{
    const x=xFor(cue.position_ms);
    const color=escapeHtml(cue.color||"#4aa8ff");
    return `<line class="remoteVideoTimelineCueLine" x1="${x}" x2="${x}" y1="9" y2="25" style="--remote-cue-color:${color}"></line><polygon class="remoteVideoTimelineCue" points="${triangle(x,false)}" style="--remote-cue-color:${color}" onclick="jumpLayerCuePoint(${layer.id},${index})"><title>${escapeHtml(cue.label)} ${formatRemoteTime(cue.position_ms)}</title></polygon>`;
  }).join("");
  return `<div class="remoteVideoTimeline">
    <svg class="remoteVideoTimelineSurface" viewBox="0 0 100 34" role="img">
      <rect class="remoteVideoTimelineHit" x="0" y="0" width="100" height="34" onpointerdown="setRemoteVideoTimelinePosition(event,${layer.id})" onpointermove="dragRemoteVideoTimelinePosition(event,${layer.id})" onpointerup="releaseRemotePointer(event)" onpointercancel="releaseRemotePointer(event)"><title>Seek ${escapeHtml(layer.label)}</title></rect>
      <rect class="remoteVideoTimelineTrack" x="4" y="12" width="92" height="8"></rect>
      <rect class="remoteVideoTimelineLoop ${layer.state&&layer.state.loop_enabled?"":"off"}" x="${startX}" y="12" width="${Math.max(.5,endX-startX)}" height="8"></rect>
      ${markers}
      <line class="remoteVideoTimelinePlayhead" x1="${playX}" x2="${playX}" y1="4" y2="30"></line>
      <polygon class="remoteVideoTimelineLoopHandle start" points="${triangle(startX,true)}" onpointerdown="setRemoteVideoTimelineLoop(event,${layer.id},'start')" onpointermove="dragRemoteVideoTimelineLoop(event,${layer.id},'start')" onpointerup="releaseRemotePointer(event)" onpointercancel="releaseRemotePointer(event)"><title>Loop in ${formatRemoteTime(loopStart)}</title></polygon>
      <polygon class="remoteVideoTimelineLoopHandle end" points="${triangle(endX,true)}" onpointerdown="setRemoteVideoTimelineLoop(event,${layer.id},'end')" onpointermove="dragRemoteVideoTimelineLoop(event,${layer.id},'end')" onpointerup="releaseRemotePointer(event)" onpointercancel="releaseRemotePointer(event)"><title>Loop out ${formatRemoteTime(loopEnd)}</title></polygon>
    </svg>
    <div class="remoteVideoTimelineReadout"><span>${formatRemoteTime(position)}</span><span>${formatRemoteTime(max)}</span><span>${layer.state&&layer.state.loop_enabled?`${formatRemoteTime(loopStart)} - ${formatRemoteTime(loopEnd)}`:"Loop off"}</span></div>
  </div>`;
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
    const bpmSync=state.bpm_sync||{};
    const bpmSyncEnabled=bpmSync.enabled===true;
    const bpmLoopBars=Number.isFinite(Number(bpmSync.loop_bars))?Number(bpmSync.loop_bars):1;
    const syncInfo=bpmSyncEnabled?` / BPM ${bpmLoopBars.toFixed(2)} bar`:"";
    const loopStartMax=Math.max(0,sliderMax-1);
    const loopStart=Math.max(0,Math.min(loopStartMax,state.loop_start_ms||0));
    const loopEnd=Math.max(loopStart+1,Math.min(sliderMax,state.loop_end_ms||sliderMax));
    const cuePoints=videoCuePointsForState(state);
    const cueButtons=cuePoints.map((cue,index)=>`<button style="border-color:${escapeHtml(cue.color)}" onclick="jumpLayerCuePoint(${l.id},${index})">${escapeHtml(cue.label)}</button>`).join("");
    return `<div class="videoDeck ${enabled&&solo?"active":""}">
      <div class="videoDeckHeader"><div><strong>${escapeHtml(l.label)}</strong><span class="small">${escapeHtml(l.source.kind)} / ${escapeHtml(l.blend_mode)} / ${enabled?"On":"Off"}${solo?" / Solo":""} / ${state.playing?"Playing":"Paused"}${syncInfo}</span></div><span class="small">${formatRemoteTime(position)}${duration?` / ${formatRemoteTime(duration)}`:""}</span></div>
      <div class="bar"><span style="width:${seekPct}%"></span></div>
      ${remoteVideoTimeline(l,position,sliderMax,loopStart,loopEnd,cuePoints)}
      <div class="videoDeckControls">
        <button onclick="selectLayer(${l.id})">Select</button>
        <button onclick="setLayerPlaying(${l.id},${!state.playing})">${state.playing?"Pause":"Play"}</button>
        <button onclick="setLayerEnabled(${l.id},${!enabled})">${enabled?"Off":"On"}</button>
        <button onclick="setLayerSolo(${l.id},${!solo})">${solo?"Unsolo":"Solo"}</button>
        <button onclick="setLayerLoopFromInputs(${l.id},${!loop})">${loop?"Loop Off":"Loop On"}</button>
        <button onclick="fadeLayerOpacity(${l.id},0)">Fade Out</button>
        <button onclick="fadeLayerOpacity(${l.id},1)">Fade In</button>
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
      <div class="remoteBpmPresetRow">
        ${[["1 Beat",0.25],["1 Bar",1],["2 Bars",2],["4 Bars",4]].map(([label,bars])=>`<button class="${bpmSyncEnabled&&Math.abs(bpmLoopBars-Number(bars))<0.001?"active":""}" onclick="setLayerBpmLoopBars(${l.id},${bars})">${label}</button>`).join("")}
        <button class="primary" onclick="matchLayerBpmLoopToCurrentLength(${l.id})">Match</button>
      </div>
      <div class="inline"><button onclick="setLayerSpeedPreset(${l.id},-1)">Reverse</button><button onclick="setLayerSpeedPreset(${l.id},0.5)">0.5x</button><button onclick="setLayerSpeedPreset(${l.id},1)">1x</button><button onclick="setLayerSpeedPreset(${l.id},2)">2x</button><button onclick="seekLayerFromInput(${l.id},0)">Restart</button><button onclick="seekLayerByFrame(${l.id},-1)">-Frame</button><button onclick="seekLayerByFrame(${l.id},1)">+Frame</button><button onclick="setLayerLoopBoundaryToCurrent(${l.id},'in')">Loop In</button><button onclick="setLayerLoopBoundaryToCurrent(${l.id},'out')">Loop Out</button><button onclick="jumpLayerRelativeCuePoint(${l.id},-1)">Prev Cue</button><button onclick="jumpLayerRelativeCuePoint(${l.id},1)">Next Cue</button><button onclick="addLayerCuePoint(${l.id})">Add Cue</button></div>
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
    const plan=remoteVideoOutputRenderPlanById(o.id);
    const planState=remoteVideoOutputRenderPlanState(plan,o);
    const active=planState.label==="Live"&&(o.opacity??1)>0;
    const selected=remoteStageSelectedOutputId===o.id;
    const detail=remoteVideoOutputRenderPlanDetail(plan,o);
    return `<div class="row ${active?"active":""} ${selected?"selected":""}"><div><strong>${escapeHtml(o.label)}</strong><span class="small">${escapeHtml(o.kind)} / ${escapeHtml(comp?comp.label:`Composition ${o.composition_id}`)} / ${pct}%${o.enabled?"":" / Disabled"}${o.blackout?" / Blackout":""}</span><span class="status ${planState.badgeClass}">${escapeHtml(planState.label)} / ${planState.layers.length} layer${planState.layers.length===1?"":"s"}</span><span class="small">${escapeHtml(detail)}</span><div class="bar"><span style="width:${Math.max(0,Math.min(100,pct))}%"></span></div><label class="deckSlider">Opacity<input type="range" min="0" max="1" step="0.01" value="${Number(o.opacity??1)}" oninput="setVideoOutputOpacityFromInput(${o.id},this.value)"><strong>${pct}%</strong></label>${remoteVideoOutputMappingPanel(o)}</div><div class="inline"><button onclick="selectRemoteVideoOutput(${o.id})">Sel</button><button onclick="setVideoOutputEnabled(${o.id},${!o.enabled})">${o.enabled?"Disable":"Enable"}</button><button onclick="setVideoOutputBlackout(${o.id},${!o.blackout})">${o.blackout?"Clear":"Blackout"}</button><button onclick="fadeVideoOutput(${o.id},0)">Fade Out</button><button onclick="fadeVideoOutput(${o.id},1)">Fade In</button><button onclick="setVideoOutputOpacity(${o.id},0)">Cut</button><button onclick="setVideoOutputOpacity(${o.id},0.5)">Half</button><button onclick="setVideoOutputOpacity(${o.id},1)">Full</button></div></div>`
  }).join("");
}
function remoteFinite(value,fallback){return Number.isFinite(Number(value))?Number(value):fallback}
function remoteClamp(value,min,max){return Math.min(max,Math.max(min,Number(value)||0))}
function remoteRoundedMappingValue(value){return Number(remoteClamp(value,-1,1).toFixed(3))}
function remoteRoundedRangeValue(value,min,max,digits=3){return Number(remoteClamp(value,min,max).toFixed(digits))}
function remoteDefaultMapping(){
  return {stage_x:0,stage_y:0,stage_z:0,offset_x:0,offset_y:0,scale_x:1,scale_y:1,rotation_deg:0,aspect_ratio:1,aspect_mode:"Stretch",lens_distortion:0,keystone_x:0,keystone_y:0,corner_top_left_x:0,corner_top_left_y:0,corner_top_right_x:0,corner_top_right_y:0,corner_bottom_right_x:0,corner_bottom_right_y:0,corner_bottom_left_x:0,corner_bottom_left_y:0};
}
function remoteMappingFieldRange(field){
  if(field==="stage_x"||field==="stage_y"||field==="stage_z")return [-1000,1000];
  if(field==="scale_x"||field==="scale_y")return [0.01,8];
  if(field==="aspect_ratio")return [0.1,10];
  if(field==="rotation_deg")return [-180,180];
  if(field==="offset_x"||field==="offset_y")return [-4,4];
  return [-1,1];
}
const remoteProjectorCornerGain=18;
const remoteProjectorKeystoneGain=24;
const remoteProjectorCorners=[
  {key:"tl",label:"TL",baseX:-1,baseY:-1,xField:"corner_top_left_x",yField:"corner_top_left_y"},
  {key:"tr",label:"TR",baseX:1,baseY:-1,xField:"corner_top_right_x",yField:"corner_top_right_y"},
  {key:"br",label:"BR",baseX:1,baseY:1,xField:"corner_bottom_right_x",yField:"corner_bottom_right_y"},
  {key:"bl",label:"BL",baseX:-1,baseY:1,xField:"corner_bottom_left_x",yField:"corner_bottom_left_y"}
];
const remoteProjectorKeystoneHandles=[
  {axis:"x",label:"Key H",baseX:50,baseY:10},
  {axis:"y",label:"Key V",baseX:90,baseY:50}
];
const remoteProjectorKeystonePresets=[
  {label:"Flat",keystoneX:0,keystoneY:0},
  {label:"H -",keystoneX:-0.35,keystoneY:0},
  {label:"H +",keystoneX:0.35,keystoneY:0},
  {label:"V -",keystoneX:0,keystoneY:-0.35},
  {label:"V +",keystoneX:0,keystoneY:0.35}
];
const remoteOutputAspectPresets=[
  {label:"16:9",ratio:16/9},
  {label:"4:3",ratio:4/3},
  {label:"1:1",ratio:1},
  {label:"9:16",ratio:9/16},
  {label:"21:9",ratio:21/9}
];
function remoteProjectorBasePoint(mapping,corner){
  const aspect=remoteClamp(remoteFinite(mapping.aspect_ratio,1),0.25,4);
  const aspectScaleX=aspect>=1?1:aspect;
  const aspectScaleY=aspect>=1?1/Math.min(aspect,2.8):1;
  const scaleX=remoteClamp(remoteFinite(mapping.scale_x,1),0.25,2.5);
  const scaleY=remoteClamp(remoteFinite(mapping.scale_y,1),0.25,2.5);
  const localX=corner.baseX*30*aspectScaleX*scaleX+remoteClamp(mapping.offset_x,-1,1)*20+remoteClamp(mapping.keystone_x,-1,1)*corner.baseY*12;
  const localY=corner.baseY*30*aspectScaleY*scaleY+remoteClamp(mapping.offset_y,-1,1)*20+remoteClamp(mapping.keystone_y,-1,1)*corner.baseX*12;
  const rotation=remoteFinite(mapping.rotation_deg,0)*(Math.PI/180);
  const sin=Math.sin(rotation);
  const cos=Math.cos(rotation);
  return {x:50+localX*cos-localY*sin,y:50+localX*sin+localY*cos};
}
function remoteProjectorPoint(mapping,corner){
  const base=remoteProjectorBasePoint(mapping,corner);
  return {x:base.x+remoteClamp(mapping[corner.xField],-1,1)*remoteProjectorCornerGain,y:base.y+remoteClamp(mapping[corner.yField],-1,1)*remoteProjectorCornerGain};
}
function remoteProjectorPointList(mapping){
  return remoteProjectorCorners.map(corner=>remoteProjectorPoint(mapping,corner));
}
function remoteProjectorCenter(mapping){
  const points=remoteProjectorPointList(mapping);
  return {x:points.reduce((sum,point)=>sum+point.x,0)/points.length,y:points.reduce((sum,point)=>sum+point.y,0)/points.length};
}
function remoteProjectorKeystonePoint(mapping,axis){
  return axis==="x"?{x:50+remoteClamp(mapping.keystone_x,-1,1)*remoteProjectorKeystoneGain,y:10}:{x:90,y:50+remoteClamp(mapping.keystone_y,-1,1)*remoteProjectorKeystoneGain};
}
function remoteProjectorPoints(mapping,base=false){
  const points=base?remoteProjectorCorners.map(corner=>remoteProjectorBasePoint(mapping,corner)):remoteProjectorPointList(mapping);
  return points.map(point=>`${point.x},${point.y}`).join(" ");
}
function remoteProjectorKeystonePresetPoints(keystoneX,keystoneY){
  return remoteProjectorCorners.map(corner=>{
    const x=20+corner.baseX*12+keystoneX*corner.baseY*5;
    const y=12+corner.baseY*8+keystoneY*corner.baseX*4;
    return `${x},${y}`;
  }).join(" ");
}
function remoteOutputRatio(output){
  const width=Number(output.width)||1;
  const height=Math.max(1,Number(output.height)||1);
  return Number((width/height).toFixed(4));
}
function remoteMapReadout(mapping){
  return `Stage ${remoteFinite(mapping.stage_x,0).toFixed(1)}, ${remoteFinite(mapping.stage_y,0).toFixed(1)}, ${remoteFinite(mapping.stage_z,0).toFixed(1)} / X ${remoteFinite(mapping.offset_x,0).toFixed(2)} / Y ${remoteFinite(mapping.offset_y,0).toFixed(2)} / Key ${remoteFinite(mapping.keystone_x,0).toFixed(2)}, ${remoteFinite(mapping.keystone_y,0).toFixed(2)} / ${mapping.aspect_mode||"Stretch"} ${remoteFinite(mapping.aspect_ratio,1).toFixed(2)}`;
}
const remoteStageSurfaceCornerGain=0.28;
const remoteStageSurfaceCorners=[
  {key:"tl",label:"TL",baseX:-1,baseZ:-1,xField:"corner_top_left_x",zField:"corner_top_left_y"},
  {key:"tr",label:"TR",baseX:1,baseZ:-1,xField:"corner_top_right_x",zField:"corner_top_right_y"},
  {key:"br",label:"BR",baseX:1,baseZ:1,xField:"corner_bottom_right_x",zField:"corner_bottom_right_y"},
  {key:"bl",label:"BL",baseX:-1,baseZ:1,xField:"corner_bottom_left_x",zField:"corner_bottom_left_y"}
];
function remoteStageSurfaceHalfSize(output,mapping){
  const outputAspect=Number(output.height)>0?Number(output.width)/Number(output.height):1;
  const mappedAspect=remoteClamp(remoteFinite(mapping.aspect_ratio,outputAspect),0.35,4);
  const aspect=(mapping.aspect_mode||"Stretch")==="Stretch"?outputAspect:mappedAspect;
  const baseHeight=4.5*remoteClamp(remoteFinite(mapping.scale_y,1),0.25,3);
  return {
    width:baseHeight*Math.max(0.35,aspect)*remoteClamp(remoteFinite(mapping.scale_x,1),0.25,3),
    height:baseHeight
  };
}
function remoteStageRotatePoint(centerX,centerZ,localX,localZ,rotationDeg){
  const angle=remoteFinite(rotationDeg,0)*(Math.PI/180);
  const cos=Math.cos(angle);
  const sin=Math.sin(angle);
  return {x:centerX+localX*cos-localZ*sin,z:centerZ+localX*sin+localZ*cos};
}
function remoteStageObjectCorners(object){
  const width=Math.max(0.1,remoteFinite(object.width,1));
  const depth=Math.max(0.1,remoteFinite(object.depth,1));
  return [
    {x:-width/2,z:-depth/2},
    {x:width/2,z:-depth/2},
    {x:width/2,z:depth/2},
    {x:-width/2,z:depth/2}
  ].map(point=>remoteStageRotatePoint(remoteFinite(object.x,0),remoteFinite(object.z,0),point.x,point.z,object.rotation_deg));
}
function remoteStageOutputCorners(output){
  const mapping={...remoteDefaultMapping(),...(output.mapping||{})};
  const size=remoteStageSurfaceHalfSize(output,mapping);
  return remoteStageSurfaceCorners.map(corner=>{
    const localX=corner.baseX*size.width+remoteClamp(mapping[corner.xField],-1,1)*size.width*2*remoteStageSurfaceCornerGain;
    const localZ=corner.baseZ*size.height+remoteClamp(mapping[corner.zField],-1,1)*size.height*2*remoteStageSurfaceCornerGain;
    return remoteStageRotatePoint(remoteFinite(mapping.stage_x,0),remoteFinite(mapping.stage_z,0),localX,localZ,mapping.rotation_deg);
  });
}
function remoteStageAddPoint(points,x,z){
  const px=Number(x);
  const pz=Number(z);
  if(Number.isFinite(px)&&Number.isFinite(pz))points.push({x:px,z:pz});
}
function remoteStageBounds(snapshot){
  const map=snapshot.stage_map||{};
  if(map.locked&&Number.isFinite(Number(map.min_x))&&Number.isFinite(Number(map.max_x))&&Number.isFinite(Number(map.min_z))&&Number.isFinite(Number(map.max_z))&&Number(map.max_x)>Number(map.min_x)&&Number(map.max_z)>Number(map.min_z)){
    return {minX:Number(map.min_x),maxX:Number(map.max_x),minZ:Number(map.min_z),maxZ:Number(map.max_z),locked:true};
  }
  const points=[];
  for(const fixture of snapshot.fixtures||[])remoteStageAddPoint(points,fixture.position&&fixture.position.x,fixture.position&&fixture.position.z);
  for(const output of (((snapshot.video||{}).outputs)||[]))for(const point of remoteStageOutputCorners(output))remoteStageAddPoint(points,point.x,point.z);
  for(const object of snapshot.stage_objects||[])for(const point of remoteStageObjectCorners(object))remoteStageAddPoint(points,point.x,point.z);
  if(!points.length)return {minX:-10,maxX:10,minZ:-10,maxZ:10,locked:false};
  let minX=Math.min(...points.map(point=>point.x));
  let maxX=Math.max(...points.map(point=>point.x));
  let minZ=Math.min(...points.map(point=>point.z));
  let maxZ=Math.max(...points.map(point=>point.z));
  const centerX=(minX+maxX)/2;
  const centerZ=(minZ+maxZ)/2;
  const spanX=Math.max(20,maxX-minX);
  const spanZ=Math.max(20,maxZ-minZ);
  const pad=Math.max(2,Math.max(spanX,spanZ)*0.08);
  minX=centerX-spanX/2-pad;
  maxX=centerX+spanX/2+pad;
  minZ=centerZ-spanZ/2-pad;
  maxZ=centerZ+spanZ/2+pad;
  return {minX,maxX,minZ,maxZ,locked:false};
}
function remoteStageGridStep(bounds){
  const span=Math.max(bounds.maxX-bounds.minX,bounds.maxZ-bounds.minZ);
  if(span>160)return 20;
  if(span>80)return 10;
  if(span>36)return 5;
  if(span>18)return 2;
  return 1;
}
function remoteStagePointList(points){
  return points.map(point=>`${point.x.toFixed(3)},${point.z.toFixed(3)}`).join(" ");
}
function remoteStageKindClass(kind){
  return String(kind||"stage").toLowerCase();
}
function remoteStageFixtureIntensity(fixture){
  const values=fixture.attribute_values||[];
  const dimmer=values.find(value=>/dimmer|intensity|master/i.test(String(value.attribute||"")));
  return dimmer?remoteClamp(Number(dimmer.value)/65535,0,1):0.35;
}
function remoteStageSelectedOutput(outputs){
  if(remoteStageSelectedOutputId===null)return null;
  const output=(outputs||[]).find(candidate=>candidate.id===remoteStageSelectedOutputId)||null;
  if(!output)remoteStageSelectedOutputId=null;
  return output;
}
function selectRemoteStageOutput(id){
  remoteStageSelectedOutputId=Number(id);
  renderRemoteStage(latestSnapshot||{});
}
function renderRemoteStageSelection(snapshot,fixture,output){
  const el=document.getElementById("remoteStageSelection");
  if(!el)return;
  const rows=[];
  if(fixture){
    const states=[fixture.highlighted?"Highlight":"",fixture.soloed?"Solo":"",fixture.parked?"Park":""].filter(Boolean).join(" / ")||"Live";
    rows.push(`<div class="remoteStageSelectionRow"><div><strong>${escapeHtml(fixture.label)}</strong><span>U${fixture.universe} / ${fixture.address} / ${escapeHtml(fixture.group_ids&&fixture.group_ids.length?fixture.group_ids.join(", "):fixture.mode_name)} / ${states}</span></div><button onclick="setSelectedFixtureHighlight(${!fixture.highlighted})">HL</button><button onclick="setSelectedFixtureSolo(${!fixture.soloed})">Solo</button><button onclick="setSelectedFixturePark(${!fixture.parked})">Park</button></div>`);
  }
  if(output){
    const opacity=Math.round(remoteClamp(Number(output.opacity??1),0,1)*100);
    const plan=remoteVideoOutputRenderPlanById(output.id);
    const planState=remoteVideoOutputRenderPlanState(plan,output);
    rows.push(`<div class="remoteStageSelectionRow output"><div><strong>${escapeHtml(output.label)}</strong><span>${escapeHtml(output.kind)} / ${output.enabled?"Enabled":"Disabled"} / ${output.blackout?"Blackout":`${opacity}%`} / ${escapeHtml(planState.label)} ${planState.layers.length} layer${planState.layers.length===1?"":"s"}</span></div><button onclick="setVideoOutputEnabled(${output.id},${!output.enabled})">${output.enabled?"Off":"On"}</button><button onclick="setVideoOutputBlackout(${output.id},${!output.blackout})">BO</button><button onclick="fadeVideoOutput(${output.id},0)">Out</button><button onclick="fadeVideoOutput(${output.id},1)">In</button><button onclick="setVideoOutputOpacity(${output.id},1)">Full</button></div>`);
  }
  el.innerHTML=rows.length?rows.join(""):`<div class="remoteStageSelectionRow empty"><div><strong>No stage target</strong><span>Tap a fixture or projector surface.</span></div></div>`;
}
function renderRemoteStage(snapshot){
  const svg=document.getElementById("remoteStage");
  const readout=document.getElementById("remoteStageReadout");
  if(!svg||!readout)return;
  const fixtures=snapshot.fixtures||[];
  const stageGroupId=selectedRemoteStageGroupId();
  const visibleFixtures=stageGroupId?fixtures.filter(fixture=>remoteFixtureMatchesGroup(fixture,stageGroupId)):fixtures;
  const outputs=(((snapshot.video||{}).outputs)||[]);
  const objects=snapshot.stage_objects||[];
  const bounds=remoteStageBounds(snapshot);
  const width=Math.max(1,bounds.maxX-bounds.minX);
  const height=Math.max(1,bounds.maxZ-bounds.minZ);
  const span=Math.max(width,height);
  const grid=remoteStageGridStep(bounds);
  const labelSize=Math.max(0.42,Math.min(1.2,span*0.018));
  const fixtureRadius=Math.max(0.28,Math.min(0.92,span*0.012));
  const selectedFixtureId=selectedNumber("fixtureId");
  const selectedFixture=remoteFixtureById(selectedFixtureId);
  const selectedOutput=remoteStageSelectedOutput(outputs);
  renderRemoteStageSelection(snapshot,selectedFixture,selectedOutput);
  svg.setAttribute("viewBox",`${bounds.minX} ${bounds.minZ} ${width} ${height}`);
  readout.innerHTML=[
    `<span>${visibleFixtures.length}${stageGroupId?` / ${fixtures.length}`:""} fixture${visibleFixtures.length===1?"":"s"}</span>`,
    `<span>${stageGroupId?`Group ${escapeHtml(stageGroupId)}`:"All groups"}</span>`,
    `<span>${outputs.length} output${outputs.length===1?"":"s"}</span>`,
    `<span>${objects.length} object${objects.length===1?"":"s"}</span>`,
    `<span>${bounds.locked?"Locked map":"Auto map"}</span>`
  ].join("");
  const objectMarkup=objects.map(object=>{
    const points=remoteStageObjectCorners(object);
    const labelX=remoteFinite(object.x,0);
    const labelZ=remoteFinite(object.z,0);
    const color=/^#[0-9a-fA-F]{6}$/.test(String(object.color||""))?String(object.color):"";
    const style=color?` style="fill:${color}22;stroke:${color}"`:"";
    return `<g><polygon class="remoteStageObject kind-${remoteStageKindClass(object.kind)}" points="${remoteStagePointList(points)}"${style}><title>${escapeHtml(object.label)} / ${escapeHtml(object.kind)}</title></polygon><text class="remoteStageSurfaceLabel" style="font-size:${labelSize}px" x="${labelX}" y="${labelZ-labelSize*.8}">${escapeHtml(object.label)}</text></g>`;
  }).join("");
  const outputMarkup=outputs.map(output=>{
    const points=remoteStageOutputCorners(output);
    const mapping={...remoteDefaultMapping(),...(output.mapping||{})};
    const className=`remoteStageOutput ${output.enabled&&!output.blackout?"":"inactive"} ${remoteStageSelectedOutputId===output.id?"selected":""}`;
    const centerX=remoteFinite(mapping.stage_x,0);
    const centerZ=remoteFinite(mapping.stage_z,0);
    return `<g class="remoteStageOutputGroup" onclick="selectRemoteStageOutput(${output.id})"><polygon class="${className}" points="${remoteStagePointList(points)}"><title>${escapeHtml(output.label)} / ${escapeHtml(output.kind)}</title></polygon><circle class="remoteStageOutputCenter" cx="${centerX}" cy="${centerZ}" r="${fixtureRadius*.62}" /><text class="remoteStageSurfaceLabel" style="font-size:${labelSize}px" x="${centerX+fixtureRadius}" y="${centerZ-labelSize*.6}">${escapeHtml(output.label)}</text></g>`;
  }).join("");
  const fixtureMarkup=visibleFixtures.map(fixture=>{
    const x=remoteFinite(fixture.position&&fixture.position.x,0);
    const z=remoteFinite(fixture.position&&fixture.position.z,0);
    const yaw=remoteFinite(fixture.rotation&&fixture.rotation.yaw,0);
    const angle=(-90+yaw)*(Math.PI/180);
    const intensity=remoteStageFixtureIntensity(fixture);
    const beamLength=fixtureRadius*(4+intensity*9);
    const beamX=x+Math.cos(angle)*beamLength;
    const beamZ=z+Math.sin(angle)*beamLength;
    const classes=["remoteStageFixture"];
    if(selectedFixtureId===fixture.id)classes.push("selected");
    const shapeClasses=["remoteStageFixtureShape"];
    if(fixture.highlighted)shapeClasses.push("highlighted");
    if(fixture.soloed)shapeClasses.push("soloed");
    if(fixture.parked)shapeClasses.push("parked");
    return `<g class="${classes.join(" ")}" onclick="selectFixture(${fixture.id})"><line class="remoteStageBeam" x1="${x}" y1="${z}" x2="${beamX}" y2="${beamZ}" /><circle class="${shapeClasses.join(" ")}" cx="${x}" cy="${z}" r="${fixtureRadius}"><title>${escapeHtml(fixture.label)} / U${fixture.universe} @ ${fixture.address}</title></circle><text class="remoteStageSurfaceLabel" style="font-size:${labelSize}px" x="${x+fixtureRadius*1.25}" y="${z-fixtureRadius*.7}">${escapeHtml(fixture.label)}</text></g>`;
  }).join("");
  svg.innerHTML=`<defs><pattern id="remote-stage-grid" width="${grid}" height="${grid}" patternUnits="userSpaceOnUse"><path d="M ${grid} 0 L 0 0 0 ${grid}" /></pattern></defs><rect class="remoteStageFloor" x="${bounds.minX}" y="${bounds.minZ}" width="${width}" height="${height}" /><rect class="remoteStageGrid" x="${bounds.minX}" y="${bounds.minZ}" width="${width}" height="${height}" fill="url(#remote-stage-grid)" /><line class="remoteStageAxis" x1="0" y1="${bounds.minZ}" x2="0" y2="${bounds.maxZ}" /><line class="remoteStageAxis" x1="${bounds.minX}" y1="0" x2="${bounds.maxX}" y2="0" />${objectMarkup}${outputMarkup}${fixtureMarkup}`;
}
function remoteVideoOutputMappingPanel(output){
  const mapping={...remoteDefaultMapping(),...(output.mapping||{})};
  const center=remoteProjectorCenter(mapping);
  const activeAspect=remoteFinite(mapping.aspect_ratio,1);
  const savedPresets=(((latestSnapshot||{}).video||{}).mapping_presets||[]);
  const savedPresetButtons=savedPresets.slice(0,8).map(preset=>`<button onclick="applyRemoteOutputMappingPreset(${output.id},${remoteJsString(preset.label)})">${escapeHtml(preset.label)}</button>`).join("");
  const aspectPresetButtons=remoteOutputAspectPresets.map(preset=>{
    const active=Math.abs(activeAspect-preset.ratio)<0.01&&mapping.aspect_mode==="Fit";
    return `<button class="${active?"active":""}" onclick="setRemoteOutputAspect(${output.id},${preset.ratio.toFixed(4)})">${preset.label}</button>`;
  }).join("");
  const keyPresetButtons=remoteProjectorKeystonePresets.map(preset=>{
    const active=Math.abs(remoteFinite(mapping.keystone_x,0)-preset.keystoneX)<0.01&&Math.abs(remoteFinite(mapping.keystone_y,0)-preset.keystoneY)<0.01;
    return `<button class="${active?"active":""}" onclick="setRemoteOutputKeystonePreset(${output.id},${preset.keystoneX},${preset.keystoneY})"><svg class="remoteMapKeyMini" viewBox="0 0 40 24" aria-hidden="true"><polygon points="${remoteProjectorKeystonePresetPoints(preset.keystoneX,preset.keystoneY)}" /></svg><span>${preset.label}</span></button>`;
  }).join("");
  return `<div class="remoteOutputMap">
    <svg class="remoteMapSurface" viewBox="0 0 100 100" role="img">
      <defs><pattern id="remote-output-map-grid-${output.id}" width="10" height="10" patternUnits="userSpaceOnUse"><path d="M 10 0 L 0 0 0 10" /></pattern></defs>
      <rect class="remoteMapFloor" x="0" y="0" width="100" height="100" />
      <rect class="remoteMapGrid" x="0" y="0" width="100" height="100" fill="url(#remote-output-map-grid-${output.id})" />
      <line class="remoteMapAxis" x1="50" y1="0" x2="50" y2="100" /><line class="remoteMapAxis" x1="0" y1="50" x2="100" y2="50" />
      <polygon class="remoteMapBase" points="${remoteProjectorPoints(mapping,true)}" />
      <polygon class="remoteMapWarp" points="${remoteProjectorPoints(mapping)}" />
      ${remoteProjectorKeystoneHandles.map(handle=>{
        const point=remoteProjectorKeystonePoint(mapping,handle.axis);
        return `<line class="remoteMapKeyGuide" x1="${handle.baseX}" y1="${handle.baseY}" x2="${point.x}" y2="${point.y}" /><circle class="remoteMapKeyHandle ${handle.axis}" cx="${point.x}" cy="${point.y}" r="3.8" onpointerdown="setRemoteProjectorKeystoneFromPointer(event,${output.id},'${handle.axis}')" onpointermove="dragRemoteProjectorKeystone(event,${output.id},'${handle.axis}')" onpointerup="releaseRemotePointer(event)" onpointercancel="releaseRemotePointer(event)"><title>${handle.label} ${escapeHtml(output.label)}</title></circle><text class="remoteMapKeyLabel" x="${point.x+4}" y="${point.y-4}">${handle.label}</text>`;
      }).join("")}
      <circle class="remoteMapCenterHandle" cx="${center.x}" cy="${center.y}" r="4.2" onpointerdown="setRemoteProjectorOffsetFromPointer(event,${output.id})" onpointermove="dragRemoteProjectorOffset(event,${output.id})" onpointerup="releaseRemotePointer(event)" onpointercancel="releaseRemotePointer(event)"><title>Move ${escapeHtml(output.label)}</title></circle>
      ${remoteProjectorCorners.map(corner=>{
        const base=remoteProjectorBasePoint(mapping,corner);
        const point=remoteProjectorPoint(mapping,corner);
        return `<line class="remoteMapHandleGuide" x1="${base.x}" y1="${base.y}" x2="${point.x}" y2="${point.y}" /><circle class="remoteMapBasePoint" cx="${base.x}" cy="${base.y}" r="1.6" /><circle class="remoteMapHandle" cx="${point.x}" cy="${point.y}" r="4" onpointerdown="setRemoteProjectorCornerFromPointer(event,${output.id},'${corner.key}')" onpointermove="dragRemoteProjectorCorner(event,${output.id},'${corner.key}')" onpointerup="releaseRemotePointer(event)" onpointercancel="releaseRemotePointer(event)"><title>${corner.label} ${escapeHtml(output.label)}</title></circle><text class="remoteMapHandleLabel" x="${point.x+5}" y="${point.y-5}">${corner.label}</text>`;
      }).join("")}
    </svg>
    <span>${escapeHtml(remoteMapReadout(mapping))}</span>
    <div class="remoteMapModeRow">
      <button class="${(mapping.aspect_mode||"Stretch")==="Stretch"?"active":""}" onclick="setRemoteOutputAspectMode(${output.id},'Stretch')">Stretch</button>
      <button class="${mapping.aspect_mode==="Fit"?"active":""}" onclick="setRemoteOutputAspectMode(${output.id},'Fit')">Fit</button>
      <button class="${mapping.aspect_mode==="Fill"?"active":""}" onclick="setRemoteOutputAspectMode(${output.id},'Fill')">Fill</button>
    </div>
    ${savedPresetButtons?`<div class="remoteMapPresetRow"><span class="small">Saved</span>${savedPresetButtons}</div>`:""}
    <div class="remoteMapPresetRow">${aspectPresetButtons}</div>
    <div class="remoteMapKeyPresetRow">${keyPresetButtons}</div>
    <div class="remoteMapTuningGrid">
      <label>Aspect<input type="range" min="0.25" max="4" step="0.01" value="${remoteClamp(remoteFinite(mapping.aspect_ratio,1),0.25,4)}" oninput="setRemoteOutputMappingField(${output.id},'aspect_ratio',this.value,0.25,4)"><strong>${remoteFinite(mapping.aspect_ratio,1).toFixed(2)}</strong></label>
      <label>Scale X<input type="range" min="0.25" max="2.5" step="0.01" value="${remoteClamp(remoteFinite(mapping.scale_x,1),0.25,2.5)}" oninput="setRemoteOutputMappingField(${output.id},'scale_x',this.value,0.25,2.5)"><strong>${remoteFinite(mapping.scale_x,1).toFixed(2)}</strong></label>
      <label>Scale Y<input type="range" min="0.25" max="2.5" step="0.01" value="${remoteClamp(remoteFinite(mapping.scale_y,1),0.25,2.5)}" oninput="setRemoteOutputMappingField(${output.id},'scale_y',this.value,0.25,2.5)"><strong>${remoteFinite(mapping.scale_y,1).toFixed(2)}</strong></label>
      <label>Rotate<input type="range" min="-180" max="180" step="0.5" value="${remoteClamp(remoteFinite(mapping.rotation_deg,0),-180,180)}" oninput="setRemoteOutputMappingField(${output.id},'rotation_deg',this.value,-180,180,1)"><strong>${remoteFinite(mapping.rotation_deg,0).toFixed(1)} deg</strong></label>
      <label>Lens<input type="range" min="-1" max="1" step="0.01" value="${remoteClamp(remoteFinite(mapping.lens_distortion,0),-1,1)}" oninput="setRemoteOutputMappingField(${output.id},'lens_distortion',this.value,-1,1)"><strong>${remoteFinite(mapping.lens_distortion,0).toFixed(2)}</strong></label>
    </div>
    <div class="remoteMapButtons">
      <button onclick="nudgeRemoteOutputMapping(${output.id},'offset_x',-0.02)">X -</button><button onclick="nudgeRemoteOutputMapping(${output.id},'offset_x',0.02)">X +</button>
      <button onclick="nudgeRemoteOutputMapping(${output.id},'offset_y',-0.02)">Y -</button><button onclick="nudgeRemoteOutputMapping(${output.id},'offset_y',0.02)">Y +</button>
      <button onclick="nudgeRemoteOutputMapping(${output.id},'keystone_x',-0.02)">Key H -</button><button onclick="nudgeRemoteOutputMapping(${output.id},'keystone_x',0.02)">Key H +</button>
      <button onclick="nudgeRemoteOutputMapping(${output.id},'keystone_y',-0.02)">Key V -</button><button onclick="nudgeRemoteOutputMapping(${output.id},'keystone_y',0.02)">Key V +</button>
      <button onclick="nudgeRemoteOutputMapping(${output.id},'stage_x',-0.1)">Stage X -</button><button onclick="nudgeRemoteOutputMapping(${output.id},'stage_x',0.1)">Stage X +</button>
      <button onclick="nudgeRemoteOutputMapping(${output.id},'stage_y',-0.1)">Stage Y -</button><button onclick="nudgeRemoteOutputMapping(${output.id},'stage_y',0.1)">Stage Y +</button>
      <button onclick="nudgeRemoteOutputMapping(${output.id},'stage_z',-0.1)">Stage Z -</button><button onclick="nudgeRemoteOutputMapping(${output.id},'stage_z',0.1)">Stage Z +</button>
      <button onclick="fitRemoteOutputMapping(${output.id})">Output Fit</button><button onclick="setRemoteOutputAspect(${output.id},1)">Square</button>
      <button onclick="clearRemoteOutputKeystone(${output.id})">Clear Key</button><button onclick="resetRemoteOutputPose(${output.id})">Reset Pose</button><button onclick="resetRemoteOutputStage(${output.id})">Reset Stage</button>
    </div>
  </div>`;
}
function renderSubmasters(submasters){
  const el=document.getElementById("submasterList");
  el.innerHTML=submasters.length?submasters.map(s=>`<div class="row"><div><strong>${escapeHtml(s.label)}</strong><label><span class="small">Submaster</span><input type="range" min="0" max="1" step="0.01" value="${Number(s.level??1)}" data-group="${escapeHtml(s.group_id)}" oninput="setGroupSubmasterFromInput(this)"></label></div><div class="inline"><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupHighlightFromButton(this,true)">Hi</button><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupHighlightFromButton(this,false)">-Hi</button><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupSoloFromButton(this,true)">Solo</button><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupSoloFromButton(this,false)">-Solo</button><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupParkFromButton(this,true)">Park</button><button data-group="${escapeHtml(s.group_id)}" onclick="setGroupParkFromButton(this,false)">-Park</button></div></div>`).join(""):`<span class="small">No fixture groups</span>`;
}
function remoteDefaultTouchSurface(snapshot){
  const cueControls=((snapshot&&snapshot.cues)||[]).slice(0,4).map((cue,index)=>({id:15+index,kind:"Button",x:index*3,y:6,w:3,h:2,label:cue.label,binding:{kind:"cue",cue_id:cue.id}}));
  return {pages:[{id:1,label:"Default Desk",controls:[
    {id:1,kind:"Label",x:0,y:0,w:2,h:1,label:"SHOW CONTROL",binding:null},
    {id:2,kind:"Image",x:0,y:1,w:2,h:2,label:"Touch Stage",binding:null},
    {id:3,kind:"Button",x:2,y:0,w:2,h:2,label:"BACK",binding:{kind:"cue_previous"}},
    {id:4,kind:"Button",x:4,y:0,w:3,h:2,label:"GO",binding:{kind:"cue_next"}},
    {id:6,kind:"Fader",x:9,y:0,w:1,h:4,label:"LIGHT",binding:{kind:"lighting_master"}},
    {id:11,kind:"ColorWheel",x:4,y:2,w:3,h:3,label:"COLOR",binding:{kind:"selected_fixture_color"}},
    {id:12,kind:"XyGrid",x:7,y:2,w:2,h:3,label:"POSITION",binding:{kind:"selected_fixture_pan_tilt",pan_attribute:"Pan",tilt_attribute:"Tilt"}},
    {id:14,kind:"Button",x:10,y:4,w:1,h:2,label:"VIDEO BO",binding:{kind:"video_blackout"}},
    ...cueControls
  ]}]};
}
function remoteEffectiveTouchSurface(snapshot){
  const surface=snapshot&&snapshot.touch_surface;
  return surface&&Array.isArray(surface.pages)&&surface.pages.length?surface:remoteDefaultTouchSurface(snapshot);
}
function remoteTouchActivePage(snapshot){
  const surface=remoteEffectiveTouchSurface(snapshot||latestSnapshot||{});
  const page=surface.pages.find(candidate=>candidate.id===remoteTouchPageId)||surface.pages[0];
  remoteTouchPageId=page?page.id:null;
  return page;
}
function remoteTouchControl(control_id){
  return remoteTouchActivePage(latestSnapshot||{})?.controls.find(control=>control.id===Number(control_id))||null;
}
function remoteTouchBindingValue(binding){
  if(!binding||!latestSnapshot)return 0;
  if(binding.kind==="lighting_master")return Number(latestSnapshot.lighting_master??1);
  if(binding.kind==="video_master")return Number((latestSnapshot.video&&latestSnapshot.video.master_opacity)??1);
  if(binding.kind==="group_submaster")return Number((latestSnapshot.submasters||[]).find(entry=>entry.group_id===binding.group_id)?.level??1);
  let fixture=null;
  if(binding.kind==="fixture_attribute")fixture=(latestSnapshot.fixtures||[]).find(candidate=>candidate.id===binding.fixture_id);
  if(binding.kind==="group_attribute")fixture=(latestSnapshot.fixtures||[]).find(candidate=>(candidate.group_ids||[]).includes(binding.group_id));
  if(binding.kind==="selected_fixture_attribute")fixture=remoteControlFixture();
  return fixture&&binding.attribute?normalizedAttributeValue(fixture,binding.attribute):0;
}
function remoteTouchSetPage(page_id){remoteTouchPageId=Number(page_id);renderRemoteTouchSurface(latestSnapshot||{})}
function remoteTouchTrigger(control_id){
  const binding=remoteTouchControl(control_id)?.binding;
  if(!binding)return;
  if(binding.kind==="cue")send({type:"triggerCue",cue_id:binding.cue_id});
  else if(binding.kind==="cue_next")send({type:"triggerNextCue"});
  else if(binding.kind==="cue_previous")send({type:"triggerPreviousCue"});
  else if(binding.kind==="cue_fade_pause")send({type:"setCueFadePaused",paused:!(latestSnapshot.active_fade&&latestSnapshot.active_fade.paused)});
  else if(binding.kind==="blackout")send({type:"blackout",enabled:!latestSnapshot.blackout});
  else if(binding.kind==="video_blackout")send({type:"videoBlackout",enabled:!(latestSnapshot.video&&latestSnapshot.video.blackout)});
  else if(binding.kind==="all_blackout")setAllBlackout(!(latestSnapshot.blackout&&latestSnapshot.video&&latestSnapshot.video.blackout));
}
function remoteTouchSetValue(control_id,rawValue){
  const binding=remoteTouchControl(control_id)?.binding;
  const value=Math.max(0,Math.min(1,Number(rawValue)||0));
  if(!binding)return;
  if(binding.kind==="fixture_attribute")send({type:"setAttribute",fixture_id:binding.fixture_id,attribute:binding.attribute,value});
  else if(binding.kind==="group_attribute")send({type:"setGroupAttribute",group_id:binding.group_id,attribute:binding.attribute,value});
  else if(binding.kind==="selected_fixture_attribute"){
    const fixture=remoteControlFixture();const group_id=selectedGroupId();
    if(group_id)send({type:"setGroupAttribute",group_id,attribute:binding.attribute,value});
    else if(fixture)send({type:"setAttribute",fixture_id:fixture.id,attribute:binding.attribute,value});
  }else if(binding.kind==="group_submaster")send({type:"setGroupSubmaster",group_id:binding.group_id,level:value});
  else if(binding.kind==="lighting_master")send({type:"lightingMaster",master:value});
  else if(binding.kind==="video_master")send({type:"videoMaster",opacity:value});
}
function remoteTouchSetColor(control_id,hex){
  const binding=remoteTouchControl(control_id)?.binding;
  if(!binding||!/^#[0-9a-fA-F]{6}$/.test(String(hex)))return;
  let fixture=null;let group_id=null;
  if(binding.kind==="fixture_color")fixture=(latestSnapshot.fixtures||[]).find(candidate=>candidate.id===binding.fixture_id);
  else if(binding.kind==="group_color"){group_id=binding.group_id;fixture=(latestSnapshot.fixtures||[]).find(candidate=>(candidate.group_ids||[]).includes(group_id));}
  else if(binding.kind==="selected_fixture_color"){fixture=remoteControlFixture();group_id=selectedGroupId();}
  const controls=remoteColorControls(fixture);if(!fixture||!controls)return;
  const values=[[controls.red,Number.parseInt(hex.slice(1,3),16)/255],[controls.green,Number.parseInt(hex.slice(3,5),16)/255],[controls.blue,Number.parseInt(hex.slice(5,7),16)/255]];
  for(const [attribute,value] of values){
    if(group_id)send({type:"setGroupAttribute",group_id,attribute,value},false);
    else send({type:"setAttribute",fixture_id:fixture.id,attribute,value},false);
  }
  queueSnapshot(160);
}
function remoteTouchControlHtml(control){
  const style=`grid-column:${Number(control.x)+1}/span ${Math.max(1,Number(control.w)||1)};grid-row:${Number(control.y)+1}/span ${Math.max(1,Number(control.h)||1)}`;
  const label=escapeHtml(control.label||control.kind||"Control");
  const disabled=control.binding?"":" disabled";
  let body="";
  if(control.kind==="Label")body=`<strong class="remoteTouchLabel">${label}</strong>`;
  else if(control.kind==="Image")body=`<div class="remoteTouchImage" role="img" aria-label="${label}">▧<strong>${label}</strong></div>`;
  else if(control.kind==="Button")body=`<button class="remoteTouchButton ${control.binding&&control.binding.kind==="cue_next"?"primary":""}"${disabled} onclick="remoteTouchTrigger(${Number(control.id)})">${label}</button>`;
  else if(control.kind==="Fader")body=`<label class="remoteTouchFader"><span>${label}</span><input aria-label="${label}" type="range" min="0" max="1" step="0.01" value="${remoteTouchBindingValue(control.binding)}"${disabled} oninput="remoteTouchSetValue(${Number(control.id)},this.value)"><strong>${Math.round(remoteTouchBindingValue(control.binding)*100)}%</strong></label>`;
  else if(control.kind==="ColorWheel")body=`<label class="remoteTouchColor"><span>${label}</span><input aria-label="${label}" type="color" value="#ff7a00"${disabled} oninput="remoteTouchSetColor(${Number(control.id)},this.value)"></label>`;
  else body=`<div class="remoteTouchUnsupported"><strong>${label}</strong><span class="small">${escapeHtml(control.kind)}</span></div>`;
  return `<div class="remoteTouchControl kind-${escapeHtml(control.kind)}" style="${style}" data-remote-touch-kind="${escapeHtml(control.kind)}">${body}</div>`;
}
function renderRemoteTouchSurface(snapshot){
  const surface=remoteEffectiveTouchSurface(snapshot);
  const page=remoteTouchActivePage(snapshot);
  const pages=document.getElementById("remoteTouchPages");
  const grid=document.getElementById("remoteTouchSurface");
  if(!pages||!grid||!page)return;
  text("remoteTouchPageLabel",page.label||"Touch page");
  pages.innerHTML=surface.pages.map(candidate=>`<button class="${candidate.id===page.id?"active":""}" onclick="remoteTouchSetPage(${Number(candidate.id)})">${escapeHtml(candidate.label)}</button>`).join("");
  grid.innerHTML=(page.controls||[]).map(remoteTouchControlHtml).join("")||`<span class="small">No controls on this Touch page</span>`;
}
function renderTimeline(timeline){
  const duration=timeline.duration_ms||0;
  const position=timeline.position_ms||0;
  const audio=timeline.audio||null;
  const beatCount=audio&&Array.isArray(audio.beats)?audio.beats.length:0;
  const bpm=audio&&Number.isFinite(audio.estimated_bpm)?audio.estimated_bpm:null;
  document.getElementById("timelineSeekMs").value=position;
  document.getElementById("timelineProgress").style.width=`${duration>0?Math.max(0,Math.min(100,(position/duration)*100)):0}%`;
  text("timelineInfo",`${timeline.playing?"Playing":"Stopped"} / ${position}ms / ${duration}ms`);
  text("timelineBeatInfo",beatCount?`${beatCount} detected beat(s)${bpm?` / ${bpm.toFixed(1)} BPM`:""}`:"Beat grid from clock BPM");
  document.getElementById("timelineEvents").innerHTML=(timeline.events||[]).map(e=>`<div class="row"><div><strong>${e.time_ms}ms</strong><span class="small">${escapeHtml(e.track)} cue ${e.cue_id}</span></div><div class="inline"><button onclick="seekTimelineRemote(${e.time_ms})">Seek</button><button onclick="send({type:'triggerCue',cue_id:${e.cue_id}})">GO</button></div></div>`).join("");
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
function formatRemoteMicros(value){
  const rounded=Math.round(Number(value)||0);
  const abs=Math.abs(rounded);
  if(abs>=10000)return `${Math.round(rounded/1000)} ms`;
  if(abs>=1000)return `${(rounded/1000).toFixed(1)} ms`;
  return `${rounded} us`;
}
function setRemoteTile(prefix,state){
  text(`${prefix}State`,state.label);
  text(`${prefix}Detail`,state.detail);
}
function remoteRtHealth(telemetry){
  const samples=Number(telemetry.tick_jitter_samples||0);
  const queueMax=Number(telemetry.queue_depth_abs_max||0);
  const failures=Number(telemetry.queue_push_failure_count||0);
  const drainLimitHits=Number(telemetry.command_drain_limit_hit_count||0);
  const jitterP95=Number(telemetry.tick_jitter_p95_us||0);
  const jitterP99=Number(telemetry.tick_jitter_p99_us||0);
  const cmdQueueP99=Number(telemetry.command_queue_latency_p99_us||0);
  const cmdDmxP99=Number(telemetry.command_to_dmx_tick_latency_p99_us||0);
  const lowLatencyRequests=Number(telemetry.low_latency_dmx_tick_request_count||0);
  const lowLatencyDetail=lowLatencyRequests>0
    ? ` / lowlat ${telemetry.low_latency_dmx_tick_advance_count||0}/${lowLatencyRequests}/${telemetry.low_latency_dmx_tick_defer_count||0}`
    : "";
  const detail=`q ${telemetry.queue_depth||0}/${queueMax} / cmd ${formatRemoteMicros(cmdQueueP99)} / dmx ${formatRemoteMicros(cmdDmxP99)} / jit ${formatRemoteMicros(jitterP99)}${lowLatencyDetail}`;
  if(samples<3)return {label:"RT WARMUP",detail:"Waiting for timing samples"};
  if(failures>0||cmdDmxP99>16000||jitterP99>8000||queueMax>1024)return {label:"RT BAD",detail};
  if(cmdDmxP99>5000||cmdQueueP99>1000||drainLimitHits>0||jitterP95>1000||queueMax>256)return {label:"RT WATCH",detail};
  return {label:"RT OK",detail};
}
function remoteLatencyHealth(telemetry){
  const cmdQueueP99=Number(telemetry.command_queue_latency_p99_us||0);
  const cmdDmxP99=Number(telemetry.command_to_dmx_tick_latency_p99_us||0);
  const queueSamples=Number(telemetry.command_queue_latency_samples||0);
  const dmxSamples=Number(telemetry.command_to_dmx_tick_latency_samples||0);
  const detail=`cmd ${formatRemoteMicros(cmdQueueP99)} / dmx ${formatRemoteMicros(cmdDmxP99)} p99`;
  if(queueSamples===0&&dmxSamples===0)return {label:"LAT WAIT",detail:"No command samples"};
  if(cmdDmxP99>16000)return {label:"LAT BAD",detail};
  if(cmdDmxP99>5000||cmdQueueP99>1000)return {label:"LAT WATCH",detail};
  return {label:"LAT OK",detail};
}
function remoteDmxLastFrameError(telemetry){
  const routeResults=Array.isArray(telemetry.last_dmx_route_results)?telemetry.last_dmx_route_results:[];
  const routeErrors=routeResults
    .filter(result=>result&&result.attempted&&!result.success)
    .map(result=>result.error||`Route ${Number(result.index||0)+1} failed`);
  if(routeErrors.length)return routeErrors.join(" / ");
  return Number(telemetry.last_dmx_send_failure_count||0)>0
    ? String(telemetry.last_error||"A DMX route failed in the last frame")
    : "";
}
function remoteDmxHealth(snapshot,telemetry){
  const routes=(snapshot.dmx_outputs||[]).filter(route=>route.enabled);
  const failures=Number(telemetry.last_dmx_send_failure_count||0);
  const success=Number(telemetry.last_dmx_send_success_count||0);
  const sent=Number(telemetry.last_dmx_output_count||0);
  const interval=Number(telemetry.last_dmx_send_interval_us||0);
  const detail=`${success}/${sent} routes / ${telemetry.last_packet_bytes||0} bytes${interval?` / ${formatRemoteMicros(interval)}`:""}`;
  if(!routes.length)return {label:"DMX OFF",detail:"No enabled output routes"};
  const dmxError=remoteDmxLastFrameError(telemetry);
  if(dmxError||failures>0)return {label:"DMX FAIL",detail:dmxError||detail};
  if(sent===0)return {label:"DMX WAIT",detail:"Route enabled, waiting for first frame"};
  return {label:"DMX OK",detail};
}
function remoteDmxProtocolLabel(protocol){
  if(protocol==="ArtNet")return "Art-Net";
  if(protocol==="Sacn")return "sACN";
  if(protocol==="EnttecUsbPro")return "Enttec Pro";
  if(protocol==="DmxKingUltraDmx")return "DMXKing ultraDMX";
  if(protocol==="EnttecOpenDmx")return "Enttec Open";
  return String(protocol||"DMX");
}
function remoteDmxRouteEndpoint(route){
  if(route.protocol==="EnttecUsbPro"||route.protocol==="DmxKingUltraDmx"||route.protocol==="EnttecOpenDmx"){
    return `${route.serial_port||"(no port)"} @ ${route.protocol==="EnttecOpenDmx"?250000:(route.serial_baud_rate||57600)}`;
  }
  return `${route.target_ip||"0.0.0.0"}:${route.port||6454}`;
}
function renderRemoteDmxRoutes(snapshot,telemetry){
  const el=document.getElementById("remoteDmxRoutes");
  if(!el)return;
  const routes=snapshot.dmx_outputs||[];
  if(!routes.length){el.innerHTML=`<span class="small">No DMX output routes</span>`;return}
  const failures=Number(telemetry.last_dmx_send_failure_count||0);
  const sent=Number(telemetry.last_dmx_output_count||0);
  const success=Number(telemetry.last_dmx_send_success_count||0);
  const routeResults=Array.isArray(telemetry.last_dmx_route_results)?telemetry.last_dmx_route_results:[];
  const error=failures>0&&telemetry.last_error?String(telemetry.last_error):"";
  el.innerHTML=routes.map((route,index)=>{
    const enabled=route.enabled!==false;
    const result=routeResults.find(candidate=>Number(candidate.index)===index);
    const routeError=result&&result.error?String(result.error):"";
    const failed=enabled&&result?result.attempted&&!result.success:enabled&&(failures>0||error);
    const className=failed?"fail":enabled?"live":"off";
    const status=!enabled
      ?"Disabled"
      :result
        ?result.attempted
          ?result.success
            ?`${Number(result.bytes)||0} bytes`
            :"Fail"
          :"Skipped"
        :sent>0?`${success}/${sent} sent`:"Waiting";
    const detailError=routeError||error;
    return `<div class="remoteDmxRoute ${className}"><strong>${remoteDmxProtocolLabel(route.protocol)} U${Number(route.universe)||0}</strong><span>${escapeHtml(remoteDmxRouteEndpoint(route))}</span><small>${index+1}. ${status}${detailError?` / ${escapeHtml(detailError)}`:""}</small></div>`;
  }).join("");
}
function remoteBackendClass(state){
  const normalized=String(state||"").toLowerCase();
  if(normalized==="available")return "available";
  if(normalized==="missing")return "missing";
  return "notbuilt";
}
function renderVideoRuntimeStatus(status){
  latestVideoRuntimeStatus=status||{backends:[]};
  const backends=Array.isArray(latestVideoRuntimeStatus.backends)?latestVideoRuntimeStatus.backends:[];
  const el=document.getElementById("videoRuntimeBackends");
  const available=backends.filter(backend=>backend.state==="Available").length;
  const missing=backends.filter(backend=>backend.state==="Missing").length;
  const notBuilt=backends.filter(backend=>backend.state==="NotBuilt").length;
  text("videoRuntimeSummary",backends.length?`${available} available / ${missing} missing / ${notBuilt} not built`:"No backend status");
  el.innerHTML=backends.length?backends.map(backend=>{
    const className=remoteBackendClass(backend.state);
    const badgeClass=className==="available"?"ok":className==="missing"?"bad":"warn";
    return `<div class="liveTile backendTile ${className}"><span class="status ${badgeClass}">${escapeHtml(backend.state)}</span><strong>${escapeHtml(backend.label||backend.id)}</strong><small>${escapeHtml(backend.detail||"")}</small></div>`;
  }).join(""):`<span class="small">Backend status is not loaded</span>`;
}
function remoteVideoOutputRenderPlanPayload(payload){
  if(Array.isArray(payload))return {plans:payload,error:""};
  const plans=payload&&Array.isArray(payload.plans)?payload.plans:[];
  return {plans,error:payload&&payload.error?String(payload.error):""};
}
function remoteVideoOutputMappingLabel(mapping){
  const m=mapping||{};
  const cornerWarp=Math.abs(Number(m.corner_top_left_x)||0)+Math.abs(Number(m.corner_top_left_y)||0)+Math.abs(Number(m.corner_top_right_x)||0)+Math.abs(Number(m.corner_top_right_y)||0)+Math.abs(Number(m.corner_bottom_right_x)||0)+Math.abs(Number(m.corner_bottom_right_y)||0)+Math.abs(Number(m.corner_bottom_left_x)||0)+Math.abs(Number(m.corner_bottom_left_y)||0);
  const parts=[];
  if(m.aspect_mode&&m.aspect_mode!=="Stretch")parts.push(String(m.aspect_mode));
  if(Math.abs(Number(m.lens_distortion)||0)>0.001)parts.push(`lens ${Number(m.lens_distortion).toFixed(2)}`);
  if(Math.abs(Number(m.keystone_x)||0)>0.001||Math.abs(Number(m.keystone_y)||0)>0.001)parts.push(`key ${Number(m.keystone_x||0).toFixed(2)},${Number(m.keystone_y||0).toFixed(2)}`);
  if(cornerWarp>0.001)parts.push("corner warp");
  return parts.length?parts.join(" / "):"flat mapping";
}
function remoteVideoOutputRenderPlanById(output_id){
  return (latestVideoOutputRenderPlans||[]).find(plan=>Number(plan.output_id)===Number(output_id))||null;
}
function remoteVideoOutputRenderPlanState(plan,output){
  const layers=((plan&&plan.composition||{}).layers||[]);
  const enabled=plan?plan.enabled!==false:output&&output.enabled!==false;
  const blackout=plan?plan.output_blackout:output&&output.blackout;
  if(!enabled)return {label:"Disabled",className:"notbuilt",badgeClass:"warn",layers};
  if(blackout)return {label:"Blackout",className:"missing",badgeClass:"bad",layers};
  if(layers.length)return {label:"Live",className:"available",badgeClass:"ok",layers};
  return {label:"Empty",className:"notbuilt",badgeClass:"warn",layers};
}
function remoteVideoOutputRenderPlanDetail(plan,output){
  if(!plan)return "Render plan not loaded";
  const state=remoteVideoOutputRenderPlanState(plan,output);
  const composition=(plan.composition||{}).label||`Composition ${(plan.composition||{}).composition_id||""}`;
  const mapping=remoteVideoOutputMappingLabel(plan.mapping);
  const layers=state.layers.length?state.layers.slice(0,3).map(layer=>`${layer.label||`Layer ${layer.layer_id}`} ${Math.round((Number(layer.opacity)||0)*100)}%`).join(", "):"No active layers";
  const more=state.layers.length>3?` +${state.layers.length-3} more`:"";
  return `${state.label} / ${composition} / ${state.layers.length} layer(s) / ${Math.round((Number(plan.output_opacity)||0)*100)}% / ${mapping} / ${layers}${more}`;
}
function renderVideoOutputRenderPlans(payload){
  const normalized=remoteVideoOutputRenderPlanPayload(payload);
  latestVideoOutputRenderPlans=normalized.plans;
  const plans=latestVideoOutputRenderPlans;
  const activeOutputs=plans.filter(plan=>plan.enabled!==false&&!plan.output_blackout).length;
  const blackoutOutputs=plans.filter(plan=>plan.output_blackout).length;
  const activeLayers=plans.reduce((total,plan)=>total+(((plan.composition||{}).layers||[]).length),0);
  const errorText=normalized.error?` / ${normalized.error}`:"";
  text("videoOutputRenderPlanSummary",plans.length?`${plans.length} output(s) / ${activeOutputs} active / ${activeLayers} render layer(s) / ${blackoutOutputs} blackout${errorText}`:normalized.error?normalized.error:"No render plans");
  const el=document.getElementById("videoOutputRenderPlans");
  el.innerHTML=plans.length?plans.map(plan=>{
    const planState=remoteVideoOutputRenderPlanState(plan,null);
    const state=planState.label;
    const className=planState.className;
    const badgeClass=planState.badgeClass;
    const endpoint=plan.kind==="Display"?`Monitor ${plan.monitor_id??0}`:(plan.endpoint_name||plan.label||"");
    const composition=(plan.composition||{}).label||`Composition ${(plan.composition||{}).composition_id||""}`;
    const layers=planState.layers;
    const layerText=layers.length?layers.slice(0,3).map(layer=>`${layer.label||`Layer ${layer.layer_id}`} ${Math.round((Number(layer.opacity)||0)*100)}%`).join(", "):"No active layers";
    const more=layers.length>3?` +${layers.length-3} more`:"";
    return `<div class="liveTile backendTile ${className}"><span class="status ${badgeClass}">${escapeHtml(state)} ${escapeHtml(plan.kind||"Output")}</span><strong>${escapeHtml(plan.label||`Output ${plan.output_id}`)}</strong><small>${escapeHtml(endpoint)} / ${Number(plan.width)||0}x${Number(plan.height)||0}</small><small>${escapeHtml(composition)} / ${Math.round((Number(plan.output_opacity)||0)*100)}% / ${escapeHtml(remoteVideoOutputMappingLabel(plan.mapping))}</small><small>${escapeHtml(layerText+more)}</small></div>`;
  }).join(""):`<span class="small">No video output render plans</span>`;
  const video=(latestSnapshot&&latestSnapshot.video)||null;
  if(video)renderVideoOutputList(video.outputs||[],video.compositions||[]);
  if(latestSnapshot)renderRemoteStage(latestSnapshot);
}
function renderExternalVideoIoPlans(plans){
  latestExternalVideoIoPlans=plans||{inputs:[],outputs:[]};
  const inputs=Array.isArray(latestExternalVideoIoPlans.inputs)?latestExternalVideoIoPlans.inputs:[];
  const outputs=Array.isArray(latestExternalVideoIoPlans.outputs)?latestExternalVideoIoPlans.outputs:[];
  const routes=[
    ...inputs.map(plan=>({...plan,direction:"IN",route_id:plan.layer_id,output:false})),
    ...outputs.map(plan=>({...plan,direction:"OUT",route_id:plan.output_id,output:true}))
  ];
  const live=routes.filter(route=>route.live).length;
  const blocked=routes.filter(route=>!route.ready).length;
  const el=document.getElementById("externalVideoIoPlanRoutes");
  text("externalVideoIoPlanSummary",routes.length?`${routes.length} planned / ${live} live / ${blocked} blocked`:"No external video I/O plans");
  el.innerHTML=routes.length?routes.map(route=>{
    const className=route.ready?(route.live?"available":"notbuilt"):"missing";
    const badgeClass=route.ready?(route.live?"ok":"warn"):"bad";
    const state=route.live?"Live":route.ready?(route.enabled?"Ready":"Disabled"):(route.backend_state||"Blocked");
    const backend=String(route.backend_id||"").toUpperCase();
    const detail=route.issue||route.backend_detail||`${route.kind||""}${route.output&&route.width&&route.height?` / ${route.width}x${route.height}`:""}`;
    return `<div class="liveTile backendTile ${className}"><span class="status ${badgeClass}">${escapeHtml(route.direction)} ${escapeHtml(backend)} ${escapeHtml(state)}</span><strong>${escapeHtml(route.label||`Route ${route.route_id}`)}</strong><small>${escapeHtml(route.endpoint_name||"")}</small><small>${escapeHtml(detail||"")}</small></div>`;
  }).join(""):`<span class="small">No external video I/O plans</span>`;
}
function renderExternalVideoTransportStatus(status){
  latestExternalVideoTransportStatus=status||{active_routes:[],active_count:0};
  const routes=Array.isArray(latestExternalVideoTransportStatus.active_routes)?latestExternalVideoTransportStatus.active_routes:[];
  const activeCount=Number.isFinite(Number(latestExternalVideoTransportStatus.active_count))?Number(latestExternalVideoTransportStatus.active_count):routes.length;
  const el=document.getElementById("externalVideoTransportRoutes");
  text("externalVideoTransportSummary",routes.length?`${activeCount} active route(s)`:"No active external routes");
  el.innerHTML=routes.length?routes.map(route=>{
    const direction=String(route.direction||"").toUpperCase();
    const backend=String(route.backend_id||"").toUpperCase();
    const label=route.label||`Route ${route.route_id}`;
    const endpoint=route.endpoint_name||"";
    return `<div class="liveTile backendTile available"><span class="status ok">${escapeHtml(direction)} ${escapeHtml(backend)}</span><strong>${escapeHtml(label)}</strong><small>${escapeHtml(endpoint)}</small></div>`;
  }).join(""):`<span class="small">No active external video transport routes</span>`;
}
function renderExternalVideoTransportSync(sync){
  latestExternalVideoTransportSync=sync||{report:null,events:[]};
  const report=latestExternalVideoTransportSync.report||{};
  const events=Array.isArray(latestExternalVideoTransportSync.events)?latestExternalVideoTransportSync.events:[];
  const count=(key)=>Array.isArray(report[key])?report[key].length:0;
  const active=Number.isFinite(Number(report.active_count))?Number(report.active_count):0;
  const failed=count("start_failed")+count("stop_failed");
  const failedText=failed?` / failed ${failed}`:"";
  const hasReport=latestExternalVideoTransportSync.report&&typeof latestExternalVideoTransportSync.report==="object";
  text("externalVideoTransportSyncSummary",hasReport?`sync active ${active} / +${count("started")} / =${count("kept")} / -${count("stopped")} / blocked ${count("blocked")}${failedText}`:"Routes not synced");
  const el=document.getElementById("externalVideoTransportSyncEvents");
  el.innerHTML=events.length?events.slice(-6).map(event=>{
    const route=event.route||{};
    const action=String(event.action||"Event");
    const badgeClass=action==="Start"?"ok":"warn";
    const backend=String(route.backend_id||"").toUpperCase();
    return `<div class="liveTile backendTile available"><span class="status ${badgeClass}">${escapeHtml(action)} ${escapeHtml(route.direction||"")}</span><strong>${escapeHtml(route.label||`Route ${route.route_id||""}`)}</strong><small>${escapeHtml(backend)} / ${escapeHtml(route.endpoint_name||"")}</small><small>${escapeHtml(event.message||"")}</small></div>`;
  }).join(""):`<span class="small">No sync driver events</span>`;
}
function renderStatus(snapshot,video){
  const clock=snapshot.clock||{};
  const telemetry=snapshot.telemetry||{};
  text("clockInfo",`${escapeHtml(clock.source||"Manual")} / beat ${clock.beat_counter||0} / phase ${Number(clock.beat_phase||0).toFixed(3)}`);
  text("masterInfo",`${snapshot.blackout?"Lighting blackout":"Lighting live"} / lighting master ${Math.round((snapshot.lighting_master??1)*100)}% / ${video.blackout?"Video blackout":"Video live"} / video master ${Math.round((video.master_opacity??1)*100)}%`);
  setRemoteTile("remoteRt",remoteRtHealth(telemetry));
  setRemoteTile("remoteDmx",remoteDmxHealth(snapshot,telemetry));
  setRemoteTile("remoteLatency",remoteLatencyHealth(telemetry));
}
function formatRemoteTime(ms){
  const value=Math.max(0,Math.round(Number(ms)||0));
  const minutes=Math.floor(value/60000);
  const seconds=Math.floor((value%60000)/1000);
  const millis=value%1000;
  return `${minutes}:${String(seconds).padStart(2,"0")}.${String(millis).padStart(3,"0")}`;
}
function videoCuePointsForState(state){
  const detailed=Array.isArray(state.cue_points)?state.cue_points:[];
  const legacy=Array.isArray(state.cue_points_ms)?state.cue_points_ms:[];
  const source=detailed.length?detailed:legacy.map((position_ms,index)=>({position_ms,label:`Cue ${index+1}`,color:"#4aa8ff"}));
  return source.map((cue,index)=>({
    position_ms:Math.max(0,Math.round(Number(cue.position_ms)||0)),
    label:String(cue.label||`Cue ${index+1}`),
    color:/^#[0-9a-fA-F]{6}$/.test(String(cue.color||""))?String(cue.color):"#4aa8ff"
  })).sort((a,b)=>a.position_ms-b.position_ms);
}
function selectFixture(id){document.getElementById("fixtureId").value=String(id);applyAttributeOptions();renderRemoteStage(latestSnapshot||{})}
function remoteFixtureById(fixture_id){
  return (latestSnapshot&&latestSnapshot.fixtures||[]).find(fixture=>fixture.id===fixture_id)||null;
}
function setRemotePanTiltValues(fixture,panValue,tiltValue){
  const controls=remotePositionControls(fixture);
  if(!fixture||!controls)return;
  const source=remoteSourcePanTiltValues(fixture,panValue,tiltValue);
  sendFixtureDmxValue(fixture,controls.pan,source.pan);
  sendFixtureDmxValue(fixture,controls.tilt,source.tilt);
}
function setRemotePanTiltFromPointer(event){
  const fixture=remoteFixtureById(Number(event.currentTarget.dataset.fixture));
  if(!fixture)return;
  event.preventDefault();
  event.currentTarget.setPointerCapture(event.pointerId);
  const bounds=event.currentTarget.getBoundingClientRect();
  const x=Math.max(0,Math.min(1,(event.clientX-bounds.left)/bounds.width));
  const y=Math.max(0,Math.min(1,(event.clientY-bounds.top)/bounds.height));
  setRemotePanTiltValues(fixture,Math.round(x*65535),Math.round((1-y)*65535));
}
function dragRemotePanTilt(event){
  if(event.buttons!==1)return;
  setRemotePanTiltFromPointer(event);
}
function releaseRemotePointer(event){
  if(event.currentTarget.hasPointerCapture(event.pointerId))event.currentTarget.releasePointerCapture(event.pointerId);
}
function setRemotePanTiltTarget(fixture_id,panRatio,tiltRatio){
  const fixture=remoteFixtureById(fixture_id);
  if(!fixture)return;
  const target=remotePanTiltTargetValues(fixture,panRatio,tiltRatio);
  setRemotePanTiltValues(fixture,target.pan,target.tilt);
}
function applyRemotePositionFavorite(fixture_id,favorite_id){
  const fixture=remoteFixtureById(fixture_id);
  const favorite=remotePositionFavorites.find(candidate=>candidate.id===favorite_id);
  if(!fixture||!favorite)return;
  setRemotePanTiltValues(fixture,favorite.pan,favorite.tilt);
}
function addRemotePositionFavorite(fixture_id){
  const fixture=remoteFixtureById(fixture_id);
  const position=remotePositionControls(fixture);
  if(!fixture||!position)return;
  const fallback=`P${remotePositionFavorites.length+1}`;
  const prompted=window.prompt("Position label",fallback);
  if(prompted===null)return;
  const label=String(prompted||fallback).trim().slice(0,16)||fallback;
  const favorite={id:`${Date.now()}-${position.panValue}-${position.tiltValue}`,label,pan:remoteDmxClamp(position.panValue),tilt:remoteDmxClamp(position.tiltValue)};
  remotePositionFavorites=[
    favorite,
    ...remotePositionFavorites.filter(candidate=>!remotePositionFavoriteMatches({panValue:favorite.pan,tiltValue:favorite.tilt},candidate))
  ].slice(0,18);
  saveRemotePositionFavorites();
  renderRemoteVisualControls(remoteControlFixture());
}
function resetRemotePositionFavorites(){
  remotePositionFavorites=remoteDefaultPositionFavorites();
  saveRemotePositionFavorites();
  renderRemoteVisualControls(remoteControlFixture());
}
function setRemotePanTiltMirror(fixture_id,axis){
  const fixture=remoteFixtureById(fixture_id);
  const controls=remotePositionControls(fixture);
  if(!fixture||!controls)return;
  setRemotePanTiltValues(
    fixture,
    axis==="pan"||axis==="both"?65535-controls.panValue:controls.panValue,
    axis==="tilt"||axis==="both"?65535-controls.tiltValue:controls.tiltValue
  );
}
function setRemoteDimmerValue(fixture_id,value){
  const fixture=remoteFixtureById(fixture_id);
  const dimmer=remoteDimmerControl(fixture);
  if(!fixture||!dimmer)return;
  sendFixtureDmxValue(fixture,dimmer.attribute,remoteDimmerWithinLimits(fixture,value));
}
function setRemoteDimmerTarget(fixture_id,ratio){
  const fixture=remoteFixtureById(fixture_id);
  const dimmer=remoteDimmerControl(fixture);
  if(!fixture||!dimmer)return;
  sendFixtureDmxValue(fixture,dimmer.attribute,remoteDimmerTargetValue(fixture,ratio));
}
function sendRemoteDimmerBumpValue(target,value){
  if(!target||!target.attribute)return;
  const safe=remoteDmxClamp(value);
  if(target.group_id){
    send({type:"setGroupAttribute",group_id:target.group_id,attribute:target.attribute,value:safe},false);
  }else{
    send({type:"setAttribute",fixture_id:target.fixture_id,attribute:target.attribute,value:safe/65535},false);
  }
  queueSnapshot(120);
}
function startRemoteDimmerBump(event,fixture_id){
  if(event)event.preventDefault();
  const fixture=remoteFixtureById(fixture_id);
  const dimmer=remoteDimmerControl(fixture);
  if(!fixture||!dimmer)return;
  if(remoteDimmerBumpRestore){
    if(remoteDimmerBumpRestore.fixture_id===fixture.id&&remoteDimmerBumpRestore.attribute===dimmer.attribute)return;
    endRemoteDimmerBump(null,remoteDimmerBumpRestore.fixture_id);
  }
  const button=event&&event.currentTarget;
  if(button&&button.classList)button.classList.add("active");
  remoteDimmerBumpRestore={fixture_id:fixture.id,group_id:selectedGroupId(),attribute:dimmer.attribute,value:dimmer.value,button};
  sendRemoteDimmerBumpValue(remoteDimmerBumpRestore,remoteDimmerTargetValue(fixture,1));
}
function endRemoteDimmerBump(event,fixture_id){
  if(event)event.preventDefault();
  if(!remoteDimmerBumpRestore)return;
  if(fixture_id&&remoteDimmerBumpRestore.fixture_id!==fixture_id)return;
  const restore=remoteDimmerBumpRestore;
  remoteDimmerBumpRestore=null;
  if(restore.button&&restore.button.classList)restore.button.classList.remove("active");
  sendRemoteDimmerBumpValue(restore,restore.value);
}
function remoteDimmerBumpKeyDown(event,fixture_id){
  if(event.key===" "||event.key==="Enter"){
    startRemoteDimmerBump(event,fixture_id);
  }
}
function remoteDimmerBumpKeyUp(event,fixture_id){
  if(event.key===" "||event.key==="Enter"){
    endRemoteDimmerBump(event,fixture_id);
  }
}
function setRemoteChannelFunction(fixture_id,attribute,value){
  const fixture=remoteFixtureById(fixture_id);
  if(!fixture||!attribute)return;
  sendFixtureDmxValue(fixture,attribute,value);
}
function setRemoteOpticsValue(fixture_id,attribute,value){
  const fixture=remoteFixtureById(fixture_id);
  if(!fixture||!attribute)return;
  sendFixtureDmxValue(fixture,attribute,value);
}
function setRemoteOpticsFromPointer(event){
  const fixture=remoteFixtureById(Number(event.currentTarget.dataset.fixture));
  const attribute=event.currentTarget.dataset.attribute;
  if(!fixture||!attribute)return;
  event.preventDefault();
  event.currentTarget.setPointerCapture(event.pointerId);
  const bounds=event.currentTarget.getBoundingClientRect();
  const x=Math.max(0,Math.min(1,(event.clientX-bounds.left)/bounds.width));
  setRemoteOpticsValue(fixture.id,attribute,Math.round(x*65535));
}
function dragRemoteOpticsValue(event){
  if(event.buttons!==1)return;
  setRemoteOpticsFromPointer(event);
}
function keyRemoteOpticsValue(event,fixture_id,attribute,currentValue){
  const step=event.shiftKey?4096:event.altKey?256:1024;
  let next=null;
  if(event.key==="ArrowLeft"||event.key==="ArrowDown")next=remoteDmxClamp(Number(currentValue)-step);
  if(event.key==="ArrowRight"||event.key==="ArrowUp")next=remoteDmxClamp(Number(currentValue)+step);
  if(event.key==="Home")next=0;
  if(event.key==="End")next=65535;
  if(next===null)return;
  event.preventDefault();
  setRemoteOpticsValue(fixture_id,attribute,next);
}
function setRemoteFixtureColor(fixture_id,hex){
  const fixture=remoteFixtureById(fixture_id);
  const controls=remoteColorControls(fixture);
  if(!fixture||!controls||!/^#[0-9a-fA-F]{6}$/.test(String(hex)))return;
  let red=Number.parseInt(hex.slice(1,3),16)*257;
  let green=Number.parseInt(hex.slice(3,5),16)*257;
  let blue=Number.parseInt(hex.slice(5,7),16)*257;
  const white=(controls.extras||[]).find(extra=>extra.key==="white");
  if(remoteColorAutoWhite&&white){
    const whiteValue=Math.min(red,green,blue);
    red=Math.max(0,red-whiteValue);
    green=Math.max(0,green-whiteValue);
    blue=Math.max(0,blue-whiteValue);
    sendFixtureDmxValue(fixture,white.attribute,whiteValue);
  }
  sendFixtureDmxValue(fixture,controls.red,red);
  sendFixtureDmxValue(fixture,controls.green,green);
  sendFixtureDmxValue(fixture,controls.blue,blue);
}
function setRemoteColorAutoWhite(enabled){
  remoteColorAutoWhite=enabled===true||enabled==="true";
  const fixture=remoteControlFixture();
  renderRemoteVisualControls(fixture);
}
function setRemoteColorChannel(fixture_id,channel,value){
  const fixture=remoteFixtureById(fixture_id);
  const controls=remoteColorControls(fixture);
  if(!fixture||!controls)return;
  const attribute=controls[channel];
  if(attribute)sendFixtureDmxValue(fixture,attribute,value);
}
function setRemoteExtraColorChannel(fixture_id,key,value){
  const fixture=remoteFixtureById(fixture_id);
  const controls=remoteColorControls(fixture);
  if(!fixture||!controls)return;
  const extra=(controls.extras||[]).find(candidate=>candidate.key===key);
  if(extra)sendFixtureDmxValue(fixture,extra.attribute,value);
}
function setSelectedFixtureHighlight(enabled){const fixture_id=selectedNumber("fixtureId");if(fixture_id!==null)send({type:"setFixtureHighlight",fixture_id,enabled})}
function setSelectedFixtureSolo(enabled){const fixture_id=selectedNumber("fixtureId");if(fixture_id!==null)send({type:"setFixtureSolo",fixture_id,enabled})}
function setSelectedFixturePark(enabled){const fixture_id=selectedNumber("fixtureId");if(fixture_id!==null)send({type:"setFixturePark",fixture_id,enabled})}
function setSelectedTargetHighlight(enabled){const group_id=selectedGroupId();if(group_id){send({type:"setGroupHighlight",group_id,enabled});return}setSelectedFixtureHighlight(enabled)}
function setSelectedTargetSolo(enabled){const group_id=selectedGroupId();if(group_id){send({type:"setGroupSolo",group_id,enabled});return}setSelectedFixtureSolo(enabled)}
function setSelectedTargetPark(enabled){const group_id=selectedGroupId();if(group_id){send({type:"setGroupPark",group_id,enabled});return}setSelectedFixturePark(enabled)}
function clearFixtureFlags(kind){send({type:"clearFixtureFlags",kind})}
function setGroupSubmasterFromInput(el){send({type:"setGroupSubmaster",group_id:el.dataset.group,level:Number(el.value)})}
function setGroupHighlightFromButton(el,enabled){send({type:"setGroupHighlight",group_id:el.dataset.group,enabled})}
function setGroupSoloFromButton(el,enabled){send({type:"setGroupSolo",group_id:el.dataset.group,enabled})}
function setGroupParkFromButton(el,enabled){send({type:"setGroupPark",group_id:el.dataset.group,enabled})}
function selectLayer(id){document.getElementById("layerId").value=String(id)}
function triggerCue(id){document.getElementById("cueId").value=String(id);send({type:"triggerCue",cue_id:id})}
function triggerFirstFilteredScene(){
  const rows=remoteFilteredSceneRows(latestSnapshot&&latestSnapshot.cues||[]);
  if(!rows.length){logEl.textContent=sceneFilterText.trim()?`No scene matches ${sceneFilterText}`:"No scene to trigger";return}
  triggerCue(rows[0].cue.id);
}
function remoteSceneFilterKeyDown(event){
  if(event.key==="Enter"){event.preventDefault();triggerFirstFilteredScene()}
  if(event.key==="Escape"){event.preventDefault();clearSceneFilter()}
}
function setSceneFilter(value){
  sceneFilterText=String(value||"");
  if(latestSnapshot)renderSceneSelector(latestSnapshot.cues||[],latestSnapshot.active_cue_id);
}
function clearSceneFilter(){
  sceneFilterText="";
  if(latestSnapshot)renderSceneSelector(latestSnapshot.cues||[],latestSnapshot.active_cue_id);
}
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
  const fixture=remoteControlFixture();
  const fixture_id=fixture&&fixture.id;
  const attribute=document.getElementById("attribute").value;
  if(!fixture_id||!attribute)return;
  const value=numberValue("attributeValue");
  text("attributeValueLabel",String(Math.round(value*65535)));
  const group_id=selectedGroupId();
  if(group_id){
    send({type:"setGroupAttribute",group_id,attribute,value},false);
  }else{
    send({type:"setAttribute",fixture_id,attribute,value},false);
  }
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
  const group_id=selectedGroupId();
  if((group_id||selectedNumber("fixtureId")===fixture_id)&&document.getElementById("attribute").value===attribute){
    document.getElementById("attributeValue").value=value;
    text("attributeValueLabel",dmxValue);
  }
  if(group_id){
    send({type:"setGroupAttribute",group_id,attribute,value},false);
  }else{
    send({type:"setAttribute",fixture_id,attribute,value},false);
  }
  queueSnapshot(160);
}
function sendCue(){const cue_id=selectedNumber("cueId");if(cue_id!==null)send({type:"triggerCue",cue_id})}
function setRemoteEffectEnabled(effect_id,enabled){send({type:"setEffectEnabled",effect_id,enabled})}
function setRemoteNodeGraphEnabled(graph_id,enabled){send({type:"setNodeGraphEnabled",graph_id,enabled})}
function moveRemoteEffect(effect_id,delta){send({type:"moveEffect",effect_id,delta})}
function removeRemoteEffect(effect_id,label){if(window.confirm(`Remove ${label}?`))send({type:"removeEffect",effect_id})}
function syncExternalVideoTransportsRemote(){
  send({type:"syncExternalVideoTransports"},false);
}
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
function fadeLayerOpacity(layer_id,opacity){send({type:"fadeVideoLayerOpacity",layer_id,opacity,duration_ms:Math.max(0,Math.round(numberValue("videoLayerFadeMs")))})}
function fadeSelectedLayer(opacity){const layer_id=selectedNumber("layerId");if(layer_id!==null)fadeLayerOpacity(layer_id,opacity)}
function selectRemoteVideoOutput(output_id,rerenderList=true){
  remoteStageSelectedOutputId=Number(output_id);
  renderRemoteStage(latestSnapshot||{});
  if(rerenderList){
    const video=(latestSnapshot&&latestSnapshot.video)||{};
    renderVideoOutputList(video.outputs||[],video.compositions||[]);
  }
}
function setVideoOutputEnabled(output_id,enabled){selectRemoteVideoOutput(output_id);send({type:"setVideoOutputEnabled",output_id,enabled})}
function setVideoOutputBlackout(output_id,blackout){selectRemoteVideoOutput(output_id);send({type:"setVideoOutputBlackout",output_id,blackout})}
function setVideoOutputOpacity(output_id,opacity){selectRemoteVideoOutput(output_id);send({type:"setVideoOutputOpacity",output_id,opacity})}
function setVideoOutputOpacityFromInput(output_id,value){const opacity=Number(value);if(Number.isFinite(opacity)){selectRemoteVideoOutput(output_id,false);send({type:"setVideoOutputOpacity",output_id,opacity},false);queueSnapshot(160)}}
function remoteOutputById(output_id){
  const video=(latestSnapshot&&latestSnapshot.video)||{};
  return (video.outputs||[]).find(output=>output.id===output_id)||null;
}
function patchRemoteOutputMapping(output_id,patch){
  const output=remoteOutputById(output_id);
  if(!output)return;
  const mapping={...remoteDefaultMapping(),...(output.mapping||{}),...patch};
  output.mapping=mapping;
  const entries=Object.entries(patch);
  const canUseFieldUpdates=entries.length>0&&entries.every(([,value])=>Number.isFinite(Number(value)));
  if(canUseFieldUpdates){
    entries.forEach(([field,value])=>send({type:"setVideoOutputMappingField",output_id,field,value:Number(value)},false));
  }else{
    send({type:"setVideoOutputMapping",output_id,mapping},false);
  }
  queueSnapshot(180);
}
function applyRemoteOutputMappingPreset(output_id,label){
  if(!String(label||"").trim())return;
  send({type:"applyVideoOutputMappingPreset",output_id,label});
}
function remoteMapPointerPoint(event){
  const svg=event.currentTarget.ownerSVGElement;
  if(!svg)return null;
  const bounds=svg.getBoundingClientRect();
  return {x:((event.clientX-bounds.left)/bounds.width)*100,y:((event.clientY-bounds.top)/bounds.height)*100};
}
function setRemoteProjectorCornerFromPointer(event,output_id,corner_key){
  const output=remoteOutputById(output_id);
  const corner=remoteProjectorCorners.find(candidate=>candidate.key===corner_key);
  if(!output||!corner)return;
  event.preventDefault();
  event.currentTarget.setPointerCapture(event.pointerId);
  const mapping={...remoteDefaultMapping(),...(output.mapping||{})};
  const pointer=remoteMapPointerPoint(event);
  if(!pointer)return;
  const base=remoteProjectorBasePoint(mapping,corner);
  patchRemoteOutputMapping(output_id,{[corner.xField]:remoteRoundedMappingValue((pointer.x-base.x)/remoteProjectorCornerGain),[corner.yField]:remoteRoundedMappingValue((pointer.y-base.y)/remoteProjectorCornerGain)});
}
function dragRemoteProjectorCorner(event,output_id,corner_key){
  if(event.buttons!==1)return;
  setRemoteProjectorCornerFromPointer(event,output_id,corner_key);
}
function setRemoteProjectorOffsetFromPointer(event,output_id){
  const output=remoteOutputById(output_id);
  if(!output)return;
  event.preventDefault();
  event.currentTarget.setPointerCapture(event.pointerId);
  const mapping={...remoteDefaultMapping(),...(output.mapping||{})};
  const pointer=remoteMapPointerPoint(event);
  if(!pointer)return;
  const center=remoteProjectorCenter(mapping);
  const deltaX=pointer.x-center.x;
  const deltaY=pointer.y-center.y;
  const rotation=remoteFinite(mapping.rotation_deg,0)*(Math.PI/180);
  const cos=Math.cos(rotation);
  const sin=Math.sin(rotation);
  const localX=deltaX*cos+deltaY*sin;
  const localY=-deltaX*sin+deltaY*cos;
  patchRemoteOutputMapping(output_id,{offset_x:remoteRoundedMappingValue(remoteFinite(mapping.offset_x,0)+localX/20),offset_y:remoteRoundedMappingValue(remoteFinite(mapping.offset_y,0)+localY/20)});
}
function dragRemoteProjectorOffset(event,output_id){
  if(event.buttons!==1)return;
  setRemoteProjectorOffsetFromPointer(event,output_id);
}
function setRemoteProjectorKeystoneFromPointer(event,output_id,axis){
  const output=remoteOutputById(output_id);
  if(!output)return;
  event.preventDefault();
  event.currentTarget.setPointerCapture(event.pointerId);
  const pointer=remoteMapPointerPoint(event);
  if(!pointer)return;
  const field=axis==="x"?"keystone_x":"keystone_y";
  const coordinate=axis==="x"?pointer.x:pointer.y;
  patchRemoteOutputMapping(output_id,{[field]:remoteRoundedMappingValue((coordinate-50)/remoteProjectorKeystoneGain)});
}
function dragRemoteProjectorKeystone(event,output_id,axis){
  if(event.buttons!==1)return;
  setRemoteProjectorKeystoneFromPointer(event,output_id,axis);
}
function nudgeRemoteOutputMapping(output_id,field,delta){
  const output=remoteOutputById(output_id);
  if(!output)return;
  const mapping={...remoteDefaultMapping(),...(output.mapping||{})};
  const fallback=field==="aspect_ratio"||field==="scale_x"||field==="scale_y"?1:0;
  const range=remoteMappingFieldRange(field);
  patchRemoteOutputMapping(output_id,{[field]:Number(remoteClamp(remoteFinite(mapping[field],fallback)+delta,range[0],range[1]).toFixed(3))});
}
function setRemoteOutputMappingField(output_id,field,value,min,max,digits=3){
  const numeric=Number(value);
  if(!Number.isFinite(numeric))return;
  patchRemoteOutputMapping(output_id,{[field]:remoteRoundedRangeValue(numeric,Number(min),Number(max),Number(digits)||3)});
}
function setRemoteOutputAspectMode(output_id,aspect_mode){
  patchRemoteOutputMapping(output_id,{aspect_mode});
}
function fitRemoteOutputMapping(output_id){
  const output=remoteOutputById(output_id);
  if(output)patchRemoteOutputMapping(output_id,{aspect_ratio:remoteOutputRatio(output),aspect_mode:"Fit"});
}
function setRemoteOutputAspect(output_id,aspect_ratio){
  patchRemoteOutputMapping(output_id,{aspect_ratio,aspect_mode:"Fit"});
}
function setRemoteOutputKeystonePreset(output_id,keystone_x,keystone_y){
  patchRemoteOutputMapping(output_id,{keystone_x,keystone_y});
}
function clearRemoteOutputKeystone(output_id){
  patchRemoteOutputMapping(output_id,{keystone_x:0,keystone_y:0,lens_distortion:0});
}
function resetRemoteOutputPose(output_id){
  patchRemoteOutputMapping(output_id,{offset_x:0,offset_y:0,scale_x:1,scale_y:1,rotation_deg:0});
}
function resetRemoteOutputStage(output_id){
  patchRemoteOutputMapping(output_id,{stage_x:0,stage_y:0,stage_z:0});
}
function fadeVideoOutput(output_id,opacity){selectRemoteVideoOutput(output_id);send({type:"fadeVideoOutputOpacity",output_id,opacity,duration_ms:Math.max(0,Math.round(numberValue("videoOutputFadeMs")))})}
function seekSelectedLayer(position_ms){const layer_id=selectedNumber("layerId");if(layer_id!==null&&Number.isFinite(position_ms))send({type:"seekVideoLayer",layer_id,position_ms:Math.max(0,Math.round(position_ms))})}
function seekLayerFromInput(layer_id,value){
  const position_ms=Math.max(0,Math.round(Number(value)));
  if(Number.isFinite(position_ms))send({type:"seekVideoLayer",layer_id,position_ms},false);
  queueSnapshot(160);
}
function setAllBlackout(enabled){
  send({type:"allBlackout",enabled});
}
function remoteLayerById(layer_id){
  const video=(latestSnapshot&&latestSnapshot.video)||{};
  return (video.layers||[]).find(layer=>layer.id===layer_id)||null;
}
function remoteLayerSliderMax(layer){
  const state=layer&&layer.state||{};
  const metadata=layer&&layer.source&&layer.source.metadata||{};
  return Math.max(Number(metadata.duration_ms)||0,state.position_ms||0,state.loop_end_ms||0,1);
}
function remoteLayerLoopRangeOrSource(layer){
  const state=layer&&layer.state||{};
  const max=remoteLayerSliderMax(layer);
  const start=Math.min(Math.max(0,Math.round(Number(state.loop_start_ms)||0)),Math.max(0,max-1));
  const end=Math.min(Math.max(start+1,Math.round(Number(state.loop_end_ms)||0)),max);
  if(state.loop_enabled===true&&end>start)return {start,end};
  return max>1?{start:0,end:max}:null;
}
function remoteBpmLoopBarsForCurrentLoop(layer){
  const range=remoteLayerLoopRangeOrSource(layer);
  const bpm=Number(latestSnapshot&&latestSnapshot.clock&&latestSnapshot.clock.bpm)||120;
  if(!range||!Number.isFinite(bpm)||bpm<=0)return 1;
  const sync=layer&&layer.state&&layer.state.bpm_sync||{};
  const ratio=remoteClamp(sync.ratio??1,0.25,4);
  const barMs=240000/bpm;
  const bars=((range.end-range.start)*ratio)/barMs;
  return remoteClamp(Math.max(0.25,Math.round(bars*4)/4),0.25,128);
}
function setLayerBpmLoopBars(layer_id,bars){
  const layer=remoteLayerById(layer_id);
  if(!layer)return;
  const range=remoteLayerLoopRangeOrSource(layer);
  if(range)send({type:"setVideoLoop",layer_id,enabled:true,loop_start_ms:range.start,loop_end_ms:range.end},false);
  send({type:"setVideoParam",layer_id,param:"bpm_sync",value:1},false);
  send({type:"setVideoParam",layer_id,param:"bpm_sync_loop_bars",value:remoteClamp(bars,0.25,128)});
}
function matchLayerBpmLoopToCurrentLength(layer_id){
  const layer=remoteLayerById(layer_id);
  if(!layer)return;
  setLayerBpmLoopBars(layer_id,remoteBpmLoopBarsForCurrentLoop(layer));
}
function remoteLayerFrameStepMs(layer){
  const metadata=layer&&layer.source&&layer.source.metadata||{};
  const frameRate=Number(metadata.frame_rate);
  return Number.isFinite(frameRate)&&frameRate>0?Math.max(1,Math.round(1000/frameRate)):33;
}
function clampRemoteLayerPosition(layer,position_ms){
  return Math.max(0,Math.min(remoteLayerSliderMax(layer),Math.round(Number(position_ms)||0)));
}
function remoteTimelinePointerMs(event,layer_id){
  const layer=remoteLayerById(layer_id);
  if(!layer)return null;
  const svg=event.currentTarget.ownerSVGElement||event.currentTarget;
  const bounds=svg.getBoundingClientRect();
  if(!bounds.width)return null;
  const x=((event.clientX-bounds.left)/bounds.width)*100;
  return clampRemoteLayerPosition(layer,((Math.max(0,Math.min(1,(x-4)/92)))*remoteLayerSliderMax(layer)));
}
function setRemoteVideoTimelinePosition(event,layer_id){
  event.preventDefault();
  event.currentTarget.setPointerCapture(event.pointerId);
  const position_ms=remoteTimelinePointerMs(event,layer_id);
  if(position_ms===null)return;
  send({type:"seekVideoLayer",layer_id,position_ms},false);
  queueSnapshot(160);
}
function dragRemoteVideoTimelinePosition(event,layer_id){
  if(event.buttons!==1)return;
  setRemoteVideoTimelinePosition(event,layer_id);
}
function setRemoteVideoTimelineLoop(event,layer_id,boundary){
  const layer=remoteLayerById(layer_id);
  if(!layer)return;
  event.preventDefault();
  event.currentTarget.setPointerCapture(event.pointerId);
  const time=remoteTimelinePointerMs(event,layer_id);
  if(time===null)return;
  const state=layer.state||{};
  const max=remoteLayerSliderMax(layer);
  const currentStart=Math.max(0,Math.min(max-1,Math.round(Number(state.loop_start_ms)||0)));
  const currentEnd=Math.max(currentStart+1,Math.min(max,Math.round(Number(state.loop_end_ms)||max)));
  const loop_start_ms=boundary==="start"?Math.min(time,currentEnd-1):currentStart;
  const loop_end_ms=boundary==="end"?Math.max(time,currentStart+1):currentEnd;
  send({type:"setVideoLoop",layer_id,enabled:true,loop_start_ms,loop_end_ms},false);
  queueSnapshot(160);
}
function dragRemoteVideoTimelineLoop(event,layer_id,boundary){
  if(event.buttons!==1)return;
  setRemoteVideoTimelineLoop(event,layer_id,boundary);
}
function seekLayerByFrame(layer_id,direction){
  const layer=remoteLayerById(layer_id);
  if(!layer)return;
  const state=layer.state||{};
  const next=clampRemoteLayerPosition(layer,(state.position_ms||0)+remoteLayerFrameStepMs(layer)*(direction>0?1:-1));
  send({type:"seekVideoLayer",layer_id,position_ms:next});
}
function setLayerLoopBoundaryToCurrent(layer_id,boundary){
  const layer=remoteLayerById(layer_id);
  if(!layer)return;
  const state=layer.state||{};
  const position=clampRemoteLayerPosition(layer,state.position_ms||0);
  const loop_start_ms=boundary==="in"?position:Math.max(0,Math.round(Number(state.loop_start_ms)||0));
  const loop_end_ms=boundary==="out"?position:Math.max(loop_start_ms+1,Math.round(Number(state.loop_end_ms)||remoteLayerSliderMax(layer)));
  send({type:"setVideoLoop",layer_id,enabled:true,loop_start_ms,loop_end_ms:Math.max(loop_start_ms+1,loop_end_ms)});
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
function jumpLayerRelativeCuePoint(layer_id,direction){send({type:"jumpVideoCuePointRelative",layer_id,direction:direction>0?1:-1})}
function jumpSelectedLayerRelativeCuePoint(direction){const layer_id=selectedNumber("layerId");if(layer_id!==null)jumpLayerRelativeCuePoint(layer_id,direction)}
function setTimelinePlaying(playing){send({type:"setTimelinePlaying",playing})}
function seekTimelineRemote(position_ms){if(Number.isFinite(position_ms))send({type:"seekTimeline",position_ms:Math.max(0,Math.round(position_ms))})}
function seekTimelineAdjacentBeat(direction){send({type:"seekTimelineBeat",direction:direction>0?1:-1})}
function syncTimelineTimecodeRemote(){const raw=document.getElementById("timelineTimecodeMs").value.trim();const numeric=Number(raw);const position_ms=raw.includes(":")?raw:numeric;const source=document.getElementById("timelineTimecodeSource").value;if(raw&&(typeof position_ms==="string"||Number.isFinite(position_ms)))send({type:"syncTimelineTimecode",position_ms:typeof position_ms==="string"?position_ms:Math.max(0,Math.round(position_ms)),source})}
if("serviceWorker" in navigator&&window.isSecureContext){navigator.serviceWorker.register("/remote-sw.js").catch(()=>{})}
connect();
</script>
</body>
</html>
"##;
const REMOTE_MANIFEST: &str = r##"{"name":"Syndocal Remote","short_name":"Syndocal","start_url":"/","scope":"/","display":"standalone","background_color":"#111419","theme_color":"#111419","icons":[{"src":"/icon.svg","sizes":"any","type":"image/svg+xml","purpose":"any maskable"}]}"##;
const REMOTE_ICON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128"><rect width="128" height="128" rx="24" fill="#111419"/><path d="M28 88h72" stroke="#4aa8ff" stroke-width="10" stroke-linecap="round"/><path d="M36 34v42M64 24v52M92 44v32" stroke="#edf3fb" stroke-width="10" stroke-linecap="round"/><circle cx="36" cy="58" r="12" fill="#f2c14e"/><circle cx="64" cy="42" r="12" fill="#74d99f"/><circle cx="92" cy="66" r="12" fill="#4aa8ff"/></svg>"##;
const REMOTE_SERVICE_WORKER_JS: &str = r##"const CACHE_NAME="syndocal-remote-v2";const SHELL=["/manifest.webmanifest","/icon.svg"];self.addEventListener("install",event=>{event.waitUntil(caches.open(CACHE_NAME).then(cache=>cache.addAll(SHELL)).then(()=>self.skipWaiting()))});self.addEventListener("activate",event=>{event.waitUntil(caches.keys().then(keys=>Promise.all(keys.filter(key=>key!==CACHE_NAME).map(key=>caches.delete(key)))).then(()=>self.clients.claim()))});self.addEventListener("fetch",event=>{if(event.request.method!=="GET")return;const url=new URL(event.request.url);if(url.pathname==="/"||url.pathname==="/remote"||url.pathname==="/index.html"||url.pathname==="/ws"||url.searchParams.has("token"))return;event.respondWith(fetch(event.request).then(response=>{const copy=response.clone();if(event.request.url.startsWith(self.location.origin)){caches.open(CACHE_NAME).then(cache=>cache.put(event.request,copy))}return response}).catch(()=>caches.match(event.request)))});"##;

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteInputEvent {
    SetAttribute {
        fixture_id: FixtureId,
        attribute: String,
        value: u16,
    },
    SetGroupAttribute {
        group_id: String,
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
    ClearFixtureFlags {
        kind: String,
    },
    Blackout(bool),
    AllBlackout(bool),
    SetBpm(f32),
    TapBpm,
    SyncExternalClock {
        bpm: f32,
        beat_phase: f32,
        source: ClockSource,
    },
    ResetTelemetry,
    TriggerCue(CueId),
    TriggerNextCue,
    TriggerPreviousCue,
    SetCueFadePaused(bool),
    SetTimelinePlaying(bool),
    SeekTimeline {
        position_ms: u64,
    },
    SeekTimelineBeat {
        direction: i32,
    },
    SyncTimelineTimecode {
        position_ms: u64,
        source: ClockSource,
    },
    SetEffectEnabled {
        effect_id: EffectId,
        enabled: bool,
    },
    SetNodeGraphEnabled {
        graph_id: NodeGraphId,
        enabled: bool,
    },
    MoveEffect {
        effect_id: EffectId,
        delta: i32,
    },
    RemoveEffect {
        effect_id: EffectId,
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
    FadeVideoLayerOpacity {
        layer_id: VideoLayerId,
        opacity: f32,
        duration_ms: u64,
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
    JumpVideoCuePointRelative {
        layer_id: VideoLayerId,
        direction: i32,
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
    SetVideoOutputMapping {
        output_id: VideoOutputId,
        mapping: VideoOutputMapping,
    },
    SetVideoOutputMappingField {
        output_id: VideoOutputId,
        field: String,
        value: f32,
    },
    ApplyVideoOutputMappingPreset {
        output_id: VideoOutputId,
        label: String,
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
    GetVideoRuntimeStatus,
    GetVideoOutputRenderPlans,
    GetExternalVideoIoPlans,
    GetExternalVideoTransportStatus,
    SyncExternalVideoTransports,
}

#[derive(Debug, Error)]
pub enum RemoteWsError {
    #[error("remote bind address is required")]
    MissingBindAddress,
    #[error("remote port must be greater than 0")]
    InvalidPort,
    #[error("remote pairing PIN must contain exactly 6 digits")]
    InvalidPairingPin,
    #[error("remote LAN access must be enabled before binding to a non-loopback address")]
    LanAccessDisabled,
    #[error("remote connection limit must be between 1 and 64")]
    InvalidConnectionLimit,
    #[error("remote message size limit must be between 1024 and 1048576 bytes")]
    InvalidMessageSizeLimit,
    #[error("remote message rate limit must be between 1 and 1000 messages per second")]
    InvalidMessageRateLimit,
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
    clients: Arc<Mutex<HashMap<u64, RemoteClientState>>>,
    rejected_connections: Arc<AtomicU64>,
}

struct RemoteClientState {
    summary: RemoteClientSummary,
    disconnect: Arc<AtomicBool>,
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
        Self::start_with_snapshot_and_video_runtime_status(
            config,
            callback,
            snapshot_provider,
            VideoRuntimeStatus::default,
        )
    }

    pub fn start_with_snapshot_and_video_runtime_status<F, S, V>(
        config: RemoteControlConfig,
        callback: F,
        snapshot_provider: S,
        video_runtime_status_provider: V,
    ) -> Result<Self, RemoteWsError>
    where
        F: Fn(RemoteInputEvent) + Send + Sync + 'static,
        S: Fn() -> EngineSnapshot + Send + Sync + 'static,
        V: Fn() -> VideoRuntimeStatus + Send + Sync + 'static,
    {
        Self::start_with_snapshot_and_video_status_providers(
            config,
            callback,
            snapshot_provider,
            video_runtime_status_provider,
            default_video_output_render_plans,
            default_external_video_io_plans,
            default_external_video_transport_status,
            default_external_video_transport_sync,
        )
    }

    pub fn start_with_snapshot_and_video_status_providers<F, S, V, R, I, T, X>(
        config: RemoteControlConfig,
        callback: F,
        snapshot_provider: S,
        video_runtime_status_provider: V,
        video_output_render_plans_provider: R,
        external_video_io_plans_provider: I,
        external_video_transport_status_provider: T,
        external_video_transport_sync_provider: X,
    ) -> Result<Self, RemoteWsError>
    where
        F: Fn(RemoteInputEvent) + Send + Sync + 'static,
        S: Fn() -> EngineSnapshot + Send + Sync + 'static,
        V: Fn() -> VideoRuntimeStatus + Send + Sync + 'static,
        R: Fn() -> Value + Send + Sync + 'static,
        I: Fn() -> Value + Send + Sync + 'static,
        T: Fn() -> Value + Send + Sync + 'static,
        X: Fn() -> Value + Send + Sync + 'static,
    {
        if config.bind_ip.trim().is_empty() {
            return Err(RemoteWsError::MissingBindAddress);
        }
        if config.port == 0 {
            return Err(RemoteWsError::InvalidPort);
        }
        if config.pairing_pin.len() != 6
            || !config.pairing_pin.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(RemoteWsError::InvalidPairingPin);
        }
        if !config.allow_lan
            && config
                .bind_ip
                .trim()
                .parse::<std::net::IpAddr>()
                .map(|ip| !ip.is_loopback())
                .unwrap_or(true)
        {
            return Err(RemoteWsError::LanAccessDisabled);
        }
        if !(1..=64).contains(&config.max_connections) {
            return Err(RemoteWsError::InvalidConnectionLimit);
        }
        if !(1_024..=1_048_576).contains(&config.max_message_bytes) {
            return Err(RemoteWsError::InvalidMessageSizeLimit);
        }
        if !(1..=1_000).contains(&config.max_messages_per_second) {
            return Err(RemoteWsError::InvalidMessageRateLimit);
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
        let video_runtime_status_provider = Arc::new(video_runtime_status_provider);
        let video_output_render_plans_provider = Arc::new(video_output_render_plans_provider);
        let external_video_io_plans_provider = Arc::new(external_video_io_plans_provider);
        let external_video_transport_status_provider =
            Arc::new(external_video_transport_status_provider);
        let external_video_transport_sync_provider =
            Arc::new(external_video_transport_sync_provider);
        let config = Arc::new(config);
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let next_client_id = Arc::new(AtomicU64::new(1));
        let rejected_connections = Arc::new(AtomicU64::new(0));
        let thread_clients = Arc::clone(&clients);
        let thread_next_client_id = Arc::clone(&next_client_id);
        let thread_rejected_connections = Arc::clone(&rejected_connections);
        let thread = thread::Builder::new()
            .name("syndocal-remote-ws".to_string())
            .spawn(move || {
                while !thread_stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let client_stop = Arc::clone(&thread_stop);
                            let client_callback = Arc::clone(&callback);
                            let client_snapshot_provider = Arc::clone(&snapshot_provider);
                            let client_video_runtime_status_provider =
                                Arc::clone(&video_runtime_status_provider);
                            let client_video_output_render_plans_provider =
                                Arc::clone(&video_output_render_plans_provider);
                            let client_external_video_io_plans_provider =
                                Arc::clone(&external_video_io_plans_provider);
                            let client_external_video_transport_status_provider =
                                Arc::clone(&external_video_transport_status_provider);
                            let client_external_video_transport_sync_provider =
                                Arc::clone(&external_video_transport_sync_provider);
                            let client_config = Arc::clone(&config);
                            let client_registry = Arc::clone(&thread_clients);
                            let client_next_id = Arc::clone(&thread_next_client_id);
                            let client_rejected_connections =
                                Arc::clone(&thread_rejected_connections);
                            let _ = thread::Builder::new()
                                .name("syndocal-remote-ws-client".to_string())
                                .spawn(move || {
                                    handle_connection(
                                        stream,
                                        client_stop,
                                        client_callback.as_ref(),
                                        client_snapshot_provider.as_ref(),
                                        client_video_runtime_status_provider.as_ref(),
                                        client_video_output_render_plans_provider.as_ref(),
                                        client_external_video_io_plans_provider.as_ref(),
                                        client_external_video_transport_status_provider.as_ref(),
                                        client_external_video_transport_sync_provider.as_ref(),
                                        client_config.as_ref(),
                                        client_registry,
                                        client_next_id,
                                        client_rejected_connections,
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
            clients,
            rejected_connections,
        })
    }

    pub fn status(&self) -> RemoteControlStatus {
        let mut clients = self
            .clients
            .lock()
            .map(|clients| {
                clients
                    .values()
                    .map(|client| client.summary.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        clients.sort_by_key(|client| client.id);
        RemoteControlStatus {
            running: true,
            active_connections: clients.len(),
            rejected_connections: self.rejected_connections.load(Ordering::Relaxed),
            clients,
        }
    }

    pub fn disconnect_client(&self, client_id: u64) -> bool {
        self.clients
            .lock()
            .ok()
            .and_then(|clients| {
                clients.get(&client_id).map(|client| {
                    client.disconnect.store(true, Ordering::Relaxed);
                })
            })
            .is_some()
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
        RemoteClientRequest::GetVideoRuntimeStatus => Err(RemoteParseError::UnknownType(
            "getVideoRuntimeStatus".to_string(),
        )),
        RemoteClientRequest::GetVideoOutputRenderPlans => Err(RemoteParseError::UnknownType(
            "getVideoOutputRenderPlans".to_string(),
        )),
        RemoteClientRequest::GetExternalVideoIoPlans => Err(RemoteParseError::UnknownType(
            "getExternalVideoIoPlans".to_string(),
        )),
        RemoteClientRequest::GetExternalVideoTransportStatus => Err(RemoteParseError::UnknownType(
            "getExternalVideoTransportStatus".to_string(),
        )),
        RemoteClientRequest::SyncExternalVideoTransports => Err(RemoteParseError::UnknownType(
            "syncExternalVideoTransports".to_string(),
        )),
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
        "getVideoRuntimeStatus" => Ok(RemoteClientRequest::GetVideoRuntimeStatus),
        "getVideoOutputRenderPlans" => Ok(RemoteClientRequest::GetVideoOutputRenderPlans),
        "getExternalVideoIoPlans" => Ok(RemoteClientRequest::GetExternalVideoIoPlans),
        "getExternalVideoTransportStatus" => {
            Ok(RemoteClientRequest::GetExternalVideoTransportStatus)
        }
        "syncExternalVideoTransports" => Ok(RemoteClientRequest::SyncExternalVideoTransports),
        "setAttribute" => Ok(RemoteClientRequest::Event(RemoteInputEvent::SetAttribute {
            fixture_id: read_u64(&value, "fixture_id")?,
            attribute: read_string(&value, "attribute")?,
            value: read_u16_value(&value, "value")?,
        })),
        "setGroupAttribute" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetGroupAttribute {
                group_id: read_string(&value, "group_id")?,
                attribute: read_string(&value, "attribute")?,
                value: read_u16_value(&value, "value")?,
            },
        )),
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
        "clearFixtureFlags" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::ClearFixtureFlags {
                kind: read_string(&value, "kind")?,
            },
        )),
        "blackout" => Ok(RemoteClientRequest::Event(RemoteInputEvent::Blackout(
            read_bool(&value, "enabled")?,
        ))),
        "allBlackout" => Ok(RemoteClientRequest::Event(RemoteInputEvent::AllBlackout(
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
        "syncAbletonLinkClock" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SyncExternalClock {
                bpm: read_f32(&value, "bpm")?,
                beat_phase: read_f32(&value, "beat_phase")?,
                source: ClockSource::AbletonLink,
            },
        )),
        "syncExternalClock" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SyncExternalClock {
                bpm: read_f32(&value, "bpm")?,
                beat_phase: read_f32(&value, "beat_phase")?,
                source: read_clock_source(&value, "source")?,
            },
        )),
        "resetTelemetry" => Ok(RemoteClientRequest::Event(RemoteInputEvent::ResetTelemetry)),
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
        "seekTimelineBeat" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SeekTimelineBeat {
                direction: read_i32(&value, "direction")?,
            },
        )),
        "syncTimelineTimecode" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SyncTimelineTimecode {
                position_ms: read_timecode_position_ms(&value, "position_ms")?,
                source: read_optional_clock_source(&value, "source")?.unwrap_or(ClockSource::Ltc),
            },
        )),
        "setEffectEnabled" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetEffectEnabled {
                effect_id: read_u64(&value, "effect_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        "setNodeGraphEnabled" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetNodeGraphEnabled {
                graph_id: read_u64(&value, "graph_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        "moveEffect" => {
            let delta = read_i32(&value, "delta")?;
            if delta != -1 && delta != 1 {
                return Err(RemoteParseError::InvalidField("delta"));
            }
            Ok(RemoteClientRequest::Event(RemoteInputEvent::MoveEffect {
                effect_id: read_u64(&value, "effect_id")?,
                delta,
            }))
        }
        "removeEffect" => Ok(RemoteClientRequest::Event(RemoteInputEvent::RemoveEffect {
            effect_id: read_u64(&value, "effect_id")?,
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
        "fadeVideoLayerOpacity" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::FadeVideoLayerOpacity {
                layer_id: read_u64(&value, "layer_id")?,
                opacity: read_f32(&value, "opacity")?,
                duration_ms: read_u64(&value, "duration_ms")?,
            },
        )),
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
        "jumpVideoCuePointRelative" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::JumpVideoCuePointRelative {
                layer_id: read_u64(&value, "layer_id")?,
                direction: read_i32(&value, "direction")?,
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
        "setVideoOutputMapping" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoOutputMapping {
                output_id: read_u64(&value, "output_id")?,
                mapping: read_video_output_mapping(&value, "mapping")?,
            },
        )),
        "setVideoOutputMappingField" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoOutputMappingField {
                output_id: read_u64(&value, "output_id")?,
                field: read_video_output_mapping_field(&value, "field")?,
                value: read_f32(&value, "value")?,
            },
        )),
        "applyVideoOutputMappingPreset" => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::ApplyVideoOutputMappingPreset {
                output_id: read_u64(&value, "output_id")?,
                label: read_string(&value, "label")?,
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
    video_runtime_status_provider: &impl Fn() -> VideoRuntimeStatus,
    video_output_render_plans_provider: &impl Fn() -> Value,
    external_video_io_plans_provider: &impl Fn() -> Value,
    external_video_transport_status_provider: &impl Fn() -> Value,
    external_video_transport_sync_provider: &impl Fn() -> Value,
    config: &RemoteControlConfig,
    clients: Arc<Mutex<HashMap<u64, RemoteClientState>>>,
    next_client_id: Arc<AtomicU64>,
    rejected_connections: Arc<AtomicU64>,
) where
    F: Fn(RemoteInputEvent) + ?Sized,
    S: Fn() -> EngineSnapshot + ?Sized,
{
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    let mut peek_buffer = [0u8; HTTP_PEEK_SIZE];
    match stream.peek(&mut peek_buffer) {
        Ok(size) if is_websocket_request(&peek_buffer[..size]) => {
            let request = String::from_utf8_lossy(&peek_buffer[..size]);
            if !request_host_is_allowed(&request, &stream, config) {
                reject_http_client(stream, "403 Forbidden", "Invalid Host or Origin");
                return;
            }
            if !request_has_pairing_token(&request, &config.pairing_pin) {
                reject_http_client(stream, "401 Unauthorized", "Pairing PIN required");
                return;
            }
            let peer_addr = stream
                .peer_addr()
                .map(|address| address.to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            let client_id = next_client_id.fetch_add(1, Ordering::Relaxed);
            let disconnect = Arc::new(AtomicBool::new(false));
            let connected_at_unix_ms = unix_time_ms();
            let registered = clients
                .lock()
                .map(|mut registry| {
                    if registry.len() >= usize::from(config.max_connections) {
                        return false;
                    }
                    registry.insert(
                        client_id,
                        RemoteClientState {
                            summary: RemoteClientSummary {
                                id: client_id,
                                peer_addr,
                                connected_at_unix_ms,
                                last_activity_unix_ms: connected_at_unix_ms,
                                messages_received: 0,
                            },
                            disconnect: Arc::clone(&disconnect),
                        },
                    );
                    true
                })
                .unwrap_or(false);
            if !registered {
                rejected_connections.fetch_add(1, Ordering::Relaxed);
                reject_http_client(
                    stream,
                    "503 Service Unavailable",
                    "Remote connection limit reached",
                );
                return;
            }
            handle_websocket_client(
                stream,
                stop,
                callback,
                snapshot_provider,
                video_runtime_status_provider,
                video_output_render_plans_provider,
                external_video_io_plans_provider,
                external_video_transport_status_provider,
                external_video_transport_sync_provider,
                config,
                client_id,
                disconnect,
                Arc::clone(&clients),
            );
            if let Ok(mut registry) = clients.lock() {
                registry.remove(&client_id);
            }
        }
        Ok(size) => {
            let request = String::from_utf8_lossy(&peek_buffer[..size]);
            let path = request_path(&request).unwrap_or("/");
            if !request_host_is_allowed(&request, &stream, config) {
                reject_http_client(stream, "403 Forbidden", "Invalid Host or Origin");
            } else if matches!(path, "/" | "/remote" | "/index.html")
                && !request_has_pairing_token(&request, &config.pairing_pin)
            {
                reject_http_client(stream, "401 Unauthorized", "Pairing PIN required");
            } else {
                serve_http_client(stream, path);
            }
        }
        Err(_) => {}
    }
}

fn handle_websocket_client<F, S>(
    stream: TcpStream,
    stop: Arc<AtomicBool>,
    callback: &F,
    snapshot_provider: &S,
    video_runtime_status_provider: &impl Fn() -> VideoRuntimeStatus,
    video_output_render_plans_provider: &impl Fn() -> Value,
    external_video_io_plans_provider: &impl Fn() -> Value,
    external_video_transport_status_provider: &impl Fn() -> Value,
    external_video_transport_sync_provider: &impl Fn() -> Value,
    config: &RemoteControlConfig,
    client_id: u64,
    disconnect: Arc<AtomicBool>,
    clients: Arc<Mutex<HashMap<u64, RemoteClientState>>>,
) where
    F: Fn(RemoteInputEvent) + ?Sized,
    S: Fn() -> EngineSnapshot + ?Sized,
{
    let Ok(mut websocket) = accept(stream) else {
        return;
    };
    let mut rate_window_started = Instant::now();
    let mut rate_window_messages = 0u16;
    while !stop.load(Ordering::Relaxed) && !disconnect.load(Ordering::Relaxed) {
        match websocket.read() {
            Ok(Message::Text(text)) => {
                if text.len() > config.max_message_bytes {
                    let _ = websocket.send(Message::Text(
                        r#"{"ok":false,"error":"message size limit exceeded"}"#.to_string(),
                    ));
                    break;
                }
                if rate_window_started.elapsed() >= Duration::from_secs(1) {
                    rate_window_started = Instant::now();
                    rate_window_messages = 0;
                }
                rate_window_messages = rate_window_messages.saturating_add(1);
                if rate_window_messages > config.max_messages_per_second {
                    let _ = websocket.send(Message::Text(
                        r#"{"ok":false,"error":"message rate limit exceeded"}"#.to_string(),
                    ));
                    break;
                }
                update_remote_client_activity(&clients, client_id);
                match request_from_text(&text) {
                    Ok(RemoteClientRequest::Event(event)) => {
                        callback(event);
                        let _ = websocket
                            .send(Message::Text(r#"{"ok":true,"type":"ack"}"#.to_string()));
                    }
                    Ok(RemoteClientRequest::GetSnapshot) => {
                        let response = snapshot_response_json(&(snapshot_provider)());
                        let _ = websocket.send(Message::Text(response));
                    }
                    Ok(RemoteClientRequest::GetVideoRuntimeStatus) => {
                        let response =
                            video_runtime_status_response_json(&(video_runtime_status_provider)());
                        let _ = websocket.send(Message::Text(response));
                    }
                    Ok(RemoteClientRequest::GetVideoOutputRenderPlans) => {
                        let response = video_output_render_plans_response_json(
                            &(video_output_render_plans_provider)(),
                        );
                        let _ = websocket.send(Message::Text(response));
                    }
                    Ok(RemoteClientRequest::GetExternalVideoIoPlans) => {
                        let response = external_video_io_plans_response_json(
                            &(external_video_io_plans_provider)(),
                        );
                        let _ = websocket.send(Message::Text(response));
                    }
                    Ok(RemoteClientRequest::GetExternalVideoTransportStatus) => {
                        let response = external_video_transport_status_response_json(
                            &(external_video_transport_status_provider)(),
                        );
                        let _ = websocket.send(Message::Text(response));
                    }
                    Ok(RemoteClientRequest::SyncExternalVideoTransports) => {
                        let response = external_video_transport_sync_response_json(
                            &(external_video_transport_sync_provider)(),
                        );
                        let _ = websocket.send(Message::Text(response));
                    }
                    Err(error) => {
                        let _ = websocket.send(Message::Text(format!(
                            r#"{{"ok":false,"error":"{error:?}"}}"#
                        )));
                    }
                }
            }
            Ok(Message::Binary(data)) if data.len() > config.max_message_bytes => {
                let _ = websocket.send(Message::Text(
                    r#"{"ok":false,"error":"message size limit exceeded"}"#.to_string(),
                ));
                break;
            }
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

fn video_runtime_status_response_json(status: &VideoRuntimeStatus) -> String {
    serde_json::json!({
        "ok": true,
        "type": "videoRuntimeStatus",
        "video_runtime_status": status,
    })
    .to_string()
}

fn video_output_render_plans_response_json(plans: &Value) -> String {
    json!({
        "ok": true,
        "type": "videoOutputRenderPlans",
        "video_output_render_plans": plans,
    })
    .to_string()
}

fn external_video_io_plans_response_json(plans: &Value) -> String {
    json!({
        "ok": true,
        "type": "externalVideoIoPlans",
        "external_video_io_plans": plans,
    })
    .to_string()
}

fn external_video_transport_status_response_json(status: &Value) -> String {
    json!({
        "ok": true,
        "type": "externalVideoTransportStatus",
        "external_video_transport_status": status,
    })
    .to_string()
}

fn external_video_transport_sync_response_json(sync: &Value) -> String {
    json!({
        "ok": true,
        "type": "externalVideoTransportSync",
        "external_video_transport_sync": sync,
    })
    .to_string()
}

fn default_external_video_io_plans() -> Value {
    json!({
        "inputs": [],
        "outputs": [],
    })
}

fn default_video_output_render_plans() -> Value {
    json!([])
}

fn default_external_video_transport_sync() -> Value {
    json!({
        "report": null,
        "events": [],
    })
}

fn default_external_video_transport_status() -> Value {
    json!({
        "active_routes": [],
        "active_count": 0,
    })
}

fn serve_http_client(mut stream: TcpStream, path: &str) {
    let mut discard = [0u8; HTTP_PEEK_SIZE];
    let _ = stream.read(&mut discard);
    let response = http_response_for_path(path);
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn reject_http_client(mut stream: TcpStream, status: &str, message: &str) {
    let mut discard = [0u8; HTTP_PEEK_SIZE];
    let _ = stream.read(&mut discard);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{message}",
        message.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn update_remote_client_activity(clients: &Mutex<HashMap<u64, RemoteClientState>>, client_id: u64) {
    if let Ok(mut clients) = clients.lock() {
        if let Some(client) = clients.get_mut(&client_id) {
            client.summary.last_activity_unix_ms = unix_time_ms();
            client.summary.messages_received = client.summary.messages_received.saturating_add(1);
        }
    }
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
    parts.next()?.split('?').next()
}

fn request_target(request: &str) -> Option<&str> {
    let mut parts = request.lines().next()?.split_whitespace();
    (parts.next()? == "GET").then_some(parts.next()?)
}

fn request_header<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    request.lines().skip(1).find_map(|line| {
        let (header_name, value) = line.split_once(':')?;
        header_name
            .eq_ignore_ascii_case(name)
            .then_some(value.trim())
    })
}

fn request_has_pairing_token(request: &str, expected: &str) -> bool {
    let Some((_, query)) = request_target(request).and_then(|target| target.split_once('?')) else {
        return false;
    };
    query.split('&').any(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        key == "token" && value == expected
    })
}

fn request_host_is_allowed(
    request: &str,
    stream: &TcpStream,
    config: &RemoteControlConfig,
) -> bool {
    request_authority_is_allowed(
        request,
        stream.local_addr().ok().map(|address| address.ip()),
        config,
    )
}

fn request_authority_is_allowed(
    request: &str,
    local_ip: Option<std::net::IpAddr>,
    config: &RemoteControlConfig,
) -> bool {
    let Some(host) = request_header(request, "Host") else {
        return false;
    };
    let port = config.port;
    let mut allowed = vec![
        format!("localhost:{port}"),
        format!("127.0.0.1:{port}"),
        format!("[::1]:{port}"),
    ];
    if let Some(local_ip) = local_ip {
        allowed.push(match local_ip {
            std::net::IpAddr::V4(ip) => format!("{ip}:{port}"),
            std::net::IpAddr::V6(ip) => format!("[{ip}]:{port}"),
        });
    }
    if let Ok(ip) = config.bind_ip.trim().parse::<std::net::IpAddr>() {
        if !ip.is_unspecified() {
            allowed.push(match ip {
                std::net::IpAddr::V4(ip) => format!("{ip}:{port}"),
                std::net::IpAddr::V6(ip) => format!("[{ip}]:{port}"),
            });
        }
    }
    if !allowed
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(host))
    {
        return false;
    }
    request_header(request, "Origin").is_none_or(|origin| {
        origin.eq_ignore_ascii_case(&format!("http://{host}"))
            || origin.eq_ignore_ascii_case(&format!("https://{host}"))
    })
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

fn read_timecode_position_ms(value: &Value, key: &'static str) -> Result<u64, RemoteParseError> {
    let raw = value.get(key).ok_or(RemoteParseError::MissingField(key))?;
    if let Some(position_ms) = raw.as_u64() {
        return Ok(position_ms);
    }
    if let Some(position) = raw.as_str() {
        return parse_timecode_position_ms(position).ok_or(RemoteParseError::InvalidField(key));
    }
    Err(RemoteParseError::InvalidField(key))
}

fn read_i32(value: &Value, key: &'static str) -> Result<i32, RemoteParseError> {
    let number = value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or(RemoteParseError::MissingField(key))?;
    i32::try_from(number).map_err(|_| RemoteParseError::InvalidField(key))
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

fn read_clock_source(value: &Value, key: &'static str) -> Result<ClockSource, RemoteParseError> {
    let Some(source) = value.get(key) else {
        return Ok(ClockSource::AbletonLink);
    };
    let source = source.as_str().ok_or(RemoteParseError::InvalidField(key))?;
    parse_clock_source_label(source).ok_or(RemoteParseError::InvalidField(key))
}

fn read_optional_clock_source(
    value: &Value,
    key: &'static str,
) -> Result<Option<ClockSource>, RemoteParseError> {
    let Some(source) = value.get(key) else {
        return Ok(None);
    };
    if source.is_null() {
        return Ok(None);
    }
    let source = source.as_str().ok_or(RemoteParseError::InvalidField(key))?;
    parse_clock_source_label(source)
        .map(Some)
        .ok_or(RemoteParseError::InvalidField(key))
}

fn read_video_output_mapping(
    value: &Value,
    key: &'static str,
) -> Result<VideoOutputMapping, RemoteParseError> {
    let mapping = value.get(key).ok_or(RemoteParseError::MissingField(key))?;
    serde_json::from_value(mapping.clone()).map_err(|_| RemoteParseError::InvalidField(key))
}

fn read_video_output_mapping_field(
    value: &Value,
    key: &'static str,
) -> Result<String, RemoteParseError> {
    let field = read_string(value, key)?;
    canonical_video_output_mapping_field(&field)
        .map(str::to_string)
        .ok_or(RemoteParseError::InvalidField(key))
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
            request_from_text(r#"{"type":"getVideoRuntimeStatus"}"#),
            Ok(RemoteClientRequest::GetVideoRuntimeStatus)
        );
        assert_eq!(
            request_from_text(r#"{"type":"getVideoOutputRenderPlans"}"#),
            Ok(RemoteClientRequest::GetVideoOutputRenderPlans)
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
            event_from_text(
                r#"{"type":"setGroupAttribute","group_id":"front","attribute":"Dimmer","value":65535}"#
            ),
            Ok(RemoteInputEvent::SetGroupAttribute {
                group_id: "front".to_string(),
                attribute: "Dimmer".to_string(),
                value: 65_535,
            })
        );
        assert_eq!(
            event_from_text(
                r#"{"type":"setGroupAttribute","group_id":"front","attribute":"Dimmer","value":0.5}"#
            ),
            Ok(RemoteInputEvent::SetGroupAttribute {
                group_id: "front".to_string(),
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
            event_from_text(r#"{"type":"clearFixtureFlags","kind":"all"}"#),
            Ok(RemoteInputEvent::ClearFixtureFlags {
                kind: "all".to_string(),
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
            event_from_text(r#"{"type":"syncAbletonLinkClock","bpm":126,"beat_phase":0.25}"#),
            Ok(RemoteInputEvent::SyncExternalClock {
                bpm: 126.0,
                beat_phase: 0.25,
                source: ClockSource::AbletonLink,
            })
        );
        assert_eq!(
            event_from_text(
                r#"{"type":"syncExternalClock","bpm":124,"beat_phase":0.5,"source":"ltc"}"#
            ),
            Ok(RemoteInputEvent::SyncExternalClock {
                bpm: 124.0,
                beat_phase: 0.5,
                source: ClockSource::Ltc,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"resetTelemetry"}"#),
            Ok(RemoteInputEvent::ResetTelemetry)
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
        assert_eq!(
            event_from_text(r#"{"type":"seekTimelineBeat","direction":-1}"#),
            Ok(RemoteInputEvent::SeekTimelineBeat { direction: -1 })
        );
        assert_eq!(
            event_from_text(
                r#"{"type":"syncTimelineTimecode","position_ms":"01:02:03.500","source":"mtc"}"#
            ),
            Ok(RemoteInputEvent::SyncTimelineTimecode {
                position_ms: 3_723_500,
                source: ClockSource::MidiTimecode,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"syncTimelineTimecode","position_ms":6789}"#),
            Ok(RemoteInputEvent::SyncTimelineTimecode {
                position_ms: 6_789,
                source: ClockSource::Ltc,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setEffectEnabled","effect_id":7,"enabled":false}"#),
            Ok(RemoteInputEvent::SetEffectEnabled {
                effect_id: 7,
                enabled: false,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"setNodeGraphEnabled","graph_id":12,"enabled":true}"#),
            Ok(RemoteInputEvent::SetNodeGraphEnabled {
                graph_id: 12,
                enabled: true,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"moveEffect","effect_id":7,"delta":-1}"#),
            Ok(RemoteInputEvent::MoveEffect {
                effect_id: 7,
                delta: -1,
            })
        );
        assert_eq!(
            event_from_text(r#"{"type":"removeEffect","effect_id":7}"#),
            Ok(RemoteInputEvent::RemoveEffect { effect_id: 7 })
        );
        assert_eq!(
            event_from_text(r#"{"type":"moveEffect","effect_id":7,"delta":3}"#),
            Err(RemoteParseError::InvalidField("delta"))
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
            request_from_text(r#"{"type":"syncExternalVideoTransports"}"#),
            Ok(RemoteClientRequest::SyncExternalVideoTransports)
        );
        assert_eq!(
            event_from_text(r#"{"type":"syncExternalVideoTransports"}"#),
            Err(RemoteParseError::UnknownType(
                "syncExternalVideoTransports".to_string()
            ))
        );
        assert_eq!(
            event_from_text(r#"{"type":"allBlackout","enabled":true}"#),
            Ok(RemoteInputEvent::AllBlackout(true))
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
            event_from_text(r#"{"type":"jumpVideoCuePointRelative","layer_id":2,"direction":-1}"#),
            Ok(RemoteInputEvent::JumpVideoCuePointRelative {
                layer_id: 2,
                direction: -1,
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
        let mut mapping = VideoOutputMapping::default();
        mapping.offset_x = 0.25;
        mapping.aspect_ratio = 1.75;
        mapping.aspect_mode = protocol::VideoOutputAspectMode::Fit;
        mapping.keystone_y = -0.1;
        assert_eq!(
            event_from_text(
                r#"{"type":"setVideoOutputMapping","output_id":4,"mapping":{"offset_x":0.25,"offset_y":0,"scale_x":1,"scale_y":1,"rotation_deg":0,"aspect_ratio":1.75,"aspect_mode":"Fit","lens_distortion":0,"keystone_x":0,"keystone_y":-0.1,"corner_top_left_x":0,"corner_top_left_y":0,"corner_top_right_x":0,"corner_top_right_y":0,"corner_bottom_right_x":0,"corner_bottom_right_y":0,"corner_bottom_left_x":0,"corner_bottom_left_y":0}}"#
            ),
            Ok(RemoteInputEvent::SetVideoOutputMapping {
                output_id: 4,
                mapping,
            })
        );
        assert_eq!(
            event_from_text(
                r#"{"type":"setVideoOutputMappingField","output_id":4,"field":"key x","value":0.25}"#
            ),
            Ok(RemoteInputEvent::SetVideoOutputMappingField {
                output_id: 4,
                field: "keystone_x".to_string(),
                value: 0.25,
            })
        );
        assert_eq!(
            event_from_text(
                r#"{"type":"setVideoOutputMappingField","output_id":4,"field":"unknown","value":0.25}"#
            ),
            Err(RemoteParseError::InvalidField("field"))
        );
        assert_eq!(
            event_from_text(
                r#"{"type":"applyVideoOutputMappingPreset","output_id":4,"label":"Front Projector"}"#
            ),
            Ok(RemoteInputEvent::ApplyVideoOutputMappingPreset {
                output_id: 4,
                label: "Front Projector".to_string(),
            })
        );
        assert_eq!(
            event_from_text(
                r#"{"type":"fadeVideoLayerOpacity","layer_id":2,"opacity":0.5,"duration_ms":750}"#
            ),
            Ok(RemoteInputEvent::FadeVideoLayerOpacity {
                layer_id: 2,
                opacity: 0.5,
                duration_ms: 750,
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
        assert_eq!(
            request_from_text(r#"{"type":"getExternalVideoTransportStatus"}"#),
            Ok(RemoteClientRequest::GetExternalVideoTransportStatus)
        );
        assert_eq!(
            request_from_text(r#"{"type":"getExternalVideoIoPlans"}"#),
            Ok(RemoteClientRequest::GetExternalVideoIoPlans)
        );
        assert_eq!(
            event_from_text(r#"{"type":"getExternalVideoIoPlans"}"#),
            Err(RemoteParseError::UnknownType(
                "getExternalVideoIoPlans".to_string()
            ))
        );
        assert_eq!(
            event_from_text(r#"{"type":"getVideoOutputRenderPlans"}"#),
            Err(RemoteParseError::UnknownType(
                "getVideoOutputRenderPlans".to_string()
            ))
        );
        assert_eq!(
            event_from_text(r#"{"type":"getExternalVideoTransportStatus"}"#),
            Err(RemoteParseError::UnknownType(
                "getExternalVideoTransportStatus".to_string()
            ))
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
        assert!(page.contains("Syndocal Remote"));
        assert!(page.contains(r#"rel="manifest" href="/manifest.webmanifest""#));
        assert!(page.contains("apple-mobile-web-app-capable"));
        assert!(page.contains("serviceWorker"));
        assert!(page.contains("token=new URLSearchParams"));
        assert!(page.contains("/ws?token="));
        assert!(page.contains("remoteActiveCue"));
        assert!(page.contains("remoteRtState"));
        assert!(page.contains("remoteDmxState"));
        assert!(page.contains("remoteDmxRoutes"));
        assert!(page.contains("remoteLatencyState"));
        assert!(page.contains("remoteRtHealth"));
        assert!(page.contains("remoteDmxHealth"));
        assert!(page.contains("remoteDmxLastFrameError"));
        assert!(page.contains("renderRemoteDmxRoutes"));
        assert!(page.contains("last_dmx_route_results"));
        assert!(page.contains("remoteDmxRouteEndpoint"));
        assert!(page.contains("remoteDmxProtocolLabel"));
        assert!(page.contains("lowlat"));
        assert!(page.contains("jit ${formatRemoteMicros(jitterP99)}"));
        assert!(page.contains("resetTelemetry"));
        assert!(page.contains("renderLiveDesk"));
        assert!(page.contains("remoteTouchSurface"));
        assert!(page.contains("renderRemoteTouchSurface"));
        assert!(page.contains("snapshot.touch_surface"));
        assert!(page.contains("cueControls"));
        assert!(page.contains("remoteTouchSetValue"));
        assert!(page.contains("remoteTouchSetColor"));
        assert!(page.contains("new WebSocket"));
        assert!(page.contains("getSnapshot"));
        assert!(page.contains("getVideoRuntimeStatus"));
        assert!(page.contains("videoRuntimeBackends"));
        assert!(page.contains("renderVideoRuntimeStatus"));
        assert!(page.contains("getVideoOutputRenderPlans"));
        assert!(page.contains("videoOutputRenderPlans"));
        assert!(page.contains("videoOutputRenderPlanSummary"));
        assert!(page.contains("renderVideoOutputRenderPlans"));
        assert!(page.contains("remoteVideoOutputRenderPlanById"));
        assert!(page.contains("remoteVideoOutputRenderPlanDetail"));
        assert!(page.contains("remoteVideoOutputRenderPlanState"));
        assert!(page.contains("getExternalVideoIoPlans"));
        assert!(page.contains("externalVideoIoPlanRoutes"));
        assert!(page.contains("renderExternalVideoIoPlans"));
        assert!(page.contains("getExternalVideoTransportStatus"));
        assert!(page.contains("externalVideoTransportRoutes"));
        assert!(page.contains("renderExternalVideoTransportStatus"));
        assert!(page.contains("syncExternalVideoTransports"));
        assert!(page.contains("syncExternalVideoTransportsRemote"));
        assert!(page.contains("externalVideoTransportSyncSummary"));
        assert!(page.contains("externalVideoTransportSyncEvents"));
        assert!(page.contains("renderExternalVideoTransportSync"));
        assert!(page.contains("fixtureCount"));
        assert!(page.contains("effectCount"));
        assert!(page.contains("nodeGraphList"));
        assert!(page.contains("renderNodeGraphList"));
        assert!(page.contains("setRemoteNodeGraphEnabled"));
        assert!(page.contains(r#"type:"setNodeGraphEnabled""#));
        assert!(page.contains("fixtureFaderBank"));
        assert!(page.contains("groupId"));
        assert!(page.contains("selectedGroupId"));
        assert!(page.contains("remoteTargetInfo"));
        assert!(page.contains("renderRemoteTargetInfo"));
        assert!(page.contains("remoteSelectedGroupSubmaster"));
        assert!(page.contains("remoteTargetSubmaster"));
        assert!(page.contains("setSelectedTargetHighlight"));
        assert!(page.contains("setSelectedTargetSolo"));
        assert!(page.contains("setSelectedTargetPark"));
        assert!(page.contains("setGroupAttribute"));
        assert!(page.contains("remoteControlFixture"));
        assert!(page.contains("remoteVisualDesk"));
        assert!(page.contains("remoteDimmerQuick"));
        assert!(page.contains("remoteDimmerControl"));
        assert!(page.contains("setRemoteDimmerTarget"));
        assert!(page.contains("startRemoteDimmerBump"));
        assert!(page.contains("endRemoteDimmerBump"));
        assert!(page.contains("remoteColorWheelGrid"));
        assert!(page.contains("remoteColorWheelRows"));
        assert!(page.contains("remoteFunctionSwatchColor"));
        assert!(page.contains("remoteGoboWheelGrid"));
        assert!(page.contains("remoteGoboWheelRows"));
        assert!(page.contains("remoteGoboSlotPattern"));
        assert!(page.contains("remoteGoboSlotIcon"));
        assert!(page.contains("Gobo Slots"));
        assert!(page.contains("remoteOpticsGrid"));
        assert!(page.contains("remoteOpticsRows"));
        assert!(page.contains("remoteOpticsPreviewStyle"));
        assert!(page.contains("setRemoteOpticsFromPointer"));
        assert!(page.contains("keyRemoteOpticsValue"));
        assert!(page.contains("Optics"));
        assert!(page.contains("remoteFunctionGrid"));
        assert!(page.contains("remoteFunctionControlRows"));
        assert!(page.contains("setRemoteChannelFunction"));
        assert!(page.contains("remotePanTiltPad"));
        assert!(page.contains("remotePanTiltLimit"));
        assert!(page.contains("remotePanTiltMirrorRow"));
        assert!(page.contains("remoteSourcePanTiltValues"));
        assert!(page.contains("remotePanTiltTargetValues"));
        assert!(page.contains("setRemotePanTiltMirror"));
        assert!(page.contains("remotePositionFavoritesStorageKey"));
        assert!(page.contains("remotePositionFavoriteGrid"));
        assert!(page.contains("applyRemotePositionFavorite"));
        assert!(page.contains("addRemotePositionFavorite"));
        assert!(page.contains("remoteColorGrid"));
        assert!(page.contains("remoteColorPresetGrid"));
        assert!(page.contains("remoteColorPresets"));
        assert!(page.contains("remoteColorMatches"));
        assert!(page.contains("remoteNormalizeColorHex"));
        assert!(page.contains(r#"label:"Tungsten""#));
        assert!(page.contains(r#"label:"Daylight""#));
        assert!(page.contains("remoteExtraColorSliders"));
        assert!(page.contains("remoteColorAutoWhite"));
        assert!(page.contains("ColorAdd_W"));
        assert!(page.contains("renderRemoteVisualControls"));
        assert!(page.contains("setRemotePanTiltFromPointer"));
        assert!(page.contains("setRemoteFixtureColor"));
        assert!(page.contains("setRemoteExtraColorChannel"));
        assert!(page.contains("renderFixtureFaderBank"));
        assert!(page.contains("setFixtureAttributeFromInput"));
        assert!(page.contains("cueButtons"));
        assert!(page.contains("cuePad"));
        assert!(page.contains("moveCuePadBank"));
        assert!(page.contains("sceneSelector"));
        assert!(page.contains("Scene Search"));
        assert!(page.contains("renderSceneSelector"));
        assert!(page.contains("remoteSceneSearchText"));
        assert!(page.contains("remoteFilteredSceneRows"));
        assert!(page.contains("triggerFirstFilteredScene"));
        assert!(page.contains("remoteSceneFilterKeyDown"));
        assert!(page.contains("GO First"));
        assert!(page.contains("Enter GO"));
        assert!(page.contains("setSceneFilter"));
        assert!(page.contains("effectList"));
        assert!(page.contains("renderEffectList"));
        assert!(page.contains("remoteEffectTimingText"));
        assert!(page.contains("setRemoteEffectEnabled"));
        assert!(page.contains("moveRemoteEffect"));
        assert!(page.contains("removeRemoteEffect"));
        assert!(page.contains(r#"type:"setEffectEnabled""#));
        assert!(page.contains(r#"type:"moveEffect""#));
        assert!(page.contains(r#"type:"removeEffect""#));
        assert!(page.contains("layerList"));
        assert!(page.contains("videoOutputList"));
        assert!(page.contains("renderVideoOutputList"));
        assert!(page.contains("selectRemoteVideoOutput"));
        assert!(page.contains(".row.selected"));
        assert!(page.contains(">Sel</button>"));
        assert!(page.contains("remoteOutputMap"));
        assert!(page.contains("remoteStage"));
        assert!(page.contains("remoteStageGroupFilter"));
        assert!(page.contains("selectedRemoteStageGroupId"));
        assert!(page.contains("renderRemoteStage"));
        assert!(page.contains("remoteStageSurfaceHalfSize"));
        assert!(page.contains("remoteStageObjectCorners"));
        assert!(page.contains("remoteStageOutputCorners"));
        assert!(page.contains("remoteVideoOutputMappingPanel"));
        assert!(page.contains("setVideoOutputMapping"));
        assert!(page.contains("nudgeRemoteOutputMapping"));
        assert!(page.contains("stage_x"));
        assert!(page.contains("Stage X"));
        assert!(page.contains("Stage Y"));
        assert!(page.contains("resetRemoteOutputStage"));
        assert!(page.contains("remoteMappingFieldRange"));
        assert!(page.contains("remoteMapCenterHandle"));
        assert!(page.contains("remoteMapKeyHandle"));
        assert!(page.contains("remoteMapHandle"));
        assert!(page.contains("remoteMapModeRow"));
        assert!(page.contains("remoteMapPresetRow"));
        assert!(page.contains("remoteOutputAspectPresets"));
        assert!(page.contains("applyRemoteOutputMappingPreset"));
        assert!(page.contains(r#"type:"applyVideoOutputMappingPreset""#));
        assert!(page.contains(r#"type:"setVideoOutputMappingField""#));
        assert!(page.contains(r#"label:"4:3""#));
        assert!(page.contains(r#"label:"21:9""#));
        assert!(page.contains("remoteMapKeyPresetRow"));
        assert!(page.contains("remoteProjectorKeystonePresets"));
        assert!(page.contains("remoteProjectorKeystonePresetPoints"));
        assert!(page.contains("setRemoteOutputKeystonePreset"));
        assert!(page.contains(r#"label:"H -""#));
        assert!(page.contains(r#"label:"V +""#));
        assert!(page.contains("remoteMapKeyMini"));
        assert!(page.contains("remoteMapTuningGrid"));
        assert!(page.contains("setRemoteOutputMappingField"));
        assert!(page.contains("setRemoteOutputAspectMode"));
        assert!(page.contains("lens_distortion"));
        assert!(page.contains("Scale X"));
        assert!(page.contains("setRemoteProjectorCornerFromPointer"));
        assert!(page.contains("setRemoteProjectorOffsetFromPointer"));
        assert!(page.contains("setRemoteProjectorKeystoneFromPointer"));
        assert!(page.contains("fitRemoteOutputMapping"));
        assert!(page.contains("timelineInfo"));
        assert!(page.contains("timelineBeatInfo"));
        assert!(page.contains("timelineProgress"));
        assert!(page.contains("timelineSeekMs"));
        assert!(page.contains("timelineTimecodeMs"));
        assert!(page.contains("syncTimelineTimecodeRemote"));
        assert!(page.contains(r#"type:"syncTimelineTimecode""#));
        assert!(page.contains("setTimelinePlaying"));
        assert!(page.contains("seekTimelineAdjacentBeat"));
        assert!(page.contains(r#"type:"seekTimelineBeat""#));
        assert!(page.contains("linkPhase"));
        assert!(page.contains("syncAbletonLinkClock"));
        assert!(page.contains("setInterval(requestSnapshot,1000)"));
        assert!(page.contains("setVideoLayerEnabled"));
        assert!(page.contains("setVideoLayerSolo"));
        assert!(page.contains("fadeVideoLayerOpacity"));
        assert!(page.contains("videoLayerFadeMs"));
        assert!(page.contains("videoDeckControls"));
        assert!(page.contains("remoteVideoTimeline"));
        assert!(page.contains("remoteVideoTimelineCue"));
        assert!(page.contains("setRemoteVideoTimelinePosition"));
        assert!(page.contains("setRemoteVideoTimelineLoop"));
        assert!(page.contains("setLayerParamFromInput"));
        assert!(page.contains("setLayerLoopFromInputs"));
        assert!(page.contains("remoteBpmPresetRow"));
        assert!(page.contains("setLayerBpmLoopBars"));
        assert!(page.contains("matchLayerBpmLoopToCurrentLength"));
        assert!(page.contains("bpm_sync_loop_bars"));
        assert!(page.contains("setVideoLoop"));
        assert!(page.contains("setVideoOutputEnabled"));
        assert!(page.contains("setVideoOutputOpacityFromInput"));
        assert!(page.contains("setVideoOutputOpacity(${o.id},0)"));
        assert!(page.contains("setVideoOutputOpacity(${o.id},0.5)"));
        assert!(page.contains("fadeVideoOutputOpacity"));
        assert!(page.contains("setVideoOutputEnabled(${output.id},${!output.enabled})"));
        assert!(page.contains(r#"${output.enabled?"Off":"On"}"#));
        assert!(page.contains("repeat(5,minmax(48px,auto))"));
        assert!(page.contains(">Cut</button>"));
        assert!(page.contains(">Half</button>"));
        assert!(page.contains("setVideoPlaying"));
        assert!(page.contains("setAllBlackout"));
        assert!(page.contains(r#"type:"allBlackout""#));
        assert!(page.contains("seekVideoLayer"));
        assert!(page.contains("seekLayerByFrame"));
        assert!(page.contains("setLayerLoopBoundaryToCurrent"));
        assert!(page.contains("addVideoCuePoint"));
        assert!(page.contains("jumpVideoCuePoint"));
        assert!(page.contains("jumpVideoCuePointRelative"));
        assert!(page.contains("setFixtureHighlight"));
        assert!(page.contains("setFixtureSolo"));
        assert!(manifest.contains("application/manifest+json"));
        assert!(manifest.contains(r#""short_name":"Syndocal""#));
        assert!(manifest.contains(r#""icons""#));
        assert!(icon.contains("image/svg+xml"));
        assert!(icon.contains("<svg"));
        assert!(service_worker.contains("application/javascript"));
        assert!(service_worker.contains("CACHE_NAME"));
        assert!(service_worker.contains("syndocal-remote-v2"));
        assert!(service_worker.contains("url.searchParams.has(\"token\")"));
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

    #[test]
    fn serializes_video_runtime_status_response() {
        let response = video_runtime_status_response_json(&VideoRuntimeStatus {
            backends: vec![protocol::VideoBackendStatus {
                id: "ffmpeg".to_string(),
                label: "FFmpeg".to_string(),
                state: protocol::VideoBackendState::Missing,
                detail: "not found".to_string(),
            }],
        });

        assert!(response.contains(r#""ok":true"#));
        assert!(response.contains(r#""type":"videoRuntimeStatus""#));
        assert!(response.contains(r#""video_runtime_status""#));
        assert!(response.contains(r#""state":"Missing""#));
    }

    #[test]
    fn serializes_video_output_render_plans_response() {
        let response = video_output_render_plans_response_json(&json!([{
            "output_id": 7,
            "label": "Projector",
            "kind": "Display",
            "enabled": true,
            "width": 1920,
            "height": 1080,
            "output_blackout": false,
            "composition": {
                "composition_id": 2,
                "label": "Main",
                "layers": []
            }
        }]));

        assert!(response.contains(r#""ok":true"#));
        assert!(response.contains(r#""type":"videoOutputRenderPlans""#));
        assert!(response.contains(r#""video_output_render_plans""#));
        assert!(response.contains(r#""label":"Projector""#));
        assert!(response.contains(r#""composition_id":2"#));
    }

    #[test]
    fn serializes_external_video_io_plans_response() {
        let response = external_video_io_plans_response_json(&json!({
            "inputs": [{
                "layer_id": 3,
                "label": "Camera",
                "backend_id": "ndi",
                "endpoint_name": "Stage Cam",
                "ready": false,
                "live": false,
                "issue": "NDI SDK backend is not linked"
            }],
            "outputs": []
        }));

        assert!(response.contains(r#""ok":true"#));
        assert!(response.contains(r#""type":"externalVideoIoPlans""#));
        assert!(response.contains(r#""external_video_io_plans""#));
        assert!(response.contains(r#""backend_id":"ndi""#));
        assert!(response.contains(r#""issue":"NDI SDK backend is not linked""#));
    }

    #[test]
    fn serializes_external_video_transport_status_response() {
        let response = external_video_transport_status_response_json(&json!({
            "active_routes": [{
                "direction": "Output",
                "route_id": 4,
                "label": "Program",
                "backend_id": "spout",
                "endpoint_name": "Syndocal Stage"
            }],
            "active_count": 1
        }));

        assert!(response.contains(r#""ok":true"#));
        assert!(response.contains(r#""type":"externalVideoTransportStatus""#));
        assert!(response.contains(r#""external_video_transport_status""#));
        assert!(response.contains(r#""backend_id":"spout""#));
        assert!(response.contains(r#""active_count":1"#));
    }

    #[test]
    fn serializes_external_video_transport_sync_response() {
        let response = external_video_transport_sync_response_json(&json!({
            "report": {
                "started": [{
                    "direction": "Output",
                    "route_id": 4,
                    "label": "Program",
                    "backend_id": "spout",
                    "endpoint_name": "Syndocal Stage"
                }],
                "kept": [],
                "stopped": [],
                "blocked": [],
                "start_failed": [],
                "stop_failed": [],
                "idle": [],
                "active_count": 1
            },
            "events": [{
                "sequence": 1,
                "action": "Start",
                "route": {
                    "direction": "Output",
                    "route_id": 4,
                    "label": "Program",
                    "backend_id": "spout",
                    "endpoint_name": "Syndocal Stage"
                },
                "message": "Queued spout output external video route 'Syndocal Stage'"
            }]
        }));

        assert!(response.contains(r#""ok":true"#));
        assert!(response.contains(r#""type":"externalVideoTransportSync""#));
        assert!(response.contains(r#""external_video_transport_sync""#));
        assert!(response.contains(r#""action":"Start""#));
        assert!(response.contains(r#""active_count":1"#));
    }

    #[test]
    fn remote_server_rejects_missing_or_malformed_pairing_pin() {
        let missing = RemoteWsServer::start(RemoteControlConfig::default(), |_| {});
        assert!(matches!(missing, Err(RemoteWsError::InvalidPairingPin)));

        let malformed = RemoteWsServer::start(
            RemoteControlConfig {
                pairing_pin: "12ab56".to_string(),
                ..RemoteControlConfig::default()
            },
            |_| {},
        );
        assert!(matches!(malformed, Err(RemoteWsError::InvalidPairingPin)));

        let lan_without_opt_in = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "0.0.0.0".to_string(),
                pairing_pin: "123456".to_string(),
                ..RemoteControlConfig::default()
            },
            |_| {},
        );
        assert!(matches!(
            lan_without_opt_in,
            Err(RemoteWsError::LanAccessDisabled)
        ));

        let invalid_connections = RemoteWsServer::start(
            RemoteControlConfig {
                pairing_pin: "123456".to_string(),
                max_connections: 0,
                ..RemoteControlConfig::default()
            },
            |_| {},
        );
        assert!(matches!(
            invalid_connections,
            Err(RemoteWsError::InvalidConnectionLimit)
        ));

        let invalid_size = RemoteWsServer::start(
            RemoteControlConfig {
                pairing_pin: "123456".to_string(),
                max_message_bytes: 128,
                ..RemoteControlConfig::default()
            },
            |_| {},
        );
        assert!(matches!(
            invalid_size,
            Err(RemoteWsError::InvalidMessageSizeLimit)
        ));

        let invalid_rate = RemoteWsServer::start(
            RemoteControlConfig {
                pairing_pin: "123456".to_string(),
                max_messages_per_second: 0,
                ..RemoteControlConfig::default()
            },
            |_| {},
        );
        assert!(matches!(
            invalid_rate,
            Err(RemoteWsError::InvalidMessageRateLimit)
        ));
    }

    #[test]
    fn tracks_limits_and_disconnects_remote_clients() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let server = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                max_connections: 1,
                ..RemoteControlConfig::default()
            },
            |_| {},
        )
        .unwrap();

        let url = format!("ws://127.0.0.1:{port}/ws?token=123456");
        let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();
        for _ in 0..20 {
            if server.status().active_connections == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let status = server.status();
        assert_eq!(status.active_connections, 1);
        assert_eq!(status.clients.len(), 1);
        let client_id = status.clients[0].id;

        let second = tungstenite::connect(url.as_str());
        assert!(second.is_err());
        for _ in 0..20 {
            if server.status().rejected_connections == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(server.status().rejected_connections, 1);
        assert!(server.disconnect_client(client_id));
        for _ in 0..30 {
            if server.status().active_connections == 0 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(server.status().active_connections, 0);
        let _ = client.close(None);
    }

    #[test]
    fn pairing_token_is_required_and_must_match_exactly() {
        let valid = "GET /ws?token=123456 HTTP/1.1\r\nHost: localhost:9100\r\n\r\n";
        let missing = "GET /ws HTTP/1.1\r\nHost: localhost:9100\r\n\r\n";
        let wrong = "GET /ws?token=654321 HTTP/1.1\r\nHost: localhost:9100\r\n\r\n";

        assert!(request_has_pairing_token(valid, "123456"));
        assert!(!request_has_pairing_token(missing, "123456"));
        assert!(!request_has_pairing_token(wrong, "123456"));
        assert_eq!(request_path(valid), Some("/ws"));
    }

    #[test]
    fn host_and_origin_must_match_the_remote_endpoint() {
        let config = RemoteControlConfig {
            pairing_pin: "123456".to_string(),
            ..RemoteControlConfig::default()
        };
        let local_ip = Some("192.168.1.24".parse().unwrap());
        let valid = "GET /ws?token=123456 HTTP/1.1\r\nHost: 192.168.1.24:9100\r\nOrigin: http://192.168.1.24:9100\r\n\r\n";
        let bad_host = "GET /ws?token=123456 HTTP/1.1\r\nHost: attacker.example:9100\r\nOrigin: http://attacker.example:9100\r\n\r\n";
        let bad_origin = "GET /ws?token=123456 HTTP/1.1\r\nHost: 192.168.1.24:9100\r\nOrigin: https://attacker.example\r\n\r\n";

        assert!(request_authority_is_allowed(valid, local_ip, &config));
        assert!(!request_authority_is_allowed(bad_host, local_ip, &config));
        assert!(!request_authority_is_allowed(bad_origin, local_ip, &config));
    }
}

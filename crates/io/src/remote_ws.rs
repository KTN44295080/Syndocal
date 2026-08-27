use std::{
    collections::{hash_map::RandomState, BTreeMap, HashMap, HashSet, VecDeque},
    hash::{BuildHasher, Hash},
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(test)]
use std::sync::OnceLock;

use protocol::{
    canonical_video_output_mapping_field, ClockSource, CueId, DjLinkAck, DjLinkAckOutcome,
    DjLinkEnvelope, DjLinkMessageType, DjLinkRuntimeStatus, DjLinkTimelineState, EffectId,
    EngineSnapshot, FixtureId, NodeGraphId, OperatorSelectionContext, RemoteClientSummary,
    RemoteControlConfig, RemoteControlStatus, VideoLayerId, VideoOutputId, VideoOutputMapping,
    VideoParam, VideoRuntimeStatus, DJ_LINK_MAX_FRAME_BYTES,
};
use serde_json::{json, Value};
use thiserror::Error;
use tungstenite::{accept, Message};
use zeroize::Zeroize;

use crate::{parse_clock_source_label, parse_timecode_position_ms};

const HTTP_PEEK_SIZE: usize = 2048;
const REMOTE_SOCKET_READ_TIMEOUT: Duration = Duration::from_millis(100);
const REMOTE_SOCKET_WRITE_TIMEOUT: Duration = Duration::from_millis(250);
/// HELLO bearer credentials must not enter canonical identity, replay, or
/// session state after the authentication comparison succeeds.
const DJ_LINK_AUTH_TOKEN_CANONICAL_SENTINEL: &str = "00000000000000000000000000000000";
/// Production DJ Link dispatches wait at most three seconds for an Engine
/// acknowledgement.  The listener stop path adds one second for socket and
/// thread-join bookkeeping.  This is a testable service contract, not a
/// cancellation mechanism: a caller-supplied handler that violates it is
/// joined safely instead of being detached as a second authority.
pub const DJ_LINK_SHUTDOWN_DEADLINE: Duration = Duration::from_secs(4);
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
    if(msg.ok&&msg.type==="snapshot"){applySnapshot(msg.snapshot);requestVideoOutputRenderPlans();}
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

// The existing event type and AI0 inventory share one declarative source.
// A new enum variant therefore receives a fail-closed inventory entry without
// any source parsing or separately maintained mirror.
macro_rules! define_remote_input_event {
    ($(
        $variant:ident
        $(($($tuple_fields:tt)*))?
        $({$($struct_fields:tt)*})?
    ,)*) => {
        #[derive(Debug, Clone, PartialEq)]
        pub enum RemoteInputEvent {
            $(
                $variant $(($($tuple_fields)*))? $({$($struct_fields)*})?,
            )*
        }

        impl RemoteInputEvent {
            /// Exact variant names generated from this enum declaration.
            pub const CONTROL_PLANE_VARIANT_NAMES: &'static [&'static str] = &[
                $(stringify!($variant),)*
            ];
        }
    };
}

define_remote_input_event! {
    SetAttribute {
        fixture_id: FixtureId,
        attribute: String,
        value: u16,
    },
    SetOperatorSelection(OperatorSelectionContext),
    SetOperatorFeatureFader {
        target_index: usize,
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
        mapping: Box<VideoOutputMapping>,
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

macro_rules! define_remote_client_request {
    ($(
        $variant:ident
        $(($($tuple_fields:tt)*))?
        $({$($struct_fields:tt)*})?
    ,)*) => {
        #[derive(Debug, Clone, PartialEq)]
        pub enum RemoteClientRequest {
            $(
                $variant $(($($tuple_fields)*))? $({$($struct_fields)*})?,
            )*
        }

        impl RemoteClientRequest {
            /// Exact variant names generated from this enum declaration.
            pub const CONTROL_PLANE_VARIANT_NAMES: &'static [&'static str] = &[
                $(stringify!($variant),)*
            ];
        }
    };
}

define_remote_client_request! {
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
    #[error("remote listener requires web_remote_enabled or dj_link_enabled")]
    NoEnabledTransport,
    #[error("remote LAN access must be enabled before binding to a non-loopback address")]
    LanAccessDisabled,
    #[error("remote connection limit must be between 1 and 64")]
    InvalidConnectionLimit,
    #[error("remote message size limit must be between 1024 and 1048576 bytes")]
    InvalidMessageSizeLimit,
    #[error("remote message rate limit must be between 1 and 1000 messages per second")]
    InvalidMessageRateLimit,
    #[error("DJ Link requires an explicit configured bind IP, not a wildcard/auto address")]
    InvalidDjLinkBindAddress,
    #[error("DJ Link requires a dedicated machine-local token")]
    InvalidDjLinkToken,
    #[error("failed to bind remote WebSocket server {bind}: {source}")]
    Bind {
        bind: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteParseError {
    InvalidJson,
    MissingType,
    UnknownType,
    MissingField(&'static str),
    InvalidField(&'static str),
}

impl RemoteParseError {
    /// Renderer-originated input is untrusted. Keep its detail out of both
    /// diagnostics and response JSON; these stable codes are the entire
    /// public contract for parse failures.
    fn response_code(&self) -> &'static str {
        match self {
            Self::InvalidJson => "invalid_json",
            Self::MissingType => "missing_type",
            Self::UnknownType => "unknown_type",
            Self::MissingField(_) => "missing_field",
            Self::InvalidField(_) => "invalid_field",
        }
    }
}

fn remote_parse_error_response_json(error: &RemoteParseError) -> String {
    json!({ "ok": false, "error": error.response_code() }).to_string()
}

/// Result returned by the app's DJ Link authority lane after the transport
/// has authenticated, validated, and reserved the event identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DjLinkDispatchOutcome {
    Accepted {
        state_generation: u64,
    },
    NoMapping {
        state_generation: u64,
    },
    TimelineState {
        state_generation: u64,
        state: DjLinkTimelineState,
    },
    Rejected {
        code: String,
        state_generation: u64,
    },
    Busy {
        code: String,
        state_generation: u64,
    },
}

pub type DjLinkDispatchHandler = Arc<dyn Fn(DjLinkEnvelope) -> DjLinkDispatchOutcome + Send + Sync>;

#[derive(Debug, Clone)]
struct DjLinkTerminal {
    shape_digest: u64,
    ack: DjLinkAck,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct DjLinkInflight {
    shape_digest: u64,
    process_lifetime_shape_digest: u64,
    identity_digest: u64,
    sequence: u64,
    generation: u64,
    /// Admission-time classification of whether this validated envelope can
    /// cause a physical side effect.  Completion and retirement must consume
    /// this stored bit; re-parsing or reclassifying the payload after a
    /// handler ran would allow a dynamic StateSync to launder its fence.
    is_physical: bool,
}

#[derive(Debug, Clone)]
struct DjLinkSession {
    session_id: String,
    generation: u64,
    last_sequence: u64,
    /// Authenticated-session liveness: the timestamp of the most recently
    /// ADMITTED frame of any kind, not only Heartbeats. Continuous valid
    /// traffic (for example periodic SYNC observations) keeps the session
    /// alive; rejected or invalid frames never extend it.
    last_liveness: Instant,
    state_sync_received: bool,
    timeline_state_request_received: bool,
    snapshot_ready: bool,
    outbound: SyncSender<DjLinkOutboundState>,
    last_outbound_state: Option<DjLinkTimelineState>,
}

#[derive(Debug, Clone)]
struct DjLinkOutboundState {
    generation: u64,
    state: DjLinkTimelineState,
}

const DJ_LINK_OUTBOUND_QUEUE_LIMIT: usize = 8;
const DJ_LINK_OBSERVATION_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DjLinkEngineObservation {
    timeline_id: u64,
    playing: bool,
    position_ms: u64,
    duration_ms: u64,
    loop_active: bool,
    bpm_millis: u32,
}

impl DjLinkEngineObservation {
    fn from_snapshot(snapshot: &EngineSnapshot) -> Self {
        let loop_active = !matches!(
            snapshot.timeline.loop_runtime.status,
            protocol::TimelineLoopRuntimeStatus::Disabled
        );
        let bpm_millis = if snapshot.clock.bpm.is_finite() && snapshot.clock.bpm > 0.0 {
            (snapshot.clock.bpm * 1_000.0)
                .round()
                .clamp(1.0, u32::MAX as f32) as u32
        } else {
            0
        };
        Self {
            timeline_id: snapshot.timeline.id.0,
            playing: snapshot.timeline.playing,
            position_ms: snapshot.timeline.position_ms,
            duration_ms: snapshot.timeline.duration_ms,
            loop_active,
            bpm_millis,
        }
    }

    fn position_bars(self) -> u64 {
        if self.bpm_millis == 0 {
            return 0;
        }
        let bar_ms = (60_000_u64)
            .saturating_mul(u64::from(protocol::DJ_LINK_BEATS_PER_BAR))
            .checked_mul(1_000)
            .and_then(|scaled| scaled.checked_div(u64::from(self.bpm_millis)))
            .filter(|bar_ms| *bar_ms > 0)
            .unwrap_or(1);
        self.position_ms / bar_ms
    }

    fn state(self, previous: Option<Self>) -> protocol::DjLinkTimelineStateValue {
        if self.playing {
            return protocol::DjLinkTimelineStateValue::Running;
        }
        if previous.is_some_and(|before| {
            before.timeline_id == self.timeline_id
                && before.position_ms > 0
                && self.position_ms == 0
        }) {
            return protocol::DjLinkTimelineStateValue::Reset;
        }
        if self.duration_ms > 0 && self.position_ms >= self.duration_ms {
            return protocol::DjLinkTimelineStateValue::Ended;
        }
        if previous.is_some_and(|before| before.playing) {
            return protocol::DjLinkTimelineStateValue::Stopped;
        }
        if self.position_ms == 0 {
            protocol::DjLinkTimelineStateValue::Idle
        } else {
            protocol::DjLinkTimelineStateValue::Stopped
        }
    }

    fn semantic_key(
        self,
        previous: Option<Self>,
    ) -> (u64, protocol::DjLinkTimelineStateValue, bool) {
        (self.timeline_id, self.state(previous), self.loop_active)
    }
}

fn dj_link_state_truth_equal(left: &DjLinkTimelineState, right: &DjLinkTimelineState) -> bool {
    left.state == right.state
        && left.loop_active == right.loop_active
        && left.timeline_id == right.timeline_id
        && left.position_bars == right.position_bars
        && left.play_session_id == right.play_session_id
        && left.pedal_owner == right.pedal_owner
        && left.release_event_id == right.release_event_id
}

/// Process-lifetime physical-event fence shared by every listener instance
/// owned by one Syndocal process.  Session/receipt/dispatch state deliberately
/// remains on `DjLinkRegistry`, so stopping a listener retires that authority
/// without reopening a physical `(agentId,eventId)` identity.
#[derive(Debug)]
pub struct DjLinkProcessFence {
    /// Process-lifetime keyed SipHash digests for side-effectful identities,
    /// mapped to a separately keyed stable command-shape digest. Hash
    /// collisions are intentionally fail-closed: a colliding new identity is
    /// rejected, never executed.
    seen_events: HashMap<u64, u64>,
    identity_hasher: RandomState,
    shape_hasher: RandomState,
    side_effect_event_limit: usize,
    side_effect_capacity_latched: bool,
}

impl Default for DjLinkProcessFence {
    fn default() -> Self {
        Self {
            seen_events: HashMap::new(),
            identity_hasher: RandomState::new(),
            shape_hasher: RandomState::new(),
            side_effect_event_limit: DJ_LINK_SIDE_EFFECT_EVENT_LIMIT,
            side_effect_capacity_latched: false,
        }
    }
}

pub type DjLinkProcessFenceHandle = Arc<Mutex<DjLinkProcessFence>>;

pub fn new_dj_link_process_fence() -> DjLinkProcessFenceHandle {
    Arc::new(Mutex::new(DjLinkProcessFence::default()))
}

#[derive(Debug)]
struct DjLinkRegistry {
    sessions: HashMap<String, DjLinkSession>,
    terminals: BTreeMap<(String, String), DjLinkTerminal>,
    /// Insertion order for the bounded terminal set (oldest-first trim when
    /// live receipts exceed capacity after TTL purging).
    terminal_order: VecDeque<(String, String)>,
    /// The process-lifetime physical-event fence is injected by the owning
    /// application state. It outlives this listener instance, but is fresh for
    /// a fresh Syndocal process (and for every un-injected test server).
    process_fence: DjLinkProcessFenceHandle,
    inflight: HashMap<(String, String), DjLinkInflight>,
    /// A dispatch lease is a short-lived generation fence around the
    /// irreversible application handler call.  Session replacement must not
    /// succeed while this lease exists, otherwise a new HELLO could replace
    /// the generation after the final check but before physical dispatch.
    dispatch_leases: HashSet<(String, String, u64)>,
    next_generation: u64,
    next_outbound_sequence: u64,
    state_generation: u64,
    /// Set while listener shutdown retires this registry. Admission observes
    /// it under the same registry lock as reservation, closing the race where
    /// a worker could parse one last frame after the stop flag was raised.
    retired: bool,
    status: DjLinkRuntimeStatus,
}

impl Default for DjLinkRegistry {
    fn default() -> Self {
        Self::with_process_fence(new_dj_link_process_fence())
    }
}

impl DjLinkRegistry {
    fn with_process_fence(process_fence: DjLinkProcessFenceHandle) -> Self {
        Self {
            sessions: HashMap::new(),
            terminals: BTreeMap::new(),
            terminal_order: VecDeque::new(),
            process_fence,
            inflight: HashMap::new(),
            dispatch_leases: HashSet::new(),
            next_generation: 0,
            next_outbound_sequence: 0,
            state_generation: 0,
            retired: false,
            status: DjLinkRuntimeStatus::default(),
        }
    }
}

const DJ_LINK_TERMINAL_LIMIT: usize = 4_096;
const DJ_LINK_TERMINAL_TTL: Duration = Duration::from_secs(15 * 60);
const DJ_LINK_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);
/// Fixed product high-water: 262,144 compact `(identity_digest, shape_digest)`
/// tombstones cover more than one hour at the normal/default 60 frame/s rate,
/// even if every frame is side-effectful. A hostile/high-rate configuration
/// can exhaust this sooner; it then fails closed until process restart rather
/// than trading bounded memory for availability or replaying old effects.
const DJ_LINK_SIDE_EFFECT_EVENT_LIMIT: usize = 262_144;

#[cfg(test)]
type DjLinkPreDispatchHook = Arc<dyn Fn() + Send + Sync>;

#[cfg(test)]
static DJ_LINK_PRE_DISPATCH_HOOK: OnceLock<Mutex<Option<DjLinkPreDispatchHook>>> = OnceLock::new();

#[cfg(test)]
fn set_dj_link_pre_dispatch_hook(hook: Option<DjLinkPreDispatchHook>) {
    let slot = DJ_LINK_PRE_DISPATCH_HOOK.get_or_init(|| Mutex::new(None));
    if let Ok(mut current) = slot.lock() {
        *current = hook;
    }
}

#[cfg(test)]
struct DjLinkPreDispatchHookGuard;

#[cfg(test)]
impl Drop for DjLinkPreDispatchHookGuard {
    fn drop(&mut self) {
        set_dj_link_pre_dispatch_hook(None);
    }
}

#[cfg(test)]
fn install_dj_link_pre_dispatch_hook(hook: DjLinkPreDispatchHook) -> DjLinkPreDispatchHookGuard {
    set_dj_link_pre_dispatch_hook(Some(hook));
    DjLinkPreDispatchHookGuard
}

#[cfg(test)]
fn run_dj_link_pre_dispatch_hook() {
    let hook = DJ_LINK_PRE_DISPATCH_HOOK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|current| current.clone());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(not(test))]
#[inline]
fn run_dj_link_pre_dispatch_hook() {}

#[cfg(test)]
static DJ_LINK_AFTER_FINAL_CHECK_HOOK: OnceLock<Mutex<Option<DjLinkPreDispatchHook>>> =
    OnceLock::new();

#[cfg(test)]
fn set_dj_link_after_final_check_hook(hook: Option<DjLinkPreDispatchHook>) {
    let slot = DJ_LINK_AFTER_FINAL_CHECK_HOOK.get_or_init(|| Mutex::new(None));
    if let Ok(mut current) = slot.lock() {
        *current = hook;
    }
}

#[cfg(test)]
struct DjLinkAfterFinalCheckHookGuard;

#[cfg(test)]
impl Drop for DjLinkAfterFinalCheckHookGuard {
    fn drop(&mut self) {
        set_dj_link_after_final_check_hook(None);
    }
}

#[cfg(test)]
fn install_dj_link_after_final_check_hook(
    hook: DjLinkPreDispatchHook,
) -> DjLinkAfterFinalCheckHookGuard {
    set_dj_link_after_final_check_hook(Some(hook));
    DjLinkAfterFinalCheckHookGuard
}

#[cfg(test)]
fn run_dj_link_after_final_check_hook() {
    let hook = DJ_LINK_AFTER_FINAL_CHECK_HOOK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|current| current.clone());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(not(test))]
#[inline]
fn run_dj_link_after_final_check_hook() {}

#[cfg(test)]
static DJ_LINK_BEFORE_OUTBOUND_PERMIT_HOOK: OnceLock<Mutex<Option<DjLinkPreDispatchHook>>> =
    OnceLock::new();

#[cfg(test)]
fn set_dj_link_before_outbound_permit_hook(hook: Option<DjLinkPreDispatchHook>) {
    let slot = DJ_LINK_BEFORE_OUTBOUND_PERMIT_HOOK.get_or_init(|| Mutex::new(None));
    if let Ok(mut current) = slot.lock() {
        *current = hook;
    }
}

#[cfg(test)]
struct DjLinkBeforeOutboundPermitHookGuard;

#[cfg(test)]
impl Drop for DjLinkBeforeOutboundPermitHookGuard {
    fn drop(&mut self) {
        set_dj_link_before_outbound_permit_hook(None);
    }
}

#[cfg(test)]
fn install_dj_link_before_outbound_permit_hook(
    hook: DjLinkPreDispatchHook,
) -> DjLinkBeforeOutboundPermitHookGuard {
    set_dj_link_before_outbound_permit_hook(Some(hook));
    DjLinkBeforeOutboundPermitHookGuard
}

#[cfg(test)]
fn run_dj_link_before_outbound_permit_hook() {
    let hook = DJ_LINK_BEFORE_OUTBOUND_PERMIT_HOOK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|current| current.clone());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(not(test))]
#[inline]
fn run_dj_link_before_outbound_permit_hook() {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DjLinkAdmission {
    Accepted,
    /// The frame passed duplicate/conflict/order checks but was not reserved
    /// because the per-session rate window is exhausted.  It is deliberately
    /// non-terminal: a unique physical frame may retry after the window
    /// without consuming a permanent process-fence identity slot.
    RateLimited,
    Duplicate(DjLinkAck),
    Conflict,
    Rollback,
    Busy,
    ReplayNotRetained,
    SideEffectCapacityLatched,
    OrderBlocked,
}

struct DjLinkRegistrationTransport {
    peer: String,
    outbound: SyncSender<DjLinkOutboundState>,
    now: Instant,
}

impl DjLinkRegistry {
    #[cfg(test)]
    fn with_side_effect_event_limit(side_effect_event_limit: usize) -> Self {
        let registry = Self::default();
        if let Ok(mut fence) = registry.process_fence.lock() {
            fence.side_effect_event_limit = side_effect_event_limit;
        }
        registry
    }

    fn keyed_digest<T: Hash + ?Sized>(state: &RandomState, value: &T) -> u64 {
        state.hash_one(value)
    }

    fn identity_digest(&self, envelope: &DjLinkEnvelope) -> u64 {
        let fence = self
            .process_fence
            .lock()
            .expect("DJ Link process fence lock was poisoned");
        Self::keyed_digest(
            &fence.identity_hasher,
            &(envelope.agent_id.as_str(), envelope.event_id.as_str()),
        )
    }

    fn exact_shape_digest(&self, shape: &str) -> u64 {
        let fence = self
            .process_fence
            .lock()
            .expect("DJ Link process fence lock was poisoned");
        Self::keyed_digest(&fence.shape_hasher, shape)
    }

    fn process_lifetime_shape_digest(&self, shape: &str) -> u64 {
        let fence = self
            .process_fence
            .lock()
            .expect("DJ Link process fence lock was poisoned");
        Self::keyed_digest(&fence.shape_hasher, &Self::process_lifetime_shape(shape))
    }

    fn register(
        &mut self,
        agent_id: &str,
        session_id: &str,
        sequence: u64,
        hello_event_id: &str,
        transport: DjLinkRegistrationTransport,
    ) -> Result<u64, String> {
        let DjLinkRegistrationTransport {
            peer,
            outbound,
            now,
        } = transport;
        let hello_key = (agent_id.to_string(), hello_event_id.to_string());
        if !self.dispatch_leases.is_empty() {
            return Err("DJ Link session replacement is busy".to_string());
        }
        // A new authenticated HELLO must be able to supersede an older
        // session even while that session has an admitted event in flight.
        // The old handler is generation-fenced at the final dispatch and
        // terminalization boundaries below; blocking replacement here would
        // leave that old physical path authoritative.
        self.next_generation = self
            .next_generation
            .checked_add(1)
            .ok_or_else(|| "DJ Link session generation exhausted".to_string())?;
        let generation = self.next_generation;
        // DJ Link has one authenticated agent authority at a time.  A new
        // authenticated session supersedes every older session; old close
        // callbacks are generation-checked and cannot clear the replacement.
        self.sessions.clear();
        self.sessions.insert(
            agent_id.to_string(),
            DjLinkSession {
                session_id: session_id.to_string(),
                generation,
                last_sequence: sequence,
                last_liveness: now,
                state_sync_received: false,
                timeline_state_request_received: false,
                snapshot_ready: false,
                outbound,
                last_outbound_state: None,
            },
        );
        self.status = DjLinkRuntimeStatus {
            connected: true,
            peer: Some(peer),
            generation,
            agent_id: Some(agent_id.to_string()),
            session_id: Some(session_id.to_string()),
            snapshot_ready: false,
            ..DjLinkRuntimeStatus::default()
        };
        let hello_inflight = self
            .inflight
            .get_mut(&hello_key)
            .ok_or_else(|| "DJ Link HELLO identity was not reserved".to_string())?;
        hello_inflight.generation = generation;
        Ok(generation)
    }

    fn is_current(&self, agent_id: &str, session_id: &str, generation: u64) -> bool {
        self.sessions.get(agent_id).is_some_and(|session| {
            session.session_id == session_id && session.generation == generation
        })
    }

    fn order_allows(&self, envelope: &DjLinkEnvelope, generation: u64) -> bool {
        let Some(session) = self.sessions.get(&envelope.agent_id).filter(|session| {
            session.session_id == envelope.session_id && session.generation == generation
        }) else {
            return false;
        };
        match envelope.message_type {
            DjLinkMessageType::Heartbeat => true,
            // STATE_SYNC is idempotent, nonphysical observation traffic. The
            // first observation opens the snapshot gate; periodic observations
            // must remain admissible afterwards because they are the agent's
            // continuous liveness/status channel. Blocking them post-ready
            // would wedge the session until the heartbeat timeout.
            DjLinkMessageType::StateSync => true,
            DjLinkMessageType::TimelineStateRequest => {
                session.state_sync_received && !session.snapshot_ready
            }
            _ => session.snapshot_ready,
        }
    }

    fn note_state_sync(&mut self, envelope: &DjLinkEnvelope, generation: u64) {
        if let Some(session) = self.sessions.get_mut(&envelope.agent_id).filter(|session| {
            session.session_id == envelope.session_id && session.generation == generation
        }) {
            session.state_sync_received = true;
        }
    }

    fn note_timeline_state_request(&mut self, envelope: &DjLinkEnvelope, generation: u64) {
        if let Some(session) = self.sessions.get_mut(&envelope.agent_id).filter(|session| {
            session.session_id == envelope.session_id && session.generation == generation
        }) {
            session.timeline_state_request_received = true;
        }
    }

    fn mark_snapshot_ready(&mut self, envelope: &DjLinkEnvelope, generation: u64) {
        if let Some(session) = self.sessions.get_mut(&envelope.agent_id).filter(|session| {
            session.session_id == envelope.session_id && session.generation == generation
        }) {
            if session.state_sync_received && session.timeline_state_request_received {
                session.snapshot_ready = true;
                self.status.snapshot_ready = true;
            }
        }
    }

    fn note_outbound_state(&mut self, state: &DjLinkTimelineState, event_id: &str, sequence: u64) {
        self.status.authoritative_state = Some(state.state);
        self.status.timeline_id = Some(state.timeline_id.clone());
        self.status.position_bars = Some(state.position_bars);
        self.status.loop_active = state.loop_active;
        self.status.last_outbound_event_id = Some(event_id.to_string());
        self.status.last_outbound_sequence = Some(sequence);
        self.status.last_outbound_delivery = Some("delivered".to_string());
        if let Some(session) = self.sessions.values_mut().find(|session| {
            session.generation == self.status.generation
                && session
                    .last_outbound_state
                    .as_ref()
                    .is_none_or(|previous| !dj_link_state_truth_equal(previous, state))
        }) {
            session.last_outbound_state = Some(state.clone());
        }
    }

    fn should_deliver_outbound_state(
        &self,
        agent_id: &str,
        session_id: &str,
        generation: u64,
        state: &DjLinkTimelineState,
    ) -> bool {
        self.sessions.get(agent_id).is_some_and(|session| {
            session.session_id == session_id
                && session.generation == generation
                && session.snapshot_ready
                && session
                    .last_outbound_state
                    .as_ref()
                    .is_none_or(|previous| !dj_link_state_truth_equal(previous, state))
        })
    }

    /// Queue one authoritative state transition for the current, snapshot-ready
    /// session. The queue is bounded and replacement/disconnect/queue failure
    /// retires the current authority instead of allowing stale delivery.
    fn queue_outbound_state(&mut self, mut state: DjLinkTimelineState) -> Result<bool, String> {
        state.validate()?;
        let Some((agent_id, generation, sender, previous)) =
            self.sessions.iter().next().map(|(agent_id, session)| {
                (
                    agent_id.clone(),
                    session.generation,
                    session.outbound.clone(),
                    session.last_outbound_state.clone(),
                )
            })
        else {
            return Ok(false);
        };
        if !self
            .sessions
            .get(&agent_id)
            .is_some_and(|session| session.snapshot_ready)
        {
            return Ok(false);
        }
        if previous
            .as_ref()
            .is_some_and(|previous| dj_link_state_truth_equal(previous, &state))
        {
            return Ok(false);
        }
        self.next_outbound_sequence = self
            .next_outbound_sequence
            .checked_add(1)
            .ok_or_else(|| "DJ Link outbound state sequence exhausted".to_string())?;
        state.event_id = format!("syndocal-dj-state-{}", self.next_outbound_sequence);
        state.sequence = self.next_outbound_sequence;
        state.validate()?;
        let queued = DjLinkOutboundState { generation, state };
        match sender.try_send(queued) {
            Ok(()) => Ok(true),
            Err(TrySendError::Full(_)) => {
                self.sessions.remove(&agent_id);
                self.status.connected = false;
                self.status.snapshot_ready = false;
                Err(format!(
                    "DJ Link outbound queue exceeded {DJ_LINK_OUTBOUND_QUEUE_LIMIT}"
                ))
            }
            Err(TrySendError::Disconnected(_)) => {
                self.sessions.remove(&agent_id);
                self.status.connected = false;
                self.status.snapshot_ready = false;
                Err("DJ Link outbound session disconnected".to_string())
            }
        }
    }

    fn begin_dispatch(&mut self, envelope: &DjLinkEnvelope, generation: u64) -> Result<(), String> {
        self.begin_dispatch_for_session(&envelope.agent_id, &envelope.session_id, generation)
    }

    fn begin_dispatch_for_session(
        &mut self,
        agent_id: &str,
        session_id: &str,
        generation: u64,
    ) -> Result<(), String> {
        if !self.is_current(agent_id, session_id, generation) {
            return Err("DJ Link session is stale before dispatch".to_string());
        }
        let key = (agent_id.to_string(), session_id.to_string(), generation);
        if !self.dispatch_leases.insert(key) {
            return Err("DJ Link dispatch is already in flight".to_string());
        }
        Ok(())
    }

    fn release_dispatch(&mut self, agent_id: &str, session_id: &str, generation: u64) {
        self.dispatch_leases
            .remove(&(agent_id.to_string(), session_id.to_string(), generation));
    }

    #[cfg(test)]
    fn admit(
        &mut self,
        envelope: &DjLinkEnvelope,
        shape: &str,
        generation: u64,
        now: Instant,
    ) -> Result<DjLinkAdmission, String> {
        self.admit_with_rate_limit(envelope, shape, generation, now, false)
    }

    fn admit_with_rate_limit(
        &mut self,
        envelope: &DjLinkEnvelope,
        shape: &str,
        generation: u64,
        now: Instant,
        rate_limited: bool,
    ) -> Result<DjLinkAdmission, String> {
        if self.retired {
            return Err("DJ Link listener registry is retired".to_string());
        }
        // Expired receipts must not shadow the permanent tombstone. In
        // particular, a same-shape physical retry after TTL is rejected as a
        // non-retained replay rather than misclassified as a conflict.
        self.purge_terminals(now);
        let key = (envelope.agent_id.clone(), envelope.event_id.clone());
        let identity_digest = self.identity_digest(envelope);
        let shape_digest = self.exact_shape_digest(shape);
        let process_lifetime_shape_digest = self.process_lifetime_shape_digest(shape);
        // The envelope has already passed `canonical_shape()`/validation.
        // Classify physical impact exactly once at admission and carry the
        // result through every later lifecycle transition.
        let is_physical = Self::classify_physical_envelope(envelope);
        if let Some(existing) = self.terminals.get(&key) {
            if existing.shape_digest == shape_digest {
                return Ok(DjLinkAdmission::Duplicate(existing.ack.clone()));
            }
            return Ok(DjLinkAdmission::Conflict);
        }
        let process_fence = self
            .process_fence
            .lock()
            .expect("DJ Link process fence lock was poisoned");
        if let Some(existing_shape_digest) = process_fence.seen_events.get(&identity_digest) {
            let admission = if *existing_shape_digest == process_lifetime_shape_digest {
                DjLinkAdmission::ReplayNotRetained
            } else {
                DjLinkAdmission::Conflict
            };
            drop(process_fence);
            return Ok(admission);
        }
        drop(process_fence);
        if let Some(existing) = self.inflight.get(&key) {
            if existing.shape_digest == shape_digest {
                return Ok(DjLinkAdmission::Busy);
            }
            return Ok(DjLinkAdmission::Conflict);
        }
        if self
            .inflight
            .values()
            .any(|inflight| inflight.is_physical && inflight.identity_digest == identity_digest)
        {
            // A keyed identity-digest collision with in-flight physical work
            // is indistinguishable without retaining the long strings. Refuse
            // it as conflict so collision can never become double execution.
            return Ok(DjLinkAdmission::Conflict);
        }
        let session = self
            .sessions
            .get(&envelope.agent_id)
            .filter(|session| {
                session.session_id == envelope.session_id && session.generation == generation
            })
            .ok_or_else(|| "DJ Link session is stale".to_string())?;
        let highest_inflight_sequence = self
            .inflight
            .iter()
            .filter(|((agent_id, _), _)| agent_id == &envelope.agent_id)
            .map(|(_, inflight)| inflight.sequence)
            .max()
            .unwrap_or(session.last_sequence);
        if envelope.sequence <= session.last_sequence
            || envelope.sequence <= highest_inflight_sequence
        {
            return Ok(DjLinkAdmission::Rollback);
        }
        if !self.order_allows(envelope, generation) {
            return Ok(DjLinkAdmission::OrderBlocked);
        }
        // Check the rate window after duplicate/conflict/inflight/order
        // classification, but before side-effect capacity reservation.  A
        // unique over-limit physical frame is therefore safely retryable and
        // cannot trip or consume the process-lifetime identity high-water.
        if rate_limited {
            return Ok(DjLinkAdmission::RateLimited);
        }
        if is_physical {
            let mut process_fence = self
                .process_fence
                .lock()
                .expect("DJ Link process fence lock was poisoned");
            if process_fence.side_effect_capacity_latched {
                return Ok(DjLinkAdmission::SideEffectCapacityLatched);
            }
            let reserved = self
                .inflight
                .values()
                .filter(|inflight| inflight.is_physical)
                .count();
            if process_fence.seen_events.len().saturating_add(reserved)
                >= process_fence.side_effect_event_limit
            {
                // Never evict an identity to make room: that would reopen an
                // old physical event. Once tripped, the latch remains closed
                // across aborts and session replacement until process restart.
                process_fence.side_effect_capacity_latched = true;
                return Ok(DjLinkAdmission::SideEffectCapacityLatched);
            }
        }
        self.inflight.insert(
            key,
            DjLinkInflight {
                shape_digest,
                process_lifetime_shape_digest,
                identity_digest,
                sequence: envelope.sequence,
                generation,
                is_physical,
            },
        );
        // The frame is now admitted: refresh authenticated-session liveness
        // so any continuous stream of valid frames (SYNC, loop state, beat
        // jumps, heartbeats, ...) keeps the session alive. Every rejection,
        // replay, conflict, rollback, rate-limit, and stale classification
        // returns above this point and never touches liveness.
        if let Some(session) = self.sessions.get_mut(&envelope.agent_id).filter(|session| {
            session.session_id == envelope.session_id && session.generation == generation
        }) {
            session.last_liveness = now;
        }
        Ok(DjLinkAdmission::Accepted)
    }

    fn admit_hello(
        &mut self,
        envelope: &DjLinkEnvelope,
        shape: &str,
        now: Instant,
    ) -> DjLinkAdmission {
        if self.retired {
            return DjLinkAdmission::OrderBlocked;
        }
        self.purge_terminals(now);
        let key = (envelope.agent_id.clone(), envelope.event_id.clone());
        let identity_digest = self.identity_digest(envelope);
        let shape_digest = self.exact_shape_digest(shape);
        let process_lifetime_shape_digest = self.process_lifetime_shape_digest(shape);
        if let Some(existing) = self.terminals.get(&key) {
            return if existing.shape_digest == shape_digest {
                DjLinkAdmission::Duplicate(existing.ack.clone())
            } else {
                DjLinkAdmission::Conflict
            };
        }
        let process_fence = self
            .process_fence
            .lock()
            .expect("DJ Link process fence lock was poisoned");
        if let Some(existing_shape_digest) = process_fence.seen_events.get(&identity_digest) {
            let admission = if *existing_shape_digest == process_lifetime_shape_digest {
                DjLinkAdmission::ReplayNotRetained
            } else {
                DjLinkAdmission::Conflict
            };
            drop(process_fence);
            return admission;
        }
        drop(process_fence);
        if let Some(existing) = self.inflight.get(&key) {
            return if existing.shape_digest == shape_digest {
                DjLinkAdmission::Busy
            } else {
                DjLinkAdmission::Conflict
            };
        }
        self.inflight.insert(
            key,
            DjLinkInflight {
                shape_digest,
                process_lifetime_shape_digest,
                identity_digest,
                sequence: envelope.sequence,
                generation: 0,
                is_physical: false,
            },
        );
        DjLinkAdmission::Accepted
    }

    fn complete(
        &mut self,
        envelope: &DjLinkEnvelope,
        generation: u64,
        shape: String,
        ack: DjLinkAck,
        now: Instant,
    ) -> Result<(), String> {
        let key = (envelope.agent_id.clone(), envelope.event_id.clone());
        if !self.is_current(&envelope.agent_id, &envelope.session_id, generation) {
            self.reject_inflight(envelope, generation);
            return Err("DJ Link session became stale before terminalization".to_string());
        }
        self.purge_terminals(now);
        // Capacity is reclaimed, never a terminalization failure: trimming
        // the oldest receipt only downgrades an exact Duplicate ack into a
        // tombstone rejection, so it can never re-execute a physical effect.
        self.trim_terminals_for_insert(&key);
        if let Some(inflight) = self
            .inflight
            .get(&key)
            .cloned()
            .filter(|inflight| inflight.generation == generation && inflight.is_physical)
        {
            let mut process_fence = self
                .process_fence
                .lock()
                .expect("DJ Link process fence lock was poisoned");
            process_fence.seen_events.insert(
                inflight.identity_digest,
                inflight.process_lifetime_shape_digest,
            );
        }
        self.inflight.remove(&key);
        if let Some(session) = self.sessions.get_mut(&envelope.agent_id).filter(|session| {
            session.session_id == envelope.session_id && session.generation == generation
        }) {
            session.last_sequence = envelope.sequence;
        }
        self.state_generation = self.state_generation.max(ack.state_generation);
        let replaced = self
            .terminals
            .insert(
                key.clone(),
                DjLinkTerminal {
                    shape_digest: self.exact_shape_digest(&shape),
                    ack,
                    expires_at: now + DJ_LINK_TERMINAL_TTL,
                },
            )
            .is_some();
        debug_assert!(
            !replaced,
            "terminal receipts are completed at most once per identity"
        );
        if !replaced {
            self.terminal_order.push_back(key);
        }
        Ok(())
    }

    /// Classify one already-validated envelope at admission. Physical
    /// commands are fenced for process lifetime. HELLO, HEARTBEAT, timeline
    /// state requests, and STATE_SYNC observations are idempotent status
    /// traffic and never consume the fence.
    fn classify_physical_envelope(envelope: &DjLinkEnvelope) -> bool {
        !matches!(
            envelope.message_type,
            DjLinkMessageType::Hello
                | DjLinkMessageType::Heartbeat
                | DjLinkMessageType::TimelineStateRequest
                | DjLinkMessageType::StateSync
        )
    }

    /// Terminal receipts require byte-semantic identity, including session
    /// and sequence, to replay the exact ACK. The longer-lived physical-event
    /// fence instead compares the stable command intent: reconnect and a
    /// higher transport sequence do not make the same `(agent,event)` fresh,
    /// while a changed type/payload remains a conflict.
    fn process_lifetime_shape(shape: &str) -> String {
        let Ok(mut value) = serde_json::from_str::<Value>(shape) else {
            return shape.to_string();
        };
        if let Some(object) = value.as_object_mut() {
            object.remove("sessionId");
            object.remove("sequence");
        }
        serde_json::to_string(&value).unwrap_or_else(|_| shape.to_string())
    }

    /// Make room for one more live terminal receipt by trimming the oldest.
    /// Called after TTL purging, so this only engages when more receipts
    /// arrive within one TTL window than the configured capacity; completion
    /// must never fail for capacity reasons because that path is fail-closed
    /// against re-execution but wedges the transport permanently.
    fn trim_terminals_for_insert(&mut self, key: &(String, String)) {
        while self.terminals.len() >= DJ_LINK_TERMINAL_LIMIT && !self.terminals.contains_key(key) {
            let victim = loop {
                match self.terminal_order.pop_front() {
                    Some(candidate) if self.terminals.contains_key(&candidate) => break candidate,
                    Some(_) => continue,
                    None => return,
                }
            };
            self.terminals.remove(&victim);
        }
    }

    fn purge_terminals(&mut self, now: Instant) {
        self.terminals
            .retain(|_, terminal| terminal.expires_at > now);
        // retain() drops map entries without touching the insertion-order
        // deque; compact it lazily so a long soak cannot accumulate orphan
        // keys.
        if self.terminal_order.len() > self.terminals.len() * 2 + 64 {
            self.terminal_order
                .retain(|key| self.terminals.contains_key(key));
        }
    }

    fn close_if_current(&mut self, agent_id: &str, session_id: &str, generation: u64) {
        if self.is_current(agent_id, session_id, generation) {
            self.sessions.remove(agent_id);
            self.status.connected = false;
            self.status.age_ms = None;
            self.status.snapshot_ready = false;
        }
    }

    fn abort_inflight(&mut self, envelope: &DjLinkEnvelope, generation: u64) {
        let key = (envelope.agent_id.clone(), envelope.event_id.clone());
        if self
            .inflight
            .get(&key)
            .is_some_and(|inflight| inflight.generation == generation)
        {
            self.inflight.remove(&key);
        }
    }

    /// Permanently tombstone a side-effectful identity when an admitted
    /// handler loses its session generation. A later session has a fresh
    /// sequence floor, so retaining this process-lifetime identity fence is
    /// the only way to prevent replay laundering across reconnection.
    fn reject_inflight(&mut self, envelope: &DjLinkEnvelope, generation: u64) {
        let key = (envelope.agent_id.clone(), envelope.event_id.clone());
        let Some(inflight) = self
            .inflight
            .get(&key)
            .filter(|inflight| inflight.generation == generation)
            .cloned()
        else {
            return;
        };
        self.inflight.remove(&key);
        if inflight.is_physical {
            let mut process_fence = self
                .process_fence
                .lock()
                .expect("DJ Link process fence lock was poisoned");
            process_fence.seen_events.insert(
                inflight.identity_digest,
                inflight.process_lifetime_shape_digest,
            );
        }
    }

    /// Preserve every admitted physical identity when a listener is retired
    /// before its worker can publish a terminal receipt.  Rate-limited frames
    /// never enter `inflight`, so they remain retryable across a listener
    /// restart; an admitted frame is fenced before the instance is dropped.
    fn fence_inflight_side_effects(&mut self) {
        let mut process_fence = self
            .process_fence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for inflight in self.inflight.values() {
            if inflight.is_physical {
                process_fence
                    .seen_events
                    .entry(inflight.identity_digest)
                    .or_insert(inflight.process_lifetime_shape_digest);
            }
        }
    }

    /// Retire this listener authority and fence every physical reservation as
    /// one registry-locked operation. A worker which reaches admission after
    /// this point fails before creating a new reservation; a worker which was
    /// already admitted is copied into the shared fence here.
    fn retire(&mut self) {
        self.retired = true;
        self.fence_inflight_side_effects();
    }

    #[cfg(test)]
    fn terminal_count(&self) -> usize {
        self.terminals.len()
    }

    #[cfg(test)]
    fn seen_event_count(&self) -> usize {
        self.process_fence
            .lock()
            .map(|fence| fence.seen_events.len())
            .unwrap_or(0)
    }

    #[cfg(test)]
    fn seen_event_contains(&self, identity_digest: u64) -> bool {
        self.process_fence
            .lock()
            .is_ok_and(|fence| fence.seen_events.contains_key(&identity_digest))
    }

    #[cfg(test)]
    fn side_effect_capacity_latched(&self) -> bool {
        self.process_fence
            .lock()
            .is_ok_and(|fence| fence.side_effect_capacity_latched)
    }

    #[cfg(test)]
    fn register_test_session(
        &mut self,
        agent_id: &str,
        session_id: &str,
        sequence: u64,
        peer: &str,
        now: Instant,
    ) -> u64 {
        let (outbound, _receiver) = mpsc::sync_channel(DJ_LINK_OUTBOUND_QUEUE_LIMIT);
        let event_id = format!("test-hello-{agent_id}-{session_id}");
        self.inflight.insert(
            (agent_id.to_string(), event_id.clone()),
            DjLinkInflight {
                shape_digest: 0,
                process_lifetime_shape_digest: 0,
                identity_digest: 0,
                sequence,
                generation: 0,
                is_physical: false,
            },
        );
        let generation = self
            .register(
                agent_id,
                session_id,
                sequence,
                &event_id,
                DjLinkRegistrationTransport {
                    peer: peer.to_string(),
                    outbound,
                    now,
                },
            )
            .expect("test session registration");
        self.inflight.remove(&(agent_id.to_string(), event_id));
        generation
    }
}

/// Owns the registry-side dispatch lease until the application handler has
/// returned and its terminal receipt has been committed.  Drop is the single
/// release path, including handler panic/unwind paths.
struct DjLinkDispatchLease {
    registry: Arc<Mutex<DjLinkRegistry>>,
    agent_id: String,
    session_id: String,
    generation: u64,
}

impl Drop for DjLinkDispatchLease {
    fn drop(&mut self) {
        if let Ok(mut registry) = self.registry.lock() {
            registry.release_dispatch(&self.agent_id, &self.session_id, self.generation);
        }
    }
}

/// Owns one admitted DJ Link connection slot.  The slot is released even if
/// any code below the transport boundary unwinds, so a handler panic cannot
/// permanently consume the configured connection capacity.
struct DjLinkConnectionSlotGuard {
    slots: Arc<AtomicU64>,
}

impl Drop for DjLinkConnectionSlotGuard {
    fn drop(&mut self) {
        self.slots.fetch_sub(1, Ordering::AcqRel);
    }
}

pub struct RemoteWsServer {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    clients: Arc<Mutex<HashMap<u64, RemoteClientState>>>,
    rejected_connections: Arc<AtomicU64>,
    dj_link_registry: Arc<Mutex<DjLinkRegistry>>,
    /// Immutable transport mode captured at start time. The generic Web
    /// Remote can never be toggled onto a running listener; only a replaced
    /// listener may change it.
    web_remote_enabled: bool,
    /// Immutable DJ Link mode, carried independently from generic Web Remote
    /// so status consumers can distinguish a DJ-only listener.
    dj_link_enabled: bool,
    /// Every accepted client worker is retained by the server until shutdown.
    /// A detached worker could otherwise keep a socket (and a DJ generation)
    /// alive after `RemoteWsServer` has been dropped.
    client_workers: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// Clones of accepted sockets used only to unblock a worker before join.
    /// No network write is performed while either this or the DJ registry is
    /// locked.
    shutdown_sockets: Arc<Mutex<HashMap<u64, TcpStream>>>,
    dj_link_connections: Arc<AtomicU64>,
    #[cfg(test)]
    local_addr: std::net::SocketAddr,
}

struct RemoteClientState {
    summary: RemoteClientSummary,
    disconnect: Arc<AtomicBool>,
}

/// Owns one registered generic Web Remote client slot until its worker has
/// finished.  The worker boundary catches callback/provider panics, so this
/// guard must perform the registry removal during unwind as well as on the
/// normal return path.
struct RemoteClientRegistrationGuard {
    clients: Arc<Mutex<HashMap<u64, RemoteClientState>>>,
    client_id: u64,
}

impl Drop for RemoteClientRegistrationGuard {
    fn drop(&mut self) {
        let mut registry = self
            .clients
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        registry.remove(&self.client_id);
    }
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
            (
                video_runtime_status_provider,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
        )
    }

    pub fn start_with_snapshot_and_video_status_providers<F, S, V, R, I, T, X>(
        config: RemoteControlConfig,
        callback: F,
        snapshot_provider: S,
        video_status_providers: (V, R, I, T, X),
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
        let (
            video_runtime_status_provider,
            video_output_render_plans_provider,
            external_video_io_plans_provider,
            external_video_transport_status_provider,
            external_video_transport_sync_provider,
        ) = video_status_providers;
        Self::start_with_snapshot_and_video_status_providers_and_dj_link(
            config,
            callback,
            snapshot_provider,
            (
                video_runtime_status_provider,
                video_output_render_plans_provider,
                external_video_io_plans_provider,
                external_video_transport_status_provider,
                external_video_transport_sync_provider,
            ),
            None,
        )
    }

    pub fn start_with_snapshot_and_video_status_providers_and_dj_link<F, S, V, R, I, T, X>(
        config: RemoteControlConfig,
        callback: F,
        snapshot_provider: S,
        video_status_providers: (V, R, I, T, X),
        dj_link_handler: Option<DjLinkDispatchHandler>,
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
        Self::start_with_snapshot_and_video_status_providers_and_dj_link_with_process_fence(
            config,
            callback,
            move || Some(snapshot_provider()),
            video_status_providers,
            dj_link_handler,
            None,
        )
    }

    /// Start a listener with a process-owned physical-event fence.  The
    /// listener registry remains instance-local; only the injected fence and
    /// its capacity latch survive stop/start within one process.
    pub fn start_with_snapshot_and_video_status_providers_and_dj_link_with_process_fence<
        F,
        S,
        V,
        R,
        I,
        T,
        X,
    >(
        config: RemoteControlConfig,
        callback: F,
        snapshot_provider: S,
        video_status_providers: (V, R, I, T, X),
        dj_link_handler: Option<DjLinkDispatchHandler>,
        process_fence: Option<DjLinkProcessFenceHandle>,
    ) -> Result<Self, RemoteWsError>
    where
        F: Fn(RemoteInputEvent) + Send + Sync + 'static,
        S: Fn() -> Option<EngineSnapshot> + Send + Sync + 'static,
        V: Fn() -> VideoRuntimeStatus + Send + Sync + 'static,
        R: Fn() -> Value + Send + Sync + 'static,
        I: Fn() -> Value + Send + Sync + 'static,
        T: Fn() -> Value + Send + Sync + 'static,
        X: Fn() -> Value + Send + Sync + 'static,
    {
        let (
            video_runtime_status_provider,
            video_output_render_plans_provider,
            external_video_io_plans_provider,
            external_video_transport_status_provider,
            external_video_transport_sync_provider,
        ) = video_status_providers;
        if config.bind_ip.trim().is_empty() {
            return Err(RemoteWsError::MissingBindAddress);
        }
        if config.port == 0 && !cfg!(test) {
            return Err(RemoteWsError::InvalidPort);
        }
        // A listener with neither transport enabled would bind a port that
        // serves nothing, so this fails closed before any socket is opened.
        if !config.web_remote_enabled && !config.dj_link_enabled {
            return Err(RemoteWsError::NoEnabledTransport);
        }
        // The generic pairing PIN guards only the Web Remote transport; a
        // DJ-only listener legitimately runs without one.
        if config.web_remote_enabled
            && (config.pairing_pin.len() != 6
                || !config.pairing_pin.bytes().all(|byte| byte.is_ascii_digit()))
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
        if config.dj_link_enabled {
            let bind_ip = config
                .dj_link_bind_ip
                .as_deref()
                .ok_or(RemoteWsError::InvalidDjLinkBindAddress)?
                .trim();
            let parsed = bind_ip
                .parse::<std::net::IpAddr>()
                .map_err(|_| RemoteWsError::InvalidDjLinkBindAddress)?;
            if parsed.is_unspecified() || bind_ip != config.bind_ip.trim() {
                return Err(RemoteWsError::InvalidDjLinkBindAddress);
            }
            let Some(token) = config.dj_link_token.as_deref() else {
                return Err(RemoteWsError::InvalidDjLinkToken);
            };
            if token.len() < protocol::DJ_LINK_MIN_TOKEN_BYTES || token.len() > 256 {
                return Err(RemoteWsError::InvalidDjLinkToken);
            }
        }

        let bind = format!("{}:{}", config.bind_ip, config.port);
        let listener = TcpListener::bind(&bind).map_err(|source| RemoteWsError::Bind {
            bind: bind.clone(),
            source,
        })?;
        let local_addr = listener
            .local_addr()
            .map_err(|source| RemoteWsError::Bind {
                bind: bind.clone(),
                source,
            })?;
        // Unit tests may request port zero so the same OS-owned listener that
        // selected the ephemeral port is carried into the server. Production
        // still rejects zero above. Publish the selected port into the config
        // before worker authority checks use it.
        let mut config = config;
        if config.port == 0 {
            config.port = local_addr.port();
        }
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
        let web_remote_enabled = config.web_remote_enabled;
        let dj_link_enabled = config.dj_link_enabled;
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let next_client_id = Arc::new(AtomicU64::new(1));
        let rejected_connections = Arc::new(AtomicU64::new(0));
        let thread_clients = Arc::clone(&clients);
        let thread_next_client_id = Arc::clone(&next_client_id);
        let thread_rejected_connections = Arc::clone(&rejected_connections);
        let dj_link_connections = Arc::new(AtomicU64::new(0));
        let thread_dj_link_connections = Arc::clone(&dj_link_connections);
        let process_fence = process_fence.unwrap_or_else(new_dj_link_process_fence);
        let dj_link_registry = Arc::new(Mutex::new(DjLinkRegistry::with_process_fence(
            process_fence,
        )));
        let thread_dj_link_registry = Arc::clone(&dj_link_registry);
        let thread_dj_link_handler = dj_link_handler.clone();
        let client_workers = Arc::new(Mutex::new(Vec::<JoinHandle<()>>::new()));
        let shutdown_sockets = Arc::new(Mutex::new(HashMap::<u64, TcpStream>::new()));
        let next_worker_id = Arc::new(AtomicU64::new(1));
        let thread_client_workers = Arc::clone(&client_workers);
        let thread_shutdown_sockets = Arc::clone(&shutdown_sockets);
        let thread_next_worker_id = Arc::clone(&next_worker_id);
        let thread = thread::Builder::new()
            .name("syndocal-remote-ws".to_string())
            .spawn(move || {
                let mut next_dj_observation = Instant::now();
                let mut previous_dj_observation: Option<DjLinkEngineObservation> = None;
                let mut last_dj_semantic_key = None;
                while !thread_stop.load(Ordering::Relaxed) {
                    // Completed workers remain joinable until their result is
                    // collected.  Reap them here so a long-lived listener
                    // does not accumulate one JoinHandle per connection.
                    if let Ok(mut workers) = thread_client_workers.lock() {
                        let mut active_workers = Vec::with_capacity(workers.len());
                        for worker in workers.drain(..) {
                            if worker.is_finished() {
                                let _ = worker.join();
                            } else {
                                active_workers.push(worker);
                            }
                        }
                        *workers = active_workers;
                    }
                    let now = Instant::now();
                    if config.dj_link_enabled && now >= next_dj_observation {
                        next_dj_observation = now + DJ_LINK_OBSERVATION_INTERVAL;
                        // Snapshot unavailability is not an observation. In
                        // particular, retain both comparison images so a
                        // committed B cannot be replaced by a synthetic or
                        // cached A transition while the Engine writer owns
                        // its publication lock.
                        if let Some(snapshot) = (snapshot_provider.as_ref())() {
                            let current = DjLinkEngineObservation::from_snapshot(&snapshot);
                            let semantic_key = current.semantic_key(previous_dj_observation);
                            if previous_dj_observation.is_some()
                                && last_dj_semantic_key != Some(semantic_key)
                            {
                                let state = DjLinkTimelineState {
                                    message_type: "DJ_TIMELINE_STATE".to_string(),
                                    event_id: "pending".to_string(),
                                    sequence: 1,
                                    state: current.state(previous_dj_observation),
                                    loop_active: current.loop_active,
                                    timeline_id: current.timeline_id.to_string(),
                                    position_bars: current.position_bars(),
                                    play_session_id: None,
                                    pedal_owner: None,
                                    release_event_id: None,
                                };
                                if let Ok(mut registry) = thread_dj_link_registry.lock() {
                                    let _ = registry.queue_outbound_state(state);
                                }
                            }
                            last_dj_semantic_key = Some(semantic_key);
                            previous_dj_observation = Some(current);
                        }
                    }
                    match listener.accept() {
                        Ok((stream, _)) => {
                            if thread_stop.load(Ordering::Acquire) {
                                let _ = stream.shutdown(Shutdown::Both);
                                break;
                            }
                            let worker_id = thread_next_worker_id.fetch_add(1, Ordering::AcqRel);
                            let shutdown_stream = match stream.try_clone() {
                                Ok(shutdown_stream) => shutdown_stream,
                                Err(_) => {
                                    let _ = stream.shutdown(Shutdown::Both);
                                    continue;
                                }
                            };
                            if let Ok(mut sockets) = thread_shutdown_sockets.lock() {
                                sockets.insert(worker_id, shutdown_stream);
                            } else {
                                let _ = stream.shutdown(Shutdown::Both);
                                continue;
                            }
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
                            let client_dj_link_registry = Arc::clone(&thread_dj_link_registry);
                            let client_dj_link_handler = thread_dj_link_handler.clone();
                            let client_dj_link_connections =
                                Arc::clone(&thread_dj_link_connections);
                            let worker_sockets = Arc::clone(&thread_shutdown_sockets);
                            let worker = thread::Builder::new()
                                .name("syndocal-remote-ws-client".to_string())
                                .spawn(move || {
                                    // The transport boundary is panic-safe: a
                                    // callback panic must still remove the
                                    // socket from the shutdown registry and
                                    // release the DJ slot guard.
                                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                                        || {
                                            handle_connection(
                                                stream,
                                                client_stop,
                                                client_callback.as_ref(),
                                                client_snapshot_provider.as_ref(),
                                                RemoteWebProviders {
                                                    video_runtime_status_provider:
                                                        client_video_runtime_status_provider,
                                                    video_output_render_plans_provider:
                                                        client_video_output_render_plans_provider,
                                                    external_video_io_plans_provider:
                                                        client_external_video_io_plans_provider,
                                                    external_video_transport_status_provider:
                                                        client_external_video_transport_status_provider,
                                                    external_video_transport_sync_provider:
                                                        client_external_video_transport_sync_provider,
                                                },
                                                RemoteConnectionContext {
                                                    config: client_config.as_ref(),
                                                    clients: client_registry,
                                                    next_client_id: client_next_id,
                                                    rejected_connections: client_rejected_connections,
                                                    dj_link_handler: client_dj_link_handler,
                                                    dj_link_registry: client_dj_link_registry,
                                                    dj_link_connections: client_dj_link_connections,
                                                },
                                            );
                                        },
                                    ));
                                    if let Ok(mut sockets) = worker_sockets.lock() {
                                        sockets.remove(&worker_id);
                                    }
                                });
                            let Ok(worker) = worker else {
                                if let Ok(mut sockets) = thread_shutdown_sockets.lock() {
                                    if let Some(socket) = sockets.remove(&worker_id) {
                                        let _ = socket.shutdown(Shutdown::Both);
                                    }
                                }
                                continue;
                            };
                            if let Ok(mut workers) = thread_client_workers.lock() {
                                workers.push(worker);
                            } else {
                                let _ = worker.join();
                                if let Ok(mut sockets) = thread_shutdown_sockets.lock() {
                                    if let Some(socket) = sockets.remove(&worker_id) {
                                        let _ = socket.shutdown(Shutdown::Both);
                                    }
                                }
                            }
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
            dj_link_registry,
            web_remote_enabled,
            dj_link_enabled,
            client_workers,
            shutdown_sockets,
            dj_link_connections,
            #[cfg(test)]
            local_addr,
        })
    }

    #[cfg(test)]
    fn local_addr(&self) -> std::net::SocketAddr {
        self.local_addr
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
            web_remote_enabled: self.web_remote_enabled,
            dj_link_enabled: self.dj_link_enabled,
            dj_link: Some(self.dj_link_status()),
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

    pub fn dj_link_status(&self) -> DjLinkRuntimeStatus {
        self.dj_link_registry
            .lock()
            .map(|registry| {
                let mut status = registry.status.clone();
                if let Some(session) = registry.sessions.values().next() {
                    status.connected = true;
                    status.age_ms = Some(
                        session
                            .last_liveness
                            .elapsed()
                            .as_millis()
                            .min(u128::from(u64::MAX)) as u64,
                    );
                }
                status
            })
            .unwrap_or_else(|_| DjLinkRuntimeStatus::default())
    }

    /// Process-local lifecycle diagnostics used by the production-loopback
    /// acceptance proof. This exposes no wire state and does not alter
    /// admission; it only observes identities whose handler has not yet
    /// returned and terminalized.
    #[doc(hidden)]
    pub fn dj_link_inflight_dispatch_count(&self) -> usize {
        self.dj_link_registry
            .lock()
            .map(|registry| registry.inflight.len())
            .unwrap_or(usize::MAX)
    }

    fn stop_and_join(&mut self) {
        self.stop.store(true, Ordering::Release);
        // Retire admitted physical identities before any socket or worker
        // join. The production start route performs listener replacement
        // serially: it joins this instance before binding the next one, and
        // that replacement shares the process fence. This retirement is not
        // a claim that arbitrary session/handler replacements may run as
        // parallel authorities.
        match self.dj_link_registry.lock() {
            Ok(mut registry) => registry.retire(),
            Err(poisoned) => poisoned.into_inner().retire(),
        }
        // Closing the accepted socket clones is what unblocks a worker that
        // is waiting for a peer that stopped reading.  The registry is never
        // held while doing network shutdown.
        if let Ok(sockets) = self.shutdown_sockets.lock() {
            for socket in sockets.values() {
                let _ = socket.shutdown(Shutdown::Both);
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let workers = self
            .client_workers
            .lock()
            .map(|mut workers| workers.drain(..).collect::<Vec<_>>())
            .unwrap_or_default();
        for worker in workers {
            let _ = worker.join();
        }
        if let Ok(mut sockets) = self.shutdown_sockets.lock() {
            sockets.clear();
        }
        if let Ok(mut clients) = self.clients.lock() {
            clients.clear();
        }
        self.dj_link_connections.store(0, Ordering::Release);
    }
}

impl Drop for RemoteWsServer {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

// Accepted `type` values are generated together with the parser selector and
// the AI0 wire inventory.  The request parser below matches this enum rather
// than raw strings, so a newly accepted wire operation cannot bypass the
// inventory or parser exhaustiveness checks.
macro_rules! define_remote_wire_operations {
    ($($variant:ident => $wire_type:literal,)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub(crate) enum RemoteWireOperation {
            $($variant,)*
        }

        impl RemoteWireOperation {
            pub(crate) const CONTROL_PLANE_OPERATIONS: &'static [(&'static str, &'static str)] = &[
                $((stringify!($variant), $wire_type),)*
            ];

            fn from_type(value: &str) -> Option<Self> {
                match value {
                    $($wire_type => Some(Self::$variant),)*
                    _ => None,
                }
            }

            #[cfg(test)]
            fn wire_type(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire_type,)*
                }
            }
        }
    };
}

define_remote_wire_operations! {
    GetSnapshot => "getSnapshot",
    GetVideoRuntimeStatus => "getVideoRuntimeStatus",
    GetVideoOutputRenderPlans => "getVideoOutputRenderPlans",
    GetExternalVideoIoPlans => "getExternalVideoIoPlans",
    GetExternalVideoTransportStatus => "getExternalVideoTransportStatus",
    SyncExternalVideoTransports => "syncExternalVideoTransports",
    SetAttribute => "setAttribute",
    SetOperatorSelection => "setOperatorSelection",
    SetOperatorFeatureFader => "setOperatorFeatureFader",
    SetGroupAttribute => "setGroupAttribute",
    SetFixtureHighlight => "setFixtureHighlight",
    SetFixtureSolo => "setFixtureSolo",
    SetFixturePark => "setFixturePark",
    SetGroupHighlight => "setGroupHighlight",
    SetGroupSolo => "setGroupSolo",
    SetGroupPark => "setGroupPark",
    ClearFixtureFlags => "clearFixtureFlags",
    Blackout => "blackout",
    AllBlackout => "allBlackout",
    LightingMaster => "lightingMaster",
    SetGroupSubmaster => "setGroupSubmaster",
    SetBpm => "setBpm",
    TapBpm => "tapBpm",
    SyncAbletonLinkClock => "syncAbletonLinkClock",
    SyncExternalClock => "syncExternalClock",
    ResetTelemetry => "resetTelemetry",
    TriggerCue => "triggerCue",
    TriggerNextCue => "triggerNextCue",
    TriggerPreviousCue => "triggerPreviousCue",
    SetCueFadePaused => "setCueFadePaused",
    SetTimelinePlaying => "setTimelinePlaying",
    SeekTimeline => "seekTimeline",
    SeekTimelineBeat => "seekTimelineBeat",
    SyncTimelineTimecode => "syncTimelineTimecode",
    SetEffectEnabled => "setEffectEnabled",
    SetNodeGraphEnabled => "setNodeGraphEnabled",
    MoveEffect => "moveEffect",
    RemoveEffect => "removeEffect",
    SetVideoParam => "setVideoParam",
    SetVideoLayerEnabled => "setVideoLayerEnabled",
    SetVideoLayerSolo => "setVideoLayerSolo",
    SetVideoPlaying => "setVideoPlaying",
    SeekVideoLayer => "seekVideoLayer",
    SetVideoLoop => "setVideoLoop",
    FadeVideoLayerOpacity => "fadeVideoLayerOpacity",
    AddVideoCuePoint => "addVideoCuePoint",
    RemoveVideoCuePoint => "removeVideoCuePoint",
    JumpVideoCuePoint => "jumpVideoCuePoint",
    JumpVideoCuePointRelative => "jumpVideoCuePointRelative",
    SetVideoOutputEnabled => "setVideoOutputEnabled",
    SetVideoOutputOpacity => "setVideoOutputOpacity",
    FadeVideoOutputOpacity => "fadeVideoOutputOpacity",
    SetVideoOutputMapping => "setVideoOutputMapping",
    SetVideoOutputMappingField => "setVideoOutputMappingField",
    ApplyVideoOutputMappingPreset => "applyVideoOutputMappingPreset",
    SetVideoOutputBlackout => "setVideoOutputBlackout",
    VideoMaster => "videoMaster",
    VideoBlackout => "videoBlackout",
}

pub fn event_from_text(text: &str) -> Result<RemoteInputEvent, RemoteParseError> {
    match request_from_text(text)? {
        RemoteClientRequest::Event(event) => Ok(event),
        RemoteClientRequest::GetSnapshot => Err(RemoteParseError::UnknownType),
        RemoteClientRequest::GetVideoRuntimeStatus => Err(RemoteParseError::UnknownType),
        RemoteClientRequest::GetVideoOutputRenderPlans => Err(RemoteParseError::UnknownType),
        RemoteClientRequest::GetExternalVideoIoPlans => Err(RemoteParseError::UnknownType),
        RemoteClientRequest::GetExternalVideoTransportStatus => Err(RemoteParseError::UnknownType),
        RemoteClientRequest::SyncExternalVideoTransports => Err(RemoteParseError::UnknownType),
    }
}

pub fn request_from_text(text: &str) -> Result<RemoteClientRequest, RemoteParseError> {
    let value: Value = serde_json::from_str(text).map_err(|_| RemoteParseError::InvalidJson)?;
    let command_type = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or(RemoteParseError::MissingType)?;
    let operation =
        RemoteWireOperation::from_type(command_type).ok_or(RemoteParseError::UnknownType)?;
    match operation {
        RemoteWireOperation::GetSnapshot => Ok(RemoteClientRequest::GetSnapshot),
        RemoteWireOperation::GetVideoRuntimeStatus => {
            Ok(RemoteClientRequest::GetVideoRuntimeStatus)
        }
        RemoteWireOperation::GetVideoOutputRenderPlans => {
            Ok(RemoteClientRequest::GetVideoOutputRenderPlans)
        }
        RemoteWireOperation::GetExternalVideoIoPlans => {
            Ok(RemoteClientRequest::GetExternalVideoIoPlans)
        }
        RemoteWireOperation::GetExternalVideoTransportStatus => {
            Ok(RemoteClientRequest::GetExternalVideoTransportStatus)
        }
        RemoteWireOperation::SyncExternalVideoTransports => {
            Ok(RemoteClientRequest::SyncExternalVideoTransports)
        }
        RemoteWireOperation::SetAttribute => {
            Ok(RemoteClientRequest::Event(RemoteInputEvent::SetAttribute {
                fixture_id: read_u64(&value, "fixture_id")?,
                attribute: read_string(&value, "attribute")?,
                value: read_u16_value(&value, "value")?,
            }))
        }
        RemoteWireOperation::SetOperatorSelection => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetOperatorSelection(OperatorSelectionContext {
                fixture_ids: read_u64_array(&value, "fixture_ids")?,
                attributes: read_string_array(&value, "attributes")?,
            }),
        )),
        RemoteWireOperation::SetOperatorFeatureFader => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetOperatorFeatureFader {
                target_index: read_u64(&value, "target_index")? as usize,
                value: read_u16_value(&value, "value")?,
            },
        )),
        RemoteWireOperation::SetGroupAttribute => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetGroupAttribute {
                group_id: read_string(&value, "group_id")?,
                attribute: read_string(&value, "attribute")?,
                value: read_u16_value(&value, "value")?,
            },
        )),
        RemoteWireOperation::SetFixtureHighlight => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetFixtureHighlight {
                fixture_id: read_u64(&value, "fixture_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        RemoteWireOperation::SetFixtureSolo => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetFixtureSolo {
                fixture_id: read_u64(&value, "fixture_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        RemoteWireOperation::SetFixturePark => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetFixturePark {
                fixture_id: read_u64(&value, "fixture_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        RemoteWireOperation::SetGroupHighlight => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetGroupHighlight {
                group_id: read_string(&value, "group_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        RemoteWireOperation::SetGroupSolo => {
            Ok(RemoteClientRequest::Event(RemoteInputEvent::SetGroupSolo {
                group_id: read_string(&value, "group_id")?,
                enabled: read_bool(&value, "enabled")?,
            }))
        }
        RemoteWireOperation::SetGroupPark => {
            Ok(RemoteClientRequest::Event(RemoteInputEvent::SetGroupPark {
                group_id: read_string(&value, "group_id")?,
                enabled: read_bool(&value, "enabled")?,
            }))
        }
        RemoteWireOperation::ClearFixtureFlags => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::ClearFixtureFlags {
                kind: read_string(&value, "kind")?,
            },
        )),
        RemoteWireOperation::Blackout => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::Blackout(read_bool(&value, "enabled")?),
        )),
        RemoteWireOperation::AllBlackout => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::AllBlackout(read_bool(&value, "enabled")?),
        )),
        RemoteWireOperation::LightingMaster => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::LightingMaster(read_f32(&value, "master")?),
        )),
        RemoteWireOperation::SetGroupSubmaster => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetGroupSubmaster {
                group_id: read_string(&value, "group_id")?,
                level: read_f32(&value, "level")?,
            },
        )),
        RemoteWireOperation::SetBpm => Ok(RemoteClientRequest::Event(RemoteInputEvent::SetBpm(
            read_f32(&value, "bpm")?,
        ))),
        RemoteWireOperation::TapBpm => Ok(RemoteClientRequest::Event(RemoteInputEvent::TapBpm)),
        RemoteWireOperation::SyncAbletonLinkClock => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SyncExternalClock {
                bpm: read_f32(&value, "bpm")?,
                beat_phase: read_f32(&value, "beat_phase")?,
                source: ClockSource::AbletonLink,
            },
        )),
        RemoteWireOperation::SyncExternalClock => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SyncExternalClock {
                bpm: read_f32(&value, "bpm")?,
                beat_phase: read_f32(&value, "beat_phase")?,
                source: read_clock_source(&value, "source")?,
            },
        )),
        RemoteWireOperation::ResetTelemetry => {
            Ok(RemoteClientRequest::Event(RemoteInputEvent::ResetTelemetry))
        }
        RemoteWireOperation::TriggerCue => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::TriggerCue(read_u64(&value, "cue_id")?),
        )),
        RemoteWireOperation::TriggerNextCue => {
            Ok(RemoteClientRequest::Event(RemoteInputEvent::TriggerNextCue))
        }
        RemoteWireOperation::TriggerPreviousCue => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::TriggerPreviousCue,
        )),
        RemoteWireOperation::SetCueFadePaused => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetCueFadePaused(read_bool(&value, "paused")?),
        )),
        RemoteWireOperation::SetTimelinePlaying => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetTimelinePlaying(read_bool(&value, "playing")?),
        )),
        RemoteWireOperation::SeekTimeline => {
            Ok(RemoteClientRequest::Event(RemoteInputEvent::SeekTimeline {
                position_ms: read_u64(&value, "position_ms")?,
            }))
        }
        RemoteWireOperation::SeekTimelineBeat => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SeekTimelineBeat {
                direction: read_i32(&value, "direction")?,
            },
        )),
        RemoteWireOperation::SyncTimelineTimecode => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SyncTimelineTimecode {
                position_ms: read_timecode_position_ms(&value, "position_ms")?,
                source: read_optional_clock_source(&value, "source")?.unwrap_or(ClockSource::Ltc),
            },
        )),
        RemoteWireOperation::SetEffectEnabled => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetEffectEnabled {
                effect_id: read_u64(&value, "effect_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        RemoteWireOperation::SetNodeGraphEnabled => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetNodeGraphEnabled {
                graph_id: read_u64(&value, "graph_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        RemoteWireOperation::MoveEffect => {
            let delta = read_i32(&value, "delta")?;
            if delta != -1 && delta != 1 {
                return Err(RemoteParseError::InvalidField("delta"));
            }
            Ok(RemoteClientRequest::Event(RemoteInputEvent::MoveEffect {
                effect_id: read_u64(&value, "effect_id")?,
                delta,
            }))
        }
        RemoteWireOperation::RemoveEffect => {
            Ok(RemoteClientRequest::Event(RemoteInputEvent::RemoveEffect {
                effect_id: read_u64(&value, "effect_id")?,
            }))
        }
        RemoteWireOperation::SetVideoParam => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoParam {
                layer_id: read_u64(&value, "layer_id")?,
                param: video_param_from_str(&read_string(&value, "param")?)
                    .ok_or(RemoteParseError::InvalidField("param"))?,
                value: read_f32(&value, "value")?,
            },
        )),
        RemoteWireOperation::SetVideoLayerEnabled => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoLayerEnabled {
                layer_id: read_u64(&value, "layer_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        RemoteWireOperation::SetVideoLayerSolo => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoLayerSolo {
                layer_id: read_u64(&value, "layer_id")?,
                solo: read_bool(&value, "solo")?,
            },
        )),
        RemoteWireOperation::SetVideoPlaying => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoPlaying {
                layer_id: read_u64(&value, "layer_id")?,
                playing: read_bool(&value, "playing")?,
            },
        )),
        RemoteWireOperation::SeekVideoLayer => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SeekVideoLayer {
                layer_id: read_u64(&value, "layer_id")?,
                position_ms: read_u64(&value, "position_ms")?,
            },
        )),
        RemoteWireOperation::SetVideoLoop => {
            Ok(RemoteClientRequest::Event(RemoteInputEvent::SetVideoLoop {
                layer_id: read_u64(&value, "layer_id")?,
                enabled: read_bool(&value, "enabled")?,
                loop_start_ms: read_optional_u64(&value, "loop_start_ms")?,
                loop_end_ms: read_optional_u64(&value, "loop_end_ms")?,
            }))
        }
        RemoteWireOperation::FadeVideoLayerOpacity => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::FadeVideoLayerOpacity {
                layer_id: read_u64(&value, "layer_id")?,
                opacity: read_f32(&value, "opacity")?,
                duration_ms: read_u64(&value, "duration_ms")?,
            },
        )),
        RemoteWireOperation::AddVideoCuePoint => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::AddVideoCuePoint {
                layer_id: read_u64(&value, "layer_id")?,
                position_ms: read_optional_u64(&value, "position_ms")?,
            },
        )),
        RemoteWireOperation::RemoveVideoCuePoint => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::RemoveVideoCuePoint {
                layer_id: read_u64(&value, "layer_id")?,
                position_ms: read_u64(&value, "position_ms")?,
            },
        )),
        RemoteWireOperation::JumpVideoCuePoint => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::JumpVideoCuePoint {
                layer_id: read_u64(&value, "layer_id")?,
                cue_point_index: read_u64(&value, "cue_point_index")? as usize,
            },
        )),
        RemoteWireOperation::JumpVideoCuePointRelative => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::JumpVideoCuePointRelative {
                layer_id: read_u64(&value, "layer_id")?,
                direction: read_i32(&value, "direction")?,
            },
        )),
        RemoteWireOperation::SetVideoOutputEnabled => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoOutputEnabled {
                output_id: read_u64(&value, "output_id")?,
                enabled: read_bool(&value, "enabled")?,
            },
        )),
        RemoteWireOperation::SetVideoOutputOpacity => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoOutputOpacity {
                output_id: read_u64(&value, "output_id")?,
                opacity: read_f32(&value, "opacity")?,
            },
        )),
        RemoteWireOperation::FadeVideoOutputOpacity => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::FadeVideoOutputOpacity {
                output_id: read_u64(&value, "output_id")?,
                opacity: read_f32(&value, "opacity")?,
                duration_ms: read_u64(&value, "duration_ms")?,
            },
        )),
        RemoteWireOperation::SetVideoOutputMapping => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoOutputMapping {
                output_id: read_u64(&value, "output_id")?,
                mapping: Box::new(read_video_output_mapping(&value, "mapping")?),
            },
        )),
        RemoteWireOperation::SetVideoOutputMappingField => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoOutputMappingField {
                output_id: read_u64(&value, "output_id")?,
                field: read_video_output_mapping_field(&value, "field")?,
                value: read_f32(&value, "value")?,
            },
        )),
        RemoteWireOperation::ApplyVideoOutputMappingPreset => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::ApplyVideoOutputMappingPreset {
                output_id: read_u64(&value, "output_id")?,
                label: read_string(&value, "label")?,
            },
        )),
        RemoteWireOperation::SetVideoOutputBlackout => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::SetVideoOutputBlackout {
                output_id: read_u64(&value, "output_id")?,
                blackout: read_bool(&value, "blackout")?,
            },
        )),
        RemoteWireOperation::VideoMaster => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::VideoMasterOpacity(read_f32(&value, "opacity")?),
        )),
        RemoteWireOperation::VideoBlackout => Ok(RemoteClientRequest::Event(
            RemoteInputEvent::VideoBlackout(read_bool(&value, "enabled")?),
        )),
    }
}

struct RemoteWebProviders {
    video_runtime_status_provider: Arc<dyn Fn() -> VideoRuntimeStatus + Send + Sync>,
    video_output_render_plans_provider: Arc<dyn Fn() -> Value + Send + Sync>,
    external_video_io_plans_provider: Arc<dyn Fn() -> Value + Send + Sync>,
    external_video_transport_status_provider: Arc<dyn Fn() -> Value + Send + Sync>,
    external_video_transport_sync_provider: Arc<dyn Fn() -> Value + Send + Sync>,
}

struct RemoteConnectionContext<'a> {
    config: &'a RemoteControlConfig,
    clients: Arc<Mutex<HashMap<u64, RemoteClientState>>>,
    next_client_id: Arc<AtomicU64>,
    rejected_connections: Arc<AtomicU64>,
    dj_link_handler: Option<DjLinkDispatchHandler>,
    dj_link_registry: Arc<Mutex<DjLinkRegistry>>,
    dj_link_connections: Arc<AtomicU64>,
}

struct RemoteWebsocketContext<'a> {
    config: &'a RemoteControlConfig,
    clients: Arc<Mutex<HashMap<u64, RemoteClientState>>>,
}

struct RemoteWebsocketClient {
    client_id: u64,
    disconnect: Arc<AtomicBool>,
}

fn handle_connection<F, S>(
    stream: TcpStream,
    stop: Arc<AtomicBool>,
    callback: &F,
    snapshot_provider: &S,
    providers: RemoteWebProviders,
    context: RemoteConnectionContext<'_>,
) where
    F: Fn(RemoteInputEvent) + ?Sized,
    S: Fn() -> Option<EngineSnapshot> + ?Sized,
{
    let RemoteConnectionContext {
        config,
        clients,
        next_client_id,
        rejected_connections,
        dj_link_handler,
        dj_link_registry,
        dj_link_connections,
    } = context;
    let _ = stream.set_read_timeout(Some(REMOTE_SOCKET_READ_TIMEOUT));
    let _ = stream.set_write_timeout(Some(REMOTE_SOCKET_WRITE_TIMEOUT));
    let mut peek_buffer = [0u8; HTTP_PEEK_SIZE];
    match stream.peek(&mut peek_buffer) {
        Ok(size) if is_websocket_request(&peek_buffer[..size]) => {
            let request = String::from_utf8_lossy(&peek_buffer[..size]);
            // DJ Link owns only the exact /dj-link target.  The generic Web
            // Remote /ws endpoint keeps its pairing-token contract even while
            // DJ Link is enabled: a bare /ws must never be reinterpreted as
            // the dedicated, token-authenticated DJ transport.
            if request_path(&request) == Some("/dj-link") {
                if !request_host_is_allowed(&request, &stream, config) {
                    reject_http_client(stream, "403 Forbidden", "Invalid Host or Origin");
                    return;
                }
                let peer_addr = stream
                    .peer_addr()
                    .map(|address| address.to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                let active = dj_link_connections.fetch_add(1, Ordering::AcqRel);
                if active >= u64::from(config.max_connections) {
                    dj_link_connections.fetch_sub(1, Ordering::AcqRel);
                    reject_http_client(
                        stream,
                        "503 Service Unavailable",
                        "DJ Link connection limit reached",
                    );
                    return;
                }
                let _dj_link_slot = DjLinkConnectionSlotGuard {
                    slots: Arc::clone(&dj_link_connections),
                };
                handle_dj_link_client(
                    stream,
                    stop,
                    config,
                    peer_addr,
                    dj_link_handler,
                    dj_link_registry,
                );
                return;
            }
            // DJ-only mode: every non-DJ WebSocket route is closed before any
            // generic Host/Origin or pairing-PIN evaluation. The response is a
            // plain 404 because the WebSocket upgrade never happens.
            if !config.web_remote_enabled {
                reject_http_client(stream, "404 Not Found", "Not found");
                return;
            }
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
            let _client_registration = RemoteClientRegistrationGuard {
                clients: Arc::clone(&clients),
                client_id,
            };
            handle_websocket_client(
                stream,
                stop,
                callback,
                snapshot_provider,
                &providers,
                RemoteWebsocketContext {
                    config,
                    clients: Arc::clone(&clients),
                },
                RemoteWebsocketClient {
                    client_id,
                    disconnect,
                },
            );
        }
        Ok(size) => {
            let request = String::from_utf8_lossy(&peek_buffer[..size]);
            let path = request_path(&request).unwrap_or("/");
            if !config.web_remote_enabled {
                // DJ-only mode serves no HTTP asset or health route, and a
                // plain non-upgrade GET /dj-link is equally absent.
                reject_http_client(stream, "404 Not Found", "Not found");
            } else if !request_host_is_allowed(&request, &stream, config) {
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

fn dj_link_ack(
    envelope: &DjLinkEnvelope,
    outcome: DjLinkAckOutcome,
    code: Option<String>,
    state_generation: u64,
) -> DjLinkAck {
    DjLinkAck {
        v: protocol::DJ_LINK_PROTOCOL_VERSION,
        message_type: "ACK".to_string(),
        event_id: envelope.event_id.clone(),
        sequence: envelope.sequence,
        outcome,
        code,
        state_generation,
    }
}

/// A DJ Link ACK failed to reach the wire. Neither case has a recoverable
/// wire representation: the caller must terminate the affected
/// connection/session path instead of emitting a fabricated frame or
/// continuing protocol processing.
#[derive(Debug, Error)]
enum DjLinkAckPublishError {
    #[error("DJ Link ACK serialization failed: {0}")]
    Serialization(serde_json::Error),
    #[error("DJ Link ACK socket write failed: {0}")]
    Transport(tungstenite::Error),
}

#[cfg(test)]
static DJ_LINK_ACK_SERIALIZATION_FAILURE_HOOK: OnceLock<Mutex<Option<DjLinkPreDispatchHook>>> =
    OnceLock::new();

#[cfg(test)]
fn set_dj_link_ack_serialization_failure_hook(hook: Option<DjLinkPreDispatchHook>) {
    let slot = DJ_LINK_ACK_SERIALIZATION_FAILURE_HOOK.get_or_init(|| Mutex::new(None));
    if let Ok(mut current) = slot.lock() {
        *current = hook;
    }
}

#[cfg(test)]
struct DjLinkAckSerializationFailureGuard;

#[cfg(test)]
impl Drop for DjLinkAckSerializationFailureGuard {
    fn drop(&mut self) {
        set_dj_link_ack_serialization_failure_hook(None);
    }
}

#[cfg(test)]
fn install_dj_link_ack_serialization_failure_hook(
    hook: DjLinkPreDispatchHook,
) -> DjLinkAckSerializationFailureGuard {
    set_dj_link_ack_serialization_failure_hook(Some(hook));
    DjLinkAckSerializationFailureGuard
}

#[cfg(test)]
fn run_dj_link_ack_serialization_failure_injection() -> bool {
    let hook = DJ_LINK_ACK_SERIALIZATION_FAILURE_HOOK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|current| current.clone());
    hook.is_some()
}

#[cfg(not(test))]
#[inline]
fn run_dj_link_ack_serialization_failure_injection() -> bool {
    false
}

#[cfg(test)]
static DJ_LINK_ACK_TRANSPORT_FAILURE_HOOK: OnceLock<Mutex<Option<DjLinkPreDispatchHook>>> =
    OnceLock::new();

#[cfg(test)]
fn set_dj_link_ack_transport_failure_hook(hook: Option<DjLinkPreDispatchHook>) {
    let slot = DJ_LINK_ACK_TRANSPORT_FAILURE_HOOK.get_or_init(|| Mutex::new(None));
    if let Ok(mut current) = slot.lock() {
        *current = hook;
    }
}

#[cfg(test)]
struct DjLinkAckTransportFailureGuard;

#[cfg(test)]
impl Drop for DjLinkAckTransportFailureGuard {
    fn drop(&mut self) {
        set_dj_link_ack_transport_failure_hook(None);
    }
}

#[cfg(test)]
fn install_dj_link_ack_transport_failure_hook(
    hook: DjLinkPreDispatchHook,
) -> DjLinkAckTransportFailureGuard {
    set_dj_link_ack_transport_failure_hook(Some(hook));
    DjLinkAckTransportFailureGuard
}

#[cfg(test)]
fn run_dj_link_ack_transport_failure_injection() -> bool {
    let hook = DJ_LINK_ACK_TRANSPORT_FAILURE_HOOK
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|current| current.clone());
    hook.is_some()
}

#[cfg(not(test))]
#[inline]
fn run_dj_link_ack_transport_failure_injection() -> bool {
    false
}

/// Serialize one v3 ACK to its exact wire text. There is deliberately no
/// fallback representation: a render failure surfaces as an error so the
/// caller terminates instead of emitting a fabricated `{}` frame.
fn dj_link_ack_wire(ack: &DjLinkAck) -> Result<String, DjLinkAckPublishError> {
    if run_dj_link_ack_serialization_failure_injection() {
        return Err(DjLinkAckPublishError::Serialization(serde_json::Error::io(
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "injected DJ Link ACK serialization failure",
            ),
        )));
    }
    serde_json::to_string(ack).map_err(DjLinkAckPublishError::Serialization)
}

/// Write one already-serialized DJ Link text frame and surface any transport
/// failure to the caller instead of dropping it on the floor.
fn send_dj_link_frame<S: Read + Write>(
    websocket: &mut tungstenite::WebSocket<S>,
    frame: String,
) -> Result<(), DjLinkAckPublishError> {
    if run_dj_link_ack_transport_failure_injection() {
        return Err(DjLinkAckPublishError::Transport(tungstenite::Error::Io(
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "injected DJ Link ACK transport failure",
            ),
        )));
    }
    websocket
        .send(Message::Text(frame))
        .map_err(DjLinkAckPublishError::Transport)
}

/// Serialize and publish one ACK in a single step for callsites whose control
/// flow does not separate the two halves across registry advancement.
fn send_dj_link_ack<S: Read + Write>(
    websocket: &mut tungstenite::WebSocket<S>,
    ack: &DjLinkAck,
) -> Result<(), DjLinkAckPublishError> {
    send_dj_link_frame(websocket, dj_link_ack_wire(ack)?)
}

/// The outbound timeline-state frame is the exact v3 envelope whose payload
/// carries exactly `state`, `loopActive`, `timelineId`, `positionBars`,
/// `playSessionId`, `pedalOwner`, and `releaseEventId`.
fn dj_link_state_wire(
    state: &DjLinkTimelineState,
    agent_id: &str,
    session_id: &str,
) -> Result<String, String> {
    state.validate()?;
    serde_json::to_string(&json!({
        "v": protocol::DJ_LINK_PROTOCOL_VERSION,
        "type": "DJ_TIMELINE_STATE",
        "agentId": agent_id,
        "sessionId": session_id,
        "sequence": state.sequence,
        "eventId": state.event_id,
        "payload": {
            "state": state.state,
            "loopActive": state.loop_active,
            "timelineId": state.timeline_id,
            "positionBars": state.position_bars,
            "playSessionId": state.play_session_id,
            "pedalOwner": state.pedal_owner,
            "releaseEventId": state.release_event_id,
        },
    }))
    .map_err(|error| error.to_string())
}

fn send_pending_dj_link_states<S: Read + Write>(
    websocket: &mut tungstenite::WebSocket<S>,
    receiver: &Receiver<DjLinkOutboundState>,
    registry: &Arc<Mutex<DjLinkRegistry>>,
    agent_id: &str,
    session_id: &str,
    generation: u64,
) -> bool {
    loop {
        let outbound = match receiver.try_recv() {
            Ok(outbound) => outbound,
            Err(TryRecvError::Empty) => return true,
            Err(TryRecvError::Disconnected) => return false,
        };
        if outbound.generation != generation {
            continue;
        }
        let should_deliver = match registry.lock() {
            Ok(registry) => registry.should_deliver_outbound_state(
                agent_id,
                session_id,
                generation,
                &outbound.state,
            ),
            Err(_) => return false,
        };
        if !should_deliver {
            continue;
        }
        // This hook is test-only and deliberately runs after the first
        // registry observation. The production permit below revalidates the
        // same generation atomically, so a HELLO replacement in this window
        // makes the old send a no-op rather than a stale socket write.
        run_dj_link_before_outbound_permit_hook();
        let dispatch_permit = match registry.lock() {
            Ok(mut registry_guard) => {
                if registry_guard
                    .begin_dispatch_for_session(agent_id, session_id, generation)
                    .is_err()
                {
                    None
                } else {
                    Some(DjLinkDispatchLease {
                        registry: Arc::clone(registry),
                        agent_id: agent_id.to_string(),
                        session_id: session_id.to_string(),
                        generation,
                    })
                }
            }
            _ => None,
        };
        let Some(_dispatch_permit) = dispatch_permit else {
            continue;
        };
        let final_should_deliver = match registry.lock() {
            Ok(registry) => registry.should_deliver_outbound_state(
                agent_id,
                session_id,
                generation,
                &outbound.state,
            ),
            Err(_) => false,
        };
        if !final_should_deliver {
            continue;
        }
        let Ok(state_text) = dj_link_state_wire(&outbound.state, agent_id, session_id) else {
            return false;
        };
        if websocket.send(Message::Text(state_text)).is_err() {
            return false;
        }
        let Ok(mut registry) = registry.lock() else {
            return false;
        };
        if !registry.is_current(agent_id, session_id, generation) {
            return false;
        }
        registry.note_outbound_state(
            &outbound.state,
            &outbound.state.event_id,
            outbound.state.sequence,
        );
    }
}

/// Compare the backend-issued credential without an early return on the
/// matching bytes.  The length is folded into the accumulator so a token of
/// a different length cannot pass while keeping the comparison bounded.
fn constant_time_token_eq(candidate: &str, expected: &str) -> bool {
    let candidate = candidate.as_bytes();
    let expected = expected.as_bytes();
    let mut difference = candidate.len() ^ expected.len();
    let width = candidate.len().max(expected.len());
    for index in 0..width {
        let left = candidate.get(index).copied().unwrap_or(0);
        let right = expected.get(index).copied().unwrap_or(0);
        difference |= usize::from(left ^ right);
    }
    difference == 0
}

/// Replace the parsed HELLO bearer credential before any envelope-derived
/// identity, replay, or session state is retained. The sentinel itself is a
/// syntactically valid token so `DjLinkEnvelope::canonical_shape` continues
/// to exercise the exact production validation path.
fn sanitize_authenticated_dj_link_hello(
    hello: &mut DjLinkEnvelope,
    payload: &mut protocol::DjLinkHelloPayload,
) -> Result<(), serde_json::Error> {
    let version = payload.version;
    let capabilities = payload.capabilities.clone();
    // `from_value` above clones the token, so overwrite both ownership sites
    // before releasing either allocation. The expected listener token remains
    // the intentional live authority in `RemoteControlConfig`.
    payload.auth_token.zeroize();
    if let Some(Value::String(auth_token)) = hello.payload.get_mut("authToken") {
        auth_token.zeroize();
    }
    hello.payload = serde_json::to_value(protocol::DjLinkHelloPayload {
        auth_token: DJ_LINK_AUTH_TOKEN_CANONICAL_SENTINEL.to_string(),
        version,
        capabilities,
    })?;
    Ok(())
}

fn handle_dj_link_client(
    stream: TcpStream,
    stop: Arc<AtomicBool>,
    config: &RemoteControlConfig,
    peer: String,
    handler: Option<DjLinkDispatchHandler>,
    registry: Arc<Mutex<DjLinkRegistry>>,
) {
    let Some(expected_token) = config.dj_link_token.as_deref() else {
        reject_http_client(stream, "404 Not Found", "DJ Link is not enabled");
        return;
    };
    if !config.dj_link_enabled {
        reject_http_client(
            stream,
            "401 Unauthorized",
            "DJ Link authentication required",
        );
        return;
    }
    let Ok(mut websocket) = accept(stream) else {
        return;
    };
    let _ = websocket
        .get_ref()
        .set_read_timeout(Some(REMOTE_SOCKET_READ_TIMEOUT));
    let _ = websocket
        .get_ref()
        .set_write_timeout(Some(REMOTE_SOCKET_WRITE_TIMEOUT));
    let hello_deadline = Instant::now() + Duration::from_secs(5);
    let mut hello = loop {
        match websocket.read() {
            Ok(Message::Text(text)) => {
                // Ingress is exact-only: anything that is not a fully valid
                // v3 envelope from the accepted agent identity is not
                // correlatable and therefore receives no reply at all.
                if let Ok(envelope) = DjLinkEnvelope::parse_json(&text) {
                    if envelope.message_type == DjLinkMessageType::Hello {
                        break envelope;
                    }
                    let ack = dj_link_ack(
                        &envelope,
                        DjLinkAckOutcome::Rejected,
                        Some("hello_required".to_string()),
                        0,
                    );
                    // Fail-close: this rejection path terminates on both
                    // outcomes and no session state exists ahead of it, but
                    // the publication itself is Result-based — a render or
                    // transport failure can never fall back to a fabricated
                    // frame.
                    let _ = send_dj_link_ack(&mut websocket, &ack);
                    return;
                }
                if Instant::now() >= hello_deadline {
                    return;
                }
            }
            Ok(Message::Close(_)) => return,
            Ok(_) => return,
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) && Instant::now() < hello_deadline =>
            {
                continue;
            }
            Err(_) => return,
        }
    };
    let Ok(mut hello_payload) =
        serde_json::from_value::<protocol::DjLinkHelloPayload>(hello.payload.clone())
    else {
        return;
    };
    if !constant_time_token_eq(&hello_payload.auth_token, expected_token) {
        let ack = dj_link_ack(
            &hello,
            DjLinkAckOutcome::Rejected,
            Some("invalid_auth".to_string()),
            0,
        );
        // Fail-close: terminal either way, with no fabricated-frame fallback.
        let _ = send_dj_link_ack(&mut websocket, &ack);
        return;
    }
    // Do not retain the accepted raw bearer credential for the rest of the
    // connection. Registry and session paths only receive the sanitized
    // envelope below.
    if sanitize_authenticated_dj_link_hello(&mut hello, &mut hello_payload).is_err() {
        return;
    }
    drop(hello_payload);
    let now = Instant::now();
    let hello_shape = match hello.canonical_shape() {
        Ok(shape) => shape,
        Err(_) => return,
    };
    let (outbound_sender, outbound_receiver) =
        mpsc::sync_channel::<DjLinkOutboundState>(DJ_LINK_OUTBOUND_QUEUE_LIMIT);
    let hello_admission = match registry.lock() {
        Ok(mut registry) => registry.admit_hello(&hello, &hello_shape, now),
        Err(_) => return,
    };
    match hello_admission {
        DjLinkAdmission::Accepted => {}
        DjLinkAdmission::Duplicate(mut ack) => {
            ack.outcome = DjLinkAckOutcome::Duplicate;
            // Fail-close: terminal either way, no fabricated-frame fallback.
            let _ = send_dj_link_ack(&mut websocket, &ack);
            return;
        }
        admission => {
            let (outcome, code) = match admission {
                DjLinkAdmission::RateLimited => {
                    (DjLinkAckOutcome::Rejected, "invalid_admission_state")
                }
                DjLinkAdmission::Busy => (DjLinkAckOutcome::Busy, "in_flight"),
                DjLinkAdmission::Conflict => (DjLinkAckOutcome::Rejected, "event_id_conflict"),
                DjLinkAdmission::ReplayNotRetained => {
                    (DjLinkAckOutcome::Rejected, "event_id_not_retained")
                }
                DjLinkAdmission::SideEffectCapacityLatched => (
                    DjLinkAckOutcome::Rejected,
                    "side_effect_id_capacity_latched",
                ),
                DjLinkAdmission::Rollback => (DjLinkAckOutcome::Rejected, "sequence_rollback"),
                DjLinkAdmission::OrderBlocked => (DjLinkAckOutcome::Rejected, "order_blocked"),
                DjLinkAdmission::Accepted | DjLinkAdmission::Duplicate(_) => {
                    (DjLinkAckOutcome::Rejected, "invalid_admission_state")
                }
            };
            let ack = dj_link_ack(&hello, outcome, Some(code.to_string()), 0);
            // Fail-close: terminal either way, no fabricated-frame fallback.
            let _ = send_dj_link_ack(&mut websocket, &ack);
            return;
        }
    }
    let generation = match registry.lock() {
        Ok(mut registry) => match registry.register(
            &hello.agent_id,
            &hello.session_id,
            hello.sequence,
            &hello.event_id,
            DjLinkRegistrationTransport {
                peer,
                outbound: outbound_sender,
                now,
            },
        ) {
            Ok(generation) => generation,
            Err(_) => {
                registry.abort_inflight(&hello, 0);
                let ack = dj_link_ack(
                    &hello,
                    DjLinkAckOutcome::Busy,
                    Some("session_replacement_busy".to_string()),
                    0,
                );
                // Fail-close: terminal either way; the inflight reservation
                // was already aborted above and nothing else advances.
                let _ = send_dj_link_ack(&mut websocket, &ack);
                return;
            }
        },
        Err(_) => return,
    };
    let hello_ack = dj_link_ack(&hello, DjLinkAckOutcome::Accepted, None, generation);
    if let Ok(mut registry) = registry.lock() {
        if registry
            .complete(&hello, generation, hello_shape, hello_ack.clone(), now)
            .is_err()
        {
            return;
        }
    } else {
        return;
    }
    // Fail-close publication: the completed HELLO receipt stays retained so
    // a reconnect replay resolves to the exact Duplicate ACK instead of
    // re-running admission, but any render or transport failure ends this
    // connection before the command loop starts. No fabricated frame and no
    // legacy fallback exists.
    if send_dj_link_ack(&mut websocket, &hello_ack).is_err() {
        return;
    }
    let mut rate_window_started = Instant::now();
    let mut rate_window_messages: u16 = 0;

    while !stop.load(Ordering::Relaxed) {
        if !send_pending_dj_link_states(
            &mut websocket,
            &outbound_receiver,
            &registry,
            &hello.agent_id,
            &hello.session_id,
            generation,
        ) {
            break;
        }
        let timed_out = registry
            .lock()
            .ok()
            .and_then(|registry| registry.sessions.get(&hello.agent_id).cloned())
            .is_none_or(|session| {
                session.session_id != hello.session_id
                    || session.generation != generation
                    || session.last_liveness.elapsed() > DJ_LINK_HEARTBEAT_TIMEOUT
            });
        if timed_out {
            break;
        }
        match websocket.read() {
            Ok(Message::Text(text)) => {
                if text.len() > DJ_LINK_MAX_FRAME_BYTES {
                    break;
                }
                // Frames that fail exact v3 parsing (including duplicate JSON
                // keys) are not correlatable and receive no reply.
                let envelope = match DjLinkEnvelope::parse_json(&text) {
                    Ok(envelope)
                        if envelope.agent_id == hello.agent_id
                            && envelope.session_id == hello.session_id =>
                    {
                        envelope
                    }
                    Ok(envelope) => {
                        let ack = dj_link_ack(
                            &envelope,
                            DjLinkAckOutcome::Rejected,
                            Some("session_mismatch".to_string()),
                            generation,
                        );
                        // Fail-close: only a successfully published reply
                        // keeps this socket reading; an unpublishable one
                        // terminates the connection without a fallback frame.
                        if send_dj_link_ack(&mut websocket, &ack).is_err() {
                            break;
                        }
                        continue;
                    }
                    Err(_) => {
                        continue;
                    }
                };
                let shape = match envelope.canonical_shape() {
                    Ok(shape) => shape,
                    Err(_) => continue,
                };
                let family_matches = registry.lock().is_ok_and(|registry| {
                    registry
                        .sessions
                        .get(&envelope.agent_id)
                        .is_some_and(|session| {
                            session.session_id == envelope.session_id
                                && session.generation == generation
                        })
                });
                if !family_matches {
                    let ack = dj_link_ack(
                        &envelope,
                        DjLinkAckOutcome::Rejected,
                        Some("session_or_generation_mismatch".to_string()),
                        generation,
                    );
                    if send_dj_link_ack(&mut websocket, &ack).is_err() {
                        break;
                    }
                    continue;
                }
                if rate_window_started.elapsed() >= Duration::from_secs(1) {
                    rate_window_started = Instant::now();
                    rate_window_messages = 0;
                }
                rate_window_messages = rate_window_messages.saturating_add(1);
                let rate_limited = rate_window_messages > config.max_messages_per_second;
                let admission = match registry.lock() {
                    Ok(mut registry) => registry.admit_with_rate_limit(
                        &envelope,
                        &shape,
                        generation,
                        Instant::now(),
                        rate_limited,
                    ),
                    Err(_) => Err("DJ Link registry lock was poisoned".to_string()),
                };
                let admission = match admission {
                    Ok(admission) => admission,
                    Err(_) => {
                        break;
                    }
                };
                let mut dispatch_lease: Option<DjLinkDispatchLease> = None;
                let mut outbound_state: Option<DjLinkTimelineState> = None;
                let mut terminalized = false;
                let ack = match admission {
                    DjLinkAdmission::RateLimited => dj_link_ack(
                        &envelope,
                        DjLinkAckOutcome::Rejected,
                        Some("rate_limit".to_string()),
                        generation,
                    ),
                    DjLinkAdmission::Duplicate(mut ack) => {
                        ack.outcome = DjLinkAckOutcome::Duplicate;
                        ack
                    }
                    DjLinkAdmission::Conflict => dj_link_ack(
                        &envelope,
                        DjLinkAckOutcome::Rejected,
                        Some("event_id_conflict".to_string()),
                        generation,
                    ),
                    DjLinkAdmission::Rollback => dj_link_ack(
                        &envelope,
                        DjLinkAckOutcome::Rejected,
                        Some("sequence_rollback".to_string()),
                        generation,
                    ),
                    DjLinkAdmission::Busy => dj_link_ack(
                        &envelope,
                        DjLinkAckOutcome::Busy,
                        Some("in_flight".to_string()),
                        generation,
                    ),
                    DjLinkAdmission::ReplayNotRetained => dj_link_ack(
                        &envelope,
                        DjLinkAckOutcome::Rejected,
                        Some("event_id_not_retained".to_string()),
                        generation,
                    ),
                    DjLinkAdmission::SideEffectCapacityLatched => dj_link_ack(
                        &envelope,
                        DjLinkAckOutcome::Rejected,
                        Some("side_effect_id_capacity_latched".to_string()),
                        generation,
                    ),
                    DjLinkAdmission::OrderBlocked => dj_link_ack(
                        &envelope,
                        DjLinkAckOutcome::Rejected,
                        Some("order_blocked".to_string()),
                        generation,
                    ),
                    DjLinkAdmission::Accepted => {
                        let admitted_current = registry.lock().is_ok_and(|registry| {
                            registry.is_current(
                                &envelope.agent_id,
                                &envelope.session_id,
                                generation,
                            )
                        });
                        let mut outcome = if !admitted_current {
                            DjLinkDispatchOutcome::Rejected {
                                code: "stale_session".to_string(),
                                state_generation: generation,
                            }
                        } else if envelope.message_type == DjLinkMessageType::Heartbeat {
                            DjLinkDispatchOutcome::Accepted {
                                state_generation: generation,
                            }
                        } else {
                            // This hook is a deterministic test barrier only;
                            // the real path is a no-op.  The final generation
                            // check and lease acquisition happen under the
                            // same registry lock, so replacement cannot win
                            // between validation and physical dispatch.
                            run_dj_link_pre_dispatch_hook();
                            let maybe_lease = match registry.lock() {
                                Ok(mut registry_guard) => {
                                    if registry_guard.begin_dispatch(&envelope, generation).is_ok()
                                    {
                                        Some(DjLinkDispatchLease {
                                            registry: Arc::clone(&registry),
                                            agent_id: envelope.agent_id.clone(),
                                            session_id: envelope.session_id.clone(),
                                            generation,
                                        })
                                    } else {
                                        None
                                    }
                                }
                                Err(_) => None,
                            };
                            if let Some(lease) = maybe_lease {
                                dispatch_lease = Some(lease);
                                // This is intentionally after the atomic final
                                // check/lease commit.  A new HELLO can now
                                // only receive Busy until this request has
                                // terminalized and sent its ACK.
                                run_dj_link_after_final_check_hook();
                                handler
                                    .as_ref()
                                    .map(|handler| {
                                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                                            || handler(envelope.clone()),
                                        ))
                                        .unwrap_or(
                                            DjLinkDispatchOutcome::Rejected {
                                                code: "handler_failed".to_string(),
                                                state_generation: generation,
                                            },
                                        )
                                    })
                                    .unwrap_or(DjLinkDispatchOutcome::NoMapping {
                                        state_generation: generation,
                                    })
                            } else {
                                DjLinkDispatchOutcome::Rejected {
                                    code: "stale_session".to_string(),
                                    state_generation: generation,
                                }
                            }
                        };
                        let dispatch_was_busy =
                            matches!(&outcome, DjLinkDispatchOutcome::Busy { .. });
                        // A replacement may arrive while the application
                        // handler is running.  It cannot undo an already
                        // published physical command, but it must prevent
                        // the stale generation from publishing an ACK or
                        // terminal receipt into the new authority lane.
                        let terminal_current = registry.lock().is_ok_and(|registry| {
                            registry.is_current(
                                &envelope.agent_id,
                                &envelope.session_id,
                                generation,
                            )
                        });
                        let stale_generation = !terminal_current;
                        if stale_generation {
                            outcome = DjLinkDispatchOutcome::Rejected {
                                code: "stale_session".to_string(),
                                state_generation: generation,
                            };
                        }
                        // Fail closed: an admitted timeline-state request
                        // whose application outcome carries no authoritative
                        // state must never complete silently. A silent
                        // completion publishes no state frame, never marks
                        // the snapshot ready, and wedges every later physical
                        // command behind "order_blocked" until the session
                        // dies at the liveness timeout. Busy keeps its own
                        // retryable abort path; this rejection is terminal
                        // and replays as an exact receipt on retry.
                        if !stale_generation
                            && envelope.message_type == DjLinkMessageType::TimelineStateRequest
                            && matches!(
                                outcome,
                                DjLinkDispatchOutcome::Accepted { .. }
                                    | DjLinkDispatchOutcome::NoMapping { .. }
                            )
                        {
                            outcome = DjLinkDispatchOutcome::Rejected {
                                code: "state_unavailable".to_string(),
                                state_generation: generation,
                            };
                        }
                        if !stale_generation {
                            if let DjLinkDispatchOutcome::TimelineState { state, .. } = &outcome {
                                outbound_state = Some(state.clone());
                            }
                        }
                        let (outcome, code, state_generation) = match outcome {
                            DjLinkDispatchOutcome::Accepted { state_generation } => {
                                (DjLinkAckOutcome::Accepted, None, state_generation)
                            }
                            DjLinkDispatchOutcome::NoMapping { state_generation } => {
                                (DjLinkAckOutcome::NoMapping, None, state_generation)
                            }
                            DjLinkDispatchOutcome::TimelineState {
                                state_generation, ..
                            } => (DjLinkAckOutcome::Accepted, None, state_generation),
                            DjLinkDispatchOutcome::Rejected {
                                code,
                                state_generation,
                            } => (DjLinkAckOutcome::Rejected, Some(code), state_generation),
                            DjLinkDispatchOutcome::Busy {
                                code,
                                state_generation,
                            } => (DjLinkAckOutcome::Busy, Some(code), state_generation),
                        };
                        let mut ack = dj_link_ack(&envelope, outcome, code, state_generation);
                        if let Ok(mut registry) = registry.lock() {
                            if stale_generation {
                                registry.reject_inflight(&envelope, generation);
                            } else if dispatch_was_busy {
                                registry.abort_inflight(&envelope, generation);
                            } else if registry
                                .complete(&envelope, generation, shape, ack.clone(), Instant::now())
                                .is_err()
                            {
                                // A final generation/capacity/poison failure
                                // is a typed fail-closed reply, never a stale
                                // success and never a physical re-execution.
                                registry.reject_inflight(&envelope, generation);
                                ack = dj_link_ack(
                                    &envelope,
                                    DjLinkAckOutcome::Rejected,
                                    Some("terminalization_failed".to_string()),
                                    generation,
                                );
                            } else {
                                terminalized = true;
                            }
                        } else {
                            ack = dj_link_ack(
                                &envelope,
                                DjLinkAckOutcome::Rejected,
                                Some("registry_poisoned".to_string()),
                                generation,
                            );
                        }
                        ack
                    }
                };
                // Fail-close publication: render the exact v3 ACK first, then
                // deliver it. A render or transport failure terminates this
                // connection immediately — no fabricated frame is possible,
                // no legacy payload fallback exists, and no further protocol
                // processing happens. The terminal receipt completed above
                // stays retained, so a reconnect replay resolves to the exact
                // Duplicate ACK rather than a re-execution.
                let Ok(ack_wire) = dj_link_ack_wire(&ack) else {
                    break;
                };
                if terminalized {
                    if let Ok(mut registry) = registry.lock() {
                        match envelope.message_type {
                            DjLinkMessageType::StateSync => {
                                registry.note_state_sync(&envelope, generation);
                            }
                            DjLinkMessageType::TimelineStateRequest => {
                                registry.note_timeline_state_request(&envelope, generation);
                            }
                            _ => {}
                        }
                    }
                }
                if send_dj_link_frame(&mut websocket, ack_wire).is_err() {
                    break;
                }
                // Keep the state response out of the terminal ACK itself:
                // exact retries replay the ACK without re-running the engine,
                // while snapshot readiness is published only after the state
                // frame has been accepted by the socket.
                if terminalized {
                    if let Some(state) = outbound_state {
                        if let Ok(state_text) =
                            dj_link_state_wire(&state, &hello.agent_id, &hello.session_id)
                        {
                            if websocket.send(Message::Text(state_text)).is_ok() {
                                if let Ok(mut registry) = registry.lock() {
                                    registry.note_outbound_state(
                                        &state,
                                        &envelope.event_id,
                                        envelope.sequence,
                                    );
                                    if envelope.message_type
                                        == DjLinkMessageType::TimelineStateRequest
                                    {
                                        registry.mark_snapshot_ready(&envelope, generation);
                                    }
                                }
                            }
                        }
                    }
                }
                // Keep the lease through ACK publication.  A replacement may
                // proceed only after the old request is no longer able to
                // publish an authority result on this socket.
                drop(dispatch_lease);
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(payload)) => {
                let _ = websocket.send(Message::Pong(payload));
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }
    if let Ok(mut registry) = registry.lock() {
        registry.close_if_current(&hello.agent_id, &hello.session_id, generation);
    }
}

fn handle_websocket_client<F, S>(
    stream: TcpStream,
    stop: Arc<AtomicBool>,
    callback: &F,
    snapshot_provider: &S,
    providers: &RemoteWebProviders,
    context: RemoteWebsocketContext<'_>,
    client: RemoteWebsocketClient,
) where
    F: Fn(RemoteInputEvent) + ?Sized,
    S: Fn() -> Option<EngineSnapshot> + ?Sized,
{
    let config = context.config;
    let clients = context.clients;
    let client_id = client.client_id;
    let disconnect = client.disconnect;
    let video_runtime_status_provider = &providers.video_runtime_status_provider;
    let video_output_render_plans_provider = &providers.video_output_render_plans_provider;
    let external_video_io_plans_provider = &providers.external_video_io_plans_provider;
    let external_video_transport_status_provider =
        &providers.external_video_transport_status_provider;
    let external_video_transport_sync_provider = &providers.external_video_transport_sync_provider;
    let Ok(mut websocket) = accept(stream) else {
        return;
    };
    let _ = websocket
        .get_ref()
        .set_read_timeout(Some(REMOTE_SOCKET_READ_TIMEOUT));
    let _ = websocket
        .get_ref()
        .set_write_timeout(Some(REMOTE_SOCKET_WRITE_TIMEOUT));
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
                        let response = match (snapshot_provider)() {
                            Some(snapshot) => snapshot_response_json(&snapshot),
                            None => snapshot_unavailable_response_json(),
                        };
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
                        let _ =
                            websocket.send(Message::Text(remote_parse_error_response_json(&error)));
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

fn snapshot_unavailable_response_json() -> String {
    serde_json::json!({
        "ok": false,
        "error": "Engine snapshot is temporarily unavailable; retry",
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
        body.len()
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

fn read_u64_array(value: &Value, key: &'static str) -> Result<Vec<u64>, RemoteParseError> {
    value
        .get(key)
        .ok_or(RemoteParseError::MissingField(key))?
        .as_array()
        .ok_or(RemoteParseError::InvalidField(key))?
        .iter()
        .map(|entry| entry.as_u64().ok_or(RemoteParseError::InvalidField(key)))
        .collect()
}

fn read_string_array(value: &Value, key: &'static str) -> Result<Vec<String>, RemoteParseError> {
    value
        .get(key)
        .ok_or(RemoteParseError::MissingField(key))?
        .as_array()
        .ok_or(RemoteParseError::InvalidField(key))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(ToString::to_string)
                .ok_or(RemoteParseError::InvalidField(key))
        })
        .collect()
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

    /// The exact v3 authority identity every fixture must use. Production
    /// ingress rejects any other `agentId` before authentication.
    const DJ_V3_AGENT: &str = protocol::DJ_LINK_AGENT_ID;
    const DJ_V3_TOKEN: &str = "0123456789abcdef0123456789abcdef";

    fn dj_v3_capabilities() -> Value {
        serde_json::json!(protocol::DJ_LINK_REQUIRED_CAPABILITIES)
    }

    #[test]
    fn authenticated_hello_sanitization_keeps_raw_token_out_of_retained_identity() {
        let raw_token = "dj-link-raw-token-must-not-survive-auth";
        let mut hello = DjLinkEnvelope {
            v: protocol::DJ_LINK_PROTOCOL_VERSION,
            message_type: DjLinkMessageType::Hello,
            agent_id: DJ_V3_AGENT.to_string(),
            session_id: "sanitized-session".to_string(),
            sequence: 1,
            event_id: "sanitized-hello".to_string(),
            payload: json!({
                "authToken": raw_token,
                "version": protocol::DJ_LINK_PROTOCOL_VERSION,
                "capabilities": protocol::DJ_LINK_REQUIRED_CAPABILITIES,
            }),
        };
        let mut payload: protocol::DjLinkHelloPayload =
            serde_json::from_value(hello.payload.clone()).unwrap();
        assert!(constant_time_token_eq(&payload.auth_token, raw_token));
        sanitize_authenticated_dj_link_hello(&mut hello, &mut payload).unwrap();
        assert!(payload.auth_token.as_bytes().iter().all(|byte| *byte == 0));
        drop(payload);

        let shape = hello.canonical_shape().unwrap();
        assert!(!shape.contains(raw_token));
        assert!(shape.contains(DJ_LINK_AUTH_TOKEN_CANONICAL_SENTINEL));
        assert!(!format!("{hello:?}").contains(raw_token));

        let mut registry = DjLinkRegistry::default();
        assert_eq!(
            registry.admit_hello(&hello, &shape, Instant::now()),
            DjLinkAdmission::Accepted
        );
        // Registry keeps only keyed digests at this point, never the HELLO
        // payload or its bearer credential.
        assert!(!format!("{registry:?}").contains(raw_token));
    }

    #[test]
    fn remote_parse_errors_are_static_valid_json_without_attacker_reflection() {
        let sentinel = "attacker\"\\dj-token-sentinel";
        let error = request_from_text(&json!({ "type": sentinel }).to_string())
            .expect_err("unknown renderer command must be rejected");
        assert_eq!(error, RemoteParseError::UnknownType);
        let response = remote_parse_error_response_json(&error);
        let decoded: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(decoded, json!({ "ok": false, "error": "unknown_type" }));
        assert!(!response.contains(sentinel));
    }

    fn dj_v3_hello(session: &str, event_id: &str) -> Value {
        json!({
            "v": 3,
            "type": "DJ_AGENT_HELLO",
            "agentId": DJ_V3_AGENT,
            "sessionId": session,
            "sequence": 1,
            "eventId": event_id,
            "payload": {
                "authToken": DJ_V3_TOKEN,
                "version": 3,
                "capabilities": dj_v3_capabilities(),
            },
        })
    }

    fn dj_v3_envelope(
        message_type: DjLinkMessageType,
        session: &str,
        sequence: u64,
        event_id: &str,
        payload: Value,
    ) -> DjLinkEnvelope {
        DjLinkEnvelope {
            v: protocol::DJ_LINK_PROTOCOL_VERSION,
            message_type,
            agent_id: DJ_V3_AGENT.to_string(),
            session_id: session.to_string(),
            sequence,
            event_id: event_id.to_string(),
            payload,
        }
    }

    /// A small, valid physical command used by registry-level idempotency
    /// tests where the concrete command shape is irrelevant.
    fn dj_v3_loop_set_payload(active: bool) -> Value {
        json!({
            "active": active,
            "timelineId": "7",
            "playSessionId": "play-1",
        })
    }

    fn dj_v3_state_sync_payload(released: bool) -> Value {
        let mut payload = json!({ "released": released });
        if !released {
            payload["ownerDeck"] = json!(1);
            payload["ownerDeckId"] = json!("rekordbox-deck-1");
            payload["activePlaySessionId"] = json!("play-1");
        }
        payload
    }

    fn dj_v3_generic_track_active_payload(play_session: &str) -> Value {
        json!({
            "deck": 1,
            "deckId": "rekordbox-deck-1",
            "contentId": "track-1",
            "trackBpm": 128.0,
            "positionAtSendSec": 1.25,
            "effectiveBpm": 128.5,
            "positionRevision": 7,
            "sampleAgeMs": 42,
            "isPlaying": true,
            "startedAt": "2026-08-21T00:00:00Z",
            "playSessionId": play_session,
        })
    }

    /// Handler that answers timeline state requests with an authoritative
    /// state frame so sockets can complete the snapshot gate.
    fn dj_v3_state_responder(state_generation: u64) -> DjLinkDispatchHandler {
        Arc::new(move |envelope| {
            if envelope.message_type == DjLinkMessageType::TimelineStateRequest {
                return DjLinkDispatchOutcome::TimelineState {
                    state_generation,
                    state: DjLinkTimelineState {
                        message_type: "DJ_TIMELINE_STATE".to_string(),
                        event_id: envelope.event_id.clone(),
                        sequence: envelope.sequence,
                        state: protocol::DjLinkTimelineStateValue::Idle,
                        loop_active: false,
                        timeline_id: "1".to_string(),
                        position_bars: 0,
                        play_session_id: None,
                        pedal_owner: None,
                        release_event_id: None,
                    },
                };
            }
            DjLinkDispatchOutcome::Accepted { state_generation }
        })
    }

    /// Drives one freshly authenticated socket through HELLO → STATE_SYNC →
    /// STATE_REQUEST (+ authoritative state frame) so physical commands pass
    /// the snapshot/order gate.
    fn complete_dj_v3_snapshot_gate<S>(
        client: &mut tungstenite::WebSocket<S>,
        server_port: u16,
        session: &str,
        server: &RemoteWsServer,
    ) where
        S: Read + Write,
    {
        let _ = server_port;
        client
            .send(Message::Text(
                dj_v3_hello(session, &format!("hello-{session}")).to_string(),
            ))
            .unwrap();
        let hello_ack = read_dj_ack(client);
        assert_eq!(hello_ack.outcome, DjLinkAckOutcome::Accepted);
        client
            .send(Message::Text(
                json!({
                    "v": 3,
                    "type": "DJ_STATE_SYNC",
                    "agentId": DJ_V3_AGENT,
                    "sessionId": session,
                    "sequence": 2,
                    "eventId": format!("sync-{session}"),
                    "payload": dj_v3_state_sync_payload(false),
                })
                .to_string(),
            ))
            .unwrap();
        let sync_ack = read_dj_ack(client);
        assert_eq!(sync_ack.outcome, DjLinkAckOutcome::Accepted);
        client
            .send(Message::Text(
                json!({
                    "v": 3,
                    "type": "DJ_TIMELINE_STATE_REQUEST",
                    "agentId": DJ_V3_AGENT,
                    "sessionId": session,
                    "sequence": 3,
                    "eventId": format!("request-{session}"),
                    "payload": {},
                })
                .to_string(),
            ))
            .unwrap();
        let state_request_ack = read_dj_ack(client);
        assert_eq!(
            state_request_ack.outcome,
            DjLinkAckOutcome::Accepted,
            "state request acknowledgement: {state_request_ack:?}"
        );
        match client.read().unwrap() {
            Message::Text(text) => {
                let state: Value = serde_json::from_str(&text).unwrap();
                assert_eq!(state["type"], "DJ_TIMELINE_STATE");
            }
            other => panic!("unexpected timeline state reply: {other:?}"),
        }
        wait_for_dj_link_snapshot_ready(server);
    }

    fn read_dj_ack<S>(client: &mut tungstenite::WebSocket<S>) -> DjLinkAck
    where
        S: Read + Write,
    {
        match client.read().unwrap() {
            Message::Text(text) => serde_json::from_str(&text).unwrap(),
            other => panic!("unexpected DJ Link reply: {other:?}"),
        }
    }

    fn wait_for_dj_link_snapshot_ready(server: &RemoteWsServer) {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if server.dj_link_status().snapshot_ready {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "DJ Link snapshot-ready publication did not complete within 1 second"
            );
            thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn typed_wire_inventory_is_exact_and_parser_recognizes_every_wire_selector() {
        const EXPECTED_WIRE_OPERATION_COUNT: usize = 58;
        assert_eq!(
            RemoteWireOperation::CONTROL_PLANE_OPERATIONS.len(),
            EXPECTED_WIRE_OPERATION_COUNT
        );
        let variants = RemoteWireOperation::CONTROL_PLANE_OPERATIONS
            .iter()
            .map(|(variant, _)| *variant)
            .collect::<std::collections::BTreeSet<_>>();
        let wire_types = RemoteWireOperation::CONTROL_PLANE_OPERATIONS
            .iter()
            .map(|(_, wire_type)| *wire_type)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(variants.len(), EXPECTED_WIRE_OPERATION_COUNT);
        assert_eq!(wire_types.len(), EXPECTED_WIRE_OPERATION_COUNT);
        for (_, wire_type) in RemoteWireOperation::CONTROL_PLANE_OPERATIONS {
            let operation = RemoteWireOperation::from_type(wire_type)
                .expect("generated wire operation must be accepted");
            assert_eq!(operation.wire_type(), *wire_type);
            let parsed = request_from_text(&format!(r#"{{"type":"{wire_type}"}}"#));
            assert!(
                !matches!(parsed, Err(RemoteParseError::UnknownType)),
                "the parser must recognize every generated wire selector: {wire_type}"
            );
        }
        assert_eq!(RemoteWireOperation::from_type("unknownOperation"), None);
    }

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
            event_from_text(
                r#"{"type":"setOperatorSelection","fixture_ids":[4,9,4],"attributes":["Dimmer","ColorRed"]}"#
            ),
            Ok(RemoteInputEvent::SetOperatorSelection(
                OperatorSelectionContext {
                    fixture_ids: vec![4, 9, 4],
                    attributes: vec!["Dimmer".to_string(), "ColorRed".to_string()],
                }
            ))
        );
        assert_eq!(
            event_from_text(r#"{"type":"setOperatorFeatureFader","target_index":1,"value":0.5}"#),
            Ok(RemoteInputEvent::SetOperatorFeatureFader {
                target_index: 1,
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
            Err(RemoteParseError::UnknownType)
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
        let mapping = VideoOutputMapping {
            offset_x: 0.25,
            aspect_ratio: 1.75,
            aspect_mode: protocol::VideoOutputAspectMode::Fit,
            keystone_y: -0.1,
            ..Default::default()
        };
        assert_eq!(
            event_from_text(
                r#"{"type":"setVideoOutputMapping","output_id":4,"mapping":{"offset_x":0.25,"offset_y":0,"scale_x":1,"scale_y":1,"rotation_deg":0,"aspect_ratio":1.75,"aspect_mode":"Fit","lens_distortion":0,"keystone_x":0,"keystone_y":-0.1,"corner_top_left_x":0,"corner_top_left_y":0,"corner_top_right_x":0,"corner_top_right_y":0,"corner_bottom_right_x":0,"corner_bottom_right_y":0,"corner_bottom_left_x":0,"corner_bottom_left_y":0}}"#
            ),
            Ok(RemoteInputEvent::SetVideoOutputMapping {
                output_id: 4,
                mapping: Box::new(mapping),
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
            Err(RemoteParseError::UnknownType)
        );
        assert_eq!(
            event_from_text(r#"{"type":"getVideoOutputRenderPlans"}"#),
            Err(RemoteParseError::UnknownType)
        );
        assert_eq!(
            event_from_text(r#"{"type":"getExternalVideoTransportStatus"}"#),
            Err(RemoteParseError::UnknownType)
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
    fn serializes_snapshot_unavailable_without_a_snapshot_event() {
        let response = snapshot_unavailable_response_json();
        let value: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(value["ok"], false);
        assert!(value["error"]
            .as_str()
            .is_some_and(|error| error.contains("retry")));
        assert!(value.get("type").is_none());
        assert!(value.get("snapshot").is_none());
        assert!(REMOTE_PAGE_HTML
            .contains(r#"if(msg.ok&&msg.type==="snapshot"){applySnapshot(msg.snapshot)"#));
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
    fn generic_remote_callback_panic_releases_registered_client_slot() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let panic_once = Arc::new(AtomicBool::new(true));
        let callback = {
            let panic_once = Arc::clone(&panic_once);
            move |_| {
                if panic_once.swap(false, Ordering::SeqCst) {
                    panic!("test-only generic Web Remote callback panic");
                }
            }
        };
        let server = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                max_connections: 1,
                ..RemoteControlConfig::default()
            },
            callback,
        )
        .unwrap();

        let url = format!("ws://127.0.0.1:{port}/ws?token=123456");
        let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();
        let connected_deadline = Instant::now() + Duration::from_secs(1);
        while server.status().active_connections != 1 {
            assert!(
                Instant::now() < connected_deadline,
                "generic Web Remote client did not register"
            );
            thread::sleep(Duration::from_millis(2));
        }

        client
            .send(Message::Text(r#"{"type":"setBpm","bpm":120}"#.to_string()))
            .unwrap();
        let _ = client.read();

        let released_deadline = Instant::now() + Duration::from_secs(1);
        while server.status().active_connections != 0 {
            assert!(
                Instant::now() < released_deadline,
                "callback panic stranded the generic Web Remote client slot"
            );
            thread::sleep(Duration::from_millis(2));
        }

        let mut replacement = None;
        let replacement_deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < replacement_deadline {
            if let Ok(connection) = tungstenite::connect(url.as_str()) {
                replacement = Some(connection);
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let (mut replacement, _) =
            replacement.expect("generic Web Remote did not re-admit after callback panic cleanup");
        let replacement_connected_deadline = Instant::now() + Duration::from_secs(1);
        while server.status().active_connections != 1 {
            assert!(
                Instant::now() < replacement_connected_deadline,
                "replacement generic Web Remote client did not register"
            );
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(server.status().active_connections, 1);
        let _ = replacement.close(None);
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

    #[test]
    fn dj_link_transport_path_auth_and_bind_are_separate_from_remote_pin() {
        let valid = "GET /dj-link HTTP/1.1\r\nHost: 192.168.1.24:9100\r\nAuthorization: Bearer ignored\r\nUpgrade: websocket\r\n\r\n";
        assert_eq!(request_path(valid), Some("/dj-link"));
        // The HTTP header is intentionally irrelevant; authentication is
        // carried solely by the strict HELLO payload.
        assert!(constant_time_token_eq(
            "0123456789abcdef0123456789abcdef",
            "0123456789abcdef0123456789abcdef"
        ));
        assert!(!constant_time_token_eq(
            "0123456789abcdef0123456789abcdee",
            "0123456789abcdef0123456789abcdef"
        ));
        let invalid = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "0.0.0.0".to_string(),
                pairing_pin: "123456".to_string(),
                allow_lan: true,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("0.0.0.0".to_string()),
                dj_link_token: Some("0123456789abcdef0123456789abcdef".to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
        );
        assert!(matches!(
            invalid,
            Err(RemoteWsError::InvalidDjLinkBindAddress)
        ));
    }

    #[test]
    fn dj_link_registry_hello_session_replay_conflict_rollback_and_ack_are_bounded() {
        let now = Instant::now();
        let mut registry = DjLinkRegistry::default();
        let old_generation = registry.register_test_session(DJ_V3_AGENT, "old", 1, "old-peer", now);
        let new_generation = registry.register_test_session(DJ_V3_AGENT, "new", 1, "new-peer", now);
        assert!(new_generation > old_generation);
        registry.close_if_current(DJ_V3_AGENT, "old", old_generation);
        assert!(registry.is_current(DJ_V3_AGENT, "new", new_generation));
        // A side-effectful command exercises the idempotency tombstones;
        // heartbeats deliberately bypass them (no physical side effect).
        registry
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;

        let envelope = dj_v3_envelope(
            DjLinkMessageType::TimelineLoopSet,
            "new",
            2,
            "event-1",
            dj_v3_loop_set_payload(true),
        );
        let shape = envelope.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&envelope, &shape, new_generation, now)
                .unwrap(),
            DjLinkAdmission::Accepted
        );
        let ack = dj_link_ack(&envelope, DjLinkAckOutcome::Accepted, None, 3);
        registry
            .complete(&envelope, new_generation, shape.clone(), ack.clone(), now)
            .unwrap();
        assert_eq!(registry.terminal_count(), 1);
        assert_eq!(registry.seen_event_count(), 1);
        match registry
            .admit(&envelope, &shape, new_generation, now)
            .unwrap()
        {
            DjLinkAdmission::Duplicate(receipt) => assert_eq!(receipt, ack),
            other => panic!("expected exact terminal replay, got {other:?}"),
        }
        let mut conflict = envelope.clone();
        conflict.sequence = 3;
        let conflict_shape = conflict.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&conflict, &conflict_shape, new_generation, now)
                .unwrap(),
            DjLinkAdmission::Conflict
        );
        let mut rollback = envelope;
        rollback.event_id = "event-2".to_string();
        rollback.sequence = 1;
        let rollback_shape = rollback.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&rollback, &rollback_shape, new_generation, now)
                .unwrap(),
            DjLinkAdmission::Rollback
        );
    }

    #[test]
    fn dj_link_process_fence_survives_listener_restart_but_not_new_owner() {
        let now = Instant::now();
        let process_fence = new_dj_link_process_fence();
        let physical = |session_id: &str, sequence: u64| {
            dj_v3_envelope(
                DjLinkMessageType::TimelineLoopSet,
                session_id,
                sequence,
                "restart-event",
                dj_v3_loop_set_payload(true),
            )
        };

        let mut first_listener = DjLinkRegistry::with_process_fence(Arc::clone(&process_fence));
        let first_generation = first_listener.register_test_session(
            DJ_V3_AGENT,
            "first-session",
            1,
            "first-peer",
            now,
        );
        first_listener
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;
        let first = physical("first-session", 2);
        let first_shape = first.canonical_shape().unwrap();
        assert_eq!(
            first_listener
                .admit(&first, &first_shape, first_generation, now)
                .unwrap(),
            DjLinkAdmission::Accepted
        );
        first_listener
            .complete(
                &first,
                first_generation,
                first_shape,
                dj_link_ack(&first, DjLinkAckOutcome::Accepted, None, first_generation),
                now,
            )
            .unwrap();

        // A replacement listener gets a new session/terminal registry but the
        // same process fence.  A fresh transport sequence cannot launder the
        // already executed physical identity.
        let mut replacement_listener =
            DjLinkRegistry::with_process_fence(Arc::clone(&process_fence));
        let replacement_generation = replacement_listener.register_test_session(
            DJ_V3_AGENT,
            "replacement-session",
            1,
            "replacement-peer",
            now,
        );
        replacement_listener
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;
        let replay = physical("replacement-session", 2);
        let replay_shape = replay.canonical_shape().unwrap();
        assert_eq!(
            replacement_listener
                .admit(&replay, &replay_shape, replacement_generation, now)
                .unwrap(),
            DjLinkAdmission::ReplayNotRetained
        );

        // A genuinely new owner/process injects a fresh fence and may admit
        // the same physical identity independently.
        let mut new_owner_listener =
            DjLinkRegistry::with_process_fence(new_dj_link_process_fence());
        let new_owner_generation = new_owner_listener.register_test_session(
            DJ_V3_AGENT,
            "new-owner-session",
            1,
            "new-owner-peer",
            now,
        );
        new_owner_listener
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;
        let new_owner_replay = physical("new-owner-session", 2);
        let new_owner_shape = new_owner_replay.canonical_shape().unwrap();
        assert_eq!(
            new_owner_listener
                .admit(
                    &new_owner_replay,
                    &new_owner_shape,
                    new_owner_generation,
                    now,
                )
                .unwrap(),
            DjLinkAdmission::Accepted
        );
    }

    #[test]
    fn dj_link_admitted_frames_refresh_liveness_and_rejected_frames_do_not() {
        let registered_at = Instant::now();
        let mut registry = DjLinkRegistry::default();
        let generation = registry.register_test_session(
            DJ_V3_AGENT,
            "liveness-session",
            10,
            "liveness-peer",
            registered_at,
        );
        registry
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;
        let liveness_of = |registry: &DjLinkRegistry| {
            registry
                .sessions
                .get(DJ_V3_AGENT)
                .expect("liveness session")
                .last_liveness
        };
        assert_eq!(liveness_of(&registry), registered_at);

        // An admitted non-heartbeat physical frame refreshes session
        // liveness: continuous valid SYNC/command traffic can never time out.
        let admitted_at = registered_at + Duration::from_millis(500);
        let loop_set = dj_v3_envelope(
            DjLinkMessageType::TimelineLoopSet,
            "liveness-session",
            11,
            "liveness-command",
            dj_v3_loop_set_payload(true),
        );
        let loop_shape = loop_set.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&loop_set, &loop_shape, generation, admitted_at)
                .unwrap(),
            DjLinkAdmission::Accepted
        );
        assert_eq!(
            liveness_of(&registry),
            admitted_at,
            "an admitted non-heartbeat frame must refresh liveness"
        );

        // Rejected classifications never extend liveness.
        let rejected_at = admitted_at + Duration::from_millis(500);
        let stale_sequence = dj_v3_envelope(
            DjLinkMessageType::TimelineLoopSet,
            "liveness-session",
            5,
            "liveness-stale-sequence",
            dj_v3_loop_set_payload(false),
        );
        let stale_shape = stale_sequence.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&stale_sequence, &stale_shape, generation, rejected_at)
                .unwrap(),
            DjLinkAdmission::Rollback
        );
        let limited = dj_v3_envelope(
            DjLinkMessageType::TimelineLoopSet,
            "liveness-session",
            12,
            "liveness-rate-limited",
            dj_v3_loop_set_payload(true),
        );
        let limited_shape = limited.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit_with_rate_limit(&limited, &limited_shape, generation, rejected_at, true,)
                .unwrap(),
            DjLinkAdmission::RateLimited
        );
        assert_eq!(
            registry
                .admit(&loop_set, &loop_shape, generation, rejected_at)
                .unwrap(),
            DjLinkAdmission::Busy,
            "an exact replay of an in-flight frame is a Busy duplicate"
        );
        assert_eq!(
            liveness_of(&registry),
            admitted_at,
            "rejected and duplicate frames must not extend liveness"
        );

        // Heartbeats keep working through the same shared admission path.
        let heartbeat_at = rejected_at + Duration::from_millis(500);
        let heartbeat = dj_v3_envelope(
            DjLinkMessageType::Heartbeat,
            "liveness-session",
            13,
            "liveness-heartbeat",
            json!({}),
        );
        let heartbeat_shape = heartbeat.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&heartbeat, &heartbeat_shape, generation, heartbeat_at)
                .unwrap(),
            DjLinkAdmission::Accepted
        );
        registry
            .complete(
                &heartbeat,
                generation,
                heartbeat_shape,
                dj_link_ack(&heartbeat, DjLinkAckOutcome::Accepted, None, generation),
                heartbeat_at,
            )
            .unwrap();
        assert_eq!(liveness_of(&registry), heartbeat_at);
    }

    #[test]
    fn dj_link_status_traffic_never_consumes_the_physical_fence() {
        let now = Instant::now();
        let mut registry = DjLinkRegistry::with_side_effect_event_limit(8);
        let generation = registry.register_test_session(
            DJ_V3_AGENT,
            "state-sync-session",
            1,
            "state-sync-peer",
            now,
        );
        // Every validated STATE_SYNC observation — regardless of released or
        // complete generic owner context — is idempotent status
        // traffic. It terminalizes normally and never tombstones a physical
        // identity or consumes fence capacity.
        for (sequence, event_id, released, owner_deck, play_session) in [
            (2u64, "sync-unreleased", false, Some(1), Some("p1")),
            (3, "sync-released", true, None, None),
            (4, "sync-released-deck", true, Some(4), Some("p-old")),
            (5, "sync-null-deck", false, None, None),
        ] {
            let mut payload = json!({ "released": released });
            if let (Some(deck), Some(play_session)) = (owner_deck, play_session) {
                payload["ownerDeck"] = json!(deck);
                payload["ownerDeckId"] = json!(format!("rekordbox-deck-{deck}"));
                payload["activePlaySessionId"] = json!(play_session);
            }
            let envelope = dj_v3_envelope(
                DjLinkMessageType::StateSync,
                "state-sync-session",
                sequence,
                event_id,
                payload,
            );
            let shape = envelope.canonical_shape().unwrap();
            assert_eq!(
                registry.admit(&envelope, &shape, generation, now).unwrap(),
                DjLinkAdmission::Accepted,
                "StateSync {event_id} admission mismatch"
            );
            let inflight_physical = registry
                .inflight
                .get(&(envelope.agent_id.clone(), envelope.event_id.clone()))
                .map(|inflight| inflight.is_physical);
            assert_eq!(inflight_physical, Some(false));

            // Completion must not reclassify: mutating the stored payload
            // between admission and completion cannot launder the bit.
            registry
                .complete(
                    &envelope,
                    generation,
                    shape,
                    dj_link_ack(&envelope, DjLinkAckOutcome::Accepted, None, generation),
                    now,
                )
                .unwrap();
        }
        assert_eq!(registry.seen_event_count(), 0);
        assert!(!registry.side_effect_capacity_latched());
        assert!(registry.inflight.is_empty());

        // The measured external LOOP command is physical and does consume a
        // permanent identity slot exactly once. It passes the snapshot/order
        // gate only after the session completed the handshake.
        registry
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;
        let loop_command = |sequence: u64, event_id: &str| {
            dj_v3_envelope(
                DjLinkMessageType::LoopState,
                "state-sync-session",
                sequence,
                event_id,
                json!({
                    "deck": 1,
                    "deckId": "rekordbox-deck-1",
                    "playSessionId": "play-loop",
                    "loop": {
                        "active": true,
                        "startBeat": 16.0,
                        "endBeat": 32.0,
                        "lengthBeats": 16.0,
                        "revision": 1,
                        "sampleAgeMs": 9,
                        "source": "rekordbox-hook-measured",
                    },
                }),
            )
        };
        let physical = loop_command(6, "loop-command");
        let physical_shape = physical.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&physical, &physical_shape, generation, now)
                .unwrap(),
            DjLinkAdmission::Accepted
        );
        registry
            .complete(
                &physical,
                generation,
                physical_shape,
                dj_link_ack(&physical, DjLinkAckOutcome::Accepted, None, generation),
                now,
            )
            .unwrap();
        assert_eq!(registry.seen_event_count(), 1);
    }

    #[test]
    fn dj_link_listener_restart_reuses_process_fence_and_fresh_owner_is_admitted() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let process_fence = new_dj_link_process_fence();
        let dispatches = Arc::new(AtomicU64::new(0));
        let handler: DjLinkDispatchHandler = {
            let dispatches = Arc::clone(&dispatches);
            let inner = dj_v3_state_responder(21);
            Arc::new(move |envelope| {
                if !matches!(
                    envelope.message_type,
                    DjLinkMessageType::StateSync | DjLinkMessageType::TimelineStateRequest
                ) {
                    dispatches.fetch_add(1, Ordering::SeqCst);
                }
                inner(envelope)
            })
        };
        let config = |port| RemoteControlConfig {
            bind_ip: "127.0.0.1".to_string(),
            port,
            pairing_pin: "123456".to_string(),
            max_connections: 1,
            dj_link_enabled: true,
            dj_link_bind_ip: Some("127.0.0.1".to_string()),
            dj_link_token: Some(DJ_V3_TOKEN.to_string()),
            ..RemoteControlConfig::default()
        };
        let start_listener = |port, process_fence, handler: DjLinkDispatchHandler| {
            RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link_with_process_fence(
                config(port),
                |_| {},
                || Some(EngineSnapshot::default()),
                (
                    VideoRuntimeStatus::default,
                    default_video_output_render_plans,
                    default_external_video_io_plans,
                    default_external_video_transport_status,
                    default_external_video_transport_sync,
                ),
                Some(handler),
                Some(process_fence),
            )
            .unwrap()
        };
        let read_ack = |client: &mut tungstenite::WebSocket<_>| -> DjLinkAck {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };
        let physical = |session_id: &str, sequence: u64| {
            json!({
                "v": 3,
                "type": "DJ_TIMELINE_LOOP_SET",
                "agentId": DJ_V3_AGENT,
                "sessionId": session_id,
                "sequence": sequence,
                "eventId": "restart-socket-event",
                "payload": {
                    "active": true,
                    "timelineId": "7",
                    "playSessionId": "play-loop",
                },
            })
        };
        let open_and_gate = |server: &RemoteWsServer, session: &str| {
            let url = format!("ws://127.0.0.1:{port}/dj-link");
            let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();
            complete_dj_v3_snapshot_gate(&mut client, port, session, server);
            client
        };

        let first = start_listener(port, Arc::clone(&process_fence), Arc::clone(&handler));
        let mut first_client = open_and_gate(&first, "first-session");
        first_client
            .send(Message::Text(physical("first-session", 4).to_string()))
            .unwrap();
        assert_eq!(
            read_ack(&mut first_client).outcome,
            DjLinkAckOutcome::Accepted
        );
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);
        let _ = first_client.close(None);
        drop(first_client);
        drop(first);

        // Replacing the actual listener on the same port gets a fresh
        // session/terminal registry but the process-owned fence still rejects
        // the already dispatched physical identity.
        let replacement = start_listener(port, Arc::clone(&process_fence), Arc::clone(&handler));
        let mut replacement_client = open_and_gate(&replacement, "replacement-session");
        replacement_client
            .send(Message::Text(
                physical("replacement-session", 4).to_string(),
            ))
            .unwrap();
        let replay_ack = read_ack(&mut replacement_client);
        assert_eq!(replay_ack.outcome, DjLinkAckOutcome::Rejected);
        assert_eq!(replay_ack.code.as_deref(), Some("event_id_not_retained"));
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);
        let _ = replacement_client.close(None);
        drop(replacement_client);
        drop(replacement);

        // A genuinely new owner/process supplies a fresh fence and may admit
        // the same wire identity independently.
        let fresh_owner = start_listener(port, new_dj_link_process_fence(), Arc::clone(&handler));
        let mut fresh_client = open_and_gate(&fresh_owner, "fresh-owner-session");
        fresh_client
            .send(Message::Text(
                physical("fresh-owner-session", 4).to_string(),
            ))
            .unwrap();
        assert_eq!(
            read_ack(&mut fresh_client).outcome,
            DjLinkAckOutcome::Accepted
        );
        assert_eq!(dispatches.load(Ordering::SeqCst), 2);
        let _ = fresh_client.close(None);
        drop(fresh_client);
        drop(fresh_owner);
    }

    #[test]
    fn dj_link_socket_capacity_latch_survives_reconnect_and_listener_restart() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let process_fence = new_dj_link_process_fence();
        process_fence.lock().unwrap().side_effect_event_limit = 2;
        let dispatches = Arc::new(AtomicU64::new(0));
        let handler: DjLinkDispatchHandler = {
            let dispatches = Arc::clone(&dispatches);
            let inner = dj_v3_state_responder(31);
            Arc::new(move |envelope| {
                if !matches!(
                    envelope.message_type,
                    DjLinkMessageType::StateSync | DjLinkMessageType::TimelineStateRequest
                ) {
                    dispatches.fetch_add(1, Ordering::SeqCst);
                }
                inner(envelope)
            })
        };
        let config = |port| RemoteControlConfig {
            bind_ip: "127.0.0.1".to_string(),
            port,
            pairing_pin: "123456".to_string(),
            max_connections: 1,
            max_messages_per_second: 1_000,
            dj_link_enabled: true,
            dj_link_bind_ip: Some("127.0.0.1".to_string()),
            dj_link_token: Some(DJ_V3_TOKEN.to_string()),
            ..RemoteControlConfig::default()
        };
        let start_listener = |port, process_fence, handler: DjLinkDispatchHandler| {
            RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link_with_process_fence(
                config(port),
                |_| {},
                || Some(EngineSnapshot::default()),
                (
                    VideoRuntimeStatus::default,
                    default_video_output_render_plans,
                    default_external_video_io_plans,
                    default_external_video_transport_status,
                    default_external_video_transport_sync,
                ),
                Some(handler),
                Some(process_fence),
            )
            .unwrap()
        };
        let read_ack = |client: &mut tungstenite::WebSocket<_>| -> DjLinkAck {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };
        let physical = |session_id: &str, sequence: u64, event_id: &str| {
            json!({
                "v": 3,
                "type": "DJ_TIMELINE_LOOP_SET",
                "agentId": DJ_V3_AGENT,
                "sessionId": session_id,
                "sequence": sequence,
                "eventId": event_id,
                "payload": {
                    "active": true,
                    "timelineId": "7",
                    "playSessionId": "play-loop",
                },
            })
        };
        let url = format!("ws://127.0.0.1:{port}/dj-link");

        // Every socket passes the exact-v3 snapshot/order handshake before
        // physical commands, and physical identities use the shared
        // process-fence reservation path. The first two identities are
        // admitted and the third trips the small injected high-water.
        let first = start_listener(port, Arc::clone(&process_fence), Arc::clone(&handler));
        let (mut first_client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(&mut first_client, port, "capacity-first-session", &first);
        for (sequence, event_id) in [(4, "capacity-first"), (5, "capacity-second")] {
            first_client
                .send(Message::Text(
                    physical("capacity-first-session", sequence, event_id).to_string(),
                ))
                .unwrap();
            assert_eq!(
                read_ack(&mut first_client).outcome,
                DjLinkAckOutcome::Accepted
            );
        }
        assert_eq!(dispatches.load(Ordering::SeqCst), 2);
        first_client
            .send(Message::Text(
                physical("capacity-first-session", 6, "capacity-third").to_string(),
            ))
            .unwrap();
        let first_latched = read_ack(&mut first_client);
        assert_eq!(first_latched.outcome, DjLinkAckOutcome::Rejected);
        assert_eq!(
            first_latched.code.as_deref(),
            Some("side_effect_id_capacity_latched")
        );
        // Repeating the same rejected frame remains latched and cannot turn
        // into an inflight reservation on the same connection.
        first_client
            .send(Message::Text(
                physical("capacity-first-session", 6, "capacity-third").to_string(),
            ))
            .unwrap();
        let repeated_latched = read_ack(&mut first_client);
        assert_eq!(repeated_latched.outcome, DjLinkAckOutcome::Rejected);
        assert_eq!(
            repeated_latched.code.as_deref(),
            Some("side_effect_id_capacity_latched")
        );
        {
            let registry = first.dj_link_registry.lock().unwrap();
            assert_eq!(registry.seen_event_count(), 2);
            assert!(registry.inflight.is_empty());
            assert!(registry.side_effect_capacity_latched());
            assert!(registry.terminal_count() <= DJ_LINK_TERMINAL_LIMIT);
        }
        let _ = first_client.close(None);
        drop(first_client);
        drop(first);

        // A reconnect and an actual listener replacement share the same
        // process fence, so the latch survives both instance-local resets.
        let replacement = start_listener(port, Arc::clone(&process_fence), Arc::clone(&handler));
        let (mut replacement_client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(
            &mut replacement_client,
            port,
            "capacity-replacement-session",
            &replacement,
        );
        replacement_client
            .send(Message::Text(
                physical("capacity-replacement-session", 4, "capacity-third").to_string(),
            ))
            .unwrap();
        let replacement_latched = read_ack(&mut replacement_client);
        assert_eq!(replacement_latched.outcome, DjLinkAckOutcome::Rejected);
        assert_eq!(
            replacement_latched.code.as_deref(),
            Some("side_effect_id_capacity_latched")
        );
        assert_eq!(dispatches.load(Ordering::SeqCst), 2);
        assert_eq!(process_fence.lock().unwrap().seen_events.len(), 2);
        assert!(process_fence.lock().unwrap().side_effect_capacity_latched);
        let _ = replacement_client.close(None);
        drop(replacement_client);
        drop(replacement);

        // A fresh process owner gets a fresh latch and can admit the same
        // physical identity, with a separately bounded tombstone set.
        let fresh_fence = new_dj_link_process_fence();
        fresh_fence.lock().unwrap().side_effect_event_limit = 2;
        let fresh_owner = start_listener(port, Arc::clone(&fresh_fence), Arc::clone(&handler));
        let (mut fresh_client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(
            &mut fresh_client,
            port,
            "capacity-fresh-session",
            &fresh_owner,
        );
        fresh_client
            .send(Message::Text(
                physical("capacity-fresh-session", 4, "capacity-third").to_string(),
            ))
            .unwrap();
        assert_eq!(
            read_ack(&mut fresh_client).outcome,
            DjLinkAckOutcome::Accepted
        );
        assert_eq!(dispatches.load(Ordering::SeqCst), 3);
        {
            let registry = fresh_owner.dj_link_registry.lock().unwrap();
            assert_eq!(registry.seen_event_count(), 1);
            assert!(!registry.side_effect_capacity_latched());
            assert!(registry.terminal_count() <= DJ_LINK_TERMINAL_LIMIT);
        }
        let _ = fresh_client.close(None);
        drop(fresh_client);
        drop(fresh_owner);
    }

    #[test]
    fn dj_link_socket_state_sync_never_fences_while_loop_commands_do() {
        let process_fence = new_dj_link_process_fence();
        process_fence.lock().unwrap().side_effect_event_limit = 1;
        let loop_dispatches = Arc::new(AtomicU64::new(0));
        let handler: DjLinkDispatchHandler = {
            let loop_dispatches = Arc::clone(&loop_dispatches);
            let inner = dj_v3_state_responder(73);
            Arc::new(move |envelope| {
                if envelope.message_type == DjLinkMessageType::LoopState {
                    loop_dispatches.fetch_add(1, Ordering::SeqCst);
                }
                inner(envelope)
            })
        };
        let config = RemoteControlConfig {
            bind_ip: "127.0.0.1".to_string(),
            // Test builds pass the OS-selected port zero listener through
            // to the server, eliminating the probe/drop/rebind TOCTOU.
            port: 0,
            pairing_pin: "123456".to_string(),
            max_connections: 1,
            max_messages_per_second: 1_000,
            dj_link_enabled: true,
            dj_link_bind_ip: Some("127.0.0.1".to_string()),
            dj_link_token: Some(DJ_V3_TOKEN.to_string()),
            ..RemoteControlConfig::default()
        };
        let server = RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link_with_process_fence(
            config,
            |_| {},
            || Some(EngineSnapshot::default()),
            (
                VideoRuntimeStatus::default,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
            Some(handler),
            Some(Arc::clone(&process_fence)),
        )
        .unwrap();
        let port = server.local_addr().port();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(&mut client, port, "quadrant-session", &server);
        let read_value = |client: &mut tungstenite::WebSocket<_>| -> Value {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };
        let send_frame = |client: &mut tungstenite::WebSocket<_>, frame: Value| {
            client.send(Message::Text(frame.to_string())).unwrap();
        };
        let state_sync = |sequence: u64, event_id: &str, released: bool, owner_deck: Option<u8>| {
            let payload = if released {
                json!({ "released": true })
            } else if let Some(owner_deck) = owner_deck {
                json!({
                    "released": false,
                    "ownerDeck": owner_deck,
                    "ownerDeckId": format!("rekordbox-deck-{owner_deck}"),
                    "activePlaySessionId": "play-1",
                })
            } else {
                json!({ "released": false })
            };
            json!({
                "v": 3,
                "type": "DJ_STATE_SYNC",
                "agentId": DJ_V3_AGENT,
                "sessionId": "quadrant-session",
                "sequence": sequence,
                "eventId": event_id,
                "payload": payload,
            })
        };

        // Every STATE_SYNC quadrant is nonphysical status traffic and remains
        // admissible even after the physical high-water latch below.
        for (sequence, event_id, released, deck) in [
            (4, "quadrant-observed", false, Some(1)),
            (5, "quadrant-released", true, None),
            (6, "quadrant-other-owner", false, Some(2)),
        ] {
            send_frame(&mut client, state_sync(sequence, event_id, released, deck));
            assert_eq!(read_value(&mut client)["outcome"], "accepted");
        }

        // The measured external LOOP command is physical: it consumes the
        // one injected process slot.
        send_frame(
            &mut client,
            json!({
                "v": 3,
                "type": "DJ_LOOP_STATE",
                "agentId": DJ_V3_AGENT,
                "sessionId": "quadrant-session",
                "sequence": 7,
                "eventId": "quadrant-physical",
                "payload": {
                    "deck": 1,
                    "deckId": "rekordbox-deck-1",
                    "playSessionId": "play-loop",
                    "loop": {
                        "active": true,
                        "startBeat": 16.0,
                        "endBeat": 32.0,
                        "lengthBeats": 16.0,
                        "revision": 1,
                        "sampleAgeMs": 12,
                        "source": "rekordbox-hook-measured",
                    },
                },
            }),
        );
        assert_eq!(read_value(&mut client)["outcome"], "accepted");
        assert_eq!(loop_dispatches.load(Ordering::SeqCst), 1);

        // A second physical command trips the latch. It is not terminalized,
        // so repeating the exact frame cannot turn it into an inflight
        // reservation or grow the permanent fence.
        let over_limit = json!({
            "v": 3,
            "type": "DJ_TIMELINE_LOOP_SET",
            "agentId": DJ_V3_AGENT,
            "sessionId": "quadrant-session",
            "sequence": 8,
            "eventId": "quadrant-over-limit",
            "payload": {
                "active": true,
                "timelineId": "7",
                "playSessionId": "play-quadrant-session",
            },
        });
        send_frame(&mut client, over_limit.clone());
        let first_reject = read_value(&mut client);
        assert_eq!(first_reject["outcome"], "rejected");
        assert_eq!(first_reject["code"], "side_effect_id_capacity_latched");
        send_frame(&mut client, over_limit);
        let repeated_reject = read_value(&mut client);
        assert_eq!(repeated_reject["outcome"], "rejected");
        assert_eq!(repeated_reject["code"], "side_effect_id_capacity_latched");

        // Status traffic keeps flowing after the latch.
        send_frame(
            &mut client,
            state_sync(9, "quadrant-after-latch", false, None),
        );
        assert_eq!(read_value(&mut client)["outcome"], "accepted");
        assert_eq!(loop_dispatches.load(Ordering::SeqCst), 1);
        {
            let registry = server.dj_link_registry.lock().unwrap();
            assert_eq!(registry.seen_event_count(), 1);
            assert!(registry.side_effect_capacity_latched());
            assert!(registry.inflight.is_empty());
            assert!(registry.terminal_count() <= DJ_LINK_TERMINAL_LIMIT);
        }
        let _ = client.close(None);
        drop(client);
        drop(server);
    }

    #[test]
    fn dj_link_stop_fences_inflight_before_join_and_replacement_rejects_replay() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let process_fence = new_dj_link_process_fence();
        let entered = mpsc::sync_channel(1);
        let (entered_tx, entered_rx) = entered;
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let release_rx = Arc::new(Mutex::new(release_rx));
        let block_first_dispatch = Arc::new(AtomicBool::new(true));
        let dispatches = Arc::new(AtomicU64::new(0));
        let handler: DjLinkDispatchHandler = {
            let block_first_dispatch = Arc::clone(&block_first_dispatch);
            let dispatches = Arc::clone(&dispatches);
            let release_rx = Arc::clone(&release_rx);
            let inner = dj_v3_state_responder(41);
            Arc::new(move |envelope| {
                if envelope.message_type == DjLinkMessageType::TimelineLoopSet
                    && block_first_dispatch.swap(false, Ordering::SeqCst)
                {
                    let _ = entered_tx.send(());
                    let _ = release_rx.lock().unwrap().recv();
                }
                // The gate's STATE_SYNC/STATE_REQUEST traffic shares this
                // handler but is idempotent status traffic; only side-effectful
                // executions may move the physical dispatch counter the
                // fencing assertions below depend on.
                if !matches!(
                    envelope.message_type,
                    DjLinkMessageType::StateSync | DjLinkMessageType::TimelineStateRequest
                ) {
                    dispatches.fetch_add(1, Ordering::SeqCst);
                }
                inner(envelope)
            })
        };
        let config = |port| RemoteControlConfig {
            bind_ip: "127.0.0.1".to_string(),
            port,
            pairing_pin: "123456".to_string(),
            max_connections: 1,
            max_messages_per_second: 1_000,
            dj_link_enabled: true,
            dj_link_bind_ip: Some("127.0.0.1".to_string()),
            dj_link_token: Some(DJ_V3_TOKEN.to_string()),
            ..RemoteControlConfig::default()
        };
        let start_listener = |port, process_fence, handler: DjLinkDispatchHandler| {
            RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link_with_process_fence(
                config(port),
                |_| {},
                || Some(EngineSnapshot::default()),
                (
                    VideoRuntimeStatus::default,
                    default_video_output_render_plans,
                    default_external_video_io_plans,
                    default_external_video_transport_status,
                    default_external_video_transport_sync,
                ),
                Some(handler),
                Some(process_fence),
            )
            .unwrap()
        };
        let read_ack = |client: &mut tungstenite::WebSocket<_>| -> DjLinkAck {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };
        let physical = |session_id: &str| {
            json!({
                "v": 3,
                "type": "DJ_TIMELINE_LOOP_SET",
                "agentId": DJ_V3_AGENT,
                "sessionId": session_id,
                "sequence": 4,
                "eventId": "shutdown-physical-event",
                "payload": {
                    "active": true,
                    "timelineId": "7",
                    "playSessionId": "play-shutdown-session",
                },
            })
        };
        let url = format!("ws://127.0.0.1:{port}/dj-link");

        let first = start_listener(port, Arc::clone(&process_fence), Arc::clone(&handler));
        let (mut first_client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(&mut first_client, port, "shutdown-session", &first);
        first_client
            .send(Message::Text(physical("shutdown-session").to_string()))
            .unwrap();
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("physical handler did not enter its bounded wait");
        assert_eq!(dispatches.load(Ordering::SeqCst), 0);

        // stop_and_join is intentionally run concurrently with the bounded
        // handler.  It must publish the physical identity before joining the
        // worker, so replacement admission is safe even before the handler's
        // Engine-timeout-equivalent wait is released.
        let stop_thread = thread::spawn(move || {
            let mut first = first;
            let started = Instant::now();
            first.stop_and_join();
            let elapsed = started.elapsed();
            (first, elapsed)
        });
        for _ in 0..200 {
            if process_fence.lock().unwrap().seen_events.len() == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(process_fence.lock().unwrap().seen_events.len(), 1);
        assert_eq!(dispatches.load(Ordering::SeqCst), 0);
        release_tx
            .send(())
            .expect("in-flight handler release channel closed unexpectedly");
        let (first, elapsed) = stop_thread.join().expect("listener stop thread panicked");
        assert!(
            elapsed < DJ_LINK_SHUTDOWN_DEADLINE,
            "bounded in-flight stop took {elapsed:?}"
        );
        assert!(first.thread.is_none(), "listener JoinHandle remained live");
        assert!(
            first.client_workers.lock().unwrap().is_empty(),
            "accepted client JoinHandle remained after stop"
        );
        assert!(
            first.shutdown_sockets.lock().unwrap().is_empty(),
            "accepted socket remained registered after stop"
        );
        assert!(
            first.clients.lock().unwrap().is_empty(),
            "generic remote client registry remained populated after stop"
        );
        assert_eq!(first.dj_link_connections.load(Ordering::Acquire), 0);
        assert_eq!(first.dj_link_inflight_dispatch_count(), 0);
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);
        let _ = first_client.close(None);
        drop(first_client);
        drop(first);

        // The replacement has a new session/terminal registry but shares the
        // process fence.  The same physical event is rejected and can never
        // invoke the handler a second time.
        let replacement = start_listener(port, Arc::clone(&process_fence), Arc::clone(&handler));
        let (mut replacement_client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(
            &mut replacement_client,
            port,
            "replacement-shutdown-session",
            &replacement,
        );
        replacement_client
            .send(Message::Text(
                physical("replacement-shutdown-session").to_string(),
            ))
            .unwrap();
        let replay = read_ack(&mut replacement_client);
        assert_eq!(replay.outcome, DjLinkAckOutcome::Rejected);
        assert_eq!(replay.code.as_deref(), Some("event_id_not_retained"));
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);
        let _ = replacement_client.close(None);
        drop(replacement_client);
        drop(replacement);
    }

    #[test]
    fn dj_link_rate_limited_unique_physical_frame_is_retryable_without_capacity_use() {
        let now = Instant::now();
        let mut registry = DjLinkRegistry::with_side_effect_event_limit(2);
        let generation =
            registry.register_test_session(DJ_V3_AGENT, "rate-session", 1, "rate-peer", now);
        registry
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;
        let physical = |sequence: u64, event_id: &str| {
            dj_v3_envelope(
                DjLinkMessageType::TimelineLoopSet,
                "rate-session",
                sequence,
                event_id,
                dj_v3_loop_set_payload(true),
            )
        };

        let first = physical(2, "rate-first");
        let first_shape = first.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&first, &first_shape, generation, now)
                .unwrap(),
            DjLinkAdmission::Accepted
        );
        registry
            .complete(
                &first,
                generation,
                first_shape,
                dj_link_ack(&first, DjLinkAckOutcome::Accepted, None, generation),
                now,
            )
            .unwrap();
        assert_eq!(registry.seen_event_count(), 1);

        // A unique physical flood remains retryable and cannot consume the
        // remaining process-fence capacity or trip its fail-closed latch.
        for sequence in 3..=128 {
            let flooded = physical(sequence, &format!("rate-flood-{sequence}"));
            let flooded_shape = flooded.canonical_shape().unwrap();
            assert_eq!(
                registry
                    .admit_with_rate_limit(&flooded, &flooded_shape, generation, now, true)
                    .unwrap(),
                DjLinkAdmission::RateLimited
            );
        }
        assert_eq!(registry.seen_event_count(), 1);
        assert!(!registry.side_effect_capacity_latched());
        assert!(registry.inflight.is_empty());

        // The same unique frame retries after throttling and is admitted;
        // the rejected attempt did not advance the sequence floor or consume
        // a permanent physical identity slot.
        let retryable = physical(3, "rate-retryable");
        let retryable_shape = retryable.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit_with_rate_limit(&retryable, &retryable_shape, generation, now, false)
                .unwrap(),
            DjLinkAdmission::Accepted
        );
        assert_eq!(registry.seen_event_count(), 1);
        registry.abort_inflight(&retryable, generation);
        assert_eq!(registry.seen_event_count(), 1);
    }

    #[test]
    fn dj_link_loopback_hello_payload_auth_and_terminal_ack() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let server = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
        )
        .unwrap();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut wrong_client, _) = tungstenite::connect(url.as_str()).unwrap();
        let wrong_hello = serde_json::json!({
            "v": 3,
            "type": "DJ_AGENT_HELLO",
            "agentId": DJ_V3_AGENT,
            "sessionId": "wrong-session",
            "sequence": 1,
            "eventId": "wrong-hello",
            "payload": {"authToken": "0123456789abcdef0123456789abcdee", "version": 3, "capabilities": dj_v3_capabilities()}
        });
        wrong_client
            .send(Message::Text(wrong_hello.to_string()))
            .unwrap();
        let wrong_ack: DjLinkAck = match wrong_client.read().unwrap() {
            Message::Text(text) => serde_json::from_str(&text).unwrap(),
            other => panic!("unexpected wrong-token DJ Link reply: {other:?}"),
        };
        assert_eq!(wrong_ack.event_id, "wrong-hello");
        assert_eq!(wrong_ack.outcome, DjLinkAckOutcome::Rejected);
        let _ = wrong_client.close(None);

        let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();
        client
            .send(Message::Text(
                dj_v3_hello("session-loopback", "hello-loopback").to_string(),
            ))
            .unwrap();
        let hello_ack: DjLinkAck = match client.read().unwrap() {
            Message::Text(text) => serde_json::from_str(&text).unwrap(),
            other => panic!("unexpected DJ Link HELLO reply: {other:?}"),
        };
        assert_eq!(hello_ack.event_id, "hello-loopback");
        assert_eq!(hello_ack.sequence, 1);
        assert_eq!(hello_ack.outcome, DjLinkAckOutcome::Accepted);
        assert_eq!(
            server.dj_link_status().agent_id.as_deref(),
            Some(DJ_V3_AGENT)
        );
        let status = server.dj_link_status();
        assert!(status.connected);
        assert!(status
            .peer
            .as_deref()
            .is_some_and(|peer| peer.starts_with("127.0.0.1:")));

        let heartbeat = |sequence: u64, event_id: &str| {
            serde_json::json!({
                "v": 3,
                "type": "DJ_HEARTBEAT",
                "agentId": DJ_V3_AGENT,
                "sessionId": "session-loopback",
                "sequence": sequence,
                "eventId": event_id,
                "payload": {}
            })
        };
        client
            .send(Message::Text(
                heartbeat(2, "heartbeat-loopback").to_string(),
            ))
            .unwrap();
        let heartbeat_ack: DjLinkAck = match client.read().unwrap() {
            Message::Text(text) => serde_json::from_str(&text).unwrap(),
            other => panic!("unexpected DJ Link heartbeat reply: {other:?}"),
        };
        assert_eq!(heartbeat_ack.event_id, "heartbeat-loopback");
        assert_eq!(heartbeat_ack.sequence, 2);
        assert_eq!(heartbeat_ack.outcome, DjLinkAckOutcome::Accepted);
        client
            .send(Message::Text(
                heartbeat(2, "heartbeat-loopback").to_string(),
            ))
            .unwrap();
        let duplicate_ack: DjLinkAck = match client.read().unwrap() {
            Message::Text(text) => serde_json::from_str(&text).unwrap(),
            other => panic!("unexpected duplicate DJ Link reply: {other:?}"),
        };
        assert_eq!(duplicate_ack.outcome, DjLinkAckOutcome::Duplicate);
        // A changed shape under a retained identity is a conflict, and an
        // out-of-order sequence is rejected as rollback.
        let conflict = serde_json::json!({
            "v": 3,
            "type": "DJ_HEARTBEAT",
            "agentId": DJ_V3_AGENT,
            "sessionId": "session-loopback",
            "sequence": 3,
            "eventId": "heartbeat-loopback",
            "payload": {}
        });
        client.send(Message::Text(conflict.to_string())).unwrap();
        let conflict_ack: DjLinkAck = match client.read().unwrap() {
            Message::Text(text) => serde_json::from_str(&text).unwrap(),
            other => panic!("unexpected conflict DJ Link reply: {other:?}"),
        };
        assert_eq!(conflict_ack.outcome, DjLinkAckOutcome::Rejected);
        let rollback = heartbeat(1, "heartbeat-rollback");
        client.send(Message::Text(rollback.to_string())).unwrap();
        let rollback_ack: DjLinkAck = match client.read().unwrap() {
            Message::Text(text) => serde_json::from_str(&text).unwrap(),
            other => panic!("unexpected rollback DJ Link reply: {other:?}"),
        };
        assert_eq!(rollback_ack.outcome, DjLinkAckOutcome::Rejected);
        let _ = client.close(None);
        drop(server);
    }

    #[test]
    fn dj_link_ack_wire_serializes_the_exact_v3_shape() {
        let envelope = dj_v3_envelope(
            DjLinkMessageType::Heartbeat,
            "ack-session",
            9,
            "ack-event",
            json!({}),
        );
        let busy = dj_link_ack(
            &envelope,
            DjLinkAckOutcome::Busy,
            Some("in_flight".to_string()),
            17,
        );
        let busy_wire: Value =
            serde_json::from_str(&dj_link_ack_wire(&busy).expect("busy ACK must render")).unwrap();
        assert_eq!(
            busy_wire,
            json!({
                "v": protocol::DJ_LINK_PROTOCOL_VERSION,
                "type": "ACK",
                "eventId": "ack-event",
                "sequence": 9,
                "outcome": "busy",
                "code": "in_flight",
                "stateGeneration": 17,
            })
        );
        let accepted = dj_link_ack(&envelope, DjLinkAckOutcome::Accepted, None, 18);
        let accepted_wire: Value =
            serde_json::from_str(&dj_link_ack_wire(&accepted).expect("accepted ACK must render"))
                .unwrap();
        assert_eq!(
            accepted_wire,
            json!({
                "v": protocol::DJ_LINK_PROTOCOL_VERSION,
                "type": "ACK",
                "eventId": "ack-event",
                "sequence": 9,
                "outcome": "accepted",
                "code": null,
                "stateGeneration": 18,
            })
        );
    }

    /// Dedicated listener config for the hostile ACK-wire fixtures: exact v3
    /// token and a generous connection budget so each phase owns its socket.
    fn dj_link_ack_wire_hostile_config(port: u16) -> RemoteControlConfig {
        RemoteControlConfig {
            bind_ip: "127.0.0.1".to_string(),
            port,
            pairing_pin: "123456".to_string(),
            max_connections: 8,
            max_messages_per_second: 1_000,
            dj_link_enabled: true,
            dj_link_bind_ip: Some("127.0.0.1".to_string()),
            dj_link_token: Some(DJ_V3_TOKEN.to_string()),
            ..RemoteControlConfig::default()
        }
    }

    fn start_dj_link_ack_wire_hostile_server(
        port: u16,
        handler: Option<DjLinkDispatchHandler>,
    ) -> RemoteWsServer {
        RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link_with_process_fence(
            dj_link_ack_wire_hostile_config(port),
            |_| {},
            || Some(EngineSnapshot::default()),
            (
                VideoRuntimeStatus::default,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
            handler,
            Some(new_dj_link_process_fence()),
        )
        .unwrap()
    }

    /// Hostile-fixture client with a bounded read timeout so a regression
    /// that leaves the connection open fails loudly instead of hanging.
    fn connect_dj_link_hostile_client(url: &str) -> tungstenite::WebSocket<TcpStream> {
        let authority = url
            .strip_prefix("ws://")
            .expect("hostile fixtures use plain ws:// URLs")
            .split('/')
            .next()
            .expect("hostile fixtures carry an authority");
        let stream = TcpStream::connect(authority).expect("hostile client must connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("hostile client read timeout must apply");
        let (client, _response) =
            tungstenite::client(url, stream).expect("hostile client handshake must succeed");
        client
    }

    /// Asserts the server terminates this connection without ever placing a
    /// fabricated text frame — in particular no `{}` fallback ACK — on the
    /// wire. Termination is observed as a Close frame or a transport error;
    /// a silent open socket is itself a failure.
    fn assert_dj_link_connection_terminates_without_text<S>(client: &mut tungstenite::WebSocket<S>)
    where
        S: Read + Write,
    {
        loop {
            match client.read() {
                Ok(Message::Text(text)) => {
                    panic!("fabricated frame reached the wire during fail-close: {text:?}")
                }
                Ok(Message::Close(_)) => return,
                Ok(Message::Ping(_) | Message::Pong(_)) => continue,
                Ok(other) => panic!("unexpected frame during fail-close: {other:?}"),
                Err(tungstenite::Error::Io(error))
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.kind() == std::io::ErrorKind::TimedOut =>
                {
                    panic!("connection stayed open with no frame during fail-close")
                }
                Err(_) => return,
            }
        }
    }

    #[test]
    fn dj_link_ack_serialization_failure_emits_zero_bytes_and_terminates() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let server = start_dj_link_ack_wire_hostile_server(port, None);
        let url = format!("ws://127.0.0.1:{port}/dj-link");

        // Phase 1 — pre-HELLO rejection: the correlation reply cannot be
        // rendered, so nothing reaches the wire and the connection ends.
        {
            let _failure = install_dj_link_ack_serialization_failure_hook(Arc::new(|| {}));
            let mut client = connect_dj_link_hostile_client(&url);
            client
                .send(Message::Text(
                    json!({
                        "v": 3,
                        "type": "DJ_HEARTBEAT",
                        "agentId": DJ_V3_AGENT,
                        "sessionId": "ser-phase1",
                        "sequence": 1,
                        "eventId": "pre-hello-heartbeat",
                        "payload": {},
                    })
                    .to_string(),
                ))
                .unwrap();
            assert_dj_link_connection_terminates_without_text(&mut client);
        }

        // Phase 2 — accepted HELLO: an unrenderable HELLO ACK terminates
        // before any command loop starts and publishes no bytes.
        {
            let _failure = install_dj_link_ack_serialization_failure_hook(Arc::new(|| {}));
            let mut client = connect_dj_link_hostile_client(&url);
            client
                .send(Message::Text(
                    dj_v3_hello("ser-phase2", "hello-ser-phase2").to_string(),
                ))
                .unwrap();
            assert_dj_link_connection_terminates_without_text(&mut client);
        }

        // The service stays healthy with the seam disarmed, and the success
        // path still emits the exact v3 ACK shape with no `{}` anywhere.
        let mut healthy = connect_dj_link_hostile_client(&url);
        healthy
            .send(Message::Text(
                dj_v3_hello("ser-healthy", "hello-ser-healthy").to_string(),
            ))
            .unwrap();
        let raw_hello_ack = match healthy.read().unwrap() {
            Message::Text(text) => text,
            other => panic!("expected the hello ACK text frame, got {other:?}"),
        };
        assert!(!raw_hello_ack.contains("{}"));
        assert!(raw_hello_ack.starts_with("{\"v\":3"));
        let ack: DjLinkAck = serde_json::from_str(&raw_hello_ack).unwrap();
        assert_eq!(ack.outcome, DjLinkAckOutcome::Accepted);
        assert_eq!(ack.v, protocol::DJ_LINK_PROTOCOL_VERSION);
        assert_eq!(ack.event_id, "hello-ser-healthy");
        let _ = healthy.close(None);
        drop(server);
    }

    #[test]
    fn dj_link_ack_send_failure_terminates_and_replay_returns_exact_receipt() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let server = start_dj_link_ack_wire_hostile_server(port, None);
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let mut client = connect_dj_link_hostile_client(&url);

        // The HELLO completes terminalization, then the delivery is forced to
        // fail: zero data bytes may reach the wire and the socket must die.
        {
            let _failure = install_dj_link_ack_transport_failure_hook(Arc::new(|| {}));
            client
                .send(Message::Text(
                    dj_v3_hello("txfr-session", "hello-txfr").to_string(),
                ))
                .unwrap();
            assert_dj_link_connection_terminates_without_text(&mut client);
        }

        // The retained receipt turns the reconnect replay into the exact
        // Duplicate ACK instead of re-running admission or execution.
        let mut replay = connect_dj_link_hostile_client(&url);
        replay
            .send(Message::Text(
                dj_v3_hello("txfr-session", "hello-txfr").to_string(),
            ))
            .unwrap();
        let replay_ack = read_dj_ack(&mut replay);
        assert_eq!(replay_ack.outcome, DjLinkAckOutcome::Duplicate);
        assert_eq!(replay_ack.event_id, "hello-txfr");
        assert_eq!(replay_ack.code, None);
        assert!(replay_ack.state_generation > 0);
        let _ = replay.close(None);
        drop(server);
    }

    #[test]
    fn dj_link_physical_ack_serialization_failure_never_reexecutes_the_event() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let dispatches = Arc::new(AtomicU64::new(0));
        let inner = dj_v3_state_responder(21);
        let handler: DjLinkDispatchHandler = {
            let dispatches = Arc::clone(&dispatches);
            Arc::new(move |envelope| {
                if !matches!(
                    envelope.message_type,
                    DjLinkMessageType::StateSync | DjLinkMessageType::TimelineStateRequest
                ) {
                    dispatches.fetch_add(1, Ordering::SeqCst);
                }
                inner(envelope)
            })
        };
        let server = start_dj_link_ack_wire_hostile_server(port, Some(handler));
        let url = format!("ws://127.0.0.1:{port}/dj-link");

        // Pass the full snapshot/order gate so the physical command is
        // admitted normally, with every gate ACK rendering successfully.
        let mut first = connect_dj_link_hostile_client(&url);
        complete_dj_v3_snapshot_gate(&mut first, port, "phys-ack-session", &server);

        // The physical command dispatches exactly once, but its final ACK can
        // no longer be rendered: no bytes may reach the wire, no state or
        // liveness bookkeeping beyond the completed receipt may run, and the
        // connection must terminate.
        {
            let _failure = install_dj_link_ack_serialization_failure_hook(Arc::new(|| {}));
            first
                .send(Message::Text(
                    json!({
                        "v": 3,
                        "type": "DJ_TIMELINE_LOOP_SET",
                        "agentId": DJ_V3_AGENT,
                        "sessionId": "phys-ack-session",
                        "sequence": 4,
                        "eventId": "phys-ack-event",
                        "payload": dj_v3_loop_set_payload(true),
                    })
                    .to_string(),
                ))
                .unwrap();
            assert_dj_link_connection_terminates_without_text(&mut first);
        }
        drop(first);
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);

        // A reconnecting agent that replays the same identity cannot launder
        // it into a second execution: the completed receipt makes the retry a
        // conflict, and the handler must not run again.
        let mut replay = connect_dj_link_hostile_client(&url);
        complete_dj_v3_snapshot_gate(&mut replay, port, "phys-ack-session-b", &server);
        replay
            .send(Message::Text(
                json!({
                    "v": 3,
                    "type": "DJ_TIMELINE_LOOP_SET",
                    "agentId": DJ_V3_AGENT,
                    "sessionId": "phys-ack-session-b",
                    "sequence": 4,
                    "eventId": "phys-ack-event",
                    "payload": dj_v3_loop_set_payload(true),
                })
                .to_string(),
            ))
            .unwrap();
        let conflict_ack = read_dj_ack(&mut replay);
        assert_eq!(conflict_ack.outcome, DjLinkAckOutcome::Rejected);
        assert_eq!(conflict_ack.code.as_deref(), Some("event_id_conflict"));
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);
        let _ = replay.close(None);
        drop(server);
    }

    #[test]
    fn dj_link_invalid_frames_receive_no_reply_and_keep_the_socket_usable() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let server = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
        )
        .unwrap();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();

        // Pre-hello: wrong identity, retired v2, future v4, duplicate keys,
        // and unknown message shapes all fail exact-v3 parsing. The transport
        // cannot correlate them and stays silent rather than inventing an ACK.
        // The following valid HELLO still completes.
        for uncorrelatable in [
            r#"{"v":3,"type":"DJ_AGENT_HELLO","agentId":"other-agent","sessionId":"s","sequence":1,"eventId":"e","payload":{}}"#,
            r#"{"v":2,"type":"DJ_AGENT_HELLO","agentId":"rb-output-dj-agent","sessionId":"s","sequence":1,"eventId":"e","payload":{}}"#,
            r#"{"v":4,"type":"DJ_AGENT_HELLO","agentId":"rb-output-dj-agent","sessionId":"s","sequence":1,"eventId":"e","payload":{}}"#,
            r#"{"v":3,"type":"DJ_HEARTBEAT","agentId":"rb-output-dj-agent","sessionId":"s","sequence":1,"sequence":2,"eventId":"e","payload":{}}"#,
            r#"{"type":"DJ_UNKNOWN","eventId":"unknown-event","sequence":7}"#,
        ] {
            assert!(
                DjLinkEnvelope::parse_json(uncorrelatable).is_err(),
                "fixture must be unparseable: {uncorrelatable}"
            );
            client
                .send(Message::Text(uncorrelatable.to_string()))
                .unwrap();
        }
        client
            .send(Message::Text(
                dj_v3_hello("invalid-frame-session", "hello-after-garbage").to_string(),
            ))
            .unwrap();
        let hello_ack = read_dj_ack(&mut client);
        assert_eq!(hello_ack.event_id, "hello-after-garbage");
        assert_eq!(hello_ack.outcome, DjLinkAckOutcome::Accepted);

        // Post-hello: a duplicate-key frame again receives no reply; the very
        // next message on the socket is the heartbeat ACK that follows it.
        client
            .send(Message::Text(
                r#"{"v":3,"type":"DJ_HEARTBEAT","agentId":"rb-output-dj-agent","sessionId":"invalid-frame-session","sequence":2,"sequence":3,"eventId":"dup-key-heartbeat","payload":{}}"#
                    .to_string(),
            ))
            .unwrap();
        client
            .send(Message::Text(
                json!({
                    "v": 3,
                    "type": "DJ_HEARTBEAT",
                    "agentId": DJ_V3_AGENT,
                    "sessionId": "invalid-frame-session",
                    "sequence": 4,
                    "eventId": "healthy-heartbeat",
                    "payload": {}
                })
                .to_string(),
            ))
            .unwrap();
        let healthy = read_dj_ack(&mut client);
        assert_eq!(healthy.event_id, "healthy-heartbeat");
        assert_eq!(healthy.outcome, DjLinkAckOutcome::Accepted);
        assert!(!server.dj_link_status().session_id.is_none());
        let _ = client.close(None);
        drop(server);
    }

    #[test]
    fn dj_link_ws_snapshot_order_and_state_broadcast_are_exact() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let handler: DjLinkDispatchHandler = Arc::new(|envelope| {
            if envelope.message_type == DjLinkMessageType::TimelineStateRequest {
                return DjLinkDispatchOutcome::TimelineState {
                    state_generation: 9,
                    state: DjLinkTimelineState {
                        message_type: "DJ_TIMELINE_STATE".to_string(),
                        event_id: envelope.event_id,
                        sequence: envelope.sequence,
                        state: protocol::DjLinkTimelineStateValue::Running,
                        loop_active: false,
                        timeline_id: "show-1".to_string(),
                        position_bars: 16,
                        play_session_id: Some("play-1".to_string()),
                        pedal_owner: None,
                        release_event_id: None,
                    },
                };
            }
            DjLinkDispatchOutcome::Accepted {
                state_generation: 9,
            }
        });
        let server = RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                max_connections: 2,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
            EngineSnapshot::default,
            (
                VideoRuntimeStatus::default,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
            Some(handler),
        )
        .unwrap();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();
        let read_value = |client: &mut tungstenite::WebSocket<_>| -> serde_json::Value {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };
        client
            .send(Message::Text(
                dj_v3_hello("snapshot-order-session", "hello-snapshot-order").to_string(),
            ))
            .unwrap();
        let hello_ack = read_value(&mut client);
        assert_eq!(hello_ack["v"], protocol::DJ_LINK_PROTOCOL_VERSION);
        assert_eq!(hello_ack["type"], "ACK");
        assert_eq!(hello_ack["eventId"], "hello-snapshot-order");
        assert_eq!(hello_ack["outcome"], "accepted");
        assert_eq!(
            hello_ack.as_object().unwrap().len(),
            7,
            "ACK wire shape must be exactly seven keys"
        );

        client
            .send(Message::Text(
                json!({
                    "v": 3,
                    "type": "DJ_STATE_SYNC",
                    "agentId": DJ_V3_AGENT,
                    "sessionId": "snapshot-order-session",
                    "sequence": 2,
                    "eventId": "sync-snapshot-order",
                    "payload": {
                        "released": false,
                        "ownerDeck": 1,
                        "ownerDeckId": "rekordbox-deck-1",
                        "activePlaySessionId": "play-1",
                    },
                })
                .to_string(),
            ))
            .unwrap();
        let sync_ack = read_value(&mut client);
        assert_eq!(sync_ack["eventId"], "sync-snapshot-order");

        client
            .send(Message::Text(
                json!({
                    "v": 3,
                    "type": "DJ_TIMELINE_STATE_REQUEST",
                    "agentId": DJ_V3_AGENT,
                    "sessionId": "snapshot-order-session",
                    "sequence": 3,
                    "eventId": "request-snapshot-order",
                    "payload": {},
                })
                .to_string(),
            ))
            .unwrap();
        let request_ack = read_value(&mut client);
        assert_eq!(request_ack["eventId"], "request-snapshot-order");
        let state = read_value(&mut client);
        assert_eq!(state["v"], protocol::DJ_LINK_PROTOCOL_VERSION);
        assert_eq!(state["type"], "DJ_TIMELINE_STATE");
        assert_eq!(state["agentId"], DJ_V3_AGENT);
        assert_eq!(state["sessionId"], "snapshot-order-session");
        assert_eq!(state["payload"]["state"], "running");
        assert_eq!(state["payload"]["timelineId"], "show-1");
        assert_eq!(state["payload"]["positionBars"], 16);
        assert_eq!(state["payload"]["playSessionId"], "play-1");
        assert_eq!(state["payload"]["pedalOwner"], serde_json::json!(null));
        assert_eq!(state["payload"]["releaseEventId"], serde_json::json!(null));
        assert_eq!(
            state["payload"].as_object().unwrap().len(),
            7,
            "timeline-state payload must be exactly seven keys"
        );
        wait_for_dj_link_snapshot_ready(&server);

        // A client-injected DJ_TIMELINE_STATE is not an ingress message type:
        // it fails exact parsing and receives no reply, and the connection
        // stays fully usable for the heartbeat that follows.
        client
            .send(Message::Text(
                json!({
                    "v": 3,
                    "type": "DJ_TIMELINE_STATE",
                    "agentId": DJ_V3_AGENT,
                    "sessionId": "snapshot-order-session",
                    "sequence": 4,
                    "eventId": "server-only-state",
                    "payload": {"state": "running", "loopActive": false, "timelineId": "show-1", "positionBars": 16},
                })
                .to_string(),
            ))
            .unwrap();
        client
            .send(Message::Text(
                json!({
                    "v": 3,
                    "type": "DJ_HEARTBEAT",
                    "agentId": DJ_V3_AGENT,
                    "sessionId": "snapshot-order-session",
                    "sequence": 5,
                    "eventId": "after-server-only-state",
                    "payload": {},
                })
                .to_string(),
            ))
            .unwrap();
        let after_injected_state = read_value(&mut client);
        assert_eq!(after_injected_state["eventId"], "after-server-only-state");
        assert_eq!(after_injected_state["outcome"], "accepted");
        // DJ Link owns only the exact /dj-link target.  The Web Remote /ws
        // endpoint keeps its pairing-token contract even while DJ Link is
        // enabled, and a bare /ws must never be reinterpreted as the
        // dedicated HELLO/token DJ transport.
        let bare_ws = format!("ws://127.0.0.1:{port}/ws");
        // A bare /ws must fail before any WebSocket upgrade (pairing PIN
        // required); under the old hijack behavior the DJ handler accepted
        // the upgrade and left an unauthenticated socket waiting for HELLO.
        assert!(tungstenite::connect(bare_ws.as_str()).is_err());
        let remote_url = format!("ws://127.0.0.1:{port}/ws?token=123456");
        let (mut remote_client, _) = tungstenite::connect(remote_url.as_str()).unwrap();
        for _ in 0..20 {
            if server.status().clients.len() == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(server.status().clients.len(), 1);
        let invalid_query = format!("ws://127.0.0.1:{port}/ws?token=654321");
        assert!(tungstenite::connect(invalid_query.as_str()).is_err());
        let _ = remote_client.close(None);
        let _ = client.close(None);
        drop(server);
    }

    #[test]
    fn dj_link_actual_socket_snapshot_unavailable_never_regresses_or_duplicates_b() {
        let snapshot = Arc::new(Mutex::new(Some(EngineSnapshot::default())));
        let provider_calls = Arc::new(AtomicU64::new(0));
        let snapshot_provider = {
            let snapshot = Arc::clone(&snapshot);
            let provider_calls = Arc::clone(&provider_calls);
            move || {
                provider_calls.fetch_add(1, Ordering::AcqRel);
                snapshot.lock().unwrap().clone()
            }
        };
        let handler = dj_v3_state_responder(1);
        let server = RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link_with_process_fence(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port: 0,
                pairing_pin: "123456".to_string(),
                max_connections: 2,
                max_messages_per_second: 1_000,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
            snapshot_provider,
            (
                VideoRuntimeStatus::default,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
            Some(handler),
            None,
        )
        .unwrap();
        let port = server.local_addr().port();
        let dj_url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut dj_client, _) = tungstenite::connect(dj_url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(&mut dj_client, port, "unavailable-snapshot", &server);
        let read_value = |client: &mut tungstenite::WebSocket<_>| -> Value {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };

        let baseline_deadline = Instant::now() + Duration::from_secs(1);
        while provider_calls.load(Ordering::Acquire) == 0 {
            assert!(
                Instant::now() < baseline_deadline,
                "observer never sampled baseline A"
            );
            thread::sleep(Duration::from_millis(2));
        }
        let mut committed_b = EngineSnapshot::default();
        committed_b.timeline.playing = true;
        committed_b.timeline.position_ms = 4_000;
        committed_b.timeline.duration_ms = 20_000;
        *snapshot.lock().unwrap() = Some(committed_b.clone());
        let transition = read_value(&mut dj_client);
        assert_eq!(transition["type"], "DJ_TIMELINE_STATE");
        assert_eq!(transition["payload"]["state"], "running");
        assert_eq!(transition["payload"]["positionBars"], 2);
        let b_sequence = server
            .dj_link_status()
            .last_outbound_sequence
            .expect("committed B must be delivered");

        *snapshot.lock().unwrap() = None;
        let unavailable_calls = provider_calls.load(Ordering::Acquire);
        let unavailable_deadline = Instant::now() + Duration::from_secs(1);
        while provider_calls.load(Ordering::Acquire) < unavailable_calls + 2 {
            assert!(
                Instant::now() < unavailable_deadline,
                "observer did not sample unavailable snapshot twice"
            );
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            server.dj_link_status().last_outbound_sequence,
            Some(b_sequence)
        );

        let remote_url = format!("ws://127.0.0.1:{port}/ws?token=123456");
        let (mut remote_client, _) = tungstenite::connect(remote_url.as_str()).unwrap();
        remote_client
            .send(Message::Text(r#"{"type":"getSnapshot"}"#.to_string()))
            .unwrap();
        let unavailable = match remote_client.read().unwrap() {
            Message::Text(text) => serde_json::from_str::<Value>(&text).unwrap(),
            other => panic!("unexpected Web Remote reply: {other:?}"),
        };
        assert_eq!(unavailable["ok"], false);
        assert!(unavailable["error"]
            .as_str()
            .is_some_and(|error| error.contains("retry")));
        assert!(unavailable.get("type").is_none());
        assert!(unavailable.get("snapshot").is_none());

        let resume_calls = provider_calls.load(Ordering::Acquire);
        *snapshot.lock().unwrap() = Some(committed_b);
        let resume_deadline = Instant::now() + Duration::from_secs(1);
        while provider_calls.load(Ordering::Acquire) < resume_calls + 2 {
            assert!(
                Instant::now() < resume_deadline,
                "observer did not resample committed B"
            );
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            server.dj_link_status().last_outbound_sequence,
            Some(b_sequence),
            "resuming the same committed B must not emit a duplicate"
        );

        let _ = remote_client.close(None);
        let _ = dj_client.close(None);
        drop(server);
    }

    #[test]
    fn dj_link_engine_transition_broadcast_is_async_and_generation_fenced() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let snapshot = Arc::new(Mutex::new(EngineSnapshot::default()));
        let snapshot_provider = {
            let snapshot = Arc::clone(&snapshot);
            move || snapshot.lock().unwrap().clone()
        };
        let handler = dj_v3_state_responder(1);
        let server = RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                max_connections: 2,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
            snapshot_provider,
            (
                VideoRuntimeStatus::default,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
            Some(handler),
        )
        .unwrap();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(&mut client, port, "engine-transition-session", &server);

        {
            let mut changed = snapshot.lock().unwrap();
            changed.timeline.playing = true;
            changed.timeline.position_ms = 4_000;
            changed.timeline.duration_ms = 20_000;
        }
        let transition = match client.read().unwrap() {
            Message::Text(text) => serde_json::from_str::<serde_json::Value>(&text).unwrap(),
            other => panic!("unexpected async DJ state: {other:?}"),
        };
        assert_eq!(transition["type"], "DJ_TIMELINE_STATE");
        assert_eq!(transition["payload"]["state"], "running");
        assert_eq!(transition["payload"]["timelineId"], "1");
        assert_eq!(transition["payload"]["positionBars"], 2);
        assert!(transition["eventId"]
            .as_str()
            .is_some_and(|event_id| event_id.starts_with("syndocal-dj-state-")));
        assert!(server.dj_link_status().last_outbound_delivery.as_deref() == Some("delivered"));

        let _ = client.close(None);
        for _ in 0..30 {
            if !server.dj_link_status().connected {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!server.dj_link_status().connected);
        assert!(!server.dj_link_status().snapshot_ready);
        {
            let mut changed = snapshot.lock().unwrap();
            changed.timeline.playing = false;
            changed.timeline.position_ms = 4_000;
        }
        thread::sleep(Duration::from_millis(150));
        assert!(!server.dj_link_status().connected);
        // A send/read failure retires the old session; a subsequent HELLO is
        // the only way to regain an observer lane and receives a new
        // generation.  No queued state from the old socket is replayed.
        let old_generation = server.dj_link_status().generation;
        let (mut reconnect, _) = (0..100)
            .find_map(|_| tungstenite::connect(url.as_str()).ok())
            .expect("DJ Link observer did not accept a reconnect");
        reconnect
            .send(Message::Text(
                dj_v3_hello("engine-reconnect-session", "hello-engine-reconnect").to_string(),
            ))
            .unwrap();
        let reconnect_ack = match reconnect.read().unwrap() {
            Message::Text(text) => serde_json::from_str::<serde_json::Value>(&text).unwrap(),
            other => panic!("unexpected DJ Link reconnect reply: {other:?}"),
        };
        assert_eq!(reconnect_ack["type"], "ACK");
        assert_eq!(reconnect_ack["outcome"], "accepted");
        assert_eq!(
            server.dj_link_status().session_id.as_deref(),
            Some("engine-reconnect-session")
        );
        assert!(server.dj_link_status().generation > old_generation);
        let _ = reconnect.close(None);
        drop(server);
    }

    #[test]
    fn dj_link_observer_generation_barrier_sends_no_stale_bytes() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let snapshot = Arc::new(Mutex::new(EngineSnapshot::default()));
        let snapshot_provider = {
            let snapshot = Arc::clone(&snapshot);
            move || snapshot.lock().unwrap().clone()
        };
        let handler = dj_v3_state_responder(1);
        let entered = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let release = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let _barrier = install_dj_link_before_outbound_permit_hook({
            let entered = Arc::clone(&entered);
            let release = Arc::clone(&release);
            Arc::new(move || {
                let (entered_lock, entered_ready) = &*entered;
                *entered_lock.lock().unwrap() = true;
                entered_ready.notify_all();
                let (release_lock, release_ready) = &*release;
                let mut allowed = release_lock.lock().unwrap();
                while !*allowed {
                    allowed = release_ready.wait(allowed).unwrap();
                }
            })
        });
        let server = RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                max_connections: 2,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
            snapshot_provider,
            (
                VideoRuntimeStatus::default,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
            Some(handler),
        )
        .unwrap();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut old_client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(&mut old_client, port, "observer-old", &server);
        {
            let mut changed = snapshot.lock().unwrap();
            changed.timeline.playing = true;
            changed.timeline.position_ms = 4_000;
            changed.timeline.duration_ms = 20_000;
        }
        for _ in 0..50 {
            if *entered.0.lock().unwrap() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            *entered.0.lock().unwrap(),
            "observer barrier was not reached"
        );

        let (mut new_client, _) = tungstenite::connect(url.as_str()).unwrap();
        new_client
            .send(Message::Text(
                dj_v3_hello("observer-new", "hello-observer-new").to_string(),
            ))
            .unwrap();
        let new_ack = match new_client.read().unwrap() {
            Message::Text(text) => serde_json::from_str::<serde_json::Value>(&text).unwrap(),
            other => panic!("unexpected replacement HELLO reply: {other:?}"),
        };
        assert_eq!(new_ack["type"], "ACK");
        assert_eq!(new_ack["eventId"], "hello-observer-new");
        assert_eq!(new_ack["outcome"], "accepted");
        assert_eq!(
            server.dj_link_status().session_id.as_deref(),
            Some("observer-new")
        );

        {
            let (release_lock, release_ready) = &*release;
            *release_lock.lock().unwrap() = true;
            release_ready.notify_all();
        }
        let old_result = old_client.read();
        if let Ok(Message::Text(text)) = old_result {
            panic!("stale observer delivered bytes after replacement: {text}");
        }
        assert_eq!(
            server.dj_link_status().session_id.as_deref(),
            Some("observer-new")
        );
        let _ = new_client.close(None);
        let _ = old_client.close(None);
        drop(server);
    }

    #[test]
    fn dj_link_two_socket_replacement_fences_before_physical_dispatch() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let entered = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let release = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let dispatches = Arc::new(AtomicU64::new(0));
        let handler: DjLinkDispatchHandler = {
            let dispatches = Arc::clone(&dispatches);
            let inner = dj_v3_state_responder(41);
            Arc::new(move |envelope| {
                if !matches!(
                    envelope.message_type,
                    DjLinkMessageType::StateSync | DjLinkMessageType::TimelineStateRequest
                ) {
                    dispatches.fetch_add(1, Ordering::SeqCst);
                }
                inner(envelope)
            })
        };
        let server = RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                max_connections: 2,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
            EngineSnapshot::default,
            (
                VideoRuntimeStatus::default,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
            Some(handler),
        )
        .unwrap();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let read_ack = |client: &mut tungstenite::WebSocket<_>| -> DjLinkAck {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };
        let active_frame = |session: &str, sequence: u64| {
            json!({
                "v": 3,
                "type": "DJ_TRACK_ACTIVE",
                "agentId": DJ_V3_AGENT,
                "sessionId": session,
                "sequence": sequence,
                "eventId": "event-active",
                "payload": dj_v3_generic_track_active_payload("play-1"),
            })
        };

        let (mut old_client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(&mut old_client, port, "old-session", &server);

        // Installed only after the snapshot gate so the barrier blocks the
        // physical command and never the gate traffic itself.
        let _pre_dispatch_guard = install_dj_link_pre_dispatch_hook({
            let entered = Arc::clone(&entered);
            let release = Arc::clone(&release);
            Arc::new(move || {
                let (entered_lock, entered_ready) = &*entered;
                *entered_lock.lock().unwrap() = true;
                entered_ready.notify_all();
                let (release_lock, release_ready) = &*release;
                let mut allowed = release_lock.lock().unwrap();
                while !*allowed {
                    allowed = release_ready.wait(allowed).unwrap();
                }
            })
        });

        old_client
            .send(Message::Text(active_frame("old-session", 4).to_string()))
            .unwrap();
        let (entered_lock, entered_ready) = &*entered;
        let mut did_enter = entered_lock.lock().unwrap();
        while !*did_enter {
            did_enter = entered_ready.wait(did_enter).unwrap();
        }
        drop(did_enter);

        let (mut replacement, _) = tungstenite::connect(url.as_str()).unwrap();
        replacement
            .send(Message::Text(
                dj_v3_hello("new-session", "hello-new").to_string(),
            ))
            .unwrap();
        let replacement_ack = read_ack(&mut replacement);
        assert_eq!(replacement_ack.outcome, DjLinkAckOutcome::Accepted);
        assert!(replacement_ack.state_generation > 1);
        assert_eq!(dispatches.load(Ordering::SeqCst), 0);

        let (release_lock, release_ready) = &*release;
        *release_lock.lock().unwrap() = true;
        release_ready.notify_all();
        let old_ack = read_ack(&mut old_client);
        assert_eq!(old_ack.outcome, DjLinkAckOutcome::Rejected);
        assert_eq!(old_ack.code.as_deref(), Some("stale_session"));
        assert_eq!(dispatches.load(Ordering::SeqCst), 0);

        let replacement_heartbeat = json!({
            "v": 3,
            "type": "DJ_HEARTBEAT",
            "agentId": DJ_V3_AGENT,
            "sessionId": "new-session",
            "sequence": 2,
            "eventId": "heartbeat-new",
            "payload": {}
        });
        replacement
            .send(Message::Text(replacement_heartbeat.to_string()))
            .unwrap();
        assert_eq!(
            read_ack(&mut replacement).outcome,
            DjLinkAckOutcome::Accepted
        );

        let (mut collision, _) = tungstenite::connect(url.as_str()).unwrap();
        collision
            .send(Message::Text(
                dj_v3_hello("collision-session", "event-active").to_string(),
            ))
            .unwrap();
        let collision_ack = read_ack(&mut collision);
        assert_eq!(collision_ack.outcome, DjLinkAckOutcome::Rejected);
        assert_eq!(collision_ack.code.as_deref(), Some("event_id_conflict"));
        drop(server);
    }

    #[test]
    fn dj_link_dispatch_lease_serializes_after_final_check_and_busy_replacement() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let entered = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let release = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let dispatches = Arc::new(AtomicU64::new(0));
        let handler: DjLinkDispatchHandler = {
            let dispatches = Arc::clone(&dispatches);
            let inner = dj_v3_state_responder(41);
            Arc::new(move |envelope| {
                if !matches!(
                    envelope.message_type,
                    DjLinkMessageType::StateSync | DjLinkMessageType::TimelineStateRequest
                ) {
                    dispatches.fetch_add(1, Ordering::SeqCst);
                }
                inner(envelope)
            })
        };
        let server = RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                max_connections: 2,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
            EngineSnapshot::default,
            (
                VideoRuntimeStatus::default,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
            Some(handler),
        )
        .unwrap();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let read_ack = |client: &mut tungstenite::WebSocket<_>| -> DjLinkAck {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };

        let (mut old_client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(&mut old_client, port, "old-session", &server);

        // Installed only after the snapshot gate so the barrier blocks the
        // physical command and never the gate traffic itself.
        let _after_final_guard = install_dj_link_after_final_check_hook({
            let entered = Arc::clone(&entered);
            let release = Arc::clone(&release);
            Arc::new(move || {
                let (entered_lock, entered_ready) = &*entered;
                *entered_lock.lock().unwrap() = true;
                entered_ready.notify_all();
                let (release_lock, release_ready) = &*release;
                let mut allowed = release_lock.lock().unwrap();
                while !*allowed {
                    allowed = release_ready.wait(allowed).unwrap();
                }
            })
        });

        let active = json!({
            "v": 3,
            "type": "DJ_TRACK_ACTIVE",
            "agentId": DJ_V3_AGENT,
            "sessionId": "old-session",
            "sequence": 4,
            "eventId": "event-active",
            "payload": dj_v3_generic_track_active_payload("play-1"),
        });
        old_client.send(Message::Text(active.to_string())).unwrap();
        let (entered_lock, entered_ready) = &*entered;
        let mut did_enter = entered_lock.lock().unwrap();
        while !*did_enter {
            did_enter = entered_ready.wait(did_enter).unwrap();
        }
        drop(did_enter);
        assert_eq!(dispatches.load(Ordering::SeqCst), 0);

        let (mut replacement, _) = tungstenite::connect(url.as_str()).unwrap();
        replacement
            .send(Message::Text(
                dj_v3_hello("new-session", "hello-new").to_string(),
            ))
            .unwrap();
        let busy = read_ack(&mut replacement);
        assert_eq!(busy.outcome, DjLinkAckOutcome::Busy);
        assert_eq!(busy.code.as_deref(), Some("session_replacement_busy"));
        assert_eq!(busy.state_generation, 0);
        assert_eq!(dispatches.load(Ordering::SeqCst), 0);

        let (release_lock, release_ready) = &*release;
        *release_lock.lock().unwrap() = true;
        release_ready.notify_all();
        let old_ack = read_ack(&mut old_client);
        assert_eq!(old_ack.outcome, DjLinkAckOutcome::Accepted);
        assert_eq!(old_ack.sequence, 4);
        assert_eq!(old_ack.state_generation, 41);
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);

        // The old terminal receipt remains exact while its generation is
        // still current, and the previously Busy HELLO can now replace it.
        old_client.send(Message::Text(active.to_string())).unwrap();
        assert_eq!(
            read_ack(&mut old_client).outcome,
            DjLinkAckOutcome::Duplicate
        );
        let (mut retry, _) = tungstenite::connect(url.as_str()).unwrap();
        retry
            .send(Message::Text(
                dj_v3_hello("new-session", "hello-new").to_string(),
            ))
            .unwrap();
        let replacement_ack = read_ack(&mut retry);
        assert_eq!(replacement_ack.outcome, DjLinkAckOutcome::Accepted);
        assert!(replacement_ack.state_generation > 1);
        drop(server);
    }

    #[test]
    fn dj_link_dispatch_lease_releases_on_panic_and_rejects_stale_begin() {
        let now = Instant::now();
        let registry = Arc::new(Mutex::new(DjLinkRegistry::default()));
        let (generation, envelope) = {
            let mut registry_guard = registry.lock().unwrap();
            let generation = registry_guard.register_test_session(
                DJ_V3_AGENT,
                "panic-session",
                1,
                "panic-peer",
                now,
            );
            let envelope = dj_v3_envelope(
                DjLinkMessageType::TimelineLoopSet,
                "panic-session",
                2,
                "panic-event",
                dj_v3_loop_set_payload(true),
            );
            assert!(registry_guard.begin_dispatch(&envelope, generation).is_ok());
            (generation, envelope)
        };
        let lease = DjLinkDispatchLease {
            registry: Arc::clone(&registry),
            agent_id: envelope.agent_id.clone(),
            session_id: envelope.session_id.clone(),
            generation,
        };
        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _lease = lease;
            panic!("test handler panic");
        }));
        assert!(panic_result.is_err());
        assert!(registry.lock().unwrap().dispatch_leases.is_empty());
        let stale = DjLinkEnvelope {
            session_id: "stale-session".to_string(),
            ..envelope
        };
        assert!(registry
            .lock()
            .unwrap()
            .begin_dispatch(&stale, generation)
            .is_err());
    }

    #[test]
    fn dj_link_socket_handler_panic_terminalizes_and_releases_connection_slot() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let dispatches = Arc::new(AtomicU64::new(0));
        let panic_once = Arc::new(AtomicBool::new(true));
        let handler: DjLinkDispatchHandler = {
            let dispatches = Arc::clone(&dispatches);
            let panic_once = Arc::clone(&panic_once);
            let inner = dj_v3_state_responder(7);
            Arc::new(move |envelope| {
                // The snapshot gate's STATE_SYNC/STATE_REQUEST traffic shares
                // this handler and must complete normally; only the physical
                // command under test may trip the one-shot panic.
                if envelope.message_type != DjLinkMessageType::TrackActive {
                    return inner(envelope);
                }
                dispatches.fetch_add(1, Ordering::SeqCst);
                if panic_once.swap(false, Ordering::SeqCst) {
                    panic!("test-only handler panic details must not cross the wire");
                }
                inner(envelope)
            })
        };
        let server = RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                max_connections: 1,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
            EngineSnapshot::default,
            (
                VideoRuntimeStatus::default,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
            Some(handler),
        )
        .unwrap();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let read_ack = |client: &mut tungstenite::WebSocket<_>| -> DjLinkAck {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };
        let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(&mut client, port, "session-panic", &server);
        let active = json!({
            "v": 3,
            "type": "DJ_TRACK_ACTIVE",
            "agentId": DJ_V3_AGENT,
            "sessionId": "session-panic",
            "sequence": 4,
            "eventId": "event-panic",
            "payload": dj_v3_generic_track_active_payload("play-panic"),
        });
        client.send(Message::Text(active.to_string())).unwrap();
        let failed = read_ack(&mut client);
        assert_eq!(failed.outcome, DjLinkAckOutcome::Rejected);
        assert_eq!(failed.code.as_deref(), Some("handler_failed"));
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);
        let failed_wire = serde_json::to_string(&failed).unwrap();
        assert!(!failed_wire.contains("test-only handler panic"));

        // The terminal retry is exact and cannot re-enter the panicking
        // handler; admission, lease, and terminal truth are all bounded.
        client.send(Message::Text(active.to_string())).unwrap();
        let duplicate = read_ack(&mut client);
        assert_eq!(duplicate.outcome, DjLinkAckOutcome::Duplicate);
        assert_eq!(dispatches.load(Ordering::SeqCst), 1);
        {
            let registry = server.dj_link_registry.lock().unwrap();
            assert!(registry.inflight.is_empty());
            assert!(registry.dispatch_leases.is_empty());
            assert!(registry.terminal_count() <= DJ_LINK_TERMINAL_LIMIT);
        }
        client.close(None).unwrap();

        // max_connections=1 is reusable after the panic-path client closes;
        // this proves the production slot guard is not leaked.
        let mut fresh = None;
        for _ in 0..100 {
            if let Ok(connection) = tungstenite::connect(url.as_str()) {
                fresh = Some(connection);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let (mut fresh, _) = fresh.expect("DJ Link slot was not released after handler panic");
        fresh
            .send(Message::Text(
                dj_v3_hello("session-fresh", "hello-fresh").to_string(),
            ))
            .unwrap();
        assert_eq!(read_ack(&mut fresh).outcome, DjLinkAckOutcome::Accepted);
        drop(server);
    }

    #[test]
    fn dj_link_observer_state_matrix_and_wire_envelope_are_deduplicated() {
        let idle = DjLinkEngineObservation {
            timeline_id: 7,
            playing: false,
            position_ms: 0,
            duration_ms: 20_000,
            loop_active: false,
            bpm_millis: 128_000,
        };
        let running = DjLinkEngineObservation {
            playing: true,
            position_ms: 4_000,
            ..idle
        };
        let stopped = DjLinkEngineObservation {
            position_ms: 4_000,
            ..idle
        };
        let ended = DjLinkEngineObservation {
            position_ms: 20_000,
            ..idle
        };
        let reset = DjLinkEngineObservation { ..idle };

        assert_eq!(idle.state(None), protocol::DjLinkTimelineStateValue::Idle);
        assert_eq!(
            running.state(Some(idle)),
            protocol::DjLinkTimelineStateValue::Running
        );
        assert_eq!(
            stopped.state(Some(running)),
            protocol::DjLinkTimelineStateValue::Stopped
        );
        assert_eq!(
            ended.state(Some(stopped)),
            protocol::DjLinkTimelineStateValue::Ended
        );
        assert_eq!(
            reset.state(Some(stopped)),
            protocol::DjLinkTimelineStateValue::Reset
        );

        // The observer's semantic key intentionally excludes positionBars:
        // ordinary engine ticks do not flood the bounded outbound queue.
        assert_eq!(
            running.semantic_key(Some(idle)),
            (7, protocol::DjLinkTimelineStateValue::Running, false)
        );
        assert_eq!(
            running.semantic_key(Some(idle)),
            running.semantic_key(Some(idle))
        );

        let state = DjLinkTimelineState {
            message_type: "DJ_TIMELINE_STATE".to_string(),
            event_id: "matrix-state".to_string(),
            sequence: 11,
            state: protocol::DjLinkTimelineStateValue::Running,
            loop_active: true,
            timeline_id: "7".to_string(),
            position_bars: 2,
            play_session_id: Some("play-1".to_string()),
            pedal_owner: Some("pedal-1".to_string()),
            release_event_id: None,
        };
        let wire_text = dj_link_state_wire(&state, DJ_V3_AGENT, "matrix-session").unwrap();
        let wire_value: serde_json::Value = serde_json::from_str(&wire_text).unwrap();
        assert_eq!(wire_value["v"], protocol::DJ_LINK_PROTOCOL_VERSION);
        assert_eq!(wire_value["type"], "DJ_TIMELINE_STATE");
        assert_eq!(wire_value["agentId"], DJ_V3_AGENT);
        assert_eq!(wire_value["sessionId"], "matrix-session");
        assert_eq!(wire_value["sequence"], 11);
        assert_eq!(wire_value["payload"]["state"], "running");
        assert_eq!(wire_value["payload"]["positionBars"], 2);
        assert_eq!(wire_value["payload"]["playSessionId"], "play-1");
        assert_eq!(wire_value["payload"]["pedalOwner"], "pedal-1");
        assert_eq!(
            wire_value["payload"]["releaseEventId"],
            serde_json::json!(null)
        );
        assert_eq!(
            wire_value
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>(),
            [
                "v",
                "type",
                "agentId",
                "sessionId",
                "sequence",
                "eventId",
                "payload"
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<std::collections::BTreeSet<_>>()
        );
    }

    #[test]
    fn dj_link_observer_queue_full_retires_generation_before_reconnect() {
        let now = Instant::now();
        let mut registry = DjLinkRegistry::default();
        let (sender, receiver) = mpsc::sync_channel(1);
        let hello_event = "queue-hello-old".to_string();
        registry.inflight.insert(
            ("queue-agent".to_string(), hello_event.clone()),
            DjLinkInflight {
                shape_digest: 0,
                process_lifetime_shape_digest: 0,
                identity_digest: 0,
                sequence: 1,
                generation: 0,
                is_physical: false,
            },
        );
        let old_generation = registry
            .register(
                "queue-agent",
                "queue-old",
                1,
                &hello_event,
                DjLinkRegistrationTransport {
                    peer: "queue-peer".to_string(),
                    outbound: sender,
                    now,
                },
            )
            .unwrap();
        registry
            .inflight
            .remove(&("queue-agent".to_string(), hello_event));
        let session = registry.sessions.get_mut("queue-agent").unwrap();
        session.state_sync_received = true;
        session.timeline_state_request_received = true;
        session.snapshot_ready = true;

        let state = DjLinkTimelineState {
            message_type: "DJ_TIMELINE_STATE".to_string(),
            event_id: "pending".to_string(),
            sequence: 1,
            state: protocol::DjLinkTimelineStateValue::Running,
            loop_active: false,
            timeline_id: "queue".to_string(),
            position_bars: 1,
            play_session_id: None,
            pedal_owner: None,
            release_event_id: None,
        };
        assert!(registry.queue_outbound_state(state.clone()).unwrap());
        assert!(registry.queue_outbound_state(state).is_err());
        assert!(!registry.is_current("queue-agent", "queue-old", old_generation));
        assert!(!registry.status.connected);
        drop(receiver);

        let (new_sender, _new_receiver) = mpsc::sync_channel(1);
        let new_hello = "queue-hello-new".to_string();
        registry.inflight.insert(
            ("queue-agent".to_string(), new_hello.clone()),
            DjLinkInflight {
                shape_digest: 0,
                process_lifetime_shape_digest: 0,
                identity_digest: 0,
                sequence: 1,
                generation: 0,
                is_physical: false,
            },
        );
        let new_generation = registry
            .register(
                "queue-agent",
                "queue-new",
                1,
                &new_hello,
                DjLinkRegistrationTransport {
                    peer: "queue-peer-new".to_string(),
                    outbound: new_sender,
                    now,
                },
            )
            .unwrap();
        assert!(new_generation > old_generation);
        assert!(registry.is_current("queue-agent", "queue-new", new_generation));
    }

    fn dj_truth_state(
        play_session_id: Option<&str>,
        pedal_owner: Option<&str>,
        release_event_id: Option<&str>,
    ) -> DjLinkTimelineState {
        DjLinkTimelineState {
            message_type: "DJ_TIMELINE_STATE".to_string(),
            event_id: "pending".to_string(),
            sequence: 1,
            state: protocol::DjLinkTimelineStateValue::Running,
            loop_active: true,
            timeline_id: "7".to_string(),
            position_bars: 2,
            play_session_id: play_session_id.map(str::to_string),
            pedal_owner: pedal_owner.map(str::to_string),
            release_event_id: release_event_id.map(str::to_string),
        }
    }

    fn dj_truth_registry(
        agent_id: &str,
        session_id: &str,
    ) -> (DjLinkRegistry, u64, Receiver<DjLinkOutboundState>) {
        let mut registry = DjLinkRegistry::default();
        let (sender, receiver) = mpsc::sync_channel(DJ_LINK_OUTBOUND_QUEUE_LIMIT);
        let hello_event = format!("truth-hello-{session_id}");
        registry.inflight.insert(
            (agent_id.to_string(), hello_event.clone()),
            DjLinkInflight {
                shape_digest: 0,
                process_lifetime_shape_digest: 0,
                identity_digest: 0,
                sequence: 1,
                generation: 0,
                is_physical: false,
            },
        );
        let generation = registry
            .register(
                agent_id,
                session_id,
                1,
                &hello_event,
                DjLinkRegistrationTransport {
                    peer: "truth-peer".to_string(),
                    outbound: sender,
                    now: Instant::now(),
                },
            )
            .expect("truth test registration");
        registry
            .inflight
            .remove(&(agent_id.to_string(), hello_event));
        {
            let session = registry.sessions.get_mut(agent_id).unwrap();
            session.state_sync_received = true;
            session.timeline_state_request_received = true;
            session.snapshot_ready = true;
        }
        (registry, generation, receiver)
    }

    fn drain_dj_truth_queue(receiver: &Receiver<DjLinkOutboundState>) -> Vec<DjLinkTimelineState> {
        let mut states = Vec::new();
        while let Ok(outbound) = receiver.try_recv() {
            states.push(outbound.state);
        }
        states
    }

    #[test]
    fn dj_link_state_truth_equal_covers_every_serialized_truth_field() {
        let baseline = dj_truth_state(Some("play-1"), Some("pedal-1"), None);
        assert!(dj_link_state_truth_equal(&baseline, &baseline));

        let truth_variants = [
            dj_truth_state(Some("play-2"), Some("pedal-1"), None),
            dj_truth_state(Some("play-1"), Some("pedal-2"), None),
            dj_truth_state(Some("play-1"), Some("pedal-1"), Some("release-7")),
            dj_truth_state(None, Some("pedal-1"), None),
            dj_truth_state(Some("play-1"), None, None),
            DjLinkTimelineState {
                state: protocol::DjLinkTimelineStateValue::Stopped,
                ..dj_truth_state(Some("play-1"), Some("pedal-1"), None)
            },
            DjLinkTimelineState {
                loop_active: false,
                ..dj_truth_state(Some("play-1"), Some("pedal-1"), None)
            },
            DjLinkTimelineState {
                timeline_id: "8".to_string(),
                ..dj_truth_state(Some("play-1"), Some("pedal-1"), None)
            },
            DjLinkTimelineState {
                position_bars: 3,
                ..dj_truth_state(Some("play-1"), Some("pedal-1"), None)
            },
        ];
        for variant in &truth_variants {
            assert!(!dj_link_state_truth_equal(&baseline, variant));
            assert!(!dj_link_state_truth_equal(variant, &baseline));
        }

        // Transport identity is per-message, never authoritative truth:
        // a fresh eventId/sequence on identical truth must stay equal so
        // duplicate suppression and heartbeat/liveness cadence are unchanged.
        let fresh_transport = DjLinkTimelineState {
            event_id: "syndocal-dj-state-99".to_string(),
            sequence: 42,
            ..baseline.clone()
        };
        assert!(dj_link_state_truth_equal(&baseline, &fresh_transport));
        assert!(dj_link_state_truth_equal(&fresh_transport, &baseline));
    }

    #[test]
    fn dj_link_ownership_and_fence_only_changes_broadcast_exactly_once_with_null_transitions() {
        let (mut registry, _generation, receiver) =
            dj_truth_registry("truth-agent", "truth-session");

        assert!(registry
            .queue_outbound_state(dj_truth_state(Some("play-1"), None, None))
            .unwrap());
        let delivered = drain_dj_truth_queue(&receiver);
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].event_id, "syndocal-dj-state-1");
        assert_eq!(delivered[0].sequence, 1);
        assert_eq!(delivered[0].play_session_id.as_deref(), Some("play-1"));
        registry.note_outbound_state(&delivered[0], &delivered[0].event_id, delivered[0].sequence);

        // Identical truth stays suppressed even with a fresh transport
        // identity; the bounded outbound queue must not be flooded.
        assert!(!registry
            .queue_outbound_state(dj_truth_state(Some("play-1"), None, None))
            .unwrap());
        assert!(!registry
            .queue_outbound_state(DjLinkTimelineState {
                event_id: "inbound-echo".to_string(),
                sequence: 77,
                ..dj_truth_state(Some("play-1"), None, None)
            })
            .unwrap());
        assert!(drain_dj_truth_queue(&receiver).is_empty());

        // playSessionId-only ownership change broadcasts exactly once.
        assert!(registry
            .queue_outbound_state(dj_truth_state(Some("play-2"), None, None))
            .unwrap());
        let delivered = drain_dj_truth_queue(&receiver);
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].event_id, "syndocal-dj-state-2");
        assert_eq!(delivered[0].play_session_id.as_deref(), Some("play-2"));
        registry.note_outbound_state(&delivered[0], &delivered[0].event_id, delivered[0].sequence);
        assert!(!registry
            .queue_outbound_state(dj_truth_state(Some("play-2"), None, None))
            .unwrap());

        // pedalOwner-only fence change broadcasts exactly once.
        assert!(registry
            .queue_outbound_state(dj_truth_state(Some("play-2"), Some("pedal-1"), None))
            .unwrap());
        let delivered = drain_dj_truth_queue(&receiver);
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].pedal_owner.as_deref(), Some("pedal-1"));
        registry.note_outbound_state(&delivered[0], &delivered[0].event_id, delivered[0].sequence);
        assert!(!registry
            .queue_outbound_state(dj_truth_state(Some("play-2"), Some("pedal-1"), None))
            .unwrap());

        // Canonical Release handoff: new owner plus releaseEventId.
        assert!(registry
            .queue_outbound_state(dj_truth_state(
                Some("play-2"),
                Some("pedal-2"),
                Some("release-9")
            ))
            .unwrap());
        let delivered = drain_dj_truth_queue(&receiver);
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].pedal_owner.as_deref(), Some("pedal-2"));
        assert_eq!(delivered[0].release_event_id.as_deref(), Some("release-9"));
        registry.note_outbound_state(&delivered[0], &delivered[0].event_id, delivered[0].sequence);

        // Null transitions broadcast exactly once per field.
        assert!(registry
            .queue_outbound_state(dj_truth_state(Some("play-2"), Some("pedal-2"), None))
            .unwrap());
        let delivered = drain_dj_truth_queue(&receiver);
        assert_eq!(delivered.len(), 1);
        registry.note_outbound_state(&delivered[0], &delivered[0].event_id, delivered[0].sequence);
        assert!(registry
            .queue_outbound_state(dj_truth_state(Some("play-2"), None, None))
            .unwrap());
        let delivered = drain_dj_truth_queue(&receiver);
        assert_eq!(delivered.len(), 1);
        registry.note_outbound_state(&delivered[0], &delivered[0].event_id, delivered[0].sequence);
        assert!(registry
            .queue_outbound_state(dj_truth_state(None, None, None))
            .unwrap());
        let delivered = drain_dj_truth_queue(&receiver);
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].play_session_id, None);
        assert_eq!(delivered[0].pedal_owner, None);
        assert_eq!(delivered[0].release_event_id, None);
        registry.note_outbound_state(&delivered[0], &delivered[0].event_id, delivered[0].sequence);
        assert!(!registry
            .queue_outbound_state(dj_truth_state(None, None, None))
            .unwrap());
        assert!(drain_dj_truth_queue(&receiver).is_empty());
    }

    #[test]
    fn dj_link_delivery_gate_broadcasts_release_handoff_exactly_once_after_note() {
        let now = Instant::now();
        let mut registry = DjLinkRegistry::default();
        let generation =
            registry.register_test_session(DJ_V3_AGENT, "gate-session", 1, "gate-peer", now);
        {
            let session = registry.sessions.get_mut(DJ_V3_AGENT).unwrap();
            session.state_sync_received = true;
            session.timeline_state_request_received = true;
            session.snapshot_ready = true;
        }
        let baseline = dj_truth_state(Some("play-1"), Some("pedal-1"), None);
        registry.note_outbound_state(&baseline, "evt-baseline", 10);

        // A replayed ACK identity over identical truth must not re-deliver.
        let replay = DjLinkTimelineState {
            event_id: "evt-replayed-ack".to_string(),
            sequence: 11,
            ..baseline.clone()
        };
        assert!(!registry.should_deliver_outbound_state(
            DJ_V3_AGENT,
            "gate-session",
            generation,
            &replay
        ));

        // Ownership-only Release handoff passes the worker delivery gate
        // exactly once and then latches until the next truth change.
        let handoff = dj_truth_state(Some("play-1"), Some("pedal-2"), Some("release-5"));
        assert!(registry.should_deliver_outbound_state(
            DJ_V3_AGENT,
            "gate-session",
            generation,
            &handoff
        ));
        registry.note_outbound_state(&handoff, "evt-handoff", 12);
        assert!(!registry.should_deliver_outbound_state(
            DJ_V3_AGENT,
            "gate-session",
            generation,
            &handoff
        ));
        let retained = registry.sessions[DJ_V3_AGENT]
            .last_outbound_state
            .clone()
            .expect("delivery gate must retain authoritative truth");
        assert_eq!(retained.play_session_id.as_deref(), Some("play-1"));
        assert_eq!(retained.pedal_owner.as_deref(), Some("pedal-2"));
        assert_eq!(retained.release_event_id.as_deref(), Some("release-5"));

        let released = dj_truth_state(None, None, None);
        assert!(registry.should_deliver_outbound_state(
            DJ_V3_AGENT,
            "gate-session",
            generation,
            &released
        ));
    }

    #[test]
    fn dj_link_server_drop_unblocks_stalled_peer_and_reaps_workers() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let snapshot = Arc::new(Mutex::new(EngineSnapshot::default()));
        let snapshot_provider = {
            let snapshot = Arc::clone(&snapshot);
            move || snapshot.lock().unwrap().clone()
        };
        let handler = dj_v3_state_responder(1);
        let mut server =
            RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link(
                RemoteControlConfig {
                    bind_ip: "127.0.0.1".to_string(),
                    port,
                    pairing_pin: "123456".to_string(),
                    max_connections: 2,
                    dj_link_enabled: true,
                    dj_link_bind_ip: Some("127.0.0.1".to_string()),
                    dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                    ..RemoteControlConfig::default()
                },
                |_| {},
                snapshot_provider,
                (
                    VideoRuntimeStatus::default,
                    default_video_output_render_plans,
                    default_external_video_io_plans,
                    default_external_video_transport_status,
                    default_external_video_transport_sync,
                ),
                Some(handler),
            )
            .unwrap();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut stalled, _) = (0..100)
            .find_map(|_| tungstenite::connect(url.as_str()).ok())
            .expect("DJ Link listener did not accept the stalled-peer socket");
        complete_dj_v3_snapshot_gate(&mut stalled, port, "stalled-session", &server);
        assert_eq!(
            server.dj_link_status().session_id.as_deref(),
            Some("stalled-session")
        );
        {
            let mut changed = snapshot.lock().unwrap();
            changed.timeline.playing = true;
            changed.timeline.position_ms = 4_000;
            changed.timeline.duration_ms = 20_000;
        }
        for _ in 0..100 {
            if server.dj_link_status().last_outbound_delivery.as_deref() == Some("delivered") {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(server.dj_link_connections.load(Ordering::Acquire), 1);
        assert_eq!(
            server.dj_link_status().last_outbound_delivery.as_deref(),
            Some("delivered")
        );

        // The peer deliberately stops reading.  Shutdown must close the
        // cloned socket first, then join both the listener and client worker;
        // it must not wait for an unbounded websocket write/read.
        let started = Instant::now();
        server.stop_and_join();
        assert!(started.elapsed() < DJ_LINK_SHUTDOWN_DEADLINE);
        assert_eq!(server.dj_link_connections.load(Ordering::Acquire), 0);
        assert!(server.client_workers.lock().unwrap().is_empty());
        assert!(server.shutdown_sockets.lock().unwrap().is_empty());
        assert!(server.dj_link_registry.lock().unwrap().sessions.is_empty());
        let _ = stalled.close(None);
    }

    #[test]
    fn dj_link_registry_busy_retry_expiry_rollback_and_session_aba_are_fail_closed() {
        let now = Instant::now();
        let mut registry = DjLinkRegistry::default();
        let old_generation =
            registry.register_test_session(DJ_V3_AGENT, "old-session", 1, "old-peer", now);
        let new_generation =
            registry.register_test_session(DJ_V3_AGENT, "new-session", 1, "new-peer", now);
        assert!(!registry.is_current(DJ_V3_AGENT, "old-session", old_generation));
        assert!(registry.is_current(DJ_V3_AGENT, "new-session", new_generation));
        registry.close_if_current(DJ_V3_AGENT, "old-session", old_generation);
        assert!(registry.is_current(DJ_V3_AGENT, "new-session", new_generation));
        // A side-effectful command exercises the idempotency tombstones;
        // heartbeats deliberately bypass them (no physical side effect).
        registry
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;

        let envelope = dj_v3_envelope(
            DjLinkMessageType::TimelineLoopSet,
            "new-session",
            2,
            "busy-event",
            dj_v3_loop_set_payload(true),
        );
        let shape = envelope.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&envelope, &shape, new_generation, now)
                .unwrap(),
            DjLinkAdmission::Accepted
        );
        assert_eq!(
            registry
                .admit(&envelope, &shape, new_generation, now)
                .unwrap(),
            DjLinkAdmission::Busy
        );
        let mut conflict = envelope.clone();
        conflict.sequence = 3;
        let conflict_shape = conflict.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&conflict, &conflict_shape, new_generation, now)
                .unwrap(),
            DjLinkAdmission::Conflict
        );
        registry.abort_inflight(&envelope, new_generation);
        assert_eq!(
            registry
                .admit(&envelope, &shape, new_generation, now)
                .unwrap(),
            DjLinkAdmission::Accepted
        );
        let ack = dj_link_ack(&envelope, DjLinkAckOutcome::Accepted, None, 2);
        registry
            .complete(&envelope, new_generation, shape, ack, now)
            .unwrap();
        registry
            .terminals
            .get_mut(&(envelope.agent_id.clone(), envelope.event_id.clone()))
            .unwrap()
            .expires_at = now.checked_sub(DJ_LINK_TERMINAL_TTL).unwrap_or(now);
        registry.purge_terminals(now);
        assert_eq!(
            registry
                .admit(
                    &envelope,
                    &envelope.canonical_shape().unwrap(),
                    new_generation,
                    now,
                )
                .unwrap(),
            DjLinkAdmission::ReplayNotRetained
        );

        let rollback = DjLinkEnvelope {
            event_id: "rollback-event".to_string(),
            sequence: 1,
            ..envelope
        };
        let rollback_shape = rollback.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&rollback, &rollback_shape, new_generation, now)
                .unwrap(),
            DjLinkAdmission::Rollback
        );
    }

    #[test]
    fn dj_link_transport_requires_exact_dj_link_path_and_preserves_web_remote_ws() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let server = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
        )
        .unwrap();

        // With DJ Link enabled, a bare generic /ws must stay on the Web
        // Remote pairing contract: it is rejected before any WebSocket
        // upgrade instead of being reinterpreted as the DJ transport.
        let bare = format!("ws://127.0.0.1:{port}/ws");
        assert!(tungstenite::connect(bare.as_str()).is_err());
        for _ in 0..20 {
            if server.status().clients.is_empty() && !server.dj_link_status().connected {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(server.status().clients.is_empty());
        assert!(!server.dj_link_status().connected);

        let read_ack = |client: &mut tungstenite::WebSocket<_>| -> DjLinkAck {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };
        let hello = |event_id: &str, auth_token: &str| {
            json!({
                "v": 3,
                "type": "DJ_AGENT_HELLO",
                "agentId": DJ_V3_AGENT,
                "sessionId": "session-routing",
                "sequence": 1,
                "eventId": event_id,
                "payload": {"authToken": auth_token, "version": 3, "capabilities": dj_v3_capabilities()}
            })
        };

        // Only the exact /dj-link target reaches the authenticated DJ
        // handshake, and it rejects bad tokens without registering a session.
        let dj_url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut wrong_client, _) = tungstenite::connect(dj_url.as_str()).unwrap();
        wrong_client
            .send(Message::Text(
                hello("hello-wrong-token", "0123456789abcdef0123456789abcdee").to_string(),
            ))
            .unwrap();
        let wrong_ack = read_ack(&mut wrong_client);
        assert_eq!(wrong_ack.outcome, DjLinkAckOutcome::Rejected);
        assert_eq!(wrong_ack.code.as_deref(), Some("invalid_auth"));
        let _ = wrong_client.close(None);
        for _ in 0..20 {
            if !server.dj_link_status().connected {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!server.dj_link_status().connected);

        let (mut dj_client, _) = tungstenite::connect(dj_url.as_str()).unwrap();
        dj_client
            .send(Message::Text(
                hello("hello-routing", DJ_V3_TOKEN).to_string(),
            ))
            .unwrap();
        assert_eq!(read_ack(&mut dj_client).outcome, DjLinkAckOutcome::Accepted);
        assert_eq!(
            server.dj_link_status().agent_id.as_deref(),
            Some(DJ_V3_AGENT)
        );
        let _ = dj_client.close(None);
        drop(server);

        // Without DJ Link, /dj-link is refused outright while the generic
        // /ws endpoint keeps serving the pairing-token Web Remote.
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let server = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                ..RemoteControlConfig::default()
            },
            |_| {},
        )
        .unwrap();
        let dj_disabled = format!("ws://127.0.0.1:{port}/dj-link");
        assert!(tungstenite::connect(dj_disabled.as_str()).is_err());
        let remote_url = format!("ws://127.0.0.1:{port}/ws?token=123456");
        let (mut remote_client, _) = tungstenite::connect(remote_url.as_str()).unwrap();
        for _ in 0..20 {
            if server.status().clients.len() == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(server.status().clients.len(), 1);
        remote_client
            .send(Message::Text(r#"{"type":"getSnapshot"}"#.to_string()))
            .unwrap();
        match remote_client.read().unwrap() {
            Message::Text(text) => {
                assert!(text.contains(r#""type":"snapshot""#));
                assert!(text.contains(r#""ok":true"#));
            }
            other => panic!("unexpected Web Remote reply: {other:?}"),
        }
        let _ = remote_client.close(None);
        drop(server);
    }

    #[test]
    fn dj_link_registry_side_effect_idempotency_is_process_lifetime() {
        let now = Instant::now();
        let mut registry = DjLinkRegistry::default();
        let generation =
            registry.register_test_session(DJ_V3_AGENT, "soak-session", 0, "soak-peer", now);
        registry
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;
        let envelope_of =
            |message_type: DjLinkMessageType, sequence: u64, event_id: &str| DjLinkEnvelope {
                v: protocol::DJ_LINK_PROTOCOL_VERSION,
                message_type,
                agent_id: DJ_V3_AGENT.to_string(),
                session_id: "soak-session".to_string(),
                sequence,
                event_id: event_id.to_string(),
                payload: if message_type == DjLinkMessageType::Heartbeat {
                    json!({})
                } else {
                    dj_v3_loop_set_payload(true)
                },
            };
        let run_accepted = |registry: &mut DjLinkRegistry,
                            message_type: DjLinkMessageType,
                            sequence: u64,
                            event_id: &str|
         -> (DjLinkEnvelope, String, DjLinkAck) {
            let envelope = envelope_of(message_type, sequence, event_id);
            let shape = envelope.canonical_shape().unwrap();
            assert_eq!(
                registry.admit(&envelope, &shape, generation, now).unwrap(),
                DjLinkAdmission::Accepted,
                "admission must never wedge behind retained capacity: {event_id}"
            );
            let ack = dj_link_ack(&envelope, DjLinkAckOutcome::Accepted, None, generation);
            registry
                .complete(&envelope, generation, shape.clone(), ack.clone(), now)
                .unwrap_or_else(|error| {
                    panic!("terminalization must reclaim capacity, not fail: {error}")
                });
            (envelope, shape, ack)
        };

        // Phase 1: far more than the bounded receipt capacity in periodic heartbeats
        // flow through the production admit/complete path without wedging,
        // and because they carry no physical side effect they consume no
        // idempotency capacity at all.
        let mut next_sequence = 1u64;
        for index in 0..(DJ_LINK_TERMINAL_LIMIT * 2) {
            run_accepted(
                &mut registry,
                DjLinkMessageType::Heartbeat,
                next_sequence,
                &format!("soak-heartbeat-{index}"),
            );
            next_sequence += 1;
        }
        assert_eq!(
            registry.seen_event_count(),
            0,
            "heartbeats must not consume idempotency slots"
        );
        assert!(registry.terminal_count() <= DJ_LINK_TERMINAL_LIMIT);

        // Phase 2: side-effectful commands exceed the old 4096-entry cap.
        // Terminal receipts remain bounded, but every process-lifetime
        // identity fence survives.
        let side_effect_count = DJ_LINK_TERMINAL_LIMIT + 64;
        for index in 0..side_effect_count {
            let sequence = next_sequence;
            next_sequence += 1;
            run_accepted(
                &mut registry,
                DjLinkMessageType::TimelineLoopSet,
                sequence,
                &format!("soak-command-{index}"),
            );
        }
        assert_eq!(registry.seen_event_count(), side_effect_count);
        assert!(registry.terminal_count() <= DJ_LINK_TERMINAL_LIMIT);
        assert!(
            registry.seen_event_contains(registry.identity_digest(&envelope_of(
                DjLinkMessageType::TimelineLoopSet,
                1,
                "soak-command-0"
            ))),
            "the oldest side-effect tombstone must survive hard pressure"
        );
        assert!(
            registry.seen_event_contains(registry.identity_digest(&envelope_of(
                DjLinkMessageType::TimelineLoopSet,
                1,
                &format!("soak-command-{}", side_effect_count - 1),
            ))),
            "the newest tombstone must survive"
        );

        // Even though the oldest exact receipt was evicted, replaying its
        // physical identity at a brand-new higher sequence cannot execute.
        let newest_side_effect_sequence = next_sequence - 1;
        let oldest_high_sequence = DjLinkEnvelope {
            sequence: next_sequence,
            event_id: "soak-command-0".to_string(),
            ..envelope_of(DjLinkMessageType::TimelineLoopSet, next_sequence, "unused")
        };
        next_sequence += 1;
        assert_eq!(
            registry
                .admit(
                    &oldest_high_sequence,
                    &oldest_high_sequence.canonical_shape().unwrap(),
                    generation,
                    now,
                )
                .unwrap(),
            DjLinkAdmission::ReplayNotRetained,
            "oldest physical identity must remain fenced after more than 4096 events"
        );

        // Exact active replay before the retention boundary returns the
        // stored receipt without re-execution.  The newest phase-2 command
        // still holds a live terminal receipt.
        let recent_sequence = newest_side_effect_sequence;
        let recent_envelope = envelope_of(
            DjLinkMessageType::TimelineLoopSet,
            recent_sequence,
            &format!("soak-command-{}", side_effect_count - 1),
        );
        let recent_shape = recent_envelope.canonical_shape().unwrap();
        let recent_ack = dj_link_ack(
            &recent_envelope,
            DjLinkAckOutcome::Accepted,
            None,
            generation,
        );
        let recent_key = (DJ_V3_AGENT.to_string(), recent_envelope.event_id.clone());
        match registry
            .admit(&recent_envelope, &recent_shape, generation, now)
            .unwrap()
        {
            DjLinkAdmission::Duplicate(receipt) => assert_eq!(receipt, recent_ack),
            other => panic!("expected exact terminal replay, got {other:?}"),
        }

        // After the terminal receipt expires the tombstone still refuses the
        // replay: the real admission entry point purges first, then rejects
        // the same shape as a non-retained replay. No test-only/manual purge
        // is allowed to make this proof pass.
        registry.terminals.get_mut(&recent_key).unwrap().expires_at = now;
        assert_eq!(
            registry
                .admit(&recent_envelope, &recent_shape, generation, now)
                .unwrap(),
            DjLinkAdmission::ReplayNotRetained
        );
        let changed_shape_replay = DjLinkEnvelope {
            sequence: next_sequence,
            payload: json!({
                "active": false,
                "timelineId": "7",
                "playSessionId": "play-1",
            }),
            ..recent_envelope.clone()
        };
        next_sequence += 1;
        assert_eq!(
            registry
                .admit(
                    &changed_shape_replay,
                    &changed_shape_replay.canonical_shape().unwrap(),
                    generation,
                    now,
                )
                .unwrap(),
            DjLinkAdmission::Conflict,
            "changed-shape identity reuse stays conflict after receipt TTL"
        );

        // Advancing far beyond the former TTL horizon never removes the
        // process-lifetime tombstone. A higher sequence still cannot execute.
        let after_old_ttl = DjLinkEnvelope {
            sequence: next_sequence,
            ..recent_envelope.clone()
        };
        next_sequence += 1;
        assert_eq!(
            registry
                .admit(
                    &after_old_ttl,
                    &after_old_ttl.canonical_shape().unwrap(),
                    generation,
                    now + DJ_LINK_TERMINAL_TTL * 4,
                )
                .unwrap(),
            DjLinkAdmission::ReplayNotRetained
        );

        // A brand-new identity remains admissible after every reclaim path.
        run_accepted(
            &mut registry,
            DjLinkMessageType::TimelineLoopSet,
            next_sequence,
            "soak-after-retention",
        );

        // Reconnection resets sequence ordering but not physical identity.
        let replacement_generation =
            registry.register_test_session(DJ_V3_AGENT, "soak-replacement", 1, "soak-peer-2", now);
        assert!(replacement_generation > generation);
        registry
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;
        let reconnect_replay = DjLinkEnvelope {
            session_id: "soak-replacement".to_string(),
            sequence: 2,
            event_id: "soak-command-0".to_string(),
            ..envelope_of(DjLinkMessageType::TimelineLoopSet, 2, "unused")
        };
        assert_eq!(
            registry
                .admit(
                    &reconnect_replay,
                    &reconnect_replay.canonical_shape().unwrap(),
                    replacement_generation,
                    now,
                )
                .unwrap(),
            DjLinkAdmission::ReplayNotRetained,
            "session replacement must not reset physical side-effect idempotency"
        );

        // Stale-generation fail-closed: an admitted-but-not-terminalized
        // identity loses its session to a replacement; reject_inflight
        // tombstones it so it cannot be laundered through the replacement's
        // fresh per-session sequence floor.
        let stale = envelope_of(DjLinkMessageType::TimelineLoopSet, 3, "soak-stale");
        let stale = DjLinkEnvelope {
            session_id: "soak-replacement".to_string(),
            ..stale
        };
        let stale_shape = stale.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&stale, &stale_shape, replacement_generation, now)
                .unwrap(),
            DjLinkAdmission::Accepted,
            "the stale identity must be inflight before the replacement"
        );
        let final_generation =
            registry.register_test_session(DJ_V3_AGENT, "soak-final", 1, "soak-peer-3", now);
        assert!(final_generation > replacement_generation);
        registry.reject_inflight(&stale, replacement_generation);
        assert!(registry.inflight.is_empty());
        assert!(
            registry.seen_event_contains(registry.identity_digest(&stale)),
            "a lease-lost identity must be tombstoned"
        );
        let stale_replay = DjLinkEnvelope {
            session_id: "soak-final".to_string(),
            sequence: 2,
            ..stale.clone()
        };
        assert_eq!(
            registry
                .admit(
                    &stale_replay,
                    &stale_replay.canonical_shape().unwrap(),
                    final_generation,
                    now,
                )
                .unwrap(),
            DjLinkAdmission::ReplayNotRetained,
            "a tombstoned stale-generation identity cannot re-execute in the replacement session"
        );
    }

    #[test]
    fn dj_link_side_effect_high_water_is_one_hour_sized_and_latches_fail_closed() {
        const {
            assert!(
                DJ_LINK_SIDE_EFFECT_EVENT_LIMIT >= 60 * 60 * 60,
                "fixed capacity covers one hour at the normal/default 60 frame/s rate"
            );
            assert!(
                DJ_LINK_SIDE_EFFECT_EVENT_LIMIT < 1_000 * 60 * 60,
                "configured max-rate availability is intentionally bounded below one hour"
            );
        }

        let now = Instant::now();
        let mut registry = DjLinkRegistry::with_side_effect_event_limit(3);
        assert_eq!(
            std::mem::size_of::<(u64, u64)>(),
            16,
            "each permanent tombstone payload is two compact digests"
        );
        let max_identity = ("a".repeat(256), "e".repeat(256));
        let max_shape = format!(r#"{{"payload":"{}"}}"#, "x".repeat(64 * 1_024 - 14));
        let compact_tombstone = {
            let fence = registry.process_fence.lock().unwrap();
            (
                DjLinkRegistry::keyed_digest(&fence.identity_hasher, &max_identity),
                DjLinkRegistry::keyed_digest(&fence.shape_hasher, &max_shape),
            )
        };
        assert_eq!(std::mem::size_of_val(&compact_tombstone), 16);
        assert_eq!(registry.seen_event_count(), 0);
        drop((max_identity, max_shape));
        let generation =
            registry.register_test_session(DJ_V3_AGENT, "cap-session", 0, "cap-peer", now);
        registry
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;
        let physical = |sequence: u64, event_id: &str| {
            dj_v3_envelope(
                DjLinkMessageType::TimelineLoopSet,
                "cap-session",
                sequence,
                event_id,
                dj_v3_loop_set_payload(true),
            )
        };

        // Reserve the complete high-water mark concurrently. Admission must
        // count in-flight physical work as well as completed tombstones, so a
        // burst cannot overrun the bound before completion records identities.
        let admitted: Vec<_> = (1..=3)
            .map(|sequence| {
                let envelope = physical(sequence, &format!("cap-{sequence}"));
                let shape = envelope.canonical_shape().unwrap();
                assert_eq!(
                    registry.admit(&envelope, &shape, generation, now).unwrap(),
                    DjLinkAdmission::Accepted
                );
                (envelope, shape)
            })
            .collect();
        let over_limit = physical(4, "cap-over-limit");
        assert_eq!(
            registry
                .admit(
                    &over_limit,
                    &over_limit.canonical_shape().unwrap(),
                    generation,
                    now,
                )
                .unwrap(),
            DjLinkAdmission::SideEffectCapacityLatched
        );
        assert!(registry.side_effect_capacity_latched());
        assert_eq!(registry.seen_event_count(), 0);

        for (envelope, shape) in &admitted {
            registry
                .complete(
                    envelope,
                    generation,
                    shape.clone(),
                    dj_link_ack(envelope, DjLinkAckOutcome::Accepted, None, generation),
                    now,
                )
                .unwrap();
        }
        assert_eq!(registry.seen_event_count(), 3);

        // Repeated new identities cannot grow the permanent map once latched.
        for sequence in 5..=32 {
            let envelope = physical(sequence, &format!("cap-rejected-{sequence}"));
            assert_eq!(
                registry
                    .admit(
                        &envelope,
                        &envelope.canonical_shape().unwrap(),
                        generation,
                        now,
                    )
                    .unwrap(),
                DjLinkAdmission::SideEffectCapacityLatched
            );
        }
        assert_eq!(registry.seen_event_count(), 3);
        assert!(registry.inflight.is_empty());

        // Existing identities retain their exact receipt/replay semantics even
        // while new physical identities are latched out.
        let (known, known_shape) = &admitted[0];
        assert!(matches!(
            registry.admit(known, known_shape, generation, now).unwrap(),
            DjLinkAdmission::Duplicate(_)
        ));
        let known_key = (known.agent_id.clone(), known.event_id.clone());
        registry.terminals.get_mut(&known_key).unwrap().expires_at = now;
        assert_eq!(
            registry.admit(known, known_shape, generation, now).unwrap(),
            DjLinkAdmission::ReplayNotRetained
        );
        assert_eq!(registry.seen_event_count(), 3);

        // Session replacement does not reset the latch. Non-side-effect
        // keepalive traffic remains operational and consumes no tombstones.
        let replacement_generation =
            registry.register_test_session(DJ_V3_AGENT, "cap-replacement", 0, "cap-peer-2", now);
        registry
            .sessions
            .get_mut(DJ_V3_AGENT)
            .unwrap()
            .snapshot_ready = true;
        let reconnect_physical = DjLinkEnvelope {
            session_id: "cap-replacement".to_string(),
            sequence: 1,
            event_id: "cap-after-reconnect".to_string(),
            ..physical(1, "unused")
        };
        assert_eq!(
            registry
                .admit(
                    &reconnect_physical,
                    &reconnect_physical.canonical_shape().unwrap(),
                    replacement_generation,
                    now,
                )
                .unwrap(),
            DjLinkAdmission::SideEffectCapacityLatched
        );
        let heartbeat = DjLinkEnvelope {
            message_type: DjLinkMessageType::Heartbeat,
            event_id: "cap-heartbeat".to_string(),
            payload: json!({}),
            ..reconnect_physical
        };
        let heartbeat_shape = heartbeat.canonical_shape().unwrap();
        assert_eq!(
            registry
                .admit(&heartbeat, &heartbeat_shape, replacement_generation, now,)
                .unwrap(),
            DjLinkAdmission::Accepted
        );
        registry
            .complete(
                &heartbeat,
                replacement_generation,
                heartbeat_shape,
                dj_link_ack(
                    &heartbeat,
                    DjLinkAckOutcome::Accepted,
                    None,
                    replacement_generation,
                ),
                now,
            )
            .unwrap();
        assert_eq!(registry.seen_event_count(), 3);
    }

    #[test]
    fn dj_link_transport_survives_periodic_heartbeat_flood_beyond_capacity() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let dispatched = Arc::new(AtomicU64::new(0));
        let handler: DjLinkDispatchHandler = {
            let dispatched = Arc::clone(&dispatched);
            let inner = dj_v3_state_responder(5);
            Arc::new(move |envelope| {
                if envelope.message_type == DjLinkMessageType::TrackActive {
                    dispatched.fetch_add(1, Ordering::SeqCst);
                }
                inner(envelope)
            })
        };
        let server = RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                max_connections: 1,
                max_messages_per_second: 1000,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
            EngineSnapshot::default,
            (
                VideoRuntimeStatus::default,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
            Some(handler),
        )
        .unwrap();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(&mut client, port, "session-flood", &server);
        let read_ack = |client: &mut tungstenite::WebSocket<_>| -> DjLinkAck {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };

        // More than the bounded terminal-receipt capacity in periodic heartbeats on the real
        // socket: the transport must keep answering every frame and never
        // answer with a capacity refusal.  Frames above the configured rate
        // window may legitimately receive typed rate_limit rejections; the
        // invariant under test is liveness without permanent capacity death.
        let total = DJ_LINK_TERMINAL_LIMIT + 128;
        for index in 0..total {
            client
                .send(Message::Text(
                    json!({
                        "v": 3,
                        "type": "DJ_HEARTBEAT",
                        "agentId": DJ_V3_AGENT,
                        "sessionId": "session-flood",
                        "sequence": index + 4,
                        "eventId": format!("flood-heartbeat-{index}"),
                        "payload": {}
                    })
                    .to_string(),
                ))
                .unwrap();
            let ack = read_ack(&mut client);
            assert_eq!(ack.event_id, format!("flood-heartbeat-{index}"));
            assert_ne!(
                ack.code.as_deref(),
                Some("terminal_capacity"),
                "periodic traffic must never exhaust admission"
            );
            assert_ne!(ack.outcome, DjLinkAckOutcome::Busy);
        }
        assert!(server.dj_link_status().connected);
        {
            let registry = server.dj_link_registry.lock().unwrap();
            assert_eq!(registry.seen_event_count(), 0);
            assert!(registry.terminal_count() <= DJ_LINK_TERMINAL_LIMIT);
        }

        // Allow the per-second rate window to reset, then prove the
        // transport still executes a physical command exactly once with an
        // exact duplicate receipt.
        thread::sleep(Duration::from_millis(1100));
        let command = json!({
            "v": 3,
            "type": "DJ_TRACK_ACTIVE",
            "agentId": DJ_V3_AGENT,
            "sessionId": "session-flood",
            "sequence": total + 4,
            "eventId": "flood-command",
            "payload": dj_v3_generic_track_active_payload("play-flood"),
        });
        client.send(Message::Text(command.to_string())).unwrap();
        assert_eq!(read_ack(&mut client).outcome, DjLinkAckOutcome::Accepted);
        assert_eq!(dispatched.load(Ordering::SeqCst), 1);
        client.send(Message::Text(command.to_string())).unwrap();
        assert_eq!(read_ack(&mut client).outcome, DjLinkAckOutcome::Duplicate);
        assert_eq!(dispatched.load(Ordering::SeqCst), 1);
        {
            let registry = server.dj_link_registry.lock().unwrap();
            assert!(registry.inflight.is_empty());
        }
        let _ = client.close(None);
        drop(server);
    }

    #[test]
    fn dj_link_socket_rate_limited_physical_flood_retries_without_fence_consumption() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let dispatched = Arc::new(AtomicU64::new(0));
        let handler: DjLinkDispatchHandler = {
            let dispatched = Arc::clone(&dispatched);
            let inner = dj_v3_state_responder(11);
            Arc::new(move |envelope| {
                if !matches!(
                    envelope.message_type,
                    DjLinkMessageType::TimelineStateRequest | DjLinkMessageType::StateSync
                ) {
                    dispatched.fetch_add(1, Ordering::SeqCst);
                }
                inner(envelope)
            })
        };
        let server = RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                max_connections: 1,
                max_messages_per_second: 3,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(DJ_V3_TOKEN.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
            EngineSnapshot::default,
            (
                VideoRuntimeStatus::default,
                default_video_output_render_plans,
                default_external_video_io_plans,
                default_external_video_transport_status,
                default_external_video_transport_sync,
            ),
            Some(handler),
        )
        .unwrap();
        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();
        complete_dj_v3_snapshot_gate(&mut client, port, "session-rate-socket", &server);
        let read_ack = |client: &mut tungstenite::WebSocket<_>| -> DjLinkAck {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };
        let frame = |sequence: u64, event_id: &str| {
            json!({
                "v": 3,
                "type": "DJ_TIMELINE_LOOP_SET",
                "agentId": DJ_V3_AGENT,
                "sessionId": "session-rate-socket",
                "sequence": sequence,
                "eventId": event_id,
                "payload": {
                    "active": true,
                    "timelineId": "7",
                    "playSessionId": "play-session-rate-socket",
                },
            })
        };

        // The gate consumed hello-exempt window slots one and two; the first
        // physical frame is the third message in the same window and is
        // admitted exactly once.
        let first = frame(4, "rate-socket-first");
        client.send(Message::Text(first.to_string())).unwrap();
        assert_eq!(read_ack(&mut client).outcome, DjLinkAckOutcome::Accepted);
        assert_eq!(dispatched.load(Ordering::SeqCst), 1);

        // Every unique over-limit physical frame receives a non-terminal
        // rate rejection. It must not consume the permanent identity cap.
        for sequence in 5..=12 {
            let event_id = format!("rate-socket-flood-{sequence}");
            client
                .send(Message::Text(frame(sequence, &event_id).to_string()))
                .unwrap();
            let ack = read_ack(&mut client);
            assert_eq!(
                ack.outcome,
                DjLinkAckOutcome::Rejected,
                "sequence {sequence} received {ack:?}"
            );
            assert_eq!(
                ack.code.as_deref(),
                Some("rate_limit"),
                "sequence {sequence} received {ack:?}"
            );
        }
        assert_eq!(dispatched.load(Ordering::SeqCst), 1);
        {
            let registry = server.dj_link_registry.lock().unwrap();
            assert_eq!(registry.seen_event_count(), 1);
            assert!(registry.inflight.is_empty());
            assert!(!registry.side_effect_capacity_latched());
        }

        // Once the rate window rolls over, the exact unique frame is safely
        // retryable and executes once; its retry then replays the terminal ACK.
        thread::sleep(Duration::from_millis(1_100));
        let retried = frame(5, "rate-socket-flood-5");
        client.send(Message::Text(retried.to_string())).unwrap();
        assert_eq!(read_ack(&mut client).outcome, DjLinkAckOutcome::Accepted);
        assert_eq!(dispatched.load(Ordering::SeqCst), 2);
        client.send(Message::Text(retried.to_string())).unwrap();
        assert_eq!(read_ack(&mut client).outcome, DjLinkAckOutcome::Duplicate);
        assert_eq!(dispatched.load(Ordering::SeqCst), 2);
        let _ = client.close(None);
        drop(server);
    }

    fn http_get_response(port: u16, path: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .write_all(
                format!(
                    "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
                )
                .as_bytes(),
            )
            .unwrap();
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        response
    }

    fn http_eventually_returns(port: u16, path: &str, expected_prefix: &str) -> bool {
        for _ in 0..50 {
            if http_get_response(port, path).starts_with(expected_prefix) {
                return true;
            }
            thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn remote_server_rejects_no_enabled_transport() {
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let disabled = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                web_remote_enabled: false,
                dj_link_enabled: false,
                ..RemoteControlConfig::default()
            },
            |_| {},
        );
        assert!(matches!(disabled, Err(RemoteWsError::NoEnabledTransport)));
        // The failure happens before bind, so the port was never opened.
        assert!(TcpStream::connect(("127.0.0.1", port)).is_err());

        // Either single transport alone still starts.
        let web_only = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port: 0,
                pairing_pin: "123456".to_string(),
                web_remote_enabled: true,
                dj_link_enabled: false,
                ..RemoteControlConfig::default()
            },
            |_| {},
        );
        drop(web_only.unwrap());
        let dj_only = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port: 0,
                pairing_pin: "123456".to_string(),
                web_remote_enabled: false,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some("0123456789abcdef0123456789abcdef".to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
        );
        drop(dj_only.unwrap());
    }

    #[test]
    fn dj_only_listener_allows_empty_pairing_pin_with_valid_dj_authority() {
        let token = "0123456789abcdef0123456789abcdef";
        let server = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port: 0,
                pairing_pin: String::new(),
                web_remote_enabled: false,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(token.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
        )
        .unwrap();
        let port = server.local_addr().port();

        // An empty PIN is legitimate because the generic Web Remote transport
        // is disabled; the DJ authority credential is what gates admission.
        let status = server.status();
        assert!(status.running);
        assert!(!status.web_remote_enabled);
        assert_eq!(status.clients.len(), 0);

        let url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut client, _) = tungstenite::connect(url.as_str()).unwrap();
        client
            .send(Message::Text(
                dj_v3_hello("session-dj-only-empty-pin", "hello-dj-only-empty-pin").to_string(),
            ))
            .unwrap();
        match client.read().unwrap() {
            Message::Text(text) => {
                let ack: DjLinkAck = serde_json::from_str(&text).unwrap();
                assert_eq!(ack.outcome, DjLinkAckOutcome::Accepted);
            }
            other => panic!("unexpected DJ Link reply: {other:?}"),
        }
        assert_eq!(
            server.dj_link_status().agent_id.as_deref(),
            Some(DJ_V3_AGENT)
        );
        assert_eq!(server.status().clients.len(), 0);
        let _ = client.close(None);
        drop(server);
    }

    #[test]
    fn dj_only_listener_accepts_exact_dj_link_and_rejects_all_web_remote_routes() {
        let token = "0123456789abcdef0123456789abcdef";
        let server = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port: 0,
                pairing_pin: String::new(),
                web_remote_enabled: false,
                dj_link_enabled: true,
                dj_link_bind_ip: Some("127.0.0.1".to_string()),
                dj_link_token: Some(token.to_string()),
                ..RemoteControlConfig::default()
            },
            |_| {},
        )
        .unwrap();
        let port = server.local_addr().port();
        assert!(!server.status().web_remote_enabled);

        // The exact /dj-link WebSocket remains the first route and still
        // performs its authenticated HELLO handshake.
        let read_ack = |client: &mut tungstenite::WebSocket<_>| -> DjLinkAck {
            match client.read().unwrap() {
                Message::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected DJ Link reply: {other:?}"),
            }
        };
        let dj_url = format!("ws://127.0.0.1:{port}/dj-link");
        let (mut dj_client, _) = tungstenite::connect(dj_url.as_str()).unwrap();
        dj_client
            .send(Message::Text(
                dj_v3_hello("session-dj-only-routes", "hello-dj-only-routes").to_string(),
            ))
            .unwrap();
        assert_eq!(read_ack(&mut dj_client).outcome, DjLinkAckOutcome::Accepted);
        assert_eq!(
            server.dj_link_status().agent_id.as_deref(),
            Some(DJ_V3_AGENT)
        );

        // Every other WebSocket route is rejected before any generic
        // Host/pairing evaluation, including near-miss /dj-link targets.
        for path in ["/ws", "/ws?token=123456", "/dj-link-extra", "/foo"] {
            let url = format!("ws://127.0.0.1:{port}{path}");
            assert!(
                tungstenite::connect(url.as_str()).is_err(),
                "route {path} must be rejected in DJ-only mode"
            );
        }

        // No generic client ever reaches the registry, while the DJ session
        // itself remains live.
        assert_eq!(server.status().clients.len(), 0);
        assert!(server.dj_link_status().connected);

        // Every non-upgrade HTTP route is a stable 404 in DJ-only mode,
        // including every known Web Remote asset/health route and a plain
        // non-upgrade GET /dj-link.
        for path in [
            "/",
            "/remote",
            "/index.html",
            "/manifest.webmanifest",
            "/icon.svg",
            "/apple-touch-icon.svg",
            "/remote-sw.js",
            "/health",
            "/dj-link",
            "/definitely-missing",
        ] {
            assert!(
                http_eventually_returns(port, path, "HTTP/1.1 404"),
                "route {path} must answer 404 in DJ-only mode"
            );
        }
        assert_eq!(server.status().clients.len(), 0);
        assert!(server.dj_link_status().connected);
        let _ = dj_client.close(None);
        drop(dj_client);
        drop(server);

        // Compatibility control: the same listener shape with the Web Remote
        // enabled keeps its full legacy behavior unchanged.
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let web_server = RemoteWsServer::start(
            RemoteControlConfig {
                bind_ip: "127.0.0.1".to_string(),
                port,
                pairing_pin: "123456".to_string(),
                ..RemoteControlConfig::default()
            },
            |_| {},
        )
        .unwrap();
        assert!(web_server.status().web_remote_enabled);
        assert!(
            http_eventually_returns(port, "/health", "HTTP/1.1 200"),
            "/health must stay available when the Web Remote is enabled"
        );
        let url = format!("ws://127.0.0.1:{port}/ws?token=123456");
        let (mut remote_client, _) = tungstenite::connect(url.as_str()).unwrap();
        let registered_deadline = Instant::now() + Duration::from_secs(1);
        while web_server.status().clients.len() != 1 {
            assert!(
                Instant::now() < registered_deadline,
                "generic Web Remote client did not register"
            );
            thread::sleep(Duration::from_millis(5));
        }
        remote_client
            .send(Message::Text(r#"{"type":"getSnapshot"}"#.to_string()))
            .unwrap();
        match remote_client.read().unwrap() {
            Message::Text(text) => {
                assert!(text.contains(r#""type":"snapshot""#));
                assert!(text.contains(r#""ok":true"#));
            }
            other => panic!("unexpected Web Remote reply: {other:?}"),
        }
        let _ = remote_client.close(None);
        drop(web_server);
    }
}

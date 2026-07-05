use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::{OsStr, OsString},
    fs,
    io::Read,
    net::{IpAddr, UdpSocket},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use engine::{EngineCommand, EngineHandle, FixtureFlagClearKind};
use io::midi::{
    MidiClockEvent, MidiClockInput, MidiControlEvent, MidiControlInput, MidiFeedbackOutput,
};
use io::osc::{OscInput, OscInputEvent};
use io::remote_ws::{RemoteInputEvent, RemoteWsServer};
use io::sacn::is_sacn_multicast_target;
use protocol::{
    canonical_video_output_mapping_field, AttributeControl, AttributeResolution,
    AudioAnalysisSummary, AutomationId, AutomationKeyframeSummary, ClockSnapshot, CompositionId,
    CompositionSummary, CueFixtureTarget, CueId, CueNodeGraphTarget, CustomFixtureProfileFile,
    CustomFixtureProfileRequest, DmxModeSummary, DmxOutputConfig, DmxOutputProtocol, EffectId,
    EffectKind, EffectPreset, EffectSummary, EngineSnapshot, EngineTelemetry, FixtureId,
    FixtureLimits, FixturePreset, FixtureProfileSummary, GeometrySummary, LearnedMidiControl,
    LearnedOscControl, LfoEffectRequest, MidiControlAction, MidiControlMapping, MidiInputSummary,
    MidiOutputSummary, NodeGraphId, NodeGraphNodeKind, NodeGraphPresetFile, NodeGraphSummary,
    NodeGraphTransformOp, OscControlAction, OscControlMapping, OscInputConfig, PatchFixtureRequest,
    PatchedFixtureSummary, PositionWaveEffectRequest, ProjectFile, RemoteControlConfig, Rotation3,
    SerialPortSummary, StageMapConfig, StageMapPresetFile, StageMapPresetSummary, StageObjectId,
    StageObjectKind, StageObjectSummary, TimelineEventId, TimelineTrackKind, Vec3,
    VideoAutomationKeyframeSummary, VideoBackendState, VideoBlendMode, VideoEffectTarget,
    VideoLayerId, VideoLayerState, VideoLayerTarget, VideoOutputId, VideoOutputKind,
    VideoOutputMapping, VideoOutputMappingPresetFile, VideoOutputMappingPresetSummary,
    VideoOutputSummary, VideoOutputTarget, VideoParam, VideoRuntimeStatus, VideoSourceKind,
    VideoSourceSummary,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tauri::Emitter;
use tauri::{Manager, State};

type AppVideoPreviewRenderer =
    video::VideoPreviewRenderer<video::DecoderBackedFrameProvider<video::FfmpegCliFrameDecoder>>;

const APP_NAME: &str = "Rayard";
const PHASE1_SAMPLE_PROJECT_LABEL: &str = "samples/phase1-mini-show.ry";
const PHASE1_SAMPLE_PROJECT_JSON: &str = include_str!("../../../samples/phase1-mini-show.ry");
const SAMPLE_EFFECT_PRESET_PULSE_LABEL: &str = "samples/front-dimmer-pulse.effect";
const SAMPLE_EFFECT_PRESET_PULSE_JSON: &str =
    include_str!("../../../samples/front-dimmer-pulse.effect");
const SAMPLE_EFFECT_PRESET_SHARED_LABEL: &str = "samples/front-dimmer-shared.effect";
const SAMPLE_EFFECT_PRESET_SHARED_JSON: &str =
    include_str!("../../../samples/front-dimmer-shared.effect");
const SAMPLE_EFFECT_PRESET_WAVE_LABEL: &str = "samples/front-dimmer-wave.effect";
const SAMPLE_EFFECT_PRESET_WAVE_JSON: &str =
    include_str!("../../../samples/front-dimmer-wave.effect");
const SAMPLE_EFFECT_PRESET_FLASH_LABEL: &str = "samples/front-dimmer-flash.effect";
const SAMPLE_EFFECT_PRESET_FLASH_JSON: &str =
    include_str!("../../../samples/front-dimmer-flash.effect");
const SAMPLE_EFFECT_PRESET_RANDOM_LABEL: &str = "samples/front-dimmer-random.effect";
const SAMPLE_EFFECT_PRESET_RANDOM_JSON: &str =
    include_str!("../../../samples/front-dimmer-random.effect");
const SAMPLE_EFFECT_PRESET_PERLIN_LABEL: &str = "samples/front-dimmer-perlin.effect";
const SAMPLE_EFFECT_PRESET_PERLIN_JSON: &str =
    include_str!("../../../samples/front-dimmer-perlin.effect");
const SAMPLE_EFFECT_PRESET_CHASE_LABEL: &str = "samples/front-dimmer-chase.effect";
const SAMPLE_EFFECT_PRESET_CHASE_JSON: &str =
    include_str!("../../../samples/front-dimmer-chase.effect");
const SAMPLE_EFFECT_PRESET_BALL_LABEL: &str = "samples/front-dimmer-ball.effect";
const SAMPLE_EFFECT_PRESET_BALL_JSON: &str =
    include_str!("../../../samples/front-dimmer-ball.effect");
const SAMPLE_EFFECT_PRESET_FAN_LABEL: &str = "samples/front-pan-fan.effect";
const SAMPLE_EFFECT_PRESET_FAN_JSON: &str = include_str!("../../../samples/front-pan-fan.effect");
const SAMPLE_EFFECT_PRESET_CIRCLE_PAN_LABEL: &str = "samples/front-circle-pan.effect";
const SAMPLE_EFFECT_PRESET_CIRCLE_PAN_JSON: &str =
    include_str!("../../../samples/front-circle-pan.effect");
const SAMPLE_EFFECT_PRESET_CIRCLE_TILT_LABEL: &str = "samples/front-circle-tilt.effect";
const SAMPLE_EFFECT_PRESET_CIRCLE_TILT_JSON: &str =
    include_str!("../../../samples/front-circle-tilt.effect");
const PHASE1_SMOKE_EXPECTED_FIRST_8: [u8; 8] = [255, 255, 255, 255, 0x80, 0x00, 0x80, 0x00];
const TELEMETRY_DMX_TARGET_FRAME_RATE_HZ: u32 = 44;
const TELEMETRY_DMX_TARGET_TICK_INTERVAL_US: u64 =
    1_000_000 / TELEMETRY_DMX_TARGET_FRAME_RATE_HZ as u64;
const TELEMETRY_TICK_JITTER_P99_TARGET_US: u64 = 1_000;
const TELEMETRY_COMMAND_QUEUE_P99_TARGET_US: u64 = 1_000;
const TELEMETRY_COMMAND_TO_DMX_P99_TARGET_US: u64 = 5_000;
const TELEMETRY_DMX_SEND_INTERVAL_TOLERANCE_US: u64 = 1_000;
const OPEN_PROJECT_EVENT: &str = "rayard://open-project";

fn validate_app_name(file_label: &str, app: &str) -> Result<(), String> {
    if app.trim() == APP_NAME {
        Ok(())
    } else {
        Err(format!("Unsupported {file_label} app '{app}'"))
    }
}

struct AppState {
    engine: EngineHandle,
    video_preview: Mutex<AppVideoPreviewRenderer>,
    external_video_transport: Arc<Mutex<video::ExternalVideoTransportRuntime>>,
    external_video_transport_events: Arc<Mutex<Vec<ExternalVideoTransportDriverEvent>>>,
    custom_profiles: Mutex<HashMap<String, FixtureProfileSummary>>,
    visualizer_model_assets: Mutex<HashMap<String, VisualizerModelAssetCacheEntry>>,
    midi_clock: Mutex<Option<MidiClockInput>>,
    midi_control: Mutex<Option<MidiControlInput>>,
    midi_feedback: Mutex<Option<MidiFeedbackOutput>>,
    osc_input: Mutex<Option<OscInput>>,
    remote_control: Mutex<Option<RemoteWsServer>>,
    pending_project_open_paths: Mutex<Vec<String>>,
    current_project_path: Mutex<Option<PathBuf>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum CueCaptureScope {
    All,
    LightingOnly,
    SelectedFixture {
        #[serde(rename = "fixtureId")]
        fixture_id: FixtureId,
    },
    SelectedGroup {
        #[serde(rename = "groupId")]
        group_id: String,
    },
    VideoOnly,
}

impl Default for CueCaptureScope {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct EffectTargetOverride {
    fixture_ids: Vec<FixtureId>,
    target_group_ids: Vec<String>,
    attribute: String,
    video_targets: Vec<VideoEffectTarget>,
}

#[derive(Debug, Clone, Deserialize)]
struct DmxTestFrameRequest {
    config: DmxOutputConfig,
    channel: u16,
    width: u16,
    value: u16,
}

#[derive(Debug, Clone, Deserialize)]
struct DmxRoutesTestFrameRequest {
    configs: Vec<DmxOutputConfig>,
    channel: u16,
    width: u16,
    value: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct DmxTestFrameResult {
    protocol: DmxOutputProtocol,
    universe: u16,
    channel: u16,
    width: u16,
    value: u8,
    bytes: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct GdtfWheelMediaPayload {
    bytes: Vec<u8>,
    mime_type: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct EngineTelemetryReport {
    version: u8,
    captured_at_unix_ms: u128,
    fixture_count: usize,
    cue_count: usize,
    effect_count: usize,
    node_graph_count: usize,
    video_layer_count: usize,
    video_output_count: usize,
    dmx_output_count: usize,
    enabled_dmx_output_count: usize,
    dmx_preview_universe_count: usize,
    clock: ClockSnapshot,
    primary_output: DmxOutputConfig,
    dmx_outputs: Vec<DmxOutputConfig>,
    budget: EngineTelemetryBudgetReport,
    telemetry: EngineTelemetry,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Phase1SmokeReport {
    path: String,
    cue_id: CueId,
    cue_label: String,
    active_cue_id: Option<CueId>,
    timeline_event_count: usize,
    timeline_automation_count: usize,
    timeline_video_automation_count: usize,
    timeline_duration_ms: u64,
    timeline_probe_ms: u64,
    timeline_probe_dimmer_byte: u8,
    timeline_probe_video_opacity: Option<f32>,
    dmx_output_count: usize,
    enabled_dmx_output_count: usize,
    dmx_preview_universe_count: usize,
    primary_output_label: String,
    video_layer_count: usize,
    video_layer_label: Option<String>,
    video_layer_playing: bool,
    video_layer_opacity: Option<f32>,
    first_8: Vec<u8>,
    expected_first_8: Vec<u8>,
    non_zero_first_8: usize,
    passed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct VideoPreviewQueueSummary {
    layer_id: VideoLayerId,
    label: String,
    source_kind: VideoSourceKind,
    position_ms: u64,
    source_duration_ms: Option<u64>,
    playing: bool,
    effective_speed: f32,
    queue_len: usize,
    expected_queue_len: usize,
    expected_positions_ms: Vec<u64>,
    ready: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct VideoOutputDecodePreviewSummary {
    output_id: VideoOutputId,
    label: String,
    width: u32,
    height: u32,
    enabled: bool,
    blackout: bool,
    report: Option<video::VideoDecodeEnqueueReport>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct VideoPreviewDiagnostics {
    queue_count: usize,
    frame_queue_capacity: usize,
    still_image_cache_len: usize,
    decoder_cache_len: usize,
    prefetch_count: usize,
    prefetch_interval_ms: u64,
    bpm: Option<f32>,
    layer_queues: Vec<VideoPreviewQueueSummary>,
    output_decode_previews: Vec<VideoOutputDecodePreviewSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ExternalVideoTransportDriverEvent {
    sequence: u64,
    action: ExternalVideoTransportDriverAction,
    route: video::ExternalVideoTransportRoute,
    message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
enum ExternalVideoTransportDriverAction {
    Start,
    Stop,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ExternalVideoTransportSyncResponse {
    report: video::ExternalVideoTransportSyncReport,
    events: Vec<ExternalVideoTransportDriverEvent>,
}

const EXTERNAL_VIDEO_TRANSPORT_EVENT_LIMIT: usize = 64;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
enum TelemetryBudgetStatus {
    Pass,
    Warn,
    Fail,
    InsufficientSamples,
    Idle,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct TelemetryBudgetCheck {
    name: String,
    status: TelemetryBudgetStatus,
    measured_us: Option<u64>,
    target_us: Option<u64>,
    samples: u64,
    detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct EngineTelemetryBudgetReport {
    overall: TelemetryBudgetStatus,
    target_dmx_frame_rate_hz: u32,
    target_tick_interval_us: u64,
    tick_jitter_p99_target_us: u64,
    command_queue_p99_target_us: u64,
    command_to_dmx_p99_target_us: u64,
    dmx_send_interval_tolerance_us: u64,
    checks: Vec<TelemetryBudgetCheck>,
}

#[derive(Debug, Clone, PartialEq)]
struct NormalizedVideoOutputConfig {
    label: String,
    kind: VideoOutputKind,
    width: u32,
    height: u32,
    fullscreen: bool,
    monitor_id: Option<u32>,
    endpoint_name: Option<String>,
}

#[derive(Debug, Clone)]
struct PreparedFixturePatch {
    request: PatchFixtureRequest,
    profile: FixtureProfileSummary,
}

#[derive(Debug, Clone, Serialize)]
struct ProjectLoadResult {
    path: String,
    profiles: Vec<FixtureProfileSummary>,
}

#[derive(Debug, Clone, Deserialize)]
struct GdtfShareSearchRequest {
    user: String,
    password: String,
    manufacturer: Option<String>,
    fixture: Option<String>,
    query: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct GdtfShareDownloadRequest {
    user: String,
    password: String,
    rid: Option<u64>,
    uuid: Option<String>,
    manufacturer: String,
    fixture: String,
    revision: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct GdtfShareModeSummary {
    name: String,
    dmx_footprint: Option<u16>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct GdtfShareFixtureSummary {
    rid: Option<u64>,
    uuid: Option<String>,
    manufacturer: String,
    fixture: String,
    revision: String,
    uploader: Option<String>,
    rating: Option<String>,
    version: Option<String>,
    creator: Option<String>,
    filesize: Option<u64>,
    modes: Vec<GdtfShareModeSummary>,
}

#[derive(Debug, Clone)]
struct VisualizerExternalModelAssetGroup {
    asset_key: String,
    profile_source_path: String,
    model_file: String,
    plans: Vec<visualizer::FixtureModelRenderPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisualizerModelAssetCacheEntry {
    file_length: Option<u64>,
    modified_millis: Option<u128>,
    bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize)]
enum VisualizerExternalModelAssetStatus {
    Loaded,
    Missing,
    Skipped,
    Error,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
enum VisualizerExternalModelAssetFormat {
    Glb,
    Gltf,
    Obj,
    ThreeDs,
    Collada,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
struct VisualizerExternalModelAsset {
    asset_key: String,
    profile_source_path: String,
    model_file: String,
    status: VisualizerExternalModelAssetStatus,
    format: VisualizerExternalModelAssetFormat,
    byte_length: usize,
    meshes: Vec<visualizer::FixtureModelPrimitiveMesh>,
    error: Option<String>,
    mesh_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VisualizerResolvedRenderPayload {
    scene: visualizer::VisualizerScene,
    model_render_plans: Vec<visualizer::FixtureModelRenderPlan>,
    primitive_meshes: Vec<visualizer::FixtureModelPrimitiveMesh>,
    external_model_assets: Vec<VisualizerExternalModelAsset>,
    resolved_meshes: Vec<visualizer::FixtureModelPrimitiveMesh>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct VisualizerModelAssetCacheSummary {
    entry_count: usize,
    loaded_count: usize,
    missing_count: usize,
    byte_length: usize,
    limit: usize,
}

#[derive(Debug, Serialize)]
struct FixturePresetGroupLoadResult {
    path: String,
    applied_count: usize,
    skipped_count: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct TimelineGroupAutomationAddResult {
    automation_ids: Vec<AutomationId>,
    applied_count: usize,
    skipped_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CustomAttributeSpec {
    attribute: String,
    resolution: AttributeResolution,
    start_offset: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CustomAttributeLayout {
    attribute: String,
    resolution: AttributeResolution,
    offsets: Vec<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MidiMappingFile {
    version: u32,
    mappings: Vec<MidiControlMapping>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OscMappingFile {
    version: u32,
    mappings: Vec<OscControlMapping>,
}

const VIDEO_FILE_EXTENSIONS: &[&str] = &["mp4", "m4v", "mov", "mkv", "avi", "webm", "hap", "hapq"];
const STILL_IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg"];
const VISUALIZER_MODEL_ASSET_CACHE_LIMIT: usize = 64;
const AUDIO_FILE_EXTENSIONS: &[&str] = &[
    "wav", "mp3", "m4a", "aac", "flac", "aiff", "aif", "ogg", "opus",
];
const ARTNET_MAX_UNIVERSE: u16 = 32_767;
const SACN_MAX_UNIVERSE: u16 = 63_999;

#[tauri::command]
fn select_gdtf_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("GDTF Fixture", &["gdtf"])
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn select_video_source_file(kind: VideoSourceKind) -> Result<Option<String>, String> {
    let (label, extensions) = video_source_file_dialog_filter(&kind)?;
    Ok(rfd::FileDialog::new()
        .add_filter(label, extensions)
        .pick_file()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
fn import_gdtf(state: State<'_, AppState>, path: String) -> Result<FixtureProfileSummary, String> {
    let profile = import_gdtf_from_path(path)?;
    cache_fixture_profile(&state.custom_profiles, &profile)?;
    Ok(profile)
}

#[tauri::command]
fn load_gdtf_wheel_media(
    path: String,
    media: String,
) -> Result<Option<GdtfWheelMediaPayload>, String> {
    if path.starts_with("memory://") || path.starts_with("snapshot://") {
        return Ok(None);
    }
    let mime_type = gdtf_wheel_media_mime_type(&media).to_string();
    gdtf::load_wheel_media(path, &media)
        .map(|bytes| bytes.map(|bytes| GdtfWheelMediaPayload { bytes, mime_type }))
        .map_err(|error| error.to_string())
}

fn gdtf_wheel_media_mime_type(media: &str) -> &'static str {
    let normalized = media.trim().to_ascii_lowercase();
    if normalized.ends_with(".jpg") || normalized.ends_with(".jpeg") {
        "image/jpeg"
    } else if normalized.ends_with(".webp") {
        "image/webp"
    } else if normalized.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "image/png"
    }
}

#[tauri::command]
fn load_gdtf_model_file(
    state: State<'_, AppState>,
    path: String,
    model_file: String,
) -> Result<Option<Vec<u8>>, String> {
    if path.starts_with("memory://") || path.starts_with("snapshot://") {
        return Ok(None);
    }
    load_gdtf_model_file_cached(&state.visualizer_model_assets, &path, &model_file)
}

#[tauri::command]
fn download_gdtf_from_url(url: String) -> Result<Option<String>, String> {
    let trimmed_url = validate_gdtf_download_url(&url)?;
    let Some(path) = rfd::FileDialog::new()
        .add_filter("GDTF Fixture", &["gdtf"])
        .set_file_name(gdtf_download_file_name_from_url(trimmed_url))
        .save_file()
    else {
        return Ok(None);
    };
    let path = normalize_gdtf_save_path(path)?;

    let status = Command::new(curl_binary_name())
        .args([
            "-L",
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            "60",
            "-o",
        ])
        .arg(&path)
        .arg(trimmed_url)
        .status()
        .map_err(|error| format!("Failed to start curl for GDTF download: {error}"))?;

    if !status.success() {
        let _ = fs::remove_file(&path);
        return Err(format!(
            "GDTF download failed with status {}",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "terminated".to_string())
        ));
    }

    gdtf::load_profile(&path).map_err(|error| {
        let _ = fs::remove_file(&path);
        format!("Downloaded file is not a valid GDTF archive: {error}")
    })?;

    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn search_gdtf_share(
    request: GdtfShareSearchRequest,
) -> Result<Vec<GdtfShareFixtureSummary>, String> {
    validate_gdtf_share_credentials(&request.user, &request.password)?;
    let cookie_path = gdtf_share_cookie_path();
    let result = (|| {
        login_gdtf_share(&request.user, &request.password, &cookie_path)?;
        let catalog = fetch_gdtf_share_catalog(&cookie_path)?;
        let fixtures = extract_gdtf_share_fixtures(&catalog);
        if fixtures.is_empty() {
            return Err(
                "GDTF Share catalog response did not contain fixture records Rayard can read"
                    .to_string(),
            );
        }
        Ok(filter_gdtf_share_results(
            fixtures,
            request.manufacturer.as_deref().unwrap_or_default(),
            request.fixture.as_deref().unwrap_or_default(),
            request.query.as_deref().unwrap_or_default(),
            request.limit.unwrap_or(40),
        ))
    })();
    let _ = fs::remove_file(&cookie_path);
    result
}

#[tauri::command]
fn download_gdtf_from_share(request: GdtfShareDownloadRequest) -> Result<Option<String>, String> {
    validate_gdtf_share_credentials(&request.user, &request.password)?;
    if request.rid.is_none()
        && request
            .uuid
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
    {
        return Err("GDTF Share result is missing both rid and uuid".to_string());
    }
    let Some(path) = rfd::FileDialog::new()
        .add_filter("GDTF Fixture", &["gdtf"])
        .set_file_name(gdtf_share_download_file_name(&request))
        .save_file()
    else {
        return Ok(None);
    };
    let path = normalize_gdtf_save_path(path)?;

    let cookie_path = gdtf_share_cookie_path();
    let result = (|| {
        login_gdtf_share(&request.user, &request.password, &cookie_path)?;
        download_gdtf_share_file(&request, &cookie_path, &path)?;
        gdtf::load_profile(&path).map_err(|error| {
            let _ = fs::remove_file(&path);
            format!("Downloaded GDTF Share file is not a valid GDTF archive: {error}")
        })?;
        Ok(Some(path.to_string_lossy().to_string()))
    })();
    let _ = fs::remove_file(&cookie_path);
    result
}

#[tauri::command]
fn create_custom_fixture_profile(
    state: State<'_, AppState>,
    request: CustomFixtureProfileRequest,
) -> Result<FixtureProfileSummary, String> {
    register_custom_fixture_profile(&state, request)
}

#[tauri::command]
fn save_custom_fixture_profile(
    request: CustomFixtureProfileRequest,
) -> Result<Option<String>, String> {
    validate_custom_fixture_profile_request(&request)?;
    let profile_file = CustomFixtureProfileFile {
        version: 1,
        request,
    };
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Fixture Profile", &["fixture"])
        .set_file_name(format!(
            "{}.fixture",
            safe_file_stem(&profile_file.request.name)
        ))
        .save_file()
    else {
        return Ok(None);
    };
    let path = normalize_custom_fixture_profile_save_path(path)?;
    let json = serde_json::to_string_pretty(&profile_file).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn load_custom_fixture_profile(
    state: State<'_, AppState>,
) -> Result<Option<FixtureProfileSummary>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Fixture Profile", &["fixture"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let profile_file: CustomFixtureProfileFile =
        serde_json::from_str(&json).map_err(|error| error.to_string())?;
    validate_custom_fixture_profile_file(&profile_file)?;
    let profile = register_custom_fixture_profile(&state, profile_file.request)?;
    Ok(Some(profile))
}

#[tauri::command]
fn use_fixture_profile(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
) -> Result<FixtureProfileSummary, String> {
    let snapshot = state.engine.snapshot();
    let fixture = snapshot
        .fixtures
        .iter()
        .find(|fixture| fixture.id == fixture_id)
        .ok_or_else(|| format!("Fixture {fixture_id} was not found"))?;
    let profile = fixture_profile_from_patched_fixture(fixture);
    state
        .custom_profiles
        .lock()
        .map_err(|_| "Memory profile state lock was poisoned".to_string())?
        .insert(profile.source_path.clone(), profile.clone());
    Ok(profile)
}

#[tauri::command]
fn patch_fixture(
    state: State<'_, AppState>,
    mut request: PatchFixtureRequest,
) -> Result<FixtureId, String> {
    request.group_ids = normalize_group_ids(request.group_ids)?;
    validate_patch_request(&request)?;
    let profile = load_patch_profile(&state, &request.profile_path)?;
    validate_patch_footprint(&request, &profile)?;
    validate_patch_address_conflicts(&request, &profile, &state.engine.snapshot().fixtures)?;
    let fixture_id = state.engine.allocate_fixture_id();
    state
        .engine
        .send(EngineCommand::PatchFixture {
            fixture_id,
            request,
            profile,
        })
        .map_err(|error| error.to_string())?;
    Ok(fixture_id)
}

#[tauri::command]
fn patch_fixtures(
    state: State<'_, AppState>,
    requests: Vec<PatchFixtureRequest>,
) -> Result<Vec<FixtureId>, String> {
    let prepared = prepare_fixture_patches(&state, requests)?;
    let fixture_ids = prepared
        .iter()
        .map(|_| state.engine.allocate_fixture_id())
        .collect::<Vec<_>>();
    for (fixture_id, prepared_patch) in fixture_ids.iter().copied().zip(prepared) {
        state
            .engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: prepared_patch.request,
                profile: prepared_patch.profile,
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(fixture_ids)
}

#[tauri::command]
fn remove_fixture(state: State<'_, AppState>, fixture_id: FixtureId) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::RemoveFixture(fixture_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_fixture_patch(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
    label: String,
    universe: u16,
    address: u16,
) -> Result<(), String> {
    let label = label.trim().to_string();
    validate_fixture_patch_update(
        &state.engine.snapshot(),
        fixture_id,
        &label,
        universe,
        address,
    )?;
    state
        .engine
        .send(EngineCommand::SetFixturePatch {
            fixture_id,
            label,
            universe,
            address,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_fixture_limits(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
    limits: FixtureLimits,
) -> Result<(), String> {
    validate_fixture_limits(&limits)?;
    let limits = normalize_fixture_limits(limits);
    if !state
        .engine
        .snapshot()
        .fixtures
        .iter()
        .any(|fixture| fixture.id == fixture_id)
    {
        return Err(format!("Fixture {fixture_id} was not found"));
    }
    state
        .engine
        .send(EngineCommand::SetFixtureLimits { fixture_id, limits })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_group_fixture_limits(
    state: State<'_, AppState>,
    group_id: String,
    limits: FixtureLimits,
) -> Result<(), String> {
    validate_fixture_limits(&limits)?;
    let limits = normalize_fixture_limits(limits);
    let group_id = normalize_control_group_id(group_id)?;
    if !state.engine.snapshot().fixtures.iter().any(|fixture| {
        fixture
            .group_ids
            .iter()
            .any(|fixture_group_id| group_matches(fixture_group_id, &group_id))
    }) {
        return Err(format!(
            "Group '{group_id}' was not found or has no fixtures"
        ));
    }
    state
        .engine
        .send(EngineCommand::SetGroupFixtureLimits { group_id, limits })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_fixture_groups(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
    group_ids: Vec<String>,
) -> Result<(), String> {
    let group_ids = normalize_group_ids(group_ids)?;
    state
        .engine
        .send(EngineCommand::SetFixtureGroups {
            fixture_id,
            group_ids,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_attribute(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
    attribute: String,
    value: u16,
) -> Result<(), String> {
    let attribute = normalize_attribute_name(attribute)?;
    state
        .engine
        .send(EngineCommand::SetAttribute {
            fixture_id,
            attribute,
            value,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_group_attribute(
    state: State<'_, AppState>,
    group_id: String,
    attribute: String,
    value: u16,
) -> Result<(), String> {
    let group_id = normalize_control_group_id(group_id)?;
    let attribute = normalize_attribute_name(attribute)?;
    state
        .engine
        .send(EngineCommand::SetGroupAttribute {
            group_id,
            attribute,
            value,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_group_highlight(
    state: State<'_, AppState>,
    group_id: String,
    enabled: bool,
) -> Result<(), String> {
    let group_id = normalize_control_group_id(group_id)?;
    state
        .engine
        .send(EngineCommand::SetGroupHighlight { group_id, enabled })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_group_solo(
    state: State<'_, AppState>,
    group_id: String,
    enabled: bool,
) -> Result<(), String> {
    let group_id = normalize_control_group_id(group_id)?;
    state
        .engine
        .send(EngineCommand::SetGroupSolo { group_id, enabled })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_group_park(
    state: State<'_, AppState>,
    group_id: String,
    enabled: bool,
) -> Result<(), String> {
    let group_id = normalize_control_group_id(group_id)?;
    state
        .engine
        .send(EngineCommand::SetGroupPark { group_id, enabled })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_fixture_transform(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
    position: Vec3,
    rotation: Rotation3,
) -> Result<(), String> {
    validate_fixture_transform(&position, &rotation)?;
    state
        .engine
        .send(EngineCommand::SetFixtureTransform {
            fixture_id,
            position,
            rotation,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_fixture_highlight(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
    enabled: bool,
) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetFixtureHighlight {
            fixture_id,
            enabled,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_fixture_solo(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
    enabled: bool,
) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetFixtureSolo {
            fixture_id,
            enabled,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_fixture_park(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
    enabled: bool,
) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetFixturePark {
            fixture_id,
            enabled,
        })
        .map_err(|error| error.to_string())
}

fn parse_fixture_flag_clear_kind(kind: &str) -> Result<FixtureFlagClearKind, String> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "highlight" => Ok(FixtureFlagClearKind::Highlight),
        "solo" => Ok(FixtureFlagClearKind::Solo),
        "park" => Ok(FixtureFlagClearKind::Park),
        "all" => Ok(FixtureFlagClearKind::All),
        _ => Err("Fixture flag clear kind must be highlight, solo, park, or all".to_string()),
    }
}

#[tauri::command]
fn clear_fixture_flags(state: State<'_, AppState>, kind: String) -> Result<(), String> {
    let kind = parse_fixture_flag_clear_kind(&kind)?;
    state
        .engine
        .send(EngineCommand::ClearFixtureFlags(kind))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_output_config(state: State<'_, AppState>, config: DmxOutputConfig) -> Result<(), String> {
    validate_dmx_output_config(&config)?;
    state
        .engine
        .send(EngineCommand::SetOutput(config))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_dmx_outputs(
    state: State<'_, AppState>,
    configs: Vec<DmxOutputConfig>,
) -> Result<(), String> {
    validate_dmx_output_routes(&configs)?;
    state
        .engine
        .send(EngineCommand::SetDmxOutputs(configs))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn send_dmx_test_frame(request: DmxTestFrameRequest) -> Result<DmxTestFrameResult, String> {
    let frame = build_dmx_test_frame(request.channel, request.width, request.value)?;
    send_dmx_config_test_frame(
        &request.config,
        request.channel,
        request.width,
        request.value,
        &frame,
    )
}

#[tauri::command]
fn send_dmx_routes_test_frame(
    request: DmxRoutesTestFrameRequest,
) -> Result<Vec<DmxTestFrameResult>, String> {
    let frame = build_dmx_test_frame(request.channel, request.width, request.value)?;
    send_dmx_route_test_frames(
        &request.configs,
        request.channel,
        request.width,
        request.value,
        &frame,
    )
}

#[tauri::command]
fn set_blackout(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::Blackout(enabled))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_all_blackout(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetAllBlackout(enabled))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_lighting_master(state: State<'_, AppState>, master: f32) -> Result<(), String> {
    if !master.is_finite() {
        return Err("Lighting master must be finite".to_string());
    }
    state
        .engine
        .send(EngineCommand::SetLightingMaster(master))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_group_submaster(
    state: State<'_, AppState>,
    group_id: String,
    level: f32,
) -> Result<(), String> {
    let group_id = normalize_control_group_id(group_id)?;
    if !level.is_finite() {
        return Err("Submaster level must be finite".to_string());
    }
    state
        .engine
        .send(EngineCommand::SetGroupSubmaster { group_id, level })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn reset_engine_telemetry(state: State<'_, AppState>) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::ResetTelemetry)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_engine_telemetry_report(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let captured_at_unix_ms = current_unix_ms();
    let snapshot = state.engine.snapshot();
    let report = engine_telemetry_report_from_snapshot(&snapshot, captured_at_unix_ms);
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Telemetry Report", &["json"])
        .set_file_name(format!("rayard-telemetry-{captured_at_unix_ms}.json"))
        .save_file()
    else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn get_engine_telemetry_report(
    state: State<'_, AppState>,
) -> Result<EngineTelemetryReport, String> {
    let captured_at_unix_ms = current_unix_ms();
    let snapshot = state.engine.snapshot();
    Ok(engine_telemetry_report_from_snapshot(
        &snapshot,
        captured_at_unix_ms,
    ))
}

#[tauri::command]
fn set_bpm(state: State<'_, AppState>, bpm: f32) -> Result<(), String> {
    if !bpm.is_finite() || !(20.0..=300.0).contains(&bpm) {
        return Err("BPM must be between 20 and 300".to_string());
    }
    state
        .engine
        .send(EngineCommand::SetBpm(bpm))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn tap_bpm(state: State<'_, AppState>) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::TapBpm)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn sync_ableton_link_clock(
    state: State<'_, AppState>,
    bpm: f32,
    beat_phase: f32,
) -> Result<(), String> {
    if !bpm.is_finite() || !(20.0..=300.0).contains(&bpm) {
        return Err("Ableton Link BPM must be between 20 and 300".to_string());
    }
    if !beat_phase.is_finite() {
        return Err("Ableton Link beat phase must be finite".to_string());
    }
    state
        .engine
        .send(EngineCommand::SyncExternalClock {
            bpm,
            beat_phase,
            source: protocol::ClockSource::AbletonLink,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn analyze_audio_file(state: State<'_, AppState>) -> Result<Option<AudioAnalysisSummary>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Audio Files", AUDIO_FILE_EXTENSIONS)
        .pick_file()
    else {
        return Ok(None);
    };
    let analysis = audio::analyze_audio_file(&path).map_err(|error| error.to_string())?;
    state
        .engine
        .send(EngineCommand::SetTimelineAudio(Some(analysis.clone())))
        .map_err(|error| error.to_string())?;
    Ok(Some(analysis))
}

#[tauri::command]
fn clear_timeline_audio(state: State<'_, AppState>) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetTimelineAudio(None))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn list_midi_inputs() -> Result<Vec<MidiInputSummary>, String> {
    io::midi::list_midi_inputs().map_err(|error| error.to_string())
}

#[tauri::command]
fn list_midi_outputs() -> Result<Vec<MidiOutputSummary>, String> {
    io::midi::list_midi_outputs().map_err(|error| error.to_string())
}

#[tauri::command]
fn list_serial_ports() -> Result<Vec<SerialPortSummary>, String> {
    io::serial_dmx::list_serial_ports().map_err(|error| error.to_string())
}

#[tauri::command]
fn connect_midi_clock(state: State<'_, AppState>, input_index: usize) -> Result<(), String> {
    let engine = state.engine.clone();
    let connection = io::midi::connect_midi_clock(input_index, move |event| match event {
        MidiClockEvent::ClockPulse => {
            let _ = engine.send(EngineCommand::MidiClockPulse);
        }
        MidiClockEvent::Start => {
            let _ = engine.send(EngineCommand::SeekTimeline(0));
            let _ = engine.send(EngineCommand::SetTimelinePlaying(true));
        }
        MidiClockEvent::Continue => {
            let _ = engine.send(EngineCommand::SetTimelinePlaying(true));
        }
        MidiClockEvent::Stop => {
            let _ = engine.send(EngineCommand::SetTimelinePlaying(false));
        }
        MidiClockEvent::SongPositionPointer(sixteenth_notes) => {
            let _ = engine.send(EngineCommand::MidiSongPositionPointer(sixteenth_notes));
        }
        MidiClockEvent::Timecode(timecode) => {
            let _ = engine.send(EngineCommand::SyncTimelineTimecode {
                position_ms: timecode.position_ms(),
                source: protocol::ClockSource::MidiTimecode,
            });
        }
    })
    .map_err(|error| error.to_string())?;
    let mut guard = state
        .midi_clock
        .lock()
        .map_err(|_| "MIDI state lock was poisoned".to_string())?;
    *guard = Some(connection);
    Ok(())
}

#[tauri::command]
fn disconnect_midi_clock(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .midi_clock
        .lock()
        .map_err(|_| "MIDI state lock was poisoned".to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
fn connect_midi_control(
    state: State<'_, AppState>,
    input_index: usize,
    mappings: Vec<MidiControlMapping>,
) -> Result<(), String> {
    let mappings = validate_midi_control_mappings(mappings)?;
    if mappings.is_empty() {
        return Err("At least one MIDI control mapping is required".to_string());
    }
    let engine = state.engine.clone();
    let connection = io::midi::connect_midi_control(input_index, mappings, move |event| {
        let command = match event {
            MidiControlEvent::SetAttribute {
                fixture_id,
                attribute,
                value,
            } => EngineCommand::SetAttribute {
                fixture_id,
                attribute,
                value,
            },
            MidiControlEvent::SetFixtureHighlight {
                fixture_id,
                enabled,
            } => EngineCommand::SetFixtureHighlight {
                fixture_id,
                enabled,
            },
            MidiControlEvent::SetFixtureSolo {
                fixture_id,
                enabled,
            } => EngineCommand::SetFixtureSolo {
                fixture_id,
                enabled,
            },
            MidiControlEvent::SetFixturePark {
                fixture_id,
                enabled,
            } => EngineCommand::SetFixturePark {
                fixture_id,
                enabled,
            },
            MidiControlEvent::SetGroupHighlight { group_id, enabled } => {
                EngineCommand::SetGroupHighlight { group_id, enabled }
            }
            MidiControlEvent::SetGroupSolo { group_id, enabled } => {
                EngineCommand::SetGroupSolo { group_id, enabled }
            }
            MidiControlEvent::SetGroupPark { group_id, enabled } => {
                EngineCommand::SetGroupPark { group_id, enabled }
            }
            MidiControlEvent::TriggerCue(cue_id) => EngineCommand::TriggerCue(cue_id),
            MidiControlEvent::TriggerNextCue => EngineCommand::TriggerNextCue,
            MidiControlEvent::TriggerPreviousCue => EngineCommand::TriggerPreviousCue,
            MidiControlEvent::SetEffectEnabled { effect_id, enabled } => {
                EngineCommand::SetEffectEnabled { effect_id, enabled }
            }
            MidiControlEvent::SetNodeGraphEnabled { graph_id, enabled } => {
                EngineCommand::SetNodeGraphEnabled { graph_id, enabled }
            }
            MidiControlEvent::SetVideoParam {
                layer_id,
                param,
                value,
            } => EngineCommand::SetVideoLayerParam {
                layer_id,
                param,
                value,
            },
            MidiControlEvent::AddVideoCuePoint {
                layer_id,
                position_ms,
            } => EngineCommand::AddVideoCuePoint {
                layer_id,
                position_ms,
            },
            MidiControlEvent::RemoveVideoCuePoint {
                layer_id,
                position_ms,
            } => EngineCommand::RemoveVideoCuePoint {
                layer_id,
                position_ms,
            },
            MidiControlEvent::JumpVideoCuePoint {
                layer_id,
                cue_point_index,
            } => EngineCommand::JumpVideoCuePoint {
                layer_id,
                cue_point_index,
            },
            MidiControlEvent::JumpVideoCuePointRelative {
                layer_id,
                direction,
            } => EngineCommand::JumpVideoCuePointRelative {
                layer_id,
                direction,
            },
            MidiControlEvent::SetVideoLayerEnabled { layer_id, enabled } => {
                EngineCommand::SetVideoLayerEnabled { layer_id, enabled }
            }
            MidiControlEvent::SetVideoLayerSolo { layer_id, solo } => {
                EngineCommand::SetVideoLayerSolo { layer_id, solo }
            }
            MidiControlEvent::SetVideoPlaying { layer_id, playing } => {
                EngineCommand::SetVideoLayerPlaying { layer_id, playing }
            }
            MidiControlEvent::SetVideoLoop {
                layer_id,
                enabled,
                loop_start_ms,
                loop_end_ms,
            } => EngineCommand::SetVideoLayerLoop {
                layer_id,
                enabled,
                loop_start_ms,
                loop_end_ms,
            },
            MidiControlEvent::FadeVideoLayerOpacity {
                layer_id,
                opacity,
                duration_ms,
            } => EngineCommand::FadeVideoLayerOpacity {
                layer_id,
                opacity,
                duration_ms,
            },
            MidiControlEvent::SetVideoOutputEnabled { output_id, enabled } => {
                EngineCommand::SetVideoOutputEnabled { output_id, enabled }
            }
            MidiControlEvent::SetVideoOutputOpacity { output_id, opacity } => {
                EngineCommand::SetVideoOutputOpacity { output_id, opacity }
            }
            MidiControlEvent::FadeVideoOutputOpacity {
                output_id,
                opacity,
                duration_ms,
            } => EngineCommand::FadeVideoOutputOpacity {
                output_id,
                opacity,
                duration_ms,
            },
            MidiControlEvent::SetVideoOutputMappingField {
                output_id,
                field,
                value,
            } => EngineCommand::SetVideoOutputMappingField {
                output_id,
                field,
                value,
            },
            MidiControlEvent::ApplyVideoOutputMappingPreset { output_id, label } => {
                EngineCommand::ApplyVideoOutputMappingPreset { output_id, label }
            }
            MidiControlEvent::SetVideoOutputBlackout {
                output_id,
                blackout,
            } => EngineCommand::SetVideoOutputBlackout {
                output_id,
                blackout,
            },
            MidiControlEvent::SetTimelinePlaying(playing) => {
                EngineCommand::SetTimelinePlaying(playing)
            }
            MidiControlEvent::SeekTimeline { position_ms } => {
                EngineCommand::SeekTimeline(position_ms)
            }
            MidiControlEvent::SeekTimelineBeat { direction } => {
                EngineCommand::SeekTimelineBeat { direction }
            }
            MidiControlEvent::SetBpm(bpm) => EngineCommand::SetBpm(bpm),
            MidiControlEvent::TapBpm => EngineCommand::TapBpm,
            MidiControlEvent::LightingMaster(master) => EngineCommand::SetLightingMaster(master),
            MidiControlEvent::SetGroupSubmaster { group_id, level } => {
                EngineCommand::SetGroupSubmaster { group_id, level }
            }
            MidiControlEvent::SetCueFadePaused(paused) => EngineCommand::SetCueFadePaused(paused),
            MidiControlEvent::Blackout(enabled) => EngineCommand::Blackout(enabled),
            MidiControlEvent::AllBlackout(enabled) => EngineCommand::SetAllBlackout(enabled),
            MidiControlEvent::VideoBlackout(enabled) => EngineCommand::SetVideoBlackout(enabled),
            MidiControlEvent::ClearFixtureFlags { kind } => {
                match parse_fixture_flag_clear_kind(&kind) {
                    Ok(kind) => EngineCommand::ClearFixtureFlags(kind),
                    Err(error) => {
                        eprintln!("Ignoring MIDI clear fixture flags command: {error}");
                        return;
                    }
                }
            }
        };
        let _ = engine.send(command);
    })
    .map_err(|error| error.to_string())?;
    let mut guard = state
        .midi_control
        .lock()
        .map_err(|_| "MIDI control state lock was poisoned".to_string())?;
    *guard = Some(connection);
    Ok(())
}

#[tauri::command]
fn disconnect_midi_control(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .midi_control
        .lock()
        .map_err(|_| "MIDI control state lock was poisoned".to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
fn connect_midi_feedback(state: State<'_, AppState>, output_index: usize) -> Result<(), String> {
    let connection =
        io::midi::connect_midi_feedback_output(output_index).map_err(|error| error.to_string())?;
    let mut guard = state
        .midi_feedback
        .lock()
        .map_err(|_| "MIDI feedback state lock was poisoned".to_string())?;
    *guard = Some(connection);
    Ok(())
}

#[tauri::command]
fn disconnect_midi_feedback(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .midi_feedback
        .lock()
        .map_err(|_| "MIDI feedback state lock was poisoned".to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
fn send_midi_feedback(
    state: State<'_, AppState>,
    mappings: Vec<MidiControlMapping>,
) -> Result<usize, String> {
    let mappings = validate_midi_control_mappings(mappings)?;
    let snapshot = state.engine.snapshot();
    let messages = io::midi::build_feedback_messages(&snapshot, &mappings);
    let mut guard = state
        .midi_feedback
        .lock()
        .map_err(|_| "MIDI feedback state lock was poisoned".to_string())?;
    let output = guard
        .as_mut()
        .ok_or_else(|| "MIDI feedback output is not connected".to_string())?;
    output
        .send_feedback_messages(&messages)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn learn_midi_control(input_index: usize) -> Result<Option<LearnedMidiControl>, String> {
    io::midi::learn_midi_control(input_index, Duration::from_secs(10))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_midi_mappings(mappings: Vec<MidiControlMapping>) -> Result<Option<String>, String> {
    let mappings = validate_midi_control_mappings(mappings)?;
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard MIDI Mapping", &["midimap"])
        .set_file_name("rayard.midimap")
        .save_file()
    else {
        return Ok(None);
    };
    let file = MidiMappingFile {
        version: 1,
        mappings,
    };
    let json = serde_json::to_string_pretty(&file).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn load_midi_mappings() -> Result<Option<Vec<MidiControlMapping>>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard MIDI Mapping", &["midimap"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let file: MidiMappingFile = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    if file.version != 1 {
        return Err(format!("Unsupported MIDI mapping version {}", file.version));
    }
    Ok(Some(validate_midi_control_mappings(file.mappings)?))
}

#[tauri::command]
fn save_osc_mappings(mappings: Vec<OscControlMapping>) -> Result<Option<String>, String> {
    let mappings = validate_osc_control_mappings(mappings)?;
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard OSC Mapping", &["oscmap"])
        .set_file_name("rayard.oscmap")
        .save_file()
    else {
        return Ok(None);
    };
    let file = OscMappingFile {
        version: 1,
        mappings,
    };
    let json = serde_json::to_string_pretty(&file).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn load_osc_mappings() -> Result<Option<Vec<OscControlMapping>>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard OSC Mapping", &["oscmap"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let file: OscMappingFile = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    if file.version != 1 {
        return Err(format!("Unsupported OSC mapping version {}", file.version));
    }
    Ok(Some(validate_osc_control_mappings(file.mappings)?))
}

fn validate_midi_control_mappings(
    mappings: Vec<MidiControlMapping>,
) -> Result<Vec<MidiControlMapping>, String> {
    mappings
        .into_iter()
        .enumerate()
        .map(|(index, mapping)| validate_midi_control_mapping(index + 1, mapping))
        .collect()
}

fn validate_midi_control_mapping(
    index: usize,
    mut mapping: MidiControlMapping,
) -> Result<MidiControlMapping, String> {
    let owner = format!("MIDI mapping {index}");
    if let Some(channel) = mapping.channel {
        if channel > 15 {
            return Err(format!("{owner} channel must be 0-15 or omitted"));
        }
    }
    if mapping.number > 127 {
        return Err(format!("{owner} control number must be 0-127"));
    }
    validate_mapping_range(&owner, mapping.low, mapping.high)?;
    validate_mapping_required_fields_for_midi(&owner, &mut mapping)?;
    Ok(mapping)
}

fn validate_mapping_required_fields_for_midi(
    owner: &str,
    mapping: &mut MidiControlMapping,
) -> Result<(), String> {
    match &mapping.action {
        MidiControlAction::FixtureAttribute => {
            require_mapping_id(mapping.fixture_id, owner, "fixture")?;
            normalize_mapping_attribute(&mut mapping.attribute, owner)?;
        }
        MidiControlAction::FixtureHighlight
        | MidiControlAction::FixtureSolo
        | MidiControlAction::FixturePark => {
            require_mapping_id(mapping.fixture_id, owner, "fixture")?;
        }
        MidiControlAction::GroupHighlight
        | MidiControlAction::GroupSolo
        | MidiControlAction::GroupPark
        | MidiControlAction::GroupSubmaster => {
            normalize_mapping_group_id(&mut mapping.group_id, owner)?;
        }
        MidiControlAction::TriggerCue
        | MidiControlAction::EffectEnabled
        | MidiControlAction::NodeGraphEnabled => {
            require_mapping_id(mapping.cue_id, owner, "cue/effect/node graph")?;
        }
        MidiControlAction::VideoParam => {
            require_mapping_id(mapping.layer_id, owner, "video layer")?;
            require_mapping_option(mapping.video_param.as_ref(), owner, "video parameter")?;
        }
        MidiControlAction::VideoCuePointAdd
        | MidiControlAction::VideoCuePointJump
        | MidiControlAction::VideoCuePointPrevious
        | MidiControlAction::VideoCuePointNext
        | MidiControlAction::VideoLayerEnabled
        | MidiControlAction::VideoLayerSolo
        | MidiControlAction::VideoPlay
        | MidiControlAction::VideoLoop
        | MidiControlAction::VideoLayerFade => {
            require_mapping_id(mapping.layer_id, owner, "video layer")?;
        }
        MidiControlAction::VideoCuePointRemove => {
            require_mapping_id(mapping.layer_id, owner, "video layer")?;
            require_mapping_id(mapping.duration_ms, owner, "cue point time")?;
        }
        MidiControlAction::VideoOutputEnabled
        | MidiControlAction::VideoOutputOpacity
        | MidiControlAction::VideoOutputFade
        | MidiControlAction::VideoOutputBlackout => {
            require_mapping_id(mapping.output_id, owner, "video output")?;
        }
        MidiControlAction::VideoOutputMappingField => {
            require_mapping_id(mapping.output_id, owner, "video output")?;
            normalize_mapping_field_attribute(&mut mapping.attribute, owner)?;
        }
        MidiControlAction::VideoOutputMappingPreset => {
            require_mapping_id(mapping.output_id, owner, "video output")?;
            normalize_mapping_preset_attribute(&mut mapping.attribute, owner)?;
        }
        MidiControlAction::ClearFixtureFlags => {
            normalize_mapping_fixture_flag_kind(&mut mapping.attribute, owner)?;
        }
        MidiControlAction::TriggerNextCue
        | MidiControlAction::TriggerPreviousCue
        | MidiControlAction::TimelinePlay
        | MidiControlAction::TimelineSeek
        | MidiControlAction::TimelineBeatPrevious
        | MidiControlAction::TimelineBeatNext
        | MidiControlAction::SetBpm
        | MidiControlAction::TapBpm
        | MidiControlAction::LightingMaster
        | MidiControlAction::CueFadePause
        | MidiControlAction::Blackout
        | MidiControlAction::AllBlackout
        | MidiControlAction::VideoBlackout => {}
    }
    Ok(())
}

fn validate_osc_control_mappings(
    mappings: Vec<OscControlMapping>,
) -> Result<Vec<OscControlMapping>, String> {
    mappings
        .into_iter()
        .enumerate()
        .map(|(index, mapping)| validate_osc_control_mapping(index + 1, mapping))
        .collect()
}

fn validate_osc_control_mapping(
    index: usize,
    mut mapping: OscControlMapping,
) -> Result<OscControlMapping, String> {
    let owner = format!("OSC mapping {index}");
    mapping.address = normalize_control_mapping_osc_address(&mapping.address)
        .map_err(|error| format!("{owner} {error}"))?;
    validate_mapping_range(&owner, mapping.low, mapping.high)?;
    validate_mapping_required_fields_for_osc(&owner, &mut mapping)?;
    Ok(mapping)
}

fn validate_mapping_required_fields_for_osc(
    owner: &str,
    mapping: &mut OscControlMapping,
) -> Result<(), String> {
    match &mapping.action {
        OscControlAction::FixtureAttribute => {
            require_mapping_id(mapping.fixture_id, owner, "fixture")?;
            normalize_mapping_attribute(&mut mapping.attribute, owner)?;
        }
        OscControlAction::FixtureHighlight
        | OscControlAction::FixtureSolo
        | OscControlAction::FixturePark => {
            require_mapping_id(mapping.fixture_id, owner, "fixture")?;
        }
        OscControlAction::GroupHighlight
        | OscControlAction::GroupSolo
        | OscControlAction::GroupPark
        | OscControlAction::GroupSubmaster => {
            normalize_mapping_group_id(&mut mapping.group_id, owner)?;
        }
        OscControlAction::TriggerCue
        | OscControlAction::EffectEnabled
        | OscControlAction::NodeGraphEnabled => {
            require_mapping_id(mapping.cue_id, owner, "cue/effect/node graph")?;
        }
        OscControlAction::VideoParam => {
            require_mapping_id(mapping.layer_id, owner, "video layer")?;
            require_mapping_option(mapping.video_param.as_ref(), owner, "video parameter")?;
        }
        OscControlAction::VideoCuePointAdd
        | OscControlAction::VideoCuePointJump
        | OscControlAction::VideoCuePointPrevious
        | OscControlAction::VideoCuePointNext
        | OscControlAction::VideoLayerEnabled
        | OscControlAction::VideoLayerSolo
        | OscControlAction::VideoPlay
        | OscControlAction::VideoLoop
        | OscControlAction::VideoLayerFade => {
            require_mapping_id(mapping.layer_id, owner, "video layer")?;
        }
        OscControlAction::VideoCuePointRemove => {
            require_mapping_id(mapping.layer_id, owner, "video layer")?;
            require_mapping_id(mapping.duration_ms, owner, "cue point time")?;
        }
        OscControlAction::VideoOutputEnabled
        | OscControlAction::VideoOutputOpacity
        | OscControlAction::VideoOutputFade
        | OscControlAction::VideoOutputBlackout => {
            require_mapping_id(mapping.output_id, owner, "video output")?;
        }
        OscControlAction::VideoOutputMappingField => {
            require_mapping_id(mapping.output_id, owner, "video output")?;
            normalize_mapping_field_attribute(&mut mapping.attribute, owner)?;
        }
        OscControlAction::VideoOutputMappingPreset => {
            require_mapping_id(mapping.output_id, owner, "video output")?;
            normalize_mapping_preset_attribute(&mut mapping.attribute, owner)?;
        }
        OscControlAction::ClearFixtureFlags => {
            normalize_mapping_fixture_flag_kind(&mut mapping.attribute, owner)?;
        }
        OscControlAction::TriggerNextCue
        | OscControlAction::TriggerPreviousCue
        | OscControlAction::TimelinePlay
        | OscControlAction::TimelineSeek
        | OscControlAction::TimelineBeatPrevious
        | OscControlAction::TimelineBeatNext
        | OscControlAction::SetBpm
        | OscControlAction::TapBpm
        | OscControlAction::LightingMaster
        | OscControlAction::CueFadePause
        | OscControlAction::Blackout
        | OscControlAction::AllBlackout
        | OscControlAction::VideoBlackout => {}
    }
    Ok(())
}

fn validate_mapping_range(owner: &str, low: f32, high: f32) -> Result<(), String> {
    if !low.is_finite() || !high.is_finite() {
        return Err(format!("{owner} range values must be finite"));
    }
    Ok(())
}

fn require_mapping_id<T>(value: Option<T>, owner: &str, field: &str) -> Result<(), String> {
    if value.is_some() {
        Ok(())
    } else {
        Err(format!("{owner} requires a {field} target"))
    }
}

fn require_mapping_option<T>(value: Option<&T>, owner: &str, field: &str) -> Result<(), String> {
    if value.is_some() {
        Ok(())
    } else {
        Err(format!("{owner} requires a {field} target"))
    }
}

fn normalize_mapping_attribute(value: &mut Option<String>, owner: &str) -> Result<(), String> {
    let normalized = normalize_required_mapping_text(value.as_deref(), owner, "attribute")?;
    *value = Some(normalized);
    Ok(())
}

fn normalize_mapping_group_id(value: &mut Option<String>, owner: &str) -> Result<(), String> {
    let raw = normalize_required_mapping_text(value.as_deref(), owner, "group ID")?;
    *value = Some(normalize_group_id(&raw).map_err(|error| format!("{owner} {error}"))?);
    Ok(())
}

fn normalize_mapping_field_attribute(
    value: &mut Option<String>,
    owner: &str,
) -> Result<(), String> {
    let raw = normalize_required_mapping_text(value.as_deref(), owner, "mapping field")?;
    *value = Some(
        normalize_video_output_mapping_field(raw).map_err(|error| format!("{owner} {error}"))?,
    );
    Ok(())
}

fn normalize_mapping_preset_attribute(
    value: &mut Option<String>,
    owner: &str,
) -> Result<(), String> {
    let raw = normalize_required_mapping_text(value.as_deref(), owner, "mapping preset")?;
    *value = Some(
        normalize_video_output_mapping_preset_label(raw)
            .map_err(|error| format!("{owner} {error}"))?,
    );
    Ok(())
}

fn normalize_mapping_fixture_flag_kind(
    value: &mut Option<String>,
    owner: &str,
) -> Result<(), String> {
    let raw = value.as_deref().unwrap_or("all").trim();
    let kind = if raw.is_empty() { "all" } else { raw };
    let normalized =
        match parse_fixture_flag_clear_kind(kind).map_err(|error| format!("{owner} {error}"))? {
            FixtureFlagClearKind::Highlight => "highlight",
            FixtureFlagClearKind::Solo => "solo",
            FixtureFlagClearKind::Park => "park",
            FixtureFlagClearKind::All => "all",
        };
    *value = Some(normalized.to_string());
    Ok(())
}

fn normalize_required_mapping_text(
    value: Option<&str>,
    owner: &str,
    field: &str,
) -> Result<String, String> {
    let Some(raw) = value else {
        return Err(format!("{owner} requires a {field} target"));
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{owner} requires a {field} target"));
    }
    Ok(trimmed.to_string())
}

fn normalize_control_mapping_osc_address(address: &str) -> Result<String, String> {
    let trimmed = address.trim().trim_matches('/');
    if trimmed.is_empty() {
        return Err("address is required".to_string());
    }
    Ok(format!("/{trimmed}"))
}

#[tauri::command]
fn learn_osc_control(config: OscInputConfig) -> Result<Option<LearnedOscControl>, String> {
    io::osc::learn_osc_control(config, Duration::from_secs(10)).map_err(|error| error.to_string())
}

#[tauri::command]
fn start_osc_input(
    state: State<'_, AppState>,
    config: OscInputConfig,
    mappings: Option<Vec<OscControlMapping>>,
) -> Result<(), String> {
    let mappings = validate_osc_control_mappings(mappings.unwrap_or_default())?;
    let engine = state.engine.clone();
    let input = OscInput::start_with_mappings(config, mappings, move |event| {
        let command = match event {
            OscInputEvent::SetAttribute {
                fixture_id,
                attribute,
                value,
            } => EngineCommand::SetAttribute {
                fixture_id,
                attribute,
                value,
            },
            OscInputEvent::SetFixtureHighlight {
                fixture_id,
                enabled,
            } => EngineCommand::SetFixtureHighlight {
                fixture_id,
                enabled,
            },
            OscInputEvent::SetFixtureSolo {
                fixture_id,
                enabled,
            } => EngineCommand::SetFixtureSolo {
                fixture_id,
                enabled,
            },
            OscInputEvent::SetFixturePark {
                fixture_id,
                enabled,
            } => EngineCommand::SetFixturePark {
                fixture_id,
                enabled,
            },
            OscInputEvent::SetGroupHighlight { group_id, enabled } => {
                EngineCommand::SetGroupHighlight { group_id, enabled }
            }
            OscInputEvent::SetGroupSolo { group_id, enabled } => {
                EngineCommand::SetGroupSolo { group_id, enabled }
            }
            OscInputEvent::SetGroupPark { group_id, enabled } => {
                EngineCommand::SetGroupPark { group_id, enabled }
            }
            OscInputEvent::Blackout(enabled) => EngineCommand::Blackout(enabled),
            OscInputEvent::AllBlackout(enabled) => EngineCommand::SetAllBlackout(enabled),
            OscInputEvent::ClearFixtureFlags { kind } => {
                match parse_fixture_flag_clear_kind(&kind) {
                    Ok(kind) => EngineCommand::ClearFixtureFlags(kind),
                    Err(error) => {
                        eprintln!("Ignoring OSC clear fixture flags command: {error}");
                        return;
                    }
                }
            }
            OscInputEvent::TriggerCue(cue_id) => EngineCommand::TriggerCue(cue_id),
            OscInputEvent::TriggerNextCue => EngineCommand::TriggerNextCue,
            OscInputEvent::TriggerPreviousCue => EngineCommand::TriggerPreviousCue,
            OscInputEvent::SetEffectEnabled { effect_id, enabled } => {
                EngineCommand::SetEffectEnabled { effect_id, enabled }
            }
            OscInputEvent::SetNodeGraphEnabled { graph_id, enabled } => {
                EngineCommand::SetNodeGraphEnabled { graph_id, enabled }
            }
            OscInputEvent::SetCueFadePaused(paused) => EngineCommand::SetCueFadePaused(paused),
            OscInputEvent::SetTimelinePlaying(playing) => {
                EngineCommand::SetTimelinePlaying(playing)
            }
            OscInputEvent::SeekTimeline { position_ms } => EngineCommand::SeekTimeline(position_ms),
            OscInputEvent::SeekTimelineBeat { direction } => {
                EngineCommand::SeekTimelineBeat { direction }
            }
            OscInputEvent::SyncTimelineTimecode {
                position_ms,
                source,
            } => EngineCommand::SyncTimelineTimecode {
                position_ms,
                source,
            },
            OscInputEvent::SetVideoParam {
                layer_id,
                param,
                value,
            } => EngineCommand::SetVideoLayerParam {
                layer_id,
                param,
                value,
            },
            OscInputEvent::SetVideoPlaying { layer_id, playing } => {
                EngineCommand::SetVideoLayerPlaying { layer_id, playing }
            }
            OscInputEvent::SetVideoLoop {
                layer_id,
                enabled,
                loop_start_ms,
                loop_end_ms,
            } => EngineCommand::SetVideoLayerLoop {
                layer_id,
                enabled,
                loop_start_ms,
                loop_end_ms,
            },
            OscInputEvent::FadeVideoLayerOpacity {
                layer_id,
                opacity,
                duration_ms,
            } => EngineCommand::FadeVideoLayerOpacity {
                layer_id,
                opacity,
                duration_ms,
            },
            OscInputEvent::SetVideoLayerEnabled { layer_id, enabled } => {
                EngineCommand::SetVideoLayerEnabled { layer_id, enabled }
            }
            OscInputEvent::SetVideoLayerSolo { layer_id, solo } => {
                EngineCommand::SetVideoLayerSolo { layer_id, solo }
            }
            OscInputEvent::SeekVideoLayer {
                layer_id,
                position_ms,
            } => EngineCommand::SetVideoLayerParam {
                layer_id,
                param: VideoParam::PositionMs,
                value: position_ms as f32,
            },
            OscInputEvent::AddVideoCuePoint {
                layer_id,
                position_ms,
            } => EngineCommand::AddVideoCuePoint {
                layer_id,
                position_ms,
            },
            OscInputEvent::RemoveVideoCuePoint {
                layer_id,
                position_ms,
            } => EngineCommand::RemoveVideoCuePoint {
                layer_id,
                position_ms,
            },
            OscInputEvent::JumpVideoCuePoint {
                layer_id,
                cue_point_index,
            } => EngineCommand::JumpVideoCuePoint {
                layer_id,
                cue_point_index,
            },
            OscInputEvent::JumpVideoCuePointRelative {
                layer_id,
                direction,
            } => EngineCommand::JumpVideoCuePointRelative {
                layer_id,
                direction,
            },
            OscInputEvent::SetVideoOutputEnabled { output_id, enabled } => {
                EngineCommand::SetVideoOutputEnabled { output_id, enabled }
            }
            OscInputEvent::SetVideoOutputOpacity { output_id, opacity } => {
                EngineCommand::SetVideoOutputOpacity { output_id, opacity }
            }
            OscInputEvent::FadeVideoOutputOpacity {
                output_id,
                opacity,
                duration_ms,
            } => EngineCommand::FadeVideoOutputOpacity {
                output_id,
                opacity,
                duration_ms,
            },
            OscInputEvent::SetVideoOutputMappingField {
                output_id,
                field,
                value,
            } => EngineCommand::SetVideoOutputMappingField {
                output_id,
                field,
                value,
            },
            OscInputEvent::ApplyVideoOutputMappingPreset { output_id, label } => {
                EngineCommand::ApplyVideoOutputMappingPreset { output_id, label }
            }
            OscInputEvent::SetVideoOutputBlackout {
                output_id,
                blackout,
            } => EngineCommand::SetVideoOutputBlackout {
                output_id,
                blackout,
            },
            OscInputEvent::VideoMasterOpacity(opacity) => {
                EngineCommand::SetVideoMasterOpacity(opacity)
            }
            OscInputEvent::VideoBlackout(enabled) => EngineCommand::SetVideoBlackout(enabled),
            OscInputEvent::LightingMaster(master) => EngineCommand::SetLightingMaster(master),
            OscInputEvent::SetGroupSubmaster { group_id, level } => {
                EngineCommand::SetGroupSubmaster { group_id, level }
            }
            OscInputEvent::SetBpm(bpm) => EngineCommand::SetBpm(bpm),
            OscInputEvent::TapBpm => EngineCommand::TapBpm,
            OscInputEvent::SyncExternalClock {
                bpm,
                beat_phase,
                source,
            } => EngineCommand::SyncExternalClock {
                bpm,
                beat_phase,
                source,
            },
        };
        let _ = engine.send(command);
    })
    .map_err(|error| error.to_string())?;
    let mut guard = state
        .osc_input
        .lock()
        .map_err(|_| "OSC state lock was poisoned".to_string())?;
    *guard = Some(input);
    Ok(())
}

#[tauri::command]
fn stop_osc_input(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .osc_input
        .lock()
        .map_err(|_| "OSC state lock was poisoned".to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
fn remote_access_urls(config: RemoteControlConfig) -> Vec<String> {
    build_remote_access_urls(&config, discover_lan_ip())
}

fn discover_lan_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

fn build_remote_access_urls(config: &RemoteControlConfig, lan_ip: Option<IpAddr>) -> Vec<String> {
    let port = config.port;
    let bind_ip = config.bind_ip.trim();
    let mut hosts = Vec::new();
    match bind_ip.parse::<IpAddr>() {
        Ok(ip) if ip.is_unspecified() => {
            hosts.push("localhost".to_string());
            if let Some(lan_ip) = lan_ip.filter(|ip| !ip.is_loopback() && !ip.is_unspecified()) {
                hosts.push(format_url_host(lan_ip));
            }
        }
        Ok(ip) if ip.is_loopback() => hosts.push("localhost".to_string()),
        Ok(ip) => hosts.push(format_url_host(ip)),
        Err(_) if bind_ip.is_empty() => {
            hosts.push("localhost".to_string());
            if let Some(lan_ip) = lan_ip.filter(|ip| !ip.is_loopback() && !ip.is_unspecified()) {
                hosts.push(format_url_host(lan_ip));
            }
        }
        Err(_) => hosts.push(bind_ip.to_string()),
    }

    hosts.sort();
    hosts.dedup();
    hosts
        .into_iter()
        .map(|host| format!("http://{host}:{port}/remote"))
        .collect()
}

fn format_url_host(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => format!("[{ip}]"),
    }
}

#[tauri::command]
fn start_remote_control(
    state: State<'_, AppState>,
    config: RemoteControlConfig,
) -> Result<(), String> {
    let command_engine = state.engine.clone();
    let snapshot_engine = state.engine.clone();
    let render_plans_engine = state.engine.clone();
    let io_plans_engine = state.engine.clone();
    let transport_status = Arc::clone(&state.external_video_transport);
    let sync_engine = state.engine.clone();
    let sync_transport = Arc::clone(&state.external_video_transport);
    let sync_events = Arc::clone(&state.external_video_transport_events);
    let server = RemoteWsServer::start_with_snapshot_and_video_status_providers(
        config,
        move |event| {
            let command = match event {
                RemoteInputEvent::SetAttribute {
                    fixture_id,
                    attribute,
                    value,
                } => EngineCommand::SetAttribute {
                    fixture_id,
                    attribute,
                    value,
                },
                RemoteInputEvent::SetGroupAttribute {
                    group_id,
                    attribute,
                    value,
                } => EngineCommand::SetGroupAttribute {
                    group_id,
                    attribute,
                    value,
                },
                RemoteInputEvent::SetFixtureHighlight {
                    fixture_id,
                    enabled,
                } => EngineCommand::SetFixtureHighlight {
                    fixture_id,
                    enabled,
                },
                RemoteInputEvent::SetFixtureSolo {
                    fixture_id,
                    enabled,
                } => EngineCommand::SetFixtureSolo {
                    fixture_id,
                    enabled,
                },
                RemoteInputEvent::SetFixturePark {
                    fixture_id,
                    enabled,
                } => EngineCommand::SetFixturePark {
                    fixture_id,
                    enabled,
                },
                RemoteInputEvent::SetGroupHighlight { group_id, enabled } => {
                    EngineCommand::SetGroupHighlight { group_id, enabled }
                }
                RemoteInputEvent::SetGroupSolo { group_id, enabled } => {
                    EngineCommand::SetGroupSolo { group_id, enabled }
                }
                RemoteInputEvent::SetGroupPark { group_id, enabled } => {
                    EngineCommand::SetGroupPark { group_id, enabled }
                }
                RemoteInputEvent::ClearFixtureFlags { kind } => {
                    match parse_fixture_flag_clear_kind(&kind) {
                        Ok(kind) => EngineCommand::ClearFixtureFlags(kind),
                        Err(error) => {
                            eprintln!("Ignoring remote clear fixture flags command: {error}");
                            return;
                        }
                    }
                }
                RemoteInputEvent::Blackout(enabled) => EngineCommand::Blackout(enabled),
                RemoteInputEvent::AllBlackout(enabled) => EngineCommand::SetAllBlackout(enabled),
                RemoteInputEvent::SetGroupSubmaster { group_id, level } => {
                    EngineCommand::SetGroupSubmaster { group_id, level }
                }
                RemoteInputEvent::SetBpm(bpm) => EngineCommand::SetBpm(bpm),
                RemoteInputEvent::TapBpm => EngineCommand::TapBpm,
                RemoteInputEvent::SyncExternalClock {
                    bpm,
                    beat_phase,
                    source,
                } => EngineCommand::SyncExternalClock {
                    bpm,
                    beat_phase,
                    source,
                },
                RemoteInputEvent::ResetTelemetry => EngineCommand::ResetTelemetry,
                RemoteInputEvent::TriggerCue(cue_id) => EngineCommand::TriggerCue(cue_id),
                RemoteInputEvent::TriggerNextCue => EngineCommand::TriggerNextCue,
                RemoteInputEvent::TriggerPreviousCue => EngineCommand::TriggerPreviousCue,
                RemoteInputEvent::SetCueFadePaused(paused) => {
                    EngineCommand::SetCueFadePaused(paused)
                }
                RemoteInputEvent::SetTimelinePlaying(playing) => {
                    EngineCommand::SetTimelinePlaying(playing)
                }
                RemoteInputEvent::SeekTimeline { position_ms } => {
                    EngineCommand::SeekTimeline(position_ms)
                }
                RemoteInputEvent::SeekTimelineBeat { direction } => {
                    EngineCommand::SeekTimelineBeat { direction }
                }
                RemoteInputEvent::SyncTimelineTimecode {
                    position_ms,
                    source,
                } => EngineCommand::SyncTimelineTimecode {
                    position_ms,
                    source,
                },
                RemoteInputEvent::SetEffectEnabled { effect_id, enabled } => {
                    EngineCommand::SetEffectEnabled { effect_id, enabled }
                }
                RemoteInputEvent::SetNodeGraphEnabled { graph_id, enabled } => {
                    EngineCommand::SetNodeGraphEnabled { graph_id, enabled }
                }
                RemoteInputEvent::MoveEffect { effect_id, delta } => {
                    EngineCommand::MoveEffect { effect_id, delta }
                }
                RemoteInputEvent::RemoveEffect { effect_id } => {
                    EngineCommand::RemoveEffect(effect_id)
                }
                RemoteInputEvent::SetVideoParam {
                    layer_id,
                    param,
                    value,
                } => EngineCommand::SetVideoLayerParam {
                    layer_id,
                    param,
                    value,
                },
                RemoteInputEvent::SetVideoLayerEnabled { layer_id, enabled } => {
                    EngineCommand::SetVideoLayerEnabled { layer_id, enabled }
                }
                RemoteInputEvent::SetVideoLayerSolo { layer_id, solo } => {
                    EngineCommand::SetVideoLayerSolo { layer_id, solo }
                }
                RemoteInputEvent::SetVideoPlaying { layer_id, playing } => {
                    EngineCommand::SetVideoLayerPlaying { layer_id, playing }
                }
                RemoteInputEvent::SeekVideoLayer {
                    layer_id,
                    position_ms,
                } => EngineCommand::SetVideoLayerParam {
                    layer_id,
                    param: VideoParam::PositionMs,
                    value: position_ms as f32,
                },
                RemoteInputEvent::SetVideoLoop {
                    layer_id,
                    enabled,
                    loop_start_ms,
                    loop_end_ms,
                } => EngineCommand::SetVideoLayerLoop {
                    layer_id,
                    enabled,
                    loop_start_ms,
                    loop_end_ms,
                },
                RemoteInputEvent::FadeVideoLayerOpacity {
                    layer_id,
                    opacity,
                    duration_ms,
                } => EngineCommand::FadeVideoLayerOpacity {
                    layer_id,
                    opacity,
                    duration_ms,
                },
                RemoteInputEvent::AddVideoCuePoint {
                    layer_id,
                    position_ms,
                } => EngineCommand::AddVideoCuePoint {
                    layer_id,
                    position_ms,
                },
                RemoteInputEvent::RemoveVideoCuePoint {
                    layer_id,
                    position_ms,
                } => EngineCommand::RemoveVideoCuePoint {
                    layer_id,
                    position_ms,
                },
                RemoteInputEvent::JumpVideoCuePoint {
                    layer_id,
                    cue_point_index,
                } => EngineCommand::JumpVideoCuePoint {
                    layer_id,
                    cue_point_index,
                },
                RemoteInputEvent::JumpVideoCuePointRelative {
                    layer_id,
                    direction,
                } => EngineCommand::JumpVideoCuePointRelative {
                    layer_id,
                    direction,
                },
                RemoteInputEvent::SetVideoOutputEnabled { output_id, enabled } => {
                    EngineCommand::SetVideoOutputEnabled { output_id, enabled }
                }
                RemoteInputEvent::SetVideoOutputOpacity { output_id, opacity } => {
                    EngineCommand::SetVideoOutputOpacity { output_id, opacity }
                }
                RemoteInputEvent::FadeVideoOutputOpacity {
                    output_id,
                    opacity,
                    duration_ms,
                } => EngineCommand::FadeVideoOutputOpacity {
                    output_id,
                    opacity,
                    duration_ms,
                },
                RemoteInputEvent::SetVideoOutputMapping { output_id, mapping } => {
                    EngineCommand::SetVideoOutputMapping { output_id, mapping }
                }
                RemoteInputEvent::SetVideoOutputMappingField {
                    output_id,
                    field,
                    value,
                } => EngineCommand::SetVideoOutputMappingField {
                    output_id,
                    field,
                    value,
                },
                RemoteInputEvent::ApplyVideoOutputMappingPreset { output_id, label } => {
                    EngineCommand::ApplyVideoOutputMappingPreset { output_id, label }
                }
                RemoteInputEvent::SetVideoOutputBlackout {
                    output_id,
                    blackout,
                } => EngineCommand::SetVideoOutputBlackout {
                    output_id,
                    blackout,
                },
                RemoteInputEvent::VideoMasterOpacity(opacity) => {
                    EngineCommand::SetVideoMasterOpacity(opacity)
                }
                RemoteInputEvent::VideoBlackout(enabled) => {
                    EngineCommand::SetVideoBlackout(enabled)
                }
                RemoteInputEvent::LightingMaster(master) => {
                    EngineCommand::SetLightingMaster(master)
                }
            };
            let _ = command_engine.send(command);
        },
        move || snapshot_engine.snapshot(),
        video::video_runtime_status,
        move || {
            let snapshot = render_plans_engine.snapshot();
            match video::build_video_output_render_plans(&snapshot.video) {
                Ok(plans) => serde_json::to_value(plans).unwrap_or_else(|_| {
                    json!({
                        "plans": [],
                        "error": "Video output render plan serialization failed",
                    })
                }),
                Err(error) => json!({
                    "plans": [],
                    "error": format!("{error:?}"),
                }),
            }
        },
        move || {
            let snapshot = io_plans_engine.snapshot();
            serde_json::to_value(video::build_external_video_io_route_plans(
                &snapshot.video,
                &video::video_runtime_status(),
            ))
            .unwrap_or_else(|_| {
                json!({
                    "inputs": [],
                    "outputs": [],
                    "error": "External video I/O plan serialization failed",
                })
            })
        },
        move || match transport_status.lock() {
            Ok(transport) => serde_json::to_value(transport.status()).unwrap_or_else(|_| {
                json!({
                    "active_routes": [],
                    "active_count": 0,
                    "error": "External video transport status serialization failed",
                })
            }),
            Err(_) => json!({
                "active_routes": [],
                "active_count": 0,
                "error": "External video transport runtime lock was poisoned",
            }),
        },
        move || {
            let snapshot = sync_engine.snapshot();
            match sync_external_video_transports_from_snapshot(
                &snapshot,
                sync_transport.as_ref(),
                sync_events.as_ref(),
            ) {
                Ok(sync) => serde_json::to_value(sync).unwrap_or_else(|_| {
                    json!({
                        "report": null,
                        "events": [],
                        "error": "External video transport sync serialization failed",
                    })
                }),
                Err(error) => json!({
                    "report": null,
                    "events": [],
                    "error": error,
                }),
            }
        },
    )
    .map_err(|error| error.to_string())?;
    let mut guard = state
        .remote_control
        .lock()
        .map_err(|_| "Remote control state lock was poisoned".to_string())?;
    *guard = Some(server);
    Ok(())
}

#[tauri::command]
fn stop_remote_control(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .remote_control
        .lock()
        .map_err(|_| "Remote control state lock was poisoned".to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
fn create_cue_from_current(
    state: State<'_, AppState>,
    label: String,
    fade_ms: u64,
    capture_scope: Option<CueCaptureScope>,
) -> Result<CueId, String> {
    let label = label.trim().to_string();
    if label.is_empty() {
        return Err("Cue label is required".to_string());
    }
    let snapshot = state.engine.snapshot();
    let scope = capture_scope.unwrap_or_default();
    let (targets, video_targets, video_output_targets, node_graph_targets) =
        cue_targets_from_snapshot_with_scope(&snapshot, &scope)?;
    ensure_cue_targets_present(
        &targets,
        &video_targets,
        &video_output_targets,
        &node_graph_targets,
        "creating",
    )?;
    let cue_id = state.engine.allocate_cue_id();
    state
        .engine
        .send(EngineCommand::CreateCue {
            cue_id,
            label,
            fade_ms,
            targets,
            video_targets,
            video_output_targets,
            node_graph_targets,
        })
        .map_err(|error| error.to_string())?;
    Ok(cue_id)
}

#[tauri::command]
fn update_cue_from_current(
    state: State<'_, AppState>,
    cue_id: CueId,
    label: String,
    fade_ms: u64,
    capture_scope: Option<CueCaptureScope>,
) -> Result<(), String> {
    let label = label.trim().to_string();
    if label.is_empty() {
        return Err("Cue label is required".to_string());
    }
    let snapshot = state.engine.snapshot();
    if !snapshot.cues.iter().any(|cue| cue.id == cue_id) {
        return Err(format!("Cue {cue_id} was not found"));
    }
    let scope = capture_scope.unwrap_or_default();
    let captured = cue_targets_from_snapshot_with_scope(&snapshot, &scope)?;
    ensure_cue_targets_present(
        &captured.0,
        &captured.1,
        &captured.2,
        &captured.3,
        "updating",
    )?;
    let (targets, video_targets, video_output_targets, node_graph_targets) =
        merge_cue_update_targets(&snapshot, cue_id, &scope, captured)?;
    state
        .engine
        .send(EngineCommand::UpdateCue {
            cue_id,
            label,
            fade_ms,
            targets,
            video_targets,
            video_output_targets,
            node_graph_targets,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_cue_metadata(
    state: State<'_, AppState>,
    cue_id: CueId,
    label: String,
    fade_ms: u64,
) -> Result<(), String> {
    let label = label.trim().to_string();
    if label.is_empty() {
        return Err("Cue label is required".to_string());
    }
    if !state
        .engine
        .snapshot()
        .cues
        .iter()
        .any(|cue| cue.id == cue_id)
    {
        return Err(format!("Cue {cue_id} was not found"));
    }
    state
        .engine
        .send(EngineCommand::SetCueMetadata {
            cue_id,
            label,
            fade_ms,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn move_cue(state: State<'_, AppState>, cue_id: CueId, delta: i32) -> Result<(), String> {
    if delta == 0 {
        return Ok(());
    }
    if !state
        .engine
        .snapshot()
        .cues
        .iter()
        .any(|cue| cue.id == cue_id)
    {
        return Err(format!("Cue {cue_id} was not found"));
    }
    state
        .engine
        .send(EngineCommand::MoveCue { cue_id, delta })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn duplicate_cue(
    state: State<'_, AppState>,
    source_cue_id: CueId,
    label: String,
) -> Result<CueId, String> {
    let label = label.trim().to_string();
    if label.is_empty() {
        return Err("Cue label is required".to_string());
    }
    if !state
        .engine
        .snapshot()
        .cues
        .iter()
        .any(|cue| cue.id == source_cue_id)
    {
        return Err(format!("Cue {source_cue_id} was not found"));
    }
    let cue_id = state.engine.allocate_cue_id();
    state
        .engine
        .send(EngineCommand::DuplicateCue {
            source_cue_id,
            cue_id,
            label,
        })
        .map_err(|error| error.to_string())?;
    Ok(cue_id)
}

#[tauri::command]
fn trigger_cue(state: State<'_, AppState>, cue_id: CueId) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::TriggerCue(cue_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn trigger_next_cue(state: State<'_, AppState>) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::TriggerNextCue)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn trigger_previous_cue(state: State<'_, AppState>) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::TriggerPreviousCue)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_cue_fade_paused(state: State<'_, AppState>, paused: bool) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetCueFadePaused(paused))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn remove_cue(state: State<'_, AppState>, cue_id: CueId) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::RemoveCue(cue_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_timeline_cue_event(
    state: State<'_, AppState>,
    cue_id: CueId,
    time_ms: u64,
    track: TimelineTrackKind,
) -> Result<TimelineEventId, String> {
    let snapshot = state.engine.snapshot();
    if !snapshot.cues.iter().any(|cue| cue.id == cue_id) {
        return Err(format!("Cue {cue_id} was not found"));
    }
    let event_id = state.engine.allocate_timeline_event_id();
    state
        .engine
        .send(EngineCommand::AddTimelineCueEvent {
            event_id,
            cue_id,
            time_ms,
            track,
        })
        .map_err(|error| error.to_string())?;
    Ok(event_id)
}

#[tauri::command]
fn set_timeline_cue_event(
    state: State<'_, AppState>,
    event_id: TimelineEventId,
    cue_id: CueId,
    time_ms: u64,
    track: TimelineTrackKind,
) -> Result<(), String> {
    let snapshot = state.engine.snapshot();
    if !snapshot.cues.iter().any(|cue| cue.id == cue_id) {
        return Err(format!("Cue {cue_id} was not found"));
    }
    if !snapshot
        .timeline
        .events
        .iter()
        .any(|event| event.id == event_id)
    {
        return Err(format!("Timeline event {event_id} was not found"));
    }
    state
        .engine
        .send(EngineCommand::SetTimelineCueEvent {
            event_id,
            cue_id,
            time_ms,
            track,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn remove_timeline_event(
    state: State<'_, AppState>,
    event_id: TimelineEventId,
) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::RemoveTimelineEvent(event_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_timeline_automation(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
    attribute: String,
    keyframes: Vec<AutomationKeyframeSummary>,
) -> Result<AutomationId, String> {
    validate_timeline_automation_request(
        &state.engine.snapshot(),
        fixture_id,
        &attribute,
        &keyframes,
    )?;
    let automation_id = state.engine.allocate_automation_id();
    state
        .engine
        .send(EngineCommand::AddTimelineAutomation {
            automation_id,
            fixture_id,
            attribute,
            keyframes,
        })
        .map_err(|error| error.to_string())?;
    Ok(automation_id)
}

#[tauri::command]
fn add_timeline_group_automation(
    state: State<'_, AppState>,
    group_id: String,
    attribute: String,
    keyframes: Vec<AutomationKeyframeSummary>,
) -> Result<TimelineGroupAutomationAddResult, String> {
    let group_id = normalize_control_group_id(group_id)?;
    let snapshot = state.engine.snapshot();
    let (fixture_ids, skipped_count) =
        compatible_timeline_automation_targets(&snapshot, &group_id, &attribute, &keyframes)?;
    let mut automation_ids = Vec::with_capacity(fixture_ids.len());
    for fixture_id in fixture_ids {
        let automation_id = state.engine.allocate_automation_id();
        state
            .engine
            .send(EngineCommand::AddTimelineAutomation {
                automation_id,
                fixture_id,
                attribute: attribute.clone(),
                keyframes: keyframes.clone(),
            })
            .map_err(|error| error.to_string())?;
        automation_ids.push(automation_id);
    }
    Ok(TimelineGroupAutomationAddResult {
        applied_count: automation_ids.len(),
        skipped_count,
        automation_ids,
    })
}

#[tauri::command]
fn set_timeline_automation(
    state: State<'_, AppState>,
    automation_id: AutomationId,
    fixture_id: FixtureId,
    attribute: String,
    keyframes: Vec<AutomationKeyframeSummary>,
) -> Result<(), String> {
    let snapshot = state.engine.snapshot();
    if !snapshot
        .timeline
        .automations
        .iter()
        .any(|automation| automation.id == automation_id)
    {
        return Err(format!("Automation {automation_id} was not found"));
    }
    validate_timeline_automation_request(&snapshot, fixture_id, &attribute, &keyframes)?;
    state
        .engine
        .send(EngineCommand::SetTimelineAutomation {
            automation_id,
            fixture_id,
            attribute,
            keyframes,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_timeline_video_automation(
    state: State<'_, AppState>,
    layer_id: VideoLayerId,
    param: VideoParam,
    keyframes: Vec<VideoAutomationKeyframeSummary>,
) -> Result<AutomationId, String> {
    let snapshot = state.engine.snapshot();
    validate_timeline_video_automation_request(&snapshot, layer_id, &keyframes)?;
    let automation_id = state.engine.allocate_automation_id();
    state
        .engine
        .send(EngineCommand::AddTimelineVideoAutomation {
            automation_id,
            layer_id,
            param,
            keyframes,
        })
        .map_err(|error| error.to_string())?;
    Ok(automation_id)
}

#[tauri::command]
fn set_timeline_video_automation(
    state: State<'_, AppState>,
    automation_id: AutomationId,
    layer_id: VideoLayerId,
    param: VideoParam,
    keyframes: Vec<VideoAutomationKeyframeSummary>,
) -> Result<(), String> {
    let snapshot = state.engine.snapshot();
    if !snapshot
        .timeline
        .video_automations
        .iter()
        .any(|automation| automation.id == automation_id)
    {
        return Err(format!("Video automation {automation_id} was not found"));
    }
    validate_timeline_video_automation_request(&snapshot, layer_id, &keyframes)?;
    state
        .engine
        .send(EngineCommand::SetTimelineVideoAutomation {
            automation_id,
            layer_id,
            param,
            keyframes,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_timeline_automation_enabled(
    state: State<'_, AppState>,
    automation_id: AutomationId,
    enabled: bool,
) -> Result<(), String> {
    let snapshot = state.engine.snapshot();
    let found = snapshot
        .timeline
        .automations
        .iter()
        .any(|automation| automation.id == automation_id)
        || snapshot
            .timeline
            .video_automations
            .iter()
            .any(|automation| automation.id == automation_id);
    if !found {
        return Err(format!("Automation {automation_id} was not found"));
    }
    state
        .engine
        .send(EngineCommand::SetTimelineAutomationEnabled {
            automation_id,
            enabled,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn remove_timeline_automation(
    state: State<'_, AppState>,
    automation_id: AutomationId,
) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::RemoveTimelineAutomation(automation_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_timeline_playing(state: State<'_, AppState>, playing: bool) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetTimelinePlaying(playing))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn seek_timeline(state: State<'_, AppState>, position_ms: u64) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SeekTimeline(position_ms))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn seek_timeline_beat(state: State<'_, AppState>, direction: i32) -> Result<(), String> {
    if direction == 0 {
        return Err("Timeline beat direction must be non-zero".to_string());
    }
    state
        .engine
        .send(EngineCommand::SeekTimelineBeat { direction })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn sync_ltc_timecode(state: State<'_, AppState>, position_ms: u64) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SyncTimelineTimecode {
            position_ms,
            source: protocol::ClockSource::Ltc,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_video_file_layer(
    state: State<'_, AppState>,
    label: String,
    path: String,
) -> Result<VideoLayerId, String> {
    let label = normalize_video_layer_label(label)?;
    let path = validate_existing_file_path(path, "Video file")?;
    let probe = video::probe_video_file_metadata(&path).ok();
    let codec = probe
        .as_ref()
        .and_then(|probe| probe.codec.clone())
        .or_else(|| video::infer_video_codec_from_path(&path));
    let layer_id = state.engine.allocate_video_layer_id();
    state
        .engine
        .send(EngineCommand::AddVideoLayer {
            layer_id,
            label,
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(path),
                name: None,
                codec,
                metadata: probe.and_then(|probe| probe.metadata),
            },
        })
        .map_err(|error| error.to_string())?;
    Ok(layer_id)
}

#[tauri::command]
fn add_still_image_layer(
    state: State<'_, AppState>,
    label: String,
    path: String,
) -> Result<VideoLayerId, String> {
    let label = normalize_video_layer_label(label)?;
    let path = validate_existing_file_path(path, "Still image")?;
    let metadata = video::probe_still_image_metadata(&path).ok();
    let layer_id = state.engine.allocate_video_layer_id();
    state
        .engine
        .send(EngineCommand::AddVideoLayer {
            layer_id,
            label,
            source: VideoSourceSummary {
                kind: VideoSourceKind::StillImage,
                path: Some(path),
                name: None,
                codec: None,
                metadata,
            },
        })
        .map_err(|error| error.to_string())?;
    Ok(layer_id)
}

#[tauri::command]
fn refresh_video_layer_metadata(
    state: State<'_, AppState>,
    layer_id: VideoLayerId,
) -> Result<String, String> {
    let snapshot = state.engine.snapshot();
    let layer = snapshot
        .video
        .layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .ok_or_else(|| format!("Video layer {layer_id} was not found"))?;
    let (source, message) = refresh_video_source_metadata(&layer.source)?;
    state
        .engine
        .send(EngineCommand::SetVideoLayerSource { layer_id, source })
        .map_err(|error| error.to_string())?;
    Ok(message)
}

fn refresh_video_source_metadata(
    source: &VideoSourceSummary,
) -> Result<(VideoSourceSummary, String), String> {
    match source.kind {
        VideoSourceKind::File => refresh_file_video_source_metadata(source),
        VideoSourceKind::StillImage => refresh_still_image_source_metadata(source),
        VideoSourceKind::Ndi | VideoSourceKind::Spout | VideoSourceKind::Syphon => {
            Err("External video inputs do not expose local file metadata in this build".to_string())
        }
    }
}

fn refresh_file_video_source_metadata(
    source: &VideoSourceSummary,
) -> Result<(VideoSourceSummary, String), String> {
    let path = source
        .path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| "Video file source path is required".to_string())?;
    let path = validate_existing_file_path(path.to_string(), "Video file")?;
    let probe = video::probe_video_file_metadata(&path);
    let (codec, metadata, message) = match probe {
        Ok(probe) => {
            let codec = probe
                .codec
                .or_else(|| video::infer_video_codec_from_path(&path))
                .or_else(|| source.codec.clone());
            let message = if probe.metadata.is_some() {
                "Refreshed video file metadata".to_string()
            } else {
                "Refreshed video codec; ffprobe returned no stream metadata".to_string()
            };
            (codec, probe.metadata, message)
        }
        Err(error) => (
            video::infer_video_codec_from_path(&path).or_else(|| source.codec.clone()),
            None,
            format!("Refreshed video codec fallback; ffprobe failed: {error:?}"),
        ),
    };
    Ok((
        VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some(path),
            name: source.name.clone(),
            codec,
            metadata,
        },
        message,
    ))
}

fn refresh_still_image_source_metadata(
    source: &VideoSourceSummary,
) -> Result<(VideoSourceSummary, String), String> {
    let path = source
        .path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| "Still image source path is required".to_string())?;
    let path = validate_existing_file_path(path.to_string(), "Still image")?;
    let metadata = video::probe_still_image_metadata(&path)
        .map_err(|error| format!("Still image metadata probe failed: {error:?}"))?;
    Ok((
        VideoSourceSummary {
            kind: VideoSourceKind::StillImage,
            path: Some(path),
            name: source.name.clone(),
            codec: source.codec.clone(),
            metadata: Some(metadata),
        },
        "Refreshed still image metadata".to_string(),
    ))
}

#[tauri::command]
fn add_video_input_layer(
    state: State<'_, AppState>,
    label: String,
    kind: VideoSourceKind,
    name: String,
) -> Result<VideoLayerId, String> {
    let label = normalize_video_layer_label(label)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Video input source name is required".to_string());
    }
    if !matches!(
        kind,
        VideoSourceKind::Ndi | VideoSourceKind::Spout | VideoSourceKind::Syphon
    ) {
        return Err("Video input kind must be NDI, Spout, or Syphon".to_string());
    }
    ensure_video_input_backend_available(&kind)?;
    let layer_id = state.engine.allocate_video_layer_id();
    state
        .engine
        .send(EngineCommand::AddVideoLayer {
            layer_id,
            label,
            source: VideoSourceSummary {
                kind,
                path: None,
                name: Some(name),
                codec: None,
                metadata: None,
            },
        })
        .map_err(|error| error.to_string())?;
    Ok(layer_id)
}

#[tauri::command]
fn duplicate_video_layer(
    state: State<'_, AppState>,
    source_layer_id: VideoLayerId,
    label: String,
) -> Result<VideoLayerId, String> {
    if !state
        .engine
        .snapshot()
        .video
        .layers
        .iter()
        .any(|layer| layer.id == source_layer_id)
    {
        return Err(format!("Video layer {source_layer_id} was not found"));
    }
    let label = normalize_video_layer_label(label)?;
    let new_layer_id = state.engine.allocate_video_layer_id();
    state
        .engine
        .send(EngineCommand::DuplicateVideoLayer {
            source_layer_id,
            new_layer_id,
            label,
        })
        .map_err(|error| error.to_string())?;
    Ok(new_layer_id)
}

#[tauri::command]
fn remove_video_layer(state: State<'_, AppState>, layer_id: VideoLayerId) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::RemoveVideoLayer(layer_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_layer_order(
    state: State<'_, AppState>,
    layer_ids: Vec<VideoLayerId>,
) -> Result<(), String> {
    validate_video_layer_ids(&state.engine.snapshot(), &layer_ids)?;
    state
        .engine
        .send(EngineCommand::SetVideoLayerOrder(layer_ids))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_layer_label(
    state: State<'_, AppState>,
    layer_id: VideoLayerId,
    label: String,
) -> Result<(), String> {
    let label = normalize_video_layer_label(label)?;
    state
        .engine
        .send(EngineCommand::SetVideoLayerLabel { layer_id, label })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_layer_state(
    state: State<'_, AppState>,
    layer_id: VideoLayerId,
    state_value: VideoLayerState,
) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetVideoLayerState {
            layer_id,
            state: state_value,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn fade_video_layer_opacity(
    state: State<'_, AppState>,
    layer_id: VideoLayerId,
    opacity: f32,
    duration_ms: u64,
) -> Result<(), String> {
    if !opacity.is_finite() {
        return Err("Video layer opacity must be finite".to_string());
    }
    state
        .engine
        .send(EngineCommand::FadeVideoLayerOpacity {
            layer_id,
            opacity,
            duration_ms,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_video_cue_point(
    state: State<'_, AppState>,
    layer_id: VideoLayerId,
    position_ms: Option<u64>,
) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::AddVideoCuePoint {
            layer_id,
            position_ms,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn remove_video_cue_point(
    state: State<'_, AppState>,
    layer_id: VideoLayerId,
    position_ms: u64,
) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::RemoveVideoCuePoint {
            layer_id,
            position_ms,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_cue_point(
    state: State<'_, AppState>,
    layer_id: VideoLayerId,
    cue_point_index: usize,
    position_ms: u64,
    label: String,
    color: Option<String>,
) -> Result<(), String> {
    let label = label.trim().to_string();
    if label.is_empty() {
        return Err("Video cue point label is required".to_string());
    }
    if let Some(color) = color.as_deref() {
        validate_video_cue_point_color(color)?;
    }
    state
        .engine
        .send(EngineCommand::SetVideoCuePoint {
            layer_id,
            cue_point_index,
            position_ms,
            label,
            color,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn jump_video_cue_point(
    state: State<'_, AppState>,
    layer_id: VideoLayerId,
    cue_point_index: usize,
) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::JumpVideoCuePoint {
            layer_id,
            cue_point_index,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn jump_video_cue_point_relative(
    state: State<'_, AppState>,
    layer_id: VideoLayerId,
    direction: i32,
) -> Result<(), String> {
    if direction == 0 {
        return Err("Video cue point direction must be non-zero".to_string());
    }
    state
        .engine
        .send(EngineCommand::JumpVideoCuePointRelative {
            layer_id,
            direction,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_layer_blend_mode(
    state: State<'_, AppState>,
    layer_id: VideoLayerId,
    blend_mode: VideoBlendMode,
) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetVideoLayerBlendMode {
            layer_id,
            blend_mode,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_master_opacity(state: State<'_, AppState>, opacity: f32) -> Result<(), String> {
    if !opacity.is_finite() {
        return Err("Video master opacity must be finite".to_string());
    }
    state
        .engine
        .send(EngineCommand::SetVideoMasterOpacity(opacity))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_blackout(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetVideoBlackout(enabled))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_video_composition(
    state: State<'_, AppState>,
    label: String,
    layer_ids: Vec<VideoLayerId>,
) -> Result<CompositionId, String> {
    if label.trim().is_empty() {
        return Err("Video composition label is required".to_string());
    }
    validate_video_layer_ids(&state.engine.snapshot(), &layer_ids)?;
    let composition_id = state.engine.allocate_composition_id();
    state
        .engine
        .send(EngineCommand::AddVideoComposition(CompositionSummary {
            id: composition_id,
            label,
            layer_ids,
            output_ids: Vec::new(),
        }))
        .map_err(|error| error.to_string())?;
    Ok(composition_id)
}

#[tauri::command]
fn remove_video_composition(
    state: State<'_, AppState>,
    composition_id: CompositionId,
) -> Result<(), String> {
    let snapshot = state.engine.snapshot();
    validate_editable_video_composition(&snapshot, composition_id)?;
    state
        .engine
        .send(EngineCommand::RemoveVideoComposition(composition_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_composition_layers(
    state: State<'_, AppState>,
    composition_id: CompositionId,
    layer_ids: Vec<VideoLayerId>,
) -> Result<(), String> {
    let snapshot = state.engine.snapshot();
    validate_editable_video_composition(&snapshot, composition_id)?;
    validate_video_layer_ids(&snapshot, &layer_ids)?;
    state
        .engine
        .send(EngineCommand::SetVideoCompositionLayers {
            composition_id,
            layer_ids,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_video_output(
    state: State<'_, AppState>,
    label: String,
    kind: VideoOutputKind,
    width: u32,
    height: u32,
    fullscreen: bool,
    monitor_id: Option<u32>,
    endpoint_name: Option<String>,
) -> Result<VideoOutputId, String> {
    let config = normalize_video_output_config(
        label,
        kind,
        width,
        height,
        fullscreen,
        monitor_id,
        endpoint_name,
    )?;
    ensure_video_output_backend_available(&config.kind)?;
    let output_id = state.engine.allocate_video_output_id();
    state
        .engine
        .send(EngineCommand::AddVideoOutput(VideoOutputSummary {
            id: output_id,
            label: config.label,
            kind: config.kind,
            enabled: true,
            composition_id: 1,
            fullscreen: config.fullscreen,
            monitor_id: config.monitor_id,
            width: config.width,
            height: config.height,
            endpoint_name: config.endpoint_name,
            opacity: 1.0,
            blackout: false,
            mapping: VideoOutputMapping::default(),
        }))
        .map_err(|error| error.to_string())?;
    Ok(output_id)
}

#[tauri::command]
fn remove_video_output(state: State<'_, AppState>, output_id: VideoOutputId) -> Result<(), String> {
    validate_video_output_exists(&state.engine.snapshot(), output_id)?;
    state
        .engine
        .send(EngineCommand::RemoveVideoOutput(output_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_output_config(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    label: String,
    kind: VideoOutputKind,
    width: u32,
    height: u32,
    fullscreen: bool,
    monitor_id: Option<u32>,
    endpoint_name: Option<String>,
) -> Result<(), String> {
    validate_video_output_exists(&state.engine.snapshot(), output_id)?;
    let config = normalize_video_output_config(
        label,
        kind,
        width,
        height,
        fullscreen,
        monitor_id,
        endpoint_name,
    )?;
    ensure_video_output_backend_available(&config.kind)?;
    state
        .engine
        .send(EngineCommand::SetVideoOutputConfig {
            output_id,
            label: config.label,
            kind: config.kind,
            fullscreen: config.fullscreen,
            monitor_id: config.monitor_id,
            width: config.width,
            height: config.height,
            endpoint_name: config.endpoint_name,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_output_enabled(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    enabled: bool,
) -> Result<(), String> {
    validate_video_output_exists(&state.engine.snapshot(), output_id)?;
    state
        .engine
        .send(EngineCommand::SetVideoOutputEnabled { output_id, enabled })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_output_routing(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    composition_id: CompositionId,
) -> Result<(), String> {
    let snapshot = state.engine.snapshot();
    validate_video_output_exists(&snapshot, output_id)?;
    validate_video_composition_exists(&snapshot, composition_id)?;
    state
        .engine
        .send(EngineCommand::SetVideoOutputRouting {
            output_id,
            composition_id,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_output_opacity(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    opacity: f32,
) -> Result<(), String> {
    if !opacity.is_finite() {
        return Err("Video output opacity must be finite".to_string());
    }
    validate_video_output_exists(&state.engine.snapshot(), output_id)?;
    state
        .engine
        .send(EngineCommand::SetVideoOutputOpacity { output_id, opacity })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn fade_video_output_opacity(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    opacity: f32,
    duration_ms: u64,
) -> Result<(), String> {
    if !opacity.is_finite() {
        return Err("Video output opacity must be finite".to_string());
    }
    validate_video_output_exists(&state.engine.snapshot(), output_id)?;
    state
        .engine
        .send(EngineCommand::FadeVideoOutputOpacity {
            output_id,
            opacity,
            duration_ms,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_output_blackout(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    blackout: bool,
) -> Result<(), String> {
    validate_video_output_exists(&state.engine.snapshot(), output_id)?;
    state
        .engine
        .send(EngineCommand::SetVideoOutputBlackout {
            output_id,
            blackout,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_output_mapping(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    mapping: VideoOutputMapping,
) -> Result<(), String> {
    validate_video_output_exists(&state.engine.snapshot(), output_id)?;
    validate_video_output_mapping(&mapping)?;
    state
        .engine
        .send(EngineCommand::SetVideoOutputMapping { output_id, mapping })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_video_output_mapping_field(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    field: String,
    value: f32,
) -> Result<(), String> {
    validate_video_output_exists(&state.engine.snapshot(), output_id)?;
    let field = normalize_video_output_mapping_field(field)?;
    if !value.is_finite() {
        return Err("Video output mapping value must be finite".to_string());
    }
    state
        .engine
        .send(EngineCommand::SetVideoOutputMappingField {
            output_id,
            field,
            value,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_video_output_mapping_preset(
    state: State<'_, AppState>,
    label: String,
    mapping: VideoOutputMapping,
) -> Result<String, String> {
    let label = normalize_video_output_mapping_preset_label(label)?;
    validate_video_output_mapping(&mapping)?;
    state
        .engine
        .send(EngineCommand::SaveVideoOutputMappingPreset {
            label: label.clone(),
            mapping,
        })
        .map_err(|error| error.to_string())?;
    Ok(label)
}

#[tauri::command]
fn apply_video_output_mapping_preset(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    label: String,
) -> Result<(), String> {
    validate_video_output_exists(&state.engine.snapshot(), output_id)?;
    let label = normalize_video_output_mapping_preset_label(label)?;
    state
        .engine
        .send(EngineCommand::ApplyVideoOutputMappingPreset { output_id, label })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn remove_video_output_mapping_preset(
    state: State<'_, AppState>,
    label: String,
) -> Result<(), String> {
    let label = normalize_video_output_mapping_preset_label(label)?;
    state
        .engine
        .send(EngineCommand::RemoveVideoOutputMappingPreset { label })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_video_output_mapping_preset_file(
    label: String,
    mapping: VideoOutputMapping,
) -> Result<Option<String>, String> {
    let label = normalize_video_output_mapping_preset_label(label)?;
    validate_video_output_mapping(&mapping)?;
    let file = VideoOutputMappingPresetFile {
        version: 1,
        app: APP_NAME.to_string(),
        preset: VideoOutputMappingPresetSummary {
            label: label.clone(),
            mapping,
        },
    };
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Projector Map", &["projmap"])
        .set_file_name(format!("{}.projmap", safe_file_stem(&label)))
        .save_file()
    else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(&file).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn load_video_output_mapping_preset_file(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Projector Map", &["projmap"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let file: VideoOutputMappingPresetFile =
        serde_json::from_str(&json).map_err(|error| error.to_string())?;
    validate_video_output_mapping_preset_file(&file)?;
    let label = file.preset.label.clone();
    state
        .engine
        .send(EngineCommand::SaveVideoOutputMappingPreset {
            label: file.preset.label,
            mapping: file.preset.mapping,
        })
        .map_err(|error| error.to_string())?;
    Ok(Some(label))
}

#[tauri::command]
fn add_lfo_effect(
    state: State<'_, AppState>,
    request: LfoEffectRequest,
) -> Result<EffectId, String> {
    validate_lfo_effect_request(&request)?;
    let effect_id = state.engine.allocate_effect_id();
    state
        .engine
        .send(EngineCommand::AddLfoEffect { effect_id, request })
        .map_err(|error| error.to_string())?;
    Ok(effect_id)
}

#[tauri::command]
fn add_position_wave_effect(
    state: State<'_, AppState>,
    request: PositionWaveEffectRequest,
) -> Result<EffectId, String> {
    validate_position_wave_effect_request(&request)?;
    let effect_id = state.engine.allocate_effect_id();
    state
        .engine
        .send(EngineCommand::AddPositionWaveEffect { effect_id, request })
        .map_err(|error| error.to_string())?;
    Ok(effect_id)
}

#[tauri::command]
fn update_lfo_effect(
    state: State<'_, AppState>,
    effect_id: EffectId,
    request: LfoEffectRequest,
) -> Result<(), String> {
    validate_lfo_effect_request(&request)?;
    if !state
        .engine
        .snapshot()
        .effects
        .iter()
        .any(|effect| effect.id == effect_id)
    {
        return Err(format!("Effect {effect_id} was not found"));
    }
    state
        .engine
        .send(EngineCommand::UpdateLfoEffect { effect_id, request })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn update_position_wave_effect(
    state: State<'_, AppState>,
    effect_id: EffectId,
    request: PositionWaveEffectRequest,
) -> Result<(), String> {
    validate_position_wave_effect_request(&request)?;
    if !state
        .engine
        .snapshot()
        .effects
        .iter()
        .any(|effect| effect.id == effect_id)
    {
        return Err(format!("Effect {effect_id} was not found"));
    }
    state
        .engine
        .send(EngineCommand::UpdatePositionWaveEffect { effect_id, request })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_node_graph(
    state: State<'_, AppState>,
    mut graph: NodeGraphSummary,
) -> Result<NodeGraphId, String> {
    if graph.id == 0 {
        graph.id = state.engine.allocate_node_graph_id();
    }
    validate_node_graph_summary(&graph, &state.engine.snapshot())?;
    state
        .engine
        .send(EngineCommand::UpsertNodeGraph(graph.clone()))
        .map_err(|error| error.to_string())?;
    Ok(graph.id)
}

#[tauri::command]
fn set_node_graph_enabled(
    state: State<'_, AppState>,
    graph_id: NodeGraphId,
    enabled: bool,
) -> Result<(), String> {
    if !state
        .engine
        .snapshot()
        .node_graphs
        .iter()
        .any(|graph| graph.id == graph_id)
    {
        return Err(format!("Node graph {graph_id} was not found"));
    }
    state
        .engine
        .send(EngineCommand::SetNodeGraphEnabled { graph_id, enabled })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn remove_node_graph(state: State<'_, AppState>, graph_id: NodeGraphId) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::RemoveNodeGraph(graph_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_node_graph_preset_file(
    state: State<'_, AppState>,
    graph_id: NodeGraphId,
) -> Result<Option<String>, String> {
    let snapshot = state.engine.snapshot();
    let graph = snapshot
        .node_graphs
        .iter()
        .find(|graph| graph.id == graph_id)
        .ok_or_else(|| format!("Node graph {graph_id} was not found"))?;
    let file = NodeGraphPresetFile {
        version: 1,
        app: APP_NAME.to_string(),
        graph: graph.clone(),
    };
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Node Graph", &["graph"])
        .set_file_name(format!("{}.graph", safe_file_stem(&graph.label)))
        .save_file()
    else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(&file).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn load_node_graph_preset_file(state: State<'_, AppState>) -> Result<Option<NodeGraphId>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Node Graph", &["graph"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let file: NodeGraphPresetFile =
        serde_json::from_str(&json).map_err(|error| error.to_string())?;
    validate_node_graph_preset_file(&file, &state.engine.snapshot())?;
    let mut graph = file.graph;
    graph.id = state.engine.allocate_node_graph_id();
    state
        .engine
        .send(EngineCommand::UpsertNodeGraph(graph.clone()))
        .map_err(|error| error.to_string())?;
    Ok(Some(graph.id))
}

#[tauri::command]
fn set_effect_enabled(
    state: State<'_, AppState>,
    effect_id: EffectId,
    enabled: bool,
) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetEffectEnabled { effect_id, enabled })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_effect_video_target_position(
    state: State<'_, AppState>,
    effect_id: EffectId,
    layer_id: VideoLayerId,
    position: Vec3,
) -> Result<(), String> {
    if !position.x.is_finite() || !position.y.is_finite() || !position.z.is_finite() {
        return Err("Video effect target position values must be finite".to_string());
    }
    let snapshot = state.engine.snapshot();
    let effect = snapshot
        .effects
        .iter()
        .find(|effect| effect.id == effect_id)
        .ok_or_else(|| format!("Effect {effect_id} was not found"))?;
    validate_video_layer_ids(&snapshot, &[layer_id])?;
    if !effect
        .video_targets
        .iter()
        .any(|target| target.layer_ids.contains(&layer_id))
    {
        return Err(format!(
            "Video target for layer {layer_id} was not found on effect {effect_id}"
        ));
    }
    state
        .engine
        .send(EngineCommand::SetEffectVideoTargetPosition {
            effect_id,
            layer_id,
            position,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn move_effect(state: State<'_, AppState>, effect_id: EffectId, delta: i32) -> Result<(), String> {
    if delta != -1 && delta != 1 {
        return Err("Effect move delta must be -1 or 1".to_string());
    }
    state
        .engine
        .send(EngineCommand::MoveEffect { effect_id, delta })
        .map_err(|error| error.to_string())
}

fn relabel_effect_preset(mut preset: EffectPreset, label: String) -> EffectPreset {
    match preset.effect_type {
        EffectKind::Lfo => {
            if let Some(request) = &mut preset.lfo {
                request.label = label;
            }
        }
        EffectKind::PositionWave => {
            if let Some(request) = &mut preset.position_wave {
                request.label = label;
            }
        }
    }
    preset
}

fn duplicate_effect_in_engine(
    engine: &EngineHandle,
    effect_id: EffectId,
) -> Result<EffectId, String> {
    let snapshot = engine.snapshot();
    let effect = snapshot
        .effects
        .iter()
        .find(|effect| effect.id == effect_id)
        .ok_or_else(|| format!("Effect {effect_id} was not found"))?;
    let preset = effect_summary_to_preset(effect)?;
    let preset = relabel_effect_preset(preset, format!("{} Copy", effect.label));
    add_effect_preset_to_engine(engine, preset, None)
}

#[tauri::command]
fn duplicate_effect(state: State<'_, AppState>, effect_id: EffectId) -> Result<EffectId, String> {
    duplicate_effect_in_engine(&state.engine, effect_id)
}

#[tauri::command]
fn remove_effect(state: State<'_, AppState>, effect_id: EffectId) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::RemoveEffect(effect_id))
        .map_err(|error| error.to_string())
}

fn sample_effect_preset_json(preset: &str) -> Result<(&'static str, &'static str), String> {
    match preset.trim().to_ascii_lowercase().as_str() {
        "pulse" | "front-dimmer-pulse" | "front_dimmer_pulse" => Ok((
            SAMPLE_EFFECT_PRESET_PULSE_LABEL,
            SAMPLE_EFFECT_PRESET_PULSE_JSON,
        )),
        "shared" | "front-dimmer-shared" | "front_dimmer_shared" => Ok((
            SAMPLE_EFFECT_PRESET_SHARED_LABEL,
            SAMPLE_EFFECT_PRESET_SHARED_JSON,
        )),
        "wave" | "front-dimmer-wave" | "front_dimmer_wave" => Ok((
            SAMPLE_EFFECT_PRESET_WAVE_LABEL,
            SAMPLE_EFFECT_PRESET_WAVE_JSON,
        )),
        "flash" | "front-dimmer-flash" | "front_dimmer_flash" => Ok((
            SAMPLE_EFFECT_PRESET_FLASH_LABEL,
            SAMPLE_EFFECT_PRESET_FLASH_JSON,
        )),
        "random" | "front-dimmer-random" | "front_dimmer_random" => Ok((
            SAMPLE_EFFECT_PRESET_RANDOM_LABEL,
            SAMPLE_EFFECT_PRESET_RANDOM_JSON,
        )),
        "perlin" | "front-dimmer-perlin" | "front_dimmer_perlin" => Ok((
            SAMPLE_EFFECT_PRESET_PERLIN_LABEL,
            SAMPLE_EFFECT_PRESET_PERLIN_JSON,
        )),
        "chase" | "front-dimmer-chase" | "front_dimmer_chase" => Ok((
            SAMPLE_EFFECT_PRESET_CHASE_LABEL,
            SAMPLE_EFFECT_PRESET_CHASE_JSON,
        )),
        "ball" | "front-dimmer-ball" | "front_dimmer_ball" => Ok((
            SAMPLE_EFFECT_PRESET_BALL_LABEL,
            SAMPLE_EFFECT_PRESET_BALL_JSON,
        )),
        "fan" | "front-pan-fan" | "front_pan_fan" => {
            Ok((SAMPLE_EFFECT_PRESET_FAN_LABEL, SAMPLE_EFFECT_PRESET_FAN_JSON))
        }
        value => Err(format!(
            "Unknown sample effect preset '{value}'. Expected 'pulse', 'shared', 'wave', 'flash', 'random', 'perlin', 'chase', 'ball', or 'fan'."
        )),
    }
}

fn sample_effect_bundle_jsons(preset: &str) -> Result<Vec<(&'static str, &'static str)>, String> {
    match preset.trim().to_ascii_lowercase().as_str() {
        "circle" | "front-circle" | "front_circle" => Ok(vec![
            (
                SAMPLE_EFFECT_PRESET_CIRCLE_PAN_LABEL,
                SAMPLE_EFFECT_PRESET_CIRCLE_PAN_JSON,
            ),
            (
                SAMPLE_EFFECT_PRESET_CIRCLE_TILT_LABEL,
                SAMPLE_EFFECT_PRESET_CIRCLE_TILT_JSON,
            ),
        ]),
        value => Err(format!(
            "Unknown sample effect bundle '{value}'. Expected 'circle'."
        )),
    }
}

fn add_effect_preset_to_engine(
    engine: &EngineHandle,
    preset: EffectPreset,
    target_override: Option<&EffectTargetOverride>,
) -> Result<EffectId, String> {
    let snapshot = engine.snapshot();
    let effect_id = engine.allocate_effect_id();
    let enabled = preset.enabled;
    match preset.effect_type {
        EffectKind::Lfo => {
            let request = preset
                .lfo
                .ok_or_else(|| "LFO effect preset is missing its request body".to_string())?;
            let request = match target_override {
                Some(target_override) => {
                    let request = apply_lfo_effect_target_override(request, target_override);
                    validate_lfo_effect_request(&request)?;
                    request
                }
                None => request,
            };
            validate_effect_target_references(
                &snapshot,
                &request.fixture_ids,
                &request.video_targets,
            )?;
            engine
                .send(EngineCommand::AddLfoEffect { effect_id, request })
                .map_err(|error| error.to_string())?;
        }
        EffectKind::PositionWave => {
            let request = preset.position_wave.ok_or_else(|| {
                "Position wave effect preset is missing its request body".to_string()
            })?;
            let request = match target_override {
                Some(target_override) => {
                    let request =
                        apply_position_wave_effect_target_override(request, target_override);
                    validate_position_wave_effect_request(&request)?;
                    request
                }
                None => request,
            };
            validate_effect_target_references(
                &snapshot,
                &request.fixture_ids,
                &request.video_targets,
            )?;
            engine
                .send(EngineCommand::AddPositionWaveEffect { effect_id, request })
                .map_err(|error| error.to_string())?;
        }
    }
    if !enabled {
        engine
            .send(EngineCommand::SetEffectEnabled {
                effect_id,
                enabled: false,
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(effect_id)
}

#[tauri::command]
fn load_sample_effect_preset(
    state: State<'_, AppState>,
    preset: String,
    target_override: Option<EffectTargetOverride>,
) -> Result<EffectId, String> {
    let (label, json) = sample_effect_preset_json(&preset)?;
    let preset: EffectPreset =
        serde_json::from_str(json).map_err(|error| format!("Failed to parse {label}: {error}"))?;
    validate_effect_preset(&preset)?;
    if let Some(target_override) = &target_override {
        validate_effect_target_override(target_override)?;
    }
    add_effect_preset_to_engine(&state.engine, preset, target_override.as_ref())
}

#[tauri::command]
fn load_sample_effect_bundle(
    state: State<'_, AppState>,
    preset: String,
) -> Result<Vec<EffectId>, String> {
    let mut effect_ids = Vec::new();
    for (label, json) in sample_effect_bundle_jsons(&preset)? {
        let preset: EffectPreset = serde_json::from_str(json)
            .map_err(|error| format!("Failed to parse {label}: {error}"))?;
        validate_effect_preset(&preset)?;
        let effect_id = add_effect_preset_to_engine(&state.engine, preset, None)?;
        effect_ids.push(effect_id);
    }
    Ok(effect_ids)
}

#[tauri::command]
fn save_effect_preset(
    state: State<'_, AppState>,
    effect_id: EffectId,
) -> Result<Option<String>, String> {
    let snapshot = state.engine.snapshot();
    let effect = snapshot
        .effects
        .iter()
        .find(|effect| effect.id == effect_id)
        .ok_or_else(|| format!("Effect {effect_id} was not found"))?;
    let preset = effect_summary_to_preset(effect)?;
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Effect", &["effect"])
        .set_file_name(format!("{}.effect", safe_file_stem(&effect.label)))
        .save_file()
    else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(&preset).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn load_effect_preset(state: State<'_, AppState>) -> Result<Option<EffectId>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Effect", &["effect"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let preset: EffectPreset = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    validate_effect_preset(&preset)?;
    add_effect_preset_to_engine(&state.engine, preset, None).map(Some)
}

#[tauri::command]
fn load_effect_preset_for_target(
    state: State<'_, AppState>,
    target_override: EffectTargetOverride,
) -> Result<Option<EffectId>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Effect", &["effect"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let preset: EffectPreset = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    validate_effect_preset(&preset)?;
    validate_effect_target_override(&target_override)?;

    add_effect_preset_to_engine(&state.engine, preset, Some(&target_override)).map(Some)
}

#[tauri::command]
fn save_fixture_preset(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
) -> Result<Option<String>, String> {
    let snapshot = state.engine.snapshot();
    let fixture = snapshot
        .fixtures
        .iter()
        .find(|fixture| fixture.id == fixture_id)
        .ok_or_else(|| format!("Fixture {fixture_id} was not found"))?;
    let preset = FixturePreset {
        version: 1,
        manufacturer: fixture.manufacturer.clone(),
        profile_name: fixture.profile_name.clone(),
        profile_source_path: non_empty_string(fixture.profile_source_path.clone()),
        mode_name: fixture.mode_name.clone(),
        values: fixture.attribute_values.clone(),
    };
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Preset", &["preset"])
        .set_file_name(format!("{}.preset", safe_file_stem(&fixture.label)))
        .save_file()
    else {
        return Ok(None);
    };
    let path = normalize_fixture_preset_save_path(path)?;
    let json = serde_json::to_string_pretty(&preset).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn load_fixture_preset(
    state: State<'_, AppState>,
    fixture_id: FixtureId,
) -> Result<Option<String>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Preset", &["preset"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let preset: FixturePreset = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    let snapshot = state.engine.snapshot();
    let fixture = snapshot
        .fixtures
        .iter()
        .find(|fixture| fixture.id == fixture_id)
        .ok_or_else(|| format!("Fixture {fixture_id} was not found"))?;
    validate_fixture_preset(
        &preset,
        &fixture.manufacturer,
        &fixture.profile_name,
        &fixture.mode_name,
        fixture
            .controls
            .iter()
            .map(|control| control.attribute.as_str()),
    )?;
    state
        .engine
        .send(EngineCommand::ApplyAttributeValues {
            fixture_id,
            values: preset.values,
        })
        .map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn load_fixture_preset_for_group(
    state: State<'_, AppState>,
    group_id: String,
) -> Result<Option<FixturePresetGroupLoadResult>, String> {
    let group_id = normalize_control_group_id(group_id)?;
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Preset", &["preset"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let preset: FixturePreset = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    if preset.version != 1 {
        return Err(format!("Unsupported preset version {}", preset.version));
    }

    let snapshot = state.engine.snapshot();
    let group_fixtures = snapshot
        .fixtures
        .iter()
        .filter(|fixture| {
            fixture
                .group_ids
                .iter()
                .any(|candidate| group_matches(candidate, &group_id))
        })
        .collect::<Vec<_>>();
    if group_fixtures.is_empty() {
        return Err(format!("Group {group_id} has no fixtures"));
    }

    let (fixture_ids, skipped_count) = compatible_fixture_preset_targets(&preset, group_fixtures);

    if fixture_ids.is_empty() {
        return Err(format!(
            "Preset is not compatible with any fixtures in group {group_id}"
        ));
    }

    for fixture_id in &fixture_ids {
        state
            .engine
            .send(EngineCommand::ApplyAttributeValues {
                fixture_id: *fixture_id,
                values: preset.values.clone(),
            })
            .map_err(|error| error.to_string())?;
    }

    Ok(Some(FixturePresetGroupLoadResult {
        path: path.to_string_lossy().to_string(),
        applied_count: fixture_ids.len(),
        skipped_count,
    }))
}

#[tauri::command]
fn load_fixture_preset_for_all_matching(
    state: State<'_, AppState>,
) -> Result<Option<FixturePresetGroupLoadResult>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Preset", &["preset"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let preset: FixturePreset = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    if preset.version != 1 {
        return Err(format!("Unsupported preset version {}", preset.version));
    }

    let snapshot = state.engine.snapshot();
    let all_fixtures = snapshot.fixtures.iter().collect::<Vec<_>>();
    let (fixture_ids, skipped_count) = compatible_fixture_preset_targets(&preset, all_fixtures);
    if fixture_ids.is_empty() {
        return Err("Preset is not compatible with any patched fixtures".to_string());
    }

    for fixture_id in &fixture_ids {
        state
            .engine
            .send(EngineCommand::ApplyAttributeValues {
                fixture_id: *fixture_id,
                values: preset.values.clone(),
            })
            .map_err(|error| error.to_string())?;
    }

    Ok(Some(FixturePresetGroupLoadResult {
        path: path.to_string_lossy().to_string(),
        applied_count: fixture_ids.len(),
        skipped_count,
    }))
}

#[tauri::command]
fn new_project(state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut custom_profiles = state
            .custom_profiles
            .lock()
            .map_err(|_| "Custom profile state lock was poisoned".to_string())?;
        custom_profiles.clear();
    }
    {
        let mut current_path = state
            .current_project_path
            .lock()
            .map_err(|_| "Current project path lock was poisoned".to_string())?;
        *current_path = None;
    }
    state
        .engine
        .load_project_snapshot(EngineSnapshot::default())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_project(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let current_path = state
        .current_project_path
        .lock()
        .map_err(|_| "Current project path lock was poisoned".to_string())?
        .clone();
    if let Some(path) = current_path {
        write_project_file(&state, &path)?;
        return Ok(Some(path.to_string_lossy().to_string()));
    }
    save_project_with_dialog(&state)
}

#[tauri::command]
fn save_project_as(state: State<'_, AppState>) -> Result<Option<String>, String> {
    save_project_with_dialog(&state)
}

fn save_project_with_dialog(state: &State<'_, AppState>) -> Result<Option<String>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Project", &["ry"])
        .set_file_name("show.ry")
        .save_file()
    else {
        return Ok(None);
    };
    let path = normalize_project_save_path(path)?;
    write_project_file(state, &path)?;
    set_current_project_path(state, &path)?;
    Ok(Some(path.to_string_lossy().to_string()))
}

fn write_project_file(state: &State<'_, AppState>, path: &Path) -> Result<(), String> {
    let project = project_file_for_save(state)?;
    let json = serde_json::to_string_pretty(&project).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| error.to_string())
}

fn project_file_for_save(state: &State<'_, AppState>) -> Result<ProjectFile, String> {
    let mut custom_profiles = state
        .custom_profiles
        .lock()
        .map_err(|_| "Custom profile state lock was poisoned".to_string())?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    custom_profiles.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(ProjectFile {
        version: 1,
        app: APP_NAME.to_string(),
        custom_profiles,
        snapshot: project_snapshot_for_save(state.engine.snapshot()),
    })
}

fn set_current_project_path(state: &State<'_, AppState>, path: &Path) -> Result<(), String> {
    let mut current_path = state
        .current_project_path
        .lock()
        .map_err(|_| "Current project path lock was poisoned".to_string())?;
    *current_path = Some(path.to_path_buf());
    Ok(())
}

fn clear_current_project_path(state: &State<'_, AppState>) -> Result<(), String> {
    let mut current_path = state
        .current_project_path
        .lock()
        .map_err(|_| "Current project path lock was poisoned".to_string())?;
    *current_path = None;
    Ok(())
}

fn project_snapshot_for_save(mut snapshot: EngineSnapshot) -> EngineSnapshot {
    snapshot.active_fade = None;
    snapshot.timeline.playing = false;
    snapshot.dmx_preview.clear();
    snapshot.dmx_previews.clear();
    snapshot.telemetry = EngineTelemetry::default();
    snapshot
}

#[tauri::command]
fn load_project(state: State<'_, AppState>) -> Result<Option<ProjectLoadResult>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Project", &["ry"])
        .pick_file()
    else {
        return Ok(None);
    };
    load_project_from_path(&state, &path).map(Some)
}

#[tauri::command]
fn load_project_path(
    state: State<'_, AppState>,
    path: String,
) -> Result<ProjectLoadResult, String> {
    let path = PathBuf::from(path);
    validate_project_open_path(&path)?;
    load_project_from_path(&state, &path)
}

#[tauri::command]
fn load_startup_project(state: State<'_, AppState>) -> Result<Option<ProjectLoadResult>, String> {
    let Some(path) = startup_project_path_from_args(env::args_os()) else {
        return Ok(None);
    };
    validate_project_open_path(&path)?;
    load_project_from_path(&state, &path).map(Some)
}

#[tauri::command]
fn load_phase1_sample_project(state: State<'_, AppState>) -> Result<ProjectLoadResult, String> {
    load_project_from_json(
        &state,
        PHASE1_SAMPLE_PROJECT_JSON,
        PHASE1_SAMPLE_PROJECT_LABEL.to_string(),
        None,
    )
}

fn phase1_smoke_first_8(snapshot: &EngineSnapshot) -> Vec<u8> {
    let preview = snapshot
        .dmx_previews
        .iter()
        .find(|candidate| candidate.universe == 0)
        .map(|candidate| candidate.values.as_slice())
        .unwrap_or(snapshot.dmx_preview.as_slice());
    let mut first_8 = preview.iter().copied().take(8).collect::<Vec<_>>();
    first_8.resize(8, 0);
    first_8
}

#[tauri::command]
fn run_phase1_smoke(state: State<'_, AppState>) -> Result<Phase1SmokeReport, String> {
    let result = load_project_from_json(
        &state,
        PHASE1_SAMPLE_PROJECT_JSON,
        PHASE1_SAMPLE_PROJECT_LABEL.to_string(),
        None,
    )?;
    let loaded = state.engine.snapshot();
    let cue = loaded
        .cues
        .first()
        .cloned()
        .ok_or_else(|| "Phase 1 sample has no cue to trigger".to_string())?;
    state
        .engine
        .send(EngineCommand::TriggerCue(cue.id))
        .map_err(|error| error.to_string())?;

    let mut snapshot = state.engine.snapshot();
    for _ in 0..30 {
        let first_8 = phase1_smoke_first_8(&snapshot);
        if snapshot.active_cue_id == Some(cue.id) && first_8 == PHASE1_SMOKE_EXPECTED_FIRST_8 {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
        snapshot = state.engine.snapshot();
    }

    let first_8 = phase1_smoke_first_8(&snapshot);
    let non_zero_first_8 = first_8.iter().filter(|value| **value != 0).count();
    let cue_video_layer = snapshot.video.layers.first().cloned();
    let video_layer_count = snapshot.video.layers.len();
    let video_layer_label = cue_video_layer.as_ref().map(|layer| layer.label.clone());
    let video_layer_playing = cue_video_layer
        .as_ref()
        .is_some_and(|layer| layer.state.playing);
    let video_layer_opacity = cue_video_layer.as_ref().map(|layer| layer.state.opacity);
    let timeline_event_count = snapshot.timeline.events.len();
    let timeline_automation_count = snapshot.timeline.automations.len();
    let timeline_video_automation_count = snapshot.timeline.video_automations.len();
    let timeline_duration_ms = snapshot.timeline.duration_ms;
    let dmx_output_count = snapshot.dmx_outputs.len();
    let enabled_dmx_output_count = snapshot
        .dmx_outputs
        .iter()
        .filter(|output| output.enabled)
        .count();
    let dmx_preview_universe_count = snapshot.dmx_previews.len();
    let primary_output = snapshot
        .dmx_outputs
        .iter()
        .find(|output| output.enabled)
        .or_else(|| snapshot.dmx_outputs.first())
        .unwrap_or(&snapshot.output);
    let primary_output_label = dmx_output_route_label(primary_output);

    let timeline_probe_ms = 2000;
    state
        .engine
        .send(EngineCommand::SeekTimeline(timeline_probe_ms))
        .map_err(|error| error.to_string())?;
    let mut probe_snapshot = state.engine.snapshot();
    for _ in 0..30 {
        if probe_snapshot.timeline.position_ms == timeline_probe_ms
            && phase1_smoke_first_8(&probe_snapshot).first() == Some(&128)
            && probe_snapshot
                .video
                .layers
                .first()
                .map(|layer| (0.49..=0.51).contains(&layer.state.opacity))
                == Some(true)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
        probe_snapshot = state.engine.snapshot();
    }
    let timeline_probe_dimmer_byte = phase1_smoke_first_8(&probe_snapshot)
        .first()
        .copied()
        .unwrap_or(0);
    let timeline_probe_video_opacity = probe_snapshot
        .video
        .layers
        .first()
        .map(|layer| layer.state.opacity);

    let passed = snapshot.active_cue_id == Some(cue.id)
        && first_8 == PHASE1_SMOKE_EXPECTED_FIRST_8
        && timeline_event_count >= 2
        && timeline_automation_count >= 1
        && timeline_video_automation_count >= 1
        && timeline_duration_ms >= 4000
        && dmx_output_count >= 1
        && enabled_dmx_output_count >= 1
        && dmx_preview_universe_count >= 1
        && video_layer_count >= 1
        && video_layer_playing
        && video_layer_opacity.is_some_and(|opacity| (opacity - 1.0).abs() < 0.001)
        && timeline_probe_dimmer_byte == 128
        && timeline_probe_video_opacity.is_some_and(|opacity| (0.49..=0.51).contains(&opacity));
    Ok(Phase1SmokeReport {
        path: result.path,
        cue_id: cue.id,
        cue_label: cue.label,
        active_cue_id: snapshot.active_cue_id,
        timeline_event_count,
        timeline_automation_count,
        timeline_video_automation_count,
        timeline_duration_ms,
        timeline_probe_ms,
        timeline_probe_dimmer_byte,
        timeline_probe_video_opacity,
        dmx_output_count,
        enabled_dmx_output_count,
        dmx_preview_universe_count,
        primary_output_label,
        video_layer_count,
        video_layer_label,
        video_layer_playing,
        video_layer_opacity,
        first_8,
        expected_first_8: PHASE1_SMOKE_EXPECTED_FIRST_8.to_vec(),
        non_zero_first_8,
        passed,
    })
}

#[tauri::command]
fn take_open_project_paths(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut paths = state
        .pending_project_open_paths
        .lock()
        .map_err(|_| "Open project path queue lock was poisoned".to_string())?;
    Ok(std::mem::take(&mut *paths))
}

fn load_project_from_path(
    state: &State<'_, AppState>,
    path: &Path,
) -> Result<ProjectLoadResult, String> {
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    load_project_from_json(state, &json, path.to_string_lossy().to_string(), Some(path))
}

fn load_project_from_json(
    state: &State<'_, AppState>,
    json: &str,
    path_label: String,
    current_path: Option<&Path>,
) -> Result<ProjectLoadResult, String> {
    let project: ProjectFile = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    validate_project_file(&project)?;
    let profiles = project.custom_profiles.clone();
    {
        let mut custom_profiles = state
            .custom_profiles
            .lock()
            .map_err(|_| "Custom profile state lock was poisoned".to_string())?;
        custom_profiles.clear();
        for profile in &project.custom_profiles {
            custom_profiles.insert(profile.source_path.clone(), profile.clone());
        }
    }
    state
        .engine
        .load_project_snapshot(project.snapshot)
        .map_err(|error| error.to_string())?;
    if let Some(path) = current_path {
        set_current_project_path(state, path)?;
    } else {
        clear_current_project_path(state)?;
    }
    Ok(ProjectLoadResult {
        path: path_label,
        profiles,
    })
}

fn normalize_project_save_path(mut path: PathBuf) -> Result<PathBuf, String> {
    if path.extension().is_none() {
        path.set_extension("ry");
        return Ok(path);
    }
    if is_rayard_project_path(&path) {
        Ok(path)
    } else {
        Err(format!(
            "Rayard project files must use the .ry extension: {}",
            path.to_string_lossy()
        ))
    }
}

fn validate_project_open_path(path: &Path) -> Result<(), String> {
    if !is_rayard_project_path(path) {
        return Err(format!(
            "Rayard project files must use the .ry extension: {}",
            path.to_string_lossy()
        ));
    }
    if !path.is_file() {
        return Err(format!(
            "Project file was not found: {}",
            path.to_string_lossy()
        ));
    }
    Ok(())
}

fn normalize_gdtf_save_path(mut path: PathBuf) -> Result<PathBuf, String> {
    if path.extension().is_none() {
        path.set_extension("gdtf");
        return Ok(path);
    }
    if has_extension(&path, "gdtf") {
        Ok(path)
    } else {
        Err(format!(
            "GDTF fixture files must use the .gdtf extension: {}",
            path.to_string_lossy()
        ))
    }
}

fn normalize_custom_fixture_profile_save_path(mut path: PathBuf) -> Result<PathBuf, String> {
    if path.extension().is_none() {
        path.set_extension("fixture");
        return Ok(path);
    }
    if has_extension(&path, "fixture") {
        Ok(path)
    } else {
        Err(format!(
            "Rayard fixture profile files must use the .fixture extension: {}",
            path.to_string_lossy()
        ))
    }
}

fn normalize_fixture_preset_save_path(mut path: PathBuf) -> Result<PathBuf, String> {
    if path.extension().is_none() {
        path.set_extension("preset");
        return Ok(path);
    }
    if is_rayard_fixture_preset_path(&path) {
        Ok(path)
    } else {
        Err(format!(
            "Rayard fixture preset files must use the .preset extension: {}",
            path.to_string_lossy()
        ))
    }
}

fn is_rayard_fixture_preset_path(path: &Path) -> bool {
    has_extension(path, "preset")
}

fn startup_project_path_from_args(args: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    project_paths_from_args(args, None).into_iter().next()
}

fn project_paths_from_single_instance_args(args: Vec<String>, cwd: &str) -> Vec<String> {
    let cwd_path = (!cwd.trim().is_empty()).then(|| PathBuf::from(cwd));
    project_paths_from_args(
        args.into_iter().map(OsString::from),
        cwd_path.as_deref(),
    )
    .into_iter()
    .map(|path| path.to_string_lossy().to_string())
    .collect()
}

fn project_paths_from_args(
    args: impl IntoIterator<Item = OsString>,
    cwd: Option<&Path>,
) -> Vec<PathBuf> {
    args.into_iter()
        .skip(1)
        .map(PathBuf::from)
        .map(|path| resolve_project_arg_path(path, cwd))
        .filter(|path| is_rayard_project_path(path))
        .collect()
}

fn resolve_project_arg_path(path: PathBuf, cwd: Option<&Path>) -> PathBuf {
    if path.is_absolute() {
        path
    } else if let Some(cwd) = cwd {
        cwd.join(path)
    } else {
        path
    }
}

fn is_rayard_project_path(path: &Path) -> bool {
    has_extension(path, "ry")
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn validate_project_file(project: &ProjectFile) -> Result<(), String> {
    if project.version != 1 {
        return Err(format!("Unsupported project version {}", project.version));
    }
    validate_app_name("project", &project.app)?;
    validate_project_custom_profiles(&project.custom_profiles)?;
    validate_unique_ids(
        "fixture",
        project.snapshot.fixtures.iter().map(|fixture| fixture.id),
    )?;
    validate_unique_ids("cue", project.snapshot.cues.iter().map(|cue| cue.id))?;
    validate_unique_ids(
        "timeline event",
        project
            .snapshot
            .timeline
            .events
            .iter()
            .map(|event| event.id),
    )?;
    validate_unique_ids(
        "timeline automation",
        project
            .snapshot
            .timeline
            .automations
            .iter()
            .map(|automation| automation.id),
    )?;
    validate_unique_ids(
        "timeline video automation",
        project
            .snapshot
            .timeline
            .video_automations
            .iter()
            .map(|automation| automation.id),
    )?;
    validate_unique_ids(
        "video layer",
        project.snapshot.video.layers.iter().map(|layer| layer.id),
    )?;
    validate_unique_ids(
        "video composition",
        project
            .snapshot
            .video
            .compositions
            .iter()
            .map(|composition| composition.id),
    )?;
    validate_unique_ids(
        "video output",
        project
            .snapshot
            .video
            .outputs
            .iter()
            .map(|output| output.id),
    )?;
    validate_unique_ids(
        "effect",
        project.snapshot.effects.iter().map(|effect| effect.id),
    )?;
    validate_unique_ids(
        "node graph",
        project.snapshot.node_graphs.iter().map(|graph| graph.id),
    )?;
    validate_project_fixture_patches(&project.snapshot.fixtures)?;
    validate_project_fixture_geometries(&project.snapshot.fixtures)?;
    validate_project_fixture_attribute_values(&project.snapshot.fixtures)?;
    validate_project_dmx_outputs(&project.snapshot)?;
    validate_stage_map_config(&project.snapshot.stage_map)?;
    validate_project_stage_map_presets(&project.snapshot.stage_map_presets)?;
    validate_project_stage_objects(&project.snapshot.stage_objects)?;
    validate_project_custom_profile_refs(&project.snapshot, &project.custom_profiles)?;
    validate_project_lighting_references(&project.snapshot)?;
    validate_project_video_graph(&project.snapshot)?;
    validate_project_node_graphs(&project.snapshot)?;
    Ok(())
}

fn validate_unique_ids(label: &str, ids: impl IntoIterator<Item = u64>) -> Result<(), String> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(format!("Project contains duplicate {label} id {id}"));
        }
    }
    Ok(())
}

fn validate_project_fixture_patches(fixtures: &[PatchedFixtureSummary]) -> Result<(), String> {
    let mut occupied = Vec::new();
    for fixture in fixtures {
        if fixture.label.trim().is_empty() {
            return Err(format!(
                "Project fixture {} has an empty fixture label",
                fixture.id
            ));
        }
        validate_fixture_transform(&fixture.position, &fixture.rotation).map_err(|error| {
            format!(
                "Project fixture {} '{}' has invalid transform: {error}",
                fixture.id, fixture.label
            )
        })?;
        validate_group_ids(&fixture.group_ids).map_err(|error| {
            format!(
                "Project fixture {} '{}' has invalid group id: {error}",
                fixture.id, fixture.label
            )
        })?;
        if fixture.address == 0 || fixture.address > 512 {
            return Err(format!(
                "Project fixture {} '{}' has invalid DMX address {}",
                fixture.id, fixture.label, fixture.address
            ));
        }
        let footprint = fixture_controls_footprint(&fixture.controls).ok_or_else(|| {
            format!(
                "Project fixture {} '{}' has no DMX channel offsets",
                fixture.id, fixture.label
            )
        })?;
        let range = dmx_range(fixture.address, footprint).ok_or_else(|| {
            format!(
                "Project fixture {} '{}' has no DMX channel offsets",
                fixture.id, fixture.label
            )
        })?;
        if range.1 > 512 {
            return Err(format!(
                "Project fixture {} '{}' exceeds DMX universe {}: start {}, footprint {}ch, end {}",
                fixture.id, fixture.label, fixture.universe, fixture.address, footprint, range.1
            ));
        }
        for (occupied_universe, occupied_range, occupied_label) in &occupied {
            if *occupied_universe == fixture.universe && ranges_overlap(range, *occupied_range) {
                return Err(format!(
                    "Project fixture {} '{}' conflicts in universe {}: {}-{} overlaps {} at {}-{}",
                    fixture.id,
                    fixture.label,
                    fixture.universe,
                    range.0,
                    range.1,
                    occupied_label,
                    occupied_range.0,
                    occupied_range.1
                ));
            }
        }
        occupied.push((
            fixture.universe,
            range,
            format!("fixture {} '{}'", fixture.id, fixture.label),
        ));
    }
    Ok(())
}

fn validate_project_fixture_geometries(fixtures: &[PatchedFixtureSummary]) -> Result<(), String> {
    for fixture in fixtures {
        validate_geometry_collection(
            &format!("fixture {} '{}'", fixture.id, fixture.label),
            &fixture.geometries,
            &fixture.controls,
        )?;
    }
    Ok(())
}

fn validate_geometry_collection(
    owner_label: &str,
    geometries: &[GeometrySummary],
    controls: &[AttributeControl],
) -> Result<(), String> {
    let mut geometry_names = HashSet::new();
    let mut parents_by_name = HashMap::new();

    for geometry in geometries {
        let name = geometry.name.trim();
        if name.is_empty() {
            return Err(format!(
                "Project {owner_label} has a geometry node with an empty name"
            ));
        }
        if !geometry_names.insert(name.to_string()) {
            return Err(format!(
                "Project {owner_label} contains duplicate geometry node '{name}'"
            ));
        }
        if geometry.kind.trim().is_empty() {
            return Err(format!(
                "Project {owner_label} geometry '{name}' has an empty kind"
            ));
        }
        if geometry.matrix.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "Project {owner_label} geometry '{name}' has a non-finite transform matrix"
            ));
        }
        let parent = geometry
            .parent
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(parent) = parent {
            if parent == name {
                return Err(format!(
                    "Project {owner_label} geometry '{name}' cannot parent itself"
                ));
            }
            parents_by_name.insert(name.to_string(), parent.to_string());
        }
        if let Some(dimensions) = geometry.model_dimensions {
            if !dimensions.x.is_finite()
                || !dimensions.y.is_finite()
                || !dimensions.z.is_finite()
                || dimensions.x < 0.0
                || dimensions.y < 0.0
                || dimensions.z < 0.0
            {
                return Err(format!(
                    "Project {owner_label} geometry '{name}' has invalid model dimensions"
                ));
            }
        }
        for (label, value) in [
            ("beam angle", geometry.beam_angle_deg),
            ("field angle", geometry.field_angle_deg),
            ("beam radius", geometry.beam_radius),
        ] {
            if let Some(value) = value {
                if !value.is_finite() || value < 0.0 {
                    return Err(format!(
                        "Project {owner_label} geometry '{name}' has invalid {label}"
                    ));
                }
            }
        }
    }

    for (name, parent) in &parents_by_name {
        if !geometry_names.contains(parent) {
            return Err(format!(
                "Project {owner_label} geometry '{name}' references missing parent '{parent}'"
            ));
        }
    }

    for geometry in geometries {
        let mut seen = HashSet::new();
        let mut current = geometry.name.trim().to_string();
        while let Some(parent) = parents_by_name.get(&current) {
            if !seen.insert(current.clone()) {
                return Err(format!(
                    "Project {owner_label} geometry '{}' contains a parent cycle",
                    geometry.name
                ));
            }
            current = parent.clone();
        }
    }

    for control in controls {
        let Some(geometry_name) = control
            .geometry
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if !geometry_names.contains(geometry_name) {
            return Err(format!(
                "Project {owner_label} control '{}' references missing geometry '{geometry_name}'",
                control.attribute
            ));
        }
    }

    Ok(())
}

fn validate_project_fixture_attribute_values(
    fixtures: &[PatchedFixtureSummary],
) -> Result<(), String> {
    for fixture in fixtures {
        let valid_attributes = fixture
            .controls
            .iter()
            .map(|control| control.attribute.as_str())
            .collect::<HashSet<_>>();
        let mut seen_attributes = HashSet::new();
        for value in &fixture.attribute_values {
            if !seen_attributes.insert(value.attribute.as_str()) {
                return Err(format!(
                    "Project fixture {} '{}' contains duplicate attribute value '{}'",
                    fixture.id, fixture.label, value.attribute
                ));
            }
            if !valid_attributes.contains(value.attribute.as_str()) {
                return Err(format!(
                    "Project fixture {} '{}' has saved value for unavailable attribute '{}'",
                    fixture.id, fixture.label, value.attribute
                ));
            }
        }
    }
    Ok(())
}

fn validate_project_dmx_outputs(snapshot: &EngineSnapshot) -> Result<(), String> {
    if snapshot.dmx_outputs.is_empty() {
        validate_dmx_output_routes(std::slice::from_ref(&snapshot.output))
    } else {
        validate_dmx_output_routes(&snapshot.dmx_outputs)
    }
}

fn validate_project_lighting_references(snapshot: &EngineSnapshot) -> Result<(), String> {
    let fixtures_by_id = snapshot
        .fixtures
        .iter()
        .map(|fixture| (fixture.id, fixture))
        .collect::<HashMap<_, _>>();

    for cue in &snapshot.cues {
        if cue.label.trim().is_empty() {
            return Err(format!("Project cue {} has an empty label", cue.id));
        }
        validate_project_unique_refs(
            &format!("cue {} fixture target", cue.id),
            &cue.targets
                .iter()
                .map(|target| target.fixture_id)
                .collect::<Vec<_>>(),
        )?;
        for target in &cue.targets {
            validate_project_fixture_attribute_values_for_target(
                &fixtures_by_id,
                target.fixture_id,
                &target.values,
                &format!("cue {}", cue.id),
            )?;
        }
    }

    for event in &snapshot.timeline.events {
        if !snapshot.cues.iter().any(|cue| cue.id == event.cue_id) {
            return Err(format!(
                "Project timeline event {} references missing cue {}",
                event.id, event.cue_id
            ));
        }
    }

    for automation in &snapshot.timeline.automations {
        validate_project_fixture_attribute_ref(
            &fixtures_by_id,
            automation.fixture_id,
            &automation.attribute,
            &format!("timeline automation {}", automation.id),
        )?;
        if automation.keyframes.is_empty() {
            return Err(format!(
                "Project timeline automation {} requires at least one keyframe",
                automation.id
            ));
        }
        validate_project_lighting_keyframes(
            &automation.keyframes,
            &format!("timeline automation {}", automation.id),
        )?;
    }

    for effect in &snapshot.effects {
        validate_project_effect_body(effect)?;
        let has_light_targets =
            !effect.fixture_ids.is_empty() || !effect.target_group_ids.is_empty();
        let has_video_targets = !effect.video_targets.is_empty();
        if !has_light_targets && !has_video_targets {
            return Err(format!(
                "Project effect {} requires a light or video target",
                effect.id
            ));
        }
        if has_light_targets && effect.attribute.trim().is_empty() {
            return Err(format!(
                "Project effect {} has light targets but no attribute",
                effect.id
            ));
        }
        validate_group_ids(&effect.target_group_ids)?;
        validate_project_effect_fixture_targets(
            snapshot,
            &fixtures_by_id,
            &effect.fixture_ids,
            &effect.target_group_ids,
            &effect.attribute,
            &format!("effect {}", effect.id),
        )?;
    }

    Ok(())
}

fn validate_project_fixture_attribute_values_for_target(
    fixtures_by_id: &HashMap<FixtureId, &PatchedFixtureSummary>,
    fixture_id: FixtureId,
    values: &[protocol::AttributeValueSummary],
    owner_label: &str,
) -> Result<(), String> {
    let fixture = fixtures_by_id
        .get(&fixture_id)
        .ok_or_else(|| format!("Project {owner_label} references missing fixture {fixture_id}"))?;
    let mut seen_attributes = HashSet::new();
    for value in values {
        if !seen_attributes.insert(value.attribute.as_str()) {
            return Err(format!(
                "Project {owner_label} fixture target {} contains duplicate attribute '{}'",
                fixture_id, value.attribute
            ));
        }
        validate_project_fixture_attribute_ref(
            fixtures_by_id,
            fixture_id,
            &value.attribute,
            owner_label,
        )?;
    }
    if values.is_empty() {
        return Err(format!(
            "Project {owner_label} fixture target {} has no attribute values",
            fixture.id
        ));
    }
    Ok(())
}

fn validate_project_fixture_attribute_ref(
    fixtures_by_id: &HashMap<FixtureId, &PatchedFixtureSummary>,
    fixture_id: FixtureId,
    attribute: &str,
    owner_label: &str,
) -> Result<(), String> {
    if attribute.trim().is_empty() {
        return Err(format!("Project {owner_label} has an empty attribute"));
    }
    let fixture = fixtures_by_id
        .get(&fixture_id)
        .ok_or_else(|| format!("Project {owner_label} references missing fixture {fixture_id}"))?;
    if !fixture
        .controls
        .iter()
        .any(|control| control.attribute == attribute)
    {
        return Err(format!(
            "Project {owner_label} references unavailable attribute '{}' on fixture {} '{}'",
            attribute, fixture.id, fixture.label
        ));
    }
    Ok(())
}

fn validate_project_effect_fixture_targets(
    snapshot: &EngineSnapshot,
    fixtures_by_id: &HashMap<FixtureId, &PatchedFixtureSummary>,
    fixture_ids: &[FixtureId],
    target_group_ids: &[String],
    attribute: &str,
    owner_label: &str,
) -> Result<(), String> {
    validate_project_unique_refs(owner_label, fixture_ids)?;
    for fixture_id in fixture_ids {
        validate_project_fixture_attribute_ref(
            fixtures_by_id,
            *fixture_id,
            attribute,
            owner_label,
        )?;
    }
    for group_id in target_group_ids {
        let group_fixtures = snapshot
            .fixtures
            .iter()
            .filter(|fixture| {
                fixture
                    .group_ids
                    .iter()
                    .any(|candidate| group_matches(candidate, group_id))
            })
            .collect::<Vec<_>>();
        if group_fixtures.is_empty() {
            return Err(format!(
                "Project {owner_label} references missing fixture group '{group_id}'"
            ));
        }
        if !group_fixtures.iter().any(|fixture| {
            fixture
                .controls
                .iter()
                .any(|control| control.attribute == attribute)
        }) {
            return Err(format!(
                "Project {owner_label} group '{group_id}' has no fixtures exposing attribute {attribute}"
            ));
        }
    }
    Ok(())
}

fn validate_project_custom_profiles(profiles: &[FixtureProfileSummary]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for profile in profiles {
        let source_path = profile.source_path.trim();
        if source_path.is_empty() {
            return Err("Project custom profile has an empty source path".to_string());
        }
        if !seen.insert(source_path.to_string()) {
            return Err(format!(
                "Project contains duplicate custom profile source path {source_path}"
            ));
        }
        if profile.manufacturer.trim().is_empty() {
            return Err(format!(
                "Project custom profile {source_path} has an empty manufacturer"
            ));
        }
        if profile.name.trim().is_empty() {
            return Err(format!(
                "Project custom profile {source_path} has an empty name"
            ));
        }
        if profile.dmx_modes.is_empty() {
            return Err(format!(
                "Project custom profile {source_path} has no DMX modes"
            ));
        }
        for mode in &profile.dmx_modes {
            if mode.name.trim().is_empty() {
                return Err(format!(
                    "Project custom profile {source_path} has an empty DMX mode name"
                ));
            }
            fixture_controls_footprint(&mode.controls).ok_or_else(|| {
                format!(
                    "Project custom profile {source_path} mode '{}' has no DMX channel offsets",
                    mode.name
                )
            })?;
            validate_geometry_collection(
                &format!("custom profile {source_path} mode '{}'", mode.name),
                &profile.geometries,
                &mode.controls,
            )?;
        }
    }
    Ok(())
}

fn validate_project_custom_profile_refs(
    snapshot: &EngineSnapshot,
    profiles: &[FixtureProfileSummary],
) -> Result<(), String> {
    let custom_profile_paths = profiles
        .iter()
        .map(|profile| profile.source_path.as_str())
        .collect::<HashSet<_>>();
    for fixture in &snapshot.fixtures {
        let source_path = fixture.profile_source_path.trim();
        if source_path.starts_with("memory://custom/")
            && !custom_profile_paths.contains(source_path)
        {
            return Err(format!(
                "Project fixture {} '{}' references missing custom profile {}",
                fixture.id, fixture.label, source_path
            ));
        }
    }
    Ok(())
}

fn validate_project_video_graph(snapshot: &EngineSnapshot) -> Result<(), String> {
    let layer_ids = snapshot
        .video
        .layers
        .iter()
        .map(|layer| layer.id)
        .collect::<HashSet<_>>();
    let composition_ids = snapshot
        .video
        .compositions
        .iter()
        .map(|composition| composition.id)
        .collect::<HashSet<_>>();
    let outputs_by_id = snapshot
        .video
        .outputs
        .iter()
        .map(|output| (output.id, output))
        .collect::<HashMap<_, _>>();
    let node_graph_ids = snapshot
        .node_graphs
        .iter()
        .map(|graph| graph.id)
        .collect::<HashSet<_>>();

    for layer in &snapshot.video.layers {
        if layer.label.trim().is_empty() {
            return Err(format!(
                "Project video layer {} has an empty label",
                layer.id
            ));
        }
        validate_project_video_source(
            &layer.source,
            &format!("video layer {} '{}'", layer.id, layer.label),
        )?;
        validate_project_video_layer_state(&layer.state, &format!("video layer {}", layer.id))?;
    }

    for composition in &snapshot.video.compositions {
        if composition.label.trim().is_empty() {
            return Err(format!(
                "Project video composition {} has an empty label",
                composition.id
            ));
        }
        validate_project_unique_refs(
            &format!("video composition {} layer", composition.id),
            &composition.layer_ids,
        )?;
        validate_project_unique_refs(
            &format!("video composition {} output", composition.id),
            &composition.output_ids,
        )?;
        for layer_id in &composition.layer_ids {
            if !layer_ids.contains(layer_id) {
                return Err(format!(
                    "Project video composition {} '{}' references missing video layer {}",
                    composition.id, composition.label, layer_id
                ));
            }
        }
        for output_id in &composition.output_ids {
            let Some(output) = outputs_by_id.get(output_id) else {
                return Err(format!(
                    "Project video composition {} '{}' references missing video output {}",
                    composition.id, composition.label, output_id
                ));
            };
            if output.composition_id != composition.id {
                return Err(format!(
                    "Project video composition {} '{}' lists output {} routed to composition {}",
                    composition.id, composition.label, output.id, output.composition_id
                ));
            }
        }
    }

    for output in &snapshot.video.outputs {
        if output.label.trim().is_empty() {
            return Err(format!(
                "Project video output {} has an empty label",
                output.id
            ));
        }
        if output.width == 0 || output.height == 0 {
            return Err(format!(
                "Project video output {} '{}' has invalid resolution {}x{}",
                output.id, output.label, output.width, output.height
            ));
        }
        if !composition_ids.contains(&output.composition_id) {
            return Err(format!(
                "Project video output {} '{}' routes to missing composition {}",
                output.id, output.label, output.composition_id
            ));
        }
        if !output.opacity.is_finite() {
            return Err(format!(
                "Project video output {} '{}' has non-finite opacity",
                output.id, output.label
            ));
        }
        validate_video_output_mapping(&output.mapping)?;
    }

    validate_project_external_video_backends(snapshot)?;

    for cue in &snapshot.cues {
        for target in &cue.video_targets {
            validate_project_video_layer_ref(&layer_ids, target.layer_id, "cue", cue.id)?;
            validate_project_video_layer_state(
                &target.state,
                &format!("cue {} video layer target {}", cue.id, target.layer_id),
            )?;
        }
        for target in &cue.video_output_targets {
            if !outputs_by_id.contains_key(&target.output_id) {
                return Err(format!(
                    "Project cue {} references missing video output {}",
                    cue.id, target.output_id
                ));
            }
            if !target.opacity.is_finite() {
                return Err(format!(
                    "Project cue {} video output target {} has non-finite opacity",
                    cue.id, target.output_id
                ));
            }
        }
        let cue_node_graph_ids = cue
            .node_graph_targets
            .iter()
            .map(|target| target.graph_id)
            .collect::<Vec<_>>();
        validate_project_unique_refs(&format!("cue {} node graph", cue.id), &cue_node_graph_ids)?;
        for target in &cue.node_graph_targets {
            if !node_graph_ids.contains(&target.graph_id) {
                return Err(format!(
                    "Project cue {} references missing node graph {}",
                    cue.id, target.graph_id
                ));
            }
        }
    }

    for automation in &snapshot.timeline.video_automations {
        validate_project_video_layer_ref(
            &layer_ids,
            automation.layer_id,
            "timeline video automation",
            automation.id,
        )?;
        if automation.keyframes.is_empty() {
            return Err(format!(
                "Project timeline video automation {} requires at least one keyframe",
                automation.id
            ));
        }
        if automation
            .keyframes
            .iter()
            .any(|keyframe| !keyframe.value.is_finite())
        {
            return Err(format!(
                "Project timeline video automation {} has non-finite keyframe values",
                automation.id
            ));
        }
        validate_project_video_keyframes(
            &automation.keyframes,
            &format!("timeline video automation {}", automation.id),
        )?;
    }

    for effect in &snapshot.effects {
        validate_video_effect_targets(&effect.video_targets)?;
        for target in &effect.video_targets {
            for layer_id in &target.layer_ids {
                validate_project_video_layer_ref(&layer_ids, *layer_id, "effect", effect.id)?;
            }
        }
    }

    let mut mapping_preset_labels = HashSet::new();
    for preset in &snapshot.video.mapping_presets {
        let label = preset.label.trim();
        if label.is_empty() {
            return Err("Project video output mapping preset has an empty label".to_string());
        }
        if !mapping_preset_labels.insert(label.to_string()) {
            return Err(format!(
                "Project contains duplicate video output mapping preset label {label}"
            ));
        }
        validate_video_output_mapping(&preset.mapping)?;
    }

    Ok(())
}

fn validate_project_video_source(source: &VideoSourceSummary, label: &str) -> Result<(), String> {
    if source
        .codec
        .as_deref()
        .is_some_and(|codec| codec.trim().is_empty())
    {
        return Err(format!("Project {label} source has an empty codec"));
    }

    if let Some(metadata) = source.metadata {
        validate_project_video_source_metadata(&metadata, label)?;
    }

    match source.kind {
        VideoSourceKind::File => {
            if source.path.as_deref().unwrap_or_default().trim().is_empty() {
                return Err(format!("Project {label} file source requires a file path"));
            }
        }
        VideoSourceKind::StillImage => {
            if source.path.as_deref().unwrap_or_default().trim().is_empty() {
                return Err(format!(
                    "Project {label} still image source requires a still image path"
                ));
            }
        }
        VideoSourceKind::Ndi | VideoSourceKind::Spout | VideoSourceKind::Syphon => {
            let (_, feature_label) =
                video_input_backend(&source.kind).expect("external source kind must have backend");
            if source.name.as_deref().unwrap_or_default().trim().is_empty() {
                return Err(format!(
                    "Project {label} requires a source name for {feature_label}"
                ));
            }
        }
    }

    Ok(())
}

fn validate_project_video_source_metadata(
    metadata: &protocol::VideoMediaMetadata,
    label: &str,
) -> Result<(), String> {
    if metadata.width.is_some_and(|width| width == 0) {
        return Err(format!("Project {label} source has invalid metadata width"));
    }
    if metadata.height.is_some_and(|height| height == 0) {
        return Err(format!(
            "Project {label} source has invalid metadata height"
        ));
    }
    if metadata
        .frame_rate
        .is_some_and(|frame_rate| !frame_rate.is_finite() || frame_rate <= 0.0)
    {
        return Err(format!(
            "Project {label} source has invalid metadata frame rate"
        ));
    }
    Ok(())
}

fn validate_project_external_video_backends(snapshot: &EngineSnapshot) -> Result<(), String> {
    for layer in &snapshot.video.layers {
        let Some((_, feature_label)) = video_input_backend(&layer.source.kind) else {
            continue;
        };
        if layer
            .source
            .name
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            return Err(format!(
                "Project video layer {} '{}' requires a source name for {feature_label}",
                layer.id, layer.label
            ));
        }
    }

    for output in &snapshot.video.outputs {
        let Some((_, feature_label)) = video_output_backend(&output.kind) else {
            continue;
        };
        if output
            .endpoint_name
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            return Err(format!(
                "Project video output {} '{}' requires an endpoint name for {feature_label}",
                output.id, output.label
            ));
        }
    }

    Ok(())
}

fn validate_project_node_graphs(snapshot: &EngineSnapshot) -> Result<(), String> {
    for graph in &snapshot.node_graphs {
        validate_node_graph_summary(graph, snapshot)?;
    }
    Ok(())
}

fn validate_project_unique_refs(label: &str, ids: &[u64]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(*id) {
            return Err(format!("Project contains duplicate {label} reference {id}"));
        }
    }
    Ok(())
}

fn validate_project_lighting_keyframes(
    keyframes: &[AutomationKeyframeSummary],
    owner_label: &str,
) -> Result<(), String> {
    let mut seen_times = HashSet::new();
    for keyframe in keyframes {
        if !seen_times.insert(keyframe.time_ms) {
            return Err(format!(
                "Project {owner_label} contains duplicate keyframe time {}ms",
                keyframe.time_ms
            ));
        }
    }
    Ok(())
}

fn validate_project_video_keyframes(
    keyframes: &[VideoAutomationKeyframeSummary],
    owner_label: &str,
) -> Result<(), String> {
    let mut seen_times = HashSet::new();
    for keyframe in keyframes {
        if !seen_times.insert(keyframe.time_ms) {
            return Err(format!(
                "Project {owner_label} contains duplicate keyframe time {}ms",
                keyframe.time_ms
            ));
        }
    }
    Ok(())
}

fn validate_project_effect_body(effect: &EffectSummary) -> Result<(), String> {
    let preset = effect_summary_to_preset(effect)
        .map_err(|error| format!("Project effect {} is invalid: {error}", effect.id))?;
    validate_effect_preset(&preset)
        .map_err(|error| format!("Project effect {} is invalid: {error}", effect.id))
}

fn validate_project_video_layer_ref(
    layer_ids: &HashSet<VideoLayerId>,
    layer_id: VideoLayerId,
    owner_label: &str,
    owner_id: u64,
) -> Result<(), String> {
    if layer_ids.contains(&layer_id) {
        Ok(())
    } else {
        Err(format!(
            "Project {owner_label} {owner_id} references missing video layer {layer_id}"
        ))
    }
}

fn validate_project_video_layer_state(state: &VideoLayerState, label: &str) -> Result<(), String> {
    let transform = state.transform;
    let color = state.color;
    let fx = state.fx;
    let values = [
        state.opacity,
        state.speed,
        state.bpm_sync.ratio,
        state.bpm_sync.loop_bars,
        transform.x,
        transform.y,
        transform.scale_x,
        transform.scale_y,
        transform.rotation_deg,
        transform.crop_left,
        transform.crop_top,
        transform.crop_right,
        transform.crop_bottom,
        color.brightness,
        color.contrast,
        color.hue_deg,
        color.saturation,
        color.gamma,
        fx.pixelate,
        fx.blur,
        fx.glow,
        fx.edge,
        fx.key_red,
        fx.key_green,
        fx.key_blue,
        fx.key_threshold,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(format!("Project {label} has non-finite video state values"));
    }
    if transform.scale_x <= 0.0 || transform.scale_y <= 0.0 {
        return Err(format!(
            "Project {label} transform scale must be greater than 0"
        ));
    }
    if state.bpm_sync.ratio <= 0.0 || state.bpm_sync.loop_bars <= 0.0 {
        return Err(format!(
            "Project {label} BPM sync ratio and loop bars must be greater than 0"
        ));
    }
    if state.loop_enabled && state.loop_end_ms <= state.loop_start_ms {
        return Err(format!(
            "Project {label} loop end must be greater than loop start"
        ));
    }
    validate_project_video_cue_points(state, label)?;
    Ok(())
}

fn validate_project_video_cue_points(state: &VideoLayerState, label: &str) -> Result<(), String> {
    let mut seen_positions = HashSet::new();
    for cue_point in &state.cue_points {
        if !seen_positions.insert(cue_point.position_ms) {
            return Err(format!(
                "Project {label} contains duplicate video cue point at {}ms",
                cue_point.position_ms
            ));
        }
        if let Some(color) = &cue_point.color {
            validate_video_cue_point_color(color)
                .map_err(|error| format!("Project {label} cue point color is invalid: {error}"))?;
        }
    }

    let mut seen_legacy_positions = HashSet::new();
    for position_ms in &state.cue_points_ms {
        if !seen_legacy_positions.insert(*position_ms) {
            return Err(format!(
                "Project {label} contains duplicate legacy video cue point at {position_ms}ms"
            ));
        }
    }
    Ok(())
}

fn effect_summary_to_preset(effect: &EffectSummary) -> Result<EffectPreset, String> {
    match effect.effect_type {
        EffectKind::Lfo => {
            let period_ms = effect
                .period_ms
                .ok_or_else(|| "LFO effect summary is missing period_ms".to_string())?;
            Ok(EffectPreset {
                version: 1,
                effect_type: EffectKind::Lfo,
                enabled: effect.enabled,
                lfo: Some(LfoEffectRequest {
                    label: effect.label.clone(),
                    fixture_ids: effect.fixture_ids.clone(),
                    target_group_ids: effect.target_group_ids.clone(),
                    attribute: effect.attribute.clone(),
                    video_targets: effect.video_targets.clone(),
                    shape: effect.shape.clone(),
                    period_ms,
                    clock_sync: effect.clock_sync,
                    low: effect.low,
                    high: effect.high,
                    phase: effect.phase,
                    blend_mode: effect.blend_mode.clone(),
                }),
                position_wave: None,
            })
        }
        EffectKind::PositionWave => {
            let origin = effect
                .origin
                .ok_or_else(|| "Position wave effect summary is missing origin".to_string())?;
            let direction = effect
                .direction
                .ok_or_else(|| "Position wave effect summary is missing direction".to_string())?;
            let speed = effect
                .speed
                .ok_or_else(|| "Position wave effect summary is missing speed".to_string())?;
            let wavelength = effect
                .wavelength
                .ok_or_else(|| "Position wave effect summary is missing wavelength".to_string())?;
            Ok(EffectPreset {
                version: 1,
                effect_type: EffectKind::PositionWave,
                enabled: effect.enabled,
                lfo: None,
                position_wave: Some(PositionWaveEffectRequest {
                    label: effect.label.clone(),
                    fixture_ids: effect.fixture_ids.clone(),
                    target_group_ids: effect.target_group_ids.clone(),
                    attribute: effect.attribute.clone(),
                    video_targets: effect.video_targets.clone(),
                    shape: effect.shape.clone(),
                    origin,
                    direction,
                    speed,
                    wavelength,
                    clock_sync: effect.clock_sync,
                    low: effect.low,
                    high: effect.high,
                    phase: effect.phase,
                    blend_mode: effect.blend_mode.clone(),
                }),
            })
        }
    }
}

fn apply_lfo_effect_target_override(
    mut request: LfoEffectRequest,
    target_override: &EffectTargetOverride,
) -> LfoEffectRequest {
    request.fixture_ids = target_override.fixture_ids.clone();
    request.target_group_ids = target_override.target_group_ids.clone();
    request.video_targets = target_override.video_targets.clone();
    request.attribute = overridden_light_attribute(target_override);
    request
}

fn apply_position_wave_effect_target_override(
    mut request: PositionWaveEffectRequest,
    target_override: &EffectTargetOverride,
) -> PositionWaveEffectRequest {
    request.fixture_ids = target_override.fixture_ids.clone();
    request.target_group_ids = target_override.target_group_ids.clone();
    request.video_targets = target_override.video_targets.clone();
    request.attribute = overridden_light_attribute(target_override);
    request
}

fn overridden_light_attribute(target_override: &EffectTargetOverride) -> String {
    if target_override.fixture_ids.is_empty() && target_override.target_group_ids.is_empty() {
        String::new()
    } else {
        target_override.attribute.trim().to_string()
    }
}

fn validate_effect_target_override(target_override: &EffectTargetOverride) -> Result<(), String> {
    let has_light_targets =
        !target_override.fixture_ids.is_empty() || !target_override.target_group_ids.is_empty();
    let has_video_targets = !target_override.video_targets.is_empty();
    if !has_light_targets && !has_video_targets {
        return Err("At least one fixture, group, or video layer must be targeted".to_string());
    }
    if has_light_targets && target_override.attribute.trim().is_empty() {
        return Err("Effect attribute is required".to_string());
    }
    validate_group_ids(&target_override.target_group_ids)?;
    validate_video_effect_targets(&target_override.video_targets)?;
    Ok(())
}

fn validate_node_graph_summary(
    graph: &NodeGraphSummary,
    snapshot: &EngineSnapshot,
) -> Result<(), String> {
    if graph.id == 0 {
        return Err("Node graph id must be greater than 0".to_string());
    }
    if graph.label.trim().is_empty() {
        return Err("Node graph label is required".to_string());
    }
    if graph.nodes.is_empty() {
        return Err("Node graph requires at least one node".to_string());
    }

    let mut node_ids = HashSet::new();
    for node in &graph.nodes {
        if node.id == 0 {
            return Err(format!("Node graph '{}' contains node id 0", graph.label));
        }
        if !node_ids.insert(node.id) {
            return Err(format!(
                "Node graph '{}' contains duplicate node id {}",
                graph.label, node.id
            ));
        }
        if node.label.trim().is_empty() {
            return Err(format!(
                "Node graph '{}' node {} has an empty label",
                graph.label, node.id
            ));
        }
        if !node.x.is_finite() || !node.y.is_finite() {
            return Err(format!(
                "Node graph '{}' node {} has non-finite coordinates",
                graph.label, node.id
            ));
        }

        match node.kind {
            NodeGraphNodeKind::Lfo => {
                if node.position_wave.is_some() || node.transform.is_some() || node.output.is_some()
                {
                    return Err(format!(
                        "Node graph '{}' LFO node {} contains a non-LFO body",
                        graph.label, node.id
                    ));
                }
                let Some(lfo) = &node.lfo else {
                    return Err(format!(
                        "Node graph '{}' LFO node {} is missing its body",
                        graph.label, node.id
                    ));
                };
                if lfo.period_ms < 10 {
                    return Err(format!(
                        "Node graph '{}' LFO node {} period must be at least 10ms",
                        graph.label, node.id
                    ));
                }
                if !lfo.phase.is_finite() || !lfo.amplitude.is_finite() || !lfo.bias.is_finite() {
                    return Err(format!(
                        "Node graph '{}' LFO node {} has non-finite values",
                        graph.label, node.id
                    ));
                }
                if let Some(clock_sync) = lfo.clock_sync {
                    if !clock_sync.beats.is_finite() || clock_sync.beats <= 0.0 {
                        return Err(format!(
                            "Node graph '{}' LFO node {} clock sync beats must be greater than 0",
                            graph.label, node.id
                        ));
                    }
                }
            }
            NodeGraphNodeKind::PositionWave => {
                if node.lfo.is_some() || node.transform.is_some() || node.output.is_some() {
                    return Err(format!(
                        "Node graph '{}' position wave node {} contains a non-position-wave body",
                        graph.label, node.id
                    ));
                }
                let Some(wave) = &node.position_wave else {
                    return Err(format!(
                        "Node graph '{}' position wave node {} is missing its body",
                        graph.label, node.id
                    ));
                };
                if !wave.origin.x.is_finite()
                    || !wave.origin.y.is_finite()
                    || !wave.origin.z.is_finite()
                    || !wave.direction.x.is_finite()
                    || !wave.direction.y.is_finite()
                    || !wave.direction.z.is_finite()
                    || !wave.speed.is_finite()
                    || !wave.wavelength.is_finite()
                    || !wave.phase.is_finite()
                {
                    return Err(format!(
                        "Node graph '{}' position wave node {} has non-finite values",
                        graph.label, node.id
                    ));
                }
                if wave.wavelength.abs() < 0.001 {
                    return Err(format!(
                        "Node graph '{}' position wave node {} wavelength must be at least 0.001",
                        graph.label, node.id
                    ));
                }
                if let Some(clock_sync) = wave.clock_sync {
                    if !clock_sync.beats.is_finite() || clock_sync.beats <= 0.0 {
                        return Err(format!(
                            "Node graph '{}' position wave node {} clock sync beats must be greater than 0",
                            graph.label, node.id
                        ));
                    }
                }
            }
            NodeGraphNodeKind::Transform => {
                if node.lfo.is_some() || node.position_wave.is_some() || node.output.is_some() {
                    return Err(format!(
                        "Node graph '{}' transform node {} contains a non-transform body",
                        graph.label, node.id
                    ));
                }
                let Some(transform) = &node.transform else {
                    return Err(format!(
                        "Node graph '{}' transform node {} is missing its body",
                        graph.label, node.id
                    ));
                };
                if !transform.amount.is_finite()
                    || !transform.min.is_finite()
                    || !transform.max.is_finite()
                {
                    return Err(format!(
                        "Node graph '{}' transform node {} has non-finite values",
                        graph.label, node.id
                    ));
                }
                if matches!(transform.op, NodeGraphTransformOp::Clamp)
                    && transform.min > transform.max
                {
                    return Err(format!(
                        "Node graph '{}' clamp node {} min must be <= max",
                        graph.label, node.id
                    ));
                }
            }
            NodeGraphNodeKind::Output => {
                if node.lfo.is_some() || node.position_wave.is_some() || node.transform.is_some() {
                    return Err(format!(
                        "Node graph '{}' output node {} contains a non-output body",
                        graph.label, node.id
                    ));
                }
                let Some(output) = &node.output else {
                    return Err(format!(
                        "Node graph '{}' output node {} is missing its body",
                        graph.label, node.id
                    ));
                };
                let has_light_targets =
                    !output.fixture_ids.is_empty() || !output.target_group_ids.is_empty();
                if has_light_targets && output.attribute.trim().is_empty() {
                    return Err(format!(
                        "Node graph '{}' output node {} is missing its light attribute",
                        graph.label, node.id
                    ));
                }
                if !has_light_targets && output.video_targets.is_empty() {
                    return Err(format!(
                        "Node graph '{}' output node {} requires a light or video target",
                        graph.label, node.id
                    ));
                }
                validate_group_ids(&output.target_group_ids)?;
                validate_video_effect_targets(&output.video_targets)?;
                let fixtures_by_id = snapshot
                    .fixtures
                    .iter()
                    .map(|fixture| (fixture.id, fixture))
                    .collect::<HashMap<_, _>>();
                validate_project_effect_fixture_targets(
                    snapshot,
                    &fixtures_by_id,
                    &output.fixture_ids,
                    &output.target_group_ids,
                    &output.attribute,
                    &format!("node graph '{}' output node {}", graph.label, node.id),
                )?;
                validate_effect_target_references(
                    snapshot,
                    &output.fixture_ids,
                    &output.video_targets,
                )?;
            }
        }
    }

    for edge in &graph.edges {
        if !node_ids.contains(&edge.from_node) || !node_ids.contains(&edge.to_node) {
            return Err(format!(
                "Node graph '{}' contains an edge with a missing node reference",
                graph.label
            ));
        }
        if edge.from_port.trim().is_empty() || edge.to_port.trim().is_empty() {
            return Err(format!(
                "Node graph '{}' contains an edge with an empty port",
                graph.label
            ));
        }
    }

    Ok(())
}

fn validate_node_graph_preset_file(
    file: &NodeGraphPresetFile,
    snapshot: &EngineSnapshot,
) -> Result<(), String> {
    if file.version != 1 {
        return Err(format!(
            "Unsupported node graph preset version {}",
            file.version
        ));
    }
    validate_app_name("node graph preset", &file.app)?;
    validate_node_graph_summary(&file.graph, snapshot)
}

fn validate_effect_preset(preset: &EffectPreset) -> Result<(), String> {
    if preset.version != 1 {
        return Err(format!(
            "Unsupported effect preset version {}",
            preset.version
        ));
    }
    match preset.effect_type {
        EffectKind::Lfo => {
            if preset.position_wave.is_some() {
                return Err("LFO effect preset must not contain a position wave body".to_string());
            }
            let request = preset
                .lfo
                .as_ref()
                .ok_or_else(|| "LFO effect preset is missing its request body".to_string())?;
            validate_lfo_effect_request(request)?;
        }
        EffectKind::PositionWave => {
            if preset.lfo.is_some() {
                return Err("Position wave effect preset must not contain an LFO body".to_string());
            }
            let request = preset.position_wave.as_ref().ok_or_else(|| {
                "Position wave effect preset is missing its request body".to_string()
            })?;
            validate_position_wave_effect_request(request)?;
        }
    }
    Ok(())
}

fn validate_effect_target_references(
    snapshot: &EngineSnapshot,
    fixture_ids: &[FixtureId],
    video_targets: &[VideoEffectTarget],
) -> Result<(), String> {
    for fixture_id in fixture_ids {
        if !snapshot
            .fixtures
            .iter()
            .any(|fixture| fixture.id == *fixture_id)
        {
            return Err(format!("Fixture {fixture_id} was not found"));
        }
    }
    for target in video_targets {
        validate_video_layer_ids(snapshot, &target.layer_ids)?;
    }
    Ok(())
}

fn validate_fixture_preset<'a>(
    preset: &FixturePreset,
    manufacturer: &str,
    profile_name: &str,
    mode_name: &str,
    valid_attributes: impl IntoIterator<Item = &'a str>,
) -> Result<(), String> {
    if preset.version != 1 {
        return Err(format!("Unsupported preset version {}", preset.version));
    }
    if preset.manufacturer != manufacturer
        || preset.profile_name != profile_name
        || preset.mode_name != mode_name
    {
        return Err(format!(
            "Preset is for {} {} / {}, but selected fixture is {} {} / {}",
            preset.manufacturer,
            preset.profile_name,
            preset.mode_name,
            manufacturer,
            profile_name,
            mode_name
        ));
    }
    let valid_attributes = valid_attributes.into_iter().collect::<HashSet<_>>();
    let mut preset_attributes = HashSet::new();
    for value in &preset.values {
        if !preset_attributes.insert(value.attribute.as_str()) {
            return Err(format!(
                "Preset contains duplicate attribute '{}'",
                value.attribute
            ));
        }
    }
    if let Some(value) = preset
        .values
        .iter()
        .find(|value| !valid_attributes.contains(value.attribute.as_str()))
    {
        return Err(format!(
            "Preset attribute '{}' is not available on the selected fixture mode",
            value.attribute
        ));
    }
    Ok(())
}

fn compatible_fixture_preset_targets(
    preset: &FixturePreset,
    fixtures: Vec<&PatchedFixtureSummary>,
) -> (Vec<FixtureId>, usize) {
    let mut fixture_ids = Vec::new();
    let mut skipped_count = 0usize;
    for fixture in fixtures {
        let compatible = validate_fixture_preset(
            preset,
            &fixture.manufacturer,
            &fixture.profile_name,
            &fixture.mode_name,
            fixture
                .controls
                .iter()
                .map(|control| control.attribute.as_str()),
        )
        .is_ok();
        if compatible {
            fixture_ids.push(fixture.id);
        } else {
            skipped_count += 1;
        }
    }
    (fixture_ids, skipped_count)
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> EngineSnapshot {
    state.engine.snapshot()
}

#[tauri::command]
fn set_stage_map_config(state: State<'_, AppState>, config: StageMapConfig) -> Result<(), String> {
    validate_stage_map_config(&config)?;
    state
        .engine
        .send(EngineCommand::SetStageMapConfig(config))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_stage_map_preset(
    state: State<'_, AppState>,
    label: String,
    config: StageMapConfig,
) -> Result<String, String> {
    let label = normalize_stage_map_preset_label(label)?;
    validate_stage_map_config(&config)?;
    state
        .engine
        .send(EngineCommand::SaveStageMapPreset {
            label: label.clone(),
            config,
            stage_objects: Some(state.engine.snapshot().stage_objects),
        })
        .map_err(|error| error.to_string())?;
    Ok(label)
}

#[tauri::command]
fn apply_stage_map_preset(state: State<'_, AppState>, label: String) -> Result<(), String> {
    let label = normalize_stage_map_preset_label(label)?;
    state
        .engine
        .send(EngineCommand::ApplyStageMapPreset { label })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn remove_stage_map_preset(state: State<'_, AppState>, label: String) -> Result<(), String> {
    let label = normalize_stage_map_preset_label(label)?;
    state
        .engine
        .send(EngineCommand::RemoveStageMapPreset { label })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_stage_object(
    state: State<'_, AppState>,
    label: String,
    kind: StageObjectKind,
    x: f32,
    z: f32,
    width: f32,
    depth: f32,
    rotation_deg: f32,
    color: Option<String>,
) -> Result<StageObjectId, String> {
    let object_id = state.engine.allocate_stage_object_id();
    let object = normalize_stage_object(StageObjectSummary {
        id: object_id,
        label,
        kind,
        x,
        z,
        width,
        depth,
        rotation_deg,
        color,
    })?;
    state
        .engine
        .send(EngineCommand::UpsertStageObject(object))
        .map_err(|error| error.to_string())?;
    Ok(object_id)
}

#[tauri::command]
fn set_stage_object(state: State<'_, AppState>, object: StageObjectSummary) -> Result<(), String> {
    let object = normalize_stage_object(object)?;
    state
        .engine
        .send(EngineCommand::UpsertStageObject(object))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn remove_stage_object(state: State<'_, AppState>, object_id: StageObjectId) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::RemoveStageObject(object_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_stage_map_preset_file(
    state: State<'_, AppState>,
    label: String,
    config: StageMapConfig,
) -> Result<Option<String>, String> {
    let label = normalize_stage_map_preset_label(label)?;
    validate_stage_map_config(&config)?;
    let file = StageMapPresetFile {
        version: 1,
        app: APP_NAME.to_string(),
        preset: StageMapPresetSummary {
            label: label.clone(),
            config,
            stage_objects: Some(state.engine.snapshot().stage_objects),
        },
    };
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Stage Map", &["stagemap"])
        .set_file_name(format!("{}.stagemap", safe_file_stem(&label)))
        .save_file()
    else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(&file).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn load_stage_map_preset_file(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Rayard Stage Map", &["stagemap"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let file: StageMapPresetFile =
        serde_json::from_str(&json).map_err(|error| error.to_string())?;
    validate_stage_map_preset_file(&file)?;
    let label = file.preset.label.clone();
    state
        .engine
        .send(EngineCommand::SaveStageMapPreset {
            label: file.preset.label,
            config: file.preset.config,
            stage_objects: file.preset.stage_objects,
        })
        .map_err(|error| error.to_string())?;
    Ok(Some(label))
}

#[tauri::command]
fn get_visualizer_scene(
    state: State<'_, AppState>,
    config: Option<visualizer::VisualizerConfig>,
) -> visualizer::VisualizerScene {
    visualizer::build_visualizer_scene(&state.engine.snapshot(), config.unwrap_or_default())
}

#[tauri::command]
fn get_visualizer_model_render_plans(
    state: State<'_, AppState>,
    config: Option<visualizer::VisualizerConfig>,
) -> Vec<visualizer::FixtureModelRenderPlan> {
    let scene =
        visualizer::build_visualizer_scene(&state.engine.snapshot(), config.unwrap_or_default());
    visualizer::build_fixture_model_render_plans(&scene)
}

#[tauri::command]
fn get_visualizer_render_payload(
    state: State<'_, AppState>,
    config: Option<visualizer::VisualizerConfig>,
) -> visualizer::VisualizerRenderPayload {
    visualizer::build_visualizer_render_payload(
        &state.engine.snapshot(),
        config.unwrap_or_default(),
    )
}

#[tauri::command]
fn get_visualizer_external_model_assets(
    state: State<'_, AppState>,
    config: Option<visualizer::VisualizerConfig>,
) -> Vec<VisualizerExternalModelAsset> {
    let payload = visualizer::build_visualizer_render_payload(
        &state.engine.snapshot(),
        config.unwrap_or_default(),
    );
    visualizer_external_model_assets_from_payload(&payload, Some(&state.visualizer_model_assets))
}

#[tauri::command]
fn get_visualizer_resolved_render_payload(
    state: State<'_, AppState>,
    config: Option<visualizer::VisualizerConfig>,
) -> VisualizerResolvedRenderPayload {
    let payload = visualizer::build_visualizer_render_payload(
        &state.engine.snapshot(),
        config.unwrap_or_default(),
    );
    visualizer_resolved_render_payload(payload, Some(&state.visualizer_model_assets))
}

#[tauri::command]
fn get_visualizer_model_asset_cache_summary(
    state: State<'_, AppState>,
) -> Result<VisualizerModelAssetCacheSummary, String> {
    let cache = state
        .visualizer_model_assets
        .lock()
        .map_err(|_| "Visualizer model asset cache is unavailable".to_string())?;
    Ok(visualizer_model_asset_cache_summary(&cache))
}

fn visualizer_resolved_render_payload(
    payload: visualizer::VisualizerRenderPayload,
    cache: Option<&Mutex<HashMap<String, VisualizerModelAssetCacheEntry>>>,
) -> VisualizerResolvedRenderPayload {
    let external_model_assets = visualizer_external_model_assets_from_payload(&payload, cache);
    let mut resolved_meshes = payload.primitive_meshes.clone();
    resolved_meshes.extend(
        external_model_assets
            .iter()
            .flat_map(|asset| asset.meshes.iter().cloned()),
    );

    VisualizerResolvedRenderPayload {
        scene: payload.scene,
        model_render_plans: payload.model_render_plans,
        primitive_meshes: payload.primitive_meshes,
        external_model_assets,
        resolved_meshes,
    }
}

fn visualizer_external_model_assets_from_payload(
    payload: &visualizer::VisualizerRenderPayload,
    cache: Option<&Mutex<HashMap<String, VisualizerModelAssetCacheEntry>>>,
) -> Vec<VisualizerExternalModelAsset> {
    let mut grouped_assets = HashMap::<String, VisualizerExternalModelAssetGroup>::new();
    for plan in payload
        .model_render_plans
        .iter()
        .filter(|plan| plan.draw_kind == visualizer::FixtureModelDrawKind::ExternalMesh)
    {
        let profile_source_path = plan.profile_source_path.trim();
        let Some(model_file) = plan.model_file.as_deref().map(str::trim) else {
            continue;
        };
        if profile_source_path.is_empty() || model_file.is_empty() {
            continue;
        }
        let asset_key = visualizer_external_model_asset_key(profile_source_path, model_file);
        grouped_assets
            .entry(asset_key.clone())
            .or_insert_with(|| VisualizerExternalModelAssetGroup {
                asset_key,
                profile_source_path: profile_source_path.to_string(),
                model_file: model_file.to_string(),
                plans: Vec::new(),
            })
            .plans
            .push(plan.clone());
    }

    grouped_assets
        .into_values()
        .map(|group| load_visualizer_external_model_asset(group, cache))
        .collect()
}

fn visualizer_external_model_asset_key(profile_source_path: &str, model_file: &str) -> String {
    format!(
        "{}::{}",
        profile_source_path.trim(),
        model_file.trim().replace('\\', "/").to_ascii_lowercase()
    )
}

fn load_gdtf_model_file_maybe_cached(
    cache: Option<&Mutex<HashMap<String, VisualizerModelAssetCacheEntry>>>,
    profile_source_path: &str,
    model_file: &str,
) -> Result<Option<Vec<u8>>, String> {
    if let Some(cache) = cache {
        load_gdtf_model_file_cached(cache, profile_source_path, model_file)
    } else {
        gdtf::load_model_file(profile_source_path, model_file).map_err(|error| error.to_string())
    }
}

fn load_gdtf_model_file_cached(
    cache: &Mutex<HashMap<String, VisualizerModelAssetCacheEntry>>,
    profile_source_path: &str,
    model_file: &str,
) -> Result<Option<Vec<u8>>, String> {
    let cache_key = visualizer_external_model_asset_key(profile_source_path, model_file);
    let (file_length, modified_millis) =
        visualizer_model_asset_file_fingerprint(profile_source_path);
    {
        let cache = cache
            .lock()
            .map_err(|_| "Visualizer model asset cache is unavailable".to_string())?;
        if let Some(entry) = cache.get(&cache_key).filter(|entry| {
            entry.file_length == file_length && entry.modified_millis == modified_millis
        }) {
            return Ok(entry.bytes.clone());
        }
    }

    let bytes = gdtf::load_model_file(profile_source_path, model_file)
        .map_err(|error| error.to_string())?;
    let mut cache = cache
        .lock()
        .map_err(|_| "Visualizer model asset cache is unavailable".to_string())?;
    if cache.len() >= VISUALIZER_MODEL_ASSET_CACHE_LIMIT {
        if let Some(key) = cache.keys().next().cloned() {
            cache.remove(&key);
        }
    }
    cache.insert(
        cache_key,
        VisualizerModelAssetCacheEntry {
            file_length,
            modified_millis,
            bytes: bytes.clone(),
        },
    );
    Ok(bytes)
}

fn visualizer_model_asset_file_fingerprint(
    profile_source_path: &str,
) -> (Option<u64>, Option<u128>) {
    let Ok(metadata) = fs::metadata(profile_source_path) else {
        return (None, None);
    };
    let modified_millis = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis());
    (Some(metadata.len()), modified_millis)
}

fn visualizer_model_asset_cache_summary(
    cache: &HashMap<String, VisualizerModelAssetCacheEntry>,
) -> VisualizerModelAssetCacheSummary {
    let mut loaded_count = 0usize;
    let mut missing_count = 0usize;
    let mut byte_length = 0usize;
    for entry in cache.values() {
        if let Some(bytes) = &entry.bytes {
            loaded_count += 1;
            byte_length += bytes.len();
        } else {
            missing_count += 1;
        }
    }
    VisualizerModelAssetCacheSummary {
        entry_count: cache.len(),
        loaded_count,
        missing_count,
        byte_length,
        limit: VISUALIZER_MODEL_ASSET_CACHE_LIMIT,
    }
}

fn load_visualizer_external_model_asset(
    group: VisualizerExternalModelAssetGroup,
    cache: Option<&Mutex<HashMap<String, VisualizerModelAssetCacheEntry>>>,
) -> VisualizerExternalModelAsset {
    let VisualizerExternalModelAssetGroup {
        asset_key,
        profile_source_path,
        model_file,
        plans,
    } = group;
    if profile_source_path.starts_with("memory://")
        || profile_source_path.starts_with("snapshot://")
    {
        let format = visualizer_external_model_asset_format(&model_file, &[]);
        return VisualizerExternalModelAsset {
            asset_key,
            profile_source_path,
            model_file,
            status: VisualizerExternalModelAssetStatus::Skipped,
            format,
            byte_length: 0,
            meshes: Vec::new(),
            error: Some("virtual profile has no embedded GDTF model assets".to_string()),
            mesh_error: None,
        };
    }

    match load_gdtf_model_file_maybe_cached(cache, &profile_source_path, &model_file) {
        Ok(Some(bytes)) => {
            let format = visualizer_external_model_asset_format(&model_file, &bytes);
            let (meshes, mesh_error) =
                visualizer_external_model_asset_meshes(&format, &bytes, &plans);
            VisualizerExternalModelAsset {
                byte_length: bytes.len(),
                meshes,
                asset_key,
                profile_source_path,
                model_file,
                status: VisualizerExternalModelAssetStatus::Loaded,
                format,
                error: None,
                mesh_error,
            }
        }
        Ok(None) => {
            let format = visualizer_external_model_asset_format(&model_file, &[]);
            VisualizerExternalModelAsset {
                asset_key,
                profile_source_path,
                model_file,
                status: VisualizerExternalModelAssetStatus::Missing,
                format,
                byte_length: 0,
                meshes: Vec::new(),
                error: None,
                mesh_error: None,
            }
        }
        Err(error) => {
            let format = visualizer_external_model_asset_format(&model_file, &[]);
            VisualizerExternalModelAsset {
                asset_key,
                profile_source_path,
                model_file,
                status: VisualizerExternalModelAssetStatus::Error,
                format,
                byte_length: 0,
                meshes: Vec::new(),
                error: Some(error.to_string()),
                mesh_error: None,
            }
        }
    }
}

fn visualizer_external_model_asset_meshes(
    format: &VisualizerExternalModelAssetFormat,
    bytes: &[u8],
    plans: &[visualizer::FixtureModelRenderPlan],
) -> (Vec<visualizer::FixtureModelPrimitiveMesh>, Option<String>) {
    if !matches!(
        format,
        VisualizerExternalModelAssetFormat::Obj
            | VisualizerExternalModelAssetFormat::ThreeDs
            | VisualizerExternalModelAssetFormat::Glb
    ) {
        return (Vec::new(), None);
    }

    let mut meshes = Vec::new();
    let mut errors = Vec::new();
    let mut error_count = 0usize;
    for plan in plans {
        let result = match format {
            VisualizerExternalModelAssetFormat::Obj => {
                visualizer::build_fixture_model_obj_mesh(plan, bytes)
                    .map_err(|error| format!("{error:?}"))
            }
            VisualizerExternalModelAssetFormat::ThreeDs => {
                visualizer::build_fixture_model_3ds_mesh(plan, bytes)
                    .map_err(|error| format!("{error:?}"))
            }
            VisualizerExternalModelAssetFormat::Glb => {
                visualizer::build_fixture_model_glb_mesh(plan, bytes)
                    .map_err(|error| format!("{error:?}"))
            }
            _ => unreachable!(),
        };
        match result {
            Ok(mesh) => meshes.push(mesh),
            Err(error) => {
                error_count += 1;
                if errors.len() < 3 {
                    errors.push(format!("{}: {error}", plan.geometry_name));
                }
            }
        }
    }
    let mesh_error = (!errors.is_empty()).then(|| {
        let suffix = if error_count > errors.len() {
            format!("; {} more error(s) not shown", error_count - errors.len())
        } else {
            String::new()
        };
        let format_label = match format {
            VisualizerExternalModelAssetFormat::Obj => "OBJ",
            VisualizerExternalModelAssetFormat::ThreeDs => "3DS",
            VisualizerExternalModelAssetFormat::Glb => "GLB",
            _ => "External",
        };
        format!(
            "{format_label} mesh conversion failed for {error_count} plan(s): {}{}",
            errors.join("; "),
            suffix
        )
    });
    (meshes, mesh_error)
}

fn visualizer_external_model_asset_format(
    model_file: &str,
    bytes: &[u8],
) -> VisualizerExternalModelAssetFormat {
    if bytes.len() >= 4 && &bytes[..4] == b"glTF" {
        return VisualizerExternalModelAssetFormat::Glb;
    }
    if bytes.len() >= 2 && bytes[0] == 0x4d && bytes[1] == 0x4d {
        return VisualizerExternalModelAssetFormat::ThreeDs;
    }

    let extension = Path::new(model_file)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match extension.as_str() {
        "glb" => VisualizerExternalModelAssetFormat::Glb,
        "gltf" => VisualizerExternalModelAssetFormat::Gltf,
        "obj" => VisualizerExternalModelAssetFormat::Obj,
        "3ds" => VisualizerExternalModelAssetFormat::ThreeDs,
        "dae" | "collada" => VisualizerExternalModelAssetFormat::Collada,
        _ => {
            let trimmed = bytes
                .iter()
                .copied()
                .skip_while(|byte| byte.is_ascii_whitespace())
                .take(16)
                .collect::<Vec<_>>();
            if trimmed.starts_with(b"{") {
                VisualizerExternalModelAssetFormat::Gltf
            } else if trimmed.starts_with(b"<") {
                VisualizerExternalModelAssetFormat::Collada
            } else {
                VisualizerExternalModelAssetFormat::Unknown
            }
        }
    }
}

#[tauri::command]
fn get_video_composition_plans(state: State<'_, AppState>) -> Vec<video::CompositionPlan> {
    video::build_composition_plans(&state.engine.snapshot().video)
}

#[tauri::command]
fn get_video_output_render_plans(
    state: State<'_, AppState>,
) -> Result<Vec<video::VideoOutputRenderPlan>, String> {
    video::build_video_output_render_plans(&state.engine.snapshot().video)
        .map_err(|error| format!("{error:?}"))
}

#[tauri::command]
fn get_external_video_io_plans(state: State<'_, AppState>) -> video::ExternalVideoIoRoutePlans {
    let snapshot = state.engine.snapshot();
    video::build_external_video_io_route_plans(&snapshot.video, &video::video_runtime_status())
}

#[tauri::command]
fn get_external_video_transport_status(
    state: State<'_, AppState>,
) -> Result<video::ExternalVideoTransportStatus, String> {
    state
        .external_video_transport
        .lock()
        .map(|transport| transport.status())
        .map_err(|_| "External video transport runtime lock was poisoned".to_string())
}

struct RecordingExternalVideoTransportDriver<'a> {
    events: &'a mut Vec<ExternalVideoTransportDriverEvent>,
}

impl RecordingExternalVideoTransportDriver<'_> {
    fn push_event(
        &mut self,
        action: ExternalVideoTransportDriverAction,
        route: &video::ExternalVideoTransportRoute,
        message: String,
    ) {
        let sequence = self.events.last().map_or(1, |event| event.sequence + 1);
        self.events.push(ExternalVideoTransportDriverEvent {
            sequence,
            action,
            route: route.clone(),
            message,
        });
        if self.events.len() > EXTERNAL_VIDEO_TRANSPORT_EVENT_LIMIT {
            let overflow = self.events.len() - EXTERNAL_VIDEO_TRANSPORT_EVENT_LIMIT;
            self.events.drain(0..overflow);
        }
    }
}

impl video::ExternalVideoTransportDriver for RecordingExternalVideoTransportDriver<'_> {
    fn start_route(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        self.push_event(
            ExternalVideoTransportDriverAction::Start,
            route,
            format!(
                "Queued {} {} external video route '{}'",
                route.backend_id,
                external_video_transport_direction_label(route.direction),
                route.endpoint_name
            ),
        );
        Ok(())
    }

    fn stop_route(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        self.push_event(
            ExternalVideoTransportDriverAction::Stop,
            route,
            format!(
                "Released {} {} external video route '{}'",
                route.backend_id,
                external_video_transport_direction_label(route.direction),
                route.endpoint_name
            ),
        );
        Ok(())
    }
}

fn external_video_transport_direction_label(
    direction: video::ExternalVideoTransportDirection,
) -> &'static str {
    match direction {
        video::ExternalVideoTransportDirection::Input => "input",
        video::ExternalVideoTransportDirection::Output => "output",
    }
}

fn sync_external_video_transports_from_snapshot(
    snapshot: &EngineSnapshot,
    transport: &Mutex<video::ExternalVideoTransportRuntime>,
    event_log: &Mutex<Vec<ExternalVideoTransportDriverEvent>>,
) -> Result<ExternalVideoTransportSyncResponse, String> {
    let plans =
        video::build_external_video_io_route_plans(&snapshot.video, &video::video_runtime_status());
    let mut transport = transport
        .lock()
        .map_err(|_| "External video transport runtime lock was poisoned".to_string())?;
    let mut events = event_log
        .lock()
        .map_err(|_| "External video transport event log lock was poisoned".to_string())?;
    let mut driver = RecordingExternalVideoTransportDriver {
        events: &mut events,
    };
    let report = transport.sync_routes_with_driver(&plans, &mut driver);
    Ok(ExternalVideoTransportSyncResponse {
        report,
        events: events.clone(),
    })
}

#[tauri::command]
fn sync_external_video_transports(
    state: State<'_, AppState>,
) -> Result<ExternalVideoTransportSyncResponse, String> {
    let snapshot = state.engine.snapshot();
    sync_external_video_transports_from_snapshot(
        &snapshot,
        state.external_video_transport.as_ref(),
        state.external_video_transport_events.as_ref(),
    )
}

#[tauri::command]
fn get_video_runtime_status() -> VideoRuntimeStatus {
    video::video_runtime_status()
}

#[tauri::command]
fn get_video_preview_diagnostics(
    state: State<'_, AppState>,
) -> Result<VideoPreviewDiagnostics, String> {
    let snapshot = state.engine.snapshot();
    let renderer = state
        .video_preview
        .lock()
        .map_err(|_| "Video preview renderer lock was poisoned".to_string())?;
    let provider = renderer.frame_provider();
    let config = renderer.config();
    let prefetch_count = provider.prefetch_count();
    let prefetch_interval_ms = provider.prefetch_interval_ms();
    let bpm = provider.bpm();
    Ok(VideoPreviewDiagnostics {
        queue_count: renderer.queue_count(),
        frame_queue_capacity: config.frame_queue_capacity,
        still_image_cache_len: provider.still_image_cache_len(),
        decoder_cache_len: provider.decoder().cache_len(),
        prefetch_count,
        prefetch_interval_ms,
        bpm,
        layer_queues: snapshot
            .video
            .layers
            .iter()
            .map(|layer| {
                let queue_len = renderer.queue_len(layer.id);
                let expected_positions_ms = video::preview_prefetch_positions_ms(
                    layer,
                    prefetch_count,
                    prefetch_interval_ms,
                    bpm,
                );
                let expected_queue_len = expected_positions_ms
                    .len()
                    .min(config.frame_queue_capacity.max(1));
                VideoPreviewQueueSummary {
                    layer_id: layer.id,
                    label: layer.label.clone(),
                    source_kind: layer.source.kind.clone(),
                    position_ms: layer.state.position_ms,
                    source_duration_ms: layer
                        .source
                        .metadata
                        .as_ref()
                        .and_then(|metadata| metadata.duration_ms),
                    playing: layer.state.playing,
                    effective_speed: video::effective_speed(&layer.state, provider.bpm()),
                    queue_len,
                    expected_queue_len,
                    expected_positions_ms,
                    ready: queue_len >= expected_queue_len,
                }
            })
            .collect(),
        output_decode_previews: video_output_decode_preview_summaries(
            &snapshot.video,
            config,
            prefetch_count,
            prefetch_interval_ms,
            bpm,
        ),
    })
}

fn video_output_decode_preview_summaries(
    snapshot: &protocol::VideoSnapshot,
    config: video::VideoRuntimeConfig,
    prefetch_count: usize,
    prefetch_interval_ms: u64,
    bpm: Option<f32>,
) -> Vec<VideoOutputDecodePreviewSummary> {
    let scheduler_capacity = snapshot
        .layers
        .len()
        .max(1)
        .saturating_mul(prefetch_count.saturating_add(1).max(1));
    snapshot
        .outputs
        .iter()
        .map(|output| {
            let mut scheduler = video::VideoDecodeScheduler::new(scheduler_capacity.max(1));
            let result = scheduler.push_output_preview(
                snapshot,
                output.id,
                config.preview_width,
                config.preview_height,
                prefetch_count,
                prefetch_interval_ms,
                bpm,
            );
            let (report, error) = match result {
                Ok(report) => (Some(report), None),
                Err(error) => (None, Some(format!("{error:?}"))),
            };
            VideoOutputDecodePreviewSummary {
                output_id: output.id,
                label: output.label.clone(),
                width: output.width,
                height: output.height,
                enabled: output.enabled,
                blackout: output.blackout,
                report,
                error,
            }
        })
        .collect()
}

#[cfg(test)]
fn expected_video_preview_queue_len(
    layer: &protocol::VideoLayerSummary,
    prefetch_count: usize,
) -> usize {
    if matches!(layer.source.kind, VideoSourceKind::StillImage) || prefetch_count == 0 {
        1
    } else {
        prefetch_count.saturating_add(1)
    }
}

#[tauri::command]
fn get_debug_video_preview(
    state: State<'_, AppState>,
    width: u32,
    height: u32,
) -> Result<video::VideoFrame, String> {
    let snapshot = state.engine.snapshot();
    let mut renderer = state
        .video_preview
        .lock()
        .map_err(|_| "Video preview renderer lock was poisoned".to_string())?;
    renderer
        .frame_provider_mut()
        .set_bpm(Some(snapshot.clock.bpm));
    let decode_budget = video_preview_decode_budget(&renderer, snapshot.video.layers.len());
    renderer
        .warm_first_composition_decode_queue(&snapshot.video, width, height, decode_budget)
        .map_err(|error| format!("{error:?}"))?;
    renderer
        .render(&snapshot.video, width, height)
        .map_err(|error| format!("{error:?}"))
}

#[tauri::command]
fn get_debug_video_output_preview(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    width: u32,
    height: u32,
    decode_budget: Option<usize>,
) -> Result<video::VideoFrame, String> {
    let snapshot = state.engine.snapshot();
    let mut renderer = state
        .video_preview
        .lock()
        .map_err(|_| "Video preview renderer lock was poisoned".to_string())?;
    renderer
        .frame_provider_mut()
        .set_bpm(Some(snapshot.clock.bpm));
    let decode_budget =
        resolved_video_preview_decode_budget(&renderer, snapshot.video.layers.len(), decode_budget);
    renderer
        .warm_output_decode_queue(&snapshot.video, output_id, width, height, decode_budget)
        .map_err(|error| format!("{error:?}"))?;
    renderer
        .render_output_preview(&snapshot.video, output_id, width, height)
        .map_err(|error| format!("{error:?}"))
}

fn video_preview_decode_budget(renderer: &AppVideoPreviewRenderer, layer_count: usize) -> usize {
    video_preview_decode_budget_from_prefetch(
        layer_count,
        renderer.frame_provider().prefetch_count(),
    )
}

fn video_preview_decode_budget_from_prefetch(layer_count: usize, prefetch_count: usize) -> usize {
    layer_count
        .max(1)
        .saturating_mul(prefetch_count.saturating_add(1).max(1))
}

fn resolved_video_preview_decode_budget(
    renderer: &AppVideoPreviewRenderer,
    layer_count: usize,
    requested: Option<usize>,
) -> usize {
    let full_budget = video_preview_decode_budget(renderer, layer_count);
    requested.unwrap_or(full_budget).clamp(1, full_budget)
}

#[tauri::command]
fn get_debug_video_output_test_pattern(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    width: u32,
    height: u32,
) -> Result<video::VideoFrame, String> {
    let snapshot = state.engine.snapshot();
    video::render_video_output_test_pattern(&snapshot.video, output_id, width, height)
        .map_err(|error| format!("{error:?}"))
}

#[derive(Debug, Clone, Serialize)]
struct VideoOutputWindowStatus {
    output_id: VideoOutputId,
    label: String,
    live_open: bool,
    test_pattern_open: bool,
    live_window_label: String,
    test_pattern_window_label: String,
}

#[derive(Debug, Clone, Serialize)]
struct VideoOutputWindowSyncSummary {
    synced_live: usize,
    synced_test_pattern: usize,
    skipped_closed: usize,
}

#[derive(Debug, Clone, Serialize)]
struct VideoOutputWindowCloseSummary {
    closed_live: usize,
    closed_test_pattern: usize,
    skipped_closed: usize,
}

fn apply_video_output_window_shell(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    output: &VideoOutputSummary,
    test_pattern: bool,
) -> Result<(), String> {
    window
        .set_title(&format!(
            "Rayard {} - {}",
            if test_pattern {
                "Test Pattern"
            } else {
                "Output"
            },
            output.label
        ))
        .map_err(|error| error.to_string())?;

    if let Some(monitor_id) = output.monitor_id {
        let monitors = app
            .available_monitors()
            .map_err(|error| error.to_string())?;
        if let Some(monitor) = monitors.get(monitor_id as usize) {
            window
                .set_position(monitor.position().clone())
                .map_err(|error| error.to_string())?;
        }
    }

    if output.fullscreen {
        window
            .set_decorations(false)
            .map_err(|error| error.to_string())?;
        window
            .set_fullscreen(true)
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    window
        .set_fullscreen(false)
        .map_err(|error| error.to_string())?;
    window
        .set_decorations(true)
        .map_err(|error| error.to_string())?;
    window
        .set_size(tauri::LogicalSize::new(
            f64::from(output.width.max(1)),
            f64::from(output.height.max(1)),
        ))
        .map_err(|error| error.to_string())?;

    Ok(())
}

#[tauri::command]
fn get_video_output_window_statuses(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Vec<VideoOutputWindowStatus> {
    let snapshot = state.engine.snapshot();
    snapshot
        .video
        .outputs
        .iter()
        .filter(|output| output.kind == VideoOutputKind::Display)
        .map(|output| {
            let live_window_label = video_output_window_label(output.id, false);
            let test_pattern_window_label = video_output_window_label(output.id, true);
            VideoOutputWindowStatus {
                output_id: output.id,
                label: output.label.clone(),
                live_open: app.get_webview_window(&live_window_label).is_some(),
                test_pattern_open: app.get_webview_window(&test_pattern_window_label).is_some(),
                live_window_label,
                test_pattern_window_label,
            }
        })
        .collect()
}

#[tauri::command]
async fn sync_open_video_output_windows(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<VideoOutputWindowSyncSummary, String> {
    let snapshot = state.engine.snapshot();
    let mut summary = VideoOutputWindowSyncSummary {
        synced_live: 0,
        synced_test_pattern: 0,
        skipped_closed: 0,
    };

    for output in snapshot
        .video
        .outputs
        .iter()
        .filter(|output| output.kind == VideoOutputKind::Display)
    {
        let live_label = video_output_window_label(output.id, false);
        if let Some(window) = app.get_webview_window(&live_label) {
            apply_video_output_window_shell(&app, &window, output, false)?;
            summary.synced_live += 1;
        } else {
            summary.skipped_closed += 1;
        }

        let test_pattern_label = video_output_window_label(output.id, true);
        if let Some(window) = app.get_webview_window(&test_pattern_label) {
            apply_video_output_window_shell(&app, &window, output, true)?;
            summary.synced_test_pattern += 1;
        } else {
            summary.skipped_closed += 1;
        }
    }

    Ok(summary)
}

#[tauri::command]
async fn close_video_output_window(
    app: tauri::AppHandle,
    output_id: VideoOutputId,
    test_pattern: Option<bool>,
) -> Result<(), String> {
    let test_pattern = test_pattern.unwrap_or(false);
    let label = video_output_window_label(output_id, test_pattern);
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("Video output window {label} is not open"))?;
    window.close().map_err(|error| error.to_string())
}

#[tauri::command]
async fn close_open_video_output_windows(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<VideoOutputWindowCloseSummary, String> {
    let snapshot = state.engine.snapshot();
    let mut summary = VideoOutputWindowCloseSummary {
        closed_live: 0,
        closed_test_pattern: 0,
        skipped_closed: 0,
    };

    for output in snapshot
        .video
        .outputs
        .iter()
        .filter(|output| output.kind == VideoOutputKind::Display)
    {
        let live_label = video_output_window_label(output.id, false);
        if let Some(window) = app.get_webview_window(&live_label) {
            window.close().map_err(|error| error.to_string())?;
            summary.closed_live += 1;
        } else {
            summary.skipped_closed += 1;
        }

        let test_pattern_label = video_output_window_label(output.id, true);
        if let Some(window) = app.get_webview_window(&test_pattern_label) {
            window.close().map_err(|error| error.to_string())?;
            summary.closed_test_pattern += 1;
        } else {
            summary.skipped_closed += 1;
        }
    }

    Ok(summary)
}

#[tauri::command]
async fn sync_video_output_window(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    test_pattern: Option<bool>,
) -> Result<(), String> {
    let test_pattern = test_pattern.unwrap_or(false);
    let snapshot = state.engine.snapshot();
    let output = snapshot
        .video
        .outputs
        .iter()
        .find(|output| output.id == output_id)
        .cloned()
        .ok_or_else(|| format!("Video output {output_id} was not found"))?;
    if output.kind != VideoOutputKind::Display {
        return Err("Only Display video outputs can be synced as windows".to_string());
    }
    let label = video_output_window_label(output_id, test_pattern);
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("Video output window {label} is not open"))?;
    apply_video_output_window_shell(&app, &window, &output, test_pattern)
}

#[tauri::command]
async fn open_video_output_window(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    test_pattern: Option<bool>,
) -> Result<(), String> {
    let test_pattern = test_pattern.unwrap_or(false);
    let snapshot = state.engine.snapshot();
    let output = snapshot
        .video
        .outputs
        .iter()
        .find(|output| output.id == output_id)
        .cloned()
        .ok_or_else(|| format!("Video output {output_id} was not found"))?;
    if output.kind != VideoOutputKind::Display {
        return Err("Only Display video outputs can be opened as windows".to_string());
    }

    let label = video_output_window_label(output_id, test_pattern);
    if let Some(window) = app.get_webview_window(&label) {
        apply_video_output_window_shell(&app, &window, &output, test_pattern)?;
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    let url = tauri::WebviewUrl::App(
        format!(
            "index.html?videoOutputId={output_id}{}",
            if test_pattern { "&testPattern=1" } else { "" }
        )
        .into(),
    );
    let mut builder = tauri::WebviewWindowBuilder::new(&app, label, url)
        .title(format!(
            "Rayard {} - {}",
            if test_pattern {
                "Test Pattern"
            } else {
                "Output"
            },
            output.label
        ))
        .inner_size(output.width as f64, output.height as f64)
        .resizable(true)
        .decorations(!output.fullscreen)
        .fullscreen(output.fullscreen);

    if let Some(monitor_id) = output.monitor_id {
        let monitors = app
            .available_monitors()
            .map_err(|error| error.to_string())?;
        if let Some(monitor) = monitors.get(monitor_id as usize) {
            let position = monitor.position();
            builder = builder.position(position.x as f64, position.y as f64);
        }
    }

    let window = builder.build().map_err(|error| error.to_string())?;
    apply_video_output_window_shell(&app, &window, &output, test_pattern)?;
    Ok(())
}

fn load_patch_profile(
    state: &State<'_, AppState>,
    profile_path: &str,
) -> Result<FixtureProfileSummary, String> {
    load_patch_profile_from_cache_or_file(&state.custom_profiles, profile_path)
}

fn import_gdtf_from_path(path: String) -> Result<FixtureProfileSummary, String> {
    let path = validate_existing_file_path(path, "GDTF file")?;
    gdtf::load_profile(path).map_err(|error| error.to_string())
}

fn cache_fixture_profile(
    profiles: &Mutex<HashMap<String, FixtureProfileSummary>>,
    profile: &FixtureProfileSummary,
) -> Result<(), String> {
    let source_path = profile.source_path.trim();
    if source_path.is_empty() {
        return Ok(());
    }
    profiles
        .lock()
        .map_err(|_| "Fixture profile state lock was poisoned".to_string())?
        .insert(source_path.to_string(), profile.clone());
    Ok(())
}

fn cached_fixture_profile(
    profiles: &Mutex<HashMap<String, FixtureProfileSummary>>,
    profile_path: &str,
) -> Result<Option<FixtureProfileSummary>, String> {
    profiles
        .lock()
        .map_err(|_| "Fixture profile state lock was poisoned".to_string())
        .map(|profiles| profiles.get(profile_path).cloned())
}

fn load_patch_profile_from_cache_or_file(
    profiles: &Mutex<HashMap<String, FixtureProfileSummary>>,
    profile_path: &str,
) -> Result<FixtureProfileSummary, String> {
    let profile_path = profile_path.trim();
    if profile_path.starts_with("memory://") || profile_path.starts_with("snapshot://") {
        return cached_fixture_profile(profiles, profile_path)?
            .ok_or_else(|| format!("Memory fixture profile {profile_path} was not found"));
    }

    match gdtf::load_profile(profile_path) {
        Ok(profile) => {
            cache_fixture_profile(profiles, &profile)?;
            Ok(profile)
        }
        Err(error) => cached_fixture_profile(profiles, profile_path)?.ok_or_else(|| {
            format!("Fixture profile {profile_path} could not be loaded from disk or project cache: {error}")
        }),
    }
}

fn register_custom_fixture_profile(
    state: &State<'_, AppState>,
    request: CustomFixtureProfileRequest,
) -> Result<FixtureProfileSummary, String> {
    validate_custom_fixture_profile_request(&request)?;
    let profile = custom_fixture_profile_from_request(request);
    state
        .custom_profiles
        .lock()
        .map_err(|_| "Fixture profile state lock was poisoned".to_string())?
        .insert(profile.source_path.clone(), profile.clone());
    Ok(profile)
}

fn fixture_profile_from_patched_fixture(fixture: &PatchedFixtureSummary) -> FixtureProfileSummary {
    let has_geometry = !fixture.geometries.is_empty();
    FixtureProfileSummary {
        source_path: format!("memory://patched-fixture/{}", fixture.id),
        manufacturer: fixture.manufacturer.clone(),
        name: fixture.profile_name.clone(),
        short_name: None,
        fixture_type_id: None,
        dmx_modes: vec![DmxModeSummary {
            name: fixture.mode_name.clone(),
            controls: fixture.controls.clone(),
        }],
        geometries: fixture.geometries.clone(),
        warnings: vec![if has_geometry {
            format!(
                "Rebuilt from patched fixture {}; original GDTF XML and mesh assets are not embedded",
                fixture.label
            )
        } else {
            format!(
                "Rebuilt from patched fixture {}; GDTF geometry and physical data are not available",
                fixture.label
            )
        }],
    }
}

fn validate_custom_fixture_profile_file(
    profile_file: &CustomFixtureProfileFile,
) -> Result<(), String> {
    if profile_file.version != 1 {
        return Err(format!(
            "Unsupported custom fixture profile version {}",
            profile_file.version
        ));
    }
    validate_custom_fixture_profile_request(&profile_file.request)
}

fn validate_custom_fixture_profile_request(
    request: &CustomFixtureProfileRequest,
) -> Result<(), String> {
    if request.manufacturer.trim().is_empty() {
        return Err("Custom profile manufacturer is required".to_string());
    }
    if request.name.trim().is_empty() {
        return Err("Custom profile name is required".to_string());
    }
    if request.mode_name.trim().is_empty() {
        return Err("Custom profile mode name is required".to_string());
    }
    if request.attributes.is_empty() {
        return Err("Custom profile requires at least one attribute".to_string());
    }
    resolve_custom_attribute_layouts(&request.attributes)?;
    Ok(())
}

fn parse_custom_attribute_spec(value: &str) -> Result<CustomAttributeSpec, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("Custom profile attributes must not be empty".to_string());
    }
    let mut attribute = trimmed;
    let mut resolution_text = None;
    let mut start_offset = None;

    if let Some((name, channel_and_resolution)) = trimmed.rsplit_once('@') {
        attribute = name.trim();
        let channel_and_resolution = channel_and_resolution.trim();
        if let Some((channel, resolution)) = channel_and_resolution.split_once(':') {
            start_offset = Some(parse_custom_attribute_start_offset(
                attribute,
                channel.trim(),
            )?);
            resolution_text = Some(resolution.trim());
        } else if !channel_and_resolution.is_empty()
            && channel_and_resolution
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            start_offset = Some(parse_custom_attribute_start_offset(
                attribute,
                channel_and_resolution,
            )?);
        } else {
            resolution_text = Some(channel_and_resolution);
        }
    } else if let Some((name, resolution)) = trimmed.rsplit_once(':') {
        attribute = name.trim();
        resolution_text = Some(resolution.trim());
    }

    if attribute.is_empty() {
        return Err("Custom profile attributes must include an attribute name".to_string());
    }
    let resolution = parse_custom_attribute_resolution(attribute, resolution_text)?;
    Ok(CustomAttributeSpec {
        attribute: attribute.to_string(),
        resolution,
        start_offset,
    })
}

fn parse_custom_attribute_start_offset(attribute: &str, value: &str) -> Result<u16, String> {
    let offset = value.parse::<u16>().map_err(|_| {
        format!("Custom profile attribute '{attribute}' has invalid start channel '{value}'")
    })?;
    if offset == 0 || offset > 512 {
        return Err(format!(
            "Custom profile attribute '{attribute}' start channel must be 1-512"
        ));
    }
    Ok(offset)
}

fn parse_custom_attribute_resolution(
    attribute: &str,
    resolution_text: Option<&str>,
) -> Result<AttributeResolution, String> {
    match resolution_text
        .map(|resolution| resolution.to_ascii_lowercase().replace(['-', '_', ' '], ""))
        .as_deref()
    {
        None | Some("") | Some("8") | Some("8bit") => Ok(AttributeResolution::EightBit),
        Some("16") | Some("16bit") => Ok(AttributeResolution::SixteenBit),
        Some(other) => Err(format!(
            "Custom profile attribute '{attribute}' has invalid resolution '{other}'"
        )),
    }
}

fn resolve_custom_attribute_layouts(
    attributes: &[String],
) -> Result<Vec<CustomAttributeLayout>, String> {
    let mut seen = HashSet::new();
    let mut occupied = HashSet::new();
    let mut next_offset = 1u16;
    let mut layouts = Vec::new();

    for attribute in attributes {
        let spec = parse_custom_attribute_spec(attribute)?;
        if !seen.insert(spec.attribute.to_ascii_lowercase()) {
            return Err(format!(
                "Custom profile attribute '{}' is duplicated",
                spec.attribute
            ));
        }

        let width = match &spec.resolution {
            AttributeResolution::EightBit => 1u16,
            AttributeResolution::SixteenBit => 2u16,
        };
        let start = spec.start_offset.unwrap_or(next_offset);
        let end = start
            .checked_add(width - 1)
            .ok_or_else(|| "Custom profile cannot exceed 512 DMX channels".to_string())?;
        if end > 512 {
            return Err("Custom profile cannot exceed 512 DMX channels".to_string());
        }

        let offsets = (start..=end).collect::<Vec<_>>();
        if let Some(overlap) = offsets
            .iter()
            .find(|offset| occupied.contains(*offset))
            .copied()
        {
            return Err(format!(
                "Custom profile attribute '{}' overlaps DMX channel {}",
                spec.attribute, overlap
            ));
        }
        for offset in &offsets {
            occupied.insert(*offset);
        }
        next_offset = next_offset.max(end.saturating_add(1));
        layouts.push(CustomAttributeLayout {
            attribute: spec.attribute,
            resolution: spec.resolution,
            offsets,
        });
    }

    Ok(layouts)
}

fn custom_fixture_profile_from_request(
    request: CustomFixtureProfileRequest,
) -> FixtureProfileSummary {
    let manufacturer = request.manufacturer.trim().to_string();
    let name = request.name.trim().to_string();
    let mode_name = request.mode_name.trim().to_string();
    let source_path = format!(
        "memory://custom/{}-{}",
        safe_file_stem(&manufacturer),
        safe_file_stem(&name)
    );
    let controls = resolve_custom_attribute_layouts(&request.attributes)
        .expect("custom fixture profile request must be valid before building")
        .into_iter()
        .map(|layout| AttributeControl {
            attribute: layout.attribute.clone(),
            channel_name: layout.attribute.clone(),
            geometry: Some(custom_attribute_geometry_name(&layout.attribute).to_string()),
            offsets: layout.offsets,
            resolution: layout.resolution,
            default_value: custom_attribute_default_value(&layout.attribute),
            functions: Vec::new(),
        })
        .collect();

    FixtureProfileSummary {
        source_path,
        manufacturer,
        name,
        short_name: None,
        fixture_type_id: None,
        dmx_modes: vec![DmxModeSummary {
            name: mode_name,
            controls,
        }],
        geometries: custom_profile_geometries(),
        warnings: vec![
            "Custom profile uses generated Body/Head/Beam geometry; no GDTF mesh assets are embedded"
                .to_string(),
        ],
    }
}

fn custom_profile_geometries() -> Vec<GeometrySummary> {
    vec![
        custom_profile_geometry(
            "Body",
            "Geometry",
            None,
            custom_profile_geometry_matrix(0.0, 0.0, 0.0),
            "Cube",
            Vec3 {
                x: 0.5,
                y: 0.25,
                z: 0.5,
            },
            None,
        ),
        custom_profile_geometry(
            "Head",
            "Axis",
            Some("Body"),
            custom_profile_geometry_matrix(0.0, 0.3, 0.0),
            "Sphere",
            Vec3 {
                x: 0.36,
                y: 0.36,
                z: 0.36,
            },
            None,
        ),
        custom_profile_geometry(
            "Beam",
            "Beam",
            Some("Head"),
            custom_profile_geometry_matrix(0.0, 0.0, -0.45),
            "Cylinder",
            Vec3 {
                x: 0.18,
                y: 0.18,
                z: 0.18,
            },
            Some(("Wash", 18.0, 28.0, 0.12)),
        ),
    ]
}

fn custom_profile_geometry(
    name: &str,
    kind: &str,
    parent: Option<&str>,
    matrix: [f32; 16],
    primitive: &str,
    dimensions: Vec3,
    beam: Option<(&str, f32, f32, f32)>,
) -> GeometrySummary {
    GeometrySummary {
        name: name.to_string(),
        kind: kind.to_string(),
        parent: parent.map(str::to_string),
        matrix,
        model_name: None,
        model_file: None,
        model_primitive: Some(primitive.to_string()),
        model_dimensions: Some(dimensions),
        beam_type: beam.map(|(beam_type, _, _, _)| beam_type.to_string()),
        beam_angle_deg: beam.map(|(_, angle, _, _)| angle),
        field_angle_deg: beam.map(|(_, _, field, _)| field),
        beam_radius: beam.map(|(_, _, _, radius)| radius),
    }
}

fn custom_profile_geometry_matrix(x: f32, y: f32, z: f32) -> [f32; 16] {
    [
        1.0, 0.0, 0.0, x, //
        0.0, 1.0, 0.0, y, //
        0.0, 0.0, 1.0, z, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

fn custom_attribute_geometry_name(attribute: &str) -> &'static str {
    let normalized = normalize_custom_attribute_name(attribute);
    if matches!(
        normalized.as_str(),
        "pan" | "tilt" | "panrotate" | "tiltrotate" | "panfine" | "tiltfine"
    ) {
        return "Head";
    }
    if normalized.contains("color")
        || normalized.contains("dimmer")
        || normalized.contains("shutter")
        || normalized.contains("strobe")
        || normalized.contains("gobo")
        || normalized.contains("beam")
        || normalized.contains("zoom")
        || normalized.contains("focus")
        || normalized.contains("frost")
        || normalized.contains("iris")
        || normalized.contains("prism")
        || matches!(
            normalized.as_str(),
            "red" | "green" | "blue" | "white" | "amber" | "uv" | "lime" | "cyan" | "magenta"
        )
    {
        return "Beam";
    }
    "Body"
}

fn normalize_custom_attribute_name(attribute: &str) -> String {
    attribute
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn custom_attribute_default_value(attribute: &str) -> u16 {
    match attribute.to_ascii_lowercase().as_str() {
        "pan" | "tilt" | "panrotate" | "tiltrotate" => 32_768,
        _ => 0,
    }
}

fn validate_dmx_output_config(config: &DmxOutputConfig) -> Result<(), String> {
    if !config.enabled {
        return Ok(());
    }

    let is_serial_dmx = matches!(
        config.protocol,
        DmxOutputProtocol::EnttecUsbPro
            | DmxOutputProtocol::DmxKingUltraDmx
            | DmxOutputProtocol::EnttecOpenDmx
    );
    let is_sacn_multicast = matches!(config.protocol, DmxOutputProtocol::Sacn)
        && is_sacn_multicast_target(&config.target_ip);
    if !is_serial_dmx && !is_sacn_multicast && config.target_ip.trim().is_empty() {
        return Err("DMX output target IP is required".to_string());
    }
    if !is_serial_dmx && !is_sacn_multicast && config.target_ip.chars().any(char::is_whitespace) {
        return Err("DMX output target IP must not contain whitespace".to_string());
    }
    if !is_serial_dmx && config.port == 0 {
        return Err("DMX output port must be greater than 0".to_string());
    }
    if matches!(config.protocol, DmxOutputProtocol::ArtNet) && config.universe > ARTNET_MAX_UNIVERSE
    {
        return Err(format!(
            "Art-Net universe must be between 0 and {ARTNET_MAX_UNIVERSE}"
        ));
    }
    if matches!(config.protocol, DmxOutputProtocol::Sacn) && config.universe == 0 {
        return Err("sACN universe must be 1 or greater".to_string());
    }
    if matches!(config.protocol, DmxOutputProtocol::Sacn) && config.universe > SACN_MAX_UNIVERSE {
        return Err(format!(
            "sACN universe must be between 1 and {SACN_MAX_UNIVERSE}"
        ));
    }
    if is_serial_dmx {
        if config.serial_port.trim().is_empty() {
            return Err("Serial port is required for serial DMX output".to_string());
        }
        if matches!(
            config.protocol,
            DmxOutputProtocol::EnttecUsbPro | DmxOutputProtocol::DmxKingUltraDmx
        ) && config.serial_baud_rate == 0
        {
            return Err("Serial baud rate must be greater than 0".to_string());
        }
    }
    Ok(())
}

fn validate_dmx_output_routes(configs: &[DmxOutputConfig]) -> Result<(), String> {
    if configs.is_empty() {
        return Err("At least one DMX output route is required".to_string());
    }
    if configs.len() > 64 {
        return Err("Cannot configure more than 64 DMX output routes".to_string());
    }

    let mut route_keys = HashSet::new();
    for config in configs {
        validate_dmx_output_config(config)?;
        if !config.enabled {
            continue;
        }
        let key = dmx_output_route_key(config);
        if !route_keys.insert(key) {
            return Err(format!(
                "Duplicate DMX output route {}",
                dmx_output_route_label(config)
            ));
        }
    }
    Ok(())
}

fn build_dmx_test_frame(channel: u16, width: u16, value: u16) -> Result<[u8; 512], String> {
    if channel == 0 || channel > 512 {
        return Err("DMX test channel must be between 1 and 512".to_string());
    }
    if width == 0 {
        return Err("DMX test width must be at least 1 channel".to_string());
    }
    let end = channel
        .checked_add(width - 1)
        .ok_or_else(|| "DMX test range exceeds 512 channels".to_string())?;
    if end > 512 {
        return Err(format!(
            "DMX test range exceeds 512 channels: CH {} + {}ch ends at CH {}",
            channel, width, end
        ));
    }
    if value > 255 {
        return Err("DMX test value must be between 0 and 255".to_string());
    }

    let mut frame = [0u8; 512];
    let start_index = (channel - 1) as usize;
    let end_index = end as usize;
    for slot in &mut frame[start_index..end_index] {
        *slot = value as u8;
    }
    Ok(frame)
}

fn send_dmx_config_test_frame(
    config: &DmxOutputConfig,
    channel: u16,
    width: u16,
    value: u16,
    frame: &[u8; 512],
) -> Result<DmxTestFrameResult, String> {
    if !config.enabled {
        return Err("Enable the DMX output route before sending a test frame".to_string());
    }
    validate_dmx_output_config(config)?;
    let bytes = match config.protocol {
        DmxOutputProtocol::ArtNet => io::artnet::ArtNetSender::new(&config.target_ip, config.port)
            .and_then(|sender| sender.send_dmx_frame(config.universe, frame))
            .map_err(|error| error.to_string())?,
        DmxOutputProtocol::Sacn => io::sacn::SacnSender::new(&config.target_ip, config.port)
            .and_then(|sender| sender.send_dmx_frame(config.universe, frame))
            .map_err(|error| error.to_string())?,
        DmxOutputProtocol::EnttecUsbPro | DmxOutputProtocol::DmxKingUltraDmx => {
            let mut sender = io::serial_dmx::EnttecUsbProSender::new(
                &config.serial_port,
                config.serial_baud_rate,
            )
            .map_err(|error| error.to_string())?;
            sender
                .send_dmx_frame(frame)
                .map_err(|error| error.to_string())?
        }
        DmxOutputProtocol::EnttecOpenDmx => {
            let mut sender = io::serial_dmx::EnttecOpenDmxSender::new(&config.serial_port)
                .map_err(|error| error.to_string())?;
            sender
                .send_dmx_frame(frame)
                .map_err(|error| error.to_string())?
        }
    };
    Ok(DmxTestFrameResult {
        protocol: config.protocol.clone(),
        universe: config.universe,
        channel,
        width,
        value: value as u8,
        bytes,
    })
}

fn send_dmx_route_test_frames(
    configs: &[DmxOutputConfig],
    channel: u16,
    width: u16,
    value: u16,
    frame: &[u8; 512],
) -> Result<Vec<DmxTestFrameResult>, String> {
    validate_dmx_output_routes(configs)?;
    let mut results = Vec::new();
    for config in configs.iter().filter(|config| config.enabled) {
        results.push(send_dmx_config_test_frame(
            config, channel, width, value, frame,
        )?);
    }
    if results.is_empty() {
        return Err("Enable at least one DMX output route before sending a test frame".to_string());
    }
    Ok(results)
}

fn dmx_output_route_key(config: &DmxOutputConfig) -> String {
    match config.protocol {
        DmxOutputProtocol::ArtNet | DmxOutputProtocol::Sacn => format!(
            "{:?}|{}|{}|{}",
            config.protocol,
            dmx_network_route_target_key(config),
            config.port,
            config.universe
        ),
        DmxOutputProtocol::EnttecUsbPro
        | DmxOutputProtocol::DmxKingUltraDmx
        | DmxOutputProtocol::EnttecOpenDmx => {
            format!("serial|{}", config.serial_port.trim().to_ascii_lowercase())
        }
    }
}

fn dmx_network_route_target_key(config: &DmxOutputConfig) -> String {
    if matches!(config.protocol, DmxOutputProtocol::Sacn)
        && is_sacn_multicast_target(&config.target_ip)
    {
        "multicast".to_string()
    } else {
        config.target_ip.trim().to_ascii_lowercase()
    }
}

fn dmx_output_route_label(config: &DmxOutputConfig) -> String {
    match config.protocol {
        DmxOutputProtocol::ArtNet | DmxOutputProtocol::Sacn => {
            format!(
                "{:?} {}:{} U{}",
                config.protocol, config.target_ip, config.port, config.universe
            )
        }
        DmxOutputProtocol::EnttecUsbPro
        | DmxOutputProtocol::DmxKingUltraDmx
        | DmxOutputProtocol::EnttecOpenDmx => {
            format!(
                "{:?} {} U{}",
                config.protocol, config.serial_port, config.universe
            )
        }
    }
}

fn validate_patch_request(request: &PatchFixtureRequest) -> Result<(), String> {
    if request.profile_path.trim().is_empty() {
        return Err("GDTF profile path is required".to_string());
    }
    if request.address == 0 || request.address > 512 {
        return Err("DMX address must be between 1 and 512".to_string());
    }
    if request.label.trim().is_empty() {
        return Err("Fixture label is required".to_string());
    }
    validate_group_ids(&request.group_ids)?;
    validate_fixture_transform(&request.position, &request.rotation)?;
    Ok(())
}

fn validate_fixture_limits(limits: &FixtureLimits) -> Result<(), String> {
    let _ = normalize_fixture_limits(*limits);
    Ok(())
}

fn normalize_fixture_limits(limits: FixtureLimits) -> FixtureLimits {
    FixtureLimits {
        dimmer_min: limits.dimmer_min.min(limits.dimmer_max),
        dimmer_max: limits.dimmer_min.max(limits.dimmer_max),
        pan_min: limits.pan_min.min(limits.pan_max),
        pan_max: limits.pan_min.max(limits.pan_max),
        tilt_min: limits.tilt_min.min(limits.tilt_max),
        tilt_max: limits.tilt_min.max(limits.tilt_max),
        invert_pan: limits.invert_pan,
        invert_tilt: limits.invert_tilt,
        swap_pan_tilt: limits.swap_pan_tilt,
    }
}

fn cue_targets_from_snapshot(
    snapshot: &EngineSnapshot,
) -> (
    Vec<CueFixtureTarget>,
    Vec<VideoLayerTarget>,
    Vec<VideoOutputTarget>,
    Vec<CueNodeGraphTarget>,
) {
    let targets = snapshot
        .fixtures
        .iter()
        .map(|fixture| CueFixtureTarget {
            fixture_id: fixture.id,
            values: fixture.attribute_values.clone(),
        })
        .collect::<Vec<_>>();
    let video_targets = snapshot
        .video
        .layers
        .iter()
        .map(|layer| VideoLayerTarget {
            layer_id: layer.id,
            state: layer.state.clone(),
        })
        .collect::<Vec<_>>();
    let video_output_targets = snapshot
        .video
        .outputs
        .iter()
        .map(|output| VideoOutputTarget {
            output_id: output.id,
            enabled: output.enabled,
            opacity: output.opacity,
            blackout: output.blackout,
        })
        .collect::<Vec<_>>();
    let node_graph_targets = snapshot
        .node_graphs
        .iter()
        .map(|graph| CueNodeGraphTarget {
            graph_id: graph.id,
            enabled: graph.enabled,
        })
        .collect::<Vec<_>>();
    (
        targets,
        video_targets,
        video_output_targets,
        node_graph_targets,
    )
}

fn cue_targets_from_snapshot_with_scope(
    snapshot: &EngineSnapshot,
    scope: &CueCaptureScope,
) -> Result<
    (
        Vec<CueFixtureTarget>,
        Vec<VideoLayerTarget>,
        Vec<VideoOutputTarget>,
        Vec<CueNodeGraphTarget>,
    ),
    String,
> {
    let (targets, video_targets, video_output_targets, node_graph_targets) =
        cue_targets_from_snapshot(snapshot);
    match scope {
        CueCaptureScope::All => Ok((
            targets,
            video_targets,
            video_output_targets,
            node_graph_targets,
        )),
        CueCaptureScope::LightingOnly => Ok((targets, Vec::new(), Vec::new(), Vec::new())),
        CueCaptureScope::SelectedFixture { fixture_id } => {
            if !snapshot
                .fixtures
                .iter()
                .any(|fixture| fixture.id == *fixture_id)
            {
                return Err(format!("Fixture {fixture_id} was not found"));
            }
            Ok((
                targets
                    .into_iter()
                    .filter(|target| target.fixture_id == *fixture_id)
                    .collect(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ))
        }
        CueCaptureScope::SelectedGroup { group_id } => {
            let group_id = normalize_control_group_id(group_id.clone())?;
            let fixture_ids = snapshot
                .fixtures
                .iter()
                .filter(|fixture| {
                    fixture
                        .group_ids
                        .iter()
                        .any(|candidate| group_matches(candidate, &group_id))
                })
                .map(|fixture| fixture.id)
                .collect::<HashSet<_>>();
            if fixture_ids.is_empty() {
                return Err(format!("Group {group_id} has no patched fixtures"));
            }
            Ok((
                targets
                    .into_iter()
                    .filter(|target| fixture_ids.contains(&target.fixture_id))
                    .collect(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ))
        }
        CueCaptureScope::VideoOnly => Ok((
            Vec::new(),
            video_targets,
            video_output_targets,
            node_graph_targets,
        )),
    }
}

fn ensure_cue_targets_present(
    fixture_targets: &[CueFixtureTarget],
    video_targets: &[VideoLayerTarget],
    video_output_targets: &[VideoOutputTarget],
    node_graph_targets: &[CueNodeGraphTarget],
    operation: &str,
) -> Result<(), String> {
    if fixture_targets.is_empty()
        && video_targets.is_empty()
        && video_output_targets.is_empty()
        && node_graph_targets.is_empty()
    {
        return Err(format!(
            "Patch a fixture, add a video layer, add a video output, or save a node graph before {operation} a cue"
        ));
    }
    Ok(())
}

fn merge_cue_update_targets(
    snapshot: &EngineSnapshot,
    cue_id: CueId,
    scope: &CueCaptureScope,
    captured: (
        Vec<CueFixtureTarget>,
        Vec<VideoLayerTarget>,
        Vec<VideoOutputTarget>,
        Vec<CueNodeGraphTarget>,
    ),
) -> Result<
    (
        Vec<CueFixtureTarget>,
        Vec<VideoLayerTarget>,
        Vec<VideoOutputTarget>,
        Vec<CueNodeGraphTarget>,
    ),
    String,
> {
    let existing = snapshot
        .cues
        .iter()
        .find(|cue| cue.id == cue_id)
        .ok_or_else(|| format!("Cue {cue_id} was not found"))?;
    let (
        captured_fixture_targets,
        captured_video_targets,
        captured_video_output_targets,
        captured_node_graph_targets,
    ) = captured;
    match scope {
        CueCaptureScope::All => Ok((
            captured_fixture_targets,
            captured_video_targets,
            captured_video_output_targets,
            captured_node_graph_targets,
        )),
        CueCaptureScope::LightingOnly => Ok((
            captured_fixture_targets,
            existing.video_targets.clone(),
            existing.video_output_targets.clone(),
            existing.node_graph_targets.clone(),
        )),
        CueCaptureScope::SelectedFixture { .. } | CueCaptureScope::SelectedGroup { .. } => Ok((
            merge_by_id(
                existing.targets.clone(),
                captured_fixture_targets,
                |target| target.fixture_id,
            ),
            existing.video_targets.clone(),
            existing.video_output_targets.clone(),
            existing.node_graph_targets.clone(),
        )),
        CueCaptureScope::VideoOnly => Ok((
            existing.targets.clone(),
            captured_video_targets,
            captured_video_output_targets,
            captured_node_graph_targets,
        )),
    }
}

fn merge_by_id<T, Id, F>(existing: Vec<T>, captured: Vec<T>, id_for: F) -> Vec<T>
where
    Id: Eq + std::hash::Hash + Copy,
    F: Fn(&T) -> Id,
{
    let captured_ids = captured
        .iter()
        .map(|target| id_for(target))
        .collect::<HashSet<_>>();
    existing
        .into_iter()
        .filter(|target| !captured_ids.contains(&id_for(target)))
        .chain(captured)
        .collect()
}

fn validate_timeline_automation_request(
    snapshot: &EngineSnapshot,
    fixture_id: FixtureId,
    attribute: &str,
    keyframes: &[AutomationKeyframeSummary],
) -> Result<(), String> {
    if attribute.trim().is_empty() {
        return Err("Automation attribute is required".to_string());
    }
    if keyframes.is_empty() {
        return Err("Automation requires at least one keyframe".to_string());
    }
    let Some(fixture) = snapshot
        .fixtures
        .iter()
        .find(|fixture| fixture.id == fixture_id)
    else {
        return Err(format!("Fixture {fixture_id} was not found"));
    };
    if !fixture
        .controls
        .iter()
        .any(|control| control.attribute == attribute)
    {
        return Err(format!(
            "Fixture {} has no attribute {}",
            fixture.label, attribute
        ));
    }
    Ok(())
}

fn compatible_timeline_automation_targets(
    snapshot: &EngineSnapshot,
    group_id: &str,
    attribute: &str,
    keyframes: &[AutomationKeyframeSummary],
) -> Result<(Vec<FixtureId>, usize), String> {
    if attribute.trim().is_empty() {
        return Err("Automation attribute is required".to_string());
    }
    if keyframes.is_empty() {
        return Err("Automation requires at least one keyframe".to_string());
    }

    let mut group_fixture_count = 0usize;
    let mut fixture_ids = Vec::new();
    for fixture in &snapshot.fixtures {
        let in_group = fixture
            .group_ids
            .iter()
            .any(|fixture_group_id| group_matches(fixture_group_id, group_id));
        if !in_group {
            continue;
        }
        group_fixture_count += 1;
        if fixture
            .controls
            .iter()
            .any(|control| control.attribute == attribute)
        {
            fixture_ids.push(fixture.id);
        }
    }

    if group_fixture_count == 0 {
        return Err(format!(
            "Group '{group_id}' was not found or has no fixtures"
        ));
    }
    if fixture_ids.is_empty() {
        return Err(format!(
            "No fixtures in group '{group_id}' expose attribute {attribute}"
        ));
    }
    for fixture_id in &fixture_ids {
        validate_timeline_automation_request(snapshot, *fixture_id, attribute, keyframes)?;
    }
    let applied_count = fixture_ids.len();
    Ok((fixture_ids, group_fixture_count - applied_count))
}

fn validate_timeline_video_automation_request(
    snapshot: &EngineSnapshot,
    layer_id: VideoLayerId,
    keyframes: &[VideoAutomationKeyframeSummary],
) -> Result<(), String> {
    if keyframes.is_empty() {
        return Err("Video automation requires at least one keyframe".to_string());
    }
    if keyframes.iter().any(|keyframe| !keyframe.value.is_finite()) {
        return Err("Video automation values must be finite".to_string());
    }
    if !snapshot
        .video
        .layers
        .iter()
        .any(|layer| layer.id == layer_id)
    {
        return Err(format!("Video layer {layer_id} was not found"));
    }
    Ok(())
}

fn validate_fixture_transform(position: &Vec3, rotation: &Rotation3) -> Result<(), String> {
    if !position.x.is_finite() || !position.y.is_finite() || !position.z.is_finite() {
        return Err("Fixture position values must be finite".to_string());
    }
    if !rotation.pitch.is_finite() || !rotation.yaw.is_finite() || !rotation.roll.is_finite() {
        return Err("Fixture rotation values must be finite".to_string());
    }
    Ok(())
}

fn validate_stage_map_config(config: &StageMapConfig) -> Result<(), String> {
    if !config.min_x.is_finite()
        || !config.max_x.is_finite()
        || !config.min_z.is_finite()
        || !config.max_z.is_finite()
    {
        return Err("Stage map bounds must be finite".to_string());
    }
    if config.min_x >= config.max_x || config.min_z >= config.max_z {
        return Err("Stage map min bounds must be lower than max bounds".to_string());
    }
    if config.max_x - config.min_x < 0.1 || config.max_z - config.min_z < 0.1 {
        return Err("Stage map bounds must span at least 0.1m".to_string());
    }
    Ok(())
}

fn normalize_stage_object(mut object: StageObjectSummary) -> Result<StageObjectSummary, String> {
    if object.id == 0 {
        return Err("Stage object id must be greater than zero".to_string());
    }
    object.label = object.label.trim().to_string();
    if object.label.is_empty() {
        return Err("Stage object label is required".to_string());
    }
    if [
        object.x,
        object.z,
        object.width,
        object.depth,
        object.rotation_deg,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err("Stage object values must be finite".to_string());
    }
    if object.width <= 0.0 || object.depth <= 0.0 {
        return Err("Stage object size must be greater than zero".to_string());
    }
    object.x = object.x.clamp(-1_000.0, 1_000.0);
    object.z = object.z.clamp(-1_000.0, 1_000.0);
    object.width = object.width.clamp(0.05, 1_000.0);
    object.depth = object.depth.clamp(0.05, 1_000.0);
    object.rotation_deg = object.rotation_deg.clamp(-360.0, 360.0);
    object.color = match object.color {
        Some(color) if color.trim().is_empty() => None,
        Some(color) => Some(normalize_stage_object_color(color)?),
        None => None,
    };
    Ok(object)
}

fn normalize_stage_object_color(color: String) -> Result<String, String> {
    let trimmed = color.trim();
    let hex = trimmed
        .strip_prefix('#')
        .ok_or_else(|| "Stage object color must be a #rrggbb value".to_string())?;
    if hex.len() == 6 && hex.chars().all(|character| character.is_ascii_hexdigit()) {
        Ok(format!("#{hex}"))
    } else {
        Err("Stage object color must be a #rrggbb value".to_string())
    }
}

fn validate_project_stage_objects(objects: &[StageObjectSummary]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for object in objects {
        let normalized = normalize_stage_object(object.clone())?;
        if !ids.insert(normalized.id) {
            return Err(format!(
                "Project contains duplicate stage object id {}",
                normalized.id
            ));
        }
    }
    Ok(())
}

fn normalize_stage_map_preset_label(label: String) -> Result<String, String> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err("Stage map preset label is required".to_string());
    }
    Ok(trimmed.to_string())
}

fn validate_stage_map_preset_file(file: &StageMapPresetFile) -> Result<(), String> {
    if file.version != 1 {
        return Err(format!(
            "Unsupported stage map preset version {}",
            file.version
        ));
    }
    validate_app_name("stage map", &file.app)?;
    normalize_stage_map_preset_label(file.preset.label.clone())?;
    validate_stage_map_config(&file.preset.config)?;
    if let Some(stage_objects) = &file.preset.stage_objects {
        validate_project_stage_objects(stage_objects)?;
    }
    Ok(())
}

fn validate_project_stage_map_presets(presets: &[StageMapPresetSummary]) -> Result<(), String> {
    let mut labels = HashSet::new();
    for preset in presets {
        let label = normalize_stage_map_preset_label(preset.label.clone())?;
        if !labels.insert(label.clone()) {
            return Err(format!(
                "Project contains duplicate stage map preset label {label}"
            ));
        }
        validate_stage_map_config(&preset.config)?;
        if let Some(stage_objects) = &preset.stage_objects {
            validate_project_stage_objects(stage_objects)?;
        }
    }
    Ok(())
}

fn validate_patch_footprint(
    request: &PatchFixtureRequest,
    profile: &FixtureProfileSummary,
) -> Result<(), String> {
    let footprint = gdtf::profile_mode_footprint(profile, request.mode_name.as_deref())
        .ok_or_else(|| "Selected GDTF mode has no DMX channel offsets".to_string())?;
    let end_address = request.address.saturating_add(footprint).saturating_sub(1);
    if end_address > 512 {
        return Err(format!(
            "Fixture footprint exceeds DMX universe: start {}, footprint {}ch, end {}",
            request.address, footprint, end_address
        ));
    }
    Ok(())
}

fn prepare_fixture_patches(
    state: &State<'_, AppState>,
    requests: Vec<PatchFixtureRequest>,
) -> Result<Vec<PreparedFixturePatch>, String> {
    if requests.is_empty() {
        return Err("At least one fixture patch request is required".to_string());
    }
    if requests.len() > 256 {
        return Err("Cannot patch more than 256 fixtures at once".to_string());
    }

    let mut prepared = Vec::with_capacity(requests.len());
    for mut request in requests {
        request.group_ids = normalize_group_ids(request.group_ids)?;
        validate_patch_request(&request)?;
        let profile = load_patch_profile(state, &request.profile_path)?;
        validate_patch_footprint(&request, &profile)?;
        prepared.push(PreparedFixturePatch { request, profile });
    }
    validate_prepared_patch_conflicts(&prepared, &state.engine.snapshot().fixtures)?;
    Ok(prepared)
}

fn validate_patch_address_conflicts(
    request: &PatchFixtureRequest,
    profile: &FixtureProfileSummary,
    fixtures: &[PatchedFixtureSummary],
) -> Result<(), String> {
    let Some(new_footprint) = gdtf::profile_mode_footprint(profile, request.mode_name.as_deref())
    else {
        return Ok(());
    };
    let Some(new_range) = dmx_range(request.address, new_footprint) else {
        return Ok(());
    };

    for fixture in fixtures
        .iter()
        .filter(|fixture| fixture.universe == request.universe)
    {
        let Some(existing_footprint) = fixture_controls_footprint(&fixture.controls) else {
            continue;
        };
        let Some(existing_range) = dmx_range(fixture.address, existing_footprint) else {
            continue;
        };
        if ranges_overlap(new_range, existing_range) {
            return Err(format!(
                "DMX address conflict in universe {}: new fixture {}-{} overlaps fixture {} '{}' at {}-{}",
                request.universe,
                new_range.0,
                new_range.1,
                fixture.id,
                fixture.label,
                existing_range.0,
                existing_range.1
            ));
        }
    }

    Ok(())
}

fn validate_prepared_patch_conflicts(
    prepared: &[PreparedFixturePatch],
    fixtures: &[PatchedFixtureSummary],
) -> Result<(), String> {
    let mut occupied = Vec::new();
    for fixture in fixtures {
        if let Some(footprint) = fixture_controls_footprint(&fixture.controls) {
            if let Some(range) = dmx_range(fixture.address, footprint) {
                occupied.push((
                    fixture.universe,
                    range,
                    format!("fixture {} '{}'", fixture.id, fixture.label),
                ));
            }
        }
    }

    for (index, prepared_patch) in prepared.iter().enumerate() {
        let footprint = gdtf::profile_mode_footprint(
            &prepared_patch.profile,
            prepared_patch.request.mode_name.as_deref(),
        )
        .ok_or_else(|| "Selected GDTF mode has no DMX channel offsets".to_string())?;
        let Some(range) = dmx_range(prepared_patch.request.address, footprint) else {
            continue;
        };
        for (occupied_universe, occupied_range, occupied_label) in &occupied {
            if *occupied_universe == prepared_patch.request.universe
                && ranges_overlap(range, *occupied_range)
            {
                return Err(format!(
                    "DMX address conflict in universe {}: new fixture {} '{}' at {}-{} overlaps {} at {}-{}",
                    prepared_patch.request.universe,
                    index + 1,
                    prepared_patch.request.label,
                    range.0,
                    range.1,
                    occupied_label,
                    occupied_range.0,
                    occupied_range.1
                ));
            }
        }
        occupied.push((
            prepared_patch.request.universe,
            range,
            format!(
                "new fixture {} '{}'",
                index + 1,
                prepared_patch.request.label
            ),
        ));
    }

    Ok(())
}

fn validate_fixture_patch_update(
    snapshot: &EngineSnapshot,
    fixture_id: FixtureId,
    label: &str,
    universe: u16,
    address: u16,
) -> Result<(), String> {
    if label.trim().is_empty() {
        return Err("Fixture label is required".to_string());
    }
    if address == 0 || address > 512 {
        return Err("DMX address must be between 1 and 512".to_string());
    }
    let fixture = snapshot
        .fixtures
        .iter()
        .find(|fixture| fixture.id == fixture_id)
        .ok_or_else(|| format!("Fixture {fixture_id} was not found"))?;
    let footprint = fixture_controls_footprint(&fixture.controls)
        .ok_or_else(|| "Fixture has no DMX channel offsets".to_string())?;
    let Some(new_range) = dmx_range(address, footprint) else {
        return Err("Fixture has no DMX channel offsets".to_string());
    };
    if new_range.1 > 512 {
        return Err(format!(
            "Fixture footprint exceeds DMX universe: start {}, footprint {}ch, end {}",
            address, footprint, new_range.1
        ));
    }

    for existing in snapshot
        .fixtures
        .iter()
        .filter(|existing| existing.id != fixture_id && existing.universe == universe)
    {
        let Some(existing_footprint) = fixture_controls_footprint(&existing.controls) else {
            continue;
        };
        let Some(existing_range) = dmx_range(existing.address, existing_footprint) else {
            continue;
        };
        if ranges_overlap(new_range, existing_range) {
            return Err(format!(
                "DMX address conflict in universe {}: fixture {}-{} overlaps fixture {} '{}' at {}-{}",
                universe,
                new_range.0,
                new_range.1,
                existing.id,
                existing.label,
                existing_range.0,
                existing_range.1
            ));
        }
    }

    Ok(())
}

fn fixture_controls_footprint(controls: &[AttributeControl]) -> Option<u16> {
    controls
        .iter()
        .flat_map(|control| control.offsets.iter().copied())
        .max()
}

fn dmx_range(start_address: u16, footprint: u16) -> Option<(u16, u16)> {
    if start_address == 0 || footprint == 0 {
        return None;
    }
    Some((
        start_address,
        start_address.saturating_add(footprint).saturating_sub(1),
    ))
}

fn ranges_overlap(first: (u16, u16), second: (u16, u16)) -> bool {
    first.0 <= second.1 && second.0 <= first.1
}

fn validate_lfo_effect_request(request: &LfoEffectRequest) -> Result<(), String> {
    if request.label.trim().is_empty() {
        return Err("Effect label is required".to_string());
    }
    let has_light_targets = !request.fixture_ids.is_empty() || !request.target_group_ids.is_empty();
    let has_video_targets = !request.video_targets.is_empty();
    if !has_light_targets && !has_video_targets {
        return Err("At least one fixture, group, or video layer must be targeted".to_string());
    }
    if has_light_targets && request.attribute.trim().is_empty() {
        return Err("Effect attribute is required".to_string());
    }
    if request.period_ms < 10 {
        return Err("LFO period must be at least 10 ms".to_string());
    }
    if let Some(clock_sync) = request.clock_sync {
        if !clock_sync.beats.is_finite() || clock_sync.beats <= 0.0 {
            return Err("LFO clock sync beats must be finite and greater than 0".to_string());
        }
    }
    if !request.phase.is_finite() {
        return Err("LFO phase must be finite".to_string());
    }
    validate_group_ids(&request.target_group_ids)?;
    validate_video_effect_targets(&request.video_targets)?;
    Ok(())
}

fn validate_position_wave_effect_request(
    request: &PositionWaveEffectRequest,
) -> Result<(), String> {
    if request.label.trim().is_empty() {
        return Err("Effect label is required".to_string());
    }
    let has_light_targets = !request.fixture_ids.is_empty() || !request.target_group_ids.is_empty();
    let has_video_targets = !request.video_targets.is_empty();
    if !has_light_targets && !has_video_targets {
        return Err("At least one fixture, group, or video layer must be targeted".to_string());
    }
    if has_light_targets && request.attribute.trim().is_empty() {
        return Err("Effect attribute is required".to_string());
    }
    if !request.speed.is_finite() {
        return Err("Wave speed must be finite".to_string());
    }
    if !vec3_is_finite(request.origin) || !vec3_is_finite(request.direction) {
        return Err("Wave origin and direction values must be finite".to_string());
    }
    if !request.wavelength.is_finite() || request.wavelength.abs() < 0.001 {
        return Err("Wave wavelength must be at least 0.001".to_string());
    }
    if let Some(clock_sync) = request.clock_sync {
        if !clock_sync.beats.is_finite() || clock_sync.beats <= 0.0 {
            return Err("Wave clock sync beats must be finite and greater than 0".to_string());
        }
    }
    if !request.phase.is_finite() {
        return Err("Wave phase must be finite".to_string());
    }
    validate_group_ids(&request.target_group_ids)?;
    validate_video_effect_targets(&request.video_targets)?;
    Ok(())
}

fn vec3_is_finite(value: Vec3) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}

fn validate_video_effect_targets(targets: &[VideoEffectTarget]) -> Result<(), String> {
    for target in targets {
        if target.layer_ids.is_empty() {
            return Err("Video effect target requires at least one layer".to_string());
        }
        if !target.low.is_finite() || !target.high.is_finite() {
            return Err("Video effect low/high values must be finite".to_string());
        }
        if let Some(position) = target.position {
            if !position.x.is_finite() || !position.y.is_finite() || !position.z.is_finite() {
                return Err("Video effect target position values must be finite".to_string());
            }
        }
    }
    Ok(())
}

fn validate_video_layer_ids(
    snapshot: &EngineSnapshot,
    layer_ids: &[VideoLayerId],
) -> Result<(), String> {
    for layer_id in layer_ids {
        if !snapshot
            .video
            .layers
            .iter()
            .any(|layer| layer.id == *layer_id)
        {
            return Err(format!("Video layer {layer_id} was not found"));
        }
    }
    Ok(())
}

fn validate_video_composition_exists(
    snapshot: &EngineSnapshot,
    composition_id: CompositionId,
) -> Result<(), String> {
    if snapshot
        .video
        .compositions
        .iter()
        .any(|composition| composition.id == composition_id)
    {
        Ok(())
    } else {
        Err(format!("Video composition {composition_id} was not found"))
    }
}

fn validate_editable_video_composition(
    snapshot: &EngineSnapshot,
    composition_id: CompositionId,
) -> Result<(), String> {
    if composition_id == 1 {
        return Err("Main video composition cannot be edited directly".to_string());
    }
    validate_video_composition_exists(snapshot, composition_id)
}

fn validate_video_output_exists(
    snapshot: &EngineSnapshot,
    output_id: VideoOutputId,
) -> Result<(), String> {
    if snapshot
        .video
        .outputs
        .iter()
        .any(|output| output.id == output_id)
    {
        Ok(())
    } else {
        Err(format!("Video output {output_id} was not found"))
    }
}

fn normalize_video_output_config(
    label: String,
    kind: VideoOutputKind,
    width: u32,
    height: u32,
    fullscreen: bool,
    monitor_id: Option<u32>,
    endpoint_name: Option<String>,
) -> Result<NormalizedVideoOutputConfig, String> {
    let label = label.trim().to_string();
    if label.is_empty() {
        return Err("Video output label is required".to_string());
    }
    if width == 0 || height == 0 {
        return Err("Video output width and height must be greater than 0".to_string());
    }

    let is_display = matches!(kind, VideoOutputKind::Display);
    let endpoint_name = endpoint_name.and_then(|name| {
        let trimmed = name.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    });

    Ok(NormalizedVideoOutputConfig {
        label,
        kind,
        width,
        height,
        fullscreen: is_display && fullscreen,
        monitor_id: if is_display { monitor_id } else { None },
        endpoint_name: if is_display { None } else { endpoint_name },
    })
}

fn ensure_video_input_backend_available(kind: &VideoSourceKind) -> Result<(), String> {
    let Some((backend_id, feature_label)) = video_input_backend(kind) else {
        return Ok(());
    };
    let status = video::video_runtime_status();
    validate_video_backend_available(&status, backend_id, feature_label)
}

fn ensure_video_output_backend_available(kind: &VideoOutputKind) -> Result<(), String> {
    let Some((backend_id, feature_label)) = video_output_backend(kind) else {
        return Ok(());
    };
    let status = video::video_runtime_status();
    validate_video_backend_available(&status, backend_id, feature_label)
}

fn video_input_backend(kind: &VideoSourceKind) -> Option<(&'static str, &'static str)> {
    match kind {
        VideoSourceKind::Ndi => Some(("ndi", "NDI input")),
        VideoSourceKind::Spout => Some(("spout", "Spout input")),
        VideoSourceKind::Syphon => Some(("syphon", "Syphon input")),
        VideoSourceKind::File | VideoSourceKind::StillImage => None,
    }
}

fn video_output_backend(kind: &VideoOutputKind) -> Option<(&'static str, &'static str)> {
    match kind {
        VideoOutputKind::NdiSender => Some(("ndi", "NDI output")),
        VideoOutputKind::SpoutSender => Some(("spout", "Spout output")),
        VideoOutputKind::SyphonServer => Some(("syphon", "Syphon output")),
        VideoOutputKind::Display => None,
    }
}

fn validate_video_backend_available(
    status: &VideoRuntimeStatus,
    backend_id: &str,
    feature_label: &str,
) -> Result<(), String> {
    let backend = status
        .backends
        .iter()
        .find(|backend| backend.id == backend_id)
        .ok_or_else(|| format!("{feature_label} backend status is unavailable"))?;
    match backend.state {
        VideoBackendState::Available => Ok(()),
        VideoBackendState::Missing | VideoBackendState::NotBuilt => Err(format!(
            "{feature_label} is unavailable: {} ({})",
            backend.label, backend.detail
        )),
    }
}

fn validate_video_output_mapping(mapping: &VideoOutputMapping) -> Result<(), String> {
    let values = [
        mapping.stage_x,
        mapping.stage_y,
        mapping.stage_z,
        mapping.offset_x,
        mapping.offset_y,
        mapping.scale_x,
        mapping.scale_y,
        mapping.rotation_deg,
        mapping.aspect_ratio,
        mapping.lens_distortion,
        mapping.keystone_x,
        mapping.keystone_y,
        mapping.corner_top_left_x,
        mapping.corner_top_left_y,
        mapping.corner_top_right_x,
        mapping.corner_top_right_y,
        mapping.corner_bottom_right_x,
        mapping.corner_bottom_right_y,
        mapping.corner_bottom_left_x,
        mapping.corner_bottom_left_y,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err("Video output mapping values must be finite".to_string());
    }
    if mapping.scale_x <= 0.0 || mapping.scale_y <= 0.0 {
        return Err("Video output mapping scale must be greater than 0".to_string());
    }
    if mapping.aspect_ratio <= 0.0 {
        return Err("Video output mapping aspect ratio must be greater than 0".to_string());
    }
    Ok(())
}

fn normalize_video_output_mapping_field(field: String) -> Result<String, String> {
    let trimmed = field.trim();
    if trimmed.is_empty() {
        return Err("Video output mapping field is required".to_string());
    }
    canonical_video_output_mapping_field(trimmed)
        .map(str::to_string)
        .ok_or_else(|| format!("Video output mapping field '{trimmed}' is not supported"))
}

fn normalize_video_output_mapping_preset_label(label: String) -> Result<String, String> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err("Video output mapping preset label is required".to_string());
    }
    Ok(trimmed.to_string())
}

fn validate_video_output_mapping_preset_file(
    file: &VideoOutputMappingPresetFile,
) -> Result<(), String> {
    if file.version != 1 {
        return Err(format!(
            "Unsupported projector map preset version {}",
            file.version
        ));
    }
    validate_app_name("projector map", &file.app)?;
    normalize_video_output_mapping_preset_label(file.preset.label.clone())?;
    validate_video_output_mapping(&file.preset.mapping)
}

fn validate_group_ids(group_ids: &[String]) -> Result<(), String> {
    for group_id in group_ids {
        normalize_group_id(group_id)?;
    }
    Ok(())
}

fn normalize_control_group_id(group_id: String) -> Result<String, String> {
    normalize_group_id(&group_id)
}

fn normalize_attribute_name(attribute: String) -> Result<String, String> {
    let trimmed = attribute.trim();
    if trimmed.is_empty() {
        return Err("Attribute is required".to_string());
    }
    Ok(trimmed.to_string())
}

fn normalize_video_layer_label(label: String) -> Result<String, String> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err("Video layer label is required".to_string());
    }
    Ok(trimmed.to_string())
}

fn validate_existing_file_path(path: String, label: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} path is required"));
    }
    let metadata =
        fs::metadata(trimmed).map_err(|_| format!("{label} path was not found: {trimmed}"))?;
    if !metadata.is_file() {
        return Err(format!("{label} path must be a file: {trimmed}"));
    }
    Ok(trimmed.to_string())
}

fn validate_video_cue_point_color(color: &str) -> Result<(), String> {
    let trimmed = color.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("Video cue point color must be #RRGGBB".to_string())
    }
}

fn video_source_file_dialog_filter(
    kind: &VideoSourceKind,
) -> Result<(&'static str, &'static [&'static str]), String> {
    match kind {
        VideoSourceKind::File => Ok(("Video Files", VIDEO_FILE_EXTENSIONS)),
        VideoSourceKind::StillImage => Ok(("Still Images", STILL_IMAGE_EXTENSIONS)),
        VideoSourceKind::Ndi | VideoSourceKind::Spout | VideoSourceKind::Syphon => {
            Err("Only file and still-image sources can browse local media".to_string())
        }
    }
}

fn normalize_group_ids(group_ids: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();
    for group_id in group_ids {
        let group_id = normalize_group_id(&group_id)?;
        if seen.insert(group_id.clone()) {
            normalized.push(group_id);
        }
    }
    Ok(normalized)
}

fn normalize_group_id(group_id: &str) -> Result<String, String> {
    let trimmed = group_id.trim();
    if trimmed.is_empty() {
        return Err("Group IDs must not be empty".to_string());
    }
    let segments = trimmed.split('/').map(str::trim).collect::<Vec<_>>();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err("Group path segments must not be empty".to_string());
    }
    Ok(segments.join("/"))
}

fn group_matches(fixture_group_id: &str, requested_group_id: &str) -> bool {
    let Ok(fixture_group_id) = normalize_group_id(fixture_group_id) else {
        return false;
    };
    let Ok(requested_group_id) = normalize_group_id(requested_group_id) else {
        return false;
    };
    fixture_group_id == requested_group_id
        || fixture_group_id
            .strip_prefix(&requested_group_id)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn validate_gdtf_share_credentials(user: &str, password: &str) -> Result<(), String> {
    if user.trim().is_empty() {
        return Err("GDTF Share user is required".to_string());
    }
    if password.is_empty() {
        return Err("GDTF Share password is required".to_string());
    }
    Ok(())
}

fn gdtf_share_cookie_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "rayard-gdtf-share-{}-{}.cookies",
        std::process::id(),
        current_unix_ms()
    ))
}

fn gdtf_share_api_url(endpoint: &str) -> String {
    format!("https://gdtf-share.com/apis/public/{endpoint}")
}

fn run_curl(args: &[&OsStr]) -> Result<Vec<u8>, String> {
    let output = Command::new(curl_binary_name())
        .args(args)
        .output()
        .map_err(|error| format!("Failed to start curl: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            output
                .status
                .code()
                .map(|code| format!("exit code {code}"))
                .unwrap_or_else(|| "terminated".to_string())
        };
        return Err(format!("curl failed: {detail}"));
    }
    Ok(output.stdout)
}

fn curl_json_post(url: &str, body: &Value, cookie_path: Option<&Path>) -> Result<Value, String> {
    let body = serde_json::to_string(body).map_err(|error| error.to_string())?;
    let mut args: Vec<&OsStr> = vec![
        OsStr::new("--silent"),
        OsStr::new("--show-error"),
        OsStr::new("--max-time"),
        OsStr::new("60"),
        OsStr::new("-H"),
        OsStr::new("Content-Type: application/json"),
    ];
    if let Some(cookie_path) = cookie_path {
        args.push(OsStr::new("-b"));
        args.push(cookie_path.as_os_str());
        args.push(OsStr::new("-c"));
        args.push(cookie_path.as_os_str());
    }
    args.push(OsStr::new("--data"));
    args.push(OsStr::new(body.as_str()));
    args.push(OsStr::new(url));

    parse_curl_json_response(run_curl(&args)?)
}

fn curl_json_get(url: &str, cookie_path: &Path) -> Result<Value, String> {
    let args: Vec<&OsStr> = vec![
        OsStr::new("--silent"),
        OsStr::new("--show-error"),
        OsStr::new("--max-time"),
        OsStr::new("60"),
        OsStr::new("-b"),
        cookie_path.as_os_str(),
        OsStr::new("-c"),
        cookie_path.as_os_str(),
        OsStr::new(url),
    ];
    parse_curl_json_response(run_curl(&args)?)
}

fn parse_curl_json_response(bytes: Vec<u8>) -> Result<Value, String> {
    let text =
        String::from_utf8(bytes).map_err(|error| format!("Response was not UTF-8: {error}"))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("GDTF Share returned an empty response".to_string());
    }
    let value: Value = serde_json::from_str(trimmed).map_err(|error| {
        let snippet = trimmed.chars().take(220).collect::<String>();
        format!("GDTF Share returned non-JSON data: {error}: {snippet}")
    })?;
    if value.get("result").and_then(Value::as_bool) == Some(false) {
        return Err(value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("GDTF Share request failed")
            .to_string());
    }
    Ok(value)
}

fn login_gdtf_share(user: &str, password: &str, cookie_path: &Path) -> Result<(), String> {
    let response = curl_json_post(
        &gdtf_share_api_url("login.php"),
        &json!({
            "user": user.trim(),
            "password": password,
        }),
        Some(cookie_path),
    )?;
    if response.get("result").and_then(Value::as_bool) == Some(false) {
        return Err(response
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("GDTF Share login failed")
            .to_string());
    }
    Ok(())
}

fn fetch_gdtf_share_catalog(cookie_path: &Path) -> Result<Value, String> {
    let attempts = [
        ("getList.php", None),
        ("getList.php", Some(json!({}))),
        ("getDiff.php", Some(json!({ "lastModified": 0 }))),
        ("getDiff.php", Some(json!({ "from": 0 }))),
        ("getDiff.php", Some(json!({}))),
    ];
    let mut last_error = String::new();
    for (endpoint, body) in attempts {
        let result = if let Some(body) = body {
            curl_json_post(&gdtf_share_api_url(endpoint), &body, Some(cookie_path))
        } else {
            curl_json_get(&gdtf_share_api_url(endpoint), cookie_path)
        };
        match result {
            Ok(value) => return Ok(value),
            Err(error) => last_error = error,
        }
    }
    Err(if last_error.is_empty() {
        "GDTF Share catalog request failed".to_string()
    } else {
        last_error
    })
}

fn find_string_field<'a>(map: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| map.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn find_u64_field(map: &Map<String, Value>, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        let value = map.get(*key)?;
        value
            .as_u64()
            .or_else(|| value.as_str()?.trim().parse::<u64>().ok())
    })
}

fn gdtf_share_modes_from_value(value: Option<&Value>) -> Vec<GdtfShareModeSummary> {
    let Some(Value::Array(modes)) = value else {
        return Vec::new();
    };
    modes
        .iter()
        .filter_map(|mode| {
            let map = mode.as_object()?;
            let name = find_string_field(map, &["name", "mode", "dmxMode", "dmx_mode"])
                .unwrap_or("Mode")
                .to_string();
            let dmx_footprint = find_u64_field(
                map,
                &["dmxfootprint", "dmxFootprint", "footprint", "dmx_footprint"],
            )
            .and_then(|value| u16::try_from(value).ok());
            Some(GdtfShareModeSummary {
                name,
                dmx_footprint,
            })
        })
        .collect()
}

fn gdtf_share_fixture_from_map(map: &Map<String, Value>) -> Option<GdtfShareFixtureSummary> {
    let manufacturer = find_string_field(map, &["manufacturer", "manufacture", "maker", "brand"])?;
    let fixture = find_string_field(map, &["fixture", "name", "fixtureName", "fixture_name"])?;
    let revision = find_string_field(map, &["revision", "revisionName", "revision_name"])
        .or_else(|| find_string_field(map, &["file", "filename", "fileName"]))
        .unwrap_or("latest");
    Some(GdtfShareFixtureSummary {
        rid: find_u64_field(map, &["rid", "id", "revisionId", "revision_id"]),
        uuid: find_string_field(map, &["uuid", "guid", "fixtureTypeId", "fixture_type_id"])
            .map(ToString::to_string),
        manufacturer: manufacturer.to_string(),
        fixture: fixture.to_string(),
        revision: revision.to_string(),
        uploader: find_string_field(map, &["uploader", "uploadedBy", "uploaded_by"])
            .map(ToString::to_string),
        rating: find_string_field(map, &["rating"]).map(ToString::to_string),
        version: find_string_field(map, &["version", "gdtfVersion", "gdtf_version"])
            .map(ToString::to_string),
        creator: find_string_field(map, &["creator", "author"]).map(ToString::to_string),
        filesize: find_u64_field(map, &["filesize", "fileSize", "size"]),
        modes: gdtf_share_modes_from_value(map.get("modes")),
    })
}

fn collect_gdtf_share_fixture_maps(value: &Value, fixtures: &mut Vec<GdtfShareFixtureSummary>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_gdtf_share_fixture_maps(item, fixtures);
            }
        }
        Value::Object(map) => {
            if let Some(fixture) = gdtf_share_fixture_from_map(map) {
                fixtures.push(fixture);
                return;
            }
            for nested in map.values() {
                collect_gdtf_share_fixture_maps(nested, fixtures);
            }
        }
        _ => {}
    }
}

fn extract_gdtf_share_fixtures(value: &Value) -> Vec<GdtfShareFixtureSummary> {
    let mut fixtures = Vec::new();
    collect_gdtf_share_fixture_maps(value, &mut fixtures);
    let mut seen = HashSet::new();
    fixtures.retain(|fixture| {
        let key = format!(
            "{}|{}|{}|{:?}|{:?}",
            fixture.manufacturer.to_ascii_lowercase(),
            fixture.fixture.to_ascii_lowercase(),
            fixture.revision.to_ascii_lowercase(),
            fixture.rid,
            fixture.uuid
        );
        seen.insert(key)
    });
    fixtures
}

fn gdtf_share_filter_matches(fixture: &GdtfShareFixtureSummary, needle: &str) -> bool {
    if needle.trim().is_empty() {
        return true;
    }
    let needle = needle.trim().to_ascii_lowercase();
    let field_match = [
        fixture.manufacturer.as_str(),
        fixture.fixture.as_str(),
        fixture.revision.as_str(),
        fixture.uploader.as_deref().unwrap_or_default(),
        fixture.creator.as_deref().unwrap_or_default(),
        fixture.version.as_deref().unwrap_or_default(),
    ]
    .iter()
    .any(|value| value.to_ascii_lowercase().contains(&needle));
    field_match
        || fixture.modes.iter().any(|mode| {
            mode.name.to_ascii_lowercase().contains(&needle)
                || mode
                    .dmx_footprint
                    .is_some_and(|footprint| format!("{footprint}ch").contains(&needle))
        })
}

fn filter_gdtf_share_results(
    mut fixtures: Vec<GdtfShareFixtureSummary>,
    manufacturer: &str,
    fixture: &str,
    query: &str,
    limit: usize,
) -> Vec<GdtfShareFixtureSummary> {
    fixtures.retain(|candidate| {
        gdtf_share_filter_matches(candidate, manufacturer)
            && gdtf_share_filter_matches(candidate, fixture)
            && gdtf_share_filter_matches(candidate, query)
    });
    fixtures.sort_by(|left, right| {
        left.manufacturer
            .cmp(&right.manufacturer)
            .then_with(|| left.fixture.cmp(&right.fixture))
            .then_with(|| left.revision.cmp(&right.revision))
    });
    fixtures.truncate(limit.clamp(1, 200));
    fixtures
}

fn gdtf_share_download_file_name(request: &GdtfShareDownloadRequest) -> String {
    let label = [
        request.manufacturer.as_str(),
        request.fixture.as_str(),
        request.revision.as_str(),
    ]
    .iter()
    .map(|value| value.trim())
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join("-");
    format!("{}.gdtf", safe_file_stem(&label))
}

fn download_gdtf_share_file(
    request: &GdtfShareDownloadRequest,
    cookie_path: &Path,
    path: &Path,
) -> Result<(), String> {
    let body = serde_json::to_string(&json!({
        "rid": request.rid,
        "id": request.rid,
        "uuid": request.uuid.as_deref().unwrap_or_default().trim(),
        "manufacturer": request.manufacturer.trim(),
        "fixture": request.fixture.trim(),
        "revision": request.revision.trim(),
    }))
    .map_err(|error| error.to_string())?;
    let url = gdtf_share_api_url("downloadFile.php");
    let args: Vec<&OsStr> = vec![
        OsStr::new("--silent"),
        OsStr::new("--show-error"),
        OsStr::new("--max-time"),
        OsStr::new("120"),
        OsStr::new("-H"),
        OsStr::new("Content-Type: application/json"),
        OsStr::new("-b"),
        cookie_path.as_os_str(),
        OsStr::new("--data"),
        OsStr::new(body.as_str()),
        OsStr::new("-o"),
        path.as_os_str(),
        OsStr::new(url.as_str()),
    ];
    if let Err(error) = run_curl(&args) {
        let _ = fs::remove_file(path);
        return Err(error);
    }
    validate_gdtf_share_download_payload(path).map_err(|error| {
        let _ = fs::remove_file(path);
        error
    })?;
    Ok(())
}

fn validate_gdtf_share_download_payload(path: &Path) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("GDTF Share download was not saved: {error}"))?;
    if metadata.len() == 0 {
        return Err("GDTF Share downloaded an empty file".to_string());
    }
    let mut file = fs::File::open(path)
        .map_err(|error| format!("Failed to inspect GDTF Share download: {error}"))?;
    let mut buffer = vec![0u8; usize::min(metadata.len() as usize, 4096)];
    let read = file
        .read(&mut buffer)
        .map_err(|error| format!("Failed to inspect GDTF Share download: {error}"))?;
    buffer.truncate(read);
    let trimmed_start = buffer
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(0);
    let payload = &buffer[trimmed_start..];
    if payload.starts_with(b"PK") {
        return Ok(());
    }
    if payload
        .first()
        .is_some_and(|byte| *byte == b'{' || *byte == b'[')
    {
        let text = String::from_utf8_lossy(payload);
        let value = serde_json::from_str::<Value>(&text).ok();
        if let Some(error) = value
            .as_ref()
            .and_then(|value| value.get("error"))
            .and_then(Value::as_str)
        {
            return Err(format!("GDTF Share download failed: {error}"));
        }
        return Err("GDTF Share returned JSON instead of a .gdtf archive".to_string());
    }
    if payload.first().is_some_and(|byte| *byte == b'<') {
        let snippet = String::from_utf8_lossy(payload)
            .chars()
            .take(180)
            .collect::<String>();
        return Err(format!(
            "GDTF Share returned HTML instead of a .gdtf archive: {snippet}"
        ));
    }
    Ok(())
}

fn validate_gdtf_download_url(url: &str) -> Result<&str, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("GDTF download URL is required".to_string());
    }
    let lower = trimmed.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err("GDTF download URL must start with http:// or https://".to_string());
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err("GDTF download URL must not contain whitespace".to_string());
    }
    Ok(trimmed)
}

fn gdtf_download_file_name_from_url(url: &str) -> String {
    let path_part = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .trim_end_matches('/');
    let raw_name = path_part
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("fixture.gdtf");
    let decoded_name = percent_decode_url_path_segment(raw_name).replace(['/', '\\'], "_");
    let stem = Path::new(&decoded_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("fixture");
    format!("{}.gdtf", safe_file_stem(stem))
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn percent_decode_url_path_segment(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = &value[index + 1..index + 3];
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                decoded.push(byte);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(decoded).unwrap_or_else(|_| value.to_string())
}

fn safe_file_stem(label: &str) -> String {
    let safe = label
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if safe.is_empty() {
        "fixture".to_string()
    } else {
        safe
    }
}

fn current_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn engine_telemetry_report_from_snapshot(
    snapshot: &EngineSnapshot,
    captured_at_unix_ms: u128,
) -> EngineTelemetryReport {
    let enabled_dmx_output_count = snapshot
        .dmx_outputs
        .iter()
        .filter(|output| output.enabled)
        .count();
    EngineTelemetryReport {
        version: 1,
        captured_at_unix_ms,
        fixture_count: snapshot.fixtures.len(),
        cue_count: snapshot.cues.len(),
        effect_count: snapshot.effects.len(),
        node_graph_count: snapshot.node_graphs.len(),
        video_layer_count: snapshot.video.layers.len(),
        video_output_count: snapshot.video.outputs.len(),
        dmx_output_count: snapshot.dmx_outputs.len(),
        enabled_dmx_output_count,
        dmx_preview_universe_count: snapshot.dmx_previews.len(),
        clock: snapshot.clock.clone(),
        primary_output: snapshot.output.clone(),
        dmx_outputs: snapshot.dmx_outputs.clone(),
        budget: engine_telemetry_budget_report(&snapshot.telemetry, enabled_dmx_output_count),
        telemetry: snapshot.telemetry.clone(),
    }
}

fn engine_telemetry_budget_report(
    telemetry: &EngineTelemetry,
    enabled_dmx_output_count: usize,
) -> EngineTelemetryBudgetReport {
    let checks = vec![
        telemetry_micro_budget_check(
            "tick_jitter_p99",
            telemetry.tick_jitter_p99_us,
            TELEMETRY_TICK_JITTER_P99_TARGET_US,
            telemetry.tick_jitter_samples,
            3,
        ),
        telemetry_micro_budget_check(
            "command_queue_p99",
            telemetry.command_queue_latency_p99_us,
            TELEMETRY_COMMAND_QUEUE_P99_TARGET_US,
            telemetry.command_queue_latency_samples,
            1,
        ),
        telemetry_micro_budget_check(
            "command_to_dmx_tick_p99",
            telemetry.command_to_dmx_tick_latency_p99_us,
            TELEMETRY_COMMAND_TO_DMX_P99_TARGET_US,
            telemetry.command_to_dmx_tick_latency_samples,
            1,
        ),
        telemetry_dmx_send_check(telemetry, enabled_dmx_output_count),
        telemetry_dmx_interval_check(telemetry, enabled_dmx_output_count),
    ];

    EngineTelemetryBudgetReport {
        overall: telemetry_overall_budget_status(&checks),
        target_dmx_frame_rate_hz: TELEMETRY_DMX_TARGET_FRAME_RATE_HZ,
        target_tick_interval_us: TELEMETRY_DMX_TARGET_TICK_INTERVAL_US,
        tick_jitter_p99_target_us: TELEMETRY_TICK_JITTER_P99_TARGET_US,
        command_queue_p99_target_us: TELEMETRY_COMMAND_QUEUE_P99_TARGET_US,
        command_to_dmx_p99_target_us: TELEMETRY_COMMAND_TO_DMX_P99_TARGET_US,
        dmx_send_interval_tolerance_us: TELEMETRY_DMX_SEND_INTERVAL_TOLERANCE_US,
        checks,
    }
}

fn telemetry_micro_budget_check(
    name: &str,
    measured_us: u64,
    target_us: u64,
    samples: u64,
    minimum_samples: u64,
) -> TelemetryBudgetCheck {
    let status = if samples < minimum_samples {
        TelemetryBudgetStatus::InsufficientSamples
    } else if measured_us <= target_us {
        TelemetryBudgetStatus::Pass
    } else {
        TelemetryBudgetStatus::Fail
    };
    let detail = if samples < minimum_samples {
        format!("Needs at least {minimum_samples} sample(s); captured {samples}.")
    } else {
        format!("{measured_us}us measured against {target_us}us target.")
    };
    TelemetryBudgetCheck {
        name: name.to_string(),
        status,
        measured_us: Some(measured_us),
        target_us: Some(target_us),
        samples,
        detail,
    }
}

fn telemetry_dmx_send_check(
    telemetry: &EngineTelemetry,
    enabled_dmx_output_count: usize,
) -> TelemetryBudgetCheck {
    let (status, detail) = if enabled_dmx_output_count == 0 {
        (
            TelemetryBudgetStatus::Idle,
            "No enabled DMX output routes.".to_string(),
        )
    } else if telemetry.last_dmx_send_failure_count > 0
        || telemetry.total_dmx_send_failure_count > 0
    {
        (
            TelemetryBudgetStatus::Fail,
            format!(
                "{} failure(s) in the last frame, {} cumulative failure(s).",
                telemetry.last_dmx_send_failure_count, telemetry.total_dmx_send_failure_count
            ),
        )
    } else if telemetry.last_dmx_output_count == 0 {
        (
            TelemetryBudgetStatus::InsufficientSamples,
            format!("{enabled_dmx_output_count} enabled route(s), no sent frame captured."),
        )
    } else if telemetry.last_dmx_send_success_count < enabled_dmx_output_count {
        (
            TelemetryBudgetStatus::Fail,
            format!(
                "{}/{} enabled route(s) sent in the last frame.",
                telemetry.last_dmx_send_success_count, enabled_dmx_output_count
            ),
        )
    } else {
        (
            TelemetryBudgetStatus::Pass,
            format!(
                "{}/{} enabled route(s) sent in the last frame.",
                telemetry.last_dmx_send_success_count, enabled_dmx_output_count
            ),
        )
    };

    TelemetryBudgetCheck {
        name: "dmx_send_success".to_string(),
        status,
        measured_us: None,
        target_us: None,
        samples: telemetry.frame_counter,
        detail,
    }
}

fn telemetry_dmx_interval_check(
    telemetry: &EngineTelemetry,
    enabled_dmx_output_count: usize,
) -> TelemetryBudgetCheck {
    let lower_bound = TELEMETRY_DMX_TARGET_TICK_INTERVAL_US
        .saturating_sub(TELEMETRY_DMX_SEND_INTERVAL_TOLERANCE_US);
    let upper_bound = TELEMETRY_DMX_TARGET_TICK_INTERVAL_US
        .saturating_add(TELEMETRY_DMX_SEND_INTERVAL_TOLERANCE_US);
    let (status, detail) = if enabled_dmx_output_count == 0 {
        (
            TelemetryBudgetStatus::Idle,
            "No enabled DMX output routes.".to_string(),
        )
    } else if telemetry.dmx_send_interval_samples == 0 {
        (
            TelemetryBudgetStatus::InsufficientSamples,
            "Needs at least one DMX send interval sample.".to_string(),
        )
    } else if telemetry.dmx_send_interval_min_us < lower_bound {
        (
            TelemetryBudgetStatus::Fail,
            format!(
                "Min interval {}us is below {}us lower bound.",
                telemetry.dmx_send_interval_min_us, lower_bound
            ),
        )
    } else if telemetry.last_dmx_send_interval_us > upper_bound {
        (
            TelemetryBudgetStatus::Warn,
            format!(
                "Last interval {}us is above {}us upper watch bound.",
                telemetry.last_dmx_send_interval_us, upper_bound
            ),
        )
    } else {
        (
            TelemetryBudgetStatus::Pass,
            format!(
                "Last interval {}us within {}us +/- {}us.",
                telemetry.last_dmx_send_interval_us,
                TELEMETRY_DMX_TARGET_TICK_INTERVAL_US,
                TELEMETRY_DMX_SEND_INTERVAL_TOLERANCE_US
            ),
        )
    };

    TelemetryBudgetCheck {
        name: "dmx_send_interval".to_string(),
        status,
        measured_us: (telemetry.dmx_send_interval_samples > 0)
            .then_some(telemetry.last_dmx_send_interval_us),
        target_us: Some(TELEMETRY_DMX_TARGET_TICK_INTERVAL_US),
        samples: telemetry.dmx_send_interval_samples,
        detail,
    }
}

fn telemetry_overall_budget_status(checks: &[TelemetryBudgetCheck]) -> TelemetryBudgetStatus {
    if checks
        .iter()
        .any(|check| check.status == TelemetryBudgetStatus::Fail)
    {
        return TelemetryBudgetStatus::Fail;
    }
    if checks
        .iter()
        .any(|check| check.status == TelemetryBudgetStatus::Warn)
    {
        return TelemetryBudgetStatus::Warn;
    }
    if checks.iter().any(|check| {
        matches!(
            check.status,
            TelemetryBudgetStatus::InsufficientSamples | TelemetryBudgetStatus::Idle
        )
    }) {
        return TelemetryBudgetStatus::InsufficientSamples;
    }
    TelemetryBudgetStatus::Pass
}

fn video_output_window_label(output_id: VideoOutputId, test_pattern: bool) -> String {
    if test_pattern {
        format!("video-output-{output_id}-test-pattern")
    } else {
        format!("video-output-{output_id}")
    }
}

fn curl_binary_name() -> &'static str {
    if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{ClockSource, VideoLayerSummary};

    fn sample_preset(values: Vec<protocol::AttributeValueSummary>) -> FixturePreset {
        FixturePreset {
            version: 1,
            manufacturer: "Rayard".to_string(),
            profile_name: "Mini Spot".to_string(),
            profile_source_path: None,
            mode_name: "Standard".to_string(),
            values,
        }
    }

    #[test]
    fn gdtf_share_catalog_extraction_filters_fixture_results() {
        let catalog = json!({
            "result": true,
            "fixtures": [
                {
                    "rid": "123",
                    "uuid": "abc",
                    "manufacturer": "Robe",
                    "fixture": "MegaPointe",
                    "revision": "Standard",
                    "version": "1.2",
                    "filesize": 4567,
                    "modes": [
                        { "name": "Mode 1", "dmxFootprint": "34" },
                        { "name": "Mode 2", "footprint": 40 }
                    ]
                },
                {
                    "id": 124,
                    "manufacturer": "ETC",
                    "name": "Source Four",
                    "revisionName": "Classic"
                }
            ]
        });

        let fixtures = extract_gdtf_share_fixtures(&catalog);
        assert_eq!(fixtures.len(), 2);
        assert_eq!(fixtures[0].rid, Some(123));
        assert_eq!(fixtures[0].uuid.as_deref(), Some("abc"));
        assert_eq!(fixtures[0].modes[0].dmx_footprint, Some(34));

        let filtered = filter_gdtf_share_results(fixtures, "Robe", "Mega", "Mode", 10);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].fixture, "MegaPointe");
    }

    #[test]
    fn gdtf_share_download_file_name_is_safe() {
        let request = GdtfShareDownloadRequest {
            user: "user".to_string(),
            password: "pass".to_string(),
            rid: Some(7),
            uuid: None,
            manufacturer: "Robe/Lighting".to_string(),
            fixture: "Mega Pointe".to_string(),
            revision: "Mode:Standard".to_string(),
        };

        assert_eq!(
            gdtf_share_download_file_name(&request),
            "Robe_Lighting-Mega_Pointe-Mode_Standard.gdtf"
        );
    }

    #[test]
    fn gdtf_share_download_payload_rejects_api_error_files() {
        let json_path = std::env::temp_dir().join(format!(
            "rayard-gdtf-share-json-error-{}.gdtf",
            std::process::id()
        ));
        let html_path = std::env::temp_dir().join(format!(
            "rayard-gdtf-share-html-error-{}.gdtf",
            std::process::id()
        ));
        let zip_path =
            std::env::temp_dir().join(format!("rayard-gdtf-share-zip-{}.gdtf", std::process::id()));

        fs::write(&json_path, br#"{"result":false,"error":"Unauthorized."}"#).unwrap();
        fs::write(&html_path, b"<html><body>login</body></html>").unwrap();
        fs::write(&zip_path, b"PK\x03\x04fixture").unwrap();

        assert!(validate_gdtf_share_download_payload(&json_path)
            .unwrap_err()
            .contains("Unauthorized"));
        assert!(validate_gdtf_share_download_payload(&html_path)
            .unwrap_err()
            .contains("returned HTML"));
        validate_gdtf_share_download_payload(&zip_path).unwrap();

        let _ = fs::remove_file(json_path);
        let _ = fs::remove_file(html_path);
        let _ = fs::remove_file(zip_path);
    }

    #[test]
    fn visualizer_external_model_asset_format_uses_magic_before_extension() {
        assert_eq!(
            visualizer_external_model_asset_format("models/lens.bin", b"glTF\x02\0\0\0"),
            VisualizerExternalModelAssetFormat::Glb
        );
        assert_eq!(
            visualizer_external_model_asset_format("models/lens.glb", &[0x4d, 0x4d, 0x00, 0x02]),
            VisualizerExternalModelAssetFormat::ThreeDs
        );
    }

    #[test]
    fn visualizer_external_model_asset_format_falls_back_to_extension_and_text() {
        assert_eq!(
            visualizer_external_model_asset_format("models/body.gltf", b""),
            VisualizerExternalModelAssetFormat::Gltf
        );
        assert_eq!(
            visualizer_external_model_asset_format("models/body.obj", b""),
            VisualizerExternalModelAssetFormat::Obj
        );
        assert_eq!(
            visualizer_external_model_asset_format("models/body.dae", b""),
            VisualizerExternalModelAssetFormat::Collada
        );
        assert_eq!(
            visualizer_external_model_asset_format("models/body.mesh", b"  {\"asset\":{}}"),
            VisualizerExternalModelAssetFormat::Gltf
        );
        assert_eq!(
            visualizer_external_model_asset_format("models/body.mesh", b"\n<COLLADA></COLLADA>"),
            VisualizerExternalModelAssetFormat::Collada
        );
        assert_eq!(
            visualizer_external_model_asset_format("models/body.mesh", &[0, 1, 2]),
            VisualizerExternalModelAssetFormat::Unknown
        );
    }

    #[test]
    fn visualizer_model_asset_cache_returns_loaded_and_missing_entries() {
        let cache = Mutex::new(HashMap::new());
        let source_path = "C:/not-real/cache-fixture.gdtf";
        let model_file = "models/head.obj";
        let cache_key = visualizer_external_model_asset_key(source_path, model_file);
        cache.lock().unwrap().insert(
            cache_key,
            VisualizerModelAssetCacheEntry {
                file_length: None,
                modified_millis: None,
                bytes: Some(vec![1, 2, 3, 4]),
            },
        );

        assert_eq!(
            load_gdtf_model_file_cached(&cache, source_path, "MODELS\\HEAD.OBJ").unwrap(),
            Some(vec![1, 2, 3, 4])
        );

        let missing_key = visualizer_external_model_asset_key(source_path, "models/missing.obj");
        cache.lock().unwrap().insert(
            missing_key,
            VisualizerModelAssetCacheEntry {
                file_length: None,
                modified_millis: None,
                bytes: None,
            },
        );
        assert_eq!(
            load_gdtf_model_file_cached(&cache, source_path, "models/missing.obj").unwrap(),
            None
        );
    }

    #[test]
    fn visualizer_model_asset_cache_ignores_stale_fingerprint() {
        let cache = Mutex::new(HashMap::new());
        let source_path = "C:/not-real/stale-fixture.gdtf";
        let model_file = "models/head.obj";
        let cache_key = visualizer_external_model_asset_key(source_path, model_file);
        cache.lock().unwrap().insert(
            cache_key,
            VisualizerModelAssetCacheEntry {
                file_length: Some(99),
                modified_millis: Some(1234),
                bytes: Some(vec![9, 9, 9]),
            },
        );

        assert!(load_gdtf_model_file_cached(&cache, source_path, model_file).is_err());
    }

    #[test]
    fn visualizer_model_asset_cache_summary_counts_entries_and_bytes() {
        let mut cache = HashMap::new();
        cache.insert(
            "a".to_string(),
            VisualizerModelAssetCacheEntry {
                file_length: None,
                modified_millis: None,
                bytes: Some(vec![1, 2, 3]),
            },
        );
        cache.insert(
            "b".to_string(),
            VisualizerModelAssetCacheEntry {
                file_length: None,
                modified_millis: None,
                bytes: None,
            },
        );

        let summary = visualizer_model_asset_cache_summary(&cache);

        assert_eq!(summary.entry_count, 2);
        assert_eq!(summary.loaded_count, 1);
        assert_eq!(summary.missing_count, 1);
        assert_eq!(summary.byte_length, 3);
        assert_eq!(summary.limit, VISUALIZER_MODEL_ASSET_CACHE_LIMIT);
    }

    #[test]
    fn visualizer_external_obj_asset_builds_meshes_per_render_plan() {
        let plans = vec![
            visualizer_obj_render_plan("Head", 0.0),
            visualizer_obj_render_plan("Mirror", 10.0),
        ];
        let (meshes, error) = visualizer_external_model_asset_meshes(
            &VisualizerExternalModelAssetFormat::Obj,
            b"
v -1 -1 0
v 1 -1 0
v 0 1 0
f 1 2 3
",
            &plans,
        );

        assert!(error.is_none());
        assert_eq!(meshes.len(), 2);
        assert_eq!(meshes[0].geometry_name, "Head");
        assert_eq!(meshes[1].geometry_name, "Mirror");
        assert_eq!(meshes[0].vertices.len(), 3);
        assert_eq!(meshes[0].indices, vec![0, 1, 2]);
        assert!((meshes[1].vertices[0].position.x - 9.0).abs() < 0.001);
    }

    #[test]
    fn visualizer_external_obj_asset_reports_mesh_conversion_errors() {
        let plans = vec![visualizer_obj_render_plan("BrokenHead", 0.0)];
        let (meshes, error) = visualizer_external_model_asset_meshes(
            &VisualizerExternalModelAssetFormat::Obj,
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 4\n",
            &plans,
        );

        assert!(meshes.is_empty());
        let error = error.unwrap();
        assert!(error.contains("BrokenHead"));
        assert!(error.contains("IndexOutOfRange"));
    }

    #[test]
    fn visualizer_external_3ds_asset_builds_meshes_per_render_plan() {
        let plans = vec![visualizer_obj_render_plan("Head3ds", 0.0)];
        let (meshes, error) = visualizer_external_model_asset_meshes(
            &VisualizerExternalModelAssetFormat::ThreeDs,
            &minimal_3ds_bytes(
                &[
                    Vec3 {
                        x: -1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                &[[0, 1, 2]],
            ),
            &plans,
        );

        assert!(error.is_none());
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].geometry_name, "Head3ds");
        assert_eq!(meshes[0].vertices.len(), 3);
        assert_eq!(meshes[0].indices, vec![0, 1, 2]);
    }

    #[test]
    fn visualizer_external_glb_asset_builds_meshes_per_render_plan() {
        let plans = vec![visualizer_obj_render_plan("HeadGlb", 0.0)];
        let (meshes, error) = visualizer_external_model_asset_meshes(
            &VisualizerExternalModelAssetFormat::Glb,
            &minimal_glb_bytes(
                &[
                    Vec3 {
                        x: -1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 1.0,
                        y: -1.0,
                        z: 0.0,
                    },
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ],
                &[0, 1, 2],
            ),
            &plans,
        );

        assert!(error.is_none());
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].geometry_name, "HeadGlb");
        assert_eq!(meshes[0].vertices.len(), 3);
        assert_eq!(meshes[0].indices, vec![0, 1, 2]);
    }

    fn visualizer_obj_render_plan(
        geometry_name: &str,
        x: f32,
    ) -> visualizer::FixtureModelRenderPlan {
        visualizer::FixtureModelRenderPlan {
            fixture_id: 1,
            profile_source_path: "memory://test-visualizer-obj".to_string(),
            geometry_name: geometry_name.to_string(),
            draw_kind: visualizer::FixtureModelDrawKind::ExternalMesh,
            fallback_mesh_kind: visualizer::GeometryModelMeshKind::Mesh,
            model_file: Some("models/model.obj".to_string()),
            dimensions: Some(Vec3 {
                x: 2.0,
                y: 2.0,
                z: 1.0,
            }),
            bounding_radius: 1.5,
            position: Vec3 { x, y: 0.0, z: 0.0 },
            right: Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            up: Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            direction: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        }
    }

    fn minimal_glb_bytes(positions: &[Vec3], indices: &[u16]) -> Vec<u8> {
        let mut bin = Vec::new();
        for position in positions {
            bin.extend(position.x.to_le_bytes());
            bin.extend(position.y.to_le_bytes());
            bin.extend(position.z.to_le_bytes());
        }
        let index_offset = bin.len();
        for index in indices {
            bin.extend(index.to_le_bytes());
        }
        let index_length = bin.len() - index_offset;
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let json = serde_json::json!({
            "asset": { "version": "2.0" },
            "buffers": [{ "byteLength": bin.len() }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0, "byteLength": positions.len() * 12 },
                { "buffer": 0, "byteOffset": index_offset, "byteLength": index_length }
            ],
            "accessors": [
                {
                    "bufferView": 0,
                    "byteOffset": 0,
                    "componentType": 5126,
                    "count": positions.len(),
                    "type": "VEC3"
                },
                {
                    "bufferView": 1,
                    "byteOffset": 0,
                    "componentType": 5123,
                    "count": indices.len(),
                    "type": "SCALAR"
                }
            ],
            "meshes": [
                {
                    "primitives": [
                        {
                            "attributes": { "POSITION": 0 },
                            "indices": 1
                        }
                    ]
                }
            ]
        })
        .to_string();
        let json_chunk = glb_chunk(0x4e4f_534a, json.into_bytes(), 0x20);
        let bin_chunk = glb_chunk(0x004e_4942, bin, 0);
        let declared_length = 12 + json_chunk.len() + bin_chunk.len();
        let mut glb = Vec::with_capacity(declared_length);
        glb.extend(0x4654_6c67u32.to_le_bytes());
        glb.extend(2u32.to_le_bytes());
        glb.extend((declared_length as u32).to_le_bytes());
        glb.extend(json_chunk);
        glb.extend(bin_chunk);
        glb
    }

    fn glb_chunk(chunk_type: u32, mut data: Vec<u8>, padding: u8) -> Vec<u8> {
        while data.len() % 4 != 0 {
            data.push(padding);
        }
        let mut chunk = Vec::with_capacity(data.len() + 8);
        chunk.extend((data.len() as u32).to_le_bytes());
        chunk.extend(chunk_type.to_le_bytes());
        chunk.extend(data);
        chunk
    }

    fn minimal_3ds_bytes(positions: &[Vec3], faces: &[[u16; 3]]) -> Vec<u8> {
        let mut vertex_payload = Vec::new();
        vertex_payload.extend((positions.len() as u16).to_le_bytes());
        for position in positions {
            vertex_payload.extend(position.x.to_le_bytes());
            vertex_payload.extend(position.y.to_le_bytes());
            vertex_payload.extend(position.z.to_le_bytes());
        }

        let mut face_payload = Vec::new();
        face_payload.extend((faces.len() as u16).to_le_bytes());
        for [a, b, c] in faces {
            face_payload.extend(a.to_le_bytes());
            face_payload.extend(b.to_le_bytes());
            face_payload.extend(c.to_le_bytes());
            face_payload.extend(0u16.to_le_bytes());
        }

        let mut mesh_payload = Vec::new();
        mesh_payload.extend(three_ds_chunk(0x4110, vertex_payload));
        mesh_payload.extend(three_ds_chunk(0x4120, face_payload));

        let mut object_payload = b"Mesh\0".to_vec();
        object_payload.extend(three_ds_chunk(0x4100, mesh_payload));
        let editor_payload = three_ds_chunk(0x4000, object_payload);
        three_ds_chunk(0x4d4d, three_ds_chunk(0x3d3d, editor_payload))
    }

    fn three_ds_chunk(id: u16, payload: Vec<u8>) -> Vec<u8> {
        let mut chunk = Vec::with_capacity(payload.len() + 6);
        chunk.extend(id.to_le_bytes());
        chunk.extend(((payload.len() + 6) as u32).to_le_bytes());
        chunk.extend(payload);
        chunk
    }

    #[test]
    fn custom_fixture_profile_request_builds_8bit_controls() {
        let request = CustomFixtureProfileRequest {
            manufacturer: "Rayard".to_string(),
            name: "Custom Bar".to_string(),
            mode_name: "8ch".to_string(),
            attributes: vec![
                "Dimmer".to_string(),
                "Pan".to_string(),
                "ColorRed".to_string(),
            ],
        };

        validate_custom_fixture_profile_request(&request).unwrap();
        let profile = custom_fixture_profile_from_request(request);

        assert_eq!(profile.source_path, "memory://custom/Rayard-Custom_Bar");
        assert_eq!(profile.dmx_modes[0].name, "8ch");
        assert_eq!(profile.dmx_modes[0].controls[0].offsets, vec![1]);
        assert_eq!(profile.dmx_modes[0].controls[1].offsets, vec![2]);
        assert_eq!(profile.dmx_modes[0].controls[1].default_value, 32_768);
        assert_eq!(
            profile
                .geometries
                .iter()
                .map(|geometry| geometry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Body", "Head", "Beam"]
        );
        assert_eq!(
            profile.dmx_modes[0].controls[0].geometry.as_deref(),
            Some("Beam")
        );
        assert_eq!(
            profile.dmx_modes[0].controls[1].geometry.as_deref(),
            Some("Head")
        );
        assert_eq!(
            profile.dmx_modes[0].controls[2].geometry.as_deref(),
            Some("Beam")
        );
        assert!(profile.warnings[0].contains("generated Body/Head/Beam"));
        assert_eq!(gdtf::profile_mode_footprint(&profile, Some("8ch")), Some(3));
    }

    #[test]
    fn custom_fixture_profile_request_builds_mixed_resolution_controls() {
        let request = CustomFixtureProfileRequest {
            manufacturer: "Rayard".to_string(),
            name: "Custom Spot".to_string(),
            mode_name: "Standard".to_string(),
            attributes: vec![
                "Dimmer@1:8".to_string(),
                "Pan@5:16".to_string(),
                "Tilt:16".to_string(),
                "ColorRed@10".to_string(),
            ],
        };

        validate_custom_fixture_profile_request(&request).unwrap();
        let profile = custom_fixture_profile_from_request(request);
        let controls = &profile.dmx_modes[0].controls;

        assert_eq!(controls[0].attribute, "Dimmer");
        assert_eq!(controls[0].resolution, AttributeResolution::EightBit);
        assert_eq!(controls[0].offsets, vec![1]);
        assert_eq!(controls[1].attribute, "Pan");
        assert_eq!(controls[1].resolution, AttributeResolution::SixteenBit);
        assert_eq!(controls[1].offsets, vec![5, 6]);
        assert_eq!(controls[2].attribute, "Tilt");
        assert_eq!(controls[2].offsets, vec![7, 8]);
        assert_eq!(controls[3].offsets, vec![10]);
        assert_eq!(controls[0].geometry.as_deref(), Some("Beam"));
        assert_eq!(controls[1].geometry.as_deref(), Some("Head"));
        assert_eq!(controls[2].geometry.as_deref(), Some("Head"));
        assert_eq!(controls[3].geometry.as_deref(), Some("Beam"));
        assert_eq!(
            gdtf::profile_mode_footprint(&profile, Some("Standard")),
            Some(10)
        );
    }

    #[test]
    fn gdtf_profile_patch_drives_engine_dmx_preview() {
        let profile = gdtf::parse_description_xml(
            "memory://mvp-fixture.gdtf",
            r#"
            <GDTF>
              <FixtureType Name="MVP Spot" Manufacturer="Rayard">
                <DMXModes>
                  <DMXMode Name="Standard">
                    <DMXChannels>
                      <DMXChannel Name="Dimmer" Offset="1">
                        <LogicalChannel Attribute="Dimmer">
                          <ChannelFunction Name="Dimmer" Attribute="Dimmer" DMXFrom="0/1" />
                        </LogicalChannel>
                      </DMXChannel>
                      <DMXChannel Name="Pan" Offset="2,3">
                        <LogicalChannel Attribute="Pan">
                          <ChannelFunction Name="Pan" Attribute="Pan" DMXFrom="0/2" />
                        </LogicalChannel>
                      </DMXChannel>
                    </DMXChannels>
                  </DMXMode>
                </DMXModes>
              </FixtureType>
            </GDTF>
            "#,
        )
        .unwrap();
        assert_eq!(
            gdtf::profile_mode_footprint(&profile, Some("Standard")),
            Some(3)
        );

        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: profile.source_path.clone(),
                    mode_name: Some("Standard".to_string()),
                    label: "GDTF MVP Spot".to_string(),
                    universe: 0,
                    address: 10,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Rotation3::default(),
                },
                profile,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetAttribute {
                fixture_id,
                attribute: "Dimmer".to_string(),
                value: 65_535,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetAttribute {
                fixture_id,
                attribute: "Pan".to_string(),
                value: 0x3456,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview.get(9) == Some(&255)
                && snapshot.dmx_preview.get(10) == Some(&0x34)
                && snapshot.dmx_preview.get(11) == Some(&0x56)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.dmx_preview[9], 255);
        assert_eq!(snapshot.dmx_preview[10], 0x34);
        assert_eq!(snapshot.dmx_preview[11], 0x56);
    }

    #[test]
    fn custom_fixture_profile_validation_rejects_duplicate_attributes() {
        let request = CustomFixtureProfileRequest {
            manufacturer: "Rayard".to_string(),
            name: "Bad".to_string(),
            mode_name: "Default".to_string(),
            attributes: vec!["Dimmer:8".to_string(), "dimmer:16".to_string()],
        };

        let error = validate_custom_fixture_profile_request(&request).unwrap_err();

        assert!(error.contains("duplicated"));
    }

    #[test]
    fn custom_fixture_profile_validation_rejects_overlapping_offsets() {
        let request = CustomFixtureProfileRequest {
            manufacturer: "Rayard".to_string(),
            name: "Bad".to_string(),
            mode_name: "Default".to_string(),
            attributes: vec!["Dimmer@1:8".to_string(), "Pan@1:16".to_string()],
        };

        let error = validate_custom_fixture_profile_request(&request).unwrap_err();

        assert!(error.contains("overlaps"));
    }

    #[test]
    fn custom_fixture_profile_validation_rejects_invalid_start_channel() {
        let request = CustomFixtureProfileRequest {
            manufacturer: "Rayard".to_string(),
            name: "Bad".to_string(),
            mode_name: "Default".to_string(),
            attributes: vec!["Dimmer@0:8".to_string()],
        };

        let error = validate_custom_fixture_profile_request(&request).unwrap_err();

        assert!(error.contains("start channel"));
    }

    #[test]
    fn custom_fixture_profile_file_serializes_and_validates() {
        let profile_file = CustomFixtureProfileFile {
            version: 1,
            request: CustomFixtureProfileRequest {
                manufacturer: "Rayard".to_string(),
                name: "Tiny Bar".to_string(),
                mode_name: "Default".to_string(),
                attributes: vec!["Dimmer".to_string(), "ColorRed".to_string()],
            },
        };

        let json = serde_json::to_string_pretty(&profile_file).unwrap();
        let parsed: CustomFixtureProfileFile = serde_json::from_str(&json).unwrap();

        validate_custom_fixture_profile_file(&parsed).unwrap();
        assert!(json.contains("\"version\": 1"));
        assert_eq!(parsed.request.attributes.len(), 2);
    }

    #[test]
    fn phase1_smoke_fixture_sample_is_valid() {
        let json = include_str!("../../../samples/phase1-mini-spot.fixture");
        let profile_file: CustomFixtureProfileFile = serde_json::from_str(json).unwrap();

        validate_custom_fixture_profile_file(&profile_file).unwrap();
        let profile = custom_fixture_profile_from_request(profile_file.request);

        assert_eq!(profile.manufacturer, "Rayard");
        assert_eq!(profile.name, "Phase 1 Mini Spot");
        assert_eq!(profile.dmx_modes[0].name, "8ch");
        assert_eq!(profile.dmx_modes[0].controls.len(), 6);
        assert_eq!(profile.dmx_modes[0].controls[0].offsets, vec![1]);
        assert_eq!(profile.dmx_modes[0].controls[4].offsets, vec![5, 6]);
        assert_eq!(profile.dmx_modes[0].controls[5].offsets, vec![7, 8]);
    }

    #[test]
    fn phase1_smoke_project_sample_is_valid() {
        let project: ProjectFile = serde_json::from_str(PHASE1_SAMPLE_PROJECT_JSON).unwrap();

        validate_project_file(&project).unwrap();

        assert_eq!(PHASE1_SAMPLE_PROJECT_LABEL, "samples/phase1-mini-show.ry");
        assert_eq!(project.app, APP_NAME);
        assert_eq!(project.custom_profiles.len(), 1);
        assert_eq!(project.snapshot.fixtures.len(), 1);
        assert_eq!(
            project.snapshot.fixtures[0].profile_source_path,
            project.custom_profiles[0].source_path
        );
        assert_eq!(project.snapshot.fixtures[0].address, 1);
        assert_eq!(
            project.snapshot.fixtures[0]
                .controls
                .last()
                .unwrap()
                .offsets,
            vec![7, 8]
        );
        assert_eq!(project.custom_profiles[0].geometries.len(), 3);
        assert_eq!(project.snapshot.fixtures[0].geometries.len(), 3);
        assert_eq!(
            project.snapshot.fixtures[0].controls[0].geometry.as_deref(),
            Some("Beam")
        );
        assert_eq!(
            project.snapshot.fixtures[0].controls[4].geometry.as_deref(),
            Some("Head")
        );
        assert_eq!(project.snapshot.cues.len(), 1);
        assert_eq!(project.snapshot.timeline.events.len(), 2);
        assert_eq!(project.snapshot.timeline.events[0].cue_id, 1);
        assert_eq!(project.snapshot.timeline.events[0].time_ms, 0);
        assert_eq!(
            project.snapshot.timeline.events[0].track,
            TimelineTrackKind::Lighting
        );
        assert_eq!(project.snapshot.timeline.events[1].cue_id, 1);
        assert_eq!(project.snapshot.timeline.events[1].time_ms, 4000);
        assert_eq!(
            project.snapshot.timeline.events[1].track,
            TimelineTrackKind::Lighting
        );
        assert_eq!(project.snapshot.timeline.automations.len(), 1);
        assert_eq!(project.snapshot.timeline.automations[0].fixture_id, 1);
        assert_eq!(project.snapshot.timeline.automations[0].attribute, "Dimmer");
        assert_eq!(
            project.snapshot.timeline.automations[0].track,
            TimelineTrackKind::Lighting
        );
        assert_eq!(
            project.snapshot.timeline.automations[0].keyframes[0].value,
            65_535
        );
        assert_eq!(
            project.snapshot.timeline.automations[0].keyframes[1].time_ms,
            4000
        );
        assert_eq!(project.snapshot.timeline.video_automations.len(), 1);
        assert_eq!(project.snapshot.timeline.video_automations[0].layer_id, 1);
        assert_eq!(
            project.snapshot.timeline.video_automations[0].param,
            VideoParam::Opacity
        );
        assert_eq!(
            project.snapshot.timeline.video_automations[0].track,
            TimelineTrackKind::Video
        );
        assert_eq!(
            project.snapshot.timeline.video_automations[0].keyframes[0].value,
            1.0
        );
        assert_eq!(
            project.snapshot.timeline.video_automations[0].keyframes[1].time_ms,
            4000
        );
        assert_eq!(project.snapshot.timeline.duration_ms, 4000);
        assert_eq!(
            project.snapshot.cues[0].targets[0].fixture_id,
            project.snapshot.fixtures[0].id
        );
        assert_eq!(project.snapshot.output.protocol, DmxOutputProtocol::ArtNet);
        assert_eq!(project.snapshot.cues[0].video_targets.len(), 1);
        assert_eq!(project.snapshot.cues[0].video_targets[0].layer_id, 1);
        assert!(project.snapshot.cues[0].video_targets[0].state.playing);
        assert!(
            project.snapshot.cues[0].video_targets[0]
                .state
                .bpm_sync
                .enabled
        );
        assert_eq!(project.snapshot.video.layers.len(), 1);
        assert_eq!(project.snapshot.video.layers[0].id, 1);
        assert_eq!(project.snapshot.video.layers[0].label, "Stage NDI");
        assert_eq!(
            project.snapshot.video.layers[0].source.kind,
            VideoSourceKind::Ndi
        );
        assert_eq!(
            project.snapshot.video.layers[0].source.name.as_deref(),
            Some("Stage NDI")
        );
        assert_eq!(project.snapshot.video.layers[0].state.opacity, 0.7);
        assert_eq!(project.snapshot.video.compositions[0].layer_ids, vec![1]);
        assert_eq!(project.snapshot.video.outputs.len(), 1);
        assert_eq!(project.snapshot.video.outputs[0].width, 1920);
        assert_eq!(project.snapshot.video.outputs[0].height, 1080);
        assert_eq!(
            project.snapshot.video.outputs[0].mapping.aspect_mode,
            protocol::VideoOutputAspectMode::Fit
        );
        assert!(
            (project.snapshot.video.outputs[0].mapping.aspect_ratio - (16.0 / 9.0)).abs() < 0.001
        );
        assert_eq!(
            project.snapshot.video.mapping_presets[0].label,
            "16:9 Front Fit"
        );
        assert_eq!(project.snapshot.stage_map_presets.len(), 1);
        assert_eq!(project.snapshot.stage_map_presets[0].label, "Mini Venue");
        assert_eq!(
            project.snapshot.stage_map_presets[0]
                .stage_objects
                .as_ref()
                .map(Vec::len),
            Some(3)
        );
        assert_eq!(project.snapshot.stage_objects.len(), 3);
        assert_eq!(project.snapshot.stage_objects[0].label, "Main Deck");
        assert_eq!(
            project.snapshot.stage_objects[2].kind,
            protocol::StageObjectKind::Screen
        );
    }

    #[test]
    fn phase1_smoke_project_sample_loads_into_engine_and_renders_cue() {
        let json = include_str!("../../../samples/phase1-mini-show.ry");
        let project: ProjectFile = serde_json::from_str(json).unwrap();
        validate_project_file(&project).unwrap();

        let cue_id = project.snapshot.cues[0].id;
        let mut snapshot_to_load = project.snapshot.clone();
        snapshot_to_load.output.enabled = false;
        for output in &mut snapshot_to_load.dmx_outputs {
            output.enabled = false;
        }

        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine.send(EngineCommand::TriggerCue(cue_id)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..30 {
            if snapshot.active_cue_id == Some(cue_id)
                && snapshot.dmx_preview.get(0) == Some(&255)
                && snapshot.dmx_preview.get(1) == Some(&255)
                && snapshot.dmx_preview.get(2) == Some(&255)
                && snapshot.dmx_preview.get(3) == Some(&255)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.active_cue_id, Some(cue_id));
        assert_eq!(snapshot.dmx_preview[0], 255);
        assert_eq!(snapshot.dmx_preview[1], 255);
        assert_eq!(snapshot.dmx_preview[2], 255);
        assert_eq!(snapshot.dmx_preview[3], 255);
        assert_eq!(snapshot.dmx_preview[4], 0x80);
        assert_eq!(snapshot.dmx_preview[5], 0x00);
        assert_eq!(snapshot.dmx_preview[6], 0x80);
        assert_eq!(snapshot.dmx_preview[7], 0x00);
        assert_eq!(snapshot.video.layers.len(), 1);
        assert_eq!(snapshot.video.layers[0].label, "Stage NDI");
        assert!(snapshot.video.layers[0].state.playing);
        assert_eq!(snapshot.video.layers[0].state.opacity, 1.0);
        assert!(snapshot.video.layers[0].state.bpm_sync.enabled);
        assert_eq!(snapshot.video.compositions[0].layer_ids, vec![1]);
    }

    #[test]
    fn phase1_smoke_project_sample_timeline_event_triggers_light_and_video() {
        let project: ProjectFile = serde_json::from_str(PHASE1_SAMPLE_PROJECT_JSON).unwrap();
        validate_project_file(&project).unwrap();

        let cue_id = project.snapshot.cues[0].id;
        let mut snapshot_to_load = project.snapshot.clone();
        snapshot_to_load.output.enabled = false;
        for output in &mut snapshot_to_load.dmx_outputs {
            output.enabled = false;
        }

        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine
            .send(EngineCommand::SyncTimelineTimecode {
                position_ms: 0,
                source: ClockSource::MidiTimecode,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..30 {
            if snapshot.active_cue_id == Some(cue_id)
                && snapshot.dmx_preview.get(0) == Some(&255)
                && snapshot
                    .video
                    .layers
                    .first()
                    .map(|layer| layer.state.playing && layer.state.opacity == 1.0)
                    == Some(true)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.active_cue_id, Some(cue_id));
        assert_eq!(snapshot.timeline.events.len(), 2);
        assert_eq!(
            snapshot.timeline.events[0].track,
            TimelineTrackKind::Lighting
        );
        assert_eq!(snapshot.timeline.events[1].time_ms, 4000);
        assert_eq!(snapshot.timeline.duration_ms, 4000);
        assert_eq!(snapshot.dmx_preview[0], 255);
        assert_eq!(snapshot.dmx_preview[1], 255);
        assert_eq!(snapshot.dmx_preview[2], 255);
        assert_eq!(snapshot.dmx_preview[3], 255);
        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(layer.label, "Stage NDI");
        assert!(layer.state.playing);
        assert_eq!(layer.state.opacity, 1.0);
        assert!(layer.state.bpm_sync.enabled);
    }

    #[test]
    fn phase1_smoke_project_sample_timeline_automations_share_position() {
        let project: ProjectFile = serde_json::from_str(PHASE1_SAMPLE_PROJECT_JSON).unwrap();
        validate_project_file(&project).unwrap();

        let mut snapshot_to_load = project.snapshot.clone();
        snapshot_to_load.output.enabled = false;
        for output in &mut snapshot_to_load.dmx_outputs {
            output.enabled = false;
        }

        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine.send(EngineCommand::SeekTimeline(2000)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..30 {
            if snapshot.timeline.position_ms == 2000
                && snapshot.timeline.automations.len() == 1
                && snapshot.timeline.video_automations.len() == 1
                && snapshot.dmx_preview.get(0) == Some(&128)
                && snapshot
                    .video
                    .layers
                    .first()
                    .map(|layer| (0.49..=0.51).contains(&layer.state.opacity))
                    == Some(true)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(snapshot.timeline.position_ms, 2000);
        assert_eq!(snapshot.timeline.automations.len(), 1);
        assert_eq!(snapshot.timeline.video_automations.len(), 1);
        assert_eq!(snapshot.dmx_preview[0], 128);
        assert!((0.49..=0.51).contains(&layer.state.opacity));
    }

    #[test]
    fn phase1_smoke_project_sample_sends_cue_to_artnet_loopback() {
        let json = include_str!("../../../samples/phase1-mini-show.ry");
        let project: ProjectFile = serde_json::from_str(json).unwrap();
        validate_project_file(&project).unwrap();

        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let artnet_port = receiver.local_addr().unwrap().port();
        let cue_id = project.snapshot.cues[0].id;
        let mut snapshot_to_load = project.snapshot.clone();
        snapshot_to_load.output.enabled = true;
        snapshot_to_load.output.protocol = DmxOutputProtocol::ArtNet;
        snapshot_to_load.output.target_ip = "127.0.0.1".to_string();
        snapshot_to_load.output.port = artnet_port;
        snapshot_to_load.output.universe = 0;
        snapshot_to_load.dmx_outputs = vec![snapshot_to_load.output.clone()];

        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine.send(EngineCommand::TriggerCue(cue_id)).unwrap();

        let mut buffer = [0u8; 600];
        let mut saw_sample_cue_packet = false;
        for _ in 0..30 {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = io::artnet::parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.universe == 0
                && packet.data.get(0) == Some(&255)
                && packet.data.get(1) == Some(&255)
                && packet.data.get(2) == Some(&255)
                && packet.data.get(3) == Some(&255)
                && packet.data.get(4) == Some(&0x80)
                && packet.data.get(5) == Some(&0x00)
                && packet.data.get(6) == Some(&0x80)
                && packet.data.get(7) == Some(&0x00)
            {
                saw_sample_cue_packet = true;
                break;
            }
        }

        assert!(saw_sample_cue_packet);
    }

    #[test]
    fn control_mapping_files_preserve_clock_actions() {
        let midi_file = MidiMappingFile {
            version: 1,
            mappings: vec![
                MidiControlMapping {
                    channel: Some(0),
                    message: protocol::MidiControlMessage::ControlChange,
                    number: 24,
                    action: protocol::MidiControlAction::SetBpm,
                    fixture_id: None,
                    attribute: None,
                    group_id: None,
                    cue_id: None,
                    layer_id: None,
                    output_id: None,
                    video_param: None,
                    cue_point_index: None,
                    duration_ms: None,
                    low: 20.0,
                    high: 300.0,
                },
                MidiControlMapping {
                    channel: Some(0),
                    message: protocol::MidiControlMessage::NoteOn,
                    number: 25,
                    action: protocol::MidiControlAction::TimelineBeatNext,
                    fixture_id: None,
                    attribute: None,
                    group_id: None,
                    cue_id: None,
                    layer_id: None,
                    output_id: None,
                    video_param: None,
                    cue_point_index: None,
                    duration_ms: None,
                    low: 0.0,
                    high: 1.0,
                },
            ],
        };
        let osc_file = OscMappingFile {
            version: 1,
            mappings: vec![OscControlMapping {
                address: "/touchosc/tap".to_string(),
                action: protocol::OscControlAction::TapBpm,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                output_id: None,
                video_param: None,
                cue_point_index: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            }],
        };

        let midi_json = serde_json::to_string_pretty(&midi_file).unwrap();
        let osc_json = serde_json::to_string_pretty(&osc_file).unwrap();
        let parsed_midi: MidiMappingFile = serde_json::from_str(&midi_json).unwrap();
        let parsed_osc: OscMappingFile = serde_json::from_str(&osc_json).unwrap();

        assert_eq!(
            parsed_midi.mappings[0].action,
            protocol::MidiControlAction::SetBpm
        );
        assert_eq!(
            parsed_midi.mappings[1].action,
            protocol::MidiControlAction::TimelineBeatNext
        );
        assert_eq!(
            parsed_osc.mappings[0].action,
            protocol::OscControlAction::TapBpm
        );
    }

    #[test]
    fn control_mapping_validation_normalizes_external_midi_files() {
        let mappings = validate_midi_control_mappings(vec![
            MidiControlMapping {
                channel: Some(0),
                message: protocol::MidiControlMessage::ControlChange,
                number: 24,
                action: MidiControlAction::GroupSubmaster,
                fixture_id: None,
                attribute: None,
                group_id: Some(" front / movers ".to_string()),
                cue_id: None,
                layer_id: None,
                output_id: None,
                video_param: None,
                cue_point_index: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            MidiControlMapping {
                channel: None,
                message: protocol::MidiControlMessage::NoteOn,
                number: 25,
                action: MidiControlAction::VideoOutputMappingPreset,
                fixture_id: None,
                attribute: Some(" Front Projector ".to_string()),
                group_id: None,
                cue_id: None,
                layer_id: None,
                output_id: Some(7),
                video_param: None,
                cue_point_index: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            MidiControlMapping {
                channel: None,
                message: protocol::MidiControlMessage::NoteOn,
                number: 26,
                action: MidiControlAction::ClearFixtureFlags,
                fixture_id: None,
                attribute: Some(" Solo ".to_string()),
                group_id: None,
                cue_id: None,
                layer_id: None,
                output_id: None,
                video_param: None,
                cue_point_index: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
        ])
        .unwrap();

        assert_eq!(mappings[0].group_id.as_deref(), Some("front/movers"));
        assert_eq!(mappings[1].attribute.as_deref(), Some("Front Projector"));
        assert_eq!(mappings[2].attribute.as_deref(), Some("solo"));

        let error = validate_midi_control_mappings(vec![MidiControlMapping {
            channel: Some(16),
            message: protocol::MidiControlMessage::ControlChange,
            number: 200,
            action: MidiControlAction::VideoParam,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: None,
            layer_id: None,
            output_id: None,
            video_param: Some(VideoParam::Opacity),
            cue_point_index: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        }])
        .unwrap_err();
        assert!(error.contains("channel"));

        let error = validate_midi_control_mappings(vec![MidiControlMapping {
            channel: None,
            message: protocol::MidiControlMessage::ControlChange,
            number: 24,
            action: MidiControlAction::VideoParam,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: None,
            layer_id: None,
            output_id: None,
            video_param: Some(VideoParam::Opacity),
            cue_point_index: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        }])
        .unwrap_err();
        assert!(error.contains("video layer"));
    }

    #[test]
    fn control_mapping_validation_normalizes_external_osc_files() {
        let mappings = validate_osc_control_mappings(vec![
            OscControlMapping {
                address: " touchosc/page/*/fader/1 ".to_string(),
                action: OscControlAction::FixtureAttribute,
                fixture_id: Some(1),
                attribute: Some(" Dimmer ".to_string()),
                group_id: None,
                cue_id: None,
                layer_id: None,
                output_id: None,
                video_param: None,
                cue_point_index: None,
                duration_ms: None,
                low: 0.0,
                high: 65_535.0,
            },
            OscControlMapping {
                address: "/map/key".to_string(),
                action: OscControlAction::VideoOutputMappingField,
                fixture_id: None,
                attribute: Some(" keystone_x ".to_string()),
                group_id: None,
                cue_id: None,
                layer_id: None,
                output_id: Some(7),
                video_param: None,
                cue_point_index: None,
                duration_ms: None,
                low: -1.0,
                high: 1.0,
            },
        ])
        .unwrap();

        assert_eq!(mappings[0].address, "/touchosc/page/*/fader/1");
        assert_eq!(mappings[0].attribute.as_deref(), Some("Dimmer"));
        assert_eq!(mappings[1].attribute.as_deref(), Some("keystone_x"));

        let error = validate_osc_control_mappings(vec![OscControlMapping {
            address: " ".to_string(),
            action: OscControlAction::TapBpm,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: None,
            layer_id: None,
            output_id: None,
            video_param: None,
            cue_point_index: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        }])
        .unwrap_err();
        assert!(error.contains("address is required"));

        let error = validate_osc_control_mappings(vec![OscControlMapping {
            address: "/fixture/1/dimmer".to_string(),
            action: OscControlAction::FixtureAttribute,
            fixture_id: Some(1),
            attribute: Some(" ".to_string()),
            group_id: None,
            cue_id: None,
            layer_id: None,
            output_id: None,
            video_param: None,
            cue_point_index: None,
            duration_ms: None,
            low: f32::NAN,
            high: 65_535.0,
        }])
        .unwrap_err();
        assert!(error.contains("range values must be finite"));
    }

    #[test]
    fn custom_fixture_profile_file_rejects_wrong_version() {
        let profile_file = CustomFixtureProfileFile {
            version: 2,
            request: CustomFixtureProfileRequest {
                manufacturer: "Rayard".to_string(),
                name: "Tiny Bar".to_string(),
                mode_name: "Default".to_string(),
                attributes: vec!["Dimmer".to_string()],
            },
        };

        assert!(validate_custom_fixture_profile_file(&profile_file)
            .unwrap_err()
            .contains("Unsupported custom fixture profile version"));
    }

    #[test]
    fn gdtf_download_url_validation_rejects_unsafe_values() {
        assert_eq!(
            validate_gdtf_download_url(" https://gdtf-share.com/example.gdtf ").unwrap(),
            "https://gdtf-share.com/example.gdtf"
        );
        assert!(validate_gdtf_download_url("")
            .unwrap_err()
            .contains("required"));
        assert!(validate_gdtf_download_url("ftp://example.com/file.gdtf")
            .unwrap_err()
            .contains("http:// or https://"));
        assert!(
            validate_gdtf_download_url("https://example.com/bad file.gdtf")
                .unwrap_err()
                .contains("whitespace")
        );
    }

    #[test]
    fn gdtf_download_file_name_is_safe_and_has_extension() {
        assert_eq!(
            gdtf_download_file_name_from_url(
                "https://gdtf-share.com/download/Robe%20Spot.gdtf?x=1"
            ),
            "Robe_Spot.gdtf"
        );
        assert_eq!(
            gdtf_download_file_name_from_url(
                "https://gdtf-share.com/download/Manufacturer%2FFixture%2EQ.gdtf"
            ),
            "Manufacturer_Fixture_Q.gdtf"
        );
        assert_eq!(
            gdtf_download_file_name_from_url("https://gdtf-share.com/download/Bad%ZZName.gdtf"),
            "Bad_ZZName.gdtf"
        );
        assert_eq!(
            gdtf_download_file_name_from_url("https://gdtf-share.com/fixtures/robe-spot"),
            "robe-spot.gdtf"
        );
        assert_eq!(
            gdtf_download_file_name_from_url("https://gdtf-share.com/download/"),
            "download.gdtf"
        );
    }

    #[test]
    fn dmx_output_route_validation_rejects_duplicate_network_routes() {
        let route = DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port: 6454,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        };

        let error = validate_dmx_output_routes(&[route.clone(), route]).unwrap_err();

        assert!(error.contains("Duplicate DMX output route"));
    }

    #[test]
    fn dmx_output_route_validation_treats_sacn_multicast_aliases_as_duplicates() {
        let first = DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::Sacn,
            target_ip: " multicast ".to_string(),
            port: 5568,
            universe: 7,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        };
        let second = DmxOutputConfig {
            target_ip: "auto".to_string(),
            ..first.clone()
        };

        let error = validate_dmx_output_routes(&[first, second]).unwrap_err();

        assert!(error.contains("Duplicate DMX output route"));
    }

    #[test]
    fn dmx_output_route_validation_allows_same_universe_to_different_targets() {
        let first = DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port: 6454,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        };
        let second = DmxOutputConfig {
            target_ip: "192.168.0.50".to_string(),
            ..first.clone()
        };

        validate_dmx_output_routes(&[first, second]).unwrap();
    }

    #[test]
    fn dmx_output_route_validation_rejects_duplicate_serial_port_routes() {
        let first = DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::EnttecUsbPro,
            target_ip: String::new(),
            port: 6454,
            universe: 0,
            serial_port: " COM3 ".to_string(),
            serial_baud_rate: 57_600,
        };
        let second = DmxOutputConfig {
            protocol: DmxOutputProtocol::EnttecOpenDmx,
            universe: 2,
            serial_port: "com3".to_string(),
            serial_baud_rate: 250_000,
            ..first.clone()
        };

        let error = validate_dmx_output_routes(&[first, second]).unwrap_err();

        assert!(error.contains("Duplicate DMX output route"));
    }

    #[test]
    fn dmx_output_config_validation_treats_dmxking_as_serial_pro_route() {
        validate_dmx_output_config(&DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::DmxKingUltraDmx,
            target_ip: String::new(),
            port: 0,
            universe: 8,
            serial_port: "COM4".to_string(),
            serial_baud_rate: 57_600,
        })
        .unwrap();

        let missing_port_error = validate_dmx_output_config(&DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::DmxKingUltraDmx,
            target_ip: String::new(),
            port: 0,
            universe: 8,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        })
        .unwrap_err();
        assert!(missing_port_error.contains("Serial port"));

        let zero_baud_error = validate_dmx_output_config(&DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::DmxKingUltraDmx,
            target_ip: String::new(),
            port: 0,
            universe: 8,
            serial_port: "COM4".to_string(),
            serial_baud_rate: 0,
        })
        .unwrap_err();
        assert!(zero_baud_error.contains("Serial baud rate"));

        let duplicate_error = validate_dmx_output_routes(&[
            DmxOutputConfig {
                enabled: true,
                protocol: DmxOutputProtocol::EnttecUsbPro,
                target_ip: String::new(),
                port: 0,
                universe: 1,
                serial_port: " COM4 ".to_string(),
                serial_baud_rate: 57_600,
            },
            DmxOutputConfig {
                enabled: true,
                protocol: DmxOutputProtocol::DmxKingUltraDmx,
                target_ip: String::new(),
                port: 0,
                universe: 2,
                serial_port: "com4".to_string(),
                serial_baud_rate: 57_600,
            },
        ])
        .unwrap_err();
        assert!(duplicate_error.contains("Duplicate DMX output route"));
    }

    #[test]
    fn dmx_output_config_validation_allows_disabled_incomplete_routes() {
        validate_dmx_output_config(&DmxOutputConfig {
            enabled: false,
            target_ip: " ".to_string(),
            port: 0,
            ..DmxOutputConfig::default()
        })
        .unwrap();

        validate_dmx_output_config(&DmxOutputConfig {
            enabled: false,
            protocol: DmxOutputProtocol::EnttecUsbPro,
            target_ip: String::new(),
            serial_port: String::new(),
            serial_baud_rate: 0,
            ..DmxOutputConfig::default()
        })
        .unwrap();
    }

    #[test]
    fn dmx_output_route_validation_ignores_disabled_draft_routes() {
        let first = DmxOutputConfig {
            enabled: false,
            protocol: DmxOutputProtocol::EnttecUsbPro,
            target_ip: String::new(),
            port: 0,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 0,
        };
        let second = DmxOutputConfig {
            protocol: DmxOutputProtocol::EnttecOpenDmx,
            universe: 2,
            ..first.clone()
        };

        validate_dmx_output_routes(&[first, second]).unwrap();
    }

    #[test]
    fn dmx_output_config_validation_rejects_blank_network_target() {
        let error = validate_dmx_output_config(&DmxOutputConfig {
            target_ip: " ".to_string(),
            ..DmxOutputConfig::default()
        })
        .unwrap_err();

        assert!(error.contains("target IP"));
    }

    #[test]
    fn dmx_output_config_validation_rejects_protocol_universe_overflow() {
        let artnet_error = validate_dmx_output_config(&DmxOutputConfig {
            universe: ARTNET_MAX_UNIVERSE + 1,
            ..DmxOutputConfig::default()
        })
        .unwrap_err();
        assert!(artnet_error.contains("Art-Net universe"));

        let sacn_error = validate_dmx_output_config(&DmxOutputConfig {
            protocol: DmxOutputProtocol::Sacn,
            target_ip: "multicast".to_string(),
            port: 5568,
            universe: SACN_MAX_UNIVERSE + 1,
            ..DmxOutputConfig::default()
        })
        .unwrap_err();
        assert!(sacn_error.contains("sACN universe"));
    }

    #[test]
    fn dmx_output_config_validation_allows_sacn_multicast_target() {
        validate_dmx_output_config(&DmxOutputConfig {
            protocol: DmxOutputProtocol::Sacn,
            target_ip: "multicast".to_string(),
            port: 5568,
            universe: 1,
            ..DmxOutputConfig::default()
        })
        .unwrap();
    }

    #[test]
    fn dmx_test_frame_marks_requested_channel_range() {
        let frame = build_dmx_test_frame(10, 4, 201).unwrap();

        assert_eq!(frame[8], 0);
        assert_eq!(&frame[9..13], &[201, 201, 201, 201]);
        assert_eq!(frame[13], 0);
    }

    #[test]
    fn dmx_test_frame_validation_rejects_invalid_ranges() {
        assert!(build_dmx_test_frame(0, 1, 255)
            .unwrap_err()
            .contains("between 1 and 512"));
        assert!(build_dmx_test_frame(512, 2, 255)
            .unwrap_err()
            .contains("exceeds 512"));
        assert!(build_dmx_test_frame(1, 0, 255)
            .unwrap_err()
            .contains("at least 1"));
        assert!(build_dmx_test_frame(1, 1, 256)
            .unwrap_err()
            .contains("between 0 and 255"));
    }

    #[test]
    fn engine_telemetry_report_captures_snapshot_counts_and_routes() {
        let mut snapshot = EngineSnapshot::default();
        snapshot.clock.bpm = 128.0;
        snapshot.telemetry.frame_counter = 44;
        snapshot.telemetry.tick_jitter_p99_us = 900;
        snapshot.telemetry.tick_jitter_samples = 32;
        snapshot.telemetry.command_queue_latency_p99_us = 700;
        snapshot.telemetry.command_queue_latency_samples = 4;
        snapshot.telemetry.command_to_dmx_tick_latency_p99_us = 4_500;
        snapshot.telemetry.command_to_dmx_tick_latency_samples = 4;
        snapshot.telemetry.last_dmx_send_success_count = 1;
        snapshot.telemetry.last_dmx_output_count = 1;
        snapshot.telemetry.last_dmx_send_interval_us = TELEMETRY_DMX_TARGET_TICK_INTERVAL_US;
        snapshot.telemetry.dmx_send_interval_min_us = TELEMETRY_DMX_TARGET_TICK_INTERVAL_US;
        snapshot.telemetry.dmx_send_interval_max_us = TELEMETRY_DMX_TARGET_TICK_INTERVAL_US;
        snapshot.telemetry.dmx_send_interval_samples = 3;
        snapshot.output = DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port: 6454,
            universe: 1,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        };
        snapshot.dmx_outputs = vec![
            snapshot.output.clone(),
            DmxOutputConfig {
                enabled: false,
                protocol: DmxOutputProtocol::Sacn,
                target_ip: "multicast".to_string(),
                port: 5568,
                universe: 2,
                serial_port: String::new(),
                serial_baud_rate: 57_600,
            },
        ];
        snapshot.dmx_previews = vec![protocol::DmxUniversePreview {
            universe: 1,
            values: vec![0; 512],
        }];

        let report = engine_telemetry_report_from_snapshot(&snapshot, 1_780_000);

        assert_eq!(report.version, 1);
        assert_eq!(report.captured_at_unix_ms, 1_780_000);
        assert_eq!(report.clock.bpm, 128.0);
        assert_eq!(report.telemetry.frame_counter, 44);
        assert_eq!(report.telemetry.tick_jitter_p99_us, 900);
        assert_eq!(report.dmx_output_count, 2);
        assert_eq!(report.enabled_dmx_output_count, 1);
        assert_eq!(report.dmx_preview_universe_count, 1);
        assert_eq!(report.dmx_outputs[1].protocol, DmxOutputProtocol::Sacn);
        assert_eq!(report.budget.overall, TelemetryBudgetStatus::Pass);
        assert_eq!(
            report.budget.command_to_dmx_p99_target_us,
            TELEMETRY_COMMAND_TO_DMX_P99_TARGET_US
        );
        assert!(report
            .budget
            .checks
            .iter()
            .any(|check| check.name == "command_to_dmx_tick_p99"
                && check.status == TelemetryBudgetStatus::Pass
                && check.measured_us == Some(4_500)));
    }

    #[test]
    fn engine_telemetry_budget_report_flags_latency_and_output_failures() {
        let mut telemetry = EngineTelemetry::default();
        telemetry.tick_jitter_samples = 10;
        telemetry.tick_jitter_p99_us = TELEMETRY_TICK_JITTER_P99_TARGET_US + 1;
        telemetry.command_queue_latency_samples = 1;
        telemetry.command_queue_latency_p99_us = TELEMETRY_COMMAND_QUEUE_P99_TARGET_US;
        telemetry.command_to_dmx_tick_latency_samples = 1;
        telemetry.command_to_dmx_tick_latency_p99_us = TELEMETRY_COMMAND_TO_DMX_P99_TARGET_US + 1;
        telemetry.last_dmx_output_count = 1;
        telemetry.last_dmx_send_success_count = 0;
        telemetry.last_dmx_send_failure_count = 1;
        telemetry.total_dmx_send_failure_count = 1;
        telemetry.dmx_send_interval_samples = 1;
        telemetry.last_dmx_send_interval_us = TELEMETRY_DMX_TARGET_TICK_INTERVAL_US;
        telemetry.dmx_send_interval_min_us = TELEMETRY_DMX_TARGET_TICK_INTERVAL_US;

        let budget = engine_telemetry_budget_report(&telemetry, 1);

        assert_eq!(budget.overall, TelemetryBudgetStatus::Fail);
        assert!(budget.checks.iter().any(|check| {
            check.name == "tick_jitter_p99" && check.status == TelemetryBudgetStatus::Fail
        }));
        assert!(budget.checks.iter().any(|check| {
            check.name == "dmx_send_success" && check.status == TelemetryBudgetStatus::Fail
        }));
    }

    #[test]
    fn engine_telemetry_budget_report_warns_on_slow_dmx_interval() {
        let mut telemetry = EngineTelemetry::default();
        telemetry.tick_jitter_samples = 10;
        telemetry.tick_jitter_p99_us = TELEMETRY_TICK_JITTER_P99_TARGET_US;
        telemetry.command_queue_latency_samples = 1;
        telemetry.command_queue_latency_p99_us = TELEMETRY_COMMAND_QUEUE_P99_TARGET_US;
        telemetry.command_to_dmx_tick_latency_samples = 1;
        telemetry.command_to_dmx_tick_latency_p99_us = TELEMETRY_COMMAND_TO_DMX_P99_TARGET_US;
        telemetry.last_dmx_output_count = 1;
        telemetry.last_dmx_send_success_count = 1;
        telemetry.dmx_send_interval_samples = 3;
        telemetry.dmx_send_interval_min_us = TELEMETRY_DMX_TARGET_TICK_INTERVAL_US;
        telemetry.last_dmx_send_interval_us =
            TELEMETRY_DMX_TARGET_TICK_INTERVAL_US + TELEMETRY_DMX_SEND_INTERVAL_TOLERANCE_US + 1;

        let budget = engine_telemetry_budget_report(&telemetry, 1);

        assert_eq!(budget.overall, TelemetryBudgetStatus::Warn);
        assert!(budget.checks.iter().any(|check| {
            check.name == "dmx_send_interval" && check.status == TelemetryBudgetStatus::Warn
        }));
    }

    #[test]
    fn sends_dmx_test_frame_to_artnet_loopback() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let config = DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port,
            universe: 7,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        };
        let frame = build_dmx_test_frame(5, 2, 180).unwrap();

        let result = send_dmx_config_test_frame(&config, 5, 2, 180, &frame).unwrap();

        let mut buffer = [0u8; 600];
        let (received, _) = receiver.recv_from(&mut buffer).unwrap();
        let packet = io::artnet::parse_art_dmx_packet(&buffer[..received]).unwrap();
        assert_eq!(result.bytes, received);
        assert_eq!(result.protocol, DmxOutputProtocol::ArtNet);
        assert_eq!(result.universe, 7);
        assert_eq!(packet.universe, 7);
        assert_eq!(packet.data[3], 0);
        assert_eq!(&packet.data[4..6], &[180, 180]);
        assert_eq!(packet.data[6], 0);
    }

    #[test]
    fn sends_dmx_test_frame_to_enabled_artnet_routes() {
        let receiver_a = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver_a
            .set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        let receiver_b = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver_b
            .set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        let disabled_receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        disabled_receiver
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let first_port = receiver_a.local_addr().unwrap().port();
        let second_port = receiver_b.local_addr().unwrap().port();
        let disabled_port = disabled_receiver.local_addr().unwrap().port();
        let routes = vec![
            DmxOutputConfig {
                enabled: true,
                protocol: DmxOutputProtocol::ArtNet,
                target_ip: "127.0.0.1".to_string(),
                port: first_port,
                universe: 1,
                serial_port: String::new(),
                serial_baud_rate: 57_600,
            },
            DmxOutputConfig {
                enabled: true,
                protocol: DmxOutputProtocol::ArtNet,
                target_ip: "127.0.0.1".to_string(),
                port: second_port,
                universe: 2,
                serial_port: String::new(),
                serial_baud_rate: 57_600,
            },
            DmxOutputConfig {
                enabled: false,
                protocol: DmxOutputProtocol::ArtNet,
                target_ip: "127.0.0.1".to_string(),
                port: disabled_port,
                universe: 3,
                serial_port: String::new(),
                serial_baud_rate: 57_600,
            },
        ];
        let frame = build_dmx_test_frame(12, 3, 64).unwrap();

        let results = send_dmx_route_test_frames(&routes, 12, 3, 64, &frame).unwrap();

        assert_eq!(results.len(), 2);
        for (receiver, universe) in [(&receiver_a, 1), (&receiver_b, 2)] {
            let mut buffer = [0u8; 600];
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = io::artnet::parse_art_dmx_packet(&buffer[..received]).unwrap();
            assert_eq!(packet.universe, universe);
            assert_eq!(packet.data[10], 0);
            assert_eq!(&packet.data[11..14], &[64, 64, 64]);
            assert_eq!(packet.data[14], 0);
        }
        let mut disabled_buffer = [0u8; 600];
        assert!(disabled_receiver.recv_from(&mut disabled_buffer).is_err());
    }

    #[test]
    fn sends_dmx_test_frame_to_enabled_sacn_routes() {
        let receiver_a = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver_a
            .set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        let receiver_b = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver_b
            .set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        let disabled_receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        disabled_receiver
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let first_port = receiver_a.local_addr().unwrap().port();
        let second_port = receiver_b.local_addr().unwrap().port();
        let disabled_port = disabled_receiver.local_addr().unwrap().port();
        let routes = vec![
            DmxOutputConfig {
                enabled: true,
                protocol: DmxOutputProtocol::Sacn,
                target_ip: "127.0.0.1".to_string(),
                port: first_port,
                universe: 4,
                serial_port: String::new(),
                serial_baud_rate: 57_600,
            },
            DmxOutputConfig {
                enabled: true,
                protocol: DmxOutputProtocol::Sacn,
                target_ip: "127.0.0.1".to_string(),
                port: second_port,
                universe: 5,
                serial_port: String::new(),
                serial_baud_rate: 57_600,
            },
            DmxOutputConfig {
                enabled: false,
                protocol: DmxOutputProtocol::Sacn,
                target_ip: "127.0.0.1".to_string(),
                port: disabled_port,
                universe: 6,
                serial_port: String::new(),
                serial_baud_rate: 57_600,
            },
        ];
        let frame = build_dmx_test_frame(20, 4, 128).unwrap();

        let results = send_dmx_route_test_frames(&routes, 20, 4, 128, &frame).unwrap();

        assert_eq!(results.len(), 2);
        for ((receiver, universe), result) in [(&receiver_a, 4), (&receiver_b, 5)]
            .into_iter()
            .zip(results.iter())
        {
            let mut buffer = [0u8; 700];
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = io::sacn::parse_sacn_dmx_packet(&buffer[..received]).unwrap();
            assert_eq!(result.bytes, received);
            assert_eq!(result.protocol, DmxOutputProtocol::Sacn);
            assert_eq!(result.universe, universe);
            assert_eq!(packet.universe, universe);
            assert_eq!(packet.data[18], 0);
            assert_eq!(&packet.data[19..23], &[128, 128, 128, 128]);
            assert_eq!(packet.data[23], 0);
        }
        let mut disabled_buffer = [0u8; 700];
        assert!(disabled_receiver.recv_from(&mut disabled_buffer).is_err());
    }

    #[test]
    fn dmx_route_test_frame_validates_all_routes_before_sending_any_frame() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let valid_port = receiver.local_addr().unwrap().port();
        let routes = vec![
            DmxOutputConfig {
                enabled: true,
                protocol: DmxOutputProtocol::ArtNet,
                target_ip: "127.0.0.1".to_string(),
                port: valid_port,
                universe: 1,
                serial_port: String::new(),
                serial_baud_rate: 57_600,
            },
            DmxOutputConfig {
                enabled: true,
                protocol: DmxOutputProtocol::ArtNet,
                target_ip: " ".to_string(),
                port: 6454,
                universe: 2,
                serial_port: String::new(),
                serial_baud_rate: 57_600,
            },
        ];
        let frame = build_dmx_test_frame(1, 1, 255).unwrap();

        let error = send_dmx_route_test_frames(&routes, 1, 1, 255, &frame).unwrap_err();

        assert!(error.contains("target IP"));
        let mut buffer = [0u8; 600];
        assert!(receiver.recv_from(&mut buffer).is_err());
    }

    #[test]
    fn dmx_route_test_frame_requires_enabled_route() {
        let frame = build_dmx_test_frame(1, 1, 255).unwrap();
        let routes = vec![DmxOutputConfig {
            enabled: false,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port: 6454,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        }];

        assert!(send_dmx_route_test_frames(&routes, 1, 1, 255, &frame)
            .unwrap_err()
            .contains("at least one DMX output route"));
    }

    #[test]
    fn normalize_group_ids_trims_and_deduplicates() {
        assert_eq!(
            normalize_group_ids(vec![
                " front ".to_string(),
                "movers".to_string(),
                "front".to_string(),
                " front / movers ".to_string(),
            ])
            .unwrap(),
            vec![
                "front".to_string(),
                "movers".to_string(),
                "front/movers".to_string()
            ]
        );
        assert!(normalize_group_ids(vec![" ".to_string()])
            .unwrap_err()
            .contains("must not be empty"));
        assert!(normalize_group_ids(vec!["front//movers".to_string()])
            .unwrap_err()
            .contains("segments"));
    }

    #[test]
    fn normalize_control_group_id_trims_and_rejects_empty() {
        assert_eq!(
            normalize_control_group_id(" front / movers ".to_string()).unwrap(),
            "front/movers"
        );
        assert!(normalize_control_group_id(" ".to_string())
            .unwrap_err()
            .contains("Group ID"));
    }

    #[test]
    fn normalize_attribute_name_trims_and_rejects_empty() {
        assert_eq!(
            normalize_attribute_name(" Dimmer ".to_string()).unwrap(),
            "Dimmer"
        );
        assert!(normalize_attribute_name(" ".to_string())
            .unwrap_err()
            .contains("Attribute"));
    }

    #[test]
    fn group_matches_treats_slash_paths_as_hierarchy_only() {
        assert!(group_matches("front/movers", "front"));
        assert!(group_matches("front / movers", "front/movers"));
        assert!(!group_matches("frontline", "front"));
        assert!(!group_matches("front", "front/movers"));
    }

    #[test]
    fn remote_access_urls_include_lan_address_for_wildcard_bind() {
        let urls = build_remote_access_urls(
            &RemoteControlConfig {
                bind_ip: "0.0.0.0".to_string(),
                port: 9_100,
            },
            Some("192.168.1.24".parse().unwrap()),
        );

        assert_eq!(
            urls,
            vec![
                "http://192.168.1.24:9100/remote".to_string(),
                "http://localhost:9100/remote".to_string()
            ]
        );
    }

    #[test]
    fn remote_access_urls_format_specific_ipv6_bind() {
        let urls = build_remote_access_urls(
            &RemoteControlConfig {
                bind_ip: "fe80::1".to_string(),
                port: 9_101,
            },
            None,
        );

        assert_eq!(urls, vec!["http://[fe80::1]:9101/remote".to_string()]);
    }

    #[test]
    fn normalize_video_layer_label_trims_and_rejects_empty() {
        assert_eq!(
            normalize_video_layer_label("  Clip A  ".to_string()).unwrap(),
            "Clip A"
        );
        assert!(normalize_video_layer_label(" ".to_string())
            .unwrap_err()
            .contains("Video layer label"));
    }

    #[test]
    fn validate_existing_file_path_trims_and_rejects_missing_or_directory() {
        let file_path = std::env::temp_dir().join(format!(
            "rayard-existing-file-path-{}.mov",
            std::process::id()
        ));
        fs::write(&file_path, b"test media").unwrap();
        let file_path_string = file_path.to_string_lossy().to_string();

        assert_eq!(
            validate_existing_file_path(format!("  {file_path_string}  "), "Video file").unwrap(),
            file_path_string
        );

        let missing_path = file_path.with_extension("missing");
        let missing_path_string = missing_path.to_string_lossy().to_string();
        let missing_error =
            validate_existing_file_path(missing_path_string.clone(), "Video file").unwrap_err();
        assert!(missing_error.contains("Video file path was not found"));
        assert!(missing_error.contains(&missing_path_string));

        let directory_error = validate_existing_file_path(
            std::env::temp_dir().to_string_lossy().to_string(),
            "Still image",
        )
        .unwrap_err();
        assert!(directory_error.contains("Still image path must be a file"));

        let _ = fs::remove_file(file_path);
    }

    #[test]
    fn import_gdtf_rejects_blank_path_before_opening() {
        let error = import_gdtf_from_path("   ".to_string()).unwrap_err();

        assert!(error.contains("GDTF file path is required"));
    }

    #[test]
    fn patch_profile_loader_uses_project_cache_when_original_file_is_missing() {
        let profiles = Mutex::new(HashMap::new());
        let mut profile = project_custom_profile();
        profile.source_path = "C:/missing/fixture.gdtf".to_string();
        profile.manufacturer = "Robe".to_string();
        profile.name = "Cached Spot".to_string();
        cache_fixture_profile(&profiles, &profile).unwrap();

        let loaded =
            load_patch_profile_from_cache_or_file(&profiles, "C:/missing/fixture.gdtf").unwrap();

        assert_eq!(loaded.manufacturer, "Robe");
        assert_eq!(loaded.name, "Cached Spot");
        assert_eq!(loaded.source_path, "C:/missing/fixture.gdtf");
    }

    #[test]
    fn validate_video_cue_point_color_accepts_hex_only() {
        validate_video_cue_point_color("#ff3366").unwrap();
        validate_video_cue_point_color("FF3366").unwrap();
        assert!(validate_video_cue_point_color("#ff36").is_err());
        assert!(validate_video_cue_point_color("red").is_err());
    }

    #[test]
    fn video_source_file_dialog_filter_accepts_local_media_kinds_only() {
        let (video_label, video_exts) =
            video_source_file_dialog_filter(&VideoSourceKind::File).unwrap();
        assert_eq!(video_label, "Video Files");
        assert!(video_exts.contains(&"mp4"));
        assert!(video_exts.contains(&"mov"));

        let (image_label, image_exts) =
            video_source_file_dialog_filter(&VideoSourceKind::StillImage).unwrap();
        assert_eq!(image_label, "Still Images");
        assert!(image_exts.contains(&"png"));
        assert!(image_exts.contains(&"jpg"));

        assert!(video_source_file_dialog_filter(&VideoSourceKind::Ndi)
            .unwrap_err()
            .contains("local media"));
    }

    #[test]
    fn video_preview_queue_expectation_accounts_for_stills_and_prefetch() {
        let mut layer = VideoLayerSummary {
            id: 1,
            label: "Clip".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some("clip.mp4".to_string()),
                name: None,
                codec: None,
                metadata: None,
            },
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
        };

        assert_eq!(expected_video_preview_queue_len(&layer, 0), 1);
        assert_eq!(expected_video_preview_queue_len(&layer, 3), 4);

        layer.source.kind = VideoSourceKind::StillImage;
        layer.source.path = Some("still.png".to_string());

        assert_eq!(expected_video_preview_queue_len(&layer, 3), 1);
    }

    #[test]
    fn video_preview_decode_budget_scales_by_layers_and_prefetch() {
        assert_eq!(video_preview_decode_budget_from_prefetch(0, 0), 1);
        assert_eq!(video_preview_decode_budget_from_prefetch(2, 0), 2);
        assert_eq!(video_preview_decode_budget_from_prefetch(3, 2), 9);
    }

    #[test]
    fn video_preview_decode_budget_request_is_clamped() {
        let renderer = video::VideoPreviewRenderer::with_frame_provider(
            video::VideoRuntimeConfig::default(),
            video::DecoderBackedFrameProvider::new(video::FfmpegCliFrameDecoder::from_env())
                .with_prefetch(2, 33),
        );

        assert_eq!(resolved_video_preview_decode_budget(&renderer, 3, None), 9);
        assert_eq!(
            resolved_video_preview_decode_budget(&renderer, 3, Some(0)),
            1
        );
        assert_eq!(
            resolved_video_preview_decode_budget(&renderer, 3, Some(1)),
            1
        );
        assert_eq!(
            resolved_video_preview_decode_budget(&renderer, 3, Some(99)),
            9
        );
    }

    #[test]
    fn video_output_decode_preview_summaries_report_schedulable_requests() {
        let snapshot = protocol::VideoSnapshot {
            layers: vec![
                VideoLayerSummary {
                    id: 2,
                    label: "Clip".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::File,
                        path: Some("clip.mp4".to_string()),
                        name: None,
                        codec: Some("H264".to_string()),
                        metadata: None,
                    },
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        position_ms: 100,
                        playing: true,
                        ..VideoLayerState::default()
                    },
                },
                VideoLayerSummary {
                    id: 3,
                    label: "Still".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::StillImage,
                        path: Some("still.png".to_string()),
                        name: None,
                        codec: None,
                        metadata: None,
                    },
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState::default(),
                },
            ],
            compositions: vec![CompositionSummary {
                id: 5,
                label: "Main".to_string(),
                layer_ids: vec![2, 3],
                output_ids: vec![9],
            }],
            outputs: vec![VideoOutputSummary {
                id: 9,
                label: "Projector".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 5,
                fullscreen: false,
                monitor_id: Some(0),
                width: 1280,
                height: 720,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: VideoOutputMapping::default(),
            }],
            mapping_presets: Vec::new(),
            master_opacity: 1.0,
            blackout: false,
        };

        let summaries = video_output_decode_preview_summaries(
            &snapshot,
            video::VideoRuntimeConfig {
                frame_queue_capacity: 4,
                preview_width: 64,
                preview_height: 36,
            },
            2,
            40,
            None,
        );

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].output_id, 9);
        assert!(summaries[0].error.is_none());
        let report = summaries[0].report.unwrap();
        assert_eq!(report.layers_considered, 2);
        assert_eq!(report.requests_attempted, 3);
        assert_eq!(report.inserted, 3);
        assert_eq!(report.pending, 3);
    }

    #[test]
    fn video_output_window_label_separates_live_and_test_pattern_windows() {
        assert_eq!(video_output_window_label(3, false), "video-output-3");
        assert_eq!(
            video_output_window_label(3, true),
            "video-output-3-test-pattern"
        );
    }

    #[test]
    fn video_output_config_normalization_trims_and_separates_output_fields() {
        let display = normalize_video_output_config(
            "  Projector  ".to_string(),
            VideoOutputKind::Display,
            1920,
            1080,
            true,
            Some(2),
            Some("Ignored".to_string()),
        )
        .unwrap();
        assert_eq!(display.label, "Projector");
        assert!(display.fullscreen);
        assert_eq!(display.monitor_id, Some(2));
        assert_eq!(display.endpoint_name, None);

        let ndi = normalize_video_output_config(
            "  Main NDI  ".to_string(),
            VideoOutputKind::NdiSender,
            1280,
            720,
            true,
            Some(5),
            Some("  Stage Feed  ".to_string()),
        )
        .unwrap();
        assert_eq!(ndi.label, "Main NDI");
        assert!(!ndi.fullscreen);
        assert_eq!(ndi.monitor_id, None);
        assert_eq!(ndi.endpoint_name.as_deref(), Some("Stage Feed"));

        assert!(normalize_video_output_config(
            " ".to_string(),
            VideoOutputKind::Display,
            1920,
            1080,
            true,
            None,
            None,
        )
        .unwrap_err()
        .contains("label"));
        assert!(normalize_video_output_config(
            "Projector".to_string(),
            VideoOutputKind::Display,
            0,
            1080,
            true,
            None,
            None,
        )
        .unwrap_err()
        .contains("greater than 0"));
    }

    #[test]
    fn external_video_backend_validation_blocks_unavailable_io() {
        let status = VideoRuntimeStatus {
            backends: vec![
                protocol::VideoBackendStatus {
                    id: "ndi".to_string(),
                    label: "NDI input/output".to_string(),
                    state: VideoBackendState::NotBuilt,
                    detail: "NDI SDK backend is not linked in this build".to_string(),
                },
                protocol::VideoBackendStatus {
                    id: "spout".to_string(),
                    label: "Spout input/output".to_string(),
                    state: VideoBackendState::Available,
                    detail: "Spout backend loaded".to_string(),
                },
            ],
        };

        validate_video_backend_available(&status, "spout", "Spout output").unwrap();
        let error = validate_video_backend_available(&status, "ndi", "NDI input").unwrap_err();
        assert!(error.contains("NDI input is unavailable"));
        assert!(error.contains("not linked"));
        assert!(
            validate_video_backend_available(&status, "syphon", "Syphon output")
                .unwrap_err()
                .contains("backend status is unavailable")
        );

        assert_eq!(
            video_input_backend(&VideoSourceKind::Ndi),
            Some(("ndi", "NDI input"))
        );
        assert_eq!(
            video_output_backend(&VideoOutputKind::NdiSender),
            Some(("ndi", "NDI output"))
        );
        assert_eq!(video_output_backend(&VideoOutputKind::Display), None);
    }

    #[test]
    fn external_video_transport_driver_events_are_bounded_and_sequenced() {
        let route = video::ExternalVideoTransportRoute {
            route_id: 42,
            direction: video::ExternalVideoTransportDirection::Input,
            backend_id: "ndi".to_string(),
            label: "Stage NDI".to_string(),
            endpoint_name: "Stage".to_string(),
        };
        let mut events = Vec::new();
        let mut driver = RecordingExternalVideoTransportDriver {
            events: &mut events,
        };

        for _ in 0..(EXTERNAL_VIDEO_TRANSPORT_EVENT_LIMIT + 4) {
            video::ExternalVideoTransportDriver::start_route(&mut driver, &route).unwrap();
        }

        assert_eq!(driver.events.len(), EXTERNAL_VIDEO_TRANSPORT_EVENT_LIMIT);
        assert_eq!(driver.events.first().unwrap().sequence, 5);
        assert_eq!(
            driver.events.last().unwrap().sequence,
            (EXTERNAL_VIDEO_TRANSPORT_EVENT_LIMIT + 4) as u64
        );
        assert_eq!(
            driver.events.last().unwrap().action,
            ExternalVideoTransportDriverAction::Start
        );
    }

    #[test]
    fn project_external_video_validation_allows_unavailable_backends_but_requires_names() {
        let mut project = project_with_valid_video_graph();
        project.snapshot.video.layers[0].source = VideoSourceSummary {
            kind: VideoSourceKind::Ndi,
            path: None,
            name: Some("Stage NDI".to_string()),
            codec: None,
            metadata: None,
        };

        validate_project_external_video_backends(&project.snapshot).unwrap();

        project.snapshot.video.layers[0].source.name = Some(" ".to_string());
        assert!(validate_project_external_video_backends(&project.snapshot)
            .unwrap_err()
            .contains("requires a source name"));

        let mut project = project_with_valid_video_graph();
        project.snapshot.video.outputs[0].kind = VideoOutputKind::NdiSender;
        project.snapshot.video.outputs[0].endpoint_name = None;
        assert!(validate_project_external_video_backends(&project.snapshot)
            .unwrap_err()
            .contains("requires an endpoint name"));
    }

    #[test]
    fn project_file_keeps_external_video_routes_as_runtime_diagnostics() {
        let mut project = project_with_valid_video_graph();
        project.snapshot.video.layers[0].source = VideoSourceSummary {
            kind: VideoSourceKind::Ndi,
            path: None,
            name: Some("Stage NDI".to_string()),
            codec: None,
            metadata: None,
        };
        project.snapshot.video.outputs[0].kind = VideoOutputKind::NdiSender;
        project.snapshot.video.outputs[0].endpoint_name = Some("Program Out".to_string());

        validate_project_file(&project).unwrap();

        let plans = video::build_external_video_io_route_plans(
            &project.snapshot.video,
            &video::video_runtime_status(),
        );
        assert_eq!(plans.inputs.len(), 1);
        assert_eq!(plans.outputs.len(), 1);
        assert!(!plans.inputs[0].ready);
        assert!(!plans.outputs[0].ready);
        assert!(plans.inputs[0]
            .issue
            .as_deref()
            .unwrap_or_default()
            .contains("NDI"));
        assert!(plans.outputs[0]
            .issue
            .as_deref()
            .unwrap_or_default()
            .contains("NDI"));
    }

    #[test]
    fn video_composition_boundary_validation_rejects_main_and_missing_ids() {
        let snapshot = EngineSnapshot {
            video: protocol::VideoSnapshot {
                compositions: vec![
                    CompositionSummary {
                        id: 1,
                        label: "Main".to_string(),
                        layer_ids: Vec::new(),
                        output_ids: Vec::new(),
                    },
                    CompositionSummary {
                        id: 7,
                        label: "Aux".to_string(),
                        layer_ids: Vec::new(),
                        output_ids: Vec::new(),
                    },
                ],
                ..protocol::VideoSnapshot::default()
            },
            ..EngineSnapshot::default()
        };

        validate_video_composition_exists(&snapshot, 1).unwrap();
        validate_editable_video_composition(&snapshot, 7).unwrap();
        assert!(validate_editable_video_composition(&snapshot, 1)
            .unwrap_err()
            .contains("Main video composition"));
        assert_eq!(
            validate_editable_video_composition(&snapshot, 99).unwrap_err(),
            "Video composition 99 was not found"
        );
    }

    #[test]
    fn video_output_boundary_validation_rejects_missing_ids() {
        let snapshot = EngineSnapshot {
            video: protocol::VideoSnapshot {
                outputs: vec![VideoOutputSummary {
                    id: 5,
                    label: "Projector".to_string(),
                    kind: VideoOutputKind::Display,
                    enabled: true,
                    composition_id: 1,
                    fullscreen: true,
                    monitor_id: Some(0),
                    width: 1920,
                    height: 1080,
                    endpoint_name: None,
                    opacity: 1.0,
                    blackout: false,
                    mapping: VideoOutputMapping::default(),
                }],
                ..protocol::VideoSnapshot::default()
            },
            ..EngineSnapshot::default()
        };

        validate_video_output_exists(&snapshot, 5).unwrap();
        assert_eq!(
            validate_video_output_exists(&snapshot, 99).unwrap_err(),
            "Video output 99 was not found"
        );
    }

    #[test]
    fn video_output_mapping_validation_rejects_invalid_ratio() {
        let valid = VideoOutputMapping {
            aspect_ratio: 1.33,
            keystone_x: 0.25,
            ..Default::default()
        };
        validate_video_output_mapping(&valid).unwrap();

        let invalid = VideoOutputMapping {
            aspect_ratio: 0.0,
            ..Default::default()
        };
        assert!(validate_video_output_mapping(&invalid)
            .unwrap_err()
            .contains("aspect ratio"));
    }

    #[test]
    fn video_output_mapping_field_is_canonicalized_and_required() {
        assert_eq!(
            normalize_video_output_mapping_field("  keystone_x  ".to_string()).unwrap(),
            "keystone_x"
        );
        assert_eq!(
            normalize_video_output_mapping_field("key y".to_string()).unwrap(),
            "keystone_y"
        );
        assert!(normalize_video_output_mapping_field(" ".to_string())
            .unwrap_err()
            .contains("field"));
        assert!(normalize_video_output_mapping_field("unknown".to_string())
            .unwrap_err()
            .contains("not supported"));
    }

    #[test]
    fn video_output_mapping_preset_label_is_trimmed_and_required() {
        assert_eq!(
            normalize_video_output_mapping_preset_label("  Front Projector  ".to_string()).unwrap(),
            "Front Projector"
        );
        assert!(normalize_video_output_mapping_preset_label(" ".to_string())
            .unwrap_err()
            .contains("label"));
    }

    #[test]
    fn video_output_mapping_preset_file_serializes_and_validates() {
        let file = VideoOutputMappingPresetFile {
            version: 1,
            app: APP_NAME.to_string(),
            preset: VideoOutputMappingPresetSummary {
                label: "Front Projector".to_string(),
                mapping: VideoOutputMapping {
                    stage_x: 2.5,
                    stage_z: -1.0,
                    aspect_ratio: 16.0 / 9.0,
                    lens_distortion: 0.25,
                    ..Default::default()
                },
            },
        };

        validate_video_output_mapping_preset_file(&file).unwrap();
        let json = serde_json::to_string_pretty(&file).unwrap();
        let parsed: VideoOutputMappingPresetFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.preset.label, "Front Projector");
        assert!((parsed.preset.mapping.stage_x - 2.5).abs() < f32::EPSILON);
        assert!((parsed.preset.mapping.stage_z + 1.0).abs() < f32::EPSILON);
        assert!((parsed.preset.mapping.aspect_ratio - 16.0 / 9.0).abs() < f32::EPSILON);

        let unsupported = VideoOutputMappingPresetFile {
            version: 2,
            ..file.clone()
        };
        assert!(validate_video_output_mapping_preset_file(&unsupported)
            .unwrap_err()
            .contains("version"));

        let wrong_app = VideoOutputMappingPresetFile {
            app: "Other".to_string(),
            ..file
        };
        assert!(validate_video_output_mapping_preset_file(&wrong_app)
            .unwrap_err()
            .contains("Unsupported projector map app"));
    }

    #[test]
    fn project_file_serializes_snapshot_as_pretty_json() {
        let project = ProjectFile {
            version: 1,
            app: APP_NAME.to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot {
                blackout: true,
                timeline: protocol::TimelineSnapshot {
                    audio: Some(AudioAnalysisSummary {
                        path: "C:/media/show.wav".to_string(),
                        sample_rate: 48_000,
                        channels: 2,
                        duration_ms: 5_000,
                        estimated_bpm: Some(120.0),
                        waveform: vec![protocol::AudioWaveformPoint {
                            time_ms: 0,
                            peak: 0.5,
                            rms: 0.25,
                        }],
                        beats: vec![0, 500, 1_000],
                    }),
                    ..protocol::TimelineSnapshot::default()
                },
                ..EngineSnapshot::default()
            },
        };

        let json = serde_json::to_string_pretty(&project).unwrap();
        let parsed: ProjectFile = serde_json::from_str(&json).unwrap();

        assert!(json.contains("\"version\": 1"));
        assert_eq!(parsed.app, APP_NAME);
        assert!(parsed.snapshot.blackout);
        assert_eq!(parsed.snapshot.dmx_outputs.len(), 1);
        assert_eq!(
            parsed
                .snapshot
                .timeline
                .audio
                .as_ref()
                .and_then(|audio| audio.estimated_bpm),
            Some(120.0)
        );
    }

    #[test]
    fn project_snapshot_for_save_drops_volatile_runtime_state() {
        let mut snapshot = EngineSnapshot::default();
        snapshot.active_cue_id = Some(7);
        snapshot.active_fade = Some(protocol::ActiveFadeSummary {
            cue_id: 7,
            progress: 0.5,
            remaining_ms: 1_200,
            paused: true,
        });
        snapshot.timeline.playing = true;
        snapshot.timeline.position_ms = 12_345;
        snapshot.video.master_opacity = 0.5;
        snapshot.dmx_preview = vec![127; 512];
        snapshot.dmx_previews = vec![protocol::DmxUniversePreview {
            universe: 2,
            values: vec![255; 512],
        }];
        snapshot.telemetry.frame_counter = 99;
        snapshot.telemetry.tick_jitter_p99_us = 850;
        snapshot.telemetry.last_error = Some("send failed".to_string());

        let saved = project_snapshot_for_save(snapshot);

        assert_eq!(saved.active_cue_id, Some(7));
        assert_eq!(saved.active_fade, None);
        assert!(!saved.timeline.playing);
        assert_eq!(saved.timeline.position_ms, 12_345);
        assert_eq!(saved.video.master_opacity, 0.5);
        assert!(saved.dmx_preview.is_empty());
        assert!(saved.dmx_previews.is_empty());
        assert_eq!(saved.telemetry, EngineTelemetry::default());
    }

    #[test]
    fn startup_project_path_from_args_uses_first_ry_argument() {
        let args = vec![
            OsString::from("rayard.exe"),
            OsString::from("--ignored"),
            OsString::from("C:/shows/opening.RY"),
            OsString::from("C:/shows/backup.ry"),
        ];

        let path = startup_project_path_from_args(args).unwrap();

        assert_eq!(path, PathBuf::from("C:/shows/opening.RY"));
    }

    #[test]
    fn startup_project_path_from_args_ignores_non_project_arguments() {
        let args = vec![
            OsString::from("rayard.exe"),
            OsString::from("C:/shows/opening.json"),
            OsString::from("--profile"),
        ];

        assert_eq!(startup_project_path_from_args(args), None);
    }

    #[test]
    fn single_instance_project_paths_collect_ry_arguments() {
        let args = vec![
            "rayard.exe".to_string(),
            "--ignored".to_string(),
            "C:/shows/opening.RY".to_string(),
            "C:/shows/notes.txt".to_string(),
            "C:/shows/backup.ry".to_string(),
        ];

        let paths = project_paths_from_single_instance_args(args, "");

        assert_eq!(paths.len(), 2);
        assert!(paths[0].ends_with("opening.RY"));
        assert!(paths[1].ends_with("backup.ry"));
    }

    #[test]
    fn single_instance_project_paths_resolve_relative_arguments_with_cwd() {
        let cwd = PathBuf::from("C:/shows");
        let args = vec!["rayard.exe".to_string(), "looks/strobe.ry".to_string()];

        let paths = project_paths_from_single_instance_args(args, &cwd.to_string_lossy());

        assert_eq!(paths, vec![cwd.join("looks/strobe.ry").to_string_lossy()]);
    }

    #[test]
    fn normalize_project_save_path_adds_ry_extension_when_missing() {
        let path = normalize_project_save_path(PathBuf::from("C:/shows/opening")).unwrap();

        assert_eq!(path, PathBuf::from("C:/shows/opening.ry"));
    }

    #[test]
    fn normalize_project_save_path_rejects_non_rayard_extension() {
        let error =
            normalize_project_save_path(PathBuf::from("C:/shows/opening.json")).unwrap_err();

        assert!(error.contains(".ry extension"));
    }

    #[test]
    fn normalize_gdtf_save_path_adds_gdtf_extension_when_missing() {
        let path = normalize_gdtf_save_path(PathBuf::from("C:/fixtures/mini-spot")).unwrap();

        assert_eq!(path, PathBuf::from("C:/fixtures/mini-spot.gdtf"));
    }

    #[test]
    fn normalize_gdtf_save_path_rejects_non_gdtf_extension() {
        let error =
            normalize_gdtf_save_path(PathBuf::from("C:/fixtures/mini-spot.zip")).unwrap_err();

        assert!(error.contains(".gdtf extension"));
    }

    #[test]
    fn normalize_custom_fixture_profile_save_path_adds_fixture_extension_when_missing() {
        let path =
            normalize_custom_fixture_profile_save_path(PathBuf::from("C:/fixtures/mini-bar"))
                .unwrap();

        assert_eq!(path, PathBuf::from("C:/fixtures/mini-bar.fixture"));
    }

    #[test]
    fn normalize_custom_fixture_profile_save_path_rejects_non_fixture_extension() {
        let error =
            normalize_custom_fixture_profile_save_path(PathBuf::from("C:/fixtures/mini-bar.json"))
                .unwrap_err();

        assert!(error.contains(".fixture extension"));
    }

    #[test]
    fn normalize_fixture_preset_save_path_adds_preset_extension_when_missing() {
        let path =
            normalize_fixture_preset_save_path(PathBuf::from("C:/shows/looks/strobe")).unwrap();

        assert_eq!(path, PathBuf::from("C:/shows/looks/strobe.preset"));
    }

    #[test]
    fn normalize_fixture_preset_save_path_rejects_non_preset_extension() {
        let error = normalize_fixture_preset_save_path(PathBuf::from("C:/shows/looks/strobe.json"))
            .unwrap_err();

        assert!(error.contains(".preset extension"));
    }

    #[test]
    fn project_file_validation_rejects_wrong_version_or_app() {
        let mut project = ProjectFile {
            version: 2,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot::default(),
        };

        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("Unsupported project version"));

        project.version = 1;
        validate_project_file(&project).unwrap();

        project.app = "Other".to_string();
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("Unsupported project app"));
    }

    #[test]
    fn project_file_validation_rejects_duplicate_ids() {
        assert!(validate_unique_ids("cue", vec![1, 2, 3]).is_ok());
        assert_eq!(
            validate_unique_ids("cue", vec![1, 2, 1]).unwrap_err(),
            "Project contains duplicate cue id 1"
        );

        let mut project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot::default(),
        };
        project.snapshot.cues = vec![
            protocol::CueSummary {
                id: 7,
                label: "First".to_string(),
                fade_ms: 0,
                targets: Vec::new(),
                video_targets: Vec::new(),
                video_output_targets: Vec::new(),

                node_graph_targets: Vec::new(),
            },
            protocol::CueSummary {
                id: 7,
                label: "Duplicate".to_string(),
                fade_ms: 0,
                targets: Vec::new(),
                video_targets: Vec::new(),
                video_output_targets: Vec::new(),

                node_graph_targets: Vec::new(),
            },
        ];

        assert_eq!(
            validate_project_file(&project).unwrap_err(),
            "Project contains duplicate cue id 7"
        );
    }

    fn project_video_layer(id: VideoLayerId) -> protocol::VideoLayerSummary {
        protocol::VideoLayerSummary {
            id,
            label: format!("Layer {id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::StillImage,
                path: Some(format!("memory://layer-{id}.png")),
                name: None,
                codec: None,
                metadata: None,
            },
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
        }
    }

    fn project_video_output(
        id: VideoOutputId,
        composition_id: CompositionId,
    ) -> VideoOutputSummary {
        VideoOutputSummary {
            id,
            label: format!("Output {id}"),
            kind: VideoOutputKind::Display,
            enabled: true,
            composition_id,
            fullscreen: true,
            monitor_id: Some(0),
            width: 1920,
            height: 1080,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: VideoOutputMapping::default(),
        }
    }

    fn project_custom_profile() -> FixtureProfileSummary {
        FixtureProfileSummary {
            source_path: "memory://custom/Rayard-Custom_Bar".to_string(),
            manufacturer: "Rayard".to_string(),
            name: "Custom Bar".to_string(),
            short_name: None,
            fixture_type_id: None,
            dmx_modes: vec![DmxModeSummary {
                name: "4ch".to_string(),
                controls: vec![
                    AttributeControl {
                        attribute: "Dimmer".to_string(),
                        channel_name: "Dimmer".to_string(),
                        geometry: None,
                        offsets: vec![1],
                        resolution: AttributeResolution::EightBit,
                        default_value: 0,
                        functions: Vec::new(),
                    },
                    AttributeControl {
                        attribute: "ColorRed".to_string(),
                        channel_name: "ColorRed".to_string(),
                        geometry: None,
                        offsets: vec![2],
                        resolution: AttributeResolution::EightBit,
                        default_value: 0,
                        functions: Vec::new(),
                    },
                ],
            }],
            geometries: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn project_with_valid_video_graph() -> ProjectFile {
        ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot {
                video: protocol::VideoSnapshot {
                    layers: vec![project_video_layer(10)],
                    compositions: vec![CompositionSummary {
                        id: 1,
                        label: "Main".to_string(),
                        layer_ids: vec![10],
                        output_ids: vec![7],
                    }],
                    outputs: vec![project_video_output(7, 1)],
                    ..protocol::VideoSnapshot::default()
                },
                ..EngineSnapshot::default()
            },
        }
    }

    #[test]
    fn project_file_preserves_custom_fixture_profiles_and_reads_legacy_files() {
        let mut fixture = project_fixture(1, "Custom Bar 1", 0, 1);
        fixture.profile_source_path = "memory://custom/Rayard-Custom_Bar".to_string();
        fixture.profile_name = "Custom Bar".to_string();
        fixture.mode_name = "4ch".to_string();
        fixture.controls = project_custom_profile().dmx_modes[0].controls.clone();
        let project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: vec![project_custom_profile()],
            snapshot: EngineSnapshot {
                fixtures: vec![fixture],
                ..EngineSnapshot::default()
            },
        };

        validate_project_file(&project).unwrap();
        let json = serde_json::to_string_pretty(&project).unwrap();
        assert!(json.contains("\"custom_profiles\""));
        assert!(json.contains("memory://custom/Rayard-Custom_Bar"));
        let parsed: ProjectFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.custom_profiles.len(), 1);

        let mut legacy_json = serde_json::to_value(&project).unwrap();
        legacy_json
            .as_object_mut()
            .unwrap()
            .remove("custom_profiles");
        let legacy: ProjectFile = serde_json::from_value(legacy_json).unwrap();
        assert!(legacy.custom_profiles.is_empty());
    }

    #[test]
    fn project_file_validation_allows_snapshot_backed_external_fixture_refs() {
        let mut fixture = project_fixture(1, "Imported Spot 1", 0, 1);
        fixture.profile_source_path = "C:/missing/profiles/imported-spot.gdtf".to_string();
        fixture.profile_name = "Imported Spot".to_string();
        fixture.mode_name = "Standard".to_string();
        let project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot {
                fixtures: vec![fixture],
                ..EngineSnapshot::default()
            },
        };

        validate_project_file(&project).unwrap();
    }

    #[test]
    fn project_file_validation_rejects_missing_custom_fixture_profile_refs() {
        let mut project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot {
                fixtures: vec![project_fixture(1, "Custom Bar 1", 0, 1)],
                ..EngineSnapshot::default()
            },
        };
        project.snapshot.fixtures[0].profile_source_path =
            "memory://custom/Rayard-Custom_Bar".to_string();
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("references missing custom profile"));

        project.custom_profiles = vec![project_custom_profile(), project_custom_profile()];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("duplicate custom profile source path"));
    }

    fn project_node_graph(id: NodeGraphId, fixture_id: FixtureId) -> NodeGraphSummary {
        NodeGraphSummary {
            id,
            label: "Graph".to_string(),
            enabled: true,
            nodes: vec![
                protocol::NodeGraphNodeSummary {
                    id: 1,
                    label: "LFO".to_string(),
                    kind: NodeGraphNodeKind::Lfo,
                    x: 8.0,
                    y: 32.0,
                    lfo: Some(protocol::NodeGraphLfoNode {
                        shape: protocol::LfoShape::Sine,
                        period_ms: 1_000,
                        clock_sync: None,
                        phase: 0.0,
                        amplitude: 1.0,
                        bias: 0.0,
                    }),
                    position_wave: None,
                    transform: None,
                    output: None,
                },
                protocol::NodeGraphNodeSummary {
                    id: 2,
                    label: "Scale".to_string(),
                    kind: NodeGraphNodeKind::Transform,
                    x: 42.0,
                    y: 32.0,
                    lfo: None,
                    position_wave: None,
                    transform: Some(protocol::NodeGraphTransformNode {
                        op: NodeGraphTransformOp::Scale,
                        amount: 1.0,
                        min: 0.0,
                        max: 1.0,
                    }),
                    output: None,
                },
                protocol::NodeGraphNodeSummary {
                    id: 3,
                    label: "Output".to_string(),
                    kind: NodeGraphNodeKind::Output,
                    x: 76.0,
                    y: 32.0,
                    lfo: None,
                    position_wave: None,
                    transform: None,
                    output: Some(protocol::NodeGraphOutputNode {
                        fixture_ids: vec![fixture_id],
                        target_group_ids: Vec::new(),
                        attribute: "Dimmer".to_string(),
                        video_targets: Vec::new(),
                        low: 0,
                        high: 65_535,
                        blend_mode: protocol::EffectBlendMode::Override,
                    }),
                },
            ],
            edges: vec![
                protocol::NodeGraphEdgeSummary {
                    from_node: 1,
                    from_port: "value".to_string(),
                    to_node: 2,
                    to_port: "input".to_string(),
                },
                protocol::NodeGraphEdgeSummary {
                    from_node: 2,
                    from_port: "value".to_string(),
                    to_node: 3,
                    to_port: "input".to_string(),
                },
            ],
        }
    }

    #[test]
    fn project_file_validation_rejects_broken_video_graph_references() {
        let mut project = project_with_valid_video_graph();
        validate_project_file(&project).unwrap();

        project.snapshot.video.compositions[0].layer_ids.push(99);
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("references missing video layer 99"));

        project = project_with_valid_video_graph();
        project.snapshot.video.outputs[0].composition_id = 99;
        project.snapshot.video.compositions[0].output_ids.clear();
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("routes to missing composition 99"));

        project = project_with_valid_video_graph();
        project.snapshot.cues.push(protocol::CueSummary {
            id: 3,
            label: "Video cue".to_string(),
            fade_ms: 0,
            targets: Vec::new(),
            video_targets: vec![VideoLayerTarget {
                layer_id: 99,
                state: VideoLayerState::default(),
            }],
            video_output_targets: Vec::new(),

            node_graph_targets: Vec::new(),
        });
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("Project cue 3 references missing video layer 99"));

        project = project_with_valid_video_graph();
        project.snapshot.timeline.video_automations =
            vec![protocol::TimelineVideoAutomationSummary {
                id: 12,
                layer_id: 10,
                param: VideoParam::Opacity,
                track: TimelineTrackKind::Video,
                keyframes: vec![VideoAutomationKeyframeSummary {
                    time_ms: 0,
                    value: f32::NAN,
                    interpolation: protocol::AutomationInterpolation::Linear,
                }],
                enabled: true,
            }];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("non-finite keyframe values"));

        project = project_with_valid_video_graph();
        project.snapshot.video.layers[0].state.transform.scale_x = 0.0;
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("transform scale must be greater than 0"));
    }

    #[test]
    fn project_file_validation_rejects_invalid_video_sources() {
        let mut project = project_with_valid_video_graph();
        project.snapshot.video.layers[0].source.kind = VideoSourceKind::File;
        project.snapshot.video.layers[0].source.path = None;
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("requires a file path"));

        project = project_with_valid_video_graph();
        project.snapshot.video.layers[0].source.path = Some(" ".to_string());
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("requires a still image path"));

        project = project_with_valid_video_graph();
        project.snapshot.video.layers[0].source.codec = Some(" ".to_string());
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("empty codec"));

        project = project_with_valid_video_graph();
        project.snapshot.video.layers[0].source.metadata = Some(protocol::VideoMediaMetadata {
            width: Some(0),
            height: Some(1080),
            frame_rate: Some(60.0),
            duration_ms: Some(1_000),
        });
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("metadata width"));

        project = project_with_valid_video_graph();
        project.snapshot.video.layers[0].source.metadata = Some(protocol::VideoMediaMetadata {
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(f32::NAN),
            duration_ms: Some(1_000),
        });
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("metadata frame rate"));

        project = project_with_valid_video_graph();
        project.snapshot.video.layers[0].source.kind = VideoSourceKind::Ndi;
        project.snapshot.video.layers[0].source.path = None;
        project.snapshot.video.layers[0].source.name = Some(" ".to_string());
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("source name for NDI input"));
    }

    #[test]
    fn project_file_validation_checks_node_graphs() {
        let mut project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot {
                fixtures: vec![project_fixture(1, "Fixture 1", 0, 1)],
                node_graphs: vec![project_node_graph(3, 1)],
                ..EngineSnapshot::default()
            },
        };
        validate_project_file(&project).unwrap();

        project.snapshot.cues.push(protocol::CueSummary {
            id: 9,
            label: "Graph Cue".to_string(),
            fade_ms: 0,
            targets: Vec::new(),
            video_targets: Vec::new(),
            video_output_targets: Vec::new(),
            node_graph_targets: vec![CueNodeGraphTarget {
                graph_id: 99,
                enabled: true,
            }],
        });
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("references missing node graph 99"));

        project.snapshot.cues.clear();
        project.snapshot.node_graphs.push(project_node_graph(3, 1));
        assert_eq!(
            validate_project_file(&project).unwrap_err(),
            "Project contains duplicate node graph id 3"
        );

        project.snapshot.node_graphs = vec![project_node_graph(4, 99)];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("references missing fixture 99"));

        project.snapshot.node_graphs = vec![project_node_graph(5, 1)];
        project.snapshot.node_graphs[0].edges[0].to_node = 99;
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("missing node reference"));
    }

    #[test]
    fn node_graph_preset_file_serializes_and_validates() {
        let snapshot = EngineSnapshot {
            fixtures: vec![project_fixture(1, "Fixture 1", 0, 1)],
            ..EngineSnapshot::default()
        };
        let file = NodeGraphPresetFile {
            version: 1,
            app: "Rayard".to_string(),
            graph: project_node_graph(3, 1),
        };

        validate_node_graph_preset_file(&file, &snapshot).unwrap();
        let json = serde_json::to_string_pretty(&file).unwrap();
        let parsed: NodeGraphPresetFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.graph.label, "Graph");

        let unsupported = NodeGraphPresetFile {
            version: 2,
            ..file.clone()
        };
        assert!(validate_node_graph_preset_file(&unsupported, &snapshot)
            .unwrap_err()
            .contains("version"));

        let wrong_app = NodeGraphPresetFile {
            app: "Other".to_string(),
            ..file
        };
        assert!(validate_node_graph_preset_file(&wrong_app, &snapshot)
            .unwrap_err()
            .contains("Unsupported node graph preset app"));
    }

    fn project_fixture(
        id: FixtureId,
        label: &str,
        universe: u16,
        address: u16,
    ) -> PatchedFixtureSummary {
        PatchedFixtureSummary {
            id,
            label: label.to_string(),
            profile_source_path: "memory://fixture.gdtf".to_string(),
            profile_name: "Mini Spot".to_string(),
            manufacturer: "Rayard".to_string(),
            mode_name: "Standard".to_string(),
            universe,
            address,
            group_ids: Vec::new(),
            position: Vec3::default(),
            rotation: Rotation3::default(),
            geometries: Vec::new(),
            controls: vec![
                AttributeControl {
                    attribute: "Dimmer".to_string(),
                    channel_name: "Dimmer".to_string(),
                    geometry: None,
                    offsets: vec![1],
                    resolution: AttributeResolution::EightBit,
                    default_value: 0,
                    functions: Vec::new(),
                },
                AttributeControl {
                    attribute: "Pan".to_string(),
                    channel_name: "Pan".to_string(),
                    geometry: None,
                    offsets: vec![2, 3],
                    resolution: AttributeResolution::SixteenBit,
                    default_value: 0,
                    functions: Vec::new(),
                },
            ],
            attribute_values: Vec::new(),
            limits: FixtureLimits::default(),
            highlighted: false,
            soloed: false,
            parked: false,
        }
    }

    fn project_geometry(name: &str, parent: Option<&str>) -> GeometrySummary {
        GeometrySummary {
            name: name.to_string(),
            kind: if name == "Beam" { "Beam" } else { "Geometry" }.to_string(),
            parent: parent.map(str::to_string),
            matrix: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
            model_name: None,
            model_file: None,
            model_primitive: None,
            model_dimensions: None,
            beam_type: None,
            beam_angle_deg: None,
            field_angle_deg: None,
            beam_radius: None,
        }
    }

    #[test]
    fn project_file_validation_rejects_missing_fixture_geometry_reference() {
        let mut fixture = project_fixture(1, "Fixture 1", 0, 1);
        fixture.geometries = vec![project_geometry("Body", None)];
        fixture.controls[0].geometry = Some("Beam".to_string());
        let project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot {
                fixtures: vec![fixture],
                ..EngineSnapshot::default()
            },
        };

        let error = validate_project_file(&project).unwrap_err();

        assert!(error.contains("control 'Dimmer' references missing geometry 'Beam'"));
    }

    #[test]
    fn project_file_validation_rejects_fixture_geometry_parent_cycle() {
        let mut fixture = project_fixture(1, "Fixture 1", 0, 1);
        fixture.geometries = vec![
            project_geometry("Body", Some("Head")),
            project_geometry("Head", Some("Body")),
        ];
        fixture.controls[0].geometry = Some("Body".to_string());
        let project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot {
                fixtures: vec![fixture],
                ..EngineSnapshot::default()
            },
        };

        let error = validate_project_file(&project).unwrap_err();

        assert!(error.contains("parent cycle"));
    }

    #[test]
    fn project_file_validation_rejects_custom_profile_geometry_mismatch() {
        let mut profile = project_custom_profile();
        profile.geometries = vec![project_geometry("Body", None)];
        profile.dmx_modes[0].controls[0].geometry = Some("Beam".to_string());
        let project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: vec![profile],
            snapshot: EngineSnapshot::default(),
        };

        let error = validate_project_file(&project).unwrap_err();

        assert!(error.contains("custom profile"));
        assert!(error.contains("references missing geometry 'Beam'"));
    }

    fn sample_automation_keyframes() -> Vec<AutomationKeyframeSummary> {
        vec![
            AutomationKeyframeSummary {
                time_ms: 1_000,
                value: 0,
                interpolation: protocol::AutomationInterpolation::Linear,
            },
            AutomationKeyframeSummary {
                time_ms: 2_000,
                value: 65_535,
                interpolation: protocol::AutomationInterpolation::Step,
            },
        ]
    }

    fn project_lfo_effect(id: EffectId, fixture_ids: Vec<FixtureId>) -> EffectSummary {
        EffectSummary {
            id,
            label: "Dimmer pulse".to_string(),
            effect_type: EffectKind::Lfo,
            fixture_ids,
            target_group_ids: Vec::new(),
            attribute: "Dimmer".to_string(),
            video_targets: Vec::new(),
            shape: protocol::LfoShape::Sine,
            period_ms: Some(500),
            clock_sync: None,
            low: 0,
            high: 65_535,
            phase: 0.0,
            blend_mode: protocol::EffectBlendMode::Override,
            origin: None,
            direction: None,
            speed: None,
            wavelength: None,
            enabled: true,
        }
    }

    #[test]
    fn compatible_timeline_automation_targets_selects_group_fixture_attributes() {
        let mut first = project_fixture(1, "Front 1", 0, 1);
        first.group_ids = vec!["front".to_string()];
        let mut nested = project_fixture(2, "Front Mover", 0, 10);
        nested.group_ids = vec!["front/movers".to_string()];
        let mut incompatible = project_fixture(3, "Front Color", 0, 20);
        incompatible.group_ids = vec!["front".to_string()];
        incompatible
            .controls
            .retain(|control| control.attribute != "Pan");
        let mut back = project_fixture(4, "Back", 0, 30);
        back.group_ids = vec!["back".to_string()];
        let snapshot = EngineSnapshot {
            fixtures: vec![first, nested, incompatible, back],
            ..EngineSnapshot::default()
        };

        let (fixture_ids, skipped_count) = compatible_timeline_automation_targets(
            &snapshot,
            "front",
            "Pan",
            &sample_automation_keyframes(),
        )
        .unwrap();

        assert_eq!(fixture_ids, vec![1, 2]);
        assert_eq!(skipped_count, 1);
    }

    #[test]
    fn compatible_timeline_automation_targets_rejects_empty_or_incompatible_group() {
        let mut fixture = project_fixture(1, "Front 1", 0, 1);
        fixture.group_ids = vec!["front".to_string()];
        fixture
            .controls
            .retain(|control| control.attribute != "Pan");
        let snapshot = EngineSnapshot {
            fixtures: vec![fixture],
            ..EngineSnapshot::default()
        };

        assert!(compatible_timeline_automation_targets(
            &snapshot,
            "front",
            "Pan",
            &sample_automation_keyframes(),
        )
        .unwrap_err()
        .contains("No fixtures"));
        assert!(compatible_timeline_automation_targets(
            &snapshot,
            "missing",
            "Dimmer",
            &sample_automation_keyframes(),
        )
        .unwrap_err()
        .contains("was not found"));
    }

    #[test]
    fn project_file_validation_rejects_invalid_fixture_patches() {
        let mut project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot::default(),
        };
        project.snapshot.fixtures = vec![
            project_fixture(1, "Fixture 1", 0, 1),
            project_fixture(2, "Fixture 2", 0, 3),
        ];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("conflicts in universe 0"));

        project.snapshot.fixtures = vec![project_fixture(1, "Overflow", 0, 511)];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("exceeds DMX universe 0"));

        project.snapshot.fixtures = vec![project_fixture(1, "", 0, 1)];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("empty fixture label"));

        project.snapshot.fixtures = vec![project_fixture(1, "Bad Transform", 0, 1)];
        project.snapshot.fixtures[0].position.x = f32::NAN;
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("invalid transform"));

        project.snapshot.fixtures = vec![project_fixture(1, "Bad Group", 0, 1)];
        project.snapshot.fixtures[0].group_ids = vec!["front//movers".to_string()];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("invalid group id"));
    }

    #[test]
    fn project_file_validation_rejects_invalid_fixture_attribute_values() {
        let mut project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot {
                fixtures: vec![project_fixture(1, "Fixture 1", 0, 1)],
                ..EngineSnapshot::default()
            },
        };
        project.snapshot.fixtures[0].attribute_values = vec![protocol::AttributeValueSummary {
            attribute: "Zoom".to_string(),
            value: 32_768,
        }];

        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("unavailable attribute 'Zoom'"));

        project.snapshot.fixtures[0].attribute_values = vec![
            protocol::AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: 1_000,
            },
            protocol::AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: 2_000,
            },
        ];

        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("duplicate attribute value 'Dimmer'"));
    }

    #[test]
    fn project_file_validation_rejects_broken_lighting_references() {
        let mut project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot {
                fixtures: vec![project_fixture(1, "Fixture 1", 0, 1)],
                ..EngineSnapshot::default()
            },
        };
        project.snapshot.cues.push(protocol::CueSummary {
            id: 1,
            label: "Look".to_string(),
            fade_ms: 0,
            targets: vec![CueFixtureTarget {
                fixture_id: 99,
                values: vec![protocol::AttributeValueSummary {
                    attribute: "Dimmer".to_string(),
                    value: 32_768,
                }],
            }],
            video_targets: Vec::new(),
            video_output_targets: Vec::new(),
            node_graph_targets: Vec::new(),
        });
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("references missing fixture 99"));

        project.snapshot.cues.clear();
        project.snapshot.timeline.automations = vec![protocol::TimelineAutomationSummary {
            id: 7,
            fixture_id: 1,
            attribute: "Zoom".to_string(),
            track: TimelineTrackKind::Lighting,
            keyframes: sample_automation_keyframes(),
            enabled: true,
        }];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("unavailable attribute 'Zoom'"));

        project.snapshot.timeline.automations.clear();
        project.snapshot.effects = vec![EffectSummary {
            id: 4,
            label: "Group chase".to_string(),
            effect_type: EffectKind::Lfo,
            fixture_ids: Vec::new(),
            target_group_ids: vec!["missing".to_string()],
            attribute: "Dimmer".to_string(),
            video_targets: Vec::new(),
            shape: protocol::LfoShape::Sine,
            period_ms: Some(500),
            clock_sync: None,
            low: 0,
            high: 65_535,
            phase: 0.0,
            blend_mode: protocol::EffectBlendMode::Override,
            origin: None,
            direction: None,
            speed: None,
            wavelength: None,
            enabled: true,
        }];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("references missing fixture group 'missing'"));

        project.snapshot.effects.clear();
        let mut graph = project_node_graph(10, 1);
        graph.nodes[2].output.as_mut().unwrap().attribute = "Zoom".to_string();
        project.snapshot.node_graphs = vec![graph];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("unavailable attribute 'Zoom'"));
    }

    #[test]
    fn project_file_validation_rejects_invalid_effect_body() {
        let mut project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot {
                fixtures: vec![project_fixture(1, "Fixture 1", 0, 1)],
                ..EngineSnapshot::default()
            },
        };

        let mut effect = project_lfo_effect(4, vec![1]);
        effect.period_ms = None;
        project.snapshot.effects = vec![effect];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("missing period_ms"));

        project.snapshot.effects = vec![EffectSummary {
            id: 5,
            label: "Wave".to_string(),
            effect_type: EffectKind::PositionWave,
            fixture_ids: vec![1],
            target_group_ids: Vec::new(),
            attribute: "Dimmer".to_string(),
            video_targets: Vec::new(),
            shape: protocol::LfoShape::Sine,
            period_ms: None,
            clock_sync: None,
            low: 0,
            high: 65_535,
            phase: 0.0,
            blend_mode: protocol::EffectBlendMode::Override,
            origin: Some(Vec3 {
                x: f32::NAN,
                y: 0.0,
                z: 0.0,
            }),
            direction: Some(Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }),
            speed: Some(1.0),
            wavelength: Some(1.0),
            enabled: true,
        }];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("origin and direction values must be finite"));
    }

    #[test]
    fn project_file_validation_rejects_duplicate_timeline_key_times() {
        let mut project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot {
                fixtures: vec![project_fixture(1, "Fixture 1", 0, 1)],
                ..EngineSnapshot::default()
            },
        };
        project.snapshot.timeline.automations = vec![protocol::TimelineAutomationSummary {
            id: 7,
            fixture_id: 1,
            attribute: "Dimmer".to_string(),
            track: TimelineTrackKind::Lighting,
            keyframes: vec![
                AutomationKeyframeSummary {
                    time_ms: 1_000,
                    value: 0,
                    interpolation: protocol::AutomationInterpolation::Linear,
                },
                AutomationKeyframeSummary {
                    time_ms: 1_000,
                    value: 65_535,
                    interpolation: protocol::AutomationInterpolation::Step,
                },
            ],
            enabled: true,
        }];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("duplicate keyframe time 1000ms"));

        let mut project = project_with_valid_video_graph();
        project.snapshot.timeline.video_automations =
            vec![protocol::TimelineVideoAutomationSummary {
                id: 8,
                layer_id: 10,
                param: VideoParam::Opacity,
                track: TimelineTrackKind::Video,
                keyframes: vec![
                    VideoAutomationKeyframeSummary {
                        time_ms: 250,
                        value: 0.0,
                        interpolation: protocol::AutomationInterpolation::Linear,
                    },
                    VideoAutomationKeyframeSummary {
                        time_ms: 250,
                        value: 1.0,
                        interpolation: protocol::AutomationInterpolation::Step,
                    },
                ],
                enabled: true,
            }];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("duplicate keyframe time 250ms"));
    }

    #[test]
    fn project_file_validation_rejects_invalid_video_mapping_presets() {
        let mut project = project_with_valid_video_graph();
        project.snapshot.video.mapping_presets = vec![
            VideoOutputMappingPresetSummary {
                label: " Front Projector ".to_string(),
                mapping: VideoOutputMapping::default(),
            },
            VideoOutputMappingPresetSummary {
                label: "Front Projector".to_string(),
                mapping: VideoOutputMapping::default(),
            },
        ];

        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("duplicate video output mapping preset label Front Projector"));

        project.snapshot.video.mapping_presets = vec![VideoOutputMappingPresetSummary {
            label: "Bad Projector".to_string(),
            mapping: VideoOutputMapping {
                aspect_ratio: 0.0,
                ..VideoOutputMapping::default()
            },
        }];

        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("aspect ratio"));
    }

    #[test]
    fn project_file_validation_rejects_invalid_video_cue_points() {
        let mut project = project_with_valid_video_graph();
        project.snapshot.video.layers[0].state.cue_points = vec![
            protocol::VideoCuePointSummary {
                position_ms: 250,
                label: "Drop".to_string(),
                color: Some("#ff3366".to_string()),
            },
            protocol::VideoCuePointSummary {
                position_ms: 250,
                label: "Duplicate".to_string(),
                color: None,
            },
        ];

        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("duplicate video cue point at 250ms"));

        project.snapshot.video.layers[0].state.cue_points = vec![protocol::VideoCuePointSummary {
            position_ms: 250,
            label: "Drop".to_string(),
            color: Some("red".to_string()),
        }];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("cue point color is invalid"));

        project.snapshot.video.layers[0].state.cue_points.clear();
        project.snapshot.video.layers[0].state.cue_points_ms = vec![100, 100];
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("duplicate legacy video cue point at 100ms"));
    }

    #[test]
    fn project_file_validation_checks_dmx_output_routes() {
        let mut project = ProjectFile {
            version: 1,
            app: "Rayard".to_string(),
            custom_profiles: Vec::new(),
            snapshot: EngineSnapshot::default(),
        };
        project.snapshot.dmx_outputs = vec![
            DmxOutputConfig {
                enabled: true,
                protocol: DmxOutputProtocol::Sacn,
                target_ip: "multicast".to_string(),
                port: 5568,
                universe: 4,
                serial_port: String::new(),
                serial_baud_rate: 57_600,
            },
            DmxOutputConfig {
                enabled: true,
                protocol: DmxOutputProtocol::Sacn,
                target_ip: "auto".to_string(),
                port: 5568,
                universe: 4,
                serial_port: String::new(),
                serial_baud_rate: 57_600,
            },
        ];

        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("Duplicate DMX output route"));

        project.snapshot.dmx_outputs.clear();
        project.snapshot.output = DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::Sacn,
            target_ip: "multicast".to_string(),
            port: 5568,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        };

        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("sACN universe"));
    }

    #[test]
    fn cue_targets_from_snapshot_includes_video_outputs_and_node_graphs() {
        let mut snapshot = EngineSnapshot::default();
        snapshot.video.outputs.push(VideoOutputSummary {
            id: 7,
            label: "Front Projector".to_string(),
            kind: VideoOutputKind::Display,
            enabled: false,
            composition_id: 1,
            fullscreen: true,
            monitor_id: Some(0),
            width: 1920,
            height: 1080,
            endpoint_name: None,
            opacity: 0.35,
            blackout: true,
            mapping: VideoOutputMapping::default(),
        });
        let mut graph = project_node_graph(9, 1);
        graph.enabled = false;
        snapshot.node_graphs.push(graph);

        let (fixture_targets, video_targets, video_output_targets, node_graph_targets) =
            cue_targets_from_snapshot(&snapshot);

        assert!(fixture_targets.is_empty());
        assert!(video_targets.is_empty());
        assert_eq!(video_output_targets.len(), 1);
        assert_eq!(video_output_targets[0].output_id, 7);
        assert!(!video_output_targets[0].enabled);
        assert!((video_output_targets[0].opacity - 0.35).abs() < f32::EPSILON);
        assert!(video_output_targets[0].blackout);
        assert_eq!(node_graph_targets.len(), 1);
        assert_eq!(node_graph_targets[0].graph_id, 9);
        assert!(!node_graph_targets[0].enabled);
    }

    #[test]
    fn cue_targets_from_snapshot_can_capture_selected_group_only() {
        let mut snapshot = EngineSnapshot::default();
        snapshot.fixtures.push(PatchedFixtureSummary {
            id: 1,
            label: "Front 1".to_string(),
            profile_source_path: String::new(),
            profile_name: "Dimmer".to_string(),
            manufacturer: "Rayard".to_string(),
            mode_name: "1ch".to_string(),
            universe: 0,
            address: 1,
            group_ids: vec!["front".to_string()],
            position: Vec3::default(),
            rotation: Rotation3::default(),
            geometries: Vec::new(),
            controls: Vec::new(),
            attribute_values: vec![protocol::AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: 10_000,
            }],
            limits: FixtureLimits::default(),
            highlighted: false,
            soloed: false,
            parked: false,
        });
        snapshot.fixtures.push(PatchedFixtureSummary {
            id: 2,
            label: "Back 1".to_string(),
            profile_source_path: String::new(),
            profile_name: "Dimmer".to_string(),
            manufacturer: "Rayard".to_string(),
            mode_name: "1ch".to_string(),
            universe: 0,
            address: 2,
            group_ids: vec!["back".to_string()],
            position: Vec3::default(),
            rotation: Rotation3::default(),
            geometries: Vec::new(),
            controls: Vec::new(),
            attribute_values: vec![protocol::AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: 20_000,
            }],
            limits: FixtureLimits::default(),
            highlighted: false,
            soloed: false,
            parked: false,
        });

        let (fixture_targets, video_targets, video_output_targets, node_graph_targets) =
            cue_targets_from_snapshot_with_scope(
                &snapshot,
                &CueCaptureScope::SelectedGroup {
                    group_id: "front".to_string(),
                },
            )
            .unwrap();

        assert_eq!(fixture_targets.len(), 1);
        assert_eq!(fixture_targets[0].fixture_id, 1);
        assert_eq!(fixture_targets[0].values[0].value, 10_000);
        assert!(video_targets.is_empty());
        assert!(video_output_targets.is_empty());
        assert!(node_graph_targets.is_empty());
    }

    #[test]
    fn scoped_cue_update_merges_selected_group_without_clearing_other_targets() {
        let mut snapshot = EngineSnapshot::default();
        snapshot.fixtures.push(PatchedFixtureSummary {
            id: 1,
            label: "Front 1".to_string(),
            profile_source_path: String::new(),
            profile_name: "Dimmer".to_string(),
            manufacturer: "Rayard".to_string(),
            mode_name: "1ch".to_string(),
            universe: 0,
            address: 1,
            group_ids: vec!["front".to_string()],
            position: Vec3::default(),
            rotation: Rotation3::default(),
            geometries: Vec::new(),
            controls: Vec::new(),
            attribute_values: vec![protocol::AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: 30_000,
            }],
            limits: FixtureLimits::default(),
            highlighted: false,
            soloed: false,
            parked: false,
        });
        snapshot.fixtures.push(PatchedFixtureSummary {
            id: 2,
            label: "Back 1".to_string(),
            profile_source_path: String::new(),
            profile_name: "Dimmer".to_string(),
            manufacturer: "Rayard".to_string(),
            mode_name: "1ch".to_string(),
            universe: 0,
            address: 2,
            group_ids: vec!["back".to_string()],
            position: Vec3::default(),
            rotation: Rotation3::default(),
            geometries: Vec::new(),
            controls: Vec::new(),
            attribute_values: vec![protocol::AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: 40_000,
            }],
            limits: FixtureLimits::default(),
            highlighted: false,
            soloed: false,
            parked: false,
        });
        snapshot.cues.push(protocol::CueSummary {
            id: 42,
            label: "Look".to_string(),
            fade_ms: 500,
            targets: vec![
                CueFixtureTarget {
                    fixture_id: 1,
                    values: vec![protocol::AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 1_000,
                    }],
                },
                CueFixtureTarget {
                    fixture_id: 2,
                    values: vec![protocol::AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 2_000,
                    }],
                },
            ],
            video_targets: vec![VideoLayerTarget {
                layer_id: 9,
                state: VideoLayerState {
                    opacity: 0.25,
                    ..VideoLayerState::default()
                },
            }],
            video_output_targets: vec![VideoOutputTarget {
                output_id: 7,
                enabled: true,
                opacity: 0.5,
                blackout: false,
            }],
            node_graph_targets: Vec::new(),
        });

        let scope = CueCaptureScope::SelectedGroup {
            group_id: "front".to_string(),
        };
        let captured = cue_targets_from_snapshot_with_scope(&snapshot, &scope).unwrap();
        let (fixture_targets, video_targets, video_output_targets, node_graph_targets) =
            merge_cue_update_targets(&snapshot, 42, &scope, captured).unwrap();

        let front = fixture_targets
            .iter()
            .find(|target| target.fixture_id == 1)
            .unwrap();
        let back = fixture_targets
            .iter()
            .find(|target| target.fixture_id == 2)
            .unwrap();
        assert_eq!(fixture_targets.len(), 2);
        assert_eq!(front.values[0].value, 30_000);
        assert_eq!(back.values[0].value, 2_000);
        assert_eq!(video_targets.len(), 1);
        assert_eq!(video_targets[0].layer_id, 9);
        assert_eq!(video_output_targets.len(), 1);
        assert_eq!(video_output_targets[0].output_id, 7);
        assert!(node_graph_targets.is_empty());
    }

    #[test]
    fn fixture_transform_validation_rejects_non_finite_values() {
        let error = validate_fixture_transform(
            &Vec3 {
                x: f32::NAN,
                y: 0.0,
                z: 0.0,
            },
            &Rotation3::default(),
        )
        .unwrap_err();
        assert!(error.contains("position"));

        let error = validate_fixture_transform(
            &Vec3::default(),
            &Rotation3 {
                pitch: 0.0,
                yaw: f32::INFINITY,
                roll: 0.0,
            },
        )
        .unwrap_err();
        assert!(error.contains("rotation"));
    }

    #[test]
    fn stage_map_config_validation_rejects_invalid_bounds() {
        let non_finite = StageMapConfig {
            locked: true,
            min_x: f32::NAN,
            max_x: 10.0,
            min_z: -5.0,
            max_z: 5.0,
        };
        assert!(validate_stage_map_config(&non_finite)
            .unwrap_err()
            .contains("finite"));

        let inverted = StageMapConfig {
            locked: true,
            min_x: 5.0,
            max_x: 5.0,
            min_z: -5.0,
            max_z: 5.0,
        };
        assert!(validate_stage_map_config(&inverted)
            .unwrap_err()
            .contains("min bounds"));
    }

    #[test]
    fn project_stage_object_validation_rejects_duplicate_ids_and_invalid_size() {
        let valid = StageObjectSummary {
            id: 1,
            label: "Front Truss".to_string(),
            kind: StageObjectKind::Truss,
            x: 0.0,
            z: -4.0,
            width: 12.0,
            depth: 0.4,
            rotation_deg: 0.0,
            color: Some("#55ccff".to_string()),
        };
        validate_project_stage_objects(&[valid.clone()]).unwrap();

        let duplicate_error =
            validate_project_stage_objects(&[valid.clone(), valid.clone()]).unwrap_err();
        assert!(duplicate_error.contains("duplicate stage object id 1"));

        let mut invalid_size = valid;
        invalid_size.id = 2;
        invalid_size.width = 0.0;
        let size_error = validate_project_stage_objects(&[invalid_size]).unwrap_err();
        assert!(size_error.contains("size must be greater than zero"));
    }

    fn sample_patch_profile() -> FixtureProfileSummary {
        custom_fixture_profile_from_request(CustomFixtureProfileRequest {
            manufacturer: "Rayard".to_string(),
            name: "Overlap Test".to_string(),
            mode_name: "Default".to_string(),
            attributes: vec![
                "Dimmer".to_string(),
                "ColorRed".to_string(),
                "ColorGreen".to_string(),
                "ColorBlue".to_string(),
            ],
        })
    }

    fn sample_patch_request(universe: u16, address: u16) -> PatchFixtureRequest {
        PatchFixtureRequest {
            profile_path: "memory://custom/Rayard-Overlap_Test".to_string(),
            mode_name: Some("Default".to_string()),
            label: "New Fixture".to_string(),
            universe,
            address,
            group_ids: Vec::new(),
            position: Vec3::default(),
            rotation: Rotation3::default(),
        }
    }

    fn sample_patched_fixture(
        profile: &FixtureProfileSummary,
        universe: u16,
        address: u16,
    ) -> PatchedFixtureSummary {
        PatchedFixtureSummary {
            id: 7,
            label: "Existing Fixture".to_string(),
            profile_source_path: profile.source_path.clone(),
            profile_name: profile.name.clone(),
            manufacturer: profile.manufacturer.clone(),
            mode_name: "Default".to_string(),
            universe,
            address,
            group_ids: Vec::new(),
            position: Vec3::default(),
            rotation: Rotation3::default(),
            geometries: profile.geometries.clone(),
            controls: profile.dmx_modes[0].controls.clone(),
            attribute_values: Vec::new(),
            limits: FixtureLimits::default(),
            highlighted: false,
            soloed: false,
            parked: false,
        }
    }

    #[test]
    fn patched_fixture_profile_can_be_reused_as_memory_profile() {
        let profile = sample_patch_profile();
        let mut fixture = sample_patched_fixture(&profile, 1, 10);
        fixture.geometries.push(GeometrySummary {
            name: "Lens".to_string(),
            kind: "Beam".to_string(),
            parent: fixture
                .geometries
                .first()
                .map(|geometry| geometry.name.clone()),
            matrix: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.3, //
                0.0, 0.0, 0.0, 1.0,
            ],
            model_name: Some("LensModel".to_string()),
            model_file: Some("models/lens.glb".to_string()),
            model_primitive: Some("Cylinder".to_string()),
            model_dimensions: Some(Vec3 {
                x: 0.12,
                y: 0.08,
                z: 0.12,
            }),
            beam_type: Some("Wash".to_string()),
            beam_angle_deg: Some(14.0),
            field_angle_deg: Some(22.0),
            beam_radius: Some(0.08),
        });

        let rebuilt = fixture_profile_from_patched_fixture(&fixture);

        assert_eq!(rebuilt.source_path, "memory://patched-fixture/7");
        assert_eq!(rebuilt.manufacturer, "Rayard");
        assert_eq!(rebuilt.name, "Overlap Test");
        assert_eq!(rebuilt.dmx_modes[0].name, "Default");
        assert_eq!(rebuilt.dmx_modes[0].controls, fixture.controls);
        assert_eq!(rebuilt.geometries, fixture.geometries);
        assert_eq!(
            rebuilt
                .geometries
                .iter()
                .find(|geometry| geometry.name == "Lens")
                .and_then(|geometry| geometry.field_angle_deg),
            Some(22.0)
        );
        assert!(rebuilt.warnings[0].contains("Rebuilt from patched fixture"));
        assert!(rebuilt.warnings[0].contains("original GDTF XML"));
    }

    #[test]
    fn patch_address_validation_rejects_overlapping_fixture_ranges() {
        let profile = sample_patch_profile();
        let existing = sample_patched_fixture(&profile, 1, 10);
        let request = sample_patch_request(1, 8);

        let error = validate_patch_address_conflicts(&request, &profile, &[existing]).unwrap_err();

        assert!(error.contains("DMX address conflict"));
        assert!(error.contains("8-11"));
        assert!(error.contains("10-13"));
    }

    #[test]
    fn patch_address_validation_allows_separate_ranges_or_universes() {
        let profile = sample_patch_profile();
        let existing = sample_patched_fixture(&profile, 1, 10);

        assert!(validate_patch_address_conflicts(
            &sample_patch_request(1, 1),
            &profile,
            std::slice::from_ref(&existing)
        )
        .is_ok());
        assert!(validate_patch_address_conflicts(
            &sample_patch_request(2, 8),
            &profile,
            &[existing]
        )
        .is_ok());
    }

    #[test]
    fn patch_footprint_validation_rejects_missing_named_mode() {
        let profile = sample_patch_profile();
        let mut request = sample_patch_request(1, 1);
        request.mode_name = Some("Missing".to_string());

        let error = validate_patch_footprint(&request, &profile).unwrap_err();

        assert!(error.contains("Selected GDTF mode"));
    }

    #[test]
    fn patch_footprint_validation_handles_universe_edge_addresses() {
        let profile = sample_patch_profile();

        validate_patch_footprint(&sample_patch_request(1, 509), &profile).unwrap();

        let error = validate_patch_footprint(&sample_patch_request(1, 510), &profile).unwrap_err();

        assert!(error.contains("Fixture footprint exceeds DMX universe"));
        assert!(error.contains("end 513"));
    }

    #[test]
    fn prepared_patch_conflicts_reject_batch_overlap() {
        let profile = sample_patch_profile();
        let prepared = vec![
            PreparedFixturePatch {
                request: sample_patch_request(1, 1),
                profile: profile.clone(),
            },
            PreparedFixturePatch {
                request: sample_patch_request(1, 4),
                profile,
            },
        ];

        let error = validate_prepared_patch_conflicts(&prepared, &[]).unwrap_err();

        assert!(error.contains("DMX address conflict"));
        assert!(error.contains("4-7"));
        assert!(error.contains("1-4"));
    }

    #[test]
    fn prepared_patch_conflicts_allow_sequential_bulk_patch() {
        let profile = sample_patch_profile();
        let existing = sample_patched_fixture(&profile, 1, 20);
        let prepared = vec![
            PreparedFixturePatch {
                request: sample_patch_request(1, 1),
                profile: profile.clone(),
            },
            PreparedFixturePatch {
                request: sample_patch_request(1, 5),
                profile,
            },
        ];

        assert!(validate_prepared_patch_conflicts(&prepared, &[existing]).is_ok());
    }

    #[test]
    fn fixture_patch_update_validation_ignores_self_and_rejects_conflicts() {
        let profile = sample_patch_profile();
        let mut first = sample_patched_fixture(&profile, 1, 10);
        first.id = 7;
        let mut second = sample_patched_fixture(&profile, 1, 20);
        second.id = 8;
        let snapshot = EngineSnapshot {
            fixtures: vec![first, second],
            ..EngineSnapshot::default()
        };

        assert!(validate_fixture_patch_update(&snapshot, 7, "Renamed", 1, 10).is_ok());

        let error = validate_fixture_patch_update(&snapshot, 7, "Renamed", 1, 18).unwrap_err();

        assert!(error.contains("DMX address conflict"));
        assert!(error.contains("18-21"));
        assert!(error.contains("20-23"));
    }

    #[test]
    fn fixture_patch_update_validation_rejects_invalid_values() {
        let profile = sample_patch_profile();
        let fixture = sample_patched_fixture(&profile, 1, 10);
        let snapshot = EngineSnapshot {
            fixtures: vec![fixture],
            ..EngineSnapshot::default()
        };

        assert!(validate_fixture_patch_update(&snapshot, 7, "", 1, 10)
            .unwrap_err()
            .contains("label"));
        assert!(validate_fixture_patch_update(&snapshot, 7, "Fixture", 1, 0)
            .unwrap_err()
            .contains("between 1 and 512"));
    }

    #[test]
    fn fixture_limits_validation_accepts_and_normalizes_reversed_ranges() {
        let limits = FixtureLimits {
            dimmer_min: 40_000,
            dimmer_max: 5_000,
            pan_min: 55_000,
            pan_max: 10_000,
            tilt_min: 44_000,
            tilt_max: 8_000,
            invert_pan: true,
            invert_tilt: false,
            swap_pan_tilt: true,
        };

        validate_fixture_limits(&limits).unwrap();
        let normalized = normalize_fixture_limits(limits);

        assert_eq!(normalized.dimmer_min, 5_000);
        assert_eq!(normalized.dimmer_max, 40_000);
        assert_eq!(normalized.pan_min, 10_000);
        assert_eq!(normalized.pan_max, 55_000);
        assert_eq!(normalized.tilt_min, 8_000);
        assert_eq!(normalized.tilt_max, 44_000);
        assert!(normalized.invert_pan);
        assert!(!normalized.invert_tilt);
        assert!(normalized.swap_pan_tilt);
    }

    #[test]
    fn fixture_preset_preserves_profile_source_path_and_reads_legacy_files() {
        let mut preset = sample_preset(vec![protocol::AttributeValueSummary {
            attribute: "Dimmer".to_string(),
            value: 32_768,
        }]);
        preset.profile_source_path = Some("memory://custom/Rayard-Mini_Spot".to_string());

        let json = serde_json::to_string_pretty(&preset).unwrap();
        let parsed: FixturePreset = serde_json::from_str(&json).unwrap();

        assert!(json.contains("\"profile_source_path\""));
        assert_eq!(
            parsed.profile_source_path.as_deref(),
            Some("memory://custom/Rayard-Mini_Spot")
        );

        let mut legacy_json = serde_json::to_value(&preset).unwrap();
        legacy_json
            .as_object_mut()
            .unwrap()
            .remove("profile_source_path");
        let legacy: FixturePreset = serde_json::from_value(legacy_json).unwrap();
        assert_eq!(legacy.profile_source_path, None);

        let no_source_json = serde_json::to_string_pretty(&sample_preset(Vec::new())).unwrap();
        assert!(!no_source_json.contains("profile_source_path"));
    }

    #[test]
    fn preset_validation_accepts_matching_fixture_identity_and_attributes() {
        let preset = sample_preset(vec![protocol::AttributeValueSummary {
            attribute: "Dimmer".to_string(),
            value: 32_768,
        }]);

        assert!(validate_fixture_preset(
            &preset,
            "Rayard",
            "Mini Spot",
            "Standard",
            ["Dimmer", "Pan"]
        )
        .is_ok());
    }

    #[test]
    fn preset_validation_rejects_wrong_profile_or_unknown_attribute() {
        let preset = sample_preset(vec![protocol::AttributeValueSummary {
            attribute: "Dimmer".to_string(),
            value: 32_768,
        }]);
        let profile_error =
            validate_fixture_preset(&preset, "Rayard", "Other Spot", "Standard", ["Dimmer"])
                .unwrap_err();
        assert!(profile_error.contains("Preset is for"));

        let preset = sample_preset(vec![protocol::AttributeValueSummary {
            attribute: "Zoom".to_string(),
            value: 12_000,
        }]);
        let attribute_error =
            validate_fixture_preset(&preset, "Rayard", "Mini Spot", "Standard", ["Dimmer"])
                .unwrap_err();
        assert!(attribute_error.contains("not available"));

        let duplicate = sample_preset(vec![
            protocol::AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: 12_000,
            },
            protocol::AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: 48_000,
            },
        ]);
        let duplicate_error =
            validate_fixture_preset(&duplicate, "Rayard", "Mini Spot", "Standard", ["Dimmer"])
                .unwrap_err();
        assert!(duplicate_error.contains("duplicate attribute"));
    }

    #[test]
    fn fixture_preset_compatibility_selects_only_matching_fixture_targets() {
        let preset = sample_preset(vec![protocol::AttributeValueSummary {
            attribute: "Dimmer".to_string(),
            value: 32_768,
        }]);
        let profile = custom_fixture_profile_from_request(CustomFixtureProfileRequest {
            manufacturer: "Rayard".to_string(),
            name: "Mini Spot".to_string(),
            mode_name: "Standard".to_string(),
            attributes: vec!["Dimmer".to_string(), "Pan".to_string()],
        });
        let mut compatible = sample_patched_fixture(&profile, 1, 1);
        compatible.id = 1;
        compatible.mode_name = "Standard".to_string();
        let mut wrong_profile = compatible.clone();
        wrong_profile.id = 2;
        wrong_profile.profile_name = "Other Spot".to_string();
        let mut missing_attribute = compatible.clone();
        missing_attribute.id = 3;
        missing_attribute
            .controls
            .retain(|control| control.attribute != "Dimmer");

        let (fixture_ids, skipped_count) = compatible_fixture_preset_targets(
            &preset,
            vec![&compatible, &wrong_profile, &missing_attribute],
        );

        assert_eq!(fixture_ids, vec![1]);
        assert_eq!(skipped_count, 2);
    }

    fn sample_lfo_request() -> LfoEffectRequest {
        LfoEffectRequest {
            label: "Dimmer chase".to_string(),
            fixture_ids: vec![1],
            target_group_ids: Vec::new(),
            attribute: "Dimmer".to_string(),
            video_targets: Vec::new(),
            shape: protocol::LfoShape::Sine,
            period_ms: 500,
            clock_sync: Some(protocol::EffectClockSync { beats: 2.0 }),
            low: 0,
            high: 65_535,
            phase: 0.25,
            blend_mode: protocol::EffectBlendMode::Override,
        }
    }

    fn sample_position_wave_request() -> PositionWaveEffectRequest {
        PositionWaveEffectRequest {
            label: "Front wave".to_string(),
            fixture_ids: Vec::new(),
            target_group_ids: vec!["front".to_string()],
            attribute: "Dimmer".to_string(),
            video_targets: Vec::new(),
            shape: protocol::LfoShape::Triangle,
            origin: protocol::Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            direction: protocol::Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            speed: 1.0,
            wavelength: 2.0,
            clock_sync: Some(protocol::EffectClockSync { beats: 4.0 }),
            low: 0,
            high: 40_000,
            phase: 0.0,
            blend_mode: protocol::EffectBlendMode::Add,
        }
    }

    fn sample_video_effect_target(layer_id: VideoLayerId) -> VideoEffectTarget {
        VideoEffectTarget {
            layer_ids: vec![layer_id],
            param: VideoParam::Opacity,
            low: 0.0,
            high: 1.0,
            position: None,
        }
    }

    #[test]
    fn effect_summary_serializes_to_lfo_preset() {
        let effect = EffectSummary {
            id: 7,
            label: "Dimmer chase".to_string(),
            effect_type: EffectKind::Lfo,
            fixture_ids: vec![1],
            target_group_ids: Vec::new(),
            attribute: "Dimmer".to_string(),
            video_targets: vec![sample_video_effect_target(3)],
            shape: protocol::LfoShape::Sine,
            period_ms: Some(500),
            clock_sync: Some(protocol::EffectClockSync { beats: 2.0 }),
            low: 0,
            high: 65_535,
            phase: 0.25,
            blend_mode: protocol::EffectBlendMode::Override,
            origin: None,
            direction: None,
            speed: None,
            wavelength: None,
            enabled: false,
        };

        let preset = effect_summary_to_preset(&effect).unwrap();

        assert_eq!(preset.version, 1);
        assert_eq!(preset.effect_type, EffectKind::Lfo);
        assert!(!preset.enabled);
        let lfo = preset.lfo.unwrap();
        assert_eq!(lfo.period_ms, 500);
        assert_eq!(
            lfo.clock_sync,
            Some(protocol::EffectClockSync { beats: 2.0 })
        );
        assert_eq!(lfo.video_targets, vec![sample_video_effect_target(3)]);
        assert!(preset.position_wave.is_none());
    }

    #[test]
    fn effect_summary_serializes_position_wave_preset_with_shared_video_target() {
        let video_target = VideoEffectTarget {
            layer_ids: vec![2, 3],
            param: VideoParam::ColorHueDeg,
            low: -45.0,
            high: 45.0,
            position: Some(Vec3 {
                x: 1.5,
                y: 0.0,
                z: -2.25,
            }),
        };
        let origin = Vec3 {
            x: -1.0,
            y: 0.25,
            z: 2.0,
        };
        let direction = Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        let effect = EffectSummary {
            id: 12,
            label: "Shared sweep".to_string(),
            effect_type: EffectKind::PositionWave,
            fixture_ids: vec![4],
            target_group_ids: vec!["Front".to_string()],
            attribute: "Dimmer".to_string(),
            video_targets: vec![video_target.clone()],
            shape: protocol::LfoShape::Sine,
            period_ms: None,
            clock_sync: Some(protocol::EffectClockSync { beats: 4.0 }),
            low: 8_192,
            high: 57_344,
            phase: 0.125,
            blend_mode: protocol::EffectBlendMode::Multiply,
            origin: Some(origin),
            direction: Some(direction),
            speed: Some(1.25),
            wavelength: Some(3.5),
            enabled: false,
        };

        let preset = effect_summary_to_preset(&effect).unwrap();
        validate_effect_preset(&preset).unwrap();
        let json = serde_json::to_string_pretty(&preset).unwrap();
        let roundtrip: EffectPreset = serde_json::from_str(&json).unwrap();

        assert_eq!(roundtrip, preset);
        assert_eq!(roundtrip.version, 1);
        assert_eq!(roundtrip.effect_type, EffectKind::PositionWave);
        assert!(!roundtrip.enabled);
        assert!(roundtrip.lfo.is_none());
        let wave = roundtrip.position_wave.unwrap();
        assert_eq!(wave.label, "Shared sweep");
        assert_eq!(wave.fixture_ids, vec![4]);
        assert_eq!(wave.target_group_ids, vec!["Front".to_string()]);
        assert_eq!(wave.attribute, "Dimmer");
        assert_eq!(wave.video_targets, vec![video_target]);
        assert_eq!(wave.origin, origin);
        assert_eq!(wave.direction, direction);
        assert_eq!(wave.speed, 1.25);
        assert_eq!(wave.wavelength, 3.5);
        assert_eq!(
            wave.clock_sync,
            Some(protocol::EffectClockSync { beats: 4.0 })
        );
        assert_eq!(wave.low, 8_192);
        assert_eq!(wave.high, 57_344);
        assert_eq!(wave.blend_mode, protocol::EffectBlendMode::Multiply);
    }

    #[test]
    fn effect_preset_target_override_retargets_lfo_without_losing_timing() {
        let request = sample_lfo_request();
        let target_override = EffectTargetOverride {
            fixture_ids: vec![8, 9],
            target_group_ids: vec!["front".to_string()],
            attribute: "Pan".to_string(),
            video_targets: vec![sample_video_effect_target(4)],
        };

        let retargeted = apply_lfo_effect_target_override(request, &target_override);

        assert_eq!(retargeted.fixture_ids, vec![8, 9]);
        assert_eq!(retargeted.target_group_ids, vec!["front".to_string()]);
        assert_eq!(retargeted.attribute, "Pan");
        assert_eq!(
            retargeted.video_targets,
            vec![sample_video_effect_target(4)]
        );
        assert_eq!(retargeted.period_ms, 500);
        assert_eq!(
            retargeted.clock_sync,
            Some(protocol::EffectClockSync { beats: 2.0 })
        );
        assert_eq!(retargeted.shape, protocol::LfoShape::Sine);
    }

    #[test]
    fn effect_preset_target_override_can_make_position_wave_video_only() {
        let request = sample_position_wave_request();
        let target_override = EffectTargetOverride {
            fixture_ids: Vec::new(),
            target_group_ids: Vec::new(),
            attribute: "Dimmer".to_string(),
            video_targets: vec![sample_video_effect_target(6)],
        };

        let retargeted = apply_position_wave_effect_target_override(request, &target_override);

        assert!(retargeted.fixture_ids.is_empty());
        assert!(retargeted.target_group_ids.is_empty());
        assert!(retargeted.attribute.is_empty());
        assert_eq!(
            retargeted.video_targets,
            vec![sample_video_effect_target(6)]
        );
        assert_eq!(retargeted.wavelength, 2.0);
        assert_eq!(
            retargeted.clock_sync,
            Some(protocol::EffectClockSync { beats: 4.0 })
        );
        assert_eq!(retargeted.shape, protocol::LfoShape::Triangle);
    }

    #[test]
    fn effect_preset_validation_rejects_mismatched_body() {
        let preset = EffectPreset {
            version: 1,
            effect_type: EffectKind::Lfo,
            enabled: true,
            lfo: Some(sample_lfo_request()),
            position_wave: Some(sample_position_wave_request()),
        };

        let error = validate_effect_preset(&preset).unwrap_err();

        assert!(error.contains("must not contain"));
    }

    #[test]
    fn sample_effect_presets_are_valid_for_phase1_mini_show() {
        let show: ProjectFile =
            serde_json::from_str(include_str!("../../../samples/phase1-mini-show.ry")).unwrap();
        validate_project_file(&show).unwrap();
        let group_ids: HashSet<&str> = show
            .snapshot
            .fixtures
            .iter()
            .flat_map(|fixture| fixture.group_ids.iter().map(String::as_str))
            .collect();
        assert!(group_ids.contains("Front"));

        let pulse: EffectPreset =
            serde_json::from_str(include_str!("../../../samples/front-dimmer-pulse.effect"))
                .unwrap();
        validate_effect_preset(&pulse).unwrap();
        let (pulse_label, pulse_json) = sample_effect_preset_json("pulse").unwrap();
        assert_eq!(pulse_label, SAMPLE_EFFECT_PRESET_PULSE_LABEL);
        assert_eq!(pulse_json, SAMPLE_EFFECT_PRESET_PULSE_JSON);
        assert_eq!(pulse.effect_type, EffectKind::Lfo);
        assert_eq!(
            pulse.lfo.as_ref().unwrap().target_group_ids,
            vec!["Front".to_string()]
        );
        assert_eq!(pulse.lfo.as_ref().unwrap().attribute, "Dimmer");

        let shared: EffectPreset =
            serde_json::from_str(include_str!("../../../samples/front-dimmer-shared.effect"))
                .unwrap();
        validate_effect_preset(&shared).unwrap();
        let (shared_label, shared_json) = sample_effect_preset_json("shared").unwrap();
        assert_eq!(shared_label, SAMPLE_EFFECT_PRESET_SHARED_LABEL);
        assert_eq!(shared_json, SAMPLE_EFFECT_PRESET_SHARED_JSON);
        assert_eq!(shared.effect_type, EffectKind::Lfo);
        let shared_lfo = shared.lfo.as_ref().unwrap();
        assert_eq!(shared_lfo.target_group_ids, vec!["Front".to_string()]);
        assert_eq!(shared_lfo.attribute, "Dimmer");
        assert!(shared_lfo.video_targets.is_empty());
        assert_eq!(shared_lfo.period_ms, 1000);
        assert_eq!(
            shared_lfo.clock_sync,
            Some(protocol::EffectClockSync { beats: 2.0 })
        );
        assert_eq!(shared_lfo.phase, 0.25);

        let wave: EffectPreset =
            serde_json::from_str(include_str!("../../../samples/front-dimmer-wave.effect"))
                .unwrap();
        validate_effect_preset(&wave).unwrap();
        let (wave_label, wave_json) = sample_effect_preset_json("wave").unwrap();
        assert_eq!(wave_label, SAMPLE_EFFECT_PRESET_WAVE_LABEL);
        assert_eq!(wave_json, SAMPLE_EFFECT_PRESET_WAVE_JSON);
        assert_eq!(wave.effect_type, EffectKind::PositionWave);
        assert_eq!(
            wave.position_wave.as_ref().unwrap().target_group_ids,
            vec!["Front".to_string()]
        );
        assert_eq!(wave.position_wave.as_ref().unwrap().attribute, "Dimmer");

        let flash: EffectPreset =
            serde_json::from_str(include_str!("../../../samples/front-dimmer-flash.effect"))
                .unwrap();
        validate_effect_preset(&flash).unwrap();
        let (flash_label, flash_json) = sample_effect_preset_json("flash").unwrap();
        assert_eq!(flash_label, SAMPLE_EFFECT_PRESET_FLASH_LABEL);
        assert_eq!(flash_json, SAMPLE_EFFECT_PRESET_FLASH_JSON);
        assert_eq!(flash.effect_type, EffectKind::Lfo);
        let flash_lfo = flash.lfo.as_ref().unwrap();
        assert_eq!(flash_lfo.target_group_ids, vec!["Front".to_string()]);
        assert_eq!(flash_lfo.attribute, "Dimmer");
        assert_eq!(flash_lfo.shape, protocol::LfoShape::Square);
        assert_eq!(flash_lfo.period_ms, 125);
        assert_eq!(
            flash_lfo.clock_sync,
            Some(protocol::EffectClockSync { beats: 0.25 })
        );

        let random: EffectPreset =
            serde_json::from_str(include_str!("../../../samples/front-dimmer-random.effect"))
                .unwrap();
        validate_effect_preset(&random).unwrap();
        let (random_label, random_json) = sample_effect_preset_json("random").unwrap();
        assert_eq!(random_label, SAMPLE_EFFECT_PRESET_RANDOM_LABEL);
        assert_eq!(random_json, SAMPLE_EFFECT_PRESET_RANDOM_JSON);
        assert_eq!(random.effect_type, EffectKind::Lfo);
        let random_lfo = random.lfo.as_ref().unwrap();
        assert_eq!(random_lfo.target_group_ids, vec!["Front".to_string()]);
        assert_eq!(random_lfo.attribute, "Dimmer");
        assert_eq!(random_lfo.shape, protocol::LfoShape::Random);
        assert_eq!(random_lfo.period_ms, 1000);
        assert_eq!(
            random_lfo.clock_sync,
            Some(protocol::EffectClockSync { beats: 1.0 })
        );

        let perlin: EffectPreset =
            serde_json::from_str(include_str!("../../../samples/front-dimmer-perlin.effect"))
                .unwrap();
        validate_effect_preset(&perlin).unwrap();
        let (perlin_label, perlin_json) = sample_effect_preset_json("perlin").unwrap();
        assert_eq!(perlin_label, SAMPLE_EFFECT_PRESET_PERLIN_LABEL);
        assert_eq!(perlin_json, SAMPLE_EFFECT_PRESET_PERLIN_JSON);
        assert_eq!(perlin.effect_type, EffectKind::Lfo);
        let perlin_lfo = perlin.lfo.as_ref().unwrap();
        assert_eq!(perlin_lfo.target_group_ids, vec!["Front".to_string()]);
        assert_eq!(perlin_lfo.attribute, "Dimmer");
        assert_eq!(perlin_lfo.shape, protocol::LfoShape::Perlin);
        assert_eq!(perlin_lfo.period_ms, 4000);
        assert_eq!(
            perlin_lfo.clock_sync,
            Some(protocol::EffectClockSync { beats: 8.0 })
        );

        let chase: EffectPreset =
            serde_json::from_str(include_str!("../../../samples/front-dimmer-chase.effect"))
                .unwrap();
        validate_effect_preset(&chase).unwrap();
        let (chase_label, chase_json) = sample_effect_preset_json("chase").unwrap();
        assert_eq!(chase_label, SAMPLE_EFFECT_PRESET_CHASE_LABEL);
        assert_eq!(chase_json, SAMPLE_EFFECT_PRESET_CHASE_JSON);
        assert_eq!(chase.effect_type, EffectKind::PositionWave);
        let chase_wave = chase.position_wave.as_ref().unwrap();
        assert_eq!(chase_wave.target_group_ids, vec!["Front".to_string()]);
        assert_eq!(chase_wave.attribute, "Dimmer");
        assert_eq!(chase_wave.shape, protocol::LfoShape::Square);
        assert_eq!(chase_wave.wavelength, 2.0);
        assert_eq!(
            chase_wave.clock_sync,
            Some(protocol::EffectClockSync { beats: 2.0 })
        );

        let ball: EffectPreset =
            serde_json::from_str(include_str!("../../../samples/front-dimmer-ball.effect"))
                .unwrap();
        validate_effect_preset(&ball).unwrap();
        let (ball_label, ball_json) = sample_effect_preset_json("ball").unwrap();
        assert_eq!(ball_label, SAMPLE_EFFECT_PRESET_BALL_LABEL);
        assert_eq!(ball_json, SAMPLE_EFFECT_PRESET_BALL_JSON);
        assert_eq!(ball.effect_type, EffectKind::PositionWave);
        let ball_wave = ball.position_wave.as_ref().unwrap();
        assert_eq!(ball_wave.target_group_ids, vec!["Front".to_string()]);
        assert_eq!(ball_wave.attribute, "Dimmer");
        assert_eq!(ball_wave.shape, protocol::LfoShape::Sine);
        assert_eq!(ball_wave.direction, Vec3::default());
        assert_eq!(ball_wave.speed, 1.0);
        assert_eq!(ball_wave.wavelength, 3.0);
        assert_eq!(
            ball_wave.clock_sync,
            Some(protocol::EffectClockSync { beats: 4.0 })
        );

        let fan: EffectPreset =
            serde_json::from_str(include_str!("../../../samples/front-pan-fan.effect")).unwrap();
        validate_effect_preset(&fan).unwrap();
        let (fan_label, fan_json) = sample_effect_preset_json("fan").unwrap();
        assert_eq!(fan_label, SAMPLE_EFFECT_PRESET_FAN_LABEL);
        assert_eq!(fan_json, SAMPLE_EFFECT_PRESET_FAN_JSON);
        assert_eq!(fan.effect_type, EffectKind::PositionWave);
        let fan_wave = fan.position_wave.as_ref().unwrap();
        assert_eq!(fan_wave.target_group_ids, vec!["Front".to_string()]);
        assert_eq!(fan_wave.attribute, "Pan");
        assert_eq!(fan_wave.shape, protocol::LfoShape::Triangle);
        assert_eq!(fan_wave.speed, 0.0);
        assert_eq!(fan_wave.wavelength, 6.0);
        assert_eq!(fan_wave.low, 24576);
        assert_eq!(fan_wave.high, 40960);
        assert_eq!(fan_wave.clock_sync, None);

        let circle_bundle = sample_effect_bundle_jsons("circle").unwrap();
        assert_eq!(circle_bundle.len(), 2);
        assert_eq!(circle_bundle[0].0, SAMPLE_EFFECT_PRESET_CIRCLE_PAN_LABEL);
        assert_eq!(circle_bundle[0].1, SAMPLE_EFFECT_PRESET_CIRCLE_PAN_JSON);
        assert_eq!(circle_bundle[1].0, SAMPLE_EFFECT_PRESET_CIRCLE_TILT_LABEL);
        assert_eq!(circle_bundle[1].1, SAMPLE_EFFECT_PRESET_CIRCLE_TILT_JSON);
        let circle_pan: EffectPreset = serde_json::from_str(circle_bundle[0].1).unwrap();
        let circle_tilt: EffectPreset = serde_json::from_str(circle_bundle[1].1).unwrap();
        validate_effect_preset(&circle_pan).unwrap();
        validate_effect_preset(&circle_tilt).unwrap();
        let circle_pan_lfo = circle_pan.lfo.as_ref().unwrap();
        let circle_tilt_lfo = circle_tilt.lfo.as_ref().unwrap();
        assert_eq!(circle_pan.effect_type, EffectKind::Lfo);
        assert_eq!(circle_tilt.effect_type, EffectKind::Lfo);
        assert_eq!(circle_pan_lfo.target_group_ids, vec!["Front".to_string()]);
        assert_eq!(circle_tilt_lfo.target_group_ids, vec!["Front".to_string()]);
        assert_eq!(circle_pan_lfo.attribute, "Pan");
        assert_eq!(circle_tilt_lfo.attribute, "Tilt");
        assert_eq!(circle_pan_lfo.shape, protocol::LfoShape::Sine);
        assert_eq!(circle_tilt_lfo.shape, protocol::LfoShape::Cosine);
        assert_eq!(circle_pan_lfo.period_ms, 2000);
        assert_eq!(circle_tilt_lfo.period_ms, 2000);
        assert_eq!(
            circle_pan_lfo.clock_sync,
            Some(protocol::EffectClockSync { beats: 4.0 })
        );
        assert_eq!(
            circle_tilt_lfo.clock_sync,
            Some(protocol::EffectClockSync { beats: 4.0 })
        );
        assert_eq!(circle_pan_lfo.low, 24576);
        assert_eq!(circle_pan_lfo.high, 40960);
        assert_eq!(circle_tilt_lfo.low, 24576);
        assert_eq!(circle_tilt_lfo.high, 40960);
        assert!(sample_effect_preset_json("missing").is_err());
        assert!(sample_effect_bundle_jsons("missing").is_err());
    }

    #[test]
    fn embedded_sample_effect_presets_can_retarget_current_effect_target() {
        let target_override = EffectTargetOverride {
            fixture_ids: vec![42],
            target_group_ids: vec!["floor".to_string()],
            attribute: "ColorRed".to_string(),
            video_targets: vec![sample_video_effect_target(7)],
        };
        validate_effect_target_override(&target_override).unwrap();

        let (_, pulse_json) = sample_effect_preset_json("pulse").unwrap();
        let pulse: EffectPreset = serde_json::from_str(pulse_json).unwrap();
        validate_effect_preset(&pulse).unwrap();
        let pulse_request = apply_lfo_effect_target_override(pulse.lfo.unwrap(), &target_override);
        validate_lfo_effect_request(&pulse_request).unwrap();
        assert_eq!(pulse_request.fixture_ids, vec![42]);
        assert_eq!(pulse_request.target_group_ids, vec!["floor".to_string()]);
        assert_eq!(pulse_request.attribute, "ColorRed");
        assert_eq!(
            pulse_request.video_targets,
            vec![sample_video_effect_target(7)]
        );
        assert_eq!(pulse_request.period_ms, 500);
        assert_eq!(
            pulse_request.clock_sync,
            Some(protocol::EffectClockSync { beats: 1.0 })
        );

        let (_, shared_json) = sample_effect_preset_json("front_dimmer_shared").unwrap();
        let shared: EffectPreset = serde_json::from_str(shared_json).unwrap();
        validate_effect_preset(&shared).unwrap();
        let shared_request =
            apply_lfo_effect_target_override(shared.lfo.unwrap(), &target_override);
        validate_lfo_effect_request(&shared_request).unwrap();
        assert_eq!(shared_request.fixture_ids, vec![42]);
        assert_eq!(shared_request.target_group_ids, vec!["floor".to_string()]);
        assert_eq!(shared_request.attribute, "ColorRed");
        assert_eq!(
            shared_request.video_targets,
            vec![sample_video_effect_target(7)]
        );
        assert_eq!(shared_request.period_ms, 1000);
        assert_eq!(
            shared_request.clock_sync,
            Some(protocol::EffectClockSync { beats: 2.0 })
        );
        assert_eq!(shared_request.phase, 0.25);

        let (_, flash_json) = sample_effect_preset_json("front_dimmer_flash").unwrap();
        let flash: EffectPreset = serde_json::from_str(flash_json).unwrap();
        validate_effect_preset(&flash).unwrap();
        let flash_request = apply_lfo_effect_target_override(flash.lfo.unwrap(), &target_override);
        validate_lfo_effect_request(&flash_request).unwrap();
        assert_eq!(flash_request.fixture_ids, vec![42]);
        assert_eq!(flash_request.target_group_ids, vec!["floor".to_string()]);
        assert_eq!(flash_request.attribute, "ColorRed");
        assert_eq!(
            flash_request.video_targets,
            vec![sample_video_effect_target(7)]
        );
        assert_eq!(flash_request.shape, protocol::LfoShape::Square);
        assert_eq!(flash_request.period_ms, 125);
        assert_eq!(
            flash_request.clock_sync,
            Some(protocol::EffectClockSync { beats: 0.25 })
        );

        let (_, random_json) = sample_effect_preset_json("front-dimmer-random").unwrap();
        let random: EffectPreset = serde_json::from_str(random_json).unwrap();
        validate_effect_preset(&random).unwrap();
        let random_request =
            apply_lfo_effect_target_override(random.lfo.unwrap(), &target_override);
        validate_lfo_effect_request(&random_request).unwrap();
        assert_eq!(random_request.fixture_ids, vec![42]);
        assert_eq!(random_request.target_group_ids, vec!["floor".to_string()]);
        assert_eq!(random_request.attribute, "ColorRed");
        assert_eq!(
            random_request.video_targets,
            vec![sample_video_effect_target(7)]
        );
        assert_eq!(random_request.shape, protocol::LfoShape::Random);
        assert_eq!(random_request.period_ms, 1000);
        assert_eq!(
            random_request.clock_sync,
            Some(protocol::EffectClockSync { beats: 1.0 })
        );

        let (_, perlin_json) = sample_effect_preset_json("front_dimmer_perlin").unwrap();
        let perlin: EffectPreset = serde_json::from_str(perlin_json).unwrap();
        validate_effect_preset(&perlin).unwrap();
        let perlin_request =
            apply_lfo_effect_target_override(perlin.lfo.unwrap(), &target_override);
        validate_lfo_effect_request(&perlin_request).unwrap();
        assert_eq!(perlin_request.fixture_ids, vec![42]);
        assert_eq!(perlin_request.target_group_ids, vec!["floor".to_string()]);
        assert_eq!(perlin_request.attribute, "ColorRed");
        assert_eq!(
            perlin_request.video_targets,
            vec![sample_video_effect_target(7)]
        );
        assert_eq!(perlin_request.shape, protocol::LfoShape::Perlin);
        assert_eq!(perlin_request.period_ms, 4000);
        assert_eq!(
            perlin_request.clock_sync,
            Some(protocol::EffectClockSync { beats: 8.0 })
        );

        let (_, wave_json) = sample_effect_preset_json("wave").unwrap();
        let wave: EffectPreset = serde_json::from_str(wave_json).unwrap();
        validate_effect_preset(&wave).unwrap();
        let wave_request = apply_position_wave_effect_target_override(
            wave.position_wave.unwrap(),
            &target_override,
        );
        validate_position_wave_effect_request(&wave_request).unwrap();
        assert_eq!(wave_request.fixture_ids, vec![42]);
        assert_eq!(wave_request.target_group_ids, vec!["floor".to_string()]);
        assert_eq!(wave_request.attribute, "ColorRed");
        assert_eq!(
            wave_request.video_targets,
            vec![sample_video_effect_target(7)]
        );
        assert_eq!(wave_request.origin, Vec3::default());
        assert_eq!(
            wave_request.direction,
            Vec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }
        );
        assert_eq!(wave_request.wavelength, 4.0);
        assert_eq!(
            wave_request.clock_sync,
            Some(protocol::EffectClockSync { beats: 4.0 })
        );

        let (_, chase_json) = sample_effect_preset_json("front-dimmer-chase").unwrap();
        let chase: EffectPreset = serde_json::from_str(chase_json).unwrap();
        validate_effect_preset(&chase).unwrap();
        let chase_request = apply_position_wave_effect_target_override(
            chase.position_wave.unwrap(),
            &target_override,
        );
        validate_position_wave_effect_request(&chase_request).unwrap();
        assert_eq!(chase_request.fixture_ids, vec![42]);
        assert_eq!(chase_request.target_group_ids, vec!["floor".to_string()]);
        assert_eq!(chase_request.attribute, "ColorRed");
        assert_eq!(
            chase_request.video_targets,
            vec![sample_video_effect_target(7)]
        );
        assert_eq!(chase_request.shape, protocol::LfoShape::Square);
        assert_eq!(chase_request.wavelength, 2.0);
        assert_eq!(
            chase_request.clock_sync,
            Some(protocol::EffectClockSync { beats: 2.0 })
        );

        let (_, ball_json) = sample_effect_preset_json("front_dimmer_ball").unwrap();
        let ball: EffectPreset = serde_json::from_str(ball_json).unwrap();
        validate_effect_preset(&ball).unwrap();
        let ball_request = apply_position_wave_effect_target_override(
            ball.position_wave.unwrap(),
            &target_override,
        );
        validate_position_wave_effect_request(&ball_request).unwrap();
        assert_eq!(ball_request.fixture_ids, vec![42]);
        assert_eq!(ball_request.target_group_ids, vec!["floor".to_string()]);
        assert_eq!(ball_request.attribute, "ColorRed");
        assert_eq!(
            ball_request.video_targets,
            vec![sample_video_effect_target(7)]
        );
        assert_eq!(ball_request.shape, protocol::LfoShape::Sine);
        assert_eq!(ball_request.direction, Vec3::default());
        assert_eq!(ball_request.speed, 1.0);
        assert_eq!(ball_request.wavelength, 3.0);
        assert_eq!(
            ball_request.clock_sync,
            Some(protocol::EffectClockSync { beats: 4.0 })
        );

        let (_, fan_json) = sample_effect_preset_json("front-pan-fan").unwrap();
        let fan: EffectPreset = serde_json::from_str(fan_json).unwrap();
        validate_effect_preset(&fan).unwrap();
        let fan_request = apply_position_wave_effect_target_override(
            fan.position_wave.unwrap(),
            &target_override,
        );
        validate_position_wave_effect_request(&fan_request).unwrap();
        assert_eq!(fan_request.fixture_ids, vec![42]);
        assert_eq!(fan_request.target_group_ids, vec!["floor".to_string()]);
        assert_eq!(fan_request.attribute, "ColorRed");
        assert_eq!(
            fan_request.video_targets,
            vec![sample_video_effect_target(7)]
        );
        assert_eq!(fan_request.shape, protocol::LfoShape::Triangle);
        assert_eq!(fan_request.speed, 0.0);
        assert_eq!(fan_request.wavelength, 6.0);
        assert_eq!(fan_request.low, 24576);
        assert_eq!(fan_request.high, 40960);
        assert_eq!(fan_request.clock_sync, None);
    }

    #[test]
    fn embedded_sample_effect_preset_adds_retargeted_effect_to_engine() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let profile = custom_fixture_profile_from_request(CustomFixtureProfileRequest {
            manufacturer: "Rayard".to_string(),
            name: "Preset Target Spot".to_string(),
            mode_name: "8ch".to_string(),
            attributes: vec!["Dimmer".to_string(), "ColorRed".to_string()],
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: profile.source_path.clone(),
                    mode_name: Some("8ch".to_string()),
                    label: "Preset Target".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Rotation3::default(),
                },
                profile,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .fixtures
                .iter()
                .any(|fixture| fixture.id == fixture_id)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
            snapshot = engine.snapshot();
        }
        assert!(snapshot
            .fixtures
            .iter()
            .any(|fixture| fixture.id == fixture_id));

        let (_, pulse_json) = sample_effect_preset_json("pulse").unwrap();
        let pulse: EffectPreset = serde_json::from_str(pulse_json).unwrap();
        let target_override = EffectTargetOverride {
            fixture_ids: vec![fixture_id],
            target_group_ids: Vec::new(),
            attribute: "Dimmer".to_string(),
            video_targets: Vec::new(),
        };
        let effect_id =
            add_effect_preset_to_engine(&engine, pulse, Some(&target_override)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.effects.iter().any(|effect| effect.id == effect_id) {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
            snapshot = engine.snapshot();
        }
        let effect = snapshot
            .effects
            .iter()
            .find(|effect| effect.id == effect_id)
            .expect("retargeted sample effect should be in the engine snapshot");
        assert_eq!(effect.effect_type, EffectKind::Lfo);
        assert_eq!(effect.fixture_ids, vec![fixture_id]);
        assert!(effect.target_group_ids.is_empty());
        assert_eq!(effect.attribute, "Dimmer");
        assert_eq!(effect.period_ms, Some(500));
        assert_eq!(
            effect.clock_sync,
            Some(protocol::EffectClockSync { beats: 1.0 })
        );
        assert!(effect.enabled);
    }

    #[test]
    fn duplicate_effect_copies_lfo_preset_details_into_new_stack_item() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let profile = custom_fixture_profile_from_request(CustomFixtureProfileRequest {
            manufacturer: "Rayard".to_string(),
            name: "Duplicate Target Spot".to_string(),
            mode_name: "8ch".to_string(),
            attributes: vec!["Dimmer".to_string()],
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: profile.source_path.clone(),
                    mode_name: Some("8ch".to_string()),
                    label: "Duplicate Target".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Rotation3::default(),
                },
                profile,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .fixtures
                .iter()
                .any(|fixture| fixture.id == fixture_id)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
            snapshot = engine.snapshot();
        }
        assert!(snapshot
            .fixtures
            .iter()
            .any(|fixture| fixture.id == fixture_id));

        let source_id = add_effect_preset_to_engine(
            &engine,
            EffectPreset {
                version: 1,
                effect_type: EffectKind::Lfo,
                enabled: false,
                lfo: Some(LfoEffectRequest {
                    label: "Dimmer pulse".to_string(),
                    fixture_ids: vec![fixture_id],
                    target_group_ids: Vec::new(),
                    attribute: "Dimmer".to_string(),
                    video_targets: Vec::new(),
                    shape: protocol::LfoShape::Square,
                    period_ms: 250,
                    clock_sync: Some(protocol::EffectClockSync { beats: 0.5 }),
                    low: 1_024,
                    high: 62_000,
                    phase: 0.125,
                    blend_mode: protocol::EffectBlendMode::Add,
                }),
                position_wave: None,
            },
            None,
        )
        .unwrap();

        snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.effects.iter().any(|effect| effect.id == source_id) {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
            snapshot = engine.snapshot();
        }
        assert!(snapshot.effects.iter().any(|effect| effect.id == source_id));

        let duplicate_id = duplicate_effect_in_engine(&engine, source_id).unwrap();
        assert_ne!(duplicate_id, source_id);

        snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .effects
                .iter()
                .any(|effect| effect.id == duplicate_id)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
            snapshot = engine.snapshot();
        }

        let source = snapshot
            .effects
            .iter()
            .find(|effect| effect.id == source_id)
            .expect("source effect should remain in the engine snapshot");
        let duplicate = snapshot
            .effects
            .iter()
            .find(|effect| effect.id == duplicate_id)
            .expect("duplicated effect should be in the engine snapshot");

        assert_eq!(duplicate.label, "Dimmer pulse Copy");
        assert_eq!(duplicate.effect_type, source.effect_type);
        assert_eq!(duplicate.fixture_ids, source.fixture_ids);
        assert_eq!(duplicate.attribute, source.attribute);
        assert_eq!(duplicate.shape, source.shape);
        assert_eq!(duplicate.period_ms, source.period_ms);
        assert_eq!(duplicate.clock_sync, source.clock_sync);
        assert_eq!(duplicate.low, source.low);
        assert_eq!(duplicate.high, source.high);
        assert_eq!(duplicate.phase, source.phase);
        assert_eq!(duplicate.blend_mode, source.blend_mode);
        assert_eq!(duplicate.enabled, source.enabled);
    }

    #[test]
    fn embedded_sample_effect_bundle_adds_circle_pair_to_engine() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let profile = custom_fixture_profile_from_request(CustomFixtureProfileRequest {
            manufacturer: "Rayard".to_string(),
            name: "Circle Target Spot".to_string(),
            mode_name: "8ch".to_string(),
            attributes: vec!["Pan".to_string(), "Tilt".to_string()],
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: profile.source_path.clone(),
                    mode_name: Some("8ch".to_string()),
                    label: "Circle Target".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: vec!["Front".to_string()],
                    position: Vec3::default(),
                    rotation: Rotation3::default(),
                },
                profile,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .fixtures
                .iter()
                .any(|fixture| fixture.id == fixture_id)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
            snapshot = engine.snapshot();
        }
        assert!(snapshot
            .fixtures
            .iter()
            .any(|fixture| fixture.id == fixture_id));

        let mut effect_ids = Vec::new();
        for (_, json) in sample_effect_bundle_jsons("front_circle").unwrap() {
            let preset: EffectPreset = serde_json::from_str(json).unwrap();
            let effect_id = add_effect_preset_to_engine(&engine, preset, None).unwrap();
            effect_ids.push(effect_id);
        }
        assert_eq!(effect_ids.len(), 2);

        snapshot = engine.snapshot();
        for _ in 0..20 {
            if effect_ids.iter().all(|effect_id| {
                snapshot
                    .effects
                    .iter()
                    .any(|effect| effect.id == *effect_id)
            }) {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
            snapshot = engine.snapshot();
        }

        let circle_effects: Vec<&EffectSummary> = effect_ids
            .iter()
            .map(|effect_id| {
                snapshot
                    .effects
                    .iter()
                    .find(|effect| effect.id == *effect_id)
                    .expect("circle bundle effect should be in the engine snapshot")
            })
            .collect();
        assert_eq!(circle_effects[0].effect_type, EffectKind::Lfo);
        assert_eq!(circle_effects[1].effect_type, EffectKind::Lfo);
        assert_eq!(circle_effects[0].attribute, "Pan");
        assert_eq!(circle_effects[1].attribute, "Tilt");
        assert_eq!(circle_effects[0].shape, protocol::LfoShape::Sine);
        assert_eq!(circle_effects[1].shape, protocol::LfoShape::Cosine);
        assert_eq!(circle_effects[0].period_ms, Some(2000));
        assert_eq!(circle_effects[1].period_ms, Some(2000));
        assert_eq!(
            circle_effects[0].clock_sync,
            Some(protocol::EffectClockSync { beats: 4.0 })
        );
        assert_eq!(
            circle_effects[1].clock_sync,
            Some(protocol::EffectClockSync { beats: 4.0 })
        );
        assert_eq!(circle_effects[0].low, 24576);
        assert_eq!(circle_effects[0].high, 40960);
        assert_eq!(circle_effects[1].low, 24576);
        assert_eq!(circle_effects[1].high, 40960);
    }
}

fn main() {
    let engine = EngineHandle::start(DmxOutputConfig::default());
    tauri::Builder::default()
        .manage(AppState {
            engine,
            video_preview: Mutex::new(video::VideoPreviewRenderer::with_frame_provider(
                video::VideoRuntimeConfig::default(),
                video::DecoderBackedFrameProvider::new(video::FfmpegCliFrameDecoder::from_env())
                    .with_prefetch(2, 33),
            )),
            external_video_transport: Arc::new(Mutex::new(
                video::ExternalVideoTransportRuntime::new(),
            )),
            external_video_transport_events: Arc::new(Mutex::new(Vec::new())),
            custom_profiles: Mutex::new(HashMap::new()),
            visualizer_model_assets: Mutex::new(HashMap::new()),
            midi_clock: Mutex::new(None),
            midi_control: Mutex::new(None),
            midi_feedback: Mutex::new(None),
            osc_input: Mutex::new(None),
            remote_control: Mutex::new(None),
            pending_project_open_paths: Mutex::new(Vec::new()),
            current_project_path: Mutex::new(None),
        })
        .plugin(tauri_plugin_single_instance::init(|app_handle, args, cwd| {
            let project_paths = project_paths_from_single_instance_args(args, &cwd);
            if project_paths.is_empty() {
                return;
            }
            if let Ok(mut pending_paths) = app_handle
                .state::<AppState>()
                .pending_project_open_paths
                .lock()
            {
                pending_paths.extend(project_paths.clone());
            }
            let _ = app_handle.emit(OPEN_PROJECT_EVENT, project_paths);
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![
            select_gdtf_file,
            select_video_source_file,
            import_gdtf,
            load_gdtf_wheel_media,
            load_gdtf_model_file,
            download_gdtf_from_url,
            search_gdtf_share,
            download_gdtf_from_share,
            create_custom_fixture_profile,
            save_custom_fixture_profile,
            load_custom_fixture_profile,
            use_fixture_profile,
            patch_fixture,
            patch_fixtures,
            remove_fixture,
            set_fixture_patch,
            set_fixture_limits,
            set_group_fixture_limits,
            set_fixture_groups,
            set_attribute,
            set_group_attribute,
            set_group_highlight,
            set_group_solo,
            set_group_park,
            set_fixture_transform,
            set_stage_map_config,
            set_fixture_highlight,
            set_fixture_solo,
            set_fixture_park,
            clear_fixture_flags,
            set_output_config,
            set_dmx_outputs,
            send_dmx_test_frame,
            send_dmx_routes_test_frame,
            set_blackout,
            set_all_blackout,
            set_lighting_master,
            set_group_submaster,
            reset_engine_telemetry,
            save_engine_telemetry_report,
            get_engine_telemetry_report,
            set_bpm,
            tap_bpm,
            sync_ableton_link_clock,
            analyze_audio_file,
            clear_timeline_audio,
            list_midi_inputs,
            list_midi_outputs,
            list_serial_ports,
            connect_midi_clock,
            disconnect_midi_clock,
            connect_midi_control,
            disconnect_midi_control,
            connect_midi_feedback,
            disconnect_midi_feedback,
            send_midi_feedback,
            learn_midi_control,
            save_midi_mappings,
            load_midi_mappings,
            save_osc_mappings,
            load_osc_mappings,
            learn_osc_control,
            start_osc_input,
            stop_osc_input,
            remote_access_urls,
            start_remote_control,
            stop_remote_control,
            create_cue_from_current,
            update_cue_from_current,
            set_cue_metadata,
            move_cue,
            duplicate_cue,
            trigger_cue,
            trigger_next_cue,
            trigger_previous_cue,
            set_cue_fade_paused,
            remove_cue,
            add_timeline_cue_event,
            set_timeline_cue_event,
            remove_timeline_event,
            add_timeline_automation,
            add_timeline_group_automation,
            set_timeline_automation,
            add_timeline_video_automation,
            set_timeline_video_automation,
            set_timeline_automation_enabled,
            remove_timeline_automation,
            set_timeline_playing,
            seek_timeline,
            seek_timeline_beat,
            sync_ltc_timecode,
            add_video_file_layer,
            add_still_image_layer,
            refresh_video_layer_metadata,
            add_video_input_layer,
            duplicate_video_layer,
            remove_video_layer,
            set_video_layer_order,
            set_video_layer_label,
            set_video_layer_state,
            fade_video_layer_opacity,
            add_video_cue_point,
            remove_video_cue_point,
            set_video_cue_point,
            jump_video_cue_point,
            jump_video_cue_point_relative,
            set_video_layer_blend_mode,
            set_video_master_opacity,
            set_video_blackout,
            add_video_composition,
            remove_video_composition,
            set_video_composition_layers,
            add_video_output,
            remove_video_output,
            set_video_output_config,
            set_video_output_enabled,
            set_video_output_routing,
            set_video_output_opacity,
            fade_video_output_opacity,
            set_video_output_blackout,
            set_video_output_mapping,
            set_video_output_mapping_field,
            save_video_output_mapping_preset,
            apply_video_output_mapping_preset,
            remove_video_output_mapping_preset,
            save_video_output_mapping_preset_file,
            load_video_output_mapping_preset_file,
            add_lfo_effect,
            add_position_wave_effect,
            update_lfo_effect,
            update_position_wave_effect,
            save_node_graph,
            set_node_graph_enabled,
            remove_node_graph,
            save_node_graph_preset_file,
            load_node_graph_preset_file,
            set_effect_enabled,
            set_effect_video_target_position,
            move_effect,
            duplicate_effect,
            remove_effect,
            save_effect_preset,
            load_effect_preset,
            load_effect_preset_for_target,
            load_sample_effect_preset,
            load_sample_effect_bundle,
            save_fixture_preset,
            load_fixture_preset,
            load_fixture_preset_for_group,
            load_fixture_preset_for_all_matching,
            new_project,
            save_project,
            save_project_as,
            load_project,
            load_project_path,
            load_startup_project,
            load_phase1_sample_project,
            run_phase1_smoke,
            take_open_project_paths,
            get_snapshot,
            get_visualizer_scene,
            save_stage_map_preset,
            apply_stage_map_preset,
            remove_stage_map_preset,
            add_stage_object,
            set_stage_object,
            remove_stage_object,
            save_stage_map_preset_file,
            load_stage_map_preset_file,
            get_visualizer_model_render_plans,
            get_visualizer_render_payload,
            get_visualizer_external_model_assets,
            get_visualizer_resolved_render_payload,
            get_visualizer_model_asset_cache_summary,
            get_video_composition_plans,
            get_video_output_render_plans,
            get_external_video_io_plans,
            get_external_video_transport_status,
            sync_external_video_transports,
            get_video_runtime_status,
            get_video_preview_diagnostics,
            get_debug_video_preview,
            get_debug_video_output_preview,
            get_debug_video_output_test_pattern,
            get_video_output_window_statuses,
            sync_open_video_output_windows,
            close_video_output_window,
            close_open_video_output_windows,
            sync_video_output_window,
            open_video_output_window
        ])
        .build(tauri::generate_context!())
        .expect("error while building Rayard")
        .run(|app_handle, event| {
            #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
            {
                let _ = app_handle;
                let _ = event;
            }
            #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
            if let tauri::RunEvent::Opened { urls } = event {
                let project_paths = urls
                    .into_iter()
                    .filter_map(|url| {
                        if url.scheme() != "file" {
                            return None;
                        }
                        let path = url.to_file_path().ok()?;
                        is_rayard_project_path(&path).then(|| path.to_string_lossy().to_string())
                    })
                    .collect::<Vec<_>>();
                if !project_paths.is_empty() {
                    if let Ok(mut pending_paths) = app_handle
                        .state::<AppState>()
                        .pending_project_open_paths
                        .lock()
                    {
                        pending_paths.extend(project_paths.clone());
                    }
                    let _ = app_handle.emit(OPEN_PROJECT_EVENT, project_paths);
                }
            }
        });
}

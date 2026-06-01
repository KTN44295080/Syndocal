use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    process::Command,
    sync::Mutex,
    time::Duration,
};

use engine::{EngineCommand, EngineHandle};
use io::midi::{
    MidiClockEvent, MidiClockInput, MidiControlEvent, MidiControlInput, MidiFeedbackOutput,
};
use io::osc::{OscInput, OscInputEvent};
use io::remote_ws::{RemoteInputEvent, RemoteWsServer};
use io::sacn::is_sacn_multicast_target;
use protocol::{
    AttributeControl, AttributeResolution, AudioAnalysisSummary, AutomationId,
    AutomationKeyframeSummary, CompositionId, CompositionSummary, CueFixtureTarget, CueId,
    CustomFixtureProfileFile, CustomFixtureProfileRequest, DmxModeSummary, DmxOutputConfig,
    DmxOutputProtocol, EffectId, EffectKind, EffectPreset, EffectSummary, EngineSnapshot,
    FixtureId, FixtureLimits, FixturePreset, FixtureProfileSummary, GeometrySummary,
    LearnedMidiControl, LearnedOscControl, LfoEffectRequest, MidiControlMapping, MidiInputSummary,
    MidiOutputSummary, OscControlMapping, OscInputConfig, PatchFixtureRequest,
    PatchedFixtureSummary, PositionWaveEffectRequest, ProjectFile, RemoteControlConfig, Rotation3,
    SerialPortSummary, TimelineEventId, TimelineTrackKind, Vec3, VideoAutomationKeyframeSummary,
    VideoBlendMode, VideoEffectTarget, VideoLayerId, VideoLayerState, VideoLayerTarget,
    VideoOutputId, VideoOutputKind, VideoOutputMapping, VideoOutputMappingPresetFile,
    VideoOutputMappingPresetSummary, VideoOutputSummary, VideoOutputTarget, VideoParam,
    VideoSourceKind, VideoSourceSummary,
};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

type AppVideoPreviewRenderer =
    video::VideoPreviewRenderer<video::DecoderBackedFrameProvider<video::FfmpegCliFrameDecoder>>;

struct AppState {
    engine: EngineHandle,
    video_preview: Mutex<AppVideoPreviewRenderer>,
    custom_profiles: Mutex<HashMap<String, FixtureProfileSummary>>,
    midi_clock: Mutex<Option<MidiClockInput>>,
    midi_control: Mutex<Option<MidiControlInput>>,
    midi_feedback: Mutex<Option<MidiFeedbackOutput>>,
    osc_input: Mutex<Option<OscInput>>,
    remote_control: Mutex<Option<RemoteWsServer>>,
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
fn import_gdtf(path: String) -> Result<FixtureProfileSummary, String> {
    gdtf::load_profile(path).map_err(|error| error.to_string())
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
        .add_filter("KDMX Fixture Profile", &["fixture"])
        .set_file_name(format!(
            "{}.fixture",
            safe_file_stem(&profile_file.request.name)
        ))
        .save_file()
    else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(&profile_file).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn load_custom_fixture_profile(
    state: State<'_, AppState>,
) -> Result<Option<FixtureProfileSummary>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("KDMX Fixture Profile", &["fixture"])
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
    if attribute.trim().is_empty() {
        return Err("Attribute is required".to_string());
    }
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
fn set_blackout(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::Blackout(enabled))
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
    if group_id.trim().is_empty() {
        return Err("Submaster group id is required".to_string());
    }
    if !level.is_finite() {
        return Err("Submaster level must be finite".to_string());
    }
    state
        .engine
        .send(EngineCommand::SetGroupSubmaster { group_id, level })
        .map_err(|error| error.to_string())
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
fn analyze_audio_file(state: State<'_, AppState>) -> Result<Option<AudioAnalysisSummary>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("WAV Audio", &["wav"])
        .pick_file()
    else {
        return Ok(None);
    };
    let analysis = audio::analyze_wav_file(&path).map_err(|error| error.to_string())?;
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
        MidiClockEvent::Timecode(timecode) => {
            let _ = engine.send(EngineCommand::SyncTimelineTimecode(timecode.position_ms()));
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
            MidiControlEvent::TriggerCue(cue_id) => EngineCommand::TriggerCue(cue_id),
            MidiControlEvent::TriggerNextCue => EngineCommand::TriggerNextCue,
            MidiControlEvent::TriggerPreviousCue => EngineCommand::TriggerPreviousCue,
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
            MidiControlEvent::LightingMaster(master) => EngineCommand::SetLightingMaster(master),
            MidiControlEvent::SetGroupSubmaster { group_id, level } => {
                EngineCommand::SetGroupSubmaster { group_id, level }
            }
            MidiControlEvent::SetCueFadePaused(paused) => EngineCommand::SetCueFadePaused(paused),
            MidiControlEvent::Blackout(enabled) => EngineCommand::Blackout(enabled),
            MidiControlEvent::VideoBlackout(enabled) => EngineCommand::SetVideoBlackout(enabled),
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
    let Some(path) = rfd::FileDialog::new()
        .add_filter("KDMX MIDI Mapping", &["midimap"])
        .set_file_name("kdmx.midimap")
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
        .add_filter("KDMX MIDI Mapping", &["midimap"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let file: MidiMappingFile = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    if file.version != 1 {
        return Err(format!("Unsupported MIDI mapping version {}", file.version));
    }
    Ok(Some(file.mappings))
}

#[tauri::command]
fn save_osc_mappings(mappings: Vec<OscControlMapping>) -> Result<Option<String>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("KDMX OSC Mapping", &["oscmap"])
        .set_file_name("kdmx.oscmap")
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
        .add_filter("KDMX OSC Mapping", &["oscmap"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let file: OscMappingFile = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    if file.version != 1 {
        return Err(format!("Unsupported OSC mapping version {}", file.version));
    }
    Ok(Some(file.mappings))
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
    let engine = state.engine.clone();
    let input = OscInput::start_with_mappings(config, mappings.unwrap_or_default(), move |event| {
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
            OscInputEvent::TriggerCue(cue_id) => EngineCommand::TriggerCue(cue_id),
            OscInputEvent::TriggerNextCue => EngineCommand::TriggerNextCue,
            OscInputEvent::TriggerPreviousCue => EngineCommand::TriggerPreviousCue,
            OscInputEvent::SetCueFadePaused(paused) => EngineCommand::SetCueFadePaused(paused),
            OscInputEvent::SetTimelinePlaying(playing) => {
                EngineCommand::SetTimelinePlaying(playing)
            }
            OscInputEvent::SeekTimeline { position_ms } => EngineCommand::SeekTimeline(position_ms),
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
fn start_remote_control(
    state: State<'_, AppState>,
    config: RemoteControlConfig,
) -> Result<(), String> {
    let command_engine = state.engine.clone();
    let snapshot_engine = state.engine.clone();
    let server = RemoteWsServer::start_with_snapshot(
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
                RemoteInputEvent::Blackout(enabled) => EngineCommand::Blackout(enabled),
                RemoteInputEvent::SetGroupSubmaster { group_id, level } => {
                    EngineCommand::SetGroupSubmaster { group_id, level }
                }
                RemoteInputEvent::SetBpm(bpm) => EngineCommand::SetBpm(bpm),
                RemoteInputEvent::TapBpm => EngineCommand::TapBpm,
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
) -> Result<CueId, String> {
    if label.trim().is_empty() {
        return Err("Cue label is required".to_string());
    }
    let snapshot = state.engine.snapshot();
    if snapshot.fixtures.is_empty()
        && snapshot.video.layers.is_empty()
        && snapshot.video.outputs.is_empty()
    {
        return Err(
            "Patch a fixture, add a video layer, or add a video output before creating a cue"
                .to_string(),
        );
    }
    let (targets, video_targets, video_output_targets) = cue_targets_from_snapshot(&snapshot);
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
) -> Result<(), String> {
    if label.trim().is_empty() {
        return Err("Cue label is required".to_string());
    }
    let snapshot = state.engine.snapshot();
    if !snapshot.cues.iter().any(|cue| cue.id == cue_id) {
        return Err(format!("Cue {cue_id} was not found"));
    }
    if snapshot.fixtures.is_empty()
        && snapshot.video.layers.is_empty()
        && snapshot.video.outputs.is_empty()
    {
        return Err(
            "Patch a fixture, add a video layer, or add a video output before updating a cue"
                .to_string(),
        );
    }
    let (targets, video_targets, video_output_targets) = cue_targets_from_snapshot(&snapshot);
    state
        .engine
        .send(EngineCommand::UpdateCue {
            cue_id,
            label,
            fade_ms,
            targets,
            video_targets,
            video_output_targets,
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
fn add_video_file_layer(
    state: State<'_, AppState>,
    label: String,
    path: String,
) -> Result<VideoLayerId, String> {
    let label = normalize_video_layer_label(label)?;
    if path.trim().is_empty() {
        return Err("Video file path is required".to_string());
    }
    let probe = video::probe_video_file_metadata(&path).ok();
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
                codec: probe.as_ref().and_then(|probe| probe.codec.clone()),
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
    if path.trim().is_empty() {
        return Err("Still image path is required".to_string());
    }
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
                metadata: None,
            },
        })
        .map_err(|error| error.to_string())?;
    Ok(layer_id)
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
    validate_video_layer_ids(&state.engine.snapshot(), &layer_ids)?;
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
    let config = normalize_video_output_config(
        label,
        kind,
        width,
        height,
        fullscreen,
        monitor_id,
        endpoint_name,
    )?;
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
    if !state
        .engine
        .snapshot()
        .video
        .compositions
        .iter()
        .any(|composition| composition.id == composition_id)
    {
        return Err(format!("Video composition {composition_id} was not found"));
    }
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
    validate_video_output_mapping(&mapping)?;
    state
        .engine
        .send(EngineCommand::SetVideoOutputMapping { output_id, mapping })
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
        app: "KDMX".to_string(),
        preset: VideoOutputMappingPresetSummary {
            label: label.clone(),
            mapping,
        },
    };
    let Some(path) = rfd::FileDialog::new()
        .add_filter("KDMX Projector Map", &["projmap"])
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
        .add_filter("KDMX Projector Map", &["projmap"])
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
fn move_effect(state: State<'_, AppState>, effect_id: EffectId, delta: i32) -> Result<(), String> {
    if delta != -1 && delta != 1 {
        return Err("Effect move delta must be -1 or 1".to_string());
    }
    state
        .engine
        .send(EngineCommand::MoveEffect { effect_id, delta })
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn remove_effect(state: State<'_, AppState>, effect_id: EffectId) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::RemoveEffect(effect_id))
        .map_err(|error| error.to_string())
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
        .add_filter("KDMX Effect", &["effect"])
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
        .add_filter("KDMX Effect", &["effect"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let preset: EffectPreset = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    validate_effect_preset(&preset)?;
    let snapshot = state.engine.snapshot();
    let effect_id = state.engine.allocate_effect_id();
    match preset.effect_type {
        EffectKind::Lfo => {
            let request = preset
                .lfo
                .ok_or_else(|| "LFO effect preset is missing its request body".to_string())?;
            validate_effect_target_references(
                &snapshot,
                &request.fixture_ids,
                &request.video_targets,
            )?;
            state
                .engine
                .send(EngineCommand::AddLfoEffect { effect_id, request })
                .map_err(|error| error.to_string())?;
        }
        EffectKind::PositionWave => {
            let request = preset.position_wave.ok_or_else(|| {
                "Position wave effect preset is missing its request body".to_string()
            })?;
            validate_effect_target_references(
                &snapshot,
                &request.fixture_ids,
                &request.video_targets,
            )?;
            state
                .engine
                .send(EngineCommand::AddPositionWaveEffect { effect_id, request })
                .map_err(|error| error.to_string())?;
        }
    }
    if !preset.enabled {
        state
            .engine
            .send(EngineCommand::SetEffectEnabled {
                effect_id,
                enabled: false,
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(Some(effect_id))
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
        mode_name: fixture.mode_name.clone(),
        values: fixture.attribute_values.clone(),
    };
    let Some(path) = rfd::FileDialog::new()
        .add_filter("KDMX Preset", &["preset"])
        .set_file_name(format!("{}.preset", safe_file_stem(&fixture.label)))
        .save_file()
    else {
        return Ok(None);
    };
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
        .add_filter("KDMX Preset", &["preset"])
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
fn save_project(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let project = ProjectFile {
        version: 1,
        app: "KDMX".to_string(),
        snapshot: state.engine.snapshot(),
    };
    let Some(path) = rfd::FileDialog::new()
        .add_filter("KDMX Project", &["kdmx"])
        .set_file_name("show.kdmx")
        .save_file()
    else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(&project).map_err(|error| error.to_string())?;
    fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
fn load_project(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("KDMX Project", &["kdmx"])
        .pick_file()
    else {
        return Ok(None);
    };
    let json = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let project: ProjectFile = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    validate_project_file(&project)?;
    state
        .engine
        .load_project_snapshot(project.snapshot)
        .map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

fn validate_project_file(project: &ProjectFile) -> Result<(), String> {
    if project.version != 1 {
        return Err(format!("Unsupported project version {}", project.version));
    }
    if project.app.trim() != "KDMX" {
        return Err(format!("Unsupported project app '{}'", project.app));
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
                    low: effect.low,
                    high: effect.high,
                    phase: effect.phase,
                    blend_mode: effect.blend_mode.clone(),
                }),
            })
        }
    }
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

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> EngineSnapshot {
    state.engine.snapshot()
}

#[tauri::command]
fn get_visualizer_scene(state: State<'_, AppState>) -> visualizer::VisualizerScene {
    visualizer::build_visualizer_scene(
        &state.engine.snapshot(),
        visualizer::VisualizerConfig::default(),
    )
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
fn get_debug_video_preview(
    state: State<'_, AppState>,
    width: u32,
    height: u32,
) -> Result<video::VideoFrame, String> {
    let snapshot = state.engine.snapshot();
    state
        .video_preview
        .lock()
        .map_err(|_| "Video preview renderer lock was poisoned".to_string())?
        .render(&snapshot.video, width, height)
        .map_err(|error| format!("{error:?}"))
}

#[tauri::command]
fn get_debug_video_output_preview(
    state: State<'_, AppState>,
    output_id: VideoOutputId,
    width: u32,
    height: u32,
) -> Result<video::VideoFrame, String> {
    let snapshot = state.engine.snapshot();
    state
        .video_preview
        .lock()
        .map_err(|_| "Video preview renderer lock was poisoned".to_string())?
        .render_output_preview(&snapshot.video, output_id, width, height)
        .map_err(|error| format!("{error:?}"))
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
            "KDMX {} - {}",
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

    builder.build().map_err(|error| error.to_string())?;
    Ok(())
}

fn load_patch_profile(
    state: &State<'_, AppState>,
    profile_path: &str,
) -> Result<FixtureProfileSummary, String> {
    if profile_path.starts_with("memory://") {
        return state
            .custom_profiles
            .lock()
            .map_err(|_| "Memory profile state lock was poisoned".to_string())?
            .get(profile_path)
            .cloned()
            .ok_or_else(|| format!("Memory fixture profile {profile_path} was not found"));
    }
    gdtf::load_profile(profile_path).map_err(|error| error.to_string())
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
        .map_err(|_| "Custom profile state lock was poisoned".to_string())?
        .insert(profile.source_path.clone(), profile.clone());
    Ok(profile)
}

fn fixture_profile_from_patched_fixture(fixture: &PatchedFixtureSummary) -> FixtureProfileSummary {
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
        geometries: Vec::new(),
        warnings: vec![format!(
            "Rebuilt from patched fixture {}; GDTF geometry and physical data are not available",
            fixture.label
        )],
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
            geometry: None,
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
        geometries: vec![GeometrySummary {
            name: "Body".to_string(),
            kind: "Geometry".to_string(),
            parent: None,
            matrix: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
        }],
        warnings: vec!["Custom profile; no GDTF geometry or physical data".to_string()],
    }
}

fn custom_attribute_default_value(attribute: &str) -> u16 {
    match attribute.to_ascii_lowercase().as_str() {
        "pan" | "tilt" | "panrotate" | "tiltrotate" => 32_768,
        _ => 0,
    }
}

fn validate_dmx_output_config(config: &DmxOutputConfig) -> Result<(), String> {
    let is_serial_dmx = matches!(
        config.protocol,
        DmxOutputProtocol::EnttecUsbPro | DmxOutputProtocol::EnttecOpenDmx
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
    if matches!(config.protocol, DmxOutputProtocol::Sacn) && config.universe == 0 {
        return Err("sACN universe must be 1 or greater".to_string());
    }
    if is_serial_dmx {
        if config.serial_port.trim().is_empty() {
            return Err("Serial port is required for serial DMX output".to_string());
        }
        if matches!(config.protocol, DmxOutputProtocol::EnttecUsbPro)
            && config.serial_baud_rate == 0
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

fn dmx_output_route_key(config: &DmxOutputConfig) -> String {
    match config.protocol {
        DmxOutputProtocol::ArtNet | DmxOutputProtocol::Sacn => format!(
            "{:?}|{}|{}|{}",
            config.protocol,
            config.target_ip.trim().to_ascii_lowercase(),
            config.port,
            config.universe
        ),
        DmxOutputProtocol::EnttecUsbPro | DmxOutputProtocol::EnttecOpenDmx => format!(
            "{:?}|{}|{}",
            config.protocol,
            config.serial_port.trim().to_ascii_lowercase(),
            config.universe
        ),
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
        DmxOutputProtocol::EnttecUsbPro | DmxOutputProtocol::EnttecOpenDmx => {
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
    if limits.dimmer_min > limits.dimmer_max {
        return Err("Dimmer minimum must be less than or equal to maximum".to_string());
    }
    if limits.pan_min > limits.pan_max {
        return Err("Pan minimum must be less than or equal to maximum".to_string());
    }
    if limits.tilt_min > limits.tilt_max {
        return Err("Tilt minimum must be less than or equal to maximum".to_string());
    }
    Ok(())
}

fn cue_targets_from_snapshot(
    snapshot: &EngineSnapshot,
) -> (
    Vec<CueFixtureTarget>,
    Vec<VideoLayerTarget>,
    Vec<VideoOutputTarget>,
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
    (targets, video_targets, video_output_targets)
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
    if !request.wavelength.is_finite() || request.wavelength.abs() < 0.001 {
        return Err("Wave wavelength must be at least 0.001".to_string());
    }
    if !request.phase.is_finite() {
        return Err("Wave phase must be finite".to_string());
    }
    validate_group_ids(&request.target_group_ids)?;
    validate_video_effect_targets(&request.video_targets)?;
    Ok(())
}

fn validate_video_effect_targets(targets: &[VideoEffectTarget]) -> Result<(), String> {
    for target in targets {
        if target.layer_ids.is_empty() {
            return Err("Video effect target requires at least one layer".to_string());
        }
        if !target.low.is_finite() || !target.high.is_finite() {
            return Err("Video effect low/high values must be finite".to_string());
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

fn validate_video_output_mapping(mapping: &VideoOutputMapping) -> Result<(), String> {
    let values = [
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
    if file.app.trim() != "KDMX" {
        return Err(format!("Unsupported projector map app '{}'", file.app));
    }
    normalize_video_output_mapping_preset_label(file.preset.label.clone())?;
    validate_video_output_mapping(&file.preset.mapping)
}

fn validate_group_ids(group_ids: &[String]) -> Result<(), String> {
    if group_ids.iter().any(|group_id| group_id.trim().is_empty()) {
        return Err("Group IDs must not be empty".to_string());
    }
    Ok(())
}

fn normalize_control_group_id(group_id: String) -> Result<String, String> {
    let trimmed = group_id.trim();
    if trimmed.is_empty() {
        return Err("Group ID is required".to_string());
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
        let trimmed = group_id.trim();
        if trimmed.is_empty() {
            return Err("Group IDs must not be empty".to_string());
        }
        if seen.insert(trimmed.to_string()) {
            normalized.push(trimmed.to_string());
        }
    }
    Ok(normalized)
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
    let stem = Path::new(raw_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("fixture");
    format!("{}.gdtf", safe_file_stem(stem))
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

    fn sample_preset(values: Vec<protocol::AttributeValueSummary>) -> FixturePreset {
        FixturePreset {
            version: 1,
            manufacturer: "KDMX".to_string(),
            profile_name: "Mini Spot".to_string(),
            mode_name: "Standard".to_string(),
            values,
        }
    }

    #[test]
    fn custom_fixture_profile_request_builds_8bit_controls() {
        let request = CustomFixtureProfileRequest {
            manufacturer: "KDMX".to_string(),
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

        assert_eq!(profile.source_path, "memory://custom/KDMX-Custom_Bar");
        assert_eq!(profile.dmx_modes[0].name, "8ch");
        assert_eq!(profile.dmx_modes[0].controls[0].offsets, vec![1]);
        assert_eq!(profile.dmx_modes[0].controls[1].offsets, vec![2]);
        assert_eq!(profile.dmx_modes[0].controls[1].default_value, 32_768);
        assert_eq!(gdtf::profile_mode_footprint(&profile, Some("8ch")), Some(3));
    }

    #[test]
    fn custom_fixture_profile_request_builds_mixed_resolution_controls() {
        let request = CustomFixtureProfileRequest {
            manufacturer: "KDMX".to_string(),
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
        assert_eq!(
            gdtf::profile_mode_footprint(&profile, Some("Standard")),
            Some(10)
        );
    }

    #[test]
    fn custom_fixture_profile_validation_rejects_duplicate_attributes() {
        let request = CustomFixtureProfileRequest {
            manufacturer: "KDMX".to_string(),
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
            manufacturer: "KDMX".to_string(),
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
            manufacturer: "KDMX".to_string(),
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
                manufacturer: "KDMX".to_string(),
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
    fn custom_fixture_profile_file_rejects_wrong_version() {
        let profile_file = CustomFixtureProfileFile {
            version: 2,
            request: CustomFixtureProfileRequest {
                manufacturer: "KDMX".to_string(),
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
            "Robe_20Spot.gdtf"
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
    fn dmx_output_config_validation_rejects_blank_network_target() {
        let error = validate_dmx_output_config(&DmxOutputConfig {
            target_ip: " ".to_string(),
            ..DmxOutputConfig::default()
        })
        .unwrap_err();

        assert!(error.contains("target IP"));
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
    fn normalize_group_ids_trims_and_deduplicates() {
        assert_eq!(
            normalize_group_ids(vec![
                " front ".to_string(),
                "movers".to_string(),
                "front".to_string(),
            ])
            .unwrap(),
            vec!["front".to_string(), "movers".to_string()]
        );
        assert!(normalize_group_ids(vec![" ".to_string()])
            .unwrap_err()
            .contains("must not be empty"));
    }

    #[test]
    fn normalize_control_group_id_trims_and_rejects_empty() {
        assert_eq!(
            normalize_control_group_id(" front ".to_string()).unwrap(),
            "front"
        );
        assert!(normalize_control_group_id(" ".to_string())
            .unwrap_err()
            .contains("Group ID"));
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
            app: "KDMX".to_string(),
            preset: VideoOutputMappingPresetSummary {
                label: "Front Projector".to_string(),
                mapping: VideoOutputMapping {
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
        assert!((parsed.preset.mapping.aspect_ratio - 16.0 / 9.0).abs() < f32::EPSILON);

        let unsupported = VideoOutputMappingPresetFile {
            version: 2,
            ..file.clone()
        };
        assert!(validate_video_output_mapping_preset_file(&unsupported)
            .unwrap_err()
            .contains("version"));
    }

    #[test]
    fn project_file_serializes_snapshot_as_pretty_json() {
        let project = ProjectFile {
            version: 1,
            app: "KDMX".to_string(),
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
        assert_eq!(parsed.app, "KDMX");
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
    fn project_file_validation_rejects_wrong_version_or_app() {
        let mut project = ProjectFile {
            version: 2,
            app: "KDMX".to_string(),
            snapshot: EngineSnapshot::default(),
        };

        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("Unsupported project version"));

        project.version = 1;
        project.app = "Other".to_string();
        assert!(validate_project_file(&project)
            .unwrap_err()
            .contains("Unsupported project app"));
    }

    #[test]
    fn cue_targets_from_snapshot_includes_video_outputs() {
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

        let (fixture_targets, video_targets, video_output_targets) =
            cue_targets_from_snapshot(&snapshot);

        assert!(fixture_targets.is_empty());
        assert!(video_targets.is_empty());
        assert_eq!(video_output_targets.len(), 1);
        assert_eq!(video_output_targets[0].output_id, 7);
        assert!(!video_output_targets[0].enabled);
        assert!((video_output_targets[0].opacity - 0.35).abs() < f32::EPSILON);
        assert!(video_output_targets[0].blackout);
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

    fn sample_patch_profile() -> FixtureProfileSummary {
        custom_fixture_profile_from_request(CustomFixtureProfileRequest {
            manufacturer: "KDMX".to_string(),
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
            profile_path: "memory://custom/KDMX-Overlap_Test".to_string(),
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
            profile_name: profile.name.clone(),
            manufacturer: profile.manufacturer.clone(),
            mode_name: "Default".to_string(),
            universe,
            address,
            group_ids: Vec::new(),
            position: Vec3::default(),
            rotation: Rotation3::default(),
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
        let fixture = sample_patched_fixture(&profile, 1, 10);

        let rebuilt = fixture_profile_from_patched_fixture(&fixture);

        assert_eq!(rebuilt.source_path, "memory://patched-fixture/7");
        assert_eq!(rebuilt.manufacturer, "KDMX");
        assert_eq!(rebuilt.name, "Overlap Test");
        assert_eq!(rebuilt.dmx_modes[0].name, "Default");
        assert_eq!(rebuilt.dmx_modes[0].controls, fixture.controls);
        assert!(rebuilt.warnings[0].contains("Rebuilt from patched fixture"));
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
    fn preset_validation_accepts_matching_fixture_identity_and_attributes() {
        let preset = sample_preset(vec![protocol::AttributeValueSummary {
            attribute: "Dimmer".to_string(),
            value: 32_768,
        }]);

        assert!(validate_fixture_preset(
            &preset,
            "KDMX",
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
            validate_fixture_preset(&preset, "KDMX", "Other Spot", "Standard", ["Dimmer"])
                .unwrap_err();
        assert!(profile_error.contains("Preset is for"));

        let preset = sample_preset(vec![protocol::AttributeValueSummary {
            attribute: "Zoom".to_string(),
            value: 12_000,
        }]);
        let attribute_error =
            validate_fixture_preset(&preset, "KDMX", "Mini Spot", "Standard", ["Dimmer"])
                .unwrap_err();
        assert!(attribute_error.contains("not available"));
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
            low: 0,
            high: 40_000,
            phase: 0.0,
            blend_mode: protocol::EffectBlendMode::Add,
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
            video_targets: Vec::new(),
            shape: protocol::LfoShape::Sine,
            period_ms: Some(500),
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
        assert_eq!(preset.lfo.unwrap().period_ms, 500);
        assert!(preset.position_wave.is_none());
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
            custom_profiles: Mutex::new(HashMap::new()),
            midi_clock: Mutex::new(None),
            midi_control: Mutex::new(None),
            midi_feedback: Mutex::new(None),
            osc_input: Mutex::new(None),
            remote_control: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            select_gdtf_file,
            select_video_source_file,
            import_gdtf,
            download_gdtf_from_url,
            create_custom_fixture_profile,
            save_custom_fixture_profile,
            load_custom_fixture_profile,
            use_fixture_profile,
            patch_fixture,
            patch_fixtures,
            remove_fixture,
            set_fixture_patch,
            set_fixture_limits,
            set_fixture_groups,
            set_attribute,
            set_group_attribute,
            set_group_highlight,
            set_group_solo,
            set_group_park,
            set_fixture_transform,
            set_fixture_highlight,
            set_fixture_solo,
            set_fixture_park,
            set_output_config,
            set_dmx_outputs,
            set_blackout,
            set_lighting_master,
            set_group_submaster,
            set_bpm,
            tap_bpm,
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
            set_timeline_automation,
            add_timeline_video_automation,
            set_timeline_video_automation,
            remove_timeline_automation,
            set_timeline_playing,
            seek_timeline,
            add_video_file_layer,
            add_still_image_layer,
            add_video_input_layer,
            duplicate_video_layer,
            remove_video_layer,
            set_video_layer_order,
            set_video_layer_label,
            set_video_layer_state,
            add_video_cue_point,
            remove_video_cue_point,
            jump_video_cue_point,
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
            save_video_output_mapping_preset,
            remove_video_output_mapping_preset,
            save_video_output_mapping_preset_file,
            load_video_output_mapping_preset_file,
            add_lfo_effect,
            add_position_wave_effect,
            set_effect_enabled,
            move_effect,
            remove_effect,
            save_effect_preset,
            load_effect_preset,
            save_fixture_preset,
            load_fixture_preset,
            save_project,
            load_project,
            get_snapshot,
            get_visualizer_scene,
            get_video_composition_plans,
            get_video_output_render_plans,
            get_debug_video_preview,
            get_debug_video_output_preview,
            get_debug_video_output_test_pattern,
            open_video_output_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running KDMX");
}

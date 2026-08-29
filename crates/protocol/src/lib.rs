use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

pub mod control_plane;
pub mod control_plane_command;
pub mod control_plane_query;
pub mod control_plane_registry_v2;
pub mod fixture_stage_layout;

pub use fixture_stage_layout::validate_fixture_stage_layout;
pub use fixture_stage_layout::{
    FixtureStageColorBinding, FixtureStageColorRole, FixtureStageLayout, FixtureStageLayoutCell,
    FixtureStageLogicalSegment, FIXTURE_STAGE_LAYOUT_MAX_ENTRIES, FIXTURE_STAGE_LAYOUT_WORLD_CAP,
};

pub type FixtureId = u64;
pub type EffectId = u64;
pub type CueId = u64;
pub type CueListId = u64;
pub type PaletteId = u64;
pub type ExecutorId = u64;
pub type TimelineEventId = u64;
pub type TimelineAudioClipId = u64;
pub type AutomationId = u64;
macro_rules! timeline_id_newtype {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            Default,
            Serialize,
            Deserialize,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
        )]
        #[serde(transparent)]
        pub struct $name(pub u64);
    };
}

timeline_id_newtype!(TimelineId);
timeline_id_newtype!(TimelinePhaseId);
timeline_id_newtype!(TimelineVideoClipId);
timeline_id_newtype!(TimelineItemGroupId);
pub type VideoLayerId = u64;
/// Runtime-only identity for one renderer input. Unlike `VideoLayerId`, this
/// identifies a concrete source instance in a render graph and is never part
/// of the authored project model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VideoRenderInputId(pub u64);

/// Runtime ownership key for decoded frames and decoder state.
///
/// `layer_id` remains useful diagnostics, but is deliberately excluded here:
/// one authored layer can feed more than one live render input at once. The
/// project render epoch fences queues/caches/sessions created for a retired
/// project image from a replacement image.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VideoRenderInputKey {
    pub project_render_epoch: u64,
    pub input_id: VideoRenderInputId,
}

impl VideoRenderInputKey {
    /// Compatibility key used by APIs that predate explicit renderer inputs.
    pub const LEGACY: Self = Self {
        project_render_epoch: 0,
        input_id: VideoRenderInputId(0),
    };
}

/// Stable project-local identity for an authored Video clip slot. This remains
/// a newtype (rather than a UI index) so reordering a bank cannot retarget a
/// persisted reference.
#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(transparent)]
pub struct VideoClipSlotId(pub u64);
macro_rules! video_effect_id_newtype {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            Default,
            Serialize,
            Deserialize,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
        )]
        #[serde(transparent)]
        pub struct $name(pub u64);
    };
}

video_effect_id_newtype!(VideoEffectId);
video_effect_id_newtype!(VideoEffectChainId);
video_effect_id_newtype!(VideoEffectStageId);
video_effect_id_newtype!(VideoEffectPresetId);
video_effect_id_newtype!(VideoLayerGroupId);
video_effect_id_newtype!(VideoTransitionBusId);

pub const VIDEO_EFFECT_CHAIN_MAX_STAGES: usize = 32;
pub const VIDEO_EFFECT_PROJECT_MAX_CHAINS: usize = 256;
pub const VIDEO_EFFECT_PROJECT_MAX_PRESETS: usize = 128;
pub const VIDEO_EFFECT_PROJECT_MAX_GROUPS: usize = 64;
pub const VIDEO_TRANSITION_BUS_PROJECT_MAX_BUSES: usize = 64;
pub const VIDEO_TRANSITION_BUS_MAX_MEMBERS: usize = 32;
/// Stable project-local identity for a reusable media library entry.
pub type MediaAssetId = u64;
pub type CompositionId = u64;
pub type VideoOutputId = u64;
pub type NodeGraphId = u64;
pub type StageObjectId = u64;
pub type TouchPageId = u64;
pub type TouchControlId = u64;

pub const MAX_TIMELINE_SCENE_BLOCK_LOOPS: u16 = 256;
pub const MIN_CUE_AUTHORED_BEATS: f32 = 0.25;
pub const MAX_CUE_AUTHORED_BEATS: f32 = 1024.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for Vec3 {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Rotation3 {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

impl Default for Rotation3 {
    fn default() -> Self {
        Self {
            pitch: 0.0,
            yaw: 0.0,
            roll: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttributeResolution {
    EightBit,
    SixteenBit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CieColorSummary {
    pub x: f32,
    pub y: f32,
    pub luminance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmitterCalibrationSummary {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<CieColorSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dominant_wavelength_nm: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChannelFunctionSummary {
    pub name: String,
    pub attribute: String,
    #[serde(default)]
    pub parent_function: Option<String>,
    pub dmx_from: u16,
    pub dmx_to: u16,
    #[serde(default)]
    pub physical_from: Option<f32>,
    #[serde(default)]
    pub physical_to: Option<f32>,
    #[serde(default)]
    pub wheel_slot: Option<String>,
    #[serde(default)]
    pub wheel_slot_name: Option<String>,
    #[serde(default)]
    pub wheel_slot_color: Option<String>,
    #[serde(default)]
    pub wheel_slot_media: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emitter: Option<EmitterCalibrationSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttributeControl {
    pub attribute: String,
    pub channel_name: String,
    pub geometry: Option<String>,
    pub offsets: Vec<u16>,
    pub resolution: AttributeResolution,
    pub default_value: u16,
    #[serde(default)]
    pub functions: Vec<ChannelFunctionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeometrySummary {
    pub name: String,
    pub kind: String,
    pub parent: Option<String>,
    pub matrix: [f32; 16],
    #[serde(default)]
    pub model_name: Option<String>,
    #[serde(default)]
    pub model_file: Option<String>,
    #[serde(default)]
    pub model_primitive: Option<String>,
    #[serde(default)]
    pub model_dimensions: Option<Vec3>,
    #[serde(default)]
    pub beam_type: Option<String>,
    #[serde(default)]
    pub beam_angle_deg: Option<f32>,
    #[serde(default)]
    pub field_angle_deg: Option<f32>,
    #[serde(default)]
    pub beam_radius: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DmxModeSummary {
    pub name: String,
    pub controls: Vec<AttributeControl>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixtureProfileSummary {
    pub source_path: String,
    pub manufacturer: String,
    pub name: String,
    pub short_name: Option<String>,
    pub fixture_type_id: Option<String>,
    pub dmx_modes: Vec<DmxModeSummary>,
    pub geometries: Vec<GeometrySummary>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatchFixtureRequest {
    pub profile_path: String,
    pub mode_name: Option<String>,
    pub label: String,
    pub universe: u16,
    pub address: u16,
    pub group_ids: Vec<String>,
    pub position: Vec3,
    pub rotation: Rotation3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FixtureLimits {
    pub dimmer_min: u16,
    pub dimmer_max: u16,
    pub pan_min: u16,
    pub pan_max: u16,
    pub tilt_min: u16,
    pub tilt_max: u16,
    pub invert_pan: bool,
    pub invert_tilt: bool,
    pub swap_pan_tilt: bool,
}

impl Default for FixtureLimits {
    fn default() -> Self {
        Self {
            dimmer_min: 0,
            dimmer_max: u16::MAX,
            pan_min: 0,
            pan_max: u16::MAX,
            tilt_min: 0,
            tilt_max: u16::MAX,
            invert_pan: false,
            invert_tilt: false,
            swap_pan_tilt: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomFixtureProfileRequest {
    pub manufacturer: String,
    pub name: String,
    pub mode_name: String,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomFixtureProfileFile {
    pub version: u32,
    pub request: CustomFixtureProfileRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioWaveformPoint {
    pub time_ms: u64,
    pub peak: f32,
    pub rms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioSpectrumPoint {
    pub time_ms: u64,
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
}

pub const LIVE_AUDIO_FEATURE_BAND_CAPACITY: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveAudioReactiveFeatures {
    pub band_count: u8,
    pub bands: [f32; LIVE_AUDIO_FEATURE_BAND_CAPACITY],
    pub rms: f32,
    pub peak: f32,
    pub spectral_flux: f32,
    pub spectral_centroid: f32,
    pub spectral_density_fast: f32,
    pub spectral_density_slow: f32,
    pub kick_strength: f32,
    pub snare_strength: f32,
    pub kick_event: bool,
    pub snare_event: bool,
    pub onset: bool,
    pub onset_strength: f32,
    pub bpm: Option<f32>,
    pub bpm_confidence: f32,
    pub beat_phase: f32,
}

impl Default for LiveAudioReactiveFeatures {
    fn default() -> Self {
        Self {
            band_count: 0,
            bands: [0.0; LIVE_AUDIO_FEATURE_BAND_CAPACITY],
            rms: 0.0,
            peak: 0.0,
            spectral_flux: 0.0,
            spectral_centroid: 0.0,
            spectral_density_fast: 0.0,
            spectral_density_slow: 0.0,
            kick_strength: 0.0,
            snare_strength: 0.0,
            kick_event: false,
            snare_event: false,
            onset: false,
            onset_strength: 0.0,
            bpm: None,
            bpm_confidence: 0.0,
            beat_phase: 0.0,
        }
    }
}

pub const MAX_LIVE_AUDIO_FRAME_ONSETS: usize = 4;

/// One atomic analysis publication from a live input generation.
///
/// Spectrum and onset sequences intentionally share one payload so the engine can
/// never accept a reactive frame without also accepting every onset derived from it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveAudioFrame {
    pub generation: u64,
    pub feature_sequence: u64,
    pub spectrum: AudioSpectrumPoint,
    #[serde(default)]
    pub features: LiveAudioReactiveFeatures,
    pub onset_feature_sequences: [u64; MAX_LIVE_AUDIO_FRAME_ONSETS],
    pub onset_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioAnalysisSummary {
    pub path: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_ms: u64,
    pub estimated_bpm: Option<f32>,
    pub waveform: Vec<AudioWaveformPoint>,
    #[serde(default)]
    pub spectrum: Vec<AudioSpectrumPoint>,
    pub beats: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttributeValueSummary {
    pub attribute: String,
    pub value: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatchedFixtureSummary {
    pub id: FixtureId,
    pub label: String,
    #[serde(default)]
    pub profile_source_path: String,
    pub profile_name: String,
    pub manufacturer: String,
    pub mode_name: String,
    pub universe: u16,
    pub address: u16,
    pub group_ids: Vec<String>,
    pub position: Vec3,
    pub rotation: Rotation3,
    #[serde(default)]
    pub geometries: Vec<GeometrySummary>,
    pub controls: Vec<AttributeControl>,
    /// Optional v1 physical cell topology. `None` is the explicit legacy
    /// marker for snapshots authored before stage-layout support.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_layout: Option<FixtureStageLayout>,
    pub attribute_values: Vec<AttributeValueSummary>,
    #[serde(default)]
    pub limits: FixtureLimits,
    pub highlighted: bool,
    pub soloed: bool,
    pub parked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixturePreset {
    pub version: u32,
    pub manufacturer: String,
    pub profile_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_source_path: Option<String>,
    pub mode_name: String,
    pub values: Vec<AttributeValueSummary>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperatorLockMode {
    Full,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorCredentialVerifier {
    pub scheme: String,
    pub iterations: u32,
    pub salt_b64: String,
    pub verifier_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorPolicy {
    pub lock_mode: OperatorLockMode,
    #[serde(default)]
    pub lock_on_load: bool,
    pub credential: OperatorCredentialVerifier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixtureGroupSummary {
    pub id: String,
    pub label: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectFile {
    pub version: u32,
    pub app: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_policy: Option<OperatorPolicy>,
    #[serde(default)]
    pub custom_profiles: Vec<FixtureProfileSummary>,
    /// First-class fixture-selection groups. Membership remains on fixtures as
    /// immutable group IDs, so changing a label never rewrites membership.
    /// Legacy `.sdc` v1 files omitted this field, so absence loads as empty.
    #[serde(default)]
    pub fixture_groups: Vec<FixtureGroupSummary>,
    pub snapshot: EngineSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CueFixtureTarget {
    pub fixture_id: FixtureId,
    pub values: Vec<AttributeValueSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CueStepSummary {
    pub values: Vec<CueFixtureTarget>,
    pub fade_ms: u64,
    pub hold_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CueEffectTarget {
    pub effect_id: EffectId,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<EffectParamsSnapshot>,
    /// Optional cross-Cue transition for this Cue-owned Effect. Absence keeps
    /// the legacy instant-recall behavior and serialized shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoSourceKind {
    File,
    Camera,
    ScreenCapture,
    Ndi,
    Spout,
    Syphon,
    StillImage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoBackendState {
    Available,
    Missing,
    NotBuilt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoBackendStatus {
    pub id: String,
    pub label: String,
    pub state: VideoBackendState,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoRuntimeStatus {
    pub backends: Vec<VideoBackendStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoSourceSummary {
    pub kind: VideoSourceKind,
    pub path: Option<String>,
    pub name: Option<String>,
    pub codec: Option<String>,
    #[serde(default)]
    pub metadata: Option<VideoMediaMetadata>,
}

/// The content-address algorithm used by a local media library entry.
///
/// This is deliberately explicit rather than treating the hash string as an
/// opaque implementation detail: persisted projects remain self-describing if
/// another identity algorithm is added in a later format revision.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaHashAlgorithm {
    Sha256,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaContentHash {
    pub algorithm: MediaHashAlgorithm,
    /// A lowercase hexadecimal digest. `validate_media_asset_catalog` checks
    /// the exact SHA-256 shape before an engine-ready project is installed.
    pub hex: String,
}

/// A reusable source entry. Availability intentionally is not a field here:
/// file presence and verification are machine-local IPC facts, never `.sdc`
/// project data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaAssetSummary {
    pub id: MediaAssetId,
    pub label: String,
    pub source: VideoSourceSummary,
    /// `None` paired with `byte_size: None` is the allowed legacy identity
    /// state. New local imports persist both values together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<MediaContentHash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<u64>,
}

/// Machine-local availability truth returned by media-library IPC. This type
/// is purposefully separate from `MediaAssetSummary` so serializing a project
/// cannot leak a machine's filesystem state into `.sdc`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MediaAssetAvailability {
    AvailableVerified {
        asset_id: MediaAssetId,
    },
    AvailableUnverified {
        asset_id: MediaAssetId,
    },
    Missing {
        asset_id: MediaAssetId,
    },
    HashMismatch {
        asset_id: MediaAssetId,
        expected: MediaContentHash,
        actual: MediaContentHash,
    },
    Unreadable {
        asset_id: MediaAssetId,
        error: String,
    },
    LiveSource {
        asset_id: MediaAssetId,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaAssetRelinkPolicy {
    RequireContentMatch,
    AdoptReplacement,
}

/// A typed relink result prevents a caller from accidentally treating a hash
/// mismatch or an unadopted legacy replacement as a successful path change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MediaAssetRelinkOutcome {
    Relinked {
        asset_id: MediaAssetId,
        adopted_replacement: bool,
    },
    NeedsExplicitAdoption {
        asset_id: MediaAssetId,
    },
    HashMismatch {
        asset_id: MediaAssetId,
        expected: MediaContentHash,
        actual: MediaContentHash,
    },
    MissingReplacement {
        asset_id: MediaAssetId,
    },
    UnreadableReplacement {
        asset_id: MediaAssetId,
        error: String,
    },
    LiveSource {
        asset_id: MediaAssetId,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VideoMediaMetadata {
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub frame_rate: Option<f32>,
    #[serde(default)]
    pub has_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoBlendMode {
    Normal,
    Add,
    Multiply,
    Screen,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VideoBpmSync {
    pub enabled: bool,
    pub ratio: f32,
    pub loop_bars: f32,
}

impl Default for VideoBpmSync {
    fn default() -> Self {
        Self {
            enabled: false,
            ratio: 1.0,
            loop_bars: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Transform2D {
    pub x: f32,
    pub y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation_deg: f32,
    pub crop_left: f32,
    pub crop_top: f32,
    pub crop_right: f32,
    pub crop_bottom: f32,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation_deg: 0.0,
            crop_left: 0.0,
            crop_top: 0.0,
            crop_right: 0.0,
            crop_bottom: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VideoColorAdjust {
    pub brightness: f32,
    pub contrast: f32,
    pub hue_deg: f32,
    pub saturation: f32,
    pub gamma: f32,
}

impl Default for VideoColorAdjust {
    fn default() -> Self {
        Self {
            brightness: 0.0,
            contrast: 1.0,
            hue_deg: 0.0,
            saturation: 1.0,
            gamma: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VideoFxAdjust {
    #[serde(default = "default_fx_pixelate")]
    pub pixelate: f32,
    #[serde(default)]
    pub blur: f32,
    #[serde(default)]
    pub glow: f32,
    #[serde(default)]
    pub edge: f32,
    #[serde(default)]
    pub key_red: f32,
    #[serde(default = "default_fx_key_green")]
    pub key_green: f32,
    #[serde(default)]
    pub key_blue: f32,
    #[serde(default)]
    pub key_threshold: f32,
}

impl Default for VideoFxAdjust {
    fn default() -> Self {
        Self {
            pixelate: default_fx_pixelate(),
            blur: 0.0,
            glow: 0.0,
            edge: 0.0,
            key_red: 0.0,
            key_green: default_fx_key_green(),
            key_blue: 0.0,
            key_threshold: 0.0,
        }
    }
}

fn default_fx_pixelate() -> f32 {
    1.0
}

fn default_fx_key_green() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoCuePointSummary {
    pub position_ms: u64,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoLayerState {
    #[serde(default = "default_video_layer_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub solo: bool,
    pub opacity: f32,
    pub speed: f32,
    pub playing: bool,
    pub position_ms: u64,
    pub loop_enabled: bool,
    pub loop_start_ms: u64,
    pub loop_end_ms: u64,
    pub bpm_sync: VideoBpmSync,
    #[serde(default)]
    pub cue_points: Vec<VideoCuePointSummary>,
    #[serde(default)]
    pub cue_points_ms: Vec<u64>,
    pub transform: Transform2D,
    #[serde(default)]
    pub color: VideoColorAdjust,
    #[serde(default)]
    pub fx: VideoFxAdjust,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoIsfControlKind {
    Event,
    Bool,
    Long,
    Float,
    Point2d,
    Color,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoIsfControlSummary {
    pub name: String,
    pub kind: VideoIsfControlKind,
    pub value: [f32; 4],
    pub default: [f32; 4],
    pub minimum: [f32; 4],
    pub maximum: [f32; 4],
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub values: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoIsfEffectStageSummary {
    pub enabled: bool,
    pub label: String,
    pub source: Arc<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub controls: Vec<VideoIsfControlSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoIsfEffectSummary {
    pub enabled: bool,
    pub label: String,
    pub source: Arc<str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub controls: Vec<VideoIsfControlSummary>,
    /// Additional single-pass effects evaluated after this effect. Keeping the
    /// first stage in the legacy object preserves `.sdc v1` compatibility while
    /// allowing newer projects to serialize an ordered GPU effect stack.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stack: Vec<VideoIsfEffectStageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VideoTransitionEffectOwner {
    ClipTake {
        layer_id: VideoLayerId,
    },
    LayerBus {
        bus_id: VideoTransitionBusId,
    },
    /// Canonical Custom transition chain for the Follow authored by one stable
    /// source Timeline. Scope uniqueness therefore permits at most one Follow
    /// Transition chain per source Timeline regardless of bank order.
    TimelineFollow {
        source_timeline_id: TimelineId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum VideoEffectScope {
    Clip {
        layer_id: VideoLayerId,
        slot_id: VideoClipSlotId,
    },
    Layer {
        layer_id: VideoLayerId,
    },
    Transition {
        owner: VideoTransitionEffectOwner,
    },
    Composition {
        composition_id: CompositionId,
    },
    Group {
        group_id: VideoLayerGroupId,
    },
    Output {
        output_id: VideoOutputId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VideoEffectKind {
    Isf { effect: VideoIsfEffectSummary },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoEffectSummary {
    pub id: VideoEffectId,
    pub kind: VideoEffectKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoEffectStageSummary {
    pub id: VideoEffectStageId,
    pub enabled: bool,
    pub label: String,
    pub effect: VideoEffectSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoEffectChainSummary {
    pub id: VideoEffectChainId,
    pub scope: VideoEffectScope,
    #[serde(default)]
    pub bypassed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<VideoEffectStageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoEffectPresetStagePayload {
    pub enabled: bool,
    pub label: String,
    pub effect: VideoEffectKind,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct VideoEffectPresetPayload {
    #[serde(default)]
    pub bypassed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<VideoEffectPresetStagePayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoEffectPresetSummary {
    pub id: VideoEffectPresetId,
    pub label: String,
    pub payload: VideoEffectPresetPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoLayerGroupSummary {
    pub id: VideoLayerGroupId,
    pub label: String,
    pub composition_id: CompositionId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_ids: Vec<VideoLayerId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VideoLayerTransitionTarget {
    Layer { layer_id: VideoLayerId },
    Group { group_id: VideoLayerGroupId },
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoLayerTransitionCurve {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

/// One authored transition bus. Membership is opt-in; a running bus may only
/// affect layers expanded from these targets in this composition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoLayerTransitionBusSummary {
    pub id: VideoTransitionBusId,
    pub label: String,
    pub composition_id: CompositionId,
    #[serde(default = "default_video_layer_transition_bus_enabled")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<VideoLayerTransitionTarget>,
    pub default_from: VideoLayerTransitionTarget,
    pub default_to: VideoLayerTransitionTarget,
    #[serde(default)]
    pub default_kind: VideoClipTakeKind,
    #[serde(default)]
    pub default_duration: VideoClipTakeDuration,
    #[serde(default)]
    pub default_curve: VideoLayerTransitionCurve,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matte_source: Option<VideoLayerTransitionTarget>,
}

fn default_video_layer_transition_bus_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoEffectControlTarget {
    pub effect_id: VideoEffectId,
    pub control_name: String,
}

impl Default for VideoLayerState {
    fn default() -> Self {
        Self {
            enabled: default_video_layer_enabled(),
            solo: false,
            opacity: 1.0,
            speed: 1.0,
            playing: false,
            position_ms: 0,
            loop_enabled: false,
            loop_start_ms: 0,
            loop_end_ms: 0,
            bpm_sync: VideoBpmSync::default(),
            cue_points: Vec::new(),
            cue_points_ms: Vec::new(),
            transform: Transform2D::default(),
            color: VideoColorAdjust::default(),
            fx: VideoFxAdjust::default(),
        }
    }
}

fn default_video_layer_enabled() -> bool {
    true
}

/// How a clip playhead behaves when it reaches its authored half-open range.
/// `PingPong` is authored here; decoder/playhead semantics are owned by the
/// later runtime tranche.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoClipLoopMode {
    #[default]
    Once,
    Loop,
    PingPong,
}

/// The musical boundary at which a queued clip may launch. Runtime owns the
/// 4-beats-per-bar clock calculation and holds the queued action across a
/// clock discontinuity rather than guessing a boundary.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoClipLaunchQuantization {
    #[default]
    Immediate,
    NextBeat,
    NextBar,
}

/// A named cue point in a clip's source-relative millisecond domain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoClipCuePointSummary {
    pub position_ms: u64,
    pub name: String,
}

/// A clip-local value for a control that already exists in the layer's ISF
/// chain. `stage_index == 0` selects the legacy root effect; `1..` selects the
/// corresponding zero-based entry in the root effect's serialized `stack`.
/// B1 deliberately does not introduce an independent clip FX chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoClipEffectOverrideSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_id: Option<VideoEffectId>,
    #[serde(default)]
    pub stage_index: usize,
    pub control_name: String,
    pub value: [f32; 4],
}

/// One ordered, authored slot in a layer's clip bank. `in_point_ms` is
/// inclusive and `out_point_ms`, when present, is exclusive. `None` means the
/// source's natural end; it avoids inventing duration metadata at migration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoClipSlotSummary {
    pub id: VideoClipSlotId,
    pub media_asset_id: MediaAssetId,
    #[serde(default)]
    pub in_point_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_point_ms: Option<u64>,
    #[serde(default)]
    pub loop_mode: VideoClipLoopMode,
    #[serde(default = "default_video_clip_speed")]
    pub speed: f32,
    #[serde(default)]
    pub cue_points: Vec<VideoClipCuePointSummary>,
    #[serde(default)]
    pub launch_quantization: VideoClipLaunchQuantization,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effect_overrides: Vec<VideoClipEffectOverrideSummary>,
}

fn default_video_clip_speed() -> f32 {
    1.0
}

/// A launch the runtime has accepted but must not execute until its captured
/// clock boundary. `target_boundary_ordinal` is the global beat-boundary
/// ordinal in `clock_generation`; a `NextBar` target is represented by the
/// corresponding 4-beat bar boundary's beat ordinal (and is therefore a
/// multiple of four). Both the ordinal and the zero-based clock generation may
/// validly be zero. A discontinuity holds this exact identity instead of
/// retargeting a different musical beat.
///
/// This is runtime truth only. It is intentionally separate from the authored
/// video snapshot and is never a project-persistence field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoClipPendingLaunchSummary {
    pub slot_id: VideoClipSlotId,
    pub quantization: VideoClipLaunchQuantization,
    pub target_boundary_ordinal: u64,
    pub clock_generation: u64,
    #[serde(default)]
    pub held_for_clock_discontinuity: bool,
    #[serde(default)]
    pub transition_kind: VideoClipTakeKind,
    #[serde(default)]
    pub duration: VideoClipTakeDuration,
    #[serde(default)]
    pub resolved_duration_ms: u64,
}

/// Runtime-only Clip Take style. `Cut` preserves the pre-C2 launch behavior;
/// every other variant keeps outgoing and incoming decoder identities alive
/// together until the resolved duration elapses. `Custom` uses the canonical
/// ClipTake transition effect chain on top of a neutral crossfade base.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoClipTakeKind {
    #[default]
    Cut,
    Crossfade,
    Dip,
    Wipe,
    Luma,
    Displacement,
    Blur,
    Glitch,
    Custom,
}

/// Unit for a Clip Take duration. Musical values use fixed-point thousandths
/// (`1000 == 1 beat/bar`) so request identity and terminal receipts never
/// depend on non-finite or platform-rounded JSON floats.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoClipTakeDurationUnit {
    #[default]
    Milliseconds,
    Beats,
    Bars,
}

/// Authored operator intent for one runtime-only Clip Take. The engine resolves
/// this against the ShowClock exactly once when Take is admitted and freezes the
/// resulting millisecond duration for pending launch, playback, and Reverse.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoClipTakeDuration {
    #[serde(default)]
    pub unit: VideoClipTakeDurationUnit,
    #[serde(default)]
    pub value_milliunits: u64,
}

impl VideoClipTakeDuration {
    pub const fn milliseconds(value: u64) -> Self {
        Self {
            unit: VideoClipTakeDurationUnit::Milliseconds,
            value_milliunits: value,
        }
    }
}

/// Ephemeral truth for one in-flight Clip Take. The authored layer projection
/// remains the outgoing slot until completion, so persistence can never capture
/// a half-transitioned source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoClipTakeTransitionSummary {
    /// Stable source-role anchor retained across Reverse so directional wipes
    /// can complement progress without a one-frame discontinuity.
    pub origin_slot_id: VideoClipSlotId,
    pub outgoing_slot_id: VideoClipSlotId,
    pub incoming_slot_id: VideoClipSlotId,
    pub kind: VideoClipTakeKind,
    pub elapsed_ms: u64,
    pub duration_ms: u64,
    #[serde(default)]
    pub duration: VideoClipTakeDuration,
    /// Integer 0..=1000 progress avoids non-finite float state at the IPC edge.
    pub progress_millis: u16,
    #[serde(default)]
    pub incoming_playhead_ms: u64,
    #[serde(default)]
    pub incoming_playing: bool,
}

/// Decoder and transport truth for one authored video layer. `playhead_ms` is
/// a source-relative, integer-millisecond position: `u64` deliberately avoids
/// a non-finite floating-point state at the protocol boundary. This type is
/// not embedded in `VideoLayerSummary`, `VideoSnapshot`, or `EngineSnapshot`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoClipLayerRuntimeSummary {
    #[serde(default)]
    pub layer_id: VideoLayerId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_slot_id: Option<VideoClipSlotId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queued_slot_id: Option<VideoClipSlotId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_launch: Option<VideoClipPendingLaunchSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<VideoClipTakeTransitionSummary>,
    #[serde(default)]
    pub playhead_ms: u64,
    #[serde(default)]
    pub playing: bool,
    #[serde(default)]
    pub ping_pong_reverse: bool,
}

/// A standalone runtime publication that an engine or UI transport can mount
/// without changing any authored or persisted video DTO.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoClipRuntimeSnapshot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<VideoClipLayerRuntimeSummary>,
    /// Exact renderer-only Timeline Video projection IDs for this published
    /// frame.  The set is runtime provenance, never authored state, and is
    /// intentionally not serialized.  Keeping it beside the public runtime
    /// snapshot lets audio/output consumers filter projections even when the
    /// optional authored video payload is omitted from the public snapshot.
    #[serde(skip, default)]
    pub timeline_video_projection_layer_ids: Vec<VideoLayerId>,
}

/// Ephemeral truth for one running Layer Transition Bus. `origin_from` is
/// stable across Reverse and gives directional mattes the same complement
/// continuity guarantee as Clip Take.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoLayerTransitionBusRuntimeSummary {
    pub bus_id: VideoTransitionBusId,
    pub origin_from: VideoLayerTransitionTarget,
    pub from: VideoLayerTransitionTarget,
    pub to: VideoLayerTransitionTarget,
    pub kind: VideoClipTakeKind,
    pub curve: VideoLayerTransitionCurve,
    pub elapsed_ms: u64,
    pub duration_ms: u64,
    pub duration: VideoClipTakeDuration,
    pub progress_millis: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoLayerTransitionRuntimeSnapshot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buses: Vec<VideoLayerTransitionBusRuntimeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoLayerSummary {
    pub id: VideoLayerId,
    pub label: String,
    pub source: VideoSourceSummary,
    /// Compatibility projection of the referenced catalog entry. Older
    /// readers continue to consume `source`; engine-ready snapshots require
    /// this reference and require the two sources to be exactly equal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_asset_id: Option<MediaAssetId>,
    pub blend_mode: VideoBlendMode,
    pub state: VideoLayerState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isf_effect: Option<VideoIsfEffectSummary>,
    /// Ordered authored slot bank. Empty is the legacy pre-T2 form and is
    /// omitted so merely reading old projects remains byte-compatible.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clip_slots: Vec<VideoClipSlotSummary>,
    /// Stable default selection for the authored layer. Runtime active/queued
    /// selection is intentionally not persisted in this B1 representation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_clip_slot_id: Option<VideoClipSlotId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoLayerTarget {
    pub layer_id: VideoLayerId,
    pub state: VideoLayerState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ExclusiveVideoTakeRequest {
    pub target_layer_id: VideoLayerId,
    pub fade_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_position_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_speed: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoOutputTarget {
    pub output_id: VideoOutputId,
    pub enabled: bool,
    pub opacity: f32,
    pub blackout: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoOutputKind {
    Display,
    NdiSender,
    SpoutSender,
    SyphonServer,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum VideoOutputAspectMode {
    #[default]
    Stretch,
    Fit,
    Fill,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct VideoMaskPoint {
    pub x: f32,
    pub y: f32,
}

pub const VIDEO_OUTPUT_MASK_POINT_CAPACITY: usize = 8;
pub const VIDEO_OUTPUT_BITMAP_MASK_WORD_CAPACITY: usize = 32;
pub const VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION: u8 = 16;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VideoOutputMapping {
    #[serde(default)]
    pub stage_x: f32,
    #[serde(default)]
    pub stage_y: f32,
    #[serde(default)]
    pub stage_z: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation_deg: f32,
    pub aspect_ratio: f32,
    #[serde(default)]
    pub aspect_mode: VideoOutputAspectMode,
    #[serde(default)]
    pub lens_distortion: f32,
    #[serde(default)]
    pub edge_blend_left: f32,
    #[serde(default)]
    pub edge_blend_right: f32,
    #[serde(default)]
    pub edge_blend_top: f32,
    #[serde(default)]
    pub edge_blend_bottom: f32,
    #[serde(default = "default_edge_blend_gamma")]
    pub edge_blend_gamma: f32,
    #[serde(default)]
    pub black_level: f32,
    #[serde(default)]
    pub mask_point_count: u8,
    #[serde(default)]
    pub mask_invert: bool,
    #[serde(default)]
    pub mask_softness: f32,
    #[serde(default = "default_video_mask_points")]
    pub mask_points: [VideoMaskPoint; VIDEO_OUTPUT_MASK_POINT_CAPACITY],
    #[serde(default)]
    pub bitmap_mask_width: u8,
    #[serde(default)]
    pub bitmap_mask_height: u8,
    #[serde(default)]
    pub bitmap_mask_luma_words: [u32; VIDEO_OUTPUT_BITMAP_MASK_WORD_CAPACITY],
    pub keystone_x: f32,
    pub keystone_y: f32,
    pub corner_top_left_x: f32,
    pub corner_top_left_y: f32,
    pub corner_top_right_x: f32,
    pub corner_top_right_y: f32,
    pub corner_bottom_right_x: f32,
    pub corner_bottom_right_y: f32,
    pub corner_bottom_left_x: f32,
    pub corner_bottom_left_y: f32,
}

impl Default for VideoOutputMapping {
    fn default() -> Self {
        Self {
            stage_x: 0.0,
            stage_y: 0.0,
            stage_z: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation_deg: 0.0,
            aspect_ratio: 1.0,
            aspect_mode: VideoOutputAspectMode::Stretch,
            lens_distortion: 0.0,
            edge_blend_left: 0.0,
            edge_blend_right: 0.0,
            edge_blend_top: 0.0,
            edge_blend_bottom: 0.0,
            edge_blend_gamma: default_edge_blend_gamma(),
            black_level: 0.0,
            mask_point_count: 0,
            mask_invert: false,
            mask_softness: 0.0,
            mask_points: default_video_mask_points(),
            bitmap_mask_width: 0,
            bitmap_mask_height: 0,
            bitmap_mask_luma_words: [0; VIDEO_OUTPUT_BITMAP_MASK_WORD_CAPACITY],
            keystone_x: 0.0,
            keystone_y: 0.0,
            corner_top_left_x: 0.0,
            corner_top_left_y: 0.0,
            corner_top_right_x: 0.0,
            corner_top_right_y: 0.0,
            corner_bottom_right_x: 0.0,
            corner_bottom_right_y: 0.0,
            corner_bottom_left_x: 0.0,
            corner_bottom_left_y: 0.0,
        }
    }
}

const fn default_video_mask_points() -> [VideoMaskPoint; VIDEO_OUTPUT_MASK_POINT_CAPACITY] {
    [VideoMaskPoint { x: 0.0, y: 0.0 }; VIDEO_OUTPUT_MASK_POINT_CAPACITY]
}

const fn default_edge_blend_gamma() -> f32 {
    2.2
}

pub fn canonical_video_output_mapping_field(field: &str) -> Option<&'static str> {
    match normalized_video_output_mapping_field_name(field).as_str() {
        "stagex" | "stageposx" | "stagepositionx" | "stagelocx" | "stagelocationx" | "sx" => {
            Some("stage_x")
        }
        "stagey" | "stageposy" | "stagepositiony" | "stagelocy" | "stagelocationy" | "sy" => {
            Some("stage_y")
        }
        "stagez" | "stageposz" | "stagepositionz" | "stagelocz" | "stagelocationz" | "depth"
        | "sz" => Some("stage_z"),
        "offsetx" | "imagex" | "screenx" | "shiftx" | "translatex" | "positionx" | "posx" | "x" => {
            Some("offset_x")
        }
        "offsety" | "imagey" | "screeny" | "shifty" | "translatey" | "positiony" | "posy" | "y" => {
            Some("offset_y")
        }
        "scalex" | "widthscale" | "horizontalscale" | "stretchx" | "sizex" | "width" | "w" => {
            Some("scale_x")
        }
        "scaley" | "heightscale" | "verticalscale" | "stretchy" | "sizey" | "height" | "h" => {
            Some("scale_y")
        }
        "rotation" | "rotationdeg" | "rotate" | "rot" | "angle" | "angledeg" => {
            Some("rotation_deg")
        }
        "aspect" | "aspectratio" | "ratio" | "outputaspect" | "outputratio" | "projectoraspect"
        | "projectorratio" | "screenaspect" | "screenratio" => Some("aspect_ratio"),
        "lens"
        | "lensdistortion"
        | "distortion"
        | "barrel"
        | "barreldistortion"
        | "pincushion"
        | "pincushiondistortion"
        | "lenswarp"
        | "warplens" => Some("lens_distortion"),
        "edgeblendleft" | "blendleft" | "featherleft" | "leftblend" | "leftfeather" => {
            Some("edge_blend_left")
        }
        "edgeblendright" | "blendright" | "featherright" | "rightblend" | "rightfeather" => {
            Some("edge_blend_right")
        }
        "edgeblendtop" | "blendtop" | "feathertop" | "topblend" | "topfeather" => {
            Some("edge_blend_top")
        }
        "edgeblendbottom" | "blendbottom" | "featherbottom" | "bottomblend" | "bottomfeather" => {
            Some("edge_blend_bottom")
        }
        "edgeblendgamma" | "blendgamma" | "feathergamma" => Some("edge_blend_gamma"),
        "blacklevel" | "blacklift" | "projectorblack" => Some("black_level"),
        "keystonex"
        | "keystoneh"
        | "keyx"
        | "keyh"
        | "hkeystone"
        | "horizontalkeystone"
        | "keystonehorizontal"
        | "perspectivex"
        | "perspectiveh"
        | "horizontalperspective" => Some("keystone_x"),
        "keystoney"
        | "keystonev"
        | "keyy"
        | "keyv"
        | "vkeystone"
        | "verticalkeystone"
        | "keystonevertical"
        | "perspectivey"
        | "perspectivev"
        | "verticalperspective" => Some("keystone_y"),
        "cornertopleftx" | "topleftx" | "upperleftx" | "lefttopx" | "tlx" | "ulx" => {
            Some("corner_top_left_x")
        }
        "cornertoplefty" | "toplefty" | "upperlefty" | "lefttopy" | "tly" | "uly" => {
            Some("corner_top_left_y")
        }
        "cornertoprightx" | "toprightx" | "upperrightx" | "righttopx" | "trx" | "urx" => {
            Some("corner_top_right_x")
        }
        "cornertoprighty" | "toprighty" | "upperrighty" | "righttopy" | "try" | "ury" => {
            Some("corner_top_right_y")
        }
        "cornerbottomrightx" | "bottomrightx" | "lowerrightx" | "rightbottomx" | "brx" | "lrx" => {
            Some("corner_bottom_right_x")
        }
        "cornerbottomrighty" | "bottomrighty" | "lowerrighty" | "rightbottomy" | "bry" | "lry" => {
            Some("corner_bottom_right_y")
        }
        "cornerbottomleftx" | "bottomleftx" | "lowerleftx" | "leftbottomx" | "blx" | "llx" => {
            Some("corner_bottom_left_x")
        }
        "cornerbottomlefty" | "bottomlefty" | "lowerlefty" | "leftbottomy" | "bly" | "lly" => {
            Some("corner_bottom_left_y")
        }
        _ => None,
    }
}

pub fn video_output_mapping_field_value(mapping: &VideoOutputMapping, field: &str) -> Option<f32> {
    match canonical_video_output_mapping_field(field)? {
        "stage_x" => Some(mapping.stage_x),
        "stage_y" => Some(mapping.stage_y),
        "stage_z" => Some(mapping.stage_z),
        "offset_x" => Some(mapping.offset_x),
        "offset_y" => Some(mapping.offset_y),
        "scale_x" => Some(mapping.scale_x),
        "scale_y" => Some(mapping.scale_y),
        "rotation_deg" => Some(mapping.rotation_deg),
        "aspect_ratio" => Some(mapping.aspect_ratio),
        "lens_distortion" => Some(mapping.lens_distortion),
        "edge_blend_left" => Some(mapping.edge_blend_left),
        "edge_blend_right" => Some(mapping.edge_blend_right),
        "edge_blend_top" => Some(mapping.edge_blend_top),
        "edge_blend_bottom" => Some(mapping.edge_blend_bottom),
        "edge_blend_gamma" => Some(mapping.edge_blend_gamma),
        "black_level" => Some(mapping.black_level),
        "keystone_x" => Some(mapping.keystone_x),
        "keystone_y" => Some(mapping.keystone_y),
        "corner_top_left_x" => Some(mapping.corner_top_left_x),
        "corner_top_left_y" => Some(mapping.corner_top_left_y),
        "corner_top_right_x" => Some(mapping.corner_top_right_x),
        "corner_top_right_y" => Some(mapping.corner_top_right_y),
        "corner_bottom_right_x" => Some(mapping.corner_bottom_right_x),
        "corner_bottom_right_y" => Some(mapping.corner_bottom_right_y),
        "corner_bottom_left_x" => Some(mapping.corner_bottom_left_x),
        "corner_bottom_left_y" => Some(mapping.corner_bottom_left_y),
        _ => None,
    }
}

pub fn set_video_output_mapping_field_value(
    mapping: &mut VideoOutputMapping,
    field: &str,
    value: f32,
) -> Result<(), String> {
    match canonical_video_output_mapping_field(field) {
        Some("stage_x") => mapping.stage_x = value,
        Some("stage_y") => mapping.stage_y = value,
        Some("stage_z") => mapping.stage_z = value,
        Some("offset_x") => mapping.offset_x = value,
        Some("offset_y") => mapping.offset_y = value,
        Some("scale_x") => mapping.scale_x = value,
        Some("scale_y") => mapping.scale_y = value,
        Some("rotation_deg") => mapping.rotation_deg = value,
        Some("aspect_ratio") => mapping.aspect_ratio = value,
        Some("lens_distortion") => mapping.lens_distortion = value,
        Some("edge_blend_left") => mapping.edge_blend_left = value,
        Some("edge_blend_right") => mapping.edge_blend_right = value,
        Some("edge_blend_top") => mapping.edge_blend_top = value,
        Some("edge_blend_bottom") => mapping.edge_blend_bottom = value,
        Some("edge_blend_gamma") => mapping.edge_blend_gamma = value,
        Some("black_level") => mapping.black_level = value,
        Some("keystone_x") => mapping.keystone_x = value,
        Some("keystone_y") => mapping.keystone_y = value,
        Some("corner_top_left_x") => mapping.corner_top_left_x = value,
        Some("corner_top_left_y") => mapping.corner_top_left_y = value,
        Some("corner_top_right_x") => mapping.corner_top_right_x = value,
        Some("corner_top_right_y") => mapping.corner_top_right_y = value,
        Some("corner_bottom_right_x") => mapping.corner_bottom_right_x = value,
        Some("corner_bottom_right_y") => mapping.corner_bottom_right_y = value,
        Some("corner_bottom_left_x") => mapping.corner_bottom_left_x = value,
        Some("corner_bottom_left_y") => mapping.corner_bottom_left_y = value,
        _ => {
            return Err(format!(
                "Video output mapping field '{}' was not found",
                normalized_video_output_mapping_field_name(field)
            ));
        }
    }
    Ok(())
}

pub fn normalized_video_output_mapping_field_name(field: &str) -> String {
    field
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoOutputSummary {
    pub id: VideoOutputId,
    pub label: String,
    pub kind: VideoOutputKind,
    pub enabled: bool,
    pub composition_id: CompositionId,
    pub fullscreen: bool,
    pub monitor_id: Option<u32>,
    /// Stable physical display identity captured with the authored output.
    /// A missing value is retained for legacy project migration but is never
    /// sufficient to open or synchronize a native Display output.
    #[serde(default)]
    pub monitor_identity: Option<String>,
    pub width: u32,
    pub height: u32,
    pub endpoint_name: Option<String>,
    pub opacity: f32,
    pub blackout: bool,
    #[serde(default)]
    pub mapping: VideoOutputMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoOutputMappingPresetSummary {
    pub label: String,
    pub mapping: VideoOutputMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoOutputMappingPresetFile {
    pub version: u32,
    pub app: String,
    pub preset: VideoOutputMappingPresetSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositionSummary {
    pub id: CompositionId,
    pub label: String,
    pub layer_ids: Vec<VideoLayerId>,
    pub output_ids: Vec<VideoOutputId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoVjConfig {
    #[serde(default)]
    pub eligible_layer_ids: Vec<VideoLayerId>,
    #[serde(default)]
    pub seed: u64,
    #[serde(default = "default_auto_vj_beats_per_change")]
    pub beats_per_change: u16,
    #[serde(default = "default_auto_vj_transition_ms")]
    pub transition_ms: u64,
    #[serde(default = "default_auto_vj_avoid_immediate_repeat")]
    pub avoid_immediate_repeat: bool,
    #[serde(default)]
    pub rhythm_source: AutoVjRhythmSource,
}

impl Default for AutoVjConfig {
    fn default() -> Self {
        Self {
            eligible_layer_ids: Vec::new(),
            seed: 0,
            beats_per_change: default_auto_vj_beats_per_change(),
            transition_ms: default_auto_vj_transition_ms(),
            avoid_immediate_repeat: default_auto_vj_avoid_immediate_repeat(),
            rhythm_source: AutoVjRhythmSource::default(),
        }
    }
}

fn default_auto_vj_beats_per_change() -> u16 {
    4
}

fn default_auto_vj_transition_ms() -> u64 {
    500
}

fn default_auto_vj_avoid_immediate_repeat() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutoVjRhythmSource {
    #[default]
    Clock,
    LiveAudio,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutoVjMode {
    #[default]
    Off,
    Armed,
    Running,
    Hold,
    Fault,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutoVjTrigger {
    #[default]
    ClockBoundary,
    LiveAudioOnset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoVjAction {
    pub sequence: u64,
    pub boundary_index: u64,
    pub beat: u64,
    pub layer_id: VideoLayerId,
    pub transition_ms: u64,
    pub selection_token: u64,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub show_revision: u64,
    #[serde(default)]
    pub trigger: AutoVjTrigger,
    #[serde(default)]
    pub live_audio_feature_sequence: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoVjStatus {
    pub mode: AutoVjMode,
    pub armed: bool,
    pub hold: bool,
    pub show_revision: u64,
    pub action_sequence: u64,
    pub last_consumed_boundary: Option<u64>,
    pub next_boundary_beat: Option<u64>,
    pub last_action: Option<AutoVjAction>,
    #[serde(default)]
    pub action_log: Vec<AutoVjAction>,
    pub fault: Option<String>,
    #[serde(default)]
    pub live_audio_beat_counter: u64,
    #[serde(default)]
    pub last_live_audio_feature_sequence: Option<u64>,
    #[serde(default)]
    pub live_audio_waiting: bool,
    #[serde(default)]
    pub live_audio_generation: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoVjSnapshot {
    #[serde(default)]
    pub config: AutoVjConfig,
    #[serde(default)]
    pub status: AutoVjStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoSnapshot {
    pub layers: Vec<VideoLayerSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media_assets: Vec<MediaAssetSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effect_chains: Vec<VideoEffectChainSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effect_presets: Vec<VideoEffectPresetSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_groups: Vec<VideoLayerGroupSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transition_buses: Vec<VideoLayerTransitionBusSummary>,
    pub compositions: Vec<CompositionSummary>,
    pub outputs: Vec<VideoOutputSummary>,
    #[serde(default)]
    pub mapping_presets: Vec<VideoOutputMappingPresetSummary>,
    pub master_opacity: f32,
    pub blackout: bool,
    #[serde(default)]
    pub auto_vj: AutoVjSnapshot,
}

impl Default for VideoSnapshot {
    fn default() -> Self {
        Self {
            layers: Vec::new(),
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            master_opacity: 1.0,
            blackout: false,
            auto_vj: AutoVjSnapshot::default(),
        }
    }
}

/// What a successful legacy-media normalization changed. The report is
/// intentionally deterministic and contains IDs in serialized layer order so
/// a caller can explain a pending explicit-save migration without inspecting
/// mutable runtime state.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaAssetMigrationReport {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub created_asset_ids: Vec<MediaAssetId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migrated_layer_ids: Vec<VideoLayerId>,
}

/// What a successful legacy clip-slot normalization changed. IDs follow the
/// serialized layer order, which keeps the first explicit save deterministic.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoClipSlotMigrationReport {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub created_slot_ids: Vec<VideoClipSlotId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migrated_layer_ids: Vec<VideoLayerId>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoEffectChainMigrationReport {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub created_chain_ids: Vec<VideoEffectChainId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migrated_layer_ids: Vec<VideoLayerId>,
}

/// Validate persisted media-library invariants while allowing an old layer to
/// retain its `None` reference before migration. Use
/// `validate_engine_ready_video_media_assets` for a runtime-installable image.
pub fn validate_media_asset_catalog(video: &VideoSnapshot) -> Result<(), String> {
    validate_video_media_asset_catalog(video, false)
}

/// Validate the post-migration form consumed by the engine. Every layer must
/// reference one catalog item, and its legacy-compatible `source` projection
/// must exactly mirror that item.
pub fn validate_engine_ready_video_media_assets(video: &VideoSnapshot) -> Result<(), String> {
    validate_video_media_asset_catalog(video, true)
}

/// Validate authored clip slots while still permitting a pre-T2 legacy layer
/// with no bank. This is suitable before the explicit in-memory migration.
pub fn validate_authored_video_clip_slots(video: &VideoSnapshot) -> Result<(), String> {
    validate_video_clip_slots(video, false)
}

/// Validate the T2 form consumed by the runtime. Every media-backed layer has
/// an ordered bank and a valid default; the compatibility `source` and
/// `media_asset_id` remain projections of that default slot.
pub fn validate_engine_ready_video_clip_slots(video: &VideoSnapshot) -> Result<(), String> {
    validate_engine_ready_video_media_assets(video)?;
    validate_video_clip_slots(video, true)
}

/// Validate runtime-local identities and queue state without requiring an
/// authored snapshot. Use `validate_video_clip_runtime_against_authored_slots`
/// when the caller also has the engine-ready authored slot banks.
pub fn validate_video_clip_runtime_snapshot(
    runtime: &VideoClipRuntimeSnapshot,
) -> Result<(), String> {
    let mut layer_ids = BTreeMap::new();
    for layer in &runtime.layers {
        if layer.layer_id == 0 {
            return Err("Video clip runtime layer IDs must be non-zero".to_string());
        }
        if layer_ids.insert(layer.layer_id, ()).is_some() {
            return Err(format!(
                "Video clip runtime layer ID {} is duplicated",
                layer.layer_id
            ));
        }
        validate_video_clip_layer_runtime_summary(layer)?;
    }
    Ok(())
}

/// Validate a runtime publication against the authored T2 slot banks. This
/// rejects runtime layers or active/queued/pending slot identities that cannot
/// be resolved in their own authored layer, without storing runtime truth in
/// the project snapshot.
pub fn validate_video_clip_runtime_against_authored_slots(
    runtime: &VideoClipRuntimeSnapshot,
    video: &VideoSnapshot,
) -> Result<(), String> {
    validate_video_clip_runtime_snapshot(runtime)?;
    validate_engine_ready_video_clip_slots(video)?;

    let authored_layers: BTreeMap<VideoLayerId, &VideoLayerSummary> =
        video.layers.iter().map(|layer| (layer.id, layer)).collect();
    for runtime_layer in &runtime.layers {
        let authored_layer = authored_layers
            .get(&runtime_layer.layer_id)
            .ok_or_else(|| {
                format!(
                    "Video clip runtime layer {} is not in the authored video snapshot",
                    runtime_layer.layer_id
                )
            })?;
        let slot_ids: BTreeMap<VideoClipSlotId, &VideoClipSlotSummary> = authored_layer
            .clip_slots
            .iter()
            .map(|slot| (slot.id, slot))
            .collect();

        let active_slot = validate_runtime_slot_reference(
            runtime_layer.layer_id,
            "active",
            runtime_layer.active_slot_id,
            &slot_ids,
        )?;
        validate_runtime_slot_reference(
            runtime_layer.layer_id,
            "queued",
            runtime_layer.queued_slot_id,
            &slot_ids,
        )?;
        if let Some(pending_launch) = &runtime_layer.pending_launch {
            validate_runtime_slot_reference(
                runtime_layer.layer_id,
                "pending",
                Some(pending_launch.slot_id),
                &slot_ids,
            )?;
        }
        if let Some(transition) = &runtime_layer.transition {
            validate_runtime_slot_reference(
                runtime_layer.layer_id,
                "transition outgoing",
                Some(transition.outgoing_slot_id),
                &slot_ids,
            )?;
            let incoming_slot = validate_runtime_slot_reference(
                runtime_layer.layer_id,
                "transition incoming",
                Some(transition.incoming_slot_id),
                &slot_ids,
            )?
            .expect("transition incoming slot reference was provided");
            if transition.incoming_playhead_ms < incoming_slot.in_point_ms
                || incoming_slot
                    .out_point_ms
                    .is_some_and(|out_point_ms| transition.incoming_playhead_ms >= out_point_ms)
            {
                return Err(format!(
                    "Video clip runtime layer {} incoming playhead is outside transition slot {} range",
                    runtime_layer.layer_id, incoming_slot.id.0
                ));
            }
        }
        if let Some(active_slot) = active_slot {
            if runtime_layer.ping_pong_reverse
                && !matches!(active_slot.loop_mode, VideoClipLoopMode::PingPong)
            {
                return Err(format!(
                    "Video clip runtime layer {} reports reverse ping-pong direction for non-ping-pong slot {}",
                    runtime_layer.layer_id, active_slot.id.0
                ));
            }
            if runtime_layer.playhead_ms < active_slot.in_point_ms
                || active_slot
                    .out_point_ms
                    .is_some_and(|out_point_ms| runtime_layer.playhead_ms >= out_point_ms)
            {
                return Err(format!(
                    "Video clip runtime layer {} playhead is outside active slot {} range",
                    runtime_layer.layer_id, active_slot.id.0
                ));
            }
        }
    }
    Ok(())
}

pub fn validate_video_layer_transition_runtime(
    runtime: &VideoLayerTransitionRuntimeSnapshot,
    video: &VideoSnapshot,
) -> Result<(), String> {
    validate_authored_video_effect_chains(video)?;
    let layers: BTreeMap<VideoLayerId, &VideoLayerSummary> =
        video.layers.iter().map(|layer| (layer.id, layer)).collect();
    let compositions: BTreeMap<CompositionId, &CompositionSummary> = video
        .compositions
        .iter()
        .map(|composition| (composition.id, composition))
        .collect();
    let groups: BTreeMap<VideoLayerGroupId, &VideoLayerGroupSummary> = video
        .layer_groups
        .iter()
        .map(|group| (group.id, group))
        .collect();
    let buses: BTreeMap<VideoTransitionBusId, &VideoLayerTransitionBusSummary> = video
        .transition_buses
        .iter()
        .map(|bus| (bus.id, bus))
        .collect();
    let mut active_bus_ids = BTreeMap::new();
    let mut active_layers = BTreeMap::new();
    for active in &runtime.buses {
        if active.bus_id.0 == 0 || active_bus_ids.insert(active.bus_id, ()).is_some() {
            return Err("Active video transition bus IDs must be non-zero and unique".to_string());
        }
        let bus = buses.get(&active.bus_id).ok_or_else(|| {
            format!(
                "Active video transition references missing bus {}",
                active.bus_id.0
            )
        })?;
        if !bus.enabled {
            return Err(format!(
                "Disabled video transition bus {} cannot be active",
                bus.id.0
            ));
        }
        if active.from == active.to
            || (active.origin_from != active.from && active.origin_from != active.to)
            || !bus.members.contains(&active.from)
            || !bus.members.contains(&active.to)
            || active.progress_millis > 1000
        {
            return Err(format!(
                "Active video transition bus {} has invalid from/to/timing truth",
                bus.id.0
            ));
        }
        let expected_progress = if matches!(active.kind, VideoClipTakeKind::Cut) {
            if active.duration.value_milliunits != 0
                || active.duration_ms != 0
                || active.elapsed_ms != 0
            {
                return Err(format!(
                    "Active Cut video transition bus {} must have zero timing",
                    bus.id.0
                ));
            }
            1000
        } else {
            if active.duration.value_milliunits == 0
                || active.duration_ms == 0
                || active.elapsed_ms > active.duration_ms
            {
                return Err(format!(
                    "Active video transition bus {} has invalid timing truth",
                    bus.id.0
                ));
            }
            if matches!(
                active.duration.unit,
                VideoClipTakeDurationUnit::Milliseconds
            ) && active.duration.value_milliunits != active.duration_ms
            {
                return Err(format!(
                    "Active video transition bus {} millisecond intent disagrees with resolved duration",
                    bus.id.0
                ));
            }
            active
                .elapsed_ms
                .saturating_mul(1000)
                .saturating_div(active.duration_ms)
                .min(1000) as u16
        };
        if active.progress_millis != expected_progress {
            return Err(format!(
                "Active video transition bus {} progress does not match elapsed duration",
                bus.id.0
            ));
        }
        let composition = compositions
            .get(&bus.composition_id)
            .expect("authored transition bus composition was validated");
        for target in [&active.from, &active.to] {
            for layer_id in
                video_transition_target_layer_ids(target, composition, &layers, &groups)?
            {
                if let Some(conflicting_bus) = active_layers.insert(layer_id, active.bus_id) {
                    return Err(format!(
                        "Active video transition buses {} and {} conflict at layer {layer_id}",
                        conflicting_bus.0, active.bus_id.0
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Convert an old inline-video project into the catalog representation without
/// changing a caller's value unless *all* validation and ID allocation steps
/// succeed. The conversion never hashes files, resolves paths, or otherwise
/// observes machine-local state, so loading a project cannot touch its source
/// files or rewrite its JSON.
pub fn normalize_legacy_video_media_assets(
    video: &mut VideoSnapshot,
) -> Result<MediaAssetMigrationReport, String> {
    // Work on a complete candidate. In particular, a duplicate/dangling ref
    // after several legacy layers must not leave those earlier layers migrated.
    let mut candidate = video.clone();
    validate_media_asset_catalog(&candidate)?;

    let mut next_asset_id = candidate
        .media_assets
        .iter()
        .map(|asset| asset.id)
        .chain(candidate.layers.iter().map(|layer| layer.id))
        .max()
        .unwrap_or(0);
    let mut report = MediaAssetMigrationReport::default();

    for layer_index in 0..candidate.layers.len() {
        if candidate.layers[layer_index].media_asset_id.is_some() {
            continue;
        }
        next_asset_id = next_asset_id.checked_add(1).ok_or_else(|| {
            "Media asset ID allocation overflow while migrating legacy video layers".to_string()
        })?;
        let layer = candidate.layers[layer_index].clone();
        candidate.media_assets.push(MediaAssetSummary {
            id: next_asset_id,
            // Lossless migration means no trim, fallback label, probe refresh,
            // path normalization, or dedupe. A legacy layer becomes exactly one
            // independently addressable library entry.
            label: layer.label.clone(),
            source: layer.source.clone(),
            content_hash: None,
            byte_size: None,
        });
        candidate.layers[layer_index].media_asset_id = Some(next_asset_id);
        report.created_asset_ids.push(next_asset_id);
        report.migrated_layer_ids.push(layer.id);
    }

    validate_engine_ready_video_media_assets(&candidate)?;
    *video = candidate;
    Ok(report)
}

/// Add one default authored slot to every media-backed legacy layer without
/// allocating or modifying any media-library entry. The whole operation is a
/// clone-then-validate-then-commit transaction: a bad later layer leaves the
/// caller's snapshot untouched, and a second successful pass is byte-stable.
///
/// Call `normalize_legacy_video_media_assets` first for an older inline-media
/// project. This helper intentionally does not create MediaAssets because T2
/// must preserve the T1 catalog identity exactly.
pub fn normalize_legacy_video_clip_slots(
    video: &mut VideoSnapshot,
) -> Result<VideoClipSlotMigrationReport, String> {
    let mut candidate = video.clone();
    validate_engine_ready_video_media_assets(&candidate)?;
    validate_authored_video_clip_slots(&candidate)?;

    let mut next_slot_id = candidate
        .layers
        .iter()
        .map(|layer| layer.id)
        .chain(candidate.media_assets.iter().map(|asset| asset.id))
        .chain(
            candidate
                .layers
                .iter()
                .flat_map(|layer| layer.clip_slots.iter().map(|slot| slot.id.0)),
        )
        .max()
        .unwrap_or(0);
    let mut report = VideoClipSlotMigrationReport::default();

    for layer_index in 0..candidate.layers.len() {
        if !candidate.layers[layer_index].clip_slots.is_empty() {
            continue;
        }
        let media_asset_id = candidate.layers[layer_index]
            .media_asset_id
            .ok_or_else(|| {
                format!(
                    "Video layer {} is missing its media asset reference",
                    candidate.layers[layer_index].id
                )
            })?;
        next_slot_id = next_slot_id.checked_add(1).ok_or_else(|| {
            "Video clip slot ID allocation overflow while migrating legacy video layers".to_string()
        })?;
        let slot_id = VideoClipSlotId(next_slot_id);
        let layer = &mut candidate.layers[layer_index];
        let (in_point_ms, out_point_ms, loop_mode, speed, cue_points) =
            legacy_video_layer_transport_to_clip_slot(&layer.state, layer.id)?;
        layer.clip_slots.push(VideoClipSlotSummary {
            id: slot_id,
            media_asset_id,
            in_point_ms,
            out_point_ms,
            loop_mode,
            speed,
            cue_points,
            launch_quantization: VideoClipLaunchQuantization::Immediate,
            effect_overrides: Vec::new(),
        });
        layer.default_clip_slot_id = Some(slot_id);
        report.created_slot_ids.push(slot_id);
        report.migrated_layer_ids.push(layer.id);
    }

    validate_engine_ready_video_clip_slots(&candidate)?;
    *video = candidate;
    Ok(report)
}

/// Normalize legacy layer ISF stacks into stable, centrally scoped chains.
/// The legacy `layers[].isf_effect` value remains an exact compatibility
/// projection; only an explicit save serializes the additive central model.
pub fn normalize_legacy_video_effect_chains(
    video: &mut VideoSnapshot,
) -> Result<VideoEffectChainMigrationReport, String> {
    let mut candidate = video.clone();
    validate_authored_video_effect_chains(&candidate)?;

    let mut next_chain_id = candidate
        .effect_chains
        .iter()
        .map(|chain| chain.id.0)
        .max()
        .unwrap_or(0);
    let mut next_stage_id = candidate
        .effect_chains
        .iter()
        .flat_map(|chain| chain.stages.iter().map(|stage| stage.id.0))
        .max()
        .unwrap_or(0);
    let mut next_effect_id = candidate
        .effect_chains
        .iter()
        .flat_map(|chain| chain.stages.iter().map(|stage| stage.effect.id.0))
        .max()
        .unwrap_or(0);
    let mut report = VideoEffectChainMigrationReport::default();

    for layer_index in 0..candidate.layers.len() {
        let layer_id = candidate.layers[layer_index].id;
        let scope = VideoEffectScope::Layer { layer_id };
        let existing_index = candidate
            .effect_chains
            .iter()
            .position(|chain| chain.scope == scope);
        if let Some(index) = existing_index {
            validate_layer_effect_chain_projection(
                &candidate.layers[layer_index],
                &candidate.effect_chains[index],
            )?;
            reconcile_clip_override_effect_ids(
                &mut candidate.layers[layer_index],
                &candidate.effect_chains[index],
            )?;
            continue;
        }
        let Some(legacy) = candidate.layers[layer_index].isf_effect.clone() else {
            continue;
        };
        next_chain_id = next_chain_id.checked_add(1).ok_or_else(|| {
            "Video effect chain ID allocation overflow during legacy migration".to_string()
        })?;
        let mut legacy_stages = Vec::with_capacity(1 + legacy.stack.len());
        let mut root = legacy.clone();
        root.stack.clear();
        legacy_stages.push(root);
        legacy_stages.extend(
            legacy
                .stack
                .iter()
                .cloned()
                .map(|stage| VideoIsfEffectSummary {
                    enabled: stage.enabled,
                    label: stage.label,
                    source: stage.source,
                    source_path: stage.source_path,
                    description: stage.description,
                    categories: stage.categories,
                    controls: stage.controls,
                    stack: Vec::new(),
                }),
        );
        let mut stages = Vec::with_capacity(legacy_stages.len());
        for mut effect in legacy_stages {
            next_stage_id = next_stage_id.checked_add(1).ok_or_else(|| {
                "Video effect stage ID allocation overflow during legacy migration".to_string()
            })?;
            next_effect_id = next_effect_id.checked_add(1).ok_or_else(|| {
                "Video effect ID allocation overflow during legacy migration".to_string()
            })?;
            let stage_enabled = effect.enabled;
            effect.enabled = true;
            stages.push(VideoEffectStageSummary {
                id: VideoEffectStageId(next_stage_id),
                enabled: stage_enabled,
                label: effect.label.clone(),
                effect: VideoEffectSummary {
                    id: VideoEffectId(next_effect_id),
                    kind: VideoEffectKind::Isf { effect },
                },
            });
        }
        let chain = VideoEffectChainSummary {
            id: VideoEffectChainId(next_chain_id),
            scope,
            bypassed: false,
            stages,
        };
        reconcile_clip_override_effect_ids(&mut candidate.layers[layer_index], &chain)?;
        candidate.effect_chains.push(chain);
        report
            .created_chain_ids
            .push(VideoEffectChainId(next_chain_id));
        report.migrated_layer_ids.push(layer_id);
    }

    validate_engine_ready_video_effect_chains(&candidate)?;
    *video = candidate;
    Ok(report)
}

pub fn validate_authored_video_effect_chains(video: &VideoSnapshot) -> Result<(), String> {
    validate_video_effect_chains(video, false)
}

pub fn validate_engine_ready_video_effect_chains(video: &VideoSnapshot) -> Result<(), String> {
    validate_video_effect_chains(video, true)
}

fn validate_video_effect_chains(
    video: &VideoSnapshot,
    require_projection: bool,
) -> Result<(), String> {
    validate_authored_video_clip_slots(video)?;
    if video.effect_chains.len() > VIDEO_EFFECT_PROJECT_MAX_CHAINS {
        return Err(format!(
            "Video project exceeds {VIDEO_EFFECT_PROJECT_MAX_CHAINS} effect chains"
        ));
    }
    if video.effect_presets.len() > VIDEO_EFFECT_PROJECT_MAX_PRESETS {
        return Err(format!(
            "Video project exceeds {VIDEO_EFFECT_PROJECT_MAX_PRESETS} effect presets"
        ));
    }
    if video.layer_groups.len() > VIDEO_EFFECT_PROJECT_MAX_GROUPS {
        return Err(format!(
            "Video project exceeds {VIDEO_EFFECT_PROJECT_MAX_GROUPS} layer groups"
        ));
    }
    if video.transition_buses.len() > VIDEO_TRANSITION_BUS_PROJECT_MAX_BUSES {
        return Err(format!(
            "Video project exceeds {VIDEO_TRANSITION_BUS_PROJECT_MAX_BUSES} transition buses"
        ));
    }

    let layers: BTreeMap<VideoLayerId, &VideoLayerSummary> =
        video.layers.iter().map(|layer| (layer.id, layer)).collect();
    let compositions: BTreeMap<CompositionId, &CompositionSummary> = video
        .compositions
        .iter()
        .map(|composition| (composition.id, composition))
        .collect();
    let outputs: BTreeMap<VideoOutputId, &VideoOutputSummary> = video
        .outputs
        .iter()
        .map(|output| (output.id, output))
        .collect();
    if layers.len() != video.layers.len() || layers.contains_key(&0) {
        return Err("Video layer IDs must be non-zero and unique".to_string());
    }
    if compositions.len() != video.compositions.len() || compositions.contains_key(&0) {
        return Err("Video composition IDs must be non-zero and unique".to_string());
    }
    if outputs.len() != video.outputs.len() || outputs.contains_key(&0) {
        return Err("Video output IDs must be non-zero and unique".to_string());
    }
    let groups: BTreeMap<VideoLayerGroupId, &VideoLayerGroupSummary> = video
        .layer_groups
        .iter()
        .map(|group| (group.id, group))
        .collect();
    if groups.len() != video.layer_groups.len() || groups.contains_key(&VideoLayerGroupId(0)) {
        return Err("Video layer group IDs must be non-zero and unique".to_string());
    }

    let mut grouped_layers = BTreeMap::new();
    for group in &video.layer_groups {
        if group.label.trim().is_empty() || group.label != group.label.trim() {
            return Err(format!(
                "Video layer group {} label must be canonical",
                group.id.0
            ));
        }
        let composition = compositions.get(&group.composition_id).ok_or_else(|| {
            format!(
                "Video layer group {} references missing composition {}",
                group.id.0, group.composition_id
            )
        })?;
        if group.layer_ids.is_empty() {
            return Err(format!(
                "Video layer group {} must contain at least one layer",
                group.id.0
            ));
        }
        let mut indices = Vec::with_capacity(group.layer_ids.len());
        for layer_id in &group.layer_ids {
            if !layers.contains_key(layer_id)
                || grouped_layers.insert(*layer_id, group.id).is_some()
            {
                return Err(format!(
                    "Video layer {} is missing or belongs to multiple groups",
                    layer_id
                ));
            }
            let index = composition
                .layer_ids
                .iter()
                .position(|candidate| candidate == layer_id)
                .ok_or_else(|| {
                    format!(
                        "Video layer group {} member {} is outside composition {}",
                        group.id.0, layer_id, composition.id
                    )
                })?;
            indices.push(index);
        }
        if indices.windows(2).any(|pair| pair[1] != pair[0] + 1) {
            return Err(format!(
                "Video layer group {} members must follow contiguous composition order",
                group.id.0
            ));
        }
    }

    let buses: BTreeMap<VideoTransitionBusId, &VideoLayerTransitionBusSummary> = video
        .transition_buses
        .iter()
        .map(|bus| (bus.id, bus))
        .collect();
    if buses.len() != video.transition_buses.len() || buses.contains_key(&VideoTransitionBusId(0)) {
        return Err("Video transition bus IDs must be non-zero and unique".to_string());
    }
    for bus in &video.transition_buses {
        validate_video_transition_bus(bus, &layers, &compositions, &groups)?;
    }

    let mut chain_ids = BTreeMap::new();
    let mut stage_ids = BTreeMap::new();
    let mut effect_ids = BTreeMap::new();
    let mut scopes = BTreeMap::new();
    for chain in &video.effect_chains {
        if chain.id.0 == 0 || chain_ids.insert(chain.id, ()).is_some() {
            return Err("Video effect chain IDs must be non-zero and unique".to_string());
        }
        if scopes.insert(chain.scope.clone(), chain.id).is_some() {
            return Err("Each video effect scope may own at most one chain".to_string());
        }
        validate_video_effect_scope(
            &chain.scope,
            &layers,
            &compositions,
            &outputs,
            &groups,
            &buses,
        )?;
        if chain.stages.len() > VIDEO_EFFECT_CHAIN_MAX_STAGES {
            return Err(format!(
                "Video effect chain {} exceeds {VIDEO_EFFECT_CHAIN_MAX_STAGES} stages",
                chain.id.0
            ));
        }
        for stage in &chain.stages {
            if stage.id.0 == 0 || stage_ids.insert(stage.id, ()).is_some() {
                return Err("Video effect stage IDs must be non-zero and unique".to_string());
            }
            if stage.effect.id.0 == 0 || effect_ids.insert(stage.effect.id, ()).is_some() {
                return Err("Video effect IDs must be non-zero and unique".to_string());
            }
            validate_canonical_label("effect stage", stage.id.0, &stage.label)?;
            validate_video_effect_kind(&stage.effect.kind)?;
        }
    }

    let mut preset_ids = BTreeMap::new();
    for preset in &video.effect_presets {
        if preset.id.0 == 0 || preset_ids.insert(preset.id, ()).is_some() {
            return Err("Video effect preset IDs must be non-zero and unique".to_string());
        }
        validate_canonical_label("effect preset", preset.id.0, &preset.label)?;
        if preset.payload.stages.len() > VIDEO_EFFECT_CHAIN_MAX_STAGES {
            return Err(format!(
                "Video effect preset {} exceeds {VIDEO_EFFECT_CHAIN_MAX_STAGES} stages",
                preset.id.0
            ));
        }
        for stage in &preset.payload.stages {
            if stage.label.trim().is_empty() || stage.label != stage.label.trim() {
                return Err(format!(
                    "Video effect preset {} stage label must be canonical",
                    preset.id.0
                ));
            }
            validate_video_effect_kind(&stage.effect)?;
        }
    }

    for layer in &video.layers {
        let layer_chain = video
            .effect_chains
            .iter()
            .find(|chain| chain.scope == VideoEffectScope::Layer { layer_id: layer.id });
        if require_projection && layer.isf_effect.is_some() != layer_chain.is_some() {
            return Err(format!(
                "Video layer {} legacy ISF and canonical layer chain must both exist",
                layer.id
            ));
        }
        if let Some(chain) = layer_chain {
            validate_layer_effect_chain_projection(layer, chain)?;
            validate_clip_override_effect_ids(layer, chain, require_projection)?;
        } else if require_projection
            && layer
                .clip_slots
                .iter()
                .any(|slot| !slot.effect_overrides.is_empty())
        {
            return Err(format!(
                "Video layer {} clip overrides require a canonical layer chain",
                layer.id
            ));
        }
    }
    Ok(())
}

fn video_transition_target_layer_ids(
    target: &VideoLayerTransitionTarget,
    composition: &CompositionSummary,
    layers: &BTreeMap<VideoLayerId, &VideoLayerSummary>,
    groups: &BTreeMap<VideoLayerGroupId, &VideoLayerGroupSummary>,
) -> Result<Vec<VideoLayerId>, String> {
    match target {
        VideoLayerTransitionTarget::Layer { layer_id } => {
            if !layers.contains_key(layer_id) || !composition.layer_ids.contains(layer_id) {
                return Err(format!(
                    "Video transition target references layer {layer_id} outside composition {}",
                    composition.id
                ));
            }
            Ok(vec![*layer_id])
        }
        VideoLayerTransitionTarget::Group { group_id } => {
            let group = groups.get(group_id).ok_or_else(|| {
                format!(
                    "Video transition target references missing group {}",
                    group_id.0
                )
            })?;
            if group.composition_id != composition.id {
                return Err(format!(
                    "Video transition group {} is outside composition {}",
                    group_id.0, composition.id
                ));
            }
            Ok(group.layer_ids.clone())
        }
    }
}

fn validate_video_transition_bus(
    bus: &VideoLayerTransitionBusSummary,
    layers: &BTreeMap<VideoLayerId, &VideoLayerSummary>,
    compositions: &BTreeMap<CompositionId, &CompositionSummary>,
    groups: &BTreeMap<VideoLayerGroupId, &VideoLayerGroupSummary>,
) -> Result<(), String> {
    validate_canonical_label("transition bus", bus.id.0, &bus.label)?;
    let composition = compositions.get(&bus.composition_id).ok_or_else(|| {
        format!(
            "Video transition bus {} references missing composition {}",
            bus.id.0, bus.composition_id
        )
    })?;
    if !(2..=VIDEO_TRANSITION_BUS_MAX_MEMBERS).contains(&bus.members.len()) {
        return Err(format!(
            "Video transition bus {} must contain 2..={VIDEO_TRANSITION_BUS_MAX_MEMBERS} members",
            bus.id.0
        ));
    }
    let mut targets = BTreeMap::new();
    let mut expanded_layers = BTreeMap::new();
    let mut expanded_positions = Vec::new();
    for target in &bus.members {
        if targets.insert(target.clone(), ()).is_some() {
            return Err(format!(
                "Video transition bus {} contains duplicate members",
                bus.id.0
            ));
        }
        if let VideoLayerTransitionTarget::Layer { layer_id } = target {
            if groups
                .values()
                .any(|group| group.layer_ids.contains(layer_id))
            {
                return Err(format!(
                    "Video transition bus {} must target group-owned layer {layer_id} through its group",
                    bus.id.0
                ));
            }
        }
        for layer_id in video_transition_target_layer_ids(target, composition, layers, groups)? {
            if expanded_layers.insert(layer_id, target).is_some() {
                return Err(format!(
                    "Video transition bus {} members overlap at layer {layer_id}",
                    bus.id.0
                ));
            }
            expanded_positions.push(
                composition
                    .layer_ids
                    .iter()
                    .position(|candidate| *candidate == layer_id)
                    .expect("transition target membership was validated"),
            );
        }
    }
    if expanded_positions
        .windows(2)
        .any(|pair| pair[1] != pair[0].saturating_add(1))
    {
        return Err(format!(
            "Video transition bus {} members must follow contiguous composition order",
            bus.id.0
        ));
    }
    if bus.default_from == bus.default_to
        || !targets.contains_key(&bus.default_from)
        || !targets.contains_key(&bus.default_to)
    {
        return Err(format!(
            "Video transition bus {} defaults must be distinct members",
            bus.id.0
        ));
    }
    if let Some(matte) = &bus.matte_source {
        video_transition_target_layer_ids(matte, composition, layers, groups)?;
    }
    let invalid_duration = if matches!(bus.default_kind, VideoClipTakeKind::Cut) {
        bus.default_duration.value_milliunits != 0
    } else {
        bus.default_duration.value_milliunits == 0
    };
    if invalid_duration {
        return Err(format!(
            "Video transition bus {} has invalid default duration",
            bus.id.0
        ));
    }
    Ok(())
}

fn validate_video_effect_scope(
    scope: &VideoEffectScope,
    layers: &BTreeMap<VideoLayerId, &VideoLayerSummary>,
    compositions: &BTreeMap<CompositionId, &CompositionSummary>,
    outputs: &BTreeMap<VideoOutputId, &VideoOutputSummary>,
    groups: &BTreeMap<VideoLayerGroupId, &VideoLayerGroupSummary>,
    buses: &BTreeMap<VideoTransitionBusId, &VideoLayerTransitionBusSummary>,
) -> Result<(), String> {
    match scope {
        VideoEffectScope::Clip { layer_id, slot_id } => {
            let layer = layers
                .get(layer_id)
                .ok_or_else(|| format!("Video clip effect references missing layer {layer_id}"))?;
            if !layer.clip_slots.iter().any(|slot| slot.id == *slot_id) {
                return Err(format!(
                    "Video clip effect references missing slot {}",
                    slot_id.0
                ));
            }
        }
        VideoEffectScope::Layer { layer_id } => {
            if !layers.contains_key(layer_id) {
                return Err(format!(
                    "Video layer effect references missing layer {layer_id}"
                ));
            }
        }
        VideoEffectScope::Transition {
            owner: VideoTransitionEffectOwner::ClipTake { layer_id },
        } => {
            if !layers.contains_key(layer_id) {
                return Err(format!(
                    "Video Clip Take effect references missing layer {layer_id}"
                ));
            }
        }
        VideoEffectScope::Transition {
            owner: VideoTransitionEffectOwner::LayerBus { bus_id },
        } => {
            if !buses.contains_key(bus_id) {
                return Err(format!(
                    "Video Layer Bus effect references missing bus {}",
                    bus_id.0
                ));
            }
        }
        VideoEffectScope::Transition {
            owner: VideoTransitionEffectOwner::TimelineFollow { source_timeline_id },
        } => {
            if source_timeline_id.0 == 0 {
                return Err("Video Timeline Follow effect references Timeline ID zero".to_string());
            }
        }
        VideoEffectScope::Composition { composition_id } => {
            if !compositions.contains_key(composition_id) {
                return Err(format!(
                    "Video composition effect references missing composition {composition_id}"
                ));
            }
        }
        VideoEffectScope::Group { group_id } => {
            if !groups.contains_key(group_id) {
                return Err(format!(
                    "Video group effect references missing group {}",
                    group_id.0
                ));
            }
        }
        VideoEffectScope::Output { output_id } => {
            if !outputs.contains_key(output_id) {
                return Err(format!(
                    "Video output effect references missing output {output_id}"
                ));
            }
        }
    }
    Ok(())
}

fn validate_canonical_label(kind: &str, id: u64, label: &str) -> Result<(), String> {
    if label.trim().is_empty() || label != label.trim() {
        return Err(format!(
            "Video {kind} {id} label must be non-empty and trimmed"
        ));
    }
    Ok(())
}

fn validate_video_effect_kind(kind: &VideoEffectKind) -> Result<(), String> {
    match kind {
        VideoEffectKind::Isf { effect } => {
            if !effect.enabled {
                return Err(
                    "Canonical ISF effect entities must be enabled; stage state owns bypass"
                        .to_string(),
                );
            }
            if effect.label.trim().is_empty()
                || effect.label != effect.label.trim()
                || effect.source.trim().is_empty()
            {
                return Err(
                    "Video ISF effect label/source must be canonical and non-empty".to_string(),
                );
            }
            if !effect.stack.is_empty() {
                return Err(
                    "Canonical ISF effect entities may not contain a legacy nested stack"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

fn projected_legacy_isf(
    chain: &VideoEffectChainSummary,
) -> Result<Option<VideoIsfEffectSummary>, String> {
    if chain.stages.is_empty() {
        return Ok(None);
    }
    let mut projected = Vec::with_capacity(chain.stages.len());
    for stage in &chain.stages {
        let VideoEffectKind::Isf { effect } = &stage.effect.kind;
        let mut effect = effect.clone();
        effect.enabled = !chain.bypassed && stage.enabled;
        effect.stack.clear();
        projected.push(effect);
    }
    let mut root = projected.remove(0);
    root.stack = projected
        .into_iter()
        .map(|effect| VideoIsfEffectStageSummary {
            enabled: effect.enabled,
            label: effect.label,
            source: effect.source,
            source_path: effect.source_path,
            description: effect.description,
            categories: effect.categories,
            controls: effect.controls,
        })
        .collect();
    Ok(Some(root))
}

fn validate_layer_effect_chain_projection(
    layer: &VideoLayerSummary,
    chain: &VideoEffectChainSummary,
) -> Result<(), String> {
    if chain.scope != (VideoEffectScope::Layer { layer_id: layer.id }) {
        return Err(format!(
            "Video layer {} effect chain has wrong scope",
            layer.id
        ));
    }
    if projected_legacy_isf(chain)? != layer.isf_effect {
        return Err(format!(
            "Video layer {} legacy ISF projection diverges from canonical chain",
            layer.id
        ));
    }
    Ok(())
}

fn reconcile_clip_override_effect_ids(
    layer: &mut VideoLayerSummary,
    chain: &VideoEffectChainSummary,
) -> Result<(), String> {
    for slot in &mut layer.clip_slots {
        for effect_override in &mut slot.effect_overrides {
            let effect_id = chain
                .stages
                .get(effect_override.stage_index)
                .map(|stage| stage.effect.id)
                .ok_or_else(|| {
                    format!(
                        "Video layer {} clip override stage is outside canonical chain",
                        layer.id
                    )
                })?;
            if effect_override
                .effect_id
                .is_some_and(|existing| existing != effect_id)
            {
                return Err(format!(
                    "Video layer {} clip override effect identity diverges",
                    layer.id
                ));
            }
            effect_override.effect_id = Some(effect_id);
        }
    }
    Ok(())
}

fn validate_clip_override_effect_ids(
    layer: &VideoLayerSummary,
    chain: &VideoEffectChainSummary,
    require_ids: bool,
) -> Result<(), String> {
    for slot in &layer.clip_slots {
        for effect_override in &slot.effect_overrides {
            let expected = chain
                .stages
                .get(effect_override.stage_index)
                .map(|stage| stage.effect.id)
                .ok_or_else(|| {
                    format!(
                        "Video layer {} clip override stage is outside canonical chain",
                        layer.id
                    )
                })?;
            if require_ids && effect_override.effect_id != Some(expected) {
                return Err(format!(
                    "Video layer {} clip override must target the exact stable effect",
                    layer.id
                ));
            }
            if effect_override
                .effect_id
                .is_some_and(|effect_id| effect_id != expected)
            {
                return Err(format!(
                    "Video layer {} clip override stable effect and stage index diverge",
                    layer.id
                ));
            }
        }
    }
    Ok(())
}

/// Translate the authored portion of the old layer transport into a default
/// slot. The layer state itself remains untouched because `position_ms` and
/// `playing` are legacy runtime truth, not new persisted slot state.
type LegacyVideoLayerClipSlot = (
    u64,
    Option<u64>,
    VideoClipLoopMode,
    f32,
    Vec<VideoClipCuePointSummary>,
);

fn legacy_video_layer_transport_to_clip_slot(
    state: &VideoLayerState,
    layer_id: VideoLayerId,
) -> Result<LegacyVideoLayerClipSlot, String> {
    if !state.speed.is_finite() || !(-4.0..=4.0).contains(&state.speed) {
        return Err(format!(
            "Video layer {layer_id} has legacy speed outside finite -4.0..=4.0"
        ));
    }
    let (in_point_ms, out_point_ms, loop_mode) = if state.loop_enabled {
        if state.loop_end_ms == 0 || state.loop_start_ms >= state.loop_end_ms {
            return Err(format!(
                "Video layer {layer_id} has invalid legacy loop bounds"
            ));
        }
        (
            state.loop_start_ms,
            Some(state.loop_end_ms),
            VideoClipLoopMode::Loop,
        )
    } else {
        (0, None, VideoClipLoopMode::Once)
    };

    let named_positions: Vec<u64> = state
        .cue_points
        .iter()
        .map(|cue_point| cue_point.position_ms)
        .collect();
    if !state.cue_points.is_empty()
        && !state.cue_points_ms.is_empty()
        && named_positions != state.cue_points_ms
    {
        return Err(format!(
            "Video layer {layer_id} has disagreeing legacy cue point positions"
        ));
    }
    let cue_points: Vec<VideoClipCuePointSummary> = if !state.cue_points.is_empty() {
        state
            .cue_points
            .iter()
            .map(|cue_point| VideoClipCuePointSummary {
                position_ms: cue_point.position_ms,
                // Legacy labels have no canonicalization contract. Normalizing
                // their surrounding whitespace here makes the new persisted
                // slot names canonical; blank/colliding labels still fail the
                // same candidate before commit.
                name: cue_point.label.trim().to_string(),
            })
            .collect()
    } else {
        state
            .cue_points_ms
            .iter()
            .enumerate()
            .map(|(index, position_ms)| VideoClipCuePointSummary {
                position_ms: *position_ms,
                name: format!("Cue {}", index + 1),
            })
            .collect()
    };
    validate_video_clip_cue_points(layer_id, in_point_ms, out_point_ms, &cue_points)?;

    Ok((
        in_point_ms,
        out_point_ms,
        loop_mode,
        state.speed,
        cue_points,
    ))
}

fn validate_video_clip_layer_runtime_summary(
    layer: &VideoClipLayerRuntimeSummary,
) -> Result<(), String> {
    if layer.active_slot_id.is_none()
        && (layer.playing || layer.playhead_ms != 0 || layer.ping_pong_reverse)
    {
        return Err(format!(
            "Video clip runtime layer {} has transport state without an active slot",
            layer.layer_id
        ));
    }

    for (role, slot_id) in [
        ("active", layer.active_slot_id),
        ("queued", layer.queued_slot_id),
    ] {
        if slot_id.is_some_and(|slot_id| slot_id.0 == 0) {
            return Err(format!(
                "Video clip runtime layer {} has a zero {role} slot ID",
                layer.layer_id
            ));
        }
    }

    if let Some(pending_launch) = &layer.pending_launch {
        match layer.queued_slot_id {
            Some(queued_slot_id) => {
                if pending_launch.slot_id.0 == 0 {
                    return Err(format!(
                        "Video clip runtime layer {} has a zero pending slot ID",
                        layer.layer_id
                    ));
                }
                if queued_slot_id != pending_launch.slot_id {
                    return Err(format!(
                        "Video clip runtime layer {} queued and pending slot IDs disagree",
                        layer.layer_id
                    ));
                }
                if matches!(
                    pending_launch.quantization,
                    VideoClipLaunchQuantization::Immediate
                ) {
                    return Err(format!(
                        "Video clip runtime layer {} cannot pend an immediate launch",
                        layer.layer_id
                    ));
                }
                if matches!(
                    pending_launch.quantization,
                    VideoClipLaunchQuantization::NextBar
                ) && pending_launch.target_boundary_ordinal % 4 != 0
                {
                    return Err(format!(
                        "Video clip runtime layer {} has a next-bar target that is not a 4-beat boundary",
                        layer.layer_id
                    ));
                }
                let invalid_transition =
                    if matches!(pending_launch.transition_kind, VideoClipTakeKind::Cut) {
                        pending_launch.duration.value_milliunits != 0
                            || pending_launch.resolved_duration_ms != 0
                    } else {
                        pending_launch.duration.value_milliunits == 0
                            || pending_launch.resolved_duration_ms == 0
                            || (matches!(
                                pending_launch.duration.unit,
                                VideoClipTakeDurationUnit::Milliseconds
                            ) && pending_launch.duration.value_milliunits
                                != pending_launch.resolved_duration_ms)
                    };
                if invalid_transition {
                    return Err(format!(
                        "Video clip runtime layer {} has invalid pending transition timing",
                        layer.layer_id
                    ));
                }
            }
            None => {
                return Err(format!(
                    "Video clip runtime layer {} has a pending launch without its queued slot",
                    layer.layer_id
                ));
            }
        }
    }
    if let Some(transition) = &layer.transition {
        if transition.outgoing_slot_id.0 == 0 || transition.incoming_slot_id.0 == 0 {
            return Err(format!(
                "Video clip runtime layer {} has a zero transition slot ID",
                layer.layer_id
            ));
        }
        if transition.origin_slot_id != transition.outgoing_slot_id
            && transition.origin_slot_id != transition.incoming_slot_id
        {
            return Err(format!(
                "Video clip runtime layer {} transition origin is not one of its live slots",
                layer.layer_id
            ));
        }
        if transition.outgoing_slot_id == transition.incoming_slot_id {
            return Err(format!(
                "Video clip runtime layer {} transition uses the same outgoing and incoming slot",
                layer.layer_id
            ));
        }
        if layer.active_slot_id != Some(transition.outgoing_slot_id)
            || layer.queued_slot_id != Some(transition.incoming_slot_id)
        {
            return Err(format!(
                "Video clip runtime layer {} transition disagrees with active/queued slots",
                layer.layer_id
            ));
        }
        if layer.pending_launch.is_some() {
            return Err(format!(
                "Video clip runtime layer {} cannot pend and transition simultaneously",
                layer.layer_id
            ));
        }
        if matches!(transition.kind, VideoClipTakeKind::Cut)
            || transition.duration_ms == 0
            || transition.elapsed_ms > transition.duration_ms
            || transition.progress_millis > 1000
            || (transition.duration.value_milliunits == 0
                && !matches!(
                    transition.duration.unit,
                    VideoClipTakeDurationUnit::Milliseconds
                ))
            || (transition.duration.value_milliunits > 0
                && matches!(
                    transition.duration.unit,
                    VideoClipTakeDurationUnit::Milliseconds
                )
                && transition.duration.value_milliunits != transition.duration_ms)
        {
            return Err(format!(
                "Video clip runtime layer {} has invalid transition timing",
                layer.layer_id
            ));
        }
        let expected_progress = transition
            .elapsed_ms
            .saturating_mul(1000)
            .saturating_div(transition.duration_ms)
            .min(1000) as u16;
        if transition.progress_millis != expected_progress {
            return Err(format!(
                "Video clip runtime layer {} transition progress {} does not match elapsed {}/{} (expected {})",
                layer.layer_id,
                transition.progress_millis,
                transition.elapsed_ms,
                transition.duration_ms,
                expected_progress
            ));
        }
    }
    Ok(())
}

fn validate_runtime_slot_reference<'a>(
    layer_id: VideoLayerId,
    role: &str,
    slot_id: Option<VideoClipSlotId>,
    slots: &'a BTreeMap<VideoClipSlotId, &'a VideoClipSlotSummary>,
) -> Result<Option<&'a VideoClipSlotSummary>, String> {
    match slot_id {
        Some(slot_id) => slots.get(&slot_id).copied().map(Some).ok_or_else(|| {
            format!(
                "Video clip runtime layer {layer_id} {role} slot {} is not in its authored slot bank",
                slot_id.0
            )
        }),
        None => Ok(None),
    }
}

fn validate_video_media_asset_catalog(
    video: &VideoSnapshot,
    require_layer_asset_references: bool,
) -> Result<(), String> {
    // Keep this first pass deliberately ID-only. It gives callers a stable
    // rejection for duplicate/zero asset IDs before a later malformed payload
    // distracts from the identity corruption.
    let mut asset_ids = BTreeMap::new();
    for asset in &video.media_assets {
        if asset.id == 0 {
            return Err("Media asset IDs must be non-zero".to_string());
        }
        if asset_ids.insert(asset.id, asset).is_some() {
            return Err(format!("Media asset ID {} is duplicated", asset.id));
        }
    }

    let mut layer_ids = BTreeMap::new();
    for layer in &video.layers {
        if layer.id == 0 {
            return Err("Video layer IDs must be non-zero".to_string());
        }
        if layer_ids.insert(layer.id, layer).is_some() {
            return Err(format!("Video layer ID {} is duplicated", layer.id));
        }
    }

    for asset in &video.media_assets {
        validate_media_asset_summary(asset)?;
    }

    for layer in &video.layers {
        validate_media_asset_source(&layer.source, &format!("video layer {}", layer.id))?;
        match layer.media_asset_id {
            Some(asset_id) => {
                let asset = asset_ids.get(&asset_id).ok_or_else(|| {
                    format!(
                        "Video layer {} references missing media asset {asset_id}",
                        layer.id
                    )
                })?;
                if layer.source != asset.source {
                    return Err(format!(
                        "Video layer {} source does not exactly mirror media asset {asset_id}",
                        layer.id
                    ));
                }
            }
            None if require_layer_asset_references => {
                return Err(format!(
                    "Video layer {} is missing its media asset reference",
                    layer.id
                ));
            }
            None => {}
        }
    }

    Ok(())
}

fn validate_video_clip_slots(
    video: &VideoSnapshot,
    require_slot_banks: bool,
) -> Result<(), String> {
    // Validate the catalog projection first, while preserving the legacy
    // allowance for a missing layer asset reference in non-engine-ready input.
    validate_video_media_asset_catalog(video, require_slot_banks)?;
    let asset_ids: BTreeMap<MediaAssetId, &MediaAssetSummary> = video
        .media_assets
        .iter()
        .map(|asset| (asset.id, asset))
        .collect();
    let mut slot_ids = BTreeMap::new();

    for layer in &video.layers {
        if layer.clip_slots.is_empty() {
            if layer.default_clip_slot_id.is_some() {
                return Err(format!(
                    "Video layer {} has a default clip slot but no clip slots",
                    layer.id
                ));
            }
            if require_slot_banks && layer.media_asset_id.is_some() {
                return Err(format!(
                    "Video layer {} has no authored clip slots",
                    layer.id
                ));
            }
            continue;
        }

        let default_slot_id = layer
            .default_clip_slot_id
            .ok_or_else(|| format!("Video layer {} is missing its default clip slot", layer.id))?;
        let mut default_slot = None;
        let mut override_identities = BTreeMap::new();

        for slot in &layer.clip_slots {
            if slot.id.0 == 0 {
                return Err("Video clip slot IDs must be non-zero".to_string());
            }
            if let Some(previous_layer_id) = slot_ids.insert(slot.id, layer.id) {
                return Err(format!(
                    "Video clip slot ID {} is duplicated by layers {} and {}",
                    slot.id.0, previous_layer_id, layer.id
                ));
            }
            if slot.media_asset_id == 0 || !asset_ids.contains_key(&slot.media_asset_id) {
                return Err(format!(
                    "Video clip slot {} on layer {} references missing media asset {}",
                    slot.id.0, layer.id, slot.media_asset_id
                ));
            }
            if let Some(out_point_ms) = slot.out_point_ms {
                if out_point_ms <= slot.in_point_ms {
                    return Err(format!(
                        "Video clip slot {} on layer {} has an invalid half-open range",
                        slot.id.0, layer.id
                    ));
                }
            }
            if !slot.speed.is_finite() || !(-4.0..=4.0).contains(&slot.speed) {
                return Err(format!(
                    "Video clip slot {} on layer {} has speed outside finite -4.0..=4.0",
                    slot.id.0, layer.id
                ));
            }

            validate_video_clip_cue_points(
                layer.id,
                slot.in_point_ms,
                slot.out_point_ms,
                &slot.cue_points,
            )?;

            override_identities.clear();
            for effect_override in &slot.effect_overrides {
                let control_name = effect_override.control_name.as_str();
                if control_name.is_empty()
                    || control_name.trim() != control_name
                    || override_identities
                        .insert((effect_override.stage_index, control_name), ())
                        .is_some()
                    || !effect_override.value.iter().all(|value| value.is_finite())
                {
                    return Err(format!(
                        "Video clip slot {} on layer {} has an invalid effect override",
                        slot.id.0, layer.id
                    ));
                }
                if !layer_has_isf_control(layer, effect_override.stage_index, control_name) {
                    return Err(format!(
                        "Video clip slot {} on layer {} overrides missing ISF stage {} control {control_name:?}",
                        slot.id.0, layer.id, effect_override.stage_index
                    ));
                }
            }
            if slot.id == default_slot_id {
                default_slot = Some(slot);
            }
        }

        let default_slot = default_slot.ok_or_else(|| {
            format!(
                "Video layer {} default clip slot {} is not in its slot bank",
                layer.id, default_slot_id.0
            )
        })?;
        if layer.media_asset_id != Some(default_slot.media_asset_id) {
            return Err(format!(
                "Video layer {} media asset projection does not match its default clip slot",
                layer.id
            ));
        }
    }
    Ok(())
}

fn validate_video_clip_cue_points(
    layer_id: VideoLayerId,
    in_point_ms: u64,
    out_point_ms: Option<u64>,
    cue_points: &[VideoClipCuePointSummary],
) -> Result<(), String> {
    let mut cue_names = BTreeMap::new();
    let mut cue_positions = BTreeMap::new();
    for cue_point in cue_points {
        let name = cue_point.name.as_str();
        if name.is_empty() || name.trim() != name {
            return Err(format!(
                "Video layer {layer_id} has an unnamed or non-canonical clip cue point"
            ));
        }
        if cue_names.insert(name, ()).is_some() {
            return Err(format!(
                "Video layer {layer_id} has duplicate clip cue point name {name:?}"
            ));
        }
        if cue_positions.insert(cue_point.position_ms, ()).is_some()
            || cue_point.position_ms < in_point_ms
            || out_point_ms.is_some_and(|out| cue_point.position_ms >= out)
        {
            return Err(format!(
                "Video layer {layer_id} has an invalid clip cue point"
            ));
        }
    }
    Ok(())
}

fn layer_has_isf_control(
    layer: &VideoLayerSummary,
    stage_index: usize,
    control_name: &str,
) -> bool {
    let Some(effect) = layer.isf_effect.as_ref() else {
        return false;
    };
    let controls = if stage_index == 0 {
        &effect.controls
    } else if let Some(stage) = effect.stack.get(stage_index - 1) {
        &stage.controls
    } else {
        return false;
    };
    controls.iter().any(|control| control.name == control_name)
}

fn validate_media_asset_summary(asset: &MediaAssetSummary) -> Result<(), String> {
    if asset.label.trim().is_empty() {
        return Err(format!("Media asset {} has an empty label", asset.id));
    }
    validate_media_asset_source(&asset.source, &format!("media asset {}", asset.id))?;

    let local = media_asset_source_is_local(&asset.source);
    match (&asset.content_hash, asset.byte_size) {
        (None, None) => {}
        (Some(hash), Some(byte_size)) if local => {
            validate_media_content_hash(hash, asset.id)?;
            if byte_size == 0 {
                return Err(format!("Media asset {} has a zero byte size", asset.id));
            }
        }
        (Some(_), Some(_)) => {
            return Err(format!(
                "Live media asset {} must not persist a content hash or byte size",
                asset.id
            ));
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(format!(
                "Media asset {} must persist content hash and byte size together",
                asset.id
            ));
        }
    }
    Ok(())
}

fn validate_media_content_hash(
    hash: &MediaContentHash,
    asset_id: MediaAssetId,
) -> Result<(), String> {
    match hash.algorithm {
        MediaHashAlgorithm::Sha256 => {}
    }
    if hash.hex.len() != 64
        || !hash
            .hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "Media asset {asset_id} SHA-256 hash must be exactly 64 lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

fn media_asset_source_is_local(source: &VideoSourceSummary) -> bool {
    matches!(
        source.kind,
        VideoSourceKind::File | VideoSourceKind::StillImage
    )
}

fn validate_media_asset_source(source: &VideoSourceSummary, owner: &str) -> Result<(), String> {
    if source
        .codec
        .as_deref()
        .is_some_and(|codec| codec.trim().is_empty())
    {
        return Err(format!("{owner} has an empty codec"));
    }

    if media_asset_source_is_local(source) {
        if source.path.as_deref().unwrap_or_default().trim().is_empty() {
            return Err(format!("{owner} requires a local source path"));
        }
    } else if source.name.as_deref().unwrap_or_default().trim().is_empty() {
        return Err(format!("{owner} requires a live source name"));
    }

    if let Some(metadata) = source.metadata {
        if metadata.width.is_some_and(|width| width == 0) {
            return Err(format!("{owner} has invalid metadata width"));
        }
        if metadata.height.is_some_and(|height| height == 0) {
            return Err(format!("{owner} has invalid metadata height"));
        }
        if metadata
            .frame_rate
            .is_some_and(|frame_rate| !frame_rate.is_finite() || frame_rate <= 0.0)
        {
            return Err(format!("{owner} has invalid metadata frame rate"));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecallMode {
    #[default]
    Coexist,
    ReplaceGroup,
}

/// T20 live playback direction. `Authored` preserves every Effect's own
/// direction and the Cue Step order, so legacy shows retain their exact
/// behavior until the operator explicitly chooses a live direction.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum CueLiveDirection {
    #[default]
    Authored,
    Forward,
    Reverse,
    Bounce,
}

impl CueLiveDirection {
    pub fn is_authored(&self) -> bool {
        matches!(self, Self::Authored)
    }
}

/// T17/T20 authored starting position for the per-scene live modifier.
/// `segment` is one-based (`0` = follow the normal timeline, `1..=N` = jump
/// to that authored Cue Step). Stored on the Cue; the latched live override
/// itself is runtime-only and never serialized.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CueLiveModifierSettings {
    #[serde(default = "default_live_modifier_scale")]
    pub speed: f32,
    #[serde(default = "default_live_modifier_scale")]
    pub size: f32,
    #[serde(default)]
    pub phase: f32,
    #[serde(default, skip_serializing_if = "CueLiveDirection::is_authored")]
    pub direction: CueLiveDirection,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub segment: u16,
    #[serde(default)]
    pub flash: bool,
}

fn is_zero_u16(value: &u16) -> bool {
    *value == 0
}

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

fn is_zero_u8(value: &u8) -> bool {
    *value == 0
}

fn is_zero_f32(value: &f32) -> bool {
    *value == 0.0
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_standard_sparkle_raster_mode(value: &ColorEffectSpatialSparkleRasterMode) -> bool {
    *value == ColorEffectSpatialSparkleRasterMode::Sparkle
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

fn default_live_modifier_scale() -> f32 {
    1.0
}

impl Default for CueLiveModifierSettings {
    fn default() -> Self {
        Self {
            speed: 1.0,
            size: 1.0,
            phase: 0.0,
            direction: CueLiveDirection::Authored,
            segment: 0,
            flash: false,
        }
    }
}

impl CueLiveModifierSettings {
    pub fn is_neutral(&self) -> bool {
        self.speed == 1.0
            && self.size == 1.0
            && self.phase == 0.0
            && self.direction.is_authored()
            && self.segment == 0
            && !self.flash
    }
}

/// Runtime-only latched live override state for one active scene. Exposed in
/// the snapshot for UI display but stripped before any `.sdc` write.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CueLiveModifierState {
    pub cue_id: CueId,
    pub speed: f32,
    pub size: f32,
    pub phase: f32,
    pub direction: CueLiveDirection,
    pub segment: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CueSummary {
    pub id: CueId,
    #[serde(default = "default_cue_list_id")]
    pub cue_list_id: CueListId,
    #[serde(default)]
    pub cue_number: String,
    pub label: String,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub recall_mode: RecallMode,
    pub fade_ms: u64,
    #[serde(default)]
    pub authored_beats: Option<f32>,
    #[serde(default)]
    pub pre_wait_ms: u64,
    #[serde(default)]
    pub follow_ms: Option<u64>,
    #[serde(default)]
    pub ifcb_timing: CueIfcbTiming,
    #[serde(default)]
    pub parts: Vec<CuePartSummary>,
    #[serde(default)]
    pub mark: bool,
    #[serde(default)]
    pub mib_fixture_ids: Vec<FixtureId>,
    #[serde(default)]
    pub palette_targets: Vec<CuePaletteTarget>,
    #[serde(default = "default_cue_tracking")]
    pub tracking: bool,
    #[serde(default)]
    pub notes: String,
    pub targets: Vec<CueFixtureTarget>,
    pub video_targets: Vec<VideoLayerTarget>,
    #[serde(default)]
    pub video_output_targets: Vec<VideoOutputTarget>,
    #[serde(default)]
    pub node_graph_targets: Vec<CueNodeGraphTarget>,
    #[serde(default)]
    pub effect_targets: Vec<CueEffectTarget>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<CueStepSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_timeline: Option<ChildTimelineSummary>,
    /// T7 persistent identity color (`#rrggbb`). `None` keeps the deterministic
    /// hash-derived hue; skipped when absent so legacy cues stay byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// T17 authored live-modifier defaults. `None` keeps legacy cues
    /// byte-identical and means the neutral position (1.0/1.0/0.0, no flash).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_modifiers: Option<CueLiveModifierSettings>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CueIfcbTiming {
    #[serde(default)]
    pub intensity_fade_ms: Option<u64>,
    #[serde(default)]
    pub intensity_delay_ms: u64,
    #[serde(default)]
    pub focus_fade_ms: Option<u64>,
    #[serde(default)]
    pub focus_delay_ms: u64,
    #[serde(default)]
    pub color_fade_ms: Option<u64>,
    #[serde(default)]
    pub color_delay_ms: u64,
    #[serde(default)]
    pub beam_fade_ms: Option<u64>,
    #[serde(default)]
    pub beam_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CuePartSummary {
    pub number: u16,
    pub label: String,
    #[serde(default)]
    pub delay_ms: u64,
    #[serde(default)]
    pub fade_ms: Option<u64>,
    #[serde(default)]
    pub fixture_ids: Vec<FixtureId>,
    #[serde(default)]
    pub video_layer_ids: Vec<VideoLayerId>,
    #[serde(default)]
    pub video_output_ids: Vec<VideoOutputId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaletteKind {
    Intensity,
    Position,
    Color,
    Beam,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReferencePaletteSummary {
    pub id: PaletteId,
    pub label: String,
    pub kind: PaletteKind,
    pub values: Vec<AttributeValueSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub color_stops: Vec<ColorEffectStop>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CuePaletteTarget {
    pub palette_id: PaletteId,
    pub fixture_ids: Vec<FixtureId>,
}

fn default_cue_tracking() -> bool {
    true
}

pub const DEFAULT_CUE_LIST_ID: CueListId = 1;

fn default_cue_list_id() -> CueListId {
    DEFAULT_CUE_LIST_ID
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CueListSummary {
    pub id: CueListId,
    pub label: String,
    #[serde(default)]
    pub active_cue_id: Option<CueId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaybackExecutorSummary {
    pub id: ExecutorId,
    pub label: String,
    pub cue_list_id: CueListId,
    pub page: u16,
    pub slot: u16,
    pub level: f32,
}

impl Default for PlaybackExecutorSummary {
    fn default() -> Self {
        Self {
            id: 1,
            label: "Bank 1".to_string(),
            cue_list_id: DEFAULT_CUE_LIST_ID,
            page: 1,
            slot: 1,
            level: 1.0,
        }
    }
}

fn default_playback_executors() -> Vec<PlaybackExecutorSummary> {
    vec![PlaybackExecutorSummary::default()]
}

fn default_level() -> f32 {
    1.0
}

impl Default for CueListSummary {
    fn default() -> Self {
        Self {
            id: DEFAULT_CUE_LIST_ID,
            label: "Bank 1".to_string(),
            active_cue_id: None,
        }
    }
}

impl Default for CueSummary {
    fn default() -> Self {
        Self {
            id: 0,
            cue_list_id: DEFAULT_CUE_LIST_ID,
            cue_number: String::new(),
            label: String::new(),
            group_id: None,
            recall_mode: RecallMode::default(),
            fade_ms: 0,
            authored_beats: None,
            pre_wait_ms: 0,
            follow_ms: None,
            ifcb_timing: CueIfcbTiming::default(),
            parts: Vec::new(),
            mark: false,
            mib_fixture_ids: Vec::new(),
            palette_targets: Vec::new(),
            tracking: true,
            notes: String::new(),
            targets: Vec::new(),
            video_targets: Vec::new(),
            video_output_targets: Vec::new(),
            node_graph_targets: Vec::new(),
            effect_targets: Vec::new(),
            steps: Vec::new(),
            child_timeline: None,
            color: None,
            live_modifiers: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CueNodeGraphTarget {
    pub graph_id: NodeGraphId,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveFadeSummary {
    pub cue_id: CueId,
    pub progress: f32,
    pub remaining_ms: u64,
    pub paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProgrammerValueSummary {
    pub fixture_id: FixtureId,
    pub attribute: String,
    pub value: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProgrammerSnapshot {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub blind: bool,
    #[serde(default)]
    pub values: Vec<ProgrammerValueSummary>,
    #[serde(default)]
    pub dmx_previews: Vec<DmxUniversePreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimelineTrackKind {
    Lighting,
    Video,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimelineLayerKind {
    #[default]
    Lighting,
    Video,
    Audio,
}

impl TimelineLayerKind {
    pub const fn display_section_rank(self) -> u8 {
        match self {
            Self::Audio => 0,
            Self::Lighting => 1,
            Self::Video => 2,
        }
    }
}

impl From<&TimelineTrackKind> for TimelineLayerKind {
    fn from(track: &TimelineTrackKind) -> Self {
        match track {
            TimelineTrackKind::Lighting => Self::Lighting,
            TimelineTrackKind::Video => Self::Video,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineLayerSummary {
    pub id: u32,
    pub label: String,
    pub order: u32,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub expanded: bool,
    #[serde(default)]
    pub kind: TimelineLayerKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimelinePhaseRole {
    Intro,
    Verse,
    PreChorus,
    Chorus,
    Interlude,
    Bridge,
    Breakdown,
    Outro,
    #[default]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelinePhaseSummary {
    pub id: TimelinePhaseId,
    pub label: String,
    #[serde(default)]
    pub role: TimelinePhaseRole,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineVideoClipSummary {
    pub id: TimelineVideoClipId,
    /// Authored Timeline Video-lane identity.  This is deliberately not a
    /// `VideoLayerId`; the engine creates a renderer-only projection for an
    /// active MediaAsset-backed clip when no authored VJ layer exists.
    pub layer_id: u32,
    pub media_asset_id: MediaAssetId,
    pub start_ms: u64,
    #[serde(default)]
    pub offset_ms: u64,
    pub duration_ms: u64,
    #[serde(default)]
    pub fade_in_ms: u64,
    #[serde(default)]
    pub fade_out_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineItemRef {
    LightingEvent { event_id: TimelineEventId },
    VideoClip { clip_id: TimelineVideoClipId },
    AudioClip { clip_id: TimelineAudioClipId },
    LightingAutomation { automation_id: AutomationId },
    VideoAutomation { automation_id: AutomationId },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineItemGroupSummary {
    pub id: TimelineItemGroupId,
    #[serde(default)]
    pub members: Vec<TimelineItemRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineLoopRegionSummary {
    pub a_ms: u64,
    pub b_ms: u64,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub musical_length_beats: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimelineFollowLightingPolicy {
    #[default]
    HoldThenCut,
    LinearMerge,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimelineFollowFaultPolicy {
    #[default]
    Hold,
    Cut,
    Fault,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineFollowSummary {
    #[serde(default)]
    pub enabled: bool,
    pub next_timeline_id: TimelineId,
    #[serde(default)]
    pub duration: VideoClipTakeDuration,
    #[serde(default)]
    pub curve: VideoLayerTransitionCurve,
    #[serde(default)]
    pub video_kind: VideoClipTakeKind,
    #[serde(default)]
    pub lighting_policy: TimelineFollowLightingPolicy,
    #[serde(default)]
    pub destination_bpm: Option<f64>,
    #[serde(default)]
    pub preroll_ms: u64,
    #[serde(default = "default_timeline_follow_trans_cadence_bars")]
    pub trans_cadence_bars: u16,
    /// Exact show-authored Trans target measures. Empty preserves the additive
    /// legacy behavior (derive targets from `trans_cadence_bars`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trans_target_measures: Vec<u64>,
    /// Hold a successfully settled destination in one runtime-only first
    /// measure loop. Legacy authored Follow data remains false.
    #[serde(default)]
    pub hold_first_destination_measure: bool,
    #[serde(default)]
    pub fault_policy: TimelineFollowFaultPolicy,
}

const fn default_timeline_follow_trans_cadence_bars() -> u16 {
    4
}

/// Logical destination for authored Timeline Audio Clips.
///
/// This is deliberately project-portable: it identifies the mix bus only and
/// must never be used to persist a physical output device or channel.  The
/// default is the one-way legacy migration for clips written before this
/// field existed.  Serde intentionally has no catch-all variant, so an
/// explicit unknown or future value is rejected rather than silently routed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimelineAudioOutputBus {
    #[default]
    Program,
    Cue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineAudioClipSummary {
    #[serde(default)]
    pub id: TimelineAudioClipId,
    #[serde(default)]
    pub layer_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_asset_id: Option<MediaAssetId>,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub start_ms: u64,
    #[serde(default)]
    pub offset_ms: u64,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default = "default_timeline_audio_clip_gain")]
    pub gain: f32,
    #[serde(default)]
    pub fade_in_ms: u64,
    #[serde(default)]
    pub fade_out_ms: u64,
    /// Logical `PROGRAM` (stereo audience/broadcast) or `CUE` (performer
    /// cue) destination. Missing legacy data is canonically PROGRAM.
    #[serde(default)]
    pub output_bus: TimelineAudioOutputBus,
}

/// The interpolation used between two authored Timeline tempo points.  The
/// default is deliberately step/hold so a legacy or partially authored map
/// cannot introduce an implicit tempo slew.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TimelineTempoInterpolation {
    #[default]
    Step,
    Linear,
}

/// One exact Timeline tempo/meter change point.  The authored transport
/// coordinate is an integer sixteenth-note grid (`1/16` note per unit), so a
/// `7/8` measure is exactly 14 units and can be followed by a `3/8` measure
/// without a fractional or floating-point persistence coordinate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineTempoMeterPoint {
    #[serde(default)]
    pub position_sixteenth_steps: u64,
    #[serde(default = "default_timeline_tempo_bpm")]
    pub bpm: f64,
    #[serde(
        default = "default_timeline_meter_numerator",
        alias = "meter_numerator"
    )]
    pub numerator: u8,
    #[serde(
        default = "default_timeline_meter_denominator",
        alias = "meter_denominator"
    )]
    pub denominator: u8,
    #[serde(default)]
    pub interpolation: TimelineTempoInterpolation,
    /// Optional display/diagnostic measure number.  It is not used to derive
    /// timing, but lets imported maps preserve a source measure identity such
    /// as the `惑う星` 113..128 range without inventing another clock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure_number: Option<u64>,
}

impl Default for TimelineTempoMeterPoint {
    fn default() -> Self {
        Self {
            position_sixteenth_steps: 0,
            bpm: default_timeline_tempo_bpm(),
            numerator: default_timeline_meter_numerator(),
            denominator: default_timeline_meter_denominator(),
            interpolation: TimelineTempoInterpolation::default(),
            measure_number: None,
        }
    }
}

pub type TimelineTempoPoint = TimelineTempoMeterPoint;
pub type TimelineMeterPoint = TimelineTempoMeterPoint;

pub const TIMELINE_TEMPO_METER_MAP_VERSION: u8 = 1;
pub const TIMELINE_TEMPO_METER_MAX_POINTS: usize = 4096;
pub const TIMELINE_TEMPO_METER_MAX_SIXTEENTH_STEPS: u64 = u64::MAX / 4;
pub const TIMELINE_TEMPO_METER_MAX_MEASURE: u64 = u64::MAX / 16;
pub const TIMELINE_TEMPO_MIN_BPM: f64 = 20.0;
pub const TIMELINE_TEMPO_MAX_BPM: f64 = 300.0;

const fn default_timeline_tempo_meter_map_version() -> u8 {
    TIMELINE_TEMPO_METER_MAP_VERSION
}

fn is_default_timeline_tempo_meter_map_version(version: &u8) -> bool {
    *version == TIMELINE_TEMPO_METER_MAP_VERSION
}

const fn default_timeline_tempo_bpm() -> f64 {
    120.0
}

const fn default_timeline_meter_numerator() -> u8 {
    4
}

const fn default_timeline_meter_denominator() -> u8 {
    4
}

/// Validate a Timeline tempo/meter map without consulting runtime state.  An
/// empty map is valid and means "use the current engine BPM and 4/4".
pub fn validate_timeline_tempo_meter_map(
    version: u8,
    points: &[TimelineTempoMeterPoint],
) -> Result<(), String> {
    if version > TIMELINE_TEMPO_METER_MAP_VERSION {
        return Err(format!(
            "Timeline tempo/meter map version {version} is newer than supported version {TIMELINE_TEMPO_METER_MAP_VERSION}"
        ));
    }
    if version == 0 && !points.is_empty() {
        return Err("Timeline tempo/meter map version 0 cannot carry authored points".to_string());
    }
    if points.len() > TIMELINE_TEMPO_METER_MAX_POINTS {
        return Err(format!(
            "Timeline tempo/meter map contains {} points; the limit is {TIMELINE_TEMPO_METER_MAX_POINTS}",
            points.len()
        ));
    }
    let mut previous_position = None;
    let mut previous_measure_number = None;
    let mut current_measure_number = 1_u64;
    let mut last_measure_boundary = 0_u64;
    let mut current_numerator = 4_u8;
    let mut current_denominator = 4_u8;
    for (index, point) in points.iter().enumerate() {
        if point.position_sixteenth_steps > TIMELINE_TEMPO_METER_MAX_SIXTEENTH_STEPS {
            return Err(format!(
                "Timeline tempo/meter point {index} exceeds the checked sixteenth-step range"
            ));
        }
        if let Some(previous) = previous_position {
            if point.position_sixteenth_steps == previous {
                return Err(format!(
                    "Timeline tempo/meter map contains duplicate point position {}",
                    point.position_sixteenth_steps
                ));
            }
            if point.position_sixteenth_steps < previous {
                return Err("Timeline tempo/meter points must be sorted by position".to_string());
            }
        }
        previous_position = Some(point.position_sixteenth_steps);
        if !point.bpm.is_finite()
            || !(TIMELINE_TEMPO_MIN_BPM..=TIMELINE_TEMPO_MAX_BPM).contains(&point.bpm)
        {
            return Err(format!(
                "Timeline tempo/meter point {index} BPM must be finite and within {TIMELINE_TEMPO_MIN_BPM}..={TIMELINE_TEMPO_MAX_BPM}"
            ));
        }
        if !(1..=16).contains(&point.numerator) {
            return Err(format!(
                "Timeline tempo/meter point {index} numerator must be within 1..=16"
            ));
        }
        if !matches!(point.denominator, 1 | 2 | 4 | 8 | 16) {
            return Err(format!(
                "Timeline tempo/meter point {index} denominator must be a supported power of two"
            ));
        }
        if point
            .measure_number
            .is_some_and(|measure| measure == 0 || measure > TIMELINE_TEMPO_METER_MAX_MEASURE)
        {
            return Err(format!(
                "Timeline tempo/meter point {index} measure number is outside the checked range"
            ));
        }
        let measure_steps = u64::from(current_numerator)
            .checked_mul(16)
            .and_then(|value| value.checked_div(u64::from(current_denominator)))
            .ok_or_else(|| format!("Timeline tempo/meter point {index} measure span overflowed"))?;
        let delta = point
            .position_sixteenth_steps
            .checked_sub(last_measure_boundary)
            .ok_or_else(|| {
                format!("Timeline tempo/meter point {index} precedes its measure boundary")
            })?;
        let at_measure_boundary = delta % measure_steps == 0;
        let meter_changes =
            (point.numerator, point.denominator) != (current_numerator, current_denominator);
        if (meter_changes || point.measure_number.is_some()) && !at_measure_boundary {
            return Err(format!(
                "Timeline tempo/meter point {index} changes meter/measure number away from an exact measure boundary"
            ));
        }
        if at_measure_boundary {
            let elapsed_measures = delta / measure_steps;
            let expected_measure = current_measure_number
                .checked_add(elapsed_measures)
                .ok_or_else(|| {
                    format!("Timeline tempo/meter point {index} measure number overflowed")
                })?;
            if let Some(measure) = point.measure_number {
                if let Some(previous_measure) = previous_measure_number {
                    if measure <= previous_measure {
                        return Err(format!(
                            "Timeline tempo/meter measure numbers must increase strictly at point {index}"
                        ));
                    }
                }
                if index > 0 && measure != expected_measure {
                    return Err(format!(
                        "Timeline tempo/meter point {index} measure number {measure} does not match expected boundary {expected_measure}"
                    ));
                }
                current_measure_number = measure;
                previous_measure_number = Some(measure);
            } else {
                current_measure_number = expected_measure;
            }
            last_measure_boundary = point.position_sixteenth_steps;
        } else if point.measure_number.is_some() {
            return Err(format!(
                "Timeline tempo/meter point {index} has an ambiguous measure number"
            ));
        }
        if meter_changes {
            current_numerator = point.numerator;
            current_denominator = point.denominator;
            last_measure_boundary = point.position_sixteenth_steps;
        }
        if matches!(point.interpolation, TimelineTempoInterpolation::Linear)
            && points.get(index + 1).is_none()
        {
            return Err(
                "The final Timeline tempo/meter point cannot request a linear slew without an end point"
                    .to_string(),
            );
        }
        // The checked cadence multiplication is part of validation rather
        // than a later scheduler assertion, so a malformed map cannot mutate
        // an already-running project before failing.
        u64::from(point.numerator)
            .checked_mul(16 / u64::from(point.denominator))
            .ok_or_else(|| format!("Timeline tempo/meter point {index} cadence overflowed"))?;
    }
    Ok(())
}

/// Validate only the authored tempo/meter authority carried by a child
/// Timeline.  Child event/cue graph validation remains engine-owned because it
/// needs the project Cue catalog; keeping this function pure lets project-load
/// preflight reject a malformed map before any runtime child transport exists.
pub fn validate_child_timeline_tempo_meter_map(child: &ChildTimelineSummary) -> Result<(), String> {
    validate_timeline_tempo_meter_map(child.tempo_meter_map_version, &child.tempo_meter_map)
}

/// Source identity for the engine-owned metronome/Guide output clock.  A
/// source transition is an ABA fence even when the visible sample frame is
/// unchanged, so a queued Root click can never be reused for a Direct child.
/// A Follow keeps the source Root authority through settlement and installs
/// its destination as a checked new Root generation only after success.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineScheduleSource {
    #[default]
    Root,
    DirectChild {
        cue_id: CueId,
        generation: u64,
    },
}

/// Runtime-only click metadata published with a Timeline audio projection.
/// The native bus can schedule the event without re-deriving musical timing;
/// all identity fields are an ABA fence for queued output frames.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineClickEventSummary {
    pub sample_frame: u64,
    pub measure: u64,
    pub beat: u16,
    pub numerator: u8,
    pub denominator: u8,
    pub downbeat: bool,
    pub frequency_hz: u16,
    pub duration_frames: u32,
    pub epoch: u64,
    pub transport_generation: u64,
    pub schedule_generation: u64,
    #[serde(default)]
    pub source: TimelineScheduleSource,
    #[serde(default)]
    pub count_in: bool,
}

impl Default for TimelineAudioClipSummary {
    fn default() -> Self {
        Self {
            id: 0,
            layer_id: 0,
            media_asset_id: None,
            path: String::new(),
            start_ms: 0,
            offset_ms: 0,
            duration_ms: 0,
            gain: default_timeline_audio_clip_gain(),
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: TimelineAudioOutputBus::Program,
        }
    }
}

fn default_timeline_audio_clip_gain() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineCueEventSummary {
    pub id: TimelineEventId,
    pub cue_id: CueId,
    pub time_ms: u64,
    #[serde(default)]
    pub time_beats: Option<f64>,
    pub track: TimelineTrackKind,
    #[serde(default)]
    pub layer_id: Option<u32>,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub duration_beats: Option<f64>,
    #[serde(default)]
    pub conform_to_tempo: bool,
    #[serde(default)]
    pub loop_fill: bool,
    /// Signed source position at the block start. Positive values trim into the
    /// source; negative values preserve Daslight pre-roll before source time 0.
    #[serde(default)]
    pub source_offset_ms: i64,
    #[serde(default)]
    pub rate: Option<f32>,
    #[serde(default)]
    pub fade_in_ms: u64,
    #[serde(default)]
    pub fade_out_ms: u64,
    #[serde(default = "default_timeline_scene_block_loop_count")]
    pub loop_count: u16,
    #[serde(default)]
    pub jump_to_event_id: Option<TimelineEventId>,
}

impl Default for TimelineCueEventSummary {
    fn default() -> Self {
        Self {
            id: 0,
            cue_id: 0,
            time_ms: 0,
            time_beats: None,
            track: TimelineTrackKind::Lighting,
            layer_id: None,
            duration_ms: 0,
            duration_beats: None,
            conform_to_tempo: false,
            loop_fill: false,
            source_offset_ms: 0,
            rate: None,
            fade_in_ms: 0,
            fade_out_ms: 0,
            loop_count: default_timeline_scene_block_loop_count(),
            jump_to_event_id: None,
        }
    }
}

fn default_timeline_scene_block_loop_count() -> u16 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutomationInterpolation {
    Step,
    Linear,
    Bezier,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutomationKeyframeSummary {
    pub time_ms: u64,
    pub value: u16,
    pub interpolation: AutomationInterpolation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineAutomationSummary {
    pub id: AutomationId,
    pub fixture_id: FixtureId,
    pub attribute: String,
    pub track: TimelineTrackKind,
    /// Authored Timeline-lane placement. Legacy snapshots omit this and are
    /// displayed in the first Lighting lane without rewriting the project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_layer_id: Option<u32>,
    pub keyframes: Vec<AutomationKeyframeSummary>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoParam {
    Opacity,
    Speed,
    PositionMs,
    BpmSyncEnabled,
    BpmSyncRatio,
    BpmSyncLoopBars,
    TransformX,
    TransformY,
    TransformScaleX,
    TransformScaleY,
    TransformRotationDeg,
    TransformCropLeft,
    TransformCropTop,
    TransformCropRight,
    TransformCropBottom,
    ColorBrightness,
    ColorContrast,
    ColorHueDeg,
    ColorSaturation,
    ColorGamma,
    FxPixelate,
    FxBlur,
    FxGlow,
    FxEdge,
    FxKeyRed,
    FxKeyGreen,
    FxKeyBlue,
    FxKeyThreshold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoAutomationKeyframeSummary {
    pub time_ms: u64,
    pub value: f32,
    pub interpolation: AutomationInterpolation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineVideoAutomationSummary {
    pub id: AutomationId,
    /// Target video-composition layer; this is not a Timeline lane identity.
    pub layer_id: VideoLayerId,
    /// Authored Timeline-lane placement. Kept separate from `layer_id` so a
    /// lane reorder cannot retarget the automated video layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_layer_id: Option<u32>,
    pub param: VideoParam,
    pub track: TimelineTrackKind,
    pub keyframes: Vec<VideoAutomationKeyframeSummary>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineEventPlacementUpdate {
    pub event_id: TimelineEventId,
    pub cue_id: CueId,
    pub time_ms: u64,
    #[serde(default)]
    pub time_beats: Option<f64>,
    pub track: TimelineTrackKind,
    #[serde(default)]
    pub layer_id: Option<u32>,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub duration_beats: Option<f64>,
    #[serde(default)]
    pub conform_to_tempo: bool,
    #[serde(default)]
    pub loop_fill: bool,
    #[serde(default)]
    pub fade_in_ms: u64,
    #[serde(default)]
    pub fade_out_ms: u64,
    #[serde(default = "default_timeline_scene_block_loop_count")]
    pub loop_count: u16,
    #[serde(default)]
    pub jump_to_event_id: Option<TimelineEventId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineAutomationKeyframesUpdate {
    pub automation_id: AutomationId,
    pub keyframes: Vec<AutomationKeyframeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineVideoAutomationKeyframesUpdate {
    pub automation_id: AutomationId,
    pub keyframes: Vec<VideoAutomationKeyframeSummary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TimelineSnapRequest {
    #[serde(default)]
    pub event_placements: Vec<TimelineEventPlacementUpdate>,
    #[serde(default)]
    pub lighting_automations: Vec<TimelineAutomationKeyframesUpdate>,
    #[serde(default)]
    pub video_automations: Vec<TimelineVideoAutomationKeyframesUpdate>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineLoopRuntimeStatus {
    #[default]
    Disabled,
    Armed,
    Looping,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineLoopScale {
    Half,
    Double,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineLoopRuntimeSummary {
    pub generation: u64,
    pub status: TimelineLoopRuntimeStatus,
    #[serde(default)]
    pub a_ms: Option<u64>,
    #[serde(default)]
    pub b_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub musical_length_millibeats: Option<u64>,
    #[serde(default)]
    pub wrap_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineGuideCueKind {
    Phase {
        phase_id: TimelinePhaseId,
    },
    Looping,
    Break,
    Trans,
    /// Emitted exactly once when an admitted Follow reaches successful
    /// terminal settlement.  Visual end, stale, abort and fault paths never
    /// use this cue.
    Complete,
}

/// Exhaustive offline Guide voice catalog. Keeping this semantic key in the
/// runtime DTO prevents native playback from guessing an asset from localized
/// or user-authored labels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineGuideAssetKey {
    Intro,
    Verse,
    PreChorus,
    Chorus,
    Interlude,
    Bridge,
    Breakdown,
    Outro,
    Looping,
    Break,
    Trans,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineGuideCueSummary {
    pub generation: u64,
    pub sequence: u64,
    pub at_ms: u64,
    pub label: String,
    pub cue: TimelineGuideCueKind,
    pub asset: TimelineGuideAssetKey,
    /// Boundary-captured natural playback rate. Native playback intentionally
    /// changes pitch with speed; an already-started word is never retimed.
    #[serde(default = "default_timeline_guide_playback_rate_milli")]
    pub playback_rate_milli: u16,
    /// Exact native-bus coordinate derived from the current tempo/meter
    /// authority. `at_ms` remains diagnostic/UI history only.
    #[serde(default)]
    pub sample_frame: u64,
    #[serde(default)]
    pub epoch: u64,
    #[serde(default)]
    pub transport_generation: u64,
    #[serde(default)]
    pub schedule_generation: u64,
    #[serde(default)]
    pub source: TimelineScheduleSource,
}

const fn default_timeline_guide_playback_rate_milli() -> u16 {
    1_000
}

/// One authoritative authored image for the advanced Timeline surfaces. The
/// established Scene Block, automation and lane editors keep their existing
/// commands; media placement, Phase, grouping, A-B and Follow commit together
/// through this image so linked A/V items can never tear across publication.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TimelineAdvancedAuthoringSummary {
    /// Optional complete ordered lane image. `None` preserves compatibility
    /// with older clients that only edit media/items; `Some` is applied in the
    /// same authoritative transaction as the linked media fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<TimelineLayerSummary>>,
    /// Optional atomic movement of established Scene Blocks and automation
    /// keyframes. Group moves use this same publication as media clips.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snap_request: Option<TimelineSnapRequest>,
    #[serde(default)]
    pub video_clips: Vec<TimelineVideoClipSummary>,
    #[serde(default)]
    pub audio_clips: Vec<TimelineAudioClipSummary>,
    #[serde(default)]
    pub phases: Vec<TimelinePhaseSummary>,
    #[serde(default)]
    pub item_groups: Vec<TimelineItemGroupSummary>,
    #[serde(default)]
    pub loop_region: Option<TimelineLoopRegionSummary>,
    #[serde(default)]
    pub follow: Option<TimelineFollowSummary>,
    #[serde(default)]
    pub guide_enabled: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineFollowRuntimeStatus {
    #[default]
    Idle,
    /// Legacy runtime spelling retained until the engine publisher has moved
    /// every caller to `Armed`. New publishers must emit `Armed` instead.
    Pending,
    /// A Follow is eligible only from the captured normal-playback boundary;
    /// it is not a synonym for a configured authored Follow.
    Armed,
    Transitioning,
    /// Both transports have reached their visual/audio/lighting boundary and
    /// the runtime is waiting for the single authoritative settlement.
    Settling,
    Held,
    Fault,
    /// A generation-fenced teardown is in progress. It remains observable
    /// until all child transports acknowledge the newer generation.
    Aborting,
}

/// The only causes that may admit a Timeline Follow. In particular, seeking
/// to the end, looping, stopping, project replacement, and a fault are never
/// natural playback boundaries.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineFollowAdmissionReason {
    NaturalPlaybackBoundary,
    PrerollBeforeNaturalPlaybackBoundary,
}

/// Why an in-flight Follow was cancelled before it could settle. This is
/// runtime truth, not authored intent; it gives the backend/UI a fail-closed
/// reason instead of inferring one from a playhead change.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineFollowAbortReason {
    Stop,
    ManualSeek,
    ProjectReplacement,
    TimelineBankReplacement,
    LoopWrap,
    PlaybackFault,
    ExplicitAbort,
    ClockDiscontinuity,
}

/// The terminal disposition for one captured Follow generation. `Cut` is the
/// selected fault-policy disposition, whereas `Completed` is a settled timed
/// transition. An abort is deliberately distinct from either result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineFollowOutcome {
    Completed,
    Cut,
    Held,
    Fault,
    Aborted { reason: TimelineFollowAbortReason },
}

/// Domains that must reach one generation-fenced settlement before a Follow
/// may replace its source Timeline with the target Timeline.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TimelineFollowSettlementDomain {
    Audio,
    Video,
    Lighting,
}

/// Stable identity of one production consumer captured when a Follow is
/// admitted. Lighting routes intentionally settle as one aggregate consumer;
/// route-specific delivery diagnostics remain owned by the lighting runtime.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineFollowSettlementConsumerId {
    Audio,
    VideoOutput { output_id: VideoOutputId },
    Lighting,
}

impl TimelineFollowSettlementConsumerId {
    pub const fn domain(self) -> TimelineFollowSettlementDomain {
        match self {
            Self::Audio => TimelineFollowSettlementDomain::Audio,
            Self::VideoOutput { .. } => TimelineFollowSettlementDomain::Video,
            Self::Lighting => TimelineFollowSettlementDomain::Lighting,
        }
    }
}

/// Runtime state for an expected Follow settlement consumer or its aggregate
/// domain. `TimedOut` is produced only by the authoritative runtime deadline;
/// it is not a consumer acknowledgement result.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineFollowSettlementState {
    #[default]
    Pending,
    Applied,
    NotApplicable,
    Fault,
    TimedOut,
}

impl TimelineFollowSettlementState {
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineFollowSettlementConsumerSummary {
    pub consumer_id: TimelineFollowSettlementConsumerId,
    #[serde(default)]
    pub state: TimelineFollowSettlementState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fault: Option<String>,
}

/// One of the three externally observable settlement domains. `consumers` is
/// the frozen admission-time production quorum: identities remain present
/// when their result is `NotApplicable`. An empty quorum derives
/// `NotApplicable` (for example, no active video presenter), except for a
/// domain-level internal Lighting fault or timeout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineFollowSettlementDomainSummary {
    pub domain: TimelineFollowSettlementDomain,
    #[serde(default)]
    pub state: TimelineFollowSettlementState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fault: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumers: Vec<TimelineFollowSettlementConsumerSummary>,
}

/// Runtime-only settlement of one captured Follow generation. The target
/// clock is frozen at its terminal boundary while this summary is active. Cut
/// follows the same Settling handshake as timed transitions. The engine uses
/// [`TIMELINE_FOLLOW_SETTLEMENT_TIMEOUT_MS`] unless a test-only override is
/// supplied and applies the captured fault policy to both faults and timeout.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineFollowSettlementSummary {
    #[serde(default)]
    pub started_at_ms: u64,
    #[serde(default)]
    pub deadline_ms: u64,
    #[serde(default)]
    pub state: TimelineFollowSettlementState,
    #[serde(default)]
    pub progress_millis: u16,
    #[serde(default)]
    pub fault_policy: TimelineFollowFaultPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fault: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub domains: Vec<TimelineFollowSettlementDomainSummary>,
}

pub const TIMELINE_FOLLOW_SETTLEMENT_TIMEOUT_MS: u64 = 2_000;

/// One consumer acknowledgement. The authoritative runtime rejects stale
/// epochs/generations and validates that `domain` matches `consumer_id` before
/// mutating settlement state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineFollowSettlementAck {
    pub epoch: u64,
    pub generation: u64,
    pub domain: TimelineFollowSettlementDomain,
    pub consumer_id: TimelineFollowSettlementConsumerId,
    pub result: TimelineFollowSettlementAckResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineFollowSettlementAckResult {
    Applied,
    NotApplicable,
    Fault { fault: String },
}

pub fn validate_timeline_follow_settlement_ack(
    ack: &TimelineFollowSettlementAck,
) -> Result<(), String> {
    if ack.consumer_id.domain() != ack.domain {
        return Err(
            "Timeline Follow settlement ACK domain does not match consumer identity".to_string(),
        );
    }
    if let TimelineFollowSettlementAckResult::Fault { fault } = &ack.result {
        if fault.trim().is_empty() {
            return Err("Timeline Follow settlement ACK fault must not be empty".to_string());
        }
    }
    Ok(())
}

pub fn validate_timeline_follow_settlement_summary(
    settlement: &TimelineFollowSettlementSummary,
) -> Result<(), String> {
    if settlement.progress_millis > 1_000 {
        return Err("Timeline Follow settlement progress must be between 0 and 1000".to_string());
    }
    if settlement.deadline_ms < settlement.started_at_ms {
        return Err("Timeline Follow settlement deadline precedes its start".to_string());
    }
    validate_timeline_follow_settlement_state_fault(
        settlement.state,
        settlement.fault.as_deref(),
        "aggregate",
    )?;

    let mut domains = BTreeSet::new();
    let mut consumers = BTreeSet::new();
    for domain in &settlement.domains {
        if !domains.insert(domain.domain) {
            return Err(format!(
                "Timeline Follow settlement contains duplicate {:?} domain",
                domain.domain
            ));
        }
        validate_timeline_follow_settlement_state_fault(
            domain.state,
            domain.fault.as_deref(),
            "domain",
        )?;
        for consumer in &domain.consumers {
            if consumer.consumer_id.domain() != domain.domain {
                return Err(format!(
                    "Timeline Follow settlement {:?} consumer is assigned to the wrong domain",
                    consumer.consumer_id
                ));
            }
            if !consumers.insert(consumer.consumer_id) {
                return Err(format!(
                    "Timeline Follow settlement contains duplicate {:?} consumer",
                    consumer.consumer_id
                ));
            }
            validate_timeline_follow_settlement_state_fault(
                consumer.state,
                consumer.fault.as_deref(),
                "consumer",
            )?;
        }
        validate_timeline_follow_settlement_domain_state(domain)?;
    }

    for required in [
        TimelineFollowSettlementDomain::Audio,
        TimelineFollowSettlementDomain::Video,
        TimelineFollowSettlementDomain::Lighting,
    ] {
        if !domains.contains(&required) {
            return Err(format!(
                "Timeline Follow settlement is missing {:?} domain",
                required
            ));
        }
    }
    let derived_state = derive_timeline_follow_settlement_aggregate_state(&settlement.domains)?;
    if settlement.state != derived_state {
        return Err(format!(
            "Timeline Follow settlement aggregate state {:?} contradicts derived {:?} state",
            settlement.state, derived_state
        ));
    }
    Ok(())
}

fn validate_timeline_follow_settlement_state_fault(
    state: TimelineFollowSettlementState,
    fault: Option<&str>,
    scope: &str,
) -> Result<(), String> {
    match (state, fault) {
        (TimelineFollowSettlementState::Fault, Some(fault)) if !fault.trim().is_empty() => Ok(()),
        (TimelineFollowSettlementState::Fault, _) => Err(format!(
            "Timeline Follow settlement {scope} Fault state requires non-empty fault text"
        )),
        (_, Some(_)) => Err(format!(
            "Timeline Follow settlement {scope} fault text is allowed only for Fault state"
        )),
        (_, None) => Ok(()),
    }
}

fn validate_timeline_follow_settlement_domain_state(
    domain: &TimelineFollowSettlementDomainSummary,
) -> Result<(), String> {
    if domain.consumers.is_empty() {
        if domain.state == TimelineFollowSettlementState::NotApplicable
            || (domain.domain == TimelineFollowSettlementDomain::Lighting
                && matches!(
                    domain.state,
                    TimelineFollowSettlementState::Fault | TimelineFollowSettlementState::TimedOut
                ))
        {
            return Ok(());
        }
        return Err(format!(
            "Timeline Follow settlement {:?} domain has no consumer and must be NotApplicable",
            domain.domain
        ));
    }

    let derived = derive_timeline_follow_settlement_consumer_state(&domain.consumers)?;
    if domain.state == derived {
        return Ok(());
    }
    let lighting_internal_terminal = domain.domain == TimelineFollowSettlementDomain::Lighting
        && matches!(
            domain.state,
            TimelineFollowSettlementState::Fault | TimelineFollowSettlementState::TimedOut
        )
        && derived != TimelineFollowSettlementState::Fault
        && (domain.state != TimelineFollowSettlementState::TimedOut
            || derived != TimelineFollowSettlementState::TimedOut);
    if lighting_internal_terminal {
        return Ok(());
    }
    Err(format!(
        "Timeline Follow settlement {:?} domain state {:?} contradicts derived {:?} consumer state",
        domain.domain, domain.state, derived
    ))
}

fn derive_timeline_follow_settlement_consumer_state(
    consumers: &[TimelineFollowSettlementConsumerSummary],
) -> Result<TimelineFollowSettlementState, String> {
    if consumers
        .iter()
        .any(|consumer| consumer.state == TimelineFollowSettlementState::Fault)
    {
        return Ok(TimelineFollowSettlementState::Fault);
    }
    if consumers
        .iter()
        .any(|consumer| consumer.state == TimelineFollowSettlementState::TimedOut)
    {
        return Ok(TimelineFollowSettlementState::TimedOut);
    }
    if consumers
        .iter()
        .any(|consumer| consumer.state == TimelineFollowSettlementState::Pending)
    {
        return Ok(TimelineFollowSettlementState::Pending);
    }
    if consumers
        .iter()
        .all(|consumer| consumer.state == TimelineFollowSettlementState::NotApplicable)
    {
        return Ok(TimelineFollowSettlementState::NotApplicable);
    }
    if consumers.iter().all(|consumer| {
        matches!(
            consumer.state,
            TimelineFollowSettlementState::Applied | TimelineFollowSettlementState::NotApplicable
        )
    }) && consumers
        .iter()
        .any(|consumer| consumer.state == TimelineFollowSettlementState::Applied)
    {
        return Ok(TimelineFollowSettlementState::Applied);
    }
    Err("Timeline Follow settlement consumers have no coherent derived state".to_string())
}

fn derive_timeline_follow_settlement_aggregate_state(
    domains: &[TimelineFollowSettlementDomainSummary],
) -> Result<TimelineFollowSettlementState, String> {
    if domains
        .iter()
        .any(|domain| domain.state == TimelineFollowSettlementState::Fault)
    {
        return Ok(TimelineFollowSettlementState::Fault);
    }
    if domains
        .iter()
        .any(|domain| domain.state == TimelineFollowSettlementState::TimedOut)
    {
        return Ok(TimelineFollowSettlementState::TimedOut);
    }
    if domains
        .iter()
        .any(|domain| domain.state == TimelineFollowSettlementState::Pending)
    {
        return Ok(TimelineFollowSettlementState::Pending);
    }
    if domains.iter().all(|domain| {
        matches!(
            domain.state,
            TimelineFollowSettlementState::Applied | TimelineFollowSettlementState::NotApplicable
        )
    }) {
        return Ok(TimelineFollowSettlementState::Applied);
    }
    Err("Timeline Follow settlement aggregate has no coherent terminal state".to_string())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineFollowRuntimeSummary {
    pub generation: u64,
    pub status: TimelineFollowRuntimeStatus,
    #[serde(default)]
    pub admission_reason: Option<TimelineFollowAdmissionReason>,
    #[serde(default)]
    pub outcome: Option<TimelineFollowOutcome>,
    #[serde(default)]
    pub source_timeline_id: Option<TimelineId>,
    #[serde(default)]
    pub target_timeline_id: Option<TimelineId>,
    #[serde(default)]
    pub elapsed_ms: u64,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub progress_millis: u16,
    #[serde(default)]
    pub fault: Option<String>,
    /// Runtime-only; this is not written into authored Timeline data.
    #[serde(default)]
    pub transition_hold_active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement: Option<TimelineFollowSettlementSummary>,
}

/// Bounded public read model for the runtime-only Follow transport. The
/// backend stamps `epoch` from its output/authority fence when publishing this
/// DTO; `(epoch, generation)` is therefore the stable stale-read fence for a
/// UI without ever serializing runtime state into authored Timeline data.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineFollowRuntimeStatusSnapshot {
    #[serde(default)]
    pub epoch: u64,
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub status: TimelineFollowRuntimeStatus,
    #[serde(default)]
    pub admission_reason: Option<TimelineFollowAdmissionReason>,
    #[serde(default)]
    pub outcome: Option<TimelineFollowOutcome>,
    #[serde(default)]
    pub source_timeline_id: Option<TimelineId>,
    #[serde(default)]
    pub target_timeline_id: Option<TimelineId>,
    #[serde(default)]
    pub elapsed_ms: u64,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub progress_millis: u16,
    #[serde(default)]
    pub fault: Option<String>,
    #[serde(default)]
    pub transition_hold_active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement: Option<TimelineFollowSettlementSummary>,
}

impl TimelineFollowRuntimeStatusSnapshot {
    pub fn from_runtime(epoch: u64, runtime: &TimelineFollowRuntimeSummary) -> Self {
        Self {
            epoch,
            generation: runtime.generation,
            status: runtime.status,
            admission_reason: runtime.admission_reason,
            outcome: runtime.outcome.clone(),
            source_timeline_id: runtime.source_timeline_id,
            target_timeline_id: runtime.target_timeline_id,
            elapsed_ms: runtime.elapsed_ms,
            duration_ms: runtime.duration_ms,
            progress_millis: runtime.progress_millis,
            fault: runtime.fault.clone(),
            transition_hold_active: runtime.transition_hold_active,
            settlement: runtime.settlement.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineSnapshot {
    #[serde(default = "default_timeline_id")]
    pub id: TimelineId,
    #[serde(default = "default_timeline_label")]
    pub label: String,
    #[serde(default)]
    pub layers: Vec<TimelineLayerSummary>,
    pub events: Vec<TimelineCueEventSummary>,
    pub automations: Vec<TimelineAutomationSummary>,
    pub video_automations: Vec<TimelineVideoAutomationSummary>,
    #[serde(default)]
    pub audio: Option<AudioAnalysisSummary>,
    #[serde(default)]
    pub audio_clips: Vec<TimelineAudioClipSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub video_clips: Vec<TimelineVideoClipSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phases: Vec<TimelinePhaseSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_groups: Vec<TimelineItemGroupSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_region: Option<TimelineLoopRegionSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow: Option<TimelineFollowSummary>,
    #[serde(default)]
    pub guide_enabled: bool,
    #[serde(default)]
    pub audio_offset_ms: i64,
    #[serde(default)]
    pub audio_muted: bool,
    #[serde(default)]
    pub metronome_enabled: bool,
    #[serde(default = "default_timeline_count_in_beats")]
    pub count_in_beats: u8,
    /// Authored tempo/meter authority shared by arranger, snap, count-in,
    /// metronome, MTC conversion, and the runtime click scheduler.  Empty is
    /// the additive legacy representation (current clock BPM, 4/4).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tempo_meter_map: Vec<TimelineTempoMeterPoint>,
    #[serde(
        default = "default_timeline_tempo_meter_map_version",
        skip_serializing_if = "is_default_timeline_tempo_meter_map_version"
    )]
    pub tempo_meter_map_version: u8,
    #[serde(default, skip_serializing)]
    pub count_in_remaining_ms: u64,
    #[serde(default, skip_serializing)]
    pub audio_transport_revision: u64,
    /// Monotonic, engine-owned Timeline Play authority epoch. It is published
    /// only through runtime/control-plane observations and cannot be persisted
    /// or injected by project JSON.
    #[serde(default, skip)]
    pub transport_epoch: u64,
    /// Monotonic, engine-owned Timeline Play authority generation. Together
    /// with `transport_epoch` it is a non-saturating ABA fence.
    #[serde(default, skip)]
    pub transport_generation: u64,
    /// Runtime-only positions for every active child Timeline transport,
    /// including recursively nested Scene Blocks. This is published to the
    /// native audio worker and is never written to `.sdc` or sent to the UI.
    #[serde(default, skip_serializing)]
    pub active_child_transports: Vec<ChildTimelineTransportRuntimeSummary>,
    #[serde(default, skip_serializing)]
    pub loop_runtime: TimelineLoopRuntimeSummary,
    /// Runtime transport status is supplied only through
    /// `TimelineFollowRuntimeStatusSnapshot`; project JSON may neither persist
    /// nor inject a Follow transport.
    #[serde(default, skip)]
    pub follow_runtime: TimelineFollowRuntimeSummary,
    #[serde(default, skip_serializing)]
    pub guide_cues: Vec<TimelineGuideCueSummary>,
    /// Runtime-only bounded click lookahead.  Authored project JSON cannot
    /// inject queued native frames or their transport identity.
    #[serde(default, skip)]
    pub click_events: Vec<TimelineClickEventSummary>,
    #[serde(default, skip)]
    pub click_schedule_generation: u64,
    #[serde(default, skip)]
    pub click_queue_overflow: Option<String>,
    pub playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
}

impl Default for TimelineSnapshot {
    fn default() -> Self {
        Self {
            id: default_timeline_id(),
            label: default_timeline_label(),
            layers: Vec::new(),
            events: Vec::new(),
            automations: Vec::new(),
            video_automations: Vec::new(),
            audio: None,
            audio_clips: Vec::new(),
            video_clips: Vec::new(),
            phases: Vec::new(),
            item_groups: Vec::new(),
            loop_region: None,
            follow: None,
            guide_enabled: false,
            audio_offset_ms: 0,
            audio_muted: false,
            metronome_enabled: false,
            count_in_beats: default_timeline_count_in_beats(),
            tempo_meter_map: Vec::new(),
            tempo_meter_map_version: default_timeline_tempo_meter_map_version(),
            count_in_remaining_ms: 0,
            audio_transport_revision: 0,
            transport_epoch: 0,
            transport_generation: 0,
            active_child_transports: Vec::new(),
            loop_runtime: TimelineLoopRuntimeSummary::default(),
            follow_runtime: TimelineFollowRuntimeSummary::default(),
            guide_cues: Vec::new(),
            click_events: Vec::new(),
            click_schedule_generation: 0,
            click_queue_overflow: None,
            playing: false,
            position_ms: 0,
            duration_ms: 0,
        }
    }
}

const fn default_timeline_id() -> TimelineId {
    TimelineId(1)
}

fn default_timeline_label() -> String {
    "Timeline 1".to_string()
}

pub const TIMELINE_MAX_PHASES: usize = 256;
pub const TIMELINE_MAX_ITEM_GROUPS: usize = 256;
pub const TIMELINE_MAX_GROUP_MEMBERS: usize = 128;
/// Follow timing is bounded so an admitted transition cannot hold two
/// Timeline transports beyond the established Clip Take runtime envelope.
pub const TIMELINE_FOLLOW_MAX_RESOLVED_DURATION_MS: u64 = 600_000;
/// At the slowest supported BPM (20), these bounds each resolve to at most ten
/// minutes: 200 beats and 50 four-beat bars.
pub const TIMELINE_FOLLOW_MAX_DURATION_BEAT_MILLIUNITS: u64 = 200_000;
pub const TIMELINE_FOLLOW_MAX_DURATION_BAR_MILLIUNITS: u64 = 50_000;
pub const TIMELINE_FOLLOW_MAX_PREROLL_MS: u64 = TIMELINE_FOLLOW_MAX_RESOLVED_DURATION_MS;
pub const TIMELINE_FOLLOW_MAX_TRANS_CADENCE_BARS: u16 = 256;
pub const TIMELINE_FOLLOW_MAX_TRANS_TARGETS: usize = 64;

/// Validate the authored Timeline additions without observing machine-local
/// media availability. Media identity is checked only against the project
/// catalog; path presence, proxies and decoder readiness remain runtime truth.
pub fn validate_timeline_authoring(
    timeline: &TimelineSnapshot,
    media_assets: &[MediaAssetSummary],
) -> Result<(), String> {
    if timeline.id.0 == 0 {
        return Err("Timeline ID must be non-zero".to_string());
    }
    if timeline.label.trim().is_empty() {
        return Err("Timeline label must not be empty".to_string());
    }
    validate_timeline_tempo_meter_map(timeline.tempo_meter_map_version, &timeline.tempo_meter_map)?;
    let layer_by_id = timeline
        .layers
        .iter()
        .map(|layer| (layer.id, layer))
        .collect::<BTreeMap<_, _>>();
    // Layer 0 remains the authored identity of the legacy implicit Lighting
    // lane. Explicit and legacy lanes must nevertheless be unambiguous.
    if layer_by_id.len() != timeline.layers.len() {
        return Err("Timeline layer IDs must be unique".to_string());
    }
    let asset_by_id = media_assets
        .iter()
        .map(|asset| (asset.id, asset))
        .collect::<BTreeMap<_, _>>();

    if timeline.phases.len() > TIMELINE_MAX_PHASES {
        return Err(format!(
            "Timeline contains {} Phases; the limit is {TIMELINE_MAX_PHASES}",
            timeline.phases.len()
        ));
    }
    let mut phase_ids = BTreeSet::new();
    let mut previous_phase_end = 0;
    for (index, phase) in timeline.phases.iter().enumerate() {
        if phase.id.0 == 0 || !phase_ids.insert(phase.id) {
            return Err("Timeline Phase IDs must be non-zero and unique".to_string());
        }
        if phase.label.trim().is_empty() || phase.start_ms >= phase.end_ms {
            return Err(format!(
                "Timeline Phase {} requires a label and a non-empty range",
                phase.id.0
            ));
        }
        if index > 0 && phase.start_ms < previous_phase_end {
            return Err("Timeline Phases must be ordered and non-overlapping".to_string());
        }
        if timeline.duration_ms > 0 && phase.end_ms > timeline.duration_ms {
            return Err(format!(
                "Timeline Phase {} exceeds the Timeline duration",
                phase.id.0
            ));
        }
        previous_phase_end = phase.end_ms;
    }

    let mut video_clip_ids = BTreeSet::new();
    for clip in &timeline.video_clips {
        if clip.id.0 == 0 || !video_clip_ids.insert(clip.id) {
            return Err("Timeline Video clip IDs must be non-zero and unique".to_string());
        }
        let layer = layer_by_id.get(&clip.layer_id).ok_or_else(|| {
            format!(
                "Timeline Video clip {} references missing layer {}",
                clip.id.0, clip.layer_id
            )
        })?;
        if !matches!(layer.kind, TimelineLayerKind::Video) {
            return Err(format!(
                "Timeline Video clip {} references non-Video layer {}",
                clip.id.0, clip.layer_id
            ));
        }
        let asset = asset_by_id.get(&clip.media_asset_id).ok_or_else(|| {
            format!(
                "Timeline Video clip {} references missing MediaAsset {}",
                clip.id.0, clip.media_asset_id
            )
        })?;
        if !matches!(
            asset.source.kind,
            VideoSourceKind::File | VideoSourceKind::StillImage
        ) {
            return Err(format!(
                "Timeline Video clip {} requires a file or still MediaAsset",
                clip.id.0
            ));
        }
        if clip.duration_ms == 0
            || clip.start_ms.checked_add(clip.duration_ms).is_none()
            || clip.fade_in_ms.saturating_add(clip.fade_out_ms) > clip.duration_ms
        {
            return Err(format!(
                "Timeline Video clip {} has invalid duration or fades",
                clip.id.0
            ));
        }
    }

    for automation in &timeline.automations {
        let Some(layer_id) = automation.timeline_layer_id else {
            continue;
        };
        let layer = layer_by_id.get(&layer_id).ok_or_else(|| {
            format!(
                "Timeline Lighting automation {} references missing Timeline layer {layer_id}",
                automation.id
            )
        })?;
        if !matches!(layer.kind, TimelineLayerKind::Lighting) {
            return Err(format!(
                "Timeline Lighting automation {} references non-Lighting Timeline layer {layer_id}",
                automation.id
            ));
        }
    }

    for automation in &timeline.video_automations {
        let Some(layer_id) = automation.timeline_layer_id else {
            continue;
        };
        let layer = layer_by_id.get(&layer_id).ok_or_else(|| {
            format!(
                "Timeline Video automation {} references missing Timeline layer {layer_id}",
                automation.id
            )
        })?;
        if !matches!(layer.kind, TimelineLayerKind::Video) {
            return Err(format!(
                "Timeline Video automation {} references non-Video Timeline layer {layer_id}",
                automation.id
            ));
        }
    }

    let mut audio_clip_ids = BTreeSet::new();
    for clip in &timeline.audio_clips {
        // ID 0 remains load-compatible only for the historical path-backed
        // singleton. New MediaAsset-backed clips and grouped clips require a
        // stable non-zero identity.
        if (clip.id == 0 && clip.media_asset_id.is_some()) || !audio_clip_ids.insert(clip.id) {
            return Err("Timeline Audio clip IDs must be non-zero and unique".to_string());
        }
        if let Some(asset_id) = clip.media_asset_id {
            let asset = asset_by_id.get(&asset_id).ok_or_else(|| {
                format!(
                    "Timeline Audio clip {} references missing MediaAsset {asset_id}",
                    clip.id
                )
            })?;
            if !asset
                .source
                .metadata
                .is_some_and(|metadata| metadata.has_audio)
            {
                return Err(format!(
                    "Timeline Audio clip {} references a MediaAsset without audio",
                    clip.id
                ));
            }
        } else if clip.path.trim().is_empty() {
            return Err(format!(
                "Legacy Timeline Audio clip {} requires a path",
                clip.id
            ));
        }
        if clip.duration_ms == 0
            || clip.start_ms.checked_add(clip.duration_ms).is_none()
            || !clip.gain.is_finite()
            || !(0.0..=2.0).contains(&clip.gain)
            || clip.fade_in_ms.saturating_add(clip.fade_out_ms) > clip.duration_ms
        {
            return Err(format!(
                "Timeline Audio clip {} has invalid duration, gain, or fades",
                clip.id
            ));
        }
    }

    if timeline.item_groups.len() > TIMELINE_MAX_ITEM_GROUPS {
        return Err(format!(
            "Timeline contains {} item groups; the limit is {TIMELINE_MAX_ITEM_GROUPS}",
            timeline.item_groups.len()
        ));
    }
    let event_ids = timeline
        .events
        .iter()
        .map(|event| event.id)
        .collect::<BTreeSet<_>>();
    let lighting_automation_ids = timeline
        .automations
        .iter()
        .map(|automation| automation.id)
        .collect::<BTreeSet<_>>();
    let video_automation_ids = timeline
        .video_automations
        .iter()
        .map(|automation| automation.id)
        .collect::<BTreeSet<_>>();
    let mut group_ids = BTreeSet::new();
    let mut grouped_items = BTreeSet::new();
    for group in &timeline.item_groups {
        if group.id.0 == 0 || !group_ids.insert(group.id) {
            return Err("Timeline item group IDs must be non-zero and unique".to_string());
        }
        if group.members.len() < 2 || group.members.len() > TIMELINE_MAX_GROUP_MEMBERS {
            return Err(format!(
                "Timeline item group {} must contain 2..={TIMELINE_MAX_GROUP_MEMBERS} items",
                group.id.0
            ));
        }
        let mut own_members = BTreeSet::new();
        for member in &group.members {
            if !own_members.insert(*member) || !grouped_items.insert(*member) {
                return Err("Timeline items may belong to at most one group".to_string());
            }
            let exists = match *member {
                TimelineItemRef::LightingEvent { event_id } => event_ids.contains(&event_id),
                TimelineItemRef::VideoClip { clip_id } => video_clip_ids.contains(&clip_id),
                TimelineItemRef::AudioClip { clip_id } => {
                    clip_id != 0 && audio_clip_ids.contains(&clip_id)
                }
                TimelineItemRef::LightingAutomation { automation_id } => {
                    lighting_automation_ids.contains(&automation_id)
                }
                TimelineItemRef::VideoAutomation { automation_id } => {
                    video_automation_ids.contains(&automation_id)
                }
            };
            if !exists {
                return Err(format!(
                    "Timeline item group {} contains a missing item",
                    group.id.0
                ));
            }
        }
    }

    if let Some(loop_region) = &timeline.loop_region {
        if loop_region.a_ms >= loop_region.b_ms
            || (timeline.duration_ms > 0 && loop_region.b_ms > timeline.duration_ms)
            || loop_region
                .musical_length_beats
                .is_some_and(|beats| !beats.is_finite() || beats <= 0.0)
        {
            return Err("Timeline loop requires a finite non-empty A-B range".to_string());
        }
    }

    if let Some(follow) = &timeline.follow {
        if follow.next_timeline_id.0 == 0 || follow.next_timeline_id == timeline.id {
            return Err(
                "Timeline Follow requires a different non-zero next Timeline ID".to_string(),
            );
        }
        let duration_limit = match follow.duration.unit {
            VideoClipTakeDurationUnit::Milliseconds => TIMELINE_FOLLOW_MAX_RESOLVED_DURATION_MS,
            VideoClipTakeDurationUnit::Beats => TIMELINE_FOLLOW_MAX_DURATION_BEAT_MILLIUNITS,
            VideoClipTakeDurationUnit::Bars => TIMELINE_FOLLOW_MAX_DURATION_BAR_MILLIUNITS,
        };
        let cut = matches!(follow.video_kind, VideoClipTakeKind::Cut);
        if follow.trans_cadence_bars == 0
            || follow.trans_cadence_bars > TIMELINE_FOLLOW_MAX_TRANS_CADENCE_BARS
            || follow
                .destination_bpm
                .is_some_and(|bpm| !bpm.is_finite() || !(20.0..=300.0).contains(&bpm))
            || follow.duration.value_milliunits > duration_limit
            || follow.preroll_ms > TIMELINE_FOLLOW_MAX_PREROLL_MS
            || (timeline.duration_ms > 0 && follow.preroll_ms > timeline.duration_ms)
            || (cut && (follow.duration.value_milliunits != 0 || follow.preroll_ms != 0))
            || (!cut && follow.duration.value_milliunits == 0)
        {
            return Err(
                "Timeline Follow has invalid timing, preroll, BPM, or Guide cadence".to_string(),
            );
        }
        if follow.trans_target_measures.len() > TIMELINE_FOLLOW_MAX_TRANS_TARGETS
            || follow
                .trans_target_measures
                .iter()
                .any(|measure| *measure == 0 || *measure > TIMELINE_TEMPO_METER_MAX_MEASURE)
            || follow
                .trans_target_measures
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(
                "Timeline Follow Trans target measures must be bounded, non-zero, sorted, and unique"
                    .to_string(),
            );
        }
        if !follow.trans_target_measures.is_empty() {
            let authored_range = timeline
                .tempo_meter_map
                .iter()
                .filter_map(|point| point.measure_number)
                .fold(None, |range, measure| match range {
                    None => Some((measure, measure)),
                    Some((minimum, maximum)) => Some((minimum.min(measure), maximum.max(measure))),
                });
            let Some((minimum, maximum)) = authored_range else {
                return Err(
                    "Timeline Follow exact Trans targets require authored measure anchors"
                        .to_string(),
                );
            };
            if follow
                .trans_target_measures
                .iter()
                .any(|measure| *measure < minimum || *measure > maximum)
            {
                return Err(
                    "Timeline Follow exact Trans target is outside the authored measure range"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

/// Normalize the legacy single-Timeline representation into the ordered bank
/// without changing which Timeline is active. Runtime-only fields inside bank
/// entries are cleared so project files cannot inject a second transport.
pub fn normalize_timeline_bank(snapshot: &mut EngineSnapshot) {
    if snapshot.timeline_bank.is_empty() {
        snapshot.timeline_bank.push(snapshot.timeline.clone());
    }
    if let Some(active) = snapshot
        .timeline_bank
        .iter_mut()
        .find(|timeline| timeline.id == snapshot.timeline.id)
    {
        *active = snapshot.timeline.clone();
    } else {
        snapshot.timeline_bank.push(snapshot.timeline.clone());
    }
    for timeline in &mut snapshot.timeline_bank {
        timeline.playing = false;
        timeline.position_ms = 0;
        timeline.count_in_remaining_ms = 0;
        timeline.audio_transport_revision = 0;
        timeline.active_child_transports.clear();
        timeline.loop_runtime = TimelineLoopRuntimeSummary::default();
        timeline.follow_runtime = TimelineFollowRuntimeSummary::default();
        timeline.guide_cues.clear();
        timeline.click_events.clear();
        timeline.click_schedule_generation = 0;
        timeline.click_queue_overflow = None;
    }
}

pub fn validate_timeline_bank(
    snapshot: &EngineSnapshot,
    media_assets: &[MediaAssetSummary],
) -> Result<(), String> {
    if snapshot.timeline_bank.is_empty() {
        validate_timeline_authoring(&snapshot.timeline, media_assets)?;
        if snapshot
            .timeline
            .follow
            .as_ref()
            .is_some_and(|follow| follow.enabled)
        {
            return Err(format!(
                "Timeline {} enables Follow but has no Timeline below it",
                snapshot.timeline.id.0
            ));
        }
        validate_timeline_follow_transition_effect_owners(
            &snapshot.video,
            &BTreeSet::from([snapshot.timeline.id]),
        )?;
        return Ok(());
    }
    let mut ids = BTreeSet::new();
    for timeline in &snapshot.timeline_bank {
        if !ids.insert(timeline.id) {
            return Err("Timeline bank IDs must be unique".to_string());
        }
        validate_timeline_authoring(timeline, media_assets)?;
    }
    validate_timeline_follow_transition_effect_owners(&snapshot.video, &ids)?;
    let active_index = snapshot
        .timeline_bank
        .iter()
        .position(|timeline| timeline.id == snapshot.timeline.id)
        .ok_or_else(|| "Active Timeline must exist in the Timeline bank".to_string())?;
    for (index, timeline) in snapshot.timeline_bank.iter().enumerate() {
        let Some(follow) = timeline.follow.as_ref().filter(|follow| follow.enabled) else {
            continue;
        };
        let expected = snapshot.timeline_bank.get(index + 1).ok_or_else(|| {
            format!(
                "Timeline {} enables Follow but has no Timeline below it",
                timeline.id.0
            )
        })?;
        if follow.next_timeline_id != expected.id {
            return Err(format!(
                "Timeline {} Follow target must be the next Timeline {} in bank order",
                timeline.id.0, expected.id.0
            ));
        }
    }
    let bank_active = &snapshot.timeline_bank[active_index];
    if bank_active.id != snapshot.timeline.id {
        return Err("Active Timeline bank projection is inconsistent".to_string());
    }
    Ok(())
}

fn validate_timeline_follow_transition_effect_owners(
    video: &VideoSnapshot,
    timeline_ids: &BTreeSet<TimelineId>,
) -> Result<(), String> {
    for chain in &video.effect_chains {
        let VideoEffectScope::Transition {
            owner: VideoTransitionEffectOwner::TimelineFollow { source_timeline_id },
        } = &chain.scope
        else {
            continue;
        };
        if !timeline_ids.contains(source_timeline_id) {
            return Err(format!(
                "Video Timeline Follow effect references missing source Timeline {}",
                source_timeline_id.0
            ));
        }
    }
    Ok(())
}

/// Collision-free identity for the root activation that owns a child
/// Timeline transport tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChildTimelineTransportRootSummary {
    Timeline {
        parent_event_id: TimelineEventId,
        parent_iteration: u64,
    },
    Direct {
        parent_cue_id: CueId,
        generation: u64,
    },
    /// Runtime-only child transport tree owned by one admitted Follow. Source
    /// identity plus generation prevents a later Follow from aliasing an older
    /// tree after Timeline-bank replacement or reordering.
    #[serde(rename = "follow")]
    Follow {
        source_timeline_id: TimelineId,
        generation: u64,
    },
}

/// One recursively nested Scene Block placement in an active transport path.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ChildTimelineTransportPathSegment {
    pub event_id: TimelineEventId,
    pub iteration: u64,
}

/// Runtime-only, exact transport position used by media playback. The path is
/// empty for a root child Timeline and contains every nested Scene Block for a
/// descendant, so identical clip ids in separate branches remain independent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildTimelineTransportRuntimeSummary {
    pub owner_cue_id: CueId,
    pub root: ChildTimelineTransportRootSummary,
    #[serde(default)]
    pub path: Vec<ChildTimelineTransportPathSegment>,
    pub position_ms: u64,
    /// Runtime-only source coordinate for Timeline audio. This can differ by
    /// deterministic fixed-point rounding from lighting's `position_ms` on a
    /// nested fractional-rate path; it is the only position that an audio
    /// Sink may use with `playback_rate_milli`.
    #[serde(default)]
    pub audio_position_ms: u64,
    /// Runtime-only audio admission. A negative source offset holds this false
    /// until its source-time delay reaches zero, so a clip cannot attach early
    /// merely because its clamped position is also zero.
    #[serde(default)]
    pub audio_active: bool,
    /// Runtime-only effective transport rate for media attached to this child
    /// Timeline, represented as deterministic thousandths (1000 = 1.0).
    /// Zero is an explicit invalid-rate sentinel, never a 1.0 fallback. This
    /// is published only through `active_child_transports`, which is never
    /// serialized into a project or exposed to the UI.
    #[serde(default)]
    pub playback_rate_milli: u32,
}

/// Authored timeline content owned by a Cue. Transport state deliberately remains on the
/// parent [`TimelineSnapshot`], so every placement receives its own runtime transport.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChildTimelineSummary {
    #[serde(default)]
    pub layers: Vec<TimelineLayerSummary>,
    #[serde(default)]
    pub events: Vec<TimelineCueEventSummary>,
    #[serde(default)]
    pub automations: Vec<TimelineAutomationSummary>,
    #[serde(default)]
    pub video_automations: Vec<TimelineVideoAutomationSummary>,
    #[serde(default)]
    pub audio: Option<AudioAnalysisSummary>,
    #[serde(default)]
    pub audio_clips: Vec<TimelineAudioClipSummary>,
    /// Drives the authored child-timeline duration from the owning Cue's
    /// `authored_beats` and the live project BPM. Child event millisecond
    /// placement remains the authoring coordinate; Scene Block
    /// `conform_to_tempo` then controls whether source content inherits that
    /// transport rate.
    #[serde(default)]
    pub tempo_driven: bool,
    #[serde(default)]
    pub metronome_enabled: bool,
    #[serde(default = "default_timeline_count_in_beats")]
    pub count_in_beats: u8,
    /// Authored tempo/meter authority for child metronome and count-in.  An
    /// empty legacy map resolves to the owning runtime BPM and 4/4.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tempo_meter_map: Vec<TimelineTempoMeterPoint>,
    #[serde(
        default = "default_timeline_tempo_meter_map_version",
        skip_serializing_if = "is_default_timeline_tempo_meter_map_version"
    )]
    pub tempo_meter_map_version: u8,
    #[serde(default)]
    pub duration_ms: u64,
}

impl Default for ChildTimelineSummary {
    fn default() -> Self {
        Self {
            layers: Vec::new(),
            events: Vec::new(),
            automations: Vec::new(),
            video_automations: Vec::new(),
            audio: None,
            audio_clips: Vec::new(),
            tempo_driven: false,
            metronome_enabled: false,
            count_in_beats: default_timeline_count_in_beats(),
            tempo_meter_map: Vec::new(),
            tempo_meter_map_version: default_timeline_tempo_meter_map_version(),
            duration_ms: 0,
        }
    }
}

const fn default_timeline_count_in_beats() -> u8 {
    4
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LfoShape {
    Sine,
    Cosine,
    /// Daslight Curve ID 4. Native Syndocal authoring does not expose this
    /// legacy shape without a `DaslightCurveSource` profile.
    Pulse,
    Triangle,
    /// Daslight Curve ID 5. Native Syndocal authoring does not expose this
    /// centered ascending ramp without a `DaslightCurveSource` profile.
    Ramp,
    Saw,
    Square,
    Strobe,
    Random,
    Perlin,
    /// Daslight Curve ID 8: a cubed sine generator.
    Sinus3,
    /// Daslight Curve ID 11. The spelling preserves the Daslight UI label.
    Tangeant,
    /// Daslight Curve ID 13. This shape requires a
    /// `DaslightCustomCurveSource`; its point order is semantically relevant.
    DaslightCustom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EffectBlendMode {
    Override,
    Add,
    Multiply,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoEffectTarget {
    pub layer_ids: Vec<VideoLayerId>,
    pub param: VideoParam,
    pub low: f32,
    pub high: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Vec3>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct EffectClockSync {
    pub beats: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeGraphNodeKind {
    Lfo,
    PositionWave,
    Audio,
    Transform,
    Output,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AudioSpectrumBand {
    #[default]
    Bass,
    Mid,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AudioReactiveFeature {
    /// Backward-compatible Bass/Mid/High selection from `band`.
    #[default]
    LegacyBand,
    /// One of the fixed logarithmic bands selected by `band_index`.
    Band,
    Rms,
    Peak,
    SpectralFlux,
    Onset,
    OnsetStrength,
    BeatPhase,
    /// BPM normalized from the supported 60-200 range to 0-1.
    Bpm,
    BpmConfidence,
    SpectralCentroid,
    SpectralDensitySlow,
    SpectralDensityFast,
    Kick,
    KickStrength,
    Snare,
    SnareStrength,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AudioReactiveCurve {
    #[default]
    Linear,
    Smoothstep,
    Exponential,
    Logarithmic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AudioSpectrumSource {
    #[default]
    Timeline,
    Live,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphAudioNode {
    #[serde(default)]
    pub source: AudioSpectrumSource,
    pub band: AudioSpectrumBand,
    #[serde(default)]
    pub feature: AudioReactiveFeature,
    #[serde(default)]
    pub band_index: u8,
    pub gain: f32,
    pub bias: f32,
    #[serde(default)]
    pub attack_ms: u32,
    #[serde(default)]
    pub release_ms: u32,
    #[serde(default)]
    pub gate: f32,
    #[serde(default)]
    pub curve: AudioReactiveCurve,
    #[serde(default)]
    pub invert: bool,
    #[serde(default)]
    pub hold_ms: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeGraphTransformOp {
    Scale,
    Offset,
    Clamp,
    Invert,
    Abs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphLfoNode {
    pub shape: LfoShape,
    pub period_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub phase: f32,
    pub amplitude: f32,
    pub bias: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphPositionWaveNode {
    pub shape: LfoShape,
    pub origin: Vec3,
    pub direction: Vec3,
    pub speed: f32,
    pub wavelength: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub phase: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphTransformNode {
    pub op: NodeGraphTransformOp,
    pub amount: f32,
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphOutputNode {
    pub fixture_ids: Vec<FixtureId>,
    pub target_group_ids: Vec<String>,
    pub attribute: String,
    pub video_targets: Vec<VideoEffectTarget>,
    pub low: u16,
    pub high: u16,
    pub blend_mode: EffectBlendMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphNodeSummary {
    pub id: u64,
    pub label: String,
    pub kind: NodeGraphNodeKind,
    pub x: f32,
    pub y: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lfo: Option<NodeGraphLfoNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_wave: Option<NodeGraphPositionWaveNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<NodeGraphAudioNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<NodeGraphTransformNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<NodeGraphOutputNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphEdgeSummary {
    pub from_node: u64,
    pub from_port: String,
    pub to_node: u64,
    pub to_port: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphAudioRuntimeStatus {
    pub node_id: u64,
    pub input_value: f32,
    pub output_value: f32,
    pub source_available: bool,
    pub safety_zeroed: bool,
    pub held: bool,
    pub feature_sequence: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphSummary {
    pub id: NodeGraphId,
    pub label: String,
    pub enabled: bool,
    pub nodes: Vec<NodeGraphNodeSummary>,
    pub edges: Vec<NodeGraphEdgeSummary>,
    /// Ephemeral engine telemetry. Project/preset writers strip this field.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audio_runtime: Vec<NodeGraphAudioRuntimeStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphPresetFile {
    pub version: u32,
    pub app: String,
    pub graph: NodeGraphSummary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ColorEffectColor {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorEffectStop {
    pub position: f32,
    pub color: ColorEffectColor,
}

/// Daslight 5.0.6.2 TYPE4/ID1 factory cardinality for COLOR, MAPPINGS,
/// and COLOR MAPPINGS palettes.
pub const DASLIGHT_COLOR_PALETTE_MIN_STOPS: usize = 1;
pub const DASLIGHT_COLOR_PALETTE_MAX_STOPS: usize = u8::MAX as usize;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorEffectAlgorithm {
    Cycle,
    Bounce,
    Sequence,
    Random,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorEffectInterpolation {
    Rgb,
    HsvShortest,
    HsvLongest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColorEffectBeamTarget {
    pub fixture_id: FixtureId,
    /// Zero-based Daslight beam/segment index within the fixture profile.
    pub beam_index: u16,
    /// Stable spatial order. Equal values intentionally evaluate in phase.
    pub selection_index: u32,
    /// Virtual Daslight feature for value mappings (for example, Dimmer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_attribute: Option<String>,
}

/// An explicitly authored beam/segment target for value-domain effects.
///
/// This is separate from `ColorEffectBeamTarget` because LFO and Chaser
/// effects modulate an existing feature (for example a virtual Dimmer over an
/// RGB segment) rather than replacing it with a generated colour.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectBeamTarget {
    pub fixture_id: FixtureId,
    /// Zero-based beam/segment index within the fixture profile.
    pub beam_index: u16,
    /// Stable source selection order. Equal values intentionally share phase.
    pub selection_index: u32,
    /// Authored feature being modulated, for example `Dimmer`.
    pub feature_attribute: String,
}

pub const COLOR_EFFECT_SPATIAL_PARAMETER_MODEL_VERSION: u8 = 1;

/// The source raster height consumed by the shared retained-particle evaluator.
///
/// Legacy and ordinary Sparkle recipes use the authored particle height (or one
/// strip row when absent). COLOR MAPPINGS Tube has no Height PARAM: it paints
/// every row of its fixed 100-row raster instead.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorEffectSpatialSparkleRasterMode {
    #[default]
    Sparkle,
    TubeFullRasterHeight,
}

/// Item primitive selected by Daslight COLOR MAPPINGS Bounce ID 21.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorEffectSpatialBounceItem {
    #[default]
    Shape,
    Points,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ColorEffectSpatialRecipe {
    KnightRider {
        /// Apply Daslight's common Grayscale post-process after source-over composition.
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        /// Apply Daslight Transform=1 after rendering the source strip.
        #[serde(default, skip_serializing_if = "is_false")]
        vertical_symmetry: bool,
        /// Lit window width as a percentage of the ordered target strip.
        size: f32,
        one_way: bool,
        fading: bool,
        go_outside: bool,
        gradient: f32,
    },
    Sweep {
        /// Apply Daslight's common Grayscale post-process after rasterization.
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        /// Apply Daslight Transform=1 after rendering the source strip.
        #[serde(default, skip_serializing_if = "is_false")]
        vertical_symmetry: bool,
        /// Alternate the sweep direction after each palette transition.
        direction_change: bool,
    },
    Burst {
        /// Apply Daslight's common Grayscale post-process after rasterization.
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        /// Apply Daslight Transform=1 after rendering the source strip.
        #[serde(default, skip_serializing_if = "is_false")]
        vertical_symmetry: bool,
        /// Radial palette width as a percentage of the ordered target strip.
        color_width: f32,
        /// Palette-segment interpolation width in percent.
        gradient: f32,
    },
    RandomFill {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        vertical_symmetry: bool,
        /// Stable source-derived seed. Zero is a valid deterministic seed.
        #[serde(default, skip_serializing_if = "is_zero_u32")]
        rng_seed: u32,
        /// Filled cell width as a percentage of the ordered target strip.
        point_width: f32,
        /// Point Height is live only when the same recipe also has a Rectangle
        /// placement, where it selects the corrected 100x100 two-dimensional
        /// evaluator. Unplaced VALUE-family recipes retain this as provenance
        /// and their one-row evaluator deliberately does not consume it.
        /// Unplaced COLOR-family recipes omit the field.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_point_height: Option<u16>,
    },
    Sparkle {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        vertical_symmetry: bool,
        /// Explicitly distinguishes Tube's full 100-row raster from ordinary
        /// Sparkle while retaining the common deterministic particle model.
        #[serde(default, skip_serializing_if = "is_standard_sparkle_raster_mode")]
        raster_mode: ColorEffectSpatialSparkleRasterMode,
        /// Stable source-derived seed. Zero is a valid deterministic seed.
        #[serde(default, skip_serializing_if = "is_zero_u32")]
        rng_seed: u32,
        number: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lifetime_ms: Option<u16>,
        /// Raw Daslight 0..0.9 source value retained for provenance.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_lifespan: Option<f32>,
        /// Particle width as a percentage of the ordered target strip.
        width: f32,
        /// Optional particle height for two-dimensional MAPPINGS/COLOR
        /// MAPPINGS rasters. Legacy strip recipes omit it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<f32>,
    },
    /// Daslight's two-dimensional conical spiral raster.
    Spiral {
        /// Apply the COLOR MAPPINGS Grayscale post-process after rasterization.
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        radius: f32,
        arms: u16,
        gradient: f32,
    },
    /// Daslight's paired rotating conical-sector raster.
    Butterfly {
        /// Apply the COLOR MAPPINGS Grayscale post-process after rasterization.
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        color_width: f32,
        gradient: f32,
        clockwise: bool,
    },
    Plasma {
        #[serde(default)]
        grayscale: bool,
        #[serde(default)]
        vertical_symmetry: bool,
        size_x: f32,
        param_x: f32,
        size_y: f32,
        param_y: f32,
        speed_x: f32,
        param_sx: f32,
        speed_y: f32,
        param_sy: f32,
    },
    ColorRainbow {
        #[serde(default)]
        grayscale: bool,
        #[serde(default)]
        vertical_symmetry: bool,
        color_width: f32,
        angle_degrees: f32,
        gradient: f32,
    },
    Rainbow {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        #[serde(default)]
        vertical_symmetry: bool,
        #[serde(default)]
        horizontal_symmetry: bool,
        rotation_degrees: f32,
        color_width: f32,
        angle_degrees: f32,
        gradient: f32,
    },
    Perlin {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        vertical_symmetry: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        horizontal_symmetry: bool,
        #[serde(default, skip_serializing_if = "is_zero_f32")]
        rotation_degrees: f32,
        /// Analytic octave count in the unified evaluator.
        octaves: u8,
        /// Normalized spatial scale. The field is sampled as coordinate / zoom.
        zoom: f32,
        direction_degrees: f32,
        speed: f32,
        amplitude: f32,
    },
    /// Daslight COLOR MAPPINGS ID 50's four-sided moving grid raster.
    Grid {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        size: u16,
        width: u16,
    },
    /// Daslight COLOR MAPPINGS ID 31's paired moving line raster.
    Lines {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        size: u16,
    },
    /// Daslight COLOR MAPPINGS ID 49's moving sinusoidal graph raster.
    Graph {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        height: u16,
        width: u16,
        pitch: u16,
        frequency: u16,
        amplitude: f32,
        offset: f32,
    },
    /// Daslight COLOR MAPPINGS ID 35's falling rain raster.
    Rain {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        /// Stable replacement for the source process-global qrand history.
        #[serde(default, skip_serializing_if = "is_zero_u32")]
        rng_seed: u32,
        speed: u16,
        width: u16,
        height: u16,
        number: u16,
        trail: u16,
    },
    /// Daslight COLOR MAPPINGS ID 21's corrected retained bounce raster.
    Bounce {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        /// Stable replacement for the source process-global qrand history.
        #[serde(default, skip_serializing_if = "is_zero_u32")]
        rng_seed: u32,
        item: ColorEffectSpatialBounceItem,
        /// TYPE7 Shape index. The imported runtime route supports Shape 0 only.
        shape: u8,
        number: u16,
        size: u16,
        speed: u16,
        collide: bool,
        fill: bool,
        points: u16,
    },
    /// Daslight COLOR MAPPINGS ID 29's corrected fixed fire raster.
    Fire {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        /// Stable replacement for the source process-global qrand history.
        #[serde(default, skip_serializing_if = "is_zero_u32")]
        rng_seed: u32,
        flames: u16,
        width: u16,
        /// Corrected live heat cutoff; the source LUT ignored this parameter.
        height: u16,
        hotspot: u16,
    },
    /// Daslight COLOR MAPPINGS ID 47's retained explosion-particle raster.
    Explosion {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        /// Stable replacement for the source process-global qrand history.
        #[serde(default, skip_serializing_if = "is_zero_u32")]
        rng_seed: u32,
        /// TYPE7 Shape index. The imported runtime route supports Shape 0 only.
        shape: u8,
        explosion_number: u16,
        explosion_size: u16,
        particle_number: u16,
        particle_size: u16,
        particle_life: f32,
        trail_size: u16,
        gravity: f32,
    },
    /// Daslight COLOR MAPPINGS ID 48's retained radial star-particle raster.
    Starfield {
        #[serde(default, skip_serializing_if = "is_false")]
        grayscale: bool,
        /// Stable replacement for the source process-global qrand history.
        #[serde(default, skip_serializing_if = "is_zero_u32")]
        rng_seed: u32,
        /// TYPE7 Shape index. The imported runtime route supports Shape 0 only.
        shape: u8,
        particles: u16,
        size: u16,
        trail: u16,
        rotation: f32,
    },
}

/// Coordinate frame used by an imported spatial generator placement.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorEffectSpatialCoordinateFrame {
    DaslightPatchCanvas,
}

/// Daslight mapping primitive used as the spatial generator's inclusion mask.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorEffectSpatialMappingShape {
    /// Daslight `MAPPING TYPE=0`.
    Rectangle,
}

/// Defines how a mapping mask and generator raster are combined.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorEffectSpatialSamplingRule {
    /// Rotate the mapping primitive for inclusion testing, but derive raster
    /// coordinates from the raw, axis-aligned `x/y/sx/sy` window without an
    /// inverse mapping-angle transform.
    RotatedInclusionMaskAxisAlignedRaster,
}

/// Patch-canvas coordinates for one authored beam target. The fixture/beam
/// pair is the stable identity used to join this table to `beam_targets`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColorEffectSpatialPlacementTarget {
    pub fixture_id: FixtureId,
    pub beam_index: u16,
    pub patch_x: i64,
    pub patch_y: i64,
}

/// Optional source-placement contract for imported spatial generators.
///
/// Raw signed values preserve the authored Daslight Patch-canvas window. The
/// importer validates positive extents before constructing this body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorEffectSpatialPlacement {
    pub source_coordinate_frame: ColorEffectSpatialCoordinateFrame,
    pub mapping_shape: ColorEffectSpatialMappingShape,
    pub x: i64,
    pub y: i64,
    pub sx: i64,
    pub sy: i64,
    pub mapping_angle_degrees: f32,
    pub sampling_rule: ColorEffectSpatialSamplingRule,
    /// Daslight MAPPINGS `Transform=1`: fold the generator raster's X axis.
    /// Kept on placement so shared one-dimensional COLOR/VALUE recipes retain
    /// their legacy wire shape while placed 2D variants use the same recipe.
    #[serde(default, skip_serializing_if = "is_false")]
    pub vertical_symmetry: bool,
    /// Daslight MAPPINGS `Transform=2`: fold the generator raster's Y axis.
    #[serde(default, skip_serializing_if = "is_false")]
    pub horizontal_symmetry: bool,
    /// Generator-raster rotation applied after Rectangle-local sampling.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub raster_rotation_degrees: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_coordinates: Vec<ColorEffectSpatialPlacementTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorEffectSpatialPattern {
    pub recipe: ColorEffectSpatialRecipe,
    /// Parameter-domain discriminator. Zero is the legacy dual-route wire model;
    /// one is the unified Syndocal-native model. Higher values are rejected.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub parameter_model_version: u8,
    /// Empty for native effects; the engine derives one beam from each target fixture.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beam_targets: Vec<ColorEffectBeamTarget>,
    /// Imported source placement. Its absence preserves the original
    /// stage-normalized spatial evaluator and the legacy `.sdc v1` byte shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<ColorEffectSpatialPlacement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorEffectRequest {
    pub label: String,
    pub fixture_ids: Vec<FixtureId>,
    pub target_group_ids: Vec<String>,
    pub stops: Vec<ColorEffectStop>,
    pub algorithm: ColorEffectAlgorithm,
    pub interpolation: ColorEffectInterpolation,
    pub period_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub phase: f32,
    pub fixture_spread: f32,
    pub blend_mode: EffectBlendMode,
    /// Optional beam-space recipe. Its absence preserves the original Color engine byte shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spatial_pattern: Option<Box<ColorEffectSpatialPattern>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaslightCurveSource {
    /// The Curve FX Rate value stored in the Daslight DVC generator.
    pub rate: f32,
    /// The generator Size value, before Daslight applies its native 0..1 clamp.
    pub size: f32,
    /// The generator Offset value, before Daslight applies its native 0..1 clamp.
    pub offset: f32,
    /// Daslight 5.0.6.2 precomputes Curve FX on a 40 ms sample grid. This is
    /// retained as source provenance; corrected runtime timing is continuous.
    #[serde(default = "default_daslight_curve_sample_ms")]
    pub sample_ms: u16,
    /// Stable source-identity seed used only by Daslight Curve ID 6 Random.
    /// Daslight's process-global qrand history is not serialized in `.dvc`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rng_seed: Option<u32>,
}

/// One source-order point from Daslight Curve ID 13 Custom.
///
/// `raw_y` preserves both the normalized value and the easing code stored in
/// the integer decade: `0..=1` Linear, `10..=11` InCubic, `20..=21`
/// OutCubic, `30..=31` InOutCubic, and `40..=41` OutInCubic. The destination
/// (right) point selects the easing for its incoming segment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaslightCustomCurvePoint {
    pub x: f32,
    pub raw_y: f32,
}

/// Daslight Curve ID 13 Custom source profile.
///
/// Points deliberately remain in authored source order. Daslight neither
/// sorts them nor requires increasing X values, and its evaluator selects the
/// first source-order interval containing the current phase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaslightCustomCurveSource {
    pub points: Vec<DaslightCustomCurvePoint>,
    /// Adjacent-target phase lag. Target `i` evaluates
    /// `fract(progress - i * phasing)`; this is not the distributed native
    /// Syndocal `fixture_spread` convention.
    pub phasing: f32,
    /// Source buffer cadence recovered from Daslight 5.0.6.2. Retained as
    /// provenance only; corrected runtime evaluates the exact easing
    /// continuously instead of its 40 ms sampled approximation.
    #[serde(default = "default_daslight_curve_sample_ms")]
    pub sample_ms: u16,
}

fn default_daslight_curve_sample_ms() -> u16 {
    40
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LfoEffectRequest {
    pub label: String,
    pub fixture_ids: Vec<FixtureId>,
    pub target_group_ids: Vec<String>,
    pub attribute: String,
    pub video_targets: Vec<VideoEffectTarget>,
    pub shape: LfoShape,
    pub period_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub low: u16,
    pub high: u16,
    pub phase: f32,
    /// Distributes stable target-order offsets across one complete LFO cycle.
    /// Zero preserves the legacy behavior where every lighting target shares one phase.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub fixture_spread: f32,
    /// Optional explicit beam/segment targets. Empty preserves the legacy
    /// fixture/group attribute path and byte shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beam_targets: Vec<EffectBeamTarget>,
    pub blend_mode: EffectBlendMode,
    /// Preserves the sampled source-buffer semantics of a DVC Curve FX.
    /// Native Syndocal LFOs leave this absent and retain their existing behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daslight_curve: Option<DaslightCurveSource>,
    /// Preserves the source-order point/easing semantics of Daslight Curve ID
    /// 13 Custom. Native Syndocal LFOs leave this absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daslight_custom_curve: Option<DaslightCustomCurveSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PositionWaveEffectRequest {
    pub label: String,
    pub fixture_ids: Vec<FixtureId>,
    pub target_group_ids: Vec<String>,
    pub attribute: String,
    pub video_targets: Vec<VideoEffectTarget>,
    pub shape: LfoShape,
    pub origin: Vec3,
    pub direction: Vec3,
    pub speed: f32,
    pub wavelength: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub low: u16,
    pub high: u16,
    pub phase: f32,
    pub blend_mode: EffectBlendMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChaserDirection {
    Forward,
    Reverse,
    Bounce,
    /// Fill targets in order, then clear them in the same order.
    BuildUpDown,
    Random,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChaserStep {
    pub fixture_ids: Vec<FixtureId>,
    pub target_group_ids: Vec<String>,
    /// Optional explicit beam/segment cells for this step. Empty preserves the
    /// original fixture/group Chaser representation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beam_targets: Vec<EffectBeamTarget>,
    pub level: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChaserFeature {
    pub attribute: String,
    pub low: u16,
    pub high: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChaserEffectRequest {
    pub label: String,
    /// Ordered target/level cells. Empty target cells are deliberate blackout gaps.
    pub steps: Vec<ChaserStep>,
    /// Independently scaled attributes; each step level maps from the feature low to high value.
    pub features: Vec<ChaserFeature>,
    /// Free-running duration of one step. Ignored while `clock_sync` is active.
    pub step_duration_ms: u64,
    /// Shared-clock beats per step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub direction: ChaserDirection,
    /// Evenly-spaced simultaneous heads.
    pub wings: u8,
    /// Width of each head in contiguous traversal steps ("pixels on").
    pub active_step_count: u16,
    /// Fraction of a step slot for which the current block remains active.
    pub duty_cycle: f32,
    /// Final fraction of a slot that crossfades the whole current block into the next block.
    /// The crossfade takes precedence over the duty gate inside this interval.
    pub overlap: f32,
    /// Global offset expressed as a fraction of the traversal path.
    pub phase: f32,
    /// Distributes stable target-order offsets across a fraction of the traversal path.
    pub fixture_spread: f32,
    /// Stable random-series selector. Daslight DVC `Random sequence` values use 0..=255;
    /// Syndocal-authored Chasers may use the full u32 range.
    pub random_seed: u64,
    /// Number of complete random permutations generated before the sequence repeats.
    /// Defaults to one for projects saved before this field was introduced.
    #[serde(
        default = "default_chaser_random_cycle_count",
        skip_serializing_if = "chaser_random_cycle_count_is_one"
    )]
    pub random_cycle_count: u8,
    pub blend_mode: EffectBlendMode,
}

fn default_chaser_random_cycle_count() -> u8 {
    1
}

fn chaser_random_cycle_count_is_one(value: &u8) -> bool {
    *value == 1
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct MovePathPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoveInterpolation {
    /// Syndocal's arc-length linear path evaluator.
    Line,
    /// Syndocal's enhanced centripetal Catmull-Rom evaluator.
    Smooth,
    /// Equal-time analytical circular arcs through adjacent path points.
    Circle,
    /// Daslight 5 Circle (ID 221) geometry with corrected continuous timing.
    DaslightCircle,
    /// Daslight 5 Curve (ID 222): uniform Catmull-Rom sampled at 16 slices
    /// per segment, then traversed by polyline arc length.
    DaslightCurve,
    /// Daslight 5 Line (ID 223) geometry with continuous equal-time edges.
    DaslightLine,
    /// Daslight 5 Polygon (ID 224), with equal time per authored edge.
    DaslightPolygon,
    /// Daslight 5 Points (ID 225), intentionally holding equal-time authored vertices.
    DaslightPoints,
}

/// An explicitly authored fixture beam/segment target for Move effects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MoveEffectBeamTarget {
    pub fixture_id: FixtureId,
    /// Raw Daslight BEAMID, retained as the fixture beam/segment index.
    pub beam_index: u16,
    /// Stable source-selection index. Equal values intentionally share phase.
    pub selection_index: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoveCoordinateMode {
    Absolute,
    Relative,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoveDirection {
    Forward,
    Reverse,
    Bounce,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MoveEffectRequest {
    pub label: String,
    pub fixture_ids: Vec<FixtureId>,
    pub target_group_ids: Vec<String>,
    /// Explicit beam/segment targets in authored source order. Empty preserves
    /// the legacy fixture-only Move target contract and `.sdc` byte shape.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beam_targets: Vec<MoveEffectBeamTarget>,
    pub points: Vec<MovePathPoint>,
    pub closed: bool,
    pub interpolation: MoveInterpolation,
    pub coordinate_mode: MoveCoordinateMode,
    pub center_x: f32,
    pub center_y: f32,
    pub size_x: f32,
    pub size_y: f32,
    pub rotation_degrees: f32,
    pub period_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub direction: MoveDirection,
    pub phase: f32,
    pub fixture_spread: f32,
    /// Applies the generator's second-half symmetry in resolved selection
    /// order: Circle reverses traversal from half-cycle, while legacy
    /// Line/Smooth Move paths mirror Pan. An odd centre selection remains in
    /// the first half.
    #[serde(default, skip_serializing_if = "is_false")]
    pub symmetry: bool,
    pub blend_mode: EffectBlendMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ValueEffectPoint {
    /// Normalized position in the envelope, 0..1, strictly increasing across the point list.
    pub position: f32,
    /// Normalized envelope value, 0..1.
    pub value: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CurveEffectPoint {
    /// Normalized channel position in 0..1, strictly increasing.
    pub position: f32,
    /// Normalized channel value in 0..1.
    pub value: f32,
    /// Incoming dy/dx tangent used by the cubic segment ending at this point.
    pub in_tangent: f32,
    /// Outgoing dy/dx tangent used by the cubic segment starting at this point.
    pub out_tangent: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValueEffectInterpolation {
    /// Hold the left point's value until the next point.
    Step,
    /// Linear ramp between adjacent points.
    Line,
    /// Centripetal Catmull-Rom smoothing across the point values.
    Smooth,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValueEffectMode {
    /// The envelope maps directly into the low..high output range.
    Absolute,
    /// The envelope is a bipolar offset around the incoming value; 0.5 means no change.
    Relative,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValueEffectDirection {
    Forward,
    Reverse,
    Bounce,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValueEffectRequest {
    pub label: String,
    pub fixture_ids: Vec<FixtureId>,
    pub target_group_ids: Vec<String>,
    pub attribute: String,
    /// Optional multi-attribute feature ranges. Empty preserves the legacy
    /// single `attribute` / `low` / `high` representation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<ChaserFeature>,
    /// Operator-drawn value envelope points, 2..32, strictly increasing positions.
    pub points: Vec<ValueEffectPoint>,
    /// Optional Daslight-compatible generator mode. When present, the scalar
    /// points above are the Black(0)..White(100) value palette and the shared
    /// colour-spatial recipe is evaluated against the authored beam order.
    /// Absence preserves Syndocal's more general custom-envelope mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spatial_pattern: Option<ColorEffectSpatialPattern>,
    pub interpolation: ValueEffectInterpolation,
    pub mode: ValueEffectMode,
    pub direction: ValueEffectDirection,
    /// Free-running duration of one envelope pass. Ignored while `clock_sync` is active.
    pub period_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub low: u16,
    pub high: u16,
    pub phase: f32,
    /// Distributes stable target-order phase offsets across a fraction of the envelope.
    pub fixture_spread: f32,
    pub blend_mode: EffectBlendMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurveEffectRequest {
    pub label: String,
    pub fixture_ids: Vec<FixtureId>,
    pub target_group_ids: Vec<String>,
    pub attribute: String,
    /// Optional multi-attribute feature ranges. Empty preserves the legacy
    /// single `attribute` / `low` / `high` representation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<ChaserFeature>,
    /// Editable cubic channel-function points, 2..32, strictly increasing.
    pub points: Vec<CurveEffectPoint>,
    pub mode: ValueEffectMode,
    pub direction: ValueEffectDirection,
    /// Free-running duration of one curve pass. Ignored while clock-synced.
    pub period_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub low: u16,
    pub high: u16,
    pub phase: f32,
    pub fixture_spread: f32,
    pub blend_mode: EffectBlendMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MappingEffectDirection {
    Forward,
    Reverse,
    Bounce,
    Static,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MappingEffectRequest {
    pub label: String,
    /// Explicit fixture order. Group members are appended in patch order at resolve time.
    pub fixture_ids: Vec<FixtureId>,
    pub target_group_ids: Vec<String>,
    pub attribute: String,
    /// Optional multi-attribute feature ranges. Empty preserves the legacy
    /// single `attribute` / `low` / `high` representation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<ChaserFeature>,
    /// Scalar function sampled over the resolved fixture order.
    pub shape: LfoShape,
    pub mode: ValueEffectMode,
    pub direction: MappingEffectDirection,
    /// Free-running duration of one mapping traversal. Ignored while clock-synced.
    pub period_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub low: u16,
    pub high: u16,
    pub phase: f32,
    /// Fraction of one order span distributed across the resolved targets.
    pub fixture_spread: f32,
    /// Number of function cycles distributed over the fixture span.
    pub repetitions: f32,
    pub blend_mode: EffectBlendMode,
}

/// Operator-facing provenance for an embedded colour-mapping raster. The media
/// itself is always stored as bounded RGB16 frames so project playback never
/// depends on an external file or a decoder on the 44 Hz path.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorMappingSourceKind {
    Image,
    Text,
    Video,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorMappingPlaybackDirection {
    Forward,
    Reverse,
    Bounce,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorMappingWrapMode {
    Clamp,
    Repeat,
    Mirror,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ColorMappingSampling {
    Nearest,
    Bilinear,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorMappingCellTarget {
    pub fixture_id: FixtureId,
    /// Zero-based RGB/RGBW beam or segment. Zero also addresses a fixture's
    /// main colour binding when no segmented binding exists.
    pub beam_index: u16,
    /// Stable authored order for strip/matrix diagnostics.
    pub selection_index: u32,
    /// Normalized source-space cell coordinate before request UV transforms.
    pub u: f32,
    pub v: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_attribute: Option<String>,
    /// Optional DMX range for a scalar feature mapping. Legacy projects that
    /// omit these fields retain the full 0..65535 range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_low: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_high: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorMappingFrame {
    /// Packed RGB16 pixels in row-major order: 0xRRRRGGGGBBBB. Values fit in
    /// JavaScript's exact integer range and keep JSON materially smaller than
    /// three-field pixel objects.
    pub pixels: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorMappingEffectRequest {
    pub label: String,
    pub fixture_ids: Vec<FixtureId>,
    pub target_group_ids: Vec<String>,
    pub source_kind: ColorMappingSourceKind,
    /// Embedded raster dimensions. Both axes are bounded to 1..=64.
    pub width: u16,
    pub height: u16,
    /// One image/text frame or 2..=64 uniformly timed video frames.
    pub frames: Vec<ColorMappingFrame>,
    /// Optional explicit matrix/beam cells. Empty derives one cell per target
    /// fixture from its normalized 2D stage X/Z position.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<ColorMappingCellTarget>,
    pub playback_direction: ColorMappingPlaybackDirection,
    /// Duration of one animation pass. Still image/text sources keep this for
    /// Scene Live speed compatibility but always sample frame zero.
    pub period_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub phase: f32,
    pub offset_u: f32,
    pub offset_v: f32,
    pub scale_u: f32,
    pub scale_v: f32,
    pub rotation_degrees: f32,
    pub wrap_mode: ColorMappingWrapMode,
    pub sampling: ColorMappingSampling,
    pub blend_mode: EffectBlendMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EffectParamsSnapshot {
    Lfo(LfoEffectRequest),
    PositionWave(PositionWaveEffectRequest),
    Color(ColorEffectRequest),
    Chaser(ChaserEffectRequest),
    Move(MoveEffectRequest),
    Value(ValueEffectRequest),
    Curve(CurveEffectRequest),
    Mapping(MappingEffectRequest),
    ColorMapping(ColorMappingEffectRequest),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EffectKind {
    Lfo,
    PositionWave,
    Color,
    Chaser,
    Move,
    Value,
    Curve,
    Mapping,
    ColorMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectSummary {
    pub id: EffectId,
    pub label: String,
    pub effect_type: EffectKind,
    pub fixture_ids: Vec<FixtureId>,
    pub target_group_ids: Vec<String>,
    pub attribute: String,
    pub video_targets: Vec<VideoEffectTarget>,
    pub shape: LfoShape,
    pub period_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_sync: Option<EffectClockSync>,
    pub low: u16,
    pub high: u16,
    pub phase: f32,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub fixture_spread: f32,
    pub blend_mode: EffectBlendMode,
    pub origin: Option<Vec3>,
    pub direction: Option<Vec3>,
    pub speed: Option<f32>,
    pub wavelength: Option<f32>,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lfo: Option<LfoEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chaser: Option<ChaserEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_effect: Option<MoveEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<ValueEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve: Option<CurveEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping: Option<MappingEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_mapping: Option<ColorMappingEffectRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectPreset {
    pub version: u32,
    pub effect_type: EffectKind,
    pub enabled: bool,
    pub lfo: Option<LfoEffectRequest>,
    pub position_wave: Option<PositionWaveEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chaser: Option<ChaserEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_effect: Option<MoveEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<ValueEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve: Option<CurveEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping: Option<MappingEffectRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_mapping: Option<ColorMappingEffectRequest>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum DmxOutputProtocol {
    #[default]
    ArtNet,
    Sacn,
    EnttecUsbPro,
    DmxKingUltraDmx,
    EnttecOpenDmx,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DmxInputProtocol {
    ArtNet,
    Sacn,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DmxMergeMode {
    Htp,
    Ltp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DmxInputConfig {
    pub protocol: DmxInputProtocol,
    pub bind_ip: String,
    pub port: u16,
    pub universe: u16,
    #[serde(default = "default_true")]
    pub merge_enabled: bool,
    pub merge_mode: DmxMergeMode,
    pub timeout_ms: u64,
}

impl Default for DmxInputConfig {
    fn default() -> Self {
        Self {
            protocol: DmxInputProtocol::ArtNet,
            bind_ip: "0.0.0.0".to_string(),
            port: 6454,
            universe: 0,
            merge_enabled: true,
            merge_mode: DmxMergeMode::Htp,
            timeout_ms: 2_500,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DmxInputStatus {
    pub running: bool,
    pub signal_present: bool,
    pub packets_received: u64,
    pub invalid_packets: u64,
    pub last_packet_unix_ms: Option<u64>,
    pub source_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DmxOutputConfig {
    pub enabled: bool,
    pub protocol: DmxOutputProtocol,
    pub target_ip: String,
    pub port: u16,
    pub universe: u16,
    pub serial_port: String,
    pub serial_baud_rate: u32,
}

impl Default for DmxOutputConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port: 6454,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        }
    }
}

/// Runtime-only ownership for this machine's physical and external outputs.
/// This type is intentionally not part of `EngineSnapshot` or `ProjectFile`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MachineOutputRole {
    Lighting,
    Video,
    #[default]
    Both,
    Standby,
}

impl MachineOutputRole {
    pub const fn lighting_allowed(self) -> bool {
        matches!(self, Self::Lighting | Self::Both)
    }

    pub const fn video_allowed(self) -> bool {
        matches!(self, Self::Video | Self::Both)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutputOwnershipReason {
    OwnedByMachineRole,
    BlockedByMachineRole,
    Transitioning,
    TransitionFailed,
    ProjectSwapDisarmed,
    StartupDenied,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum OutputOwnershipState {
    #[default]
    Ready,
    Transitioning,
    Activating,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputOwnershipStatus {
    /// Effective role. This remains `Standby` while a transition is in flight or failed.
    pub role: MachineOutputRole,
    pub effective_role: MachineOutputRole,
    pub desired_role: MachineOutputRole,
    pub persisted_role: Option<MachineOutputRole>,
    pub state: OutputOwnershipState,
    pub generation: u64,
    pub epoch: u64,
    pub lighting_allowed: bool,
    pub video_allowed: bool,
    pub lighting_reason: OutputOwnershipReason,
    pub video_reason: OutputOwnershipReason,
    pub error: Option<String>,
}

impl Default for OutputOwnershipStatus {
    fn default() -> Self {
        Self::failed(
            MachineOutputRole::Standby,
            None,
            0,
            0,
            OutputOwnershipReason::StartupDenied,
            "Machine output ownership has not been initialized".to_string(),
        )
    }
}

impl OutputOwnershipStatus {
    pub fn for_role(role: MachineOutputRole) -> Self {
        Self::ready(role, Some(role), 0, 0)
    }

    pub fn ready(
        role: MachineOutputRole,
        persisted_role: Option<MachineOutputRole>,
        generation: u64,
        epoch: u64,
    ) -> Self {
        let lighting_allowed = role.lighting_allowed();
        let video_allowed = role.video_allowed();
        Self {
            role,
            effective_role: role,
            desired_role: role,
            persisted_role,
            state: OutputOwnershipState::Ready,
            generation,
            epoch,
            lighting_allowed,
            video_allowed,
            lighting_reason: if lighting_allowed {
                OutputOwnershipReason::OwnedByMachineRole
            } else {
                OutputOwnershipReason::BlockedByMachineRole
            },
            video_reason: if video_allowed {
                OutputOwnershipReason::OwnedByMachineRole
            } else {
                OutputOwnershipReason::BlockedByMachineRole
            },
            error: None,
        }
    }

    pub fn transitioning(
        desired_role: MachineOutputRole,
        persisted_role: Option<MachineOutputRole>,
        generation: u64,
        epoch: u64,
    ) -> Self {
        Self {
            role: MachineOutputRole::Standby,
            effective_role: MachineOutputRole::Standby,
            desired_role,
            persisted_role,
            state: OutputOwnershipState::Transitioning,
            generation,
            epoch,
            lighting_allowed: false,
            video_allowed: false,
            lighting_reason: OutputOwnershipReason::Transitioning,
            video_reason: OutputOwnershipReason::Transitioning,
            error: None,
        }
    }

    pub fn activating(
        desired_role: MachineOutputRole,
        persisted_role: Option<MachineOutputRole>,
        generation: u64,
        epoch: u64,
    ) -> Self {
        Self {
            role: MachineOutputRole::Standby,
            effective_role: MachineOutputRole::Standby,
            desired_role,
            persisted_role,
            state: OutputOwnershipState::Activating,
            generation,
            epoch,
            lighting_allowed: false,
            video_allowed: false,
            lighting_reason: OutputOwnershipReason::Transitioning,
            video_reason: OutputOwnershipReason::Transitioning,
            error: None,
        }
    }

    pub fn project_swap_disarmed(
        desired_role: MachineOutputRole,
        persisted_role: Option<MachineOutputRole>,
        generation: u64,
        epoch: u64,
    ) -> Self {
        Self {
            role: MachineOutputRole::Standby,
            effective_role: MachineOutputRole::Standby,
            desired_role,
            persisted_role,
            state: OutputOwnershipState::Ready,
            generation,
            epoch,
            lighting_allowed: false,
            video_allowed: false,
            lighting_reason: OutputOwnershipReason::ProjectSwapDisarmed,
            video_reason: OutputOwnershipReason::ProjectSwapDisarmed,
            error: None,
        }
    }

    pub fn failed(
        desired_role: MachineOutputRole,
        persisted_role: Option<MachineOutputRole>,
        generation: u64,
        epoch: u64,
        reason: OutputOwnershipReason,
        error: String,
    ) -> Self {
        Self {
            role: MachineOutputRole::Standby,
            effective_role: MachineOutputRole::Standby,
            desired_role,
            persisted_role,
            state: OutputOwnershipState::Failed,
            generation,
            epoch,
            lighting_allowed: false,
            video_allowed: false,
            lighting_reason: reason,
            video_reason: reason,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DmxOutputRouteTelemetry {
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub universe: u16,
    #[serde(default)]
    pub attempted: bool,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub bytes: usize,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub consecutive_failures: u32,
    #[serde(default)]
    pub reconnect_attempts: u64,
    #[serde(default)]
    pub reconnecting: bool,
    #[serde(default)]
    pub retry_in_ms: Option<u64>,
    #[serde(default)]
    pub last_success_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerialPortSummary {
    pub name: String,
    pub port_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usb_vid: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usb_pid: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    /// Windows PnP devnode that backs the current COM alias. This is an
    /// observation only; live output still requires the backend's exact
    /// approved-instance gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows_device_instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_protocol: Option<DmxOutputProtocol>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OscInputConfig {
    pub bind_ip: String,
    pub port: u16,
}

impl Default for OscInputConfig {
    fn default() -> Self {
        Self {
            bind_ip: "0.0.0.0".to_string(),
            port: 9_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OscControlAction {
    FixtureAttribute,
    SelectedFeatureFader,
    FixtureHighlight,
    FixtureSolo,
    FixturePark,
    GroupHighlight,
    GroupSolo,
    GroupPark,
    TriggerCue,
    TriggerNextCue,
    TriggerPreviousCue,
    EffectEnabled,
    NodeGraphEnabled,
    VideoParam,
    VideoCuePointAdd,
    VideoCuePointRemove,
    VideoCuePointJump,
    VideoCuePointPrevious,
    VideoCuePointNext,
    VideoLayerEnabled,
    VideoLayerSolo,
    VideoPlay,
    VideoLoop,
    VideoLayerFade,
    VideoOutputEnabled,
    VideoOutputOpacity,
    VideoOutputFade,
    VideoOutputMappingField,
    VideoOutputMappingPreset,
    VideoOutputBlackout,
    TimelinePlay,
    TimelineSeek,
    TimelineBeatPrevious,
    TimelineBeatNext,
    TimelineLoopToggle,
    TimelineLoopHalf,
    TimelineLoopDouble,
    SetBpm,
    TapBpm,
    LightingMaster,
    GroupSubmaster,
    CueFadePause,
    Blackout,
    AllBlackout,
    VideoMaster,
    VideoBlackout,
    ClearFixtureFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OscControlMapping {
    pub address: String,
    pub action: OscControlAction,
    pub fixture_id: Option<FixtureId>,
    pub attribute: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    pub cue_id: Option<CueId>,
    pub layer_id: Option<VideoLayerId>,
    #[serde(default)]
    pub output_id: Option<VideoOutputId>,
    pub video_param: Option<VideoParam>,
    pub cue_point_index: Option<usize>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    pub low: f32,
    pub high: f32,
}

pub type DmxControlAction = OscControlAction;

/// Maps one 1-based DMX input channel in an internal 0-based universe to a
/// backend-callable control. Keeping this typed (instead of encoding
/// `/dmx/...` as an OSC address) prevents the transports from triggering each
/// other's assignments. It shares the OSC action enum, so future backend
/// actions inherit the same typed persistence contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DmxControlMapping {
    pub universe: u16,
    pub channel: u16,
    pub action: DmxControlAction,
    pub fixture_id: Option<FixtureId>,
    pub attribute: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    pub cue_id: Option<CueId>,
    pub layer_id: Option<VideoLayerId>,
    #[serde(default)]
    pub output_id: Option<VideoOutputId>,
    pub video_param: Option<VideoParam>,
    pub cue_point_index: Option<usize>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    pub low: f32,
    pub high: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct LearnedDmxControl {
    pub universe: u16,
    pub channel: u16,
    pub value: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LearnedOscControl {
    pub address: String,
    pub value: Option<f32>,
    pub argument_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineTelemetry {
    pub frame_counter: u64,
    #[serde(default)]
    pub enabled_effect_count: usize,
    #[serde(default)]
    pub supported_effect_count: usize,
    #[serde(default)]
    pub effects_over_supported_envelope: bool,
    pub queue_depth: usize,
    #[serde(default)]
    pub queue_depth_abs_max: usize,
    #[serde(default)]
    pub queue_push_failure_count: u64,
    pub last_tick_interval_us: u64,
    #[serde(default)]
    pub tick_jitter_last_us: i64,
    #[serde(default)]
    pub tick_jitter_abs_max_us: u64,
    #[serde(default)]
    pub tick_jitter_stddev_us: f32,
    #[serde(default)]
    pub tick_jitter_p95_us: u64,
    #[serde(default)]
    pub tick_jitter_p99_us: u64,
    #[serde(default)]
    pub tick_jitter_samples: u64,
    #[serde(default)]
    pub last_command_queue_latency_us: u64,
    #[serde(default)]
    pub command_queue_latency_abs_max_us: u64,
    #[serde(default)]
    pub command_queue_latency_p95_us: u64,
    #[serde(default)]
    pub command_queue_latency_p99_us: u64,
    #[serde(default)]
    pub command_queue_latency_samples: u64,
    #[serde(default)]
    pub last_command_drain_count: usize,
    #[serde(default)]
    pub command_drain_abs_max: usize,
    #[serde(default)]
    pub command_drain_limit_hit_count: u64,
    #[serde(default)]
    pub last_command_to_dmx_tick_latency_us: u64,
    #[serde(default)]
    pub command_to_dmx_tick_latency_abs_max_us: u64,
    #[serde(default)]
    pub command_to_dmx_tick_latency_p95_us: u64,
    #[serde(default)]
    pub command_to_dmx_tick_latency_p99_us: u64,
    #[serde(default)]
    pub command_to_dmx_tick_latency_samples: u64,
    #[serde(default)]
    pub last_dmx_send_interval_us: u64,
    #[serde(default)]
    pub dmx_send_interval_min_us: u64,
    #[serde(default)]
    pub dmx_send_interval_max_us: u64,
    #[serde(default)]
    pub dmx_send_interval_samples: u64,
    #[serde(default)]
    pub low_latency_dmx_tick_request_count: u64,
    #[serde(default)]
    pub low_latency_dmx_tick_advance_count: u64,
    #[serde(default)]
    pub low_latency_dmx_tick_defer_count: u64,
    pub last_packet_bytes: usize,
    #[serde(default)]
    pub last_dmx_output_count: usize,
    #[serde(default)]
    pub last_dmx_send_success_count: usize,
    #[serde(default)]
    pub last_dmx_send_failure_count: usize,
    #[serde(default)]
    pub total_dmx_send_success_count: u64,
    #[serde(default)]
    pub total_dmx_send_failure_count: u64,
    #[serde(default)]
    pub last_dmx_route_results: Vec<DmxOutputRouteTelemetry>,
    pub last_error: Option<String>,
}

impl Default for EngineTelemetry {
    fn default() -> Self {
        Self {
            frame_counter: 0,
            enabled_effect_count: 0,
            supported_effect_count: 0,
            effects_over_supported_envelope: false,
            queue_depth: 0,
            queue_depth_abs_max: 0,
            queue_push_failure_count: 0,
            last_tick_interval_us: 0,
            tick_jitter_last_us: 0,
            tick_jitter_abs_max_us: 0,
            tick_jitter_stddev_us: 0.0,
            tick_jitter_p95_us: 0,
            tick_jitter_p99_us: 0,
            tick_jitter_samples: 0,
            last_command_queue_latency_us: 0,
            command_queue_latency_abs_max_us: 0,
            command_queue_latency_p95_us: 0,
            command_queue_latency_p99_us: 0,
            command_queue_latency_samples: 0,
            last_command_drain_count: 0,
            command_drain_abs_max: 0,
            command_drain_limit_hit_count: 0,
            last_command_to_dmx_tick_latency_us: 0,
            command_to_dmx_tick_latency_abs_max_us: 0,
            command_to_dmx_tick_latency_p95_us: 0,
            command_to_dmx_tick_latency_p99_us: 0,
            command_to_dmx_tick_latency_samples: 0,
            last_dmx_send_interval_us: 0,
            dmx_send_interval_min_us: 0,
            dmx_send_interval_max_us: 0,
            dmx_send_interval_samples: 0,
            low_latency_dmx_tick_request_count: 0,
            low_latency_dmx_tick_advance_count: 0,
            low_latency_dmx_tick_defer_count: 0,
            last_packet_bytes: 0,
            last_dmx_output_count: 0,
            last_dmx_send_success_count: 0,
            last_dmx_send_failure_count: 0,
            total_dmx_send_success_count: 0,
            total_dmx_send_failure_count: 0,
            last_dmx_route_results: Vec::new(),
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MidiInputSummary {
    pub index: usize,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MidiOutputSummary {
    pub index: usize,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MidiControlMessage {
    NoteOn,
    NoteOff,
    ControlChange,
    ProgramChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MidiFeedbackMessage {
    pub message: MidiControlMessage,
    pub channel: u8,
    pub number: u8,
    pub value: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MidiControlFeedback {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off: Option<MidiFeedbackMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<MidiFeedbackMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unknown: Option<MidiFeedbackMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MidiControlAction {
    FixtureAttribute,
    SelectedFeatureFader,
    FixtureHighlight,
    FixtureSolo,
    FixturePark,
    GroupHighlight,
    GroupSolo,
    GroupPark,
    TriggerCue,
    FlashCue,
    TriggerCueDirection,
    FlashCueDirection,
    TriggerCueListNext,
    TriggerNextCue,
    TriggerPreviousCue,
    EffectEnabled,
    NodeGraphEnabled,
    VideoParam,
    VideoCuePointAdd,
    VideoCuePointRemove,
    VideoCuePointJump,
    VideoCuePointPrevious,
    VideoCuePointNext,
    VideoLayerEnabled,
    VideoLayerSolo,
    VideoPlay,
    VideoLoop,
    VideoLayerFade,
    VideoOutputEnabled,
    VideoOutputOpacity,
    VideoOutputFade,
    VideoOutputMappingField,
    VideoOutputMappingPreset,
    VideoOutputBlackout,
    TimelinePlay,
    TimelineSeek,
    TimelineBeatPrevious,
    TimelineBeatNext,
    TimelineLoopToggle,
    TimelineLoopHalf,
    TimelineLoopDouble,
    SetBpm,
    TapBpm,
    LightingMaster,
    GroupSubmaster,
    CueFadePause,
    Blackout,
    AllBlackout,
    VideoMaster,
    VideoBlackout,
    ClearFixtureFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MidiControlMapping {
    pub channel: Option<u8>,
    pub message: MidiControlMessage,
    pub number: u8,
    pub action: MidiControlAction,
    pub fixture_id: Option<FixtureId>,
    pub attribute: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    pub cue_id: Option<CueId>,
    pub layer_id: Option<VideoLayerId>,
    #[serde(default)]
    pub output_id: Option<VideoOutputId>,
    pub video_param: Option<VideoParam>,
    pub cue_point_index: Option<usize>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback: Option<MidiControlFeedback>,
    pub low: f32,
    pub high: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LearnedMidiControl {
    pub channel: u8,
    pub message: MidiControlMessage,
    pub number: u8,
    pub value: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ClockSource {
    #[default]
    Manual,
    Tap,
    MidiClock,
    MidiTimecode,
    Ltc,
    AbletonLink,
    DjLink,
}

/// `Debug` is implemented manually for this type so the generic Web Remote
/// pairing PIN and the DJ Link authority token can never be rendered into
/// logs, crash reports, or test output.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteControlConfig {
    pub bind_ip: String,
    pub port: u16,
    #[serde(default)]
    pub pairing_pin: String,
    #[serde(default)]
    pub allow_lan: bool,
    #[serde(default = "default_remote_max_connections")]
    pub max_connections: u16,
    #[serde(default = "default_remote_max_message_bytes")]
    pub max_message_bytes: usize,
    #[serde(default = "default_remote_max_messages_per_second")]
    pub max_messages_per_second: u16,
    /// Generic Web Remote `/ws` clients plus the HTTP asset/health routes.
    /// Legacy configs predate this switch, so absence deserializes as
    /// enabled and current behavior is preserved unchanged.
    #[serde(default = "default_web_remote_enabled")]
    pub web_remote_enabled: bool,
    /// Dedicated DJ Link is opt-in and never uses the generic pairing PIN.
    #[serde(default)]
    pub dj_link_enabled: bool,
    #[serde(default)]
    pub dj_link_bind_ip: Option<String>,
    /// Machine-local secret. It is injected by the backend after the public
    /// config has been deserialized and is never accepted from project or UI
    /// input.  Skipping both directions prevents a renderer from selecting
    /// its own authority credential.
    #[serde(skip)]
    pub dj_link_token: Option<String>,
}

/// Runtime-only operator targeting state shared by the desktop UI, MIDI/OSC,
/// Remote WebSocket clients, and future automation clients. The attribute
/// order is the authoritative visible fader order; it is never persisted in a
/// project file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorSelectionContext {
    #[serde(default)]
    pub fixture_ids: Vec<FixtureId>,
    #[serde(default)]
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorFeatureFaderResult {
    pub target_index: usize,
    pub attribute: String,
    pub fixture_ids: Vec<FixtureId>,
    pub value: u16,
}

fn default_remote_max_connections() -> u16 {
    8
}

fn default_remote_max_message_bytes() -> usize {
    64 * 1024
}

fn default_remote_max_messages_per_second() -> u16 {
    60
}

fn default_web_remote_enabled() -> bool {
    true
}

impl std::fmt::Debug for RemoteControlConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Presence-only credential rendering: neither the pairing PIN nor the
        // DJ Link token value is ever exposed, only whether one is set.
        let pin_presence = if self.pairing_pin.is_empty() {
            "unset"
        } else {
            "redacted"
        };
        let token_presence = match self.dj_link_token.as_deref() {
            Some(token) if !token.is_empty() => "redacted",
            _ => "unset",
        };
        formatter
            .debug_struct("RemoteControlConfig")
            .field("bind_ip", &self.bind_ip)
            .field("port", &self.port)
            .field("pairing_pin", &pin_presence)
            .field("allow_lan", &self.allow_lan)
            .field("max_connections", &self.max_connections)
            .field("max_message_bytes", &self.max_message_bytes)
            .field("max_messages_per_second", &self.max_messages_per_second)
            .field("web_remote_enabled", &self.web_remote_enabled)
            .field("dj_link_enabled", &self.dj_link_enabled)
            .field("dj_link_bind_ip", &self.dj_link_bind_ip)
            .field("dj_link_token", &token_presence)
            .finish()
    }
}

impl Default for RemoteControlConfig {
    fn default() -> Self {
        Self {
            bind_ip: "127.0.0.1".to_string(),
            port: 9_100,
            pairing_pin: String::new(),
            allow_lan: false,
            max_connections: default_remote_max_connections(),
            max_message_bytes: default_remote_max_message_bytes(),
            max_messages_per_second: default_remote_max_messages_per_second(),
            web_remote_enabled: default_web_remote_enabled(),
            dj_link_enabled: false,
            dj_link_bind_ip: None,
            dj_link_token: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteClientSummary {
    pub id: u64,
    pub peer_addr: String,
    pub connected_at_unix_ms: u64,
    pub last_activity_unix_ms: u64,
    pub messages_received: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RemoteControlStatus {
    pub running: bool,
    pub active_connections: usize,
    pub rejected_connections: u64,
    pub clients: Vec<RemoteClientSummary>,
    /// Immutable transport mode of the owning listener. A status produced
    /// without a running server (or a legacy payload missing this field)
    /// reports `false`: no generic Web Remote transport is available.
    #[serde(default)]
    pub web_remote_enabled: bool,
    /// Immutable DJ Link transport mode of the owning listener. This is
    /// separate from `dj_link` connection telemetry so a DJ-only listener is
    /// never projected as a generic Web Remote listener in the UI.
    #[serde(default)]
    pub dj_link_enabled: bool,
    /// Additive process-local DJ Link truth.  The token is intentionally not
    /// represented anywhere in this status DTO.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dj_link: Option<DjLinkRuntimeStatus>,
}

/// DJ Link is a dedicated, authenticated LAN protocol. It intentionally does
/// not reuse the generic remote PIN or expose the credential in any status
/// payload.
pub const DJ_LINK_PROTOCOL_VERSION: u8 = 3;
/// The only peer identity production ingress accepts. Anything else fails
/// envelope validation before authentication is attempted.
pub const DJ_LINK_AGENT_ID: &str = "rb-output-dj-agent";
pub const DJ_LINK_MAX_FRAME_BYTES: usize = 64 * 1024;
pub const DJ_LINK_MAX_STRING_BYTES: usize = 256;
pub const DJ_LINK_MIN_TOKEN_BYTES: usize = 32;
pub const DJ_LINK_MAX_MAPPINGS: usize = 128;
pub const DJ_LINK_MAX_SEQUENCE: u64 = 9_007_199_254_740_991;
pub const DJ_LINK_BEATS_PER_BAR: u8 = 4;
/// Exact capability strings every HELLO must advertise. The list must be
/// complete, duplicate-free, and carry no extras; partial capability
/// negotiation does not exist on this wire revision.
pub const DJ_LINK_REQUIRED_CAPABILITIES: [&str; 10] = [
    "DJ_TRACK_ACTIVE",
    "DJ_TRACK_SYNC",
    "DJ_LOOP_STATE",
    "DJ_LOOP_FALLBACK",
    "DJ_RELEASE",
    "DJ_TIMELINE_BEAT_JUMP",
    "DJ_TIMELINE_LOOP_SET",
    "DJ_TIMELINE_LOOP_HALF",
    "DJ_TIMELINE_STATE_REQUEST",
    "DJ_STATE_SYNC",
];
/// Mandatory source discriminator of every measured loop report.
pub const DJ_LINK_MEASURED_LOOP_SOURCE: &str = "rekordbox-hook-measured";
/// Exact source discriminator for the bounded Stage-1 pedal fallback. This is
/// deliberately distinct from a measured Rekordbox report.
pub const DJ_LINK_LOOP_FALLBACK_SOURCE: &str = "pedal-no-response-predicted";
/// Official Rekordbox Beat Loop profile used by the DSF show. Values are
/// exact dyadic fractions, so callers must never round a nearby value into
/// this set.
pub const DJ_LINK_LOOP_PROFILE_LENGTH_BEATS: [f64; 10] = [
    8.0,
    4.0,
    2.0,
    1.0,
    1.0 / 2.0,
    1.0 / 4.0,
    1.0 / 8.0,
    1.0 / 16.0,
    1.0 / 32.0,
    1.0 / 64.0,
];
pub const DJ_LINK_LOOP_FALLBACK_MIN_RESPONSE_WINDOW_MS: u64 = 50;
pub const DJ_LINK_LOOP_FALLBACK_MAX_RESPONSE_WINDOW_MS: u64 = 1_500;
pub const DJ_LINK_MAX_BPM: f64 = 1000.0;
pub const DJ_LINK_MAX_POSITION_AT_SEND_SEC: f64 = 7200.0;
pub const DJ_LINK_MAX_SAMPLE_AGE_MS: u64 = 1500;
/// Inclusive consistency tolerance between `lengthBeats` and the reported
/// beat span of an active measured loop.
pub const DJ_LINK_LOOP_LENGTH_TOLERANCE_BEATS: f64 = 0.001;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DjLinkMessageType {
    #[serde(rename = "DJ_AGENT_HELLO")]
    Hello,
    #[serde(rename = "DJ_HEARTBEAT")]
    Heartbeat,
    #[serde(rename = "DJ_TRACK_ACTIVE")]
    TrackActive,
    #[serde(rename = "DJ_TRACK_SYNC")]
    TrackSync,
    #[serde(rename = "DJ_LOOP_STATE")]
    LoopState,
    #[serde(rename = "DJ_LOOP_FALLBACK")]
    LoopFallback,
    #[serde(rename = "DJ_RELEASE")]
    Release,
    #[serde(rename = "DJ_STATE_SYNC")]
    StateSync,
    #[serde(rename = "DJ_TIMELINE_STATE_REQUEST")]
    TimelineStateRequest,
    #[serde(rename = "DJ_TIMELINE_BEAT_JUMP")]
    TimelineBeatJump,
    #[serde(rename = "DJ_TIMELINE_LOOP_SET")]
    TimelineLoopSet,
    #[serde(rename = "DJ_TIMELINE_LOOP_HALF")]
    TimelineLoopHalf,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkEnvelope {
    pub v: u8,
    #[serde(rename = "type")]
    pub message_type: DjLinkMessageType,
    #[serde(rename = "agentId")]
    pub agent_id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub sequence: u64,
    #[serde(rename = "eventId")]
    pub event_id: String,
    pub payload: serde_json::Value,
}

/// HELLO payloads contain a bearer credential. Keep every envelope payload
/// out of diagnostics so formatting an ingress error can never disclose it.
impl std::fmt::Debug for DjLinkEnvelope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DjLinkEnvelope")
            .field("v", &self.v)
            .field("message_type", &self.message_type)
            .field("agent_id", &self.agent_id)
            .field("session_id", &self.session_id)
            .field("sequence", &self.sequence)
            .field("event_id", &self.event_id)
            .field("payload", &"REDACTED")
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkHelloPayload {
    #[serde(rename = "authToken")]
    pub auth_token: String,
    pub version: u8,
    pub capabilities: Vec<String>,
}

impl std::fmt::Debug for DjLinkHelloPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DjLinkHelloPayload")
            .field("auth_token", &"REDACTED")
            .field("version", &self.version)
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct DjLinkHeartbeatPayload {}

/// Exact measured-loop report carried by per-deck track and loop-state frames.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkMeasuredLoop {
    pub active: bool,
    #[serde(rename = "startBeat")]
    pub start_beat: Option<f64>,
    #[serde(rename = "endBeat")]
    pub end_beat: Option<f64>,
    #[serde(rename = "lengthBeats")]
    pub length_beats: Option<f64>,
    pub revision: u64,
    #[serde(rename = "sampleAgeMs")]
    pub sample_age_ms: u64,
    pub source: String,
}

/// Strict per-deck track report used by both `DJ_TRACK_ACTIVE` and
/// `DJ_TRACK_SYNC`. Identity is exactly one nonempty `contentId` or both
/// `title` and `artist`; Master state is deliberately absent from this route.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkTrackPayload {
    pub deck: u8,
    #[serde(rename = "deckId")]
    pub deck_id: String,
    #[serde(default, rename = "contentId")]
    pub content_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default, rename = "trackBpm")]
    pub track_bpm: Option<f64>,
    #[serde(rename = "positionAtSendSec")]
    pub position_at_send_sec: f64,
    #[serde(rename = "effectiveBpm")]
    pub effective_bpm: f64,
    #[serde(rename = "positionRevision")]
    pub position_revision: u64,
    #[serde(rename = "sampleAgeMs")]
    pub sample_age_ms: u64,
    #[serde(rename = "isPlaying")]
    pub is_playing: bool,
    #[serde(rename = "startedAt")]
    pub started_at: String,
    #[serde(rename = "playSessionId")]
    pub play_session_id: String,
    #[serde(default, rename = "loop")]
    pub loop_state: Option<DjLinkMeasuredLoop>,
}

/// Canonical measured-loop event for an admitted per-deck route. It binds to
/// the exact deck and play session, never to a Master-deck revision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkTrackLoopStatePayload {
    pub deck: u8,
    #[serde(rename = "deckId")]
    pub deck_id: String,
    #[serde(rename = "playSessionId")]
    pub play_session_id: String,
    #[serde(rename = "loop")]
    pub loop_state: DjLinkMeasuredLoop,
}

/// Canonical per-deck bounded absolute loop target emitted only after an F14
/// response window expires without a fresh valid measured loop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkTrackLoopFallbackPayload {
    pub deck: u8,
    #[serde(rename = "deckId")]
    pub deck_id: String,
    #[serde(rename = "playSessionId")]
    pub play_session_id: String,
    /// Monotonic physical F14 intent identity. A fallback may consume this
    /// identity only once for the current deck/session runtime.
    #[serde(rename = "pedalIntentId")]
    pub pedal_intent_id: u64,
    #[serde(rename = "baseMeasuredLoopRevision")]
    pub base_measured_loop_revision: Option<u64>,
    #[serde(rename = "baseLoopDivision")]
    pub base_loop_division: Option<u8>,
    #[serde(rename = "targetLengthBeats")]
    pub target_length_beats: f64,
    #[serde(rename = "responseWindowMs")]
    pub response_window_ms: u64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkReleasePayload {
    pub state: String,
    #[serde(rename = "timelineId")]
    pub timeline_id: String,
    #[serde(rename = "playSessionId")]
    pub play_session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkTrackStateSyncPayload {
    pub released: bool,
    #[serde(default, rename = "ownerDeck")]
    pub owner_deck: Option<u8>,
    #[serde(default, rename = "ownerDeckId")]
    pub owner_deck_id: Option<String>,
    #[serde(default, rename = "activePlaySessionId")]
    pub active_play_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct DjLinkTimelineStateRequestPayload {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkTimelineBeatJumpPayload {
    pub bars: i8,
    #[serde(rename = "timelineId")]
    pub timeline_id: String,
    #[serde(rename = "playSessionId")]
    pub play_session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkTimelineLoopSetPayload {
    pub active: bool,
    #[serde(rename = "timelineId")]
    pub timeline_id: String,
    #[serde(rename = "playSessionId")]
    pub play_session_id: String,
}

/// Stage-2 F14: halve the currently active Timeline loop.  This carries only
/// the exact authority pair; loop shape is always derived in the engine from
/// the current runtime image and never accepted from the peer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkTimelineLoopHalfPayload {
    #[serde(rename = "timelineId")]
    pub timeline_id: String,
    #[serde(rename = "playSessionId")]
    pub play_session_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DjLinkTimelineStateValue {
    Idle,
    Running,
    Stopped,
    Ended,
    Reset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkTimelineState {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(rename = "eventId")]
    pub event_id: String,
    pub sequence: u64,
    pub state: DjLinkTimelineStateValue,
    #[serde(rename = "loopActive")]
    pub loop_active: bool,
    /// Strict runtime truth for the post-Follow, pedal-released destination
    /// hold. This is deliberately distinct from an authored A-B loop.
    #[serde(rename = "transitionHoldActive")]
    pub transition_hold_active: bool,
    #[serde(rename = "timelineId")]
    pub timeline_id: String,
    #[serde(rename = "positionBars")]
    pub position_bars: u64,
    #[serde(rename = "playSessionId", default)]
    pub play_session_id: Option<String>,
    #[serde(rename = "pedalOwner", default)]
    pub pedal_owner: Option<String>,
    #[serde(rename = "releaseEventId", default)]
    pub release_event_id: Option<String>,
    /// Explicit operator reconciliation correlation. Ordinary authoritative
    /// state frames carry `null`; only the confirmed Syndocal-side return
    /// action assigns a fresh process-epoch/counter request identity.
    #[serde(
        rename = "operatorReturnRequestId",
        deserialize_with = "deserialize_required_nullable_dj_link_string"
    )]
    pub operator_return_request_id: Option<String>,
}

fn deserialize_required_nullable_dj_link_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

impl DjLinkTimelineState {
    pub fn validate(&self) -> Result<(), String> {
        if self.message_type != "DJ_TIMELINE_STATE" {
            return Err("DJ timeline state type must be DJ_TIMELINE_STATE".to_string());
        }
        validate_dj_link_string(&self.event_id, "eventId")?;
        if self.sequence == 0 || self.sequence > DJ_LINK_MAX_SEQUENCE {
            return Err("DJ timeline state sequence must be a positive safe integer".to_string());
        }
        validate_dj_link_string(&self.timeline_id, "timelineId")?;
        for (value, label) in [
            (self.play_session_id.as_deref(), "playSessionId"),
            (self.pedal_owner.as_deref(), "pedalOwner"),
            (self.release_event_id.as_deref(), "releaseEventId"),
        ] {
            if let Some(value) = value {
                validate_dj_link_string(value, label)?;
            }
        }
        if let Some(value) = self.operator_return_request_id.as_deref() {
            validate_dj_link_operator_return_request_id(value)?;
        }
        Ok(())
    }
}

const DJ_LINK_OPERATOR_RETURN_REQUEST_PREFIX: &str = "syndocal-dj-operator-return-";
const DJ_LINK_OPERATOR_RETURN_EPOCH_HEX_LEN: usize = 32;

/// Validate the one canonical operator-return identity shape. The opaque
/// lowercase epoch changes only with the authoritative Syndocal process while
/// the decimal counter remains monotonic for that process. This lets the Agent
/// reject reconnect replay without suppressing a fresh process whose counter
/// legitimately starts again at one.
pub fn validate_dj_link_operator_return_request_id(value: &str) -> Result<(), String> {
    validate_dj_link_string(value, "operatorReturnRequestId")?;
    let remainder = value
        .strip_prefix(DJ_LINK_OPERATOR_RETURN_REQUEST_PREFIX)
        .ok_or_else(|| {
            "operatorReturnRequestId must use the canonical Syndocal epoch/counter form".to_string()
        })?;
    let (epoch, counter) = remainder.rsplit_once('-').ok_or_else(|| {
        "operatorReturnRequestId must use the canonical Syndocal epoch/counter form".to_string()
    })?;
    if epoch.len() != DJ_LINK_OPERATOR_RETURN_EPOCH_HEX_LEN
        || !epoch
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(
            "operatorReturnRequestId epoch must be exactly 32 lowercase hexadecimal characters"
                .to_string(),
        );
    }
    if counter.is_empty()
        || counter.starts_with('0')
        || !counter.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(
            "operatorReturnRequestId counter must be a canonical positive decimal integer"
                .to_string(),
        );
    }
    let parsed = counter.parse::<u64>().map_err(|_| {
        "operatorReturnRequestId counter exceeds the supported unsigned 64-bit range".to_string()
    })?;
    if parsed == 0 {
        return Err(
            "operatorReturnRequestId counter must be a canonical positive decimal integer"
                .to_string(),
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DjLinkAckOutcome {
    Accepted,
    Duplicate,
    NoMapping,
    Rejected,
    Busy,
}

/// Exact non-envelope ACK frame. The serialized shape is exactly
/// `{v,type,eventId,sequence,outcome,code,stateGeneration}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkAck {
    pub v: u8,
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(rename = "eventId")]
    pub event_id: String,
    pub sequence: u64,
    pub outcome: DjLinkAckOutcome,
    pub code: Option<String>,
    #[serde(rename = "stateGeneration")]
    pub state_generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DjLinkRuntimeStatus {
    pub connected: bool,
    pub peer: Option<String>,
    pub generation: u64,
    #[serde(rename = "agentId")]
    pub agent_id: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    #[serde(rename = "trackActive")]
    pub track_active: bool,
    #[serde(rename = "loopDivision")]
    pub loop_division: Option<u8>,
    pub released: bool,
    #[serde(rename = "lastEventId")]
    pub last_event_id: Option<String>,
    #[serde(rename = "ageMs")]
    pub age_ms: Option<u64>,
    /// The deck that owns the admitted per-track route.
    #[serde(default, rename = "ownerDeck")]
    pub owner_deck: Option<String>,
    #[serde(default, rename = "trackContentId")]
    pub track_content_id: Option<String>,
    #[serde(default, rename = "trackTitle")]
    pub track_title: Option<String>,
    #[serde(default, rename = "trackArtist")]
    pub track_artist: Option<String>,
    #[serde(default, rename = "trackDeckId")]
    pub track_deck_id: Option<String>,
    #[serde(default, rename = "trackStartedAt")]
    pub track_started_at: Option<String>,
    #[serde(default, rename = "trackPlaying")]
    pub track_playing: bool,
    #[serde(default, rename = "trackBpm")]
    pub track_bpm: Option<f64>,
    #[serde(default, rename = "positionSec")]
    pub position_sec: Option<f64>,
    #[serde(default, rename = "snapshotReady")]
    pub snapshot_ready: bool,
    #[serde(default, rename = "authoritativeState")]
    pub authoritative_state: Option<DjLinkTimelineStateValue>,
    #[serde(default, rename = "timelineId")]
    pub timeline_id: Option<String>,
    #[serde(default, rename = "playSessionId")]
    pub play_session_id: Option<String>,
    #[serde(default, rename = "pedalOwner")]
    pub pedal_owner: Option<String>,
    #[serde(default, rename = "releaseEventId")]
    pub release_event_id: Option<String>,
    #[serde(default, rename = "positionBars")]
    pub position_bars: Option<u64>,
    #[serde(default, rename = "loopActive")]
    pub loop_active: bool,
    #[serde(default, rename = "lastOutboundEventId")]
    pub last_outbound_event_id: Option<String>,
    #[serde(default, rename = "lastOutboundSequence")]
    pub last_outbound_sequence: Option<u64>,
    #[serde(default, rename = "lastOutboundDelivery")]
    pub last_outbound_delivery: Option<String>,
    #[serde(default, rename = "lastOperatorReturnRequestId")]
    pub last_operator_return_request_id: Option<String>,
    #[serde(default, rename = "lastOperatorReturnSequence")]
    pub last_operator_return_sequence: Option<u64>,
    #[serde(default, rename = "lastOperatorReturnDelivery")]
    pub last_operator_return_delivery: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DjTrackRetriggerPolicy {
    #[default]
    OncePerPlaySession,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjTrackSelector {
    #[serde(default, rename = "contentId")]
    pub content_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    /// Case-sensitive normalized title substring for a production DJ cue.
    /// This selector is deliberately exclusive with the exact identities.
    #[serde(default, rename = "titleContains")]
    pub title_contains: Option<String>,
    /// Explicit fallback deck if no primary exact/titleContains selector
    /// matches. The show policy currently admits only deck 1.
    #[serde(default, rename = "fallbackDeck")]
    pub fallback_deck: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjTrackTriggerMapping {
    pub id: String,
    pub selector: DjTrackSelector,
    #[serde(rename = "timelineId")]
    pub timeline_id: TimelineId,
    #[serde(default)]
    pub retrigger: DjTrackRetriggerPolicy,
}

impl DjTrackSelector {
    pub fn canonical_key(&self) -> Result<String, String> {
        let title_contains = self
            .title_contains
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(title_contains) = title_contains {
            if self.content_id.is_some() || self.title.is_some() || self.artist.is_some() {
                return Err(
                    "DJ titleContains selector must not include contentId, title, or artist"
                        .to_string(),
                );
            }
            if let Some(fallback_deck) = self.fallback_deck {
                if fallback_deck != 1 {
                    return Err(
                        "DJ titleContains fallbackDeck must be exactly deck 1 for this show"
                            .to_string(),
                    );
                }
            }
            let title_contains = title_contains.nfc().collect::<String>();
            validate_dj_link_string(&title_contains, "normalized titleContains")?;
            return Ok(format!("title_contains:{title_contains}"));
        }
        if self.fallback_deck.is_some() {
            return Err("DJ fallbackDeck requires titleContains".to_string());
        }
        let content_id = self
            .content_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(content_id) = content_id {
            validate_dj_link_string(content_id, "contentId")?;
            return Ok(format!("content:{content_id}"));
        }
        let title = self
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "DJ track selector requires contentId or title+artist".to_string())?;
        let artist = self
            .artist
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "DJ track selector requires both title and artist".to_string())?;
        validate_dj_link_string(title, "title")?;
        validate_dj_link_string(artist, "artist")?;
        let title = title.nfc().collect::<String>();
        let artist = artist.nfc().collect::<String>();
        validate_dj_link_string(&title, "normalized title")?;
        validate_dj_link_string(&artist, "normalized artist")?;
        Ok(format!("title_artist:{title}\u{001f}{artist}"))
    }
}

pub fn validate_dj_track_trigger_mappings(
    mappings: &[DjTrackTriggerMapping],
) -> Result<(), String> {
    if mappings.len() > DJ_LINK_MAX_MAPPINGS {
        return Err(format!(
            "DJ track mapping count exceeds {DJ_LINK_MAX_MAPPINGS}"
        ));
    }
    let mut ids = BTreeSet::new();
    let mut selectors = BTreeSet::new();
    for mapping in mappings {
        validate_dj_link_string(&mapping.id, "mapping id")?;
        if mapping.id.trim().is_empty() || !ids.insert(mapping.id.clone()) {
            return Err("DJ track mapping IDs must be non-empty and unique".to_string());
        }
        let selector = mapping.selector.canonical_key()?;
        if !selectors.insert(selector) {
            return Err("DJ track selectors must be unique and unambiguous".to_string());
        }
        if mapping.timeline_id.0 == 0 {
            return Err("DJ track mapping timelineId must be non-zero".to_string());
        }
    }
    Ok(())
}

fn validate_dj_link_string(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > DJ_LINK_MAX_STRING_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(format!(
            "DJ Link {label} is empty, too long, or contains control characters"
        ));
    }
    Ok(())
}

/// A session fence is an exact opaque identity, not display text. Preserve
/// meaningful interior whitespace on the wire, but reject any representation
/// that could normalize to the same identity after transport or UI handling.
fn validate_dj_link_required_identity(value: &str, label: &str) -> Result<(), String> {
    validate_dj_link_string(value, label)?;
    if value.trim().is_empty() || value != value.trim() {
        return Err(format!(
            "DJ Link {label} must be non-empty and have no leading or trailing whitespace"
        ));
    }
    Ok(())
}

/// Serde maps both a missing `Option<T>` field and an explicit JSON `null` to
/// `None`. The generic per-deck route needs the stronger wire distinction for
/// identity: selected keys must be present strings, while every non-selected
/// identity key must be absent. `trackBpm` and the track-level `loop` are the
/// only generic-track optional fields and intentionally retain omitted/null
/// compatibility.
fn validate_dj_link_generic_track_raw_shape(payload: &serde_json::Value) -> Result<(), String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "generic DJ track payload must be an object".to_string())?;
    match (
        object.get("contentId"),
        object.get("title"),
        object.get("artist"),
    ) {
        (Some(content_id), None, None) if content_id.is_string() => Ok(()),
        (None, Some(title), Some(artist)) if title.is_string() && artist.is_string() => Ok(()),
        _ => Err(
            "generic DJ track identity must be contentId alone or title+artist with non-selected keys absent"
                .to_string(),
        ),
    }
}

/// Generic StateSync owner context is all-or-nothing on the raw wire. This
/// fence runs before `Option` deserialization so explicit null cannot be
/// laundered into the omitted-owner diagnostic form. `ownerDeck` remains the
/// exact numeric deck identifier; the paired deck and session identities are
/// exact strings and receive their semantic validation after deserialization.
fn validate_dj_link_generic_state_sync_raw_shape(
    payload: &serde_json::Value,
) -> Result<(), String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "generic DJ state sync payload must be an object".to_string())?;
    match (
        object.get("ownerDeck"),
        object.get("ownerDeckId"),
        object.get("activePlaySessionId"),
    ) {
        (None, None, None) => Ok(()),
        (Some(deck), Some(deck_id), Some(play_session_id))
            if deck.as_u64().is_some() && deck_id.is_string() && play_session_id.is_string() =>
        {
            Ok(())
        }
        _ => Err(
            "generic DJ state sync must omit owner context or provide non-null ownerDeck,ownerDeckId,activePlaySessionId together"
                .to_string(),
        ),
    }
}

impl DjLinkEnvelope {
    /// Ingress is exact-only. The raw text is first scanned for duplicate
    /// object keys at every nesting level (serde alone would silently keep
    /// the last occurrence), then deserialized into the exact v3 envelope
    /// shape with unknown-field rejection, then fully payload-validated.
    pub fn parse_json(text: &str) -> Result<Self, String> {
        if text.len() > DJ_LINK_MAX_FRAME_BYTES {
            return Err("DJ Link frame exceeds the bounded size".to_string());
        }
        reject_duplicate_json_keys(text)?;
        let envelope: Self = serde_json::from_str(text).map_err(|error| error.to_string())?;
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.v != DJ_LINK_PROTOCOL_VERSION {
            return Err("unsupported DJ Link protocol version".to_string());
        }
        if self.agent_id != DJ_LINK_AGENT_ID {
            return Err("DJ Link agentId is not the accepted peer identity".to_string());
        }
        validate_dj_link_string(&self.session_id, "sessionId")?;
        validate_dj_link_string(&self.event_id, "eventId")?;
        if self.sequence == 0 || self.sequence > DJ_LINK_MAX_SEQUENCE {
            return Err("DJ Link sequence must be a positive safe integer".to_string());
        }
        if !self.payload.is_object() {
            return Err("DJ Link payload must be an object".to_string());
        }
        match self.message_type {
            DjLinkMessageType::Hello => {
                let hello: DjLinkHelloPayload = parse_dj_link_payload(&self.payload)?;
                if hello.version != DJ_LINK_PROTOCOL_VERSION {
                    return Err("DJ Link HELLO version must equal the protocol version".to_string());
                }
                validate_dj_link_token(&hello.auth_token)?;
                validate_dj_link_capability_set(&hello.capabilities)?;
            }
            DjLinkMessageType::Heartbeat => {
                let _: DjLinkHeartbeatPayload = parse_dj_link_payload(&self.payload)?;
            }
            DjLinkMessageType::TrackActive | DjLinkMessageType::TrackSync => {
                validate_dj_link_generic_track_raw_shape(&self.payload)?;
                let payload: DjLinkTrackPayload = parse_dj_link_payload(&self.payload)?;
                validate_dj_link_track_payload(&payload)?;
            }
            DjLinkMessageType::LoopState => {
                let payload: DjLinkTrackLoopStatePayload = parse_dj_link_payload(&self.payload)?;
                payload.validate()?;
            }
            DjLinkMessageType::LoopFallback => {
                let payload: DjLinkTrackLoopFallbackPayload = parse_dj_link_payload(&self.payload)?;
                payload.validate()?;
            }
            DjLinkMessageType::Release => {
                let payload: DjLinkReleasePayload = parse_dj_link_payload(&self.payload)?;
                if payload.state != "released" {
                    return Err("DJ release state must be exactly released".to_string());
                }
                validate_dj_link_string(&payload.timeline_id, "timelineId")?;
                validate_dj_link_string(&payload.play_session_id, "playSessionId")?;
            }
            DjLinkMessageType::StateSync => {
                let owner_context_fields = ["ownerDeck", "ownerDeckId", "activePlaySessionId"]
                    .into_iter()
                    .filter(|field| self.payload.get(*field).is_some())
                    .count();
                if !matches!(owner_context_fields, 0 | 3) {
                    return Err(
                        "generic DJ state sync must omit owner context or provide ownerDeck,ownerDeckId,activePlaySessionId together"
                            .to_string(),
                    );
                }
                validate_dj_link_generic_state_sync_raw_shape(&self.payload)?;
                let payload: DjLinkTrackStateSyncPayload = parse_dj_link_payload(&self.payload)?;
                payload.validate()?;
            }
            DjLinkMessageType::TimelineStateRequest => {
                let _: DjLinkTimelineStateRequestPayload = parse_dj_link_payload(&self.payload)?;
            }
            DjLinkMessageType::TimelineBeatJump => {
                let payload: DjLinkTimelineBeatJumpPayload = parse_dj_link_payload(&self.payload)?;
                if payload.bars != 4 {
                    return Err("DJ timeline beat jump must be exactly +4 bars".to_string());
                }
                validate_dj_link_string(&payload.timeline_id, "timelineId")?;
                validate_dj_link_required_identity(&payload.play_session_id, "playSessionId")?;
            }
            DjLinkMessageType::TimelineLoopSet => {
                let payload: DjLinkTimelineLoopSetPayload = parse_dj_link_payload(&self.payload)?;
                validate_dj_link_string(&payload.timeline_id, "timelineId")?;
                validate_dj_link_required_identity(&payload.play_session_id, "playSessionId")?;
            }
            DjLinkMessageType::TimelineLoopHalf => {
                let payload: DjLinkTimelineLoopHalfPayload = parse_dj_link_payload(&self.payload)?;
                validate_dj_link_string(&payload.timeline_id, "timelineId")?;
                validate_dj_link_required_identity(&payload.play_session_id, "playSessionId")?;
            }
        }
        Ok(())
    }

    pub fn canonical_shape(&self) -> Result<String, String> {
        self.validate()?;
        let mut value = serde_json::to_value(self).map_err(|error| error.to_string())?;
        canonicalize_dj_link_value(&mut value);
        serde_json::to_string(&value).map_err(|error| error.to_string())
    }
}

fn parse_dj_link_payload<T>(payload: &serde_json::Value) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(payload.clone()).map_err(|error| error.to_string())
}

pub fn validate_dj_link_token(value: &str) -> Result<(), String> {
    if value.len() < DJ_LINK_MIN_TOKEN_BYTES || value.len() > DJ_LINK_MAX_STRING_BYTES {
        return Err("DJ Link auth token length is outside 32..256 UTF-8 bytes".to_string());
    }
    if value
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err("DJ Link auth token contains whitespace or control characters".to_string());
    }
    Ok(())
}

fn validate_dj_link_capability_set(capabilities: &[String]) -> Result<(), String> {
    let required_capabilities = if capabilities.len() == DJ_LINK_REQUIRED_CAPABILITIES.len()
        && capabilities
            .iter()
            .all(|capability| DJ_LINK_REQUIRED_CAPABILITIES.contains(&capability.as_str()))
    {
        &DJ_LINK_REQUIRED_CAPABILITIES[..]
    } else {
        return Err(
            "DJ Link HELLO capabilities must list the exact per-deck capability set".to_string(),
        );
    };
    let mut seen = BTreeSet::new();
    for capability in capabilities {
        validate_dj_link_string(capability, "capability")?;
        if !required_capabilities.contains(&capability.as_str()) {
            return Err(format!("unknown DJ Link HELLO capability {capability}"));
        }
        if !seen.insert(capability.as_str()) {
            return Err(format!("duplicate DJ Link HELLO capability {capability}"));
        }
    }
    Ok(())
}

pub fn validate_dj_link_deck(deck: u8) -> Result<(), String> {
    if !(1..=4).contains(&deck) {
        return Err("DJ Link deck must be an integer 1..=4".to_string());
    }
    Ok(())
}

pub fn dj_link_deck_id(deck: u8) -> String {
    format!("rekordbox-deck-{deck}")
}

fn validate_dj_link_bpm(value: f64, label: &str) -> Result<(), String> {
    if !value.is_finite() || !(0.0..DJ_LINK_MAX_BPM).contains(&value) || value <= 0.0 {
        return Err(format!(
            "DJ Link {label} must be positive and <= {DJ_LINK_MAX_BPM}"
        ));
    }
    Ok(())
}

/// Accepts RFC3339 timestamps (`YYYY-MM-DDTHH:MM:SS[.fraction](Z|±HH:MM)`),
/// validating the calendar date including leap years.
fn validate_dj_link_timestamp(value: &str) -> Result<(), String> {
    let invalid = || "DJ Link startedAt must be a nonempty RFC3339 timestamp".to_string();
    let bytes = value.as_bytes();
    if bytes.len() < 20 {
        return Err(invalid());
    }
    let digits = |slice: &[u8]| slice.iter().all(|byte| byte.is_ascii_digit());
    let number = |slice: &[u8]| -> Option<u32> {
        if slice.is_empty() || !digits(slice) {
            return None;
        }
        std::str::from_utf8(slice)
            .ok()
            .and_then(|text| text.parse::<u32>().ok())
    };
    let year = number(&bytes[0..4]).ok_or_else(invalid)?;
    if bytes[4] != b'-' {
        return Err(invalid());
    }
    let month = number(&bytes[5..7]).ok_or_else(invalid)?;
    if bytes[7] != b'-' {
        return Err(invalid());
    }
    let day = number(&bytes[8..10]).ok_or_else(invalid)?;
    if !(1..=12).contains(&month) {
        return Err(invalid());
    }
    let leap_year = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let max_day = if month == 2 && leap_year {
        29
    } else {
        days_in_month[(month - 1) as usize]
    };
    if !(1..=max_day).contains(&day) {
        return Err(invalid());
    }
    if bytes[10] != b'T' && bytes[10] != b't' {
        return Err(invalid());
    }
    let rest = &bytes[11..];
    if rest.len() < 8 {
        return Err(invalid());
    }
    let hour = number(&rest[0..2]).ok_or_else(invalid)?;
    if rest[2] != b':' {
        return Err(invalid());
    }
    let minute = number(&rest[3..5]).ok_or_else(invalid)?;
    if rest[5] != b':' {
        return Err(invalid());
    }
    let second = number(&rest[6..8]).ok_or_else(invalid)?;
    if hour > 23 || minute > 59 || second > 60 {
        return Err(invalid());
    }
    let mut tail = &rest[8..];
    if !tail.is_empty() && tail[0] == b'.' {
        let mut fraction = 1usize;
        while fraction < tail.len() && tail[fraction].is_ascii_digit() {
            fraction += 1;
        }
        if fraction == 1 {
            return Err(invalid());
        }
        tail = &tail[fraction..];
    }
    match tail {
        [b'Z'] | [b'z'] => {}
        [sign, h0, h1, b':', m0, m1] => {
            if *sign != b'+' && *sign != b'-' {
                return Err(invalid());
            }
            let offset_hour = number(&[*h0, *h1]).ok_or_else(invalid)?;
            let offset_minute = number(&[*m0, *m1]).ok_or_else(invalid)?;
            if offset_hour > 23 || offset_minute > 59 {
                return Err(invalid());
            }
        }
        _ => return Err(invalid()),
    }
    Ok(())
}

impl DjLinkMeasuredLoop {
    pub fn validate(&self) -> Result<(), String> {
        if self.revision == 0 || self.revision > DJ_LINK_MAX_SEQUENCE {
            return Err("measured loop revision must be a positive safe integer".to_string());
        }
        if self.sample_age_ms > DJ_LINK_MAX_SAMPLE_AGE_MS {
            return Err("measured loop sampleAgeMs exceeds the freshness bound".to_string());
        }
        if self.source != DJ_LINK_MEASURED_LOOP_SOURCE {
            return Err(format!(
                "measured loop source must be exactly {DJ_LINK_MEASURED_LOOP_SOURCE}"
            ));
        }
        for (value, label) in [
            (self.start_beat, "startBeat"),
            (self.end_beat, "endBeat"),
            (self.length_beats, "lengthBeats"),
        ] {
            if let Some(value) = value {
                if !value.is_finite() {
                    return Err(format!("measured loop {label} must be finite"));
                }
                if matches!(label, "startBeat" | "endBeat") && value < 0.0 {
                    return Err(format!("measured loop {label} must be non-negative"));
                }
            }
        }
        let span_consistent = |start: f64,
                               end: f64,
                               length: Option<f64>,
                               label: &'static str|
         -> Result<(), String> {
            if end <= start {
                return Err(format!(
                    "measured loop {label} requires endBeat > startBeat"
                ));
            }
            if let Some(length) = length {
                if length <= 0.0 {
                    return Err("measured loop lengthBeats must be > 0 when present".to_string());
                }
                if (length - (end - start)).abs() > DJ_LINK_LOOP_LENGTH_TOLERANCE_BEATS {
                    return Err(
                        "measured loop lengthBeats exceeds the beat-span tolerance".to_string()
                    );
                }
            }
            Ok(())
        };
        match (self.start_beat, self.end_beat, self.length_beats) {
            (Some(start), Some(end), length) => {
                span_consistent(start, end, length, "span")?;
            }
            (None, None, None) => {}
            (None, None, Some(length)) => {
                if length <= 0.0 {
                    return Err("measured loop lengthBeats must be > 0 when present".to_string());
                }
            }
            _ => {}
        }
        if self.active
            && matches!(
                (self.start_beat, self.end_beat, self.length_beats),
                (None, _, _) | (_, None, _) | (_, _, None)
            )
        {
            return Err(
                "an active measured loop requires non-null startBeat,endBeat,lengthBeats"
                    .to_string(),
            );
        }
        if !self.active
            && (self.start_beat.is_some() || self.end_beat.is_some() || self.length_beats.is_some())
        {
            return Err(
                "an inactive measured loop requires null startBeat,endBeat,lengthBeats".to_string(),
            );
        }
        Ok(())
    }
}

impl DjLinkTrackLoopStatePayload {
    pub fn validate(&self) -> Result<(), String> {
        validate_dj_link_deck(self.deck)?;
        if self.deck_id != dj_link_deck_id(self.deck) {
            return Err("deckId must be rekordbox-deck-N matching deck".to_string());
        }
        validate_dj_link_string(&self.play_session_id, "playSessionId")?;
        self.loop_state.validate()
    }
}

impl DjLinkTrackLoopFallbackPayload {
    pub fn validate(&self) -> Result<(), String> {
        validate_dj_link_deck(self.deck)?;
        if self.deck_id != dj_link_deck_id(self.deck) {
            return Err("deckId must be rekordbox-deck-N matching deck".to_string());
        }
        validate_dj_link_loop_fallback_fields(
            &self.play_session_id,
            self.pedal_intent_id,
            self.base_measured_loop_revision,
            self.base_loop_division,
            self.target_length_beats,
            self.response_window_ms,
            &self.source,
        )
    }
}

impl DjLinkTrackStateSyncPayload {
    pub fn validate(&self) -> Result<(), String> {
        match (
            self.owner_deck,
            self.owner_deck_id.as_deref(),
            self.active_play_session_id.as_deref(),
        ) {
            (None, None, None) => Ok(()),
            (Some(deck), Some(deck_id), Some(play_session_id)) => {
                validate_dj_link_deck(deck)?;
                if deck_id != dj_link_deck_id(deck) {
                    return Err("ownerDeckId must be rekordbox-deck-N matching ownerDeck".to_string());
                }
                validate_dj_link_string(play_session_id, "activePlaySessionId")
            }
            _ => Err(
                "generic DJ state sync must omit owner context or provide ownerDeck,ownerDeckId,activePlaySessionId together"
                    .to_string(),
            ),
        }
    }
}

fn validate_dj_link_loop_fallback_fields(
    play_session_id: &str,
    pedal_intent_id: u64,
    base_measured_loop_revision: Option<u64>,
    base_loop_division: Option<u8>,
    target_length_beats: f64,
    response_window_ms: u64,
    source: &str,
) -> Result<(), String> {
    validate_dj_link_required_identity(play_session_id, "playSessionId")?;
    if pedal_intent_id == 0 || pedal_intent_id > DJ_LINK_MAX_SEQUENCE {
        return Err("fallback pedalIntentId must be a positive safe integer".to_string());
    }
    if let Some(revision) = base_measured_loop_revision {
        if revision == 0 || revision > DJ_LINK_MAX_SEQUENCE {
            return Err(
                "fallback baseMeasuredLoopRevision must be null or a positive safe integer"
                    .to_string(),
            );
        }
    }
    if let Some(division) = base_loop_division {
        if usize::from(division) >= DJ_LINK_LOOP_PROFILE_LENGTH_BEATS.len() {
            return Err("fallback baseLoopDivision is outside the supported profile".to_string());
        }
    }
    if !target_length_beats.is_finite() {
        return Err("fallback targetLengthBeats must be finite".to_string());
    }
    if !DJ_LINK_LOOP_PROFILE_LENGTH_BEATS.contains(&target_length_beats) {
        return Err(
            "fallback targetLengthBeats must be an exact supported Rekordbox profile value"
                .to_string(),
        );
    }
    let expected_target_index = base_loop_division
        .map(|division| {
            (usize::from(division) + 1).min(DJ_LINK_LOOP_PROFILE_LENGTH_BEATS.len() - 1)
        })
        .unwrap_or(0);
    if target_length_beats != DJ_LINK_LOOP_PROFILE_LENGTH_BEATS[expected_target_index] {
        return Err(
            "fallback targetLengthBeats must be the exact next downward profile value from baseLoopDivision"
                .to_string(),
        );
    }
    if !(DJ_LINK_LOOP_FALLBACK_MIN_RESPONSE_WINDOW_MS
        ..=DJ_LINK_LOOP_FALLBACK_MAX_RESPONSE_WINDOW_MS)
        .contains(&response_window_ms)
    {
        return Err("fallback responseWindowMs is outside the bounded window".to_string());
    }
    if source != DJ_LINK_LOOP_FALLBACK_SOURCE {
        return Err(format!(
            "fallback source must be exactly {DJ_LINK_LOOP_FALLBACK_SOURCE}"
        ));
    }
    Ok(())
}

impl DjLinkTrackPayload {
    pub fn validate(&self) -> Result<(), String> {
        validate_dj_link_track_payload(self)
    }
}

fn validate_dj_link_track_payload(payload: &DjLinkTrackPayload) -> Result<(), String> {
    validate_dj_link_deck(payload.deck)?;
    if payload.deck_id != dj_link_deck_id(payload.deck) {
        return Err("deckId must be rekordbox-deck-N matching deck".to_string());
    }
    if payload.position_revision == 0 || payload.position_revision > DJ_LINK_MAX_SEQUENCE {
        return Err("positionRevision must be a positive safe integer".to_string());
    }
    match (
        payload.content_id.as_deref(),
        payload.title.as_deref(),
        payload.artist.as_deref(),
    ) {
        (Some(content_id), None, None) => validate_dj_link_string(content_id, "contentId")?,
        (None, Some(title), Some(artist)) => {
            validate_dj_link_string(title, "title")?;
            validate_dj_link_string(artist, "artist")?;
        }
        _ => {
            return Err(
                "track identity must be exactly one nonempty contentId or both title and artist"
                    .to_string(),
            )
        }
    }
    if let Some(track_bpm) = payload.track_bpm {
        validate_dj_link_bpm(track_bpm, "trackBpm")?;
    }
    validate_dj_link_bpm(payload.effective_bpm, "effectiveBpm")?;
    if !payload.position_at_send_sec.is_finite()
        || !(0.0..=DJ_LINK_MAX_POSITION_AT_SEND_SEC).contains(&payload.position_at_send_sec)
    {
        return Err(format!(
            "positionAtSendSec must be finite within 0..={DJ_LINK_MAX_POSITION_AT_SEND_SEC}"
        ));
    }
    if payload.sample_age_ms > DJ_LINK_MAX_SAMPLE_AGE_MS {
        return Err(format!(
            "sampleAgeMs must be within 0..={DJ_LINK_MAX_SAMPLE_AGE_MS}"
        ));
    }
    if !payload.is_playing {
        return Err("isPlaying must be true on track events".to_string());
    }
    validate_dj_link_timestamp(&payload.started_at)?;
    validate_dj_link_string(&payload.play_session_id, "playSessionId")?;
    if let Some(loop_state) = payload.loop_state.as_ref() {
        loop_state.validate()?;
    }
    Ok(())
}

/// Detects duplicate JSON object keys at every nesting level before serde can
/// silently collapse them. Escape sequences are decoded so `\u0041gent` and
/// `agent` compare as the same key.
fn reject_duplicate_json_keys(text: &str) -> Result<(), String> {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    #[derive(Debug)]
    enum Container {
        Object {
            keys: BTreeSet<String>,
            expecting_key: bool,
        },
        Array,
    }
    let mut stack: Vec<Container> = Vec::new();

    fn skip_whitespace(bytes: &[u8], index: &mut usize) {
        while *index < bytes.len() && matches!(bytes[*index], b' ' | b'\t' | b'\n' | b'\r') {
            *index += 1;
        }
    }

    fn decode_escape(bytes: &[u8], index: &mut usize) -> Result<String, String> {
        *index += 1;
        if *index >= bytes.len() {
            return Err("truncated escape sequence in JSON string".to_string());
        }
        let escape = bytes[*index];
        *index += 1;
        let simple = match escape {
            b'"' => Some('"'),
            b'\\' => Some('\\'),
            b'/' => Some('/'),
            b'b' => Some('\u{0008}'),
            b'f' => Some('\u{000C}'),
            b'n' => Some('\n'),
            b'r' => Some('\r'),
            b't' => Some('\t'),
            _ => None,
        };
        if let Some(character) = simple {
            return Ok(character.to_string());
        }
        if escape != b'u' || *index + 4 > bytes.len() {
            return Err("invalid escape sequence in JSON string".to_string());
        }
        let hex = std::str::from_utf8(&bytes[*index..*index + 4])
            .map_err(|_| "invalid \\u escape".to_string())?;
        let code = u16::from_str_radix(hex, 16).map_err(|_| "invalid \\u escape".to_string())?;
        *index += 4;
        if (0xD800..0xDC00).contains(&code) {
            if *index + 6 <= bytes.len() && bytes[*index] == b'\\' && bytes[*index + 1] == b'u' {
                let low_hex = std::str::from_utf8(&bytes[*index + 2..*index + 6])
                    .map_err(|_| "invalid surrogate pair".to_string())?;
                let low = u16::from_str_radix(low_hex, 16)
                    .map_err(|_| "invalid surrogate pair".to_string())?;
                *index += 6;
                if (0xDC00..0xE000).contains(&low) {
                    let combined =
                        0x1_0000 + ((u32::from(code) - 0xD800) << 10) + (u32::from(low) - 0xDC00);
                    return char::from_u32(combined)
                        .map(|character| character.to_string())
                        .ok_or_else(|| "invalid surrogate pair".to_string());
                }
            }
            return Ok('\u{FFFD}'.to_string());
        }
        if (0xDC00..0xE000).contains(&code) {
            return Ok('\u{FFFD}'.to_string());
        }
        char::from_u32(u32::from(code))
            .map(|character| character.to_string())
            .ok_or_else(|| "invalid \\u escape".to_string())
    }

    fn parse_json_string(bytes: &[u8], index: &mut usize) -> Result<String, String> {
        *index += 1;
        let mut decoded = String::new();
        while *index < bytes.len() {
            match bytes[*index] {
                b'"' => {
                    *index += 1;
                    return Ok(decoded);
                }
                b'\\' => decoded.push_str(&decode_escape(bytes, index)?),
                _ => {
                    let start = *index;
                    *index += 1;
                    while *index < bytes.len() && (bytes[*index] & 0xC0) == 0x80 {
                        *index += 1;
                    }
                    let chunk = std::str::from_utf8(&bytes[start..*index])
                        .map_err(|_| "invalid UTF-8 in JSON string".to_string())?;
                    decoded.push_str(chunk);
                }
            }
        }
        Err("unterminated JSON string".to_string())
    }

    skip_whitespace(bytes, &mut index);
    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b'{' => {
                stack.push(Container::Object {
                    keys: BTreeSet::new(),
                    expecting_key: true,
                });
                index += 1;
            }
            b'[' => {
                stack.push(Container::Array);
                index += 1;
            }
            b'}' => {
                if !matches!(stack.pop(), Some(Container::Object { .. })) {
                    return Err("unbalanced JSON object".to_string());
                }
                index += 1;
            }
            b']' => {
                if !matches!(stack.pop(), Some(Container::Array)) {
                    return Err("unbalanced JSON array".to_string());
                }
                index += 1;
            }
            b',' => {
                if let Some(Container::Object { expecting_key, .. }) = stack.last_mut() {
                    *expecting_key = true;
                }
                index += 1;
            }
            b':' => {
                match stack.last() {
                    Some(Container::Object {
                        expecting_key: false,
                        ..
                    }) => {}
                    _ => return Err("unexpected ':' outside a JSON object pair".to_string()),
                }
                index += 1;
            }
            b'"' => {
                let decoded = parse_json_string(bytes, &mut index)?;
                if let Some(Container::Object {
                    keys,
                    expecting_key,
                }) = stack.last_mut()
                {
                    if *expecting_key {
                        if !keys.insert(decoded.clone()) {
                            return Err(format!("duplicate JSON key {decoded}"));
                        }
                        *expecting_key = false;
                    }
                }
            }
            _ => {
                while index < bytes.len()
                    && !matches!(
                        bytes[index],
                        b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r'
                    )
                {
                    index += 1;
                }
            }
        }
        skip_whitespace(bytes, &mut index);
    }
    if !stack.is_empty() {
        return Err("unterminated JSON container".to_string());
    }
    Ok(())
}

fn canonicalize_dj_link_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            let mut ordered = BTreeMap::new();
            for (key, mut child) in std::mem::take(object) {
                canonicalize_dj_link_value(&mut child);
                ordered.insert(key, child);
            }
            *object = ordered.into_iter().collect();
        }
        serde_json::Value::Array(items) => {
            for item in items {
                canonicalize_dj_link_value(item);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClockSnapshot {
    pub bpm: f32,
    pub beat_phase: f32,
    pub beat_counter: u64,
    pub tap_count: usize,
    pub source: ClockSource,
    #[serde(default)]
    pub external_sync_age_ms: Option<u64>,
    #[serde(default)]
    pub external_sync_locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubmasterSummary {
    pub group_id: String,
    pub label: String,
    pub level: f32,
    /// Runtime-only Live Mixer strobe rate. Project persistence normalizers
    /// reset this to zero; omission preserves legacy `.sdc` byte shape.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub strobe_hz: f32,
    /// Number of patched fixtures in this group with a canonical GDTF
    /// Shutter/Strobe function and physical frequency metadata. A requested
    /// rate can still fail closed if equally near functions are ambiguous.
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub strobe_fixture_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DmxUniversePreview {
    pub universe: u16,
    pub values: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct StageMapConfig {
    pub locked: bool,
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StageObjectKind {
    Stage,
    Truss,
    Screen,
    Riser,
    Mask,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageObjectSummary {
    pub id: StageObjectId,
    pub label: String,
    pub kind: StageObjectKind,
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub depth: f32,
    pub rotation_deg: f32,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageMapPresetSummary {
    pub label: String,
    pub config: StageMapConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_objects: Option<Vec<StageObjectSummary>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageMapPresetFile {
    pub version: u32,
    pub app: String,
    pub preset: StageMapPresetSummary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TouchControlKind {
    Label,
    Image,
    Button,
    Fader,
    Dial,
    IncrementalWheel,
    ColorWheel,
    XyGrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TouchFeaturePresetTarget {
    pub fixture_id: FixtureId,
    pub attribute: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TouchControlBinding {
    FixtureAttribute {
        fixture_id: FixtureId,
        attribute: String,
    },
    GroupAttribute {
        group_id: String,
        attribute: String,
    },
    FeaturePreset {
        targets: Vec<TouchFeaturePresetTarget>,
        min_value: u16,
        max_value: u16,
        inverted: bool,
    },
    FixtureColor {
        fixture_id: FixtureId,
    },
    GroupColor {
        group_id: String,
    },
    FixturePanTilt {
        fixture_id: FixtureId,
        pan_attribute: String,
        tilt_attribute: String,
    },
    GroupPanTilt {
        group_id: String,
        pan_attribute: String,
        tilt_attribute: String,
    },
    Cue {
        cue_id: CueId,
    },
    GroupSelect {
        group_id: String,
    },
    GroupSubmaster {
        group_id: String,
    },
    TapTempo,
    LightingMaster,
    VideoMaster,
    Blackout,
    VideoBlackout,
    AllBlackout,
    CueNext,
    CuePrevious,
    CueFadePause,
    SelectedFixtureAttribute {
        attribute: String,
    },
    SelectedFixtureColor,
    SelectedFixturePanTilt {
        pan_attribute: String,
        tilt_attribute: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TouchControlSummary {
    pub id: TouchControlId,
    pub kind: TouchControlKind,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<TouchControlBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TouchPageSummary {
    pub id: TouchPageId,
    pub label: String,
    #[serde(default)]
    pub controls: Vec<TouchControlSummary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TouchSurfaceSummary {
    #[serde(default)]
    pub pages: Vec<TouchPageSummary>,
}

/// Runtime-only transport state for a Super Scene triggered directly from the
/// Scene Matrix. This is published for operator feedback and is stripped from
/// every persistence snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectChildTimelineTransportSummary {
    pub cue_id: CueId,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub playing: bool,
    pub generation: u64,
    #[serde(default)]
    pub count_in_remaining_ms: u64,
}

impl TouchSurfaceSummary {
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }
}

impl Default for StageMapConfig {
    fn default() -> Self {
        Self {
            locked: false,
            min_x: -10.0,
            max_x: 10.0,
            min_z: -10.0,
            max_z: 10.0,
        }
    }
}

impl Default for ClockSnapshot {
    fn default() -> Self {
        Self {
            bpm: 120.0,
            beat_phase: 0.0,
            beat_counter: 0,
            tap_count: 0,
            source: ClockSource::Manual,
            external_sync_age_ms: None,
            external_sync_locked: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineSnapshot {
    pub fixtures: Vec<PatchedFixtureSummary>,
    pub cues: Vec<CueSummary>,
    #[serde(default = "default_cue_lists")]
    pub cue_lists: Vec<CueListSummary>,
    #[serde(default)]
    pub palettes: Vec<ReferencePaletteSummary>,
    #[serde(default = "default_playback_executors")]
    pub playback_executors: Vec<PlaybackExecutorSummary>,
    #[serde(default = "default_level")]
    pub playback_master: f32,
    pub active_cue_id: Option<CueId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub direct_child_timeline_transports: Vec<DirectChildTimelineTransportSummary>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub active_group_cue_ids: BTreeMap<String, CueId>,
    /// T17 runtime-only latched scene live-modifier overrides. UI display
    /// only: the persistence snapshot clears this before any `.sdc` write and
    /// project load never reads it, so live overrides are never saved.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cue_live_modifiers: Vec<CueLiveModifierState>,
    /// T7 persistent identity colors per group path (`#rrggbb`). BTreeMap keeps
    /// serialization order deterministic for project snapshot comparison;
    /// skipped when empty so legacy snapshots stay byte-identical.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub group_colors: BTreeMap<String, String>,
    pub active_fade: Option<ActiveFadeSummary>,
    #[serde(default)]
    pub programmer: ProgrammerSnapshot,
    pub timeline: TimelineSnapshot,
    /// Ordered authored Timeline bank. Empty is the legacy single-Timeline
    /// representation; loaders normalize it to contain `timeline` once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timeline_bank: Vec<TimelineSnapshot>,
    pub video: VideoSnapshot,
    /// Runtime-only Clip Slot transport truth published atomically with the
    /// rendered engine image. This field is deliberately excluded in both
    /// serialization directions: project files cannot persist or inject
    /// active, queued, pending, or playhead state.
    #[serde(skip, default)]
    pub video_clip_runtime: VideoClipRuntimeSnapshot,
    /// Runtime-only Layer Transition Bus truth. Project files cannot persist
    /// or inject active from/to/progress state.
    #[serde(skip, default)]
    pub video_transition_runtime: VideoLayerTransitionRuntimeSnapshot,
    /// Raw authored layer state before ephemeral Effect/Node Graph modulation.
    /// Project and cue writers consume this value, then strip it from persisted data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored_video: Option<VideoSnapshot>,
    pub effects: Vec<EffectSummary>,
    #[serde(default)]
    pub node_graphs: Vec<NodeGraphSummary>,
    pub output: DmxOutputConfig,
    pub dmx_outputs: Vec<DmxOutputConfig>,
    pub lighting_master: f32,
    pub submasters: Vec<SubmasterSummary>,
    pub blackout: bool,
    pub clock: ClockSnapshot,
    #[serde(default)]
    pub stage_map: StageMapConfig,
    #[serde(default)]
    pub stage_map_presets: Vec<StageMapPresetSummary>,
    #[serde(default)]
    pub stage_objects: Vec<StageObjectSummary>,
    /// T11 show-owned composed Touch layout. Empty layouts are skipped so a
    /// legacy v1 snapshot serializes to exactly the same bytes as before T11.
    #[serde(default, skip_serializing_if = "TouchSurfaceSummary::is_empty")]
    pub touch_surface: TouchSurfaceSummary,
    pub dmx_preview: Vec<u8>,
    #[serde(default)]
    pub dmx_previews: Vec<DmxUniversePreview>,
    pub telemetry: EngineTelemetry,
}

/// Which persisted video image owns authored project references after a
/// successful current-schema integrity check.
///
/// This is deliberately returned to the caller instead of silently applying
/// `authored_video.unwrap_or(video)`: a caller must make the authority choice
/// visible at its project-load boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectVideoAuthority {
    Video,
    AuthoredVideo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectAuthoredVideoConflictKind {
    RuntimeProjectionDeclaration,
    IdentitySetMismatch,
    ReferenceTopologyMismatch,
}

/// A reference registry that snapshot-level validation cannot reach because
/// its authoritative table lives outside `EngineSnapshot`.
///
/// The report names the boundary explicitly so a caller can never mistake a
/// successful snapshot check for complete `ProjectFile` validation. Use
/// [`validate_project_file_reference_integrity`] at a `ProjectFile` boundary
/// to also validate the named registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectExternalRegistryBoundary {
    /// Reference domain carried inside `EngineSnapshot` without its registry.
    pub domain: &'static str,
    /// Where the authoritative registry table actually lives.
    pub registry_location: &'static str,
}

impl ProjectExternalRegistryBoundary {
    /// Lighting fixture groups are referenced by string id from
    /// `target_group_ids`/`group_ids`, but the first-class group table is
    /// `ProjectFile.fixture_groups`, which is not part of `EngineSnapshot`.
    pub const LIGHTING_FIXTURE_GROUPS: Self = Self {
        domain: "lighting fixture group",
        registry_location: "ProjectFile.fixture_groups",
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectReferenceIntegrityReport {
    pub video_authority: ProjectVideoAuthority,
    /// Registries this validation deliberately did not check because their
    /// tables live outside the validated input. `None` means every referenced
    /// registry was reachable and validated.
    pub unvalidated_external_registry: Option<ProjectExternalRegistryBoundary>,
}

/// Typed, deterministic project-reference rejection.  No variant performs a
/// migration, synthesizes a host, drops a dangling entity, or chooses between
/// competing authorities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectReferenceIntegrityError {
    ZeroId {
        domain: &'static str,
        owner: String,
    },
    DuplicateId {
        domain: &'static str,
        id: u64,
    },
    MissingReference {
        owner: String,
        target_domain: &'static str,
        target_id: u64,
    },
    ReferenceMismatch {
        owner: String,
        detail: String,
    },
    ReferenceCycle {
        domain: &'static str,
        path: Vec<u64>,
    },
    /// The decoded snapshot declares the current multi-Timeline schema (or is
    /// being gated as such) but carries no Timeline bank at all. This is its
    /// own deterministic rejection, distinct from a genuine projection
    /// conflict between an active Timeline and its bank entry.
    RequiredTimelineBankEmpty {
        active_timeline_id: TimelineId,
    },
    /// String-keyed counterpart of [`ProjectReferenceIntegrityError::MissingReference`]
    /// for lighting fixture groups, whose identity is an authored label
    /// rather than a numeric id.
    MissingGroupReference {
        owner: String,
        group_id: String,
    },
    /// String-keyed counterpart of [`ProjectReferenceIntegrityError::DuplicateId`].
    DuplicateGroupIdentity {
        registry_owner: String,
        group_id: String,
    },
    ActiveTimelineProjectionConflict {
        timeline_id: TimelineId,
        detail: String,
    },
    AuthoredVideoConflict {
        kind: ProjectAuthoredVideoConflictKind,
        detail: String,
    },
}

impl std::fmt::Display for ProjectReferenceIntegrityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroId { domain, owner } => {
                write!(formatter, "{owner} has zero {domain} identity")
            }
            Self::DuplicateId { domain, id } => {
                write!(formatter, "{domain} identity {id} is duplicated")
            }
            Self::MissingReference {
                owner,
                target_domain,
                target_id,
            } => write!(
                formatter,
                "{owner} references missing {target_domain} {target_id}"
            ),
            Self::ReferenceMismatch { owner, detail } => {
                write!(formatter, "{owner} has inconsistent reference: {detail}")
            }
            Self::ReferenceCycle { domain, path } => {
                write!(formatter, "{domain} reference cycle: {path:?}")
            }
            Self::RequiredTimelineBankEmpty { active_timeline_id } => write!(
                formatter,
                "current schema requires a non-empty Timeline bank; \
                 active Timeline {} has no bank entries",
                active_timeline_id.0
            ),
            Self::MissingGroupReference { owner, group_id } => write!(
                formatter,
                "{owner} references lighting fixture group {group_id:?} with no member fixtures"
            ),
            Self::DuplicateGroupIdentity {
                registry_owner,
                group_id,
            } => write!(
                formatter,
                "{registry_owner} declares lighting fixture group {group_id:?} more than once"
            ),
            Self::ActiveTimelineProjectionConflict {
                timeline_id,
                detail,
            } => write!(
                formatter,
                "active Timeline {} projection conflicts with its bank entry: {detail}",
                timeline_id.0
            ),
            Self::AuthoredVideoConflict { kind, detail } => {
                write!(
                    formatter,
                    "authored_video/video conflict ({kind:?}): {detail}"
                )
            }
        }
    }
}

impl std::error::Error for ProjectReferenceIntegrityError {}

fn project_zero_id(
    domain: &'static str,
    owner: impl Into<String>,
) -> ProjectReferenceIntegrityError {
    ProjectReferenceIntegrityError::ZeroId {
        domain,
        owner: owner.into(),
    }
}

fn project_duplicate_id(domain: &'static str, id: u64) -> ProjectReferenceIntegrityError {
    ProjectReferenceIntegrityError::DuplicateId { domain, id }
}

fn project_missing_reference(
    owner: impl Into<String>,
    target_domain: &'static str,
    target_id: u64,
) -> ProjectReferenceIntegrityError {
    ProjectReferenceIntegrityError::MissingReference {
        owner: owner.into(),
        target_domain,
        target_id,
    }
}

fn project_reference_mismatch(
    owner: impl Into<String>,
    detail: impl Into<String>,
) -> ProjectReferenceIntegrityError {
    ProjectReferenceIntegrityError::ReferenceMismatch {
        owner: owner.into(),
        detail: detail.into(),
    }
}

/// Validate a raw, already-decoded current-schema `EngineSnapshot` before any
/// legacy normalization, sanitization, allocator reservation, or runtime
/// installation. This API intentionally does not call the older permissive
/// project-load validators: their compatibility transforms are outside this
/// clean-break boundary.
///
/// Runtime-only Timeline transport fields are excluded from the active-bank
/// authored projection comparison. Likewise, rendered-only video layers are
/// accepted only when their exact IDs are declared by the runtime-only
/// `timeline_video_projection_layer_ids` fence.
///
/// Every snapshot field that carries entity references is validated,
/// including the runtime-published programmer values, active fade, direct
/// child Timeline transports, and latched scene live-modifier overrides; a
/// hostile file cannot hide dangling references behind runtime-only lists.
/// The lighting fixture-group registry is the one reference domain outside
/// this input, and the returned report names that boundary explicitly.
pub fn validate_current_engine_snapshot_reference_integrity(
    snapshot: &EngineSnapshot,
) -> Result<ProjectReferenceIntegrityReport, ProjectReferenceIntegrityError> {
    let cue_lists = project_index_nonzero_u64(
        snapshot
            .cue_lists
            .iter()
            .map(|cue_list| (cue_list.id, cue_list)),
        "Cue List",
    )?;
    let cues = project_index_nonzero_u64(snapshot.cues.iter().map(|cue| (cue.id, cue)), "Cue")?;
    let fixtures = project_index_nonzero_u64(
        snapshot
            .fixtures
            .iter()
            .map(|fixture| (fixture.id, fixture)),
        "fixture",
    )?;
    for fixture in &snapshot.fixtures {
        validate_fixture_stage_layout(fixture)?;
    }
    let palettes = project_index_nonzero_u64(
        snapshot
            .palettes
            .iter()
            .map(|palette| (palette.id, palette)),
        "palette",
    )?;
    let effects = project_index_nonzero_u64(
        snapshot.effects.iter().map(|effect| (effect.id, effect)),
        "lighting effect",
    )?;
    let node_graphs = project_index_nonzero_u64(
        snapshot.node_graphs.iter().map(|graph| (graph.id, graph)),
        "node graph",
    )?;

    if snapshot.timeline_bank.is_empty() {
        return Err(ProjectReferenceIntegrityError::RequiredTimelineBankEmpty {
            active_timeline_id: snapshot.timeline.id,
        });
    }
    let timelines = project_index_nonzero_u64(
        snapshot
            .timeline_bank
            .iter()
            .map(|timeline| (timeline.id.0, timeline)),
        "Timeline",
    )?;
    let active_bank_timeline = timelines.get(&snapshot.timeline.id.0).ok_or_else(|| {
        ProjectReferenceIntegrityError::ActiveTimelineProjectionConflict {
            timeline_id: snapshot.timeline.id,
            detail: "active Timeline identity is absent from the Timeline bank".to_string(),
        }
    })?;
    if !timeline_authored_projection_matches(&snapshot.timeline, active_bank_timeline) {
        return Err(
            ProjectReferenceIntegrityError::ActiveTimelineProjectionConflict {
                timeline_id: snapshot.timeline.id,
                detail:
                    "authored fields differ; runtime transport fields are intentionally ignored"
                        .to_string(),
            },
        );
    }

    let (video, video_authority) = match snapshot.authored_video.as_ref() {
        Some(authored) => {
            validate_authored_video_reference_projection(snapshot, authored)?;
            (authored, ProjectVideoAuthority::AuthoredVideo)
        }
        None => (&snapshot.video, ProjectVideoAuthority::Video),
    };
    let video_catalog = validate_video_project_references(video, &timelines)?;

    validate_active_cue_references(snapshot, &cue_lists, &cues)?;
    let fixture_attributes = fixture_attribute_catalog(&fixtures);
    validate_runtime_reference_fields(snapshot, &cues, &fixtures, &fixture_attributes)?;
    validate_cue_project_references(
        snapshot,
        CueProjectReferenceContext {
            cue_lists: &cue_lists,
            fixtures: &fixtures,
            fixture_attributes: &fixture_attributes,
            palettes: &palettes,
            effects: &effects,
            node_graphs: &node_graphs,
            video: &video_catalog,
        },
    )?;
    validate_effect_and_node_graph_references(
        snapshot,
        &fixtures,
        &fixture_attributes,
        &video_catalog,
    )?;

    for (index, timeline) in snapshot.timeline_bank.iter().enumerate() {
        validate_timeline_project_references(
            &format!("Timeline {}", timeline.id.0),
            TimelineProjectReferenceInput {
                layers: &timeline.layers,
                events: &timeline.events,
                automations: &timeline.automations,
                video_automations: &timeline.video_automations,
                audio_clips: &timeline.audio_clips,
                video_clips: &timeline.video_clips,
                item_groups: &timeline.item_groups,
                cues: &cues,
                fixtures: &fixtures,
                fixture_attributes: &fixture_attributes,
                video: &video_catalog,
            },
        )?;
        if let Some(follow) = &timeline.follow {
            if !timelines.contains_key(&follow.next_timeline_id.0) {
                return Err(project_missing_reference(
                    format!("Timeline {} Follow", timeline.id.0),
                    "Timeline",
                    follow.next_timeline_id.0,
                ));
            }
            if follow.enabled {
                let expected = snapshot.timeline_bank.get(index + 1).ok_or_else(|| {
                    project_reference_mismatch(
                        format!("Timeline {} Follow", timeline.id.0),
                        "enabled Follow has no next bank entry",
                    )
                })?;
                if follow.next_timeline_id != expected.id {
                    return Err(project_reference_mismatch(
                        format!("Timeline {} Follow", timeline.id.0),
                        format!(
                            "target {} is not the next bank Timeline {}",
                            follow.next_timeline_id.0, expected.id.0
                        ),
                    ));
                }
            }
        }
    }
    validate_timeline_follow_cycles(&snapshot.timeline_bank)?;

    for cue in &snapshot.cues {
        let Some(child) = &cue.child_timeline else {
            continue;
        };
        validate_timeline_project_references(
            &format!("Cue {} child Timeline", cue.id),
            TimelineProjectReferenceInput {
                layers: &child.layers,
                events: &child.events,
                automations: &child.automations,
                video_automations: &child.video_automations,
                audio_clips: &child.audio_clips,
                video_clips: &[],
                item_groups: &[],
                cues: &cues,
                fixtures: &fixtures,
                fixture_attributes: &fixture_attributes,
                video: &video_catalog,
            },
        )?;
    }
    validate_child_timeline_reference_cycles(&snapshot.cues)?;

    // Lighting fixture groups are referenced by string identity throughout
    // this snapshot, but the first-class group registry is
    // `ProjectFile.fixture_groups`, which lives outside `EngineSnapshot`.
    // Report that boundary explicitly instead of implying complete
    // ProjectFile validation; `validate_project_file_reference_integrity`
    // closes it.
    Ok(ProjectReferenceIntegrityReport {
        video_authority,
        unvalidated_external_registry: Some(
            ProjectExternalRegistryBoundary::LIGHTING_FIXTURE_GROUPS,
        ),
    })
}

/// Validate every persisted or runtime-published `EngineSnapshot` field that
/// carries entity references but sits outside the authored cue/timeline
/// tables: programmer values, the active fade's Cue, direct child Timeline
/// transports, and latched scene live-modifier overrides.
///
/// The live-modifier and transport lists are stripped by persistence, so a
/// decoded project normally carries them empty; a hostile or hand-edited file
/// can still inject them, and they are rejected here exactly like any other
/// dangling reference instead of being silently ignored.
fn validate_runtime_reference_fields(
    snapshot: &EngineSnapshot,
    cues: &BTreeMap<u64, &CueSummary>,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    for value in &snapshot.programmer.values {
        validate_fixture_attribute_reference(
            "programmer value",
            value.fixture_id,
            &value.attribute,
            fixtures,
            fixture_attributes,
        )?;
    }
    if let Some(fade) = &snapshot.active_fade {
        if !cues.contains_key(&fade.cue_id) {
            return Err(project_missing_reference("active fade", "Cue", fade.cue_id));
        }
    }
    for transport in &snapshot.direct_child_timeline_transports {
        if !cues.contains_key(&transport.cue_id) {
            return Err(project_missing_reference(
                "direct child Timeline transport",
                "Cue",
                transport.cue_id,
            ));
        }
    }
    for modifier in &snapshot.cue_live_modifiers {
        if !cues.contains_key(&modifier.cue_id) {
            return Err(project_missing_reference(
                "scene live modifier",
                "Cue",
                modifier.cue_id,
            ));
        }
    }
    Ok(())
}

/// Complete clean-break reference-integrity gate for a decoded
/// [`ProjectFile`].
///
/// This runs the full snapshot-level validation of
/// [`validate_current_engine_snapshot_reference_integrity`] *and* validates
/// the lighting fixture-group registry that lives outside
/// `EngineSnapshot`: registry identity, fixture membership well-formedness,
/// and resolvability of every lighting-effect `target_group_ids` reference
/// (including embedded request payloads) against actual fixture membership.
///
/// Use this at `ProjectFile` boundaries; the snapshot-level API alone cannot
/// see the group registry, and its report says so explicitly instead of
/// implying complete project validation.
pub fn validate_project_file_reference_integrity(
    project: &ProjectFile,
) -> Result<ProjectReferenceIntegrityReport, ProjectReferenceIntegrityError> {
    let mut report = validate_current_engine_snapshot_reference_integrity(&project.snapshot)?;
    validate_project_file_fixture_group_registry(project)?;
    // Every registry referenced by the snapshot was reachable here and has
    // now been validated, so the boundary statement is cleared.
    report.unvalidated_external_registry = None;
    Ok(report)
}

/// Validate the lighting fixture-group surfaces whose authoritative truth is
/// reachable only from a [`ProjectFile`]:
///
/// 1. `fixture_groups` registry entries are unique and well-formed.
/// 2. Every fixture's own `group_ids` membership strings are well-formed;
///    they remain the authoritative membership truth.
/// 3. Every lighting-effect `target_group_ids` reference (summary plus each
///    embedded request payload, including Chaser steps) is well-formed and
///    resolves to at least one fixture membership under the engine's
///    hierarchical prefix rule, so an effect can never persistently target a
///    group no fixture belongs to.
///
/// Malformed input is rejected, never normalized or repaired.
pub fn validate_project_file_fixture_group_registry(
    project: &ProjectFile,
) -> Result<(), ProjectReferenceIntegrityError> {
    let mut registered = BTreeSet::new();
    for group in &project.fixture_groups {
        validate_lighting_group_id_shape("ProjectFile fixture group registry", &group.id)?;
        if !registered.insert(normalized_lighting_group_id(&group.id)) {
            return Err(ProjectReferenceIntegrityError::DuplicateGroupIdentity {
                registry_owner: ProjectExternalRegistryBoundary::LIGHTING_FIXTURE_GROUPS
                    .registry_location
                    .to_string(),
                group_id: group.id.clone(),
            });
        }
    }

    let mut memberships = BTreeSet::new();
    for fixture in &project.snapshot.fixtures {
        for group_id in &fixture.group_ids {
            let owner = format!("fixture {} group membership", fixture.id);
            validate_lighting_group_id_shape(&owner, group_id)?;
            memberships.insert(normalized_lighting_group_id(group_id));
        }
    }

    let mut references = Vec::new();
    for effect in &project.snapshot.effects {
        references.push((
            format!("lighting effect {} targets", effect.id),
            effect.target_group_ids.as_slice(),
        ));
        if let Some(request) = &effect.lfo {
            references.push((
                format!("lighting effect {} LFO request", effect.id),
                request.target_group_ids.as_slice(),
            ));
        }
        if let Some(request) = &effect.chaser {
            for (step_index, step) in request.steps.iter().enumerate() {
                references.push((
                    format!("lighting effect {} Chaser step {step_index}", effect.id),
                    step.target_group_ids.as_slice(),
                ));
            }
        }
        if let Some(request) = &effect.move_effect {
            references.push((
                format!("lighting effect {} Move request", effect.id),
                request.target_group_ids.as_slice(),
            ));
        }
        if let Some(request) = &effect.value {
            references.push((
                format!("lighting effect {} Value request", effect.id),
                request.target_group_ids.as_slice(),
            ));
        }
        if let Some(request) = &effect.curve {
            references.push((
                format!("lighting effect {} Curve request", effect.id),
                request.target_group_ids.as_slice(),
            ));
        }
        if let Some(request) = &effect.mapping {
            references.push((
                format!("lighting effect {} Mapping request", effect.id),
                request.target_group_ids.as_slice(),
            ));
        }
        if let Some(request) = &effect.color_mapping {
            references.push((
                format!("lighting effect {} ColorMapping request", effect.id),
                request.target_group_ids.as_slice(),
            ));
        }
    }
    for (owner, group_ids) in references {
        for group_id in group_ids {
            validate_lighting_group_id_shape(&owner, group_id)?;
            if !membership_resolves(&memberships, group_id) {
                return Err(project_missing_reference_for_group(&owner, group_id));
            }
        }
    }
    Ok(())
}

/// Well-formedness rule mirroring the engine's own group-id acceptance: the
/// trimmed identifier is non-empty and contains no empty `/` path segment.
fn validate_lighting_group_id_shape(
    owner: &str,
    group_id: &str,
) -> Result<(), ProjectReferenceIntegrityError> {
    let trimmed = group_id.trim();
    let malformed =
        trimmed.is_empty() || trimmed.split('/').any(|segment| segment.trim().is_empty());
    if malformed {
        return Err(project_reference_mismatch(
            owner,
            format!("group id {group_id:?} is empty or has an empty path segment"),
        ));
    }
    Ok(())
}

fn normalized_lighting_group_id(group_id: &str) -> String {
    group_id
        .split('/')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("/")
}

/// Engine resolution rule: a requested group matches a fixture membership
/// when they are equal or the membership is a deeper path below the request.
fn membership_resolves(memberships: &BTreeSet<String>, requested: &str) -> bool {
    let requested = normalized_lighting_group_id(requested);
    memberships.iter().any(|member| {
        member == &requested
            || member
                .strip_prefix(requested.as_str())
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

fn project_missing_reference_for_group(
    owner: &str,
    group_id: &str,
) -> ProjectReferenceIntegrityError {
    ProjectReferenceIntegrityError::MissingGroupReference {
        owner: owner.to_string(),
        group_id: group_id.to_string(),
    }
}

fn project_index_nonzero_u64<'a, T>(
    values: impl Iterator<Item = (u64, &'a T)>,
    domain: &'static str,
) -> Result<BTreeMap<u64, &'a T>, ProjectReferenceIntegrityError> {
    let mut indexed = BTreeMap::new();
    for (id, value) in values {
        if id == 0 {
            return Err(project_zero_id(domain, domain));
        }
        if indexed.insert(id, value).is_some() {
            return Err(project_duplicate_id(domain, id));
        }
    }
    Ok(indexed)
}

fn timeline_authored_projection_matches(
    active: &TimelineSnapshot,
    bank: &TimelineSnapshot,
) -> bool {
    active.id == bank.id
        && active.label == bank.label
        && active.layers == bank.layers
        && active.events == bank.events
        && active.automations == bank.automations
        && active.video_automations == bank.video_automations
        && active.audio == bank.audio
        && active.audio_clips == bank.audio_clips
        && active.video_clips == bank.video_clips
        && active.phases == bank.phases
        && active.item_groups == bank.item_groups
        && active.loop_region == bank.loop_region
        && active.follow == bank.follow
        && active.guide_enabled == bank.guide_enabled
        && active.audio_offset_ms == bank.audio_offset_ms
        && active.audio_muted == bank.audio_muted
        && active.metronome_enabled == bank.metronome_enabled
        && active.count_in_beats == bank.count_in_beats
        && active.tempo_meter_map == bank.tempo_meter_map
        && active.tempo_meter_map_version == bank.tempo_meter_map_version
        && active.duration_ms == bank.duration_ms
    // Excluded deliberately: count-in/audio transport revisions, transport
    // epoch/generation, child transports, loop/follow runtime, guide/click
    // queues, playing, and position_ms.
}

struct ProjectVideoCatalog<'a> {
    layers: BTreeMap<VideoLayerId, &'a VideoLayerSummary>,
    media_assets: BTreeMap<MediaAssetId, &'a MediaAssetSummary>,
    outputs: BTreeMap<VideoOutputId, &'a VideoOutputSummary>,
}

fn validate_video_project_references<'a>(
    video: &'a VideoSnapshot,
    timelines: &BTreeMap<u64, &TimelineSnapshot>,
) -> Result<ProjectVideoCatalog<'a>, ProjectReferenceIntegrityError> {
    let layers = project_index_nonzero_u64(
        video.layers.iter().map(|layer| (layer.id, layer)),
        "video layer",
    )?;
    let media_assets = project_index_nonzero_u64(
        video.media_assets.iter().map(|asset| (asset.id, asset)),
        "MediaAsset",
    )?;
    let compositions = project_index_nonzero_u64(
        video
            .compositions
            .iter()
            .map(|composition| (composition.id, composition)),
        "video composition",
    )?;
    let outputs = project_index_nonzero_u64(
        video.outputs.iter().map(|output| (output.id, output)),
        "video output",
    )?;
    let groups = project_index_nonzero_u64(
        video.layer_groups.iter().map(|group| (group.id.0, group)),
        "video layer group",
    )?;
    let buses = project_index_nonzero_u64(
        video.transition_buses.iter().map(|bus| (bus.id.0, bus)),
        "video transition bus",
    )?;

    let mut slot_ids = BTreeSet::new();
    for layer in &video.layers {
        let owner = format!("video layer {}", layer.id);
        if let Some(asset_id) = layer.media_asset_id {
            if !media_assets.contains_key(&asset_id) {
                return Err(project_missing_reference(&owner, "MediaAsset", asset_id));
            }
        }
        let mut local_slots = BTreeSet::new();
        for slot in &layer.clip_slots {
            if slot.id.0 == 0 {
                return Err(project_zero_id("video clip slot", &owner));
            }
            if !slot_ids.insert(slot.id) {
                return Err(project_duplicate_id("video clip slot", slot.id.0));
            }
            local_slots.insert(slot.id);
            if !media_assets.contains_key(&slot.media_asset_id) {
                return Err(project_missing_reference(
                    format!("{owner} clip slot {}", slot.id.0),
                    "MediaAsset",
                    slot.media_asset_id,
                ));
            }
        }
        if let Some(default_slot_id) = layer.default_clip_slot_id {
            if !local_slots.contains(&default_slot_id) {
                return Err(project_missing_reference(
                    format!("{owner} default clip slot"),
                    "video clip slot",
                    default_slot_id.0,
                ));
            }
        }
    }

    let mut layer_memberships = BTreeMap::<VideoLayerId, usize>::new();
    for composition in &video.compositions {
        let mut own_layers = BTreeSet::new();
        for layer_id in &composition.layer_ids {
            if !own_layers.insert(*layer_id) {
                return Err(project_reference_mismatch(
                    format!("video composition {}", composition.id),
                    format!("contains video layer {layer_id} more than once"),
                ));
            }
            if !layers.contains_key(layer_id) {
                return Err(project_missing_reference(
                    format!("video composition {}", composition.id),
                    "video layer",
                    *layer_id,
                ));
            }
            *layer_memberships.entry(*layer_id).or_default() += 1;
        }
        let mut own_outputs = BTreeSet::new();
        for output_id in &composition.output_ids {
            if !own_outputs.insert(*output_id) {
                return Err(project_reference_mismatch(
                    format!("video composition {}", composition.id),
                    format!("contains video output {output_id} more than once"),
                ));
            }
            let output = outputs.get(output_id).ok_or_else(|| {
                project_missing_reference(
                    format!("video composition {}", composition.id),
                    "video output",
                    *output_id,
                )
            })?;
            if output.composition_id != composition.id {
                return Err(project_reference_mismatch(
                    format!("video composition {}", composition.id),
                    format!(
                        "output {output_id} points back to composition {}",
                        output.composition_id
                    ),
                ));
            }
        }
    }
    for layer_id in layers.keys() {
        if !layer_memberships.contains_key(layer_id) {
            return Err(project_reference_mismatch(
                format!("video layer {layer_id}"),
                "is not hosted by any composition",
            ));
        }
    }
    for output in &video.outputs {
        let composition = compositions.get(&output.composition_id).ok_or_else(|| {
            project_missing_reference(
                format!("video output {}", output.id),
                "video composition",
                output.composition_id,
            )
        })?;
        if !composition.output_ids.contains(&output.id) {
            return Err(project_reference_mismatch(
                format!("video output {}", output.id),
                format!(
                    "owning composition {} omits the reverse output reference",
                    output.composition_id
                ),
            ));
        }
    }

    let mut grouped_layers = BTreeSet::new();
    for group in &video.layer_groups {
        let composition = compositions.get(&group.composition_id).ok_or_else(|| {
            project_missing_reference(
                format!("video layer group {}", group.id.0),
                "video composition",
                group.composition_id,
            )
        })?;
        let mut own_layers = BTreeSet::new();
        for layer_id in &group.layer_ids {
            if !own_layers.insert(*layer_id) || !grouped_layers.insert(*layer_id) {
                return Err(project_reference_mismatch(
                    format!("video layer group {}", group.id.0),
                    format!("video layer {layer_id} has ambiguous group ownership"),
                ));
            }
            if !layers.contains_key(layer_id) {
                return Err(project_missing_reference(
                    format!("video layer group {}", group.id.0),
                    "video layer",
                    *layer_id,
                ));
            }
            if !composition.layer_ids.contains(layer_id) {
                return Err(project_reference_mismatch(
                    format!("video layer group {}", group.id.0),
                    format!(
                        "member layer {layer_id} is outside composition {}",
                        group.composition_id
                    ),
                ));
            }
        }
    }

    for bus in &video.transition_buses {
        let composition = compositions.get(&bus.composition_id).ok_or_else(|| {
            project_missing_reference(
                format!("video transition bus {}", bus.id.0),
                "video composition",
                bus.composition_id,
            )
        })?;
        let mut members = BTreeSet::new();
        for target in &bus.members {
            if !members.insert(target.clone()) {
                return Err(project_reference_mismatch(
                    format!("video transition bus {}", bus.id.0),
                    "contains duplicate transition members",
                ));
            }
            validate_video_transition_target_reference(
                target,
                bus.id,
                composition,
                &layers,
                &groups,
            )?;
        }
        for (role, target) in [
            ("default_from", Some(&bus.default_from)),
            ("default_to", Some(&bus.default_to)),
            ("matte_source", bus.matte_source.as_ref()),
        ] {
            let Some(target) = target else { continue };
            validate_video_transition_target_reference(
                target,
                bus.id,
                composition,
                &layers,
                &groups,
            )?;
            if role != "matte_source" && !members.contains(target) {
                return Err(project_reference_mismatch(
                    format!("video transition bus {} {role}", bus.id.0),
                    "target is not a member of the transition bus",
                ));
            }
        }
    }

    let mut chain_ids = BTreeSet::new();
    let mut stage_ids = BTreeSet::new();
    let mut effect_ids = BTreeSet::new();
    let mut scopes = BTreeSet::new();
    for chain in &video.effect_chains {
        if chain.id.0 == 0 {
            return Err(project_zero_id("video effect chain", "video effect chain"));
        }
        if !chain_ids.insert(chain.id) {
            return Err(project_duplicate_id("video effect chain", chain.id.0));
        }
        if !scopes.insert(chain.scope.clone()) {
            return Err(project_reference_mismatch(
                format!("video effect chain {}", chain.id.0),
                "duplicates an existing effect scope",
            ));
        }
        validate_video_effect_scope_reference(
            &chain.scope,
            VideoEffectScopeReferenceContext {
                layers: &layers,
                outputs: &outputs,
                compositions: &compositions,
                groups: &groups,
                buses: &buses,
                timelines,
            },
        )?;
        for stage in &chain.stages {
            if stage.id.0 == 0 {
                return Err(project_zero_id(
                    "video effect stage",
                    format!("video effect chain {}", chain.id.0),
                ));
            }
            if !stage_ids.insert(stage.id) {
                return Err(project_duplicate_id("video effect stage", stage.id.0));
            }
            if stage.effect.id.0 == 0 {
                return Err(project_zero_id(
                    "video effect",
                    format!("video effect stage {}", stage.id.0),
                ));
            }
            if !effect_ids.insert(stage.effect.id) {
                return Err(project_duplicate_id("video effect", stage.effect.id.0));
            }
        }
    }
    let mut preset_ids = BTreeSet::new();
    for preset in &video.effect_presets {
        if preset.id.0 == 0 {
            return Err(project_zero_id(
                "video effect preset",
                "video effect preset",
            ));
        }
        if !preset_ids.insert(preset.id) {
            return Err(project_duplicate_id("video effect preset", preset.id.0));
        }
    }
    for layer in &video.layers {
        for slot in &layer.clip_slots {
            for effect_override in &slot.effect_overrides {
                if let Some(effect_id) = effect_override.effect_id {
                    if !effect_ids.contains(&effect_id) {
                        return Err(project_missing_reference(
                            format!(
                                "video layer {} clip slot {} effect override",
                                layer.id, slot.id.0
                            ),
                            "video effect",
                            effect_id.0,
                        ));
                    }
                }
            }
        }
    }
    let mut eligible = BTreeSet::new();
    for layer_id in &video.auto_vj.config.eligible_layer_ids {
        if !eligible.insert(*layer_id) {
            return Err(project_reference_mismatch(
                "Auto VJ eligible layer list",
                format!("contains video layer {layer_id} more than once"),
            ));
        }
        if !layers.contains_key(layer_id) {
            return Err(project_missing_reference(
                "Auto VJ eligible layer list",
                "video layer",
                *layer_id,
            ));
        }
    }

    Ok(ProjectVideoCatalog {
        layers,
        media_assets,
        outputs,
    })
}

fn validate_video_transition_target_reference(
    target: &VideoLayerTransitionTarget,
    bus_id: VideoTransitionBusId,
    composition: &CompositionSummary,
    layers: &BTreeMap<u64, &VideoLayerSummary>,
    groups: &BTreeMap<u64, &VideoLayerGroupSummary>,
) -> Result<(), ProjectReferenceIntegrityError> {
    match target {
        VideoLayerTransitionTarget::Layer { layer_id } => {
            if !layers.contains_key(layer_id) {
                return Err(project_missing_reference(
                    format!("video transition bus {}", bus_id.0),
                    "video layer",
                    *layer_id,
                ));
            }
            if !composition.layer_ids.contains(layer_id) {
                return Err(project_reference_mismatch(
                    format!("video transition bus {}", bus_id.0),
                    format!("layer {layer_id} is outside composition {}", composition.id),
                ));
            }
        }
        VideoLayerTransitionTarget::Group { group_id } => {
            let group = groups.get(&group_id.0).ok_or_else(|| {
                project_missing_reference(
                    format!("video transition bus {}", bus_id.0),
                    "video layer group",
                    group_id.0,
                )
            })?;
            if group.composition_id != composition.id {
                return Err(project_reference_mismatch(
                    format!("video transition bus {}", bus_id.0),
                    format!(
                        "group {} belongs to composition {} instead of {}",
                        group_id.0, group.composition_id, composition.id
                    ),
                ));
            }
        }
    }
    Ok(())
}

struct VideoEffectScopeReferenceContext<'maps, 'snapshot> {
    layers: &'maps BTreeMap<u64, &'snapshot VideoLayerSummary>,
    outputs: &'maps BTreeMap<u64, &'snapshot VideoOutputSummary>,
    compositions: &'maps BTreeMap<u64, &'snapshot CompositionSummary>,
    groups: &'maps BTreeMap<u64, &'snapshot VideoLayerGroupSummary>,
    buses: &'maps BTreeMap<u64, &'snapshot VideoLayerTransitionBusSummary>,
    timelines: &'maps BTreeMap<u64, &'snapshot TimelineSnapshot>,
}

fn validate_video_effect_scope_reference<'maps, 'snapshot>(
    scope: &VideoEffectScope,
    context: VideoEffectScopeReferenceContext<'maps, 'snapshot>,
) -> Result<(), ProjectReferenceIntegrityError> {
    let VideoEffectScopeReferenceContext {
        layers,
        outputs,
        compositions,
        groups,
        buses,
        timelines,
    } = context;
    let owner = format!("video effect scope {scope:?}");
    match scope {
        VideoEffectScope::Clip { layer_id, slot_id } => {
            let layer = layers
                .get(layer_id)
                .ok_or_else(|| project_missing_reference(&owner, "video layer", *layer_id))?;
            if !layer.clip_slots.iter().any(|slot| slot.id == *slot_id) {
                return Err(project_missing_reference(
                    owner,
                    "video clip slot",
                    slot_id.0,
                ));
            }
        }
        VideoEffectScope::Layer { layer_id }
        | VideoEffectScope::Transition {
            owner: VideoTransitionEffectOwner::ClipTake { layer_id },
        } => {
            if !layers.contains_key(layer_id) {
                return Err(project_missing_reference(owner, "video layer", *layer_id));
            }
        }
        VideoEffectScope::Transition {
            owner: VideoTransitionEffectOwner::LayerBus { bus_id },
        } => {
            if !buses.contains_key(&bus_id.0) {
                return Err(project_missing_reference(
                    owner,
                    "video transition bus",
                    bus_id.0,
                ));
            }
        }
        VideoEffectScope::Transition {
            owner: VideoTransitionEffectOwner::TimelineFollow { source_timeline_id },
        } => {
            if !timelines.contains_key(&source_timeline_id.0) {
                return Err(project_missing_reference(
                    owner,
                    "Timeline",
                    source_timeline_id.0,
                ));
            }
        }
        VideoEffectScope::Composition { composition_id } => {
            if !compositions.contains_key(composition_id) {
                return Err(project_missing_reference(
                    owner,
                    "video composition",
                    *composition_id,
                ));
            }
        }
        VideoEffectScope::Group { group_id } => {
            if !groups.contains_key(&group_id.0) {
                return Err(project_missing_reference(
                    owner,
                    "video layer group",
                    group_id.0,
                ));
            }
        }
        VideoEffectScope::Output { output_id } => {
            if !outputs.contains_key(output_id) {
                return Err(project_missing_reference(owner, "video output", *output_id));
            }
        }
    }
    Ok(())
}

fn authored_video_conflict(
    kind: ProjectAuthoredVideoConflictKind,
    detail: impl Into<String>,
) -> ProjectReferenceIntegrityError {
    ProjectReferenceIntegrityError::AuthoredVideoConflict {
        kind,
        detail: detail.into(),
    }
}

fn validate_authored_video_reference_projection(
    snapshot: &EngineSnapshot,
    authored: &VideoSnapshot,
) -> Result<(), ProjectReferenceIntegrityError> {
    let rendered = &snapshot.video;
    let authored_layer_ids = project_unique_id_set(
        authored.layers.iter().map(|layer| layer.id),
        "authored video layer",
    )?;
    let rendered_layer_ids = project_unique_id_set(
        rendered.layers.iter().map(|layer| layer.id),
        "rendered video layer",
    )?;
    let mut runtime_projection_ids = BTreeSet::new();
    for layer_id in &snapshot
        .video_clip_runtime
        .timeline_video_projection_layer_ids
    {
        if *layer_id == 0 || !runtime_projection_ids.insert(*layer_id) {
            return Err(authored_video_conflict(
                ProjectAuthoredVideoConflictKind::RuntimeProjectionDeclaration,
                format!("runtime projection layer ID {layer_id} is zero or duplicated"),
            ));
        }
        if authored_layer_ids.contains(layer_id) || !rendered_layer_ids.contains(layer_id) {
            return Err(authored_video_conflict(
                ProjectAuthoredVideoConflictKind::RuntimeProjectionDeclaration,
                format!(
                    "runtime projection layer {layer_id} is authored or absent from rendered video"
                ),
            ));
        }
    }
    let expected_rendered_layers = authored_layer_ids
        .union(&runtime_projection_ids)
        .copied()
        .collect::<BTreeSet<_>>();
    if rendered_layer_ids != expected_rendered_layers {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
            "rendered layer IDs are not exactly authored IDs plus declared runtime projections",
        ));
    }

    let authored_assets = project_unique_id_set(
        authored.media_assets.iter().map(|asset| asset.id),
        "authored MediaAsset",
    )?;
    let rendered_assets = project_unique_id_set(
        rendered.media_assets.iter().map(|asset| asset.id),
        "rendered MediaAsset",
    )?;
    if authored_assets != rendered_assets {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
            "MediaAsset identity sets differ",
        ));
    }

    for authored_layer in &authored.layers {
        let rendered_layer = rendered
            .layers
            .iter()
            .find(|layer| layer.id == authored_layer.id)
            .ok_or_else(|| {
                authored_video_conflict(
                    ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
                    format!(
                        "authored layer {} is absent from rendered video",
                        authored_layer.id
                    ),
                )
            })?;
        let authored_slots = authored_layer
            .clip_slots
            .iter()
            .map(|slot| (slot.id, slot.media_asset_id))
            .collect::<Vec<_>>();
        let rendered_slots = rendered_layer
            .clip_slots
            .iter()
            .map(|slot| (slot.id, slot.media_asset_id))
            .collect::<Vec<_>>();
        if authored_layer.default_clip_slot_id != rendered_layer.default_clip_slot_id
            || authored_slots != rendered_slots
        {
            return Err(authored_video_conflict(
                ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
                format!("layer {} clip-bank topology differs", authored_layer.id),
            ));
        }
    }

    let authored_compositions = project_unique_id_set(
        authored
            .compositions
            .iter()
            .map(|composition| composition.id),
        "authored video composition",
    )?;
    let rendered_compositions = project_unique_id_set(
        rendered
            .compositions
            .iter()
            .map(|composition| composition.id),
        "rendered video composition",
    )?;
    if authored_compositions != rendered_compositions {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
            "video composition identity sets differ",
        ));
    }
    for authored_composition in &authored.compositions {
        let rendered_composition = rendered
            .compositions
            .iter()
            .find(|composition| composition.id == authored_composition.id)
            .expect("matching composition identity set was checked");
        let rendered_authored_layers = rendered_composition
            .layer_ids
            .iter()
            .filter(|layer_id| !runtime_projection_ids.contains(layer_id))
            .copied()
            .collect::<Vec<_>>();
        if rendered_authored_layers != authored_composition.layer_ids
            || rendered_composition.output_ids != authored_composition.output_ids
        {
            return Err(authored_video_conflict(
                ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
                format!(
                    "composition {} layer/output topology differs",
                    authored_composition.id
                ),
            ));
        }
    }

    let authored_outputs = project_unique_id_set(
        authored.outputs.iter().map(|output| output.id),
        "authored video output",
    )?;
    let rendered_outputs = project_unique_id_set(
        rendered.outputs.iter().map(|output| output.id),
        "rendered video output",
    )?;
    if authored_outputs != rendered_outputs {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
            "video output identity sets differ",
        ));
    }
    for authored_output in &authored.outputs {
        let rendered_output = rendered
            .outputs
            .iter()
            .find(|output| output.id == authored_output.id)
            .expect("matching output identity set was checked");
        if authored_output.composition_id != rendered_output.composition_id {
            return Err(authored_video_conflict(
                ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
                format!("output {} composition host differs", authored_output.id),
            ));
        }
    }

    compare_authored_video_group_topology(authored, rendered)?;
    compare_authored_video_bus_topology(authored, rendered)?;
    compare_authored_video_effect_topology(authored, rendered)?;
    compare_authored_video_persisted_state(authored, rendered)?;
    Ok(())
}

/// Exact fences over the remaining persisted `VideoSnapshot` state.
///
/// These fields are authored project state, not runtime-only render output:
/// the engine derives `authored_video` from the same source image it renders
/// from, so a divergence between the two images means the persisted file is
/// internally inconsistent and is rejected instead of silently picking one
/// side. `auto_vj.status` is deliberately excluded: Auto VJ runtime status is
/// reset on every project load and never participates in authored authority.
fn compare_authored_video_persisted_state(
    authored: &VideoSnapshot,
    rendered: &VideoSnapshot,
) -> Result<(), ProjectReferenceIntegrityError> {
    if authored.mapping_presets != rendered.mapping_presets {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
            "video output mapping presets differ between authored and rendered video",
        ));
    }
    if authored.master_opacity != rendered.master_opacity {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
            "video master opacity differs between authored and rendered video",
        ));
    }
    if authored.blackout != rendered.blackout {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
            "authored blackout bit differs between authored and rendered video",
        ));
    }
    if authored.auto_vj.config != rendered.auto_vj.config {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
            "Auto VJ config (including eligible layer list) differs between \
             authored and rendered video",
        ));
    }
    Ok(())
}

fn project_unique_id_set(
    values: impl Iterator<Item = u64>,
    domain: &'static str,
) -> Result<BTreeSet<u64>, ProjectReferenceIntegrityError> {
    let mut ids = BTreeSet::new();
    for id in values {
        if id == 0 {
            return Err(authored_video_conflict(
                ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
                format!("{domain} contains ID zero"),
            ));
        }
        if !ids.insert(id) {
            return Err(authored_video_conflict(
                ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
                format!("{domain} contains duplicate ID {id}"),
            ));
        }
    }
    Ok(ids)
}

fn compare_authored_video_group_topology(
    authored: &VideoSnapshot,
    rendered: &VideoSnapshot,
) -> Result<(), ProjectReferenceIntegrityError> {
    let authored_ids = project_unique_id_set(
        authored.layer_groups.iter().map(|group| group.id.0),
        "authored video layer group",
    )?;
    let rendered_ids = project_unique_id_set(
        rendered.layer_groups.iter().map(|group| group.id.0),
        "rendered video layer group",
    )?;
    if authored_ids != rendered_ids {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
            "video layer-group identity sets differ",
        ));
    }
    for group in &authored.layer_groups {
        let peer = rendered
            .layer_groups
            .iter()
            .find(|peer| peer.id == group.id)
            .expect("matching group identity set was checked");
        if group.composition_id != peer.composition_id || group.layer_ids != peer.layer_ids {
            return Err(authored_video_conflict(
                ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
                format!("video layer group {} topology differs", group.id.0),
            ));
        }
    }
    Ok(())
}

fn compare_authored_video_bus_topology(
    authored: &VideoSnapshot,
    rendered: &VideoSnapshot,
) -> Result<(), ProjectReferenceIntegrityError> {
    let authored_ids = project_unique_id_set(
        authored.transition_buses.iter().map(|bus| bus.id.0),
        "authored video transition bus",
    )?;
    let rendered_ids = project_unique_id_set(
        rendered.transition_buses.iter().map(|bus| bus.id.0),
        "rendered video transition bus",
    )?;
    if authored_ids != rendered_ids {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
            "video transition-bus identity sets differ",
        ));
    }
    for bus in &authored.transition_buses {
        let peer = rendered
            .transition_buses
            .iter()
            .find(|peer| peer.id == bus.id)
            .expect("matching transition-bus identity set was checked");
        if bus.composition_id != peer.composition_id
            || bus.members != peer.members
            || bus.default_from != peer.default_from
            || bus.default_to != peer.default_to
            || bus.matte_source != peer.matte_source
        {
            return Err(authored_video_conflict(
                ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
                format!("video transition bus {} topology differs", bus.id.0),
            ));
        }
    }
    Ok(())
}

fn compare_authored_video_effect_topology(
    authored: &VideoSnapshot,
    rendered: &VideoSnapshot,
) -> Result<(), ProjectReferenceIntegrityError> {
    let authored_ids = project_unique_id_set(
        authored.effect_chains.iter().map(|chain| chain.id.0),
        "authored video effect chain",
    )?;
    let rendered_ids = project_unique_id_set(
        rendered.effect_chains.iter().map(|chain| chain.id.0),
        "rendered video effect chain",
    )?;
    if authored_ids != rendered_ids {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
            "video effect-chain identity sets differ",
        ));
    }
    for chain in &authored.effect_chains {
        let peer = rendered
            .effect_chains
            .iter()
            .find(|peer| peer.id == chain.id)
            .expect("matching effect-chain identity set was checked");
        let stages = chain
            .stages
            .iter()
            .map(|stage| (stage.id, stage.effect.id))
            .collect::<Vec<_>>();
        let peer_stages = peer
            .stages
            .iter()
            .map(|stage| (stage.id, stage.effect.id))
            .collect::<Vec<_>>();
        if chain.scope != peer.scope || stages != peer_stages {
            return Err(authored_video_conflict(
                ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
                format!("video effect chain {} topology differs", chain.id.0),
            ));
        }
    }
    let authored_presets = project_unique_id_set(
        authored.effect_presets.iter().map(|preset| preset.id.0),
        "authored video effect preset",
    )?;
    let rendered_presets = project_unique_id_set(
        rendered.effect_presets.iter().map(|preset| preset.id.0),
        "rendered video effect preset",
    )?;
    if authored_presets != rendered_presets {
        return Err(authored_video_conflict(
            ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
            "video effect-preset identity sets differ",
        ));
    }
    Ok(())
}

fn validate_active_cue_references(
    snapshot: &EngineSnapshot,
    cue_lists: &BTreeMap<u64, &CueListSummary>,
    cues: &BTreeMap<u64, &CueSummary>,
) -> Result<(), ProjectReferenceIntegrityError> {
    for cue in &snapshot.cues {
        if !cue_lists.contains_key(&cue.cue_list_id) {
            return Err(project_missing_reference(
                format!("Cue {}", cue.id),
                "Cue List",
                cue.cue_list_id,
            ));
        }
    }
    for cue_list in &snapshot.cue_lists {
        let Some(active_cue_id) = cue_list.active_cue_id else {
            continue;
        };
        let cue = cues.get(&active_cue_id).ok_or_else(|| {
            project_missing_reference(
                format!("Cue List {} active Cue", cue_list.id),
                "Cue",
                active_cue_id,
            )
        })?;
        if cue.cue_list_id != cue_list.id {
            return Err(project_reference_mismatch(
                format!("Cue List {} active Cue", cue_list.id),
                format!(
                    "Cue {active_cue_id} belongs to Cue List {}",
                    cue.cue_list_id
                ),
            ));
        }
    }
    if let Some(active_cue_id) = snapshot.active_cue_id {
        let cue = cues.get(&active_cue_id).ok_or_else(|| {
            project_missing_reference("snapshot active Cue", "Cue", active_cue_id)
        })?;
        let cue_list = cue_lists
            .get(&cue.cue_list_id)
            .expect("Cue membership was validated above");
        if cue_list.active_cue_id != Some(active_cue_id) {
            return Err(project_reference_mismatch(
                "snapshot active Cue",
                format!(
                    "Cue {active_cue_id} is not the active Cue of its Cue List {}",
                    cue.cue_list_id
                ),
            ));
        }
        if let Some(group_id) = &cue.group_id {
            if snapshot.active_group_cue_ids.get(group_id) != Some(&active_cue_id) {
                return Err(project_reference_mismatch(
                    "snapshot active Cue",
                    format!("group {group_id:?} does not point back to Cue {active_cue_id}"),
                ));
            }
        }
    }
    for (group_id, cue_id) in &snapshot.active_group_cue_ids {
        let cue = cues.get(cue_id).ok_or_else(|| {
            project_missing_reference(format!("active Cue group {group_id:?}"), "Cue", *cue_id)
        })?;
        if cue.group_id.as_deref() != Some(group_id.as_str()) {
            return Err(project_reference_mismatch(
                format!("active Cue group {group_id:?}"),
                format!("Cue {cue_id} is authored for group {:?}", cue.group_id),
            ));
        }
    }
    Ok(())
}

fn fixture_attribute_catalog(
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
) -> BTreeMap<FixtureId, BTreeSet<String>> {
    fixtures
        .iter()
        .map(|(fixture_id, fixture)| {
            let attributes = fixture
                .controls
                .iter()
                .map(|control| control.attribute.clone())
                .chain(
                    fixture
                        .attribute_values
                        .iter()
                        .map(|value| value.attribute.clone()),
                )
                .collect::<BTreeSet<_>>();
            (*fixture_id, attributes)
        })
        .collect()
}

fn validate_fixture_attribute_reference(
    owner: impl Into<String>,
    fixture_id: FixtureId,
    attribute: &str,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    let owner = owner.into();
    if !fixtures.contains_key(&fixture_id) {
        return Err(project_missing_reference(owner, "fixture", fixture_id));
    }
    if attribute.trim().is_empty()
        || !fixture_attributes
            .get(&fixture_id)
            .is_some_and(|attributes| attributes.contains(attribute))
    {
        return Err(project_reference_mismatch(
            owner,
            format!("fixture {fixture_id} has no attribute {attribute:?}"),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MoveReferenceAxis {
    Pan,
    Tilt,
}

/// Keep static project validation byte-for-byte aligned with the engine's
/// movement-axis classifier: ASCII alphanumeric normalization, Tilt taking
/// precedence over Pan, and substring matching for profile-specific aliases.
fn move_reference_axis(attribute: &str) -> Option<MoveReferenceAxis> {
    let normalized = attribute
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect::<String>();
    if normalized.contains("tilt") {
        Some(MoveReferenceAxis::Tilt)
    } else if normalized.contains("pan") {
        Some(MoveReferenceAxis::Pan)
    } else {
        None
    }
}

/// Mirror the runtime's indexed Pan/Tilt pairing. A missing axis deliberately
/// leaves a hole instead of falling back to another control or beam.
fn move_reference_attribute_pairs(fixture: &PatchedFixtureSummary) -> Vec<Option<(&str, &str)>> {
    let pan_controls = fixture
        .controls
        .iter()
        .filter(|control| move_reference_axis(&control.attribute) == Some(MoveReferenceAxis::Pan))
        .collect::<Vec<_>>();
    let tilt_controls = fixture
        .controls
        .iter()
        .filter(|control| move_reference_axis(&control.attribute) == Some(MoveReferenceAxis::Tilt))
        .collect::<Vec<_>>();
    let pair_count = pan_controls.len().max(tilt_controls.len());
    (0..pair_count)
        .map(|index| {
            let (Some(pan), Some(tilt)) = (pan_controls.get(index), tilt_controls.get(index))
            else {
                return None;
            };
            Some((pan.attribute.as_str(), tilt.attribute.as_str()))
        })
        .collect()
}

struct CueProjectReferenceContext<'maps, 'snapshot> {
    cue_lists: &'maps BTreeMap<u64, &'snapshot CueListSummary>,
    fixtures: &'maps BTreeMap<u64, &'snapshot PatchedFixtureSummary>,
    fixture_attributes: &'maps BTreeMap<FixtureId, BTreeSet<String>>,
    palettes: &'maps BTreeMap<u64, &'snapshot ReferencePaletteSummary>,
    effects: &'maps BTreeMap<u64, &'snapshot EffectSummary>,
    node_graphs: &'maps BTreeMap<u64, &'snapshot NodeGraphSummary>,
    video: &'maps ProjectVideoCatalog<'snapshot>,
}

fn validate_cue_project_references<'maps, 'snapshot>(
    snapshot: &EngineSnapshot,
    context: CueProjectReferenceContext<'maps, 'snapshot>,
) -> Result<(), ProjectReferenceIntegrityError> {
    let CueProjectReferenceContext {
        cue_lists,
        fixtures,
        fixture_attributes,
        palettes,
        effects,
        node_graphs,
        video,
    } = context;
    let mut executor_ids = BTreeSet::new();
    for executor in &snapshot.playback_executors {
        if executor.id == 0 {
            return Err(project_zero_id("playback executor", "playback executor"));
        }
        if !executor_ids.insert(executor.id) {
            return Err(project_duplicate_id("playback executor", executor.id));
        }
        if !cue_lists.contains_key(&executor.cue_list_id) {
            return Err(project_missing_reference(
                format!("playback executor {}", executor.id),
                "Cue List",
                executor.cue_list_id,
            ));
        }
    }
    for cue in &snapshot.cues {
        let cue_owner = format!("Cue {}", cue.id);
        let mut target_fixtures = BTreeSet::new();
        for target in &cue.targets {
            if !target_fixtures.insert(target.fixture_id) {
                return Err(project_reference_mismatch(
                    &cue_owner,
                    format!("fixture {} is targeted more than once", target.fixture_id),
                ));
            }
            for value in &target.values {
                validate_fixture_attribute_reference(
                    format!("{cue_owner} fixture target"),
                    target.fixture_id,
                    &value.attribute,
                    fixtures,
                    fixture_attributes,
                )?;
            }
        }
        for (step_index, step) in cue.steps.iter().enumerate() {
            for target in &step.values {
                for value in &target.values {
                    validate_fixture_attribute_reference(
                        format!("{cue_owner} step {step_index}"),
                        target.fixture_id,
                        &value.attribute,
                        fixtures,
                        fixture_attributes,
                    )?;
                }
            }
        }
        for fixture_id in &cue.mib_fixture_ids {
            if !fixtures.contains_key(fixture_id) {
                return Err(project_missing_reference(
                    format!("{cue_owner} MIB fixture"),
                    "fixture",
                    *fixture_id,
                ));
            }
        }
        for part in &cue.parts {
            for fixture_id in &part.fixture_ids {
                if !fixtures.contains_key(fixture_id) {
                    return Err(project_missing_reference(
                        format!("{cue_owner} part {}", part.number),
                        "fixture",
                        *fixture_id,
                    ));
                }
            }
            for layer_id in &part.video_layer_ids {
                if !video.layers.contains_key(layer_id) {
                    return Err(project_missing_reference(
                        format!("{cue_owner} part {}", part.number),
                        "video layer",
                        *layer_id,
                    ));
                }
            }
            for output_id in &part.video_output_ids {
                if !video.outputs.contains_key(output_id) {
                    return Err(project_missing_reference(
                        format!("{cue_owner} part {}", part.number),
                        "video output",
                        *output_id,
                    ));
                }
            }
        }
        for palette_target in &cue.palette_targets {
            let palette = palettes.get(&palette_target.palette_id).ok_or_else(|| {
                project_missing_reference(
                    format!("{cue_owner} palette target"),
                    "palette",
                    palette_target.palette_id,
                )
            })?;
            for fixture_id in &palette_target.fixture_ids {
                for value in &palette.values {
                    validate_fixture_attribute_reference(
                        format!("{cue_owner} palette {}", palette.id),
                        *fixture_id,
                        &value.attribute,
                        fixtures,
                        fixture_attributes,
                    )?;
                }
            }
        }
        for target in &cue.video_targets {
            if !video.layers.contains_key(&target.layer_id) {
                return Err(project_missing_reference(
                    &cue_owner,
                    "video layer",
                    target.layer_id,
                ));
            }
        }
        for target in &cue.video_output_targets {
            if !video.outputs.contains_key(&target.output_id) {
                return Err(project_missing_reference(
                    &cue_owner,
                    "video output",
                    target.output_id,
                ));
            }
        }
        for target in &cue.effect_targets {
            if !effects.contains_key(&target.effect_id) {
                return Err(project_missing_reference(
                    &cue_owner,
                    "lighting effect",
                    target.effect_id,
                ));
            }
            if let Some(params) = &target.params {
                validate_effect_params_snapshot_references(
                    format!("{cue_owner} owned effect {} request", target.effect_id),
                    params,
                    fixtures,
                    fixture_attributes,
                )?;
            }
        }
        for target in &cue.node_graph_targets {
            if !node_graphs.contains_key(&target.graph_id) {
                return Err(project_missing_reference(
                    &cue_owner,
                    "node graph",
                    target.graph_id,
                ));
            }
        }
    }
    Ok(())
}

fn validate_effect_and_node_graph_references(
    snapshot: &EngineSnapshot,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
    video: &ProjectVideoCatalog<'_>,
) -> Result<(), ProjectReferenceIntegrityError> {
    for effect in &snapshot.effects {
        if effect.effect_type == EffectKind::Move && effect.attribute != "Pan/Tilt" {
            return Err(project_reference_mismatch(
                format!("lighting effect {}", effect.id),
                format!(
                    "Move summary attribute must be the exact virtual attribute \"Pan/Tilt\", got {:?}",
                    effect.attribute
                ),
            ));
        }
        for fixture_id in &effect.fixture_ids {
            if effect.effect_type == EffectKind::Move {
                if !fixtures.contains_key(fixture_id) {
                    return Err(project_missing_reference(
                        format!("lighting effect {}", effect.id),
                        "fixture",
                        *fixture_id,
                    ));
                }
            } else {
                validate_fixture_attribute_reference(
                    format!("lighting effect {}", effect.id),
                    *fixture_id,
                    &effect.attribute,
                    fixtures,
                    fixture_attributes,
                )?;
            }
        }
        validate_effect_summary_payload_references(effect, fixtures, fixture_attributes)?;
        for target in &effect.video_targets {
            for layer_id in &target.layer_ids {
                if !video.layers.contains_key(layer_id) {
                    return Err(project_missing_reference(
                        format!("lighting effect {} video target", effect.id),
                        "video layer",
                        *layer_id,
                    ));
                }
            }
        }
    }
    for graph in &snapshot.node_graphs {
        let mut node_ids = BTreeSet::new();
        for node in &graph.nodes {
            if node.id == 0 {
                return Err(project_zero_id(
                    "node graph node",
                    format!("node graph {}", graph.id),
                ));
            }
            if !node_ids.insert(node.id) {
                return Err(project_duplicate_id("node graph node", node.id));
            }
            if let Some(output) = &node.output {
                for fixture_id in &output.fixture_ids {
                    validate_fixture_attribute_reference(
                        format!("node graph {} node {}", graph.id, node.id),
                        *fixture_id,
                        &output.attribute,
                        fixtures,
                        fixture_attributes,
                    )?;
                }
                for target in &output.video_targets {
                    for layer_id in &target.layer_ids {
                        if !video.layers.contains_key(layer_id) {
                            return Err(project_missing_reference(
                                format!("node graph {} node {}", graph.id, node.id),
                                "video layer",
                                *layer_id,
                            ));
                        }
                    }
                }
            }
        }
        for edge in &graph.edges {
            for (role, node_id) in [("from", edge.from_node), ("to", edge.to_node)] {
                if !node_ids.contains(&node_id) {
                    return Err(project_missing_reference(
                        format!("node graph {} edge {role}", graph.id),
                        "node graph node",
                        node_id,
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Validate every fixture/attribute reference embedded in one `EffectSummary`
/// request payload, independently of the summary mirrors. The summary fields
/// are checked separately; a hostile file must not be able to smuggle a
/// dangling fixture or unsupported attribute through an embedded request just
/// because the mirrored summary happens to be consistent.
fn validate_effect_summary_payload_references(
    effect: &EffectSummary,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    if let Some(request) = &effect.lfo {
        validate_lfo_request_payload(
            &format!("lighting effect {} LFO request", effect.id),
            request,
            fixtures,
            fixture_attributes,
        )?;
    }
    if let Some(request) = &effect.color {
        validate_color_request_payload(
            &format!("lighting effect {} Color request", effect.id),
            request,
            fixtures,
            fixture_attributes,
        )?;
    }
    if let Some(request) = &effect.chaser {
        validate_chaser_request_payload(
            &format!("lighting effect {} Chaser request", effect.id),
            request,
            fixtures,
            fixture_attributes,
        )?;
    }
    if let Some(request) = &effect.move_effect {
        validate_move_request_payload(
            &format!("lighting effect {} Move request", effect.id),
            request,
            fixtures,
        )?;
    }
    if let Some(request) = &effect.value {
        validate_value_request_payload(
            &format!("lighting effect {} Value request", effect.id),
            request,
            fixtures,
            fixture_attributes,
        )?;
    }
    if let Some(request) = &effect.curve {
        validate_curve_request_payload(
            &format!("lighting effect {} Curve request", effect.id),
            request,
            fixtures,
            fixture_attributes,
        )?;
    }
    if let Some(request) = &effect.mapping {
        validate_mapping_request_payload(
            &format!("lighting effect {} Mapping request", effect.id),
            request,
            fixtures,
            fixture_attributes,
        )?;
    }
    if let Some(request) = &effect.color_mapping {
        validate_color_mapping_request_payload(
            &format!("lighting effect {} ColorMapping request", effect.id),
            request,
            fixtures,
            fixture_attributes,
        )?;
    }
    Ok(())
}

/// Validate the Cue-owned embedded effect request of a Cue effect target.
fn validate_effect_params_snapshot_references(
    owner: String,
    params: &EffectParamsSnapshot,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    match params {
        EffectParamsSnapshot::Lfo(request) => {
            validate_lfo_request_payload(&owner, request, fixtures, fixture_attributes)
        }
        EffectParamsSnapshot::PositionWave(request) => {
            validate_position_wave_request_payload(&owner, request, fixtures, fixture_attributes)
        }
        EffectParamsSnapshot::Color(request) => {
            validate_color_request_payload(&owner, request, fixtures, fixture_attributes)
        }
        EffectParamsSnapshot::Chaser(request) => {
            validate_chaser_request_payload(&owner, request, fixtures, fixture_attributes)
        }
        EffectParamsSnapshot::Move(request) => {
            validate_move_request_payload(&owner, request, fixtures)
        }
        EffectParamsSnapshot::Value(request) => {
            validate_value_request_payload(&owner, request, fixtures, fixture_attributes)
        }
        EffectParamsSnapshot::Curve(request) => {
            validate_curve_request_payload(&owner, request, fixtures, fixture_attributes)
        }
        EffectParamsSnapshot::Mapping(request) => {
            validate_mapping_request_payload(&owner, request, fixtures, fixture_attributes)
        }
        EffectParamsSnapshot::ColorMapping(request) => {
            validate_color_mapping_request_payload(&owner, request, fixtures, fixture_attributes)
        }
    }
}

/// Every listed fixture must exist and support `attribute` exactly.
fn validate_payload_attribute_targets(
    owner: &str,
    fixture_ids: &[FixtureId],
    attribute: &str,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    for fixture_id in fixture_ids {
        validate_fixture_attribute_reference(
            owner,
            *fixture_id,
            attribute,
            fixtures,
            fixture_attributes,
        )?;
    }
    Ok(())
}

/// Every listed fixture must exist. Used by families whose fixture list does
/// not itself carry an attribute reference.
fn validate_payload_fixture_existence(
    owner: &str,
    fixture_ids: &[FixtureId],
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
) -> Result<(), ProjectReferenceIntegrityError> {
    for fixture_id in fixture_ids {
        if !fixtures.contains_key(fixture_id) {
            return Err(project_missing_reference(owner, "fixture", *fixture_id));
        }
    }
    Ok(())
}

/// A feature attribute applies to whichever targeted fixtures support it, so
/// the deterministic acceptance rule mirrors the engine's resolution
/// semantics: at least one explicit target fixture must support it. When a
/// family targets groups only, membership lives behind the external group
/// registry boundary and cannot be decided here.
fn validate_payload_feature_attribute_resolves(
    owner: &str,
    attribute: &str,
    candidate_fixture_ids: impl IntoIterator<Item = FixtureId>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    let mut candidates = candidate_fixture_ids.into_iter().collect::<Vec<_>>();
    candidates.sort_unstable();
    candidates.dedup();
    let resolves = candidates.iter().any(|fixture_id| {
        fixture_attributes
            .get(fixture_id)
            .is_some_and(|attributes| attributes.contains(attribute))
    });
    if !resolves && !candidates.is_empty() {
        return Err(project_reference_mismatch(
            owner,
            format!(
                "feature attribute {attribute:?} is supported by none of the explicit target fixtures"
            ),
        ));
    }
    Ok(())
}

fn validate_effect_beam_target(
    owner: &str,
    target: &EffectBeamTarget,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    validate_fixture_attribute_reference(
        format!("{owner} beam target"),
        target.fixture_id,
        &target.feature_attribute,
        fixtures,
        fixture_attributes,
    )
}

fn validate_color_effect_beam_target(
    owner: &str,
    target: &ColorEffectBeamTarget,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    if !fixtures.contains_key(&target.fixture_id) {
        return Err(project_missing_reference(
            format!("{owner} beam target"),
            "fixture",
            target.fixture_id,
        ));
    }
    if let Some(feature_attribute) = &target.feature_attribute {
        return validate_fixture_attribute_reference(
            format!("{owner} beam target"),
            target.fixture_id,
            feature_attribute,
            fixtures,
            fixture_attributes,
        );
    }
    Ok(())
}

fn validate_color_spatial_pattern_payload(
    owner: &str,
    pattern: &ColorEffectSpatialPattern,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    for target in &pattern.beam_targets {
        validate_color_effect_beam_target(owner, target, fixtures, fixture_attributes)?;
    }
    if let Some(placement) = &pattern.placement {
        for coordinate in &placement.target_coordinates {
            if !fixtures.contains_key(&coordinate.fixture_id) {
                return Err(project_missing_reference(
                    format!("{owner} placement target"),
                    "fixture",
                    coordinate.fixture_id,
                ));
            }
        }
    }
    Ok(())
}

fn validate_lfo_request_payload(
    owner: &str,
    request: &LfoEffectRequest,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    validate_payload_attribute_targets(
        owner,
        &request.fixture_ids,
        &request.attribute,
        fixtures,
        fixture_attributes,
    )?;
    for target in &request.beam_targets {
        validate_effect_beam_target(owner, target, fixtures, fixture_attributes)?;
    }
    Ok(())
}

fn validate_position_wave_request_payload(
    owner: &str,
    request: &PositionWaveEffectRequest,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    validate_payload_attribute_targets(
        owner,
        &request.fixture_ids,
        &request.attribute,
        fixtures,
        fixture_attributes,
    )
}

fn validate_color_request_payload(
    owner: &str,
    request: &ColorEffectRequest,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    validate_payload_fixture_existence(owner, &request.fixture_ids, fixtures)?;
    if let Some(pattern) = &request.spatial_pattern {
        validate_color_spatial_pattern_payload(owner, pattern, fixtures, fixture_attributes)?;
    }
    Ok(())
}

fn validate_chaser_request_payload(
    owner: &str,
    request: &ChaserEffectRequest,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    let mut explicit_targets = BTreeSet::new();
    for (step_index, step) in request.steps.iter().enumerate() {
        let step_owner = format!("{owner} step {step_index}");
        validate_payload_fixture_existence(&step_owner, &step.fixture_ids, fixtures)?;
        explicit_targets.extend(step.fixture_ids.iter().copied());
        for target in &step.beam_targets {
            validate_effect_beam_target(&step_owner, target, fixtures, fixture_attributes)?;
            explicit_targets.insert(target.fixture_id);
        }
    }
    for feature in &request.features {
        validate_payload_feature_attribute_resolves(
            &format!("{owner} feature"),
            &feature.attribute,
            explicit_targets.iter().copied(),
            fixture_attributes,
        )?;
    }
    Ok(())
}

fn validate_move_request_payload(
    owner: &str,
    request: &MoveEffectRequest,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
) -> Result<(), ProjectReferenceIntegrityError> {
    validate_payload_fixture_existence(owner, &request.fixture_ids, fixtures)?;
    if !request.beam_targets.is_empty() && !request.target_group_ids.is_empty() {
        return Err(project_reference_mismatch(
            owner,
            "Move beam targets cannot be combined with group targets".to_string(),
        ));
    }
    if request.beam_targets.is_empty() {
        for fixture_id in &request.fixture_ids {
            let fixture = fixtures
                .get(fixture_id)
                .expect("fixture existence was validated above");
            let pairs = move_reference_attribute_pairs(fixture);
            if pairs.len() != 1 || pairs[0].is_none() {
                return Err(project_reference_mismatch(
                    owner,
                    format!(
                        "fixture {fixture_id} must expose exactly one indexed Pan/Tilt pair when no Move beam target is authored"
                    ),
                ));
            }
        }
    } else {
        for target in &request.beam_targets {
            let Some(fixture) = fixtures.get(&target.fixture_id) else {
                return Err(project_missing_reference(
                    format!("{owner} beam target"),
                    "fixture",
                    target.fixture_id,
                ));
            };
            let fixture_is_targeted = request.fixture_ids.contains(&target.fixture_id);
            if !fixture_is_targeted {
                return Err(project_reference_mismatch(
                    format!("{owner} beam target"),
                    format!(
                        "fixture {} is not present in the resolved fixture targets",
                        target.fixture_id
                    ),
                ));
            }
            let pairs = move_reference_attribute_pairs(fixture);
            if !pairs
                .get(target.beam_index as usize)
                .is_some_and(Option::is_some)
            {
                return Err(project_reference_mismatch(
                    format!("{owner} beam target"),
                    format!(
                        "fixture {} has no paired Pan/Tilt beam {}",
                        target.fixture_id, target.beam_index
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn validate_value_request_payload(
    owner: &str,
    request: &ValueEffectRequest,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    validate_payload_fixture_existence(owner, &request.fixture_ids, fixtures)?;
    if request.features.is_empty() {
        validate_payload_attribute_targets(
            owner,
            &request.fixture_ids,
            &request.attribute,
            fixtures,
            fixture_attributes,
        )?;
    } else {
        for feature in &request.features {
            validate_payload_feature_attribute_resolves(
                &format!("{owner} feature"),
                &feature.attribute,
                request.fixture_ids.iter().copied(),
                fixture_attributes,
            )?;
        }
    }
    if let Some(pattern) = &request.spatial_pattern {
        validate_color_spatial_pattern_payload(owner, pattern, fixtures, fixture_attributes)?;
    }
    Ok(())
}

fn validate_curve_or_mapping_request_payload(
    owner: &str,
    fixture_ids: &[FixtureId],
    attribute: &str,
    features: &[ChaserFeature],
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    validate_payload_fixture_existence(owner, fixture_ids, fixtures)?;
    if features.is_empty() {
        validate_payload_attribute_targets(
            owner,
            fixture_ids,
            attribute,
            fixtures,
            fixture_attributes,
        )?;
    } else {
        for feature in features {
            validate_payload_feature_attribute_resolves(
                &format!("{owner} feature"),
                &feature.attribute,
                fixture_ids.iter().copied(),
                fixture_attributes,
            )?;
        }
    }
    Ok(())
}

fn validate_curve_request_payload(
    owner: &str,
    request: &CurveEffectRequest,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    validate_curve_or_mapping_request_payload(
        owner,
        &request.fixture_ids,
        &request.attribute,
        &request.features,
        fixtures,
        fixture_attributes,
    )
}

fn validate_mapping_request_payload(
    owner: &str,
    request: &MappingEffectRequest,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    validate_curve_or_mapping_request_payload(
        owner,
        &request.fixture_ids,
        &request.attribute,
        &request.features,
        fixtures,
        fixture_attributes,
    )
}

fn validate_color_mapping_request_payload(
    owner: &str,
    request: &ColorMappingEffectRequest,
    fixtures: &BTreeMap<u64, &PatchedFixtureSummary>,
    fixture_attributes: &BTreeMap<FixtureId, BTreeSet<String>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    validate_payload_fixture_existence(owner, &request.fixture_ids, fixtures)?;
    for cell in &request.cells {
        if !fixtures.contains_key(&cell.fixture_id) {
            return Err(project_missing_reference(
                format!("{owner} cell target"),
                "fixture",
                cell.fixture_id,
            ));
        }
        if let Some(feature_attribute) = &cell.feature_attribute {
            validate_fixture_attribute_reference(
                format!("{owner} cell target"),
                cell.fixture_id,
                feature_attribute,
                fixtures,
                fixture_attributes,
            )?;
        }
    }
    Ok(())
}

struct TimelineProjectReferenceInput<'input, 'snapshot> {
    layers: &'input [TimelineLayerSummary],
    events: &'input [TimelineCueEventSummary],
    automations: &'input [TimelineAutomationSummary],
    video_automations: &'input [TimelineVideoAutomationSummary],
    audio_clips: &'input [TimelineAudioClipSummary],
    video_clips: &'input [TimelineVideoClipSummary],
    item_groups: &'input [TimelineItemGroupSummary],
    cues: &'input BTreeMap<u64, &'snapshot CueSummary>,
    fixtures: &'input BTreeMap<u64, &'snapshot PatchedFixtureSummary>,
    fixture_attributes: &'input BTreeMap<FixtureId, BTreeSet<String>>,
    video: &'input ProjectVideoCatalog<'snapshot>,
}

fn validate_timeline_project_references<'input, 'snapshot>(
    owner: &str,
    input: TimelineProjectReferenceInput<'input, 'snapshot>,
) -> Result<(), ProjectReferenceIntegrityError> {
    let TimelineProjectReferenceInput {
        layers,
        events,
        automations,
        video_automations,
        audio_clips,
        video_clips,
        item_groups,
        cues,
        fixtures,
        fixture_attributes,
        video,
    } = input;
    let mut layer_by_id = BTreeMap::new();
    for layer in layers {
        // Layer 0 remains the explicit legacy lane identity in the current
        // data model. Unlike video graph IDs, it is therefore not rejected.
        if layer_by_id.insert(layer.id, layer).is_some() {
            return Err(project_duplicate_id("Timeline layer", u64::from(layer.id)));
        }
    }
    let mut event_ids = BTreeSet::new();
    for event in events {
        if event.id == 0 {
            return Err(project_zero_id("Timeline event", owner));
        }
        if !event_ids.insert(event.id) {
            return Err(project_duplicate_id("Timeline event", event.id));
        }
        if !cues.contains_key(&event.cue_id) {
            return Err(project_missing_reference(
                format!("{owner} event {}", event.id),
                "Cue",
                event.cue_id,
            ));
        }
        if let Some(layer_id) = event.layer_id {
            let layer = layer_by_id.get(&layer_id).ok_or_else(|| {
                project_missing_reference(
                    format!("{owner} event {}", event.id),
                    "Timeline layer",
                    u64::from(layer_id),
                )
            })?;
            let expected = TimelineLayerKind::from(&event.track);
            if layer.kind != expected {
                return Err(project_reference_mismatch(
                    format!("{owner} event {}", event.id),
                    format!(
                        "Timeline layer {layer_id} kind {:?} does not match {:?}",
                        layer.kind, event.track
                    ),
                ));
            }
        }
    }
    for event in events {
        if let Some(target_id) = event.jump_to_event_id {
            if !event_ids.contains(&target_id) {
                return Err(project_missing_reference(
                    format!("{owner} event {} jump", event.id),
                    "Timeline event",
                    target_id,
                ));
            }
        }
    }

    let mut automation_ids = BTreeSet::new();
    for automation in automations {
        if automation.id == 0 {
            return Err(project_zero_id("Timeline automation", owner));
        }
        if !automation_ids.insert(automation.id) {
            return Err(project_duplicate_id("Timeline automation", automation.id));
        }
        validate_fixture_attribute_reference(
            format!("{owner} lighting automation {}", automation.id),
            automation.fixture_id,
            &automation.attribute,
            fixtures,
            fixture_attributes,
        )?;
        if !matches!(automation.track, TimelineTrackKind::Lighting) {
            return Err(project_reference_mismatch(
                format!("{owner} lighting automation {}", automation.id),
                "track is not Lighting",
            ));
        }
        if let Some(layer_id) = automation.timeline_layer_id {
            let layer = layer_by_id.get(&layer_id).ok_or_else(|| {
                project_missing_reference(
                    format!("{owner} lighting automation {}", automation.id),
                    "Timeline layer",
                    u64::from(layer_id),
                )
            })?;
            if !matches!(layer.kind, TimelineLayerKind::Lighting) {
                return Err(project_reference_mismatch(
                    format!("{owner} lighting automation {}", automation.id),
                    format!("Timeline layer {layer_id} is not Lighting"),
                ));
            }
        }
    }
    for automation in video_automations {
        if automation.id == 0 {
            return Err(project_zero_id("Timeline automation", owner));
        }
        if !automation_ids.insert(automation.id) {
            return Err(project_duplicate_id("Timeline automation", automation.id));
        }
        if !video.layers.contains_key(&automation.layer_id) {
            return Err(project_missing_reference(
                format!("{owner} video automation {}", automation.id),
                "video layer",
                automation.layer_id,
            ));
        }
        if !matches!(automation.track, TimelineTrackKind::Video) {
            return Err(project_reference_mismatch(
                format!("{owner} video automation {}", automation.id),
                "track is not Video",
            ));
        }
        if let Some(layer_id) = automation.timeline_layer_id {
            let layer = layer_by_id.get(&layer_id).ok_or_else(|| {
                project_missing_reference(
                    format!("{owner} video automation {}", automation.id),
                    "Timeline layer",
                    u64::from(layer_id),
                )
            })?;
            if !matches!(layer.kind, TimelineLayerKind::Video) {
                return Err(project_reference_mismatch(
                    format!("{owner} video automation {}", automation.id),
                    format!("Timeline layer {layer_id} is not Video"),
                ));
            }
        }
    }

    let mut audio_clip_ids = BTreeSet::new();
    for clip in audio_clips {
        // Rejected with the same taxonomy as Timeline video clips; zero is no
        // more a valid audio clip identity than a video clip identity.
        if clip.id == 0 {
            return Err(project_zero_id("Timeline audio clip", owner));
        }
        if !audio_clip_ids.insert(clip.id) {
            return Err(project_duplicate_id("Timeline audio clip", clip.id));
        }
        if let Some(asset_id) = clip.media_asset_id {
            if !video.media_assets.contains_key(&asset_id) {
                return Err(project_missing_reference(
                    format!("{owner} audio clip {}", clip.id),
                    "MediaAsset",
                    asset_id,
                ));
            }
        }
        if clip.layer_id != 0 {
            let layer = layer_by_id.get(&clip.layer_id).ok_or_else(|| {
                project_missing_reference(
                    format!("{owner} audio clip {}", clip.id),
                    "Timeline layer",
                    u64::from(clip.layer_id),
                )
            })?;
            if !matches!(layer.kind, TimelineLayerKind::Audio) {
                return Err(project_reference_mismatch(
                    format!("{owner} audio clip {}", clip.id),
                    format!("Timeline layer {} is not Audio", clip.layer_id),
                ));
            }
        }
    }

    let mut video_clip_ids = BTreeSet::new();
    for clip in video_clips {
        if clip.id.0 == 0 {
            return Err(project_zero_id("Timeline video clip", owner));
        }
        if !video_clip_ids.insert(clip.id) {
            return Err(project_duplicate_id("Timeline video clip", clip.id.0));
        }
        let layer = layer_by_id.get(&clip.layer_id).ok_or_else(|| {
            project_missing_reference(
                format!("{owner} video clip {}", clip.id.0),
                "Timeline layer",
                u64::from(clip.layer_id),
            )
        })?;
        if !matches!(layer.kind, TimelineLayerKind::Video) {
            return Err(project_reference_mismatch(
                format!("{owner} video clip {}", clip.id.0),
                format!("Timeline layer {} is not Video", clip.layer_id),
            ));
        }
        if !video.media_assets.contains_key(&clip.media_asset_id) {
            return Err(project_missing_reference(
                format!("{owner} video clip {}", clip.id.0),
                "MediaAsset",
                clip.media_asset_id,
            ));
        }
    }

    let mut grouped_items = BTreeSet::new();
    let mut item_group_ids = BTreeSet::new();
    for group in item_groups {
        if group.id.0 == 0 {
            return Err(project_zero_id("Timeline item group", owner));
        }
        if !item_group_ids.insert(group.id) {
            return Err(project_duplicate_id("Timeline item group", group.id.0));
        }
        for member in &group.members {
            if !grouped_items.insert(*member) {
                return Err(project_reference_mismatch(
                    format!("{owner} item group {}", group.id.0),
                    "an item belongs to more than one group or is duplicated",
                ));
            }
            let exists = match member {
                TimelineItemRef::LightingEvent { event_id } => event_ids.contains(event_id),
                TimelineItemRef::VideoClip { clip_id } => video_clip_ids.contains(clip_id),
                TimelineItemRef::AudioClip { clip_id } => {
                    *clip_id != 0 && audio_clip_ids.contains(clip_id)
                }
                TimelineItemRef::LightingAutomation { automation_id } => automations
                    .iter()
                    .any(|automation| automation.id == *automation_id),
                TimelineItemRef::VideoAutomation { automation_id } => video_automations
                    .iter()
                    .any(|automation| automation.id == *automation_id),
            };
            if !exists {
                return Err(project_reference_mismatch(
                    format!("{owner} item group {}", group.id.0),
                    format!("contains missing item {member:?}"),
                ));
            }
        }
    }
    Ok(())
}

fn validate_child_timeline_reference_cycles(
    cues: &[CueSummary],
) -> Result<(), ProjectReferenceIntegrityError> {
    let adjacency = cues
        .iter()
        .map(|cue| {
            let children = cue
                .child_timeline
                .as_ref()
                .map(|child| child.events.iter().map(|event| event.cue_id).collect())
                .unwrap_or_default();
            (cue.id, children)
        })
        .collect::<BTreeMap<CueId, Vec<CueId>>>();
    validate_u64_reference_cycles("Cue child Timeline", &adjacency)
}

fn validate_timeline_follow_cycles(
    timelines: &[TimelineSnapshot],
) -> Result<(), ProjectReferenceIntegrityError> {
    let adjacency = timelines
        .iter()
        .map(|timeline| {
            (
                timeline.id.0,
                timeline
                    .follow
                    .as_ref()
                    .map(|follow| vec![follow.next_timeline_id.0])
                    .unwrap_or_default(),
            )
        })
        .collect::<BTreeMap<u64, Vec<u64>>>();
    validate_u64_reference_cycles("Timeline Follow", &adjacency)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CycleVisitState {
    Unvisited,
    InProgress,
    Done,
}

/// Iterative, bounded, deterministic cycle detection.
///
/// Traversal order is fixed by the `BTreeMap` key order plus each node's
/// authored child list, so the reported cycle path is a pure function of the
/// input. The explicit work stack replaces recursion: memory grows with the
/// graph size (each node is visited at most once per state), never with
/// unbounded call depth, so a hostile deep chain returns a typed failure or
/// completes instead of aborting the process on stack exhaustion.
fn validate_u64_reference_cycles(
    domain: &'static str,
    adjacency: &BTreeMap<u64, Vec<u64>>,
) -> Result<(), ProjectReferenceIntegrityError> {
    let mut state: BTreeMap<u64, CycleVisitState> = BTreeMap::new();
    // Path of nodes currently being explored, root-first. Its top always
    // matches the node of the most recently discovered unfinished frame.
    let mut path: Vec<u64> = Vec::new();
    // Explicit DFS frames: (node, index of the next child to visit).
    let mut work: Vec<(u64, usize)> = Vec::new();

    for root in adjacency.keys() {
        if state
            .get(root)
            .copied()
            .unwrap_or(CycleVisitState::Unvisited)
            != CycleVisitState::Unvisited
        {
            continue;
        }
        state.insert(*root, CycleVisitState::InProgress);
        path.push(*root);
        work.push((*root, 0));
        while let Some((node, child_index)) = work.pop() {
            let children = adjacency.get(&node).map(Vec::as_slice).unwrap_or(&[]);
            if child_index >= children.len() {
                state.insert(node, CycleVisitState::Done);
                path.pop();
                continue;
            }
            // Revisit this frame after the child subtree finishes; the path
            // top is still `node` at that point because finished children
            // restore it exactly.
            work.push((node, child_index + 1));
            let child = &children[child_index];
            match state
                .get(child)
                .copied()
                .unwrap_or(CycleVisitState::Unvisited)
            {
                CycleVisitState::Done => {}
                CycleVisitState::InProgress => {
                    let start = path.iter().position(|candidate| candidate == child);
                    let start = start.unwrap_or(0);
                    let mut cycle_path = path[start..].to_vec();
                    cycle_path.push(*child);
                    return Err(ProjectReferenceIntegrityError::ReferenceCycle {
                        domain,
                        path: cycle_path,
                    });
                }
                CycleVisitState::Unvisited => {
                    state.insert(*child, CycleVisitState::InProgress);
                    path.push(*child);
                    work.push((*child, 0));
                }
            }
        }
    }
    Ok(())
}

impl Default for EngineSnapshot {
    fn default() -> Self {
        Self {
            fixtures: Vec::new(),
            cues: Vec::new(),
            cue_lists: default_cue_lists(),
            palettes: Vec::new(),
            playback_executors: default_playback_executors(),
            playback_master: 1.0,
            active_cue_id: None,
            direct_child_timeline_transports: Vec::new(),
            cue_live_modifiers: Vec::new(),
            active_group_cue_ids: BTreeMap::new(),
            group_colors: BTreeMap::new(),
            active_fade: None,
            programmer: ProgrammerSnapshot::default(),
            timeline: TimelineSnapshot::default(),
            timeline_bank: Vec::new(),
            video: VideoSnapshot::default(),
            video_clip_runtime: VideoClipRuntimeSnapshot::default(),
            video_transition_runtime: VideoLayerTransitionRuntimeSnapshot::default(),
            authored_video: None,
            effects: Vec::new(),
            node_graphs: Vec::new(),
            output: DmxOutputConfig::default(),
            dmx_outputs: vec![DmxOutputConfig::default()],
            lighting_master: 1.0,
            submasters: Vec::new(),
            blackout: false,
            clock: ClockSnapshot::default(),
            stage_map: StageMapConfig::default(),
            stage_map_presets: Vec::new(),
            stage_objects: Vec::new(),
            touch_surface: TouchSurfaceSummary::default(),
            dmx_preview: vec![0; 512],
            dmx_previews: vec![DmxUniversePreview {
                universe: 0,
                values: vec![0; 512],
            }],
            telemetry: EngineTelemetry::default(),
        }
    }
}

fn default_cue_lists() -> Vec<CueListSummary> {
    vec![CueListSummary::default()]
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_video_output_mapping_field, set_video_output_mapping_field_value,
        validate_timeline_tempo_meter_map, video_output_mapping_field_value, AudioSpectrumBand,
        AudioSpectrumSource, ChildTimelineSummary, CueListSummary, DmxControlMapping,
        MidiControlMapping, NodeGraphAudioNode, OscControlMapping, PlaybackExecutorSummary,
        TimelineSnapshot, TimelineTempoInterpolation, TimelineTempoMeterPoint, VideoOutputMapping,
        DEFAULT_CUE_LIST_ID, TIMELINE_TEMPO_METER_MAP_VERSION,
    };

    #[test]
    fn cue_list_and_playback_executor_defaults_are_ordinary_bank_one_labels() {
        assert_eq!(CueListSummary::default().label, "Bank 1");
        assert_eq!(PlaybackExecutorSummary::default().label, "Bank 1");
        assert_eq!(CueListSummary::default().id, DEFAULT_CUE_LIST_ID);
        assert_eq!(
            PlaybackExecutorSummary::default().cue_list_id,
            DEFAULT_CUE_LIST_ID
        );
    }

    #[test]
    fn video_output_mapping_fields_are_canonicalized() {
        assert_eq!(
            canonical_video_output_mapping_field("  Keystone X  "),
            Some("keystone_x")
        );
        assert_eq!(
            canonical_video_output_mapping_field("horizontal keystone"),
            Some("keystone_x")
        );
        assert_eq!(
            canonical_video_output_mapping_field("vertical perspective"),
            Some("keystone_y")
        );
        assert_eq!(
            canonical_video_output_mapping_field("projector ratio"),
            Some("aspect_ratio")
        );
        assert_eq!(
            canonical_video_output_mapping_field("barrel distortion"),
            Some("lens_distortion")
        );
        assert_eq!(
            canonical_video_output_mapping_field("left feather"),
            Some("edge_blend_left")
        );
        assert_eq!(
            canonical_video_output_mapping_field("black lift"),
            Some("black_level")
        );
        assert_eq!(
            canonical_video_output_mapping_field("rotate"),
            Some("rotation_deg")
        );
        assert_eq!(
            canonical_video_output_mapping_field("top left x"),
            Some("corner_top_left_x")
        );
        assert_eq!(
            canonical_video_output_mapping_field("corner_top_left_y"),
            Some("corner_top_left_y")
        );
        assert_eq!(
            canonical_video_output_mapping_field("stage location z"),
            Some("stage_z")
        );
        assert_eq!(
            canonical_video_output_mapping_field("position x"),
            Some("offset_x")
        );
        assert_eq!(canonical_video_output_mapping_field("unknown"), None);
    }

    #[test]
    fn video_output_mapping_field_helpers_read_and_write_values() {
        let mut mapping = VideoOutputMapping::default();
        set_video_output_mapping_field_value(&mut mapping, "key y", -0.25).unwrap();
        set_video_output_mapping_field_value(&mut mapping, "stage_position_x", 3.5).unwrap();
        set_video_output_mapping_field_value(&mut mapping, "screen x", 0.125).unwrap();
        set_video_output_mapping_field_value(&mut mapping, "projector ratio", 16.0 / 9.0).unwrap();
        set_video_output_mapping_field_value(&mut mapping, "barrel", -0.1).unwrap();
        set_video_output_mapping_field_value(&mut mapping, "blend right", 0.25).unwrap();
        set_video_output_mapping_field_value(&mut mapping, "blend gamma", 1.8).unwrap();

        assert_eq!(
            video_output_mapping_field_value(&mapping, "keystone_y"),
            Some(-0.25)
        );
        assert_eq!(video_output_mapping_field_value(&mapping, "sx"), Some(3.5));
        assert_eq!(
            video_output_mapping_field_value(&mapping, "offset_x"),
            Some(0.125)
        );
        assert_eq!(
            video_output_mapping_field_value(&mapping, "aspect_ratio"),
            Some(16.0 / 9.0)
        );
        assert_eq!(
            video_output_mapping_field_value(&mapping, "lens distortion"),
            Some(-0.1)
        );
        assert_eq!(
            video_output_mapping_field_value(&mapping, "edge_blend_right"),
            Some(0.25)
        );
        assert_eq!(
            video_output_mapping_field_value(&mapping, "edge_blend_gamma"),
            Some(1.8)
        );
        assert!(set_video_output_mapping_field_value(&mut mapping, "not a field", 1.0).is_err());
    }

    #[test]
    fn legacy_audio_node_defaults_to_timeline_source() {
        let node: NodeGraphAudioNode =
            serde_json::from_str(r#"{"band":"Bass","gain":1.0,"bias":0.0}"#).unwrap();
        assert_eq!(node.source, AudioSpectrumSource::Timeline);
        assert_eq!(node.feature, super::AudioReactiveFeature::LegacyBand);
        assert_eq!(node.attack_ms, 0);
        assert_eq!(node.release_ms, 0);
        let live = NodeGraphAudioNode {
            source: AudioSpectrumSource::Live,
            band: AudioSpectrumBand::High,
            gain: 2.0,
            bias: -0.1,
            feature: super::AudioReactiveFeature::Band,
            band_index: 15,
            attack_ms: 25,
            release_ms: 180,
            gate: 0.08,
            curve: super::AudioReactiveCurve::Smoothstep,
            invert: true,
            hold_ms: 120,
        };
        let roundtrip: NodeGraphAudioNode =
            serde_json::from_str(&serde_json::to_string(&live).unwrap()).unwrap();
        assert_eq!(roundtrip, live);
    }

    #[test]
    fn legacy_cue_defaults_to_inherited_ifcb_timing() {
        let cue = super::CueSummary::default();
        let mut value = serde_json::to_value(cue).unwrap();
        value.as_object_mut().unwrap().remove("ifcb_timing");
        value.as_object_mut().unwrap().remove("cue_list_id");
        value.as_object_mut().unwrap().remove("authored_beats");
        value.as_object_mut().unwrap().remove("parts");
        value.as_object_mut().unwrap().remove("mark");
        value.as_object_mut().unwrap().remove("mib_fixture_ids");
        value.as_object_mut().unwrap().remove("palette_targets");
        value.as_object_mut().unwrap().remove("effect_targets");
        value.as_object_mut().unwrap().remove("group_id");
        value.as_object_mut().unwrap().remove("recall_mode");

        let parsed: super::CueSummary = serde_json::from_value(value).unwrap();

        assert_eq!(parsed.ifcb_timing, super::CueIfcbTiming::default());
        assert_eq!(parsed.ifcb_timing.intensity_fade_ms, None);
        assert_eq!(parsed.ifcb_timing.focus_delay_ms, 0);
        assert_eq!(parsed.cue_list_id, super::DEFAULT_CUE_LIST_ID);
        assert_eq!(parsed.authored_beats, None);
        assert!(parsed.parts.is_empty());
        assert!(!parsed.mark);
        assert!(parsed.mib_fixture_ids.is_empty());
        assert!(parsed.palette_targets.is_empty());
        assert!(parsed.effect_targets.is_empty());
        assert!(parsed.steps.is_empty());
        assert_eq!(parsed.group_id, None);
        assert_eq!(parsed.recall_mode, super::RecallMode::Coexist);
    }

    #[test]
    fn cue_steps_roundtrip_and_empty_steps_are_omitted() {
        let legacy = super::CueSummary::default();
        let legacy_json = serde_json::to_value(&legacy).unwrap();
        assert!(legacy_json.get("steps").is_none());
        let decoded_legacy: super::CueSummary = serde_json::from_value(legacy_json).unwrap();
        assert!(decoded_legacy.steps.is_empty());

        let cue = super::CueSummary {
            id: 21,
            steps: vec![
                super::CueStepSummary {
                    values: vec![super::CueFixtureTarget {
                        fixture_id: 7,
                        values: vec![super::AttributeValueSummary {
                            attribute: "Dimmer".to_string(),
                            value: 12_345,
                        }],
                    }],
                    fade_ms: 250,
                    hold_ms: 750,
                },
                super::CueStepSummary {
                    values: vec![super::CueFixtureTarget {
                        fixture_id: 7,
                        values: vec![super::AttributeValueSummary {
                            attribute: "Dimmer".to_string(),
                            value: 54_321,
                        }],
                    }],
                    fade_ms: 500,
                    hold_ms: 500,
                },
            ],
            ..Default::default()
        };

        let encoded = serde_json::to_vec(&cue).unwrap();
        let decoded: super::CueSummary = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, cue);
    }

    #[test]
    fn cue_scene_matrix_fields_roundtrip_and_default_for_legacy_json() {
        let cue = super::CueSummary {
            id: 18,
            group_id: Some("Front/Wash".to_string()),
            recall_mode: super::RecallMode::ReplaceGroup,
            ..Default::default()
        };

        let encoded = serde_json::to_value(&cue).unwrap();
        assert_eq!(encoded["group_id"], "Front/Wash");
        assert_eq!(encoded["recall_mode"], "ReplaceGroup");
        let decoded: super::CueSummary = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(decoded, cue);

        let mut legacy = encoded;
        legacy.as_object_mut().unwrap().remove("group_id");
        legacy.as_object_mut().unwrap().remove("recall_mode");
        let legacy: super::CueSummary = serde_json::from_value(legacy).unwrap();
        assert_eq!(legacy.group_id, None);
        assert_eq!(legacy.recall_mode, super::RecallMode::Coexist);
    }

    #[test]
    fn cue_authored_beats_roundtrip() {
        let cue = super::CueSummary {
            id: 17,
            authored_beats: Some(4.0),
            ..Default::default()
        };

        let encoded = serde_json::to_string(&cue).unwrap();
        let decoded: super::CueSummary = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.authored_beats, Some(4.0));
        assert_eq!(decoded, cue);
    }

    #[test]
    fn legacy_timeline_cue_event_defaults_to_point_placement() {
        let event: super::TimelineCueEventSummary = serde_json::from_value(serde_json::json!({
            "id": 9,
            "cue_id": 3,
            "time_ms": 1_250,
            "track": "Lighting"
        }))
        .unwrap();

        assert_eq!(event.layer_id, None);
        assert_eq!(event.time_beats, None);
        assert_eq!(event.duration_ms, 0);
        assert_eq!(event.duration_beats, None);
        assert!(!event.conform_to_tempo);
        assert!(!event.loop_fill);
        assert_eq!(event.source_offset_ms, 0);
        assert_eq!(event.rate, None);
        assert_eq!(event.fade_in_ms, 0);
        assert_eq!(event.fade_out_ms, 0);
        assert_eq!(event.loop_count, 1);
        assert_eq!(event.jump_to_event_id, None);
    }

    #[test]
    fn timeline_layer_kind_defaults_derives_from_track_and_has_fixed_section_rank() {
        assert_eq!(
            super::TimelineLayerKind::default(),
            super::TimelineLayerKind::Lighting
        );
        assert_eq!(
            super::TimelineLayerKind::from(&super::TimelineTrackKind::Lighting),
            super::TimelineLayerKind::Lighting
        );
        assert_eq!(
            super::TimelineLayerKind::from(&super::TimelineTrackKind::Video),
            super::TimelineLayerKind::Video
        );
        assert_eq!(super::TimelineLayerKind::Audio.display_section_rank(), 0);
        assert_eq!(super::TimelineLayerKind::Lighting.display_section_rank(), 1);
        assert_eq!(super::TimelineLayerKind::Video.display_section_rank(), 2);
    }

    #[test]
    fn timeline_layer_summary_defaults_flags_and_kind() {
        let layer: super::TimelineLayerSummary = serde_json::from_value(serde_json::json!({
            "id": 7,
            "label": "Front Wash",
            "order": 3
        }))
        .unwrap();

        assert!(!layer.muted);
        assert!(!layer.locked);
        assert!(!layer.solo);
        assert!(!layer.expanded);
        assert_eq!(layer.kind, super::TimelineLayerKind::Lighting);
    }

    #[test]
    fn timeline_automation_lane_identity_is_additive_and_kind_checked() {
        let legacy_lighting: super::TimelineAutomationSummary =
            serde_json::from_value(serde_json::json!({
                "id": 31,
                "fixture_id": 7,
                "attribute": "Dimmer",
                "track": "Lighting",
                "keyframes": [],
                "enabled": true
            }))
            .unwrap();
        let legacy_video: super::TimelineVideoAutomationSummary =
            serde_json::from_value(serde_json::json!({
                "id": 32,
                "layer_id": 91,
                "param": "Opacity",
                "track": "Video",
                "keyframes": [],
                "enabled": true
            }))
            .unwrap();
        assert_eq!(legacy_lighting.timeline_layer_id, None);
        assert_eq!(legacy_video.timeline_layer_id, None);
        assert!(serde_json::to_value(&legacy_lighting)
            .unwrap()
            .get("timeline_layer_id")
            .is_none());

        let mut timeline = super::TimelineSnapshot {
            layers: vec![
                super::TimelineLayerSummary {
                    id: 11,
                    label: "Lighting A".to_string(),
                    order: 0,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: true,
                    kind: super::TimelineLayerKind::Lighting,
                },
                super::TimelineLayerSummary {
                    id: 12,
                    label: "Video A".to_string(),
                    order: 1,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: true,
                    kind: super::TimelineLayerKind::Video,
                },
            ],
            automations: vec![super::TimelineAutomationSummary {
                timeline_layer_id: Some(11),
                ..legacy_lighting
            }],
            video_automations: vec![super::TimelineVideoAutomationSummary {
                timeline_layer_id: Some(12),
                ..legacy_video
            }],
            ..super::TimelineSnapshot::default()
        };
        super::validate_timeline_authoring(&timeline, &[]).unwrap();
        let roundtrip: super::TimelineSnapshot =
            serde_json::from_value(serde_json::to_value(&timeline).unwrap()).unwrap();
        assert_eq!(roundtrip, timeline);
        assert_eq!(roundtrip.video_automations[0].layer_id, 91);
        assert_eq!(roundtrip.video_automations[0].timeline_layer_id, Some(12));

        timeline.automations[0].timeline_layer_id = Some(12);
        assert!(super::validate_timeline_authoring(&timeline, &[])
            .unwrap_err()
            .contains("non-Lighting Timeline layer"));
        timeline.automations[0].timeline_layer_id = Some(11);
        timeline.video_automations[0].timeline_layer_id = Some(99);
        assert!(super::validate_timeline_authoring(&timeline, &[])
            .unwrap_err()
            .contains("missing Timeline layer 99"));
    }

    #[test]
    fn legacy_timeline_snapshot_defaults_layers_to_empty() {
        let snapshot: super::TimelineSnapshot = serde_json::from_value(serde_json::json!({
            "events": [],
            "automations": [],
            "video_automations": [],
            "playing": false,
            "position_ms": 0,
            "duration_ms": 0
        }))
        .unwrap();

        assert!(snapshot.layers.is_empty());
        assert!(snapshot.audio_clips.is_empty());
        assert_eq!(snapshot.id, super::TimelineId(1));
        assert_eq!(snapshot.label, "Timeline 1");
        assert!(snapshot.video_clips.is_empty());
        assert!(snapshot.phases.is_empty());
        assert!(snapshot.item_groups.is_empty());
        assert!(snapshot.loop_region.is_none());
        assert!(snapshot.follow.is_none());
        assert!(!snapshot.guide_enabled);
        assert_eq!(snapshot.audio_offset_ms, 0);
        assert!(!snapshot.audio_muted);
        assert!(!snapshot.metronome_enabled);
        assert_eq!(snapshot.count_in_beats, 4);
        assert_eq!(snapshot.count_in_remaining_ms, 0);
        assert_eq!(snapshot.audio_transport_revision, 0);
        assert!(snapshot.active_child_transports.is_empty());
    }

    #[test]
    fn timeline_phase_media_group_loop_and_follow_contract_is_strict_and_roundtrips() {
        let media_assets = vec![super::MediaAssetSummary {
            id: 51,
            label: "Song visual".to_string(),
            source: super::VideoSourceSummary {
                kind: super::VideoSourceKind::File,
                path: Some("C:/show/song.mp4".to_string()),
                name: None,
                codec: Some("H264".to_string()),
                metadata: Some(super::VideoMediaMetadata {
                    duration_ms: Some(16_000),
                    width: Some(1920),
                    height: Some(1080),
                    frame_rate: Some(30.0),
                    has_audio: true,
                }),
            },
            content_hash: None,
            byte_size: None,
        }];
        let mut timeline = super::TimelineSnapshot {
            id: super::TimelineId(7),
            label: "Opening song".to_string(),
            layers: vec![
                super::TimelineLayerSummary {
                    id: 1,
                    label: "Video".to_string(),
                    order: 0,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: true,
                    kind: super::TimelineLayerKind::Video,
                },
                super::TimelineLayerSummary {
                    id: 2,
                    label: "Audio".to_string(),
                    order: 1,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: true,
                    kind: super::TimelineLayerKind::Audio,
                },
            ],
            video_clips: vec![super::TimelineVideoClipSummary {
                id: super::TimelineVideoClipId(61),
                layer_id: 1,
                media_asset_id: 51,
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 16_000,
                fade_in_ms: 250,
                fade_out_ms: 250,
            }],
            audio_clips: vec![super::TimelineAudioClipSummary {
                id: 62,
                layer_id: 2,
                media_asset_id: Some(51),
                path: String::new(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 16_000,
                gain: 1.0,
                fade_in_ms: 250,
                fade_out_ms: 250,
                output_bus: super::TimelineAudioOutputBus::Program,
            }],
            phases: vec![
                super::TimelinePhaseSummary {
                    id: super::TimelinePhaseId(71),
                    label: "Intro".to_string(),
                    role: super::TimelinePhaseRole::Intro,
                    start_ms: 0,
                    end_ms: 8_000,
                },
                super::TimelinePhaseSummary {
                    id: super::TimelinePhaseId(72),
                    label: "Verse".to_string(),
                    role: super::TimelinePhaseRole::Verse,
                    start_ms: 8_000,
                    end_ms: 16_000,
                },
            ],
            item_groups: vec![super::TimelineItemGroupSummary {
                id: super::TimelineItemGroupId(81),
                members: vec![
                    super::TimelineItemRef::VideoClip {
                        clip_id: super::TimelineVideoClipId(61),
                    },
                    super::TimelineItemRef::AudioClip { clip_id: 62 },
                ],
            }],
            loop_region: Some(super::TimelineLoopRegionSummary {
                a_ms: 4_000,
                b_ms: 8_000,
                enabled: true,
                musical_length_beats: Some(8.0),
            }),
            follow: Some(super::TimelineFollowSummary {
                enabled: true,
                next_timeline_id: super::TimelineId(8),
                duration: super::VideoClipTakeDuration {
                    unit: super::VideoClipTakeDurationUnit::Bars,
                    value_milliunits: 1_000,
                },
                curve: super::VideoLayerTransitionCurve::EaseInOut,
                video_kind: super::VideoClipTakeKind::Crossfade,
                lighting_policy: super::TimelineFollowLightingPolicy::LinearMerge,
                destination_bpm: Some(128.0),
                preroll_ms: 500,
                trans_cadence_bars: 4,
                trans_target_measures: Vec::new(),
                hold_first_destination_measure: false,
                fault_policy: super::TimelineFollowFaultPolicy::Hold,
            }),
            guide_enabled: true,
            duration_ms: 16_000,
            ..super::TimelineSnapshot::default()
        };
        super::validate_timeline_authoring(&timeline, &media_assets).unwrap();
        let roundtrip: super::TimelineSnapshot =
            serde_json::from_str(&serde_json::to_string(&timeline).unwrap()).unwrap();
        assert_eq!(roundtrip, timeline);

        timeline.phases[1].start_ms = 7_999;
        assert!(super::validate_timeline_authoring(&timeline, &media_assets)
            .unwrap_err()
            .contains("non-overlapping"));
        timeline.phases[1].start_ms = 8_000;
        timeline.item_groups[0]
            .members
            .push(super::TimelineItemRef::VideoClip {
                clip_id: super::TimelineVideoClipId(61),
            });
        assert!(super::validate_timeline_authoring(&timeline, &media_assets)
            .unwrap_err()
            .contains("at most one group"));
    }

    #[test]
    fn timeline_bank_normalizes_legacy_runtime_free_and_requires_exact_next_follow_target() {
        let mut snapshot = super::EngineSnapshot::default();
        snapshot.timeline.id = super::TimelineId(41);
        snapshot.timeline.label = "Intro".to_string();
        snapshot.timeline.playing = true;
        snapshot.timeline.position_ms = 750;
        snapshot.timeline.loop_runtime.status = super::TimelineLoopRuntimeStatus::Looping;
        snapshot
            .timeline
            .guide_cues
            .push(super::TimelineGuideCueSummary {
                generation: 1,
                sequence: 1,
                at_ms: 750,
                label: "Looping".to_string(),
                cue: super::TimelineGuideCueKind::Looping,
                asset: super::TimelineGuideAssetKey::Looping,
                playback_rate_milli: 1_000,
                sample_frame: 36_000,
                epoch: 1,
                transport_generation: 1,
                schedule_generation: 1,
                source: super::TimelineScheduleSource::Root,
            });

        super::normalize_timeline_bank(&mut snapshot);
        assert_eq!(snapshot.timeline_bank.len(), 1);
        assert_eq!(snapshot.timeline_bank[0].id, super::TimelineId(41));
        assert!(!snapshot.timeline_bank[0].playing);
        assert_eq!(snapshot.timeline_bank[0].position_ms, 0);
        assert!(snapshot.timeline_bank[0].guide_cues.is_empty());

        let second = super::TimelineSnapshot {
            id: super::TimelineId(42),
            label: "Verse".to_string(),
            ..Default::default()
        };
        snapshot.timeline_bank.push(second);
        snapshot.timeline_bank[0].follow = Some(super::TimelineFollowSummary {
            enabled: true,
            next_timeline_id: super::TimelineId(42),
            duration: super::VideoClipTakeDuration {
                unit: super::VideoClipTakeDurationUnit::Bars,
                value_milliunits: 1_000,
            },
            curve: super::VideoLayerTransitionCurve::Linear,
            video_kind: super::VideoClipTakeKind::Crossfade,
            lighting_policy: super::TimelineFollowLightingPolicy::LinearMerge,
            destination_bpm: Some(128.0),
            preroll_ms: 0,
            trans_cadence_bars: 4,
            trans_target_measures: Vec::new(),
            hold_first_destination_measure: false,
            fault_policy: super::TimelineFollowFaultPolicy::Hold,
        });
        super::validate_timeline_bank(&snapshot, &[]).unwrap();

        snapshot.timeline_bank[0]
            .follow
            .as_mut()
            .unwrap()
            .next_timeline_id = super::TimelineId(99);
        assert!(super::validate_timeline_bank(&snapshot, &[])
            .unwrap_err()
            .contains("next Timeline"));

        snapshot.timeline_bank[0]
            .follow
            .as_mut()
            .unwrap()
            .next_timeline_id = super::TimelineId(42);
        snapshot
            .timeline_bank
            .push(snapshot.timeline_bank[1].clone());
        assert!(super::validate_timeline_bank(&snapshot, &[])
            .unwrap_err()
            .contains("unique"));
    }

    #[test]
    fn timeline_follow_ltl5_runtime_contract_is_omitted_from_authored_json_and_epoch_fenced() {
        let timeline = super::TimelineSnapshot {
            id: super::TimelineId(91),
            label: "Runtime-free Follow".to_string(),
            follow_runtime: super::TimelineFollowRuntimeSummary {
                generation: 17,
                status: super::TimelineFollowRuntimeStatus::Aborting,
                admission_reason: Some(
                    super::TimelineFollowAdmissionReason::PrerollBeforeNaturalPlaybackBoundary,
                ),
                outcome: Some(super::TimelineFollowOutcome::Aborted {
                    reason: super::TimelineFollowAbortReason::ManualSeek,
                }),
                source_timeline_id: Some(super::TimelineId(91)),
                target_timeline_id: Some(super::TimelineId(92)),
                elapsed_ms: 250,
                duration_ms: 1_000,
                progress_millis: 250,
                fault: None,
                transition_hold_active: false,
                settlement: Some(super::TimelineFollowSettlementSummary {
                    started_at_ms: 9_000,
                    deadline_ms: 11_000,
                    state: super::TimelineFollowSettlementState::Pending,
                    progress_millis: 333,
                    fault_policy: super::TimelineFollowFaultPolicy::Hold,
                    fault: None,
                    domains: vec![
                        super::TimelineFollowSettlementDomainSummary {
                            domain: super::TimelineFollowSettlementDomain::Audio,
                            state: super::TimelineFollowSettlementState::Applied,
                            fault: None,
                            consumers: vec![super::TimelineFollowSettlementConsumerSummary {
                                consumer_id: super::TimelineFollowSettlementConsumerId::Audio,
                                state: super::TimelineFollowSettlementState::Applied,
                                fault: None,
                            }],
                        },
                        super::TimelineFollowSettlementDomainSummary {
                            domain: super::TimelineFollowSettlementDomain::Video,
                            state: super::TimelineFollowSettlementState::Pending,
                            fault: None,
                            consumers: vec![super::TimelineFollowSettlementConsumerSummary {
                                consumer_id:
                                    super::TimelineFollowSettlementConsumerId::VideoOutput {
                                        output_id: 41,
                                    },
                                state: super::TimelineFollowSettlementState::Pending,
                                fault: None,
                            }],
                        },
                        super::TimelineFollowSettlementDomainSummary {
                            domain: super::TimelineFollowSettlementDomain::Lighting,
                            state: super::TimelineFollowSettlementState::NotApplicable,
                            fault: None,
                            consumers: Vec::new(),
                        },
                    ],
                }),
            },
            ..Default::default()
        };

        let authored_json = serde_json::to_value(&timeline).unwrap();
        assert!(authored_json.get("follow_runtime").is_none());
        let loaded: super::TimelineSnapshot = serde_json::from_value(authored_json).unwrap();
        assert_eq!(
            loaded.follow_runtime,
            super::TimelineFollowRuntimeSummary::default()
        );

        let runtime_json = serde_json::json!({
            "generation": 99,
            "status": "fault",
            "source_timeline_id": 91,
            "target_timeline_id": 92,
            "elapsed_ms": 1_000,
            "duration_ms": 1_000,
            "progress_millis": 1_000,
            "fault": "injected authored runtime must be ignored"
        });
        let mut injected = serde_json::to_value(&timeline).unwrap();
        injected
            .as_object_mut()
            .unwrap()
            .insert("follow_runtime".to_string(), runtime_json);
        let loaded: super::TimelineSnapshot = serde_json::from_value(injected).unwrap();
        assert_eq!(
            loaded.follow_runtime,
            super::TimelineFollowRuntimeSummary::default()
        );

        let public =
            super::TimelineFollowRuntimeStatusSnapshot::from_runtime(44, &timeline.follow_runtime);
        let public_json = serde_json::to_value(&public).unwrap();
        assert_eq!(public_json["epoch"], 44);
        assert_eq!(public_json["status"], "aborting");
        assert_eq!(
            public_json["admission_reason"],
            "preroll_before_natural_playback_boundary"
        );
        assert_eq!(public_json["outcome"]["kind"], "aborted");
        assert_eq!(public_json["outcome"]["reason"], "manual_seek");
        assert_eq!(public_json["settlement"]["deadline_ms"], 11_000);
        assert_eq!(public_json["settlement"]["domains"][1]["domain"], "video");
        assert_eq!(
            public_json["settlement"]["domains"][1]["consumers"][0]["consumer_id"]["kind"],
            "video_output"
        );
        assert_eq!(
            serde_json::from_value::<super::TimelineFollowRuntimeStatusSnapshot>(public_json)
                .unwrap(),
            public
        );
    }

    #[test]
    fn timeline_follow_ltl5_settlement_roundtrips_defaults_and_generation_fenced_ack() {
        let settlement = super::TimelineFollowSettlementSummary {
            started_at_ms: 4_000,
            deadline_ms: 4_000 + super::TIMELINE_FOLLOW_SETTLEMENT_TIMEOUT_MS,
            state: super::TimelineFollowSettlementState::Pending,
            progress_millis: 333,
            fault_policy: super::TimelineFollowFaultPolicy::Cut,
            fault: None,
            domains: vec![
                super::TimelineFollowSettlementDomainSummary {
                    domain: super::TimelineFollowSettlementDomain::Audio,
                    state: super::TimelineFollowSettlementState::Applied,
                    fault: None,
                    consumers: vec![super::TimelineFollowSettlementConsumerSummary {
                        consumer_id: super::TimelineFollowSettlementConsumerId::Audio,
                        state: super::TimelineFollowSettlementState::Applied,
                        fault: None,
                    }],
                },
                super::TimelineFollowSettlementDomainSummary {
                    domain: super::TimelineFollowSettlementDomain::Video,
                    state: super::TimelineFollowSettlementState::Pending,
                    fault: None,
                    consumers: vec![
                        super::TimelineFollowSettlementConsumerSummary {
                            consumer_id: super::TimelineFollowSettlementConsumerId::VideoOutput {
                                output_id: 7,
                            },
                            state: super::TimelineFollowSettlementState::Applied,
                            fault: None,
                        },
                        super::TimelineFollowSettlementConsumerSummary {
                            consumer_id: super::TimelineFollowSettlementConsumerId::VideoOutput {
                                output_id: 9,
                            },
                            state: super::TimelineFollowSettlementState::Pending,
                            fault: None,
                        },
                    ],
                },
                super::TimelineFollowSettlementDomainSummary {
                    domain: super::TimelineFollowSettlementDomain::Lighting,
                    state: super::TimelineFollowSettlementState::NotApplicable,
                    fault: None,
                    consumers: Vec::new(),
                },
            ],
        };
        super::validate_timeline_follow_settlement_summary(&settlement).unwrap();

        let encoded = serde_json::to_value(&settlement).unwrap();
        assert_eq!(encoded["fault_policy"], "cut");
        assert_eq!(encoded["domains"][2]["state"], "not_applicable");
        assert_eq!(
            serde_json::from_value::<super::TimelineFollowSettlementSummary>(encoded).unwrap(),
            settlement
        );

        let ack = super::TimelineFollowSettlementAck {
            epoch: 12,
            generation: 34,
            domain: super::TimelineFollowSettlementDomain::Video,
            consumer_id: super::TimelineFollowSettlementConsumerId::VideoOutput { output_id: 9 },
            result: super::TimelineFollowSettlementAckResult::Fault {
                fault: "present failed".to_string(),
            },
        };
        super::validate_timeline_follow_settlement_ack(&ack).unwrap();
        let ack_json = serde_json::to_value(&ack).unwrap();
        assert_eq!(ack_json["domain"], "video");
        assert_eq!(ack_json["consumer_id"]["kind"], "video_output");
        assert_eq!(ack_json["result"]["kind"], "fault");
        assert_eq!(
            serde_json::from_value::<super::TimelineFollowSettlementAck>(ack_json).unwrap(),
            ack
        );

        let not_applicable_ack = super::TimelineFollowSettlementAck {
            epoch: 12,
            generation: 34,
            domain: super::TimelineFollowSettlementDomain::Video,
            consumer_id: super::TimelineFollowSettlementConsumerId::VideoOutput { output_id: 7 },
            result: super::TimelineFollowSettlementAckResult::NotApplicable,
        };
        super::validate_timeline_follow_settlement_ack(&not_applicable_ack).unwrap();
        let not_applicable_json = serde_json::to_value(&not_applicable_ack).unwrap();
        assert_eq!(not_applicable_json["result"]["kind"], "not_applicable");
        assert_eq!(
            serde_json::from_value::<super::TimelineFollowSettlementAck>(not_applicable_json)
                .unwrap(),
            not_applicable_ack
        );

        let legacy_runtime: super::TimelineFollowRuntimeSummary =
            serde_json::from_value(serde_json::json!({
                "generation": 0,
                "status": "idle"
            }))
            .unwrap();
        let legacy_public: super::TimelineFollowRuntimeStatusSnapshot =
            serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(legacy_runtime.settlement.is_none());
        assert!(legacy_public.settlement.is_none());
    }

    #[test]
    fn timeline_follow_ltl5_settlement_validation_rejects_malformed_or_duplicate_consumers() {
        let domains = vec![
            super::TimelineFollowSettlementDomainSummary {
                domain: super::TimelineFollowSettlementDomain::Audio,
                state: super::TimelineFollowSettlementState::Applied,
                fault: None,
                consumers: vec![super::TimelineFollowSettlementConsumerSummary {
                    consumer_id: super::TimelineFollowSettlementConsumerId::Audio,
                    state: super::TimelineFollowSettlementState::Applied,
                    fault: None,
                }],
            },
            super::TimelineFollowSettlementDomainSummary {
                domain: super::TimelineFollowSettlementDomain::Video,
                state: super::TimelineFollowSettlementState::Applied,
                fault: None,
                consumers: vec![super::TimelineFollowSettlementConsumerSummary {
                    consumer_id: super::TimelineFollowSettlementConsumerId::VideoOutput {
                        output_id: 5,
                    },
                    state: super::TimelineFollowSettlementState::Applied,
                    fault: None,
                }],
            },
            super::TimelineFollowSettlementDomainSummary {
                domain: super::TimelineFollowSettlementDomain::Lighting,
                state: super::TimelineFollowSettlementState::Applied,
                fault: None,
                consumers: vec![super::TimelineFollowSettlementConsumerSummary {
                    consumer_id: super::TimelineFollowSettlementConsumerId::Lighting,
                    state: super::TimelineFollowSettlementState::Applied,
                    fault: None,
                }],
            },
        ];
        let valid = super::TimelineFollowSettlementSummary {
            started_at_ms: 100,
            deadline_ms: 2_100,
            state: super::TimelineFollowSettlementState::Applied,
            progress_millis: 1_000,
            fault_policy: super::TimelineFollowFaultPolicy::Hold,
            fault: None,
            domains,
        };
        super::validate_timeline_follow_settlement_summary(&valid).unwrap();

        let mut duplicate = valid.clone();
        let duplicate_consumer = duplicate.domains[1].consumers[0].clone();
        duplicate.domains[1].consumers.push(duplicate_consumer);
        assert!(
            super::validate_timeline_follow_settlement_summary(&duplicate)
                .unwrap_err()
                .contains("duplicate")
        );

        let mut wrong_domain = valid.clone();
        wrong_domain.domains[0].consumers[0].consumer_id =
            super::TimelineFollowSettlementConsumerId::Lighting;
        assert!(
            super::validate_timeline_follow_settlement_summary(&wrong_domain)
                .unwrap_err()
                .contains("wrong domain")
        );

        let mut missing_domain = valid.clone();
        missing_domain.domains.pop();
        assert!(
            super::validate_timeline_follow_settlement_summary(&missing_domain)
                .unwrap_err()
                .contains("missing Lighting")
        );

        let mut invalid_progress = valid.clone();
        invalid_progress.progress_millis = 1_001;
        assert!(
            super::validate_timeline_follow_settlement_summary(&invalid_progress)
                .unwrap_err()
                .contains("between 0 and 1000")
        );

        let malformed_ack = super::TimelineFollowSettlementAck {
            epoch: 1,
            generation: 2,
            domain: super::TimelineFollowSettlementDomain::Audio,
            consumer_id: super::TimelineFollowSettlementConsumerId::Lighting,
            result: super::TimelineFollowSettlementAckResult::Applied,
        };
        assert!(super::validate_timeline_follow_settlement_ack(&malformed_ack).is_err());
        assert!(
            serde_json::from_value::<super::TimelineFollowSettlementAck>(serde_json::json!({
                "epoch": 1,
                "generation": 2,
                "domain": "video",
                "consumer_id": { "kind": "video_output", "output_id": 5 },
                "result": { "kind": "timed_out" }
            }))
            .is_err()
        );
    }

    #[test]
    fn timeline_follow_ltl5_settlement_derived_state_accept_reject_table_is_exact() {
        fn applied_domain(
            domain: super::TimelineFollowSettlementDomain,
            consumer_id: super::TimelineFollowSettlementConsumerId,
        ) -> super::TimelineFollowSettlementDomainSummary {
            super::TimelineFollowSettlementDomainSummary {
                domain,
                state: super::TimelineFollowSettlementState::Applied,
                fault: None,
                consumers: vec![super::TimelineFollowSettlementConsumerSummary {
                    consumer_id,
                    state: super::TimelineFollowSettlementState::Applied,
                    fault: None,
                }],
            }
        }

        fn all_applied() -> super::TimelineFollowSettlementSummary {
            super::TimelineFollowSettlementSummary {
                started_at_ms: 10,
                deadline_ms: 2_010,
                state: super::TimelineFollowSettlementState::Applied,
                progress_millis: 1_000,
                fault_policy: super::TimelineFollowFaultPolicy::Hold,
                fault: None,
                domains: vec![
                    applied_domain(
                        super::TimelineFollowSettlementDomain::Audio,
                        super::TimelineFollowSettlementConsumerId::Audio,
                    ),
                    applied_domain(
                        super::TimelineFollowSettlementDomain::Video,
                        super::TimelineFollowSettlementConsumerId::VideoOutput { output_id: 8 },
                    ),
                    applied_domain(
                        super::TimelineFollowSettlementDomain::Lighting,
                        super::TimelineFollowSettlementConsumerId::Lighting,
                    ),
                ],
            }
        }

        let applied = all_applied();

        let mut all_not_applicable = applied.clone();
        for domain in &mut all_not_applicable.domains {
            domain.state = super::TimelineFollowSettlementState::NotApplicable;
            domain.consumers.clear();
        }

        let mut captured_all_not_applicable = applied.clone();
        for domain in &mut captured_all_not_applicable.domains {
            domain.state = super::TimelineFollowSettlementState::NotApplicable;
            for consumer in &mut domain.consumers {
                consumer.state = super::TimelineFollowSettlementState::NotApplicable;
            }
        }

        let mut mixed_applied_not_applicable = applied.clone();
        mixed_applied_not_applicable.domains[1].consumers.push(
            super::TimelineFollowSettlementConsumerSummary {
                consumer_id: super::TimelineFollowSettlementConsumerId::VideoOutput {
                    output_id: 9,
                },
                state: super::TimelineFollowSettlementState::NotApplicable,
                fault: None,
            },
        );

        let mut pending = applied.clone();
        pending.state = super::TimelineFollowSettlementState::Pending;
        pending.domains[1].state = super::TimelineFollowSettlementState::Pending;
        pending.domains[1].consumers[0].state = super::TimelineFollowSettlementState::Pending;

        let mut timed_out = pending.clone();
        timed_out.state = super::TimelineFollowSettlementState::TimedOut;
        timed_out.domains[0].state = super::TimelineFollowSettlementState::TimedOut;
        timed_out.domains[0].consumers[0].state = super::TimelineFollowSettlementState::TimedOut;

        let mut fault_over_timeout = timed_out.clone();
        fault_over_timeout.state = super::TimelineFollowSettlementState::Fault;
        fault_over_timeout.fault = Some("video settlement failed".to_string());
        fault_over_timeout.domains[1].state = super::TimelineFollowSettlementState::Fault;
        fault_over_timeout.domains[1].fault = Some("present failed".to_string());
        fault_over_timeout.domains[1].consumers[0].state =
            super::TimelineFollowSettlementState::Fault;
        fault_over_timeout.domains[1].consumers[0].fault = Some("present failed".to_string());

        let mut lighting_internal_fault = applied.clone();
        lighting_internal_fault.state = super::TimelineFollowSettlementState::Fault;
        lighting_internal_fault.fault = Some("lighting merge failed".to_string());
        lighting_internal_fault.domains[2].state = super::TimelineFollowSettlementState::Fault;
        lighting_internal_fault.domains[2].fault = Some("lighting merge failed".to_string());
        lighting_internal_fault.domains[2].consumers.clear();

        let mut lighting_internal_timeout = applied.clone();
        lighting_internal_timeout.state = super::TimelineFollowSettlementState::TimedOut;
        lighting_internal_timeout.domains[2].state = super::TimelineFollowSettlementState::TimedOut;
        lighting_internal_timeout.domains[2].consumers.clear();

        for (name, accepted) in [
            ("all applied", applied.clone()),
            ("empty domains not applicable", all_not_applicable),
            (
                "captured consumers all not applicable",
                captured_all_not_applicable,
            ),
            (
                "mixed applied and not applicable consumers",
                mixed_applied_not_applicable,
            ),
            ("one pending", pending.clone()),
            ("timeout outranks pending", timed_out.clone()),
            ("fault outranks timeout", fault_over_timeout.clone()),
            ("lighting internal fault", lighting_internal_fault),
            ("lighting internal timeout", lighting_internal_timeout),
        ] {
            super::validate_timeline_follow_settlement_summary(&accepted)
                .unwrap_or_else(|error| panic!("accepted case '{name}' failed: {error}"));
        }

        let mut aggregate_applied_while_pending = pending.clone();
        aggregate_applied_while_pending.state = super::TimelineFollowSettlementState::Applied;

        let mut aggregate_pending_while_timed_out = timed_out.clone();
        aggregate_pending_while_timed_out.state = super::TimelineFollowSettlementState::Pending;

        let mut aggregate_fault_without_text = fault_over_timeout.clone();
        aggregate_fault_without_text.fault = None;

        let mut aggregate_text_without_fault = applied.clone();
        aggregate_text_without_fault.fault = Some("stale".to_string());

        let mut domain_applied_with_pending = applied.clone();
        domain_applied_with_pending.domains[1].consumers[0].state =
            super::TimelineFollowSettlementState::Pending;

        let mut domain_not_applicable_with_consumer = applied.clone();
        domain_not_applicable_with_consumer.domains[1].state =
            super::TimelineFollowSettlementState::NotApplicable;

        let mut domain_applied_without_consumer = applied.clone();
        domain_applied_without_consumer.domains[1].consumers.clear();

        let mut domain_fault_without_consumer_fault = applied.clone();
        domain_fault_without_consumer_fault.state = super::TimelineFollowSettlementState::Fault;
        domain_fault_without_consumer_fault.fault = Some("video failed".to_string());
        domain_fault_without_consumer_fault.domains[1].state =
            super::TimelineFollowSettlementState::Fault;
        domain_fault_without_consumer_fault.domains[1].fault = Some("video failed".to_string());

        let mut domain_timeout_without_consumer_timeout = applied.clone();
        domain_timeout_without_consumer_timeout.state =
            super::TimelineFollowSettlementState::TimedOut;
        domain_timeout_without_consumer_timeout.domains[0].state =
            super::TimelineFollowSettlementState::TimedOut;

        let mut domain_applied_with_all_not_applicable = applied.clone();
        domain_applied_with_all_not_applicable.domains[1].consumers[0].state =
            super::TimelineFollowSettlementState::NotApplicable;

        let mut consumer_fault_without_text = fault_over_timeout.clone();
        consumer_fault_without_text.domains[1].consumers[0].fault = None;

        let mut consumer_text_without_fault = applied.clone();
        consumer_text_without_fault.domains[1].consumers[0].fault = Some("stale".to_string());

        let mut domain_text_without_fault = applied.clone();
        domain_text_without_fault.domains[1].fault = Some("stale".to_string());

        for (name, rejected, expected) in [
            (
                "aggregate applied while pending",
                aggregate_applied_while_pending,
                "aggregate state",
            ),
            (
                "aggregate pending while timed out",
                aggregate_pending_while_timed_out,
                "aggregate state",
            ),
            (
                "aggregate fault without text",
                aggregate_fault_without_text,
                "requires non-empty fault text",
            ),
            (
                "aggregate text without fault",
                aggregate_text_without_fault,
                "allowed only for Fault state",
            ),
            (
                "domain applied with pending consumer",
                domain_applied_with_pending,
                "contradicts derived Pending",
            ),
            (
                "domain not applicable with consumer",
                domain_not_applicable_with_consumer,
                "contradicts derived Applied",
            ),
            (
                "domain applied without consumer",
                domain_applied_without_consumer,
                "must be NotApplicable",
            ),
            (
                "video domain fault without consumer fault",
                domain_fault_without_consumer_fault,
                "contradicts derived Applied",
            ),
            (
                "audio domain timeout without consumer timeout",
                domain_timeout_without_consumer_timeout,
                "contradicts derived Applied",
            ),
            (
                "domain applied with all consumers not applicable",
                domain_applied_with_all_not_applicable,
                "contradicts derived NotApplicable",
            ),
            (
                "consumer fault without text",
                consumer_fault_without_text,
                "requires non-empty fault text",
            ),
            (
                "consumer text without fault",
                consumer_text_without_fault,
                "allowed only for Fault state",
            ),
            (
                "domain text without fault",
                domain_text_without_fault,
                "allowed only for Fault state",
            ),
        ] {
            let error = super::validate_timeline_follow_settlement_summary(&rejected).unwrap_err();
            assert!(
                error.contains(expected),
                "rejected case '{name}' returned unexpected error: {error}"
            );
        }
    }

    #[test]
    fn timeline_follow_ltl5_runtime_settlement_and_unknown_video_descriptor_cannot_be_injected() {
        let mut authored = serde_json::to_value(super::TimelineSnapshot::default()).unwrap();
        authored.as_object_mut().unwrap().insert(
            "follow_runtime".to_string(),
            serde_json::json!({
                "generation": 55,
                "status": "settling",
                "settlement": {
                    "started_at_ms": 10,
                    "deadline_ms": 2010,
                    "state": "fault",
                    "progress_millis": 1000,
                    "fault_policy": "fault",
                    "domains": []
                },
                "video_descriptor": {
                    "kind": "wipe",
                    "progress_millis": 500
                }
            }),
        );
        authored.as_object_mut().unwrap().insert(
            "follow_video_runtime".to_string(),
            serde_json::json!({ "kind": "slide", "generation": 55 }),
        );

        let decoded: super::TimelineSnapshot = serde_json::from_value(authored).unwrap();
        assert_eq!(
            decoded.follow_runtime,
            super::TimelineFollowRuntimeSummary::default()
        );
        let persisted = serde_json::to_value(decoded).unwrap();
        assert!(persisted.get("follow_runtime").is_none());
        assert!(persisted.get("follow_video_runtime").is_none());
    }

    #[test]
    fn timeline_follow_ltl5_standalone_bank_requires_a_real_next_timeline() {
        let mut snapshot = super::EngineSnapshot::default();
        snapshot.timeline.id = super::TimelineId(111);
        snapshot.timeline.label = "Standalone Follow".to_string();
        snapshot.timeline.follow = Some(super::TimelineFollowSummary {
            enabled: true,
            next_timeline_id: super::TimelineId(112),
            duration: super::VideoClipTakeDuration::milliseconds(1_000),
            curve: super::VideoLayerTransitionCurve::Linear,
            video_kind: super::VideoClipTakeKind::Crossfade,
            lighting_policy: super::TimelineFollowLightingPolicy::HoldThenCut,
            destination_bpm: None,
            preroll_ms: 0,
            trans_cadence_bars: 4,
            trans_target_measures: Vec::new(),
            hold_first_destination_measure: false,
            fault_policy: super::TimelineFollowFaultPolicy::Hold,
        });
        assert!(super::validate_timeline_bank(&snapshot, &[])
            .unwrap_err()
            .contains("no Timeline below"));

        snapshot.timeline.follow.as_mut().unwrap().enabled = false;
        super::validate_timeline_bank(&snapshot, &[]).unwrap();
        snapshot.timeline.follow = None;
        super::validate_timeline_bank(&snapshot, &[]).unwrap();
    }

    #[test]
    fn timeline_follow_ltl5_validation_bounds_and_legacy_defaults_are_fail_closed() {
        let mut timeline = super::TimelineSnapshot {
            id: super::TimelineId(101),
            label: "Follow bounds".to_string(),
            duration_ms: super::TIMELINE_FOLLOW_MAX_PREROLL_MS,
            follow: Some(super::TimelineFollowSummary {
                enabled: true,
                next_timeline_id: super::TimelineId(102),
                duration: super::VideoClipTakeDuration {
                    unit: super::VideoClipTakeDurationUnit::Beats,
                    value_milliunits: super::TIMELINE_FOLLOW_MAX_DURATION_BEAT_MILLIUNITS,
                },
                curve: super::VideoLayerTransitionCurve::EaseInOut,
                video_kind: super::VideoClipTakeKind::Crossfade,
                lighting_policy: super::TimelineFollowLightingPolicy::LinearMerge,
                destination_bpm: Some(20.0),
                preroll_ms: super::TIMELINE_FOLLOW_MAX_PREROLL_MS,
                trans_cadence_bars: super::TIMELINE_FOLLOW_MAX_TRANS_CADENCE_BARS,
                trans_target_measures: Vec::new(),
                hold_first_destination_measure: false,
                fault_policy: super::TimelineFollowFaultPolicy::Cut,
            }),
            ..Default::default()
        };
        super::validate_timeline_authoring(&timeline, &[]).unwrap();

        timeline.follow.as_mut().unwrap().duration.value_milliunits += 1;
        assert!(super::validate_timeline_authoring(&timeline, &[])
            .unwrap_err()
            .contains("invalid timing"));
        timeline.follow.as_mut().unwrap().duration.value_milliunits = 1_000;
        timeline.follow.as_mut().unwrap().preroll_ms = timeline.duration_ms + 1;
        assert!(super::validate_timeline_authoring(&timeline, &[])
            .unwrap_err()
            .contains("invalid timing"));
        timeline.follow.as_mut().unwrap().preroll_ms = 0;
        timeline.follow.as_mut().unwrap().trans_cadence_bars = 0;
        assert!(super::validate_timeline_authoring(&timeline, &[])
            .unwrap_err()
            .contains("Guide cadence"));
        timeline.follow.as_mut().unwrap().trans_cadence_bars = 4;
        timeline.follow.as_mut().unwrap().video_kind = super::VideoClipTakeKind::Cut;
        timeline.follow.as_mut().unwrap().duration.value_milliunits = 0;
        timeline.follow.as_mut().unwrap().preroll_ms = 1;
        assert!(super::validate_timeline_authoring(&timeline, &[])
            .unwrap_err()
            .contains("invalid timing"));

        let mut legacy_json = serde_json::to_value(super::TimelineSnapshot::default()).unwrap();
        legacy_json.as_object_mut().unwrap().remove("follow");
        legacy_json
            .as_object_mut()
            .unwrap()
            .remove("follow_runtime");
        let legacy: super::TimelineSnapshot = serde_json::from_value(legacy_json).unwrap();
        assert!(legacy.follow.is_none());
        assert_eq!(
            legacy.follow_runtime,
            super::TimelineFollowRuntimeSummary::default()
        );

        let valid_follow = serde_json::to_value(timeline.follow.unwrap()).unwrap();
        for key in ["curve", "video_kind", "lighting_policy", "fault_policy"] {
            let mut invalid = valid_follow.clone();
            invalid.as_object_mut().unwrap().insert(
                key.to_string(),
                serde_json::Value::String("not_a_follow_variant".to_string()),
            );
            assert!(serde_json::from_value::<super::TimelineFollowSummary>(invalid).is_err());
        }
    }

    #[test]
    fn timeline_follow_exact_trans_targets_require_sorted_in_range_measure_anchors() {
        let mut timeline = super::TimelineSnapshot {
            id: super::TimelineId(101),
            label: "Exact Trans targets".to_string(),
            tempo_meter_map: vec![
                super::TimelineTempoMeterPoint {
                    position_sixteenth_steps: 0,
                    measure_number: Some(148),
                    bpm: 170.0,
                    interpolation: super::TimelineTempoInterpolation::Linear,
                    ..super::TimelineTempoMeterPoint::default()
                },
                super::TimelineTempoMeterPoint {
                    position_sixteenth_steps: 144,
                    measure_number: Some(157),
                    bpm: 194.0,
                    ..super::TimelineTempoMeterPoint::default()
                },
            ],
            follow: Some(super::TimelineFollowSummary {
                enabled: true,
                next_timeline_id: super::TimelineId(102),
                duration: super::VideoClipTakeDuration {
                    unit: super::VideoClipTakeDurationUnit::Beats,
                    value_milliunits: 4_000,
                },
                curve: super::VideoLayerTransitionCurve::EaseInOut,
                video_kind: super::VideoClipTakeKind::Crossfade,
                lighting_policy: super::TimelineFollowLightingPolicy::LinearMerge,
                destination_bpm: Some(194.0),
                preroll_ms: 0,
                trans_cadence_bars: 4,
                trans_target_measures: vec![149, 151, 153, 155],
                hold_first_destination_measure: false,
                fault_policy: super::TimelineFollowFaultPolicy::Hold,
            }),
            ..Default::default()
        };
        super::validate_timeline_authoring(&timeline, &[]).unwrap();

        for (targets, expected) in [
            (vec![149, 149], "sorted"),
            (vec![151, 149], "sorted"),
            (vec![147], "outside"),
            (vec![158], "outside"),
            (vec![0], "non-zero"),
        ] {
            timeline.follow.as_mut().unwrap().trans_target_measures = targets;
            assert!(super::validate_timeline_authoring(&timeline, &[])
                .unwrap_err()
                .contains(expected));
        }

        timeline.follow.as_mut().unwrap().trans_target_measures = vec![149];
        timeline.tempo_meter_map.clear();
        assert!(super::validate_timeline_authoring(&timeline, &[])
            .unwrap_err()
            .contains("measure anchors"));

        timeline
            .follow
            .as_mut()
            .unwrap()
            .trans_target_measures
            .clear();
        super::validate_timeline_authoring(&timeline, &[]).unwrap();
    }

    #[test]
    fn active_child_transport_runtime_is_never_serialized() {
        let snapshot = super::TimelineSnapshot {
            active_child_transports: vec![
                super::ChildTimelineTransportRuntimeSummary {
                    owner_cue_id: 7,
                    root: super::ChildTimelineTransportRootSummary::Timeline {
                        parent_event_id: 100,
                        parent_iteration: 2,
                    },
                    path: vec![super::ChildTimelineTransportPathSegment {
                        event_id: 200,
                        iteration: 3,
                    }],
                    position_ms: 450,
                    audio_position_ms: 450,
                    audio_active: true,
                    playback_rate_milli: 500,
                },
                super::ChildTimelineTransportRuntimeSummary {
                    owner_cue_id: 8,
                    root: super::ChildTimelineTransportRootSummary::Follow {
                        source_timeline_id: super::TimelineId(91),
                        generation: 17,
                    },
                    path: Vec::new(),
                    position_ms: 1_000,
                    audio_position_ms: 1_000,
                    audio_active: true,
                    playback_rate_milli: 1_000,
                },
            ],
            ..Default::default()
        };

        let follow_root = snapshot.active_child_transports[1].root.clone();
        let follow_root_json = serde_json::to_value(&follow_root).unwrap();
        assert_eq!(follow_root_json["follow"]["source_timeline_id"], 91);
        assert_eq!(
            serde_json::from_value::<super::ChildTimelineTransportRootSummary>(follow_root_json)
                .unwrap(),
            follow_root
        );

        let encoded = serde_json::to_string(&snapshot).unwrap();
        assert!(!encoded.contains("active_child_transports"));
        assert!(
            !encoded.contains("playback_rate_milli"),
            "runtime-only child audio speed must never enter project JSON"
        );
        assert!(
            !encoded.contains("audio_position_ms"),
            "runtime-only child audio coordinate must never enter project JSON"
        );
        assert!(
            !encoded.contains("audio_active"),
            "runtime-only child audio admission must never enter project JSON"
        );
        let decoded: super::TimelineSnapshot = serde_json::from_str(&encoded).unwrap();
        assert!(decoded.active_child_transports.is_empty());
    }

    #[test]
    fn timeline_audio_clip_serde_defaults_are_sensible() {
        let clip: super::TimelineAudioClipSummary =
            serde_json::from_value(serde_json::json!({})).unwrap();

        assert_eq!(clip.id, 0);
        assert_eq!(clip.layer_id, 0);
        assert!(clip.path.is_empty());
        assert_eq!(clip.start_ms, 0);
        assert_eq!(clip.offset_ms, 0);
        assert_eq!(clip.duration_ms, 0);
        assert_eq!(clip.gain, 1.0);
        assert_eq!(clip.fade_in_ms, 0);
        assert_eq!(clip.fade_out_ms, 0);
        assert_eq!(clip.output_bus, super::TimelineAudioOutputBus::Program);
    }

    #[test]
    fn timeline_audio_clip_and_master_fields_roundtrip() {
        let snapshot = super::TimelineSnapshot {
            audio_clips: vec![super::TimelineAudioClipSummary {
                id: 4,
                layer_id: 12,
                media_asset_id: None,
                path: "music/show.wav".to_string(),
                start_ms: 1_000,
                offset_ms: 250,
                duration_ms: 8_000,
                gain: 1.25,
                fade_in_ms: 500,
                fade_out_ms: 750,
                output_bus: super::TimelineAudioOutputBus::Cue,
            }],
            audio_offset_ms: -250,
            audio_muted: true,
            ..super::TimelineSnapshot::default()
        };

        let encoded = serde_json::to_string(&snapshot).unwrap();
        let decoded: super::TimelineSnapshot = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, snapshot);
        assert_eq!(
            serde_json::to_value(&decoded).unwrap()["audio_clips"][0]["output_bus"],
            "CUE"
        );
    }

    #[test]
    fn timeline_audio_clip_output_bus_rejects_explicit_unknown_values() {
        let legacy: super::TimelineAudioClipSummary = serde_json::from_value(serde_json::json!({
            "id": 4,
            "layer_id": 12,
            "path": "music/show.wav",
            "duration_ms": 8_000
        }))
        .unwrap();
        assert_eq!(legacy.output_bus, super::TimelineAudioOutputBus::Program);

        for output_bus in ["cue_v2", "SIDECHAIN", "program"] {
            let error =
                serde_json::from_value::<super::TimelineAudioClipSummary>(serde_json::json!({
                    "id": 4,
                    "layer_id": 12,
                    "path": "music/show.wav",
                    "duration_ms": 8_000,
                    "output_bus": output_bus
                }))
                .unwrap_err();
            assert!(error.to_string().contains("unknown variant"));
        }
    }

    #[test]
    fn timeline_audio_clip_persists_only_logical_bus_not_physical_output_mapping() {
        let clip = super::TimelineAudioClipSummary {
            id: 4,
            layer_id: 12,
            media_asset_id: None,
            path: "music/cue.wav".to_string(),
            start_ms: 0,
            offset_ms: 0,
            duration_ms: 8_000,
            gain: 1.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: super::TimelineAudioOutputBus::Cue,
        };
        let encoded = serde_json::to_value(clip).unwrap();

        assert_eq!(encoded["output_bus"], "CUE");
        for forbidden in [
            "backend",
            "driver",
            "device",
            "program_left",
            "program_right",
            "cue_channel",
            "spare_channel",
        ] {
            assert!(
                encoded.get(forbidden).is_none(),
                "portable Timeline Audio Clip must not carry {forbidden}"
            );
        }
    }

    #[test]
    fn timeline_scene_block_fields_roundtrip() {
        let event = super::TimelineCueEventSummary {
            id: 9,
            cue_id: 3,
            time_ms: 1_250,
            time_beats: Some(2.5),
            track: super::TimelineTrackKind::Lighting,
            layer_id: Some(7),
            duration_ms: 2_000,
            duration_beats: Some(4.0),
            conform_to_tempo: true,
            loop_fill: true,
            source_offset_ms: 375,
            rate: Some(1.25),
            fade_in_ms: 350,
            fade_out_ms: 700,
            loop_count: 4,
            jump_to_event_id: Some(2),
        };

        let encoded = serde_json::to_string(&event).unwrap();
        let decoded: super::TimelineCueEventSummary = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, event);
        let encoded_value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(encoded_value["track"], "Lighting");
        assert_eq!(encoded_value["layer_id"], 7);
        assert_eq!(encoded_value["source_offset_ms"], 375);
    }

    #[test]
    fn timeline_scene_block_negative_source_position_roundtrips() {
        let event = super::TimelineCueEventSummary {
            id: 10,
            cue_id: 3,
            time_ms: 2_000,
            track: super::TimelineTrackKind::Lighting,
            duration_ms: 1_000,
            source_offset_ms: -375,
            ..super::TimelineCueEventSummary::default()
        };

        let encoded = serde_json::to_string(&event).unwrap();
        let decoded: super::TimelineCueEventSummary = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, event);
        let encoded_value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(encoded_value["source_offset_ms"], -375);
    }

    #[test]
    fn timeline_snap_request_roundtrips_complete_items_and_defaults_missing_lists() {
        let request = super::TimelineSnapRequest {
            event_placements: vec![super::TimelineEventPlacementUpdate {
                event_id: 9,
                cue_id: 3,
                time_ms: 1_250,
                time_beats: Some(2.5),
                track: super::TimelineTrackKind::Lighting,
                layer_id: Some(7),
                duration_ms: 2_000,
                duration_beats: Some(4.0),
                conform_to_tempo: true,
                loop_fill: true,
                fade_in_ms: 350,
                fade_out_ms: 700,
                loop_count: 4,
                jump_to_event_id: Some(2),
            }],
            lighting_automations: vec![super::TimelineAutomationKeyframesUpdate {
                automation_id: 11,
                keyframes: vec![super::AutomationKeyframeSummary {
                    time_ms: 1_000,
                    value: 32_768,
                    interpolation: super::AutomationInterpolation::Linear,
                }],
            }],
            video_automations: vec![super::TimelineVideoAutomationKeyframesUpdate {
                automation_id: 12,
                keyframes: vec![super::VideoAutomationKeyframeSummary {
                    time_ms: 1_000,
                    value: 0.5,
                    interpolation: super::AutomationInterpolation::Step,
                }],
            }],
        };

        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: super::TimelineSnapRequest = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, request);

        let legacy_placement: super::TimelineEventPlacementUpdate =
            serde_json::from_value(serde_json::json!({
                "event_id": 9,
                "cue_id": 3,
                "time_ms": 1_250,
                "track": "Lighting"
            }))
            .unwrap();
        assert_eq!(legacy_placement.time_beats, None);
        assert_eq!(legacy_placement.duration_beats, None);
        assert!(!legacy_placement.conform_to_tempo);
        assert!(!legacy_placement.loop_fill);
        assert_eq!(legacy_placement.fade_in_ms, 0);
        assert_eq!(legacy_placement.fade_out_ms, 0);

        let defaults: super::TimelineSnapRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(defaults, super::TimelineSnapRequest::default());
    }

    #[test]
    fn cue_effect_targets_roundtrip() {
        let cue = super::CueSummary {
            id: 17,
            effect_targets: vec![
                super::CueEffectTarget {
                    effect_id: 3,
                    enabled: true,
                    params: None,
                    transition_ms: None,
                },
                super::CueEffectTarget {
                    effect_id: 8,
                    enabled: false,
                    params: None,
                    transition_ms: None,
                },
            ],
            ..Default::default()
        };

        let encoded = serde_json::to_string(&cue).unwrap();
        let decoded: super::CueSummary = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, cue);
    }

    #[test]
    fn cue_effect_target_params_roundtrip_and_legacy_default() {
        let target = super::CueEffectTarget {
            effect_id: 3,
            enabled: true,
            params: Some(super::EffectParamsSnapshot::Lfo(super::LfoEffectRequest {
                label: "Owned pulse".to_string(),
                fixture_ids: vec![7],
                target_group_ids: vec!["Front".to_string()],
                attribute: "Dimmer".to_string(),
                video_targets: Vec::new(),
                shape: super::LfoShape::Sine,
                period_ms: 1_000,
                clock_sync: Some(super::EffectClockSync { beats: 2.0 }),
                low: 1_024,
                high: 60_000,
                phase: 0.25,
                fixture_spread: 0.5,
                beam_targets: vec![super::EffectBeamTarget {
                    fixture_id: 7,
                    beam_index: 3,
                    selection_index: 2,
                    feature_attribute: "Dimmer".to_string(),
                }],
                blend_mode: super::EffectBlendMode::Override,
                daslight_curve: None,
                daslight_custom_curve: None,
            })),
            transition_ms: Some(900),
        };

        let encoded = serde_json::to_string(&target).unwrap();
        let decoded: super::CueEffectTarget = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, target);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&encoded).unwrap()["params"]["Lfo"]
                ["period_ms"],
            1_000
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&encoded).unwrap()["params"]["Lfo"]
                ["fixture_spread"],
            0.5
        );
        assert_eq!(decoded.transition_ms, Some(900));
        assert!(
            serde_json::from_str::<serde_json::Value>(&encoded).unwrap()["params"]["Lfo"]
                .get("daslight_curve")
                .is_none()
        );
        assert!(
            serde_json::from_str::<serde_json::Value>(&encoded).unwrap()["params"]["Lfo"]
                .get("daslight_custom_curve")
                .is_none()
        );

        let mut imported = target.clone();
        let Some(super::EffectParamsSnapshot::Lfo(imported_request)) = imported.params.as_mut()
        else {
            panic!("imported target must contain LFO params");
        };
        imported_request.daslight_curve = Some(super::DaslightCurveSource {
            rate: 2.0,
            size: 1.562,
            offset: -0.848,
            sample_ms: 40,
            rng_seed: None,
        });
        let imported_encoded = serde_json::to_string(&imported).unwrap();
        let imported_decoded: super::CueEffectTarget =
            serde_json::from_str(&imported_encoded).unwrap();
        assert_eq!(imported_decoded, imported);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&imported_encoded).unwrap()["params"]["Lfo"]
                ["daslight_curve"]["sample_ms"],
            40
        );
        assert!(
            serde_json::from_str::<serde_json::Value>(&imported_encoded).unwrap()["params"]["Lfo"]
                ["daslight_curve"]
                .get("rng_seed")
                .is_none()
        );

        let mut custom = target.clone();
        let Some(super::EffectParamsSnapshot::Lfo(custom_request)) = custom.params.as_mut() else {
            panic!("Custom target must contain LFO params");
        };
        custom_request.shape = super::LfoShape::DaslightCustom;
        custom_request.phase = 0.0;
        custom_request.fixture_spread = 0.0;
        custom_request.daslight_custom_curve = Some(super::DaslightCustomCurveSource {
            points: vec![
                super::DaslightCustomCurvePoint { x: 0.0, raw_y: 0.5 },
                super::DaslightCustomCurvePoint {
                    x: 1.0,
                    raw_y: 31.0,
                },
            ],
            phasing: 0.25,
            sample_ms: 40,
        });
        let custom_encoded = serde_json::to_string(&custom).unwrap();
        let custom_decoded: super::CueEffectTarget = serde_json::from_str(&custom_encoded).unwrap();
        assert_eq!(custom_decoded, custom);
        let custom_json = serde_json::from_str::<serde_json::Value>(&custom_encoded).unwrap();
        assert_eq!(
            custom_json["params"]["Lfo"]["daslight_custom_curve"]["points"][1]["raw_y"],
            31.0
        );
        assert_eq!(
            custom_json["params"]["Lfo"]["daslight_custom_curve"]["phasing"],
            0.25
        );

        let mut imported_random = imported.clone();
        let Some(super::EffectParamsSnapshot::Lfo(random_request)) =
            imported_random.params.as_mut()
        else {
            panic!("imported random target must contain LFO params");
        };
        random_request.shape = super::LfoShape::Random;
        random_request.daslight_curve.as_mut().unwrap().rng_seed = Some(0x1357_9BDF);
        let random_encoded = serde_json::to_string(&imported_random).unwrap();
        let random_decoded: super::CueEffectTarget = serde_json::from_str(&random_encoded).unwrap();
        assert_eq!(random_decoded, imported_random);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&random_encoded).unwrap()["params"]["Lfo"]
                ["daslight_curve"]["rng_seed"],
            0x1357_9BDF_u32
        );

        let mut legacy_owned_value = serde_json::to_value(&target).unwrap();
        legacy_owned_value["params"]["Lfo"]
            .as_object_mut()
            .unwrap()
            .remove("fixture_spread");
        legacy_owned_value["params"]["Lfo"]
            .as_object_mut()
            .unwrap()
            .remove("beam_targets");
        legacy_owned_value["params"]["Lfo"]
            .as_object_mut()
            .unwrap()
            .remove("daslight_curve");
        let legacy_owned: super::CueEffectTarget =
            serde_json::from_value(legacy_owned_value).unwrap();
        let Some(super::EffectParamsSnapshot::Lfo(legacy_request)) = legacy_owned.params else {
            panic!("legacy owned LFO must remain an LFO");
        };
        assert_eq!(legacy_request.fixture_spread, 0.0);
        assert!(legacy_request.beam_targets.is_empty());
        assert!(legacy_request.daslight_curve.is_none());
        assert!(serde_json::to_value(legacy_request)
            .unwrap()
            .get("fixture_spread")
            .is_none());

        let legacy: super::CueEffectTarget =
            serde_json::from_str(r#"{"effect_id":3,"enabled":true}"#).unwrap();
        assert!(legacy.params.is_none());
        assert!(legacy.transition_ms.is_none());
        let legacy_value = serde_json::to_value(legacy).unwrap();
        assert!(legacy_value.get("params").is_none());
        assert!(legacy_value.get("transition_ms").is_none());
    }

    #[test]
    fn legacy_engine_snapshot_defaults_reference_palettes_to_empty() {
        let mut value = serde_json::to_value(super::EngineSnapshot::default()).unwrap();
        value.as_object_mut().unwrap().remove("palettes");
        value.as_object_mut().unwrap().remove("playback_executors");
        value.as_object_mut().unwrap().remove("playback_master");

        let parsed: super::EngineSnapshot = serde_json::from_value(value).unwrap();

        assert!(parsed.palettes.is_empty());
        assert_eq!(
            parsed.playback_executors,
            vec![super::PlaybackExecutorSummary::default()]
        );
        assert_eq!(parsed.playback_master, 1.0);
    }

    #[test]
    fn legacy_reference_palette_defaults_fx_color_stops_to_empty() {
        let parsed: super::ReferencePaletteSummary = serde_json::from_str(
            r#"{"id":7,"label":"Legacy look","kind":"Color","values":[{"attribute":"Red","value":65535}]}"#,
        )
        .unwrap();

        assert!(parsed.color_stops.is_empty());
        assert_eq!(parsed.values.len(), 1);
        let encoded = serde_json::to_value(parsed).unwrap();
        assert!(encoded.get("color_stops").is_none());
    }

    #[test]
    fn legacy_clock_snapshot_defaults_external_sync_health() {
        let mut value = serde_json::to_value(super::ClockSnapshot::default()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("external_sync_age_ms");
        value
            .as_object_mut()
            .unwrap()
            .remove("external_sync_locked");

        let parsed: super::ClockSnapshot = serde_json::from_value(value).unwrap();

        assert_eq!(parsed.external_sync_age_ms, None);
        assert!(!parsed.external_sync_locked);
    }

    #[test]
    fn legacy_video_snapshot_defaults_auto_vj_to_safe_off_state() {
        let mut value = serde_json::to_value(super::VideoSnapshot::default()).unwrap();
        value.as_object_mut().unwrap().remove("auto_vj");

        let parsed: super::VideoSnapshot = serde_json::from_value(value).unwrap();

        assert_eq!(parsed.auto_vj, super::AutoVjSnapshot::default());
        assert!(!parsed.auto_vj.status.armed);
        assert!(matches!(parsed.auto_vj.status.mode, super::AutoVjMode::Off));
    }

    fn media_asset_test_source(path: &str) -> super::VideoSourceSummary {
        super::VideoSourceSummary {
            kind: super::VideoSourceKind::File,
            path: Some(path.to_string()),
            name: None,
            codec: Some("h264".to_string()),
            metadata: Some(super::VideoMediaMetadata {
                duration_ms: Some(1_250),
                width: Some(1920),
                height: Some(1080),
                frame_rate: Some(29.97),
                has_audio: true,
            }),
        }
    }

    fn media_asset_test_layer(
        id: super::VideoLayerId,
        label: &str,
        path: &str,
    ) -> super::VideoLayerSummary {
        super::VideoLayerSummary {
            id,
            label: label.to_string(),
            source: media_asset_test_source(path),
            media_asset_id: None,
            blend_mode: super::VideoBlendMode::Screen,
            state: super::VideoLayerState {
                opacity: 0.72,
                speed: 1.25,
                playing: true,
                position_ms: 444,
                loop_enabled: true,
                loop_start_ms: 100,
                loop_end_ms: 900,
                ..super::VideoLayerState::default()
            },
            isf_effect: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
        }
    }

    fn media_asset_test_asset(
        id: super::MediaAssetId,
        label: &str,
        path: &str,
    ) -> super::MediaAssetSummary {
        super::MediaAssetSummary {
            id,
            label: label.to_string(),
            source: media_asset_test_source(path),
            content_hash: None,
            byte_size: None,
        }
    }

    #[test]
    fn media_asset_legacy_wire_roundtrip_preserves_exact_source_and_omits_machine_availability() {
        let legacy_snapshot = super::VideoSnapshot {
            layers: vec![media_asset_test_layer(
                4,
                "Legacy Cut",
                "C:/show/legacy.mp4",
            )],
            media_assets: Vec::new(),
            ..super::VideoSnapshot::default()
        };
        let legacy_source = legacy_snapshot.layers[0].source.clone();
        let mut legacy_json = serde_json::to_value(&legacy_snapshot).unwrap();
        legacy_json.as_object_mut().unwrap().remove("media_assets");
        legacy_json["layers"][0]
            .as_object_mut()
            .unwrap()
            .remove("media_asset_id");

        let mut parsed: super::VideoSnapshot = serde_json::from_value(legacy_json).unwrap();
        assert!(parsed.media_assets.is_empty());
        assert_eq!(parsed.layers[0].media_asset_id, None);
        assert!(parsed.layers[0].clip_slots.is_empty());
        assert_eq!(parsed.layers[0].default_clip_slot_id, None);
        assert_eq!(parsed.layers[0].source, legacy_source);

        let report = super::normalize_legacy_video_media_assets(&mut parsed).unwrap();
        assert_eq!(report.created_asset_ids, vec![5]);
        assert_eq!(parsed.layers[0].media_asset_id, Some(5));
        assert_eq!(parsed.media_assets[0].source, legacy_source);
        let encoded = serde_json::to_value(&parsed).unwrap();
        assert!(encoded.get("media_assets").is_some());
        assert!(encoded["layers"][0].get("media_asset_id").is_some());
        assert!(!serde_json::to_string(&encoded)
            .unwrap()
            .contains("availability"));

        #[derive(serde::Deserialize)]
        struct LegacyLayerReader {
            source: super::VideoSourceSummary,
        }
        #[derive(serde::Deserialize)]
        struct LegacyVideoReader {
            layers: Vec<LegacyLayerReader>,
        }
        let legacy_reader: LegacyVideoReader = serde_json::from_value(encoded).unwrap();
        assert_eq!(legacy_reader.layers[0].source, legacy_source);
    }

    #[test]
    fn media_asset_migration_is_all_or_nothing_and_second_pass_byte_equivalent() {
        let mut video = super::VideoSnapshot {
            layers: vec![
                media_asset_test_layer(7, "Existing Ref", "C:/show/existing.mp4"),
                media_asset_test_layer(8, "Legacy A", "C:/show/a.mp4"),
                media_asset_test_layer(9, "Legacy B", "C:/show/b.mp4"),
            ],
            media_assets: vec![media_asset_test_asset(
                40,
                "Existing",
                "C:/show/existing.mp4",
            )],
            ..super::VideoSnapshot::default()
        };
        video.layers[0].media_asset_id = Some(40);
        let original_legacy_a = video.layers[1].clone();
        let original_legacy_b = video.layers[2].clone();

        let report = super::normalize_legacy_video_media_assets(&mut video).unwrap();
        assert_eq!(report.created_asset_ids, vec![41, 42]);
        assert_eq!(report.migrated_layer_ids, vec![8, 9]);
        assert_eq!(video.layers[0].media_asset_id, Some(40));
        assert_eq!(video.layers[1].media_asset_id, Some(41));
        assert_eq!(video.layers[2].media_asset_id, Some(42));
        assert_eq!(video.media_assets[1].label, original_legacy_a.label);
        assert_eq!(video.media_assets[1].source, original_legacy_a.source);
        assert_eq!(video.media_assets[2].label, original_legacy_b.label);
        assert_eq!(video.media_assets[2].source, original_legacy_b.source);

        let once = serde_json::to_vec(&video).unwrap();
        let second = super::normalize_legacy_video_media_assets(&mut video).unwrap();
        assert!(second.created_asset_ids.is_empty());
        assert!(second.migrated_layer_ids.is_empty());
        assert_eq!(serde_json::to_vec(&video).unwrap(), once);
        super::validate_engine_ready_video_media_assets(&video).unwrap();

        let invalid_cases = [
            {
                super::VideoSnapshot {
                    layers: vec![media_asset_test_layer(1, "Legacy", "C:/show/a.mp4")],
                    media_assets: vec![
                        media_asset_test_asset(3, "A", "C:/show/a.mp4"),
                        media_asset_test_asset(3, "B", "C:/show/b.mp4"),
                    ],
                    ..super::VideoSnapshot::default()
                }
            },
            {
                let mut value = super::VideoSnapshot {
                    layers: vec![media_asset_test_layer(1, "Dangling", "C:/show/a.mp4")],
                    ..super::VideoSnapshot::default()
                };
                value.layers[0].media_asset_id = Some(88);
                value
            },
            {
                let mut value = super::VideoSnapshot {
                    layers: vec![media_asset_test_layer(1, "Mismatch", "C:/show/a.mp4")],
                    media_assets: vec![media_asset_test_asset(2, "Other", "C:/show/b.mp4")],
                    ..super::VideoSnapshot::default()
                };
                value.layers[0].media_asset_id = Some(2);
                value
            },
            super::VideoSnapshot {
                layers: vec![media_asset_test_layer(1, "Overflow", "C:/show/a.mp4")],
                media_assets: vec![media_asset_test_asset(
                    u64::MAX,
                    "Maximum",
                    "C:/show/existing.mp4",
                )],
                ..super::VideoSnapshot::default()
            },
        ];
        for mut invalid in invalid_cases {
            let before = invalid.clone();
            assert!(super::normalize_legacy_video_media_assets(&mut invalid).is_err());
            assert_eq!(invalid, before);
        }
    }

    fn clip_slot_test_slot(
        id: u64,
        media_asset_id: super::MediaAssetId,
    ) -> super::VideoClipSlotSummary {
        super::VideoClipSlotSummary {
            id: super::VideoClipSlotId(id),
            media_asset_id,
            in_point_ms: 0,
            out_point_ms: None,
            loop_mode: super::VideoClipLoopMode::Once,
            speed: 1.0,
            cue_points: Vec::new(),
            launch_quantization: super::VideoClipLaunchQuantization::Immediate,
            effect_overrides: Vec::new(),
        }
    }

    fn clip_slot_runtime_test_video() -> super::VideoSnapshot {
        let mut layer = media_asset_test_layer(1, "Layer", "C:/show/a.mp4");
        layer.media_asset_id = Some(10);
        layer.clip_slots = vec![clip_slot_test_slot(20, 10), clip_slot_test_slot(21, 10)];
        layer.clip_slots[0].loop_mode = super::VideoClipLoopMode::PingPong;
        layer.default_clip_slot_id = Some(super::VideoClipSlotId(20));
        super::VideoSnapshot {
            layers: vec![layer],
            media_assets: vec![media_asset_test_asset(10, "A", "C:/show/a.mp4")],
            ..super::VideoSnapshot::default()
        }
    }

    fn clip_slot_runtime_test_snapshot() -> super::VideoClipRuntimeSnapshot {
        super::VideoClipRuntimeSnapshot {
            layers: vec![super::VideoClipLayerRuntimeSummary {
                layer_id: 1,
                active_slot_id: Some(super::VideoClipSlotId(20)),
                queued_slot_id: Some(super::VideoClipSlotId(21)),
                pending_launch: Some(super::VideoClipPendingLaunchSummary {
                    slot_id: super::VideoClipSlotId(21),
                    quantization: super::VideoClipLaunchQuantization::NextBar,
                    target_boundary_ordinal: 16,
                    clock_generation: 3,
                    held_for_clock_discontinuity: true,
                    transition_kind: super::VideoClipTakeKind::Cut,
                    duration: super::VideoClipTakeDuration::milliseconds(0),
                    resolved_duration_ms: 0,
                }),
                transition: None,
                playhead_ms: 400,
                playing: true,
                ping_pong_reverse: true,
            }],
            ..super::VideoClipRuntimeSnapshot::default()
        }
    }

    #[test]
    fn video_clip_runtime_dto_defaults_roundtrips_and_stays_out_of_authored_snapshots() {
        let default_runtime: super::VideoClipRuntimeSnapshot =
            serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(default_runtime, super::VideoClipRuntimeSnapshot::default());

        let runtime = clip_slot_runtime_test_snapshot();
        let encoded = serde_json::to_vec(&runtime).unwrap();
        let decoded: super::VideoClipRuntimeSnapshot = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, runtime);

        let legacy_layer: super::VideoClipLayerRuntimeSummary =
            serde_json::from_value(serde_json::json!({ "layer_id": 1 })).unwrap();
        assert_eq!(legacy_layer.playhead_ms, 0);
        assert!(!legacy_layer.playing);
        assert!(!legacy_layer.ping_pong_reverse);
        assert_eq!(legacy_layer.active_slot_id, None);
        assert_eq!(legacy_layer.queued_slot_id, None);
        assert_eq!(legacy_layer.pending_launch, None);
        assert_eq!(legacy_layer.transition, None);

        let zero_boundary: super::VideoClipPendingLaunchSummary =
            serde_json::from_value(serde_json::json!({
                "slot_id": 21,
                "quantization": "NextBeat",
                "target_boundary_ordinal": 0,
                "clock_generation": 0
            }))
            .unwrap();
        assert_eq!(zero_boundary.target_boundary_ordinal, 0);
        assert_eq!(zero_boundary.clock_generation, 0);
        assert!(!zero_boundary.held_for_clock_discontinuity);
        super::validate_video_clip_runtime_snapshot(&super::VideoClipRuntimeSnapshot {
            layers: vec![super::VideoClipLayerRuntimeSummary {
                layer_id: 1,
                queued_slot_id: Some(zero_boundary.slot_id),
                pending_launch: Some(zero_boundary.clone()),
                ..super::VideoClipLayerRuntimeSummary::default()
            }],
            ..super::VideoClipRuntimeSnapshot::default()
        })
        .unwrap();

        let complete_pending = serde_json::json!({
            "slot_id": 21,
            "quantization": "NextBeat",
            "target_boundary_ordinal": 0,
            "clock_generation": 0
        });
        for required_field in [
            "slot_id",
            "quantization",
            "target_boundary_ordinal",
            "clock_generation",
        ] {
            let mut missing = complete_pending.clone();
            missing.as_object_mut().unwrap().remove(required_field);
            assert!(
                serde_json::from_value::<super::VideoClipPendingLaunchSummary>(missing).is_err()
            );
        }

        let authored_json = serde_json::to_value(clip_slot_runtime_test_video()).unwrap();
        let engine = super::EngineSnapshot {
            video_clip_runtime: runtime.clone(),
            ..Default::default()
        };
        let engine_clone = engine.clone();
        assert_eq!(engine_clone.video_clip_runtime, runtime);
        let engine_json = serde_json::to_value(&engine).unwrap();
        assert!(engine_json.get("video_clip_runtime").is_none());
        assert!(authored_json.get("video_clip_runtime").is_none());
        for runtime_field in [
            "active_slot_id",
            "queued_slot_id",
            "pending_launch",
            "playhead_ms",
            "ping_pong_reverse",
        ] {
            assert!(!authored_json.to_string().contains(runtime_field));
            assert!(!engine_json.to_string().contains(runtime_field));
        }

        let mut injected_engine_json = engine_json;
        injected_engine_json.as_object_mut().unwrap().insert(
            "video_clip_runtime".to_string(),
            serde_json::to_value(&runtime).unwrap(),
        );
        let decoded_engine: super::EngineSnapshot =
            serde_json::from_value(injected_engine_json).unwrap();
        assert_eq!(
            decoded_engine.video_clip_runtime,
            super::VideoClipRuntimeSnapshot::default()
        );
    }

    #[test]
    fn video_clip_runtime_validation_rejects_invalid_or_dangling_runtime_truth() {
        let video = clip_slot_runtime_test_video();
        let runtime = clip_slot_runtime_test_snapshot();
        super::validate_video_clip_runtime_snapshot(&runtime).unwrap();
        super::validate_video_clip_runtime_against_authored_slots(&runtime, &video).unwrap();
        let mut invalid_pending_timing = runtime.clone();
        invalid_pending_timing.layers[0]
            .pending_launch
            .as_mut()
            .unwrap()
            .transition_kind = super::VideoClipTakeKind::Crossfade;
        assert!(
            super::validate_video_clip_runtime_snapshot(&invalid_pending_timing)
                .unwrap_err()
                .contains("pending transition timing")
        );

        let transition_runtime = super::VideoClipRuntimeSnapshot {
            layers: vec![super::VideoClipLayerRuntimeSummary {
                layer_id: 1,
                active_slot_id: Some(super::VideoClipSlotId(20)),
                queued_slot_id: Some(super::VideoClipSlotId(21)),
                transition: Some(super::VideoClipTakeTransitionSummary {
                    origin_slot_id: super::VideoClipSlotId(20),
                    outgoing_slot_id: super::VideoClipSlotId(20),
                    incoming_slot_id: super::VideoClipSlotId(21),
                    kind: super::VideoClipTakeKind::Crossfade,
                    elapsed_ms: 250,
                    duration_ms: 1_000,
                    duration: super::VideoClipTakeDuration::milliseconds(1_000),
                    progress_millis: 250,
                    incoming_playhead_ms: 0,
                    incoming_playing: true,
                }),
                playhead_ms: 400,
                playing: true,
                ..super::VideoClipLayerRuntimeSummary::default()
            }],
            ..super::VideoClipRuntimeSnapshot::default()
        };
        super::validate_video_clip_runtime_snapshot(&transition_runtime).unwrap();
        super::validate_video_clip_runtime_against_authored_slots(&transition_runtime, &video)
            .unwrap();
        let mut invalid_transition = transition_runtime.clone();
        invalid_transition.layers[0]
            .transition
            .as_mut()
            .unwrap()
            .progress_millis = 1001;
        assert!(super::validate_video_clip_runtime_snapshot(&invalid_transition).is_err());
        let mut inconsistent_transition = transition_runtime.clone();
        inconsistent_transition.layers[0]
            .transition
            .as_mut()
            .unwrap()
            .progress_millis = 251;
        assert!(super::validate_video_clip_runtime_snapshot(&inconsistent_transition).is_err());

        let invalid_nan: Result<super::VideoClipRuntimeSnapshot, _> =
            serde_json::from_value(serde_json::json!({
                "layers": [{ "layer_id": 1, "playhead_ms": "NaN" }]
            }));
        assert!(invalid_nan.is_err());

        let mut duplicate_layer = runtime.clone();
        duplicate_layer.layers.push(runtime.layers[0].clone());
        assert!(super::validate_video_clip_runtime_snapshot(&duplicate_layer).is_err());

        for inactive_transport in [
            super::VideoClipLayerRuntimeSummary {
                layer_id: 1,
                playing: true,
                ..super::VideoClipLayerRuntimeSummary::default()
            },
            super::VideoClipLayerRuntimeSummary {
                layer_id: 1,
                playhead_ms: 1,
                ..super::VideoClipLayerRuntimeSummary::default()
            },
            super::VideoClipLayerRuntimeSummary {
                layer_id: 1,
                ping_pong_reverse: true,
                ..super::VideoClipLayerRuntimeSummary::default()
            },
        ] {
            let invalid = super::VideoClipRuntimeSnapshot {
                layers: vec![inactive_transport],
                ..super::VideoClipRuntimeSnapshot::default()
            };
            assert!(super::validate_video_clip_runtime_snapshot(&invalid).is_err());
        }

        let inactive_queued_only = super::VideoClipRuntimeSnapshot {
            layers: vec![super::VideoClipLayerRuntimeSummary {
                layer_id: 1,
                queued_slot_id: Some(super::VideoClipSlotId(21)),
                ..super::VideoClipLayerRuntimeSummary::default()
            }],
            ..super::VideoClipRuntimeSnapshot::default()
        };
        super::validate_video_clip_runtime_snapshot(&inactive_queued_only).unwrap();
        super::validate_video_clip_runtime_against_authored_slots(&inactive_queued_only, &video)
            .unwrap();

        let mut pending_without_queue = runtime.clone();
        pending_without_queue.layers[0].queued_slot_id = None;
        assert!(super::validate_video_clip_runtime_snapshot(&pending_without_queue).is_err());

        let mut queued_only = runtime.clone();
        queued_only.layers[0].pending_launch = None;
        super::validate_video_clip_runtime_snapshot(&queued_only).unwrap();
        super::validate_video_clip_runtime_against_authored_slots(&queued_only, &video).unwrap();

        let mut immediate_pending = runtime.clone();
        immediate_pending.layers[0]
            .pending_launch
            .as_mut()
            .unwrap()
            .quantization = super::VideoClipLaunchQuantization::Immediate;
        assert!(super::validate_video_clip_runtime_snapshot(&immediate_pending).is_err());

        let mut non_bar_boundary = runtime.clone();
        non_bar_boundary.layers[0]
            .pending_launch
            .as_mut()
            .unwrap()
            .target_boundary_ordinal = 17;
        assert!(super::validate_video_clip_runtime_snapshot(&non_bar_boundary).is_err());

        let mut non_ping_pong_authored = video.clone();
        non_ping_pong_authored.layers[0].clip_slots[0].loop_mode = super::VideoClipLoopMode::Loop;
        assert!(super::validate_video_clip_runtime_against_authored_slots(
            &runtime,
            &non_ping_pong_authored
        )
        .is_err());

        let mut dangling_pending = runtime.clone();
        dangling_pending.layers[0].queued_slot_id = Some(super::VideoClipSlotId(99));
        dangling_pending.layers[0]
            .pending_launch
            .as_mut()
            .unwrap()
            .slot_id = super::VideoClipSlotId(99);
        super::validate_video_clip_runtime_snapshot(&dangling_pending).unwrap();
        assert!(super::validate_video_clip_runtime_against_authored_slots(
            &dangling_pending,
            &video
        )
        .is_err());

        let mut invalid_playhead = runtime;
        invalid_playhead.layers[0].playhead_ms = u64::MAX;
        let mut bounded_video = video.clone();
        bounded_video.layers[0].clip_slots[0].out_point_ms = Some(500);
        assert!(super::validate_video_clip_runtime_against_authored_slots(
            &invalid_playhead,
            &bounded_video
        )
        .is_err());
    }

    fn clip_slot_test_effect() -> super::VideoIsfEffectSummary {
        super::VideoIsfEffectSummary {
            enabled: true,
            label: "Color".to_string(),
            source: std::sync::Arc::from("/* test */"),
            source_path: None,
            description: None,
            categories: vec!["Test".to_string()],
            controls: vec![super::VideoIsfControlSummary {
                name: "amount".to_string(),
                kind: super::VideoIsfControlKind::Float,
                value: [0.5, 0.0, 0.0, 0.0],
                default: [0.0, 0.0, 0.0, 0.0],
                minimum: [0.0, 0.0, 0.0, 0.0],
                maximum: [1.0, 0.0, 0.0, 0.0],
                labels: Vec::new(),
                values: Vec::new(),
            }],
            stack: Vec::new(),
        }
    }

    #[test]
    fn video_clip_slot_migration_creates_one_default_without_asset_duplication() {
        let mut video = super::VideoSnapshot {
            layers: vec![media_asset_test_layer(
                4,
                "Legacy Cut",
                "C:/show/legacy.mp4",
            )],
            compositions: vec![super::CompositionSummary {
                id: 19,
                label: "Program".to_string(),
                layer_ids: vec![4],
                output_ids: Vec::new(),
            }],
            ..super::VideoSnapshot::default()
        };
        video.layers[0].isf_effect = Some(clip_slot_test_effect());
        let legacy = video.layers[0].clone();

        let asset_report = super::normalize_legacy_video_media_assets(&mut video).unwrap();
        assert_eq!(asset_report.created_asset_ids, vec![5]);
        let asset_count = video.media_assets.len();
        let report = super::normalize_legacy_video_clip_slots(&mut video).unwrap();

        assert_eq!(report.created_slot_ids, vec![super::VideoClipSlotId(6)]);
        assert_eq!(report.migrated_layer_ids, vec![4]);
        assert_eq!(video.media_assets.len(), asset_count);
        let layer = &video.layers[0];
        assert_eq!(layer.source, legacy.source);
        assert_eq!(layer.blend_mode, legacy.blend_mode);
        assert_eq!(layer.state, legacy.state);
        assert_eq!(layer.isf_effect, legacy.isf_effect);
        assert_eq!(layer.media_asset_id, Some(5));
        assert_eq!(layer.default_clip_slot_id, Some(super::VideoClipSlotId(6)));
        assert_eq!(layer.clip_slots.len(), 1);
        assert_eq!(layer.clip_slots[0].media_asset_id, 5);
        assert_eq!(layer.clip_slots[0].in_point_ms, 100);
        assert_eq!(layer.clip_slots[0].out_point_ms, Some(900));
        assert_eq!(
            layer.clip_slots[0].loop_mode,
            super::VideoClipLoopMode::Loop
        );
        assert_eq!(layer.clip_slots[0].speed, 1.25);
        assert_eq!(video.compositions[0].layer_ids, vec![4]);
        super::validate_engine_ready_video_clip_slots(&video).unwrap();
    }

    #[test]
    fn video_clip_slot_migration_is_atomic_idempotent_and_mixed_deterministic() {
        let mut first = media_asset_test_layer(1, "Existing", "C:/show/existing.mp4");
        first.media_asset_id = Some(10);
        first.clip_slots = vec![clip_slot_test_slot(100, 10)];
        first.default_clip_slot_id = Some(super::VideoClipSlotId(100));
        let mut second = media_asset_test_layer(2, "Legacy", "C:/show/legacy.mp4");
        second.media_asset_id = Some(11);
        let mut video = super::VideoSnapshot {
            layers: vec![first, second],
            media_assets: vec![
                media_asset_test_asset(10, "Existing", "C:/show/existing.mp4"),
                media_asset_test_asset(11, "Legacy", "C:/show/legacy.mp4"),
            ],
            ..super::VideoSnapshot::default()
        };

        let report = super::normalize_legacy_video_clip_slots(&mut video).unwrap();
        assert_eq!(report.created_slot_ids, vec![super::VideoClipSlotId(101)]);
        assert_eq!(report.migrated_layer_ids, vec![2]);
        assert_eq!(
            video.layers[0].clip_slots[0].id,
            super::VideoClipSlotId(100)
        );
        assert_eq!(
            video.layers[1].default_clip_slot_id,
            Some(super::VideoClipSlotId(101))
        );
        let once = serde_json::to_vec(&video).unwrap();
        let second_report = super::normalize_legacy_video_clip_slots(&mut video).unwrap();
        assert_eq!(
            second_report,
            super::VideoClipSlotMigrationReport::default()
        );
        assert_eq!(serde_json::to_vec(&video).unwrap(), once);

        let mut invalid = video.clone();
        invalid.layers[1].clip_slots.clear();
        invalid.layers[1].default_clip_slot_id = None;
        invalid.layers[1].media_asset_id = Some(99);
        let before = invalid.clone();
        assert!(super::normalize_legacy_video_clip_slots(&mut invalid).is_err());
        assert_eq!(invalid, before);
    }

    #[test]
    fn video_clip_slot_migration_preserves_legacy_transport_semantics_across_json() {
        let mut layer = media_asset_test_layer(4, "Legacy Transport", "C:/show/transport.mp4");
        layer.media_asset_id = Some(10);
        layer.state.speed = -1.5;
        layer.state.loop_enabled = true;
        layer.state.loop_start_ms = 120;
        layer.state.loop_end_ms = 840;
        layer.state.position_ms = 777;
        layer.state.playing = true;
        layer.state.cue_points = vec![
            super::VideoCuePointSummary {
                position_ms: 200,
                label: " Intro ".to_string(),
                color: Some("#00ff00".to_string()),
            },
            super::VideoCuePointSummary {
                position_ms: 600,
                label: "Break".to_string(),
                color: None,
            },
        ];
        layer.state.cue_points_ms = vec![200, 600];
        let legacy_state = layer.state.clone();
        let mut video = super::VideoSnapshot {
            layers: vec![layer],
            media_assets: vec![media_asset_test_asset(
                10,
                "Transport",
                "C:/show/transport.mp4",
            )],
            ..super::VideoSnapshot::default()
        };

        super::normalize_legacy_video_clip_slots(&mut video).unwrap();
        let migrated = &video.layers[0];
        assert_eq!(migrated.state, legacy_state);
        let slot = &migrated.clip_slots[0];
        assert_eq!(slot.speed, -1.5);
        assert_eq!(slot.loop_mode, super::VideoClipLoopMode::Loop);
        assert_eq!(slot.in_point_ms, 120);
        assert_eq!(slot.out_point_ms, Some(840));
        assert_eq!(
            slot.cue_points,
            vec![
                super::VideoClipCuePointSummary {
                    position_ms: 200,
                    name: "Intro".to_string(),
                },
                super::VideoClipCuePointSummary {
                    position_ms: 600,
                    name: "Break".to_string(),
                },
            ]
        );
        let encoded = serde_json::to_vec(&video).unwrap();
        let decoded: super::VideoSnapshot = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, video);

        let mut numeric_only = decoded.clone();
        numeric_only.layers[0].clip_slots.clear();
        numeric_only.layers[0].default_clip_slot_id = None;
        numeric_only.layers[0].state.loop_enabled = false;
        numeric_only.layers[0].state.cue_points.clear();
        numeric_only.layers[0].state.cue_points_ms = vec![1, 2];
        super::normalize_legacy_video_clip_slots(&mut numeric_only).unwrap();
        assert_eq!(
            numeric_only.layers[0].clip_slots[0].loop_mode,
            super::VideoClipLoopMode::Once
        );
        assert_eq!(numeric_only.layers[0].clip_slots[0].in_point_ms, 0);
        assert_eq!(numeric_only.layers[0].clip_slots[0].out_point_ms, None);
        assert_eq!(
            numeric_only.layers[0].clip_slots[0].cue_points,
            vec![
                super::VideoClipCuePointSummary {
                    position_ms: 1,
                    name: "Cue 1".to_string(),
                },
                super::VideoClipCuePointSummary {
                    position_ms: 2,
                    name: "Cue 2".to_string(),
                },
            ]
        );
    }

    #[test]
    fn video_clip_slot_migration_rejects_invalid_legacy_transport_atomically() {
        let base = || {
            let mut layer = media_asset_test_layer(1, "Legacy", "C:/show/a.mp4");
            layer.media_asset_id = Some(10);
            super::VideoSnapshot {
                layers: vec![layer],
                media_assets: vec![media_asset_test_asset(10, "A", "C:/show/a.mp4")],
                ..super::VideoSnapshot::default()
            }
        };
        let mut cases = Vec::new();

        let mut invalid_loop = base();
        invalid_loop.layers[0].state.loop_enabled = true;
        invalid_loop.layers[0].state.loop_start_ms = 100;
        invalid_loop.layers[0].state.loop_end_ms = 100;
        cases.push(invalid_loop);

        let mut zero_loop_end = base();
        zero_loop_end.layers[0].state.loop_enabled = true;
        zero_loop_end.layers[0].state.loop_start_ms = 0;
        zero_loop_end.layers[0].state.loop_end_ms = 0;
        cases.push(zero_loop_end);

        let mut invalid_speed = base();
        invalid_speed.layers[0].state.speed = f32::INFINITY;
        cases.push(invalid_speed);

        let mut disagreeing_cues = base();
        disagreeing_cues.layers[0].state.cue_points = vec![super::VideoCuePointSummary {
            position_ms: 10,
            label: "Ten".to_string(),
            color: None,
        }];
        disagreeing_cues.layers[0].state.cue_points_ms = vec![11];
        cases.push(disagreeing_cues);

        let mut invalid_named_cue = base();
        invalid_named_cue.layers[0].state.cue_points = vec![super::VideoCuePointSummary {
            position_ms: 10,
            label: "   ".to_string(),
            color: None,
        }];
        cases.push(invalid_named_cue);

        let mut duplicate_trimmed_names = base();
        duplicate_trimmed_names.layers[0].state.cue_points = vec![
            super::VideoCuePointSummary {
                position_ms: 10,
                label: "Cue".to_string(),
                color: None,
            },
            super::VideoCuePointSummary {
                position_ms: 20,
                label: " Cue ".to_string(),
                color: None,
            },
        ];
        cases.push(duplicate_trimmed_names);

        for mut invalid in cases {
            let before = invalid.clone();
            assert!(super::normalize_legacy_video_clip_slots(&mut invalid).is_err());
            assert_eq!(invalid, before);
        }
    }

    #[test]
    fn video_clip_slot_validation_rejects_identity_ranges_and_invalid_local_data() {
        let mut layer = media_asset_test_layer(1, "Layer", "C:/show/a.mp4");
        layer.media_asset_id = Some(10);
        layer.clip_slots = vec![clip_slot_test_slot(20, 10)];
        layer.default_clip_slot_id = Some(super::VideoClipSlotId(20));
        let video = super::VideoSnapshot {
            layers: vec![layer],
            media_assets: vec![media_asset_test_asset(10, "A", "C:/show/a.mp4")],
            ..super::VideoSnapshot::default()
        };
        super::validate_engine_ready_video_clip_slots(&video).unwrap();

        let mut cases = Vec::new();
        let mut zero = video.clone();
        zero.layers[0].clip_slots[0].id = super::VideoClipSlotId(0);
        cases.push(zero);

        let mut duplicate = video.clone();
        duplicate.layers[0]
            .clip_slots
            .push(clip_slot_test_slot(20, 10));
        cases.push(duplicate);

        let mut dangling = video.clone();
        dangling.layers[0].clip_slots[0].media_asset_id = 99;
        cases.push(dangling);

        let mut default_projection_mismatch = video.clone();
        default_projection_mismatch
            .media_assets
            .push(media_asset_test_asset(11, "B", "C:/show/b.mp4"));
        default_projection_mismatch.layers[0].clip_slots[0].media_asset_id = 11;
        cases.push(default_projection_mismatch);

        let mut wrong_default = video.clone();
        wrong_default.layers[0].default_clip_slot_id = Some(super::VideoClipSlotId(21));
        cases.push(wrong_default);

        let mut bad_range = video.clone();
        bad_range.layers[0].clip_slots[0].in_point_ms = 50;
        bad_range.layers[0].clip_slots[0].out_point_ms = Some(50);
        cases.push(bad_range);

        let mut bad_speed = video.clone();
        bad_speed.layers[0].clip_slots[0].speed = f32::NAN;
        cases.push(bad_speed);

        let mut duplicate_cue_name = video.clone();
        duplicate_cue_name.layers[0].clip_slots[0].cue_points = vec![
            super::VideoClipCuePointSummary {
                position_ms: 1,
                name: "hit".to_string(),
            },
            super::VideoClipCuePointSummary {
                position_ms: 2,
                name: "hit".to_string(),
            },
        ];
        cases.push(duplicate_cue_name);

        let mut invalid_cue_range = video.clone();
        invalid_cue_range.layers[0].clip_slots[0].out_point_ms = Some(10);
        invalid_cue_range.layers[0].clip_slots[0].cue_points =
            vec![super::VideoClipCuePointSummary {
                position_ms: 10,
                name: "past-end".to_string(),
            }];
        cases.push(invalid_cue_range);

        let mut missing_control = video.clone();
        missing_control.layers[0].clip_slots[0].effect_overrides =
            vec![super::VideoClipEffectOverrideSummary {
                effect_id: None,
                stage_index: 0,
                control_name: "missing".to_string(),
                value: [0.0; 4],
            }];
        cases.push(missing_control);

        for invalid in cases {
            assert!(super::validate_engine_ready_video_clip_slots(&invalid).is_err());
        }

        let mut override_ok = video;
        override_ok.layers[0].isf_effect = Some(clip_slot_test_effect());
        override_ok.layers[0].clip_slots[0].effect_overrides =
            vec![super::VideoClipEffectOverrideSummary {
                effect_id: None,
                stage_index: 0,
                control_name: "amount".to_string(),
                value: [0.75, 0.0, 0.0, 0.0],
            }];
        super::validate_engine_ready_video_clip_slots(&override_ok).unwrap();
    }

    #[test]
    fn video_clip_slot_effect_overrides_are_stage_exact_and_canonical() {
        let mut layer = media_asset_test_layer(1, "Layer", "C:/show/a.mp4");
        layer.media_asset_id = Some(10);
        layer.clip_slots = vec![clip_slot_test_slot(20, 10)];
        layer.default_clip_slot_id = Some(super::VideoClipSlotId(20));
        let mut effect = clip_slot_test_effect();
        effect.stack.push(super::VideoIsfEffectStageSummary {
            enabled: true,
            label: "Stack Color".to_string(),
            source: std::sync::Arc::from("/* stack test */"),
            source_path: None,
            description: None,
            categories: Vec::new(),
            controls: effect.controls.clone(),
        });
        layer.isf_effect = Some(effect);
        let video = super::VideoSnapshot {
            layers: vec![layer],
            media_assets: vec![media_asset_test_asset(10, "A", "C:/show/a.mp4")],
            ..super::VideoSnapshot::default()
        };

        assert!(super::layer_has_isf_control(&video.layers[0], 0, "amount"));
        assert!(super::layer_has_isf_control(&video.layers[0], 1, "amount"));
        assert!(!super::layer_has_isf_control(&video.layers[0], 2, "amount"));

        let mut exact = video.clone();
        exact.layers[0].clip_slots[0].effect_overrides = vec![
            super::VideoClipEffectOverrideSummary {
                effect_id: None,
                stage_index: 0,
                control_name: "amount".to_string(),
                value: [0.25, 0.0, 0.0, 0.0],
            },
            super::VideoClipEffectOverrideSummary {
                effect_id: None,
                stage_index: 1,
                control_name: "amount".to_string(),
                value: [0.75, 0.0, 0.0, 0.0],
            },
        ];
        super::validate_engine_ready_video_clip_slots(&exact).unwrap();

        let mut out_of_range_stage = exact.clone();
        out_of_range_stage.layers[0].clip_slots[0].effect_overrides[1].stage_index = 2;
        assert!(super::validate_engine_ready_video_clip_slots(&out_of_range_stage).is_err());

        let mut whitespace_name = exact.clone();
        whitespace_name.layers[0].clip_slots[0].effect_overrides[0].control_name =
            " amount ".to_string();
        assert!(super::validate_engine_ready_video_clip_slots(&whitespace_name).is_err());

        let mut duplicate_identity = exact;
        duplicate_identity.layers[0].clip_slots[0]
            .effect_overrides
            .push(super::VideoClipEffectOverrideSummary {
                effect_id: None,
                stage_index: 1,
                control_name: "amount".to_string(),
                value: [0.5, 0.0, 0.0, 0.0],
            });
        assert!(super::validate_engine_ready_video_clip_slots(&duplicate_identity).is_err());

        let legacy: super::VideoClipEffectOverrideSummary =
            serde_json::from_value(serde_json::json!({
                "control_name": "amount",
                "value": [0.5, 0.0, 0.0, 0.0]
            }))
            .unwrap();
        assert_eq!(legacy.stage_index, 0);
    }

    #[test]
    fn video_clip_slot_migration_rejects_allocator_overflow_without_mutation() {
        let mut layer = media_asset_test_layer(1, "Maximum", "C:/show/maximum.mp4");
        layer.media_asset_id = Some(u64::MAX);
        let mut video = super::VideoSnapshot {
            layers: vec![layer],
            media_assets: vec![media_asset_test_asset(
                u64::MAX,
                "Maximum",
                "C:/show/maximum.mp4",
            )],
            ..super::VideoSnapshot::default()
        };
        let before = video.clone();
        assert!(super::normalize_legacy_video_clip_slots(&mut video).is_err());
        assert_eq!(video, before);
    }

    #[test]
    fn video_clip_slot_migration_requires_existing_t1_asset_without_creating_one() {
        let mut video = super::VideoSnapshot {
            layers: vec![media_asset_test_layer(1, "Pre-T1", "C:/show/pre-t1.mp4")],
            ..super::VideoSnapshot::default()
        };
        let before = video.clone();
        assert!(super::normalize_legacy_video_clip_slots(&mut video).is_err());
        assert_eq!(video, before);
    }

    #[test]
    fn media_asset_catalog_validates_hash_pairing_and_live_identity() {
        let source = media_asset_test_source("C:/show/verified.mp4");
        let asset = super::MediaAssetSummary {
            id: 17,
            label: "Verified".to_string(),
            source: source.clone(),
            content_hash: Some(super::MediaContentHash {
                algorithm: super::MediaHashAlgorithm::Sha256,
                hex: "a".repeat(64),
            }),
            byte_size: Some(3),
        };
        let mut layer = media_asset_test_layer(3, "Verified layer", "C:/show/verified.mp4");
        layer.media_asset_id = Some(17);
        let video = super::VideoSnapshot {
            layers: vec![layer],
            media_assets: vec![asset.clone()],
            ..super::VideoSnapshot::default()
        };
        super::validate_engine_ready_video_media_assets(&video).unwrap();

        let mut upper = video.clone();
        upper.media_assets[0].content_hash.as_mut().unwrap().hex = "A".repeat(64);
        assert!(super::validate_media_asset_catalog(&upper).is_err());

        let mut partial = video.clone();
        partial.media_assets[0].byte_size = None;
        assert!(super::validate_media_asset_catalog(&partial).is_err());

        // Orphan library entries are legal, so their source metadata must be
        // validated independently of whether a layer currently references
        // them.
        let mut malformed_orphan = video.clone();
        malformed_orphan.layers.clear();
        malformed_orphan.media_assets[0].source.metadata = Some(super::VideoMediaMetadata {
            duration_ms: Some(1_000),
            width: Some(0),
            height: Some(1080),
            frame_rate: Some(30.0),
            has_audio: false,
        });
        assert!(super::validate_media_asset_catalog(&malformed_orphan).is_err());

        malformed_orphan.media_assets[0].source.metadata = Some(super::VideoMediaMetadata {
            duration_ms: Some(1_000),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(f32::NAN),
            has_audio: false,
        });
        assert!(super::validate_media_asset_catalog(&malformed_orphan).is_err());

        let mut live = asset;
        live.source = super::VideoSourceSummary {
            kind: super::VideoSourceKind::Ndi,
            path: None,
            name: Some("Stage feed".to_string()),
            codec: None,
            metadata: None,
        };
        let mut live_layer = media_asset_test_layer(4, "Live", "C:/unused.mp4");
        live_layer.source = live.source.clone();
        live_layer.media_asset_id = Some(live.id);
        let live_video = super::VideoSnapshot {
            layers: vec![live_layer],
            media_assets: vec![live],
            ..super::VideoSnapshot::default()
        };
        assert!(super::validate_engine_ready_video_media_assets(&live_video).is_err());
    }

    #[test]
    fn media_asset_availability_and_relink_outcomes_are_typed_machine_local_dtos() {
        let availability = super::MediaAssetAvailability::HashMismatch {
            asset_id: 4,
            expected: super::MediaContentHash {
                algorithm: super::MediaHashAlgorithm::Sha256,
                hex: "a".repeat(64),
            },
            actual: super::MediaContentHash {
                algorithm: super::MediaHashAlgorithm::Sha256,
                hex: "b".repeat(64),
            },
        };
        let encoded = serde_json::to_value(&availability).unwrap();
        assert_eq!(encoded["kind"], "hash_mismatch");
        assert_eq!(
            serde_json::from_value::<super::MediaAssetAvailability>(encoded).unwrap(),
            availability
        );
        assert!(matches!(
            super::MediaAssetRelinkOutcome::NeedsExplicitAdoption { asset_id: 4 },
            super::MediaAssetRelinkOutcome::NeedsExplicitAdoption { .. }
        ));
    }

    #[test]
    fn legacy_auto_vj_defaults_to_clock_without_live_audio_runtime_state() {
        let mut value = serde_json::to_value(super::AutoVjSnapshot::default()).unwrap();
        value["config"]
            .as_object_mut()
            .unwrap()
            .remove("rhythm_source");
        value["status"]
            .as_object_mut()
            .unwrap()
            .remove("live_audio_beat_counter");
        value["status"]
            .as_object_mut()
            .unwrap()
            .remove("last_live_audio_feature_sequence");
        value["status"]
            .as_object_mut()
            .unwrap()
            .remove("live_audio_waiting");
        value["status"]
            .as_object_mut()
            .unwrap()
            .remove("live_audio_generation");

        let parsed: super::AutoVjSnapshot = serde_json::from_value(value).unwrap();

        assert_eq!(
            parsed.config.rhythm_source,
            super::AutoVjRhythmSource::Clock
        );
        assert_eq!(parsed.status.live_audio_beat_counter, 0);
        assert_eq!(parsed.status.last_live_audio_feature_sequence, None);
        assert!(!parsed.status.live_audio_waiting);
        assert_eq!(parsed.status.live_audio_generation, None);
    }

    #[test]
    fn live_audio_frame_round_trips_as_one_fixed_capacity_payload() {
        let frame = super::LiveAudioFrame {
            generation: 7,
            feature_sequence: 13,
            spectrum: super::AudioSpectrumPoint {
                time_ms: 41,
                bass: 0.25,
                mid: 0.5,
                high: 0.75,
            },
            features: super::LiveAudioReactiveFeatures {
                band_count: 16,
                bands: std::array::from_fn(|index| index as f32 / 15.0),
                rms: 0.4,
                peak: 0.8,
                spectral_flux: 0.3,
                spectral_centroid: 0.55,
                spectral_density_fast: 0.6,
                spectral_density_slow: 0.45,
                kick_strength: 0.9,
                snare_strength: 0.2,
                kick_event: true,
                snare_event: false,
                onset: true,
                onset_strength: 0.7,
                bpm: Some(128.0),
                bpm_confidence: 0.85,
                beat_phase: 0.25,
            },
            onset_feature_sequences: [8, 10, 13, 0],
            onset_count: 3,
        };

        let encoded = serde_json::to_value(&frame).unwrap();
        let decoded: super::LiveAudioFrame = serde_json::from_value(encoded).unwrap();

        assert_eq!(decoded, frame);

        let mut legacy = serde_json::to_value(&frame).unwrap();
        legacy.as_object_mut().unwrap().remove("features");
        let legacy: super::LiveAudioFrame = serde_json::from_value(legacy).unwrap();
        assert_eq!(legacy.features, super::LiveAudioReactiveFeatures::default());
    }

    #[test]
    fn legacy_cue_part_defaults_video_assignments_to_empty() {
        let mut value = serde_json::to_value(super::CuePartSummary {
            number: 1,
            label: "Lighting".to_string(),
            delay_ms: 100,
            fade_ms: Some(250),
            fixture_ids: vec![7],
            video_layer_ids: vec![3],
            video_output_ids: vec![4],
        })
        .unwrap();
        value.as_object_mut().unwrap().remove("video_layer_ids");
        value.as_object_mut().unwrap().remove("video_output_ids");

        let parsed: super::CuePartSummary = serde_json::from_value(value).unwrap();

        assert_eq!(parsed.fixture_ids, vec![7]);
        assert!(parsed.video_layer_ids.is_empty());
        assert!(parsed.video_output_ids.is_empty());
    }

    #[test]
    fn legacy_video_output_mapping_defaults_bitmap_mask_to_disabled() {
        let mapping = super::VideoOutputMapping::default();
        let mut value = serde_json::to_value(mapping).unwrap();
        value.as_object_mut().unwrap().remove("bitmap_mask_width");
        value.as_object_mut().unwrap().remove("bitmap_mask_height");
        value
            .as_object_mut()
            .unwrap()
            .remove("bitmap_mask_luma_words");

        let parsed: super::VideoOutputMapping = serde_json::from_value(value).unwrap();

        assert_eq!(parsed.bitmap_mask_width, 0);
        assert_eq!(parsed.bitmap_mask_height, 0);
        assert_eq!(parsed.bitmap_mask_luma_words, [0; 32]);
    }

    #[test]
    fn legacy_effect_summary_defaults_color_body_to_none() {
        let mut value = serde_json::json!({
            "id": 7,
            "label": "Legacy LFO",
            "effect_type": "Lfo",
            "fixture_ids": [1],
            "target_group_ids": [],
            "attribute": "Dimmer",
            "video_targets": [],
            "shape": "Sine",
            "period_ms": 1000,
            "clock_sync": null,
            "low": 0,
            "high": 65535,
            "phase": 0.0,
            "blend_mode": "Override",
            "origin": null,
            "direction": null,
            "speed": null,
            "wavelength": null,
            "enabled": true
        });
        value.as_object_mut().unwrap().remove("color");

        let parsed: super::EffectSummary = serde_json::from_value(value).unwrap();

        assert_eq!(parsed.effect_type, super::EffectKind::Lfo);
        assert_eq!(parsed.fixture_spread, 0.0);
        assert!(parsed.color.is_none());
        assert!(parsed.chaser.is_none());
        assert!(parsed.move_effect.is_none());
        assert!(parsed.value.is_none());
        assert!(parsed.curve.is_none());
        assert!(parsed.mapping.is_none());
        assert!(parsed.color_mapping.is_none());
        assert!(serde_json::to_value(&parsed)
            .unwrap()
            .get("fixture_spread")
            .is_none());
        let serialized = serde_json::to_string(&parsed).unwrap();
        assert!(!serialized.contains("\"mapping\""));
        assert!(!serialized.contains("\"color_mapping\""));
    }

    #[test]
    fn curve_effect_request_and_preset_roundtrip_independent_body() {
        let request = super::CurveEffectRequest {
            label: "Dimmer channel curve".to_string(),
            fixture_ids: vec![1, 2],
            target_group_ids: vec!["Front".to_string()],
            attribute: "Dimmer".to_string(),
            features: Vec::new(),
            points: vec![
                super::CurveEffectPoint {
                    position: 0.0,
                    value: 0.0,
                    in_tangent: 0.0,
                    out_tangent: 1.5,
                },
                super::CurveEffectPoint {
                    position: 1.0,
                    value: 1.0,
                    in_tangent: 0.25,
                    out_tangent: 0.0,
                },
            ],
            mode: super::ValueEffectMode::Absolute,
            direction: super::ValueEffectDirection::Forward,
            period_ms: 2_000,
            clock_sync: Some(super::EffectClockSync { beats: 4.0 }),
            low: 1_000,
            high: 60_000,
            phase: 0.125,
            fixture_spread: 0.5,
            blend_mode: super::EffectBlendMode::Override,
        };
        let preset = super::EffectPreset {
            version: 1,
            effect_type: super::EffectKind::Curve,
            enabled: true,
            lfo: None,
            position_wave: None,
            color: None,
            chaser: None,
            move_effect: None,
            value: None,
            curve: Some(request),
            mapping: None,
            color_mapping: None,
        };

        let json = serde_json::to_string(&preset).unwrap();
        assert!(json.contains("\"effect_type\":\"Curve\""));
        assert!(json.contains("\"in_tangent\":0.25"));
        let parsed: super::EffectPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, preset);
    }

    #[test]
    fn mapping_effect_request_and_preset_roundtrip_independent_body() {
        let request = super::MappingEffectRequest {
            label: "Front order wave".to_string(),
            fixture_ids: vec![3, 1, 2],
            target_group_ids: vec!["Front".to_string()],
            attribute: "Dimmer".to_string(),
            features: Vec::new(),
            shape: super::LfoShape::Sine,
            mode: super::ValueEffectMode::Absolute,
            direction: super::MappingEffectDirection::Bounce,
            period_ms: 2_000,
            clock_sync: Some(super::EffectClockSync { beats: 4.0 }),
            low: 1_000,
            high: 60_000,
            phase: 0.125,
            fixture_spread: 1.0,
            repetitions: 2.0,
            blend_mode: super::EffectBlendMode::Override,
        };
        let preset = super::EffectPreset {
            version: 1,
            effect_type: super::EffectKind::Mapping,
            enabled: true,
            lfo: None,
            position_wave: None,
            color: None,
            chaser: None,
            move_effect: None,
            value: None,
            curve: None,
            mapping: Some(request),
            color_mapping: None,
        };

        let json = serde_json::to_string(&preset).unwrap();
        assert!(json.contains("\"effect_type\":\"Mapping\""));
        assert!(json.contains("\"fixture_ids\":[3,1,2]"));
        let parsed: super::EffectPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, preset);
    }

    #[test]
    fn color_mapping_effect_request_and_preset_roundtrip_independent_body() {
        let request = super::ColorMappingEffectRequest {
            label: "Matrix clip".to_string(),
            fixture_ids: vec![3, 1, 2],
            target_group_ids: vec!["Matrix".to_string()],
            source_kind: super::ColorMappingSourceKind::Video,
            width: 2,
            height: 1,
            frames: vec![
                super::ColorMappingFrame {
                    pixels: vec![0xffff_0000_0000, 0x0000_ffff_0000],
                },
                super::ColorMappingFrame {
                    pixels: vec![0x0000_0000_ffff, 0xffff_ffff_ffff],
                },
            ],
            cells: vec![super::ColorMappingCellTarget {
                fixture_id: 3,
                beam_index: 0,
                selection_index: 0,
                u: 0.25,
                v: 0.5,
                feature_attribute: None,
                feature_low: None,
                feature_high: None,
            }],
            playback_direction: super::ColorMappingPlaybackDirection::Bounce,
            period_ms: 2_000,
            clock_sync: Some(super::EffectClockSync { beats: 4.0 }),
            phase: 0.125,
            offset_u: 0.1,
            offset_v: -0.1,
            scale_u: 1.5,
            scale_v: 0.75,
            rotation_degrees: 30.0,
            wrap_mode: super::ColorMappingWrapMode::Repeat,
            sampling: super::ColorMappingSampling::Bilinear,
            blend_mode: super::EffectBlendMode::Override,
        };
        let preset = super::EffectPreset {
            version: 1,
            effect_type: super::EffectKind::ColorMapping,
            enabled: true,
            lfo: None,
            position_wave: None,
            color: None,
            chaser: None,
            move_effect: None,
            value: None,
            curve: None,
            mapping: None,
            color_mapping: Some(request),
        };

        let json = serde_json::to_string(&preset).unwrap();
        assert!(json.contains("\"effect_type\":\"ColorMapping\""));
        assert!(json.contains("281470681743360"));
        let parsed: super::EffectPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, preset);
        assert!(!json.contains("feature_low"));
        assert!(!json.contains("feature_high"));

        let legacy_cell: super::ColorMappingCellTarget = serde_json::from_str(
            r#"{"fixture_id":9,"beam_index":0,"selection_index":0,"u":0.5,"v":0.5,"feature_attribute":"Dimmer"}"#,
        )
        .unwrap();
        assert_eq!(legacy_cell.feature_low, None);
        assert_eq!(legacy_cell.feature_high, None);
    }

    #[test]
    fn color_effect_request_and_preset_roundtrip_rgb16_stops() {
        let request = super::ColorEffectRequest {
            label: "Rainbow".to_string(),
            fixture_ids: vec![1, 2],
            target_group_ids: vec!["Front".to_string()],
            stops: vec![
                super::ColorEffectStop {
                    position: 0.0,
                    color: super::ColorEffectColor {
                        red: 65_535,
                        green: 0,
                        blue: 0,
                    },
                },
                super::ColorEffectStop {
                    position: 1.0,
                    color: super::ColorEffectColor {
                        red: 0,
                        green: 0,
                        blue: 65_535,
                    },
                },
            ],
            algorithm: super::ColorEffectAlgorithm::Bounce,
            interpolation: super::ColorEffectInterpolation::HsvShortest,
            period_ms: 2_000,
            clock_sync: Some(super::EffectClockSync { beats: 4.0 }),
            phase: 0.125,
            fixture_spread: 1.0,
            blend_mode: super::EffectBlendMode::Multiply,
            spatial_pattern: None,
        };
        let preset = super::EffectPreset {
            version: 1,
            effect_type: super::EffectKind::Color,
            enabled: false,
            lfo: None,
            position_wave: None,
            color: Some(request),
            chaser: None,
            move_effect: None,
            value: None,
            curve: None,
            mapping: None,
            color_mapping: None,
        };

        let json = serde_json::to_string(&preset).unwrap();
        assert!(!json.contains("spatial_pattern"));
        let parsed: super::EffectPreset = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, preset);
    }

    #[test]
    fn color_effect_spatial_pattern_roundtrips_and_legacy_defaults_to_none() {
        let legacy = serde_json::json!({
            "label": "Legacy color",
            "fixture_ids": [1],
            "target_group_ids": [],
            "stops": [
                {"position": 0.0, "color": {"red": 0, "green": 0, "blue": 0}},
                {"position": 1.0, "color": {"red": 65535, "green": 65535, "blue": 65535}}
            ],
            "algorithm": "Sequence",
            "interpolation": "Rgb",
            "period_ms": 1000,
            "phase": 0.0,
            "fixture_spread": 0.0,
            "blend_mode": "Override"
        });
        let parsed: super::ColorEffectRequest = serde_json::from_value(legacy).unwrap();
        assert!(parsed.spatial_pattern.is_none());

        let mut spatial = parsed;
        spatial.spatial_pattern = Some(Box::new(super::ColorEffectSpatialPattern {
            recipe: super::ColorEffectSpatialRecipe::Perlin {
                grayscale: false,
                vertical_symmetry: false,
                horizontal_symmetry: false,
                rotation_degrees: 0.0,
                octaves: 5,
                zoom: 0.5,
                direction_degrees: 0.0,
                speed: 1.0,
                amplitude: 100.0,
            },
            parameter_model_version: super::COLOR_EFFECT_SPATIAL_PARAMETER_MODEL_VERSION,
            beam_targets: vec![super::ColorEffectBeamTarget {
                fixture_id: 1,
                beam_index: 7,
                selection_index: 12,
                feature_attribute: Some("Dimmer".to_string()),
            }],
            placement: None,
        }));
        let json = serde_json::to_string(&spatial).unwrap();
        let roundtrip: super::ColorEffectRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, spatial);

        let legacy_plasma: super::ColorEffectSpatialRecipe =
            serde_json::from_value(serde_json::json!({
                "Plasma": {
                    "size_x": 1.0,
                    "param_x": 2.0,
                    "size_y": 1.0,
                    "param_y": 2.0,
                    "speed_x": -1.0,
                    "param_sx": 2.0,
                    "speed_y": 1.0,
                    "param_sy": -1.0
                }
            }))
            .unwrap();
        assert!(matches!(
            legacy_plasma,
            super::ColorEffectSpatialRecipe::Plasma {
                grayscale: false,
                vertical_symmetry: false,
                ..
            }
        ));

        let legacy_mapping_rainbow_json = r#"{"Rainbow":{"vertical_symmetry":true,"horizontal_symmetry":false,"rotation_degrees":0.0,"color_width":50.0,"angle_degrees":90.0,"gradient":100.0}}"#;
        let legacy_mapping_rainbow: super::ColorEffectSpatialRecipe =
            serde_json::from_str(legacy_mapping_rainbow_json).unwrap();
        assert!(matches!(
            &legacy_mapping_rainbow,
            super::ColorEffectSpatialRecipe::Rainbow {
                grayscale: false,
                vertical_symmetry: true,
                horizontal_symmetry: false,
                ..
            }
        ));
        assert_eq!(
            serde_json::to_string(&legacy_mapping_rainbow).unwrap(),
            legacy_mapping_rainbow_json,
            "default Grayscale must not change an existing Rainbow byte shape"
        );

        let legacy_grid: super::ColorEffectSpatialRecipe =
            serde_json::from_str(r#"{"Grid":{"size":5,"width":20}}"#).unwrap();
        assert_eq!(
            legacy_grid,
            super::ColorEffectSpatialRecipe::Grid {
                grayscale: false,
                size: 5,
                width: 20,
            }
        );
        assert_eq!(
            serde_json::to_string(&legacy_grid).unwrap(),
            r#"{"Grid":{"size":5,"width":20}}"#,
            "default Grid Grayscale must remain omitted"
        );
        let lines = super::ColorEffectSpatialRecipe::Lines {
            grayscale: true,
            size: 20,
        };
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialRecipe>(
                &serde_json::to_string(&lines).unwrap()
            )
            .unwrap(),
            lines,
            "Lines must round-trip without changing its parameter model"
        );
        let legacy_graph: super::ColorEffectSpatialRecipe = serde_json::from_str(
            r#"{"Graph":{"height":10,"width":10,"pitch":10,"frequency":2,"amplitude":1.0,"offset":0.0}}"#,
        )
        .unwrap();
        assert_eq!(
            legacy_graph,
            super::ColorEffectSpatialRecipe::Graph {
                grayscale: false,
                height: 10,
                width: 10,
                pitch: 10,
                frequency: 2,
                amplitude: 1.0,
                offset: 0.0,
            }
        );
        assert_eq!(
            serde_json::to_string(&legacy_graph).unwrap(),
            r#"{"Graph":{"height":10,"width":10,"pitch":10,"frequency":2,"amplitude":1.0,"offset":0.0}}"#,
            "default Graph Grayscale must remain omitted"
        );
        let graph = super::ColorEffectSpatialRecipe::Graph {
            grayscale: true,
            height: 100,
            width: 100,
            pitch: 100,
            frequency: 10,
            amplitude: 2.0,
            offset: 1.0,
        };
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialRecipe>(
                &serde_json::to_string(&graph).unwrap()
            )
            .unwrap(),
            graph,
            "Graph must round-trip at its validated maxima"
        );
        let legacy_rain: super::ColorEffectSpatialRecipe = serde_json::from_str(
            r#"{"Rain":{"speed":1,"width":5,"height":10,"number":50,"trail":10}}"#,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_string(&legacy_rain).unwrap(),
            r#"{"Rain":{"speed":1,"width":5,"height":10,"number":50,"trail":10}}"#,
            "default Rain Grayscale and RNG seed must remain omitted"
        );
        let rain = super::ColorEffectSpatialRecipe::Rain {
            grayscale: true,
            rng_seed: u32::MAX,
            speed: 10,
            width: 10,
            height: 30,
            number: 100,
            trail: 30,
        };
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialRecipe>(
                &serde_json::to_string(&rain).unwrap()
            )
            .unwrap(),
            rain,
            "Rain must round-trip at its validated maxima"
        );
        let legacy_fire: super::ColorEffectSpatialRecipe =
            serde_json::from_str(r#"{"Fire":{"flames":20,"width":20,"height":50,"hotspot":250}}"#)
                .unwrap();
        assert_eq!(
            serde_json::to_string(&legacy_fire).unwrap(),
            r#"{"Fire":{"flames":20,"width":20,"height":50,"hotspot":250}}"#,
            "default Fire Grayscale and RNG seed must remain omitted"
        );
        let fire = super::ColorEffectSpatialRecipe::Fire {
            grayscale: true,
            rng_seed: u32::MAX,
            flames: 100,
            width: 200,
            height: 100,
            hotspot: 255,
        };
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialRecipe>(
                &serde_json::to_string(&fire).unwrap()
            )
            .unwrap(),
            fire,
            "Fire must round-trip at its validated maxima"
        );
        let legacy_bounce: super::ColorEffectSpatialRecipe = serde_json::from_str(
            r#"{"Bounce":{"item":"Points","shape":28,"number":6,"size":40,"speed":1,"collide":false,"fill":true,"points":3}}"#,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_string(&legacy_bounce).unwrap(),
            r#"{"Bounce":{"item":"Points","shape":28,"number":6,"size":40,"speed":1,"collide":false,"fill":true,"points":3}}"#,
            "default Bounce Grayscale and RNG seed must remain omitted"
        );
        let bounce = super::ColorEffectSpatialRecipe::Bounce {
            grayscale: true,
            rng_seed: u32::MAX,
            item: super::ColorEffectSpatialBounceItem::Shape,
            shape: 0,
            number: 20,
            size: 100,
            speed: 10,
            collide: true,
            fill: false,
            points: 10,
        };
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialRecipe>(
                &serde_json::to_string(&bounce).unwrap()
            )
            .unwrap(),
            bounce,
            "Bounce must round-trip at its validated maxima"
        );
        let legacy_explosion: super::ColorEffectSpatialRecipe = serde_json::from_str(
            r#"{"Explosion":{"shape":0,"explosion_number":5,"explosion_size":5,"particle_number":10,"particle_size":10,"particle_life":0.0,"trail_size":10,"gravity":0.0}}"#,
        )
        .unwrap();
        assert!(matches!(
            &legacy_explosion,
            super::ColorEffectSpatialRecipe::Explosion {
                grayscale: false,
                rng_seed: 0,
                shape: 0,
                ..
            }
        ));
        assert_eq!(
            serde_json::to_string(&legacy_explosion).unwrap(),
            r#"{"Explosion":{"shape":0,"explosion_number":5,"explosion_size":5,"particle_number":10,"particle_size":10,"particle_life":0.0,"trail_size":10,"gravity":0.0}}"#,
            "default Explosion Grayscale and RNG seed must remain omitted"
        );
        let starfield = super::ColorEffectSpatialRecipe::Starfield {
            grayscale: true,
            rng_seed: u32::MAX,
            shape: 0,
            particles: 10,
            size: 100,
            trail: 25,
            rotation: -5.0,
        };
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialRecipe>(
                &serde_json::to_string(&starfield).unwrap()
            )
            .unwrap(),
            starfield,
            "Starfield must round-trip at its validated maxima"
        );

        let placed_rainbow = super::ColorEffectSpatialPattern {
            recipe: super::ColorEffectSpatialRecipe::Rainbow {
                grayscale: true,
                vertical_symmetry: false,
                horizontal_symmetry: true,
                rotation_degrees: 45.0,
                color_width: 0.25,
                angle_degrees: 90.0,
                gradient: 0.75,
            },
            parameter_model_version: super::COLOR_EFFECT_SPATIAL_PARAMETER_MODEL_VERSION,
            beam_targets: vec![super::ColorEffectBeamTarget {
                fixture_id: 8,
                beam_index: 3,
                selection_index: 2,
                feature_attribute: None,
            }],
            placement: Some(super::ColorEffectSpatialPlacement {
                source_coordinate_frame:
                    super::ColorEffectSpatialCoordinateFrame::DaslightPatchCanvas,
                mapping_shape: super::ColorEffectSpatialMappingShape::Rectangle,
                x: 2_630,
                y: -140,
                sx: 140,
                sy: 50,
                mapping_angle_degrees: 17.5,
                sampling_rule:
                    super::ColorEffectSpatialSamplingRule::RotatedInclusionMaskAxisAlignedRaster,
                vertical_symmetry: false,
                horizontal_symmetry: false,
                raster_rotation_degrees: 0.0,
                target_coordinates: vec![super::ColorEffectSpatialPlacementTarget {
                    fixture_id: 8,
                    beam_index: 3,
                    patch_x: 2_640,
                    patch_y: -130,
                }],
            }),
        };
        let placed_json = serde_json::to_string(&placed_rainbow).unwrap();
        assert!(placed_json.contains(r#""grayscale":true"#));
        assert!(placed_json.contains(r#""placement""#));
        assert!(!placed_json.contains(r#""raster_rotation_degrees""#));
        assert_eq!(
            placed_json.matches(r#""horizontal_symmetry":true"#).count(),
            1,
            "default placement fields stay omitted; the one match belongs to the Rainbow recipe"
        );
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialPattern>(&placed_json).unwrap(),
            placed_rainbow
        );

        let mut transformed_placement = placed_rainbow.clone();
        let placement = transformed_placement.placement.as_mut().unwrap();
        placement.horizontal_symmetry = true;
        placement.raster_rotation_degrees = 90.0;
        let transformed_json = serde_json::to_string(&transformed_placement).unwrap();
        assert!(transformed_json.contains(r#""horizontal_symmetry":true"#));
        assert!(transformed_json.contains(r#""raster_rotation_degrees":90.0"#));
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialPattern>(&transformed_json).unwrap(),
            transformed_placement
        );

        let legacy_pattern_json = r#"{"recipe":{"Perlin":{"octaves":5,"zoom":20.0,"direction_degrees":1.0,"speed":1.0,"amplitude":100.0}}}"#;
        let legacy_pattern: super::ColorEffectSpatialPattern =
            serde_json::from_str(legacy_pattern_json).unwrap();
        assert!(legacy_pattern.placement.is_none());
        assert_eq!(
            serde_json::to_string(&legacy_pattern).unwrap(),
            legacy_pattern_json,
            "a missing placement stays omitted and preserves the legacy byte shape"
        );
    }

    #[test]
    fn spiral_and_butterfly_grayscale_roundtrip_without_legacy_shape_drift() {
        for legacy_json in [
            r#"{"Spiral":{"radius":30.0,"arms":1,"gradient":100.0}}"#,
            r#"{"Butterfly":{"color_width":100.0,"gradient":50.0,"clockwise":true}}"#,
        ] {
            let legacy: super::ColorEffectSpatialRecipe =
                serde_json::from_str(legacy_json).unwrap();
            assert_eq!(serde_json::to_string(&legacy).unwrap(), legacy_json);
        }

        for recipe in [
            super::ColorEffectSpatialRecipe::Spiral {
                grayscale: true,
                radius: 30.0,
                arms: 1,
                gradient: 100.0,
            },
            super::ColorEffectSpatialRecipe::Butterfly {
                grayscale: true,
                color_width: 100.0,
                gradient: 50.0,
                clockwise: true,
            },
        ] {
            let json = serde_json::to_string(&recipe).unwrap();
            assert!(json.contains(r#""grayscale":true"#));
            assert_eq!(
                serde_json::from_str::<super::ColorEffectSpatialRecipe>(&json).unwrap(),
                recipe
            );
        }
    }

    #[test]
    fn unified_knight_rider_recipe_accepts_and_drops_legacy_route_flags() {
        let legacy_json = r#"{"KnightRider":{"daslight_exact":true,"size":8,"one_way":false,"fading":true,"go_outside":false,"gradient":50.0}}"#;
        let legacy: super::ColorEffectSpatialRecipe = serde_json::from_str(legacy_json).unwrap();
        let super::ColorEffectSpatialRecipe::KnightRider {
            grayscale,
            vertical_symmetry,
            size,
            ..
        } = &legacy
        else {
            unreachable!()
        };
        assert!(!grayscale);
        assert!(!vertical_symmetry);
        assert_eq!(*size, 8.0);
        assert!(!serde_json::to_string(&legacy)
            .unwrap()
            .contains("daslight_exact"));

        let unified = super::ColorEffectSpatialRecipe::KnightRider {
            grayscale: true,
            vertical_symmetry: true,
            size: 80.0,
            one_way: false,
            fading: true,
            go_outside: false,
            gradient: 50.0,
        };
        let json = serde_json::to_string(&unified).unwrap();
        assert!(!json.contains("daslight_exact"));
        assert!(json.contains(r#""grayscale":true"#));
        assert!(json.contains(r#""vertical_symmetry":true"#));
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialRecipe>(&json).unwrap(),
            unified
        );
    }

    #[test]
    fn unified_burst_recipe_accepts_and_drops_legacy_route_flags() {
        let legacy_json =
            r#"{"Burst":{"daslight_exact":false,"color_width":50.0,"gradient":100.0}}"#;
        let legacy: super::ColorEffectSpatialRecipe = serde_json::from_str(legacy_json).unwrap();
        let super::ColorEffectSpatialRecipe::Burst {
            grayscale,
            vertical_symmetry,
            ..
        } = &legacy
        else {
            unreachable!()
        };
        assert!(!grayscale);
        assert!(!vertical_symmetry);
        assert!(!serde_json::to_string(&legacy)
            .unwrap()
            .contains("daslight_exact"));

        let unified = super::ColorEffectSpatialRecipe::Burst {
            grayscale: true,
            vertical_symmetry: true,
            color_width: 50.0,
            gradient: 100.0,
        };
        let json = serde_json::to_string(&unified).unwrap();
        assert!(!json.contains("daslight_exact"));
        assert!(json.contains(r#""grayscale":true"#));
        assert!(json.contains(r#""vertical_symmetry":true"#));
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialRecipe>(&json).unwrap(),
            unified
        );
    }

    #[test]
    fn unified_random_recipes_accept_and_drop_legacy_route_fields() {
        let legacy_fill_json = r#"{"RandomFill":{"syndocal_corrected":false,"point_width":2}}"#;
        let legacy_fill: super::ColorEffectSpatialRecipe =
            serde_json::from_str(legacy_fill_json).unwrap();
        assert!(!serde_json::to_string(&legacy_fill)
            .unwrap()
            .contains("syndocal_corrected"));

        let corrected_fill = super::ColorEffectSpatialRecipe::RandomFill {
            grayscale: true,
            vertical_symmetry: true,
            rng_seed: 0x1234_5678,
            point_width: 20.0,
            source_point_height: Some(7),
        };
        let fill_json = serde_json::to_string(&corrected_fill).unwrap();
        assert!(!fill_json.contains("syndocal_corrected"));
        assert!(fill_json.contains(r#""rng_seed":305419896"#));
        assert!(fill_json.contains(r#""source_point_height":7"#));
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialRecipe>(&fill_json).unwrap(),
            corrected_fill
        );
        let unplaced_color_fill = super::ColorEffectSpatialRecipe::RandomFill {
            grayscale: false,
            vertical_symmetry: false,
            rng_seed: 0,
            point_width: 2.0,
            source_point_height: None,
        };
        assert!(!serde_json::to_string(&unplaced_color_fill)
            .unwrap()
            .contains("source_point_height"));

        let legacy_sparkle_json =
            r#"{"Sparkle":{"syndocal_corrected":false,"number":5,"lifespan":25.0,"width":1}}"#;
        let legacy_sparkle: super::ColorEffectSpatialRecipe =
            serde_json::from_str(legacy_sparkle_json).unwrap();
        assert!(!serde_json::to_string(&legacy_sparkle)
            .unwrap()
            .contains("lifespan"));

        let corrected_sparkle = super::ColorEffectSpatialRecipe::Sparkle {
            grayscale: true,
            vertical_symmetry: true,
            raster_mode: super::ColorEffectSpatialSparkleRasterMode::Sparkle,
            rng_seed: 7,
            number: 5,
            lifetime_ms: Some(250),
            source_lifespan: Some(0.6),
            width: 30.0,
            height: None,
        };
        let sparkle_json = serde_json::to_string(&corrected_sparkle).unwrap();
        assert!(sparkle_json.contains(r#""lifetime_ms":250"#));
        assert!(sparkle_json.contains(r#""source_lifespan":0.6"#));
        assert!(!sparkle_json.contains("raster_mode"));
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialRecipe>(&sparkle_json).unwrap(),
            corrected_sparkle
        );

        let tube = super::ColorEffectSpatialRecipe::Sparkle {
            grayscale: true,
            vertical_symmetry: true,
            raster_mode: super::ColorEffectSpatialSparkleRasterMode::TubeFullRasterHeight,
            rng_seed: 7,
            number: 5,
            lifetime_ms: Some(250),
            source_lifespan: Some(0.6),
            width: 30.0,
            height: None,
        };
        let tube_json = serde_json::to_string(&tube).unwrap();
        assert!(tube_json.contains(r#""raster_mode":"TubeFullRasterHeight""#));
        assert_eq!(
            serde_json::from_str::<super::ColorEffectSpatialRecipe>(&tube_json).unwrap(),
            tube
        );
    }

    #[test]
    fn value_effect_spatial_pattern_roundtrips_and_legacy_defaults_to_none() {
        let legacy = serde_json::json!({
            "label": "Legacy value",
            "fixture_ids": [1],
            "target_group_ids": [],
            "attribute": "Dimmer",
            "points": [
                {"position": 0.0, "value": 0.0},
                {"position": 1.0, "value": 1.0}
            ],
            "interpolation": "Line",
            "mode": "Absolute",
            "direction": "Forward",
            "period_ms": 1000,
            "low": 0,
            "high": 65535,
            "phase": 0.0,
            "fixture_spread": 0.0,
            "blend_mode": "Override"
        });
        let mut parsed: super::ValueEffectRequest = serde_json::from_value(legacy).unwrap();
        assert!(parsed.spatial_pattern.is_none());
        assert!(!serde_json::to_string(&parsed)
            .unwrap()
            .contains("spatial_pattern"));

        parsed.spatial_pattern = Some(super::ColorEffectSpatialPattern {
            recipe: super::ColorEffectSpatialRecipe::ColorRainbow {
                grayscale: false,
                vertical_symmetry: true,
                color_width: 0.25,
                angle_degrees: 90.0,
                gradient: 75.0,
            },
            parameter_model_version: super::COLOR_EFFECT_SPATIAL_PARAMETER_MODEL_VERSION,
            beam_targets: vec![super::ColorEffectBeamTarget {
                fixture_id: 1,
                beam_index: 3,
                selection_index: 8,
                feature_attribute: Some("ColorRed 4".to_string()),
            }],
            placement: None,
        });
        let json = serde_json::to_string(&parsed).unwrap();
        let roundtrip: super::ValueEffectRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, parsed);

        parsed.spatial_pattern = Some(super::ColorEffectSpatialPattern {
            recipe: super::ColorEffectSpatialRecipe::Sweep {
                grayscale: false,
                vertical_symmetry: false,
                direction_change: true,
            },
            parameter_model_version: super::COLOR_EFFECT_SPATIAL_PARAMETER_MODEL_VERSION,
            beam_targets: vec![super::ColorEffectBeamTarget {
                fixture_id: 1,
                beam_index: 3,
                selection_index: 8,
                feature_attribute: Some("ColorRed 4".to_string()),
            }],
            placement: None,
        });
        let json = serde_json::to_string(&parsed).unwrap();
        assert!(!json.contains("daslight_exact"));
        assert!(!json.contains("grayscale"));
        assert!(!json.contains("vertical_symmetry"));
        let roundtrip: super::ValueEffectRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, parsed);

        if let Some(pattern) = parsed.spatial_pattern.as_mut() {
            pattern.recipe = super::ColorEffectSpatialRecipe::Sweep {
                grayscale: true,
                vertical_symmetry: true,
                direction_change: false,
            };
        }
        let json = serde_json::to_string(&parsed).unwrap();
        let roundtrip: super::ValueEffectRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, parsed);
    }

    #[test]
    fn chaser_effect_request_and_preset_roundtrip_multi_feature_width() {
        let request = super::ChaserEffectRequest {
            label: "Front multi-feature chase".to_string(),
            steps: vec![
                super::ChaserStep {
                    fixture_ids: vec![1],
                    target_group_ids: Vec::new(),
                    beam_targets: vec![super::EffectBeamTarget {
                        fixture_id: 1,
                        beam_index: 7,
                        selection_index: 0,
                        feature_attribute: "Dimmer".to_string(),
                    }],
                    level: 65_535,
                },
                super::ChaserStep {
                    fixture_ids: vec![2],
                    target_group_ids: vec!["Front".to_string()],
                    beam_targets: Vec::new(),
                    level: 32_768,
                },
            ],
            features: vec![
                super::ChaserFeature {
                    attribute: "Dimmer".to_string(),
                    low: 0,
                    high: 65_535,
                },
                super::ChaserFeature {
                    attribute: "Pan".to_string(),
                    low: 10_000,
                    high: 50_000,
                },
            ],
            step_duration_ms: 250,
            clock_sync: Some(super::EffectClockSync { beats: 0.5 }),
            direction: super::ChaserDirection::Bounce,
            wings: 2,
            active_step_count: 2,
            duty_cycle: 0.75,
            overlap: 0.25,
            phase: 0.125,
            fixture_spread: 0.5,
            random_seed: 0x5eed,
            random_cycle_count: 3,
            blend_mode: super::EffectBlendMode::Override,
        };
        let preset = super::EffectPreset {
            version: 1,
            effect_type: super::EffectKind::Chaser,
            enabled: true,
            lfo: None,
            position_wave: None,
            color: None,
            chaser: Some(request),
            move_effect: None,
            value: None,
            curve: None,
            mapping: None,
            color_mapping: None,
        };

        let json = serde_json::to_string(&preset).unwrap();
        let parsed: super::EffectPreset = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, preset);

        let mut legacy_json = serde_json::to_value(&preset).unwrap();
        legacy_json["chaser"]["steps"][0]
            .as_object_mut()
            .unwrap()
            .remove("beam_targets");
        let legacy: super::EffectPreset = serde_json::from_value(legacy_json).unwrap();
        assert!(legacy.chaser.unwrap().steps[0].beam_targets.is_empty());

        let mut legacy_json = serde_json::to_value(&preset).unwrap();
        legacy_json["chaser"]
            .as_object_mut()
            .unwrap()
            .remove("random_cycle_count");
        let legacy: super::EffectPreset = serde_json::from_value(legacy_json).unwrap();
        assert_eq!(legacy.chaser.unwrap().random_cycle_count, 1);
    }

    #[test]
    fn move_effect_request_and_preset_roundtrip() {
        let request = super::MoveEffectRequest {
            label: "Front circle".to_string(),
            fixture_ids: vec![1, 2],
            target_group_ids: vec!["Moving".to_string()],
            beam_targets: Vec::new(),
            points: vec![
                super::MovePathPoint { x: 0.5, y: 0.0 },
                super::MovePathPoint { x: 1.0, y: 0.5 },
                super::MovePathPoint { x: 0.5, y: 1.0 },
                super::MovePathPoint { x: 0.0, y: 0.5 },
            ],
            closed: true,
            interpolation: super::MoveInterpolation::Smooth,
            coordinate_mode: super::MoveCoordinateMode::Absolute,
            center_x: 0.5,
            center_y: 0.5,
            size_x: 0.75,
            size_y: 0.5,
            rotation_degrees: 30.0,
            period_ms: 2_000,
            clock_sync: Some(super::EffectClockSync { beats: 4.0 }),
            direction: super::MoveDirection::Bounce,
            phase: 0.125,
            fixture_spread: 1.0,
            symmetry: true,
            blend_mode: super::EffectBlendMode::Override,
        };
        let preset = super::EffectPreset {
            version: 1,
            effect_type: super::EffectKind::Move,
            enabled: false,
            lfo: None,
            position_wave: None,
            color: None,
            chaser: None,
            move_effect: Some(request),
            value: None,
            curve: None,
            mapping: None,
            color_mapping: None,
        };

        let json = serde_json::to_string(&preset).unwrap();
        let parsed: super::EffectPreset = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, preset);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).unwrap()["move_effect"]["symmetry"],
            true
        );

        let legacy_json = json.replace(",\"symmetry\":true", "");
        let legacy: super::EffectPreset = serde_json::from_str(&legacy_json).unwrap();
        assert!(!legacy.move_effect.unwrap().symmetry);
    }

    #[test]
    fn move_circle_beam_targets_roundtrip_in_authored_order() {
        let request = super::MoveEffectRequest {
            label: "Imported Circle".to_string(),
            fixture_ids: vec![1, 2],
            target_group_ids: Vec::new(),
            beam_targets: vec![
                super::MoveEffectBeamTarget {
                    fixture_id: 1,
                    beam_index: 4,
                    selection_index: 0,
                },
                super::MoveEffectBeamTarget {
                    fixture_id: 1,
                    beam_index: 2,
                    selection_index: 1,
                },
                super::MoveEffectBeamTarget {
                    fixture_id: 2,
                    beam_index: 7,
                    selection_index: 2,
                },
            ],
            points: vec![
                super::MovePathPoint { x: 0.25, y: 0.5 },
                super::MovePathPoint { x: 0.5, y: 0.75 },
                super::MovePathPoint { x: 0.75, y: 0.5 },
                super::MovePathPoint { x: 0.5, y: 0.25 },
            ],
            closed: true,
            interpolation: super::MoveInterpolation::Circle,
            coordinate_mode: super::MoveCoordinateMode::Absolute,
            center_x: 0.5,
            center_y: 0.5,
            size_x: 1.0,
            size_y: 1.0,
            rotation_degrees: 0.0,
            period_ms: 2_000,
            clock_sync: None,
            direction: super::MoveDirection::Forward,
            phase: 0.0,
            fixture_spread: 0.0,
            symmetry: false,
            blend_mode: super::EffectBlendMode::Override,
        };
        let preset = super::EffectPreset {
            version: 1,
            effect_type: super::EffectKind::Move,
            enabled: true,
            lfo: None,
            position_wave: None,
            color: None,
            chaser: None,
            move_effect: Some(request),
            value: None,
            curve: None,
            mapping: None,
            color_mapping: None,
        };

        let json = serde_json::to_string(&preset).unwrap();
        let parsed: super::EffectPreset = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, preset);
        let value = serde_json::from_str::<serde_json::Value>(&json).unwrap();
        assert_eq!(value["move_effect"]["interpolation"], "Circle");
        assert_eq!(
            value["move_effect"]["beam_targets"],
            serde_json::json!([
                {"fixture_id": 1, "beam_index": 4, "selection_index": 0},
                {"fixture_id": 1, "beam_index": 2, "selection_index": 1},
                {"fixture_id": 2, "beam_index": 7, "selection_index": 2}
            ]),
            "Move beam targets must retain authored Vec order"
        );
    }

    #[test]
    fn daslight_move_interpolation_variants_have_distinct_persistent_names() {
        let variants = [
            (super::MoveInterpolation::DaslightCircle, "DaslightCircle"),
            (super::MoveInterpolation::DaslightCurve, "DaslightCurve"),
            (super::MoveInterpolation::DaslightLine, "DaslightLine"),
            (super::MoveInterpolation::DaslightPolygon, "DaslightPolygon"),
            (super::MoveInterpolation::DaslightPoints, "DaslightPoints"),
        ];

        for (variant, name) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{name}\""));
            assert_eq!(
                serde_json::from_str::<super::MoveInterpolation>(&json).unwrap(),
                variant
            );
        }
    }

    #[test]
    fn legacy_move_effect_request_roundtrips_without_changing_bytes() {
        let legacy_json = r#"{"label":"Legacy move","fixture_ids":[1,2],"target_group_ids":["Moving"],"points":[{"x":0.5,"y":0.0},{"x":1.0,"y":0.5}],"closed":false,"interpolation":"Smooth","coordinate_mode":"Absolute","center_x":0.5,"center_y":0.5,"size_x":1.0,"size_y":1.0,"rotation_degrees":0.0,"period_ms":2000,"direction":"Forward","phase":0.0,"fixture_spread":0.0,"blend_mode":"Override"}"#;

        let parsed: super::MoveEffectRequest = serde_json::from_str(legacy_json).unwrap();

        assert!(parsed.beam_targets.is_empty());
        assert_eq!(
            serde_json::to_string(&parsed).unwrap(),
            legacy_json,
            "a missing Move beam target list must stay omitted and preserve the legacy byte shape"
        );
    }

    #[test]
    fn child_timeline_defaults_every_collection_and_legacy_cue_loads_none() {
        let child: super::ChildTimelineSummary =
            serde_json::from_str(r#"{"duration_ms":1200}"#).unwrap();
        assert!(child.layers.is_empty());
        assert!(child.events.is_empty());
        assert!(child.automations.is_empty());
        assert!(child.video_automations.is_empty());
        assert!(child.audio.is_none());
        assert!(child.audio_clips.is_empty());
        assert!(!child.tempo_driven);
        assert!(!child.metronome_enabled);
        assert_eq!(child.count_in_beats, 4);
        assert_eq!(child.duration_ms, 1_200);

        let legacy = serde_json::to_value(super::CueSummary::default()).unwrap();
        assert!(legacy.get("child_timeline").is_none());
        let parsed: super::CueSummary = serde_json::from_value(legacy).unwrap();
        assert!(parsed.child_timeline.is_none());
    }

    #[test]
    fn identity_color_defaults_are_skipped_and_roundtrip() {
        // Legacy cues carry no color field and stay byte-identical on save.
        let legacy = serde_json::to_value(super::CueSummary::default()).unwrap();
        assert!(legacy.get("color").is_none());
        let parsed: super::CueSummary = serde_json::from_value(legacy).unwrap();
        assert!(parsed.color.is_none());

        let cue = super::CueSummary {
            id: 12,
            color: Some("#ff3366".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&cue).unwrap();
        let back: super::CueSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.color.as_deref(), Some("#ff3366"));

        // Empty group_colors is skipped on the snapshot; populated maps
        // roundtrip in deterministic BTreeMap order.
        let snapshot = super::EngineSnapshot::default();
        let value = serde_json::to_value(&snapshot).unwrap();
        assert!(value.get("group_colors").is_none());
        let mut colored = super::EngineSnapshot::default();
        colored
            .group_colors
            .insert("Front".to_string(), "#22aa88".to_string());
        colored
            .group_colors
            .insert("Back".to_string(), "#8844cc".to_string());
        let json = serde_json::to_string(&colored).unwrap();
        let back: super::EngineSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.group_colors.len(), 2);
        assert_eq!(
            back.group_colors.get("Front").map(String::as_str),
            Some("#22aa88")
        );
        assert_eq!(
            back.group_colors.get("Back").map(String::as_str),
            Some("#8844cc")
        );
    }

    #[test]
    fn machine_output_role_has_exact_runtime_only_ownership_matrix() {
        assert_eq!(
            super::MachineOutputRole::default(),
            super::MachineOutputRole::Both
        );
        assert_eq!(
            serde_json::to_string(&super::MachineOutputRole::Lighting).unwrap(),
            "\"Lighting\""
        );
        let cases = [
            (super::MachineOutputRole::Lighting, true, false),
            (super::MachineOutputRole::Video, false, true),
            (super::MachineOutputRole::Both, true, true),
            (super::MachineOutputRole::Standby, false, false),
        ];
        for (role, lighting_allowed, video_allowed) in cases {
            let status = super::OutputOwnershipStatus::for_role(role);
            assert_eq!(status.lighting_allowed, lighting_allowed);
            assert_eq!(status.video_allowed, video_allowed);
            assert_eq!(
                status.lighting_reason,
                if lighting_allowed {
                    super::OutputOwnershipReason::OwnedByMachineRole
                } else {
                    super::OutputOwnershipReason::BlockedByMachineRole
                }
            );
            assert_eq!(
                status.video_reason,
                if video_allowed {
                    super::OutputOwnershipReason::OwnedByMachineRole
                } else {
                    super::OutputOwnershipReason::BlockedByMachineRole
                }
            );
        }
        let activating = super::OutputOwnershipStatus::activating(
            super::MachineOutputRole::Video,
            Some(super::MachineOutputRole::Standby),
            0,
            1,
        );
        assert_eq!(activating.state, super::OutputOwnershipState::Activating);
        assert!(!activating.lighting_allowed && !activating.video_allowed);
    }

    #[test]
    fn output_ownership_status_default_is_fail_closed() {
        assert_eq!(
            super::OutputOwnershipStatus::default(),
            super::OutputOwnershipStatus {
                role: super::MachineOutputRole::Standby,
                effective_role: super::MachineOutputRole::Standby,
                desired_role: super::MachineOutputRole::Standby,
                persisted_role: None,
                state: super::OutputOwnershipState::Failed,
                generation: 0,
                epoch: 0,
                lighting_allowed: false,
                video_allowed: false,
                lighting_reason: super::OutputOwnershipReason::StartupDenied,
                video_reason: super::OutputOwnershipReason::StartupDenied,
                error: Some("Machine output ownership has not been initialized".to_string()),
            }
        );
    }

    #[test]
    fn project_swap_disarmed_status_constructor_preserves_every_field_exactly() {
        let status = super::OutputOwnershipStatus::project_swap_disarmed(
            super::MachineOutputRole::Both,
            Some(super::MachineOutputRole::Video),
            42,
            7,
        );

        assert_eq!(
            status,
            super::OutputOwnershipStatus {
                role: super::MachineOutputRole::Standby,
                effective_role: super::MachineOutputRole::Standby,
                desired_role: super::MachineOutputRole::Both,
                persisted_role: Some(super::MachineOutputRole::Video),
                state: super::OutputOwnershipState::Ready,
                generation: 42,
                epoch: 7,
                lighting_allowed: false,
                video_allowed: false,
                lighting_reason: super::OutputOwnershipReason::ProjectSwapDisarmed,
                video_reason: super::OutputOwnershipReason::ProjectSwapDisarmed,
                error: None,
            }
        );
        assert_eq!(
            serde_json::to_string(&super::OutputOwnershipReason::ProjectSwapDisarmed).unwrap(),
            "\"ProjectSwapDisarmed\""
        );
    }

    #[test]
    fn machine_output_role_is_absent_from_project_snapshot_serialization() {
        let value = serde_json::to_value(super::EngineSnapshot::default()).unwrap();
        assert!(value.get("machine_output_role").is_none());
        assert!(value.get("output_ownership").is_none());
    }

    #[test]
    fn touch_surface_roundtrips_all_control_kinds() {
        use super::{
            TouchControlBinding as Binding, TouchControlKind as Kind, TouchControlSummary,
            TouchPageSummary, TouchSurfaceSummary,
        };

        let bindings = vec![
            Some(Binding::FixtureAttribute {
                fixture_id: 7,
                attribute: "Dimmer".to_string(),
            }),
            Some(Binding::GroupAttribute {
                group_id: "front".to_string(),
                attribute: "Dimmer".to_string(),
            }),
            Some(Binding::Cue { cue_id: 12 }),
            Some(Binding::GroupSubmaster {
                group_id: "front".to_string(),
            }),
            Some(Binding::LightingMaster),
            Some(Binding::SelectedFixtureColor),
            Some(Binding::SelectedFixturePanTilt {
                pan_attribute: "Pan".to_string(),
                tilt_attribute: "Tilt".to_string(),
            }),
            None,
        ];
        let kinds = [
            Kind::Label,
            Kind::Image,
            Kind::Button,
            Kind::Fader,
            Kind::Dial,
            Kind::IncrementalWheel,
            Kind::ColorWheel,
            Kind::XyGrid,
        ];
        let controls = kinds
            .into_iter()
            .zip(bindings)
            .enumerate()
            .map(|(index, (kind, binding))| TouchControlSummary {
                id: index as u64 + 1,
                kind,
                x: (index % 4) as u16 * 3,
                y: (index / 4) as u16 * 4,
                w: 3,
                h: 4,
                label: format!("Control {}", index + 1),
                binding,
            })
            .collect();
        let surface = TouchSurfaceSummary {
            pages: vec![TouchPageSummary {
                id: 1,
                label: "Default".to_string(),
                controls,
            }],
        };

        let json = serde_json::to_string(&surface).unwrap();
        let decoded: TouchSurfaceSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, surface);
    }

    #[test]
    fn touch_surface_roundtrips_all_binding_variants() {
        use super::TouchControlBinding as Binding;

        let bindings = vec![
            Binding::FixtureAttribute {
                fixture_id: 7,
                attribute: "Dimmer".to_string(),
            },
            Binding::GroupAttribute {
                group_id: "front".to_string(),
                attribute: "Dimmer".to_string(),
            },
            Binding::FeaturePreset {
                targets: vec![super::TouchFeaturePresetTarget {
                    fixture_id: 7,
                    attribute: "Dimmer".to_string(),
                }],
                min_value: 0,
                max_value: u16::MAX,
                inverted: false,
            },
            Binding::FixtureColor { fixture_id: 7 },
            Binding::GroupColor {
                group_id: "front".to_string(),
            },
            Binding::FixturePanTilt {
                fixture_id: 7,
                pan_attribute: "Pan".to_string(),
                tilt_attribute: "Tilt".to_string(),
            },
            Binding::GroupPanTilt {
                group_id: "front".to_string(),
                pan_attribute: "Pan".to_string(),
                tilt_attribute: "Tilt".to_string(),
            },
            Binding::Cue { cue_id: 12 },
            Binding::GroupSelect {
                group_id: "front".to_string(),
            },
            Binding::GroupSubmaster {
                group_id: "front".to_string(),
            },
            Binding::TapTempo,
            Binding::LightingMaster,
            Binding::VideoMaster,
            Binding::Blackout,
            Binding::VideoBlackout,
            Binding::AllBlackout,
            Binding::CueNext,
            Binding::CuePrevious,
            Binding::CueFadePause,
            Binding::SelectedFixtureAttribute {
                attribute: "Dimmer".to_string(),
            },
            Binding::SelectedFixtureColor,
            Binding::SelectedFixturePanTilt {
                pan_attribute: "Pan".to_string(),
                tilt_attribute: "Tilt".to_string(),
            },
        ];

        let json = serde_json::to_string(&bindings).unwrap();
        let decoded: Vec<Binding> = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, bindings);
    }

    #[test]
    fn touch_surface_legacy_absence_defaults_empty_and_is_byte_identical() {
        let legacy_bytes = serde_json::to_vec(&super::EngineSnapshot::default()).unwrap();
        let legacy_value: serde_json::Value = serde_json::from_slice(&legacy_bytes).unwrap();
        assert!(legacy_value.get("touch_surface").is_none());

        let mut legacy_without_touch = legacy_value;
        legacy_without_touch
            .as_object_mut()
            .unwrap()
            .remove("touch_surface");
        let decoded: super::EngineSnapshot = serde_json::from_value(legacy_without_touch).unwrap();
        assert!(decoded.touch_surface.pages.is_empty());
        let roundtrip_bytes = serde_json::to_vec(&decoded).unwrap();

        assert_eq!(roundtrip_bytes, legacy_bytes);
        eprintln!(
            "legacy touch snapshot bytes: {} == {}",
            legacy_bytes.len(),
            roundtrip_bytes.len()
        );
    }

    #[test]
    fn child_timeline_roundtrip_includes_f7_audio_clips() {
        let cue = super::CueSummary {
            id: 9,
            child_timeline: Some(super::ChildTimelineSummary {
                layers: vec![super::TimelineLayerSummary {
                    id: 4,
                    label: "Child Audio".to_string(),
                    order: 0,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: false,
                    kind: super::TimelineLayerKind::Audio,
                }],
                audio_clips: vec![super::TimelineAudioClipSummary {
                    id: 7,
                    layer_id: 4,
                    media_asset_id: None,
                    path: "child.wav".to_string(),
                    start_ms: 100,
                    offset_ms: 20,
                    duration_ms: 900,
                    gain: 0.75,
                    fade_in_ms: 50,
                    fade_out_ms: 80,
                    output_bus: super::TimelineAudioOutputBus::Cue,
                }],
                duration_ms: 1_000,
                ..super::ChildTimelineSummary::default()
            }),
            ..Default::default()
        };
        let json = serde_json::to_vec(&cue).unwrap();
        let parsed: super::CueSummary = serde_json::from_slice(&json).unwrap();
        assert_eq!(parsed, cue);
    }

    #[test]
    fn channel_function_emitter_calibration_is_additive_and_legacy_omits_it() {
        let legacy = serde_json::json!({
            "name": "Red",
            "attribute": "ColorAdd_R",
            "parent_function": null,
            "dmx_from": 0,
            "dmx_to": 65535,
            "physical_from": null,
            "physical_to": null,
            "wheel_slot": null,
            "wheel_slot_name": null,
            "wheel_slot_color": null,
            "wheel_slot_media": null
        });
        let parsed: super::ChannelFunctionSummary = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(parsed.emitter, None);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), legacy);

        let mut calibrated = parsed;
        calibrated.emitter = Some(super::EmitterCalibrationSummary {
            name: "Red LED".to_string(),
            color: Some(super::CieColorSummary {
                x: 0.64,
                y: 0.33,
                luminance: 0.2126,
            }),
            dominant_wavelength_nm: Some(625.0),
        });
        let encoded = serde_json::to_value(&calibrated).unwrap();
        assert_eq!(encoded["emitter"]["name"], "Red LED");
        assert_eq!(
            serde_json::from_value::<super::ChannelFunctionSummary>(encoded).unwrap(),
            calibrated
        );
    }

    #[test]
    fn cue_live_direction_and_segment_are_additive_to_t17_settings() {
        let legacy = r#"{"speed":2.0,"size":0.5,"phase":0.25,"flash":true}"#;
        let parsed: super::CueLiveModifierSettings = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.direction, super::CueLiveDirection::Authored);
        assert_eq!(parsed.segment, 0);
        assert_eq!(serde_json::to_string(&parsed).unwrap(), legacy);

        let extended = super::CueLiveModifierSettings {
            direction: super::CueLiveDirection::Reverse,
            segment: 3,
            ..parsed
        };
        let encoded = serde_json::to_string(&extended).unwrap();
        assert!(encoded.contains(r#""direction":"Reverse""#));
        assert!(encoded.contains(r#""segment":3"#));
        assert_eq!(
            serde_json::from_str::<super::CueLiveModifierSettings>(&encoded).unwrap(),
            extended
        );
    }

    #[test]
    fn submaster_live_strobe_fields_are_additive_and_legacy_byte_identical() {
        let legacy = r#"{"group_id":"front","label":"Front","level":0.75}"#;
        let parsed: super::SubmasterSummary = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.strobe_hz, 0.0);
        assert_eq!(parsed.strobe_fixture_count, 0);
        assert_eq!(serde_json::to_string(&parsed).unwrap(), legacy);

        let live = super::SubmasterSummary {
            strobe_hz: 12.0,
            strobe_fixture_count: 4,
            ..parsed
        };
        let encoded = serde_json::to_string(&live).unwrap();
        assert!(encoded.contains(r#""strobe_hz":12.0"#));
        assert!(encoded.contains(r#""strobe_fixture_count":4"#));
        assert_eq!(
            serde_json::from_str::<super::SubmasterSummary>(&encoded).unwrap(),
            live
        );
    }

    #[test]
    fn video_effect_chain_legacy_migration_is_atomic_idempotent_and_exact() {
        let mut layer = media_asset_test_layer(7, "Layer FX", "C:/show/fx.mp4");
        layer.isf_effect = Some(clip_slot_test_effect());
        layer.media_asset_id = Some(10);
        layer.clip_slots = vec![clip_slot_test_slot(20, 10)];
        layer.default_clip_slot_id = Some(super::VideoClipSlotId(20));
        layer.clip_slots[0].effect_overrides = vec![super::VideoClipEffectOverrideSummary {
            effect_id: None,
            stage_index: 0,
            control_name: "amount".to_string(),
            value: [0.5, 0.0, 0.0, 0.0],
        }];
        let mut video = super::VideoSnapshot {
            layers: vec![layer],
            media_assets: vec![media_asset_test_asset(10, "FX", "C:/show/fx.mp4")],
            ..super::VideoSnapshot::default()
        };

        let report = super::normalize_legacy_video_effect_chains(&mut video).unwrap();
        assert_eq!(report.created_chain_ids, vec![super::VideoEffectChainId(1)]);
        assert_eq!(report.migrated_layer_ids, vec![7]);
        assert_eq!(video.effect_chains.len(), 1);
        assert_eq!(
            video.effect_chains[0].scope,
            super::VideoEffectScope::Layer { layer_id: 7 }
        );
        assert_eq!(
            video.effect_chains[0].stages[0].id,
            super::VideoEffectStageId(1)
        );
        assert_eq!(
            video.effect_chains[0].stages[0].effect.id,
            super::VideoEffectId(1)
        );
        assert_eq!(
            video.layers[0].clip_slots[0].effect_overrides[0].effect_id,
            Some(super::VideoEffectId(1))
        );
        super::validate_engine_ready_video_effect_chains(&video).unwrap();
        let stable = video.clone();
        assert_eq!(
            super::normalize_legacy_video_effect_chains(&mut video).unwrap(),
            super::VideoEffectChainMigrationReport::default()
        );
        assert_eq!(video, stable);

        let json = serde_json::to_string(&video).unwrap();
        assert_eq!(
            serde_json::from_str::<super::VideoSnapshot>(&json).unwrap(),
            video
        );
        let mut divergent = video;
        let super::VideoEffectKind::Isf { effect } =
            &mut divergent.effect_chains[0].stages[0].effect.kind;
        effect.label = "Different".to_string();
        let original = divergent.clone();
        assert!(super::normalize_legacy_video_effect_chains(&mut divergent).is_err());
        assert_eq!(divergent, original);
    }

    #[test]
    fn video_effect_chain_scope_group_limits_and_forward_bus_fail_closed() {
        let layers = (1..=3)
            .map(|id| {
                media_asset_test_layer(id, &format!("Layer {id}"), &format!("C:/show/{id}.mp4"))
            })
            .collect::<Vec<_>>();
        let mut video = super::VideoSnapshot {
            layers,
            compositions: vec![super::CompositionSummary {
                id: 9,
                label: "Main".to_string(),
                layer_ids: vec![1, 2, 3],
                output_ids: Vec::new(),
            }],
            layer_groups: vec![super::VideoLayerGroupSummary {
                id: super::VideoLayerGroupId(11),
                label: "Backgrounds".to_string(),
                composition_id: 9,
                layer_ids: vec![1, 2],
            }],
            effect_chains: vec![super::VideoEffectChainSummary {
                id: super::VideoEffectChainId(21),
                scope: super::VideoEffectScope::Group {
                    group_id: super::VideoLayerGroupId(11),
                },
                bypassed: false,
                stages: Vec::new(),
            }],
            ..super::VideoSnapshot::default()
        };
        super::validate_authored_video_effect_chains(&video).unwrap();

        let mut duplicate_scope = video.clone();
        duplicate_scope
            .effect_chains
            .push(super::VideoEffectChainSummary {
                id: super::VideoEffectChainId(22),
                scope: super::VideoEffectScope::Group {
                    group_id: super::VideoLayerGroupId(11),
                },
                bypassed: true,
                stages: Vec::new(),
            });
        assert!(super::validate_authored_video_effect_chains(&duplicate_scope).is_err());

        video.effect_chains[0].scope = super::VideoEffectScope::Transition {
            owner: super::VideoTransitionEffectOwner::LayerBus {
                bus_id: super::VideoTransitionBusId(5),
            },
        };
        assert!(super::validate_authored_video_effect_chains(&video).is_err());

        let mut non_contiguous = video.clone();
        non_contiguous.effect_chains.clear();
        non_contiguous.layer_groups[0].layer_ids = vec![1, 3];
        assert!(super::validate_authored_video_effect_chains(&non_contiguous).is_err());

        let mut reversed = video.clone();
        reversed.effect_chains.clear();
        reversed.layer_groups[0].layer_ids = vec![2, 1];
        assert!(super::validate_authored_video_effect_chains(&reversed).is_err());

        let mut duplicate_composition = video.clone();
        duplicate_composition.effect_chains.clear();
        duplicate_composition.layer_groups.clear();
        duplicate_composition
            .compositions
            .push(super::CompositionSummary {
                id: 9,
                label: "Ambiguous".to_string(),
                layer_ids: vec![3],
                output_ids: Vec::new(),
            });
        assert!(super::validate_authored_video_effect_chains(&duplicate_composition).is_err());

        let mut zero_output = video.clone();
        zero_output.effect_chains.clear();
        zero_output.layer_groups.clear();
        zero_output.outputs.push(super::VideoOutputSummary {
            id: 0,
            label: "Invalid".to_string(),
            kind: super::VideoOutputKind::Display,
            enabled: true,
            composition_id: 9,
            fullscreen: false,
            monitor_id: None,
            monitor_identity: None,
            width: 1920,
            height: 1080,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: super::VideoOutputMapping::default(),
        });
        assert!(super::validate_authored_video_effect_chains(&zero_output).is_err());

        let mut too_many_stages = non_contiguous;
        too_many_stages.layer_groups[0].layer_ids = vec![1, 2];
        too_many_stages.effect_chains = vec![super::VideoEffectChainSummary {
            id: super::VideoEffectChainId(40),
            scope: super::VideoEffectScope::Clip {
                layer_id: 1,
                slot_id: super::VideoClipSlotId(1),
            },
            bypassed: false,
            stages: Vec::new(),
        }];
        too_many_stages.layers[0].clip_slots = vec![clip_slot_test_slot(1, 1)];
        too_many_stages.layers[0].media_asset_id = Some(1);
        too_many_stages.media_assets = vec![media_asset_test_asset(1, "A", "C:/show/1.mp4")];
        let stage = super::VideoEffectStageSummary {
            id: super::VideoEffectStageId(1),
            enabled: true,
            label: "Stage".to_string(),
            effect: super::VideoEffectSummary {
                id: super::VideoEffectId(1),
                kind: super::VideoEffectKind::Isf {
                    effect: clip_slot_test_effect(),
                },
            },
        };
        too_many_stages.effect_chains[0].stages =
            vec![stage; super::VIDEO_EFFECT_CHAIN_MAX_STAGES + 1];
        assert!(super::validate_authored_video_effect_chains(&too_many_stages).is_err());
    }

    #[test]
    fn timeline_follow_transition_effect_owner_is_stable_unique_and_reference_checked() {
        let scope = super::VideoEffectScope::Transition {
            owner: super::VideoTransitionEffectOwner::TimelineFollow {
                source_timeline_id: super::TimelineId(91),
            },
        };
        let mut video = super::VideoSnapshot {
            effect_chains: vec![super::VideoEffectChainSummary {
                id: super::VideoEffectChainId(71),
                scope: scope.clone(),
                bypassed: false,
                stages: Vec::new(),
            }],
            ..super::VideoSnapshot::default()
        };
        super::validate_authored_video_effect_chains(&video).unwrap();

        let wire = serde_json::to_value(&scope).unwrap();
        assert_eq!(wire["scope"], "transition");
        assert_eq!(wire["owner"]["kind"], "timeline_follow");
        assert_eq!(wire["owner"]["source_timeline_id"], 91);
        assert_eq!(
            serde_json::from_value::<super::VideoEffectScope>(wire).unwrap(),
            scope
        );

        video.effect_chains.push(super::VideoEffectChainSummary {
            id: super::VideoEffectChainId(72),
            scope: scope.clone(),
            bypassed: true,
            stages: Vec::new(),
        });
        assert!(super::validate_authored_video_effect_chains(&video)
            .unwrap_err()
            .contains("at most one chain"));
        video.effect_chains.pop();

        let mut zero_source = video.clone();
        zero_source.effect_chains[0].scope = super::VideoEffectScope::Transition {
            owner: super::VideoTransitionEffectOwner::TimelineFollow {
                source_timeline_id: super::TimelineId(0),
            },
        };
        assert!(super::validate_authored_video_effect_chains(&zero_source)
            .unwrap_err()
            .contains("Timeline ID zero"));

        let mut snapshot = super::EngineSnapshot {
            timeline: super::TimelineSnapshot {
                id: super::TimelineId(91),
                label: "Source".to_string(),
                ..super::TimelineSnapshot::default()
            },
            video: video.clone(),
            ..super::EngineSnapshot::default()
        };
        super::validate_timeline_bank(&snapshot, &[]).unwrap();

        snapshot.video.effect_chains[0].scope = super::VideoEffectScope::Transition {
            owner: super::VideoTransitionEffectOwner::TimelineFollow {
                source_timeline_id: super::TimelineId(92),
            },
        };
        assert!(super::validate_timeline_bank(&snapshot, &[])
            .unwrap_err()
            .contains("missing source Timeline 92"));

        snapshot.timeline_bank = vec![
            snapshot.timeline.clone(),
            super::TimelineSnapshot {
                id: super::TimelineId(92),
                label: "Target".to_string(),
                ..super::TimelineSnapshot::default()
            },
        ];
        super::validate_timeline_bank(&snapshot, &[]).unwrap();

        let legacy_clip_take: super::VideoEffectScope = serde_json::from_value(serde_json::json!({
            "scope": "transition",
            "owner": { "kind": "clip_take", "layer_id": 5 }
        }))
        .unwrap();
        assert_eq!(
            legacy_clip_take,
            super::VideoEffectScope::Transition {
                owner: super::VideoTransitionEffectOwner::ClipTake { layer_id: 5 }
            }
        );
    }

    #[test]
    fn video_transition_bus_authored_runtime_and_conflicts_are_exact() {
        let layers = (1..=3)
            .map(|id| {
                media_asset_test_layer(id, &format!("Layer {id}"), &format!("C:/show/{id}.mp4"))
            })
            .collect::<Vec<_>>();
        let group = super::VideoLayerGroupSummary {
            id: super::VideoLayerGroupId(11),
            label: "Backgrounds".to_string(),
            composition_id: 9,
            layer_ids: vec![1, 2],
        };
        let from = super::VideoLayerTransitionTarget::Group { group_id: group.id };
        let to = super::VideoLayerTransitionTarget::Layer { layer_id: 3 };
        let duration = super::VideoClipTakeDuration::milliseconds(1_000);
        let bus = super::VideoLayerTransitionBusSummary {
            id: super::VideoTransitionBusId(5),
            label: "Program Background".to_string(),
            composition_id: 9,
            enabled: true,
            members: vec![from.clone(), to.clone()],
            default_from: from.clone(),
            default_to: to.clone(),
            default_kind: super::VideoClipTakeKind::Wipe,
            default_duration: duration,
            default_curve: super::VideoLayerTransitionCurve::EaseInOut,
            matte_source: Some(super::VideoLayerTransitionTarget::Layer { layer_id: 2 }),
        };
        let mut video = super::VideoSnapshot {
            layers,
            layer_groups: vec![group],
            transition_buses: vec![bus.clone()],
            compositions: vec![super::CompositionSummary {
                id: 9,
                label: "Main".to_string(),
                layer_ids: vec![1, 2, 3],
                output_ids: Vec::new(),
            }],
            effect_chains: vec![super::VideoEffectChainSummary {
                id: super::VideoEffectChainId(21),
                scope: super::VideoEffectScope::Transition {
                    owner: super::VideoTransitionEffectOwner::LayerBus { bus_id: bus.id },
                },
                bypassed: false,
                stages: Vec::new(),
            }],
            ..super::VideoSnapshot::default()
        };
        super::validate_authored_video_effect_chains(&video).unwrap();
        let active = super::VideoLayerTransitionBusRuntimeSummary {
            bus_id: bus.id,
            origin_from: from.clone(),
            from: from.clone(),
            to: to.clone(),
            kind: super::VideoClipTakeKind::Wipe,
            curve: super::VideoLayerTransitionCurve::EaseInOut,
            elapsed_ms: 250,
            duration_ms: 1_000,
            duration,
            progress_millis: 250,
        };
        let runtime = super::VideoLayerTransitionRuntimeSnapshot {
            buses: vec![active.clone()],
        };
        super::validate_video_layer_transition_runtime(&runtime, &video).unwrap();

        let second_bus = super::VideoLayerTransitionBusSummary {
            id: super::VideoTransitionBusId(6),
            label: "Conflicting".to_string(),
            members: vec![from.clone(), to.clone()],
            default_from: from.clone(),
            default_to: to.clone(),
            matte_source: None,
            ..bus.clone()
        };
        video.transition_buses.push(second_bus.clone());
        let mut conflicting = active;
        conflicting.bus_id = second_bus.id;
        conflicting.origin_from = second_bus.default_from.clone();
        conflicting.from = second_bus.default_from;
        assert!(super::validate_video_layer_transition_runtime(
            &super::VideoLayerTransitionRuntimeSnapshot {
                buses: vec![runtime.buses[0].clone(), conflicting],
            },
            &video,
        )
        .unwrap_err()
        .contains("conflict"));

        let engine = super::EngineSnapshot {
            video_transition_runtime: runtime.clone(),
            ..Default::default()
        };
        assert!(serde_json::to_value(engine)
            .unwrap()
            .get("video_transition_runtime")
            .is_none());
    }

    #[test]
    fn video_effect_chain_migration_overflow_leaves_snapshot_unchanged() {
        let mut first = media_asset_test_layer(1, "No FX", "C:/show/one.mp4");
        let mut second = media_asset_test_layer(2, "FX", "C:/show/two.mp4");
        second.isf_effect = Some(clip_slot_test_effect());
        first.media_asset_id = Some(10);
        second.media_asset_id = Some(11);
        let mut video = super::VideoSnapshot {
            layers: vec![first, second],
            media_assets: vec![
                media_asset_test_asset(10, "One", "C:/show/one.mp4"),
                media_asset_test_asset(11, "Two", "C:/show/two.mp4"),
            ],
            effect_chains: vec![super::VideoEffectChainSummary {
                id: super::VideoEffectChainId(u64::MAX),
                scope: super::VideoEffectScope::Layer { layer_id: 1 },
                bypassed: false,
                stages: Vec::new(),
            }],
            ..super::VideoSnapshot::default()
        };
        let original = video.clone();
        assert!(super::normalize_legacy_video_effect_chains(&mut video).is_err());
        assert_eq!(video, original);
    }

    #[test]
    fn video_effect_chain_serde_defaults_do_not_pollute_legacy_snapshot() {
        let mut legacy = serde_json::to_value(super::VideoSnapshot::default()).unwrap();
        let object = legacy.as_object_mut().unwrap();
        object.remove("effect_chains");
        object.remove("effect_presets");
        object.remove("layer_groups");
        let parsed: super::VideoSnapshot = serde_json::from_value(legacy).unwrap();
        assert!(parsed.effect_chains.is_empty());
        assert!(parsed.effect_presets.is_empty());
        assert!(parsed.layer_groups.is_empty());
        assert!(!serde_json::to_string(&parsed)
            .unwrap()
            .contains("effect_chains"));

        let decoded: super::VideoEffectScope = serde_json::from_value(serde_json::json!({
            "scope": "clip",
            "layer_id": 1,
            "slot_id": 2
        }))
        .unwrap();
        assert_eq!(
            decoded,
            super::VideoEffectScope::Clip {
                layer_id: 1,
                slot_id: super::VideoClipSlotId(2)
            }
        );
    }

    #[test]
    fn dj_link_envelope_is_strict_and_canonical() {
        fn envelope_text(message_type: &str, payload: &serde_json::Value) -> String {
            format!(
                r#"{{"v":3,"type":"{message_type}","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{payload}}}"#
            )
        }
        let active = envelope_text(
            "DJ_TRACK_ACTIVE",
            &serde_json::json!({
                "deck": 1,
                "deckId": "rekordbox-deck-1",
                "contentId": "abc",
                "trackBpm": 128.0,
                "positionAtSendSec": 1.25,
                "effectiveBpm": 128.5,
                "positionRevision": 7,
                "sampleAgeMs": 42,
                "isPlaying": true,
                "startedAt": "2026-08-21T00:00:00Z",
                "playSessionId": "p1",
            }),
        );
        let envelope = super::DjLinkEnvelope::parse_json(&active).unwrap();
        assert_eq!(envelope.message_type, super::DjLinkMessageType::TrackActive);
        assert_eq!(envelope.agent_id, super::DJ_LINK_AGENT_ID);
        let shape = envelope.canonical_shape().unwrap();
        assert!(shape.contains("DJ_TRACK_ACTIVE"));
        assert!(super::DjLinkEnvelope::parse_json(&active.replace("\"v\":3", "\"v\":2")).is_err());
        assert!(super::DjLinkEnvelope::parse_json(&active.replace("\"v\":3", "\"v\":4")).is_err());
        assert!(super::DjLinkEnvelope::parse_json(
            &active.replace("rb-output-dj-agent", "other-agent")
        )
        .is_err());
        assert!(super::DjLinkEnvelope::parse_json(
            &active.replace("\"sequence\":1", "\"sequence\":0")
        )
        .is_err());
        assert!(super::DjLinkEnvelope::parse_json(
            &active.replace("\"payload\":", "\"extra\":1,\"payload\":")
        )
        .is_err());
        let mut missing = serde_json::from_str::<serde_json::Value>(&active).unwrap();
        missing.as_object_mut().unwrap().remove("eventId").unwrap();
        assert!(super::DjLinkEnvelope::parse_json(&missing.to_string()).is_err());
    }

    #[test]
    fn dj_link_hello_debug_is_redacted_in_normal_and_pretty_output() {
        let sentinel = "dj-link-auth-token-must-never-reach-debug";
        let hello = super::DjLinkHelloPayload {
            auth_token: sentinel.to_string(),
            version: super::DJ_LINK_PROTOCOL_VERSION,
            capabilities: vec!["track-active".to_string()],
        };
        let envelope = super::DjLinkEnvelope {
            v: super::DJ_LINK_PROTOCOL_VERSION,
            message_type: super::DjLinkMessageType::Hello,
            agent_id: super::DJ_LINK_AGENT_ID.to_string(),
            session_id: "session-1".to_string(),
            sequence: 1,
            event_id: "hello-1".to_string(),
            payload: serde_json::json!({"authToken": sentinel}),
        };
        for rendered in [
            format!("{hello:?}"),
            format!("{hello:#?}"),
            format!("{envelope:?}"),
            format!("{envelope:#?}"),
        ] {
            assert!(
                !rendered.contains(sentinel),
                "Debug leaked credential: {rendered}"
            );
            assert!(
                rendered.contains("REDACTED"),
                "Debug did not mark redaction: {rendered}"
            );
        }
    }

    #[test]
    fn dj_link_peer_wire_fixtures_are_strict_and_distinct() {
        fn envelope_text(message_type: &str, payload: &serde_json::Value) -> String {
            format!(
                r#"{{"v":3,"type":"{message_type}","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{payload}}}"#
            )
        }
        let hello = envelope_text(
            "DJ_AGENT_HELLO",
            &serde_json::json!({
                "authToken": "0123456789abcdef0123456789abcdef",
                "version": 3,
                "capabilities": super::DJ_LINK_REQUIRED_CAPABILITIES,
            }),
        );
        assert_eq!(
            super::DjLinkEnvelope::parse_json(&hello)
                .unwrap()
                .message_type,
            super::DjLinkMessageType::Hello
        );
        let sync = envelope_text(
            "DJ_STATE_SYNC",
            &serde_json::json!({
                "released": false,
                "ownerDeck": 2,
                "ownerDeckId": "rekordbox-deck-2",
                "activePlaySessionId": "play-1",
            }),
        );
        let sync_envelope = super::DjLinkEnvelope::parse_json(&sync).unwrap();
        assert_eq!(
            sync_envelope.message_type,
            super::DjLinkMessageType::StateSync
        );
        let release = envelope_text(
            "DJ_RELEASE",
            &serde_json::json!({"state":"released","timelineId":"show-1","playSessionId":"p1"}),
        );
        assert_eq!(
            super::DjLinkEnvelope::parse_json(&release)
                .unwrap()
                .message_type,
            super::DjLinkMessageType::Release
        );
        let loop_state = envelope_text(
            "DJ_LOOP_STATE",
            &serde_json::json!({
                "deck": 1,
                "deckId": "rekordbox-deck-1",
                "playSessionId": "p1",
                "loop": {
                    "active": true,
                    "startBeat": 16.0,
                    "endBeat": 32.0,
                    "lengthBeats": 16.0,
                    "revision": 1,
                    "sampleAgeMs": 30,
                    "source": "rekordbox-hook-measured",
                },
            }),
        );
        assert_eq!(
            super::DjLinkEnvelope::parse_json(&loop_state)
                .unwrap()
                .message_type,
            super::DjLinkMessageType::LoopState
        );
        let loop_fallback = envelope_text(
            "DJ_LOOP_FALLBACK",
            &serde_json::json!({
                "deck": 1,
                "deckId": "rekordbox-deck-1",
                "playSessionId": "p1",
                "pedalIntentId": 1,
                "baseMeasuredLoopRevision": null,
                "baseLoopDivision": 8,
                "targetLengthBeats": 1.0 / 64.0,
                "responseWindowMs": 250,
                "source": "pedal-no-response-predicted",
            }),
        );
        assert_eq!(
            super::DjLinkEnvelope::parse_json(&loop_fallback)
                .unwrap()
                .message_type,
            super::DjLinkMessageType::LoopFallback
        );
        assert!(
            super::DjLinkEnvelope::parse_json(&hello.replace("DJ_AGENT_HELLO", "HELLO")).is_err()
        );
        assert!(super::DjLinkEnvelope::parse_json(
            &hello.replace("rb-output-dj-agent", "generic-json")
        )
        .is_err());
    }

    #[test]
    fn dj_link_timeline_pedal_commands_require_exact_play_session_fence() {
        fn envelope_text(message_type: &str, payload: serde_json::Value) -> String {
            format!(
                r#"{{"v":3,"type":"{message_type}","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{payload}}}"#
            )
        }

        let cases = [
            (
                "DJ_TIMELINE_BEAT_JUMP",
                serde_json::json!({
                    "bars": 4,
                    "timelineId": "show-1",
                    "playSessionId": "play-1",
                }),
            ),
            (
                "DJ_TIMELINE_LOOP_SET",
                serde_json::json!({
                    "active": true,
                    "timelineId": "show-1",
                    "playSessionId": "play-1",
                }),
            ),
            (
                "DJ_TIMELINE_LOOP_HALF",
                serde_json::json!({
                    "timelineId": "show-1",
                    "playSessionId": "play-1",
                }),
            ),
        ];

        for (message_type, valid_payload) in cases {
            let canonical = super::DjLinkEnvelope::parse_json(&envelope_text(
                message_type,
                valid_payload.clone(),
            ))
            .unwrap()
            .canonical_shape()
            .unwrap();
            assert!(canonical.contains(r#""playSessionId":"play-1""#));

            let mut missing = valid_payload.clone();
            missing
                .as_object_mut()
                .unwrap()
                .remove("playSessionId")
                .unwrap();
            assert!(
                super::DjLinkEnvelope::parse_json(&envelope_text(message_type, missing)).is_err()
            );

            let mut empty = valid_payload.clone();
            empty["playSessionId"] = serde_json::json!("");
            assert!(
                super::DjLinkEnvelope::parse_json(&envelope_text(message_type, empty)).is_err()
            );

            for invalid_session_id in ["   ", " play-1", "play-1 ", "\u{2003}play-1"] {
                let mut noncanonical = valid_payload.clone();
                noncanonical["playSessionId"] = serde_json::json!(invalid_session_id);
                assert!(
                    super::DjLinkEnvelope::parse_json(&envelope_text(message_type, noncanonical))
                        .is_err(),
                    "{message_type} must reject noncanonical playSessionId {invalid_session_id:?}"
                );
            }

            let mut interior_whitespace = valid_payload.clone();
            interior_whitespace["playSessionId"] = serde_json::json!("play 1");
            let preserved = super::DjLinkEnvelope::parse_json(&envelope_text(
                message_type,
                interior_whitespace,
            ))
            .unwrap()
            .canonical_shape()
            .unwrap();
            assert!(preserved.contains(r#""playSessionId":"play 1""#));

            let mut legacy_alias = valid_payload.clone();
            let session = legacy_alias
                .as_object_mut()
                .unwrap()
                .remove("playSessionId")
                .unwrap();
            legacy_alias["play_session_id"] = session;
            assert!(
                super::DjLinkEnvelope::parse_json(&envelope_text(message_type, legacy_alias,))
                    .is_err()
            );

            let mut unknown = valid_payload;
            unknown["legacyPedalCommand"] = serde_json::json!(true);
            assert!(
                super::DjLinkEnvelope::parse_json(&envelope_text(message_type, unknown)).is_err()
            );
        }

        for bars in [-4, 3] {
            let error = super::DjLinkEnvelope::parse_json(&envelope_text(
                "DJ_TIMELINE_BEAT_JUMP",
                serde_json::json!({
                    "bars": bars,
                    "timelineId": "show-1",
                    "playSessionId": "play-1",
                }),
            ))
            .unwrap_err();
            assert_eq!(error, "DJ timeline beat jump must be exactly +4 bars");
        }
    }

    #[test]
    fn dj_link_duplicate_keys_are_rejected_before_serde_at_every_level() {
        let base = |payload: &str| {
            format!(
                r#"{{"v":3,"type":"DJ_HEARTBEAT","agentId":"rb-output-dj-agent","sessionId":"s1",{payload}"sequence":1,"eventId":"e1","payload":{{}}}}"#
            )
        };
        assert!(super::DjLinkEnvelope::parse_json(&base("")).is_ok());
        assert!(
            super::DjLinkEnvelope::parse_json(&base(r#""sequence":9, "#))
                .err()
                .is_some_and(|error| error.contains("duplicate")),
            "root-level duplicate must be detected before serde"
        );

        let nested = format!(
            r#"{{"v":3,"type":"DJ_STATE_SYNC","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{{"released":false,"released":true}}}}"#
        );
        assert!(super::DjLinkEnvelope::parse_json(&nested)
            .err()
            .is_some_and(|error| error.contains("duplicate")));

        let deep = format!(
            r#"{{"v":3,"type":"DJ_TRACK_SYNC","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{{"deck":1,"deckId":"rekordbox-deck-1","contentId":"c","trackBpm":null,"positionAtSendSec":0,"effectiveBpm":120,"positionRevision":1,"sampleAgeMs":0,"isPlaying":true,"startedAt":"2026-08-25T00:00:00Z","playSessionId":"p1","loop":{{"active":false,"startBeat":null,"endBeat":null,"lengthBeats":null,"revision":1,"revision":2,"sampleAgeMs":0,"source":"rekordbox-hook-measured"}}}}}}"#
        );
        assert!(super::DjLinkEnvelope::parse_json(&deep)
            .err()
            .is_some_and(|error| error.contains("duplicate")));

        let escaped = format!(
            r#"{{"v":3,"type":"DJ_HEARTBEAT","agentId":"rb-output-dj-agent","agentId":"other","sessionId":"s1","sequence":1,"eventId":"e1","payload":{{}}}}"#
        );
        assert!(super::DjLinkEnvelope::parse_json(&escaped).is_err());
        let unicode_escaped = format!(
            r#"{{"v":3,"\u0074ype":"DJ_HEARTBEAT","type":"DJ_HEARTBEAT","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{{}}}}"#
        );
        assert!(super::DjLinkEnvelope::parse_json(&unicode_escaped).is_err());
    }

    #[test]
    fn dj_link_track_payload_bounds_and_identity_are_fail_closed() {
        let payload = serde_json::json!({
            "deck": 1,
            "deckId": "rekordbox-deck-1",
            "contentId": "abc",
            "trackBpm": 128.0,
            "positionAtSendSec": 1.25,
            "effectiveBpm": 128.5,
            "positionRevision": 7,
            "sampleAgeMs": 42,
            "isPlaying": true,
            "startedAt": "2026-08-21T00:00:00Z",
            "playSessionId": "p1",
        });
        let envelope = |payload: &serde_json::Value| {
            format!(
                r#"{{"v":3,"type":"DJ_TRACK_SYNC","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{payload}}}"#
            )
        };
        assert!(super::DjLinkEnvelope::parse_json(&envelope(&payload)).is_ok());

        let reject = |mutant: serde_json::Value| {
            assert!(
                super::DjLinkEnvelope::parse_json(&envelope(&mutant)).is_err(),
                "expected rejection for {mutant}"
            );
        };
        let mut variant = payload.clone();
        variant["contentId"] = serde_json::json!("abc");
        variant["title"] = serde_json::json!("Title");
        reject(variant);
        let mut variant = payload.clone();
        variant
            .as_object_mut()
            .unwrap()
            .remove("contentId")
            .unwrap();
        reject(variant);
        let mut variant = payload.clone();
        variant["title"] = serde_json::json!("Title");
        reject(variant);
        let mut variant = payload.clone();
        variant["sampleAgeMs"] = serde_json::json!(1501);
        reject(variant);
        let mut variant = payload.clone();
        variant["trackBpm"] = serde_json::json!(1000.5);
        reject(variant);
        let mut variant = payload.clone();
        variant["effectiveBpm"] = serde_json::json!(0);
        reject(variant);
        let mut variant = payload.clone();
        variant["positionAtSendSec"] = serde_json::json!(7200.5);
        reject(variant);
        let mut variant = payload.clone();
        variant["positionAtSendSec"] = serde_json::json!(-0.5);
        reject(variant);
        let mut variant = payload.clone();
        variant["deckId"] = serde_json::json!("rekordbox-deck-2");
        reject(variant);
        let mut variant = payload.clone();
        variant["deck"] = serde_json::json!(5);
        reject(variant);
        let mut variant = payload.clone();
        variant["isPlaying"] = serde_json::json!(false);
        reject(variant);
        let mut variant = payload.clone();
        variant["startedAt"] = serde_json::json!("not-a-timestamp");
        reject(variant);
        let mut variant = payload.clone();
        variant["positionRevision"] = serde_json::json!(super::DJ_LINK_MAX_SEQUENCE + 1);
        reject(variant);
        let mut variant = payload.clone();
        variant["unknownField"] = serde_json::json!(1);
        reject(variant);

        let mut accepted = payload.clone();
        accepted
            .as_object_mut()
            .unwrap()
            .remove("contentId")
            .unwrap();
        accepted["title"] = serde_json::json!("Title");
        accepted["artist"] = serde_json::json!("Artist");
        assert!(super::DjLinkEnvelope::parse_json(&envelope(&accepted)).is_ok());
    }

    #[test]
    fn dj_link_measured_loop_rules_are_exact() {
        let loop_of = |loop_payload: serde_json::Value| {
            format!(
                r#"{{"v":3,"type":"DJ_TRACK_ACTIVE","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{{"deck":1,"deckId":"rekordbox-deck-1","contentId":"abc","trackBpm":null,"positionAtSendSec":10,"effectiveBpm":120,"positionRevision":7,"sampleAgeMs":20,"isPlaying":true,"startedAt":"2026-08-25T00:00:00+09:00","playSessionId":"p1","loop":{loop_payload}}}}}"#
            )
        };
        let valid_active = serde_json::json!({
            "active": true,
            "startBeat": 16.0,
            "endBeat": 32.0,
            "lengthBeats": 16.0,
            "revision": 2,
            "sampleAgeMs": 11,
            "source": "rekordbox-hook-measured",
        });
        assert!(super::DjLinkEnvelope::parse_json(&loop_of(valid_active.clone())).is_ok());
        let valid_inactive = serde_json::json!({
            "active": false,
            "startBeat": null,
            "endBeat": null,
            "lengthBeats": null,
            "revision": 2,
            "sampleAgeMs": 11,
            "source": "rekordbox-hook-measured",
        });
        assert!(super::DjLinkEnvelope::parse_json(&loop_of(valid_inactive.clone())).is_ok());

        let reject = |loop_payload: serde_json::Value| {
            assert!(
                super::DjLinkEnvelope::parse_json(&loop_of(loop_payload.clone())).is_err(),
                "expected rejection for {loop_payload}"
            );
        };
        let mut variant = valid_inactive.clone();
        variant["active"] = serde_json::json!(true);
        reject(variant);
        let mut variant = valid_active.clone();
        variant["active"] = serde_json::json!(false);
        reject(variant);
        let mut variant = valid_active.clone();
        variant["endBeat"] = serde_json::json!(16.0);
        reject(variant);
        let mut variant = valid_active.clone();
        variant["lengthBeats"] = serde_json::json!(17.0);
        reject(variant);
        let mut variant = valid_active.clone();
        variant["lengthBeats"] = serde_json::json!(16.002);
        reject(variant);
        let mut variant = valid_active.clone();
        variant["source"] = serde_json::json!("guessed");
        reject(variant);
        let mut variant = valid_active.clone();
        variant["revision"] = serde_json::json!(0);
        reject(variant);
        let mut variant = valid_active.clone();
        variant["revision"] = serde_json::json!(super::DJ_LINK_MAX_SEQUENCE + 1);
        reject(variant);
        let mut variant = valid_active.clone();
        variant["sampleAgeMs"] = serde_json::json!(1501);
        reject(variant);
        let mut variant = valid_active.clone();
        variant["startBeat"] = serde_json::json!(-1.0);
        variant["endBeat"] = serde_json::json!(15.0);
        reject(variant);
        let boundary = valid_active.clone();
        let mut variant = boundary;
        variant["lengthBeats"] = serde_json::json!(16.0005);
        assert!(super::DjLinkEnvelope::parse_json(&loop_of(variant)).is_ok());
    }

    #[test]
    fn dj_link_loop_fallback_rules_are_exact_and_bounded() {
        let envelope_of = |payload: &serde_json::Value| {
            format!(
                r#"{{"v":3,"type":"DJ_LOOP_FALLBACK","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{payload}}}"#
            )
        };
        let payload = serde_json::json!({
            "deck": 1,
            "deckId": "rekordbox-deck-1",
            "playSessionId": "p1",
            "pedalIntentId": 1,
            "baseMeasuredLoopRevision": null,
            "baseLoopDivision": 0,
            "targetLengthBeats": 4.0,
            "responseWindowMs": 250,
            "source": "pedal-no-response-predicted",
        });
        for (index, target) in super::DJ_LINK_LOOP_PROFILE_LENGTH_BEATS
            .into_iter()
            .enumerate()
        {
            let mut variant = payload.clone();
            variant["targetLengthBeats"] = serde_json::json!(target);
            variant["baseLoopDivision"] = if index == 0 {
                serde_json::Value::Null
            } else {
                serde_json::json!(index - 1)
            };
            assert!(
                super::DjLinkEnvelope::parse_json(&envelope_of(&variant)).is_ok(),
                "supported target {target} must be accepted"
            );
        }
        for response_window_ms in [
            super::DJ_LINK_LOOP_FALLBACK_MIN_RESPONSE_WINDOW_MS,
            super::DJ_LINK_LOOP_FALLBACK_MAX_RESPONSE_WINDOW_MS,
        ] {
            let mut variant = payload.clone();
            variant["responseWindowMs"] = serde_json::json!(response_window_ms);
            assert!(super::DjLinkEnvelope::parse_json(&envelope_of(&variant)).is_ok());
        }

        let reject = |mutant: serde_json::Value| {
            assert!(
                super::DjLinkEnvelope::parse_json(&envelope_of(&mutant)).is_err(),
                "expected rejection for {mutant}"
            );
        };
        for target in [16.0, 3.0, 1.0 / 128.0, f64::NAN, f64::INFINITY] {
            let mut variant = payload.clone();
            variant["targetLengthBeats"] = serde_json::json!(target);
            reject(variant);
        }
        let mut variant = payload.clone();
        variant["targetLengthBeats"] =
            serde_json::json!(4.0 + (super::DJ_LINK_LOOP_LENGTH_TOLERANCE_BEATS / 2.0));
        reject(variant);
        let mut variant = payload.clone();
        variant["responseWindowMs"] =
            serde_json::json!(super::DJ_LINK_LOOP_FALLBACK_MIN_RESPONSE_WINDOW_MS - 1);
        reject(variant);
        let mut variant = payload.clone();
        variant["responseWindowMs"] =
            serde_json::json!(super::DJ_LINK_LOOP_FALLBACK_MAX_RESPONSE_WINDOW_MS + 1);
        reject(variant);
        let mut variant = payload.clone();
        variant["source"] = serde_json::json!(super::DJ_LINK_MEASURED_LOOP_SOURCE);
        reject(variant);
        let mut variant = payload.clone();
        variant["deckId"] = serde_json::json!("rekordbox-deck-2");
        reject(variant);
        let mut variant = payload.clone();
        variant["playSessionId"] = serde_json::json!(" ");
        reject(variant);
        let mut variant = payload.clone();
        variant["pedalIntentId"] = serde_json::json!(0);
        reject(variant);
        let mut variant = payload.clone();
        variant["baseMeasuredLoopRevision"] = serde_json::json!(0);
        reject(variant);
        let mut variant = payload.clone();
        variant["baseMeasuredLoopRevision"] = serde_json::json!(super::DJ_LINK_MAX_SEQUENCE + 1);
        reject(variant);
        let mut variant = payload.clone();
        variant["baseLoopDivision"] = serde_json::json!(10);
        reject(variant);
        let mut variant = payload.clone();
        variant["legacyMeasured"] = serde_json::json!(false);
        reject(variant);
    }

    #[test]
    fn dj_link_hello_token_version_and_capabilities_are_exact() {
        let hello = |token: &str, version: u8, capabilities: &[&str]| {
            format!(
                r#"{{"v":3,"type":"DJ_AGENT_HELLO","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{{"authToken":"{token}","version":{version},"capabilities":[{}]}}}}"#,
                capabilities
                    .iter()
                    .map(|capability| format!(r#""{capability}""#))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let generic_set = super::DJ_LINK_REQUIRED_CAPABILITIES;
        assert!(super::DjLinkEnvelope::parse_json(&hello(
            "0123456789abcdef0123456789abcdef",
            3,
            &generic_set
        ))
        .is_ok());
        assert!(generic_set.contains(&"DJ_TRACK_ACTIVE"));
        assert!(generic_set.contains(&"DJ_TRACK_SYNC"));
        assert!(generic_set.contains(&"DJ_TIMELINE_LOOP_HALF"));
        assert!(!generic_set.contains(&"DJ_MASTER_TRACK_ACTIVE"));
        assert!(super::DjLinkEnvelope::parse_json(&hello(
            "0123456789abcdef0123456789abcdef",
            3,
            &[
                "DJ_MASTER_TRACK_ACTIVE",
                "DJ_MASTER_TRACK_SYNC",
                "DJ_LOOP_STATE",
                "DJ_LOOP_FALLBACK",
                "DJ_RELEASE",
                "DJ_TIMELINE_BEAT_JUMP",
                "DJ_TIMELINE_LOOP_SET",
                "DJ_TIMELINE_STATE_REQUEST",
                "DJ_STATE_SYNC",
            ]
        ))
        .is_err());
        assert!(super::DjLinkEnvelope::parse_json(&hello("too-short", 3, &generic_set)).is_err());
        assert!(
            super::DjLinkEnvelope::parse_json(&hello(&"a".repeat(257), 3, &generic_set)).is_err()
        );
        assert!(super::DjLinkEnvelope::parse_json(&hello(
            "0123456789abcdef 0123456789abcdef",
            3,
            &generic_set
        ))
        .is_err());
        assert!(super::DjLinkEnvelope::parse_json(&hello(
            "0123456789abcdef\t0123456789abcdef",
            3,
            &generic_set
        ))
        .is_err());
        assert!(super::DjLinkEnvelope::parse_json(&hello(
            "0123456789abcdef0123456789abcdef",
            2,
            &generic_set
        ))
        .is_err());
        assert!(super::DjLinkEnvelope::parse_json(&hello(
            "0123456789abcdef0123456789abcdef",
            3,
            &generic_set[..8]
        ))
        .is_err());
        let mut extra = generic_set.to_vec();
        extra.push("DJ_EXTRA");
        assert!(super::DjLinkEnvelope::parse_json(&hello(
            "0123456789abcdef0123456789abcdef",
            3,
            &extra
        ))
        .is_err());
        let mut duplicated = generic_set.to_vec();
        duplicated[9] = duplicated[0];
        assert!(super::DjLinkEnvelope::parse_json(&hello(
            "0123456789abcdef0123456789abcdef",
            3,
            &duplicated
        ))
        .is_err());
        let mut mixed = generic_set.to_vec();
        mixed[0] = "DJ_MASTER_TRACK_ACTIVE";
        assert!(super::DjLinkEnvelope::parse_json(&hello(
            "0123456789abcdef0123456789abcdef",
            3,
            &mixed
        ))
        .is_err());
    }

    #[test]
    fn dj_link_empty_payload_types_reject_extra_keys_and_ack_wire_is_exact() {
        for message_type in ["DJ_HEARTBEAT", "DJ_TIMELINE_STATE_REQUEST"] {
            let empty = format!(
                r#"{{"v":3,"type":"{message_type}","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{{}}}}"#
            );
            assert!(
                super::DjLinkEnvelope::parse_json(&empty).is_ok(),
                "{message_type}"
            );
            let extra = format!(
                r#"{{"v":3,"type":"{message_type}","agentId":"rb-output-dj-agent","sessionId":"s1","sequence":1,"eventId":"e1","payload":{{"at":"now"}}}}"#
            );
            assert!(
                super::DjLinkEnvelope::parse_json(&extra).is_err(),
                "{message_type}"
            );
        }

        let ack = super::DjLinkAck {
            v: super::DJ_LINK_PROTOCOL_VERSION,
            message_type: "ACK".to_string(),
            event_id: "ack-event".to_string(),
            sequence: 9,
            outcome: super::DjLinkAckOutcome::Busy,
            code: Some("in_flight".to_string()),
            state_generation: 17,
        };
        let wire: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&ack).unwrap()).unwrap();
        assert_eq!(
            wire,
            serde_json::json!({
                "v": 3,
                "type": "ACK",
                "eventId": "ack-event",
                "sequence": 9,
                "outcome": "busy",
                "code": "in_flight",
                "stateGeneration": 17,
            })
        );
        let accepted = super::DjLinkAck {
            outcome: super::DjLinkAckOutcome::Accepted,
            code: None,
            ..ack
        };
        let wire: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&accepted).unwrap()).unwrap();
        assert_eq!(wire["code"], serde_json::json!(null));
        assert_eq!(
            wire.as_object().unwrap().len(),
            7,
            "ACK wire shape must be exactly seven keys"
        );
    }

    #[test]
    fn dj_link_timeline_state_output_envelope_is_exact_v3() {
        let state = super::DjLinkTimelineState {
            message_type: "DJ_TIMELINE_STATE".to_string(),
            event_id: "state-event".to_string(),
            sequence: 3,
            state: super::DjLinkTimelineStateValue::Running,
            loop_active: false,
            transition_hold_active: false,
            timeline_id: "show-1".to_string(),
            position_bars: 16,
            play_session_id: Some("play-1".to_string()),
            pedal_owner: None,
            release_event_id: None,
            operator_return_request_id: None,
        };
        state.validate().unwrap();
        let wire = serde_json::to_value(&state).unwrap();
        let keys = wire.as_object().unwrap();
        for key in [
            "type",
            "eventId",
            "sequence",
            "state",
            "loopActive",
            "transitionHoldActive",
            "timelineId",
            "positionBars",
            "playSessionId",
            "pedalOwner",
            "releaseEventId",
            "operatorReturnRequestId",
        ] {
            assert!(keys.contains_key(key), "missing output key {key}");
        }
        assert_eq!(wire["pedalOwner"], serde_json::json!(null));
        let mut missing_operator_return = wire.clone();
        missing_operator_return
            .as_object_mut()
            .unwrap()
            .remove("operatorReturnRequestId");
        assert!(
            serde_json::from_value::<super::DjLinkTimelineState>(missing_operator_return).is_err(),
            "current v3 timeline state must reject an omitted operatorReturnRequestId"
        );
    }

    #[test]
    fn dj_link_operator_return_request_id_requires_exact_epoch_and_counter() {
        let valid = "syndocal-dj-operator-return-0123456789abcdef0123456789abcdef-1";
        super::validate_dj_link_operator_return_request_id(valid).unwrap();
        super::validate_dj_link_operator_return_request_id(
            "syndocal-dj-operator-return-ffffffffffffffffffffffffffffffff-18446744073709551615",
        )
        .unwrap();

        for invalid in [
            "return-request-1",
            "syndocal-dj-operator-return-1",
            "syndocal-dj-operator-return-0123456789ABCDEF0123456789abcdef-1",
            "syndocal-dj-operator-return-0123456789abcdef0123456789abcde-1",
            "syndocal-dj-operator-return-0123456789abcdef0123456789abcdef-0",
            "syndocal-dj-operator-return-0123456789abcdef0123456789abcdef-01",
            "syndocal-dj-operator-return-0123456789abcdef0123456789abcdef-18446744073709551616",
        ] {
            assert!(
                super::validate_dj_link_operator_return_request_id(invalid).is_err(),
                "noncanonical operator return ID must fail closed: {invalid}"
            );
        }
    }

    #[test]
    fn dj_link_backend_token_is_not_deserialized_or_serialized() {
        let config = super::RemoteControlConfig {
            bind_ip: "127.0.0.1".to_string(),
            port: 9_100,
            pairing_pin: "123456".to_string(),
            dj_link_enabled: true,
            dj_link_bind_ip: Some("127.0.0.1".to_string()),
            dj_link_token: Some("0123456789abcdef0123456789abcdef".to_string()),
            ..super::RemoteControlConfig::default()
        };
        let encoded = serde_json::to_value(&config).unwrap();
        assert!(encoded.get("dj_link_token").is_none());
        let mut incoming = encoded;
        incoming["dj_link_token"] =
            serde_json::json!("renderer-selected-token-that-must-be-ignored");
        let decoded: super::RemoteControlConfig = serde_json::from_value(incoming).unwrap();
        assert!(decoded.dj_link_token.is_none());
    }

    #[test]
    fn remote_control_config_legacy_defaults_web_remote_enabled() {
        // A legacy/current config payload predating the switch must keep the
        // generic Web Remote enabled.
        let legacy = serde_json::json!({
            "bind_ip": "127.0.0.1",
            "port": 9_100,
            "pairing_pin": "123456",
            "allow_lan": false,
            "dj_link_enabled": false,
        });
        let decoded: super::RemoteControlConfig = serde_json::from_value(legacy).unwrap();
        assert!(decoded.web_remote_enabled);
        assert!(super::RemoteControlConfig::default().web_remote_enabled);

        let explicit = serde_json::json!({
            "bind_ip": "127.0.0.1",
            "port": 9_100,
            "web_remote_enabled": false,
            "dj_link_enabled": true,
        });
        let decoded: super::RemoteControlConfig = serde_json::from_value(explicit).unwrap();
        assert!(!decoded.web_remote_enabled);
        assert!(decoded.dj_link_enabled);
        let encoded = serde_json::to_value(&decoded).unwrap();
        assert_eq!(encoded["web_remote_enabled"], serde_json::json!(false));

        // Status is additive too: legacy payloads missing the mode field load
        // as stopped/disabled rather than inheriting a stale default of true.
        let status: super::RemoteControlStatus = serde_json::from_value(serde_json::json!({
            "running": false,
            "active_connections": 0,
            "rejected_connections": 0,
            "clients": [],
        }))
        .unwrap();
        assert!(!status.web_remote_enabled);
        assert!(!super::RemoteControlStatus::default().web_remote_enabled);
        let running: super::RemoteControlStatus = serde_json::from_value(serde_json::json!({
            "running": true,
            "active_connections": 0,
            "rejected_connections": 0,
            "clients": [],
            "web_remote_enabled": true,
        }))
        .unwrap();
        assert!(running.web_remote_enabled);
    }

    #[test]
    fn remote_control_config_debug_redacts_credentials() {
        let pin = "429913";
        let token = "secret-dj-link-token-0123456789abcdef";
        let config = super::RemoteControlConfig {
            pairing_pin: pin.to_string(),
            dj_link_enabled: true,
            dj_link_bind_ip: Some("127.0.0.1".to_string()),
            dj_link_token: Some(token.to_string()),
            ..super::RemoteControlConfig::default()
        };
        for rendered in [format!("{config:?}"), format!("{config:#?}")] {
            assert!(!rendered.contains(pin), "Debug leaked pairing PIN");
            assert!(!rendered.contains(token), "Debug leaked DJ Link token");
            assert!(rendered.contains("RemoteControlConfig"));
        }
        let compact = format!("{config:?}");
        // Presence-safe rendering only: set credentials appear as a stable
        // redaction marker, unset ones as an explicit absence marker.
        assert!(compact.contains("pairing_pin: \"redacted\""));
        assert!(compact.contains("dj_link_token: \"redacted\""));

        let empty = super::RemoteControlConfig::default();
        let rendered = format!("{empty:?}");
        assert!(rendered.contains("pairing_pin: \"unset\""));
        assert!(rendered.contains("dj_link_token: \"unset\""));
    }

    #[test]
    fn video_output_monitor_identity_is_persisted_and_legacy_missing_is_stale() {
        let identity = "a".repeat(64);
        let summary = super::VideoOutputSummary {
            id: 7,
            label: "Display 2".to_string(),
            kind: super::VideoOutputKind::Display,
            enabled: true,
            composition_id: 1,
            fullscreen: true,
            monitor_id: Some(1),
            monitor_identity: Some(identity.clone()),
            width: 1920,
            height: 1080,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: super::VideoOutputMapping::default(),
        };
        let mut legacy = serde_json::to_value(&summary).unwrap();
        legacy.as_object_mut().unwrap().remove("monitor_identity");
        let parsed_legacy: super::VideoOutputSummary = serde_json::from_value(legacy).unwrap();
        assert_eq!(parsed_legacy.monitor_id, Some(1));
        assert_eq!(parsed_legacy.monitor_identity, None);

        let round_trip: super::VideoOutputSummary =
            serde_json::from_value(serde_json::to_value(&summary).unwrap()).unwrap();
        assert_eq!(
            round_trip.monitor_identity.as_deref(),
            Some(identity.as_str())
        );
    }

    #[test]
    fn dj_link_selector_priority_and_ambiguity_are_fail_closed() {
        let content = super::DjTrackSelector {
            content_id: Some("  track-1 ".to_string()),
            title: Some("ignored".to_string()),
            artist: Some("ignored".to_string()),
            title_contains: None,
            fallback_deck: None,
        };
        assert_eq!(content.canonical_key().unwrap(), "content:track-1");
        let title_artist = super::DjTrackSelector {
            content_id: None,
            title: Some(" Track ".to_string()),
            artist: Some(" Artist ".to_string()),
            title_contains: None,
            fallback_deck: None,
        };
        assert!(title_artist.canonical_key().unwrap().contains("Track"));
        assert!(super::validate_dj_track_trigger_mappings(&[
            super::DjTrackTriggerMapping {
                id: "a".to_string(),
                selector: title_artist.clone(),
                timeline_id: super::TimelineId(1),
                retrigger: super::DjTrackRetriggerPolicy::OncePerPlaySession,
            },
            super::DjTrackTriggerMapping {
                id: "b".to_string(),
                selector: title_artist,
                timeline_id: super::TimelineId(2),
                retrigger: super::DjTrackRetriggerPolicy::OncePerPlaySession,
            },
        ])
        .is_err());
        assert!(super::DjTrackSelector {
            content_id: None,
            title: Some("only title".to_string()),
            artist: None,
            title_contains: None,
            fallback_deck: None,
        }
        .canonical_key()
        .is_err());

        let title_contains = super::DjTrackSelector {
            content_id: None,
            title: None,
            artist: None,
            title_contains: Some(
                "  \u{4eba}\u{751f}\u{30aa}\u{30fc}\u{30d0}\u{30fc}  ".to_string(),
            ),
            fallback_deck: Some(1),
        };
        assert_eq!(
            title_contains.canonical_key().unwrap(),
            "title_contains:\u{4eba}\u{751f}\u{30aa}\u{30fc}\u{30d0}\u{30fc}"
        );
        for invalid in [
            super::DjTrackSelector {
                content_id: Some("content".to_string()),
                title: None,
                artist: None,
                title_contains: Some("needle".to_string()),
                fallback_deck: None,
            },
            super::DjTrackSelector {
                content_id: None,
                title: None,
                artist: None,
                title_contains: Some("needle".to_string()),
                fallback_deck: Some(2),
            },
            super::DjTrackSelector {
                content_id: None,
                title: None,
                artist: None,
                title_contains: None,
                fallback_deck: Some(1),
            },
        ] {
            assert!(invalid.canonical_key().is_err());
        }
        assert!(super::validate_dj_track_trigger_mappings(&[
            super::DjTrackTriggerMapping {
                id: "contains-a".to_string(),
                selector: super::DjTrackSelector {
                    content_id: None,
                    title: None,
                    artist: None,
                    title_contains: Some(
                        "\u{4eba}\u{751f}\u{30aa}\u{30fc}\u{30d0}\u{30fc}".to_string()
                    ),
                    fallback_deck: None,
                },
                timeline_id: super::TimelineId(3),
                retrigger: super::DjTrackRetriggerPolicy::OncePerPlaySession,
            },
            super::DjTrackTriggerMapping {
                id: "contains-b".to_string(),
                selector: super::DjTrackSelector {
                    content_id: None,
                    title: None,
                    artist: None,
                    title_contains: Some(
                        "  \u{4eba}\u{751f}\u{30aa}\u{30fc}\u{30d0}\u{30fc}  ".to_string()
                    ),
                    fallback_deck: None,
                },
                timeline_id: super::TimelineId(4),
                retrigger: super::DjTrackRetriggerPolicy::OncePerPlaySession,
            },
        ])
        .is_err());
    }

    #[test]
    fn dj_link_title_contains_selector_round_trips_and_rejects_unknown_fields() {
        let value = serde_json::json!({
            "titleContains": "\u{4eba}\u{751f}\u{30aa}\u{30fc}\u{30d0}\u{30fc}",
            "fallbackDeck": 1
        });
        let selector: super::DjTrackSelector = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            selector.title_contains.as_deref(),
            Some("\u{4eba}\u{751f}\u{30aa}\u{30fc}\u{30d0}\u{30fc}")
        );
        assert_eq!(selector.fallback_deck, Some(1));
        let round_trip: super::DjTrackSelector =
            serde_json::from_value(serde_json::to_value(&selector).unwrap()).unwrap();
        assert_eq!(round_trip, selector);
        assert!(
            serde_json::from_value::<super::DjTrackSelector>(serde_json::json!({
                "titleContains": "\u{4eba}\u{751f}\u{30aa}\u{30fc}\u{30d0}\u{30fc}",
                "unexpected": true
            }))
            .is_err()
        );
    }

    #[test]
    fn timeline_tempo_meter_legacy_defaults_and_strict_validation() {
        let legacy: TimelineSnapshot = serde_json::from_str(
            r#"{"id":1,"label":"Legacy","events":[],"automations":[],"video_automations":[],"playing":false,"position_ms":0,"duration_ms":0}"#,
        )
        .unwrap();
        assert!(legacy.tempo_meter_map.is_empty());
        assert_eq!(
            legacy.tempo_meter_map_version,
            TIMELINE_TEMPO_METER_MAP_VERSION
        );
        assert_eq!(ChildTimelineSummary::default().tempo_meter_map_version, 1);
        let legacy_json = serde_json::to_value(&legacy).unwrap();
        assert!(legacy_json.get("tempo_meter_map").is_none());
        assert!(legacy_json.get("tempo_meter_map_version").is_none());
        let legacy_child_json = serde_json::to_value(ChildTimelineSummary::default()).unwrap();
        assert!(legacy_child_json.get("tempo_meter_map").is_none());
        assert!(legacy_child_json.get("tempo_meter_map_version").is_none());
        assert_eq!(
            serde_json::to_string(&super::TimelinePhaseRole::Interlude).unwrap(),
            "\"interlude\""
        );
        let legacy_guide: super::TimelineGuideCueSummary =
            serde_json::from_value(serde_json::json!({
            "generation": 1,
            "sequence": 1,
            "at_ms": 0,
            "label": "Intro",
            "cue": { "kind": "phase", "phase_id": 1 },
            "asset": "intro",
            "sample_frame": 0,
            "epoch": 1,
            "transport_generation": 1,
            "schedule_generation": 1,
            "source": { "kind": "root" }
            }))
            .unwrap();
        assert_eq!(legacy_guide.playback_rate_milli, 1_000);

        let valid = vec![TimelineTempoMeterPoint {
            position_sixteenth_steps: 0,
            bpm: 194.0,
            numerator: 7,
            denominator: 8,
            interpolation: TimelineTempoInterpolation::Step,
            ..TimelineTempoMeterPoint::default()
        }];
        validate_timeline_tempo_meter_map(1, &valid).unwrap();
        for (label, points) in [
            ("duplicate", vec![valid[0].clone(), valid[0].clone()]),
            (
                "unsorted",
                vec![
                    TimelineTempoMeterPoint {
                        position_sixteenth_steps: 8,
                        ..valid[0].clone()
                    },
                    valid[0].clone(),
                ],
            ),
            (
                "non-finite",
                vec![TimelineTempoMeterPoint {
                    bpm: f64::NAN,
                    ..valid[0].clone()
                }],
            ),
            (
                "overflow",
                vec![TimelineTempoMeterPoint {
                    position_sixteenth_steps: u64::MAX,
                    ..valid[0].clone()
                }],
            ),
            (
                "measure-overflow",
                vec![TimelineTempoMeterPoint {
                    measure_number: Some(u64::MAX),
                    ..valid[0].clone()
                }],
            ),
        ] {
            assert!(
                validate_timeline_tempo_meter_map(1, &points).is_err(),
                "{label} map must reject"
            );
        }
        assert!(validate_timeline_tempo_meter_map(2, &valid).is_err());
        assert!(validate_timeline_tempo_meter_map(0, &valid).is_err());

        let exact_meter_switch = vec![
            TimelineTempoMeterPoint {
                position_sixteenth_steps: 0,
                numerator: 7,
                denominator: 8,
                ..TimelineTempoMeterPoint::default()
            },
            TimelineTempoMeterPoint {
                position_sixteenth_steps: 14,
                numerator: 3,
                denominator: 8,
                ..TimelineTempoMeterPoint::default()
            },
        ];
        validate_timeline_tempo_meter_map(1, &exact_meter_switch).unwrap();
        let mid_measure_meter_switch = vec![
            TimelineTempoMeterPoint::default(),
            TimelineTempoMeterPoint {
                position_sixteenth_steps: 8,
                numerator: 7,
                denominator: 8,
                ..TimelineTempoMeterPoint::default()
            },
        ];
        assert!(validate_timeline_tempo_meter_map(1, &mid_measure_meter_switch).is_err());
        let reverse_measure_numbers = vec![
            TimelineTempoMeterPoint {
                measure_number: Some(10),
                ..TimelineTempoMeterPoint::default()
            },
            TimelineTempoMeterPoint {
                position_sixteenth_steps: 16,
                measure_number: Some(9),
                ..TimelineTempoMeterPoint::default()
            },
        ];
        assert!(validate_timeline_tempo_meter_map(1, &reverse_measure_numbers).is_err());
        let tempo_only_mid_measure = vec![
            TimelineTempoMeterPoint::default(),
            TimelineTempoMeterPoint {
                position_sixteenth_steps: 2,
                bpm: 130.0,
                ..TimelineTempoMeterPoint::default()
            },
        ];
        validate_timeline_tempo_meter_map(1, &tempo_only_mid_measure).unwrap();
    }

    #[test]
    fn osc_control_mapping_rejects_unknown_keys_and_roundtrips_canonical_v1() {
        let canonical = r#"{"address":"/main/fader","action":"LightingMaster","fixture_id":null,"attribute":null,"group_id":"all","cue_id":null,"layer_id":3,"output_id":2,"video_param":null,"cue_point_index":null,"duration_ms":250,"low":0.0,"high":1.0}"#;
        let mapping: OscControlMapping = serde_json::from_str(canonical).unwrap();
        assert_eq!(serde_json::to_string(&mapping).unwrap(), canonical);
        assert_eq!(
            serde_json::from_str::<OscControlMapping>(&serde_json::to_string(&mapping).unwrap())
                .unwrap(),
            mapping
        );

        let misspelled = canonical.replace(r#""group_id":"all""#, r#""group_ids":"all""#);
        let error = serde_json::from_str::<OscControlMapping>(&misspelled)
            .expect_err("misspelled optional key must be rejected");
        assert!(error.to_string().contains("`group_ids`"), "{error}");

        let future = canonical.replace(
            r#""duration_ms":250"#,
            r#""duration_ms":250,"future_param":true"#,
        );
        let error = serde_json::from_str::<OscControlMapping>(&future)
            .expect_err("future key must be rejected");
        assert!(error.to_string().contains("`future_param`"), "{error}");
    }

    #[test]
    fn dmx_control_mapping_rejects_unknown_keys_and_roundtrips_canonical_v1() {
        let canonical = r#"{"universe":1,"channel":12,"action":"FixtureAttribute","fixture_id":9,"attribute":"dimmer","group_id":null,"cue_id":null,"layer_id":null,"output_id":null,"video_param":null,"cue_point_index":null,"duration_ms":null,"low":0.0,"high":1.0}"#;
        let mapping: DmxControlMapping = serde_json::from_str(canonical).unwrap();
        assert_eq!(serde_json::to_string(&mapping).unwrap(), canonical);
        assert_eq!(
            serde_json::from_str::<DmxControlMapping>(&serde_json::to_string(&mapping).unwrap())
                .unwrap(),
            mapping
        );

        let misspelled = canonical.replace(r#""output_id":null"#, r#""outputs_id":null"#);
        let error = serde_json::from_str::<DmxControlMapping>(&misspelled)
            .expect_err("misspelled optional key must be rejected");
        assert!(error.to_string().contains("`outputs_id`"), "{error}");

        let future = canonical.replace(
            r#""duration_ms":null"#,
            r#""duration_ms":null,"future_flag":false"#,
        );
        let error = serde_json::from_str::<DmxControlMapping>(&future)
            .expect_err("future key must be rejected");
        assert!(error.to_string().contains("`future_flag`"), "{error}");
    }

    #[test]
    fn midi_control_mapping_rejects_unknown_keys_and_roundtrips_canonical_v1() {
        let canonical = r#"{"channel":5,"message":"ControlChange","number":74,"action":"VideoParam","fixture_id":null,"attribute":null,"group_id":null,"cue_id":null,"layer_id":2,"output_id":null,"video_param":"Opacity","cue_point_index":null,"duration_ms":null,"feedback":{"off":{"message":"NoteOn","channel":5,"number":74,"value":0},"on":{"message":"NoteOn","channel":5,"number":74,"value":127}},"low":0.0,"high":1.0}"#;
        let mapping: MidiControlMapping = serde_json::from_str(canonical).unwrap();
        assert_eq!(serde_json::to_string(&mapping).unwrap(), canonical);
        assert_eq!(
            serde_json::from_str::<MidiControlMapping>(&serde_json::to_string(&mapping).unwrap())
                .unwrap(),
            mapping
        );

        let misspelled = canonical.replace(r#""feedback":{"off""#, r#""feeback":{"off""#);
        let error = serde_json::from_str::<MidiControlMapping>(&misspelled)
            .expect_err("misspelled optional key must be rejected");
        assert!(error.to_string().contains("`feeback`"), "{error}");

        let future = canonical.replace(
            r#""action":"VideoParam""#,
            r#""action":"VideoParam","future_action":null"#,
        );
        let error = serde_json::from_str::<MidiControlMapping>(&future)
            .expect_err("future key must be rejected");
        assert!(error.to_string().contains("`future_action`"), "{error}");
    }

    fn reference_integrity_fixture() -> super::PatchedFixtureSummary {
        super::PatchedFixtureSummary {
            id: 1,
            label: "Fixture 1".to_string(),
            profile_source_path: "fixture.gdtf".to_string(),
            profile_name: "Test".to_string(),
            manufacturer: "Test".to_string(),
            mode_name: "Mode".to_string(),
            universe: 0,
            address: 1,
            group_ids: vec!["front".to_string()],
            position: super::Vec3::default(),
            rotation: super::Rotation3::default(),
            geometries: Vec::new(),
            controls: vec![super::AttributeControl {
                attribute: "Dimmer".to_string(),
                channel_name: "Dimmer".to_string(),
                geometry: None,
                offsets: vec![1],
                resolution: super::AttributeResolution::EightBit,
                default_value: 0,
                functions: Vec::new(),
            }],
            attribute_values: vec![super::AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: 0,
            }],
            stage_layout: None,
            limits: super::FixtureLimits::default(),
            highlighted: false,
            soloed: false,
            parked: false,
        }
    }

    fn reference_integrity_video() -> super::VideoSnapshot {
        let source = super::VideoSourceSummary {
            kind: super::VideoSourceKind::StillImage,
            path: Some("test.png".to_string()),
            name: None,
            codec: None,
            metadata: None,
        };
        let asset = super::MediaAssetSummary {
            id: 1,
            label: "Test image".to_string(),
            source: source.clone(),
            content_hash: None,
            byte_size: None,
        };
        let layer = super::VideoLayerSummary {
            id: 10,
            label: "Layer 10".to_string(),
            source,
            media_asset_id: Some(1),
            blend_mode: super::VideoBlendMode::Normal,
            state: super::VideoLayerState::default(),
            isf_effect: None,
            clip_slots: vec![super::VideoClipSlotSummary {
                id: super::VideoClipSlotId(20),
                media_asset_id: 1,
                in_point_ms: 0,
                out_point_ms: Some(1_000),
                loop_mode: super::VideoClipLoopMode::Once,
                speed: 1.0,
                cue_points: Vec::new(),
                launch_quantization: super::VideoClipLaunchQuantization::Immediate,
                effect_overrides: Vec::new(),
            }],
            default_clip_slot_id: Some(super::VideoClipSlotId(20)),
        };
        let output = super::VideoOutputSummary {
            id: 30,
            label: "Display".to_string(),
            kind: super::VideoOutputKind::Display,
            enabled: true,
            composition_id: 40,
            fullscreen: false,
            monitor_id: None,
            monitor_identity: None,
            width: 1_920,
            height: 1_080,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: super::VideoOutputMapping::default(),
        };
        super::VideoSnapshot {
            layers: vec![layer],
            media_assets: vec![asset],
            compositions: vec![super::CompositionSummary {
                id: 40,
                label: "Main".to_string(),
                layer_ids: vec![10],
                output_ids: vec![30],
            }],
            outputs: vec![output],
            ..super::VideoSnapshot::default()
        }
    }

    fn reference_integrity_timeline() -> super::TimelineSnapshot {
        super::TimelineSnapshot {
            id: super::TimelineId(1),
            label: "Timeline 1".to_string(),
            layers: vec![
                super::TimelineLayerSummary {
                    id: 1,
                    label: "Lighting".to_string(),
                    order: 0,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: true,
                    kind: super::TimelineLayerKind::Lighting,
                },
                super::TimelineLayerSummary {
                    id: 2,
                    label: "Video".to_string(),
                    order: 1,
                    muted: false,
                    locked: false,
                    solo: false,
                    expanded: true,
                    kind: super::TimelineLayerKind::Video,
                },
            ],
            events: vec![super::TimelineCueEventSummary {
                id: 100,
                cue_id: 1,
                layer_id: Some(1),
                ..super::TimelineCueEventSummary::default()
            }],
            automations: vec![super::TimelineAutomationSummary {
                id: 200,
                fixture_id: 1,
                attribute: "Dimmer".to_string(),
                track: super::TimelineTrackKind::Lighting,
                timeline_layer_id: Some(1),
                keyframes: vec![super::AutomationKeyframeSummary {
                    time_ms: 0,
                    value: 0,
                    interpolation: super::AutomationInterpolation::Step,
                }],
                enabled: true,
            }],
            video_automations: vec![super::TimelineVideoAutomationSummary {
                id: 201,
                layer_id: 10,
                timeline_layer_id: Some(2),
                param: super::VideoParam::Opacity,
                track: super::TimelineTrackKind::Video,
                keyframes: vec![super::VideoAutomationKeyframeSummary {
                    time_ms: 0,
                    value: 1.0,
                    interpolation: super::AutomationInterpolation::Step,
                }],
                enabled: true,
            }],
            video_clips: vec![super::TimelineVideoClipSummary {
                id: super::TimelineVideoClipId(300),
                layer_id: 2,
                media_asset_id: 1,
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 1_000,
                fade_in_ms: 0,
                fade_out_ms: 0,
            }],
            duration_ms: 1_000,
            ..super::TimelineSnapshot::default()
        }
    }

    fn reference_integrity_snapshot() -> super::EngineSnapshot {
        let mut timeline = reference_integrity_timeline();
        let bank_timeline = timeline.clone();
        // These are runtime fields and must not participate in authored-bank
        // equality.
        timeline.playing = true;
        timeline.position_ms = 500;
        timeline.transport_epoch = 7;
        timeline.transport_generation = 9;
        let cue = super::CueSummary {
            id: 1,
            cue_list_id: 1,
            cue_number: "1".to_string(),
            label: "Cue 1".to_string(),
            group_id: Some("front".to_string()),
            targets: vec![super::CueFixtureTarget {
                fixture_id: 1,
                values: vec![super::AttributeValueSummary {
                    attribute: "Dimmer".to_string(),
                    value: u16::MAX,
                }],
            }],
            video_targets: vec![super::VideoLayerTarget {
                layer_id: 10,
                state: super::VideoLayerState::default(),
            }],
            video_output_targets: vec![super::VideoOutputTarget {
                output_id: 30,
                enabled: true,
                opacity: 1.0,
                blackout: false,
            }],
            ..super::CueSummary::default()
        };
        super::EngineSnapshot {
            fixtures: vec![reference_integrity_fixture()],
            cues: vec![cue],
            cue_lists: vec![super::CueListSummary {
                id: 1,
                label: "Bank 1".to_string(),
                active_cue_id: Some(1),
            }],
            active_cue_id: Some(1),
            active_group_cue_ids: std::collections::BTreeMap::from([("front".to_string(), 1)]),
            timeline,
            timeline_bank: vec![bank_timeline],
            video: reference_integrity_video(),
            ..super::EngineSnapshot::default()
        }
    }

    fn reference_integrity_follow(
        target: super::TimelineId,
        enabled: bool,
    ) -> super::TimelineFollowSummary {
        super::TimelineFollowSummary {
            enabled,
            next_timeline_id: target,
            duration: super::VideoClipTakeDuration::milliseconds(0),
            curve: super::VideoLayerTransitionCurve::Linear,
            video_kind: super::VideoClipTakeKind::Cut,
            lighting_policy: super::TimelineFollowLightingPolicy::HoldThenCut,
            destination_bpm: None,
            preroll_ms: 0,
            trans_cadence_bars: 4,
            trans_target_measures: Vec::new(),
            hold_first_destination_measure: false,
            fault_policy: super::TimelineFollowFaultPolicy::Hold,
        }
    }

    #[test]
    fn current_project_reference_integrity_accepts_exact_graph_and_ignores_runtime_fields() {
        let snapshot = reference_integrity_snapshot();
        assert_eq!(
            super::validate_current_engine_snapshot_reference_integrity(&snapshot).unwrap(),
            super::ProjectReferenceIntegrityReport {
                video_authority: super::ProjectVideoAuthority::Video,
                unvalidated_external_registry: Some(
                    super::ProjectExternalRegistryBoundary::LIGHTING_FIXTURE_GROUPS,
                ),
            }
        );

        let mut conflict = snapshot.clone();
        conflict.timeline_bank[0].label = "Different authored label".to_string();
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&conflict),
            Err(
                super::ProjectReferenceIntegrityError::ActiveTimelineProjectionConflict {
                    timeline_id: super::TimelineId(1),
                    ..
                }
            )
        ));
    }

    #[test]
    fn current_project_reference_integrity_rejects_active_and_timeline_hostile_refs() {
        let mut missing_active = reference_integrity_snapshot();
        missing_active.cue_lists[0].active_cue_id = Some(99);
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&missing_active),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                target_domain: "Cue",
                target_id: 99,
                ..
            })
        ));

        let mut wrong_group = reference_integrity_snapshot();
        wrong_group.active_group_cue_ids =
            std::collections::BTreeMap::from([("other".to_string(), 1)]);
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&wrong_group),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { .. })
        ));

        let mut wrong_bank_membership = reference_integrity_snapshot();
        wrong_bank_membership.cue_lists.push(super::CueListSummary {
            id: 2,
            label: "Bank 2".to_string(),
            active_cue_id: None,
        });
        wrong_bank_membership.cues[0].cue_list_id = 2;
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&wrong_bank_membership),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { .. })
        ));

        let mut inactive_bank = reference_integrity_snapshot();
        let mut second = inactive_bank.timeline_bank[0].clone();
        second.id = super::TimelineId(2);
        second.label = "Inactive hostile".to_string();
        second.events[0].cue_id = 999;
        inactive_bank.timeline_bank.push(second);
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&inactive_bank),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                target_domain: "Cue",
                target_id: 999,
                ..
            })
        ));

        let mut missing_jump = reference_integrity_snapshot();
        missing_jump.timeline.events[0].jump_to_event_id = Some(999);
        missing_jump.timeline_bank[0] = missing_jump.timeline.clone();
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&missing_jump),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                target_domain: "Timeline event",
                target_id: 999,
                ..
            })
        ));

        let mut missing_attribute = reference_integrity_snapshot();
        missing_attribute.timeline.automations[0].attribute = "Pan".to_string();
        missing_attribute.timeline_bank[0] = missing_attribute.timeline.clone();
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&missing_attribute),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { .. })
        ));

        let mut missing_media = reference_integrity_snapshot();
        missing_media.timeline.video_clips[0].media_asset_id = 999;
        missing_media.timeline_bank[0] = missing_media.timeline.clone();
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&missing_media),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                target_domain: "MediaAsset",
                target_id: 999,
                ..
            })
        ));

        // Reciprocal jump links are an authored routing feature, not a child
        // Timeline recursion cycle.
        let mut reciprocal_jumps = reference_integrity_snapshot();
        reciprocal_jumps.timeline.events[0].jump_to_event_id = Some(101);
        let mut second_event = reciprocal_jumps.timeline.events[0].clone();
        second_event.id = 101;
        second_event.jump_to_event_id = Some(100);
        reciprocal_jumps.timeline.events.push(second_event);
        reciprocal_jumps.timeline_bank[0] = reciprocal_jumps.timeline.clone();
        assert!(
            super::validate_current_engine_snapshot_reference_integrity(&reciprocal_jumps).is_ok()
        );
    }

    #[test]
    fn current_project_reference_integrity_rejects_child_and_follow_cycles() {
        let mut child_cycle = reference_integrity_snapshot();
        let mut cue_two = super::CueSummary {
            id: 2,
            cue_list_id: 1,
            cue_number: "2".to_string(),
            label: "Cue 2".to_string(),
            ..super::CueSummary::default()
        };
        child_cycle.cues[0].child_timeline = Some(super::ChildTimelineSummary {
            events: vec![super::TimelineCueEventSummary {
                id: 400,
                cue_id: 2,
                ..super::TimelineCueEventSummary::default()
            }],
            ..super::ChildTimelineSummary::default()
        });
        cue_two.child_timeline = Some(super::ChildTimelineSummary {
            events: vec![super::TimelineCueEventSummary {
                id: 401,
                cue_id: 1,
                ..super::TimelineCueEventSummary::default()
            }],
            ..super::ChildTimelineSummary::default()
        });
        child_cycle.cues.push(cue_two);
        assert_eq!(
            super::validate_current_engine_snapshot_reference_integrity(&child_cycle),
            Err(super::ProjectReferenceIntegrityError::ReferenceCycle {
                domain: "Cue child Timeline",
                path: vec![1, 2, 1],
            })
        );

        let mut follow_cycle = reference_integrity_snapshot();
        follow_cycle.timeline.follow =
            Some(reference_integrity_follow(super::TimelineId(2), false));
        follow_cycle.timeline_bank[0] = follow_cycle.timeline.clone();
        let mut second = follow_cycle.timeline_bank[0].clone();
        second.id = super::TimelineId(2);
        second.label = "Timeline 2".to_string();
        second.follow = Some(reference_integrity_follow(super::TimelineId(1), false));
        follow_cycle.timeline_bank.push(second);
        assert_eq!(
            super::validate_current_engine_snapshot_reference_integrity(&follow_cycle),
            Err(super::ProjectReferenceIntegrityError::ReferenceCycle {
                domain: "Timeline Follow",
                path: vec![1, 2, 1],
            })
        );
    }

    #[test]
    fn current_project_reference_integrity_rejects_video_graph_hostile_refs() {
        let mut duplicate_layer = reference_integrity_snapshot();
        duplicate_layer
            .video
            .layers
            .push(duplicate_layer.video.layers[0].clone());
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&duplicate_layer),
            Err(super::ProjectReferenceIntegrityError::DuplicateId {
                domain: "video layer",
                id: 10,
            })
        ));

        let mut broken_reverse = reference_integrity_snapshot();
        broken_reverse.video.compositions[0].output_ids.clear();
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&broken_reverse),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { .. })
        ));

        let mut missing_scope = reference_integrity_snapshot();
        missing_scope
            .video
            .effect_chains
            .push(super::VideoEffectChainSummary {
                id: super::VideoEffectChainId(1),
                scope: super::VideoEffectScope::Output { output_id: 999 },
                bypassed: false,
                stages: Vec::new(),
            });
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&missing_scope),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                target_domain: "video output",
                target_id: 999,
                ..
            })
        ));

        let mut missing_transition_member = reference_integrity_snapshot();
        missing_transition_member.video.transition_buses.push(
            super::VideoLayerTransitionBusSummary {
                id: super::VideoTransitionBusId(50),
                label: "Bus".to_string(),
                composition_id: 40,
                enabled: true,
                members: vec![
                    super::VideoLayerTransitionTarget::Layer { layer_id: 10 },
                    super::VideoLayerTransitionTarget::Layer { layer_id: 999 },
                ],
                default_from: super::VideoLayerTransitionTarget::Layer { layer_id: 10 },
                default_to: super::VideoLayerTransitionTarget::Layer { layer_id: 999 },
                default_kind: super::VideoClipTakeKind::Crossfade,
                default_duration: super::VideoClipTakeDuration::milliseconds(500),
                default_curve: super::VideoLayerTransitionCurve::Linear,
                matte_source: None,
            },
        );
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&missing_transition_member),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                target_domain: "video layer",
                target_id: 999,
                ..
            })
        ));

        let mut zero_group = reference_integrity_snapshot();
        zero_group
            .video
            .layer_groups
            .push(super::VideoLayerGroupSummary {
                id: super::VideoLayerGroupId(0),
                label: "Zero".to_string(),
                composition_id: 40,
                layer_ids: vec![10],
            });
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&zero_group),
            Err(super::ProjectReferenceIntegrityError::ZeroId {
                domain: "video layer group",
                ..
            })
        ));
    }

    #[test]
    fn current_project_reference_integrity_types_authored_video_conflicts_and_runtime_projection() {
        let mut authored = reference_integrity_snapshot();
        authored.authored_video = Some(authored.video.clone());
        // Rendered live state may differ from authored state without changing
        // reference authority.
        authored.video.layers[0].state.opacity = 0.25;
        authored.video.layers[0].media_asset_id = None;
        assert_eq!(
            super::validate_current_engine_snapshot_reference_integrity(&authored)
                .unwrap()
                .video_authority,
            super::ProjectVideoAuthority::AuthoredVideo
        );

        let mut runtime_projection = authored.clone();
        let mut projected_layer = runtime_projection.video.layers[0].clone();
        projected_layer.id = 99;
        projected_layer.clip_slots.clear();
        projected_layer.default_clip_slot_id = None;
        runtime_projection.video.layers.push(projected_layer);
        runtime_projection.video.compositions[0].layer_ids.push(99);
        runtime_projection
            .video_clip_runtime
            .timeline_video_projection_layer_ids = vec![99];
        assert!(
            super::validate_current_engine_snapshot_reference_integrity(&runtime_projection)
                .is_ok()
        );

        runtime_projection
            .video_clip_runtime
            .timeline_video_projection_layer_ids
            .clear();
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&runtime_projection),
            Err(
                super::ProjectReferenceIntegrityError::AuthoredVideoConflict {
                    kind: super::ProjectAuthoredVideoConflictKind::IdentitySetMismatch,
                    ..
                }
            )
        ));

        let mut topology_conflict = authored;
        topology_conflict
            .authored_video
            .as_mut()
            .unwrap()
            .compositions[0]
            .output_ids
            .clear();
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&topology_conflict),
            Err(
                super::ProjectReferenceIntegrityError::AuthoredVideoConflict {
                    kind: super::ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
                    ..
                }
            )
        ));
    }

    #[test]
    fn required_empty_timeline_bank_has_dedicated_taxonomy() {
        let mut empty_bank = reference_integrity_snapshot();
        empty_bank.timeline_bank.clear();
        assert_eq!(
            super::validate_current_engine_snapshot_reference_integrity(&empty_bank),
            Err(
                super::ProjectReferenceIntegrityError::RequiredTimelineBankEmpty {
                    active_timeline_id: super::TimelineId(1)
                }
            )
        );
        assert_eq!(
            super::ProjectReferenceIntegrityError::RequiredTimelineBankEmpty {
                active_timeline_id: super::TimelineId(1)
            }
            .to_string(),
            "current schema requires a non-empty Timeline bank; active Timeline 1 has no bank entries"
        );
    }

    fn multi_attribute_fixture(id: super::FixtureId) -> super::PatchedFixtureSummary {
        let mut fixture = reference_integrity_fixture();
        fixture.id = id;
        fixture.label = format!("Fixture {id}");
        for (attribute, offset) in [("Pan", 2), ("Tilt", 3)] {
            fixture.controls.push(super::AttributeControl {
                attribute: attribute.to_string(),
                channel_name: attribute.to_string(),
                geometry: None,
                offsets: vec![offset],
                resolution: super::AttributeResolution::EightBit,
                default_value: 0,
                functions: Vec::new(),
            });
            fixture.attribute_values.push(super::AttributeValueSummary {
                attribute: attribute.to_string(),
                value: 0,
            });
        }
        fixture
    }

    fn payload_test_snapshot() -> super::EngineSnapshot {
        let mut snapshot = reference_integrity_snapshot();
        snapshot.fixtures = vec![multi_attribute_fixture(1)];
        snapshot
    }

    fn payload_effect(summary_id: u64) -> super::EffectSummary {
        super::EffectSummary {
            id: summary_id,
            label: "Payload effect".to_string(),
            effect_type: super::EffectKind::Lfo,
            fixture_ids: vec![1],
            target_group_ids: vec!["front".to_string()],
            attribute: "Dimmer".to_string(),
            video_targets: Vec::new(),
            shape: super::LfoShape::Sine,
            period_ms: Some(1_000),
            clock_sync: None,
            low: 0,
            high: u16::MAX,
            phase: 0.0,
            fixture_spread: 0.0,
            blend_mode: super::EffectBlendMode::Override,
            origin: None,
            direction: None,
            speed: None,
            wavelength: None,
            enabled: true,
            lfo: Some(payload_lfo_request()),
            color: Some(payload_color_request()),
            chaser: Some(payload_chaser_request()),
            move_effect: Some(payload_move_request()),
            value: Some(payload_value_request()),
            curve: Some(payload_curve_request()),
            mapping: Some(payload_mapping_request()),
            color_mapping: Some(payload_color_mapping_request()),
        }
    }

    fn payload_lfo_request() -> super::LfoEffectRequest {
        super::LfoEffectRequest {
            label: "LFO".to_string(),
            fixture_ids: vec![1],
            target_group_ids: vec!["front".to_string()],
            attribute: "Dimmer".to_string(),
            video_targets: Vec::new(),
            shape: super::LfoShape::Sine,
            period_ms: 1_000,
            clock_sync: None,
            low: 0,
            high: u16::MAX,
            phase: 0.0,
            fixture_spread: 0.0,
            beam_targets: vec![super::EffectBeamTarget {
                fixture_id: 1,
                beam_index: 0,
                selection_index: 0,
                feature_attribute: "Pan".to_string(),
            }],
            blend_mode: super::EffectBlendMode::Override,
            daslight_curve: None,
            daslight_custom_curve: None,
        }
    }

    fn payload_position_wave_request() -> super::PositionWaveEffectRequest {
        super::PositionWaveEffectRequest {
            label: "Position wave".to_string(),
            fixture_ids: vec![1],
            target_group_ids: Vec::new(),
            attribute: "Dimmer".to_string(),
            video_targets: Vec::new(),
            shape: super::LfoShape::Sine,
            origin: super::Vec3::default(),
            direction: super::Vec3::default(),
            speed: 1.0,
            wavelength: 1.0,
            clock_sync: None,
            low: 0,
            high: u16::MAX,
            phase: 0.0,
            blend_mode: super::EffectBlendMode::Override,
        }
    }

    fn payload_color_request() -> super::ColorEffectRequest {
        super::ColorEffectRequest {
            label: "Color".to_string(),
            fixture_ids: vec![1],
            target_group_ids: Vec::new(),
            stops: vec![super::ColorEffectStop {
                position: 0.0,
                color: super::ColorEffectColor {
                    red: 65_535,
                    green: 0,
                    blue: 0,
                },
            }],
            algorithm: super::ColorEffectAlgorithm::Cycle,
            interpolation: super::ColorEffectInterpolation::Rgb,
            period_ms: 1_000,
            clock_sync: None,
            phase: 0.0,
            fixture_spread: 0.0,
            blend_mode: super::EffectBlendMode::Override,
            spatial_pattern: Some(Box::new(super::ColorEffectSpatialPattern {
                recipe: super::ColorEffectSpatialRecipe::Sweep {
                    grayscale: false,
                    vertical_symmetry: false,
                    direction_change: false,
                },
                parameter_model_version: 1,
                beam_targets: vec![super::ColorEffectBeamTarget {
                    fixture_id: 1,
                    beam_index: 0,
                    selection_index: 0,
                    feature_attribute: Some("Dimmer".to_string()),
                }],
                placement: Some(super::ColorEffectSpatialPlacement {
                    source_coordinate_frame:
                        super::ColorEffectSpatialCoordinateFrame::DaslightPatchCanvas,
                    mapping_shape: super::ColorEffectSpatialMappingShape::Rectangle,
                    x: 0,
                    y: 0,
                    sx: 1,
                    sy: 1,
                    mapping_angle_degrees: 0.0,
                    sampling_rule:
                        super::ColorEffectSpatialSamplingRule::RotatedInclusionMaskAxisAlignedRaster,
                    vertical_symmetry: false,
                    horizontal_symmetry: false,
                    raster_rotation_degrees: 0.0,
                    target_coordinates: vec![super::ColorEffectSpatialPlacementTarget {
                        fixture_id: 1,
                        beam_index: 0,
                        patch_x: 0,
                        patch_y: 0,
                    }],
                }),
            })),
        }
    }

    fn payload_chaser_request() -> super::ChaserEffectRequest {
        super::ChaserEffectRequest {
            label: "Chaser".to_string(),
            steps: vec![
                super::ChaserStep {
                    fixture_ids: vec![1],
                    target_group_ids: Vec::new(),
                    beam_targets: vec![super::EffectBeamTarget {
                        fixture_id: 1,
                        beam_index: 0,
                        selection_index: 0,
                        feature_attribute: "Dimmer".to_string(),
                    }],
                    level: u16::MAX,
                },
                super::ChaserStep {
                    fixture_ids: Vec::new(),
                    target_group_ids: Vec::new(),
                    beam_targets: Vec::new(),
                    level: 0,
                },
            ],
            features: vec![super::ChaserFeature {
                attribute: "Dimmer".to_string(),
                low: 0,
                high: u16::MAX,
            }],
            step_duration_ms: 250,
            clock_sync: None,
            direction: super::ChaserDirection::Forward,
            wings: 1,
            active_step_count: 1,
            duty_cycle: 1.0,
            overlap: 0.0,
            phase: 0.0,
            fixture_spread: 0.0,
            random_seed: 0,
            random_cycle_count: 1,
            blend_mode: super::EffectBlendMode::Override,
        }
    }

    fn payload_move_request() -> super::MoveEffectRequest {
        super::MoveEffectRequest {
            label: "Move".to_string(),
            fixture_ids: vec![1],
            target_group_ids: Vec::new(),
            beam_targets: vec![super::MoveEffectBeamTarget {
                fixture_id: 1,
                beam_index: 0,
                selection_index: 0,
            }],
            points: vec![
                super::MovePathPoint { x: 0.0, y: 0.0 },
                super::MovePathPoint { x: 1.0, y: 1.0 },
            ],
            closed: false,
            interpolation: super::MoveInterpolation::Line,
            coordinate_mode: super::MoveCoordinateMode::Absolute,
            center_x: 0.0,
            center_y: 0.0,
            size_x: 1.0,
            size_y: 1.0,
            rotation_degrees: 0.0,
            period_ms: 1_000,
            clock_sync: None,
            direction: super::MoveDirection::Forward,
            phase: 0.0,
            fixture_spread: 0.0,
            symmetry: false,
            blend_mode: super::EffectBlendMode::Override,
        }
    }

    fn payload_value_request() -> super::ValueEffectRequest {
        super::ValueEffectRequest {
            label: "Value".to_string(),
            fixture_ids: vec![1],
            target_group_ids: Vec::new(),
            attribute: "Dimmer".to_string(),
            features: Vec::new(),
            points: vec![
                super::ValueEffectPoint {
                    position: 0.0,
                    value: 0.0,
                },
                super::ValueEffectPoint {
                    position: 1.0,
                    value: 1.0,
                },
            ],
            spatial_pattern: None,
            interpolation: super::ValueEffectInterpolation::Line,
            mode: super::ValueEffectMode::Absolute,
            direction: super::ValueEffectDirection::Forward,
            period_ms: 1_000,
            clock_sync: None,
            low: 0,
            high: u16::MAX,
            phase: 0.0,
            fixture_spread: 0.0,
            blend_mode: super::EffectBlendMode::Override,
        }
    }

    fn payload_curve_request() -> super::CurveEffectRequest {
        super::CurveEffectRequest {
            label: "Curve".to_string(),
            fixture_ids: vec![1],
            target_group_ids: Vec::new(),
            attribute: "Dimmer".to_string(),
            features: Vec::new(),
            points: vec![
                super::CurveEffectPoint {
                    position: 0.0,
                    value: 0.0,
                    in_tangent: 0.0,
                    out_tangent: 0.0,
                },
                super::CurveEffectPoint {
                    position: 1.0,
                    value: 1.0,
                    in_tangent: 0.0,
                    out_tangent: 0.0,
                },
            ],
            mode: super::ValueEffectMode::Absolute,
            direction: super::ValueEffectDirection::Forward,
            period_ms: 1_000,
            clock_sync: None,
            low: 0,
            high: u16::MAX,
            phase: 0.0,
            fixture_spread: 0.0,
            blend_mode: super::EffectBlendMode::Override,
        }
    }

    fn payload_mapping_request() -> super::MappingEffectRequest {
        super::MappingEffectRequest {
            label: "Mapping".to_string(),
            fixture_ids: vec![1],
            target_group_ids: Vec::new(),
            attribute: "Dimmer".to_string(),
            features: Vec::new(),
            shape: super::LfoShape::Sine,
            mode: super::ValueEffectMode::Absolute,
            direction: super::MappingEffectDirection::Static,
            period_ms: 1_000,
            clock_sync: None,
            low: 0,
            high: u16::MAX,
            phase: 0.0,
            fixture_spread: 0.0,
            repetitions: 1.0,
            blend_mode: super::EffectBlendMode::Override,
        }
    }

    fn payload_color_mapping_request() -> super::ColorMappingEffectRequest {
        super::ColorMappingEffectRequest {
            label: "Color mapping".to_string(),
            fixture_ids: vec![1],
            target_group_ids: Vec::new(),
            source_kind: super::ColorMappingSourceKind::Image,
            width: 1,
            height: 1,
            frames: vec![super::ColorMappingFrame { pixels: vec![0] }],
            cells: vec![super::ColorMappingCellTarget {
                fixture_id: 1,
                beam_index: 0,
                selection_index: 0,
                u: 0.0,
                v: 0.0,
                feature_attribute: Some("Dimmer".to_string()),
                feature_low: None,
                feature_high: None,
            }],
            playback_direction: super::ColorMappingPlaybackDirection::Forward,
            period_ms: 1_000,
            clock_sync: None,
            phase: 0.0,
            offset_u: 0.0,
            offset_v: 0.0,
            scale_u: 1.0,
            scale_v: 1.0,
            rotation_degrees: 0.0,
            wrap_mode: super::ColorMappingWrapMode::Clamp,
            sampling: super::ColorMappingSampling::Nearest,
            blend_mode: super::EffectBlendMode::Override,
        }
    }

    #[test]
    fn fully_populated_embedded_effect_payloads_pass_reference_integrity() {
        let mut snapshot = payload_test_snapshot();
        snapshot.effects.push(payload_effect(500));
        assert!(super::validate_current_engine_snapshot_reference_integrity(&snapshot).is_ok());
    }

    #[test]
    fn embedded_effect_payload_hostile_references_are_rejected_per_family() {
        enum Mutation {
            MissingFixture,
            UnsupportedAttribute,
            UnsupportedBeamFeature,
            MissingBeamFixture,
            MissingPlacementFixture,
            UnresolvableFeature,
        }

        let cases: Vec<(&str, Mutation, Box<dyn Fn(&mut super::EngineSnapshot)>)> = vec![
            (
                "LFO",
                Mutation::MissingFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].lfo.as_mut().unwrap().fixture_ids = vec![99];
                }),
            ),
            (
                "LFO",
                Mutation::UnsupportedAttribute,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].lfo.as_mut().unwrap().attribute = "Zoom".to_string();
                }),
            ),
            (
                "LFO",
                Mutation::UnsupportedBeamFeature,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].lfo.as_mut().unwrap().beam_targets[0].feature_attribute =
                        "Zoom".to_string();
                }),
            ),
            (
                "LFO",
                Mutation::MissingBeamFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].lfo.as_mut().unwrap().beam_targets[0].fixture_id = 99;
                }),
            ),
            (
                "Color",
                Mutation::MissingFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].color.as_mut().unwrap().fixture_ids = vec![99];
                }),
            ),
            (
                "Color",
                Mutation::UnsupportedBeamFeature,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0]
                        .color
                        .as_mut()
                        .unwrap()
                        .spatial_pattern
                        .as_mut()
                        .unwrap()
                        .beam_targets[0]
                        .feature_attribute = Some("Zoom".to_string());
                }),
            ),
            (
                "Color",
                Mutation::MissingBeamFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0]
                        .color
                        .as_mut()
                        .unwrap()
                        .spatial_pattern
                        .as_mut()
                        .unwrap()
                        .beam_targets[0]
                        .fixture_id = 99;
                }),
            ),
            (
                "Color",
                Mutation::MissingPlacementFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0]
                        .color
                        .as_mut()
                        .unwrap()
                        .spatial_pattern
                        .as_mut()
                        .unwrap()
                        .placement
                        .as_mut()
                        .unwrap()
                        .target_coordinates[0]
                        .fixture_id = 99;
                }),
            ),
            (
                "Chaser",
                Mutation::MissingFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].chaser.as_mut().unwrap().steps[0].fixture_ids = vec![99];
                }),
            ),
            (
                "Chaser",
                Mutation::UnsupportedBeamFeature,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].chaser.as_mut().unwrap().steps[0].beam_targets[0]
                        .feature_attribute = "Zoom".to_string();
                }),
            ),
            (
                "Chaser",
                Mutation::UnresolvableFeature,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].chaser.as_mut().unwrap().features[0].attribute =
                        "Zoom".to_string();
                }),
            ),
            (
                "Move",
                Mutation::MissingFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0]
                        .move_effect
                        .as_mut()
                        .unwrap()
                        .fixture_ids = vec![99];
                }),
            ),
            (
                "Move",
                Mutation::MissingBeamFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0]
                        .move_effect
                        .as_mut()
                        .unwrap()
                        .beam_targets[0]
                        .fixture_id = 99;
                }),
            ),
            (
                "Value",
                Mutation::MissingFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].value.as_mut().unwrap().fixture_ids = vec![99];
                }),
            ),
            (
                "Value",
                Mutation::UnsupportedAttribute,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].value.as_mut().unwrap().attribute = "Zoom".to_string();
                }),
            ),
            (
                "Value",
                Mutation::UnresolvableFeature,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    let value = snapshot.effects[0].value.as_mut().unwrap();
                    value.features = vec![super::ChaserFeature {
                        attribute: "Zoom".to_string(),
                        low: 0,
                        high: u16::MAX,
                    }];
                }),
            ),
            (
                "Value",
                Mutation::UnsupportedBeamFeature,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    let value = snapshot.effects[0].value.as_mut().unwrap();
                    value.spatial_pattern = Some(super::ColorEffectSpatialPattern {
                        recipe: super::ColorEffectSpatialRecipe::Sweep {
                            grayscale: false,
                            vertical_symmetry: false,
                            direction_change: false,
                        },
                        parameter_model_version: 1,
                        beam_targets: vec![super::ColorEffectBeamTarget {
                            fixture_id: 1,
                            beam_index: 0,
                            selection_index: 0,
                            feature_attribute: Some("Zoom".to_string()),
                        }],
                        placement: None,
                    });
                }),
            ),
            (
                "Curve",
                Mutation::MissingFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].curve.as_mut().unwrap().fixture_ids = vec![99];
                }),
            ),
            (
                "Curve",
                Mutation::UnsupportedAttribute,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].curve.as_mut().unwrap().attribute = "Zoom".to_string();
                }),
            ),
            (
                "Curve",
                Mutation::UnresolvableFeature,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    let curve = snapshot.effects[0].curve.as_mut().unwrap();
                    curve.features = vec![super::ChaserFeature {
                        attribute: "Zoom".to_string(),
                        low: 0,
                        high: u16::MAX,
                    }];
                }),
            ),
            (
                "Mapping",
                Mutation::MissingFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].mapping.as_mut().unwrap().fixture_ids = vec![99];
                }),
            ),
            (
                "Mapping",
                Mutation::UnsupportedAttribute,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].mapping.as_mut().unwrap().attribute = "Zoom".to_string();
                }),
            ),
            (
                "ColorMapping",
                Mutation::MissingFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0]
                        .color_mapping
                        .as_mut()
                        .unwrap()
                        .fixture_ids = vec![99];
                }),
            ),
            (
                "ColorMapping",
                Mutation::MissingBeamFixture,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].color_mapping.as_mut().unwrap().cells[0].fixture_id = 99;
                }),
            ),
            (
                "ColorMapping",
                Mutation::UnsupportedBeamFeature,
                Box::new(|snapshot: &mut super::EngineSnapshot| {
                    snapshot.effects[0].color_mapping.as_mut().unwrap().cells[0]
                        .feature_attribute = Some("Zoom".to_string());
                }),
            ),
        ];

        assert!(
            cases.len() >= 25,
            "table-driven coverage must stay exhaustive across families"
        );
        for (family, mutation, apply) in cases {
            let mut snapshot = payload_test_snapshot();
            snapshot.effects.push(payload_effect(500));
            apply(&mut snapshot);
            let result = super::validate_current_engine_snapshot_reference_integrity(&snapshot);
            let error = result.expect_err(&format!("{family} hostile payload must be rejected"));
            match mutation {
                Mutation::MissingFixture
                | Mutation::MissingBeamFixture
                | Mutation::MissingPlacementFixture => assert!(
                    matches!(
                        error,
                        super::ProjectReferenceIntegrityError::MissingReference {
                            target_domain: "fixture",
                            target_id: 99,
                            ..
                        }
                    ),
                    "{family} missing-fixture rejection mismatched: {error}"
                ),
                Mutation::UnsupportedAttribute | Mutation::UnsupportedBeamFeature => assert!(
                    matches!(
                        error,
                        super::ProjectReferenceIntegrityError::ReferenceMismatch { .. }
                    ),
                    "{family} unsupported-attribute rejection mismatched: {error}"
                ),
                Mutation::UnresolvableFeature => assert!(
                    matches!(
                        error,
                        super::ProjectReferenceIntegrityError::ReferenceMismatch { .. }
                    ) && error.to_string().contains("supported by none"),
                    "{family} unresolvable-feature rejection mismatched: {error}"
                ),
            }
        }
    }

    #[test]
    fn cue_owned_effect_params_hostile_references_are_rejected_per_family() {
        let mut snapshot = payload_test_snapshot();
        snapshot.effects.push(payload_effect(1));

        // PositionWave has no EffectSummary slot; it embeds through Cue-owned
        // params, which must be validated with the same rigor.
        snapshot.cues[0]
            .effect_targets
            .push(super::CueEffectTarget {
                effect_id: 1,
                enabled: true,
                params: Some(super::EffectParamsSnapshot::PositionWave(
                    payload_position_wave_request(),
                )),
                transition_ms: None,
            });
        assert!(super::validate_current_engine_snapshot_reference_integrity(&snapshot).is_ok());

        let mut missing_fixture = snapshot.clone();
        let super::EffectParamsSnapshot::PositionWave(request) = missing_fixture.cues[0]
            .effect_targets[0]
            .params
            .as_mut()
            .unwrap()
        else {
            panic!("params must remain PositionWave");
        };
        request.fixture_ids = vec![99];
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&missing_fixture),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                target_domain: "fixture",
                target_id: 99,
                ..
            })
        ));

        let mut unsupported = snapshot;
        let super::EffectParamsSnapshot::PositionWave(request) = unsupported.cues[0].effect_targets
            [0]
        .params
        .as_mut()
        .unwrap() else {
            panic!("params must remain PositionWave");
        };
        request.attribute = "Zoom".to_string();
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&unsupported),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { .. })
        ));
    }

    #[test]
    fn runtime_reference_fields_carry_no_hidden_dangling_refs() {
        let mut valid = reference_integrity_snapshot();
        valid.programmer.values = vec![super::ProgrammerValueSummary {
            fixture_id: 1,
            attribute: "Dimmer".to_string(),
            value: 32_768,
        }];
        valid.active_fade = Some(super::ActiveFadeSummary {
            cue_id: 1,
            progress: 0.5,
            remaining_ms: 100,
            paused: false,
        });
        valid.direct_child_timeline_transports = vec![super::DirectChildTimelineTransportSummary {
            cue_id: 1,
            position_ms: 10,
            duration_ms: 100,
            playing: true,
            generation: 3,
            count_in_remaining_ms: 0,
        }];
        valid.cue_live_modifiers = vec![super::CueLiveModifierState {
            cue_id: 1,
            speed: 2.0,
            size: 1.0,
            phase: 0.0,
            direction: super::CueLiveDirection::Authored,
            segment: 0,
        }];
        assert!(super::validate_current_engine_snapshot_reference_integrity(&valid).is_ok());

        let mut hostile_programmer = valid.clone();
        hostile_programmer.programmer.values[0].fixture_id = 99;
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&hostile_programmer),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                owner,
                target_domain: "fixture",
                target_id: 99,
                ..
            }) if owner == "programmer value"
        ));

        let mut hostile_programmer_attribute = valid.clone();
        hostile_programmer_attribute.programmer.values[0].attribute = "Zoom".to_string();
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(
                &hostile_programmer_attribute
            ),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { .. })
        ));

        let mut hostile_fade = valid.clone();
        hostile_fade.active_fade.as_mut().unwrap().cue_id = 99;
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&hostile_fade),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                owner,
                target_domain: "Cue",
                target_id: 99,
                ..
            }) if owner == "active fade"
        ));

        let mut hostile_transport = valid.clone();
        hostile_transport.direct_child_timeline_transports[0].cue_id = 99;
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&hostile_transport),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                owner,
                target_domain: "Cue",
                target_id: 99,
                ..
            }) if owner == "direct child Timeline transport"
        ));

        let mut hostile_modifier = valid;
        hostile_modifier.cue_live_modifiers[0].cue_id = 99;
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&hostile_modifier),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                owner,
                target_domain: "Cue",
                target_id: 99,
                ..
            }) if owner == "scene live modifier"
        ));
    }

    #[test]
    fn timeline_audio_clip_zero_identity_is_rejected_like_video_clips() {
        fn audio_clip(id: super::TimelineAudioClipId) -> super::TimelineAudioClipSummary {
            super::TimelineAudioClipSummary {
                id,
                // Layer 0 is the explicit legacy lane identity, so identity
                // rejections below cannot be shadowed by lane-kind checks.
                layer_id: 0,
                media_asset_id: None,
                path: String::new(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 0,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus: super::TimelineAudioOutputBus::Program,
            }
        }

        let mut active_hostile = reference_integrity_snapshot();
        active_hostile.timeline.audio_clips = vec![audio_clip(0)];
        active_hostile.timeline_bank[0] = active_hostile.timeline.clone();
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&active_hostile),
            Err(super::ProjectReferenceIntegrityError::ZeroId {
                domain: "Timeline audio clip",
                ..
            })
        ));

        // An inactive bank entry is validated with the same taxonomy.
        let mut bank_only_hostile = reference_integrity_snapshot();
        let mut inactive = bank_only_hostile.timeline_bank[0].clone();
        inactive.id = super::TimelineId(2);
        inactive.label = "Inactive".to_string();
        inactive.audio_clips = vec![audio_clip(0)];
        bank_only_hostile.timeline_bank.push(inactive);
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&bank_only_hostile),
            Err(super::ProjectReferenceIntegrityError::ZeroId {
                domain: "Timeline audio clip",
                ..
            })
        ));

        let mut child_hostile = reference_integrity_snapshot();
        child_hostile.cues[0].child_timeline = Some(super::ChildTimelineSummary {
            audio_clips: vec![audio_clip(0)],
            ..super::ChildTimelineSummary::default()
        });
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&child_hostile),
            Err(super::ProjectReferenceIntegrityError::ZeroId {
                domain: "Timeline audio clip",
                ..
            })
        ));

        let mut duplicate = reference_integrity_snapshot();
        duplicate.timeline.audio_clips = vec![audio_clip(700), audio_clip(700)];
        duplicate.timeline_bank[0] = duplicate.timeline.clone();
        match super::validate_current_engine_snapshot_reference_integrity(&duplicate) {
            Err(super::ProjectReferenceIntegrityError::DuplicateId { domain, id }) => {
                assert_eq!(domain, "Timeline audio clip");
                assert_eq!(id, 700);
            }
            other => panic!("expected Timeline audio clip DuplicateId(700), got {other:?}"),
        }
    }

    #[test]
    fn authored_video_persisted_state_is_compared_exactly() {
        let base = || {
            let mut snapshot = reference_integrity_snapshot();
            snapshot.authored_video = Some(snapshot.video.clone());
            snapshot
        };
        assert!(super::validate_current_engine_snapshot_reference_integrity(&base()).is_ok());

        let mut presets = base();
        presets.authored_video.as_mut().unwrap().mapping_presets =
            vec![super::VideoOutputMappingPresetSummary {
                label: "Preset".to_string(),
                mapping: super::VideoOutputMapping::default(),
            }];
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&presets),
            Err(
                super::ProjectReferenceIntegrityError::AuthoredVideoConflict {
                    kind: super::ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
                    detail
                }
            ) if detail.contains("mapping presets")
        ));

        let mut opacity = base();
        opacity.authored_video.as_mut().unwrap().master_opacity = 0.25;
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&opacity),
            Err(
                super::ProjectReferenceIntegrityError::AuthoredVideoConflict {
                    kind: super::ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
                    detail
                }
            ) if detail.contains("master opacity")
        ));

        let mut blackout = base();
        blackout.authored_video.as_mut().unwrap().blackout = true;
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&blackout),
            Err(
                super::ProjectReferenceIntegrityError::AuthoredVideoConflict {
                    kind: super::ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
                    detail
                }
            ) if detail.contains("blackout")
        ));

        let mut eligible_layers = base();
        eligible_layers
            .authored_video
            .as_mut()
            .unwrap()
            .auto_vj
            .config
            .eligible_layer_ids = vec![10];
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&eligible_layers),
            Err(
                super::ProjectReferenceIntegrityError::AuthoredVideoConflict {
                    kind: super::ProjectAuthoredVideoConflictKind::ReferenceTopologyMismatch,
                    detail
                }
            ) if detail.contains("Auto VJ config")
        ));

        // Runtime Auto VJ status is deliberately not compared.
        let mut status_only = base();
        status_only.video.auto_vj.status.mode = super::AutoVjMode::Running;
        assert!(super::validate_current_engine_snapshot_reference_integrity(&status_only).is_ok());
    }

    #[test]
    fn reference_cycle_detection_is_iterative_bounded_and_deterministic() {
        const DEPTH: u64 = 20_000;
        let deep_chain = (1..=DEPTH)
            .map(|cue_id| {
                let next = if cue_id < DEPTH { cue_id + 1 } else { 1 };
                super::CueSummary {
                    id: cue_id,
                    cue_list_id: 1,
                    child_timeline: Some(super::ChildTimelineSummary {
                        events: vec![super::TimelineCueEventSummary {
                            id: 4_000 + cue_id,
                            cue_id: next,
                            ..super::TimelineCueEventSummary::default()
                        }],
                        ..super::ChildTimelineSummary::default()
                    }),
                    ..super::CueSummary::default()
                }
            })
            .collect::<Vec<_>>();
        let mut cyclic = reference_integrity_snapshot();
        cyclic.cues = deep_chain;
        let result = super::validate_child_timeline_reference_cycles(&cyclic.cues);
        let mut expected_path = Vec::<u64>::new();
        expected_path.extend(1..=DEPTH);
        expected_path.push(1);
        assert_eq!(
            result,
            Err(super::ProjectReferenceIntegrityError::ReferenceCycle {
                domain: "Cue child Timeline",
                path: expected_path,
            })
        );

        // The same depth without the closing edge must complete with a typed
        // success instead of exhausting the call stack.
        let mut acyclic = reference_integrity_snapshot();
        acyclic.cues = (1..=DEPTH)
            .map(|cue_id| {
                let next = cue_id + 1;
                super::CueSummary {
                    id: cue_id,
                    cue_list_id: 1,
                    child_timeline: (cue_id < DEPTH).then(|| super::ChildTimelineSummary {
                        events: vec![super::TimelineCueEventSummary {
                            id: 4_000 + cue_id,
                            cue_id: next,
                            ..super::TimelineCueEventSummary::default()
                        }],
                        ..super::ChildTimelineSummary::default()
                    }),
                    ..super::CueSummary::default()
                }
            })
            .collect::<Vec<_>>();
        assert!(super::validate_child_timeline_reference_cycles(&acyclic.cues).is_ok());

        // A self-reference reports the minimal deterministic cycle path.
        let mut self_cycle = reference_integrity_snapshot();
        self_cycle.cues[0].child_timeline = Some(super::ChildTimelineSummary {
            events: vec![super::TimelineCueEventSummary {
                id: 400,
                cue_id: 1,
                ..super::TimelineCueEventSummary::default()
            }],
            ..super::ChildTimelineSummary::default()
        });
        assert_eq!(
            super::validate_child_timeline_reference_cycles(&self_cycle.cues),
            Err(super::ProjectReferenceIntegrityError::ReferenceCycle {
                domain: "Cue child Timeline",
                path: vec![1, 1],
            })
        );
    }

    fn move_summary_effect(summary_id: super::EffectId) -> super::EffectSummary {
        super::EffectSummary {
            id: summary_id,
            label: "Move summary".to_string(),
            effect_type: super::EffectKind::Move,
            fixture_ids: vec![1],
            target_group_ids: vec!["front".to_string()],
            attribute: "Pan/Tilt".to_string(),
            video_targets: Vec::new(),
            shape: super::LfoShape::Sine,
            period_ms: Some(1_000),
            clock_sync: None,
            low: 0,
            high: u16::MAX,
            phase: 0.0,
            fixture_spread: 0.0,
            blend_mode: super::EffectBlendMode::Override,
            origin: None,
            direction: None,
            speed: None,
            wavelength: None,
            enabled: true,
            lfo: None,
            color: None,
            chaser: None,
            move_effect: Some(payload_move_request()),
            value: None,
            curve: None,
            mapping: None,
            color_mapping: None,
        }
    }

    fn move_test_fixture(
        id: super::FixtureId,
        movement_controls: &[(&str, u16)],
    ) -> super::PatchedFixtureSummary {
        let mut fixture = reference_integrity_fixture();
        fixture.id = id;
        fixture.label = format!("Mover {id}");
        for (attribute, offset) in movement_controls {
            fixture.controls.push(super::AttributeControl {
                attribute: (*attribute).to_string(),
                channel_name: (*attribute).to_string(),
                geometry: None,
                offsets: vec![*offset],
                resolution: super::AttributeResolution::SixteenBit,
                default_value: 0,
                functions: Vec::new(),
            });
        }
        fixture
    }

    #[test]
    fn move_summary_accepts_virtual_pan_tilt_mirror_over_distinct_controls() {
        let mut snapshot = payload_test_snapshot();
        snapshot.fixtures = vec![move_test_fixture(1, &[("Pan coarse", 2), ("tilt-fine", 4)])];
        snapshot.effects.push(move_summary_effect(500));
        assert!(super::validate_current_engine_snapshot_reference_integrity(&snapshot).is_ok());
    }

    #[test]
    fn move_payload_rejects_missing_or_unselected_axis_pairs() {
        for movement_controls in [
            vec![("Pan", 2)],
            vec![("Tilt", 4)],
            vec![("", 2), ("Tilt", 4)],
        ] {
            let mut snapshot = payload_test_snapshot();
            snapshot.fixtures = vec![move_test_fixture(1, &movement_controls)];
            snapshot.effects.push(move_summary_effect(500));
            assert!(matches!(
                super::validate_current_engine_snapshot_reference_integrity(&snapshot),
                Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { ref owner, .. })
                    if owner == "lighting effect 500 Move request beam target"
            ));
        }

        let mut snapshot = payload_test_snapshot();
        snapshot.fixtures = vec![move_test_fixture(
            1,
            &[("Pan 1", 2), ("Tilt 1", 4), ("Pan 2", 6), ("Tilt 2", 8)],
        )];
        let mut effect = move_summary_effect(500);
        effect.move_effect.as_mut().unwrap().beam_targets[0].beam_index = 2;
        snapshot.effects.push(effect);
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&snapshot),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { ref owner, .. })
                if owner == "lighting effect 500 Move request beam target"
        ));
    }

    #[test]
    fn move_payload_accepts_indexed_multi_beam_pair_and_rejects_ambiguous_unindexed_target() {
        let controls = &[("Pan 1", 2), ("Tilt 1", 4), ("Pan 2", 6), ("Tilt 2", 8)];
        let mut indexed = payload_test_snapshot();
        indexed.fixtures = vec![move_test_fixture(1, controls)];
        let mut indexed_effect = move_summary_effect(500);
        indexed_effect.move_effect.as_mut().unwrap().beam_targets[0].beam_index = 1;
        indexed.effects.push(indexed_effect);
        assert!(super::validate_current_engine_snapshot_reference_integrity(&indexed).is_ok());

        let mut unindexed = indexed.clone();
        unindexed.effects[0]
            .move_effect
            .as_mut()
            .unwrap()
            .beam_targets
            .clear();
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&unindexed),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { ref owner, .. })
                if owner == "lighting effect 500 Move request"
        ));
    }

    #[test]
    fn move_beam_target_requires_explicit_fixture_coverage_and_rejects_group_combination() {
        let controls = &[("Pan", 2), ("Tilt", 4)];
        let mut rejected = payload_test_snapshot();
        rejected.fixtures = vec![move_test_fixture(1, controls)];
        let mut rejected_effect = move_summary_effect(500);
        let rejected_request = rejected_effect.move_effect.as_mut().unwrap();
        rejected_request.fixture_ids.clear();
        rejected_request.target_group_ids.clear();
        rejected.effects.push(rejected_effect);
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&rejected),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch {
                ref owner,
                ref detail,
            }) if owner == "lighting effect 500 Move request beam target"
                && detail.contains("not present in the resolved fixture targets")
        ));

        let mut group_combined = rejected.clone();
        group_combined.effects.clear();
        let mut group_effect = move_summary_effect(501);
        let group_request = group_effect.move_effect.as_mut().unwrap();
        group_request.fixture_ids = vec![1];
        group_request.target_group_ids = vec!["front".to_string()];
        group_combined.effects.push(group_effect);
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&group_combined),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch {
                ref owner,
                ref detail,
            }) if owner == "lighting effect 501 Move request"
                && detail == "Move beam targets cannot be combined with group targets"
        ));
    }

    #[test]
    fn move_summary_and_payload_references_remain_fail_closed() {
        let mut wrong_virtual_attribute = payload_test_snapshot();
        wrong_virtual_attribute.fixtures = vec![move_test_fixture(1, &[("Pan", 2), ("Tilt", 4)])];
        let mut effect = move_summary_effect(500);
        effect.attribute = "Pan".to_string();
        wrong_virtual_attribute.effects.push(effect);
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&wrong_virtual_attribute),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { ref owner, .. })
                if owner == "lighting effect 500"
        ));

        let mut missing_summary = payload_test_snapshot();
        missing_summary.fixtures = vec![move_test_fixture(1, &[("Pan", 2), ("Tilt", 4)])];
        let mut effect = move_summary_effect(500);
        effect.fixture_ids = vec![99];
        missing_summary.effects.push(effect);
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&missing_summary),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                ref owner,
                target_domain: "fixture",
                target_id: 99,
                ..
            }) if owner == "lighting effect 500"
        ));

        let mut dangling_payload = payload_test_snapshot();
        dangling_payload.fixtures = vec![move_test_fixture(1, &[("Pan", 2), ("Tilt", 4)])];
        let mut effect = move_summary_effect(500);
        let request = effect.move_effect.as_mut().unwrap();
        request.fixture_ids = vec![99];
        request.beam_targets[0].fixture_id = 99;
        dangling_payload.effects.push(effect);
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&dangling_payload),
            Err(super::ProjectReferenceIntegrityError::MissingReference {
                ref owner,
                target_domain: "fixture",
                target_id: 99,
                ..
            }) if owner == "lighting effect 500 Move request"
        ));
    }

    #[test]
    fn non_move_pan_tilt_attribute_keeps_exact_validation() {
        let mut snapshot = payload_test_snapshot();
        snapshot.fixtures = vec![move_test_fixture(1, &[("Pan", 2), ("Tilt", 4)])];
        let mut effect = move_summary_effect(500);
        effect.effect_type = super::EffectKind::Lfo;
        effect.move_effect = None;
        effect.lfo = Some(payload_lfo_request());
        snapshot.effects.push(effect);
        assert!(matches!(
            super::validate_current_engine_snapshot_reference_integrity(&snapshot),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { .. })
        ));
    }

    fn registry_test_project() -> super::ProjectFile {
        let mut project = super::ProjectFile {
            version: 1,
            app: "Syndocal".to_string(),
            operator_policy: None,
            custom_profiles: Vec::new(),
            fixture_groups: vec![super::FixtureGroupSummary {
                id: "front".to_string(),
                label: "Front".to_string(),
                color: None,
            }],
            snapshot: payload_test_snapshot(),
        };
        project.snapshot.effects.push(payload_effect(500));
        project
    }

    #[test]
    fn snapshot_level_report_declares_external_group_registry_boundary() {
        let report =
            super::validate_current_engine_snapshot_reference_integrity(&payload_test_snapshot())
                .unwrap();
        assert_eq!(
            report.unvalidated_external_registry,
            Some(super::ProjectExternalRegistryBoundary::LIGHTING_FIXTURE_GROUPS)
        );
        assert_eq!(
            super::ProjectExternalRegistryBoundary::LIGHTING_FIXTURE_GROUPS.domain,
            "lighting fixture group"
        );
    }

    #[test]
    fn project_file_validation_closes_the_group_registry_boundary() {
        let project = registry_test_project();
        let report = super::validate_project_file_reference_integrity(&project).unwrap();
        assert_eq!(report.unvalidated_external_registry, None);
        assert_eq!(report.video_authority, super::ProjectVideoAuthority::Video);
        assert!(super::validate_project_file_fixture_group_registry(&project).is_ok());

        let mut duplicate_registry = registry_test_project();
        duplicate_registry
            .fixture_groups
            .push(super::FixtureGroupSummary {
                id: "front".to_string(),
                label: "Duplicate".to_string(),
                color: None,
            });
        assert!(matches!(
            super::validate_project_file_reference_integrity(&duplicate_registry),
            Err(super::ProjectReferenceIntegrityError::DuplicateGroupIdentity { group_id, .. })
                if group_id == "front"
        ));

        let mut malformed_membership = registry_test_project();
        malformed_membership.snapshot.fixtures[0].group_ids = vec!["a//b".to_string()];
        assert!(matches!(
            super::validate_project_file_fixture_group_registry(&malformed_membership),
            Err(super::ProjectReferenceIntegrityError::ReferenceMismatch { .. })
        ));

        let mut unresolved_target = registry_test_project();
        unresolved_target.snapshot.effects[0].target_group_ids = vec!["ghost".to_string()];
        assert!(matches!(
            super::validate_project_file_reference_integrity(&unresolved_target),
            Err(super::ProjectReferenceIntegrityError::MissingGroupReference { group_id, .. })
                if group_id == "ghost"
        ));

        // Hierarchical membership resolves parent group references exactly
        // like the engine's runtime rule, while sibling memberships keep the
        // other payload references resolvable.
        let mut hierarchical = registry_test_project();
        hierarchical.snapshot.fixtures[0].group_ids =
            vec!["front".to_string(), "stage/front/left".to_string()];
        hierarchical.fixture_groups.insert(
            0,
            super::FixtureGroupSummary {
                id: "stage/front".to_string(),
                label: "Stage front".to_string(),
                color: None,
            },
        );
        hierarchical.snapshot.effects[0].target_group_ids = vec!["stage/front".to_string()];
        assert!(super::validate_project_file_fixture_group_registry(&hierarchical).is_ok());
    }
}

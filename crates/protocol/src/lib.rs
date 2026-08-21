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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoOutputAspectMode {
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

impl Default for VideoOutputAspectMode {
    fn default() -> Self {
        Self::Stretch
    }
}

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
fn legacy_video_layer_transport_to_clip_slot(
    state: &VideoLayerState,
    layer_id: VideoLayerId,
) -> Result<
    (
        u64,
        Option<u64>,
        VideoClipLoopMode,
        f32,
        Vec<VideoClipCuePointSummary>,
    ),
    String,
> {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelinePhaseRole {
    Intro,
    Verse,
    PreChorus,
    Chorus,
    Bridge,
    Breakdown,
    Outro,
    Custom,
}

impl Default for TimelinePhaseRole {
    fn default() -> Self {
        Self::Custom
    }
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineFollowLightingPolicy {
    HoldThenCut,
    LinearMerge,
}

impl Default for TimelineFollowLightingPolicy {
    fn default() -> Self {
        Self::HoldThenCut
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineFollowFaultPolicy {
    Hold,
    Cut,
    Fault,
}

impl Default for TimelineFollowFaultPolicy {
    fn default() -> Self {
        Self::Hold
    }
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
    #[serde(default)]
    pub fault_policy: TimelineFollowFaultPolicy,
}

const fn default_timeline_follow_trans_cadence_bars() -> u16 {
    4
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
    Phase { phase_id: TimelinePhaseId },
    Looping,
    Break,
    Trans,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineGuideCueSummary {
    pub generation: u64,
    pub sequence: u64,
    pub at_ms: u64,
    pub label: String,
    pub cue: TimelineGuideCueKind,
}

/// One authoritative authored image for the advanced Timeline surfaces. The
/// established Scene Block, automation and lane editors keep their existing
/// commands; media placement, Phase, grouping, A-B and Follow commit together
/// through this image so linked A/V items can never tear across publication.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TimelineAdvancedAuthoringSummary {
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
            count_in_remaining_ms: 0,
            audio_transport_revision: 0,
            transport_epoch: 0,
            transport_generation: 0,
            active_child_transports: Vec::new(),
            loop_runtime: TimelineLoopRuntimeSummary::default(),
            follow_runtime: TimelineFollowRuntimeSummary::default(),
            guide_cues: Vec::new(),
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DmxOutputProtocol {
    ArtNet,
    Sacn,
    EnttecUsbPro,
    DmxKingUltraDmx,
    EnttecOpenDmx,
}

impl Default for DmxOutputProtocol {
    fn default() -> Self {
        Self::ArtNet
    }
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MachineOutputRole {
    Lighting,
    Video,
    Both,
    Standby,
}

impl Default for MachineOutputRole {
    fn default() -> Self {
        Self::Both
    }
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutputOwnershipState {
    Ready,
    Transitioning,
    Activating,
    Failed,
}

impl Default for OutputOwnershipState {
    fn default() -> Self {
        Self::Ready
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClockSource {
    Manual,
    Tap,
    MidiClock,
    MidiTimecode,
    Ltc,
    AbletonLink,
}

impl Default for ClockSource {
    fn default() -> Self {
        Self::Manual
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    /// Additive process-local DJ Link truth.  The token is intentionally not
    /// represented anywhere in this status DTO.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dj_link: Option<DjLinkRuntimeStatus>,
}

/// DJ Link is a dedicated, authenticated LAN protocol. It intentionally does
/// not reuse the generic remote PIN or expose the credential in any status
/// payload.
pub const DJ_LINK_PROTOCOL_VERSION: u8 = 1;
pub const DJ_LINK_MAX_FRAME_BYTES: usize = 64 * 1024;
pub const DJ_LINK_MAX_STRING_BYTES: usize = 256;
pub const DJ_LINK_MIN_TOKEN_BYTES: usize = 32;
pub const DJ_LINK_MAX_CAPABILITIES: usize = 32;
pub const DJ_LINK_MAX_MAPPINGS: usize = 128;
pub const DJ_LINK_MAX_SEQUENCE: u64 = 9_007_199_254_740_991;
pub const DJ_LINK_BEATS_PER_BAR: u8 = 4;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DjLinkMessageType {
    #[serde(rename = "DJ_AGENT_HELLO")]
    Hello,
    #[serde(rename = "DJ_HEARTBEAT")]
    Heartbeat,
    #[serde(rename = "DJ_MASTER_CHANGED")]
    MasterChanged,
    #[serde(rename = "DJ_MASTER_TRACK_ACTIVE")]
    MasterTrackActive,
    #[serde(rename = "DJ_LOOP_STATE")]
    LoopState,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkHelloPayload {
    #[serde(rename = "authToken")]
    pub auth_token: String,
    pub version: u8,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct DjLinkHeartbeatPayload {
    #[serde(default)]
    pub at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkMasterTrackPayload {
    #[serde(default, rename = "contentId")]
    pub content_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    /// Deck identity is required for a track-active event. A string keeps
    /// peer/device identifiers opaque and bounded instead of guessing a
    /// numeric deck mapping.
    pub deck: String,
    #[serde(default, rename = "deckId")]
    pub deck_id: Option<String>,
    #[serde(default, rename = "trackBpm")]
    pub track_bpm: Option<f64>,
    #[serde(default, rename = "positionSec")]
    pub position_sec: Option<f64>,
    #[serde(default, rename = "startedAt")]
    pub started_at: Option<String>,
    #[serde(rename = "playSessionId")]
    pub play_session_id: String,
    #[serde(rename = "isPlaying", alias = "playing")]
    pub playing: bool,
    #[serde(default = "default_true")]
    pub master: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkMasterChangedPayload {
    #[serde(default, rename = "masterDeck")]
    pub master_deck: Option<String>,
    #[serde(default)]
    pub deck: Option<String>,
    #[serde(default, rename = "isPlaying", alias = "playing")]
    pub playing: bool,
    #[serde(default = "default_true")]
    pub master: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkLoopStatePayload {
    pub division: u8,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct DjLinkReleasePayload {
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkStateSyncPayload {
    #[serde(default, rename = "loopDivision")]
    pub loop_division: Option<u8>,
    pub released: bool,
    #[serde(default, rename = "masterDeck")]
    pub master_deck: Option<String>,
    #[serde(default, rename = "masterTrack")]
    pub master_track: Option<DjLinkMasterTrackStatePayload>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkTimelineLoopSetPayload {
    pub active: bool,
    #[serde(rename = "timelineId")]
    pub timeline_id: String,
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
    #[serde(rename = "timelineId")]
    pub timeline_id: String,
    #[serde(rename = "positionBars")]
    pub position_bars: u64,
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
        Ok(())
    }
}

/// The peer's final generic-json adapter is deliberately flat: only HELLO
/// carries the adapter protocol and token, while event payload fields live at
/// the envelope root. The older v1 `{v,agentId,sessionId,payload}` envelope is
/// kept separately for compatibility and is never used as a silent fallback
/// for an unknown flat protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DjLinkFlatFrame {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(rename = "eventId")]
    pub event_id: String,
    pub sequence: u64,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(flatten)]
    pub payload: serde_json::Map<String, serde_json::Value>,
}

impl DjLinkFlatFrame {
    pub fn parse_json(text: &str) -> Result<Self, String> {
        if text.len() > DJ_LINK_MAX_FRAME_BYTES {
            return Err("DJ Link frame exceeds the bounded size".to_string());
        }
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let field_names = value
            .as_object()
            .ok_or_else(|| "DJ Link flat frame must be an object".to_string())?
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let frame: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        validate_dj_link_string(&frame.message_type, "type")?;
        validate_dj_link_string(&frame.event_id, "eventId")?;
        if frame.sequence == 0 || frame.sequence > DJ_LINK_MAX_SEQUENCE {
            return Err("DJ Link sequence must be a positive safe integer".to_string());
        }
        let known = matches!(
            frame.message_type.as_str(),
            "DJ_AGENT_HELLO"
                | "DJ_AGENT_HEARTBEAT"
                | "DJ_HEARTBEAT"
                | "DJ_MASTER_CHANGED"
                | "DJ_MASTER_TRACK_ACTIVE"
                | "DJ_LOOP_STATE"
                | "DJ_RELEASE"
                | "DJ_STATE_SYNC"
                | "DJ_TIMELINE_STATE_REQUEST"
                | "DJ_TIMELINE_BEAT_JUMP"
                | "DJ_TIMELINE_LOOP_SET"
        );
        if !known {
            return Err(format!(
                "unknown DJ Link flat message type {}",
                frame.message_type
            ));
        }
        if frame.message_type == "DJ_AGENT_HELLO" {
            if frame.protocol.as_deref() != Some("generic-json") {
                return Err("DJ Link HELLO protocol must be generic-json".to_string());
            }
            let token = frame
                .token
                .as_deref()
                .ok_or_else(|| "DJ Link HELLO token is required".to_string())?;
            if token.len() < DJ_LINK_MIN_TOKEN_BYTES || token.len() > DJ_LINK_MAX_STRING_BYTES {
                return Err("DJ Link HELLO token length is invalid".to_string());
            }
            for capability in &frame.capabilities {
                validate_dj_link_string(capability, "capability")?;
            }
        }
        for key in &field_names {
            validate_dj_link_string(key, "field name")?;
        }
        Ok(frame)
    }

    pub fn is_hello(&self) -> bool {
        self.message_type == "DJ_AGENT_HELLO"
    }

    pub fn to_envelope(&self, agent_id: &str, session_id: &str) -> Result<DjLinkEnvelope, String> {
        validate_dj_link_string(agent_id, "agentId")?;
        validate_dj_link_string(session_id, "sessionId")?;
        let message_type = match self.message_type.as_str() {
            "DJ_AGENT_HELLO" => DjLinkMessageType::Hello,
            "DJ_AGENT_HEARTBEAT" | "DJ_HEARTBEAT" => DjLinkMessageType::Heartbeat,
            "DJ_MASTER_CHANGED" => DjLinkMessageType::MasterChanged,
            "DJ_MASTER_TRACK_ACTIVE" => DjLinkMessageType::MasterTrackActive,
            "DJ_LOOP_STATE" => DjLinkMessageType::LoopState,
            "DJ_RELEASE" => DjLinkMessageType::Release,
            "DJ_STATE_SYNC" => DjLinkMessageType::StateSync,
            "DJ_TIMELINE_STATE_REQUEST" => DjLinkMessageType::TimelineStateRequest,
            "DJ_TIMELINE_BEAT_JUMP" => DjLinkMessageType::TimelineBeatJump,
            "DJ_TIMELINE_LOOP_SET" => DjLinkMessageType::TimelineLoopSet,
            _ => return Err("unknown DJ Link flat message type".to_string()),
        };
        let mut payload = self.payload.clone();
        if self.is_hello() {
            let token = self
                .token
                .clone()
                .ok_or_else(|| "DJ Link HELLO token is required".to_string())?;
            payload.insert("authToken".to_string(), serde_json::Value::String(token));
            payload.insert("version".to_string(), serde_json::Value::from(1u8));
            payload.insert(
                "capabilities".to_string(),
                serde_json::to_value(&self.capabilities).map_err(|e| e.to_string())?,
            );
        }
        let envelope = DjLinkEnvelope {
            v: DJ_LINK_PROTOCOL_VERSION,
            message_type,
            agent_id: agent_id.to_string(),
            session_id: session_id.to_string(),
            sequence: self.sequence,
            event_id: self.event_id.clone(),
            payload: serde_json::Value::Object(payload),
        };
        envelope.validate()?;
        Ok(envelope)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DjLinkMasterTrackStatePayload {
    #[serde(default, rename = "contentId")]
    pub content_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(rename = "isPlaying", alias = "playing")]
    pub playing: bool,
    #[serde(default, rename = "playSessionId")]
    pub play_session_id: Option<String>,
    #[serde(default, rename = "trackBpm")]
    pub track_bpm: Option<f64>,
    #[serde(default, rename = "positionSec")]
    pub position_sec: Option<f64>,
    #[serde(default, rename = "startedAt")]
    pub started_at: Option<String>,
    #[serde(default, rename = "deckId")]
    pub deck_id: Option<String>,
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
    /// Additive handoff-compatible ACK fields. Existing v1 consumers use the
    /// outcome/code fields; timeline-control peers consume ok/message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
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
    pub master: bool,
    #[serde(rename = "trackActive")]
    pub track_active: bool,
    #[serde(rename = "loopDivision")]
    pub loop_division: Option<u8>,
    pub released: bool,
    #[serde(rename = "lastEventId")]
    pub last_event_id: Option<String>,
    #[serde(rename = "ageMs")]
    pub age_ms: Option<u64>,
    #[serde(default, rename = "masterDeck")]
    pub master_deck: Option<String>,
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

impl DjLinkEnvelope {
    pub fn parse_json(text: &str) -> Result<Self, String> {
        if text.len() > DJ_LINK_MAX_FRAME_BYTES {
            return Err("DJ Link frame exceeds the bounded size".to_string());
        }
        let envelope: Self = serde_json::from_str(text).map_err(|error| error.to_string())?;
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.v != DJ_LINK_PROTOCOL_VERSION {
            return Err("unsupported DJ Link protocol version".to_string());
        }
        validate_dj_link_string(&self.agent_id, "agentId")?;
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
                if hello.version != DJ_LINK_PROTOCOL_VERSION
                    || hello.auth_token.len() < DJ_LINK_MIN_TOKEN_BYTES
                    || hello.auth_token.len() > DJ_LINK_MAX_STRING_BYTES
                    || hello.capabilities.len() > DJ_LINK_MAX_CAPABILITIES
                {
                    return Err(
                        "DJ Link HELLO authentication/version/capabilities are invalid".to_string(),
                    );
                }
                for capability in hello.capabilities {
                    validate_dj_link_string(&capability, "capability")?;
                }
            }
            DjLinkMessageType::Heartbeat => {
                let _: DjLinkHeartbeatPayload = parse_dj_link_payload(&self.payload)?;
            }
            DjLinkMessageType::MasterChanged => {
                let payload: DjLinkMasterChangedPayload = parse_dj_link_payload(&self.payload)?;
                if let Some(deck) = payload.master_deck.as_deref().or(payload.deck.as_deref()) {
                    validate_dj_link_string(deck, "masterDeck")?;
                }
            }
            DjLinkMessageType::MasterTrackActive => {
                let payload: DjLinkMasterTrackPayload = parse_dj_link_payload(&self.payload)?;
                validate_dj_link_track_payload(&payload)?;
            }
            DjLinkMessageType::LoopState => {
                let payload: DjLinkLoopStatePayload = parse_dj_link_payload(&self.payload)?;
                if payload.division > 63 {
                    return Err("DJ Link loop division exceeds the safe bound".to_string());
                }
            }
            DjLinkMessageType::Release => {
                let payload: DjLinkReleasePayload = parse_dj_link_payload(&self.payload)?;
                if let Some(state) = payload.state.as_deref() {
                    validate_dj_link_string(state, "state")?;
                }
            }
            DjLinkMessageType::StateSync => {
                let payload: DjLinkStateSyncPayload = parse_dj_link_payload(&self.payload)?;
                if let Some(deck) = payload.master_deck.as_deref() {
                    validate_dj_link_string(deck, "masterDeck")?;
                }
                if let Some(loop_division) = payload.loop_division {
                    if loop_division > 63 {
                        return Err("DJ Link loop division exceeds the safe bound".to_string());
                    }
                }
                if let Some(track) = payload.master_track {
                    for (value, label) in [
                        (track.content_id.as_deref(), "contentId"),
                        (track.title.as_deref(), "title"),
                        (track.artist.as_deref(), "artist"),
                        (track.play_session_id.as_deref(), "playSessionId"),
                    ] {
                        if let Some(value) = value {
                            validate_dj_link_string(value, label)?;
                        }
                    }
                    if track.track_bpm.is_some_and(|value| {
                        !value.is_finite() || !(0.0..=1_000.0).contains(&value)
                    }) || track
                        .position_sec
                        .is_some_and(|value| !value.is_finite() || value < 0.0)
                    {
                        return Err("DJ Link state sync numeric fields are invalid".to_string());
                    }
                }
            }
            DjLinkMessageType::TimelineStateRequest => {
                let _: DjLinkTimelineStateRequestPayload = parse_dj_link_payload(&self.payload)?;
            }
            DjLinkMessageType::TimelineBeatJump => {
                let payload: DjLinkTimelineBeatJumpPayload = parse_dj_link_payload(&self.payload)?;
                if !matches!(payload.bars, -4 | 4) {
                    return Err("DJ timeline beat jump must be exactly -4 or 4 bars".to_string());
                }
                validate_dj_link_string(&payload.timeline_id, "timelineId")?;
            }
            DjLinkMessageType::TimelineLoopSet => {
                let payload: DjLinkTimelineLoopSetPayload = parse_dj_link_payload(&self.payload)?;
                validate_dj_link_string(&payload.timeline_id, "timelineId")?;
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

fn validate_dj_link_track_payload(payload: &DjLinkMasterTrackPayload) -> Result<(), String> {
    validate_dj_link_string(&payload.play_session_id, "playSessionId")?;
    validate_dj_link_string(&payload.deck, "deck")?;
    for (value, label) in [
        (payload.content_id.as_deref(), "contentId"),
        (payload.title.as_deref(), "title"),
        (payload.artist.as_deref(), "artist"),
    ] {
        if let Some(value) = value {
            validate_dj_link_string(value, label)?;
        }
    }
    if let Some(deck_id) = payload.deck_id.as_deref() {
        validate_dj_link_string(deck_id, "deckId")?;
    }
    if let Some(started_at) = payload.started_at.as_deref() {
        validate_dj_link_string(started_at, "startedAt")?;
    }
    if payload
        .track_bpm
        .is_some_and(|value| !value.is_finite() || !(0.0..=1_000.0).contains(&value))
        || payload
            .position_sec
            .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err("DJ Link track numeric fields are invalid".to_string());
    }
    if !payload.playing {
        return Err("DJ Link track event must identify a playing master track".to_string());
    }
    if payload.master
        && payload
            .content_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        && payload.title.as_deref().unwrap_or("").trim().is_empty()
    {
        return Err("DJ Link master track event has no track identity".to_string());
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
        video_output_mapping_field_value, AudioSpectrumBand, AudioSpectrumSource, CueListSummary,
        NodeGraphAudioNode, PlaybackExecutorSummary, VideoOutputMapping, DEFAULT_CUE_LIST_ID,
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

        let mut cue = super::CueSummary::default();
        cue.id = 21;
        cue.steps = vec![
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
        ];

        let encoded = serde_json::to_vec(&cue).unwrap();
        let decoded: super::CueSummary = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, cue);
    }

    #[test]
    fn cue_scene_matrix_fields_roundtrip_and_default_for_legacy_json() {
        let mut cue = super::CueSummary::default();
        cue.id = 18;
        cue.group_id = Some("Front/Wash".to_string());
        cue.recall_mode = super::RecallMode::ReplaceGroup;

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
        let mut cue = super::CueSummary::default();
        cue.id = 17;
        cue.authored_beats = Some(4.0);

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
            });

        super::normalize_timeline_bank(&mut snapshot);
        assert_eq!(snapshot.timeline_bank.len(), 1);
        assert_eq!(snapshot.timeline_bank[0].id, super::TimelineId(41));
        assert!(!snapshot.timeline_bank[0].playing);
        assert_eq!(snapshot.timeline_bank[0].position_ms, 0);
        assert!(snapshot.timeline_bank[0].guide_cues.is_empty());

        let mut second = super::TimelineSnapshot::default();
        second.id = super::TimelineId(42);
        second.label = "Verse".to_string();
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
        let mut timeline = super::TimelineSnapshot::default();
        timeline.id = super::TimelineId(91);
        timeline.label = "Runtime-free Follow".to_string();
        timeline.follow_runtime = super::TimelineFollowRuntimeSummary {
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
                            consumer_id: super::TimelineFollowSettlementConsumerId::VideoOutput {
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
        let mut timeline = super::TimelineSnapshot::default();
        timeline.id = super::TimelineId(101);
        timeline.label = "Follow bounds".to_string();
        timeline.duration_ms = super::TIMELINE_FOLLOW_MAX_PREROLL_MS;
        timeline.follow = Some(super::TimelineFollowSummary {
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
            fault_policy: super::TimelineFollowFaultPolicy::Cut,
        });
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
    fn active_child_transport_runtime_is_never_serialized() {
        let mut snapshot = super::TimelineSnapshot::default();
        snapshot.active_child_transports = vec![
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
            },
            super::ChildTimelineTransportRuntimeSummary {
                owner_cue_id: 8,
                root: super::ChildTimelineTransportRootSummary::Follow {
                    source_timeline_id: super::TimelineId(91),
                    generation: 17,
                },
                path: Vec::new(),
                position_ms: 1_000,
            },
        ];

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
            }],
            audio_offset_ms: -250,
            audio_muted: true,
            ..super::TimelineSnapshot::default()
        };

        let encoded = serde_json::to_string(&snapshot).unwrap();
        let decoded: super::TimelineSnapshot = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, snapshot);
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
        let mut cue = super::CueSummary::default();
        cue.id = 17;
        cue.effect_targets = vec![
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
        ];

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
                let value = super::VideoSnapshot {
                    layers: vec![media_asset_test_layer(1, "Legacy", "C:/show/a.mp4")],
                    media_assets: vec![
                        media_asset_test_asset(3, "A", "C:/show/a.mp4"),
                        media_asset_test_asset(3, "B", "C:/show/b.mp4"),
                    ],
                    ..super::VideoSnapshot::default()
                };
                value
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
        let mut engine = super::EngineSnapshot::default();
        engine.video_clip_runtime = runtime.clone();
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
            };
            assert!(super::validate_video_clip_runtime_snapshot(&invalid).is_err());
        }

        let inactive_queued_only = super::VideoClipRuntimeSnapshot {
            layers: vec![super::VideoClipLayerRuntimeSummary {
                layer_id: 1,
                queued_slot_id: Some(super::VideoClipSlotId(21)),
                ..super::VideoClipLayerRuntimeSummary::default()
            }],
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

        let mut cue = super::CueSummary::default();
        cue.id = 12;
        cue.color = Some("#ff3366".to_string());
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
        let mut cue = super::CueSummary::default();
        cue.id = 9;
        cue.child_timeline = Some(super::ChildTimelineSummary {
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
            }],
            duration_ms: 1_000,
            ..super::ChildTimelineSummary::default()
        });
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

        let mut engine = super::EngineSnapshot::default();
        engine.video_transition_runtime = runtime.clone();
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
        let text = r#"{"v":1,"type":"DJ_MASTER_TRACK_ACTIVE","agentId":"rekordbox","sessionId":"s1","sequence":1,"eventId":"e1","payload":{"contentId":"abc","playSessionId":"p1","isPlaying":true,"master":true,"deck":"A","deckId":"deck-a","trackBpm":128.0,"positionSec":1.25,"startedAt":"2026-08-21T00:00:00Z"}}"#;
        let envelope = super::DjLinkEnvelope::parse_json(text).unwrap();
        assert_eq!(
            envelope.message_type,
            super::DjLinkMessageType::MasterTrackActive
        );
        let shape = envelope.canonical_shape().unwrap();
        assert!(shape.contains("DJ_MASTER_TRACK_ACTIVE"));
        assert!(super::DjLinkEnvelope::parse_json(
            &text.replace("\"sequence\":1", "\"sequence\":0")
        )
        .is_err());
        assert!(super::DjLinkEnvelope::parse_json(
            &text.replace("\"payload\":", "\"extra\":1,\"payload\":")
        )
        .is_err());
    }

    #[test]
    fn dj_link_peer_wire_fixtures_are_strict_and_distinct() {
        let hello = r#"{"v":1,"type":"DJ_AGENT_HELLO","agentId":"rekordbox","sessionId":"s1","sequence":1,"eventId":"hello-1","payload":{"authToken":"0123456789abcdef0123456789abcdef","version":1,"capabilities":["track","loop"]}}"#;
        assert_eq!(
            super::DjLinkEnvelope::parse_json(hello)
                .unwrap()
                .message_type,
            super::DjLinkMessageType::Hello
        );
        let sync = r#"{"v":1,"type":"DJ_STATE_SYNC","agentId":"rekordbox","sessionId":"s1","sequence":2,"eventId":"sync-1","payload":{"loopDivision":2,"released":false,"masterDeck":"A","masterTrack":{"contentId":"abc","title":"Track","artist":"Artist","isPlaying":true}}}"#;
        let sync_envelope = super::DjLinkEnvelope::parse_json(sync).unwrap();
        assert_eq!(
            sync_envelope.message_type,
            super::DjLinkMessageType::StateSync
        );
        let changed = r#"{"v":1,"type":"DJ_MASTER_CHANGED","agentId":"rekordbox","sessionId":"s1","sequence":3,"eventId":"master-1","payload":{"masterDeck":"B","master":true}}"#;
        assert_eq!(
            super::DjLinkEnvelope::parse_json(changed)
                .unwrap()
                .message_type,
            super::DjLinkMessageType::MasterChanged
        );
        assert!(
            super::DjLinkEnvelope::parse_json(&hello.replace("DJ_AGENT_HELLO", "HELLO")).is_err()
        );
    }

    #[test]
    fn dj_link_flat_generic_json_cross_fixture_is_strict_and_canonical() {
        let hello = r#"{"type":"DJ_AGENT_HELLO","eventId":"hello-flat","sequence":1,"protocol":"generic-json","token":"0123456789abcdef0123456789abcdef","capabilities":["DJ_STATE_SYNC","DJ_TIMELINE_STATE_REQUEST"]}"#;
        let frame = super::DjLinkFlatFrame::parse_json(hello).unwrap();
        assert!(frame.is_hello());
        let envelope = frame
            .to_envelope("generic-json", "socket-hello-flat")
            .unwrap();
        assert_eq!(envelope.message_type, super::DjLinkMessageType::Hello);
        assert_eq!(envelope.agent_id, "generic-json");
        assert!(
            super::DjLinkFlatFrame::parse_json(&hello.replace("generic-json", "unknown")).is_err()
        );
        assert!(super::DjLinkFlatFrame::parse_json(&hello.replace(
            "\"token\":\"0123456789abcdef0123456789abcdef\"",
            "\"token\":\"short\""
        ))
        .is_err());

        let sync = r#"{"type":"DJ_STATE_SYNC","eventId":"sync-flat","sequence":2,"loopDivision":2,"released":false,"masterDeck":"A","masterTrack":{"contentId":"abc","title":"Track","artist":"Artist","trackBpm":128.0,"isPlaying":true}}"#;
        let sync_frame = super::DjLinkFlatFrame::parse_json(sync).unwrap();
        let sync_envelope = sync_frame
            .to_envelope("generic-json", "socket-hello-flat")
            .unwrap();
        assert_eq!(
            sync_envelope.message_type,
            super::DjLinkMessageType::StateSync
        );
        assert_eq!(
            sync_envelope
                .payload
                .get("masterTrack")
                .and_then(|value| value.get("trackBpm"))
                .and_then(serde_json::Value::as_f64),
            Some(128.0)
        );

        let request =
            r#"{"type":"DJ_TIMELINE_STATE_REQUEST","eventId":"request-flat","sequence":3}"#;
        let request_envelope = super::DjLinkFlatFrame::parse_json(request)
            .unwrap()
            .to_envelope("generic-json", "socket-hello-flat")
            .unwrap();
        assert_eq!(
            request_envelope.message_type,
            super::DjLinkMessageType::TimelineStateRequest
        );
        let state = super::DjLinkTimelineState {
            message_type: "DJ_TIMELINE_STATE".to_string(),
            event_id: "request-flat".to_string(),
            sequence: 3,
            state: super::DjLinkTimelineStateValue::Running,
            loop_active: false,
            timeline_id: "show-1".to_string(),
            position_bars: 16,
        };
        state.validate().unwrap();
        let wire = serde_json::to_value(&state).unwrap();
        assert_eq!(wire["type"], "DJ_TIMELINE_STATE");
        assert_eq!(wire["timelineId"], "show-1");
        assert_eq!(wire["positionBars"], 16);
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
        };
        assert_eq!(content.canonical_key().unwrap(), "content:track-1");
        let title_artist = super::DjTrackSelector {
            content_id: None,
            title: Some(" Track ".to_string()),
            artist: Some(" Artist ".to_string()),
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
        }
        .canonical_key()
        .is_err());
    }
}

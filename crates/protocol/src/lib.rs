use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};

pub type FixtureId = u64;
pub type EffectId = u64;
pub type CueId = u64;
pub type CueListId = u64;
pub type PaletteId = u64;
pub type ExecutorId = u64;
pub type TimelineEventId = u64;
pub type TimelineAudioClipId = u64;
pub type AutomationId = u64;
pub type VideoLayerId = u64;
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoLayerSummary {
    pub id: VideoLayerId,
    pub label: String,
    pub source: VideoSourceSummary,
    pub blend_mode: VideoBlendMode,
    pub state: VideoLayerState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isf_effect: Option<VideoIsfEffectSummary>,
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
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            master_opacity: 1.0,
            blackout: false,
            auto_vj: AutoVjSnapshot::default(),
        }
    }
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

fn is_zero_f32(value: &f32) -> bool {
    *value == 0.0
}

fn is_false(value: &bool) -> bool {
    !*value
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
            label: "Main".to_string(),
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
            label: "Main".to_string(),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineAudioClipSummary {
    #[serde(default)]
    pub id: TimelineAudioClipId,
    #[serde(default)]
    pub layer_id: u32,
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
    pub layer_id: VideoLayerId,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineSnapshot {
    #[serde(default)]
    pub layers: Vec<TimelineLayerSummary>,
    pub events: Vec<TimelineCueEventSummary>,
    pub automations: Vec<TimelineAutomationSummary>,
    pub video_automations: Vec<TimelineVideoAutomationSummary>,
    #[serde(default)]
    pub audio: Option<AudioAnalysisSummary>,
    #[serde(default)]
    pub audio_clips: Vec<TimelineAudioClipSummary>,
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
    pub playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
}

impl Default for TimelineSnapshot {
    fn default() -> Self {
        Self {
            layers: Vec::new(),
            events: Vec::new(),
            automations: Vec::new(),
            video_automations: Vec::new(),
            audio: None,
            audio_clips: Vec::new(),
            audio_offset_ms: 0,
            audio_muted: false,
            metronome_enabled: false,
            count_in_beats: default_timeline_count_in_beats(),
            count_in_remaining_ms: 0,
            audio_transport_revision: 0,
            playing: false,
            position_ms: 0,
            duration_ms: 0,
        }
    }
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
    Triangle,
    Saw,
    Square,
    Strobe,
    Random,
    Perlin,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ColorEffectSpatialRecipe {
    KnightRider {
        /// Lit window width in ordered beam cells (1..=100).
        size: u16,
        one_way: bool,
        fading: bool,
        go_outside: bool,
        gradient: f32,
    },
    Burst {
        color_width: f32,
        gradient: f32,
    },
    RandomFill {
        point_width: u16,
    },
    Sparkle {
        number: u16,
        lifespan: f32,
        width: u16,
    },
    Plasma {
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
        color_width: f32,
        angle_degrees: f32,
        gradient: f32,
    },
    Rainbow {
        vertical_symmetry: bool,
        rotation_degrees: f32,
        color_width: f32,
        angle_degrees: f32,
        gradient: f32,
    },
    Perlin {
        octaves: u8,
        zoom: f32,
        direction_degrees: f32,
        speed: f32,
        amplitude: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorEffectSpatialPattern {
    pub recipe: ColorEffectSpatialRecipe,
    /// Empty for native effects; the engine derives one beam from each target fixture.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beam_targets: Vec<ColorEffectBeamTarget>,
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
    pub random_seed: u64,
    pub blend_mode: EffectBlendMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct MovePathPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoveInterpolation {
    Line,
    Smooth,
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
    /// Mirrors Pan for the second half of the resolved fixture order. The
    /// first half (and an odd centre fixture) retain the authored path.
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
            merge_mode: DmxMergeMode::Htp,
            timeout_ms: 2_500,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LearnedOscControl {
    pub address: String,
    pub value: Option<f32>,
    pub argument_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineTelemetry {
    pub frame_counter: u64,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteControlStatus {
    pub running: bool,
    pub active_connections: usize,
    pub rejected_connections: u64,
    pub clients: Vec<RemoteClientSummary>,
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
    pub video: VideoSnapshot,
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
            video: VideoSnapshot::default(),
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
        video_output_mapping_field_value, AudioSpectrumBand, AudioSpectrumSource,
        NodeGraphAudioNode, VideoOutputMapping,
    };

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
        assert_eq!(snapshot.audio_offset_ms, 0);
        assert!(!snapshot.audio_muted);
        assert!(!snapshot.metronome_enabled);
        assert_eq!(snapshot.count_in_beats, 4);
        assert_eq!(snapshot.count_in_remaining_ms, 0);
        assert_eq!(snapshot.audio_transport_revision, 0);
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

        let mut legacy_owned_value = serde_json::to_value(&target).unwrap();
        legacy_owned_value["params"]["Lfo"]
            .as_object_mut()
            .unwrap()
            .remove("fixture_spread");
        legacy_owned_value["params"]["Lfo"]
            .as_object_mut()
            .unwrap()
            .remove("beam_targets");
        let legacy_owned: super::CueEffectTarget =
            serde_json::from_value(legacy_owned_value).unwrap();
        let Some(super::EffectParamsSnapshot::Lfo(legacy_request)) = legacy_owned.params else {
            panic!("legacy owned LFO must remain an LFO");
        };
        assert_eq!(legacy_request.fixture_spread, 0.0);
        assert!(legacy_request.beam_targets.is_empty());
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
                octaves: 5,
                zoom: 20.0,
                direction_degrees: 1.0,
                speed: 1.0,
                amplitude: 100.0,
            },
            beam_targets: vec![super::ColorEffectBeamTarget {
                fixture_id: 1,
                beam_index: 7,
                selection_index: 12,
                feature_attribute: Some("Dimmer".to_string()),
            }],
        }));
        let json = serde_json::to_string(&spatial).unwrap();
        let roundtrip: super::ColorEffectRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, spatial);
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
    }

    #[test]
    fn move_effect_request_and_preset_roundtrip() {
        let request = super::MoveEffectRequest {
            label: "Front circle".to_string(),
            fixture_ids: vec![1, 2],
            target_group_ids: vec!["Moving".to_string()],
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
    fn child_timeline_defaults_every_collection_and_legacy_cue_loads_none() {
        let child: super::ChildTimelineSummary =
            serde_json::from_str(r#"{"duration_ms":1200}"#).unwrap();
        assert!(child.layers.is_empty());
        assert!(child.events.is_empty());
        assert!(child.automations.is_empty());
        assert!(child.video_automations.is_empty());
        assert!(child.audio.is_none());
        assert!(child.audio_clips.is_empty());
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
}

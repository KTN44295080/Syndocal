use serde::{Deserialize, Serialize};

pub type FixtureId = u64;
pub type EffectId = u64;
pub type CueId = u64;
pub type TimelineEventId = u64;
pub type AutomationId = u64;
pub type VideoLayerId = u64;
pub type CompositionId = u64;
pub type VideoOutputId = u64;
pub type NodeGraphId = u64;
pub type StageObjectId = u64;

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
pub struct AudioAnalysisSummary {
    pub path: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_ms: u64,
    pub estimated_bpm: Option<f32>,
    pub waveform: Vec<AudioWaveformPoint>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectFile {
    pub version: u32,
    pub app: String,
    #[serde(default)]
    pub custom_profiles: Vec<FixtureProfileSummary>,
    pub snapshot: EngineSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CueFixtureTarget {
    pub fixture_id: FixtureId,
    pub values: Vec<AttributeValueSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoSourceKind {
    File,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoLayerTarget {
    pub layer_id: VideoLayerId,
    pub state: VideoLayerState,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoSnapshot {
    pub layers: Vec<VideoLayerSummary>,
    pub compositions: Vec<CompositionSummary>,
    pub outputs: Vec<VideoOutputSummary>,
    #[serde(default)]
    pub mapping_presets: Vec<VideoOutputMappingPresetSummary>,
    pub master_opacity: f32,
    pub blackout: bool,
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CueSummary {
    pub id: CueId,
    pub label: String,
    pub fade_ms: u64,
    pub targets: Vec<CueFixtureTarget>,
    pub video_targets: Vec<VideoLayerTarget>,
    #[serde(default)]
    pub video_output_targets: Vec<VideoOutputTarget>,
    #[serde(default)]
    pub node_graph_targets: Vec<CueNodeGraphTarget>,
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
pub enum TimelineTrackKind {
    Lighting,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineCueEventSummary {
    pub id: TimelineEventId,
    pub cue_id: CueId,
    pub time_ms: u64,
    pub track: TimelineTrackKind,
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
pub struct TimelineSnapshot {
    pub events: Vec<TimelineCueEventSummary>,
    pub automations: Vec<TimelineAutomationSummary>,
    pub video_automations: Vec<TimelineVideoAutomationSummary>,
    #[serde(default)]
    pub audio: Option<AudioAnalysisSummary>,
    pub playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
}

impl Default for TimelineSnapshot {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            automations: Vec::new(),
            video_automations: Vec::new(),
            audio: None,
            playing: false,
            position_ms: 0,
            duration_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LfoShape {
    Sine,
    Cosine,
    Triangle,
    Saw,
    Square,
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
    Transform,
    Output,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphSummary {
    pub id: NodeGraphId,
    pub label: String,
    pub enabled: bool,
    pub nodes: Vec<NodeGraphNodeSummary>,
    pub edges: Vec<NodeGraphEdgeSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeGraphPresetFile {
    pub version: u32,
    pub app: String,
    pub graph: NodeGraphSummary,
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
pub enum EffectKind {
    Lfo,
    PositionWave,
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
    pub blend_mode: EffectBlendMode,
    pub origin: Option<Vec3>,
    pub direction: Option<Vec3>,
    pub speed: Option<f32>,
    pub wavelength: Option<f32>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectPreset {
    pub version: u32,
    pub effect_type: EffectKind,
    pub enabled: bool,
    pub lfo: Option<LfoEffectRequest>,
    pub position_wave: Option<PositionWaveEffectRequest>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerialPortSummary {
    pub name: String,
    pub port_type: String,
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
}

impl Default for RemoteControlConfig {
    fn default() -> Self {
        Self {
            bind_ip: "0.0.0.0".to_string(),
            port: 9_100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClockSnapshot {
    pub bpm: f32,
    pub beat_phase: f32,
    pub beat_counter: u64,
    pub tap_count: usize,
    pub source: ClockSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubmasterSummary {
    pub group_id: String,
    pub label: String,
    pub level: f32,
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineSnapshot {
    pub fixtures: Vec<PatchedFixtureSummary>,
    pub cues: Vec<CueSummary>,
    pub active_cue_id: Option<CueId>,
    pub active_fade: Option<ActiveFadeSummary>,
    pub timeline: TimelineSnapshot,
    pub video: VideoSnapshot,
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
            active_cue_id: None,
            active_fade: None,
            timeline: TimelineSnapshot::default(),
            video: VideoSnapshot::default(),
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
            dmx_preview: vec![0; 512],
            dmx_previews: vec![DmxUniversePreview {
                universe: 0,
                values: vec![0; 512],
            }],
            telemetry: EngineTelemetry::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_video_output_mapping_field, set_video_output_mapping_field_value,
        video_output_mapping_field_value, VideoOutputMapping,
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
        assert!(set_video_output_mapping_field_value(&mut mapping, "not a field", 1.0).is_err());
    }
}

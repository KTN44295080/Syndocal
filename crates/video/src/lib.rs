use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{Duration, SystemTime},
};

use protocol::{
    validate_video_layer_transition_runtime, ClockSnapshot, CompositionId, CompositionSummary,
    MachineOutputRole, TimelineId, Transform2D, VideoBackendState, VideoBackendStatus,
    VideoBlendMode, VideoClipRuntimeSnapshot, VideoClipSlotId, VideoColorAdjust,
    VideoCuePointSummary, VideoEffectChainId, VideoEffectChainSummary, VideoEffectId,
    VideoEffectKind, VideoEffectScope, VideoEffectStageId, VideoFxAdjust, VideoIsfEffectSummary,
    VideoLayerId, VideoLayerState, VideoLayerSummary, VideoLayerTransitionCurve,
    VideoLayerTransitionRuntimeSnapshot, VideoLayerTransitionTarget, VideoMediaMetadata,
    VideoOutputAspectMode, VideoOutputId, VideoOutputKind, VideoOutputMapping, VideoOutputSummary,
    VideoRuntimeStatus, VideoSnapshot, VideoSourceKind, VideoSourceSummary,
    VideoTransitionEffectOwner, VIDEO_EFFECT_CHAIN_MAX_STAGES,
    VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION, VIDEO_OUTPUT_BITMAP_MASK_WORD_CAPACITY,
};
use serde::{Deserialize, Serialize};

mod builtin_isf;
mod gpu_compositor;
mod gpu_surface;
mod hap_decoder;
mod isf_runtime;
mod libav_decoder;

pub use builtin_isf::{builtin_isf_effect, BuiltinIsfPreset, BUILTIN_ISF_PRESETS};
pub use gpu_compositor::{GpuCompositeError, GpuCompositor};
pub use gpu_surface::{GpuSurfaceBufferStats, GpuSurfaceError, GpuSurfacePresenter};
pub use hap_decoder::{HapMovFrameDecoder, PreferredVideoFrameDecoder, VideoDecoderDiagnostics};
pub use isf_runtime::{
    isf_effect_from_prepared, prepare_isf_shader, resolved_isf_control_values,
    resolved_isf_control_values_from_controls, IsfControlDefinition, IsfControlKind, IsfGpuRuntime,
    IsfPrepareError, IsfRuntimeError, PreparedIsfShader, ISF_MAX_CONTROL_INPUTS,
    ISF_MAX_SOURCE_BYTES,
};
pub use libav_decoder::LibavFrameDecoder;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoPixelFormat {
    Rgba8,
    Bgra8,
    Dxt1,
    Dxt5,
    YcoCgDxt5,
    Bc7,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoFrame {
    pub layer_id: VideoLayerId,
    pub width: u32,
    pub height: u32,
    pub pts_ms: u64,
    pub duration_ms: u64,
    pub format: VideoPixelFormat,
    pub data: Vec<u8>,
}

/// One runtime decode/render request. The key, rather than its diagnostic
/// `layer_id`, owns queues, decoder sessions, and caches.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoRenderInput {
    pub key: protocol::VideoRenderInputKey,
    pub request: VideoFrameRequest,
    pub isf_effect: Option<VideoIsfEffectSummary>,
    pub clip_slot_id: Option<VideoClipSlotId>,
}

impl VideoRenderInput {
    pub fn new(key: protocol::VideoRenderInputKey, request: VideoFrameRequest) -> Self {
        Self {
            key,
            request,
            isf_effect: None,
            clip_slot_id: None,
        }
    }

    /// Bridges the layer-only renderer APIs to the pre-C0 input identity.
    pub fn legacy(request: VideoFrameRequest) -> Self {
        Self::new(protocol::VideoRenderInputKey::LEGACY, request)
    }

    pub fn layer_id(&self) -> VideoLayerId {
        self.request.layer_id
    }

    pub fn with_isf_effect(mut self, effect: Option<VideoIsfEffectSummary>) -> Self {
        self.isf_effect = effect;
        self
    }

    pub fn with_clip_slot_id(mut self, clip_slot_id: Option<VideoClipSlotId>) -> Self {
        self.clip_slot_id = clip_slot_id;
        self
    }
}

/// Runtime inputs are owned solely by their key. The one compatibility
/// exception is the pre-C0 `LEGACY` key: its layer-only API historically kept
/// a bounded cache per layer, so retain that behavior without applying it to
/// any explicit multi-input key.
pub(crate) fn render_input_cache_owner_matches(
    key: protocol::VideoRenderInputKey,
    layer_id: VideoLayerId,
    candidate_key: protocol::VideoRenderInputKey,
    candidate_layer_id: VideoLayerId,
) -> bool {
    key == candidate_key
        && (key != protocol::VideoRenderInputKey::LEGACY || layer_id == candidate_layer_id)
}

/// A decoded frame associated with its runtime owner. `VideoFrame::layer_id`
/// stays as diagnostics only; input identity selects the frame during a mix.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoRenderInputFrame {
    pub key: protocol::VideoRenderInputKey,
    pub frame: VideoFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoIsfStageError {
    pub layer_id: VideoLayerId,
    pub stage_index: Option<usize>,
    pub stage_label: Option<String>,
    pub message: String,
}

/// Stable C1 identity for a renderer-local effect fault. Authored IDs remain
/// separate from `VideoRenderInputKey`; legacy-only snapshots deliberately use
/// `None` because they do not contain stable C1 entities to report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoEffectStageFault {
    pub scope: VideoEffectScope,
    pub chain_id: Option<VideoEffectChainId>,
    pub stage_id: Option<VideoEffectStageId>,
    pub effect_id: Option<VideoEffectId>,
    pub render_input_key: Option<protocol::VideoRenderInputKey>,
    pub stage_index: Option<usize>,
    pub stage_label: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedVideoEffectStage {
    pub stage_id: Option<VideoEffectStageId>,
    pub effect_id: Option<VideoEffectId>,
    pub enabled: bool,
    pub label: String,
    pub effect: VideoIsfEffectSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedVideoEffectChain {
    pub chain_id: Option<VideoEffectChainId>,
    pub scope: VideoEffectScope,
    pub bypassed: bool,
    pub stages: Vec<ResolvedVideoEffectStage>,
}

#[derive(Debug, Clone, Copy)]
pub struct VideoEffectRenderContext<'a> {
    pub clip_runtime: &'a VideoClipRuntimeSnapshot,
    pub project_render_epoch: u64,
}

#[derive(Debug, Clone)]
struct CachedIsfShader {
    value: Result<Arc<PreparedIsfShader>, IsfPrepareError>,
    last_used: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VideoOutputLastValidKey {
    project_render_epoch: u64,
    output_id: VideoOutputId,
    width: u32,
    height: u32,
    composition_id: CompositionId,
    kind: VideoOutputKind,
    fullscreen: bool,
    monitor_id: Option<u32>,
    endpoint_name: Option<String>,
}

#[derive(Debug, Clone)]
struct VideoOutputLastValidFrame {
    key: VideoOutputLastValidKey,
    frame: VideoFrame,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoRenderInputIsfStageError {
    pub key: protocol::VideoRenderInputKey,
    pub error: VideoIsfStageError,
}

pub struct FrameQueue {
    key: protocol::VideoRenderInputKey,
    layer_id: VideoLayerId,
    slots: Vec<Option<VideoFrame>>,
    start: usize,
    len: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrameQueuePush {
    Inserted,
    DroppedOldest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositionPlan {
    pub composition_id: CompositionId,
    pub label: String,
    pub output_ids: Vec<VideoOutputId>,
    pub master_opacity: f32,
    pub blackout: bool,
    pub layers: Vec<CompositionLayerPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompositionLayerPlan {
    pub layer_id: VideoLayerId,
    pub label: String,
    pub source: VideoSourceSummary,
    pub blend_mode: VideoBlendMode,
    pub opacity: f32,
    pub position_ms: u64,
    pub transform: Transform2D,
    pub color: VideoColorAdjust,
    pub fx: VideoFxAdjust,
}

/// Explicit input binding for an additive multi-input composition seam.
/// Multiple entries may intentionally have the same `layer_id`; their runtime
/// keys keep their decode and presentation histories distinct.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionRenderInput {
    pub key: protocol::VideoRenderInputKey,
    pub layer: CompositionLayerPlan,
}

/// Runtime-only composition input list. It deliberately does not alter the
/// persisted `CompositionPlan`/`VideoSnapshot` contract.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionInputMix {
    pub inputs: Vec<CompositionRenderInput>,
}

fn validate_unique_render_input_keys<'a>(
    keys: impl IntoIterator<Item = &'a protocol::VideoRenderInputKey>,
) -> Result<(), CpuCompositeError> {
    let mut seen = Vec::new();
    for key in keys {
        if seen.contains(key) {
            return Err(CpuCompositeError::DuplicateRenderInputKey { key: *key });
        }
        seen.push(*key);
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoOutputRenderPlan {
    pub output_id: VideoOutputId,
    pub label: String,
    pub kind: VideoOutputKind,
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub monitor_id: Option<u32>,
    pub endpoint_name: Option<String>,
    pub output_opacity: f32,
    pub output_blackout: bool,
    pub mapping: VideoOutputMapping,
    pub composition: CompositionPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreparedVideoOutput {
    pub plan: VideoOutputRenderPlan,
    pub frames: Vec<VideoFrame>,
}

/// Runtime evidence for one output render. This is intentionally separate
/// from `VideoFrame`: a last-valid frame is safe for presentation but is not
/// proof that a newly admitted Follow transition rendered successfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoOutputRenderFreshness {
    Fresh,
    LastValid,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoOutputRenderEvidence {
    pub project_render_epoch: u64,
    pub output_id: VideoOutputId,
    pub freshness: VideoOutputRenderFreshness,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoOutputRenderResult {
    pub frame: VideoFrame,
    pub evidence: VideoOutputRenderEvidence,
}

/// The resolved Follow/output transition parameters for a single composed
/// program frame. The curve is applied exactly once before the transition
/// kind is dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoFollowOutputTransitionEvidence {
    pub kind: protocol::VideoClipTakeKind,
    pub curve: VideoLayerTransitionCurve,
    pub raw_progress_millis: u16,
    pub curved_progress_millis: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoFollowOutputTransitionResult {
    pub frame: VideoFrame,
    pub evidence: VideoFollowOutputTransitionEvidence,
}

/// Render evidence for the Follow/output seam after its optional Transition
/// effect chain has run. `LastValid` is only returned from an explicitly
/// supplied fallback; a chain fault is always retained in `stage_faults` and
/// therefore is never safe to acknowledge as a fresh transition settle.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoFollowOutputTransitionRenderEvidence {
    pub transition: VideoFollowOutputTransitionEvidence,
    pub freshness: VideoOutputRenderFreshness,
    pub error: Option<String>,
    pub stage_faults: Vec<VideoEffectStageFault>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoFollowOutputTransitionRenderResult {
    pub frame: VideoFrame,
    pub evidence: VideoFollowOutputTransitionRenderEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalVideoInputPlan {
    pub layer_id: VideoLayerId,
    pub label: String,
    pub kind: VideoSourceKind,
    pub backend_id: String,
    pub endpoint_name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalVideoOutputPlan {
    pub output_id: VideoOutputId,
    pub label: String,
    pub kind: VideoOutputKind,
    pub backend_id: String,
    pub endpoint_name: String,
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
    pub opacity: f32,
    pub blackout: bool,
    pub composition_id: CompositionId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalVideoIoRoutePlans {
    pub inputs: Vec<ExternalVideoInputRoutePlan>,
    pub outputs: Vec<ExternalVideoOutputRoutePlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalVideoInputRoutePlan {
    pub layer_id: VideoLayerId,
    pub label: String,
    pub kind: VideoSourceKind,
    pub backend_id: String,
    pub backend_label: Option<String>,
    pub backend_state: Option<VideoBackendState>,
    pub backend_detail: Option<String>,
    pub endpoint_name: String,
    pub enabled: bool,
    pub ready: bool,
    pub live: bool,
    pub issue: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalVideoOutputRoutePlan {
    pub output_id: VideoOutputId,
    pub label: String,
    pub kind: VideoOutputKind,
    pub backend_id: String,
    pub backend_label: Option<String>,
    pub backend_state: Option<VideoBackendState>,
    pub backend_detail: Option<String>,
    pub endpoint_name: String,
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
    pub opacity: f32,
    pub blackout: bool,
    pub composition_id: CompositionId,
    pub ready: bool,
    pub live: bool,
    pub issue: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExternalVideoBackendRouteState {
    label: Option<String>,
    state: Option<VideoBackendState>,
    detail: Option<String>,
    ready: bool,
    issue: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExternalVideoTransportDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalVideoTransportRoute {
    pub direction: ExternalVideoTransportDirection,
    pub route_id: u64,
    pub label: String,
    pub backend_id: String,
    pub endpoint_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalVideoTransportBlockedRoute {
    pub route: ExternalVideoTransportRoute,
    pub issue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalVideoTransportFailedRoute {
    pub route: ExternalVideoTransportRoute,
    pub issue: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalVideoTransportSyncReport {
    pub started: Vec<ExternalVideoTransportRoute>,
    pub kept: Vec<ExternalVideoTransportRoute>,
    pub stopped: Vec<ExternalVideoTransportRoute>,
    pub blocked: Vec<ExternalVideoTransportBlockedRoute>,
    pub start_failed: Vec<ExternalVideoTransportFailedRoute>,
    pub stop_failed: Vec<ExternalVideoTransportFailedRoute>,
    pub idle: Vec<ExternalVideoTransportRoute>,
    pub active_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalVideoTransportStatus {
    pub active_routes: Vec<ExternalVideoTransportRoute>,
    pub active_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ExternalVideoTransportRuntime {
    active_routes: Vec<ExternalVideoTransportRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalVideoTransportDriverError {
    pub message: String,
}

pub trait ExternalVideoTransportDriver {
    fn start_route(
        &mut self,
        route: &ExternalVideoTransportRoute,
    ) -> Result<(), ExternalVideoTransportDriverError>;

    fn stop_route(
        &mut self,
        route: &ExternalVideoTransportRoute,
    ) -> Result<(), ExternalVideoTransportDriverError>;
}

#[derive(Debug, Clone, Default)]
pub struct NoopExternalVideoTransportDriver;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuCompositeError {
    InvalidOutputSize,
    DuplicateRenderInputKey {
        key: protocol::VideoRenderInputKey,
    },
    UnsupportedFrameFormat {
        layer_id: VideoLayerId,
        format: VideoPixelFormat,
    },
    FrameSizeMismatch {
        layer_id: VideoLayerId,
    },
    InvalidTransitionProgress {
        progress_millis: u16,
    },
    InvalidTransitionEffectScope {
        scope: VideoEffectScope,
    },
    InvalidFollowTransitionEffectOwner {
        expected_source_timeline_id: TimelineId,
        owner: VideoTransitionEffectOwner,
    },
    MissingFrame {
        layer_id: VideoLayerId,
    },
    UnsupportedRenderInputFrameFormat {
        key: protocol::VideoRenderInputKey,
        layer_id: VideoLayerId,
        format: VideoPixelFormat,
    },
    RenderInputFrameSizeMismatch {
        key: protocol::VideoRenderInputKey,
        layer_id: VideoLayerId,
    },
    MissingRenderInputFrame {
        key: protocol::VideoRenderInputKey,
        layer_id: VideoLayerId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StillImageError {
    Decode(String),
    EmptyImage,
    UnsupportedFormat(VideoPixelFormat),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoRuntimeConfig {
    pub frame_queue_capacity: usize,
    pub preview_width: u32,
    pub preview_height: u32,
}

impl Default for VideoRuntimeConfig {
    fn default() -> Self {
        Self {
            frame_queue_capacity: 8,
            preview_width: 320,
            preview_height: 180,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoRuntimeError {
    MissingComposition,
    Composite(CpuCompositeError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoPreviewError {
    InvalidSize,
    InvalidLayerTransition(String),
    MissingLayer {
        layer_id: VideoLayerId,
    },
    MissingTransitionSlot {
        layer_id: VideoLayerId,
        slot_id: VideoClipSlotId,
    },
    MissingTransitionAsset {
        layer_id: VideoLayerId,
        asset_id: protocol::MediaAssetId,
    },
    Output(VideoOutputRenderError),
    MissingStillImagePath {
        layer_id: VideoLayerId,
        label: String,
    },
    StillImage {
        layer_id: VideoLayerId,
        label: String,
        error: StillImageError,
    },
    Decode {
        layer_id: VideoLayerId,
        label: String,
        error: VideoDecodeError,
    },
    RenderInput {
        key: protocol::VideoRenderInputKey,
        error: VideoFrameProviderError,
    },
    Runtime(VideoRuntimeError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoOutputRenderError {
    MissingOutput {
        output_id: VideoOutputId,
    },
    MissingComposition {
        output_id: VideoOutputId,
        composition_id: CompositionId,
    },
    InvalidOutputSize {
        output_id: VideoOutputId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoFrameProviderError {
    MissingStillImagePath {
        layer_id: VideoLayerId,
        label: String,
    },
    StillImage {
        layer_id: VideoLayerId,
        label: String,
        error: StillImageError,
    },
    Decode {
        layer_id: VideoLayerId,
        label: String,
        error: VideoDecodeError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoDecodeError {
    MissingSourcePath {
        layer_id: VideoLayerId,
        label: String,
    },
    UnsupportedSource {
        layer_id: VideoLayerId,
        label: String,
        kind: VideoSourceKind,
    },
    Decode {
        layer_id: VideoLayerId,
        label: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoProbeError {
    Probe { path: PathBuf, message: String },
    InvalidJson(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoProbeSummary {
    pub codec: Option<String>,
    pub metadata: Option<VideoMediaMetadata>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoFrameRequest {
    pub layer_id: VideoLayerId,
    pub label: String,
    pub source: VideoSourceSummary,
    pub position_ms: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoDecodePriority {
    Current,
    Lookahead,
    Background,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledVideoDecode {
    pub input_key: protocol::VideoRenderInputKey,
    pub request: VideoFrameRequest,
    pub priority: VideoDecodePriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoDecodeSchedulePush {
    Inserted,
    Duplicate,
    Reprioritized,
    DroppedLowerPriority,
    RejectedFull,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoDecodeEnqueueReport {
    pub layers_considered: usize,
    pub requests_attempted: usize,
    pub inserted: usize,
    pub duplicate: usize,
    pub reprioritized: usize,
    pub dropped_lower_priority: usize,
    pub rejected_full: usize,
    pub pending: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoDecodeWarmupReport {
    pub enqueue: VideoDecodeEnqueueReport,
    pub decode: VideoDecodeWorkerReport,
}

#[derive(Debug, Clone)]
pub struct VideoDecodeScheduler {
    capacity: usize,
    sequence: u64,
    jobs: Vec<ScheduledVideoDecodeJob>,
}

pub struct VideoDecodeWorker<D = NullVideoDecoder> {
    scheduler: VideoDecodeScheduler,
    decoder: D,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoDecodeWorkerReport {
    pub attempted: usize,
    pub decoded: usize,
    pub skipped: usize,
    pub pending: usize,
    pub errors: Vec<VideoDecodeWorkerError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoDecodeWorkerStep {
    Decoded(FrameQueuePush),
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoDecodeWorkerError {
    pub input_key: protocol::VideoRenderInputKey,
    pub layer_id: VideoLayerId,
    pub label: String,
    pub position_ms: u64,
    pub priority: VideoDecodePriority,
    pub error: VideoDecodeError,
}

#[derive(Debug, Clone)]
struct ScheduledVideoDecodeJob {
    input_key: protocol::VideoRenderInputKey,
    request: VideoFrameRequest,
    priority: VideoDecodePriority,
    sequence: u64,
}

impl From<CpuCompositeError> for VideoRuntimeError {
    fn from(error: CpuCompositeError) -> Self {
        Self::Composite(error)
    }
}

impl From<VideoFrameProviderError> for VideoPreviewError {
    fn from(error: VideoFrameProviderError) -> Self {
        match error {
            VideoFrameProviderError::MissingStillImagePath { layer_id, label } => {
                Self::MissingStillImagePath { layer_id, label }
            }
            VideoFrameProviderError::StillImage {
                layer_id,
                label,
                error,
            } => Self::StillImage {
                layer_id,
                label,
                error,
            },
            VideoFrameProviderError::Decode {
                layer_id,
                label,
                error,
            } => Self::Decode {
                layer_id,
                label,
                error,
            },
        }
    }
}

impl VideoDecodePriority {
    fn rank(self) -> u8 {
        match self {
            Self::Current => 0,
            Self::Lookahead => 1,
            Self::Background => 2,
        }
    }
}

impl VideoDecodeScheduler {
    pub fn new(capacity: usize) -> Self {
        assert!(
            capacity > 0,
            "decode scheduler capacity must be greater than zero"
        );
        Self {
            capacity,
            sequence: 0,
            jobs: Vec::new(),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    pub fn clear(&mut self) {
        self.jobs.clear();
    }

    pub fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.jobs.retain(|job| {
            layer_ids
                .iter()
                .any(|layer_id| *layer_id == job.request.layer_id)
        });
    }

    pub fn push(
        &mut self,
        request: VideoFrameRequest,
        priority: VideoDecodePriority,
    ) -> VideoDecodeSchedulePush {
        self.push_input(VideoRenderInput::legacy(request), priority)
    }

    pub fn push_input(
        &mut self,
        input: VideoRenderInput,
        priority: VideoDecodePriority,
    ) -> VideoDecodeSchedulePush {
        if let Some(index) = self.jobs.iter().position(|job| {
            job.input_key == input.key
                && video_decode_request_key_matches(&job.request, &input.request)
        }) {
            if priority.rank() < self.jobs[index].priority.rank() {
                let sequence = self.next_sequence();
                self.jobs[index].request = input.request;
                self.jobs[index].priority = priority;
                self.jobs[index].sequence = sequence;
                return VideoDecodeSchedulePush::Reprioritized;
            }
            return VideoDecodeSchedulePush::Duplicate;
        }

        if self.jobs.len() < self.capacity {
            self.insert(input, priority);
            return VideoDecodeSchedulePush::Inserted;
        }

        let Some(worst_index) = self.worst_job_index() else {
            return VideoDecodeSchedulePush::RejectedFull;
        };
        if priority.rank() < self.jobs[worst_index].priority.rank() {
            self.jobs.remove(worst_index);
            self.insert(input, priority);
            return VideoDecodeSchedulePush::DroppedLowerPriority;
        }
        VideoDecodeSchedulePush::RejectedFull
    }

    pub fn push_layer_preview(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
        prefetch_count: usize,
        prefetch_interval_ms: u64,
        bpm: Option<f32>,
    ) -> Vec<VideoDecodeSchedulePush> {
        preview_decode_requests_for_layer(
            layer,
            width,
            height,
            prefetch_count,
            prefetch_interval_ms,
            bpm,
        )
        .into_iter()
        .enumerate()
        .map(|(index, request)| {
            self.push(
                request,
                if index == 0 {
                    VideoDecodePriority::Current
                } else {
                    VideoDecodePriority::Lookahead
                },
            )
        })
        .collect()
    }

    pub fn push_composition_preview(
        &mut self,
        snapshot: &VideoSnapshot,
        composition_id: Option<CompositionId>,
        width: u32,
        height: u32,
        prefetch_count: usize,
        prefetch_interval_ms: u64,
        bpm: Option<f32>,
    ) -> Result<VideoDecodeEnqueueReport, VideoRuntimeError> {
        let plan = match composition_id {
            Some(composition_id) => build_composition_plans(snapshot)
                .into_iter()
                .find(|plan| plan.composition_id == composition_id)
                .ok_or(VideoRuntimeError::MissingComposition)?,
            None => build_composition_plans(snapshot)
                .into_iter()
                .next()
                .ok_or(VideoRuntimeError::MissingComposition)?,
        };
        Ok(self.push_composition_layer_previews(
            snapshot,
            plan.layers.iter().map(|layer| layer.layer_id),
            width,
            height,
            prefetch_count,
            prefetch_interval_ms,
            bpm,
        ))
    }

    pub fn push_output_preview(
        &mut self,
        snapshot: &VideoSnapshot,
        output_id: VideoOutputId,
        width: u32,
        height: u32,
        prefetch_count: usize,
        prefetch_interval_ms: u64,
        bpm: Option<f32>,
    ) -> Result<VideoDecodeEnqueueReport, VideoOutputRenderError> {
        let plan = build_video_output_render_plan(snapshot, output_id)?;
        Ok(self.push_composition_layer_previews(
            snapshot,
            plan.composition.layers.iter().map(|layer| layer.layer_id),
            width,
            height,
            prefetch_count,
            prefetch_interval_ms,
            bpm,
        ))
    }

    pub fn pop_next(&mut self) -> Option<ScheduledVideoDecode> {
        let index = self.best_job_index()?;
        let job = self.jobs.remove(index);
        Some(ScheduledVideoDecode {
            input_key: job.input_key,
            request: job.request,
            priority: job.priority,
        })
    }

    pub fn pending(&self) -> Vec<ScheduledVideoDecode> {
        let mut jobs = self.jobs.clone();
        jobs.sort_by_key(|job| (job.priority.rank(), job.sequence));
        jobs.into_iter()
            .map(|job| ScheduledVideoDecode {
                input_key: job.input_key,
                request: job.request,
                priority: job.priority,
            })
            .collect()
    }

    fn insert(&mut self, input: VideoRenderInput, priority: VideoDecodePriority) {
        let sequence = self.next_sequence();
        self.jobs.push(ScheduledVideoDecodeJob {
            input_key: input.key,
            request: input.request,
            priority,
            sequence,
        });
    }

    fn next_sequence(&mut self) -> u64 {
        let sequence = self.sequence;
        self.sequence = self.sequence.saturating_add(1);
        sequence
    }

    fn best_job_index(&self) -> Option<usize> {
        self.jobs
            .iter()
            .enumerate()
            .min_by_key(|(_, job)| (job.priority.rank(), job.sequence))
            .map(|(index, _)| index)
    }

    fn worst_job_index(&self) -> Option<usize> {
        let worst_rank = self.jobs.iter().map(|job| job.priority.rank()).max()?;
        self.jobs
            .iter()
            .enumerate()
            .filter(|(_, job)| job.priority.rank() == worst_rank)
            .min_by_key(|(_, job)| job.sequence)
            .map(|(index, _)| index)
    }

    fn push_composition_layer_previews(
        &mut self,
        snapshot: &VideoSnapshot,
        layer_ids: impl IntoIterator<Item = VideoLayerId>,
        width: u32,
        height: u32,
        prefetch_count: usize,
        prefetch_interval_ms: u64,
        bpm: Option<f32>,
    ) -> VideoDecodeEnqueueReport {
        let mut report = VideoDecodeEnqueueReport {
            pending: self.len(),
            ..VideoDecodeEnqueueReport::default()
        };
        let mut seen_layer_ids = Vec::new();
        for layer_id in layer_ids {
            if seen_layer_ids.contains(&layer_id) {
                continue;
            }
            seen_layer_ids.push(layer_id);
            let Some(layer) = snapshot.layers.iter().find(|layer| layer.id == layer_id) else {
                continue;
            };
            report.layers_considered += 1;
            for push in self.push_layer_preview(
                layer,
                width,
                height,
                prefetch_count,
                prefetch_interval_ms,
                bpm,
            ) {
                report.record(push);
            }
        }
        report.pending = self.len();
        report
    }
}

fn video_decode_request_key_matches(first: &VideoFrameRequest, second: &VideoFrameRequest) -> bool {
    first.layer_id == second.layer_id
        && first.source.kind == second.source.kind
        && first.source.path == second.source.path
        && first.source.name == second.source.name
        && first.position_ms == second.position_ms
        && first.width == second.width
        && first.height == second.height
}

impl VideoDecodeEnqueueReport {
    fn record(&mut self, push: VideoDecodeSchedulePush) {
        self.requests_attempted += 1;
        match push {
            VideoDecodeSchedulePush::Inserted => self.inserted += 1,
            VideoDecodeSchedulePush::Duplicate => self.duplicate += 1,
            VideoDecodeSchedulePush::Reprioritized => self.reprioritized += 1,
            VideoDecodeSchedulePush::DroppedLowerPriority => self.dropped_lower_priority += 1,
            VideoDecodeSchedulePush::RejectedFull => self.rejected_full += 1,
        }
    }
}

impl<D> VideoDecodeWorker<D> {
    pub fn new(scheduler_capacity: usize, decoder: D) -> Self {
        Self {
            scheduler: VideoDecodeScheduler::new(scheduler_capacity),
            decoder,
        }
    }

    pub fn scheduler(&self) -> &VideoDecodeScheduler {
        &self.scheduler
    }

    pub fn scheduler_mut(&mut self) -> &mut VideoDecodeScheduler {
        &mut self.scheduler
    }

    pub fn decoder(&self) -> &D {
        &self.decoder
    }

    pub fn decoder_mut(&mut self) -> &mut D {
        &mut self.decoder
    }
}

impl<D: VideoFrameDecoder> VideoDecodeWorker<D> {
    pub fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.scheduler.retain_layers(layer_ids);
        self.decoder.retain_layers(layer_ids);
    }

    pub fn retain_inputs(&mut self, inputs: &[VideoRenderInput]) {
        self.scheduler
            .jobs
            .retain(|job| inputs.iter().any(|input| input.key == job.input_key));
        self.decoder.retain_inputs(inputs);
    }

    pub fn enqueue_layer_preview(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
        prefetch_count: usize,
        prefetch_interval_ms: u64,
        bpm: Option<f32>,
    ) -> Vec<VideoDecodeSchedulePush> {
        self.scheduler.push_layer_preview(
            layer,
            width,
            height,
            prefetch_count,
            prefetch_interval_ms,
            bpm,
        )
    }

    pub fn enqueue_input(
        &mut self,
        input: VideoRenderInput,
        priority: VideoDecodePriority,
    ) -> VideoDecodeSchedulePush {
        self.scheduler.push_input(input, priority)
    }

    pub fn enqueue_composition_preview(
        &mut self,
        snapshot: &VideoSnapshot,
        composition_id: Option<CompositionId>,
        width: u32,
        height: u32,
        prefetch_count: usize,
        prefetch_interval_ms: u64,
        bpm: Option<f32>,
    ) -> Result<VideoDecodeEnqueueReport, VideoRuntimeError> {
        self.scheduler.push_composition_preview(
            snapshot,
            composition_id,
            width,
            height,
            prefetch_count,
            prefetch_interval_ms,
            bpm,
        )
    }

    pub fn enqueue_output_preview(
        &mut self,
        snapshot: &VideoSnapshot,
        output_id: VideoOutputId,
        width: u32,
        height: u32,
        prefetch_count: usize,
        prefetch_interval_ms: u64,
        bpm: Option<f32>,
    ) -> Result<VideoDecodeEnqueueReport, VideoOutputRenderError> {
        self.scheduler.push_output_preview(
            snapshot,
            output_id,
            width,
            height,
            prefetch_count,
            prefetch_interval_ms,
            bpm,
        )
    }

    pub fn decode_next_into_runtime(
        &mut self,
        runtime: &mut VideoRuntime,
    ) -> Option<Result<VideoDecodeWorkerStep, VideoDecodeWorkerError>> {
        let scheduled = self.scheduler.pop_next()?;
        let input = VideoRenderInput::new(scheduled.input_key, scheduled.request);
        let priority = scheduled.priority;
        Some(match self.decoder.decode_input_frame(&input) {
            Ok(Some(mut frame)) => {
                frame.layer_id = input.layer_id();
                Ok(VideoDecodeWorkerStep::Decoded(
                    runtime.push_input_frame(&input, frame),
                ))
            }
            Ok(None) => Ok(VideoDecodeWorkerStep::Skipped),
            Err(error) => Err(VideoDecodeWorkerError {
                input_key: input.key,
                layer_id: input.request.layer_id,
                label: input.request.label,
                position_ms: input.request.position_ms,
                priority,
                error,
            }),
        })
    }

    pub fn decode_budget_into_runtime(
        &mut self,
        runtime: &mut VideoRuntime,
        max_requests: usize,
    ) -> VideoDecodeWorkerReport {
        let mut report = VideoDecodeWorkerReport {
            attempted: 0,
            decoded: 0,
            skipped: 0,
            pending: self.scheduler.len(),
            errors: Vec::new(),
        };
        for _ in 0..max_requests {
            let Some(result) = self.decode_next_into_runtime(runtime) else {
                break;
            };
            report.attempted += 1;
            match result {
                Ok(VideoDecodeWorkerStep::Decoded(_)) => report.decoded += 1,
                Ok(VideoDecodeWorkerStep::Skipped) => report.skipped += 1,
                Err(error) => {
                    report.skipped += 1;
                    report.errors.push(error);
                }
            }
        }
        report.pending = self.scheduler.len();
        report
    }
}

pub struct VideoRuntime {
    config: VideoRuntimeConfig,
    queues: Vec<FrameQueue>,
}

pub trait VideoFrameProvider {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]);
    fn retain_inputs(&mut self, inputs: &[VideoRenderInput]) {
        let layer_ids = inputs
            .iter()
            .map(VideoRenderInput::layer_id)
            .collect::<Vec<_>>();
        self.retain_layers(&layer_ids);
    }
    fn frame_for_layer(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError>;
    fn frame_for_input(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        self.frame_for_layer(
            &VideoLayerSummary {
                id: input.request.layer_id,
                label: input.request.label.clone(),
                source: input.request.source.clone(),
                media_asset_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    position_ms: input.request.position_ms,
                    ..VideoLayerState::default()
                },
                isf_effect: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
            },
            input.request.width,
            input.request.height,
        )
    }
    fn frames_for_layer(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<Vec<VideoFrame>, VideoFrameProviderError> {
        self.frame_for_layer(layer, width, height)
            .map(|frame| vec![frame])
    }
}

pub trait VideoFrameDecoder {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]);
    fn retain_inputs(&mut self, inputs: &[VideoRenderInput]) {
        let layer_ids = inputs
            .iter()
            .map(VideoRenderInput::layer_id)
            .collect::<Vec<_>>();
        self.retain_layers(&layer_ids);
    }
    fn release_layer(&mut self, _layer_id: VideoLayerId) {}
    fn release_input(&mut self, input: &VideoRenderInput) {
        self.release_layer(input.layer_id());
    }
    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError>;
    fn decode_input_frame(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.decode_frame(&input.request)
    }
}

#[derive(Debug, Default)]
pub struct NullVideoDecoder;

pub struct PreviewFrameProvider {
    still_images: StillImageFrameCache,
}

#[derive(Debug, Clone)]
pub struct FfmpegCliFrameDecoder {
    binary: PathBuf,
    entries: Vec<FfmpegCliFrameCacheEntry>,
}

pub struct DecoderBackedFrameProvider<D = NullVideoDecoder> {
    still_images: StillImageFrameCache,
    decoder: D,
    placeholder_when_missing: bool,
    prefetch_count: usize,
    prefetch_interval_ms: u64,
    bpm: Option<f32>,
}

pub struct VideoPreviewRenderer<P = PreviewFrameProvider> {
    runtime: VideoRuntime,
    frame_provider: P,
    isf_shader_cache: HashMap<Arc<str>, CachedIsfShader>,
    isf_shader_cache_clock: u64,
    isf_runtime: Option<Result<IsfGpuRuntime, IsfRuntimeError>>,
    last_isf_error: Option<String>,
    last_isf_stage_errors: Vec<VideoIsfStageError>,
    last_render_input_isf_stage_errors: Vec<VideoRenderInputIsfStageError>,
    last_effect_stage_faults: Vec<VideoEffectStageFault>,
    output_last_valid_frames: Vec<VideoOutputLastValidFrame>,
    last_output_render_error: Option<String>,
}

const ISF_SHADER_CACHE_CAPACITY: usize = 64;
pub const VIDEO_ISF_EFFECT_STACK_MAX_STAGES: usize = VIDEO_EFFECT_CHAIN_MAX_STAGES;
const FFMPEG_CLI_FRAME_CACHE_CAPACITY: usize = 8;

fn format_video_isf_stage_error(error: &VideoIsfStageError, layer_label: Option<&str>) -> String {
    let layer = layer_label
        .map(|label| format!("layer {} '{}'", error.layer_id, label))
        .unwrap_or_else(|| format!("layer {}", error.layer_id));
    match (error.stage_index, error.stage_label.as_deref()) {
        (Some(stage_index), Some(stage_label)) => format!(
            "{} FX {} '{}': {}",
            layer,
            stage_index + 1,
            stage_label,
            error.message
        ),
        _ => format!("{} FX stack: {}", layer, error.message),
    }
}

fn video_effect_fault(
    scope: VideoEffectScope,
    chain_id: Option<VideoEffectChainId>,
    stage_id: Option<VideoEffectStageId>,
    effect_id: Option<VideoEffectId>,
    render_input_key: Option<protocol::VideoRenderInputKey>,
    stage_index: Option<usize>,
    stage_label: Option<String>,
    message: impl Into<String>,
) -> VideoEffectStageFault {
    VideoEffectStageFault {
        scope,
        chain_id,
        stage_id,
        effect_id,
        render_input_key,
        stage_index,
        stage_label,
        message: message.into(),
    }
}

fn legacy_isf_stages(effect: &VideoIsfEffectSummary) -> Vec<VideoIsfEffectSummary> {
    let mut root = effect.clone();
    root.stack.clear();
    std::iter::once(root)
        .chain(effect.stack.iter().map(|stage| VideoIsfEffectSummary {
            enabled: stage.enabled,
            label: stage.label.clone(),
            source: Arc::clone(&stage.source),
            source_path: stage.source_path.clone(),
            description: stage.description.clone(),
            categories: stage.categories.clone(),
            controls: stage.controls.clone(),
            stack: Vec::new(),
        }))
        .collect()
}

fn resolved_legacy_layer_chain(
    layer_id: VideoLayerId,
    effect: &VideoIsfEffectSummary,
) -> Result<ResolvedVideoEffectChain, VideoEffectStageFault> {
    let scope = VideoEffectScope::Layer { layer_id };
    let legacy_stages = legacy_isf_stages(effect);
    if legacy_stages.len() > VIDEO_EFFECT_CHAIN_MAX_STAGES {
        return Err(video_effect_fault(
            scope,
            None,
            None,
            None,
            None,
            None,
            None,
            format!(
                "legacy ISF stack has {} stages; the limit is {}",
                legacy_stages.len(),
                VIDEO_EFFECT_CHAIN_MAX_STAGES
            ),
        ));
    }
    Ok(ResolvedVideoEffectChain {
        chain_id: None,
        scope,
        bypassed: false,
        stages: legacy_stages
            .into_iter()
            .enumerate()
            .map(|(index, effect)| ResolvedVideoEffectStage {
                stage_id: None,
                effect_id: None,
                enabled: effect.enabled,
                label: if effect.label.is_empty() {
                    format!("Legacy FX {}", index + 1)
                } else {
                    effect.label.clone()
                },
                effect,
            })
            .collect(),
    })
}

fn resolved_chain_from_canonical(
    chain: &VideoEffectChainSummary,
) -> Result<ResolvedVideoEffectChain, VideoEffectStageFault> {
    if chain.stages.len() > VIDEO_EFFECT_CHAIN_MAX_STAGES {
        return Err(video_effect_fault(
            chain.scope.clone(),
            Some(chain.id),
            None,
            None,
            None,
            None,
            None,
            format!(
                "effect chain has {} stages; the limit is {}",
                chain.stages.len(),
                VIDEO_EFFECT_CHAIN_MAX_STAGES
            ),
        ));
    }
    let stages = chain
        .stages
        .iter()
        .map(|stage| {
            let VideoEffectKind::Isf { effect } = &stage.effect.kind;
            let mut effect = effect.clone();
            effect.stack.clear();
            ResolvedVideoEffectStage {
                stage_id: Some(stage.id),
                effect_id: Some(stage.effect.id),
                enabled: stage.enabled,
                label: stage.label.clone(),
                effect,
            }
        })
        .collect();
    Ok(ResolvedVideoEffectChain {
        chain_id: Some(chain.id),
        scope: chain.scope.clone(),
        bypassed: chain.bypassed,
        stages,
    })
}

fn resolve_effective_layer_chain(
    snapshot: &VideoSnapshot,
    layer: &VideoLayerSummary,
) -> Result<Option<ResolvedVideoEffectChain>, VideoEffectStageFault> {
    let scope = VideoEffectScope::Layer { layer_id: layer.id };
    let canonical = snapshot
        .effect_chains
        .iter()
        .find(|chain| chain.scope == scope);
    let Some(canonical) = canonical else {
        let Some(legacy) = layer.isf_effect.as_ref() else {
            return Ok(None);
        };
        return resolved_legacy_layer_chain(layer.id, legacy).map(Some);
    };

    let mut resolved = resolved_chain_from_canonical(canonical)?;
    if resolved.stages.is_empty() {
        if layer.isf_effect.is_some() {
            return Err(video_effect_fault(
                scope,
                Some(canonical.id),
                None,
                None,
                None,
                None,
                None,
                "empty canonical Layer chain has a legacy ISF projection",
            ));
        }
        return Ok(Some(resolved));
    }
    let Some(legacy) = layer.isf_effect.as_ref() else {
        return Err(video_effect_fault(
            scope,
            Some(canonical.id),
            None,
            None,
            None,
            None,
            None,
            "canonical Layer chain is missing its rendered legacy projection",
        ));
    };
    let effective = legacy_isf_stages(legacy);
    if effective.len() != resolved.stages.len() {
        return Err(video_effect_fault(
            scope,
            Some(canonical.id),
            None,
            None,
            None,
            None,
            None,
            format!(
                "canonical Layer chain has {} stages but its rendered projection has {}",
                resolved.stages.len(),
                effective.len()
            ),
        ));
    }
    for (index, (stage, effective)) in resolved
        .stages
        .iter_mut()
        .zip(effective.into_iter())
        .enumerate()
    {
        if stage.effect.label != effective.label || stage.effect.source != effective.source {
            return Err(video_effect_fault(
                scope,
                resolved.chain_id,
                stage.stage_id,
                stage.effect_id,
                None,
                Some(index),
                Some(stage.label.clone()),
                "canonical Layer stage diverges from its rendered legacy projection",
            ));
        }
        stage.enabled = effective.enabled;
        stage.effect = effective;
    }
    Ok(Some(resolved))
}

pub fn resolve_video_effect_chain(
    snapshot: &VideoSnapshot,
    scope: &VideoEffectScope,
) -> Result<Option<ResolvedVideoEffectChain>, VideoEffectStageFault> {
    if let VideoEffectScope::Layer { layer_id } = scope {
        let Some(layer) = snapshot.layers.iter().find(|layer| layer.id == *layer_id) else {
            return Err(video_effect_fault(
                scope.clone(),
                None,
                None,
                None,
                None,
                None,
                None,
                format!("effect scope references missing layer {layer_id}"),
            ));
        };
        return resolve_effective_layer_chain(snapshot, layer);
    }
    snapshot
        .effect_chains
        .iter()
        .find(|chain| &chain.scope == scope)
        .map(resolved_chain_from_canonical)
        .transpose()
}

fn resolved_chain_executes(chain: &ResolvedVideoEffectChain) -> bool {
    !chain.bypassed && chain.stages.iter().any(|stage| stage.enabled)
}

#[derive(Default)]
pub struct StillImageFrameCache {
    entries: Vec<StillImageFrameCacheEntry>,
}

struct StillImageFrameCacheEntry {
    key: protocol::VideoRenderInputKey,
    layer_id: VideoLayerId,
    path: PathBuf,
    width: u32,
    height: u32,
    signature: StillImageSignature,
    frame: VideoFrame,
}

#[derive(Debug, Clone)]
struct FfmpegCliFrameCacheEntry {
    key: protocol::VideoRenderInputKey,
    layer_id: VideoLayerId,
    path: PathBuf,
    width: u32,
    height: u32,
    position_ms: u64,
    signature: StillImageSignature,
    frame: VideoFrame,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StillImageSignature {
    len: Option<u64>,
    modified: Option<SystemTime>,
}

impl FrameQueue {
    pub fn new(layer_id: VideoLayerId, capacity: usize) -> Self {
        Self::new_for_input(protocol::VideoRenderInputKey::LEGACY, layer_id, capacity)
    }

    pub fn new_for_input(
        key: protocol::VideoRenderInputKey,
        layer_id: VideoLayerId,
        capacity: usize,
    ) -> Self {
        assert!(
            capacity > 0,
            "frame queue capacity must be greater than zero"
        );
        Self {
            key,
            layer_id,
            slots: vec![None; capacity],
            start: 0,
            len: 0,
        }
    }

    pub fn layer_id(&self) -> VideoLayerId {
        self.layer_id
    }

    pub fn key(&self) -> protocol::VideoRenderInputKey {
        self.key
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn push(&mut self, mut frame: VideoFrame) -> FrameQueuePush {
        frame.layer_id = self.layer_id;
        if let Some(index) = (0..self.len)
            .map(|offset| (self.start + offset) % self.capacity())
            .find(|index| {
                self.slots[*index]
                    .as_ref()
                    .is_some_and(|existing| existing.pts_ms == frame.pts_ms)
            })
        {
            self.slots[index] = Some(frame);
            return FrameQueuePush::Inserted;
        }
        let result = if self.len == self.capacity() {
            let index = self.start;
            self.slots[index] = Some(frame);
            self.start = (self.start + 1) % self.capacity();
            FrameQueuePush::DroppedOldest
        } else {
            let index = (self.start + self.len) % self.capacity();
            self.slots[index] = Some(frame);
            self.len += 1;
            FrameQueuePush::Inserted
        };
        self.sort_by_pts();
        result
    }

    pub fn nearest(&self, target_ms: u64) -> Option<&VideoFrame> {
        self.iter()
            .min_by_key(|frame| frame.pts_ms.abs_diff(target_ms))
    }

    pub fn frame_at_or_before(&self, target_ms: u64) -> Option<&VideoFrame> {
        self.iter()
            .filter(|frame| frame.pts_ms <= target_ms)
            .max_by_key(|frame| frame.pts_ms)
    }

    pub fn drop_before(&mut self, target_ms: u64) -> usize {
        let original_len = self.len;
        let mut frames = self
            .drain_frames()
            .into_iter()
            .filter(|frame| frame.pts_ms >= target_ms)
            .collect::<Vec<_>>();
        let removed = original_len.saturating_sub(frames.len());
        self.rebuild_from_sorted(&mut frames);
        removed
    }

    pub fn iter(&self) -> impl Iterator<Item = &VideoFrame> {
        (0..self.len).filter_map(|offset| {
            let index = (self.start + offset) % self.capacity();
            self.slots[index].as_ref()
        })
    }

    fn sort_by_pts(&mut self) {
        let mut frames = self.drain_frames();
        self.rebuild_from_sorted(&mut frames);
    }

    fn drain_frames(&mut self) -> Vec<VideoFrame> {
        let frames = (0..self.len)
            .filter_map(|offset| {
                let index = (self.start + offset) % self.capacity();
                self.slots[index].take()
            })
            .collect::<Vec<_>>();
        self.start = 0;
        self.len = 0;
        frames
    }

    fn rebuild_from_sorted(&mut self, frames: &mut Vec<VideoFrame>) {
        frames.sort_by_key(|frame| frame.pts_ms);
        for slot in &mut self.slots {
            *slot = None;
        }
        self.start = 0;
        self.len = frames.len().min(self.capacity());
        for (index, frame) in frames.drain(..self.len).enumerate() {
            self.slots[index] = Some(frame);
        }
    }
}

impl VideoRuntime {
    pub fn new(config: VideoRuntimeConfig) -> Self {
        assert!(
            config.frame_queue_capacity > 0,
            "frame queue capacity must be greater than zero"
        );
        Self {
            config,
            queues: Vec::new(),
        }
    }

    pub fn config(&self) -> VideoRuntimeConfig {
        self.config
    }

    pub fn sync_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.queues.retain(|queue| {
            layer_ids
                .iter()
                .any(|layer_id| *layer_id == queue.layer_id())
        });
        for layer_id in layer_ids {
            if !self
                .queues
                .iter()
                .any(|queue| queue.layer_id() == *layer_id)
            {
                self.queues
                    .push(FrameQueue::new(*layer_id, self.config.frame_queue_capacity));
            }
        }
    }

    /// Synchronizes queue ownership by the runtime input key. This is separate
    /// from `sync_layers`: a layer is only a diagnostic label and may appear in
    /// several simultaneous render inputs.
    pub fn sync_inputs(&mut self, inputs: &[VideoRenderInput]) {
        self.queues
            .retain(|queue| inputs.iter().any(|input| input.key == queue.key()));
        for input in inputs {
            if !self.queues.iter().any(|queue| queue.key() == input.key) {
                self.queues.push(FrameQueue::new_for_input(
                    input.key,
                    input.layer_id(),
                    self.config.frame_queue_capacity,
                ));
            }
        }
    }

    pub fn push_frame(&mut self, frame: VideoFrame) -> FrameQueuePush {
        let layer_id = frame.layer_id;
        if !self.queues.iter().any(|queue| queue.layer_id() == layer_id) {
            self.queues
                .push(FrameQueue::new(layer_id, self.config.frame_queue_capacity));
        }
        self.queues
            .iter_mut()
            .find(|queue| queue.layer_id() == layer_id)
            .expect("queue exists after insertion")
            .push(frame)
    }

    pub fn push_input_frame(
        &mut self,
        input: &VideoRenderInput,
        mut frame: VideoFrame,
    ) -> FrameQueuePush {
        frame.layer_id = input.layer_id();
        if !self.queues.iter().any(|queue| queue.key() == input.key) {
            self.queues.push(FrameQueue::new_for_input(
                input.key,
                input.layer_id(),
                self.config.frame_queue_capacity,
            ));
        }
        self.queues
            .iter_mut()
            .find(|queue| queue.key() == input.key)
            .expect("queue exists after insertion")
            .push(frame)
    }

    pub fn queue_len(&self, layer_id: VideoLayerId) -> usize {
        self.queues
            .iter()
            .find(|queue| queue.layer_id() == layer_id)
            .map(FrameQueue::len)
            .unwrap_or(0)
    }

    pub fn queue_count(&self) -> usize {
        self.queues.len()
    }

    pub fn input_queue_len(&self, key: protocol::VideoRenderInputKey) -> usize {
        self.queues
            .iter()
            .find(|queue| queue.key() == key)
            .map(FrameQueue::len)
            .unwrap_or(0)
    }

    pub fn compose_first_plan(
        &self,
        snapshot: &VideoSnapshot,
    ) -> Result<VideoFrame, VideoRuntimeError> {
        let plan = build_composition_plans(snapshot)
            .into_iter()
            .next()
            .ok_or(VideoRuntimeError::MissingComposition)?;
        self.compose_plan(&plan, self.config.preview_width, self.config.preview_height)
    }

    pub fn compose_plan(
        &self,
        plan: &CompositionPlan,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoRuntimeError> {
        let frames = self.select_frames_for_plan(plan);
        composite_rgba8(plan, &frames, width, height).map_err(VideoRuntimeError::from)
    }

    pub fn compose_input_mix(
        &self,
        mix: &CompositionInputMix,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoRuntimeError> {
        let frames = self.select_frames_for_input_mix(mix);
        composite_input_mix_rgba8(mix, &frames, width, height).map_err(VideoRuntimeError::from)
    }

    fn select_frames_for_plan(&self, plan: &CompositionPlan) -> Vec<VideoFrame> {
        plan.layers
            .iter()
            .filter_map(|layer| {
                self.queues
                    .iter()
                    .find(|queue| queue.layer_id() == layer.layer_id)
                    .and_then(|queue| {
                        queue
                            .frame_at_or_before(layer.position_ms)
                            .or_else(|| queue.nearest(layer.position_ms))
                    })
                    .cloned()
            })
            .collect()
    }

    fn select_frames_for_input_mix(&self, mix: &CompositionInputMix) -> Vec<VideoRenderInputFrame> {
        mix.inputs
            .iter()
            .filter_map(|input| {
                self.queues
                    .iter()
                    .find(|queue| queue.key() == input.key)
                    .and_then(|queue| {
                        queue
                            .frame_at_or_before(input.layer.position_ms)
                            .or_else(|| queue.nearest(input.layer.position_ms))
                    })
                    .cloned()
                    .map(|frame| VideoRenderInputFrame {
                        key: input.key,
                        frame,
                    })
            })
            .collect()
    }
}

impl Default for PreviewFrameProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewFrameProvider {
    pub fn new() -> Self {
        Self {
            still_images: StillImageFrameCache::new(),
        }
    }

    pub fn still_image_cache_len(&self) -> usize {
        self.still_images.len()
    }
}

impl VideoFrameProvider for PreviewFrameProvider {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.still_images.retain_layers(layer_ids);
    }

    fn retain_inputs(&mut self, inputs: &[VideoRenderInput]) {
        self.still_images.retain_inputs(inputs);
    }

    fn frame_for_layer(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        let state = sanitize_layer_state(layer.state.clone());
        match layer.source.kind {
            VideoSourceKind::StillImage => {
                let path = layer
                    .source
                    .path
                    .as_deref()
                    .filter(|path| !path.is_empty())
                    .ok_or_else(|| VideoFrameProviderError::MissingStillImagePath {
                        layer_id: layer.id,
                        label: layer.label.clone(),
                    })?;
                self.still_images
                    .frame_for_layer(layer.id, path, state.position_ms, width, height)
                    .map_err(|error| VideoFrameProviderError::StillImage {
                        layer_id: layer.id,
                        label: layer.label.clone(),
                        error,
                    })
            }
            _ => Ok(debug_solid_frame_for_layer(
                layer.id,
                state.position_ms,
                width,
                height,
            )),
        }
    }

    fn frame_for_input(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        let request = &input.request;
        match request.source.kind {
            VideoSourceKind::StillImage => {
                let path = request
                    .source
                    .path
                    .as_deref()
                    .filter(|path| !path.is_empty())
                    .ok_or_else(|| VideoFrameProviderError::MissingStillImagePath {
                        layer_id: request.layer_id,
                        label: request.label.clone(),
                    })?;
                self.still_images
                    .frame_for_input(
                        input.key,
                        request.layer_id,
                        path,
                        request.position_ms,
                        request.width,
                        request.height,
                    )
                    .map_err(|error| VideoFrameProviderError::StillImage {
                        layer_id: request.layer_id,
                        label: request.label.clone(),
                        error,
                    })
            }
            _ => Ok(debug_solid_frame_for_layer(
                request.layer_id,
                request.position_ms,
                request.width,
                request.height,
            )),
        }
    }
}

impl Default for FfmpegCliFrameDecoder {
    fn default() -> Self {
        Self::from_env()
    }
}

impl FfmpegCliFrameDecoder {
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            entries: Vec::new(),
        }
    }

    pub fn from_env() -> Self {
        Self::new(std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into()))
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }

    pub fn cache_len(&self) -> usize {
        self.entries.len()
    }

    fn evict_input(&mut self, key: protocol::VideoRenderInputKey, layer_id: VideoLayerId) {
        self.entries.retain(|entry| {
            !render_input_cache_owner_matches(key, layer_id, entry.key, entry.layer_id)
        });
    }

    fn cached_frame(
        &mut self,
        key: protocol::VideoRenderInputKey,
        request: &VideoFrameRequest,
        path: &Path,
        signature: &StillImageSignature,
    ) -> Option<VideoFrame> {
        let index = self.entries.iter().position(|entry| {
            entry.key == key
                && entry.layer_id == request.layer_id
                && entry.path == path
                && entry.width == request.width
                && entry.height == request.height
                && entry.position_ms == request.position_ms
                && &entry.signature == signature
        })?;
        let entry = self.entries.remove(index);
        let frame = entry.frame.clone();
        self.entries.push(entry);
        Some(frame)
    }

    fn cache_frame(&mut self, entry: FfmpegCliFrameCacheEntry) {
        self.evict_input(entry.key, entry.layer_id);
        self.entries.push(entry);
        if self.entries.len() > FFMPEG_CLI_FRAME_CACHE_CAPACITY {
            self.entries.remove(0);
        }
    }

    fn decode_input(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        let request = &input.request;
        if request.source.kind != VideoSourceKind::File {
            self.evict_input(input.key, request.layer_id);
            return Ok(None);
        }

        let Some(path) = request
            .source
            .path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
        else {
            self.evict_input(input.key, request.layer_id);
            return Err(VideoDecodeError::MissingSourcePath {
                layer_id: request.layer_id,
                label: request.label.clone(),
            });
        };
        let path = PathBuf::from(path);
        let signature = StillImageSignature::from_path(&path);
        if let Some(frame) = self.cached_frame(input.key, request, &path, &signature) {
            return Ok(Some(frame));
        }

        self.evict_input(input.key, request.layer_id);
        let frame = decode_ffmpeg_cli_frame(request, &self.binary, &path)?;
        self.cache_frame(FfmpegCliFrameCacheEntry {
            key: input.key,
            layer_id: request.layer_id,
            path,
            width: request.width,
            height: request.height,
            position_ms: request.position_ms,
            signature,
            frame: frame.clone(),
        });
        Ok(Some(frame))
    }
}

pub fn probe_video_file_metadata(
    path: impl AsRef<Path>,
) -> Result<VideoProbeSummary, VideoProbeError> {
    probe_video_file_metadata_with_binary(path, ffprobe_binary_from_env())
}

pub fn video_runtime_status() -> VideoRuntimeStatus {
    video_runtime_status_with_binaries(
        FfmpegCliFrameDecoder::from_env().binary().to_path_buf(),
        ffprobe_binary_from_env(),
    )
}

pub fn video_runtime_status_with_binaries(
    ffmpeg_binary: impl AsRef<Path>,
    ffprobe_binary: impl AsRef<Path>,
) -> VideoRuntimeStatus {
    VideoRuntimeStatus {
        backends: vec![
            VideoBackendStatus {
                id: "still_image".to_string(),
                label: "Still image decode".to_string(),
                state: VideoBackendState::Available,
                detail: "PNG/JPEG decode is built in".to_string(),
            },
            VideoBackendStatus {
                id: "libav".to_string(),
                label: "In-process general video decode".to_string(),
                state: if LibavFrameDecoder::is_built() {
                    VideoBackendState::Available
                } else {
                    VideoBackendState::NotBuilt
                },
                detail: if LibavFrameDecoder::is_built() {
                    "libavcodec/libavformat decode is built in for H.264, H.265, and ProRes"
                        .to_string()
                } else {
                    "Build with the libav feature; FFmpeg CLI remains the compatibility path"
                        .to_string()
                },
            },
            command_backend_status("ffmpeg", "FFmpeg frame decode", ffmpeg_binary.as_ref()),
            capture_backend_status("camera", "Camera capture", ffmpeg_binary.as_ref()),
            capture_backend_status("screen_capture", "Screen capture", ffmpeg_binary.as_ref()),
            command_backend_status("ffprobe", "FFprobe metadata", ffprobe_binary.as_ref()),
            ffmpeg_decoder_backend_status(
                "hap_ffmpeg",
                "HAP FFmpeg decode",
                ffmpeg_binary.as_ref(),
                "hap",
            ),
            VideoBackendStatus {
                id: "dxt_cpu_reference".to_string(),
                label: "DXT CPU reference".to_string(),
                state: VideoBackendState::Available,
                detail: "DXT1/DXT5 compressed frames can be expanded by the CPU reference compositor"
                    .to_string(),
            },
            VideoBackendStatus {
                id: "hap_gpu".to_string(),
                label: "HAP in-process decode".to_string(),
                state: VideoBackendState::Available,
                detail: "Pure Rust MOV demux and HAP/HAP Q BC1/BC3 decode are built in; HAP Q Alpha merges its BC4 plane to RGBA, while HAP R BC7 remains staged separately"
                    .to_string(),
            },
            external_backend_status("ndi", "NDI input/output", "NDI SDK"),
            platform_external_backend_status(
                "spout",
                "Spout input/output",
                "Spout",
                PlatformSupport::WindowsOnly,
            ),
            platform_external_backend_status(
                "syphon",
                "Syphon input/output",
                "Syphon",
                PlatformSupport::MacosOnly,
            ),
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlatformSupport {
    WindowsOnly,
    MacosOnly,
}

fn external_backend_status(id: &str, label: &str, sdk_name: &str) -> VideoBackendStatus {
    if id == "ndi" && cfg!(feature = "ndi") {
        return VideoBackendStatus {
            id: id.to_string(),
            label: label.to_string(),
            state: VideoBackendState::Available,
            detail: "NDI SDK transport is built in; routes initialize the runtime on demand"
                .to_string(),
        };
    }
    if id == "spout"
        && cfg!(all(
            feature = "spout",
            target_os = "windows",
            target_arch = "x86_64"
        ))
    {
        return VideoBackendStatus {
            id: id.to_string(),
            label: label.to_string(),
            state: VideoBackendState::Available,
            detail: "Spout2 2.007.017 DirectX 11 CPU-pixel transport is built in; routes initialize senders and receivers on demand".to_string(),
        };
    }
    VideoBackendStatus {
        id: id.to_string(),
        label: label.to_string(),
        state: VideoBackendState::NotBuilt,
        detail: format!("{sdk_name} backend is not linked in this build"),
    }
}

fn platform_external_backend_status(
    id: &str,
    label: &str,
    sdk_name: &str,
    support: PlatformSupport,
) -> VideoBackendStatus {
    if platform_backend_is_supported(support) {
        return external_backend_status(id, label, sdk_name);
    }
    let platform = match support {
        PlatformSupport::WindowsOnly => "Windows-only",
        PlatformSupport::MacosOnly => "macOS-only",
    };
    VideoBackendStatus {
        id: id.to_string(),
        label: label.to_string(),
        state: VideoBackendState::NotBuilt,
        detail: format!("{sdk_name} is {platform} and is not available on this platform"),
    }
}

fn platform_backend_is_supported(support: PlatformSupport) -> bool {
    match support {
        PlatformSupport::WindowsOnly => cfg!(target_os = "windows"),
        PlatformSupport::MacosOnly => cfg!(target_os = "macos"),
    }
}

fn capture_backend_status(id: &str, label: &str, binary: &Path) -> VideoBackendStatus {
    let mut status = command_backend_status(id, label, binary);
    if status.state == VideoBackendState::Available {
        let input = if id == "camera" {
            "camera device"
        } else {
            "desktop/display"
        };
        status.detail = format!(
            "FFmpeg persistent {input} capture is available on {}; routes start on demand",
            std::env::consts::OS
        );
    }
    status
}

fn command_backend_status(id: &str, label: &str, binary: &Path) -> VideoBackendStatus {
    match Command::new(binary).arg("-version").output() {
        Ok(output) if output.status.success() => VideoBackendStatus {
            id: id.to_string(),
            label: label.to_string(),
            state: VideoBackendState::Available,
            detail: command_version_detail(binary, &output.stdout),
        },
        Ok(output) => VideoBackendStatus {
            id: id.to_string(),
            label: label.to_string(),
            state: VideoBackendState::Missing,
            detail: format!(
                "'{} -version' exited with {}",
                binary.display(),
                output.status
            ),
        },
        Err(error) => VideoBackendStatus {
            id: id.to_string(),
            label: label.to_string(),
            state: VideoBackendState::Missing,
            detail: format!("'{}' is not available: {error}", binary.display()),
        },
    }
}

fn ffmpeg_decoder_backend_status(
    id: &str,
    label: &str,
    binary: &Path,
    decoder_name: &str,
) -> VideoBackendStatus {
    match Command::new(binary).arg("-decoders").output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if ffmpeg_decoders_list_contains(&stdout, decoder_name) {
                VideoBackendStatus {
                    id: id.to_string(),
                    label: label.to_string(),
                    state: VideoBackendState::Available,
                    detail: format!(
                        "{}: FFmpeg decoder '{decoder_name}' is available",
                        binary.display()
                    ),
                }
            } else {
                VideoBackendStatus {
                    id: id.to_string(),
                    label: label.to_string(),
                    state: VideoBackendState::Missing,
                    detail: format!(
                        "{}: FFmpeg decoder '{decoder_name}' was not listed by -decoders",
                        binary.display()
                    ),
                }
            }
        }
        Ok(output) => VideoBackendStatus {
            id: id.to_string(),
            label: label.to_string(),
            state: VideoBackendState::Missing,
            detail: format!(
                "'{} -decoders' exited with {}",
                binary.display(),
                output.status
            ),
        },
        Err(error) => VideoBackendStatus {
            id: id.to_string(),
            label: label.to_string(),
            state: VideoBackendState::Missing,
            detail: format!("'{}' is not available: {error}", binary.display()),
        },
    }
}

fn ffmpeg_decoders_list_contains(decoders_output: &str, decoder_name: &str) -> bool {
    decoders_output.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("V")
            && trimmed
                .split_whitespace()
                .nth(1)
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(decoder_name))
    })
}

fn command_version_detail(binary: &Path, stdout: &[u8]) -> String {
    let first_line = String::from_utf8_lossy(stdout)
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "version command succeeded".to_string());
    format!("{}: {first_line}", binary.display())
}

pub fn infer_video_codec_from_path(path: impl AsRef<Path>) -> Option<String> {
    let extension = path
        .as_ref()
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase();
    match extension.as_str() {
        "hap" => Some("hap".to_string()),
        "hapq" => Some("hap-q".to_string()),
        _ => None,
    }
}

pub fn probe_video_file_metadata_with_binary(
    path: impl AsRef<Path>,
    binary: impl AsRef<Path>,
) -> Result<VideoProbeSummary, VideoProbeError> {
    let path = path.as_ref();
    let output = Command::new(binary.as_ref())
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg(
            "stream=codec_type,codec_name,width,height,avg_frame_rate,r_frame_rate:format=duration",
        )
        .arg("-of")
        .arg("json")
        .arg(path)
        .output()
        .map_err(|error| VideoProbeError::Probe {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(VideoProbeError::Probe {
            path: path.to_path_buf(),
            message: if stderr.is_empty() {
                format!("ffprobe exited with {}", output.status)
            } else {
                stderr
            },
        });
    }
    let json = String::from_utf8_lossy(&output.stdout);
    parse_ffprobe_metadata_json(&json)
}

fn ffprobe_binary_from_env() -> PathBuf {
    if let Some(binary) = std::env::var_os("SYNDOCAL_FFPROBE") {
        return binary.into();
    }
    if let Some(ffmpeg) = std::env::var_os("SYNDOCAL_FFMPEG") {
        let path = PathBuf::from(ffmpeg);
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if file_name.eq_ignore_ascii_case("ffmpeg") || file_name.eq_ignore_ascii_case("ffmpeg.exe")
        {
            let ffprobe_name = if file_name.ends_with(".exe") {
                "ffprobe.exe"
            } else {
                "ffprobe"
            };
            return path.with_file_name(ffprobe_name);
        }
    }
    "ffprobe".into()
}

fn parse_ffprobe_metadata_json(json: &str) -> Result<VideoProbeSummary, VideoProbeError> {
    let parsed: FfprobeOutput = serde_json::from_str(json)
        .map_err(|error| VideoProbeError::InvalidJson(error.to_string()))?;
    let has_audio = parsed
        .streams
        .iter()
        .any(|stream| stream.codec_type.as_deref() == Some("audio"));
    let stream = parsed
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("video"))
        .or_else(|| parsed.streams.iter().find(|stream| stream.width.is_some()));
    let duration_ms = parsed
        .format
        .and_then(|format| format.duration)
        .and_then(|duration| parse_duration_ms(&duration));
    let codec = stream
        .as_ref()
        .and_then(|stream| stream.codec_name.as_ref())
        .filter(|codec| !codec.trim().is_empty())
        .cloned();
    let width = stream
        .as_ref()
        .and_then(|stream| stream.width)
        .filter(|width| *width > 0);
    let height = stream
        .as_ref()
        .and_then(|stream| stream.height)
        .filter(|height| *height > 0);
    let frame_rate = stream.as_ref().and_then(|stream| {
        stream
            .avg_frame_rate
            .as_deref()
            .and_then(parse_frame_rate)
            .or_else(|| stream.r_frame_rate.as_deref().and_then(parse_frame_rate))
    });
    let metadata = VideoMediaMetadata {
        duration_ms,
        width,
        height,
        frame_rate,
        has_audio,
    };
    let metadata = if metadata.duration_ms.is_some()
        || metadata.width.is_some()
        || metadata.height.is_some()
        || metadata.frame_rate.is_some()
    {
        Some(metadata)
    } else {
        None
    };
    Ok(VideoProbeSummary { codec, metadata })
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

fn parse_duration_ms(value: &str) -> Option<u64> {
    let seconds = value.trim().parse::<f64>().ok()?;
    if seconds.is_finite() && seconds >= 0.0 {
        Some((seconds * 1000.0).round() as u64)
    } else {
        None
    }
}

fn parse_frame_rate(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some((numerator, denominator)) = value.split_once('/') {
        let numerator = numerator.trim().parse::<f64>().ok()?;
        let denominator = denominator.trim().parse::<f64>().ok()?;
        if numerator.is_finite() && denominator.is_finite() && denominator.abs() > f64::EPSILON {
            let rate = numerator / denominator;
            return (rate.is_finite() && rate > 0.0).then_some(rate as f32);
        }
        return None;
    }
    let rate = value.parse::<f32>().ok()?;
    (rate.is_finite() && rate > 0.0).then_some(rate)
}

impl VideoFrameDecoder for FfmpegCliFrameDecoder {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.entries
            .retain(|entry| layer_ids.iter().any(|layer_id| *layer_id == entry.layer_id));
    }

    fn release_layer(&mut self, layer_id: VideoLayerId) {
        self.entries.retain(|entry| entry.layer_id != layer_id);
    }

    fn retain_inputs(&mut self, inputs: &[VideoRenderInput]) {
        self.entries
            .retain(|entry| inputs.iter().any(|input| input.key == entry.key));
    }

    fn release_input(&mut self, input: &VideoRenderInput) {
        self.evict_input(input.key, input.layer_id());
    }

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.decode_input(&VideoRenderInput::legacy(request.clone()))
    }

    fn decode_input_frame(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.decode_input(input)
    }
}

impl VideoFrameDecoder for NullVideoDecoder {
    fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

    fn decode_frame(
        &mut self,
        _request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        Ok(None)
    }
}

impl<D> DecoderBackedFrameProvider<D> {
    pub fn new(decoder: D) -> Self {
        Self {
            still_images: StillImageFrameCache::new(),
            decoder,
            placeholder_when_missing: true,
            prefetch_count: 0,
            prefetch_interval_ms: 33,
            bpm: None,
        }
    }

    pub fn with_placeholder_when_missing(mut self, enabled: bool) -> Self {
        self.placeholder_when_missing = enabled;
        self
    }

    pub fn with_prefetch(mut self, count: usize, interval_ms: u64) -> Self {
        self.prefetch_count = count;
        self.prefetch_interval_ms = interval_ms.max(1);
        self
    }

    pub fn with_bpm(mut self, bpm: Option<f32>) -> Self {
        self.set_bpm(bpm);
        self
    }

    pub fn set_bpm(&mut self, bpm: Option<f32>) {
        self.bpm = bpm.filter(|bpm| bpm.is_finite() && *bpm > 0.0);
    }

    pub fn bpm(&self) -> Option<f32> {
        self.bpm
    }

    pub fn prefetch_count(&self) -> usize {
        self.prefetch_count
    }

    pub fn prefetch_interval_ms(&self) -> u64 {
        self.prefetch_interval_ms
    }

    pub fn decoder(&self) -> &D {
        &self.decoder
    }

    pub fn decoder_mut(&mut self) -> &mut D {
        &mut self.decoder
    }

    pub fn still_image_cache_len(&self) -> usize {
        self.still_images.len()
    }
}

impl<D: VideoFrameDecoder> VideoFrameProvider for DecoderBackedFrameProvider<D> {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.still_images.retain_layers(layer_ids);
        self.decoder.retain_layers(layer_ids);
    }

    fn retain_inputs(&mut self, inputs: &[VideoRenderInput]) {
        self.still_images.retain_inputs(inputs);
        self.decoder.retain_inputs(inputs);
    }

    fn frame_for_layer(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        self.decode_or_placeholder_for_layer(layer, width, height, None)
    }

    fn frame_for_input(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        self.decode_or_placeholder_for_input(input)
    }

    fn frames_for_layer(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<Vec<VideoFrame>, VideoFrameProviderError> {
        let positions = preview_prefetch_positions_ms(
            layer,
            self.prefetch_count,
            self.prefetch_interval_ms,
            self.bpm,
        );
        let mut frames = Vec::with_capacity(positions.len());
        for position_ms in positions {
            frames.push(self.decode_or_placeholder_for_layer(
                layer,
                width,
                height,
                Some(position_ms),
            )?);
        }
        Ok(frames)
    }
}

impl<D: VideoFrameDecoder> DecoderBackedFrameProvider<D> {
    fn decode_or_placeholder_for_layer(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
        position_override_ms: Option<u64>,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        let state = sanitize_layer_state(layer.state.clone());
        let position_ms = position_override_ms.unwrap_or(state.position_ms);
        let input = VideoRenderInput::legacy(VideoFrameRequest {
            layer_id: layer.id,
            label: layer.label.clone(),
            source: layer.source.clone(),
            position_ms,
            width,
            height,
        });
        self.decode_or_placeholder_for_input(&input)
    }

    fn decode_or_placeholder_for_input(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        let request = &input.request;
        if matches!(request.source.kind, VideoSourceKind::StillImage) {
            self.decoder.release_input(input);
            let path = request
                .source
                .path
                .as_deref()
                .filter(|path| !path.is_empty())
                .ok_or_else(|| VideoFrameProviderError::MissingStillImagePath {
                    layer_id: request.layer_id,
                    label: request.label.clone(),
                })?;
            return self
                .still_images
                .frame_for_input(
                    input.key,
                    request.layer_id,
                    path,
                    request.position_ms,
                    request.width,
                    request.height,
                )
                .map_err(|error| VideoFrameProviderError::StillImage {
                    layer_id: request.layer_id,
                    label: request.label.clone(),
                    error,
                });
        }

        match self.decoder.decode_input_frame(input) {
            Ok(Some(frame)) => Ok(frame),
            Ok(None) if self.placeholder_when_missing => Ok(debug_solid_frame_for_layer(
                request.layer_id,
                request.position_ms,
                request.width,
                request.height,
            )),
            Ok(None) => Err(VideoFrameProviderError::Decode {
                layer_id: request.layer_id,
                label: request.label.clone(),
                error: VideoDecodeError::UnsupportedSource {
                    layer_id: request.layer_id,
                    label: request.label.clone(),
                    kind: request.source.kind.clone(),
                },
            }),
            Err(error) => Err(VideoFrameProviderError::Decode {
                layer_id: request.layer_id,
                label: request.label.clone(),
                error,
            }),
        }
    }
}

impl VideoPreviewRenderer<PreviewFrameProvider> {
    pub fn new(config: VideoRuntimeConfig) -> Self {
        Self::with_frame_provider(config, PreviewFrameProvider::new())
    }

    pub fn still_image_cache_len(&self) -> usize {
        self.frame_provider.still_image_cache_len()
    }

    pub fn with_decoder<D: VideoFrameDecoder>(
        config: VideoRuntimeConfig,
        decoder: D,
    ) -> VideoPreviewRenderer<DecoderBackedFrameProvider<D>> {
        VideoPreviewRenderer::with_frame_provider(config, DecoderBackedFrameProvider::new(decoder))
    }
}

impl<D: VideoFrameDecoder> VideoPreviewRenderer<DecoderBackedFrameProvider<D>> {
    pub fn warm_first_composition_decode_queue(
        &mut self,
        snapshot: &VideoSnapshot,
        width: u32,
        height: u32,
        max_requests: usize,
    ) -> Result<VideoDecodeWarmupReport, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        self.sync_decode_layers(snapshot);
        let mut scheduler = self.decode_scheduler_for_snapshot(snapshot);
        let enqueue = scheduler
            .push_composition_preview(
                snapshot,
                None,
                width,
                height,
                self.frame_provider.prefetch_count(),
                self.frame_provider.prefetch_interval_ms(),
                self.frame_provider.bpm(),
            )
            .map_err(VideoPreviewError::Runtime)?;
        let decode = self.decode_scheduled_into_runtime(&mut scheduler, max_requests);
        Ok(VideoDecodeWarmupReport { enqueue, decode })
    }

    pub fn warm_output_decode_queue(
        &mut self,
        snapshot: &VideoSnapshot,
        output_id: VideoOutputId,
        width: u32,
        height: u32,
        max_requests: usize,
    ) -> Result<VideoDecodeWarmupReport, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        self.sync_decode_layers(snapshot);
        let mut scheduler = self.decode_scheduler_for_snapshot(snapshot);
        let enqueue = scheduler
            .push_output_preview(
                snapshot,
                output_id,
                width,
                height,
                self.frame_provider.prefetch_count(),
                self.frame_provider.prefetch_interval_ms(),
                self.frame_provider.bpm(),
            )
            .map_err(VideoPreviewError::Output)?;
        let decode = self.decode_scheduled_into_runtime(&mut scheduler, max_requests);
        Ok(VideoDecodeWarmupReport { enqueue, decode })
    }

    fn decode_scheduler_for_snapshot(&self, snapshot: &VideoSnapshot) -> VideoDecodeScheduler {
        let capacity = snapshot.layers.len().max(1).saturating_mul(
            self.frame_provider
                .prefetch_count()
                .saturating_add(1)
                .max(1),
        );
        VideoDecodeScheduler::new(capacity.max(1))
    }

    fn sync_decode_layers(&mut self, snapshot: &VideoSnapshot) {
        let layer_ids = snapshot
            .layers
            .iter()
            .map(|layer| layer.id)
            .collect::<Vec<_>>();
        self.frame_provider.retain_layers(&layer_ids);
        self.runtime.sync_layers(&layer_ids);
    }

    fn decode_scheduled_into_runtime(
        &mut self,
        scheduler: &mut VideoDecodeScheduler,
        max_requests: usize,
    ) -> VideoDecodeWorkerReport {
        let layer_ids = self
            .runtime
            .queues
            .iter()
            .map(FrameQueue::layer_id)
            .collect::<Vec<_>>();
        self.frame_provider.retain_layers(&layer_ids);
        let mut report = VideoDecodeWorkerReport {
            attempted: 0,
            decoded: 0,
            skipped: 0,
            pending: scheduler.len(),
            errors: Vec::new(),
        };
        for _ in 0..max_requests {
            let Some(scheduled) = scheduler.pop_next() else {
                break;
            };
            let request = scheduled.request;
            let priority = scheduled.priority;
            report.attempted += 1;
            match self.frame_provider.decoder_mut().decode_frame(&request) {
                Ok(Some(mut frame)) => {
                    frame.layer_id = request.layer_id;
                    self.runtime.push_frame(frame);
                    report.decoded += 1;
                }
                Ok(None) => report.skipped += 1,
                Err(error) => {
                    report.skipped += 1;
                    report.errors.push(VideoDecodeWorkerError {
                        input_key: scheduled.input_key,
                        layer_id: request.layer_id,
                        label: request.label,
                        position_ms: request.position_ms,
                        priority,
                        error,
                    });
                }
            }
        }
        report.pending = scheduler.len();
        report
    }
}

impl<P: VideoFrameProvider> VideoPreviewRenderer<P> {
    pub fn with_frame_provider(config: VideoRuntimeConfig, frame_provider: P) -> Self {
        Self {
            runtime: VideoRuntime::new(config),
            frame_provider,
            isf_shader_cache: HashMap::new(),
            isf_shader_cache_clock: 0,
            isf_runtime: None,
            last_isf_error: None,
            last_isf_stage_errors: Vec::new(),
            last_render_input_isf_stage_errors: Vec::new(),
            last_effect_stage_faults: Vec::new(),
            output_last_valid_frames: Vec::new(),
            last_output_render_error: None,
        }
    }

    pub fn render(
        &mut self,
        snapshot: &VideoSnapshot,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }

        let plan = build_composition_plans(snapshot).into_iter().next().ok_or(
            VideoPreviewError::Runtime(VideoRuntimeError::MissingComposition),
        )?;
        let layer_ids = plan
            .layers
            .iter()
            .map(|layer| layer.layer_id)
            .collect::<Vec<_>>();
        self.prepare_frames(snapshot, &layer_ids, width, height)?;
        self.runtime
            .compose_plan(&plan, width, height)
            .map_err(VideoPreviewError::Runtime)
    }

    /// Renders an explicit runtime input mix without changing the authored
    /// composition schema. Callers allocate `input_id`s per live source and
    /// advance `project_render_epoch` whenever the mounted project changes.
    pub fn render_input_mix(
        &mut self,
        inputs: &[VideoRenderInput],
        mix: &CompositionInputMix,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        validate_unique_render_input_keys(inputs.iter().map(|input| &input.key))
            .map_err(VideoRuntimeError::from)
            .map_err(VideoPreviewError::Runtime)?;
        self.frame_provider.retain_inputs(inputs);
        self.runtime.sync_inputs(inputs);
        self.reset_isf_stack_metrics();
        let mut keyed_isf_errors = Vec::new();
        for input in inputs {
            let frame = self
                .frame_provider
                .frame_for_input(input)
                .map_err(|error| VideoPreviewError::RenderInput {
                    key: input.key,
                    error,
                })?;
            let layer = VideoLayerSummary {
                id: input.request.layer_id,
                label: input.request.label.clone(),
                source: input.request.source.clone(),
                media_asset_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    position_ms: input.request.position_ms,
                    ..VideoLayerState::default()
                },
                isf_effect: input.isf_effect.clone(),
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
            };
            let (frame, errors) = self.apply_isf_effect_to_frame(&layer, frame);
            keyed_isf_errors.extend(errors.into_iter().map(|error| {
                VideoRenderInputIsfStageError {
                    key: input.key,
                    error,
                }
            }));
            self.runtime.push_input_frame(input, frame);
        }
        self.last_isf_stage_errors = keyed_isf_errors
            .iter()
            .map(|error| error.error.clone())
            .collect();
        self.last_isf_error = (!keyed_isf_errors.is_empty()).then(|| {
            keyed_isf_errors
                .iter()
                .map(|error| {
                    format!(
                        "render input {:?}: {}",
                        error.key,
                        format_video_isf_stage_error(&error.error, None)
                    )
                })
                .collect::<Vec<_>>()
                .join("; ")
        });
        self.last_render_input_isf_stage_errors = keyed_isf_errors;
        self.runtime
            .compose_input_mix(mix, width, height)
            .map_err(VideoPreviewError::Runtime)
    }

    /// C1 counterpart to `render_input_mix`. Runtime ownership remains the C0
    /// key, while authored Clip and Layer effects retain their independent
    /// stable identities in any reported fault.
    pub fn render_input_mix_with_effects(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        inputs: &[VideoRenderInput],
        mix: &CompositionInputMix,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        validate_unique_render_input_keys(inputs.iter().map(|input| &input.key))
            .map_err(VideoRuntimeError::from)
            .map_err(VideoPreviewError::Runtime)?;
        self.frame_provider.retain_inputs(inputs);
        self.runtime.sync_inputs(inputs);
        self.reset_isf_stack_metrics();
        let mut faults = Vec::new();

        for input in inputs {
            let frame = self
                .frame_provider
                .frame_for_input(input)
                .map_err(|error| VideoPreviewError::RenderInput {
                    key: input.key,
                    error,
                })?;
            let active_slot_id = context
                .clip_runtime
                .layers
                .iter()
                .find(|runtime| runtime.layer_id == input.layer_id())
                .and_then(|runtime| runtime.active_slot_id);
            let executes_active_clip = active_slot_id.is_some()
                && input
                    .clip_slot_id
                    .map(|slot_id| Some(slot_id) == active_slot_id)
                    .unwrap_or(true);
            let (frame, clip_faults) = if executes_active_clip {
                let scope = VideoEffectScope::Clip {
                    layer_id: input.layer_id(),
                    slot_id: active_slot_id.expect("active Clip slot was checked"),
                };
                match resolve_video_effect_chain(snapshot, &scope) {
                    Ok(Some(chain)) => {
                        self.apply_resolved_effect_chain_to_frame(&chain, frame, Some(input.key))
                    }
                    Ok(None) => (frame, Vec::new()),
                    Err(fault) => (
                        frame,
                        vec![VideoEffectStageFault {
                            render_input_key: Some(input.key),
                            ..fault
                        }],
                    ),
                }
            } else {
                (frame, Vec::new())
            };
            faults.extend(clip_faults);

            let layer = snapshot
                .layers
                .iter()
                .find(|layer| layer.id == input.layer_id());
            let layer_chain = match layer {
                Some(layer) => resolve_effective_layer_chain(snapshot, layer),
                None => input
                    .isf_effect
                    .as_ref()
                    .map(|effect| resolved_legacy_layer_chain(input.layer_id(), effect))
                    .transpose(),
            };
            let (frame, layer_faults) = match layer_chain {
                Ok(Some(chain)) => {
                    self.apply_resolved_effect_chain_to_frame(&chain, frame, Some(input.key))
                }
                Ok(None) => (frame, Vec::new()),
                Err(fault) => (
                    frame,
                    vec![VideoEffectStageFault {
                        render_input_key: Some(input.key),
                        ..fault
                    }],
                ),
            };
            faults.extend(layer_faults);
            self.runtime.push_input_frame(input, frame);
        }

        let keyed_isf_errors = faults
            .iter()
            .filter_map(|fault| {
                let key = fault.render_input_key?;
                let layer_id = match fault.scope {
                    VideoEffectScope::Clip { layer_id, .. }
                    | VideoEffectScope::Layer { layer_id } => layer_id,
                    _ => return None,
                };
                Some(VideoRenderInputIsfStageError {
                    key,
                    error: VideoIsfStageError {
                        layer_id,
                        stage_index: fault.stage_index,
                        stage_label: fault.stage_label.clone(),
                        message: fault.message.clone(),
                    },
                })
            })
            .collect::<Vec<_>>();
        self.record_effect_faults(faults, snapshot);
        self.last_render_input_isf_stage_errors = keyed_isf_errors;
        self.runtime
            .compose_input_mix(mix, width, height)
            .map_err(VideoPreviewError::Runtime)
    }

    /// Renders one layer independently from its live enable, opacity, solo, master, and
    /// blackout state. This is intended for media-bin and clip-grid thumbnails, where an
    /// operator must be able to identify a source before taking it live.
    pub fn render_layer_preview(
        &mut self,
        snapshot: &VideoSnapshot,
        layer_id: VideoLayerId,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        let layer = snapshot
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .ok_or(VideoPreviewError::MissingLayer { layer_id })?;
        let state = sanitize_layer_state(layer.state.clone());
        let position_ms = if state.position_ms > 0 {
            state.position_ms
        } else {
            state.loop_start_ms
        };
        let plan = CompositionPlan {
            composition_id: 0,
            label: format!("{} Thumbnail", layer.label),
            output_ids: Vec::new(),
            master_opacity: 1.0,
            blackout: false,
            layers: vec![CompositionLayerPlan {
                layer_id,
                label: layer.label.clone(),
                source: layer.source.clone(),
                blend_mode: VideoBlendMode::Normal,
                opacity: 1.0,
                position_ms,
                transform: state.transform,
                color: state.color,
                fx: state.fx,
            }],
        };
        self.prepare_frames(snapshot, &[layer_id], width, height)?;
        self.runtime
            .compose_plan(&plan, width, height)
            .map_err(VideoPreviewError::Runtime)
    }

    pub fn render_output(
        &mut self,
        snapshot: &VideoSnapshot,
        output_id: VideoOutputId,
    ) -> Result<VideoFrame, VideoPreviewError> {
        let plan = build_video_output_render_plan(snapshot, output_id)
            .map_err(VideoPreviewError::Output)?;
        self.render_output_plan(snapshot, &plan, plan.width, plan.height)
    }

    pub fn render_output_preview(
        &mut self,
        snapshot: &VideoSnapshot,
        output_id: VideoOutputId,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        let plan = build_video_output_render_plan(snapshot, output_id)
            .map_err(VideoPreviewError::Output)?;
        self.render_output_plan(snapshot, &plan, width, height)
    }

    /// C1 CPU correctness path. Existing callers remain source-compatible;
    /// production owners can opt into Clip scope by passing runtime slot truth.
    pub fn render_output_with_effects(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        output_id: VideoOutputId,
    ) -> Result<VideoFrame, VideoPreviewError> {
        let plan = build_video_output_render_plan(snapshot, output_id)
            .map_err(VideoPreviewError::Output)?;
        self.render_output_plan_with_effects(
            snapshot,
            context,
            None,
            &plan,
            plan.width,
            plan.height,
        )
    }

    /// Additive acknowledged-output seam. Unlike the legacy frame-only API,
    /// callers can distinguish a fresh render from a matching last-valid
    /// fallback before settling a Follow/output transition.
    pub fn render_output_with_effects_evidenced(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        output_id: VideoOutputId,
    ) -> Result<VideoOutputRenderResult, VideoPreviewError> {
        let plan = build_video_output_render_plan(snapshot, output_id)
            .map_err(VideoPreviewError::Output)?;
        self.render_output_plan_with_effects_evidenced(
            snapshot,
            context,
            None,
            &plan,
            plan.width,
            plan.height,
        )
    }

    pub fn render_output_with_effects_and_transitions(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transition_runtime: &VideoLayerTransitionRuntimeSnapshot,
        output_id: VideoOutputId,
    ) -> Result<VideoFrame, VideoPreviewError> {
        let mut plan = build_video_output_render_plan(snapshot, output_id)
            .map_err(VideoPreviewError::Output)?;
        apply_video_layer_transition_weights(snapshot, transition_runtime, &mut plan.composition)?;
        self.render_output_plan_with_effects(
            snapshot,
            context,
            Some(transition_runtime),
            &plan,
            plan.width,
            plan.height,
        )
    }

    /// Evidenced variant of the existing transition-bus output path.
    pub fn render_output_with_effects_and_transitions_evidenced(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transition_runtime: &VideoLayerTransitionRuntimeSnapshot,
        output_id: VideoOutputId,
    ) -> Result<VideoOutputRenderResult, VideoPreviewError> {
        let mut plan = build_video_output_render_plan(snapshot, output_id)
            .map_err(VideoPreviewError::Output)?;
        apply_video_layer_transition_weights(snapshot, transition_runtime, &mut plan.composition)?;
        self.render_output_plan_with_effects_evidenced(
            snapshot,
            context,
            Some(transition_runtime),
            &plan,
            plan.width,
            plan.height,
        )
    }

    /// C1 counterpart to `render_output_preview`. The complete scoped artistic
    /// chain and output mapping are evaluated at the requested presentation
    /// size while cache identity remains fenced by the caller's project epoch.
    pub fn render_output_preview_with_effects(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        output_id: VideoOutputId,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        let plan = build_video_output_render_plan(snapshot, output_id)
            .map_err(VideoPreviewError::Output)?;
        self.render_output_plan_with_effects(snapshot, context, None, &plan, width, height)
    }

    /// Evidenced preview rendering for recording/monitor owners that render
    /// at an explicit presentation size.
    pub fn render_output_preview_with_effects_evidenced(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        output_id: VideoOutputId,
        width: u32,
        height: u32,
    ) -> Result<VideoOutputRenderResult, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        let plan = build_video_output_render_plan(snapshot, output_id)
            .map_err(VideoPreviewError::Output)?;
        self.render_output_plan_with_effects_evidenced(
            snapshot, context, None, &plan, width, height,
        )
    }

    pub fn render_output_preview_with_effects_and_transitions(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transition_runtime: &VideoLayerTransitionRuntimeSnapshot,
        output_id: VideoOutputId,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        let mut plan = build_video_output_render_plan(snapshot, output_id)
            .map_err(VideoPreviewError::Output)?;
        apply_video_layer_transition_weights(snapshot, transition_runtime, &mut plan.composition)?;
        self.render_output_plan_with_effects(
            snapshot,
            context,
            Some(transition_runtime),
            &plan,
            width,
            height,
        )
    }

    /// Evidenced preview counterpart of the existing transition-bus path.
    pub fn render_output_preview_with_effects_and_transitions_evidenced(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transition_runtime: &VideoLayerTransitionRuntimeSnapshot,
        output_id: VideoOutputId,
        width: u32,
        height: u32,
    ) -> Result<VideoOutputRenderResult, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        let mut plan = build_video_output_render_plan(snapshot, output_id)
            .map_err(VideoPreviewError::Output)?;
        apply_video_layer_transition_weights(snapshot, transition_runtime, &mut plan.composition)?;
        self.render_output_plan_with_effects_evidenced(
            snapshot,
            context,
            Some(transition_runtime),
            &plan,
            width,
            height,
        )
    }

    /// Follow/output transition seam for a pair of already-composed program
    /// frames. `source_timeline_id` binds an optional resolved chain to its
    /// exact `TimelineFollow` owner. Every accepted chain runs once after the
    /// canonical base combine, for every take kind; in particular `Custom`
    /// means neutral crossfade followed by its Transition chain. A stage fault
    /// is returned as non-fresh evidence, never a fresh-settlement ACK.
    ///
    /// `last_valid` is an optional caller-owned fallback. It is used only for
    /// a chain fault and is structurally checked against the new combined
    /// frame, so this seam cannot silently present a mismatched cache entry.
    pub fn render_follow_output_transition_rgba8(
        &mut self,
        outgoing: &VideoFrame,
        incoming: &VideoFrame,
        kind: protocol::VideoClipTakeKind,
        curve: VideoLayerTransitionCurve,
        progress_millis: u16,
        source_timeline_id: TimelineId,
        transition_chain: Option<&ResolvedVideoEffectChain>,
        last_valid: Option<&VideoFrame>,
    ) -> Result<VideoFollowOutputTransitionRenderResult, CpuCompositeError> {
        if let Some(chain) = transition_chain {
            match &chain.scope {
                VideoEffectScope::Transition { owner } => {
                    let expected_owner =
                        VideoTransitionEffectOwner::TimelineFollow { source_timeline_id };
                    if *owner != expected_owner {
                        return Err(CpuCompositeError::InvalidFollowTransitionEffectOwner {
                            expected_source_timeline_id: source_timeline_id,
                            owner: owner.clone(),
                        });
                    }
                }
                _ => {
                    return Err(CpuCompositeError::InvalidTransitionEffectScope {
                        scope: chain.scope.clone(),
                    });
                }
            }
        }
        let base = combine_follow_output_transition_rgba8(
            outgoing,
            incoming,
            kind,
            curve,
            progress_millis,
        )?;
        let fallback = last_valid
            .map(validated_follow_output_transition_frame)
            .transpose()?;
        if let Some(fallback) = fallback.as_ref() {
            if fallback.width != base.frame.width
                || fallback.height != base.frame.height
                || fallback.data.len() != base.frame.data.len()
            {
                return Err(CpuCompositeError::FrameSizeMismatch {
                    layer_id: base.frame.layer_id,
                });
            }
        }
        let (rendered, stage_faults) = match transition_chain {
            Some(chain) => {
                self.apply_resolved_effect_chain_to_frame(chain, base.frame.clone(), None)
            }
            None => (base.frame.clone(), Vec::new()),
        };
        let error = (!stage_faults.is_empty()).then(|| {
            stage_faults
                .iter()
                .map(|fault| match fault.stage_label.as_deref() {
                    Some(label) => format!("{label}: {}", fault.message),
                    None => fault.message.clone(),
                })
                .collect::<Vec<_>>()
                .join("; ")
        });
        let (frame, freshness) = if error.is_none() {
            (rendered, VideoOutputRenderFreshness::Fresh)
        } else if let Some(fallback) = fallback {
            (fallback, VideoOutputRenderFreshness::LastValid)
        } else {
            (rendered, VideoOutputRenderFreshness::Error)
        };
        Ok(VideoFollowOutputTransitionRenderResult {
            frame,
            evidence: VideoFollowOutputTransitionRenderEvidence {
                transition: base.evidence,
                freshness,
                error,
                stage_faults,
            },
        })
    }

    fn render_output_plan_with_effects(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transition_runtime: Option<&VideoLayerTransitionRuntimeSnapshot>,
        plan: &VideoOutputRenderPlan,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        Ok(self
            .render_output_plan_with_effects_evidenced(
                snapshot,
                context,
                transition_runtime,
                plan,
                width,
                height,
            )?
            .frame)
    }

    fn render_output_plan_with_effects_evidenced(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transition_runtime: Option<&VideoLayerTransitionRuntimeSnapshot>,
        plan: &VideoOutputRenderPlan,
        width: u32,
        height: u32,
    ) -> Result<VideoOutputRenderResult, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        if plan.output_blackout || plan.output_opacity <= f32::EPSILON || plan.composition.blackout
        {
            return Ok(VideoOutputRenderResult {
                frame: self.blackout_output_artistic_frame(snapshot, plan.output_id, width, height),
                evidence: VideoOutputRenderEvidence {
                    project_render_epoch: context.project_render_epoch,
                    output_id: plan.output_id,
                    freshness: VideoOutputRenderFreshness::Fresh,
                    error: None,
                },
            });
        }
        let key = VideoOutputLastValidKey {
            project_render_epoch: context.project_render_epoch,
            output_id: plan.output_id,
            width,
            height,
            composition_id: plan.composition.composition_id,
            kind: plan.kind.clone(),
            fullscreen: plan.fullscreen,
            monitor_id: plan.monitor_id,
            endpoint_name: plan.endpoint_name.clone(),
        };
        self.output_last_valid_frames.retain(|entry| {
            entry.key.project_render_epoch == context.project_render_epoch
                && (entry.key.output_id != plan.output_id || entry.key == key)
        });
        match self.prepare_output_artistic_frame_with_effects_impl(
            snapshot,
            context,
            transition_runtime,
            plan,
            width,
            height,
        ) {
            Ok(frame) => {
                self.last_output_render_error = None;
                if let Some(entry) = self
                    .output_last_valid_frames
                    .iter_mut()
                    .find(|entry| entry.key == key)
                {
                    entry.frame = frame.clone();
                } else {
                    self.output_last_valid_frames
                        .push(VideoOutputLastValidFrame {
                            key,
                            frame: frame.clone(),
                        });
                }
                Ok(VideoOutputRenderResult {
                    frame: apply_video_output_mapping(frame, &plan.mapping),
                    evidence: VideoOutputRenderEvidence {
                        project_render_epoch: context.project_render_epoch,
                        output_id: plan.output_id,
                        freshness: VideoOutputRenderFreshness::Fresh,
                        error: None,
                    },
                })
            }
            Err(error) => {
                let error = format!("{error:?}");
                self.last_output_render_error = Some(error.clone());
                let fallback = self
                    .output_last_valid_frames
                    .iter()
                    .find(|entry| entry.key == key)
                    .map(|entry| entry.frame.clone());
                let freshness = if fallback.is_some() {
                    VideoOutputRenderFreshness::LastValid
                } else {
                    VideoOutputRenderFreshness::Error
                };
                let frame =
                    fallback.unwrap_or_else(|| video_output_transparent_black_frame(width, height));
                Ok(VideoOutputRenderResult {
                    frame: apply_video_output_mapping(frame, &plan.mapping),
                    evidence: VideoOutputRenderEvidence {
                        project_render_epoch: context.project_render_epoch,
                        output_id: plan.output_id,
                        freshness,
                        error: Some(error),
                    },
                })
            }
        }
    }

    /// Produces the artistic post-chain/pre-mapping frame consumed by the C1
    /// last-valid cache. Native GPU presentation can adopt this additive seam
    /// without changing its mapping pass.
    pub fn prepare_output_artistic_frame_with_effects(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        plan: &VideoOutputRenderPlan,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        self.prepare_output_artistic_frame_with_effects_impl(
            snapshot, context, None, plan, width, height,
        )
    }

    pub fn prepare_output_artistic_frame_with_effects_and_transitions(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transition_runtime: &VideoLayerTransitionRuntimeSnapshot,
        plan: &VideoOutputRenderPlan,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        let mut plan = plan.clone();
        apply_video_layer_transition_weights(snapshot, transition_runtime, &mut plan.composition)?;
        self.prepare_output_artistic_frame_with_effects_impl(
            snapshot,
            context,
            Some(transition_runtime),
            &plan,
            width,
            height,
        )
    }

    fn prepare_output_artistic_frame_with_effects_impl(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transition_runtime: Option<&VideoLayerTransitionRuntimeSnapshot>,
        plan: &VideoOutputRenderPlan,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        if plan.output_blackout || plan.output_opacity <= f32::EPSILON || plan.composition.blackout
        {
            return Ok(self.blackout_output_artistic_frame(
                snapshot,
                plan.output_id,
                width,
                height,
            ));
        }
        // Every valid invocation replaces diagnostics, including a fatal
        // decode/composite error.
        self.clear_full_output_effect_diagnostics(snapshot);
        let layer_ids = plan
            .composition
            .layers
            .iter()
            .map(|layer| layer.layer_id)
            .collect::<Vec<_>>();
        let mut faults = self.prepare_frames_with_effects(
            snapshot,
            context.clip_runtime,
            context.project_render_epoch,
            &layer_ids,
            width,
            height,
        )?;
        let (mut frame, composition_faults) = self
            .compose_scoped_plan(
                snapshot,
                transition_runtime,
                &plan.composition,
                width,
                height,
            )
            .map_err(VideoPreviewError::Runtime)?;
        faults.extend(composition_faults);
        let output_scope = VideoEffectScope::Output {
            output_id: plan.output_id,
        };
        match resolve_video_effect_chain(snapshot, &output_scope) {
            Ok(Some(chain)) => {
                let (rendered, output_faults) =
                    self.apply_resolved_effect_chain_to_frame(&chain, frame, None);
                frame = rendered;
                faults.extend(output_faults);
            }
            Ok(None) => {}
            Err(fault) => faults.push(fault),
        }
        let mut ordered_faults = Vec::with_capacity(faults.len());
        for fault in faults {
            if !ordered_faults.contains(&fault) {
                ordered_faults.push(fault);
            }
        }
        self.record_effect_faults(ordered_faults, snapshot);
        Ok(frame)
    }

    fn blackout_output_artistic_frame(
        &mut self,
        snapshot: &VideoSnapshot,
        output_id: VideoOutputId,
        width: u32,
        height: u32,
    ) -> VideoFrame {
        // A blackout is a hard artistic fence. Discard the previous image so
        // a failed first frame after recovery cannot leak pre-blackout content
        // through the last-valid fallback. Keep both public rendering seams
        // behaviorally identical.
        self.output_last_valid_frames
            .retain(|entry| entry.key.output_id != output_id);
        self.last_output_render_error = None;
        self.clear_full_output_effect_diagnostics(snapshot);
        video_output_black_frame(width, height)
    }

    fn compose_scoped_plan(
        &mut self,
        snapshot: &VideoSnapshot,
        transition_runtime: Option<&VideoLayerTransitionRuntimeSnapshot>,
        plan: &CompositionPlan,
        width: u32,
        height: u32,
    ) -> Result<(VideoFrame, Vec<VideoEffectStageFault>), VideoRuntimeError> {
        let frames = self.runtime.select_frames_for_plan(plan);
        let mut output = vec![0u8; width as usize * height as usize * 4];
        let mut faults = Vec::new();
        let active_buses = transition_runtime
            .into_iter()
            .flat_map(|runtime| runtime.buses.iter())
            .filter_map(|active| {
                snapshot
                    .transition_buses
                    .iter()
                    .find(|bus| {
                        bus.id == active.bus_id && bus.composition_id == plan.composition_id
                    })
                    .map(|bus| {
                        let member_ids = bus
                            .members
                            .iter()
                            .flat_map(|target| {
                                video_layer_transition_target_ids(snapshot, target)
                                    .expect("validated transition target must resolve")
                            })
                            .collect::<Vec<_>>();
                        (active, member_ids)
                    })
            })
            .collect::<Vec<_>>();
        let mut active_groups = Vec::new();
        for group in snapshot
            .layer_groups
            .iter()
            .filter(|group| group.composition_id == plan.composition_id)
        {
            let scope = VideoEffectScope::Group { group_id: group.id };
            match resolve_video_effect_chain(snapshot, &scope) {
                Ok(Some(chain)) if resolved_chain_executes(&chain) => {
                    active_groups.push((group.layer_ids.clone(), chain));
                }
                Ok(_) => {}
                Err(fault) => faults.push(fault),
            }
        }

        for layer in &plan.layers {
            if let Some((active, member_ids)) = active_buses
                .iter()
                .find(|(_, member_ids)| member_ids.contains(&layer.layer_id))
            {
                let first_visible_member = plan
                    .layers
                    .iter()
                    .find(|candidate| member_ids.contains(&candidate.layer_id))
                    .map(|candidate| candidate.layer_id);
                if first_visible_member != Some(layer.layer_id) {
                    continue;
                }
                let bus_layers = plan
                    .layers
                    .iter()
                    .filter(|candidate| member_ids.contains(&candidate.layer_id))
                    .cloned()
                    .collect::<Vec<_>>();
                let bus_plan = CompositionPlan {
                    composition_id: plan.composition_id,
                    label: format!("{} Transition Bus {}", plan.label, active.bus_id.0),
                    output_ids: Vec::new(),
                    master_opacity: plan.master_opacity,
                    blackout: plan.blackout,
                    layers: bus_layers,
                };
                let (bus_data, bus_group_faults) = self
                    .compose_plan_layers_with_groups(snapshot, &bus_plan, &frames, width, height)?;
                faults.extend(bus_group_faults);
                let bus_frame = VideoFrame {
                    layer_id: 0,
                    width,
                    height,
                    pts_ms: frames.iter().map(|frame| frame.pts_ms).max().unwrap_or(0),
                    duration_ms: 0,
                    format: VideoPixelFormat::Rgba8,
                    data: bus_data,
                };
                let scope = VideoEffectScope::Transition {
                    owner: VideoTransitionEffectOwner::LayerBus {
                        bus_id: active.bus_id,
                    },
                };
                let bus_frame = match resolve_video_effect_chain(snapshot, &scope) {
                    Ok(Some(chain)) => {
                        let (rendered, bus_faults) =
                            self.apply_resolved_effect_chain_to_frame(&chain, bus_frame, None);
                        faults.extend(bus_faults);
                        rendered
                    }
                    Ok(None) => bus_frame,
                    Err(fault) => {
                        faults.push(fault);
                        bus_frame
                    }
                };
                for (destination, source) in output
                    .chunks_exact_mut(4)
                    .zip(bus_frame.data.chunks_exact(4))
                {
                    blend_pixel(destination, source, 1.0, &VideoBlendMode::Normal);
                }
                continue;
            }
            let group = active_groups
                .iter()
                .find(|(member_ids, _)| member_ids.contains(&layer.layer_id));
            let Some((member_ids, chain)) = group else {
                blend_composition_layer_onto(&mut output, layer, &frames, width, height)?;
                continue;
            };
            let first_visible_member = plan
                .layers
                .iter()
                .find(|candidate| member_ids.contains(&candidate.layer_id))
                .map(|candidate| candidate.layer_id);
            if first_visible_member != Some(layer.layer_id) {
                continue;
            }
            let group_layers = plan
                .layers
                .iter()
                .filter(|candidate| member_ids.contains(&candidate.layer_id))
                .cloned()
                .collect::<Vec<_>>();
            let group_plan = CompositionPlan {
                composition_id: plan.composition_id,
                label: format!("{} Group", plan.label),
                output_ids: Vec::new(),
                master_opacity: plan.master_opacity,
                blackout: plan.blackout,
                layers: group_layers,
            };
            let group_frame = composite_rgba8(&group_plan, &frames, width, height)?;
            let (group_frame, group_faults) =
                self.apply_resolved_effect_chain_to_frame(chain, group_frame, None);
            faults.extend(group_faults);
            for (destination, source) in output
                .chunks_exact_mut(4)
                .zip(group_frame.data.chunks_exact(4))
            {
                blend_pixel(destination, source, 1.0, &VideoBlendMode::Normal);
            }
        }

        let mut frame = VideoFrame {
            layer_id: 0,
            width,
            height,
            pts_ms: frames.iter().map(|frame| frame.pts_ms).max().unwrap_or(0),
            duration_ms: 0,
            format: VideoPixelFormat::Rgba8,
            data: output,
        };
        let scope = VideoEffectScope::Composition {
            composition_id: plan.composition_id,
        };
        match resolve_video_effect_chain(snapshot, &scope) {
            Ok(Some(chain)) => {
                let (rendered, composition_faults) =
                    self.apply_resolved_effect_chain_to_frame(&chain, frame, None);
                frame = rendered;
                faults.extend(composition_faults);
            }
            Ok(None) => {}
            Err(fault) => faults.push(fault),
        }
        Ok((frame, faults))
    }

    fn compose_plan_layers_with_groups(
        &mut self,
        snapshot: &VideoSnapshot,
        plan: &CompositionPlan,
        frames: &[VideoFrame],
        width: u32,
        height: u32,
    ) -> Result<(Vec<u8>, Vec<VideoEffectStageFault>), VideoRuntimeError> {
        let mut output = vec![0u8; width as usize * height as usize * 4];
        let mut faults = Vec::new();
        let mut active_groups = Vec::new();
        for group in snapshot
            .layer_groups
            .iter()
            .filter(|group| group.composition_id == plan.composition_id)
        {
            let scope = VideoEffectScope::Group { group_id: group.id };
            match resolve_video_effect_chain(snapshot, &scope) {
                Ok(Some(chain)) if resolved_chain_executes(&chain) => {
                    active_groups.push((group.layer_ids.clone(), chain));
                }
                Ok(_) => {}
                Err(fault) => faults.push(fault),
            }
        }
        for layer in &plan.layers {
            let group = active_groups
                .iter()
                .find(|(member_ids, _)| member_ids.contains(&layer.layer_id));
            let Some((member_ids, chain)) = group else {
                blend_composition_layer_onto(&mut output, layer, frames, width, height)?;
                continue;
            };
            let first_visible_member = plan
                .layers
                .iter()
                .find(|candidate| member_ids.contains(&candidate.layer_id))
                .map(|candidate| candidate.layer_id);
            if first_visible_member != Some(layer.layer_id) {
                continue;
            }
            let group_layers = plan
                .layers
                .iter()
                .filter(|candidate| member_ids.contains(&candidate.layer_id))
                .cloned()
                .collect::<Vec<_>>();
            let group_plan = CompositionPlan {
                composition_id: plan.composition_id,
                label: format!("{} Group", plan.label),
                output_ids: Vec::new(),
                master_opacity: plan.master_opacity,
                blackout: plan.blackout,
                layers: group_layers,
            };
            let group_frame = composite_rgba8(&group_plan, frames, width, height)?;
            let (group_frame, group_faults) =
                self.apply_resolved_effect_chain_to_frame(chain, group_frame, None);
            faults.extend(group_faults);
            for (destination, source) in output
                .chunks_exact_mut(4)
                .zip(group_frame.data.chunks_exact(4))
            {
                blend_pixel(destination, source, 1.0, &VideoBlendMode::Normal);
            }
        }
        Ok((output, faults))
    }

    pub fn prepare_output_frames(
        &mut self,
        snapshot: &VideoSnapshot,
        output_id: VideoOutputId,
        width: u32,
        height: u32,
    ) -> Result<PreparedVideoOutput, VideoPreviewError> {
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        let plan = build_video_output_render_plan(snapshot, output_id)
            .map_err(VideoPreviewError::Output)?;
        if plan.output_blackout
            || plan.output_opacity <= f32::EPSILON
            || plan.composition.layers.is_empty()
        {
            return Ok(PreparedVideoOutput {
                plan,
                frames: Vec::new(),
            });
        }
        let layer_ids = plan
            .composition
            .layers
            .iter()
            .map(|layer| layer.layer_id)
            .collect::<Vec<_>>();
        self.prepare_frames(snapshot, &layer_ids, width, height)?;
        let frames = self.runtime.select_frames_for_plan(&plan.composition);
        Ok(PreparedVideoOutput { plan, frames })
    }

    fn render_output_plan(
        &mut self,
        snapshot: &VideoSnapshot,
        plan: &VideoOutputRenderPlan,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoPreviewError> {
        if plan.output_blackout || plan.output_opacity <= f32::EPSILON {
            return Ok(video_output_black_frame(width, height));
        }
        let layer_ids = plan
            .composition
            .layers
            .iter()
            .map(|layer| layer.layer_id)
            .collect::<Vec<_>>();
        if layer_ids.is_empty() {
            return Ok(video_output_black_frame(width, height));
        }
        self.prepare_frames(snapshot, &layer_ids, width, height)?;
        let frame = self
            .runtime
            .compose_plan(&plan.composition, width, height)
            .map_err(VideoPreviewError::Runtime)?;
        Ok(apply_video_output_mapping(frame, &plan.mapping))
    }

    fn prepare_frames(
        &mut self,
        snapshot: &VideoSnapshot,
        requested_layer_ids: &[VideoLayerId],
        width: u32,
        height: u32,
    ) -> Result<(), VideoPreviewError> {
        let clip_runtime = VideoClipRuntimeSnapshot::default();
        let faults = self.prepare_frames_with_effects(
            snapshot,
            &clip_runtime,
            0,
            requested_layer_ids,
            width,
            height,
        )?;
        self.record_effect_faults(faults, snapshot);
        Ok(())
    }

    fn prepare_frames_with_effects(
        &mut self,
        snapshot: &VideoSnapshot,
        clip_runtime: &VideoClipRuntimeSnapshot,
        project_render_epoch: u64,
        requested_layer_ids: &[VideoLayerId],
        width: u32,
        height: u32,
    ) -> Result<Vec<VideoEffectStageFault>, VideoPreviewError> {
        let layer_ids = snapshot
            .layers
            .iter()
            .map(|layer| layer.id)
            .collect::<Vec<_>>();
        let mut retained_inputs = Vec::new();
        for layer in &snapshot.layers {
            if !requested_layer_ids.contains(&layer.id) {
                continue;
            }
            let runtime_layer = clip_runtime
                .layers
                .iter()
                .find(|runtime| runtime.layer_id == layer.id);
            if let Some(transition) = runtime_layer.and_then(|runtime| runtime.transition.as_ref())
            {
                for (slot_id, position_ms) in [
                    (
                        transition.outgoing_slot_id,
                        runtime_layer
                            .map(|runtime| runtime.playhead_ms)
                            .unwrap_or(0),
                    ),
                    (transition.incoming_slot_id, transition.incoming_playhead_ms),
                ] {
                    retained_inputs.push(video_render_input_for_clip_slot(
                        snapshot,
                        layer,
                        slot_id,
                        position_ms,
                        project_render_epoch,
                        width,
                        height,
                    )?);
                }
            } else {
                retained_inputs.push(VideoRenderInput::legacy(VideoFrameRequest {
                    layer_id: layer.id,
                    label: layer.label.clone(),
                    source: layer.source.clone(),
                    position_ms: layer.state.position_ms,
                    width,
                    height,
                }));
            }
        }
        self.frame_provider.retain_inputs(&retained_inputs);
        self.runtime.sync_layers(&layer_ids);
        self.reset_isf_stack_metrics();
        let mut effect_faults = Vec::new();

        for layer in &snapshot.layers {
            if !requested_layer_ids.contains(&layer.id) {
                continue;
            }
            let runtime_layer = clip_runtime
                .layers
                .iter()
                .find(|runtime| runtime.layer_id == layer.id);
            if let Some(transition) = runtime_layer.and_then(|runtime| runtime.transition.as_ref())
            {
                let outgoing = video_render_input_for_clip_slot(
                    snapshot,
                    layer,
                    transition.outgoing_slot_id,
                    runtime_layer
                        .map(|runtime| runtime.playhead_ms)
                        .unwrap_or(0),
                    project_render_epoch,
                    width,
                    height,
                )?;
                let incoming = video_render_input_for_clip_slot(
                    snapshot,
                    layer,
                    transition.incoming_slot_id,
                    transition.incoming_playhead_ms,
                    project_render_epoch,
                    width,
                    height,
                )?;
                let outgoing_frame =
                    self.frame_provider
                        .frame_for_input(&outgoing)
                        .map_err(|error| VideoPreviewError::RenderInput {
                            key: outgoing.key,
                            error,
                        })?;
                let incoming_frame =
                    self.frame_provider
                        .frame_for_input(&incoming)
                        .map_err(|error| VideoPreviewError::RenderInput {
                            key: incoming.key,
                            error,
                        })?;
                let (outgoing_frame, outgoing_faults) = self.apply_clip_chain_to_frame(
                    snapshot,
                    layer.id,
                    transition.outgoing_slot_id,
                    outgoing_frame,
                    Some(outgoing.key),
                );
                let (incoming_frame, incoming_faults) = self.apply_clip_chain_to_frame(
                    snapshot,
                    layer.id,
                    transition.incoming_slot_id,
                    incoming_frame,
                    Some(incoming.key),
                );
                effect_faults.extend(outgoing_faults);
                effect_faults.extend(incoming_faults);
                let (base_outgoing, base_incoming, base_progress) =
                    if transition.origin_slot_id == transition.outgoing_slot_id {
                        (&outgoing_frame, &incoming_frame, transition.progress_millis)
                    } else {
                        (
                            &incoming_frame,
                            &outgoing_frame,
                            1000u16.saturating_sub(transition.progress_millis),
                        )
                    };
                let frame = transition_video_frames_rgba8(
                    base_outgoing,
                    base_incoming,
                    transition.kind,
                    base_progress,
                )
                .map_err(VideoRuntimeError::from)
                .map_err(VideoPreviewError::Runtime)?;
                let transition_scope = VideoEffectScope::Transition {
                    owner: protocol::VideoTransitionEffectOwner::ClipTake { layer_id: layer.id },
                };
                let (frame, transition_faults) =
                    match resolve_video_effect_chain(snapshot, &transition_scope) {
                        Ok(Some(chain)) => {
                            self.apply_resolved_effect_chain_to_frame(&chain, frame, None)
                        }
                        Ok(None) => (frame, Vec::new()),
                        Err(fault) => (frame, vec![fault]),
                    };
                effect_faults.extend(transition_faults);
                let (frame, layer_faults) = match resolve_effective_layer_chain(snapshot, layer) {
                    Ok(Some(chain)) => {
                        self.apply_resolved_effect_chain_to_frame(&chain, frame, None)
                    }
                    Ok(None) => (frame, Vec::new()),
                    Err(fault) => (frame, vec![fault]),
                };
                effect_faults.extend(layer_faults);
                self.runtime.push_frame(frame);
                continue;
            }
            let frames = self
                .frame_provider
                .frames_for_layer(layer, width, height)
                .map_err(VideoPreviewError::from)?;
            for frame in frames {
                let active_slot_id = clip_runtime
                    .layers
                    .iter()
                    .find(|runtime| runtime.layer_id == layer.id)
                    .and_then(|runtime| runtime.active_slot_id);
                let (frame, clip_faults) = match active_slot_id {
                    Some(slot_id) => {
                        let scope = VideoEffectScope::Clip {
                            layer_id: layer.id,
                            slot_id,
                        };
                        match resolve_video_effect_chain(snapshot, &scope) {
                            Ok(Some(chain)) => {
                                self.apply_resolved_effect_chain_to_frame(&chain, frame, None)
                            }
                            Ok(None) => (frame, Vec::new()),
                            Err(fault) => (frame, vec![fault]),
                        }
                    }
                    None => (frame, Vec::new()),
                };
                for fault in clip_faults {
                    if !effect_faults.contains(&fault) {
                        effect_faults.push(fault);
                    }
                }
                let (frame, layer_faults) = match resolve_effective_layer_chain(snapshot, layer) {
                    Ok(Some(chain)) => {
                        self.apply_resolved_effect_chain_to_frame(&chain, frame, None)
                    }
                    Ok(None) => (frame, Vec::new()),
                    Err(fault) => (frame, vec![fault]),
                };
                for fault in layer_faults {
                    if !effect_faults.contains(&fault) {
                        effect_faults.push(fault);
                    }
                }
                self.runtime.push_frame(frame);
            }
        }
        self.last_render_input_isf_stage_errors.clear();
        Ok(effect_faults)
    }

    fn apply_clip_chain_to_frame(
        &mut self,
        snapshot: &VideoSnapshot,
        layer_id: VideoLayerId,
        slot_id: VideoClipSlotId,
        frame: VideoFrame,
        key: Option<protocol::VideoRenderInputKey>,
    ) -> (VideoFrame, Vec<VideoEffectStageFault>) {
        let scope = VideoEffectScope::Clip { layer_id, slot_id };
        match resolve_video_effect_chain(snapshot, &scope) {
            Ok(Some(chain)) => self.apply_resolved_effect_chain_to_frame(&chain, frame, key),
            Ok(None) => (frame, Vec::new()),
            Err(fault) => (
                frame,
                vec![VideoEffectStageFault {
                    render_input_key: key,
                    ..fault
                }],
            ),
        }
    }

    fn cached_isf_shader(
        &mut self,
        source: &Arc<str>,
    ) -> Result<Arc<PreparedIsfShader>, IsfPrepareError> {
        self.isf_shader_cache_clock = self.isf_shader_cache_clock.wrapping_add(1);
        let last_used = self.isf_shader_cache_clock;
        if let Some(entry) = self.isf_shader_cache.get_mut(source.as_ref()) {
            entry.last_used = last_used;
            return entry.value.clone();
        }
        if self.isf_shader_cache.len() >= ISF_SHADER_CACHE_CAPACITY {
            if let Some(oldest) = self
                .isf_shader_cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(source, _)| Arc::clone(source))
            {
                self.isf_shader_cache.remove(oldest.as_ref());
            }
        }
        let value = prepare_isf_shader(source.as_ref()).map(Arc::new);
        self.isf_shader_cache.insert(
            Arc::clone(source),
            CachedIsfShader {
                value: value.clone(),
                last_used,
            },
        );
        value
    }

    fn apply_resolved_effect_chain_to_frame(
        &mut self,
        chain: &ResolvedVideoEffectChain,
        frame: VideoFrame,
        render_input_key: Option<protocol::VideoRenderInputKey>,
    ) -> (VideoFrame, Vec<VideoEffectStageFault>) {
        if !resolved_chain_executes(chain) {
            return (frame, Vec::new());
        }
        let mut prepared_stages = Vec::new();
        let mut faults = Vec::new();
        for (stage_index, stage) in chain.stages.iter().enumerate() {
            if !stage.enabled {
                continue;
            }
            match self.cached_isf_shader(&stage.effect.source) {
                Ok(shader) => {
                    let controls =
                        resolved_isf_control_values_from_controls(&shader, &stage.effect.controls);
                    prepared_stages.push((stage_index, shader, controls));
                }
                Err(error) => faults.push(video_effect_fault(
                    chain.scope.clone(),
                    chain.chain_id,
                    stage.stage_id,
                    stage.effect_id,
                    render_input_key,
                    Some(stage_index),
                    Some(stage.label.clone()),
                    error.to_string(),
                )),
            }
        }
        if self.isf_runtime.is_none() {
            self.isf_runtime = Some(IsfGpuRuntime::new());
        }
        let runtime = match self.isf_runtime.as_mut().expect("ISF runtime initialized") {
            Ok(runtime) => runtime,
            Err(error) => {
                for (stage_index, _, _) in &prepared_stages {
                    let stage = &chain.stages[*stage_index];
                    faults.push(video_effect_fault(
                        chain.scope.clone(),
                        chain.chain_id,
                        stage.stage_id,
                        stage.effect_id,
                        render_input_key,
                        Some(*stage_index),
                        Some(stage.label.clone()),
                        error.to_string(),
                    ));
                }
                return (frame, faults);
            }
        };
        if prepared_stages.is_empty() {
            runtime.advance_stage_local_chain_without_prepared_stages();
            return (frame, faults);
        }
        let stage_refs = prepared_stages
            .iter()
            .map(|(_, shader, controls)| (shader.as_ref(), controls.as_slice()))
            .collect::<Vec<_>>();
        let (rendered, runtime_failures) = runtime.apply_stack_stage_local(
            &frame,
            &stage_refs,
            frame.pts_ms as f32 / 1_000.0,
            frame.duration_ms as f32 / 1_000.0,
        );
        for (prepared_index, error) in runtime_failures {
            let stage_index = prepared_stages[prepared_index].0;
            let stage = &chain.stages[stage_index];
            faults.push(video_effect_fault(
                chain.scope.clone(),
                chain.chain_id,
                stage.stage_id,
                stage.effect_id,
                render_input_key,
                Some(stage_index),
                Some(stage.label.clone()),
                error.to_string(),
            ));
        }
        (rendered, faults)
    }

    fn apply_isf_effect_to_frame(
        &mut self,
        layer: &VideoLayerSummary,
        frame: VideoFrame,
    ) -> (VideoFrame, Vec<VideoIsfStageError>) {
        let Some(effect) = layer.isf_effect.as_ref() else {
            return (frame, Vec::new());
        };
        let scope = VideoEffectScope::Layer { layer_id: layer.id };
        let chain = ResolvedVideoEffectChain {
            chain_id: None,
            scope,
            bypassed: false,
            stages: legacy_isf_stages(effect)
                .into_iter()
                .map(|effect| ResolvedVideoEffectStage {
                    stage_id: None,
                    effect_id: None,
                    enabled: effect.enabled,
                    label: effect.label.clone(),
                    effect,
                })
                .collect(),
        };
        let (frame, faults) = self.apply_resolved_effect_chain_to_frame(&chain, frame, None);
        let errors = faults
            .into_iter()
            .map(|fault| VideoIsfStageError {
                layer_id: layer.id,
                stage_index: fault.stage_index,
                stage_label: fault.stage_label,
                message: fault.message,
            })
            .collect();
        (frame, errors)
    }

    fn record_effect_faults(
        &mut self,
        faults: Vec<VideoEffectStageFault>,
        snapshot: &VideoSnapshot,
    ) {
        self.last_isf_stage_errors = faults
            .iter()
            .filter_map(|fault| {
                let layer_id = match fault.scope {
                    VideoEffectScope::Layer { layer_id }
                    | VideoEffectScope::Clip { layer_id, .. } => layer_id,
                    _ => return None,
                };
                Some(VideoIsfStageError {
                    layer_id,
                    stage_index: fault.stage_index,
                    stage_label: fault.stage_label.clone(),
                    message: fault.message.clone(),
                })
            })
            .collect();
        self.last_isf_error = (!faults.is_empty()).then(|| {
            faults
                .iter()
                .map(|fault| {
                    let owner = match fault.scope {
                        VideoEffectScope::Layer { layer_id }
                        | VideoEffectScope::Clip { layer_id, .. } => snapshot
                            .layers
                            .iter()
                            .find(|layer| layer.id == layer_id)
                            .map(|layer| layer.label.as_str()),
                        _ => None,
                    };
                    let legacy = VideoIsfStageError {
                        layer_id: match fault.scope {
                            VideoEffectScope::Layer { layer_id }
                            | VideoEffectScope::Clip { layer_id, .. } => layer_id,
                            _ => 0,
                        },
                        stage_index: fault.stage_index,
                        stage_label: fault.stage_label.clone(),
                        message: fault.message.clone(),
                    };
                    if owner.is_some() {
                        format_video_isf_stage_error(&legacy, owner)
                    } else {
                        format!("{:?}: {}", fault.scope, fault.message)
                    }
                })
                .collect::<Vec<_>>()
                .join("; ")
        });
        self.last_effect_stage_faults = faults;
    }

    fn clear_full_output_effect_diagnostics(&mut self, snapshot: &VideoSnapshot) {
        self.last_render_input_isf_stage_errors.clear();
        self.record_effect_faults(Vec::new(), snapshot);
    }

    pub fn isf_pipeline_count(&self) -> usize {
        self.isf_runtime
            .as_ref()
            .and_then(|runtime| runtime.as_ref().ok())
            .map(IsfGpuRuntime::pipeline_count)
            .unwrap_or(0)
    }

    pub fn isf_last_stack_stage_count(&self) -> usize {
        self.isf_runtime
            .as_ref()
            .and_then(|runtime| runtime.as_ref().ok())
            .map(IsfGpuRuntime::last_stack_stage_count)
            .unwrap_or(0)
    }

    pub fn isf_last_stack_render_us(&self) -> u64 {
        self.isf_runtime
            .as_ref()
            .and_then(|runtime| runtime.as_ref().ok())
            .map(IsfGpuRuntime::last_stack_render_us)
            .unwrap_or(0)
    }

    fn reset_isf_stack_metrics(&mut self) {
        if let Some(Ok(runtime)) = self.isf_runtime.as_mut() {
            runtime.reset_last_stack_metrics();
        }
    }

    pub fn isf_shader_cache_count(&self) -> usize {
        self.isf_shader_cache.len()
    }

    pub fn last_isf_error(&self) -> Option<&str> {
        self.last_isf_error.as_deref()
    }

    pub fn last_isf_stage_errors(&self) -> &[VideoIsfStageError] {
        &self.last_isf_stage_errors
    }

    pub fn last_render_input_isf_stage_errors(&self) -> &[VideoRenderInputIsfStageError] {
        &self.last_render_input_isf_stage_errors
    }

    pub fn last_effect_stage_faults(&self) -> &[VideoEffectStageFault] {
        &self.last_effect_stage_faults
    }

    pub fn last_output_render_error(&self) -> Option<&str> {
        self.last_output_render_error.as_deref()
    }

    pub fn queue_count(&self) -> usize {
        self.runtime.queue_count()
    }

    pub fn queue_len(&self, layer_id: VideoLayerId) -> usize {
        self.runtime.queue_len(layer_id)
    }

    pub fn config(&self) -> VideoRuntimeConfig {
        self.runtime.config()
    }

    pub fn frame_provider(&self) -> &P {
        &self.frame_provider
    }

    pub fn frame_provider_mut(&mut self) -> &mut P {
        &mut self.frame_provider
    }
}

fn video_render_input_for_clip_slot(
    snapshot: &VideoSnapshot,
    layer: &VideoLayerSummary,
    slot_id: VideoClipSlotId,
    position_ms: u64,
    project_render_epoch: u64,
    width: u32,
    height: u32,
) -> Result<VideoRenderInput, VideoPreviewError> {
    let slot = layer
        .clip_slots
        .iter()
        .find(|slot| slot.id == slot_id)
        .ok_or(VideoPreviewError::MissingTransitionSlot {
            layer_id: layer.id,
            slot_id,
        })?;
    let asset = snapshot
        .media_assets
        .iter()
        .find(|asset| asset.id == slot.media_asset_id)
        .ok_or(VideoPreviewError::MissingTransitionAsset {
            layer_id: layer.id,
            asset_id: slot.media_asset_id,
        })?;
    Ok(VideoRenderInput::new(
        protocol::VideoRenderInputKey {
            project_render_epoch,
            input_id: protocol::VideoRenderInputId(slot_id.0),
        },
        VideoFrameRequest {
            layer_id: layer.id,
            label: asset.label.clone(),
            source: asset.source.clone(),
            position_ms,
            width,
            height,
        },
    )
    .with_clip_slot_id(Some(slot_id)))
}

fn crossfade_video_frames_rgba8(
    outgoing: &VideoFrame,
    incoming: &VideoFrame,
    progress_millis: u16,
) -> Result<VideoFrame, CpuCompositeError> {
    let outgoing = convert_frame_to_rgba8(outgoing)?;
    let incoming = convert_frame_to_rgba8(incoming)?;
    if outgoing.width != incoming.width
        || outgoing.height != incoming.height
        || outgoing.data.len() != incoming.data.len()
    {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: outgoing.layer_id,
        });
    }
    let incoming_weight = u32::from(progress_millis.min(1000));
    let outgoing_weight = 1000 - incoming_weight;
    let data = outgoing
        .data
        .iter()
        .zip(&incoming.data)
        .map(|(outgoing, incoming)| {
            ((u32::from(*outgoing) * outgoing_weight
                + u32::from(*incoming) * incoming_weight
                + 500)
                / 1000) as u8
        })
        .collect();
    Ok(VideoFrame {
        layer_id: outgoing.layer_id,
        width: outgoing.width,
        height: outgoing.height,
        pts_ms: outgoing.pts_ms.max(incoming.pts_ms),
        duration_ms: outgoing.duration_ms.min(incoming.duration_ms),
        format: VideoPixelFormat::Rgba8,
        data,
    })
}

fn transition_video_frames_rgba8(
    outgoing: &VideoFrame,
    incoming: &VideoFrame,
    kind: protocol::VideoClipTakeKind,
    progress_millis: u16,
) -> Result<VideoFrame, CpuCompositeError> {
    match kind {
        protocol::VideoClipTakeKind::Cut => Ok(convert_frame_to_rgba8(incoming)?),
        protocol::VideoClipTakeKind::Crossfade | protocol::VideoClipTakeKind::Custom => {
            crossfade_video_frames_rgba8(outgoing, incoming, progress_millis)
        }
        protocol::VideoClipTakeKind::Dip => {
            dip_video_frames_rgba8(outgoing, incoming, progress_millis)
        }
        protocol::VideoClipTakeKind::Wipe => {
            wipe_video_frames_rgba8(outgoing, incoming, progress_millis, false)
        }
        protocol::VideoClipTakeKind::Luma => {
            wipe_video_frames_rgba8(outgoing, incoming, progress_millis, true)
        }
        protocol::VideoClipTakeKind::Displacement => {
            displaced_video_frames_rgba8(outgoing, incoming, progress_millis)
        }
        protocol::VideoClipTakeKind::Blur => {
            blurred_video_frames_rgba8(outgoing, incoming, progress_millis)
        }
        protocol::VideoClipTakeKind::Glitch => {
            glitch_video_frames_rgba8(outgoing, incoming, progress_millis)
        }
    }
}

/// Returns the exact integer transition progress after applying the authored
/// output curve once. This is public so the output owner can report the same
/// evidence that the CPU transition dispatcher consumed.
pub fn video_follow_output_transition_progress_millis(
    curve: VideoLayerTransitionCurve,
    progress_millis: u16,
) -> u16 {
    (video_layer_transition_curve_progress(curve, progress_millis) * 1000.0)
        .round()
        .clamp(0.0, 1000.0) as u16
}

/// Combine two fully composed program frames for Timeline Follow/output
/// transition presentation. It validates both sources structurally before
/// dispatch, including `Cut`, while preserving transparent program frames as
/// normal source-only or target-only output. `Custom` deliberately uses the neutral
/// crossfade base; scoped Transition effects remain the caller's next chain.
pub fn combine_follow_output_transition_rgba8(
    outgoing: &VideoFrame,
    incoming: &VideoFrame,
    kind: protocol::VideoClipTakeKind,
    curve: VideoLayerTransitionCurve,
    progress_millis: u16,
) -> Result<VideoFollowOutputTransitionResult, CpuCompositeError> {
    if progress_millis > 1000 {
        return Err(CpuCompositeError::InvalidTransitionProgress { progress_millis });
    }
    let outgoing = validated_follow_output_transition_frame(outgoing)?;
    let incoming = validated_follow_output_transition_frame(incoming)?;
    if outgoing.width != incoming.width
        || outgoing.height != incoming.height
        || outgoing.data.len() != incoming.data.len()
    {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: outgoing.layer_id,
        });
    }
    let curved_progress_millis =
        video_follow_output_transition_progress_millis(curve, progress_millis);
    let frame = transition_video_frames_rgba8(&outgoing, &incoming, kind, curved_progress_millis)?;
    Ok(VideoFollowOutputTransitionResult {
        frame,
        evidence: VideoFollowOutputTransitionEvidence {
            kind,
            curve,
            raw_progress_millis: progress_millis,
            curved_progress_millis,
        },
    })
}

fn validated_follow_output_transition_frame(
    frame: &VideoFrame,
) -> Result<VideoFrame, CpuCompositeError> {
    let frame = convert_frame_to_rgba8(frame)?;
    if frame.width == 0 || frame.height == 0 || frame.data.is_empty() {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    }
    Ok(frame)
}

fn compatible_rgba8_transition_frames(
    outgoing: &VideoFrame,
    incoming: &VideoFrame,
) -> Result<(VideoFrame, VideoFrame), CpuCompositeError> {
    let outgoing = convert_frame_to_rgba8(outgoing)?;
    let incoming = convert_frame_to_rgba8(incoming)?;
    if outgoing.width != incoming.width
        || outgoing.height != incoming.height
        || outgoing.data.len() != incoming.data.len()
    {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: outgoing.layer_id,
        });
    }
    Ok((outgoing, incoming))
}

fn transition_frame_like(
    outgoing: &VideoFrame,
    incoming: &VideoFrame,
    data: Vec<u8>,
) -> VideoFrame {
    VideoFrame {
        layer_id: outgoing.layer_id,
        width: outgoing.width,
        height: outgoing.height,
        pts_ms: outgoing.pts_ms.max(incoming.pts_ms),
        duration_ms: outgoing.duration_ms.min(incoming.duration_ms),
        format: VideoPixelFormat::Rgba8,
        data,
    }
}

fn dip_video_frames_rgba8(
    outgoing: &VideoFrame,
    incoming: &VideoFrame,
    progress_millis: u16,
) -> Result<VideoFrame, CpuCompositeError> {
    let (outgoing, incoming) = compatible_rgba8_transition_frames(outgoing, incoming)?;
    let progress = u32::from(progress_millis.min(1000));
    let (source, source_weight) = if progress <= 500 {
        (&outgoing, 1000 - progress * 2)
    } else {
        (&incoming, (progress - 500) * 2)
    };
    let black_weight = 1000 - source_weight;
    let data = source
        .data
        .chunks_exact(4)
        .flat_map(|pixel| {
            [
                ((u32::from(pixel[0]) * source_weight + 500) / 1000) as u8,
                ((u32::from(pixel[1]) * source_weight + 500) / 1000) as u8,
                ((u32::from(pixel[2]) * source_weight + 500) / 1000) as u8,
                ((u32::from(pixel[3]) * source_weight + 255 * black_weight + 500) / 1000) as u8,
            ]
        })
        .collect();
    Ok(transition_frame_like(&outgoing, &incoming, data))
}

fn wipe_video_frames_rgba8(
    outgoing: &VideoFrame,
    incoming: &VideoFrame,
    progress_millis: u16,
    luminance_ordered: bool,
) -> Result<VideoFrame, CpuCompositeError> {
    let (outgoing, incoming) = compatible_rgba8_transition_frames(outgoing, incoming)?;
    let progress = u32::from(progress_millis.min(1000));
    if progress == 0 {
        return Ok(outgoing);
    }
    if progress == 1000 {
        return Ok(incoming);
    }
    let width = outgoing.width.max(1);
    let data = outgoing
        .data
        .chunks_exact(4)
        .zip(incoming.data.chunks_exact(4))
        .enumerate()
        .flat_map(|(index, (out_pixel, in_pixel))| {
            let threshold = if luminance_ordered {
                (u32::from(in_pixel[0]) * 2126
                    + u32::from(in_pixel[1]) * 7152
                    + u32::from(in_pixel[2]) * 722)
                    / 2550
            } else {
                ((index as u32 % width) + 1) * 1000 / width
            };
            let selected = if threshold <= progress {
                in_pixel
            } else {
                out_pixel
            };
            [selected[0], selected[1], selected[2], selected[3]]
        })
        .collect();
    Ok(transition_frame_like(&outgoing, &incoming, data))
}

fn shifted_frame_data(frame: &VideoFrame, shift: i32) -> Vec<u8> {
    let width = frame.width.max(1) as i32;
    let height = frame.height as usize;
    let mut data = vec![0; frame.data.len()];
    for y in 0..height {
        for x in 0..width as usize {
            let source_x = (x as i32 - shift).rem_euclid(width) as usize;
            let from = (y * width as usize + source_x) * 4;
            let to = (y * width as usize + x) * 4;
            data[to..to + 4].copy_from_slice(&frame.data[from..from + 4]);
        }
    }
    data
}

fn displaced_video_frames_rgba8(
    outgoing: &VideoFrame,
    incoming: &VideoFrame,
    progress_millis: u16,
) -> Result<VideoFrame, CpuCompositeError> {
    let (mut outgoing, mut incoming) = compatible_rgba8_transition_frames(outgoing, incoming)?;
    let progress = i32::from(progress_millis.min(1000));
    let span = outgoing.width.min(64) as i32;
    outgoing.data = shifted_frame_data(&outgoing, -(span * progress / 1000));
    incoming.data = shifted_frame_data(&incoming, span * (1000 - progress) / 1000);
    crossfade_video_frames_rgba8(&outgoing, &incoming, progress_millis)
}

fn box_blur_rgba8(frame: &VideoFrame, radius: usize) -> Vec<u8> {
    if radius == 0 || frame.width == 0 || frame.height == 0 {
        return frame.data.clone();
    }
    let width = frame.width as usize;
    let height = frame.height as usize;
    let mut data = vec![0; frame.data.len()];
    for y in 0..height {
        for x in 0..width {
            for channel in 0..4 {
                let mut sum = 0u32;
                let mut count = 0u32;
                for sample_x in x.saturating_sub(radius)..=(x + radius).min(width - 1) {
                    sum += u32::from(frame.data[(y * width + sample_x) * 4 + channel]);
                    count += 1;
                }
                data[(y * width + x) * 4 + channel] = (sum / count) as u8;
            }
        }
    }
    data
}

fn blurred_video_frames_rgba8(
    outgoing: &VideoFrame,
    incoming: &VideoFrame,
    progress_millis: u16,
) -> Result<VideoFrame, CpuCompositeError> {
    let (mut outgoing, mut incoming) = compatible_rgba8_transition_frames(outgoing, incoming)?;
    let progress = u32::from(progress_millis.min(1000));
    outgoing.data = box_blur_rgba8(&outgoing, (progress * 6 / 1000) as usize);
    incoming.data = box_blur_rgba8(&incoming, ((1000 - progress) * 6 / 1000) as usize);
    crossfade_video_frames_rgba8(&outgoing, &incoming, progress_millis)
}

fn glitch_video_frames_rgba8(
    outgoing: &VideoFrame,
    incoming: &VideoFrame,
    progress_millis: u16,
) -> Result<VideoFrame, CpuCompositeError> {
    let (outgoing, mut incoming) = compatible_rgba8_transition_frames(outgoing, incoming)?;
    let progress = u32::from(progress_millis.min(1000));
    let width = incoming.width.max(1) as usize;
    let height = incoming.height as usize;
    let mut data = incoming.data.clone();
    for y in 0..height {
        let band = ((y as u32 * 17 + progress / 25) % 7) as i32 - 3;
        let shift =
            band * ((1000 - (progress as i32 - 500).unsigned_abs() * 2) as i32).max(0) / 250;
        for x in 0..width {
            let source_x = (x as i32 - shift).rem_euclid(width as i32) as usize;
            let from = (y * width + source_x) * 4;
            let to = (y * width + x) * 4;
            data[to..to + 4].copy_from_slice(&incoming.data[from..from + 4]);
        }
    }
    incoming.data = data;
    crossfade_video_frames_rgba8(&outgoing, &incoming, progress_millis)
}

impl StillImageFrameCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.entries
            .retain(|entry| layer_ids.iter().any(|layer_id| *layer_id == entry.layer_id));
    }

    pub fn retain_inputs(&mut self, inputs: &[VideoRenderInput]) {
        self.entries
            .retain(|entry| inputs.iter().any(|input| input.key == entry.key));
    }

    pub fn frame_for_layer(
        &mut self,
        layer_id: VideoLayerId,
        path: impl AsRef<Path>,
        pts_ms: u64,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, StillImageError> {
        self.frame_for_input(
            protocol::VideoRenderInputKey::LEGACY,
            layer_id,
            path,
            pts_ms,
            width,
            height,
        )
    }

    pub fn frame_for_input(
        &mut self,
        key: protocol::VideoRenderInputKey,
        layer_id: VideoLayerId,
        path: impl AsRef<Path>,
        pts_ms: u64,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, StillImageError> {
        let path = path.as_ref().to_path_buf();
        let signature = StillImageSignature::from_path(&path);
        if let Some(entry) = self.entries.iter().find(|entry| {
            entry.key == key
                && entry.layer_id == layer_id
                && entry.path == path
                && entry.width == width
                && entry.height == height
                && entry.signature == signature
        }) {
            let mut frame = entry.frame.clone();
            frame.pts_ms = pts_ms;
            return Ok(frame);
        }

        let loaded = load_still_image_frame(layer_id, &path, pts_ms)?;
        let frame = resize_rgba8_nearest(&loaded, width, height)?;
        self.entries.retain(|entry| {
            !(entry.key == key
                && entry.layer_id == layer_id
                && entry.path == path
                && entry.width == width
                && entry.height == height)
        });
        self.entries.push(StillImageFrameCacheEntry {
            key,
            layer_id,
            path,
            width,
            height,
            signature,
            frame: frame.clone(),
        });
        Ok(frame)
    }
}

impl StillImageSignature {
    fn from_path(path: &Path) -> Self {
        let metadata = fs::metadata(path).ok();
        Self {
            len: metadata.as_ref().map(|metadata| metadata.len()),
            modified: metadata.and_then(|metadata| metadata.modified().ok()),
        }
    }
}

fn video_output_black_frame(width: u32, height: u32) -> VideoFrame {
    VideoFrame {
        layer_id: 0,
        width,
        height,
        pts_ms: 0,
        duration_ms: 0,
        format: VideoPixelFormat::Rgba8,
        data: [0, 0, 0, 255].repeat(width as usize * height as usize),
    }
}

fn video_output_transparent_black_frame(width: u32, height: u32) -> VideoFrame {
    VideoFrame {
        layer_id: 0,
        width,
        height,
        pts_ms: 0,
        duration_ms: 0,
        format: VideoPixelFormat::Rgba8,
        data: [0, 0, 0, 0].repeat(width as usize * height as usize),
    }
}

pub fn composite_rgba8(
    plan: &CompositionPlan,
    frames: &[VideoFrame],
    width: u32,
    height: u32,
) -> Result<VideoFrame, CpuCompositeError> {
    if width == 0 || height == 0 {
        return Err(CpuCompositeError::InvalidOutputSize);
    }
    let mut output = vec![0u8; width as usize * height as usize * 4];

    for layer in &plan.layers {
        blend_composition_layer_onto(&mut output, layer, frames, width, height)?;
    }

    Ok(VideoFrame {
        layer_id: 0,
        width,
        height,
        pts_ms: plan
            .layers
            .iter()
            .filter_map(|layer| {
                frames
                    .iter()
                    .filter(|frame| frame.layer_id == layer.layer_id)
                    .map(|frame| frame.pts_ms)
                    .max()
            })
            .max()
            .unwrap_or(0),
        duration_ms: 0,
        format: VideoPixelFormat::Rgba8,
        data: output,
    })
}

fn blend_composition_layer_onto(
    output: &mut [u8],
    layer: &CompositionLayerPlan,
    frames: &[VideoFrame],
    width: u32,
    height: u32,
) -> Result<(), CpuCompositeError> {
    let frame = frames
        .iter()
        .filter(|frame| frame.layer_id == layer.layer_id)
        .max_by_key(|frame| frame.pts_ms)
        .ok_or(CpuCompositeError::MissingFrame {
            layer_id: layer.layer_id,
        })?;
    let normalized_frame;
    let frame = if frame.format == VideoPixelFormat::Rgba8 {
        validate_rgba_frame_size(frame)?;
        frame
    } else {
        normalized_frame = convert_frame_to_rgba8(frame)?;
        &normalized_frame
    };
    blend_transformed_rgba8(
        output,
        frame,
        width,
        height,
        layer.opacity.clamp(0.0, 1.0),
        &layer.blend_mode,
        &layer.transform,
        &layer.color,
        &layer.fx,
    );
    Ok(())
}

/// Mixes explicitly keyed runtime inputs. A repeated `layer_id` is valid: the
/// input key is the selector, so two source instances at the same PTS neither
/// deduplicate nor replace one another.
pub fn composite_input_mix_rgba8(
    mix: &CompositionInputMix,
    frames: &[VideoRenderInputFrame],
    width: u32,
    height: u32,
) -> Result<VideoFrame, CpuCompositeError> {
    if width == 0 || height == 0 {
        return Err(CpuCompositeError::InvalidOutputSize);
    }
    validate_unique_render_input_keys(mix.inputs.iter().map(|input| &input.key))?;
    let mut output = vec![0u8; width as usize * height as usize * 4];

    for input in &mix.inputs {
        let frame = frames
            .iter()
            .filter(|frame| frame.key == input.key)
            .max_by_key(|frame| frame.frame.pts_ms)
            .map(|frame| &frame.frame)
            .ok_or(CpuCompositeError::MissingRenderInputFrame {
                key: input.key,
                layer_id: input.layer.layer_id,
            })?;
        let normalized_frame;
        let frame = if frame.format == VideoPixelFormat::Rgba8 {
            validate_render_input_rgba_frame_size(input.key, frame)?;
            frame
        } else {
            normalized_frame = convert_render_input_frame_to_rgba8(input.key, frame)?;
            &normalized_frame
        };
        blend_transformed_rgba8(
            &mut output,
            frame,
            width,
            height,
            input.layer.opacity.clamp(0.0, 1.0),
            &input.layer.blend_mode,
            &input.layer.transform,
            &input.layer.color,
            &input.layer.fx,
        );
    }

    Ok(VideoFrame {
        layer_id: 0,
        width,
        height,
        pts_ms: frames
            .iter()
            .filter(|frame| mix.inputs.iter().any(|input| input.key == frame.key))
            .map(|frame| frame.frame.pts_ms)
            .max()
            .unwrap_or(0),
        duration_ms: 0,
        format: VideoPixelFormat::Rgba8,
        data: output,
    })
}

fn validate_render_input_rgba_frame_size(
    key: protocol::VideoRenderInputKey,
    frame: &VideoFrame,
) -> Result<(), CpuCompositeError> {
    let Some(expected_len) = rgba_frame_len(frame.width, frame.height) else {
        return Err(CpuCompositeError::RenderInputFrameSizeMismatch {
            key,
            layer_id: frame.layer_id,
        });
    };
    if frame.width == 0 || frame.height == 0 || frame.data.len() != expected_len {
        return Err(CpuCompositeError::RenderInputFrameSizeMismatch {
            key,
            layer_id: frame.layer_id,
        });
    }
    Ok(())
}

fn convert_render_input_frame_to_rgba8(
    key: protocol::VideoRenderInputKey,
    frame: &VideoFrame,
) -> Result<VideoFrame, CpuCompositeError> {
    convert_frame_to_rgba8(frame).map_err(|error| match error {
        CpuCompositeError::FrameSizeMismatch { .. } => {
            CpuCompositeError::RenderInputFrameSizeMismatch {
                key,
                layer_id: frame.layer_id,
            }
        }
        CpuCompositeError::UnsupportedFrameFormat { format, .. } => {
            CpuCompositeError::UnsupportedRenderInputFrameFormat {
                key,
                layer_id: frame.layer_id,
                format,
            }
        }
        other => other,
    })
}

fn validate_rgba_frame_size(frame: &VideoFrame) -> Result<(), CpuCompositeError> {
    let Some(expected_len) = rgba_frame_len(frame.width, frame.height) else {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    };
    if frame.width == 0 || frame.height == 0 || frame.data.len() != expected_len {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    }
    Ok(())
}

fn rgba_frame_len(width: u32, height: u32) -> Option<usize> {
    (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)
}

pub(crate) fn convert_frame_to_rgba8(frame: &VideoFrame) -> Result<VideoFrame, CpuCompositeError> {
    let data = match frame.format {
        VideoPixelFormat::Rgba8 => {
            validate_rgba_frame_size(frame)?;
            frame.data.clone()
        }
        VideoPixelFormat::Bgra8 => convert_bgra8_to_rgba8(frame)?,
        VideoPixelFormat::Dxt1 => decode_dxt1_rgba8(frame)?,
        VideoPixelFormat::Dxt5 => decode_dxt5_rgba8(frame)?,
        VideoPixelFormat::YcoCgDxt5 => decode_ycocg_dxt5_rgba8(frame)?,
        VideoPixelFormat::Bc7 => decode_bc7_rgba8(frame)?,
    };
    Ok(VideoFrame {
        layer_id: frame.layer_id,
        width: frame.width,
        height: frame.height,
        pts_ms: frame.pts_ms,
        duration_ms: frame.duration_ms,
        format: VideoPixelFormat::Rgba8,
        data,
    })
}

fn convert_bgra8_to_rgba8(frame: &VideoFrame) -> Result<Vec<u8>, CpuCompositeError> {
    validate_rgba_frame_size(frame)?;
    let mut data = frame.data.clone();
    for pixel in data.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Ok(data)
}

fn dxt_block_count(width: u32, height: u32) -> Option<usize> {
    if width == 0 || height == 0 {
        return None;
    }
    let blocks_x = ((width as usize).checked_add(3)?) / 4;
    let blocks_y = ((height as usize).checked_add(3)?) / 4;
    blocks_x.checked_mul(blocks_y)
}

fn decode_dxt1_rgba8(frame: &VideoFrame) -> Result<Vec<u8>, CpuCompositeError> {
    let Some(block_count) = dxt_block_count(frame.width, frame.height) else {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    };
    let expected_len = block_count
        .checked_mul(8)
        .ok_or(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        })?;
    if frame.data.len() != expected_len {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    }
    let Some(output_len) = rgba_frame_len(frame.width, frame.height) else {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    };
    let mut output = vec![0u8; output_len];
    let blocks_x = (frame.width as usize + 3) / 4;
    for (block_index, block) in frame.data.chunks_exact(8).enumerate() {
        let block_x = block_index % blocks_x;
        let block_y = block_index / blocks_x;
        write_dxt_color_block(
            &mut output,
            frame.width,
            frame.height,
            block_x,
            block_y,
            block,
            true,
            None,
        );
    }
    Ok(output)
}

fn decode_bc7_rgba8(frame: &VideoFrame) -> Result<Vec<u8>, CpuCompositeError> {
    let Some(block_count) = dxt_block_count(frame.width, frame.height) else {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    };
    let expected_len = block_count
        .checked_mul(16)
        .ok_or(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        })?;
    if frame.data.len() != expected_len {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    }
    let Some(output_len) = rgba_frame_len(frame.width, frame.height) else {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    };
    let mut output = vec![0_u8; output_len];
    let blocks_x = frame.width.div_ceil(4) as usize;
    for (block_index, block) in frame.data.chunks_exact(16).enumerate() {
        let block_x = block_index % blocks_x;
        let block_y = block_index / blocks_x;
        let mut decoded = [0_u8; 4 * 4 * 4];
        bcdec_rs::bc7(block, &mut decoded, 4 * 4);
        for local_y in 0..4_usize {
            let y = block_y * 4 + local_y;
            if y >= frame.height as usize {
                continue;
            }
            for local_x in 0..4_usize {
                let x = block_x * 4 + local_x;
                if x >= frame.width as usize {
                    continue;
                }
                let source = (local_y * 4 + local_x) * 4;
                let target = (y * frame.width as usize + x) * 4;
                output[target..target + 4].copy_from_slice(&decoded[source..source + 4]);
            }
        }
    }
    Ok(output)
}

fn decode_dxt5_rgba8(frame: &VideoFrame) -> Result<Vec<u8>, CpuCompositeError> {
    let Some(block_count) = dxt_block_count(frame.width, frame.height) else {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    };
    let expected_len = block_count
        .checked_mul(16)
        .ok_or(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        })?;
    if frame.data.len() != expected_len {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    }
    let Some(output_len) = rgba_frame_len(frame.width, frame.height) else {
        return Err(CpuCompositeError::FrameSizeMismatch {
            layer_id: frame.layer_id,
        });
    };
    let mut output = vec![0u8; output_len];
    let blocks_x = (frame.width as usize + 3) / 4;
    for (block_index, block) in frame.data.chunks_exact(16).enumerate() {
        let block_x = block_index % blocks_x;
        let block_y = block_index / blocks_x;
        let alpha_palette = dxt5_alpha_palette(block[0], block[1]);
        let mut alpha_bits = 0u64;
        for (index, byte) in block[2..8].iter().enumerate() {
            alpha_bits |= (*byte as u64) << (index * 8);
        }
        write_dxt_color_block(
            &mut output,
            frame.width,
            frame.height,
            block_x,
            block_y,
            &block[8..16],
            false,
            Some((alpha_palette, alpha_bits)),
        );
    }
    Ok(output)
}

fn decode_ycocg_dxt5_rgba8(frame: &VideoFrame) -> Result<Vec<u8>, CpuCompositeError> {
    let mut data = decode_dxt5_rgba8(frame)?;
    for pixel in data.chunks_exact_mut(4) {
        let scale = f32::from(pixel[2]) / 8.0 + 1.0;
        let co = ((f32::from(pixel[0]) - 128.0) / 255.0) / scale;
        let cg = ((f32::from(pixel[1]) - 128.0) / 255.0) / scale;
        let y = f32::from(pixel[3]) / 255.0;
        pixel[0] = ((y + co - cg).clamp(0.0, 1.0) * 255.0).round() as u8;
        pixel[1] = ((y + cg).clamp(0.0, 1.0) * 255.0).round() as u8;
        pixel[2] = ((y - co - cg).clamp(0.0, 1.0) * 255.0).round() as u8;
        pixel[3] = 255;
    }
    Ok(data)
}

fn write_dxt_color_block(
    output: &mut [u8],
    width: u32,
    height: u32,
    block_x: usize,
    block_y: usize,
    color_block: &[u8],
    dxt1_alpha: bool,
    alpha_override: Option<([u8; 8], u64)>,
) {
    let palette = dxt_color_palette(color_block, dxt1_alpha);
    let color_indices = u32::from_le_bytes([
        color_block[4],
        color_block[5],
        color_block[6],
        color_block[7],
    ]);
    for local_y in 0..4usize {
        let y = block_y * 4 + local_y;
        if y >= height as usize {
            continue;
        }
        for local_x in 0..4usize {
            let x = block_x * 4 + local_x;
            if x >= width as usize {
                continue;
            }
            let pixel_index = local_y * 4 + local_x;
            let palette_index = ((color_indices >> (pixel_index * 2)) & 0x03) as usize;
            let mut pixel = palette[palette_index];
            if let Some((alpha_palette, alpha_bits)) = alpha_override {
                let alpha_index = ((alpha_bits >> (pixel_index * 3)) & 0x07) as usize;
                pixel[3] = alpha_palette[alpha_index];
            }
            let dst_index = (y * width as usize + x) * 4;
            output[dst_index..dst_index + 4].copy_from_slice(&pixel);
        }
    }
}

fn dxt_color_palette(color_block: &[u8], dxt1_alpha: bool) -> [[u8; 4]; 4] {
    let color0 = u16::from_le_bytes([color_block[0], color_block[1]]);
    let color1 = u16::from_le_bytes([color_block[2], color_block[3]]);
    let c0 = rgb565_to_rgba8(color0, 255);
    let c1 = rgb565_to_rgba8(color1, 255);
    if !dxt1_alpha || color0 > color1 {
        [
            c0,
            c1,
            interpolate_rgba(c0, c1, 2, 1, 3, 255),
            interpolate_rgba(c0, c1, 1, 2, 3, 255),
        ]
    } else {
        [c0, c1, interpolate_rgba(c0, c1, 1, 1, 2, 255), [0, 0, 0, 0]]
    }
}

fn rgb565_to_rgba8(value: u16, alpha: u8) -> [u8; 4] {
    let red = ((value >> 11) & 0x1f) as u8;
    let green = ((value >> 5) & 0x3f) as u8;
    let blue = (value & 0x1f) as u8;
    [
        (red << 3) | (red >> 2),
        (green << 2) | (green >> 4),
        (blue << 3) | (blue >> 2),
        alpha,
    ]
}

fn interpolate_rgba(
    a: [u8; 4],
    b: [u8; 4],
    a_weight: u16,
    b_weight: u16,
    divisor: u16,
    alpha: u8,
) -> [u8; 4] {
    [
        ((a[0] as u16 * a_weight + b[0] as u16 * b_weight) / divisor) as u8,
        ((a[1] as u16 * a_weight + b[1] as u16 * b_weight) / divisor) as u8,
        ((a[2] as u16 * a_weight + b[2] as u16 * b_weight) / divisor) as u8,
        alpha,
    ]
}

fn dxt5_alpha_palette(alpha0: u8, alpha1: u8) -> [u8; 8] {
    if alpha0 > alpha1 {
        [
            alpha0,
            alpha1,
            ((6 * alpha0 as u16 + alpha1 as u16) / 7) as u8,
            ((5 * alpha0 as u16 + 2 * alpha1 as u16) / 7) as u8,
            ((4 * alpha0 as u16 + 3 * alpha1 as u16) / 7) as u8,
            ((3 * alpha0 as u16 + 4 * alpha1 as u16) / 7) as u8,
            ((2 * alpha0 as u16 + 5 * alpha1 as u16) / 7) as u8,
            ((alpha0 as u16 + 6 * alpha1 as u16) / 7) as u8,
        ]
    } else {
        [
            alpha0,
            alpha1,
            ((4 * alpha0 as u16 + alpha1 as u16) / 5) as u8,
            ((3 * alpha0 as u16 + 2 * alpha1 as u16) / 5) as u8,
            ((2 * alpha0 as u16 + 3 * alpha1 as u16) / 5) as u8,
            ((alpha0 as u16 + 4 * alpha1 as u16) / 5) as u8,
            0,
            255,
        ]
    }
}

pub fn debug_solid_frame_for_layer(
    layer_id: VideoLayerId,
    pts_ms: u64,
    width: u32,
    height: u32,
) -> VideoFrame {
    let color = debug_color(layer_id, pts_ms);
    let mut data = Vec::with_capacity(width as usize * height as usize * 4);
    for _ in 0..(width as usize * height as usize) {
        data.extend_from_slice(&color);
    }
    VideoFrame {
        layer_id,
        width,
        height,
        pts_ms,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data,
    }
}

pub fn load_still_image_frame(
    layer_id: VideoLayerId,
    path: impl AsRef<Path>,
    pts_ms: u64,
) -> Result<VideoFrame, StillImageError> {
    let image = image::open(path)
        .map_err(|error| StillImageError::Decode(error.to_string()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Err(StillImageError::EmptyImage);
    }
    Ok(VideoFrame {
        layer_id,
        width,
        height,
        pts_ms,
        duration_ms: 0,
        format: VideoPixelFormat::Rgba8,
        data: image.into_raw(),
    })
}

pub fn probe_still_image_metadata(
    path: impl AsRef<Path>,
) -> Result<VideoMediaMetadata, StillImageError> {
    let (width, height) = image::image_dimensions(path)
        .map_err(|error| StillImageError::Decode(error.to_string()))?;
    if width == 0 || height == 0 {
        return Err(StillImageError::EmptyImage);
    }
    Ok(VideoMediaMetadata {
        duration_ms: None,
        width: Some(width),
        height: Some(height),
        frame_rate: None,
        has_audio: false,
    })
}

fn prefetch_position_ms(
    mut state: VideoLayerState,
    offset: usize,
    interval_ms: u64,
    bpm: Option<f32>,
    source_duration_ms: Option<u64>,
) -> u64 {
    if offset == 0 {
        return sanitize_layer_state(state).position_ms;
    }
    state = sanitize_layer_state(state);
    clamp_layer_state_to_source_duration(&mut state, source_duration_ms);
    state.playing = true;
    advance_sanitized_layer_state(
        state,
        Duration::from_millis(interval_ms.saturating_mul(offset as u64)),
        bpm,
        source_duration_ms,
    )
    .position_ms
}

pub fn preview_prefetch_positions_ms(
    layer: &VideoLayerSummary,
    prefetch_count: usize,
    prefetch_interval_ms: u64,
    bpm: Option<f32>,
) -> Vec<u64> {
    let state = sanitize_layer_state(layer.state.clone());
    if matches!(layer.source.kind, VideoSourceKind::StillImage) || prefetch_count == 0 {
        return vec![state.position_ms];
    }

    let source_duration_ms = layer
        .source
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.duration_ms);
    (0..=prefetch_count)
        .map(|offset| {
            prefetch_position_ms(
                state.clone(),
                offset,
                prefetch_interval_ms,
                bpm,
                source_duration_ms,
            )
        })
        .collect()
}

pub fn preview_decode_requests_for_layer(
    layer: &VideoLayerSummary,
    width: u32,
    height: u32,
    prefetch_count: usize,
    prefetch_interval_ms: u64,
    bpm: Option<f32>,
) -> Vec<VideoFrameRequest> {
    if width == 0 || height == 0 || matches!(layer.source.kind, VideoSourceKind::StillImage) {
        return Vec::new();
    }
    preview_prefetch_positions_ms(layer, prefetch_count, prefetch_interval_ms, bpm)
        .into_iter()
        .map(|position_ms| VideoFrameRequest {
            layer_id: layer.id,
            label: layer.label.clone(),
            source: layer.source.clone(),
            position_ms,
            width,
            height,
        })
        .collect()
}

pub fn resize_rgba8_nearest(
    frame: &VideoFrame,
    width: u32,
    height: u32,
) -> Result<VideoFrame, StillImageError> {
    if width == 0 || height == 0 || frame.width == 0 || frame.height == 0 {
        return Err(StillImageError::EmptyImage);
    }
    if frame.format != VideoPixelFormat::Rgba8 {
        return Err(StillImageError::UnsupportedFormat(frame.format));
    }
    let mut data = vec![0u8; width as usize * height as usize * 4];
    for y in 0..height {
        let src_y = (y as u64 * frame.height as u64 / height as u64) as u32;
        for x in 0..width {
            let src_x = (x as u64 * frame.width as u64 / width as u64) as u32;
            let src_index = ((src_y * frame.width + src_x) * 4) as usize;
            let dst_index = ((y * width + x) * 4) as usize;
            data[dst_index..dst_index + 4].copy_from_slice(&frame.data[src_index..src_index + 4]);
        }
    }
    Ok(VideoFrame {
        layer_id: frame.layer_id,
        width,
        height,
        pts_ms: frame.pts_ms,
        duration_ms: frame.duration_ms,
        format: frame.format,
        data,
    })
}

pub fn decode_ffmpeg_cli_frame(
    request: &VideoFrameRequest,
    binary: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> Result<VideoFrame, VideoDecodeError> {
    if request.width == 0 || request.height == 0 {
        return Err(VideoDecodeError::Decode {
            layer_id: request.layer_id,
            label: request.label.clone(),
            message: "requested frame size must be greater than zero".to_string(),
        });
    }

    let expected_len = request.width as usize * request.height as usize * 4;
    let seek_seconds = format!("{:.3}", request.position_ms as f64 / 1000.0);
    let scale_filter = format!(
        "scale={}:{}:flags=fast_bilinear,format=rgba",
        request.width, request.height
    );
    let output = Command::new(binary.as_ref())
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-ss",
            &seek_seconds,
            "-i",
        ])
        .arg(path.as_ref())
        .args([
            "-frames:v",
            "1",
            "-vf",
            &scale_filter,
            "-an",
            "-sn",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "pipe:1",
        ])
        .output()
        .map_err(|error| VideoDecodeError::Decode {
            layer_id: request.layer_id,
            label: request.label.clone(),
            message: format!(
                "failed to run FFmpeg binary '{}': {error}",
                binary.as_ref().display()
            ),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(VideoDecodeError::Decode {
            layer_id: request.layer_id,
            label: request.label.clone(),
            message: format!("FFmpeg decode failed: {}", stderr.trim()),
        });
    }

    if output.stdout.len() != expected_len {
        return Err(VideoDecodeError::Decode {
            layer_id: request.layer_id,
            label: request.label.clone(),
            message: format!(
                "FFmpeg returned {} bytes, expected {expected_len}",
                output.stdout.len()
            ),
        });
    }

    Ok(VideoFrame {
        layer_id: request.layer_id,
        width: request.width,
        height: request.height,
        pts_ms: request.position_ms,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: output.stdout,
    })
}

pub fn build_composition_plans(snapshot: &VideoSnapshot) -> Vec<CompositionPlan> {
    let compositions = if snapshot.compositions.is_empty() {
        vec![CompositionSummary {
            id: 1,
            label: "Main".to_string(),
            layer_ids: snapshot.layers.iter().map(|layer| layer.id).collect(),
            output_ids: snapshot.outputs.iter().map(|output| output.id).collect(),
        }]
    } else {
        snapshot.compositions.clone()
    };

    compositions
        .into_iter()
        .map(|composition| build_composition_plan(snapshot, composition))
        .collect()
}

pub fn build_video_output_render_plans(
    snapshot: &VideoSnapshot,
) -> Result<Vec<VideoOutputRenderPlan>, VideoOutputRenderError> {
    snapshot
        .outputs
        .iter()
        .map(|output| build_video_output_render_plan(snapshot, output.id))
        .collect()
}

pub fn build_external_video_input_plans(snapshot: &VideoSnapshot) -> Vec<ExternalVideoInputPlan> {
    snapshot
        .layers
        .iter()
        .filter_map(|layer| {
            let backend_id = external_video_source_backend_id(&layer.source.kind)?;
            let endpoint_name = layer
                .source
                .name
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string();
            (!endpoint_name.is_empty()).then(|| ExternalVideoInputPlan {
                layer_id: layer.id,
                label: layer.label.clone(),
                kind: layer.source.kind.clone(),
                backend_id: backend_id.to_string(),
                endpoint_name,
                enabled: layer.state.enabled,
            })
        })
        .collect()
}

pub fn build_external_video_output_plans(snapshot: &VideoSnapshot) -> Vec<ExternalVideoOutputPlan> {
    snapshot
        .outputs
        .iter()
        .filter_map(|output| {
            let backend_id = external_video_output_backend_id(&output.kind)?;
            let endpoint_name = output
                .endpoint_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(&output.label)
                .to_string();
            Some(ExternalVideoOutputPlan {
                output_id: output.id,
                label: output.label.clone(),
                kind: output.kind.clone(),
                backend_id: backend_id.to_string(),
                endpoint_name,
                enabled: output.enabled,
                width: output.width,
                height: output.height,
                opacity: output.opacity,
                blackout: output.blackout,
                composition_id: output.composition_id,
            })
        })
        .collect()
}

pub fn build_external_video_io_route_plans(
    snapshot: &VideoSnapshot,
    status: &VideoRuntimeStatus,
) -> ExternalVideoIoRoutePlans {
    ExternalVideoIoRoutePlans {
        inputs: build_external_video_input_plans(snapshot)
            .into_iter()
            .map(|plan| {
                let backend = external_video_backend_route_state(status, &plan.backend_id);
                let ready = backend.ready;
                ExternalVideoInputRoutePlan {
                    layer_id: plan.layer_id,
                    label: plan.label,
                    kind: plan.kind,
                    backend_id: plan.backend_id,
                    backend_label: backend.label,
                    backend_state: backend.state,
                    backend_detail: backend.detail,
                    endpoint_name: plan.endpoint_name,
                    enabled: plan.enabled,
                    ready,
                    live: plan.enabled && ready,
                    issue: backend.issue,
                }
            })
            .collect(),
        outputs: build_external_video_output_plans(snapshot)
            .into_iter()
            .map(|plan| {
                let backend = external_video_backend_route_state(status, &plan.backend_id);
                let ready = backend.ready;
                ExternalVideoOutputRoutePlan {
                    output_id: plan.output_id,
                    label: plan.label,
                    kind: plan.kind,
                    backend_id: plan.backend_id,
                    backend_label: backend.label,
                    backend_state: backend.state,
                    backend_detail: backend.detail,
                    endpoint_name: plan.endpoint_name,
                    enabled: plan.enabled,
                    width: plan.width,
                    height: plan.height,
                    opacity: plan.opacity,
                    blackout: plan.blackout,
                    composition_id: plan.composition_id,
                    ready,
                    live: plan.enabled && !plan.blackout && plan.opacity > 0.0 && ready,
                    issue: backend.issue,
                }
            })
            .collect(),
    }
}

fn external_video_backend_route_state(
    status: &VideoRuntimeStatus,
    backend_id: &str,
) -> ExternalVideoBackendRouteState {
    match status
        .backends
        .iter()
        .find(|backend| backend.id == backend_id)
    {
        Some(backend) => {
            let ready = backend.state == VideoBackendState::Available;
            ExternalVideoBackendRouteState {
                label: Some(backend.label.clone()),
                state: Some(backend.state.clone()),
                detail: Some(backend.detail.clone()),
                ready,
                issue: (!ready).then(|| format!("{}: {}", backend.label, backend.detail)),
            }
        }
        None => ExternalVideoBackendRouteState {
            label: None,
            state: None,
            detail: None,
            ready: false,
            issue: Some(format!("Backend status '{backend_id}' is unavailable")),
        },
    }
}

impl ExternalVideoTransportRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_routes(&self) -> &[ExternalVideoTransportRoute] {
        &self.active_routes
    }

    pub fn status(&self) -> ExternalVideoTransportStatus {
        ExternalVideoTransportStatus {
            active_routes: self.active_routes.clone(),
            active_count: self.active_routes.len(),
        }
    }

    pub fn sync_routes(
        &mut self,
        plans: &ExternalVideoIoRoutePlans,
    ) -> ExternalVideoTransportSyncReport {
        let mut driver = NoopExternalVideoTransportDriver;
        self.sync_routes_with_driver(plans, &mut driver)
    }

    pub fn sync_routes_with_driver<D: ExternalVideoTransportDriver>(
        &mut self,
        plans: &ExternalVideoIoRoutePlans,
        driver: &mut D,
    ) -> ExternalVideoTransportSyncReport {
        self.sync_routes_with_driver_and_role(plans, driver, MachineOutputRole::Both)
    }

    pub fn sync_routes_with_driver_and_role<D: ExternalVideoTransportDriver>(
        &mut self,
        plans: &ExternalVideoIoRoutePlans,
        driver: &mut D,
        role: MachineOutputRole,
    ) -> ExternalVideoTransportSyncReport {
        self.sync_routes_with_driver_and_role_with_start_admission(plans, driver, role, |_| Ok(()))
    }

    /// Synchronize routes while allowing the owner to re-admit external
    /// resource creation after every stop phase. The callback is deliberately
    /// between stop and start: a route replacement must never create a new
    /// sender/window using an activation token that a teardown transition has
    /// already invalidated.
    pub fn sync_routes_with_driver_and_role_with_start_admission<
        D: ExternalVideoTransportDriver,
        F: FnMut(&mut D) -> Result<(), String>,
    >(
        &mut self,
        plans: &ExternalVideoIoRoutePlans,
        driver: &mut D,
        role: MachineOutputRole,
        mut admit_start: F,
    ) -> ExternalVideoTransportSyncReport {
        let mut desired = Vec::new();
        let mut blocked = Vec::new();
        let mut idle = Vec::new();

        for input in &plans.inputs {
            let route = external_video_input_transport_route(input);
            if input.ready && input.live {
                push_unique_transport_route(&mut desired, route);
            } else if !input.ready {
                blocked.push(ExternalVideoTransportBlockedRoute {
                    route,
                    issue: input
                        .issue
                        .clone()
                        .unwrap_or_else(|| "Input route is unavailable".to_string()),
                });
            } else {
                idle.push(route);
            }
        }

        for output in &plans.outputs {
            let route = external_video_output_transport_route(output);
            if !role.video_allowed() {
                blocked.push(ExternalVideoTransportBlockedRoute {
                    route,
                    issue: "External video output blocked by machine output role".to_string(),
                });
            } else if output.ready && output.live {
                push_unique_transport_route(&mut desired, route);
            } else if !output.ready {
                blocked.push(ExternalVideoTransportBlockedRoute {
                    route,
                    issue: output
                        .issue
                        .clone()
                        .unwrap_or_else(|| "Output route is unavailable".to_string()),
                });
            } else {
                idle.push(route);
            }
        }

        let stopped = self
            .active_routes
            .iter()
            .filter(|route| !desired.contains(route))
            .cloned()
            .collect::<Vec<_>>();
        let kept = desired
            .iter()
            .filter(|route| self.active_routes.contains(route))
            .cloned()
            .collect::<Vec<_>>();
        let started = desired
            .iter()
            .filter(|route| !self.active_routes.contains(route))
            .cloned()
            .collect::<Vec<_>>();

        let mut active_routes = kept.clone();
        let mut started_ok = Vec::new();
        let mut stopped_ok = Vec::new();
        let mut start_failed = Vec::new();
        let mut stop_failed = Vec::new();

        for route in &stopped {
            match driver.stop_route(route) {
                Ok(()) => stopped_ok.push(route.clone()),
                Err(error) => {
                    push_unique_transport_route(&mut active_routes, route.clone());
                    stop_failed.push(ExternalVideoTransportFailedRoute {
                        route: route.clone(),
                        issue: error.message,
                    });
                }
            }
        }
        if stop_failed.is_empty() {
            if let Err(error) = admit_start(driver) {
                for route in &started {
                    start_failed.push(ExternalVideoTransportFailedRoute {
                        route: route.clone(),
                        issue: format!(
                            "External video start admission was denied after route stop: {error}"
                        ),
                    });
                }
            } else {
                for route in &started {
                    match driver.start_route(route) {
                        Ok(()) => {
                            push_unique_transport_route(&mut active_routes, route.clone());
                            started_ok.push(route.clone());
                        }
                        Err(error) => start_failed.push(ExternalVideoTransportFailedRoute {
                            route: route.clone(),
                            issue: error.message,
                        }),
                    }
                }
            }
        } else {
            for route in &started {
                start_failed.push(ExternalVideoTransportFailedRoute {
                    route: route.clone(),
                    issue: "External video start was withheld because route teardown failed"
                        .to_string(),
                });
            }
        }

        self.active_routes = active_routes;
        ExternalVideoTransportSyncReport {
            active_count: self.active_routes.len(),
            started: started_ok,
            kept,
            stopped: stopped_ok,
            blocked,
            start_failed,
            stop_failed,
            idle,
        }
    }

    pub fn clear(&mut self) -> Vec<ExternalVideoTransportRoute> {
        std::mem::take(&mut self.active_routes)
    }
}

impl ExternalVideoTransportDriver for NoopExternalVideoTransportDriver {
    fn start_route(
        &mut self,
        _route: &ExternalVideoTransportRoute,
    ) -> Result<(), ExternalVideoTransportDriverError> {
        Ok(())
    }

    fn stop_route(
        &mut self,
        _route: &ExternalVideoTransportRoute,
    ) -> Result<(), ExternalVideoTransportDriverError> {
        Ok(())
    }
}

fn external_video_input_transport_route(
    plan: &ExternalVideoInputRoutePlan,
) -> ExternalVideoTransportRoute {
    ExternalVideoTransportRoute {
        direction: ExternalVideoTransportDirection::Input,
        route_id: plan.layer_id,
        label: plan.label.clone(),
        backend_id: plan.backend_id.clone(),
        endpoint_name: plan.endpoint_name.clone(),
    }
}

fn external_video_output_transport_route(
    plan: &ExternalVideoOutputRoutePlan,
) -> ExternalVideoTransportRoute {
    ExternalVideoTransportRoute {
        direction: ExternalVideoTransportDirection::Output,
        route_id: plan.output_id,
        label: plan.label.clone(),
        backend_id: plan.backend_id.clone(),
        endpoint_name: plan.endpoint_name.clone(),
    }
}

fn push_unique_transport_route(
    routes: &mut Vec<ExternalVideoTransportRoute>,
    route: ExternalVideoTransportRoute,
) {
    if !routes.contains(&route) {
        routes.push(route);
    }
}

pub fn external_video_source_backend_id(kind: &VideoSourceKind) -> Option<&'static str> {
    match kind {
        VideoSourceKind::Camera => Some("camera"),
        VideoSourceKind::ScreenCapture => Some("screen_capture"),
        VideoSourceKind::Ndi => Some("ndi"),
        VideoSourceKind::Spout => Some("spout"),
        VideoSourceKind::Syphon => Some("syphon"),
        VideoSourceKind::File | VideoSourceKind::StillImage => None,
    }
}

pub fn external_video_output_backend_id(kind: &VideoOutputKind) -> Option<&'static str> {
    match kind {
        VideoOutputKind::NdiSender => Some("ndi"),
        VideoOutputKind::SpoutSender => Some("spout"),
        VideoOutputKind::SyphonServer => Some("syphon"),
        VideoOutputKind::Display => None,
    }
}

pub fn build_video_output_render_plan(
    snapshot: &VideoSnapshot,
    output_id: VideoOutputId,
) -> Result<VideoOutputRenderPlan, VideoOutputRenderError> {
    let output = snapshot
        .outputs
        .iter()
        .find(|candidate| candidate.id == output_id)
        .cloned()
        .ok_or(VideoOutputRenderError::MissingOutput { output_id })?;
    if output.width == 0 || output.height == 0 {
        return Err(VideoOutputRenderError::InvalidOutputSize { output_id });
    }
    let composition = build_composition_plans(snapshot)
        .into_iter()
        .find(|plan| plan.composition_id == output.composition_id)
        .ok_or(VideoOutputRenderError::MissingComposition {
            output_id,
            composition_id: output.composition_id,
        })?;
    Ok(video_output_render_plan(output, composition))
}

pub fn render_video_output_test_pattern(
    snapshot: &VideoSnapshot,
    output_id: VideoOutputId,
    width: u32,
    height: u32,
) -> Result<VideoFrame, VideoPreviewError> {
    if width == 0 || height == 0 {
        return Err(VideoPreviewError::InvalidSize);
    }
    let plan =
        build_video_output_render_plan(snapshot, output_id).map_err(VideoPreviewError::Output)?;
    Ok(apply_video_output_mapping(
        video_output_test_pattern(width, height, output_id),
        &plan.mapping,
    ))
}

fn video_layer_transition_target_ids(
    snapshot: &VideoSnapshot,
    target: &VideoLayerTransitionTarget,
) -> Result<Vec<VideoLayerId>, VideoPreviewError> {
    match target {
        VideoLayerTransitionTarget::Layer { layer_id } => Ok(vec![*layer_id]),
        VideoLayerTransitionTarget::Group { group_id } => snapshot
            .layer_groups
            .iter()
            .find(|group| group.id == *group_id)
            .map(|group| group.layer_ids.clone())
            .ok_or_else(|| {
                VideoPreviewError::InvalidLayerTransition(format!(
                    "Video transition references missing group {}",
                    group_id.0
                ))
            }),
    }
}

fn video_layer_transition_curve_progress(
    curve: VideoLayerTransitionCurve,
    progress_millis: u16,
) -> f32 {
    let p = f32::from(progress_millis.min(1000)) / 1000.0;
    match curve {
        VideoLayerTransitionCurve::Linear => p,
        VideoLayerTransitionCurve::EaseIn => p * p,
        VideoLayerTransitionCurve::EaseOut => 1.0 - (1.0 - p) * (1.0 - p),
        VideoLayerTransitionCurve::EaseInOut => p * p * (3.0 - 2.0 * p),
    }
}

fn apply_video_layer_transition_weights(
    snapshot: &VideoSnapshot,
    runtime: &VideoLayerTransitionRuntimeSnapshot,
    plan: &mut CompositionPlan,
) -> Result<(), VideoPreviewError> {
    validate_video_layer_transition_runtime(runtime, snapshot)
        .map_err(VideoPreviewError::InvalidLayerTransition)?;
    for active in &runtime.buses {
        let Some(bus) = snapshot
            .transition_buses
            .iter()
            .find(|bus| bus.id == active.bus_id && bus.composition_id == plan.composition_id)
        else {
            continue;
        };
        let from_ids = video_layer_transition_target_ids(snapshot, &active.from)?;
        let to_ids = video_layer_transition_target_ids(snapshot, &active.to)?;
        let mut member_ids = Vec::new();
        for member in &bus.members {
            member_ids.extend(video_layer_transition_target_ids(snapshot, member)?);
        }
        let progress = video_layer_transition_curve_progress(active.curve, active.progress_millis);
        for layer in &mut plan.layers {
            if !member_ids.contains(&layer.layer_id) {
                continue;
            }
            let weight = if from_ids.contains(&layer.layer_id) {
                1.0 - progress
            } else if to_ids.contains(&layer.layer_id) {
                progress
            } else {
                0.0
            };
            layer.opacity = (layer.opacity * weight).clamp(0.0, 1.0);
        }
    }
    plan.layers.retain(|layer| layer.opacity > 0.0);
    Ok(())
}

fn build_composition_plan(
    snapshot: &VideoSnapshot,
    composition: CompositionSummary,
) -> CompositionPlan {
    let master_opacity = snapshot.master_opacity.clamp(0.0, 1.0);
    let source_layers = composition
        .layer_ids
        .iter()
        .filter_map(|layer_id| snapshot.layers.iter().find(|layer| layer.id == *layer_id))
        .collect::<Vec<_>>();
    let solo_active = !snapshot.blackout
        && source_layers.iter().any(|layer| {
            let state = sanitize_layer_state(layer.state.clone());
            state.enabled && state.solo && state.opacity > 0.0
        });
    let layers = source_layers
        .into_iter()
        .map(|layer| {
            let state = sanitize_layer_state(layer.state.clone());
            CompositionLayerPlan {
                layer_id: layer.id,
                label: layer.label.clone(),
                source: layer.source.clone(),
                blend_mode: layer.blend_mode.clone(),
                opacity: if snapshot.blackout {
                    0.0
                } else if !state.enabled {
                    0.0
                } else if solo_active && !state.solo {
                    0.0
                } else {
                    (state.opacity * master_opacity).clamp(0.0, 1.0)
                },
                position_ms: state.position_ms,
                transform: state.transform,
                color: state.color,
                fx: state.fx,
            }
        })
        .filter(|layer| layer.opacity > 0.0)
        .collect();

    CompositionPlan {
        composition_id: composition.id,
        label: composition.label,
        output_ids: composition.output_ids,
        master_opacity,
        blackout: snapshot.blackout,
        layers,
    }
}

fn video_output_render_plan(
    output: VideoOutputSummary,
    mut composition: CompositionPlan,
) -> VideoOutputRenderPlan {
    let output_opacity = if output.opacity.is_finite() {
        output.opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let output_blackout = output.blackout || !output.enabled;
    if output_blackout {
        composition.layers.clear();
        composition.blackout = true;
    } else {
        for layer in &mut composition.layers {
            layer.opacity = (layer.opacity * output_opacity).clamp(0.0, 1.0);
        }
        composition.layers.retain(|layer| layer.opacity > 0.0);
    }

    VideoOutputRenderPlan {
        output_id: output.id,
        label: output.label,
        kind: output.kind,
        enabled: output.enabled,
        width: output.width,
        height: output.height,
        fullscreen: output.fullscreen,
        monitor_id: output.monitor_id,
        endpoint_name: output.endpoint_name,
        output_opacity,
        output_blackout,
        mapping: output.mapping,
        composition,
    }
}

fn debug_color(layer_id: VideoLayerId, pts_ms: u64) -> [u8; 4] {
    let seed = layer_id
        .wrapping_mul(1_103_515_245)
        .wrapping_add(pts_ms / 16);
    [
        40u8.saturating_add((seed & 0x7f) as u8),
        60u8.saturating_add(((seed >> 8) & 0x7f) as u8),
        80u8.saturating_add(((seed >> 16) & 0x7f) as u8),
        255,
    ]
}

fn video_output_test_pattern(width: u32, height: u32, output_id: VideoOutputId) -> VideoFrame {
    let mut data = vec![0u8; width as usize * height as usize * 4];
    let grid_x = (width / 16).max(1);
    let grid_y = (height / 16).max(1);
    let major_grid_x = (width / 4).max(1);
    let major_grid_y = (height / 4).max(1);
    let center_x = width / 2;
    let center_y = height / 2;
    let min_dimension = width.min(height).max(1);
    let border_x = (width / 160).max(1);
    let border_y = (height / 160).max(1);
    let corner_size = (min_dimension / 7).max(6).min(min_dimension);
    let corner_thickness = (min_dimension / 48).max(2).min(corner_size);
    let center_thickness = (min_dimension / 140).max(1);
    let safe_left = width / 10;
    let safe_right = width.saturating_sub(safe_left + 1);
    let safe_top = height / 10;
    let safe_bottom = height.saturating_sub(safe_top + 1);
    let third_x_1 = width / 3;
    let third_x_2 = width.saturating_mul(2) / 3;
    let third_y_1 = height / 3;
    let third_y_2 = height.saturating_mul(2) / 3;
    let ring_radius_a = min_dimension as f32 * 0.16;
    let ring_radius_b = min_dimension as f32 * 0.30;
    let ring_thickness = (min_dimension as f32 * 0.008).max(1.0);
    let diagonal_tolerance = (min_dimension as i64 / 90).max(1);

    for y in 0..height {
        for x in 0..width {
            let is_border =
                x < border_x || y < border_y || x + border_x >= width || y + border_y >= height;
            let is_minor_grid = x % grid_x == 0 || y % grid_y == 0;
            let is_major_grid = x % major_grid_x == 0 || y % major_grid_y == 0;
            let is_center = x.abs_diff(center_x) <= center_thickness
                || y.abs_diff(center_y) <= center_thickness;
            let is_safe_area = (x.abs_diff(safe_left) <= border_x
                || x.abs_diff(safe_right) <= border_x)
                && (safe_top..=safe_bottom).contains(&y)
                || (y.abs_diff(safe_top) <= border_y || y.abs_diff(safe_bottom) <= border_y)
                    && (safe_left..=safe_right).contains(&x);
            let is_thirds = (x.abs_diff(third_x_1) <= border_x
                || x.abs_diff(third_x_2) <= border_x)
                || (y.abs_diff(third_y_1) <= border_y || y.abs_diff(third_y_2) <= border_y);
            let diagonal_y = (x as u64 * height as u64 / width.max(1) as u64) as i64;
            let anti_diagonal_y = (((width.saturating_sub(1).saturating_sub(x)) as u64
                * height as u64)
                / width.max(1) as u64) as i64;
            let is_diagonal = (y as i64 - diagonal_y).abs() <= diagonal_tolerance
                || (y as i64 - anti_diagonal_y).abs() <= diagonal_tolerance;
            let distance_from_center = ((x as f32 - center_x as f32).powi(2)
                + (y as f32 - center_y as f32).powi(2))
            .sqrt();
            let is_target_ring = (distance_from_center - ring_radius_a).abs() <= ring_thickness
                || (distance_from_center - ring_radius_b).abs() <= ring_thickness;
            let checker = ((x / grid_x) + (y / grid_y)) % 2 == 0;
            let mut rgba = if checker {
                [18, 22, 28, 255]
            } else {
                [10, 13, 18, 255]
            };

            if is_minor_grid {
                rgba = [74, 88, 105, 255];
            }
            if is_major_grid {
                rgba = [118, 136, 158, 255];
            }
            if is_diagonal {
                rgba = [92, 73, 125, 255];
            }
            if is_thirds {
                rgba = [92, 120, 78, 255];
            }
            if is_safe_area {
                rgba = [226, 174, 64, 255];
            }
            if is_target_ring {
                rgba = [242, 206, 88, 255];
            }
            if is_center {
                rgba = if x.abs_diff(center_x) <= 1 {
                    [0, 210, 255, 255]
                } else {
                    [255, 80, 180, 255]
                };
            }
            if is_border {
                rgba = [245, 248, 252, 255];
            }
            if x < corner_size && y < corner_size && (x < corner_thickness || y < corner_thickness)
            {
                rgba = [255, 64, 64, 255];
            } else if x >= width.saturating_sub(corner_size)
                && y < corner_size
                && (x >= width.saturating_sub(corner_thickness) || y < corner_thickness)
            {
                rgba = [64, 255, 96, 255];
            } else if x >= width.saturating_sub(corner_size)
                && y >= height.saturating_sub(corner_size)
                && (x >= width.saturating_sub(corner_thickness)
                    || y >= height.saturating_sub(corner_thickness))
            {
                rgba = [64, 144, 255, 255];
            } else if x < corner_size
                && y >= height.saturating_sub(corner_size)
                && (x < corner_thickness || y >= height.saturating_sub(corner_thickness))
            {
                rgba = [255, 220, 64, 255];
            }

            let index = ((y * width + x) * 4) as usize;
            data[index..index + 4].copy_from_slice(&rgba);
        }
    }

    VideoFrame {
        layer_id: output_id,
        width,
        height,
        pts_ms: 0,
        duration_ms: 33,
        format: VideoPixelFormat::Rgba8,
        data,
    }
}

fn apply_video_output_mapping(frame: VideoFrame, mapping: &VideoOutputMapping) -> VideoFrame {
    if frame.format != VideoPixelFormat::Rgba8
        || frame.width == 0
        || frame.height == 0
        || frame.data.len() != frame.width as usize * frame.height as usize * 4
        || video_output_mapping_is_identity(mapping)
    {
        return frame;
    }

    let mut data = vec![0u8; frame.data.len()];
    let output_aspect_ratio = (frame.width as f32 / frame.height as f32).max(0.001);
    for y in 0..frame.height {
        let centered_y = (y as f32 + 0.5) / frame.height as f32 - 0.5;
        for x in 0..frame.width {
            let centered_x = (x as f32 + 0.5) / frame.width as f32 - 0.5;
            let (src_x, src_y) =
                inverse_video_output_mapping(centered_x, centered_y, mapping, output_aspect_ratio);
            if !(-0.5..0.5).contains(&src_x) || !(-0.5..0.5).contains(&src_y) {
                continue;
            }

            let source_x = ((src_x + 0.5) * frame.width as f32)
                .floor()
                .clamp(0.0, (frame.width - 1) as f32) as u32;
            let source_y = ((src_y + 0.5) * frame.height as f32)
                .floor()
                .clamp(0.0, (frame.height - 1) as f32) as u32;
            let src_index = ((source_y * frame.width + source_x) * 4) as usize;
            let dst_index = ((y * frame.width + x) * 4) as usize;
            data[dst_index..dst_index + 4].copy_from_slice(&frame.data[src_index..src_index + 4]);
            apply_output_blend_correction(
                &mut data[dst_index..dst_index + 4],
                (x as f32 + 0.5) / frame.width as f32,
                (y as f32 + 0.5) / frame.height as f32,
                mapping,
            );
        }
    }

    VideoFrame { data, ..frame }
}

fn apply_output_blend_correction(rgba: &mut [u8], u: f32, v: f32, mapping: &VideoOutputMapping) {
    let edge = |value: f32, width: f32| {
        let width = finite_or(width, 0.0).clamp(0.0, 1.0);
        if width <= 0.0001 {
            return 1.0;
        }
        let t = (value / width).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let mask = edge(u, mapping.edge_blend_left)
        * edge(1.0 - u, mapping.edge_blend_right)
        * edge(v, mapping.edge_blend_top)
        * edge(1.0 - v, mapping.edge_blend_bottom);
    let gamma = finite_or(mapping.edge_blend_gamma, 2.2).clamp(0.1, 8.0);
    let factor = mask.clamp(0.0, 1.0).powf(gamma) * combined_output_mask_factor(u, v, mapping);
    let black_level = finite_or(mapping.black_level, 0.0).clamp(0.0, 1.0);
    for channel in &mut rgba[..3] {
        let normalized = *channel as f32 / 255.0;
        *channel = ((black_level + normalized * (1.0 - black_level)) * factor * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    rgba[3] = (rgba[3] as f32 * factor).round().clamp(0.0, 255.0) as u8;
}

fn polygon_mask_factor(u: f32, v: f32, mapping: &VideoOutputMapping) -> f32 {
    let count = usize::from(mapping.mask_point_count).min(mapping.mask_points.len());
    if count < 3 {
        return 1.0;
    }
    let point = (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
    let points = &mapping.mask_points[..count];
    let mut inside = false;
    let mut minimum_distance = f32::MAX;
    let mut previous = points[count - 1];
    for current in points {
        let crosses = (current.y > point.1) != (previous.y > point.1)
            && point.0
                < (previous.x - current.x) * (point.1 - current.y) / (previous.y - current.y)
                    + current.x;
        if crosses {
            inside = !inside;
        }
        minimum_distance = minimum_distance.min(point_segment_distance(
            point,
            (previous.x, previous.y),
            (current.x, current.y),
        ));
        previous = *current;
    }
    let softness = finite_or(mapping.mask_softness, 0.0).clamp(0.0, 0.5);
    let base = if !inside {
        0.0
    } else if softness <= 0.0001 {
        1.0
    } else {
        let t = (minimum_distance / softness).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    base
}

fn combined_output_mask_factor(u: f32, v: f32, mapping: &VideoOutputMapping) -> f32 {
    let polygon_enabled = mapping.mask_point_count >= 3;
    let bitmap_enabled = video_bitmap_mask_dimensions(mapping).is_some();
    if !polygon_enabled && !bitmap_enabled {
        return 1.0;
    }
    let base = polygon_mask_factor(u, v, mapping) * bitmap_mask_factor(u, v, mapping);
    if mapping.mask_invert {
        1.0 - base.clamp(0.0, 1.0)
    } else {
        base.clamp(0.0, 1.0)
    }
}

fn video_bitmap_mask_dimensions(mapping: &VideoOutputMapping) -> Option<(usize, usize)> {
    let width = usize::from(
        mapping
            .bitmap_mask_width
            .min(VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION),
    );
    let height = usize::from(
        mapping
            .bitmap_mask_height
            .min(VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION),
    );
    (width > 0
        && height > 0
        && width.saturating_mul(height) <= VIDEO_OUTPUT_BITMAP_MASK_WORD_CAPACITY * 8)
        .then_some((width, height))
}

pub fn video_bitmap_mask_luma(mapping: &VideoOutputMapping, x: usize, y: usize) -> u8 {
    let Some((width, height)) = video_bitmap_mask_dimensions(mapping) else {
        return 255;
    };
    let index = y.min(height - 1) * width + x.min(width - 1);
    let word = mapping.bitmap_mask_luma_words[index / 8];
    let nibble = ((word >> ((index % 8) * 4)) & 0x0f) as u8;
    nibble * 17
}

fn bitmap_mask_factor(u: f32, v: f32, mapping: &VideoOutputMapping) -> f32 {
    let Some((width, height)) = video_bitmap_mask_dimensions(mapping) else {
        return 1.0;
    };
    let x = u.clamp(0.0, 1.0) * (width.saturating_sub(1)) as f32;
    let y = v.clamp(0.0, 1.0) * (height.saturating_sub(1)) as f32;
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let sample =
        |sample_x, sample_y| video_bitmap_mask_luma(mapping, sample_x, sample_y) as f32 / 255.0;
    let top = sample(x0, y0) + (sample(x1, y0) - sample(x0, y0)) * tx;
    let bottom = sample(x0, y1) + (sample(x1, y1) - sample(x0, y1)) * tx;
    top + (bottom - top) * ty
}

pub fn load_video_bitmap_mask(
    path: impl AsRef<Path>,
) -> Result<(u8, u8, [u32; VIDEO_OUTPUT_BITMAP_MASK_WORD_CAPACITY]), StillImageError> {
    let image = image::open(path.as_ref())
        .map_err(|error| StillImageError::Decode(error.to_string()))?
        .resize_exact(
            u32::from(VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION),
            u32::from(VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION),
            image::imageops::FilterType::Triangle,
        )
        .to_luma8();
    let mut words = [0_u32; VIDEO_OUTPUT_BITMAP_MASK_WORD_CAPACITY];
    for (index, pixel) in image.pixels().enumerate() {
        let nibble = ((u16::from(pixel.0[0]) + 8) / 17).min(15) as u32;
        words[index / 8] |= nibble << ((index % 8) * 4);
    }
    Ok((
        VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION,
        VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION,
        words,
    ))
}

fn point_segment_distance(point: (f32, f32), start: (f32, f32), end: (f32, f32)) -> f32 {
    let edge = (end.0 - start.0, end.1 - start.1);
    let length_squared = edge.0 * edge.0 + edge.1 * edge.1;
    if length_squared <= f32::EPSILON {
        return ((point.0 - start.0).powi(2) + (point.1 - start.1).powi(2)).sqrt();
    }
    let t = (((point.0 - start.0) * edge.0 + (point.1 - start.1) * edge.1) / length_squared)
        .clamp(0.0, 1.0);
    let nearest = (start.0 + edge.0 * t, start.1 + edge.1 * t);
    ((point.0 - nearest.0).powi(2) + (point.1 - nearest.1).powi(2)).sqrt()
}

fn inverse_video_output_mapping(
    centered_x: f32,
    centered_y: f32,
    mapping: &VideoOutputMapping,
    output_aspect_ratio: f32,
) -> (f32, f32) {
    let mut x = centered_x;
    let mut y = centered_y;

    let (corner_x, corner_y) = bilinear_corner_offset(centered_x, centered_y, mapping);
    x -= corner_x + finite_or(mapping.keystone_x, 0.0).clamp(-1.0, 1.0) * centered_y;
    y -= corner_y + finite_or(mapping.keystone_y, 0.0).clamp(-1.0, 1.0) * centered_x;

    x -= finite_or(mapping.offset_x, 0.0);
    y -= finite_or(mapping.offset_y, 0.0);

    let angle = -finite_or(mapping.rotation_deg, 0.0).to_radians();
    let sin = angle.sin();
    let cos = angle.cos();
    let rotated_x = x * cos - y * sin;
    let rotated_y = x * sin + y * cos;
    let (scale_x, scale_y) = video_output_mapping_scale(mapping, output_aspect_ratio);
    let mut src_x = rotated_x / scale_x;
    let mut src_y = rotated_y / scale_y;
    let distortion = finite_or(mapping.lens_distortion, 0.0).clamp(-1.0, 1.0);
    if distortion.abs() > 0.0001 {
        let radius2 = src_x * src_x + src_y * src_y;
        let distortion_factor = (1.0 + distortion * radius2 * 1.5).max(0.05);
        src_x *= distortion_factor;
        src_y *= distortion_factor;
    }

    (src_x, src_y)
}

fn video_output_mapping_scale(
    mapping: &VideoOutputMapping,
    output_aspect_ratio: f32,
) -> (f32, f32) {
    let mut scale_x = finite_or(mapping.scale_x, 1.0).clamp(0.001, 100.0);
    let mut scale_y = finite_or(mapping.scale_y, 1.0).clamp(0.001, 100.0);
    let desired_aspect = finite_or(mapping.aspect_ratio, 1.0).clamp(0.001, 100.0);
    let output_aspect = finite_or(output_aspect_ratio, 1.0).clamp(0.001, 100.0);

    match mapping.aspect_mode {
        VideoOutputAspectMode::Stretch => {
            scale_x *= desired_aspect;
        }
        VideoOutputAspectMode::Fit => {
            if output_aspect > desired_aspect {
                scale_x *= desired_aspect / output_aspect;
            } else {
                scale_y *= output_aspect / desired_aspect;
            }
        }
        VideoOutputAspectMode::Fill => {
            if output_aspect > desired_aspect {
                scale_y *= output_aspect / desired_aspect;
            } else {
                scale_x *= desired_aspect / output_aspect;
            }
        }
    }

    (scale_x.max(0.001), scale_y.max(0.001))
}

fn bilinear_corner_offset(
    centered_x: f32,
    centered_y: f32,
    mapping: &VideoOutputMapping,
) -> (f32, f32) {
    let u = (centered_x + 0.5).clamp(0.0, 1.0);
    let v = (centered_y + 0.5).clamp(0.0, 1.0);
    let top_x = lerp(
        finite_or(mapping.corner_top_left_x, 0.0),
        finite_or(mapping.corner_top_right_x, 0.0),
        u,
    );
    let top_y = lerp(
        finite_or(mapping.corner_top_left_y, 0.0),
        finite_or(mapping.corner_top_right_y, 0.0),
        u,
    );
    let bottom_x = lerp(
        finite_or(mapping.corner_bottom_left_x, 0.0),
        finite_or(mapping.corner_bottom_right_x, 0.0),
        u,
    );
    let bottom_y = lerp(
        finite_or(mapping.corner_bottom_left_y, 0.0),
        finite_or(mapping.corner_bottom_right_y, 0.0),
        u,
    );
    (lerp(top_x, bottom_x, v), lerp(top_y, bottom_y, v))
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn video_output_mapping_is_identity(mapping: &VideoOutputMapping) -> bool {
    const EPSILON: f32 = 0.0001;
    mapping.offset_x.abs() < EPSILON
        && mapping.offset_y.abs() < EPSILON
        && (mapping.scale_x - 1.0).abs() < EPSILON
        && (mapping.scale_y - 1.0).abs() < EPSILON
        && mapping.rotation_deg.abs() < EPSILON
        && (mapping.aspect_ratio - 1.0).abs() < EPSILON
        && mapping.aspect_mode == VideoOutputAspectMode::Stretch
        && mapping.lens_distortion.abs() < EPSILON
        && mapping.edge_blend_left.abs() < EPSILON
        && mapping.edge_blend_right.abs() < EPSILON
        && mapping.edge_blend_top.abs() < EPSILON
        && mapping.edge_blend_bottom.abs() < EPSILON
        && (mapping.edge_blend_gamma - 2.2).abs() < EPSILON
        && mapping.black_level.abs() < EPSILON
        && mapping.mask_point_count < 3
        && mapping.bitmap_mask_width == 0
        && mapping.bitmap_mask_height == 0
        && mapping.keystone_x.abs() < EPSILON
        && mapping.keystone_y.abs() < EPSILON
        && mapping.corner_top_left_x.abs() < EPSILON
        && mapping.corner_top_left_y.abs() < EPSILON
        && mapping.corner_top_right_x.abs() < EPSILON
        && mapping.corner_top_right_y.abs() < EPSILON
        && mapping.corner_bottom_right_x.abs() < EPSILON
        && mapping.corner_bottom_right_y.abs() < EPSILON
        && mapping.corner_bottom_left_x.abs() < EPSILON
        && mapping.corner_bottom_left_y.abs() < EPSILON
}

fn blend_transformed_rgba8(
    destination: &mut [u8],
    frame: &VideoFrame,
    width: u32,
    height: u32,
    opacity: f32,
    blend_mode: &VideoBlendMode,
    transform: &Transform2D,
    color: &VideoColorAdjust,
    fx: &VideoFxAdjust,
) {
    if opacity <= 0.0 {
        return;
    }
    let fx = sanitize_fx_adjust(*fx);

    let crop_left = transform.crop_left.clamp(0.0, 1.0);
    let crop_top = transform.crop_top.clamp(0.0, 1.0);
    let crop_right = (1.0 - transform.crop_right.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let crop_bottom = (1.0 - transform.crop_bottom.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    if crop_right <= crop_left || crop_bottom <= crop_top {
        return;
    }

    let angle = -transform.rotation_deg.to_radians();
    let sin = angle.sin();
    let cos = angle.cos();
    let scale_x = transform.scale_x.max(0.001);
    let scale_y = transform.scale_y.max(0.001);

    for y in 0..height {
        let centered_y = (y as f32 + 0.5) / height as f32 - 0.5 - transform.y;
        for x in 0..width {
            let centered_x = (x as f32 + 0.5) / width as f32 - 0.5 - transform.x;
            let rotated_x = centered_x * cos - centered_y * sin;
            let rotated_y = centered_x * sin + centered_y * cos;
            let local_x = rotated_x / scale_x + 0.5;
            let local_y = rotated_y / scale_y + 0.5;
            if !(0.0..1.0).contains(&local_x) || !(0.0..1.0).contains(&local_y) {
                continue;
            }

            let src_u = crop_left + local_x * (crop_right - crop_left);
            let src_v = crop_top + local_y * (crop_bottom - crop_top);
            let src_x = (src_u * frame.width as f32)
                .floor()
                .clamp(0.0, (frame.width - 1) as f32) as u32;
            let src_y = (src_v * frame.height as f32)
                .floor()
                .clamp(0.0, (frame.height - 1) as f32) as u32;
            let dst_index = ((y * width + x) * 4) as usize;
            let sample = sample_rgba_pixel(frame, src_x, src_y, &fx);
            let mut adjusted = adjust_rgba_pixel(&sample, color);
            apply_color_key(&mut adjusted, &fx);
            blend_pixel(
                &mut destination[dst_index..dst_index + 4],
                &adjusted,
                opacity,
                blend_mode,
            );
        }
    }
}

fn sample_rgba_pixel(
    frame: &VideoFrame,
    mut src_x: u32,
    mut src_y: u32,
    fx: &VideoFxAdjust,
) -> [u8; 4] {
    let pixelate = fx.pixelate.round().clamp(1.0, 128.0) as u32;
    if pixelate > 1 {
        src_x = ((src_x / pixelate) * pixelate + pixelate / 2).min(frame.width - 1);
        src_y = ((src_y / pixelate) * pixelate + pixelate / 2).min(frame.height - 1);
    }

    let radius = fx.blur.round().clamp(0.0, 8.0) as i32;
    let mut sample = if radius == 0 {
        raw_rgba_pixel(frame, src_x, src_y)
    } else {
        average_rgba_pixel(frame, src_x, src_y, radius)
    };
    apply_glow_and_edge(frame, src_x, src_y, &mut sample, fx);
    sample
}

fn raw_rgba_pixel(frame: &VideoFrame, src_x: u32, src_y: u32) -> [u8; 4] {
    let index = ((src_y * frame.width + src_x) * 4) as usize;
    [
        frame.data[index],
        frame.data[index + 1],
        frame.data[index + 2],
        frame.data[index + 3],
    ]
}

fn average_rgba_pixel(frame: &VideoFrame, src_x: u32, src_y: u32, radius: i32) -> [u8; 4] {
    let mut channels = [0u32; 4];
    let mut count = 0u32;
    for y in (src_y as i32 - radius)..=(src_y as i32 + radius) {
        if !(0..frame.height as i32).contains(&y) {
            continue;
        }
        for x in (src_x as i32 - radius)..=(src_x as i32 + radius) {
            if !(0..frame.width as i32).contains(&x) {
                continue;
            }
            let index = (((y as u32) * frame.width + x as u32) * 4) as usize;
            for channel in 0..4 {
                channels[channel] += frame.data[index + channel] as u32;
            }
            count += 1;
        }
    }

    if count == 0 {
        return [0, 0, 0, 0];
    }
    [
        (channels[0] / count) as u8,
        (channels[1] / count) as u8,
        (channels[2] / count) as u8,
        (channels[3] / count) as u8,
    ]
}

fn apply_glow_and_edge(
    frame: &VideoFrame,
    src_x: u32,
    src_y: u32,
    sample: &mut [u8; 4],
    fx: &VideoFxAdjust,
) {
    if fx.glow > 0.0 {
        let glow_radius = (fx.glow * 2.0).ceil().clamp(1.0, 8.0) as i32;
        let glow = bright_neighbor_average(frame, src_x, src_y, glow_radius);
        for channel in 0..3 {
            sample[channel] = (sample[channel] as f32 + glow[channel] * fx.glow)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }

    if fx.edge > 0.0 {
        let edge = edge_intensity(frame, src_x, src_y) * 255.0;
        let amount = fx.edge.clamp(0.0, 4.0);
        for channel in 0..3 {
            sample[channel] = (sample[channel] as f32 + (edge - sample[channel] as f32) * amount)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }
}

fn bright_neighbor_average(frame: &VideoFrame, src_x: u32, src_y: u32, radius: i32) -> [f32; 3] {
    let mut channels = [0.0f32; 3];
    let mut weight_sum = 0.0f32;
    for y in (src_y as i32 - radius)..=(src_y as i32 + radius) {
        if !(0..frame.height as i32).contains(&y) {
            continue;
        }
        for x in (src_x as i32 - radius)..=(src_x as i32 + radius) {
            if !(0..frame.width as i32).contains(&x) {
                continue;
            }
            let pixel = raw_rgba_pixel(frame, x as u32, y as u32);
            let brightness = pixel[0].max(pixel[1]).max(pixel[2]) as f32 / 255.0;
            let weight = ((brightness - 0.6) / 0.4).clamp(0.0, 1.0);
            if weight <= 0.0 {
                continue;
            }
            for channel in 0..3 {
                channels[channel] += pixel[channel] as f32 * weight;
            }
            weight_sum += weight;
        }
    }
    if weight_sum <= f32::EPSILON {
        return [0.0, 0.0, 0.0];
    }
    [
        channels[0] / weight_sum,
        channels[1] / weight_sum,
        channels[2] / weight_sum,
    ]
}

fn edge_intensity(frame: &VideoFrame, src_x: u32, src_y: u32) -> f32 {
    let center = luminance(raw_rgba_pixel(frame, src_x, src_y));
    let left = luminance(raw_rgba_pixel(frame, src_x.saturating_sub(1), src_y));
    let right = luminance(raw_rgba_pixel(
        frame,
        (src_x + 1).min(frame.width - 1),
        src_y,
    ));
    let top = luminance(raw_rgba_pixel(frame, src_x, src_y.saturating_sub(1)));
    let bottom = luminance(raw_rgba_pixel(
        frame,
        src_x,
        (src_y + 1).min(frame.height - 1),
    ));
    (center - left)
        .abs()
        .max((center - right).abs())
        .max((center - top).abs())
        .max((center - bottom).abs())
        .clamp(0.0, 1.0)
}

fn luminance(pixel: [u8; 4]) -> f32 {
    (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32) / 255.0
}

fn adjust_rgba_pixel(src: &[u8], color: &VideoColorAdjust) -> [u8; 4] {
    let color = sanitize_color_adjust(*color);
    let mut red = src[0] as f32 / 255.0;
    let mut green = src[1] as f32 / 255.0;
    let mut blue = src[2] as f32 / 255.0;

    if color.hue_deg.abs() > f32::EPSILON || (color.saturation - 1.0).abs() > f32::EPSILON {
        let y = 0.299 * red + 0.587 * green + 0.114 * blue;
        let i = 0.596 * red - 0.274 * green - 0.322 * blue;
        let q = 0.211 * red - 0.523 * green + 0.312 * blue;
        let angle = color.hue_deg.to_radians();
        let sin = angle.sin();
        let cos = angle.cos();
        let rotated_i = (i * cos - q * sin) * color.saturation;
        let rotated_q = (i * sin + q * cos) * color.saturation;
        red = y + 0.956 * rotated_i + 0.621 * rotated_q;
        green = y - 0.272 * rotated_i - 0.647 * rotated_q;
        blue = y - 1.106 * rotated_i + 1.703 * rotated_q;
    }

    red = apply_tone_adjust(red, &color);
    green = apply_tone_adjust(green, &color);
    blue = apply_tone_adjust(blue, &color);

    [
        (red * 255.0).round().clamp(0.0, 255.0) as u8,
        (green * 255.0).round().clamp(0.0, 255.0) as u8,
        (blue * 255.0).round().clamp(0.0, 255.0) as u8,
        src[3],
    ]
}

fn apply_color_key(pixel: &mut [u8; 4], fx: &VideoFxAdjust) {
    if fx.key_threshold <= 0.0 {
        return;
    }
    let red = pixel[0] as f32 / 255.0;
    let green = pixel[1] as f32 / 255.0;
    let blue = pixel[2] as f32 / 255.0;
    let key_red = fx.key_red.clamp(0.0, 1.0);
    let key_green = fx.key_green.clamp(0.0, 1.0);
    let key_blue = fx.key_blue.clamp(0.0, 1.0);
    let distance =
        ((red - key_red).powi(2) + (green - key_green).powi(2) + (blue - key_blue).powi(2)).sqrt();
    let alpha_scale = (distance / fx.key_threshold.max(0.001)).clamp(0.0, 1.0);
    pixel[3] = (pixel[3] as f32 * alpha_scale).round().clamp(0.0, 255.0) as u8;
}

fn apply_tone_adjust(value: f32, color: &VideoColorAdjust) -> f32 {
    let corrected =
        ((value.clamp(0.0, 1.0) - 0.5) * color.contrast + 0.5 + color.brightness).clamp(0.0, 1.0);
    if (color.gamma - 1.0).abs() <= f32::EPSILON {
        corrected
    } else {
        corrected.powf(1.0 / color.gamma)
    }
}

fn blend_pixel(dst: &mut [u8], src: &[u8], opacity: f32, blend_mode: &VideoBlendMode) {
    let src_alpha = (src[3] as f32 / 255.0) * opacity;
    if src_alpha <= 0.0 {
        return;
    }

    for channel in 0..3 {
        let dst_value = dst[channel] as f32;
        let src_value = src[channel] as f32;
        let blended = match blend_mode {
            VideoBlendMode::Normal => src_value,
            VideoBlendMode::Add => (dst_value + src_value).min(255.0),
            VideoBlendMode::Multiply => dst_value * src_value / 255.0,
            VideoBlendMode::Screen => 255.0 - ((255.0 - dst_value) * (255.0 - src_value) / 255.0),
        };
        dst[channel] = (dst_value + (blended - dst_value) * src_alpha)
            .round()
            .clamp(0.0, 255.0) as u8;
    }

    let dst_alpha = dst[3] as f32 / 255.0;
    dst[3] = ((src_alpha + dst_alpha * (1.0 - src_alpha)) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8;
}

pub fn sanitize_layer_state(mut state: VideoLayerState) -> VideoLayerState {
    state.opacity = finite_or(state.opacity, 1.0).clamp(0.0, 1.0);
    state.speed = finite_or(state.speed, 1.0).clamp(-4.0, 4.0);
    state.bpm_sync.ratio = finite_or(state.bpm_sync.ratio, 1.0).clamp(0.25, 4.0);
    state.bpm_sync.loop_bars = finite_or(state.bpm_sync.loop_bars, 1.0).clamp(0.25, 128.0);
    state.transform.x = finite_or(state.transform.x, 0.0);
    state.transform.y = finite_or(state.transform.y, 0.0);
    state.transform.scale_x = finite_or(state.transform.scale_x, 1.0).clamp(0.001, 100.0);
    state.transform.scale_y = finite_or(state.transform.scale_y, 1.0).clamp(0.001, 100.0);
    state.transform.rotation_deg = finite_or(state.transform.rotation_deg, 0.0);
    state.transform.crop_left = finite_or(state.transform.crop_left, 0.0).clamp(0.0, 1.0);
    state.transform.crop_top = finite_or(state.transform.crop_top, 0.0).clamp(0.0, 1.0);
    state.transform.crop_right = finite_or(state.transform.crop_right, 0.0).clamp(0.0, 1.0);
    state.transform.crop_bottom = finite_or(state.transform.crop_bottom, 0.0).clamp(0.0, 1.0);
    state.color = sanitize_color_adjust(state.color);
    state.fx = sanitize_fx_adjust(state.fx);
    if state.loop_enabled && state.loop_end_ms <= state.loop_start_ms {
        state.loop_end_ms = state.loop_start_ms.saturating_add(1);
    }
    state.cue_points = sanitize_video_cue_points(state.cue_points, &state.cue_points_ms);
    state.cue_points_ms = state
        .cue_points
        .iter()
        .map(|cue_point| cue_point.position_ms)
        .collect();
    state
}

pub fn sanitize_video_cue_points(
    cue_points: Vec<VideoCuePointSummary>,
    legacy_positions: &[u64],
) -> Vec<VideoCuePointSummary> {
    let mut cue_points = cue_points
        .into_iter()
        .map(|cue_point| VideoCuePointSummary {
            position_ms: cue_point.position_ms,
            label: cue_point.label.trim().to_string(),
            color: sanitize_cue_point_color(cue_point.color),
        })
        .collect::<Vec<_>>();
    for position_ms in legacy_positions {
        if !cue_points
            .iter()
            .any(|cue_point| cue_point.position_ms == *position_ms)
        {
            cue_points.push(VideoCuePointSummary {
                position_ms: *position_ms,
                label: String::new(),
                color: None,
            });
        }
    }
    cue_points.sort_by_key(|cue_point| cue_point.position_ms);
    cue_points.dedup_by_key(|cue_point| cue_point.position_ms);
    for (index, cue_point) in cue_points.iter_mut().enumerate() {
        if cue_point.label.is_empty() {
            cue_point.label = format!("Cue {}", index + 1);
        }
    }
    cue_points
}

fn sanitize_cue_point_color(color: Option<String>) -> Option<String> {
    let color = color?;
    let color = color.trim();
    let hex = color.strip_prefix('#').unwrap_or(color);
    if hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Some(format!("#{hex}"))
    } else {
        None
    }
}

pub fn sanitize_color_adjust(mut color: VideoColorAdjust) -> VideoColorAdjust {
    color.brightness = finite_or(color.brightness, 0.0).clamp(-1.0, 1.0);
    color.contrast = finite_or(color.contrast, 1.0).clamp(0.0, 4.0);
    color.hue_deg = finite_or(color.hue_deg, 0.0).rem_euclid(360.0);
    color.saturation = finite_or(color.saturation, 1.0).clamp(0.0, 4.0);
    color.gamma = finite_or(color.gamma, 1.0).clamp(0.1, 4.0);
    color
}

pub fn sanitize_fx_adjust(mut fx: VideoFxAdjust) -> VideoFxAdjust {
    fx.pixelate = finite_or(fx.pixelate, 1.0).clamp(1.0, 128.0);
    fx.blur = finite_or(fx.blur, 0.0).clamp(0.0, 8.0);
    fx.glow = finite_or(fx.glow, 0.0).clamp(0.0, 4.0);
    fx.edge = finite_or(fx.edge, 0.0).clamp(0.0, 4.0);
    fx.key_red = finite_or(fx.key_red, 0.0).clamp(0.0, 1.0);
    fx.key_green = finite_or(fx.key_green, 1.0).clamp(0.0, 1.0);
    fx.key_blue = finite_or(fx.key_blue, 0.0).clamp(0.0, 1.0);
    fx.key_threshold = finite_or(fx.key_threshold, 0.0).clamp(0.0, 1.0);
    fx
}

pub fn advance_layer_state(state: VideoLayerState, delta: Duration) -> VideoLayerState {
    advance_layer_state_with_bpm(state, delta, None)
}

pub fn advance_layer_state_with_bpm(
    state: VideoLayerState,
    delta: Duration,
    bpm: Option<f32>,
) -> VideoLayerState {
    advance_layer_state_with_bpm_and_duration(state, delta, bpm, None)
}

pub fn advance_layer_state_with_bpm_and_duration(
    state: VideoLayerState,
    delta: Duration,
    bpm: Option<f32>,
    source_duration_ms: Option<u64>,
) -> VideoLayerState {
    let mut state = sanitize_layer_state(state);
    clamp_layer_state_to_source_duration(&mut state, source_duration_ms);
    advance_sanitized_layer_state(state, delta, bpm, source_duration_ms)
}

pub fn advance_layer_state_with_clock_and_duration(
    state: VideoLayerState,
    delta: Duration,
    clock: Option<&ClockSnapshot>,
    source_duration_ms: Option<u64>,
) -> VideoLayerState {
    let mut state = sanitize_layer_state(state);
    clamp_layer_state_to_source_duration(&mut state, source_duration_ms);
    if let Some(clock) = clock {
        if should_lock_layer_to_bpm_clock(&state, clock) {
            return sync_layer_state_to_bpm_clock(state, clock);
        }
    }
    advance_sanitized_layer_state(
        state,
        delta,
        clock.map(|clock| clock.bpm),
        source_duration_ms,
    )
}

fn clamp_layer_state_to_source_duration(
    state: &mut VideoLayerState,
    source_duration_ms: Option<u64>,
) {
    let source_duration_ms = source_duration_ms.filter(|duration| *duration > 0);
    if let Some(duration) = source_duration_ms {
        state.position_ms = state.position_ms.min(duration);
        if state.loop_enabled {
            state.loop_start_ms = state.loop_start_ms.min(duration);
            state.loop_end_ms = state.loop_end_ms.min(duration);
            if state.loop_end_ms <= state.loop_start_ms {
                state.loop_enabled = false;
            }
        }
    }
}

fn advance_sanitized_layer_state(
    mut state: VideoLayerState,
    delta: Duration,
    bpm: Option<f32>,
    source_duration_ms: Option<u64>,
) -> VideoLayerState {
    let source_duration_ms = source_duration_ms.filter(|duration| *duration > 0);
    let speed = effective_speed(&state, bpm);
    if !state.playing || speed.abs() <= f32::EPSILON || delta.is_zero() {
        return state;
    }

    let delta_ms = (delta.as_secs_f64() * 1000.0 * f64::from(speed)).round() as i128;
    if delta_ms == 0 {
        return state;
    }

    let next_position = state.position_ms as i128 + delta_ms;
    if state.loop_enabled && state.loop_end_ms > state.loop_start_ms {
        let start = state.loop_start_ms as i128;
        let length = (state.loop_end_ms - state.loop_start_ms) as i128;
        let offset = (next_position - start).rem_euclid(length);
        state.position_ms = (start + offset) as u64;
    } else {
        if next_position <= 0 {
            state.position_ms = 0;
            state.playing = false;
        } else if let Some(duration) = source_duration_ms {
            if next_position >= duration as i128 {
                state.position_ms = duration;
                state.playing = false;
            } else {
                state.position_ms = next_position as u64;
            }
        } else {
            state.position_ms = next_position as u64;
        }
    }
    state
}

fn should_lock_layer_to_bpm_clock(state: &VideoLayerState, clock: &ClockSnapshot) -> bool {
    state.playing
        && state.bpm_sync.enabled
        && clock.bpm.is_finite()
        && clock.bpm > 0.0
        && state.loop_enabled
        && state.loop_end_ms > state.loop_start_ms
        && state.bpm_sync.loop_bars.is_finite()
        && state.bpm_sync.loop_bars > 0.0
        && state.bpm_sync.ratio.is_finite()
        && state.bpm_sync.ratio > 0.0
}

fn sync_layer_state_to_bpm_clock(
    mut state: VideoLayerState,
    clock: &ClockSnapshot,
) -> VideoLayerState {
    let loop_length_ms = state.loop_end_ms - state.loop_start_ms;
    if loop_length_ms == 0 {
        return state;
    }
    let cycle_beats = f64::from(state.bpm_sync.loop_bars.max(0.25)) * 4.0
        / f64::from(state.bpm_sync.ratio.max(0.25));
    if cycle_beats <= f64::EPSILON {
        return state;
    }
    let beat_position = clock.beat_counter as f64 + f64::from(clock.beat_phase.rem_euclid(1.0));
    let mut cycle_phase = (beat_position / cycle_beats).rem_euclid(1.0);
    if state.speed < 0.0 {
        cycle_phase = (1.0 - cycle_phase).rem_euclid(1.0);
    }
    let loop_offset = (cycle_phase * loop_length_ms as f64).round() as u64;
    state.position_ms = state
        .loop_start_ms
        .saturating_add(loop_offset.min(loop_length_ms.saturating_sub(1)));
    state
}

pub fn effective_speed(state: &VideoLayerState, bpm: Option<f32>) -> f32 {
    let state = sanitize_layer_state(state.clone());
    let Some(bpm) = bpm else {
        return state.speed;
    };
    if !state.bpm_sync.enabled
        || !bpm.is_finite()
        || bpm <= 0.0
        || !state.loop_enabled
        || state.loop_end_ms <= state.loop_start_ms
    {
        return state.speed;
    }

    let loop_length_ms = (state.loop_end_ms - state.loop_start_ms) as f64;
    let bar_ms = 240_000.0 / f64::from(bpm);
    let target_cycle_ms =
        (bar_ms * f64::from(state.bpm_sync.loop_bars)) / f64::from(state.bpm_sync.ratio);
    if target_cycle_ms <= f64::EPSILON {
        return state.speed;
    }

    let direction = if state.speed < 0.0 { -1.0 } else { 1.0 };
    (loop_length_ms / target_cycle_ms) as f32 * direction
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{
        AutoVjSnapshot, CompositionSummary, Transform2D, VideoBlendMode, VideoLayerSummary,
        VideoMaskPoint, VideoOutputKind, VideoOutputSummary, VideoSourceKind, VideoSourceSummary,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    static FAKE_FFMPEG_BINARY_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn frame(layer_id: VideoLayerId, pts_ms: u64) -> VideoFrame {
        VideoFrame {
            layer_id,
            width: 2,
            height: 2,
            pts_ms,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: vec![0; 16],
        }
    }

    fn rgba_frame(layer_id: VideoLayerId, rgba: [u8; 4]) -> VideoFrame {
        VideoFrame {
            layer_id,
            width: 1,
            height: 1,
            pts_ms: layer_id,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: rgba.to_vec(),
        }
    }

    fn rgba_frame_with_size(
        layer_id: VideoLayerId,
        width: u32,
        height: u32,
        data: Vec<u8>,
    ) -> VideoFrame {
        VideoFrame {
            layer_id,
            width,
            height,
            pts_ms: layer_id,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data,
        }
    }

    fn bgra_frame(layer_id: VideoLayerId, bgra: [u8; 4]) -> VideoFrame {
        VideoFrame {
            layer_id,
            width: 1,
            height: 1,
            pts_ms: layer_id,
            duration_ms: 16,
            format: VideoPixelFormat::Bgra8,
            data: bgra.to_vec(),
        }
    }

    fn dxt1_frame(layer_id: VideoLayerId, width: u32, height: u32, data: Vec<u8>) -> VideoFrame {
        VideoFrame {
            layer_id,
            width,
            height,
            pts_ms: layer_id,
            duration_ms: 16,
            format: VideoPixelFormat::Dxt1,
            data,
        }
    }

    fn dxt5_frame(layer_id: VideoLayerId, width: u32, height: u32, data: Vec<u8>) -> VideoFrame {
        VideoFrame {
            layer_id,
            width,
            height,
            pts_ms: layer_id,
            duration_ms: 16,
            format: VideoPixelFormat::Dxt5,
            data,
        }
    }

    #[test]
    fn ffprobe_json_parses_media_metadata() {
        let summary = parse_ffprobe_metadata_json(
            r#"{
                "streams": [
                    {
                        "codec_type": "video",
                        "codec_name": "h264",
                        "width": 1920,
                        "height": 1080,
                        "avg_frame_rate": "30000/1001",
                        "r_frame_rate": "30/1"
                    },
                    {
                        "codec_type": "audio",
                        "codec_name": "aac"
                    }
                ],
                "format": {
                    "duration": "12.345000"
                }
            }"#,
        )
        .unwrap();

        assert_eq!(summary.codec.as_deref(), Some("h264"));
        let metadata = summary.metadata.unwrap();
        assert_eq!(metadata.duration_ms, Some(12_345));
        assert_eq!(metadata.width, Some(1920));
        assert_eq!(metadata.height, Some(1080));
        assert!((metadata.frame_rate.unwrap() - 29.97).abs() < 0.01);
        assert!(metadata.has_audio);
    }

    #[test]
    fn ffprobe_json_ignores_zero_frame_rate_and_invalid_duration() {
        let summary = parse_ffprobe_metadata_json(
            r#"{
                "streams": [
                    {
                        "codec_type": "video",
                        "codec_name": "hap",
                        "width": 1280,
                        "height": 720,
                        "avg_frame_rate": "0/0"
                    }
                ],
                "format": {
                    "duration": "N/A"
                }
            }"#,
        )
        .unwrap();

        assert_eq!(summary.codec.as_deref(), Some("hap"));
        let metadata = summary.metadata.unwrap();
        assert_eq!(metadata.duration_ms, None);
        assert_eq!(metadata.width, Some(1280));
        assert_eq!(metadata.height, Some(720));
        assert_eq!(metadata.frame_rate, None);
        assert!(!metadata.has_audio);
    }

    fn layer_plan(
        layer_id: VideoLayerId,
        blend_mode: VideoBlendMode,
        opacity: f32,
    ) -> CompositionLayerPlan {
        CompositionLayerPlan {
            layer_id,
            label: format!("Layer {layer_id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(format!("layer-{layer_id}.mp4")),
                name: None,
                codec: None,
                metadata: None,
            },
            blend_mode,
            opacity,
            position_ms: 0,
            transform: Transform2D::default(),
            color: VideoColorAdjust::default(),
            fx: VideoFxAdjust::default(),
        }
    }

    fn plan(layers: Vec<CompositionLayerPlan>) -> CompositionPlan {
        CompositionPlan {
            composition_id: 1,
            label: "Main".to_string(),
            output_ids: Vec::new(),
            master_opacity: 1.0,
            blackout: false,
            layers,
        }
    }

    fn fake_ffmpeg_binary() -> PathBuf {
        let extension = if cfg!(windows) { "cmd" } else { "sh" };
        let serial = FAKE_FFMPEG_BINARY_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "syndocal-fake-ffmpeg-{}-{serial}.{extension}",
            std::process::id(),
        ));
        #[cfg(windows)]
        std::fs::write(
            &path,
            b"@echo off\r\nif \"%1\"==\"-version\" (\r\n  <nul set /p dummy=ffmpeg fake ABCD\r\n  exit /b 0\r\n)\r\nif \"%1\"==\"-decoders\" (\r\n  echo  V..... hap                  Vidvox Hap decoder\r\n  exit /b 0\r\n)\r\n<nul set /p dummy=ABCD\r\nexit /b 0\r\n",
        )
        .unwrap();
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(
                &path,
                b"#!/bin/sh\nif [ \"$1\" = \"-version\" ]; then printf 'ffmpeg fake ABCD'; exit 0; fi\nif [ \"$1\" = \"-decoders\" ]; then printf ' V..... hap                  Vidvox Hap decoder\\n'; exit 0; fi\nprintf ABCD\n",
            )
            .unwrap();
            let mut permissions = std::fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&path, permissions).unwrap();
        }
        path
    }

    #[test]
    fn sanitizes_layer_state_ranges() {
        let state = sanitize_layer_state(VideoLayerState {
            enabled: false,
            solo: true,
            opacity: 2.0,
            speed: 8.0,
            playing: true,
            position_ms: 0,
            loop_enabled: true,
            loop_start_ms: 100,
            loop_end_ms: 10,
            bpm_sync: protocol::VideoBpmSync {
                enabled: true,
                ratio: 10.0,
                loop_bars: f32::NAN,
            },
            cue_points: vec![VideoCuePointSummary {
                position_ms: 250,
                label: "  Drop  ".to_string(),
                color: Some("ff3366".to_string()),
            }],
            cue_points_ms: vec![500, 100, 500],
            transform: Transform2D {
                scale_x: 0.0,
                scale_y: 200.0,
                crop_left: f32::NAN,
                crop_right: 2.0,
                ..Transform2D::default()
            },
            color: VideoColorAdjust {
                brightness: 2.0,
                contrast: f32::NAN,
                hue_deg: -30.0,
                saturation: 5.0,
                gamma: 0.0,
            },
            fx: VideoFxAdjust {
                pixelate: 0.0,
                blur: 20.0,
                glow: 8.0,
                edge: f32::NAN,
                key_red: -1.0,
                key_green: f32::NAN,
                key_blue: 2.0,
                key_threshold: 2.0,
            },
        });

        assert!(!state.enabled);
        assert!(state.solo);
        assert_eq!(state.opacity, 1.0);
        assert_eq!(state.speed, 4.0);
        assert_eq!(state.transform.scale_x, 0.001);
        assert_eq!(state.transform.scale_y, 100.0);
        assert_eq!(state.transform.crop_left, 0.0);
        assert_eq!(state.transform.crop_right, 1.0);
        assert_eq!(state.loop_end_ms, 101);
        assert_eq!(state.bpm_sync.ratio, 4.0);
        assert_eq!(state.bpm_sync.loop_bars, 1.0);
        assert_eq!(state.color.brightness, 1.0);
        assert_eq!(state.color.contrast, 1.0);
        assert_eq!(state.color.hue_deg, 330.0);
        assert_eq!(state.color.saturation, 4.0);
        assert_eq!(state.color.gamma, 0.1);
        assert_eq!(state.fx.pixelate, 1.0);
        assert_eq!(state.fx.blur, 8.0);
        assert_eq!(state.fx.glow, 4.0);
        assert_eq!(state.fx.edge, 0.0);
        assert_eq!(state.fx.key_red, 0.0);
        assert_eq!(state.fx.key_green, 1.0);
        assert_eq!(state.fx.key_blue, 1.0);
        assert_eq!(state.fx.key_threshold, 1.0);
        assert_eq!(state.cue_points_ms, vec![100, 250, 500]);
        assert_eq!(
            state
                .cue_points
                .iter()
                .map(|cue_point| cue_point.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Cue 1", "Drop", "Cue 3"]
        );
        assert_eq!(state.cue_points[1].color.as_deref(), Some("#ff3366"));
    }

    #[test]
    fn advances_playing_layer_position() {
        let state = advance_layer_state(
            VideoLayerState {
                playing: true,
                position_ms: 1_000,
                speed: 2.0,
                ..VideoLayerState::default()
            },
            Duration::from_millis(25),
        );

        assert_eq!(state.position_ms, 1_050);
    }

    #[test]
    fn loops_forward_and_reverse_inside_ab_region() {
        let forward = advance_layer_state(
            VideoLayerState {
                playing: true,
                position_ms: 1_900,
                speed: 1.0,
                loop_enabled: true,
                loop_start_ms: 1_000,
                loop_end_ms: 2_000,
                ..VideoLayerState::default()
            },
            Duration::from_millis(250),
        );
        let reverse = advance_layer_state(
            VideoLayerState {
                playing: true,
                position_ms: 1_100,
                speed: -1.0,
                loop_enabled: true,
                loop_start_ms: 1_000,
                loop_end_ms: 2_000,
                ..VideoLayerState::default()
            },
            Duration::from_millis(250),
        );

        assert_eq!(forward.position_ms, 1_150);
        assert_eq!(reverse.position_ms, 1_850);
    }

    #[test]
    fn clamps_non_looping_playback_to_source_duration() {
        let forward = advance_layer_state_with_bpm_and_duration(
            VideoLayerState {
                playing: true,
                position_ms: 1_950,
                speed: 1.0,
                ..VideoLayerState::default()
            },
            Duration::from_millis(100),
            None,
            Some(2_000),
        );
        let reverse = advance_layer_state_with_bpm_and_duration(
            VideoLayerState {
                playing: true,
                position_ms: 50,
                speed: -1.0,
                ..VideoLayerState::default()
            },
            Duration::from_millis(100),
            None,
            Some(2_000),
        );

        assert_eq!(forward.position_ms, 2_000);
        assert!(!forward.playing);
        assert_eq!(reverse.position_ms, 0);
        assert!(!reverse.playing);
    }

    #[test]
    fn bpm_sync_derives_speed_from_loop_region_and_clock() {
        let state = VideoLayerState {
            playing: true,
            speed: 1.0,
            loop_enabled: true,
            loop_start_ms: 0,
            loop_end_ms: 2_000,
            bpm_sync: protocol::VideoBpmSync {
                enabled: true,
                ratio: 1.0,
                loop_bars: 1.0,
            },
            ..VideoLayerState::default()
        };

        assert_eq!(effective_speed(&state, Some(120.0)), 1.0);

        let doubled = VideoLayerState {
            bpm_sync: protocol::VideoBpmSync {
                enabled: true,
                ratio: 2.0,
                loop_bars: 1.0,
            },
            ..state.clone()
        };
        assert_eq!(effective_speed(&doubled, Some(120.0)), 2.0);

        let advanced =
            advance_layer_state_with_bpm(doubled, Duration::from_millis(100), Some(120.0));
        assert_eq!(advanced.position_ms, 200);
    }

    #[test]
    fn bpm_sync_can_lock_position_to_shared_clock_phase() {
        let state = VideoLayerState {
            playing: true,
            speed: 1.0,
            loop_enabled: true,
            loop_start_ms: 1_000,
            loop_end_ms: 5_000,
            bpm_sync: protocol::VideoBpmSync {
                enabled: true,
                ratio: 1.0,
                loop_bars: 1.0,
            },
            ..VideoLayerState::default()
        };
        let clock = ClockSnapshot {
            bpm: 120.0,
            beat_phase: 0.0,
            beat_counter: 1,
            tap_count: 0,
            source: protocol::ClockSource::AbletonLink,
            ..ClockSnapshot::default()
        };

        let advanced =
            advance_layer_state_with_clock_and_duration(state, Duration::ZERO, Some(&clock), None);

        assert_eq!(advanced.position_ms, 2_000);
    }

    #[test]
    fn frame_queue_keeps_sorted_frames_and_drops_oldest_slot_when_full() {
        let mut queue = FrameQueue::new(7, 3);

        assert_eq!(queue.push(frame(99, 40)), FrameQueuePush::Inserted);
        assert_eq!(queue.push(frame(99, 10)), FrameQueuePush::Inserted);
        assert_eq!(queue.push(frame(99, 25)), FrameQueuePush::Inserted);
        assert_eq!(queue.push(frame(99, 55)), FrameQueuePush::DroppedOldest);

        let pts = queue.iter().map(|frame| frame.pts_ms).collect::<Vec<_>>();
        assert_eq!(pts, vec![25, 40, 55]);
        assert_eq!(queue.nearest(42).map(|frame| frame.pts_ms), Some(40));
        assert_eq!(
            queue.frame_at_or_before(42).map(|frame| frame.pts_ms),
            Some(40)
        );
        assert!(queue.iter().all(|frame| frame.layer_id == 7));
    }

    #[test]
    fn frame_queue_replaces_existing_pts_without_growing() {
        let mut queue = FrameQueue::new(7, 2);
        let mut first = frame(99, 40);
        first.data = vec![1; 16];
        let mut replacement = frame(99, 40);
        replacement.data = vec![2; 16];

        assert_eq!(queue.push(first), FrameQueuePush::Inserted);
        assert_eq!(queue.push(replacement), FrameQueuePush::Inserted);

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.nearest(40).unwrap().data, vec![2; 16]);
    }

    #[test]
    fn frame_queue_can_drop_stale_frames_before_playhead() {
        let mut queue = FrameQueue::new(1, 4);

        for pts in [0, 16, 32, 48] {
            queue.push(frame(1, pts));
        }

        assert_eq!(queue.drop_before(32), 2);
        let pts = queue.iter().map(|frame| frame.pts_ms).collect::<Vec<_>>();
        assert_eq!(pts, vec![32, 48]);
    }

    #[test]
    fn video_runtime_syncs_queues_and_composes_selected_frames() {
        let mut runtime = VideoRuntime::new(VideoRuntimeConfig {
            frame_queue_capacity: 4,
            preview_width: 1,
            preview_height: 1,
        });
        runtime.sync_layers(&[1, 2]);
        runtime.push_frame(rgba_frame(1, [100, 0, 0, 255]));
        runtime.push_frame(VideoFrame {
            pts_ms: 10,
            ..rgba_frame(2, [0, 100, 0, 255])
        });
        runtime.push_frame(VideoFrame {
            pts_ms: 30,
            ..rgba_frame(2, [0, 200, 0, 255])
        });

        let output = runtime
            .compose_plan(
                &plan(vec![
                    layer_plan(1, VideoBlendMode::Normal, 1.0),
                    CompositionLayerPlan {
                        position_ms: 20,
                        ..layer_plan(2, VideoBlendMode::Add, 1.0)
                    },
                ]),
                1,
                1,
            )
            .unwrap();

        assert_eq!(runtime.queue_count(), 2);
        assert_eq!(runtime.queue_len(2), 2);
        assert_eq!(output.data, vec![100, 100, 0, 255]);

        runtime.sync_layers(&[2]);
        assert_eq!(runtime.queue_count(), 1);
        assert_eq!(runtime.queue_len(1), 0);
        assert_eq!(runtime.queue_len(2), 2);
    }

    #[test]
    fn video_preview_renderer_uses_still_image_pixels() {
        let path = std::env::temp_dir().join(format!(
            "syndocal-preview-still-{}-{}.png",
            std::process::id(),
            1
        ));
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([12, 34, 56, 255]));
        image.save(&path).unwrap();
        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 12,
                label: "Still".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::StillImage,
                    path: Some(path.to_string_lossy().to_string()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };

        let mut renderer = VideoPreviewRenderer::new(VideoRuntimeConfig {
            frame_queue_capacity: 2,
            preview_width: 1,
            preview_height: 1,
        });
        let output = renderer.render(&snapshot, 1, 1).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(output.data, vec![12, 34, 56, 255]);
        assert_eq!(renderer.queue_count(), 1);
        assert_eq!(renderer.queue_len(12), 1);
        assert_eq!(renderer.still_image_cache_len(), 1);
    }

    #[test]
    fn video_preview_renderer_uses_debug_frame_for_non_still_sources() {
        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 3,
                label: "Clip".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("clip.mp4".to_string()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    position_ms: 160,
                    ..VideoLayerState::default()
                },
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };

        let mut renderer = VideoPreviewRenderer::new(VideoRuntimeConfig {
            frame_queue_capacity: 2,
            preview_width: 1,
            preview_height: 1,
        });
        let output = renderer.render(&snapshot, 1, 1).unwrap();
        let expected = debug_solid_frame_for_layer(3, 160, 1, 1);

        assert_eq!(output.data, expected.data);
        assert_eq!(renderer.queue_count(), 1);
        assert_eq!(renderer.queue_len(3), 1);
        assert_eq!(renderer.still_image_cache_len(), 0);
    }

    #[test]
    fn video_preview_renderer_reports_invalid_size_and_missing_still_path() {
        let mut renderer = VideoPreviewRenderer::new(VideoRuntimeConfig::default());
        assert_eq!(
            renderer.render(&VideoSnapshot::default(), 0, 1),
            Err(VideoPreviewError::InvalidSize)
        );

        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 8,
                label: "Missing".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::StillImage,
                    path: None,
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };

        assert_eq!(
            renderer.render(&snapshot, 1, 1),
            Err(VideoPreviewError::MissingStillImagePath {
                layer_id: 8,
                label: "Missing".to_string(),
            })
        );
    }

    #[test]
    fn video_preview_renderer_accepts_swappable_frame_provider() {
        #[derive(Default)]
        struct TestFrameProvider {
            retained: Vec<Vec<VideoLayerId>>,
            requested: Vec<(VideoLayerId, u32, u32)>,
        }

        impl VideoFrameProvider for TestFrameProvider {
            fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
                self.retained.push(layer_ids.to_vec());
            }

            fn frame_for_layer(
                &mut self,
                layer: &VideoLayerSummary,
                width: u32,
                height: u32,
            ) -> Result<VideoFrame, VideoFrameProviderError> {
                self.requested.push((layer.id, width, height));
                Ok(VideoFrame {
                    layer_id: layer.id,
                    width,
                    height,
                    pts_ms: layer.state.position_ms,
                    duration_ms: 16,
                    format: VideoPixelFormat::Rgba8,
                    data: [layer.id as u8, width as u8, height as u8, 255]
                        .repeat(width as usize * height as usize),
                })
            }
        }

        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 9,
                label: "Provided".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("provided.mp4".to_string()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 2,
                preview_width: 2,
                preview_height: 1,
            },
            TestFrameProvider::default(),
        );
        let output = renderer.render(&snapshot, 2, 1).unwrap();

        assert_eq!(output.data, vec![9, 2, 1, 255, 9, 2, 1, 255]);
        assert_eq!(renderer.frame_provider().retained, vec![vec![9]]);
        assert_eq!(renderer.frame_provider().requested, vec![(9, 2, 1)]);
    }

    #[test]
    fn video_preview_renderer_applies_and_caches_isf_shader() {
        if IsfGpuRuntime::new().is_err() {
            eprintln!("ISF renderer integration test skipped: no GPU adapter");
            return;
        }
        const SOURCE: &str = r#"/*{
          "INPUTS": [
            {"NAME":"inputImage","TYPE":"image"},
            {"NAME":"level","TYPE":"float","DEFAULT":0.5,"MIN":0.0,"MAX":1.0}
          ]
        }*/
        void main() {
          vec4 pixel = IMG_THIS_PIXEL(inputImage);
          float luma = (pixel.r + pixel.g + pixel.b) / 3.0;
          gl_FragColor = (luma > level) ? pixel : vec4(0.0, 0.0, 0.0, 1.0);
        }"#;

        struct IsfFrameProvider;
        impl VideoFrameProvider for IsfFrameProvider {
            fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

            fn frame_for_layer(
                &mut self,
                layer: &VideoLayerSummary,
                _width: u32,
                _height: u32,
            ) -> Result<VideoFrame, VideoFrameProviderError> {
                Ok(VideoFrame {
                    layer_id: layer.id,
                    width: 2,
                    height: 1,
                    pts_ms: 1_000,
                    duration_ms: 33,
                    format: VideoPixelFormat::Rgba8,
                    data: vec![64, 64, 64, 255, 200, 200, 200, 255],
                })
            }
        }

        let prepared = prepare_isf_shader(SOURCE).unwrap();
        let effect =
            isf_effect_from_prepared("Threshold".to_string(), SOURCE.to_string(), None, &prepared);
        let mut snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 44,
                label: "ISF Layer".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("isf-test.mp4".to_string()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
                isf_effect: Some(effect),
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 2,
                preview_width: 2,
                preview_height: 1,
            },
            IsfFrameProvider,
        );

        let first = renderer.render(&snapshot, 2, 1).unwrap();
        let second = renderer.render(&snapshot, 2, 1).unwrap();

        assert_eq!(first.data, vec![0, 0, 0, 255, 200, 200, 200, 255]);
        assert_eq!(second.data, first.data);
        assert_eq!(renderer.isf_shader_cache_count(), 1);
        assert_eq!(renderer.isf_pipeline_count(), 1);
        assert_eq!(renderer.isf_last_stack_stage_count(), 1);
        assert_eq!(renderer.last_isf_error(), None);

        let invert = builtin_isf_effect("invert").unwrap().unwrap();
        snapshot.layers[0].isf_effect.as_mut().unwrap().stack.push(
            protocol::VideoIsfEffectStageSummary {
                enabled: invert.enabled,
                label: invert.label,
                source: invert.source,
                source_path: invert.source_path,
                description: invert.description,
                categories: invert.categories,
                controls: invert.controls,
            },
        );
        let stacked = renderer.render(&snapshot, 2, 1).unwrap();
        assert_eq!(stacked.data, vec![255, 255, 255, 255, 55, 55, 55, 255]);
        assert_eq!(renderer.isf_shader_cache_count(), 2);
        assert_eq!(renderer.isf_pipeline_count(), 2);
        assert_eq!(renderer.isf_last_stack_stage_count(), 2);
        assert!(renderer.isf_last_stack_render_us() > 0);

        snapshot.layers[0].isf_effect.as_mut().unwrap().enabled = false;
        let root_bypassed = renderer.render(&snapshot, 2, 1).unwrap();
        assert_eq!(
            root_bypassed.data,
            vec![191, 191, 191, 255, 55, 55, 55, 255]
        );
        assert_eq!(renderer.isf_last_stack_stage_count(), 1);

        snapshot.layers[0].isf_effect.as_mut().unwrap().enabled = true;
        let invert = builtin_isf_effect("invert").unwrap().unwrap();
        snapshot.layers[0].isf_effect.as_mut().unwrap().stack.push(
            protocol::VideoIsfEffectStageSummary {
                enabled: invert.enabled,
                label: invert.label,
                source: invert.source,
                source_path: invert.source_path,
                description: invert.description,
                categories: invert.categories,
                controls: invert.controls,
            },
        );
        let three_stage = renderer.render(&snapshot, 2, 1).unwrap();
        assert_eq!(three_stage.data, first.data);
        assert_eq!(renderer.isf_shader_cache_count(), 2);
        assert_eq!(renderer.isf_pipeline_count(), 2);
        assert_eq!(renderer.isf_last_stack_stage_count(), 3);
    }

    #[test]
    fn video_preview_renderer_fails_open_for_invalid_isf_shader() {
        struct SolidFrameProvider;
        impl VideoFrameProvider for SolidFrameProvider {
            fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

            fn frame_for_layer(
                &mut self,
                layer: &VideoLayerSummary,
                _width: u32,
                _height: u32,
            ) -> Result<VideoFrame, VideoFrameProviderError> {
                Ok(rgba_frame(layer.id, [24, 96, 180, 255]))
            }
        }

        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 45,
                label: "Broken ISF".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("broken-isf.mp4".to_string()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
                isf_effect: Some(protocol::VideoIsfEffectSummary {
                    enabled: true,
                    label: "Invalid".to_string(),
                    source: "not an ISF shader".into(),
                    source_path: None,
                    description: None,
                    categories: Vec::new(),
                    controls: Vec::new(),
                    stack: Vec::new(),
                }),
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 2,
                preview_width: 1,
                preview_height: 1,
            },
            SolidFrameProvider,
        );

        let output = renderer.render(&snapshot, 1, 1).unwrap();

        assert_eq!(output.data, vec![24, 96, 180, 255]);
        assert_eq!(renderer.isf_shader_cache_count(), 1);
        assert!(renderer
            .last_isf_error()
            .is_some_and(|error| error.contains("Broken ISF")));
    }

    #[test]
    fn decoder_backed_provider_uses_decoder_for_file_sources() {
        #[derive(Default)]
        struct TestDecoder {
            retained: Vec<Vec<VideoLayerId>>,
            requests: Vec<VideoFrameRequest>,
        }

        impl VideoFrameDecoder for TestDecoder {
            fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
                self.retained.push(layer_ids.to_vec());
            }

            fn decode_frame(
                &mut self,
                request: &VideoFrameRequest,
            ) -> Result<Option<VideoFrame>, VideoDecodeError> {
                self.requests.push(request.clone());
                Ok(Some(rgba_frame(request.layer_id, [30, 60, 90, 255])))
            }
        }

        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 11,
                label: "Clip".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    position_ms: 250,
                    ..VideoLayerState::default()
                },
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };

        let mut renderer = VideoPreviewRenderer::with_decoder(
            VideoRuntimeConfig {
                frame_queue_capacity: 2,
                preview_width: 1,
                preview_height: 1,
            },
            TestDecoder::default(),
        );
        let output = renderer.render(&snapshot, 1, 1).unwrap();
        let provider = renderer.frame_provider();

        assert_eq!(output.data, vec![30, 60, 90, 255]);
        assert_eq!(provider.decoder().retained, vec![vec![11]]);
        assert_eq!(provider.decoder().requests.len(), 1);
        assert_eq!(provider.decoder().requests[0].position_ms, 250);
        assert_eq!(provider.decoder().requests[0].width, 1);
        assert_eq!(provider.decoder().requests[0].height, 1);
    }

    #[test]
    fn decoder_backed_provider_prefetches_lookahead_frames_into_runtime_queue() {
        #[derive(Default)]
        struct TestDecoder {
            requests: Vec<VideoFrameRequest>,
        }

        impl VideoFrameDecoder for TestDecoder {
            fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

            fn decode_frame(
                &mut self,
                request: &VideoFrameRequest,
            ) -> Result<Option<VideoFrame>, VideoDecodeError> {
                self.requests.push(request.clone());
                Ok(Some(VideoFrame {
                    layer_id: request.layer_id,
                    width: request.width,
                    height: request.height,
                    pts_ms: request.position_ms,
                    duration_ms: 16,
                    format: VideoPixelFormat::Rgba8,
                    data: [request.position_ms as u8, 0, 0, 255]
                        .repeat(request.width as usize * request.height as usize),
                }))
            }
        }

        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 12,
                label: "Prefetch Clip".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    position_ms: 100,
                    ..VideoLayerState::default()
                },
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let provider = DecoderBackedFrameProvider::new(TestDecoder::default()).with_prefetch(2, 40);
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 4,
                preview_width: 1,
                preview_height: 1,
            },
            provider,
        );
        let output = renderer.render(&snapshot, 1, 1).unwrap();
        let provider = renderer.frame_provider();

        assert_eq!(renderer.queue_len(12), 3);
        assert_eq!(output.pts_ms, 100);
        assert_eq!(
            provider
                .decoder()
                .requests
                .iter()
                .map(|request| request.position_ms)
                .collect::<Vec<_>>(),
            vec![100, 140, 180]
        );
        assert_eq!(provider.prefetch_count(), 2);
        assert_eq!(provider.prefetch_interval_ms(), 40);
    }

    #[test]
    fn decoder_backed_provider_prefetch_respects_reverse_loop_direction() {
        #[derive(Default)]
        struct TestDecoder {
            requests: Vec<VideoFrameRequest>,
        }

        impl VideoFrameDecoder for TestDecoder {
            fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

            fn decode_frame(
                &mut self,
                request: &VideoFrameRequest,
            ) -> Result<Option<VideoFrame>, VideoDecodeError> {
                self.requests.push(request.clone());
                Ok(Some(VideoFrame {
                    layer_id: request.layer_id,
                    width: request.width,
                    height: request.height,
                    pts_ms: request.position_ms,
                    duration_ms: 16,
                    format: VideoPixelFormat::Rgba8,
                    data: [request.position_ms as u8, 0, 0, 255]
                        .repeat(request.width as usize * request.height as usize),
                }))
            }
        }

        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 13,
                label: "Reverse Loop Clip".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: Some(VideoMediaMetadata {
                        duration_ms: Some(1_000),
                        width: Some(1920),
                        height: Some(1080),
                        frame_rate: Some(60.0),
                        has_audio: false,
                    }),
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    position_ms: 110,
                    speed: -1.0,
                    loop_enabled: true,
                    loop_start_ms: 100,
                    loop_end_ms: 200,
                    ..VideoLayerState::default()
                },
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let provider = DecoderBackedFrameProvider::new(TestDecoder::default()).with_prefetch(3, 40);
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 4,
                preview_width: 1,
                preview_height: 1,
            },
            provider,
        );
        let output = renderer.render(&snapshot, 1, 1).unwrap();
        let provider = renderer.frame_provider();

        assert_eq!(renderer.queue_len(13), 4);
        assert_eq!(output.pts_ms, 110);
        assert_eq!(
            provider
                .decoder()
                .requests
                .iter()
                .map(|request| request.position_ms)
                .collect::<Vec<_>>(),
            vec![110, 170, 130, 190]
        );
    }

    #[test]
    fn decoder_backed_provider_prefetch_uses_bpm_sync_when_manual_speed_is_zero() {
        #[derive(Default)]
        struct TestDecoder {
            requests: Vec<VideoFrameRequest>,
        }

        impl VideoFrameDecoder for TestDecoder {
            fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

            fn decode_frame(
                &mut self,
                request: &VideoFrameRequest,
            ) -> Result<Option<VideoFrame>, VideoDecodeError> {
                self.requests.push(request.clone());
                Ok(Some(VideoFrame {
                    layer_id: request.layer_id,
                    width: request.width,
                    height: request.height,
                    pts_ms: request.position_ms,
                    duration_ms: 16,
                    format: VideoPixelFormat::Rgba8,
                    data: [request.position_ms as u8, 0, 0, 255]
                        .repeat(request.width as usize * request.height as usize),
                }))
            }
        }

        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 14,
                label: "BPM Loop Clip".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: Some(VideoMediaMetadata {
                        duration_ms: Some(4_000),
                        width: Some(1920),
                        height: Some(1080),
                        frame_rate: Some(60.0),
                        has_audio: false,
                    }),
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    position_ms: 0,
                    speed: 0.0,
                    loop_enabled: true,
                    loop_start_ms: 0,
                    loop_end_ms: 2_000,
                    bpm_sync: protocol::VideoBpmSync {
                        enabled: true,
                        ratio: 1.0,
                        loop_bars: 1.0,
                    },
                    ..VideoLayerState::default()
                },
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let provider = DecoderBackedFrameProvider::new(TestDecoder::default())
            .with_prefetch(2, 100)
            .with_bpm(Some(120.0));
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 4,
                preview_width: 1,
                preview_height: 1,
            },
            provider,
        );
        let output = renderer.render(&snapshot, 1, 1).unwrap();
        let provider = renderer.frame_provider();

        assert_eq!(renderer.queue_len(14), 3);
        assert_eq!(output.pts_ms, 0);
        assert_eq!(provider.bpm(), Some(120.0));
        assert_eq!(
            provider
                .decoder()
                .requests
                .iter()
                .map(|request| request.position_ms)
                .collect::<Vec<_>>(),
            vec![0, 100, 200]
        );
    }

    #[test]
    fn preview_prefetch_positions_report_decoder_plan() {
        let mut layer = VideoLayerSummary {
            id: 15,
            label: "Plan Clip".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some("clip.mp4".to_string()),
                name: None,
                codec: Some("H264".to_string()),
                metadata: Some(VideoMediaMetadata {
                    duration_ms: Some(1_000),
                    width: Some(1920),
                    height: Some(1080),
                    frame_rate: Some(60.0),
                    has_audio: false,
                }),
            },
            media_asset_id: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState {
                position_ms: 110,
                speed: -1.0,
                loop_enabled: true,
                loop_start_ms: 100,
                loop_end_ms: 200,
                ..VideoLayerState::default()
            },
            isf_effect: None,
        };

        assert_eq!(
            preview_prefetch_positions_ms(&layer, 3, 40, None),
            vec![110, 170, 130, 190]
        );

        layer.source.kind = VideoSourceKind::StillImage;
        layer.source.path = Some("still.png".to_string());

        assert_eq!(
            preview_prefetch_positions_ms(&layer, 3, 40, None),
            vec![110]
        );
    }

    fn decode_request(layer_id: VideoLayerId, position_ms: u64) -> VideoFrameRequest {
        VideoFrameRequest {
            layer_id,
            label: format!("Layer {layer_id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(format!("clip-{layer_id}.mp4")),
                name: None,
                codec: Some("H264".to_string()),
                metadata: None,
            },
            position_ms,
            width: 16,
            height: 9,
        }
    }

    #[test]
    fn decode_scheduler_prioritizes_current_frames_and_deduplicates_requests() {
        let mut scheduler = VideoDecodeScheduler::new(4);
        assert_eq!(
            scheduler.push(decode_request(1, 100), VideoDecodePriority::Lookahead),
            VideoDecodeSchedulePush::Inserted
        );
        assert_eq!(
            scheduler.push(decode_request(1, 133), VideoDecodePriority::Lookahead),
            VideoDecodeSchedulePush::Inserted
        );
        assert_eq!(
            scheduler.push(decode_request(1, 100), VideoDecodePriority::Current),
            VideoDecodeSchedulePush::Reprioritized
        );
        assert_eq!(
            scheduler.push(decode_request(1, 100), VideoDecodePriority::Current),
            VideoDecodeSchedulePush::Duplicate
        );

        assert_eq!(scheduler.len(), 2);
        let first = scheduler.pop_next().unwrap();
        let second = scheduler.pop_next().unwrap();
        assert_eq!(first.priority, VideoDecodePriority::Current);
        assert_eq!(first.request.position_ms, 100);
        assert_eq!(second.priority, VideoDecodePriority::Lookahead);
        assert_eq!(second.request.position_ms, 133);
        assert!(scheduler.is_empty());
    }

    #[test]
    fn decode_scheduler_drops_lower_priority_when_capacity_is_full() {
        let mut scheduler = VideoDecodeScheduler::new(2);
        assert_eq!(
            scheduler.push(decode_request(1, 10), VideoDecodePriority::Background),
            VideoDecodeSchedulePush::Inserted
        );
        assert_eq!(
            scheduler.push(decode_request(1, 20), VideoDecodePriority::Lookahead),
            VideoDecodeSchedulePush::Inserted
        );
        assert_eq!(
            scheduler.push(decode_request(1, 30), VideoDecodePriority::Current),
            VideoDecodeSchedulePush::DroppedLowerPriority
        );
        assert_eq!(
            scheduler.push(decode_request(1, 40), VideoDecodePriority::Background),
            VideoDecodeSchedulePush::RejectedFull
        );

        let pending = scheduler.pending();
        assert_eq!(
            pending
                .iter()
                .map(|job| (job.priority, job.request.position_ms))
                .collect::<Vec<_>>(),
            vec![
                (VideoDecodePriority::Current, 30),
                (VideoDecodePriority::Lookahead, 20)
            ]
        );
    }

    #[test]
    fn decode_scheduler_can_enqueue_layer_preview_plan_and_retain_layers() {
        let layer = VideoLayerSummary {
            id: 21,
            label: "Scheduled Clip".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some("scheduled.mp4".to_string()),
                name: None,
                codec: Some("H264".to_string()),
                metadata: None,
            },
            media_asset_id: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState {
                position_ms: 200,
                playing: true,
                ..VideoLayerState::default()
            },
            isf_effect: None,
        };
        let mut scheduler = VideoDecodeScheduler::new(5);

        let pushes = scheduler.push_layer_preview(&layer, 32, 18, 2, 50, None);

        assert_eq!(
            pushes,
            vec![
                VideoDecodeSchedulePush::Inserted,
                VideoDecodeSchedulePush::Inserted,
                VideoDecodeSchedulePush::Inserted
            ]
        );
        assert_eq!(
            scheduler
                .pending()
                .iter()
                .map(|job| (job.priority, job.request.position_ms, job.request.width))
                .collect::<Vec<_>>(),
            vec![
                (VideoDecodePriority::Current, 200, 32),
                (VideoDecodePriority::Lookahead, 250, 32),
                (VideoDecodePriority::Lookahead, 300, 32)
            ]
        );

        scheduler.retain_layers(&[99]);
        assert!(scheduler.is_empty());
    }

    fn scheduled_file_layer(layer_id: VideoLayerId, position_ms: u64) -> VideoLayerSummary {
        VideoLayerSummary {
            id: layer_id,
            label: format!("Layer {layer_id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(format!("clip-{layer_id}.mp4")),
                name: None,
                codec: Some("H264".to_string()),
                metadata: Some(VideoMediaMetadata {
                    duration_ms: Some(1_000),
                    width: Some(1920),
                    height: Some(1080),
                    frame_rate: Some(60.0),
                    has_audio: false,
                }),
            },
            media_asset_id: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState {
                position_ms,
                playing: true,
                ..VideoLayerState::default()
            },
            isf_effect: None,
        }
    }

    fn scheduled_still_layer(layer_id: VideoLayerId) -> VideoLayerSummary {
        VideoLayerSummary {
            id: layer_id,
            label: format!("Still {layer_id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::StillImage,
                path: Some(format!("still-{layer_id}.png")),
                name: None,
                codec: None,
                metadata: None,
            },
            media_asset_id: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
            isf_effect: None,
        }
    }

    fn decode_schedule_snapshot(output_blackout: bool) -> VideoSnapshot {
        VideoSnapshot {
            layers: vec![
                scheduled_file_layer(31, 100),
                scheduled_still_layer(32),
                VideoLayerSummary {
                    state: VideoLayerState {
                        enabled: false,
                        ..VideoLayerState::default()
                    },
                    ..scheduled_file_layer(33, 200)
                },
            ],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: vec![CompositionSummary {
                id: 7,
                label: "Program".to_string(),
                layer_ids: vec![31, 32, 33],
                output_ids: vec![9],
            }],
            outputs: vec![VideoOutputSummary {
                id: 9,
                label: "Projector".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 7,
                fullscreen: false,
                monitor_id: Some(0),
                monitor_identity: None,
                width: 1280,
                height: 720,
                endpoint_name: None,
                opacity: 1.0,
                blackout: output_blackout,
                mapping: VideoOutputMapping::default(),
            }],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        }
    }

    #[test]
    fn decode_scheduler_can_enqueue_visible_composition_preview_plan() {
        let snapshot = decode_schedule_snapshot(false);
        let mut scheduler = VideoDecodeScheduler::new(8);

        let report = scheduler
            .push_composition_preview(&snapshot, Some(7), 64, 36, 2, 40, None)
            .unwrap();

        assert_eq!(
            report,
            VideoDecodeEnqueueReport {
                layers_considered: 2,
                requests_attempted: 3,
                inserted: 3,
                pending: 3,
                ..VideoDecodeEnqueueReport::default()
            }
        );
        assert_eq!(
            scheduler
                .pending()
                .iter()
                .map(|job| (
                    job.priority,
                    job.request.layer_id,
                    job.request.position_ms,
                    job.request.width
                ))
                .collect::<Vec<_>>(),
            vec![
                (VideoDecodePriority::Current, 31, 100, 64),
                (VideoDecodePriority::Lookahead, 31, 140, 64),
                (VideoDecodePriority::Lookahead, 31, 180, 64),
            ]
        );

        let duplicate_report = scheduler
            .push_composition_preview(&snapshot, Some(7), 64, 36, 2, 40, None)
            .unwrap();
        assert_eq!(
            duplicate_report,
            VideoDecodeEnqueueReport {
                layers_considered: 2,
                requests_attempted: 3,
                duplicate: 3,
                pending: 3,
                ..VideoDecodeEnqueueReport::default()
            }
        );
    }

    #[test]
    fn decode_worker_can_enqueue_output_preview_and_skip_blacked_outputs() {
        let snapshot = decode_schedule_snapshot(false);
        let mut worker = VideoDecodeWorker::new(8, WorkerTestDecoder::default());
        let mut runtime = VideoRuntime::new(VideoRuntimeConfig {
            frame_queue_capacity: 4,
            preview_width: 1,
            preview_height: 1,
        });

        let enqueue = worker
            .enqueue_output_preview(&snapshot, 9, 32, 18, 1, 50, None)
            .unwrap();
        assert_eq!(
            enqueue,
            VideoDecodeEnqueueReport {
                layers_considered: 2,
                requests_attempted: 2,
                inserted: 2,
                pending: 2,
                ..VideoDecodeEnqueueReport::default()
            }
        );

        let decode = worker.decode_budget_into_runtime(&mut runtime, 8);
        assert_eq!(decode.decoded, 2);
        assert_eq!(runtime.queue_len(31), 2);
        assert_eq!(
            worker
                .decoder()
                .requests
                .iter()
                .map(|request| (request.layer_id, request.position_ms, request.width))
                .collect::<Vec<_>>(),
            vec![(31, 100, 32), (31, 150, 32)]
        );

        let blacked = decode_schedule_snapshot(true);
        let empty = worker
            .enqueue_output_preview(&blacked, 9, 32, 18, 1, 50, None)
            .unwrap();
        assert_eq!(
            empty,
            VideoDecodeEnqueueReport {
                pending: 0,
                ..VideoDecodeEnqueueReport::default()
            }
        );
    }

    #[derive(Default)]
    struct WorkerTestDecoder {
        retained: Vec<Vec<VideoLayerId>>,
        requests: Vec<VideoFrameRequest>,
    }

    impl VideoFrameDecoder for WorkerTestDecoder {
        fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
            self.retained.push(layer_ids.to_vec());
        }

        fn decode_frame(
            &mut self,
            request: &VideoFrameRequest,
        ) -> Result<Option<VideoFrame>, VideoDecodeError> {
            self.requests.push(request.clone());
            if request.position_ms == 777 {
                return Ok(None);
            }
            if request.position_ms == 999 {
                return Err(VideoDecodeError::Decode {
                    layer_id: request.layer_id,
                    label: request.label.clone(),
                    message: "decode failed".to_string(),
                });
            }
            Ok(Some(VideoFrame {
                layer_id: 9_999,
                width: request.width,
                height: request.height,
                pts_ms: request.position_ms,
                duration_ms: 16,
                format: VideoPixelFormat::Rgba8,
                data: [request.position_ms as u8, 0, 0, 255]
                    .repeat(request.width as usize * request.height as usize),
            }))
        }
    }

    #[test]
    fn preview_renderer_can_warm_output_decode_queue() {
        let snapshot = decode_schedule_snapshot(false);
        let provider =
            DecoderBackedFrameProvider::new(WorkerTestDecoder::default()).with_prefetch(1, 50);
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 4,
                preview_width: 32,
                preview_height: 18,
            },
            provider,
        );

        let warmup = renderer
            .warm_output_decode_queue(&snapshot, 9, 32, 18, 8)
            .unwrap();

        assert_eq!(warmup.enqueue.layers_considered, 2);
        assert_eq!(warmup.enqueue.requests_attempted, 2);
        assert_eq!(warmup.decode.decoded, 2);
        assert_eq!(warmup.decode.pending, 0);
        assert_eq!(renderer.queue_len(31), 2);
        assert_eq!(
            renderer
                .frame_provider()
                .decoder()
                .requests
                .iter()
                .map(|request| (request.layer_id, request.position_ms, request.width))
                .collect::<Vec<_>>(),
            vec![(31, 100, 32), (31, 150, 32)]
        );
    }

    #[test]
    fn decode_worker_decodes_priority_order_into_runtime() {
        let mut worker = VideoDecodeWorker::new(4, WorkerTestDecoder::default());
        let mut runtime = VideoRuntime::new(VideoRuntimeConfig {
            frame_queue_capacity: 4,
            preview_width: 1,
            preview_height: 1,
        });
        worker
            .scheduler_mut()
            .push(decode_request(1, 120), VideoDecodePriority::Lookahead);
        worker
            .scheduler_mut()
            .push(decode_request(1, 40), VideoDecodePriority::Current);

        let first = worker.decode_budget_into_runtime(&mut runtime, 1);

        assert_eq!(first.attempted, 1);
        assert_eq!(first.decoded, 1);
        assert_eq!(first.pending, 1);
        assert_eq!(runtime.queue_len(1), 1);
        assert_eq!(runtime.queue_len(9_999), 0);
        assert_eq!(worker.decoder().requests[0].position_ms, 40);

        let second = worker.decode_budget_into_runtime(&mut runtime, 8);

        assert_eq!(second.attempted, 1);
        assert_eq!(second.decoded, 1);
        assert_eq!(second.pending, 0);
        assert_eq!(runtime.queue_len(1), 2);
        assert_eq!(
            worker
                .decoder()
                .requests
                .iter()
                .map(|request| request.position_ms)
                .collect::<Vec<_>>(),
            vec![40, 120]
        );
    }

    #[test]
    fn decode_worker_reports_skips_and_errors_without_stopping_budget() {
        let mut worker = VideoDecodeWorker::new(4, WorkerTestDecoder::default());
        let mut runtime = VideoRuntime::new(VideoRuntimeConfig {
            frame_queue_capacity: 4,
            preview_width: 1,
            preview_height: 1,
        });
        worker
            .scheduler_mut()
            .push(decode_request(1, 10), VideoDecodePriority::Current);
        worker
            .scheduler_mut()
            .push(decode_request(1, 777), VideoDecodePriority::Lookahead);
        let error_key = protocol::VideoRenderInputKey {
            project_render_epoch: 4,
            input_id: protocol::VideoRenderInputId(12),
        };
        worker.scheduler_mut().push_input(
            VideoRenderInput::new(error_key, decode_request(1, 999)),
            VideoDecodePriority::Lookahead,
        );

        let report = worker.decode_budget_into_runtime(&mut runtime, 8);

        assert_eq!(report.attempted, 3);
        assert_eq!(report.decoded, 1);
        assert_eq!(report.skipped, 2);
        assert_eq!(report.pending, 0);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].input_key, error_key);
        assert_eq!(report.errors[0].position_ms, 999);
        assert_eq!(runtime.queue_len(1), 1);
    }

    #[test]
    fn decode_worker_retain_layers_prunes_scheduler_and_decoder_cache() {
        let mut worker = VideoDecodeWorker::new(4, WorkerTestDecoder::default());
        worker
            .scheduler_mut()
            .push(decode_request(1, 10), VideoDecodePriority::Current);
        worker
            .scheduler_mut()
            .push(decode_request(2, 20), VideoDecodePriority::Current);

        worker.retain_layers(&[2]);

        assert_eq!(
            worker
                .scheduler()
                .pending()
                .iter()
                .map(|job| job.request.layer_id)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(worker.decoder().retained, vec![vec![2]]);
    }

    #[test]
    fn decoder_backed_provider_can_surface_decoder_errors() {
        #[derive(Default)]
        struct ErrorDecoder;

        impl VideoFrameDecoder for ErrorDecoder {
            fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

            fn decode_frame(
                &mut self,
                request: &VideoFrameRequest,
            ) -> Result<Option<VideoFrame>, VideoDecodeError> {
                Err(VideoDecodeError::Decode {
                    layer_id: request.layer_id,
                    label: request.label.clone(),
                    message: "decode failed".to_string(),
                })
            }
        }

        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 4,
                label: "Broken".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("broken.mp4".to_string()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let provider =
            DecoderBackedFrameProvider::new(ErrorDecoder).with_placeholder_when_missing(false);
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 2,
                preview_width: 1,
                preview_height: 1,
            },
            provider,
        );

        assert_eq!(
            renderer.render(&snapshot, 1, 1),
            Err(VideoPreviewError::Decode {
                layer_id: 4,
                label: "Broken".to_string(),
                error: VideoDecodeError::Decode {
                    layer_id: 4,
                    label: "Broken".to_string(),
                    message: "decode failed".to_string(),
                },
            })
        );
    }

    #[test]
    fn ffmpeg_cli_decoder_reads_raw_rgba_from_command_and_caches() {
        let binary = fake_ffmpeg_binary();
        let mut decoder = FfmpegCliFrameDecoder::new(&binary);
        let request = VideoFrameRequest {
            layer_id: 14,
            label: "Clip".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some("ignored-by-fake-ffmpeg.mp4".to_string()),
                name: None,
                codec: Some("H264".to_string()),
                metadata: None,
            },
            position_ms: 250,
            width: 1,
            height: 1,
        };

        let first = decoder.decode_frame(&request).unwrap().unwrap();
        let _ = std::fs::remove_file(&binary);
        let second = decoder.decode_frame(&request).unwrap().unwrap();

        assert_eq!(decoder.binary(), binary.as_path());
        assert_eq!(decoder.cache_len(), 1);
        assert_eq!(first.data, b"ABCD");
        assert_eq!(first, second);
        assert_eq!(first.pts_ms, 250);
    }

    #[test]
    fn ffmpeg_cli_decoder_replaces_the_cached_frame_for_each_layer() {
        let binary = fake_ffmpeg_binary();
        let mut decoder = FfmpegCliFrameDecoder::new(&binary);
        let mut request = VideoFrameRequest {
            layer_id: 21,
            label: "Clip".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some("ignored-by-fake-ffmpeg.mp4".to_string()),
                name: None,
                codec: Some("H264".to_string()),
                metadata: None,
            },
            position_ms: 100,
            width: 1,
            height: 1,
        };

        decoder.decode_frame(&request).unwrap();
        request.position_ms = 200;
        decoder.decode_frame(&request).unwrap();
        let _ = std::fs::remove_file(&binary);

        assert_eq!(decoder.cache_len(), 1);
        assert_eq!(decoder.entries[0].layer_id, 21);
        assert_eq!(decoder.entries[0].position_ms, 200);
        assert_eq!(decoder.entries[0].frame.pts_ms, 200);
    }

    #[test]
    fn ffmpeg_cli_decoder_enforces_global_lru_capacity() {
        let binary = fake_ffmpeg_binary();
        let mut decoder = FfmpegCliFrameDecoder::new(&binary);
        let mut request = VideoFrameRequest {
            layer_id: 1,
            label: "Clip".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some("ignored-by-fake-ffmpeg.mp4".to_string()),
                name: None,
                codec: Some("H264".to_string()),
                metadata: None,
            },
            position_ms: 0,
            width: 1,
            height: 1,
        };

        for layer_id in 1..=FFMPEG_CLI_FRAME_CACHE_CAPACITY as VideoLayerId {
            request.layer_id = layer_id;
            request.position_ms = layer_id;
            decoder.decode_frame(&request).unwrap();
        }
        request.layer_id = 1;
        request.position_ms = 1;
        decoder.decode_frame(&request).unwrap();
        request.layer_id = 9;
        request.position_ms = 9;
        decoder.decode_frame(&request).unwrap();
        let _ = std::fs::remove_file(&binary);

        let cached_layer_ids = decoder
            .entries
            .iter()
            .map(|entry| entry.layer_id)
            .collect::<Vec<_>>();
        assert_eq!(decoder.cache_len(), FFMPEG_CLI_FRAME_CACHE_CAPACITY);
        assert_eq!(cached_layer_ids, vec![3, 4, 5, 6, 7, 8, 1, 9]);
    }

    #[test]
    fn ffmpeg_cli_decoder_evicts_invalid_non_file_and_pathless_sources() {
        let binary = fake_ffmpeg_binary();
        let mut decoder = FfmpegCliFrameDecoder::new(&binary);
        let mut request = VideoFrameRequest {
            layer_id: 22,
            label: "Clip".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some("ignored-by-fake-ffmpeg.mp4".to_string()),
                name: None,
                codec: Some("H264".to_string()),
                metadata: None,
            },
            position_ms: 100,
            width: 1,
            height: 1,
        };

        decoder.decode_frame(&request).unwrap();
        request.source.kind = VideoSourceKind::Ndi;
        assert_eq!(decoder.decode_frame(&request), Ok(None));
        assert_eq!(decoder.cache_len(), 0);

        request.source.kind = VideoSourceKind::File;
        decoder.decode_frame(&request).unwrap();
        request.source.path = Some("  ".to_string());
        assert_eq!(
            decoder.decode_frame(&request),
            Err(VideoDecodeError::MissingSourcePath {
                layer_id: 22,
                label: "Clip".to_string(),
            })
        );
        assert_eq!(decoder.cache_len(), 0);

        request.source.path = Some("ignored-by-fake-ffmpeg.mp4".to_string());
        decoder.decode_frame(&request).unwrap();
        request.width = 0;
        assert!(matches!(
            decoder.decode_frame(&request),
            Err(VideoDecodeError::Decode { .. })
        ));
        assert_eq!(decoder.cache_len(), 0);
        let _ = std::fs::remove_file(&binary);
    }

    #[test]
    fn ffmpeg_cli_decoder_defers_non_file_sources_to_provider_fallback() {
        let mut decoder = FfmpegCliFrameDecoder::new("ffmpeg");
        let request = VideoFrameRequest {
            layer_id: 15,
            label: "NDI".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::Ndi,
                path: None,
                name: Some("camera".to_string()),
                codec: None,
                metadata: None,
            },
            position_ms: 0,
            width: 1,
            height: 1,
        };

        assert_eq!(decoder.decode_frame(&request), Ok(None));
    }

    #[test]
    fn video_runtime_status_reports_cli_and_external_backend_availability() {
        let binary = fake_ffmpeg_binary();

        let status = video_runtime_status_with_binaries(&binary, &binary);

        let ffmpeg = status
            .backends
            .iter()
            .find(|backend| backend.id == "ffmpeg")
            .unwrap();
        assert_eq!(ffmpeg.state, VideoBackendState::Available);
        assert!(ffmpeg.detail.contains("ABCD"));
        let ffprobe = status
            .backends
            .iter()
            .find(|backend| backend.id == "ffprobe")
            .unwrap();
        assert_eq!(ffprobe.state, VideoBackendState::Available);
        let hap_ffmpeg = status
            .backends
            .iter()
            .find(|backend| backend.id == "hap_ffmpeg")
            .unwrap();
        assert_eq!(hap_ffmpeg.state, VideoBackendState::Available);
        assert!(hap_ffmpeg.detail.contains("decoder 'hap' is available"));
        let dxt_cpu = status
            .backends
            .iter()
            .find(|backend| backend.id == "dxt_cpu_reference")
            .unwrap();
        assert_eq!(dxt_cpu.state, VideoBackendState::Available);
        assert!(dxt_cpu.detail.contains("DXT1/DXT5"));
        let hap_gpu = status
            .backends
            .iter()
            .find(|backend| backend.id == "hap_gpu")
            .unwrap();
        assert_eq!(hap_gpu.state, VideoBackendState::Available);
        assert!(hap_gpu.detail.contains("Pure Rust MOV demux"));
        let ndi = status
            .backends
            .iter()
            .find(|backend| backend.id == "ndi")
            .unwrap();
        if cfg!(feature = "ndi") {
            assert_eq!(ndi.state, VideoBackendState::Available);
            assert!(ndi
                .detail
                .contains("routes initialize the runtime on demand"));
        } else {
            assert_eq!(ndi.state, VideoBackendState::NotBuilt);
            assert!(ndi.detail.contains("NDI SDK backend is not linked"));
        }

        let spout = status
            .backends
            .iter()
            .find(|backend| backend.id == "spout")
            .unwrap();
        if cfg!(all(
            feature = "spout",
            target_os = "windows",
            target_arch = "x86_64"
        )) {
            assert_eq!(spout.state, VideoBackendState::Available);
            assert!(spout.detail.contains("routes initialize"));
        } else if cfg!(target_os = "windows") {
            assert_eq!(spout.state, VideoBackendState::NotBuilt);
            assert!(spout.detail.contains("not linked"));
        } else {
            assert_eq!(spout.state, VideoBackendState::NotBuilt);
            assert!(spout.detail.contains("Windows-only"));
        }

        let syphon = status
            .backends
            .iter()
            .find(|backend| backend.id == "syphon")
            .unwrap();
        assert_eq!(syphon.state, VideoBackendState::NotBuilt);
        if cfg!(target_os = "macos") {
            assert!(syphon.detail.contains("not linked"));
        } else {
            assert!(syphon.detail.contains("macOS-only"));
        }
    }

    #[test]
    fn video_codec_can_be_inferred_from_hap_file_extensions() {
        assert_eq!(
            infer_video_codec_from_path("C:/media/loop.hap").as_deref(),
            Some("hap")
        );
        assert_eq!(
            infer_video_codec_from_path("C:/media/loop.HAPQ").as_deref(),
            Some("hap-q")
        );
        assert_eq!(infer_video_codec_from_path("C:/media/loop.mov"), None);
    }

    #[test]
    fn ffmpeg_decoder_list_detects_hap_decoder() {
        assert!(ffmpeg_decoders_list_contains(
            " V..... h264                 H.264\n V..... hap                  Vidvox Hap decoder\n",
            "hap",
        ));
        assert!(ffmpeg_decoders_list_contains(
            " V..... HAP                  Vidvox Hap decoder\n",
            "hap",
        ));
        assert!(!ffmpeg_decoders_list_contains(
            " A..... hap                  not a video decoder\n V..... h264                 H.264\n",
            "hap",
        ));
        assert!(!ffmpeg_decoders_list_contains(
            " V..... hapqa                different decoder\n",
            "hap",
        ));
    }

    #[test]
    fn video_runtime_status_reports_missing_cli_tools() {
        let missing =
            std::env::temp_dir().join(format!("syndocal-missing-ffmpeg-{}", std::process::id()));

        let status = video_runtime_status_with_binaries(&missing, &missing);

        assert_eq!(
            status
                .backends
                .iter()
                .find(|backend| backend.id == "ffmpeg")
                .unwrap()
                .state,
            VideoBackendState::Missing
        );
        assert_eq!(
            status
                .backends
                .iter()
                .find(|backend| backend.id == "ffprobe")
                .unwrap()
                .state,
            VideoBackendState::Missing
        );
        assert_eq!(
            status
                .backends
                .iter()
                .find(|backend| backend.id == "hap_ffmpeg")
                .unwrap()
                .state,
            VideoBackendState::Missing
        );
    }

    #[test]
    fn debug_solid_frame_is_stable_per_layer_and_timestamp() {
        let first = debug_solid_frame_for_layer(3, 160, 2, 1);
        let second = debug_solid_frame_for_layer(3, 160, 2, 1);
        let changed = debug_solid_frame_for_layer(3, 176, 2, 1);

        assert_eq!(first, second);
        assert_ne!(first.data, changed.data);
        assert_eq!(first.data.len(), 8);
    }

    #[test]
    fn still_image_loader_reads_png_as_rgba_frame() {
        let path = std::env::temp_dir().join(format!(
            "syndocal-still-image-{}-{}.png",
            std::process::id(),
            1
        ));
        let mut image = image::RgbaImage::new(2, 1);
        image.put_pixel(0, 0, image::Rgba([10, 20, 30, 255]));
        image.put_pixel(1, 0, image::Rgba([40, 50, 60, 128]));
        image.save(&path).unwrap();

        let frame = load_still_image_frame(9, &path, 123).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(frame.layer_id, 9);
        assert_eq!(frame.width, 2);
        assert_eq!(frame.height, 1);
        assert_eq!(frame.pts_ms, 123);
        assert_eq!(frame.format, VideoPixelFormat::Rgba8);
        assert_eq!(frame.data, vec![10, 20, 30, 255, 40, 50, 60, 128]);
    }

    #[test]
    fn still_image_metadata_probe_reads_dimensions_without_duration() {
        let path = std::env::temp_dir().join(format!(
            "syndocal-still-probe-{}-{}.png",
            std::process::id(),
            1
        ));
        let image = image::RgbaImage::from_pixel(3, 2, image::Rgba([12, 34, 56, 255]));
        image.save(&path).unwrap();

        let metadata = probe_still_image_metadata(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(metadata.width, Some(3));
        assert_eq!(metadata.height, Some(2));
        assert_eq!(metadata.duration_ms, None);
        assert_eq!(metadata.frame_rate, None);
    }

    #[test]
    fn still_image_cache_reuses_frames_and_invalidates_when_file_changes() {
        let path = std::env::temp_dir().join(format!(
            "syndocal-still-cache-{}-{}.png",
            std::process::id(),
            1
        ));
        let mut red = image::RgbaImage::new(1, 1);
        red.put_pixel(0, 0, image::Rgba([200, 0, 0, 255]));
        red.save(&path).unwrap();

        let mut cache = StillImageFrameCache::new();
        let first = cache.frame_for_layer(5, &path, 10, 1, 1).unwrap();
        let second = cache.frame_for_layer(5, &path, 20, 1, 1).unwrap();

        let mut green = image::RgbaImage::new(3, 1);
        for x in 0..3 {
            green.put_pixel(x, 0, image::Rgba([0, 200, 0, 255]));
        }
        green.save(&path).unwrap();
        let changed = cache.frame_for_layer(5, &path, 30, 1, 1).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(cache.len(), 1);
        assert_eq!(first.data, vec![200, 0, 0, 255]);
        assert_eq!(second.pts_ms, 20);
        assert_eq!(second.data, first.data);
        assert_eq!(changed.data, vec![0, 200, 0, 255]);
    }

    #[test]
    fn still_image_cache_can_retain_active_layers() {
        let path_a =
            std::env::temp_dir().join(format!("syndocal-still-cache-{}-a.png", std::process::id()));
        let path_b =
            std::env::temp_dir().join(format!("syndocal-still-cache-{}-b.png", std::process::id()));
        let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]));
        image.save(&path_a).unwrap();
        image.save(&path_b).unwrap();

        let mut cache = StillImageFrameCache::new();
        cache.frame_for_layer(1, &path_a, 0, 1, 1).unwrap();
        cache.frame_for_layer(2, &path_b, 0, 1, 1).unwrap();
        cache.retain_layers(&[2]);
        let _ = std::fs::remove_file(&path_a);
        let _ = std::fs::remove_file(&path_b);

        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn resize_rgba8_nearest_preserves_corner_samples() {
        let frame = VideoFrame {
            layer_id: 4,
            width: 2,
            height: 2,
            pts_ms: 77,
            duration_ms: 0,
            format: VideoPixelFormat::Rgba8,
            data: vec![10, 0, 0, 255, 20, 0, 0, 255, 30, 0, 0, 255, 40, 0, 0, 255],
        };

        let resized = resize_rgba8_nearest(&frame, 4, 4).unwrap();

        assert_eq!(resized.layer_id, 4);
        assert_eq!(resized.width, 4);
        assert_eq!(resized.height, 4);
        assert_eq!(&resized.data[0..4], &[10, 0, 0, 255]);
        assert_eq!(&resized.data[12..16], &[20, 0, 0, 255]);
        assert_eq!(&resized.data[48..52], &[30, 0, 0, 255]);
        assert_eq!(&resized.data[60..64], &[40, 0, 0, 255]);
    }

    #[test]
    fn external_video_io_plans_extract_routable_inputs_and_outputs() {
        let snapshot = VideoSnapshot {
            layers: vec![
                VideoLayerSummary {
                    id: 1,
                    label: "File".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::File,
                        path: Some("clip.mp4".to_string()),
                        name: None,
                        codec: None,
                        metadata: None,
                    },
                    media_asset_id: None,
                    clip_slots: Vec::new(),
                    default_clip_slot_id: None,
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState::default(),
                    isf_effect: None,
                },
                VideoLayerSummary {
                    id: 2,
                    label: "NDI Camera".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::Ndi,
                        path: None,
                        name: Some("Camera A".to_string()),
                        codec: None,
                        metadata: None,
                    },
                    media_asset_id: None,
                    clip_slots: Vec::new(),
                    default_clip_slot_id: None,
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        enabled: true,
                        ..VideoLayerState::default()
                    },
                    isf_effect: None,
                },
                VideoLayerSummary {
                    id: 3,
                    label: "Unnamed Spout".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::Spout,
                        path: None,
                        name: Some("  ".to_string()),
                        codec: None,
                        metadata: None,
                    },
                    media_asset_id: None,
                    clip_slots: Vec::new(),
                    default_clip_slot_id: None,
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState::default(),
                    isf_effect: None,
                },
            ],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: vec![
                VideoOutputSummary {
                    id: 10,
                    label: "Display".to_string(),
                    kind: VideoOutputKind::Display,
                    enabled: true,
                    composition_id: 1,
                    fullscreen: false,
                    monitor_id: None,
                    monitor_identity: None,
                    width: 1280,
                    height: 720,
                    endpoint_name: None,
                    opacity: 1.0,
                    blackout: false,
                    mapping: VideoOutputMapping::default(),
                },
                VideoOutputSummary {
                    id: 11,
                    label: "Program NDI".to_string(),
                    kind: VideoOutputKind::NdiSender,
                    enabled: true,
                    composition_id: 1,
                    fullscreen: false,
                    monitor_id: None,
                    monitor_identity: None,
                    width: 1920,
                    height: 1080,
                    endpoint_name: Some("Syndocal Program".to_string()),
                    opacity: 0.75,
                    blackout: false,
                    mapping: VideoOutputMapping::default(),
                },
                VideoOutputSummary {
                    id: 12,
                    label: "Syphon Fallback".to_string(),
                    kind: VideoOutputKind::SyphonServer,
                    enabled: false,
                    composition_id: 2,
                    fullscreen: false,
                    monitor_id: None,
                    monitor_identity: None,
                    width: 640,
                    height: 360,
                    endpoint_name: Some(" ".to_string()),
                    opacity: 1.0,
                    blackout: true,
                    mapping: VideoOutputMapping::default(),
                },
            ],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };

        let inputs = build_external_video_input_plans(&snapshot);
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].layer_id, 2);
        assert_eq!(inputs[0].backend_id, "ndi");
        assert_eq!(inputs[0].endpoint_name, "Camera A");
        assert!(inputs[0].enabled);

        let outputs = build_external_video_output_plans(&snapshot);
        assert_eq!(
            outputs
                .iter()
                .map(|output| {
                    (
                        output.output_id,
                        output.backend_id.as_str(),
                        output.endpoint_name.as_str(),
                        output.enabled,
                        output.blackout,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (11, "ndi", "Syndocal Program", true, false),
                (12, "syphon", "Syphon Fallback", false, true)
            ]
        );
        assert_eq!(
            external_video_source_backend_id(&VideoSourceKind::File),
            None
        );
        assert_eq!(
            external_video_source_backend_id(&VideoSourceKind::Camera),
            Some("camera")
        );
        assert_eq!(
            external_video_source_backend_id(&VideoSourceKind::ScreenCapture),
            Some("screen_capture")
        );
        assert_eq!(
            external_video_output_backend_id(&VideoOutputKind::SpoutSender),
            Some("spout")
        );
    }

    #[test]
    fn external_video_io_route_plans_include_backend_readiness() {
        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 2,
                label: "NDI Camera".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::Ndi,
                    path: None,
                    name: Some("Camera A".to_string()),
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    enabled: true,
                    ..VideoLayerState::default()
                },
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: vec![VideoOutputSummary {
                id: 11,
                label: "Spout Program".to_string(),
                kind: VideoOutputKind::SpoutSender,
                enabled: true,
                composition_id: 1,
                fullscreen: false,
                monitor_id: None,
                monitor_identity: None,
                width: 1920,
                height: 1080,
                endpoint_name: Some("Syndocal Stage".to_string()),
                opacity: 0.75,
                blackout: false,
                mapping: VideoOutputMapping::default(),
            }],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let status = VideoRuntimeStatus {
            backends: vec![
                VideoBackendStatus {
                    id: "ndi".to_string(),
                    label: "NDI input/output".to_string(),
                    state: VideoBackendState::NotBuilt,
                    detail: "NDI SDK backend is not linked in this build".to_string(),
                },
                VideoBackendStatus {
                    id: "spout".to_string(),
                    label: "Spout input/output".to_string(),
                    state: VideoBackendState::Available,
                    detail: "Spout backend loaded".to_string(),
                },
            ],
        };

        let plans = build_external_video_io_route_plans(&snapshot, &status);

        assert_eq!(plans.inputs.len(), 1);
        assert_eq!(plans.outputs.len(), 1);
        assert_eq!(
            plans.inputs[0].backend_state,
            Some(VideoBackendState::NotBuilt)
        );
        assert!(!plans.inputs[0].ready);
        assert!(!plans.inputs[0].live);
        assert!(plans.inputs[0]
            .issue
            .as_deref()
            .is_some_and(|issue| issue.contains("not linked")));

        assert_eq!(
            plans.outputs[0].backend_state,
            Some(VideoBackendState::Available)
        );
        assert!(plans.outputs[0].ready);
        assert!(plans.outputs[0].live);
        assert_eq!(
            plans.outputs[0].backend_detail.as_deref(),
            Some("Spout backend loaded")
        );
        assert!(plans.outputs[0].issue.is_none());
    }

    #[test]
    fn external_video_transport_runtime_syncs_ready_and_blocked_routes() {
        #[derive(Default)]
        struct RecordingTransportDriver {
            events: Vec<String>,
        }

        impl ExternalVideoTransportDriver for RecordingTransportDriver {
            fn start_route(
                &mut self,
                route: &ExternalVideoTransportRoute,
            ) -> Result<(), ExternalVideoTransportDriverError> {
                self.events.push(format!(
                    "start:{:?}:{}:{}",
                    route.direction, route.backend_id, route.endpoint_name
                ));
                Ok(())
            }

            fn stop_route(
                &mut self,
                route: &ExternalVideoTransportRoute,
            ) -> Result<(), ExternalVideoTransportDriverError> {
                self.events.push(format!(
                    "stop:{:?}:{}:{}",
                    route.direction, route.backend_id, route.endpoint_name
                ));
                Ok(())
            }
        }

        let mut snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 2,
                label: "NDI Camera".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::Ndi,
                    path: None,
                    name: Some("Camera A".to_string()),
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    enabled: true,
                    ..VideoLayerState::default()
                },
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: vec![VideoOutputSummary {
                id: 11,
                label: "Spout Program".to_string(),
                kind: VideoOutputKind::SpoutSender,
                enabled: true,
                composition_id: 1,
                fullscreen: false,
                monitor_id: None,
                monitor_identity: None,
                width: 1920,
                height: 1080,
                endpoint_name: Some("Syndocal Stage".to_string()),
                opacity: 1.0,
                blackout: false,
                mapping: VideoOutputMapping::default(),
            }],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let status = VideoRuntimeStatus {
            backends: vec![
                VideoBackendStatus {
                    id: "ndi".to_string(),
                    label: "NDI input/output".to_string(),
                    state: VideoBackendState::NotBuilt,
                    detail: "NDI SDK backend is not linked in this build".to_string(),
                },
                VideoBackendStatus {
                    id: "spout".to_string(),
                    label: "Spout input/output".to_string(),
                    state: VideoBackendState::Available,
                    detail: "Spout backend loaded".to_string(),
                },
            ],
        };
        let mut runtime = ExternalVideoTransportRuntime::new();
        let mut driver = RecordingTransportDriver::default();

        let plans = build_external_video_io_route_plans(&snapshot, &status);
        let first = runtime.sync_routes_with_driver(&plans, &mut driver);
        assert_eq!(first.started.len(), 1);
        assert_eq!(
            first.started[0].direction,
            ExternalVideoTransportDirection::Output
        );
        assert_eq!(first.blocked.len(), 1);
        assert_eq!(
            first.blocked[0].route.direction,
            ExternalVideoTransportDirection::Input
        );
        assert_eq!(first.active_count, 1);
        assert_eq!(
            driver.events,
            vec!["start:Output:spout:Syndocal Stage".to_string()]
        );
        let first_status = runtime.status();
        assert_eq!(first_status.active_count, 1);
        assert_eq!(first_status.active_routes, first.started);

        let second = runtime.sync_routes_with_driver(&plans, &mut driver);
        assert!(second.started.is_empty());
        assert_eq!(second.kept, first.started);
        assert!(second.stopped.is_empty());
        assert_eq!(second.active_count, 1);
        assert_eq!(driver.events.len(), 1);

        snapshot.outputs[0].blackout = true;
        let blackout_plans = build_external_video_io_route_plans(&snapshot, &status);
        let third = runtime.sync_routes_with_driver(&blackout_plans, &mut driver);
        assert!(third.started.is_empty());
        assert_eq!(third.stopped, first.started);
        assert_eq!(third.idle.len(), 1);
        assert_eq!(
            third.idle[0].direction,
            ExternalVideoTransportDirection::Output
        );
        assert_eq!(third.active_count, 0);
        assert!(runtime.active_routes().is_empty());
        assert_eq!(runtime.status(), ExternalVideoTransportStatus::default());
        assert_eq!(
            driver.events,
            vec![
                "start:Output:spout:Syndocal Stage".to_string(),
                "stop:Output:spout:Syndocal Stage".to_string()
            ]
        );

        snapshot.outputs[0].blackout = false;
        let rearmed_plans = build_external_video_io_route_plans(&snapshot, &status);
        let rearmed = runtime.sync_routes_with_driver_and_role_with_start_admission(
            &rearmed_plans,
            &mut driver,
            MachineOutputRole::Both,
            |driver| {
                driver.events.push("admit".to_string());
                Ok(())
            },
        );
        assert_eq!(rearmed.started.len(), 1);
        assert_eq!(
            driver.events,
            vec![
                "start:Output:spout:Syndocal Stage".to_string(),
                "stop:Output:spout:Syndocal Stage".to_string(),
                "admit".to_string(),
                "start:Output:spout:Syndocal Stage".to_string(),
            ]
        );

        snapshot.outputs[0].endpoint_name = Some("Syndocal Stage B".to_string());
        let replacement_plans = build_external_video_io_route_plans(&snapshot, &status);
        let replacement = runtime.sync_routes_with_driver_and_role_with_start_admission(
            &replacement_plans,
            &mut driver,
            MachineOutputRole::Both,
            |driver| {
                driver.events.push("ack-old-teardown".to_string());
                Ok(())
            },
        );
        assert_eq!(replacement.stopped.len(), 1);
        assert_eq!(replacement.started.len(), 1);
        assert_eq!(
            &driver.events[4..],
            &[
                "stop:Output:spout:Syndocal Stage".to_string(),
                "ack-old-teardown".to_string(),
                "start:Output:spout:Syndocal Stage B".to_string(),
            ]
        );
        let lighting_only = runtime.sync_routes_with_driver_and_role(
            &replacement_plans,
            &mut driver,
            MachineOutputRole::Lighting,
        );
        assert!(lighting_only.started.is_empty());
        assert_eq!(lighting_only.stopped, replacement.started);
        assert!(lighting_only
            .blocked
            .iter()
            .any(|route| route.route.direction == ExternalVideoTransportDirection::Output));
        assert!(runtime.active_routes().is_empty());
    }

    #[test]
    fn external_video_transport_runtime_reports_driver_failures() {
        struct FailableTransportDriver {
            fail_start: bool,
            fail_stop: bool,
        }

        impl ExternalVideoTransportDriver for FailableTransportDriver {
            fn start_route(
                &mut self,
                route: &ExternalVideoTransportRoute,
            ) -> Result<(), ExternalVideoTransportDriverError> {
                if self.fail_start {
                    Err(ExternalVideoTransportDriverError {
                        message: format!("cannot start {}", route.endpoint_name),
                    })
                } else {
                    Ok(())
                }
            }

            fn stop_route(
                &mut self,
                route: &ExternalVideoTransportRoute,
            ) -> Result<(), ExternalVideoTransportDriverError> {
                if self.fail_stop {
                    Err(ExternalVideoTransportDriverError {
                        message: format!("cannot stop {}", route.endpoint_name),
                    })
                } else {
                    Ok(())
                }
            }
        }

        let mut snapshot = VideoSnapshot {
            layers: Vec::new(),
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: vec![VideoOutputSummary {
                id: 11,
                label: "Spout Program".to_string(),
                kind: VideoOutputKind::SpoutSender,
                enabled: true,
                composition_id: 1,
                fullscreen: false,
                monitor_id: None,
                monitor_identity: None,
                width: 1920,
                height: 1080,
                endpoint_name: Some("Syndocal Stage".to_string()),
                opacity: 1.0,
                blackout: false,
                mapping: VideoOutputMapping::default(),
            }],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let status = VideoRuntimeStatus {
            backends: vec![VideoBackendStatus {
                id: "spout".to_string(),
                label: "Spout input/output".to_string(),
                state: VideoBackendState::Available,
                detail: "Spout backend loaded".to_string(),
            }],
        };
        let plans = build_external_video_io_route_plans(&snapshot, &status);
        let mut runtime = ExternalVideoTransportRuntime::new();

        let start_failed = runtime.sync_routes_with_driver(
            &plans,
            &mut FailableTransportDriver {
                fail_start: true,
                fail_stop: false,
            },
        );
        assert!(start_failed.started.is_empty());
        assert_eq!(start_failed.start_failed.len(), 1);
        assert!(start_failed.start_failed[0].issue.contains("cannot start"));
        assert_eq!(start_failed.active_count, 0);
        assert!(runtime.active_routes().is_empty());

        let started = runtime.sync_routes_with_driver(
            &plans,
            &mut FailableTransportDriver {
                fail_start: false,
                fail_stop: false,
            },
        );
        assert_eq!(started.started.len(), 1);
        assert_eq!(started.active_count, 1);

        snapshot.outputs[0].blackout = true;
        let blackout_plans = build_external_video_io_route_plans(&snapshot, &status);
        let stop_failed = runtime.sync_routes_with_driver(
            &blackout_plans,
            &mut FailableTransportDriver {
                fail_start: false,
                fail_stop: true,
            },
        );
        assert!(stop_failed.stopped.is_empty());
        assert_eq!(stop_failed.stop_failed.len(), 1);
        assert!(stop_failed.stop_failed[0].issue.contains("cannot stop"));
        assert_eq!(stop_failed.active_count, 1);
        assert_eq!(runtime.active_routes().len(), 1);

        snapshot.outputs[0].blackout = false;
        snapshot.outputs[0].endpoint_name = Some("Syndocal Stage B".to_string());
        let replacement_plans = build_external_video_io_route_plans(&snapshot, &status);
        let withheld_start = runtime.sync_routes_with_driver_and_role_with_start_admission(
            &replacement_plans,
            &mut FailableTransportDriver {
                fail_start: false,
                fail_stop: true,
            },
            MachineOutputRole::Both,
            |_| panic!("start admission must not run after a failed stop"),
        );
        assert!(withheld_start.started.is_empty());
        assert_eq!(withheld_start.stop_failed.len(), 1);
        assert_eq!(withheld_start.start_failed.len(), 1);
        assert!(withheld_start.start_failed[0]
            .issue
            .contains("withheld because route teardown failed"));
        assert_eq!(runtime.active_routes().len(), 1);
    }

    #[test]
    fn composition_plan_follows_composition_order_and_applies_master_opacity() {
        let snapshot = VideoSnapshot {
            layers: vec![
                VideoLayerSummary {
                    id: 1,
                    label: "Back".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::File,
                        path: Some("back.mp4".to_string()),
                        name: None,
                        codec: None,
                        metadata: None,
                    },
                    media_asset_id: None,
                    clip_slots: Vec::new(),
                    default_clip_slot_id: None,
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        opacity: 0.5,
                        position_ms: 100,
                        ..VideoLayerState::default()
                    },
                    isf_effect: None,
                },
                VideoLayerSummary {
                    id: 2,
                    label: "Front".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::StillImage,
                        path: Some("front.png".to_string()),
                        name: None,
                        codec: None,
                        metadata: None,
                    },
                    media_asset_id: None,
                    clip_slots: Vec::new(),
                    default_clip_slot_id: None,
                    blend_mode: VideoBlendMode::Screen,
                    state: VideoLayerState {
                        opacity: 1.0,
                        position_ms: 200,
                        ..VideoLayerState::default()
                    },
                    isf_effect: None,
                },
            ],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: vec![CompositionSummary {
                id: 5,
                label: "Output A".to_string(),
                layer_ids: vec![2, 1],
                output_ids: vec![7],
            }],
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 0.5,
            blackout: false,
        };

        let plans = build_composition_plans(&snapshot);

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].composition_id, 5);
        assert_eq!(plans[0].output_ids, vec![7]);
        assert_eq!(plans[0].layers[0].layer_id, 2);
        assert_eq!(plans[0].layers[0].blend_mode, VideoBlendMode::Screen);
        assert_eq!(plans[0].layers[0].opacity, 0.5);
        assert_eq!(plans[0].layers[1].layer_id, 1);
        assert_eq!(plans[0].layers[1].opacity, 0.25);
    }

    #[test]
    fn composition_plan_omits_layers_during_blackout() {
        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 1,
                label: "Layer".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("clip.mp4".to_string()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: true,
        };

        let plans = build_composition_plans(&snapshot);

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].label, "Main");
        assert!(plans[0].layers.is_empty());
    }

    #[test]
    fn composition_plan_omits_disabled_layers_without_changing_opacity() {
        let snapshot = VideoSnapshot {
            layers: vec![
                VideoLayerSummary {
                    id: 1,
                    label: "Muted".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::File,
                        path: Some("muted.mp4".to_string()),
                        name: None,
                        codec: None,
                        metadata: None,
                    },
                    media_asset_id: None,
                    clip_slots: Vec::new(),
                    default_clip_slot_id: None,
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        enabled: false,
                        opacity: 0.8,
                        ..VideoLayerState::default()
                    },
                    isf_effect: None,
                },
                VideoLayerSummary {
                    id: 2,
                    label: "Live".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::File,
                        path: Some("live.mp4".to_string()),
                        name: None,
                        codec: None,
                        metadata: None,
                    },
                    media_asset_id: None,
                    clip_slots: Vec::new(),
                    default_clip_slot_id: None,
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        enabled: true,
                        opacity: 0.6,
                        ..VideoLayerState::default()
                    },
                    isf_effect: None,
                },
            ],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };

        let plans = build_composition_plans(&snapshot);

        assert_eq!(snapshot.layers[0].state.opacity, 0.8);
        assert_eq!(plans[0].layers.len(), 1);
        assert_eq!(plans[0].layers[0].layer_id, 2);
        assert_eq!(plans[0].layers[0].opacity, 0.6);
    }

    #[test]
    fn composition_plan_limits_output_to_enabled_solo_layers() {
        let snapshot = VideoSnapshot {
            layers: vec![
                VideoLayerSummary {
                    id: 1,
                    label: "Back".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::File,
                        path: Some("back.mp4".to_string()),
                        name: None,
                        codec: None,
                        metadata: None,
                    },
                    media_asset_id: None,
                    clip_slots: Vec::new(),
                    default_clip_slot_id: None,
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        opacity: 1.0,
                        ..VideoLayerState::default()
                    },
                    isf_effect: None,
                },
                VideoLayerSummary {
                    id: 2,
                    label: "Solo".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::File,
                        path: Some("solo.mp4".to_string()),
                        name: None,
                        codec: None,
                        metadata: None,
                    },
                    media_asset_id: None,
                    clip_slots: Vec::new(),
                    default_clip_slot_id: None,
                    blend_mode: VideoBlendMode::Add,
                    state: VideoLayerState {
                        solo: true,
                        opacity: 0.75,
                        ..VideoLayerState::default()
                    },
                    isf_effect: None,
                },
                VideoLayerSummary {
                    id: 3,
                    label: "Muted Solo".to_string(),
                    source: VideoSourceSummary {
                        kind: VideoSourceKind::File,
                        path: Some("muted-solo.mp4".to_string()),
                        name: None,
                        codec: None,
                        metadata: None,
                    },
                    media_asset_id: None,
                    clip_slots: Vec::new(),
                    default_clip_slot_id: None,
                    blend_mode: VideoBlendMode::Screen,
                    state: VideoLayerState {
                        enabled: false,
                        solo: true,
                        opacity: 1.0,
                        ..VideoLayerState::default()
                    },
                    isf_effect: None,
                },
            ],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };

        let plans = build_composition_plans(&snapshot);

        assert_eq!(
            plans[0]
                .layers
                .iter()
                .map(|layer| layer.layer_id)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(plans[0].layers[0].blend_mode, VideoBlendMode::Add);
        assert_eq!(plans[0].layers[0].opacity, 0.75);
    }

    #[test]
    fn output_render_plan_applies_output_opacity_and_blackout() {
        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 1,
                label: "Layer".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("clip.mp4".to_string()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    opacity: 0.8,
                    ..VideoLayerState::default()
                },
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: vec![CompositionSummary {
                id: 1,
                label: "Main".to_string(),
                layer_ids: vec![1],
                output_ids: vec![7],
            }],
            outputs: vec![VideoOutputSummary {
                id: 7,
                label: "Projector".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 1,
                fullscreen: true,
                monitor_id: Some(1),
                monitor_identity: None,
                width: 1920,
                height: 1080,
                endpoint_name: None,
                opacity: 0.5,
                blackout: false,
                mapping: Default::default(),
            }],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };

        let plan = build_video_output_render_plan(&snapshot, 7).unwrap();

        assert_eq!(plan.output_id, 7);
        assert_eq!(plan.width, 1920);
        assert_eq!(plan.height, 1080);
        assert_eq!(plan.composition.layers.len(), 1);
        assert!((plan.composition.layers[0].opacity - 0.4).abs() < f32::EPSILON);

        let mut blacked_out = snapshot.clone();
        blacked_out.outputs[0].blackout = true;
        let plan = build_video_output_render_plan(&blacked_out, 7).unwrap();
        assert!(plan.output_blackout);
        assert!(plan.composition.layers.is_empty());
    }

    #[test]
    fn output_render_plan_uses_routed_composition_layer_order() {
        let layer = |id, label: &str| VideoLayerSummary {
            id,
            label: label.to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(format!("{label}.mp4")),
                name: None,
                codec: None,
                metadata: None,
            },
            media_asset_id: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
            isf_effect: None,
        };
        let snapshot = VideoSnapshot {
            layers: vec![layer(1, "Back"), layer(2, "Middle"), layer(3, "Front")],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: vec![
                CompositionSummary {
                    id: 1,
                    label: "Main".to_string(),
                    layer_ids: vec![1, 2, 3],
                    output_ids: Vec::new(),
                },
                CompositionSummary {
                    id: 9,
                    label: "Screen B".to_string(),
                    layer_ids: vec![3, 1],
                    output_ids: vec![12],
                },
            ],
            outputs: vec![VideoOutputSummary {
                id: 12,
                label: "Projector B".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 9,
                fullscreen: true,
                monitor_id: Some(1),
                monitor_identity: None,
                width: 1280,
                height: 720,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: VideoOutputMapping {
                    aspect_ratio: 1.2,
                    keystone_y: -0.1,
                    ..Default::default()
                },
            }],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };

        let plan = build_video_output_render_plan(&snapshot, 12).unwrap();
        let layer_ids = plan
            .composition
            .layers
            .iter()
            .map(|layer| layer.layer_id)
            .collect::<Vec<_>>();

        assert_eq!(plan.composition.composition_id, 9);
        assert_eq!(layer_ids, vec![3, 1]);
        assert!((plan.mapping.aspect_ratio - 1.2).abs() < f32::EPSILON);
        assert!((plan.mapping.keystone_y + 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn output_preview_renderer_uses_output_plan() {
        #[derive(Default)]
        struct TestFrameProvider;

        impl VideoFrameProvider for TestFrameProvider {
            fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

            fn frame_for_layer(
                &mut self,
                layer: &VideoLayerSummary,
                width: u32,
                height: u32,
            ) -> Result<VideoFrame, VideoFrameProviderError> {
                Ok(VideoFrame {
                    layer_id: layer.id,
                    width,
                    height,
                    pts_ms: layer.state.position_ms,
                    duration_ms: 16,
                    format: VideoPixelFormat::Rgba8,
                    data: [200, 0, 0, 255].repeat(width as usize * height as usize),
                })
            }
        }

        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 1,
                label: "Layer".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("clip.mp4".to_string()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: vec![CompositionSummary {
                id: 1,
                label: "Main".to_string(),
                layer_ids: vec![1],
                output_ids: vec![9],
            }],
            outputs: vec![VideoOutputSummary {
                id: 9,
                label: "Projector".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 1,
                fullscreen: true,
                monitor_id: None,
                monitor_identity: None,
                width: 2,
                height: 1,
                endpoint_name: None,
                opacity: 0.5,
                blackout: false,
                mapping: Default::default(),
            }],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 2,
                preview_width: 2,
                preview_height: 1,
            },
            TestFrameProvider,
        );

        let frame = renderer.render_output(&snapshot, 9).unwrap();

        assert_eq!(frame.width, 2);
        assert_eq!(frame.height, 1);
        assert_eq!(frame.data, vec![100, 0, 0, 128, 100, 0, 0, 128]);
    }

    #[test]
    fn layer_preview_renderer_ignores_live_visibility_and_blackout() {
        #[derive(Default)]
        struct TestFrameProvider;

        impl VideoFrameProvider for TestFrameProvider {
            fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

            fn frame_for_layer(
                &mut self,
                layer: &VideoLayerSummary,
                width: u32,
                height: u32,
            ) -> Result<VideoFrame, VideoFrameProviderError> {
                Ok(VideoFrame {
                    layer_id: layer.id,
                    width,
                    height,
                    pts_ms: layer.state.position_ms,
                    duration_ms: 16,
                    format: VideoPixelFormat::Rgba8,
                    data: [24, 96, 180, 255].repeat(width as usize * height as usize),
                })
            }
        }

        let mut state = VideoLayerState::default();
        state.enabled = false;
        state.opacity = 0.0;
        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 7,
                label: "Hidden Clip".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("hidden.mp4".to_string()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Add,
                state,
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 0.0,
            blackout: true,
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 2,
                preview_width: 2,
                preview_height: 1,
            },
            TestFrameProvider,
        );

        let frame = renderer.render_layer_preview(&snapshot, 7, 2, 1).unwrap();

        assert_eq!(frame.data, vec![24, 96, 180, 255, 24, 96, 180, 255]);
        assert_eq!(
            renderer.render_layer_preview(&snapshot, 99, 2, 1),
            Err(VideoPreviewError::MissingLayer { layer_id: 99 })
        );
    }

    #[test]
    fn output_preview_renderer_requests_only_routed_layers_and_skips_blackout_decode() {
        #[derive(Default)]
        struct TestFrameProvider {
            retained: Vec<Vec<VideoLayerId>>,
            requested: Vec<VideoLayerId>,
        }

        impl VideoFrameProvider for TestFrameProvider {
            fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
                self.retained.push(layer_ids.to_vec());
            }

            fn frame_for_layer(
                &mut self,
                layer: &VideoLayerSummary,
                width: u32,
                height: u32,
            ) -> Result<VideoFrame, VideoFrameProviderError> {
                self.requested.push(layer.id);
                Ok(VideoFrame {
                    layer_id: layer.id,
                    width,
                    height,
                    pts_ms: layer.state.position_ms,
                    duration_ms: 16,
                    format: VideoPixelFormat::Rgba8,
                    data: [layer.id as u8, 0, 0, 255].repeat(width as usize * height as usize),
                })
            }
        }

        let layer = |id| VideoLayerSummary {
            id,
            label: format!("Layer {id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(format!("layer-{id}.mp4")),
                name: None,
                codec: None,
                metadata: None,
            },
            media_asset_id: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
            isf_effect: None,
        };
        let mut snapshot = VideoSnapshot {
            layers: vec![layer(1), layer(2)],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: vec![CompositionSummary {
                id: 8,
                label: "Screen".to_string(),
                layer_ids: vec![2],
                output_ids: vec![9],
            }],
            outputs: vec![VideoOutputSummary {
                id: 9,
                label: "Projector".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 8,
                fullscreen: false,
                monitor_id: None,
                monitor_identity: None,
                width: 2,
                height: 1,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: Default::default(),
            }],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 2,
                preview_width: 2,
                preview_height: 1,
            },
            TestFrameProvider::default(),
        );

        let prepared = renderer.prepare_output_frames(&snapshot, 9, 2, 1).unwrap();
        assert_eq!(prepared.plan.output_id, 9);
        assert_eq!(prepared.plan.composition.layers[0].layer_id, 2);
        assert_eq!(prepared.frames.len(), 1);
        assert_eq!(prepared.frames[0].layer_id, 2);

        let frame = renderer.render_output(&snapshot, 9).unwrap();
        assert_eq!(frame.data, vec![2, 0, 0, 255, 2, 0, 0, 255]);
        assert_eq!(renderer.frame_provider().retained, vec![vec![2], vec![2]]);
        assert_eq!(renderer.frame_provider().requested, vec![2, 2]);

        snapshot.outputs[0].blackout = true;
        let prepared = renderer.prepare_output_frames(&snapshot, 9, 2, 1).unwrap();
        assert!(prepared.plan.output_blackout);
        assert!(prepared.frames.is_empty());
        let frame = renderer.render_output(&snapshot, 9).unwrap();
        assert_eq!(frame.data, vec![0, 0, 0, 255, 0, 0, 0, 255]);
        assert_eq!(renderer.frame_provider().retained, vec![vec![2], vec![2]]);
        assert_eq!(renderer.frame_provider().requested, vec![2, 2]);

        snapshot.outputs[0].blackout = false;
        snapshot.outputs[0].enabled = false;
        let frame = renderer.render_output(&snapshot, 9).unwrap();
        assert_eq!(frame.data, vec![0, 0, 0, 255, 0, 0, 0, 255]);
        assert_eq!(renderer.frame_provider().retained, vec![vec![2], vec![2]]);
        assert_eq!(renderer.frame_provider().requested, vec![2, 2]);
    }

    #[test]
    fn output_preview_renderer_applies_projector_scale_mapping() {
        #[derive(Default)]
        struct TestFrameProvider;

        impl VideoFrameProvider for TestFrameProvider {
            fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

            fn frame_for_layer(
                &mut self,
                layer: &VideoLayerSummary,
                width: u32,
                height: u32,
            ) -> Result<VideoFrame, VideoFrameProviderError> {
                assert_eq!(width, 4);
                assert_eq!(height, 1);
                Ok(VideoFrame {
                    layer_id: layer.id,
                    width,
                    height,
                    pts_ms: 0,
                    duration_ms: 16,
                    format: VideoPixelFormat::Rgba8,
                    data: vec![
                        255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255,
                    ],
                })
            }
        }

        let snapshot = VideoSnapshot {
            layers: vec![VideoLayerSummary {
                id: 1,
                label: "Layer".to_string(),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("clip.mp4".to_string()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
                isf_effect: None,
            }],
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: vec![CompositionSummary {
                id: 1,
                label: "Main".to_string(),
                layer_ids: vec![1],
                output_ids: vec![9],
            }],
            outputs: vec![VideoOutputSummary {
                id: 9,
                label: "Projector".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 1,
                fullscreen: true,
                monitor_id: None,
                monitor_identity: None,
                width: 4,
                height: 1,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: VideoOutputMapping {
                    scale_x: 0.5,
                    ..Default::default()
                },
            }],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 2,
                preview_width: 4,
                preview_height: 1,
            },
            TestFrameProvider,
        );

        let frame = renderer.render_output(&snapshot, 9).unwrap();

        assert_eq!(
            frame.data,
            vec![0, 0, 0, 0, 255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0,]
        );
    }

    #[test]
    fn inverse_output_mapping_applies_keystone_correction() {
        let mapping = VideoOutputMapping {
            keystone_x: 0.4,
            keystone_y: -0.2,
            ..Default::default()
        };

        let (x, y) = inverse_video_output_mapping(0.25, 0.25, &mapping, 1.0);

        assert!((x - 0.15).abs() < f32::EPSILON);
        assert!((y - 0.30).abs() < f32::EPSILON);
    }

    #[test]
    fn inverse_output_mapping_fit_mode_preserves_requested_aspect() {
        let mapping = VideoOutputMapping {
            aspect_ratio: 4.0 / 3.0,
            aspect_mode: VideoOutputAspectMode::Fit,
            ..Default::default()
        };

        let (x, y) = inverse_video_output_mapping(0.45, 0.0, &mapping, 16.0 / 9.0);

        assert!(x > 0.5);
        assert!(y.abs() < f32::EPSILON);
    }

    #[test]
    fn inverse_output_mapping_applies_lens_distortion() {
        let mapping = VideoOutputMapping {
            lens_distortion: 0.5,
            ..Default::default()
        };

        let (x, y) = inverse_video_output_mapping(0.4, 0.0, &mapping, 1.0);

        assert!(x > 0.4);
        assert!(y.abs() < f32::EPSILON);
    }

    #[test]
    fn output_mapping_applies_edge_blend_gamma_and_black_level() {
        let mut edge_pixel = [255_u8, 255, 255, 255];
        apply_output_blend_correction(
            &mut edge_pixel,
            0.05,
            0.5,
            &VideoOutputMapping {
                edge_blend_left: 0.5,
                edge_blend_gamma: 1.0,
                ..Default::default()
            },
        );
        assert!(edge_pixel[0] < 16);
        assert_eq!(edge_pixel[0], edge_pixel[1]);
        assert_eq!(edge_pixel[1], edge_pixel[2]);
        assert!(edge_pixel[3] < 16);

        let mut black_pixel = [0_u8, 0, 0, 255];
        apply_output_blend_correction(
            &mut black_pixel,
            0.5,
            0.5,
            &VideoOutputMapping {
                black_level: 0.2,
                ..Default::default()
            },
        );
        assert_eq!(black_pixel, [51, 51, 51, 255]);
    }

    #[test]
    fn output_polygon_mask_supports_feather_and_inversion() {
        let mut points = [VideoMaskPoint::default(); 8];
        points[..4].copy_from_slice(&[
            VideoMaskPoint { x: 0.25, y: 0.25 },
            VideoMaskPoint { x: 0.75, y: 0.25 },
            VideoMaskPoint { x: 0.75, y: 0.75 },
            VideoMaskPoint { x: 0.25, y: 0.75 },
        ]);
        let mapping = VideoOutputMapping {
            mask_point_count: 4,
            mask_softness: 0.1,
            mask_points: points,
            ..VideoOutputMapping::default()
        };

        assert!((polygon_mask_factor(0.5, 0.5, &mapping) - 1.0).abs() < f32::EPSILON);
        assert_eq!(polygon_mask_factor(0.1, 0.5, &mapping), 0.0);
        let feathered = polygon_mask_factor(0.27, 0.5, &mapping);
        assert!(feathered > 0.0 && feathered < 1.0);

        let inverted = VideoOutputMapping {
            mask_invert: true,
            ..mapping
        };
        assert_eq!(combined_output_mask_factor(0.5, 0.5, &inverted), 0.0);
        assert_eq!(combined_output_mask_factor(0.1, 0.5, &inverted), 1.0);
    }

    #[test]
    fn output_bitmap_mask_is_embedded_bilinear_and_invertible() {
        let mut words = [0_u32; VIDEO_OUTPUT_BITMAP_MASK_WORD_CAPACITY];
        words[0] = 0x0000_00f0;
        let mapping = VideoOutputMapping {
            bitmap_mask_width: 2,
            bitmap_mask_height: 1,
            bitmap_mask_luma_words: words,
            ..VideoOutputMapping::default()
        };

        assert_eq!(video_bitmap_mask_luma(&mapping, 0, 0), 0);
        assert_eq!(video_bitmap_mask_luma(&mapping, 1, 0), 255);
        assert_eq!(combined_output_mask_factor(0.0, 0.5, &mapping), 0.0);
        assert_eq!(combined_output_mask_factor(1.0, 0.5, &mapping), 1.0);
        assert!((combined_output_mask_factor(0.5, 0.5, &mapping) - 0.5).abs() < 0.001);

        let inverted = VideoOutputMapping {
            mask_invert: true,
            ..mapping
        };
        assert_eq!(combined_output_mask_factor(0.0, 0.5, &inverted), 1.0);
        assert_eq!(combined_output_mask_factor(1.0, 0.5, &inverted), 0.0);
    }

    #[test]
    fn bitmap_mask_import_resizes_and_quantizes_luma() {
        let path =
            std::env::temp_dir().join(format!("syndocal-bitmap-mask-{}.png", std::process::id()));
        let source =
            image::GrayImage::from_fn(2, 1, |x, _| image::Luma([if x == 0 { 0 } else { 255 }]));
        source.save(&path).unwrap();

        let (width, height, words) = load_video_bitmap_mask(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let mapping = VideoOutputMapping {
            bitmap_mask_width: width,
            bitmap_mask_height: height,
            bitmap_mask_luma_words: words,
            ..VideoOutputMapping::default()
        };

        assert_eq!(width, VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION);
        assert_eq!(height, VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION);
        assert_eq!(video_bitmap_mask_luma(&mapping, 0, 8), 0);
        assert_eq!(video_bitmap_mask_luma(&mapping, 15, 8), 255);
    }

    #[test]
    fn output_test_pattern_is_routed_through_projector_mapping() {
        let mut snapshot = VideoSnapshot {
            layers: Vec::new(),
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: Vec::new(),
            outputs: vec![VideoOutputSummary {
                id: 9,
                label: "Projector".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 1,
                fullscreen: true,
                monitor_id: None,
                monitor_identity: None,
                width: 8,
                height: 8,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: Default::default(),
            }],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        };

        let identity = render_video_output_test_pattern(&snapshot, 9, 8, 8).unwrap();
        snapshot.outputs[0].mapping = VideoOutputMapping {
            scale_x: 0.5,
            ..Default::default()
        };
        let scaled = render_video_output_test_pattern(&snapshot, 9, 8, 8).unwrap();

        assert_eq!(scaled.width, 8);
        assert_eq!(scaled.height, 8);
        assert_eq!(identity.data[4 * 8 * 4 + 3], 255);
        assert_eq!(scaled.data[4 * 8 * 4 + 3], 0);
        assert_ne!(identity.data, scaled.data);
    }

    #[test]
    fn output_test_pattern_contains_projector_calibration_guides() {
        let frame = video_output_test_pattern(160, 90, 3);
        let pixel = |x: u32, y: u32| {
            let index = ((y * frame.width + x) * 4) as usize;
            [
                frame.data[index],
                frame.data[index + 1],
                frame.data[index + 2],
                frame.data[index + 3],
            ]
        };

        assert_eq!(pixel(0, 0), [255, 64, 64, 255]);
        assert_eq!(pixel(159, 0), [64, 255, 96, 255]);
        assert_eq!(pixel(159, 89), [64, 144, 255, 255]);
        assert_eq!(pixel(0, 89), [255, 220, 64, 255]);
        assert_eq!(pixel(80, 45), [0, 210, 255, 255]);
        assert_eq!(pixel(20, 45), [255, 80, 180, 255]);
        assert_eq!(pixel(16, 22), [226, 174, 64, 255]);
        assert_eq!(pixel(53, 20), [92, 120, 78, 255]);
        assert_eq!(pixel(90, 55), [242, 206, 88, 255]);
    }

    #[test]
    fn cpu_compositor_blends_normal_with_layer_opacity() {
        let output = composite_rgba8(
            &plan(vec![
                layer_plan(1, VideoBlendMode::Normal, 1.0),
                layer_plan(2, VideoBlendMode::Normal, 0.5),
            ]),
            &[
                rgba_frame(1, [10, 20, 30, 255]),
                rgba_frame(2, [110, 220, 30, 255]),
            ],
            1,
            1,
        )
        .unwrap();

        assert_eq!(output.data, vec![60, 120, 30, 255]);
    }

    #[test]
    fn cpu_compositor_matches_add_multiply_and_screen_reference_modes() {
        let add = composite_rgba8(
            &plan(vec![
                layer_plan(1, VideoBlendMode::Normal, 1.0),
                layer_plan(2, VideoBlendMode::Add, 1.0),
            ]),
            &[
                rgba_frame(1, [100, 100, 100, 255]),
                rgba_frame(2, [50, 200, 255, 255]),
            ],
            1,
            1,
        )
        .unwrap();
        let multiply = composite_rgba8(
            &plan(vec![
                layer_plan(1, VideoBlendMode::Normal, 1.0),
                layer_plan(2, VideoBlendMode::Multiply, 1.0),
            ]),
            &[
                rgba_frame(1, [100, 100, 100, 255]),
                rgba_frame(2, [128, 255, 0, 255]),
            ],
            1,
            1,
        )
        .unwrap();
        let screen = composite_rgba8(
            &plan(vec![
                layer_plan(1, VideoBlendMode::Normal, 1.0),
                layer_plan(2, VideoBlendMode::Screen, 1.0),
            ]),
            &[
                rgba_frame(1, [100, 100, 100, 255]),
                rgba_frame(2, [128, 0, 255, 255]),
            ],
            1,
            1,
        )
        .unwrap();

        assert_eq!(add.data, vec![150, 255, 255, 255]);
        assert_eq!(multiply.data, vec![50, 100, 0, 255]);
        assert_eq!(screen.data, vec![178, 100, 255, 255]);
    }

    #[test]
    fn cpu_compositor_applies_layer_color_adjustments() {
        let brighter = composite_rgba8(
            &plan(vec![CompositionLayerPlan {
                color: VideoColorAdjust {
                    brightness: 0.2,
                    ..VideoColorAdjust::default()
                },
                ..layer_plan(1, VideoBlendMode::Normal, 1.0)
            }]),
            &[rgba_frame(1, [100, 100, 100, 255])],
            1,
            1,
        )
        .unwrap();
        let desaturated = composite_rgba8(
            &plan(vec![CompositionLayerPlan {
                color: VideoColorAdjust {
                    saturation: 0.0,
                    ..VideoColorAdjust::default()
                },
                ..layer_plan(1, VideoBlendMode::Normal, 1.0)
            }]),
            &[rgba_frame(1, [200, 0, 0, 255])],
            1,
            1,
        )
        .unwrap();

        assert_eq!(brighter.data, vec![151, 151, 151, 255]);
        assert_eq!(desaturated.data, vec![60, 60, 60, 255]);
    }

    #[test]
    fn cpu_compositor_applies_layer_pixelate_and_blur() {
        let source = rgba_frame_with_size(
            1,
            4,
            1,
            vec![10, 0, 0, 255, 20, 0, 0, 255, 100, 0, 0, 255, 200, 0, 0, 255],
        );
        let pixelated = composite_rgba8(
            &plan(vec![CompositionLayerPlan {
                fx: VideoFxAdjust {
                    pixelate: 2.0,
                    ..VideoFxAdjust::default()
                },
                ..layer_plan(1, VideoBlendMode::Normal, 1.0)
            }]),
            &[source.clone()],
            4,
            1,
        )
        .unwrap();
        let blurred = composite_rgba8(
            &plan(vec![CompositionLayerPlan {
                fx: VideoFxAdjust {
                    blur: 1.0,
                    ..VideoFxAdjust::default()
                },
                ..layer_plan(1, VideoBlendMode::Normal, 1.0)
            }]),
            &[source],
            4,
            1,
        )
        .unwrap();

        assert_eq!(
            pixelated.data,
            vec![20, 0, 0, 255, 20, 0, 0, 255, 200, 0, 0, 255, 200, 0, 0, 255]
        );
        assert_eq!(
            blurred.data,
            vec![15, 0, 0, 255, 43, 0, 0, 255, 106, 0, 0, 255, 150, 0, 0, 255]
        );
    }

    #[test]
    fn cpu_compositor_applies_layer_glow_edge_and_color_key() {
        let glow = composite_rgba8(
            &plan(vec![CompositionLayerPlan {
                fx: VideoFxAdjust {
                    glow: 0.5,
                    ..VideoFxAdjust::default()
                },
                ..layer_plan(1, VideoBlendMode::Normal, 1.0)
            }]),
            &[rgba_frame_with_size(
                1,
                2,
                1,
                vec![255, 0, 0, 255, 0, 0, 0, 255],
            )],
            2,
            1,
        )
        .unwrap();
        let edge = composite_rgba8(
            &plan(vec![CompositionLayerPlan {
                fx: VideoFxAdjust {
                    edge: 1.0,
                    ..VideoFxAdjust::default()
                },
                ..layer_plan(1, VideoBlendMode::Normal, 1.0)
            }]),
            &[rgba_frame_with_size(
                1,
                2,
                1,
                vec![0, 0, 0, 255, 255, 255, 255, 255],
            )],
            2,
            1,
        )
        .unwrap();
        let keyed = composite_rgba8(
            &plan(vec![CompositionLayerPlan {
                fx: VideoFxAdjust {
                    key_threshold: 0.1,
                    ..VideoFxAdjust::default()
                },
                ..layer_plan(1, VideoBlendMode::Normal, 1.0)
            }]),
            &[rgba_frame_with_size(
                1,
                2,
                1,
                vec![0, 255, 0, 255, 200, 0, 0, 255],
            )],
            2,
            1,
        )
        .unwrap();

        assert_eq!(glow.data, vec![255, 0, 0, 255, 128, 0, 0, 255]);
        assert_eq!(edge.data, vec![255, 255, 255, 255, 255, 255, 255, 255]);
        assert_eq!(keyed.data, vec![0, 0, 0, 0, 200, 0, 0, 255]);
    }

    #[test]
    fn cpu_compositor_applies_layer_scale_position_and_crop() {
        let centered = composite_rgba8(
            &plan(vec![CompositionLayerPlan {
                transform: Transform2D {
                    scale_x: 1.0 / 3.0,
                    ..Transform2D::default()
                },
                ..layer_plan(1, VideoBlendMode::Normal, 1.0)
            }]),
            &[rgba_frame(1, [200, 0, 0, 255])],
            3,
            1,
        )
        .unwrap();
        let shifted = composite_rgba8(
            &plan(vec![CompositionLayerPlan {
                transform: Transform2D {
                    x: 1.0 / 3.0,
                    scale_x: 1.0 / 3.0,
                    ..Transform2D::default()
                },
                ..layer_plan(1, VideoBlendMode::Normal, 1.0)
            }]),
            &[rgba_frame(1, [200, 0, 0, 255])],
            3,
            1,
        )
        .unwrap();
        let cropped = composite_rgba8(
            &plan(vec![CompositionLayerPlan {
                transform: Transform2D {
                    crop_left: 0.5,
                    ..Transform2D::default()
                },
                ..layer_plan(1, VideoBlendMode::Normal, 1.0)
            }]),
            &[rgba_frame_with_size(
                1,
                2,
                1,
                vec![200, 0, 0, 255, 0, 200, 0, 255],
            )],
            1,
            1,
        )
        .unwrap();

        assert_eq!(centered.data, vec![0, 0, 0, 0, 200, 0, 0, 255, 0, 0, 0, 0]);
        assert_eq!(shifted.data, vec![0, 0, 0, 0, 0, 0, 0, 0, 200, 0, 0, 255]);
        assert_eq!(cropped.data, vec![0, 200, 0, 255]);
    }

    #[test]
    fn cpu_compositor_applies_layer_rotation() {
        let output = composite_rgba8(
            &plan(vec![CompositionLayerPlan {
                transform: Transform2D {
                    rotation_deg: 180.0,
                    ..Transform2D::default()
                },
                ..layer_plan(1, VideoBlendMode::Normal, 1.0)
            }]),
            &[rgba_frame_with_size(
                1,
                2,
                2,
                vec![
                    200, 0, 0, 255, 0, 200, 0, 255, 0, 0, 200, 255, 240, 240, 240, 255,
                ],
            )],
            2,
            2,
        )
        .unwrap();

        assert_eq!(
            output.data,
            vec![240, 240, 240, 255, 0, 0, 200, 255, 0, 200, 0, 255, 200, 0, 0, 255,]
        );
    }

    #[test]
    fn cpu_compositor_accepts_bgra8_frames() {
        let output = composite_rgba8(
            &plan(vec![layer_plan(1, VideoBlendMode::Normal, 1.0)]),
            &[bgra_frame(1, [30, 20, 10, 255])],
            1,
            1,
        )
        .unwrap();

        assert_eq!(output.data, vec![10, 20, 30, 255]);
    }

    #[test]
    fn cpu_compositor_decodes_dxt1_blocks() {
        let mut block = Vec::new();
        block.extend_from_slice(&0xf800u16.to_le_bytes());
        block.extend_from_slice(&0x001fu16.to_le_bytes());
        block.extend_from_slice(&0u32.to_le_bytes());
        let output = composite_rgba8(
            &plan(vec![layer_plan(1, VideoBlendMode::Normal, 1.0)]),
            &[dxt1_frame(1, 2, 2, block)],
            2,
            2,
        )
        .unwrap();

        assert_eq!(
            output.data,
            vec![255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255]
        );
    }

    #[test]
    fn cpu_compositor_decodes_dxt5_alpha_blocks() {
        let mut block = Vec::new();
        block.push(128);
        block.push(0);
        block.extend_from_slice(&[0; 6]);
        block.extend_from_slice(&0xf800u16.to_le_bytes());
        block.extend_from_slice(&0x001fu16.to_le_bytes());
        block.extend_from_slice(&0u32.to_le_bytes());
        let output = composite_rgba8(
            &plan(vec![layer_plan(1, VideoBlendMode::Normal, 1.0)]),
            &[dxt5_frame(1, 1, 1, block)],
            1,
            1,
        )
        .unwrap();

        assert_eq!(output.data, vec![128, 0, 0, 128]);
    }

    #[test]
    fn cpu_compositor_converts_hap_q_ycocg_blocks() {
        let mut block = vec![100, 100, 0, 0, 0, 0, 0, 0];
        block.extend_from_slice(&0x8400u16.to_le_bytes());
        block.extend_from_slice(&0x8400u16.to_le_bytes());
        block.extend_from_slice(&0u32.to_le_bytes());
        let output = composite_rgba8(
            &plan(vec![layer_plan(1, VideoBlendMode::Normal, 1.0)]),
            &[VideoFrame {
                format: VideoPixelFormat::YcoCgDxt5,
                data: block,
                ..dxt5_frame(1, 1, 1, Vec::new())
            }],
            1,
            1,
        )
        .unwrap();

        assert_eq!(output.data, vec![102, 102, 94, 255]);
    }

    #[test]
    fn cpu_compositor_rejects_missing_or_mismatched_frames() {
        let missing = composite_rgba8(
            &plan(vec![layer_plan(1, VideoBlendMode::Normal, 1.0)]),
            &[],
            1,
            1,
        );
        let mismatch = composite_rgba8(
            &plan(vec![layer_plan(1, VideoBlendMode::Normal, 1.0)]),
            &[VideoFrame {
                data: vec![0, 0, 0],
                ..rgba_frame(1, [0, 0, 0, 255])
            }],
            1,
            1,
        );

        assert_eq!(
            missing,
            Err(CpuCompositeError::MissingFrame { layer_id: 1 })
        );
        assert_eq!(
            mismatch,
            Err(CpuCompositeError::FrameSizeMismatch { layer_id: 1 })
        );
        let bad_dxt = composite_rgba8(
            &plan(vec![layer_plan(1, VideoBlendMode::Normal, 1.0)]),
            &[dxt1_frame(1, 4, 4, vec![0; 7])],
            1,
            1,
        );
        assert_eq!(
            bad_dxt,
            Err(CpuCompositeError::FrameSizeMismatch { layer_id: 1 })
        );
    }

    #[test]
    fn render_input_key_keeps_same_layer_same_pts_sources_distinct_through_queue_and_mix() {
        let first = VideoRenderInput::new(
            protocol::VideoRenderInputKey {
                project_render_epoch: 42,
                input_id: protocol::VideoRenderInputId(1),
            },
            decode_request(7, 100),
        );
        let second = VideoRenderInput::new(
            protocol::VideoRenderInputKey {
                project_render_epoch: 42,
                input_id: protocol::VideoRenderInputId(2),
            },
            decode_request(7, 100),
        );
        let mut runtime = VideoRuntime::new(VideoRuntimeConfig {
            frame_queue_capacity: 2,
            preview_width: 1,
            preview_height: 1,
        });
        runtime.sync_inputs(&[first.clone(), second.clone()]);

        let mut red = rgba_frame(7, [100, 0, 0, 255]);
        red.pts_ms = 100;
        let mut green = rgba_frame(7, [0, 100, 0, 255]);
        green.pts_ms = 100;
        assert_eq!(
            runtime.push_input_frame(&first, red),
            FrameQueuePush::Inserted
        );
        assert_eq!(
            runtime.push_input_frame(&second, green),
            FrameQueuePush::Inserted
        );
        assert_eq!(runtime.queue_count(), 2);
        assert_eq!(runtime.input_queue_len(first.key), 1);
        assert_eq!(runtime.input_queue_len(second.key), 1);

        let mut first_layer = layer_plan(7, VideoBlendMode::Normal, 1.0);
        first_layer.position_ms = 100;
        let mut second_layer = layer_plan(7, VideoBlendMode::Add, 1.0);
        second_layer.position_ms = 100;
        let mix = CompositionInputMix {
            inputs: vec![
                CompositionRenderInput {
                    key: first.key,
                    layer: first_layer,
                },
                CompositionRenderInput {
                    key: second.key,
                    layer: second_layer,
                },
            ],
        };
        let output = runtime.compose_input_mix(&mix, 1, 1).unwrap();

        assert_eq!(output.data, vec![100, 100, 0, 255]);
        assert_eq!(output.pts_ms, 100);
        assert_eq!(first.layer_id(), second.layer_id());
    }

    #[test]
    fn render_input_mix_rejects_duplicate_runtime_owners() {
        let key = protocol::VideoRenderInputKey {
            project_render_epoch: 42,
            input_id: protocol::VideoRenderInputId(9),
        };
        let mix = CompositionInputMix {
            inputs: vec![
                CompositionRenderInput {
                    key,
                    layer: layer_plan(7, VideoBlendMode::Normal, 1.0),
                },
                CompositionRenderInput {
                    key,
                    layer: layer_plan(7, VideoBlendMode::Add, 1.0),
                },
            ],
        };
        let frames = vec![VideoRenderInputFrame {
            key,
            frame: rgba_frame(7, [100, 0, 0, 255]),
        }];

        assert_eq!(
            composite_input_mix_rgba8(&mix, &frames, 1, 1),
            Err(CpuCompositeError::DuplicateRenderInputKey { key })
        );
        assert_eq!(
            validate_unique_render_input_keys([&key, &key]),
            Err(CpuCompositeError::DuplicateRenderInputKey { key })
        );
    }

    #[test]
    fn render_input_mix_errors_preserve_exact_runtime_owner() {
        let key = protocol::VideoRenderInputKey {
            project_render_epoch: 42,
            input_id: protocol::VideoRenderInputId(17),
        };
        let mix = CompositionInputMix {
            inputs: vec![CompositionRenderInput {
                key,
                layer: layer_plan(7, VideoBlendMode::Normal, 1.0),
            }],
        };

        assert_eq!(
            composite_input_mix_rgba8(&mix, &[], 1, 1),
            Err(CpuCompositeError::MissingRenderInputFrame { key, layer_id: 7 })
        );
        assert_eq!(
            composite_input_mix_rgba8(
                &mix,
                &[VideoRenderInputFrame {
                    key,
                    frame: VideoFrame {
                        data: vec![0, 0, 0],
                        ..rgba_frame(7, [0, 0, 0, 255])
                    },
                }],
                1,
                1,
            ),
            Err(CpuCompositeError::RenderInputFrameSizeMismatch { key, layer_id: 7 })
        );

        let hap_frame = dxt1_frame(7, 4, 4, vec![0; 8]);
        let converted = convert_render_input_frame_to_rgba8(key, &hap_frame).unwrap();
        assert_eq!(converted.format, VideoPixelFormat::Rgba8);
        assert_eq!(converted.data.len(), 4 * 4 * 4);
    }

    #[test]
    fn preview_render_input_failure_and_isf_diagnostics_preserve_owner() {
        struct InputFrameProvider;
        impl VideoFrameProvider for InputFrameProvider {
            fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

            fn frame_for_layer(
                &mut self,
                layer: &VideoLayerSummary,
                _width: u32,
                _height: u32,
            ) -> Result<VideoFrame, VideoFrameProviderError> {
                Ok(rgba_frame(layer.id, [24, 96, 180, 255]))
            }

            fn frame_for_input(
                &mut self,
                input: &VideoRenderInput,
            ) -> Result<VideoFrame, VideoFrameProviderError> {
                if input.request.position_ms == 999 {
                    return Err(VideoFrameProviderError::Decode {
                        layer_id: input.layer_id(),
                        label: input.request.label.clone(),
                        error: VideoDecodeError::Decode {
                            layer_id: input.layer_id(),
                            label: input.request.label.clone(),
                            message: "decode failed".to_string(),
                        },
                    });
                }
                Ok(rgba_frame(input.layer_id(), [24, 96, 180, 255]))
            }
        }

        let key = protocol::VideoRenderInputKey {
            project_render_epoch: 8,
            input_id: protocol::VideoRenderInputId(3),
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig {
                frame_queue_capacity: 2,
                preview_width: 1,
                preview_height: 1,
            },
            InputFrameProvider,
        );
        let bad_decode = VideoRenderInput::new(key, decode_request(7, 999));
        let mix = CompositionInputMix {
            inputs: vec![CompositionRenderInput {
                key,
                layer: layer_plan(7, VideoBlendMode::Normal, 1.0),
            }],
        };
        assert!(matches!(
            renderer.render_input_mix(&[bad_decode], &mix, 1, 1),
            Err(VideoPreviewError::RenderInput {
                key: error_key,
                ..
            }) if error_key == key
        ));

        let bad_isf = VideoRenderInput::new(key, decode_request(7, 10)).with_isf_effect(Some(
            protocol::VideoIsfEffectSummary {
                enabled: true,
                label: "Invalid".to_string(),
                source: "not an ISF shader".into(),
                source_path: None,
                description: None,
                categories: Vec::new(),
                controls: Vec::new(),
                stack: Vec::new(),
            },
        ));
        let output = renderer.render_input_mix(&[bad_isf], &mix, 1, 1).unwrap();
        assert_eq!(output.data, vec![24, 96, 180, 255]);
        assert_eq!(renderer.last_render_input_isf_stage_errors().len(), 1);
        assert_eq!(renderer.last_render_input_isf_stage_errors()[0].key, key);
    }

    #[test]
    fn decode_scheduler_deduplicates_only_within_one_render_input_key() {
        let request = decode_request(7, 100);
        let first = VideoRenderInput::new(
            protocol::VideoRenderInputKey {
                project_render_epoch: 9,
                input_id: protocol::VideoRenderInputId(3),
            },
            request.clone(),
        );
        let second = VideoRenderInput::new(
            protocol::VideoRenderInputKey {
                project_render_epoch: 9,
                input_id: protocol::VideoRenderInputId(4),
            },
            request,
        );
        let mut scheduler = VideoDecodeScheduler::new(3);

        assert_eq!(
            scheduler.push_input(first.clone(), VideoDecodePriority::Current),
            VideoDecodeSchedulePush::Inserted
        );
        assert_eq!(
            scheduler.push_input(second.clone(), VideoDecodePriority::Current),
            VideoDecodeSchedulePush::Inserted
        );
        assert_eq!(
            scheduler.push_input(first.clone(), VideoDecodePriority::Current),
            VideoDecodeSchedulePush::Duplicate
        );
        assert_eq!(scheduler.len(), 2);
        assert_eq!(
            scheduler
                .pending()
                .into_iter()
                .map(|scheduled| scheduled.input_key)
                .collect::<Vec<_>>(),
            vec![first.key, second.key]
        );
        assert_eq!(
            VideoRenderInput::legacy(first.request).key,
            protocol::VideoRenderInputKey::LEGACY
        );
    }

    #[test]
    fn ffmpeg_cache_is_owned_by_render_input_key_not_layer_id() {
        let binary = fake_ffmpeg_binary();
        let request = VideoFrameRequest {
            layer_id: 21,
            label: "Shared authored layer".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some("same-layer.mp4".to_string()),
                name: None,
                codec: Some("h264".to_string()),
                metadata: None,
            },
            position_ms: 250,
            width: 1,
            height: 1,
        };
        let first = VideoRenderInput::new(
            protocol::VideoRenderInputKey {
                project_render_epoch: 4,
                input_id: protocol::VideoRenderInputId(1),
            },
            request.clone(),
        );
        let second = VideoRenderInput::new(
            protocol::VideoRenderInputKey {
                project_render_epoch: 4,
                input_id: protocol::VideoRenderInputId(2),
            },
            request,
        );
        let mut decoder = FfmpegCliFrameDecoder::new(&binary);

        assert_eq!(
            decoder.decode_input_frame(&first).unwrap().unwrap().data,
            b"ABCD"
        );
        assert_eq!(
            decoder.decode_input_frame(&second).unwrap().unwrap().data,
            b"ABCD"
        );
        assert_eq!(decoder.cache_len(), 2);
        let _ = std::fs::remove_file(&binary);
        assert_eq!(
            decoder.decode_input_frame(&first).unwrap().unwrap().data,
            b"ABCD"
        );
        decoder.retain_inputs(&[second]);
        assert_eq!(decoder.cache_len(), 1);
    }

    #[derive(Default)]
    struct C1SolidFrameProvider {
        pixels: HashMap<VideoLayerId, [u8; 4]>,
        fail: bool,
    }

    impl VideoFrameProvider for C1SolidFrameProvider {
        fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

        fn frame_for_layer(
            &mut self,
            layer: &VideoLayerSummary,
            width: u32,
            height: u32,
        ) -> Result<VideoFrame, VideoFrameProviderError> {
            if self.fail {
                return Err(VideoFrameProviderError::Decode {
                    layer_id: layer.id,
                    label: layer.label.clone(),
                    error: VideoDecodeError::Decode {
                        layer_id: layer.id,
                        label: layer.label.clone(),
                        message: "injected C1 decode failure".to_string(),
                    },
                });
            }
            let pixel = self
                .pixels
                .get(&layer.id)
                .copied()
                .unwrap_or([0, 0, 0, 255]);
            Ok(VideoFrame {
                layer_id: layer.id,
                width,
                height,
                pts_ms: layer.state.position_ms,
                duration_ms: 16,
                format: VideoPixelFormat::Rgba8,
                data: pixel.repeat(width as usize * height as usize),
            })
        }
    }

    fn c1_test_layer(id: VideoLayerId, blend_mode: VideoBlendMode) -> VideoLayerSummary {
        VideoLayerSummary {
            id,
            label: format!("Layer {id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(format!("layer-{id}.mp4")),
                name: None,
                codec: None,
                metadata: None,
            },
            media_asset_id: None,
            blend_mode,
            state: VideoLayerState::default(),
            isf_effect: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
        }
    }

    fn c1_test_snapshot(layers: Vec<VideoLayerSummary>) -> VideoSnapshot {
        let layer_ids = layers.iter().map(|layer| layer.id).collect::<Vec<_>>();
        VideoSnapshot {
            layers,
            media_assets: Vec::new(),
            effect_chains: Vec::new(),
            effect_presets: Vec::new(),
            layer_groups: Vec::new(),
            transition_buses: Vec::new(),
            compositions: vec![CompositionSummary {
                id: 70,
                label: "C1 Composition".to_string(),
                layer_ids,
                output_ids: vec![80],
            }],
            outputs: vec![VideoOutputSummary {
                id: 80,
                label: "C1 Output".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 70,
                fullscreen: false,
                monitor_id: Some(0),
                monitor_identity: None,
                width: 1,
                height: 1,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: VideoOutputMapping::default(),
            }],
            mapping_presets: Vec::new(),
            auto_vj: AutoVjSnapshot::default(),
            master_opacity: 1.0,
            blackout: false,
        }
    }

    fn c1_chain(
        chain_id: u64,
        scope: VideoEffectScope,
        bypassed: bool,
        effects: Vec<VideoIsfEffectSummary>,
    ) -> VideoEffectChainSummary {
        VideoEffectChainSummary {
            id: VideoEffectChainId(chain_id),
            scope,
            bypassed,
            stages: effects
                .into_iter()
                .enumerate()
                .map(|(index, effect)| protocol::VideoEffectStageSummary {
                    id: VideoEffectStageId(chain_id * 100 + index as u64 + 1),
                    enabled: effect.enabled,
                    label: effect.label.clone(),
                    effect: protocol::VideoEffectSummary {
                        id: VideoEffectId(chain_id * 1_000 + index as u64 + 1),
                        kind: VideoEffectKind::Isf { effect },
                    },
                })
                .collect(),
        }
    }

    fn c1_invalid_effect(label: &str) -> VideoIsfEffectSummary {
        VideoIsfEffectSummary {
            enabled: true,
            label: label.to_string(),
            source: Arc::<str>::from(format!("invalid ISF source for {label}")),
            source_path: None,
            description: None,
            categories: Vec::new(),
            controls: Vec::new(),
            stack: Vec::new(),
        }
    }

    fn c1_disabled_legacy_stack(stage_count: usize) -> VideoIsfEffectSummary {
        assert!(stage_count > 0);
        let mut root = c1_invalid_effect("disabled legacy root");
        root.enabled = false;
        root.stack = (1..stage_count)
            .map(|index| protocol::VideoIsfEffectStageSummary {
                enabled: false,
                label: format!("disabled legacy stage {index}"),
                source: Arc::clone(&root.source),
                source_path: None,
                description: None,
                categories: Vec::new(),
                controls: Vec::new(),
            })
            .collect();
        root
    }

    #[test]
    fn c1_layer_resolution_keeps_canonical_ids_and_uses_legacy_projection_once() {
        let effect = builtin_isf_effect("invert").unwrap().unwrap();
        let mut layer = c1_test_layer(4, VideoBlendMode::Normal);
        layer.isf_effect = Some(effect.clone());
        let mut snapshot = c1_test_snapshot(vec![layer]);
        snapshot.effect_chains.push(c1_chain(
            9,
            VideoEffectScope::Layer { layer_id: 4 },
            false,
            vec![effect],
        ));

        let resolved =
            resolve_video_effect_chain(&snapshot, &VideoEffectScope::Layer { layer_id: 4 })
                .unwrap()
                .unwrap();

        assert_eq!(resolved.chain_id, Some(VideoEffectChainId(9)));
        assert_eq!(resolved.stages.len(), 1);
        assert_eq!(resolved.stages[0].stage_id, Some(VideoEffectStageId(901)));
        assert_eq!(resolved.stages[0].effect_id, Some(VideoEffectId(9_001)));
        assert_eq!(
            resolved.stages[0].effect.source,
            snapshot.layers[0].isf_effect.as_ref().unwrap().source
        );
    }

    #[test]
    fn c1_resolver_fails_closed_above_the_protocol_stage_limit() {
        let mut snapshot = c1_test_snapshot(vec![c1_test_layer(4, VideoBlendMode::Normal)]);
        snapshot.effect_chains.push(c1_chain(
            10,
            VideoEffectScope::Output { output_id: 80 },
            false,
            (0..=VIDEO_EFFECT_CHAIN_MAX_STAGES)
                .map(|index| c1_invalid_effect(&format!("stage-{index}")))
                .collect(),
        ));

        let fault =
            resolve_video_effect_chain(&snapshot, &VideoEffectScope::Output { output_id: 80 })
                .unwrap_err();

        assert_eq!(fault.chain_id, Some(VideoEffectChainId(10)));
        assert!(fault.message.contains("limit is 32"));
    }

    #[test]
    fn c1_shader_source_cache_is_bounded_lru_including_failures() {
        let mut renderer = VideoPreviewRenderer::new(VideoRuntimeConfig::default());
        for index in 0..ISF_SHADER_CACHE_CAPACITY {
            let source = Arc::<str>::from(format!("invalid shader {index}"));
            assert!(renderer.cached_isf_shader(&source).is_err());
        }
        let newest = Arc::<str>::from("invalid shader 0");
        assert!(renderer.cached_isf_shader(&newest).is_err());
        let overflow = Arc::<str>::from("invalid shader overflow");
        assert!(renderer.cached_isf_shader(&overflow).is_err());

        assert_eq!(renderer.isf_shader_cache.len(), ISF_SHADER_CACHE_CAPACITY);
        assert!(renderer.isf_shader_cache.contains_key("invalid shader 0"));
        assert!(!renderer.isf_shader_cache.contains_key("invalid shader 1"));
        assert!(renderer
            .isf_shader_cache
            .contains_key("invalid shader overflow"));
    }

    #[test]
    fn c1_bypassed_group_chain_is_pixel_identical_to_legacy_flat_composition() {
        let mut snapshot = c1_test_snapshot(vec![
            c1_test_layer(1, VideoBlendMode::Normal),
            c1_test_layer(2, VideoBlendMode::Add),
        ]);
        snapshot
            .layer_groups
            .push(protocol::VideoLayerGroupSummary {
                id: protocol::VideoLayerGroupId(7),
                label: "Group".to_string(),
                composition_id: 70,
                layer_ids: vec![1, 2],
            });
        snapshot.effect_chains.push(c1_chain(
            11,
            VideoEffectScope::Group {
                group_id: protocol::VideoLayerGroupId(7),
            },
            true,
            vec![c1_invalid_effect("must remain dormant")],
        ));
        let provider = C1SolidFrameProvider {
            pixels: HashMap::from([(1, [100, 10, 0, 255]), (2, [0, 90, 20, 255])]),
            fail: false,
        };
        let mut legacy = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: provider.pixels.clone(),
                fail: false,
            },
        );
        let mut scoped =
            VideoPreviewRenderer::with_frame_provider(VideoRuntimeConfig::default(), provider);

        let legacy_frame = legacy.render_output(&snapshot, 80).unwrap();
        let scoped_frame = scoped
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &VideoClipRuntimeSnapshot::default(),
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();

        assert_eq!(scoped_frame.data, legacy_frame.data);
        assert!(scoped.last_effect_stage_faults().is_empty());
    }

    #[test]
    fn c1_output_failure_reuses_only_matching_epoch_last_valid_artistic_frame() {
        let snapshot = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(1, [12, 34, 56, 255])]),
                fail: false,
            },
        );
        let runtime = VideoClipRuntimeSnapshot::default();
        let first = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 41,
                },
                80,
            )
            .unwrap();
        renderer.frame_provider_mut().fail = true;
        let last_valid = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 41,
                },
                80,
            )
            .unwrap();
        let new_epoch = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 42,
                },
                80,
            )
            .unwrap();

        assert_eq!(first.data, vec![12, 34, 56, 255]);
        assert_eq!(last_valid.data, first.data);
        assert_eq!(new_epoch.data, vec![0, 0, 0, 0]);
        assert!(renderer.last_output_render_error().is_some());
    }

    #[test]
    fn c1_output_last_valid_invalidates_dimensions_route_and_composition_identity() {
        let base = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        let mut resized = base.clone();
        resized.outputs[0].width = 2;
        let mut rerouted = base.clone();
        rerouted.outputs[0].fullscreen = true;
        let mut recomposed = base.clone();
        recomposed.compositions.push(CompositionSummary {
            id: 71,
            label: "Replacement Composition".to_string(),
            layer_ids: vec![1],
            output_ids: vec![80],
        });
        recomposed.outputs[0].composition_id = 71;

        for (changed, transparent_bytes) in [
            (resized, vec![0; 8]),
            (rerouted, vec![0; 4]),
            (recomposed, vec![0; 4]),
        ] {
            let runtime = VideoClipRuntimeSnapshot::default();
            let mut renderer = VideoPreviewRenderer::with_frame_provider(
                VideoRuntimeConfig::default(),
                C1SolidFrameProvider {
                    pixels: HashMap::from([(1, [12, 34, 56, 255])]),
                    fail: false,
                },
            );
            renderer
                .render_output_with_effects(
                    &base,
                    VideoEffectRenderContext {
                        clip_runtime: &runtime,
                        project_render_epoch: 1,
                    },
                    80,
                )
                .unwrap();
            renderer.frame_provider_mut().fail = true;

            let fallback = renderer
                .render_output_with_effects(
                    &changed,
                    VideoEffectRenderContext {
                        clip_runtime: &runtime,
                        project_render_epoch: 1,
                    },
                    80,
                )
                .unwrap();

            assert_eq!(fallback.data, transparent_bytes);
        }
    }

    #[test]
    fn c1_output_last_valid_is_cached_before_mapping_and_remapped_on_fallback() {
        let base = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        let mut remapped = base.clone();
        remapped.outputs[0].mapping.black_level = 0.2;
        let runtime = VideoClipRuntimeSnapshot::default();
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(1, [0, 0, 0, 255])]),
                fail: false,
            },
        );
        let first = renderer
            .render_output_with_effects(
                &base,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();
        renderer.frame_provider_mut().fail = true;

        let fallback = renderer
            .render_output_with_effects(
                &remapped,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();

        assert_eq!(first.data, vec![0, 0, 0, 255]);
        assert_eq!(fallback.data, vec![51, 51, 51, 255]);
    }

    #[test]
    fn c1_output_preview_preserves_mapping_fault_blackout_and_size_fenced_last_valid() {
        IsfGpuRuntime::new().expect("C1 output-preview test requires a GPU adapter");
        let mut snapshot = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        snapshot.outputs[0].mapping.black_level = 0.2;
        snapshot.effect_chains.push(c1_chain(
            104,
            VideoEffectScope::Output { output_id: 80 },
            false,
            vec![c1_invalid_effect("preview output fault")],
        ));
        let runtime = VideoClipRuntimeSnapshot::default();
        let context = VideoEffectRenderContext {
            clip_runtime: &runtime,
            project_render_epoch: 71,
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(1, [10, 20, 30, 255])]),
                fail: false,
            },
        );

        let seeded = renderer
            .render_output_preview_with_effects(&snapshot, context, 80, 2, 1)
            .unwrap();
        assert_eq!((seeded.width, seeded.height), (2, 1));
        assert_eq!(seeded.data, vec![59, 67, 75, 255, 59, 67, 75, 255]);
        assert_eq!(renderer.last_effect_stage_faults().len(), 1);
        assert_eq!(
            renderer.last_effect_stage_faults()[0].chain_id,
            Some(VideoEffectChainId(104))
        );

        renderer.frame_provider_mut().fail = true;
        let same_size_fallback = renderer
            .render_output_preview_with_effects(&snapshot, context, 80, 2, 1)
            .unwrap();
        assert_eq!(same_size_fallback, seeded);
        assert!(renderer.last_output_render_error().is_some());

        let resized_failure = renderer
            .render_output_preview_with_effects(&snapshot, context, 80, 1, 1)
            .unwrap();
        assert_eq!(resized_failure.data, vec![51, 51, 51, 0]);

        snapshot.blackout = true;
        let blackout = renderer
            .render_output_preview_with_effects(&snapshot, context, 80, 2, 1)
            .unwrap();
        assert_eq!(blackout.data, vec![0, 0, 0, 255, 0, 0, 0, 255]);
        assert!(renderer.output_last_valid_frames.is_empty());
        assert_eq!(renderer.last_output_render_error(), None);
        assert!(renderer.last_effect_stage_faults().is_empty());
    }

    #[test]
    fn c1_successful_empty_composition_replaces_an_older_output_last_valid_frame() {
        let mut snapshot = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        let runtime = VideoClipRuntimeSnapshot::default();
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(1, [80, 90, 100, 255])]),
                fail: false,
            },
        );
        let visible = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();
        snapshot.layers[0].state.enabled = false;
        let hidden = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();
        snapshot.layers[0].state.enabled = true;
        renderer.frame_provider_mut().fail = true;
        let fallback = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();

        assert_eq!(visible.data, vec![80, 90, 100, 255]);
        assert_eq!(hidden.data, vec![0, 0, 0, 0]);
        assert_eq!(fallback.data, hidden.data);
    }

    #[test]
    fn c1_calibration_test_pattern_bypasses_artistic_effect_chains() {
        let mut snapshot = c1_test_snapshot(Vec::new());
        let without_effects = render_video_output_test_pattern(&snapshot, 80, 8, 8).unwrap();
        snapshot.effect_chains.extend([
            c1_chain(
                91,
                VideoEffectScope::Composition { composition_id: 70 },
                false,
                vec![c1_invalid_effect("calibration must bypass Composition")],
            ),
            c1_chain(
                92,
                VideoEffectScope::Output { output_id: 80 },
                false,
                vec![c1_invalid_effect("calibration must bypass Output")],
            ),
        ]);

        let with_effects = render_video_output_test_pattern(&snapshot, 80, 8, 8).unwrap();

        assert_eq!(with_effects, without_effects);
        assert!(with_effects
            .data
            .chunks_exact(4)
            .any(|pixel| pixel != [0, 0, 0, 0]));
    }

    #[test]
    fn c1_transition_scope_remains_dormant_and_fail_closed_for_rendering() {
        let mut snapshot = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        snapshot.effect_chains.push(c1_chain(
            12,
            VideoEffectScope::Transition {
                owner: protocol::VideoTransitionEffectOwner::ClipTake { layer_id: 1 },
            },
            false,
            vec![c1_invalid_effect("dormant transition")],
        ));
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(1, [20, 40, 60, 255])]),
                fail: false,
            },
        );

        let frame = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &VideoClipRuntimeSnapshot::default(),
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();

        assert_eq!(frame.data, vec![20, 40, 60, 255]);
        assert!(renderer.last_effect_stage_faults().is_empty());
    }

    #[test]
    fn c1_empty_hidden_and_master_zero_compositions_still_run_artistic_scopes_in_order() {
        let mut empty = c1_test_snapshot(Vec::new());
        let mut hidden_layer = c1_test_layer(1, VideoBlendMode::Normal);
        hidden_layer.state.enabled = false;
        let mut hidden = c1_test_snapshot(vec![hidden_layer]);
        let mut master_zero = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        master_zero.master_opacity = 0.0;

        for snapshot in [&mut empty, &mut hidden, &mut master_zero] {
            snapshot.effect_chains.extend([
                c1_chain(
                    90,
                    VideoEffectScope::Composition { composition_id: 70 },
                    false,
                    vec![c1_invalid_effect("empty Composition stage")],
                ),
                c1_chain(
                    2,
                    VideoEffectScope::Output { output_id: 80 },
                    false,
                    vec![c1_invalid_effect("empty Output stage")],
                ),
            ]);
            let mut renderer = VideoPreviewRenderer::with_frame_provider(
                VideoRuntimeConfig::default(),
                C1SolidFrameProvider::default(),
            );

            let frame = renderer
                .render_output_with_effects(
                    snapshot,
                    VideoEffectRenderContext {
                        clip_runtime: &VideoClipRuntimeSnapshot::default(),
                        project_render_epoch: 1,
                    },
                    80,
                )
                .unwrap();

            assert_eq!(frame.data, vec![0, 0, 0, 0]);
            assert_eq!(renderer.last_effect_stage_faults().len(), 2);
            assert_eq!(
                renderer.last_effect_stage_faults()[0].scope,
                VideoEffectScope::Composition { composition_id: 70 }
            );
            assert_eq!(
                renderer.last_effect_stage_faults()[1].scope,
                VideoEffectScope::Output { output_id: 80 }
            );

            for chain in &mut snapshot.effect_chains {
                chain.bypassed = true;
            }
            renderer
                .render_output_with_effects(
                    snapshot,
                    VideoEffectRenderContext {
                        clip_runtime: &VideoClipRuntimeSnapshot::default(),
                        project_render_epoch: 1,
                    },
                    80,
                )
                .unwrap();
            assert!(renderer.last_effect_stage_faults().is_empty());
            assert_eq!(renderer.last_isf_error(), None);
        }
    }

    #[test]
    fn c1_valid_composition_and_output_chains_transform_an_empty_base_in_order() {
        IsfGpuRuntime::new().expect("C1 empty artistic-chain test requires a GPU adapter");
        const SOLID_RED: &str = r#"/*{
          "INPUTS": [{"NAME":"inputImage","TYPE":"image"}]
        }*/
        void main() {
          gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
        }"#;
        let prepared = prepare_isf_shader(SOLID_RED).unwrap();
        let solid_red = isf_effect_from_prepared(
            "Solid Red".to_string(),
            SOLID_RED.to_string(),
            None,
            &prepared,
        );
        let mut snapshot = c1_test_snapshot(Vec::new());
        snapshot.effect_chains.extend([
            c1_chain(
                93,
                VideoEffectScope::Composition { composition_id: 70 },
                false,
                vec![solid_red],
            ),
            c1_chain(
                94,
                VideoEffectScope::Output { output_id: 80 },
                false,
                vec![builtin_isf_effect("invert").unwrap().unwrap()],
            ),
        ]);
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider::default(),
        );

        let frame = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &VideoClipRuntimeSnapshot::default(),
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();

        assert_eq!(frame.data, vec![0, 255, 255, 255]);
        assert!(renderer.last_effect_stage_faults().is_empty());
    }

    #[test]
    fn c1_master_blackout_fences_artistic_generators_faults_and_last_valid() {
        IsfGpuRuntime::new().expect("C1 master-blackout test requires a GPU adapter");
        const SOLID_RED: &str = r#"/*{
          "INPUTS": [{"NAME":"inputImage","TYPE":"image"}]
        }*/
        void main() {
          gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
        }"#;
        let prepared = prepare_isf_shader(SOLID_RED).unwrap();
        let solid_red = isf_effect_from_prepared(
            "Solid Red".to_string(),
            SOLID_RED.to_string(),
            None,
            &prepared,
        );
        let mut snapshot = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        snapshot.effect_chains.extend([
            c1_chain(
                95,
                VideoEffectScope::Composition { composition_id: 70 },
                false,
                vec![c1_invalid_effect("blocked Composition fault"), solid_red],
            ),
            c1_chain(
                96,
                VideoEffectScope::Output { output_id: 80 },
                false,
                vec![
                    c1_invalid_effect("blocked Output fault"),
                    builtin_isf_effect("invert").unwrap().unwrap(),
                ],
            ),
        ]);
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(1, [10, 20, 30, 255])]),
                fail: false,
            },
        );
        let runtime = VideoClipRuntimeSnapshot::default();

        let before_blackout = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();
        assert_eq!(before_blackout.data, vec![0, 255, 255, 255]);
        assert_eq!(renderer.last_effect_stage_faults().len(), 2);

        snapshot.blackout = true;
        let blackout = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();
        assert_eq!(blackout.data, vec![0, 0, 0, 255]);
        assert!(renderer.last_effect_stage_faults().is_empty());
        assert_eq!(renderer.last_isf_error(), None);

        snapshot.blackout = false;
        let recovered = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();
        assert_eq!(recovered.data, vec![0, 255, 255, 255]);
        assert_eq!(renderer.last_effect_stage_faults().len(), 2);

        snapshot.blackout = true;
        renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();
        snapshot.blackout = false;
        renderer.frame_provider_mut().fail = true;
        let failed_recovery = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();

        assert_eq!(failed_recovery.data, vec![0, 0, 0, 0]);
        assert!(renderer.last_effect_stage_faults().is_empty());
        assert!(renderer.last_output_render_error().is_some());
    }

    #[test]
    fn c1_direct_artistic_blackout_clears_cache_error_and_diagnostics() {
        let mut snapshot = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        snapshot.effect_chains.push(c1_chain(
            97,
            VideoEffectScope::Output { output_id: 80 },
            false,
            vec![c1_invalid_effect("direct seam diagnostic")],
        ));
        let runtime = VideoClipRuntimeSnapshot::default();
        let context = VideoEffectRenderContext {
            clip_runtime: &runtime,
            project_render_epoch: 1,
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(1, [30, 60, 90, 255])]),
                fail: false,
            },
        );

        let seeded = renderer
            .render_output_with_effects(&snapshot, context, 80)
            .unwrap();
        assert_eq!(seeded.data, vec![30, 60, 90, 255]);
        assert_eq!(renderer.output_last_valid_frames.len(), 1);
        assert_eq!(renderer.last_effect_stage_faults().len(), 1);

        renderer.frame_provider_mut().fail = true;
        let last_valid = renderer
            .render_output_with_effects(&snapshot, context, 80)
            .unwrap();
        assert_eq!(last_valid.data, seeded.data);
        assert_eq!(renderer.output_last_valid_frames.len(), 1);
        assert!(renderer.last_output_render_error().is_some());

        snapshot.blackout = true;
        let blackout_plan = build_video_output_render_plan(&snapshot, 80).unwrap();
        let blackout = renderer
            .prepare_output_artistic_frame_with_effects(
                &snapshot,
                context,
                &blackout_plan,
                blackout_plan.width,
                blackout_plan.height,
            )
            .unwrap();
        assert_eq!(blackout.data, vec![0, 0, 0, 255]);
        assert!(renderer.output_last_valid_frames.is_empty());
        assert_eq!(renderer.last_output_render_error(), None);
        assert!(renderer.last_effect_stage_faults().is_empty());
        assert_eq!(renderer.last_isf_error(), None);

        snapshot.blackout = false;
        let failed_recovery = renderer
            .render_output_with_effects(&snapshot, context, 80)
            .unwrap();
        assert_eq!(failed_recovery.data, vec![0, 0, 0, 0]);
        assert!(renderer.last_output_render_error().is_some());
    }

    #[test]
    fn c1_wrapper_and_direct_blackout_clear_keyed_input_diagnostics() {
        let invalid = c1_invalid_effect("keyed fault before blackout");
        let mut layer = c1_test_layer(7, VideoBlendMode::Normal);
        layer.isf_effect = Some(invalid.clone());
        let mut snapshot = c1_test_snapshot(vec![layer]);
        snapshot.effect_chains.push(c1_chain(
            98,
            VideoEffectScope::Layer { layer_id: 7 },
            false,
            vec![invalid],
        ));
        let runtime = VideoClipRuntimeSnapshot::default();
        let context = VideoEffectRenderContext {
            clip_runtime: &runtime,
            project_render_epoch: 1,
        };
        let keyed_input = |input_id| {
            let key = protocol::VideoRenderInputKey {
                project_render_epoch: 1,
                input_id: protocol::VideoRenderInputId(input_id),
            };
            (
                key,
                VideoRenderInput::new(key, decode_request(7, 0)),
                CompositionInputMix {
                    inputs: vec![CompositionRenderInput {
                        key,
                        layer: layer_plan(7, VideoBlendMode::Normal, 1.0),
                    }],
                },
            )
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(7, [20, 40, 60, 255])]),
                fail: false,
            },
        );

        let (first_key, first_input, first_mix) = keyed_input(1);
        renderer
            .render_input_mix_with_effects(&snapshot, context, &[first_input], &first_mix, 1, 1)
            .unwrap();
        assert_eq!(renderer.last_render_input_isf_stage_errors().len(), 1);
        assert_eq!(
            renderer.last_render_input_isf_stage_errors()[0].key,
            first_key
        );

        snapshot.blackout = true;
        renderer
            .render_output_with_effects(&snapshot, context, 80)
            .unwrap();
        assert!(renderer.last_render_input_isf_stage_errors().is_empty());

        snapshot.blackout = false;
        let (second_key, second_input, second_mix) = keyed_input(2);
        renderer
            .render_input_mix_with_effects(&snapshot, context, &[second_input], &second_mix, 1, 1)
            .unwrap();
        assert_eq!(renderer.last_render_input_isf_stage_errors().len(), 1);
        assert_eq!(
            renderer.last_render_input_isf_stage_errors()[0].key,
            second_key
        );

        snapshot.blackout = true;
        let blackout_plan = build_video_output_render_plan(&snapshot, 80).unwrap();
        renderer
            .prepare_output_artistic_frame_with_effects(
                &snapshot,
                context,
                &blackout_plan,
                blackout_plan.width,
                blackout_plan.height,
            )
            .unwrap();
        assert!(renderer.last_render_input_isf_stage_errors().is_empty());

        snapshot.blackout = false;
        let (third_key, third_input, third_mix) = keyed_input(3);
        renderer
            .render_input_mix_with_effects(&snapshot, context, &[third_input], &third_mix, 1, 1)
            .unwrap();
        assert_eq!(renderer.last_render_input_isf_stage_errors().len(), 1);
        assert_eq!(
            renderer.last_render_input_isf_stage_errors()[0].key,
            third_key
        );
    }

    #[test]
    fn c1_keyed_input_fault_keeps_authored_and_runtime_identity_separate() {
        let invalid = c1_invalid_effect("keyed broken layer stage");
        let mut layer = c1_test_layer(7, VideoBlendMode::Normal);
        layer.isf_effect = Some(invalid.clone());
        let mut snapshot = c1_test_snapshot(vec![layer]);
        snapshot.effect_chains.push(c1_chain(
            14,
            VideoEffectScope::Layer { layer_id: 7 },
            false,
            vec![invalid],
        ));
        let key = protocol::VideoRenderInputKey {
            project_render_epoch: 91,
            input_id: protocol::VideoRenderInputId(5),
        };
        let input = VideoRenderInput::new(key, decode_request(7, 0));
        let mix = CompositionInputMix {
            inputs: vec![CompositionRenderInput {
                key,
                layer: layer_plan(7, VideoBlendMode::Normal, 1.0),
            }],
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(7, [24, 96, 180, 255])]),
                fail: false,
            },
        );

        let output = renderer
            .render_input_mix_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &VideoClipRuntimeSnapshot::default(),
                    project_render_epoch: 91,
                },
                &[input],
                &mix,
                1,
                1,
            )
            .unwrap();

        assert_eq!(output.data, vec![24, 96, 180, 255]);
        assert_eq!(renderer.last_render_input_isf_stage_errors().len(), 1);
        assert_eq!(renderer.last_render_input_isf_stage_errors()[0].key, key);
        let fault = &renderer.last_effect_stage_faults()[0];
        assert_eq!(fault.render_input_key, Some(key));
        assert_eq!(fault.chain_id, Some(VideoEffectChainId(14)));
        assert_eq!(fault.stage_id, Some(VideoEffectStageId(1_401)));
        assert_eq!(fault.effect_id, Some(VideoEffectId(14_001)));
    }

    #[test]
    fn c1_keyed_missing_layer_legacy_fallback_enforces_the_32_stage_limit() {
        let snapshot = c1_test_snapshot(Vec::new());
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(7, [11, 22, 33, 255])]),
                fail: false,
            },
        );
        let runtime = VideoClipRuntimeSnapshot::default();
        let context = VideoEffectRenderContext {
            clip_runtime: &runtime,
            project_render_epoch: 1,
        };

        for (input_id, stage_count, expects_limit_fault) in [(1, 32, false), (2, 33, true)] {
            let key = protocol::VideoRenderInputKey {
                project_render_epoch: 1,
                input_id: protocol::VideoRenderInputId(input_id),
            };
            let input = VideoRenderInput::new(key, decode_request(7, 0))
                .with_isf_effect(Some(c1_disabled_legacy_stack(stage_count)));
            let mix = CompositionInputMix {
                inputs: vec![CompositionRenderInput {
                    key,
                    layer: layer_plan(7, VideoBlendMode::Normal, 1.0),
                }],
            };

            let frame = renderer
                .render_input_mix_with_effects(&snapshot, context, &[input], &mix, 1, 1)
                .unwrap();

            assert_eq!(frame.data, vec![11, 22, 33, 255]);
            if expects_limit_fault {
                assert_eq!(renderer.last_effect_stage_faults().len(), 1);
                let fault = &renderer.last_effect_stage_faults()[0];
                assert_eq!(fault.scope, VideoEffectScope::Layer { layer_id: 7 });
                assert_eq!(fault.render_input_key, Some(key));
                assert_eq!(fault.chain_id, None);
                assert!(fault.message.contains("33 stages; the limit is 32"));
                assert_eq!(renderer.last_render_input_isf_stage_errors().len(), 1);
                assert_eq!(renderer.last_render_input_isf_stage_errors()[0].key, key);
            } else {
                assert!(renderer.last_effect_stage_faults().is_empty());
                assert!(renderer.last_render_input_isf_stage_errors().is_empty());
            }
        }
    }

    #[test]
    fn c1_active_clip_runs_before_effective_layer_chain() {
        IsfGpuRuntime::new().expect("C1 Clip/Layer order test requires a GPU adapter");
        let invert = builtin_isf_effect("invert").unwrap().unwrap();
        let mut layer = c1_test_layer(1, VideoBlendMode::Normal);
        layer.isf_effect = Some(invert.clone());
        let mut snapshot = c1_test_snapshot(vec![layer]);
        snapshot.effect_chains.extend([
            c1_chain(
                15,
                VideoEffectScope::Clip {
                    layer_id: 1,
                    slot_id: VideoClipSlotId(2),
                },
                false,
                vec![invert.clone()],
            ),
            c1_chain(
                16,
                VideoEffectScope::Layer { layer_id: 1 },
                false,
                vec![invert],
            ),
        ]);
        let provider = C1SolidFrameProvider {
            pixels: HashMap::from([(1, [10, 20, 30, 255])]),
            fail: false,
        };
        let mut renderer =
            VideoPreviewRenderer::with_frame_provider(VideoRuntimeConfig::default(), provider);
        let active_runtime = VideoClipRuntimeSnapshot {
            layers: vec![protocol::VideoClipLayerRuntimeSummary {
                layer_id: 1,
                active_slot_id: Some(VideoClipSlotId(2)),
                ..protocol::VideoClipLayerRuntimeSummary::default()
            }],
            ..VideoClipRuntimeSnapshot::default()
        };

        let active = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &active_runtime,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();
        let layer_only = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &VideoClipRuntimeSnapshot::default(),
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();

        assert_eq!(active.data, vec![10, 20, 30, 255]);
        assert_eq!(layer_only.data, vec![245, 235, 225, 255]);
    }

    #[test]
    fn c2_clip_take_custom_decodes_distinct_slot_owners_neutral_blend_then_transition_chain() {
        struct ClipTakeFrameProvider;
        impl VideoFrameProvider for ClipTakeFrameProvider {
            fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

            fn frame_for_layer(
                &mut self,
                layer: &VideoLayerSummary,
                width: u32,
                height: u32,
            ) -> Result<VideoFrame, VideoFrameProviderError> {
                Ok(VideoFrame {
                    layer_id: layer.id,
                    width,
                    height,
                    pts_ms: layer.state.position_ms,
                    duration_ms: 16,
                    format: VideoPixelFormat::Rgba8,
                    data: [255, 0, 0, 255].repeat(width as usize * height as usize),
                })
            }

            fn frame_for_input(
                &mut self,
                input: &VideoRenderInput,
            ) -> Result<VideoFrame, VideoFrameProviderError> {
                assert_eq!(input.key.project_render_epoch, 77);
                assert_eq!(input.key.input_id.0, input.clip_slot_id.unwrap().0);
                let pixel = match input.clip_slot_id {
                    Some(VideoClipSlotId(20)) => [255, 0, 0, 255],
                    Some(VideoClipSlotId(21)) => [0, 0, 255, 255],
                    other => panic!("unexpected Clip Take input {other:?}"),
                };
                Ok(VideoFrame {
                    layer_id: input.layer_id(),
                    width: input.request.width,
                    height: input.request.height,
                    pts_ms: input.request.position_ms,
                    duration_ms: 16,
                    format: VideoPixelFormat::Rgba8,
                    data: pixel
                        .repeat(input.request.width as usize * input.request.height as usize),
                })
            }
        }

        let outgoing_source = VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some("outgoing.mp4".to_string()),
            name: None,
            codec: None,
            metadata: None,
        };
        let incoming_source = VideoSourceSummary {
            path: Some("incoming.mp4".to_string()),
            ..outgoing_source.clone()
        };
        let mut layer = c1_test_layer(1, VideoBlendMode::Normal);
        layer.source = outgoing_source.clone();
        layer.media_asset_id = Some(100);
        layer.clip_slots = vec![
            protocol::VideoClipSlotSummary {
                id: VideoClipSlotId(20),
                media_asset_id: 100,
                in_point_ms: 0,
                out_point_ms: None,
                loop_mode: protocol::VideoClipLoopMode::Once,
                speed: 1.0,
                cue_points: Vec::new(),
                launch_quantization: protocol::VideoClipLaunchQuantization::Immediate,
                effect_overrides: Vec::new(),
            },
            protocol::VideoClipSlotSummary {
                id: VideoClipSlotId(21),
                media_asset_id: 101,
                in_point_ms: 0,
                out_point_ms: None,
                loop_mode: protocol::VideoClipLoopMode::Once,
                speed: 1.0,
                cue_points: Vec::new(),
                launch_quantization: protocol::VideoClipLaunchQuantization::Immediate,
                effect_overrides: Vec::new(),
            },
        ];
        layer.default_clip_slot_id = Some(VideoClipSlotId(20));
        let mut snapshot = c1_test_snapshot(vec![layer]);
        snapshot.media_assets = vec![
            protocol::MediaAssetSummary {
                id: 100,
                label: "Outgoing".to_string(),
                source: outgoing_source,
                content_hash: None,
                byte_size: None,
            },
            protocol::MediaAssetSummary {
                id: 101,
                label: "Incoming".to_string(),
                source: incoming_source,
                content_hash: None,
                byte_size: None,
            },
        ];
        snapshot.effect_chains.push(c1_chain(
            212,
            VideoEffectScope::Transition {
                owner: protocol::VideoTransitionEffectOwner::ClipTake { layer_id: 1 },
            },
            false,
            vec![c1_invalid_effect(
                "Clip Take transition executes after blend",
            )],
        ));
        let runtime = VideoClipRuntimeSnapshot {
            layers: vec![protocol::VideoClipLayerRuntimeSummary {
                layer_id: 1,
                active_slot_id: Some(VideoClipSlotId(20)),
                queued_slot_id: Some(VideoClipSlotId(21)),
                transition: Some(protocol::VideoClipTakeTransitionSummary {
                    origin_slot_id: VideoClipSlotId(20),
                    outgoing_slot_id: VideoClipSlotId(20),
                    incoming_slot_id: VideoClipSlotId(21),
                    kind: protocol::VideoClipTakeKind::Custom,
                    elapsed_ms: 250,
                    duration_ms: 1_000,
                    duration: protocol::VideoClipTakeDuration::milliseconds(1_000),
                    progress_millis: 250,
                    incoming_playhead_ms: 600,
                    incoming_playing: true,
                }),
                playhead_ms: 400,
                playing: true,
                ..protocol::VideoClipLayerRuntimeSummary::default()
            }],
            ..VideoClipRuntimeSnapshot::default()
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            ClipTakeFrameProvider,
        );
        let frame = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 77,
                },
                80,
            )
            .unwrap();
        assert_eq!(frame.data, vec![191, 0, 64, 255]);
        assert_eq!(renderer.last_effect_stage_faults().len(), 1);
        assert_eq!(
            renderer.last_effect_stage_faults()[0].chain_id,
            Some(VideoEffectChainId(212))
        );
        snapshot.media_assets.retain(|asset| asset.id != 101);
        let fallback = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 77,
                },
                80,
            )
            .unwrap();
        assert_eq!(
            fallback, frame,
            "missing incoming media keeps last valid program"
        );
        assert!(renderer.last_output_render_error().is_some_and(|error| {
            error.contains("MissingTransitionAsset") && error.contains("asset_id: 101")
        }));
        let mut cold_renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            ClipTakeFrameProvider,
        );
        let cold_fallback = cold_renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 77,
                },
                80,
            )
            .unwrap();
        assert_eq!(cold_fallback.data, vec![0, 0, 0, 0]);
        let red = VideoFrame {
            layer_id: 1,
            width: 1,
            height: 1,
            pts_ms: 0,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: vec![255, 0, 0, 255],
        };
        let blue = VideoFrame {
            data: vec![0, 0, 255, 255],
            ..red.clone()
        };
        assert_eq!(
            crossfade_video_frames_rgba8(&red, &blue, 250).unwrap().data,
            crossfade_video_frames_rgba8(&blue, &red, 750).unwrap().data,
            "reversing source roles and complementing progress is pixel-continuous"
        );
    }

    #[test]
    fn c2_clip_take_transition_modes_are_distinct_bounded_and_reverse_continuous() {
        let outgoing = VideoFrame {
            layer_id: 1,
            width: 2,
            height: 1,
            pts_ms: 10,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: vec![255, 0, 0, 255, 0, 255, 0, 255],
        };
        let incoming = VideoFrame {
            pts_ms: 20,
            data: vec![0, 0, 255, 255, 255, 255, 255, 255],
            ..outgoing.clone()
        };
        assert_eq!(
            transition_video_frames_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Dip,
                500,
            )
            .unwrap()
            .data,
            vec![0, 0, 0, 255, 0, 0, 0, 255]
        );
        assert_eq!(
            transition_video_frames_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Wipe,
                500,
            )
            .unwrap()
            .data,
            vec![0, 0, 255, 255, 0, 255, 0, 255]
        );
        assert_eq!(
            transition_video_frames_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Luma,
                500,
            )
            .unwrap()
            .data,
            vec![0, 0, 255, 255, 0, 255, 0, 255]
        );
        for kind in [
            protocol::VideoClipTakeKind::Crossfade,
            protocol::VideoClipTakeKind::Dip,
            protocol::VideoClipTakeKind::Wipe,
            protocol::VideoClipTakeKind::Luma,
            protocol::VideoClipTakeKind::Displacement,
            protocol::VideoClipTakeKind::Blur,
            protocol::VideoClipTakeKind::Glitch,
            protocol::VideoClipTakeKind::Custom,
        ] {
            let forward = transition_video_frames_rgba8(&outgoing, &incoming, kind, 250).unwrap();
            // Runtime Reverse swaps the live source roles and complements the
            // public progress. The stable origin reorients them back before
            // evaluating the directional transition, preserving this frame.
            let reversed = transition_video_frames_rgba8(
                &outgoing,
                &incoming,
                kind,
                1000u16.saturating_sub(750),
            )
            .unwrap();
            assert_eq!(
                forward.data, reversed.data,
                "{kind:?} reverse discontinuity"
            );
            assert_eq!(
                transition_video_frames_rgba8(&outgoing, &incoming, kind, 0)
                    .unwrap()
                    .data,
                outgoing.data,
                "{kind:?} must start at outgoing"
            );
            assert_eq!(
                transition_video_frames_rgba8(&outgoing, &incoming, kind, 1000)
                    .unwrap()
                    .data,
                incoming.data,
                "{kind:?} must finish at incoming"
            );
        }
    }

    #[test]
    fn follow_output_transition_combiner_covers_kinds_curves_custom_and_fail_closed_inputs() {
        let outgoing = VideoFrame {
            layer_id: 71,
            width: 2,
            height: 1,
            pts_ms: 10,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: vec![255, 0, 0, 255, 0, 255, 0, 255],
        };
        let incoming = VideoFrame {
            layer_id: 72,
            pts_ms: 20,
            data: vec![0, 0, 255, 255, 255, 255, 255, 255],
            ..outgoing.clone()
        };
        let kinds = [
            protocol::VideoClipTakeKind::Cut,
            protocol::VideoClipTakeKind::Crossfade,
            protocol::VideoClipTakeKind::Dip,
            protocol::VideoClipTakeKind::Wipe,
            protocol::VideoClipTakeKind::Luma,
            protocol::VideoClipTakeKind::Displacement,
            protocol::VideoClipTakeKind::Blur,
            protocol::VideoClipTakeKind::Glitch,
            protocol::VideoClipTakeKind::Custom,
        ];
        for kind in kinds {
            for raw_progress_millis in [0, 500, 1000] {
                let result = combine_follow_output_transition_rgba8(
                    &outgoing,
                    &incoming,
                    kind,
                    VideoLayerTransitionCurve::Linear,
                    raw_progress_millis,
                )
                .unwrap();
                assert_eq!(result.evidence.kind, kind);
                assert_eq!(result.evidence.raw_progress_millis, raw_progress_millis);
                assert_eq!(result.evidence.curved_progress_millis, raw_progress_millis);
                if matches!(kind, protocol::VideoClipTakeKind::Cut) {
                    assert_eq!(result.frame.data, incoming.data, "Cut ignores progress");
                } else if raw_progress_millis == 0 {
                    assert_eq!(result.frame.data, outgoing.data, "{kind:?} start");
                } else if raw_progress_millis == 1000 {
                    assert_eq!(result.frame.data, incoming.data, "{kind:?} finish");
                }
            }
        }

        let crossfade = combine_follow_output_transition_rgba8(
            &outgoing,
            &incoming,
            protocol::VideoClipTakeKind::Crossfade,
            VideoLayerTransitionCurve::Linear,
            500,
        )
        .unwrap();
        let dip = combine_follow_output_transition_rgba8(
            &outgoing,
            &incoming,
            protocol::VideoClipTakeKind::Dip,
            VideoLayerTransitionCurve::Linear,
            500,
        )
        .unwrap();
        assert_ne!(
            dip.frame.data, crossfade.frame.data,
            "non-crossfade output must not collapse to opacity blending"
        );
        let custom = combine_follow_output_transition_rgba8(
            &outgoing,
            &incoming,
            protocol::VideoClipTakeKind::Custom,
            VideoLayerTransitionCurve::EaseInOut,
            400,
        )
        .unwrap();
        let neutral_crossfade = combine_follow_output_transition_rgba8(
            &outgoing,
            &incoming,
            protocol::VideoClipTakeKind::Crossfade,
            VideoLayerTransitionCurve::EaseInOut,
            400,
        )
        .unwrap();
        assert_eq!(
            custom.frame, neutral_crossfade.frame,
            "Custom supplies the neutral blend before the caller applies its Transition chain"
        );
        assert_eq!(
            custom.evidence.curved_progress_millis,
            neutral_crossfade.evidence.curved_progress_millis
        );

        for curve in [
            VideoLayerTransitionCurve::Linear,
            VideoLayerTransitionCurve::EaseIn,
            VideoLayerTransitionCurve::EaseOut,
            VideoLayerTransitionCurve::EaseInOut,
        ] {
            let mut previous = 0;
            for raw_progress_millis in (0..=1000).step_by(10) {
                let current =
                    video_follow_output_transition_progress_millis(curve, raw_progress_millis);
                assert!(current >= previous, "{curve:?} must be monotonic");
                previous = current;
            }
            assert_eq!(video_follow_output_transition_progress_millis(curve, 0), 0);
            assert_eq!(
                video_follow_output_transition_progress_millis(curve, 1000),
                1000
            );
        }

        let mismatched = VideoFrame {
            width: 1,
            data: vec![1, 2, 3, 255],
            ..incoming.clone()
        };
        assert_eq!(
            combine_follow_output_transition_rgba8(
                &outgoing,
                &mismatched,
                protocol::VideoClipTakeKind::Cut,
                VideoLayerTransitionCurve::Linear,
                0,
            ),
            Err(CpuCompositeError::FrameSizeMismatch {
                layer_id: outgoing.layer_id
            })
        );
        assert_eq!(
            combine_follow_output_transition_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Crossfade,
                VideoLayerTransitionCurve::Linear,
                1001,
            ),
            Err(CpuCompositeError::InvalidTransitionProgress {
                progress_millis: 1001
            })
        );
        let transparent = VideoFrame {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0],
            ..outgoing.clone()
        };
        for kind in [
            protocol::VideoClipTakeKind::Cut,
            protocol::VideoClipTakeKind::Dip,
            protocol::VideoClipTakeKind::Wipe,
            protocol::VideoClipTakeKind::Glitch,
        ] {
            let source_only = combine_follow_output_transition_rgba8(
                &transparent,
                &incoming,
                kind,
                VideoLayerTransitionCurve::Linear,
                0,
            )
            .expect("transparent outgoing program frame is valid");
            let target_only = combine_follow_output_transition_rgba8(
                &outgoing,
                &transparent,
                kind,
                VideoLayerTransitionCurve::Linear,
                1000,
            )
            .expect("transparent incoming program frame is valid");
            if matches!(kind, protocol::VideoClipTakeKind::Cut) {
                assert_eq!(
                    source_only.frame.data, incoming.data,
                    "Cut always selects B"
                );
            } else {
                assert_eq!(
                    source_only.frame.data, transparent.data,
                    "{kind:?} starts at A"
                );
            }
            assert_eq!(
                target_only.frame.data, transparent.data,
                "{kind:?} finishes at B"
            );
        }
    }

    #[test]
    fn dip_transition_uses_a_continuous_rgba_envelope_through_opaque_black() {
        let transparent = VideoFrame {
            layer_id: 73,
            width: 1,
            height: 1,
            pts_ms: 0,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: vec![40, 80, 120, 0],
        };
        let opaque = VideoFrame {
            layer_id: 74,
            data: vec![10, 20, 30, 255],
            ..transparent.clone()
        };
        for (outgoing, incoming, expected_alpha) in [
            (&transparent, &opaque, [254_u8, 255, 255]),
            (&opaque, &transparent, [255_u8, 255, 254]),
        ] {
            let neighbors = [499, 500, 501].map(|progress| {
                transition_video_frames_rgba8(
                    outgoing,
                    incoming,
                    protocol::VideoClipTakeKind::Dip,
                    progress,
                )
                .unwrap()
            });
            assert_eq!(
                [
                    neighbors[0].data[3],
                    neighbors[1].data[3],
                    neighbors[2].data[3],
                ],
                expected_alpha,
                "dip alpha must approach and leave opaque black without a switch jump"
            );
            assert_eq!(neighbors[1].data, vec![0, 0, 0, 255]);
            assert!(i16::from(neighbors[1].data[3]).abs_diff(i16::from(neighbors[0].data[3])) <= 1);
            assert!(i16::from(neighbors[2].data[3]).abs_diff(i16::from(neighbors[1].data[3])) <= 1);
        }
        assert_eq!(
            transition_video_frames_rgba8(
                &transparent,
                &opaque,
                protocol::VideoClipTakeKind::Dip,
                0,
            )
            .unwrap()
            .data,
            transparent.data
        );
        assert_eq!(
            transition_video_frames_rgba8(
                &transparent,
                &opaque,
                protocol::VideoClipTakeKind::Dip,
                1000,
            )
            .unwrap()
            .data,
            opaque.data
        );
    }

    #[test]
    fn follow_output_renderer_custom_runs_transition_chain_once_and_faults_are_not_fresh() {
        IsfGpuRuntime::new().expect("Follow Custom transition-chain test requires a GPU adapter");
        const SOLID_RED: &str = r#"/*{
          "INPUTS": [{"NAME":"inputImage","TYPE":"image"}]
        }*/
        void main() {
          gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
        }"#;
        let source_timeline_id = TimelineId(701);
        let scope = VideoEffectScope::Transition {
            owner: VideoTransitionEffectOwner::TimelineFollow { source_timeline_id },
        };
        let prepared_solid = prepare_isf_shader(SOLID_RED).unwrap();
        let solid_red = isf_effect_from_prepared(
            "Solid Red".to_string(),
            SOLID_RED.to_string(),
            None,
            &prepared_solid,
        );
        let invert = builtin_isf_effect("invert").unwrap().unwrap();
        let solid_then_invert = resolved_chain_from_canonical(&c1_chain(
            713,
            scope.clone(),
            false,
            vec![solid_red, invert.clone()],
        ))
        .unwrap();
        let invert_only =
            resolved_chain_from_canonical(&c1_chain(714, scope.clone(), false, vec![invert]))
                .unwrap();
        let failed_chain = resolved_chain_from_canonical(&c1_chain(
            715,
            scope.clone(),
            false,
            vec![c1_invalid_effect("Follow Transition fault")],
        ))
        .unwrap();
        let wrong_timeline_owner = VideoTransitionEffectOwner::TimelineFollow {
            source_timeline_id: TimelineId(702),
        };
        let wrong_timeline_chain = resolved_chain_from_canonical(&c1_chain(
            716,
            VideoEffectScope::Transition {
                owner: wrong_timeline_owner.clone(),
            },
            false,
            vec![builtin_isf_effect("invert").unwrap().unwrap()],
        ))
        .unwrap();
        let clip_take_owner = VideoTransitionEffectOwner::ClipTake { layer_id: 71 };
        let clip_take_chain = resolved_chain_from_canonical(&c1_chain(
            717,
            VideoEffectScope::Transition {
                owner: clip_take_owner.clone(),
            },
            false,
            vec![builtin_isf_effect("invert").unwrap().unwrap()],
        ))
        .unwrap();
        let layer_bus_owner = VideoTransitionEffectOwner::LayerBus {
            bus_id: protocol::VideoTransitionBusId(718),
        };
        let layer_bus_chain = resolved_chain_from_canonical(&c1_chain(
            718,
            VideoEffectScope::Transition {
                owner: layer_bus_owner.clone(),
            },
            false,
            vec![builtin_isf_effect("invert").unwrap().unwrap()],
        ))
        .unwrap();
        let outgoing = VideoFrame {
            layer_id: 71,
            width: 1,
            height: 1,
            pts_ms: 1_000,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: vec![255, 0, 0, 255],
        };
        let incoming = VideoFrame {
            layer_id: 72,
            data: vec![0, 0, 255, 255],
            ..outgoing.clone()
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider::default(),
        );
        let crossfade = renderer
            .render_follow_output_transition_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Crossfade,
                VideoLayerTransitionCurve::Linear,
                500,
                source_timeline_id,
                None,
                None,
            )
            .unwrap();
        assert_eq!(crossfade.frame.data, vec![128, 0, 128, 255]);
        assert_eq!(
            crossfade.evidence.freshness,
            VideoOutputRenderFreshness::Fresh
        );

        let custom_ordered = renderer
            .render_follow_output_transition_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Custom,
                VideoLayerTransitionCurve::Linear,
                500,
                source_timeline_id,
                Some(&solid_then_invert),
                None,
            )
            .unwrap();
        assert_eq!(custom_ordered.frame.data, vec![0, 255, 255, 255]);
        assert_ne!(custom_ordered.frame, crossfade.frame);
        assert_eq!(
            custom_ordered.evidence.freshness,
            VideoOutputRenderFreshness::Fresh
        );
        assert!(custom_ordered.evidence.stage_faults.is_empty());
        assert_eq!(renderer.isf_last_stack_stage_count(), 2);

        let custom_inverted_once = renderer
            .render_follow_output_transition_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Custom,
                VideoLayerTransitionCurve::Linear,
                500,
                source_timeline_id,
                Some(&invert_only),
                None,
            )
            .unwrap();
        assert_eq!(custom_inverted_once.frame.data, vec![127, 255, 127, 255]);
        assert_ne!(custom_inverted_once.frame, crossfade.frame);
        assert_eq!(
            custom_inverted_once.evidence.freshness,
            VideoOutputRenderFreshness::Fresh
        );
        assert_eq!(renderer.isf_last_stack_stage_count(), 1);

        let fault = renderer
            .render_follow_output_transition_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Custom,
                VideoLayerTransitionCurve::Linear,
                500,
                source_timeline_id,
                Some(&failed_chain),
                None,
            )
            .unwrap();
        assert_eq!(fault.evidence.freshness, VideoOutputRenderFreshness::Error);
        assert!(fault.evidence.error.is_some());
        assert_eq!(fault.evidence.stage_faults.len(), 1);
        assert_eq!(
            fault.frame, crossfade.frame,
            "faulted chain cannot make a fresh Custom frame"
        );

        let last_valid = renderer
            .render_follow_output_transition_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Custom,
                VideoLayerTransitionCurve::Linear,
                500,
                source_timeline_id,
                Some(&failed_chain),
                Some(&crossfade.frame),
            )
            .unwrap();
        assert_eq!(
            last_valid.evidence.freshness,
            VideoOutputRenderFreshness::LastValid
        );
        assert!(last_valid.evidence.error.is_some());
        assert_eq!(last_valid.evidence.stage_faults.len(), 1);
        assert_eq!(last_valid.frame, crossfade.frame);

        assert_eq!(renderer.isf_last_stack_stage_count(), 0);
        assert_eq!(
            renderer.render_follow_output_transition_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Custom,
                VideoLayerTransitionCurve::Linear,
                500,
                source_timeline_id,
                Some(&wrong_timeline_chain),
                None,
            ),
            Err(CpuCompositeError::InvalidFollowTransitionEffectOwner {
                expected_source_timeline_id: source_timeline_id,
                owner: wrong_timeline_owner,
            })
        );
        assert_eq!(
            renderer.isf_last_stack_stage_count(),
            0,
            "mismatched TimelineFollow chain must not execute or produce fresh evidence"
        );
        assert_eq!(
            renderer.render_follow_output_transition_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Custom,
                VideoLayerTransitionCurve::Linear,
                500,
                source_timeline_id,
                Some(&clip_take_chain),
                None,
            ),
            Err(CpuCompositeError::InvalidFollowTransitionEffectOwner {
                expected_source_timeline_id: source_timeline_id,
                owner: clip_take_owner,
            })
        );
        assert_eq!(
            renderer.isf_last_stack_stage_count(),
            0,
            "ClipTake chain must not execute or produce fresh Follow evidence"
        );
        assert_eq!(
            renderer.render_follow_output_transition_rgba8(
                &outgoing,
                &incoming,
                protocol::VideoClipTakeKind::Custom,
                VideoLayerTransitionCurve::Linear,
                500,
                source_timeline_id,
                Some(&layer_bus_chain),
                None,
            ),
            Err(CpuCompositeError::InvalidFollowTransitionEffectOwner {
                expected_source_timeline_id: source_timeline_id,
                owner: layer_bus_owner,
            })
        );
        assert_eq!(
            renderer.isf_last_stack_stage_count(),
            0,
            "LayerBus chain must not execute or produce fresh Follow evidence"
        );
    }

    #[test]
    fn follow_output_render_evidence_distinguishes_fresh_last_valid_and_error() {
        let snapshot = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        let runtime = VideoClipRuntimeSnapshot::default();
        let context = VideoEffectRenderContext {
            clip_runtime: &runtime,
            project_render_epoch: 913,
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(1, [12, 34, 56, 255])]),
                fail: false,
            },
        );
        let fresh = renderer
            .render_output_with_effects_evidenced(&snapshot, context, 80)
            .unwrap();
        assert_eq!(fresh.evidence.freshness, VideoOutputRenderFreshness::Fresh);
        assert_eq!(fresh.evidence.error, None);
        renderer.frame_provider_mut().fail = true;
        let last_valid = renderer
            .render_output_with_effects_evidenced(&snapshot, context, 80)
            .unwrap();
        assert_eq!(last_valid.frame, fresh.frame);
        assert_eq!(
            last_valid.evidence.freshness,
            VideoOutputRenderFreshness::LastValid
        );
        assert!(last_valid.evidence.error.is_some());

        let mut cold_renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::new(),
                fail: true,
            },
        );
        let error = cold_renderer
            .render_output_with_effects_evidenced(&snapshot, context, 80)
            .unwrap();
        assert_eq!(error.frame.data, vec![0, 0, 0, 0]);
        assert_eq!(error.evidence.freshness, VideoOutputRenderFreshness::Error);
        assert!(error.evidence.error.is_some());
    }

    #[test]
    fn c3_layer_transition_bus_weights_only_opted_in_members_and_resolves_bus_scope() {
        let make_layer = |id, opacity| VideoLayerSummary {
            id,
            label: format!("Layer {id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(format!("memory://bus-{id}.mp4")),
                name: None,
                codec: None,
                metadata: None,
            },
            media_asset_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState {
                opacity,
                ..VideoLayerState::default()
            },
            isf_effect: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
        };
        let bus_id = protocol::VideoTransitionBusId(1);
        let from = VideoLayerTransitionTarget::Layer { layer_id: 1 };
        let to = VideoLayerTransitionTarget::Layer { layer_id: 2 };
        let mut snapshot = VideoSnapshot {
            layers: vec![make_layer(1, 0.8), make_layer(2, 0.6), make_layer(3, 0.4)],
            transition_buses: vec![protocol::VideoLayerTransitionBusSummary {
                id: bus_id,
                label: "Program bus".to_string(),
                composition_id: 1,
                enabled: true,
                members: vec![from.clone(), to.clone()],
                default_from: from.clone(),
                default_to: to.clone(),
                default_kind: protocol::VideoClipTakeKind::Crossfade,
                default_duration: protocol::VideoClipTakeDuration::milliseconds(1_000),
                default_curve: VideoLayerTransitionCurve::Linear,
                matte_source: None,
            }],
            compositions: vec![CompositionSummary {
                id: 1,
                label: "Main".to_string(),
                layer_ids: vec![1, 2, 3],
                output_ids: Vec::new(),
            }],
            ..VideoSnapshot::default()
        };
        snapshot.effect_chains.push(VideoEffectChainSummary {
            id: VideoEffectChainId(1),
            scope: VideoEffectScope::Transition {
                owner: VideoTransitionEffectOwner::LayerBus { bus_id },
            },
            bypassed: false,
            stages: Vec::new(),
        });
        let runtime = VideoLayerTransitionRuntimeSnapshot {
            buses: vec![protocol::VideoLayerTransitionBusRuntimeSummary {
                bus_id,
                origin_from: from.clone(),
                from: from.clone(),
                to: to.clone(),
                kind: protocol::VideoClipTakeKind::Crossfade,
                curve: VideoLayerTransitionCurve::Linear,
                elapsed_ms: 250,
                duration_ms: 1_000,
                duration: protocol::VideoClipTakeDuration::milliseconds(1_000),
                progress_millis: 250,
            }],
        };
        let mut plan = build_composition_plan(&snapshot, snapshot.compositions[0].clone());
        apply_video_layer_transition_weights(&snapshot, &runtime, &mut plan).unwrap();
        let opacity = |id| {
            plan.layers
                .iter()
                .find(|layer| layer.layer_id == id)
                .unwrap()
                .opacity
        };
        assert!((opacity(1) - 0.6).abs() < f32::EPSILON);
        assert!((opacity(2) - 0.15).abs() < f32::EPSILON);
        assert!((opacity(3) - 0.4).abs() < f32::EPSILON);
        assert!(resolve_video_effect_chain(
            &snapshot,
            &VideoEffectScope::Transition {
                owner: VideoTransitionEffectOwner::LayerBus { bus_id },
            },
        )
        .unwrap()
        .is_some());

        let reversed = VideoLayerTransitionRuntimeSnapshot {
            buses: vec![protocol::VideoLayerTransitionBusRuntimeSummary {
                from: to,
                to: from,
                elapsed_ms: 750,
                progress_millis: 750,
                ..runtime.buses[0].clone()
            }],
        };
        let mut reversed_plan = build_composition_plan(&snapshot, snapshot.compositions[0].clone());
        apply_video_layer_transition_weights(&snapshot, &reversed, &mut reversed_plan).unwrap();
        assert_eq!(
            plan.layers
                .iter()
                .map(|layer| (layer.layer_id, layer.opacity))
                .collect::<Vec<_>>(),
            reversed_plan
                .layers
                .iter()
                .map(|layer| (layer.layer_id, layer.opacity))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn c1_enabled_group_chain_isolates_members_and_preserves_internal_blend_order() {
        IsfGpuRuntime::new().expect("C1 Group isolation test requires a GPU adapter");
        let mut snapshot = c1_test_snapshot(vec![
            c1_test_layer(1, VideoBlendMode::Normal),
            c1_test_layer(2, VideoBlendMode::Add),
        ]);
        snapshot
            .layer_groups
            .push(protocol::VideoLayerGroupSummary {
                id: protocol::VideoLayerGroupId(8),
                label: "Isolated".to_string(),
                composition_id: 70,
                layer_ids: vec![1, 2],
            });
        snapshot.effect_chains.push(c1_chain(
            17,
            VideoEffectScope::Group {
                group_id: protocol::VideoLayerGroupId(8),
            },
            false,
            vec![builtin_isf_effect("invert").unwrap().unwrap()],
        ));
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(1, [100, 0, 0, 255]), (2, [0, 100, 0, 255])]),
                fail: false,
            },
        );

        let output = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &VideoClipRuntimeSnapshot::default(),
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();

        assert_eq!(output.data, vec![155, 155, 255, 255]);
    }

    #[test]
    fn c1_stage_local_failure_keeps_later_stage_and_stable_fault_identity() {
        IsfGpuRuntime::new().expect("C1 stage-local renderer test requires a GPU adapter");
        let mut snapshot = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        snapshot.effect_chains.push(c1_chain(
            13,
            VideoEffectScope::Output { output_id: 80 },
            false,
            vec![
                c1_invalid_effect("broken stage"),
                builtin_isf_effect("invert").unwrap().unwrap(),
            ],
        ));
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider {
                pixels: HashMap::from([(1, [10, 20, 30, 255])]),
                fail: false,
            },
        );

        let frame = renderer
            .render_output_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &VideoClipRuntimeSnapshot::default(),
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();

        assert_eq!(frame.data, vec![245, 235, 225, 255]);
        assert_eq!(renderer.last_effect_stage_faults().len(), 1);
        let fault = &renderer.last_effect_stage_faults()[0];
        assert_eq!(fault.chain_id, Some(VideoEffectChainId(13)));
        assert_eq!(fault.stage_id, Some(VideoEffectStageId(1_301)));
        assert_eq!(fault.effect_id, Some(VideoEffectId(13_001)));
        assert_eq!(fault.stage_index, Some(0));
    }

    #[test]
    fn c1_prepare_failure_topology_consumes_exactly_one_frame_index_per_chain() {
        IsfGpuRuntime::new().expect("C1 prepare-failure FRAMEINDEX test requires a GPU adapter");
        const WRITE_RED_INDEX: &str = r#"/*{
          "INPUTS": [{"NAME":"inputImage","TYPE":"image"}]
        }*/
        void main() {
          vec4 pixel = IMG_THIS_PIXEL(inputImage);
          pixel.r = float(FRAMEINDEX) / 255.0;
          gl_FragColor = pixel;
        }"#;
        const WRITE_GREEN_INDEX: &str = r#"/*{
          "INPUTS": [{"NAME":"inputImage","TYPE":"image"}]
        }*/
        void main() {
          vec4 pixel = IMG_THIS_PIXEL(inputImage);
          pixel.g = float(FRAMEINDEX) / 255.0;
          gl_FragColor = pixel;
        }"#;
        let write_red_prepared = prepare_isf_shader(WRITE_RED_INDEX).unwrap();
        let write_green_prepared = prepare_isf_shader(WRITE_GREEN_INDEX).unwrap();
        let write_red = isf_effect_from_prepared(
            "Write red FRAMEINDEX".to_string(),
            WRITE_RED_INDEX.to_string(),
            None,
            &write_red_prepared,
        );
        let write_green = isf_effect_from_prepared(
            "Write green FRAMEINDEX".to_string(),
            WRITE_GREEN_INDEX.to_string(),
            None,
            &write_green_prepared,
        );
        let empty = resolved_chain_from_canonical(&c1_chain(
            99,
            VideoEffectScope::Output { output_id: 80 },
            false,
            Vec::new(),
        ))
        .unwrap();
        let bypassed = resolved_chain_from_canonical(&c1_chain(
            100,
            VideoEffectScope::Output { output_id: 80 },
            true,
            vec![c1_invalid_effect("bypassed prepare failure")],
        ))
        .unwrap();
        let all_prepare_fail = resolved_chain_from_canonical(&c1_chain(
            101,
            VideoEffectScope::Output { output_id: 80 },
            false,
            vec![
                c1_invalid_effect("prepare failure one"),
                c1_invalid_effect("prepare failure two"),
            ],
        ))
        .unwrap();
        let mixed = resolved_chain_from_canonical(&c1_chain(
            102,
            VideoEffectScope::Output { output_id: 80 },
            false,
            vec![
                c1_invalid_effect("mixed prepare failure"),
                write_red.clone(),
                write_green.clone(),
            ],
        ))
        .unwrap();
        let all_valid = resolved_chain_from_canonical(&c1_chain(
            103,
            VideoEffectScope::Output { output_id: 80 },
            false,
            vec![write_red, write_green],
        ))
        .unwrap();
        let frame = VideoFrame {
            layer_id: 0,
            width: 1,
            height: 1,
            pts_ms: 0,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: vec![9, 19, 29, 255],
        };
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            C1SolidFrameProvider::default(),
        );

        for no_op in [&empty, &bypassed] {
            let (unchanged, faults) =
                renderer.apply_resolved_effect_chain_to_frame(no_op, frame.clone(), None);
            assert_eq!(unchanged, frame);
            assert!(faults.is_empty());
        }

        let (after_prepare_failures, faults) =
            renderer.apply_resolved_effect_chain_to_frame(&all_prepare_fail, frame.clone(), None);
        assert_eq!(after_prepare_failures, frame);
        assert_eq!(faults.len(), 2);

        let (after_mixed, faults) =
            renderer.apply_resolved_effect_chain_to_frame(&mixed, frame.clone(), None);
        assert_eq!(faults.len(), 1);
        assert_eq!(faults[0].stage_index, Some(0));
        assert_eq!(after_mixed.data, vec![1, 1, 29, 255]);

        let (after_mixed_next, faults) =
            renderer.apply_resolved_effect_chain_to_frame(&all_valid, frame, None);
        assert!(faults.is_empty());
        assert_eq!(after_mixed_next.data, vec![2, 2, 29, 255]);
    }
}

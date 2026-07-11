use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

use protocol::{
    ClockSnapshot, CompositionId, CompositionSummary, Transform2D, VideoBackendState,
    VideoBackendStatus, VideoBlendMode, VideoColorAdjust, VideoCuePointSummary, VideoFxAdjust,
    VideoLayerId, VideoLayerState, VideoLayerSummary, VideoMediaMetadata, VideoOutputAspectMode,
    VideoOutputId, VideoOutputKind, VideoOutputMapping, VideoOutputSummary, VideoRuntimeStatus,
    VideoSnapshot, VideoSourceKind, VideoSourceSummary,
};
use serde::{Deserialize, Serialize};

mod gpu_compositor;
mod gpu_surface;
mod hap_decoder;
mod libav_decoder;

pub use gpu_compositor::{GpuCompositeError, GpuCompositor};
pub use gpu_surface::{GpuSurfaceBufferStats, GpuSurfaceError, GpuSurfacePresenter};
pub use hap_decoder::{HapMovFrameDecoder, PreferredVideoFrameDecoder, VideoDecoderDiagnostics};
pub use libav_decoder::LibavFrameDecoder;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoPixelFormat {
    Rgba8,
    Bgra8,
    Dxt1,
    Dxt5,
    YcoCgDxt5,
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

pub struct FrameQueue {
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
    UnsupportedFrameFormat {
        layer_id: VideoLayerId,
        format: VideoPixelFormat,
    },
    FrameSizeMismatch {
        layer_id: VideoLayerId,
    },
    MissingFrame {
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
    pub layer_id: VideoLayerId,
    pub label: String,
    pub position_ms: u64,
    pub priority: VideoDecodePriority,
    pub error: VideoDecodeError,
}

#[derive(Debug, Clone)]
struct ScheduledVideoDecodeJob {
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
        if let Some(index) = self
            .jobs
            .iter()
            .position(|job| video_decode_request_key_matches(&job.request, &request))
        {
            if priority.rank() < self.jobs[index].priority.rank() {
                let sequence = self.next_sequence();
                self.jobs[index].request = request;
                self.jobs[index].priority = priority;
                self.jobs[index].sequence = sequence;
                return VideoDecodeSchedulePush::Reprioritized;
            }
            return VideoDecodeSchedulePush::Duplicate;
        }

        if self.jobs.len() < self.capacity {
            self.insert(request, priority);
            return VideoDecodeSchedulePush::Inserted;
        }

        let Some(worst_index) = self.worst_job_index() else {
            return VideoDecodeSchedulePush::RejectedFull;
        };
        if priority.rank() < self.jobs[worst_index].priority.rank() {
            self.jobs.remove(worst_index);
            self.insert(request, priority);
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
            request: job.request,
            priority: job.priority,
        })
    }

    pub fn pending(&self) -> Vec<ScheduledVideoDecode> {
        let mut jobs = self.jobs.clone();
        jobs.sort_by_key(|job| (job.priority.rank(), job.sequence));
        jobs.into_iter()
            .map(|job| ScheduledVideoDecode {
                request: job.request,
                priority: job.priority,
            })
            .collect()
    }

    fn insert(&mut self, request: VideoFrameRequest, priority: VideoDecodePriority) {
        let sequence = self.next_sequence();
        self.jobs.push(ScheduledVideoDecodeJob {
            request,
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
        let request = scheduled.request;
        let priority = scheduled.priority;
        Some(match self.decoder.decode_frame(&request) {
            Ok(Some(mut frame)) => {
                frame.layer_id = request.layer_id;
                Ok(VideoDecodeWorkerStep::Decoded(runtime.push_frame(frame)))
            }
            Ok(None) => Ok(VideoDecodeWorkerStep::Skipped),
            Err(error) => Err(VideoDecodeWorkerError {
                layer_id: request.layer_id,
                label: request.label,
                position_ms: request.position_ms,
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
    fn frame_for_layer(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError>;
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
    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError>;
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
}

#[derive(Default)]
pub struct StillImageFrameCache {
    entries: Vec<StillImageFrameCacheEntry>,
}

struct StillImageFrameCacheEntry {
    layer_id: VideoLayerId,
    path: PathBuf,
    width: u32,
    height: u32,
    signature: StillImageSignature,
    frame: VideoFrame,
}

#[derive(Debug, Clone)]
struct FfmpegCliFrameCacheEntry {
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
        assert!(
            capacity > 0,
            "frame queue capacity must be greater than zero"
        );
        Self {
            layer_id,
            slots: vec![None; capacity],
            start: 0,
            len: 0,
        }
    }

    pub fn layer_id(&self) -> VideoLayerId {
        self.layer_id
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
                detail: "Pure Rust MOV demux and HAP/HAP Q BC1/BC3 frame decode are built in; HAP Q Alpha and BC7 are staged separately"
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
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=codec_name,width,height,avg_frame_rate,r_frame_rate:format=duration")
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
    let stream = parsed.streams.into_iter().next();
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

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        if request.source.kind != VideoSourceKind::File {
            return Ok(None);
        }

        let path = request
            .source
            .path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| VideoDecodeError::MissingSourcePath {
                layer_id: request.layer_id,
                label: request.label.clone(),
            })?;
        let path = PathBuf::from(path);
        let signature = StillImageSignature::from_path(&path);
        if let Some(entry) = self.entries.iter().find(|entry| {
            entry.layer_id == request.layer_id
                && entry.path == path
                && entry.width == request.width
                && entry.height == request.height
                && entry.position_ms == request.position_ms
                && entry.signature == signature
        }) {
            return Ok(Some(entry.frame.clone()));
        }

        let frame = decode_ffmpeg_cli_frame(request, &self.binary, &path)?;
        self.entries.retain(|entry| {
            !(entry.layer_id == request.layer_id
                && entry.path == path
                && entry.width == request.width
                && entry.height == request.height
                && entry.position_ms == request.position_ms)
        });
        self.entries.push(FfmpegCliFrameCacheEntry {
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

    fn frame_for_layer(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        self.decode_or_placeholder_for_layer(layer, width, height, None)
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
        if matches!(layer.source.kind, VideoSourceKind::StillImage) {
            let path = layer
                .source
                .path
                .as_deref()
                .filter(|path| !path.is_empty())
                .ok_or_else(|| VideoFrameProviderError::MissingStillImagePath {
                    layer_id: layer.id,
                    label: layer.label.clone(),
                })?;
            return self
                .still_images
                .frame_for_layer(layer.id, path, position_ms, width, height)
                .map_err(|error| VideoFrameProviderError::StillImage {
                    layer_id: layer.id,
                    label: layer.label.clone(),
                    error,
                });
        }

        let request = VideoFrameRequest {
            layer_id: layer.id,
            label: layer.label.clone(),
            source: layer.source.clone(),
            position_ms,
            width,
            height,
        };
        match self.decoder.decode_frame(&request) {
            Ok(Some(frame)) => Ok(frame),
            Ok(None) if self.placeholder_when_missing => Ok(debug_solid_frame_for_layer(
                layer.id,
                position_ms,
                width,
                height,
            )),
            Ok(None) => Err(VideoFrameProviderError::Decode {
                layer_id: layer.id,
                label: layer.label.clone(),
                error: VideoDecodeError::UnsupportedSource {
                    layer_id: layer.id,
                    label: layer.label.clone(),
                    kind: layer.source.kind.clone(),
                },
            }),
            Err(error) => Err(VideoFrameProviderError::Decode {
                layer_id: layer.id,
                label: layer.label.clone(),
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
        let layer_ids = snapshot
            .layers
            .iter()
            .map(|layer| layer.id)
            .collect::<Vec<_>>();
        self.frame_provider.retain_layers(&layer_ids);
        self.runtime.sync_layers(&layer_ids);

        for layer in &snapshot.layers {
            if !requested_layer_ids.contains(&layer.id) {
                continue;
            }
            let frames = self
                .frame_provider
                .frames_for_layer(layer, width, height)
                .map_err(VideoPreviewError::from)?;
            for frame in frames {
                self.runtime.push_frame(frame);
            }
        }
        Ok(())
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

    pub fn frame_for_layer(
        &mut self,
        layer_id: VideoLayerId,
        path: impl AsRef<Path>,
        pts_ms: u64,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, StillImageError> {
        let path = path.as_ref().to_path_buf();
        let signature = StillImageSignature::from_path(&path);
        if let Some(entry) = self.entries.iter().find(|entry| {
            entry.layer_id == layer_id
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
            !(entry.layer_id == layer_id
                && entry.path == path
                && entry.width == width
                && entry.height == height)
        });
        self.entries.push(StillImageFrameCacheEntry {
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
            &mut output,
            frame,
            width,
            height,
            layer.opacity.clamp(0.0, 1.0),
            &layer.blend_mode,
            &layer.transform,
            &layer.color,
            &layer.fx,
        );
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
            if output.ready && output.live {
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
        }
    }

    VideoFrame { data, ..frame }
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
        CompositionSummary, Transform2D, VideoBlendMode, VideoLayerSummary, VideoOutputKind,
        VideoOutputSummary, VideoSourceKind, VideoSourceSummary,
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
                        "codec_name": "h264",
                        "width": 1920,
                        "height": 1080,
                        "avg_frame_rate": "30000/1001",
                        "r_frame_rate": "30/1"
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
    }

    #[test]
    fn ffprobe_json_ignores_zero_frame_rate_and_invalid_duration() {
        let summary = parse_ffprobe_metadata_json(
            r#"{
                "streams": [
                    {
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
            }],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    position_ms: 160,
                    ..VideoLayerState::default()
                },
            }],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
            }],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
            }],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    position_ms: 250,
                    ..VideoLayerState::default()
                },
            }],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    position_ms: 100,
                    ..VideoLayerState::default()
                },
            }],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                    }),
                },
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    position_ms: 110,
                    speed: -1.0,
                    loop_enabled: true,
                    loop_start_ms: 100,
                    loop_end_ms: 200,
                    ..VideoLayerState::default()
                },
            }],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                    }),
                },
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
            }],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                }),
            },
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState {
                position_ms: 110,
                speed: -1.0,
                loop_enabled: true,
                loop_start_ms: 100,
                loop_end_ms: 200,
                ..VideoLayerState::default()
            },
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
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState {
                position_ms: 200,
                playing: true,
                ..VideoLayerState::default()
            },
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
                }),
            },
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState {
                position_ms,
                playing: true,
                ..VideoLayerState::default()
            },
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
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
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
                width: 1280,
                height: 720,
                endpoint_name: None,
                opacity: 1.0,
                blackout: output_blackout,
                mapping: VideoOutputMapping::default(),
            }],
            mapping_presets: Vec::new(),
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
        worker
            .scheduler_mut()
            .push(decode_request(1, 999), VideoDecodePriority::Lookahead);

        let report = worker.decode_budget_into_runtime(&mut runtime, 8);

        assert_eq!(report.attempted, 3);
        assert_eq!(report.decoded, 1);
        assert_eq!(report.skipped, 2);
        assert_eq!(report.pending, 0);
        assert_eq!(report.errors.len(), 1);
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
            }],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
        assert_eq!(spout.state, VideoBackendState::NotBuilt);
        if cfg!(target_os = "windows") {
            assert!(spout.detail.contains("not linked"));
        } else {
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
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState::default(),
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
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        enabled: true,
                        ..VideoLayerState::default()
                    },
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
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState::default(),
                },
            ],
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
                    width: 640,
                    height: 360,
                    endpoint_name: Some(" ".to_string()),
                    opacity: 1.0,
                    blackout: true,
                    mapping: VideoOutputMapping::default(),
                },
            ],
            mapping_presets: Vec::new(),
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    enabled: true,
                    ..VideoLayerState::default()
                },
            }],
            compositions: Vec::new(),
            outputs: vec![VideoOutputSummary {
                id: 11,
                label: "Spout Program".to_string(),
                kind: VideoOutputKind::SpoutSender,
                enabled: true,
                composition_id: 1,
                fullscreen: false,
                monitor_id: None,
                width: 1920,
                height: 1080,
                endpoint_name: Some("Syndocal Stage".to_string()),
                opacity: 0.75,
                blackout: false,
                mapping: VideoOutputMapping::default(),
            }],
            mapping_presets: Vec::new(),
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    enabled: true,
                    ..VideoLayerState::default()
                },
            }],
            compositions: Vec::new(),
            outputs: vec![VideoOutputSummary {
                id: 11,
                label: "Spout Program".to_string(),
                kind: VideoOutputKind::SpoutSender,
                enabled: true,
                composition_id: 1,
                fullscreen: false,
                monitor_id: None,
                width: 1920,
                height: 1080,
                endpoint_name: Some("Syndocal Stage".to_string()),
                opacity: 1.0,
                blackout: false,
                mapping: VideoOutputMapping::default(),
            }],
            mapping_presets: Vec::new(),
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
            compositions: Vec::new(),
            outputs: vec![VideoOutputSummary {
                id: 11,
                label: "Spout Program".to_string(),
                kind: VideoOutputKind::SpoutSender,
                enabled: true,
                composition_id: 1,
                fullscreen: false,
                monitor_id: None,
                width: 1920,
                height: 1080,
                endpoint_name: Some("Syndocal Stage".to_string()),
                opacity: 1.0,
                blackout: false,
                mapping: VideoOutputMapping::default(),
            }],
            mapping_presets: Vec::new(),
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
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        opacity: 0.5,
                        position_ms: 100,
                        ..VideoLayerState::default()
                    },
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
                    blend_mode: VideoBlendMode::Screen,
                    state: VideoLayerState {
                        opacity: 1.0,
                        position_ms: 200,
                        ..VideoLayerState::default()
                    },
                },
            ],
            compositions: vec![CompositionSummary {
                id: 5,
                label: "Output A".to_string(),
                layer_ids: vec![2, 1],
                output_ids: vec![7],
            }],
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
            }],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        enabled: false,
                        opacity: 0.8,
                        ..VideoLayerState::default()
                    },
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
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        enabled: true,
                        opacity: 0.6,
                        ..VideoLayerState::default()
                    },
                },
            ],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        opacity: 1.0,
                        ..VideoLayerState::default()
                    },
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
                    blend_mode: VideoBlendMode::Add,
                    state: VideoLayerState {
                        solo: true,
                        opacity: 0.75,
                        ..VideoLayerState::default()
                    },
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
                    blend_mode: VideoBlendMode::Screen,
                    state: VideoLayerState {
                        enabled: false,
                        solo: true,
                        opacity: 1.0,
                        ..VideoLayerState::default()
                    },
                },
            ],
            compositions: Vec::new(),
            outputs: Vec::new(),
            mapping_presets: Vec::new(),
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState {
                    opacity: 0.8,
                    ..VideoLayerState::default()
                },
            }],
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
                width: 1920,
                height: 1080,
                endpoint_name: None,
                opacity: 0.5,
                blackout: false,
                mapping: Default::default(),
            }],
            mapping_presets: Vec::new(),
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
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
        };
        let snapshot = VideoSnapshot {
            layers: vec![layer(1, "Back"), layer(2, "Middle"), layer(3, "Front")],
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
            }],
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
                width: 2,
                height: 1,
                endpoint_name: None,
                opacity: 0.5,
                blackout: false,
                mapping: Default::default(),
            }],
            mapping_presets: Vec::new(),
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
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
        };
        let mut snapshot = VideoSnapshot {
            layers: vec![layer(1), layer(2)],
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
                width: 2,
                height: 1,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: Default::default(),
            }],
            mapping_presets: Vec::new(),
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
        assert_eq!(
            renderer.frame_provider().retained,
            vec![vec![1, 2], vec![1, 2]]
        );
        assert_eq!(renderer.frame_provider().requested, vec![2, 2]);

        snapshot.outputs[0].blackout = true;
        let prepared = renderer.prepare_output_frames(&snapshot, 9, 2, 1).unwrap();
        assert!(prepared.plan.output_blackout);
        assert!(prepared.frames.is_empty());
        let frame = renderer.render_output(&snapshot, 9).unwrap();
        assert_eq!(frame.data, vec![0, 0, 0, 255, 0, 0, 0, 255]);
        assert_eq!(
            renderer.frame_provider().retained,
            vec![vec![1, 2], vec![1, 2]]
        );
        assert_eq!(renderer.frame_provider().requested, vec![2, 2]);

        snapshot.outputs[0].blackout = false;
        snapshot.outputs[0].enabled = false;
        let frame = renderer.render_output(&snapshot, 9).unwrap();
        assert_eq!(frame.data, vec![0, 0, 0, 255, 0, 0, 0, 255]);
        assert_eq!(
            renderer.frame_provider().retained,
            vec![vec![1, 2], vec![1, 2]]
        );
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
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
            }],
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
    fn output_test_pattern_is_routed_through_projector_mapping() {
        let mut snapshot = VideoSnapshot {
            layers: Vec::new(),
            compositions: Vec::new(),
            outputs: vec![VideoOutputSummary {
                id: 9,
                label: "Projector".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 1,
                fullscreen: true,
                monitor_id: None,
                width: 8,
                height: 8,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: Default::default(),
            }],
            mapping_presets: Vec::new(),
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
}

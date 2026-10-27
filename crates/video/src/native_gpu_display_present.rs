//! Public, affine presentation facade. Only real P `Presented` receipts count.
use super::*;
use crate::gpu_surface::native::{
    CpuArtisticPresentation, HardBlackoutPresentation, NativePresentContent, NativePresentGate,
    NativePresentReceipt,
};
use crate::native_gpu_contract::{NativePresentSink, PresentOutcome, PresentRejection};
use crate::native_gpu_output::{
    present_native_output_with_retirement, NativePresentPermit, NativePresentation, OutputBoundary,
    ReadyNativeGpuOutput,
};
use crate::native_gpu_result::NativeCpuFrameOwner;
use crate::native_gpu_snapshot::NativeSnapshotGpu;
use crate::{
    VideoFrame, VideoOutputMapping, VideoOutputRenderEvidence, VideoOutputRenderFreshness,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeResidentPresentBoundary {
    BeforePrepare,
    BeforeAcquire,
    BeforePhysicalPresent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeResidentPresentDecision {
    Permit,
    Cancelled,
    Revoked,
}

pub trait NativeResidentPresentationAuthority {
    fn revalidate(&self, boundary: NativeResidentPresentBoundary) -> NativeResidentPresentDecision;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeResidentPresentResult {
    Presented { hard_blackout: bool },
    NotPresented { reason: String },
}

pub(super) struct ResidentAuthority<'a, A>(pub(super) &'a A);
impl<A: NativeResidentPresentationAuthority> NativeOutputAuthority for ResidentAuthority<'_, A> {
    fn revalidate(
        &self,
        _request: &NativeOutputRequest,
        boundary: OutputBoundary,
    ) -> Result<(), PresentRejection> {
        let boundary = match boundary {
            OutputBoundary::BeforeAcquire => NativeResidentPresentBoundary::BeforeAcquire,
            OutputBoundary::BeforePhysicalPresent => {
                NativeResidentPresentBoundary::BeforePhysicalPresent
            }
            _ => NativeResidentPresentBoundary::BeforePrepare,
        };
        match self.0.revalidate(boundary) {
            NativeResidentPresentDecision::Permit => Ok(()),
            NativeResidentPresentDecision::Cancelled => Err(PresentRejection::Cancelled),
            NativeResidentPresentDecision::Revoked => Err(PresentRejection::Revoked),
        }
    }
}

pub(super) struct NativeCpuArtisticDisplay {
    pub(super) request: NativeOutputRequest,
    pub(super) owner: Arc<NativeCpuFrameOwner>,
}
impl CpuArtisticPresentation for NativeCpuArtisticDisplay {
    fn frame_id(&self) -> NativeFrameId {
        self.request.frame
    }
    fn frame(&self) -> &VideoFrame {
        &self.owner.frame
    }
    fn mapping(&self) -> &VideoOutputMapping {
        &self.request.mapping
    }
}

pub(super) struct NativeHardBlackoutDisplay {
    pub(super) request: NativeOutputRequest,
}
impl HardBlackoutPresentation for NativeHardBlackoutDisplay {
    fn frame_id(&self) -> NativeFrameId {
        self.request.frame
    }
}

impl<A: NativeOutputAuthority> NativePresentGate<NativeCpuArtisticDisplay>
    for NativePresentPermit<'_, A>
{
    fn before_acquire(&self, content: &NativeCpuArtisticDisplay) -> Result<(), PresentRejection> {
        self.check_request(&content.request, OutputBoundary::BeforeAcquire)
    }
    fn before_present(&self, content: &NativeCpuArtisticDisplay) -> Result<(), PresentRejection> {
        self.check_request(&content.request, OutputBoundary::BeforePhysicalPresent)
    }
}
impl<A: NativeOutputAuthority> NativePresentGate<NativeHardBlackoutDisplay>
    for NativePresentPermit<'_, A>
{
    fn before_acquire(&self, content: &NativeHardBlackoutDisplay) -> Result<(), PresentRejection> {
        self.check_request(&content.request, OutputBoundary::BeforeAcquire)
    }
    fn before_present(&self, content: &NativeHardBlackoutDisplay) -> Result<(), PresentRejection> {
        self.check_request(&content.request, OutputBoundary::BeforePhysicalPresent)
    }
}

pub(super) enum ResidentDisplayContent {
    Resident(ReadyNativeGpuOutput<Arc<NativeSnapshotGpu>>),
    Cpu(NativeCpuArtisticDisplay),
    HardBlackout(NativeHardBlackoutDisplay),
    Rejected,
}

fn validate_receipt(
    receipt: &NativePresentReceipt,
    request: &NativeOutputRequest,
    content: NativePresentContent,
    sequence: u64,
) -> Result<(), String> {
    let snapshot = match content {
        NativePresentContent::Resident(snapshot) => Some(snapshot),
        NativePresentContent::CpuArtistic | NativePresentContent::HardBlackout => None,
    };
    if receipt.frame() != request.frame
        || receipt.content() != content
        || receipt.snapshot() != snapshot
        || receipt.extent() != (request.key.width, request.key.height)
        || receipt.submission().context != request.context
        || receipt.submission().sequence == 0
        || snapshot.is_some_and(|snapshot| {
            receipt.submission().sequence <= snapshot.completed_submission.sequence
        })
        || receipt.sequence() != sequence
    {
        return Err("native physical present receipt does not match its frozen request".into());
    }
    Ok(())
}

/// Neither Clone nor Copy: surface retry consumes the same frozen owner once.
pub struct NativeResidentDisplayFrame {
    pub(super) content: ResidentDisplayContent,
    pub(super) evidence: VideoOutputRenderEvidence,
    pub(super) extent: (u32, u32),
    pub(super) pts_ms: u64,
    pub(super) output_mapping: VideoOutputMapping,
}
impl NativeResidentDisplayFrame {
    pub fn evidence(&self) -> &VideoOutputRenderEvidence {
        &self.evidence
    }
    pub fn extent(&self) -> (u32, u32) {
        self.extent
    }
    pub fn pts_ms(&self) -> u64 {
        self.pts_ms
    }
    pub fn output_mapping(&self) -> &VideoOutputMapping {
        &self.output_mapping
    }
}

impl NativeResidentDisplaySession {
    pub fn present<A: NativeResidentPresentationAuthority>(
        &mut self,
        frame: NativeResidentDisplayFrame,
        authority: &A,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<NativeResidentPresentResult, String> {
        self.ensure_usable()?;
        if frame.evidence.freshness != VideoOutputRenderFreshness::Fresh
            || frame.evidence.error.is_some()
        {
            return Ok(NativeResidentPresentResult::NotPresented {
                reason: format!("non-fresh artistic output: {:?}", frame.evidence),
            });
        }
        let request = match &frame.content {
            ResidentDisplayContent::Resident(ready) => &ready.request,
            ResidentDisplayContent::Cpu(cpu) => &cpu.request,
            ResidentDisplayContent::HardBlackout(blackout) => &blackout.request,
            ResidentDisplayContent::Rejected => {
                return Ok(NativeResidentPresentResult::NotPresented {
                    reason: "rejected artistic output".into(),
                })
            }
        }
        .clone();
        if request.context != self.owner.context().id()
            || request.frame.output_id != self.output_id
            || request.frame.geometry_generation != self.geometry_generation
            || request.frame.project_epoch != frame.evidence.project_render_epoch
            || request.frame.output_id != frame.evidence.output_id
            || (request.key.width, request.key.height) != frame.extent
            || self.reserved_geometry.is_some()
        {
            return Err(
                "native present requires its own configured session and frozen frame identity"
                    .into(),
            );
        }
        let authority = ResidentAuthority(authority);
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| "shared native presenter unavailable".to_string())?;
        if presenter.configured_extent() != Some(frame.extent) {
            return Err("native present extent is not the configured physical extent".into());
        }
        let sequence = presenter
            .presents()
            .checked_add(1)
            .ok_or_else(|| "native present sequence exhausted".to_string())?;
        #[cfg(feature = "qa-native-frame-timing")]
        let _present_trace =
            crate::native_frame_timing::span(crate::native_frame_timing::Phase::PresenterPresent);
        match frame.content {
            ResidentDisplayContent::Resident(mut ready) => {
                let expected = NativePresentContent::Resident(ready.snapshot_identity());
                let delivered = self
                    .delivery
                    .delivered()
                    .checked_add(1)
                    .ok_or_else(|| "native delivery sequence exhausted".to_string())?;
                loop {
                    let result = {
                        let mut sink = presenter
                            .sink(&mut self.owner)
                            .map_err(|error| format!("resident sink rejected: {error:?}"))?;
                        present_native_output_with_retirement(
                            ready,
                            |ready| ready,
                            &mut sink,
                            &authority,
                            cancellation,
                            &mut self.delivery,
                        )
                    };
                    match result {
                        NativePresentation::Presented(delivery) => {
                            validate_receipt(delivery.receipt(), &request, expected, sequence)?;
                            if delivery.frame() != request.frame
                                || self.delivery.delivered() != delivered
                            {
                                return Err(
                                    "native delivery does not match its physical present receipt"
                                        .into(),
                                );
                            }
                            return Ok(NativeResidentPresentResult::Presented {
                                hard_blackout: false,
                            });
                        }
                        NativePresentation::Retry(returned) => ready = returned,
                        NativePresentation::Retiring(retiring) => {
                            let reason = format!("{:?}", retiring.reason());
                            self.retired_ready.push(retiring);
                            return Ok(NativeResidentPresentResult::NotPresented { reason });
                        }
                        NativePresentation::NotPresented(reason) => {
                            return Ok(NativeResidentPresentResult::NotPresented {
                                reason: format!("{reason:?}"),
                            })
                        }
                        NativePresentation::Failed(error) => {
                            return Err(format!("resident present failed: {error:?}"))
                        }
                    }
                }
            }
            ResidentDisplayContent::Cpu(mut content) => {
                for retry in 0..=1 {
                    let outcome = {
                        let mut sink = presenter
                            .cpu_artistic_sink(&mut self.owner)
                            .map_err(|error| format!("CPU artistic sink rejected: {error:?}"))?;
                        sink.present_once(
                            content,
                            NativePresentPermit::new(&authority, cancellation),
                        )
                    }
                    .map_err(|error| format!("CPU artistic present failed: {error:?}"))?;
                    match outcome {
                        PresentOutcome::Presented(receipt) => {
                            validate_receipt(
                                &receipt,
                                &request,
                                NativePresentContent::CpuArtistic,
                                sequence,
                            )?;
                            return Ok(NativeResidentPresentResult::Presented {
                                hard_blackout: false,
                            });
                        }
                        PresentOutcome::NotPresented {
                            reason: PresentRejection::SurfaceLostOrOutdated,
                            retry: Some(returned),
                        } if retry == 0 => content = returned,
                        PresentOutcome::NotPresented { reason, .. } => {
                            return Ok(NativeResidentPresentResult::NotPresented {
                                reason: format!("{reason:?}"),
                            })
                        }
                    }
                }
                unreachable!("bounded surface retry terminates")
            }
            ResidentDisplayContent::HardBlackout(mut content) => {
                for retry in 0..=1 {
                    let outcome = {
                        let mut sink = presenter
                            .hard_blackout_sink(&mut self.owner)
                            .map_err(|error| format!("blackout sink rejected: {error:?}"))?;
                        sink.present_once(
                            content,
                            NativePresentPermit::new(&authority, cancellation),
                        )
                    }
                    .map_err(|error| format!("hard blackout present failed: {error:?}"))?;
                    match outcome {
                        PresentOutcome::Presented(receipt) => {
                            validate_receipt(
                                &receipt,
                                &request,
                                NativePresentContent::HardBlackout,
                                sequence,
                            )?;
                            return Ok(NativeResidentPresentResult::Presented {
                                hard_blackout: true,
                            });
                        }
                        PresentOutcome::NotPresented {
                            reason: PresentRejection::SurfaceLostOrOutdated,
                            retry: Some(returned),
                        } if retry == 0 => content = returned,
                        PresentOutcome::NotPresented { reason, .. } => {
                            return Ok(NativeResidentPresentResult::NotPresented {
                                reason: format!("{reason:?}"),
                            })
                        }
                    }
                }
                unreachable!("bounded surface retry terminates")
            }
            ResidentDisplayContent::Rejected => Ok(NativeResidentPresentResult::NotPresented {
                reason: "rejected artistic output".into(),
            }),
        }
    }

    pub fn presented_frames(&self) -> u64 {
        self.presenter
            .as_ref()
            .map_or(0, SharedNativePresenter::presents)
    }
    pub fn hard_blackout_frames_presented(&self) -> u64 {
        self.presenter
            .as_ref()
            .map_or(0, SharedNativePresenter::hard_blackout_presents)
    }
}

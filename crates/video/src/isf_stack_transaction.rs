//! Synchronous ownership of encoded ISF work through mapping and scope cleanup.
//! Independent stacks share a submission/wait, never their upload or scratch.
use std::{
    borrow::Cow,
    sync::{mpsc, Arc},
    time::Instant,
};

use super::*;
use crate::isf_runtime::StackBatch;

pub(in crate::isf_runtime) struct IndependentStack<'a> {
    pub frame: Arc<VideoFrame>,
    pub stages: &'a [IsfStageInputs<'a>],
    pub keys: Cow<'a, [IsfInstanceKey]>,
    pub frame_index: u32,
    pub time_seconds: f32,
    pub time_delta_seconds: f32,
    pub date: [f32; 4],
}

#[derive(Clone, Copy)]
pub(super) struct StackClocks {
    pub time_seconds: f32,
    pub time_delta_seconds: f32,
    pub frame_index: u32,
    pub date: [f32; 4],
}

#[derive(Clone, Copy)]
struct FrameIdentity {
    layer_id: u64,
    width: u32,
    height: u32,
    pts_ms: u64,
    duration_ms: u64,
}

struct MapCompletion {
    result: Result<(), String>,
}

pub(super) struct PendingStack<'a> {
    // Batch input ownership survives every error, including a poisoned drain.
    // A synchronous single call borrows its caller's input through the upload.
    _input: Option<Arc<VideoFrame>>,
    identity: FrameIdentity,
    pub keys: Cow<'a, [IsfInstanceKey]>,
    pub clocks: StackClocks,
    pub workspace: Option<StackWorkspace>,
    pub encoder: Option<wgpu::CommandEncoder>,
    pub states: Vec<(IsfInstanceKey, StageResources)>,
    pub images: Vec<GpuImage>,
    #[cfg(test)]
    pub native_pipeline_pins: Vec<Arc<IsfGpuPipeline>>,
    #[cfg(test)]
    pub native_uniforms: Vec<wgpu::Buffer>,
    receiver: Option<mpsc::Receiver<MapCompletion>>,
    mapping_started: bool,
}

impl<'a> PendingStack<'a> {
    pub(super) fn new(
        frame: &VideoFrame,
        input: Option<Arc<VideoFrame>>,
        keys: Cow<'a, [IsfInstanceKey]>,
        clocks: StackClocks,
        workspace: Option<StackWorkspace>,
    ) -> Self {
        Self {
            _input: input,
            identity: FrameIdentity {
                layer_id: frame.layer_id,
                width: frame.width,
                height: frame.height,
                pts_ms: frame.pts_ms,
                duration_ms: frame.duration_ms,
            },
            keys,
            clocks,
            workspace,
            encoder: None,
            states: Vec::new(),
            images: Vec::new(),
            #[cfg(test)]
            native_pipeline_pins: Vec::new(),
            #[cfg(test)]
            native_uniforms: Vec::new(),
            receiver: None,
            mapping_started: false,
        }
    }

    fn begin_mapping(&mut self) {
        let (sender, receiver) = mpsc::sync_channel(1);
        self.workspace
            .as_ref()
            .unwrap()
            .readback
            .as_ref()
            .expect("CPU stack has a readback buffer")
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(MapCompletion {
                    result: result.map_err(|error| error.to_string()),
                });
            });
        self.receiver = Some(receiver);
        self.mapping_started = true;
    }

    fn receive_mapping(&mut self) -> Result<(), IsfRuntimeError> {
        let completion = self
            .receiver
            .take()
            .unwrap()
            .recv()
            .map_err(|error| runtime_error(format!("ISF readback channel failed: {error}")))?;
        completion
            .result
            .map_err(|error| runtime_error(format!("ISF readback failed: {error}")))
    }

    fn unmap(&mut self) {
        if self.mapping_started {
            #[cfg(feature = "qa-native-frame-timing")]
            let _qa_unmap =
                crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfUnmap);
            self.workspace
                .as_ref()
                .unwrap()
                .readback
                .as_ref()
                .unwrap()
                .unmap();
            self.mapping_started = false;
        }
    }

    fn copy_and_unmap(&mut self) -> VideoFrame {
        let identity = self.identity;
        #[cfg(feature = "qa-native-frame-timing")]
        let qa_copy =
            crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfMappedCopy);
        let readback = self.workspace.as_ref().unwrap().readback.as_ref().unwrap();
        let mapped = readback.slice(..).get_mapped_range();
        let data = copy_rgba_readback(
            &mapped,
            (identity.width * 4) as usize,
            StackWorkspace::bytes_per_row(identity.width) as usize,
            identity.width as usize * identity.height as usize * 4,
        );
        drop(mapped);
        #[cfg(feature = "qa-native-frame-timing")]
        drop(qa_copy);
        self.unmap();
        VideoFrame {
            layer_id: identity.layer_id,
            width: identity.width,
            height: identity.height,
            pts_ms: identity.pts_ms,
            duration_ms: identity.duration_ms,
            format: VideoPixelFormat::Rgba8,
            data,
        }
    }
}

/// A failed cleanup wait cannot prove that resources have been reaped.
/// Keep the owner until runtime teardown and forbid all later GPU work,
/// including the caller's ordinary stage-local recovery attempts.
pub(super) struct PoisonedStackTransaction {
    _pending: Vec<RetainedStackOwnership>,
    reason: String,
}

// A completion receiver belongs only to the synchronous call. Keeping it in
// the runtime would unnecessarily remove IsfGpuRuntime's Sync auto-trait.
// A failed drain retains every CPU/GPU owner, but callbacks may safely observe
// a disconnected receiver after the outstanding map has been cancelled.
struct RetainedStackOwnership {
    _input: Option<Arc<VideoFrame>>,
    _identity: FrameIdentity,
    _keys: Vec<IsfInstanceKey>,
    _clocks: StackClocks,
    _workspace: Option<StackWorkspace>,
    _encoder: Option<wgpu::CommandEncoder>,
    _states: Vec<(IsfInstanceKey, StageResources)>,
    _images: Vec<GpuImage>,
}

impl PoisonedStackTransaction {
    fn new(pending: &mut [PendingStack<'_>], reason: String) -> Self {
        Self {
            _pending: pending
                .iter_mut()
                .map(|stack| RetainedStackOwnership {
                    _input: stack._input.take(),
                    _identity: stack.identity,
                    _keys: std::mem::take(&mut stack.keys).into_owned(),
                    _clocks: stack.clocks,
                    _workspace: stack.workspace.take(),
                    _encoder: stack.encoder.take(),
                    _states: std::mem::take(&mut stack.states),
                    _images: std::mem::take(&mut stack.images),
                })
                .collect(),
            reason,
        }
    }
}

impl IsfGpuRuntime {
    #[cfg(test)]
    pub(crate) fn set_stack_phase_timing_enabled(&mut self, enabled: bool) {
        self.resources.phase_timing_suppressed = !enabled;
        if !enabled {
            self.resources.phase_timing = None;
        }
    }

    #[cfg(test)]
    pub(crate) fn independent_batch_count(&self) -> u64 {
        self.resources.independent_batch_count
    }

    pub(in crate::isf_runtime) fn render_pass_stack(
        &mut self,
        frame: &VideoFrame,
        stages: &[IsfStageInputs<'_>],
        time_seconds: f32,
        time_delta_seconds: f32,
        frame_index: u32,
        date: [f32; 4],
        keys: &[IsfInstanceKey],
    ) -> Result<VideoFrame, IsfRuntimeError> {
        #[cfg(feature = "qa-native-frame-timing")]
        let _qa_isf = crate::native_frame_timing::span(crate::native_frame_timing::Phase::Isf);
        #[cfg(feature = "qa-native-frame-timing")]
        crate::native_frame_timing::set_isf_metadata_for_layer(
            crate::native_frame_timing::isf_single_path(u64::from(frame_index), frame.layer_id),
            frame.layer_id,
            stages.len() as u32,
            u64::from(frame_index),
        );
        self.last_stack_stage_count = stages.len();
        self.last_stack_render_us = 0;
        if stages.is_empty() {
            return Ok(frame.clone());
        }
        self.reject_poisoned_stack_transaction()?;
        #[cfg(test)]
        {
            self.resources.phase_timing = if self.resources.phase_timing_suppressed {
                None
            } else {
                self.resources.phase_sequence = self.resources.phase_sequence.wrapping_add(1);
                Some(phase_timing::StackPhaseTiming::new(
                    self.resources.phase_sequence,
                ))
            };
        }
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        #[cfg(test)]
        if let Some(timing) = self.resources.phase_timing.as_mut() {
            timing.mark(0);
        }
        // Extra scratch must never evict feedback on the ordinary single path.
        self.resources.batch_workspaces.clear();
        let workspace = self.resources.workspace.take();
        let mut pending = [PendingStack::new(
            frame,
            None,
            Cow::Borrowed(keys),
            StackClocks {
                time_seconds,
                time_delta_seconds,
                frame_index,
                date,
            },
            workspace,
        )];
        let started = Instant::now();
        let result = self
            .encode_pass_stack(
                &mut pending[0],
                frame,
                stages,
                None,
                #[cfg(test)]
                None,
            )
            .and_then(|()| self.submit_and_complete_stacks(&mut pending));
        let result = self.finish_stack_transaction(&mut pending, result);
        #[cfg(feature = "qa-native-frame-timing")]
        if result.is_err() {
            crate::native_frame_timing::mark_isf_failed(u64::from(frame_index), frame.layer_id);
        }
        self.record_stack_diagnostic(started, stages.len());
        result.map(|frames| match frames {
            StackBatch::One([frame]) => frame,
            _ => unreachable!("single-stack completion"),
        })
    }

    /// None declines before uploads, encoding, resource eviction or clock edits.
    /// Ordinary Err has drained all submitted/pending uploads. If the drain
    /// itself fails, retained poison blocks recovery from issuing any GPU work.
    pub(in crate::isf_runtime) fn try_render_independent_stacks(
        &mut self,
        requests: &[IndependentStack<'_>],
    ) -> Option<Result<StackBatch<VideoFrame>, IsfRuntimeError>> {
        #[cfg(feature = "qa-native-frame-timing")]
        let _qa_isf = crate::native_frame_timing::span(crate::native_frame_timing::Phase::Isf);
        if let Err(error) = self.reject_poisoned_stack_transaction() {
            return Some(Err(error));
        }
        let reservation = self.independent_stack_reservation(requests);
        #[cfg(feature = "qa-native-frame-timing")]
        for request in requests {
            crate::native_frame_timing::set_isf_metadata_for_layer(
                if reservation.is_none() {
                    crate::native_frame_timing::IsfPath::Decline
                } else if requests.len() == 2 {
                    crate::native_frame_timing::IsfPath::Batch2
                } else {
                    crate::native_frame_timing::IsfPath::Batch3
                },
                request.frame.layer_id,
                request.stages.len() as u32,
                u64::from(request.frame_index),
            );
        }
        let reservation = reservation?;
        #[cfg(test)]
        {
            self.resources.phase_timing = None;
        }
        self.last_stack_stage_count = requests.iter().map(|request| request.stages.len()).sum();
        self.last_stack_render_us = 0;
        let mut available = std::mem::take(&mut self.resources.batch_workspaces);
        if let Some(workspace) = self.resources.workspace.take() {
            available.push(workspace);
        }
        let mut pending = StackBatch::from_fn(requests.len(), |index| {
            let request = &requests[index];
            let workspace = available
                .iter()
                .position(|workspace| workspace.matches(request.frame.width, request.frame.height))
                .map(|index| available.swap_remove(index));
            PendingStack::new(
                &request.frame,
                Some(Arc::clone(&request.frame)),
                Cow::Borrowed(&request.keys),
                StackClocks {
                    time_seconds: request.time_seconds,
                    time_delta_seconds: request.time_delta_seconds,
                    frame_index: request.frame_index,
                    date: request.date,
                },
                workspace,
            )
        });
        // Drop mismatched/unused scratch before allocating replacements, so the
        // live peak is the admitted reservation, not old plus new dimensions.
        available.clear();
        self.resources.batch_workspaces = available;
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let started = Instant::now();
        let result = requests
            .iter()
            .zip(pending.iter_mut())
            .try_for_each(|(request, pending)| {
                self.encode_pass_stack(
                    pending,
                    &request.frame,
                    request.stages,
                    Some(reservation),
                    #[cfg(test)]
                    None,
                )
            })
            .and_then(|()| self.submit_and_complete_stacks(&mut pending));
        let result = self.finish_stack_transaction(&mut pending, result);
        #[cfg(feature = "qa-native-frame-timing")]
        if result.is_err() {
            for request in requests {
                crate::native_frame_timing::mark_isf_failed(
                    u64::from(request.frame_index),
                    request.frame.layer_id,
                );
            }
        }
        self.record_stack_diagnostic(
            started,
            requests.iter().map(|request| request.stages.len()).sum(),
        );
        Some(result)
    }

    fn independent_stack_reservation(&self, requests: &[IndependentStack<'_>]) -> Option<u64> {
        let upload_pitch = self.resources.queue_upload_pitch?;
        if !(2..=3).contains(&requests.len())
            || self.resources.entries.len() > RESOURCE_CACHE_CAPACITY
        {
            return None;
        }
        let mut reservation = 0_u64;
        for (request_index, request) in requests.iter().enumerate() {
            let frame = &request.frame;
            if request.stages.is_empty()
                || request.keys.len() != request.stages.len()
                || !request
                    .stages
                    .iter()
                    .all(IsfStageInputs::can_share_host_buffers)
                || !super::super::date::valid_isf_date(request.date)
                || frame.format != VideoPixelFormat::Rgba8
                || self.validate_pass_size(frame.width, frame.height).is_err()
                || frame.data.len() as u64 != u64::from(frame.width) * u64::from(frame.height) * 4
                || request.keys.iter().enumerate().any(|(index, key)| {
                    request.keys[..index].contains(key)
                        || requests[..request_index]
                            .iter()
                            .any(|earlier| earlier.keys.contains(key))
                })
            {
                return None;
            }
            // The 512 MiB gate includes all in-flight GPU scratch and cached
            // resources, including padded queue-write upload staging. Include
            // retained input Arc bytes AND
            // the completed CPU output bytes while the other maps finish.
            reservation = reservation
                .checked_add(StackWorkspace::required_bytes(frame.width, frame.height))?
                .checked_add((frame.data.len() as u64).checked_mul(2)?)?
                .checked_add(StackWorkspace::upload_staging_bytes(
                    frame.width,
                    frame.height,
                    upload_pitch,
                )?)?;
        }
        let cached = self
            .resources
            .entries
            .values()
            .try_fold(0_u64, |bytes, state| bytes.checked_add(state.bytes))?;
        (cached.checked_add(reservation)? <= MAX_CACHED_RESOURCE_BYTES).then_some(reservation)
    }

    // One diagnostic unit: encode start through CPU copies and scope cleanup.
    // A single/recovered chain counts its stages; an admitted batch counts all
    // stages covered by the shared encode/completion interval.
    fn record_stack_diagnostic(&mut self, started: Instant, stage_count: usize) {
        self.last_stack_stage_count = stage_count;
        self.last_stack_render_us = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    }

    fn reject_poisoned_stack_transaction(&self) -> Result<(), IsfRuntimeError> {
        match &self.resources.poisoned_transaction {
            Some(poisoned) => Err(runtime_error(format!(
                "ISF GPU owner retained after failed cleanup; further GPU work is disabled: {}",
                poisoned.reason,
            ))),
            None => Ok(()),
        }
    }

    fn submit_and_complete_stacks(
        &mut self,
        pending: &mut [PendingStack],
    ) -> Result<StackBatch<VideoFrame>, IsfRuntimeError> {
        #[cfg(feature = "qa-native-frame-timing")]
        let qa_submit =
            crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfSubmit);
        let commands = pending
            .iter_mut()
            .map(|stack| stack.encoder.take().unwrap().finish());
        self.queue.submit(commands);
        #[cfg(feature = "qa-native-frame-timing")]
        drop(qa_submit);
        #[cfg(test)]
        {
            self.resources.stack_submit_count += 1;
        }
        #[cfg(test)]
        {
            if pending.len() > 1 {
                self.resources.independent_batch_count =
                    self.resources.independent_batch_count.saturating_add(1);
            }
            if let Some(timing) = self.resources.phase_timing.as_mut() {
                timing.mark(4);
            }
        }
        #[cfg(feature = "qa-native-frame-timing")]
        let qa_map_wait =
            crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfMapWait);
        for stack in pending.iter_mut() {
            stack.begin_mapping();
        }
        self.device
            .poll(wgpu::PollType::wait())
            .map_err(|error| runtime_error(format!("ISF GPU wait failed: {error}")))?;
        let mut failed = None;
        // Receive every completion before exposing any frame or returning an
        // error. A successful earlier map never releases a later ticket owner.
        for stack in pending.iter_mut() {
            if let Err(error) = stack.receive_mapping() {
                failed.get_or_insert(error);
            }
        }
        #[cfg(feature = "qa-native-frame-timing")]
        drop(qa_map_wait);
        if let Some(error) = failed {
            return Err(error);
        }
        #[cfg(test)]
        if let Some(timing) = self.resources.phase_timing.as_mut() {
            timing.mark(5);
        }
        let frames = StackBatch::from_fn(pending.len(), |index| pending[index].copy_and_unmap());
        #[cfg(test)]
        if let Some(timing) = self.resources.phase_timing.as_mut() {
            timing.mark(6);
        }
        #[cfg(feature = "qa-native-frame-timing")]
        let _qa_cleanup =
            crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfCleanup);
        // Preserve the canonical single-stack feedback commit. Validation
        // scope failure below invalidates the same ownership keys as before.
        self.commit_stage_states(pending.iter_mut().flat_map(|stack| stack.states.drain(..)));
        #[cfg(test)]
        if let Some(timing) = self.resources.phase_timing.as_mut() {
            timing.mark(7);
        }
        Ok(frames)
    }

    fn finish_stack_transaction(
        &mut self,
        pending: &mut [PendingStack<'_>],
        result: Result<StackBatch<VideoFrame>, IsfRuntimeError>,
    ) -> Result<StackBatch<VideoFrame>, IsfRuntimeError> {
        #[cfg(feature = "qa-native-frame-timing")]
        let _qa_cleanup =
            crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfCleanup);
        let cleanup_failure = if result.is_err() {
            // Flush writes even when encoding stopped before a command buffer
            // was submitted. Keep every ticket and its resources alive here.
            {
                #[cfg(feature = "qa-native-frame-timing")]
                let _qa_submit =
                    crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfSubmit);
                self.queue.submit(std::iter::empty::<wgpu::CommandBuffer>());
            }
            #[cfg(test)]
            {
                self.resources.stack_submit_count += 1;
            }
            #[cfg(feature = "qa-native-frame-timing")]
            let _qa_map_wait =
                crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfMapWait);
            self.device
                .poll(wgpu::PollType::wait())
                .err()
                .map(|error| error.to_string())
        } else {
            None
        };
        for stack in pending.iter_mut() {
            stack.unmap();
            // Successful wait has already dispatched all map callbacks. On a
            // failed wait, unmap cancels mapping; retain owners, not receivers.
            if cleanup_failure.is_none() {
                if let Some(receiver) = stack.receiver.take() {
                    let _ = receiver.recv();
                }
            }
        }
        let validation = pollster::block_on(self.device.pop_error_scope());
        let allocation = pollster::block_on(self.device.pop_error_scope());
        #[cfg(test)]
        if let Some(timing) = self.resources.phase_timing.as_mut() {
            timing.mark(8);
        }
        if validation.is_some() || allocation.is_some() || cleanup_failure.is_some() {
            for stack in pending.iter() {
                self.invalidate_stage_states(&stack.keys);
            }
        }
        if let Some(reason) = cleanup_failure {
            self.resources.poisoned_transaction =
                Some(PoisonedStackTransaction::new(pending, reason.clone()));
            return Err(runtime_error(format!(
                "ISF GPU upload cleanup failed; owner retained and GPU work disabled: {reason}"
            )));
        }
        if let Some(error) = validation.or(allocation) {
            return Err(runtime_error(format!("ISF GPU pass failed: {error}")));
        }
        match result {
            Ok(frames) => {
                let mut workspaces = pending
                    .iter_mut()
                    .map(|stack| stack.workspace.take().unwrap());
                self.resources.workspace = workspaces.next();
                self.resources.batch_workspaces.clear();
                self.resources.batch_workspaces.extend(workspaces);
                Ok(frames)
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
#[path = "isf_stack_transaction_tests.rs"]
mod tests;

#[cfg(all(test, feature = "qa-native-frame-timing"))]
mod native_timing_probe {
    use super::*;
    use crate::isf_runtime::IndependentChain;
    use crate::native_frame_timing::{self as timing, IsfPath, Outcome, Phase};

    /// Real admitted 3-job ? 3-stage work through the canonical batch encoder.
    /// Run serialized with the other capture tests; this probe asserts no time budget.
    #[test]
    fn isf_native_timing_actual_three_by_three_batch_order_and_coverage() {
        timing::enable_for_tests();
        let shader = crate::prepare_isf_shader(
            r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
            void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage); }"#,
        )
        .unwrap();
        let stages: [_; 3] = std::array::from_fn(|_| IsfStageInputs {
            shader: &shader,
            controls: &[],
            images: &[],
        });
        let frames: [_; 3] = std::array::from_fn(|index| {
            Arc::new(VideoFrame {
                layer_id: index as u64 + 1,
                width: 65,
                height: 3,
                pts_ms: 17,
                duration_ms: 17,
                format: VideoPixelFormat::Rgba8,
                data: [index as u8 * 39, 91, 173, 255].repeat(65 * 3),
            })
        });
        let keys: [_; 3] = std::array::from_fn(|index| {
            (0..3)
                .map(|stage| IsfInstanceKey::legacy(frames[index].layer_id, stage))
                .collect::<Vec<_>>()
        });
        let chains: [_; 3] = std::array::from_fn(|index| IndependentChain {
            frame: Arc::clone(&frames[index]),
            stages: &stages,
            keys: &keys[index],
            time_seconds: 7.0,
            time_delta_seconds: 0.017,
            date: [2026.0, 10.0, 25.0, 0.0],
        });
        let mut runtime = IsfGpuRuntime::new().expect("actual 3?3 probe requires a GPU");
        runtime.set_stack_phase_timing_enabled(false);
        timing::start().unwrap();
        timing::prepare_thread().unwrap();
        {
            let _parent = timing::begin_parent(44, 1, 65, 3);
            let output = runtime
                .apply_independent_chains(&chains)
                .expect("3?3 resource-neutral jobs must be admitted");
            assert_eq!(output.len(), 3);
            for ((actual, failures), expected) in output.iter().zip(&frames) {
                assert!(failures.is_empty());
                assert_eq!(actual, expected.as_ref());
            }
            assert_eq!(runtime.independent_batch_count(), 1);
            assert_eq!(runtime.resources.stack_upload_count, 3);
            assert_eq!(runtime.resources.stack_submit_count, 1);
            assert_eq!(runtime.frame_index, 3);
            assert_eq!(runtime.last_stack_stage_count, 9);
            timing::set_outcome(Outcome::Presented);
        }
        timing::stop_capture().unwrap();
        timing::retire_thread();
        timing::acknowledge_stop().unwrap();
        let snapshot = timing::snapshot().unwrap();
        assert!(!snapshot.overflow);
        assert_eq!(snapshot.unprepared_parents, 0);
        assert_eq!(snapshot.attempted_parents, 1);
        assert_eq!(snapshot.parents.len(), 1);
        let parent = &snapshot.parents[0];
        assert!(parent.complete);
        assert_eq!(parent.flags, 0);
        assert_eq!(parent.attempts.len(), 3);
        for (index, attempt) in parent.attempts.iter().enumerate() {
            assert_eq!(
                attempt.path,
                IsfPath::Batch3,
                "no recovery/ordinary fallback"
            );
            assert_eq!(attempt.layer_id, index as u64 + 1);
            assert_eq!(attempt.frame_index, index as u64);
            assert_eq!(attempt.stage_count, 3);
            assert_eq!(attempt.span_id, parent.children[0].id);
        }
        assert_eq!(
            parent.attempts.iter().map(|a| a.stage_count).sum::<u32>(),
            9
        );
        let expected = [
            Phase::Isf,
            Phase::IsfEncode,
            Phase::IsfUpload,
            Phase::IsfEncode,
            Phase::IsfUpload,
            Phase::IsfEncode,
            Phase::IsfUpload,
            Phase::IsfSubmit,
            Phase::IsfMapWait,
            Phase::IsfMappedCopy,
            Phase::IsfUnmap,
            Phase::IsfMappedCopy,
            Phase::IsfUnmap,
            Phase::IsfMappedCopy,
            Phase::IsfUnmap,
            Phase::IsfCleanup,
            Phase::IsfCleanup,
        ];
        assert_eq!(
            parent.children.iter().map(|c| c.phase).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(parent.children[0].parent_id, None);
        for (index, child) in parent.children.iter().enumerate().skip(1) {
            let expected_parent = if child.phase == Phase::IsfUpload {
                parent.children[index - 1].id
            } else {
                parent.children[0].id
            };
            assert_eq!(child.parent_id, Some(expected_parent));
            let enclosing = &parent.children[usize::from(expected_parent)];
            assert!(enclosing.start_us <= child.start_us);
            assert!(child.start_us <= child.end_us);
            assert!(child.end_us <= enclosing.end_us);
        }
        assert_eq!(
            parent.residual_us + parent.children.iter().map(|c| c.exclusive_us).sum::<u64>(),
            parent.end_us - parent.start_us,
        );
        // One cold post-Stop/ACK export keeps the probe's raw primitive evidence.
        println!(
            "QA_NATIVE_3X3 {}",
            serde_json::to_string(&snapshot).unwrap()
        );
    }
}

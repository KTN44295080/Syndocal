//! I2: compare CPU delegation with the frozen pre-delegation queue and P's oracle.
use super::*;
use crate::frame_history_policy::{
    drop_before as drop_history_before, plan_history_update, select_at_or_before_or_nearest,
    select_nearest, HistoryMeta, HistoryPushKind,
};
use crate::native_gpu_contract::HistoryEntryId;
use protocol::{VideoRenderInputId, VideoRenderInputKey};

const KEY: VideoRenderInputKey = VideoRenderInputKey {
    project_render_epoch: 29,
    input_id: VideoRenderInputId(41),
};
const LAYER: VideoLayerId = 7;

// The queue algorithm at HEAD 54bbc3f1, before canonical delegation. Keep this
// test-only oracle independent of frame_history_policy and the new CPU adapter.
struct FrozenFrameQueue {
    layer_id: VideoLayerId,
    slots: Vec<Option<Arc<VideoFrame>>>,
    start: usize,
    len: usize,
}

impl FrozenFrameQueue {
    fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            layer_id: LAYER,
            slots: vec![None; capacity],
            start: 0,
            len: 0,
        }
    }

    fn capacity(&self) -> usize {
        self.slots.len()
    }

    fn push(&mut self, frame: VideoFrame) -> FrameQueuePush {
        let mut frame = Arc::new(frame);
        if frame.layer_id != self.layer_id {
            Arc::make_mut(&mut frame).layer_id = self.layer_id;
        }
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
        let mut frames = self.drain_frames();
        self.rebuild_from_sorted(&mut frames);
        result
    }

    fn iter(&self) -> impl Iterator<Item = &VideoFrame> {
        (0..self.len).filter_map(|offset| {
            let index = (self.start + offset) % self.capacity();
            self.slots[index].as_deref()
        })
    }

    fn nearest(&self, target_ms: u64) -> Option<&VideoFrame> {
        self.iter()
            .min_by_key(|frame| frame.pts_ms.abs_diff(target_ms))
    }

    fn before(&self, target_ms: u64) -> Option<&VideoFrame> {
        self.iter()
            .filter(|frame| frame.pts_ms <= target_ms)
            .max_by_key(|frame| frame.pts_ms)
    }

    fn drop_before(&mut self, target_ms: u64) -> usize {
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

    fn drain_frames(&mut self) -> Vec<Arc<VideoFrame>> {
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

    fn rebuild_from_sorted(&mut self, frames: &mut Vec<Arc<VideoFrame>>) {
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

fn input_frame(sequence: u64, pts_ms: u64, layer_id: VideoLayerId) -> VideoFrame {
    VideoFrame {
        layer_id,
        width: 2,
        height: 1,
        pts_ms,
        duration_ms: sequence % 39,
        format: VideoPixelFormat::Rgba8,
        data: sequence.to_le_bytes().to_vec(),
    }
}

fn identity(frame: &VideoFrame) -> HistoryEntryId {
    HistoryEntryId {
        input: KEY,
        queue_epoch: 13,
        sequence: u64::from_le_bytes(frame.data.as_slice().try_into().unwrap()),
    }
}

fn metadata(frame: &VideoFrame) -> HistoryMeta {
    HistoryMeta {
        id: identity(frame),
        pts_ms: frame.pts_ms,
        duration_ms: frame.duration_ms,
    }
}

fn assert_equivalent(
    queue: &FrameQueue,
    frozen: &FrozenFrameQueue,
    ledger: &[HistoryMeta],
    targets: &[u64],
) {
    assert_eq!(queue.key(), KEY);
    assert_eq!(queue.layer_id(), LAYER);
    assert_eq!(queue.capacity(), frozen.capacity());
    assert_eq!(queue.len(), frozen.len);
    assert_eq!(queue.is_empty(), frozen.len == 0);
    assert_eq!(
        queue.iter().collect::<Vec<_>>(),
        frozen.iter().collect::<Vec<_>>()
    );
    assert_eq!(queue.iter().map(metadata).collect::<Vec<_>>(), ledger);
    for &target in targets {
        assert_eq!(
            queue.nearest(target),
            frozen.nearest(target),
            "nearest {target}"
        );
        assert_eq!(
            queue.frame_at_or_before(target),
            frozen.before(target),
            "predecessor {target}"
        );
        assert_eq!(
            queue.nearest(target).map(identity),
            select_nearest(ledger, target),
            "P nearest {target}"
        );
        assert_eq!(
            queue
                .frame_at_or_before(target)
                .or_else(|| queue.nearest(target))
                .map(identity),
            select_at_or_before_or_nearest(ledger, target),
            "P combined selection {target}"
        );
    }
}

#[test]
fn c1_frame_queue_delegation_matches_frozen_and_p_oracles_for_mixed_operations() {
    for capacity in [1, 2, 3, 8, 17] {
        let mut queue = FrameQueue::new_for_input(KEY, LAYER, capacity);
        let mut frozen = FrozenFrameQueue::new(capacity);
        let mut ledger = Vec::new();
        let mut random = 0x8b6f_072a_aa13_91b5u64;
        assert_equivalent(&queue, &frozen, &ledger, &[0, 1, u64::MAX]);
        for sequence in 1..=512u64 {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let pts = match sequence % 37 {
                0 => u64::MAX,
                1 => 0,
                2 => u64::MAX - 1,
                _ => (random >> 32) % 240,
            };
            let frame = input_frame(sequence, pts, 999);
            let incoming = metadata(&frame);
            let plan = plan_history_update(&ledger, &[incoming], capacity, pts).unwrap();
            let expected = match plan.pushes()[0] {
                HistoryPushKind::Inserted | HistoryPushKind::Replaced { .. } => {
                    FrameQueuePush::Inserted
                }
                HistoryPushKind::DroppedOldest { .. } => FrameQueuePush::DroppedOldest,
            };
            let actual = queue.push(frame.clone());
            assert_eq!(actual, frozen.push(frame));
            assert_eq!(actual, expected);
            ledger = plan.survivors().to_vec();
            let targets = [
                0,
                1,
                100,
                pts.saturating_sub(1),
                pts,
                pts.saturating_add(1),
                u64::MAX,
            ];
            assert_equivalent(&queue, &frozen, &ledger, &targets);
            if sequence % 11 == 0 {
                let threshold = if sequence % 33 == 0 {
                    u64::MAX
                } else {
                    (random >> 17) % 180
                };
                let expected_removed = drop_history_before(&mut ledger, threshold);
                assert_eq!(queue.drop_before(threshold), expected_removed);
                assert_eq!(frozen.drop_before(threshold), expected_removed);
                assert_equivalent(&queue, &frozen, &ledger, &targets);
            }
        }
    }
}

#[test]
fn c1_frame_queue_preserves_predecessor_none_ties_replacement_and_full_old_insert() {
    let mut queue = FrameQueue::new_for_input(KEY, LAYER, 2);
    queue.push(input_frame(1, 100, LAYER));
    queue.push(input_frame(2, 200, LAYER));
    assert!(queue.frame_at_or_before(99).is_none());
    assert_eq!(queue.nearest(99).unwrap().pts_ms, 100);
    assert_eq!(queue.nearest(150).unwrap().pts_ms, 100);
    assert_eq!(queue.frame_at_or_before(199).unwrap().pts_ms, 100);
    assert_eq!(
        queue.push(input_frame(3, 200, LAYER)),
        FrameQueuePush::Inserted
    );
    assert_eq!(queue.len(), 2);
    assert_eq!(identity(queue.nearest(200).unwrap()).sequence, 3);
    assert_eq!(
        queue.push(input_frame(4, 50, LAYER)),
        FrameQueuePush::DroppedOldest
    );
    assert_eq!(
        queue.iter().map(|frame| frame.pts_ms).collect::<Vec<_>>(),
        [50, 200]
    );
    assert_eq!(queue.drop_before(50), 0);
    assert_eq!(queue.drop_before(200), 1);
    assert_eq!(queue.frame_at_or_before(200).unwrap().pts_ms, 200);
    assert_eq!(FrameQueue::new(LAYER, 1).key(), VideoRenderInputKey::LEGACY);
}

#[test]
fn c1_frame_queue_preserves_arc_ownership_and_only_clones_for_shared_normalization() {
    let mut queue = FrameQueue::new_for_input(KEY, LAYER, 2);
    let shared = Arc::new(input_frame(1, 100, LAYER));
    let weak = Arc::downgrade(&shared);
    let shared_ptr = Arc::as_ptr(&shared);
    let pixels_ptr = shared.data.as_ptr();
    queue.push_shared(Arc::clone(&shared));
    assert_eq!(Arc::strong_count(&shared), 2);
    assert_eq!(Arc::as_ptr(queue.nearest_shared(100).unwrap()), shared_ptr);
    assert_eq!(queue.nearest(100).unwrap().data.as_ptr(), pixels_ptr);

    queue.push(input_frame(2, 100, LAYER));
    assert_eq!(Arc::strong_count(&shared), 1);
    drop(shared);
    assert!(
        weak.upgrade().is_none(),
        "replacement releases only the superseded owner"
    );

    let unique_foreign = Arc::new(input_frame(3, 200, 999));
    let unique_ptr = Arc::as_ptr(&unique_foreign);
    let unique_pixels = unique_foreign.data.as_ptr();
    queue.push_shared(unique_foreign);
    assert_eq!(Arc::as_ptr(queue.nearest_shared(200).unwrap()), unique_ptr);
    assert_eq!(queue.nearest(200).unwrap().data.as_ptr(), unique_pixels);
    assert_eq!(queue.nearest(200).unwrap().layer_id, LAYER);

    let shared_foreign = Arc::new(input_frame(4, 300, 999));
    let foreign_pixels = shared_foreign.data.as_ptr();
    queue.push_shared(Arc::clone(&shared_foreign));
    assert_eq!(shared_foreign.layer_id, 999);
    assert_eq!(Arc::strong_count(&shared_foreign), 1);
    assert_ne!(queue.nearest(300).unwrap().data.as_ptr(), foreign_pixels);
    assert_eq!(queue.nearest(300).unwrap().data, shared_foreign.data);
    assert_eq!(queue.nearest(300).unwrap().layer_id, LAYER);
    let retained = Arc::downgrade(queue.nearest_shared(200).unwrap());
    assert_eq!(queue.drop_before(300), 1);
    assert!(
        retained.upgrade().is_none(),
        "drop_before releases the removed queue owner"
    );
}

#[test]
#[should_panic(expected = "frame queue capacity must be greater than zero")]
fn c1_frame_queue_zero_capacity_remains_rejected() {
    FrameQueue::new_for_input(KEY, LAYER, 0);
}

#[test]
fn c1_frame_queue_prepared_push_rejects_stale_aba_and_foreign_source_owners() {
    let mut queue = FrameQueue::new_for_input(KEY, LAYER, 2);
    queue.push(input_frame(1, 100, LAYER));
    let prepared = queue.prepare_push(Arc::new(input_frame(2, 200, LAYER)));
    // Same PTS and duration, different pixels/owner: metadata alone is an ABA.
    queue.push(input_frame(40, 100, LAYER));
    assert_eq!(prepared.policy.current(), queue.policy.as_slice());
    let revision = queue.policy_revision;
    assert_eq!(
        queue.apply_prepared_push(prepared),
        Err(FrameQueuePlanError::StaleRevision)
    );
    assert_eq!(queue.policy_revision, revision);
    assert_eq!(identity(queue.nearest(100).unwrap()).sequence, 40);
    assert_eq!(queue.len(), 1);

    let mut origin = FrameQueue::new_for_input(KEY, LAYER, 2);
    let mut receiver = FrameQueue::new_for_input(KEY, LAYER, 2);
    origin.push(input_frame(1, 100, LAYER));
    receiver.push(input_frame(1, 100, LAYER));
    assert_eq!(origin.policy_revision, receiver.policy_revision);
    assert_eq!(origin.policy, receiver.policy);
    let prepared = origin.prepare_push(Arc::new(input_frame(2, 200, LAYER)));
    assert_eq!(
        receiver.apply_prepared_push(prepared),
        Err(FrameQueuePlanError::SourceMismatch)
    );
    assert_eq!(receiver.len(), 1);
    assert_eq!(receiver.policy_revision, 1);

    let prepared = receiver.prepare_push(Arc::new(input_frame(3, 300, LAYER)));
    assert_eq!(receiver.drop_before(200), 1);
    assert_eq!(
        receiver.apply_prepared_push(prepared),
        Err(FrameQueuePlanError::StaleRevision)
    );
    assert!(receiver.is_empty());
}

#[test]
fn c1_frame_queue_prepared_push_checks_current_incoming_selection_and_capacity() {
    let mut queue = FrameQueue::new_for_input(KEY, LAYER, 2);
    queue.push(input_frame(1, 100, LAYER));
    let baseline = queue.iter().cloned().collect::<Vec<_>>();
    let revision = queue.policy_revision;

    let mut prepared = queue.prepare_push(Arc::new(input_frame(2, 200, LAYER)));
    let mut wrong_current = prepared.policy.current().to_vec();
    wrong_current[0].duration_ms += 1;
    prepared.policy = plan_history_update(
        &wrong_current,
        prepared.policy.incoming(),
        queue.capacity(),
        200,
    )
    .unwrap()
    .bind_revision(revision);
    assert_eq!(
        queue.apply_prepared_push(prepared),
        Err(FrameQueuePlanError::SourceMismatch)
    );

    let mut prepared = queue.prepare_push(Arc::new(input_frame(2, 200, LAYER)));
    Arc::make_mut(&mut prepared.incoming).pts_ms += 1;
    assert_eq!(
        queue.apply_prepared_push(prepared),
        Err(FrameQueuePlanError::IncomingMismatch)
    );

    let mut prepared = queue.prepare_push(Arc::new(input_frame(2, 200, LAYER)));
    prepared.policy = plan_history_update(
        &queue.policy,
        prepared.policy.incoming(),
        queue.capacity(),
        100,
    )
    .unwrap()
    .bind_revision(revision);
    assert_eq!(
        queue.apply_prepared_push(prepared),
        Err(FrameQueuePlanError::IncomingMismatch)
    );

    let mut prepared = queue.prepare_push(Arc::new(input_frame(2, 200, LAYER)));
    // This plan is valid for capacity one, but must not evict from capacity two.
    prepared.policy = plan_history_update(&queue.policy, prepared.policy.incoming(), 1, 200)
        .unwrap()
        .bind_revision(revision);
    assert_eq!(
        queue.apply_prepared_push(prepared),
        Err(FrameQueuePlanError::InvalidPlan)
    );
    assert_eq!(queue.policy_revision, revision);
    assert_eq!(queue.iter().cloned().collect::<Vec<_>>(), baseline);
}

#[test]
fn c1_frame_queue_revision_exhaustion_fails_before_mutation_and_keeps_queries_available() {
    let mut queue = FrameQueue::new_for_input(KEY, LAYER, 2);
    queue.push(input_frame(1, 100, LAYER));
    queue.policy_revision = u64::MAX;
    let before = queue.iter().cloned().collect::<Vec<_>>();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        queue.push(input_frame(2, 200, LAYER));
    }))
    .is_err());
    assert_eq!(queue.iter().cloned().collect::<Vec<_>>(), before);
    assert_eq!(queue.policy_revision, u64::MAX);
    assert_eq!(
        queue.drop_before(100),
        0,
        "a no-op drop does not consume a revision"
    );
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        queue.drop_before(101);
    }))
    .is_err());
    assert_eq!(queue.iter().cloned().collect::<Vec<_>>(), before);
    assert_eq!(queue.policy_revision, u64::MAX);
    assert_eq!(queue.nearest(99).unwrap().pts_ms, 100);
    assert!(queue.frame_at_or_before(99).is_none());
}

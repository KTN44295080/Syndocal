//! CPU FrameQueue equivalence, not GPU execution evidence.

use super::*;
use crate::{FrameQueue, FrameQueuePush, VideoFrame, VideoPixelFormat};
use protocol::{VideoRenderInputId, VideoRenderInputKey};

const KEY: VideoRenderInputKey = VideoRenderInputKey {
    project_render_epoch: 23,
    input_id: VideoRenderInputId(41),
};
const LAYER: u64 = 7;

fn meta(sequence: u64, pts_ms: u64) -> HistoryMeta {
    HistoryMeta {
        id: HistoryEntryId {
            input: KEY,
            queue_epoch: 5,
            sequence,
        },
        pts_ms,
        duration_ms: sequence % 34,
    }
}

fn frame(meta: HistoryMeta) -> VideoFrame {
    VideoFrame {
        // Deliberately foreign diagnostic layer; CPU normalizes this to LAYER.
        layer_id: 999,
        width: 2,
        height: 1,
        pts_ms: meta.pts_ms,
        duration_ms: meta.duration_ms,
        format: VideoPixelFormat::Rgba8,
        data: meta.id.sequence.to_le_bytes().to_vec(),
    }
}

fn identity(frame: &VideoFrame) -> HistoryEntryId {
    HistoryEntryId {
        input: KEY,
        queue_epoch: 5,
        sequence: u64::from_le_bytes(frame.data.as_slice().try_into().unwrap()),
    }
}

fn assert_queue_equivalent(queue: &FrameQueue, native: &[HistoryMeta], targets: &[u64]) {
    assert_eq!(queue.key(), KEY);
    assert_eq!(queue.len(), native.len());
    assert_eq!(queue.capacity(), NATIVE_HISTORY_CAPACITY);
    assert_eq!(queue.is_empty(), native.is_empty());
    let actual: Vec<_> = queue
        .iter()
        .map(|f| {
            assert_eq!(f.layer_id, LAYER);
            HistoryMeta {
                id: identity(f),
                pts_ms: f.pts_ms,
                duration_ms: f.duration_ms,
            }
        })
        .collect();
    assert_eq!(actual, native);
    for &target in targets {
        assert_eq!(
            select_nearest(native, target),
            queue.nearest(target).map(identity),
            "nearest target={target}"
        );
        let selected = queue
            .frame_at_or_before(target)
            .or_else(|| queue.nearest(target));
        assert_eq!(
            select_at_or_before_or_nearest(native, target),
            selected.map(identity),
            "selected target={target}"
        );
    }
}

fn push_both(
    queue: &mut FrameQueue,
    native: &[HistoryMeta],
    incoming: &[HistoryMeta],
    target: u64,
) -> Vec<HistoryMeta> {
    let plan = plan_history_update(native, incoming, NATIVE_HISTORY_CAPACITY, target).unwrap();
    assert_eq!(plan.current(), native);
    assert_eq!(plan.incoming(), incoming);
    for (incoming, kind) in incoming.iter().zip(plan.pushes()) {
        let expected_push = match kind {
            HistoryPushKind::Inserted | HistoryPushKind::Replaced { .. } => {
                FrameQueuePush::Inserted
            }
            HistoryPushKind::DroppedOldest { .. } => FrameQueuePush::DroppedOldest,
        };
        assert_eq!(queue.push(frame(*incoming)), expected_push);
    }
    assert_queue_equivalent(
        queue,
        plan.survivors(),
        &[0, 1, 16, 33, 50, 99, 100, 116, 150, 200, 800, u64::MAX],
    );
    assert_eq!(
        plan.selected(),
        queue
            .frame_at_or_before(target)
            .or_else(|| queue.nearest(target))
            .map(identity)
    );
    plan.survivors().to_vec()
}

#[test]
fn table_pushes_match_cpu_eight_slots_replacement_old_insert_and_reverse_order() {
    let cases: &[(&[u64], &[u64], &[u64])] = &[
        (&[], &[], &[]),
        (&[], &[0, 33, 66], &[0, 33, 66]),
        (&[0, 33, 66], &[33], &[0, 33, 66]),
        (
            &[100, 200, 300, 400, 500, 600, 700, 800],
            &[50],
            &[50, 200, 300, 400, 500, 600, 700, 800],
        ),
        (
            &[],
            &[800, 700, 600, 500, 400, 300, 200, 100, 50],
            &[50, 200, 300, 400, 500, 600, 700, 800],
        ),
        (
            &[100, 200, 300, 400, 500, 600, 700, 800],
            &[800, 900],
            &[200, 300, 400, 500, 600, 700, 800, 900],
        ),
    ];
    for (old_pts, new_pts, expected_pts) in cases {
        let mut queue = FrameQueue::new_for_input(KEY, LAYER, NATIVE_HISTORY_CAPACITY);
        let old: Vec<_> = old_pts
            .iter()
            .enumerate()
            .map(|(i, pts)| meta(i as u64 + 1, *pts))
            .collect();
        for &m in &old {
            queue.push(frame(m));
        }
        let incoming: Vec<_> = new_pts
            .iter()
            .enumerate()
            .map(|(i, pts)| meta(i as u64 + 100, *pts))
            .collect();
        let result = push_both(&mut queue, &old, &incoming, 150);
        assert_eq!(
            result.iter().map(|m| m.pts_ms).collect::<Vec<_>>(),
            *expected_pts
        );
    }
}

#[test]
fn predecessor_is_selected_even_when_new_decode_is_future_and_nearest_ties_low() {
    let old = [meta(1, 0)];
    let plan = plan_history_update(&old, &[meta(2, 33)], NATIVE_HISTORY_CAPACITY, 16).unwrap();
    assert_eq!(plan.selected(), Some(old[0].id));
    let mut queue = FrameQueue::new_for_input(KEY, LAYER, NATIVE_HISTORY_CAPACITY);
    for m in [meta(3, 100), meta(4, 200)] {
        queue.push(frame(m));
    }
    let ledger = [meta(3, 100), meta(4, 200)];
    assert_eq!(select_nearest(&ledger, 150), Some(ledger[0].id));
    assert_queue_equivalent(&queue, &ledger, &[0, 99, 100, 149, 150, 151, 200, u64::MAX]);
}

#[test]
fn drop_before_matches_cpu_including_equal_timestamp_and_u64_edges() {
    for threshold in [0, 1, 100, 150, 200, u64::MAX] {
        let mut native = vec![meta(1, 0), meta(2, 100), meta(3, 200), meta(4, u64::MAX)];
        let mut queue = FrameQueue::new_for_input(KEY, LAYER, NATIVE_HISTORY_CAPACITY);
        for &m in &native {
            queue.push(frame(m));
        }
        assert_eq!(
            drop_before(&mut native, threshold),
            queue.drop_before(threshold)
        );
        assert_queue_equivalent(&queue, &native, &[0, threshold, u64::MAX]);
    }
}

#[test]
fn prospective_plan_preserves_source_and_revision_without_publication() {
    let current = vec![meta(1, 100)];
    let incoming = [meta(2, 100), meta(3, 200)];
    let plan = plan_history_update(&current, &incoming, NATIVE_HISTORY_CAPACITY, 200)
        .unwrap()
        .bind_revision(9);
    assert_eq!(current, vec![meta(1, 100)]);
    assert_eq!(plan.expected_revision(), 9);
    assert_eq!(plan.current(), current);
    assert_eq!(plan.incoming(), incoming);
    assert_eq!(plan.survivors(), incoming);
    assert_eq!(
        plan.pushes(),
        &[
            HistoryPushKind::Replaced { old: current[0].id },
            HistoryPushKind::Inserted
        ]
    );
    assert_eq!(plan.selected(), Some(incoming[1].id));
}

#[test]
fn malformed_ledgers_and_reused_entry_identity_are_rejected() {
    let a = meta(1, 10);
    let b = meta(2, 20);
    assert!(matches!(
        plan_history_update(&[], &[], 0, 0),
        Err(HistoryPolicyError::InvalidCapacity)
    ));
    assert!(matches!(
        plan_history_update(&[a, b], &[], 1, 0),
        Err(HistoryPolicyError::OverCapacity)
    ));
    assert!(matches!(
        plan_history_update(&[b, a], &[], 8, 0),
        Err(HistoryPolicyError::NotStrictlyAscending)
    ));
    assert!(matches!(
        plan_history_update(&[a, meta(2, 10)], &[], 8, 0),
        Err(HistoryPolicyError::NotStrictlyAscending)
    ));
    assert!(matches!(
        plan_history_update(&[a], &[a], 8, 0),
        Err(HistoryPolicyError::DuplicateIdentity)
    ));
    assert!(matches!(
        plan_history_update(&[], &[a, a], 8, 0),
        Err(HistoryPolicyError::DuplicateIdentity)
    ));
    assert!(matches!(
        plan_history_update(&[a, HistoryMeta { pts_ms: 20, ..a }], &[], 8, 0),
        Err(HistoryPolicyError::DuplicateIdentity)
    ));
}

#[test]
fn cadence_thirty_to_sixty_two_twelve_second_loops_and_reverse_seeks_match_cpu_identity() {
    let mut queue = FrameQueue::new_for_input(KEY, LAYER, NATIVE_HISTORY_CAPACITY);
    let mut native = Vec::new();
    let mut sequence = 0;
    // Each render tick gets a new logical result even at a repeated source PTS.
    for tick in 0..1440u64 {
        let target = ((tick * 1000) / 60) % 12_000;
        // Model the decoder's first PTS at or above target exactly in 30Hz ticks.
        let source_tick = (target * 30).div_ceil(1000);
        let pts = (source_tick * 1000).div_ceil(30).min(12_000);
        assert!(pts >= target);
        sequence += 1;
        native = push_both(&mut queue, &native, &[meta(sequence, pts)], target);
        // A future decode must never force selection over an available past ID.
        assert_queue_equivalent(
            &queue,
            &native,
            &[target, target.saturating_sub(50), target.saturating_add(50)],
        );
    }
    for target in [11_900u64, 11_850, 11_000, 10, 0, 100, 50, 11_999] {
        sequence += 1;
        let pts = ((target * 30).div_ceil(1000) * 1000).div_ceil(30);
        native = push_both(&mut queue, &native, &[meta(sequence, pts)], target);
    }
}

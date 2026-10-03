//! Authored Timeline boundaries must never carry a previous transport session.
use crate::{TimelineFollowRuntimeSummary, TimelineLoopRuntimeSummary, TimelineSnapshot};

/// Clear only runtime transport state. Authored content, audio policy, tempo,
/// loop/follow configuration and duration remain unchanged. Callers validate
/// authored references before using this at project ingress.
pub fn clear_timeline_transport_runtime(timeline: &mut TimelineSnapshot) {
    timeline.playing = false;
    timeline.position_ms = 0;
    timeline.count_in_remaining_ms = 0;
    timeline.audio_transport_revision = 0;
    timeline.transport_epoch = 0;
    timeline.transport_generation = 0;
    timeline.active_child_transports.clear();
    timeline.loop_runtime = TimelineLoopRuntimeSummary::default();
    timeline.follow_runtime = TimelineFollowRuntimeSummary::default();
    timeline.guide_cues.clear();
    timeline.click_events.clear();
    timeline.click_schedule_generation = 0;
    timeline.click_queue_overflow = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChildTimelineTransportRootSummary, ChildTimelineTransportRuntimeSummary,
        TimelineClickEventSummary, TimelineGuideAssetKey, TimelineGuideCueKind,
        TimelineGuideCueSummary, TimelineScheduleSource,
    };

    #[test]
    fn timeline_persistence_clears_transport_without_changing_authored_content() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../qa/migration/authored-control-project.json"
        ))
        .unwrap();
        let authored: TimelineSnapshot =
            serde_json::from_value(fixture["snapshot"]["timeline"].clone()).unwrap();
        let mut runtime = authored.clone();
        runtime.playing = true;
        runtime.position_ms = 750;
        runtime.count_in_remaining_ms = 1_000;
        runtime.audio_transport_revision = 777;
        runtime.transport_epoch = 9;
        runtime.transport_generation = 10;
        runtime
            .active_child_transports
            .push(ChildTimelineTransportRuntimeSummary {
                owner_cue_id: 1,
                root: ChildTimelineTransportRootSummary::Direct {
                    parent_cue_id: 1,
                    generation: 2,
                },
                path: Vec::new(),
                position_ms: 50,
                audio_position_ms: 50,
                audio_active: true,
                playback_rate_milli: 1_000,
            });
        runtime.loop_runtime.generation = 11;
        runtime.loop_runtime.wrap_count = 12;
        runtime.follow_runtime.generation = 13;
        runtime.follow_runtime.transition_hold_active = true;
        runtime.follow_runtime.waiting_for_pedal_start = true;
        runtime.guide_cues.push(TimelineGuideCueSummary {
            generation: 14,
            sequence: 15,
            at_ms: 0,
            label: "Complete".into(),
            cue: TimelineGuideCueKind::Complete,
            asset: TimelineGuideAssetKey::Complete,
            playback_rate_milli: 1_000,
            sample_frame: 0,
            epoch: 9,
            transport_generation: 10,
            schedule_generation: 16,
            source: TimelineScheduleSource::Root,
        });
        runtime.click_events.push(TimelineClickEventSummary {
            sample_frame: 17,
            ..TimelineClickEventSummary::default()
        });
        runtime.click_schedule_generation = 16;
        runtime.click_queue_overflow = Some("previous session overflow".into());
        clear_timeline_transport_runtime(&mut runtime);
        // Struct equality covers runtime fields skipped by JSON as well as
        // every authored field; serialization alone would miss those queues.
        assert_eq!(runtime, authored);
        clear_timeline_transport_runtime(&mut runtime);
        assert_eq!(runtime, authored, "projection must be idempotent");
    }
}

//! Shared admission and emission for OSC/DMX Perform input.
//! The renderer remains the sole owner of the resulting project command.

use crate::{midi_fader_input, midi_renderer_routed, ExternalControlDispatchState};
use io::osc::OscInputEvent;
use protocol::{DmxControlMapping, RendererRoutedInput};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};
use tauri::Emitter;

pub(crate) fn forward(
    app: &tauri::AppHandle,
    event_name: &'static str,
    connection: midi_fader_input::Connection,
    sequence: &AtomicU64,
    fences: ExternalControlDispatchState<'_>,
    input: RendererRoutedInput,
) {
    forward_with(connection, sequence, fences, input, |payload| {
        let _ = app.emit(event_name, payload);
    });
}

fn forward_with(
    connection: midi_fader_input::Connection,
    sequence: &AtomicU64,
    fences: ExternalControlDispatchState<'_>,
    input: RendererRoutedInput,
    emit: impl FnMut(midi_renderer_routed::Input),
) -> bool {
    forward_batch_with(connection, sequence, fences, vec![input], emit)
}

fn forward_batch_with(
    connection: midi_fader_input::Connection,
    sequence: &AtomicU64,
    fences: ExternalControlDispatchState<'_>,
    inputs: Vec<RendererRoutedInput>,
    mut emit: impl FnMut(midi_renderer_routed::Input),
) -> bool {
    let (callback_epoch, transaction_active, admission, installed, captured_epoch) = fences;
    let permitted = crate::run_if_installed_project_callback_epoch(
        admission,
        callback_epoch,
        captured_epoch,
        transaction_active,
        installed,
        || true,
    )
    .unwrap_or(false);
    if !permitted {
        return false;
    }
    let Ok(count) = u64::try_from(inputs.len()) else {
        return false;
    };
    // Reserve the whole frame before emitting. A busy gate or exhausted sequence
    // never half-consumes a DMX frame and replays its already emitted triggers.
    let Ok(previous) = sequence.fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
        value.checked_add(count)
    }) else {
        return false;
    };
    // Admission is released before emission. The receiver checks the captured
    // project epoch and exact input session; sequence gaps imply no command.
    for (offset, input) in inputs.into_iter().enumerate() {
        emit(midi_renderer_routed::Input {
            connection,
            sequence: previous + offset as u64 + 1,
            input,
        });
    }
    true
}

pub(crate) struct DmxFrame<'a> {
    pub universe: u16,
    pub values: &'a [u8; 512],
    pub previous: &'a Mutex<HashMap<u16, Box<[u8; 512]>>>,
    pub mappings: &'a [DmxControlMapping],
}

pub(crate) fn forward_dmx_frame(
    app: &tauri::AppHandle,
    connection: midi_fader_input::Connection,
    sequence: &AtomicU64,
    fences: ExternalControlDispatchState<'_>,
    frame: DmxFrame<'_>,
) -> Option<Vec<OscInputEvent>> {
    forward_dmx_frame_with(connection, sequence, fences, frame, |payload| {
        let _ = app.emit(midi_renderer_routed::DMX_EVENT, payload);
    })
}

fn forward_dmx_frame_with(
    connection: midi_fader_input::Connection,
    sequence: &AtomicU64,
    fences: ExternalControlDispatchState<'_>,
    frame: DmxFrame<'_>,
    emit: impl FnMut(midi_renderer_routed::Input),
) -> Option<Vec<OscInputEvent>> {
    let mut previous = frame.previous.lock().ok()?;
    let events = io::dmx_input::control_events_from_changed_frame(
        frame.universe,
        frame.values,
        previous.get(&frame.universe).map(|value| value.as_ref()),
        frame.mappings,
    );
    let mut routed = Vec::new();
    let mut legacy = Vec::new();
    for event in events {
        match event {
            OscInputEvent::RendererRouted(input) => routed.push(input),
            event => legacy.push(event),
        }
    }
    if !forward_batch_with(connection, sequence, fences, routed, emit) {
        return None;
    }
    // Keep the previous accepted frame when publication is busy. Repeating the
    // same final DMX value after admission reopens must deliver that value once.
    previous.insert(frame.universe, Box::new(*frame.values));
    Some(legacy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProjectExternalCommandAdmission;
    use protocol::MidiControlAction;
    use std::sync::{atomic::AtomicBool, Arc};

    fn input() -> RendererRoutedInput {
        RendererRoutedInput {
            action: MidiControlAction::VideoTake,
            layer_id: None,
            clip_slot_id: None,
            isf_stage_id: None,
            control: None,
            position: 1.0,
            value: 1.0,
            active: true,
            toggle: false,
        }
    }

    #[test]
    fn forwarding_checks_every_callback_fence_without_holding_admission_during_emit() {
        let epoch = Arc::new(AtomicU64::new(7));
        let active = Arc::new(AtomicBool::new(false));
        let admission = Arc::new(ProjectExternalCommandAdmission::default());
        let installed = Arc::new(AtomicBool::new(true));
        let sequence = AtomicU64::new(0);
        let connection = midi_fader_input::Connection::new(42).unwrap();
        for (current_epoch, pending, is_installed, expected) in [
            (7, false, true, true),
            (8, false, true, false),
            (7, true, true, false),
            (7, false, false, false),
        ] {
            epoch.store(current_epoch, Ordering::Release);
            active.store(pending, Ordering::Release);
            installed.store(is_installed, Ordering::Release);
            let mut forwarded = false;
            forward_with(
                connection,
                &sequence,
                (&epoch, &active, &admission, &installed, 7),
                input(),
                |payload| {
                    assert!(admission.gate.try_lock().is_ok());
                    assert_eq!(payload.connection.project_epoch, 42);
                    assert_eq!(payload.connection.input_session, connection.input_session);
                    assert_eq!(payload.sequence, 1);
                    forwarded = true;
                },
            );
            assert_eq!(forwarded, expected);
        }
        assert_eq!(sequence.load(Ordering::Acquire), 1);
    }

    #[test]
    fn forwarding_rejects_a_busy_admission_gate_and_sequence_exhaustion() {
        let epoch = Arc::new(AtomicU64::new(7));
        let active = Arc::new(AtomicBool::new(false));
        let admission = Arc::new(ProjectExternalCommandAdmission::default());
        let installed = Arc::new(AtomicBool::new(true));
        let sequence = AtomicU64::new(0);
        let connection = midi_fader_input::Connection::new(42).unwrap();
        let guard = admission.gate.lock().unwrap();
        forward_with(
            connection,
            &sequence,
            (&epoch, &active, &admission, &installed, 7),
            input(),
            |_| panic!("busy project admission must not forward input"),
        );
        drop(guard);
        assert_eq!(sequence.load(Ordering::Acquire), 0);
        sequence.store(u64::MAX, Ordering::Release);
        forward_with(
            connection,
            &sequence,
            (&epoch, &active, &admission, &installed, 7),
            input(),
            |_| panic!("exhausted sequence must not wrap"),
        );
    }

    #[test]
    fn dmx_repeated_final_frame_recovers_after_every_admission_fence_without_duplicate_triggers() {
        let epoch = Arc::new(AtomicU64::new(7));
        let active = Arc::new(AtomicBool::new(false));
        let admission = Arc::new(ProjectExternalCommandAdmission::default());
        let installed = Arc::new(AtomicBool::new(false));
        let sequence = AtomicU64::new(0);
        let connection = midi_fader_input::Connection::new(42).unwrap();
        let previous = Mutex::new(HashMap::new());
        let mappings: Vec<DmxControlMapping> = ["VideoTake", "VideoIsfParam"]
            .into_iter()
            .map(|action| {
                serde_json::from_value(serde_json::json!({
                    "universe": 0, "channel": 1, "action": action,
                    "layer_id": 1, "isf_stage_id": 3, "attribute": "gain", "low": 0.0, "high": 1.0
                }))
                .unwrap()
            })
            .collect();
        let mut values = [0; 512];
        values[0] = 255;
        let mut emitted = Vec::new();
        for (busy, pending, is_installed) in [
            (false, false, false),
            (false, true, true),
            (true, false, true),
        ] {
            active.store(pending, Ordering::Release);
            installed.store(is_installed, Ordering::Release);
            let guard = busy.then(|| admission.gate.lock().unwrap());
            let result = forward_dmx_frame_with(
                connection,
                &sequence,
                (&epoch, &active, &admission, &installed, 7),
                DmxFrame {
                    universe: 0,
                    values: &values,
                    previous: &previous,
                    mappings: &mappings,
                },
                |input| emitted.push(input),
            );
            assert!(result.is_none());
            drop(guard);
            assert!(previous.lock().unwrap().is_empty());
            assert!(emitted.is_empty());
            assert_eq!(sequence.load(Ordering::Acquire), 0);
        }
        active.store(false, Ordering::Release);
        for _ in 0..2 {
            let result = forward_dmx_frame_with(
                connection,
                &sequence,
                (&epoch, &active, &admission, &installed, 7),
                DmxFrame {
                    universe: 0,
                    values: &values,
                    previous: &previous,
                    mappings: &mappings,
                },
                |input| {
                    assert!(admission.gate.try_lock().is_ok());
                    emitted.push(input);
                },
            );
            assert!(result.unwrap().is_empty());
        }
        assert_eq!(
            emitted.len(),
            2,
            "one trigger and one final fader value, without replay"
        );
        assert_eq!(emitted[0].input.action, MidiControlAction::VideoTake);
        assert_eq!(emitted[1].input.value, 1.0);
        assert_eq!((emitted[0].sequence, emitted[1].sequence), (1, 2));
    }
}

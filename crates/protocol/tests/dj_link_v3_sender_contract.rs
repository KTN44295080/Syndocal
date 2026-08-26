use protocol::{DjLinkEnvelope, DjLinkLoopStatePayload, DjLinkMessageType};

const RB_OUTPUT_MEASURED_LOOP_FRAME: &str = r#"{
  "v": 3,
  "type": "DJ_LOOP_STATE",
  "agentId": "rb-output-dj-agent",
  "sessionId": "rb-session-1",
  "sequence": 17,
  "eventId": "measured-loop-17",
  "payload": {
    "deck": 1,
    "deckId": "rekordbox-deck-1",
    "masterDeckRevision": 4,
    "playSessionId": "play-session-1",
    "loop": {
      "active": true,
      "startBeat": 32.0,
      "endBeat": 32.015625,
      "lengthBeats": 0.015625,
      "revision": 9,
      "sampleAgeMs": 0,
      "source": "rekordbox-hook-measured"
    }
  }
}"#;

#[test]
fn rb_output_measured_loop_wire_shape_is_accepted_by_strict_v3_ingress() {
    let envelope = DjLinkEnvelope::parse_json(RB_OUTPUT_MEASURED_LOOP_FRAME)
        .expect("rb-output measured loop frame must satisfy strict v3 ingress");
    assert_eq!(envelope.message_type, DjLinkMessageType::LoopState);

    let payload: DjLinkLoopStatePayload = serde_json::from_value(envelope.payload)
        .expect("validated measured loop payload must deserialize exactly");
    assert_eq!(payload.deck, 1);
    assert_eq!(payload.master_deck_revision, 4);
    assert_eq!(payload.play_session_id, "play-session-1");
    assert!(payload.loop_state.active);
    assert_eq!(payload.loop_state.length_beats, Some(1.0 / 64.0));
    assert_eq!(payload.loop_state.revision, 9);
}

#[test]
fn retired_flat_measured_loop_wire_shape_fails_closed() {
    let flat = RB_OUTPUT_MEASURED_LOOP_FRAME.replace(
        r#""loop": {
      "active": true,
      "startBeat": 32.0,
      "endBeat": 32.015625,
      "lengthBeats": 0.015625,
      "revision": 9,
      "sampleAgeMs": 0,
      "source": "rekordbox-hook-measured"
    }"#,
        r#""active": true,
    "startBeat": 32.0,
    "endBeat": 32.015625,
    "lengthBeats": 0.015625,
    "revision": 9,
    "sampleAgeMs": 0,
    "source": "rekordbox-hook-measured""#,
    );

    let error = DjLinkEnvelope::parse_json(&flat)
        .expect_err("retired flat measured loop payload must stay unreachable");
    assert!(error.contains("unknown field") || error.contains("missing field"));
}

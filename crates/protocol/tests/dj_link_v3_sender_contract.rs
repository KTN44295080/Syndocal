use protocol::{DjLinkEnvelope, DjLinkMessageType, DjLinkTrackLoopStatePayload};

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

const RB_OUTPUT_TRACK_ACTIVE_FRAME: &str = r#"{
  "v":3,"type":"DJ_TRACK_ACTIVE","agentId":"rb-output-dj-agent",
  "sessionId":"rb-session-1","sequence":19,"eventId":"track-active-19",
  "payload":{
    "deck":2,"deckId":"rekordbox-deck-2","contentId":"content-2",
    "positionAtSendSec":12.5,"effectiveBpm":128.0,"positionRevision":4,
    "sampleAgeMs":0,"isPlaying":true,"startedAt":"2026-08-26T00:00:00Z",
    "playSessionId":"play-session-2","loop":null
  }
}"#;

#[test]
fn rb_output_measured_loop_wire_shape_is_accepted_by_strict_v3_ingress() {
    let envelope = DjLinkEnvelope::parse_json(RB_OUTPUT_MEASURED_LOOP_FRAME)
        .expect("rb-output measured loop frame must satisfy strict v3 ingress");
    assert_eq!(envelope.message_type, DjLinkMessageType::LoopState);

    let payload: DjLinkTrackLoopStatePayload = serde_json::from_value(envelope.payload)
        .expect("validated measured loop payload must deserialize exactly");
    assert_eq!(payload.deck, 1);
    assert_eq!(payload.play_session_id, "play-session-1");
    assert!(payload.loop_state.active);
    assert_eq!(payload.loop_state.length_beats, Some(1.0 / 64.0));
    assert_eq!(payload.loop_state.revision, 9);
}

#[test]
fn canonical_per_deck_measured_loop_rejects_master_revision() {
    let legacy_field = RB_OUTPUT_MEASURED_LOOP_FRAME.replace(
        r#""playSessionId": "play-session-1""#,
        r#""masterDeckRevision": 4,
    "playSessionId": "play-session-1""#,
    );
    let envelope = DjLinkEnvelope::parse_json(&legacy_field)
        .expect("the retired compatibility payload remains parseable at ingress");
    assert!(serde_json::from_value::<DjLinkTrackLoopStatePayload>(envelope.payload).is_err());
}

#[test]
fn canonical_per_deck_track_requires_exactly_one_identity_and_no_master_fields() {
    assert!(DjLinkEnvelope::parse_json(RB_OUTPUT_TRACK_ACTIVE_FRAME).is_ok());
    let null_track_bpm = RB_OUTPUT_TRACK_ACTIVE_FRAME.replace(
        r#""contentId":"content-2","#,
        r#""contentId":"content-2","trackBpm":null,"#,
    );
    assert!(DjLinkEnvelope::parse_json(&null_track_bpm).is_ok());
    let omitted_loop = null_track_bpm.replace(r#","loop":null"#, "");
    assert!(DjLinkEnvelope::parse_json(&omitted_loop).is_ok());
    let title_artist = RB_OUTPUT_TRACK_ACTIVE_FRAME.replace(
        r#""contentId":"content-2""#,
        r#""title":"Track 2","artist":"Artist 2""#,
    );
    assert!(DjLinkEnvelope::parse_json(&title_artist).is_ok());

    let ambiguous = RB_OUTPUT_TRACK_ACTIVE_FRAME.replace(
        r#""contentId":"content-2""#,
        r#""contentId":"content-2","title":"Track 2","artist":"Artist 2""#,
    );
    assert!(DjLinkEnvelope::parse_json(&ambiguous).is_err());
    let incomplete =
        RB_OUTPUT_TRACK_ACTIVE_FRAME.replace(r#""contentId":"content-2""#, r#""title":"Track 2""#);
    assert!(DjLinkEnvelope::parse_json(&incomplete).is_err());
    let master_field = RB_OUTPUT_TRACK_ACTIVE_FRAME
        .replace(r#""isPlaying":true"#, r#""isPlaying":true,"master":true"#);
    assert!(DjLinkEnvelope::parse_json(&master_field).is_err());

    for explicit_null in [
        RB_OUTPUT_TRACK_ACTIVE_FRAME.replace(r#""contentId":"content-2""#, r#""contentId":null"#),
        RB_OUTPUT_TRACK_ACTIVE_FRAME.replace(
            r#""contentId":"content-2","#,
            r#""contentId":"content-2","title":null,"#,
        ),
        RB_OUTPUT_TRACK_ACTIVE_FRAME.replace(
            r#""contentId":"content-2","#,
            r#""contentId":"content-2","artist":null,"#,
        ),
        title_artist.replace(r#""title":"Track 2""#, r#""title":null"#),
        title_artist.replace(r#""artist":"Artist 2""#, r#""artist":null"#),
        title_artist.replace(
            r#""title":"Track 2","#,
            r#""contentId":null,"title":"Track 2","#,
        ),
    ] {
        assert!(
            DjLinkEnvelope::parse_json(&explicit_null).is_err(),
            "generic track identity null must fail closed: {explicit_null}"
        );
    }
}

#[test]
fn generic_state_sync_requires_omitted_or_one_exact_owner_context() {
    let frame = r#"{
      "v":3,"type":"DJ_STATE_SYNC","agentId":"rb-output-dj-agent",
      "sessionId":"rb-session-1","sequence":18,"eventId":"generic-state-18",
      "payload":{"released":false,"ownerDeck":2,"ownerDeckId":"rekordbox-deck-2","activePlaySessionId":"play-session-2"}
    }"#;
    assert!(DjLinkEnvelope::parse_json(frame).is_ok());
    let mixed = frame.replace(r#""ownerDeck":2,"#, r#""masterDeck":2,"ownerDeck":2,"#);
    assert!(DjLinkEnvelope::parse_json(&mixed).is_err());
    let missing_owner_id = frame.replace(r#","ownerDeckId":"rekordbox-deck-2""#, "");
    assert!(DjLinkEnvelope::parse_json(&missing_owner_id).is_err());
    let null_owner_id = frame.replace(
        r#""ownerDeckId":"rekordbox-deck-2""#,
        r#""ownerDeckId":null"#,
    );
    assert!(DjLinkEnvelope::parse_json(&null_owner_id).is_err());
    for explicit_null in [
        frame.replace(r#""ownerDeck":2"#, r#""ownerDeck":null"#),
        frame.replace(
            r#""activePlaySessionId":"play-session-2""#,
            r#""activePlaySessionId":null"#,
        ),
        frame
            .replace(r#""ownerDeck":2"#, r#""ownerDeck":null"#)
            .replace(
                r#""ownerDeckId":"rekordbox-deck-2""#,
                r#""ownerDeckId":null"#,
            )
            .replace(
                r#""activePlaySessionId":"play-session-2""#,
                r#""activePlaySessionId":null"#,
            ),
    ] {
        assert!(
            DjLinkEnvelope::parse_json(&explicit_null).is_err(),
            "generic StateSync explicit null must fail closed: {explicit_null}"
        );
    }
    let owner_omitted = frame.replace(
        r#","ownerDeck":2,"ownerDeckId":"rekordbox-deck-2","activePlaySessionId":"play-session-2""#,
        "",
    );
    assert!(DjLinkEnvelope::parse_json(&owner_omitted).is_ok());
}

#[test]
fn generic_and_legacy_capability_fixtures_are_exact_and_cross_family_fields_fail_closed() {
    let hello = |capabilities: &[&str]| {
        format!(
            r#"{{"v":3,"type":"DJ_AGENT_HELLO","agentId":"rb-output-dj-agent","sessionId":"rb-session-1","sequence":1,"eventId":"hello","payload":{{"authToken":"0123456789abcdef0123456789abcdef","version":3,"capabilities":[{}]}}}}"#,
            capabilities
                .iter()
                .map(|capability| format!(r#""{capability}""#))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let generic = protocol::DJ_LINK_REQUIRED_CAPABILITIES;
    let legacy = protocol::DJ_LINK_LEGACY_REQUIRED_CAPABILITIES;
    assert!(DjLinkEnvelope::parse_json(&hello(&generic)).is_ok());
    assert!(DjLinkEnvelope::parse_json(&hello(&legacy)).is_ok());
    assert!(generic.contains(&"DJ_TRACK_ACTIVE"));
    assert!(generic.contains(&"DJ_TRACK_SYNC"));
    assert!(!generic.contains(&"DJ_MASTER_TRACK_ACTIVE"));
    assert!(legacy.contains(&"DJ_MASTER_TRACK_ACTIVE"));
    assert!(legacy.contains(&"DJ_MASTER_TRACK_SYNC"));
    assert!(!legacy.contains(&"DJ_TRACK_ACTIVE"));

    let mut mixed = generic.to_vec();
    mixed[0] = "DJ_MASTER_TRACK_ACTIVE";
    assert!(DjLinkEnvelope::parse_json(&hello(&mixed)).is_err());

    assert!(DjLinkEnvelope::parse_json(RB_OUTPUT_TRACK_ACTIVE_FRAME).is_ok());
    assert!(DjLinkEnvelope::parse_json(
        &RB_OUTPUT_TRACK_ACTIVE_FRAME.replace("DJ_TRACK_ACTIVE", "DJ_TRACK_SYNC"),
    )
    .is_ok());
    let generic_state = r#"{"v":3,"type":"DJ_STATE_SYNC","agentId":"rb-output-dj-agent","sessionId":"rb-session-1","sequence":2,"eventId":"generic-state","payload":{"released":false,"ownerDeck":2,"ownerDeckId":"rekordbox-deck-2","activePlaySessionId":"play-session-2"}}"#;
    assert!(DjLinkEnvelope::parse_json(generic_state).is_ok());
    let legacy_state = r#"{"v":3,"type":"DJ_STATE_SYNC","agentId":"rb-output-dj-agent","sessionId":"rb-session-1","sequence":2,"eventId":"legacy-state","payload":{"released":false,"masterDeck":2,"activePlaySessionId":"play-session-2"}}"#;
    assert!(DjLinkEnvelope::parse_json(legacy_state).is_ok());
    assert!(DjLinkEnvelope::parse_json(
        &generic_state.replace(r#""ownerDeck":2,"#, r#""masterDeck":2,"ownerDeck":2,"#,)
    )
    .is_err());
    assert!(DjLinkEnvelope::parse_json(&legacy_state.replace(
        r#""masterDeck":2,"#,
        r#""masterDeck":2,"ownerDeck":2,"ownerDeckId":"rekordbox-deck-2","#,
    ))
    .is_err());

    let legacy_master = r#"{"v":3,"type":"DJ_MASTER_TRACK_ACTIVE","agentId":"rb-output-dj-agent","sessionId":"rb-session-1","sequence":3,"eventId":"legacy-active","payload":{"deck":2,"deckId":"rekordbox-deck-2","masterDeckRevision":1,"contentId":"content-2","trackBpm":null,"positionAtSendSec":0,"effectiveBpm":120,"positionRevision":1,"sampleAgeMs":0,"isPlaying":true,"master":true,"startedAt":"2026-08-26T00:00:00Z","playSessionId":"play-session-2","loop":null}}"#;
    assert!(DjLinkEnvelope::parse_json(legacy_master).is_ok());
    assert!(DjLinkEnvelope::parse_json(
        &legacy_master.replace("DJ_MASTER_TRACK_ACTIVE", "DJ_MASTER_TRACK_SYNC"),
    )
    .is_ok());
    assert!(DjLinkEnvelope::parse_json(
        &legacy_master.replace("DJ_MASTER_TRACK_ACTIVE", "DJ_TRACK_ACTIVE"),
    )
    .is_err());
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

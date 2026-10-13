//! Seeded, deterministic hostile-input corpus for strict DJ Link v3 ingress.
//! The oracle is an independent strict decode that rejects a repeated object
//! key at every nesting level.  `DjLinkEnvelope::parse_json` must reject
//! every input that oracle rejects and every input over the frame bound, and
//! anything it accepts must survive a serialize/parse round trip unchanged.

use std::{
    collections::HashSet,
    fmt,
    panic::{catch_unwind, AssertUnwindSafe},
};

use protocol::{
    DjLinkEnvelope, DJ_LINK_AGENT_ID, DJ_LINK_MAX_FRAME_BYTES, DJ_LINK_REQUIRED_CAPABILITIES,
};
use serde::de::{Deserialize, Deserializer, Error, MapAccess, SeqAccess, Visitor};

const GENERATOR_SEED: u64 = 0x5344_435f_2026_0913;
const TARGET: u64 = 0x444a_4c49_4e4b_0008;
const CASES: usize = 2048;

/// serde_json's grammar, except that a repeated key in any object is an
/// error instead of last-value-wins.
struct StrictJson;

impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = StrictJson;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without repeated object keys")
    }

    fn visit_bool<E: Error>(self, _: bool) -> Result<StrictJson, E> {
        Ok(StrictJson)
    }

    fn visit_i64<E: Error>(self, _: i64) -> Result<StrictJson, E> {
        Ok(StrictJson)
    }

    fn visit_u64<E: Error>(self, _: u64) -> Result<StrictJson, E> {
        Ok(StrictJson)
    }

    fn visit_f64<E: Error>(self, _: f64) -> Result<StrictJson, E> {
        Ok(StrictJson)
    }

    fn visit_str<E: Error>(self, _: &str) -> Result<StrictJson, E> {
        Ok(StrictJson)
    }

    fn visit_unit<E: Error>(self) -> Result<StrictJson, E> {
        Ok(StrictJson)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<StrictJson, A::Error> {
        while seq.next_element::<StrictJson>()?.is_some() {}
        Ok(StrictJson)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<StrictJson, A::Error> {
        let mut keys = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key) {
                return Err(A::Error::custom("repeated object key"));
            }
            map.next_value::<StrictJson>()?;
        }
        Ok(StrictJson)
    }
}

struct CorpusRng(u64);

impl CorpusRng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        ((self.next_u64() >> 32) % bound as u64) as usize
    }

    fn pick<'a, T>(&mut self, values: &'a [T]) -> &'a T {
        &values[self.below(values.len())]
    }
}

fn valid_frames() -> Vec<String> {
    let hello = serde_json::json!({
        "v": 3,
        "type": "DJ_AGENT_HELLO",
        "agentId": DJ_LINK_AGENT_ID,
        "sessionId": "corpus-session",
        "sequence": 1,
        "eventId": "hello-1",
        "payload": {
            "authToken": "0123456789abcdef0123456789abcdef",
            "version": 3,
            "capabilities": DJ_LINK_REQUIRED_CAPABILITIES,
        },
    });
    vec![
        hello.to_string(),
        format!(
            r#"{{"v":3,"type":"DJ_HEARTBEAT","agentId":"{DJ_LINK_AGENT_ID}","sessionId":"corpus-session","sequence":2,"eventId":"heartbeat-2","payload":{{}}}}"#
        ),
        format!(
            r#"{{"v":3,"type":"DJ_LOOP_STATE","agentId":"{DJ_LINK_AGENT_ID}","sessionId":"corpus-session","sequence":17,"eventId":"loop-17","payload":{{"deck":1,"deckId":"rekordbox-deck-1","playSessionId":"play-session-1","loop":{{"active":true,"startBeat":32.0,"endBeat":32.015625,"lengthBeats":0.015625,"revision":9,"sampleAgeMs":0,"source":"rekordbox-hook-measured"}}}}}}"#
        ),
        format!(
            r#"{{"v":3,"type":"DJ_TRACK_ACTIVE","agentId":"{DJ_LINK_AGENT_ID}","sessionId":"corpus-session","sequence":19,"eventId":"track-active-19","payload":{{"deck":2,"deckId":"rekordbox-deck-2","contentId":"content-2","positionAtSendSec":12.5,"effectiveBpm":128.0,"positionRevision":4,"sampleAgeMs":0,"isPlaying":true,"startedAt":"2026-08-26T00:00:00Z","playSessionId":"play-session-2","loop":null}}}}"#
        ),
    ]
}

/// Repeats a key at a chosen nesting level, including through a `\u`
/// escape that only a decoding comparison recognises as the same key.
fn with_repeated_key(rng: &mut CorpusRng, frame: &str) -> String {
    let (anchor, insertion) = *rng.pick(&[
        ("{", r#""v":3,"#),
        ("{", "\"\\u0076\":3,"),
        ("{", r#""sequence":99,"#),
        ("{", r#""payload":{},"#),
        (r#""payload":{"#, r#""deck":1,"#),
        (r#""payload":{"#, "\"\\u0064eck\":1,"),
        (
            r#""payload":{"#,
            r#""authToken":"fedcba9876543210fedcba9876543210","#,
        ),
        (r#""loop":{"#, r#""active":false,"#),
    ]);
    match frame.find(anchor) {
        Some(at) => {
            let split = at + anchor.len();
            format!("{}{insertion}{}", &frame[..split], &frame[split..])
        }
        None => frame.to_string(),
    }
}

fn with_char_mutation(rng: &mut CorpusRng, frame: &str) -> String {
    let mut chars = frame.chars().collect::<Vec<_>>();
    for _ in 0..1 + rng.below(3) {
        let at = rng.below(chars.len() + 1);
        match rng.below(4) {
            0 => {
                let end = (at + 1 + rng.below(8)).min(chars.len());
                chars.drain(at..end);
            }
            1 => {
                let token = *rng.pick(&[
                    "\"", ",", "{", "}", "[", "]", ":", "\\", "\u{0}", "1e999", "-0", "NaN",
                    "\\uD800", "\\u0000", " ", "null", "\u{feff}",
                ]);
                chars.splice(at..at, token.chars());
            }
            2 if at < chars.len() => {
                chars[at] = *rng.pick(&['0', '9', 'x', '"', '\u{7f}', 'é']);
            }
            _ => chars.truncate(at),
        }
    }
    chars.into_iter().collect()
}

#[test]
fn dj_link_parse_json_rejects_whatever_a_strict_duplicate_key_decode_rejects() {
    let mut rng = CorpusRng(GENERATOR_SEED ^ TARGET);
    let frames = valid_frames();
    let padded = |frame: &str, total: usize| {
        let mut text = frame.to_string();
        text.push_str(&" ".repeat(total.saturating_sub(frame.len())));
        text
    };
    let mut fixed = vec![
        String::new(),
        "{}".to_string(),
        "null".to_string(),
        padded(&frames[1], DJ_LINK_MAX_FRAME_BYTES),
        padded(&frames[1], DJ_LINK_MAX_FRAME_BYTES + 1),
        format!("{}{}", "[".repeat(200), "]".repeat(200)),
    ];
    fixed.extend(frames.iter().cloned());

    let (mut accepted, mut rejected, mut repeated_keys, mut oversized) = (0, 0, 0, 0);
    for index in 0..CASES {
        let text = if index < fixed.len() {
            fixed[index].clone()
        } else {
            let frame = rng.pick(&frames).clone();
            match rng.below(10) {
                0..=2 => frame,
                3..=5 => with_repeated_key(&mut rng, &frame),
                6..=8 => with_char_mutation(&mut rng, &frame),
                _ => padded(&frame, DJ_LINK_MAX_FRAME_BYTES + 1 + rng.below(64)),
            }
        };
        let parsed = catch_unwind(AssertUnwindSafe(|| DjLinkEnvelope::parse_json(&text)))
            .unwrap_or_else(|_| panic!("DJ Link case {index} panicked"));
        let strict_rejects = serde_json::from_str::<StrictJson>(&text).is_err();
        if strict_rejects && serde_json::from_str::<serde_json::Value>(&text).is_ok() {
            repeated_keys += 1;
        }
        if text.len() > DJ_LINK_MAX_FRAME_BYTES {
            oversized += 1;
            assert!(parsed.is_err(), "case {index}: oversized frame accepted");
        }
        if strict_rejects {
            assert!(
                parsed.is_err(),
                "case {index}: accepted input the strict decode rejects: {text}"
            );
        }
        match parsed {
            Ok(envelope) => {
                accepted += 1;
                let reserialized = serde_json::to_string(&envelope).unwrap();
                assert_eq!(
                    DjLinkEnvelope::parse_json(&reserialized).ok(),
                    Some(envelope),
                    "case {index}: accepted envelope does not round-trip"
                );
            }
            Err(_) => rejected += 1,
        }
    }
    assert!(accepted > 0, "corpus must exercise accepted frames");
    assert!(rejected > 0, "corpus must exercise rejection");
    assert!(repeated_keys > 0, "corpus must exercise repeated keys");
    assert!(oversized > 0, "corpus must exercise the frame bound");
    eprintln!(
        "DJ Link hostile corpus: seed={GENERATOR_SEED:#x}^{TARGET:#x}, {CASES} cases, {accepted} accepted, {rejected} rejected, {repeated_keys} repeated-key, {oversized} oversized"
    );
}

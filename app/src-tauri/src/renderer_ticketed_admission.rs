use serde_json::Value;

/// Renderer-ticketed commands with a typed request parameter receive the
/// transaction ticket inside the command's exact `request` envelope.  Keep
/// this list separate from the flat legacy routes: accepting either shape
/// here would let an outer admission check a different object than the
/// generated command handler dispatches.
const NESTED_RENDERER_TICKETED_ROUTES: [&str; 3] = [
    "move_cue_between_scene_banks_batch",
    "set_fixture_transform",
    "set_fixture_transforms",
];

pub(crate) fn renderer_ticketed_request_payload<'a>(
    command: &str,
    payload: &'a Value,
) -> Result<&'a Value, String> {
    if !NESTED_RENDERER_TICKETED_ROUTES.contains(&command) {
        return Ok(payload);
    }

    let object = payload.as_object().ok_or_else(|| {
        format!(
            "Tauri project mutation '{command}' requires an exact nested request envelope"
        )
    })?;
    if object.len() != 1 || !object.contains_key("request") {
        return Err(format!(
            "Tauri project mutation '{command}' requires an exact nested request envelope"
        ));
    }
    let request = object
        .get("request")
        .expect("request key was checked above");
    if !request.is_object() {
        return Err(format!(
            "Tauri project mutation '{command}' requires nested object field 'request'"
        ));
    }
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_renderer_routes_extract_only_the_exact_request_envelope() {
        for command in [
            "set_fixture_transform",
            "set_fixture_transforms",
            "move_cue_between_scene_banks_batch",
        ] {
            let payload = serde_json::json!({
                "request": {
                    "projectTransactionId": 19,
                    "expectedEpoch": 7,
                    "ownerId": "main:fixture-test"
                }
            });
            let request = renderer_ticketed_request_payload(command, &payload).unwrap();
            assert_eq!(
                request
                    .get("projectTransactionId")
                    .and_then(Value::as_u64),
                Some(19)
            );
            assert!(request.get("request").is_none());
        }
    }

    #[test]
    fn typed_renderer_routes_reject_flat_missing_and_ambiguous_envelopes() {
        for command in [
            "set_fixture_transform",
            "set_fixture_transforms",
            "move_cue_between_scene_banks_batch",
        ] {
            for payload in [
                serde_json::json!({
                    "projectTransactionId": 19,
                    "expectedEpoch": 7,
                    "ownerId": "main:fixture-test"
                }),
                serde_json::json!({}),
                serde_json::json!({
                    "request": {
                        "projectTransactionId": 19,
                        "expectedEpoch": 7,
                        "ownerId": "main:fixture-test"
                    },
                    "projectTransactionId": 19
                }),
            ] {
                assert!(
                    renderer_ticketed_request_payload(command, &payload).is_err(),
                    "{command} must not fall back to a flat or ambiguous payload"
                );
            }
        }
    }

    #[test]
    fn typed_renderer_routes_reject_non_object_requests() {
        for command in [
            "set_fixture_transform",
            "set_fixture_transforms",
            "move_cue_between_scene_banks_batch",
        ] {
            for request in [serde_json::Value::Null, serde_json::json!([])] {
                let payload = serde_json::json!({ "request": request });
                assert!(renderer_ticketed_request_payload(command, &payload).is_err());
            }
        }
    }

    #[test]
    fn flat_renderer_routes_keep_their_existing_payload_shape() {
        let payload = serde_json::json!({
            "projectTransactionId": 19,
            "expectedEpoch": 7,
            "ownerId": "main:fixture-test"
        });
        let preserved = renderer_ticketed_request_payload("set_attribute", &payload).unwrap();
        assert_eq!(preserved, &payload);
    }

    #[test]
    fn flat_business_request_routes_keep_their_ticketed_payload_shape() {
        let payload = serde_json::json!({
            "request": { "items": [11, 12] },
            "projectTransactionId": 19,
            "expectedEpoch": 7,
            "ownerId": "main:timeline-test"
        });
        let preserved =
            renderer_ticketed_request_payload("snap_timeline_items", &payload).unwrap();
        assert_eq!(preserved, &payload);
    }
}

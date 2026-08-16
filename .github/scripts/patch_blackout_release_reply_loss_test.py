from pathlib import Path
import runpy


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


runpy.run_path(".github/scripts/patch_blackout_release_receipt_expiry.py", run_name="__main__")
runtime = Path("app/src-tauri/src/control_plane_runtime.rs")

anchor = '''    fn test_binding(principal: &str, window_label: &str, owner_incarnation: u64) -> CallerBinding {
'''
insert = '''    #[test]
    fn blackout_release_exact_reply_loss_reuses_terminal_and_tombstones_shape_conflicts() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-release", "main", 61);
        let request = OutputControlCommandRequestV1 {
            operation_id: "syndocal.output.blackout.release.v1".to_string(),
            request_id: 90_101,
            expected_fence: test_output_control_fence(),
            consent_token: "AAAAAAAAAAAAAAAAAAAAAA".to_string(),
            action: OutputControlActionV1::ReleaseBlackout,
        };
        request.validate().unwrap();
        let shape = hex_sha256(&request.canonical_shape_bytes().unwrap());
        let argument_fingerprint = hex_sha256(&request.argument_fingerprint_bytes().unwrap());
        let key = output_control_receipt_key(&request, &binding);

        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now),
            OutputControlLaneReservation::Lane(_)
        ));
        let (inflight, audit_sequence) = state
            .admit_and_audit_output_control(
                &binding,
                &key,
                &shape,
                &argument_fingerprint,
                now,
            )
            .unwrap();
        state.finish_output_control_inflight(&inflight);

        let response = OutputControlResponseV1::Receipt(OutputControlReceiptV1 {
            operation_id: request.operation_id.clone(),
            request_id: request.request_id,
            shape_sha256: shape.clone(),
            argument_fingerprint,
            audit_sequence,
            fence_before: request.expected_fence.clone(),
            fence_after: blackout_release_receipt_fence(&request.expected_fence, true).unwrap(),
            outcome: OutputControlReceiptOutcomeV1::Applied,
        });
        response.validate().unwrap();
        state
            .store_output_control_terminal(
                key.clone(),
                shape.clone(),
                response.clone(),
                now,
            )
            .unwrap();

        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now),
            OutputControlLaneReservation::Terminal(actual) if actual == response
        ));

        let mut different_token = request.clone();
        different_token.consent_token = "AQEBAQEBAQEBAQEBAQEBAQ".to_string();
        different_token.validate().unwrap();
        let different_token_key = output_control_receipt_key(&different_token, &binding);
        assert_eq!(different_token_key, key);
        let different_token_shape =
            hex_sha256(&different_token.canonical_shape_bytes().unwrap());
        assert_ne!(different_token_shape, shape);
        assert!(matches!(
            state.reserve_output_control_lane(&different_token_key, &different_token_shape, now),
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
        ));

        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now + RECEIPT_TTL),
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::ReceiptExpired)
        ));
    }

'''
replace_exact(
    runtime,
    anchor,
    insert + anchor,
    "focused Blackout Release reply-loss test",
)

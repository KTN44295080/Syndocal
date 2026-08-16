from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


runtime = Path("app/src-tauri/src/control_plane_runtime.rs")
app = Path("app/src/App.tsx")

# The strict Release wire renames existing Tauri/frontend sources in place.
# The one count correction performed by patch_blackout_release_receipt.py is
# legitimate: the Engine enum already contains both SafetyBlackout published
# variants, so the generated engine inventory is 250 even though the app-side
# registry assertion was stale at 249. Do not fabricate any Tauri +4 change.

# A Release receipt must describe exactly the Release commit boundary. Re-read
# of current project/output state after the engine ACK can accidentally absorb
# an unrelated concurrent mutation into the terminal receipt. Derive the
# Release post-fence from its admitted start fence instead; Arm/Takeover retain
# the generic snapshot path until their own reviewed AI3 slices.
replace_exact(
    runtime,
    '''    let (applied, fence_after) = match operation_result {
        Ok(applied) => {
            let fence_after = match current_output_control_fence(state, &request.expected_fence) {
                Ok(fence) => fence,
                Err(_) => {
                    state
                        .runtime_control_plane
                        .finish_output_control_inflight(&inflight);
                    let response =
                        output_control_rejection(&request, OutputControlErrorCodeV1::Internal);
                    let _ = state.runtime_control_plane.store_output_control_terminal(
                        key,
                        shape_sha256,
                        response.clone(),
                        Instant::now(),
                    );
                    return response;
                }
            };
            (applied, fence_after)
        }
        Err(_) => {
            state
                .runtime_control_plane
                .finish_output_control_inflight(&inflight);
            let response =
                output_control_rejection(&request, OutputControlErrorCodeV1::PublicationFailed);
            let _ = state.runtime_control_plane.store_output_control_terminal(
                key,
                shape_sha256,
                response.clone(),
                Instant::now(),
            );
            return response;
        }
    };
''',
    '''    let (applied, fence_after) = match operation_result {
        Ok(applied) => {
            let fence_after = if matches!(&request.action, OutputControlActionV1::ReleaseBlackout) {
                blackout_release_receipt_fence(&request.expected_fence, applied)
            } else {
                current_output_control_fence(state, &request.expected_fence)
            };
            let fence_after = match fence_after {
                Ok(fence) => fence,
                Err(_) => {
                    state
                        .runtime_control_plane
                        .finish_output_control_inflight(&inflight);
                    let response =
                        output_control_rejection(&request, OutputControlErrorCodeV1::Internal);
                    let _ = state.runtime_control_plane.store_output_control_terminal(
                        key,
                        shape_sha256,
                        response.clone(),
                        Instant::now(),
                    );
                    return response;
                }
            };
            (applied, fence_after)
        }
        Err(error) => {
            state
                .runtime_control_plane
                .finish_output_control_inflight(&inflight);
            let error_code = if matches!(&request.action, OutputControlActionV1::ReleaseBlackout)
                && (error.contains("stale") || error.contains("expired"))
            {
                OutputControlErrorCodeV1::StaleFence
            } else {
                OutputControlErrorCodeV1::PublicationFailed
            };
            let response = output_control_rejection(&request, error_code);
            let _ = state.runtime_control_plane.store_output_control_terminal(
                key,
                shape_sha256,
                response.clone(),
                Instant::now(),
            );
            return response;
        }
    };
''',
    "deterministic blackout release terminal fence",
)

replace_exact(
    runtime,
    '''fn current_output_control_fence(
    state: &AppState,
    basis: &OutputControlFenceV1,
) -> Result<OutputControlFenceV1, String> {
''',
    '''fn blackout_release_receipt_fence(
    basis: &OutputControlFenceV1,
    applied: bool,
) -> Result<OutputControlFenceV1, String> {
    if !applied {
        return Ok(basis.clone());
    }
    let mut fence = basis.clone();
    if fence.safety_blackout_generation < MAX_SAFE_JAVASCRIPT_INTEGER {
        fence.safety_blackout_generation += 1;
    } else if fence.safety_blackout_epoch < MAX_SAFE_JAVASCRIPT_INTEGER {
        fence.safety_blackout_epoch += 1;
        fence.safety_blackout_generation = 1;
    } else {
        return Err("Safety blackout authority is exhausted".to_string());
    }
    fence
        .validate()
        .map_err(|_| "Blackout Release result fence is invalid".to_string())?;
    Ok(fence)
}

fn current_output_control_fence(
    state: &AppState,
    basis: &OutputControlFenceV1,
) -> Result<OutputControlFenceV1, String> {
''',
    "blackout release terminal fence helper",
)

replace_exact(
    runtime,
    '''    fn test_binding(principal: &str, window_label: &str, owner_incarnation: u64) -> CallerBinding {
''',
    '''    fn test_output_control_fence() -> OutputControlFenceV1 {
        OutputControlFenceV1 {
            process_incarnation: 1,
            session_incarnation: 2,
            project_epoch: 3,
            project_revision: 4,
            project_checkpoint_hash: "a".repeat(64),
            project_publication_generation: 5,
            output_epoch: 6,
            output_generation: 7,
            safety_blackout_epoch: 8,
            safety_blackout_generation: 9,
        }
    }

    #[test]
    fn blackout_release_receipt_fence_is_exact_and_rolls_safely() {
        let basis = test_output_control_fence();
        assert_eq!(blackout_release_receipt_fence(&basis, false).unwrap(), basis);

        let mut expected = basis.clone();
        expected.safety_blackout_generation += 1;
        assert_eq!(
            blackout_release_receipt_fence(&basis, true).unwrap(),
            expected
        );

        let mut rollover = basis.clone();
        rollover.safety_blackout_epoch = 17;
        rollover.safety_blackout_generation = MAX_SAFE_JAVASCRIPT_INTEGER;
        let mut rollover_expected = rollover.clone();
        rollover_expected.safety_blackout_epoch = 18;
        rollover_expected.safety_blackout_generation = 1;
        assert_eq!(
            blackout_release_receipt_fence(&rollover, true).unwrap(),
            rollover_expected
        );

        let mut exhausted = basis;
        exhausted.safety_blackout_epoch = MAX_SAFE_JAVASCRIPT_INTEGER;
        exhausted.safety_blackout_generation = MAX_SAFE_JAVASCRIPT_INTEGER;
        assert!(blackout_release_receipt_fence(&exhausted, true).is_err());
    }

    fn test_binding(principal: &str, window_label: &str, owner_incarnation: u64) -> CallerBinding {
''',
    "blackout release terminal fence focused tests",
)

# All-blackout clear must not provide a second local UI route around R4. The
# safer-direction engage remains available; only the energizing clear is
# redirected to the physical Release control.
replace_exact(
    app,
    '''  const setAllBlackout = async (enabled: boolean) => {
    try {
      await invoke("set_all_blackout", { enabled });
      setMessage(enabled ? "All blackout enabled." : "All blackout cleared.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };
''',
    '''  const setAllBlackout = async (enabled: boolean) => {
    try {
      if (!enabled) {
        setMessage("All blackout clear requires physical DMX Blackout Release in Runtime controls.");
        return;
      }
      await invoke("set_all_blackout", { enabled: true });
      setMessage("All blackout enabled.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };
''',
    "block legacy All Clear release route",
)

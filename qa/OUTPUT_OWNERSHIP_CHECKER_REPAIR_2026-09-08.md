# Output ownership checker repair — 2026-09-08

## Scope

This checkpoint repairs an existing static checker boundary after the current
main source moved `stop_video_output_recording_runtime` into the dedicated
`video_recording_runtime.rs` module. The product implementation is unchanged;
the checker now delimits the existing `with_output_resource_creation_lease`
helper with its adjacent cleanup-aware helper in `main.rs`.

- Source base: `730aa92ce903cd398d46e3e5e58396e01f0b84b7`
- Product source changes: none
- Changed file: `app/scripts/check-output-ownership.mjs`
- Prior real-file thumbnail missing → UI Retry → recovery test: not rerun

## Reproduced failure and repair

Before the repair, the checker failed because it searched for the retired
marker `fn stop_video_output_recording_runtime` while slicing the display
resource-creation helper. The failure was a stale checker/source boundary, not
a product runtime failure. The assertion itself was retained: the checker
still requires admission before native creation, publication before resource
retirement, and lease retirement after the resource lifecycle.

The end marker now uses the current neighboring
`fn with_output_resource_creation_lease_with_cleanup` helper. No assertion was
removed, inverted, or relaxed.

## Verification

| Command | Result |
| --- | --- |
| `node app/scripts/check-output-ownership.mjs` | PASS — output ownership static contract |
| `git diff --check` | PASS |

No app process, physical output, device, or external client was started.

## Boundary

This checkpoint repairs only the checker’s source boundary. It does not close
the F2 output-ownership ledger row, prove physical Lighting/Video/Both/Standby
ownership, or change ASIO/NDI/DMX behavior. External hardware, Mac real-device,
signing, publication, and product-wide acceptance remain unclaimed.

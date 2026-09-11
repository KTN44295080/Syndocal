# F2 output-ownership audit — 2026-09-08

## Scope

This is a bounded audit of the existing local output-ownership and teardown
boundaries. It does not enable physical output, merge an old platform
candidate, or close the full F2 ledger row.

- Source base: `5683bd55d977363f96f3201a5d4ebdb887a145fb`
- Working tree before this document: clean; `main` matched `origin/main`
- Product source changes: none
- Prior real-file thumbnail missing → UI Retry → recovery test: not rerun

## Existing implementation checked

The current ownership path keeps output role changes behind the local
OutputControl/lease boundary. Engine ownership gates fence in-flight work,
retain teardown leases until cleanup acknowledgement, reject stale role/epoch
retries, and require an explicit activation phase for re-arm. The Syndocal
adapter keeps atomic Both candidates, Standby publication conditions,
project-swap retirement, native display-window cleanup, and safety-blackout
authority separate from authored project state.

The existing static contracts also retain the fail-closed distinction between
software ownership evidence and physical delivery. No output route was enabled
by this audit.

## Focused verification

The following static contracts all passed:

| Check | Result |
| --- | --- |
| `node app/scripts/check-output-ownership.mjs` | PASS |
| `node app/scripts/check-output-control-runtime.mjs` | PASS |
| `node app/scripts/check-standby-sync-output-lease-ui.mjs` | PASS |
| `node app/scripts/check-video-output-routing-runtime.mjs` | PASS |
| `node app/scripts/check-video-output-window-runtime.mjs` | PASS |
| `node app/scripts/check-video-output-window-observation.mjs` | PASS |

The Rust commands ran after `vcvars64.bat -vcvars_ver=14.44`, with the exact
Build Tools MSVC `14.44.35207` x64 linker pinned in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` and returned first by
`where.exe link.exe`.

| Package/filter | Result |
| --- | --- |
| `cargo test -p engine --release --locked output_ownership_gate -- --test-threads=1` | PASS — 3 passed |
| `cargo test -p engine --release --locked output_ownership_teardown_lease -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p engine --release --locked output_worker_failure_fence -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p engine --release --locked output_resource_creation_lease -- --test-threads=1` | PASS — 2 passed |
| `cargo test -p engine --release --locked project_swap_disarmed -- --test-threads=1` | PASS — 1 passed |
| Eight targeted `cargo test -p syndocal --release --locked ...` filters | PASS — 8 passed |

The Syndocal cases cover the candidate commit boundary, atomic Both arm
rollback, Standby role/takeover admission, completed-deny Standby publication,
project replacement blocked by output retirement failure, partial native
creation cleanup, destroyed-window lease failure, and stale native display
safety authority. No app process, physical output, device, or external client
was started.

## Remaining boundary

`F2-OUTPUT-OWNERSHIP-001` remains `Open` in
`qa/SYNDOCAL_COMPLETION_LEDGER.json`. This checkpoint proves only the audited
local software boundaries. It does not prove real Lighting/Video/Both/Standby
ownership across DMX, NDI, Spout/Syphon, displays, SDK resources, Take Over,
or venue teardown ACKs. ASIO/NDI/DMX physical acceptance, ShowClock decisions,
Mac real-device, signing, publication, and product-wide acceptance remain
unclaimed.

No assertion was weakened and no runtime or physical-output behavior changed.

## Current-main software revalidation

The bounded F2 checks were repeated on current `main` at source HEAD
`9389ec5e74a6457b2c93beb197933953b37ced22`. No ownership policy, output route,
or physical-output state was changed.

The six static contracts all passed:

```text
check-output-ownership.mjs
check-output-control-runtime.mjs
check-standby-sync-output-lease-ui.mjs
check-video-output-routing-runtime.mjs
check-video-output-window-runtime.mjs
check-video-output-window-observation.mjs
```

The exact MSVC 14.44.35207 x64 linker pin and `where.exe link.exe`
first-match check were used for the native tests. Results were:

```text
cargo test -p engine --release --locked output_ownership -- --test-threads=1
test result: ok. 9 passed; 0 failed; 0 ignored

Syndocal targeted ownership tests: 11 passed; 0 failed; 0 ignored
```

The targeted Syndocal set included the existing atomic role/persistence,
Standby, stale-window, project-load, fenced-creation, and injected-Spout
cases. Two historical filter labels in the older audit (`native_video_output_window_close_attempts_all_labels`
and `warm_standby_project_disarms_every_output`) matched zero tests on current
source and were explicitly excluded from the pass count; the actual current
test names were resolved and run instead. A zero-test filter is not evidence.

This current-main software evidence does not close `F2-OUTPUT-OWNERSHIP-001`:
real Lighting/Video/Both/Standby devices, NDI/Spout/display teardown ACKs,
Take Over hardware behavior, and venue acceptance remain unclaimed.

## Current-main software revalidation — 2026-09-10

The bounded F2 checks were rerun against current `main` source HEAD
`0589258c1ea3145fc914483d3d80a1e6ed2b1cff`. No output route, ownership
policy, or physical-output state changed.

The static contracts all passed:

```text
pnpm.cmd --dir app run check:output-ownership
pnpm.cmd --dir app run check:output-control-runtime
pnpm.cmd --dir app run check:video-output-routing-runtime
pnpm.cmd --dir app run check:video-output-window-runtime
pnpm.cmd --dir app run check:video-output-window-observation
```

`check:output-control-runtime` also passed the Standby Sync output-lease UI
contract. With the exact Build Tools MSVC `14.44.35207` x64 linker pinned
after `vcvars64.bat -vcvars_ver=14.44` and returned first by
`where.exe link.exe`, the focused native tests were:

| Package/filter | Result |
| --- | --- |
| `cargo test -p engine --release --locked output_ownership_gate -- --test-threads=1` | PASS — 3 passed |
| `cargo test -p engine --release --locked output_ownership_teardown_lease -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p engine --release --locked output_worker_failure_fence -- --test-threads=1` | PASS — 1 passed |
| `cargo test -p engine --release --locked output_resource_creation_lease -- --test-threads=1` | PASS — 2 passed |
| `cargo test -p engine --release --locked project_swap_disarmed -- --test-threads=1` | PASS — 1 passed |
| Current targeted Syndocal ownership filters | PASS — 11 passed, 0 failed, 0 ignored |

The Syndocal set covered atomic Both rollback, Standby admission/publication,
project replacement retirement fencing, partial native creation cleanup,
destroyed-window lease failure, stale-window labeling, authored-project
preservation, constructor admission, output-lease commit separation, and
injected-Spout authority. No zero-test historical filter was counted.

This remains local software evidence only. Physical Lighting/Video/Both/
Standby ownership, DMX/NDI/Spout/display teardown acknowledgements, Take Over
hardware behavior, venue acceptance, Mac, signing, publication, and thumbnail
file-move recovery remain unclaimed.

## Current-main ownership and teardown revalidation — 2026-09-11

The bounded F2 software checks were rerun after the current snapshot/output
reader and managed-terminal changes at source HEAD
`cba6ca7b949d28d318f02bd38c779e49850d0b5b`. Evidence is preserved under
`target/qa/f2-current-main-20260911-01/`. No application process or physical
output was started.

The five static contracts passed:

```text
check-output-ownership.mjs
check-output-control-runtime.mjs
check-video-output-routing-runtime.mjs
check-video-output-window-runtime.mjs
check-video-output-window-observation.mjs
```

With MSVC 14.44.35207 initialized by `vcvars64.bat -vcvars_ver=14.44`, and
the pinned linker first in `where.exe link.exe`, the focused native test set
passed with zero failures:

```text
engine output_ownership                  9 passed
engine output_worker_failure_fence       1 passed
engine output_resource_creation_lease    2 passed
engine project_swap_disarmed             1 passed
Syndocal show_spout_managed_terminal     2 passed
Syndocal Standby/output-role             2 passed
Syndocal project retirement/display      5 passed
Syndocal narrow display/Spout readers    2 passed
```

This current-main software evidence covers role gating, teardown leases,
failure fences, stale epoch/role rejection, explicit Standby re-arm,
managed-terminal replay, project retirement before publication, pending
display admission, and the narrow snapshot readers. It does not close
`F2-OUTPUT-OWNERSHIP-001`: real Lighting/Video/Both/Standby resources,
DMX/NDI/Spout/display teardown acknowledgements, Take Over hardware,
physical output, venue acceptance, Mac, signing, publication, and thumbnail
file-move recovery remain unclaimed.

## Current-main ownership and retirement revalidation — 2026-09-12

The bounded F2 software/native checks were rerun against current `main` at
source HEAD `278b5101dc5d0e74595b0e9b5e7ba2e99045b18d`; `origin/main` matched
before the run. No ownership policy or physical-output state was changed.
The documented MSVC 14.44.35207 x64 linker was pinned and returned first by
`where.exe link.exe`.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:output-ownership` | PASS |
| `pnpm.cmd --dir app run check:output-control-runtime` | PASS — output commands, strict receipts, fail-closed query, and Standby Sync UI |
| `pnpm.cmd --dir app run check:video-output-routing-runtime` | PASS — R4 contract |
| `pnpm.cmd --dir app run check:video-output-window-runtime` | PASS — exact-Both recovery, receipt rejection, singleflight, incarnation reducer, legacy invoke count 0 |
| `pnpm.cmd --dir app run check:video-output-window-observation` | PASS |
| `cargo test -p engine --release --locked output_ownership -- --test-threads=1` | PASS — 9 passed, 0 failed, 0 ignored, 1074 filtered out |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 output_ownership -- --test-threads=1` | PASS — 5 passed, 0 failed, 0 ignored, 1861 filtered out |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 show_spout_managed_terminal -- --test-threads=1` | PASS — 2 passed, 0 failed, 0 ignored, 1864 filtered out |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 project_retirement_spout -- --test-threads=1` | PASS — 5 passed, 0 failed, 0 ignored, 1861 filtered out |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 project_retirement_callback_failure -- --test-threads=1` | PASS — 1 passed, 0 failed, 0 ignored, 1865 filtered out |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 show_spout_disabled_activation -- --test-threads=1` | PASS — 2 passed, 0 failed, 0 ignored, 1864 filtered out |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 managed_display_window_projection -- --test-threads=1` | PASS — 1 passed, 0 failed, 0 ignored, 1865 filtered out |

The historical `project_swap_disarmed` filter was also checked and matched
zero tests; it was explicitly excluded from the pass count and no assertion
or filter was weakened. The current passing cases retain role/lease gating,
worker teardown and failure harvest ordering, managed terminal replay and
invalid-fence rejection, project retirement before publication, disabled
activation compensation, exact project display projection, and no accidental
Add Display path.

This remains local software/native evidence only. `F2-OUTPUT-OWNERSHIP-001`
stays Open for real Lighting/Video/Both/Standby resources, DMX/NDI/Spout/
display teardown acknowledgements, Take Over hardware, physical output,
venue acceptance, Mac, signing, publication, and thumbnail file-move
recovery.

## Current-main output software gate — 2026-09-12

The same bounded F2 checks were rerun at current source HEAD
`48d57f1d86188ec097006387009df3a0aedefb7e`. No product output policy or
physical resource was changed. All six static contracts passed. The exact
MSVC 14.44.35207 x64 linker was initialized and returned first by
`where.exe link.exe` for every Rust invocation.

| Check group | Result |
| --- | --- |
| output ownership/control, safety blackout, video routing/window/observation | PASS — 6 static gates |
| Engine `output_ownership` | PASS — 9 passed, 0 failed, 0 ignored |
| Syndocal `output_ownership` | PASS — 5 passed, 0 failed, 0 ignored |
| Syndocal `show_spout_managed_terminal` | PASS — 2 passed, 0 failed, 0 ignored |
| Syndocal `project_retirement_spout` | PASS — 5 passed, 0 failed, 0 ignored |
| Syndocal `project_retirement_callback_failure` | PASS — 1 passed, 0 failed, 0 ignored |
| Syndocal `show_spout_disabled_activation` | PASS — 2 passed, 0 failed, 0 ignored |
| Syndocal `managed_display_window_projection` | PASS — 1 passed, 0 failed, 0 ignored |

The automated proof is recorded as passing for `COV-OUTPUT-LOCAL-001`.
This remains software/native focused evidence only. Physical DMX/NDI/Spout/
display teardown acknowledgements, Take Over hardware, venue acceptance, Mac,
signing, publication, and product-wide completion remain open; the row stays
`In progress`.

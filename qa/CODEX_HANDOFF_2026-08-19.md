# Syndocal Codex handoff — 2026-08-19

## Current checkpoint

- Branch: `codex/syndocal-v1.2`
- Current completion authority: `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md`. The active product train is `1.2.0-alpha.1`.
- Baseline HEAD at takeover: `df7e335c14fe82bb534fbd8867dcd431777e1522`
- Verified local OutputControl R4 implementation checkpoint: `105c522e795ad021776649bff07d2ecf77bb0d0f` (`feat: route local output controls through R4`). The verified documentation follow-up is `94f4259eb982b4ecfa7b6ea3c645bbf8bd0c64ac`; both were pushed successfully to `origin/codex/syndocal-v1.0`.
- The owner-incarnation output-lease acceptance contract was fixed in docs-only checkpoint `7b411c4e5a7b26ddf9ae91cea4fa8181daedfa24` (`docs: define AI3 output lease contract`). The later pure transition-core checkpoint is `2b889a753a6f55fc308ff5d82509138cfffd41a0` (`feat: add pure output lease transition core`); it is intentionally not AppState/runtime integration.
- The bounded Windows W1 warning ratchet is implemented at `3ee303f4ce7fed897e4d2473ddf80b4335b20591` (`feat: enforce Windows warning ratchet`). Its docs-only normal-mode proof is `33f58df2f69c45892f2089037cbf00ed0aab3e56` (`docs: pin Windows warning ratchet checkpoint`). W0 remains incomplete because nine required configurations are still pending. First-party warnings remain tracked debt and must reach zero before beta/RC; do not suppress them globally.
- The completion-flow/version checkpoint advances every checked product metadata surface and all first-party Cargo packages to `1.2.0-alpha.1`. `pnpm --dir app run check:release`, `cargo check -p protocol`, locked Cargo metadata, Markdown local-link validation, Rust format, and `git diff --check` passed. The fresh native build used the same full VS/FFmpeg/libclang/Node command recorded below and completed in 2m50s after exact checkout PID `9436` was stopped and the remaining exact count reached zero. The rebuilt executable reports ProductVersion/FileVersion `1.2.0-alpha.1`; it was launched as PID `35944` with one exact process, one responsive `Syndocal` window, and `IsZoomed=True`.
- The implementation/documentation commit for that checkpoint is `4e18b0ff134953c7312483d896c0404de115849c` (`docs: define completion flow and advance version`). This follow-up removes the two Markdown hard-break trailing spaces caught by the cached diff check and pins the implementation hash.
- That build also records the warning debt requested for cleanup: Engine 9 warnings, Syndocal release target 58 warnings, and the Vite oversized-chunk warning. This is the provisional default-release baseline only, not a warning allowlist or the complete W0 feature/platform inventory.
- The previous Codex reached its context/token limit while continuing AI3. Treat Timeline Transport and the canonical Timeline Follow Abort tranche as verified. AI3 has now been audited and remains incomplete in all five roadmap categories. The detailed audit matrix and ordered gaps are recorded in `qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md`.
- The replacement Syndocal × rekordbox × Stream Deck Pedal requirement is normative in `qa/REKORDBOX_STREAM_DECK_PEDAL_ACCEPTANCE.md`. `Seraf0-org/rekordbox-DJ-Link-ForPCDJ` is the sole DJ-PC Agent and owns Pedal/global-hotkey input plus all rekordbox MIDI, Filter, Stop, and reset behavior. Syndocal only receives authenticated `DJ_*` semantic events over the reused Web Remote listener, owns `.sdc` Track-to-Timeline mappings, and applies absolute Loop/Release show actions. The peer implementation and wired physical acceptance remain separate evidence and are not yet complete.
- The 2026-08-20 P2/warning implementation checkpoint is `a228ba5e492841618ce0262038c965b5519d1ddd` (`fix: close takeover proof and reduce native warnings`). It closes the bounded Take Over proof gaps and removes honest production warning debt without suppression. It does not complete W0, AI3, physical video-output acceptance, or the new rekordbox/Pedal requirement.

## W0/W1 warning-ratchet checkpoint

The H1 implementation commit is
`3ee303f4ce7fed897e4d2473ddf80b4335b20591`. It pins Rust/Cargo `1.97.1`,
Node `22.22.1`, pnpm `10.9.0`, a schema-versioned warning inventory, and a
fail-closed Cargo JSON runner. Independent adversarial review of the exact
11-file hash set concluded P0 0 and P1 0 for this bounded Windows W1 gate.

The enforced configurations and exact first-party baselines are:

- Windows workspace default all targets: 83 occurrences, 67 identities,
  artifact coverage 11/11;
- Windows workspace release all targets: 83 occurrences, 67 identities,
  artifact coverage 11/11;
- Windows workspace tests `--no-run`: 25 occurrences, 21 identities,
  artifact coverage 11/11;
- Windows isolated Spout: 79 occurrences, 67 identities, artifact coverage
  9/9.

The deterministic ratchet self-test passes 52 assertion groups. The gate checks
exact identity multiplicity, line-independent paths, changed files, trusted Git
base/head state, baseline laundering, toolchain drift, `build-finished`, expected
artifacts, timeout, malformed JSON, warning-shaped stderr, first-party ownership,
external-warning ownership/expiry, Windows path forms, and warning/compiler input
through environment variables and semantic Cargo TOML. Added `allow`/`expect`,
warning flags, compiler wrappers, profile/target overrides, malformed Cargo config,
and Vite chunk-limit increases fail closed.

This is not W0 completion. `qa/warnings/warning-inventory.json` intentionally keeps
`requiredMatrixComplete=false`. Pending configurations are Windows ASIO loader,
the separately licensed excluded ASIO bridge, NDI, macOS default/release, Linux
default/release, structured frontend warnings, and structured native-release
warnings. The ASIO loader currently also fails at `main.rs:45388-45389` because the
no-default-feature build references missing `spout` and uses the `engine` crate as
a value. Local NDI capture is blocked first by missing NDI 6 headers and retains
the same no-Spout source boundary afterward. These are code/environment blockers,
not zero-warning results.

Native integration was rerun with the exact checkout process count at zero before
each attempt. Two setup attempts failed because Git's Unix `link.exe` was selected;
the successful invocation fixed
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` to the Visual Studio 2022 Community
MSVC linker and used the local FFmpeg 8.1.2 shared SDK plus LLVM. The final
`pnpm --dir app tauri build --no-bundle` completed in 2m55s. Its log reproduced
Engine 9 warnings, Syndocal 58 warnings, and one Vite oversized-chunk warning:
no increase from the observed native baseline and no threshold change. The rebuilt
executable reports FileVersion/ProductVersion `1.2.0-alpha.1`; it was launched as
PID `59296`, with exactly one exact-path process, one visible responsive `Syndocal`
window, and `IsZoomed=True`.

Bootstrap is deliberately not available to normal CI. H1 was locally verified with
explicit bootstrap against evidence commit
`e87ad9c9629bc25b847d9197216dd8aa20181dd8`. The documentation-only H2 is
`33f58df2f69c45892f2089037cbf00ed0aab3e56`; its parent is H1 and its diff contains
only this handoff and the completion-flow document. With `base=H1` and `head=H2`, all
four ratchets passed in normal mode: 83/83 with 11/11 artifacts, 83/83 with 11/11,
25/25 with 11/11, and 79/79 with 9/9. All gate, inventory, workflow, package, lock,
toolchain, and fixture files remained unchanged. The first H1 Actions run,
`32351887066`, had zero job steps: every OS was rejected by GitHub for account
billing/spending limits. Do not interpret it as code or platform evidence, and do
not claim PR bootstrap until the target base already contains the inventory.

The 2026-08-20 pushed checkpoint triggered Cross-platform run `32358575019` at
HEAD `48f9d1779f440dd364c67d9f44afdd3d624698eb`. Windows, macOS, and Ubuntu each
completed with failure and zero steps. The GitHub check annotation states that the
jobs were not started because recent account payments failed or the spending limit
must be increased. This is the same external billing block, not Windows/macOS/Linux
build evidence.

## Take Over repair checkpoint

The source-level Take Over blocker from the takeover baseline is repaired in the
current checkpoint:

- project-load/replacement internals now accept `&AppState`, so both Tauri
  `State<'_, AppState>` callers and the authenticated runtime use the same
  boundary without weakening lifecycle or output-ownership ordering;
- control-plane Take Over carries the consent-bound `{session_id, generation}`
  into the core, rechecks status after taking the lifecycle lock, and strictly
  verifies the exact manifest before stopping the Standby worker;
- a valid, invalid-JSON, corrupt-project, or noncanonical newer manifest cannot
  make Exact mode apply an older checkpoint. At this earlier repair checkpoint,
  the legacy local command still retained explicit `LocalLatest` fallback;
- stale or invalid Exact selection returns before worker stop, project
  publication, or output transition.

The normal default-feature `cargo check -p syndocal --locked` now succeeds in
the configured MSVC/FFmpeg/libclang environment. Focused proof passes the strict
filesystem reader, four Take Over tests, external replacement join ordering, and
polling-worker replacement scope. This closes the compile/generation-binding
repair only. It does **not** close Take Over's larger project-swap physical-output
retirement/re-arm fence or AI3 as a whole.

The later local OutputControl R4 checkpoint below supersedes that compatibility
detail: the production legacy Take Over Tauri handler is now fail-closed, so
`LocalLatest` is no longer reachable from a production command.

The 2026-08-20 follow-up closes the three residual proof gaps: explicit
empty/`+011`/alphabetic manifest-tail cases, a deterministic production-core test
proving stale Exact rejection leaves the installed worker running, and a shared
production execute-to-core seam which passes the actual `TakeOverStandby` action's
force bit, exact session/generation selector, and unchanged fence. Independent
adversarial review of the frozen follow-up returned P0 0, P1 0, and P2 0 for this
bounded repair.

## Local OutputControl R4 checkpoint

The checkpoint containing this section completes the next bounded AI3 slice, not
AI3 as a whole:

- local safety-latch Release Blackout, active Arm, and exact Standby Take Over now
  use three action-specific Tauri commands and three canonical R4 operations;
- the registry wire/query contract is version 3 and enforces exactly one local
  adapter per R4 operation. Authority/status are reviewed local queries and consent
  preparation is a non-adapter support phase;
- the browser validates exact authority/challenge/status/receipt DTOs and requires
  a visible nonblocking six-digit Raw Input challenge. Typed rejection is terminal;
  a transport reply loss retries once with the same request object;
- S0 engage remains a separate Full-Lock-capable priority path. Legacy local
  release/energizing handlers reject unsafe directions. Authored all/video/per-output
  release remains explicitly unavailable rather than being mislabeled as the
  safety-latch action;
- Arm/Release revalidate under the actual output transition guard. Take Over
  revalidates the exact Standby session/generation and force condition before any
  recovery/input/project/worker side effect; stale rejection leaves the worker
  running, while success prevents a queued worker from republishing;
- action commit returns its exact after-fence directly. There is no post-action
  fallible read that can turn an applied action into a false rejection, and the
  process-local terminal lane preserves exact retry after bookkeeping-lock poison.

Independent Terra xHigh review found no remaining code P0/P1 after the race,
receipt, registry-version, and DTO corrections. This claim is deliberately limited:
MIDI/OSC/Remote, target-aware all/video/per-output release, owner lease/orphan/restart,
durable receipts/audit, complete physical project retirement/re-arm, saturation,
Raw Input E2E, and real hardware remain open AI3 evidence. The `LocalLatest` selector
is no longer reachable from a production Tauri command; the old direct Take Over
handler is fail-closed.

## AI3 audit result

The five-part AI3 definition has now been audited against committed baseline
`896fb407f896edd46fe938a07a8bd5c71cfc956d`; the detailed matrix is recorded in
`qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md`. Every category remains partial:
runtime generations, output ownership, rate limits, Blackout/Arm/Take Over
safety, and physical-resource idempotency.

The former disconnected local R4 output vertical is closed by the checkpoint above.
It does not make every output ingress canonical: MIDI/OSC/Remote and wider physical
output actions remain open. The next safe slice is the AI3 owner-incarnation output
lease/orphan/forced-transfer state machine, followed by full project-swap physical
retirement/re-arm and durable receipt/audit/saturation proof. Do not add or claim
AI4 external principals, grants, revocation, sidecar access, or consent service in
the AI3 lease slice.

## Verified takeover environment

The Windows development environment was installed and locally verified:

- Node.js `22.22.1`
- pnpm `10.9.0`
- Rust/Cargo `1.97.1`
- Visual Studio 2022 Build Tools `17.14` with MSVC and Windows SDK `10.0.26100`
- LLVM/Clang and libclang `19.1.5`
- shared FFmpeg `n8.1.2-44-g7c533d0f86-20260818`

User environment variables now contain the Node, Cargo, and FFmpeg binary directories plus `FFMPEG_DIR` and `LIBCLANG_PATH`. A newly started terminal/Codex process is required to inherit them.

The following commands and gates passed during environment takeover and the Take Over repair. The absolute tool paths were used because the already-running Codex process had not inherited the newly registered user environment yet:

- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\pnpm.cmd --dir app install --frozen-lockfile`
- `set PATH=C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64;%PATH%&& C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\pnpm.cmd --dir app build`
- `cargo fmt --all -- --check`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-release-metadata.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-backend-operator-contract.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-timeline-follow-runtime.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-timeline-transport-runtime.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\node.exe app\scripts\check-safety-blackout-runtime.mjs`
- `C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\pnpm.cmd --dir app run check:output-ownership`
- `cargo test -p syndocal --locked exact_standby_reader_rejects_invalid_newer_candidates_without_local_fallback` — 1 passed
- `cargo test -p syndocal --locked takeover` — 4 passed
- `cargo test -p syndocal --locked external_project_replacement_joins_standby_worker_before_fenced_swap` — 1 passed
- `cargo test -p syndocal --locked standby_polling_replacement_never_attempts_lifecycle_stop` — 1 passed

The exact native check invocation was:

```cmd
call C:\Progra~2\MICROS~2\2022\BuildTools\Common7\Tools\VsDevCmd.bat -arch=amd64&& set FFMPEG_DIR=C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1&& set LIBCLANG_PATH=C:\Progra~2\MICROS~2\2022\BuildTools\VC\Tools\Llvm\x64\bin&& set PATH=C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1\bin;C:\Users\janua\.cargo\bin;%PATH%&& C:\Users\janua\.cargo\bin\cargo.exe check -p syndocal --locked
```

The takeover-baseline invocation originally exposed the committed type mismatch.
After the repair, the same default-feature command completed successfully with
warnings only.

The repository-native completion gate for this repair checkpoint also passed:

```powershell
$target = [IO.Path]::GetFullPath((Join-Path (Get-Location) 'target\release\syndocal.exe'))
Get-CimInstance Win32_Process -Filter "Name='syndocal.exe'" |
  Where-Object { $_.ExecutablePath -and [StringComparer]::OrdinalIgnoreCase.Equals([IO.Path]::GetFullPath($_.ExecutablePath), $target) } |
  ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
```

The exact-checkout process count was `0` immediately before the build. The exact,
reproducible build invocation was:

```cmd
call C:\Progra~2\MICROS~2\2022\BuildTools\Common7\Tools\VsDevCmd.bat -arch=amd64&& set FFMPEG_DIR=C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1&& set LIBCLANG_PATH=C:\Progra~2\MICROS~2\2022\BuildTools\VC\Tools\Llvm\x64\bin&& set PATH=C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64;C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1\bin;C:\Users\janua\.cargo\bin;%PATH%&& C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64\pnpm.cmd --dir app tauri build --no-bundle
```

It completed successfully and produced
`target/release/syndocal.exe`. That exact executable was launched and inspected
through Win32/process metadata at verification time: PID `20540`, executable path equal to this
checkout, title `Syndocal`, `Responding=True`, exactly one exact process, exactly
one responsive Syndocal window, and `IsZoomed=True` after maximizing. A later
exact-path reinspection still found PID `20540` responsive; this PID is historical
checkpoint evidence rather than a promise that the process will remain alive.
Before the next release build, inspect again and stop only that exact path as
required by `AGENTS.md`.

### Local R4 verification evidence

The following additional checks passed for this checkpoint:

- `cargo test -p protocol --locked` — 132 unit tests and 4 doc-tests passed.
- `cargo test -p syndocal --locked --bin syndocal output_control_ --no-fail-fast` — 10 passed.
- `cargo test -p syndocal --locked --bin syndocal canonical_registry --no-fail-fast` — 3 passed.
- `cargo test -p syndocal --locked --bin syndocal takeover --no-fail-fast` — 5 passed.
- `cargo test -p syndocal --locked --bin syndocal compiled_handler_and_registry_have_the_exact_same_set --no-fail-fast` — 1 passed.
- `cargo test -p syndocal --locked --bin syndocal external_project_replacement_joins_standby_worker_before_fenced_swap --no-fail-fast` — 1 passed.
- `pnpm --dir app run check:output-control-runtime`
- `pnpm --dir app run check:safety-blackout-runtime`
- `pnpm --dir app run check:output-ownership`
- `pnpm --dir app run check:backend-operator-contract`
- `pnpm --dir app run check:release`
- `pnpm --dir app run check:timeline-follow-runtime`
- `pnpm --dir app run check:timeline-transport-runtime`
- `pnpm --dir app run check:frontend-invokes`
- `pnpm --dir app build`
- the default-feature `cargo check -p syndocal --locked` in the exact VS/FFmpeg/libclang environment.

All Cargo commands in this Local R4 evidence block used the same `VsDevCmd.bat`,
`FFMPEG_DIR`, `LIBCLANG_PATH`, FFmpeg `PATH`, and absolute Cargo setup shown in the
earlier exact native-check command. All pnpm commands prepended
`C:\Users\janua\AppData\Local\SyndocalDev\node-v22.22.1-win-x64` to `PATH` and used
that directory's `pnpm.cmd`. The native build used the full reproducible command
shown above without modification.

Immediately before the new native build, exact-path inspection found only historical
checkpoint PID `20540`; that process was stopped and the remaining exact-path count
was `0`. The same reproducible build command shown above completed successfully in
4m16s. The exact new `target/release/syndocal.exe` was launched as PID `36980` and,
at verification time, was the only exact process and the only responsive window,
with title `Syndocal` and `IsZoomed=True`. This proves the bounded native integration,
not physical output, Raw Input confirmation, or ASIO acceptance.

## Required continuation order

1. Preserve the completed Take Over type/generation-binding repair and its strict filesystem regressions.
2. Preserve the completed local-GUI R4 vertical for safety-latch Release Blackout, active Arm, and exact Take Over. Do not broaden its claim to MIDI/OSC/Remote or all/video/per-output release.
3. Implement the missing AI3 output lease state machine: owner incarnation, resource set, monotonic generation, TTL expiry with unchanged physical state, orphaned state, stale-owner rejection, idempotent retry, restart non-reclamation, and a fail-closed forced-transfer ownership/state transition. AI4 supplies the human presence, authorization, grants, and consent for that transition; it does not own the underlying AI3 transfer state machine.
4. Fence Take Over/new/load/recovery through acknowledged physical retirement, project replacement, and explicit re-arm; then add durable physical receipt/audit and crash/reply-loss/saturation proof.
5. Close AI3 with the repository-native completion gate: exact-checkout process stop, `pnpm --dir app tauri build --no-bundle`, exact executable launch, exactly one responsive Syndocal window, and maximized-window QA.
6. Only then proceed to AI4.

### Exact next-slice contract: owner-incarnation output lease

The next Codex should delegate implementation to Luna Max and keep an independent
Terra High/xHigh reviewer read-only until a stable checkpoint. Do not extend the
existing `MachineOutputRole` / `OutputOwnershipGate` status object into a hybrid
lease. Add a distinct backend lease authority layer above that physical local gate.

Implement and fake-clock-test persisted `unclaimed`, `held_active`, and
`held_orphaned`; forced transfer is one synchronous atomic authority transition,
not a persisted intermediate state. Bind principal, window/renderer, backend process/session incarnation, backend-issued owner
incarnation), canonical resource set, monotonic lease generation, and
backend-bounded TTL. Expiry, disconnect, owner retirement, restart,
acquire, renew, recover, release, and forced transfer must have **no implicit physical
side effect**. They must not send output, change Blackout, restore the previous role,
Arm, or Take Over. Restart must not reconstruct authority from cached sidecar state
or old receipts.

Only an owner registered in the current backend process/session with the exact
renderer and owner incarnations may renew/recover. Forced transfer must be a
distinct, atomic all-resources-or-none, generation-fenced R4 authority transition
that invalidates the old owner without implicitly energizing or de-energizing output;
any physical change remains a later explicit action. It may serialize on
`output_ownership_transition`, but must not call the Engine ownership fence, change
machine role, stop a worker, or start teardown. Until AI4 exists it requires the
existing local prepared confirmation and otherwise fails closed. Extend the existing
lifecycle -> external admission -> coordinator -> output transition commit boundary
with lease revalidation for ordinary Release, Arm, and Take Over. Never make S0
Blackout wait for or require a lease.

Canonical resource mapping is mandatory: global safety-latch Release Blackout and
exact Take Over require `{lighting, video}`; Arm requires the exact resources enabled
by the backend-authoritative desired role and rejects Standby/unmapped targets. All
wider/target-specific operations remain fail closed until mapped. Project identity
replacement advances the generation and moves affected leases to `held_orphaned`
at the same commit boundary, with no physical side effect and an explicit Recover
required afterward. Name lease release `relinquish_output_lease` or equivalent so it
cannot be confused or routed to Release Blackout.

Minimum proof: same-owner renew, wrong-owner and incarnation-ABA rejection,
overlapping-resource rejection, expiry to orphaned with unchanged engine role/safety
latch/output-worker operation counts, exact retry and same-ID/different-shape,
restart non-reclamation, forced-transfer races with old-owner output actions, project
replacement interaction, and retained priority S0. AI4 still owns presence, grants,
authorization, and consent; this slice must not claim those services complete. Add
bounded single-flight/rate limits and exact receipts/audit containing session/owner
incarnations, canonical resources, generation before/after, and outcome. Prove that
an S0 race advances the safety generation and rejects an ordinary R4 before commit.

The first bounded code layer now exists in `app/src-tauri/src/output_lease.rs`: a
crate-private pure single-lease authority state machine only. It has no AppState,
Tauri, Engine, physical output, consent, receipt, or registry integration. Focused
ten fake-clock-style tests cover acquire/renew/exact-deadline orphan/recover/relinquish,
owner and process-session ABA, restart non-reclamation, checked overflow, atomic
authority transfer, and rejected-state equality. Independent Terra review found no
P0/P1 in this bounded core. Do not mistake it for an operational output lease.

The exact next code step is a bounded multi-lease registry with atomic overlap
handling and process-local exact request receipts/same-ID shape rejection, followed
by a separate reviewed AppState/R4 commit-boundary integration. Keep physical calls
out of the registry layer and preserve S0 independence.

Verification for this pure-core checkpoint:

- `cargo test -p syndocal --locked output_lease -- --nocapture`: 10 passed, 0
  failed, 757 filtered out;
- `cargo fmt --all -- --check` and `git diff --check`: passed;
- immediately before the native build, exact checkout PID `36980` was verified as
  `target/release/syndocal.exe`, stopped, and the remaining exact-path count was 0;
- the established VS/FFmpeg/libclang/Node command for
  `pnpm --dir app tauri build --no-bundle` succeeded and rebuilt the exact release
  executable;
- the exact executable was launched as PID `9436`; verification found exactly one
  exact-path process and one responsive `Syndocal` window with `IsZoomed=True`.

This native evidence proves only that the pure core compiles into the application;
because the core is deliberately not wired to AppState or commands, it is not
runtime output-lease acceptance or physical hardware proof.

## 2026-08-20 P2, warning, native, and five-display checkpoint

Implementation commit `a228ba5e492841618ce0262038c965b5519d1ddd`
closes the bounded Take Over P2 proof set and removes honest warning debt. The
production `TakeOverStandby` branch and its test now share the same injectable
production helper; the test proves `force=true`, `Exact(primary-a, 42)`, and the
unchanged consent-bound fence arrive at the core call. The earlier test-only
request/identity dispatch was removed. Focused serial proof passed 6/6 takeover
tests, including the actual stale-worker continuation case. The final independent
read-only review returned P0 0, P1 0, and P2 0 for this bounded repair.

The four enforced Windows warning ratchets passed without changing the inventory:

- default all-targets: 83 -> 61 first-party occurrences, 17 identities removed,
  artifact coverage 11/11;
- release all-targets: 83 -> 61, 17 identities removed, coverage 11/11;
- workspace tests `--no-run`: 25 -> 20, 5 identities removed, coverage 11/11;
- isolated Spout: 79 -> 57, 17 identities removed, coverage 9/9.

This cleanup reconnected real helpers and removed genuinely obsolete production
paths. It did not add `allow`, fake reads, warning flags, or a Vite threshold change.
The `output_lease.rs::checked_deadline` warning remains because the entire pure
Output Lease transition module is still deliberately disconnected from AppState/R4.
Deleting or suppressing that one helper would conceal the unfinished AI3 integration;
wire the whole reviewed lease state machine at the next AI3 boundary instead. The
current native build therefore still reports Engine 9 warnings, Syndocal 41 warnings,
and the known Vite oversized-chunk warning. W0 and warning-zero acceptance remain open.

The broader workspace test run performed before the final shared-helper replacement
selected 769 tests: 755 passed, 9 ignored, and 5 failed. Four failures were
parallel shared-state flakes and the same `authored_control_plane::tests::` set
passed 12/12 when rerun serially. The remaining deterministic pre-existing blocker
is `control_plane::tests::legacy_v1_registry_json_and_count_remain_inventory_honest`
with expected 1410 versus actual 1423. This checkpoint does not launder that count
by blindly updating the assertion. The final focused takeover run after the helper
replacement is green as recorded above.

Immediately before the successful native build, exact-path inspection stopped only
PID 63932 whose resolved executable was this checkout's
`target/release/syndocal.exe`; the remaining exact count and debug-port 9339 listener
count were both zero. The first build attempt selected Git's `/usr/bin/link` and
failed before application linking. The retry fixed
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` to the Visual Studio 2022 Community
MSVC linker and `pnpm --dir app tauri build --no-bundle` completed successfully in
2m41s. The resulting executable has SHA-256
`4D85B345021FCF507440BE9ADB3B205BBE906E5D4CAAE583A1B00590F2A506E5` and
FileVersion/ProductVersion `1.2.0-alpha.1`. It was launched as PID 31076; exactly
one exact-path process and one responsive `Syndocal` window existed, and the window
was maximized (`IsZoomed=True`, observed rectangle 1936x1048 including frame bounds).

The Windows display topology observed for video-output QA was:

- `DISPLAY2` primary: 1920x1080 at (0, 0);
- `DISPLAY1`: 2048x1152 at (1920, -364);
- `DISPLAY3`: 2560x1440 at (-3840, -429);
- `DISPLAY5`: 1280x720 at (-2465, 1731);
- `DISPLAY6`: 2560x720 at (1598, 1080).

All five were 32 bpp. The installed adapters included AMD Radeon Graphics and an
NVIDIA GeForce RTX 5090, but also Parsec Virtual Display Adapter and Meta Virtual
Monitor. Therefore the OS-visible count of five is not proof of five physical
panels. In the same-checkout runtime probe, five enabled fullscreen Display outputs
were configured with monitor indices 0 through 4. Opening them was correctly
blocked while the effective machine role was Standby and output ownership reason
was `ProjectSwapDisarmed`. The visible six-digit OutputControl R4 challenge also
rejected synthetic key injection, preserving the physical Raw Input boundary.
Consequently this run proves fail-closed multi-display routing and the five-target
enumeration, but it does **not** claim successful physical fullscreen output. The
intended three-screen editor + LED panel + projector workflow, refresh/DPI/reorder/
unplug behavior, and successful human-entered physical consent remain explicit
hardware acceptance work.

## Persistent collaboration and checkpoint rules

- Use a dedicated implementation agent and a separate read-only/adversarial reviewer for material implementation work.
- Default implementation delegation is `gpt-5.6-luna` with maximum reasoning. Escalate difficult work to `gpt-5.6-terra` high/xhigh, then `gpt-5.6-sol` when needed. The supervising Sol agent owns integration and the final claim.
- While an agent or build is running, advance non-overlapping investigation, test planning, documentation, or review work; do not idle.
- At each meaningful verified checkpoint, update the roadmap/status/handoff documentation, commit with a descriptive message, and push the active branch. Never leave the only usable handoff in chat history.

## ASIO product requirement

Windows ASIO support is an explicit product implementation and release requirement, not an optional undocumented experiment. Existing bridge, smoke, 100-cycle, and native-UI evidence must be preserved, but they do not close the requirement by themselves. Completion must also resolve the distribution/license boundary and the open acceptance items recorded in `qa/ASIO_INPUT_ACCEPTANCE.md`, including supported-driver breadth, device loss/recovery, sustained low-latency operation, observable actual buffer/XRUN behavior, and native end-to-end QA. Unsupported or failed ASIO selection must remain fail-closed and must not silently fall back to another driver or WASAPI.

## 2026-08-21 DJ Link, output authority, warning-P2, and header checkpoint

The active implementation remains on `codex/syndocal-v1.2`, based on
`2885ac6a4b3844f67bf55ccaa6efafa190d698c8`. The Syndocal side of the
replacement DJ Link specification extends the existing Web Remote listener; the
separate `Seraf0-org/rekordbox-DJ-Link-ForPCDJ` repository was not modified. That
peer still owns rekordbox discovery, Pedal/global-hotkey input, MIDI, Filter,
Stop, reconnect, and its operator UI. Wired two-PC, rekordbox, and physical Pedal
acceptance remains external evidence and is not claimed here.

The frozen DJ transport authenticates a backend-issued token, requires strict
HELLO/session envelopes, ACKs exact event identities, retains bounded high-water
dedupe, treats StateSync as state rather than a trigger, applies absolute Loop
division and idempotent Release through existing Engine paths, and persists exact
Track-to-Timeline mappings in `.sdc`. Same-token socket replacement is fenced at
the irreversible dispatch commit point. A dispatch lease serializes replacement
against the physical handler, and handler panic/send/close paths return connection
slots and terminalize the admitted identity without exposing panic details or
allowing a physical retry. The final independent Terra review of the frozen diff
reported P0 0, P1 0, and P2 0.

Focused evidence at this checkpoint:

- `io remote_ws`: 24/24, including the post-final-check ABA barrier and
  max-connections=1 panic recovery;
- `protocol dj_link`: 4/4, `engine dj_link`: 4/4, `syndocal dj_link`: 3/3;
- Output Lease: 33/33; Output Control: 13/13;
- frontend DJ, Output Control, bundled-library retry, invoke inventory (403),
  localization (3512/3512), and topbar contract all passed;
- the topbar contract now contains zero master sliders. Only the always-visible
  Lighting and Video master sliders were removed; non-header master controls and
  runtime APIs remain. No typography, button, spacing, or hit target was shrunk;
- Vite transformed 265 modules and emitted no warning-shaped output. The largest
  application chunk was about 435.5 kB, below the unchanged warning threshold;
- the generic warning runner passes 52 assertion groups both through the official
  pnpm script and direct Node. Package-manager variables are scrubbed from the
  child process while `NODE_OPTIONS` remains fail-closed.

Immediately before the final native build, exact-path inspection found zero
running instances of this checkout's `target/release/syndocal.exe`. With FFmpeg
8.1.2 shared, LLVM, and the Visual Studio 2022 MSVC linker explicitly selected,
`pnpm --dir app tauri build --no-bundle` completed in 2m12s and emitted no
first-party or Vite warning. Native launch/maximize and the five-display then
three-display VJ workflow are intentionally the next operation after the warning
inventory rebaseline, because the structured native warning gate performs another
exact build and would otherwise invalidate the running-process evidence.

`checked_deadline` is no longer dead code: Output Lease integration calls it from
acquire, renew, transfer, receipt, and recovery paths. It must not be added to an
allowlist. The seven locally measurable pending warning configurations are ready
for a separately reviewed zero-warning inventory promotion; macOS dev/release
remain externally blocked by the unavailable host and the recorded Actions billing
failure. `requiredMatrixComplete` must therefore stay false until those two rows
are measured.

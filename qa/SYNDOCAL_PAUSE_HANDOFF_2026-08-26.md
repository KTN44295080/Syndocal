# Syndocal show-readiness pause / resume contract — 2026-08-26

> **Authoritative resume boundary while this file is current.** This is a
> truthful operational handoff, not an acceptance record and not permission to
> promote incomplete work. If the project is paused, resume from this document,
> `AGENTS.md`, and the current worktree together. Do not infer completion from
> a source build, a browser harness, a status label, or an earlier artifact.

## 1. Identity at this draft checkpoint

| item | recorded fact | status / required refresh |
| --- | --- | --- |
| KDMX branch | `codex/syndocal-v1.2` | Verify `git branch --show-current` on resume. |
| Last already-pushed KDMX HEAD | `aed77a2626a0306fa466b3077f3fb7997919faf8` (`feat: freeze alpha.12 show-critical runtime`), equal to its upstream at the 2026-08-26 source-freeze snapshot | This is the integrated alpha.12 source checkpoint. A later small three-display GDI clean-break checkpoint and this final handoff still require their own commit/push and final refresh below. |
| Preceding checkpoints | `7ee3b8f` (`test(cleanup): gate exact build cache reclamation`), `a0c76c5` (`test(native): harden window acceptance trust boundary`), `0df10d2` (`docs: correct current show completion authority`) | Test/documentation checkpoints preceding the integrated alpha.12 source freeze. Cache deletion and unrun physical gates are not implied. |
| Product train | `1.2.0-alpha.12` source metadata is synchronized and the integrated source checkpoint is pushed. | Alpha.12 is a development artifact, not beta/RC/release. The final accepted standard and local-only Show-ASIO artifacts must each remain bound to the exact final pushed HEAD recorded below. |
| DJ repository | `C:\Users\kouty\Desktop\rb-output`, branch `beta-v1.1.2`, pushed HEAD `925880068500d42d71b2671fa8a99e5895aca4e2` | Verify it remains equal to `origin/beta-v1.1.2`; do not replace it with an installer for this show-source exception. |
| Current pre-final checkout artifact | PID `160808`, exact path `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`, `1.2.0-alpha.12`, 58,471,936 bytes, SHA-256 `BBD4B9B5C803B3B00B38387A3C06A5CFFB49F6A88A3CB8AC5B4066FBB06363A1`, built from `aed77a2626a0306fa466b3077f3fb7997919faf8`; exactly one responsive maximized `Syndocal` window was observed | Valid evidence for the pushed source freeze only. The later GDI-harness checkpoint changes HEAD, so this process must be exact-path stopped and rebuilt/reverified before the final pause identity is recorded. |

Additional protected state at this draft checkpoint:

- `stash@{0}` is alpha.9 WIP (`e9209d6`) and `stash@{1}` is orphaned open-dmx
  pacing. They are user/work history, not cleanup candidates; do not drop,
  apply, or rewrite them without a separate attribution decision.
- The companion worktree `C:\Users\kouty\Documents\KDMX-asio-persistence`
  is on `codex/asio-persistence-v2`, HEAD/upstream
  `0ab0ca46569b0ab4f08b3c61ca4f4f202c4a0de4`, with the following 12 tracked
  dirty paths at the 2026-08-26 snapshot: `app/scripts/check-backend-operator-contract.mjs`,
  `app/scripts/check-frontend-command-routing.mjs`,
  `app/scripts/check-live-audio-input.mjs`, `app/src-tauri/src/control_plane.rs`,
  `app/src-tauri/src/main.rs`, `app/src/App.tsx`,
  `app/src/tauri-invoke-manifest.json`, `app/src/tauriInvokeCommands.ts`,
  `app/src/types.ts`, `qa/ASIO_INPUT_ACCEPTANCE.md`, `qa/harnesses/README.md`,
  and `qa/harnesses/check-asio-build.ps1`. It overlaps current App/main/ASIO
  files and is protected concurrent evidence; do not merge, remove, clean, or
  use it as current-checkout acceptance until its ownership is explicitly
  reconciled.

This file must be updated with the final exact commit/push/hash/process/gate
facts immediately before any true pause. The placeholders labelled
**IN-FLIGHT** below deliberately remain non-claims until the owner completes
and an independent review accepts the exact frozen diff.

## 2. Non-negotiable operating rules

### Fail closed and make clean breaks

- Invalid, ambiguous, stale, unsupported, unverified, or future state must
  stop with a visible, actionable failure. It must not become a default,
  guessed, normalized, or silently recovered state.
- Do not add fallback/legacy/shim/retry paths merely to preserve an obsolete
  route. When a clean break is selected, remove its retired entry points,
  persisted data, schema/adapter paths, UI, tests, documentation, and generated
  inventories in the same bounded tranche unless an explicit one-way migration
  is required by real user data or an external protocol.
- Current DJ wire authority is `syndocal-envelope-v2` only. Flat generic JSON,
  `syndocal-envelope-v1`, aliases, and unknown adapters must reject without
  conversion or fallback.

### Parallel work, capability order, and ownership

- The supervising Sol lane owns decomposition, integration decisions, exact
  acceptance, and all completion claims. Delegated summaries are evidence, not
  acceptance.
- Capability order is **Sol > Ox (Zen / `opencode/x-preview-f-free`) > Terra >
  Luna**. Ox is the default lane for bounded implementation, investigation, and
  adversarial review. Terra is a secondary difficult lane; Terra changes need a
  separate Ox review. Luna Max is only for small, explicit, low-ambiguity work.
- Keep every safely independent lane occupied. Do not wait while an independent
  audit, gate planning, documentation correction, or disjoint implementation is
  available. Serialize only actual dependencies, exclusive UI/native/hardware
  steps, destructive operations, or same-file ownership.
- Assign files before editing. No two writers may edit the same file. Review is
  read-only and independent until the implementation owner freezes the diff.
- Every material implementation requires an independent adversarial review;
  blockers found by review must be repaired and re-reviewed before integration.
  Near the show deadline this means one bounded review of the changed authority
  for concrete crash, wrong-output, data-loss, unsafe-process, or build-bypass
  P0/P1 failures. Do not spend time or tokens on repeated whole-repository
  reviews, stylistic P2 exploration, theoretical parser games, or already
  accepted unchanged surfaces unless a focused gate supplies new evidence.

### Checkpoint, version, warnings, and cleanup discipline

- At every meaningful checkpoint: update the relevant QA/roadmap/release
  documents, run focused gates, record warning counts, stage an explicit file
  list, inspect staged diff/name list, commit, push, and verify upstream
  equality. Preserve a failed push's commit and record the recovery rather than
  rewriting history.
- Advance product prerelease ordinals only for intentionally distributed
  development artifacts. Synchronize Cargo, lockfile, frontend, Tauri, scripts,
  artifact names, updater metadata, and current user-facing text. Run
  `pnpm --dir app run check:release` after a version change. Product version is
  not a schema/API/ABI version bump.
- Warning ratchet: a modified file with a first-party warning blocks its tranche.
  Do not use broad `allow`, `-Awarnings`, fake reads, or chunk-limit inflation.
  Historical zero-warning measurements are not claims about the dirty alpha.12
  tree; the full alpha.12 gate remains unmeasured and FC-28 recorded 12 open
  first-party large-error clippy lints pending a fresh full-gate rerun.
- Delete stale generated/cache material continuously, but only after its exact
  recurring path set has a tracked cleanup harness, passing focused tests, and
  an independent adversarial review. Never run cleanup deletion while the tree
  is dirty or writers are active. Resolve each absolute target first; preserve
  current artifacts, QA evidence, and user-authored files; record reclaimed
  bytes. After deletion of shared dependency/cache trees, verify/rebuild the
  active checkout from its frozen lockfile before calling cleanup safe.

## 3. Windows native build and UI acceptance ceremony

Before **every** Windows Cargo or Tauri native build/test, use the following
exact local ceremony. A gate that lets Cargo use Git's `usr\bin\link.exe` is invalid
and must be rerun from the beginning.

1. Start `vcvars64.bat -vcvars_ver=14.44` in `cmd.exe /v:on`.
2. Require the first `where.exe link.exe` result to be exactly:

   ```text
   C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe
   ```

3. Set and print `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` to that exact
   absolute path. Do not use `%PATH%` after `vcvars64`; use delayed expansion
   (`!PATH!`) so the compiler environment is retained. Fail closed if the exact
   toolset/path is absent or not first.
   The only edition-root exception is the official GitHub-hosted `windows-2022`
   runner: only in that explicitly marked hosted context, require the exact
   corresponding Visual Studio 2022 Enterprise 14.44.35207 `Hostx64\x64\link.exe`.
   Never admit that exception on this local machine or on a self-hosted runner.
4. Immediately before a release build, enumerate all processes, resolve their
   executable paths, and terminate only a process whose exact resolved path is
   this checkout's `target/release/syndocal.exe`. Never terminate Daslight or a
   similarly named executable from another checkout.
5. Run `pnpm --dir app tauri build --no-bundle` successfully.
6. Launch that exact resulting `target/release/syndocal.exe`; verify exactly one
   responsive `Syndocal` window. Maximize that verified target before any UI
   automation/manual QA (except a deliberate restore/minimize test).

Browser, Vite, TypeScript, static PowerShell, and Rust-unit results are useful
but never substitute for this native ceremony. A native/UI/runtime tranche is
not complete until this exact executable gate is recorded.

## 4. DJ-Link controlled source-acceptance boundary

The source checkpoint at `rb-output` HEAD `9258800` is pushed and the controlled
source launcher/preflight has passed. That does **not** complete physical DJ-Link
acceptance.

- The only show route is the checked-out source launcher in the same PowerShell
  that sets checkout-external `DJ_AGENT_CONFIG_PATH`; no installer/shortcut may
  be substituted for the current source-acceptance exception.
- The intended external JSON selects Syndocal host `192.168.50.1`, DJ local NIC
  `192.168.50.2`, `CustomMIDI1` with its exact enumerated port, and the sole
  adapter `syndocal-envelope-v2`.
- The user must rotate/copy the currently displayed Syndocal one-time token only
  after an accepted current native Syndocal artifact is running. A fake token may
  prove preflight only; it must never be used for a no-argument show start.
- The DJ process, Rekordbox, MIDI output, pedal, authenticated HELLO/ACK,
  StateSync, Master-track trigger, timing/position/8-4-2 loop behavior, release,
  disconnect/reconnect, and both-PC restart are **hardware acceptance 0/12**.
  A `connected` label or preflight pass is not authenticated operation.

### Exact DJ resume start

1. Wake the DJ PC and attach the wired show LAN.
2. Obtain a newly accepted/current Syndocal native artifact, explicitly bind the
   FOH listener to `Ethernet 4 / 192.168.50.1`, and rotate/copy its one-time
   token into the checkout-external JSON.
3. In a fresh DJ-PC PowerShell, set `DJ_AGENT_CONFIG_PATH` to the external JSON
   and run `start-all.bat --preflight-only`. Record the result.
4. Only with the real token and an accepted listener, run `start-all.bat` with no
   arguments. Verify the Setup values, then execute and record all 12 physical
   acceptance rows. On any mismatch, stop and diagnose the authoritative route;
   do not add a fallback launcher or adapter.

## 5. Show target and explicitly deferred authoring

The public performance is **2026-08-30**, but the development, native/hardware
acceptance, and show-project preparation completion deadline is **2026-08-29**.
August 30 is not development contingency time. The immediate operating objective
is therefore to be show-operable by August 29 in the physical three-screen
scenario:

| role | required physical native resolution |
| --- | --- |
| Editor / operator | 1920x1080 |
| LED panel | 1920x1080 |
| Projector | 3840x2160 |

The 2026-08-26 read-only physical inventory found five connected displays and
resolved the active show-role mapping by stable DisplayConfig identity. Its
current GDI observations were Editor `DISPLAY2` at 1920x1080/DPI 96, LED
`DISPLAY33` at 1920x1080/DPI 144, and Projector `DISPLAY3` at 3840x2160/DPI 144.
GDI `DISPLAY<n>` names are transient observations, not role authority; the LED
was previously observed as `DISPLAY5` without its physical identity changing.
The projector is physically/currently 4K; the earlier statement that this PC
had no 4K display confused its 150% scaling with native resolution and was
wrong. The final native three-output acceptance must bind each role to its exact
stable identity, then verify the current nonblank GDI name, DPI, physical
resolution, and window binding rather than infer a route from effective CSS
coordinates or a hardcoded GDI number.

The exact source import is
`C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc` (not a similarly named file).
It must be imported through the evidence-bound importer/report path. The
authored show project is deferred until the current native product is accepted;
do not claim it exists now. After that boundary, create representative Lighting,
temporary deterministic LED/projector media, and the `人生オーバー` / `惑う星`
Timelines. The final user-authored artistic scenes and videos remain outside
software acceptance, but the software path must prove the Master trigger,
position sync, 8/4/2 loop, pedal release to band continuation, BPM-aware
automatic next-Timeline transition, save/restart, and physical route binding.

## 6. ASIO product and local-only licensing boundary

Windows ASIO is a release requirement, not a preference. Default MIT/WASAPI and
the separately licensed non-default ASIO bridge remain distinct.

ASIO acceptance still requires explicit device enumeration/selection,
sample-rate/native-format/channel/buffer negotiation, low-latency callback I/O,
exclusive open/start/stop/free behavior, typed fail-closed disconnect/XRUN/
no-callback recovery, stale-selection lock after persistence/re-enumeration,
deterministic tests, and real-device QA. The authoritative ledger is
`qa/ASIO_INPUT_ACCEPTANCE.md`.

The show-local ASIO packaging implementation and deterministic self-tests are
integrated, but the exact final pushed-head artifact construction and physical
device acceptance remain **IN-FLIGHT**. It must remain
local-only and separately licensed: no normal installer/updater/artifact may
stage, glob, or publish the bridge until the exact DLL/hash/license/notice/
manifest gate approves it. Do not report the existing direct-device evidence as
fresh alpha.12 native UI, persistence, hot-plug/fault, one-hour soak, or
input-to-pixel latency acceptance.

## 7. Stabilization tranche progress (not final acceptance)

The following were progressed while preserving the remaining work as open:

- `0df10d2` corrected active completion guidance so historical warning/v1/network
  observations are not current instructions; its subsequent topology correction
  makes the old `192.168.1.34` route explicitly historical and names the current
  `.50.1/.50.2` show topology.
- `a0c76c5` hardened the native-window acceptance trust boundary. It is a test
  checkpoint only; it does not prove current integrated native UI, hardware, or
  alpha.12 completion.
- `7ee3b8f` added and independently reviewed an exact cleanup plan/test gate for
  `target/debug/incremental`. Its current production Plan result is blocked by
  the dirty worktree; no deletion has been run and reclaimed bytes remain zero.
- DJ source checkpoint `9258800` documents the controlled source path and its
  fail-closed configuration preflight. Hardware remains 0/12.
- Integrated checkpoint `aed77a2` contains the correlated Timeline close fence,
  protocol/engine active-cue validation, Display presentation revalidation,
  App ASIO revalidation, runtime/packaging checks, Spout/NDI closure, and the
  local-only ASIO build/checker route. Focused gates and independent reviews
  found no unresolved P0/P1 in the frozen source checkpoint.
- Protocol focused tests passed 175/175; I/O focused tests passed 158/158;
  engine full tests passed 868 with two manual benchmarks ignored; Spout passed
  37 with five hardware-only tests ignored; NDI passed 76. The complete warning
  matrix recorded zero first-party warnings in every executed configuration.
- Frontend localization passed 3568/3568, Timeline source shelf passed, and the
  full Timeline slim matrix passed at 1920x1080, 1920x1032, 2048x1152,
  1366x768, and 1280x720. `pnpm --dir app build`, release metadata, release
  self-tests, and the exact-linker native release configuration passed.
- The three-display harness now accepts only the standard exact checkout
  artifact or the same-host manifest-verified Show-ASIO artifact. Its additional
  GDI clean-break removes hardcoded `DISPLAY<n>` role authority and verifies
  stable-identity renumber acceptance plus role-swap rejection. PS7 and Windows
  PowerShell 5.1 self-tests each pass 80/80. Ox-alpha reported no P0/P1 and
  approved the focused diff; its one accepted residual P2 is that the inventory
  blank-GDI test invokes the shared guard directly while the selected-monitor
  and window-metrics blank-GDI tests exercise their full sample paths. The final
  checkpoint SHA is pending below.
- The DJ-Link ACK writer now makes serialization/send an explicit terminating
  Result path with zero fake bytes and no legacy retry. Hardware remains 0/12.

No item in this section promotes the product to beta/RC/release, a clean
alpha.12 artifact, ASIO completion, DJ hardware completion, three-display
completion, or the authored show project.

## 8. Ordered open work at pause

1. **Finish and push the bounded GDI clean-break checkpoint.** Preserve stable
   DisplayConfig identity as sole role authority; rerun both PowerShell selftests
   and one final bounded review before committing.
2. **Construct exact final artifacts.** Rebuild/reverify the standard alpha.12
   executable from the final pushed HEAD, then build/check the local-only
   Show-ASIO artifact from the same clean pushed HEAD. Record exact paths,
   versions, byte counts, hashes, manifests, and process identity.
3. **Run native physical acceptance.** Verify the exact single maximized main
   window, 1920 editor + 1920 LED + 3840 projector physical bindings, ASIO,
   persistence/restart, and faults using the accepted alpha.12 executable.
4. **Run DJ physical matrix.** Use the one-time-token source route and record all
   12 rows; fail closed on any missing HELLO/ACK, mapping, timing, or restart
   result.
5. **Import and exercise DSF show material.** Import exact `DSF2026.dvc`, save
   the importer report, construct the representative three-output project, and
   execute the two-song scenario with temporary media before user artistic work.
6. **Clean only after a stable pushed freeze.** Run the reviewed exact-path
   cleanup harness only after no writers/dirty state remain; record deletion
   targets and reclaimed bytes, then re-verify dependencies.
7. **Retire mega-file coupling after the show-critical boundary.** `App.tsx`
    and `main.rs` still combine multiple frontend, Tauri, engine, recovery, and
    output authorities. The near-show gate requires an integration review of
    their frozen hashes; a later bounded clean-break must extract authority-
    owned modules without adding compatibility paths. This debt is not silently
    declared solved by alpha.12.
    - At the 2026-08-26 pre-freeze audit, `app/src/App.tsx` was 28,810 lines
      (1,202,918 bytes) and `app/src-tauri/src/main.rs` was 128,698 lines
      (4,937,271 bytes). The current tranche was still net-positive in both;
      therefore the size/coupling obligation is **open**, not completed.
    - Effective extractions already in use are `bankAuthority.ts`,
      `projectTransactionRecovery.ts`, `liveAudioInputSelectionStorage.ts`,
      `videoOutputWindowObservation.ts`, `asio_bridge_v2.rs`, and
      `physical_output_fence.rs`. They are evidence of partial reduction only.
    - From the next tranche onward, no new domain policy or authority may be
      added directly to `App.tsx` or `main.rs`. New work must enter an owned
      module; changes to the mega-files are limited to thin wiring until the
      remaining seams are removed.
    - Post-show extraction order is: native live-audio runtime, native display
      output, cue-list authority mutation, frontend project-transaction
      controller, then frontend bank/live-audio hooks. Each extraction is a
      clean break with focused tests; no parallel legacy route or silent
      fallback may be retained.

## 9. Exact resume checklist

Before allocating any new implementation task:

```powershell
Set-Location C:\Users\kouty\Documents\KDMX
git branch --show-current
git rev-parse HEAD
git status --short
git rev-parse --abbrev-ref --symbolic-full-name '@{u}'
git rev-list --left-right --count '@{u}'...HEAD

git -C C:\Users\kouty\Desktop\rb-output branch --show-current
git -C C:\Users\kouty\Desktop\rb-output rev-parse HEAD
git -C C:\Users\kouty\Desktop\rb-output status --short
```

Then:

1. Compare all results against Section 1. If any identity, upstream, owner, or
   process differs, update this handoff before editing and treat unexpected work
   as someone else’s until attributed.
2. List active agents and assign non-overlapping files. Fill safe independent
   Ox lanes first; assign a separate read-only Ox reviewer for each material
   frozen change.
3. Re-run or explicitly mark stale every recorded gate. Never reuse alpha.11,
   browser-only, previous device, or previous LAN evidence as alpha.12 proof.
4. Apply the exact-linker ceremony before any Cargo/Tauri operation; verify the
   exact native process path before terminating it.
5. Before any pause, replace every **IN-FLIGHT** label with either an exact
   owner/frozen-diff/gate/review state or an explicit blocked reason; update
   branch/HEAD/upstream, artifacts, processes, warnings, hardware results,
   deletion results, and the first next action. Commit/push that handoff, then
   terminate delegated continuation work if an actual stop is requested.

## 10. Final pause fields — **IN-FLIGHT / MUST REFRESH**

| field | final value required before a true stop |
| --- | --- |
| KDMX branch / HEAD / upstream equality | **IN-FLIGHT** |
| Staged and unstaged owned files by agent | **IN-FLIGHT** |
| Protected untracked evidence / stashes | **IN-FLIGHT** |
| Running process paths, PID, version, SHA-256 | **IN-FLIGHT** |
| Exact gate commands, pass/fail counts, first-party warning counts | **IN-FLIGHT** |
| Independent Ox reviews and unresolved P0/P1/P2 | **IN-FLIGHT** |
| DJ 12-row hardware evidence | **IN-FLIGHT; currently 0/12** |
| ASIO physical-device evidence | **IN-FLIGHT** |
| Three-display and DSF show-project evidence | **IN-FLIGHT** |
| Cleanup absolute paths and reclaimed bytes | **IN-FLIGHT; do not delete while dirty/writers exist** |

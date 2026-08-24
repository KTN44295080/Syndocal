# Syndocal Codex handoff — 2026-08-19

## Current checkpoint

- Branch: `codex/syndocal-v1.2`
- Current implementation checkpoint: `0a3e91ab87d4be4735710234d1ba002e484375ef`
  (`feat: add sample-accurate timeline cue audio`). Push evidence is recorded by
  the documentation follow-up commit containing this line.
- Current completion authority: `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md`. The active product train is `1.2.0-alpha.8`.
- **2026-08-22 Windows-only scope:** current completion targets this operator's
  Windows PC. macOS/Linux control/native/warning rows and the six distribution,
  legal, signing, SBOM, clean-machine, updater, and publication rows are deferred
  outside the active denominator. Windows native release, zero warnings,
  the final editor + LED panel + projector topology, ASIO, physical I/O/DJ Link, crash recovery, security, and soak
  remain required. The post-E1 baseline is 14/71 (19.7%), not 14/79.
- **2026-08-22 backend-first verification rule:** routine tests must invoke the
  same registered Tauri/control-plane production paths from backend drivers.
  Do not repeat Computer Use for every fix. Run one maximized release-executable
  UI/hardware acceptance after the integrated bundle is otherwise green.
- **Historical 2026-08-22 `1.2.0-alpha.3` checkpoint:** Commit A `486d420` records the
  integrated Windows show-core work and synchronized product metadata; Commit B
  `4120539` contains only the reviewed frontend build-marker rebaseline. The
  explicit `486d420...4120539` output-marker audit passed. Release metadata and
  its 65 self-test groups, Cargo locked metadata, frontend build (268 modules),
  localization 3530/3530, invoke inventory 411, OutputControl, video-window,
  project-authority, fixture-catalog, Timeline DnD, empty-state, and the focused
  five-viewport Edit/Timeline/Setup checks all passed. The no-default Syndocal
  check emitted zero first-party warnings. Timeline media audio passed 31/31,
  engine audio 12/12, Follow and child-quorum 1/1, with fixed-hash independent
  review P0/P1/P2=0 (`engine` `C9FB7C0A...`, `main` `D85641CF...`). The normal
  warning gate on the documentation-only successor passed at 0/0/0. Before the
  native build, exact-checkout `syndocal.exe` process count was 0. The successful
  `pnpm --dir app tauri build --no-bundle` used the WinGet FFmpeg 8.1.2 shared SDK,
  VS 2022 amd64 environment, and an absolute MSVC linker to avoid the unrelated
  Git `link.exe` name collision. After the final typed audio-fault and Guide-asset
  integration, the release build was repeated with delayed VS environment
  expansion; the preceding attempt failed before linking with environment-only
  `LNK1181: opengl32.lib` and produced no acceptance artifact. The resulting
  executable SHA-256 is
  `7A630F1DBAF54A1E7B74A5CD108A3EE33C968C8E77BC9F983236D1E19F269AA9`;
  FileVersion/ProductVersion are both `1.2.0-alpha.3`. PID 102788 supplied exactly
  one responsive `Syndocal` window and was maximized. No feature UI operation was
  repeated; the single full operator/hardware pass remains reserved for final
  acceptance. The normal warning gate from the reviewed `4120539` baseline to
  `4b3781d` passed with total/first-party/third-party counts `0/0/0`, and the
  checkpoint push completed successfully.
- **Historical 2026-08-22 alpha.3 Timeline click gap (superseded by the alpha.4
  scheduler checkpoint below):** the then-current Rodio click was a 25 ms polling,
  fixed-4/4 sine-tone implementation. It is not accepted for the show-core. The
  next independent tranche after the real MediaAsset path must add an authored
  numerator/denominator meter map, sample-frame scheduling and generation-fenced
  cancellation, the reference 1320/920 Hz square-wave envelope and click bus,
  and exact `惑う星` 113-128 / 194 BPM / 74-click proof. MTC, loop, seek, BPM,
  replacement and DJ transport discontinuities must not emit stale clicks.
- **Historical 2026-08-22 alpha.3 Timeline Guide scope/gap (superseded by the
  alpha.4 cue-audio checkpoint below):** the engine/native path then emitted and
  generation-fences Phase/`Looping`/`Break`/`Trans` cues and plays the fixed
  embedded English WAV vocabulary through its own device/gain bus. It is not the
  final timing/routing path: playback is appended after the 25 ms media-audio
  poll, `Trans` cadence assumes four beats per bar, and device/gain selection is
  only session-local. The fixed English vocabulary is sufficient by product
  decision; custom labels remain visible text-only faults and TTS/Japanese voice
  packs are not required. Guide speech follows Timeline BPM only through a
  pitch-preserving rate captured at cue start and bounded to 0.92x-1.08x around
  1.00x at 120 BPM; ordinary sync correction must not wobble a word. The Guide
  tranche must share the new sample-frame
  tempo/meter authority, retain a separate bus, persist machine-local routing
  safely, and prove stale-generation/device/custom-label failures through the
  production command path before completion.
- **Historical 2026-08-22 Guide completion cue (superseded by the alpha.4 cue-audio
  checkpoint below):** the selected Follow-end phrase was the
  single word `Complete`, generated with the same `Microsoft Zira Desktop`, Rate
  2, Volume 100 voice as the existing fixed pack. It must play exactly once only
  after successful Follow settlement, before a due destination Phase cue; abort,
  failure, or stale generation must produce zero `Complete` cues. The temporary
  audition file is `C:\TEMP\syndocal-guide-candidates\complete.wav` and is not yet
  a product asset.
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
- The Windows-first CI checkpoint is `85d6eb2b6d7877cd49801c288357d3c72a36046f` (`ci: enforce complete Windows warning matrix`). It wires all nine Windows-enforced warning configurations, including SDK validation and exact-checkout process retirement before native builds. Independent read-only review of workflow SHA-256 `B94512DB87C7990F4505635FE78760620F8BFFEE189E55B4688CE39DAC2A90AA` returned P0 0, P1 0, and P2 0. macOS dev/release remain the only two pending warning rows.
- **2026-08-21 OutputControl superseding checkpoint:** the physical-input consent feature is removed, not deferred. OutputControl no longer exposes or runs the six-digit/Raw Input/physical Enter challenge, its 15-second timer, prepare/status/consume IPC, or `control_plane_security.rs`. The normal operator action is one local `enable_output_control_v2` click that atomically acquires and arms exact `{lighting, video}` authority. Release, advanced Arm, Take Over, Add Display, and Force Transfer use a parented OS-native Warning/Yes-No dialog; only Yes proceeds, while No/close is retained as a terminal Forbidden result. All ten mutations use v2 operation IDs and command schema 2; registry wire schema is 4, retired v1 and future v3 operations fail closed, and no Remote/MIDI/OSC/DMX/Web/shortcut route can invoke them. Independent frozen review reported code P0 0/P1 0. Focused Rust counts are control-plane 59/59 after OS-locale coverage, protocol 47/47, OutputControl 19/19, and OutputLease 37/37; frontend invoke inventory is 406, localization is 3536/3536, and Windows default/all-targets, release, tests, and frontend warning ratchets are first-party 0. The first native `1.2.0-alpha.2` release build completed after the exact checkout process count was verified as zero. Executable SHA-256 was `BD10CB4E382883913C52390A4F63094BE5A9402E20AE32DB567D4F1DA81D0C83`, FileVersion/ProductVersion were both `1.2.0-alpha.2`, and PID `96124` provided exactly one responsive `Syndocal` window. The verified window was maximized before automation. One click on `照明と映像の出力を有効化` reached the accessible `出力が有効です` state without a physical-input prompt, Enter, six digits, or a modal. That first display pass confirmed Tauri's physical pixel modes matched `EnumDisplaySettings`; the actual integration defects were Add Display selecting a video-only lease instead of the active exact-Both lease, missing stable monitor identity persistence, and record-only creation that did not publish a native output window. The current tranche fixes those defects and must be rebuilt and exercised before commit.

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

## Historical Local OutputControl R4 checkpoint (superseded)

This historical section records the earlier implementation state only. The
2026-08-21 OutputControl superseding checkpoint at the top of this handoff is the
current contract: the six-digit/Raw Input challenge and its IPC are removed.

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

## 2026-08-21 warning-P2 promotion checkpoint

The warning promotion mechanism and inventory were deliberately separated. Commit
`bd360277592c22a02ec584b7027d816a8a44af72` adds an explicit, inventory-only
`--promote-zero-warning` audit; normal and bootstrap immutability remain unchanged.
The audit requires explicit trusted base/head and configuration ID, exact host and
toolchain, zero diagnostics/warning-shaped output, complete Cargo artifact or
generic marker coverage, no suppressions, and an inventory-only diff. Commit
`98b1f6947125888e17f643d8f5938a4c370b3a22` promotes seven measured rows. Commit
`dd46dd3a9ec50d0e9e88c8db9f1d65079d7e9b7e` adds the final safe rule that a pending
row may gain a stricter required SDK environment while an existing requirement
cannot be removed or changed.

Promotion audits passed on the pinned toolchain for Windows ASIO loader, NDI,
licensed ASIO bridge, frontend production, Windows native release, Linux dev, and
Linux release. All seven report zero first-party warnings. Cargo coverage was 9/9
for ASIO and NDI, 2/2 for the bridge, and 11/11 for each Linux profile; frontend
and native marker coverage was complete with no warning-shaped output. The
inventory is now 11 enforced / 2 pending. Only macOS dev/release remain pending,
so `requiredMatrixComplete` remains false and cross-platform W0 completion is not
claimed.

## 2026-08-21 Windows-first CI and five-display continuation

The Windows warning checkpoint is CI-complete in commit
`85d6eb2b6d7877cd49801c288357d3c72a36046f`. The Windows
job enforces all nine inventory rows: default dev, release, tests, Spout, ASIO
loader, NDI, separately licensed ASIO bridge, frontend, and native release. The
job fails closed unless `FFMPEG_DIR`, `LIBCLANG_PATH`, `NDI_SDK_DIR`, and
`CPAL_ASIO_DIR` resolve to the expected SDK directories/files. It also resolves
and stops only this checkout's `target/release/syndocal.exe` before both the native
warning ratchet and bundle build. The frozen workflow diff was independently
reviewed with P0 0, P1 0, and P2 0. This closes the Windows CI P2 boundary; it does
not change `requiredMatrixComplete=false` because the two macOS rows remain pending.

The exact rebuilt executable was restarted after an earlier PID became
unresponsive. Current native evidence is PID `13652`, exact path
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`, one exact process,
`Responding=True`, one titled `Syndocal` main window, and the window remained
maximized throughout UI work. The header has no Lighting/Video master sliders and
no adjacent controls were reduced.

Five enabled Display outputs are configured in the live native UI against monitor
indices 0 through 4. The visible output names are `Video Output 1`, `3`, `4`, `5`,
and `6`; the skipped label is only creation-sequence naming, not a missing target.
The status surface reports five video outputs. Their native fullscreen windows
remain correctly fenced: `Both` is requested and persisted, but effective role is
still Standby and both lighting/video ownership report startup denial. Output Lease
acquisition and active-role arming intentionally require the backend-issued lease
plus the visible six-digit challenge entered on a physical keyboard. Computer
automation must not synthesize that Raw Input consent. Therefore five-target native
configuration is verified, while successful physical fullscreen placement and the
final editor + LED panel + projector three-screen state still require one coordinated
human-keyboard consent pass. Do not claim those hardware gates complete yet.

Checkpoint validation passed with `pnpm --dir app run
check:warnings:self-test` (61 assertion groups), `pnpm --dir app run
check:release`, PyYAML parsing of `.github/workflows/cross-platform.yml`, and
`git diff --check`. Commit `85d6eb2b6d7877cd49801c288357d3c72a36046f`
was pushed to `origin/codex/syndocal-v1.2`.

## 2026-08-21 HOTONE second-vendor ASIO checkpoint

The connected HOTONE Ampero Mini was tested through the isolated, pinned ASIO
bridge using explicit driver ID `asio:HOTONE AUDIO USB Audio Device`. Its exact
advertised input configuration is 44.1 kHz, one/two channels, native `i32`, and
8..2048 buffer frames. An intentional 48 kHz mismatch failed before Start and did
not fall back. The exact 44.1 kHz / 2-channel / i32 / 128-frame configuration then
passed 100 Start/Stop/Free cycles in 12.7754482 seconds: callbacks 200, actual
buffer 128 on every cycle, Stops 100, Frees 100, warnings 0, terminal events 0,
XRUNs 0, nonfinite samples 0, and fallback 0. This closes the second-vendor stream
gate but not the complete rate/buffer/channel matrix, fault injection, one-hour
soak, physical latency, or ASIO distribution-license decision.

Additional HOTONE one-cycle trials passed for 44.1 kHz / i32 at one-channel
64/256 frames and two-channel 64/256 frames with exact buffers and zero warning,
terminal, XRUN, or nonfinite events. After the separate TOPPING hang below, a
HOTONE one-channel/128 attempt also stopped returning; its exact test processes
were terminated and the result is kept unresolved rather than misattributed or
counted as a pass.

A subsequent bounded TOPPING matrix probe did not convert advertised capability
into a false pass. Explicit 44.1 kHz / one-channel Starts at 64 and 128 frames
failed with the driver's `hardware is malfunctioning` backend error and no
fallback. The next 256-frame trial stopped returning, so the run was cancelled
and the remaining matrix was not attempted. This is recorded hardware/driver
failure evidence; rate/buffer/channel and fault-recovery acceptance remain open.

## Historical 2026-08-21 OutputControl Raw Input checkpoint (superseded)

Work continues on `codex/syndocal-v1.2` from clean upstream base
`d70eaf207930a17900785992ad70cc14e9edab7a`. The previous physical-keyboard
attempt exposed a real Ready handoff race: backend state remained consumable for a
short handoff, while the public expiry and frontend poll still ended at the
original deadline. The current implementation publishes one monotonic deadline
bounded to five seconds beyond the original challenge, keeps Pending expiry exact,
accepts only the bounded Ready extension in the frontend, and exercises the full
prepare/status/execute boundary in the controller harness. Expiry, replacement,
replay, sticky device removal, and one-shot consumption remain fail closed.

The warning/P2 ownership checker now derives candidate ingress from all production
Tauri command bodies, not command names alone. Sorted sink markers cover DMX,
native Display windows, NDI, Spout, and output-ownership transitions. Exact legacy
reject and canonical R4/S0 inventories are enforced, with negative fixtures for an
unknown native command, inventory removal, and rejection after a side effect. The
frozen checker blob is `5f1c15cd2992f71139b4b99f5641b787992b4f4d`;
the final independent read-only verdict is P0 0, P1 0, P2 0, ACCEPT.

Current verified gates:

- Raw Input security: 12 passed, 0 failed, 1 ignored interactive SendInput test;
- legacy output routes: 6 passed; OutputControl: 19 passed;
- frontend OutputControl contract: PASS (8 operations); ownership static gate:
  PASS; DJ Link and safety-blackout runtime contracts: PASS;
- warning-ratchet self-test: 61 groups; release metadata, `cargo fmt --check`, and
  `git diff --check`: PASS;
- final native build: `pnpm --dir app tauri build --no-bundle`, 1m31s, zero
  first-party Rust warnings and zero Vite warnings.

Immediately before that build, exact-path inspection found and stopped zero
processes. The resulting release executable has SHA-256
`D5CC6BF3908AE4295A134B21DDB7CF953D717BFF1AE52E68230EE27C7AC235A6` and
FileVersion/ProductVersion `1.2.0-alpha.1`. It was launched as exact-path PID
`89104`; inspection returned exactly one responsive `Syndocal` process and one
main window. Computer Use selected that exact process-backed app, observed a
1920x1032 client capture and the system `Restore` action, and kept the window
maximized for every native action.

That historical six-digit attempt did not reach Ready before expiry. The feature
was subsequently removed and must not be resumed. Current acceptance uses one
local Enable click for exact `Both`, then measures the LED-panel and projector
native output windows beside the editor.

## 2026-08-21 simple-operation and Scene Matrix continuation

The current uncommitted continuation is on `codex/syndocal-v1.2` at base HEAD
`aa4a7059a1f231fa43004d65b7ebe1a97a0068f6`. It adds three mandatory Windows
completion tranches which must be reviewed and committed together only after their
native gates close:

- Scene Matrix renders all Cue List Banks simultaneously as horizontal columns,
  collapses the two header rows into one jump/action row, prefills and selects an
  unused `Bank N` name, moves rename/delete to an accessible Bank context menu,
  persists Bank drag order through one project transaction/Undo, and makes each
  Bank's `+ Scene` create `New Scene` in that exact Bank even with no fixtures;
- lighting Banks have no privileged `Main`: ID 1 is an ordinary Bank, only the
  final remaining Bank is protected, and deleting any other Bank atomically deletes
  all of its child Scenes rather than moving them. Save/reload must not recreate or
  pin ID 1. This does not change the separate video-composition `Main` model;
- Display output uses detected native targets and a normal select-plus-add path,
  while manual/video-transport settings stay under Advanced. Creation must travel
  through a new canonical OutputControl R4 action; legacy output creation stays
  rejected. I/O defaults show state and the real connect/start/stop action while
  raw configuration remains reachable in closed disclosures;
- the former Raw Input/six-digit/physical-Enter confirmation design was removed by
  the later OutputControl v2 superseding checkpoint. It is historical evidence,
  not a resume action. Normal Enable is one explicit local click; dangerous
  advanced operations use a parented Windows warning dialog.

This paragraph records the state before the OutputControl v2 supersession. The
current remaining Windows output work is an exact-process native rebuild, one
maximized responsive editor, exact LED-panel and projector window/identity checks,
the final three-screen state, documentation refresh, descriptive
commit, and push. No Raw Input, six-digit, Enter, or 15-second challenge remains.

### 2026-08-21 Bank semantics and native checkpoint

The lighting Bank model no longer has a privileged `Main`. New projects create
ordinary `Bank 1` labels for both the first Bank and its playback executor. ID 1
may be reordered, renamed, or deleted like any other Bank; only the final remaining
Bank is protected. Deleting a Bank deletes all of its child Scenes instead of
migrating them. The canonical and legacy engine routes share that exact deletion
path. Save/reload after deleting ID 1 neither revives it nor creates an orphan
executor. Deleted active/group/live/fade/pending/timeline/direct-child references
are removed, including a pending direct-child count-in, and publication rollback
restores that count-in exactly. The independent final review returned P0 0, P1 0,
and P2 0.

Measured gates: engine 767 passed / 2 ignored / 0 failed; protocol 139/139;
strict control plane 73 passed / 1 interactive-only ignored; Scene Matrix,
Scene drag, and Setup I/O each passed all five viewports; frontend invokes 407;
localization 3542/3542; OutputControl, output ownership, empty states, release
metadata, formatting, and diff checks passed. The frontend warning ratchet remained
zero first-party / zero third-party.

Immediately before the native build, exact-path process inspection found zero
running instances. `pnpm --dir app tauri build --no-bundle` completed in 2m20s
with zero first-party Rust and zero Vite warnings. The rebuilt exact executable has
SHA-256 `620EA06C9696EBD6B45E53D42840CD5806EC16D627D2ED3B89BFB0A05A6B1017` and
was launched as PID 53816 with `Responding=True`, title `Syndocal`, and one main
window. Computer Use then failed twice to bind that window with `foreground window
did not report a process id`; no PowerShell/UIAutomation or synthetic-input bypass
was used. Maximize and output placement were unverified at that historical
checkpoint; Raw Input is no longer a product requirement. Resume only the current
OutputControl v2 display checks through the verified maximized native window.

The reviewed implementation checkpoint is commit
`262b8c0f43035ee44ff23cdfc195f4ac1a08b374`, pushed successfully to
`origin/codex/syndocal-v1.2`. The branch and upstream were equal immediately after
push.

### 2026-08-21 Scene Matrix Bank visual-density checkpoint

The follow-up Scene Matrix tranche fixes the remaining visual mismatch without
reducing typography, controls, spacing, or hit targets. Every Bank column now has
the same 156px border-box width, and every top jump button has the same 72px
border-box width with an ellipsis plus the full accessible label for long names.
An empty Bank retains its header and both exact-Bank `+ Scene` actions but no
longer repeats the Bank name or displays the redundant `No scenes in this bank`
message. Scene-card label, left edge, focus/progress treatment, and right edit
strip derive from the containing Bank identity, so moving a Scene between Banks,
Undo, and snapshot reload all follow the destination/source Bank colour.

Measured frontend evidence: Scene Matrix containment and the one-gesture strip
drag suite each passed all five viewports (1920x1080, 1920x1032, 2048x1152,
1366x768, and 1280x720); localization passed 3541/3541; empty-state checks passed;
the frontend TypeScript/Vite warning ratchet remained zero first-party and zero
third-party; `git diff --check` passed apart from line-ending notices. The frozen
independent review found P0 0, P1 0, and P2 0.

The exact-path native release build succeeded after explicitly selecting the VS
2022 MSVC linker and the installed FFmpeg 8.1.2 shared SDK. The resulting
`target/release/syndocal.exe` is 52,851,712 bytes with SHA-256
`5C5FE3242DD2CBBCD2FD738F91497DEAD80D4ACD5761F26780340A6B36AAC3F6`.
It was launched as the sole exact-path process, PID 63996, with
`Responding=True`, title `Syndocal`, and main-window handle 164434820. Computer
Use bound that exact process-backed window, observed the system `Restore` action,
and performed every operation while the 1920x1032 client was maximized. In the
real release app it created `Bank 2` and `Bank 3` by accepting the preselected
default names, then created one `New Scene` in each of Bank 1/2/3. The three Bank
columns were visibly equal and the three Scene treatments matched their purple,
pink, and green Bank colours; empty-Bank redundant copy was absent before scene
creation.

This closes the Bank visual-density tranche only. The later OutputControl v2
checkpoint removed Raw Input and separate `Both` Arm consent. Five fullscreen
output-target checks and the final editor + LED panel + projector state remain
explicitly unverified here.

### 2026-08-21 durable output and release-evidence checkpoint

Branch `codex/syndocal-v1.2` began this tranche at clean/upstream-equal HEAD
`46f8692a64e4fd2f9d8cc2d7c2e7fd0248a45335`. The release checker now requires
explicit manifest-backed candidate mode for RC versions, validates the exact
tag/HEAD/prior-version/clean-tree/artifact set, inspects a fixed copy of hashed
executable bytes, and performs real Tauri signer verification. Its self-test is
65 assertion groups; normal alpha release metadata also passes.

The Output Lease durable journal now covers Take Over and ordinary R4 physical
commit boundaries. Terminal retry bypasses physical/project publication and does
not restore authority; pending retry fails closed before the callback; corrupt,
unknown, invalid, oversized (>8 MiB), and unwritable journal states fail closed.
The journal uses atomic replacement and validates all nested records. The final
independent read-only verdict is P0 0 / P1 0 / code P2 0.

Exact verification:

- full Windows no-default Rust tests: 806 passed, 0 failed, 5 ignored;
- subsequent no-default `cargo check`: zero first-party warnings;
- Take Over focused 9/9, durable focused 3/3, in-doubt focused 1/1;
- release self-test 65 groups, release metadata, node syntax, formatting, and
  `git diff --check`: PASS;
- frontend TypeScript/Vite warning ratchet: 0 first-party / 0 third-party.

Before the final native build, exact-path process inspection stopped zero
instances. The no-bundle build completed in 1m53s. The 53,442,560-byte executable
has SHA-256
`E28FF0260A9A2781BA6F072057C67DB18B53BA1139CD3065497BF3FB28D58ABF`,
FileVersion/ProductVersion `1.2.0-alpha.1`, and its compiled runtime diagnostic
returned beta endpoint/channel identity with exit 0. It was launched as exact-path
PID 83004, `Responding=True`, title `Syndocal`, with exactly one main window.
Computer Use found the window restored, invoked Maximize from the native system
menu, and then captured the sole 1920x1032 maximized Syndocal window.

Do not resume the retired Raw Input/six-digit/Enter flow. The current OutputControl
v2 path is one local click to acquire and arm exact `Both`, followed immediately by
Add Display using a detected target's index plus stable identity. Verify all five
native output windows, then leave the editor + LED panel + projector state. The
advanced dangerous operations alone require the parented Windows warning dialog.

### 2026-08-22 expired-lease Add Display false-freeze checkpoint

Branch `codex/syndocal-v1.2` remains at upstream-equal HEAD
`eb4a70c8796f5fdd300d4a8cea23a0e16b0f4f1a`; this checkpoint is still an
uncommitted integration candidate. The reported Add Display "freeze" was not an
OS hang in the inspected process: the exact Syndocal/WebView process answered a
window message in 54 ms and no native dialog was open. The visible failure was
`Exactly one active output lease ... is required` after the exact-Both authority
naturally passed its 60-second TTL. The old public authority query intentionally
hid that expired record, so the frontend rejected Add before IPC and displayed
the error far from the Add control.

The normal Add path now uses the dedicated read-only
`query_display_add_lease_authority_v1` endpoint. It truthfully distinguishes
`held_active`, `expired_recoverable`, `held_orphaned`, and `unavailable`, and
returns authority only for exactly one globally overlapping lease with the exact
renderer/window/process/owner incarnation, current project identity, and
canonical `{lighting, video}` resources. The query is async/offloaded, bounded by
try-lock failure, and changes no generation, audit, or durable state. The single
subsequent `add_display_output_v2` request can authorize an active exact lease or
privately recover the exact naturally expired/orphaned lease in the same durable
candidate. Public Acquire/Recover semantics are unchanged. The Add surface now
shows pending/success/failure adjacent to the button and prevents duplicate
submission.

Backend-first verification is the default repeatable path. The production-faithful
driver constructs the same canonical v2 request and executes the same durable,
lease, engine, and project publication boundaries as the frontend; only the native
shell/worker is injected. Computer Use is reserved for the final maximized native
placement and hardware-visible acceptance.

Measured evidence on the frozen candidate: control-plane 61/61; output-lease
46/46; exact natural-expiry query-to-Add recovery 1/1; four-display canonical
driver 1/1; frontend invoke inventory 407; localization 3538/3538; TypeScript/Vite
build; focused display and output runtime contracts; no-default Rust check with
zero first-party warnings; frontend ratchet zero first-party/third-party; format,
Node syntax, and diff checks all pass. Final independent review and a fresh native
no-bundle build are still pending. No current native placement is claimed: editor
+ four sub-displays remains 0/4 verified. Product-roadmap progress remains 13 of
79 checked items (16.5%); this bounded output tranche is not product completion.

### 2026-08-22 current-source native Add Display hang correction

The preceding false-freeze diagnosis described the earlier executable only. A
fresh current-source native build disproved completion of the new Add path.
Immediately before the build, exact-path inspection found no running instance of
this checkout's executable. `pnpm --dir app tauri build --no-bundle` completed in
1m59s. The resulting 54,498,816-byte
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` has SHA-256
`30DD06DBD3642B6CE3D31E403FCDB6AA9A35D9D419929B4D744A837D3509C06E`.
It launched as the sole exact-path process, PID 81160, with one responsive
`Syndocal` window, and that exact window was maximized before any UI operation.

One-click Enable reached the authoritative `Output enabled` state. After waiting
beyond the 60-second lease TTL, Add selected the default non-editor target
`\\.\DISPLAY5` at 1920x1080 and displayed the parented Japanese native warning.
After Yes, no output window appeared, the adjacent Add state remained pending,
and the exact process became unresponsive. The durable journal proves that the
current renderer's request 1 reached terminal `Acquired`, while request 2 reached
durable `Pending` with no terminal receipt. The process was then terminated by
its already-verified exact executable path; no unrelated process was touched.

Static tracing matches the measured approximately 10-second transition to Not
Responding. Add retained lifecycle, external-admission, project-coordinator,
ownership-transition, and lease-registry guards after durable prepare while it
performed native window, engine, synchronous GPU-device, and first-frame work.
The frontend recovery timer then invoked synchronous
`get_project_checkpoint_bundle` on the Tauri event loop and blocked on the same
project coordinator. The repair stop condition is therefore a phased transaction:
durable Pending first, unpublished native/GPU preparation with no long-lived
project/transition/lease guards, then a short final exact-fence revalidation and
publication. The checkpoint query must also run off the event loop and fail
boundedly under contention. Safe rollback may clear Pending only after complete
native/engine cleanup is positively acknowledged; ambiguous cleanup remains
in-doubt and blocks replay.

Do not repeat this path through UI while implementing. Use the production
backend transaction seam for the stalled-phase, expiry, stale-generation,
replay, and four-display matrix. Computer Use is reserved for one final rebuilt,
maximized native session because physical Windows placement and native modal
parenting cannot be proven headlessly. Current native display acceptance remains
0/4 and the product checklist remains 13/79 (16.5%). No commit or push is allowed
until the phased fix, independent review, full focused gates, warning-zero checks,
and a new native acceptance pass are green.

### 2026-08-22 phased Add Display final Windows acceptance

The phased Add correction is accepted on branch `codex/syndocal-v1.2` at the
still-upstream-equal pre-commit HEAD
`eb4a70c8796f5fdd300d4a8cea23a0e16b0f4f1a`. The frozen implementation hashes
are `main.rs` SHA-256
`2B78C6FF639125AB87E0B03ACE51B00D5DCFC992436DB17C93CC4323E9CD7CA4` and
`control_plane_runtime.rs` SHA-256
`2AD1BCFFC64EB5E9A54C1112E53BAEC6FC30E77DF4516430A23A7B51F68F3D61`.
Independent read-only review reports P0 0 / P1 0 for this tranche.

The production Add wrapper, the four-display driver, the stalled-phase test,
and the terminal-record-fault test now enter the same full
`add_display_output_with_output_control_fence_core`. Native preparation is
unpublished and deadline-bounded outside long-lived authority/project guards;
final publication revalidates the exact owner, project, monitor, output/safety
fence, and exact-Both lease. Worker/metrics insertion and start-gate release use
one shared state machine for the Tauri and injected implementations. Ambiguous
timeout or cleanup retains durable Pending and blocks replay; acknowledged total
cleanup alone may SafeAbort. The checkpoint query is async/off-event-loop and
returns bounded Busy under contention.

Immediately before the final build, exact-path inspection found zero running
instances of this checkout's executable. `pnpm --dir app tauri build
--no-bundle` passed. The resulting 54,403,072-byte executable is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`, SHA-256
`C88A1DE53812DD773FE4AE28079977202104F33F63B67054119DD338C1EF2BAA`.
It launched as the sole exact-path process, PID 56460. The verified main
`Syndocal` window was maximized at 1920x1032 before every final UI action.

One-click Enable reached `Output enabled` and remained responsive across a
greater-than-ten-second recovery interval. Four sequential canonical v2 Add
requests used the four non-editor targets DISPLAY5, DISPLAY6, DISPLAY1, and
DISPLAY3. Each displayed the parented Japanese Windows Warning/Yes-No dialog;
each Yes terminalized and produced one live native window. Final enumeration is
exactly one 1920x1032 editor plus four responsive black output windows titled
`Syndocal Output - Display 1`, `Display 2`, `Display 3`, and `Display 5`.
Captured output surfaces were 1280x720, 1536x864, 2560x720, and 1707x960 after
Windows DPI scaling, while the authored physical modes remained 1920x1080,
2560x720, 2560x1440, and 3840x2160. The editor monitor was not used.

The current renderer origin `renderer:0ab58f46-0e39-4537-bafe-0e25eeb1dae0`
has request 1 `Acquired` and requests 2 through 5 terminal `Authorized`, with
`replay_guard=false` and no current-origin Pending. Two older origins retain
historical fail-closed Pending evidence and are not current authority.

The post-commit stale-message P2 is closed in `App.tsx` blob
`44a1f7c25fa668e4f105ea12e7ac8076caee7dcc` and
`check-video-display-target.mjs` blob
`506fc9834f129835375c5839f6f0477317e67a4f`. Add mutation success is now fixed
before refresh convergence: the two exact transaction/publication messages are
cleared only after refresh settles, an unrelated refresh failure is reported as
`Display output added; refresh pending`, and a real Add mutation failure still
rejects beside the Add control. The checker executes the extracted production
classifier and proves mutation failure, known-stale refresh rejection,
controller-caught stale status, and unrelated refresh failure separately.

The final P2 native rebuild first encountered a Windows Defender ML false
positive (`Trojan:Script/ObfusScript.A!ml`) against Tauri's generated minified JS
asset. No antivirus protection or exclusion was changed. The same production
code was rebuilt successfully with an unminified Vite asset and a temporary
no-before-build Tauri overlay, which was deleted immediately afterward.
`pnpm --dir app tauri build --no-bundle` then produced a 54,587,904-byte release
executable with SHA-256
`977D3D63A60E77ACE9E6A6E1CA91652EC0356E8389367921DAB7032894DA0DCB`.
The sole exact-path process, PID 87060, exposed exactly one responsive `Syndocal`
window; it was verified maximized at 1920x1032 and remained responsive after a
greater-than-ten-second wait. Repeated four-display proof remains the canonical
backend driver plus the immediately preceding physical 4/4 acceptance; the P2
rebuild intentionally did not repeat four manual Add clicks.

This closes the Windows phased-Add/five-screen tranche only. The authoritative
product roadmap remains 13/79 (16.5%); it is not a whole-product completion
claim. Repeated regression validation must continue through the canonical
backend driver. Computer Use remains limited to the final physical placement,
native modal, and visible-window acceptance pass.

### 2026-08-22 E1 generic project-transaction final checkpoint

Branch `codex/syndocal-v1.2` started this tranche from upstream-equal HEAD
`41ab143d7926b2d13f424f3fe2266870680f2524`. E1 now closes generic Begin
reply-loss, terminal query/adopt/ack recovery, live-owner transaction liveness,
renderer retirement, and stale delayed Commit/Cancel rejection. Requests bind a
strict client operation ID and shape to the backend-issued owner/window
incarnation and current project authority. Retirement produces exactly one
Interrupted history entry for a partial mutation, no entry for no change, and
does not discard retryable mappings on failure. Same-label owner ABA is blocked
by a bounded 1024-entry retired-binding tombstone with fail-closed capacity.

Frozen source SHA-256 values are:

- `app/src-tauri/src/main.rs`:
  `1FDD6AE23647D021CDB20FA9C0E8CD925F52B6875E69FD3ED63BB65AE91CB7EB`
- `app/src/App.tsx`:
  `079F0DDD6894F5DDFF3ED74C5CF5D57104A45D86B090CE51A06F2E81DE9FD5FC`
- `app/src/types.ts`:
  `474CCC6C6E20B25CB79DB1901FCF8507E21404FA4851CD57AE2EFAA2A45102FB`
- invoke manifest / typed tuple:
  `55D1B6D8A3CE4CFF20A90F6291689BEC6A6C425476799E3F1532226BF43AE07A` /
  `9B46C5CD11BFB67812990F9EE5C590BF7A3004C50ACA3FE845AB7ACF1698F9E2`
- project-authority / transaction checkers:
  `70E6239AB1B223F437FE76919F396716FEE3E9E4B7212F96D0942D12751CC70A` /
  `2D052455343CBBC5E37D8F10E46D9679C61C234C1C25F0AACEF619DEE1D48551`

Evidence: `check:project-transaction` PASS; exact frontend invoke inventory 410;
4/4 focused project-transaction Rust tests; focused partial-cancel Undo 1/1;
no-default and default-feature Rust checks; TypeScript/Vite build;
`check:release`; localization 3538/3538; empty-state; frontend warning ratchet;
format, Node syntax, and diff checks. Independent adversarial review returned
P0 0 / P1 0 / P2 0. First-party warnings are zero for the no-default check,
default-feature check, release native build, and frontend ratchet. The structured
all-target warning runner was attempted but failed in its controlled build-script
environment because it could not use the absolute MSVC linker; do not count that
attempt as a completed warning row.

Immediately before the native build, exact-path running process count was zero.
`pnpm --dir app tauri build --no-bundle` passed in 2m29s. The resulting
54,958,080-byte `target/release/syndocal.exe` has SHA-256
`0AACEA71AC666329A56DFBD57515650621E68DCD4D1B54E3ABA37918636C818C`.
It launched as exactly one responsive exact-path process/window titled
`Syndocal`; the verified window was explicitly maximized at 1920x1032. Because
E1 changes transaction recovery rather than a visible workflow, no redundant
Computer Use mutation sequence was repeated.

Current progress is E1 100% and the active Windows-only product checklist is
14/71 (19.7%). This is not a
whole-product completion claim. Resume at E2 authority bundle/generation
consistency, then E3 restart-durable recovery and E4 Save/Save As. macOS warning
rows and all external/hardware acceptance outside this E1 boundary remain open.

### 2026-08-22 Video Setup truth and simplification checkpoint

Current source inspection confirms that the Video Setup inspector still exposes
legacy output mutation buttons whose Tauri handlers intentionally fail closed,
including enable, blackout release, opacity/fade, mapping, open, sync, and
remove routes. The projection-mapping canvas is an advanced projector alignment
surface, not a required part of the ordinary detected-display workflow. It must
move behind an initially closed semantic disclosure with a plain explanation;
no typography, control, or hit-target shrink is authorized.

The saved `VideoOutputSummary.enabled` state is also not physical window truth.
Native CloseRequested/Destroyed handlers currently stop their renderer worker,
while the setup list continues to describe the authored output and receives no
immediate close event. The next integrated video tranche therefore has one
truthful normal surface: authored desired state, actual live-window state, and a
single canonical reopen/close action. Manual window close must originate a
backend native-window event/status refresh, retire worker/metrics truth exactly
once, and update the selected output inline. Legacy commands that always reject
must not remain as actionable controls; diagnostics and test-pattern controls
belong under Advanced only when backed by a working canonical route.

Routine proof for this tranche must drive the same registered Tauri/control-plane
production operations as the frontend. Repeated Computer Use is forbidden; the
only UI pass for the active show-core is the final rebuilt, maximized editor plus
two physical VJ output windows. At this checkpoint the accepted numerator remains 14/71. E2/D1,
physical-pixel 4K/DPI correction, and Video Setup/window-lifecycle integration
are still in progress and do not advance the product count.

#### Accepted implementation evidence after the checkpoint

### 2026-08-22 Edit domain and unified Timeline correction

Current source still presents `Lighting` and `Video` as the only persistent Edit
domains. `Video` swaps to a standalone dense mixer containing Media Library,
Preview/Program, Outputs, Layers, FX, and automation, while Timeline is retained as
a hidden Lighting-context mode. The operator rejected that information architecture.

The accepted target is one Edit desk with `Lighting / Video / Timeline` domains.
Lighting's upper surface remains Banks/Scenes; Video's upper surface becomes Media
Library; Timeline is one arranger containing Lighting, Video, and Audio layers.
The two lower panes retain their layout and change contextual content only. The
lower-right pane shows lighting attributes, selected video/media properties, or
selected Timeline-placement properties according to the active domain.

Domain ownership is explicit: Lighting authors reusable lighting Scenes; Video
imports/previews/organizes media and authors reusable clip/source properties;
Timeline sequences those sources. Video is not a second arranger and not the
physical display setup page. Detected displays, fullscreen output creation, and
projection calibration remain under Setup Video. The current dense Video mixer
may survive only as advanced disclosed tools backed by working canonical routes.

Timeline must render a co-visible source pane in the existing lower-right desk
position while the arranger remains in the upper pane. It switches between the
same `Scenes` and `Media Library` project sources (with All/Video/Audio media
filters), with a sibling Inspector view for the selected placement. Sources is
the default authoring view. Dragging from a different persistent Edit tab is not
the product flow because the target would not be visible. The pane reuses source
identities and drag contracts rather than maintaining a second catalog, and keeps
an accessible keyboard placement action for each item.

The active implementation tranche is the prerequisite unified DnD contract:
Lighting Scene -> Lighting layer and Media Library asset -> compatible Video/Audio
layers through existing authoritative mutations, with one grouped Video+Audio
placement for an audiovisual asset. The navigation/desk refactor follows after
that tranche freezes and passes independent review. Do not perform intermediate
Computer Use passes; rebuild once and reserve native UI operation for the final
three-screen acceptance: one maximized editor/control window plus one LED-panel
output and one projector output.

The pinned final native sample is
`C:\Users\kouty\Downloads\06.flash back背景途中経過02.mp4`, SHA-256
`70C2B6C9D9F7F0F687E309C3207E9EEB78FDADCD738D1980165A643F15A45012`.
Current `ffprobe` evidence is H.264 1920x768 at 30000/1001 fps plus AAC 48 kHz
stereo, duration 84.3843 seconds, size 146,312,255 bytes. Import it once through
the Media Library production transaction, then place it through the unified DnD
route and verify one grouped Video+Audio placement. The same video image is the
temporary content for both the LED-panel and projector outputs in the single
final native pass.

The 4K/DPI and D1 backend slice subsequently passed independent frozen review
with P0/P1/P2 all zero. It now treats authored dimensions as physical pixels,
converges the post-show client/surface extent at 100/125/150/200 percent DPI,
uses `PhysicalSize` for windowed outputs, follows the live physical client size,
and fails closed on convergence timeout or staged monitor-topology change. The
focused native-video suite passed 23/23 and the no-default Rust check reported
zero first-party warnings. D1 cache listing no longer creates a missing cache
directory and the retired custom-fixture compatibility command is pure preview.
The reviewed git blobs were `main.rs` `5676eea3616cf7f060206e71babe633ff6e5fbfe`,
video checker `2cc88d90c59af8f1c2263e800563a81efe9ffc29`, and fixture
checker `62bbed6bf5586cbae4670fa5b6d6f054b0797a80`.

E2 authority/runtime application and the D1 frontend preview-only boundary also
passed independent frozen review with P0/P1/P2 all zero. The production module
is imported by App event, reply, poll, and fallback paths; its executable harness
covers delayed lower input generations, duplicate/reordered replacement,
mapping hydrate/rebase, stale recovery consumption, and startup recovery intent
delivery. Final SHA-256 values were App
`146A07F3812EC92A2041B3435EA9C6D83DFEFBC2A0839B5D0508EB1EEC153F3D`,
authority runtime
`911D0C623AA05587A31FE6C5CE692242701BFA9EE6B72B85907DBC4E7665A884`,
and checker
`D7DEA3AA8C24B054A9A7E6856B754E513E4D4D5420FA2DD2CF0C3E36B662F555`.
Project-transaction/authority, localization 3538/3538, fixture catalog 69,
TypeScript, Vite, and diff gates passed. Neither accepted slice has yet had the
required integrated native release/UI pass, so the product numerator remains
14/71 until the current Video Setup/window-lifecycle bundle is integrated.

The Video window backend slice subsequently reached a second independent frozen
review with P0/P1/P2 all zero. It adds the sole canonical live-window mutation
`set_display_output_window_open_v2`, exact-Both acquire/recover authority, strict
wire/registry binding, and the schema-1 `syndocal://video-output-window-state`
event. The shared AddDisplay/reopen Destroyed observer claims the exact monotonic
window incarnation before cleanup; delayed A cannot retire reopened B, and a
join timeout retains metrics/quarantine truth until acknowledged reaping. Final
SHA-256 values were `main.rs`
`4A303FD730F0347B9932F1813ED1F8287C324BA326EFF3C81BB208EFB22BFC79`,
runtime `131656CC5D958EB8D3B55EB35392631DA6B819AFB932A8BA435AC2AADBAC9EF0`,
control-plane `1C6E8535CB27A7AA885329B3C941B81994E70D207840E3629CCF2439ECF78FC8`,
and protocol command
`6790F622FB94AB421AAE0E95EB8D76C257D8EBD8563AF05DBF82A8FEA7126AD5`.
Protocol (11), control-plane (25), output-lease (31), the production callback
seam, both OutputControl/video-window Node gates, no-default Rust check, fmt,
and diff-check were green with zero first-party warnings. Native HWND proof is
still deliberately pending the integrated frontend build.

The operator also confirmed that a real fixture/serial DMX interface is available
without an external scheduling blocker. Lighting scene, Static, Bank, FX, and
Timeline authoring plus serial/Art-Net/sACN output are already implemented; do
not count them as missing features. Remaining lighting acceptance is the batched
production-path fixture run (channel values, 44 Hz, blackout/release, reconnect,
and soak) followed by the one final maximized integrated UI pass.

### 2026-08-22 unified Timeline DnD and real-media checkpoint

The backend `InsertMedia` tranche is frozen at `app/src-tauri/src/main.rs`
SHA-256 `3A34268A3781BC54423AAEF84444D9F7750792BC26DF48B467625BF0D3BA1F30`.
Independent review returned P0/P1/P2 all zero and the focused no-default suite
passed 3/3. Explicit Video and Audio lane targets are exact command meaning;
wrong-kind, missing, locked, or stream-incompatible lanes reject before any
allocator, snapshot, history, transaction, or publication delta. AV placement
creates one linked Video+Audio group and one history entry. Exact retries are
idempotent and a changed target under the same request identity is a shape
conflict whose canonical receipt remains queryable.

The first frontend freeze was not accepted. Independent review found no P0 but
four P1 and two P2: a generic target-less Add-to-Timeline route remained in the
Video panel; selected Video/Audio companion lanes were not both propagated;
stale Scenes reached the placement callback; the frontend warning gate still
expected a variable literal module count; and the new source shelf lacked
complete semantic/five-viewport proof. These are being fixed as one batch before
the `Lighting / Video / Timeline` desk refactor resumes. The warning work uses a
new explicit, inventory-only output-marker rebaseline audit; it must not relax
normal inventory immutability or the pending-to-enforced promotion path.

Read-only tracing of the pinned MP4 production path found a separate real-playback
blocker: Timeline Video lane IDs and runtime VJ `VideoLayerId` values are allocated
in different domains, while the current engine treats them as the same ID during
`apply_timeline_video_clips`. Therefore an import and authoritative Timeline
receipt can succeed while actual playback reports a missing Video layer. The
active engine repair must make `media_asset_id` the Timeline clip source authority
and provide a distinct, non-persisted runtime projection (or an equivalently
explicit persisted mapping) without conflating the two ID spaces. Completion
requires real source resolution, linked AV seek/play/pause coherence, no ephemeral
runtime state in project persistence/history, and a backend driver that exercises
the same staged import, availability, Timeline receipt, transport, and output
cores. Native HWND/GPU proof remains reserved for the final single maximized pass.

The repaired frontend DnD freeze subsequently cleared both independent P0/P1
reviews. `VideoControlPanel` no longer owns a generic target-less Timeline
insert or drag entry. `TimelineSourceShelf` is the sole visible source entry and
propagates the operator's exact Video and Audio lane IDs; the shared runtime
revalidates the current Scene/MediaAsset and both lane identities, kinds, and
locks before calling the one authoritative placement callback. There is no
first-unlocked companion fallback, and stale Scenes are rejected with callback
count zero. Pointer drop and keyboard/click placement use the same runtime route.
The accepted freeze hashes were App
`2A7A6AD1AEB70A6152C256D1DA9C0DE824A95E12A2F30C5FD1D8B2E79F5F87E0`,
source shelf
`38C928163FC93600F277B99FE7DF3E160E58DE88DF6DB53752C9A9733BDCEAE0`,
drag payload
`1205CBF31946D6365D6E400CCE28BB4A53DC112697F522B7CD7C92C79B4E3AEE`,
drop runtime
`37C1D7F41BB8D9E39239F57F7BF01CD14E55505284640F53847976EAB87C9296`,
and focused checker
`7E259EDA3D4C5D9663307467A3CC527828CE62222D4DA0B1EDCECCF412433C97`.
The focused DnD, localization, empty-state, invoke, build, and source-shelf
viewport gates passed. Two proof-strength P2 assertions -- Audio-origin AV
placement and a dedicated locked-lane callback-zero case -- are being added to
the active three-domain Edit IA checkpoint; the source switches intentionally
remain semantic pressed-button groups rather than claiming tablist semantics.

The warning output-marker rebaseline implementation also passed a second
independent review with code P0/P1/P2 all zero. Its dedicated audit now requires
the caller's explicit base and head, verifies that exact base is an ancestor,
and never substitutes a merge base. Normal ratchet and pending-to-enforced
promotion behavior remain unchanged. The executable self-test covers the real
A-to-B inventory-only rebaseline, a B-to-C normal gate, non-ancestor rejection,
multi/broad marker changes, Cargo/pending and sibling configuration changes,
toolchain/evidence drift, inventory-external files, and warning-suppression
attempts. `check:warnings:self-test`, Node syntax, diff-check, and the frontend
build passed; Vite transformed 268 modules with no warning-shaped output. The
accepted script hashes were
`33AF03258F7CF9DB68E061370194D65E197E97808446AF6AEC7D7AB5396B1358`,
`10E9DACFB820AC2EED6C21688D446AF0F518BEC4349CD46996B75B4FDC6C3736`,
and `C4CC9F83844CB92CAC4BB9B4CDD311F3F9E439E99EB6F450330E28BB8B5A4CD9`.
Final operation is deliberately deferred until all feature files freeze: commit
A with feature code plus audit implementation, commit B containing only the
inventory marker/evidence update and run the explicit A..B audit, then create an
inventory-unchanged child C and run the normal B..C gate.

### 2026-08-22 final Edit IA and pinned show-audio checkpoint

The three-domain Edit IA is frozen and independently accepted with P0/P1/P2 all
zero. `Lighting / Video / Timeline` are the persistent domains; Lighting owns the
Bank/Scene matrix, Video owns the Media Library plus thumbnail and selected-media
properties, and Timeline owns one upper arranger with the co-visible lower-right
Sources/Inspector shelf. Scene and MediaAsset pointer and keyboard placement
share one exact-lane production route. Wrong, locked, stale, missing, unreadable,
hash-mismatched, and live-only sources reject before a mutation callback.
Timeline pre-callback and runtime rejection strings now pass the App locale
boundary; Japanese output does not expose internal availability enum tokens.
The final focused gates passed DnD orchestration, localization 3530/3530,
TypeScript, frontend invokes 411, five source-shelf viewports, five pane-window
viewports, and diff-check. Final reviewed hashes were TimelineOverview
`5C181D051C5EFAFB0B9993190F6F5012676BD6BA519D7022DF88B282D20D383A`,
App `0FBE4BFE4AD3B88E1D7C28A974F83888CE26C6D54454FA93343BEB093B36015E`,
localization `66F29B4BA44EFBD9FA13BD17F94F414E70EB80003914CC8FEAD2F49C85DE9BB9`,
and DnD checker `0BE8D1D967D986314F45440265232D644AD621494F05B2C8F3139FA5635AC3E3`.

The operator confirmed the show chart as `人生オーバー` 156 measures, all 4/4,
despite the separate FRET STEP definition still ending at 151. Its Guide chart
announces Intro, Verse, Pre Chorus, Chorus, Interlude, Breakdown, and Outro on
the preceding measure's final beat, except Intro at frame zero. Measure 98 is
performed for eight total passes with `Looping` on every pass. `Bridge` is
deliberately suppressed, and `Break` targets measure 99 from its preceding beat.
Measures 149-156 ramp phase-continuously from 170 to 194 BPM with `Trans` targeting
measures 149, 151, 153, and 155. There is no song overlap: `Complete` begins on
the final beat of `人生オーバー` and continues uncut across the identical frame
where `惑う星` begins at 194 BPM. The conflicting `惑う星` Intro is suppressed;
its Verse, Pre Chorus, Chorus, Interlude, and Outro calls retain the same
preceding-beat rule over the pinned 207-measure 4/4, 5/4, and 6/4 map.

The reproducible exporter and Zira generator are
`tools/audio/export-jinsei-madow-click-guide.mjs` and
`tools/audio/generate-guide-complete.ps1`; `complete.wav` and `interlude.wav` are
now in the embedded English Guide source pack. Canonical audition files are in
`C:\TEMP\syndocal-show-audio`. Independent PCM review accepted P0/P1/P2 all zero:
all nine WAVs are 48 kHz PCM16 mono, total click count is 1,492, and physical and
semantic Guide event counts are both 33. Thirty-two Guide onsets are exactly one
beat before their target and the only exception is the frame-zero `人生オーバー`
Intro. The song boundary is frame 11,010,639, connected duration is 489.182125
seconds, fresh-export WAV hashes are identical, all voice intervals are globally
non-overlapping, and all mixes remain below clip.
The canonical manifest SHA-256 is
`FA2613B575E0AB68026E3A91AB49ABF6418FDDD2DE2544A5C26F4440ED24CF58`;
exporter, generator, and Interlude asset hashes are respectively
`8516F8C18099CF21D32C419E0774116CEE99ABBB06B1854518A31DED8BFD9C7A`,
`C13A377E926790AAE4006252738A30272CE5C7DB712586CBFE654CC19ED37259`,
and `695E2ADDF83D1F3C236E14B57BF20124269B587A098293293A4EE9CAB99EAE21`.
This is a pre-rendered material checkpoint only. Runtime click/Guide completion
still requires the shared sample-frame tempo/meter scheduler and discontinuity
generation fences.

Audio-lane mute/solo, Follow settlement, and sink-generation work is accepted at
this checkpoint. The final production worker uses a semantic audio generation,
exact Follow-context attribution, device-fenced single-flight preparation,
bounded quarantine/recovery, and a typed `BudgetExhausted` fault from worker to
coordinator. Potentially blocking open/decode/seek work occurs outside the shared
media-audio mutex; the final commit revalidates publication, position, device,
deadline, and timeout/commit linearization before installing a sink. Independent
review accepted P0/P1/P2 all zero. Focused evidence passed 31/31 media-audio,
12/12 engine timeline-audio, Follow 1/1, child-quorum 1/1, no-default Cargo check
with zero first-party warnings, fmt, and diff-check. Stable hashes are engine
`C9FB7C0AA033A7CC4C03C4A3928B86CF83B390247154861A256D9467FB106EEC`
and main
`D85641CF45731C9F2C5C773A1F4AA0A6619632EA75F3753D4B354D4BE963904C`.

## 2026-08-22 alpha.4 cue-audio integration handoff

- Branch/implementation checkpoint: `codex/syndocal-v1.2` /
  `0a3e91ab87d4be4735710234d1ba002e484375ef`. Its parent was
  `9b0bd7e1755571037c9ff552e880467f6ee47f8d`.
- Product metadata has advanced consistently to `1.2.0-alpha.4` across the
  workspace manifest, first-party Cargo.lock entries, frontend/Tauri manifests,
  release checker, artifact names, README, completion flow, roadmap, and release
  status. No project/command/API/ABI/asset schema version was changed.
- Independently frozen scheduler hashes are protocol
  `48FF684AFD66680DC97F1AB5447F5FAFF920E63A5609D9ABF1FC6F6CF502BD1B` and Engine
  `0031D9EC1AB5004779A9C28A2665DF2BC14D6122B1FAE32B40DFCB15D5735F66`.
  Engine validation was 805/805 with zero failures, two ignored tests, one
  known filtered case, and zero first-party warnings.
- Final independently green native integration hashes are main
  `E69A8989E7AE0D030C7AB3FE0AA31B2D36075CB22FADA0382EDDFE99D5214750`, cue core
  `D6C1A18FD09E98AB7CA849EE439AFEF93C67C700E42466CB5B4C6854F90C1927`, and DVC
  `B81913413A4796A37B4F6FE5A146CB75114AB20F0BE1A74B700A4A3C226F3546`.
  Independent review reported P0/P1/P2 zero; the old polling/Sink production
  route is absent and exact clock/topology/settings/lifecycle fences are live.
- The authored two-song material remains 33 Guide events, natural
  `playback_rate_milli = clamp(round((1 + (BPM - 170) / 600) * 1000), 920, 1080)`
  (170 BPM = 1000; 194 BPM = 1040), `Trans` at 149/151/153/155 every two
  measures, and Complete-only settlement collision with destination Intro
  suppression.
- Final automated evidence is green: Cue Audio 41/41, DVC import 105/105,
  protocol 143/143, frontend TypeScript/build, 411 invokes, 3541/3541
  localization, Cue runtime/browser, the four-viewport Timeline Performance
  matrix, and Windows default/release warning ratchets at zero first-party
  warnings. The one control-plane inventory correction passed its exact test.
- Immediately before the final build the exact-checkout process count was zero.
  `pnpm --dir app tauri build --no-bundle` passed in 2m37s and produced a
  56,342,016-byte `1.2.0-alpha.4` executable with SHA-256
  `AB98EA14F8439E23CC2E3BD80A4041C2B9CC82244A68B6938A3E9B2515A87297`.
  One responsive exact-path window was launched and maximized. Edit > Timeline
  displayed Click, Guide, Follow waiting, shared lanes, and the source shelf
  without a runtime fault. No project or output setting was mutated.
- Remaining external acceptance is audible playback plus the editor + LED panel
  + projector operation, fixture/serial-DMX, MTC/DJ Link, audio device, and soak/
  recovery rows. Do not infer those from the native UI smoke.

## 2026-08-22 alpha.5 E3 recovery durability automated handoff

- Branch/starting HEAD: `codex/syndocal-v1.2` /
  `396f9f207bb264a39bce29869f7d7f54d93096ad`. Product metadata is advancing to
  `1.2.0-alpha.5`; no project, command, API, ABI, recovery-storage, or journal
  schema version changes with this product ordinal.
- E2 authority generation/bundle consistency and D1 cache/read-purity were already
  included in the independently accepted alpha.3 tranche. Current project-storage
  and project-transaction/authority checks remain green, so their previously stale
  completion-flow boxes are now reconciled as accepted.
- E3 uses one production orchestration. App and the JavaScript driver import the
  same publication/startup/intent-consumer functions. The registered Rust load
  and ACK commands call the same external-admission, standby lifecycle, durable
  journal, output-retirement, engine-publication, coordinator-commit, and event
  service exercised by the Rust process-boundary test.
- Stable pre-version-bump implementation hashes are main
  `58C0E9F64C6A53EADF54AE5A475A58F0FA8FC46549418CDFB9A9148EA118764C`, App
  `A53767D042D0B9D37FDA3EA6E8A5C7BD50124CC6EED726B630A903C6AF1AAABF`, runtime
  `EFD6AC1392DC567A896500CF365CBC4FA46C013CB9E690A0AAD74E01687DF14B`, storage
  `C90ACC544375958EFAD6694B5167E7B3CBEF69DEB96BBE557BF35FA337A92D99`, E3 driver
  `C25B901AED824C4CF93C4AF73396685E190ADA2B379F82801A369891B9359FE9`, and package
  `E97C76F1DB583D648528D7EE046DD953B7C0D2C3FA7836FC2EC25665C96F9077`.
- Two independent reviews report P0/P1/P2 zero. Automated gates pass: E3 driver,
  project transaction/authority, project storage, TypeScript, Vite build, exact
  411 frontend invokes, 3541/3541 localization, Rust E3 1/1, project-recovery
  authority 2/2, recovery-authority 3/3, no-default Cargo check, Rust format,
  diff-check, and frontend warning ratchet at zero first-party/total warnings.
- Automated production-core evidence is not native process-kill evidence. Before
  E3 is checked complete, one final maximized Windows operation must exercise the
  real Tauri/localStorage path at B-published/reply-lost kill/relaunch and at
  ACK-durable/browser-cleanup-pending kill/relaunch. Record exact alpha.5 build,
  executable hash, process/window, and recovery outcomes below this handoff.
- Native alpha.5 repair checkpoint: exact-path PID `113004` was the sole old
  checkout process and was terminated before building. After exposing the existing
  LGPL shared FFmpeg SDK through `FFMPEG_DIR`/`PATH` and LLVM through
  `LIBCLANG_PATH`, `pnpm --dir app tauri build --no-bundle` passed. The executable
  SHA-256 is `F318FAEBEFBA88B03EC179DF34395E34F5BD8D960D37C2C579F8A209BD4BADC5`;
  ProductVersion/FileVersion are `1.2.0-alpha.5`. PID `54456` supplied one
  responsive, maximized `Syndocal` window. Edit > Lighting created
  `Native QA Group` and reported success, proving the repaired renderer-ticketed
  group route no longer rejects its own Begin as a Display publication.
- E3 is complete. The ordinary-off pause/trace implementation froze at main hash
  `5014DEA37D6788FB6EECFAA4A5A7C9FF59FD83C593F93EC6D27F5EBE83D5AF23` and helper
  hash `17CF98F17095EE89559CA26B03772A09AE5C85CE60F158D6712434A643DDADC8`;
  independent review returned P0/P1/P2 zero. `e3_` passed 6/6, no-default Cargo
  check had zero first-party warnings, and format/diff checks passed.
- The final native release build passed in 1m42s and produced the 56,461,824-byte
  `1.2.0-alpha.5` executable with SHA-256
  `97C21F367A46375A2B6010CC6E9307E7D853CEC4EAF420150A06CA47AD9D4182`.
  The exact-checkout process count was zero before the build.
- B/reply-loss pass: PID `94632`, request
  `69fb623e-7640-4cda-b7cf-261e70f0685d`, checkpoint
  `3a7e24b723a38dd7cffd478b67fc87c78a9df6fba09eb9e1e2825a2a6d4d22cd`,
  journal 43 -> 44. The ready trace and journal matched before the exact process
  was force-terminated. The journal and WebView LevelDB intent remained afterward.
- ACK/cleanup-pending pass: PID `67752` issued the fresh request
  `da5547eb-fdd5-4fcc-84f9-3f658aae706f`; RecoveryPublication advanced 44 -> 45,
  the exact ready record resumed it, and durable RecoveryAcknowledged advanced
  45 -> 46 before reply. Journal SHA-256 was
  `22E329110C96997CFEC893CFE6EF48D5F9F77A5A453C9DE64FC91ABACAF650F9`.
  The exact process was killed while unresponsive only because it was deliberately
  paused; the matching browser intent remained.
- Final hook-free relaunch PID `44376` provided exactly one responsive, maximized
  `Syndocal` window. Journal serial 46/hash stayed unchanged and the acknowledged
  recovery offer remained available until a coherent CleanSave, matching the E3
  startup contract. The next tranche is E4 Save durability; do not begin AI4.
- Commit-preflight warning audit then found five new `too_many_arguments`
  suppressions in renderer-ticketed fixture helpers. They were removed through one
  internal context object; public Tauri signatures and wire schemas are unchanged.
  The lock-order audit now proves direct routes, transaction delegation, the shared
  renderer owner gate, and all nine exact wrapper/helper/command bindings.
  Independent review returned P0/P1/P2 zero, and the audit, E3 6/6, recovered
  fixture 1/1, project-transaction 4/4, no-default check, format/diff, and warning
  ratchet passed at zero first-party warnings and zero new suppressions. Final main
  SHA-256 is `EEFAFB425E935BCBFB3C94278021EBB58D4143B94EB30F78DA9CA6CA794E2B62`.
- The final checkpoint-source native build passed in 1m49s after the exact checkout
  process count was verified as zero. The 56,470,016-byte executable reports
  ProductVersion/FileVersion `1.2.0-alpha.5` and SHA-256
  `8FDFD0EFB8D71D49BD1A137A4AA68F440F098B2C7B3BA253461F5C2AC14D6F19`.
  PID `113948` supplied exactly one responsive, maximized `Syndocal` window. The
  two crash boundaries remain explicitly bound to the earlier `97C21F...`
  acceptance build; this final launch covers the later behavior-preserving internal
  refactor and strengthened proof gate.

## 2026-08-23 alpha.6 E4 automated handoff

- Branch/starting HEAD: `codex/syndocal-v1.2` /
  `b012ef6c3634a87d36c755a3d44bf98ef0213711`.
- Product metadata is advancing to `1.2.0-alpha.6`. E4 intentionally adds a
  versioned, additive project-publication command/journal/receipt contract; product
  SemVer remains separate from that compatibility boundary.
- Save, Save As, user template, backup, and update preflight now share durable
  reservation, exact mapping/authority freshness, target selection, staging,
  final publication, query, terminal ACK, and restart recovery behavior.
- Backend fixed hashes before the version/document patch: main
  `59AC69E8D91E2D8EA2CEBFCA9374425D048F47218AFEE8B2B470346F2F356AAA` and
  control plane `3B73847CAF4F9FC39F84F44D2109BEB5224BA349A10560ECF3232B463CB9BA75`.
  Independent review returned P0/P1/P2 zero.
- Frontend fixed-hash review also returned P0/P1/P2 zero. App and the executable
  checker share `createProjectPublicationControllerV1`; restart owner adoption is
  exact, terminal receipts are never adopted, and durable local rewrite precedes
  resume.
- Green automated gates: project-publication 15/15, updater fence 1/1,
  control-plane 25/25, E4 checker, exact 415 invokes, TypeScript/Vite build,
  no-default Cargo check, format/diff checks, and zero first-party warnings for the
  modified frontend/native configurations.
- Remaining acceptance: run synchronized release/self-tests and full focused
  matrix, terminate only the exact checkout executable before the native build,
  build `1.2.0-alpha.6`, launch exactly one responsive maximized window, and use a
  scratch project for Save, Save As, template, and backup verification. E4 remains
  unchecked and the denominator remains 17/71 until those steps pass.

## 2026-08-23 alpha.6 E4 native acceptance handoff

- Branch/build source: `codex/syndocal-v1.2` / `d707872761e344805ac51550a38d9ce3c416d8d6`.
- Immediately before build, only the running process whose resolved path exactly
  matched this checkout's `target/release/syndocal.exe` was stopped. The native
  command `pnpm --dir app tauri build --no-bundle` passed in 1m50s.
- Artifact: `target/release/syndocal.exe`, 57,085,440 bytes, ProductVersion and
  FileVersion `1.2.0-alpha.6`, SHA-256
  `687E7BFD8B9A74C7A3F91493C2FFAF56D97B60B32B0D0EDF6D45C2209EA563F7`.
- Native launch/UI: exactly one responsive `Syndocal` window, maximized at
  1920x1032. Save and Save As succeeded against
  `C:\TEMP\syndocal-e4-qa-alpha6\phase1-mini-show.sdc` and
  `qa-final-save-as.sdc`.
- The initial User Template operation exposed an invalid native terminal DTO.
  Commit `d707872` makes User Template/Backup receipts authority-free while
  preserving Save/Save As authority. The direct/restart Rust proof, canonical
  frontend positive/negative cases, focused gates, warning-zero checks, and
  independent P0/P1/P2-zero review all passed before rebuilding.
- The rebuilt UI saved
  `C:\TEMP\syndocal-e4-qa-alpha6\qa-template-fixed.sdctemplate` (SHA-256
  `53F0C03BE45CA55E9F9469B4819366C1AE118D75B510E78EED482C6468F6783`) without a
  malformed receipt. Creating the `QA Backup` fixture group dirtied the project;
  after the autosave interval the UI listed `qa-final-save-as.sdc · autosave`.
  The managed file
  `C:\Users\kouty\AppData\Local\jp.seraf.ktn.syndocal\project-backups\backup-1787421898035.json`
  is 36,053 bytes, SHA-256
  `01CCCBC4F99ECCC038FB80788ECBA774CDC2F1E6E1E9DBA65C7425E8A98C0EC3`, names the
  exact source path, and contains `QA Backup`.
- E4 is accepted for this Windows tranche; the broader denominator is 18/71
  (25.4%). The product is not complete. Current-PC ASIO/hardware/security/soak and
  later roadmap gates remain.

## 2026-08-23 alpha.7 D2/UI/Guide integration handoff

- Branch/starting HEAD: `codex/syndocal-v1.2` /
  `da02e3fd20190b1e8656023c09ec532f3b9016d7`. The integrated tree remains
  intentionally uncommitted for root-owned final validation; unrelated concurrent
  changes were preserved.
- Product metadata advances to `1.2.0-alpha.7` because the next integrated native
  development artifact must not replace alpha.6 bytes. No product-adjacent schema
  version changes with this ordinal.
- D2 production PATCH/GDTF Repair is independently accepted at P0/P1/P2 zero.
  Frozen backend SHA-256 values are main
  `5B91759B9B4789AF17B0376220F094B08603D0AD271DEF9B5CD67AA46103215D`, Engine
  `03685241C9FC35B12F7FDA6A880ADEB64D3EF092F60B63717E8325ACEC0C0C6F`, and
  control plane
  `A0864AB959688F5B857F5F49BF985F82F64036F145C3445FDCFA113E9B480750`.
  D2 9/9, project-transaction 4/4, control-plane 25/25, Engine PATCH 10/10,
  Engine Repair 5/5, D2/frontend inventory/operator checks, format/diff, and the
  no-default zero-first-party-warning gate passed.
- Integrated UI evidence passed Lighting/Video/Timeline geometry and screenshots
  at 1920x1080, 1366x768, 860x520, and 1280x720; Timeline operator passed five
  viewports; the Timeline Sources lower pane remained at outer-scroll 0 and
  body-scroll 0. The deterministic Guide perceptual boundary passed at 150 ms.
- D2 alone advances the accepted Windows denominator from 18/71 to 19/71 (26.8%).
  The UI and Guide evidence does not close native, hardware, ASIO, MTC/DJ, DMX,
  three-screen, or soak rows.
- Native alpha.7 build and GUI verification were not run in this checkpoint.
  Root must stop only the exact checkout executable, run
  `pnpm --dir app tauri build --no-bundle`, launch exactly one responsive
  `Syndocal` window, maximize it, and perform the remaining integrated acceptance.
  No commit or push was made by this owner.
- Version/checkpoint gates: `pnpm --dir app run check:release` PASS for exact
  alpha.7 metadata; `cargo check -p syndocal --locked --bin syndocal
  --no-default-features` PASS with zero first-party warnings; `pnpm --dir app run
  check:warnings -- --configuration frontend-typescript-vite-windows` PASS with
  baseline/current 0/0 total and first-party warnings; `git diff --check` PASS
  with line-ending notices only.

## 2026-08-23 alpha.7 Control upper-workspace native/UI closure handoff

- Branch/build base: `codex/syndocal-v1.2` /
  `da02e3fd20190b1e8656023c09ec532f3b9016d7`. D2 remains independently accepted
  and complete; its accepted denominator is 19/71 (26.8%).
- Root cause and repair: Lighting's populated Scene Matrix was hidden by a stale
  upper-panel selector; Video's Media Library had escaped the intended grid/flow;
  Timeline's header and portalled upper content occupied incompatible rows; and
  Sources inherited the resulting short header/outer-overflow geometry. The fixed
  layout restores the populated owners, assigns Timeline a header plus remaining-
  height content row, keeps the Sources shell contained with body-only scrolling,
  and preserves Tools/Live Mixer, expansion, and ordered Escape behavior. Shared
  control, type, spacing, icon, and hit-target sizes were not reduced.
- Dedicated browser acceptance: PASS at exactly 3840x2160, 2560x1440,
  1920x1080, and 1280x720, with 28 state screenshots covering Lighting, Video,
  Video Import, Timeline normal/expanded, Tools, Live Mixer, and Sources. 960x640
  remains the product minimum configuration only. 860x520 and 1366x768 remain
  historical supplemental cases, not current acceptance substitutes. Browser PASS
  is not recorded as native PASS.
- Native build: after the exact-path process pre-stop,
  `pnpm --dir app tauri build --no-bundle` passed in 2m46s for
  `1.2.0-alpha.7`.
  `target/release/syndocal.exe` SHA-256 is
  `00253F26A8D3A933172D7B07923E430B455CA18F98E439CE45C5B62405F86EF4`.
  Launch verification found exactly one responsive `Syndocal` main window at PID
  `100260` from that path and the window was maximized before native interaction.
- Native UI evidence: at the 1920 desktop class (1920x1032 work area), screenshots
  verify non-empty Lighting Scene Matrix, non-empty Video Media Library, contained
  Video Import, Timeline lanes, full-height Sources, contained Tools, expansion,
  and Escape close-then-restore. Evidence is under
  `qa/artifacts/native-ui-alpha7/2026-08-23-1920x1032`. The maximized 2048x1104
  capture is supplemental. Native 2560x1440 and 1280x720 remain unverified because
  no safe verified monitor-move route was available; 3840x2160 was connected but
  not active as a desktop mode.
- Guide timing: PASS at the exact click target minus 7,200 frames, which is 150 ms
  at 48 kHz. This does not prove audible playback or physical output routing.
- Warning boundary: the configurations run for this checkpoint report zero
  first-party warnings. Supplemental `check:edit-live` remains red only on two
  existing unrelated assertions: scene-identity badge color and the historical
  <=86 px mode-tab compactness expectation. It is not evidence of an upper-
  workspace regression and shared controls were not shrunk to make it pass.
- Remaining blockers: physical multi-display VJ output/reconnect/soak, DMX,
  MIDI, DJ Link Agent plus Stream Deck Pedal end-to-end, audible click/Guide
  device routing, the remaining ASIO device/rate/buffer/channel/licensing/fault
  matrix, and the integrated long soak. Do not call the product complete until
  those rows are recorded against their real hardware and exact artifact.
- No commit or push was made by this documentation owner. Root retains integration,
  final hash review, checkpoint commit, push, and all native/hardware completion
  claims.

## 2026-08-23 intentional pause after D3/Tauri/DJ transport work

The current dirty-tree state, exact partial hashes, completed evidence, immediate
resume order, native/hardware matrix, and every remaining master-roadmap task are
recorded in `qa/SYNDOCAL_PAUSE_HANDOFF_2026-08-23.md`. Treat that file as the
authoritative resume point. D3, the new DJ transport hash, the Tauri native crash
mitigation, alpha.8, and all hardware/soak rows remain unaccepted at this pause.

## 2026-08-24 pre-alpha.8 software/source closure handoff

- Branch/base HEAD: `codex/syndocal-v1.2` /
  `23f350c366ede2fdffcfbf3232e18112eada51ea`. Product metadata remains
  `1.2.0-alpha.7`; no alpha.8 metadata or artifact is claimed yet.
- D3 is accepted at the software/source boundary. Its final independent fixed-hash
  review returned P0=0/P1=0/release-blocking P2=0. Full suite evidence is Engine
  822 passed / 2 ignored, I/O 148 passed / 1 ignored, and Syndocal no-default 951
  passed / 5 ignored, all with zero failures and zero first-party warnings. Current
  source hashes are main
  `2CC94D3E5307E6BBC026F1EC815ABC02BFF47E987D62526446AFD1059984E6C7`, Engine
  `F2DCE4D4F6E4B9CA4E15DB722608461D9D0ADBD9948FB41D5796FD4B94E1B2F7`, and I/O
  `E552FEA70D017BBAE40B534A6D854BE1C1B20AFFE111B9E733E0C19FA7A1E8B4`.
- The Tauri FlatInvoke adapter covers 51 commands (39 common + 11 video + 1
  repair), and all six focused adapter tests pass. Raw bodies fail before parsing,
  absent/null optional arguments retain their wire semantics, and malformed bodies
  produce invalid-args without dispatch. The final main hash received a separate
  P0/P1/P2-zero fixed-hash review.
- The warning-ratchet implementation and adversarial self-tests are frozen at
  `55D2388C3EA244F3C58B207E651E56F33DB10B6E76BD404A2943497E384926AA` and
  `3391F2F149ADCED3B5641E48E326680686B8D243389EAFC7E2DCC3799AEA7E55`.
  Independent review returned no release-blocking finding. Git/Cargo/scanner
  overflow or ambiguity fails closed, and the exercised frontend warning row is
  total/first-party 0/0.
- The frozen frontend chain passed build/warnings; routing 133 renderer mutations,
  29 server-authoritative routes, 33 raw calls, and 402 facade calls; 417 invokes;
  backend 478 commands / 311 literal calls / 133 transactions; project
  authority/transaction; E3/E4; output ownership/control/runtime; Scene Matrix
  strip and full matrix at five viewports; DVC DMX 35 assertions; localization
  3556/3556; and worktree/cached diff checks. The Scene Matrix fix measures real
  Live/Playback executor labels before restoring Edit/Scene Matrix. It and the DVC
  DMX strengthening each passed fixed-hash review without a release blocker.
- The companion DJ Agent is committed and pushed on `Beta` at
  `6c4f4328a6866d9d48022bd8ee20a7887c9de851`; 54 tests and 16 Node syntax checks
  pass with zero warnings. Packaged `dist/server.exe` SHA-256 is
  `339ECF6E82EB463F55977F63A137CB0CB52886CD7E2874E87F5AD4724234377B`.
  This is not physical rekordbox/DJ Link/State Sync/Pedal acceptance.
- The accepted denominator remains exactly 19/71 (26.8%). Software/source closure
  is kept separate from native and physical acceptance. The final source manifest
  is intended at
  `qa/artifacts/source-freeze/2026-08-24-alpha7-pre-alpha8-source-freeze.sha256`
  after these documents settle.
- Next action: synchronize the product ordinal to alpha.8 and run `check:release`;
  then perform the exact-path no-bundle native build, one responsive maximized
  window, WER/100-reload stress, physical outputs and current-PC hardware, ASIO/DJ
  fault matrices, and the integrated one-hour soak. No current KDMX commit or push
  is made by this documentation owner.

## 2026-08-24 current-source alpha.8 native checkpoint

The inherited alpha.8 work is now at branch `codex/syndocal-v1.2`, pre-commit base HEAD
`23f350c366ede2fdffcfbf3232e18112eada51ea`, with synchronized product version
`1.2.0-alpha.8`. The final current-source gate log is
`target/qa/alpha8-current-source-final-gates-20260824-114918`; all 25/25 gates
passed. The current CSS and upper-workspace checker hashes are
`4974a2f828b8b8bd1c9fbe43390d97d5d6702179` and
`73a43ecfb2c6f10c07fb638f84f50375de5213a0`; the Scene Matrix checker SHA-256 is
`F164CD5B5C6C130E1D27B21C6A04CB1C361CEE3346F08FA9DFF77DE522C5FE11`.
Independent fixed-hash review found P0=0/P1=0/release-blocking P2=0, and the
exercised warning counts were total/first-party/third-party 0/0/0.

The authoritative staged-source inventory is
`qa/artifacts/source-freeze/2026-08-24-alpha8-current-source-freeze.sha256`.
It contains 103 payload records and excludes its own manifest envelope from the
payload to avoid recursive self-hashing. The alpha.7 pre-alpha.8 manifest remains
historical evidence and is not current alpha.8 source authority.

After stopping only the exact checkout `target/release/syndocal.exe` process,
`pnpm --dir app tauri build --no-bundle` passed from the Visual Studio Developer
Shell (MSVC 14.43.34808, Windows SDK 10.0.26100.0). The resulting executable is
57,491,456 bytes, SHA-256
`627BE88032774C7FA0A4C3CD3510A7BFB52E8ED0E76884ADD414B9BFD101F459`; the PDB
is 19,582,976 bytes, SHA-256
`5548E4F4B2C3CBB38F1881AAA6C9299AE42211616A8A05E9189C3019838F56AB`. Both
FileVersion and ProductVersion are `1.2.0-alpha.8`. Launch verification found
one responsive maximized Syndocal window from the exact executable.

The fresh reload artifact
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-reload-stress-100-current-source.json`
has SHA-256
`6D12AA14371B1C837DF67DDE80AB44CB1C6B1329F093567B81524E35076FB621` and records
100/100 reloads, 100 unique origins, exit 0, total 40.1424 s, p50 407.3 ms,
p95 479 ms, maximum 530.1 ms, zero runtime/log issues, and zero WER Event and
Reliability deltas. The previous `2026-08-24-alpha8-final` artifact directory
is historical pre-CSS evidence and is superseded for current-source claims.

The native five-display pane route is complete through the supported
`open_pane_window` placement path: `D5 -> D2 -> D3 -> D1 -> D6`. Each pane was
maximized and each step recorded zero document scroll. The route measurements
are D5 monitor 1920x1080, scale 1.5, viewport 1280x650, position `{-2465,1731}`;
D2 1920x1080, scale 1, viewport 1920x1009, position `{0,0}`; D3 real physical
3840x2160 4K, scale 1.5, viewport 2560x1370, position `{-3840,-429}`; D1
2560x1440, scale 1.25, viewport 2048x1082, position `{1920,-364}`; and D6
2560x720, scale 1, viewport 2560x649, position `{1598,1080}`.

Canonical route evidence is
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-display-route-current-source.json`,
SHA-256 `41F1D6E2528E7439657F8879F753255221E25F2DF0474139B56C1570E2C32C41`.
The five pane screenshots in that directory are D5
`37EDB56EA3F81BA014C23A96A1C78F32EEBB68C6B3E4C3D7C6692FD30E2FD0E1`, D2
`0A037F7E6359FBC22C1FC16F297D9A3860157544777764679FABD2C194C20CFD`, D3-4K
`55E83F6AFC32A32DD9BD5AAB187DA659DC0221DB0EF82EAEAA65215327F591CD`, D1
`C22918826714D16E0E02DB5190AFB1E2724CEB387943A0D3FA4FABCDE5DD645E`, and D6
`2336172DB637B9E4F9D4906B3E7D6D781408CD0A4B11D18229974FC92E820340`.

The expanded D5 evidence `D5-timeline-tools-expanded-fixed.jpg` is
`B780CD4CCDA35CC8A8F1148A64B26C91065EB15E33D57A38FDCC04F8B0A3724D`. Native
metrics record popup client/scroll 345/345, nested surfaces 335/335, and the
deepest 44px target inside the viewport/popup with hit/focus proof and Escape
focus return. The pane was closed; final state was one exact responsive PID
123952, CDP page 1, and maximized D5 main viewport 1280x672.

All five detached Timeline pane screenshots still show the non-secret status six
seconds after each pane opened:
`Window 'pane-timeline' has no current project transaction owner registration`.
At the same six-second point, `get_project_authority_bundle` from the pane succeeds
with epoch 0/revision 1, but that separate read does not prove transaction-owner
registration or make the persisted status stale. This evidence accepts placement,
maximize, and containment only; it does not accept transactional pane operation,
warning-clean pane startup, or completed owner registration. The main D5 Timeline
Tools proof remains clean. Keep the owner-registration ordering/status behavior as
an alpha.9 P1 investigation boundary, not a completed acceptance row.

This closes native multi-display pane placement/containment only. It does not
claim display-output playback, fullscreen playback, GPU reset/recovery, physical
DJ Link/rekordbox, Stream Deck Pedal/MIDI, ASIO, real DMX, failure matrices, or
the one-hour integrated soak. The accepted denominator remains 19/71 (26.8%).
Root retains staging, commit/push, and final broader hardware completion claims;
this documentation update performs no Git write.

The non-destructive capacity audit measured about 343.777 GiB for the workspace,
343.04 GiB for `target`, 238.15 GiB for `target/debug` including about 149 GiB
incremental, and 90.36 GiB for the old named QA target directories. The
`.claude/worktrees` copies are only about 5.3 MiB each and `vendor` is about
0.29 MiB, so the size is overwhelmingly generated Rust build and QA cache output,
not historical source/worktree copies. No cleanup was performed because the
deletion/rebuild-cost tradeoff was not authorized; generated `target` remains
excluded from Git.

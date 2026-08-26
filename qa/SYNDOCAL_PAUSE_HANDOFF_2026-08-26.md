# Syndocal pause / resume authority — 2026-08-26

Status: **RESUMED THROUGH THE ALPHA.14 SHOW-ASIO, TIMELINE-NATIVE, AND DVC SOFTWARE CHECKPOINT**

This is the current resumption contract for the 2026-08-30 performance. The
show-completion deadline is **2026-08-29**, not the performance day. The code is
not accepted as fully show-ready: the alpha.14 strict-v3 software and standard
native artifact gates are complete, while the physical DJ, three-output, and
DSF native show-program gates below remain open. The exact DSF2026 Rust importer
gate and DVC frontend-controller extraction are complete, but the native UI
import/report, Save As, restart/reload, and physical route remain open. The
separately licensed alpha.14
Show-ASIO artifact has one bounded physical Ampero native/operator proof only;
it does not close fault, long-duration, latency, or full ASIO acceptance.

## 1. Exact checkpoints

The checkpoints are deliberately separate. Do not relabel a later QA/docs
commit as the source identity of an already-built binary.

| Layer | Exact authority |
| --- | --- |
| Current alpha.14 runtime/code and standard artifact | `92122f1b148d40845b2cfe3e4618a57ce132b3df` |
| Alpha.14 local-only Show-ASIO artifact | `6b4cd1afb4d228158d04a15dbe3e4a73c922baeb` |
| DVC import controller extraction and focused software proof | `652b197d3cce9cfc119a790baffefbd47f08cc8c` |
| Current rb-output strict-v3 runtime / docs tip | `862cf8035dfb365a7d799f820936585882d0a1e7` / `e3d390d912a2c3a9be418ecbc31771d2bf515de7` |
| Timeline menu/localization alpha.13 checkpoint | `bbb684cee4c8b01cfc019575569bd26835dbc732` |
| Historical alpha.12 standard and Show-ASIO artifacts | `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae` |
| Display stable-identity QA | `b543067b0cbde4015ee632a8c6e6ccd77e6bfd9f` |
| Show-ASIO Cargo hardlink fix | `fb25ab106e9994fb6215520795961f4909bea7ae` and `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae` |
| Cleanup inaccessible-process fix | `ef7b6479f69e89dd134acfe39051f43c22f769aa` |
| Cleanup exact Codex-control-plane gate | `c40cfd89ccb2203b92e76d3a8d72f00980aa1a30` |
| This final documentation publication | The pushed commit containing this file; resolve with `git log -1 --format=%H -- qa/SYNDOCAL_PAUSE_HANDOFF_2026-08-26.md`. A self-hash is intentionally not embedded. |

Branch: `codex/syndocal-v1.2`. The alpha.14 runtime source and native artifact
are bound to pushed, upstream-equal commit
`92122f1b148d40845b2cfe3e4618a57ce132b3df`. A later commit containing only
checkpoint documentation must not redefine that binary source identity.

## 2. Mandatory operating rules

- The capability order is **Sol > Ox-alpha (`opencode/x-preview-f-free`) >
  Terra > Luna**. Sol owns decomposition, integration, and completion claims.
  Ox is the default bounded implementer and independent adversarial reviewer.
  Terra implementation requires Ox review. Luna Max is limited to small,
  explicit, low-ambiguity units.
- Keep independent lanes busy for material work, assign non-overlapping file
  ownership, and do not let reviewers edit until an implementation checkpoint
  is stable. Parallelism must reduce elapsed time, not create merge ambiguity.
- **Fail-close**: missing, stale, ambiguous, unauthenticated, or unverified
  state must produce an explicit typed failure. Do not silently promote it to a
  working state.
- **Clean-break**: remove retired designs and legacy paths when their replacement
  is accepted. Do not add fallback or compatibility paths without a documented
  compatibility boundary and tests.
- The pinned Microsoft VS Community 14.44 linker must be first and
  `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` must be explicit. Never accept
  Cargo selecting Git's `link.exe` merely because a build happened to finish.
- Before each native release build, resolve and stop only this checkout's exact
  `target/release/syndocal.exe`. After build, launch that exact binary and verify
  exactly one responsive, maximized `Syndocal` window. Browser-only evidence is
  not native acceptance.
- Every meaningful checkpoint updates its QA/release authority, reports
  first-party warnings, then commits and pushes. Published artifacts and tags
  are immutable; prerelease ordinals advance after product changes.
- Cleanup is exact-path, reviewed, clean-tree, upstream-equal, no-writer, and
  literal-path only. A blocked Plan never authorizes Apply.
- Do not add new domain policy directly to `app/src/App.tsx` or
  `app/src-tauri/src/main.rs`. Extract the owning domain first and preserve the
  contract with focused tests.

## 3. Current alpha.14 standard and local-only Show-ASIO artifacts

### Standard MIT/WASAPI artifact

- Source checkpoint: `92122f1b148d40845b2cfe3e4618a57ce132b3df`
- Path: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
- Product/File version: `1.2.0-alpha.14`
- Size: `58,523,648` bytes
- SHA-256: `B140E9DA515741C8A6A318963C6BAB576CED62ECABEDEA50BA8DE15593AE325C`
- Build timestamp: `2026-08-26T05:07:44.5595600Z`
- Resumed native-QA process: PID `99748`; exact app path as above; responsive
  title `Syndocal`; main HWND `6430754`. Computer Use reselected that exact
  app/window and kept it maximized at `1920x1032`. The temporary and sample
  project changes used below were explicitly discarded rather than saved;
  after Close, the exact-path process count was `0`.

### Current local-only Show-ASIO artifact: bounded Ampero physical proof

- Source checkpoint: `6b4cd1afb4d228158d04a15dbe3e4a73c922baeb`
- Directory:
  `C:\Users\kouty\Documents\KDMX\target\show-asio-local\Syndocal_Show_ASIO_1.2.0-alpha.14_6b4cd1afb4d2_x64`
- Application: `syndocal-show-asio.exe`, `58,683,904` bytes, SHA-256
  `CC2D1E28B9063250E86106F04DD082A5C860A840EB721C8208EEFEE009BD0599`
- Bridge: `syndocal_asio_bridge.dll`, `813,568` bytes, SHA-256
  `40BB8D19C7B5C8DFA52C21C879C8887645CDE83DF6A4FAB5CF59D2A396546AE2`
- Manifest: `show-asio-local-manifest.json`, SHA-256
  `BBEA830122B999A7F985A8F0E88330361E74D3EEE9C5DF2E982EB932A4B687DD`
- Dedicated checker: PASS, `files=14`, `distributionApproved:false`.
- Official `node app/scripts/build-windows-show-asio.mjs` route: PASS with the
  exact VS Community 14.44 x64 linker pinned and first, `check:release` PASS,
  and first-party build warnings `0`.
- `distributionApproved:false`; same-host, unbundled, separately licensed,
  local show artifact only. It is not a public installer/updater artifact.
- Explicit driver: `HOTONE AUDIO USB Audio Device`; ASIO native `i32`, two
  channels, 44.1 kHz, fixed requested/applied `128f`.
- First Start held five seconds stable then Stop with no pending work:
  callback `128/128/128`; `OVR 0/0f`; `XRUN 0`; capture-to-worker `4.4/4.5 ms`;
  `Q 0/1/4`; app/bridge process counts `0` after Close.
- Restart restored `ASIO RESTORED` locked/no auto-start. Explicit Start emitted
  `ASIO REVALIDATED` and ACTIVE with the same configuration; capture-to-worker
  was `4.4/4.7 ms`, and final Stop/Close again left both process counts `0`.
- This does not claim unplug, XRUN/fault, TOPPING, long-duration, matrix, or
  latency-threshold rows. After this documentation commit, reuse requires a
  clean checkout detached at `6b4cd1a` or a rebuild from the new HEAD.

## 4. Completed software evidence

Current alpha.14 evidence:

- rb-output runtime `862cf8035dfb365a7d799f820936585882d0a1e7`:
  full `npm test` `389 total / 387 pass / 0 fail / 2 intentional package
  skips`; Stage-1 plus strict-v3 focused tests `33/33`; changed-JavaScript
  `node --check` PASS; first-party warnings `0`.
- Exact MSVC 14.44 linker was pinned and first in `where.exe link.exe` for all
  Cargo gates. Protocol DJ-Link passed `12/12`; the dedicated rb-output sender
  contract passed `2/2`; I/O `remote_ws` passed `57/57`; Syndocal DJ-Link
  dispatch passed `10/10`; extracted loop-range mapping passed `3/3`.
- `pnpm --dir app run check:release` PASS: release metadata alpha.14, ASIO
  packaging boundary `169` assertions, and video-output observation PASS.
- Three-display deterministic harness: `80/80`.
- Native warning gate PASS with baseline/current warnings both
  `0 total / 0 first-party / 0 third-party`; it executed the required
  `pnpm --dir app tauri build --no-bundle` from clean, upstream-equal
  `92122f1b...`.
- The alpha.13 Timeline checkpoint `bbb684c...`, carried unchanged into
  alpha.14, implements compact grouped item submenus and outside-primary-click
  dismissal; its focused browser contract passes. The exact standard alpha.14
  native app then opened `samples/phase1-mini-show.sdc` maximized at
  `1920x1032`: right-clicking point event #1 showed the compact collapsed
  groups `選択`, `クリップボード`, `タイミング`, and `レーン`, plus only
  `選択項目を削除` and `閉じる`. Expanding `タイミング` exposed its eight
  nested actions; a primary click at blank coordinates `1000,500` removed both
  the menu and the expanded actions from the accessibility tree. The sample
  was closed with discard, remained unmodified on disk, and the exact standard
  process count returned to `0`.
- The Bank menu command `New Scene` rendered as `新しいシーン`. A temporary
  user/project title literally named `New Scene` remained literal project data
  and was not translated behind the operator's back; it was discarded without
  saving.
- The exact `DSF2026.dvc` input is `67,873` bytes with SHA-256
  `22D86D7C0F0C56543B440356F76C467F86C37B0E9D06D76EA8F56B2952D0F841`.
  Under the exact Community VS 14.44 linker, pinned first ahead of Git's
  linker, the focused Rust importer test passed with `fixtures=46`, `cues=2`,
  `effects=0/0`, `midi=2`, `approximate=4`, `skipped=0`, `unsupported=1`, and
  `warnings=0`. Approximate details are the MIDI input/feedback device-affinity
  boundary; the Unsupported detail is the device-specific Daslight hardware
  binding. This is backend import evidence, not native UI/save/reload proof.
- Commit `652b197d3cce9cfc119a790baffefbd47f08cc8c` extracts the 42-line DVC
  import orchestration and paired result type from `App.tsx` into
  `app/src/dvcImportController.ts`. Focused controller proof passed `7/7`; DVC
  MIDI/DMX shortcut contracts passed `39/39` and `35/35`; frontend invoke
  inventory remained exact at `419`; project transaction/authority gates
  passed; `pnpm --dir app run build` passed; the frontend warning ratchet
  stayed at baseline/current `0/0` with no first-party warning. Independent
  Terra xHigh review found no P0/P1/P2; Ox-alpha remained unavailable under the
  recorded narrow exception. Native build and UI proof for this later internal
  commit remain open.
- Final independent Terra xHigh adversarial review found P0/P1/P2 none after
  fixing rapid-F14/inactive-loop races, late-fallback causality, and the
  measured-loop sender/receiver wire mismatch. Ox-alpha was not callable in
  this session, so Sol recorded this narrow review exception.
- One non-authoritative parallel I/O attempt produced socket-reset failures;
  it is not completion evidence. The final authoritative sequence was rerun
  serially with `--test-threads=1`. One later sequence omitted `FFMPEG_DIR` and
  stopped before Syndocal tests; it too is invalid. The entire sequence was
  rerun from gate 1 with `FFMPEG_DIR`, `LIBCLANG_PATH`, and the exact linker and
  produced the passing counts above.

Historical alpha.12 broad-matrix evidence retained for reference:

- protocol: `175/175`
- I/O: `158/158`
- engine: `868` passed, `2` explicit hardware/manual ignored
- Spout: `37` passed, `5` explicit hardware ignored
- NDI: `76` passed
- localization: `3568/3568`
- Timeline shelf focused gate: PASS
- Timeline slim layouts: PASS at `1920x1080`, `1920x1032`, `2048x1152`,
  `1366x768`, and `1280x720`
- frontend: `pnpm --dir app build` PASS
- native warning gate: `pnpm --dir app run check:warnings --
  --configuration windows-native-release` PASS; it ran
  `pnpm --dir app tauri build --no-bundle`
- First-party warnings were `0` for:
  `windows-default-all-targets`, `windows-default-release`,
  `windows-workspace-tests`, `windows-syndocal-asio`,
  `windows-syndocal-ndi`, `windows-syndocal-spout`,
  `windows-asio-bridge`, `frontend-typescript-vite-windows`, and
  `windows-native-release`. No checkpoint added a first-party warning.
- Three-display stable-identity harness: PowerShell 7 and 5.1 both `80/80`.
- Cleanup harness after the Codex-control-plane safety fix: PowerShell 7 and
  5.1 both `93/93`.

Independent Ox results:

- Display stable identity: APPROVE; no P0/P1/P2 after blank-GDI coverage.
- Show-ASIO hardlink topology: initial REJECT identified P0/P1; the final exact
  app/bridge hardlink pair passed follow-up review with no P0/P1.
- Cleanup inaccessible-process metadata: APPROVE; no P0/P1/P2.
- Exact Codex-control-plane exception: final APPROVE; all prior P2-1 through
  P2-5 resolved, no remaining P0/P1/P2. Ox independently reran `93/93` on both
  PowerShell versions.

## 5. Cleanup inventory and final result

Fresh read-only inventory after the alpha.14 source checkpoint, before the
native build:

- `C:\Users\kouty\Documents\KDMX\target`: `204,754,923,466` logical bytes,
  `134,462` files, `16,327` directories including root, reparse points `0`.
- `C:\Users\kouty\Desktop\rb-output\node_modules`: `81,619,354` logical
  bytes, `6,053` files, `910` directories including root, reparse points `0`.
- `C:\Users\kouty\Desktop\rb-output\dist`: `277,382,202` logical bytes,
  `44` files, `12` directories including root, reparse points `0`.
- No cleanup Apply ran; no path was deleted; reclaimed bytes remain `0`.

The only reviewed production candidate was:

`C:\Users\kouty\Documents\KDMX\target\debug\incremental`

Read-only inventory measured `57,544,180,106` logical bytes, `29,401` files,
and `593` directories. From clean, upstream-equal checkpoint `c40cfd8`, the
production Plan was run with:

```powershell
& .\qa\harnesses\invoke-syndocal-build-cache-cleanup.ps1
```

Final result:

- Mode: `Plan`
- Outcome: `Blocked`
- Blocker: `HardlinkDetected`
- Exact first blocker:
  `C:\Users\kouty\Documents\KDMX\target\debug\incremental\audio-0nuw1tgmhpz4j\s-hln3a7p0dg-1xgadpp-2vnp8sov454s93u5khsbi2d9r\metadata.rmeta`
- Link count: `2`; the second name is
  `C:\Users\kouty\Documents\KDMX\target\debug\deps\libaudio-37f836d0d4d0a1f2.rmeta`.
- Apply: **not run**
- Reclaimed: `0` bytes
- Recovery: not applicable because nothing was deleted.

Do not bypass this blocker with an ad-hoc `Remove-Item`. Resumption may design a
reviewed hardlink-aware candidate proof, but that is new work and was
deliberately not added to this stop tranche.

## 6. Exact physical display authority

Five active displays were observed. Stable DisplayConfig identity is authority;
GDI `DISPLAYn` numbers are current observations and may change.

| Role/status | Stable identity | Current GDI | Mode | DPI |
| --- | --- | --- | --- | --- |
| available | `\\?\DISPLAY#IOC9125#5&eb37e8d&1&UID4353#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}` | `DISPLAY1` | `2560x1440` | `120` |
| intended projector | `\\?\DISPLAY#MSI3DD2#5&eb37e8d&1&UID4357#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}` | `DISPLAY3` | `3840x2160` | `144` |
| intended editor | `\\?\DISPLAY#PXO2500#5&eb37e8d&1&UID4355#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}` | `DISPLAY2` | `1920x1080` | `96` |
| intended LED | `\\?\DISPLAY#PXO1560#5&2c959af3&0&UID768#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}` | `DISPLAY33` | `1920x1080` | `144` |
| available | `\\?\DISPLAY#CRXED00#5&2c959af3&0&UID772#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}` | `DISPLAY34` | `2560x720` | `96` |

The earlier assertion that Windows had no `3840x2160` mode was wrong; DPI
scaling caused the confusion. Syndocal must still bind roles by stable identity,
not infer roles from resolution.

Discovery-only evidence exists for both artifacts and correctly returned
`not-configured`, `accepted:false`, `native_hardware_claim:false`. A real Apply
with content visible on Editor `1920x1080`, LED `1920x1080`, and projector
`3840x2160` has not been accepted.

## 7. DJ-Link authority

- Checkout: `C:\Users\kouty\Desktop\rb-output`
- Branch: `beta-v1.1.2`
- Runtime source: `862cf8035dfb365a7d799f820936585882d0a1e7`
- HEAD/upstream docs tip: `e3d390d912a2c3a9be418ecbc31771d2bf515de7`, clean at resumed audit
- FOH/Syndocal host: `.50.1`; DJ PC: `.50.2`
- The controlled source path requires the external show JSON through
  `DJ_AGENT_CONFIG_PATH`; a preflight-only pass is not an active Agent session.
- Normal production authority is fresh measured `DJ_LOOP_STATE`, nested under
  `payload.loop`. Only actual no-response after the armed F14 window emits
  `DJ_LOOP_FALLBACK`. Each prediction carries a monotonic intent ID and exact
  measured-revision/effective-division base; late fresh measurement overrides
  and rebases it. The complete profile is
  `8 → 4 → 2 → 1 → 1/2 → 1/4 → 1/8 → 1/16 → 1/32 → 1/64`; it does not stop at 2.
- F13 `DJ_RELEASE` is independent from the Rekordbox Stop MIDI result and is
  routed exactly once after the configured release sequence reaches Stop.
- DJ-LINK may be started whenever needed. Moving the controller is safe for
  the current software/native checkpoint, but any resulting loss of Rekordbox
  MIDI In remains unverified hardware state and does not satisfy a response or
  no-response row by itself.
- Physical acceptance remains `0/12`: real token, HELLO/ACK, Rekordbox track
  detection, MIDI output, pedal, the full measured/no-response loop profile,
  release, reconnect, and restart are not recorded as accepted.

## 8. Explicit remaining show gates

These are OPEN, not implicit completion claims:

1. Run the full DJ-Link 12-row hardware table on `.50.1/.50.2` with the real
   show token and retain logs.
2. Complete the remaining Show-ASIO matrix from its exact local-only artifact.
   The bounded Ampero selection, persistence/revalidation, native format/rate/
   channels/buffer, and short Start/Stop/Close/restart path above are proven;
   exclusive contention, reset/resync/XRUN/unplug/no-callback, long-duration,
   and measured latency remain open.
3. Treat the recorded `44.1 kHz / 3,600,031 ms` run as bridge-only and the new
   alpha.14 Ampero run as short native/operator evidence only. Matched `48 kHz`
   ASIO/WASAPI and the remaining recovery matrix remain open.
4. Configure and Apply the stable three-output roles, verify exact placement and
   visible content on all three physical displays, then save/restart and repeat.
5. Repeat the now-passing exact
   `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc` import through the native UI;
   retain the visible import report and exact skipped/fail-closed mappings.
   Create representative Lighting scenes,
   LED/projector substitute media, the two required timelines, DJ trigger and
   measured/no-response loop behavior through `1/64`, release/transition
   behavior, then save/restart and run the full show
   sequence. User-authored final lighting remains outside this minimum proof.
6. Complete native/manual QA for the remaining UI surfaces. Timeline item-menu
   hierarchy and outside-primary-click dismissal are now proven. Remaining work
   includes compact Bank/Scene placement, Lighting-only Bank management, empty-bank
   control de-duplication, active-scene color, bank toolbar alignment, scene FX
   internal scroll/removal/density/width, detached Timeline/Stage/source panes,
   and integrated Stage rendering.
7. Decide and prove a hardlink-aware cleanup contract before deleting the
   current incremental cache; otherwise leave it untouched.

## 9. Giant-file debt is not resolved

Current exact metrics:

- `app/src/App.tsx`: `28,778` lines, `1,201,419` bytes, SHA-256
  `FB4295521386B246629EC1B6BC5DFBD229D1E53DB2E4AB058DCD41B46495AF9E`
- `app/src-tauri/src/main.rs`: `129,413` lines, `4,965,088` bytes, SHA-256
  `A337B15E66C69A56FEC12A9FFF2D3275829BA610B28F787BE58AB56CB803E5C2`
- This tranche extracted the exact loop-profile mapping and its tests into
  `app/src-tauri/src/dj_loop_range.rs` (`102` lines, `3,504` bytes, SHA-256
  `71A7CD025FA0888A045AAF0E110CB8F337672F0F3E365AF82527DCBF601F324F`)
  and added the cross-repository wire proof as the separate
  `crates/protocol/tests/dj_link_v3_sender_contract.rs` (`67` lines, `2,158`
  bytes). It did not attempt a risky whole-file rewrite before the show.
- The resumed DVC tranche extracted the import coordinator into
  `app/src/dvcImportController.ts` (`77` lines, `3,379` bytes, SHA-256
  `1F78CC6963596902114A88ECCBF50116DC9F4806D4265E3BFBDC75CDF40941B0`)
  with its deterministic `185`-line focused checker. The old orchestration was
  removed from `App.tsx`; the MIDI/DMX contract checkers now inspect the new
  production module rather than retaining source-string comments in the giant
  file.

This remains an architectural risk: frontend, Tauri, engine, and recovery paths
still touch shared state through oversized compilation units. Do not attempt a
whole-file rewrite before the show. New work must extract one owned domain at a
time behind existing contracts, beginning with the smallest show-critical seam
that otherwise requires touching these files.

## 10. Preserved Git state and resumption order

- Companion worktree:
  `C:\Users\kouty\Documents\KDMX-asio-persistence`, branch
  `codex/asio-persistence-v2`, HEAD/upstream
  `0ab0ca46569b0ab4f08b3c61ca4f4f202c4a0de4`.
- Its exact 12 dirty tracked paths are:
  `app/scripts/check-backend-operator-contract.mjs`,
  `app/scripts/check-frontend-command-routing.mjs`,
  `app/scripts/check-live-audio-input.mjs`,
  `app/src-tauri/src/control_plane.rs`, `app/src-tauri/src/main.rs`,
  `app/src/App.tsx`, `app/src/tauri-invoke-manifest.json`,
  `app/src/tauriInvokeCommands.ts`, `app/src/types.ts`,
  `qa/ASIO_INPUT_ACCEPTANCE.md`, `qa/harnesses/README.md`, and
  `qa/harnesses/check-asio-build.ps1`. Preserve these user/agent-owned edits.
- Preserved stashes:
  `stash@{0}` alpha.9 validation checkpoint and `stash@{1}` orphaned Open-DMX
  pacing WIP. Do not drop or apply them without a separate adjudication.
- Main internal DVC checkpoint `652b197...`, alpha.14 artifact source checkpoint
  `92122f1b...`, and rb-output docs tip `e3d390d...` were clean and
  upstream-equal when recorded. The final documentation
  commit containing this handoff must again be pushed and rechecked clean.
- The current exact standard alpha.14 process and window identities are in
  section 3. Re-resolve them on resume; do not assume PID/HWND stability.

Resume in this order:

1. Read this file, `qa/ASIO_INPUT_ACCEPTANCE.md`, and the DJ acceptance table.
2. Recheck main/companion/DJ branch, HEAD, upstream, dirty ownership, stashes,
   exact artifact hashes, and exact running process identity.
3. Prioritize physical acceptance over installer polish or broad refactoring:
   DJ 12-row table, native ASIO, stable three-output Apply, then DSF show flow.
4. Fix only evidence-backed blockers; keep unsupported mappings fail-closed.
5. After each meaningful pass, update authority, report warning counts,
   commit, and push.

This checkpoint does not claim public-release readiness or completed performance
acceptance. It preserves a verified alpha.14 strict-v3 standard-native
checkpoint, the bounded alpha.14 local Show-ASIO Ampero proof, and the shortest
exact route to the remaining physical show proof.

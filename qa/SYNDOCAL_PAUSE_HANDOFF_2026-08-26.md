# Syndocal pause / resume authority — 2026-08-26

Status: **ALPHA.21 SETUP-I/O OPERATOR SOURCE/NATIVE CHECKPOINT VERIFIED; PUBLIC RELEASE AND PHYSICAL SHOW GATES OPEN**

This is the current resumption contract for the 2026-08-30 performance. Section
31 is the only current alpha.21 source/native authority; every earlier
alpha.18-or-earlier `current` label below is retained as dated historical
evidence and must not be used for execution. The show-completion deadline is
**2026-08-29**, not the performance day. The code is not accepted as fully
show-ready: the alpha.21 local Setup-I/O operator slice and the historical
DSF native import/report/Save As/restart/reload software slice are complete,
while physical DJ, three-output Apply/content/save/restart, representative-scene
real output, and the remaining DSF show-program gates remain open. The DVC
authority race is fixed in pushed commit `1d372e795870c1a6e5687d1116161042ddac627e`:
the old unconditional async authority fallback let an event or poll hydrate
state while the DVC-specific report/navigation continuation went stale. The new
inline paired-authority branch remains fail-closed for mismatched or later-C
state, and the compatibility fallback is unchanged. The separately licensed alpha.14
Show-ASIO artifact has one bounded physical Ampero native/operator proof only;
it does not close fault, long-duration, latency, or full ASIO acceptance.

The historical alpha.18 KDMX source checkpoint was at branch
`codex/syndocal-v1.2`, exact `HEAD`/upstream
`db4eefc348b01ee05dd2dc87945afa85de8803e`; its required native build, launch,
and maximized-window layout are verified, while the direct wired-binding refresh
click remains unconfirmed. The standard native artifact recorded below is
`1.2.0-alpha.17`, built from source inputs committed and
upstream-equal at `fb5d18fdf898a1435bed173ddd17934a04a97897` after the focused
software gates, warning-0 checks, and independent reviews recorded below. Its
exact executable identity and active process proof remain in section 20 as
historical alpha.17 evidence and are not re-bound to alpha.18.
The separately licensed alpha.14 Show-ASIO artifact remains the only
Show-ASIO artifact recorded here. Its runtime baseline includes the DVC
controller checkpoint `652b197d3cce9cfc119a790baffefbd47f08cc8c`. The alpha.15
artifact identity, warning ratchet, and native process gate are recorded below;
they do not promote physical hardware or output acceptance. This same-version
rebuild is local validation evidence only; advance the prerelease ordinal before
distributing another development artifact.

Alpha.17 release-metadata evidence is split by filesystem boundary. The active
OneDrive checkout correctly rejected direct `check:release` because
`qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json` has an external hard-link alias. A
detached clean validation worktree overlaid with the exact alpha.17 source and
pinned runtime bytes passed `check:release`, including the 169-assertion Windows
packaging self-test and exact `1.2.0-alpha.17` metadata. Its Git worktree
registration and `.git` directory were removed, but the generated unregistered
directory `C:\TEMP\KDMX-alpha16-release-gate-01a03b78` remains because the
recursive cleanup operation was policy-rejected. It contains no authoritative
source or Git registration and remains an explicit cleanup item; do not weaken
the hard-link checker or delete the external OneDrive alias to hide it.

## 1. Exact checkpoints

The checkpoints are deliberately separate. Do not relabel a later QA/docs
commit as the source identity of an already-built binary.

| Layer | Exact authority |
| --- | --- |
| Current alpha.21 Setup-I/O operator source/native | Product metadata is `1.2.0-alpha.21`; authoritative I/O/native source is `536742db968b242164349c34dd6940fe3ced8e92`; local artifact SHA-256 is `F73F1AD8F9E8229B8ACB713AE0C10C36E199C878D869A2E98B554190FC908FA3`; public release and HW-4 remain open |
| Current token-free show structural preflight | Read-only/authored-only source is committed and pushed at `b35d3ba351b29caaedfb7d0c6f4e29fc84580e83`; valid fixture exits `0`, invalid/legacy/usage paths exit `2`; this is not native or hardware evidence |
| Current rb-output v1.1.9 controlled source | `beta-v1.1.2` clean/upstream-equal source authority at `b03d66a87b8d9dcdedfbd9b5c395bda7df7e0eec`; full suite `455` total / `453` pass / `0` fail / `2` intentional skips; target-DJ-PC identity and HW-4 remain open |
| Historical alpha.18 source/native (db4eefc; direct refresh click unconfirmed) | Product metadata was `1.2.0-alpha.18`; exact source `HEAD`/upstream was `db4eefc348b01ee05dd2dc87945afa85de8803e`; historical native artifact/process identity is recorded in section 21 |
| Historical alpha.17 standard native source/artifact | `fb5d18fdf898a1435bed173ddd17934a04a97897`; exact artifact and process identity in section 20; physical acceptance remains open |
| Historical local alpha.15 native verification runtime/source/artifact | Pushed, upstream-equal source commit `c609b61c77e44ee028ed7322c29a0cfd04b8182c` (`refactor(scene): extract bank scene creation controller`); local artifact identity is in section 3 and is not a new distributable artifact |
| Historical prior alpha.15 standard runtime/source/artifact | Pushed, upstream-equal source commit `1d372e795870c1a6e5687d1116161042ddac627e` (`fix(project): preserve inline authority continuation`); prior standard artifact identity is retained in section 3 |
| Historical alpha.14 runtime/code and standard artifact | `92122f1b148d40845b2cfe3e4618a57ce132b3df` |
| Alpha.14 local-only Show-ASIO artifact | `6b4cd1afb4d228158d04a15dbe3e4a73c922baeb` |
| DVC import controller extraction and focused software proof | `652b197d3cce9cfc119a790baffefbd47f08cc8c` |
| Historical rb-output v1.1.8 controlled source | `beta-v1.1.2` was clean/upstream-equal at `0f3e8c6851857c8542c132a89a7d44289002b1f5`; stable suite `415` total / `413` pass / `0` fail / `2` intentional skips; superseded for execution by v1.1.9 |
| Timeline menu/localization alpha.13 checkpoint | `bbb684cee4c8b01cfc019575569bd26835dbc732` |
| Historical alpha.12 standard and Show-ASIO artifacts | `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae` |
| Display stable-identity QA | `b543067b0cbde4015ee632a8c6e6ccd77e6bfd9f` |
| Show-ASIO Cargo hardlink fix | `fb25ab106e9994fb6215520795961f4909bea7ae` and `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae` |
| Cleanup inaccessible-process fix | `ef7b6479f69e89dd134acfe39051f43c22f769aa` |
| Cleanup exact Codex-control-plane gate | `c40cfd89ccb2203b92e76d3a8d72f00980aa1a30` |
| Cleanup directional ownership repair and final Plan | `0b8a992f9389e39fc07a53e9fb74b7fa1f20368b` (pushed; supersedes the prior `c40cfd8` cleanup authority) |
| This final documentation publication | The pushed commit containing this file; resolve with `git log -1 --format=%H -- qa/SYNDOCAL_PAUSE_HANDOFF_2026-08-26.md`. A self-hash is intentionally not embedded. |

Branch: `codex/syndocal-v1.2`. The current alpha.21 I/O/native source authority
is section 31 at exact commit `536742db968b242164349c34dd6940fe3ced8e92`;
the separate show structural preflight is committed and pushed at exact
`b35d3ba351b29caaedfb7d0c6f4e29fc84580e83`. The historical alpha.20 source
authority is section 29 at `03b70cd14a285a41c63cfd1d9b3bd89c025eec16`. The historical
alpha.18 source checkpoint was
`db4eefc348b01ee05dd2dc87945afa85de8803e` at exact `HEAD`/upstream; its native
artifact/process identity is recorded in section 21, and the direct refresh
click remains unconfirmed. The alpha.17 native runtime/source/artifact remains bound to pushed commit
`fb5d18fdf898a1435bed173ddd17934a04a97897`. The alpha.16 and alpha.15
standard/runtime artifacts remain historical and are bound to their recorded
source commits. The historical alpha.14 runtime
source and native artifact remain bound to
`92122f1b148d40845b2cfe3e4618a57ce132b3df`; a later commit containing only
checkpoint documentation must not redefine either binary's source identity.
Resolve the current branch/docs tip after publishing this handoff; do not report
the artifact source checkpoint as the later documentation HEAD.

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

## 3. Historical alpha.15 standard artifact; historical alpha.14 and local-only Show-ASIO artifacts

### Historical local alpha.15 MIT/WASAPI verification artifact (2026-08-26)

- Source checkpoint: `c609b61c77e44ee028ed7322c29a0cfd04b8182c`
- Path: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
- Product/File version: `1.2.0-alpha.15`
- Size: `58,523,648` bytes
- SHA-256: `58C059A1B52DD61F0E7FD2C51897BE58B933FCE83598CDDEE8903374D05226DC`
- Warning counts: `frontend-typescript-vite-windows` baseline/current is
  `0/0` first-party (`0` third-party), and the authoritative native build
  emitted `0` first-party warning lines.
- Exact process gate: the earlier PID `34792` was resolved to this checkout's
  exact release executable and terminated before build. Process count was `0`
  immediately before the authoritative rerun. The first attempt failed in
  `ffmpeg-sys` because the FFmpeg environment was absent and was not accepted.
  The gate was restarted from the beginning with `FFMPEG_DIR` and
  `LIBCLANG_PATH` restored. Both attempts printed and verified the exact VS
  Community 14.44 x64 linker as pinned and first; Git's incompatible linker was
  second and never selected. `pnpm --dir app tauri build --no-bundle` then
  completed in `2m 51s`.
- The rebuilt executable ran as PID `45724`; its resolved path was exact, it was
  responding, and exactly one maximized `Syndocal` window was available. Native
  QA created a Scene through `Edit -> Lighting -> Bank 1 -> + Scene`:
  the Bank count changed `0 -> 1`, literal project label `New Scene` was
  selected, the Scene settings editor opened, and the status reported
  `Created scene in Bank 1`. The localized accessibility description remained
  `新しいシーン`; it did not rewrite the stored project label.
- The temporary `Untitled.sdc` Scene was not saved. QA ended through the
  protected `破棄して停止して閉じる` path, which discarded the change and
  stopped live output. Exact-checkout `syndocal.exe` process count is now `0`.
- This same-version rebuild is local verification evidence and must not be
  distributed as a new alpha.15 artifact. Advance the prerelease ordinal before
  the next distributed development artifact.
- This proves the alpha.15 local software/native checkpoint only;
  physical output, DJ, representative-scene real-output, and full ASIO gates
  remain open.

### Historical prior alpha.15 standard MIT/WASAPI artifact

- Source checkpoint: `1d372e795870c1a6e5687d1116161042ddac627e`
- Product/File version: `1.2.0-alpha.15`
- Size: `58,523,648` bytes
- SHA-256: `A22BE8BD7CFB9C95D551E24DE51862C352D7B02C2FA22D4E469C2E085CEBC23E`
- The current file at the release path has replaced this artifact locally; this
  identity remains the prior standard checkpoint and is not re-bound to
  `c609b61`.

### Historical alpha.14 standard MIT/WASAPI artifact

- Source checkpoint: `92122f1b148d40845b2cfe3e4618a57ce132b3df`
- Recorded build path: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
  (the current file at this path has since been replaced by the alpha.15 artifact above)
- Product/File version: `1.2.0-alpha.14`
- Size: `58,523,648` bytes
- SHA-256: `B140E9DA515741C8A6A318963C6BAB576CED62ECABEDEA50BA8DE15593AE325C`
- Build timestamp: `2026-08-26T05:07:44.5595600Z`
- Resumed native-QA process: PID `99748`; exact app path as above; responsive
  title `Syndocal`; main HWND `6430754`. Computer Use reselected that exact
  app/window and kept it maximized at `1920x1032`. The temporary and sample
  project changes used below were explicitly discarded rather than saved;
  after Close, the exact-path process count was `0`.

### Historical alpha.14 local-only Show-ASIO artifact: bounded Ampero physical proof

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

## 4. Historical completed software evidence

### Historical alpha.15 native/software checkpoint (2026-08-26)

- Source/artifact checkpoint: pushed, upstream-equal `1d372e795870c1a6e5687d1116161042ddac627e`;
  standard artifact identity and the `windows-native-release` warning ratchet
  are recorded in section 3.
- The exact `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc` input is
  `67,873` bytes with SHA-256
  `22D86D7C0F0C56543B440356F76C467F86C37B0E9D06D76EA8F56B2952D0F841`.
  Native UI navigation reached `Setup > Patch` and displayed the import report:
  `fixtures=46`, `profiles=12`, `fixture_groups=15`, `scene_banks=2`,
  `cues=2`, `values converted=2`, `skipped=0`, `beam records=234`,
  `mismatches=0`, `audio=0`, `scene blocks=0`, `effects=0/0`, `unknown=0`,
  `missing=0`; summary `Converted=84`, `Approximate=4`, `Skipped=0`,
  `Unsupported=1`. Approximate details are MIDI input device affinity and MIDI
  feedback output affinity. The Unsupported detail is the Daslight hardware
  device; imported DMX routes are disabled.
- Save As produced `target/qa/dsf2026-native-alpha15/DSF2026-imported-alpha15.sdc`,
  `1,035,667` bytes, SHA-256
  `CDECBC4D3D3D947C0DA4915009D3480A605E4B4B363C25622907E1D78A6FA6FB`.
  Clean close, restart, and reopen verified `46` fixtures, `12` embedded
  profiles, `2 MIDI / 0 OSC / 0 DMX / 0 DJ Link` mappings, and two scene banks
  with color/dimmer cues `all_white` and `all_max`.
- This closes only the software import/report/Save As/save/reload slice. It does
  not close physical three-output Apply/content/save/restart, representative
  scene real-output acceptance, DJ physical `0/12`, or ASIO unplug/XRUN/fault,
  long-duration, TOPPING, or matrix gates.

Historical alpha.14 evidence:

- rb-output runtime `862cf8035dfb365a7d799f820936585882d0a1e7`:
  full `npm test` `389 total / 387 pass / 0 fail / 2 intentional package
  skips`; Stage-1 plus strict-v3 focused tests `33/33`; changed-JavaScript
  `node --check` PASS; first-party warnings `0`.
- Exact MSVC 14.44 linker was pinned and first in `where.exe link.exe` for all
  Cargo gates. Protocol DJ-Link passed `12/12`; the dedicated rb-output sender
  contract passed `2/2`; I/O `remote_ws` passed `57/57`; Syndocal DJ-Link
  dispatch passed `10/10`; extracted loop-range mapping passed `3/3`.
- `pnpm --dir app run check:release` PASS at the historical alpha.14 metadata, ASIO
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
- Cleanup harness after the directional ownership repair: PowerShell 5.1 and
  7 both `110/110`; first-party warnings were `0`.

Independent Ox results:

- Display stable identity: APPROVE; no P0/P1/P2 after blank-GDI coverage.
- Show-ASIO hardlink topology: initial REJECT identified P0/P1; the final exact
  app/bridge hardlink pair passed follow-up review with no P0/P1.
- Cleanup inaccessible-process metadata: APPROVE; no P0/P1/P2.
- Exact Codex-control-plane exception: final APPROVE; all prior P2-1 through
  P2-5 resolved, no remaining P0/P1/P2. Ox independently reran `93/93` on both
  PowerShell versions.
- Cleanup directional ownership repair: independent Terra re-review found no
  P0/P1/P2 (Ox was unavailable); both PowerShell self-tests passed `110/110`.
- The focused `git diff --check` was clean with only Git's informational
  LF-to-CRLF working-copy notices.

## 5. Cleanup inventory and final result

### Latest directional ownership checkpoint (2026-08-26)

- The initial Plan from clean, upstream-equal `d463381` was blocked by
  `ActiveOwnedWriter` on six StreamDeck-plugin `node` PIDs: `41276`, `40272`,
  `41884`, `37132`, `41588`, and `31272`. The ownership graph
  incorrectly traversed both parents and children, so the unrelated StreamDeck
  branch reached a Codex/shell anchor. Apply was not run and nothing was deleted.
- The fix was committed and pushed as `0b8a992f9389e39fc07a53e9fb74b7fa1f20368b`.
  It adds the exact SystemRoot Explorer boundary and uses directional
  self/ancestor ownership only; generic duplicate-PID and missing-parent/cycle
  process topologies remain fail-closed.
- PowerShell 5.1 and PowerShell 7 self-tests both passed `110/110`; the
  focused cleanup test output emitted `0` first-party warnings. No native build
  or native warning configuration was rerun for this QA-only harness fix.
  Independent Terra re-review found no P0/P1/P2 because Ox was unavailable, and
  `git diff --check` was clean apart from Git's informational CRLF notices.
- The final production Plan from clean, upstream-equal `0b8a992` remained
  blocked by `HardlinkDetected` at
  `C:\Users\kouty\Documents\KDMX\target\debug\incremental\audio-0nuw1tgmhpz4j\s-hln3a7p0dg-1xgadpp-2vnp8sov454s93u5khsbi2d9r\metadata.rmeta`;
  link count was `2`. `Candidates=[]`, `PlannedLogicalBytes=0`, and
  `ReclaimedLogicalBytes=0`; Apply was not run and nothing was deleted.
- Current target inventory is `205,883,610,246` logical bytes, `135,967` files,
  `16,377` directories including root, and `0` reparse points.

The following c40cfd8 inventory and Plan are retained as historical evidence;
they are superseded by the directional-ownership checkpoint above:

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
- Current peer target: branch `beta-v1.1.2`, package version `1.1.9`, clean and
  upstream-equal at exact commit `b03d66a87b8d9dcdedfbd9b5c395bda7df7e0eec`.
  The latest non-Master Deck 2
  router-to-real-MIDI seven-byte proof passed focused `12/12`; the stable suite
  passed `455` total / `453` pass / `0` fail / `2` intentional skips. The commit
  is pushed, clean, and independently reviewed GO. Target-DJ-PC deployment and
  physical acceptance remain open.
  This exact peer identity and the section 1 current-authority table supersede
  section 20 and every older v1.1.6/v1.1.7 operational instruction; section 31
  is the current KDMX I/O/native source authority.
- FOH/Syndocal host: `.50.1`; DJ PC: `.50.2`
- The controlled source path requires the external show JSON through
  `DJ_AGENT_CONFIG_PATH`; a preflight-only pass is not an active Agent session.
- Normal production authority is fresh measured `DJ_LOOP_STATE`, nested under
  `payload.loop`. Only actual no-response after the armed F14 window emits
  `DJ_LOOP_FALLBACK`. Each prediction carries a monotonic intent ID and exact
  measured-revision/effective-division base; late fresh measurement overrides
  and rebases it. The complete profile is
  `8 → 4 → 2 → 1 → 1/2 → 1/4 → 1/8 → 1/16 → 1/32 → 1/64`; it does not stop at 2.
- F13's accepted edge begins HPF and synchronously routes exactly one correlated
  `DJ_RELEASE` before local MIDI completion. Syndocal turns the DJ loop OFF and
  relinquishes DJ clock authority only when the Timeline is already playing; it
  does not seek, jump, start, or change position, playing state, child
  transport, or Follow. Independently, the DJ Agent runs HPF CC16 `64 -> 127`,
  ChannelFader CC17 `127 -> 0`, Cue/Stop Note37, then resets HPF/fader to
  `64`/`127` (both ramps `1000 ms`/`50 ms`). Local MIDI failure never suppresses
  Release delivery.
- DJ-LINK may be started whenever needed. Moving the controller is safe for
  the current software/native checkpoint, but any resulting loss of Rekordbox
  MIDI In remains unverified hardware state and does not satisfy a response or
  no-response row by itself.
- A resumed 2026-08-26 FOH preflight found `イーサネット 4` up at
  `192.168.50.1/24` (`ifIndex 3`, `1 Gbps`), while the target DJ PC `.50.2` did
  not answer one bounded reachability probe. No listener existed on TCP `8787`
  or `9100`; UDP `22346` was not inspected. The exact alpha.15 process was PID
  `34792`, and its only `Syndocal` window was responsive and maximized for Setup
  I/O inspection.
- The FOH machine had neither `C:\SyndocalShow\dj-agent-v1.1.5.json` nor a
  `DJ_AGENT_CONFIG_PATH` value. After explicit operator approval, the
  process-local token was rotated, but its show-once value was not successfully
  copied into the external configuration and was not recorded in logs, QA, or
  Git. `pnpm --dir app run check:dj-link` passed with no first-party warning.
  The next real peer action is to have the `.50.2` DJ PC ready, rotate once more,
  copy the new show-once token only into its checkout-external JSON, launch the
  real `rb-output` session, and verify HELLO/ACK.
- Physical acceptance remains `0/12`: real token, HELLO/ACK, Rekordbox track
  detection, MIDI output, pedal, the full measured/no-response loop profile,
  release, reconnect, and restart are not recorded as accepted.

### SUPERSEDED / HISTORICAL — 2026-08-26 alpha.15 live listener activation

- After the preflight above, the operator-approved live start was verified in
  the exact alpha.15 Syndocal process: PID `34792` owns the TCP listener bound to
  `192.168.50.1:9100`.
- The maximized Setup I/O surface now shows `Remote Stop` and `DJ Link Available`.
  No PIN or token value is recorded in this handoff.
- The approved token rotation completed, but copying the show-once value was not
  verified. The currently running listener is therefore not evidence of a
  usable `.50.2` credential; rotate again only when the real DJ PC is ready.
- This is FOH-side listener evidence only. Physical DJ acceptance remains
  `0/12`; the `.50.2` peer and the real `rb-output` connection are still
  unverified.
- This live observation is limited to the lifetime of PID `34792`. Listener and
  token restoration after stop/restart remain unproven, so HW-4.11 is still
  unchecked within the `0/12` matrix.
- The later native build gate resolved and terminated exact PID `34792` before
  relinking. The rebuilt exact-path QA process was PID `45724`; after restart
  the Setup I/O surface returned to local-only/stopped and no listener existed
  on TCP `9100`. PID `45724` was then closed after native QA, so exact-checkout
  process count is now `0`. This confirms that the observed listener/configuration
  was not restored across process restart. Do not treat the earlier listener as
  active show state. When the real `.50.2` DJ PC is ready, configure the
  listener, rotate once, copy the show-once token directly into its external
  config, and run HELLO/ACK without another intervening Syndocal restart.

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
5. The alpha.15 native `DSF2026.dvc` import/report/Save As/close/restart/reopen
   software slice is complete; retain its visible report and exact
   skipped/fail-closed mappings. Create representative Lighting scenes,
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

## 9. Giant-file debt is partially reduced, not resolved

Current exact metrics:

- `app/src/App.tsx`: `28,722` lines, `1,199,135` bytes, SHA-256
  `78DBC6EAA78ABF4CA890F595EDE74D4BE4B9D2E093A8BDAAB31F7E9958DEED48`
- `app/src-tauri/src/main.rs`: `129,601` lines, `4,972,209` bytes, SHA-256
  `82595C0042D411891836EA3ACC898DBF72E330EC83B7F2921799A7F01897B3EF`
- Alpha.16 checkpoint `ad3faa253c155cb1173b17fb81a711f60baea1f1`
  extracted the any-deck owner/loop runtime into
  `app/src-tauri/src/dj_track_runtime.rs` (`412` lines, `17,228` bytes, SHA-256
  `06F81C2AE850A240A9E1929CFE908F84F06C744FC1C46044CF4FD2E69BD895AB`)
  and its two large regression tests into
  `app/src-tauri/src/tests/dj_track_runtime_tests.rs` (`794` lines, `28,579`
  bytes, SHA-256
  `FEB1876ECEFF0D94C448CB84D80FAAB22A1BF3E72572A6A20E2E4676D0029EAF`).
  `main.rs` remains oversized and requires further bounded extractions.
- Source checkpoint `c609b61c77e44ee028ed7322c29a0cfd04b8182c`
  extracted Scene creation for a Bank from `App.tsx` into
  `app/src/sceneBankSceneCreationController.ts` (`128` lines, `5,254` bytes,
  SHA-256
  `23F38FA011D27EC1659AEE125C4B39705332E0CC3A35ADA1EBE9B29979B7FACC`).
  `App.tsx` is 72 physical lines smaller than its exact parent checkpoint.
  Both Scene-create call sites retain the same callback. The controller keeps
  the browser Scene Matrix fixture path and the native post-flush
  epoch/revision/checkpoint-hash/owner fence, stale-ack rejection, refreshed
  new-Cue difference check, selection, settings surface, and visible error
  behavior.
- `app/scripts/check-project-transaction.mjs` now checks the extracted module
  and its exact 16 required dependencies instead of relying on the removed
  inline function as a source boundary. Its final shorthand-property check
  rejects swapped setter wiring. Independent implementation review first found
  the stale checker boundary, independent re-review found the setter false
  positive, and both were repaired; final review reported no P0/P1/P2.
- Focused proof at this checkpoint: `check:project-transaction` PASS,
  `tsc --noEmit` PASS, `check:frontend-invokes` PASS with `419` exact commands,
  `check:timeline-source-shelf` PASS, `check:warnings -- --configuration
  frontend-typescript-vite-windows` PASS with baseline/current
  `0/0` first-party warnings, `git diff --check` PASS, native build PASS, and
  the native Scene-create path described in section 3 PASS. The dedicated
  Scene Matrix CDP viewport attempt remains unverified because its initial
  `Page.navigate` timed out at 15 seconds; it is not counted as a pass. The
  focused Timeline source-shelf viewport/parity route did pass, and no CDP/Vite
  listener was left running.
- Post-source-commit inventory: the checkout contains `207,327,554,715` bytes,
  of which `target` contains `205,883,767,495` bytes and `app/node_modules`
  contains `545,336,685` bytes. No deletion was performed: the recurring
  cleanup target set is not yet eligible under the exact-path/adversarial-review
  gate, and the current release/QA evidence must remain protected. Cleanup stays
  open rather than treating this large inventory as disposable by assumption.
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
- Resume checkpoint `dbbaffe677cb91211fc73b0b644f8ee34ef1a997`
  extracted the inline `gpu_surface` test module without changing the runtime
  GPU path. `crates/video/src/gpu_surface.rs` is now `1,120` lines / `43,552`
  bytes (SHA-256
  `1FDF9D2BB287156876B2748BD1EFFC45BD6B917062D8653C3D733BCBD537A3C8`),
  and the six tests live in `crates/video/src/gpu_surface_tests.rs` (`387`
  lines / `13,254` bytes, SHA-256
  `19F530368D17875163A8F52D74345179E95AF6224D3A6D4284019621AAD5A171`).
  Independent Terra review found no P0/P1 behavior or module-visibility issue.
  `cargo fmt --all -- --check` and staged `git diff --cached --check` passed.
  In a fresh `cmd.exe /d /v:on` session, `vcvars64.bat -vcvars_ver=14.44`
  initialized the exact Community toolset, the explicitly pinned
  `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` was the required
  `14.44.35207\bin\Hostx64\x64\link.exe`, and `where.exe link.exe`
  returned it before Git's linker. `cargo test -p video --lib
  gpu_surface::tests --locked -- --nocapture --test-threads=1` passed `6/6`
  with no adapter/compositor skip message and zero first-party warnings. This
  is a test-module extraction checkpoint, not native output presentation or
  physical projector acceptance.

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
- Current alpha.17 source checkpoint
  `fb5d18fdf898a1435bed173ddd17934a04a97897`; alpha.16, alpha.15, DVC
  controller, and alpha.14 entries remain historical checkpoint-chain records.
  The rb-output v1.1.7/v1.1.6 entries are likewise historical. The DJ peer tip
  was clean and upstream-equal when each recorded checkpoint was made. The final documentation
  commit containing this handoff must again be pushed and rechecked clean.
- The historical exact standard alpha.14 process and window identities are in
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
acceptance. It preserves the verified alpha.15 standard-native artifact and DSF
software import/report/save/reload slice, the bounded alpha.14 local Show-ASIO
Ampero proof, and the shortest exact route to the remaining physical show proof.

## 11. 2026-08-26 Web Remote disclosure-scroll checkpoint

- The prior Setup I/O route let expanded DJ Link / Remote disclosures grow the
  Web Remote zone without an owned scrollport. In a maximized window, the fixed
  desk then clipped controls above the visible area and the operator could not
  scroll back to the Web Remote header/actions.
- The corrected route keeps `.remoteControl` inside its assigned desk row and
  gives only its `.ioDisclosureStack` the remaining-height vertical scrollport.
  Typography, control geometry, and hit targets are unchanged. The focused
  contract lives in the extracted
  `app/scripts/remote-disclosure-scroll-contract.mjs` module rather than adding
  more inline logic to the already oversized viewport runner.
- Focused CDP proof passed at `1920x1032` and `1280x802`: document/app scroll
  stayed zero, the Remote disclosure stack scrolled, the Web Remote header and
  action remained reachable, and the action hit-test stayed centered. The
  localization inventory remained `3553/3553`; the
  `frontend-typescript-vite-windows` warning ratchet remained `0 -> 0`.
  `node --check` for both runner and extracted module plus `git diff --check`
  passed. Independent Terra xHigh adversarial review reported no P0/P1 finding;
  its P2 suggestions are future strengthening only.
- Native build and maximized native QA are deliberately still open. Performing
  the mandatory native build now would require stopping this checkout's running
  Syndocal and regenerating its process-local DJ token, breaking the operator's
  live Rekordbox/DJ-Link session. This checkpoint therefore proves the focused
  frontend contract only and must not be promoted to native acceptance.

## 12. SUPERSEDED / HISTORICAL — 2026-08-27 HW-4.11 source checkpoint and exact resume state

Historical record only; section 20 is the current resume authority and supersedes
the alpha.15/alpha.16 action state recorded here.

At that checkpoint, the branch was `codex/syndocal-v1.2`; implementation commit
`dcf6e524eddfaf79a54856af458efef202c079e1` is pushed and equals
`origin/codex/syndocal-v1.2`. This checkpoint replaces the prior source-level
restart/NIC gap with a V2 machine-local authority, separate Windows Credential
Manager primary/rollback secrets, generation high-water, crash-compensated
journal, durable disarm cleanup retry, exact NLM network/adapter/IPv4 trust, and
DJ-only auto-start through the one shared Remote listener. Invalid/future/V1,
ambiguous, stale, partially committed, or unsupported state fails closed.

The same tranche also closes the remaining authored-label localization leak for
`New Scene` in both Cue Pads, the Timeline Armed display, and the Phase-1 report;
only persisted labels are marked `data-no-localize`, while `Empty`, `Armed`, and
the New Scene operation remain localizable UI copy. The previously committed
Timeline context menu still dismisses on outside left pointer/Escape and uses a
compact hierarchical menu. The Remote pane retains original control sizes and
uses its disclosure stack as the internal scroll owner.

Supervisor and independent Terra evidence on the fixed source:

- exact Community MSVC `14.44.35207` linker was pinned and first in
  `where.exe link.exe` before every Cargo gate;
- `cargo check -p syndocal --no-default-features --locked` passed;
- focused Rust tests passed: Syndocal `119/119`, I/O `38/38`, protocol `13/13`,
  and control-plane freeze `1/1` with exactly `482` routes;
- first-party warning count was `0` for every executed configuration;
- `pnpm --dir app build`, DJ Link/output-control/invoke/routing/localization,
  five-size disclosure scroll, and Timeline performance/context-menu checks all
  passed. Localization is `3560/3560`; frontend invokes are exactly `422`;
- independent Terra xHigh adversarial review found no blocking P0/P1/P2 issue.

During test diagnosis, only verified debug workers were terminated: PIDs
`59872`, `61748`, `59696`, and `26320`, all exact
`target\debug\deps\io-...exe` processes. The then-current release process was never
terminated or restarted. Historical live evidence is PID `46120`, exact
`target\release\syndocal.exe`, responsive `1.2.0-alpha.15`, size `58,523,648`,
SHA-256
`42D7B5AEECD520855D4645DF6178E2DE617EC1B7D794A3010750DE94912BE33E`,
LISTEN `192.168.50.1:9100`, ESTABLISHED peer `192.168.50.2:58211`. Do not relabel
this artifact as alpha.16.

Cleanup inventory was read-only. No Apply ran, no path was deleted, and
reclaimed bytes are exactly `0`. `target` is `225,690,640,640` logical bytes;
the reviewed `target\debug\incremental` candidate is `85,807,780,400` logical
bytes. Cleanup remains blocked by the dirty-worktree phase of this checkpoint
and `HardlinkDetected`: the cited `metadata.rmeta` now has four names, including
two under external `C:\Users\kouty\Documents\.tmp.driveupload`. Preserve QA,
evidence, and `C:\TEMP\KDMX-alpha16-release-gate-01a03b78`.

Do not run a native release build while the operator intends to retain the live
session. On operator return, first re-resolve PID/path/hash/socket and repository
equality. Then, with explicit acceptance of stopping only that exact checkout
process, run the mandatory alpha.16 `tauri build --no-bundle`, launch/maximize
exactly one responsive Syndocal window, deploy rb-output `1.1.6` to the DJ PC,
and execute HW-4.1 through HW-4.12. Native alpha.16, real Credential Manager/NLM
restart, DJ-PC token reuse, hardware pedal/MIDI, ASIO, three displays, DSF
output, and full rehearsal remain unverified; the DJ matrix remains **0/12**.

## 13. SUPERSEDED / HISTORICAL — 2026-08-27 bounded DJ machine test-module extraction

Commit `684ecc01ceba141047f49cc595337f138719b749` is pushed and equal to
`origin/codex/syndocal-v1.2`. It is a pure post-HW-4.11 maintainability
checkpoint: the inline `dj_link_machine.rs` test module moved to
`app/src-tauri/src/tests/dj_link_machine_tests.rs` while remaining the same
private child module through `#[path = "tests/dj_link_machine_tests.rs"]`.
Production visibility, entrypoints, persistence behavior, and acceptance state
did not change. The production file shrank from `2,644` to `1,491` lines; all
`38` test names, order, `125` assertion-macro invocations, and comments were
retained.
Independent Terra comparison found only two rustfmt line-wrap differences and
no P0/P1/P2 issue. Exact MSVC 14.44 focused proof passed `37/37` on Windows;
the remaining unchanged test is `cfg(not(target_os = "windows"))`. `cargo fmt
--all -- --check`, `git diff --check`, and the warning ratchet passed with zero
first-party warnings. No native build, process restart, cleanup Apply, or file
deletion occurred; reclaimed bytes remain `0`. PID `46120` remains responsive
with the same LISTEN and ESTABLISHED DJ-PC socket. HW-4 remains **0/12**.

## 14. SUPERSEDED / HISTORICAL — 2026-08-27 post-DJ P0 source checkpoint

Branch `codex/syndocal-v1.2` is pushed through
`627e35b32008c4087bd344f6531ccd6a5707d13d`. Commit `3391200` bounds
dead-owner terminal project-receipt compaction and recovers the fixed 256-slot
admission capacity without weakening Pending, active, indeterminate, ABA, or
owner-incarnation authority. Commit `627e35b` removes the obsolete backend and
frontend last-FX rejection so an effect-only Scene may publish, save, and reload
with zero owned FX through the authoritative command. Invalid effect targets
continue to reject unchanged; the frontend presents a visible empty-list
warning beside the enabled Save Recall action, and obsolete helper/copy/style
paths are gone.

Supervisor gates passed: exact-linker transaction tests `5/5` and `10/10`, two
focused Syndocal zero-FX tests, the engine published-clear test, transaction and
recall checkers, localization `3560/3560`, TypeScript/Vite build, Rust format,
and diff checks. First-party warnings were `0` for the Cargo configurations.
Independent Terra xHigh backend and frontend reviews returned no P0/P1/P2 after
three frontend P2 observations were repaired and re-reviewed.

A read-only HEAD audit also confirms that the previously stale SHOW-P0-2,
FC-26, and FC-27 source findings were repaired in `aed77a2`: Bank rename uses a
typed strict request, a non-null missing Bank stays unavailable and locks its
dependent actions, and malformed active Cue references reject before queue or
runtime activation. The existing focused gates were inspected but not rerun in
that audit; fresh exact-HEAD native proof remains open for every row.

At that dated checkpoint, the independently approved physical operator companion
still described v1.1.6 direct Stop. Section 18 and the then-updated
`qa/DJ_HW4_OPERATOR_RUNBOOK_2026-08-27.md` required an exact clean peer
identity, kept the token out of evidence, and fixed then-current HW-4.6 to the
v1.1.7 `filter-then-stop` contract while distinguishing direct hardware proof
from the bounded fault harness. This is historical context, not acceptance
evidence: HW-4 remains **0/12**.

The following is historical pre-alpha.16-build evidence superseded by the later
native checkpoint: PID `46120` was the responsive historical
alpha.15 executable at the exact checkout path, with LISTEN
`192.168.50.1:9100` and ESTABLISHED peer `192.168.50.2:58211`. Do not stop it,
run the mandatory native release build, or relabel it as alpha.16 until the
operator returns and explicitly authorizes replacing the live session. Native
alpha.16, Bank/Scene authoring/save/restart, zero-FX native acceptance, target
DJ-PC deployment identity, real Credential Manager/NLM restart, HW-4, ASIO,
three displays, representative DSF show content, and the full rehearsal remain
open.

## 15. SUPERSEDED / HISTORICAL — 2026-08-27 bounded media/audio playback test-module extraction

Commit `6cfb71c4e23141103e5ce33542b404d759828e95` is pushed and equal to
`origin/codex/syndocal-v1.2`. It replaces the 1,595-line inline
`media_audio_playback_tests` block in `main.rs` with a private test-only path
module at `app/src-tauri/src/tests/media_audio_playback_tests.rs`. All 34 helper
and test names/order, 31 tests, and 166 assertion macros remain; normalized body
comparison found only rustfmt wrapping differences. `main.rs` is now 128,582
lines instead of 130,174.

With exact MSVC 14.44 initialization, the absolute pin
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`,
and `where.exe link.exe` first-path verification, the exact command `cargo test
-p syndocal --no-default-features --locked media_audio_playback_tests` passed
`31/31`, 1,099 filtered, and zero first-party warnings. Format/diff gates passed
and independent Terra xHigh review returned no P0/P1/P2. No native release/Tauri build,
release-process stop, cleanup Apply, or hardware operation occurred. The live
alpha.15 PID `46120` and DJ socket remain protected; HW-4 stays **0/12**.

## 16. SUPERSEDED / HISTORICAL — 2026-08-27 strict Live Audio IPC V1 checkpoint

Commit `32cc47515bffb25c8a86e9c599ef98aeabcf7878` is pushed and equal to
`origin/codex/syndocal-v1.2`. The retired renderer/native boundary accepted
flat command arguments and the internal snake_case representation. The only
supported boundary now requires the exact outer `{ request: { ... } }` shape,
`schemaVersion: 1`, camelCase fields, present nullable keys, exact backend and
channel-mix discriminants, and no unknown fields. Invalid, direct, raw,
snake_case, future-version, missing-nullable, or extra-field payloads reject
before AppState, owner/external/coordinator locks, device-catalog access, or
stream lifecycle work. There is no legacy alias or permissive fallback.

The boundary is extracted into
`app/src-tauri/src/live_audio_ipc_v1.rs` (`520` lines) and
`app/src/liveAudioInputIpcV1.ts` (`283` lines); `App.tsx` only wires the three
request builders into the four production calls. The initially proposed
unused frontend status mapper/parser was removed rather than adding a second
response representation. The browser fixture independently rejects the
retired nested `stereo_pair` wire shape and no longer converts an unknown mix
to `average_all`.

Supervisor proof passed: `check:live-audio-ipc-v1`, `check:live-audio`,
`check:timeline-cue-audio`, `check:timeline-cue-audio:browser`, TypeScript/Vite
build, Rust format, staged diff check, and the live-audio browser matrix in
English and Japanese at `1920x1080`, measured-client `1920x1032`,
`2048x1152`, `1366x768`, and `1280x720`, all with no failed checks. The
frontend warning ratchet remained baseline/current `0/0`. After exact
`vcvars64.bat -vcvars_ver=14.44` initialization, the required Community
`14.44.35207` absolute linker pin and first `where.exe link.exe` result were
verified, then the no-default-feature focused Rust suite passed `5/5`, 1,130
filtered, with zero first-party warnings. Independent Terra xHigh backend and
frontend re-reviews both returned no P0/P1/P2.

This is a source/browser checkpoint, not native acceptance. PID `46120` was
re-resolved as the same responsive historical alpha.15 executable with the
same size/hash, LISTEN `192.168.50.1:9100`, and ESTABLISHED
`192.168.50.2:58211` peer; it was not stopped or rebuilt. Fresh alpha.16 native
proof remains open. FC-09 was still open at this exact checkpoint and was
closed by the later checkpoint below.

## 17. SUPERSEDED / HISTORICAL — 2026-08-27 FC-09 unavailable audio-input selection checkpoint

Commit `a65e3ccb4a8dd14f4941109d06e1644876329a8c` is pushed to
`origin/codex/syndocal-v1.2`. The retired behavior could silently substitute
WASAPI Shared or the first returned backend when an explicitly selected input
disappeared, and a failed device catalogue could leave a saved `ready` request
usable. The supported path now retains the exact missing backend/device
identity, downgrades saved readiness to `stale`, and locks Start before every
control/start IPC boundary. Invalid, duplicate, wrong-backend, missing, or
failed catalogues remain typed failures; no backend/device substitution is
performed.

Cold restore retains only the persisted stable device name and label for the
renderer's unavailable option; generation-scoped device IDs are not persisted
or synthesized. A successful passive refresh cannot re-arm a selection after
a catalogue failure, even when the same name/label returns with a new ID. Only
an explicit exact current-device selection may revalidate it. The stale option
is disabled, and a forged stale input event rejects before capability IPC,
downgrades any saved ready state, and leaves Start locked.

The browser acceptance was extracted from the already oversized viewport
runner into
`app/scripts/live-audio-backend-disappearance-contract.mjs`; the runner keeps
only the fixture/dispatch seam. A named `check:live-audio-restore` package gate
now executes that contract. Supervisor proof passed:

- syntax checks for the static checker, extracted contract, and runner;
- `check:live-audio`;
- `check:live-audio-restore`, including cold-first-failure and passive-recovery
  EN/JA cases, raw storage byte equality, and unchanged backend/device/
  capability/Start IPC counts;
- `check:live-audio-viewport` in English and Japanese at `1920x1080`, measured
  client `1920x1032`, `2048x1152`, `1366x768`, and `1280x720`;
- TypeScript/Vite production build and `git diff --check`.

All executed frontend gates reported zero first-party warnings. The final
independent Terra xHigh review reported P0/P1/P2 none. No Cargo/Tauri native
build or test was run: the protected historical alpha.15 process remains PID
`46120` at this checkout's exact `target/release/syndocal.exe` path and was not
stopped. Its DJ socket was not re-probed in this checkpoint. Fresh alpha.16
native/maximized UI proof, current final ASIO DLL/operator proof, the DJ HW-4
matrix, three displays, representative DSF content, and the full rehearsal all
remain open.

## 18. SUPERSEDED / HISTORICAL — 2026-08-27 rb-output v1.1.7 controlled-source checkpoint

The DJ peer clean-break is pushed at
`2577496767cf4ca8c8abdcadddbb891c7a609a32` on `beta-v1.1.2`; source and upstream
are equal at package version `1.1.7`. It retires the v1.1.5/v1.1.6 active
configuration and mapping paths. Current deployment requires only
`C:\SyndocalShow\dj-agent-v1.1.7.json` plus
`server/public/setup/CustomMIDI1-Syndocal-v1.1.7.csv`, exact
`syndocal-envelope-v3`, and a show-once token that is never recorded in QA.

Stage 1 F13 is now the reviewed `filter-then-stop` contract: owner-channel HPF
CC16 `64 -> 127` over `1000 ms`, one planned Cue/Stop Note37, one independently
correlated `DJ_RELEASE`, then best-effort CC16 reset `64`; channel-fader/fade MIDI
is unreachable. F14 consumes measured Rekordbox loop state down through `1/64`
and uses prediction only after true non-response. Stage 2 stays Syndocal-only and
emits zero Rekordbox MIDI. Exact mapped actual playback on any intended deck is
the trigger; Rekordbox MASTER remains diagnostic and cannot steal or retrigger
the admitted owner.

Fresh peer proof passed `406` total tests: `404` passed, `0` failed, and the two
REAL-package opt-in cases were skipped. Staged syntax passed `30/30`, focused
runtime/release/config/launcher proof passed `17/17`, and independent final
review reported P0/P1/P2 none. The commit contains source acceptance only; stale
`dist` and missing v1.1.7 release artifacts remain an explicit distribution
boundary. The target DJ PC still must pull this exact commit, create/validate the
external v1.1.7 configuration, import the exact CSV into licensed Rekordbox, and
execute HW-4.1 through HW-4.12. No hardware checkbox is promoted here: the
matrix remains **0/12 checked (0%)**.

## 19. SUPERSEDED / HISTORICAL — 2026-08-27 alpha.16 standard native build and maximized launch

The prior exact-checkout alpha.15 process was re-resolved at
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe` as sole PID `46120`
and only that process was force-terminated immediately before the release build.
Rekordbox, the DJ Agent, Daslight, and any unrelated Syndocal process were not
terminated.

The first build attempt correctly initialized `vcvars64.bat -vcvars_ver=14.44`,
pinned
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`,
and printed that path first in `where.exe link.exe`, but failed before product
linking because the installed shared FFmpeg SDK and `libclang.dll` were not
exposed. That attempt is invalid as completion evidence. The gate was restarted
from its beginning with the same exact linker checks plus the validated SDK root
`C:\Users\kouty\AppData\Local\Microsoft\WinGet\Packages\Gyan.FFmpeg.Shared_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg-8.1.2-full_build-shared`
as `FFMPEG_DIR`, its `bin` prepended after vcvars via delayed expansion, and
`C:\Program Files\LLVM\bin` as `LIBCLANG_PATH`.

`pnpm --dir app tauri build --no-bundle` then passed from clean,
upstream-equal source `15b2d3f8886d11d571b2fc804ca9b774afff4746` in `2m 53s`.
The build output contained zero first-party warnings. The standard artifact is:

- path: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`;
- Product/FileVersion: `1.2.0-alpha.16`;
- size: `59,024,384` bytes;
- SHA-256: `CB2CCA8102F71C187A35F645EAEBB6C5A99052A48F2575B59782D44D57274C4C`.

The exact executable was launched once as PID `74772`. Process inspection found
exactly one checkout-owned `syndocal.exe`, `Responding=True`, title `Syndocal`.
Computer Use independently selected the sole window owned by that exact process,
confirmed the native restore button (`元のサイズに戻す`) and a `1920x1032`
client screenshot, then activated it. The I/O workspace remained rendered and
responsive. After the operator expanded DJ Link, the same native screenshot
showed both the Web Remote heading and DJ Link controls within the right-pane
contained scroll region. This proves the new standard artifact launches as one
responsive maximized window and covers the observed 1920 containment state; it
does not yet prove the full scroll traversal at every viewport.

DJ Link was not armed at this checkpoint, so the new process owned no DJ listener
socket and no HW-4 row was attempted. The target DJ PC has pulled peer v1.1.7 but
still must create/configure the external v1.1.7 JSON, clear forbidden legacy
environment overrides, import/confirm the exact MIDI mapping and port, pass
preflight, then launch. The current standard native artifact also does not close
the separately licensed Show-ASIO fault/endurance/latency matrix, three-display
acceptance, representative content, or full rehearsal.

## 20. SUPERSEDED / HISTORICAL — 2026-08-27 alpha.17 native and DJ peer v1.1.8 resume checkpoint

The alpha.17 KDMX source checkpoint is pushed and upstream-equal at
`fb5d18fdf898a1435bed173ddd17934a04a97897`, product version
`1.2.0-alpha.17`. Focused engine `dj_link_` proof passed `25/25`, Syndocal
`dj_link_dispatch_` passed `8/8`, the three-display harness passed `80/80`, the
frontend build passed, detached `check:release` passed including `169` ASIO
packaging assertions, and first-party warnings are `0`. The exact native
artifact is:

- path: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`;
- Product/FileVersion: `1.2.0-alpha.17`;
- size: `59,021,824` bytes;
- SHA-256: `8B35A0F89ED6FA9A1BF8B1929BFA323F7F6250DF059D6314CCE7DDD6D39EBE45`;
- exactly one responsive maximized `Syndocal` window, PID `57640`.

The detached alpha.17 release gate at
`C:\TEMP\KDMX-alpha17-release-gate-01a03b78-2` passed `check:release` and was
removed from Git worktree registration. Its directory removal was rejected by
the safety policy, so the unregistered regenerable copy remains as a cleanup
item at `394,438,512` logical bytes. The active checkout immediately passed
`pnpm install --frozen-lockfile` and `pnpm --dir app build` afterward, verifying
that dependency state was unaffected; no cleanup bytes are claimed yet.

The current DJ-PC target is branch `beta-v1.1.2`, package version `1.1.8`, clean
and upstream-equal at exact commit
`0f3e8c6851857c8542c132a89a7d44289002b1f5`, exact adapter `syndocal-envelope-v3`,
and the checkout-external `C:\SyndocalShow\dj-agent-v1.1.8.json` plus
`CustomMIDI1-Syndocal-v1.1.8.csv`. Production remains on version `1.1.8`.
The latest non-Master Deck 2 router-to-real-MIDI seven-byte proof passed focused
`12/12`; the stable peer suite passed `415` total / `413` pass / `0` fail /
`2` intentional skips. The commit is pushed, clean, and independently reviewed
GO.

`RB-1.1.8-CHECKPOINT`: external rb-output v1.1.8 is committed, pushed, clean,
and independently reviewed at the exact identity above. This completes the peer
source/full-suite gate only; target-DJ-PC deployment and HW-4 remain unverified.

The external source worktree is clean, but its stale ignored/untracked
`C:\Users\kouty\Desktop\rb-output\dist` is `277,382,202` logical bytes. Exact
recursive deletion was policy-blocked before execution, so reclaimed bytes are
`0`; remove or regenerate this directory before the release seal.

The accepted Stage 1 F13 contract is edge-triggered: HPF starts and exactly one
correlated `DJ_RELEASE` is routed synchronously before local MIDI completion.
Syndocal then turns the DJ loop OFF and relinquishes DJ clock authority only when
the Timeline is already playing; it performs no seek, jump, start, or transport
change, so position, playing state, child transport, and Follow remain intact
and the local clock naturally progresses from the current playhead. Independently,
the DJ Agent completes HPF CC16 `64 -> 127`, ChannelFader CC17 `127 -> 0`, and
Cue/Stop Note37, each ramped over `1000 ms` with `50 ms` updates, then resets HPF
to `64` and fader to `127`. Local MIDI failure never suppresses Release delivery;
duplicate Release is idempotent.

The existing Stage 2 `timeline-control` boundary remains F13/F15 Timeline
`-4/+4` beat-jump, F14 absolute Timeline loop, and zero Rekordbox MIDI. This
boundary is unchanged and still requires direct confirmation; it is not the
Stage 1 Release path. The DJ/Pedal matrix remains **0/12 checked (0%)** and the
alpha.17/native evidence above does not promote physical acceptance.

## 21. 2026-08-27 historical alpha.18 source/UI authority

This section is retained only as dated alpha.18 source/native evidence. The KDMX
source checkpoint was `1.2.0-alpha.18` at branch `codex/syndocal-v1.2`, exact
`HEAD`/upstream `db4eefc348b01ee05dd2dc87945afa85de8803e`. The alpha.17 source,
native artifact, process identity, and hash in section 20 remain historical and
are not re-bound to alpha.18. The required `pnpm --dir app tauri build
--no-bundle` passed with exact MSVC 14.44 linker-first setup and zero first-party
warnings. The resulting alpha.18 artifact is:

- path: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
- Product/FileVersion: `1.2.0-alpha.18`
- size: `58,740,224` bytes
- SHA-256: `841068E08F80EB877FBA919FB86D3F52B3D4444314B47ABEF995BFA593E8D4F9`
- LastWriteTimeUtc: `2026-08-27T05:38:40.1840641Z`
- launch: exactly one responsive process, PID `80264`, title `Syndocal`, window handle `854080`, `IsMaximized=true`, start `2026-08-27T14:38:57.8350806+09:00`

The native screenshot confirmed Web Remote/Security/Endpoints/DJ Link/Standby
in the same disclosure stack and showed two wired candidates. The attempted
native wired-binding refresh click remains unconfirmed because foreground PID
retrieval failed. The five-viewport setup harness independently confirmed
listener empty→count `0`→Ethernet4 `192.168.50.1`→count `1`, one invoke per
phase, and disabled mutation controls; this does not promote the native click
to accepted. This section does not promote physical acceptance.

The existing alpha.17 native artifact returned
`wired_candidate_discovery_failed` during wired refresh; its typed live
diagnostic exposed the old `Structural DuplicateIpv4Address` code. The root
cause was a typed `sin_addr` read against the `SOCKADDR_IN` `+8` padding. The
source fix corrects that layout handling and drops COM objects before
`CoUninitialize`. Exact MSVC/FFmpeg live-unit and hardware-enumeration checks
passed and identify Ethernet4 `192.168.50.1` as eligible. The native build and
maximized-window visual gate are recorded above; only the direct refresh-button
action remains unconfirmed.

The Web Remote source layout now uses the same connection disclosure stack as
DJ Link and Endpoints without shrinking controls; native visual confirmation is
recorded above, while the direct refresh-button click remains unconfirmed. The
standard and dedicated Setup I/O browser contracts pass all
five viewports, including `1280x720`. Independent review found and closed an
adjacent fail-closed defect where rejected DMX network-route buttons mutated the
protocol draft before reporting `no state changed`; the candidate-only path now
leaves the full draft and all `128` route signatures unchanged and invokes no
retired output command. Remote Start/Stop is asserted exactly once and in order;
independent re-review is GO with P0/P1/P2 all zero. Focused root revalidation of the MASTER clean break passed
protocol `7/7`, runtime `5/5`, I/O `37/37`, frontend/build, live, and static
checks under the required single-thread standard gate; independent review is GO.
A parallel I/O race is baseline-existing and is not acceptance evidence;
the exact MSVC 14.44 / locked full-workspace rerun passed `1164` pass / `0` fail
/ `11` intentional hardware-media ignores in the Syndocal binary target with
zero first-party warnings. The touched full-gate fixture repairs were
independently reviewed GO with P0/P1/P2 all zero; frontend invokes are `422`,
localization is `3559`, and the frontend production, viewport/setup, and warning
ratchets are green. The clean release gate `4fc443d` passed after staging seven
pinned DLLs. Main-checkout `check:release` remains intentionally blocked by the
tracked, unchanged runtime-inventory hard-link alias to
`C:\Users\kouty\Documents\.tmp.driveupload\867492`; no alias deletion or file
replacement was performed. Native build/launch/layout integration is verified;
the direct refresh-button action remains unconfirmed.

`app/dist` has already been freshly rebuilt and its old stale marker is `0`.
The ignored peer `dist` remains stale at `277,382,202` bytes but is outside the
production/source checkpoint. Cleanup of the prior alpha.17 temporary tree
(`394,438,512` bytes) remains policy-blocked with no bytes reclaimed. The
reviewer baseline
`C:\Users\kouty\AppData\Local\Temp\kdmx-head-baseline-review-20260827-1246`
is now `49,009,359` bytes after `885.6 MiB` was reclaimed by `cargo clean`;
direct cleanup remains policy-blocked. HW-4 remains **0/12**; this continuation
does not claim native or physical acceptance.

## 22. 2026-08-27 Stage 2 / Release worker-TOCTOU checkpoint

This checkpoint starts from branch `codex/syndocal-v1.2`, exact base
`HEAD`/upstream `7196390fa3b4a9d923f2d8e244b187a8096855dc`. The owned source
set is limited to `crates/engine/src/lib.rs`,
`crates/engine/src/tests/dj_link_release.rs`,
`app/src-tauri/src/main.rs`, `app/src-tauri/src/dj_track_runtime.rs`, and
`app/src-tauri/src/tests/dj_track_runtime_tests.rs`. Other UI and mapping-
authority changes remain separate dirty work and are not part of this
checkpoint.

`EngineCommand::DjLinkRelease` and the absolute Stage 2 loop command now carry
the exact expected Timeline identity. The single engine worker revalidates that
identity and `timeline_playing` immediately before mutation, so a queued stop or
Timeline restart/swap cannot release, loop, or beat-jump a replacement Timeline.
The app boundary independently requires an active admitted track, authoritative
Running state, DJ pedal ownership, exact Timeline/play-session correlation, and
a non-released runtime before enqueue. Only an exact prior Timeline-owned,
loop-off Release receipt is an idempotent replay; foreign or incomplete replay
state fails closed.

The exact MSVC 14.44 Community linker was pinned and first in `where.exe`.
`cargo test -p engine dj_link_ -- --nocapture --test-threads=1` passed
**27/27**, `cargo test -p syndocal dj_link_ -- --nocapture` passed
**118**, failed **0**, ignored the one intentional live-network test, and both
configurations emitted **0 first-party warnings**. `cargo fmt --all -- --check`
and `git diff --check` passed; the latter emitted only Git LF-to-CRLF notices.
Independent Terra xHigh adversarial review is **GO**. Ox was unavailable, so
this is the recorded narrow review exception.

No native release build or deployed Syndocal process contains this checkpoint
yet. The currently running alpha.18 hardware observation proves only the Stage 1
track-admit and F13 Release path; it does not accept Stage 2, the new worker
TOCTOU fence, project-mapping persistence, or the requested `title contains`
show selector. The next safe action is to finish those separate source tranches,
advance the prerelease version, then run the exact native build/launch/maximized
window gate before Stage 2 hardware testing.

## 23. 2026-08-27 project-control mapping persistence race checkpoint

This source checkpoint starts from branch `codex/syndocal-v1.2`, exact
`HEAD`/upstream `8b34b40054cab21c7fae96929bab7ff95ae48ae2`. Its owned change is
limited to the project-control mapping authority bridge, its narrow
`App.tsx` integration hunks, `projectAuthority.ts`, and the focused authority
checker. The separate DJ wired-refresh/UI/localization/viewport dirty hunks are
not staged in this checkpoint.

The old path could durably commit mapping image B, receive an ordinary poll for
the same B before the original R1 IPC reply, then reject the stale reply and
abort Save even though B was already authoritative. The new path permits
exactly one R2 acknowledgement only when the returned and current project
epoch/revision/checkpoint hash are identical and the project identity was not
replaced. A foreign token, CAS/validation/worker failure, identity replacement,
or a second stale result remains untrusted and fail-closed. The explicit flush
owns and cancels only its own zero-delay retry; ordinary autosave remains live.

The retry/flush/publication and media terminal logic was extracted from the
oversized `App.tsx` into `projectControlMappingsAuthorityBridge.ts`. The final
production App chunk is **499.51 kB** (gzip **152.58 kB**) and the prior Vite
chunk warning is gone without raising the warning limit. Focused
`check:project-transaction`, `check:frontend-command-routing`, `check:dj-link`,
`check-project-publication-e4`, TypeScript, and frontend production build gates
passed. `git diff --check` passed with only Git LF-to-CRLF notices. The modified
frontend configuration emitted **0 first-party warnings**. Independent Terra
xHigh adversarial review is **GO**; Ox was unavailable, so this is the recorded
narrow review exception.

No native build or real Tauri transport test was run for this source checkpoint.
The next safe action is to land the requested deterministic
`titleContains = 人生オーバー` owner selector separately, then advance the
prerelease and run the required exact native build/launch/maximized-window and
physical save/reload gates.

## 24. 2026-08-27 DJ wired-refresh feedback / disclosure checkpoint

This frontend-only checkpoint starts from branch `codex/syndocal-v1.2`, exact
`HEAD`/upstream `b1b8783`. It owns only the DJ wired-refresh state/feedback,
Remote panel rendering, localization, and focused DJ Link/viewport assertions.
It does not include the separately running `titleContains` implementation.

The wired-binding refresh now exposes a busy state, disables and guards a
duplicate click, and renders accessible live status for refresh-in-progress,
candidate count, zero candidates, and a typed failure. An actual browser double
click produced exactly one `list_dj_link_wired_candidates` invoke. Web Remote
remains a peer in the same `ioDisclosureStack` as Security, Endpoints, DJ Link,
and Standby; the stack owns scrolling and all actions remained reachable and
hit-testable without shrinking typography, controls, spacing, or targets.

`check:dj-link` and `check:localization` passed; localization is **3559/3559**
with **0** unprotected user-data labels. The isolated `1280x720 --setup-io-only`
browser gate passed with remote and stack scrolling present and no failed
assertions. The frontend production build passed with App **499.51 kB** and no
Vite chunk warning. `git diff --check` passed with only Git LF-to-CRLF notices;
first-party warnings are **0**. Independent Terra xHigh review is **GO** with
P0/P1/P2 all zero. Ox was unavailable, so this is the recorded narrow review
exception.

No native or physical click acceptance is claimed by this checkpoint. The next
safe action is to finish and independently review the `人生オーバー` production
owner policy, then advance the prerelease and run the exact native and hardware
gates.

## 25. 2026-08-27 production title selector runtime checkpoint

This source checkpoint starts from branch `codex/syndocal-v1.2`, exact
`HEAD`/upstream `45709e8154d707354dca7a96ea9ef12c6dc0a130`. It adds the
runtime and persisted schema for the production selector
`titleContains = 人生オーバー` with explicit `fallbackDeck = 1`. The selector
uses trim plus NFC and a case-sensitive substring match; it is mutually
exclusive with exact `contentId` or `title + artist` selectors. Exact and
substring matches are collected rather than accepted by authored order, so an
overlap fails closed as `track_mapping_ambiguous` without changing engine or DJ
runtime state. Only a payload already selected as Deck 1 may use the fallback;
Deck 2 cannot.

The old app-local first-match finder has been removed. The new deterministic
resolver is isolated in `app/src-tauri/src/dj_track_selector.rs`, keeping the
selection policy out of the oversized `main.rs`. The protocol rejects
`fallbackDeck` without `titleContains`, any fallback other than Deck 1, mixed
selector forms, duplicate canonical selectors, and unknown nested selector
fields. A real project JSON round trip preserves `titleContains` and
`fallbackDeck`. Runtime proof covers a positive Remix title on any Deck, a
known nonmatching Deck 1 fallback such as the temporary demo track, no Deck 2
fallback, primary and fallback ambiguity, same-identity newer Sync, and
different-identity Sync rejection with exact engine/runtime immutability.

The bounded zero/multiple-positive Deck choice remains the responsibility of
the external DJ Agent and is not inferred again by Syndocal. The intended
external rule is: one positive selects that Deck; zero positives wait for and
select only a fresh playing Deck 1; multiple positives prefer a fresh playing
Deck 1 and otherwise use the lowest valid positive. The controlled external
config and timer implementation are a separate uncommitted checkpoint at this
point. The KDMX mapping editor also does not yet expose the two new selector
fields, so no current native build can author this mapping through the UI.

With the exact MSVC 14.44 Community linker pinned and first in `where.exe`,
`cargo test -p protocol dj_link_ -- --nocapture` passed **14/14** and
`cargo test -p syndocal dj_link_ -- --nocapture` passed **119**, failed **0**,
and ignored the one intentional live-network test. Both configurations emitted
**0 first-party warnings**. `cargo fmt --all -- --check` and
`git diff --check` passed; the latter emitted only Git LF-to-CRLF notices.
Independent Terra xHigh adversarial review is **GO** after its initial fallback
interpretation was corrected against the user's explicit Deck 1 demo-track
requirement. Ox was unavailable, so this is the recorded narrow review
exception. No native build, deployed executable, final show project, or
physical DJ/pedal result is claimed by this checkpoint.

The operator also superseded the older fixed-eight-pass C-melody material in
this session. Both the `人生オーバー` C-melody hold and the post-Follow
`惑う星` hold must be indefinite one-measure loops with no repeat counter and no
automatic release; the same F13 pedal releases either loop. Existing authored
A-B loops are already indefinite, but current Timeline-control assigns F13 to
`-4 bars`, Follow does not yet rebase DJ runtime authority to the destination,
and the click/Guide exporter plus older completion-flow text still encode eight
measure-98 passes. Those are explicit remaining implementation and clean-break
items, not acceptance evidence.

## 26. 2026-08-27 C-melody indefinite-loop clean-break checkpoint

This source checkpoint starts from branch `codex/syndocal-v1.2`, exact
`HEAD`/upstream `0de98aad4d4398c66acccbfb85cc4f35b1648319`. Its owned
changes are limited to the Life Over / Madow Hoshi click-and-Guide exporter,
its perceptual checker, and the two active completion/handoff documents. The
concurrent protocol, engine, and app Rust Follow-hold changes remain unstaged.

The retired path rendered measure 98 eight times by appending 28 synthetic
beats, then placed an automatic Break cue. The new path preserves the natural
624-beat `人生オーバー` source schedule and declares exactly one runtime loop
`[measure 98, measure 99)` with `repeatMode = indefinite`,
`releaseTrigger = F13`, and `automaticRelease = false`. It emits one Looping
Guide at measure 98 and one post-release Break Guide at measure 99. There is no
repeat counter and no automatic release; the pedal releases the authored
one-measure loop. The manifest clean-break advances from schema v2 to v3 and
removes `loopTotalPasses` and `loopAddedBeats` rather than retaining a legacy
finite-loop representation.

The canonical no-argument exporter wrote
`C:\TEMP\syndocal-show-audio-indefinite-loop`: Life frames **10,536,286**,
Madow frames **12,470,103**, connected frames **23,006,389**, clicks **1,464**,
physical/semantic Guide events **26/26**, outputs **9**. The no-argument
perceptual checker passed all **12/12** deterministic PCM/hash/non-overlap/
no-clipping/semantic gates. Canonical manifest and click-schedule hashes are
`c763d7776a77247d5da6809132fa781c7fba23133b4d140b54d381cb81e288a9`
and `0f6d83cbf869a45998980f6db8afba070a781acf09d334b2e2d5627fa27551c1`.
The physical manifest/exporter/checker SHA-256 values are respectively
`F5F940F55532C75905C8E738698E4D92DA7B65FC98519DA619B1262E8DE800EC`,
`8BAD5DEE68A762697560011C1AEAD68EC72D89A6A8E5A875BD83991740E46E01`,
and `5ADBB19B2771FC83F6BBAFF2DA7D2052A739630C2D161F6D3F026F87F07CDE09`.
`node --check` for both modified scripts and `git diff --check` passed; the
latter emitted only Git LF-to-CRLF notices. First-party warnings are **0**.
Independent Terra xHigh adversarial review is **GO**, including its own
no-argument checker execution. Ox was unavailable, so this is the recorded
narrow review exception.

This checkpoint proves only the generated show-audio and documentation
clean-break. It does not prove that the running native engine holds or releases
the C-melody loop, that Follow settles into the destination hold, or that the
physical F13 pedal releases either hold. The next safe action is to finish the
separate engine/runtime Follow-hold and F13 routing tranches, advance the
prerelease, run the exact native build/launch/maximized-window gate, and then
perform both physical pedal acceptances.

## 27. 2026-08-27 production title-selector editor checkpoint

This source checkpoint starts from branch `codex/syndocal-v1.2`, exact
`HEAD`/upstream `129d08d76141e7e69363a454d429bc3acca6b50e`. Its owned
files are the DJ mapping policy/checker, the narrow App and Remote panel hunks,
localization, the package checker entry, and these active QA documents. The
concurrent Rust protocol/engine/app Follow-hold files remain unstaged.

The previous editor knew only exact Content ID and exact Title + Artist and its
App normalizer discarded the already-supported production fields. The new
default draft is `titleContains = 人生オーバー` with explicit Deck 1 fallback;
`Use Current Track` deliberately remains exact. The new 191-line pure policy
module keeps validation out of the oversized App, applies trim plus NFC without
case folding, nulls unused selector fields, and rejects mixed forms, invalid
fallbacks, duplicate normalized selectors/trimmed IDs, more than 128 mappings,
invalid Timeline IDs, control characters, and values over 256 UTF-8 bytes.
Rejected add/edit attempts keep the local draft and show the exact local error.

`pnpm --dir app run check:dj-link`, `check:localization`, TypeScript, frontend
production build, and `check-viewport-containment --setup-io-only` passed. The
viewport gate passed **5/5**, including `1280x720`; localization is
**3564/3564** with **0** unprotected user-data labels. The App chunk is
**498.92 kB** (gzip **152.36 kB**) without a Vite warning. `git diff --check`
passed with only LF-to-CRLF Git notices and first-party warnings are **0**.
Independent Terra xHigh review is **GO** after identifying and verifying the
draft-retention P1 fix. Ox was unavailable, so this is the recorded narrow
review exception. The `baseline-ui` constraints influenced this tranche by
retaining native labeled controls, the existing checkbox class, existing
sizes, and the disclosure-owned scrollport; no animation or new CSS was added.

No native build or hardware acceptance is claimed. Read-only live verification
found the prior exact-checkout alpha.18 PID `80264` still listening on
`192.168.50.1:9100` with an established `.50.2` peer; it and Rekordbox were not
stopped. The next safe action is to finish, review, and commit the concurrent
Follow-hold/F13 source work, advance to alpha.19, then replace that process only
at the explicit native-build boundary and execute the physical matrix.

## 28. 2026-08-27 alpha.19 Follow-hold / Stage 2 source handoff

Source is committed/pushed `1.2.0-alpha.19` at
`41faefc054a3c37cef81cfd2e69b4e3f3df5ab4f`. With hold enabled on non-Cut Follow, source
settlement is one admission meter-aware bar; only successful settlement installs
the destination's first meter-aware measure as an indefinite runtime-only loop.
There is no counter or automatic release. `人生オーバー` C-melody and post-Follow
holds both release via F13 loop-off; F14 is the absolute-loop toggle and F15 is
the sole `+4` jump. `-4` and Stage 2 MIDI fail closed. The required outbound
state boolean `transitionHoldActive` diagnoses the post-Follow hold but is not
the F13 gate. F13 requires exact running timeline/play-session/pedal-owner/Release
correlation plus authoritative `loopActive:true`; completed Follow rebase is
additionally required only for post-Follow destination authority, not for the
ordinary C-melody loop.

DJ authority may rebase only on the same released play session, exact Release
receipt/pedal owner, and exact completed Follow source/target pair. Abort, fault,
stale generation, or target/session/receipt mismatch never rebases. Any-deck
exact mapping remains primary; only fresh Deck 1 resolves zero positives and
multiple positives prefer Deck 1 then the lowest valid deck. The strict title
wire retains its actual `artist` field whenever it uses title identity.

Exact MSVC 14.44 serial locked-workspace results are `2622` pass / `0` fail /
`15` intentional hardware-media ignores; Syndocal DJ Link is `119/0/1`, warnings
are `0`, and frontend/Mapping/Stage/Follow/fmt/diff checks pass. Independent
Terra xHigh review is GO with P0/P1/P2 zero. The clean non-OneDrive release gate
passed at exact `41faefc`. Peer v1.1.9 is committed/pushed at
`b03d66a87b8d9dcdedfbd9b5c395bda7df7e0eec`, full suite `453/0/2`. PID `80264`
is the prior alpha.18 artifact; the next safe action is the exact-path process
termination followed by the required alpha.19 native build/launch gate. HW-4
remains **0/12**.

## 29. 2026-08-27 alpha.20 Stage and Setup I/O operator-layout native checkpoint

This checkpoint began on `codex/syndocal-v1.2` at
`7cda8c8fbf04bde6efc705ef59a65dd5816da551`. Product metadata is
`1.2.0-alpha.20`; exact commit
`03b70cd14a285a41c63cfd1d9b3bd89c025eec16` is the authoritative
Stage/Setup-I/O source checkpoint. The implementation diff is frozen after
independent review. This is not a deployed or physical-hardware acceptance
claim.

The old Stage mapping surface mixed independent X/Z conversion with a
square-only interaction frame while the rectangular viewport, glyph scale, and
nested minor/major grid used different rules. At zoom this made segmented
bars/strobes appear to change relative size, shifted visual grid phases, and
could clamp a drag to the old square. The new contract uses one physical world
scale for fixture position, footprint/segment pitch, grid, snap, and pointer
conversion; its viewport is rectangular and keeps both grid layers on the same
origin. The old absolute selections drawer overlaid the Stage and hid fixtures.
It is replaced by normal workspace flow, retaining selection access without
obscuring the mapped area. Fixture transforms now require an authoritative
post-write confirmation; batch movement is serialized and a rejected or
partially rejected batch reports failure without a false success message.

The old Stage limits editor used a tall raw-value form that separated the
movement map from its inputs. The new compact, local two-column Dimmer /
Movement presentation keeps exact raw values and accessibility while adding
human-readable percentage/degrees and a compact square movement map. It does
not globally shrink controls or typography.

The old Setup I/O split connection settings into fixed quadrants, leaving empty
space below Control Mapping while an open DMX connection had too little room.
The new operator surface uses five connection cards (DMX, MIDI, OSC, Web
Remote, DJ Link) and one full-width workbench with its own body scroll. DMX
Input and Merge live with DMX output; advanced Web security/endpoints stay
separate from DJ Link authority and token controls. Inactive Web/DJ workbenches
are not mounted, so neither path duplicates listener, authority, or token
handling.

Focused proof is complete. `check:mapping-stage-geometry`,
`check:mapping-live-segments`, `check:bar-beams`,
`check:control-stage-persistence-failure`, `check:control-stage-fixture-edit`,
`check:mapping-live-snapshot`, and `check:stage-labels` passed.
`check:mapping-viewport-conformance` passed `5/5`; the strongpoint proof kept
the physical cell pitch at `18.1818` for both `4x1` and `4x10`; the Control
fixture edit browser gate proved position move/Undo, yaw `30 -> 90`, a single
`22 x 22` yaw target, and zero yaw handles after clear-pick. Workspace operator
passed `28`; project storage passed; frontend invokes are `422` exact;
localization is `3582/3582` with zero unprotected strings. The Tauri wrapper
checker passed `231` assertions including `27` hostile cases. Workspace split,
Setup I/O, and remote disclosure scroll each passed all five viewport cases.
The production build completed without a Vite warning; the final App chunk is
`499.67 kB`. The `frontend-typescript-vite-windows` warning ratchet is
baseline/current `0/0`, with zero first-party and zero third-party warnings.
`git diff --check` passed with only Git line-ending notices.

After the source commit, the broad `check:dj-link` gate exposed a QA-only
contract drift: its static assertion still required Web Remote `<details>` to
be the direct child of `ioDisclosureStack`, while alpha.20 intentionally wraps
Web and DJ disclosures in mutually exclusive Solid `<Show>` boundaries so the
inactive authority surface is not mounted. The checker now requires both exact
lazy wrappers (`surface !== "dj"` for Web and `surface !== "web"` for DJ), rather
than accepting the retired direct-child layout. The live browser helper also
requires Web selection to mount Web/Security/Endpoints/Standby with no DJ Link,
and DJ selection to mount DJ Link with no Web disclosures; restored Web must
again have no DJ Link. `node --check`, `pnpm --dir app run check:dj-link`, and
all five `check:viewport -- --setup-io-only` cases pass; the DJ gate reports
both mapping policy and frontend contract passed, and every viewport reports
`remoteScroll=1`. This QA/docs follow-up does not change the alpha.20
product/native source commit or close any HW-4 row.

The first native attempt failed before product linking because `FFMPEG_DIR` was
not set; no artifact from that attempt is accepted. The full gate restarted
from exact-path process inspection, `vcvars64.bat -vcvars_ver=14.44`, the pinned
Community linker
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`,
and exact `where.exe link.exe` first. With the pinned FFmpeg 8.1.2 shared root
and `C:\Program Files\LLVM\bin`,
`pnpm --dir app tauri build --no-bundle` completed in `2m11s` with zero
first-party warning lines. The exact artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`, `58,777,088`
bytes, Product/FileVersion `1.2.0-alpha.20`, SHA-256
`E8100D160158034A63901EA1BF775EC06A48EFF4D97CAF454D0378C0D6988D7D`,
LastWriteTimeUtc `2026-08-27T14:02:29.3834470Z`. Launch observed exactly one
responsive process, PID `18456`, title `Syndocal`, handle `19597984`. Computer
Use confirmed a maximized `1920x1080` window by the native
`元のサイズに戻す` title-bar action. Native visual QA confirmed the five-card
connection deck, one full-width DMX workbench with internal scroll, rectangular
Stage, and non-overlay context pane. Switching Web/DJ cards through native UI
automation was not accepted because the accessibility element cache rejected
the action; the five-viewport browser gate covers that behavior but is not
promoted to native proof.

Independent Terra xHigh final reviews are **GO** with P0/P1/P2 zero after the
Control yaw browser assertion was strengthened. Ox callable capacity was
unavailable, so the narrow recorded exception applies. The product-version
surface is synchronized and the Show-ASIO metadata self-test passed `38`, but
`pnpm --dir app run check:release` remains fail-closed before metadata
validation because `qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json` has the external
hard-link alias `C:\Users\kouty\Documents\.tmp.driveupload\867492`. Neither
link was deleted or replaced. This blocks a public release/installer claim, not
the accepted local source/native checkpoint.

Checkpoint cleanup inventory was read-only. Logical file-length totals are
repo `278,371,465,649` bytes (`259.25 GiB`), `.git` `217,408,416` bytes,
`target` `277,042,280,989` bytes (`258.02 GiB`), `app/node_modules`
`545,338,492` bytes, and `app/dist` `5,108,607` bytes. `target/debug` accounts
for `242.23 GiB`, including `115.02 GiB` incremental and `112.06 GiB` deps.
No deletion was performed. After `03b70cd` was clean, pushed, and
upstream-equal, read-only command
`& .\qa\harnesses\invoke-syndocal-build-cache-cleanup.ps1` returned `Mode=Plan`,
`Outcome=Blocked`, `Blocker=WriterOwnershipTopologyUnverifiable`: Adobe Creative
Cloud Libraries `node.exe` PID `61616` referred to a missing positive parent PID
`49864`, so ownership could not be proved. The exact allowlist remained only
`target/debug/incremental`; it contained `72,840` files / `123,505,799,091`
logical bytes (`115.02 GiB`), was only `0.122` days old versus the seven-day
minimum, and separately reproduced `HardlinkDetected` with link count `5`.
`tools/asio-bridge/target` remains outside the reviewed allowlist, and the legacy
harness remains plan-only. Preserve release, QA, ASIO, Show-ASIO, and debug
dependency artifacts until writer topology, age, hard-link, and exact-path
review gates all pass.

Real fixture placement against the supplied Daslight project, DMX output and
input/merge, MIDI, Rekordbox, DJ Link, pedal HW-4 (`0/12`), target-PC deployment,
three-display operation, and the remaining ASIO matrix are still unverified.
Those are the next show-critical boundaries.

## 30. 2026-08-27 Deck 1 fallback contract adjudication

The clean starting point was branch `codex/syndocal-v1.2`, exact
`HEAD`/upstream `45386a49eca0f9e53a4a11b17aaeba5140b68776`. The responsive
alpha.20 release process at exact checkout path remained running; this
documentation-only adjudication did not stop or replace it.

An adversarial audit correctly found that Syndocal's explicit
`fallbackDeck = 1` accepts a known nonmatching playing Deck 1. That behavior is
not an accidental implicit resolver fallback: it is the operator-requested
zero-positive rule already implemented by the controlled rb-output `1.1.9`
peer at exact clean/upstream-equal commit
`b03d66a87b8d9dcdedfbd9b5c395bda7df7e0eec`. Its strict production selector is
`titleContains = 人生オーバー`, NFC/case-sensitive, with
`deck1MetadataWaitMs = 1400`. One positive selects that Deck; zero positives may
select only a fresh actually-playing Deck 1 after the wait; multiple positives
prefer fresh playing Deck 1 and otherwise the lowest valid positive Deck.
Rekordbox Master state is not an admission input.

The rejected alternative was to silently narrow the KDMX fallback to
content-ID-only input. That would have contradicted the current peer wire and
the user's explicit zero/multiple-positive Deck 1 fallback requirement. No
runtime source was changed. Instead, HW-4.2 now keeps the true negative cases
(pre-load, preview, Cue, stopped/non-playing, ambiguous mapping, and
nonmatching identity outside the explicit Deck 1 fallback), while HW-4.3 owns
the exact zero/multiple-positive fallback demonstration. Deck 2 never gains
fallback authority. This resolves the documentation contradiction without
claiming physical acceptance.

Current-head software revalidation used exact `vcvars64.bat
-vcvars_ver=14.44`, pinned
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`,
and verified that linker first in `where.exe link.exe`. Engine DJ-Link focused
tests passed `28/28`; Syndocal DJ-Link focused tests passed `119`, failed `0`,
ignored `1` live-network test; first-party warnings were `0`. The first two
shell attempts never started Cargo and are not accepted evidence. HW-4 remains
exactly `0/12`; real token, wired peer, Rekordbox decks, pedal/MIDI,
response/no-response, reconnect/restart, and shared Art-Net/sACN observations
remain required.

## 31. 2026-08-28 alpha.21 Setup I/O operator checkpoint

The active product/file version is `1.2.0-alpha.21` on branch
`codex/syndocal-v1.2`. The I/O/native source checkpoint is committed and
pushed at exact `536742db968b242164349c34dd6940fe3ced8e92`
(`feat(setup): streamline the I/O operator workbench`). The separate token-free
show structural preflight is committed and pushed at exact
`b35d3ba351b29caaedfb7d0c6f4e29fc84580e83`. Tool/helper `node --check` and
`node qa/tests/show-structural-preflight.mjs` passed; the valid fixture exits
`0`, while legacy, missing, usage, and unknown inputs exit `2`. Independent
Terra xHigh final review is GO with P0/P1/P2 all `0`.

Setup I/O now presents five selectors (DMX, MIDI, OSC, Web Remote, DJ Link)
and one full-width workbench whose body is the single scroll owner. Its
selector row uses `role=tablist`/`role=tab`, current-selection semantics, and an
exactly labelled `tabpanel`; the active workbench keeps only a compact useful
label/state rather than redundant selected-workbench summary text. Service and
quick actions remain separate sibling controls with independent hit targets and
do not change inactive selection. The dedicated browser contract proves real
pointer hit-testing (including the
fifth DJ tab), real keyboard selection (Enter/Space, ArrowLeft/Right,
ArrowUp/Down, Home/End), exact tab/tabpanel relation, strict overflow
(`scrollHeight > clientHeight`), and no competing scrollports among DMX/MIDI/
OSC/Web/DJ descendants. The
remote disclosure contract opens the Web disclosures, scrolls to the bottom
`remote-standby` summary/control, and requires both visible and center
hit-testable. Web/DJ behavior and bottom-scroll evidence are browser-contract
evidence only, not native visual proof.

Focused gates passed at all five supported viewports (`1920x1080`,
`1920x1032`, `2048x1152`, `1366x768`, `1280x720`): Setup I/O selector/
workbench and remote disclosure scroll contracts. `check:localization` passed
`3577/3577 (100%)` with `0` unprotected labels. Frontend `tsc --noEmit` plus
Vite build passed (`294` modules) without build warnings; JS syntax checks and
`git diff --check` passed. Independent Terra xHigh final review is GO with
P0/P1/P2 all `0`.

The native gate initialized `vcvars64.bat -vcvars_ver=14.44`, pinned
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`
in `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and verified that exact path
first in `where.exe link.exe`. `pnpm --dir app tauri build --no-bundle`
produced `target/release/syndocal.exe`, Product/FileVersion
`1.2.0-alpha.21`, `58,778,112` bytes, SHA-256
`F73F1AD8F9E8229B8ACB713AE0C10C36E199C878D869A2E98B554190FC908FA3`.
Exactly one responsive maximized exact-checkout Syndocal window was observed
(PID `41912`). Native visual proof is limited to DMX/MIDI/OSC selector and
workbench paths; Web/DJ five-way and bottom-scroll proof comes only from the
browser contract and must not be promoted to native evidence.

The token-free show structural preflight is read-only: it consumes no real
token, launches no show process, changes no runtime/project/hardware state,
and cannot advance HW-4. Its strict boundary excludes production `.sdc` and
media, target-DJ-PC checkout/config/token/NIC, LAN/HELLO/ACK, Rekordbox,
pedal/MIDI, reconnect/restart, three-display, and physical output proof.

HW-4 remains exactly `0/12`. Real production `.sdc`/media, target-DJ-PC,
LAN, and reconnect evidence remain unverified. Cleanup remains Plan-blocked
after the latest post-`536742d` rerun: `Mode=Plan`, `Outcome=Blocked`,
`Blocker=DirtyWorktree`, `Candidates=[]`, `PlannedLogicalBytes=0`, and
`ReclaimedLogicalBytes=0`; no deletion was performed. The exact 12-path
hardlink remediation was content-preserving with no content diff. The earlier
`WriterOwnershipTopologyUnverifiable` result is historical for this checkpoint.

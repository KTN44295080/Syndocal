# Syndocal pause / resume authority — 2026-08-26

Status: **ALPHA.16 DEVELOPMENT / ALPHA.15 NATIVE ARTIFACT + DSF SOFTWARE CHECKPOINT; PHYSICAL SHOW GATES OPEN**

This is the current resumption contract for the 2026-08-30 performance. The
show-completion deadline is **2026-08-29**, not the performance day. The code is
not accepted as fully show-ready: the alpha.15 standard native artifact and the
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

The current synchronized development metadata and any-deck source checkpoint
are `1.2.0-alpha.16` at `ad3faa253c155cb1173b17fb81a711f60baea1f1`, with focused
software gates, warning 0, and independent adversarial review passed. It has no
native artifact; the latest local native verification artifact remains
`1.2.0-alpha.15`, bound to pushed, upstream-equal source
commit `c609b61`,
while the separately licensed alpha.14 Show-ASIO artifact remains the only
Show-ASIO artifact recorded here. Its runtime baseline includes the DVC
controller checkpoint `652b197d3cce9cfc119a790baffefbd47f08cc8c`. The alpha.15
artifact identity, warning ratchet, and native process gate are recorded below;
they do not promote physical hardware or output acceptance. This same-version
rebuild is local validation evidence only; advance the prerelease ordinal before
distributing another development artifact.

Alpha.16 release-metadata evidence is split by filesystem boundary. The active
OneDrive checkout correctly rejected direct `check:release` because
`qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json` has an external hard-link alias. A
detached clean validation worktree overlaid with the exact alpha.16 source and
pinned runtime bytes passed `check:release`, including the 169-assertion Windows
packaging self-test and exact `1.2.0-alpha.16` metadata. Its Git worktree
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
| Current alpha.16 any-deck DJ-Link source + synchronized metadata | `ad3faa253c155cb1173b17fb81a711f60baea1f1`; no alpha.16 native artifact or physical acceptance yet |
| Current local alpha.15 native verification runtime/source/artifact | Pushed, upstream-equal source commit `c609b61c77e44ee028ed7322c29a0cfd04b8182c` (`refactor(scene): extract bank scene creation controller`); local artifact identity is in section 3 and is not a new distributable artifact |
| Prior alpha.15 standard runtime/source/artifact | Pushed, upstream-equal source commit `1d372e795870c1a6e5687d1116161042ddac627e` (`fix(project): preserve inline authority continuation`); prior standard artifact identity is retained in section 3 |
| Historical alpha.14 runtime/code and standard artifact | `92122f1b148d40845b2cfe3e4618a57ce132b3df` |
| Alpha.14 local-only Show-ASIO artifact | `6b4cd1afb4d228158d04a15dbe3e4a73c922baeb` |
| DVC import controller extraction and focused software proof | `652b197d3cce9cfc119a790baffefbd47f08cc8c` |
| Current rb-output v1.1.6 strict-v3 runtime / docs tip | `ee2f6c3148f36dfd63e0b70e2ab372247dbb8572` / `789f7724a699324cd87171ef835b69486bcd4e70`; pushed, target DJ PC not yet updated |
| Timeline menu/localization alpha.13 checkpoint | `bbb684cee4c8b01cfc019575569bd26835dbc732` |
| Historical alpha.12 standard and Show-ASIO artifacts | `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae` |
| Display stable-identity QA | `b543067b0cbde4015ee632a8c6e6ccd77e6bfd9f` |
| Show-ASIO Cargo hardlink fix | `fb25ab106e9994fb6215520795961f4909bea7ae` and `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae` |
| Cleanup inaccessible-process fix | `ef7b6479f69e89dd134acfe39051f43c22f769aa` |
| Cleanup exact Codex-control-plane gate | `c40cfd89ccb2203b92e76d3a8d72f00980aa1a30` |
| Cleanup directional ownership repair and final Plan | `0b8a992f9389e39fc07a53e9fb74b7fa1f20368b` (pushed; supersedes the prior `c40cfd8` cleanup authority) |
| This final documentation publication | The pushed commit containing this file; resolve with `git log -1 --format=%H -- qa/SYNDOCAL_PAUSE_HANDOFF_2026-08-26.md`. A self-hash is intentionally not embedded. |

Branch: `codex/syndocal-v1.2`. The current local alpha.15 native verification
runtime/source/artifact is bound to pushed commit
`c609b61c77e44ee028ed7322c29a0cfd04b8182c`. The prior alpha.15 standard
runtime/source/artifact remains bound to
`1d372e795870c1a6e5687d1116161042ddac627e`. The historical alpha.14 runtime
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

## 3. Alpha.15 standard artifact; historical alpha.14 and local-only Show-ASIO artifacts

### Current local alpha.15 MIT/WASAPI verification artifact (2026-08-26)

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

### Prior alpha.15 standard MIT/WASAPI artifact

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

## 4. Completed software evidence

### Alpha.15 native/software checkpoint (2026-08-26)

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
- Current v1.1.6 runtime source: `ee2f6c3148f36dfd63e0b70e2ab372247dbb8572`
- HEAD/upstream docs tip: `789f7724a699324cd87171ef835b69486bcd4e70`, clean after the 2026-08-27 push
- Live target-DJ-PC evidence is still the historical v1.1.5 session; v1.1.6 has
  not yet been deployed or physically accepted there.
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

### 2026-08-26 alpha.15 live listener activation

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
- Current alpha.16 source checkpoint `ad3faa2...`, local alpha.15
  source/artifact checkpoint `c609b61...`, prior standard
  alpha.15 source/artifact checkpoint `1d372e7...`, DVC controller
  checkpoint `652b197...`, alpha.14 artifact source checkpoint `92122f1b...`,
  and rb-output v1.1.6 docs tip `789f772...` are the current checkpoint chain.
  The rb-output tip was clean and upstream-equal when
  recorded. The final documentation
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

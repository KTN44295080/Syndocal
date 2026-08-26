# Syndocal pause / resume authority — 2026-08-26

Status: **PAUSED AFTER THE CURRENT CLEANUP AND DOCUMENTATION TRANCHE**

This is the current resumption contract for the 2026-08-30 performance. The
show-completion deadline is **2026-08-29**, not the performance day. The code is
not accepted as fully show-ready: software gates and alpha.12 artifacts exist,
but the physical DJ, native ASIO, three-output, and DSF show-program gates below
remain open.

## 1. Exact checkpoints

The checkpoints are deliberately separate. Do not relabel a later QA/docs
commit as the source identity of an already-built binary.

| Layer | Exact authority |
| --- | --- |
| Runtime/code and both alpha.12 artifacts | `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae` |
| Display stable-identity QA | `b543067b0cbde4015ee632a8c6e6ccd77e6bfd9f` |
| Show-ASIO Cargo hardlink fix | `fb25ab106e9994fb6215520795961f4909bea7ae` and `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae` |
| Cleanup inaccessible-process fix | `ef7b6479f69e89dd134acfe39051f43c22f769aa` |
| Cleanup exact Codex-control-plane gate | `c40cfd89ccb2203b92e76d3a8d72f00980aa1a30` |
| This final documentation publication | The pushed commit containing this file; resolve with `git log -1 --format=%H -- qa/SYNDOCAL_PAUSE_HANDOFF_2026-08-26.md`. A self-hash is intentionally not embedded. |

Branch at cleanup checkpoint: `codex/syndocal-v1.2`; HEAD and upstream were
both `c40cfd89ccb2203b92e76d3a8d72f00980aa1a30` before the final documentation
commit.

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

## 3. Current alpha.12 artifacts

### Standard MIT/WASAPI artifact

- Source checkpoint: `ff61a6d`
- Path: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
- Product version: `1.2.0-alpha.12`
- Size: `58,471,936` bytes
- SHA-256: `224F512673C8A84EAEB2557691414B2F6CA090D1201E357DCD9B38F019237680`
- Pause-time process: PID `158832`, exact-path process count `1`, responsive
  window count `1`, title `Syndocal`, maximized. This was reverified read-only
  on 2026-08-26 from the exact executable path, process identity, window state,
  version, byte count, and file hash.

### Local-only Show-ASIO artifact

- Source checkpoint: `ff61a6d`
- Directory:
  `C:\Users\kouty\Documents\KDMX\target\show-asio-local\Syndocal_Show_ASIO_1.2.0-alpha.12_ff61a6dec6eb_x64`
- Application: `syndocal-show-asio.exe`, `58,637,824` bytes, SHA-256
  `1D313900AB94A2429BF784B7D4CCA8E8EC39FBF17E11CB257D76A19656AA2F8D`
- Bridge: `syndocal_asio_bridge.dll`, `813,568` bytes, SHA-256
  `40BB8D19C7B5C8DFA52C21C879C8887645CDE83DF6A4FAB5CF59D2A396546AE2`
- Manifest: `show-asio-local-manifest.json`, `8,660` bytes, SHA-256
  `DCDFA0D381C851483D9E206637E803920ADDD6CC5B0C313780604FFC1D0EAAC4`
- Manifest payload files: `14`; filesystem files including the manifest: `15`.
- `distributionApproved:false`; same-host, unbundled, separately licensed,
  local show artifact only. It is not a public installer/updater artifact.
- It was built, manifest-checked, launched, and verified as one responsive
  maximized window, then its exact PID was stopped. Physical ASIO operator
  acceptance remains open.

## 4. Completed software evidence

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

## 5. Cleanup final result

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
- HEAD/upstream: `925880068500d42d71b2671fa8a99e5895aca4e2`, clean at pause audit
- FOH/Syndocal host: `.50.1`; DJ PC: `.50.2`
- The controlled source path requires the external show JSON through
  `DJ_AGENT_CONFIG_PATH`; a preflight-only pass is not an active Agent session.
- Physical acceptance remains `0/12`: real token, HELLO/ACK, Rekordbox track
  detection, MIDI output, pedal, 8/4/2-beat loop, release, reconnect, and restart
  are not recorded as accepted.

## 8. Explicit remaining show gates

These are OPEN, not implicit completion claims:

1. Run the full DJ-Link 12-row hardware table on `.50.1/.50.2` with the real
   show token and retain logs.
2. Run Show-ASIO from its exact local-only artifact: enumerate/select the real
   device; verify persistence/revalidation; negotiate native format, sample
   rate, channels, and buffer; exercise Start/Stop/Close, exclusive contention,
   reset/resync/XRUN/unplug/no-callback/restart, and measure latency.
3. Treat the recorded `44.1 kHz / 3,600,031 ms` run as bridge-only. Matched
   `48 kHz` ASIO/WASAPI and native operator acceptance remain open.
4. Configure and Apply the stable three-output roles, verify exact placement and
   visible content on all three physical displays, then save/restart and repeat.
5. Import `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc`; retain the import report and
   exact skipped/fail-closed mappings. Create representative Lighting scenes,
   LED/projector substitute media, the two required timelines, DJ trigger and
   loop/release/transition behavior, then save/restart and run the full show
   sequence. User-authored final lighting remains outside this minimum proof.
6. Complete native/manual QA for recent UI fixes, including Timeline compact
   Bank/Scene placement, Lighting-only Bank management, context menus, empty-bank
   control de-duplication, active-scene color, bank toolbar alignment, scene FX
   internal scroll/removal/density/width, detached Timeline/Stage/source panes,
   and integrated Stage rendering.
7. Decide and prove a hardlink-aware cleanup contract before deleting the
   current incremental cache; otherwise leave it untouched.

## 9. Giant-file debt is not resolved

Current exact metrics:

- `app/src/App.tsx`: `28,810` lines, `1,202,918` bytes, SHA-256
  `4BC665227797FA2F138A802DC9979B93D44881A1E2182C921B1B5BD67E5DACB2`
- `app/src-tauri/src/main.rs`: `128,698` lines, `4,937,271` bytes, SHA-256
  `8D5C30181151B7AB16404A4A8320607A06377524EA334D35A24A5BD7CCBDAE17`

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
- Main checkout had no untracked files at the `c40cfd8` cleanup checkpoint.

Resume in this order:

1. Read this file, `qa/ASIO_INPUT_ACCEPTANCE.md`, and the DJ acceptance table.
2. Recheck main/companion/DJ branch, HEAD, upstream, dirty ownership, stashes,
   exact artifact hashes, and exact running process identity.
3. Prioritize physical acceptance over installer polish or broad refactoring:
   DJ 12-row table, native ASIO, stable three-output Apply, then DSF show flow.
4. Fix only evidence-backed blockers; keep unsupported mappings fail-closed.
5. After each meaningful pass, update authority, report warning counts,
   commit, and push.

This pause does not claim public-release readiness or completed performance
acceptance. It preserves a verified alpha.12 software/artifact checkpoint and
the shortest exact route to the remaining physical show proof.

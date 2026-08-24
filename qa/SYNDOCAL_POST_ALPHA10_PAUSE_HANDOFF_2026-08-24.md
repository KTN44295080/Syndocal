# Syndocal post-alpha.10 pause handoff — 2026-08-24

> Finalization state: the alpha.10 source, focused gates, native build, same-HEAD
> physical evidence, cleanup, and exhaustive residual registry are captured
> below. This file becomes the sole resume authority only after its documentation
> checkpoint is committed/pushed and a final promotion edit records the stopped
> process and upstream state. Until that edit, the 2026-08-23 pause handoff is the
> current record.

## 1. Requested stop boundary

The operator requested one bounded outcome: finish `1.2.0-alpha.10`, make the
operating rules and remaining-work record internally consistent, then stop.
No alpha.11 implementation, D4 integration, ASIO integration, DJ Link hardware
acceptance, or cleanup-harness completion belongs before this stop.

The stop is valid only after all of the following are true:

1. the corrected Timeline / Stage / Sources pane lifecycle has an independent
   Ox adversarial review and all focused gates are green;
2. `pnpm --dir app tauri build --no-bundle` succeeds with the exact Visual
   Studio `Hostx64\x64\link.exe` pinned, after proactively stopping only this
   checkout's exact old release executable;
3. the new executable launches as exactly one responsive `Syndocal` main
   window and the intended window is maximized before UI operations;
4. native maximized/restored/F11 and physical `3840x2160 @ 150%` checks cover
   both detach orders, both titlebar-X closes, both rejoin orders, expanded
   Timeline preservation, no duplicate pane window, and no blank main region;
5. product metadata, first-party warning counts, evidence, rules, remaining
   work, and prohibited claims are recorded;
6. the alpha.10 checkpoint and the final pause documentation are committed and
   pushed, with final `HEAD == upstream`;
7. delegated work is stopped and no automatic continuation or alpha.11 work is
   left running.

## 2. Alpha.10 acceptance record

Accepted source checkpoint: branch `codex/syndocal-v1.2`, commit
`5c7e19a72a97e20f5ece553594841103990a78a9` (`fix: finalize alpha.10 pane
window lifecycle`), pushed to `origin/codex/syndocal-v1.2`. The product remains
`1.2.0-alpha.10`; this checkpoint does not change the 19/71 whole-product
denominator.

Focused gates were rerun against that pushed source. Every command below exited
0:

- `pnpm --dir app run check:backend-operator-contract`: 480 commands, 310
  literal frontend calls, 133 transactional mutations;
- `pnpm --dir app run check:frontend-command-routing`: 133 renderer mutations,
  29 server-authoritative mutations, 30 raw dispatches, 407 facade dispatches;
- `pnpm --dir app run check:workspace-operator`: 28 assertions;
- `pnpm --dir app run check:frontend-invokes`: 419 commands;
- `pnpm --dir app run check:tauri-build-wrapper`: 26 assertions;
- `pnpm --dir app run check:release`: synchronized `1.2.0-alpha.10` metadata;
- `pnpm --dir app run check:release:self-test`: 102 assertion groups;
- `pnpm --dir app run check:warnings:self-test`: green, including the Cargo
  output-marker false-positive cases introduced in alpha.10;
- `pnpm --dir app run check:pane-window-lifecycle`: all fixtures/source gates;
- `pnpm --dir app run check:patch-viewport`, `check:pane-reflow`, and
  `check:workspace-split`: each passed 1920x1080, measured-client 1920x1032,
  2048x1152, 1366x768, and 1280x720. The pane-reflow gate measured the
  Setup/Patch reserved band at 0 and the remaining surface at 968/920/1040/
  656/612 px respectively when Stage was detached;
- `pnpm --dir app build`: TypeScript plus Vite exit 0, 272 modules transformed,
  and no Vite/chunk warning.

The first post-cleanup viewport attempt exposed missing active-checkout pnpm
links before Vite started (`@babel/core`, then `@babel/types`); no product
assertion had run. `pnpm --dir app install --force --frozen-lockfile` restored
all 142 packages from the frozen lock/store without a source or lockfile edit.
All three five-viewport gates and the 272-module production build above were
then rerun green. The cleanup rule in `AGENTS.md` now requires this dependency
integrity/rebuild check after deleting a potentially linked worktree or cache.

Immediately before the final native rebuild, the only `syndocal.exe` candidate
resolved exactly to this checkout's `target\release\syndocal.exe`: PID 105812.
Only that PID was force-stopped and the follow-up exact-path count was 0. The
successful gate entered the Visual Studio 2022 x64 Developer Shell with
`-vcvars_ver=14.44` (`VSCMD_VER=17.14.29`). The guarded Tauri wrapper resolved
and internally pinned
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`
(file version 14.44.35225.0); Cargo never selected Git for Windows' `link.exe`.
An earlier attempt to inject `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`
outside the warning ratchet was intentionally rejected before compilation as a
warning-affecting environment. The accepted route was
`pnpm --dir app run check:warnings -- --configuration windows-native-release`,
whose recorded command is the required
`pnpm --dir app tauri build --no-bundle`. It exited 0 in 156.930 seconds with
output-marker coverage 2/2 and baseline/current warnings both
total 0, first-party 0, third-party 0.

Final rebuilt executable identity:

- path: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`;
- size: 57,608,192 bytes;
- SHA-256: `2262CAE19253D2CB4A5F390C9A2831A36DB128286275CBEBD75B49E89D834693`;
- ProductVersion/FileVersion: `1.2.0-alpha.10` / `1.2.0-alpha.10`;
- last write UTC: `2026-08-24T17:30:38.0889782Z`.

That exact executable launched as PID 107500. The fail-closed geometry observer
found one matched exact process and exactly one responsive titled main window,
HWND 731516404, maximized on DISPLAY2 at client 1920x1032. Persisted detached
children were unique and responsive: Stage HWND 148513242 and Timeline HWND
717164626; each was maximized and measured at client 1920x1009 before the pause.
Live final-artifact inspection proved the both-detached main contains the Sources
surface at full height, with no reserved blank band, and the Edit domain exposes
all three Lighting/Video/Timeline tabs. The Stage child contains the real 2D map;
the Timeline child contains the real two-lane editor. This also resolves the
earlier screenshot ambiguity: the two-tab surface was the internal Touch/Control
workspace, not a clipped Edit Timeline tab.

The full interaction and physical matrix was run on the immediately preceding
native build from the same committed source (`5c7e19a`), whose SHA-256 was
`7E399FB6CE5389B06A32FA7E4BCFCE4B91F9B3D436A73890DDB85481B45F3E89`.
The repeated native link is not byte-reproducible, so its evidence is kept
source-bound and is not relabelled as physical proof of the later binary hash.
It exercised restored 1280x800, maximized 1920x1032, F11 1920x1080, both detach
orders, both reverse rejoin orders, both titlebar-X closes, expand-before-detach,
expand-before-rejoin, and integrated/one-detached/both-detached topology without
duplicate pane windows. CDP measured document/app outer scroll at zero.

Physical DISPLAY3 was an actual 3840x2160 monitor at 150% scaling with a
3840x2088 work area and 2560x1392 CSS viewport. Evidence is under
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha10-final/`. Key hashes:

| Evidence | SHA-256 | Meaning |
|---|---|---|
| `03-integrated-timeline-stage-sources.jpg` | `4312F58B7C7D89DD20C39232F270EA5CDD4FC3E7345D810EAC63BDAC48DA87E9` | real integrated Timeline, Stage/Groups, Sources |
| `04-timeline-detached-main-stage-sources-fill.jpg` | `34D1569CC884AB9338B03D229D3B34273EA93F9028DF6CEF47BAA70C482ADAEB` | Timeline absent from main; Stage/Sources fill |
| `05-stage-detached-main-timeline-sources-fill.jpg` | `159639BF7C2ACDB4C9DA0D049217F760C2D07A3167E5CDD1688A5C7E51996C3E` | Stage absent; Timeline/Sources remain |
| `06-both-detached-main-sources-only-fill.jpg` | `516972EAB08FC5D5951E14802DE40BE9F544334D14750F260975B01250073C79` | Sources-only full main workspace |
| `10-setup-patch-stage-detached-after-reflow.jpg` and `11-setup-patch-both-detached-after-reflow.jpg` | `78F978534657D25B6C33C41F12ABBCDBA77573C8F404B1CD9BCFFFFFEA808A4E` | corrected Setup/Patch release of the lower band |
| `12-physical-4k-display3-integrated-final.jpg` | `35B3F044930CE681B99270147202471B1DEEB781D9A59067B8177E64AE627614` | final same-HEAD physical-4K integrated view |

The required visual contract is:

- integrated Control/LIVE: real upper Timeline, lower-left Stage / Groups, and
  lower-right Sources;
- Timeline detached: the upper Timeline and its splitter are absent; Stage /
  Groups and Sources fill the main window; the child contains the real Timeline;
- Stage detached: the upper Timeline remains; Stage / Groups are absent and
  Sources consume their remaining band; the child contains the real Stage;
- both detached: Sources alone fill the main workspace; Timeline, Stage,
  Groups, and both splitters are absent and not keyboard-focusable;
- Setup / Edit / Mixer remain unchanged except for their explicitly shared pane
  state. Setup/Patch may retain Groups only as an intentional, populated strip
  while the real Stage remains integrated. When Stage is detached, Setup must
  not reserve an inert Stage/Timeline band or horizontal splitter: PATCH or the
  remaining active Setup surface consumes the released height.

The physical-4K pre-fix diagnostic
`09-setup-patch-both-detached-before-reflow.jpg` records the rejected state:
with Timeline and Stage detached, a maximized Setup/Patch main window still
reserved roughly 42% of its height for Groups/Stage-settings and an empty
projection area. It is defect evidence, not acceptance evidence. Alpha.10 was
not accepted until the independently reviewed fix, fresh exact-linker native
build, and separate after-reflow captures 10/11 proved that lower band was gone.

Browser/static checks never substitute for native child creation, titlebar-X,
F11/Esc, responsive-window identity, or physical-4K evidence.

## 3. Final Git, process, and artifact state

At this evidence checkpoint the pushed code HEAD and upstream are both
`5c7e19a72a97e20f5ece553594841103990a78a9` with ahead/behind 0/0. The only
remaining main-checkout changes are the bounded rules, release/QA evidence, and
pause documents being committed from this file; the unfinished cleanup harness
is deliberately excluded. The final promotion edit must replace this paragraph
with the documentation commit, stopped PID 107500 disposition, final
`HEAD == upstream`, and exact residual `git status --short` before this file is
the resume authority.

Preserved stashes:

- `stash@{0}` / `490a4bd8f62e5f5dcbcf20b7ae9b7cf1d7dc4581`: accidental/safety WIP on the
  alpha.9 checkpoint. It was not applied or dropped;
- `stash@{1}` / `804d9bd01cc0595e7572a99b96a408152516834d`: orphaned OpenDMX pacing WIP.

Registered worktrees after cleanup are main, ASIO, and D4 only. The D4 worktree
owns `app/src-tauri/src/main.rs`. The ASIO worktree owns
`app/scripts/check-project-storage-helpers.mjs`, `app/src-tauri/src/main.rs`,
`app/src/App.tsx`, and `app/src/types.ts`. Their exact HEADs and acceptance
boundaries are in section 4; neither diff was modified here. The protected
untracked main-checkout paths after the evidence commit are the unfinished
`tools/cleanup-stale-targets.ps1` and `tools/tests/` only.

No Cargo, rustc, link, lld-link, or mspdbsrv process remained after the native
gate. The observed Node PID 97208 still owns the external rekordbox peer runtime;
its command is `node server/index.js` and its identity must be reverified before
any future stop. It was not changed or stopped here. No delegated implementation
continues past the final promotion.

## 4. Frozen companion work

These lanes were intentionally stopped when the post-alpha.10 stop boundary was
requested. Their diffs are useful work in progress, not accepted mainline work.

### 4.1 D4 Stage transaction backend

- worktree: `C:\Users\kouty\Documents\KDMX-d4-stage-transaction`
- branch / HEAD: `codex/d4-stage-transaction` /
  `389b060dec199e299caf0ff2ce2f86e1b7f050b7`
- dirty owner: `app/src-tauri/src/main.rs` only; no commit or push for Stage 2
- implemented: nine Stage routes use the existing receipt lane; Applied and
  Unchanged results persist before reply; retry/mismatch and one-allocation Add
  intent semantics have focused coverage
- recorded green gates: exact x64 linker `cargo check -p syndocal --locked`
  warning 0; focused D4 tests 4/4; existing reply-loss route test 1/1;
  `git diff --check` exit 0
- prohibited claim: frontend transaction callers, independent Ox review,
  integration, native build, and physical acceptance are incomplete
- first safe resume action: re-verify the exact dirty diff and gates, then run an
  independent Ox adversarial review before any frontend or mainline integration

### 4.2 ASIO project-v2 persistence rehydrate

- worktree: `C:\Users\kouty\Documents\KDMX-asio-persistence`
- branch / HEAD: `codex/asio-persistence-v2` /
  `85de2f02d28a6f8a03b0fd75d2181b608b5eb306`
- dirty owner files: `app/src-tauri/src/main.rs`, `app/src/App.tsx`,
  `app/src/types.ts`, and `app/scripts/check-project-storage-helpers.mjs`; no
  commit or push for this stage
- implemented: exact `driver_id` rehydrate into a fresh stopped-only catalog,
  token/epoch/hash fencing, Ready/Stale fail-closed behavior, WASAPI defer, and
  no automatic open/start
- recorded green gates: TypeScript no-emit, project storage, live-audio,
  frontend and ASIO warning ratchets at 0, exact-linker ASIO all-targets check,
  and nine focused Rust tests
- native boundary: a retry produced a release executable, but the long-running
  tool did not return a final exit code; that executable was not launched or
  accepted, and no real ASIO device QA was run
- first safe resume action: independent Ox review, then rerun the exact native
  gate with an explicit exit result and execute the real-device Ready/Stale
  matrix from `qa/ASIO_INPUT_ACCEPTANCE.md`

## 5. rekordbox-dj-link peer

- repo: `C:\Users\kouty\Desktop\rb-output`, branch `Beta`
- current source HEAD: `616c89792016a1c17c94ebd20e8cf8de3aea5ece`
  (`feat:add-live-Rekordbox-mixer-dashboard`), clean at the latest read-only
  inspection
- the update fast-forwarded after envelope work. It did not edit the dedicated
  `server/dj-agent/syndocalClient.js`, envelope tests, build-identity module,
  package metadata, or environment example, but it did edit the shared
  `server/index.js`, hook provider/state, native hook, dashboard, and smoke tests
- post-update recorded tests: focused envelope 9/9 and full Node suite 69/69
- static interop: one connection-scoped session identity is used for HELLO,
  State Sync/request, heartbeat, and physical frames; KDMX rejects mismatched
  identities and returns Timeline State using the peer HELLO identity
- unresolved peer metadata: package is `1.1.1-alpha.11`, while native hook text is
  `rb-hook-7.2.13-7.2.18-alpha14-mixer-faders`; determine whether these are
  intentionally independent peer version domains before changing either. Neither
  ordinal is a Syndocal alpha.11 version or authority to advance Syndocal
- prohibited claim: the existing `dist\server.exe` predates the current source
  and cannot prove this HEAD. No build-time HEAD/fingerprint binding, current
  executable hash, `/api/health` capture, isolated wired-LAN handshake, real
  rekordbox event, pedal, or concurrent Art-Net/sACN acceptance is complete
- first safe resume action: independent Ox review against this exact HEAD, then
  build an identity-bound artifact and follow
  `qa/REKORDBOX_STREAM_DECK_PEDAL_ACCEPTANCE.md` without recording its token

The latest observed old runtime was a Node process owning TCP 8787 / UDP 22346.
Re-verify its executable/source/config identity before stopping or replacing it;
those ports are not the KDMX `/dj-link` WebSocket endpoint.

## 6. Cleanup and storage

Historical recorded cleanup removed 26 verified-regenerable paths totaling
260,604,124,772 logical bytes (about 242.7 GiB). That is historical evidence,
not a deletion rerun for this pause.

The current untracked `tools/cleanup-stale-targets.ps1` and
`tools/tests/run-cleanup-stale-targets-tests.ps1` are unfinished. Eight focused
test failures were previously observed, including identity-swap/quarantine and
message/recovery cases. Static review also found that recursive staleness is not
yet proven. They must not be called a reviewed harness or executed until tracked,
all focused tests are green, and an independent Ox review accepts the exact
target set.

Final inventory and manual exact-path cleanup:

- 25 of the historical fixed 26 candidates are absent;
- the remaining main `target\debug\incremental` is recently updated and is not
  stale under the seven-day policy;
- ASIO and D4 companion targets are active work products and must be preserved;
- alpha.7 and alpha.8 native evidence directories are protected evidence, not
  cleanup candidates.

After the pushed alpha.10 code checkpoint, exact resolved paths, no-writer state,
cleanliness, reachability, and recovery were checked manually. The unfinished
harness was not executed. These four regenerable targets were deleted:

- `C:\Users\kouty\Documents\KDMX-fable-lane`: 7,935,180,433 bytes. Normal
  `git worktree remove` unregistered it but left generated residue; the residue
  was then removed by exact path. Branch `fable/preset-expansion-3` remains at
  `6430b55f43a59bb0182d85c8a4696b4e02c96f3f`, which is contained in main and
  `origin/codex/syndocal-v1.2`, so the worktree is recreatable;
- `C:\TEMP\opencode\d4-review-nonvacuity`: 3,318,653,250 bytes. It was a clean
  detached scratch clone at `e9209d6` whose origin was the preserved D4 worktree;
- `C:\TEMP\opencode\app-diff.txt`: 34,320 bytes;
- `C:\TEMP\opencode\band-diff.txt`: 26,120 bytes.

All four paths are absent after deletion. Total reclaimed logical bytes are
11,253,894,123 (about 10.48 GiB). The worktree/clone can be regenerated from the
preserved commits; the two diff files were disposable review exports. Current
main release/debug output, ASIO/D4 worktrees, both stashes, rb-output, evidence,
and `oxalpha-backup` were preserved. The active-checkout dependency restoration
and complete rerun are recorded in section 2. No Recycle Bin recovery is claimed.

## 7. Remaining product work

Closing alpha.10 fixes this pane defect but does not advance an unrelated product
denominator. This is the exhaustive operational registry at the stop. It expands
all 51 unchecked rows in `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md`; none may be
silently collapsed into a completion claim. The section/evidence references
below point to the subordinate detailed gate record.

| ID | Remaining independently resumable work | Evidence | Dependency and first safe resume action |
|---|---|---|---|
| P0-1 | Expand every phase into Q1-Q4 requirement, decision, and evidence coverage | Flow §6 Phase 0; master roadmap Q0 | Before implementation, create one traceable Q1 row per existing domain requirement. |
| P0-2 | Extend `check:release` with tag, previous-version, updater, and artifact checks | Flow §6 Phase 0 | Implement as a bounded script tranche after this stop and obtain Ox review. |
| P1-D4 | Stage import/mutation identity fencing, atomicity, Undo, and truthful errors | Flow §6 Phase 1 | Re-audit the frozen D4 diff, obtain Ox review, then integrate frontend/native work. |
| P2-1 | AI3 crash-safe durable terminal recovery | Flow §6 Phase 2 | After D4, replace process-local receipt assumptions with durable journal/terminal design and proof. |
| P2-2 | Native-client/hardware verification for MIDI, OSC, DMX, Remote, and other ingress | Flow §6 Phase 2 | Freeze the canonical fail-closed route inventory, then run real clients and hardware. |
| P2-3 | Physical output retirement and exact re-Arm for New, Load, Recovery, Backup, and Take Over | Flow §6 Phase 2 | Implement acknowledged resource retirement and exact re-Arm for project replacement. |
| P2-4 | Reply-loss/crash, native Yes/No/close, physical create/teardown ACK, and five-display proof | Flow §6 Phase 2 | Complete durable receipts and P2-3, then use one exact native artifact. |
| P3-AI0 | Source inventory and fail-closed unknown mutations | Flow §6 Phase 3 | After P2 exit, inventory every GUI/backend/adapter mutation. |
| P3-AI1 | Query/event schemas, snapshot/generation, gap/resnapshot, and bounds | Flow §6 Phase 3 | Generate from the accepted AI0 registry. |
| P3-AI2 | Authored command bridge for E/R/H, owner incarnation, receipts, and Undo | Flow §6 Phase 3 | Requires AI1 schemas and the durable AI3 receipt boundary. |
| P3-AI4 | Principal, pairing, grant, revocation, kill switch, consent, and bypass-absence proof | Flow §6 Phase 3 | Preserve the AI3 owner lease and local-only v2 contract; never restore Raw Input consent. |
| P3-AI5 | Authenticated sidecar, MCP, JSON-RPC/REST/WS, discovery, and bounded lifecycle | Flow §6 Phase 3 | Follow AI4 policy; the sidecar must never own authority. |
| P3-AI6 | Principal/grant/revocation/audit/health management UI | Flow §6 Phase 3 | Build only after AI4 and AI5 backend contracts. |
| P3-AI7 | Adapter parity, security, rate, reply-loss, fuzz, and saturation proof | Flow §6 Phase 3 | Run adversarial matrices after AI0-AI6. |
| P3-AI8 | Native external client, clean install, hardware, crash/restart, security, and artifact proof | Flow §6 Phase 3 | Requires AI7 and an exact release artifact. |
| P4-F1 | Monotonic generation and stale-callback retirement for MIDI/OSC/DMX workers | Flow §6 Phase 4 | Inventory input lifecycle after P3 exit. |
| P4-F2 | Complete Lighting/Video/Both/Standby output ownership | Flow §6 Phase 4 | Fence DMX, NDI, Spout, Display, native windows, and SDK resources under F1/owner leases. |
| P4-SC | ShowClock transport, discovery, authentication, key rotation, replay, master/slew/Hold, witness, and fencing | Flow §6 Phase 4 | Freeze the distributed-output decision after F2 and before implementation. |
| P5-1 | Reintegrate all accepted Clip Slot, Layer Bus, and FX gates | Flow §6 Phase 5 | Run full current-source regression after P4 exit. |
| P5-2 | C2 Clip Take and C4 mapping/Timeline integration | Flow §6 Phase 5 | Resume after P5-1. |
| P5-3 | Follow/crossfade, BPM slew, failure policy, and Transition/Complete Guide | Flow §6 Phase 5 | Resume after P5-2. |
| P5-4 | Undo/Redo/save/reload selection and focus, fixed Guide routing, native A/V/L sync | Flow §6 Phase 5 | Resume after P5-3. |
| P5-5 | Thumbnail/waveform/proxy/analysis, bounded workers, cache/eviction, and performance | Flow §6 Phase 5 | Fix asset identity and cancellation first. |
| P5-6 | Authored Audio schema/migration/history, clock/resample/seek/loop, and fault policy | Flow §6 Phase 5 | Decide the ShowClock/audio/PTS master clock first. |
| P5-7 | Camera/screen/NDI/Spout/Syphon/generator identity, fault/reconnect, and worker retirement | Flow §6 Phase 5 | Requires output ownership and the live-source model. |
| P5-8 | Recording state machine, reservation, crash/disk/encoder recovery, atomic artifact, and two-PC ownership | Flow §6 Phase 5 | Resume after P5-5, P5-6, and P5-7. |
| P6-SC | ShowClock simulator, authenticated peer sync, exactly-once action, and two-machine fault/soak | Flow §6 Phase 6 | Requires P4-SC and P5 audio/recording ownership. |
| P6-H1 | Operator reachability for every supported feature | Flow §6 Phase 6 | Freeze the P5 feature set first. |
| P6-H2 | Shared-shell truth across Setup, Edit, Control, Touch, and native windows | Flow §6 Phase 6 | Resume after H1. |
| P6-H3 | Complete Setup Patch/GDTF/OFL/mapping/I/O/output/device UI | Flow §6 Phase 6 | Resume after H2. |
| P6-H3-DJ | DJ Link NIC/bind/token/session/Track mapping/recovery/Use Current Track UI | Flow §6 Phase 6 | Requires current peer artifact identity and AI4 policy. |
| P6-H4 | Complete Edit Media/Timeline/Guide/loop/group/FX/Stage/history | Flow §6 Phase 6 | Requires P5 completion. |
| P6-H5 | Complete Control live A/V/L, Take, Blackout, Arm, recording, and diagnostics | Flow §6 Phase 6 | Requires P4/P5 and exact ownership. |
| P6-N | Remote/Touch LAN/TLS, pairing, Origin/Host, RDM, parser/archive fuzz, and SBOM | Flow §6 Phase 6 | Requires AI4/AI5 and RDM hardware. |
| P6-O | `.sdc`/template/cache/protocol version matrix, migration, corrupt/fuzz, and recovery | Flow §6 Phase 6 | Stabilize D4, audio, and recording schemas first. |
| P6-P | Generation status, redacted diagnostics, updater failure, and support runbook | Flow §6 Phase 6 | Requires domain telemetry and release checker. |
| P6-A11Y | NVDA, High Contrast, color independence, DPI, keyboard, IME, popout focus, and reduced motion | Flow §6 Phase 6 | Run the native matrix after H1-H5. |
| ASIO-1 | GPLv3-separated artifact or Steinberg agreement, notice/source/installer separation | Flow §7; ASIO Gate state | Decide the legal/distribution route before artifact work. |
| ASIO-2 | Advertised rate, buffer, format, and channel matrix | Flow §7; ASIO Gate state | Ox-review/integrate the frozen rehydrate diff, then measure each real driver. |
| ASIO-3 | Occupied/control-panel/reset/resync/XRUN/unplug/no-callback matrix | Flow §7; ASIO Gate state | Run fail-closed recovery capture after ASIO-2. |
| ASIO-4 | Matched one-hour ASIO/WASAPI soak and thresholds | Flow §7; ASIO Gate state | Use the exact ASIO-2/3 configuration. |
| ASIO-5 | Physical input-to-pixel latency and five TouchDesigner trials | Flow §7; ASIO Gate state | Use the same interface/rate/buffer/content as ASIO-4. |
| ASIO-6 | Selection persistence, stale/ambiguous lock, native telemetry, and final-package fallback absence | Flow §7 | Integrate the frozen persistence lane and verify the exact packaged artifact. |
| HW-1 | Real Art-Net/sACN nodes and fixtures | Flow §8 | Capture 44 Hz, Blackout, and reconnect on real nodes/fixtures. |
| HW-2 | Enttec USB PRO/DMXKing, OpenDMX analyzer, and RDM/TOD | Flow §8 | Obtain interfaces/fixture/analyzer and run waveform/RDM matrix. |
| HW-3 | Physical MIDI/OSC/TouchOSC/mobile Remote latency | Flow §8 | Run real-device round trips after F1. |
| HW-4 | rekordbox Pedal to DJ Agent to Syndocal wired acceptance | Flow §8 | Ox-review exact peer HEAD, build identity-bound artifact, then isolated LAN matrix. |
| HW-5 | Dual display/HDMI/fullscreen/DPI/refresh/unplug/reorder/GPU reset plus NDI/Spout/camera/screen | Flow §8 | Requires F2 and an exact release artifact. |
| HW-6 | Venue GPU maximum-load and one-hour recording | Flow §8 | Requires completed P5 recording/media. |
| HW-7 | Two-machine crash/restart/partition/rejoin/device-loss rehearsal | Flow §8 | Requires ShowClock witness/fence and M5 hardware setup. |
| HW-8 | Pinned Daslight/SynapseRack/TouchDesigner comparative evidence | Flow §8 | Freeze identical task/start-state/content measurement plans first. |

`COMPLETION_PLAN.md` has two unchecked umbrella rows, not two additional tasks:
its advanced-Timeline row is P5-2 through P5-6 plus P6-H4, and its ASIO row is
ASIO-1 through ASIO-6. `qa/ASIO_INPUT_ACCEPTANCE.md` has five unchecked rows
matching ASIO-1 through ASIO-5; ASIO-6 remains the additional final-package and
persistence proof in the Flow.

Two historical checklists contain no additional independent residual. The 16
unchecked rows in `RELEASE_STATUS.md` map as follows: Art-Net/sACN to HW-1;
USB PRO/DMXKing/OpenDMX to HW-2; MIDI/Touch/mobile Remote to HW-3;
camera/display/GPU/platform operation to HW-5/HW-6 and the M4 subordinate matrix;
Authenticode/Apple/NDI/FFmpeg/notices/tag/clean-machine work to non-checkbox item
6 below; and ASIO licensing to ASIO-1. The 39 unchecked rows preserved in
`qa/SYNDOCAL_PAUSE_HANDOFF_2026-08-23.md` are the historical, less-granular
pre-alpha.10 forms of P0-1 through HW-8, warning item 1, distribution item 6,
and its old final integrated gate. They add no 52nd task, retain no current
resume authority, and remain useful only as provenance.

The following non-checkbox or subordinate work is also explicitly preserved and
must not disappear merely because it is outside the 51-row count:

1. warning ratchet: Windows/Linux measured rows must remain zero; macOS dev and
   release rows are pending, `requiredMatrixComplete=false`, and warning-as-error
   CI ratcheting remains (`Flow` §10 and §14);
2. UI: populated clip grids can hide persistent Import Media, free-text Group and
   Group Picker remain split, and UI-0 through UI-5 still need native,
   accessibility, and comparative exit evidence
   (`qa/SYNDOCAL_UI_IMPLEMENTATION_ROADMAP.md`, UI-0 through UI-5 and known gaps);
3. M4 physical coverage: Art-Net/sACN, USB PRO/DMXKing, OpenDMX waveform,
   physical MIDI, TouchOSC/mobile Remote, camera unplug, RDM, venue recording,
   HAP/ISF/projector material/soak, and Syphon/macOS/Linux remain incomplete or
   partial (`qa/M4_IO_VALIDATION.md`, required hardware and status matrices);
4. M5 two-PC coverage: shared folder, Primary/Standby, failover gap, zero overlap,
   share disconnect, split brain, operator, and evidence rows remain pending
   (`qa/M5_RELIABILITY_VALIDATION.md`, two-machine physical acceptance matrix);
5. DJ peer: current-source artifact fingerprint, wired HELLO/auth/session
   replacement, real Master/switch/loop/release/reconnect/Pedal/restart/next-show,
   and Art-Net/sACN coexistence remain unproved (DJ acceptance physical matrix and
   section 5 above);
6. deferred distribution outside the current Windows-local denominator remains:
   platform/package matrix, Authenticode/Apple signing, BOM/SBOM/notices,
   clean-machine install/upgrade/uninstall, signed updater, and public
   tag/artifact/evidence (`Flow` §9 and `qa/M6_RELEASE_VALIDATION.md`);
7. cleanup remains: the untracked harness has eight known failures and unproved
   recursive staleness; it must be tracked, focused-green, and independently
   Ox-reviewed for the exact target set before execution;
8. the D4 and ASIO companion diffs remain mainline-unintegrated and without final
   native acceptance; use sections 4.1 and 4.2 as their first safe resume actions;
9. never restore the retired six-digit/Raw Input/Enter/15-second consent. The
   current contract is one-click Enable plus native Yes/No for advanced actions.

## 8. Resume order

This section records a future safe order; it grants no implementation authority.
Without a new explicit user instruction, do not select a residual, start an agent,
advance to alpha.11, or execute any step below. After such an instruction, resume
only in this order:

1. verify this document's final main HEAD/upstream/status/stash/process facts;
2. run no implementation until ownership of each frozen dirty diff is confirmed;
3. choose one bounded residual from section 7 and assign non-overlapping files;
4. use Sol > Zen/Ox-alpha (`opencode/x-preview-f-free`) > Terra > Luna; Ox is
   the default bounded implementer and independent reviewer, Terra
   implementation requires independent Ox review, and Luna Max is limited to
   small explicit tasks;
5. keep all safe lanes productive, but serialize native UI, destructive cleanup,
   same-file ownership, and other true dependencies;
6. repeat the exact linker, native, warning, documentation, commit, push, and
   cleanup obligations from `AGENTS.md` at the next meaningful checkpoint. For
   every direct Windows Cargo/Tauri command, first enter the VS 2022 x64
   Developer Shell and pin the resolved Hostx64/x64 linker through
   `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`; fail closed rather than falling
   through to Git for Windows' `usr/bin/link.exe`.

## 9. Claims prohibited at the pause

Do not claim beta/RC/release completion, ASIO completion, D4 completion, current
DJ peer artifact acceptance, hardware acceptance not explicitly captured in
section 2, full Q1-Q4 coverage, zero first-party warnings for unrun matrices, or
successful cleanup for a path that was only inventoried.

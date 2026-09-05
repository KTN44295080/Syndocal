# Unity show video canvas and preview sizing

Branch/base: `codex/syndocal-v1.2` / `bff930559bffb5428aacb9fc2ccb2d97be23a53a`.
The preceding preview clock/decoder/fixture checkpoint is committed and pushed;
its native playback evidence is in `PREVIEW_CLOCK_FIXTURE_AIM_2026-09-05.md`.

## Requested behavior

- Foreground transport3840x2160, background1920x1080; both16:9. Physical Unity
  screen surfaces remain5:2. Display their centered5:2 region, not stretched16:9.
- Existing5:2 media fits within the16:9 transport canvas before Unity cropping:
  vertical bars312px per foreground edge,156px per background edge. Full16:9
  source content needs no added bars. Preserve fixture placements and black-key.
- Timeline right preview removes its180px height cap. Each output remains
  aspect-contained within available width/height; two5:2 outputs become
  width-limited, so some vertical space is geometrically necessary.

## Implementation and boundaries

- Role-specific strict Show Spout dimensions and first/keepalive black buffers.
  Current distinct-composition V2 control API remains; this is a canvas-spec
  revision, not a control-plane or project-schema version change.
- Retired all-HD distinct-composition pair and old all-Main pair are accepted
  only by exact Reset. No automatic conversion, generic legacy output route,
  relaxed identity/mapping/backreference or retirement acknowledgement.
- Source aspect fit belongs to the shared output composition plan, covering
  CPU/GPU and Program previews. Only the exact Spout role/name/canvas applies.
  Display outputs and persisted transforms/mapping remain unchanged. Missing or
  zero source dimensions produce an explicit error for visible source layers;
  unknown live-source dimensions are unsupported in this contract. Transition
  weight0 can still encounter this conservative validation before bus weighting.
- Dedicated preview CSS affects only the preview shelf. Existing font/control
  sizes and Source/Inspector behavior are preserved. Multiple outputs scroll.
- Unity changes are local to video receiver dimensions/UVs and a scoped editor
  action. Do not rebuild or directly overwrite the open scene; unrelated Unity
  dirty/untracked files and all fixture transforms are protected.

## Evidence / remaining

- Browser checker `check-timeline-output-preview-browser.mjs`: PASS at1920x1080
  and1280x720,1/2/6 outputs, mixed aspects, maximum contain sizing, scrolling and
  existing frame lifecycle. Root inspected `sizing-1920-2-2.5.png`.
- `pnpm --dir app exec tsc --noEmit`: PASS.
- Exact MSVC wrapper `run-native.mjs cargo test -p engine --locked show_spout_
  -- --nocapture --test-threads=1`: 7 PASS.
- Same wrapper `cargo test -p syndocal --locked show_spout_ -- --nocapture
  --test-threads=1`:37 PASS, including production old-pair Reset/retirement and
  foreground4K first-black/keepalive. Mock transports, not actual Spout receipt.
- `run-gpu-native.mjs cargo test -p video --locked --features libav
  show_spout_aspect_fit -- --include-ignored --nocapture --test-threads=1`:4 PASS.
  Independent review found the rotated crop fixture was empty; corrected to a
  nonempty crop and asserted both image and black pixels in CPU/GPU frames.
  All4 tests rerun PASS with the strengthened fixture. Initial compile used an incorrect enum name; corrected to
  SpoutSender and reran successfully. Compiler warnings0 for passing runs.
- Native contract and renderer production reviews accepted; integrated native
  build/start are pending. User has now closed
  Syndocal after automatic approval review rejected the earlier agent stop.
- Unity CLI returns authentication-required; no live engine execution API is
  callable in this session. Actual Unity compilation, scene update, Art-Net
  receiver bind and both Spout connections remain unverified. Daslight owned
  UDP6454 in preflight; never terminate it automatically.
- Protected KDMX dirty file `app/scripts/check-viewport-containment.mjs` remains
  excluded from this tranche (SHA256 recorded in the preceding checkpoint).

## Test project and Unity handoff

- New file `C:/Users/kouty/Downloads/DSF2026_NightGirl-Unity-4K-test.sdc`, SHA256
  `c60c7f50471775a1c5b245e1bc8e86d5f7a519a0094e6c1a641bb213d5372179`.
  Original SHA256 `990299ba037742215459751485c5be4368c22fa21384847ee1ef24b19cf99006`
  unchanged. Only composition labels2/3 and output kind/name/dimensions changed;
  IDs, timeline bindings, media, fixtures and Art-Net disabled state preserved.
  Engine ignored local-project sampling fixture reads this new file:1 PASS.
- This is authored output configuration, not a physical start. Loading does not
  acquire a lease or start sender workers. User Enable V2 uses the usual R4
  authority/lease and both initial-black acknowledgements.
- Unity local changes: Runtime/Dsf2026VideoSurface.cs,
  Editor/Dsf2026VideoSurfaceUpdater.cs, Editor/Dsf2026VisualizerBuilder.cs, README.
  Independent review ACCEPT. New menu `ArtNet > DSF2026 > Update existing video
  surfaces` validates both surfaces/receivers/RT GUIDs/names/shaders and5:2
  geometry before one Undo group. It changes only video fields and dedicated RT
  dimensions; rolls back on failure, never saves/rebuilds/opens/plays the scene.
  Existing material tiling is restored when crop is disabled. HDRP base/emission
  and foreground shader all consume the updated ST property blocks.
- C# syntax checked, but Editor.log predates changes, so actual Unity compilation
  is not established. User asked to open the exact scene and run the scoped menu,
  without Play. Unity files are not staged: the project has substantial preexisting
  dirty/untracked work, including this DSF2026 tree.
- User confirmed the Unity updater completed, then confirmed Daslight stopped;
  UDP6454 was no longer occupied. User asked to save scene/project and enter Play.
  This confirms the user could execute the editor menu, not an independent Unity
  compiler-warning count or received-frame measurement.
- Integrated `pnpm --dir app tauri build --no-bundle`: PASS2m50s, warnings0,
  exact MSVC linker gate. Log `unity-4k-preview-sizing-native-build.log` under
  `target/qa/snapshot-cleanup-20260905`. Native SHA256
  `87BCF247EAF549BCDF73BEC9155882E2E8D4A88E7AA1968395C0FEBDE6C83037`.
  Normal launch PID91532, exactly one responsive maximized Syndocal main window,
  no debugger flags supplied; proof `unity-4k-preview-sizing-normal-launch.json`.
- User instructions issued: open new4K test copy; SETUP/VIDEO enableV2 Spout;
  SETUP/I/O/DMX Connections/Diagnostics enable Art-Net loopback; play unified
  Timeline. Unity lighting/foreground/background visibility or exact error reply
  remains pending. Do not claim actual Art-Net/Spout receipt or4K output cadence.

## Pending: one-step output activation and reported rejection

- Base `66c59255de6b0f100adf85190c42fd57e5a702f8`, branch
  `codex/syndocal-v1.2`. User reports Art-Net activation succeeded after global
  Enable. Unity PID58932 owns UDP6454; visual receipt is not independently proven.
- Standalone Art-Net/Spout activation now performs canonical output preparation,
  verifies Ready/Both and selects fresh lease authority before route publication.
  No separate global Enable prerequisite. Same-action clicks share one operation;
  different overlapping actions are rejected without a deferred queue. Existing
  S0 and ambiguous-ownership rejection remain. Canonical Enable restores configured
  Both outputs; it is not a promise to activate only one transport.
- Simplified Spout button label and Japanese translation; removed user-facing V2
  and redundant confirmation wording. No control-size changes.
- Focused actual-controller `node app/scripts/check-dmx-show-setup.mjs`: PASS;
  independent review ACCEPT. `pnpm --dir app exec tsc --noEmit`: PASS, warnings0.
  Owned diff check PASS. Localization checker fails at3658/3691 (99.1%): unrelated
  existing strings remain untranslated; the changed Spout label has a matching
  Japanese entry. Full localization acceptance is not claimed.
- User corrected the Spout error to `OutputControl rejected (invalid_request)`.
  The authored4K copy passes static topology checks; the currently loaded project
  is not yet confirmed. Native current-action checks discard the concrete topology
  error, while request/lease validation can return the same code. Do not infer a
  GPU/sender failure or claim this rejection fixed by lease preparation alone.
- Separately investigated recovery-owner error: a durable old-owner publication
  intent with a missing receipt can be resent under the old owner. Receipt absence
  does not prove the request was never reserved. No user storage was cleared or
  owner replaced; a durable recovery fix remains separate unresolved work.
- User confirmed closure; PID91532 remained, but the maintained build wrapper
  verified its exact checkout path and stopped it successfully. Native gate
  `pnpm --dir app tauri build --no-bundle`: PASS1m58s, first-party warnings0
  (prior native gate0, delta0), exact pinned MSVC/where-first verified. Log:
  `target/qa/snapshot-cleanup-20260905/one-step-output-native-build.log`.
- Launched PID60304, one responsive maximized Syndocal main window, SHA256
  `2890F6FDD9BAAFDA995AF78E5CD916A9E3D368964C84099043C42BC9E56F22A3`.
  Temporary diagnostic port38479 is enabled for read-only snapshot inspection;
  no timing hooks installed. Normal restart without the port remains due after
  diagnosis. Evidence: `one-step-output-diagnostic-launch.json` in the same folder.
- Live read-only `get_snapshot` after startup shows outputs=[] and only Main
  composition, not the user's prior project. User asked to open the4K copy;
  this empty startup state does not establish the cause of the prior rejection.
  Next: read loaded topology and reproduce the rejected action through the user.
  Spout rejection remains unresolved; commit covers one-step preparation only.
  Protected viewport checker remains unchanged and excluded.

## Live blocker after one-step activation checkpoint

- Checkpoint `d50bdef56cf5c2cb954950f108d1e661103d8290` pushed/upstream equal.
  Read-only live topology still has original Display5:2 outputs and Japanese
  composition names; visible title is `DSF2026_NightGirl-AV-test.sdc *`.
  Disk4K copy SHA remains unchanged. The requested replacement was not adopted.
- User reports `Managed output authority is already Faulted and remains retained`.
  Read-only `get_output_ownership_status` confirms Failed/Standby, desired Both,
  both output permissions false, with exact cause:
  `Managed output lease external project replacement; Managed DMX retire completion
  failed: InDoubt("Managed show DMX fail-stop found no retained USB-DMX route for
  its exact worker proof")`.
- Production keepalive calls the USB-specific engine retirement proof for every
  Both run, including Art-Net-only. This invalid applicability requirement blocks
  project replacement; do not clear Faulted or weaken existing USB proof. Engine
  implementation/review is in progress for exact no-USB topology retirement.
- A temporary read-only IPC result observer `__projectOpenReadProbe` is installed
  for load_project/load_project_path only. Remove it and restart without diagnostic
  port after verification. No project-load/output mutation has been automated.

## No-USB retirement correction

- Engine retirement now accepts route-absent USB only with authoritative pristine
  serial status: inactive, no queued/live/physical zero history, worker shutdown
  complete and no fault. Prior worker loss/uncertainty remains rejected.
- Exact active loopback Art-Net sends one zero datagram then retires; disabled
  sender-absent DMX sends nothing and never binds a socket. Existing USB physical
  zero/shutdown proof and mixed-route/input rejection remain unchanged.
- Completion evidence distinguishes whether USB was required, retains real USB
  status, and revalidates authority/status/absence for replay prevention. Cleanup
  no longer invents a USB fault when USB was demonstrably unused. Manager Faulted
  protections were not weakened; the required process restart replaces this run.
- `node target/qa/recording-atomic-20260905/run-native.mjs cargo test -p engine
  --locked managed_fail_stop -- --nocapture --test-threads=1`: final23 PASS,
  0failed,1028filtered; exact MSVC gate, first-party warnings0 (delta0).
  Initial22/1 failure exposed a fake-worker fixture that omitted production's
  active-status publication. The fixture now publishes it before deleting the
  worker; missing-worker rejection assertions remain. Independent review ACCEPT
  for both implementation and fixture adjustment; owned diff check PASS.
- PID60304's visible WebView closed before the next build (debug port refused),
  while the single-instance process remained. Observer removal could not connect;
  process replacement removes its in-memory observer.
- Native `pnpm --dir app tauri build --no-bundle`: PASS2m54s, warnings0,
  pinned MSVC gate and exact PID60304 retirement verified. Log
  `target/qa/snapshot-cleanup-20260905/no-usb-retirement-native-build.log`.
  Launched PID64164, one responsive maximized Syndocal main window; SHA256
  `FD58D145294DDB28A3AE3EC548B58810AFD77F1AF807EAF2FC960A13179B31C8`.
  Evidence `no-usb-retirement-diagnostic-launch.json` in the same folder.
- Port38479 and project-open result observer are temporarily enabled on PID64164
  for user-driven load verification. Startup ownership is ordinary StartupDenied
  Standby, not the prior managed-retirement error. User asked to open4K copy.
  Actual successful replacement and Unity Spout receipt remain unverified; remove
  observer and restart normally after these checks. Commit covers source regression
  and native build/startup evidence, not end-to-end Unity acceptance.

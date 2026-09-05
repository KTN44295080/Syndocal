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

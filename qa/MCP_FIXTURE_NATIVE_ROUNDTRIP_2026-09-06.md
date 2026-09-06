# Native MCP fixture round trip

Branch `codex/syndocal-v1.2`, base `5cb4540c3d61de1a4cfa7fd8db66dc74de23b032`,
internal product `1.2.0-alpha.69`.

## Reproduction

MCP initially returned an empty startup project. The documented single-instance
command-line open route (completion flow, alpha.55 checkpoint) is implemented by
`project_paths_from_single_instance_args` and the frontend's ordinary fenced
`loadProjectPath`. Independent review confirmed that project replacement finishes
disarmed and does not automatically arm output. Saved playback flags still
require inspection; this is not a claim about arbitrary show files.

Prepared an offline derivative of the user's
`C:/Users/kouty/Downloads/DSF2026_NightGirl-Unity-4K-test.sdc` under
`target/qa/mcp-fixture-roundtrip-20260906/DSF2026-MCP-offline.sdc`.
The only changed values were the two video output `enabled` flags, set false.
DMX and Timeline playback were already disabled. No authored-video override was
present. All 46 fixture records were preserved exactly; no synthetic fixture was
created. Original SHA256:
`c60c7f50471775a1c5b245e1bc8e86d5f7a519a0094e6c1a641bb213d5372179`.
Derivative SHA256:
`972572bf38b9976f90900bd6daf7fa04fe17ddd712916892e41eb7d343dc460f`.

The exact running checkout process PID 108972 accepted the official forwarding
invocation (temporary PID 12492 exited 0). MCP read 46 fixtures at E1/R0/hash
`65586db811fde56ba7946142f797f9e9b31207049e869d9c352ad154c0d0c1ed`.
Output status was Ready/Standby/ProjectSwapDisarmed, lighting/video denied;
both video outputs were disabled. No playback or physical output was requested.

Fixture 1 (`mega par profile ep`) had yaw 180. The intended reversible test was
180 -> 181 -> 180, preserving position/pitch/roll. Request
`8d32b806-947f-4d4e-96b7-12d9c9b217c3` completed with `ok:false`,
`mutation_not_confirmed`: `Tauri mutation admission requires unsigned integer
field 'projectTransactionId'`. A new MCP read confirmed yaw 180 and identical
E1/R0/hash. The failed intent was not resubmitted. Original request IDs were
polled for pending results; no unknown mutation was replayed.

## Cause and verification boundary

App's generic transaction wrapper correctly sends `set_fixture_transform` as
`{request: ticketedRequest}`. The native command accepts this typed request,
but the outer RendererTicketedProjectMutation admission reads ticket fields
from the top-level JSON object. The same mismatch applies to the typed
`move_cue_between_scene_banks_batch` route. It affects the shared GUI admission
path, not only MCP. The correct repair is command-specific envelope parsing,
retaining the existing transaction/owner/epoch/lane checks and rejecting
malformed or retired forms; no ticket synthesis or alternate mutation path.

The repair extracts envelope selection into `renderer_ticketed_admission.rs`.
Only those two typed-ticket commands use a single outer `request` object;
flat/mixed/malformed envelopes are rejected. Other commands retain their flat
ticket, including `snap_timeline_items` and effect commands whose `request`
contains business data instead. Independent review checked all 132
RendererTicketed roster entries and accepted the stable parser/wiring diff.
Existing required-u64/string readers and owner/epoch/lane checks are retained.
The MCP adapter was not changed; its 12 fake-broker integration groups passed.

## Accepted evidence

Exact MSVC 14.44.35207 linker pin/PATH-first checks were used throughout.
`cargo test -p syndocal --locked renderer_ticketed -- --nocapture --test-threads=1`
passed 6 tests (1741 filtered, 0.05s).
`cargo test -p syndocal --locked ticketed_invoke_fields_reject_wrong_case_type_and_fractional_numbers -- --nocapture --test-threads=1`
passed 1 test (1746 filtered). `pnpm --dir app tauri build --no-bundle` passed:
release 2m33s, Vite 9.54s, App 499.96kB unchanged. First-party compiler warnings
and Vite advisories are zero, baseline zero/delta zero.

Launched the exact checkout release executable, PID 109224, SHA256
`20149052DD008445784B5A71F95A90BCEBEC5ABB24D940B2F7E8AF4547AC01FC`;
one responsive maximized Syndocal main window. Reopened the same offline
derivative through the official command-line route, then used MCP tools for
all fixture operations. Pending results were queried by their original UUID.

- `dda92cf7-9e3b-47c1-9a3d-2f2ed3a9c460`: yaw 180 -> 181 succeeded with
  `verification: committed_project_state`, E1/R1/hash
  `8048c6e80bef87e4f90db36c167c29f7bbcd70a5fbd08672737144473d56796b`.
  An independent MCP fixture read confirmed 181.
- `17667fae-9174-43e7-8477-c5313546fb41`: deliberately stale E1/R0 intent to
  set 182 was rejected before bundle capture. A fresh read remained 181/E1/R1.
- `7d319c1d-0232-4895-9bb7-790949061a7b`: restoration to 180 succeeded with
  committed-state verification at E1/R2. The complete checkpoint hash returned
  to the original value; a final fixture-list read matched all 46 original
  fixture views exactly. Original and derivative file SHA256 values remained
  unchanged. No save operation was performed.

Final MCP diagnostics showed Ready/Standby/ProjectSwapDisarmed, both outputs
disabled, lighting/video denied and Timeline paused. The app is left on the
clearly named offline test copy. This closes native MCP fixture mutation,
verification, stale-token rejection and restoration acceptance. Unity video
restoration, GUI drag-release, GUI Undo/Redo and project disk saving remain
separate acceptance; no physical output or visual interaction was exercised.

The earlier fixture-bridge checkpoint's empty-project obstacle was resolved
using the documented official path, not an undocumented invoke or fake fixture.
Evidence files live under `target/qa/mcp-fixture-roundtrip-20260906/`:
`preparation.json`, `loaded-mcp-probe.json`,
`reproduced-admission-failure.json`, `admission-tests.log`,
`ticket-fields-tests.log`, `native-build.log`, `native-launch.json`,
`repaired-loaded-mcp.json`, `success-native-roundtrip.json`,
`final-mcp-probe.json`, `verification.json`. `verify.mjs` asserts the observed
round-trip, stale rejection, all-fixture equality, output denial and file hashes.

## Subsequent read-only Unity acceptance audit

Base `ab5589723c66f98ab6791952ece71042dd64bfd9`; product remains alpha.69.
The Unity MCP connection is now available and was pinned to instance
`590bbd28-07bd-4938-adc3-e6283ede43e7`, project
`ArtNetForUnity-OshinoTools-HDRP-Test`. Live MCP reads found
`DSF2026_Visualizer` loaded and clean, Play active, not paused, compiling or
updating. Both screen objects and their Spout receivers/surface components are
active. This audit made no Unity scene, playback or asset changes.

Scoped component reads confirmed:

| Surface | Receiver name | Expected source | Crop to physical 5:2 | Runtime observation |
| --- | --- | --- | --- | --- |
| Foreground | Syndocal Foreground | 3840 x 2160 | true | Stale, connected false, contract supported |
| Background | Syndocal Background | 1920 x 1080 | true | Stale, connected false, contract supported |

Both contract-error strings were empty. Receiver texture references exist, but
neither their existence nor Play mode proves fresh frames. Syndocal remains on
the output-disabled offline derivative described above; this audit did not
activate output. Consequently the Stale observations do not reproduce the
reported Video BO recovery failure or establish frame cadence.

The current acceptance boundary is:

- Native MCP fixture editing, independent readback, stale-intent rejection and
  restoration are complete as evidenced above.
- Video BO/Follow fixes and Spout worker recovery have focused and Windows
  native evidence in [the overnight checkpoint](OVERNIGHT_FOLLOW_MAPPING_2026-09-06.md)
  and the Spout recovery section of [the worker lifecycle checkpoint](RECORDING_LIFECYCLE_2026-09-06.md).
  Unity Video BO recovery, lighting/video isolation and frame cadence still
  require an active-output observation; receiver configuration alone does not
  close them.
- GUI drag-release and Undo/Redo remain separate from the native transform
  round-trip. Project disk saving was not exercised.
- [Recording lifecycle](RECORDING_LIFECYCLE_2026-09-06.md) owns the implemented
  worker retention, separation and diagnostic-reader joining evidence. Full
  renderer/encoder/Drop cancellation deadlines remain open; the bounded
  explicit Stop wait is not a total shutdown deadline.

No new performance improvement or full-roadmap completion is claimed by this
read-only audit. Original show files, Unity assets and the unrelated dirty
viewport checker remain preserved. Next acceptance should use the original
Unity test show deliberately: the currently open offline copy intentionally
has both video outputs disabled.

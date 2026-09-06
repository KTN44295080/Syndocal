# MCP Video BO control

Base `c7c45bcecc7a3ec07a344118216af3e0125e189f`, branch
`codex/syndocal-v1.2`, internal product `1.2.0-alpha.69`.

The existing MCP interface could inspect runtime state but could not operate
Video BO. The additive `syndocal_set_video_blackout` tool takes a caller UUID,
boolean `enabled`, and the exact E/R/H project token from a fresh read. The
broker persists its identity as a mutation: duplicates cannot redispatch,
changed arguments conflict, and unresolved intents survive restart as unknown.
The adapter never invents a replacement UUID or automatically repeats an intent.

The renderer uses the same `executeTargetBlackout` operation as the GUI. It
requires the existing active Both lease and never acquires, enables or arms
output, changes routes, or releases S0. The expected project is checked against
the initial bundle and against the actual OutputControl authority fence used
for native dispatch. Native CAS and the existing exact-request receipt retry
remain authoritative. Missing refresh hooks reject before mutation. After a
definitive receipt, canonical authority convergence precedes snapshot refresh;
success additionally requires the receipt's exact E/R/H and Video BO bit in a
fresh bundle. A concurrent replacement cannot be reported as success merely
because its boolean happens to match.

Native source inspection caught an invalid readback assumption during review:
public snapshots deliberately omit the backend-only `authored_video` payload.
`video.blackout` itself retains the authored target bit; S0 is separate in
`safety_blackout_engaged`. The implementation and runtime diagnostic field now
use this public bit explicitly. An engine regression exercises actual target
commands and S0 publication before cloning the public snapshot, proving this
contract for both Video BO values without activating physical output.

MCP request execution, startup/disposal, and Solid/Tauri main-window mounting
are separate small modules. The App supplies its existing canonical refresh
callbacks. Deferred loading is guarded against unmount before registration;
an existing receiver is disposed once. No per-frame polling, rendering, copying
or transport work was added.

## Focused evidence

- Node MCP adapter: 14 integration groups, fake loopback only.
- Frontend real bridge processor/runtime: 9 groups; bootstrap lifecycle: 4 groups.
- OutputControl runtime, target controller and target-authority convergence: PASS.
- Frontend Tauri invoke inventory: exactly 454 commands; validator unchanged.
  Explicit typed forwarding preserves the default transport behavior without
  allowing an invoke callable to escape through a tuple or untracked alias.
- Exact MSVC native `agent_bridge_` filter: 8 passed, 0 ignored, 1741 filtered.
- Exact MSVC engine public-video/S0 projection filter: 1 passed, 1060 filtered.
- TypeScript no-emit: PASS. Focused diff checks: PASS.

Native test logs and generated verification scripts are under
`target/qa/mcp-video-blackout-20260906/`. All implementation lanes and final
mount/bootstrap integration received independent review. The unrelated dirty
viewport checker and original show/Unity files remain protected.

## Native integration

`pnpm --dir app tauri build --no-bundle` passed with exact MSVC pin and
PATH-first validation: final release 2m30s, Vite 8.38s, App 499.87kB.
Initial integrations briefly raised the App chunk above 500kB. Moving MCP mount
and deferred lifecycle ownership into their own modules resolved the advisory
without changing thresholds or reducing UI sizes. Prior accepted App was
499.96kB. Final first-party compiler warnings and Vite advisories are zero
(baseline zero, final delta zero); no FPS improvement is claimed.

Launched this checkout's exact `target/release/syndocal.exe`, PID 6456, SHA256
`30EF81F5FA1CC6140A62333741CF7B618FC06A57029528C7E34447423E3C12FE`.
Exactly one responsive maximized Syndocal main window was verified. The
documented single-instance CLI opened the existing output-disabled derivative
`target/qa/mcp-fixture-roundtrip-20260906/DSF2026-MCP-offline.sdc`.
A fresh official MCP stdio connection discovered six tools and exercised:

- `4ae857cd-6dfb-458f-b0e6-6d89368bed9c`: Video BO false with stale E2/R0
  rejected before editing the actual E1 project (`request_rejected`).
- `e58ee123-91aa-4f90-a7ed-120a216203ad`: Video BO false with correct E1/R0
  rejected because no active Both lease exists (`request_rejected`).

Both were polled by their original UUID to terminal failure; neither was
resubmitted. Reads before and after agreed exactly on E1/R0/hash
`65586db811fde56ba7946142f797f9e9b31207049e869d9c352ad154c0d0c1ed`,
Video BO, lighting BO, S0, both disabled video outputs, paused Timeline and
Standby/ProjectSwapDisarmed ownership. Original show, derivative and protected
viewport-checker hashes match the preceding checkpoint. No save, output
activation, Unity change or GUI BlackOut click occurred.

Evidence: `native-final-build.log`, `native-launch.json`,
`native-negative-probe.json` and `probe.mjs` in the generated directory above.
The app remains on the output-OFF test copy. This proves native admission and
rejection behavior; successful active-lease application currently has focused
software receipt/readback evidence, not a live Unity observation.

This checkpoint does not claim active-output Unity Video BO recovery, frame
cadence, GUI drag-release/Undo/Redo, or a full recording cancellation deadline.

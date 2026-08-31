# Show Spout V2 and Reset Boundary

## Clean break

`syndocal.output.show_spout_outputs.enable.v1` and its Tauri/UI route are
retired. Output-control command schema `8` exposes only the payloadless
`syndocal.output.show_spout_outputs.enable.v2` action and the local,
payloadless `syndocal.output.show_spout_outputs.reset.v1` action.

Enable derives, rather than accepts, its targets from the authored project:

- `Syndocal Background` -> the unique exact `Background Video2 Camera`
  composition;
- `Syndocal Foreground` -> the unique exact `Foreground Video 1`
  composition.

Both targets must be nonzero, distinct, and not `Main` (id `1`). Missing or
duplicate exact names, a malformed pair, a third Spout sender, or inconsistent
composition output backreferences fail closed before new show senders are
published. Ordinary `Display` outputs remain outside this strict pair.

## Shared inventory reconciliation

The exact inventory guards are derived from the current shared tree, not
blindly incremented:

| Inventory | Old | New | Reason |
| --- | ---: | ---: | --- |
| Tauri routes | 503 | 509 | Five frozen USB-DMX routes plus local Spout Reset; Enable V1 -> V2 is a replacement. |
| Engine commands | 273 | 276 | Frozen USB-DMX `EnableShowSerialDmxSafetyBlackoutRoute` and `StopShowSerialDmxSafetyBlackoutRoute`, plus the separate atomic `ResetShowSpoutOutputsExactPublished`; the authority-loss rename is count-neutral. |
| Frontend invoke manifest | 443 | 449 | The five USB-DMX routes plus Spout Reset; the V1 -> V2 Enable rename has no count delta. |
| Legacy source inventory | 1541 | 1556 | `509 + 276 + 116 remote + 206 MIDI/OSC/DMX + 449 frontend`. |
| Canonical operations/direct sources | 41 | 44 | USB-DMX Enable/Stop and the R4 Spout Reset. |
| Canonical source inventory | 1574 | 1589 | Legacy source inventory plus 30 app and 3 project-file keyboard sources. |

The six unclassified sources are five deliberate frozen USB-DMX inventory rows
(three local status/selection routes and two unavailable engine commands), plus
the exact Reset engine descriptor. The latter is represented canonically by its
local Reset Tauri route; none creates a remote adapter or broadens Spout
authority.

## Audit remediation: reset mutation boundary

| Old | New | Reason |
| --- | --- | --- |
| `RetireShowSpoutOutputsPublished` mixed authority-loss compensation and user Reset cleanup. | `RetireShowSpoutOutputsAfterAuthorityLossPublished` is explicitly physical-teardown compensation; `ResetShowSpoutOutputsExactPublished` is a separate exact command. | A user Reset must validate the entire pair and every Spout sender before mutating either recognized output. |
| A third Spout sender could cause best-effort retirement to remove the recognized pair before reporting the conflict. | Exact Reset counts the two named senders and every third Spout sender before retaining/removing anything; invalid state returns without authored mutation. | The local R4 Reset cannot become a generic delete route. |
| Inventory stopped at 275 engine commands / 1555 legacy sources / 1588 canonical sources. | Engine 276 / legacy 1556 / canonical 1589, with exact assertions and source guards. | The new exact Reset engine descriptor is a real additional source; frozen USB semantics remain unchanged. |

## Local reset

Reset is an R4 local-window action with a parented native Yes/No confirmation.
It needs no active output lease and is therefore available in Standby, but it
keeps the exact caller, owner, project, output-fence, serialized-cleanup, and
native-retirement boundaries. It is not a generic deletion surface.

It removes only an absent pair (no-op), the exact retired V1 same-`Main` pair,
or the exact V2 pair. Active physical senders retire before the canonical engine
pair; an inactive recognized pair retires directly. Ambiguous, partial,
duplicate, third-sender, bad-backreference, or unresolved-native-retirement
state remains unmodified and reports failure. The target compositions remain;
removing authored outputs advances the project checkpoint/fence.

## Evidence boundary

This source/checker checkpoint covers protocol shape/schema rejection,
composition derivation, strict pair/reset classification, first-black sender
identity, and fenced cleanup paths. It does not claim a native release build,
Spout SDK/hardware sender observation, receiver validation, or venue acceptance.

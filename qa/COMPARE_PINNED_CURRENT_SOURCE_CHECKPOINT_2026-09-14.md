# COMPARE-PINNED-001 current-source checkpoint

- Date: 2026-09-14
- Branch: `codex/showclock-review-20260912`
- Base: `28f8fac6` (`qa: record ShowClock venue source rerun`)
- Marker: `COMPARE-PINNED-001`
- Scope: current-source comparison precondition audit only

## Result

`COMPARE-PINNED-001` remains `Open`. No product code was changed for this checkpoint, and no parity or equivalence claim is made.

The authoritative Q1 rows still require direct, pinned measurements:

- `COV-COMPARE-VIDEO-001`: SynapseRack V01-V17 with a pinned build, license, reference hardware, and the exact comparison content.
- `COV-COMPARE-LIGHTING-001`: Daslight task/parity measurements with pinned version, license, hardware, and content, preserving first failures and unmeasured rows.

The existing acceptance documents remain plans rather than completed comparator runs:

- `qa/SYNDOCAL_SYNAPSERACK_VIDEO_OPERATOR_BENCHMARK.md` requires the SynapseRack pins and operator measurements.
- `qa/DASLIGHT_PARITY_COMPLETION_PLAN.md` records the remaining Daslight-side measurements, live node/fixture/controller, and external operator boundaries.
- `DEC-COMPARE-001` remains `Open`; the required versions, licenses, tiers, and reference hardware are not recorded as an approved comparison fixture.

## Current-source evidence

The following current-source check passed on the current source:

```text
pnpm.cmd --dir app run check:status
status model helpers ok
exit code: 0
```

This only verifies the local status-model contract. It is not a comparator measurement and cannot substitute for SynapseRack or Daslight execution.

## Takeover rerun — 2026-09-14

The comparison precondition was rerun after takeover from `28f8fac6` against
the current branch and passed:

```text
pnpm.cmd --dir app run check:status: PASS
  status model helpers ok
  exit code: 0
```

The rerun found no newly pinned SynapseRack or Daslight build, license/tier,
reference hardware, content manifest, synchronized capture, or operator
measurement package in this checkout. The existing partial Daslight operation
counts and current Syndocal software checks remain non-comparator evidence;
they do not close either comparison Q1 row. No product code was changed by
this rerun.

## Acceptance boundary

Not run in this checkpoint or takeover rerun:

- pinned SynapseRack or Daslight builds;
- license/tier confirmation;
- reference hardware and fixture/content topology;
- synchronized video, lighting, or semantic/profile comparison capture;
- first-failure and unmeasured-row operator record;
- physical output, external client, venue, or release acceptance.

## Resume procedure

1. Assign the comparative acceptance owner and resolve `DEC-COMPARE-001`.
2. Record exact comparator versions, licenses/tiers, hardware, content, and Syndocal artifact hash.
3. Execute the V01-V17 and Daslight task matrices with synchronized capture.
4. Preserve every first failure and every unmeasured row; do not infer parity from counts or loopback.
5. Update both Q1 rows and the completion ledger only from the resulting evidence.

## Takeover continuity — 2026-09-14

At HEAD `b9c95b3d`, `pnpm.cmd --dir app run check:status` still passes. The
takeover introduced no comparator or product-runtime artifact, and no pinned
SynapseRack/Daslight version, license, hardware/content topology, synchronized
capture, operator measurement, or physical-output evidence became available.
`COMPARE-PINNED-001` remains `Open`; parity must be established from the named
V01-V17 and Daslight records rather than inferred from software or loopback
counts.

## Takeover continuation — current-source comparison precondition recheck after Video repair — 2026-09-14

At current source HEAD `56a81537`, the local status precondition was rerun:

```text
pnpm.cmd run check:status
status model helpers ok
exit code: 0
```

No pinned SynapseRack V01-V17 or Daslight build, license/tier, reference
hardware, content manifest, synchronized capture, comparator operator record,
or physical-output measurement was added. The status-model pass is not parity
evidence and does not close either comparison Q1 row. `DEC-COMPARE-001` and
`COMPARE-PINNED-001` remain `Open`; completion still requires the named
versions, approved fixture, synchronized matrix, and first-failure/unmeasured
row records.

# DMX Art-Net Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `DMX-ARTNET-001` (section 8, Open)
- Q1 rows: `COV-OUTPUT-LOCAL-001`
- Branch: `codex/showclock-review-20260912`
- Base: `594eedd729ee416876a41ed439664c08dbd93e60`
- Authority: `qa/M4_IO_VALIDATION.md` and the section 8 Flow requirements

This checkpoint covers current-source DMX preparation, addressing, ownership,
and blackout contracts. It does not claim Art-Net/sACN hardware acceptance.

## Verification

```text
pnpm.cmd --dir app run check:dmx-addressing
pnpm.cmd --dir app run check:output-ownership
pnpm.cmd --dir app run check:safety-blackout-runtime
node app/scripts/check-dmx-show-setup.mjs
```

Result: exit code 0.

- DMX addressing helpers: PASS.
- Output ownership static contract: PASS.
- Safety blackout runtime contract: PASS.
- DMX show setup UI contract: PASS, including canonical acquire/recover/reuse,
  fresh authority, fail-closed preparation, singleflight, busy rejection, and
  loopback-before-S0 boundaries.
- First-party warning count observed in this focused source run: `0`.

## Takeover rerun — 2026-09-14

The DMX addressing, output ownership, safety-blackout, and show-setup checks
were rerun after takeover and all passed. The show-setup contract again kept
loopback-before-S0 and fail-closed preparation boundaries. No Art-Net/sACN node,
fixture, reconnect path, or sustained 44 Hz physical output was exercised.

## Unresolved acceptance

`DMX-ARTNET-001` stays Open. Real Art-Net/sACN nodes and fixtures still require
addressing, RGB/wheel, pan/tilt, intensity, reconnect, topology, and 44 Hz
continuity evidence. The historical serial-DMX slice does not establish this
network-output marker, and no network or fixture output was emitted here.

Next action is the named Art-Net/sACN node and fixture matrix with raw packets,
fixture observations, reconnect timing, and exact output-owner identity.

## Takeover continuation — current-source Art-Net preparation recheck — 2026-09-14

At HEAD `de9eb1c0`, DMX addressing helpers, output ownership, safety-blackout
runtime, and the DMX show-setup UI contract all passed. The show-setup result
retained canonical acquire/recover/reuse, fresh authority, fail-closed
preparation, same-action singleflight, cross-action busy rejection, and
loopback-before-S0 boundaries. No Art-Net/sACN node, fixture, reconnect path,
or sustained physical 44 Hz output was exercised; `DMX-ARTNET-001` remains
`Open`.

## Takeover continuation — current-source Art-Net preparation recheck after Video repair — 2026-09-14

At current source HEAD `fadecced`, the focused DMX preparation contracts were
rerun:

```text
pnpm.cmd run check:dmx-addressing
dmx addressing helpers ok

pnpm.cmd run check:output-ownership
output ownership static contract: PASS

pnpm.cmd run check:safety-blackout-runtime
safety blackout runtime contract: PASS

node scripts/check-dmx-show-setup.mjs
DMX show setup UI contract: PASS (canonical acquire/recover/reuse, fresh
authority, fail-closed preparation, same-action singleflight, cross-action
busy rejection, loopback-before-S0 boundary)
```

All commands exited `0`. Current-source addressing, ownership, blackout, and
show-setup preparation remain fail-closed and loopback-before-S0 bounded. No
Art-Net/sACN node or fixture was opened, no network packet was sent, and no
reconnect, topology, RGB/wheel, pan/tilt, intensity, or sustained 44 Hz
physical-output artifact was produced. `DMX-ARTNET-001` remains `Open` pending
the named node/fixture matrix with raw packets, fixture observations,
reconnect timing, and exact output-owner identity.

# Multi-universe USB-DMX plan — 2026-10-14

Owner input (2026-10-14): "FTDIケーブルを2本以上". Each DMX universe must be
able to drive its own Enttec Open DMX-style FT232R cable, and the DMX output UI
should become one universe list instead of separate Show DMX / Network
universes / Advanced panels.

## Facts that shape the design

- **The binding file is already at content version 2.** It is
  `serial-dmx-machine-binding-v1.json`, writes `version: 2` and accepts 1
  (`app/src-tauri/src/serial_dmx_machine.rs:15-16`). Any multi-universe
  schema is version 3.
- **One Open DMX gate cannot serve two cables.** `OpenDmxSafetyWriteGate`
  holds one physical-write mutex across BREAK/MAB/write/flush, plus one
  zero-write hold counter (`crates/io/src/serial_dmx.rs:325-336, 495-507`).
  Sharing it would cause three problems:
  - it would halve each cable's frame rate;
  - cable B's startup zero hold would zero cable A;
  - a wedged driver on one cable would stall the other.
- **Network routes `remove` their frame from `frames_by_universe`.** USB
  frames must be captured before that loop, as U0 already is
  (`crates/engine/src/lib.rs`, tick send).
- **The channel-500 zero rule is a U0 / DSF Unity rule only.**
- **Network routes and USB stop need opposite S0 states.** Applying network
  routes requires S0 clear; stopping USB requires S0 engaged.

## Decision

USB becomes a first-class transport per universe. To protect the proven U0
path:

- **The existing U0 route stays exactly as it is.** That covers the Unity
  Art-Net mirror coupling, the channel-500 rule, the root gate, the status
  store and the v1 commands.
- **New independent USB-DMX universes** (U1 and up, at most 3) are added
  beside it. Each one has:
  - a **device gate** that shares the S0 latch but has its own write mutex
    and zero-hold counter;
  - its own status store;
  - a zero-first start, a zero-then-shutdown stop, and a fault that latches
    the shared S0 first.
- **S0 stays global** (fail-closed default). A fault on any cable engages S0,
  so every cable sends zeros within one frame. Releasing S0 is refused while
  any USB route is faulted.
- **Cable bindings live in a separate machine-local file**,
  `usb-dmx-universe-bindings-v1.json`: `{version: 1, universes: [{universe,
  identity}]}`. This is the as-built decision (phase 2, `deecf6d7`): the
  Universe 0 file and its version stay untouched instead of going to
  version 3.
  - One physical cable may not be bound twice: same instance ID, same COM
    name, or same VID+PID+serial (a cloned serial counts as the same
    cable).
  - That rule is enforced across both files at bind time, and again when
    either kind of cable starts (Tauri resolution and engine), because an
    older build that ignores the universes file could rebind Universe 0.
  - An unreadable Universe 0 file blocks binding or starting any other
    universe; an unreadable universes file blocks Universe 0 binding and
    start.
- **Enable and stop go through the existing OutputControl lease/fence path**
  as new operations that carry the universe.
- **The UI** becomes one universe list:
  - columns: Universe | Transport (Art-Net / sACN / USB device) | Destination
    | Status | Enabled;
  - one Apply, which refuses a step that needs the other S0 state and never
    changes S0 itself;
  - diagnostics move to a disclosure.

Open owner questions, resolved for now with the fail-closed default:

1. Should the U0 Unity coupling stay? It stays unchanged.
2. Should one cable fault black out all universes? Yes, S0 stays global.

## Phases (each keeps main green and fail-closed)

1. **io device gate + engine independent USB universes, with engine tests.**
   - Tests: two cables run at once; a fault on B latches global S0 and A
     sends zeros; release is blocked until B is re-armed or cleared; startup
     on B does not zero A; managed fail-stop and retirement include the
     independent universes.
   - Independent review.
2. **Tauri binding store v3, commands, control-plane inventories and
   counts, manifests.** Independent review.
3. **Unified universe list UI and checks.** Independent review.
4. **Physical verification with two FTDI cables:** runbook blocks C–E, with
   U0 on cable A and U1 on cable B.
   - Both cables should hold about 32.5 fps at the same time.
   - Unplugging B while live should send everything to S0 and A to zeros.
   - Re-arm B, then release S0.
   - New/Open with both cables.

## Risks

- **Identical clone serial numbers** could swap universes after a port swap.
  Mitigation: refuse to bind both, and show the instance ID.
- **One cable unplugged blacks out every universe.** This is the
  availability cost of a global S0.
- **Apply is a sequence of steps, not atomic.** It stops at the failed step
  and reports per-row status.
- **Hot path:** each USB universe adds one 512-byte copy per tick and no
  allocation.

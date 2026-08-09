# DVC DMX Control Mapping Evidence

Updated: 2026-08-09
Scope: Daslight 5 `.dvc` `SHORTCUT TYPE="3"` import, runtime dispatch, visual Learn and persistence.

## Authoritative specimen

The read-only specimen is `C:\Users\kouty\Documents\Daslight 5\Projects\Panel.dvc`.
It contains nine `TYPE="3"` shortcuts. Each uses an input event such as
`/dmx/1/25:5`, action `TYPE="210"`, a profile UID plus zero-based feature
index in `TARGET`, primary-beam targets, and the verified scalar settings
`SMODE=1`, `CMODE=1`, `TMODE=0`, `MIN=0`, `MAX=1`, `LOOP=0`, `FLASH=0`,
`INV=0`.

The real-file golden fixes the complete result:

| Daslight input | Syndocal target |
|---|---|
| U1 Ch25 | fixture 1 `ColorRed` |
| U1 Ch26 | fixture 1 `ColorGreen` |
| U1 Ch27 | fixture 1 `ColorBlue` |
| U1 Ch36 | fixture 2 `ColorRed` |
| U1 Ch37 | fixture 2 `ColorGreen` |
| U1 Ch38 | fixture 2 `ColorBlue` |
| U1 Ch47 | fixture 3 `ColorBlue` |
| U1 Ch48 | fixture 3 `ColorRed` |
| U1 Ch49 | fixture 3 `ColorGreen` |

Every source range is restored as `0..65535`. Daslight's displayed universe is
normalized from one-based U1 to Syndocal's internal universe 0. The apparent
Blue/Red/Green order on the third fixture is retained from the embedded profile;
it is not reordered into an assumed RGB sequence.

## Fail-closed conversion contract

The importer converts only when all of the following are proven:

1. Shortcut type is exactly 3 and `EVENT DATA` is `/dmx/<universe>/<channel>:5`.
2. Universe and channel are valid; the Daslight universe is one-based.
3. Action type is exactly 210 and `TARGET` is `<profile UID>:<raw feature index>`.
4. The action contains a primary beam (`BEAMID=0`) whose fixture exists.
5. The target profile UID matches that fixture's embedded profile and the raw
   feature index resolves to an actual imported binding.
6. The scalar settings match the observed semantics and MIN/MAX are finite in
   `0..1`; verified `INV=1` reverses the range.

Unknown selector, action, setting, beam, profile or feature variants remain in
`Skipped`/`Unsupported`. The synthetic regression contains one valid mapping
plus deliberately unverified selector, action and mode variants so this rule
cannot silently become permissive.

## Runtime and operator workflow

- `DmxControlMapping` uses the complete typed OSC action surface, but retains
  its own universe/channel source identity so DMX and OSC assignments cannot
  collide.
- Setup > I/O explicitly selects either `Merge raw DMX` or `Control mappings`.
  The paths are mutually exclusive, preventing a mapped channel from also
  being merged into output as a duplicate command.
- Control dispatch is change-driven. A held non-zero button is not retriggered
  on every streaming packet; signal loss clears its history so the next valid
  stream can re-arm it.
- Persisted mappings always use zero-based universes. Art-Net's native universe
  is retained, while sACN's standards-defined one-based wire universe is
  normalized by subtracting one for Learn and Control Mapping only. Raw Merge
  keeps its existing protocol-native routing contract.
- Incoming values reuse the OSC value-to-action converter and the shared
  backend command dispatcher. Fixture attributes, Cue/Timeline transport,
  blackout, master, group, Effect, node graph and video controls therefore use
  the same validation and engine commands.
- The shared topbar has a third DMX Learn control. It colors the same eligible
  targets, selection does not execute the target, and the strongest changed
  input channel after a baseline replaces any existing mapping from that
  source. The active input is restarted immediately in Control mappings mode.

Mappings are preserved by normal `.sdc` Save/Open, browser Recovery, desktop
autosave/pre-update backup and `.sdctemplate`. Legacy files omit the field and
load an empty list without changing their serialized shape. Every restore path
that contains at least one mapping also restores Control Mapping mode, rather
than leaving the project silently running as Raw Merge.

## Reproducible evidence

```text
node app/scripts/check-dvc-dmx-shortcuts.mjs
  33/33 source-contract assertions

cargo test -p protocol --locked
  43 passed
cargo test -p io --locked
  108 passed, 1 physical-MIDI test ignored
cargo test -p syndocal --locked
  369 passed, 9 physical/long-running tests ignored

node app/scripts/check-viewport-containment.mjs --setup-io-only
  5/5 viewports passed
node app/scripts/check-viewport-containment.mjs --live-desk-header-only
  5/5 viewports passed, including real pointer-path DMX Learn
node app/scripts/check-localization.mjs
  2964/2964 static Japanese labels
```

The I/O suite includes a real UDP loopback test: two ArtDMX frames carrying
U1 Ch25 value 128 produce one `ColorRed=32896` control event, with no duplicate
event for the unchanged second frame. A separate protocol-numbering regression
fixes Art-Net U0 and sACN U1 to the same persisted mapping U0. The local Panel
golden imports all 9/9 mappings with zero skipped and zero unsupported entries.

## External acceptance boundary

No claim is made yet about a physical DMX input interface, electrical DMX line,
commercial Art-Net/sACN node, controller movement latency, packet loss on a
venue LAN or final lamp response. Those require the external rig. The parser,
UDP transport, mapping, engine dispatch, project persistence and operator UI are
software-verified independently of that hardware.

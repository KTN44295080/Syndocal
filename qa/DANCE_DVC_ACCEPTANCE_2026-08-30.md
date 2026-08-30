# dance.dvc acceptance — 2026-08-30

## Source identity and scope

- Source: `C:\Users\kouty\Downloads\dance.dvc`
- Size: `268,702` bytes
- SHA-256: `33E1EAD49F59B511D6A812FD06F506A89FF23721AE6CD99A927FDC89877B6B65`
- Producer metadata: Daslight 5 build `25.0905.165.111`, file version `2`

This source is used as an external, read-only importer and Art-Net acceptance
fixture. The acceptance test must reject a different file identity, must not
write a project, `.sdc`, or `.dvc`, and must not enable a hardware route. It is
not a claim that every Daslight effect or laser semantic is packet-identical.

## Source inventory and import projection

The reviewed source contains 12 profiles, 46 fixtures, 15 fixture groups, 16
bank elements, 56 scenes, 31 FX definitions, two MIDI shortcuts, no DMX
shortcuts, and one Super Scene named `TIMELINE / New Scene`. The Super Scene
contains 18 lanes and 195 blocks: 194 lighting scene blocks plus one audio
block, ending at `201,090 ms`. Every lighting block resolves to a known source
scene.

The import projection is 12 profiles, 46 fixtures, 15 groups, 56 cues, and 16
cue lists. Fifty-four scenes contain static fixture data and 31 FX references
are retained. Four `Sidepar-B` mode `0x01` rows take the importer fallback and
remain visible as warnings rather than silently disappearing.

The single audio block refers to the unavailable source path
`/Users/nakazawa_mac/Downloads/ダンスデカダンスChevon Lyric Video.mp3`.
Consequently the import must report exactly one missing/relink-required media
item. This is the only known block-level source loss in the fixture.

## Patch projection

Daslight Universe 1 maps to Syndocal internal Universe 0 and Art-Net wire
Universe 0. The non-overlapping address projection is:

| DMX channels | Fixture family | Count / width |
| --- | --- | --- |
| 1–60 | Mega PAR | 12 × 5 |
| 61–76 | generic RGBA | 4 × 4 |
| 77–115 | Strongpoint | 3 × 13 |
| 116–127 | PinSpot | 2 × 6 |
| 128–147 | Saber Spot | 4 × 5 |
| 148–187 | Mini Moving Head | 4 × 10 |
| 188–223 | Stage Evolution | 4 × 9 |
| 224–226 | Encore | 3 × 1 |
| 227–430 | Mega Bar RGBA | 6 × 34 |
| 431–498 | F3200A laser | 2 × 34 |
| 499 | Smoke | 1 × 1 |
| 500 | unpatched | must remain 0 |
| 501–512 | Wristband | 1 × 12 |

The imported route remains disabled and its preview remains all-zero until an
operator acquires output authority and explicitly enables it.

## Executable acceptance gate

The opt-in test is keyed by `KDMX_DVC_ACCEPTANCE_PATH` and the exact source
hash. It independently decodes the reviewed big-endian fixture payload for the
static `color/all_white` scene, builds the expected 512-byte Universe 0 frame,
and asserts channel 500 is zero. It then publishes the imported scene through
an ephemeral localhost Art-Net route and requires one exact 530-byte ArtDmx
datagram with OpCode `0x5000`, wire Universe 0, length 512, and a payload equal
to that independent expected frame. The receive loop is bounded by a total
deadline and packet count; oversized datagrams and a stream of nonmatching
packets must fail closed.

The default test remains ignored so a normal test run never depends on a file
outside the checkout. Final pass counts and exact MSVC evidence are recorded in
the current release checkpoint after independent review.

## Deliberately unaccepted boundaries

- The missing audio must be relinked before its block can play.
- Chaser types 321 and 324 remain approximations. Type 325 is deterministic,
  but has not been proven packet-identical to Daslight.
- Full dynamic parity for every FX, movement, dimmer curve, and laser channel is
  not established by the static `all_white` frame.
- Physical fixture output and a sustained 40–44 fps show run remain hardware
  acceptance tasks; the localhost loopback proves Syndocal's packet projection,
  not fixture behavior.

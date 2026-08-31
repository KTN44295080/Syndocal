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

## 2026-08-31 alpha.43 native observation

The exact source was imported again through the maximized native
`1.2.0-alpha.43` window. The native report showed 46 fixtures, 12 profiles, 15
fixture groups, 56 cues, 194 scene blocks, one audio clip, and exactly one
missing audio file. In EDIT / LIGHTING, `TIMELINE / New Scene` `#8.1` was
visibly classified as `TIMELINE` and exposed the `TL` open action. The separate
`ber / New Scene` `#15.2` remained `STATIC`. This is direct native evidence
that the raw DVC import does not collapse the Super Scene to Static.

The intermediate project saved before the alpha.43 build at
`target/qa/dance-dvc-import-20260831.sdc` is not an accepted project artifact.
It is 1,442,222 bytes with SHA-256
`A56DBA4FE15A60B47D6CD6AEE495587980C6ECE46D703D07BF3E5C51D654500F`,
but alpha.43 project-open bootstrap rejected it with
`project snapshot reference integrity: Cue 4 references missing lighting effect 1`.
Do not use or overwrite that file as the show project. The raw DVC import and
the intermediate SDC reopen are different boundaries; the latter remains an
explicit persistence/integrity defect to diagnose.

## 2026-08-31 alpha.44 source save/reopen repair

The defect was a protocol-validator contract mismatch, not effect loss during
serialization. All 29 imported targets retain complete cue-owned `params`, so
they do not require duplicate global effect summaries. Alpha.44 accepts that
single-authority form while still rejecting zero effect IDs, missing global
definitions when `params` is absent, duplicate effect IDs within one Cue, and
invalid fixture, group, attribute, or video-layer references.

Under the exact pinned MSVC 14.44 linker, the protocol suite passed `214/214`
and the default-feature hash-pinned external regression passed `1/1`. That
regression performs raw import -> Engine normalization -> persistence snapshot
-> canonical Save JSON roundtrip -> fresh current-schema Engine reopen and
retains 18 lanes, 194 Lighting events, one audio clip, `201090 ms`, 29
cue-owned effect targets, no global effect summaries, and a nonempty Timeline
bank. The wider DVC suite passed `109`, failed `0`, ignored `3`; first-party
warnings were `0`. Independent Terra xHigh review is GO with no P0/P1.

Native alpha.44 build/reopen remains pending while the operator is actively
using the current unsaved alpha.43 import. The rejected alpha.43 evidence file
remains preserved and must not be overwritten.

## Deliberately unaccepted boundaries

- The missing audio must be relinked before its block can play.
- Source-level save/reopen is accepted by the exact alpha.44 regression;
  native alpha.44 save/reopen remains pending.
- Chaser types 321 and 324 remain approximations. Type 325 is deterministic,
  but has not been proven packet-identical to Daslight.
- Full dynamic parity for every FX, movement, dimmer curve, and laser channel is
  not established by the static `all_white` frame.
- Physical fixture output and a sustained 40–44 fps show run remain hardware
  acceptance tasks; the localhost loopback proves Syndocal's packet projection,
  not fixture behavior.

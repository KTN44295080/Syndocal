# 960 sound waves strongpoint Mapping segment contract

## Authoritative input

- The channel tables in the hardware-manual photos supplied by the user for this fixture.
- The user's physical-fixture clarification that 13-channel mode is four segments in one horizontal row and that even 121-channel mode remains four segments wide.
- The user's front-view physical-order clarification: numbering starts at the bottom of the leftmost column, runs upward, then restarts at the bottom of the next column to the right.
- The supplied tables describe 60/61-channel modes as 20 controllable groups and 120/121-channel modes as 40 controllable groups. With the verified four-cell width, those modes extend vertically rather than widening the fixture.

The unrelated `GP-120-RGB` guide is not a source for this contract and is not retained in the workspace.

## Mapping contract

| DMX mode | Controllable cells | Mapping grid |
| --- | ---: | ---: |
| 3 / 4 / 8ch | 1 | 1 x 1 |
| 12 / 13ch | 4 | 4 x 1 |
| 24 / 25ch | 8 | 4 x 2 |
| 60 / 61ch | 20 | 4 x 5 |
| 120 / 121ch | 40 | 4 x 10 |

Cell order is column-major from the front view: start at the bottom of the leftmost column, run upward, then continue at the bottom of the next column to the right. In 121ch mode, segments 1/10/11/40 therefore map to left-bottom / left-top / second-column-bottom / right-top. The same order drives glyph color cells and per-cell beam origins. The one-row 12/13ch modes consequently remain the visually expected left-to-right sequence.

## DVC compatibility repair

Daslight-imported 13ch controls use an unsuffixed first RGB triplet followed by suffixes 2, 3, and 4. The first triplet is segment 1, not a global color. Regression coverage proves that it produces four independent cells and does not leak into cells 2-4.

## Verification boundary

The focused checker proves catalog channel grouping, DVC-style naming, grid dimensions, the user-confirmed physical cell order, and shared glyph/beam placement logic. Automated tests cannot independently re-observe the physical LEDs; no unrelated public PDF is used as a substitute for the user's fixture evidence.

The existing multi-segment browser gate continues to lock generic bars to a single horizontal row. Its zoom assertion uses the ratio from the stage's initial auto-fit zoom (currently 1.1x in the focused fixture) instead of assuming that auto-fit always resolves to exactly 1.0x.

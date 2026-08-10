# Daslight DVC symmetric Chaser parity evidence

Date: 2026-08-11

## Source under test

- Installed binary: `C:\Daslight 5\Daslight 5\Daslight 5.exe`
- Product version: `5.0.6.2`
- File version: `25.0905.165.111`
- SHA-256: `325D83EC54D41305D60B466B486EFE413B8F3E2656B45A544277C0CE9B87AE2A`
- Chaser #3: `CChaserType3Effect`, evaluator `0x1403775F0`
- Chaser #4: `CChaserType4Effect`, evaluator `0x140378400`

The formulas below come from static disassembly of that pinned executable and are version-specific.
The name/ID/default/schema cross-check comes from a separately authored real saved project.

## Recovered topology

Let the ordered saved beam list have length `N`, `M=ceil(N/2)`, and pair index `p` in
`0..M`. An odd list retains its centre beam once.

- Chaser #3 produces outside-in pairs:
  `pair3(p) = {p, N-1-p}` with the duplicate centre removed.
- Chaser #4 produces the exact reverse order, centre-out:
  `pair4(p) = pair3(M-1-p)`.

For `One Way Only=true`, the evaluator advances cyclically through the `M` pair groups. For
`One Way Only=false`, it traverses the same pair order in both directions. `Nb pixels on`
selects a contiguous window of pair groups, and `Fading` linearly transfers the leading and
trailing boundary weights. The disassembly evenizes odd `N` internally and subtracts the parity
bit from the mirrored index; that is why the centre is written once rather than paired with a
manufactured cell.

The evaluator reads the same three registered properties as Chaser #1:

| PARAM | Type | UI name | Default | Factory domain |
|---:|---:|---|---:|---:|
| 10 | 2 | One Way Only | 1 | boolean |
| 11 | 2 | Fading | 0 | boolean |
| 12 | 0 | Nb pixels on | 1 | integer 0..1000 |

## Corrected Syndocal representation

Import converts the saved linear beam order once into ordered paired `ChaserStep` values. Type 3
stores the outside-in order; type 4 stores its reverse. The existing production Chaser evaluator
then supplies Forward or Bounce traversal, contiguous active-step width, and optional continuous
overlap. No second evaluator or persisted legacy-mode switch was added.

This intentionally removes the recovered 40 ms work-frame grid and its duplicated reversal-frame
artefacts. Authored `DURATION` remains the full generator period: Forward divides it by `M`, while
Bounce divides it by the production bounce path length `2*M-2`. The report classifies both routes
as `SyndocalCorrected`, names the recovered evaluator address, and records the pair topology.
`Nb pixels on=0` remains a valid saved no-op only when the rack has no beams; a populated target
cannot represent an always-empty Chaser in the current non-zero-duty runtime and therefore stays
fail-closed. Values above the available pair count are clamped to that count and reported.

## Real saved specimen

- File: `qa/specimens/ChaserCatalog-323-324.dvc`
- Daslight build: `25.0905.165.111`
- Chaser #3: ID 323, `DURATION=5000`, One Way=1, Fading=0, pixels=1.
- Chaser #4: ID 324, `DURATION=5000`, One Way=0, Fading=1, pixels=1.
- Both effects use the same concrete mega-bar Dimmer PRESET
  (`SSLCHANNEL=33`, `SSLPRESET=0`, `MIN=0`, `MAX=1`) and the same ordered 32-beam selection.

The golden importer regression asserts 16 paired steps per effect. Chaser #3 begins with saved
selection indices `[0,31]` and ends `[15,16]`; Chaser #4 begins `[15,16]` and ends `[0,31]`.
It also fixes Forward/Bounce, fading overlap, derived 313/167 ms step durations, evaluator-address
reporting, and absence of a skipped target. A separate odd-count regression fixes
`[[0,4],[1,3],[2]]` versus `[[2],[1,3],[0,4]]`.

## Remaining evidence boundary

- The software path is normalized-DMX verified. No physical fixture is needed to establish the
  saved beam-pair order, but photometry and device latency are still physical acceptance items.
- The real specimen covers 32 beams and both direction/fading boolean branches at pixels=1.
  Multi-pixel behavior is statically recovered and exercises the already-tested production
  Chaser active-width evaluator; it has not been compared to a captured physical Daslight output
  trace on this binary.

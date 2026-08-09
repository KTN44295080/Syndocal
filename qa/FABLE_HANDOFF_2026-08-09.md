# Claude Fable planning handoff — 2026-08-09

## Working agreement

- Claude Fable owns the next implementation plan; Codex owns implementation, verification,
  evidence updates, commit and push.
- Stop at this checkpoint before beginning another feature tranche. The checkpoint is the
  commit that contains this file on `codex/syndocal-v1.0`.
- Treat `.dvc` behavior as evidence-led. Read the named saved specimen or installed-binary
  producer path before mapping it, and keep unproven variants `Skipped` / fail-closed.
- Preserve current UI contracts: do not generally shrink the application or replace the
  persistent layout with a different surface. Daslight desktop comparisons run maximized.
- Before every compile, build or test, stop only the exact checkout executable at
  `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`; never stop Daslight or an
  unrelated Syndocal checkout.
- Commit and push each verified, coherent tranche to `origin/codex/syndocal-v1.0`.

## Closed checkpoint: Daslight VALUE Sweep / DVC generator 625

Codex recovered the installed Daslight `CSweepEffect` evaluator, common VALUE constructor
and common serializer. The static producer path fixes generator 625 as:

1. `PARAM TYPE=4 ID=1` — grayscale palette / `COLORS`
2. `PARAM TYPE=6 ID=3 VAL=0|1` — Transform
3. `PARAM TYPE=2 ID=10 VAL=0|1` — Direction Change

The importer now recognizes family/type/generator `7/7/625`, preserves an empty BEAMS
source as a converted DMX no-op, preserves targeted fixture/beam/selection ordering, maps
Direction Change to `ColorEffectSpatialRecipe::Sweep`, and rejects wrong types, extra
parameters and unsupported Transform=1. ID621 now also enforces its exact property types.
`qa/DVC_SWEEP_SOURCE_PARITY.md` contains addresses, evaluator math and the deliberate
one-row/2D boundary.

## Verification fixed at handoff

- DVC filter: 57 passed, 0 failed.
- focused VALUE 621/625: 2 passed, 0 failed.
- locked workspace: 1,189 passed, 13 existing hardware/GPU/long-duration ignores,
  0 failed.
- DVC MIDI source: 39 assertions; DVC DMX source: 33 assertions.
- backend operator contract: 364 commands, 204 literal frontend calls,
  162 transactional mutations.
- localization: 3033/3033, zero bare user-data labels.
- Scene Settings Sweep authoring: 5/5 focused viewports, zero app/document scroll.
- production frontend build and `pnpm --dir app tauri build --no-bundle`: PASS.
- release executable: version 1.1.0, 39,068,160 bytes,
  SHA-256 `B40955BCDD65581230B25D91238449799CEE3C9DB3B98D5FB5B1FB239763F95A`.
- native Windows: exactly one full-screen Syndocal window; Setup -> Control round trip
  responsive; `ready=true`; Timeline surface present.

## Open evidence and acceptance boundaries for Fable to plan

These are intentionally not claimed complete by this checkpoint:

1. Capture a real saved targeted VALUE FX specimen and a real saved generator-625 golden.
2. Capture any real COLOR MAPPINGS saved specimen.
3. Capture a Move rack specimen with `BEAMID>0`.
4. Decide whether ID625 Transform=1 and arbitrary 2D MAPPINGS Sweep orientation merit a
   separate runtime tranche; the current one-row VALUE result is exact and Transform=1 is
   deliberately fail-closed.
5. External-only acceptance remains: physical fixtures, physical MIDI feedback, DMX
   node/electrical output, commercial visualizer/live LAN, macOS and Linux.

The larger persistent goal remains active. This is a clean implementation boundary for
Fable to turn the remaining evidence list into the next ordered plan; it is not a claim
that every physical or cross-platform acceptance item is complete.

## Fable disposition — 2026-08-09

1. Items 1-3 (real saved specimens) cannot be produced by an agent: saving inside the
   competitor app is prohibited by the supervision rules. They are converted into the
   user-executable capture sheet `qa/DVC_SPECIMEN_REQUESTS.md`. Until specimens exist,
   every dependent variant stays `Skipped`/fail-closed.
2. Item 4 decision: ID625 `Transform=1` and arbitrary 2D MAPPINGS Sweep orientation do
   **not** get a runtime tranche now. Grounds: an XML sweep of all three real projects
   (`Sin.dvc`, `Panel.dvc`, `Shinkan2026.dvc`) found no ID625 at all (only 321/223/10),
   so there is no real-world driver; fail-closed is exact and explicit. A bounded static
   check of the already-recovered common Transform post-process on the one-row raster is
   folded into DVC-V3 below; a real Transform=1 specimen (capture sheet, specimen 2)
   reopens the question with product evidence.
3. Next ordered tranche **DVC-V3 (Codex): VALUE FX catalog completion by static proof.**
   RTTI enumeration of the audited binary confirms classes for all remaining VALUE
   generators (`CBurstEffect`, `CPlasmaEffect`, `CKnightRiderEffect`, `CSparklesEffect`,
   `CRandomFillEffect`, `CPerlinEffect`), and `crates/protocol` already carries all eight
   runtime recipes. The tranche recovers factory ID mapping, property registration and
   evaluator parity per generator with the same method that proved ID625, then extends
   the `(7,7,ID)` importer arms for proven generators only. Anything unproven stays
   fail-closed; partial landing is acceptable.
4. Parallel Fable lane: F8 (Scene Matrix column density) re-measurement. T27-B fixed
   columns at 156px, which likely closes the audited ~1.7x deficit; measure live, then
   either document closure with numbers or implement compaction in the worktree lane.
5. T25-D was implemented on 2026-07-29 (`7f73533`) but the plan document was never
   marked; corrected in this checkpoint. No unimplemented T-series items remain.

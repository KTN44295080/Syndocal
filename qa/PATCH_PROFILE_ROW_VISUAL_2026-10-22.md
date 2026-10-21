# Patch profile tree row visual adjustment — 2026-10-22

- Scope: one local declaration in `app/src/styles.css`, `.patchProfileTreeBranchRow`; no component or global button changes.
- Cause: the 27px branch row inherited `button`'s `inset 0 -3px 0 var(--theme-tally-bar)` shadow.
- Change: override with `inset 0 -1px 0 var(--theme-line)`. Existing border, hover, focus outline, dimensions and handlers remain intact.

## Verification

- `node app/scripts/check-patch-profile-counts.mjs`: exit 0; 10 assertions passed.
- `git --no-optional-locks diff --check -- app/src/styles.css`: exit 0.
- Isolated Vite `?syndocalViewportFixture=patch`, headless Microsoft Edge, 1920x1080 and 1366x768: final browser check exit 0, no page errors. Browser and Vite server closed in finally.
- Both sizes: row height 27px, label 14px, chevron 13px, padding and 1px border unchanged; before/after row widths identical (285.03125px / 230px).
- Computed shadow changed from 3px RGB(86,97,112) to 1px RGB(42,50,61). Hover background/border change, 2px focus-visible outline, manufacturer/fixture expansion, visible mode children and keyboard collapse passed.
- Visual inspection: 1920x1080 before/normal and 1366x768 normal/focus PNGs. University rig, Personal rig and PAR / Wash are present in the verified tree.
- The first browser probe exited 1 because it incorrectly searched for PAR Wash beneath University rig. The corrected probe uses the actual University fixture and separately checks the PAR / Wash sibling; final evidence is from that successful run.

## Evidence and limits

- `target/qa/patch-profile-row-2026-10-22/browser-evidence.json`
- Same directory: `{1920x1080,1366x768}-{before,normal,hover,expanded,focus,full}.png`. Before images restore the original shadow only in browser memory.
- The supplied clipboard image under AppData/Local/Temp was outside the connector's allowed read roots and could not be opened. Review used the user's description and the actual frontend fixture DOM.
- No new test files, native build/app/device operations, workers, commit or push. Only the scoped CSS and this record were edited.
- Started from supplied main HEAD `1d012e7b`; final read observed main HEAD `cfcc20d5` after a concurrent update. No changes were made to Codex-owned files.

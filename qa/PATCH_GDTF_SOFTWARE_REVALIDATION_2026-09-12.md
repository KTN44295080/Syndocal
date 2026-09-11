# PATCH/GDTF software revalidation — 2026-09-12

This checkpoint revalidates the current `main` fixture catalog, profile/cache,
PATCH/Repair transaction, GDTF parser, and allocator rollback boundaries.
Product source was not changed by this checkpoint.

- Source under test: `main` at `558c575f1ca0da45b3ff31edb80b6fa6353ba344`.
- Native tests used `vcvars64.bat -vcvars_ver=14.44`, the exact Build Tools
  MSVC `14.44.35207` x64 linker, an absolute Cargo linker pin, and
  `where.exe link.exe` confirmed that linker first.
- Evidence logs: `target/qa/patch-gdtf-current-main-20260912/patch-gdtf-tests.log`
  and `target/qa/patch-gdtf-current-main-20260912/engine-tests.log`.

## Current-source results

Node checkers passed:

- `check:patch-transaction-d2`;
- `check:fixture-catalog` — 69 assertions;
- `check:patch-profile-counts` — 10 assertions;
- `check:profile-library` — 2,217 fixtures / 7,359 modes;
- `check:gdtf-profile-actions` — 15 context-bound actions;
- `check:fixture-live-color`;
- `check:fixture-limits-degrees`;
- `check:dmx-addressing`.

Focused native tests passed:

```text
app d2_:       9 passed; 0 failed; 0 ignored
app patch_:   30 passed; 0 failed; 0 ignored
app gdtf_:    10 passed; 0 failed; 0 ignored
engine fixture_patch_:          10 passed; 0 failed; 0 ignored
engine fixture_profile_repair_:  6 passed; 0 failed; 0 ignored
gdtf crate:   18 passed; 0 failed; 0 ignored; doc-tests 0 passed
```

The regression set includes pre-publication rejection, stale resolve/result,
reply-loss idempotency, cancellation fencing, conflict and footprint rejection,
allocator rollback/exhaustion, repair identity/layout rejection, publication
failure restoration, missing named mode, GDTF parsing, and malformed/unsafe
download/cache input handling. No physical DMX or fixture output was started.

## Remaining boundary

The software gate remains `In progress`: `R-PATCH-ATOMIC-001` is open for the
broader D-lane audit, and real repaired-profile fixture output, venue behavior,
and comparator parity are not proven here. `DEC-STANDALONE-001` remains an
out-of-scope boundary; no standalone-hardware programming was added.

`git diff --check`: PASS before commit.

## Current-main rerun — 2026-09-12

The same software gate was rerun against current `main` at
`aff411a7a97c8129368b257c17940091e38d4fa2`, with the documented MSVC
14.44.35207 x64 absolute linker pin confirmed first by `where.exe link.exe`.

All focused checks passed: the eight static/source contracts; Tauri `d2_`
9/9, `patch_` 30/30, and `gdtf_` 10/10; engine `fixture_patch_` 10/10 and
`fixture_profile_repair_` 6/6; and the gdtf crate 18/18 plus zero doc-test
failures. No physical DMX, fixture output, native UI, or visual comparator
acceptance was performed.

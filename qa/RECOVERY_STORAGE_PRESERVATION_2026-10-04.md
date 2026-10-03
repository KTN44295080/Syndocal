# Preserve invalid and unsupported browser recovery data

Branch `codex/showclock-review-20260912`, base `623281b1`.
Requirement: `MIGRATION-COMPATIBILITY-001`, O4 non-destructive old-format handling.

## Confirmed failure and behavior

The production browser recovery reader deleted its localStorage key when JSON
parsing failed. A future envelope returned `none`, and the next automatic save
overwrote it. The [baseline module reproduction](artifacts/recovery-preservation-before-2026-10-04.json)
loads the actual Git-base TypeScript module against isolated Map storage:
corrupt JSON disappears on read; future v4 survives read but is overwritten by
a successful save. It uses a complete authored project fixture, with no native
load, output activation, browser profile mutation or device interaction.

The reader now distinguishes absent storage from stored null/empty/invalid data.
It preserves exact bytes and rejects malformed or unsupported data with a fixed,
actionable message: restore a verified project backup, or use a compatible
Syndocal version for an unsupported format. Native exception strings, input
values, project content and paths are not reflected. The existing App startup
catch presents the error and offers no unverifiable recovery payload; App.tsx
was read but not edited.

Automatic checkpoint writes, recovery-intent registration and tombstone writes
first validate any existing recovery entry. If it cannot be read or recognized,
they return failure without replacing it. Recovery publication consequently
never enters its owner-install/native-invoke callbacks. Existing callers retain
their boolean/null failure contracts. Explicit discard still removes the key.
No quarantine copy, schema bump, silent migration, unsupported fallback or
replacement of the native authority serial/tag was introduced.

Legacy direct v1 checkpoints retain their serial-zero-only eligibility. Known
v3 checkpoint/tombstone/intent admission and later-serial retirement remain on
the same authority path. The helper uses one canonical recognition boundary;
the unchanged native project loader remains responsible for complete project
reference/value validation. This does not claim that the frontend envelope
parser validates every nested project field or editor draft.

## Evidence

- [Storage helper run](artifacts/recovery-preservation-storage-2026-10-04.txt):
  the existing complete helper suite passed. Seven new invalid-state variants
  cover malformed JSON, empty bytes, stored null, future v4, unsupported v2,
  invalid v3 checkpoint and negative tombstone serial. Each must raise an
  actionable read error, preserve exact source bytes, reject checkpoint,
  tombstone and intent writes, and never enter recovery publication callbacks.
- Legacy direct v1 is offered only at serial zero and is never rewritten by a
  read. Normal save emits canonical v3; the second save is byte-identical.
  Explicit discard and denied-storage read/write behavior are tested separately.
  The old deletion assertion changed because the required behavior is now
  preservation; rejection and byte comparisons were strengthened.
- [E3 production driver](artifacts/recovery-preservation-e3-2026-10-04.txt)
  and its imported deterministic project-authority checks passed. Existing
  intent/acknowledgement/publication/restart-state simulations remain valid.
  `check-project-open-bootstrap` also passed.
- [Ordinary optimized Windows build](artifacts/recovery-preservation-normal-build-2026-10-04.txt):
  `pnpm --dir app tauri build --no-bundle`, maintained MSVC 14.44.35207 pin and
  PATH-first checks. Only the exact checkout executable was stopped.
- [Passive native gate](artifacts/recovery-preservation-normal-window-2026-10-04.json):
  owned PID 49536, exactly one visible responsive maximized `Syndocal` window,
  unauthenticated broker read rejected. No primary debugger/authenticated
  mutation or recovery corruption was injected into the real user profile.
  Artifact SHA-256:
  `f3b8a847e83ed73c816d68cfdc94198784068548545fff4ed47e3a96f26b1172`.
- Measured ordinary Rust warnings and TypeScript diagnostics baseline/current/
  delta 0/0/0; existing Vite advisory 1/1/0. No warning allowance/limit changed.
  The Node helper/driver checks have no compiler-warning measurement.
- Owned staged diff, references and both ledger validators pass. All five
  protected file fingerprints remain unchanged. No Computer Use or subagent.

The production-module storage proof uses Map storage, not actual WebView2
localStorage or a physical UI action. The passive native gate proves build and
ordinary window/unauthenticated startup only; it does not establish the corrupt
recovery offer/error UI in that native window. O4 verified older-generation
fallback/reporting, whole backup/template/upgrade/restart matrix, bounded nested
parser fuzz, physical devices, independent review and release acceptance remain
open. Other-owner dirty frontend work is preserved and included in this local
build. Q4 records this bounded correction without changing requirement/risk
states or the 27 Complete / 23 Open / eight Deferred marker counts.

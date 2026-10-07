# Typed File audit outcomes — 2026-10-07

Branch codex/showclock-review-20260912; base 0f606b0a35361b6bd475e54c93b9b5490273a9ab.
The goal remains active and parent completion markers remain Open.

Backup deletion and deletion-journal management return typed receipts with no
publication phase. The previous File diagnostic projection classified both
successful receipts as invalid and could prevent ZIP publication after these
R5 mutations. Two regression cases reproduced that failure before the repair
(0 passed / 2 failed, Cargo exit 101); the failed evidence is retained.

project_file_audit.rs borrows the three native result types and validates their
operation/request identity and existing deletion/management receipt contracts.
Publication phases preserve success/failure/unknown. Valid deletion and journal
management receipts establish success; journal generations are exported.
Generic errors now leave the effect outcome unknown: deletion can commit before
terminal journal publication fails. This intentionally changes the earlier
generic-error false assertion to null; it does not change the returned error.
Invalid typed results block diagnostic publication without rewriting a possibly
committed result, retrying it, repairing storage or inventing an empty history.

File audit storage holds typed redacted rows at admission/termination. It no
longer serializes/copies full results, raw identities, paths or error bodies.
A non-Serialize 2 MiB result proves that audit does not serialize its body;
two retained rows use less than 2 KiB. Another fixture preserves a multi-MiB
private result/error for its caller while retaining less than 4 KiB for four
audit rows. Selected-page projection clones only bounded redacted rows.
The existing 65,536-row capacity/reserved-terminal, rate limit, request shape,
single-flight, immutable native dispatch, File grant and write-new policies
remain. Only process-local private audit representation changes: no stored
user-data migration, public command/schema/version change or ZIP-format bump.

Final pinned PATH-first MSVC 14.44.35207 units: 71 selected, 70 passed, 0 failed,
one intentional inherited-stderr subprocess helper ignored. The initial 69-pass
run and private build predate generic-error unknown semantics and are historical.
Both optimized final private and ordinary no-bundle builds exited 0.
The optimized private executable bff849011f97857b43c1c7a58bfe9527adb1b27d208f55da803e0ad148a36567 passed
57 actual native/stdio groups, including Save/Save As/template,
Tap callback feedback, backup create/inspect/list/delete, journal management,
restore, strict ingress/grants, receipt replay, revocation and lifecycle.
New diagnostic export groups verify the successful deletion/management rows,
archive hash, privacy, exact immutable export replay and unchanged project/output.

- external-diagnostic-export-after-backup_deleted-typed-success-redaction-replay-and-read-purity: 22888 bytes; 1366.42 ms for authenticated export, status, exact replay and state reads (not engine tick timing).
- external-diagnostic-export-after-deletion_journal_managed-typed-success-redaction-replay-and-read-purity: 22868 bytes; 1357.90 ms for authenticated export, status, exact replay and state reads (not engine tick timing).

The first native probe passed 54 groups including both new ZIP exports, but
failed its final project-delete revocation assertion: the first admission reply
was unknown rather than pending. That failed evidence is retained. The harness
now resolves unknown replies with at most four status reads of the original UUID,
never resends the mutation, and still requires observed pending before revocation.
The final probe uses new owned QA principals/UUIDs and the same optimized binary;
only the JS harness changed after the final Rust units/private build.
Its admission replies were pending directly (all admissionObservations empty);
the new unknown-reply reconciliation branch was source-reviewed, not exercised
as a successful recovery by this final native probe.

Ordinary executable 3239bf02014829a1d3d2e0278a796e1a078532154377e73a3b9eb9ffc8aa7075: one visible responsive maximized
Syndocal window, unauthenticated broker read rejected. Previous normal instance
closed gracefully only. No primary debugger, force termination, Computer Use,
subagents, physical output activation or new human-confirmation requirement.
All original private backup artifacts and existing deletion/management/retired
facts remain; this run adds only its own QA journal facts, so journal bytes are
not claimed unchanged. Five protected dirty files remain byte-identical/unstaged.
Frontend/native inventories remain 481/561 commands; broker adapter check 16
groups passed. Final Rust/TypeScript compiler warning baseline/current/delta
0/0/0; existing Vite chunk advisory 1/1/0, without threshold suppression.
Stable self-review is not independent review.

Current-source typed File audit no longer requires a publication phase on backup deletion or journal management receipts, retains no private result/error body, and leaves generic-error effect outcomes unknown. Real authenticated MCP exports after both successful mutations passed with digest, redaction, exact replay and project/output read purity. This current-source evidence does not establish full attempt fields, durable audit retention, crash/power loss, physical output, operator Control clicks, independent review or AI8/release acceptance.

Next action: continue the remaining canonical attempt-field and durable audit/
retention work without treating diagnostic publication as full acceptance.
Ledger remains 27 Complete / 23 Open / 8 Deferred.

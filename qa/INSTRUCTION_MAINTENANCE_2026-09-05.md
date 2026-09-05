# Instruction inventory and maintenance checkpoint — 2026-09-05

Scope: repository instructions only, authorized after the read-only audit. The
user additionally requested the AGENTS inventory and no future Terra use.
Branch: `codex/syndocal-v1.2`; source base:
`9ec650ea06372f417909da4826f29f028bbd8a8b`.
This file's Git commit identifies the completed documentation checkpoint.

## Source and decisions

Read Eric Provencher's
[Rethinking skills and prompts for GPT-6 Astra](https://x.com/pvncher/status/2095991462416490862)
in the browser, including the article body. Direct web fetching returned 403;
the browser displayed the full text. Applied its guidance on concise skill
descriptions, conditional references, task-sized validation, precise decision
boundaries and completion within authorized scope. This is prompting guidance,
not new permission to operate hardware or weaken product acceptance.

The installed `skill-creator` guidance was used for the skill edit. No global
skills, plugins, Codex settings or memories were changed. Model choice is
capability-based except the user's explicit exclusion of `gpt-5.6-terra`.

## AGENTS.md inventory

Every substantive group in the old AGENTS file is accounted for below. Old line
numbers refer to the source base above, not the rewritten file. "Remove" means
remove from standing instructions, not erase historical Git evidence.

| Old lines / rule | Disposition | Result and reason |
| --- | --- | --- |
| 3, 24-27: native build, launch and evidence | Consolidate | A change-class table separates docs, copy, presentation and native behavior. Interaction/runtime/IPC/persistence/output/native configuration keep native acceptance; browser proof is never promoted to native/hardware proof. |
| 4: exact-path process termination | Retain | Only the verified checkout release executable may be stopped before native release builds; Daslight and other checkouts remain protected. |
| 5-23: MSVC pin, hosted exception, delayed PATH expansion | Consolidate | Exact invariants moved intact to `WINDOWS_NATIVE_BUILD.md`, loaded for native build/test tasks. Existing wrapper/checker still owns enforcement. |
| 31-34: UI sizing | Retain | Named-element permission and reflow/disclosure/scrolling preference retained in two bullets. |
| 38: every periodic checkpoint updates multiple docs | Consolidate | A checkpoint is an agreed completed work unit; one relevant record suffices. A status message does not trigger document churn. |
| 39: commit and push | Retain / scope | Applies to validated owned checkpoint files, unless user says otherwise; read-only answers are exempt and unrelated dirty files remain protected. |
| 40: inventory/delete caches every checkpoint | Remove trigger / retain safety | Cleanup becomes separate maintenance. Exact targets, prior checkpoint publication, reviewed harness, dependency restoration and reclaimed-byte accounting remain in `BUILD_CACHE_CLEANUP.md`. |
| 44-46: bounded pause and exhaustive handoff | Consolidate | Explicit stop honored; detailed worktree/stash/artifact/agent inventory applies to relevant multi-lane pause work, not every minor edit. |
| 50-51: Windows ASIO | Retain | Default/license separation and all detailed acceptance remain under `ASIO_INPUT_ACCEPTANCE.md`; unrelated tasks are not blocked by open hardware rows. |
| 55-57: product and schema versions | Retain / clarify | Distributed artifacts/RCs advance, published bytes immutable; ordinary internal docs commits need no bump. Metadata synchronization and separate schema proof retained. |
| 61-63: warning ratchet | Retain / scope | Report actual configurations, no new warnings, zero before beta/RC, no debt suppression; docs checks do not claim compiler warnings measured at zero. |
| 67-69: evidenced compatibility and fail-closed | Retain | Invalid/stale/unsupported product state stays fail-closed; no permissive adapter/retry stack to hide broken invariants. |
| 70-72: clean break, migration and boundary proof | Retain / scope | Remove retired paths within the chosen boundary; needed migrations/fallbacks stay bounded and evidenced. Old/new/reason report applies to changed boundaries. |
| 76-81: Sol > Ox > Terra > Luna, model-gated integration | Remove / replace | Supervisor/implementer/independent reviewer roles replace model rankings. No Terra use per current user instruction. |
| 82, 85-86: independent review, ownership, supervision | Retain / consolidate | Explicit non-overlapping ownership and supervisor responsibility preserved; independent review required for material runtime/schema/safety/cross-cutting work. |
| 83-84: fill every lane and report idle capacity | Remove obligation | Delegate useful independent work; no forced extra tasks or idle-lane justification. |

## Other instruction surfaces

- `CLAUDE.md`: short entrypoint. Its already-historical long log is available at
  `9ec650ea06372f417909da4826f29f028bbd8a8b:CLAUDE.md`; do not read it routinely.
- `.claude/skills/tranche-cycle/SKILL.md`: precise trigger and short router to
  conditional comparison/viewport references. Removed fixed rescue APIs, fixed
  plugin paths, blanket all-matrix plus supervisor reruns, serde-default-only
  compatibility, shared-junction worktrees and mandatory Fable/Codex arbitration.
  Real comparison evidence, competitor-app protection, test-selection truth,
  process ownership and conditional known viewport hazards are retained.
- Completion flow: read-by-task entrypoint, single operating authority in
  AGENTS, rescinded-pause precedence repaired, old alpha.43/44 "current" train
  narrative removed from policy, alpha.25 cleanup heading marked historical.
  Long dated evidence sections remain in place to preserve existing references.
  Machine-consumed sections 6-9 and their acceptance claims are not changed.
- `UI_REDESIGN_PLAN.md` and `LIGHTING_SHOW_MODEL_V3.md`: historical role/approval
  notices prevent old workflow text overriding current AGENTS; domain contracts
  remain available without reopening completed tranches.
- Nine `qa/.codex-luna-*-prompt.txt` files: marked historical in place. Original
  bodies retained; no claim that each assignment completed, no renewed ownership.
- `.claude/launch.json`, `.claude/settings.local.json`, application code,
  compiler/build wrappers, acceptance ledgers and product metadata: unchanged.

## Validation and handoff

Documentation-only scope: no native build, product launch, hardware action,
version change, cache deletion or restoration. Compiler warning counts are
not measured; no first-party compiler-warning claim is made.

Pre-existing user-owned dirty file preserved:
`app/scripts/check-viewport-containment.mjs`, SHA-256
`3F9A46901AF267CF6DBA6509CF82CF3C44B28AF21C7EEAE8839F1D967340F063`.
Companion worktrees and both existing stashes are outside this task and untouched.

Validation completed on the owned documentation diff:

| Command / inspection | Result |
| --- | --- |
| `node app/scripts/check-completion-ledger.mjs` | Exit 0; 50 Open + 8 Deferred (58 total); sections 6=37, 7=6, 8=9, 9=6. |
| `node app/scripts/check-q1-q4-ledger.mjs` | Exit 0; 32 Q1 rows, 29/29 domains, 10/10 contracts, 58/58 markers, master mirror parity. |
| `node app/scripts/check-release-metadata.mjs` | Exit 0; product remains Syndocal 1.2.0-alpha.68. This is the metadata checker, not the full `check:release` aggregate. |
| `python -X utf8 C:/Users/kouty/.codex/skills/.system/skill-creator/scripts/quick_validate.py .claude/skills/tranche-cycle` | Exit 0; Skill is valid. Initial default-codepage invocation failed decoding Japanese; explicit UTF-8 invocation passed. |
| `git -c core.safecrlf=false diff --check -- AGENTS.md CLAUDE.md .claude/skills/tranche-cycle qa` | Exit 0. The per-command option avoids LF/CRLF conversion notices; no Git configuration was changed. |
| `python -X utf8 -` inline read-only comparison against `git show 9ec650ea06372f417909da4826f29f028bbd8a8b:<path>` | Acceptance sections 6-9 identical; all nine original prompt bodies identical after their new header; short-guidance Markdown targets exist; pre-existing viewport checker hash unchanged. |
| `git ls-remote origin refs/heads/codex/syndocal-v1.2` before commit | Remote still at the exact source base; no intervening remote change observed. |

Independent review of the stable diff returned GO, P0/P1/P2 = 0/0/0.
It exercised docs/copy/CSS/native-IPC/Cargo/ASIO/cleanup/pause/reviewer-selection
decisions against the new rules. The implementer and reviewer were independent
inherited-model sessions; no Terra session was launched. The later wording
clarification also excludes Terra investigation, consistent with the user's
unqualified preference.

LF-normalized entrypoint sizes: AGENTS 86 -> 64 lines (13,126 bytes -> about
9 KB); CLAUDE 402 -> 14 lines (161,673 -> 606 bytes); tranche skill 167 -> 26
lines (12,565 -> 1,607 bytes). Conditional references are separate and are not
required for ordinary small edits. The completion flow remains a long evidence
record (4,119 -> 3,970 lines), but no longer requires full reading or duplicates
the operating workflow.

Publication procedure for this checkpoint: stage only the owned documentation
paths; inspect `git diff --cached --name-only` and
`git -c core.safecrlf=false diff --cached --check`; commit, then
`git push origin HEAD:refs/heads/codex/syndocal-v1.2` and compare `git rev-parse
HEAD`, `git rev-parse '@{upstream}'` and `git ls-remote origin
refs/heads/codex/syndocal-v1.2`. The exact resulting commit is supplied by Git
and the final response, not a recursive hash-only documentation commit.

Remaining boundary: this work changes repository instructions only. App/global
plugin skills and previously injected instructions in existing sessions are
outside these files; no product, native, hardware or whole-matrix acceptance
is advanced. The user-owned viewport checker still needs its own continuation
with ownership confirmed first. Old task prompts are preserved, not resumed.
The stop boundary is this cleanup; no next product tranche is started.

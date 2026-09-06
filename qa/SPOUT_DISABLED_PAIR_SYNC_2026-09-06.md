# Disabled show Spout pair synchronization

Branch: `codex/syndocal-v1.2`; base: `e10990c32c4ed9400206870c0d7a44fd81e84975`.

## Observed failure and change

The user reported `OutputControl rejected (publication_failed)` when starting
Spout output. A read-only native MCP capture showed desired role Both,
effective Standby, ownership Failed, and both fixed show outputs disabled.
The underlying error was `show Spout sender must be enabled`; the same
validation error also prevented generic external-video cleanup.

Generic synchronization had applied the strict activation validator before
reaching its worker registry. A fully disabled, otherwise valid show pair is
now admitted to generic synchronization with its original disabled plans so
stale routes can retire. Validation uses private cloned descriptors solely to
check every non-activation invariant; these clones never reach the engine or
driver. Mixed, malformed, partial and conflicting pairs remain rejected.
An active strict pair remains excluded from the generic Spout driver.

The policy is extracted from `main.rs` into `show_spout_generic_sync.rs`, with
separate tests. No output lease, authority, publication or physical activation
guard is bypassed. There is no per-frame work added to video rendering.

## Evidence and remaining acceptance

Generated evidence is under `target/qa/spout-disabled-pair-20260906/`.
`before-runtime.json` contains the read-only native failure capture. The real
autosave was copied there before rebuilding; the original show is untouched.

Independent source review accepted the policy and tests: the actual generic
transport runtime with a fake driver retires existing sender routes without
new activation, preserves input/NDI plans, rejects invalid mappings,
backreferences, extra senders and mixed enabled states, and leaves authored
data unchanged.

Focused Windows Cargo checks using the pinned MSVC 14.44 linker passed:
`show_spout_generic_sync` (3 passed, 0 failed/ignored) and
`show_spout_outputs::tests` (12 passed, 0 failed/ignored). Logs are
`focused-tests.log` and `strict-pair-tests.log`; no compiler warnings.

The preserved autosave also has a separate `.sdc` export, retaining its exact
project content and control mappings. `preserve-autosave.mjs` and
`autosave-export.json` record the source and export; the standard project-open
path must still validate it. No original project was overwritten.

`pnpm --dir app tauri build --no-bundle` passed (native release 2m27s,
Vite 8.05s); compiler/Vite warnings baseline 0, current 0, delta 0. The wrapper
stopped only PID 6456 at this checkout's exact release executable path.
The new executable SHA256 is
`05E6C6CEC24B6B705465007E59F11A79547D8AE4B55F24065716BA707339A5B6`.
Launch evidence confirms PID 102516, one responsive maximized Syndocal main
window. The standard single-instance project-open route loaded the preserved
copy. Read-only MCP then reported 46 fixtures, both named outputs retained,
Timeline paused, ownership Ready/Standby with no error and outputs disarmed.
Logs: `native-build.log`, `native-launch.json`, `after-runtime.json`.

Original Unity show and the preexisting dirty viewport checker retain their
recorded SHA256 hashes. No Unity settings or original project files changed.
Successful user-operated Spout start and Unity reception remain separate from
the software regression and responsive-window evidence; the user has been
asked to repeat Spout start in the now-open preserved copy.

## Retest: activation still blocked

The user reported `OutputControl rejected (invalid_request); nothing was
applied` at Spout start. `retest-runtime.json` now
shows Ready/Both, both domains allowed, no ownership error, unchanged E1/R0,
and both show outputs still disabled. Thus the generic synchronization failure
is corrected, but this is not successful Spout activation acceptance.

Further source tracing found active-only validation in the Spout activation
preflight, derivation, final confirmation, and engine publication. Enabling
these stored disabled descriptors also changes persisted project content;
the previous Spout receipt contract incorrectly assumes an unchanged project
fence for every successful activation. The follow-up must align strict
activation validation, engine publication, managed authority, and canonical
project receipt convergence before claiming the user path works.

## Activation correction

The activation classifier now accepts an exact fully disabled pair and enables
it in place. Mixed enabled states, malformed mappings, wrong composition
backreferences and extra senders remain rejected. Engine restoration preserves
the authored pair and fades; only a newly added pair is removed on compensation.

The managed exact-Both authorization route now includes Spout. Project commit
preflight prepares the hash, checked counters and ordinary Undo entry before
engine/native publication. The engine image is verified before the assignment-only
coordinator commit. Unverified cleanup or post-callback durable failure fences
all project/output command admission. Successful receipts preserve output/S0
identity and use either the unchanged project fence or its exact persisted
successor. The frontend converges canonical project authority before refreshing.

Spout enable now participates in the existing public durable terminal journal;
Reset remains excluded. Journal tests prove write/reload/exact response lookup
and invalid-terminal rejection. Runtime ordering before mutable-fence checking
was source-reviewed. These tests do not prove native redispatch behavior or
atomic recovery across the separate private/public terminal writes.

Independent frontend, native and protocol/runtime reviews accepted the stable
diff. Focused checks passed:

- Frontend output-control and authority-convergence checkers; TypeScript.
- Engine Spout coverage: 8 tests, plus 2 restore rejection/idempotence tests.
- Protocol control-plane command suite: 18 tests.
- Final Windows native `show_spout` suite: 49 tests, including real-engine
  project commit/Undo preservation, rollback hash verification, managed
  authorization and durable journal replay. All selected tests ran with
  zero failures or ignored tests; no compiler warnings in these final logs.

Logs: `activation-engine-tests.log`, `activation-engine-restore-tests.log`,
`activation-protocol-tests.log`, `activation-native-final-tests.log`.
Earlier compile diagnostics were corrected before the final passing run.

The latest autosave `backup-1788668815818.json` is separately preserved and
exported as `DSF2026-before-activation-fix.sdc` with its project and control
mappings intact. It includes later runtime cue state than the first backup.
The original Unity show and protected dirty viewport checker still match their
recorded hashes.

`pnpm --dir app tauri build --no-bundle` passed (release 3m52s; Vite 8.77s).
Compiler/Vite warnings for the checked Windows configuration: baseline 0,
current 0, delta 0. The wrapper stopped only this checkout's PID 102516.
Executable SHA256:
`0326104031AFA2DE91B18F34E762129F5BBCF50F7B5A450BA45BB02E3B710A30`.
Launch evidence records PID 55780 and one responsive maximized Syndocal main
window. Standard project-open forwarding loaded the latest preserved copy;
read-only MCP confirmed 46 fixtures, both named outputs with original IDs and
dimensions, paused Timeline, Ready/Standby with no ownership error, and both
outputs retained/disarmed. Logs: `activation-native-build.log`,
`activation-native-launch.json`, `activation-after-runtime.json`.

## User acceptance

After the activation fix at `fa21f20`, the user replied "通りました" to the
request to start Spout and check the Foreground/Background reception in Unity.
The reported Spout-start rejection is resolved by user acceptance. This does
not establish frame cadence, Video BO restoration/isolation, or crash recovery
between the private/public terminal writes.

A subsequent read-only MCP attempt could not verify its descriptor's process;
the process inventory confirmed that Syndocal was no longer running. No output
mutation or restart was performed during this acceptance update. Video BO
active-output checks remain separate and require a running test session.

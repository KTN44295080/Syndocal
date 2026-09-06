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

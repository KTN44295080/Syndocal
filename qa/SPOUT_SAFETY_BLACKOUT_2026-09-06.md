# Spout safety blackout lifecycle

Base `9d64ef9`, branch `codex/syndocal-v1.2`. The user accepted Unity video
playback as resolved and requested continuation. This tranche addresses the
previously recorded direct safety BlackOut boundary, not further video tuning.
Preserve unrelated `app/scripts/check-viewport-containment.mjs` and Unity files.

## Failure and intended boundary

A published strict Spout pair pins its activation safety epoch/generation.
Normal S0 engage/release therefore invalidates the worker, which can retire the
pair and leave release or project replacement blocked by a retained fault.
Additionally, no-Follow rendering lacks the explicit S0 hard-black branch, and
keepalive collapses transient revocation and SDK failure into strings.

Activation must still require clear S0. Published senders may survive normal S0
cycles only with fresh per-send authority checks and hard-black materialization
when either current safety authority is engaged or the snapshot remains black.
This covers the interval between synchronous S0 reservation and engine snapshot
publication. Old visible frames must never be reused across that change.
Initial render/keepalive revalidation classifies safety changes only after
ownership and the complete bound output match. Its content token may also have
changed; this is not proof that S0 was the token's only cause. Published worker
project/ownership/pair validation remains strict. Existing normal-send token and
Follow revocation/resampling behavior is preserved. SDK errors are never
reclassified using a later safety observation.

## Evidence and pending work

Root ran `node app/scripts/check-output-control-runtime.mjs`: PASS. Existing
frontend release already uses the canonical single local Both lease and retains
backend fence validation; no frontend change is currently required.

## Implementation and review

Published workers stop pinning the activation S0 identity; initial activation
still requires its exact clear proof. Both Follow and no-Follow Spout rendering
materialize black from current safety or snapshot blackout. Typed revalidation
allows a normal safety transition to discard old frames without retiring the
pair. Keepalive preserves nested revoke/SDK error types.

Independent review identified a final enqueue race after earlier safety reads.
The corrected live and keepalive paths re-read token and safety immediately
after sender-name/control validation and before `send_image`. The shared fence
also checks safety after its token read. The residual single-load-to-SDK window
remains: the engine exposes no atomic lease across an external SDK call.
Independent final production review ACCEPT, including the keepalive wrapper.

Exact-MSVC wrapper command `cargo test -p syndocal --locked spout_safety --
--nocapture --test-threads=1`: 5 PASS, 1709 filtered, warnings 0, compilation
1m39s. Evidence `target/qa/snapshot-cleanup-20260905/spout-safety-tests-final.log`.
An earlier test-only ambiguous `.into()` failed compilation and was corrected;
that failed attempt is not gate evidence.

Tests cover stale-token S0 reservation with zero SDK calls, black generation
before snapshot catch-up, release waiting for both states, repeated engage/
release authority cycles, stale-frame discard, SDK error retention, and
simultaneous output removal/ownership loss. They use the real engine and fake
output definitions; they do not exercise a full strict worker's published
validator/keepalive loop. That native user acceptance remains pending.

Final wrapper invocation `cargo test -p syndocal --locked spout_transport::tests
-- --nocapture --test-threads=1`: 59 PASS, 5 ignored, 1650 filtered, warnings 0,
compilation 40.29s. The ignored cases require real Spout/GPU/external-application
operation; no acceptance is claimed for them. Evidence:
`spout-safety-transport-regression.log` in the same directory.

Earlier NDI invocations selected an incorrect module filter (zero tests) or
failed process startup because the runtime DLL was absent from PATH; none are
passing test evidence. The final invocation used the installed
`C:/Program Files/NDI/NDI 6 Runtime/v6` on the child PATH, then the same exact-MSVC
wrapper with `cargo test -p syndocal --locked --features ndi ndi_safety_blackout
-- --nocapture --test-threads=1`: 1 PASS, 1717 filtered, warnings 0. Confirmed log:
`spout-safety-ndi-confirmed.log`. No DLL was installed or copied into the product.

## Native integration checkpoint

`pnpm --dir app tauri build --no-bundle` PASS, release compilation 2m31s,
frontend build 9.20s, first-party warnings baseline 0 / current 0 / delta 0.
The maintained wrapper verified and stopped only checkout PID83812, then printed
the exact MSVC 14.44.35207 pin and PATH-first match. Log:
`target/qa/snapshot-cleanup-20260905/spout-safety-native-build.log`.

Launched exact checkout PID103936; one responsive maximized Syndocal main window.
Executable SHA256:
`EE32AE76F4AAC40E424BAE6ECD58BE299E6881C86AEF2D0C8679D29222746A52`.
Evidence `spout-safety-launch.json` in the same directory. Spout timing is no
longer enabled. Temporary localhost CDP38479 remains for read-only acceptance
inspection and should be removed by normal restart after acceptance.

The user was asked to open the original Unity4K project, enable Spout, and toggle
DMX BlackOut ON/OFF during playback and while stopped. Error-free restoration of
Unity video and lighting during playback, plus retained sender/output state,
remain user-driven acceptance. Do not mark the S0 issue closed from unit tests.
Managed project Open/New acceptance remains separate. No physical output or UI
action was automated for the tests or launch verification.

## Follow-up: target isolation

The user accepted S0 restoration, then reported DMX BO also blanking video.
[TARGET_BLACKOUT_2026-09-06.md](TARGET_BLACKOUT_2026-09-06.md) records the subsequent
separation of authored DMX/VID/ALL from emergency S0. Its video predicate supersedes
the effective-DMX snapshot predicate described at this checkpoint.

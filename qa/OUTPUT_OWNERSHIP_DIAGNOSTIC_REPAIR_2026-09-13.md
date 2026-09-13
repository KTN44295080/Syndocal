# Output ownership diagnostic attribution repair — 2026-09-13

## Identity and scope

- Branch: `codex/showclock-review-20260912`.
- Base: `0836b965` (H4 Edit checkpoint); working tree was clean on entry.
- Owned implementation: `app/scripts/check-output-ownership.mjs` only.
- This repairs the known static-checker blocker carried by the H4 checkpoint.
  It changes no native/runtime output behavior, protocol, UI, or product version.

## Failure and repair

The unchanged baseline failed `check:output-ownership` with:

```text
get_external_video_transport_status: native physical sink is outside the exact legacy/R4 ingress inventory (ndi_transport, spout_transport)
```

The command passes immutable transport references to
`external_video_transport_status_for`. The inspected helper chain collects
transport/input-fault and ownership status; it does not start or stop output.
The checker nevertheless treated the two owner identifiers alone as physical
output sinks.

The repair recognizes only the complete, whitespace-normalized, existing
read-only command delegation. Only its two transport-identifier hits are
suppressed. The command name is not globally allowlisted and no broad sink
marker, legacy rejection, R4/S0 classification, or external-adapter check was
removed. A changed delegate, argument, command name, or additional statement
loses this exemption and remains subject to the original sink inventory.

Seven new negative source fixtures prove rejection of an inserted NDI start,
Spout stop, Spout worker harvest, safety-blackout call, unknown delegate,
mutable transport borrow, and renamed unclassified query. Existing negative
fixtures for missing inventory entries, new output sinks, and late rejection
continue to run. These are source fixtures, not executed device operations.

This remains a bounded source-contract checker, not a complete Rust parser or
transitive call-graph/effect analysis. Future changes inside delegated helpers
still require review and their applicable runtime tests.

Independent read-only review approved this bounded checker repair. Its in-memory
probe confirmed that inserting an output command in the wrapper is rejected,
whereas changing only the delegated helper is outside the inventory's scan.
The reviewer found no output mutation in the current diagnostic helper chain;
the acceptance claim is limited to the exact wrapper and its negative fixtures.

## Verification

| Command | Observed result |
| --- | --- |
| `pnpm.cmd --dir app run check:output-ownership` | PASS, including all seven new negative fixtures. Baseline failed before the edit. |
| `pnpm.cmd --dir app run check:output-control-runtime` | PASS, including Standby Sync output-lease UI contract. |
| `pnpm.cmd --dir app run check:safety-blackout-runtime` | PASS. |
| `pnpm.cmd --dir app run check:release:static` | Entire aggregate PASS, exit 0. |
| `node app/scripts/check-audio-output-panel.mjs` | PASS, 46 assertions. |
| `node app/scripts/check-audio-output-control.mjs` | PASS, 82 checks plus 65 runtime-regression assertions against mocks. |
| `pnpm.cmd --dir app run check:cue-live-modifier` | PASS. |
| `pnpm.cmd --dir app run check:cue-effect-recall` | PASS. |
| `git diff --check` | PASS. Git reported its LF-to-CRLF working-copy advisory. |

Raw aggregate output is retained at
`target/qa/output-ownership-h5-20260913/release-static.log` (ignored local QA
evidence, not a distributed artifact). No Cargo/native build was run for this
checker-only change, so there is no new native compiler-warning measurement.

## H5 continuation boundary

`UI-H5-CONTROL-001` remains **Open**; no completion marker or Q1/Q4 mirror is
closed by this repair. Counts remain **23 Open + 8 Deferred + 27 Complete**.

Source inspection found concrete H5 work, rather than merely missing evidence:

- `app/src/App.tsx` `setLightingMaster` and `setGroupSubmaster` currently report
  that lease-bound OutputControl actions are unavailable and make no mutation.
  Restore these through the canonical lease-bound authority, never by enabling
  the retired raw output commands.
- The Control domain navigation currently exposes only Lighting and Video;
  the H5 Both emphasis is absent.
- `check-control-upper-workspaces-browser.mjs` largely targets the internally
  named `control` workspace, which is the user-facing Edit workspace. Its
  success must not be substituted for H5 Control live-workflow proof.

The requested `check:touch` launch was blocked by the platform safety check
before execution. It was not retried through an alternate tool or command.
Consequently no new browser, native-window, physical-output, device, or H5
interaction acceptance is claimed. Chat-specific plan updates were also
refused for missing exact chat identity, while repository reads, edits, and
the recorded terminal checks succeeded.

Next bounded work: implement and independently review the missing lease-bound
master/submaster operation, then the H5 Control presentation/interaction
contract. Run the actual Control browser and Windows native gates only when
the authorized execution surface permits them. Hardware and venue acceptance
remain separate.

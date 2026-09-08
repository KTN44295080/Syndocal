# Owner-bound thumbnail cancellation — 2026-09-08

Base: `8a8e530c6801a5399f0d71c93dcead343a35ad0a`.
Branch: `chatgpt/macos-artifact-validation`.
Status: bounded source candidate, not complete MEDIA-DERIVED or release acceptance.
ChatGPT implemented and tested the change directly. The user permitted Luna when
needed; one read-only Codex CLI / gpt-5.6-luna review was used for independent review.
The reviewer did not edit, build, commit, publish, or operate physical devices.

## Change and ownership

The existing latest-batch controller now aborts its active AbortSignal on source
replacement, project reset and disposal while retaining the original promise until
settlement. The native request client listens to a per-request Tauri Channel.
After admission, native code sends a schema-v1 ticket containing the lane and a
random 128-bit request ID. An abort preceding that announcement sends cancellation
once the ticket arrives; repeated/late announcements cannot retarget another job.

The gate binds the ticket to the implicit Tauri caller window. Cancel accepts only
that exact window, lane and active ID, and sets the existing cooperative flag.
It does not release admission, terminate another job, mutate the project, or claim
worker completion. Queued/running ownership survives cancellation until worker return.
The two lanes stay independent and bounded. No pending native-ticket table, timeout
thread, retry loop or rendering-loop work was added. Randomness uses existing
getrandom; there are no new package dependencies.

## Compatibility and safety boundaries

Both existing thumbnail commands now require a `started` Channel; the old argument
shape has no fallback. The native-frame result and product-data schemas are unchanged.
`cancel_native_thumbnail_request_v1` is one new finite frontend command (456 -> 457).
The 47 reviewed MCP operations are not expanded. App only constructs the Channel and
classifies exact-ticket cancellation with its existing terminal recovery commands,
allowing cleanup during Full Lock while retaining the owner-registration barrier.
Windows ASIO/NDI fail-closed paths and the fixed three-screen UI are unchanged.

A proposal for a separate raw terminal transport was denied by the tool and was
not applied. Therefore App teardown can still unregister the frontend owner barrier
before the asynchronous cancellation call is dispatched. In that case cancellation
fails, the old result is discarded and the worker stays owned until completion;
end-to-end cancellation after App/window teardown is NOT accepted by this checkpoint.
No authorization bypass or disabled guard is used to conceal that limitation.

A running synchronous libav, OS I/O or GPU operation remains interruptible only at
its existing cooperative checkpoints. No hard stop/whole-request deadline, actual
GPU/video cancellation latency, or process-tree hard-kill proof is claimed.

## Independent review

Luna's read-only review found no logic blocker in the supplied cancellation core.
It identified a release evidence gap: a real WebView-to-native Channel/cancellation
round-trip for both lanes is not established by mock frontend tests. That release
blocker remains open. Additional native tests exercise real Tauri Channel serialization
and send failure, plus concurrent-lane isolation; they are not a WebView round-trip.
The later App terminal-command classification is self-reviewed, not independently
re-reviewed. The source candidate is not advertised as fully independently accepted.

## Native artifact observation

The maintained no-bundle Windows build exited 0. Its optimized Rust stage finished
in 4m39s. The native test and build logs contain zero Rust warning diagnostics;
a pre-change warning baseline for this configuration was not measured.

Artifact: `target/release/syndocal.exe`, version `1.2.0-alpha.69`, 64,696,832 bytes.
SHA-256: `A47AD5D8E589C03D04C86E2E7ECA2508BF3571DECB25D0E7851D5D880D898EE5`.
At `2026-09-08T01:30:39.1745865Z`, PID 26140 had one responsive, maximized Syndocal
window for that exact executable. The launch check exited 0, issued no physical-output
command, and stopped its own process. The follow-up found zero remaining processes
for that executable. This is window-launch evidence, not thumbnail GUI acceptance.

Evidence is in `target/qa/thumbnail-ui-cancel-20260908/`, including native test logs,
`native-build-01.log`, `native-launch-01.json`, `native-launch-01.log`, and the read-only
review input/result. Existing Mac work matches all seven starting file hashes.
This candidate is not a signed, tagged or published release and is not promoted to main.

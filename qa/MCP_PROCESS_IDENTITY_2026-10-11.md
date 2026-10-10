# MCP OS process identity — 2026-10-11

Branch: `codex/native-output-guards-20261010`.
Base: `76ea9e7ff31e568a54a716080d0d357a5f877c16`.
Internal repair; product version remains `1.2.0-alpha.71` and delivered files are unchanged.

## Findings and compatibility

The sidecar inspected every non-Windows PID through Linux `/proc`, which does
not provide the macOS process path. It also lowercased canonical paths on every
OS, allowing distinct case-only Linux executable paths to compare equal.

Move OS introspection into one focused module. Windows retains its fixed hidden
PowerShell query, deadline and output bound; Linux retains its exact kernel
`/proc/<pid>/exe` read. Only Windows folds case. Linux/macOS require exact
canonical paths and reject ambiguous spellings instead of accepting substitutes.
Unsupported platforms fail closed before credentials or broker dispatch.

On macOS the caller-selected executable is launched with one fixed read-only
`--syndocal-process-path-v1 <PID>` argument pair. Its strict early CLI runs before
Engine/Tauri initialization and uses Apple's kernel-backed
[proc_pidpath implementation](https://github.com/apple-oss-distributions/xnu/blob/main/libsyscall/wrappers/libproc/libproc.c).
The bounded 4096-byte path buffer follows
[PROC_PIDPATHINFO_MAXSIZE](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info.h).
The response requires exact version/PID/path fields, valid UTF-8, an absolute
non-NUL path and bounded output. Invalid/misplaced flags reject rather than
falling through to ordinary app startup. No argv/label fallback, retry, cached
identity, UI startup, Engine worker or implicit output activation is introduced.
The per-request macOS helper is a short-lived process, not a frame/output worker.
Existing HMAC nonce, pairing, grants, revocation, lease and generation checks remain.

Older macOS builds without the CLI reject inspection. An explicit absolute
descriptor path and actual executable spelling are required; no legacy fallback
or claim about the already-delivered alpha.71 MCP path is made. Tool discovery's
operation count now comes from the existing reviewed set rather than stale copy.
No broker protocol, project schema or operation allowlist was changed.

## Validation

- [Node identity tests](artifacts/mcp-process-identity-20261011/node-identity-tests.tap): six passed on Windows, one Linux-only native case-spelling
  regression skipped. Malformed/oversized/non-UTF-8/wrong-PID responses, invalid
  PIDs, unavailable platforms, fixed spawn bounds and uncached proof are covered.
- [Windows Rust CLI tests](artifacts/mcp-process-identity-20261011/syndocal-process-identity-tests-20261011.txt): two passed, none ignored, with exact MSVC pin/PATH-first
  verification. Strict parsing and unsupported-platform rejection are exercised.
- [AI5 sidecar checks](artifacts/mcp-process-identity-20261011/syndocal-process-identity-sidecar-20261011.txt) passed: 16 adapter groups/128 hostile frames, HTTP/REST/WS,
  Origin/Host rejection, bounded session retention and nine request-capacity
  groups, including 10,000 intents and 9,992 pre-dispatch rejections. The broker
  is fake loopback; Windows OS inspection runs against actual test processes.
- [Windows normal no-bundle build](artifacts/mcp-process-identity-20261011/syndocal-process-identity-normal-build-20261011.txt)
  passed with exact MSVC verification, zero Rust warnings and the unchanged one
  Vite large-chunk warning. Baseline/current/delta: Rust 0/0/0, Vite 1/1/0.
- [Native CLI rejection](artifacts/mcp-process-identity-20261011/native-cli-rejection.json):
  unsupported, invalid and misplaced inspection requests all exited 1 without
  identity stdout or a surviving Syndocal process.
- [Normal native window](artifacts/mcp-process-identity-20261011/normal-native-window.json):
  one visible, responsive, maximized exact-path window; unauthenticated reads
  rejected. The owned candidate closed gracefully.
- [Final release aggregate](artifacts/mcp-process-identity-20261011/syndocal-process-identity-release-20261011.txt)
  passed with exit 0, including the latest identity/AI5 checks. Ledger remains
  23 Open / 8 Deferred / 27 Complete. macOS hosted results remain pending until
  this source revision is pushed and the dedicated job runs.

The dedicated macOS CI compiles the exact production Rust module into an owned
temporary peer, requires zero compiler warnings, tests kernel self-path and
reaped-PID rejection, and exercises descriptor admission/HMAC forwarding against
a fake loopback broker. It deliberately does not build/package the Tauri app or
claim Mac UI, physical DMX, clean install, signing or venue acceptance.

No subagents, Computer Use or independent review were used. The primary
checkout's unrelated dirty work remains protected. Ledger acceptance stays unchanged.

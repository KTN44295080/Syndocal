# Syphon remote acceptance kit checkpoint

Base: `main` at `80831ffd6d3b9398f2e19b6f2ec97395d535679c`.

## Delivered behavior

- Native Syphon output records bounded diagnostics at construction and retirement:
  OS/Metal identity, server name, successful publications, latest send latency,
  dimensions, errors, and the observed output ownership status. Successful frame
  processing only updates counters; it does not write diagnostic files.
- Native teardown/failure fencing remains authoritative. A recorded native close
  is not proof of the parent's worker join, ownership settlement, or reception.
- The media-free project has RGB corner labels, an animated gold circle, and one 1920x1080
  `Syndocal-Syphon-Test` output. Lighting and DMX routes are disabled.
  Its source is animated Noise: static Solid generators intentionally retain
  PTS zero, which freezes the shader's `TIME`. The inherited Solid fixture failed
  the actual two-frame motion assertion. The source was corrected; RGB corner
  and motion assertions remain unchanged, and no generator/runtime policy changed.
- The bilingual procedure, scoped Gatekeeper helper, and collection script are
  under `tools/distribution/`. Collection checks schema, exact CI
  commit/version, recency, named route, publication and retirement. Missing or
  stale evidence never becomes physical acceptance. No whole profile is copied.
- After existing app/DMG/PKG acceptance, the manual macOS workflow packages both
  installers, the project, instructions, collection tools, and SHA-256 manifests.
  It also runs native published-texture RGBA/orientation readback, JXA collection
  tests, the complete native test suite, and warnings-as-errors compilation.
- Syphon input remains `NotBuilt`. There is no protocol, project schema or IPC
  change. Windows Display rendering is unchanged.
- The Syphon UI virtual-element checker now loads the actual NDI attribution
  component added on main; existing Syphon assertions are preserved.

## Evidence and remaining gates

Independent review found no blocking source issues in native lifecycle,
diagnostics, collection, fixture, or packaging. Local evidence is retained in
`target/qa/syphon-handoff-20261102/` and the isolated build logs under
`%LOCALAPPDATA%/SyndocalDev/syphon-handoff-20261102/logs/`.

- Node artifact/workflow/kit suite: 173 passed, 5 macOS-only tests skipped on
  Windows. Syphon UI and NDI contracts, both shell syntax checks, and owned Rust
  formatting (`skip_children=true`) passed. No compiler warning measurement is
  implied by these script checks.
- Windows native regression suite: 2350 passed, 25 intentionally ignored,
  0 failed after correcting the fixture; Syphon-focused suite: 31 passed.
  Both used the pinned MSVC 14.44.35207 linker with PATH-first verification and
  hash-verified FFmpeg/NDI payloads in isolated resource/test directories.
  Current Windows compilation emitted 0 warnings. A fresh unchanged-base warning
  count was not measured, so a baseline-to-candidate warning delta is not claimed.
- Hosted macOS compile, Metal readback, JXA, app/DMG/PKG and kit generation:
  pending until the exact checkpoint workflow completes. The first run
  `41408678752` built Syphon.framework successfully, then failed because
  `lipo -verify_arch` consumed the trailing binary path as an architecture.
  Preparation now puts the input binary before `-verify_arch`; shell syntax and
  all 12 workflow contract tests pass. The repaired Mac run remains required.
- External Simple Client reception, visible orientation/motion/blackout,
  discovery retirement, and the friend's returned evidence remain physical gates.
- 2026-11-05: product metadata advanced to `1.2.0-alpha.83` for the friend
  distribution; the kit is taken from the macOS installer run on that commit.
  No tag, release publication or message to the friend is part of this checkpoint.
- Integrated C5/30-minute Windows Display performance acceptance passed at
  `2eea47b5` (fixture `c5/fixture-3l-groupfx.sdc`). This Syphon checkpoint
  still does not establish Syphon reception or hardware acceptance.

The older [Syphon implementation handoff](SYPHON_OUTPUT_2026-10-20.md) retains
the detailed output-runtime and broader Mac acceptance boundaries; its historical
statements about unavailable output-authoring UI are superseded by main's
canonical authoring routes.

## 2026-11-05 Kit 2: Syphon input checkpoint

Owner decision (2026-11-05): Syphon input enters 1.2.0. Product metadata is
`1.2.0-alpha.84` for the Kit 2 distribution.

- Native bridge adds bounded server discovery (`SyphonServerDirectory`, at
  most 64 servers, name/app/uuid only), `SyphonMetalClient` creation by exact
  UUID with an unambiguous name+app fallback after a publisher restart,
  BGRA8/RGBA8 readback into a reused shared staging buffer, and stop/release
  that keeps the retained owner on failure.
- Runtime: one owner thread per input layer, demand-driven (the decoder only
  pops a one-slot queue and unparks; no polling thread, no lock or wait on the
  render tick). A missing or vanished server is an actionable source fault
  and recovers when the publisher returns. Retirement keeps the JoinHandle
  until native close succeeds.
- The selected server identity is stored as bounded JSON in the existing
  `source.name` field; no project-schema change. Malformed or legacy
  free-form names are rejected with a re-select instruction.
- Contract change: the Syphon backend detail and
  `external_video_input_backend_route_state` no longer report input as
  `NotBuilt` on macOS builds with `feature = "syphon"`; other platforms
  still reject Syphon input with a specific reason.
- Diagnostics schema 2 adds per-input records (selected server, frames
  received, dimensions, latest receive latency, errors, close outcome);
  the collector checks them, and the guide adds a Simple Server input step
  with `screenshot3.png`.

Evidence (Windows, isolated copy, pinned MSVC 14.44.35207): full
`cargo test -p syndocal --bin syndocal` 2359 passed / 0 failed / 25 ignored,
0 compiler warnings; `video` Syphon availability test passed; `tsc`,
`check:release`, Syphon input/output UI contracts, frontend routing and the
friend-kit node suite passed. Implementation by Codex (Sol) until its usage
limit, then completed and reviewed by Claude.

Pending: hosted macOS compile with `-D warnings`, the in-process
server→client loopback test (dimensions, channel order, orientation,
release), app/DMG/PKG and Kit 2 generation; then the friend's Simple Server
input observation. Kit 2 also repeats every Kit 1 output step.

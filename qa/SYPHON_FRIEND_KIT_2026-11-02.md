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
- This is a SHA-scoped internal alpha.82 CI candidate. Before intentionally
  distributing a development artifact to the friend, advance the prerelease
  ordinal and synchronize/validate release metadata under the completion flow.
  No tag, release publication or message to the friend is part of this checkpoint.
- Integrated C5/30-minute Windows Display performance acceptance is a separate
  remaining gate after the recent group/extended-ISF changes. This Syphon
  checkpoint does not establish 60fps or hardware acceptance.

The older [Syphon implementation handoff](SYPHON_OUTPUT_2026-10-20.md) retains
the detailed output-runtime and broader Mac acceptance boundaries; its historical
statements about unavailable output-authoring UI are superseded by main's
canonical authoring routes.

# Alpha.72 development distribution candidate

Branch `codex/distribution-alpha72-20261011`, base `abb2ab296214b8a85c7197c57df654f94e06129f`.
The primary OneDrive checkout and distributed alpha.71 files are preserved.

Alpha.72 collects the internal native Video blackout, output-route admission,
Windows recording publication/Stop and macOS MCP process-identity repairs since
alpha.71. The prerelease ordinal advances because this is a new distribution
candidate. Product version changes do not change project or API schemas.

The final-DMG validator now executes the extracted product's read-only process
inspection CLI against the live validator PID and compares the case-sensitive
real executable path. PID zero and a misplaced reserved flag must exit 1, with
no identity on stdout, within ten seconds and 8192 bytes per stream. This proves
the packaged app contains the CLI; the previous source-module fixture alone did
not prove that. It does not establish native GUI responsiveness or physical output.

Artifact acceptance reports advance independently from schema 1 to 2. Schema 1,
missing process-identity checks, incomplete rejection evidence and mismatched
executable identities are rejected. Historical reports remain unchanged; only
the next package's upload accepts the new report. Local validator tests passed
169/169, including strict negative cases. No compiler-warning measurement applies
to these JavaScript tests.

The release aggregate passed with exit 0. Windows no-bundle build passed with
the exact MSVC 14.44.35207 PATH-first linker in 4m05s. The exact executable
provided one visible, responsive, maximized Syndocal window and rejected an
unauthenticated broker read; it was closed gracefully. First-party Rust warnings
were 0; Vite chunk advisories were baseline 1 / current 1 / delta 0.
Evidence is in [artifacts/distribution-alpha72](artifacts/distribution-alpha72).

Windows installer payload/ZIP and hosted macOS final-DMG validation are pending.
This candidate is unsigned development distribution, not full Recording,
hardware, clean-machine, signing, notarization or release acceptance.
Independent review is not claimed because the user prohibits subagents.

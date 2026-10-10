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

Windows installer payload/ZIP acceptance passed. Hosted macOS final-DMG
validation is pending in [run 38087771147](https://github.com/KTN44295080/Syndocal/actions/runs/38087771147)
against product source `80fccf1401de2c2d8dc5c4ea2f03e9b96348d5eb`.
This candidate is unsigned development distribution, not full Recording,
hardware, clean-machine, signing, notarization or release acceptance.
Independent review is not claimed because the user prohibits subagents.

## Windows delivery

NSIS extraction and MSI administrative extraction passed the maintained two-root
validator: exact executable version/SHA-256, seven pinned FFmpeg DLLs, four common
resources and ASIO exclusion. Six NSIS installer-only plugins were inventoried
separately. No product installer was executed. The final executable SHA-256 is
`e479a9e43de095c17c3347f20061ec5a0d7c13944d6db2b8c44671b3df672a65`.
Its own normal-profile window also passed responsiveness, visibility,
maximization and unauthenticated-read rejection, then closed gracefully.

The actual delivery helper passed six process/registry-mocked lifecycle cases.
The final ZIP was re-extracted and all seven files matched byte-for-byte; its
helper confirmed the exact installer hash and valid Microsoft prerequisite
signature with `-ValidateOnly`. No prerequisite or product installation ran.
The version-specific helper source is in `tools/distribution/alpha72`.

Delivery folder: `C:\Users\janua\Downloads\Syndocal-1.2.0-alpha.72`.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Windows-x64-install.zip | 100903032 | `f655c596261dbab476f462dc826f1ec700ad249631116d8ee8075aa588599edd` |
| x64-setup.exe | 82422097 | `515fcad0240c54a400a7150f364f0f91a47c170d9a3bf7449d25e89c42761acb` |
| x64_ja-JP.msi | 123170816 | `7e8d41b445a078eb544e6f3a3d495de7cd0ca0f435ebf29b815ab8a6cee1931d` |

All names start with `Syndocal_1.2.0-alpha.72_`. Both installer payloads and the
ZIP have independent JSON evidence in this checkpoint's artifact directory.
Rust compiler warnings remain 0; Vite chunk advisories are baseline 1 / current
1 in each native build / delta 0. Tauri reconstructed its third-party NSIS cache,
as in alpha.71, and verified tooling hashes.
USB-DMX detection/transport sources are unchanged from alpha.71; the friend’s
alpha.71 hardware test remains relevant to that path.

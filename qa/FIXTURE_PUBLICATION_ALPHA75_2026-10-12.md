# Fixture publication lane lifetime — alpha.75 development packages

Branch `codex/fixture-publication-alpha75-20261012`, base `44d4c86a`.
The primary dirty checkout, alpha.72 and already-delivered Windows alpha.74
artifacts are protected. Historical alpha.74 source, native, browser, package
and Mac evidence is in [its checkpoint](FIXTURE_CONTROL_ALPHA74_2026-10-12.md).

Full Linux CI `38182052232` passed 1107 engine tests and found one native
contract failure among 1672: `set_fixture_patches` was async and could outlive
its outer renderer admission guard. The handler now remains synchronous through
the existing bounded engine publication acknowledgement. It uses the canonical
atomic patch implementation directly, without a detached publication worker.
The two-second bound, exact ticket owner/epoch, all-or-nothing validation and
Undo/Redo policy remain. No acceptance assertion is weakened.

Distributed alpha.74 is immutable, so all current product metadata, MSI upgrade
version, build scripts and filenames advance to alpha.75 / MSI 1.2.75. The
installer-lifecycle workflow now requires an explicit successful source run;
its retired alpha.73 push trigger and stale implicit run ID are removed.

Focused Windows native regressions passed 54 distinct cases, including the
previously failing synchronous-handler contract, real bulk Undo/Redo, fader
history and admission/inventory checks. The complete release aggregate passed.
The exact MSVC 14.44 no-bundle build passed in 6m12s; first-party Rust compiler
warnings observed were 0. Vite retained the existing single chunk advisory
(baseline 1, current 1, delta 0). The exact normal executable passed one visible,
responsive, maximized Syndocal window and unauthenticated broker rejection,
then closed gracefully. No unrelated instance was terminated.

Private backend acceptance, Win/Mac packages, full CI and actual hosted installer
lifecycle have passed at the checkpoints recorded below. USB timing/signal continuity, Mac native GUI/macOS 12 execution,
signing/notarization, general multi-universe route editing, optional integration
installation and full release/venue acceptance remain unobserved or incomplete.

The private alpha.75 release executable built from `b71aefa9` passed all 32
backend checks, including 60 fader samples with intermediate/final engine
readback, strict replay/invalid rejection, atomic two-fixture movement with
one Undo/Redo, DMX/VID BO, restart, ownership retirement and credential cleanup.
All physical outputs remained disarmed. Fader queue acceptance averaged 2.681 ms,
maximum 4.470 ms; this is not physical USB latency or a Control DOM measurement.
The normal profile was unchanged by this isolated proof.

Final local Windows NSIS/MSI payload checks passed, including exact executable
hash/version, seven pinned runtime DLLs and approved resource contents. The
extracted NSIS executable passed the same responsive, maximized single-window
gate. MSI ProductVersion is 1.2.75 and its major-upgrade table is present. The
13-file ZIP was re-extracted, all hashes matched, its real Install helper passed
ValidateOnly, and all eight prerequisite/install seam cases passed. These checks
did not install/uninstall the product on the user host.

Mac run `38197136731` passed for exact source `b71aefa9`: 83 unique selected Rust
tests, five cleanup tests, 169 artifact-validator tests and all 11 final DMG
checks. Mac default native-test compiler diagnostics retained the existing
first-party baseline/current 28/28 (delta 0); warning header multisets match.
Seven vendor/dependency warning diagnostics also remain. This is not a zero-
warning Mac release configuration. Both packages are available for user testing following the full Windows CI
and hosted lifecycle results recorded below.

Windows full CI `38197136505` passed its warning ratchets but stopped after
2053 native tests passed and one existing ShowClock test read an intermediate
`held_until_locked` scheduler state. The test broke its observation wait as soon
as the receipt count increased. A test-only follow-up now asserts the initial
clock lock and waits for the exact admitted/scheduled/disarmed-gate outcome.
The original three-second acquisition and one-second observation bounds remain;
all final assertions are preserved. Seven focused ShowClock tests passed.
Only code inside `show_clock_ipc.rs` `cfg(test)` changes; prepared native products
remain built from `b71aefa9`. The final full CI revalidated this test-only source delta.

The next full Windows CI run `38200983796` passed the corrected ShowClock test
but found a separate existing retirement test race: the worker publishes its
failed-send flag before its thread terminates. The runtime correctly retains
that still-running worker. The test now observes JoinHandle completion before
requesting terminal reap; both receiver-drop and empty-retirement assertions
remain. All 86 media-audio tests passed.

A local full native run then found a restoration fixture whose enabled Dimmer
automation immediately changes 0 to 65535 at t=0. The restore comparison now
uses a fixture whose initial automation keyframe agrees with its authored value,
retaining enabled automation, all definitions, complete snapshot/mapping equality
and exact original-backup-byte checks. Existing paused seek automation behavior
is unchanged. The three restoration contracts passed. A complete native rerun
passed 2054 / 0 / 21 (passed / failed / ignored); this test-only follow-up changes
no production behavior or prepared alpha.75 installer bytes.

## Final software and distribution checkpoint

Cross-platform run `38205849972` succeeded at source `70cc1e8`: all workspace
Rust tests, release checks, packaging and smoke checks passed on Win/Linux.
Windows native tests passed 2054 / 0 / 21; Linux native tests passed
1672 / 0 / 12; both engine suites passed 1107 / 0 / 14. Ignored tests remain
ignored and establish no device acceptance. The hosted Windows native-release
warning ratchet measured first-party baseline/current 0/0, delta 0.

Hosted lifecycle run `38210335064` passed all 44 recorded checks using actual
CI-built NSIS/MSI: alpha.72 to alpha.75 update preserved both product data roots
and left one registered installation. Complete removal deleted the executable,
registration, both data roots, three test credentials and installer preferences;
the external project survived. This used a disposable hosted runner, never the
user installation. Artifact source is `70cc1e8`, baseline source `80fccf14`.

Prepared local Win75 and hosted Mac75 products were built from `b71aefa9`.
The only three product-input differences through `70cc1e8` are inside
`cfg(test)`; [source equivalence](artifacts/fixture-publication-alpha75/validation-source-equivalence.json)
records their exact inclusions. The hosted installer hashes differ from the
local delivery installer hashes. Lifecycle results prove the same production
source and installer logic, not binary equality or execution of the delivered
local installer bytes. Local delivered payloads have their own extraction,
hash, runtime-file, helper and native-window proof.

The files in `C:\Users\janua\Downloads\Syndocal-1.2.0-alpha.75` are now
available for user testing; previous alpha.72/74 bytes remain unchanged.

| Artifact | SHA-256 |
| --- | --- |
| Windows install ZIP | `3822cd3604781373fe683eebfbe2e2fcbc2db302a5479f150862fdbc4a4f2e78` |
| Local NSIS | `f21dca05c9fca5202b15883e75fd0c27e7ee6c864ca8bf863af702aaf845837f` |
| Local MSI | `ba9679bf3aeb99b6e9eea4cc3825eaa551eb82f0f1d55c83df5055949170ae51` |
| Apple Silicon DMG | `3eb1b75a5f2ce561926084293b39a8704bd552f11ca5909f1b075ec4a48a3d81` |

Evidence is under `qa/artifacts/fixture-publication-alpha75/`. Windows ZIP uses
`Install-Windows.cmd`; full removal uses `Uninstall-Windows.cmd`. Mac DMG has
`Remove-Syndocal.command`. Update and complete removal preserve externally
saved `.sdc` and media; installer archives and shared runtimes are outside
product-data cleanup. This is an unsigned development distribution. Physical
USB-DMX signal timing/continuity, native Mac GUI and macOS 12 execution, signing/
notarization, general multi-universe route editing, optional integration
installation and broader ASIO/release/venue gates remain incomplete.

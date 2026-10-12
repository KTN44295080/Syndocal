# Fixture publication lane lifetime — alpha.75 candidate

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
lifecycle are pending. USB timing/signal continuity, Mac native GUI/macOS 12 execution,
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
warning Mac release configuration. Both package files are prepared but alpha.75
has not been delivered while full Windows CI and hosted lifecycle are pending.

Windows full CI `38197136505` passed its warning ratchets but stopped after
2053 native tests passed and one existing ShowClock test read an intermediate
`held_until_locked` scheduler state. The test broke its observation wait as soon
as the receipt count increased. A test-only follow-up now asserts the initial
clock lock and waits for the exact admitted/scheduled/disarmed-gate outcome.
The original three-second acquisition and one-second observation bounds remain;
all final assertions are preserved. Seven focused ShowClock tests passed.
Only code inside `show_clock_ipc.rs` `cfg(test)` changes; prepared native products
remain built from `b71aefa9`. Full CI will revalidate this test-only source delta.

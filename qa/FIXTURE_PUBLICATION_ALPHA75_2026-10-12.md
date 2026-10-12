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

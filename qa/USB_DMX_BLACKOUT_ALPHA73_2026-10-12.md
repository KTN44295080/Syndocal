# USB-DMX blackout and restart identity repair — alpha.73 candidate

Branch `codex/usb-dmx-blackout-alpha73-20261012`, base
`9da61808706236c290d2f974fcbf07928ef65618`.
The primary OneDrive checkout and distributed alpha.71/72 artifacts are preserved.

The user reported `OutputControl rejected (invalid_request)` when enabling USB-DMX
and releasing Safe BO on Windows and macOS. The renderer restarted its numeric
output counter at 1, while the canonical lease journal retains request high-water
and managed terminal identities across restart. Those requests can collide before
publication; unchanged retry does not repair their identity.

The renderer now allocates each fresh output/lease request through a local native
identity preparation command. The shared atomic sequence advances past both the
durable journal and live registry high-water. Invalid window, poisoned state and
JavaScript-safe integer exhaustion reject without a renderer fallback. Reply-loss
resends retain the exact originally allocated request. Allocation grants no output
authority: existing owner/project/generation/lease/replay checks still authorize
each mutation. Native inventory advances 561 to 562, frontend 481 to 482, with
one local R0 identity-preparation source and no added external output operation.

Healthy USB-DMX startup now retains the original clear or engaged S0 authority.
A bounded, scoped physical zero-write hold lasts through the initial worker zero
receipt, independent of the global blackout latch. The UI no longer engages or
releases global S0 as a USB startup stage. An existing BO remains engaged; clear
BO permits subsequent live frames. Faulted recovery still requires both logical
and physical S0 latches and proof that the old worker joined. Initial-zero failure
latches S0 and advances its generation, fencing releases authorized before that
fault. No physical-device result is inferred from the fake-port worker tests.

The existing joined-fault regression now obtains the fresh authority after its
injected initial-zero fault, because that new fault intentionally invalidates the
previous generation. UI contracts replace the former required global engage/release
sequence with preservation of the original BO state; containment and font sizing
are unchanged.

This candidate also includes the previously isolated project-retirement drain fix.
The parent Windows CI E3 recovery failed because an already-admitted frame could
remain in flight after platform-worker teardown. Retirement now retains its exact
reservation while waiting on the existing condition variable, bounded by the
unchanged three-second transition budget. Timeout fences output; superseding fault
identity/error is preserved. No polling thread, frame copy or production frame-path
allocation is added. Deterministic tests cover drain, expiry and superseding fault.

The reused native QA profile exposed another real failure at 128 durable receipts:
ordinary eviction removed a canonical lease receipt while its completed public
physical response remained, so the next journal publication failed validation.
`output_lease_receipt_retention.rs` now bounds that pair together. Pending physical
barriers and the two fixed one-shot probe domains stay protected; the newest
committing receipt cannot be discarded. A bounded retired-physical origin guard
prevents old operations becoming fresh after eviction, including after restart
and subsequent ordinary traffic. Journal schema advances independently from 2
to 3 with a one-way validated migration; legacy 1 retains its existing conservative
pending-probe rejection. Future 4 and mislabeled version-2 physical guards are
rejected. Earlier executables reject schema 3; product downgrade is unsupported.

Windows MSI previously fixed `1.2.0.70`: its fourth field does not advance Windows
Installer's three-field upgrade ordering. The current MSI is `1.2.73`; the bounded
stage/ordinal mapping preserves alpha < beta < RC < stable < next patch. The
bundle disallows downgrades. NSIS opt-in data removal preflights all paths before
its ordinary deletion and then removes this product's OS credentials and owned
temporary files. Junctions and symlinks are rejected. Full removal helpers support
Windows NSIS/MSI and Mac validated bundles; external projects/media and shared
Microsoft/WebView2 runtimes remain. Mac DMG extraction verifies the exact executable
uninstall helper source. Windows updates retain their existing installer format;
ambiguous duplicate installations are refused. User settings are preserved on update.

Validated locally: IO serial 28, engine USB 27, engine retirement 12, native request
identity 2, control-plane inventory 42, exact E3 restart regression 1, durable
retention 3 and existing durable journal regressions 7, all selected tests passed.
Release aggregate, release metadata self-tests and macOS artifact validator 169
tests passed. Windows installer version/removal policy 13 tests passed using real
filesystem fixtures and process/registry seams, without installing or uninstalling
the user's app or touching real OS credentials. The NSIS hook matches its source.

Windows exact-MSVC private no-bundle build and actual native MCP/loopback suite
passed 31 checks, including allocation after retained operations, Safe BO engage
and release, full-journal capacity recovery, readback and cleanup. The initial test
incorrectly inspected the persistence snapshot (which deliberately clears runtime
S0); it now reads `get_snapshot`. No journal reset or retention wait is used to
manufacture this pass. The normal no-bundle build also passed and showed exactly
one visible, responsive, maximized `Syndocal` window; unauthenticated broker reads
were rejected and the owned candidate closed gracefully. Evidence is in
[artifacts/usb-dmx-blackout-alpha73](artifacts/usb-dmx-blackout-alpha73).

Windows configurations measured: first-party Rust warnings baseline 0/current 0/
delta 0; frontend Vite chunk advisory baseline 1/current 1/delta 0, TypeScript
errors 0. Both Windows installers passed exact payload validation. The NSIS payload
also launched as exactly one responsive maximized window and closed gracefully.
The MSI keeps the previous UpgradeCode, has a new ProductCode and advances its
actual three-field version from 1.2.0.70 to 1.2.73; its MajorUpgrade rejects newer
installed versions. The 13-file Windows ZIP was re-extracted, all hashes matched,
and its real setup helper passed validation without installing anything. Eight
setup-helper cases also passed, including existing NSIS/MSI format preservation.
Hosted macOS final-DMG checks remain pending at this source checkpoint.
Final Windows candidates also remove the DJ-Link rollback credential. A scoped
vault seam proves primary/rollback and exact generic agent targets are retired,
while foreign, malformed and non-generic entries remain. Windows installers/ZIP
were regenerated and revalidated before distribution; the earlier unpublished
candidate and failed proof attempts remain in Temp. The additional disposable
Windows CI lifecycle gate exercises real alpha.72 to alpha.73 NSIS/MSI upgrades,
settings retention, registration replacement and complete removal. It refuses
non-hosted runners and binds both artifact sources to successful CI commits.
Physical USB, friend's cable, clean-machine
installation, macOS GUI, signing/notarization and full product/release acceptance
remain unobserved. Independent review is not claimed because the user prohibits
subagents.

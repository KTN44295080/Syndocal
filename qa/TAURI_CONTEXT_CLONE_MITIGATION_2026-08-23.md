# Tauri Context clone crash mitigation (2026-08-23)

## Scope and incident

The Windows `1.2.0-alpha.7` native artifact repeatedly terminated with
exception `0xc000001d`. Symbolized failure addresses converged on
`tauri_runtime_wry::Context<EventLoopMessage>::clone`. This mitigation is a
temporary vendored backport for the pinned `tauri-runtime-wry 2.6.0`; it is not
an assertion that the upstream change has shipped in a released Tauri version.

## Upstream provenance

- Source crate: `tauri-runtime-wry 2.6.0`, crates.io checksum
  `f85d056f4d4b014fe874814034f3416d57114b617a493a4fe552580851a3f3a2`.
- Upstream issue: <https://github.com/tauri-apps/tauri/issues/15408>.
- Related teardown issue: <https://github.com/tauri-apps/tauri/issues/15785>.
- Upstream change under review: <https://github.com/tauri-apps/tauri/pull/15411>.
- Core Weak/strong ownership commit:
  <https://github.com/tauri-apps/tauri/commit/021476b4db623051c1494e19a1cb85289d2ff45d>.
- Main-thread raw-display retrieval commit:
  <https://github.com/tauri-apps/tauri/commit/8cfc82175f20bacd9ad0ce711a4b18c75f7f193a>.
- Required field/drop order commit:
  <https://github.com/tauri-apps/tauri/commit/9a99265803f7aec47da8362d81d6eaf843eeaa3c>.
- Current upstream documentation commit:
  <https://github.com/tauri-apps/tauri/commit/2d51238d2597fcb677fd52b7179e5fbee25fd75f>.

The vendored crate retains the upstream dual MIT/Apache-2.0 license files.

## Backport delta

`Context` clones now carry only a `Weak<EventLoopWindowTarget<_>>`. The sole
strong `Arc` is owned by `Wry` on the event-loop thread. `_window_target` is
declared before `event_loop`, so Rust field drop order releases the strong clone
before the event loop itself. No `Rc` in tao was converted to `Arc`, and no new
unsafe `Send`/`Sync` implementation was introduced.

Operations that require the event-loop target are routed as follows:

- same-thread message dispatch upgrades the Weak reference and fails with
  `EventLoopClosed` if the runtime has ended;
- monitor queries upgrade the Weak reference or return an empty/`None` result;
- raw display-handle retrieval and theme changes are sent to the event loop;
- runtime-owned monitor queries use the runtime's own event loop directly.

Two compiler-warning-only compatibility edits were made in the copied 2.6.0
source: obsolete `#[must_use]` attributes on trait implementation methods were
removed, and the existing display-handle return lifetime is written explicitly
as `DisplayHandle<'_>`.

The upstream pull request remains open and documents a pre-existing raw-handle
lifetime limitation. On the supported Windows path the raw display handle is
pointer-free; native Windows stress is still mandatory before acceptance.

## Dependency integration

The workspace uses a `[patch.crates-io]` path override to
`vendor/tauri-runtime-wry-2.6.0-syndocal`. `Cargo.lock` keeps version `2.6.0`
but removes the registry source and checksum only for that package. No other
dependency version is intentionally changed.

## Verification and remaining acceptance

Static/compile acceptance requires:

1. `cargo check --manifest-path vendor/tauri-runtime-wry-2.6.0-syndocal/Cargo.toml --locked`
   with no warnings in the patched source;
2. `cargo check -p syndocal --locked --no-default-features` with zero
   first-party warnings;
3. exact diff review showing the Weak/strong ownership and drop order;
4. an independent fixed-hash adversarial review.

This mitigation is not complete until a fresh prerelease native artifact is
built and survives the native matrix: at least 100 WebView reloads with async
query traffic, rapid trusted-LAN/DJ configuration toggles, repeated maximize
and F11 transitions, and a one-hour integrated soak. Acceptance requires one
responsive Syndocal process throughout and no new Windows Error Reporting
Application Error event for the exact executable.

## Rollback

After an upstream released Tauri version contains a reviewed fix, remove the
`[patch.crates-io]` entry and vendor directory, update the pinned Tauri family
intentionally, regenerate `Cargo.lock`, and repeat the full native stress
matrix. Do not silently delete this workaround while the workspace remains on
`tauri-runtime-wry 2.6.0`.

## 2026-08-24 pre-alpha.8 source verification checkpoint

The mitigation and its native command boundary are now source/compile accepted on
`codex/syndocal-v1.2` at base HEAD
`23f350c366ede2fdffcfbf3232e18112eada51ea`. The integrated D3 fixed-hash review
returned P0=0/P1=0/release-blocking P2=0; Engine 822 passed / 2 ignored, I/O 148
passed / 1 ignored, and Syndocal no-default 951 passed / 5 ignored with zero
failures and zero first-party warnings. Final main SHA-256 is
`2CC94D3E5307E6BBC026F1EC815ABC02BFF47E987D62526446AFD1059984E6C7`.

The FlatInvoke adapter protects all 51 affected commands: 39 common mutation
commands, 11 video commands, and one repair command. Six focused tests pass. A Raw
body is never parsed, missing/null optional payloads preserve the Tauri wire
contract, and malformed payloads fail with standard invalid-args before mutation
dispatch. Its independent final fixed-hash review returned P0/P1/P2 zero. Together
with D3's backend admission and the frozen routing/invoke inventories (133/29/33/402,
417 invokes, backend 478/311/133), this closes the software raw-invoke bypass; it
does not prove WebView/runtime crash survival.

The vendored backport warning cleanup and the hardened warning ratchet are also
independently reviewed. Warning-ratchet hashes are
`55D2388C3EA244F3C58B207E651E56F33DB10B6E76BD404A2943497E384926AA` and
`3391F2F149ADCED3B5641E48E326680686B8D243389EAFC7E2DCC3799AEA7E55`;
the exercised warning counts are total/first-party 0/0, and missing/overflowing or
ambiguous Git/Cargo/scanner evidence fails closed. The source freeze will be recorded
at `qa/artifacts/source-freeze/2026-08-24-alpha7-pre-alpha8-source-freeze.sha256`
after checkpoint documents stabilize.

This section does not alter the native acceptance condition above. Product metadata
remains `1.2.0-alpha.7`, the accepted denominator remains exactly 19/71 (26.8%),
and no current native/WER/100-reload or one-hour soak result is claimed. Next is the
synchronized alpha.8 ordinal and `check:release`, then an exact-path native build and
the complete reload, maximize/F11, LAN/DJ-toggle, WER, and integrated soak matrix.

## 2026-08-24 current-source alpha.8 native evidence

The source/compile mitigation is now exercised by the current synchronized
`1.2.0-alpha.8` artifact on `codex/syndocal-v1.2` at pre-commit base HEAD
`23f350c366ede2fdffcfbf3232e18112eada51ea`. The final current-source frontend
chain passed 25/25 in
`target/qa/alpha8-current-source-final-gates-20260824-114918`; its independent
fixed-hash review returned P0=0/P1=0/release-blocking P2=0, and the exercised
warning counts were total/first-party/third-party 0/0/0. Current-source CSS and
checker hashes are `4974a2f828b8b8bd1c9fbe43390d97d5d6702179` and
`73a43ecfb2c6f10c07fb638f84f50375de5213a0`; the Scene Matrix checker SHA-256 is
`F164CD5B5C6C130E1D27B21C6A04CB1C361CEE3346F08FA9DFF77DE522C5FE11`.

The authoritative staged-source inventory is
`qa/artifacts/source-freeze/2026-08-24-alpha8-current-source-freeze.sha256`.
It contains 103 payload records and excludes its own manifest envelope from the
payload to avoid recursive self-hashing. The alpha.7 pre-alpha.8 manifest remains
historical evidence and is not current alpha.8 source authority.

Only the exact checkout release executable was stopped before running the
required `pnpm --dir app tauri build --no-bundle` from the Visual Studio Developer
Shell (MSVC 14.43.34808, Windows SDK 10.0.26100.0). The resulting executable is
57,491,456 bytes with SHA-256
`627BE88032774C7FA0A4C3CD3510A7BFB52E8ED0E76884ADD414B9BFD101F459`; the PDB is
19,582,976 bytes with SHA-256
`5548E4F4B2C3CBB38F1881AAA6C9299AE42211616A8A05E9189C3019838F56AB`. Both
version resources are `1.2.0-alpha.8`. Launch verification found exactly one
responsive maximized Syndocal window from that executable.

The fresh 100-reload artifact
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-reload-stress-100-current-source.json`
has SHA-256
`6D12AA14371B1C837DF67DDE80AB44CB1C6B1329F093567B81524E35076FB621`: 100/100
reloads, 100 unique origins, exit 0, total 40.1424 s, p50 407.3 ms, p95 479
ms, max 530.1 ms, runtime/log issues 0, and WER Application Error/Reliability
deltas 0. This confirms the fresh artifact survived the exercised reload path;
it does not close the required maximize/F11, rapid trusted-LAN/DJ-toggle,
disconnect/fault, or one-hour integrated soak matrix. The old
`2026-08-24-alpha8-final` artifacts remain pre-CSS historical evidence and are
not overwritten.

The native five-display pane route is complete via the supported
`open_pane_window` placement path: `D5 -> D2 -> D3 -> D1 -> D6`. Each pane was
maximized and each route step recorded zero document scroll. The measured route
is D5 monitor 1920x1080/scale 1.5/viewport 1280x650 at `{-2465,1731}`; D2
1920x1080/scale 1/viewport 1920x1009 at `{0,0}`; D3 real physical 3840x2160
4K/scale 1.5/viewport 2560x1370 at `{-3840,-429}`; D1 2560x1440/scale
1.25/viewport 2048x1082 at `{1920,-364}`; and D6 2560x720/scale 1/viewport
2560x649 at `{1598,1080}`.

Canonical route evidence is
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-display-route-current-source.json`,
SHA-256 `41F1D6E2528E7439657F8879F753255221E25F2DF0474139B56C1570E2C32C41`.
The expanded D5 evidence `D5-timeline-tools-expanded-fixed.jpg` has SHA-256
`B780CD4CCDA35CC8A8F1148A64B26C91065EB15E33D57A38FDCC04F8B0A3724D`; native
popup client/scroll is 345/345, nested surfaces are 335/335, and the deepest
44px target is inside the viewport/popup with hit/focus proof and Escape focus
return. The pane was closed; final verification found one exact responsive PID
123952, CDP page 1, and maximized D5 main viewport 1280x672.

All five detached Timeline pane screenshots still show
`Window 'pane-timeline' has no current project transaction owner registration`
six seconds after each pane opened. A same-time
`get_project_authority_bundle` read succeeds with epoch 0/revision 1, but that
does not prove transaction-owner registration or make the persisted status stale.
This evidence accepts placement, maximize, and containment only; it does not
accept transactional pane operation, warning-clean pane startup, or completed
owner registration. That P1 remains an alpha.9 boundary.

This closes native multi-display pane placement/containment only. It does not
claim display-output playback, fullscreen playback, GPU reset/recovery, physical
DJ Link/rekordbox, Stream Deck Pedal/MIDI, ASIO device/fault acceptance, real
DMX, or the one-hour integrated soak. The accepted denominator remains 19/71
(26.8%); this is an evidence checkpoint, not mitigation or release completion.

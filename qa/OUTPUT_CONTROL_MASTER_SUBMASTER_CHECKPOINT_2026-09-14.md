# H5 lighting master / group submaster checkpoint — 2026-09-14

## Checkpoint identity

- Branch: `codex/showclock-review-20260912`
- Base before this checkpoint: `a6549d4c` (observability diagnostic package checkpoint)
- Scope: current-source H5 Control implementation for the Lighting master and group submaster, canonical OutputControl admission, Both control surface, focused rendered-browser checks, and the required Windows native build/process smoke.
- This checkpoint does not close the H5 Flow marker or claim product-wide completion. Physical output, native button-by-button interaction, accessibility, and external release gates remain separate.
- No product-version bump was required; this is an internal checkpoint.

## Implementation and repair

1. `crates/protocol/src/control_plane_command.rs` appends schema-v9 `SetLightingMaster` and `SetGroupSubmaster` actions. Both carry the typed Lighting/Both role and opaque output lease authority; the group id and 0..=1000 milliunit bounds are validated and included in canonical bytes with new append-only discriminants.
2. `app/src-tauri/src/main.rs` routes both actions through the existing external-admission, lease coordinator, output-transition lock, checkpoint/fence revalidation, and exact terminal receipt path. The native dispatch emits only the bounded `EngineCommand` after the final lease candidate is committed. The retired raw `set_lighting_master` and `set_group_submaster` routes remain fail-closed.
3. `app/src/outputControlController.ts` selects exactly one active held Lighting-capable lease, preserves the Lighting/Both resource requirement, converts UI values to milliunits, validates the receipt and unchanged output fence, and exposes the two canonical invoke helpers.
4. `app/src/App.tsx`, `app/src/createWorkspaceNavigationController.ts`, and `app/src/styles.css` add the Control `Both` domain and keep the two surfaces in equal columns without reducing the existing 44px touch controls.
5. The Tauri invoke manifest, frontend inventory, runtime contract checker, canonical registry, source-schema validator, and control-plane test inventory were updated together. The new operations are R4, immutable-audited, rate-limited local OutputControl operations with local explicit action consent.

## Verification evidence

All commands below were run on the current checkout and their exit status was checked.

| Area | Command/result |
| --- | --- |
| Protocol wire contract | `cargo test -p protocol --locked control_plane_command -- --nocapture --test-threads=1` — PASS, 18 passed, 0 failed. This includes canonical-byte, JSON round-trip, role/value/group rejection coverage for both actions. |
| Native control-plane inventory | Fixed MSVC 14.44.35207 x64 linker; `cargo test --manifest-path app/src-tauri/Cargo.toml --locked --bin syndocal control_plane::tests -- --nocapture --test-threads=1` — PASS, 31 passed, 0 failed. |
| Frontend and route inventories | `pnpm.cmd --dir app exec tsc --noEmit`; `pnpm.cmd --dir app run check:frontend-invokes`; `pnpm.cmd --dir app run check:frontend-command-routing`; `node app/scripts/check-tauri-admission-inventory.mjs` — PASS; frontend invokes 477, routing 133 renderer mutations/31 server-authoritative mutations/28 raw dispatches/481 facade dispatches, native Tauri inventory 536 with 18 negative fixtures rejected. |
| OutputControl contracts | `pnpm.cmd --dir app run check:output-control-runtime`; `pnpm.cmd --dir app run check:output-ownership` — PASS. The checker covers schema-v9 operations, fixed milliunit conversions, lease resources/outcomes, receipt/fence checks, backend dispatch, and raw-route rejection. |
| Touch/browser rendering | `pnpm.cmd --dir app run check:touch` — PASS across the five configured viewports and the composed Touch surface at each viewport: 10 pass records, no duplicate controls, no overflow/scroll failures. The browser plugin was unavailable; this is the repository's local Chrome/Playwright rendered-browser harness, not native UI proof. |
| Frontend production build | `pnpm.cmd --dir app run build` — PASS, 357 modules. Vite emitted the existing large-chunk advisory; no first-party compiler warning or failure was introduced. |
| Windows native build | `vcvars64.bat -vcvars_ver=14.44` with the exact Build Tools `14.44.35207` linker, then `pnpm.cmd --dir app tauri build --no-bundle` — PASS; generated `target/release/syndocal.exe`. |
| Native process smoke | Exact executable `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal\target\release\syndocal.exe`; before count 0, PID `34828`, exactly one matching process, title `Syndocal`, `Responding=True`, window handle `0x80BEC`; stopped only that exact process and confirmed remaining exact count 0. |
| Native artifact identity | SHA-256: `21B68A04DDFF948DD39C156B7CB79297770B8AFF0AFBD59F24235B50E75BE41E`. This is an unsigned current-source build artifact, not a signed or published release. |

## Repository branch hygiene

Before this checkpoint, 31 local branches that were already merged into the current branch were deleted together with their matching `origin/*` refs. The current branch, `main`, unmerged branches, and worktree-backed branches were retained. No unmerged or active worktree branch was deleted.

## Evidence boundary and remaining work

The software path now has a canonical lease/fence-bound Lighting master and group submaster implementation and a reachable Both surface. The checkpoint does not claim that a physical fixture, DMX/Art-Net/sACN/NDI/Spout/display device, external client, or venue received output. It also does not claim native button-by-button interaction, NVDA/High Contrast/scaling/IME/reduced-motion acceptance, real two-machine operation, crash/restart replay restoration, soak, signing, publication, or product-wide completion.

The Flow ledger remains **23 Open + 8 Deferred + 27 Complete = 58 markers**. H5 remains open because its broader native, physical, external, and release acceptance boundaries are not proven by this software checkpoint.

Next action: continue the next bounded H5 item only after this checkpoint is pushed; retain the same fail-closed OutputControl authority and update the ledger with evidence for each separately proven slice.

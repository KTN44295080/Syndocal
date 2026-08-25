# Syndocal video engine checkpoint — 2026-08-25

## Repository state

- Branch: `codex/syndocal-v1.2`
- Code checkpoint: `cd11b59` (`test(engine): prove complete video rollback path`)
- Upstream: `origin/codex/syndocal-v1.2` at `cd11b59`
- Scope: `crates/engine/src/lib.rs` only, 271 insertions and no deletions

## Verified evidence

The new `video_full_gate_engine_path_publishes_and_rolls_back_complete_video_state`
test exercises one production `EngineRuntime` command/publication path for media
assets, authored and active clip slots, layer/clip/transition C1 scopes, and the C3
transition bus. It proves exact runtime and published-snapshot rollback after a
forced publication failure, and fail-closed atomicity for an invalid transition
target.

Immediately before commit, Visual Studio Community MSVC 14.44 x64
`link.exe` was the first resolved linker and
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` was pinned to that exact executable.
The following gates passed:

- exact full-gate test: 1 passed, 0 failed, 831 filtered
- adjacent C3 transition-bus test: 1 passed, 0 failed, 831 filtered
- `cargo check -p engine --tests`: exit 0
- first-party warnings observed: 0
- independent Ox-alpha adversarial review: P0 0, P1 0; lane-scoped commit ready

## Explicit non-claims and remaining work

- This checkpoint ends at `EngineSnapshot`; it does not prove decoder, GPU,
  renderer-window, physical display, or three-output presentation.
- Native 3840x2160 at 150% verification remains required on the intended maximized
  Syndocal window.
- `cargo fmt -p engine --check` still reports one pre-existing formatting drift at
  `crates/engine/src/lib.rs:68318`, outside the committed hunk. Remove it in a
  separate focused checkpoint before release-candidate acceptance.
- Next action: finish the pane-startup recovery and 4K deterministic gate, then run
  the native video/output acceptance after the alpha.12 integration build.

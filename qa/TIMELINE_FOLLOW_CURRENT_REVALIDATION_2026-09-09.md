# Timeline Follow current software revalidation — 2026-09-09

This is a current-main software revalidation of the existing Timeline Follow
authority and abort paths. It does not add a new clock policy, output path, or
physical-output acceptance.

- Source before this checkpoint: `main` at `3dbf4f62f08f7af7664679135123dfa9247de993`.
- Product source was unchanged for this checkpoint; the preceding current-main
  checker repair remains the only source change in the immediately prior unit.

## Verification

Static gates, all exit 0:

- `node app/scripts/check-timeline-follow-runtime.mjs` — stale
  epoch/generation rejection, E/R/H abort receipt recovery, visibility/focus/
  target/localization contracts.
- `node app/scripts/check-timeline-follow-hold-ui.mjs` — Immediate/Hold and
  one-measure wait-for-Pedal-1 modes, normalization, and persistence.
- `node app/scripts/check-timeline-cue-audio-runtime.mjs` — fences, singleflight,
  latest mutation, duplicate endpoint, and canonical invoke contracts.
- `node app/scripts/check-timeline-source-shelf-contract.mjs` — authoritative
  Bank/Scene source-shelf contract.

Windows release focused tests used `vcvars64.bat -vcvars_ver=14.44`, the
absolute Build Tools MSVC `14.44.35207` linker pinned in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and that linker first in
`where.exe link.exe`:

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 timeline_follow_runtime_abort_terminal_is_exact_and_history_free -- --nocapture --test-threads=1
```

Result: **1 passed, 0 failed, 0 ignored, 1814 filtered out**.

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 canonical_timeline_follow_abort_full_lock_retry_is_one_publish_and_history_free -- --nocapture --test-threads=1
```

Result: **1 passed, 0 failed, 0 ignored, 1814 filtered out**.

Both tests used the optimized Windows test binary and emitted no first-party
compiler warning in the recorded focused reruns.

## Boundary

These checks cover software authority, stale-result rejection, abort receipt
recovery, and frontend contract shape. They do not prove native Tauri window
behavior, real audio/MIDI/DJ Link/ASIO/NDI/DMX devices, physical output,
multi-machine ShowClock, venue operation, or product-wide completion.

`git diff --check`: PASS before commit.

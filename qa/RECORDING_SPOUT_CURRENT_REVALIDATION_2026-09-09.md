# Recording and Spout current software revalidation — 2026-09-09

This checkpoint reruns the existing Windows release regressions for recording
worker/artifact ownership and fake Spout safety boundaries. It does not change
product source or enable physical output.

- Source before this checkpoint: `main` at `15f19bed0e926d4efeb85163e8f3a7f0e1a61ff3`.
- The completed real-file missing → UI Retry → restore → recovery result was
  not rerun.

Both commands used `vcvars64.bat -vcvars_ver=14.44`, the absolute Build Tools
MSVC `14.44.35207` x64 linker pinned in
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and that path first in
`where.exe link.exe`:

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 recording_ -- --nocapture --test-threads=1
```

Result: **57 passed, 0 failed, 6 ignored, 1752 filtered out**. The run
covered atomic recording publication/recovery, collision and ownership
rejection, diagnostics-reader join/error retention, inherited stdin/stderr
cancellation, explicit Stop/Drop/reaper ownership, worker panic reporting, and
runtime start admission. Ignored tests were subprocess helpers or opt-in
30-minute/real FFmpeg recording tests.

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 spout_ -- --nocapture --test-threads=1
```

Result: **101 passed, 0 failed, 5 ignored, 1709 filtered out**. The run
covered fake Spout authority/fence, hard-black startup and retirement,
generation/route rejection, retry/settlement deadlines, worker teardown, and
Video blackout recovery. Ignored tests require a Windows GPU, real Spout2, or
an external Spout sender/receiver and were not counted as acceptance.

The expected injected-failure thread messages were observed, and their tests
returned `ok`; no first-party assertion was weakened. No application process,
external client, device, or physical output was started.

## Boundary

This is Windows release test evidence only. It does not prove real FFmpeg
codec acceptance, external Spout/NDI interoperability, display hardware,
ASIO/DMX/MIDI, venue operation, Mac behavior, signing, publication, or
product-wide completion.

`git diff --check`: PASS before commit.

## Current-main revalidation — 2026-09-09

The same focused release regressions were rerun against current `main` at
`52c749c1c456107466e183b81329eb2b324981fa`. Product source was not changed by
this checkpoint, and the previously completed real-file thumbnail recovery was
not rerun.

Both commands used the exact Build Tools 14.44.35207 x64 linker, with
`where.exe link.exe` resolving that linker first. Results:

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 recording_ -- --nocapture --test-threads=1
test result: ok. 60 passed; 0 failed; 6 ignored; 1752 filtered out

cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 spout_ -- --nocapture --test-threads=1
test result: ok. 101 passed; 0 failed; 5 ignored; 1712 filtered out
```

The expected injected panic/failure messages were observed inside tests that
returned `ok`; no assertion was weakened. This remains Windows software
evidence only. It does not claim real FFmpeg H.264/AAC recording, real Spout2,
external Spout interoperability, GPU/display hardware, NDI, ASIO/DMX/MIDI,
physical output, Mac behavior, signing, publication, or venue acceptance.

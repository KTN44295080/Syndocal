# Recording software revalidation — 2026-09-12

This checkpoint revalidates the current `main` recording ownership, encoder
shutdown, failure recovery, and Windows publication boundaries. Product source
was not changed by this checkpoint.

- Source under test: `main` at `7eeba1618d6c7a5d7c8af58324baee4c341e1a7d`.
- The native test environment used `vcvars64.bat -vcvars_ver=14.44`, the exact
  Build Tools MSVC `14.44.35207` x64 linker, and an absolute
  `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` pin. `where.exe link.exe`
  resolved that linker first.
- Evidence log: `target/qa/recording-current-main-20260912/recording-tests.log`.

## Focused current-source result

```text
cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 recording_ -- --nocapture --test-threads=1
test result: ok. 60 passed; 0 failed; 6 ignored; 1800 filtered out
```

The run covered:

- recording publication/recovery interruptions at each boundary;
- target collision, late creator, locked target, hard-link, changed identity,
  malformed/future/oversized/escaping recovery intent, and zero-frame rejection;
- diagnostic reader join, primary-error preservation, reader error/panic,
  inherited stdin/stderr cancellation, and descendant reaping;
- Stop timeout ownership retention, late reaping, worker panic reporting,
  poisoned status rejection, and restart admission fencing;
- renderer request framing, malformed/oversized response rejection, and
  recording audio filter/status contracts.

The injected panic/failure messages were expected test stimuli; their tests
returned `ok`. No assertion was weakened. The six ignored cases are subprocess
helpers or opt-in real FFmpeg/30-minute A/V tests and are not counted as
passing acceptance.

## Real codec boundary

The installed SDK-independent FFmpeg/FFprobe pair is available, but it does
not expose the production-required `libx264` encoder. The real MP4 test was
therefore not forced through an incompatible codec and remains an explicit
environment-limited ignored test. The previously recorded full Gyan FFmpeg
8.1.2 synthetic MP4/A/V result remains historical evidence only; it is not
promoted to current-source acceptance here.

## Remaining boundary

This checkpoint records current Windows software evidence only. It does not
close `DEC-RECORD-OWN-001`, prove two-PC ownership or takeover, prove the
recording UI through the deployed executable, prove disk-full/power-loss or
full in-app artifact-import acceptance, or claim ASIO/DMX/MIDI, Mac,
signature, publication, venue, or product-wide completion.

`git diff --check`: PASS before commit.

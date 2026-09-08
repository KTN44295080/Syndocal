# Windows native final validation — 2026-09-08

## Scope

This is the Windows native launch/state/thumbnail-IPC validation for the
current main product source. It uses the existing exact release executable;
the current checkpoint after that artifact changes only the checker and QA
documentation, so no embedded product source input changed. No physical
output was enabled.

## Artifact and probe

| Item | Result |
| --- | --- |
| Executable | `target/release/syndocal.exe` |
| Version | `1.2.0-alpha.69` |
| Size | 64,700,928 bytes |
| SHA-256 | `CC984A75AB399E80C373C6B9133C5FFA6661ABA6878B4D17EF606A063F6523B3` |
| Evidence | `target/qa/native-final-validation-20260908-08/native-final-validation.json` |

The prepared probe was executed in a new isolated evidence directory with the
exact artifact hash. Its first invocation exposed a stale probe omission: the
thumbnail commands now require the existing `started` Channel argument. The
next invocation reached the native command and received a ticket, but the
probe overwrote it with the Channel end sentinel. The final in-memory probe
adaptation passed a Tauri-compatible Channel serialization and retained the
first ticket only. Product source was not changed and the earlier failed
probe results were not treated as acceptance.

## Native result

The final run used port `51490` and process `79752`:

- exactly one visible `Syndocal` window, title `Syndocal`;
- window responsive and maximized;
- output ownership `Standby`, lighting `false`, video `false`;
- `get_snapshot` returned fixture/cue/video output arrays, with zero video outputs;
- `get_media_asset_thumbnail` rejected missing asset `999999` with the expected
  not-found error;
- `get_video_layer_thumbnail` rejected missing layer `999999` with the expected
  `MissingLayer` error, after announcing a valid native ticket
  (`schemaVersion: 1`, lane, and 32-character lowercase request ID);
- physical output operations: `0`;
- owned application exited and the debug listener count returned to `0`;
- post-check found no process at the exact executable path and no listener on
  port `51490`.

This proves the bounded Windows native launch/state/IPC layer only. It does
not prove physical ASIO/NDI/DMX output, venue acceptance, Mac execution,
signing, publication, or product-wide completion. The real-file missing PNG →
Retry → recovery evidence was not rerun.

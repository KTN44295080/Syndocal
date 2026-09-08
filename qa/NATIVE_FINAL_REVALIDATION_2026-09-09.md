# Native final revalidation — 2026-09-09

This checkpoint re-probes the exact executable rebuilt by the warning-gate
run. It contains no product source change; the current `main` changes since
the preceding native probe are QA records only.

## Artifact identity

- Source main: `6215db3b57be0115636f6ead731cfbd2a3ac1064`
- Path: `target/release/syndocal.exe`
- Product version: `1.2.0-alpha.69`
- Size: `64,700,928` bytes
- SHA-256:
  `7ADB24A521BFF21513556EBB56E89ABEA9B628391EF893446C051D0525190559`

The hash was read immediately before launch. The earlier `C22A...` artifact
was not reused after the no-bundle rebuild produced this new hash.

## Fresh native probe

The existing file-move-free probe was run against this exact executable using
a fresh owned WebView2 profile and loopback port. The result was PASS:

- exactly one visible `Syndocal` window, responsive and maximized;
- output ownership was `Standby`, with `lighting_allowed=false` and
  `video_allowed=false`;
- `get_snapshot` returned the expected fixture/cue/video-output arrays with
  zero initial video outputs;
- invalid media-asset and video-layer thumbnail requests were rejected with
  the expected errors and valid `{schemaVersion:1,lane,requestId}` tickets;
- `physicalOutputOperations=0`;
- the owned application exited and the debug listener count was zero.

Evidence JSON:
`target/qa/native-final-validation-20260909-02/native-final-validation.json`.

This is exact-artifact startup/IPC/cleanup evidence. It does not prove a real
file-missing → UI Retry → re-fetch recovery flow, a cancellation hard-stop
deadline, physical output, device acceptance, Mac execution, signing,
publication, or product-wide completion.

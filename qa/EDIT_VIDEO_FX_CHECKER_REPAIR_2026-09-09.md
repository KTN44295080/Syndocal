# Edit Video FX checker repair (2026-09-09)

## Scope

The focused `check-edit-video-fx` gate failed against the current source before
exercising the FX assertions. Its negative selector classified
`.videoClipSlotBankPanel` as a hidden legacy mixer consumer.

That classification was stale after commit `35c66c2`: the Edit Video upper
surface intentionally mounts the Clip Bank in `libraryOnly` mode, and the
viewport containment contract requires that bank to remain reachable. The
product component was not changed in this repair.

## Repair

- Keep the negative assertion for hidden mixer/output/transport consumers.
- Remove only `.videoClipSlotBankPanel` from that negative selector.
- Add a positive assertion that exactly one Edit Video Clip Bank is mounted.

This preserves the fixed three-screen and responsibility-separation contract:
the library surface retains its required bank, while the old mixer context,
legacy transport, clip grid, diagnostics, and pane headers remain unmounted.

## Evidence

- `node --check app/scripts/check-edit-video-fx.mjs`: PASS.
- `CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe node app/scripts/check-edit-video-fx.mjs`: PASS for 1920x1080 and 1280x720.
- The gate retained closed/open/closed FX mounting `0/1/0`, layer-2 builtin
  callback, eight-stage stack, layer-switch reset, no overflow, and no browser
  errors.
- `git diff --check`: PASS.

No product code, IPC, native output, or acceptance boundary was changed.

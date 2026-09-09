# Edit Video FX browser revalidation — 2026-09-09

## Scope

This checkpoint re-runs the existing Edit Video FX browser contract against
the current `main` source. It records the previously unavailable browser gate;
it does not add a video feature, change the three-workspace layout, alter the
native renderer, or reopen an old candidate branch.

- Source HEAD: `d700a71acfe3f2f82428a5372252dbcfb37e484a`
- Product source changes in this checkpoint: none
- Browser: installed Google Chrome executable
- Playwright runtime: desktop bundled runtime at the configured local path
- Native application, physical output, device, and external client: not started

## Verification

The command was run from the repository root with an exclusive Vite port:

```text
$env:PLAYWRIGHT_MODULE_PATH='C:\Users\janua\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\node_modules\playwright'
$env:CHROME_PATH='C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe'
$env:EDIT_VIDEO_FX_PORT='5217'
node app/scripts/check-edit-video-fx.mjs
```

Exit code: `0`.

```text
PASS edit-video-fx 1920x1080: mounted=0/1/0, selected-layer callback=2, controls=64, outer-overflow=0
PASS edit-video-fx 1280x720: mounted=0/1/0, selected-layer callback=2, controls=64, outer-overflow=0
```

The gate verified that the Edit Video inspector remains reachable without a
selected Media Library item, hidden legacy consumers stay unmounted, the
selected layer's existing eight-stage FX stack remains editable, the builtin
effect callback targets the selected layer exactly once, the nested editor
survives the same-layer snapshot refresh, controls remain contained, and no
page errors occur. The native command boundary is replaced by the documented
browser fixture; this is not native renderer or GPU acceptance.

Raw screenshots and result JSON are retained under the generated local
evidence directory:

```text
target/qa/edit-video-fx-20260905/
```

## Boundary

This closes only the browser-contract evidence for the bounded Edit Video FX
slice. `VIDEO-FULL-GATE-001`, `VIDEO-C2-C4-001`, and `TIMELINE-FOLLOW-001`
remain open in the completion ledger until their wider acceptance conditions
are met. This run does not claim native renderer/GPU/display behavior, real
show media, physical output, hardware, Mac execution, signing, publication,
or product-wide completion.

## Current-main revalidation — 2026-09-09

The same browser fixture was rerun against current `main` at
`d394370875ef8ce4592633d9ac52b2693feb1e15` using the configured desktop
Playwright runtime, installed Chrome, and exclusive port `5217`.

Exit code was `0`, with the same results at 1920×1080 and 1280×720:

```text
PASS edit-video-fx 1920x1080: mounted=0/1/0, selected-layer callback=2, controls=64, outer-overflow=0
PASS edit-video-fx 1280x720: mounted=0/1/0, selected-layer callback=2, controls=64, outer-overflow=0
```

No product source was changed, no Playwright dependency was downloaded, and
no native application, physical output, device, or external client was
started. This strengthens browser-contract evidence only and does not close
the broader video ledger rows.

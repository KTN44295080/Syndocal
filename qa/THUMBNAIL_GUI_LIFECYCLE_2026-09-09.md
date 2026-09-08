# Thumbnail GUI lifecycle gate — 2026-09-09

## Scope

This checkpoint records the existing browser harness for the thumbnail UI
controller and its lifecycle boundaries. It does not claim the real-file
missing → Retry → recovery flow, native decoder hard-stop, physical output,
or whole-product completion.

- Source base: `8466811d990a6d445d0f2e050745e8bd7f874637`
- Worktree before this record: clean `main`, equal to `origin/main`
- Product source changes: none
- File-moving fault injection: not attempted

## Verification

The existing fixture was run with an exclusive evidence directory:

```text
THUMBNAIL_QA_DIR=target/qa/thumbnail-recovery-browser-20260909-01
node app/scripts/check-thumbnail-recovery-browser.mjs
```

Result: exit 0, `status: pass` at both `1280x720` and `1920x1080`.
The fixture exercised real DOM interactions with deferred readers and checked:

- no reads before explicit authorization and no reads when native capability is unavailable;
- disabled repeated clicks and independent layer/asset lanes;
- reset aborts both active deferred readers, and a later authorization starts fresh work;
- keyboard asset Retry and layer Retry reachability;
- successful cache reuse without re-reading already successful entries;
- reset, live-only catalog handling, and no dead retry action.

The observed Retry button remained at least 44×24 (`180×28` and `180×30` in
the two viewports). The owned Vite server and browser were closed, and port
5199 had no remaining listener.

## Boundary

The fixture uses deferred test readers rather than a native Syndocal process.
It therefore does not prove a real missing file, native IPC pixel recovery,
native resource reclamation, decoder/OS-I/O/GPU stop latency, or hardware and
venue acceptance. The previously blocked helper that would move a test PNG
was not executed.

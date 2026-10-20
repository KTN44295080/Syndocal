# Claude integration takeover — 2026-10-20

## Scope and recovered state

The owner asked Codex to continue Claude's work after Claude exhausted its usage
limit. The recovered local session ended while validating the T33 merge; its
last intended next step was T35 integration, followed by collecting T32b and T20d.
This checkpoint covers that integration queue, not a new feature tranche or
release/hardware acceptance.

At takeover the original checkout was clean on `task20-perf` at `50ba8291`.
`main` was clean at `7d15227d` in the existing `wt-merge` worktree, two commits
ahead of `origin/main`. T32, T33 and T35 worktrees were also clean. The merge
worktree was detached at its existing commit and the original checkout switched
to `main`; all source branches and retained ignored evidence were preserved.

| Lane | Recovered head | State at takeover |
| --- | --- | --- |
| T33 output authoring | `ad5fe649`, merged as `7d15227d` | Merge present; validation failed |
| T35 browser/layout repair | `6d9e8efb` | Complete lane evidence; awaiting integration |
| T32 ISF DATE/images/audio | `e40d6bbd` | Integrated T27 at `cbae80d3`; awaiting main integration |
| T20 performance envelope | `50ba8291` | Measured product source `c204074d`; awaiting integration |

## T33 merge repair

The merge retained T33's 590-route fingerprint while correctly expecting 591
routes after T27. The immutable admission table consequently rejected the whole
inventory, including `set_bpm`. Two Engine count constants also remained 286
although T27 plus T33 declare 287 commands. Existing aggregate assertions were
already correct: 1745 legacy sources and 1778 including keyboard sources.

The repair pins the independently enumerated 591-route SHA-256
`8910316b231c1a76e447a9800ade5249e56a2410395d0164153f37ba8b003adf`
and updates those two Engine constants. Existing route classifications,
fail-closed checks and negative tests remain intact. An independent read-only
review confirmed the exact handler names, Engine variants, totals and classes.

## Integration validation

T35, T32 and T20 were merged as `704458a7`, `e64818cd` and `71e9fe7f`.
T32 already borrowed RGBA with `Cow`; the sole T20 merge conflict retained that
implementation together with image and audio textures. An independent review
of the merged product confirmed startup admission, lease receipt gating,
preparation timers and both NDI smoke assertions were preserved.

Two local integration failures were repaired without changing acceptance:

- The Syphon component fixture now maps the performance-budget module added by
  T20 when compiling the component into a data URL. This fixes fixture module
  resolution; it does not replace the production implementation with a stub.
- T33's extra output configuration disclosure left the projection editor with
  only 23 client pixels at 1920 × 1032. Its open state now has a 120 px floor,
  its closed summary has a 44 px floor and the status pane can scroll internally.
  Existing compact grid reflow and all typography/control sizes are retained.
  The focused Setup Video lane also honors the existing optional screenshot
  directory, so its repaired rendering can be inspected.

| Integrated check | Result |
| --- | --- |
| Exact admission/BPM repair focus before merges | 50 passed, 0 failed |
| `check --workspace --all-targets` | Passed; 0 compiler warnings |
| Full app, `--test-threads=1` | 2234 passed, 0 failed, 23 ignored |
| Full Engine, `--test-threads=1` | 1182 passed, 0 failed, 14 ignored |
| Full Protocol | 299 unit + 8 integration + 4 doc tests passed |
| Video ISF / libav / c1 with libav feature | 66 / 34 / 27 passed; libav has 6 ignored |
| AI0, AI7, frontend routing, exact Tauri inventory | Passed |
| Output authoring, FX catalog/parameters, audio mapping | Passed |
| Monitor controller, performance budget, destructive actions, localization | Passed |
| TypeScript | Passed |
| Complete broad browser run | All groups passed except Setup Video at 1920 × 1032 |
| Repaired Setup Video at all five matrix sizes | Passed, unchanged assertions; original failed log preserved |
| Timeline browser | Passed at 1920 × 1080, 1366 × 768, 860 × 520 and 1280 × 720, including direct resize routes |
| FX browser | Passed, image bindings and existing flows, EN/JA and two themes |

The affected local Setup Video scenario was rerun after its bounded CSS repair;
the unrelated passing broad groups were not repeated. At 1920 × 1032 its status
pane overflow is 94 px, within the existing 96 px bound, while mapping controls
remain reachable and the document/app remain fixed. No threshold was weakened.
The two focused runs reported a locked temporary Chrome Crashpad metrics file
during profile deletion; both validation exits were zero. Unrelated processes
and temporary directories were not cleaned up.

Browser plugin was not available in this session. The repository's existing
CDP/Chromium harnesses were used with Vite 5380 / CDP 9380 and FX 5382 / 9382.
Control Both, compact Video, Japanese image inputs and the repaired Setup Video
screenshots were visually inspected. These fixture results are not native or
physical-output proof.

The earlier full-suite failures did not recur in the integrated single-thread
runs. This establishes the stated configuration only; it does not diagnose the
earlier parallel-run failures. Compiler warning baseline for the initial pinned
repair focus was zero; integrated checks/tests are zero, delta zero. Existing
Vite bundle-size advice is tracked separately from Rust compiler warnings.

## Final Windows native gate

The final `pnpm --dir app tauri build --no-bundle` passed in 2 min 58 sec after
the local CSS repair, with MSVC `14.44.35207`, an absolute pinned Build Tools
linker and a verified PATH-first `where.exe link.exe`. `CARGO_TARGET_DIR` was
unset. Rust emitted zero warnings; the existing Vite large-chunk notice remains.
An earlier integrated build also passed, but it predates the CSS repair and is
not the final window evidence.

| Artifact / observation | Final evidence |
| --- | --- |
| Executable | This checkout's `target/release/syndocal.exe` |
| Product version / bytes | `1.2.0-alpha.82` / 74,672,640 |
| SHA-256 | `c7a286658b0fff2d0df24347d988a3bff182005f3377d4bc6f74cefad431c4a7` |
| Exact launch | PID 47328, verified absolute executable path |
| Window | Exactly one `Syndocal`, handle 268286, responsive |
| Maximized | System menu: Restore enabled, Maximize disabled; 1280 × 752 logical capture |
| Interaction | Tools opened and closed normally on the verified window |
| Exit | Graceful close completed; no Syndocal process remained at 21:13:19 JST |

The Computer Use launcher initially selected the separately installed application
despite the supplied path. That instance was closed and excluded from acceptance.
The final launch used the explicit executable path and its process identity was
verified before window actions. No output activation or hardware action was used.

The owned Vite process on 5380 was stopped after validation. Existing unrelated
servers, worktrees and their evidence were preserved. This is an internal source
checkpoint; no installer, product-version bump or published release was created.

Local validation logs are retained under
`target/qa/claude-takeover-2026-10-20/`.

The recorded Task20 counterfactual patch `before-source.diff` intentionally
retains its original whitespace and provenance. The full integration diff check
reports its existing line 33 patch-context blank line; the check excluding that
archived payload passes. New source/checkpoint changes pass `git diff --check`.

## Preserved boundaries

- T20's 63 supported after workloads belong to measured source `c204074d`:
  22 passed the declared envelope and 41 missed. They are not measurements of
  the later integrated head. Six NDI configurations were unsupported in that
  default build.
- Overloaded decoder/output preparation can exceed the 1000 ms retirement
  deadline. The existing recovery evidence does not establish a runtime fix.
- Prior full-suite Engine failures and the T20 backup-listing failure require
  accurate reporting; an isolated pass alone does not establish their cause.
- Physical DMX/audio/camera/network/receiver behavior and macOS Syphon runtime
  remain outside this local integration checkpoint.
- Existing worktree directories and their ignored validation evidence are
  preserved. Cache/worktree cleanup is separate maintenance.

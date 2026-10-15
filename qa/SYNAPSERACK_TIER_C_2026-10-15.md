# SynapseRack Tier C comparison — 2026-10-15

- **State:** 8 Syndocal scenes measured; all six SynapseRack scenes
  will remain unmeasured by owner decision on 2026-10-15. Performance comparison
  with SynapseRack is dropped; no cross-application ranking follows.
- **Replacement:** [functional comparison (Part D)](SYNAPSERACK_FEATURE_COMPARISON_2026-10-15.md)
  compares features, without performance claims.
- **Checkout:** shared `main`, base `d07f7c504acd24077176dc42f8f553578317874e`.
  No product code changed in this Tier C measurement checkpoint.
- The first attempt was blocked by setup elevation and PresentMon permissions; the owner cleared both.

## Setup

- ASUS ROG Flow Z13 `GZ302EA_GZ302EA`; AMD Ryzen AI MAX+ 395,
  16 cores / 32 threads; 59,751,862,272 bytes visible RAM.
- Windows 11 Home `10.0.26200`, build `26200`; AMD Radeon 8060S,
  driver `32.0.31032.1003`.
- One active TMA `TL134ADXP03` (`TMA0803`); desktop **2560 × 1600,
  180 Hz**, unchanged. The enumerated Parsec adapter has no current mode.
- Syndocal: decorated, windowed native Display, **1920 × 1080 physical
  client pixels**, on that monitor; main editor maximized. DPI is 192 (200%).
  Geometry is read with a per-monitor-aware helper; an initial DPI-unaware
  query returned virtualized 960 × 540, not a smaller physical output.
- SynapseRack: intended same windowed 1920 × 1080 output; no benchmark
  scene or output window was verified there.

## Software and hashes

- **SynapseRack**, installed by the owner at
  `C:\Program Files (x86)\SynapseRack\SynapseRack.exe`.
  - App visibly reports **Demo**, **3.3.0**. Installed `build-info.json`:
    **build 122**, `v3.3.0-build122-run1-188051f`,
    build time `2026-09-18T11:41:23Z`.
  - Main EXE: 667,648 bytes; SHA-256
    `04b0d372d954b14817668999cab06628823758251dfb48e58031f39a2b74aadc`.
    Its Windows file version identifies the Unity player, not the app release.
  - App assembly `SynapseRack_Data\Managed\Assembly-CSharp.dll`:
    4,489,728 bytes; SHA-256
    `747bfc8affab2245e00adda0052fee2217b3df83fa067e171a4e5b21ec8293c2`.
  - Original online installer under
    `%LOCALAPPDATA%\SyndocalDev\synapserack-download\`:
    `SynapseRack-Windows-Online-v3.3.0-build122-Installer.exe`.
    Rechecked **1,544,597,166 bytes**, SHA-256
    `a21a9767dc9ae7fa8fd4882f17047892932336f54634b4ae00d82e924778558a`;
    signature **NotSigned**. Both expected size and digest match.
- **PresentMon console 2.6.0**, portable at
  `%LOCALAPPDATA%\SyndocalDev\presentmon\PresentMon-2.6.0-x64.exe`.
  - [Official download](https://github.com/GameTechDev/PresentMon/releases/download/v2.6.0/PresentMon-2.6.0-x64.exe),
    980,320 bytes; SHA-256
    `b2a706bc6ad475749e3b7e3409263aa1e6906d45bdcf993f6dbc0f660188f1af`.
    `--help` confirms the version and capture flags. No reinstall or elevation.
  - Current `whoami /groups` contains:
    `BUILTIN\Performance Log Users  Alias  S-1-5-32-559  Mandatory group, Enabled by default, Enabled group`.
    Token remains medium integrity. Capture succeeds unelevated.
- **Syndocal 1.2.0-alpha.82**, this checkout's `target\release\syndocal.exe`.
  - 72,367,104 bytes, modified `2026-10-15T17:20:12+09:00`; SHA-256
    `d8acf52ebd920c3be9f003b23a22a30037478d9fe3661768f8831e498b31c10f`.
  - Existing release reused: no newer product source file was found in the
    checked Rust/frontend source trees. No native rebuild or CI wait.

## Media and scenes

- Exact source directory:
  `C:\Users\janua\AppData\Local\SyndocalDev\vj-bench-media`.
- `h264_1080p30.mp4`: 1920 × 1080, 30 fps, 12 s, no audio,
  17,716,533 bytes; SHA-256
  `53861258351df591b53450228d532c60fd914caa8de1d88b2029e480488e0c2b`.
- `hap_1080p30.mov`: 1920 × 1080, 30 fps, 12 s, no audio,
  35,146,526 bytes; SHA-256
  `e6a0b9bc2bbcf6ff68eb85ed85d42e4144c787290b1b735c23bd77ba459bec81`.
- Rehashed against `manifest.json`. Each scene repeats its named file on
  every layer, with distinct layer/clip identities, speed 1 and full-clip
  hard looping. No effects, transforms, audio, fixtures or enabled DMX routes.
- Back-to-front blend order: 1 layer Normal at 100%; 2 layers Normal/Add
  at 50% each; 4 layers Normal/Add/Screen/Multiply at 50% each;
  8 layers repeat that four-mode sequence at 50% each (Syndocal only).
- Authored checkpoints, output ownership, window observation, geometry and
  decoder/presenter status are saved with the captured rows.
- A first one-layer H.264 pilot is retained separately; its final table row
  uses the later capture with the same start/end evidence as the other rows.

## Capture and calculation

- Only one comparison application ran during each capture. SynapseRack was
  closed before Syndocal measurements. This was the ordinary desktop session,
  not an isolated machine-wide benchmark environment.
- Scene setup uses actual native IPC through the existing native backend
  helper and normal project loading. No browser renderer substitute, fake
  broker, account, sign-in, licence key, payment or security-setting change.
  No external MCP principal or pairing credential was created.
- Command, from the repository root:
  `PresentMon-2.6.0-x64.exe --process_name syndocal.exe --timed 60 --delay 10 --terminate_after_timed --v1_metrics --no_console_stats --output_file target\qa\tier-c\<scene>.csv`.
- Ten seconds of warm-up precede each 60-second capture. PresentMon exit
  status is checked. Raw rows include presents marked dropped; they are not
  silently removed. Table statistics use positive finite `msBetweenPresents`.
- Each accepted CSV has one Syndocal PID / DXGI swap chain. The editor's
  WebView renders in separate processes, excluded by the process target;
  control and output chains are not pooled. Raw chain IDs and present modes
  are in `results.json`.
- Quantiles use linear interpolation at `(N - 1) × p`. Thresholds are
  strictly `>16.7` and `>33.4` ms; percentages use the same N as quantiles.
- Syndocal's presenter uses `AutoVsync`; measured SyncInterval is 1.
  The native frame clock targets 60 Hz while the monitor stays at 180 Hz.
  This is present-to-present cadence, not render work time or unique decoded
  media-frame cadence (the media is 30 fps).

## Results

- **Measured** means a completed 60-second scene capture. **Not measured**
  and `—` mean no percentile, count or percentage, not zero missed frames.

| Media | App | Layers | Status | N | p50 ms | p95 ms | p99 ms | >16.7 ms count (%) | >33.4 ms count (%) |
| --- | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| H.264 1080p30 | SynapseRack | 1 | Not measured | — | — | — | — | — | — |
| H.264 1080p30 | SynapseRack | 2 | Not measured | — | — | — | — | — | — |
| H.264 1080p30 | SynapseRack | 4 | Not measured | — | — | — | — | — | — |
| H.264 1080p30 | Syndocal | 1 | Measured | 3513 | 16.963 | 17.912 | 18.472 | 2447 (69.66%) | 6 (0.17%) |
| H.264 1080p30 | Syndocal | 2 | Measured | 3498 | 16.966 | 18.406 | 19.097 | 2145 (61.32%) | 6 (0.17%) |
| H.264 1080p30 | Syndocal | 4 | Measured | 3460 | 16.931 | 19.559 | 20.530 | 1907 (55.12%) | 9 (0.26%) |
| H.264 1080p30 | Syndocal | 8 | Measured | 3376 | 16.429 | 23.357 | 25.341 | 1660 (49.17%) | 12 (0.36%) |
| HAP 1080p30 | SynapseRack | 1 | Not measured | — | — | — | — | — | — |
| HAP 1080p30 | SynapseRack | 2 | Not measured | — | — | — | — | — | — |
| HAP 1080p30 | SynapseRack | 4 | Not measured | — | — | — | — | — | — |
| HAP 1080p30 | Syndocal | 1 | Measured | 3533 | 16.965 | 17.900 | 18.356 | 2499 (70.73%) | 0 (0.00%) |
| HAP 1080p30 | Syndocal | 2 | Measured | 3524 | 17.002 | 18.325 | 18.931 | 2304 (65.38%) | 0 (0.00%) |
| HAP 1080p30 | Syndocal | 4 | Measured | 2144 | 27.819 | 30.088 | 31.722 | 2144 (100.00%) | 2 (0.09%) |
| HAP 1080p30 | Syndocal | 8 | Measured | 3532 | 16.995 | 18.312 | 18.789 | 2261 (64.01%) | 0 (0.00%) |

## Meaning and limits

- Syndocal p95 for **1 / 2 / 4 / 8 layers**: H.264 **17.912 / 18.406 / 19.559 / 23.357 ms**; HAP **17.900 / 18.325 / 30.088 / 18.312 ms**.
- These describe native presentation on this machine. SynapseRack has no
  measured rows, so neither a speedup nor a winner can be inferred.

- A present interval slightly above 16.7 ms counts as over budget even when
  close to a 60 Hz cadence. The over-budget percentage is not the percentage
  of dropped media frames. PresentMon cadence does not establish blend pixel
  equivalence or physical display latency.
- The earlier Tier A tables time rendering without this window/present path;
  their render-budget claims cannot replace these Tier C measurements.
- These layer counts cross compositing paths: the native policy uses GPU
  composition for six or more plain layers. Presenter diagnostics distinguish
  the four-layer CPU path (zero GPU layer slots) from the eight-layer GPU
  path. HAP ×4 presented at p50 **27.819 ms (about 36 fps)** on the CPU,
  while HAP ×8 presented at p50 **16.995 ms (about 60 fps)** on the GPU.
  This drives Part C: widen eligible GPU composition to two or more layers
  at any output size. A faster eight-layer result is not a universal scaling claim.
- SynapseRack's Demo limit is four layers / two group layers according to the
  vendor; Demo state was observed, but limit enforcement was not exercised.
- No comparable SynapseRack presents, verified Normal/AlphaBlend equivalence,
  4K comparison, fullscreen comparison, physical output/release acceptance,
  NDI or Spout performance measurement. The bundled SynapseRack output app's
  Spout hand-off would itself be a presentation-path difference to record.
- No product compiler or tests were run; compiler warnings are unmeasured.
  PresentMon's warning about short-lived/other-account process names is a
  capture-tool advisory, not a product compiler warning.

## Blockers and recovery in this resume

- **SynapseRack UI:** the app is reachable and Demo can be entered, but the
  custom content controls expose no semantic accessibility elements.
  `MediaBrowserCache → ImportFromDirectory` opens a native folder picker;
  a freshly observed element action failed with
  `element 30 is not available in cached app state for SynapseRack.exe`.
  A parent-screenshot coordinate action expanded C: but also changed the
  parent window placement; Escape did not dismiss the picker. No reliable
  scene assignment/opacity/output setup was established. Further exploratory
  clicks stopped; the precise owner checklist below is the fallback.
- **Automation bootstrap:** importing the existing native backend module
  directly into the restricted Node REPL failed with `process is not defined`.
  Used ordinary Node scripts under ignored `target/qa/tier-c/` instead.
- **Native scene preparation:** replacing the renderer's registered project
  owner caused `Project transaction owner does not match the invoking renderer
  window`; premature requests returned `stale_fence`. Explicit lease/Arm
  attempts also returned `invalid_request` / `forbidden` after lease expiry
  or ownership changes. These attempts are setup failures, not measured rows.
  Normal renderer ownership and a clean process restored ordinary Enable.
- **Restart trial:** one extra startup produced Windows error `0xc0000142`;
  the OS error was dismissed and only exact-checkout owned processes were
  stopped. A subsequent launch had one responsive maximized Syndocal window.
- **Switching an open Display:** a direct project replacement closed the
  output but retained a faulted managed authority:
  `Managed output authority is already Faulted and remains retained`.
  It also blocked normal process exit. The last completed capture remains
  valid; the next scene was not published. The retained owned process was
  terminated by exact executable path and the next scene started cleanly.
  Subsequent setup closes the Display before replacement or uses a fresh
  process. No product workaround or permissive fallback was added.
- **PresentMon:** no access-denied error in this resume. An idle 1-second
  permission probe started/stopped with exit 0 but produced no CSV; it was
  not a scene measurement. Completed output captures produce real CSVs.
- Trying to read a CSV while PresentMon held it returned `EBUSY`; final
  analysis waits for recorder completion. No incomplete file is accepted.
- **Cleanup:** the legacy `set_output_ownership_role` call was rejected as
  `Tauri app command 'set_output_ownership_role' is retired and unavailable`.
  Normal New-project cleanup restored empty state and disarmed both gates;
  the on-disk role preference was then verified as `Standby`. No retired
  command was enabled and no preference file was edited by hand.

## Historical owner checklist: six SynapseRack scenes (cancelled)

- Retained as setup history only. The owner cancelled SynapseRack performance
  measurements on 2026-10-15; none of these steps remains a required action.

- Use a fresh unsaved scene, no effects or node output, no group layers.
  Import the exact media directory above with
  `MediaBrowserCache → ImportFromDirectory`, using the folder picker by hand.
  [Vendor import guide](https://synapserack.com/en/docs/load-video/).
- Build each row below twice: first using `h264_1080p30.mp4` on **every**
  listed layer, then `hap_1080p30.mov` on every listed layer. Leave all other
  layers unassigned and stopped; remove unused empty layers if needed.

| Scene | Back layer | Next layer | Next layer | Front layer |
| --- | --- | --- | --- | --- |
| 1 active layer | Layer_0: Normal, 100% | — | — | — |
| 2 active layers | Layer_0: Normal, 50% | Layer_1: Add, 50% | — | — |
| 4 active layers | Layer_0: Normal, 50% | Layer_1: Add, 50% | Layer_2: Screen, 50% | Layer_3: Multiply, 50% |

- Click each layer's preview, then its exact clip thumbnail to assign it.
  Set speed 1, start playback, loop the full 12 seconds; disable beat sync
  and crossfade looping. Watch the burnt-in frame number advance and wrap.
  Raise each opacity from its default zero and verify the intended value.
  The vendor documents a scaled opacity-readout bug, so its displayed number
  alone is insufficient evidence of 0.5 or 1.0.
- **Normal naming must be resolved:** the layer menu lists `AlphaBlend`,
  not `Normal`. Check the alpha/opacity behavior against Normal before
  treating it as equivalent; document any remaining mismatch. Do not silently
  substitute Add. [Vendor layer controls](https://synapserack.com/en/docs/window/layer/).
- In Layer Hierarchy, top is back and bottom is front. Confirm exactly the
  order in the table. [Vendor order guide](https://synapserack.com/en/docs/window/layer-hierarchy/).
- Set `Setting → GlobalSettings → Output → Resolution` to Width 1920,
  Height 1080, Apply. Choose `Output → Create Output Window`; keep it
  windowed on the TMA monitor and verify a **1920 × 1080 physical client**.
  Do not maximize/fullscreen it to the 2560 × 1600 desktop.
  [Vendor output guide](https://synapserack.com/en/docs/projector/).
- The installed bundled receiver is `OutputEXE\Output.exe`. Confirm which
  process owns the visible output; the vendor says this window receives the
  master via Spout. Capture `SynapseRack.exe` and the verified receiver PID
  separately as needed; label the visible-output chain and never pool them.
  Record vsync/present mode. Warm up 10 s, then capture 60 s for each row,
  with Syndocal closed. Do not add an 8-layer Demo row.
- If any step requires an account, credentials, a key, payment or a security
  setting change, stop and report that requirement.

## Evidence and completion boundary

- Ignored raw evidence: `target/qa/tier-c/`, including `syndocal-*.csv`,
  scene `.sdc` files, `*-start.json`, `*-end.json`, `*-setup.json`,
  `*-presentmon.log`, `results.json`, geometry/setup scripts and screenshots.
  Historical `setup-evidence.json` and `permission-probe.log` describe only
  the original blocked attempt; `resume-groups.txt` describes the new token.
- Both comparison applications and the owned PresentMon recorders are stopped.
  No benchmark Display remains open. The six SynapseRack captures are cancelled
  by owner decision, rather than pending owner setup.
- Affected references reviewed; `git diff --check` passed. Both QA documents
  form the Part A docs checkpoint; no product-code change or version bump.

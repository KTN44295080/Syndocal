# VJ Operator Desk Acceptance

Updated: 2026-07-14

## Scope

This gate fixes the live operator contract for a populated VJ show. It is separate from the empty-show first-run gate, native window/F11 acceptance, video decode benchmarks, and hardware output acceptance.

The fixture contains:

- seven 1080p/60 video layers, shown as a six-layer live bank plus a one-layer terminal bank;
- mixed layer FX states: enabled, bypassed, no effect, parameterized and control-free;
- an ordered maximum-eight-stage stack on layer 1: `Threshold`, bypassed `Monochrome`, `Invert`, `RGB Split`, `Mirror`, `Scanlines`, `Vignette` and `Posterize`; the selected root exposes one control of every supported ISF kind (`Event`, `Bool`, `Long`, `Float`, `Point2D`, `Color`) and the selected tail has one control;
- three routed outputs: `Main LED` (`Live`), `Side Projection` (`Off`) and `Stream Fill` (`BO`).

## Operator contract

- Every visible compact layer keeps source state, opacity/transport actions, built-in FX add, selected-stage Enable/Bypass and Advanced actions on the live surface. The quick status exposes active/total stage count and the selected label/state.
- Stack rows, imported add, reorder, reset, remove, description and parameter controls are not mounted while Advanced is closed. Initial Advanced DOM count is zero; opening layer 1 mounts eight ordered rows and only the selected stage editor. The root mounts six typed control rows and ten labelled interactive components; selecting `Monochrome` changes that to one control, and returning to `Threshold` restores all six rows. Closing Advanced returns the editor count to zero.
- At eight stages, built-in and imported Add actions are disabled. Removing one stage exposes `7/8 FX` and re-enables both Add paths.
- Enable/Bypass, Move and Remove use the normal project transaction boundary and targeted `stageIndex`. Snapshot refresh must preserve the open Advanced surface, selected FX identity and keyboard focus; Move follows the same FX across indices, and removing the selected FX chooses a safe adjacent stage. Each committed mutation refreshes stage-local GPU diagnostics.
- Event is a dedicated non-historical pulse. While its `1 -> 0` command is in flight, the affected FX controls and global Undo/Redo are disabled. The pulse must not begin or commit a project transaction, change Undo/Redo depth, or leave a stored value of `1`.
- The output rail keeps every output and its `Live`/`Off`/`BO` state visible. Selecting a rail item must synchronize the selected rail item, the only visible output detail card, the Program monitor label and the Program monitor request `outputId`.
- Internal scroll containers may scroll vertically where intended, but hidden overflow and action/control rectangles outside their owning pane are rejected.

## Automated acceptance

Run:

```powershell
pnpm --dir app run check:vj-operator
```

The gate runs English and Japanese at 1920x1080, 1366x768 and 1280x720. On 2026-07-14 all six cases passed with, at each measured phase:

- unsafe internal overflow: `0`;
- operator/control rectangles outside their owner: `0`;
- layer bank transition: `1-6 / 7` to `7-7 / 7`;
- Advanced lifecycle: `0` editors to `1` editor with `8` stack rows / selected-root `6` typed rows and `10` labelled controls, selected-tail `1` control, selected-root `6` rows, then `0` editors;
- supported control kinds: `Float,Bool,Long,Point2D,Color,Event`, including integer step `1`, a named Bool checkbox, labelled vector/color components and a labelled Event trigger in both languages;
- maximum-stack state: `8/8 FX` disables both Add paths; Remove changes it to `7/8 FX` and re-enables them;
- Event pulse: `1 -> 0`, stored value `0`, Undo/Redo depth unchanged, no project transaction, and mutation/Undo/Redo interlock while busy;
- stack identity: `Threshold,Monochrome,Invert,...` -> crossing Move -> selected Move restore -> selected Remove; the selected label follows `Monochrome`, then safely becomes adjacent `Invert` after removal while focus remains on the moved action;
- committed FX mutations: `6` begin/commit pairs, `0` cancels, `6` snapshot refreshes and `7` diagnostic refreshes; the final quick Bypass carries `stageIndex = 0` and the stored and published root stage is disabled;
- output synchronization: initial output 1, rail-selected output 3, detail `Stream Fill`, Program label `Stream Fill` and Program request `outputId = 3`.
- low-height safety: at 1280x720 the A/V status and Clip Grid remain on explicit rows 3 and 4, while recording error/drop telemetry remains visible (`2 dropped` in the fixture).

Setting `SYNDOCAL_VIEWPORT_SCREENSHOT_DIR` captures the initial and expanded operator states for visual review. The 1920x1080 capture is the design review surface; 1366x768 and 1280x720 are containment fallbacks.

The first element-level run found a real approximately four-pixel overflow in the compact Advanced action row at fallback widths. Compact button and parameter grids were constrained, then all six cases passed.

The final current source was rebuilt as the separate `Syndocal QA - ISF Stack Final` ASIO-capable, no-default-feature native target. Its maximized decorated window capture was 1913x1080 with the normal `VIDEO CONTROL` header. F11 produced an origin-0 exact 1920x1080 surface without the title bar, taskbar or duplicate header; Escape restored the 1913x1080 decorated window and header. The native run used an empty project and did not start microphone capture or an external output, so populated Layer/Output behavior remains the browser stateful evidence above rather than a claimed native media/pixel test.

## Claim boundary

This proves the compact operator interaction, maximum-eight-stage display, all six supported ISF control kinds, selected-stage lazy editor, Event pulse interlock, targeted Enable/Bypass, Move and Remove transactions, state/focus/diagnostic synchronization and containment contract. Add/import capacity locking is exercised, but an actual Add/import, Reset and Clear transaction is not. It also does not prove native effect pixels, the headless-readback/native-re-upload cost, 4K multi-layer endurance, TouchDesigner/SynapseRack performance parity, or venue usability. Those remain governed by the competitive audits and hardware acceptance documents.

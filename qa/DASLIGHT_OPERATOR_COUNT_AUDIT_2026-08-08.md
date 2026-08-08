# Daslight 5 operator-count audit (2026-08-08)

## Scope and method

- Compared the real Daslight 5 desktop application with Syndocal's executable
  operation-count contract in `qa/harnesses/check-operation-counts.mjs`.
- Daslight stayed maximized at 2559 x 1391 logical pixels for the entire audit.
  No window resize, restore, or layout compression was used.
- The working project was `Codex-Chaser322-Probe.dvc*`. The asterisk was
  already present and the project was never saved.
- One click, one drag, or one press-hold-release gesture counts as one
  operation. Setup needed to reach the same stated starting surface is not
  counted.
- Outcomes were verified in the visible application state, not inferred from
  button labels.
- 2026-08-09 addendum: Save/reopen used a new scratch copy,
  `C:\tmp\syndocal-daslight-op-audit-20260809.dvc`; the original user project
  was not overwritten. Daslight remained maximized during both observations.

## Same-task results

| Task | Daslight 5 | Syndocal | Result | Daslight outcome evidence |
|---|---:|---:|---|---|
| Place scene on timeline lane | 1 drag | 1 drag | Same | Scene card dragged directly to a visible timeline lane. |
| Toggle timeline layer mute | 1 click | 1 click | Same | Lane mute state changed. |
| Expand and restore timeline pane | 2 clicks | 2 ops | Same | Expand control, then restore control. |
| Trigger scene from matrix | 1 click | 1 click | Same | `B-WineRed` became active and its elapsed timer appeared. |
| Latch scene Live Speed | 1 drag | 1 gesture | Same | SPEED dial moved and the five-second scene advanced at the altered rate. |
| Reset all scene live modifiers | No one-action equivalent | 1 click | Syndocal fewer | Daslight exposes four independent dials. Right-click on SPEED showed `Add control to Touch view`; double-click changed the dial value. The v1.4 manual documents the four Live Control Dials but no dial reset action. `Reset All/Selected Beams` applies to Live Edit values, not scene dials. |
| Release active scene from matrix | 1 click | 1 click | Same | The scene's right-edge action stopped playback and removed the elapsed timer. |
| Open Touch workspace | 1 click | 1 click | Same | TOUCH workspace became visible. |
| Trigger scene from Touch | 1 click | 1 click | Same | Existing Touch scene button became active. |
| Momentary flash from Touch | 1 press/release | 1 press/release | Same | Enabled `Flash mode` in the selected scene's Advanced Properties, added that scene to Touch, then verified one click gesture fired it without latching after pointer release. |
| Enter Touch Edit mode | 1 click | 1 click | Same | Daslight palette/grid and Syndocal Edit surface became editable. |
| Add Touch button control | 2 clicks | 1 click | Syndocal fewer | Daslight required `BUTTON` in the palette, then a placement click on the grid. Syndocal creates the control from its palette click. |
| Create Mapping FX from an open family chooser | 1 click | 1 click | Same | Daslight `+` menu was opened as setup; one `MAPPINGS` click added a visible `MAPPINGS` generator. |
| Save current project | 1 key (`Ctrl+S`) | 1 key (`Ctrl+S`) | Same | Daslight kept the scratch-project title clean after one key gesture. Syndocal wrote `C:\tmp\syndocal-op-audit-20260809.sdc` in the native app; `check:project-shortcuts` proves that one `Ctrl+S` gesture dispatches only Save. |
| Reopen a named project | 3 ops (`Ctrl+O`, path, `Enter`) | 3 ops (`Ctrl+O`, path, `Enter`) | Same | Daslight reopened the scratch `.dvc` with its groups/scenes restored. Syndocal reopened the scratch `.sdc` with its Touch surface restored and reported `プロジェクトを読み込みました`; `check:project-shortcuts` proves that the first gesture dispatches only Open. |

## Honest boundary

Fourteen of the fifteen executable tasks now have direct, same-task Daslight
counts: thirteen are equal and one (`Add Touch button control`) requires one
fewer operation in Syndocal. The remaining task (`Reset all scene live
modifiers`) has no matching one-action Daslight control and is a measured
Syndocal usability advantage.

Windows Computer Use delivered text and Enter to the native file picker but did
not deliver modifier shortcuts into the WebView2 content. Therefore the
Syndocal evidence is deliberately split without inventing a result: native menu
Save/Open proves actual `.sdc` write/read persistence, while the executable
`check:project-shortcuts` gate proves the one-key `Ctrl+S` and `Ctrl+O`
dispatch routes. Daslight accepted the modifier shortcuts directly.

This audit measures operator gestures and visible state changes. It does not
replace physical-fixture output acceptance or a commercial visualizer render
comparison.

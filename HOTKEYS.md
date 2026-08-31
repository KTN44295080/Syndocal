# Syndocal Hotkeys

入力欄、select、textareaを編集中は、プロジェクト操作を除くグローバルキーを無効化します。

## Global

| Key | Action |
| --- | --- |
| `Ctrl/Cmd+N` | New project |
| `Ctrl/Cmd+O` | Load project |
| `Ctrl/Cmd+S` | Save project |
| `Ctrl/Cmd+Shift+S` | Save As |
| `Ctrl/Cmd+Z` | Undo the latest authoritative project edit |
| `Ctrl+Y` / `Ctrl/Cmd+Shift+Z` | Redo the latest authoritative project edit |
| `F1` / `F2` / `F3` | Setup / Control / Touch |

Setupでは`Alt+1`〜`Alt+9`でLibrary、Profiles、Patch、Outputs、Stage Map、DMX、MIDI、OSC、Remoteへ移動します。

## Control

| Key | Action |
| --- | --- |
| `E` / `L` / `M` | Live Edit / Timeline / Mixer |
| `Space` / `Shift+Space` | Next cue / Previous cue |
| `G` or `Enter` | GO next cue |
| `1`〜`0` | Current cue bank 1〜10 |
| `P` | Pause/resume active cue fade |
| `T` | Timeline play/pause |
| `B` | Lighting blackout |
| `V` | Video blackout |
| `K` | Tap BPM |

## Control / Timeline

| Input | Action |
| --- | --- |
| Mouse wheel | Scroll Timeline lanes vertically |
| `Shift` + mouse wheel | Pan the visible Timeline time range horizontally |
| Horizontal trackpad wheel | Pan the visible Timeline time range horizontally |
| `Ctrl/Cmd` + mouse wheel | Zoom around the pointer position |

New Timeline sessions start with a logical `Grid 500 ms` snap and Scene-edge
magnet enabled. The ruler subdivisions adapt visually as the Timeline is zoomed;
they are display guides and do not silently change the configured logical Grid
interval. `Alt` remains reserved for isolated linked-item editing and is not a
Timeline wheel modifier.

## Setup / Stage Map

`?`でアプリ内の完全なMapping Shortcutsを表示できます。

| Key | Action |
| --- | --- |
| `S` / `P` / `R` / `H` | Select / Place / Rotate / Pan |
| `F` / `Shift+F` | Fit visible / Fit selection |
| `+` / `-` / `0` | Zoom in / Zoom out / Reset view |
| `L B G V O` | Labels / Beams / Geometry / Projectors / Objects |
| `Ctrl/Cmd+A` | Pick visible fixtures |
| `Ctrl/Cmd+Shift+A` | Clear pick |
| `I` / `Shift+I` | Pick / add fixtures inside object |
| Arrow / `Shift+Arrow` | Nudge by snap / 5x snap |
| `Ctrl/Cmd+D` | Duplicate pick |
| `Delete` | Remove pick |
| `1` / `2` / `3` | Line / Grid / Circle |
| `Shift+1` / `Shift+2` | Line / Grid on stage object |
| `X` / `Z` | Align X / Z |
| `Shift+X` / `Shift+Z` | Distribute X / Z |
| `[` / `]` | Rotate -15 / +15 degrees |
| `Shift+[` / `Shift+]` | Mirror X / Z |
| `4` | Rotate 180 degrees |
| `Q` / `W` / `E` | Highlight / Solo / Park |
| `Esc` | Close help / cancel active map tool |

Projector mapping, video transform/crop/playback, color, Pan/Tilt、各数値エディタはフォーカス時にArrowキーで微調整でき、`Shift`または`Alt`でステップ幅が変わるコントロールがあります。

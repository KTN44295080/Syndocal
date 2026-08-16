from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def replace_all(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count == 0:
        raise RuntimeError(f"{label}: expected at least one match")
    path.write_text(text.replace(old, new), encoding="utf-8")


app = Path("app/src/App.tsx")
main = Path("app/src-tauri/src/main.rs")

replace_exact(
    app,
    '''  const setAllBlackout = async (enabled: boolean) => {
    try {
      if (!enabled) {
        setMessage("All blackout clear requires physical DMX Blackout Release in Runtime controls.");
        return;
      }
      await invoke("set_all_blackout", { enabled: true });
      setMessage("All blackout enabled.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };
''',
    '''  const setAllBlackout = async (enabled: boolean) => {
    try {
      if (!enabled) {
        setMessage("All blackout clear requires physical DMX Blackout Release in Runtime controls.");
        return;
      }
      // Combined blackout must use the same sticky S0 DMX latch as the
      // dedicated DMX button. Otherwise the R4 Release control could succeed
      // as a no-op while a separate legacy blackout remained asserted.
      await safetyBlackoutRuntime.engage();
      await setVideoBlackout(true);
      await refreshSnapshot();
      if (!snapshot().blackout || !snapshot().video.blackout) {
        throw new Error("All blackout did not reach the combined DMX/video safe state.");
      }
      setMessage("All blackout enabled.");
    } catch (error) {
      setMessage(String(error));
    }
  };
''',
    "All Blackout S0 convergence",
)

# Legacy local Tauri commands remain in the inventory during migration, but
# their energizing direction must not be an alternate R4 source. Safer-direction
# engagement stays compatible; release fails closed and names the canonical path.
replace_exact(
    main,
    '''#[tauri::command]
fn set_blackout(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::Blackout(enabled))
        .map_err(|error| error.to_string())
}
''',
    '''#[tauri::command]
fn set_blackout(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    if !enabled {
        return Err(
            "Legacy DMX blackout release is disabled; use the physical R4 Blackout Release control"
                .to_string(),
        );
    }
    state
        .engine
        .send(EngineCommand::Blackout(true))
        .map_err(|error| error.to_string())
}
''',
    "legacy Tauri DMX release guard",
)
replace_exact(
    main,
    '''#[tauri::command]
fn set_all_blackout(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state
        .engine
        .send(EngineCommand::SetAllBlackout(enabled))
        .map_err(|error| error.to_string())
}
''',
    '''#[tauri::command]
fn set_all_blackout(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    if !enabled {
        return Err(
            "Legacy All Blackout clear is disabled; release DMX through the physical R4 control and clear video separately"
                .to_string(),
        );
    }
    state
        .engine
        .send(EngineCommand::SetAllBlackout(true))
        .map_err(|error| error.to_string())
}
''',
    "legacy Tauri All Blackout release guard",
)

# MIDI, OSC and Remote WebSocket are legacy adapters over the same engine. They
# may still move toward the safer blackout-on state during migration, but they
# must never clear DMX/All blackout without the local prepared-consent R4 lane.
replace_all(
    main,
    '''            MidiControlEvent::Blackout(enabled) => EngineCommand::Blackout(enabled),
            MidiControlEvent::AllBlackout(enabled) => EngineCommand::SetAllBlackout(enabled),
''',
    '''            MidiControlEvent::Blackout(false) | MidiControlEvent::AllBlackout(false) => {
                eprintln!("Ignoring MIDI blackout release; use local physical R4 Blackout Release");
                return;
            }
            MidiControlEvent::Blackout(true) => EngineCommand::Blackout(true),
            MidiControlEvent::AllBlackout(true) => EngineCommand::SetAllBlackout(true),
''',
    "MIDI legacy blackout release guards",
)
replace_all(
    main,
    '''        OscInputEvent::Blackout(enabled) => EngineCommand::Blackout(enabled),
        OscInputEvent::AllBlackout(enabled) => EngineCommand::SetAllBlackout(enabled),
''',
    '''        OscInputEvent::Blackout(false) | OscInputEvent::AllBlackout(false) => {
            eprintln!("Ignoring {source} blackout release; use local physical R4 Blackout Release");
            return;
        }
        OscInputEvent::Blackout(true) => EngineCommand::Blackout(true),
        OscInputEvent::AllBlackout(true) => EngineCommand::SetAllBlackout(true),
''',
    "OSC legacy blackout release guards",
)
replace_all(
    main,
    '''                        RemoteInputEvent::Blackout(enabled) => EngineCommand::Blackout(enabled),
                        RemoteInputEvent::AllBlackout(enabled) => {
                            EngineCommand::SetAllBlackout(enabled)
                        }
''',
    '''                        RemoteInputEvent::Blackout(false)
                        | RemoteInputEvent::AllBlackout(false) => {
                            eprintln!(
                                "Ignoring remote blackout release; use local physical R4 Blackout Release"
                            );
                            return;
                        }
                        RemoteInputEvent::Blackout(true) => EngineCommand::Blackout(true),
                        RemoteInputEvent::AllBlackout(true) => {
                            EngineCommand::SetAllBlackout(true)
                        }
''',
    "Remote legacy blackout release guards",
)

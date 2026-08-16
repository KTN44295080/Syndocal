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

# The legacy target-valued blackout setters are incompatible with the split S0
# engage / R4 release model. Allowing even `true` here would assert the ordinary
# `blackout` bit, while R4 Release clears only the emergency safety latch; that
# can create a blackout which the canonical Release cannot clear. Keep these
# names inventory-visible but unavailable until their callers migrate.
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
fn set_blackout(_state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    Err(if enabled {
        "Legacy DMX blackout engage is disabled; use safety_blackout_engage_v1"
    } else {
        "Legacy DMX blackout release is disabled; use the physical R4 Blackout Release control"
    }
    .to_string())
}
''',
    "disable legacy target-valued DMX blackout setter",
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
fn set_all_blackout(_state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    Err(if enabled {
        "Legacy All Blackout is disabled; engage DMX through safety_blackout_engage_v1 and video blackout separately"
    } else {
        "Legacy All Blackout clear is disabled; release DMX through the physical R4 control and clear video separately"
    }
    .to_string())
}
''',
    "disable legacy target-valued All Blackout setter",
)

# MIDI, OSC, DMX-control mappings (which become OscInputEvent), and Remote
# WebSocket do not yet have the canonical principal/capability/request identity
# required by S0/R4. Drop both directions. Retaining their old `true` path would
# create the non-safety `blackout` bit described above; retaining `false` would
# bypass physical consent.
replace_all(
    main,
    '''            MidiControlEvent::Blackout(enabled) => EngineCommand::Blackout(enabled),
            MidiControlEvent::AllBlackout(enabled) => EngineCommand::SetAllBlackout(enabled),
''',
    '''            MidiControlEvent::Blackout(_) | MidiControlEvent::AllBlackout(_) => {
                eprintln!(
                    "Ignoring legacy MIDI blackout control until it is migrated to canonical S0/R4"
                );
                return;
            }
''',
    "disable legacy MIDI blackout setter",
)
replace_all(
    main,
    '''        OscInputEvent::Blackout(enabled) => EngineCommand::Blackout(enabled),
        OscInputEvent::AllBlackout(enabled) => EngineCommand::SetAllBlackout(enabled),
''',
    '''        OscInputEvent::Blackout(_) | OscInputEvent::AllBlackout(_) => {
            eprintln!(
                "Ignoring legacy {source} blackout control until it is migrated to canonical S0/R4"
            );
            return;
        }
''',
    "disable legacy OSC/DMX blackout setter",
)
replace_all(
    main,
    '''                        RemoteInputEvent::Blackout(enabled) => EngineCommand::Blackout(enabled),
                        RemoteInputEvent::AllBlackout(enabled) => {
                            EngineCommand::SetAllBlackout(enabled)
                        }
''',
    '''                        RemoteInputEvent::Blackout(_)
                        | RemoteInputEvent::AllBlackout(_) => {
                            eprintln!(
                                "Ignoring legacy remote blackout control until it is migrated to canonical S0/R4"
                            );
                            return;
                        }
''',
    "disable legacy Remote blackout setter",
)

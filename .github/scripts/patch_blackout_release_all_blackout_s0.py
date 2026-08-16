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
registry = Path("app/src-tauri/src/control_plane.rs")
invoke_manifest = Path("app/src/tauri-invoke-manifest.json")
invoke_types = Path("app/src/tauriInvokeCommands.ts")

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
# backend names inventory-visible but unavailable until their callers migrate.
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
# required by S0/R4. Drop both directions.
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

# These two unavailable compatibility handlers are no longer legitimate
# production renderer invoke authorities. Leave them in backend inventory for
# migration accounting, but remove them from the finite frontend command union.
for path in (invoke_manifest, invoke_types):
    replace_exact(path, '  "set_all_blackout",\n', '', f"remove set_all_blackout from {path}")
    replace_exact(path, '  "set_blackout",\n', '', f"remove set_blackout from {path}")

# Removing two frontend alias sources changes only frontend and aggregate source
# counts. The strict Release wire is rename-in-place, so Tauri/Engine/unclassified
# counts remain at their current actual values.
replace_exact(
    registry,
    "        const FRONTEND_INVOKE_COUNT: usize = 393;",
    "        const FRONTEND_INVOKE_COUNT: usize = 391;",
    "frontend invoke inventory after legacy blackout removal",
)
replace_exact(
    registry,
    "        assert_eq!(registry.operations.len(), 1414);",
    "        assert_eq!(registry.operations.len(), 1412);",
    "legacy registry total after frontend blackout removal",
)
replace_exact(
    registry,
    "        const FRONTEND_COUNT: usize = 393;",
    "        const FRONTEND_COUNT: usize = 391;",
    "canonical frontend alias count",
)
replace_exact(
    registry,
    "        assert_eq!(LEGACY_SOURCE_TOTAL, 1414);",
    "        assert_eq!(LEGACY_SOURCE_TOTAL, 1412);",
    "canonical legacy source total after frontend blackout removal",
)
replace_exact(
    registry,
    "        assert_eq!(SOURCE_TOTAL, 1447);",
    "        assert_eq!(SOURCE_TOTAL, 1445);",
    "canonical complete source total after frontend blackout removal",
)
replace_exact(
    registry,
    "        assert_eq!(legacy.operations.len(), 1414);",
    "        assert_eq!(legacy.operations.len(), 1412);",
    "legacy registry JSON source total after frontend blackout removal",
)
replace_exact(
    registry,
    "        assert_eq!(operations.len(), 1414);",
    "        assert_eq!(operations.len(), 1412);",
    "legacy registry JSON encoded total after frontend blackout removal",
)

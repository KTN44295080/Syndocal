from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


control = Path("app/src/components/BlackoutReleaseControl.tsx")
panel = Path("app/src/components/LightingRuntimeControlsPanel.tsx")
app = Path("app/src/App.tsx")

replace_exact(
    control,
    '''type BlackoutReleaseControlProps = {
  onReleased: () => void | Promise<void>;
};
''',
    '''type BlackoutReleaseControlProps = {
  disabled?: boolean;
  onReleased: () => void | Promise<void>;
};
''',
    "BlackoutReleaseControl disabled prop",
)
replace_exact(
    control,
    '''        aria-haspopup="dialog"
        disabled={busy()}
        onClick={() => void begin()}
''',
    '''        aria-haspopup="dialog"
        disabled={busy() || props.disabled}
        title={props.disabled ? "DMX Blackout Release is blocked by Operator Full Lock" : "DMX Blackout Release"}
        onClick={() => void begin()}
''',
    "BlackoutReleaseControl Full Lock disabled state",
)

replace_exact(
    panel,
    '''  midiConnected: boolean;
  onLightingMaster: (level: number) => void | Promise<void>;
''',
    '''  midiConnected: boolean;
  blackoutReleaseDisabled: boolean;
  onLightingMaster: (level: number) => void | Promise<void>;
''',
    "Lighting Runtime Release disabled prop",
)
replace_exact(
    panel,
    '''        <BlackoutReleaseControl onReleased={props.onBlackoutReleased} />
''',
    '''        <BlackoutReleaseControl
          disabled={props.blackoutReleaseDisabled}
          onReleased={props.onBlackoutReleased}
        />
''',
    "Lighting Runtime forwards Full Lock state",
)

replace_exact(
    app,
    '''            midiConnected={midiConnected()}
            onLightingMaster={setLightingMaster}
''',
    '''            midiConnected={midiConnected()}
            blackoutReleaseDisabled={operatorLockMode() === "Full"}
            onLightingMaster={setLightingMaster}
''',
    "App binds Full Lock to Release UI",
)

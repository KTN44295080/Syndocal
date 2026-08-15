from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


workspace = Path("app/src/components/WorkspaceChrome.tsx")
runtime_panel = Path("app/src/components/LightingRuntimeControlsPanel.tsx")

# The global DMX button is safety-direction only. Once blackout is engaged it
# must not advertise or dispatch a release; the only release UI is the R4
# physical-confirmation control in Runtime.
replace_exact(
    workspace,
    '''            <button
              type="button"
              class={`topbarIconButton topbarSafetyButton${props.blackout ? " engaged" : ""}`}
              data-global-operator-action="dmx-blackout"
              title={props.blackout ? "Clear DMX Blackout" : "DMX Blackout"}
              aria-label={props.blackout ? "Clear DMX Blackout" : "DMX Blackout"}
              aria-pressed={props.blackout}
              onClick={() => props.onSetBlackout(!props.blackout)}
              {...controlMappingTargetData({ action: "Blackout", label: "DMX Blackout" })}
            >
              <span data-no-localize>DMX</span>
            </button>
''',
    '''            <button
              type="button"
              class={`topbarIconButton topbarSafetyButton${props.blackout ? " engaged" : ""}`}
              data-global-operator-action="dmx-blackout"
              title={props.blackout ? "DMX Blackout engaged — release in Runtime controls" : "DMX Blackout"}
              aria-label={props.blackout ? "DMX Blackout engaged — release in Runtime controls" : "DMX Blackout"}
              aria-pressed={props.blackout}
              disabled={props.blackout}
              onClick={() => props.onSetBlackout(true)}
            >
              <span data-no-localize>DMX</span>
            </button>
''',
    "topbar DMX blackout is engage-only",
)

# Combined All Clear is intentionally removed as a release surface. When both
# outputs are already blacked out, the button stays latched/disabled. Video may
# be cleared with its own control; DMX safety blackout requires R4 consent.
replace_exact(
    workspace,
    '''            <button
              type="button"
              class={`topbarIconButton topbarSafetyButton${props.blackout && props.videoBlackout ? " engaged" : ""}`}
              data-global-operator-action="all-blackout"
              title={props.blackout && props.videoBlackout ? "Clear All Blackout" : "All Blackout"}
              aria-label={props.blackout && props.videoBlackout ? "Clear All Blackout" : "All Blackout"}
              aria-pressed={props.blackout && props.videoBlackout}
              onClick={() => props.onSetAllBlackout(!(props.blackout && props.videoBlackout))}
              {...controlMappingTargetData({ action: "AllBlackout", label: "All Blackout" })}
            >
              <span data-no-localize>ALL</span>
            </button>
''',
    '''            <button
              type="button"
              class={`topbarIconButton topbarSafetyButton${props.blackout && props.videoBlackout ? " engaged" : ""}`}
              data-global-operator-action="all-blackout"
              title={props.blackout && props.videoBlackout ? "All Blackout engaged — release DMX in Runtime controls" : "All Blackout"}
              aria-label={props.blackout && props.videoBlackout ? "All Blackout engaged — release DMX in Runtime controls" : "All Blackout"}
              aria-pressed={props.blackout && props.videoBlackout}
              disabled={props.blackout && props.videoBlackout}
              onClick={() => props.onSetAllBlackout(true)}
            >
              <span data-no-localize>ALL</span>
            </button>
''',
    "topbar All Blackout is engage-only",
)

# Runtime keeps the explicit R4 DMX Clear control. Do not leave a neighboring
# legacy All Clear button which suggests a second unconfirmed release route.
replace_exact(
    runtime_panel,
    '''        <button onClick={() => void props.onAllBlackout(true)}>All BO</button>
        <button onClick={() => void props.onAllBlackout(false)}>All Clear</button>
''',
    '''        <button onClick={() => void props.onAllBlackout(true)}>All BO</button>
''',
    "remove Runtime legacy All Clear button",
)

interface ControlFaderWriteHeaderProps {
  targetKind: "fixture" | "group" | "selection" | "empty";
  compactReadout: string;
  writeMode: "live" | "edit";
  editingSceneLabel: string | null;
  editingSceneIdentity: string | null;
  editingSceneIdentityText: string | null;
  onWriteMode: (mode: "live" | "edit") => void;
}

export function ControlFaderWriteHeader(props: ControlFaderWriteHeaderProps) {
  return (
    <div
      class={`controlFaderWriteHeader ${props.writeMode}`}
      data-control-fader-write-header
      data-control-fader-write-mode={props.writeMode}
      data-control-fader-target-kind={props.targetKind}
      style={{
        "--control-edit-identity": props.editingSceneIdentity ?? "#4dbb78",
        "--control-edit-identity-text": props.editingSceneIdentityText ?? "#b8e8ca",
      }}
    >
      <div
        class="controlFaderWriteMode"
        role="group"
        aria-label="Fader write mode"
      >
        <button
          type="button"
          classList={{ active: props.writeMode === "live" }}
          aria-label="Use LIVE fader mode"
          aria-pressed={props.writeMode === "live"}
          data-control-fader-write-mode-option="live"
          onClick={() => props.onWriteMode("live")}
        >
          <span data-no-localize>LIVE</span>
        </button>
        <button
          type="button"
          classList={{ active: props.writeMode === "edit" }}
          aria-label="Use EDIT scene-write mode"
          aria-pressed={props.writeMode === "edit"}
          data-control-fader-write-mode-option="edit"
          onClick={() => props.onWriteMode("edit")}
        >
          <span data-no-localize>EDIT</span>
        </button>
      </div>
      <div
        class="controlFaderEditState"
        classList={{ unselected: !props.editingSceneLabel }}
        data-control-fader-edit-state={props.editingSceneLabel ? "selected" : "unselected"}
        aria-live="polite"
      >
        <strong data-no-localize>EDIT:</strong>
        {props.editingSceneLabel
          ? <span data-no-localize>{props.editingSceneLabel}</span>
          : <span>No scene selected</span>}
      </div>
      <output
        class="controlFaderCompactReadout tabularNums"
        data-control-fader-compact-readout
        title={props.compactReadout}
      >
        {props.compactReadout}
      </output>
    </div>
  );
}

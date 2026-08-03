import { Show } from "solid-js";

interface ControlFaderWriteHeaderProps {
  targetKind: "fixture" | "group" | "selection" | "empty";
  compactReadout: string;
  writeMode: "live" | "edit";
  editingSceneLabel: string | null;
  editingSceneIdentity: string | null;
  editingSceneIdentityText: string | null;
  blindActive: boolean;
  blindStagedCount: number;
  onWriteMode: (mode: "live" | "edit") => void;
  onBlindToggle: (enabled: boolean) => void | Promise<void>;
  onBlindCommit: () => void | Promise<void>;
  onBlindDiscard: () => void | Promise<void>;
}

export function ControlFaderWriteHeader(props: ControlFaderWriteHeaderProps) {
  let blindDiscardDialog!: HTMLDialogElement;
  const blindToggleLabel = () => props.blindActive ? "Commit and exit Blind editing" : "Enable Blind editing";

  return (
    <div
      class={`controlFaderWriteHeader ${props.writeMode}${props.blindActive ? " blindActive" : ""}`}
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
          classList={{ active: props.writeMode === "edit" }}
          aria-label="Use EDIT scene-write mode"
          aria-pressed={props.writeMode === "edit"}
          data-control-fader-write-mode-option="edit"
          onClick={() => props.onWriteMode("edit")}
        >
          <span data-no-localize>EDIT</span>
        </button>
        <button
          type="button"
          classList={{ active: props.writeMode === "live" }}
          aria-label="Use LIVE fader mode"
          aria-pressed={props.writeMode === "live"}
          disabled={props.blindActive}
          data-control-fader-write-mode-option="live"
          onClick={() => props.onWriteMode("live")}
        >
          <span data-no-localize>LIVE</span>
        </button>
      </div>
      <div class="controlBlindControls" role="group" aria-label="Blind edit controls">
        <button
          type="button"
          class="controlBlindToggle"
          classList={{ active: props.blindActive }}
          data-control-blind-toggle
          title={blindToggleLabel()}
          aria-label={blindToggleLabel()}
          aria-pressed={props.blindActive}
          disabled={!props.blindActive && (props.writeMode !== "edit" || !props.editingSceneLabel)}
          onClick={() => void props.onBlindToggle(!props.blindActive)}
        >
          <svg viewBox="0 0 18 18" aria-hidden="true">
            <path d="M1.5 9s2.8-4.5 7.5-4.5S16.5 9 16.5 9 13.7 13.5 9 13.5 1.5 9 1.5 9Z" />
            <circle cx="9" cy="9" r="2.25" />
          </svg>
        </button>
        <Show when={props.blindActive}>
          <button
            type="button"
            class="controlBlindCommit"
            data-control-blind-commit
            title="Commit Blind edit"
            aria-label="Commit Blind edit"
            disabled={props.blindStagedCount === 0}
            onClick={() => void props.onBlindCommit()}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3 8 3 3 7-7" /></svg>
          </button>
          <button
            type="button"
            class="controlBlindDiscard"
            data-control-blind-discard
            title="Discard Blind edit"
            aria-label="Discard Blind edit"
            disabled={props.blindStagedCount === 0}
            onClick={() => blindDiscardDialog.showModal()}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" /></svg>
          </button>
        </Show>
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
      <dialog
        ref={blindDiscardDialog}
        class="programmerClearDialog"
        aria-labelledby="control-blind-discard-title"
        data-control-blind-discard-dialog
      >
        <form method="dialog">
          <h2 id="control-blind-discard-title">Discard Blind changes?</h2>
          <p class="textPretty">This discards all Blind edits without changing live DMX output.</p>
          <div class="buttonRow">
            <button value="cancel">Keep Editing</button>
            <button
              type="button"
              class="danger"
              data-control-blind-discard-confirm
              onClick={() => {
                blindDiscardDialog.close();
                void props.onBlindDiscard();
              }}
            >
              Discard Changes
            </button>
          </div>
        </form>
      </dialog>
    </div>
  );
}

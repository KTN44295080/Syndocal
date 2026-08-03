import type { ProgrammerSnapshot } from "../types";

interface ProgrammerPanelProps {
  programmer: ProgrammerSnapshot;
  onSetMode: (enabled: boolean, blind: boolean) => void | Promise<void>;
  onCommit: () => void | Promise<void>;
  onClear: () => void | Promise<void>;
}

export function ProgrammerPanel(props: ProgrammerPanelProps) {
  let clearDialog!: HTMLDialogElement;

  return (
    <section class={`programmerPanel ${props.programmer.enabled ? "enabled" : ""} ${props.programmer.blind ? "blind" : ""}`} aria-label="Lighting programmer">
      <div class="programmerSummary">
        <strong>Programmer</strong>
        <span class="tabularNums">{props.programmer.values.length} staged · {props.programmer.dmx_previews.length} preview universe(s)</span>
        <span class={`status ${props.programmer.blind ? "warn" : props.programmer.enabled ? "good" : ""}`}>
          {props.programmer.blind ? "BLIND" : props.programmer.enabled ? "LIVE PREVIEW" : "DIRECT"}
        </span>
      </div>
      <div class="programmerActions">
        <label class="checkbox">
          <input
            type="checkbox"
            checked={props.programmer.enabled}
            onChange={(event) => void props.onSetMode(event.currentTarget.checked, event.currentTarget.checked && props.programmer.blind)}
          />
          Stage edits
        </label>
        <label class="checkbox">
          <input
            type="checkbox"
            disabled={!props.programmer.enabled}
            checked={props.programmer.blind}
            onChange={(event) => void props.onSetMode(true, event.currentTarget.checked)}
          />
          Blind
        </label>
        <button class="primary" disabled={props.programmer.values.length === 0} onClick={() => void props.onCommit()}>Commit</button>
        <button disabled={props.programmer.values.length === 0} onClick={() => clearDialog.showModal()}>Clear</button>
      </div>
      <p class="hint">Stage edits routes faders to the Programmer. Live Preview reaches DMX; Blind changes only the editor preview. Commit updates the edited scene; live output changes only when that scene is active.</p>
      <dialog ref={clearDialog} class="programmerClearDialog" aria-labelledby="programmer-clear-title">
        <form method="dialog">
          <h2 id="programmer-clear-title">Clear Programmer?</h2>
          <p class="textPretty">This discards all staged values without changing the current live base state.</p>
          <div class="buttonRow">
            <button value="cancel">Cancel</button>
            <button type="button" class="danger" onClick={() => { clearDialog.close(); void props.onClear(); }}>Clear Staged Values</button>
          </div>
        </form>
      </dialog>
    </section>
  );
}

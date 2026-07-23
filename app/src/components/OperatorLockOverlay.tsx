import { createSignal, Show } from "solid-js";
import type { OperatorLockMode } from "../types";

type OperatorLockOverlayProps = {
  mode: OperatorLockMode;
  restrictedPane?: boolean;
  blackout: boolean;
  videoBlackout: boolean;
  onSetBlackout: (enabled: boolean) => void;
  onSetAllBlackout: (enabled: boolean) => void;
  onUnlock: (password: string) => Promise<boolean>;
};

export function OperatorLockOverlay(props: OperatorLockOverlayProps) {
  const [password, setPassword] = createSignal("");
  const [busy, setBusy] = createSignal(false);

  const unlock = async () => {
    setBusy(true);
    try {
      if (await props.onUnlock(password())) setPassword("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div class="operatorLockOverlay" role="dialog" aria-modal="true" aria-label={`${props.mode} operator lock`}>
      <div class="operatorLockCard">
        <div class="operatorLockMark" aria-hidden="true">LOCK</div>
        <h1>{props.mode} Operator Lock</h1>
        <p>
          {props.restrictedPane
            ? "This programming pane is unavailable while Partial Lock is active. Live, Mixer, and Touch remain available."
            : "Show output continues. Programming and project controls are closed until an operator unlocks the desk."}
        </p>
        <Show when={props.mode === "Full" && !props.restrictedPane}>
          <div class="operatorEmergencyDeck" aria-label="Emergency blackout controls">
            <button
              type="button"
              class={props.blackout ? "danger active" : ""}
              aria-pressed={props.blackout}
              onClick={() => props.onSetBlackout(!props.blackout)}
            >
              {props.blackout ? "DMX BLACKOUT ON" : "DMX BLACKOUT"}
            </button>
            <button
              type="button"
              class={props.videoBlackout ? "danger active" : ""}
              aria-pressed={props.videoBlackout}
              onClick={() => props.onSetAllBlackout(true)}
            >
              ALL BLACKOUT
            </button>
            <button type="button" class="safe" onClick={() => props.onSetAllBlackout(false)}>
              ALL CLEAR
            </button>
          </div>
        </Show>
        <form
          class="operatorUnlockForm"
          onSubmit={(event) => {
            event.preventDefault();
            void unlock();
          }}
        >
          <label for="operator-lock-password">Operator password</label>
          <input
            id="operator-lock-password"
            type="password"
            autocomplete="current-password"
            autofocus
            value={password()}
            onInput={(event) => setPassword(event.currentTarget.value)}
          />
          <button type="submit" disabled={busy() || password().length === 0}>
            {busy() ? "Checking…" : "Unlock"}
          </button>
        </form>
        <small>PBKDF2-SHA256 verifier · no plaintext password stored</small>
      </div>
    </div>
  );
}

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { createSignal, onCleanup, Show } from "solid-js";
import {
  blackoutReleaseInvokeCommands,
  createBlackoutReleaseRuntimeController,
  type BlackoutReleaseConsentState,
  type BlackoutReleaseInvoke,
  type BlackoutReleaseOutcome,
  type BlackoutReleasePreparedConsent,
} from "../blackoutReleaseRuntimeController";

const releaseCommands = new Set<string>(blackoutReleaseInvokeCommands);
const invokeBlackoutRelease: BlackoutReleaseInvoke = async <T,>(command, args) => {
  if (!releaseCommands.has(command)) {
    throw new Error("Blackout release dispatcher rejected an unknown command.");
  }
  return tauriInvoke<T>(command, args);
};

const releaseRuntime = createBlackoutReleaseRuntimeController({
  invoke: invokeBlackoutRelease,
});

const sleep = (durationMs: number) => new Promise<void>((resolve) => {
  window.setTimeout(resolve, durationMs);
});

type BlackoutReleaseControlProps = {
  onReleased: () => void | Promise<void>;
};

export function BlackoutReleaseControl(props: BlackoutReleaseControlProps) {
  const [prepared, setPrepared] = createSignal<BlackoutReleasePreparedConsent | null>(null);
  const [consentState, setConsentState] = createSignal<BlackoutReleaseConsentState>(
    "pending_physical_input",
  );
  const [outcome, setOutcome] = createSignal<BlackoutReleaseOutcome | null>(null);
  const [error, setError] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  let generation = 0;

  const close = () => {
    generation += 1;
    setPrepared(null);
    setConsentState("pending_physical_input");
    setOutcome(null);
    setError("");
    setBusy(false);
  };

  onCleanup(() => {
    generation += 1;
  });

  const pollAndRelease = async (
    consent: BlackoutReleasePreparedConsent,
    currentGeneration: number,
  ) => {
    try {
      while (currentGeneration === generation) {
        if (Date.now() >= consent.challenge.expires_at_unix_ms) {
          throw new Error("Physical confirmation expired. Start a new blackout release request.");
        }
        const state = await releaseRuntime.consentStatus(consent);
        if (currentGeneration !== generation) return;
        setConsentState(state);
        if (state === "ready") {
          const receipt = await releaseRuntime.release(consent);
          if (currentGeneration !== generation) return;
          setOutcome(receipt.outcome);
          setPrepared(null);
          setBusy(false);
          await props.onReleased();
          return;
        }
        await sleep(200);
      }
    } catch (pollError) {
      if (currentGeneration !== generation) return;
      setError(String(pollError));
      setBusy(false);
    }
  };

  const begin = async () => {
    if (busy()) return;
    const currentGeneration = ++generation;
    setBusy(true);
    setPrepared(null);
    setConsentState("pending_physical_input");
    setOutcome(null);
    setError("");
    try {
      const consent = await releaseRuntime.prepareRelease();
      if (currentGeneration !== generation) return;
      setPrepared(consent);
      void pollAndRelease(consent, currentGeneration);
    } catch (prepareError) {
      if (currentGeneration !== generation) return;
      setError(String(prepareError));
      setBusy(false);
    }
  };

  const dialogVisible = () => prepared() !== null || outcome() !== null || error().length > 0;

  return (
    <>
      <button
        type="button"
        class="safe"
        aria-haspopup="dialog"
        disabled={busy()}
        onClick={() => void begin()}
      >
        {busy() ? "DMX Clear · Confirming…" : "DMX Clear"}
      </button>
      <Show when={dialogVisible()}>
        <div
          class="operatorLockOverlay"
          role="dialog"
          aria-modal="true"
          aria-label="Blackout release confirmation"
          data-block-global-shortcuts="true"
        >
          <div class="operatorLockCard">
            <div class="operatorLockMark" aria-hidden="true">R4</div>
            <h1>Release DMX blackout</h1>
            <Show when={prepared()} keyed>
              {(consent) => (
                <>
                  <p>
                    Type this six-digit challenge on a physical keyboard connected to this machine.
                    Pasting, clicking, remote input, and automation do not confirm the release.
                  </p>
                  <strong class="operatorConsentCode" data-no-localize>
                    {consent.challenge.display_code}
                  </strong>
                  <p role="status">
                    {consentState() === "ready"
                      ? "Physical input accepted. Releasing blackout…"
                      : "Waiting for matching physical keyboard input…"}
                  </p>
                  <small>This single-use request expires automatically.</small>
                </>
              )}
            </Show>
            <Show when={outcome()} keyed>
              {(result) => (
                <p role="status">
                  {result === "applied"
                    ? "DMX blackout was released."
                    : "DMX blackout was already released."}
                </p>
              )}
            </Show>
            <Show when={error().length > 0}>
              <p role="alert">{error()}</p>
            </Show>
            <div class="buttonRow">
              <button type="button" onClick={close}>
                Close
              </button>
            </div>
          </div>
        </div>
      </Show>
    </>
  );
}

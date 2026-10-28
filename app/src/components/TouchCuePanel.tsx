import { For, Show, onCleanup } from "solid-js";
import type { CueSummary, EngineSnapshot } from "../types";
import { cueIdentityCss } from "../identityColor";
import { authoredCueLiveModifier } from "../cueLiveModifier";
import { bankAuthorityIssueMessage, type FullBankAuthoritySnapshot } from "../bankAuthority";
import { touchSceneActive, touchSceneBanks } from "../touchScenePanel";
import "./TouchCuePanel.css";

type MaybePromise = void | Promise<void>;

interface TouchCuePanelProps {
  bankAuthority: FullBankAuthoritySnapshot;
  snapshot: EngineSnapshot;
  onTriggerCue: (cueId: number) => MaybePromise;
  onReleaseCue: (cueId: number) => MaybePromise;
}

/**
 * Control > Lighting Scene panel: every Scene grouped by Bank, scrolled inside
 * the panel. Press fires; pressing an active Scene releases it; Flash Scenes
 * are momentary, matching the Edit > Lighting Scene matrix.
 */
export function TouchCuePanel(props: TouchCuePanelProps) {
  const heldFlashCues = new Set<number>();
  const releaseFlash = (cueId: number) => {
    if (!heldFlashCues.delete(cueId)) return;
    // Safety-lowering half of an activation already issued; never gated.
    void props.onReleaseCue(cueId);
  };
  const beginFlash = (cueId: number) => {
    if (heldFlashCues.has(cueId)) return;
    heldFlashCues.add(cueId);
    void props.onTriggerCue(cueId);
  };
  onCleanup(() => {
    for (const cueId of [...heldFlashCues]) releaseFlash(cueId);
  });
  const banks = () => touchSceneBanks(props.bankAuthority);

  return (
    <section class="touchScenePanel" aria-label="Scenes" data-touch-scene-panel>
      <Show
        when={props.bankAuthority.issue === null}
        fallback={
          <p class="empty" role="alert" data-bank-authority-unavailable={props.bankAuthority.issue?.kind}>
            {props.bankAuthority.issue ? bankAuthorityIssueMessage(props.bankAuthority.issue) : ""}
          </p>
        }
      >
        <Show when={banks().length > 0} fallback={<p class="empty">No scenes. Store one in Edit &gt; Lighting.</p>}>
          <For each={banks()}>
            {(entry) => (
              <section class="touchSceneBank" aria-label={entry.bank.label} data-touch-scene-bank={entry.bank.id}>
                <h3 data-no-localize>{entry.bank.label}</h3>
                <div class="touchSceneButtons">
                  <For each={entry.cues}>
                    {(cue: CueSummary) => {
                      const flash = () => authoredCueLiveModifier(cue).flash;
                      const active = () => touchSceneActive(cue, props.snapshot);
                      return (
                        <button
                          type="button"
                          class="touchSceneButton"
                          classList={{ active: active(), flash: flash() }}
                          aria-pressed={active()}
                          data-touch-scene={cue.id}
                          title={cue.label}
                          style={{
                            "--identity": cueIdentityCss(
                              cue.id,
                              cue.color,
                              "text",
                              cue.group_id,
                              cue.group_id ? props.snapshot.group_colors?.[cue.group_id] : null,
                            ),
                          }}
                          onPointerDown={(event) => {
                            if (!flash()) return;
                            event.preventDefault();
                            event.currentTarget.setPointerCapture(event.pointerId);
                            beginFlash(cue.id);
                          }}
                          onPointerUp={() => releaseFlash(cue.id)}
                          onPointerCancel={() => releaseFlash(cue.id)}
                          onKeyDown={(event) => {
                            if (!flash() || event.repeat || (event.key !== " " && event.key !== "Enter")) return;
                            event.preventDefault();
                            beginFlash(cue.id);
                          }}
                          onKeyUp={(event) => {
                            if (!flash() || (event.key !== " " && event.key !== "Enter")) return;
                            event.preventDefault();
                            releaseFlash(cue.id);
                          }}
                          onClick={(event) => {
                            if (flash()) {
                              event.preventDefault();
                              return;
                            }
                            void (active() ? props.onReleaseCue(cue.id) : props.onTriggerCue(cue.id));
                          }}
                        >
                          <span data-no-localize>{cue.cue_number || cue.id}</span>
                          <strong data-no-localize>{cue.label}</strong>
                        </button>
                      );
                    }}
                  </For>
                </div>
              </section>
            )}
          </For>
        </Show>
      </Show>
    </section>
  );
}

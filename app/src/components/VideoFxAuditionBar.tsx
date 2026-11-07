import { Show } from "solid-js";
import type { FxAuditionSession } from "../createFxAuditionSession";
import { translateUiText } from "../uiLocalization";
import "./VideoFxAuditionBar.css";

const staleText = {
  "layer-missing": "The layer was removed. Discard this audition.",
  "preview-changed": "Preview now shows something else. Discard this audition and start again.",
  "live-changed": "This layer's live FX changed after the audition began. Discard it and start again.",
} as const;
const errorText = {
  "preview-unavailable": "This layer cannot be shown in Preview. Only local files, still images and generators can.",
  "audition-rejected": "Preview did not accept the effect chain. Your live output was not changed.",
  "take-failed": "The live layer did not take the new chain. Check the project status and try again.",
  "stage-limit": "A layer holds at most 8 effects. Remove one to add another.",
  "builtin-unavailable": "That effect could not be prepared for Preview.",
  "preview-busy": "Preview is busy and may still show the audition. Press Discard again.",
} as const;

export interface VideoFxAuditionBarProps {
  session?: FxAuditionSession;
  layerId: number | null;
  layerLabel?: string;
  locale?: string;
}

/** Preview-only audition controls. Program changes only through the explicit Take action. */
export function VideoFxAuditionBar(props: VideoFxAuditionBarProps) {
  const state = () => props.session?.active(props.layerId) ?? null;
  const locale = () => props.locale === "ja" ? "ja" : "en";
  const t = (text: string) => translateUiText(text, locale());
  const problem = () => {
    const value = state();
    const error = props.session?.error();
    if (value?.stale) return t(staleText[value.stale]);
    if (error) return t(errorText[error]);
    return "";
  };
  return (
    <Show when={props.session && props.layerId !== null}>
      <aside class="videoFxAudition" data-fx-audition-bar data-fx-audition-active={state() ? "true" : "false"} aria-label={t("Effect audition")}>
        <Show when={state()} fallback={
          <div class="videoFxAuditionIdle">
            <button type="button" data-fx-audition-begin disabled={props.session!.busy()} onClick={() => void props.session!.begin(props.layerId!)}>
              Try on Preview
            </button>
            <p>Edit a copy of this chain on Preview only. Program stays unchanged until you take it.</p>
            <Show when={problem()}><p class="videoFxAuditionProblem" role="alert" data-no-localize>{problem()}</p></Show>
          </div>
        }>
          {(value) => <>
            <div class="videoFxAuditionTitle">
              <strong>AUDITION · PREVIEW ONLY</strong>
              <span>Program is unchanged.</span>
            </div>
            <div class="videoFxAuditionToggle" role="group" aria-label={t("Compare on Preview")}>
              <button type="button" data-fx-audition-side="A" aria-pressed={value().side === "A"} disabled={Boolean(value().stale) || props.session!.busy()}
                onClick={() => void props.session!.show("A")}>A · Current</button>
              <button type="button" data-fx-audition-side="B" aria-pressed={value().side === "B"} disabled={Boolean(value().stale) || props.session!.busy()}
                onClick={() => void props.session!.show("B")}>B · Edited</button>
            </div>
            <div class="videoFxAuditionActions">
              <button type="button" data-fx-audition-undo disabled={!props.session!.canUndo()} onClick={() => void props.session!.undo()}>Undo audition edit</button>
              <button type="button" class="primary" data-fx-audition-take disabled={Boolean(value().stale) || props.session!.busy()}
                onClick={() => void props.session!.take()}>Take to Program</button>
              <button type="button" data-fx-audition-discard disabled={props.session!.busy()} onClick={() => void props.session!.discard()}>Discard</button>
            </div>
            <Show when={problem()}><p class="videoFxAuditionProblem" role="alert" data-no-localize>{problem()}</p></Show>
            <Show when={props.session!.reason()}><p class="videoFxAuditionReason" data-no-localize>{props.session!.reason()}</p></Show>
          </>}
        </Show>
      </aside>
    </Show>
  );
}

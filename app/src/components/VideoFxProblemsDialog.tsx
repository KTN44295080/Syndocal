import { For, Show } from "solid-js";
import type { FxProblem } from "../fxRejectionExplanation";
import "./VideoFxProblemsDialog.css";

export interface VideoFxProblemsDialogProps {
  problems: FxProblem[];
  locale?: string;
  onClose: () => void;
  ref?: (element: HTMLDialogElement) => void;
}

/** Plain-language explanation of every rejected or failing effect. The engine's
 * own text stays one disclosure away for experts. */
export function VideoFxProblemsDialog(props: VideoFxProblemsDialogProps) {
  const ja = () => props.locale === "ja";
  return (
    <dialog ref={props.ref} class="videoFxProblems" data-no-localize data-fx-problems-dialog aria-labelledby="video-fx-problems-title" onClose={props.onClose}>
      <header>
        <h2 id="video-fx-problems-title">{ja() ? "エフェクトの問題" : "Effect problems"}</h2>
        <button type="button" data-fx-problems-close onClick={(event) => event.currentTarget.closest("dialog")?.close()}>{ja() ? "閉じる" : "Close"}</button>
      </header>
      <Show when={props.problems.length > 0} fallback={<p class="videoFxProblemsEmpty">{ja() ? "現在、問題のあるエフェクトはありません。" : "No effect has a problem right now."}</p>}>
        <ul class="videoFxProblemList">
          <For each={props.problems}>{(problem) =>
            <li data-fx-problem={problem.origin}>
              <p class="videoFxProblemWhere">{problem.where}<span>{problem.origin === "runtime" ? (ja() ? "実行中のエラー" : "Runtime error") : (ja() ? "読み込みを拒否" : "Import rejected")}</span></p>
              <strong>{problem.explanation.title}</strong>
              <p>{problem.explanation.detail}</p>
              <p class="videoFxProblemAction"><span>{ja() ? "対処：" : "What to do: "}</span>{problem.explanation.action}</p>
              <details><summary>{ja() ? "技術的な詳細（上級者向け）" : "Technical details (for experts)"}</summary><pre>{problem.explanation.raw}</pre></details>
            </li>
          }</For>
        </ul>
      </Show>
    </dialog>
  );
}

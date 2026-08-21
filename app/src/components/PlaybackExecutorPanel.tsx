import { createMemo, createSignal, For, Show } from "solid-js";
import type { CueListSummary, CueSummary, PlaybackExecutorSummary } from "../types";

interface PlaybackExecutorPanelProps {
  executors: PlaybackExecutorSummary[];
  cueLists: CueListSummary[];
  cues: CueSummary[];
  playbackMaster: number;
  onCreate: (label: string, cueListId: number, page: number, slot: number) => void | Promise<void>;
  onUpdate: (executor: PlaybackExecutorSummary, patch: Partial<PlaybackExecutorSummary>) => void | Promise<void>;
  onRemove: (executor: PlaybackExecutorSummary) => void | Promise<void>;
  onSetLevel: (executorId: number, level: number) => void | Promise<void>;
  onSetMaster: (level: number) => void | Promise<void>;
  onTrigger: (executorId: number, direction: "next" | "previous") => void | Promise<void>;
}

const cueListDisplayLabel = (cueList: CueListSummary): string =>
  cueList.id === 1 && cueList.label.trim().toLowerCase() === "main"
    ? "Bank 1"
    : cueList.label;

export function PlaybackExecutorPanel(props: PlaybackExecutorPanelProps) {
  const [page, setPage] = createSignal(1);
  const [label, setLabel] = createSignal("Playback");
  const [cueListId, setCueListId] = createSignal(1);
  const displayCueLists = createMemo(() => props.cueLists.map((cueList) => ({
    ...cueList,
    label: cueListDisplayLabel(cueList),
  })));
  const pageExecutors = createMemo(() => props.executors.filter((executor) => executor.page === page()));
  const nextSlot = createMemo(() => {
    const used = new Set(pageExecutors().map((executor) => executor.slot));
    for (let slot = 1; slot <= 16; slot += 1) if (!used.has(slot)) return slot;
    return null;
  });

  const activeCueFor = (executor: PlaybackExecutorSummary) => {
    const cueList = props.cueLists.find((candidate) => candidate.id === executor.cue_list_id);
    return props.cues.find((cue) => cue.id === cueList?.active_cue_id);
  };

  return (
    <section class="playbackExecutorPanel" aria-label="Playback executors">
      <div class="programmerSummary">
        <strong>Playback</strong>
        <span>{props.executors.length} executor(s) · Page {page()}</span>
        <span class="status">HTP MASTERS</span>
      </div>
      <div class="playbackMasterControl">
        <label>
          Playback Master
          <input
            aria-label="Playback Master level"
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={props.playbackMaster}
            onInput={(event) => void props.onSetMaster(Number(event.currentTarget.value))}
          />
        </label>
        <output>{Math.round(props.playbackMaster * 100)}%</output>
        <label>
          Page
          <input type="number" min="1" max="99" value={page()} onInput={(event) => setPage(Math.max(1, Math.min(99, Math.round(Number(event.currentTarget.value) || 1))))} />
        </label>
      </div>
      <div class="playbackExecutorCreate">
        <label>Label<input maxlength="64" value={label()} onInput={(event) => setLabel(event.currentTarget.value)} /></label>
        <label>
          Cue List
          <select value={cueListId()} onInput={(event) => setCueListId(Number(event.currentTarget.value))}>
            <For each={displayCueLists()}>{(cueList) => <option data-no-localize value={cueList.id}>{cueList.label}</option>}</For>
          </select>
        </label>
        <button
          disabled={!label().trim() || nextSlot() === null}
          onClick={() => {
            const slot = nextSlot();
            if (slot !== null) void props.onCreate(label(), cueListId(), page(), slot);
          }}
        >
          Add Fader{nextSlot() === null ? " (Page Full)" : ` · Slot ${nextSlot()}`}
        </button>
      </div>
      <div class="playbackExecutorGrid">
        <Show when={pageExecutors().length > 0} fallback={<p class="empty">No faders on this page. Add a Cue List assignment above.</p>}>
          <For each={pageExecutors()}>
            {(executor) => {
              const activeCue = () => activeCueFor(executor);
              return (
                <article class="playbackExecutorStrip">
                  <div class="playbackExecutorHeading">
                    <b>{executor.slot}</b>
                    <input
                      aria-label={`Executor ${executor.slot} label`}
                      maxlength="64"
                      value={executor.label}
                      onChange={(event) => void props.onUpdate(executor, { label: event.currentTarget.value })}
                    />
                  </div>
                  <select
                    aria-label={`Executor ${executor.slot} Cue List`}
                    value={executor.cue_list_id}
                    onInput={(event) => void props.onUpdate(executor, { cue_list_id: Number(event.currentTarget.value) })}
                  >
                    <For each={displayCueLists()}>{(cueList) => <option data-no-localize value={cueList.id}>{cueList.label}</option>}</For>
                  </select>
                  <Show when={activeCue()} fallback={<span class="playbackExecutorNow">Ready</span>}>
                    {(cue) => <span class="playbackExecutorNow" data-no-localize>{cue().cue_number || cue().id} {cue().label}</span>}
                  </Show>
                  <label class="playbackExecutorFader">
                    Level
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.01"
                      value={executor.level}
                      onInput={(event) => void props.onSetLevel(executor.id, Number(event.currentTarget.value))}
                    />
                    <output>{Math.round(executor.level * 100)}%</output>
                  </label>
                  <div class="buttonRow">
                    <button onClick={() => void props.onTrigger(executor.id, "previous")}>Back</button>
                    <button class="primary" onClick={() => void props.onTrigger(executor.id, "next")}>GO</button>
                    <button class="danger" onClick={() => void props.onRemove(executor)}>Remove</button>
                  </div>
                </article>
              );
            }}
          </For>
        </Show>
      </div>
      <p class="hint">Faders assigned to the same Cue List share its Back/GO position. Their levels merge HTP; Playback Master scales all Cue-origin intensity without reducing Programmer overrides.</p>
    </section>
  );
}

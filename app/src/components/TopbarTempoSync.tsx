import { For } from "solid-js";
import { clockSourceLabel, clockSyncStatusLabel } from "../clockDisplay";
import type { EngineSnapshot } from "../types";
import { translateUiText, type UiLocale } from "../uiLocalization";

/**
 * Read-only companion of the global BPM and Tap controls: which clock drives
 * the tempo, whether it is locked, and the current beat within the bar.
 */
export function TopbarTempoSync(props: { clock: EngineSnapshot["clock"]; uiLocale: UiLocale }) {
  const source = () => clockSourceLabel(props.clock.source);
  const status = () => clockSyncStatusLabel(props.clock);
  const beat = () => props.clock.beat_counter;
  return <span class="topbarTempoSync" role="group" aria-label="Clock sync" data-topbar-sync
    data-topbar-sync-state={status()} title={`${translateUiText(source(), props.uiLocale)} · ${translateUiText(status(), props.uiLocale)}`}>
    <span class="topbarSyncSource">{source()}</span>
    <span class="topbarSyncRow">
      <span class="topbarBeats" role="img" aria-label="Beat indicator" data-topbar-beat={beat()}>
        <For each={[0, 1, 2, 3]}>{index => <i class={beat() % 4 === index ? "lit" : ""} />}</For>
      </span>
      <span class="topbarSyncStatus">{status()}</span>
    </span>
  </span>;
}

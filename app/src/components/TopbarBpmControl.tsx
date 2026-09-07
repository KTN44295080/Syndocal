import { createSignal, onCleanup, Show } from "solid-js";
import { Portal } from "solid-js/web";
import "./TopbarBpmControl.css";

export function TopbarBpmControl(props: { bpm: number; onSetBpm: (bpm: number) => void | Promise<void> }) {
  const [editing, setEditing] = createSignal(false);
  const [draft, setDraft] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let mounted = true;
  let container!: HTMLSpanElement;
  onCleanup(() => { mounted = false; });
  const begin = () => {
    if (editing()) return;
    setDraft(String(props.bpm)); setError(null); setEditing(true);
  };
  const commit = async () => {
    if (!editing() || busy()) return;
    const value = Number(draft());
    if (!draft().trim() || !Number.isFinite(value) || value < 20 || value > 300) {
      setError("BPMは20〜300で入力してください。"); return;
    }
    if (value === props.bpm) { setError(null); setEditing(false); return; }
    setBusy(true); setError(null);
    try {
      await props.onSetBpm(value);
      if (mounted) setEditing(false);
    } catch (cause) {
      if (mounted) setError(`BPMを変更できませんでした: ${String(cause)}`);
    } finally {
      if (mounted) setBusy(false);
    }
  };
  return <span ref={container} class="bpmReadout topbarBpmControl" onClick={begin} onPointerDown={event => event.stopPropagation()}
    onMouseDown={event => event.stopPropagation()} onDblClick={event => event.stopPropagation()}>
    <small>BPM</small>
    <Show when={editing()} fallback={<button type="button" class="topbarBpmValue" aria-label="Edit BPM"
      title="Edit BPM" onClick={begin}><strong>{props.bpm.toFixed(0)}</strong></button>}>
      <input class="topbarBpmInput" type="number" min="20" max="300" step="0.1" aria-label="BPM"
        aria-invalid={Boolean(error())} title={error() ?? "BPM (20〜300)"} disabled={busy()} value={draft()}
        ref={element => queueMicrotask(() => { if (mounted && element.isConnected) { element.focus(); element.select(); } })}
        onInput={event => { setDraft(event.currentTarget.value); setError(null); }}
        onBlur={() => void commit()}
        onKeyDown={event => {
          if (event.key === "Enter") { event.preventDefault(); event.stopPropagation(); void commit(); }
          if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); if (!busy()) { setEditing(false); setError(null); } }
        }} />
    </Show>
    <Show when={error()}>{message => {
      const bounds = container.getBoundingClientRect();
      return <Portal><span class="topbarBpmError" role="alert" style={{
        top: `${bounds.bottom + 4}px`, left: `${Math.max(4, Math.min(bounds.left, window.innerWidth - 280))}px`,
      }}>{message()}</span></Portal>;
    }}</Show>
  </span>;
}

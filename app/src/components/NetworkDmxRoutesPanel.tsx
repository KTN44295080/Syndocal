import { Index, Show, createEffect, createMemo, createSignal, untrack } from "solid-js";
import type { DmxOutputConfig } from "../types";
import { networkDmxRoutePayload } from "../networkDmxRoutes";

interface Props {
  routes: DmxOutputConfig[];
  projectEpoch: number;
  onApply: (routes: DmxOutputConfig[]) => Promise<void>;
}

export function NetworkDmxRoutesPanel(props: Props) {
  const [draft, setDraft] = createSignal<DmxOutputConfig[]>([]);
  const [dirty, setDirty] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);
  const [error, setError] = createSignal("");
  const [page, setPage] = createSignal(0);
  const pageSize = 5;
  let epoch: number | undefined;
  let observed = "";
  const reset = () => {
    setDraft(props.routes.slice(1).map(row => ({ ...row })));
    setDirty(false); setConflict(false); setError(""); setPage(0);
  };
  const canonicalRoutes = createMemo(() => JSON.stringify(props.routes));
  createEffect(() => {
    const nextEpoch = props.projectEpoch, serialized = canonicalRoutes();
    untrack(() => {
      if (epoch !== nextEpoch || observed !== serialized && !dirty()) reset();
      else if (!busy() && observed !== serialized) setConflict(true);
      epoch = nextEpoch; observed = serialized;
    });
  });
  const pageCount = createMemo(() => Math.max(1, Math.ceil(draft().length / pageSize)));
  const activePage = createMemo(() => Math.min(page(), pageCount() - 1));
  const rows = createMemo(() => draft().slice(activePage() * pageSize, (activePage() + 1) * pageSize));
  const update = (index: number, patch: Partial<DmxOutputConfig>) => {
    setDraft(current => current.map((row, i) => i === index ? { ...row, ...patch } : row));
    setDirty(true); setError("");
  };
  const add = (protocol: "ArtNet" | "Sacn") => {
    const used = new Set(draft().filter(row => row.protocol === protocol).map(row => row.universe));
    let universe = 1;
    while (used.has(universe)) universe++;
    setDraft(current => [...current, {
      enabled: true, protocol, universe,
      target_ip: protocol === "Sacn" ? "multicast" : "127.0.0.1",
      port: protocol === "Sacn" ? 5568 : 6454, serial_port: "", serial_baud_rate: 57600,
    }]);
    setDirty(true); setPage(pageCount() - 1); setError("");
  };
  const save = async () => {
    if (busy() || conflict()) return;
    const capturedEpoch = props.projectEpoch;
    try {
      networkDmxRoutePayload(draft());
      setBusy(true); setError("");
      await props.onApply([props.routes[0], ...draft()].map(row => ({ ...row })));
      if (props.projectEpoch === capturedEpoch) reset();
    } catch (failure) {
      if (props.projectEpoch === capturedEpoch) setError(String(failure));
    } finally { setBusy(false); }
  };
  return <section class="networkDmxRoutes" data-network-dmx-routes>
    <header class="panelHeader"><h3>Network universes</h3><span>{draft().length} / 255</span></header>
    <p class="hint">Add an output destination for each patched universe. U0 USB-DMX and its loopback remain managed above.</p>
    <p class="hint">A single-universe USB interface cannot output two universes. Use an Art-Net or sACN node for additional universes.</p>
    <div class="buttonRow">
      <button type="button" disabled={busy() || draft().length >= 255} onClick={() => add("ArtNet")}>+ Art-Net</button>
      <button type="button" disabled={busy() || draft().length >= 255} onClick={() => add("Sacn")}>+ sACN</button>
      <button type="button" class="primary" data-network-dmx-save disabled={!dirty() || busy() || conflict() || !props.routes[0]?.enabled} onClick={() => void save()}>{busy() ? "Saving…" : "Apply network routes"}</button>
      <button type="button" disabled={busy()} onClick={reset}>Reload routes</button>
    </div>
    <Show when={!props.routes[0]?.enabled}><p class="hint">Prepare U0 output before applying network universes.</p></Show>
    <Show when={conflict()}><p role="alert">Routes changed while editing. Reload routes before applying.</p></Show>
    <Show when={error()}><p class="fieldError" role="alert" data-no-localize>{error()}</p></Show>
    <div class="networkDmxRouteRows">
      <Index each={rows()}>{(row, localIndex) => {
        const index = () => activePage() * pageSize + localIndex;
        return <fieldset class="networkDmxRouteRow" disabled={busy()} data-network-dmx-route={index()}>
          <legend data-no-localize>{row().protocol === "ArtNet" ? "Art-Net" : "sACN"} · U{row().universe}</legend>
          <label>Enabled<input type="checkbox" checked={row().enabled} onChange={e => update(index(), {enabled: e.currentTarget.checked})} /></label>
          <label>Protocol<select value={row().protocol} onChange={e => update(index(), {protocol: e.currentTarget.value as "ArtNet" | "Sacn", port: e.currentTarget.value === "Sacn" ? 5568 : 6454, universe: Math.max(1, row().universe), target_ip: e.currentTarget.value === "Sacn" ? "multicast" : "127.0.0.1"})}><option value="ArtNet">Art-Net</option><option value="Sacn">sACN</option></select></label>
          <label>Universe<input data-network-dmx-universe type="number" min={row().protocol === "Sacn" ? 1 : 0} max={row().protocol === "Sacn" ? 63999 : 32767} value={row().universe} onInput={e => update(index(), {universe: e.currentTarget.value === "" ? NaN : Number(e.currentTarget.value)})} /></label>
          <label class="networkDmxDestination">Destination<input data-network-dmx-destination type="text" value={row().target_ip} placeholder="IPv4 / sACN multicast" onInput={e => update(index(), {target_ip: e.currentTarget.value})} /></label>
          <label>Port<input type="number" min="1" max="65535" value={row().port} onInput={e => update(index(), {port: e.currentTarget.value === "" ? NaN : Number(e.currentTarget.value)})} /></label>
          <button type="button" onClick={() => {setDraft(current => current.filter((_, i) => i !== index())); setDirty(true);}}>Remove</button>
        </fieldset>;
      }}</Index>
    </div>
    <Show when={pageCount() > 1}><nav class="buttonRow" aria-label="Network route pages">
      <button type="button" disabled={activePage() === 0} onClick={() => setPage(activePage() - 1)}>Previous</button>
      <span>{activePage() + 1} / {pageCount()}</span>
      <button type="button" disabled={activePage() + 1 >= pageCount()} onClick={() => setPage(activePage() + 1)}>Next</button>
    </nav></Show>
  </section>;
}

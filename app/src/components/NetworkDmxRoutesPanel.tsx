import { Index, createEffect, createMemo, createSignal, untrack } from "solid-js";
import type { DmxOutputConfig, DmxOutputRouteTelemetry } from "../types";
import { networkDmxRoutePayload } from "../networkDmxRoutes";
import { networkDmxRouteState, sameNetworkDmxRoute } from "../dmxUniverseState";

export const NETWORK_DMX_ROUTE_PAGE_SIZE = 5;
export const MAX_NETWORK_DMX_ROUTES = 255;

export interface NetworkDmxRouteSource {
  /** `dmx_outputs`; index 0 is the Universe 0 show route and is never edited here. */
  readonly routes: DmxOutputConfig[];
  readonly projectEpoch: number;
  onApply: (routes: DmxOutputConfig[]) => Promise<void>;
}

export type NetworkDmxProtocol = "ArtNet" | "Sacn";

/**
 * The editable Art-Net / sACN universes (`routes[1..]`). Edits stay a local
 * draft until Apply sends the whole set; a change from elsewhere while the
 * draft is dirty blocks Apply until Reload.
 */
export function createNetworkDmxRouteDraft(source: NetworkDmxRouteSource) {
  const [draft, setDraft] = createSignal<DmxOutputConfig[]>([]);
  const [dirty, setDirty] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [conflict, setConflict] = createSignal(false);
  const [error, setError] = createSignal("");
  const [page, setPage] = createSignal(0);
  let epoch: number | undefined;
  let observed = "";
  const reset = () => {
    setDraft(source.routes.slice(1).map(row => ({ ...row })));
    setDirty(false); setConflict(false); setError(""); setPage(0);
  };
  const canonicalRoutes = createMemo(() => JSON.stringify(source.routes));
  createEffect(() => {
    const nextEpoch = source.projectEpoch, serialized = canonicalRoutes();
    untrack(() => {
      if (epoch !== nextEpoch || observed !== serialized && !dirty()) reset();
      else if (!busy() && observed !== serialized) setConflict(true);
      epoch = nextEpoch; observed = serialized;
    });
  });
  const pageCount = createMemo(() => Math.max(1, Math.ceil(draft().length / NETWORK_DMX_ROUTE_PAGE_SIZE)));
  const activePage = createMemo(() => Math.min(page(), pageCount() - 1));
  const rows = createMemo(() => draft().slice(
    activePage() * NETWORK_DMX_ROUTE_PAGE_SIZE,
    (activePage() + 1) * NETWORK_DMX_ROUTE_PAGE_SIZE,
  ));
  const update = (index: number, patch: Partial<DmxOutputConfig>) => {
    setDraft(current => current.map((row, i) => i === index ? { ...row, ...patch } : row));
    setDirty(true); setError("");
  };
  const remove = (index: number) => {
    setDraft(current => current.filter((_, i) => i !== index));
    setDirty(true); setError("");
  };
  const nextFreeUniverse = (protocol: NetworkDmxProtocol) => {
    const used = new Set(draft().filter(row => row.protocol === protocol).map(row => row.universe));
    let universe = 1;
    while (used.has(universe)) universe++;
    return universe;
  };
  const canAdd = () => !busy() && draft().length < MAX_NETWORK_DMX_ROUTES;
  const add = (protocol: NetworkDmxProtocol, universe = nextFreeUniverse(protocol)) => {
    if (!canAdd()) return;
    setDraft(current => [...current, {
      enabled: true, protocol, universe,
      target_ip: protocol === "Sacn" ? "multicast" : "127.0.0.1",
      port: protocol === "Sacn" ? 5568 : 6454, serial_port: "", serial_baud_rate: 57600,
    }]);
    setDirty(true); setPage(pageCount() - 1); setError("");
  };
  const showRoutePrepared = () => source.routes[0]?.enabled === true;
  const canApply = () => dirty() && !busy() && !conflict() && showRoutePrepared();
  const save = async () => {
    if (busy() || conflict()) return;
    const capturedEpoch = source.projectEpoch;
    try {
      networkDmxRoutePayload(draft());
      setBusy(true); setError("");
      await source.onApply([source.routes[0], ...draft()].map(row => ({ ...row })));
      if (source.projectEpoch === capturedEpoch) reset();
    } catch (failure) {
      if (source.projectEpoch === capturedEpoch) setError(String(failure));
    } finally { setBusy(false); }
  };
  return {
    draft, dirty, busy, conflict, error, rows, pageCount, activePage, setPage,
    update, remove, add, canAdd, nextFreeUniverse, showRoutePrepared, canApply, save, reset,
    /** The applied route behind draft row `index`, if any. */
    applied: (index: number): DmxOutputConfig | undefined => source.routes[index + 1],
  };
}

export type NetworkDmxRouteDraft = ReturnType<typeof createNetworkDmxRouteDraft>;

/**
 * The network universes as rows of the DMX universes table. A row that
 * differs from the applied route shows "Not applied" until Apply.
 */
export function NetworkDmxRouteRows(props: { draft: NetworkDmxRouteDraft; telemetry: DmxOutputRouteTelemetry[] }) {
  const draft = props.draft;
  return <tbody class="dmxUniverseGroup" data-network-dmx-routes>
    <Index each={draft.rows()}>{(row, localIndex) => {
      const index = () => draft.activePage() * NETWORK_DMX_ROUTE_PAGE_SIZE + localIndex;
      const state = () => {
        const applied = draft.applied(index());
        return sameNetworkDmxRoute(applied, row())
          ? networkDmxRouteState(applied!, props.telemetry.find(result => result.index === index() + 1))
          : { label: "Not applied", tone: "idle" as const };
      };
      return <tr data-network-dmx-route={index()}>
        <th scope="row">
          <input
            data-network-dmx-universe
            aria-label="Universe"
            type="number"
            disabled={draft.busy()}
            min={row().protocol === "Sacn" ? 1 : 0}
            max={row().protocol === "Sacn" ? 63999 : 32767}
            value={row().universe}
            onInput={e => draft.update(index(), {universe: e.currentTarget.value === "" ? NaN : Number(e.currentTarget.value)})}
          />
        </th>
        <td>
          <select
            aria-label="Transport"
            disabled={draft.busy()}
            value={row().protocol}
            onChange={e => draft.update(index(), {
              protocol: e.currentTarget.value as NetworkDmxProtocol,
              port: e.currentTarget.value === "Sacn" ? 5568 : 6454,
              universe: Math.max(1, row().universe),
              target_ip: e.currentTarget.value === "Sacn" ? "multicast" : "127.0.0.1",
            })}
          ><option value="ArtNet">Art-Net</option><option value="Sacn">sACN</option></select>
        </td>
        <td>
          <div class="dmxUniverseDestination">
            <input
              data-network-dmx-destination
              aria-label="Destination"
              type="text"
              disabled={draft.busy()}
              value={row().target_ip}
              placeholder="IPv4 / sACN multicast"
              onInput={e => draft.update(index(), {target_ip: e.currentTarget.value})}
            />
            <input
              aria-label="Port"
              type="number"
              min="1"
              max="65535"
              disabled={draft.busy()}
              value={row().port}
              onInput={e => draft.update(index(), {port: e.currentTarget.value === "" ? NaN : Number(e.currentTarget.value)})}
            />
          </div>
        </td>
        <td>
          <span class={`ioConnectionState ${state().tone}`} title={state().label}>
            <i aria-hidden="true" />{state().label}
          </span>
        </td>
        <td>
          <div class="dmxUniverseActions">
            <label class="dmxUniverseEnabled">
              <input
                type="checkbox"
                disabled={draft.busy()}
                checked={row().enabled}
                onChange={e => draft.update(index(), {enabled: e.currentTarget.checked})}
              />Enabled
            </label>
            <button type="button" disabled={draft.busy()} onClick={() => draft.remove(index())}>Remove</button>
          </div>
        </td>
      </tr>;
    }}</Index>
  </tbody>;
}

import { For } from "solid-js";
import type { DmxUniverseOverviewRow } from "../dmxUniverseOverview";

interface DmxUniverseOverviewProps {
  rows: DmxUniverseOverviewRow[];
  safetyBlackoutEngaged: boolean;
}

/**
 * Every DMX output universe in one table: Universe 0 (Show DMX), the
 * USB-DMX cables and the Art-Net / sACN routes, with their live state.
 * Each is still edited in its own section below.
 */
export function DmxUniverseOverview(props: DmxUniverseOverviewProps) {
  return (
    <section class="dmxUniverseOverview ioConnectionDesk" data-dmx-universe-overview>
      <header class="ioDeskHeader">
        <h2>DMX universes</h2>
        <span class={`ioConnectionState ${props.safetyBlackoutEngaged ? "error" : "idle"}`}>
          <i aria-hidden="true" />{props.safetyBlackoutEngaged ? "S0 engaged" : "S0 clear"}
        </span>
      </header>
      <table class="dmxUniverseTable">
        <thead>
          <tr>
            <th scope="col">Universe</th>
            <th scope="col">Transport</th>
            <th scope="col">Device / destination</th>
            <th scope="col">State</th>
          </tr>
        </thead>
        <tbody>
          <For each={props.rows}>{(row) => (
            <tr data-dmx-universe-row={row.key}>
              <td class="tabularNums" data-no-localize>U{row.universe}</td>
              <td>{row.transport}</td>
              <td class="dmxUniverseTarget" data-no-localize title={row.target}>{row.target}</td>
              <td>
                <span class={`ioConnectionState ${row.tone}`} title={row.state}>
                  <i aria-hidden="true" />{row.state}
                </span>
              </td>
            </tr>
          )}</For>
        </tbody>
      </table>
    </section>
  );
}

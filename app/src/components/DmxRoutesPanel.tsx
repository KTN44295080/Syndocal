import { For } from "solid-js";
import type { DmxOutputConfig } from "../types";

interface DmxRoutesPanelProps {
  routes: DmxOutputConfig[];
  routeLabel: (route: DmxOutputConfig) => string;
  onAddCurrent: () => void | Promise<void>;
  onApplyRoutes: () => void | Promise<void>;
  onRemoveRoute: (index: number) => void | Promise<void>;
}

export function DmxRoutesPanel(props: DmxRoutesPanelProps) {
  return (
    <div class="dmxRoutes">
      <div class="panelHeader">
        <h3>DMX Routes</h3>
        <span>{props.routes.length}</span>
      </div>
      <div class="buttonRow">
        <button onClick={props.onAddCurrent}>Add Current</button>
        <button onClick={props.onApplyRoutes}>Apply Routes</button>
      </div>
      <div class="timelineList">
        <For each={props.routes}>
          {(route, index) => (
            <div class="timelineItem">
              <strong>{props.routeLabel(route)}</strong>
              <span>{route.enabled ? "Enabled" : "Disabled"}</span>
              <button onClick={() => props.onRemoveRoute(index())} disabled={props.routes.length <= 1}>
                Remove
              </button>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}

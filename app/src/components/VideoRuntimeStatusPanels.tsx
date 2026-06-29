import { For, Show } from "solid-js";
import type { VideoBackendStatus } from "../types";

export interface VideoOutputRenderPlanRow {
  id: number;
  label: string;
  endpoint: string;
  stateLabel: string;
  stateClass: string;
  detail: string;
}

export interface ExternalVideoStatusRow {
  id: string;
  direction: string;
  backend: string;
  label: string;
  endpoint: string;
  stateLabel: string;
  stateClass: string;
  detail: string;
}

interface VideoOutputRenderPlanStatusPanelProps {
  summary: string;
  checked: boolean;
  rows: VideoOutputRenderPlanRow[];
  onRefresh: () => void | Promise<void>;
}

interface VideoBackendStatusPanelProps {
  summary: string;
  backends: VideoBackendStatus[] | null;
  backendClass: (state: VideoBackendStatus["state"]) => string;
  onRefresh: () => void | Promise<void>;
}

interface ExternalVideoIoStatusPanelProps {
  ioSummary: string;
  transportSummary: string;
  checked: boolean;
  planRows: ExternalVideoStatusRow[];
  activeTransportRows: ExternalVideoStatusRow[];
  transportRows: ExternalVideoStatusRow[];
  transportEventRows: ExternalVideoStatusRow[];
  planClass: (stateClass: string) => string;
  transportClass: (stateClass: string) => string;
  onRefreshPlans: () => void | Promise<void>;
  onSyncRoutes: () => void | Promise<void>;
}

export function VideoOutputRenderPlanStatusPanel(props: VideoOutputRenderPlanStatusPanelProps) {
  return (
    <div class="videoBackendStatus videoOutputPlanStatus">
      <div>
        <strong>Render Plans</strong>
        <span>{props.summary}</span>
      </div>
      <div class="videoBackendGrid videoOutputPlanGrid">
        <Show when={props.checked} fallback={<small>No render plan status</small>}>
          <Show when={props.rows.length > 0} fallback={<small>No video outputs</small>}>
            <For each={props.rows}>
              {(plan) => (
                <span class={`videoBackendPill videoOutputPlanPill state-${plan.stateClass}`} title={plan.detail}>
                  <strong>{plan.stateLabel}</strong>
                  <small>{plan.label}</small>
                  <small>{plan.endpoint}</small>
                </span>
              )}
            </For>
          </Show>
        </Show>
      </div>
      <button onClick={() => void props.onRefresh()}>Check Plans</button>
    </div>
  );
}

export function VideoBackendStatusPanel(props: VideoBackendStatusPanelProps) {
  return (
    <div class="videoBackendStatus">
      <div>
        <strong>Backends</strong>
        <span>{props.summary}</span>
      </div>
      <div class="videoBackendGrid">
        <Show when={props.backends} fallback={<small>No backend status</small>}>
          {(backends) => (
            <For each={backends()}>
              {(backend) => (
                <span class={props.backendClass(backend.state)} title={backend.detail}>
                  <strong>{backend.label}</strong>
                  <small>{backend.state}</small>
                </span>
              )}
            </For>
          )}
        </Show>
      </div>
      <button onClick={() => void props.onRefresh()}>Check Backends</button>
    </div>
  );
}

export function ExternalVideoIoStatusPanel(props: ExternalVideoIoStatusPanelProps) {
  return (
    <div class="videoBackendStatus videoIoPlanStatus">
      <div>
        <strong>I/O Plans</strong>
        <span>{props.ioSummary}</span>
        <span>{props.transportSummary}</span>
      </div>
      <div class="videoBackendGrid videoIoPlanGrid">
        <Show when={props.checked} fallback={<small>No I/O plan status</small>}>
          <Show when={props.planRows.length > 0} fallback={<small>No external routes</small>}>
            <For each={props.planRows}>
              {(plan) => (
                <span class={props.planClass(plan.stateClass)} title={`${plan.detail} / ${plan.endpoint}`}>
                  <strong>
                    {plan.direction} {plan.backend}
                  </strong>
                  <small>
                    {plan.stateLabel} / {plan.label}
                  </small>
                  <small>{plan.endpoint}</small>
                </span>
              )}
            </For>
          </Show>
        </Show>
        <For each={props.activeTransportRows}>
          {(route) => (
            <span class={props.transportClass(route.stateClass)} title={`${route.detail} / ${route.endpoint}`}>
              <strong>
                {route.stateLabel} {route.direction}
              </strong>
              <small>
                {route.backend} / {route.label}
              </small>
              <small>{route.endpoint}</small>
            </span>
          )}
        </For>
        <For each={props.transportRows}>
          {(route) => (
            <span class={props.transportClass(route.stateClass)} title={`${route.detail} / ${route.endpoint}`}>
              <strong>
                {route.stateLabel} {route.direction}
              </strong>
              <small>
                {route.backend} / {route.label}
              </small>
              <small>{route.endpoint}</small>
            </span>
          )}
        </For>
        <For each={props.transportEventRows}>
          {(event) => (
            <span class={props.transportClass(event.stateClass)} title={`${event.detail} / ${event.endpoint}`}>
              <strong>Driver {event.stateLabel}</strong>
              <small>
                {event.direction} {event.backend} / {event.label}
              </small>
              <small>{event.endpoint}</small>
            </span>
          )}
        </For>
      </div>
      <div class="videoIoPlanActions">
        <button onClick={() => void props.onRefreshPlans()}>Check I/O</button>
        <button onClick={() => void props.onSyncRoutes()}>Sync Routes</button>
      </div>
    </div>
  );
}

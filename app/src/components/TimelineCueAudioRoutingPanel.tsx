import { createEffect, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type {
  MachineTimelineCueAudioSettingsV1,
  TimelineCueAudioStatus,
} from "../types";
import {
  timelineCueAudioEndpointByExactName,
  timelineCueAudioEndpointIsSelectable,
  timelineCueAudioMissingSavedName,
} from "../timelineCueAudioRouting";
import { createTimelineCueAudioSetupOperationGate } from "../timelineCueAudioStatusSync";

export const timelineCueAudioOpenSetupEvent = "syndocal:open-timeline-cue-audio-setup";

export interface TimelineCueAudioRoutingPanelProps {
  status: TimelineCueAudioStatus;
  mutationBusy: boolean;
  localError: string | null;
  onConfigure: (settings: MachineTimelineCueAudioSettingsV1) => void;
  onRefresh: () => void | Promise<void>;
}

export const timelineCueAudioLifecycleLabel = (status: TimelineCueAudioStatus): string => {
  switch (status.lifecycle) {
    case "loading_settings": return "Loading settings";
    case "disabled_by_project": return "Disabled by project";
    case "waiting_for_program_output": return "Waiting for PROGRAM";
    case "applying": return "Applying";
    case "running": return "Running";
    case "missing_device": return "Missing device";
    case "ambiguous_device": return "Ambiguous device";
    case "topology_changed": return "Topology changed";
    case "stalled": return "Stalled";
    case "fault": return "Fault";
  }
};

const routeLabel = (status: TimelineCueAudioStatus): string =>
  status.desiredSettings.route === "explicit_device" ? "Explicit WDM device" : "Follow PROGRAM";

/**
 * The one editable Timeline authoring audio route.  It is rendered from the
 * same backend status/settings path as the Timeline monitor summary; there is
 * deliberately no device-specific default or name-only fallback here.
 */
export function TimelineCueAudioRoutingPanel(props: TimelineCueAudioRoutingPanelProps) {
  const [pendingExplicitRoute, setPendingExplicitRoute] = createSignal(false);
  const [setupOperationBusy, setSetupOperationBusy] = createSignal(true);
  const [setupOperationError, setSetupOperationError] = createSignal<string | null>(null);
  const mutationIdleWaiters = new Set<() => void>();
  let outputSelect: HTMLSelectElement | undefined;

  const waitForMutationIdle = () => {
    if (!props.mutationBusy) return Promise.resolve();
    return new Promise<void>((resolve) => mutationIdleWaiters.add(resolve));
  };
  const waitForMutationTerminal = async () => {
    // App's settings queue flips mutationBusy synchronously before its first
    // native await. Yield once so a void callback cannot look terminal before
    // that state has become observable.
    await Promise.resolve();
    await waitForMutationIdle();
  };
  const setupOperationGate = createTimelineCueAudioSetupOperationGate({
    waitForMutationIdle,
    waitForMutationTerminal,
    refresh: () => props.onRefresh(),
    onBusy: setSetupOperationBusy,
  });
  const controlsBusy = () => setupOperationBusy() || props.mutationBusy;
  const operationErrorMessage = (error: unknown) => error instanceof Error
    ? error.message
    : "Timeline audio setup operation failed.";
  const refreshOutputs = () => {
    setSetupOperationError(null);
    void setupOperationGate.refresh().catch((error) => setSetupOperationError(operationErrorMessage(error)));
  };
  const configure = (next: () => MachineTimelineCueAudioSettingsV1 | null) => {
    setSetupOperationError(null);
    void setupOperationGate.configure(() => {
      const settings = next();
      if (settings) props.onConfigure(settings);
    }).catch((error) => setSetupOperationError(operationErrorMessage(error)));
  };

  createEffect(() => {
    if (props.mutationBusy) return;
    const waiters = [...mutationIdleWaiters];
    mutationIdleWaiters.clear();
    waiters.forEach((resolve) => resolve());
  });

  // Setup > I/O > Audio is the discoverable entry point. Opening it performs
  // one topology enumeration automatically, so the operator never has to
  // know about a hidden manual-refresh prerequisite.
  onMount(() => {
    refreshOutputs();
  });
  onCleanup(() => {
    setupOperationGate.dispose();
    const waiters = [...mutationIdleWaiters];
    mutationIdleWaiters.clear();
    waiters.forEach((resolve) => resolve());
  });

  createEffect(() => {
    if (props.status.desiredSettings.route === "explicit_device") {
      setPendingExplicitRoute(false);
    }
    const select = outputSelect;
    if (!select || props.status.desiredSettings.route !== "explicit_device") return;
    const desired = props.status.desiredSettings.device_name ?? "";
    select.value = desired;
  });

  const explicitRouteVisible = () =>
    pendingExplicitRoute() || props.status.desiredSettings.route === "explicit_device";
  const desiredName = () => props.status.desiredSettings.device_name;
  const missingDesiredName = () => {
    const name = desiredName();
    return props.status.desiredSettings.route === "explicit_device"
      && timelineCueAudioMissingSavedName(props.status.endpoints, name);
  };
  const topologyChanged = () =>
    props.status.desiredSettings.route === "explicit_device"
    && props.status.requestedTopologyFingerprint !== props.status.observedTopologyFingerprint;

  const configureFollowProgram = () => {
    setPendingExplicitRoute(false);
    configure(() => ({
      ...props.status.desiredSettings,
      route: "follow_program",
      device_name: null,
      topology_fingerprint: null,
    }));
  };

  const configureExplicitDevice = (name: string) => {
    configure(() => {
      const endpoint = props.status.endpoints.find((candidate) => candidate.name === name);
      const fingerprint = props.status.observedTopologyFingerprint;
      if (!endpoint || !timelineCueAudioEndpointIsSelectable(props.status.endpoints, endpoint) || !fingerprint) {
        return null;
      }
      return {
        ...props.status.desiredSettings,
        route: "explicit_device",
        device_name: endpoint.name,
        topology_fingerprint: fingerprint,
      };
    });
  };

  return (
    <section
      class="audioOutputTimelineCueAudio"
      data-timeline-cue-audio-routing
      aria-labelledby="timeline-cue-audio-routing-title"
      aria-busy={controlsBusy()}
    >
      <header class="audioOutputTimelineCueAudioHeader">
        <div>
          <h4 id="timeline-cue-audio-routing-title">Timeline authoring audio</h4>
          <p>Choose the Windows output for Timeline media, Guide, and Click.</p>
        </div>
        <output
          classList={{ fault: Boolean(props.status.lastError) || props.status.lifecycle === "fault" }}
          data-timeline-cue-audio-status
          aria-live="polite"
        >
          {timelineCueAudioLifecycleLabel(props.status)}
        </output>
      </header>

      <div class="audioOutputTimelineCueAudioBody">
        <p class="audioOutputTimelineCueAudioPolicy" role="note">
          Devices are enumerated from the current Windows WDM output catalogue. Missing or duplicate names remain unselectable until an exact current endpoint is chosen.
        </p>
        <label class="audioOutputField" data-audio-output-field="timeline-cue-route">
          <span>Timeline route</span>
          <select
            aria-label="Timeline authoring audio route"
            disabled={controlsBusy()}
            value={explicitRouteVisible() ? "explicit_device" : "follow_program"}
            onInput={(event) => {
              if (event.currentTarget.value === "follow_program") configureFollowProgram();
              else setPendingExplicitRoute(true);
            }}
          >
            <option value="follow_program">Follow PROGRAM output</option>
            <option value="explicit_device">Explicit WDM output</option>
          </select>
        </label>

        <Show when={explicitRouteVisible()}>
          <label class="audioOutputField" data-audio-output-field="timeline-cue-device">
            <span>Timeline output device</span>
            <select
              ref={(element) => { outputSelect = element; }}
              data-timeline-cue-audio-output
              aria-label="Timeline authoring audio output device"
              disabled={controlsBusy()}
              value={desiredName() ?? ""}
              onInput={(event) => configureExplicitDevice(event.currentTarget.value)}
            >
              <option value="" disabled>Select an exact Windows output…</option>
              <Show when={missingDesiredName()}>
                <option value={desiredName() ?? ""} disabled data-timeline-cue-audio-missing>
                  {desiredName()} (missing; reselect)
                </option>
              </Show>
              <For each={props.status.endpoints}>
                {(endpoint) => {
                  const selectable = timelineCueAudioEndpointIsSelectable(props.status.endpoints, endpoint);
                  return (
                    <option
                      value={endpoint.name}
                      disabled={!selectable}
                      data-timeline-cue-audio-duplicate={endpoint.occurrences > 1 ? "true" : undefined}
                    >
                      {endpoint.occurrences > 1
                        ? `${endpoint.name} (${endpoint.occurrences} matching outputs; ambiguous)`
                        : endpoint.name}
                    </option>
                  );
                }}
              </For>
              <Show when={props.status.endpoints.length === 0}>
                <option value="" disabled>No current Windows output was enumerated</option>
              </Show>
            </select>
          </label>
        </Show>

        <label class="audioOutputField" data-audio-output-field="timeline-cue-click-gain">
          <span>Click gain</span>
          <input
            type="range"
            min="0"
            max="2"
            step="0.05"
            aria-label="Timeline Click gain"
            value={props.status.desiredSettings.click_gain}
            disabled={controlsBusy()}
            onInput={(event) => configure(() => ({
              ...props.status.desiredSettings,
              click_gain: Number(event.currentTarget.value),
            }))}
          />
          <output class="tabularNums" data-no-localize>{Math.round(props.status.desiredSettings.click_gain * 100)}%</output>
        </label>
        <label class="audioOutputField" data-audio-output-field="timeline-cue-guide-gain">
          <span>Guide gain</span>
          <input
            type="range"
            min="0"
            max="2"
            step="0.05"
            aria-label="Timeline Guide gain"
            value={props.status.desiredSettings.guide_gain}
            disabled={controlsBusy()}
            onInput={(event) => configure(() => ({
              ...props.status.desiredSettings,
              guide_gain: Number(event.currentTarget.value),
            }))}
          />
          <output class="tabularNums" data-no-localize>{Math.round(props.status.desiredSettings.guide_gain * 100)}%</output>
        </label>

        <button
          type="button"
          data-timeline-cue-audio-refresh
          disabled={controlsBusy()}
          onClick={refreshOutputs}
        >
          Refresh Windows outputs
        </button>

        <output class="audioOutputTimelineCueAudioRuntime" role="status">
          <span>{routeLabel(props.status)}</span>
          <Show when={props.status.resolvedDeviceName}>
            {(name) => <span data-no-localize>{` · ${name()}`}</span>}
          </Show>
          <Show when={topologyChanged()}>
            <span> · reselect after topology change</span>
          </Show>
        </output>
        <Show when={props.status.lastError}>
          {(error) => <p class="inlineError" role="alert">{error()}</p>}
        </Show>
        <Show when={props.localError}>
          {(error) => <p class="inlineError" role="alert">{error()}</p>}
        </Show>
        <Show when={setupOperationError()}>
          {(error) => <p class="inlineError" role="alert">{error()}</p>}
        </Show>
      </div>
    </section>
  );
}

export interface TimelineCueAudioStatusSummaryProps {
  status: TimelineCueAudioStatus;
}

/** Compact read-only status left in Timeline tools after Setup owns editing. */
export function TimelineCueAudioStatusSummary(props: TimelineCueAudioStatusSummaryProps) {
  const explicit = () => props.status.desiredSettings.route === "explicit_device";
  const requestedName = () => props.status.desiredSettings.device_name;
  const endpoint = () => timelineCueAudioEndpointByExactName(props.status.endpoints, requestedName());
  return (
    <div class="timelineCueAudioStatusSummary" data-timeline-cue-audio-summary>
      <button
        type="button"
        class="secondary"
        data-timeline-cue-audio-open-setup
        onClick={() => window.dispatchEvent(new Event(timelineCueAudioOpenSetupEvent))}
      >
        Configure in Setup → I/O → Audio
      </button>
      <output role="status" data-timeline-cue-audio-lifecycle>{timelineCueAudioLifecycleLabel(props.status)}</output>
      <span data-timeline-cue-audio-route>{routeLabel(props.status)}</span>
      <Show when={explicit()}>
        <span data-timeline-cue-audio-selected-device>
          {endpoint()?.name ?? (requestedName() ? `${requestedName()} (not currently selectable)` : "No exact device selected")}
        </span>
      </Show>
      <Show when={props.status.lastError}>
        {(error) => <span class="timelineCueAudioFault" role="alert">{error()}</span>}
      </Show>
    </div>
  );
}

import { For, Show, type JSX } from "solid-js";
import "../setupIo.css";

export type IoConnectionId = "dmx" | "midi" | "osc" | "web" | "dj";
export type IoConnectionStateTone = "ok" | "idle" | "warning" | "error";

/**
 * One top-level I/O connection. The selection button is deliberately separate
 * from its action slot so starting/stopping a service never also changes the
 * operator's current workbench.
 */
export interface IoConnectionDeckItem {
  id: IoConnectionId;
  label: string;
  summary: string;
  state: string;
  stateTone?: IoConnectionStateTone;
  primaryAction?: JSX.Element;
}

export interface IoConnectionDeckProps {
  items: readonly IoConnectionDeckItem[];
  activeId: IoConnectionId;
  onActiveId: (id: IoConnectionId) => void;
  renderWorkbench: (id: IoConnectionId) => JSX.Element;
  /** A stable id allows the caller to connect its own shortcut/help text. */
  workbenchId?: string;
  ariaLabel?: string;
}

export interface SetupIoConnectionDeckProps {
  activeId: IoConnectionId;
  onActiveId: (id: IoConnectionId) => void;
  renderWorkbench: (id: IoConnectionId) => JSX.Element;
  outputEnabled: boolean;
  midiClockConnected: boolean;
  midiControlConnected: boolean;
  oscRunning: boolean;
  webRemoteRunning: boolean;
  djListenerRunning: boolean;
  djLinkEnabled: boolean;
  onApplyOutput: () => unknown;
  onConnectMidiClock: () => unknown;
  onDisconnectMidiClock: () => unknown;
  onStartOsc: () => unknown;
  onStopOsc: () => unknown;
  onStartWebRemote: () => unknown;
  onStopWebRemote: () => unknown;
  onArmDjLink: () => unknown;
  onDisarmDjLink: () => unknown;
}

export interface IoDisclosureProps {
  id: string;
  summary: string;
  description: string;
  bodyClass?: string;
  children: JSX.Element;
}

export function IoDisclosure(props: IoDisclosureProps) {
  return (
    <details class="ioDisclosure" data-io-disclosure={props.id}>
      <summary>{props.summary}</summary>
      <div
        class={`ioDisclosureBody${props.bodyClass ? ` ${props.bodyClass}` : ""}`}
        data-io-disclosure-body
      >
        <p class="ioDisclosureDescription">{props.description}</p>
        {props.children}
      </div>
    </details>
  );
}

export function IoConnectionDeck(props: IoConnectionDeckProps) {
  const workbenchId = () => props.workbenchId ?? "setup-io-connection-workbench";
  const activeItem = () => props.items.find((item) => item.id === props.activeId) ?? props.items[0];

  return (
    <section class="setupIoConnectionDeck" data-io-connection-deck aria-label={props.ariaLabel ?? "I/O connections"}>
      <div class="setupIoConnectionCards" role="list">
        <For each={props.items}>
          {(item) => {
            const selected = () => item.id === activeItem()?.id;
            return (
              <article
                classList={{
                  setupIoConnectionCard: true,
                  active: selected(),
                }}
                data-io-connection={item.id}
                data-io-primary-connection={item.id}
                role="listitem"
              >
                <button
                  type="button"
                  class="setupIoConnectionSelect"
                  aria-pressed={selected()}
                  aria-controls={workbenchId()}
                  onClick={() => props.onActiveId(item.id)}
                >
                  <span class="setupIoConnectionLabel">{item.label}</span>
                  <span class="setupIoConnectionSummary">{item.summary}</span>
                </button>
                <div class="setupIoConnectionMeta">
                  <span class={`setupIoConnectionState ${item.stateTone ?? "idle"}`} role="status">
                    <i aria-hidden="true" />{item.state}
                  </span>
                  <Show when={item.primaryAction}>
                    <div class="setupIoConnectionAction">{item.primaryAction}</div>
                  </Show>
                </div>
              </article>
            );
          }}
        </For>
      </div>

      <section
        id={workbenchId()}
        class="setupIoConnectionWorkbench"
        data-io-connection-workbench={activeItem()?.id}
        aria-live="polite"
      >
        <Show when={activeItem()}>
          {(item) => (
            <>
              <header class="setupIoWorkbenchHeader">
                <div>
                  <h2>{item().label}</h2>
                  <p>{item().summary}</p>
                </div>
                <span class={`setupIoConnectionState ${item().stateTone ?? "idle"}`}>
                  <i aria-hidden="true" />{item().state}
                </span>
              </header>
              <div class="setupIoWorkbenchBody" data-io-workbench-body>
                {props.renderWorkbench(item().id)}
              </div>
            </>
          )}
        </Show>
      </section>
    </section>
  );
}

/**
 * Keeps the operator-facing connection inventory out of App.tsx while leaving
 * every existing start/stop/apply authority callback owned by the caller.
 */
export function SetupIoConnectionDeck(props: SetupIoConnectionDeckProps) {
  const items = (): readonly IoConnectionDeckItem[] => [
    {
      id: "dmx",
      label: "DMX",
      summary: "Output routing and external input",
      state: props.outputEnabled ? "Output enabled" : "Output disabled",
      stateTone: props.outputEnabled ? "ok" : "idle",
      primaryAction: <button type="button" onClick={() => void props.onApplyOutput()}>Apply output</button>,
    },
    {
      id: "midi",
      label: "MIDI",
      summary: "Clock, control, and feedback",
      state: props.midiClockConnected || props.midiControlConnected ? "Connected" : "Stopped",
      stateTone: props.midiClockConnected || props.midiControlConnected ? "ok" : "idle",
      primaryAction: props.midiClockConnected
        ? <button type="button" onClick={() => void props.onDisconnectMidiClock()}>Disconnect clock</button>
        : <button type="button" onClick={() => void props.onConnectMidiClock()}>Connect clock</button>,
    },
    {
      id: "osc",
      label: "OSC",
      summary: "UDP listener and mappings",
      state: props.oscRunning ? "Listening" : "Stopped",
      stateTone: props.oscRunning ? "ok" : "idle",
      primaryAction: props.oscRunning
        ? <button type="button" onClick={() => void props.onStopOsc()}>Stop</button>
        : <button type="button" onClick={() => void props.onStartOsc()}>Start</button>,
    },
    {
      id: "web",
      label: "Web Remote",
      summary: "Browser control on local or trusted LAN",
      state: props.webRemoteRunning ? "Running" : "Stopped",
      stateTone: props.webRemoteRunning ? "ok" : "idle",
      primaryAction: props.webRemoteRunning
        ? <button type="button" onClick={() => void props.onStopWebRemote()}>Stop</button>
        : <button type="button" onClick={() => void props.onStartWebRemote()}>Start</button>,
    },
    {
      id: "dj",
      label: "DJ Link",
      summary: "Rekordbox authority and show trigger",
      state: props.djListenerRunning ? "Connected" : props.djLinkEnabled ? "Armed" : "Disarmed",
      stateTone: props.djListenerRunning ? "ok" : props.djLinkEnabled ? "warning" : "idle",
      primaryAction: props.djLinkEnabled
        ? <button type="button" onClick={() => void props.onDisarmDjLink()}>Disarm</button>
        : <button type="button" onClick={() => void props.onArmDjLink()}>Arm</button>,
    },
  ];

  return (
    <IoConnectionDeck
      activeId={props.activeId}
      onActiveId={props.onActiveId}
      items={items()}
      renderWorkbench={props.renderWorkbench}
    />
  );
}

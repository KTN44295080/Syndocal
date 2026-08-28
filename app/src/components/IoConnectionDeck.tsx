import { For, Show, createMemo, type JSX } from "solid-js";
import "../setupIo.css";

export type IoConnectionId = "dmx" | "audio" | "midi" | "osc" | "web" | "dj";
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
  audioOutputSummary: string;
  audioOutputState: string;
  audioOutputStateTone?: IoConnectionStateTone;
  midiClockConnected: boolean;
  midiControlConnected: boolean;
  oscRunning: boolean;
  webRemoteRunning: boolean;
  djListenerRunning: boolean;
  djLinkEnabled: boolean;
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
  const activeItem = createMemo(
    () => props.items.find((item) => item.id === props.activeId) ?? props.items[0],
  );
  // Items are intentionally rebuilt when a connection's live status changes.
  // Keep the selected workbench keyed by its primitive connection id so those
  // status refreshes update the header without remounting local draft state in
  // the workbench (for example, a DJ Link mapping draft).
  const activeItemId = createMemo(() => activeItem()?.id);
  const tabId = (id: IoConnectionId) => `${workbenchId()}-tab-${id}`;

  return (
    <section class="setupIoConnectionDeck" data-io-connection-deck aria-label={props.ariaLabel ?? "I/O connections"}>
      <div class="setupIoConnectionCards" role="tablist" aria-label="I/O connection selectors">
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
                role="presentation"
              >
                <button
                  type="button"
                  class="setupIoConnectionSelect"
                  id={tabId(item.id)}
                  role="tab"
                  aria-selected={selected()}
                  aria-controls={workbenchId()}
                  tabIndex={selected() ? 0 : -1}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      props.onActiveId(item.id);
                      return;
                    }
                    const currentIndex = props.items.findIndex((candidate) => candidate.id === item.id);
                    const nextIndex = event.key === "ArrowLeft" || event.key === "ArrowUp"
                      ? Math.max(0, currentIndex - 1)
                      : event.key === "ArrowRight" || event.key === "ArrowDown"
                        ? Math.min(props.items.length - 1, currentIndex + 1)
                        : event.key === "Home"
                          ? 0
                          : event.key === "End"
                            ? props.items.length - 1
                            : -1;
                    if (nextIndex < 0 || nextIndex === currentIndex || !props.items[nextIndex]) return;
                    event.preventDefault();
                    const nextId = props.items[nextIndex].id;
                    props.onActiveId(nextId);
                    document.getElementById(tabId(nextId))?.focus();
                  }}
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
        role="tabpanel"
        aria-labelledby={activeItem() ? tabId(activeItem()!.id) : undefined}
        aria-live="polite"
      >
        <Show when={activeItemId()} keyed>
          {(itemId) => (
            <>
              <header class="setupIoWorkbenchHeader">
                <h2>{activeItem()?.label}</h2>
                <span class={`setupIoConnectionState ${activeItem()?.stateTone ?? "idle"}`}>
                  <i aria-hidden="true" />{activeItem()?.state}
                </span>
              </header>
              <div class="setupIoWorkbenchBody" data-io-workbench-body>
                {props.renderWorkbench(itemId)}
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
      summary: "Output routing and optional input",
      state: props.outputEnabled ? "Output enabled" : "Output disabled",
      stateTone: props.outputEnabled ? "ok" : "idle",
    },
    {
      id: "audio",
      label: "Audio",
      summary: props.audioOutputSummary,
      state: props.audioOutputState,
      stateTone: props.audioOutputStateTone ?? "idle",
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

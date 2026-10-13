import { For, Show, createMemo, type JSX } from "solid-js";
import "../setupIo.css";

export type IoConnectionId = "dmx" | "audio" | "midi" | "osc" | "web" | "sync" | "dj";
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
  /** Active/Standby sync and ShowClock LAN, shown as their own connection. */
  syncRunning?: boolean;
  syncRole?: "primary" | "standby" | null;
  djLinkInstalled?: boolean;
}

export interface IoDisclosureProps {
  id: string;
  summary: string;
  /** Omit when the body already explains itself. */
  description?: string;
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
        <Show when={props.description}><p class="ioDisclosureDescription">{props.description}</p></Show>
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
  // status refreshes update the selected card without remounting local draft
  // state in the workbench (for example, a DJ Link mapping draft).
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
                  aria-label={`${item.label}: ${item.summary}`}
                  title={item.summary}
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
            <div class="setupIoWorkbenchBody" data-io-workbench-body>
              {props.renderWorkbench(itemId)}
            </div>
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
  const audioConnectionSummary = () => {
    if (props.audioOutputState === "Ready") return "PROGRAM output";
    if (props.audioOutputState === "Active") return "PROGRAM active";
    return props.audioOutputSummary;
  };

  const items = (): readonly IoConnectionDeckItem[] => [
    {
      id: "dmx",
      label: "DMX",
      summary: "Output + input",
      state: props.outputEnabled ? "Output enabled" : "Output disabled",
      stateTone: props.outputEnabled ? "ok" : "idle",
    },
    {
      id: "audio",
      label: "Audio",
      summary: audioConnectionSummary(),
      state: props.audioOutputState,
      stateTone: props.audioOutputStateTone ?? "idle",
    },
    {
      id: "midi",
      label: "MIDI",
      summary: "Clock + control",
      state: props.midiClockConnected || props.midiControlConnected ? "Connected" : "Stopped",
      stateTone: props.midiClockConnected || props.midiControlConnected ? "ok" : "idle",
    },
    {
      id: "osc",
      label: "OSC",
      summary: "UDP listener",
      state: props.oscRunning ? "Listening" : "Stopped",
      stateTone: props.oscRunning ? "ok" : "idle",
    },
    {
      id: "web",
      label: "Web Remote",
      summary: "Remote access",
      state: props.webRemoteRunning ? "Running" : "Stopped",
      stateTone: props.webRemoteRunning ? "ok" : "idle",
    },
    {
      id: "sync",
      label: "Sync",
      summary: "Standby + ShowClock",
      state: props.syncRunning ? (props.syncRole === "primary" ? "Primary" : "Standby") : "Stopped",
      stateTone: props.syncRunning ? "ok" : "idle",
    },
    {
      id: "dj",
      label: "DJ Link",
      summary: "Track triggers",
      state: props.djListenerRunning ? "Connected" : props.djLinkEnabled ? "Armed" : "Disarmed",
      stateTone: props.djListenerRunning ? "ok" : props.djLinkEnabled ? "warning" : "idle",
    },
  ].filter((item) => item.id !== "dj" || props.djLinkInstalled === true) as IoConnectionDeckItem[];

  return (
    <IoConnectionDeck
      activeId={props.activeId}
      onActiveId={props.onActiveId}
      items={items()}
      renderWorkbench={props.renderWorkbench}
    />
  );
}

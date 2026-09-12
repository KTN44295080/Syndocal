import { createSignal, onCleanup, onMount, Show } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import type {
  ShowClockIpcPhase,
  ShowClockIpcRole,
  ShowClockIpcStatus,
  ShowClockStartRequest,
} from "../types";

interface ShowClockStatusPanelProps {
  backendAvailable: boolean;
  invokeCommand: FrontendTauriInvoke;
}

const emptyStatus: ShowClockIpcStatus = {
  running: false,
  role: null,
  state: "STOPPED",
  local_address: null,
  peer_address: null,
  session_id: null,
  node_id: null,
  clock_generation: 0,
  fencing_generation: 0,
  accepted_samples: 0,
  last_sequence: 0,
  offset_us: 0,
  sample_age_us: null,
  output_armed: false,
  last_error: null,
};

const statusPollMs = 500;
const roleKey = "syndocal.showClock.role";
const bindKey = "syndocal.showClock.bind";
const peerKey = "syndocal.showClock.peer";
const nodeKey = "syndocal.showClock.node";
const peerNodeKey = "syndocal.showClock.peerNode";
const projectKey = "syndocal.showClock.projectHash";
const mediaKey = "syndocal.showClock.mediaHash";

function stored(key: string, fallback: string): string {
  try {
    return window.localStorage.getItem(key) ?? fallback;
  } catch {
    return fallback;
  }
}

function phaseLabel(phase: ShowClockIpcPhase): string {
  return phase;
}

function formatAge(age: number | null): string {
  return age === null ? "—" : `${Math.round(age / 1000)} ms`;
}

export function ShowClockStatusPanel(props: ShowClockStatusPanelProps) {
  const [status, setStatus] = createSignal(emptyStatus);
  const [role, setRole] = createSignal<ShowClockIpcRole>(stored(roleKey, "standby") === "primary" ? "primary" : "standby");
  const [bindAddress, setBindAddress] = createSignal(stored(bindKey, "0.0.0.0:44666"));
  const [peerAddress, setPeerAddress] = createSignal(stored(peerKey, "127.0.0.1:44666"));
  // A session is a replay/estimator incarnation. It must be entered again
  // after an app restart, so it is intentionally never restored or stored.
  const [sessionId, setSessionId] = createSignal("");
  const [nodeId, setNodeId] = createSignal(stored(nodeKey, "node-standby"));
  const [peerNodeId, setPeerNodeId] = createSignal(stored(peerNodeKey, "node-primary"));
  const [projectHash, setProjectHash] = createSignal(stored(projectKey, ""));
  const [mediaHash, setMediaHash] = createSignal(stored(mediaKey, ""));
  const [keyHex, setKeyHex] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let disposed = false;
  let pollFlight: Promise<void> | null = null;
  let pollTimer: number | undefined;

  const poll = async () => {
    if (disposed || pollFlight || !props.backendAvailable) return;
    const flight = props.invokeCommand<ShowClockIpcStatus>("get_show_clock_status")
      .then((next) => {
        if (!disposed) setStatus(next);
      })
      .catch((reason) => {
        if (!disposed) setError(String(reason));
      });
    pollFlight = flight;
    await flight.finally(() => {
      if (pollFlight === flight) pollFlight = null;
    });
  };

  onMount(() => {
    if (!props.backendAvailable) return;
    void poll();
    pollTimer = window.setInterval(() => void poll(), statusPollMs);
    onCleanup(() => {
      disposed = true;
      if (pollTimer !== undefined) window.clearInterval(pollTimer);
    });
  });

  const persist = () => {
    for (const [key, value] of [
      [roleKey, role()], [bindKey, bindAddress()], [peerKey, peerAddress()],
      [nodeKey, nodeId()], [peerNodeKey, peerNodeId()],
      [projectKey, projectHash()], [mediaKey, mediaHash()],
    ] as const) {
      window.localStorage.setItem(key, value);
    }
  };

  const start = async () => {
    setBusy(true);
    setError(null);
    try {
      if (!/^[0-9a-fA-F]{64}$/.test(projectHash()) || !/^[0-9a-fA-F]{64}$/.test(mediaHash())) {
        throw new Error("Project and media hashes must each be 64 hexadecimal characters.");
      }
      if (!/^[0-9a-fA-F]{64}$/.test(keyHex())) {
        throw new Error("The pairing key is required as 64 hexadecimal characters; it is never stored.");
      }
      persist();
      const request: ShowClockStartRequest = {
        role: role(),
        bind_address: bindAddress().trim(),
        peer_address: peerAddress().trim(),
        session_id: sessionId().trim(),
        node_id: nodeId().trim(),
        peer_node_id: peerNodeId().trim(),
        project_hash_hex: projectHash().trim(),
        media_hash_hex: mediaHash().trim(),
        key_hex: keyHex().trim(),
        clock_generation: 1,
        fencing_generation: 1,
        bpm_milli: 120000,
        initial_show_time_us: 1000000,
      };
      setStatus(await props.invokeCommand<ShowClockIpcStatus>("start_show_clock", { request }));
      setKeyHex("");
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
      void poll();
    }
  };

  const stop = async () => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await props.invokeCommand<ShowClockIpcStatus>("stop_show_clock"));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section class="showClockPanel" aria-label="ShowClock LAN">
      <header class="ioDeskHeader">
        <h2 class="textBalance">ShowClock LAN</h2>
        <span class="tabularNums">{phaseLabel(status().state)}</span>
      </header>
      <p class="textPretty standbyIntro">
        Manually paired UDP clock samples. Standby follows authenticated samples only; physical output remains fenced until the separate local Arm path succeeds.
      </p>
      <dl class="standbyStatusGrid showClockStatusGrid tabularNums">
        <div><dt>Role</dt><dd>{status().role ?? role()}</dd></div>
        <div><dt>Samples</dt><dd>{status().accepted_samples}</dd></div>
        <div><dt>Offset</dt><dd>{status().offset_us} µs</dd></div>
        <div><dt>Age</dt><dd>{formatAge(status().sample_age_us)}</dd></div>
        <div><dt>Generation</dt><dd>{status().clock_generation} / {status().fencing_generation}</dd></div>
        <div><dt>Sequence</dt><dd>{status().last_sequence}</dd></div>
        <div><dt>Peer</dt><dd>{status().peer_address ?? "—"}</dd></div>
        <div><dt>Output</dt><dd>{status().output_armed ? "Armed" : "Fenced"}</dd></div>
      </dl>
      <details class="advancedOutputControls">
        <summary>Pairing and runtime settings</summary>
        <div class="showClockForm">
          <div class="split">
            <label>Role<select value={role()} disabled={status().running || busy()} onChange={(event) => setRole(event.currentTarget.value as ShowClockIpcRole)}><option value="primary">Primary</option><option value="standby">Standby</option></select></label>
            <label>Node ID<input value={nodeId()} disabled={status().running || busy()} onInput={(event) => setNodeId(event.currentTarget.value)} /></label>
          </div>
          <div class="split">
            <label>Bind address<input value={bindAddress()} disabled={status().running || busy()} onInput={(event) => setBindAddress(event.currentTarget.value)} /></label>
            <label>Peer address<input value={peerAddress()} disabled={status().running || busy()} onInput={(event) => setPeerAddress(event.currentTarget.value)} /></label>
          </div>
          <div class="split">
            <label>Session ID<input value={sessionId()} disabled={status().running || busy()} placeholder="fresh paired session after restart" onInput={(event) => setSessionId(event.currentTarget.value)} /></label>
            <label>Peer node ID<input value={peerNodeId()} disabled={status().running || busy()} onInput={(event) => setPeerNodeId(event.currentTarget.value)} /></label>
          </div>
          <label>Project hash<input value={projectHash()} disabled={status().running || busy()} placeholder="64 hex characters" onInput={(event) => setProjectHash(event.currentTarget.value)} /></label>
          <label>Media hash<input value={mediaHash()} disabled={status().running || busy()} placeholder="64 hex characters" onInput={(event) => setMediaHash(event.currentTarget.value)} /></label>
          <label>Pairing key<input type="password" value={keyHex()} disabled={status().running || busy()} placeholder="64 hex characters; not stored" onInput={(event) => setKeyHex(event.currentTarget.value)} /></label>
        </div>
      </details>
      <div class="buttonRow">
        <button class="primary" onClick={() => void start()} disabled={!props.backendAvailable || status().running || busy()}>Start ShowClock</button>
        <button onClick={() => void stop()} disabled={!props.backendAvailable || !status().running || busy()}>Stop</button>
      </div>
      <p class="showClockEndpoint tabularNums">Local {status().local_address ?? "—"} · Session {status().session_id ?? sessionId()}</p>
      <Show when={error() ?? status().last_error}>{(message) => <p class="fieldError textPretty" role="alert" aria-live="polite">{message()}</p>}</Show>
    </section>
  );
}

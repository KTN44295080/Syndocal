import { createSignal, onCleanup, onMount, Show } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import type {
  ShowClockIpcPhase,
  ShowClockIpcRole,
  ShowClockIpcStatus,
  ShowClockActionKind,
  ShowClockActionPayload,
  ShowClockLatePolicy,
  ShowClockArmOutputRequest,
  ShowClockReArmRequest,
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
  show_clock_gate_armed: false,
  fence_state: "DISARMED",
  accepted_actions: 0,
  scheduled_actions: 0,
  last_action_sequence: 0,
  last_action_id: null,
  last_action_status: null,
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

function newActionId(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
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
  const [actionSequence, setActionSequence] = createSignal(1);
  const [targetShowTime, setTargetShowTime] = createSignal(1500000);
  const [actionKind, setActionKind] = createSignal<ShowClockActionKind>("go");
  const [latePolicy, setLatePolicy] = createSignal<ShowClockLatePolicy>("hold");
  const [cueId, setCueId] = createSignal(1);
  const [videoLayerId, setVideoLayerId] = createSignal(1);
  const [videoFadeMs, setVideoFadeMs] = createSignal(0);
  const [videoSlotId, setVideoSlotId] = createSignal(1);
  const [timelineJumpPosition, setTimelineJumpPosition] = createSignal(0);
  const [confirmOutputArm, setConfirmOutputArm] = createSignal(false);
  const [rearmClockGeneration, setRearmClockGeneration] = createSignal(1);
  const [rearmFencingGeneration, setRearmFencingGeneration] = createSignal(2);
  const [confirmPrimaryStopped, setConfirmPrimaryStopped] = createSignal(false);
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
        project_generation: 1,
        lease_generation: 1,
        audio_generation: 1,
        recording_generation: 1,
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

  const scheduleAction = async () => {
    setBusy(true);
    setError(null);
    try {
      if (role() !== "primary") throw new Error("Only Primary may schedule ShowClock actions.");
      if (!Number.isSafeInteger(actionSequence()) || actionSequence() <= 0 || !Number.isSafeInteger(targetShowTime()) || targetShowTime() <= 0) {
        throw new Error("Action sequence and target show time must be positive integers.");
      }
      const payload: ShowClockActionPayload | null = (() => {
        switch (actionKind()) {
          case "release":
            return { kind: "cue_release", cue_id: cueId() };
          case "take":
            return { kind: "video_take", target_layer_id: videoLayerId(), fade_ms: videoFadeMs() };
          case "clip_launch":
          case "transition":
            return { kind: "clip_launch", layer_id: videoLayerId(), slot_id: videoSlotId(), transition_kind: "Cut", transition_duration_ms: videoFadeMs() };
          case "timeline_jump":
            return { kind: "timeline_jump", position_ms: timelineJumpPosition() };
          default:
            return null;
        }
      })();
      if (payload && (!Number.isSafeInteger(cueId()) || cueId() <= 0 || !Number.isSafeInteger(videoLayerId()) || videoLayerId() <= 0 || !Number.isSafeInteger(videoFadeMs()) || videoFadeMs() < 0 || !Number.isSafeInteger(videoSlotId()) || videoSlotId() <= 0 || !Number.isSafeInteger(timelineJumpPosition()) || timelineJumpPosition() < 0)) {
        throw new Error("Action payload values must be safe non-negative integers; IDs must be positive.");
      }
      setStatus(await props.invokeCommand<ShowClockIpcStatus>("schedule_show_clock_action", {
        request: {
          action_id_hex: newActionId(),
          sequence: actionSequence(),
          target_show_time_us: targetShowTime(),
          action: actionKind(),
          late_policy: latePolicy(),
          payload,
        },
      }));
      setActionSequence((value) => value + 1);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
      void poll();
    }
  };

  const armOutput = async () => {
    setBusy(true);
    setError(null);
    try {
      if (!confirmOutputArm()) throw new Error("Confirm local output ownership before Arm.");
      const request: ShowClockArmOutputRequest = { operator_confirmed: true };
      setStatus(await props.invokeCommand<ShowClockIpcStatus>("arm_show_clock_output", { request }));
      setConfirmOutputArm(false);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
      void poll();
    }
  };

  const hold = async () => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await props.invokeCommand<ShowClockIpcStatus>("hold_show_clock"));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
      void poll();
    }
  };

  const rearm = async () => {
    setBusy(true);
    setError(null);
    try {
      if (!confirmPrimaryStopped()) throw new Error("Confirm that the old Primary is stopped before Re-arm.");
      if (!/^[0-9a-fA-F]{64}$/.test(projectHash()) || !/^[0-9a-fA-F]{64}$/.test(mediaHash())) {
        throw new Error("Project and media hashes must each be 64 hexadecimal characters.");
      }
      const request: ShowClockReArmRequest = {
        operator_confirmed_primary_stopped: true,
        clock_generation: rearmClockGeneration(),
        fencing_generation: rearmFencingGeneration(),
        project_generation: 1,
        lease_generation: 1,
        audio_generation: 1,
        recording_generation: 1,
        project_hash_hex: projectHash().trim(),
        media_hash_hex: mediaHash().trim(),
      };
      setStatus(await props.invokeCommand<ShowClockIpcStatus>("rearm_show_clock", { request }));
      setConfirmPrimaryStopped(false);
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
        <div><dt>Fence</dt><dd>{status().fence_state}</dd></div>
        <div><dt>Actions</dt><dd>{status().scheduled_actions} queued / {status().accepted_actions} accepted</dd></div>
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
        <button onClick={() => void hold()} disabled={!props.backendAvailable || !status().running || busy()}>Manual Hold</button>
      </div>
      <details class="advancedOutputControls">
        <summary>Action scheduling and Re-arm</summary>
        <div class="showClockForm">
          <div class="split">
            <label>Action<select value={actionKind()} disabled={busy()} onChange={(event) => setActionKind(event.currentTarget.value as ShowClockActionKind)}><option value="go">Go</option><option value="stop">Stop</option><option value="back">Back</option><option value="release">Release</option><option value="blackout">Blackout</option><option value="take">Take</option><option value="clip_launch">Clip launch</option><option value="transition">Transition</option><option value="timeline_jump">Timeline jump</option></select></label>
            <label>Late policy<select value={latePolicy()} disabled={busy()} onChange={(event) => setLatePolicy(event.currentTarget.value as ShowClockLatePolicy)}><option value="hold">Hold</option><option value="execute_immediately">Execute immediately</option><option value="drop">Drop</option></select></label>
          </div>
          <div class="split">
            <label>Action sequence<input type="number" min="1" value={actionSequence()} disabled={busy()} onInput={(event) => setActionSequence(Number(event.currentTarget.value))} /></label>
            <label>Target show time (µs)<input type="number" min="1" value={targetShowTime()} disabled={busy()} onInput={(event) => setTargetShowTime(Number(event.currentTarget.value))} /></label>
          </div>
          <Show when={actionKind() === "release"}>
            <label>Cue ID<input type="number" min="1" value={cueId()} disabled={busy()} onInput={(event) => setCueId(Number(event.currentTarget.value))} /></label>
          </Show>
          <Show when={actionKind() === "take" || actionKind() === "clip_launch" || actionKind() === "transition"}>
            <div class="split">
              <label>Video layer ID<input type="number" min="1" value={videoLayerId()} disabled={busy()} onInput={(event) => setVideoLayerId(Number(event.currentTarget.value))} /></label>
              <label>Fade/transition (ms)<input type="number" min="0" value={videoFadeMs()} disabled={busy()} onInput={(event) => setVideoFadeMs(Number(event.currentTarget.value))} /></label>
            </div>
          </Show>
          <Show when={actionKind() === "clip_launch" || actionKind() === "transition"}>
            <label>Video clip slot ID<input type="number" min="1" value={videoSlotId()} disabled={busy()} onInput={(event) => setVideoSlotId(Number(event.currentTarget.value))} /></label>
          </Show>
          <Show when={actionKind() === "timeline_jump"}>
            <label>Timeline position (ms)<input type="number" min="0" value={timelineJumpPosition()} disabled={busy()} onInput={(event) => setTimelineJumpPosition(Number(event.currentTarget.value))} /></label>
          </Show>
          <button onClick={() => void scheduleAction()} disabled={!props.backendAvailable || !status().running || status().role !== "primary" || busy()}>Send authenticated action</button>
          <div class="split">
            <label>Re-arm clock generation<input type="number" min="1" value={rearmClockGeneration()} disabled={busy()} onInput={(event) => setRearmClockGeneration(Number(event.currentTarget.value))} /></label>
            <label>Re-arm fencing generation<input type="number" min="2" value={rearmFencingGeneration()} disabled={busy()} onInput={(event) => setRearmFencingGeneration(Number(event.currentTarget.value))} /></label>
          </div>
          <label class="checkboxLabel"><input type="checkbox" checked={confirmPrimaryStopped()} disabled={busy()} onChange={(event) => setConfirmPrimaryStopped(event.currentTarget.checked)} /> I confirm the old Primary is stopped</label>
          <button onClick={() => void rearm()} disabled={!props.backendAvailable || !status().running || status().role !== "standby" || busy()}>Manual Re-arm</button>
          <label class="checkboxLabel"><input type="checkbox" checked={confirmOutputArm()} disabled={busy()} onChange={(event) => setConfirmOutputArm(event.currentTarget.checked)} /> I confirm local lighting output ownership before Arm</label>
          <button onClick={() => void armOutput()} disabled={!props.backendAvailable || !status().running || status().output_armed || busy()}>Arm local output</button>
          <p class="textPretty standbyIntro">Re-arm clears prior queued actions. Arm then requires local lighting ownership and LOCKED state; scheduled actions are dispatched through the existing EngineHandle boundary.</p>
        </div>
      </details>
      <p class="showClockEndpoint tabularNums">Local {status().local_address ?? "—"} · Session {status().session_id ?? sessionId()}</p>
      <Show when={status().last_action_status}>{(message) => <p class="showClockEndpoint">Action: {message()}</p>}</Show>
      <Show when={error() ?? status().last_error}>{(message) => <p class="fieldError textPretty" role="alert" aria-live="polite">{message()}</p>}</Show>
    </section>
  );
}

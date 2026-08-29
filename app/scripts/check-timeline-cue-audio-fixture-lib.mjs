import assert from "node:assert/strict";

export const cueAudioEventTargets = {
  "syndocal://pane-window-terminal": { kind: "Any" },
  "syndocal://video-output-window-state": { kind: "Any" },
  "tauri://close-requested": { kind: "Window", label: "main" },
  "syndocal://application-update-progress": { kind: "Any" },
  "syndocal://project-authority-replaced": { kind: "Any" },
  "syndocal://project-control-inputs-retired": { kind: "Any" },
  "syndocal://open-project": { kind: "Any" },
  "tauri://drag-enter": { kind: "Webview", label: "main" },
  "tauri://drag-over": { kind: "Webview", label: "main" },
  "tauri://drag-drop": { kind: "Webview", label: "main" },
  "tauri://drag-leave": { kind: "Webview", label: "main" },
};

export const hitVerifiedCdpClick = async (client, selector, text, label) => {
  const evaluate = async (expression) => {
    const result = await client.send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.text);
    return result.result.value;
  };
  const target = await evaluate(`(() => {
    const candidates = [...document.querySelectorAll(${JSON.stringify(selector)})];
    const element = candidates.find((candidate) => candidate.textContent?.trim() === ${JSON.stringify(text)});
    if (!(element instanceof HTMLElement)) return { found: false, candidateCount: 0 };
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    const x = rect.left + rect.width / 2;
    const y = rect.top + rect.height / 2;
    const centerInViewport = x >= 0 && x < innerWidth && y >= 0 && y < innerHeight;
    const hit = centerInViewport ? document.elementFromPoint(x, y) : null;
    return {
      found: true,
      candidateCount: candidates.filter((candidate) => candidate.textContent?.trim() === ${JSON.stringify(text)}).length,
      enabled: !(element instanceof HTMLButtonElement) || !element.disabled,
      x,
      y,
      width: rect.width,
      height: rect.height,
      centerInViewport,
      topHitOwnsTarget: Boolean(hit && (hit === element || element.contains(hit))),
      display: style.display,
      visibility: style.visibility,
    };
  })()`);
  assert.ok(
    target.found && target.candidateCount === 1 && target.enabled && target.centerInViewport && target.topHitOwnsTarget
      && target.width > 0 && target.height > 0 && target.display !== "none" && target.visibility !== "hidden",
    `${label} must be an enabled, visible, unique CDP hit target: ${JSON.stringify(target)}`,
  );
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: target.x, y: target.y, button: "none", buttons: 0 });
  await client.send("Input.dispatchMouseEvent", { type: "mousePressed", x: target.x, y: target.y, button: "left", buttons: 1, clickCount: 1 });
  await client.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: target.x, y: target.y, button: "left", buttons: 0, clickCount: 1 });
  return { ...target, pointerSequence: "mouseMoved>mousePressed>mouseReleased" };
};
export const installCueAudioMock = (eventTargets) => {
  if (!eventTargets || typeof eventTargets !== "object" || Array.isArray(eventTargets)) {
    throw new Error("Cue Audio event target contract is missing");
  }
  const clone = (value) => structuredClone(value);
  const exactKeys = (value, keys) => {
    if (!value || typeof value !== "object" || Array.isArray(value)) return false;
    const actual = Object.keys(value).sort();
    const expected = [...keys].sort();
    return actual.length === expected.length && actual.every((key, index) => key === expected[index]);
  };
  const rendererOwner = /^renderer:[A-Za-z0-9_.:-]{1,119}$/;
  const validateOwnerRegistration = (args) => {
    if (!exactKeys(args, ["ownerId"]) || typeof args.ownerId !== "string" || !rendererOwner.test(args.ownerId)) {
      throw new Error("Cue Audio owner registration contract violated: " + JSON.stringify(args));
    }
  };
  const validateProgramAudioHandoff = (args) => {
    if (!exactKeys(args, ["deviceName", "enabled", "volume"])) {
      throw new Error("Cue Audio Program Audio handoff contract violated: " + JSON.stringify(args));
    }
    if (typeof args.enabled !== "boolean") {
      throw new Error("Cue Audio Program Audio enabled must be boolean");
    }
    if (typeof args.volume !== "number" || !Number.isFinite(args.volume) || args.volume < 0 || args.volume > 1) {
      throw new Error("Cue Audio Program Audio volume must be a finite number in [0,1]");
    }
    if (args.deviceName !== null && (typeof args.deviceName !== "string" || args.deviceName.trim() === "")) {
      throw new Error("Cue Audio Program Audio deviceName must be null or a nonempty string");
    }
  };
  const createProductionSnapshot = () => {
    const defaultOutput = {
      enabled: true,
      protocol: "ArtNet",
      target_ip: "127.0.0.1",
      port: 6454,
      universe: 0,
      serial_port: "",
      serial_baud_rate: 57600,
    };
    const telemetry = {
      frame_counter: 0,
      enabled_effect_count: 0,
      supported_effect_count: 0,
      effects_over_supported_envelope: false,
      queue_depth: 0,
      queue_depth_abs_max: 0,
      queue_push_failure_count: 0,
      last_tick_interval_us: 0,
      tick_jitter_last_us: 0,
      tick_jitter_abs_max_us: 0,
      tick_jitter_stddev_us: 0,
      tick_jitter_p95_us: 0,
      tick_jitter_p99_us: 0,
      tick_jitter_samples: 0,
      last_command_queue_latency_us: 0,
      command_queue_latency_abs_max_us: 0,
      command_queue_latency_p95_us: 0,
      command_queue_latency_p99_us: 0,
      command_queue_latency_samples: 0,
      last_command_drain_count: 0,
      command_drain_abs_max: 0,
      command_drain_limit_hit_count: 0,
      last_command_to_dmx_tick_latency_us: 0,
      command_to_dmx_tick_latency_abs_max_us: 0,
      command_to_dmx_tick_latency_p95_us: 0,
      command_to_dmx_tick_latency_p99_us: 0,
      command_to_dmx_tick_latency_samples: 0,
      last_dmx_send_interval_us: 0,
      dmx_send_interval_min_us: 0,
      dmx_send_interval_max_us: 0,
      dmx_send_interval_samples: 0,
      low_latency_dmx_tick_request_count: 0,
      low_latency_dmx_tick_advance_count: 0,
      low_latency_dmx_tick_defer_count: 0,
      last_packet_bytes: 0,
      last_dmx_output_count: 0,
      last_dmx_send_success_count: 0,
      last_dmx_send_failure_count: 0,
      total_dmx_send_success_count: 0,
      total_dmx_send_failure_count: 0,
      last_dmx_route_results: [],
      last_error: null,
    };
    return {
      fixtures: [],
      cues: [],
      cue_lists: [{ id: 1, label: "Bank 1", active_cue_id: null }],
      palettes: [],
      playback_executors: [{ id: 1, label: "Bank 1", cue_list_id: 1, page: 1, slot: 1, level: 1 }],
      playback_master: 1,
      active_cue_id: null,
      direct_child_timeline_transports: [],
      active_fade: null,
      programmer: { enabled: false, blind: false, values: [], dmx_previews: [] },
      timeline: {
        events: [],
        automations: [],
        video_automations: [],
        audio: null,
        audio_clips: [],
        audio_offset_ms: 0,
        audio_muted: false,
        playing: false,
        position_ms: 0,
        duration_ms: 0,
      },
      video: {
        layers: [],
        media_assets: [],
        compositions: [],
        outputs: [],
        mapping_presets: [],
        master_opacity: 1,
        blackout: false,
      },
      effects: [],
      node_graphs: [],
      output: defaultOutput,
      dmx_outputs: [defaultOutput],
      lighting_master: 1,
      submasters: [],
      blackout: false,
      clock: {
        bpm: 120,
        beat_phase: 0,
        beat_counter: 0,
        tap_count: 0,
        source: "Manual",
        external_sync_age_ms: null,
        external_sync_locked: false,
      },
      stage_map: { locked: false, min_x: -10, max_x: 10, min_z: -10, max_z: 10 },
      stage_map_presets: [],
      stage_objects: [],
      touch_surface: { pages: [] },
      dmx_preview: Array.from({ length: 512 }, () => 0),
      dmx_previews: [{ universe: 0, values: Array.from({ length: 512 }, () => 0) }],
      telemetry,
    };
  };
  const createProductionAuthorityBundle = () => ({
    project_epoch: 0,
    project_revision: 0,
    checkpoint_hash: "browser-authority-checkpoint",
    publication_generation: 0,
    publication_kind: "runtime_status",
    mapping_replacement_generation: 0,
    authority_disposition_generation: 0,
    authority_disposition: "clean_at_path",
    recovery_authority_serial: 0,
    recovery_authority_last_transition: { kind: "legacy_unknown" },
    path_generation: 0,
    history_generation: 0,
    current_project_path: null,
    snapshot: createProductionSnapshot(),
    profiles: [],
    fixture_groups: [],
    operator_policy: null,
    midi_mappings: [],
    osc_mappings: [],
    dmx_mappings: [],
    dj_track_triggers: [],
    history: {
      can_undo: false,
      can_redo: false,
      undo_depth: 0,
      redo_depth: 0,
      undo_label: null,
      redo_label: null,
      project_epoch: 0,
      project_revision: 0,
      checkpoint_hash: "browser-authority-checkpoint",
      history_generation: 0,
      undo_entry_id: null,
      undo_checkpoint_hash: null,
      redo_entry_id: null,
      redo_checkpoint_hash: null,
    },
    input_runtime: {
      project_input_runtime_generation: 0,
      mapping_input_runtime_generation: 0,
      midi_clock_active: false,
      midi_control_active: false,
      midi_feedback_output_active: false,
      midi_feedback_runtime_active: false,
      osc_active: false,
      dmx_active: false,
    },
  });
  const createProductionLiveAudioStatus = () => ({
    running: false,
    stale: false,
    safety_clear_pending: false,
    device_id: null,
    device_name: null,
    backend: null,
    sample_format: null,
    sample_rate: 0,
    channels: 0,
    configured_buffer_frames: null,
    applied_buffer_frames: null,
    channel_mix: { mode: "average_all" },
    bass: 0,
    mid: 0,
    high: 0,
    bands: Array.from({ length: 16 }, () => 0),
    band_count: 0,
    rms: 0,
    peak: 0,
    spectral_flux: 0,
    spectral_centroid: 0,
    spectral_density_fast: 0,
    spectral_density_slow: 0,
    kick_strength: 0,
    snare_strength: 0,
    kick_event: false,
    snare_event: false,
    onset: false,
    onset_strength: 0,
    bpm: null,
    bpm_confidence: 0,
    beat_phase: 0,
    feature_sequence: 0,
    analyzed_windows: 0,
    dropped_chunks: 0,
    dropped_frames: 0,
    backend_xruns: 0,
    callback_count: 0,
    last_callback_frames: 0,
    min_callback_frames: 0,
    max_callback_frames: 0,
    capture_to_worker_us: 0,
    max_capture_to_worker_us: 0,
    queue_depth: 0,
    queue_capacity: 0,
    queue_depth_high_water: 0,
    last_error: null,
    asio_selection: null,
  });
  const createProductionTelemetryReport = () => {
    const snapshot = createProductionSnapshot();
    return {
      version: 1,
      captured_at_unix_ms: 0,
      fixture_count: 0,
      cue_count: 0,
      effect_count: 0,
      node_graph_count: 0,
      video_layer_count: 0,
      video_output_count: 0,
      dmx_output_count: snapshot.dmx_outputs.length,
      enabled_dmx_output_count: snapshot.dmx_outputs.filter((output) => output.enabled).length,
      dmx_preview_universe_count: snapshot.dmx_previews.length,
      clock: clone(snapshot.clock),
      primary_output: clone(snapshot.output),
      dmx_outputs: clone(snapshot.dmx_outputs),
      budget: {
        overall: "Idle",
        target_dmx_frame_rate_hz: 44,
        target_tick_interval_us: 22727,
        tick_jitter_p99_target_us: 5000,
        command_queue_p99_target_us: 5000,
        command_to_dmx_p99_target_us: 5000,
        dmx_send_interval_tolerance_us: 5000,
        checks: [],
      },
      telemetry: clone(snapshot.telemetry),
    };
  };
  const mock = {
    calls: [],
    cueAudioCalls: [],
    endpoints: [],
    settings: { version: 1, route: "follow_program", device_name: null, topology_fingerprint: null, click_gain: 1, guide_gain: 0.85 },
    statusRevision: 0,
    registrationArgs: null,
    programAudioHandoffConfigs: [],
    nextCallbackId: 0,
    nextEventId: 0,
    captureCueAudioCalls: false,
    manualRefreshListSeen: false,
    rejected: [],
    authorityBundle: createProductionAuthorityBundle(),
    authorityBundleDeliveryCount: 0,
    projectControlMappingsCalls: 0,
    knownCommands: [
      "plugin:event|listen",
      "plugin:event|unlisten",
      "plugin:window|is_fullscreen",
      "plugin:window|is_maximized",
      "plugin:window|maximize",
      "register_project_transaction_owner",
      "get_operator_policy",
      "get_project_recovery_authority_status",
      "get_project_authority_bundle",
      "get_fixture_groups",
      "get_project_control_mappings",
      "get_video_output_window_statuses",
      "live_audio_input_status",
      "live_audio_input_backends",
      "set_operator_selection_context",
      "get_snapshot",
      "get_engine_telemetry_report",
      "list_midi_inputs",
      "list_midi_outputs",
      "list_serial_ports",
      "dmx_input_status",
      "list_project_backups",
      "get_project_history_status",
      "get_application_update_configuration",
      "get_fixture_profile_health",
      "remote_access_urls",
      "get_dj_link_machine_status",
      "remote_control_status",
      "list_dj_link_wired_candidates",
      "list_audio_input_devices",
      "get_live_audio_input_capabilities",
      "set_program_audio_handoff_config",
      "list_gdtf_fixture_cache",
      "list_audio_output_devices",
      "set_machine_timeline_cue_audio_settings",
      "load_startup_project",
      "take_open_project_paths",
      "get_timeline_cue_audio_status",
      "get_external_video_transport_status",
      "get_timeline_follow_runtime",
      "get_snapshot_delta",
      "poll_project_authority_bundle",
    ],
  };
  const jsonEqual = (left, right) => JSON.stringify(left) === JSON.stringify(right);
  const status = () => ({
    runtimeIncarnation: 41,
    statusRevision: ++mock.statusRevision,
    desiredSettings: clone(mock.settings),
    appliedSettings: clone(mock.settings),
    settingsRevision: 7,
    lifecycle: "running",
    requestedDeviceName: mock.settings.device_name,
    resolvedDeviceName: mock.settings.route === "explicit_device" ? mock.settings.device_name : "Program Output",
    requestedTopologyFingerprint: mock.settings.topology_fingerprint,
    observedTopologyFingerprint: mock.endpoints.length ? "topology-fingerprint-a" : null,
    topologyGeneration: mock.endpoints.length ? 1 : 0,
    endpoints: clone(mock.endpoints),
    outputClockEpoch: 2,
    scheduleGeneration: 3,
    sourceFence: 4,
    nextOutputFrame: 5,
    callbackLive: true,
    faultCode: "None",
    faultCount: 0,
    faultSequence: 0,
    lastError: null,
    rotationCount: 1,
    stallCount: 0,
    configCount: 1,
  });
  window.__syndocalCueAudioMock = mock;
  window.__TAURI_INTERNALS__ = {
    invoke: async (command, args = {}) => {
      const received = { command, args: clone(args) };
      mock.calls.push(received);
      if (command === "register_project_transaction_owner") {
        if (mock.registrationArgs !== null) throw new Error("Cue Audio owner registration must occur exactly once");
        validateOwnerRegistration(args);
        mock.registrationArgs = clone(args);
        return null;
      }
      if (command === "set_program_audio_handoff_config") {
        validateProgramAudioHandoff(args);
        mock.programAudioHandoffConfigs.push(clone(args));
        return null;
      }
      if (command === "plugin:window|is_fullscreen") {
        if (!exactKeys(args, ["label"]) || args.label !== "main") throw new Error("Cue Audio fullscreen probe payload differs from the exact main-window contract");
        return false;
      }
      if (command === "plugin:window|is_maximized") {
        if (!exactKeys(args, ["label"]) || args.label !== "main") throw new Error("Cue Audio maximized probe payload differs from the exact main-window contract");
        return false;
      }
      if (command === "plugin:window|maximize") {
        if (!exactKeys(args, ["label"]) || args.label !== "main") throw new Error("Cue Audio maximize payload differs from the exact main-window contract");
        return null;
      }
      if (command === "get_operator_policy") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio operator-policy startup payload must be empty");
        return null;
      }
      if (command === "get_project_recovery_authority_status") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio recovery-authority startup payload must be empty");
        return { recovery_authority_serial: 0, last_transition: { kind: "legacy_unknown" } };
      }
      if (command === "get_project_authority_bundle") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio project-authority startup payload must be empty");
        mock.authorityBundleDeliveryCount += 1;
        return clone(mock.authorityBundle);
      }
      if (command === "poll_project_authority_bundle") {
        if (!exactKeys(args, [
          "knownEpoch",
          "knownRevision",
          "knownCheckpointHash",
          "knownPathGeneration",
          "knownHistoryGeneration",
          "knownMappingReplacementGeneration",
          "knownAuthorityDispositionGeneration",
          "knownRecoveryAuthoritySerial",
          "knownProjectInputRuntimeGeneration",
          "knownMappingInputRuntimeGeneration",
        ])) throw new Error("Cue Audio project-authority poll payload differs from the exact startup contract");
        return null;
      }
      if (command === "get_fixture_groups") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio fixture-groups startup payload must be empty");
        return [];
      }
      if (command === "get_project_control_mappings") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio project-control startup payload must be empty");
        const authoritative = mock.projectControlMappingsCalls++ < 2;
        return {
          project_epoch: 0,
          project_revision: 0,
          checkpoint_hash: authoritative ? mock.authorityBundle.checkpoint_hash : "stale-browser-response",
          publication_generation: 0,
          publication_kind: "runtime_status",
          mapping_replacement_generation: 0,
          path_generation: 0,
          history_generation: 0,
          midi_mappings: [],
          osc_mappings: [],
          dmx_mappings: [],
          dj_track_triggers: [],
        };
      }
      if (command === "get_video_output_window_statuses") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio video-output startup payload must be empty");
        return [];
      }
      if (command === "live_audio_input_status") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio live-audio status startup payload must be empty");
        return createProductionLiveAudioStatus();
      }
      if (command === "live_audio_input_backends") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio live-audio backend startup payload must be empty");
        return [
          { id: "wasapi_shared", label: "WASAPI Shared", built: true, requires_explicit_device: false, distribution: "bundled", availability: "ready", availability_detail: null },
          { id: "asio", label: "ASIO", built: false, requires_explicit_device: true, distribution: "optional", availability: "not_packaged", availability_detail: "ASIO is not packaged in this browser gate." },
        ];
      }
      if (command === "set_operator_selection_context") {
        if (!exactKeys(args, ["context"]) || !args.context || !exactKeys(args.context, ["fixture_ids", "attributes"])) {
          throw new Error("Cue Audio operator-selection startup payload differs from the exact context contract");
        }
        return null;
      }
      if (command === "get_snapshot") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio snapshot startup payload must be empty");
        return clone(mock.authorityBundle.snapshot);
      }
      if (command === "get_snapshot_delta") {
        if (!exactKeys(args, ["clientRevision"]) || (args.clientRevision !== null && !Number.isSafeInteger(args.clientRevision))) {
          throw new Error("Cue Audio snapshot-delta payload differs from the exact revision contract");
        }
        return { revision: 0, full: clone(mock.authorityBundle.snapshot) };
      }
      if (command === "get_engine_telemetry_report") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio telemetry startup payload must be empty");
        return createProductionTelemetryReport();
      }
      if (command === "list_midi_inputs" || command === "list_midi_outputs" || command === "list_serial_ports" || command === "list_project_backups" || command === "get_fixture_profile_health" || command === "list_gdtf_fixture_cache") {
        if (!exactKeys(args, [])) throw new Error(`Cue Audio ${command} startup payload must be empty`);
        return [];
      }
      if (command === "dmx_input_status") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio DMX status startup payload must be empty");
        return { running: false, signal_present: false, packets_received: 0, invalid_packets: 0, last_packet_unix_ms: null, source_address: null };
      }
      if (command === "get_project_history_status") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio history startup payload must be empty");
        return clone(mock.authorityBundle.history);
      }
      if (command === "get_application_update_configuration") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio update startup payload must be empty");
        return { enabled: false, current_version: "1.2.0-alpha.27", channel: "alpha", endpoint_origin: null, reason: "disabled in browser gate" };
      }
      if (command === "remote_access_urls") {
        if (!exactKeys(args, ["config"]) || !args.config || typeof args.config !== "object") throw new Error("Cue Audio remote-access startup payload differs from the exact config contract");
        return [];
      }
      if (command === "get_dj_link_machine_status") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio DJ Link startup payload must be empty");
        return { configured: false, credentialReady: false, autoStartArmed: false, bindIp: null, bindPort: null, networkGuid: null, adapterGuid: null, credentialGeneration: null, credentialCleanupPending: false, blockReason: null };
      }
      if (command === "remote_control_status") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio remote-control startup payload must be empty");
        return { running: false, web_remote_enabled: true, dj_link_enabled: false, active_connections: 0, rejected_connections: 0, clients: [], dj_link: null };
      }
      if (command === "list_dj_link_wired_candidates") {
        if (!exactKeys(args, [])) throw new Error(`Cue Audio ${command} startup payload must be empty`);
        return [];
      }
      if (command === "list_audio_input_devices") {
        if (!exactKeys(args, ["request"])
          || !exactKeys(args.request, ["schemaVersion", "backend"])
          || args.request.schemaVersion !== 1
          || !["wasapiShared", "asio"].includes(args.request.backend)) {
          throw new Error("Cue Audio list_audio_input_devices startup payload must contain one exact V1 request");
        }
        return [];
      }
      if (command === "get_live_audio_input_capabilities") {
        const request = args?.request;
        if (!exactKeys(args, ["request"])
          || !exactKeys(request, ["schemaVersion", "backend", "deviceId", "sampleRate"])
          || request.schemaVersion !== 1
          || !["wasapiShared", "asio"].includes(request.backend)
          || (request.deviceId !== null && typeof request.deviceId !== "string")
          || (request.sampleRate !== null && (typeof request.sampleRate !== "number" || !Number.isFinite(request.sampleRate)))) {
          throw new Error("Cue Audio live-audio capabilities payload differs from the exact V1 startup contract");
        }
        return {
          device_id: request.deviceId,
          device_name: "Program Audio",
          backend: request.backend,
          default_config: { channels: 2, sample_rate: request.sampleRate ?? 48_000, sample_format: "f32" },
          supported_configs: [],
          resolved_config: { channels: 2, sample_rate: request.sampleRate ?? 48_000, sample_format: "f32" },
          max_capture_frames: 0,
        };
      }
      if (command === "load_startup_project") {
        if (!exactKeys(args, ["ownerId", "expectedEpoch", "expectedRevision", "expectedCheckpointHash"])) throw new Error("Cue Audio startup-project payload differs from the exact authority contract");
        return null;
      }
      if (command === "take_open_project_paths") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio open-project startup payload must be empty");
        return [];
      }
      if (command === "get_timeline_follow_runtime") {
        if (!exactKeys(args, ["expectedEpoch", "expectedRevision", "expectedCheckpointHash", "ownerId"])) throw new Error("Cue Audio Timeline Follow startup payload differs from the exact authority contract");
        return {
          project_epoch: 0,
          project_revision: 0,
          checkpoint_hash: "browser-authority-checkpoint",
          runtime: { epoch: 0, generation: 0, status: "idle", admission_reason: null, outcome: null, source_timeline_id: null, target_timeline_id: null, elapsed_ms: 0, duration_ms: 0, progress_millis: 0, fault: null, settlement: null },
        };
      }
      if (command === "get_external_video_transport_status") {
        if (!exactKeys(args, [])) throw new Error("Cue Audio external-video transport status payload must be empty");
        return {
          active_routes: [],
          active_count: 0,
          capture_faults: [],
          ownership_allowed: false,
          ownership_state: "Failed",
          ownership_reason: "StartupDenied",
          ownership_error: "Machine output ownership has not been initialized",
        };
      }
      if (command === "list_audio_output_devices") {
        if (mock.captureCueAudioCalls) {
          mock.cueAudioCalls.push(received);
          mock.manualRefreshListSeen = true;
        }
        mock.endpoints = [
          { name: "Duplicate Program", occurrences: 2, selectable: false },
          { name: "Exact Program", occurrences: 1, selectable: true },
        ];
        return ["Duplicate Program", "Duplicate Program", "Exact Program"];
      }
      if (command === "get_timeline_cue_audio_status") {
        if (mock.captureCueAudioCalls && mock.manualRefreshListSeen) {
          mock.cueAudioCalls.push(received);
          mock.captureCueAudioCalls = false;
          mock.manualRefreshListSeen = false;
        }
        return status();
      }
      if (command === "set_machine_timeline_cue_audio_settings") {
        if (mock.captureCueAudioCalls) mock.cueAudioCalls.push(received);
        mock.settings = clone(args.settings);
        return status();
      }
      throw new Error("Unexpected Cue Audio invoke: " + command);
    },
  };
  window.__TAURI_INTERNALS__.metadata = {
    currentWindow: { label: "main" },
    currentWebview: { label: "main" },
  };
  window.__TAURI_INTERNALS__.transformCallback = () => ++mock.nextCallbackId;
  const invoke = window.__TAURI_INTERNALS__.invoke;
  window.__TAURI_INTERNALS__.invoke = async (command, args = {}) => {
    try {
      if (command === "plugin:event|listen") {
        const expectedTarget = eventTargets[args?.event];
        if (!exactKeys(args, ["event", "target", "handler"])
          || typeof args.event !== "string"
          || !expectedTarget
          || !jsonEqual(args.target, expectedTarget)
          || !Number.isSafeInteger(args.handler)
          || args.handler <= 0) {
          throw new Error("Cue Audio event-listen payload differs from the exact known event contract");
        }
        mock.calls.push({ command, args: clone(args) });
        return ++mock.nextEventId;
      }
      if (command === "plugin:event|unlisten") {
        if (!exactKeys(args, ["event", "eventId"])
          || !eventTargets[args.event]
          || typeof args.eventId !== "number"
          || !Number.isSafeInteger(args.eventId)
          || args.eventId <= 0) {
          throw new Error("Cue Audio event-unlisten payload differs from the exact known event contract");
        }
        mock.calls.push({ command, args: clone(args) });
        return null;
      }
      return await invoke(command, args);
    } catch (error) {
      mock.rejected.push({ command, reason: String(error?.message ?? error) });
      throw error;
    }
  };
};

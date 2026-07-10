import type { Accessor, Setter } from "solid-js";
import { videoFrameToDataUrl } from "./videoFrameCanvas";
import { defaultColorAdjust, defaultFxAdjust, defaultTransform } from "./videoLayerDefaults";
import type {
  EngineSnapshot,
  ExternalVideoIoPlans,
  ExternalVideoTransportDriverEvent,
  ExternalVideoTransportStatus,
  ExternalVideoTransportSyncReport,
  ExternalVideoTransportSyncResponse,
  VideoBlendMode,
  VideoFrame,
  VideoLayerState,
  VideoOutputRenderPlan,
  VideoOutputWindowCloseSummary,
  VideoOutputWindowStatus,
  VideoOutputWindowSyncSummary,
  VideoPreviewDiagnostics,
  VideoRuntimeStatus,
  VideoSourceKind,
} from "./types";

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface VideoRuntimeControllerOptions {
  invoke: Invoke;
  snapshot: Accessor<EngineSnapshot>;
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
  setMessage: (message: string) => unknown;
  videoSourceKind: Accessor<VideoSourceKind>;
  videoLabel: Accessor<string>;
  setVideoLabel: Setter<string>;
  videoPath: Accessor<string>;
  setVideoPreviewUrl: Setter<string>;
  setVideoPreviewInfo: Setter<string>;
  setVideoPreviewDiagnostics: Setter<VideoPreviewDiagnostics | null>;
  setVideoOutputRenderPlans: Setter<VideoOutputRenderPlan[] | null>;
  setVideoOutputWindowStatuses: Setter<VideoOutputWindowStatus[] | null>;
  setVideoRuntimeStatus: Setter<VideoRuntimeStatus | null>;
  setExternalVideoIoPlans: Setter<ExternalVideoIoPlans | null>;
  setExternalVideoTransportStatus: Setter<ExternalVideoTransportStatus | null>;
  setExternalVideoTransportReport: Setter<ExternalVideoTransportSyncReport | null>;
  setExternalVideoTransportEvents: Setter<ExternalVideoTransportDriverEvent[]>;
  setVideoOutputPreviewUrl: Setter<string>;
  setVideoOutputPreviewInfo: Setter<string>;
  setVideoOutputPreviewId: Setter<number | null>;
  setVideoOutputPreviewMode: Setter<"output" | "test">;
}

export function createVideoRuntimeController(options: VideoRuntimeControllerOptions) {
  const addVideoLayer = async () => {
    try {
      const sourceKind = options.videoSourceKind();
      const layerId = sourceKind === "File" || sourceKind === "StillImage"
        ? await options.invoke<number>(sourceKind === "StillImage" ? "add_still_image_layer" : "add_video_file_layer", {
          label: options.videoLabel(), path: options.videoPath(),
        })
        : await options.invoke<number>("add_video_input_layer", {
          label: options.videoLabel(), kind: sourceKind, name: options.videoPath(),
        });
      options.setVideoLabel(`Layer ${options.snapshot().video.layers.length + 2}`);
      options.setMessage(`Added video layer ${layerId}`);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };
  const removeVideoLayer = async (layerId: number) => {
    try {
      await options.invoke("remove_video_layer", { layerId });
      options.setMessage(`Removed video layer ${layerId}`);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };
  const duplicateVideoLayer = async (layer: { id: number; label: string }) => {
    try {
      const layerId = await options.invoke<number>("duplicate_video_layer", {
        sourceLayerId: layer.id, label: `${layer.label} Copy`,
      });
      options.setMessage(`Duplicated video layer ${layerId}`);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };
  const moveVideoLayer = async (layerId: number, delta: -1 | 1) => {
    const layerIds = options.snapshot().video.layers.map((layer) => layer.id);
    const index = layerIds.indexOf(layerId);
    const nextIndex = index + delta;
    if (index < 0 || nextIndex < 0 || nextIndex >= layerIds.length) return;
    const nextLayerIds = [...layerIds];
    [nextLayerIds[index], nextLayerIds[nextIndex]] = [nextLayerIds[nextIndex], nextLayerIds[index]];
    try {
      await options.invoke("set_video_layer_order", { layerIds: nextLayerIds });
      options.setMessage(`Moved video layer ${layerId}`);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };
  const setVideoLayerLabel = async (layerId: number, label: string) => {
    try {
      await options.invoke("set_video_layer_label", { layerId, label });
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };
  const refreshVideoLayerMetadata = async (layerId: number) => {
    try {
      options.setMessage(await options.invoke<string>("refresh_video_layer_metadata", { layerId }));
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };

  const refreshVideoPreviewDiagnostics = async (silent = false) => {
    try {
      options.setVideoPreviewDiagnostics(await options.invoke<VideoPreviewDiagnostics>("get_video_preview_diagnostics"));
      if (!silent) options.setMessage("Updated video preview diagnostics.");
    } catch (error) { options.setMessage(String(error)); }
  };
  const renderDebugVideoPreview = async () => {
    try {
      const frame = await options.invoke<VideoFrame>("get_debug_video_preview", { width: 64, height: 36 });
      options.setVideoPreviewUrl(videoFrameToDataUrl(frame));
      options.setVideoPreviewInfo(`${frame.width}x${frame.height} ${frame.format} / pts ${frame.pts_ms}ms / ${frame.data.length} bytes`);
      options.setMessage("Rendered CPU video preview.");
      await refreshVideoPreviewDiagnostics(true);
    } catch (error) {
      options.setVideoPreviewUrl("");
      options.setMessage(String(error));
    }
  };
  const refreshVideoOutputRenderPlans = async (silent = false) => {
    try {
      const plans = await options.invoke<VideoOutputRenderPlan[]>("get_video_output_render_plans");
      options.setVideoOutputRenderPlans(plans);
      if (!silent) {
        const activeLayers = plans.reduce((total, plan) => total + plan.composition.layers.length, 0);
        options.setMessage(`Video output render plans: ${plans.length} output(s), ${activeLayers} render layer(s).`);
      }
    } catch (error) { options.setMessage(String(error)); }
  };
  const refreshVideoOutputWindowStatuses = async (silent = false) => {
    try {
      const statuses = await options.invoke<VideoOutputWindowStatus[]>("get_video_output_window_statuses");
      options.setVideoOutputWindowStatuses(statuses);
      if (!silent) {
        const liveOpen = statuses.filter((status) => status.live_open).length;
        const patternOpen = statuses.filter((status) => status.test_pattern_open).length;
        options.setMessage(`Video output windows: ${liveOpen} live, ${patternOpen} pattern open.`);
      }
    } catch (error) { options.setMessage(String(error)); }
  };
  const refreshSnapshotAndVideoOutputRenderPlans = async () => {
    await Promise.all([options.refreshSnapshot(), refreshVideoOutputRenderPlans(true), refreshVideoOutputWindowStatuses(true)]);
  };
  const syncOpenVideoOutputWindows = async () => {
    try {
      const summary = await options.invoke<VideoOutputWindowSyncSummary>("sync_open_video_output_windows");
      await Promise.all([refreshVideoOutputRenderPlans(true), refreshVideoOutputWindowStatuses(true)]);
      options.setMessage(
        `Synced ${summary.synced_live} live and ${summary.synced_test_pattern} pattern output window(s); ${summary.skipped_closed} closed slot(s) skipped.`,
      );
    } catch (error) { options.setMessage(String(error)); }
  };
  const closeOpenVideoOutputWindows = async () => {
    try {
      const summary = await options.invoke<VideoOutputWindowCloseSummary>("close_open_video_output_windows");
      await refreshVideoOutputWindowStatuses(true);
      options.setMessage(
        `Closed ${summary.closed_live} live and ${summary.closed_test_pattern} pattern output window(s); ${summary.skipped_closed} closed slot(s) skipped.`,
      );
    } catch (error) { options.setMessage(String(error)); }
  };
  const openAllVideoOutputWindows = async (testPattern = false) => {
    const displayOutputs = options.snapshot().video.outputs.filter((output) => output.kind === "Display");
    if (displayOutputs.length === 0) {
      options.setMessage("No Display video outputs to open.");
      return;
    }
    let opened = 0;
    let failed = 0;
    for (const output of displayOutputs) {
      try {
        await options.invoke("open_video_output_window", { outputId: output.id, testPattern });
        opened += 1;
      } catch { failed += 1; }
    }
    await Promise.all([refreshVideoOutputRenderPlans(true), refreshVideoOutputWindowStatuses(true)]);
    options.setMessage(
      `Opened ${opened}/${displayOutputs.length} ${testPattern ? "pattern" : "live"} output window(s)${failed > 0 ? `; ${failed} failed` : ""}.`,
    );
  };
  const refreshVideoRuntimeStatus = async () => {
    try {
      const status = await options.invoke<VideoRuntimeStatus>("get_video_runtime_status");
      options.setVideoRuntimeStatus(status);
      options.setMessage(`Video backends: ${status.backends.filter((backend) => backend.state === "Available").length}/${status.backends.length} available.`);
    } catch (error) { options.setMessage(String(error)); }
  };
  const refreshExternalVideoIoPlans = async () => {
    try {
      const [plans, status, transportStatus] = await Promise.all([
        options.invoke<ExternalVideoIoPlans>("get_external_video_io_plans"),
        options.invoke<VideoRuntimeStatus>("get_video_runtime_status"),
        options.invoke<ExternalVideoTransportStatus>("get_external_video_transport_status"),
      ]);
      options.setExternalVideoIoPlans(plans);
      options.setVideoRuntimeStatus(status);
      options.setExternalVideoTransportStatus(transportStatus);
      const available = status.backends.filter((backend) => backend.state === "Available").length;
      options.setMessage(
        `External video I/O: ${plans.inputs.length} input(s), ${plans.outputs.length} output(s), ${transportStatus.active_count} active route(s), ${available}/${status.backends.length} backend(s) available.`,
      );
    } catch (error) { options.setMessage(String(error)); }
  };
  const syncExternalVideoTransports = async () => {
    try {
      const [plans, status, sync] = await Promise.all([
        options.invoke<ExternalVideoIoPlans>("get_external_video_io_plans"),
        options.invoke<VideoRuntimeStatus>("get_video_runtime_status"),
        options.invoke<ExternalVideoTransportSyncResponse>("sync_external_video_transports"),
      ]);
      const transportStatus = await options.invoke<ExternalVideoTransportStatus>("get_external_video_transport_status");
      options.setExternalVideoIoPlans(plans);
      options.setVideoRuntimeStatus(status);
      options.setExternalVideoTransportStatus(transportStatus);
      options.setExternalVideoTransportReport(sync.report);
      options.setExternalVideoTransportEvents(sync.events);
      const failedRoutes = sync.report.start_failed.length + sync.report.stop_failed.length;
      options.setMessage(
        `External video routes: ${sync.report.active_count} active, ${sync.report.started.length} started, ${sync.report.stopped.length} stopped, ${sync.report.blocked.length} blocked, ${failedRoutes} failed, ${sync.events.length} driver event(s).`,
      );
    } catch (error) { options.setMessage(String(error)); }
  };
  const renderDebugVideoOutputPreview = async (outputId: number, testPattern = false) => {
    try {
      const frame = await options.invoke<VideoFrame>(
        testPattern ? "get_debug_video_output_test_pattern" : "get_debug_video_output_preview",
        { outputId, width: 128, height: 72 },
      );
      const previewUrl = videoFrameToDataUrl(frame);
      const label = testPattern ? "Pattern" : "Output";
      const info = `${frame.width}x${frame.height} ${frame.format} / ${frame.data.length} bytes`;
      options.setVideoOutputPreviewUrl(previewUrl);
      options.setVideoOutputPreviewInfo(info);
      options.setVideoOutputPreviewId(outputId);
      options.setVideoOutputPreviewMode(testPattern ? "test" : "output");
      options.setVideoPreviewUrl(previewUrl);
      options.setVideoPreviewInfo(`${label} ${outputId}: ${info}`);
      options.setMessage(`Rendered ${testPattern ? "test pattern" : "output"} ${outputId} preview.`);
      if (!testPattern) await Promise.all([refreshVideoPreviewDiagnostics(true), refreshVideoOutputRenderPlans(true)]);
    } catch (error) {
      options.setVideoOutputPreviewUrl("");
      options.setVideoOutputPreviewId(outputId);
      options.setVideoPreviewUrl("");
      options.setMessage(String(error));
    }
  };

  const setVideoLayerState = async (layerId: number, stateValue: VideoLayerState) => {
    try {
      await options.invoke("set_video_layer_state", { layerId, stateValue });
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };
  const setVideoLayerTransform = (layerId: number, stateValue: VideoLayerState, patch: Partial<VideoLayerState["transform"]>) =>
    setVideoLayerState(layerId, { ...stateValue, transform: { ...defaultTransform, ...stateValue.transform, ...patch } });
  const setVideoLayerColor = (layerId: number, stateValue: VideoLayerState, patch: Partial<VideoLayerState["color"]>) =>
    setVideoLayerState(layerId, { ...stateValue, color: { ...defaultColorAdjust, ...stateValue.color, ...patch } });
  const setVideoLayerFx = (layerId: number, stateValue: VideoLayerState, patch: Partial<VideoLayerState["fx"]>) =>
    setVideoLayerState(layerId, { ...stateValue, fx: { ...defaultFxAdjust, ...stateValue.fx, ...patch } });
  const addVideoCuePoint = async (layerId: number, positionMs?: number) => {
    try {
      await options.invoke("add_video_cue_point", { layerId, positionMs });
      options.setMessage(`Added video cue point${positionMs === undefined ? "" : ` at ${positionMs}ms`}`);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };
  const removeVideoCuePoint = async (layerId: number, positionMs: number) => {
    try {
      await options.invoke("remove_video_cue_point", { layerId, positionMs });
      options.setMessage(`Removed video cue point ${positionMs}ms`);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };
  const jumpVideoCuePoint = async (layerId: number, cuePointIndex: number) => {
    try {
      await options.invoke("jump_video_cue_point", { layerId, cuePointIndex });
      options.setMessage(`Jumped video layer ${layerId} to cue point ${cuePointIndex + 1}`);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };
  const setVideoLayerBlendMode = async (layerId: number, blendMode: VideoBlendMode) => {
    try {
      await options.invoke("set_video_layer_blend_mode", { layerId, blendMode });
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };
  const setVideoMasterOpacity = async (opacity: number) => {
    try {
      await options.invoke("set_video_master_opacity", { opacity });
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };
  const setVideoBlackout = async (enabled: boolean) => {
    try {
      await options.invoke("set_video_blackout", { enabled });
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };

  return {
    addVideoLayer, removeVideoLayer, duplicateVideoLayer, moveVideoLayer, setVideoLayerLabel,
    refreshVideoLayerMetadata, renderDebugVideoPreview, refreshVideoPreviewDiagnostics,
    refreshVideoOutputRenderPlans, refreshVideoOutputWindowStatuses, refreshSnapshotAndVideoOutputRenderPlans,
    syncOpenVideoOutputWindows, closeOpenVideoOutputWindows, openAllVideoOutputWindows,
    refreshVideoRuntimeStatus, refreshExternalVideoIoPlans, syncExternalVideoTransports,
    renderDebugVideoOutputPreview, setVideoLayerState, setVideoLayerTransform, setVideoLayerColor,
    setVideoLayerFx, addVideoCuePoint, removeVideoCuePoint, jumpVideoCuePoint,
    setVideoLayerBlendMode, setVideoMasterOpacity, setVideoBlackout,
  };
}

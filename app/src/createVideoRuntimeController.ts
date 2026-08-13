import type { Accessor, Setter } from "solid-js";
import { videoFrameToDataUrl } from "./videoFrameCanvas";
import { defaultColorAdjust, defaultFxAdjust, defaultTransform } from "./videoLayerDefaults";
import {
  mediaAssetImportReportMessage,
  mediaAssetRelinkOutcomeMessage,
  type MediaAssetOperationPreflight,
  type MediaAssetOperationLease,
  preflightAndBeginMediaAssetOperation,
  prepareFinalizeAndCommitMediaAssetRelink,
  prepareFinalizeAndCommitMediaAssets,
} from "./mediaAssetAuthority";
import type { ProjectAuthorityToken } from "./projectAuthority";
import type {
  EngineSnapshot,
  ExternalVideoIoPlans,
  ExternalVideoTransportDriverEvent,
  ExternalVideoTransportStatus,
  ExternalVideoTransportSyncReport,
  ExternalVideoTransportSyncResponse,
  MediaAssetId,
  MediaAssetImportReport,
  MediaAssetOperationPhase,
  MediaAssetRelinkOutcome,
  VideoBlendMode,
  VideoAudioMonitorStatus,
  VideoFrame,
  VideoIsfEffectSummary,
  VideoLayerState,
  VideoOutputRenderPlan,
  VideoOutputWindowCloseSummary,
  VideoOutputWindowStatus,
  VideoOutputWindowSyncSummary,
  VideoPreviewDiagnostics,
  VideoRuntimeStatus,
  VideoRecordingStatus,
  VideoSourceKind,
} from "./types";

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface VideoRuntimeControllerOptions {
  invoke: Invoke;
  snapshot: Accessor<EngineSnapshot>;
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
  setMessage: (message: string) => unknown;
  videoSourceKind: Accessor<VideoSourceKind>;
  /** Captured before any picker/prepare so A cannot commit or report into B. */
  getCurrentProjectAuthority: Accessor<ProjectAuthorityToken>;
  /**
   * Operator-gated mapping flush before registration. Returns the exact
   * post-flush E/R/H for same-project terminal side effects.
   */
  prepareMediaAssetOperationStart: (expectedAuthority: ProjectAuthorityToken) => Promise<MediaAssetOperationPreflight>;
  /** Live E/R/H check immediately before any terminal UI side effect. */
  isProjectAuthorityCurrent: (authority: ProjectAuthorityToken) => boolean;
  /** Uses the full E/R/H captured before Browse, then clears that one-shot fence. */
  consumeVideoSourceExpectedAuthority: () => ProjectAuthorityToken;
  /** Registers one picker/hash/commit operation for replacement/unmount abort. */
  beginMediaAssetOperation: (
    label: string,
    phase: MediaAssetOperationPhase,
  ) => MediaAssetOperationLease;
  setMediaAssetImportReport: (report: MediaAssetImportReport | null) => void;
  confirmMediaAssetAdoption: (assetId: MediaAssetId, replacementPath: string) => boolean;
  projectTransactionOwnerId: string;
  videoLabel: Accessor<string>;
  setVideoLabel: Setter<string>;
  videoPath: Accessor<string>;
  setVideoPreviewUrl: Setter<string>;
  setVideoPreviewInfo: Setter<string>;
  setVideoPreviewDiagnostics: Setter<VideoPreviewDiagnostics | null>;
  isIsfEventPulseBusy: Accessor<boolean>;
  setIsfEventPulseBusy: Setter<boolean>;
  setVideoOutputRenderPlans: Setter<VideoOutputRenderPlan[] | null>;
  setVideoOutputWindowStatuses: Setter<VideoOutputWindowStatus[] | null>;
  setVideoRuntimeStatus: Setter<VideoRuntimeStatus | null>;
  setVideoAudioMonitorStatus: Setter<VideoAudioMonitorStatus>;
  setAudioOutputDevices: Setter<string[]>;
  setVideoRecordingStatus: Setter<VideoRecordingStatus>;
  setExternalVideoIoPlans: Setter<ExternalVideoIoPlans | null>;
  setExternalVideoTransportStatus: Setter<ExternalVideoTransportStatus | null>;
  setExternalVideoTransportReport: Setter<ExternalVideoTransportSyncReport | null>;
  setExternalVideoTransportEvents: Setter<ExternalVideoTransportDriverEvent[]>;
  setVideoOutputPreviewUrl: Setter<string>;
  setVideoOutputPreviewInfo: Setter<string>;
  setVideoOutputPreviewId: Setter<number | null>;
  setVideoOutputPreviewMode: Setter<"output" | "test">;
}

type MediaAssetRelinkUiResult = {
  outcome: MediaAssetRelinkOutcome;
  terminalAuthority: ProjectAuthorityToken;
};

const sameProjectAuthority = (left: ProjectAuthorityToken, right: ProjectAuthorityToken): boolean =>
  left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.checkpoint_hash === right.checkpoint_hash;

export function createVideoRuntimeController(options: VideoRuntimeControllerOptions) {
  const setMessageIfAuthorityCurrent = (authority: ProjectAuthorityToken, message: string) => {
    if (options.isProjectAuthorityCurrent(authority)) options.setMessage(message);
  };
  const canMutateVideoIsf = () => {
    if (!options.isIsfEventPulseBusy()) return true;
    options.setMessage("Wait for the active FX Event pulse to finish.");
    return false;
  };
  const addVideoLayer = async () => {
    let sideEffectAuthority = options.getCurrentProjectAuthority();
    try {
      const sourceKind = options.videoSourceKind();
      if (sourceKind === "File" || sourceKind === "StillImage") {
        const path = options.videoPath().trim();
        if (!path) {
          options.setMessage("Choose a local media file before adding a layer.");
          return;
        }
        const initiatingAuthority = options.consumeVideoSourceExpectedAuthority();
        sideEffectAuthority = initiatingAuthority;
        const preparedStart = await preflightAndBeginMediaAssetOperation(
          initiatingAuthority,
          options.prepareMediaAssetOperationStart,
          () => options.beginMediaAssetOperation(`Add ${sourceKind === "StillImage" ? "still" : "video"} layer`, "preparing"),
        );
        const operationAuthority = preparedStart.authority;
        sideEffectAuthority = operationAuthority;
        const mediaOperation = preparedStart.operation;
        try {
          if (!options.isProjectAuthorityCurrent(operationAuthority)) return;
          const staged = await prepareFinalizeAndCommitMediaAssets({
            invoke: options.invoke,
            kind: sourceKind,
            paths: [path],
            expectedEpoch: operationAuthority.project_epoch,
            expectedAuthority: operationAuthority,
            ownerId: options.projectTransactionOwnerId,
            signal: mediaOperation.signal,
            onPhase: mediaOperation.setPhase,
            onReport: options.setMediaAssetImportReport,
            commitCommand: sourceKind === "StillImage"
              ? "commit_prepared_still_image_layer_authoritative"
              : "commit_prepared_video_file_layer_authoritative",
            commitArgs: { label: options.videoLabel() },
          });
          const layerId = staged.committed;
          sideEffectAuthority = staged.terminalAuthority;
          if (!staged.applicationCurrent || !options.isProjectAuthorityCurrent(staged.terminalAuthority)) {
            return;
          }
          options.setMediaAssetImportReport(staged.report);
          if (layerId === null) {
            options.setMessage(mediaAssetImportReportMessage(staged.report));
            return;
          }
          options.setVideoLabel(`Layer ${options.snapshot().video.layers.length + 2}`);
          const failures = staged.report.failed + staged.report.skipped;
          options.setMessage(
            `Added video layer ${layerId} from 1 prepared media source.${failures > 0 ? ` ${mediaAssetImportReportMessage(staged.report)}` : ""}`,
          );
          await options.refreshSnapshot();
        } finally {
          mediaOperation.release();
        }
        return;
      }
      const layerId = await options.invoke<number>("add_video_input_layer", {
        label: options.videoLabel(), kind: sourceKind, name: options.videoPath(),
      });
      options.setVideoLabel(`Layer ${options.snapshot().video.layers.length + 2}`);
      options.setMessage(`Added video layer ${layerId}`);
      await options.refreshSnapshot();
    } catch (error) {
      // A picker/add error from A is not an error of a replacement B/C show.
      // The actual current operation owns the status line after its E/R/H wins.
      setMessageIfAuthorityCurrent(sideEffectAuthority, String(error));
    }
  };
  const importMediaFiles = async () => {
    const kind = options.videoSourceKind();
    if (kind !== "File" && kind !== "StillImage") {
      options.setMessage("Batch import supports local video files and still images only.");
      return;
    }
    const initiatingAuthority = options.getCurrentProjectAuthority();
    let sideEffectAuthority = initiatingAuthority;
    try {
      // Fence before the native picker: dialog A, hashing A, and commit A all
      // use this identity. App's mutation wrapper rejects a later project B.
      const preparedStart = await preflightAndBeginMediaAssetOperation(
        initiatingAuthority,
        options.prepareMediaAssetOperationStart,
        () => options.beginMediaAssetOperation("Import media", "picker"),
      );
      const operationAuthority = preparedStart.authority;
      sideEffectAuthority = operationAuthority;
      const mediaOperation = preparedStart.operation;
      try {
        const paths = await options.invoke<string[]>("select_video_source_files", { kind });
        if (!options.isProjectAuthorityCurrent(operationAuthority)) return;
        if (paths.length === 0) {
          options.setMessage("Media import canceled.");
          return;
        }
        const staged = await prepareFinalizeAndCommitMediaAssets({
          invoke: options.invoke,
          kind,
          paths,
          expectedEpoch: operationAuthority.project_epoch,
          expectedAuthority: operationAuthority,
          ownerId: options.projectTransactionOwnerId,
          signal: mediaOperation.signal,
          onPhase: mediaOperation.setPhase,
          onReport: options.setMediaAssetImportReport,
          commitCommand: "commit_prepared_media_assets_authoritative",
        });
        const report = staged.committed ?? staged.report;
        sideEffectAuthority = staged.terminalAuthority;
        if (!staged.applicationCurrent || !options.isProjectAuthorityCurrent(staged.terminalAuthority)) {
          return;
        }
        options.setMediaAssetImportReport(report);
        // Normal library import never creates layers: one selected file becomes
        // one catalog entry (or a dedupe reuse), preserving the operator's mix.
        options.setMessage(mediaAssetImportReportMessage(report));
        await options.refreshSnapshot();
      } finally {
        mediaOperation.release();
      }
    } catch (error) { setMessageIfAuthorityCurrent(sideEffectAuthority, String(error)); }
  };
  const launchVideoClip = async (layerId: number, fadeMs: number) => {
    try {
      await options.invoke("launch_video_clip", { layerId, fadeMs: Math.max(0, Math.round(fadeMs)) });
      options.setMessage(`Launched video clip ${layerId}${fadeMs > 0 ? ` with ${Math.round(fadeMs)}ms fade` : ""}.`);
      await options.refreshSnapshot();
      return true;
    } catch (error) { options.setMessage(String(error)); return false; }
  };
  const takeVideoClip = async (layerId: number, fadeMs: number) => {
    try {
      const duration = Math.max(0, Math.round(fadeMs));
      await options.invoke("take_video_clip", { layerId, fadeMs: duration });
      options.setMessage(`${duration > 0 ? "Took" : "Cut to"} video clip ${layerId}${duration > 0 ? ` over ${duration}ms` : ""}.`);
      await options.refreshSnapshot();
      return true;
    } catch (error) { options.setMessage(String(error)); return false; }
  };
  const stopVideoClip = async (layerId: number, fadeMs: number) => {
    try {
      await options.invoke("stop_video_clip", { layerId, fadeMs: Math.max(0, Math.round(fadeMs)) });
      options.setMessage(`Stopped video clip ${layerId}${fadeMs > 0 ? ` with ${Math.round(fadeMs)}ms fade` : ""}.`);
      await options.refreshSnapshot();
      return true;
    } catch (error) { options.setMessage(String(error)); return false; }
  };
  const playVideoLayerAudioMonitor = async (layerId: number, volume: number, deviceName?: string) => {
    try {
      options.setVideoAudioMonitorStatus(await options.invoke<VideoAudioMonitorStatus>(
        "play_video_layer_audio_monitor",
        { layerId, volume: Math.max(0, Math.min(2, volume)), deviceName: deviceName?.trim() || null },
      ));
      options.setMessage(`Monitoring embedded audio for video layer ${layerId}.`);
    } catch (error) { options.setMessage(String(error)); }
  };
  const refreshAudioOutputDevices = async () => {
    try {
      const devices = await options.invoke<string[]>("list_audio_output_devices");
      options.setAudioOutputDevices(devices);
      options.setMessage(`Found ${devices.length} audio output device(s).`);
    } catch (error) { options.setMessage(String(error)); }
  };
  const startVideoOutputRecording = async (outputId: number, includeAudio = false) => {
    try {
      const status = await options.invoke<VideoRecordingStatus | null>("start_video_output_recording", {
        outputId,
        frameRate: 30,
        includeAudio,
      });
      if (status) {
        options.setVideoRecordingStatus(status);
        options.setMessage(`Recording output ${outputId} to ${status.path ?? "MP4"}${status.audio_included ? ` with ${status.audio_track_count} audio track(s)` : " without audio"}.`);
      }
    } catch (error) { options.setMessage(String(error)); }
  };
  const stopVideoOutputRecording = async () => {
    try {
      const status = await options.invoke<VideoRecordingStatus>("stop_video_output_recording");
      options.setVideoRecordingStatus(status);
      options.setMessage(status.last_error
        ? `Recording failed: ${status.last_error}`
        : `Recording stopped after ${status.frames_written} frame(s).`);
    } catch (error) { options.setMessage(String(error)); }
  };
  const refreshVideoRecordingStatus = async () => {
    try {
      options.setVideoRecordingStatus(await options.invoke<VideoRecordingStatus>("video_output_recording_status"));
    } catch { /* Background status polling is best-effort. */ }
  };
  const stopVideoLayerAudioMonitor = async (layerId: number) => {
    try {
      options.setVideoAudioMonitorStatus(await options.invoke<VideoAudioMonitorStatus>(
        "stop_video_layer_audio_monitor",
        { layerId },
      ));
      options.setMessage(`Stopped audio monitor for video layer ${layerId}.`);
    } catch (error) { options.setMessage(String(error)); }
  };
  const setVideoLayerAudioMonitorVolume = async (layerId: number, volume: number) => {
    try {
      options.setVideoAudioMonitorStatus(await options.invoke<VideoAudioMonitorStatus>(
        "set_video_layer_audio_monitor_volume",
        { layerId, volume: Math.max(0, Math.min(2, volume)) },
      ));
      return true;
    } catch { return false; }
  };
  const refreshVideoAudioMonitorStatus = async (silent = false) => {
    try {
      const status = await options.invoke<VideoAudioMonitorStatus>("video_audio_monitor_status");
      options.setVideoAudioMonitorStatus(status);
      if (!silent) options.setMessage(`Audio monitor: ${status.active_layer_ids.length} active layer(s).`);
    } catch (error) { if (!silent) options.setMessage(String(error)); }
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
    let sideEffectAuthority = options.getCurrentProjectAuthority();
    try {
      const layer = options.snapshot().video.layers.find((candidate) => candidate.id === layerId);
      const assetId = layer?.media_asset_id;
      const path = layer?.source.path?.trim();
      if (assetId === null || assetId === undefined || !path) {
        options.setMessage("This layer has no relinkable Media Library source.");
        return;
      }
      const initiatingAuthority = options.getCurrentProjectAuthority();
      sideEffectAuthority = initiatingAuthority;
      const preparedStart = await preflightAndBeginMediaAssetOperation(
        initiatingAuthority,
        options.prepareMediaAssetOperationStart,
        () => options.beginMediaAssetOperation("Refresh media metadata", "preparing"),
      );
      const operationAuthority = preparedStart.authority;
      sideEffectAuthority = operationAuthority;
      const mediaOperation = preparedStart.operation;
      try {
        if (!options.isProjectAuthorityCurrent(operationAuthority)) return;
        const staged = await prepareFinalizeAndCommitMediaAssetRelink({
          invoke: options.invoke,
          assetId,
          replacementPath: path,
          policy: "RequireContentMatch",
          expectedEpoch: operationAuthority.project_epoch,
          expectedAuthority: operationAuthority,
          ownerId: options.projectTransactionOwnerId,
          signal: mediaOperation.signal,
          onPhase: mediaOperation.setPhase,
        });
        sideEffectAuthority = staged.terminalAuthority;
        if (!staged.applicationCurrent || !options.isProjectAuthorityCurrent(staged.terminalAuthority)) {
          return;
        }
        if (!staged.outcome) {
          options.setMessage("Media metadata refresh did not return a result.");
          return;
        }
        await options.refreshSnapshot();
        // Refresh itself awaits. A later project C can therefore replace the
        // asset ID while B's refresh is still returning; do not report B.
        if (!options.isProjectAuthorityCurrent(staged.terminalAuthority)) return;
        options.setMessage(mediaAssetRelinkOutcomeMessage(staged.outcome));
      } finally {
        mediaOperation.release();
      }
    } catch (error) { setMessageIfAuthorityCurrent(sideEffectAuthority, String(error)); }
  };
  const runMediaAssetRelink = async (
    assetId: MediaAssetId,
    replacementPath: string,
    policy: "RequireContentMatch" | "AdoptReplacement",
    label: string,
    decisionAuthority?: ProjectAuthorityToken,
    onOperationAuthority?: (authority: ProjectAuthorityToken) => void,
  ) => {
    if (decisionAuthority && !options.isProjectAuthorityCurrent(decisionAuthority)) {
      throw new Error("Project changed after replacement approval was requested; choose the replacement again.");
    }
    const expectedAuthority = decisionAuthority ?? options.getCurrentProjectAuthority();
    if (!options.isProjectAuthorityCurrent(expectedAuthority)) {
      throw new Error("Project changed before replacement approval completed. Choose the replacement again.");
    }
    const preparedStart = await preflightAndBeginMediaAssetOperation(
      expectedAuthority,
      options.prepareMediaAssetOperationStart,
      () => options.beginMediaAssetOperation(label, "preparing"),
    );
    const operationAuthority = preparedStart.authority;
    onOperationAuthority?.(operationAuthority);
    const mediaOperation = preparedStart.operation;
    try {
      // A RequireContentMatch decision is a one-shot decision for exact E/R/H.
      // Even a successful local mapping ACK during the confirmation window is
      // a different project image, so require a fresh confirmation rather than
      // silently adopting into that newer image.
      if (decisionAuthority && !sameProjectAuthority(decisionAuthority, operationAuthority)) {
        throw new Error("Project changed while replacement approval was being confirmed; choose the replacement again.");
      }
      if (!options.isProjectAuthorityCurrent(operationAuthority)) {
        throw new Error("Project changed while replacement approval was being confirmed; nothing was applied.");
      }
      return await prepareFinalizeAndCommitMediaAssetRelink({
        invoke: options.invoke,
        assetId,
        replacementPath,
        policy,
        expectedEpoch: operationAuthority.project_epoch,
        expectedAuthority: operationAuthority,
        ownerId: options.projectTransactionOwnerId,
        signal: mediaOperation.signal,
        onPhase: mediaOperation.setPhase,
      });
    } finally {
      mediaOperation.release();
    }
  };
  const relinkMediaAsset = async (
    assetId: MediaAssetId,
    setRelinkMessage: (message: string) => unknown = options.setMessage,
  ): Promise<MediaAssetRelinkUiResult | null> => {
    const setRelinkMessageIfAuthorityCurrent = (authority: ProjectAuthorityToken, message: string) => {
      if (options.isProjectAuthorityCurrent(authority)) setRelinkMessage(message);
    };
    const initiatingAuthority = options.getCurrentProjectAuthority();
    let sideEffectAuthority = initiatingAuthority;
    try {
      const asset = options.snapshot().video.media_assets.find((candidate) => candidate.id === assetId);
      if (!asset || (asset.source.kind !== "File" && asset.source.kind !== "StillImage")) {
        setRelinkMessageIfAuthorityCurrent(initiatingAuthority, `Media asset ${assetId} is not a relinkable local file.`);
        return null;
      }

      const preparedStart = await preflightAndBeginMediaAssetOperation(
        initiatingAuthority,
        options.prepareMediaAssetOperationStart,
        () => options.beginMediaAssetOperation(`Relink ${asset.label}`, "picker"),
      );
      const operationAuthority = preparedStart.authority;
      sideEffectAuthority = operationAuthority;
      const pickerOperation = preparedStart.operation;
      let replacementPath: string | null = null;
      let firstOutcome: MediaAssetRelinkOutcome | null = null;
      let decisionAuthority: ProjectAuthorityToken | null = null;
      try {
        replacementPath = await options.invoke<string | null>("select_video_source_file", {
          kind: asset.source.kind,
        });
        if (!options.isProjectAuthorityCurrent(operationAuthority)) return null;
        if (!replacementPath) {
          setRelinkMessage("Media relink canceled. No project changes were made.");
          return null;
        }
        const first = await prepareFinalizeAndCommitMediaAssetRelink({
          invoke: options.invoke,
          assetId,
          replacementPath,
          policy: "RequireContentMatch",
          expectedEpoch: operationAuthority.project_epoch,
          expectedAuthority: operationAuthority,
          ownerId: options.projectTransactionOwnerId,
          signal: pickerOperation.signal,
          onPhase: pickerOperation.setPhase,
        });
        if (!first.applicationCurrent || !options.isProjectAuthorityCurrent(first.terminalAuthority)) return null;
        sideEffectAuthority = first.terminalAuthority;
        firstOutcome = first.outcome;
        decisionAuthority = first.terminalAuthority;
        if (first.committed) {
          await options.refreshSnapshot();
          // The refresh can cross an authority replacement.  Its asset IDs may
          // be reused by C, so do not return a B outcome for App to clear.
          if (!options.isProjectAuthorityCurrent(first.terminalAuthority)) return null;
          if (firstOutcome) setRelinkMessage(mediaAssetRelinkOutcomeMessage(firstOutcome));
          return firstOutcome
            ? { outcome: firstOutcome, terminalAuthority: first.terminalAuthority }
            : null;
        }
      } finally {
        pickerOperation.release();
      }

      // Adoption is never inferred from a generic mismatch. The backend must
      // first return the dedicated decision, then the operator must approve it.
      if (!replacementPath || !decisionAuthority || firstOutcome?.kind !== "needs_explicit_adoption") {
        if (!options.isProjectAuthorityCurrent(sideEffectAuthority)) return null;
        if (firstOutcome) setRelinkMessage(mediaAssetRelinkOutcomeMessage(firstOutcome));
        return firstOutcome ? { outcome: firstOutcome, terminalAuthority: sideEffectAuthority } : null;
      }
      if (!options.confirmMediaAssetAdoption(assetId, replacementPath)) {
        setRelinkMessageIfAuthorityCurrent(decisionAuthority, "Replacement adoption canceled. The existing content identity was kept.");
        return options.isProjectAuthorityCurrent(decisionAuthority)
          ? { outcome: firstOutcome, terminalAuthority: decisionAuthority }
          : null;
      }
      if (!options.isProjectAuthorityCurrent(decisionAuthority)) {
        setRelinkMessage("Project changed before replacement approval completed. Choose the replacement again.");
        return null;
      }

      const adopted = await runMediaAssetRelink(
        assetId,
        replacementPath,
        "AdoptReplacement",
        `Adopt replacement for ${asset.label}`,
        decisionAuthority,
        (authority) => { sideEffectAuthority = authority; },
      );
      if (!adopted.applicationCurrent || !options.isProjectAuthorityCurrent(adopted.terminalAuthority)) return null;
      sideEffectAuthority = adopted.terminalAuthority;
      if (adopted.committed) await options.refreshSnapshot();
      if (!options.isProjectAuthorityCurrent(adopted.terminalAuthority)) return null;
      if (adopted.outcome) setRelinkMessage(mediaAssetRelinkOutcomeMessage(adopted.outcome));
      return adopted.outcome
        ? { outcome: adopted.outcome, terminalAuthority: adopted.terminalAuthority }
        : null;
    } catch (error) {
      setRelinkMessageIfAuthorityCurrent(sideEffectAuthority, String(error));
      return null;
    }
  };
  const setVideoLayerIsfEffect = async (layerId: number, effect: VideoIsfEffectSummary | null) => {
    if (!canMutateVideoIsf()) return;
    try {
      await options.invoke("set_video_layer_isf_effect", { layerId, effect });
      options.setMessage(effect ? `${effect.enabled ? "Applied" : "Bypassed"} ISF ${effect.label} on layer ${layerId}.` : `Cleared ISF on layer ${layerId}.`);
      await options.refreshSnapshot();
      await refreshVideoPreviewDiagnostics(false);
    } catch (error) {
      options.setMessage(`ISF update failed: ${String(error)}`);
    }
  };
  const importVideoLayerIsf = async (layerId: number) => {
    if (!canMutateVideoIsf()) return;
    try {
      const effect = await options.invoke<VideoIsfEffectSummary | null>("select_video_isf_file");
      if (!effect) {
        options.setMessage("ISF import canceled.");
        return;
      }
      await options.invoke("add_video_layer_isf_effect", { layerId, effect });
      options.setMessage(`Added imported FX ${effect.label} to layer ${layerId}.`);
      await options.refreshSnapshot();
      await refreshVideoPreviewDiagnostics(false);
    } catch (error) {
      options.setMessage(`ISF import failed: ${String(error)}`);
    }
  };
  const applyBuiltinVideoIsfEffect = async (layerId: number, presetId: string) => {
    if (!canMutateVideoIsf()) return;
    try {
      const effect = await options.invoke<VideoIsfEffectSummary>("add_builtin_video_isf_effect", {
        layerId,
        presetId,
      });
      options.setMessage(`Added built-in FX ${effect.label} to layer ${layerId}.`);
      await options.refreshSnapshot();
      await refreshVideoPreviewDiagnostics(false);
    } catch (error) {
      options.setMessage(`Built-in FX failed: ${String(error)}`);
    }
  };
  const moveVideoLayerIsfEffect = async (layerId: number, stageIndex: number, delta: -1 | 1) => {
    if (!canMutateVideoIsf()) return;
    try {
      await options.invoke("move_video_layer_isf_effect", { layerId, stageIndex, delta });
      options.setMessage(`Moved FX ${stageIndex + 1} on layer ${layerId}.`);
      await options.refreshSnapshot();
      await refreshVideoPreviewDiagnostics(true);
    } catch (error) { options.setMessage(`FX move failed: ${String(error)}`); }
  };
  const removeVideoLayerIsfEffect = async (layerId: number, stageIndex: number) => {
    if (!canMutateVideoIsf()) return;
    try {
      await options.invoke("remove_video_layer_isf_effect", { layerId, stageIndex });
      options.setMessage(`Removed FX ${stageIndex + 1} from layer ${layerId}.`);
      await options.refreshSnapshot();
      await refreshVideoPreviewDiagnostics(false);
    } catch (error) { options.setMessage(`FX removal failed: ${String(error)}`); }
  };
  const setVideoLayerIsfEffectEnabled = async (layerId: number, stageIndex: number, enabled: boolean) => {
    if (!canMutateVideoIsf()) return;
    try {
      await options.invoke("set_video_layer_isf_effect_enabled", { layerId, stageIndex, enabled });
      options.setMessage(`${enabled ? "Enabled" : "Bypassed"} FX ${stageIndex + 1} on layer ${layerId}.`);
      await options.refreshSnapshot();
      await refreshVideoPreviewDiagnostics(true);
    } catch (error) { options.setMessage(`FX bypass failed: ${String(error)}`); }
  };
  const resetVideoLayerIsfEffect = async (layerId: number, stageIndex: number) => {
    if (!canMutateVideoIsf()) return;
    try {
      await options.invoke("reset_video_layer_isf_effect", { layerId, stageIndex });
      options.setMessage(`Reset FX ${stageIndex + 1} on layer ${layerId}.`);
      await options.refreshSnapshot();
      await refreshVideoPreviewDiagnostics(true);
    } catch (error) { options.setMessage(`FX reset failed: ${String(error)}`); }
  };
  const setVideoLayerIsfControl = async (
    layerId: number,
    stageIndex: number,
    controlName: string,
    value: [number, number, number, number],
  ) => {
    if (!canMutateVideoIsf()) return;
    try {
      await options.invoke("set_video_layer_isf_control", { layerId, stageIndex, controlName, value });
      await options.refreshSnapshot();
      await refreshVideoPreviewDiagnostics(true);
    } catch (error) { options.setMessage(`FX control failed: ${String(error)}`); }
  };
  const triggerVideoLayerIsfEvent = async (
    layerId: number,
    stageIndex: number,
    controlName: string,
  ) => {
    if (options.isIsfEventPulseBusy()) {
      options.setMessage("Another FX Event pulse is still active.");
      return;
    }
    options.setIsfEventPulseBusy(true);
    try {
      await options.invoke("pulse_video_layer_isf_event", { layerId, stageIndex, controlName });
      await refreshVideoPreviewDiagnostics(true);
    } catch (error) {
      options.setMessage(`FX event failed: ${String(error)}`);
    } finally {
      options.setIsfEventPulseBusy(false);
    }
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
  const loadVideoLayerThumbnail = async (layerId: number) => {
    const frame = await options.invoke<VideoFrame>("get_video_layer_thumbnail", {
      layerId,
      width: 160,
      height: 90,
    });
    return videoFrameToDataUrl(frame);
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
    addVideoLayer, importMediaFiles, launchVideoClip, takeVideoClip, stopVideoClip,
    playVideoLayerAudioMonitor, stopVideoLayerAudioMonitor, setVideoLayerAudioMonitorVolume,
    refreshVideoAudioMonitorStatus,
    refreshAudioOutputDevices,
    startVideoOutputRecording, stopVideoOutputRecording, refreshVideoRecordingStatus,
    removeVideoLayer, duplicateVideoLayer, moveVideoLayer, setVideoLayerLabel,
    refreshVideoLayerMetadata, relinkMediaAsset, importVideoLayerIsf, applyBuiltinVideoIsfEffect, setVideoLayerIsfEffect,
    moveVideoLayerIsfEffect, removeVideoLayerIsfEffect, setVideoLayerIsfEffectEnabled,
    resetVideoLayerIsfEffect, setVideoLayerIsfControl, triggerVideoLayerIsfEvent,
    renderDebugVideoPreview, loadVideoLayerThumbnail, refreshVideoPreviewDiagnostics,
    refreshVideoOutputRenderPlans, refreshVideoOutputWindowStatuses, refreshSnapshotAndVideoOutputRenderPlans,
    syncOpenVideoOutputWindows, closeOpenVideoOutputWindows, openAllVideoOutputWindows,
    refreshVideoRuntimeStatus, refreshExternalVideoIoPlans, syncExternalVideoTransports,
    renderDebugVideoOutputPreview, setVideoLayerState, setVideoLayerTransform, setVideoLayerColor,
    setVideoLayerFx, addVideoCuePoint, removeVideoCuePoint, jumpVideoCuePoint,
    setVideoLayerBlendMode, setVideoMasterOpacity, setVideoBlackout,
  };
}

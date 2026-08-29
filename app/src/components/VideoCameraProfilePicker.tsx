import { For, Show, createEffect, createMemo, createSignal } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import {
  cameraDeviceOptions,
  cameraProbeAllowsAdd,
  cameraProbeDisplayLabel,
  cameraProfileDisplayLabel,
  idleCameraProbeState,
  invalidatedCameraProbeState,
  parseVideoCameraProbe,
  parseVideoCameraProfiles,
  type VideoCameraProbeResult,
  type VideoCameraProbeState,
  type VideoCameraProfileDescriptor,
} from "../videoCameraProfiles";

export interface VideoCameraProfilePickerProps {
  path: string;
  onSetPath: (path: string) => void;
  invokeCommand?: FrontendTauriInvoke;
  /** Reports the exact current-endpoint probe admission to the source shell. */
  onAdmissionChange?: (allowed: boolean) => void;
}

const cameraErrorMessage = (error: unknown) => error instanceof Error ? error.message : String(error);

const cameraRatePresentationNote = (numerator: number, denominator: number) =>
  numerator > denominator * 60
    ? "Capture rate is above 60fps; output presentation is capped at 60fps."
    : null;

export function VideoCameraProfilePicker(props: VideoCameraProfilePickerProps) {
  const [cameraProfiles, setCameraProfiles] = createSignal<VideoCameraProfileDescriptor[]>([]);
  const [selectedCameraDeviceIdentity, setSelectedCameraDeviceIdentity] = createSignal("");
  const [selectedCameraProfileIdentity, setSelectedCameraProfileIdentity] = createSignal("");
  const [cameraCatalogLoaded, setCameraCatalogLoaded] = createSignal(false);
  const [cameraCatalogBusy, setCameraCatalogBusy] = createSignal(false);
  const [cameraCatalogError, setCameraCatalogError] = createSignal<string | null>(null);
  const [cameraProbe, setCameraProbe] = createSignal<VideoCameraProbeState>(idleCameraProbeState());
  let catalogGeneration = 0;
  let probeGeneration = 0;

  const cameraDevices = createMemo(() => cameraDeviceOptions(cameraProfiles()));
  const selectedCameraProfiles = createMemo(() => {
    const deviceIdentity = selectedCameraDeviceIdentity();
    if (!deviceIdentity) return [];
    return cameraProfiles().filter((profile) => profile.device_identity === deviceIdentity);
  });
  const selectedCameraProfile = createMemo(() => {
    const profileIdentity = selectedCameraProfileIdentity();
    return selectedCameraProfiles().find((profile) => profile.profile_identity === profileIdentity);
  });
  const cameraProbeIsCurrent = createMemo(() => cameraProbeAllowsAdd(
    "Camera",
    props.path,
    selectedCameraProfile(),
    cameraProbe(),
  ));

  createEffect(() => {
    props.onAdmissionChange?.(cameraProbeIsCurrent());
  });

  const invalidateCameraProbe = (
    message: string,
    selection?: VideoCameraProfileDescriptor,
    forceStale = true,
  ) => {
    probeGeneration += 1;
    if (forceStale || selection || cameraProbe().status !== "idle") {
      setCameraProbe(invalidatedCameraProbeState(message, {
        endpoint_name: selection?.endpoint_name ?? null,
        profile_identity: selection?.profile_identity ?? null,
      }));
    } else {
      setCameraProbe(idleCameraProbeState());
    }
  };

  const clearCameraSelection = (message: string, forceStale = true) => {
    setSelectedCameraDeviceIdentity("");
    setSelectedCameraProfileIdentity("");
    invalidateCameraProbe(message, undefined, forceStale);
    props.onSetPath("");
  };

  const refreshCameraCatalog = async () => {
    const generation = ++catalogGeneration;
    setCameraCatalogBusy(true);
    setCameraCatalogLoaded(false);
    setCameraCatalogError(null);
    setCameraProfiles([]);
    clearCameraSelection("Camera catalog refresh invalidated the previous probe; select and test a profile.");
    const invokeCommand = props.invokeCommand;
    if (!invokeCommand) {
      if (generation === catalogGeneration) {
        setCameraCatalogBusy(false);
        setCameraCatalogError("Camera profile discovery is unavailable until the native IPC bridge is connected.");
      }
      return;
    }
    try {
      const value = await invokeCommand<unknown>("list_video_camera_profiles");
      if (generation !== catalogGeneration) return;
      const profiles = parseVideoCameraProfiles(value);
      setCameraProfiles(profiles);
      setCameraCatalogLoaded(true);
    } catch (error) {
      if (generation === catalogGeneration) setCameraCatalogError(cameraErrorMessage(error));
    } finally {
      if (generation === catalogGeneration) setCameraCatalogBusy(false);
    }
  };

  const selectCameraDevice = (deviceIdentity: string) => {
    const device = cameraDevices().find((candidate) => candidate.device_identity === deviceIdentity);
    if (!device) return;
    setSelectedCameraDeviceIdentity(device.device_identity);
    setSelectedCameraProfileIdentity("");
    invalidateCameraProbe("Camera device changed; test the selected profile before adding a layer.");
    props.onSetPath("");
  };

  const selectCameraProfile = (profileIdentity: string) => {
    const profile = selectedCameraProfiles().find((candidate) => candidate.profile_identity === profileIdentity);
    if (!profile) return;
    setSelectedCameraProfileIdentity(profile.profile_identity);
    invalidateCameraProbe("Camera profile changed; test the selected profile before adding a layer.", profile);
    // The endpoint is an opaque native identity returned by the catalog. Do
    // not derive, normalize, or replace it with the display name.
    props.onSetPath(profile.endpoint_name);
  };

  const testSelectedCameraProfile = async () => {
    const profile = selectedCameraProfile();
    if (!profile) {
      setCameraProbe(invalidatedCameraProbeState("Select a camera device and profile before testing."));
      return;
    }
    const invokeCommand = props.invokeCommand;
    const endpointName = profile.endpoint_name;
    const generation = ++probeGeneration;
    setCameraProbe({
      status: "loading",
      endpoint_name: endpointName,
      profile_identity: profile.profile_identity,
      result: null,
      message: "Testing selected camera profile…",
    });
    if (!invokeCommand) {
      setCameraProbe({
        status: "error",
        endpoint_name: endpointName,
        profile_identity: profile.profile_identity,
        result: null,
        message: "Camera profile testing is unavailable until the native IPC bridge is connected.",
      });
      return;
    }
    try {
      const value = await invokeCommand<unknown>("probe_video_camera_profile", { endpointName });
      // Selection changes increment probeGeneration, so an older response can
      // never re-arm Add Video Layer for a newer endpoint.
      if (generation !== probeGeneration
        || props.path !== endpointName
        || selectedCameraProfileIdentity() !== profile.profile_identity) return;
      const result = parseVideoCameraProbe(value, endpointName);
      setCameraProbe({
        status: "success",
        endpoint_name: endpointName,
        profile_identity: profile.profile_identity,
        result,
        message: "Camera profile probe passed for the current endpoint.",
      });
    } catch (error) {
      if (generation !== probeGeneration) return;
      setCameraProbe({
        status: "error",
        endpoint_name: endpointName,
        profile_identity: profile.profile_identity,
        result: null,
        message: cameraErrorMessage(error),
      });
    }
  };

  const selectedProbeResult = (): VideoCameraProbeResult | null => cameraProbe().result;

  return (
    <section class="videoSourceCameraPanel" data-video-camera-source>
      <div class="videoSourceCameraActions">
        <button
          type="button"
          data-video-camera-refresh
          onClick={() => { void refreshCameraCatalog(); }}
          disabled={cameraCatalogBusy()}
        >
          {cameraCatalogBusy() ? "Refreshing cameras…" : "Refresh cameras"}
        </button>
        <Show when={cameraCatalogLoaded() && !cameraCatalogError()}>
          <span class="fieldHint" data-camera-catalog-count>{cameraProfiles().length} profile(s) enumerated</span>
        </Show>
      </div>

      <label>
        Camera device
        <select
          data-video-camera-device
          value={selectedCameraDeviceIdentity()}
          disabled={cameraCatalogBusy() || cameraDevices().length === 0}
          onChange={(event) => selectCameraDevice(event.currentTarget.value)}
        >
          <option value="" disabled>Select a camera device</option>
          <For each={cameraDevices()}>
            {(device) => <option value={device.device_identity}>{device.device_name} ({device.profile_count} profile(s))</option>}
          </For>
        </select>
      </label>

      <label>
        Camera profile
        <select
          data-video-camera-profile
          value={selectedCameraProfileIdentity()}
          disabled={!selectedCameraDeviceIdentity() || selectedCameraProfiles().length === 0 || cameraCatalogBusy()}
          onChange={(event) => selectCameraProfile(event.currentTarget.value)}
        >
          <option value="" disabled>Select a camera profile</option>
          <For each={selectedCameraProfiles()}>
            {(profile) => <option value={profile.profile_identity}>{cameraProfileDisplayLabel(profile)}</option>}
          </For>
        </select>
      </label>

      <Show when={selectedCameraProfile()}>
        {(profile) => (
          <div class="videoSourceDraftStatus" data-camera-selected-profile>
            <strong>Selected capture</strong>
            <small>{cameraProfileDisplayLabel(profile())}</small>
            <Show when={cameraRatePresentationNote(profile().frame_rate_numerator, profile().frame_rate_denominator)}>
              {(note) => <small data-camera-presentation-cap>{note()}</small>}
            </Show>
          </div>
        )}
      </Show>

      <Show when={cameraCatalogBusy()}>
        <p class="fieldHint" data-camera-status="loading" role="status">Loading camera profiles…</p>
      </Show>
      <Show when={!cameraCatalogBusy() && cameraCatalogError()}>
        {(error) => <p class="fieldHint" data-camera-status="error" role="alert">Camera profile refresh failed: {error()}</p>}
      </Show>
      <Show when={!cameraCatalogBusy() && !cameraCatalogError() && cameraCatalogLoaded() && cameraProfiles().length === 0}>
        <p class="fieldHint" data-camera-status="empty" role="status">No camera profiles were enumerated. Refresh cameras to try again.</p>
      </Show>
      <Show when={!cameraCatalogBusy() && !cameraCatalogError() && !cameraCatalogLoaded()}>
        <p class="fieldHint" data-camera-status="idle" role="status">Camera profiles are not loaded. Choose Refresh cameras.</p>
      </Show>

      <Show when={cameraProbe().status === "loading"}>
        <p class="fieldHint" data-camera-probe-status="loading" role="status">{cameraProbe().message}</p>
      </Show>
      <Show when={cameraProbe().status === "stale"}>
        <p class="fieldHint" data-camera-probe-status="stale" role="status">{cameraProbe().message}</p>
      </Show>
      <Show when={cameraProbe().status === "error"}>
        <p class="fieldHint" data-camera-probe-status="error" role="alert">Camera profile test failed: {cameraProbe().message}</p>
      </Show>
      <Show when={cameraProbe().status === "success" && selectedProbeResult()}>
        {(result) => (
          <div class="fieldHint" data-camera-probe-status="success" role="status">
            <strong>Profile test passed</strong>: {cameraProbeDisplayLabel(result())}
            <Show when={cameraRatePresentationNote(result().frame_rate_numerator, result().frame_rate_denominator)}>
              {(note) => <span data-camera-presentation-cap> {note()}</span>}
            </Show>
          </div>
        )}
      </Show>
      <Show when={selectedCameraProfile() && cameraProbe().status === "idle"}>
        <p class="fieldHint" data-camera-probe-status="untested" role="status">Test selected profile before adding a camera layer.</p>
      </Show>

      <button
        type="button"
        data-video-camera-test
        onClick={() => { void testSelectedCameraProfile(); }}
        disabled={!selectedCameraProfile() || cameraCatalogBusy() || cameraProbe().status === "loading"}
      >
        {cameraProbe().status === "loading" ? "Testing selected profile…" : "Test selected profile"}
      </button>
    </section>
  );
}

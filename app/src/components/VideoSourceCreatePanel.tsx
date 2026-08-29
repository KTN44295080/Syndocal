import { Show, createSignal } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import type { VideoSourceKind } from "../types";
import { videoSourceCanBrowseFile, videoSourceInputLabel, videoSourceInputPlaceholder } from "../videoHelpers";
import { VideoCameraProfilePicker } from "./VideoCameraProfilePicker";

interface VideoPreviewImagePanelProps {
  previewUrl: string | null;
  layerCount: number;
}
interface VideoSourceCreatePanelProps {
  sourceKind: VideoSourceKind;
  label: string;
  path: string;
  onSetSourceKind: (kind: VideoSourceKind) => void;
  onSetLabel: (label: string) => void;
  onSetPath: (path: string) => void;
  onBrowseSource: () => void | Promise<void>;
  onImportMultiple: () => void | Promise<void>;
  onAddLayer: () => void | Promise<void>;
  /** The mounted App's typed bridge. Camera discovery is unavailable without it. */
  invokeCommand?: FrontendTauriInvoke;
}

export function VideoPreviewImagePanel(props: VideoPreviewImagePanelProps) {
  return (
    <div class="videoPreview">
      <Show when={props.previewUrl} fallback={<span>{props.layerCount === 0 ? "Add a video layer" : "Preview (Reference)"}</span>}>
        {(url) => <img src={url()} alt="Reference video preview" />}
      </Show>
    </div>
  );
}

export function VideoSourceCreatePanel(props: VideoSourceCreatePanelProps) {
  const [cameraAddAllowed, setCameraAddAllowed] = createSignal(false);

  const selectSourceKind = (kind: VideoSourceKind) => {
    setCameraAddAllowed(false);
    if (kind === "Camera") props.onSetPath("");
    props.onSetSourceKind(kind);
  };

  const handleAddLayer = async () => {
    if (props.sourceKind === "Camera" && !cameraAddAllowed()) return;
    await props.onAddLayer();
  };

  return (
    <>
      <label>
        Source
        <select value={props.sourceKind} onInput={(event) => selectSourceKind(event.currentTarget.value as VideoSourceKind)}>
          <option value="File">Video file</option>
          <option value="StillImage">Still image</option>
          <option value="Camera">Camera input</option>
          <option value="ScreenCapture">Screen capture</option>
          <option value="Ndi">NDI input</option>
          <option value="Spout">Spout input</option>
          <option value="Syphon">Syphon input</option>
        </select>
      </label>
      <label>
        Video Layer label
        <input value={props.label} onInput={(event) => props.onSetLabel(event.currentTarget.value)} />
      </label>

      <Show when={props.sourceKind === "Camera"} fallback={
        <>
          <label>
            {videoSourceInputLabel(props.sourceKind)}
            <input
              value={props.path}
              placeholder={videoSourceInputPlaceholder(props.sourceKind)}
              onInput={(event) => props.onSetPath(event.currentTarget.value)}
            />
          </label>
          <Show when={props.sourceKind === "ScreenCapture"}>
            <p class="fieldHint" data-screen-capture-contract>
              Screen capture uses a separate truthful 1280x720 / 30fps path. Leave Display source blank for the primary desktop.
            </p>
          </Show>
        </>
      }>
        <VideoCameraProfilePicker
          path={props.path}
          onSetPath={props.onSetPath}
          invokeCommand={props.invokeCommand}
          onAdmissionChange={setCameraAddAllowed}
        />
      </Show>

      <button onClick={() => void props.onBrowseSource()} disabled={!videoSourceCanBrowseFile(props.sourceKind)}>
        Browse Source
      </button>
      <button onClick={() => void props.onImportMultiple()} disabled={!videoSourceCanBrowseFile(props.sourceKind)}>
        Import Multiple
      </button>
      <button
        class="primary"
        data-video-add-layer
        onClick={() => { void handleAddLayer(); }}
        disabled={props.sourceKind === "Camera" && !cameraAddAllowed()}
      >
        Add Video Layer
      </button>
    </>
  );
}

import { Show } from "solid-js";
import type { VideoSourceKind } from "../types";
import { videoSourceCanBrowseFile, videoSourceInputLabel, videoSourceInputPlaceholder } from "../videoHelpers";

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
  onAddLayer: () => void | Promise<void>;
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
  return (
    <>
      <label>
        Source
        <select value={props.sourceKind} onInput={(event) => props.onSetSourceKind(event.currentTarget.value as VideoSourceKind)}>
          <option value="File">Video file</option>
          <option value="StillImage">Still image</option>
          <option value="Ndi">NDI input</option>
          <option value="Spout">Spout input</option>
          <option value="Syphon">Syphon input</option>
        </select>
      </label>
      <label>
        Video Layer label
        <input value={props.label} onInput={(event) => props.onSetLabel(event.currentTarget.value)} />
      </label>
      <label>
        {videoSourceInputLabel(props.sourceKind)}
        <input
          value={props.path}
          placeholder={videoSourceInputPlaceholder(props.sourceKind)}
          onInput={(event) => props.onSetPath(event.currentTarget.value)}
        />
      </label>
      <button onClick={() => void props.onBrowseSource()} disabled={!videoSourceCanBrowseFile(props.sourceKind)}>
        Browse Source
      </button>
      <button class="primary" onClick={() => void props.onAddLayer()}>
        Add Video Layer
      </button>
    </>
  );
}

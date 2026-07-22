import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import type {
  ColorMappingCellTarget,
  ColorMappingFrame,
  ColorMappingPlaybackDirection,
  ColorMappingSampling,
  ColorMappingSourceKind,
  ColorMappingWrapMode,
} from "../types";

export interface ColorMappingFixtureOption {
  id: number;
  label: string;
  x: number;
  z: number;
}

export interface ColorMappingEffectEditorPanelProps {
  sourceKind: ColorMappingSourceKind;
  width: number;
  height: number;
  frames: ColorMappingFrame[];
  cells: ColorMappingCellTarget[];
  fixtures: ColorMappingFixtureOption[];
  playbackDirection: ColorMappingPlaybackDirection;
  periodMs: number;
  bpm: number;
  clockSyncBeats: number | null;
  phase: number;
  offsetU: number;
  offsetV: number;
  scaleU: number;
  scaleV: number;
  rotationDegrees: number;
  wrapMode: ColorMappingWrapMode;
  sampling: ColorMappingSampling;
  onRaster: (kind: ColorMappingSourceKind, width: number, height: number, frames: ColorMappingFrame[]) => void;
  onCells: (cells: ColorMappingCellTarget[]) => void;
  onPlaybackDirection: (direction: ColorMappingPlaybackDirection) => void;
  onPeriodMs: (periodMs: number) => void;
  onClockSyncBeats: (beats: number | null) => void;
  onPhase: (phase: number) => void;
  onOffsetU: (offset: number) => void;
  onOffsetV: (offset: number) => void;
  onScaleU: (scale: number) => void;
  onScaleV: (scale: number) => void;
  onRotationDegrees: (degrees: number) => void;
  onWrapMode: (mode: ColorMappingWrapMode) => void;
  onSampling: (sampling: ColorMappingSampling) => void;
}

const MAX_AXIS = 64;
const MAX_FRAMES = 64;
const playbackDirections: ColorMappingPlaybackDirection[] = ["Forward", "Reverse", "Bounce"];
const clockPresets = [null, 0.25, 0.5, 1, 2, 4] as const;
const TWO_POW_32 = 4_294_967_296;

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, Number.isFinite(value) ? value : minimum));

const packRgb16 = (red8: number, green8: number, blue8: number, alpha8 = 255) => {
  const alpha = clamp(alpha8, 0, 255) / 255;
  const red = Math.round(clamp(red8, 0, 255) * alpha) * 257;
  const green = Math.round(clamp(green8, 0, 255) * alpha) * 257;
  const blue = Math.round(clamp(blue8, 0, 255) * alpha) * 257;
  return red * TWO_POW_32 + green * 65_536 + blue;
};

const unpackRgb8 = (pixel: number) => {
  const normalized = Math.max(0, Math.min(281_474_976_710_655, Math.round(pixel)));
  const red = Math.floor(normalized / TWO_POW_32);
  const green = Math.floor(normalized / 65_536) % 65_536;
  const blue = normalized % 65_536;
  return [Math.round(red / 257), Math.round(green / 257), Math.round(blue / 257)] as const;
};

const frameFromCanvas = (canvas: HTMLCanvasElement): ColorMappingFrame => {
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) throw new Error("2D canvas is unavailable.");
  const data = context.getImageData(0, 0, canvas.width, canvas.height).data;
  const pixels = new Array<number>(canvas.width * canvas.height);
  for (let pixel = 0, offset = 0; pixel < pixels.length; pixel += 1, offset += 4) {
    pixels[pixel] = packRgb16(data[offset], data[offset + 1], data[offset + 2], data[offset + 3]);
  }
  return { pixels };
};

const fittedRasterSize = (width: number, height: number) => {
  const safeWidth = Math.max(1, Number.isFinite(width) ? width : 1);
  const safeHeight = Math.max(1, Number.isFinite(height) ? height : 1);
  const scale = Math.min(1, MAX_AXIS / safeWidth, MAX_AXIS / safeHeight);
  return {
    width: Math.max(1, Math.round(safeWidth * scale)),
    height: Math.max(1, Math.round(safeHeight * scale)),
  };
};

const drawSourceToRaster = (
  source: CanvasImageSource,
  sourceWidth: number,
  sourceHeight: number,
) => {
  const size = fittedRasterSize(sourceWidth, sourceHeight);
  const canvas = document.createElement("canvas");
  canvas.width = size.width;
  canvas.height = size.height;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("2D canvas is unavailable.");
  context.clearRect(0, 0, size.width, size.height);
  context.drawImage(source, 0, 0, size.width, size.height);
  return { ...size, frame: frameFromCanvas(canvas) };
};

const waitForEvent = (target: EventTarget, event: string, errorEvent = "error") =>
  new Promise<void>((resolve, reject) => {
    const complete = () => {
      target.removeEventListener(errorEvent, fail);
      resolve();
    };
    const fail = () => {
      target.removeEventListener(event, complete);
      reject(new Error(`Media ${errorEvent} while decoding.`));
    };
    target.addEventListener(event, complete, { once: true });
    target.addEventListener(errorEvent, fail, { once: true });
  });

export const defaultColorMappingRaster = () => {
  const width = 8;
  const height = 8;
  const pixels = Array.from({ length: width * height }, (_, index) => {
    const x = index % width;
    const y = Math.floor(index / width);
    const hue = (x / width + y / height) * Math.PI * 2;
    return packRgb16(
      128 + Math.round(Math.sin(hue) * 127),
      128 + Math.round(Math.sin(hue + (Math.PI * 2) / 3) * 127),
      128 + Math.round(Math.sin(hue + (Math.PI * 4) / 3) * 127),
    );
  });
  return { width, height, frames: [{ pixels }] as ColorMappingFrame[] };
};

export function ColorMappingEffectEditorPanel(props: ColorMappingEffectEditorPanelProps) {
  let previewCanvas: HTMLCanvasElement | undefined;
  const [textSource, setTextSource] = createSignal("SYN");
  const [textColor, setTextColor] = createSignal("#ffffff");
  const [busy, setBusy] = createSignal(false);
  const [status, setStatus] = createSignal("");
  const [previewFrame, setPreviewFrame] = createSignal(0);
  const frameCount = createMemo(() => props.frames.length);
  const phasePercent = createMemo(() => Math.round(clamp(props.phase, 0, 1) * 100));

  createEffect(() => {
    const canvas = previewCanvas;
    if (!canvas) return;
    const width = Math.max(1, Math.round(props.width));
    const height = Math.max(1, Math.round(props.height));
    const frame = props.frames[Math.min(previewFrame(), Math.max(0, props.frames.length - 1))];
    if (!frame || frame.pixels.length !== width * height) return;
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext("2d");
    if (!context) return;
    const image = context.createImageData(width, height);
    for (let index = 0; index < frame.pixels.length; index += 1) {
      const [red, green, blue] = unpackRgb8(frame.pixels[index]);
      const offset = index * 4;
      image.data[offset] = red;
      image.data[offset + 1] = green;
      image.data[offset + 2] = blue;
      image.data[offset + 3] = 255;
    }
    context.putImageData(image, 0, 0);
  });

  const normalizedFixturePoints = createMemo(() => {
    if (props.fixtures.length === 0) return [];
    const xs = props.fixtures.map((fixture) => fixture.x);
    const zs = props.fixtures.map((fixture) => fixture.z);
    const minX = Math.min(...xs);
    const maxX = Math.max(...xs);
    const minZ = Math.min(...zs);
    const maxZ = Math.max(...zs);
    const normalize = (value: number, minimum: number, maximum: number) =>
      Math.abs(maximum - minimum) < 0.000_001 ? 0.5 : clamp((value - minimum) / (maximum - minimum), 0, 1);
    return props.fixtures.map((fixture, index) => ({
      ...fixture,
      selectionIndex: index,
      u: normalize(fixture.x, minX, maxX),
      v: normalize(fixture.z, minZ, maxZ),
    }));
  });
  const previewCells = createMemo(() =>
    props.cells.length > 0
      ? props.cells.map((cell) => ({ id: cell.fixture_id, label: `#${cell.fixture_id}`, u: cell.u, v: cell.v }))
      : normalizedFixturePoints(),
  );

  const importImage = async (file: File) => {
    setBusy(true);
    setStatus("Decoding image…");
    try {
      const bitmap = await createImageBitmap(file);
      const raster = drawSourceToRaster(bitmap, bitmap.width, bitmap.height);
      bitmap.close();
      props.onRaster("Image", raster.width, raster.height, [raster.frame]);
      setPreviewFrame(0);
      setStatus(`Embedded image ${raster.width}×${raster.height}.`);
    } catch (error) {
      setStatus(String(error));
    } finally {
      setBusy(false);
    }
  };

  const importVideo = async (file: File) => {
    setBusy(true);
    setStatus("Decoding bounded video frames…");
    const url = URL.createObjectURL(file);
    try {
      const video = document.createElement("video");
      video.muted = true;
      video.preload = "auto";
      video.src = url;
      await waitForEvent(video, "loadedmetadata");
      if (video.readyState < 2) await waitForEvent(video, "loadeddata");
      const duration = Number.isFinite(video.duration) && video.duration > 0 ? video.duration : 1;
      const frameTotal = Math.max(2, Math.min(MAX_FRAMES, Math.ceil(duration * 8)));
      const size = fittedRasterSize(video.videoWidth, video.videoHeight);
      const canvas = document.createElement("canvas");
      canvas.width = size.width;
      canvas.height = size.height;
      const context = canvas.getContext("2d");
      if (!context) throw new Error("2D canvas is unavailable.");
      const frames: ColorMappingFrame[] = [];
      for (let index = 0; index < frameTotal; index += 1) {
        if (index > 0) {
          video.currentTime = (duration * index) / frameTotal;
          await waitForEvent(video, "seeked");
        }
        context.clearRect(0, 0, size.width, size.height);
        context.drawImage(video, 0, 0, size.width, size.height);
        frames.push(frameFromCanvas(canvas));
        setStatus(`Decoding video frame ${index + 1}/${frameTotal}…`);
      }
      props.onRaster("Video", size.width, size.height, frames);
      setPreviewFrame(0);
      setStatus(`Embedded ${frames.length} video frames at ${size.width}×${size.height}.`);
      video.removeAttribute("src");
      video.load();
    } catch (error) {
      setStatus(String(error));
    } finally {
      URL.revokeObjectURL(url);
      setBusy(false);
    }
  };

  const renderText = () => {
    const canvas = document.createElement("canvas");
    canvas.width = 64;
    canvas.height = 32;
    const context = canvas.getContext("2d");
    if (!context) {
      setStatus("2D canvas is unavailable.");
      return;
    }
    context.fillStyle = "#000000";
    context.fillRect(0, 0, canvas.width, canvas.height);
    context.fillStyle = textColor();
    context.textAlign = "center";
    context.textBaseline = "middle";
    context.font = "700 24px system-ui, sans-serif";
    context.fillText(textSource().slice(0, 32), canvas.width / 2, canvas.height / 2, canvas.width - 4);
    props.onRaster("Text", canvas.width, canvas.height, [frameFromCanvas(canvas)]);
    setPreviewFrame(0);
    setStatus(`Embedded text as ${canvas.width}×${canvas.height} RGB16.`);
  };

  const freezeFixtureCells = () => {
    props.onCells(normalizedFixturePoints().map((fixture) => ({
      fixture_id: fixture.id,
      beam_index: 0,
      selection_index: fixture.selectionIndex,
      u: fixture.u,
      v: fixture.v,
      feature_attribute: null,
    })));
    setStatus(`Frozen ${props.fixtures.length} stage positions as matrix cells.`);
  };

  return (
    <section class="colorMappingEffectEditor" aria-label="Colour Mapping effect editor">
      <header class="valueEffectHeader">
        <div><strong>2D Colour Mapping</strong><span>Embedded image, text, or bounded video frames mapped to lighting cells</span></div>
        <div class="valueEffectStatusStrip">
          <span><small>Source</small><strong>{props.sourceKind}</strong></span>
          <span><small>Raster</small><strong>{props.width}×{props.height}</strong></span>
          <span><small>Frames / cells</small><strong>{frameCount()} / {previewCells().length}</strong></span>
        </div>
      </header>

      <div class="colorMappingPreviewGrid">
        <div class="colorMappingPreview" role="img" aria-label={`Colour Mapping raster preview with ${previewCells().length} cells`}>
          <canvas ref={previewCanvas} />
          <div class="colorMappingCellOverlay" aria-hidden="true">
            <For each={previewCells()}>{(cell, index) => <span style={{ left: `${clamp(cell.u, 0, 1) * 100}%`, top: `${clamp(cell.v, 0, 1) * 100}%` }} title={cell.label}>{index() + 1}</span>}</For>
          </div>
        </div>
        <fieldset class="colorMappingSourcePanel">
          <legend>Embedded source</legend>
          <label class="colorMappingFileButton">Image<input type="file" accept="image/*" disabled={busy()} onChange={(event) => { const file = event.currentTarget.files?.[0]; if (file) void importImage(file); event.currentTarget.value = ""; }} /></label>
          <label class="colorMappingFileButton">Video<input type="file" accept="video/*" disabled={busy()} onChange={(event) => { const file = event.currentTarget.files?.[0]; if (file) void importVideo(file); event.currentTarget.value = ""; }} /></label>
          <div class="colorMappingTextRow"><input value={textSource()} maxlength={32} aria-label="Colour Mapping text" onInput={(event) => setTextSource(event.currentTarget.value)} /><input type="color" value={textColor()} aria-label="Colour Mapping text color" onInput={(event) => setTextColor(event.currentTarget.value)} /><button type="button" disabled={busy() || textSource().length === 0} onClick={renderText}>Render text</button></div>
          <Show when={frameCount() > 1}><label>Preview frame<input type="range" min="0" max={frameCount() - 1} value={Math.min(previewFrame(), frameCount() - 1)} onInput={(event) => setPreviewFrame(Number(event.currentTarget.value))} /><span class="tabularNums">{Math.min(previewFrame(), frameCount() - 1) + 1}/{frameCount()}</span></label></Show>
          <p class="textPretty" role="status">{status() || "Media is converted before engine submission; no decoder runs at 44 Hz."}</p>
        </fieldset>
      </div>

      <div class="colorMappingCellActions">
        <button type="button" disabled={props.fixtures.length === 0} onClick={freezeFixtureCells}>Freeze stage positions as cells</button>
        <button type="button" disabled={props.cells.length === 0} onClick={() => props.onCells([])}>Follow live stage positions</button>
        <span>{props.cells.length > 0 ? `${props.cells.length} authored cells` : "Stage X/Z normalized at command time"}</span>
      </div>

      <div class="colorMappingControlGrid">
        <label>Direction<select value={props.playbackDirection} onInput={(event) => props.onPlaybackDirection(event.currentTarget.value as ColorMappingPlaybackDirection)}><For each={playbackDirections}>{(direction) => <option value={direction}>{direction}</option>}</For></select></label>
        <label>Period ms<input type="number" min="10" step="10" value={props.periodMs} onInput={(event) => props.onPeriodMs(Math.max(10, Math.round(Number(event.currentTarget.value) || 10)))} /></label>
        <label>Wrap<select value={props.wrapMode} onInput={(event) => props.onWrapMode(event.currentTarget.value as ColorMappingWrapMode)}><option value="Clamp">Clamp</option><option value="Repeat">Repeat</option><option value="Mirror">Mirror</option></select></label>
        <label>Sampling<select value={props.sampling} onInput={(event) => props.onSampling(event.currentTarget.value as ColorMappingSampling)}><option value="Nearest">Nearest</option><option value="Bilinear">Bilinear</option></select></label>
        <label>Offset U<input type="number" min="-16" max="16" step="0.01" value={props.offsetU} onInput={(event) => props.onOffsetU(clamp(Number(event.currentTarget.value), -16, 16))} /></label>
        <label>Offset V<input type="number" min="-16" max="16" step="0.01" value={props.offsetV} onInput={(event) => props.onOffsetV(clamp(Number(event.currentTarget.value), -16, 16))} /></label>
        <label>Scale U<input type="number" min="-16" max="16" step="0.01" value={props.scaleU} onInput={(event) => props.onScaleU(clamp(Number(event.currentTarget.value), -16, 16))} /></label>
        <label>Scale V<input type="number" min="-16" max="16" step="0.01" value={props.scaleV} onInput={(event) => props.onScaleV(clamp(Number(event.currentTarget.value), -16, 16))} /></label>
        <label>Rotation °<input type="number" min="-3600" max="3600" step="1" value={props.rotationDegrees} onInput={(event) => props.onRotationDegrees(clamp(Number(event.currentTarget.value), -3600, 3600))} /></label>
        <label>Phase %<input type="number" min="0" max="100" step="1" value={phasePercent()} onInput={(event) => props.onPhase(clamp(Number(event.currentTarget.value) / 100, 0, 1))} /></label>
      </div>
      <div class="colorEffectClockPresets" aria-label="Colour Mapping clock sync presets"><For each={clockPresets}>{(beats) => <button type="button" class={props.clockSyncBeats === beats ? "active" : ""} aria-pressed={props.clockSyncBeats === beats} onClick={() => props.onClockSyncBeats(beats)}>{beats === null ? "Free" : `${beats} beat`}</button>}</For></div>
    </section>
  );
}

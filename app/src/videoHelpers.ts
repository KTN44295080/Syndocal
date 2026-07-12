// Pure video-source label, media path/time formatting helpers extracted from App.tsx.
// Depend only on the VideoSourceKind type; no SolidJS/state deps.
import type { VideoSourceKind } from "./types";

export const videoSourceInputLabel = (kind: VideoSourceKind) => {
  switch (kind) {
    case "File":
    case "StillImage":
      return "Source path";
    case "Camera":
      return "Camera device";
    case "ScreenCapture":
      return "Display source";
    case "Ndi":
      return "NDI source";
    case "Spout":
      return "Spout sender";
    case "Syphon":
      return "Syphon server";
  }
};

export const videoSourceInputPlaceholder = (kind: VideoSourceKind) => {
  switch (kind) {
    case "File":
      return "C:\\path\\clip.mp4";
    case "StillImage":
      return "C:\\path\\image.png";
    case "Camera":
      return "Camera device name (Windows) or device/index";
    case "ScreenCapture":
      return "Primary desktop (optional display/index)";
    case "Ndi":
      return "OBS / Program";
    case "Spout":
      return "Spout sender name";
    case "Syphon":
      return "Syphon server name";
  }
};

export const videoSourceCanBrowseFile = (kind: VideoSourceKind) => kind === "File" || kind === "StillImage";

export const videoSourceKindLabel = (kind: VideoSourceKind) => {
  switch (kind) {
    case "File":
      return "File";
    case "StillImage":
      return "Still";
    case "Camera":
      return "Camera";
    case "ScreenCapture":
      return "Screen";
    case "Ndi":
      return "NDI";
    case "Spout":
      return "Spout";
    case "Syphon":
      return "Syphon";
  }
};

export const mediaLabelFromPath = (path: string) => {
  const fileName = path.split(/[\\/]/).pop()?.trim() || path.trim();
  return fileName.replace(/\.[^/.]+$/, "") || fileName;
};

export const shouldReplaceVideoLayerDraftLabel = (label: string) => {
  const trimmed = label.trim();
  return trimmed.length === 0 || /^Layer \d+$/i.test(trimmed);
};

export const formatDuration = (durationMs?: number | null) => {
  if (!durationMs) {
    return null;
  }
  const totalSeconds = Math.round(durationMs / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
};

export const formatVideoTime = (positionMs: number, durationMs?: number | null) => {
  const position = formatDuration(Math.max(0, positionMs)) ?? "0:00";
  const duration = formatDuration(durationMs);
  return duration ? `${position} / ${duration}` : position;
};

export const videoSourceMetadataLabel = (source: {
  codec?: string | null;
  metadata?: { duration_ms?: number | null; width?: number | null; height?: number | null; frame_rate?: number | null; has_audio?: boolean } | null;
}) => {
  const metadata = source.metadata;
  const parts = [
    source.codec,
    metadata?.width && metadata?.height ? `${metadata.width}x${metadata.height}` : null,
    metadata?.frame_rate ? `${metadata.frame_rate.toFixed(2)} fps` : null,
    metadata?.has_audio === true ? "audio" : metadata?.has_audio === false ? "silent" : null,
    formatDuration(metadata?.duration_ms),
  ].filter((part): part is string => Boolean(part));
  return parts.join(" / ");
};

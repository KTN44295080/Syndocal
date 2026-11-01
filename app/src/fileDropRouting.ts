import { isVideoPerformFirstShowDropTarget } from "./videoPerformFirstShow";
import { videoClipSlotDropTarget } from "./videoClipSlotBankModel";
import { unsupportedFileDropMessage } from "./nativeFileDrop";

const visualExtensions = ["mp4", "mov", "mxf", "avi", "mkv", "webm", "gif", "png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff"];
const audioExtensions = ["wav", "mp3", "m4a", "aac", "flac", "aiff", "aif", "ogg", "opus"];
export const isVisualMediaFilePath = (path: string) => visualExtensions.some(extension => path.trim().toLowerCase().endsWith(`.${extension}`));
export const isTimelineAudioFilePath = (path: string) => audioExtensions.some(extension => path.trim().toLowerCase().endsWith(`.${extension}`));
export const isSyndocalProjectPath = (path: string) => path.trim().toLowerCase().endsWith(".sdc");

export function routeFileDrop(paths: string[], target: Element | null, actions: {
  firstShow: (paths: string[]) => unknown;
  importMedia: (paths: string[]) => unknown;
  assignSlots: (paths: string[], layerId: number, beforeSlotId: number | null) => unknown;
  openProject: (path: string) => unknown;
  addAudio: (layerId: number, path: string) => unknown;
  message: (message: string) => void;
}): void {
  if (paths.length === 0) return;
  const project = paths.find(isSyndocalProjectPath);
  if (project && target?.closest("[data-project-drop-surface]")) {
    void actions.openProject(project);
    return;
  }
  const firstShow = isVideoPerformFirstShowDropTarget(target);
  const library = Boolean(target?.closest("[data-media-library-rail]"));
  // Match the visual picker filter. Reject unsupported/mixed gestures instead
  // of silently importing a subset or sending Audio/SDC to the media engine.
  if (firstShow || library) {
    if (!paths.every(isVisualMediaFilePath)) actions.message(unsupportedFileDropMessage);
    else if (firstShow) void actions.firstShow(paths);
    else void actions.importMedia(paths);
    return;
  }
  const audio = paths.find(isTimelineAudioFilePath);
  if (audio) {
    const lane = target?.closest<HTMLElement>("[data-timeline-audio-layer-id], [data-timeline-layer-kind='Audio'][data-timeline-layer-id]");
    const id = Number(lane?.dataset.timelineAudioLayerId ?? lane?.dataset.timelineLayerId);
    if (!Number.isFinite(id)) actions.message("Drop Audio Clips on the Audio section header or an Audio lane.");
    else void actions.addAudio(id, audio);
    return;
  }
  const visual = paths.filter(isVisualMediaFilePath);
  if (visual.length > 0) {
    const slot = videoClipSlotDropTarget(target);
    if (slot) void actions.assignSlots(visual, slot.layerId, slot.beforeSlotId);
    else actions.message("Drop visual media on Media Library, a Video layer, or an exact Clip Slot.");
    return;
  }
  actions.message(project ? "Drop .sdc projects on the Project menu button." : unsupportedFileDropMessage);
}

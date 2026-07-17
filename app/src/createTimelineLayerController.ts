import { sortedTimelineLayers, timelineLayerKindOrder } from "./timelineLayers";
import type { TimelineLayerKind, TimelineLayerSummary } from "./types";

interface TimelineLayerControllerOptions {
  layers: () => TimelineLayerSummary[];
  invoke: <T = unknown>(command: string, args?: Record<string, unknown>) => Promise<T>;
  setMessage: (message: string) => void;
  refreshSnapshot: () => Promise<unknown>;
}

const timelineLayerErrorMessage = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

export const createTimelineLayerController = (options: TimelineLayerControllerOptions) => {
  const refresh = async () => {
    await options.refreshSnapshot();
  };

  return {
    async add(label: string, kind: TimelineLayerKind) {
      const sameKindCount = options.layers().filter((layer) => layer.kind === kind).length;
      const normalizedLabel = label.trim() || `${kind} ${sameKindCount + 1}`;
      try {
        const layerId = await options.invoke<number>("add_timeline_layer", { label: normalizedLabel, kind });
        options.setMessage(`Added ${kind} layer ${normalizedLabel} (${layerId})`);
        await refresh();
      } catch (error) {
        options.setMessage(timelineLayerErrorMessage(error));
      }
    },

    async update(layer: TimelineLayerSummary) {
      try {
        await options.invoke("update_timeline_layer", { layer });
        options.setMessage(`Updated timeline layer ${layer.label}`);
        await refresh();
      } catch (error) {
        options.setMessage(timelineLayerErrorMessage(error));
      }
    },

    async remove(layerId: number, reassignToLayerId: number | null) {
      const layer = options.layers().find((candidate) => candidate.id === layerId);
      try {
        await options.invoke("remove_timeline_layer", { layerId, reassignToLayerId });
        options.setMessage(`Removed timeline layer ${layer?.label ?? layerId}`);
        await refresh();
        return true;
      } catch (error) {
        options.setMessage(timelineLayerErrorMessage(error));
        return false;
      }
    },

    async reorder(layerId: number, direction: -1 | 1) {
      const ordered = sortedTimelineLayers(options.layers());
      const layer = ordered.find((candidate) => candidate.id === layerId);
      if (!layer) return;
      const section = ordered.filter((candidate) => candidate.kind === layer.kind);
      const index = section.findIndex((candidate) => candidate.id === layerId);
      const targetIndex = index + direction;
      if (index < 0 || targetIndex < 0 || targetIndex >= section.length) return;
      [section[index], section[targetIndex]] = [section[targetIndex], section[index]];
      const layerIds = timelineLayerKindOrder.flatMap((kind) =>
        (kind === layer.kind ? section : ordered.filter((candidate) => candidate.kind === kind))
          .map((candidate) => candidate.id));
      try {
        await options.invoke("reorder_timeline_layers", { layerIds });
        options.setMessage(`Moved timeline layer ${layer.label} ${direction < 0 ? "up" : "down"}`);
        await refresh();
      } catch (error) {
        options.setMessage(timelineLayerErrorMessage(error));
      }
    },
  };
};

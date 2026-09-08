import { batch, createEffect, createMemo, createSignal, onCleanup, untrack, type Accessor } from "solid-js";
import type { MediaAssetId, MediaAssetSummary, VideoLayerSummary, VideoSourceKind } from "./types";
import type { ProjectAuthorityToken } from "./projectAuthority";
import { createLatestThumbnailBatch } from "./createLatestThumbnailBatch";
import { readThumbnailWithRetry } from "./thumbnailReadRetry";

interface Options {
  layers: Accessor<readonly VideoLayerSummary[]>;
  assets: Accessor<readonly MediaAssetSummary[]>;
  projectMappingsAuthority: Accessor<ProjectAuthorityToken>;
  isProjectAuthorityIdentityCurrent: (authority: ProjectAuthorityToken) => boolean;
  isTauriRuntime: () => boolean;
  loadVideoLayerThumbnail: (id: number) => Promise<string>;
  loadMediaAssetThumbnail: (id: MediaAssetId) => Promise<string>;
}

/** Owns opt-in thumbnail caches; project replacement resets permission and both batches. */
export const createMediaThumbnailController = (options: Options) => {
  const {
    projectMappingsAuthority, isProjectAuthorityIdentityCurrent, isTauriRuntime,
    loadVideoLayerThumbnail, loadMediaAssetThumbnail,
  } = options;
  const [videoClipThumbnails, setVideoClipThumbnails] = createSignal<Record<number, string>>({});
  const [mediaAssetThumbnails, setMediaAssetThumbnails] = createSignal<Record<number, string>>({});
  const [videoThumbnailAccessAuthorized, setVideoThumbnailAccessAuthorized] = createSignal(false);
  const [videoThumbnailBusy, setVideoThumbnailBusy] = createSignal(false);
  const [mediaAssetThumbnailBusy, setMediaAssetThumbnailBusy] = createSignal(false);
  const videoBatch = createLatestThumbnailBatch(setVideoThumbnailBusy);
  const assetBatch = createLatestThumbnailBatch(setMediaAssetThumbnailBusy);
  let disposed = false;
  let videoThumbnailUrlCache: Record<number, string> = {};
  let mediaAssetThumbnailUrlCache: Record<number, string> = {};
  const videoThumbnailSignatures = new Map<number, string>();
  const mediaAssetThumbnailSignatures = new Map<number, string>();
  const [videoReloadRequest, setVideoReloadRequest] = createSignal(0);
  const [assetReloadRequest, setAssetReloadRequest] = createSignal(0);
  const authorizeVideoThumbnailAccess = () => batch(() => {
    if (disposed) return;
    setVideoThumbnailAccessAuthorized(true);
    // Repeated clicks never retire active work; the independent idle lane may retry.
    if (!untrack(videoThumbnailBusy)) setVideoReloadRequest(value => value + 1);
    if (!untrack(mediaAssetThumbnailBusy)) setAssetReloadRequest(value => value + 1);
  });
  const reset = () => {
    videoBatch.clear();
    assetBatch.clear();
    videoThumbnailUrlCache = {};
    mediaAssetThumbnailUrlCache = {};
    videoThumbnailSignatures.clear();
    mediaAssetThumbnailSignatures.clear();
    setVideoThumbnailAccessAuthorized(false);
    setVideoClipThumbnails({});
    setMediaAssetThumbnails({});
  };
  // Snapshot publications often retain these arrays. Track their identity before
  // projecting/serializing sources so unrelated updates do not scan the catalog.
  const layers = createMemo(options.layers);
  const assets = createMemo(options.assets);
  const videoThumbnailSourceSignature = createMemo(() => JSON.stringify(
    layers().map((layer) => ({
      id: layer.id,
      kind: layer.source.kind,
      path: layer.source.path ?? null,
      name: layer.source.name ?? null,
    })),
  ));
  const mediaAssetThumbnailSourceSignature = createMemo(() => JSON.stringify(
    assets().map((asset) => ({
      id: asset.id,
      kind: asset.source.kind,
      path: asset.source.path ?? null,
      name: asset.source.name ?? null,
      hash_algorithm: asset.content_hash?.algorithm ?? null,
      hash_hex: asset.content_hash?.hex ?? null,
      byte_size: asset.byte_size ?? null,
    })),
  ));
  // This deliberately projects only the durable E/R/H identity.  The
  // authority signal itself is re-published as a fresh object by polling, and
  // subscribing to that object would otherwise restart a sequential batch on
  // every equivalent publication.  A genuine identity change produces a
  // different scalar, so the effect below starts one fresh fenced batch.
  const mediaAssetThumbnailAuthoritySignature = createMemo(() => {
    const authority = projectMappingsAuthority();
    return JSON.stringify({
      project_epoch: authority.project_epoch,
      project_revision: authority.project_revision,
      checkpoint_hash: authority.checkpoint_hash,
    });
  });
  createEffect(() => {
    videoReloadRequest(); // An explicit load action retries missing entries, not cached successes.
    const sources = JSON.parse(videoThumbnailSourceSignature()) as Array<{
      id: number;
      kind: VideoSourceKind;
      path: string | null;
      name: string | null;
    }>;
    if (!videoThumbnailAccessAuthorized()) {
      videoBatch.clear();
      setVideoClipThumbnails({});
      return;
    }
    const activeIds = new Set(sources.map((source) => source.id));
    for (const layerId of videoThumbnailSignatures.keys()) {
      if (!activeIds.has(layerId)) videoThumbnailSignatures.delete(layerId);
    }
    videoThumbnailUrlCache = Object.fromEntries(
      Object.entries(videoThumbnailUrlCache).filter(([layerId]) => activeIds.has(Number(layerId))),
    );
    if (!isTauriRuntime()) {
      videoBatch.clear();
      setVideoClipThumbnails(videoThumbnailUrlCache);
      return;
    }
    videoBatch.replace(async (isCurrent) => {
      const nextUrls = { ...videoThumbnailUrlCache };
      const nextSignatures = new Map(videoThumbnailSignatures);
      for (const source of sources) {
        if (!isCurrent()) return;
        const signature = JSON.stringify(source);
        if (nextSignatures.get(source.id) === signature && nextUrls[source.id]) continue;
        try {
          nextUrls[source.id] = await readThumbnailWithRetry(() => loadVideoLayerThumbnail(source.id), isCurrent);
          nextSignatures.set(source.id, signature);
        } catch {
          delete nextUrls[source.id];
          nextSignatures.delete(source.id);
        }
        if (!isCurrent()) return;
      }
      if (!isCurrent()) return;
      videoThumbnailUrlCache = nextUrls;
      videoThumbnailSignatures.clear();
      for (const [layerId, signature] of nextSignatures) {
        videoThumbnailSignatures.set(layerId, signature);
      }
      setVideoClipThumbnails(nextUrls);
    }, () => {
      videoThumbnailUrlCache = {};
      videoThumbnailSignatures.clear();
      setVideoClipThumbnails({});
    });
  });
  createEffect(() => {
    assetReloadRequest(); // An explicit load action retries missing entries, not cached successes.
    const sources = JSON.parse(mediaAssetThumbnailSourceSignature()) as Array<{
      id: MediaAssetId;
      kind: VideoSourceKind;
      path: string | null;
      name: string | null;
      hash_algorithm: string | null;
      hash_hex: string | null;
      byte_size: number | null;
    }>;
    const authority = JSON.parse(mediaAssetThumbnailAuthoritySignature()) as ProjectAuthorityToken;
    if (!videoThumbnailAccessAuthorized()) {
      assetBatch.clear();
      setMediaAssetThumbnails({});
      return;
    }
    const thumbnailable = sources.filter((source) => source.kind === "File" || source.kind === "StillImage");
    const activeIds = new Set(thumbnailable.map((source) => source.id));
    for (const assetId of mediaAssetThumbnailSignatures.keys()) {
      if (!activeIds.has(assetId)) mediaAssetThumbnailSignatures.delete(assetId);
    }
    mediaAssetThumbnailUrlCache = Object.fromEntries(
      Object.entries(mediaAssetThumbnailUrlCache).filter(([assetId]) => activeIds.has(Number(assetId))),
    );
    if (!isTauriRuntime()) {
      assetBatch.clear();
      setMediaAssetThumbnails(mediaAssetThumbnailUrlCache);
      return;
    }
    assetBatch.replace(async (isCurrent) => {
      const nextUrls = { ...mediaAssetThumbnailUrlCache };
      const nextSignatures = new Map(mediaAssetThumbnailSignatures);
      for (const source of thumbnailable) {
        if (!isCurrent() || !untrack(() => isProjectAuthorityIdentityCurrent(authority))) return;
        const signature = JSON.stringify(source);
        if (nextSignatures.get(source.id) === signature && nextUrls[source.id]) continue;
        try {
          nextUrls[source.id] = await readThumbnailWithRetry(() => loadMediaAssetThumbnail(source.id),
            () => isCurrent() && untrack(() => isProjectAuthorityIdentityCurrent(authority)));
          nextSignatures.set(source.id, signature);
        } catch {
          delete nextUrls[source.id];
          nextSignatures.delete(source.id);
        }
        if (!isCurrent() || !untrack(() => isProjectAuthorityIdentityCurrent(authority))) return;
      }
      // Cache hits can reach this check synchronously inside the effect. Read
      // current authority without subscribing beyond the durable signature.
      if (!isCurrent() || !untrack(() => isProjectAuthorityIdentityCurrent(authority))) return;
      mediaAssetThumbnailUrlCache = nextUrls;
      mediaAssetThumbnailSignatures.clear();
      for (const [assetId, signature] of nextSignatures) mediaAssetThumbnailSignatures.set(assetId, signature);
      setMediaAssetThumbnails(nextUrls);
    }, () => {
      mediaAssetThumbnailUrlCache = {};
      mediaAssetThumbnailSignatures.clear();
      setMediaAssetThumbnails({});
    });
  });
  onCleanup(() => {
    disposed = true;
    // Retain active reads until settled; discard successors and stale results.
    videoBatch.dispose();
    assetBatch.dispose();
  });
  return {
    videoClipThumbnails, mediaAssetThumbnails, videoThumbnailAccessAuthorized,
    authorizeVideoThumbnailAccess, videoThumbnailBusy, mediaAssetThumbnailBusy, reset,
    layerThumbnailView: {
      get thumbnails() { return videoClipThumbnails(); },
      get thumbnailsAuthorized() { return videoThumbnailAccessAuthorized(); },
      get thumbnailsBusy() { return videoThumbnailBusy(); },
      get thumbnailsAvailable() { return isTauriRuntime(); },
    },
    assetThumbnailView: {
      get thumbnails() { return mediaAssetThumbnails(); },
      get thumbnailsBusy() { return mediaAssetThumbnailBusy(); },
      get thumbnailsAvailable() { return isTauriRuntime(); },
    },
  };
};

import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { VideoFrame } from "./types";

export type ThumbnailLane = "layer" | "asset";
export interface ThumbnailTicket {
  schemaVersion: 1;
  lane: ThumbnailLane;
  requestId: string;
}

/** Structural type keeps the request protocol independent from Tauri's concrete Channel class. */
export interface ThumbnailStartChannel {
  onmessage: (message: unknown) => void;
  toJSON: () => string;
}

interface Options {
  invoke: FrontendTauriInvoke;
  createChannel: () => ThumbnailStartChannel;
}

const disposeChannel = (channel: ThumbnailStartChannel) => {
  // Tauri v2 exposes no public Channel.dispose(). Its runtime Channel owns a
  // cleanupCallback that removes the transform callback; use it only as an
  // optional lifecycle seam so a pre-dispatch rejection cannot retain a
  // callback forever. Normal command completion also makes this idempotent.
  const cleanup = Reflect.get(channel, "cleanupCallback");
  if (typeof cleanup === "function") cleanup.call(channel);
};

const validTicket = (value: unknown, lane: ThumbnailLane): value is ThumbnailTicket => {
  if (value === null || typeof value !== "object" || Array.isArray(value)) return false;
  const item = value as Partial<ThumbnailTicket>;
  return Object.keys(item).sort().join(",") === "lane,requestId,schemaVersion"
    && item.schemaVersion === 1
    && item.lane === lane
    && typeof item.requestId === "string"
    && /^[0-9a-f]{32}$/.test(item.requestId);
};

/** Abort signals cancel one exact announced native job and never another active request. */
export async function requestNativeThumbnail(
  options: Options,
  lane: ThumbnailLane,
  id: number,
  signal: AbortSignal,
): Promise<VideoFrame> {
  signal.throwIfAborted();
  const started = options.createChannel();
  let ticket: ThumbnailTicket | undefined;
  let finished = false;
  let protocolError: Error | undefined;
  let cancellation: Promise<void> | undefined;
  let cancellationError: unknown;
  let cancellationFailed = false;

  const requestCancel = () => {
    if (!ticket || finished || cancellation) return;
    const exactTicket = ticket;
    cancellation = Promise.resolve()
      .then(() => options.invoke<boolean>("cancel_native_thumbnail_request_v1", { ticket: exactTicket }))
      .then((ack) => {
        if (ack !== true) throw new Error("Thumbnail cancellation was rejected");
      })
      .catch((error: unknown) => {
        cancellationFailed = true;
        cancellationError = error;
      });
  };

  started.onmessage = (message) => {
    if (finished) return;
    if (!validTicket(message, lane) || (ticket && ticket.requestId !== message.requestId)) {
      protocolError = new Error("Invalid thumbnail start ticket");
      return;
    }
    ticket = { schemaVersion: 1, lane: message.lane, requestId: message.requestId };
    if (signal.aborted) requestCancel();
  };
  signal.addEventListener("abort", requestCancel, { once: true });

  let frame: VideoFrame | undefined;
  let readFailed = false;
  let readError: unknown;
  try {
    signal.throwIfAborted();
    frame = lane === "layer"
      ? await options.invoke<VideoFrame>("get_video_layer_thumbnail", {
        layerId: id, width: 160, height: 90, started,
      })
      : await options.invoke<VideoFrame>("get_media_asset_thumbnail", {
        assetId: id, width: 160, height: 90, started,
      });
  } catch (error) {
    readFailed = true;
    readError = error;
  } finally {
    // Do not detach the native ownership protocol before its command has settled.
    finished = true;
    signal.removeEventListener("abort", requestCancel);
    started.onmessage = () => {};
    await cancellation;
    disposeChannel(started);
  }
  if (cancellationFailed) {
    throw new Error("Thumbnail cancellation was not acknowledged", { cause: cancellationError });
  }
  if (protocolError) throw protocolError;
  signal.throwIfAborted();
  if (readFailed) throw readError;
  return frame!;
}

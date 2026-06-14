// GDTF wheel-slot media helpers extracted from App.tsx: path/cache-key derivation,
// a guard for when media can be loaded, and byte-payload -> object URL conversion.

export type WheelMediaBytes = number[] | Uint8Array | ArrayBuffer;

export interface WheelMediaPayload {
  bytes: WheelMediaBytes;
  mime_type: string;
}

export const wheelSlotMediaPath = (media: string | null | undefined) =>
  media?.split(/[\\/]/).filter(Boolean).pop() ?? null;

export const wheelMediaCacheKey = (profileSourcePath: string, media: string) => `${profileSourcePath}\n${media}`;

export const canLoadWheelMedia = (profileSourcePath: string) =>
  Boolean(profileSourcePath.trim()) &&
  !profileSourcePath.startsWith("memory://") &&
  !profileSourcePath.startsWith("snapshot://");

export const wheelMediaPayloadToObjectUrl = (payload: WheelMediaPayload) => {
  const bytes = payload.bytes;
  const data =
    bytes instanceof ArrayBuffer
      ? new Uint8Array(bytes)
      : bytes instanceof Uint8Array
        ? bytes
        : new Uint8Array(bytes);
  const copy = new ArrayBuffer(data.byteLength);
  new Uint8Array(copy).set(data);
  const mimeType = payload.mime_type.trim() || "image/png";
  const blob = new Blob([copy], { type: mimeType });
  return URL.createObjectURL(blob);
};

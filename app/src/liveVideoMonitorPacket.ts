export const LIVE_VIDEO_MONITOR_HEADER_SIZE = 40;

const LIVE_VIDEO_MONITOR_MAGIC = [0x53, 0x59, 0x4c, 0x56] as const;

export type LiveVideoMonitorKind = "program" | "preview";

export interface LiveVideoMonitorPacket {
  status: "frame" | "busy";
  kind: LiveVideoMonitorKind;
  sequence: bigint;
  ptsMs: bigint;
  renderUs: number;
  encodeUs: number;
  width: number;
  height: number;
  jpeg: Uint8Array;
}

const packetBytes = (value: unknown): Uint8Array => {
  if (value instanceof ArrayBuffer) return new Uint8Array(value);
  if (ArrayBuffer.isView(value)) {
    return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
  }
  if (Array.isArray(value) && value.every((entry) => Number.isInteger(entry) && entry >= 0 && entry <= 255)) {
    return Uint8Array.from(value as number[]);
  }
  throw new Error("Live monitor returned an unsupported binary response.");
};

export const parseLiveVideoMonitorPacket = (value: unknown): LiveVideoMonitorPacket => {
  const bytes = packetBytes(value);
  if (bytes.byteLength < LIVE_VIDEO_MONITOR_HEADER_SIZE) {
    throw new Error("Live monitor packet is shorter than its header.");
  }
  for (let index = 0; index < LIVE_VIDEO_MONITOR_MAGIC.length; index += 1) {
    if (bytes[index] !== LIVE_VIDEO_MONITOR_MAGIC[index]) {
      throw new Error("Live monitor packet has an invalid signature.");
    }
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const version = view.getUint8(4);
  const statusCode = view.getUint8(5);
  const kindCode = view.getUint8(6);
  if (version !== 1) throw new Error(`Unsupported live monitor packet version ${version}.`);
  if (statusCode > 1) throw new Error(`Invalid live monitor packet status ${statusCode}.`);
  if (kindCode > 1) throw new Error(`Invalid live monitor kind ${kindCode}.`);

  const width = view.getUint16(32, true);
  const height = view.getUint16(34, true);
  const jpegLength = view.getUint32(36, true);
  if (LIVE_VIDEO_MONITOR_HEADER_SIZE + jpegLength !== bytes.byteLength) {
    throw new Error("Live monitor packet length does not match its header.");
  }
  if (statusCode === 0 && (width === 0 || height === 0 || jpegLength < 4)) {
    throw new Error("Live monitor frame is missing image data.");
  }
  if (statusCode === 1 && jpegLength !== 0) {
    throw new Error("Busy live monitor packets must not contain image data.");
  }

  const jpeg = bytes.subarray(LIVE_VIDEO_MONITOR_HEADER_SIZE);
  if (statusCode === 0 && (jpeg[0] !== 0xff || jpeg[1] !== 0xd8)) {
    throw new Error("Live monitor frame is not a JPEG image.");
  }

  return {
    status: statusCode === 0 ? "frame" : "busy",
    kind: kindCode === 0 ? "program" : "preview",
    sequence: view.getBigUint64(8, true),
    ptsMs: view.getBigUint64(16, true),
    renderUs: view.getUint32(24, true),
    encodeUs: view.getUint32(28, true),
    width,
    height,
    jpeg,
  };
};

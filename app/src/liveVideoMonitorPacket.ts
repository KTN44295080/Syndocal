export const LIVE_VIDEO_MONITOR_HEADER_SIZE = 40;

const LIVE_VIDEO_MONITOR_MAGIC = [0x53, 0x59, 0x4c, 0x56] as const;

export type LiveVideoMonitorKind = "program" | "preview";

interface LiveVideoMonitorPacketHeader {
  status: "frame" | "busy";
  kind: LiveVideoMonitorKind;
  sequence: bigint;
  ptsMs: bigint;
  renderUs: number;
  encodeUs: number;
  width: number;
  height: number;
}

export type LiveVideoMonitorPacket = LiveVideoMonitorPacketHeader & (
  | { encoding: "jpeg"; jpeg: Uint8Array<ArrayBuffer> }
  | { encoding: "rgba"; rgba: Uint8ClampedArray<ArrayBuffer> }
);

const packetBytes = (value: unknown): Uint8Array<ArrayBuffer> => {
  if (value instanceof ArrayBuffer) return new Uint8Array(value);
  if (ArrayBuffer.isView(value) && value.buffer instanceof ArrayBuffer) {
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
  const encodingCode = view.getUint8(7);
  if (version !== 2) throw new Error(`Unsupported live monitor packet version ${version}.`);
  if (statusCode > 1) throw new Error(`Invalid live monitor packet status ${statusCode}.`);
  if (kindCode > 1) throw new Error(`Invalid live monitor kind ${kindCode}.`);

  if (encodingCode > 1) throw new Error(`Invalid live monitor encoding ${encodingCode}.`);

  const width = view.getUint16(32, true);
  const height = view.getUint16(34, true);
  const payloadLength = view.getUint32(36, true);
  if (LIVE_VIDEO_MONITOR_HEADER_SIZE + payloadLength !== bytes.byteLength) {
    throw new Error("Live monitor packet length does not match its header.");
  }
  if (statusCode === 0 && (width === 0 || height === 0 || payloadLength < 4)) {
    throw new Error("Live monitor frame is missing image data.");
  }
  if (statusCode === 1 && payloadLength !== 0) {
    throw new Error("Busy live monitor packets must not contain image data.");
  }

  const payload = bytes.subarray(LIVE_VIDEO_MONITOR_HEADER_SIZE);
  if (statusCode === 0 && encodingCode === 0 && (payload[0] !== 0xff || payload[1] !== 0xd8)) {
    throw new Error("Live monitor frame is not a JPEG image.");
  }

  if (statusCode === 0 && encodingCode === 1 && payloadLength !== width * height * 4) {
    throw new Error("Live monitor RGBA byte length does not match its dimensions.");
  }
  const header: LiveVideoMonitorPacketHeader = {
    status: statusCode === 0 ? "frame" : "busy",
    kind: kindCode === 0 ? "program" : "preview",
    sequence: view.getBigUint64(8, true),
    ptsMs: view.getBigUint64(16, true),
    renderUs: view.getUint32(24, true),
    encodeUs: view.getUint32(28, true),
    width,
    height,
  };
  return encodingCode === 0
    ? { ...header, encoding: "jpeg", jpeg: payload }
    : { ...header, encoding: "rgba", rgba: new Uint8ClampedArray(payload.buffer, payload.byteOffset, payload.byteLength) };
};

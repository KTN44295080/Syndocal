import type { VideoFrame } from "./types";

export const assertRgbaVideoFrame = (frame: VideoFrame) => {
  if (frame.format !== "Rgba8") {
    throw new Error(`Unsupported preview format ${frame.format}`);
  }
  const expectedLength = frame.width * frame.height * 4;
  if (frame.width <= 0 || frame.height <= 0 || frame.data.length !== expectedLength) {
    throw new Error("Invalid preview frame size");
  }
};

export const drawVideoFrameToCanvas = (canvas: HTMLCanvasElement, frame: VideoFrame) => {
  assertRgbaVideoFrame(frame);
  if (canvas.width !== frame.width) {
    canvas.width = frame.width;
  }
  if (canvas.height !== frame.height) {
    canvas.height = frame.height;
  }
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("Canvas 2D context is unavailable");
  }
  context.putImageData(new ImageData(new Uint8ClampedArray(frame.data), frame.width, frame.height), 0, 0);
};

export const videoFrameToDataUrl = (frame: VideoFrame) => {
  assertRgbaVideoFrame(frame);
  const canvas = document.createElement("canvas");
  canvas.width = frame.width;
  canvas.height = frame.height;
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("Canvas 2D context is unavailable");
  }
  context.putImageData(
    new ImageData(new Uint8ClampedArray(frame.data), frame.width, frame.height),
    0,
    0,
  );
  return canvas.toDataURL("image/png");
};

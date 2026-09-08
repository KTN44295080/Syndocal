import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/nativeThumbnailRequest.ts", import.meta.url), "utf8");
const code = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { requestNativeThumbnail } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);
const flush = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };
const frame = { width: 1, height: 1, format: "rgba8", pts_ms: 0, data: [255, 0, 0, 255] };
const ticket = (lane, requestId = "a".repeat(32)) => ({ schemaVersion: 1, lane, requestId });
const fakeChannel = () => ({ onmessage: () => {}, toJSON: () => "__CHANNEL__:thumbnail-test" });
const disposableChannel = () => {
  const channel = { onmessage: () => {}, toJSON: () => "__CHANNEL__:thumbnail-test", cleaned: 0 };
  channel.cleanupCallback = () => { channel.cleaned += 1; };
  return channel;
};

{
  const calls = [];
  const result = await requestNativeThumbnail({
    createChannel: fakeChannel,
    invoke: async (command, args) => {
      calls.push({ command, args });
      if (command === "get_video_layer_thumbnail") {
        args.started.onmessage(ticket("layer"));
        return frame;
      }
      throw new Error(`unexpected command ${command}`);
    },
  }, "layer", 7, new AbortController().signal);
  assert.deepEqual(result, frame);
  assert.deepEqual(calls.map(({ command }) => command), ["get_video_layer_thumbnail"]);
}

{
  const controller = new AbortController();
  const calls = [];
  let finishRead;
  const request = requestNativeThumbnail({
    createChannel: fakeChannel,
    invoke: (command, args) => {
      calls.push({ command, args });
      if (command === "get_media_asset_thumbnail") {
        args.started.onmessage(ticket("asset", "b".repeat(32)));
        return new Promise(resolve => { finishRead = resolve; });
      }
      if (command === "cancel_native_thumbnail_request_v1") return true;
      throw new Error(`unexpected command ${command}`);
    },
  }, "asset", 9, controller.signal);
  await flush();
  controller.abort();
  await flush();
  finishRead(frame);
  await assert.rejects(request);
  assert.deepEqual(calls.map(({ command }) => command), [
    "get_media_asset_thumbnail",
    "cancel_native_thumbnail_request_v1",
  ]);
  assert.deepEqual(calls[1].args.ticket, ticket("asset", "b".repeat(32)));
}

{
  const controller = new AbortController();
  const calls = [];
  let finishRead;
  const request = requestNativeThumbnail({
    createChannel: fakeChannel,
    invoke: (command, args) => {
      calls.push({ command, args });
      if (command === "get_video_layer_thumbnail") {
        args.started.onmessage(ticket("layer", "c".repeat(32)));
        return new Promise(resolve => { finishRead = resolve; });
      }
      if (command === "cancel_native_thumbnail_request_v1") {
        return Promise.reject(new Error("owner barrier is already unregistered"));
      }
      throw new Error(`unexpected command ${command}`);
    },
  }, "layer", 11, controller.signal);
  await flush();
  controller.abort();
  await flush();
  finishRead(frame);
  await assert.rejects(request, /cancellation was not acknowledged/);
  assert.equal(calls.filter(({ command }) => command === "cancel_native_thumbnail_request_v1").length, 1);
}

{
  const controller = new AbortController();
  let finishRead;
  const request = requestNativeThumbnail({
    createChannel: fakeChannel,
    invoke: (command, args) => {
      if (command === "get_media_asset_thumbnail") {
        args.started.onmessage(ticket("asset", "e".repeat(32)));
        return new Promise(resolve => { finishRead = resolve; });
      }
      if (command === "cancel_native_thumbnail_request_v1") return false;
      throw new Error(`unexpected command ${command}`);
    },
  }, "asset", 12, controller.signal);
  await flush();
  controller.abort();
  await flush();
  finishRead(frame);
  await assert.rejects(request, /cancellation was not acknowledged/);
}

{
  const channel = disposableChannel();
  const request = requestNativeThumbnail({
    createChannel: () => channel,
    invoke: async () => { throw new Error("owner barrier is already unregistered"); },
  }, "layer", 14, new AbortController().signal);
  await assert.rejects(request, /owner barrier is already unregistered/);
  assert.equal(channel.cleaned, 1, "pre-dispatch rejection disposes the Tauri callback");
}

{
  const malformed = async (message) => {
    const request = requestNativeThumbnail({
      createChannel: fakeChannel,
      invoke: async (command, args) => {
        if (command !== "get_video_layer_thumbnail") throw new Error(`unexpected command ${command}`);
        args.started.onmessage(message);
        return frame;
      },
    }, "layer", 13, new AbortController().signal);
    await assert.rejects(request, /Invalid thumbnail start ticket/);
  };
  await malformed({ ...ticket("asset"), owner: "other" });
  await malformed(ticket("layer", "d".repeat(31)));
  await malformed({ ...ticket("layer"), requestId: "D".repeat(32) });
}

console.log("native thumbnail request protocol: PASS (normal success, exact abort cancellation, stale-result rejection, cancellation failure, malformed/foreign ticket rejection)");

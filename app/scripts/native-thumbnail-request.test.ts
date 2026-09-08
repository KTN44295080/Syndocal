import test from 'node:test';
import assert from 'node:assert/strict';
import { requestNativeThumbnail } from '../src/nativeThumbnailRequest.ts';
import type { ThumbnailLane } from '../src/nativeThumbnailRequest.ts';
import type { FrontendTauriInvoke } from '../src/tauriInvokeCommands.ts';

const flush = () => new Promise(resolve => setImmediate(resolve));
const ticket = (lane: ThumbnailLane = 'layer', id = 'a'.repeat(32)) => ({ schemaVersion: 1, lane, requestId: id });
function fixture(lane: ThumbnailLane = 'layer', cancel: () => Promise<unknown> = async () => true) {
  const calls: { command: string; args: Record<string, unknown> }[] = [];
  const channel = { onmessage(_message: unknown) {} };
  const abort = new AbortController();
  let resolve!: (value: unknown) => void, reject!: (error: unknown) => void;
  let settled = false;
  const native = new Promise((yes, no) => { resolve = yes; reject = no; });
  const invoke = ((command: string, args: Record<string, unknown>) => {
    calls.push({ command, args });
    return command === 'cancel_native_thumbnail_request_v1' ? cancel() : native;
  }) as FrontendTauriInvoke;
  const operation = requestNativeThumbnail({ createChannel: () => channel, invoke }, lane, 7, abort.signal)
    .then(value => ({ value, error: undefined }), error => ({ value: undefined, error }))
    .then(result => { settled = true; return result; });
  return { calls, channel, abort, operation, resolve, reject, settled: () => settled };
}

test('already-aborted requests do not construct a channel or invoke native code', async () => {
  const abort = new AbortController(); abort.abort();
  await assert.rejects(requestNativeThumbnail({
    createChannel: () => assert.fail('unexpected channel'),
    invoke: (() => assert.fail('unexpected invoke')) as FrontendTauriInvoke,
  }, 'layer', 7, abort.signal), { name: 'AbortError' });
});
for (const lane of ['layer', 'asset'] as const) test(`${lane} success uses one native call and detaches late cancellation`, async () => {
  const f = fixture(lane), frame = { pixels: 'fixture' };
  f.channel.onmessage(ticket(lane)); f.resolve(frame);
  assert.deepEqual(await f.operation, { value: frame, error: undefined });
  f.abort.abort(); f.channel.onmessage(ticket(lane)); await flush();
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0].command, lane === 'layer' ? 'get_video_layer_thumbnail' : 'get_media_asset_thumbnail');
});
test('abort before announcement waits for the ticket, cancels once, retains pending native work', async () => {
  const f = fixture(); f.abort.abort(); await flush();
  assert.equal(f.calls.length, 1); assert.equal(f.settled(), false);
  f.channel.onmessage(ticket()); f.channel.onmessage(ticket()); await flush();
  assert.equal(f.calls.length, 2); assert.deepEqual(f.calls[1].args, { ticket: ticket() });
  assert.equal(f.settled(), false, 'cancel acknowledgement is not worker completion');
  f.resolve({ pixels: 'late' });
  assert.equal((await f.operation).error.name, 'AbortError');
});

test('abort after announcement sends the exact ticket and waits for cancel acknowledgement', async () => {
  let acknowledge!: (value: unknown) => void;
  const f = fixture('asset', () => new Promise(resolve => { acknowledge = resolve; }));
  f.channel.onmessage(ticket('asset')); f.abort.abort(); await flush();
  assert.deepEqual(f.calls[1].args, { ticket: ticket('asset') });
  f.resolve({}); await flush(); assert.equal(f.settled(), false);
  acknowledge(false);
  assert.equal((await f.operation).error.name, 'AbortError');
});
for (const ack of [undefined, 'yes', 0]) test(`malformed cancel acknowledgement ${String(ack)} is not success`, async () => {
  const f = fixture('layer', async () => ack);
  f.channel.onmessage(ticket()); f.abort.abort(); await flush();
  assert.equal(f.settled(), false); f.resolve({});
  assert.match((await f.operation).error.message, /not acknowledged/);
});
test('cancel failure is reported only after retaining and settling native work', async () => {
  const f = fixture('layer', async () => { throw undefined; });
  f.channel.onmessage(ticket()); f.abort.abort(); await flush();
  assert.equal(f.settled(), false); f.reject(new Error('native cancelled'));
  assert.match((await f.operation).error.message, /not acknowledged/);
});

for (const message of [null, {}, ticket('asset'), { ...ticket(), schemaVersion: 2 },
  { ...ticket(), requestId: 'bad' }, { ...ticket(), owner: 'other' }]) {
  test(`invalid ticket is never used for cancellation: ${JSON.stringify(message)}`, async () => {
    const f = fixture(); f.abort.abort(); f.channel.onmessage(message); await flush();
    assert.equal(f.calls.length, 1); assert.equal(f.settled(), false);
    f.resolve({}); assert.match((await f.operation).error.message, /Invalid thumbnail start ticket/);
  });
}
test('a changed announcement cannot retarget cancellation', async () => {
  const f = fixture(); f.channel.onmessage(ticket());
  f.channel.onmessage(ticket('layer', 'b'.repeat(32))); f.abort.abort(); await flush();
  assert.deepEqual(f.calls[1].args, { ticket: ticket() });
  f.resolve({}); assert.match((await f.operation).error.message, /Invalid thumbnail start ticket/);
});
test('failed native admission needs no cancellation or hidden retry', async () => {
  const f = fixture(); const error = new Error('busy'); f.reject(error);
  assert.equal((await f.operation).error, error);
  f.abort.abort(); f.channel.onmessage(ticket()); await flush();
  assert.equal(f.calls.length, 1);
});

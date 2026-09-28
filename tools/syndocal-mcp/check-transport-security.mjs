import assert from 'node:assert/strict';
import { fork } from 'node:child_process';
import net from 'node:net';
import http from 'node:http';
import { once } from 'node:events';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import os from 'node:os';

// A separate process lets the parent detect an event-loop starvation regression.
if (process.argv[2] === '--fixture') {
  const { serveHttp } = await import('./server.mjs');
  const server = serveHttp({ descriptor: path.join(os.tmpdir(), 'absent-security-test-descriptor'), executable: process.execPath });
  await once(server, 'listening');
  process.send({ port: server.address().port });
  process.on('disconnect', () => process.exit(0));
} else {
  const child = fork(fileURLToPath(import.meta.url), ['--fixture'], { stdio: ['ignore', 'ignore', 'pipe', 'ipc'], windowsHide: true });
  let raw;
  const deadline = (promise, label, ms = 3000) => {
    let timer;
    return Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`${label} deadline`)), ms);
    })]).finally(() => clearTimeout(timer));
  };
  try {
    const [{ port }] = await deadline(once(child, 'message'), 'fixture start', 10000);
    const base = `http://127.0.0.1:${port}`;
    const request = (pathname, headers = {}, method = 'GET') => fetch(`${base}${pathname}`, {
      method, headers, signal: AbortSignal.timeout(3000),
    });
    for (const headers of [
      { origin: 'https://attacker.invalid' }, { origin: 'null' },
      { origin: `http://127.0.0.1:${port + 1}` },
      { 'sec-fetch-site': 'cross-site' },
    ]) {
      assert.equal((await request('/healthz', headers)).status, 403, JSON.stringify(headers));
    }
    const rebound = await deadline(new Promise((resolve, reject) => {
      const call = http.get(`${base}/healthz`, { headers: { host: `rebind.attacker.invalid:${port}` } }, response => {
        response.resume(); resolve(response.statusCode);
      });
      call.on('error', reject);
    }), 'rebound host');
    assert.equal(rebound, 403);
    assert.equal((await request('/healthz')).status, 200);
    assert.equal((await request('/healthz', { origin: base, 'sec-fetch-site': 'same-origin' })).status, 200);
    assert.equal((await request('/rest/tools/%ZZ', { 'content-type': 'application/json' }, 'POST')).status, 400);
    assert.equal((await request('/healthz')).status, 200, 'Malformed tool path must not crash the adapter');

    const connect = async (origin) => {
      const socket = net.createConnection({ host: '127.0.0.1', port });
      await deadline(once(socket, 'connect'), 'socket connect');
      const response = once(socket, 'data');
      socket.write(`GET /ws HTTP/1.1\r\nHost: 127.0.0.1:${port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n${origin ? `Origin: ${origin}\r\n` : ''}\r\n`);
      try { return { socket, response: (await deadline(response, 'upgrade'))[0].toString() }; }
      catch (error) { socket.destroy(); throw error; }
    };
    const denied = await connect('https://attacker.invalid');
    try { assert.match(denied.response, /^HTTP\/1\.1 403 /); }
    finally { denied.socket.destroy(); }
    const accepted = await connect();
    raw = accepted.socket;
    assert.match(accepted.response, /^HTTP\/1\.1 101 /);
    const payload = Buffer.from(JSON.stringify({ jsonrpc: '2.0', id: 77, method: 'initialize', params: {
      protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'fragment-check', version: '1' },
    } }));
    assert.ok(payload.length < 65536);
    const mask = Buffer.from([1, 2, 3, 4]);
    const masked = Buffer.from(payload);
    for (let index = 0; index < masked.length; index++) masked[index] ^= mask[index % 4];
    const header = Buffer.from([0x81, 0xfe, payload.length >> 8, payload.length & 255]);
    const frame = Buffer.concat([header, mask, masked]);
    raw.write(frame.subarray(0, 1));
    await new Promise(resolve => setTimeout(resolve, 50));
    assert.equal((await request('/healthz')).status, 200, 'Partial header must not starve HTTP');
    raw.write(frame.subarray(1, 10));
    await new Promise(resolve => setTimeout(resolve, 50));
    assert.equal((await request('/healthz')).status, 200, 'Partial payload must not starve HTTP');
    const reply = deadline(new Promise((resolve, reject) => {
      let bytes = Buffer.alloc(0);
      raw.on('data', chunk => {
        bytes = Buffer.concat([bytes, chunk]);
        if (bytes.length < 2) return;
        let size = bytes[1] & 127;
        let offset = 2;
        if (size === 126) { if (bytes.length < 4) return; size = bytes.readUInt16BE(2); offset = 4; }
        if (size === 127 || bytes.length > 65536) { reject(new Error('Unexpected response bound')); return; }
        if (bytes.length >= offset + size) {
          try { resolve(JSON.parse(bytes.subarray(offset, offset + size).toString())); }
          catch (error) { reject(error); }
        }
      });
    }), 'fragmented frame response');
    raw.write(frame.subarray(10));
    const result = await reply;
    assert.equal(result.id, 77);
    assert.equal(result.result.protocolVersion, '2025-11-25');
    console.log('PASS transport security: host/origin rejection, malformed path recovery, split-header/payload liveness and frame completion; no native/device calls');
  } finally {
    raw?.destroy();
    if (child.exitCode === null) {
      const exited = once(child, 'exit');
      child.kill();
      await deadline(exited, 'fixture cleanup');
    }
  }
}

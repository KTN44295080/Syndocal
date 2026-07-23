#!/usr/bin/env node

import dgram from "node:dgram";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const ARTNET_ID = Buffer.from("Art-Net\0", "ascii");
const ART_DMX_OPCODE = 0x5000;
const DEFAULT_ARTNET_PORT = 6454;
const DEFAULT_HTTP_PORT = 6455;
const MAX_TRANSITIONS = 128;

export function parseArtDmx(message) {
  if (!Buffer.isBuffer(message) || message.length < 18) return null;
  if (!message.subarray(0, ARTNET_ID.length).equals(ARTNET_ID)) return null;
  if (message.readUInt16LE(8) !== ART_DMX_OPCODE) return null;

  const length = message.readUInt16BE(16);
  if (length < 2 || length > 512 || message.length !== 18 + length) return null;

  return {
    protocolVersion: message.readUInt16BE(10),
    sequence: message[12],
    physical: message[13],
    universe: message.readUInt16LE(14) & 0x7fff,
    data: Buffer.from(message.subarray(18)),
  };
}

export function buildArtDmxForSelfTest({
  universe = 0,
  sequence = 1,
  values = [255, 127],
} = {}) {
  const data = Buffer.from(values);
  const packet = Buffer.alloc(18 + data.length);
  ARTNET_ID.copy(packet, 0);
  packet.writeUInt16LE(ART_DMX_OPCODE, 8);
  packet.writeUInt16BE(14, 10);
  packet[12] = sequence;
  packet.writeUInt16LE(universe, 14);
  packet.writeUInt16BE(data.length, 16);
  data.copy(packet, 18);
  return packet;
}

function parseArgs(argv) {
  const options = {
    artnetPort: DEFAULT_ARTNET_PORT,
    httpPort: DEFAULT_HTTP_PORT,
    durationSeconds: 0,
    evidencePath: "",
    selfTest: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const nextValue = () => {
      index += 1;
      if (index >= argv.length) throw new Error(`Missing value for ${argument}`);
      return argv[index];
    };
    if (argument === "--artnet-port") options.artnetPort = Number(nextValue());
    else if (argument === "--http-port") options.httpPort = Number(nextValue());
    else if (argument === "--duration-seconds") options.durationSeconds = Number(nextValue());
    else if (argument === "--evidence") options.evidencePath = nextValue();
    else if (argument === "--self-test") options.selfTest = true;
    else throw new Error(`Unknown argument: ${argument}`);
  }

  for (const [label, value] of [
    ["Art-Net port", options.artnetPort],
    ["HTTP port", options.httpPort],
  ]) {
    if (!Number.isInteger(value) || value < 1 || value > 65535) {
      throw new Error(`${label} must be an integer from 1 to 65535`);
    }
  }
  if (!Number.isFinite(options.durationSeconds) || options.durationSeconds < 0) {
    throw new Error("Duration must be zero or a positive number of seconds");
  }
  return options;
}

function frameDigest(data) {
  let digest = 2166136261;
  for (const value of data) {
    digest ^= value;
    digest = Math.imul(digest, 16777619) >>> 0;
  }
  return digest.toString(16).padStart(8, "0");
}

function summarizeFrame(data) {
  let nonZeroChannels = 0;
  let maxValue = 0;
  let firstNonZero = null;
  for (let index = 0; index < data.length; index += 1) {
    const value = data[index];
    if (value > 0) {
      nonZeroChannels += 1;
      maxValue = Math.max(maxValue, value);
      firstNonZero ??= { channel: index + 1, value };
    }
  }
  return { nonZeroChannels, maxValue, firstNonZero };
}

function createState(options) {
  return {
    schema: 1,
    product: "Syndocal external Art-Net monitor",
    startedAt: new Date().toISOString(),
    endedAt: null,
    listen: { address: "0.0.0.0", port: options.artnetPort },
    monitorUrl: `http://127.0.0.1:${options.httpPort}/`,
    totalDatagrams: 0,
    artDmxFrames: 0,
    rejectedDatagrams: 0,
    changedFrames: 0,
    sequenceDiscontinuities: 0,
    maxGapMs: 0,
    firstFrameAt: null,
    lastFrameAt: null,
    sources: {},
    streams: {},
    universes: {},
    transitions: [],
    lastFrame: null,
  };
}

function recordFrame(state, packet, remote, nowMs) {
  const timestamp = new Date(nowMs).toISOString();
  const digest = frameDigest(packet.data);
  const summary = summarizeFrame(packet.data);
  const sourceKey = `${remote.address}:${remote.port}`;
  const universeKey = String(packet.universe);
  const streamKey = `${sourceKey}/u${packet.universe}`;
  const priorStream = state.streams[streamKey];
  const priorUniverse = state.universes[universeKey];
  const gapMs = priorStream ? nowMs - priorStream.lastFrameEpochMs : 0;
  const priorSequence = priorStream?.lastSequence ?? 0;
  const expectedSequence = priorSequence === 255 ? 1 : priorSequence + 1;
  const sequenceDiscontinuity =
    priorSequence !== 0 && packet.sequence !== 0 && packet.sequence !== expectedSequence;

  state.artDmxFrames += 1;
  state.firstFrameAt ??= timestamp;
  state.lastFrameAt = timestamp;
  state.maxGapMs = Math.max(state.maxGapMs, gapMs);
  state.sources[sourceKey] = (state.sources[sourceKey] ?? 0) + 1;
  if (sequenceDiscontinuity) state.sequenceDiscontinuities += 1;

  state.streams[streamKey] = {
    frames: (priorStream?.frames ?? 0) + 1,
    lastSequence: packet.sequence,
    lastFrameAt: timestamp,
    lastFrameEpochMs: nowMs,
  };

  const changed = priorUniverse?.lastDigest !== digest;
  if (changed) {
    state.changedFrames += 1;
    if (state.transitions.length < MAX_TRANSITIONS) {
      state.transitions.push({
        at: timestamp,
        universe: packet.universe,
        sequence: packet.sequence,
        digest,
        ...summary,
      });
    }
  }

  state.universes[universeKey] = {
    frames: (priorUniverse?.frames ?? 0) + 1,
    changedFrames: (priorUniverse?.changedFrames ?? 0) + (changed ? 1 : 0),
    protocolVersion: packet.protocolVersion,
    physical: packet.physical,
    dataLength: packet.data.length,
    lastSequence: packet.sequence,
    lastDigest: digest,
    lastFrameAt: timestamp,
    lastFrameEpochMs: nowMs,
    ...summary,
  };
  state.lastFrame = {
    at: timestamp,
    source: sourceKey,
    universe: packet.universe,
    sequence: packet.sequence,
    digest,
    data: Array.from(packet.data),
    ...summary,
  };
}

function publicState(state) {
  const { lastFrame, streams, universes, ...summary } = state;
  const publicStreams = Object.fromEntries(
    Object.entries(streams).map(([key, value]) => {
      const { lastFrameEpochMs: _epoch, ...rest } = value;
      return [key, rest];
    }),
  );
  const publicUniverses = Object.fromEntries(
    Object.entries(universes).map(([key, value]) => {
      const { lastFrameEpochMs: _epoch, ...rest } = value;
      return [key, rest];
    }),
  );
  return { ...summary, streams: publicStreams, universes: publicUniverses, lastFrame };
}

function renderHtml() {
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Syndocal External Art-Net Monitor</title>
  <style>
    :root { color-scheme: dark; font: 14px/1.4 Inter, system-ui, sans-serif; }
    body { margin: 0; background: #101315; color: #ecf0f2; }
    header { display:flex; gap:24px; align-items:center; padding:16px 20px; background:#181d20; border-bottom:1px solid #394147; }
    h1 { margin:0; font-size:18px; }
    #status { color:#99a5ad; }
    #status.live { color:#57dc87; }
    main { padding:18px 20px; }
    .metrics { display:grid; grid-template-columns:repeat(6,minmax(100px,1fr)); gap:10px; margin-bottom:18px; }
    .metric { background:#181d20; border:1px solid #30383d; border-radius:6px; padding:10px; }
    .metric b { display:block; font-size:20px; color:#f3a44a; }
    .channels { display:grid; grid-template-columns:repeat(64,minmax(4px,1fr)); gap:2px; height:420px; align-items:end; }
    .channel { position:relative; min-height:2px; background:#384148; border-radius:2px 2px 0 0; }
    .channel.active { background:linear-gradient(#ffd166,#e05a37); }
    footer { padding:10px 20px 18px; color:#8e9aa2; }
    @media (max-width:1000px) { .metrics { grid-template-columns:repeat(3,1fr); } .channels { grid-template-columns:repeat(32,1fr); height:520px; } }
  </style>
</head>
<body>
  <header><h1>Syndocal External Art-Net Monitor</h1><span id="status">WAITING FOR ArtDMX</span></header>
  <main>
    <section class="metrics">
      <div class="metric">Frames<b id="frames">0</b></div>
      <div class="metric">Changed<b id="changed">0</b></div>
      <div class="metric">Universe<b id="universe">-</b></div>
      <div class="metric">Sequence<b id="sequence">-</b></div>
      <div class="metric">Non-zero<b id="nonzero">0</b></div>
      <div class="metric">Max gap<b id="gap">0 ms</b></div>
    </section>
    <section class="channels" id="channels"></section>
  </main>
  <footer>512 DMX channels · UDP/Art-Net process boundary · no internal 3D renderer</footer>
  <script>
    const channelRoot = document.querySelector('#channels');
    const bars = Array.from({length:512}, (_, index) => {
      const bar = document.createElement('div');
      bar.className = 'channel';
      bar.title = 'CH ' + (index + 1);
      channelRoot.appendChild(bar);
      return bar;
    });
    async function refresh() {
      try {
        const state = await fetch('/state', {cache:'no-store'}).then(response => response.json());
        const frame = state.lastFrame;
        document.querySelector('#frames').textContent = state.artDmxFrames;
        document.querySelector('#changed').textContent = state.changedFrames;
        document.querySelector('#gap').textContent = state.maxGapMs + ' ms';
        document.querySelector('#status').className = frame ? 'live' : '';
        document.querySelector('#status').textContent = frame ? 'LIVE ArtDMX · ' + frame.source : 'WAITING FOR ArtDMX';
        document.querySelector('#universe').textContent = frame?.universe ?? '-';
        document.querySelector('#sequence').textContent = frame?.sequence ?? '-';
        document.querySelector('#nonzero').textContent = frame?.nonZeroChannels ?? 0;
        bars.forEach((bar, index) => {
          const value = frame?.data?.[index] ?? 0;
          bar.style.height = Math.max(2, value / 255 * 100) + '%';
          bar.classList.toggle('active', value > 0);
          bar.title = 'CH ' + (index + 1) + ': ' + value;
        });
      } catch { document.querySelector('#status').textContent = 'MONITOR DISCONNECTED'; }
    }
    setInterval(refresh, 100);
    refresh();
  </script>
</body>
</html>`;
}

function writeEvidence(evidencePath, state) {
  if (!evidencePath) return;
  fs.mkdirSync(path.dirname(path.resolve(evidencePath)), { recursive: true });
  fs.writeFileSync(evidencePath, `${JSON.stringify(publicState(state), null, 2)}\n`, "utf8");
}

async function runSelfTest() {
  const parsed = parseArtDmx(buildArtDmxForSelfTest({ universe: 42, sequence: 7 }));
  if (!parsed || parsed.universe !== 42 || parsed.sequence !== 7 || parsed.data[0] !== 255) {
    throw new Error("ArtDMX parser self-test failed");
  }
  if (parseArtDmx(Buffer.from("not Art-Net")) !== null) {
    throw new Error("Non-Art-Net datagram was accepted");
  }
  const state = createState({ artnetPort: 6454, httpPort: 6455 });
  const sourceA = { address: "127.0.0.1", port: 50001 };
  const sourceB = { address: "127.0.0.1", port: 50002 };
  recordFrame(state, parseArtDmx(buildArtDmxForSelfTest({ sequence: 10 })), sourceA, 1000);
  recordFrame(state, parseArtDmx(buildArtDmxForSelfTest({ sequence: 11 })), sourceA, 1023);
  recordFrame(state, parseArtDmx(buildArtDmxForSelfTest({ sequence: 90 })), sourceB, 1024);
  if (state.sequenceDiscontinuities !== 0 || Object.keys(state.streams).length !== 2) {
    throw new Error("Per-source ArtDMX sequence tracking self-test failed");
  }
  console.log("external Art-Net monitor self-test: 3 assertions passed");
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.selfTest) {
    await runSelfTest();
    return;
  }

  const state = createState(options);
  const socket = dgram.createSocket("udp4");
  const server = http.createServer((request, response) => {
    response.setHeader("Cache-Control", "no-store");
    if (request.url === "/state") {
      response.setHeader("Content-Type", "application/json; charset=utf-8");
      response.end(JSON.stringify(publicState(state)));
      return;
    }
    response.setHeader("Content-Type", "text/html; charset=utf-8");
    response.end(renderHtml());
  });

  socket.on("message", (message, remote) => {
    state.totalDatagrams += 1;
    const packet = parseArtDmx(message);
    if (!packet) {
      state.rejectedDatagrams += 1;
      return;
    }
    recordFrame(state, packet, remote, Date.now());
  });

  await new Promise((resolve, reject) => {
    socket.once("error", reject);
    socket.bind(options.artnetPort, "0.0.0.0", resolve);
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(options.httpPort, "127.0.0.1", resolve);
  });

  console.log(
    `external Art-Net monitor ready udp=0.0.0.0:${options.artnetPort} web=http://127.0.0.1:${options.httpPort}/`,
  );

  let finished = false;
  const finish = async (reason) => {
    if (finished) return;
    finished = true;
    state.endedAt = new Date().toISOString();
    state.reason = reason;
    writeEvidence(options.evidencePath, state);
    await Promise.all([
      new Promise((resolve) => socket.close(resolve)),
      new Promise((resolve) => server.close(resolve)),
    ]);
    console.log(
      `external Art-Net monitor stopped frames=${state.artDmxFrames} changed=${state.changedFrames} rejected=${state.rejectedDatagrams}`,
    );
  };

  process.once("SIGINT", () => void finish("SIGINT"));
  process.once("SIGTERM", () => void finish("SIGTERM"));
  if (options.durationSeconds > 0) {
    setTimeout(() => void finish("duration"), options.durationSeconds * 1000);
  }
}

const invokedDirectly =
  process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;
if (invokedDirectly) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exitCode = 1;
  });
}

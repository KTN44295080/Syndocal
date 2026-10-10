// Software-only macOS gate: exact production libproc module, temporary CLI peer,
// and a fake authenticated loopback broker. No Tauri window or physical output.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import { createServer } from 'node:net';
import { randomBytes, randomUUID, createHmac } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { nativeRequest, readDescriptor } from './server.mjs';
import { processExecutable } from './process-identity.mjs';

assert.equal(process.platform, 'darwin');
const run = promisify(execFile);
const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-process-identity-'));
let peer; let broker;
try {
  const root = fileURLToPath(new URL('../../', import.meta.url));
  const lock = await fs.readFile(path.join(root, 'Cargo.lock'), 'utf8');
  const version = lock.match(/name = "serde_json"\r?\nversion = "([^"]+)"/)[1];
  await fs.mkdir(path.join(temporary, 'src'));
  await fs.writeFile(path.join(temporary, 'Cargo.toml'), `[package]\nname="syndocal-process-identity-fixture"\nversion="0.0.0"\nedition="2024"\n[dependencies]\nserde_json="=${version}"\n`);
  await fs.writeFile(path.join(temporary, 'src/main.rs'), `#[path=${JSON.stringify(path.join(root, 'app/src-tauri/src/process_identity_cli.rs'))}] mod process_identity_cli;
fn main(){if std::env::args().nth(1).as_deref()==Some("--fixture-hold"){println!("ready");loop{std::thread::park();}}
if !process_identity_cli::requested(std::env::args_os().skip(1)){std::process::exit(2);}
if let Err(error)=process_identity_cli::run(std::env::args_os().skip(1)){eprintln!("{error}");std::process::exit(1);}}
`);
  const options = { cwd: temporary, env: { ...process.env, CARGO_TARGET_DIR: path.join(temporary, 'target'), RUSTFLAGS: '-Dwarnings' }, maxBuffer: 1024 * 1024, timeout: 180000 };
  const tests = await run('cargo', ['test', '--', '--test-threads=1'], options);
  process.stdout.write(tests.stdout); process.stdout.write(tests.stderr);
  await run('cargo', ['build', '--locked'], options);
  const executable = path.join(temporary, 'target/debug/syndocal-process-identity-fixture');
  peer = spawn(executable, ['--fixture-hold'], { stdio: ['ignore', 'pipe', 'pipe'] });
  await new Promise((resolve, reject) => { peer.stdout.once('data', resolve); peer.once('error', reject); peer.once('exit', () => reject(new Error('fixture exited before ready'))); });
  assert.equal(await fs.realpath(await processExecutable(peer.pid, executable)), await fs.realpath(executable));
  await assert.rejects(run(executable, ['--syndocal-process-path-v1', '0']));
  await assert.rejects(run(executable, ['--syndocal-process-path-v1', String(peer.pid), 'extra']));
  const token = randomBytes(32).toString('hex'); const credential = randomBytes(32).toString('hex'); const sessionNonce = randomBytes(32).toString('hex');
  let forwarded = 0;
  broker = createServer(socket => {
    let text = '';
    socket.on('data', chunk => {
      text += chunk; if (!text.includes('\n')) return;
      const request = JSON.parse(text.slice(0, text.indexOf('\n'))); forwarded++;
      assert.equal(request.token, token); assert.equal(request.auth.principalId, 'identity-test');
      assert.equal(request.auth.proof, createHmac('sha256', Buffer.from(credential, 'hex')).update(`${sessionNonce}\0${request.auth.clientNonce}\0${request.requestId}\0${request.method}`).digest('hex'));
      socket.end(JSON.stringify({ requestId: request.requestId, status: 'completed', result: { ok: true } }) + '\n');
    });
  });
  await new Promise(resolve => broker.listen(0, '127.0.0.1', resolve));
  const descriptorPath = path.join(temporary, 'descriptor.json'); const credentialFile = path.join(temporary, 'credential');
  await fs.writeFile(credentialFile, credential, { mode: 0o600 });
  const descriptor = { protocolVersion: 1, port: broker.address().port, token, sessionNonce, instanceId: randomBytes(16).toString('hex'), processId: peer.pid, executablePath: executable };
  await fs.writeFile(descriptorPath, JSON.stringify(descriptor));
  const requestOptions = { descriptor: descriptorPath, executable, principalId: 'identity-test', principalIncarnation: 1, credentialFile };
  assert.deepEqual(await readDescriptor(requestOptions), descriptor);
  assert.equal((await nativeRequest(requestOptions, 'runtime.get', {}, randomUUID())).status, 'completed');
  assert.equal(forwarded, 1);
  await fs.writeFile(descriptorPath, JSON.stringify({ ...descriptor, processId: process.pid }));
  assert.equal((await nativeRequest(requestOptions, 'runtime.get', {}, randomUUID())).status, 'rejected');
  assert.equal(forwarded, 1, 'different live executable never reaches broker');
  await fs.writeFile(descriptorPath, JSON.stringify(descriptor));
  peer.kill('SIGTERM'); await new Promise(resolve => peer.once('exit', resolve));
  assert.equal((await nativeRequest(requestOptions, 'runtime.get', {}, randomUUID())).status, 'rejected');
  assert.equal(forwarded, 1, 'reaped peer never reaches broker');
  console.log('PASS macOS libproc identity, malformed CLI, exact-PID/path, authenticated forwarding and retired/mismatched process rejection; software fixture only.');
} finally {
  if (peer?.exitCode === null && peer?.signalCode === null) { peer.kill('SIGTERM'); await new Promise(resolve => peer.once('exit', resolve)); }
  if (broker) await new Promise(resolve => broker.close(resolve));
  await fs.rm(temporary, { recursive: true, force: true });
}

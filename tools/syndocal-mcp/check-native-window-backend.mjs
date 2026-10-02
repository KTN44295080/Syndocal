// Native window observation and unauthenticated broker rejection. No UI actions.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import net from 'node:net';
import { createHash, randomUUID } from 'node:crypto';
import { parseOptions, readDescriptor } from './server.mjs';

const args = process.argv.slice(2);
const take = name => {
  const index = args.indexOf(name);
  assert.ok(index >= 0 && index + 1 < args.length, `Required: ${name}`);
  return args.splice(index, 2)[1];
};
const executable = take('--expected-executable');
const evidence = take('--evidence');
assert.equal(args.length, 0);
assert.equal(process.platform, 'win32');
assert.ok(path.isAbsolute(executable) && path.isAbsolute(evidence));
assert.equal(await fs.stat(evidence).then(() => true, error => {
  if (error.code === 'ENOENT') return false;
  throw error;
}), false);
const options = parseOptions(['--expected-executable', executable]);
const descriptor = await readDescriptor(options);
{
  const { stdout } = await promisify(execFile)('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `
Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class NativeWindowProof { [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr hWnd); [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd); }'
$matching = @(Get-Process syndocal -ErrorAction Stop | Where-Object { $_.Path -eq $env:SYNDOCAL_WINDOW_EXPECTED_EXE -and $_.MainWindowHandle -ne 0 })
if ($matching.Count -ne 1 -or $matching[0].Id -ne ${descriptor.processId} -or !$matching[0].Responding -or $matching[0].MainWindowTitle -ne 'Syndocal' -or ![NativeWindowProof]::IsWindowVisible($matching[0].MainWindowHandle)) { throw 'Native current-checkout window gate failed' }
[pscustomobject]@{ processId=$matching[0].Id; title=$matching[0].MainWindowTitle; responsive=$matching[0].Responding; visible=$true; maximized=[NativeWindowProof]::IsZoomed($matching[0].MainWindowHandle); exactPathWindowCount=$matching.Count } | ConvertTo-Json -Compress
`], { windowsHide: true, timeout: 10000, maxBuffer: 8192,
    env: { ...process.env, SYNDOCAL_WINDOW_EXPECTED_EXE: await fs.realpath(executable) } });
  const window = JSON.parse(stdout.trim());
  assert.equal(window.maximized, true, 'The exact intended native window must be maximized');
  const requestId = randomUUID();
  const denied = await new Promise((resolve, reject) => {
    const socket = net.createConnection({ host: '127.0.0.1', port: descriptor.port });
    let buffer = '';
    const finish = (error, value) => { clearTimeout(timer); socket.destroy(); error ? reject(error) : resolve(value); };
    const timer = setTimeout(() => finish(new Error('Unauthenticated native probe deadline')), 3000);
    socket.on('connect', () => socket.write(`${JSON.stringify({ token: descriptor.token, requestId, method: 'fixtures.list', params: {} })}\n`));
    socket.on('error', () => finish(new Error('Unauthenticated native connection failed')));
    socket.on('data', chunk => {
      buffer += chunk.toString();
      if (Buffer.byteLength(buffer) > 256 * 1024) { finish(new Error('Native response bound')); return; }
      if (buffer.includes('\n')) {
        try { finish(undefined, JSON.parse(buffer.split('\n')[0])); }
        catch { finish(new Error('Invalid native response')); }
      }
    });
  });
  assert.equal(denied.requestId, requestId);
  assert.equal(denied.status, 'rejected');
  assert.equal(denied.error, 'agent_authentication_required');
  assert.equal(denied.result, undefined);
  assert.equal((await readDescriptor(options)).instanceId, descriptor.instanceId);
  await fs.writeFile(evidence, `${JSON.stringify({ schemaVersion: 1, timestamp: new Date().toISOString(), passed: true,
    executable: await fs.realpath(executable),
    executableSha256: createHash('sha256').update(await fs.readFile(executable)).digest('hex'),
    runnerSha256: createHash('sha256').update(await fs.readFile(new URL('./check-native-window-backend.mjs', import.meta.url))).digest('hex'),
    sidecarSha256: createHash('sha256').update(await fs.readFile(new URL('./server.mjs', import.meta.url))).digest('hex'),
    window, unauthenticatedReadRejected: true, authenticationError: denied.error,
    nonclaims: ['No UI actions or Control layout measurement', 'No authenticated primary-profile MCP acceptance', 'No physical output, clean-install, distribution or venue acceptance'],
  }, null, 2)}\n`, { flag: 'wx' });
  console.log(`PASS native current-checkout: one visible responsive Syndocal window; maximized=${window.maximized}; unauthenticated read rejected.`);
}

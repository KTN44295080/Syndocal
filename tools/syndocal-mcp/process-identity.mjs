import fs from 'node:fs/promises';
import path from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execute = promisify(execFile);
export function canonicalExecutable(value, platform = process.platform) {
  return platform === 'win32' ? value.toLowerCase() : value;
}

export function parseMacosProcessPath(stdout, pid) {
  const bytes = Buffer.isBuffer(stdout) ? stdout : Buffer.from(stdout);
  if (bytes.length > 8192) throw new Error('process path response too large');
  const value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  if (!value || Array.isArray(value) || Object.keys(value).sort().join(',') !== 'executablePath,processId,schemaVersion'
      || value.schemaVersion !== 1 || value.processId !== pid || typeof value.executablePath !== 'string'
      || !path.posix.isAbsolute(value.executablePath) || value.executablePath.includes('\0')
      || Buffer.byteLength(value.executablePath) >= 4096) throw new Error('process path response invalid');
  return value.executablePath;
}

export async function processExecutable(pid, expectedExecutable, { platform = process.platform, run = execute, readlink = fs.readlink } = {}) {
  if (!Number.isSafeInteger(pid) || pid < 1 || pid > 2147483647) throw new Error('process PID invalid');
  if (platform === 'win32') {
    const shell = path.join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe');
    const { stdout } = await run(shell, ['-NoProfile', '-NonInteractive', '-Command',
      '[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new(); $p=Get-Process -Id ([int]$env:SYNDOCAL_BRIDGE_PID) -ErrorAction Stop; if (!$p -or !$p.Path) { exit 2 }; [Console]::Write($p.Path)'],
    { windowsHide: true, timeout: 10000, maxBuffer: 8192, env: { ...process.env, SYNDOCAL_BRIDGE_PID: String(pid) } });
    return stdout.trim();
  }
  if (platform === 'darwin') {
    // Only the caller-selected executable is launched. Its early read-only CLI
    // uses libproc, before Engine/Tauri initialization, not argv/process labels.
    const { stdout } = await run(expectedExecutable, ['--syndocal-process-path-v1', String(pid)],
      { windowsHide: true, timeout: 10000, maxBuffer: 8192, encoding: 'buffer' });
    return parseMacosProcessPath(stdout, pid);
  }
  if (platform === 'linux') return readlink(`/proc/${pid}/exe`);
  throw new Error('process identity inspection unavailable on this platform');
}

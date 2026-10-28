// Tauri CLI 2.5.0 NSIS packaging only. Never invokes Cargo or Tauri.
// https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.5.0/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const self = fileURLToPath(import.meta.url);
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const requireThat = (condition, message) => { if (!condition) throw new Error(message); };

// Authenticode changes checksum/security directory and appends a certificate table.
// Comparing the remaining image proves signed bytes came from the captured input.
export function imageHash(input) {
  const bytes = Buffer.from(input);
  requireThat(bytes.length >= 64 && bytes.toString('ascii', 0, 2) === 'MZ', 'Invalid DOS header');
  const pe = bytes.readUInt32LE(60);
  requireThat(pe + 24 + 144 <= bytes.length && bytes.toString('ascii', pe, pe + 4) === 'PE\0\0', 'Invalid PE header');
  const opt = pe + 24;
  const magic = bytes.readUInt16LE(opt);
  requireThat(magic === 0x10b || magic === 0x20b, 'Unsupported PE image');
  const directory = opt + (magic === 0x20b ? 112 : 96) + 32;
  const certificate = bytes.readUInt32LE(directory);
  const size = bytes.readUInt32LE(directory + 4);
  requireThat((certificate === 0 && size === 0) || (size > 0 && certificate >= directory + 8 && certificate + size === bytes.length), 'Invalid or non-terminal certificate table');
  bytes.fill(0, opt + 64, opt + 68);
  bytes.fill(0, directory, directory + 8);
  // Authenticode may pad the original image to eight bytes before its table.
  // Normalize only that alignment, preserving every non-padding image byte.
  const image = bytes.subarray(0, certificate || bytes.length);
  return hash(Buffer.concat([image, Buffer.alloc((8 - image.length % 8) % 8)]));
}

function quote(value) {
  requireThat(!/["'\r\n$]/.test(value), 'Unsafe NSIS command path');
  return `"${value}"`;
}
function replaceOnce(text, pattern, replacement, label) {
  requireThat([...text.matchAll(new RegExp(pattern.source, 'gm'))].length === 1, `Expected exactly one ${label}`);
  return text.replace(pattern, replacement);
}
export function patchScript(text, { work, mode, payload, output }) {
  requireThat(text.includes("!uninstfinalize '${UNINSTALLERSIGNCOMMAND}'"), 'Missing pinned Tauri uninstaller hook');
  requireThat((text.match(/^\s*WriteUninstaller "\$INSTDIR\\uninstall\.exe"\s*$/gm) || []).length === 1, 'Unexpected uninstaller generation');
  let patched = replaceOnce(text, /^!define MAINBINARYSRCPATH ".*"$/m,
    () => `!define MAINBINARYSRCPATH ${quote(path.join(payload, 'syndocal.exe'))}`, 'main binary definition');
  patched = replaceOnce(patched, /^!define OUTFILE ".*"$/m, () => `!define OUTFILE ${quote(output)}`, 'output definition');
  const command = `${quote(process.execPath)} ${quote(self)} hook ${mode} ${quote(work)} ${quote(payload)} "%1"`;
  patched = replaceOnce(patched, /^!define UNINSTALLERSIGNCOMMAND ".*"$/m,
    () => `!define UNINSTALLERSIGNCOMMAND \`${command}\``, 'uninstaller signing definition');
  patched = replaceOnce(patched, /^!if "\$\{UNINSTALLERSIGNCOMMAND\}" != ""$/m,
    '!if 1', 'uninstaller hook guard');
  patched = replaceOnce(patched, /^\s*!uninstfinalize '\$\{UNINSTALLERSIGNCOMMAND\}'\s*$/m,
    "  !uninstfinalize '${UNINSTALLERSIGNCOMMAND}' = 0", 'checked uninstaller finalizer');
  return patched;
}

export function compile(work, mode, payload, output) {
  const receipt = JSON.parse(fs.readFileSync(path.join(work, 'receipt.json'), 'utf8'));
  requireThat(hash(fs.readFileSync(receipt.makensis)) === receipt.makensisSha256, 'NSIS compiler changed since capture');
  const source = fs.readFileSync(receipt.script, 'utf8');
  requireThat(hash(source) === receipt.scriptSha256, 'Generated NSIS script changed since capture');
  for (const resource of receipt.resources) requireThat(hash(fs.readFileSync(resource.path)) === resource.sha256, `NSIS resource changed: ${resource.path}`);
  const patched = patchScript(source, { work, mode, payload, output });
  const script = path.join(work, 'nsis', `installer-${mode}.nsi`);
  fs.writeFileSync(script, '\uFEFF' + patched);
  const run = spawnSync(receipt.makensis, ['/V3', script], { cwd: path.dirname(script), stdio: 'inherit', windowsHide: true });
  requireThat(!run.error && run.status === 0, `makensis failed (${run.error?.message || run.status})`);
  requireThat(fs.existsSync(output), 'makensis did not produce installer');
}

function hook(mode, work, payload, generated) {
  const source = fs.readFileSync(generated);
  const captured = path.join(work, 'uninstaller-image.sha256');
  if (mode === 'capture') {
    fs.copyFileSync(generated, path.join(payload, 'uninstall.exe'));
    fs.writeFileSync(captured, imageHash(source));
  } else {
    requireThat(mode === 'replace', 'Unknown uninstaller hook mode');
    const expected = fs.readFileSync(captured, 'utf8');
    const signed = fs.readFileSync(path.join(payload, 'uninstall.exe'));
    requireThat(imageHash(source) === expected, 'Regenerated uninstaller image changed');
    requireThat(imageHash(signed) === expected, 'Signed uninstaller does not match captured input');
    fs.copyFileSync(path.join(payload, 'uninstall.exe'), generated);
  }
}

function options(args) {
  const result = {};
  for (let i = 0; i < args.length; i += 2) {
    requireThat(args[i]?.startsWith('--') && args[i + 1], 'Expected --name value arguments');
    const key = args[i].slice(2);
    result[key] = key === 'certificate-thumbprint' ? args[i + 1] : path.resolve(args[i + 1]);
  }
  return result;
}

export function prepare({ script, makensis, work }) {
  requireThat(script && makensis && work, 'prepare requires --script --makensis --work');
  requireThat(!fs.existsSync(work), 'Work directory must not already exist');
  const relativeWork = path.relative(path.dirname(script), work);
  requireThat(relativeWork.startsWith('..') || path.isAbsolute(relativeWork), 'Work directory must be outside generated NSIS directory');
  const text = fs.readFileSync(script, 'utf8');
  const main = /^!define MAINBINARYSRCPATH "([^"]+)"$/m.exec(text)?.[1];
  requireThat(main && fs.existsSync(main), 'Missing generated main binary');
  // Snapshot every literal File resource; upstream files stay in place and must
  // remain byte-identical until the second packaging pass.
  const resources = [...text.matchAll(/^\s*File\s+.*"([^"\r\n]+)"\s*$/gm)]
    .map(match => match[1]).filter(name => !name.includes('$'))
    .map(name => ({ path: path.resolve(path.dirname(script), name), sha256: hash(fs.readFileSync(path.resolve(path.dirname(script), name))) }));
  // Guard externally included project hooks and explicit WebView redistributables.
  const extraInputs = [...text.matchAll(/^!include "([^"\r\n]+)"/gm)].map(match => match[1])
    .filter(name => !name.includes('$'))
    .map(name => path.resolve(path.dirname(script), name)).filter(name => fs.existsSync(name));
  for (const macro of ['WEBVIEW2BOOTSTRAPPERPATH', 'WEBVIEW2INSTALLERPATH']) {
    const value = new RegExp(`^!define ${macro} "([^"\\r\\n]+)"$`, 'm').exec(text)?.[1];
    if (value) extraInputs.push(path.resolve(path.dirname(script), value));
  }
  for (const name of extraInputs) resources.push({ path: name, sha256: hash(fs.readFileSync(name)) });
  fs.mkdirSync(path.join(work, 'payload'), { recursive: true });
  fs.cpSync(path.dirname(script), path.join(work, 'nsis'), { recursive: true });
  for (const name of fs.readdirSync(path.join(work, 'nsis'))) {
    if (name.endsWith('.nsh')) {
      const include = path.join(work, 'nsis', name);
      resources.push({ path: include, sha256: hash(fs.readFileSync(include)) });
    }
  }
  fs.copyFileSync(main, path.join(work, 'payload', 'syndocal.exe'));
  fs.writeFileSync(path.join(work, 'receipt.json'), JSON.stringify({ schemaVersion: 1, script, scriptSha256: hash(text), makensis, makensisSha256: hash(fs.readFileSync(makensis)), resources, mainImageSha256: imageHash(fs.readFileSync(main)) }, null, 2));
  compile(work, 'capture', path.join(work, 'payload'), path.join(work, 'unsigned-installer.exe'));
  requireThat(fs.existsSync(path.join(work, 'payload', 'uninstall.exe')), 'Uninstaller capture hook did not run');
}

export function packageSigned({ work, 'signed-payload': payload, output, 'certificate-thumbprint': thumbprint }) {
  requireThat(work && payload && output, 'package requires --work --signed-payload --output');
  requireThat(!fs.existsSync(output), 'Output must not already exist');
  requireThat(typeof thumbprint === 'string' && /^[A-Fa-f0-9]{40}$/.test(thumbprint), 'package requires --certificate-thumbprint (40 hex characters)');
  const receipt = JSON.parse(fs.readFileSync(path.join(work, 'receipt.json'), 'utf8'));
  requireThat(imageHash(fs.readFileSync(path.join(payload, 'syndocal.exe'))) === receipt.mainImageSha256, 'Signed main binary does not match captured input');
  for (const name of ['syndocal.exe', 'uninstall.exe']) {
    const file = path.join(payload, name);
    const run = spawnSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', '$s=Get-AuthenticodeSignature -LiteralPath $env:SIGNPATH_VERIFY_FILE; if($s.Status -ne "Valid" -or $s.SignerCertificate.Subject -notmatch "(^|,\\s*)CN=SignPath Foundation(,|$)" -or $s.SignerCertificate.Thumbprint -ne $env:SIGNPATH_VERIFY_THUMBPRINT -or $null -eq $s.TimeStamperCertificate){throw "Expected timestamped valid SignPath Foundation signature with configured certificate thumbprint"}'],
      { env: { ...process.env, SIGNPATH_VERIFY_FILE: file, SIGNPATH_VERIFY_THUMBPRINT: thumbprint }, stdio: 'inherit', windowsHide: true });
    requireThat(!run.error && run.status === 0, `Invalid payload signature: ${name}`);
  }
  fs.mkdirSync(path.dirname(output), { recursive: true });
  compile(work, 'replace', payload, output);
}

if (process.argv[1] && path.resolve(process.argv[1]) === self) {
  try {
    const [command, ...args] = process.argv.slice(2);
    if (command === 'hook') hook(...args);
    else if (command === 'prepare') prepare(options(args));
    else if (command === 'package') packageSigned(options(args));
    else throw new Error('Expected prepare or package command');
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}

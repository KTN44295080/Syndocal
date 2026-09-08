import * as fs from 'node:fs/promises';
import path from 'node:path';
import { parseLoadCommands, requireCondition, validateInfo, validateMachO } from './macos-artifact-policy.mjs';

export async function readPlist(file, run, options = {}) {
  return JSON.parse((await run('/usr/bin/plutil', ['-convert', 'json', '-o', '-', file], options)).stdout);
}
export async function inspectInfo(app, expected, run) {
  const info = await readPlist(path.join(app, 'Contents', 'Info.plist'), run);
  validateInfo(info, expected);
  const executable = path.join(app, 'Contents', 'MacOS', 'syndocal');
  const stat = await fs.lstat(executable);
  requireCondition(stat.isFile() && (stat.mode & 0o111) !== 0, 'Missing/non-executable bundle entry point');
  return { version: info.CFBundleShortVersionString, buildVersion: info.CFBundleVersion,
    identifier: info.CFBundleIdentifier, minimum: info.LSMinimumSystemVersion };
}
export async function inspectMachO(app, minimum, run) {
  const images = [], pending = [app];
  let entries = 0;
  const magic = new Set([0xfeedface, 0xfeedfacf, 0xcefaedfe, 0xcffaedfe,
    0xcafebabe, 0xbebafeca, 0xcafebabf, 0xbfbafeca]);
  while (pending.length) {
    const directory = pending.pop();
    for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
      requireCondition(++entries <= 4096, 'Bundle entry limit exceeded');
      const file = path.join(directory, entry.name);
      requireCondition(!entry.isSymbolicLink(), `Bundle symlink needs explicit containment review: ${file}`);
      if (entry.isDirectory()) { pending.push(file); continue; }
      requireCondition(entry.isFile(), `Unexpected bundle entry: ${file}`);
      const handle = await fs.open(file, 'r');
      let header;
      try {
        const buffer = Buffer.alloc(4);
        const { bytesRead } = await handle.read(buffer, 0, 4, 0);
        header = bytesRead === 4 ? buffer.readUInt32BE(0) : 0;
      } finally { await handle.close(); }
      if (!magic.has(header)) continue;
      requireCondition(images.length < 64, 'Mach-O image limit exceeded');
      const architectures = (await run('/usr/bin/lipo', ['-archs', file])).stdout.trim().split(/\s+/);
      const commands = parseLoadCommands((await run('/usr/bin/otool', ['-l', file])).stdout);
      images.push({ path: path.relative(app, file).split(path.sep).join('/'), architectures, ...commands });
    }
  }
  return { images, dependencies: validateMachO(images, 'Contents/MacOS/syndocal', minimum) };
}
export async function inspectSignature(app, run) {
  await run('/usr/bin/codesign', ['--verify', '--deep', '--strict', '--verbose=2', app]);
  const details = await run('/usr/bin/codesign', ['-d', '--verbose=4', app]);
  return { integrity: 'pass', kind: /^Signature=adhoc\s*$/m.test(details.stderr) ? 'ad-hoc' : 'other',
    developerIdTrust: 'not_verified', notarization: 'not_verified', gatekeeper: 'not_verified' };
}

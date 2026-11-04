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
// Only the standard versioned-framework links (Headers, Resources, the binary,
// Versions/Current) are accepted: relative, inside their own .framework, and
// never followed by the walk, which inspects the real Versions/<v> files.
export async function requireFrameworkSymlink(file) {
  const parts = file.split(path.sep);
  const index = parts.findLastIndex((part) => part.endsWith('.framework'));
  requireCondition(index >= 0 && index < parts.length - 1,
    `Bundle symlink outside a framework needs explicit containment review: ${file}`);
  const framework = parts.slice(0, index + 1).join(path.sep);
  const target = await fs.readlink(file);
  requireCondition(!path.isAbsolute(target), `Absolute framework symlink: ${file}`);
  const resolved = path.resolve(path.dirname(file), target);
  requireCondition(resolved.startsWith(framework + path.sep),
    `Framework symlink escapes its framework: ${file}`);
  await fs.lstat(resolved).catch(() => requireCondition(false, `Dangling framework symlink: ${file}`));
  return resolved;
}
export async function inspectMachO(app, minimum, run) {
  const images = [], pending = [app], links = new Map();
  const relative = (file) => path.relative(app, file).split(path.sep).join('/');
  let entries = 0;
  const magic = new Set([0xfeedface, 0xfeedfacf, 0xcefaedfe, 0xcffaedfe,
    0xcafebabe, 0xbebafeca, 0xcafebabf, 0xbfbafeca]);
  while (pending.length) {
    const directory = pending.pop();
    for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
      requireCondition(++entries <= 4096, 'Bundle entry limit exceeded');
      const file = path.join(directory, entry.name);
      if (entry.isSymbolicLink()) {
        links.set(relative(file), relative(await requireFrameworkSymlink(file)));
        continue;
      }
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
      // A universal image lists every slice's load commands; the installer
      // only loads arm64, so inspect that slice alone.
      const slice = architectures.length > 1 ? ['-arch', 'arm64'] : [];
      const commands = parseLoadCommands((await run('/usr/bin/otool', [...slice, '-l', file])).stdout);
      images.push({ path: path.relative(app, file).split(path.sep).join('/'), architectures, ...commands });
    }
  }
  return { images, links: Object.fromEntries(links),
    dependencies: validateMachO(images, 'Contents/MacOS/syndocal', minimum, links) };
}
export async function inspectSignature(app, run) {
  await run('/usr/bin/codesign', ['--verify', '--deep', '--strict', '--verbose=2', app]);
  const details = await run('/usr/bin/codesign', ['-d', '--verbose=4', app]);
  return { integrity: 'pass', kind: /^Signature=adhoc\s*$/m.test(details.stderr) ? 'ad-hoc' : 'other',
    developerIdTrust: 'not_verified', notarization: 'not_verified', gatekeeper: 'not_verified' };
}

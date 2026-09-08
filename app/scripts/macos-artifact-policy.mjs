import path from 'node:path';
const p = path.posix;

export const ARTIFACT_CHECKS = Object.freeze(['environment', 'dmg', 'image_integrity',
  'extraction', 'bundle_info', 'macho', 'signature', 'process_survival', 'cleanup', 'checksum']);

export function requireCondition(ok, message) {
  if (!ok) throw new Error(message);
}
export function selectDmg(names, version) {
  const candidates = names.filter(name => name.endsWith('.dmg'));
  requireCondition(candidates.length === 1, `Expected exactly one DMG; found ${candidates.length}`);
  requireCondition(candidates[0] === `Syndocal_${version}_arm64.dmg`, 'Unexpected DMG version/architecture/name');
  return candidates[0];
}
export function compareVersions(a, b) {
  const parse = value => {
    requireCondition(typeof value === 'string' && /^\d+(?:\.\d+){0,2}$/.test(value), `Invalid OS version: ${value}`);
    return value.split('.').map(Number);
  };
  const x = parse(a), y = parse(b);
  for (let i = 0; i < 3; i++) {
    if ((x[i] ?? 0) !== (y[i] ?? 0)) return Math.sign((x[i] ?? 0) - (y[i] ?? 0));
  }
  return 0;
}
export function validateInfo(info, expected) {
  requireCondition(info.CFBundleExecutable === 'syndocal', 'Unexpected bundle executable');
  requireCondition(info.CFBundleIdentifier === expected.identifier, 'Bundle identifier mismatch');
  requireCondition(info.CFBundleShortVersionString === expected.version, 'Bundle version mismatch');
  requireCondition(compareVersions(info.LSMinimumSystemVersion, expected.minimum) === 0, 'Info.plist minimum OS mismatch');
}
export function parseLoadCommands(text) {
  const blocks = text.split(/^Load command \d+\r?$/m).slice(1);
  requireCondition(blocks.length > 0, 'No readable Mach-O load commands');
  const result = { dependencies: [], rpaths: [], minimum: null };
  for (const block of blocks) {
    const cmd = block.match(/^\s*cmd (LC_\w+)\s*$/m)?.[1];
    requireCondition(cmd, 'Unparseable load command');
    if (/^LC_(LOAD|LOAD_WEAK|REEXPORT|LOAD_UPWARD|LAZY_LOAD)_DYLIB$/.test(cmd)) {
      const name = block.match(/^\s*name (.+) \(offset \d+\)\s*$/m)?.[1];
      requireCondition(name, `Unparseable ${cmd}`);
      result.dependencies.push(name);
    } else if (cmd === 'LC_RPATH') {
      const value = block.match(/^\s*path (.+) \(offset \d+\)\s*$/m)?.[1];
      requireCondition(value, 'Unparseable LC_RPATH');
      result.rpaths.push(value);
    } else if (cmd === 'LC_BUILD_VERSION' || cmd === 'LC_VERSION_MIN_MACOSX') {
      requireCondition(result.minimum === null, 'Ambiguous minimum OS commands');
      if (cmd === 'LC_BUILD_VERSION') {
        requireCondition(/^\s*platform (?:1|macos)\s*$/im.test(block), 'Non-macOS Mach-O platform');
      }
      result.minimum = block.match(/^\s*(?:minos|version) (\d+(?:\.\d+){0,2})\s*$/m)?.[1];
      requireCondition(result.minimum, 'Unparseable minimum OS');
    }
  }
  requireCondition(result.minimum, 'Missing minimum OS command');
  return result;
}
function insideContents(value) {
  return value === 'Contents' || value.startsWith('Contents/');
}
function expandPath(value, owner, executable) {
  let expanded;
  if (value === '@loader_path' || value.startsWith('@loader_path/')) {
    expanded = p.join(p.dirname(owner), value.slice('@loader_path'.length));
  } else if (value === '@executable_path' || value.startsWith('@executable_path/')) {
    expanded = p.join(p.dirname(executable), value.slice('@executable_path'.length));
  } else {
    throw new Error(`Unsupported or external runpath: ${value}`);
  }
  requireCondition(insideContents(expanded), `Runpath escapes bundle: ${value}`);
  return expanded;
}
function isSystemLibrary(value) {
  const normalized = p.normalize(value);
  return normalized.startsWith('/usr/lib/') || normalized.startsWith('/System/Library/');
}
export function validateMachO(images, executable, minimum) {
  const byPath = new Map(images.map(image => [image.path, image]));
  requireCondition(byPath.size === images.length && byPath.has(executable), 'Missing/duplicate Mach-O executable');
  for (const image of images) {
    requireCondition(image.architectures.length === 1 && image.architectures[0] === 'arm64', `Not arm64-only: ${image.path}`);
    requireCondition(compareVersions(image.minimum, minimum) <= 0, `Minimum OS too new: ${image.path}: ${image.minimum}`);
  }
  const visited = new Set(), reachable = new Set(), edges = [];
  const queue = [{ file: executable, inherited: [] }];
  for (let i = 0; i < queue.length; i++) {
    requireCondition(i < 512, 'Dependency traversal limit exceeded');
    const { file, inherited } = queue[i], image = byPath.get(file);
    const roots = [...new Set([...image.rpaths.map(r => expandPath(r, file, executable)), ...inherited])];
    const key = JSON.stringify([file, roots]);
    if (visited.has(key)) continue;
    visited.add(key); reachable.add(file);
    for (const dependency of image.dependencies) {
      if (isSystemLibrary(dependency)) { edges.push({ from: file, dependency, system: true }); continue; }
      let candidates;
      if (dependency.startsWith('@rpath/')) {
        candidates = roots.map(root => p.join(root, dependency.slice(7)));
      } else {
        candidates = [expandPath(dependency, file, executable)];
      }
      requireCondition(candidates.every(insideContents), `Dependency escapes bundle: ${dependency}`);
      const matches = [...new Set(candidates.filter(candidate => byPath.has(candidate)))];
      requireCondition(matches.length === 1, `Unresolved/ambiguous dependency: ${file} -> ${dependency}`);
      edges.push({ from: file, dependency, resolved: matches[0] });
      queue.push({ file: matches[0], inherited: roots });
    }
  }
  requireCondition(reachable.size === images.length, 'Unreachable Mach-O payload requires explicit loader support');
  return edges;
}

// Unknown mount metadata must never authorize deletion of the mount directory.
export function hasMountedImage(info, mount) {
  requireCondition(info && Array.isArray(info.images), 'Cannot establish mount state');
  requireCondition(typeof mount === 'string' && p.isAbsolute(mount), 'Invalid expected mount point');
  let found = false;
  for (const image of info.images) {
    requireCondition(image && Array.isArray(image['system-entities']), 'Unknown image entity list');
    for (const entity of image['system-entities']) {
      requireCondition(entity && typeof entity === 'object' && !Array.isArray(entity), 'Unknown mount entity');
      const value = entity['mount-point'];
      if (value === undefined) continue;
      requireCondition(typeof value === 'string' && p.isAbsolute(value), 'Unknown mount point');
      if (p.normalize(value) === p.normalize(mount)) found = true;
    }
  }
  return found;
}

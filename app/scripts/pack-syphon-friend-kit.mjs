// Package only validated CI installers and the portable acceptance assets.
import { copyFile, mkdir, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const sha = process.env.GITHUB_SHA;
if (process.platform !== 'darwin') throw new Error('Build the friend kit on the macOS CI runner.');
if (!/^[0-9a-f]{40}$/.test(sha ?? '')) throw new Error('GITHUB_SHA must identify the exact CI commit.');
const version = JSON.parse(await readFile(join(root, 'app/src-tauri/tauri.conf.json'), 'utf8')).version;
const output = resolve(process.argv[2] ?? join(root, 'target/qa/macos-artifact'));
await mkdir(output, { recursive: true });
const name = `syndocal-macos-friend-syphon-kit-${sha}`;
const stage = await mkdtemp(join(tmpdir(), 'syndocal-syphon-kit-'));
try {
  const kit = join(stage, name);
  await mkdir(join(kit, 'screenshots'), { recursive: true });
  const files = [];
  for (const extension of ['dmg', 'pkg']) {
    const directory = join(root, 'target/release/bundle', extension);
    const matches = (await readdir(directory)).filter(file => file.endsWith(`.${extension}`));
    if (matches.length !== 1 || !matches[0].includes(version) || !matches[0].includes('arm64')) {
      throw new Error(`Expected exactly one ${version} arm64 .${extension}, found ${matches.join(', ')}`);
    }
    files.push([join(directory, matches[0]), matches[0]]);
  }
  for (const asset of ['syphon-acceptance-project.sdc', 'SYPHON-FRIEND-TEST-GUIDE.md', 'allow-syndocal.command', 'observations.txt']) {
    files.push([join(root, 'tools/distribution/syphon-friend', asset), asset]);
  }
  files.push([join(root, 'tools/distribution/collect-syphon-acceptance.command'), 'collect-syphon-acceptance.command']);
  const receipt = { schema_version: 1, commit: sha, version, architecture: 'arm64',
    syphon_input_enabled: false, physical_acceptance: 'pending', files: [] };
  for (const [source, filename] of files) {
    const bytes = await readFile(source);
    if (!bytes.length) throw new Error(`Empty kit asset: ${filename}`);
    await copyFile(source, join(kit, filename));
    receipt.files.push({ filename, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') });
  }
  await writeFile(join(kit, 'kit-manifest.json'), `${JSON.stringify(receipt, null, 2)}\n`);
  // .command permissions survive the zip when generated on the actual runner.
  for (const command of ['allow-syndocal.command', 'collect-syphon-acceptance.command']) {
    const result = spawnSync('/bin/chmod', ['755', join(kit, command)], { encoding: 'utf8' });
    if (result.status !== 0) throw new Error(result.stderr || `chmod failed: ${command}`);
  }
  const zip = join(output, `${name}.zip`);
  const result = spawnSync('/usr/bin/ditto', ['-c', '-k', '--sequesterRsrc', '--keepParent', kit, zip], { encoding: 'utf8' });
  if (result.status !== 0) throw new Error(result.stderr || 'ditto failed');
  const bytes = await readFile(zip);
  await writeFile(`${zip}.sha256`, `${createHash('sha256').update(bytes).digest('hex')}  ${name}.zip\n`);
  console.log(zip);
} finally {
  await rm(stage, { recursive: true, force: true });
}

import * as fs from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { createHash } from 'node:crypto';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createCommandRunner } from './macos-artifact-process.mjs';
import { reserveReport } from './macos-artifact-report.mjs';
import { setImmediate as nextTurn } from 'node:timers/promises';
import { ARTIFACT_CHECKS as CHECKS, requireCondition, selectDmg, hasMountedImage } from './macos-artifact-policy.mjs';
import { inspectInfo, inspectMachO, inspectSignature, readPlist } from './macos-artifact-inspect.mjs';
const ROOT = fileURLToPath(new URL('../../', import.meta.url));


export async function cleanupSession(session, io) {
  if (!session.root) return { status: 'not_needed' };
  requireCondition(!io.hasUnreaped(), 'Unreaped process: preserving temporary directory');
  if (session.mountAttempted) {
    if (await io.isMounted(session.mount)) await io.detach(session.mount);
    requireCondition(!await io.isMounted(session.mount), 'DMG still mounted: preserving temporary directory');
  }
  await io.remove(session.root);
  return { status: 'removed' };
}
async function sha256(file) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest('hex');
}
export async function main(argv = process.argv.slice(2)) {
  const options = {};
  for (let i = 0; i < argv.length; i += 2) {
    requireCondition(['--dmg-dir', '--report'].includes(argv[i]) && argv[i + 1] && !options[argv[i]], 'Usage: --dmg-dir DIR --report NEW_JSON_PATH');
    options[argv[i]] = path.resolve(argv[i + 1]);
  }
  requireCondition(options['--dmg-dir'] && options['--report'], 'Both --dmg-dir and --report are required');
  const reportPath = options['--report'];
  const report = { schemaVersion: 1, status: 'fail', startedAt: new Date().toISOString(),
    host: { platform: process.platform, architecture: process.arch, release: os.release() },
    commit: null, runId: process.env.GITHUB_RUN_ID ?? null, runAttempt: process.env.GITHUB_RUN_ATTEMPT ?? null,
    checks: Object.fromEntries(CHECKS.map(name => [name, { status: 'not_run' }])), commands: [],
    unverified: ['M2 hardware', 'macOS 12 execution', 'native UI responsiveness', 'camera/DMX/MIDI/ASIO/NDI',
      'real show acceptance', 'Developer ID trust', 'notarization', 'Gatekeeper'] };
  await fs.mkdir(path.dirname(reportPath), { recursive: true });
  const reservation = await reserveReport(reportPath); // Exclusive final evidence; no open unused handle.
  const runner = createCommandRunner(report.commands), run = runner.run;
  const session = { root: null, mount: null, mountAttempted: false };
  const interrupt = () => runner.interrupt();
  process.on('SIGINT', interrupt); process.on('SIGTERM', interrupt);
  const step = async (name, operation) => {
    try { const value = await operation(); report.checks[name] = { status: 'pass', value }; return value; }
    catch (error) {
      report.checks[name] = { status: 'fail', reason: error.message,
        exitCode: error.result?.code ?? null, signal: error.result?.signal ?? null };
      throw error;
    }
  };
  try {
    const expected = await step('environment', async () => {
      requireCondition(process.platform === 'darwin' && process.arch === 'arm64', 'Requires macOS arm64; no product process launched');
      requireCondition(process.env.GITHUB_ACTIONS === 'true' && process.env.RUNNER_ENVIRONMENT === 'github-hosted', 'Only disposable GitHub-hosted runners are permitted');
      const head = (await run('/usr/bin/git', ['-C', ROOT, 'rev-parse', 'HEAD'])).stdout.trim();
      requireCondition(/^[a-f0-9]{40}$/.test(head) && head === process.env.GITHUB_SHA, 'Checkout SHA does not match workflow SHA');
      requireCondition(!(await run('/usr/bin/git', ['-C', ROOT, 'status', '--porcelain'])).stdout.trim(), 'Tracked/untracked source changes invalidate provenance');
      report.commit = head;
      report.osVersion = (await run('/usr/bin/sw_vers', ['-productVersion'])).stdout.trim();
      const config = JSON.parse(await fs.readFile(path.join(ROOT, 'app/src-tauri/tauri.conf.json'), 'utf8'));
      const pkg = JSON.parse(await fs.readFile(path.join(ROOT, 'app/package.json'), 'utf8'));
      requireCondition(config.version === pkg.version, 'Source version metadata mismatch');
      return { version: pkg.version, identifier: config.identifier, minimum: config.bundle.macOS.minimumSystemVersion };
    });
    const dmg = await step('dmg', async () => {
      const name = selectDmg(await fs.readdir(options['--dmg-dir']), expected.version);
      const file = path.join(options['--dmg-dir'], name), stat = await fs.lstat(file);
      requireCondition(stat.isFile(), 'DMG must be a regular file, not a link');
      return { name, path: file, size: stat.size, sha256: await sha256(file) };
    });
    await step('image_integrity', async () => {
      await run('/usr/bin/hdiutil', ['imageinfo', dmg.path]);
      await run('/usr/bin/hdiutil', ['verify', dmg.path], { timeoutMs: 120000 });
    });
    const app = await step('extraction', async () => {
      session.root = await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-dmg-check-')));
      session.mount = path.join(session.root, 'volume');
      await fs.mkdir(session.mount);
      session.mountAttempted = true;
      await run('/usr/bin/hdiutil', ['attach', '-readonly', '-nobrowse', '-noautoopen', '-mountpoint', session.mount, dmg.path], { timeoutMs: 120000 });
      const source = path.join(session.mount, 'Syndocal.app');
      requireCondition((await fs.lstat(source)).isDirectory(), 'DMG does not contain a regular Syndocal.app directory');
      requireCondition(await fs.readlink(path.join(session.mount, 'Applications')) === '/Applications', 'Invalid drag-to-Applications link');
      const copy = path.join(session.root, 'extracted', 'Syndocal.app');
      await run('/usr/bin/ditto', [source, copy], { timeoutMs: 120000 });
      requireCondition((await fs.lstat(copy)).isDirectory(), 'App extraction failed');
      return copy;
    });
    await step('bundle_info', () => inspectInfo(app, expected, run));
    await step('macho', () => inspectMachO(app, expected.minimum, run));
    await step('signature', () => inspectSignature(app, run));
    await step('process_survival', async () => {
      const home = path.join(session.root, 'home'), tmp = path.join(session.root, 'tmp');
      await fs.mkdir(home); await fs.mkdir(tmp);
      const result = await run(path.join(app, 'Contents/MacOS/syndocal'), [], { cwd: home, survivalMs: 8000,
        env: { PATH: '/usr/bin:/bin:/usr/sbin:/sbin', HOME: home, CFFIXED_USER_HOME: home,
          TMPDIR: `${tmp}/`, LANG: 'en_US.UTF-8' } });
      return { observedMs: result.survivalObservedMs, reaped: result.reaped, exitCode: result.code, signal: result.signal };
    });
    requireCondition(await sha256(dmg.path) === dmg.sha256, 'DMG changed during validation');
  } catch (error) { report.error = error.message; }
  finally {
    try {
      await step('cleanup', () => cleanupSession(session, {
        hasUnreaped: runner.hasUnreaped,
        isMounted: async mount => {
          const info = await run('/usr/bin/hdiutil', ['info', '-plist'], { cleanup: true });
          const file = path.join(session.root, 'mount-info.plist');
          await fs.writeFile(file, info.stdout);
          const parsed = await readPlist(file, run, { cleanup: true });
          const listed = hasMountedImage(parsed, mount);
          const [mounted, parent] = await Promise.all([fs.stat(mount), fs.stat(session.root)]);
          requireCondition(listed || mounted.dev === parent.dev, 'Unlisted mounted filesystem; preserving temporary directory');
          return listed;
        },
        detach: mount => run('/usr/bin/hdiutil', ['detach', mount], { cleanup: true }),
        remove: root => fs.rm(root, { recursive: true, force: false }),
      }));
    } catch (error) { report.cleanupError = error.message; report.retainedTemporaryDirectory = session.root; }
    await finalizeChecks(report, runner.wasInterrupted, async () => {
      const dmg = report.checks.dmg.value;
      await fs.writeFile(`${dmg.path}.sha256`, `${dmg.sha256}  ${dmg.name}\n`, { flag: 'wx' });
    });
    try {
      await reservation.publish(report, runner.wasInterrupted);
      await nextTurn(); // Observe queued signals before declaring the CLI successful.
      requireCondition(!runner.wasInterrupted(), 'Validation interrupted after report publication');
    } finally { process.off('SIGINT', interrupt); process.off('SIGTERM', interrupt); }
  }
  if (report.status !== 'pass') throw new Error(`Final DMG validation failed; see ${reportPath}`);
  const dmg = report.checks.dmg.value;
  requireCondition(!runner.wasInterrupted(), 'Validation interrupted after report publication');
  console.log(`Final DMG CI validation passed: ${dmg.name}; SHA-256 ${dmg.sha256}`);
}

if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) {
  main().catch(error => { console.error(error.message); process.exitCode = 1; });
}

export async function finalizeChecks(report, isInterrupted, publishChecksum) {
  const precedingPass = () => !report.error && CHECKS.filter(name => name !== 'checksum')
    .every(name => report.checks[name]?.status === 'pass');
  if (precedingPass() && !isInterrupted()) {
    try {
      await publishChecksum();
      report.checks.checksum = { status: 'pass' };
    } catch (error) {
      report.checks.checksum = { status: 'fail', reason: error.message };
      report.error = `Checksum publication failed: ${error.message}`;
    }
  }
  report.interrupted = isInterrupted();
  if (report.interrupted) report.error ??= 'Validation interrupted';
  report.status = !report.error && CHECKS.every(name => report.checks[name]?.status === 'pass') ? 'pass' : 'fail';
}

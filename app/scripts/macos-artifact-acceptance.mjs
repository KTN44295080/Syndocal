import * as fs from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { ARTIFACT_CHECKS, requireCondition, selectDmg } from './macos-artifact-policy.mjs';

/** CI upload authorization; a standalone JSON status is never sufficient. */
export function validateAcceptanceReport(report, expected) {
  requireCondition(expected.validationOutcome === 'success', 'Validation step outcome is not success');
  requireCondition(path.isAbsolute(expected.reportPath) && path.isAbsolute(expected.dmgDirectory), 'Expected paths must be absolute');
  requireCondition(/^[a-f0-9]{40}$/.test(expected.commit ?? '')
    && /^[1-9]\d*$/.test(expected.runId ?? '') && /^[1-9]\d*$/.test(expected.runAttempt ?? ''), 'Missing CI identity');
  requireCondition(report?.schemaVersion === 1 && report.status === 'pass'
    && report.interrupted === false && !report.error && !report.cleanupError
    && !report.publicationError, 'Artifact validation did not finish successfully');
  requireCondition(report.commit === expected.commit && report.runId === expected.runId
    && report.runAttempt === expected.runAttempt, 'Report belongs to another commit/run/attempt');
  requireCondition(report.host?.platform === 'darwin' && report.host.architecture === 'arm64', 'Report host mismatch');
  requireCondition(report.checks && JSON.stringify(Object.keys(report.checks).sort())
    === JSON.stringify([...ARTIFACT_CHECKS].sort()), 'Incomplete or unexpected check inventory');
  requireCondition(ARTIFACT_CHECKS.every(name => report.checks[name]?.status === 'pass'), 'A required check did not pass');
  requireCondition(report.checks.bundle_info.value?.version === expected.version, 'Source/artifact version mismatch');
  const publication = report.publication;
  requireCondition(publication?.workflowSuccessRequired === true
    && publication.finalPath === expected.reportPath
    && publication.stagingPolicy === 'success requires alias removal', 'Invalid publication boundary');
  const staging = publication.stagingPath;
  requireCondition(typeof staging === 'string' && path.dirname(staging) === path.dirname(expected.reportPath)
    && staging.startsWith(`${expected.reportPath}.`) && /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\.pending$/.test(staging.slice(expected.reportPath.length + 1)), 'Invalid staging evidence path');
  const dmg = report.checks.dmg.value;
  requireCondition(dmg && typeof dmg.name === 'string'
    && dmg.path === path.join(expected.dmgDirectory, dmg.name), 'DMG evidence path mismatch');
  selectDmg([dmg.name], expected.version);
  requireCondition(/^[a-f0-9]{64}$/.test(dmg.sha256 ?? '')
    && Number.isSafeInteger(dmg.size) && dmg.size > 0, 'Invalid DMG hash/size evidence');
  const survival = report.checks.process_survival.value;
  requireCondition(survival?.reaped === true && Number.isFinite(survival.observedMs)
    && survival.observedMs >= 8000, 'Incomplete process survival evidence');
  requireCondition((survival.exitCode === 0 && survival.signal === null)
    || (survival.exitCode === null && survival.signal === 'SIGTERM'), 'Abnormal process termination');
  return { dmg, staging };
}

export async function verifyAcceptanceFiles(report, expected) {
  const { dmg, staging } = validateAcceptanceReport(report, expected);
  const entries = await fs.readdir(expected.dmgDirectory);
  selectDmg(entries, expected.version);
  const sidecars = entries.filter(name => name.endsWith('.dmg.sha256'));
  requireCondition(sidecars.length === 1 && sidecars[0] === `${dmg.name}.sha256`, 'Unexpected checksum upload inventory');
  try { await fs.lstat(staging); throw new Error('Writable staging alias remains'); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  const stat = await fs.lstat(dmg.path);
  requireCondition(stat.isFile() && stat.size === dmg.size, 'DMG is missing, replaced, or not a regular file');
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(dmg.path)) hash.update(chunk);
  requireCondition(hash.digest('hex') === dmg.sha256, 'Final DMG hash changed');
  requireCondition((await fs.lstat(`${dmg.path}.sha256`)).isFile(), 'Checksum sidecar is not a regular file');
  requireCondition(await fs.readFile(`${dmg.path}.sha256`, 'utf8') === `${dmg.sha256}  ${dmg.name}\n`, 'Checksum sidecar mismatch');
  return dmg;
}

async function main(argv = process.argv.slice(2)) {
  requireCondition(argv.length === 4 && argv[0] === '--dmg-dir' && argv[2] === '--report', 'Usage: --dmg-dir DIR --report JSON_PATH');
  requireCondition(process.env.GITHUB_ACTIONS === 'true'
    && process.env.RUNNER_ENVIRONMENT === 'github-hosted'
    && process.env.MACOS_VALIDATION_OUTCOME === 'success', 'Requires a successful validation step on GitHub-hosted CI');
  const reportPath = path.resolve(argv[3]);
  const stat = await fs.lstat(reportPath);
  requireCondition(stat.isFile() && stat.size <= 64 * 1024 * 1024, 'Invalid final report file');
  const root = fileURLToPath(new URL('../../', import.meta.url));
  const pkg = JSON.parse(await fs.readFile(path.join(root, 'app/package.json'), 'utf8'));
  const report = JSON.parse(await fs.readFile(reportPath, 'utf8'));
  const dmg = await verifyAcceptanceFiles(report, {
    validationOutcome: process.env.MACOS_VALIDATION_OUTCOME,
    commit: process.env.GITHUB_SHA, runId: process.env.GITHUB_RUN_ID,
    runAttempt: process.env.GITHUB_RUN_ATTEMPT, version: pkg.version,
    reportPath, dmgDirectory: path.resolve(argv[1]),
  });
  console.log(`Final DMG evidence matched this run: ${dmg.name}; SHA-256 ${dmg.sha256}`);
}
if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) {
  main().catch(error => { console.error(error.message); process.exitCode = 1; });
}

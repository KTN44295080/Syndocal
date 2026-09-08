import * as fs from 'node:fs/promises';
import * as syncFs from 'node:fs';
import { randomUUID } from 'node:crypto';
import { setImmediate as nextTurn } from 'node:timers/promises';

const MAX_REPORT_BYTES = 64 * 1024 * 1024;
function bytesFor(report) {
  const bytes = Buffer.from(`${JSON.stringify(report, null, 2)}\n`);
  if (bytes.length > MAX_REPORT_BYTES) throw new Error('Artifact report exceeds 64 MiB');
  return bytes;
}
function markInterrupted(report, isInterrupted) {
  if (!isInterrupted()) return;
  report.interrupted = true;
  report.status = 'fail';
  report.error ??= 'Validation interrupted before report commit';
}
function writePreparedSync(file, bytes, io) {
  const fd = io.openSync(file, 'r+');
  try {
    io.ftruncateSync(fd, 0);
    io.writeFileSync(fd, bytes);
    io.fsyncSync(fd);
  } finally { io.closeSync(fd); }
}

/** The final link is a point-in-time commit, not proof that its CI job succeeded. */
export async function reserveReport(destination, io = { fs, sync: syncFs, nextTurn }) {
  try {
    await io.fs.lstat(destination);
    throw new Error(`Report already exists: ${destination}`);
  } catch (error) { if (error.code !== 'ENOENT') throw error; }
  const pending = `${destination}.${randomUUID()}.pending`;
  let used = false;
  return {
    pending,
    async publish(report, isInterrupted) {
      if (used) throw new Error('Report reservation already consumed');
      used = true;
      const handle = await io.fs.open(pending, 'wx');
      // No pass is visible at the final path while asynchronous I/O can yield.
      try {
        await handle.writeFile(bytesFor({ ...report, status: 'incomplete' }));
        await handle.sync();
      } finally { await handle.close(); }
      await io.nextTurn();
      markInterrupted(report, isInterrupted);
      report.finishedAt = new Date().toISOString();
      report.publication = { finalPath: destination, workflowSuccessRequired: true,
        scope: 'checks committed at this path; pending files are not acceptance',
        stagingPath: pending, stagingPolicy: 'success requires alias removal' };
      let linked = false;
      try {
        // Bounded offline CLI I/O. No await between the last signal check and commit.
        writePreparedSync(pending, bytesFor(report), io.sync);
        if (isInterrupted() && !report.interrupted) {
          markInterrupted(report, isInterrupted);
          writePreparedSync(pending, bytesFor(report), io.sync);
        }
        io.sync.linkSync(pending, destination); // Atomic visibility, no overwrite.
        linked = true;
        io.sync.unlinkSync(pending);
      } catch (error) {
        report.status = 'fail';
        report.publicationError = error.message;
        try { if (!linked) writePreparedSync(pending, bytesFor(report), io.sync); }
        catch { /* The failed operation and retained path are reported to the caller. */ }
        throw new Error(`Report publication failed; pending evidence ${pending}: ${error.message}`);
      }
      // A linked record alone is insufficient: alias removal and CI success are required.
      return report.status;
    },
  };
}

import { spawn } from 'node:child_process';

// Keep ownership until close, including when the OS fails to reap a child.
export function createCommandRunner(log, spawnProcess = spawn) {
  const active = new Set();
  let interrupted = false;
  const interrupt = () => { interrupted = true; for (const stop of active) stop('interrupted'); };
  async function run(command, args, options = {}) {
    if (interrupted && !options.cleanup) throw new Error('Validation interrupted');
    const result = { command, args, code: null, signal: null, reaped: false, reason: null, sentSignals: [] };
    const started = Date.now();
    const chunks = { stdout: [], stderr: [] }, sizes = { stdout: 0, stderr: 0 };
    let child, closed = false, finished = false, observed = false;
    let timer, escalation, reapDeadline;
    const outputLimit = options.outputLimit ?? 1024 * 1024;
    await new Promise(resolve => {
      const finish = () => {
        if (finished) return;
        finished = true; clearTimeout(timer); clearTimeout(escalation); clearTimeout(reapDeadline);
        result.elapsedMs = Date.now() - started;
        resolve();
      };
      const signal = name => {
        if (closed || !child?.pid || child.exitCode !== null || child.signalCode !== null) return;
        try {
          if (!child.kill(name)) result.reason ??= `Could not send ${name}`;
          else {
            result.sentSignals.push(name);
            if (name === 'SIGKILL') result.reason ??= 'Child required forced termination';
          }
        }
        catch (error) { result.reason ??= error.message; }
      };
      const stop = reason => {
        result.reason ??= reason;
        if (escalation || closed) return;
        signal('SIGTERM');
        escalation = setTimeout(() => signal('SIGKILL'), options.graceMs ?? 2000);
        reapDeadline = setTimeout(() => {
          result.reason ??= 'Child did not close after termination';
          finish(); // Active ownership is deliberately retained until the close event.
        }, options.reapMs ?? 5000);
      };
      try {
        child = spawnProcess(command, args, { shell: false, windowsHide: true,
          cwd: options.cwd, env: options.env ?? process.env, stdio: ['ignore', 'pipe', 'pipe'] });
      } catch (error) { result.reason = error.message; finish(); return; }
      active.add(stop);
      for (const name of ['stdout', 'stderr']) child[name].on('data', data => {
        sizes[name] += data.length;
        if (sizes[name] <= outputLimit && !finished) chunks[name].push(Buffer.from(data));
        else stop(`Command ${name} exceeded output limit`);
      });
      child.on('error', error => { result.reason ??= error.message; stop(error.message); });
      child.once('close', (code, exitSignal) => {
        closed = true; active.delete(stop);
        result.code = code; result.signal = exitSignal; result.reaped = true;
        if (options.survivalMs && !observed) result.reason ??= 'Process exited before survival interval';
        finish();
      });
      timer = setTimeout(() => {
        if (options.survivalMs && child.exitCode === null && child.signalCode === null) {
          observed = true; result.survivalObservedMs = Date.now() - started; stop(null);
        } else stop('Command timed out');
      }, options.survivalMs ?? options.timeoutMs ?? 30000);
    });
    result.stdout = Buffer.concat(chunks.stdout).toString('utf8');
    result.stderr = Buffer.concat(chunks.stderr).toString('utf8');
    const expectedSurvivalExit = observed && result.sentSignals.includes('SIGTERM')
      && ((result.code === 0 && result.signal === null)
        || (result.code === null && result.signal === 'SIGTERM'));
    if (options.survivalMs && !expectedSurvivalExit) {
      result.reason ??= `Unexpected survival termination: ${result.code}/${result.signal}`;
    }
    log.push({ ...result, stdout: result.stdout.slice(0, 65536), stderr: result.stderr.slice(0, 65536),
      stdoutBytes: sizes.stdout, stderrBytes: sizes.stderr });
    if (result.reason || !result.reaped || (!options.survivalMs && result.code !== 0)) {
      const error = new Error(`${command}: ${result.reason ?? `exit ${result.code}`} ${result.stderr.slice(-2000)}`);
      error.result = result;
      throw error;
    }
    return result;
  }
  return { run, interrupt, hasUnreaped: () => active.size !== 0, wasInterrupted: () => interrupted };
}

import { outputMissedWarningThreshold, type LiveVideoOutputHealth } from "./liveVideoMonitorPolicy";

/** Enough completed intervals to avoid presenting a startup percentile as headroom. */
export const performanceBudgetMinimumSamples = 15;

/** Render is native preparation wall time (including decode), not present cadence.
 * This is a per-output diagnostic, not a sum across outputs sharing a render. */
export function liveVideoPerformanceBudget(output: LiveVideoOutputHealth) {
  const requestedBudgetMs = Number.isFinite(output.targetFps) && output.targetFps > 0
    ? 1_000 / output.targetFps : null;
  const budgetMs = requestedBudgetMs !== null && Number.isFinite(requestedBudgetMs) && requestedBudgetMs > 0
    ? requestedBudgetMs : null;
  const stale = output.lastFrameAgeMs !== null && output.lastFrameAgeMs > 1_000;
  const fresh = output.lastFrameAgeMs !== null && Number.isFinite(output.lastFrameAgeMs)
    && output.lastFrameAgeMs >= 0 && !stale;
  const measured = fresh && Number.isInteger(output.renderSampleCount)
    && (output.renderSampleCount ?? 0) >= performanceBudgetMinimumSamples
    && output.renderTimeP95Ms !== undefined && Number.isFinite(output.renderTimeP95Ms)
    && output.renderTimeP95Ms >= 0;
  const renderP95Ms = measured ? output.renderTimeP95Ms! : null;
  const warning = stale ? "Output stalled"
    : (output.recentMissedFrames ?? 0) > outputMissedWarningThreshold ? "Output missing frames"
    : renderP95Ms !== null && budgetMs !== null && renderP95Ms > budgetMs ? "Frame preparation over budget"
    : null;
  return { budgetMs, renderP95Ms, warning };
}

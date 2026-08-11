import { createMemo, Show } from "solid-js";
import {
  buildDaslightCustomCurvePreviewPath,
  decodeDaslightCustomRawY,
} from "../effectVisualization";
import type { DaslightCustomCurveSource } from "../types";

export interface DaslightCustomCurvePanelProps {
  source: DaslightCustomCurveSource | null;
}

/** Read-only provenance card for imported Daslight Custom CURVE effects. */
export function DaslightCustomCurvePanel(props: DaslightCustomCurvePanelProps) {
  const path = createMemo(() => props.source
    ? buildDaslightCustomCurvePreviewPath(props.source)
    : "");
  const sourceOrder = createMemo(() => props.source?.points
    .map((point) => `${point.x}:${point.raw_y}`)
    .join(",") ?? "");
  const easingSummary = createMemo(() => props.source?.points
    .slice(1)
    .map((point) => decodeDaslightCustomRawY(point.raw_y).easing)
    .filter((easing, index, values) => values.indexOf(easing) === index)
    .join(", ") ?? "");

  return (
    <Show when={props.source}>
      {(source) => (
        <section
          class="daslightCustomCurvePanel"
          data-daslight-custom-curve
          data-point-count={source().points.length}
          data-source-order={sourceOrder()}
          aria-label="Imported Daslight Custom CURVE"
        >
          <header>
            <strong>Custom CURVE</strong>
            <span>Daslight import</span>
          </header>
          <svg viewBox="0 0 100 32" preserveAspectRatio="none" aria-hidden="true">
            <line x1="0" y1="16" x2="100" y2="16" />
            <path d={path()} />
          </svg>
          <div class="daslightCustomCurveReadout tabularNums">
            <span>{source().points.length} points · Daslight order</span>
            <span>Phasing {Math.round(source().phasing * 100)}%</span>
            <span>{easingSummary() || "Linear"}</span>
          </div>
          <p>Imported source; point values and easing remain read-only.</p>
        </section>
      )}
    </Show>
  );
}

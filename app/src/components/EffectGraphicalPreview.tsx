import { createMemo, For, Show } from "solid-js";
import {
  buildColorGradient,
  buildCurvePreviewPath,
  buildLfoPreviewPath,
  buildPointPath,
  buildValuePreviewPath,
  movePointToPreview,
  normalizeMovePathPoints,
  sampleMovePath,
  transformMovePreview,
} from "../effectVisualization";
import type {
  ChaserEffectRequest,
  ColorEffectRequest,
  ColorMappingEffectRequest,
  CurveEffectRequest,
  DaslightCurveSource,
  EffectKind,
  EffectParamsSnapshot,
  EffectSummary,
  MappingEffectRequest,
  MoveEffectRequest,
  ValueEffectRequest,
} from "../types";

interface EffectGraphicalPreviewProps {
  effect?: EffectSummary;
  params?: EffectParamsSnapshot | null;
  compact?: boolean;
}

interface PreviewModel {
  kind: EffectKind;
  label: string;
  shape?: EffectSummary["shape"];
  low?: number;
  high?: number;
  phase?: number;
  periodMs?: number;
  daslightCurve?: DaslightCurveSource | null;
  color?: ColorEffectRequest | null;
  chaser?: ChaserEffectRequest | null;
  move?: MoveEffectRequest | null;
  value?: ValueEffectRequest | null;
  curve?: CurveEffectRequest | null;
  mapping?: MappingEffectRequest | null;
  colorMapping?: ColorMappingEffectRequest | null;
}

const modelFromParams = (params: EffectParamsSnapshot): PreviewModel => {
  if ("Lfo" in params) {
    return {
      kind: "Lfo",
      label: params.Lfo.label,
      shape: params.Lfo.shape,
      low: params.Lfo.low,
      high: params.Lfo.high,
      phase: params.Lfo.phase,
      periodMs: params.Lfo.period_ms,
      daslightCurve: params.Lfo.daslight_curve,
    };
  }
  if ("PositionWave" in params) {
    return {
      kind: "PositionWave",
      label: params.PositionWave.label,
      shape: params.PositionWave.shape,
      low: params.PositionWave.low,
      high: params.PositionWave.high,
      phase: params.PositionWave.phase,
    };
  }
  if ("Color" in params) return { kind: "Color", label: params.Color.label, color: params.Color };
  if ("Chaser" in params) return { kind: "Chaser", label: params.Chaser.label, chaser: params.Chaser };
  if ("Move" in params) return { kind: "Move", label: params.Move.label, move: params.Move };
  if ("Value" in params) return { kind: "Value", label: params.Value.label, value: params.Value };
  if ("Curve" in params) return { kind: "Curve", label: params.Curve.label, curve: params.Curve };
  if ("ColorMapping" in params) return { kind: "ColorMapping", label: params.ColorMapping.label, colorMapping: params.ColorMapping };
  return {
    kind: "Mapping",
    label: params.Mapping.label,
    shape: params.Mapping.shape,
    low: params.Mapping.low,
    high: params.Mapping.high,
    phase: params.Mapping.phase,
    mapping: params.Mapping,
  };
};

const modelFromEffect = (effect: EffectSummary): PreviewModel => ({
  kind: effect.effect_type,
  label: effect.label,
  shape: effect.shape,
  low: effect.low,
  high: effect.high,
  phase: effect.phase,
  periodMs: effect.period_ms ?? undefined,
  daslightCurve: effect.lfo?.daslight_curve,
  color: effect.color,
  chaser: effect.chaser,
  move: effect.move_effect,
  value: effect.value,
  curve: effect.curve,
  mapping: effect.mapping,
  colorMapping: effect.color_mapping,
});

const effectKindLabel = (kind: EffectKind) => {
  if (kind === "PositionWave") return "Spatial Wave";
  if (kind === "Mapping") return "Fixture Mapping";
  if (kind === "ColorMapping") return "Colour Mapping";
  return kind;
};

export function EffectGraphicalPreview(props: EffectGraphicalPreviewProps) {
  const packedColorCss = (pixel: number) => {
    const normalized = Math.max(0, Math.min(281_474_976_710_655, Math.round(pixel)));
    const red = Math.round(Math.floor(normalized / 4_294_967_296) / 257);
    const green = Math.round((Math.floor(normalized / 65_536) % 65_536) / 257);
    const blue = Math.round((normalized % 65_536) / 257);
    return `rgb(${red} ${green} ${blue})`;
  };
  const model = createMemo<PreviewModel | null>(() => {
    if (props.params) return modelFromParams(props.params);
    if (props.effect) return modelFromEffect(props.effect);
    return null;
  });
  const lfoPath = createMemo(() => {
    const current = model();
    return current?.shape
      ? buildLfoPreviewPath(
          current.shape,
          current.phase ?? 0,
          100,
          32,
          64,
          current.low ?? 0,
          current.high ?? 65_535,
          current.kind === "Lfo",
          current.daslightCurve,
          current.periodMs,
        )
      : "";
  });
  const moveControlPoints = createMemo(() => model()?.move?.points.map(movePointToPreview) ?? []);
  const moveRuntimePoints = createMemo(() => {
    const move = model()?.move;
    return move
      ? normalizeMovePathPoints(move.points, move.closed, move.interpolation).map(movePointToPreview)
      : [];
  });
  const moveControlSamples = createMemo(() => {
    const move = model()?.move;
    return move ? sampleMovePath(moveRuntimePoints(), move.interpolation, move.closed) : [];
  });
  const moveOutputSamples = createMemo(() => {
    const move = model()?.move;
    return move ? transformMovePreview(moveControlSamples(), move) : [];
  });
  const valuePath = createMemo(() => {
    const value = model()?.value;
    return value
      ? buildValuePreviewPath(value.points, value.interpolation, value.direction, value.phase)
      : "";
  });
  const curvePath = createMemo(() => {
    const curve = model()?.curve;
    return curve ? buildCurvePreviewPath(curve.points, curve.direction, curve.phase) : "";
  });
  const accessibleLabel = createMemo(() => {
    const current = model();
    return current ? `${effectKindLabel(current.kind)} graphical preview` : "Effect graphical preview";
  });

  return (
    <Show when={model()}>
      {(current) => (
        <div
          class={props.compact ? "effectGraphicalPreview compact" : "effectGraphicalPreview"}
          role="img"
          aria-label={accessibleLabel()}
          data-effect-preview="true"
          data-preview-kind={current().kind}
          data-effect-label={current().label}
        >
          <Show when={current().kind === "Lfo" || current().kind === "PositionWave" || current().kind === "Mapping"}>
            <svg
              class="effectGraphicalCurve"
              viewBox="0 0 100 32"
              preserveAspectRatio="none"
              data-lfo-shape={current().shape}
              data-lfo-low={current().low}
              data-lfo-high={current().high}
              data-lfo-phase={current().phase}
              data-daslight-curve={current().daslightCurve ? current().shape : undefined}
              data-mapping-direction={current().mapping?.direction}
              data-mapping-repetitions={current().mapping?.repetitions}
              data-mapping-fixture-order={current().mapping?.fixture_ids.join(",")}
              aria-hidden="true"
            >
              <line x1="0" y1="16" x2="100" y2="16" />
              <path d={lfoPath()} />
            </svg>
            <span class="effectGraphicalReadout tabularNums">
              <b>{current().shape}</b>
              <Show when={current().daslightCurve}><span>DVC source</span></Show>
              <span>{Math.round(current().low ?? 0)}–{Math.round(current().high ?? 0)}</span>
              <span>φ {Math.round((current().phase ?? 0) * 100)}%</span>
              <Show when={current().mapping}>
                {(mapping) => <span>{mapping().repetitions.toFixed(2)}× order</span>}
              </Show>
            </span>
          </Show>

          <Show when={current().color}>
            {(color) => (
              <>
                <span
                  class="effectGraphicalGradient"
                  style={{ background: buildColorGradient(color().stops, color().interpolation) }}
                  data-stop-count={color().stops.length}
                  data-stop-positions={color().stops.map((stop) => stop.position).join(",")}
                  data-stop-colors={color().stops.map((stop) => `${stop.color.red}:${stop.color.green}:${stop.color.blue}`).join(",")}
                  data-color-interpolation={color().interpolation}
                  aria-hidden="true"
                />
                <span class="effectGraphicalReadout tabularNums">
                  <b>{color().algorithm}</b>
                  <span>{color().stops.length} stops</span>
                  <span>φ {Math.round(color().phase * 100)}%</span>
                </span>
              </>
            )}
          </Show>

          <Show when={current().colorMapping}>
            {(mapping) => (
              <>
                <span
                  class="effectGraphicalColorMap"
                  style={{ "grid-template-columns": `repeat(${Math.min(mapping().width, 16)}, 1fr)` }}
                  data-raster={`${mapping().width}x${mapping().height}`}
                  data-frame-count={mapping().frames.length}
                  data-cell-count={mapping().cells?.length ?? 0}
                  data-source-kind={mapping().source_kind}
                  data-playback-direction={mapping().playback_direction}
                  data-wrap-mode={mapping().wrap_mode}
                  data-sampling={mapping().sampling}
                  aria-hidden="true"
                >
                  <For each={(mapping().frames[0]?.pixels ?? []).slice(0, 128)}>
                    {(pixel) => <i style={{ background: packedColorCss(pixel) }} />}
                  </For>
                </span>
                <span class="effectGraphicalReadout tabularNums">
                  <b>{mapping().source_kind}</b>
                  <span>{mapping().width}×{mapping().height}</span>
                  <span>{mapping().frames.length} frame{mapping().frames.length === 1 ? "" : "s"}</span>
                </span>
              </>
            )}
          </Show>

          <Show when={current().move}>
            {(move) => (
              <>
                <svg
                  class="effectGraphicalMove"
                  viewBox="0 0 100 100"
                  preserveAspectRatio="xMidYMid meet"
                  data-point-count={move().points.length}
                  data-move-points={move().points.map((point) => `${point.x}:${point.y}`).join(",")}
                   data-move-center={`${move().center_x}:${move().center_y}`}
                   data-move-size={`${move().size_x}:${move().size_y}`}
                   data-move-rotation={move().rotation_degrees}
                   data-move-coordinate-mode={move().coordinate_mode}
                   data-move-preview-base={move().coordinate_mode === "Relative" ? "0.5:0.5" : "absolute"}
                   aria-hidden="true"
                >
                  <rect x="0" y="0" width="100" height="100" />
                  <line x1="50" y1="0" x2="50" y2="100" />
                  <line x1="0" y1="50" x2="100" y2="50" />
                  <path class="control" d={buildPointPath(moveControlSamples(), move().closed)} />
                  <path class="output" d={buildPointPath(moveOutputSamples(), move().closed)} />
                  <For each={moveControlPoints()}>
                    {(point) => <circle cx={point.x} cy={point.y} r="2.8" />}
                  </For>
                </svg>
                <span class="effectGraphicalReadout tabularNums">
                   <b>{move().interpolation}</b>
                   <span>{move().points.length} points</span>
                   <span>{move().direction}</span>
                   <Show when={move().coordinate_mode === "Relative"}>
                     <span>REL @ 50%</span>
                   </Show>
                 </span>
              </>
            )}
          </Show>

          <Show when={current().value}>
            {(value) => (
              <>
                <svg
                  class="effectGraphicalCurve value"
                  viewBox="0 0 100 32"
                  preserveAspectRatio="none"
                  data-value-points={value().points.map((point) => `${point.position}:${point.value}`).join(",")}
                  data-value-interpolation={value().interpolation}
                  aria-hidden="true"
                >
                  <line x1="0" y1="16" x2="100" y2="16" />
                  <path d={valuePath()} />
                </svg>
                <span class="effectGraphicalReadout tabularNums">
                  <b>{value().interpolation}</b>
                  <span>{value().points.length} points</span>
                  <span>{value().direction}</span>
                </span>
              </>
            )}
          </Show>

          <Show when={current().curve}>
            {(curve) => (
              <>
                <svg
                  class="effectGraphicalCurve curve"
                  viewBox="0 0 100 32"
                  preserveAspectRatio="none"
                  data-curve-points={curve().points
                    .map((point) => `${point.position}:${point.value}:${point.in_tangent}:${point.out_tangent}`)
                    .join(",")}
                  aria-hidden="true"
                >
                  <line x1="0" y1="16" x2="100" y2="16" />
                  <path d={curvePath()} />
                </svg>
                <span class="effectGraphicalReadout tabularNums">
                  <b>Cubic</b>
                  <span>{curve().points.length} points</span>
                  <span>{curve().direction}</span>
                </span>
              </>
            )}
          </Show>

          <Show when={current().chaser}>
            {(chaser) => (
              <>
                <span
                  class="effectGraphicalChaser"
                  data-step-count={chaser().steps.length}
                  data-active-step-count={chaser().active_step_count}
                  aria-hidden="true"
                >
                  <For each={chaser().steps.slice(0, 16)}>
                    {(step, index) => (
                      <i
                        class={index() < chaser().active_step_count ? "active" : ""}
                        style={{ "--effect-step-level": `${Math.max(0.08, Math.min(1, step.level / 65_535))}` }}
                      />
                    )}
                  </For>
                </span>
                <span class="effectGraphicalReadout tabularNums">
                  <b>{chaser().direction}</b>
                  <span>{chaser().steps.length} steps</span>
                  <span>{chaser().active_step_count} active</span>
                </span>
              </>
            )}
          </Show>
        </div>
      )}
    </Show>
  );
}

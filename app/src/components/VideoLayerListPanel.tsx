import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import type { VideoBlendMode, VideoColorAdjust, VideoFxAdjust, VideoIsfEffectSummary, VideoLayerState, VideoLayerSummary } from "../types";
import { videoSourceMetadataLabel } from "../videoHelpers";
import { defaultColorAdjust, defaultFxAdjust } from "../videoLayerDefaults";
import { VideoIsfEffectPanel } from "./VideoIsfEffectPanel";

interface VideoLayerListPanelProps {
  layers: VideoLayerSummary[];
  compact?: boolean;
  onSetLayerLabel: (layerId: number, label: string) => void | Promise<void>;
  onMoveLayer: (layerId: number, delta: -1 | 1) => void | Promise<void>;
  onDuplicateLayer: (layer: VideoLayerSummary) => void | Promise<void>;
  onRefreshMetadata: (layerId: number) => void | Promise<void>;
  onSetBlendMode: (layerId: number, blendMode: VideoBlendMode) => void | Promise<void>;
  onSetLayerState: (layerId: number, state: VideoLayerState) => void | Promise<void>;
  onSetLayerTransform: (
    layerId: number,
    state: VideoLayerState,
    transformPatch: Partial<VideoLayerState["transform"]>,
  ) => void | Promise<void>;
  onSetLayerColor: (
    layerId: number,
    state: VideoLayerState,
    colorPatch: Partial<VideoColorAdjust>,
  ) => void | Promise<void>;
  onSetLayerFx: (layerId: number, state: VideoLayerState, fxPatch: Partial<VideoFxAdjust>) => void | Promise<void>;
  isfRuntimeError?: string | null;
  onImportIsf: (layerId: number) => void | Promise<void>;
  onApplyBuiltinIsf: (layerId: number, presetId: string) => void | Promise<void>;
  onSetIsfEffect: (layerId: number, effect: VideoIsfEffectSummary | null) => void | Promise<void>;
  onAddCuePoint: (layerId: number) => void | Promise<void>;
  onJumpCuePoint: (layerId: number, cuePointIndex: number) => void | Promise<void>;
  onRemoveCuePoint: (layerId: number, positionMs: number) => void | Promise<void>;
  onRemoveLayer: (layerId: number) => void | Promise<void>;
}

export function VideoLayerListPanel(props: VideoLayerListPanelProps) {
  const pageSize = () => (props.compact ? 6 : Math.max(1, props.layers.length));
  const [page, setPage] = createSignal(0);
  const pageCount = createMemo(() => Math.max(1, Math.ceil(props.layers.length / pageSize())));
  const visibleLayers = createMemo(() => {
    const start = page() * pageSize();
    return props.layers.slice(start, start + pageSize()).map((layer, offset) => ({ layer, index: start + offset }));
  });

  createEffect(() => {
    if (page() >= pageCount()) setPage(pageCount() - 1);
  });

  return (
    <div class={`videoLayerList ${props.compact ? "compact" : ""}`}>
      <Show when={props.compact && props.layers.length > 0}>
        <div class="deckPager">
          <strong>Layers</strong>
          <span>
            {page() * pageSize() + 1}-{Math.min((page() + 1) * pageSize(), props.layers.length)} / {props.layers.length}
          </span>
          <button onClick={() => setPage(Math.max(0, page() - 1))} disabled={page() === 0} aria-label="Previous layer bank">
            Prev
          </button>
          <button
            onClick={() => setPage(Math.min(pageCount() - 1, page() + 1))}
            disabled={page() >= pageCount() - 1}
            aria-label="Next layer bank"
          >
            Next
          </button>
        </div>
      </Show>
      <Show when={props.layers.length > 0} fallback={<span class="emptyState">No video layers. Add a file, still, or input above.</span>}>
        <For each={visibleLayers()}>
          {(entry) => {
            const layer = entry.layer;
            const index = () => entry.index;
            return (
            <div class="videoLayerItem">
            <div>
              <strong class="videoLayerTitle" data-no-localize>{layer.label}</strong>
              <label>
                Video Layer name
                <input value={layer.label} onChange={(event) => void props.onSetLayerLabel(layer.id, event.currentTarget.value)} />
              </label>
              <span>{layer.source.path ?? layer.source.name ?? layer.source.kind}</span>
              <Show when={videoSourceMetadataLabel(layer.source)}>{(metadata) => <small>{metadata()}</small>}</Show>
            </div>
            <div class="videoMixerLayerDeck">
              <label>
                Opacity
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={layer.state.opacity}
                  onInput={(event) =>
                    void props.onSetLayerState(layer.id, {
                      ...layer.state,
                      opacity: Number(event.currentTarget.value),
                    })
                  }
                />
                <strong>{Math.round(layer.state.opacity * 100)}%</strong>
              </label>
              <div class="buttonRow">
                <button
                  class={layer.state.playing ? "active" : ""}
                  onClick={() =>
                    void props.onSetLayerState(layer.id, {
                      ...layer.state,
                      playing: !layer.state.playing,
                    })
                  }
                >
                  {layer.state.playing ? "Pause" : "Play"}
                </button>
                <button
                  class={layer.state.solo ? "active" : ""}
                  onClick={() =>
                    void props.onSetLayerState(layer.id, {
                      ...layer.state,
                      solo: !layer.state.solo,
                    })
                  }
                >
                  {layer.state.solo ? "Solo On" : "Solo"}
                </button>
                <button
                  class={layer.state.enabled ? "active" : ""}
                  onClick={() =>
                    void props.onSetLayerState(layer.id, {
                      ...layer.state,
                      enabled: !layer.state.enabled,
                    })
                  }
                >
                  {layer.state.enabled ? "On" : "Off"}
                </button>
                <button
                  onClick={() =>
                    void props.onSetLayerState(layer.id, {
                      ...layer.state,
                      opacity: 0,
                    })
                  }
                >
                  Out
                </button>
                <button
                  onClick={() =>
                    void props.onSetLayerState(layer.id, {
                      ...layer.state,
                      opacity: 1,
                    })
                  }
                >
                  Full
                </button>
              </div>
            </div>
            <div class="buttonRow">
              <button onClick={() => void props.onMoveLayer(layer.id, -1)} disabled={index() === 0}>
                Up
              </button>
              <button onClick={() => void props.onMoveLayer(layer.id, 1)} disabled={index() === props.layers.length - 1}>
                Down
              </button>
              <button onClick={() => void props.onDuplicateLayer(layer)}>Duplicate</button>
              <button
                onClick={() => void props.onRefreshMetadata(layer.id)}
                disabled={layer.source.kind !== "File" && layer.source.kind !== "StillImage"}
              >
                Refresh Metadata
              </button>
            </div>
            <label>
              Blend
              <select
                value={layer.blend_mode}
                onInput={(event) => void props.onSetBlendMode(layer.id, event.currentTarget.value as VideoBlendMode)}
              >
                <option value="Normal">Normal</option>
                <option value="Add">Add</option>
                <option value="Multiply">Multiply</option>
                <option value="Screen">Screen</option>
              </select>
            </label>
            <label class="checkbox">
              <input
                type="checkbox"
                checked={layer.state.enabled}
                onChange={(event) =>
                  void props.onSetLayerState(layer.id, {
                    ...layer.state,
                    enabled: event.currentTarget.checked,
                  })
                }
              />
              Video layer enabled
            </label>
            <label class="checkbox">
              <input
                type="checkbox"
                checked={layer.state.solo}
                onChange={(event) =>
                  void props.onSetLayerState(layer.id, {
                    ...layer.state,
                    solo: event.currentTarget.checked,
                  })
                }
              />
              Solo layer
            </label>
            <VideoIsfEffectPanel
              layerId={layer.id}
              layerLabel={layer.label}
              compact={props.compact}
              effect={layer.isf_effect}
              runtimeError={props.isfRuntimeError?.startsWith(`${layer.label}:`) ? props.isfRuntimeError : null}
              onImport={props.onImportIsf}
              onApplyBuiltin={props.onApplyBuiltinIsf}
              onSetEffect={props.onSetIsfEffect}
            />
            <div class="split">
              <label>
                Opacity
                <input
                  type="number"
                  min="0"
                  max="1"
                  step="0.01"
                  value={layer.state.opacity}
                  onInput={(event) =>
                    void props.onSetLayerState(layer.id, {
                      ...layer.state,
                      opacity: Number(event.currentTarget.value),
                    })
                  }
                />
              </label>
              <label>
                Speed
                <input
                  type="number"
                  min="-4"
                  max="4"
                  step="0.1"
                  value={layer.state.speed}
                  onInput={(event) =>
                    void props.onSetLayerState(layer.id, {
                      ...layer.state,
                      speed: Number(event.currentTarget.value),
                    })
                  }
                />
              </label>
            </div>
            <div class="videoTransformControls">
              <h3>Transform</h3>
              <div class="split">
                <label>
                  X
                  <input
                    type="number"
                    step="0.01"
                    value={layer.state.transform.x}
                    onChange={(event) =>
                      void props.onSetLayerTransform(layer.id, layer.state, {
                        x: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label>
                  Y
                  <input
                    type="number"
                    step="0.01"
                    value={layer.state.transform.y}
                    onChange={(event) =>
                      void props.onSetLayerTransform(layer.id, layer.state, {
                        y: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
              </div>
              <div class="split">
                <label>
                  Scale X
                  <input
                    type="number"
                    min="0.001"
                    step="0.01"
                    value={layer.state.transform.scale_x}
                    onChange={(event) =>
                      void props.onSetLayerTransform(layer.id, layer.state, {
                        scale_x: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label>
                  Scale Y
                  <input
                    type="number"
                    min="0.001"
                    step="0.01"
                    value={layer.state.transform.scale_y}
                    onChange={(event) =>
                      void props.onSetLayerTransform(layer.id, layer.state, {
                        scale_y: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
              </div>
              <label>
                Rotation
                <input
                  type="number"
                  step="1"
                  value={layer.state.transform.rotation_deg}
                  onChange={(event) =>
                    void props.onSetLayerTransform(layer.id, layer.state, {
                      rotation_deg: Number(event.currentTarget.value),
                    })
                  }
                />
              </label>
              <div class="split">
                <label>
                  Crop L
                  <input
                    type="number"
                    min="0"
                    max="1"
                    step="0.01"
                    value={layer.state.transform.crop_left}
                    onChange={(event) =>
                      void props.onSetLayerTransform(layer.id, layer.state, {
                        crop_left: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label>
                  Crop R
                  <input
                    type="number"
                    min="0"
                    max="1"
                    step="0.01"
                    value={layer.state.transform.crop_right}
                    onChange={(event) =>
                      void props.onSetLayerTransform(layer.id, layer.state, {
                        crop_right: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
              </div>
              <div class="split">
                <label>
                  Crop T
                  <input
                    type="number"
                    min="0"
                    max="1"
                    step="0.01"
                    value={layer.state.transform.crop_top}
                    onChange={(event) =>
                      void props.onSetLayerTransform(layer.id, layer.state, {
                        crop_top: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label>
                  Crop B
                  <input
                    type="number"
                    min="0"
                    max="1"
                    step="0.01"
                    value={layer.state.transform.crop_bottom}
                    onChange={(event) =>
                      void props.onSetLayerTransform(layer.id, layer.state, {
                        crop_bottom: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
              </div>
            </div>
            <div class="videoColorControls">
              <h3>Color</h3>
              <div class="split">
                <label>
                  Bright
                  <input
                    type="number"
                    min="-1"
                    max="1"
                    step="0.01"
                    value={layer.state.color?.brightness ?? defaultColorAdjust.brightness}
                    onChange={(event) =>
                      void props.onSetLayerColor(layer.id, layer.state, {
                        brightness: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label>
                  Contrast
                  <input
                    type="number"
                    min="0"
                    max="4"
                    step="0.01"
                    value={layer.state.color?.contrast ?? defaultColorAdjust.contrast}
                    onChange={(event) =>
                      void props.onSetLayerColor(layer.id, layer.state, {
                        contrast: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
              </div>
              <div class="split">
                <label>
                  Hue
                  <input
                    type="number"
                    step="1"
                    value={layer.state.color?.hue_deg ?? defaultColorAdjust.hue_deg}
                    onChange={(event) =>
                      void props.onSetLayerColor(layer.id, layer.state, {
                        hue_deg: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label>
                  Sat
                  <input
                    type="number"
                    min="0"
                    max="4"
                    step="0.01"
                    value={layer.state.color?.saturation ?? defaultColorAdjust.saturation}
                    onChange={(event) =>
                      void props.onSetLayerColor(layer.id, layer.state, {
                        saturation: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
              </div>
              <label>
                Gamma
                <input
                  type="number"
                  min="0.1"
                  max="4"
                  step="0.01"
                  value={layer.state.color?.gamma ?? defaultColorAdjust.gamma}
                  onChange={(event) =>
                    void props.onSetLayerColor(layer.id, layer.state, {
                      gamma: Number(event.currentTarget.value),
                    })
                  }
                />
              </label>
            </div>
            <div class="videoFxControls">
              <h3>FX</h3>
              <div class="split">
                <label>
                  Pixelate
                  <input
                    type="number"
                    min="1"
                    max="128"
                    step="1"
                    value={layer.state.fx?.pixelate ?? defaultFxAdjust.pixelate}
                    onChange={(event) =>
                      void props.onSetLayerFx(layer.id, layer.state, {
                        pixelate: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label>
                  Blur
                  <input
                    type="number"
                    min="0"
                    max="8"
                    step="1"
                    value={layer.state.fx?.blur ?? defaultFxAdjust.blur}
                    onChange={(event) =>
                      void props.onSetLayerFx(layer.id, layer.state, {
                        blur: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
              </div>
              <div class="split">
                <label>
                  Glow
                  <input
                    type="number"
                    min="0"
                    max="4"
                    step="0.01"
                    value={layer.state.fx?.glow ?? defaultFxAdjust.glow}
                    onChange={(event) =>
                      void props.onSetLayerFx(layer.id, layer.state, {
                        glow: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label>
                  Edge
                  <input
                    type="number"
                    min="0"
                    max="4"
                    step="0.01"
                    value={layer.state.fx?.edge ?? defaultFxAdjust.edge}
                    onChange={(event) =>
                      void props.onSetLayerFx(layer.id, layer.state, {
                        edge: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
              </div>
              <div class="triple">
                <label>
                  Key R
                  <input
                    type="number"
                    min="0"
                    max="1"
                    step="0.01"
                    value={layer.state.fx?.key_red ?? defaultFxAdjust.key_red}
                    onChange={(event) =>
                      void props.onSetLayerFx(layer.id, layer.state, {
                        key_red: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label>
                  Key G
                  <input
                    type="number"
                    min="0"
                    max="1"
                    step="0.01"
                    value={layer.state.fx?.key_green ?? defaultFxAdjust.key_green}
                    onChange={(event) =>
                      void props.onSetLayerFx(layer.id, layer.state, {
                        key_green: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label>
                  Key B
                  <input
                    type="number"
                    min="0"
                    max="1"
                    step="0.01"
                    value={layer.state.fx?.key_blue ?? defaultFxAdjust.key_blue}
                    onChange={(event) =>
                      void props.onSetLayerFx(layer.id, layer.state, {
                        key_blue: Number(event.currentTarget.value),
                      })
                    }
                  />
                </label>
              </div>
              <label>
                Key Threshold
                <input
                  type="number"
                  min="0"
                  max="1"
                  step="0.01"
                  value={layer.state.fx?.key_threshold ?? defaultFxAdjust.key_threshold}
                  onChange={(event) =>
                    void props.onSetLayerFx(layer.id, layer.state, {
                      key_threshold: Number(event.currentTarget.value),
                    })
                  }
                />
              </label>
            </div>
            <label class="checkbox">
              <input
                type="checkbox"
                checked={layer.state.playing}
                onChange={(event) =>
                  void props.onSetLayerState(layer.id, {
                    ...layer.state,
                    playing: event.currentTarget.checked,
                  })
                }
              />
              Playing
            </label>
            <div class="split">
              <label>
                Position ms
                <input
                  type="number"
                  min="0"
                  max={layer.source.metadata?.duration_ms ?? undefined}
                  value={layer.state.position_ms}
                  onChange={(event) =>
                    void props.onSetLayerState(layer.id, {
                      ...layer.state,
                      position_ms: Number(event.currentTarget.value),
                    })
                  }
                />
              </label>
              <label class="checkbox inlineCheckbox">
                <input
                  type="checkbox"
                  checked={layer.state.loop_enabled}
                  onChange={(event) =>
                    void props.onSetLayerState(layer.id, {
                      ...layer.state,
                      loop_enabled: event.currentTarget.checked,
                    })
                  }
                />
                Loop
              </label>
            </div>
            <Show when={layer.state.loop_enabled}>
              <>
                <div class="split">
                  <label>
                    Loop in
                    <input
                      type="number"
                      min="0"
                      max={layer.source.metadata?.duration_ms ?? undefined}
                      value={layer.state.loop_start_ms}
                      onChange={(event) =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          loop_start_ms: Number(event.currentTarget.value),
                        })
                      }
                    />
                  </label>
                  <label>
                    Loop out
                    <input
                      type="number"
                      min="0"
                      max={layer.source.metadata?.duration_ms ?? undefined}
                      value={layer.state.loop_end_ms}
                      onChange={(event) =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          loop_end_ms: Number(event.currentTarget.value),
                        })
                      }
                    />
                  </label>
                </div>
                <Show when={layer.source.metadata?.duration_ms}>
                  {(durationMs) => (
                    <button
                      onClick={() =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          loop_enabled: true,
                          loop_start_ms: 0,
                          loop_end_ms: durationMs(),
                        })
                      }
                    >
                      Use Source Length
                    </button>
                  )}
                </Show>
              </>
            </Show>
            <div class="videoSyncControls">
              <label class="checkbox">
                <input
                  type="checkbox"
                  checked={layer.state.bpm_sync.enabled}
                  onChange={(event) =>
                    void props.onSetLayerState(layer.id, {
                      ...layer.state,
                      bpm_sync: {
                        ...layer.state.bpm_sync,
                        enabled: event.currentTarget.checked,
                      },
                    })
                  }
                />
                BPM sync
              </label>
              <div class="split">
                <label>
                  Ratio
                  <select
                    value={layer.state.bpm_sync.ratio}
                    onInput={(event) =>
                      void props.onSetLayerState(layer.id, {
                        ...layer.state,
                        bpm_sync: {
                          ...layer.state.bpm_sync,
                          ratio: Number(event.currentTarget.value),
                        },
                      })
                    }
                  >
                    <option value="0.25">0.25x</option>
                    <option value="0.5">0.5x</option>
                    <option value="1">1x</option>
                    <option value="2">2x</option>
                    <option value="4">4x</option>
                  </select>
                </label>
                <label>
                  Bars
                  <input
                    type="number"
                    min="0.25"
                    step="0.25"
                    value={layer.state.bpm_sync.loop_bars}
                    onChange={(event) =>
                      void props.onSetLayerState(layer.id, {
                        ...layer.state,
                        bpm_sync: {
                          ...layer.state.bpm_sync,
                          loop_bars: Number(event.currentTarget.value),
                        },
                      })
                    }
                  />
                </label>
              </div>
            </div>
            <div class="cuePointList">
              <div class="buttonRow">
                <button onClick={() => void props.onAddCuePoint(layer.id)}>Add Cue Point</button>
                <button onClick={() => void props.onJumpCuePoint(layer.id, 0)} disabled={layer.state.cue_points_ms.length === 0}>
                  Jump First
                </button>
              </div>
              <For each={layer.state.cue_points_ms}>
                {(cuePoint, cuePointIndex) => (
                  <div class="cuePointItem">
                    <button onClick={() => void props.onJumpCuePoint(layer.id, cuePointIndex())}>{cuePoint} ms</button>
                    <button onClick={() => void props.onRemoveCuePoint(layer.id, cuePoint)}>Remove</button>
                  </div>
                )}
              </For>
            </div>
            <div class="buttonRow">
              <button
                onClick={() =>
                  void props.onSetLayerState(layer.id, {
                    ...layer.state,
                    opacity: 0,
                  })
                }
              >
                Fade Out
              </button>
              <button onClick={() => void props.onRemoveLayer(layer.id)}>Remove</button>
            </div>
            </div>
            );
          }}
        </For>
      </Show>
    </div>
  );
}

import { For } from "solid-js";
import type { ChaserFeature } from "../types";

export interface EffectFeatureEditorPanelProps {
  effectLabel: string;
  features: ChaserFeature[];
  attributeOptions: string[];
  attributeCoverage: Record<string, string>;
  onFeatures: (features: ChaserFeature[]) => void;
}

const clampLevel = (value: number) => Math.max(0, Math.min(65_535, Math.round(value || 0)));
const featureKey = (attribute: string) => attribute.trim().toLowerCase();

export function EffectFeatureEditorPanel(props: EffectFeatureEditorPanelProps) {
  const updateFeature = (index: number, patch: Partial<ChaserFeature>) => {
    props.onFeatures(props.features.map((feature, candidate) => candidate === index
      ? {
          ...feature,
          ...patch,
          low: clampLevel(patch.low ?? feature.low),
          high: clampLevel(patch.high ?? feature.high),
        }
      : { ...feature }));
  };
  const moveFeature = (index: number, delta: -1 | 1) => {
    const destination = index + delta;
    if (destination < 0 || destination >= props.features.length) return;
    const next = props.features.map((feature) => ({ ...feature }));
    [next[index], next[destination]] = [next[destination], next[index]];
    props.onFeatures(next);
  };
  const featureOptions = (feature: ChaserFeature) => {
    const used = new Set(props.features
      .filter((candidate) => candidate !== feature)
      .map((candidate) => featureKey(candidate.attribute)));
    const options = props.attributeOptions.filter((attribute) => !used.has(featureKey(attribute)));
    return options.some((attribute) => featureKey(attribute) === featureKey(feature.attribute))
      ? options
      : [feature.attribute, ...options].filter(Boolean);
  };
  const addFeature = () => {
    const used = new Set(props.features.map((feature) => featureKey(feature.attribute)));
    const attribute = props.attributeOptions.find((candidate) => !used.has(featureKey(candidate)));
    if (!attribute || props.features.length >= 16) return;
    props.onFeatures([...props.features, { attribute, low: 0, high: 65_535 }]);
  };
  const coverage = (feature: ChaserFeature) =>
    props.attributeCoverage[featureKey(feature.attribute)] ?? "0 fixtures";

  return (
    <fieldset class="effectFeaturesPanel" data-effect-feature-editor={props.effectLabel}>
      <legend>Features</legend>
      <p class="effectFeaturesHint">
        One shared generator drives every assigned attribute; each Feature keeps its own range.
      </p>
      <div class="effectFeatureList" role="list" aria-label={`${props.effectLabel} feature ranges`}>
        <For each={props.features}>
          {(feature, index) => (
            <div class="effectFeatureRow" role="listitem" aria-posinset={index() + 1} aria-setsize={props.features.length}>
              <span class="effectFeatureNumber tabularNums" aria-hidden="true">{index() + 1}</span>
              <label>
                <span>Attribute</span>
                <select
                  value={feature.attribute}
                  onInput={(event) => updateFeature(index(), { attribute: event.currentTarget.value })}
                >
                  <For each={featureOptions(feature)}>
                    {(attribute) => <option value={attribute} data-no-localize>{attribute}</option>}
                  </For>
                </select>
                <small class="effectFeatureCoverage tabularNums" data-no-localize>{coverage(feature)}</small>
              </label>
              <label>
                <span>Low</span>
                <input
                  class="tabularNums"
                  type="number"
                  min="0"
                  max="65535"
                  value={feature.low}
                  onChange={(event) => updateFeature(index(), { low: Number(event.currentTarget.value) })}
                />
              </label>
              <label>
                <span>High</span>
                <input
                  class="tabularNums"
                  type="number"
                  min="0"
                  max="65535"
                  value={feature.high}
                  onChange={(event) => updateFeature(index(), { high: Number(event.currentTarget.value) })}
                />
              </label>
              <button type="button" aria-label={`Move ${props.effectLabel} feature ${index() + 1} up`} title="Move feature up" disabled={index() === 0} onClick={() => moveFeature(index(), -1)}>↑</button>
              <button type="button" aria-label={`Move ${props.effectLabel} feature ${index() + 1} down`} title="Move feature down" disabled={index() === props.features.length - 1} onClick={() => moveFeature(index(), 1)}>↓</button>
              <button type="button" class="remove" aria-label={`Remove ${props.effectLabel} feature ${index() + 1}`} title="Remove feature" disabled={props.features.length <= 1} onClick={() => props.onFeatures(props.features.filter((_, candidate) => candidate !== index()))}>×</button>
            </div>
          )}
        </For>
      </div>
      <div class="effectFeatureFooter">
        <span class="tabularNums">{props.features.length} / 16 features</span>
        <button type="button" onClick={addFeature} disabled={props.features.length >= 16 || props.attributeOptions.length <= props.features.length}>Add feature</button>
      </div>
    </fieldset>
  );
}

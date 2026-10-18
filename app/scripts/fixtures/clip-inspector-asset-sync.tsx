import { createSignal } from 'solid-js';
import { render } from 'solid-js/web';
import { VideoClipSlotInspectorPanel } from '../../src/components/VideoClipSlotInspectorPanel';
import '../../src/styles.css';

// Test-only entry: the real Clip Inspector with authoritative props that the check drives.
const unexpected = (name) => () => { throw new Error(`Unexpected Clip Inspector callback: ${name}`); };
const makeAssets = () => ['h264_1080p30', 'Pattern 02', 'Pattern 03', 'Pattern 04', 'Pattern 05', 'Pattern 06']
  .map((label, index) => ({ id: index + 1, label, source: { kind: 'StillImage', path: `${label}.png` }, content_hash: { algorithm: 'Sha256', hex: 'a'.repeat(64) }, byte_size: 1 }));
const slotBase = { in_point_ms: 0, out_point_ms: null, loop_mode: 'Loop', speed: 1, launch_quantization: 'Immediate', cue_points: [], effect_overrides: [] };
let slotAssets = { 1: 1, 3: 3, 5: 6 };
const makeLayer = () => ({
  id: 1, label: 'Texture', default_clip_slot_id: 1,
  clip_slots: Object.entries(slotAssets).map(([id, media_asset_id]) => ({ ...slotBase, id: Number(id), media_asset_id })),
});
const emptyCatalog = { effect_chains: [], effect_presets: [], layer_groups: [], transition_buses: [] };
const calls = [];
function Fixture() {
  const [assets, setAssets] = createSignal(makeAssets());
  const [layer, setLayer] = createSignal(makeLayer());
  const [slotId, setSlotId] = createSignal(5);
  window.__clipInspectorFixture = {
    calls: () => JSON.parse(JSON.stringify(calls)),
    setSlotId: (id) => setSlotId(id),
    refreshAssets: () => setAssets(makeAssets()),
    refreshLayer: () => setLayer(makeLayer()),
    setSlotAsset: (id, assetId) => { slotAssets = { ...slotAssets, [id]: assetId }; setLayer(makeLayer()); },
  };
  return <main style={{ padding: '12px', height: '100vh', overflow: 'auto' }}>
    <VideoClipSlotInspectorPanel
      layer={layer()} slotId={slotId()} assets={assets()} returnFocus={null} removeDisabledReason={null}
      effectCatalog={emptyCatalog}
      onClose={unexpected('onClose')}
      onCreate={(assetId, before) => { calls.push({ kind: 'create', assetId, before }); }}
      onAssign={(id, assetId) => { calls.push({ kind: 'assign', slotId: id, assetId }); }}
      onUpdate={(slot) => { calls.push({ kind: 'update', slotId: slot.id, assetId: slot.media_asset_id }); }}
      onRemove={unexpected('onRemove')} onDuplicate={unexpected('onDuplicate')} onSetDefault={unexpected('onSetDefault')}
      onReorder={unexpected('onReorder')} onApplyEffectCatalog={unexpected('onApplyEffectCatalog')}
      onImportScopeIsf={unexpected('onImportScopeIsf')} onApplyEffectPreset={unexpected('onApplyEffectPreset')}
      onRemoveEffectChain={unexpected('onRemoveEffectChain')}
    />
  </main>;
}
render(() => <Fixture />, document.getElementById('fixture-root'));

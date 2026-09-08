import { createSignal, mergeProps } from 'solid-js';
import { render } from 'solid-js/web';
import { VideoClipGridPanel } from '../../src/components/VideoClipGridPanel';
import { VideoControlPanel } from '../../src/components/VideoControlPanel';
import { createMediaThumbnailController } from '../../src/createMediaThumbnailController';
import '../../src/styles.css';

// Test-only entry: real components/controller, deferred readers, no native API.
const unexpected = () => { throw new Error('Unexpected non-thumbnail fixture operation'); };
const png = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==';
function Fixture() {
  const [layers] = createSignal([{ id: 1, label: 'Layer fixture', source: { kind: 'StillImage', path: 'fixture.png' }, state: { enabled: true, opacity: 1, playing: false } }]);
  const [assets, setAssets] = createSignal([{ id: 2, label: 'Asset fixture', source: { kind: 'StillImage', path: 'fixture.png' }, content_hash: { algorithm: 'Sha256', hex: 'a'.repeat(64) }, byte_size: 1 }]);
  const [nativeAvailable, setNativeAvailable] = createSignal(true);
  const requests = { layer: [], asset: [] };
  const reader = lane => id => new Promise((resolve, reject) => requests[lane].push({ id, resolve, reject }));
  const authority = { project_epoch: 1, project_revision: 0, checkpoint_hash: 'fixture' };
  const controller = createMediaThumbnailController({ layers, assets,
    projectMappingsAuthority: () => authority, isProjectAuthorityIdentityCurrent: a => a === authority || JSON.stringify(a) === JSON.stringify(authority),
    isTauriRuntime: nativeAvailable, loadVideoLayerThumbnail: reader('layer'), loadMediaAssetThumbnail: reader('asset'),
  });
  const clip = mergeProps(controller.layerThumbnailView, {
    compact: true, get layers() { return layers(); },
    onRequestThumbnails: controller.authorizeVideoThumbnailAccess,
    fadeMs: 0, audioMonitorVolume: 0, audioMonitorLayerIds: [], audioMonitorStatus: { active_layer_ids: [], last_sync_error: null },
    programAudioEnabled: false, audioOutputDevices: [], selectedAudioOutputDevice: '', deckALayerId: null, deckBLayerId: null, abMix: 0,
    selectedOutputId: null, recordingStatus: { active: false }, previewLayerId: null, previewBusy: false, previewError: null,
    previewBackendAvailable: false, firstRunAvailable: false, firstRunBusy: false, firstRunError: null, firstRunBackendAvailable: false,
  });
  const mediaLibrary = mergeProps(controller.assetThumbnailView, {
    get assets() { return assets(); },
    availabilityById: {}, activeOperations: [], lastImportReport: null,
    backendAvailable: false, onVerify: unexpected, onRelink: unexpected, onPreviewStart: unexpected,
    onPreviewFrame: unexpected, onPreviewEnd: unexpected, onCancelOperation: unexpected,
  });
  const sourceCreate = { sourceKind: 'StillImage', label: '', path: '', onSetSourceKind: unexpected,
    onSetLabel: unexpected, onSetPath: unexpected, onBrowseSource: unexpected, onImportMultiple: unexpected, onAddLayer: unexpected };
  window.__thumbnailRecoveryFixture = {
    counts: () => ({ layer: requests.layer.length, asset: requests.asset.length }),
    settle: (lane, index, success) => {
      const request = requests[lane][index]; if (!request) throw new Error('Missing fixture request');
      if (success) request.resolve(png); else request.reject(new Error('Fixture decode failure'));
    },
    reset: controller.reset, setNativeAvailable, authorize: controller.authorizeVideoThumbnailAccess,
    useLiveCatalog: () => setAssets([{ ...assets()[0], source: { kind: 'Camera', path: '' } }]),
  };
  return <main style={{ display: 'grid', 'grid-template-columns': '1fr 1fr', gap: '12px', padding: '12px', height: '100vh', overflow: 'auto' }}>
    <div style={{ 'min-width': '0', overflow: 'auto' }}><VideoClipGridPanel {...clip} /></div>
    <div style={{ 'min-width': '0', overflow: 'auto' }}><VideoControlPanel libraryOnly={true} mixer={false} layerCount={1}
      clipGrid={clip} mediaLibrary={mediaLibrary} sourceCreate={sourceCreate}
      clipSlotBank={{ mode: "edit", layers: [], layer: null, runtime: null, assets: [], thumbnails: {}, selectedSlotId: null }} /></div>
  </main>;
}
render(() => <Fixture />, document.getElementById('fixture-root'));

import { createSignal } from 'solid-js';
import { render } from 'solid-js/web';
import { MediaLibraryHealthDialog } from '../../src/components/MediaHealthDialog';
import { installUiLocalization, type UiLocale } from '../../src/uiLocalization';
import '../../src/theme.css';
import '../../src/styles.css';

// Test-only entry: the real health dialog with scripted callbacks. It proves the
// view and the relink review flow; the native pickers and the ticketed relink
// operation they call are covered by their own contracts, not by this fixture.
const hash = (hex: string) => ({ algorithm: 'Sha256', hex });
const asset = (id: number, label: string, path: string, byte_size: number | null, kind = 'File') =>
  ({ id, label, source: { kind, path }, content_hash: hash(String(id).repeat(64).slice(0, 64)), byte_size });
const initialAssets = [
  asset(1, 'Intro', 'D:\\old\\Intro.mp4', 1_048_576),
  asset(2, 'Loop', 'D:\\old\\loop.mov', 2_000_000),
  asset(3, 'Twins', 'D:\\old\\twins.mp4', 600),
  asset(4, 'Lost', 'D:\\old\\lost.mp4', 700),
  asset(5, 'Locked', 'D:\\old\\locked.mp4', 800),
  asset(6, 'Changed', 'D:\\old\\changed.png', 900, 'StillImage'),
  asset(7, 'Fresh', 'D:\\old\\fresh.mp4', null),
  asset(8, 'Good', 'D:\\old\\good.mp4', 5_368_709_120),
  { id: 9, label: 'Camera 1', source: { kind: 'Camera', path: null } },
];
const initialAvailability: Record<number, unknown> = {
  1: { kind: 'missing', asset_id: 1 },
  2: { kind: 'missing', asset_id: 2 },
  3: { kind: 'missing', asset_id: 3 },
  4: { kind: 'missing', asset_id: 4 },
  5: { kind: 'unreadable', asset_id: 5, error: 'Access is denied. (os error 5)' },
  6: { kind: 'hash_mismatch', asset_id: 6, expected: hash('e'.repeat(64)), actual: hash('f'.repeat(64)) },
  8: { kind: 'available_verified', asset_id: 8 },
};

function Fixture() {
  const [assets, setAssets] = createSignal<any[]>(initialAssets);
  const [availability, setAvailability] = createSignal<Record<number, any>>(initialAvailability);
  const [checkedAt, setCheckedAt] = createSignal<Record<number, number>>({ 8: Date.UTC(2026, 9, 20, 3, 4, 5) });
  const [busy, setBusy] = createSignal(false);
  const [folderEnabled, setFolderEnabled] = createSignal(true);
  const calls = { verify: [] as number[][], relink: [] as number[], scan: 0, apply: [] as any[][] };
  let nextRelinkResult = true;
  let nextScan: any = null;
  let nextApply: any = { relinked: [], differentContent: [], failed: [], skipped: [] };
  let dialog!: HTMLDialogElement;
  const library = {
    get assets() { return assets(); },
    get availabilityById() { return availability(); },
    get checkedAtById() { return checkedAt(); },
    backendAvailable: true,
    get activeOperations() { return busy() ? [{ id: 1, label: 'Verify 9 media assets', phase: 'hashing' }] : []; },
    onVerify: (ids: number[]) => {
      calls.verify.push(ids);
      const now = Date.UTC(2026, 9, 20, 6, 7, 8);
      setCheckedAt({ ...checkedAt(), ...Object.fromEntries(ids.map(id => [id, now])) });
    },
    onRelink: async (id: number) => { calls.relink.push(id); return nextRelinkResult; },
    get onScanRelinkFolder() {
      return folderEnabled() ? async () => { calls.scan += 1; return nextScan; } : undefined;
    },
    get onApplyRelinkRequests() {
      return folderEnabled() ? async (requests: any[], onProgress?: (done: number, total: number) => void) => {
        calls.apply.push(requests);
        onProgress?.(1, requests.length);
        return nextApply;
      } : undefined;
    },
  };
  (window as any).__mediaHealthFixture = {
    calls: () => JSON.parse(JSON.stringify(calls)),
    open: () => dialog.showModal(),
    setBusy,
    replaceAssetPath: (id: number, path: string) => setAssets(assets().map(a => a.id === id ? { ...a, source: { ...a.source, path } } : a)),
    setFolderEnabled,
    setRelinkResult: (value: boolean) => { nextRelinkResult = value; },
    setScan: (value: unknown) => { nextScan = value; },
    setApply: (value: unknown) => { nextApply = value; },
  };
  return <main style={{ padding: '12px' }}>
    <button type="button" data-fixture-open onClick={() => dialog.showModal()}>Media health…</button>
    <MediaLibraryHealthDialog library={library as any} ref={(element) => { dialog = element; }} />
  </main>;
}
const locale: UiLocale = new URLSearchParams(location.search).get('locale') === 'ja' ? 'ja' : 'en';
render(() => <Fixture />, document.getElementById('fixture-root')!);
installUiLocalization(document.body, () => locale);

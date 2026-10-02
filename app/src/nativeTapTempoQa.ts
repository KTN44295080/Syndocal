// Compiled out of ordinary builds. The isolated native acceptance runner can
// call the same registered-owner Tap handler through backend diagnostics,
// without native clicks or a second implementation of tempo control.
export function installNativeTapTempoQa(tap: () => Promise<void>, readState: () => unknown): () => void {
  const target = window as Window & {
    __syndocalQaTapTempo?: () => Promise<void>;
    __syndocalQaTapTempoState?: () => unknown;
  };
  if (target.__syndocalQaTapTempo) throw new Error("Native Tap QA receiver already installed");
  target.__syndocalQaTapTempo = tap;
  target.__syndocalQaTapTempoState = readState;
  return () => {
    if (target.__syndocalQaTapTempo === tap) delete target.__syndocalQaTapTempo;
    if (target.__syndocalQaTapTempoState === readState) delete target.__syndocalQaTapTempoState;
  };
}

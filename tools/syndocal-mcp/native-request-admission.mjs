// One process-wide budget covers every transport before descriptor/process or
// credential I/O. No queue or retries: excess requests are definitely unsent.
export const NATIVE_REQUEST_LIMIT = 8;

export function overloadedReceipt(requestId) {
  return { requestId, status: 'rejected', error: 'sidecar_overloaded',
    nextAction: 'This request was not sent. Wait for outstanding requests to finish before issuing another request.' };
}

export function createNativeRequestAdmission() {
  let active = 0;
  return {
    async run(requestId, operation) {
      if (active >= NATIVE_REQUEST_LIMIT) return overloadedReceipt(requestId);
      active++;
      try { return await operation(); }
      finally { active--; }
    },
  };
}

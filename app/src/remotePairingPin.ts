const PAIRING_PIN_SPACE = 1_000_000;
const UINT32_SPACE = 0x1_0000_0000;
const ACCEPTED_UINT32_LIMIT = Math.floor(UINT32_SPACE / PAIRING_PIN_SPACE) * PAIRING_PIN_SPACE;
const MAX_RANDOM_ATTEMPTS = 128;

/**
 * Generates a six-digit pairing PIN only from a cryptographically secure
 * source. `null` means secure randomness is unavailable or failed; callers
 * must keep Remote stopped instead of falling back to Math.random().
 */
export function generateRemotePairingPin(
  cryptoSource: Pick<Crypto, "getRandomValues"> | null | undefined = globalThis.crypto,
): string | null {
  if (!cryptoSource || typeof cryptoSource.getRandomValues !== "function") return null;

  const sample = new Uint32Array(1);
  for (let attempt = 0; attempt < MAX_RANDOM_ATTEMPTS; attempt += 1) {
    try {
      cryptoSource.getRandomValues(sample);
    } catch {
      return null;
    }

    const value = sample[0];
    if (value < ACCEPTED_UINT32_LIMIT) {
      return String(value % PAIRING_PIN_SPACE).padStart(6, "0");
    }
  }

  return null;
}

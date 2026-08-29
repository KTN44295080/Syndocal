import { createHash, createPublicKey, verify as verifySignature } from "node:crypto";

function strictBase64(text, label) {
  if (
    typeof text !== "string"
    || text.length === 0
    || text.length % 4 !== 0
    || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/u.test(text)
  ) {
    throw new Error(label + " is not strict base64.");
  }
  const decoded = Buffer.from(text, "base64");
  if (decoded.toString("base64") !== text) throw new Error(label + " is not canonical base64.");
  return decoded;
}

function canonicalUtf8EvidenceText(bytes, label) {
  if (!Buffer.isBuffer(bytes)) throw new Error(label + " must be UTF-8 bytes.");
  const decoded = bytes.toString("utf8");
  if (!Buffer.from(decoded, "utf8").equals(bytes)) throw new Error(label + " is not valid UTF-8.");
  return decoded.trim();
}

function canonicalBase64EvidenceText(bytes, label) {
  const canonical = canonicalUtf8EvidenceText(bytes, label);
  strictBase64(canonical, label);
  return canonical;
}

function parseMinisignSignature(signatureText, publicKey, artifactFilename, label) {
  const decodedSignature = strictBase64(signatureText, label + " base64 signature");
  const signatureBody = decodedSignature.toString("utf8");
  if (!Buffer.from(signatureBody, "utf8").equals(decodedSignature)) {
    throw new Error(label + " is not valid UTF-8.");
  }
  const lines = signatureBody.replace(/\r\n/gu, "\n").replace(/\n$/u, "").split("\n");
  if (
    lines.length !== 4
    || lines[0] !== "untrusted comment: signature from tauri secret key"
    || !lines[2].startsWith("trusted comment: ")
  ) {
    throw new Error(label + " is not an exact Tauri minisign signature.");
  }
  const signaturePacket = strictBase64(lines[1], label + " packet");
  const algorithm = signaturePacket.subarray(0, 2).toString("ascii");
  if (
    signaturePacket.length !== 74
    || !["ED", "Ed"].includes(algorithm)
    || !signaturePacket.subarray(2, 10).equals(publicKey.packet.subarray(2, 10))
  ) {
    throw new Error(label + " key identifier/packet is invalid.");
  }
  const trustedComment = lines[2].slice("trusted comment: ".length);
  const trustedCommentMatch = /^timestamp:[1-9]\d*\tfile:(.+)$/u.exec(trustedComment);
  if (!trustedCommentMatch || trustedCommentMatch[1] !== artifactFilename) {
    throw new Error(label + " trusted filename does not match the artifact.");
  }
  const trustedCommentSignature = strictBase64(lines[3], label + " trusted-comment signature");
  if (trustedCommentSignature.length !== 64) {
    throw new Error(label + " trusted-comment signature packet is invalid.");
  }
  return Object.freeze({ algorithm, signaturePacket, trustedComment, trustedCommentSignature });
}

/**
 * Decodes the repository's canonical base64-wrapped Tauri minisign public-key
 * evidence. The returned fingerprint is the SHA-256 of the trimmed outer
 * base64 text, which is the release-candidate evidence identity.
 */
export function parseTauriMinisignPublicKey(publicKeyEvidenceBytes, {
  expectedFingerprint,
  label = "Updater public-key evidence",
} = {}) {
  const canonicalText = canonicalUtf8EvidenceText(publicKeyEvidenceBytes, label);
  const privateKeyPattern = /PRIVATE KEY|SECRET KEY|TAURI_SIGNING_PRIVATE_KEY|BEGIN [A-Z ]*PRIVATE/iu;
  if (privateKeyPattern.test(canonicalText)) {
    throw new Error(label + " contains private-key material.");
  }
  strictBase64(canonicalText, label);
  const decodedPublicKey = strictBase64(canonicalText, label);
  const publicKeyText = decodedPublicKey.toString("utf8");
  if (!Buffer.from(publicKeyText, "utf8").equals(decodedPublicKey) || privateKeyPattern.test(publicKeyText)) {
    throw new Error(label + " contains private-key material or invalid UTF-8.");
  }
  const lines = publicKeyText.replace(/\r\n/gu, "\n").replace(/\n$/u, "").split("\n");
  const commentMatch = /^untrusted comment: minisign public key: ([0-9A-F]{16})$/u.exec(lines[0] ?? "");
  if (!commentMatch || lines.length !== 2) {
    throw new Error(label + " is not an exact Tauri minisign public key.");
  }
  const packet = strictBase64(lines[1], label + " packet");
  if (packet.length !== 42 || packet.subarray(0, 2).toString("ascii") !== "Ed") {
    throw new Error(label + " is not an exact Tauri minisign public key.");
  }
  const packetKeyId = Buffer.from(packet.subarray(2, 10)).reverse().toString("hex").toUpperCase();
  if (packetKeyId !== commentMatch[1]) {
    throw new Error(label + " key identifier does not match its minisign packet.");
  }
  const fingerprint = createHash("sha256").update(canonicalText, "utf8").digest("hex");
  if (
    expectedFingerprint !== undefined
    && (typeof expectedFingerprint !== "string" || !/^[0-9a-fA-F]{64}$/u.test(expectedFingerprint) || fingerprint !== expectedFingerprint.toLowerCase())
  ) {
    throw new Error(label.replace(/ evidence$/u, "") + " fingerprint mismatch.");
  }
  return Object.freeze({
    canonicalText,
    fingerprint,
    packet,
    cryptoKey: createPublicKey({
      key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), packet.subarray(10)]),
      format: "der",
      type: "spki",
    }),
  });
}

/**
 * Parses only canonical credential-free HTTPS updater URLs. Empty path
 * segments, encoded separators, and non-canonical percent escapes are rejected
 * so callers compare decoded channel/file identities rather than URL text.
 */
export function canonicalDecodedUpdaterUrlPath(value, label = "Updater URL") {
  if (typeof value !== "string" || value.length === 0 || value !== value.trim() || /[\0\r\n]/u.test(value)) {
    throw new Error(label + " must be a non-empty canonical URL.");
  }
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new Error(label + " is not a valid URL.");
  }
  if (url.href !== value || url.protocol !== "https:" || url.username || url.password || url.search || url.hash) {
    throw new Error(label + " must be credential-free HTTPS with canonical text.");
  }
  if (url.pathname === "/" || url.pathname.endsWith("/") || url.pathname.includes("//")) {
    throw new Error(label + " path has an empty directory segment; empty directories are not accepted.");
  }
  const rawSegments = url.pathname.slice(1).split("/");
  const decodedSegments = rawSegments.map((segment) => {
    let decoded;
    try {
      decoded = decodeURIComponent(segment);
    } catch {
      throw new Error(label + " path contains an invalid percent escape.");
    }
    if (
      decoded.length === 0
      || decoded === "."
      || decoded === ".."
      || /[\0\\/\r\n]/u.test(decoded)
      || encodeURIComponent(decoded) !== segment
    ) {
      throw new Error(label + " path segment is not a canonical decoded path component.");
    }
    return decoded;
  });
  return Object.freeze({ url, pathSegments: Object.freeze(decodedSegments) });
}

export function assertCanonicalUpdaterArtifactUrl(value, { channel, artifactFilename, label = "Updater artifact URL" } = {}) {
  if (typeof channel !== "string" || channel.length === 0 || channel !== channel.trim()) {
    throw new Error(label + " requires an exact release channel.");
  }
  if (typeof artifactFilename !== "string" || artifactFilename.length === 0 || artifactFilename !== artifactFilename.trim()) {
    throw new Error(label + " requires an exact artifact filename.");
  }
  const parsed = canonicalDecodedUpdaterUrlPath(value, label);
  if (!parsed.pathSegments.includes(channel) || parsed.pathSegments.at(-1) !== artifactFilename) {
    throw new Error(label + " channel/filename mismatch.");
  }
  return parsed;
}

/**
 * Verifies the signed Tauri updater payload against the same public-key and
 * canonical URL rules used by the release-candidate metadata checker. It does
 * no I/O and must complete before any candidate executable is inspected.
 */
export function verifyTauriUpdaterPayload({
  payloadBytes,
  artifactFilename,
  signatureEvidenceBytes,
  publicKey,
  publicKeyEvidenceBytes,
  expectedPublicKeyFingerprint,
  updaterManifest,
  target,
  channel,
  label = "Updater payload",
} = {}) {
  if (!Buffer.isBuffer(payloadBytes) || payloadBytes.length === 0) throw new Error(label + " bytes are empty.");
  if (typeof artifactFilename !== "string" || artifactFilename.length === 0) throw new Error(label + " filename is invalid.");
  if (typeof target !== "string" || target.length === 0) throw new Error(label + " target is invalid.");
  if (typeof updaterManifest !== "object" || updaterManifest === null || Array.isArray(updaterManifest)) {
    throw new Error(label + " updater manifest is invalid.");
  }
  const parsedPublicKey = publicKey ?? parseTauriMinisignPublicKey(publicKeyEvidenceBytes, {
    expectedFingerprint: expectedPublicKeyFingerprint,
    label: label + " public key",
  });
  if (expectedPublicKeyFingerprint !== undefined && parsedPublicKey.fingerprint !== String(expectedPublicKeyFingerprint).toLowerCase()) {
    throw new Error(label + " public-key fingerprint mismatch.");
  }
  const signatureText = canonicalUtf8EvidenceText(signatureEvidenceBytes, label + " signature evidence");
  const platform = updaterManifest.platforms?.[target];
  if (typeof platform !== "object" || platform === null || Array.isArray(platform) || platform.signature !== signatureText) {
    throw new Error(label + " updater-manifest signature does not match the exact signature file.");
  }
  assertCanonicalUpdaterArtifactUrl(platform.url, { channel, artifactFilename, label: label + " URL" });
  const signature = parseMinisignSignature(signatureText, parsedPublicKey, artifactFilename, "Updater signature");
  const payloadMessage = signature.algorithm === "ED"
    ? createHash("blake2b512").update(payloadBytes).digest()
    : payloadBytes;
  if (
    !verifySignature(null, payloadMessage, parsedPublicKey.cryptoKey, signature.signaturePacket.subarray(10))
    || !verifySignature(
      null,
      Buffer.concat([signature.signaturePacket.subarray(10), Buffer.from(signature.trustedComment, "utf8")]),
      parsedPublicKey.cryptoKey,
      signature.trustedCommentSignature,
    )
  ) {
    throw new Error(label + " cryptographic signature verification failed.");
  }
  return Object.freeze({ publicKey: parsedPublicKey, signatureText });
}

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { imageHash } from "./nsis-signing-pipeline.mjs";
import { inspectAuthenticodeBytes } from "../../../app/scripts/sign-windows-release-state.mjs";

const [mode, actual, expected] = process.argv.slice(2);
if (mode === "installer") {
  const bytes = readFileSync(actual);
  assert.equal(imageHash(bytes), imageHash(readFileSync(expected)), "SignPath returned a different NSIS image than the uploaded installer.");
  const signature = inspectAuthenticodeBytes(bytes);
  assert.equal(signature.status, "Valid");
  assert.equal(signature.timestamped, true);
  assert.match(signature.subject, /(?:^|,\s*)CN=SignPath Foundation(?:,|$)/u);
  assert.equal(signature.thumbprint, process.env.SIGNPATH_CERTIFICATE_THUMBPRINT);
  console.log("Returned NSIS image matches the uploaded CI-built installer and configured Foundation certificate.");
} else if (mode === "payload") {
  for (const name of ["syndocal.exe", "uninstall.exe"]) {
    assert.ok(readFileSync(join(actual, name)).equals(readFileSync(join(expected, name))), "Installed bytes differ from approved CI payload: " + name);
  }
  console.log("Installed project bytes match the exact approved CI payload.");
} else throw new Error("Usage: verify-ci-origin.mjs installer|payload <actual> <expected>");

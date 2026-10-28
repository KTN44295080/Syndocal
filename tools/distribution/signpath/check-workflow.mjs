import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

const action = "signpath/github-action-submit-signing-request@f6d04783b4569d051e0c80105fe66e82819d0092";
export function validateWorkflow(text) {
  const requireText = (snippet) => assert.ok(text.includes(snippet), "Missing release workflow safety contract: " + snippet);
  requireText("  workflow_dispatch:");
  assert.ok(!/^\s*(?:push|pull_request|pull_request_target|workflow_run|schedule):/mu.test(text), "Release workflow must be manually dispatched.");
  requireText("  contents: read\n  actions: read");
  assert.ok(!/^\s*[a-z-]+:\s*(?:write|write-all|inherit)\s*$|^\s*runs-on:.*self-hosted/gmu.test(text), "Signing workflow cannot grant write permissions or use self-hosted runners.");
  requireText("github.repository == 'KTN44295080/Syndocal' && github.ref_protected && (github.ref == 'refs/heads/main' || startsWith(github.ref, 'refs/tags/v'))");
  requireText("runs-on: windows-2022");
  requireText("environment: release-signpath");
  requireText("persist-credentials: false");
  assert.ok(!/\binputs[.:]|\bworkflow_call\b|gh release|git push|create-release|upload-release-asset/u.test(text), "No foreign source input or publishing path is allowed.");
  for (const use of text.matchAll(/^\s*uses:\s*(\S+)/gmu)) assert.match(use[1], /@[0-9a-f]{40}$/u, "Actions must be pinned by commit.");
  assert.equal(text.split("uses: " + action).length - 1, 2, "Exactly two official signing requests required.");
  for (const fragment of ["github-artifact-id: ${{ steps.payload.outputs.artifact-id }}", "github-artifact-id: ${{ steps.installer.outputs.artifact-id }}", "wait-for-completion: true", "artifact-configuration-slug: ${{ vars.SIGNPATH_PAYLOAD_CONFIGURATION_SLUG }}", "artifact-configuration-slug: ${{ vars.SIGNPATH_INSTALLER_CONFIGURATION_SLUG }}", "declared-unsigned", "missing-production-updater-configuration", "14.44.35207", "where.exe link.exe", "if (-not $exactLinkerFirst)", "--ci --no-bundle", "tauri bundle --ci --bundles nsis", "--certificate-thumbprint $env:SIGNPATH_CERTIFICATE_THUMBPRINT", "candidate/windows-signing.json", "steps.sign-payload.outputs.signing-request-id", "steps.sign-installer.outputs.signing-request-id"]) requireText(fragment);
  assert.ok(!/--bundles nsis,/u.test(text), "Only NSIS may be bundled for release candidates.");
  assert.ok(!/continue-on-error:\s*true/u.test(text), "A started signing failure cannot silently downgrade.");
  const ordered = ["id: payload", "id: sign-payload", "nsis-signing-pipeline.mjs package", "id: installer", "id: sign-installer", "signer sign", "finalize-candidate.mjs candidate"];
  let previous = -1;
  for (const fragment of ordered) { const current = text.indexOf(fragment); assert.ok(current > previous, "Signing/hash ordering violated at " + fragment); previous = current; }
}

function validateXml(text, names) {
  assert.ok(text.includes('xmlns="http://signpath.io/artifact-configuration/v1"'));
  assert.match(text, /<parameter name="version" required="true"\s*\/>/u);
  assert.deepEqual([...text.matchAll(/<pe-file path="([^"]+)"/gu)].map((match) => match[1]), names);
  assert.equal([...text.matchAll(/product-name="Syndocal" product-version="\$\{version\}"/gu)].length, names.length);
  assert.equal([...text.matchAll(/<authenticode-sign\s*\/>/gu)].length, names.length);
  assert.ok(!/<nsis-file|path="[^"\n]*\*/u.test(text), "Only explicitly owned PE files may be signed.");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const workflow = readFileSync(".github/workflows/windows-release-signpath.yml", "utf8");
  validateWorkflow(workflow);
  validateXml(readFileSync("tools/distribution/signpath/artifact-configuration.xml", "utf8"), ["Syndocal_${version}_x64-setup.exe"]);
  validateXml(readFileSync("tools/distribution/signpath/payload-artifact-configuration.xml", "utf8"), ["syndocal.exe", "uninstall.exe"]);
  let tests = 3;
  if (process.argv.includes("--self-test")) {
    for (const [before, after] of [["github.ref_protected", "true"], ["contents: read", "contents: write"], ["runs-on: windows-2022", "runs-on: self-hosted"], ["  workflow_dispatch:", "  push:"], ["--bundles nsis", "--bundles nsis,msi"], ["persist-credentials: false", "persist-credentials: true"], ["github-artifact-id: ${{ steps.payload.outputs.artifact-id }}", "github-artifact-id: 123"], ["id: sign-installer", "continue-on-error: true\n        id: sign-installer"], ["id: sign-installer", "id: wrong-installer"], [action, "signpath/github-action-submit-signing-request@v3"]]) {
      assert.throws(() => validateWorkflow(workflow.replace(before, after))); tests++;
    }
  }
  console.log(`SignPath workflow/XML contracts passed: ${tests} checks`);
}

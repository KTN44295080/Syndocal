import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const actionSource = await readFile(new URL("../src/gdtfProfileActions.ts", import.meta.url), "utf8");

const actionNames = [
  "selectGdtfFile",
  "selectLoadedProfile",
  "loadGdtfProfile",
  "importGdtf",
  "downloadGdtfFromUrl",
  "setCustomAttributesText",
  "commitCustomAttributeDrafts",
  "updateCustomAttributeDraft",
  "addCustomAttributeDraft",
  "appendCustomAttributeTemplate",
  "removeCustomAttributeDraft",
  "moveCustomAttributeDraft",
  "createCustomProfile",
  "saveCustomProfile",
  "loadCustomProfile",
];

assert.match(
  appSource,
  /import \{ createGdtfProfileActions \} from "\.\/gdtfProfileActions";/,
  "App must consume the extracted GDTF/custom-profile action factory",
);
assert.match(
  appSource,
  /\} = createGdtfProfileActions\(\{/,
  "App must construct the extracted actions with an explicit context",
);

for (const actionName of actionNames) {
  assert.match(
    actionSource,
    new RegExp(`const ${actionName}\\s*=`),
    `${actionName} must be owned by gdtfProfileActions.ts`,
  );
  assert.doesNotMatch(
    appSource,
    new RegExp(`const ${actionName}\\s*=`),
    `${actionName} must not remain as a closure in App.tsx`,
  );
}

assert.match(
  actionSource,
  /export interface GdtfProfileActionContext[\s\S]*?invoke: FrontendTauriInvoke[\s\S]*?setProfile: Setter<FixtureProfileSummary \| null>/,
  "the extracted module must declare its native/state dependencies explicitly",
);
assert.match(
  actionSource,
  /export function createGdtfProfileActions\(context: GdtfProfileActionContext\)/,
  "the extracted module must expose a context-bound factory",
);
assert.match(actionSource, /context\.invoke<string \| null>\("select_gdtf_file"\)/);
assert.match(actionSource, /context\.invoke<FixtureProfileSummary>\("import_gdtf"/);
assert.match(actionSource, /context\.invoke<string \| null>\("download_gdtf_from_url"/);
assert.match(actionSource, /context\.invoke<FixtureProfileSummary>\("preview_custom_fixture_profile"/);
assert.match(actionSource, /context\.invoke<string \| null>\("save_custom_fixture_profile"/);
assert.match(actionSource, /context\.invoke<FixtureProfileSummary \| null>\("load_custom_fixture_profile"/);
assert.match(
  actionSource,
  /const authority = context\.captureProjectAuthorityIdentity\(\);[\s\S]*?context\.isProjectAuthorityIdentityCurrent\(authority\)/,
  "custom preview must retain its project-authority discard fence",
);
assert.match(
  actionSource,
  /customProfileAttributeDraftsFromText[\s\S]*?customProfileAttributeTextFromDrafts/,
  "custom-profile draft actions must reuse the canonical draft conversion helpers",
);
assert.doesNotMatch(actionSource, /@tauri-apps\/api|(?:^|[^A-Za-z])tauriInvoke\s*[<(]/m);

const returnedNames = actionSource
  .match(/return \{([\s\S]*?)\n  \};/)?.[1]
  ?.match(/\b[A-Za-z][A-Za-z0-9]*\b/g) ?? [];
assert.deepEqual(
  returnedNames,
  actionNames,
  "the extracted factory must return exactly the existing Setup Profile actions",
);

console.log(`GDTF/custom-profile action split passed: ${actionNames.length} actions remain context-bound`);

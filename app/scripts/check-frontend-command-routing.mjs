import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const srcRoot = path.join(appRoot, "src");
const appPath = path.join(srcRoot, "App.tsx");
const detachedVideoPath = path.join(srcRoot, "components", "VideoOutputWindow.tsx");
const controlPlanePath = path.join(appRoot, "src-tauri", "src", "control_plane.rs");
const manifest = JSON.parse(fs.readFileSync(path.join(srcRoot, "tauri-invoke-manifest.json"), "utf8"));
const manifestSet = new Set(manifest);

const configPath = path.join(appRoot, "tsconfig.json");
const config = ts.readConfigFile(configPath, ts.sys.readFile);
if (config.error) throw new Error(ts.flattenDiagnosticMessageText(config.error.messageText, "\n"));
const parsed = ts.parseJsonConfigFileContent(config.config, ts.sys, appRoot, undefined, configPath);
if (parsed.errors.length > 0) {
  throw new Error(parsed.errors.map((error) => ts.flattenDiagnosticMessageText(error.messageText, "\n")).join("\n"));
}
const program = ts.createProgram(parsed.fileNames, parsed.options);
const checker = program.getTypeChecker();
const sourceFiles = program.getSourceFiles().filter((sourceFile) => {
  const relative = path.relative(srcRoot, sourceFile.fileName);
  return !sourceFile.isDeclarationFile && !relative.startsWith("..") && !path.isAbsolute(relative);
});
const appSource = sourceFiles.find((sourceFile) => path.resolve(sourceFile.fileName) === appPath);
assert(appSource, "App.tsx must be part of the frontend TypeScript program");
const appText = appSource.getFullText();

const functionSlice = (source, marker, nextMarker) => {
  const start = source.indexOf(marker);
  assert(start >= 0, `missing source marker ${marker}`);
  const end = source.indexOf(nextMarker, start + marker.length);
  assert(end > start, `missing source end marker ${nextMarker}`);
  return source.slice(start, end);
};

const remoteAccessRefreshBody = functionSlice(
  appText,
  "const refreshRemoteAccessUrls = async",
  "const copyRemoteUrl = async",
);
const djInterfaceRefreshBody = functionSlice(
  appText,
  "const refreshDjLinkLanInterfaces = async",
  "let djLinkTokenClearTimer",
);
for (const [label, body, generation, setter] of [
  ["remote URLs", remoteAccessRefreshBody, "remoteAccessUrlRequestGeneration", "setRemoteAccessUrls"],
  ["DJ LAN interfaces", djInterfaceRefreshBody, "djLinkLanInterfaceRequestGeneration", "setDjLinkLanInterfaces"],
]) {
  assert.match(body, new RegExp(`const\\s+requestGeneration\\s*=\\s*\\+\\+${generation}`), `${label} production helper must capture its generation`);
  const writes = [...body.matchAll(new RegExp(`${setter}\\(`, "g"))].length;
  const guardedWrites = [...body.matchAll(new RegExp(`requestGeneration\\s*===\\s*${generation}[^\\n]*${setter}\\(`, "g"))].length
    + [...body.matchAll(new RegExp(`requestGeneration\\s*===\\s*${generation}\\)\\s*\\{[\\s\\S]*?${setter}\\(`, "g"))].length;
  assert(writes >= 2, `${label} production helper must own success/failure writes`);
  assert(guardedWrites >= writes, `${label} production helper has an unguarded setter write`);
}

assert.equal(
  [...appText.matchAll(/void\s+refreshDjLinkLanInterfaces\s*\(\s*\)/g)].length,
  1,
  "DJ Link enable must have exactly one automatic LAN-interface refresh",
);
const djLinkEnabledHandler = appText.match(
  /onDjLinkEnabled=\{\(value\)\s*=>\s*\{([\s\S]*?)\n\s*\}\}/,
);
assert(djLinkEnabledHandler, "DJ Link enabled handler was not found");
assert.doesNotMatch(
  djLinkEnabledHandler[1],
  /refreshDjLinkLanInterfaces/,
  "DJ Link enabled handler must leave automatic refresh to the reactive effect",
);
assert.match(
  appText,
  /djLinkLanInterfaceRequestGeneration\s*\+=\s*1;\s*setDjLinkLanInterfaces\(\[\]\)/,
  "disabling DJ Link must retire in-flight interface refresh write authority",
);
for (const [generation, minimumGuards] of [
  ["remoteAccessUrlRequestGeneration", 3],
  ["djLinkLanInterfaceRequestGeneration", 3],
]) {
  assert.match(appText, new RegExp(`let\\s+${generation}\\s*=\\s*0`), `${generation} must be initialized`);
  const guards = [...appText.matchAll(new RegExp(`requestGeneration\\s*===\\s*${generation}`, "g"))];
  assert(
    guards.length >= minimumGuards,
    `${generation} must guard success, failure, and unavailable writes`,
  );
}

const deferred = () => {
  let resolve;
  let reject;
  const promise = new Promise((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
};
const runLatestGenerationRegression = async (label) => {
  let requestGeneration = 0;
  let visible = [];
  const refresh = async (request) => {
    const generation = ++requestGeneration;
    try {
      const next = await request;
      if (generation === requestGeneration) visible = next;
      return next;
    } catch {
      if (generation === requestGeneration) visible = [];
      return [];
    }
  };

  const older = deferred();
  const newer = deferred();
  const olderRefresh = refresh(older.promise);
  const newerRefresh = refresh(newer.promise);
  newer.resolve([`${label}-newer`]);
  await newerRefresh;
  older.resolve([`${label}-older`]);
  await olderRefresh;
  assert.deepEqual(visible, [`${label}-newer`], `${label}: stale success overwrote newer state`);

  const staleFailure = deferred();
  const currentSuccess = deferred();
  const staleRefresh = refresh(staleFailure.promise);
  const currentRefresh = refresh(currentSuccess.promise);
  currentSuccess.resolve([`${label}-current`]);
  await currentRefresh;
  staleFailure.reject(new Error(`${label} stale failure`));
  await staleRefresh;
  assert.deepEqual(visible, [`${label}-current`], `${label}: stale failure cleared newer state`);

  const retired = deferred();
  const retiredRefresh = refresh(retired.promise);
  requestGeneration += 1;
  visible = [];
  retired.resolve([`${label}-retired`]);
  await retiredRefresh;
  assert.deepEqual(visible, [], `${label}: retired request wrote after local invalidation`);
};
await runLatestGenerationRegression("remote URLs");
await runLatestGenerationRegression("DJ LAN interfaces");

const locationOf = (sourceFile, node) => {
  const { line, character } = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
  return `${path.relative(appRoot, sourceFile.fileName)}:${line + 1}:${character + 1}`;
};

const canonicalSymbol = (original) => {
  let symbol = original;
  const seen = new Set();
  while (symbol && (symbol.flags & ts.SymbolFlags.Alias) !== 0 && !seen.has(symbol)) {
    seen.add(symbol);
    const target = checker.getAliasedSymbol(symbol);
    if (!target || target === symbol) break;
    symbol = target;
  }
  return symbol;
};

const rawInvokeSymbols = new Set();
const addRawSymbol = (symbol) => {
  if (!symbol) return false;
  let changed = false;
  for (const candidate of [symbol, canonicalSymbol(symbol)]) {
    if (candidate && !rawInvokeSymbols.has(candidate)) {
      rawInvokeSymbols.add(candidate);
      changed = true;
    }
  }
  return changed;
};
const rawSymbolTracked = (symbol) => Boolean(
  symbol && (rawInvokeSymbols.has(symbol) || rawInvokeSymbols.has(canonicalSymbol(symbol))),
);

for (const sourceFile of sourceFiles) {
  for (const statement of sourceFile.statements) {
    if (!ts.isImportDeclaration(statement)
      || !ts.isStringLiteral(statement.moduleSpecifier)
      || statement.moduleSpecifier.text !== "@tauri-apps/api/core") continue;
    const bindings = statement.importClause?.namedBindings;
    if (!bindings || !ts.isNamedImports(bindings)) continue;
    for (const binding of bindings.elements) {
      if ((binding.propertyName?.text ?? binding.name.text) === "invoke") {
        addRawSymbol(checker.getSymbolAtLocation(binding.name));
      }
    }
  }
}

let aliasesChanged = true;
while (aliasesChanged) {
  aliasesChanged = false;
  for (const sourceFile of sourceFiles) {
    const visit = (node) => {
      if (ts.isVariableDeclaration(node) && ts.isIdentifier(node.name) && node.initializer) {
        const initializer = node.initializer;
        const symbol = ts.isIdentifier(initializer) ? checker.getSymbolAtLocation(initializer) : null;
        if (rawSymbolTracked(symbol)) {
          aliasesChanged = addRawSymbol(checker.getSymbolAtLocation(node.name)) || aliasesChanged;
        }
      }
      ts.forEachChild(node, visit);
    };
    visit(sourceFile);
  }
}

const parameterDeclaresFrontendCommand = (declaration) => {
  const typeNode = declaration?.type;
  if (!typeNode || !ts.isTypeReferenceNode(typeNode)) return false;
  const name = ts.isIdentifier(typeNode.typeName) ? typeNode.typeName.text : typeNode.typeName.right.text;
  if (name === "FrontendTauriInvokeCommand") return true;
  return canonicalSymbol(checker.getSymbolAtLocation(typeNode.typeName))?.name === "FrontendTauriInvokeCommand";
};
const typeIsFrontendInvoke = (type) => type.getCallSignatures().some((signature) => {
  const parameter = signature.getParameters()[0];
  if (!parameter) return false;
  const declarations = parameter.declarations ?? (parameter.valueDeclaration ? [parameter.valueDeclaration] : []);
  if (declarations.some(parameterDeclaresFrontendCommand)) return true;
  const declaration = parameter.valueDeclaration ?? declarations[0];
  if (!declaration) return false;
  const parameterType = checker.getTypeOfSymbolAtLocation(parameter, declaration);
  return parameterType.aliasSymbol?.name === "FrontendTauriInvokeCommand"
    || checker.typeToString(parameterType) === "FrontendTauriInvokeCommand";
});

const finiteStringLiterals = (originalType, seen = new Set()) => {
  const type = originalType.flags & ts.TypeFlags.TypeParameter
    ? checker.getBaseConstraintOfType(originalType) ?? originalType
    : originalType;
  if (seen.has(type)) return null;
  seen.add(type);
  if (type.isStringLiteral()) return new Set([type.value]);
  if (!type.isUnion()) return null;
  const values = new Set();
  for (const member of type.types) {
    const memberValues = finiteStringLiterals(member, seen);
    if (!memberValues) return null;
    for (const value of memberValues) values.add(value);
  }
  return values;
};

const argumentDeclaresBroadFrontendCommand = (argument) => {
  if (!ts.isIdentifier(argument)) return false;
  const symbol = checker.getSymbolAtLocation(argument);
  return Boolean(symbol?.declarations?.some(parameterDeclaresFrontendCommand));
};

const setLiteral = (name) => {
  let result = null;
  const visit = (node) => {
    if (ts.isVariableDeclaration(node)
      && ts.isIdentifier(node.name)
      && node.name.text === name
      && node.initializer
      && ts.isNewExpression(node.initializer)) {
      const values = node.initializer.arguments?.[0];
      if (values && ts.isArrayLiteralExpression(values)) {
        result = values.elements.map((entry) => {
          assert(ts.isStringLiteralLike(entry), `${name} must contain only string literals`);
          return entry.text;
        });
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(appSource);
  assert(result, `${name} was not found in App.tsx`);
  return result;
};

const rendererMutations = setLiteral("projectMutationCommands");
const serverMutations = setLiteral("serverAuthoritativeProjectMutationCommands");
const rendererMutationSet = new Set(rendererMutations);
assert.equal(new Set(rendererMutations).size, rendererMutations.length, "renderer mutation classification contains duplicates");
assert.equal(new Set(serverMutations).size, serverMutations.length, "server-authoritative classification contains duplicates");
for (const command of [...rendererMutations, ...serverMutations]) {
  assert(manifestSet.has(command), `mutation classification is outside the frontend manifest: ${command}`);
}
const overlap = rendererMutations.filter((command) => new Set(serverMutations).has(command));
assert.deepEqual(overlap, [], "renderer and server-authoritative mutation classifications must be disjoint");
assert(serverMutations.includes("set_project_control_mappings"), "project mapping persistence must use the server-authoritative facade lane");

const controlPlaneSource = fs.readFileSync(controlPlanePath, "utf8");
const rustClassification = (functionName, nextMarker) => {
  const body = functionSlice(
    controlPlaneSource,
    `fn ${functionName}(command: &str) -> bool`,
    nextMarker,
  );
  return [...body.matchAll(/"([a-z0-9_]+)"/g)].map((match) => match[1]);
};
const backendServerMutations = rustClassification(
  "is_backend_authoritative_project_mutation",
  "fn is_renderer_ticketed_project_mutation",
);
const backendRendererMutations = rustClassification(
  "is_renderer_ticketed_project_mutation",
  "#[derive(Debug, Clone, PartialEq, Eq)]",
);
// The two atomic batch routes intentionally advance the backend inventory
// from 476/131 to 478/133 and the used-by-frontend manifest from 415 to 417.
// The legacy single-operation routes stay registered.
assert.equal(manifest.length, 417, "frontend Tauri manifest count drifted");
assert.equal(backendRendererMutations.length, 133, "backend renderer-ticketed classification count drifted");
assert.equal(backendServerMutations.length, 29, "backend authoritative classification count drifted");
assert.deepEqual(
  [...rendererMutations].sort(),
  [...backendRendererMutations].sort(),
  "frontend renderer-ticketed classification differs from control_plane.rs",
);
assert.deepEqual(
  [...serverMutations].sort(),
  [...backendServerMutations].sort(),
  "frontend server-authoritative classification differs from control_plane.rs",
);

const rendererDispatchBody = functionSlice(
  appText,
  "const invoke = async <T,>(",
  "const listen = <T,>",
);
for (const marker of [
  "projectMutationCommands.has(command)",
  "projectTransactionId: transaction.transaction_id",
  "expectedEpoch: transaction.project_epoch",
  "ownerId: projectTransactionOwnerId",
]) {
  assert(rendererDispatchBody.includes(marker), `central invoke facade is missing ${marker}`);
}
const legacySingleCueCommands = [
  "update_cue_from_current",
  "set_cue_list",
  "set_cue_metadata",
  "move_cue",
];
for (const command of legacySingleCueCommands) {
  assert(rendererMutationSet.has(command), `${command} must remain classified renderer-ticketed`);
  assert.doesNotMatch(
    appText,
    new RegExp(`tauriInvoke(?:\\s*<[^>]+>)?\\s*\\(\\s*["']${command}["']`),
    `${command} must not use a raw literal renderer dispatch`,
  );
}

const atomicBatchCommands = [
  "update_cue_from_current_batch",
  "move_cue_between_scene_banks_batch",
];
for (const command of atomicBatchCommands) {
  assert(rendererMutationSet.has(command), `${command} must be classified renderer-ticketed`);
  assert(manifestSet.has(command), `${command} must be present in the frontend manifest`);
}
const controlEditBatchBody = functionSlice(
  appText,
  "const runControlEditLookUpdate = async",
  "const flushControlEditLookUpdates",
);
assert.equal(
  [...controlEditBatchBody.matchAll(/invoke\s*\(\s*["']update_cue_from_current_batch["']/g)].length,
  1,
  "control Edit capture must issue one atomic batch dispatch",
);
assert.doesNotMatch(controlEditBatchBody, /beginProjectTransaction|update_cue_from_current["']/,
  "control Edit capture must not open a transaction or dispatch the legacy per-scope command");
assert.match(controlEditBatchBody, /captureScopes\s*:\s*captureTargets/,
  "control Edit batch must preserve all capture scopes in one payload");

const sceneMatrixBatchBody = functionSlice(
  appText,
  "const runSceneMatrixBankMoveTransaction = async",
  "const reorderSceneMatrixCue",
);
assert.equal(
  [...sceneMatrixBatchBody.matchAll(/invoke\s*\(\s*["']move_cue_between_scene_banks_batch["']/g)].length,
  1,
  "Scene Matrix cross-bank move must issue one atomic batch dispatch",
);
assert.doesNotMatch(sceneMatrixBatchBody, /beginProjectTransaction|["'](?:set_cue_list|set_cue_metadata|move_cue)["']/,
  "Scene Matrix cross-bank move must not reuse a ticket across legacy commands");
for (const field of ["targetCueListId", "targetGroupId", "targetCueId", "position"]) {
  assert.match(sceneMatrixBatchBody, new RegExp(`\\b${field}\\s*:`), `Scene Matrix batch must send ${field}`);
}
const sceneMatrixReorderBody = functionSlice(
  appText,
  "const reorderSceneMatrixCue = async",
  "const beginTimelineCueDrag",
);
assert.match(
  sceneMatrixReorderBody,
  /const\s+resolvedPosition\s*=\s*targetCueId\s*===\s*null\s*\?\s*["']after["']\s*:\s*position/,
  "Scene Matrix null target must normalize to destination-end position after",
);
assert.match(sceneMatrixReorderBody, /runSceneMatrixBankMoveTransaction\([\s\S]*?targetCueId,\s*resolvedPosition,/,
  "Scene Matrix native batch must receive the normalized exact insertion intent");
assert.match(sceneMatrixReorderBody, /applySceneMatrixCueListMove\([\s\S]*?targetCueId,[\s\S]*?resolvedPosition,/,
  "Scene Matrix browser projection must receive the same normalized exact insertion intent");

const applySceneMatrixIntentFixture = (targetCueId, position) => {
  const next = [
    { id: 301, cueListId: 2 },
    { id: 302, cueListId: 2 },
    { id: 303, cueListId: 3 },
    { id: 320, cueListId: 3 },
  ];
  const sourceIndex = next.findIndex((cue) => cue.id === 302);
  const [source] = next.splice(sourceIndex, 1);
  source.cueListId = 3;
  const resolvedPosition = targetCueId === null ? "after" : position;
  const targetIndex = targetCueId === null
    ? next.reduce((last, cue, index) => cue.cueListId === 3 ? index : last, -1)
    : next.findIndex((cue) => cue.id === targetCueId);
  next.splice(resolvedPosition === "after" ? targetIndex + 1 : targetIndex, 0, source);
  return { resolvedPosition, order: next.filter((cue) => cue.cueListId === 3).map((cue) => cue.id) };
};
assert.deepEqual(applySceneMatrixIntentFixture(303, "before"), {
  resolvedPosition: "before",
  order: [302, 303, 320],
}, "Scene Matrix native/browser fixture must preserve before insertion");
assert.deepEqual(applySceneMatrixIntentFixture(303, "after"), {
  resolvedPosition: "after",
  order: [303, 302, 320],
}, "Scene Matrix native/browser fixture must preserve after insertion");
assert.deepEqual(applySceneMatrixIntentFixture(null, "before"), {
  resolvedPosition: "after",
  order: [303, 320, 302],
}, "Scene Matrix null-target fixture must normalize to exact destination-end insertion");

const rendererLane = { dispatchCount: 0, sealed: false };
const dispatchRendererAttempt = () => {
  if (rendererLane.sealed) throw new Error("renderer transaction lane already sealed");
  rendererLane.dispatchCount += 1;
  rendererLane.sealed = true;
};
dispatchRendererAttempt();
assert.equal(rendererLane.dispatchCount, 1, "atomic batch must consume exactly one renderer dispatch");
assert.throws(dispatchRendererAttempt, /already sealed/, "a second renderer dispatch must fail the negative fixture");

assert.match(
  appText,
  /invoke\s*<\s*ProjectControlMappingsAuthority\s*>\s*\(\s*"set_project_control_mappings"/,
  "project mapping persistence must dispatch through the central invoke facade",
);
assert.doesNotMatch(
  appText,
  /tauriInvoke\s*<[^>]*>\s*\(\s*"set_project_control_mappings"/,
  "project mapping persistence must not bypass the central invoke facade",
);
const mappingCall = appText.match(
  /invoke\s*<\s*ProjectControlMappingsAuthority\s*>\s*\(\s*"set_project_control_mappings"\s*,\s*\{([\s\S]*?)\}\s*\)/,
);
assert(mappingCall, "project mapping persistence call was not found");
for (const field of ["__expectedProjectEpoch", "ownerId", "expectedEpoch", "expectedRevision"]) {
  assert.match(mappingCall[1], new RegExp(`\\b${field}\\s*:`), `project mapping persistence must send ${field}`);
}

for (const relative of [
  path.join("components", "FixtureCatalogPanel.tsx"),
  path.join("components", "PatchProfileBrowserPanel.tsx"),
]) {
  const component = sourceFiles.find(
    (sourceFile) => path.relative(srcRoot, sourceFile.fileName) === relative,
  );
  assert(component, `${relative} must be part of the frontend TypeScript program`);
  const text = component.getFullText();
  assert.doesNotMatch(text, /from\s+["']@tauri-apps\/api\/core["']/, `${relative} must not import raw Tauri invoke`);
  assert.match(
    text,
    /props\.invokeCommand\s*<[^>]+>\s*\(\s*"cache_gdtf_from_share"/,
    `${relative} must route cache_gdtf_from_share through the injected App facade`,
  );
}

const requiredAuthorityFields = new Map([
  ["new_project", ["ownerId", "expectedEpoch", "expectedRevision", "expectedCheckpointHash"]],
  ["load_user_template", ["ownerId", "expectedEpoch", "expectedRevision", "expectedCheckpointHash"]],
  ["load_project", ["ownerId", "expectedEpoch", "expectedRevision", "expectedCheckpointHash"]],
  ["import_daslight_project_with_result", ["ownerId", "expectedEpoch", "expectedRevision", "expectedCheckpointHash"]],
  ["load_project_path", ["ownerId", "expectedEpoch", "expectedRevision", "expectedCheckpointHash"]],
  ["load_project_backup", ["ownerId", "expectedEpoch", "expectedRevision", "expectedCheckpointHash"]],
  ["load_startup_project", ["ownerId", "expectedEpoch", "expectedRevision", "expectedCheckpointHash"]],
  ["clear_project_history", ["ownerId", "expectedEpoch", "expectedHistoryGeneration"]],
  ["undo_project_transaction", ["ownerId", "expectedEpoch", "expectedEntryId", "expectedCheckpointHash"]],
  ["redo_project_transaction", ["ownerId", "expectedEpoch", "expectedEntryId", "expectedCheckpointHash"]],
]);
const seenAuthorityCommands = new Set();
const auditAuthorityCalls = (node) => {
  if (ts.isCallExpression(node)
    && node.arguments.length >= 1
    && ts.isStringLiteralLike(node.arguments[0])
    && requiredAuthorityFields.has(node.arguments[0].text)) {
    const command = node.arguments[0].text;
    assert(!seenAuthorityCommands.has(command), `${command} must have one auditable frontend callsite`);
    seenAuthorityCommands.add(command);
    const args = node.arguments[1];
    assert(args && ts.isObjectLiteralExpression(args), `${command} must send an explicit authority object`);
    const fields = new Set(args.properties.flatMap((property) => {
      if (!ts.isPropertyAssignment(property) && !ts.isShorthandPropertyAssignment(property)) return [];
      const name = property.name;
      return ts.isIdentifier(name) || ts.isStringLiteralLike(name) ? [name.text] : [];
    }));
    for (const field of requiredAuthorityFields.get(command)) {
      assert(fields.has(field), `${command} must send ${field}`);
    }
  }
  ts.forEachChild(node, auditAuthorityCalls);
};
auditAuthorityCalls(appSource);
for (const command of requiredAuthorityFields.keys()) {
  assert(seenAuthorityCommands.has(command), `${command} authority-fenced frontend callsite was not found`);
}

const outputChecker = fs.readFileSync(path.join(appRoot, "scripts", "check-output-ownership.mjs"), "utf8");
const legacyStart = outputChecker.indexOf("const legacyTauriOutputRoutes = [");
const legacyEnd = outputChecker.indexOf("];", legacyStart);
assert(legacyStart >= 0 && legacyEnd > legacyStart, "legacy output route inventory was not found");
const legacyOutputRoutes = new Set(
  [...outputChecker.slice(legacyStart, legacyEnd).matchAll(/"([a-z0-9_]+)"/g)].map((match) => match[1]),
);

const allowedRawInvokeFiles = new Set([appPath, detachedVideoPath]);
const machineFileMutations = new Set(["cache_gdtf_from_share"]);
const broadCastPattern = /^(?:Parameters\s*<\s*FrontendTauriInvoke\s*>\s*\[\s*0\s*\]|FrontendTauriInvokeCommand)$/;
const isBroadCommandCast = (node, sourceFile) => {
  if (!ts.isAsExpression(node) && !ts.isTypeAssertionExpression(node)) return false;
  return broadCastPattern.test(node.type.getText(sourceFile).replace(/\s+/g, ""));
};
const selfTestSource = ts.createSourceFile(
  "wide-cast.ts",
  "invoke(command as Parameters<FrontendTauriInvoke>[0]);",
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.TS,
);
let selfTestRejected = false;
const selfTestVisit = (node) => {
  if (isBroadCommandCast(node, selfTestSource)) selfTestRejected = true;
  ts.forEachChild(node, selfTestVisit);
};
selfTestVisit(selfTestSource);
assert(selfTestRejected, "wide frontend-command cast negative fixture was not rejected");

const rawRendererNegativeSource = ts.createSourceFile(
  "raw-renderer-ticketed.ts",
  [...legacySingleCueCommands, ...atomicBatchCommands]
    .map((command) => `tauriInvoke("${command}", {});`)
    .join("\n"),
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.TS,
);
const rawRendererNegativeCommands = [];
const rawRendererNegativeVisit = (node) => {
  if (ts.isCallExpression(node)
    && ts.isIdentifier(node.expression)
    && node.expression.text === "tauriInvoke"
    && ts.isStringLiteralLike(node.arguments[0])) {
    rawRendererNegativeCommands.push(node.arguments[0].text);
  }
  ts.forEachChild(node, rawRendererNegativeVisit);
};
rawRendererNegativeVisit(rawRendererNegativeSource);
assert.deepEqual(
  rawRendererNegativeCommands.filter((command) => rendererMutationSet.has(command)),
  [...legacySingleCueCommands, ...atomicBatchCommands],
  "raw renderer-ticketed negative fixture did not reject the cue mutation routes",
);

const tauriInternalsNegativeSource = ts.createSourceFile(
  "tauri-internals.ts",
  "window.__TAURI_INTERNALS__.invoke(command, args); window['__TAURI_INTERNALS__']['invoke'](command, args);",
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.TS,
);
const accessPropertyName = (node) => {
  if (ts.isPropertyAccessExpression(node)) return node.name.text;
  if (ts.isElementAccessExpression(node) && node.argumentExpression
    && ts.isStringLiteralLike(node.argumentExpression)) return node.argumentExpression.text;
  return null;
};
const accessReceiverContainsProperty = (original, propertyName) => {
  let current = original;
  while (ts.isPropertyAccessExpression(current) || ts.isElementAccessExpression(current)) {
    if (accessPropertyName(current) === propertyName) return true;
    current = current.expression;
  }
  return ts.isIdentifier(current) && current.text === propertyName;
};
const isTauriInternalsInvoke = (node) => {
  if (!ts.isCallExpression(node)) return false;
  const callee = node.expression;
  if ((!ts.isPropertyAccessExpression(callee) && !ts.isElementAccessExpression(callee))
    || accessPropertyName(callee) !== "invoke") return false;
  return accessReceiverContainsProperty(callee.expression, "__TAURI_INTERNALS__");
};
let tauriInternalsNegativeCount = 0;
const tauriInternalsNegativeVisit = (node) => {
  if (isTauriInternalsInvoke(node)) tauriInternalsNegativeCount += 1;
  ts.forEachChild(node, tauriInternalsNegativeVisit);
};
tauriInternalsNegativeVisit(tauriInternalsNegativeSource);
assert.equal(tauriInternalsNegativeCount, 2, "Tauri internals invoke negative fixture did not reject dot and bracket access");

const enclosingDispatcherName = (node) => {
  let current = node.parent;
  while (current) {
    if ((ts.isArrowFunction(current) || ts.isFunctionExpression(current))
      && ts.isVariableDeclaration(current.parent)
      && ts.isIdentifier(current.parent.name)) return current.parent.name.text;
    if (ts.isFunctionDeclaration(current) && current.name) return current.name.text;
    current = current.parent;
  }
  return null;
};
const approvedRawDynamicDispatchers = new Map([
  [appPath, new Set([
    "invoke",
    "invokeTimelineFollowAbortRuntime",
    "invokeProjectPublicationCommand",
    "invokeSafetyBlackoutRuntime",
    "invokeTimelineTransportRuntime",
  ])],
  [detachedVideoPath, new Set(["invoke"])],
]);

const sourceContract = (source, marker, nextMarker, required, forbidden = []) => {
  const body = functionSlice(source, marker, nextMarker);
  for (const value of required) assert(body.includes(value), `${marker} is missing finite dispatcher contract ${value}`);
  for (const value of forbidden) assert(!body.includes(value), `${marker} admits forbidden dispatcher route ${value}`);
  return body;
};
sourceContract(appText, "const invokeTimelineFollowAbortRuntime", "const timelineFollowAbortRuntime", [
  '"query_timeline_follow_abort_authority_v1"',
  '"abort_timeline_follow_runtime_v1"',
]);
sourceContract(appText, "const invokeSafetyBlackoutRuntime", "const safetyBlackoutRuntime", [
  '"safety_blackout_engage_v1"',
]);
sourceContract(appText, "const invokeTimelineTransportRuntime", "const timelineTransportRuntime", [
  '"query_timeline_transport_authority_v1"',
  '"set_timeline_transport_playing_runtime_v1"',
]);
const publicationDispatcher = sourceContract(
  appText,
  "const invokeProjectPublicationCommand",
  "const acknowledgeProjectPublicationRequest",
  ["ProjectPublicationCommandV1", "ensureProjectPublicationMutationAllowed"],
);
assert.match(publicationDispatcher, /command\s*:\s*ProjectPublicationCommandV1/,
  "project publication raw dispatcher must be constrained by its finite command union");
const detachedVideoText = fs.readFileSync(detachedVideoPath, "utf8");
assert.match(detachedVideoText, /get_debug_video_output_(?:test_pattern|preview)/,
  "detached video raw dispatcher must remain limited to debug image reads");
assert.doesNotMatch(detachedVideoText, /(?:set_|add_|remove_|update_|create_|delete_)[a-z0-9_]+/,
  "detached video raw dispatcher must not grow mutation routes");

const errors = [];
const observed = { raw: 0, injected: 0, broadDispatchers: 0 };
for (const sourceFile of sourceFiles) {
  const resolvedFile = path.resolve(sourceFile.fileName);
  const visit = (node) => {
    if (isTauriInternalsInvoke(node)) {
      errors.push(`${locationOf(sourceFile, node)} window.__TAURI_INTERNALS__.invoke bypasses typed Tauri routing`);
    }
    if (isBroadCommandCast(node, sourceFile)) {
      errors.push(`${locationOf(sourceFile, node)} broad FrontendTauriInvoke command cast hides the actual route`);
    }
    if (ts.isImportDeclaration(node)
      && ts.isStringLiteral(node.moduleSpecifier)
      && node.moduleSpecifier.text === "@tauri-apps/api/core"
      && !allowedRawInvokeFiles.has(resolvedFile)) {
      errors.push(`${locationOf(sourceFile, node)} raw Tauri invoke import must use the injected App facade`);
    }
    if (ts.isCallExpression(node) && node.arguments[0]) {
      const callee = node.expression;
      const calleeSymbol = checker.getSymbolAtLocation(ts.isPropertyAccessExpression(callee) ? callee.name : callee);
      const raw = rawSymbolTracked(calleeSymbol);
      const injected = !raw && typeIsFrontendInvoke(checker.getTypeAtLocation(callee));
      if (raw || injected) {
        if (raw) observed.raw += 1;
        else observed.injected += 1;
        const argument = node.arguments[0];
        let commands = ts.isStringLiteralLike(argument)
          ? new Set([argument.text])
          : finiteStringLiterals(checker.getTypeAtLocation(argument));
        if (argumentDeclaresBroadFrontendCommand(argument)
          || commands && commands.size === manifest.length
          && [...commands].every((command) => manifestSet.has(command))) {
          observed.broadDispatchers += 1;
          commands = null;
        }
        if (commands) {
          for (const command of commands) {
            if (legacyOutputRoutes.has(command)) {
              errors.push(`${locationOf(sourceFile, argument)} frontend invokes fail-closed legacy output route ${command}`);
            }
            if (raw && rendererMutationSet.has(command)) {
              errors.push(`${locationOf(sourceFile, argument)} raw renderer-ticketed mutation ${command} bypasses the approved transaction helper`);
            }
            if (machineFileMutations.has(command) && raw) {
              errors.push(`${locationOf(sourceFile, argument)} machine/file mutation ${command} bypasses the App facade`);
            }
          }
        } else if (raw) {
          const dispatcher = enclosingDispatcherName(node);
          const allowlist = approvedRawDynamicDispatchers.get(resolvedFile);
          if (!dispatcher || !allowlist?.has(dispatcher)) {
            errors.push(`${locationOf(sourceFile, node)} raw dynamic dispatcher ${dispatcher ?? "<unknown>"} is outside the finite allowlist`);
          }
        }
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
}

if (errors.length > 0) {
  console.error(errors.join("\n"));
  process.exitCode = 1;
} else {
  console.log(
    `frontend command routing exact: ${rendererMutations.length} renderer mutations, ${serverMutations.length} server-authoritative mutations, ${observed.raw} raw dispatches, ${observed.injected} facade dispatches`,
  );
}

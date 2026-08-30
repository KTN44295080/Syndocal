import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const config = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const nativeAcceptanceConfig = JSON.parse(
  await readFile(new URL("../src-tauri/tauri.native-acceptance.conf.json", import.meta.url), "utf8"),
);
const f11FocusConfig = JSON.parse(
  await readFile(new URL("../src-tauri/tauri.f11-focus-qa.conf.json", import.meta.url), "utf8"),
);
const capability = JSON.parse(
  await readFile(new URL("../src-tauri/capabilities/main.json", import.meta.url), "utf8"),
);
const source = await readFile(new URL("../src/desktopWindowMode.ts", import.meta.url), "utf8");
const keyboardController = await readFile(
  new URL("../src/createAppKeyboardController.ts", import.meta.url),
  "utf8",
);
const controller = await readFile(
  new URL("../src/components/DesktopWindowModeController.tsx", import.meta.url),
  "utf8",
);
const workspaceChrome = await readFile(
  new URL("../src/components/WorkspaceChrome.tsx", import.meta.url),
  "utf8",
);
const styles = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");
const main = await readFile(new URL("../src/main.tsx", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const uiLocalization = await readFile(new URL("../src/uiLocalization.ts", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const nativeAcceptance = await readFile(
  new URL("./check-native-window-acceptance.ps1", import.meta.url),
  "utf8",
);
const appSourceFile = ts.createSourceFile(
  "App.tsx",
  app,
  ts.ScriptTarget.ES2022,
  true,
  ts.ScriptKind.TSX,
);
const projectReadInitializationHelpers = [
  "captureProjectAuthorityIdentity",
  "isProjectAuthorityIdentityCurrent",
  "beginProjectReadGeneration",
  "captureProjectReadGuard",
  "projectReadGuardIsCurrent",
];
const projectReadHelperDeclarations = new Map(
  projectReadInitializationHelpers.map((name) => [name, []]),
);
const projectReadHelperCalls = new Map(
  projectReadInitializationHelpers.map((name) => [name, []]),
);
const visitProjectReadInitialization = (node) => {
  if (
    ts.isVariableDeclaration(node)
    && ts.isIdentifier(node.name)
    && projectReadHelperDeclarations.has(node.name.text)
  ) {
    projectReadHelperDeclarations.get(node.name.text).push(node.getStart(appSourceFile));
  }
  if (
    ts.isCallExpression(node)
    && ts.isIdentifier(node.expression)
    && projectReadHelperCalls.has(node.expression.text)
  ) {
    projectReadHelperCalls.get(node.expression.text).push(node.getStart(appSourceFile));
  }
  ts.forEachChild(node, visitProjectReadInitialization);
};
visitProjectReadInitialization(appSourceFile);
for (const helper of projectReadInitializationHelpers) {
  const declarations = projectReadHelperDeclarations.get(helper);
  const calls = projectReadHelperCalls.get(helper);
  assert.equal(declarations.length, 1, `${helper} must have exactly one lexical declaration`);
  assert.ok(calls.length > 0, `${helper} must remain exercised by App`);
  assert.ok(
    declarations[0] < Math.min(...calls),
    `${helper} must be initialized before every App call to prevent a native-startup TDZ`,
  );
}
const mainRuntimeActiveStart = app.indexOf("  const mainRuntimeActive = (current: EngineSnapshot) => {");
const protectedCloseRequestStart = app.indexOf(
  "  const protectedCloseRequestForCurrentState = (current: EngineSnapshot = latestEngineSnapshot): ProtectedCloseRequest | null => {",
);
const protectedCloseUnknownRequestStart = app.indexOf(
  "  const protectedCloseUnknownRequest = (): MainProtectedCloseRequest => ({",
);
const closeProtectionRequiredStart = app.indexOf("  const closeProtectionRequired = () =>");
const protectedCloseMainDetailStart = app.indexOf(
  "  const protectedCloseMainDetail = (request: MainProtectedCloseRequest) => {",
);
const protectedCloseCopyStart = app.indexOf("  const protectedCloseCopy = () => {");
const scheduleApprovedNativeCloseStart = app.indexOf("  const scheduleApprovedNativeClose = () => {");
const protectedCloseRefreshStart = app.indexOf(
  "  const refreshSnapshotForProtectedClose = async (): Promise<EngineSnapshot | null> => {",
);
const protectedCloseCleanupStart = app.indexOf("  onCleanup(() => {", protectedCloseCopyStart);
const protectedCloseCleanupEnd = app.indexOf("  });", protectedCloseCleanupStart);
assert.ok(
  mainRuntimeActiveStart >= 0 &&
    protectedCloseRequestStart > mainRuntimeActiveStart &&
    protectedCloseUnknownRequestStart > protectedCloseRequestStart &&
    closeProtectionRequiredStart > mainRuntimeActiveStart &&
    protectedCloseMainDetailStart > closeProtectionRequiredStart &&
    protectedCloseCopyStart > protectedCloseMainDetailStart &&
    scheduleApprovedNativeCloseStart > protectedCloseCopyStart &&
    protectedCloseRefreshStart > protectedCloseCopyStart &&
    protectedCloseCleanupStart > protectedCloseRefreshStart &&
    protectedCloseCleanupEnd > protectedCloseCleanupStart,
  "protected-close predicates must remain statically discoverable",
);
const mainRuntimeActiveSource = app.slice(mainRuntimeActiveStart, closeProtectionRequiredStart);
const protectedCloseRequestSource = app.slice(protectedCloseRequestStart, closeProtectionRequiredStart);
const protectedCloseUnknownRequestSource = app.slice(protectedCloseUnknownRequestStart, closeProtectionRequiredStart);
const closeProtectionRequiredSource = app.slice(closeProtectionRequiredStart, protectedCloseMainDetailStart);
const protectedCloseMainDetailSource = app.slice(protectedCloseMainDetailStart, protectedCloseCopyStart);
const protectedCloseCopySource = app.slice(protectedCloseCopyStart, protectedCloseCleanupStart);
const scheduleApprovedNativeCloseSource = app.slice(scheduleApprovedNativeCloseStart, protectedCloseRefreshStart);
const protectedCloseRefreshSource = app.slice(protectedCloseRefreshStart, protectedCloseCleanupStart);
const protectedCloseCleanupSource = app.slice(protectedCloseCleanupStart, protectedCloseCleanupEnd + "  });".length);
const protectedCloseSource = app.slice(mainRuntimeActiveStart, protectedCloseCleanupStart);
const closeRequestedStart = app.indexOf("      .onCloseRequested(async (event) => {");
const closeRequestedEnd = app.indexOf("      .then((unlisten)", closeRequestedStart);
assert.ok(closeRequestedStart >= 0 && closeRequestedEnd > closeRequestedStart, "CloseRequested source must remain discoverable");
const closeRequestedSource = app.slice(closeRequestedStart, closeRequestedEnd);

const unwrapExpression = (expression) => {
  let current = expression;
  while (ts.isParenthesizedExpression(current)) current = current.expression;
  return current;
};

const isIdentifierNamed = (node, name) => ts.isIdentifier(node) && node.text === name;
const isNullLiteral = (node) => node.kind === ts.SyntaxKind.NullKeyword;
const isBooleanLiteral = (node, value) => value
  ? node.kind === ts.SyntaxKind.TrueKeyword
  : node.kind === ts.SyntaxKind.FalseKeyword;
const isCallNamed = (node, name) => {
  const expression = unwrapExpression(node);
  return ts.isCallExpression(expression) && isIdentifierNamed(expression.expression, name);
};
const isAwaitedCallNamed = (node, name) => {
  const expression = unwrapExpression(node);
  return ts.isAwaitExpression(expression) && isCallNamed(expression.expression, name);
};
const statementIsReturn = (statement) => ts.isReturnStatement(statement);
const blockStatements = (statement) => ts.isBlock(statement) ? [...statement.statements] : [statement];
const blockReturnsImmediately = (statement) => {
  const statements = blockStatements(statement);
  return statements.length === 1 && statementIsReturn(statements[0]);
};
const isCallStatement = (statement, name) => ts.isExpressionStatement(statement) && isCallNamed(statement.expression, name);
const isPreventDefaultStatement = (statement) => {
  if (!ts.isExpressionStatement(statement)) return false;
  const expression = unwrapExpression(statement.expression);
  return ts.isCallExpression(expression)
    && ts.isPropertyAccessExpression(expression.expression)
    && isIdentifierNamed(expression.expression.expression, "event")
    && expression.expression.name.text === "preventDefault";
};
const isIfCondition = (statement, predicate) => ts.isIfStatement(statement) && predicate(unwrapExpression(statement.expression));
const isTimelinePaneCondition = (expression) => ts.isBinaryExpression(expression)
  && expression.operatorToken.kind === ts.SyntaxKind.EqualsEqualsEqualsToken
  && isIdentifierNamed(expression.left, "paneWindow")
  && ts.isStringLiteral(expression.right)
  && expression.right.text === "timeline";
const isCloseRefreshDuplicateCondition = (expression) => ts.isBinaryExpression(expression)
  && expression.operatorToken.kind === ts.SyntaxKind.BarBarToken
  && isIdentifierNamed(expression.left, "protectedCloseRefreshInFlight")
  && ts.isBinaryExpression(expression.right)
  && expression.right.operatorToken.kind === ts.SyntaxKind.ExclamationEqualsEqualsToken
  && isCallNamed(expression.right.left, "protectedCloseRequest")
  && isNullLiteral(expression.right.right);
const isNegatedIdentifier = (expression, name) => ts.isPrefixUnaryExpression(expression)
  && expression.operator === ts.SyntaxKind.ExclamationToken
  && isIdentifierNamed(expression.operand, name);
const hasVariableInitializer = (statement, name, predicate) => ts.isVariableStatement(statement)
  && [...statement.declarationList.declarations].some((declaration) => isIdentifierNamed(declaration.name, name)
    && declaration.initializer !== undefined
    && predicate(unwrapExpression(declaration.initializer)));
const staticBooleanTruth = (expression) => {
  const current = unwrapExpression(expression);
  if (current.kind === ts.SyntaxKind.TrueKeyword) return true;
  if (current.kind === ts.SyntaxKind.FalseKeyword) return false;
  return undefined;
};
const visitOutsideNestedClosures = (root, onNode) => {
  const walk = (node, isRoot) => {
    onNode(node);
    if (!isRoot && ts.isFunctionLike(node)) return;
    if (node.kind === ts.SyntaxKind.StaticBlock) return;
    if (ts.isIfStatement(node)) {
      const truthiness = staticBooleanTruth(node.expression);
      walk(unwrapExpression(node.expression), false);
      if (truthiness === false) {
        if (node.elseStatement !== undefined) walk(node.elseStatement, false);
      } else if (truthiness === true) {
        walk(node.thenStatement, false);
      } else {
        walk(node.thenStatement, false);
        if (node.elseStatement !== undefined) walk(node.elseStatement, false);
      }
      return;
    }
    if (ts.isConditionalExpression(node)) {
      const truthiness = staticBooleanTruth(node.condition);
      walk(unwrapExpression(node.condition), false);
      if (truthiness === false) {
        walk(node.whenFalse, false);
      } else if (truthiness === true) {
        walk(node.whenTrue, false);
      } else {
        walk(node.whenTrue, false);
        walk(node.whenFalse, false);
      }
      return;
    }
    ts.forEachChild(node, (child) => walk(child, false));
  };
  walk(root, true);
};
const hasAssignment = (node, target, predicate) => {
  let matched = false;
  visitOutsideNestedClosures(node, (child) => {
    if (
      ts.isBinaryExpression(child)
      && child.operatorToken.kind === ts.SyntaxKind.EqualsToken
      && isIdentifierNamed(child.left, target)
      && predicate(unwrapExpression(child.right))
    ) {
      matched = true;
    }
  });
  return matched;
};
const hasCall = (node, name) => {
  let matched = false;
  visitOutsideNestedClosures(node, (child) => {
    if (isCallNamed(child, name)) matched = true;
  });
  return matched;
};
const getVariableAwaitingCall = (statement, name, callName) => ts.isVariableStatement(statement)
  && [...statement.declarationList.declarations].some((declaration) => isIdentifierNamed(declaration.name, name)
    && declaration.initializer !== undefined
    && isAwaitedCallNamed(declaration.initializer, callName));
const isDisposedReturnGuard = (statement) => isIfCondition(statement, (expression) => isIdentifierNamed(expression, "closeRequestListenerDisposed"))
  && blockReturnsImmediately(statement.thenStatement);
const isEnrichedTimelineCloseRequest = (statement) => {
  if (!isCallStatement(statement, "setProtectedCloseRequest")) return false;
  const call = unwrapExpression(statement.expression);
  if (call.arguments.length !== 1 || !ts.isObjectLiteralExpression(call.arguments[0])) return false;
  const properties = [...call.arguments[0].properties];
  const hasCloseRequestSpread = properties.some((property) => ts.isSpreadAssignment(property)
    && isIdentifierNamed(property.expression, "closeRequest"));
  const hasContext = properties.some((property) => ts.isPropertyAssignment(property)
    && ts.isIdentifier(property.name)
    && property.name.text === "pendingClose"
    && isIdentifierNamed(unwrapExpression(property.initializer), "pendingContext"));
  const hasFailure = properties.some((property) => ts.isPropertyAssignment(property)
    && ts.isIdentifier(property.name)
    && property.name.text === "pendingCloseQueryFailed"
    && isIdentifierNamed(unwrapExpression(property.initializer), "pendingCloseQueryFailed"))
    || properties.some((property) => ts.isShorthandPropertyAssignment(property)
      && property.name.text === "pendingCloseQueryFailed");
  return hasCloseRequestSpread && hasContext && hasFailure;
};
const findCloseRequestedHandler = (candidate, label) => {
  const sourceFile = ts.createSourceFile(`${label}.tsx`, candidate, ts.ScriptTarget.ES2022, true, ts.ScriptKind.TSX);
  let handler;
  const visit = (node) => {
    if (
      ts.isCallExpression(node)
      && ts.isPropertyAccessExpression(node.expression)
      && node.expression.name.text === "onCloseRequested"
      && node.arguments.length === 1
      && ts.isArrowFunction(node.arguments[0])
      && node.arguments[0].modifiers?.some((modifier) => modifier.kind === ts.SyntaxKind.AsyncKeyword)
    ) {
      assert.equal(handler, undefined, `${label}: exactly one async CloseRequested handler is required`);
      handler = node.arguments[0];
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  assert.ok(handler !== undefined, `${label}: async CloseRequested handler is required`);
  assert.ok(ts.isBlock(handler.body), `${label}: CloseRequested handler must use a statement block`);
  return handler;
};
const assertCorrelatedTimelineCloseControlFlow = (candidate, label) => {
  const handler = findCloseRequestedHandler(candidate, label);
  const statements = [...handler.body.statements];
  const approvalIndex = statements.findIndex((statement) => isIfCondition(statement, (expression) => isCallNamed(expression, "consumeNativeCloseApproval"))
    && blockReturnsImmediately(statement.thenStatement));
  const timelineIndex = statements.findIndex((statement) => isIfCondition(statement, isTimelinePaneCondition));
  const duplicateIndex = statements.findIndex((statement) => isIfCondition(statement, isCloseRefreshDuplicateCondition));
  const preventBeforeRefreshIndex = statements.findIndex((statement) => isPreventDefaultStatement(statement));
  const refreshIndex = statements.findIndex((statement) => getVariableAwaitingCall(statement, "freshSnapshot", "refreshSnapshotForProtectedClose"));
  assert.ok(approvalIndex === 0, `${label}: an approved close must bypass every remaining CloseRequested branch first`);
  assert.ok(timelineIndex > approvalIndex, `${label}: timeline close handling must follow the approval bypass`);
  assert.ok(duplicateIndex > timelineIndex, `${label}: main duplicate suppression must follow the timeline branch`);
  assert.ok(preventBeforeRefreshIndex > duplicateIndex && refreshIndex > preventBeforeRefreshIndex, `${label}: main close must prevent before its bounded refresh`);

  const timelineStatements = blockStatements(statements[timelineIndex].thenStatement);
  const closeRequestIndex = timelineStatements.findIndex((statement) => hasVariableInitializer(statement, "closeRequest", (expression) => isCallNamed(expression, "protectedCloseRequestForCurrentState")));
  const missingRequestIndex = timelineStatements.findIndex((statement) => isIfCondition(statement, (expression) => isNegatedIdentifier(expression, "closeRequest"))
    && blockReturnsImmediately(statement.thenStatement));
  const preventIndex = timelineStatements.findIndex((statement) => isPreventDefaultStatement(statement));
  const pendingContextIndex = timelineStatements.findIndex((statement) => hasVariableInitializer(statement, "pendingContext", isNullLiteral));
  const queryFailureIndex = timelineStatements.findIndex((statement) => hasVariableInitializer(statement, "pendingCloseQueryFailed", (expression) => isBooleanLiteral(expression, false)));
  const queryTryIndex = timelineStatements.findIndex((statement) => ts.isTryStatement(statement));
  const disposedIndex = timelineStatements.findIndex((statement) => isDisposedReturnGuard(statement));
  const enrichedRequestIndices = timelineStatements
    .map((statement, index) => (isEnrichedTimelineCloseRequest(statement) ? index : -1))
    .filter((index) => index >= 0);
  assert.equal(
    enrichedRequestIndices.length,
    1,
    `${label}: the timeline branch must publish the enriched request as exactly one authoritative post-disposal publish`,
  );
  const enrichedRequestIndex = enrichedRequestIndices[0];
  const postPublishStatements = timelineStatements.slice(enrichedRequestIndex + 1);
  const terminatesAtBarePostPublishReturn = postPublishStatements.length === 1
    && statementIsReturn(postPublishStatements[0])
    && postPublishStatements[0].expression === undefined;
  assert.ok(
    closeRequestIndex >= 0
      && missingRequestIndex > closeRequestIndex
      && preventIndex > missingRequestIndex
      && pendingContextIndex > preventIndex
      && queryFailureIndex > pendingContextIndex
      && queryTryIndex > queryFailureIndex
      && disposedIndex > queryTryIndex
      && enrichedRequestIndex > disposedIndex
      && terminatesAtBarePostPublishReturn,
    `${label}: timeline close must prevent, query, fence disposal, publish the enriched request, then terminate at a bare post-publish return with no dead statements`,
  );

  const queryTry = timelineStatements[queryTryIndex];
  assert.ok(
    hasAssignment(queryTry.tryBlock, "pendingContext", (expression) => isAwaitedCallNamed(expression, "queryOwnPanePendingCloseContext")),
    `${label}: timeline close must await the correlated pending-close query in its try path`,
  );
  assert.ok(queryTry.catchClause !== undefined, `${label}: timeline close must handle pending-close query failure explicitly`);
  assert.ok(
    hasAssignment(queryTry.catchClause, "pendingCloseQueryFailed", (expression) => isBooleanLiteral(expression, true))
      && hasCall(queryTry.catchClause, "setMessage"),
    `${label}: timeline close query failure must set the failure flag and operator-visible message`,
  );
};

assertCorrelatedTimelineCloseControlFlow(app, "App CloseRequested");
const replaceCloseRequestedOnce = (candidate, expected, replacement, label) => {
  const closeHandler = candidate.slice(closeRequestedStart, closeRequestedEnd);
  const index = closeHandler.indexOf(expected);
  assert.ok(index >= 0, `${label}: hostile fixture anchor must remain discoverable`);
  assert.equal(closeHandler.indexOf(expected, index + expected.length), -1, `${label}: hostile fixture anchor must stay unique inside CloseRequested`);
  const absoluteIndex = closeRequestedStart + index;
  return candidate.slice(0, absoluteIndex) + replacement + candidate.slice(absoluteIndex + expected.length);
};
const queryOnlyComment = replaceCloseRequestedOnce(
  app,
  "pendingContext = await queryOwnPanePendingCloseContext();",
  "// pendingContext = await queryOwnPanePendingCloseContext();",
  "comment-only correlated query",
);
assert.throws(
  () => assertCorrelatedTimelineCloseControlFlow(queryOnlyComment, "comment-only correlated query"),
  /await the correlated pending-close query/,
  "a query token inside a comment must not satisfy the CloseRequested contract",
);
const noQueryFailureFlag = replaceCloseRequestedOnce(
  app,
  "pendingCloseQueryFailed = true;",
  "// pendingCloseQueryFailed = true;",
  "comment-only query failure flag",
);
assert.throws(
  () => assertCorrelatedTimelineCloseControlFlow(noQueryFailureFlag, "comment-only query failure flag"),
  /failure flag and operator-visible message/,
  "a failure-flag token inside a comment must not satisfy the CloseRequested contract",
);
const enrichedRequestStart = app.indexOf("          setProtectedCloseRequest({", closeRequestedStart);
const enrichedRequestEnd = app.indexOf("          });", enrichedRequestStart);
assert.ok(enrichedRequestStart >= 0 && enrichedRequestEnd > enrichedRequestStart, "enriched timeline close-request fixture must remain discoverable");
const bareCloseRequest = `${app.slice(0, enrichedRequestStart)}          setProtectedCloseRequest(closeRequest);${app.slice(enrichedRequestEnd + "          });".length)}`;
assert.throws(
  () => assertCorrelatedTimelineCloseControlFlow(bareCloseRequest, "bare timeline close request"),
  /publish the enriched request/,
  "the old bare close-request path must not satisfy the correlated CloseRequested contract",
);
const noDisposalFence = replaceCloseRequestedOnce(
  app,
  "if (closeRequestListenerDisposed) return;\n          setProtectedCloseRequest({",
  "// if (closeRequestListenerDisposed) return;\n          setProtectedCloseRequest({",
  "comment-only disposal fence",
);
assert.throws(
  () => assertCorrelatedTimelineCloseControlFlow(noDisposalFence, "comment-only disposal fence"),
  /fence disposal/,
  "a disposal-fence token inside a comment must not satisfy the CloseRequested contract",
);
const nestedCallbackCorrelatedQuery = replaceCloseRequestedOnce(
  app,
  "pendingContext = await queryOwnPanePendingCloseContext();",
  "await (async () => {\n              pendingContext = await queryOwnPanePendingCloseContext();\n            })()",
  "nested-callback correlated query",
);
assert.throws(
  () => assertCorrelatedTimelineCloseControlFlow(nestedCallbackCorrelatedQuery, "nested-callback correlated query"),
  /await the correlated pending-close query/,
  "a correlated query hidden inside a nested callback closure must not satisfy the CloseRequested contract",
);
const nestedCallbackFailureHandling = replaceCloseRequestedOnce(
  app,
  "pendingCloseQueryFailed = true;\n            setMessage(`Pane window close status could not be verified: ${error}`);",
  "void (class {\n              static flagFailure() {\n                pendingCloseQueryFailed = true;\n                setMessage(`Pane window close status could not be verified: ${error}`);\n              }\n            })",
  "nested-callback query failure handling",
);
assert.throws(
  () => assertCorrelatedTimelineCloseControlFlow(nestedCallbackFailureHandling, "nested-callback query failure handling"),
  /failure flag and operator-visible message/,
  "failure flag and operator message hidden inside a nested class-method closure must not satisfy the CloseRequested contract",
);
const staticallyDeadQueryBranch = replaceCloseRequestedOnce(
  app,
  "pendingContext = await queryOwnPanePendingCloseContext();",
  "if (false) {\n              pendingContext = await queryOwnPanePendingCloseContext();\n            }",
  "statically-dead correlated query branch",
);
assert.throws(
  () => assertCorrelatedTimelineCloseControlFlow(staticallyDeadQueryBranch, "statically-dead correlated query branch"),
  /await the correlated pending-close query/,
  "the correlated query hidden under a statically false if-branch must not satisfy the CloseRequested contract",
);
const deadStatementAfterPostPublishReturn = replaceCloseRequestedOnce(
  app,
  "            pendingCloseQueryFailed,\n          });\n          return;",
  "            pendingCloseQueryFailed,\n          });\n          return;\n          event.preventDefault();",
  "dead statement after post-publish return",
);
assert.throws(
  () => assertCorrelatedTimelineCloseControlFlow(deadStatementAfterPostPublishReturn, "dead statement after post-publish return"),
  /terminate at a bare post-publish return/,
  "executable statements after the post-publish return must fail the CloseRequested contract",
);
const missingPostPublishReturn = replaceCloseRequestedOnce(
  app,
  "            pendingCloseQueryFailed,\n          });\n          return;",
  "            pendingCloseQueryFailed,\n          });",
  "missing post-publish return",
);
assert.throws(
  () => assertCorrelatedTimelineCloseControlFlow(missingPostPublishReturn, "missing post-publish return"),
  /terminate at a bare post-publish return/,
  "a timeline branch that falls through instead of returning after publishing must fail the CloseRequested contract",
);
const earlyFakePublishMisplacedReal = replaceCloseRequestedOnce(
  app,
  "if (closeRequestListenerDisposed) return;\n          setProtectedCloseRequest({\n            ...closeRequest,\n            pendingClose: pendingContext,\n            pendingCloseQueryFailed,\n          });\n          return;",
  "if (closeRequestListenerDisposed) return;\n          setProtectedCloseRequest({ ...closeRequest, pendingClose: pendingContext, pendingCloseQueryFailed });\n          return;\n          setProtectedCloseRequest({\n            ...closeRequest,\n            pendingClose: pendingContext,\n            pendingCloseQueryFailed,\n          });",
  "early fake publish with misplaced authoritative publish",
);
assert.throws(
  () => assertCorrelatedTimelineCloseControlFlow(earlyFakePublishMisplacedReal, "early fake publish with misplaced authoritative publish"),
  /exactly one authoritative/,
  "an earlier duplicate fake publish must not satisfy the authoritative publish ordering while the real publish stays misplaced after the bare return",
);

const replaceAppOnce = (expected, replacement, label) => {
  const index = app.indexOf(expected);
  assert.ok(index >= 0, `${label}: hostile fixture anchor must remain discoverable`);
  assert.equal(app.indexOf(expected, index + expected.length), -1, `${label}: hostile fixture anchor must stay unique`);
  return app.slice(0, index) + replacement + app.slice(index + expected.length);
};
const readsProperty = (expression, root, property) => {
  const current = unwrapExpression(expression);
  return ts.isPropertyAccessExpression(current)
    && isIdentifierNamed(unwrapExpression(current.expression), root)
    && current.name.text === property;
};
const isEqualityOnStringProperty = (expression, root, property, value) => ts.isBinaryExpression(expression)
  && expression.operatorToken.kind === ts.SyntaxKind.EqualsEqualsEqualsToken
  && readsProperty(expression.left, root, property)
  && ts.isStringLiteral(expression.right)
  && expression.right.text === value;
const findSoleArrowDeclaration = (sourceFileCandidate, name, label) => {
  let found;
  const visit = (node) => {
    if (
      ts.isVariableDeclaration(node)
      && ts.isIdentifier(node.name)
      && node.name.text === name
      && node.initializer !== undefined
      && ts.isArrowFunction(node.initializer)
    ) {
      assert.equal(found, undefined, `${label}: ${name} must have exactly one arrow-function declaration`);
      found = node.initializer;
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFileCandidate);
  assert.ok(found !== undefined, `${label}: ${name} arrow declaration is required`);
  return found;
};
const translatedFirstStringArgument = (expression, label, why) => {
  const call = unwrapExpression(expression);
  assert.ok(
    ts.isCallExpression(call)
      && isIdentifierNamed(call.expression, "translateUiText")
      && call.arguments[0] !== undefined
      && ts.isStringLiteral(call.arguments[0]),
    `${label}: ${why}`,
  );
  return call.arguments[0].text;
};
const messageArgumentText = (statement, sourceFileCandidate) => {
  const call = unwrapExpression(statement.expression);
  const argument = call.arguments[0];
  if (argument === undefined) return "";
  if (ts.isStringLiteral(argument) || ts.isNoSubstitutionTemplateLiteral(argument)) return argument.text;
  if (ts.isTemplateExpression(argument)) return argument.getText(sourceFileCandidate);
  return "";
};

const assertProtectedCloseDialogConsumesPendingCloseSignals = (candidate, label) => {
  const sourceFileCandidate = ts.createSourceFile(
    `${label}.tsx`,
    candidate,
    ts.ScriptTarget.ES2022,
    true,
    ts.ScriptKind.TSX,
  );
  const copyFunction = findSoleArrowDeclaration(sourceFileCandidate, "protectedCloseCopy", label);
  assert.ok(ts.isBlock(copyFunction.body), `${label}: protectedCloseCopy must use a statement block`);
  const timelineCopyGuard = [...copyFunction.body.statements].find((statement) => isIfCondition(
    statement,
    (expression) => ts.isBinaryExpression(expression)
      && expression.operatorToken.kind === ts.SyntaxKind.BarBarToken
      && isNegatedIdentifier(expression.left, "request")
      && isEqualityOnStringProperty(expression.right, "request", "pane", "timeline"),
  ));
  assert.ok(timelineCopyGuard !== undefined, `${label}: protected-close copy must branch on the captured timeline pane`);
  const timelineCopyReturn = blockStatements(timelineCopyGuard.thenStatement).find(
    (statement) => statementIsReturn(statement) && statement.expression !== undefined,
  );
  assert.ok(timelineCopyReturn !== undefined, `${label}: protected-close timeline copy must return the operator-facing copy object`);
  const timelineCopyLiteral = unwrapExpression(timelineCopyReturn.expression);
  assert.ok(ts.isObjectLiteralExpression(timelineCopyLiteral), `${label}: protected-close timeline copy must be an object literal`);
  const detailProperty = [...timelineCopyLiteral.properties].find((property) => ts.isPropertyAssignment(property)
    && ts.isIdentifier(property.name)
    && property.name.text === "detail");
  assert.ok(detailProperty !== undefined, `${label}: protected-close timeline copy must expose an operator-visible detail`);
  const detailConditional = unwrapExpression(detailProperty.initializer);
  assert.ok(
    ts.isConditionalExpression(detailConditional),
    `${label}: protected-close timeline copy must consume request.pendingClose to choose the correlated parent-waiting detail`,
  );
  assert.ok(
    readsProperty(detailConditional.condition, "request", "pendingClose"),
    `${label}: protected-close timeline copy must consume request.pendingClose to choose the correlated parent-waiting detail`,
  );
  assert.equal(
    translatedFirstStringArgument(detailConditional.whenTrue, label, "pending-close-aware timeline detail must come from translateUiText"),
    "Timeline edits will be discarded. The main window is waiting for this Timeline window.",
    `${label}: consumed request.pendingClose must surface the correlated parent-waiting consequence to the operator`,
  );
  assert.equal(
    translatedFirstStringArgument(detailConditional.whenFalse, label, "plain timeline detail must come from translateUiText"),
    "Timeline edits will be discarded.",
    `${label}: unconsumed request.pendingClose must keep the plain timeline consequence`,
  );
  const cancelFunction = findSoleArrowDeclaration(sourceFileCandidate, "cancelProtectedClose", label);
  assert.ok(ts.isBlock(cancelFunction.body), `${label}: cancelProtectedClose must use a statement block`);
  const cancelStatements = [...cancelFunction.body.statements];
  const cancelTimelineGuard = cancelStatements.find((statement) => isIfCondition(
    statement,
    (expression) => isEqualityOnStringProperty(expression, "request", "pane", "timeline"),
  ));
  assert.ok(cancelTimelineGuard !== undefined, `${label}: cancel must branch on the captured timeline pane`);
  const cancelBranchStatements = blockStatements(cancelTimelineGuard.thenStatement);
  const seededContextIndex = cancelBranchStatements.findIndex((statement) => hasVariableInitializer(
    statement,
    "pendingContext",
    (expression) => readsProperty(expression, "request", "pendingClose"),
  ));
  assert.ok(seededContextIndex >= 0, `${label}: cancel must seed its decision from the captured request.pendingClose`);
  const recheckGuardIndex = cancelBranchStatements.findIndex((statement, index) => index > seededContextIndex && isIfCondition(
    statement,
    (expression) => ts.isBinaryExpression(expression)
      && expression.operatorToken.kind === ts.SyntaxKind.BarBarToken
      && readsProperty(expression.left, "request", "pendingCloseQueryFailed")
      && ts.isBinaryExpression(expression.right)
      && expression.right.operatorToken.kind === ts.SyntaxKind.EqualsEqualsEqualsToken
      && isIdentifierNamed(expression.right.left, "pendingContext")
      && isNullLiteral(expression.right.right),
  ));
  assert.ok(recheckGuardIndex >= 0, `${label}: cancel must consult request.pendingCloseQueryFailed before trusting the captured pending-close context`);
  const recheckTry = blockStatements(cancelBranchStatements[recheckGuardIndex].thenStatement).find((statement) => ts.isTryStatement(statement));
  assert.ok(recheckTry !== undefined, `${label}: failed-flag or missing context must trigger the bounded cancel re-query`);
  assert.ok(
    hasAssignment(recheckTry.tryBlock, "pendingContext", (expression) => isAwaitedCallNamed(expression, "queryOwnPanePendingCloseContext")),
    `${label}: the cancel re-query must await the correlated pending-close query`,
  );
  assert.ok(recheckTry.catchClause !== undefined, `${label}: the cancel re-query must handle failure explicitly`);
  const recheckCatchStatements = [...recheckTry.catchClause.block.statements];
  assert.equal(
    recheckCatchStatements.length,
    3,
    `${label}: failed cancel re-query must re-arm the flag, warn the operator, and stop before dismissing`,
  );
  const republishStatement = recheckCatchStatements[0];
  assert.ok(isCallStatement(republishStatement, "setProtectedCloseRequest"), `${label}: failed cancel re-query must republish the captured request`);
  const republishCall = unwrapExpression(republishStatement.expression);
  const republishLiteral = republishCall.arguments[0] !== undefined ? unwrapExpression(republishCall.arguments[0]) : undefined;
  assert.ok(
    republishLiteral !== undefined
      && ts.isObjectLiteralExpression(republishLiteral)
      && [...republishLiteral.properties].some((property) => ts.isSpreadAssignment(property) && isIdentifierNamed(property.expression, "request"))
      && [...republishLiteral.properties].some((property) => ts.isPropertyAssignment(property)
        && ts.isIdentifier(property.name)
        && property.name.text === "pendingCloseQueryFailed"
        && isBooleanLiteral(unwrapExpression(property.initializer), true)),
    `${label}: failed cancel re-query must fail closed by re-arming pendingCloseQueryFailed on the kept-open request`,
  );
  assert.ok(isCallStatement(recheckCatchStatements[1], "setMessage"), `${label}: failed cancel re-query must warn through the operator-visible message channel`);
  assert.ok(
    messageArgumentText(recheckCatchStatements[1], sourceFileCandidate).includes("Pane window close status could not be verified"),
    `${label}: failed cancel re-query must surface a visible fail-closed operator warning`,
  );
  assert.ok(
    statementIsReturn(recheckCatchStatements[2]) && recheckCatchStatements[2].expression === undefined,
    `${label}: failed cancel re-query must stop before the dismiss path so the dialog stays open`,
  );
  assert.ok(
    cancelStatements.length >= 2
      && isCallStatement(cancelStatements[cancelStatements.length - 2], "setProtectedCloseRequest")
      && (() => {
        const call = unwrapExpression(cancelStatements[cancelStatements.length - 2].expression);
        return call.arguments.length === 1 && isNullLiteral(unwrapExpression(call.arguments[0]));
      })()
      && isCallStatement(cancelStatements[cancelStatements.length - 1], "setMessage")
      && messageArgumentText(cancelStatements[cancelStatements.length - 1], sourceFileCandidate) === "Close canceled.",
    `${label}: only the unfailed cancel path may dismiss the dialog with the operator confirmation`,
  );
};

assertProtectedCloseDialogConsumesPendingCloseSignals(app, "App protected-close dialog");
const unconditionalTimelineDetail = replaceAppOnce(
  "detail: request?.pendingClose\n          ? translateUiText(",
  "// detail: request?.pendingClose\n          detail: true ? translateUiText(",
  "comment-only pending-close copy consumption",
);
assert.throws(
  () => assertProtectedCloseDialogConsumesPendingCloseSignals(unconditionalTimelineDetail, "comment-only pending-close copy consumption"),
  /consume request\.pendingClose/,
  "a pendingClose token inside a comment must not satisfy the dialog-consumption contract",
);
const failureFlagFreeRecheckGuard = replaceAppOnce(
  "if (request.pendingCloseQueryFailed || pendingContext === null) {",
  "if (pendingContext === null) { // request.pendingCloseQueryFailed || pendingContext === null",
  "comment-only failure-flag decision consumption",
);
assert.throws(
  () => assertProtectedCloseDialogConsumesPendingCloseSignals(failureFlagFreeRecheckGuard, "comment-only failure-flag decision consumption"),
  /consult request\.pendingCloseQueryFailed/,
  "a pendingCloseQueryFailed token inside a comment must not satisfy the dialog-decision contract",
);
const silentFailedRequery = replaceAppOnce(
  "setProtectedCloseRequest({ ...request, pendingCloseQueryFailed: true });",
  "setProtectedCloseRequest({ ...request }); // pendingCloseQueryFailed: true",
  "silent failed cancel re-query",
);
assert.throws(
  () => assertProtectedCloseDialogConsumesPendingCloseSignals(silentFailedRequery, "silent failed cancel re-query"),
  /fail closed by re-arming pendingCloseQueryFailed/,
  "a failed cancel re-query without the re-armed failure flag must fail the dialog-consumption contract",
);

assert.equal(
  config.app.windows[0].maximized,
  false,
  "the primary desktop window must create WebView2 windowed before the mounted controller maximizes it",
);
assert.equal(config.app.windows[0].decorations, false, "the primary desktop window must be frameless");
assert.equal(config.app.windows[0].resizable, true, "the frameless primary window must stay resizable");
assert.notEqual(
  nativeAcceptanceConfig.identifier,
  config.identifier,
  "native acceptance must use an isolated application identifier",
);
assert.equal(nativeAcceptanceConfig.app.windows[0].title, "Syndocal QA - Native 1920 Acceptance");
assert.equal(nativeAcceptanceConfig.app.windows[0].width, 1920);
assert.equal(nativeAcceptanceConfig.app.windows[0].height, 1080);
assert.equal(
  nativeAcceptanceConfig.app.windows[0].maximized,
  false,
  "native acceptance must exercise the same windowed WebView2 creation boundary before runtime maximize",
);
assert.equal(
  f11FocusConfig.app.windows[0].maximized,
  false,
  "F11 focus QA must not reintroduce maximized-at-WebView2-creation startup",
);
assert.equal(
  nativeAcceptanceConfig.app.windows[0].decorations,
  false,
  "native acceptance must exercise the frameless main-window contract",
);
assert.equal(packageJson.scripts["check:native-window"], "node scripts/run-native-window-acceptance.mjs");
assert.ok(
  packageJson.scripts["check:release-ui"].includes("check:native-window"),
  "the primary release UI gate must include real native maximized/fullscreen acceptance",
);
assert.ok(
  keyboardController.includes("const hasMappingInteraction =") &&
    keyboardController.includes('options.mappingStageTool() !== "select"') &&
    keyboardController.includes("if (!hasMappingInteraction) return"),
  "an idle Setup workspace must leave Escape available to exit fullscreen",
);
assert.ok(nativeAcceptance.includes('[string]$MinimumMaximizedClient = "1920x1000"'));
assert.ok(nativeAcceptance.includes('[string]$ExpectedFullscreen = "1920x1080"'));
assert.ok(nativeAcceptance.includes("-VirtualKey 0x7A"), "native acceptance must exercise F11");
assert.ok(nativeAcceptance.includes("-VirtualKey 0x1B"), "native acceptance must exercise Escape restore");
assert.ok(
  nativeAcceptance.includes("-Expected $expectedFullscreenSize -AllowedTolerancePx 0") &&
    nativeAcceptance.includes("-Expected $maximized -AllowedTolerancePx 0"),
  "native acceptance must require exact fullscreen and maximized restoration dimensions",
);
assert.ok(nativeAcceptance.includes("Save-ClientScreenshot"), "native acceptance must capture full-size visual evidence");
assert.ok(
  backend.includes("AcceleratorKeyPressedEventHandler::create") &&
    backend.includes("add_AcceleratorKeyPressed") &&
    backend.includes("args.SetHandled(true)") &&
    backend.includes("physical.WasKeyDown.as_bool()") &&
    backend.includes("shortcut_window") &&
    backend.includes(".is_fullscreen()") &&
    backend.includes("event_window.unmaximize()") &&
    backend.includes("event_window.set_fullscreen(true)") &&
    backend.includes("event_window.set_fullscreen(false)") &&
    backend.includes("event_window.maximize()") &&
    backend.includes("DESKTOP_ESCAPE_SHORTCUT_EVENT") &&
    backend.includes("shortcut_webview.ExecuteScript(") &&
    backend.includes("detail.consumed") &&
    backend.includes('if result != "true"') &&
    backend.includes("fallback_window.maximize()"),
  "Windows must own WebView2 F11/Escape, reject repeats, preserve real DOM Escape consumers, and restore maximized mode through a native fallback",
);
assert.ok(
  !nativeAcceptance.includes("AppActivate") && nativeAcceptance.includes("GetForegroundWindow() -ne $Handle"),
  "native acceptance must inject keys only after verifying the exact isolated QA window handle",
);
assert.ok(
  nativeAcceptance.includes("$preexistingQaWindow = Find-WindowByTitle") &&
    nativeAcceptance.includes("$qaProcess.StartTime.ToUniversalTime()") &&
    nativeAcceptance.includes("[IO.Path]::GetFullPath($qaProcess.Path) -ine $expectedQaExecutable") &&
    nativeAcceptance.includes("if ($qaWindowVerified -and $qaWindow -ne [IntPtr]::Zero)"),
  "native acceptance must reject stale same-title windows and bind evidence to the current isolated executable",
);
assert.ok(
  nativeAcceptance.includes("PrintWindow($Handle, $deviceContext, 3)") &&
    nativeAcceptance.includes("Save-VerifiedClientScreenshot") &&
    nativeAcceptance.includes("UniqueSampledColors -ge 32") &&
    nativeAcceptance.includes("visual_metrics"),
  "native acceptance must capture the exact HWND and reject blank or visually unready evidence",
);
assert.ok(
  capability.permissions.includes("core:window:allow-maximize"),
  "the main window must be allowed to enforce its operational maximized mode",
);
assert.ok(
  capability.permissions.includes("core:window:allow-set-fullscreen"),
  "the main window must be allowed to change fullscreen state",
);
for (const permission of [
  "core:window:allow-close",
  "core:window:allow-minimize",
  "core:window:allow-start-dragging",
  "core:window:allow-start-resize-dragging",
  "core:window:allow-toggle-maximize",
]) {
  assert.ok(capability.permissions.includes(permission), `frameless chrome requires ${permission}`);
}
assert.ok(
  main.includes("shouldMountDesktopWindowModeController(window.location.search)"),
  "the app entrypoint must scope desktop window control by route",
);
assert.ok(main.includes("<DesktopWindowModeController>"), "desktop window control must wrap the primary app");
assert.match(
  main,
  /shouldMountDesktopWindowModeController\(window\.location\.search\)\s*\?\s*\([\s\S]*?<DesktopWindowModeController>[\s\S]*?<App \/>[\s\S]*?<\/DesktopWindowModeController>[\s\S]*?\)\s*:\s*\(\s*<App \/>/,
  "video output routes must render App directly, outside DesktopWindowModeController",
);
assert.ok(controller.includes("appWindow.setFullscreen(next)"), "fullscreen changes must use the Tauri window API");
assert.ok(controller.includes("if (!next) await appWindow.maximize()"), "leaving fullscreen must restore the maximized operator workspace");
assert.ok(
  controller.includes('window.addEventListener(DESKTOP_ESCAPE_SHORTCUT_EVENT, handleNativeEscape)') &&
    controller.includes('querySelectorAll<HTMLDialogElement>("dialog[open]")') &&
    controller.includes("forwardingNativeEscape = true") &&
    controller.includes('if (forwardingNativeEscape && event.code === "Escape") return') &&
    controller.includes("detail.consumed = true") &&
    controller.includes("detail.consumed = keyEvent.defaultPrevented") &&
    controller.includes('window.removeEventListener(DESKTOP_ESCAPE_SHORTCUT_EVENT, handleNativeEscape)'),
  "the DOM controller must arbitrate native Escape in dialog, editor, then fullscreen order without recursive handling",
);
assert.ok(
  controller.includes("Starting already maximized can strand the controller") &&
    controller.includes("void enterOperationalWindowMode()") &&
    controller.includes("await appWindow.maximize()") &&
    controller.includes("if (transitionInFlight) return") &&
    controller.includes('mode() === "error"') &&
    controller.includes('shortcut: "RESTART APP"'),
  "runtime startup must maximize only after the mounted WebView2 controller is available",
);
const nativeWebViewHook = backend.indexOf('main_window.with_webview(move |webview| unsafe {');
const nativeStartupMaximize = backend.indexOf('main_window.maximize()?;', nativeWebViewHook);
const nativeStartupReady = backend.indexOf(
  'main_window_startup_ready.store(true, Ordering::Release);',
  nativeStartupMaximize,
);
assert.ok(
  nativeWebViewHook >= 0 &&
    nativeStartupMaximize > nativeWebViewHook &&
    nativeStartupReady > nativeStartupMaximize &&
    backend.includes('if !shortcut_startup_ready.load(Ordering::Acquire)') &&
    backend.includes('main_window_startup_ready = Arc::new(AtomicBool::new(false))'),
  "Windows startup must create the WebView2 controller windowed, maximize natively, then admit F11",
);
assert.ok(
  nativeAcceptance.includes('-RequireMaximized') &&
    !nativeAcceptance.includes('[void][SyndocalNativeWindow]::ShowWindowAsync($qaWindow, 3)'),
  "native acceptance must observe app-owned startup maximization without manufacturing it",
);
assert.ok(controller.includes('window.addEventListener("resize"'), "native window-mode changes must be resynchronized");
assert.ok(controller.includes('window.removeEventListener("keydown"'), "the global shortcut listener must be cleaned up");
assert.ok(controller.includes('aria-live="polite"'), "window-mode feedback must be announced accessibly");
assert.ok(
  app.includes("data-protected-close-dialog") &&
    app.includes("event.preventDefault()") &&
    app.includes("approveNativeCloseOnce()") &&
    app.includes("await getCurrentWindow().close()") &&
    app.includes("const consumeNativeCloseApproval =") &&
    app.includes("protectedCloseCompletionInFlight") &&
    !app.includes("const confirmProtectedClose =") &&
    !protectedCloseSource.includes("snapshot()") &&
    !protectedCloseSource.includes("UNSAVED SESSION"),
  "protected native close requests must use the operator-styled in-app dialog and reissue one approved close",
);
assert.ok(
  closeRequestedSource.includes("protectedCloseRefreshInFlight") &&
    closeRequestedSource.includes("protectedCloseRequest() !== null") &&
    !closeRequestedSource.includes("await refreshSnapshot(false, false)") &&
    closeRequestedSource.includes("await refreshSnapshotForProtectedClose()"),
  "main CloseRequested must reject the old unbounded refresh await and suppress repeated checks while one is pending",
);
assert.ok(
  app.includes("const PROTECTED_CLOSE_REFRESH_TIMEOUT_MS = 2_000") &&
    protectedCloseRefreshSource.includes("PROTECTED_CLOSE_REFRESH_TIMEOUT_MS") &&
    protectedCloseRefreshSource.includes("protectedCloseRefreshInFlight") &&
    protectedCloseRefreshSource.includes("refreshSnapshot(false, false)") &&
    protectedCloseRefreshSource.includes("window.setTimeout") &&
    protectedCloseRefreshSource.includes(".then(settle)") &&
    protectedCloseRefreshSource.includes(".catch(() => settle(null))") &&
    protectedCloseRefreshSource.includes("protectedCloseRefreshInFlight = false"),
  "close-specific refresh must be bounded, fail closed on rejection/timeout, and release its duplicate-check guard",
);
const closeRefreshIndex = closeRequestedSource.indexOf("await refreshSnapshotForProtectedClose()");
const closePreventIndex = closeRequestedSource.lastIndexOf("event.preventDefault();", closeRefreshIndex);
assert.ok(
  closePreventIndex >= 0 && closeRefreshIndex > closePreventIndex,
  "main CloseRequested must prevent the initial close before awaiting its bounded fresh snapshot",
);
assert.ok(
  closeRequestedSource.includes("protectedCloseRequestForCurrentState(freshSnapshot)") &&
    !closeRequestedSource.includes("protectedCloseRequestForCurrentState(latestEngineSnapshot)") &&
    closeRequestedSource.includes("setProtectedCloseRequest(protectedCloseUnknownRequest())"),
  "main CloseRequested must capture from the fresh snapshot and fail closed with an unknown-output request",
);
assert.ok(
  closeRequestedSource.includes("scheduleApprovedNativeClose()") &&
    scheduleApprovedNativeCloseSource.includes("window.setTimeout(() =>") &&
    protectedCloseSource.includes("void completeProtectedClose();"),
  "a fresh clean close must schedule the approved reissue on a later task",
);
assert.ok(
  scheduleApprovedNativeCloseSource.includes("protectedCloseRefreshInFlight") &&
    scheduleApprovedNativeCloseSource.includes("closeRequestListenerDisposed") &&
    scheduleApprovedNativeCloseSource.includes("protectedCloseRequest() !== null") &&
    !scheduleApprovedNativeCloseSource.match(/if \(protectedCloseRequest\(\) !== null\) return;/),
  "the scheduled approved close must not bypass a newer refresh check and must guard unmount",
);
assert.ok(
  protectedCloseRefreshSource.includes("protectedCloseRefreshTimeoutId") &&
    protectedCloseRefreshSource.includes("let cancelRefresh: (() => void) | undefined") &&
    protectedCloseRefreshSource.includes("cancelRefresh = () => settle(null)") &&
    protectedCloseRefreshSource.includes("protectedCloseRefreshCancel = cancelRefresh") &&
    protectedCloseRefreshSource.includes("protectedCloseRefreshCancel === cancelRefresh") &&
    protectedCloseRefreshSource.includes("if (closeRequestListenerDisposed)") &&
    protectedCloseCleanupSource.includes("protectedCloseRefreshCancel?.()") &&
    protectedCloseCleanupSource.includes("window.clearTimeout(scheduledApprovedNativeCloseTimer)") &&
    protectedCloseCleanupSource.includes("window.clearTimeout(protectedCloseRefreshTimeoutId)") &&
    closeRequestedSource.includes("const freshSnapshot = await refreshSnapshotForProtectedClose();") &&
    closeRequestedSource.includes("if (closeRequestListenerDisposed) return;"),
  "close timers and post-await work must be owned and stopped when the close listener is disposed",
);
assert.equal(
  (app.match(/await getCurrentWindow\(\)\.close\(\)/g) ?? []).length,
  1,
  "protected close confirmation must issue exactly one native close call",
);
assert.ok(
  app.includes("let latestEngineSnapshot = initialEngineSnapshot;") &&
    protectedCloseRequestSource.includes("mainRuntimeActive(current)") &&
    protectedCloseRequestSource.includes("current: EngineSnapshot = latestEngineSnapshot") &&
    closeProtectionRequiredSource.includes("protectedCloseRequestForCurrentState() !== null") &&
    !mainRuntimeActiveSource.includes("createMemo") &&
    mainRuntimeActiveSource.includes("timelineExecutionIsLive(") &&
    mainRuntimeActiveSource.includes("current.timeline.playing") &&
    mainRuntimeActiveSource.includes("current.clock.source") &&
    mainRuntimeActiveSource.includes("current.clock.external_sync_locked") &&
    mainRuntimeActiveSource.includes("current.clock.external_sync_age_ms") &&
    mainRuntimeActiveSource.includes("current.direct_child_timeline_transports?.some(") &&
    mainRuntimeActiveSource.includes("transport.playing") &&
    mainRuntimeActiveSource.includes("Boolean(current.active_fade)") &&
    mainRuntimeActiveSource.includes("current.active_cue_id !== null") &&
    mainRuntimeActiveSource.includes("current.active_cue_id !== undefined") &&
    mainRuntimeActiveSource.includes("Object.keys(current.active_group_cue_ids ?? {}).length > 0") &&
    mainRuntimeActiveSource.includes("current.video.layers.some((layer) => layer.state.playing)") &&
    mainRuntimeActiveSource.includes("current.effects.some((effect) => effect.enabled)") &&
    mainRuntimeActiveSource.includes("current.programmer.enabled") &&
    mainRuntimeActiveSource.includes("!current.programmer.blind") &&
    mainRuntimeActiveSource.includes("current.programmer.values.length > 0") &&
    !mainRuntimeActiveSource.includes("dmx_outputs") &&
    !mainRuntimeActiveSource.includes("current.output") &&
    !mainRuntimeActiveSource.includes("current.video.outputs"),
  "main runtime activity must use latest authoritative snapshot signals, including timeline clock freshness, video/effects/programmer activity, not configured output routes",
);
assert.ok(
  app.includes("type ProtectedCloseRequest =") &&
    app.includes('reason: "timeline-dirty"') &&
    app.includes('reason: "dirty-only" | "runtime-only" | "dirty-and-runtime" | "output-state-unknown"') &&
    app.includes("projectDirty: boolean") &&
    app.includes("timelineDirty: boolean") &&
    app.includes("runtimeActive: boolean | null") &&
    protectedCloseRequestSource.includes("const projectIsDirty = projectDirty()") &&
    protectedCloseRequestSource.includes("const timelineIsDirty = timelineEditorDirty()") &&
    protectedCloseRequestSource.includes("const runtimeIsActive = mainRuntimeActive(current)") &&
    app.includes("setProtectedCloseRequest(closeRequest)"),
  "protected close must capture a discriminated dirty/runtime reason object from the latest snapshot",
);
assert.ok(
  protectedCloseUnknownRequestSource.includes('reason: "output-state-unknown"') &&
    protectedCloseUnknownRequestSource.includes("projectDirty: projectDirty()") &&
    protectedCloseUnknownRequestSource.includes("timelineDirty: timelineEditorDirty()") &&
    protectedCloseUnknownRequestSource.includes("runtimeActive: null"),
  "refresh failure must capture dirty flags without pretending that runtime is inactive",
);
assert.match(
  protectedCloseRequestSource,
  /if \(paneWindow === "timeline"\) \{[\s\S]*?return timelineEditorDirty\(\)\s*\?\s*\{[\s\S]*?pane:\s*"timeline",[\s\S]*?reason:\s*"timeline-dirty",[\s\S]*?pendingClose:\s*null,[\s\S]*?pendingCloseQueryFailed:\s*false,[\s\S]*?\}[\s\S]*?:\s*null;/,
  "timeline child-pane close protection must begin dirty-only with no invented pending-close context",
);
assert.ok(
  !protectedCloseMainDetailSource.includes("projectDirty()") &&
    !protectedCloseMainDetailSource.includes("timelineEditorDirty()") &&
    !protectedCloseMainDetailSource.includes("mainRuntimeActive(") &&
    !protectedCloseCopySource.includes("projectDirty()") &&
    !protectedCloseCopySource.includes("timelineEditorDirty()") &&
    !protectedCloseCopySource.includes("mainRuntimeActive("),
  "protected-close copy must use captured request fields and cannot drift from live state while open",
);
for (const copy of [
  "OUTPUT STATE UNKNOWN",
  "Close Anyway",
  "Discard and Close Anyway",
  "LIVE OUTPUT ACTIVE",
  "Playback/live output is active. Stop and close?",
  "Stop and Close",
  "UNSAVED CHANGES",
  "Discard and Close",
  "UNSAVED CHANGES + LIVE OUTPUT",
  "Discard, Stop and Close",
  "Project changes will be discarded.",
  "Timeline edits will be discarded.",
  "Project changes and Timeline edits will be discarded.",
  "Project changes will be discarded. Playback/live output will stop.",
  "Timeline edits will be discarded. Playback/live output will stop.",
  "Project changes and Timeline edits will be discarded. Playback/live output will stop.",
  "Playback/live output state could not be verified before closing. Keep Syndocal open to avoid an unsafe shutdown.",
  "Project changes will be discarded. Playback/live output state could not be verified before closing.",
  "Timeline edits will be discarded. Playback/live output state could not be verified before closing.",
  "Project changes and Timeline edits will be discarded. Playback/live output state could not be verified before closing.",
]) {
  assert.ok(protectedCloseSource.includes(`\"${copy}\"`), `protected-close copy must cover ${copy}`);
}
assert.ok(
  protectedCloseCopySource.includes('if (request.reason === "output-state-unknown")') &&
    protectedCloseCopySource.includes('eyebrow: translateUiText("OUTPUT STATE UNKNOWN"') &&
    protectedCloseCopySource.includes('translateUiText("Close Anyway"') &&
    protectedCloseCopySource.includes('translateUiText("Discard and Close Anyway"') &&
    protectedCloseCopySource.includes('if (request.reason === "runtime-only")') &&
    protectedCloseCopySource.includes('eyebrow: translateUiText("LIVE OUTPUT ACTIVE"') &&
    protectedCloseCopySource.includes('confirm: translateUiText("Stop and Close"') &&
    protectedCloseCopySource.includes('if (request.reason === "dirty-and-runtime")') &&
    protectedCloseCopySource.includes('eyebrow: translateUiText("UNSAVED CHANGES + LIVE OUTPUT"') &&
    protectedCloseCopySource.includes('confirm: translateUiText("Discard, Stop and Close"') &&
    protectedCloseCopySource.includes('eyebrow: translateUiText("UNSAVED CHANGES"') &&
    protectedCloseCopySource.includes('confirm: translateUiText("Discard and Close"'),
  "protected-close copy must expose distinct runtime-only, dirty-only, and combined eyebrow/action cases",
);
assert.ok(
  protectedCloseMainDetailSource.includes("Playback/live output state could not be verified before closing.") &&
    protectedCloseMainDetailSource.includes("Keep Syndocal open to avoid an unsafe shutdown.") &&
  protectedCloseMainDetailSource.includes("Playback/live output is active. Stop and close?") &&
    protectedCloseMainDetailSource.includes("Playback/live output will stop.") &&
    protectedCloseMainDetailSource.includes("request.projectDirty") &&
    protectedCloseMainDetailSource.includes("request.timelineDirty") &&
    protectedCloseMainDetailSource.includes("request.runtimeActive") &&
    !protectedCloseMainDetailSource.includes("Live DMX output"),
  "protected-close detail must capture unknown, runtime-only, dirty-only, and combined consequences with generic playback/live wording",
);
const unknownCopyStart = protectedCloseCopySource.indexOf('if (request.reason === "output-state-unknown")');
const runtimeOnlyCopyStart = protectedCloseCopySource.indexOf('if (request.reason === "runtime-only")');
const dirtyAndRuntimeCopyStart = protectedCloseCopySource.indexOf('if (request.reason === "dirty-and-runtime")');
const dirtyOnlyCopyStart = protectedCloseCopySource.search(
  /return\s*\{\s*eyebrow:\s*translateUiText\("UNSAVED CHANGES"/,
);
assert.ok(unknownCopyStart >= 0 && runtimeOnlyCopyStart > unknownCopyStart && dirtyAndRuntimeCopyStart > runtimeOnlyCopyStart && dirtyOnlyCopyStart > dirtyAndRuntimeCopyStart);
const unknownCopySource = protectedCloseCopySource.slice(unknownCopyStart, runtimeOnlyCopyStart);
const runtimeOnlyCopySource = protectedCloseCopySource.slice(runtimeOnlyCopyStart, dirtyAndRuntimeCopyStart);
const dirtyOnlyCopySource = protectedCloseCopySource.slice(dirtyOnlyCopyStart);
assert.ok(
  !unknownCopySource.includes("UNSAVED SESSION") &&
    !unknownCopySource.includes("UNSAVED CHANGES") &&
  !runtimeOnlyCopySource.includes("UNSAVED SESSION") &&
    !runtimeOnlyCopySource.includes("Close Without Saving") &&
    !dirtyOnlyCopySource.toLowerCase().includes("output"),
  "runtime-only copy must not claim unsaved-session state, and dirty-only copy must not mention output",
);
for (const [english, japanese] of [
  ["OUTPUT STATE UNKNOWN", "出力状態不明"],
  ["Close Anyway", "確認せずに閉じる"],
  ["Discard and Close Anyway", "破棄して確認せずに閉じる"],
  ["LIVE OUTPUT ACTIVE", "ライブ出力中"],
  ["Stop and Close", "停止して閉じる"],
  ["Discard and Close", "破棄して閉じる"],
  ["Discard, Stop and Close", "破棄して停止して閉じる"],
  ["UNSAVED CHANGES", "未保存の変更"],
  ["UNSAVED CHANGES + LIVE OUTPUT", "未保存の変更 + ライブ出力中"],
  ["Playback/live output is active. Stop and close?", "再生／ライブ出力が有効です。停止して閉じますか？"],
  ["Playback/live output will stop.", "再生／ライブ出力は停止します。"],
  ["Playback/live output state could not be verified before closing. Keep Syndocal open to avoid an unsafe shutdown.", "閉じる前に再生／ライブ出力の状態を確認できませんでした。安全のためSyndocalを開いたままにしてください。"],
  ["Project changes will be discarded. Playback/live output state could not be verified before closing.", "プロジェクトの変更は破棄されます。閉じる前に再生／ライブ出力の状態を確認できませんでした。"],
  ["Timeline edits will be discarded. Playback/live output state could not be verified before closing.", "タイムライン編集は破棄されます。閉じる前に再生／ライブ出力の状態を確認できませんでした。"],
  ["Project changes and Timeline edits will be discarded. Playback/live output state could not be verified before closing.", "プロジェクトの変更とタイムライン編集は破棄されます。閉じる前に再生／ライブ出力の状態を確認できませんでした。"],
]) {
  assert.ok(uiLocalization.includes(english) && uiLocalization.includes(japanese), `localization must cover ${english}`);
}
assert.ok(
  styles.includes(".protectedCloseDialog") &&
    styles.includes(".protectedCloseMark") &&
    styles.includes(".protectedCloseActions button.danger"),
  "protected close must use the dedicated console-style visual hierarchy",
);
assert.ok(
  app.includes('const confirmDiscardProjectChanges = (actionLabel: string): Promise<boolean> => {') &&
    !app.slice(
      app.indexOf('const confirmDiscardProjectChanges = (actionLabel: string): Promise<boolean> => {'),
      app.indexOf('  const applyEngineSnapshot ='),
    ).includes("window.confirm") &&
    app.includes("data-project-discard-dialog") &&
    app.includes('role="alertdialog"') &&
    app.includes("data-project-discard-cancel") &&
    app.includes("data-project-discard-confirm") &&
    app.includes('class="protectedCloseDialog"') &&
    app.includes('class="protectedCloseFrame"') &&
    app.includes('class="protectedCloseMark"') &&
    app.includes('class="protectedCloseActions"') &&
    app.includes('if (!await confirmDiscardProjectChanges("create a new project"))'),
  "project replacement must use the in-app protected-close alertdialog instead of a browser confirmation",
);
for (const copy of [
  "Discard unsaved changes?",
  "Create a new project?",
  "Unsaved project changes will be discarded before continuing.",
  "Unsaved Timeline edits will be discarded before continuing.",
  "Unsaved project changes and Timeline edits will be discarded before continuing.",
  "Discard and Continue",
]) {
  assert.ok(
    uiLocalization.includes(copy),
    `project-discard localization must cover ${copy}`,
  );
}
assert.ok(
  controller.includes("DESKTOP_RESIZE_DIRECTIONS") &&
    controller.includes("await appWindow.startResizeDragging(direction)") &&
    controller.includes("await appWindow.isFullscreen()") &&
    controller.includes("await appWindow.isMaximized()") &&
    controller.includes("<DesktopWindowResizeZones />"),
  "windowed frameless mode must expose eight native resize-drag boundaries and suppress them while maximized/fullscreen",
);
assert.ok(
  styles.includes(".desktopResizeZoneNorthEast") &&
    styles.includes(".desktopResizeZoneSouthEast") &&
    styles.includes(".desktopResizeZoneSouthWest") &&
    styles.includes(".desktopResizeZoneNorthWest") &&
    styles.includes('html[data-window-mode="maximized"] .desktopResizeZones') &&
    styles.includes('html[data-window-mode="fullscreen"] .desktopResizeZones'),
  "frameless resize zones must include corners and stay inactive outside windowed mode",
);
assert.ok(
  workspaceChrome.includes('<header class="topbar" data-tauri-drag-region>') &&
    workspaceChrome.includes('<div class="topbarLeft" data-tauri-drag-region ref={projectMenuRoot}>') &&
    workspaceChrome.includes('<div class="status" data-tauri-drag-region>') &&
    workspaceChrome.includes("<strong data-tauri-drag-region>Syndocal</strong>") &&
    workspaceChrome.includes("<span data-tauri-drag-region>{props.projectLabel}</span>") &&
    workspaceChrome.includes('data-window-control="minimize"') &&
    workspaceChrome.includes('data-window-control="maximize"') &&
    workspaceChrome.includes('data-window-control="close"'),
  "the T25-E topbar must expose each top-level empty background as a drag region and keep all three window controls",
);
assert.doesNotMatch(
  workspaceChrome,
  /<(?:button|input|select|textarea)\b[^>]*data-tauri-drag-region/,
  "interactive topbar controls must not become native drag regions",
);
assert.ok(
  workspaceChrome.includes('data-project-menu-action="save"') &&
    workspaceChrome.includes('data-project-menu-action="load"') &&
    !workspaceChrome.includes('class="projectAction"'),
  "Save and Load must be absent from the topbar while remaining available in the project menu",
);
assert.ok(
  workspaceChrome.includes('aria-label="最小化"') &&
    workspaceChrome.includes('aria-label="最大化または元に戻す"') &&
    workspaceChrome.includes('aria-label="閉じる"'),
  "window controls must expose Japanese accessible names",
);
assert.ok(
  workspaceChrome.includes("await appWindow.minimize()") &&
    workspaceChrome.includes("await appWindow.toggleMaximize()") &&
    workspaceChrome.includes("await appWindow.close()") &&
    !workspaceChrome.includes("appWindow.destroy()"),
  "custom controls must use native minimize/toggle and route close through CloseRequested instead of a forced destroy",
);
assert.ok(
  workspaceChrome.includes("if (!isTauriRuntime()) return"),
  "window controls must remain browser-rendered no-ops outside Tauri",
);
assert.ok(
  controller.includes("const documentRoot = document.documentElement") &&
    controller.includes("documentRoot.setAttribute(") &&
    controller.includes("documentRoot.setAttribute(DESKTOP_WINDOW_MODE_ATTRIBUTE, nextMode)") &&
    controller.includes("documentRoot.removeAttribute(DESKTOP_WINDOW_MODE_ATTRIBUTE)"),
  "the controller must publish window mode on the document root and remove an attribute it owns",
);
assert.match(
  controller,
  /if \(hadPreviousWindowMode\) \{[\s\S]*?documentRoot\.setAttribute\(DESKTOP_WINDOW_MODE_ATTRIBUTE, previousWindowMode \?\? ""\);[\s\S]*?\} else \{[\s\S]*?documentRoot\.removeAttribute\(DESKTOP_WINDOW_MODE_ATTRIBUTE\);/,
  "cleanup must restore a pre-existing document window-mode value instead of overwriting it",
);

const transpiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "desktopWindowMode.ts",
});
const shortcuts = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);

assert.equal(shortcuts.DESKTOP_WINDOW_MODE_ATTRIBUTE, "data-window-mode");
assert.equal(shortcuts.desktopWindowModeFromWindowState(false, false), "windowed");
assert.equal(shortcuts.desktopWindowModeFromWindowState(false, true), "maximized");
assert.equal(shortcuts.desktopWindowModeFromWindowState(true, false), "fullscreen");
assert.equal(shortcuts.desktopWindowModeFromWindowState(true, true), "fullscreen");

assert.equal(shortcuts.shouldMountDesktopWindowModeController(""), true);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?syndocalViewportFixture=primary"), true);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?syndocalPaneWindow=stage"), false);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?syndocalPaneWindow=timeline"), false);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?videoOutputId=1"), false);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?testPattern=1&videoOutputId=12"), false);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?videoOutputId=0"), true);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?videoOutputId=invalid"), true);
const event = (overrides = {}) => ({
  code: "KeyA",
  repeat: false,
  altKey: false,
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  isComposing: false,
  defaultPrevented: false,
  editableTarget: false,
  ...overrides,
});

assert.equal(shortcuts.desktopWindowShortcutAction(event({ code: "F11" }), false), "toggleFullscreen");
assert.equal(shortcuts.desktopWindowShortcutAction(event({ code: "F11", editableTarget: true }), false), "toggleFullscreen");
assert.equal(shortcuts.desktopWindowShortcutAction(event({ code: "Escape" }), true), "exitFullscreen");
assert.equal(shortcuts.desktopWindowShortcutAction(event({ code: "Escape" }), false), null);
assert.equal(
  shortcuts.desktopWindowShortcutAction(
    event({ code: "Escape", editableTarget: false, defaultPrevented: true }),
    true,
  ),
  null,
  "a drawer or other non-editable surface that consumes Escape must keep fullscreen",
);
assert.equal(
  shortcuts.desktopWindowShortcutAction(
    event({ code: "Escape", editableTarget: true, defaultPrevented: true }),
    true,
  ),
  null,
  "a form editor that consumes Escape must keep control of the key",
);
for (const guarded of [
  { code: "F11", repeat: true },
  { code: "F11", ctrlKey: true },
  { code: "F11", isComposing: true },
  { code: "F11", defaultPrevented: true },
]) {
  assert.equal(shortcuts.desktopWindowShortcutAction(event(guarded), false), null);
}

console.log("primary-only window mode controller and safe desktop fullscreen shortcuts ok");

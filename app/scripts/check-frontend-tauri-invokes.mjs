import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manifestPath = path.join(appRoot, "src", "tauri-invoke-manifest.json");

const locationOf = (sourceFile, node) => {
  const { line, character } = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
  return `${path.relative(appRoot, sourceFile.fileName)}:${line + 1}:${character + 1}`;
};

const unwrapExpression = (expression) => {
  let current = expression;
  while (ts.isParenthesizedExpression(current)
    || ts.isAsExpression(current)
    || ts.isSatisfiesExpression(current)
    || ts.isNonNullExpression(current)) current = current.expression;
  return current;
};

const symbolAt = (checker, node) => checker.getSymbolAtLocation(node);

const canonicalSymbol = (checker, originalSymbol) => {
  let symbol = originalSymbol;
  const seen = new Set();
  while (symbol && (symbol.flags & ts.SymbolFlags.Alias) !== 0 && !seen.has(symbol)) {
    seen.add(symbol);
    const target = checker.getAliasedSymbol(symbol);
    if (!target || target === symbol) break;
    symbol = target;
  }
  return symbol;
};

const addTrackedSymbol = (checker, invokeSymbols, symbol) => {
  if (!symbol) return false;
  let changed = false;
  for (const candidate of [symbol, canonicalSymbol(checker, symbol)]) {
    if (candidate && !invokeSymbols.has(candidate)) {
      invokeSymbols.add(candidate);
      changed = true;
    }
  }
  return changed;
};

const symbolIsTracked = (checker, invokeSymbols, symbol) => Boolean(
  symbol && (invokeSymbols.has(symbol) || invokeSymbols.has(canonicalSymbol(checker, symbol))),
);

const parameterDeclaresFrontendCommand = (checker, declaration) => {
  const typeNode = declaration?.type;
  if (!typeNode || !ts.isTypeReferenceNode(typeNode)) return false;
  const name = ts.isIdentifier(typeNode.typeName) ? typeNode.typeName.text : typeNode.typeName.right.text;
  if (name === "FrontendTauriInvokeCommand") return true;
  const symbol = symbolAt(checker, typeNode.typeName);
  return canonicalSymbol(checker, symbol)?.name === "FrontendTauriInvokeCommand";
};

const typeIsFrontendInvoke = (checker, type) => type.getCallSignatures().some((signature) => {
  const parameter = signature.getParameters()[0];
  if (!parameter) return false;
  const declarations = parameter.declarations ?? (parameter.valueDeclaration ? [parameter.valueDeclaration] : []);
  if (declarations.some((declaration) => parameterDeclaresFrontendCommand(checker, declaration))) return true;
  const declaration = parameter.valueDeclaration ?? declarations[0];
  if (!declaration) return false;
  const parameterType = checker.getTypeOfSymbolAtLocation(parameter, declaration);
  return parameterType.aliasSymbol?.name === "FrontendTauriInvokeCommand"
    || checker.typeToString(parameterType) === "FrontendTauriInvokeCommand";
});

const expressionIsInvokeCallable = (checker, expression, invokeSymbols) => {
  const current = unwrapExpression(expression);
  const symbol = symbolAt(checker, ts.isPropertyAccessExpression(current) ? current.name : current);
  return symbolIsTracked(checker, invokeSymbols, symbol)
    || typeIsFrontendInvoke(checker, checker.getTypeAtLocation(current));
};

const expressionResolvesOnlyToInvoke = (checker, expression, invokeSymbols) => {
  const current = unwrapExpression(expression);
  if (ts.isConditionalExpression(current)) {
    return expressionResolvesOnlyToInvoke(checker, current.whenTrue, invokeSymbols)
      && expressionResolvesOnlyToInvoke(checker, current.whenFalse, invokeSymbols);
  }
  return expressionIsInvokeCallable(checker, current, invokeSymbols);
};

const bindingPropertyName = (element) => {
  const name = element.propertyName ?? element.name;
  return ts.isIdentifier(name) || ts.isStringLiteralLike(name) ? name.text : null;
};

const isInvokeAliasPropertyName = (name) => name === "invoke" || name === "invokeCommand";

const sourcePropertySymbol = (checker, source, propertyName) => {
  const type = checker.getTypeAtLocation(source);
  return type.getProperty(propertyName);
};

const collectInvokeSymbols = (checker, sourceFiles) => {
  const invokeSymbols = new Set();
  for (const sourceFile of sourceFiles) {
    const seed = (node) => {
      if (ts.isImportDeclaration(node)
        && ts.isStringLiteral(node.moduleSpecifier)
        && node.moduleSpecifier.text === "@tauri-apps/api/core") {
        const bindings = node.importClause?.namedBindings;
        if (bindings && ts.isNamedImports(bindings)) {
          for (const binding of bindings.elements) {
            if ((binding.propertyName?.text ?? binding.name.text) === "invoke") {
              addTrackedSymbol(checker, invokeSymbols, symbolAt(checker, binding.name));
            }
          }
        } else if (bindings && ts.isNamespaceImport(bindings)) {
          const invokeExport = checker.getTypeAtLocation(bindings.name).getProperty("invoke");
          addTrackedSymbol(checker, invokeSymbols, invokeExport);
        }
      }
      if (ts.isExportDeclaration(node)
        && node.moduleSpecifier
        && ts.isStringLiteral(node.moduleSpecifier)
        && node.moduleSpecifier.text === "@tauri-apps/api/core"
        && node.exportClause
        && ts.isNamedExports(node.exportClause)) {
        for (const binding of node.exportClause.elements) {
          if ((binding.propertyName?.text ?? binding.name.text) === "invoke") {
            addTrackedSymbol(checker, invokeSymbols, symbolAt(checker, binding.name));
          }
        }
      }
      ts.forEachChild(node, seed);
    };
    seed(sourceFile);
  }

  let changed = true;
  while (changed) {
    changed = false;
    for (const sourceFile of sourceFiles) {
      const propagate = (node) => {
        if (ts.isVariableDeclaration(node)) {
          if (ts.isIdentifier(node.name)
            && node.initializer
            && expressionResolvesOnlyToInvoke(checker, node.initializer, invokeSymbols)) {
            changed = addTrackedSymbol(checker, invokeSymbols, symbolAt(checker, node.name)) || changed;
          } else if (ts.isObjectBindingPattern(node.name) && node.initializer) {
            for (const element of node.name.elements) {
              const propertyName = bindingPropertyName(element);
              const property = propertyName && sourcePropertySymbol(checker, node.initializer, propertyName);
              if (ts.isIdentifier(element.name) && symbolIsTracked(checker, invokeSymbols, property)) {
                changed = addTrackedSymbol(checker, invokeSymbols, symbolAt(checker, element.name)) || changed;
              }
            }
          }
        }
        if (ts.isBinaryExpression(node)
          && node.operatorToken.kind === ts.SyntaxKind.EqualsToken
          && ts.isIdentifier(node.left)
          && expressionResolvesOnlyToInvoke(checker, node.right, invokeSymbols)) {
          changed = addTrackedSymbol(checker, invokeSymbols, symbolAt(checker, node.left)) || changed;
        }
        if (ts.isBinaryExpression(node)
          && node.operatorToken.kind === ts.SyntaxKind.EqualsToken
          && ts.isObjectLiteralExpression(unwrapExpression(node.left))) {
          const pattern = unwrapExpression(node.left);
          for (const property of pattern.properties) {
            if (!ts.isPropertyAssignment(property) || !ts.isIdentifier(property.initializer)) continue;
            const name = ts.isIdentifier(property.name) || ts.isStringLiteralLike(property.name)
              ? property.name.text
              : null;
            const sourceProperty = name && sourcePropertySymbol(checker, node.right, name);
            if (symbolIsTracked(checker, invokeSymbols, sourceProperty)) {
              changed = addTrackedSymbol(checker, invokeSymbols, symbolAt(checker, property.initializer)) || changed;
            }
          }
        }
        ts.forEachChild(node, propagate);
      };
      propagate(sourceFile);
    }
  }
  return invokeSymbols;
};

const invokeCommandArgument = (checker, node, invokeSymbols) => {
  if (!ts.isCallExpression(node)) return null;
  const callee = unwrapExpression(node.expression);
  if (ts.isPropertyAccessExpression(callee) || ts.isElementAccessExpression(callee)) {
    const receiver = callee.expression;
    const member = ts.isPropertyAccessExpression(callee)
      ? callee.name.text
      : ts.isStringLiteralLike(callee.argumentExpression) ? callee.argumentExpression.text : null;
    if (expressionIsInvokeCallable(checker, receiver, invokeSymbols)) {
      if (member === "call") return { argument: node.arguments[1], escape: null };
      if (member === "apply" || member === "bind") return { argument: null, escape: member };
      return { argument: null, escape: `member ${member ?? "<dynamic>"}` };
    }
  }
  if (expressionIsInvokeCallable(checker, callee, invokeSymbols)) {
    return { argument: node.arguments[0], escape: null };
  }
  return null;
};

const finiteStringLiterals = (checker, originalType, seen = new Set()) => {
  const type = originalType.flags & ts.TypeFlags.TypeParameter
    ? checker.getBaseConstraintOfType(originalType) ?? originalType
    : originalType;
  if (seen.has(type)) return null;
  seen.add(type);
  if (type.isStringLiteral()) return new Set([type.value]);
  if (!type.isUnion()) return null;
  const values = new Set();
  for (const member of type.types) {
    const memberValues = finiteStringLiterals(checker, member, seen);
    if (!memberValues) return null;
    for (const value of memberValues) values.add(value);
  }
  return values;
};

const commandsAtArgument = (checker, argument) => {
  if (ts.isStringLiteralLike(argument)) return new Set([argument.text]);
  return finiteStringLiterals(checker, checker.getTypeAtLocation(argument));
};

const isExplicitManifestType = (checker, argument) => {
  const type = checker.getTypeAtLocation(argument);
  return type.aliasSymbol?.name === "FrontendTauriInvokeCommand"
    || checker.typeToString(type) === "FrontendTauriInvokeCommand";
};

const isTypeOnlyOrDeclarationReference = (node) => {
  let current = node;
  while (current.parent) {
    const parent = current.parent;
    if (ts.isTypeNode(parent)
      || ts.isQualifiedName(parent)
      || ts.isImportSpecifier(parent)
      || ts.isExportSpecifier(parent)
      || ts.isNamespaceImport(parent)
      || ts.isImportClause(parent)
      || ts.isImportDeclaration(parent)
      || ts.isExportDeclaration(parent)
      || (ts.isJsxAttribute(parent) && parent.name === current)) return true;
    if ((ts.isPropertyAccessExpression(parent) && parent.name === current)
      || (ts.isPropertyAssignment(parent) && parent.name === current)
      || (ts.isBindingElement(parent) && (parent.name === current || parent.propertyName === current))
      || (ts.isVariableDeclaration(parent) && parent.name === current)
      || (ts.isParameter(parent) && parent.name === current)
      || (ts.isFunctionDeclaration(parent) && parent.name === current)
      || (ts.isFunctionExpression(parent) && parent.name === current)
      || (ts.isMethodDeclaration(parent) && parent.name === current)
      || (ts.isPropertyDeclaration(parent) && parent.name === current)
      || (ts.isPropertySignature(parent) && parent.name === current)
      || (ts.isMethodSignature(parent) && parent.name === current)
      || (ts.isTypeAliasDeclaration(parent) && parent.name === current)
      || (ts.isInterfaceDeclaration(parent) && parent.name === current)
      || (ts.isClassDeclaration(parent) && parent.name === current)) return true;
    if (ts.isParenthesizedExpression(parent)
      || ts.isAsExpression(parent)
      || ts.isSatisfiesExpression(parent)
      || ts.isNonNullExpression(parent)) {
      current = parent;
      continue;
    }
    if ((ts.isPropertyAccessExpression(parent) || ts.isElementAccessExpression(parent))
      && parent.expression === current) {
      current = parent;
      continue;
    }
    break;
  }
  return false;
};

const aliasSourceRoot = (checker, node, invokeSymbols) => {
  let current = node;
  while (current.parent) {
    const parent = current.parent;
    if ((ts.isParenthesizedExpression(parent)
      || ts.isAsExpression(parent)
      || ts.isSatisfiesExpression(parent)
      || ts.isNonNullExpression(parent)) && parent.expression === current) {
      current = parent;
      continue;
    }
    if (ts.isConditionalExpression(parent)
      && (parent.whenTrue === current || parent.whenFalse === current)
      && expressionResolvesOnlyToInvoke(checker, parent, invokeSymbols)) {
      current = parent;
      continue;
    }
    break;
  }
  return current;
};

const transparentReferenceRoot = (node) => {
  let current = node;
  while (current.parent
    && (ts.isParenthesizedExpression(current.parent)
      || ts.isAsExpression(current.parent)
      || ts.isSatisfiesExpression(current.parent)
      || ts.isNonNullExpression(current.parent))
    && current.parent.expression === current) current = current.parent;
  return current;
};

const declarationHasAnalyzedBody = (declaration, sourceFileSet) => Boolean(
  declaration
  && sourceFileSet.has(declaration.getSourceFile())
  && "body" in declaration
  && declaration.body,
);

const nodeExplicitlyDeclaresFrontendInvoke = (checker, node) => Boolean(
  node?.type && typeIsFrontendInvoke(checker, checker.getTypeFromTypeNode(node.type)),
);

const propertyExplicitlyDeclaresFrontendInvoke = (checker, type, propertyName) => {
  const property = type.getProperty(propertyName);
  return Boolean(property?.declarations?.some((declaration) =>
    nodeExplicitlyDeclaresFrontendInvoke(checker, declaration)));
};

const localCallConsumerAcceptsInvokeProperty = (
  checker,
  objectLiteral,
  propertyName,
  sourceFileSet,
) => {
  const root = transparentReferenceRoot(objectLiteral);
  const call = root.parent;
  if (!ts.isCallExpression(call)) return false;
  const argumentIndex = call.arguments.indexOf(root);
  if (argumentIndex < 0) return false;
  const declaration = checker.getResolvedSignature(call)?.declaration;
  if (!declarationHasAnalyzedBody(declaration, sourceFileSet)) return false;
  const parameter = declaration.parameters?.[argumentIndex];
  return Boolean(parameter && !parameter.dotDotDotToken
    && propertyExplicitlyDeclaresFrontendInvoke(
      checker,
      checker.getTypeAtLocation(parameter),
      propertyName,
    ));
};

const localJsxConsumerAcceptsInvokeProperty = (checker, jsxAttribute, sourceFileSet) => {
  const opening = jsxAttribute.parent;
  if (!ts.isJsxAttributes(opening)) return false;
  const element = opening.parent;
  if (!ts.isJsxOpeningElement(element) && !ts.isJsxSelfClosingElement(element)) return false;
  const symbol = canonicalSymbol(checker, symbolAt(checker, element.tagName));
  for (const declaration of symbol?.declarations ?? []) {
    let implementation = declaration;
    if (ts.isVariableDeclaration(declaration)
      && declaration.initializer
      && (ts.isArrowFunction(declaration.initializer) || ts.isFunctionExpression(declaration.initializer))) {
      implementation = declaration.initializer;
    }
    if (!declarationHasAnalyzedBody(implementation, sourceFileSet)) continue;
    const parameter = implementation.parameters?.[0];
    if (parameter && propertyExplicitlyDeclaresFrontendInvoke(
      checker,
      checker.getTypeAtLocation(parameter),
      jsxAttribute.name.text,
    )) return true;
  }
  return false;
};

const isExplicitAliasSource = (checker, node, sourceFileSet, invokeSymbols) => {
  const root = aliasSourceRoot(checker, node, invokeSymbols);
  const parent = root.parent;
  if (ts.isVariableDeclaration(parent) && parent.initializer === root && ts.isIdentifier(parent.name)) return true;
  if (ts.isBinaryExpression(parent)
    && parent.operatorToken.kind === ts.SyntaxKind.EqualsToken
    && parent.right === root
    && ts.isIdentifier(parent.left)) return true;
  if (ts.isPropertyAssignment(parent)
    && parent.initializer === root
    && isInvokeAliasPropertyName(parent.name.text)
    && ts.isObjectLiteralExpression(parent.parent)) {
    return localCallConsumerAcceptsInvokeProperty(
      checker,
      parent.parent,
      parent.name.text,
      sourceFileSet,
    );
  }
  if (ts.isJsxExpression(parent)
    && parent.expression === root
    && ts.isJsxAttribute(parent.parent)) {
    return isInvokeAliasPropertyName(parent.parent.name.text)
      && localJsxConsumerAcceptsInvokeProperty(checker, parent.parent, sourceFileSet);
  }
  return ts.isShorthandPropertyAssignment(parent)
    && parent.name === root
    && isInvokeAliasPropertyName(parent.name.text)
    && ts.isObjectLiteralExpression(parent.parent)
    && localCallConsumerAcceptsInvokeProperty(
      checker,
      parent.parent,
      parent.name.text,
      sourceFileSet,
    );
};

const isExplicitAliasDestination = (checker, node, invokeSymbols) => {
  const parent = node.parent;
  if (ts.isBinaryExpression(parent)
    && parent.operatorToken.kind === ts.SyntaxKind.EqualsToken
    && parent.left === node
    && expressionResolvesOnlyToInvoke(checker, parent.right, invokeSymbols)) return true;
  if (ts.isPropertyAssignment(parent)
    && parent.initializer === node
    && ts.isObjectLiteralExpression(parent.parent)) {
    const assignment = parent.parent.parent;
    return ts.isBinaryExpression(assignment)
      && assignment.operatorToken.kind === ts.SyntaxKind.EqualsToken
      && unwrapExpression(assignment.left) === parent.parent;
  }
  return false;
};

const memberName = (node) => ts.isPropertyAccessExpression(node)
  ? node.name.text
  : ts.isStringLiteralLike(node.argumentExpression) ? node.argumentExpression.text : null;

const isTrackedLocalHelperArgument = (checker, node, sourceFileSet) => {
  const root = transparentReferenceRoot(node);
  const call = root.parent;
  if (!ts.isCallExpression(call)) return false;
  const argumentIndex = call.arguments.indexOf(root);
  if (argumentIndex < 0) return false;
  const signature = checker.getResolvedSignature(call);
  const declaration = signature?.declaration;
  if (!declarationHasAnalyzedBody(declaration, sourceFileSet)) return false;
  const parameter = declaration.parameters?.[argumentIndex];
  return Boolean(parameter && !parameter.dotDotDotToken
    && nodeExplicitlyDeclaresFrontendInvoke(checker, parameter));
};

const validateInvokeReferences = (checker, sourceFile, sourceFileSet, invokeSymbols) => {
  const errors = [];
  const report = (node, detail) => errors.push(
    `${locationOf(sourceFile, node)} invoke callable escapes static inventory (${detail}; ${ts.SyntaxKind[node.kind]})`,
  );
  const visit = (node) => {
    if ((ts.isPropertyAccessExpression(node) || ts.isElementAccessExpression(node))
      && expressionIsInvokeCallable(checker, node.expression, invokeSymbols)) {
      const member = memberName(node);
      const parent = unwrapExpression(node.parent);
      if (!ts.isCallExpression(parent) || unwrapExpression(parent.expression) !== node) {
        report(node, `uninvoked member ${member ?? "<dynamic>"}`);
      } else if (member !== "call" && member !== "apply" && member !== "bind") {
        report(node, `unsupported member ${member ?? "<dynamic>"}`);
      }
    }

    const candidate = ts.isIdentifier(node)
      || ts.isPropertyAccessExpression(node)
      || ts.isElementAccessExpression(node);
    if (candidate
      && !isTypeOnlyOrDeclarationReference(node)
      && expressionIsInvokeCallable(checker, node, invokeSymbols)) {
      const root = transparentReferenceRoot(node);
      const parent = root.parent;
      if ((ts.isPropertyAccessExpression(parent) || ts.isElementAccessExpression(parent))
        && parent.expression === root) {
        // The member operation is validated above.
      } else if (ts.isCallExpression(parent) && unwrapExpression(parent.expression) === unwrapExpression(root)) {
        // Direct, statically analyzable invoke call.
      } else if (isTrackedLocalHelperArgument(checker, node, sourceFileSet)) {
        // A local implementation with an explicit frontend-invoke parameter is scanned too.
      } else if (isExplicitAliasDestination(checker, node, invokeSymbols)) {
        // A tracked assignment target is not a callable escape.
      } else if (isExplicitAliasSource(checker, node, sourceFileSet, invokeSymbols)) {
        // Explicit alias declarations/assignments remain in the tracked symbol graph.
      } else {
        report(node, ts.SyntaxKind[parent.kind]);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return errors;
};

const analyzeProgram = (program, sourceFiles) => {
  const checker = program.getTypeChecker();
  const sourceFileSet = new Set(sourceFiles);
  const invokeSymbols = collectInvokeSymbols(checker, sourceFiles);
  const commands = new Set();
  const errors = [];
  for (const sourceFile of sourceFiles) {
    errors.push(...validateInvokeReferences(checker, sourceFile, sourceFileSet, invokeSymbols));
    const visit = (node) => {
      const invokeCall = invokeCommandArgument(checker, node, invokeSymbols);
      if (invokeCall) {
        const { argument, escape } = invokeCall;
        if (escape) {
          errors.push(`${locationOf(sourceFile, node)} invoke.${escape} is not statically inventory-safe`);
          ts.forEachChild(node, visit);
          return;
        }
        if (!argument) {
          errors.push(`${locationOf(sourceFile, node)} invoke has no command argument`);
        } else {
          const values = commandsAtArgument(checker, argument);
          if (!values || values.size === 0) {
            const type = checker.typeToString(checker.getTypeAtLocation(argument));
            errors.push(
              `${locationOf(sourceFile, argument)} dynamic invoke is not a finite string-literal union (${ts.SyntaxKind[argument.kind]}, type ${type})`,
            );
          } else if (!isExplicitManifestType(checker, argument)) {
            for (const value of values) commands.add(value);
          }
        }
      }
      ts.forEachChild(node, visit);
    };
    visit(sourceFile);
  }
  return { commands, errors };
};

const compareManifest = (commands, manifest) => {
  const errors = [];
  if (!Array.isArray(manifest) || manifest.some((value) => typeof value !== "string")) {
    return ["manifest must be a JSON array containing only strings"];
  }
  const sorted = [...manifest].sort();
  if (new Set(manifest).size !== manifest.length) errors.push("manifest contains duplicate commands");
  if (manifest.some((value, index) => value !== sorted[index])) {
    errors.push("manifest commands must be sorted byte-for-byte");
  }
  for (const command of manifest) {
    const bytes = Buffer.from(command, "utf8");
    const valid = bytes.length > 0
      && bytes[0] >= 0x61
      && bytes[0] <= 0x7a
      && bytes.every((byte) => (byte >= 0x61 && byte <= 0x7a)
        || (byte >= 0x30 && byte <= 0x39)
        || byte === 0x5f);
    if (!valid) {
      errors.push(`manifest command is not lower_snake ASCII: ${JSON.stringify(command)}`);
    }
  }
  const manifestSet = new Set(manifest);
  for (const command of [...commands].sort()) {
    if (!manifestSet.has(command)) errors.push(`manifest is missing invoked command: ${command}`);
  }
  for (const command of manifest) {
    if (!commands.has(command)) errors.push(`manifest has no matching invoke call: ${command}`);
  }
  return errors;
};

const typedManifestFromProgram = (program) => {
  const sourceFile = program.getSourceFiles().find((candidate) =>
    path.basename(candidate.fileName) === "tauriInvokeCommands.ts");
  if (!sourceFile) throw new Error("typed frontend invoke manifest source is missing");
  for (const statement of sourceFile.statements) {
    if (!ts.isVariableStatement(statement)) continue;
    for (const declaration of statement.declarationList.declarations) {
      if (!ts.isIdentifier(declaration.name)
        || declaration.name.text !== "FRONTEND_TAURI_INVOKE_COMMANDS"
        || !declaration.initializer) continue;
      let initializer = declaration.initializer;
      while (ts.isAsExpression(initializer) || ts.isSatisfiesExpression(initializer)) {
        initializer = initializer.expression;
      }
      if (!ts.isArrayLiteralExpression(initializer)) {
        throw new Error("typed frontend invoke manifest must be an array literal");
      }
      return initializer.elements.map((element) => {
        if (!ts.isStringLiteralLike(element)) {
          throw new Error("typed frontend invoke manifest entries must be string literals");
        }
        return element.text;
      });
    }
  }
  throw new Error("typed frontend invoke manifest declaration is missing");
};

const fixtureProgram = (files) => {
  const fixtureRoot = path.join(appRoot, "__frontend_invoke_fixtures__");
  const coreFile = path.join(fixtureRoot, "node_modules", "@tauri-apps", "api", "core.d.ts");
  const virtualFiles = new Map(Object.entries(files)
    .filter(([, source]) => typeof source === "string")
    .map(([name, source]) => [path.resolve(fixtureRoot, name), source]));
  virtualFiles.set(coreFile, "export declare function invoke<T>(command: string, args?: unknown): Promise<T>;");
  const options = {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.Node10,
    jsx: ts.JsxEmit.Preserve,
    strict: true,
    noLib: true,
  };
  const host = ts.createCompilerHost(options);
  const originalGetSourceFile = host.getSourceFile.bind(host);
  host.fileExists = (candidate) => virtualFiles.has(path.resolve(candidate));
  host.readFile = (candidate) => virtualFiles.get(path.resolve(candidate));
  host.getSourceFile = (candidate, languageVersion, onError, shouldCreateNewSourceFile) => {
    const source = virtualFiles.get(path.resolve(candidate));
    return source === undefined
      ? originalGetSourceFile(candidate, languageVersion, onError, shouldCreateNewSourceFile)
      : ts.createSourceFile(candidate, source, languageVersion, true);
  };
  host.resolveModuleNames = (moduleNames, containingFile) => moduleNames.map((moduleName) => {
    if (moduleName === "@tauri-apps/api/core") {
      return { resolvedFileName: coreFile, extension: ts.Extension.Dts };
    }
    if (moduleName.startsWith(".")) {
      const base = path.resolve(path.dirname(containingFile), moduleName);
      for (const [candidate, extension] of [
        [`${base}.ts`, ts.Extension.Ts],
        [`${base}.tsx`, ts.Extension.Tsx],
        [`${base}.d.ts`, ts.Extension.Dts],
        [path.join(base, "index.ts"), ts.Extension.Ts],
      ]) {
        if (virtualFiles.has(candidate)) return { resolvedFileName: candidate, extension };
      }
    }
    return undefined;
  });
  const rootNames = [...virtualFiles.keys()].filter((fileName) =>
    (fileName.endsWith(".ts") || fileName.endsWith(".tsx")) && !fileName.endsWith(".d.ts"));
  const program = ts.createProgram(rootNames, options, host);
  const sourceFiles = rootNames.map((fileName) => program.getSourceFile(fileName)).filter(Boolean);
  return { program, sourceFiles };
};

const analyzeFixture = (body, extraFiles = {}, includeDirectImport = true) => {
  const files = {
    "main.ts": `${includeDirectImport ? 'import { invoke } from "@tauri-apps/api/core";\n' : ""}${body}\n`,
    ...extraFiles,
  };
  const { program, sourceFiles } = fixtureProgram(files);
  return analyzeProgram(program, sourceFiles);
};

const runSelfTests = () => {
  const assertAnalysisDiscovered = (label, analysis) => {
    if (analysis.errors.length !== 0
      || compareManifest(analysis.commands, [])
        .every((error) => error !== "manifest is missing invoked command: untracked_command")) {
      throw new Error(`${label} fixture was not nonvacuously inventoried: ${analysis.errors.join("; ")}`);
    }
  };
  const assertDiscovered = (label, body, extraFiles = {}, includeDirectImport = true) => {
    assertAnalysisDiscovered(label, analyzeFixture(body, extraFiles, includeDirectImport));
  };
  const assertAnalysisEscape = (label, analysis, expected) => {
    if (analysis.errors.length === 0 || !analysis.errors.some((error) => error.includes(expected))) {
      throw new Error(`${label} escape fixture was not rejected: ${analysis.errors.join("; ")}`);
    }
  };
  const assertEscape = (label, body, expected) => {
    assertAnalysisEscape(label, analyzeFixture(body), expected);
  };
  const finite = analyzeFixture(
    'type Command = "alpha" | "beta"; declare const command: Command; invoke(command); invoke(true ? "alpha" : "beta");',
  );
  if (finite.errors.length !== 0 || [...finite.commands].sort().join(",") !== "alpha,beta") {
    throw new Error(`finite-union fixture failed: ${finite.errors.join("; ")}`);
  }
  const unknownIdentifier = analyzeFixture("declare const command: string; invoke(command);");
  if (unknownIdentifier.errors.length !== 1) throw new Error("unknown Identifier fixture did not fail");
  const unknownConditional = analyzeFixture(
    'declare const command: string; invoke(true ? "alpha" : command);',
  );
  if (unknownConditional.errors.length !== 1) throw new Error("unknown Conditional fixture did not fail");
  const literal = analyzeFixture('invoke("alpha");');
  if (compareManifest(literal.commands, []).every((error) => !error.includes("missing"))) {
    throw new Error("missing-manifest fixture did not fail");
  }
  if (compareManifest(literal.commands, ["alpha", "extra"]).every((error) => !error.includes("no matching"))) {
    throw new Error("extra-manifest fixture did not fail");
  }
  assertDiscovered(
    "variable",
    'const forwarded = invoke; forwarded("untracked_command");',
  );
  assertDiscovered(
    "plain assignment",
    'let forwarded: typeof invoke; forwarded = invoke; forwarded("untracked_command");',
  );
  assertDiscovered(
    "destructuring assignment",
    'import * as core from "@tauri-apps/api/core"; let forwarded: typeof core.invoke; ({ invoke: forwarded } = core); forwarded("untracked_command");',
    {},
    false,
  );
  assertDiscovered(
    "parenthesized as alias",
    'const forwarded = ((invoke as typeof invoke)); forwarded("untracked_command");',
  );
  assertDiscovered(
    "conditional alias",
    'declare const condition: boolean; const forwarded = condition ? invoke : invoke; forwarded("untracked_command");',
  );
  assertDiscovered(
    "Function.call",
    'invoke.call(null, "untracked_command");',
  );
  assertDiscovered(
    "bracket Function.call",
    'invoke["call"](null, "untracked_command");',
  );
  const barrel = analyzeFixture(
    'import { forwardedInvoke } from "./barrel"; forwardedInvoke("untracked_command");',
    { "barrel.ts": 'export { invoke as forwardedInvoke } from "@tauri-apps/api/core";' },
    false,
  );
  if (barrel.errors.length !== 0 || !barrel.commands.has("untracked_command")) {
    throw new Error(`barrel/re-export alias fixture was not inventoried: ${barrel.errors.join("; ")}`);
  }
  assertEscape("comma call", '(0, invoke)("untracked_command");', "escapes static inventory");
  assertEscape("dot apply", 'invoke.apply(null, ["untracked_command"]);', "invoke.apply");
  assertEscape("bracket apply", 'invoke["apply"](null, ["untracked_command"]);', "invoke.apply");
  assertEscape("dot bind", 'invoke.bind(null, "untracked_command");', "invoke.bind");
  assertEscape("bracket bind", 'invoke["bind"](null, "untracked_command");', "invoke.bind");
  assertEscape(
    "mixed conditional alias",
    'declare const condition: boolean; declare const other: typeof invoke; const forwarded = condition ? invoke : other;',
    "ConditionalExpression",
  );
  assertEscape(
    "helper callback passing",
    'declare function consume(callback: typeof invoke): void; consume(invoke);',
    "CallExpression",
  );
  assertEscape(
    "wrapped helper callback passing",
    'declare function consume(callback: typeof invoke): void; consume((invoke as typeof invoke));',
    "CallExpression",
  );
  assertEscape(
    "helper callback property passing",
    'declare function consume(options: { callback: typeof invoke }): void; consume({ callback: invoke });',
    "PropertyAssignment",
  );
  assertDiscovered(
    "typed local helper forwarding",
    'type FrontendTauriInvokeCommand = "untracked_command"; type FrontendTauriInvoke = <T>(command: FrontendTauriInvokeCommand) => Promise<T>; const forwarded = invoke as unknown as FrontendTauriInvoke; function consume(callback: FrontendTauriInvoke) { return callback("untracked_command"); } consume(forwarded);',
  );
  assertEscape("uninvoked call member", 'const escaped = invoke["call"];', "uninvoked member call");
  assertEscape("unsupported member call", 'invoke["unknown"]("untracked_command");', "unsupported member unknown");
  const frontendTypes = 'export type FrontendTauriInvokeCommand = "untracked_command"; export type FrontendTauriInvoke = <T>(command: FrontendTauriInvokeCommand) => Promise<T>;';
  const localObject = fixtureProgram({
    "main.ts": 'import { invoke } from "@tauri-apps/api/core"; import type { FrontendTauriInvoke } from "./types"; function local(options: { invoke: FrontendTauriInvoke }) { return options.invoke("untracked_command"); } local({ invoke: invoke as unknown as FrontendTauriInvoke });',
    "types.ts": frontendTypes,
  });
  assertAnalysisDiscovered(
    "local analyzed object consumer",
    analyzeProgram(localObject.program, localObject.sourceFiles),
  );
  const localJsx = fixtureProgram({
    "main.tsx": 'import { invoke } from "@tauri-apps/api/core"; import type { FrontendTauriInvoke } from "./types"; import { Local } from "./local"; <Local invoke={invoke as unknown as FrontendTauriInvoke} />;',
    "local.tsx": 'import type { FrontendTauriInvoke } from "./types"; export function Local(props: { invoke: FrontendTauriInvoke }) { props.invoke("untracked_command"); return null; }',
    "types.ts": frontendTypes,
  });
  assertAnalysisDiscovered(
    "local analyzed JSX consumer",
    analyzeProgram(localJsx.program, localJsx.sourceFiles),
  );
  assertEscape(
    "external object consumer",
    'type FrontendTauriInvokeCommand = "untracked_command"; type FrontendTauriInvoke = <T>(command: FrontendTauriInvokeCommand) => Promise<T>; declare function external(options: { invoke: FrontendTauriInvoke }): void; external({ invoke: invoke as unknown as FrontendTauriInvoke });',
    "PropertyAssignment",
  );
  assertEscape(
    "standalone invokeCommand property",
    'const escaped = { invokeCommand: invoke };',
    "PropertyAssignment",
  );
  const externalJsx = fixtureProgram({
    "main.tsx": 'import { invoke } from "@tauri-apps/api/core"; import type { FrontendTauriInvoke } from "./types"; import { External } from "./external"; <External invoke={invoke as unknown as FrontendTauriInvoke} />;',
    "types.ts": frontendTypes,
    "external.d.ts": 'import type { FrontendTauriInvoke } from "./types"; export declare function External(props: { invoke: FrontendTauriInvoke }): unknown;',
  });
  assertAnalysisEscape(
    "external JSX consumer",
    analyzeProgram(externalJsx.program, externalJsx.sourceFiles),
    "JsxExpression",
  );
};

const loadApplicationProgram = () => {
  const configPath = path.join(appRoot, "tsconfig.json");
  const config = ts.readConfigFile(configPath, ts.sys.readFile);
  if (config.error) throw new Error(ts.flattenDiagnosticMessageText(config.error.messageText, "\n"));
  const parsed = ts.parseJsonConfigFileContent(config.config, ts.sys, appRoot, undefined, configPath);
  if (parsed.errors.length > 0) {
    throw new Error(parsed.errors.map((error) => ts.flattenDiagnosticMessageText(error.messageText, "\n")).join("\n"));
  }
  const program = ts.createProgram(parsed.fileNames, parsed.options);
  const sourceFiles = program.getSourceFiles().filter((sourceFile) => {
    const relative = path.relative(path.join(appRoot, "src"), sourceFile.fileName);
    return !relative.startsWith("..") && !path.isAbsolute(relative) && !sourceFile.isDeclarationFile;
  });
  return { program, sourceFiles };
};

runSelfTests();
const { program, sourceFiles } = loadApplicationProgram();
const analysis = analyzeProgram(program, sourceFiles);
if (process.argv.includes("--print")) {
  console.log(JSON.stringify([...analysis.commands].sort(), null, 2));
  if (analysis.errors.length > 0) {
    console.error(analysis.errors.join("\n"));
    process.exitCode = 1;
  }
} else {
  let manifest;
  try {
    manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  } catch (error) {
    console.error(`failed to read ${path.relative(appRoot, manifestPath)}: ${error}`);
    process.exitCode = 1;
  }
  const typedManifest = manifest ? typedManifestFromProgram(program) : [];
  const errors = [
    ...analysis.errors,
    ...(manifest ? compareManifest(analysis.commands, manifest) : []),
  ];
  if (manifest && JSON.stringify(typedManifest) !== JSON.stringify(manifest)) {
    errors.push("typed const tuple and embedded JSON manifest differ");
  }
  if (errors.length > 0) {
    console.error(errors.join("\n"));
    process.exitCode = 1;
  } else {
    console.log(`frontend Tauri invoke inventory exact: ${analysis.commands.size} commands`);
  }
}

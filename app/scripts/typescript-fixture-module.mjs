import { readFile } from "node:fs/promises";
import ts from "typescript";

/** Compile the real module and its local imports for isolated Node contracts. */
export async function typescriptFixtureModuleUrl(sourceUrl) {
  const source = await readFile(sourceUrl, "utf8");
  let output = ts.transpileModule(source, {
    compilerOptions: {module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022},
    fileName: sourceUrl.pathname,
  }).outputText;
  const ast = ts.createSourceFile("fixture.js", output, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
  const edits = [];
  for (const statement of ast.statements) {
    if (!ts.isImportDeclaration(statement) || !ts.isStringLiteral(statement.moduleSpecifier)
      || !statement.moduleSpecifier.text.startsWith(".")) continue;
    const specifier = statement.moduleSpecifier;
    const target = new URL(specifier.text + (/\.[cm]?[jt]sx?$/.test(specifier.text) ? "" : ".ts"), sourceUrl);
    edits.push({start: specifier.getStart(ast), end: specifier.getEnd(), replacement: JSON.stringify(await typescriptFixtureModuleUrl(target))});
  }
  for (const edit of edits.reverse()) output = output.slice(0, edit.start) + edit.replacement + output.slice(edit.end);
  return "data:text/javascript;base64," + Buffer.from(output).toString("base64");
}

// Guard: the WebView2 browser dialogs ("tauri.localhost says...") must never be
// used. Every question goes through the in-app dialog service (src/inAppDialog.ts).
// The scan is syntax-aware (TypeScript AST), so comments and strings are ignored.
import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const forbidden = new Set(["confirm", "alert", "prompt"]);
const globals = new Set(["window", "globalThis", "self", "top", "parent"]);

/** Returns one finding per browser-dialog reference in `text`. */
export function findBrowserDialogs(text, fileName = "source.tsx") {
  const kind = fileName.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS;
  const source = ts.createSourceFile(fileName, text, ts.ScriptTarget.Latest, true, kind);
  const findings = [];
  const report = (node, what) => {
    const { line, character } = source.getLineAndCharacterOfPosition(node.getStart(source));
    findings.push({ line: line + 1, column: character + 1, what });
  };
  const visit = (node) => {
    // confirm(...), alert(...), prompt(...) as bare calls.
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && forbidden.has(node.expression.text)) {
      report(node, `${node.expression.text}(...)`);
    }
    // window.confirm / globalThis.alert / self["prompt"], called or merely referenced.
    if (ts.isPropertyAccessExpression(node) && ts.isIdentifier(node.expression)
      && globals.has(node.expression.text) && forbidden.has(node.name.text)) {
      report(node, `${node.expression.text}.${node.name.text}`);
    }
    if (ts.isElementAccessExpression(node) && ts.isIdentifier(node.expression) && globals.has(node.expression.text)
      && ts.isStringLiteralLike(node.argumentExpression) && forbidden.has(node.argumentExpression.text)) {
      report(node, `${node.expression.text}["${node.argumentExpression.text}"]`);
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
  return findings;
}

async function collect(directory, files = []) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) await collect(path, files);
    else if (/\.(ts|tsx)$/.test(entry.name) && !entry.name.endsWith(".d.ts")) files.push(path);
  }
  return files;
}

function selfTest() {
  const hit = (text) => findBrowserDialogs(text).length;
  assert.equal(hit('if (!window.confirm("x")) return;'), 1);
  assert.equal(hit('globalThis.confirm("x")'), 1);
  assert.equal(hit('confirm("x")'), 1);
  assert.equal(hit('alert("x"); prompt("y")'), 2);
  assert.equal(hit('const ask = window["prompt"];'), 1);
  assert.equal(hit('// window.confirm("x")\nconst text = "confirm(x)";'), 0, "comments and strings are ignored");
  assert.equal(hit("await confirmDialog({ title: 'x', message: 'y' });"), 0, "the in-app service is allowed");
  assert.equal(hit("const confirmed = options.confirm(1); other.alert(2);"), 0, "member calls on other objects are allowed");
  console.log("check-no-browser-dialogs self-test ok");
}

selfTest();
const root = fileURLToPath(new URL("../src/", import.meta.url));
const files = await collect(root);
assert.ok(files.length > 100, "the scan must cover the application source");
const offenders = [];
for (const file of files) {
  for (const finding of findBrowserDialogs(await readFile(file, "utf8"), file)) {
    offenders.push(`${relative(root, file)}:${finding.line}:${finding.column} ${finding.what}`);
  }
}
assert.deepEqual(
  offenders,
  [],
  `browser dialogs are forbidden; use confirmDialog/alertDialog from src/inAppDialog.ts:\n${offenders.join("\n")}`,
);
console.log(`no browser dialogs in ${files.length} source files`);

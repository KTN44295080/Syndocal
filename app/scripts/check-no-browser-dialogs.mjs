// Guard: the WebView2 browser dialogs ("tauri.localhost says...") must never be
// used. Every question goes through the in-app dialog service (src/inAppDialog.ts).
// The scan is syntax-aware (TypeScript AST), so comments and strings are ignored.
// The remote-control web page embedded in crates/io/src/remote_ws.rs runs in the
// remote device's own browser and is held to the same rule: its script is
// AST-scanned, and the whole page is also scanned as text, because it builds
// inline event handlers inside template strings that an AST scan cannot see.
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

const remoteCall = /(?<![\w$.])(?:(?:window|globalThis|self|top|parent)\s*\.\s*)?(?:confirm|alert|prompt)\s*\(/g;
const remoteIndexed = /\[\s*["'`](?:confirm|alert|prompt)["'`]\s*\]/g;

/** Returns one finding per browser-dialog use anywhere in the remote page HTML. */
export function findRemotePageBrowserDialogs(html) {
  const findings = [];
  const lineOf = (offset) => html.slice(0, offset).split("\n").length;
  for (const pattern of [remoteCall, remoteIndexed]) {
    for (const match of html.matchAll(pattern)) findings.push({ line: lineOf(match.index), what: match[0] });
  }
  for (const script of html.matchAll(/<script[^>]*>([\s\S]*?)<\/script>/g)) {
    const before = html.slice(0, script.index + script[0].indexOf(">") + 1).split("\n").length - 1;
    for (const finding of findBrowserDialogs(script[1], "remote-page.js")) {
      findings.push({ line: before + finding.line, what: finding.what });
    }
  }
  return findings;
}

function remotePageHtml(rust) {
  const open = rust.indexOf('const REMOTE_PAGE_HTML: &str = r##"');
  assert.ok(open >= 0, "REMOTE_PAGE_HTML must stay a raw string in crates/io/src/remote_ws.rs");
  const start = open + 'const REMOTE_PAGE_HTML: &str = r##"'.length;
  const end = rust.indexOf('"##;', start);
  assert.ok(end > start, "REMOTE_PAGE_HTML must be terminated");
  return rust.slice(start, end);
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
  const remoteHit = (text) => findRemotePageBrowserDialogs(text).length;
  assert.ok(remoteHit('<script>if(window.confirm("x"))go()</script>') >= 1);
  assert.ok(remoteHit("<script>const html=`<button onclick=\"confirm('x')\">`</script>") >= 1, "inline handlers in templates");
  assert.ok(remoteHit('<button onclick="window.prompt(1)">') >= 1);
  assert.ok(remoteHit('<script>window["alert"]("x")</script>') >= 1);
  assert.equal(remoteHit("<script>remoteAskDialog({title:'x'});options.confirm(1);const confirmed=1;</script>"), 0);
  assert.equal(remoteHit("<script>// confirm/alert/prompt are not used\nconfirmButton.onclick=()=>1</script>"), 0);
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
const remotePath = fileURLToPath(new URL("../../crates/io/src/remote_ws.rs", import.meta.url));
const remoteFindings = findRemotePageBrowserDialogs(remotePageHtml(await readFile(remotePath, "utf8")));
assert.deepEqual(
  remoteFindings,
  [],
  `the embedded remote page must use its in-page dialog, not browser dialogs:\n${remoteFindings.map((finding) => `remote_ws.rs page line ${finding.line} ${finding.what}`).join("\n")}`,
);
console.log(`no browser dialogs in ${files.length} source files and the embedded remote page`);

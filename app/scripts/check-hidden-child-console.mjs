// Guards the Windows "no console window" contract:
//   1. release builds of the app are GUI-subsystem executables, and
//   2. production Rust code builds every child process with
//      `protocol::hidden_console_command::hidden_command` (CREATE_NO_WINDOW)
//      instead of a bare `Command::new`, so console children (ffmpeg, curl, ...)
//      never flash a console window.
// Test-only code (`#[cfg(test)]`, `#[test]`, `*_tests.rs`, `tests/`) is exempt.
import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const helperPath = path.join("crates", "protocol", "src", "hidden_console_command.rs");

// Replaces comments, string literals and char literals with spaces while keeping
// newlines and offsets, so structural scanning cannot be fooled by their content.
export function blankRustNonCode(source) {
  const out = source.split("");
  const blank = (from, to) => {
    for (let index = from; index < to; index += 1) {
      if (out[index] !== "\n") out[index] = " ";
    }
  };
  const isIdent = (char) => char !== undefined && /[A-Za-z0-9_]/.test(char);
  let i = 0;
  while (i < source.length) {
    const char = source[i];
    const next = source[i + 1];
    if (char === "/" && next === "/") {
      let end = source.indexOf("\n", i);
      if (end < 0) end = source.length;
      blank(i, end);
      i = end;
    } else if (char === "/" && next === "*") {
      let depth = 1;
      let end = i + 2;
      while (end < source.length && depth > 0) {
        if (source[end] === "/" && source[end + 1] === "*") {
          depth += 1;
          end += 2;
        } else if (source[end] === "*" && source[end + 1] === "/") {
          depth -= 1;
          end += 2;
        } else {
          end += 1;
        }
      }
      blank(i, end);
      i = end;
    } else if (
      (char === "r" || (char === "b" && next === "r")) &&
      !isIdent(source[i - 1]) &&
      /^b?r#*"/.test(source.slice(i, i + 40))
    ) {
      const match = /^b?r(#*)"/.exec(source.slice(i, i + 40));
      const closing = `"${match[1]}`;
      const bodyStart = i + match[0].length;
      let end = source.indexOf(closing, bodyStart);
      end = end < 0 ? source.length : end + closing.length;
      blank(i, end);
      i = end;
    } else if (char === '"') {
      let end = i + 1;
      while (end < source.length && source[end] !== '"') end += source[end] === "\\" ? 2 : 1;
      end += 1;
      blank(i, end);
      i = end;
    } else if (char === "'") {
      let end = -1;
      if (next === "\\") {
        end = source.indexOf("'", i + 3);
      } else if (next !== undefined) {
        const width = next.codePointAt(0) > 0xffff ? 2 : 1;
        if (source[i + 1 + width] === "'") end = i + 1 + width;
      }
      if (end >= 0) {
        blank(i, end + 1);
        i = end + 1;
      } else {
        i += 1; // lifetime
      }
    } else {
      i += 1;
    }
  }
  return out.join("");
}

function matchingClose(code, openIndex) {
  const pairs = { "{": "}", "(": ")", "[": "]" };
  let depth = 0;
  for (let index = openIndex; index < code.length; index += 1) {
    const char = code[index];
    if (char === "{" || char === "(" || char === "[") depth += 1;
    else if (char === "}" || char === ")" || char === "]") {
      depth -= 1;
      if (depth === 0) {
        assert.equal(char, pairs[code[openIndex]], "unbalanced Rust delimiters");
        return index;
      }
    }
  }
  return code.length - 1;
}

const TEST_ATTRIBUTE = /^(test|tokio::test|cfg\(test\)|cfg\(all\(test\b.*)$/;

// Returns [start, end] offset ranges covered by test-only items.
export function testRegions(code) {
  const regions = [];
  const attribute = /#\s*\[/g;
  let match;
  let skipUntil = 0;
  while ((match = attribute.exec(code))) {
    if (match.index < skipUntil) continue;
    const open = match.index + match[0].length - 1;
    const close = matchingClose(code, open);
    const body = code.slice(open + 1, close).replace(/\s+/g, "");
    if (!TEST_ATTRIBUTE.test(body)) continue;
    // Skip any further attributes on the same item.
    let cursor = close + 1;
    for (;;) {
      const following = /^\s*#\s*\[/.exec(code.slice(cursor));
      if (!following) break;
      cursor = matchingClose(code, cursor + following[0].length - 1) + 1;
    }
    // The item ends at its first top-level `{ ... }` block or `;`.
    let end = code.length - 1;
    for (let index = cursor; index < code.length; index += 1) {
      const char = code[index];
      if (char === "(" || char === "[") {
        index = matchingClose(code, index);
      } else if (char === "{") {
        end = matchingClose(code, index);
        break;
      } else if (char === ";") {
        end = index;
        break;
      }
    }
    regions.push([match.index, end]);
    skipUntil = end + 1;
  }
  return regions;
}

export function bareCommandSites(source) {
  const code = blankRustNonCode(source);
  const regions = testRegions(code);
  const sites = [];
  // `Command::new`/`Command::default` (also via `std::process::` or `tokio::process::`),
  // plus aliasing or glob imports that would hide a later bare constructor.
  const command =
    /\bCommand\s*::\s*(?:new|default)\b|\bCommand\s+as\s+[A-Za-z_]\w*|\bprocess\s*::\s*\*|\bprocess\s*::\s*\{[^}]*(?:\*|\bCommand\s+as\b)/g;
  let match;
  while ((match = command.exec(code))) {
    if (regions.some(([start, end]) => match.index >= start && match.index <= end)) continue;
    sites.push(code.slice(0, match.index).split("\n").length);
  }
  return sites;
}

async function rustFiles(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "tests" || entry.name === "target") continue;
      files.push(...(await rustFiles(full)));
    } else if (entry.name.endsWith(".rs") && !entry.name.endsWith("_tests.rs")) {
      files.push(full);
    }
  }
  return files;
}

function selfTest() {
  const sample = [
    'fn production() { let _ = std::process::Command::new("a"); }',
    '// Command::new("in a comment")',
    'const TEXT: &str = "Command::new(\\"in a string\\")";',
    "#[cfg(test)]",
    "mod tests { fn t<'a>(x: &'a str) { let _ = Command::new(x); let c = '{'; } }",
    '#[cfg(windows)] #[test] fn windows_only() { Command::new("b"); }',
    '#[cfg(not(test))] fn not_test() { Command::new("c"); }',
    "#[cfg(test)] use std::process::Command;",
    'fn after() { Command::new("d"); }',
    "use std::process::Command as Cmd;",
    "use std::process::*;",
    "use tokio::process::{Child, Command as AsyncCommand};",
    "fn dflt() { let _ = tokio::process::Command::default(); }",
  ].join("\n");
  assert.deepEqual(bareCommandSites(sample), [1, 7, 9, 10, 11, 12, 13]);
}

if (process.argv.includes("--self-test")) {
  selfTest();
  console.log("hidden child console guard self-test passed");
  process.exit(0);
}

selfTest();

const main = await readFile(path.join(repoRoot, "app", "src-tauri", "src", "main.rs"), "utf8");
assert.match(
  main,
  /^#!\[cfg_attr\(all\(not\(debug_assertions\), not\(test\)\), windows_subsystem = "windows"\)\]\r?$/m,
  'app/src-tauri/src/main.rs must declare #![cfg_attr(all(not(debug_assertions), not(test)), windows_subsystem = "windows")] so release builds do not open a console window',
);
assert.match(
  main,
  /updater_release_identity_cli_requested\(env::args_os\(\)\.skip\(1\)\) \{\s*cli_console::attach_parent_console_for_cli\(\);/,
  "the terminal-invoked updater identity CLI mode must attach the parent console before printing",
);

const helper = await readFile(path.join(repoRoot, helperPath), "utf8");
assert.match(helper, /CREATE_NO_WINDOW: u32 = 0x0800_0000/, "hidden_command must use CREATE_NO_WINDOW");
assert.match(helper, /creation_flags\(CREATE_NO_WINDOW\)/, "hidden_command must apply CREATE_NO_WINDOW");

const roots = [path.join(repoRoot, "app", "src-tauri", "src")];
for (const entry of await readdir(path.join(repoRoot, "crates"), { withFileTypes: true })) {
  if (entry.isDirectory()) roots.push(path.join(repoRoot, "crates", entry.name, "src"));
}
const offenders = [];
let scanned = 0;
for (const root of roots) {
  for (const file of await rustFiles(root)) {
    const relative = path.relative(repoRoot, file);
    if (relative === helperPath) continue;
    scanned += 1;
    for (const line of bareCommandSites(await readFile(file, "utf8"))) {
      offenders.push(`${relative.replaceAll("\\", "/")}:${line}`);
    }
  }
}
assert.deepEqual(
  offenders,
  [],
  `production code must spawn children with protocol::hidden_console_command::hidden_command, not Command::new (a bare Command::new opens a console window on Windows):\n${offenders.join("\n")}`,
);
console.log(`hidden child console guard passed (${scanned} Rust files scanned)`);

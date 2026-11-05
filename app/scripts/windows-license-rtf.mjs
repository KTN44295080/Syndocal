// Renders THIRD_PARTY_NOTICES.md as the ASCII-only RTF that the ordinary
// Windows installers show as their component license. The Japanese MSI
// database uses code page 932, which cannot hold characters such as the
// required "NDI®" trademark sign, so every non-ASCII character is written as
// an RTF Unicode escape (\uN?) instead of raw text. The output is
// deterministic; check-release-metadata.mjs verifies the committed copy.
//
//   node scripts/windows-license-rtf.mjs --write   regenerate the RTF
//   node scripts/windows-license-rtf.mjs --check   fail if it is stale
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const WINDOWS_LICENSE_RTF = "app/src-tauri/windows/THIRD_PARTY_NOTICES.rtf";
export const WINDOWS_LICENSE_SOURCE = "THIRD_PARTY_NOTICES.md";

function escapeRtfText(text) {
  let out = "";
  for (const ch of text) {
    const code = ch.codePointAt(0);
    if (ch === "\\" || ch === "{" || ch === "}") {
      out += `\\${ch}`;
    } else if (ch === "\t") {
      out += "\\tab ";
    } else if (code < 0x20) {
      throw new Error(`THIRD_PARTY_NOTICES.md contains control character U+${code.toString(16).padStart(4, "0")}`);
    } else if (code < 0x80) {
      out += ch;
    } else {
      // RTF \u takes a signed 16-bit value; astral characters use UTF-16 halves.
      for (const unit of String.fromCodePoint(code).split("")) {
        const value = unit.charCodeAt(0);
        out += `\\u${value > 0x7fff ? value - 0x10000 : value}?`;
      }
    }
  }
  return out;
}

export function renderWindowsLicenseRtf(markdown) {
  const lines = String(markdown).replace(/\r\n/g, "\n").replace(/\n$/, "").split("\n");
  const body = lines.map((line) => `${escapeRtfText(line)}\\par`).join("\n");
  return "{\\rtf1\\ansi\\ansicpg1252\\deff0\\uc1{\\fonttbl{\\f0\\fnil\\fcharset0 Calibri;}}\n"
    + "\\viewkind4\\pard\\sa120\\f0\\fs18\n"
    + `${body}\n}\n`;
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
  const mode = process.argv[2];
  const expected = renderWindowsLicenseRtf(readFileSync(path.join(repoRoot, WINDOWS_LICENSE_SOURCE), "utf8"));
  const target = path.join(repoRoot, WINDOWS_LICENSE_RTF);
  if (mode === "--write") {
    writeFileSync(target, expected, "ascii");
    console.log(`wrote ${WINDOWS_LICENSE_RTF}`);
  } else if (mode === "--check") {
    if (readFileSync(target, "latin1") !== expected) {
      throw new Error(`${WINDOWS_LICENSE_RTF} is stale; run node app/scripts/windows-license-rtf.mjs --write`);
    }
    console.log(`${WINDOWS_LICENSE_RTF} matches ${WINDOWS_LICENSE_SOURCE}`);
  } else {
    throw new Error("usage: windows-license-rtf.mjs --write | --check");
  }
}

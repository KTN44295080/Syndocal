import assert from "node:assert/strict";
import test from "node:test";
import { assertPeImageDelayLoadsNdi, assertPeImageHasNoNdiReference, readPeImports } from "./pe-imports.mjs";

function fixture({ regular = [], delayed = [], legalText = "", delayAttributes = 1, imageBase = 0x140000000n } = {}) {
  const bytes = Buffer.alloc(0x1200);
  bytes.write("MZ", 0, "latin1");
  bytes.writeUInt32LE(0x80, 0x3c);
  bytes.write("PE\0\0", 0x80, "latin1");
  bytes.writeUInt16LE(0x8664, 0x84);
  bytes.writeUInt16LE(1, 0x86);
  bytes.writeUInt16LE(240, 0x94);
  bytes.writeUInt16LE(0x22, 0x96);
  const optional = 0x98;
  bytes.writeUInt16LE(0x20b, optional);
  bytes.writeBigUInt64LE(imageBase, optional + 24);
  bytes.writeUInt32LE(0x200, optional + 60);
  bytes.writeUInt32LE(16, optional + 108);
  const section = optional + 240;
  bytes.write(".rdata", section, "latin1");
  bytes.writeUInt32LE(0x1000, section + 8);
  bytes.writeUInt32LE(0x1000, section + 12);
  bytes.writeUInt32LE(0x1000, section + 16);
  bytes.writeUInt32LE(0x200, section + 20);
  let nameOffset = 0x900;
  const toRva = (offset) => offset - 0x200 + 0x1000;
  for (const [names, directoryIndex, table, descriptorSize] of [[regular, 1, 0x200, 20], [delayed, 13, 0x400, 32]]) {
    if (!names.length) continue;
    const directory = optional + 112 + directoryIndex * 8;
    bytes.writeUInt32LE(toRva(table), directory);
    bytes.writeUInt32LE((names.length + 1) * descriptorSize, directory + 4);
    names.forEach((name, index) => {
      const descriptor = table + index * descriptorSize;
      const nameRva = toRva(nameOffset);
      if (directoryIndex === 1) {
        bytes.writeUInt32LE(nameRva, descriptor + 12);
        bytes.writeUInt32LE(0x1800, descriptor + 16);
      } else {
        bytes.writeUInt32LE(delayAttributes, descriptor);
        bytes.writeUInt32LE(delayAttributes === 1 ? nameRva : Number(imageBase) + nameRva, descriptor + 4);
        bytes.writeUInt32LE(0x1800, descriptor + 12);
        bytes.writeUInt32LE(0x1810, descriptor + 16);
      }
      bytes.write(name + "\0", nameOffset, "latin1");
      nameOffset += name.length + 1;
    });
  }
  bytes.write(legalText, 0xc00, "latin1");
  return bytes;
}

test("reads actual regular and delay-import descriptors", () => {
  assert.deepEqual(readPeImports(fixture({ regular: ["KERNEL32.dll"], delayed: ["user32.dll"] })), [
    { name: "KERNEL32.dll", kind: "import" }, { name: "user32.dll", kind: "delay-import" },
  ]);
});

test("allows embedded NDI legal text and unrelated DLL imports", () => {
  assert.doesNotThrow(() => assertPeImageHasNoNdiReference(fixture({
    regular: ["KERNEL32.dll"], delayed: ["D3D11.dll"],
    legalText: "licenses/Processing.NDI.Lib.Licenses.txt Processing.NDI.Lib.x64.dll NDIlib_initialize",
  })));
  assert.deepEqual(readPeImports(fixture()), []);
});

for (const kind of ["regular", "delayed"]) {
  test(`rejects a real ${kind} NDI DLL import case-insensitively`, () => {
    assert.throws(() => assertPeImageHasNoNdiReference(fixture({ [kind]: ["pRoCeSsInG.NdI.LiB.x64.DlL"] }), "Show-ASIO"), /Show-ASIO must never reference or import Processing\.NDI\.Lib/);
  });
}

test("supports bounded legacy VA-based delay descriptors", () => {
  assert.deepEqual(readPeImports(fixture({ delayed: ["user32.dll"], delayAttributes: 0, imageBase: 0x400000n })), [
    { name: "user32.dll", kind: "delay-import" },
  ]);
});

test("ordinary package requires the exact delay-loaded NDI dependency", () => {
  assert.doesNotThrow(() => assertPeImageDelayLoadsNdi(fixture({ delayed: ["Processing.NDI.Lib.x64.dll"] })));
  for (const options of [{}, { regular: ["Processing.NDI.Lib.x64.dll"] },
    { delayed: ["Processing.NDI.Lib.x86.dll"] },
    { regular: ["Processing.NDI.Lib.x64.dll"], delayed: ["Processing.NDI.Lib.x64.dll"] }]) {
    assert.throws(() => assertPeImageDelayLoadsNdi(fixture(options)), /must delay-import exactly/);
  }
});

const rejectsMutation = (mutate, pattern) => {
  const bytes = fixture({ regular: ["KERNEL32.dll"], delayed: ["user32.dll"] });
  mutate(bytes);
  assert.throws(() => readPeImports(bytes), pattern);
};

test("fails closed on malformed headers and directories", () => {
  assert.throws(() => readPeImports(Buffer.alloc(10)), /truncated DOS/);
  rejectsMutation((bytes) => bytes.writeUInt16LE(0x14c, 0x84), /requires AMD64/);
  rejectsMutation((bytes) => bytes.writeUInt16LE(0, 0x94), /complete PE32/);
  rejectsMutation((bytes) => bytes.writeUInt32LE(17, 0x98 + 108), /data-directory count/);
  rejectsMutation((bytes) => bytes.writeUInt32LE(0, 0x98 + 60), /SizeOfHeaders/);
  rejectsMutation((bytes) => bytes.writeUInt32LE(0, 0x98 + 112 + 8), /invalid import directory/);
  rejectsMutation((bytes) => bytes.writeUInt32LE(0x9000, 0x98 + 112 + 8), /unmapped/);
  rejectsMutation((bytes) => bytes.writeUInt32LE(20, 0x98 + 112 + 12), /unterminated import descriptor/);
  rejectsMutation((bytes) => bytes.writeUInt32LE(2, 0x400), /unsupported delay-import attributes/);
  rejectsMutation((bytes) => bytes.writeUInt32LE(0, 0x200 + 16), /no first thunk/);
  rejectsMutation((bytes) => bytes.writeUInt32LE(0, 0x400 + 16), /no thunk tables/);
});

test("fails closed on invalid names and uninitialized section data", () => {
  rejectsMutation((bytes) => bytes.writeUInt32LE(0, 0x200 + 12), /invalid import RVA/);
  rejectsMutation((bytes) => bytes.writeUInt8(0, 0x900), /empty imported DLL/);
  rejectsMutation((bytes) => bytes.writeUInt8(0x1f, 0x900), /invalid imported DLL/);
  rejectsMutation((bytes) => bytes.writeUInt32LE(0x40, 0x188 + 16), /uninitialized section data/);
  rejectsMutation((bytes) => bytes.writeUInt32LE(0x100, 0x188 + 12), /ambiguous|unmapped/);
});

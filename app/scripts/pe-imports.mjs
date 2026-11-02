// Inspect the loader's import directories, not unrelated strings in embedded
// frontend assets or legal notices. This intentionally supports AMD64 PE32+ only.
export function readPeImports(bytes, label = "PE image") {
  if (!Buffer.isBuffer(bytes)) throw new Error(`${label} must be a Buffer.`);
  const fail = (detail) => { throw new Error(`${label}: ${detail}`); };
  const range = (offset, length, detail) => {
    if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(length)
      || offset < 0 || length < 0 || offset > bytes.length - length) fail(detail);
    return offset;
  };
  range(0, 64, "truncated DOS header");
  if (bytes.toString("latin1", 0, 2) !== "MZ") fail("missing DOS signature");
  const pe = bytes.readUInt32LE(0x3c);
  range(pe, 24, "truncated PE header");
  if (bytes.toString("latin1", pe, pe + 4) !== "PE\0\0") fail("missing PE signature");
  if (bytes.readUInt16LE(pe + 4) !== 0x8664) fail("requires AMD64 PE32+");
  const sectionCount = bytes.readUInt16LE(pe + 6);
  const optionalSize = bytes.readUInt16LE(pe + 20);
  const optional = pe + 24;
  range(optional, optionalSize, "truncated optional header");
  if (optionalSize < 112 || bytes.readUInt16LE(optional) !== 0x20b) fail("requires a complete PE32+ optional header");
  const directoryCount = bytes.readUInt32LE(optional + 108);
  if (directoryCount > 16 || 112 + directoryCount * 8 > optionalSize) fail("invalid data-directory count");
  const headerSize = bytes.readUInt32LE(optional + 60);
  const sectionTable = optional + optionalSize;
  range(sectionTable, sectionCount * 40, "truncated section table");
  if (headerSize < sectionTable + sectionCount * 40 || headerSize > bytes.length) fail("invalid SizeOfHeaders");
  const imageBase = bytes.readBigUInt64LE(optional + 24);
  const sections = [];
  for (let index = 0; index < sectionCount; index += 1) {
    const entry = sectionTable + index * 40;
    const virtualSize = bytes.readUInt32LE(entry + 8);
    const address = bytes.readUInt32LE(entry + 12);
    const rawSize = bytes.readUInt32LE(entry + 16);
    const rawOffset = bytes.readUInt32LE(entry + 20);
    range(rawOffset, rawSize, "section raw data escapes file");
    if (address + Math.max(virtualSize, rawSize) > 0x100000000) fail("section RVA overflow");
    sections.push({ address, virtualSize, rawSize, rawOffset });
  }
  const rvaOffset = (rva, length) => {
    if (!Number.isSafeInteger(rva) || rva <= 0 || rva + length > 0x100000000) fail("invalid import RVA");
    const matches = [];
    if (rva < headerSize && length <= headerSize - rva) matches.push(rva);
    for (const section of sections) {
      if (rva >= section.address && rva - section.address < Math.max(section.virtualSize, section.rawSize)) {
        const delta = rva - section.address;
        if (delta > section.rawSize - length) fail("import RVA references uninitialized section data");
        matches.push(section.rawOffset + delta);
      }
    }
    if (matches.length !== 1) fail("unmapped or ambiguous import RVA");
    return range(matches[0], length, "import data escapes file");
  };
  const dllName = (rva) => {
    let name = "";
    for (let index = 0; index < 4096; index += 1) {
      const value = bytes[rvaOffset(rva + index, 1)];
      if (value === 0) {
        if (!name) fail("empty imported DLL name");
        return name;
      }
      if (value < 32 || value > 126) fail("invalid imported DLL name");
      name += String.fromCharCode(value);
    }
    fail("unterminated or excessive imported DLL name");
  };
  const imports = [];
  for (const [directoryIndex, descriptorSize, kind] of [[1, 20, "import"], [13, 32, "delay-import"]]) {
    if (directoryIndex >= directoryCount) continue;
    const directory = optional + 112 + directoryIndex * 8;
    const rva = bytes.readUInt32LE(directory);
    const size = bytes.readUInt32LE(directory + 4);
    if (rva === 0 && size === 0) continue;
    if (rva === 0 || size < descriptorSize || size > 16 * 1024 * 1024) fail(`invalid ${kind} directory`);
    rvaOffset(rva, size);
    let terminated = false;
    for (let index = 0; index < Math.floor(size / descriptorSize); index += 1) {
      if (index >= 4096) fail(`excessive ${kind} descriptors`);
      const descriptor = rvaOffset(rva + index * descriptorSize, descriptorSize);
      const fields = Array.from({ length: descriptorSize / 4 }, (_, field) => bytes.readUInt32LE(descriptor + field * 4));
      if (fields.every((field) => field === 0)) { terminated = true; break; }
      let nameRva;
      if (kind === "import") {
        nameRva = fields[3];
        if (fields[4] === 0) fail("import descriptor has no first thunk");
      } else {
        if ((fields[0] & ~1) !== 0) fail("unsupported delay-import attributes");
        if (fields[3] === 0 || fields[4] === 0) fail("delay-import descriptor has no thunk tables");
        if ((fields[0] & 1) === 1) nameRva = fields[1];
        else {
          const relative = BigInt(fields[1]) - imageBase;
          if (relative <= 0n || relative > 0xffffffffn) fail("invalid delay-import name VA");
          nameRva = Number(relative);
        }
      }
      imports.push(Object.freeze({ name: dllName(nameRva), kind }));
    }
    if (!terminated) fail(`unterminated ${kind} descriptor table`);
  }
  return Object.freeze(imports);
}

export function assertPeImageHasNoNdiReference(bytes, label = "PE image") {
  for (const imported of readPeImports(bytes, label)) {
    if (/processing\.ndi\.lib/iu.test(imported.name)) {
      throw new Error(`${label} must never reference or import Processing.NDI.Lib (${imported.kind}: ${imported.name}).`);
    }
  }
}

export function assertPeImageDelayLoadsNdi(bytes, label = "PE image") {
  const ndi = readPeImports(bytes, label).filter((entry) => /processing\.ndi\.lib/iu.test(entry.name));
  if (ndi.length !== 1 || ndi[0].kind !== "delay-import"
    || ndi[0].name.toLowerCase() !== "processing.ndi.lib.x64.dll") {
    throw new Error(`${label} must delay-import exactly Processing.NDI.Lib.x64.dll; ordinary imports or missing NDI are forbidden.`);
  }
}

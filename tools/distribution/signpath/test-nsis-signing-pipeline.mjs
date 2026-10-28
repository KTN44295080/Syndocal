import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { imageHash, patchScript, prepare, compile } from './nsis-signing-pipeline.mjs';

const root = fs.mkdtempSync(path.join(os.tmpdir(), 'syndocal-signpath-test-'));
const source = path.join(root, 'source');
fs.mkdirSync(source);
function pe() {
  const bytes = Buffer.alloc(512);
  bytes.write('MZ'); bytes.writeUInt32LE(64, 60); bytes.write('PE\0\0', 64);
  bytes.writeUInt16LE(0x20b, 88); return bytes;
}
try {
  const raw = pe();
  const signed = Buffer.concat([raw, Buffer.alloc(16, 42)]);
  signed.writeUInt32LE(512, 232); signed.writeUInt32LE(16, 236);
  signed.writeUInt32LE(1234, 152);
  assert.equal(imageHash(raw), imageHash(signed));
  signed[300] = 1;
  assert.notEqual(imageHash(raw), imageHash(signed));
  assert.throws(() => imageHash(Buffer.from('bad')), /Invalid/);
  signed.writeUInt32LE(999, 236);
  assert.throws(() => imageHash(signed), /certificate/);
  const odd = Buffer.concat([raw, Buffer.from([1])]);
  const padded = Buffer.concat([odd, Buffer.alloc(7), Buffer.alloc(16, 42)]);
  padded.writeUInt32LE(520, 232); padded.writeUInt32LE(16, 236);
  assert.equal(imageHash(odd), imageHash(padded));
  const script = `Unicode true
!define MAINBINARYSRCPATH "${path.join(source, 'syndocal.exe')}"
!define OUTFILE "unused.exe"
!define UNINSTALLERSIGNCOMMAND ""
OutFile "\${OUTFILE}"
RequestExecutionLevel user
VIProductVersion "1.2.0.0"
VIAddVersionKey "ProductName" "Syndocal"
VIAddVersionKey "ProductVersion" "1.2.0-alpha.82"
VIAddVersionKey "FileVersion" "1.2.0-alpha.82"
VIAddVersionKey "FileDescription" "Syndocal"
VIAddVersionKey "LegalCopyright" "Syndocal"
!if "\${UNINSTALLERSIGNCOMMAND}" != ""
!uninstfinalize '\${UNINSTALLERSIGNCOMMAND}'
!endif
Section
  WriteUninstaller "$INSTDIR\\uninstall.exe"
SectionEnd
Section Uninstall
SectionEnd
`;
  const args = { work: path.join(root, 'work'), mode: 'capture', payload: path.join(root, 'work', 'payload'), output: path.join(root, 'installer.exe') };
  const patched = patchScript(script, args);
  assert.ok(patched.includes('hook capture'));
  assert.ok(patched.includes("!uninstfinalize '${UNINSTALLERSIGNCOMMAND}' = 0"));
  assert.throws(() => patchScript(script.replace('WriteUninstaller', 'NotWriteUninstaller'), args), /Unexpected/);
  assert.throws(() => patchScript(script + '\n!define OUTFILE "duplicate.exe"', args), /exactly one/);
  assert.throws(() => patchScript(script, { ...args, work: 'bad$path' }), /Unsafe/);
  const makensis = process.env.SIGNPATH_TEST_MAKENSIS;
  if (makensis) {
    fs.writeFileSync(path.join(source, 'syndocal.exe'), raw);
    const scriptPath = path.join(source, 'installer.nsi');
    fs.writeFileSync(scriptPath, script);
    prepare({ script: scriptPath, makensis: path.resolve(makensis), work: args.work });
    assert.ok(fs.statSync(path.join(args.work, 'payload', 'uninstall.exe')).size > 512);
    assert.ok(fs.statSync(path.join(args.work, 'unsigned-installer.exe')).size > 512);
    const originalUninstaller = fs.readFileSync(path.join(args.work, 'payload', 'uninstall.exe'));
    const paddedLength = Math.ceil(originalUninstaller.length / 8) * 8;
    const certificateFixture = Buffer.concat([originalUninstaller, Buffer.alloc(paddedLength - originalUninstaller.length), Buffer.alloc(16, 42)]);
    const peOffset = certificateFixture.readUInt32LE(60) + 24;
    const securityDirectory = peOffset + (certificateFixture.readUInt16LE(peOffset) === 0x20b ? 112 : 96) + 32;
    certificateFixture.writeUInt32LE(paddedLength, securityDirectory);
    certificateFixture.writeUInt32LE(16, securityDirectory + 4);
    fs.writeFileSync(path.join(args.work, 'payload', 'uninstall.exe'), certificateFixture);
    compile(args.work, 'replace', path.join(args.work, 'payload'), path.join(root, 'second-installer.exe'));
    assert.ok(fs.statSync(path.join(root, 'second-installer.exe')).size > 512);
    if (process.env.SIGNPATH_TEST_7ZIP) {
      const extraction = path.join(root, 'extracted');
      const result = spawnSync(process.env.SIGNPATH_TEST_7ZIP, ['x', '-y', `-o${extraction}`, path.join(root, 'second-installer.exe')], { encoding: 'utf8' });
      assert.equal(result.status, 0, result.stderr);
      assert.deepEqual(fs.readFileSync(path.join(extraction, 'uninstall.exe')), certificateFixture);
      console.log('7-Zip extracted byte-identical certificate-table uninstaller fixture');
    }
    // Exercise replacement hook directly: exact image accepted, modified image rejected.
    const hook = path.resolve('tools/distribution/signpath/nsis-signing-pipeline.mjs');
    const generated = path.join(root, 'generated.exe');
    const captured = path.join(args.work, 'payload', 'uninstall.exe');
    fs.writeFileSync(generated, originalUninstaller);
    const invoke = () => spawnSync(process.execPath, [hook, 'hook', 'replace', args.work, path.dirname(captured), generated], { encoding: 'utf8' });
    assert.equal(invoke().status, 0);
    const changed = fs.readFileSync(captured); changed[300] ^= 1; fs.writeFileSync(captured, changed);
    assert.notEqual(invoke().status, 0);
    console.log(`NSIS pipeline: ${process.env.SIGNPATH_TEST_7ZIP ? 17 : 15} assertions passed, two real makensis packaging passes and synthetic certificate-table replacement exercised`);
  } else console.log('NSIS pipeline: 10 assertions passed; set SIGNPATH_TEST_MAKENSIS for packaging smoke');
} finally { fs.rmSync(root, { recursive: true, force: true }); }

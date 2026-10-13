import { spawnSync } from 'node:child_process';
import { mkdirSync, existsSync, copyFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

export function buildMsiExtensions(test = false) {
  if (process.platform !== 'win32') return;
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  const linker = process.env.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER;
  const editions = ['C:/Program Files/Microsoft Visual Studio/2022/Community', 'C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools'];
  if (process.env.GITHUB_ACTIONS === 'true' && process.env.RUNNER_ENVIRONMENT === 'github-hosted') editions.push('C:/Program Files/Microsoft Visual Studio/2022/Enterprise');
  const normalize = value => value.replaceAll('\\', '/').toLowerCase();
  if (!linker || !editions.some(edition => normalize(linker) === normalize(edition + '/VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64/link.exe')) || !existsSync(linker)) throw new Error('MSI extension build requires the exact initialized MSVC 14.44.35207 linker');
  const first = spawnSync('where.exe', ['link.exe'], { encoding: 'utf8', windowsHide: true });
  if (first.status !== 0 || normalize(first.stdout.trim().split(/\r?\n/)[0]) !== normalize(linker)) throw new Error('MSI extension build requires the pinned linker first on PATH');
  const triple = process.env.TAURI_ENV_TARGET_TRIPLE;
  if (triple && triple !== 'x86_64-pc-windows-msvc') throw new Error('MSI extension action supports the verified Windows x64 package only');
  const target = process.env.CARGO_TARGET_DIR;
  if (target && !/^(?:[A-Za-z]:[\\/]|\\\\)/.test(target)) throw new Error('CARGO_TARGET_DIR must be absolute');
  const targetRoot = resolve(target || resolve(root, 'target'));
  const release = resolve(targetRoot, 'release');
  mkdirSync(release, { recursive: true });
  const destination = resolve(release, test ? 'msi-extension-policy-tests.exe' : 'syndocal.exe.extensions-ca');
  const args = ['--edition=2021', '-D', 'warnings', '--target', 'x86_64-pc-windows-msvc', '-C', 'linker=' + linker, '-C', 'opt-level=s', '-C', 'target-feature=+crt-static', '-C', 'link-arg=/NOIMPLIB', '-C', 'link-arg=/NOEXP', '--crate-name', 'syndocal_msi_extensions', ...(test ? ['--test'] : ['--crate-type', 'cdylib']), resolve(root, 'tools/distribution/wix/extensions-ca.rs'), '-o', destination];
  const result = spawnSync('rustc', args, { cwd: root, stdio: 'inherit', windowsHide: true });
  if (result.error || result.status !== 0) throw result.error || new Error('MSI extension action compilation failed');
  if (!test && triple) {
    // Tauri reports the host triple even without an explicit --target. Mirror
    // the action into the two possible build outputs, as runtime staging does.
    const tripleRelease = resolve(targetRoot, triple, 'release');
    mkdirSync(tripleRelease, { recursive: true });
    copyFileSync(destination, resolve(tripleRelease, 'syndocal.exe.extensions-ca'));
  }
  if (test) {
    const result = spawnSync(destination, [], { stdio: 'inherit', windowsHide: true });
    if (result.error || result.status !== 0) throw result.error || new Error('MSI extension transaction tests failed');
  }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) buildMsiExtensions(process.argv.includes('--test'));

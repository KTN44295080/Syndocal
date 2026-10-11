// MSI compares only three version fields. Encode prereleases in the third field.
export function msiProductVersion(version) {
  const match = /^(\d+)\.(\d+)\.(\d+)(?:-(alpha|beta|rc)\.(\d+))?$/.exec(version);
  if (!match) throw new Error('Unsupported MSI product version');
  const [, major, minor, patch, stage, ordinal] = match;
  const offset = { alpha: 0, beta: 3000, rc: 6000 };
  const number = stage ? Number(ordinal) : 0;
  const build = Number(patch) * 10000 + (stage ? offset[stage] + number : 9000);
  if (Number(major) > 255 || Number(minor) > 255 || build > 65535 ||
      (stage && (number < 1 || number >= 3000))) throw new Error('MSI version range exhausted');
  return `${Number(major)}.${Number(minor)}.${build}`;
}

export function assertInstallerLifecycle(tauri, version) {
  if (tauri.bundle?.windows?.wix?.version !== msiProductVersion(version)) {
    throw new Error('MSI three-field version must advance with each distributed product version');
  }
  if (tauri.bundle?.windows?.nsis?.installerHooks !== 'installer-hooks.nsh' ||
      tauri.bundle.windows.allowDowngrades !== false) {
    throw new Error('Windows uninstall lifecycle hooks are missing');
  }
}

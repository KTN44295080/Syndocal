import assert from 'node:assert/strict';
import test from 'node:test';
import { msiProductVersion, assertInstallerLifecycle } from './installer-lifecycle-policy.mjs';
test('MSI upgrade ordering includes prerelease ordinal and release stages', () => {
  const versions = ['1.2.0-alpha.72','1.2.0-alpha.73','1.2.0-beta.1','1.2.0-rc.1','1.2.0','1.2.1-alpha.1'];
  const builds = versions.map(v => Number(msiProductVersion(v).split('.')[2]));
  for (let i = 1; i < builds.length; ++i) assert.ok(builds[i] > builds[i - 1]);
  assert.equal(msiProductVersion('1.2.0-alpha.73'), '1.2.73');
  for (const v of ['1.2.0-alpha.0','1.2.0-alpha.3000','1.2.0-dev.1','256.2.0','1.2.7']) {
    assert.throws(() => msiProductVersion(v));
  }
});
test('retired fourth-field MSI identity and absent cleanup hook are rejected', () => {
  const config = {bundle:{windows:{allowDowngrades:false,wix:{version:'1.2.0.70'},nsis:{installerHooks:'installer-hooks.nsh'}}}};
  assert.throws(() => assertInstallerLifecycle(config, '1.2.0-alpha.73'));
  config.bundle.windows.wix.version = '1.2.73';
  assert.doesNotThrow(() => assertInstallerLifecycle(config, '1.2.0-alpha.73'));
  delete config.bundle.windows.nsis.installerHooks;
  assert.throws(() => assertInstallerLifecycle(config, '1.2.0-alpha.73'));
});

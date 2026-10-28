# SignPath Foundation NSIS candidate preparation

This path creates candidates, never a public release. Enrollment, identity, MFA,
GitHub App installation, secrets and human signing approval belong to the owner.
No paid certificate, runner or service is a fallback. See
[Code signing policy](../../../qa/CODE_SIGNING_POLICY.md).

## Requirements checked on 2026-10-28

| Requirement | Repository evidence / remaining action |
| --- | --- |
| OSI license | Public repository and MIT root license verified. Bundled-component eligibility, complete corresponding source and ASIO profile review remain separate gates. |
| Maintained and documented | Public repository is active and README describes show-control functionality. |
| Existing release in intended form | **Missing public evidence:** public release API returned only the FFmpeg SDK dependency prerelease, no Syndocal NSIS product release. Do not call that SDK release a product release. |
| Source and trusted build | Lockfiles and exact SDK/toolchain pins exist; new hosted candidate workflow is prepared. Successful hosted execution and provider origin verification remain unobserved. |
| MFA and roles | Named roles are in the public policy. Owner must verify account MFA and disclose single-maintainer role overlap. |
| Public signing policy | Added policy, README link and release-note link. Owner must include the link on each actual download/release page. |
| Human approval | Two request steps are prepared. Owner must configure mandatory approval for every request in SignPath. No approval was performed. |
| Privacy/component disclosures | Policy identifies network-use and component-review boundaries. Actual release endpoint, complete privacy/consent review and provider acceptance remain open. |

Sources: [Foundation terms](https://signpath.org/terms) (published as draft),
[GitHub trusted builds](https://docs.signpath.io/trusted-build-systems/github),
[public repository API](https://api.github.com/repos/KTN44295080/Syndocal),
[public releases API](https://api.github.com/repos/KTN44295080/Syndocal/releases?per_page=100).
Foundation makes the eligibility decision; this table is not its approval.

## Owner actions in order

1. Verify GitHub MFA at [account security](https://github.com/settings/security),
   and MFA for all signing participants in [SignPath](https://app.signpath.io/).
   Confirm the named author/reviewer/approver in the public policy and disclose
   the single-maintainer arrangement. Do not give CI approval authority.
2. Resolve the existing-public-NSIS-release eligibility gap. Provide a genuine
   prior public Syndocal NSIS release URL if one exists. Otherwise complete the
   ordinary unsigned product release acceptance process before publishing that
   first release; this task does not authorize publication. Include the policy
   link and unsigned note on its download page. Do not publish an unaccepted
   candidate merely to satisfy an application checkbox.
3. Apply personally at [SignPath Foundation](https://signpath.org/apply), giving
   [repository](https://github.com/KTN44295080/Syndocal),
   [MIT license](https://github.com/KTN44295080/Syndocal/blob/main/LICENSE), actual
   product release/download URL, and
   [public policy](https://github.com/KTN44295080/Syndocal/blob/main/qa/CODE_SIGNING_POLICY.md).
   Provide contact/identity details yourself. Disclose the FFmpeg inventory and
   source/license gates; ask about upstream runtimes, generated NSIS uninstaller,
   and single-maintainer roles. Optional GPL-ASIO is outside this normal profile
   and needs separate approval before adding it. There is no approval-time promise.
4. After acceptance, configure the Syndocal project at
   [SignPath](https://app.signpath.io/) using the exact repository URL and the
   GitHub.com trusted build system. Create artifact configurations from
   `payload-artifact-configuration.xml` and `artifact-configuration.xml`. Both
   require `version`; they enforce ProductName `Syndocal` and one ProductVersion.
   Create a release policy with mandatory human approval, allowed submitter,
   repository/ref/workflow restrictions and origin verification. Require hosted
   runners. Verify actual certificate CN is `SignPath Foundation`, record its
   40-character uppercase thumbprint, and confirm PE metadata on a real run.
   See [projects/policies](https://docs.signpath.io/projects) and
   [trusted-build setup](https://docs.signpath.io/trusted-build-systems/github).
5. Install the [SignPath GitHub App](https://github.com/apps/signpath), limiting
   access to this repository. Confirm repository access and audit/origin checks
   in SignPath. This requires your GitHub identity and possibly MFA.
6. Configure active branch/tag protection at
   [repository rules](https://github.com/KTN44295080/Syndocal/settings/rules).
   Live public API inspection on 2026-10-28 reports `main` as **unprotected**;
   this setup step is required before either signing state can run.
   Protect `main` and any `v*` tags used for candidates. At
   [environments](https://github.com/KTN44295080/Syndocal/settings/environments),
   create `release-signpath` with deployment restrictions matching those refs and
   the available review controls. The job also checks `github.ref_protected`;
   an unprotected ref is skipped, including unsigned mode. No self-hosted runner.
7. Create a SignPath CI credential restricted to submission for this project and
   policy. In that environment, store secret `SIGNPATH_API_TOKEN` and the six
   public variables below. Copy actual identifiers, never example credentials.

   | Environment variable | Value |
   | --- | --- |
   | `SIGNPATH_ORGANIZATION_ID` | Assigned SignPath organization ID |
   | `SIGNPATH_PROJECT_SLUG` | Configured project slug |
   | `SIGNPATH_SIGNING_POLICY_SLUG` | Human-approved release policy slug |
   | `SIGNPATH_PAYLOAD_CONFIGURATION_SLUG` | Configuration for the two owned payload EXEs |
   | `SIGNPATH_INSTALLER_CONFIGURATION_SLUG` | Configuration for final NSIS EXE |
   | `SIGNPATH_CERTIFICATE_THUMBPRINT` | Actual approved certificate, 40 uppercase hex characters |

   The token is the secret; the six table entries are nonsecret configuration.
   GitHub's token is restricted to `contents: read` and `actions: read`. No PFX,
   local executable upload, paid certificate or broad personal GitHub token.
8. Separately wire the existing production updater identity/key custody at
   [environment settings](https://github.com/KTN44295080/Syndocal/settings/environments).
   Use secrets `TAURI_SIGNING_PRIVATE_KEY` and
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, and variables `SYNDOCAL_UPDATE_PUBKEY`
   (base64 Tauri public key), `SYNDOCAL_UPDATE_PUBLIC_KEY_FINGERPRINT`
   (SHA-256 of its trimmed base64 text), `SYNDOCAL_UPDATE_ENDPOINT` (canonical
   HTTPS endpoint), `SYNDOCAL_UPDATE_CHANNEL` (`stable`, `beta`, or `nightly`),
   and `SYNDOCAL_UPDATE_DOWNLOAD_BASE` (canonical HTTPS download directory ending
   in that channel, without trailing slash). Use the actual production key,
   never repository fixtures. Back up the key under your established custody;
   agents must not create, enter, print or retain it. See
   [Tauri signing](https://v2.tauri.app/plugin/updater/#signing-updates).
9. When release preparation is authorized, manually run
   [Windows release SignPath candidate](https://github.com/KTN44295080/Syndocal/actions/workflows/windows-release-signpath.yml)
   on the protected ref. Review and approve the **payload request first**, then
   the **installer request**, in SignPath. Verify commit, run, input artifact,
   metadata, component inventory, tests and certificate for each. If SignPath
   configuration is absent/incomplete, CI creates a declared-unsigned candidate.
   A started request that fails/rejects/times out stops that run; use a fresh,
   explicitly unsigned run if the owner selects the fallback.
10. Download the final candidate and inspect `SIGNING-STATE.txt`, `candidate.json`,
    `windows-signing.json`, `SHA256SUMS`, and the `.sig`/`updater.json`/`updater.pub`
    when configured. A missing-production-updater marker is **not publishable**.
    Freeze these bytes/state; perform required native, clean-machine, actual
    download/SmartScreen and N→N+1/wrong-key/corruption/offline acceptance. Full
    release evidence remains subject to the existing RC-only admission rule;
    the owner-approved stable-version freeze needs its separate release-evidence
    alignment before it can pass that gate. Do not interpret this signing work
    as closing `DIST-UPDATER-001`, legal, physical or publication acceptance.
    Publish only through the separately authorized release process. If freezing
    unsigned, record `DIST-SIGNING-001` Deferred with reason/owner/date and update
    the completion ledger/mirror; this task does not preempt that decision.

## Pipeline and local validation

CI builds the current source with no binary inputs. Tauri's rendered NSIS script
is reused to capture the uninstaller through `!uninstfinalize`. A single payload
request signs `syndocal.exe` and `uninstall.exe`; the helper compares their PE
images and recompiles only the installer script, preserving resources and exact
signed bytes. A second request signs the outer installer. Its image is compared
with the uploaded installer; installed application/uninstaller bytes are compared
with the approved payload. Upstream FFmpeg stays unchanged and pinned.

After Authenticode, the final NSIS itself is signed with the independent Tauri
updater key. The finalizer verifies that signature, embedded updater identity
and exact signing tree before writing hashes. The finalizer's `candidate.json`
schema 1 is a CI receipt, not release-evidence schema 2. The latter requires
`windowsSigning`; old schema 1 release evidence is deliberately rejected.
Extraction schema 3 is NSIS/updater only; historical schema 2 MSI extractions
cannot pass final candidate admission. Local PFX/store signing CLI flags are
retired. `sign-windows-artifact.mjs` forwards to the final-state verifier.

Run focused checks without contacting SignPath:

```powershell
node tools/distribution/signpath/check-workflow.mjs --self-test
node tools/distribution/signpath/finalize-candidate.mjs --self-test
node app/scripts/sign-windows-release-state.test.mjs
node app/scripts/test-check-release-metadata.mjs
node app/scripts/check-windows-release-artifacts.mjs --self-test
node app/scripts/windows-candidate-extractor.mjs --self-test
$env:SIGNPATH_TEST_MAKENSIS = "$env:LOCALAPPDATA/tauri/NSIS/makensis.exe"
$env:SIGNPATH_TEST_7ZIP = 'C:/Program Files/7-Zip/7z.exe'
node tools/distribution/signpath/test-nsis-signing-pipeline.mjs
```

The packaging test uses synthetic PE/certificate data, not a real signature or
product release. Neither its success nor workflow lint proves provider enrollment,
hosted execution, runtime behavior or clean-machine acceptance.

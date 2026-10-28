# Windows NSIS download signing note

Include the following paragraph on the release/download page whenever its
manifest says `declared-unsigned`:

> This Windows installer is distributed without an Authenticode signature.
> Windows SmartScreen may show an unrecognized-app or unknown-publisher warning;
> Smart App Control or organization policy may prevent execution. Verify the
> download URL and compare `Get-FileHash .\Syndocal_<version>_x64-setup.exe -Algorithm SHA256`
> with the release's published `SHA256SUMS` before running it. Updates still require
> the production updater signature; an unsigned Windows installer does not disable
> update verification. Free Authenticode signing remains pending.

Publish `SHA256SUMS`, the exact installer, component/signing inventory and, when
production updater signing is configured, the installer's `.sig` together.
The updater verifies that signature against its embedded public key; SHA-256
alone verifies a match to the published digest, not the publisher's identity.
Do not publish a candidate marked `missing-production-updater-configuration`.

An Authenticode signature does not guarantee that SmartScreen will show no
warning. Do not instruct users to disable Windows protection. Actual download
and warning behavior must be checked on the frozen installer during acceptance.

See [Code signing policy](CODE_SIGNING_POLICY.md),
[Microsoft SmartScreen](https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/)
and [Tauri updater signing](https://v2.tauri.app/plugin/updater/).

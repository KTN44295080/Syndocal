# Cross-platform CI recovery (2026-11-05)

Base: `main` red since `cb80e485` (2026-10-28, last green). Green again at
`f421ba04`, run `41989942056` (Windows 10+ and Ubuntu 22.04 succeeded,
including installer bundling and smoke tests).

Each blocker surfaced only after the previous one was cleared:

| Commit | Blocker | Fix |
| --- | --- | --- |
| `94c7e8a0` | NDI runtime download returned `AccessDenied` (direct `downloads.ndi.tv` path is not public). | Use NDI's published redistributable link; Authenticode and pinned DLL hash unchanged. |
| `df2b6be7` | Native release warning ratchet failed with no diagnosis (output captured). | Print a bounded tail of the failing command's output. |
| `1e55ac42` | `grafton-ndi` needs NDI SDK headers + import lib; SDK download requires registration (owner decision 2026-11-05: provision automatically). | Vendor the MIT-licensed NDI 6.3.2 headers (`vendor/ndi-sdk-6.3.2-headers`, hash-manifested, never packaged); `synthesize-ndi-sdk.ps1` generates the import library from the header declarations (runtime binary not inspected). Locally: 152 imports equal to the official lib's declared set; `io --features ndi` links. |
| `8a99a8a3`, `5c2d5485` | DJ Link socket-gate test got `Busy` on the slow runner. | Bounded 2 s retry of the identical request, as other DJ Link tests do. |
| `4520a725` | WiX `light` LGHT0311: the ja-JP MSI (code page 932) cannot hold `NDI®` from the license page added in `2b4a61b9`. | Installers show an ASCII-only RTF rendering of `THIRD_PARTY_NOTICES.md` (`\u174?` escapes); `check-release-metadata` rejects a stale RTF. Bundled Markdown and its pin unchanged. |
| `f421ba04` | MSI smoke test: stale `1.2.82` literal; and with a license page WixUI skipped `SyndocalExtensionsDlg` (DJ Link could not be chosen on a fresh MSI install). | Expected version read from `tauri.conf.json`; flow is Welcome → License → Extensions → Install folder, asserted by `check-msi-extensions.ps1`. |

Not covered: interactive click-through of the MSI dialogs on a real
desktop (the table-level navigation is asserted; the installer smoke test
installs NSIS and extracts the MSI).

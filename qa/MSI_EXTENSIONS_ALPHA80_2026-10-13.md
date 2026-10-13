# Native MSI extension selection and I/O cleanup — alpha.80

Branch codex/scene-edit-installer-alpha79-20261013; base e626ce82d20f27faed0f5e68670a98c09f98c2c8. Runtime/I/O source 717fada93425ee2f6f3972bb3eda960864c8ee8c; final Windows installer source b2b741419b729abc9e1b2a1b157e8bc310cd87ad. Delivered alpha.79 and alpha.80 bytes are immutable. New FX/universe work belongs to the next checkpoint.

Direct MSI includes its own DJ Link checkbox before installation-directory selection. Repair enters the choice page; standard removal remains available. Core is the initial default, supported existing choices preselect on upgrade, and DJLINK=0/1 specifies quiet installation choice. The embedded action shares the app schema parser, rejects invalid/future configuration before mutation, writes atomically as the installing user, restores prior bytes on rollback and removes transaction files on commit. The ZIP helper delegates choice to the installer. No application launch is required. Removing upstream ARPNOREPAIR makes the documented Repair route reachable.

Setup I/O has no Extensions tab or editor. Installed DJ Link gates its own connection. Selected-scene saved fader values from alpha.79 remain intact.

## Distributed artifacts

- Windows ZIP: 223482980 bytes, SHA256 1aab5eb4bd9f9eb1aad7efa24315ab0a5e6cf9ebef5318669183e7edc7def5dc. Thirteen entries match staging; local MSI SHA256 149656ae2cc1526eeadade1268af22f68d46da8a8a1c3c266230cbf3b86fa875, NSIS fed647be9d3166a1813d0152c74122991453769365864b7d13dd122b411ab2f0, executable f576c194cc152266dbe4251fc5a987e5534e006fcab5021a9e3c1b206454a1c6.
- Apple Silicon PKG: 28986817 bytes, SHA256 d643635f8952de6e241e41bf51b7d2331825b64b69278d6ff0038384780778be. Mac source 717fada93425ee2f6f3972bb3eda960864c8ee8c; later Windows-only Repair changes do not affect Mac inputs.
- Mac removal ZIP: SHA256 2dac0f4d7e1ab4cf91259aca131991321e5d2fdc94851af21548f19b67f8aab3. Its single unchanged command file retains UNIX mode 100755; source SHA256 6c72694154395a54c5035ea3efd13d148a4d3c2cc00c90034b70721479235e9c.

## Evidence and limits

Local TypeScript, localization/release metadata, installer action policy, setup helper (8 cases), cleanup/uninstall (18 cases), and I/O browser (5 viewports) checks pass. Windows native build and exact-source/extracted-NSIS one-responsive-visible-maximized-window gates pass, including unauthenticated broker rejection and graceful close. Local compiled MSI passes 24 native table/transition checks and 17 actual embedded prepare/plan checks with unchanged user settings. Administrative extraction matches the executable and seven pinned FFmpeg DLLs/resources. No product installer was run on the human PC.

[Cross-platform run 38412363208](https://github.com/KTN44295080/Syndocal/actions/runs/38412363208) passes Windows/Linux checks on b2b741. [Actual Windows installer lifecycle 38428104764](https://github.com/KTN44295080/Syndocal/actions/runs/38428104764) passes NSIS/MSI installation, alpha.72 update, choice enable/disable/preservation, future-schema rejection, transaction cleanup, full product/registration/preferences/credentials/data removal, and external-project preservation. Hosted installer bytes differ from local packaging bytes, as recorded in windows-hosted-lifecycle.json; this is exact-source lifecycle evidence alongside local exact-byte inspection, not a claim that the local ZIP was installed on the runner. Native MSI GUI pixels remain unobserved; native UI tables/transitions and quiet installer actions are verified separately.

[Mac run 38406855771](https://github.com/KTN44295080/Syndocal/actions/runs/38406855771) passes all 11 DMG gates and 11 actual PKG lifecycle gates, including exact app/signature at /Applications, choice changes/preservation, future-config rejection and full removal. The prior relocation defect is fixed by staged-root packaging with BundleIsRelocatable=false.

Windows final first-party compiler warnings baseline/current/delta 0/0/0; the existing Vite size advisory remains one. Mac first-party test warnings 29/29/0 and release 82/82/0; third-party tauri-runtime-wry remains five. This is development-artifact acceptance, not beta/RC, full Recording completion, physical device acceptance or Mac GUI acceptance. Developer ID signing/notarization is absent. Pre-maintenance evidence is retained under explicit before-maintenance filenames and its unannounced ZIP is archived outside Downloads.

References: [Tauri MSI templates](https://v2.tauri.app/distribute/windows-installer/), [WiX deferred actions](https://docs.firegiant.com/wix3/tutorial/events-and-actions/at-a-later-stage/), [MSI property API](https://learn.microsoft.com/en-us/windows/win32/api/msiquery/nf-msiquery-msigetpropertyw). Upstream template is tauri-bundler-v2.5.0, commit 594822aa55eede86530a0b24dc68bb7d1fc1da67; only extension UI/actions, two navigation destinations and ARPNOREPAIR removal differ.

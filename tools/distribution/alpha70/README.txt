Syndocal 1.2.0-alpha.70 (development build)

Windows 10/11 x64:
  Keep this folder together. Run Install-Windows.cmd.
  It verifies the exact setup files, installs the signed Microsoft C++ x64
  runtime, then opens the Syndocal setup wizard. Allow administrator access
  for the Microsoft runtime if requested. If a restart is requested, restart
  Windows and run Install-Windows.cmd again.
  The EXE and MSI are alternatives; you do not need to install both.
  Internet access can be required for the WebView2 runtime if it is missing.
  Syndocal itself is unsigned; verify the provided SHA-256 before allowing it.

macOS 12+ Apple Silicon (M1/M2/M3/M4/etc.):
  Open the arm64 DMG and drag Syndocal.app to Applications.
  This development app is not Developer ID signed or notarized. If macOS
  blocks it, use the security settings for this specific app after verifying
  the supplied checksum. Intel Macs are not supported by this package.

Projects and media:
  Copy your .sdc project and its external media files to the other computer.
  Output devices, audio devices and network targets must be selected there.

Microsoft prerequisite source:
  https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist
  https://aka.ms/vc14/vc_redist.x64.exe
  Microsoft license: https://aka.ms/VCRedistLicense

Actual installation on another physical computer remains untested.

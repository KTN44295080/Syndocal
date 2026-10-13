Syndocal 1.2.0-alpha.78 (development build)

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

Update and removal:
  Close Syndocal before updating. Run Install-Windows.cmd from the NEW package;
  existing NSIS installations use /UPDATE to preserve settings. MSI is an
  alternative; do not install both formats. Its upgrade version now advances.
  On Mac, replace the complete Syndocal.app in Applications; keep your settings.
  Windows full removal: run Uninstall-Windows.cmd and type REMOVE. It runs the
  registered product uninstaller, then purges settings, caches, recovery data,
  owned temporary files and credentials. A cancelled/failed uninstall stops it.
  The normal Windows uninstaller also cleans data when its data checkbox is on.
  MSI users need Uninstall-Windows.cmd for full data removal.
  Mac full removal: open Remove-Syndocal.command in the DMG and type REMOVE.
  This removes the verified application bundle and this user's product data.
  External .sdc/media files and shared Microsoft/WebView2 runtimes remain.
  Downloads and installer archives are yours to delete separately.

Microsoft prerequisite source:
  https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist
  https://aka.ms/vc14/vc_redist.x64.exe
  Microsoft license: https://aka.ms/VCRedistLicense

Actual installation on another physical computer remains untested.

USB-DMX (DSD TECH SH-RS09B / FT232R Open DMX):
  Close QLC+ before using Syndocal. Select your USB-DMX interface in Setup >
  I/O > DMX, then prepare/confirm the selected device on that computer.
  On macOS, either /dev/cu.* or the native USB FTDI entry can be selected.
  Do not install a VCP driver solely for QLC+; Syndocal includes its own
  native USB path when no serial callout port exists. USB discovery does
  not prove that a generic FTDI cable is DMX: choose only your DMX adapter.
  After reconnect/restart, refresh and confirm again if the stored device
  generation is stale. No other interface is automatically substituted.
  Physical output on SH-RS09B has not been verified in this environment.

Included BO / USB startup repairs:
  Repairs OutputControl request identity collisions and full receipt history.
  Healthy USB-DMX startup does not add a blackout. Initial zero-write
  confirmation and fault shutdown remain. Upgrade/removal improved.
  Physical USB-DMX output still needs device testing.

Alpha.74 changes:
  Control base fixture/type/group faders send intermediate values through
  bounded history gestures, with latest-value coalescing. Bulk patch movement
  preserves fixture spacing and validates all destinations before applying.
  Ctrl/Cmd selects multiple fixtures; Move Selected also works in list mode.
  The default DJ title filter is empty; existing user mappings are retained.

Multiple universes:
  This build uses internal U0 for its managed USB-DMX output. The engine has
  multiple-route processing, but general output-route editing is currently
  unavailable. Do not rely on this package for multi-universe live output.

Alpha.75 changes:
  Reliability of bulk fixture movement improved.

Alpha.76 changes:
  Repairs MIDI mapping synchronization and Control/Setup MIDI Learn.
  Setup shows MIDI input selection and a direct Learn in Control entry.
  New creates an empty project with outputs disabled. The separate SAFE
  button is removed; ordinary DMX BO also clears the internal blackout.
  DMX, VID and ALL BO switching is verified in the native Windows app.

MIDI Learn:
  1. Select your device in Setup > I/O > MIDI.
  2. Choose Learn in Control, then select a pink button or fader.
  3. Wait for MIDI input ready, then move or press your hardware control.
  4. Close Learn mode and try the assigned control.
  Clock/MTC is optional. Use Manual mapping only for advanced setup.
  Physical MIDI reception on your controller still requires device testing.

Alpha.77 changes:
  MIDI Learn no longer restarts mapping hydration on its own save. Concurrent
  Learn and New share the pending mapping flush before replacing the project.
  Rename a scene directly in its Bank by double-clicking its name or pressing
  F2; Enter/blur saves and Escape cancels. The right-hand duplicate is removed.
  New removes authored Banks, scenes and mappings. One empty Bank 1 remains.

Alpha.78 changes:
  Repairs managed USB/Art-Net activation leaving a pending receipt that rejected BO.
  MIDI fader input updates the screen directly with project/session/sequence fences.
  New scene Learn toggles play/stop (Flash stays momentary). Relearn older play-only
  bindings or choose Scene play / stop. Learn creates LED feedback and connects an
  exact matching output when unique; otherwise choose the MIDI output manually.
  Use CC Mode for ordinary mappings; a Mackie/DAW protocol decoder is not included.
  Duplicate Faders/Attributes tabs are removed. Use the left category rail.
  Edit requires a selected scene. Live overrides do not rewrite saved scenes.
  Edit writes finish before switching to Live.
  DJ Link is optional. NSIS setup offers a selection page; MSI/macOS offer the
  first-launch choice. Change it later in Setup > I/O > Extensions. Updates preserve
  the choice, and full removal includes this setting. Core DMX/video/MIDI are included.
  Physical controller feedback, USB-DMX continuity and Mac GUI/device checks remain
  to be confirmed on the actual devices.

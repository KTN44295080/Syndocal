Syndocal 1.2.0-alpha.82 (development build)

Windows 10/11 x64
Keep the folder together and run Install-Windows.cmd. It verifies installer
hashes, installs the signed Microsoft C++ runtime, then opens Syndocal setup.
EXE and MSI are alternatives; install only one. Both have their own DJ Link
selection page. A fresh installation defaults to core DMX/video/MIDI.
To change extensions with the same MSI, rerun it and choose Repair.
An Internet connection can be required when WebView2 is missing.

What to check in this build
The MIDI card shows the feedback output, sent count and last message.
If LEDs stay dark while the count rises, check the controller mode/port.
Setup I/O and Security no longer show the fixture/stage band. Tabs follow
the UI language. I/O cards show state only; start/connect in each panel.
DMX diagnostics no longer repeats masters, blackout, BPM or MIDI clock.
Active/Standby sync and ShowClock LAN moved from Web Remote to a new I/O
Sync card.
Web Remote, ShowClock, sACN input and GDTF import validate input strictly.

Update
Save and close Syndocal, then run Install-Windows.cmd from this NEW package.
It preserves the existing NSIS/MSI format, settings and extension choice.
If the prerequisite asks for restart, restart Windows and rerun setup.

Complete removal
Run Uninstall-Windows.cmd to remove the app, settings, caches and product
credentials. External .sdc projects and media are preserved. NSIS also
offers application-data removal in its normal uninstaller.

macOS 12+ Apple Silicon
Open the arm64 PKG and select DJ Link in Customize when needed. The app
is installed into /Applications. Rerun the PKG to change extensions.
Remove-Syndocal.command beside the PKG performs complete removal after
typing REMOVE. External projects and media are preserved.
This development app is not Developer ID signed or notarized. Verify the
checksum before allowing this specific app in macOS security settings.
Intel Macs are not supported by this package.

Usage
Copy .sdc projects together with external media. Select devices/network
targets on the destination computer. CC Mode is supported for normal MIDI
mappings; a Mackie/DAW decoder is not included.
Control Edit shows the selected scene's saved values; Live overrides do not
rewrite it. Extension settings are selected in the installer, not in I/O.
Physical MIDI/USB-DMX and interactive Mac device acceptance remain separate.
